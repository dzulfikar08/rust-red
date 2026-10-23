//! Minimal BACnet/IP client over UDP: confirmed ReadProperty (ASHRAE 135 clause 15.5), WriteProperty (15.9) and
//! SubscribeCOV (13.14) with the COV notifications (13.7, 13.8) that follow.
//!
//! The client sends one request per call from a fresh ephemeral UDP socket, connected to the
//! target so only that device's replies are seen. Segmented ComplexAcks (clause 5.4) are
//! reassembled and acknowledged, so large properties such as `objectList` can be read.
//! Values are returned as JSON: Real/Double/Unsigned/Signed/Enumerated as numbers, Boolean as
//! boolean, character strings as strings, bit strings as arrays of booleans, dates and times as
//! strings, object identifiers as `{ "objectType", "instance" }` and constructed values as arrays.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::net::UdpSocket;
use tokio::time::Instant;

static NEXT_INVOKE_ID: AtomicU8 = AtomicU8::new(0);

const BVLC_TYPE: u8 = 0x81;
const BVLC_UNICAST: u8 = 0x0A;
const BVLC_BROADCAST: u8 = 0x0B;
const BVLC_FORWARDED: u8 = 0x04;
const SERVICE_READ_PROPERTY: u8 = 0x0C;
/// Largest APDU this client accepts (Max_APDU_Length_Accepted code 5 = 1476 octets).
const MAX_APDU_CODE: u8 = 0x05;
const MAX_FRAME: usize = 1600;

const OBJECT_TYPES: &[(&str, u32)] = &[
    ("analogInput", 0),
    ("analogOutput", 1),
    ("analogValue", 2),
    ("binaryInput", 3),
    ("binaryOutput", 4),
    ("binaryValue", 5),
    ("calendar", 6),
    ("command", 7),
    ("device", 8),
    ("eventEnrollment", 9),
    ("file", 10),
    ("group", 11),
    ("loop", 12),
    ("multiStateInput", 13),
    ("multiStateOutput", 14),
    ("notificationClass", 15),
    ("program", 16),
    ("schedule", 17),
    ("averaging", 18),
    ("multiStateValue", 19),
    ("trendLog", 20),
    ("lifeSafetyPoint", 21),
    ("lifeSafetyZone", 22),
    ("accumulator", 23),
    ("pulseConverter", 24),
    ("eventLog", 25),
    ("globalGroup", 26),
    ("trendLogMultiple", 27),
    ("loadControl", 28),
    ("structuredView", 29),
    ("accessDoor", 30),
    ("characterstringValue", 40),
    ("datetimeValue", 44),
    ("integerValue", 45),
    ("largeAnalogValue", 46),
    ("positiveIntegerValue", 48),
];

const PROPERTIES: &[(&str, u32)] = &[
    ("ackedTransitions", 0),
    ("activeText", 4),
    ("applicationSoftwareVersion", 12),
    ("covIncrement", 22),
    ("deadband", 25),
    ("description", 28),
    ("eventEnable", 35),
    ("eventState", 36),
    ("feedbackValue", 40),
    ("firmwareRevision", 44),
    ("highLimit", 45),
    ("inactiveText", 46),
    ("limitEnable", 52),
    ("lowLimit", 59),
    ("maxApduLengthAccepted", 62),
    ("maxPresValue", 65),
    ("minPresValue", 69),
    ("modelName", 70),
    ("notificationClass", 17),
    ("notifyType", 72),
    ("numberOfStates", 74),
    ("objectIdentifier", 75),
    ("objectList", 76),
    ("objectName", 77),
    ("objectType", 79),
    ("outOfService", 81),
    ("polarity", 84),
    ("presentValue", 85),
    ("priorityArray", 87),
    ("protocolRevision", 139),
    ("protocolVersion", 98),
    ("reliability", 103),
    ("relinquishDefault", 104),
    ("resolution", 106),
    ("segmentationSupported", 107),
    ("stateText", 110),
    ("statusFlags", 111),
    ("systemStatus", 112),
    ("timeDelay", 113),
    ("units", 117),
    ("vendorIdentifier", 120),
    ("vendorName", 121),
    ("databaseRevision", 155),
];

/// Lower-cases and drops everything but letters and digits, so `presentValue`, `present-value`
/// and `Present_Value` all compare equal.
fn normalize(name: &str) -> String {
    name.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
}

fn lookup(table: &[(&str, u32)], name: &str) -> Option<u32> {
    let wanted = normalize(name);
    if let Ok(n) = name.trim().parse::<u32>() {
        return Some(n);
    }
    table.iter().find(|(n, _)| normalize(n) == wanted).map(|(_, id)| *id)
}

pub(crate) fn object_type_id(name: &str) -> Option<u32> {
    lookup(OBJECT_TYPES, name).filter(|id| *id < 1024)
}

pub(crate) fn property_id(name: &str) -> Option<u32> {
    lookup(PROPERTIES, name)
}

fn object_type_name(id: u32) -> Value {
    OBJECT_TYPES.iter().find(|(_, i)| *i == id).map(|(n, _)| json!(n)).unwrap_or_else(|| json!(id))
}

fn push_tag(out: &mut Vec<u8>, number: u32, context: bool, len: usize) {
    let class = if context { 0x08 } else { 0 };
    let mut first = if number < 15 { (number as u8) << 4 } else { 0xF0 } | class;
    if len < 5 {
        first |= len as u8;
    } else {
        first |= 5;
    }
    out.push(first);
    if number >= 15 {
        out.push(number as u8);
    }
    if len >= 5 {
        if len < 254 {
            out.push(len as u8);
        } else {
            out.push(254);
            out.extend_from_slice(&(len as u16).to_be_bytes());
        }
    }
}

fn push_context_unsigned(out: &mut Vec<u8>, number: u32, value: u32) {
    let bytes = value.to_be_bytes();
    let skip = bytes.iter().take(3).take_while(|b| **b == 0).count();
    push_tag(out, number, true, 4 - skip);
    out.extend_from_slice(&bytes[skip..]);
}

const SERVICE_WRITE_PROPERTY: u8 = 0x0F;
const SERVICE_SUBSCRIBE_COV: u8 = 0x05;

fn push_object_id(out: &mut Vec<u8>, tag: u32, object_type: u32, instance: u32) {
    push_tag(out, tag, true, 4);
    out.extend_from_slice(&((object_type << 22) | (instance & 0x3F_FFFF)).to_be_bytes());
}

/// Complete BVLC frame carrying a confirmed request with `body` as its service data.
fn encode_confirmed(invoke_id: u8, service: u8, body: &[u8]) -> Vec<u8> {
    let mut apdu = vec![0x02, 0x70 | MAX_APDU_CODE, invoke_id, service];
    apdu.extend_from_slice(body);
    frame(&apdu, true)
}

fn read_property_body(object_type: u32, instance: u32, property: u32) -> Vec<u8> {
    let mut body = Vec::new();
    push_object_id(&mut body, 0, object_type, instance);
    push_context_unsigned(&mut body, 1, property);
    body
}

/// Complete BVLC frame carrying a confirmed ReadProperty request.
#[cfg(test)]
pub(crate) fn encode_read_property(invoke_id: u8, object_type: u32, instance: u32, property: u32) -> Vec<u8> {
    encode_confirmed(invoke_id, SERVICE_READ_PROPERTY, &read_property_body(object_type, instance, property))
}

/// A value to write, as one application-tagged BACnet primitive.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum WriteValue {
    Null,
    Boolean(bool),
    Unsigned(u64),
    Signed(i64),
    Real(f32),
    Double(f64),
    Enumerated(u32),
    String(String),
}

fn push_application(out: &mut Vec<u8>, value: &WriteValue) {
    fn minimal(bytes: &[u8], keep: usize) -> &[u8] {
        let skip = bytes.iter().take(bytes.len() - keep).take_while(|b| **b == 0).count();
        &bytes[skip..]
    }
    match value {
        WriteValue::Null => out.push(0x00),
        WriteValue::Boolean(b) => out.push(0x10 | *b as u8),
        WriteValue::Unsigned(v) => {
            let bytes = v.to_be_bytes();
            let content = minimal(&bytes, 1);
            push_tag(out, 2, false, content.len());
            out.extend_from_slice(content);
        }
        WriteValue::Signed(v) => {
            let bytes = v.to_be_bytes();
            let mut start = 0;
            while start < 7
                && ((bytes[start] == 0x00 && bytes[start + 1] & 0x80 == 0)
                    || (bytes[start] == 0xFF && bytes[start + 1] & 0x80 != 0))
            {
                start += 1;
            }
            push_tag(out, 3, false, 8 - start);
            out.extend_from_slice(&bytes[start..]);
        }
        WriteValue::Real(v) => {
            push_tag(out, 4, false, 4);
            out.extend_from_slice(&v.to_be_bytes());
        }
        WriteValue::Double(v) => {
            push_tag(out, 5, false, 8);
            out.extend_from_slice(&v.to_be_bytes());
        }
        WriteValue::Enumerated(v) => {
            let bytes = v.to_be_bytes();
            let content = minimal(&bytes, 1);
            push_tag(out, 9, false, content.len());
            out.extend_from_slice(content);
        }
        WriteValue::String(s) => {
            push_tag(out, 7, false, s.len() + 1);
            out.push(0x00);
            out.extend_from_slice(s.as_bytes());
        }
    }
}

fn write_property_body(
    object_type: u32,
    instance: u32,
    property: u32,
    value: &WriteValue,
    priority: Option<u8>,
) -> Vec<u8> {
    let mut body = read_property_body(object_type, instance, property);
    body.push(0x3E);
    push_application(&mut body, value);
    body.push(0x3F);
    if let Some(p) = priority {
        push_context_unsigned(&mut body, 4, p as u32);
    }
    body
}

fn json_number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::Bool(b) => Some(*b as u8 as f64),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Choose the BACnet type of `value` for a property. `hint` is `auto` or one of `real`, `double`, `unsigned`,
/// `signed`, `enumerated`, `boolean`, `string`, `null`. With `auto`, Present_Value follows the object type (clause 12:
/// analog objects take Real, binary objects Enumerated 0 or 1, multi-state objects Unsigned) and other properties
/// follow the JSON type. JSON `null` is always BACnet Null, which relinquishes a commandable priority slot.
pub(crate) fn write_value(object_type: u32, property: u32, value: &Value, hint: &str) -> Result<WriteValue, String> {
    if value.is_null() {
        return Ok(WriteValue::Null);
    }
    let bad = |what: &str| format!("cannot write {value} as {what}");
    let kind = match normalize(hint).as_str() {
        "" | "auto" => {
            if property == 85 {
                match object_type {
                    0..=2 => "real",
                    3..=5 => "enumerated",
                    13 | 14 | 19 => "unsigned",
                    _ => "generic",
                }
            } else {
                "generic"
            }
        }
        "real" | "float" => "real",
        "double" => "double",
        "unsigned" | "uint" => "unsigned",
        "signed" | "int" => "signed",
        "enumerated" | "enum" => "enumerated",
        "boolean" | "bool" => "boolean",
        "string" | "characterstring" => "string",
        "null" => return Ok(WriteValue::Null),
        other => return Err(format!("unknown BACnet value type '{other}'")),
    };
    match kind {
        "real" => json_number(value).map(|n| WriteValue::Real(n as f32)).ok_or_else(|| bad("Real")),
        "double" => json_number(value).map(WriteValue::Double).ok_or_else(|| bad("Double")),
        "unsigned" => match json_number(value) {
            Some(n) if n >= 0.0 && n.fract() == 0.0 => Ok(WriteValue::Unsigned(n as u64)),
            _ => Err(bad("Unsigned")),
        },
        "signed" => match json_number(value) {
            Some(n) if n.fract() == 0.0 => Ok(WriteValue::Signed(n as i64)),
            _ => Err(bad("Signed")),
        },
        "enumerated" => {
            if let Value::String(s) = value {
                match normalize(s).as_str() {
                    "active" | "on" | "true" => return Ok(WriteValue::Enumerated(1)),
                    "inactive" | "off" | "false" => return Ok(WriteValue::Enumerated(0)),
                    _ => {}
                }
            }
            match json_number(value) {
                Some(n) if n >= 0.0 && n.fract() == 0.0 => Ok(WriteValue::Enumerated(n as u32)),
                _ => Err(bad("Enumerated")),
            }
        }
        "boolean" => match value {
            Value::Bool(b) => Ok(WriteValue::Boolean(*b)),
            other => json_number(other).map(|n| WriteValue::Boolean(n != 0.0)).ok_or_else(|| bad("Boolean")),
        },
        "string" => Ok(WriteValue::String(match value {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        })),
        _ => match value {
            Value::Bool(b) => Ok(WriteValue::Boolean(*b)),
            Value::String(s) => Ok(WriteValue::String(s.clone())),
            Value::Number(n) if n.is_u64() => Ok(WriteValue::Unsigned(n.as_u64().unwrap())),
            Value::Number(n) if n.is_i64() => Ok(WriteValue::Signed(n.as_i64().unwrap())),
            Value::Number(n) => Ok(WriteValue::Real(n.as_f64().unwrap_or(0.0) as f32)),
            other => Err(format!("cannot write {other} to a BACnet property")),
        },
    }
}

fn frame(apdu: &[u8], expecting_reply: bool) -> Vec<u8> {
    let len = (apdu.len() + 6) as u16;
    let mut out = vec![BVLC_TYPE, BVLC_UNICAST];
    out.extend_from_slice(&len.to_be_bytes());
    out.push(0x01);
    out.push(if expecting_reply { 0x04 } else { 0x00 });
    out.extend_from_slice(apdu);
    out
}

/// The APDU inside a received BVLC frame, or `None` for anything that is not an application-layer frame.
fn apdu_of_frame(buf: &[u8]) -> Option<&[u8]> {
    if buf.len() < 4 || buf[0] != BVLC_TYPE || ((buf[2] as usize) << 8 | buf[3] as usize) != buf.len() {
        return None;
    }
    let mut pos = match buf[1] {
        BVLC_UNICAST | BVLC_BROADCAST => 4,
        BVLC_FORWARDED => 10,
        _ => return None,
    };
    let control = *buf.get(pos + 1)?;
    if buf[pos] != 1 || control & 0x80 != 0 {
        return None;
    }
    pos += 2;
    if control & 0x20 != 0 {
        let dlen = *buf.get(pos + 2)? as usize;
        pos += 3 + dlen;
    }
    if control & 0x08 != 0 {
        let slen = *buf.get(pos + 2)? as usize;
        pos += 3 + slen;
    }
    if control & 0x20 != 0 {
        pos += 1;
    }
    buf.get(pos..).filter(|a| !a.is_empty())
}

#[derive(Debug, PartialEq)]
enum Reply {
    /// Complete service data of a ComplexAck.
    Data(Vec<u8>),
    /// A segment was stored; send this SegmentAck APDU.
    SegmentAck(Vec<u8>),
    Error(String),
    Ignore,
}

#[derive(Default)]
struct Reassembly {
    data: Vec<u8>,
    next: u8,
}

fn class_name(class: u32) -> &'static str {
    match class {
        0 => "device",
        1 => "object",
        2 => "property",
        3 => "resources",
        4 => "security",
        5 => "services",
        6 => "vt",
        7 => "communication",
        _ => "unknown",
    }
}

fn code_name(code: u32) -> String {
    match code {
        2 => "configuration-in-progress".into(),
        9 => "invalid-data-type".into(),
        25 => "operational-problem".into(),
        26 => "optional-functionality-not-supported".into(),
        27 => "read-access-denied".into(),
        31 => "unknown-object".into(),
        32 => "unknown-property".into(),
        37 => "value-out-of-range".into(),
        40 => "service-request-denied".into(),
        42 => "property-is-not-an-array".into(),
        other => format!("code {other}"),
    }
}

fn handle_apdu(apdu: &[u8], invoke_id: u8, state: &mut Reassembly) -> Reply {
    let kind = apdu[0] >> 4;
    if apdu.len() < 2 || (kind != 3 && apdu[1] != invoke_id) {
        return Reply::Ignore;
    }
    match kind {
        3 => {
            if apdu.len() < 3 || apdu[1] != invoke_id {
                return Reply::Ignore;
            }
            let segmented = apdu[0] & 0x08 != 0;
            let more = apdu[0] & 0x04 != 0;
            let (seq, window, body) = if segmented {
                if apdu.len() < 5 {
                    return Reply::Ignore;
                }
                (apdu[2], apdu[3], &apdu[4..])
            } else {
                (0, 1, &apdu[2..])
            };
            if segmented && seq != state.next {
                // Negative SegmentAck asking for the segment we expect.
                return Reply::SegmentAck(vec![0x42, invoke_id, state.next.wrapping_sub(1), window]);
            }
            // Non-segmented bodies start with the service choice; segmented ones carry it in segment 0 only.
            let body = if !segmented || seq == 0 { body.get(1..).unwrap_or(&[]) } else { body };
            state.data.extend_from_slice(body);
            state.next = seq.wrapping_add(1);
            if segmented && more {
                Reply::SegmentAck(vec![0x40, invoke_id, seq, window])
            } else {
                Reply::Data(std::mem::take(&mut state.data))
            }
        }
        5 => {
            if apdu.len() < 3 {
                return Reply::Error("BACnet error (truncated)".into());
            }
            let mut pos = 3;
            let class = read_enum(apdu, &mut pos);
            let code = read_enum(apdu, &mut pos);
            match (class, code) {
                (Some(c), Some(k)) => {
                    Reply::Error(format!("BACnet error from device: {} / {}", class_name(c), code_name(k)))
                }
                _ => Reply::Error("BACnet error from device".into()),
            }
        }
        6 => Reply::Error(format!("BACnet reject from device (reason {})", apdu.get(2).copied().unwrap_or(0))),
        7 => Reply::Error(format!("BACnet abort from device (reason {})", apdu.get(2).copied().unwrap_or(0))),
        2 => Reply::Data(Vec::new()),
        _ => Reply::Ignore,
    }
}

fn read_enum(buf: &[u8], pos: &mut usize) -> Option<u32> {
    let (tag, header) = parse_tag(buf.get(*pos..)?)?;
    let content = buf.get(*pos + header..*pos + header + tag.len)?;
    *pos += header + tag.len;
    Some(content.iter().fold(0u32, |acc, b| (acc << 8) | *b as u32))
}

#[derive(Debug, Clone, Copy)]
struct Tag {
    number: u32,
    context: bool,
    opening: bool,
    closing: bool,
    len: usize,
}

/// Parse a tag header; returns the tag and the number of header octets.
fn parse_tag(buf: &[u8]) -> Option<(Tag, usize)> {
    let first = *buf.first()?;
    let mut pos = 1;
    let mut number = (first >> 4) as u32;
    if number == 15 {
        number = *buf.get(pos)? as u32;
        pos += 1;
    }
    let context = first & 0x08 != 0;
    let lvt = (first & 0x07) as usize;
    if context && lvt == 6 {
        return Some((Tag { number, context, opening: true, closing: false, len: 0 }, pos));
    }
    if context && lvt == 7 {
        return Some((Tag { number, context, opening: false, closing: true, len: 0 }, pos));
    }
    let len = if lvt == 5 {
        let b = *buf.get(pos)? as usize;
        pos += 1;
        if b == 254 {
            let v = (*buf.get(pos)? as usize) << 8 | *buf.get(pos + 1)? as usize;
            pos += 2;
            v
        } else if b == 255 {
            let v = u32::from_be_bytes(buf.get(pos..pos + 4)?.try_into().ok()?) as usize;
            pos += 4;
            v
        } else {
            b
        }
    } else {
        lvt
    };
    Some((Tag { number, context, opening: false, closing: false, len }, pos))
}

fn unsigned(content: &[u8]) -> u64 {
    content.iter().fold(0u64, |acc, b| (acc << 8) | *b as u64)
}

fn signed(content: &[u8]) -> i64 {
    if content.is_empty() {
        return 0;
    }
    let mut v = if content[0] & 0x80 != 0 { -1i64 } else { 0 };
    for b in content {
        v = (v << 8) | *b as i64;
    }
    v
}

fn number(v: f64) -> Value {
    serde_json::Number::from_f64(v).map(Value::Number).unwrap_or(Value::Null)
}

fn decode_string(content: &[u8]) -> String {
    let Some((&encoding, text)) = content.split_first() else {
        return String::new();
    };
    match encoding {
        0 => String::from_utf8_lossy(text).into_owned(),
        3 => {
            text.chunks_exact(4).filter_map(|c| char::from_u32(u32::from_be_bytes([c[0], c[1], c[2], c[3]]))).collect()
        }
        4 => char::decode_utf16(text.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])))
            .map(|r| r.unwrap_or('\u{FFFD}'))
            .collect(),
        5 => text.iter().map(|b| *b as char).collect(),
        _ => String::from_utf8_lossy(text).into_owned(),
    }
}

fn wildcard(b: u8) -> String {
    if b == 0xFF { "*".into() } else { b.to_string() }
}

fn decode_application(tag: Tag, content: &[u8]) -> Value {
    match tag.number {
        0 => Value::Null,
        // Boolean carries its value in the length field.
        1 => json!(tag.len != 0),
        2 => json!(unsigned(content)),
        3 => json!(signed(content)),
        4 if content.len() == 4 => number(f32::from_be_bytes(content.try_into().unwrap()) as f64),
        5 if content.len() == 8 => number(f64::from_be_bytes(content.try_into().unwrap())),
        6 => json!(content.iter().map(|b| format!("{b:02x}")).collect::<String>()),
        7 => json!(decode_string(content)),
        8 => match content.split_first() {
            Some((&unused, bytes)) => {
                let total = (bytes.len() * 8).saturating_sub(unused as usize);
                Value::Array((0..total).map(|i| json!(bytes[i / 8] & (0x80 >> (i % 8)) != 0)).collect())
            }
            None => json!([]),
        },
        9 => json!(unsigned(content)),
        10 if content.len() == 4 => {
            let year = if content[0] == 0xFF { "*".to_string() } else { (1900 + content[0] as u32).to_string() };
            json!(format!("{}-{}-{}", year, wildcard(content[1]), wildcard(content[2])))
        }
        11 if content.len() == 4 => {
            json!(format!(
                "{}:{}:{}.{}",
                wildcard(content[0]),
                wildcard(content[1]),
                wildcard(content[2]),
                wildcard(content[3])
            ))
        }
        12 if content.len() == 4 => {
            let raw = u32::from_be_bytes(content.try_into().unwrap());
            json!({ "objectType": object_type_name(raw >> 22), "instance": raw & 0x3F_FFFF })
        }
        _ => json!(content.iter().map(|b| format!("{b:02x}")).collect::<String>()),
    }
}

/// Decode a sequence of tagged values until the buffer ends or a closing tag is reached.
fn decode_sequence(buf: &[u8], pos: &mut usize, top: bool) -> Result<Vec<Value>, String> {
    let mut values = Vec::new();
    while *pos < buf.len() {
        let (tag, header) = parse_tag(&buf[*pos..]).ok_or("truncated BACnet value")?;
        if tag.closing {
            if top {
                return Err("unbalanced closing tag in BACnet value".into());
            }
            return Ok(values);
        }
        *pos += header;
        if tag.opening {
            let inner = decode_sequence(buf, pos, false)?;
            let (close, close_header) =
                parse_tag(buf.get(*pos..).ok_or("missing closing tag")?).ok_or("missing closing tag")?;
            if !close.closing {
                return Err("missing closing tag in BACnet value".into());
            }
            *pos += close_header;
            values.push(Value::Array(inner));
            continue;
        }
        // Boolean application tag has no content octets.
        let len = if !tag.context && tag.number == 1 { 0 } else { tag.len };
        let content = buf.get(*pos..*pos + len).ok_or("truncated BACnet value")?;
        *pos += len;
        values.push(if tag.context { json!(unsigned(content)) } else { decode_application(tag, content) });
    }
    if top { Ok(values) } else { Err("missing closing tag in BACnet value".into()) }
}

/// Decode the service data of a ReadProperty ComplexAck and check it answers the request.
pub(crate) fn decode_read_property_ack(
    data: &[u8],
    object_type: u32,
    instance: u32,
    property: u32,
) -> Result<Value, String> {
    let mut pos = 0;
    let (tag, header) = parse_tag(data).ok_or("empty ReadProperty reply")?;
    if !tag.context || tag.number != 0 || tag.len != 4 {
        return Err("malformed ReadProperty reply".into());
    }
    let raw =
        u32::from_be_bytes(data.get(header..header + 4).ok_or("truncated ReadProperty reply")?.try_into().unwrap());
    pos += header + 4;
    // A request for device instance 4194303 is answered with the device's real instance.
    if raw >> 22 != object_type || (raw & 0x3F_FFFF != instance && !(object_type == 8 && instance == 0x3F_FFFF)) {
        return Err("ReadProperty reply is for another object".into());
    }
    let prop = read_context_unsigned(data, &mut pos, 1).ok_or("ReadProperty reply has no property id")?;
    if prop != property {
        return Err("ReadProperty reply is for another property".into());
    }
    // An optional array index precedes the value.
    let _ = read_context_unsigned(data, &mut pos, 2);
    let (open, h) = parse_tag(data.get(pos..).ok_or("ReadProperty reply has no value")?)
        .ok_or("ReadProperty reply has no value")?;
    if !open.opening || open.number != 3 {
        return Err("ReadProperty reply has no value".into());
    }
    pos += h;
    let mut values = decode_sequence(data, &mut pos, false)?;
    Ok(if values.len() == 1 { values.remove(0) } else { Value::Array(values) })
}

fn read_context_unsigned(data: &[u8], pos: &mut usize, number: u32) -> Option<u32> {
    let (tag, header) = parse_tag(data.get(*pos..)?)?;
    if !tag.context || tag.number != number || tag.opening || tag.closing {
        return None;
    }
    let content = data.get(*pos + header..*pos + header + tag.len)?;
    *pos += header + tag.len;
    Some(unsigned(content) as u32)
}

async fn open_socket(target: SocketAddr) -> Result<UdpSocket, String> {
    let bind: SocketAddr = if target.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" }.parse().unwrap();
    UdpSocket::bind(bind).await.map_err(|e| format!("BACnet socket: {e}"))
}

/// Send one confirmed request and wait for its answer, resending after a timeout. Returns the service data of a
/// ComplexAck (segments reassembled) or an empty vector for a SimpleAck. Other traffic from the device is skipped.
async fn confirmed_request(
    socket: &UdpSocket,
    target: SocketAddr,
    service: u8,
    body: &[u8],
    apdu_timeout: Duration,
    retries: u32,
) -> Result<Vec<u8>, String> {
    let invoke_id = NEXT_INVOKE_ID.fetch_add(1, Ordering::Relaxed);
    let request = encode_confirmed(invoke_id, service, body);
    let mut buf = vec![0u8; MAX_FRAME];
    for _ in 0..=retries {
        socket.send_to(&request, target).await.map_err(|e| format!("BACnet send to {target}: {e}"))?;
        let mut state = Reassembly::default();
        let deadline = Instant::now() + apdu_timeout;
        loop {
            let (n, from) = match tokio::time::timeout_at(deadline, socket.recv_from(&mut buf)).await {
                Ok(Ok(got)) => got,
                Ok(Err(e)) => return Err(format!("BACnet receive from {target}: {e}")),
                Err(_) => break,
            };
            if from.ip() != target.ip() {
                continue;
            }
            let Some(apdu) = apdu_of_frame(&buf[..n]) else { continue };
            match handle_apdu(apdu, invoke_id, &mut state) {
                Reply::Ignore => {}
                Reply::Error(e) => return Err(e),
                Reply::SegmentAck(ack) => {
                    socket
                        .send_to(&frame(&ack, false), from)
                        .await
                        .map_err(|e| format!("BACnet send to {target}: {e}"))?;
                }
                Reply::Data(data) => {
                    if apdu[0] & 0x08 != 0 {
                        // Acknowledge the final segment (window size copied from the segment).
                        let ack = [0x40, invoke_id, apdu[2], apdu[3]];
                        let _ = socket.send_to(&frame(&ack, false), from).await;
                    }
                    return Ok(data);
                }
            }
        }
    }
    Err(format!("BACnet timeout waiting for {target} ({} ms, {retries} retries)", apdu_timeout.as_millis()))
}

/// Read one property of one object from the BACnet/IP device at `target`.
pub(crate) async fn read_property(
    target: SocketAddr,
    object_type: u32,
    instance: u32,
    property: u32,
    apdu_timeout: Duration,
    retries: u32,
) -> Result<Value, String> {
    let socket = open_socket(target).await?;
    let body = read_property_body(object_type, instance, property);
    let data = confirmed_request(&socket, target, SERVICE_READ_PROPERTY, &body, apdu_timeout, retries).await?;
    decode_read_property_ack(&data, object_type, instance, property)
}

/// Write one property (WriteProperty). `priority` 1-16 selects the slot of a commandable property; writing
/// [`WriteValue::Null`] at a priority relinquishes that slot.
pub(crate) async fn write_property(
    target: SocketAddr,
    object_type: u32,
    instance: u32,
    property: u32,
    value: &WriteValue,
    priority: Option<u8>,
    apdu_timeout: Duration,
    retries: u32,
) -> Result<(), String> {
    let socket = open_socket(target).await?;
    let body = write_property_body(object_type, instance, property, value, priority);
    confirmed_request(&socket, target, SERVICE_WRITE_PROPERTY, &body, apdu_timeout, retries).await.map(|_| ())
}

/// One COV notification: the properties the device reported for the monitored object.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CovEvent {
    pub process_id: u32,
    pub object_type: u32,
    pub instance: u32,
    pub time_remaining: u32,
    pub values: Vec<(u32, Value)>,
}

fn subscribe_cov_body(process_id: u32, object_type: u32, instance: u32, confirmed: bool, lifetime: u32) -> Vec<u8> {
    let mut body = Vec::new();
    push_context_unsigned(&mut body, 0, process_id);
    push_object_id(&mut body, 1, object_type, instance);
    push_tag(&mut body, 2, true, 1);
    body.push(confirmed as u8);
    push_context_unsigned(&mut body, 3, lifetime);
    body
}

fn read_context_object(data: &[u8], pos: &mut usize, number: u32) -> Option<(u32, u32)> {
    let (tag, header) = parse_tag(data.get(*pos..)?)?;
    if !tag.context || tag.number != number || tag.len != 4 {
        return None;
    }
    let raw = u32::from_be_bytes(data.get(*pos + header..*pos + header + 4)?.try_into().ok()?);
    *pos += header + 4;
    Some((raw >> 22, raw & 0x3F_FFFF))
}

fn expect_context(data: &[u8], pos: &mut usize, number: u32, opening: bool) -> Option<()> {
    let (tag, header) = parse_tag(data.get(*pos..)?)?;
    if !tag.context || tag.number != number || tag.opening != opening || tag.closing == opening {
        return None;
    }
    *pos += header;
    Some(())
}

/// Decode the service data of a Confirmed or Unconfirmed COV notification (clause 13.8 and 13.9).
fn decode_cov_notification(data: &[u8]) -> Option<CovEvent> {
    let mut pos = 0;
    let process_id = read_context_unsigned(data, &mut pos, 0)?;
    read_context_object(data, &mut pos, 1)?;
    let (object_type, instance) = read_context_object(data, &mut pos, 2)?;
    let time_remaining = read_context_unsigned(data, &mut pos, 3)?;
    expect_context(data, &mut pos, 4, true)?;
    let mut values = Vec::new();
    while let Some(property) = read_context_unsigned(data, &mut pos, 0) {
        let _ = read_context_unsigned(data, &mut pos, 1);
        expect_context(data, &mut pos, 2, true)?;
        let mut decoded = decode_sequence(data, &mut pos, false).ok()?;
        expect_context(data, &mut pos, 2, false)?;
        let _ = read_context_unsigned(data, &mut pos, 3);
        values.push((property, if decoded.len() == 1 { decoded.remove(0) } else { Value::Array(decoded) }));
    }
    expect_context(data, &mut pos, 4, false)?;
    Some(CovEvent { process_id, object_type, instance, time_remaining, values })
}

/// What a received frame means to a COV subscriber.
#[derive(Debug, PartialEq)]
enum CovFrame {
    /// A notification; `ack` is the SimpleAck to send back when the device asked for confirmation.
    Notification(CovEvent, Option<Vec<u8>>),
    Other,
}

fn classify_cov_frame(apdu: &[u8]) -> CovFrame {
    match apdu.first().map(|b| b >> 4) {
        // UnconfirmedCOVNotification: type, service 2.
        Some(1) if apdu.get(1) == Some(&2) => {
            decode_cov_notification(&apdu[2..]).map(|e| CovFrame::Notification(e, None)).unwrap_or(CovFrame::Other)
        }
        // ConfirmedCOVNotification: type, max-apdu, invoke id, service 1.
        Some(0) if apdu.len() > 4 && apdu[3] == 1 => decode_cov_notification(&apdu[4..])
            .map(|e| CovFrame::Notification(e, Some(vec![0x20, apdu[2], 1])))
            .unwrap_or(CovFrame::Other),
        _ => CovFrame::Other,
    }
}

/// A COV subscription held on one UDP socket: the device sends notifications to the socket the subscription came from.
pub(crate) struct CovSubscription {
    socket: UdpSocket,
    target: SocketAddr,
    process_id: u32,
    object_type: u32,
    instance: u32,
    buf: Vec<u8>,
}

impl CovSubscription {
    pub(crate) async fn open(
        target: SocketAddr,
        process_id: u32,
        object_type: u32,
        instance: u32,
    ) -> Result<Self, String> {
        Ok(Self {
            socket: open_socket(target).await?,
            target,
            process_id,
            object_type,
            instance,
            buf: vec![0u8; MAX_FRAME],
        })
    }

    /// Subscribe, or renew when already subscribed. A lifetime of 0 asks for an indefinite subscription.
    pub(crate) async fn subscribe(&self, lifetime: u32, apdu_timeout: Duration, retries: u32) -> Result<(), String> {
        let body = subscribe_cov_body(self.process_id, self.object_type, self.instance, false, lifetime);
        confirmed_request(&self.socket, self.target, SERVICE_SUBSCRIBE_COV, &body, apdu_timeout, retries)
            .await
            .map(|_| ())
    }

    /// Cancel the subscription (SubscribeCOV without confirmation flag and lifetime).
    pub(crate) async fn cancel(&self, apdu_timeout: Duration) {
        let mut body = Vec::new();
        push_context_unsigned(&mut body, 0, self.process_id);
        push_object_id(&mut body, 1, self.object_type, self.instance);
        let _ = confirmed_request(&self.socket, self.target, SERVICE_SUBSCRIBE_COV, &body, apdu_timeout, 0).await;
    }

    /// Wait for the next notification of this subscription until `deadline`; `None` when the deadline passes.
    pub(crate) async fn next(&mut self, deadline: Instant) -> Result<Option<CovEvent>, String> {
        loop {
            let (n, from) = match tokio::time::timeout_at(deadline, self.socket.recv_from(&mut self.buf)).await {
                Ok(Ok(got)) => got,
                Ok(Err(e)) => return Err(format!("BACnet receive from {}: {e}", self.target)),
                Err(_) => return Ok(None),
            };
            if from.ip() != self.target.ip() {
                continue;
            }
            let Some(apdu) = apdu_of_frame(&self.buf[..n]) else { continue };
            if let CovFrame::Notification(event, ack) = classify_cov_frame(apdu) {
                if let Some(ack) = ack {
                    let _ = self.socket.send_to(&frame(&ack, false), from).await;
                }
                if event.process_id == self.process_id
                    && event.object_type == self.object_type
                    && event.instance == self.instance
                {
                    return Ok(Some(event));
                }
            }
        }
    }
}

/// Property name for a property number, or the number as text.
pub(crate) fn property_name(id: u32) -> String {
    PROPERTIES.iter().find(|(_, i)| *i == id).map(|(n, _)| (*n).to_string()).unwrap_or_else(|| id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_read_property_like_the_standard_example() {
        // ASHRAE 135 Annex F.3.1: ReadProperty of analog-input,5 present-value.
        let frame = encode_read_property(1, 0, 5, 85);
        assert_eq!(
            frame,
            vec![0x81, 0x0A, 0x00, 0x11, 0x01, 0x04, 0x02, 0x75, 0x01, 0x0C, 0x0C, 0x00, 0x00, 0x00, 0x05, 0x19, 0x55]
        );
    }

    #[test]
    fn decodes_the_annex_f_ack() {
        // ComplexAck data of Annex F.3.1: analog-input,5 present-value = 72.1 (Real).
        let data = [0x0C, 0x00, 0x00, 0x00, 0x05, 0x19, 0x55, 0x3E, 0x44, 0x42, 0x90, 0x33, 0x33, 0x3F];
        let v = decode_read_property_ack(&data, 0, 5, 85).unwrap();
        assert!((v.as_f64().unwrap() - 72.1).abs() < 1e-4);
    }

    #[test]
    fn decodes_values_by_type() {
        let mut pos = 0;
        let bytes = [
            0x91, 0x01, // enumerated 1
            0x21, 0x2A, // unsigned 42
            0x11, // boolean true
            0x75, 0x04, 0x00, b'h', b'i', b'!', // string "hi!"
            0x82, 0x04, 0x20, // bit string, 4 bits unused 4: 0010 -> [false,false,true,false]
            0xC4, 0x00, 0x80, 0x00, 0x07, // object id analog-value,7
        ];
        let values = decode_sequence(&bytes, &mut pos, true).unwrap();
        assert_eq!(
            values,
            vec![
                json!(1),
                json!(42),
                json!(true),
                json!("hi!"),
                json!([false, false, true, false]),
                json!({"objectType": "analogValue", "instance": 7})
            ]
        );
    }

    #[test]
    fn decodes_constructed_values() {
        let mut pos = 0;
        let inner = [0x4E, 0x21, 0x01, 0x21, 0x02, 0x4F];
        let values = decode_sequence(&inner, &mut pos, true).unwrap();
        assert_eq!(values, vec![json!([1, 2])]);
    }

    #[test]
    fn reports_device_errors() {
        let mut state = Reassembly::default();
        // Error PDU: unknown-object (class object=1, code 31).
        let apdu = [0x50, 7, 0x0C, 0x91, 0x01, 0x91, 0x1F];
        assert_eq!(
            handle_apdu(&apdu, 7, &mut state),
            Reply::Error("BACnet error from device: object / unknown-object".into())
        );
        assert_eq!(handle_apdu(&apdu, 8, &mut state), Reply::Ignore);
    }

    #[test]
    fn reassembles_segments() {
        let mut state = Reassembly::default();
        let first = [0x3C, 9, 0, 4, 0x0C, 0xAA, 0xBB];
        assert_eq!(handle_apdu(&first, 9, &mut state), Reply::SegmentAck(vec![0x40, 9, 0, 4]));
        let second = [0x38, 9, 1, 4, 0xCC];
        assert_eq!(handle_apdu(&second, 9, &mut state), Reply::Data(vec![0xAA, 0xBB, 0xCC]));
    }

    #[test]
    fn maps_names() {
        assert_eq!(object_type_id("analogInput"), Some(0));
        assert_eq!(object_type_id("multi-state-value"), Some(19));
        assert_eq!(property_id("present_value"), Some(85));
        assert_eq!(property_id("77"), Some(77));
        assert_eq!(property_id("nope"), None);
    }

    #[test]
    fn encodes_write_property() {
        // analog-output 1, present-value, Real 180.0 at priority 8.
        let body = write_property_body(1, 1, 85, &WriteValue::Real(180.0), Some(8));
        assert_eq!(
            body,
            vec![0x0C, 0x00, 0x40, 0x00, 0x01, 0x19, 0x55, 0x3E, 0x44, 0x43, 0x34, 0x00, 0x00, 0x3F, 0x49, 0x08]
        );
        // Relinquish: Null at priority 16, no priority when none is given.
        let body = write_property_body(1, 1, 85, &WriteValue::Null, Some(16));
        assert_eq!(&body[7..], &[0x3E, 0x00, 0x3F, 0x49, 0x10]);
        assert_eq!(&write_property_body(1, 1, 85, &WriteValue::Null, None)[7..], &[0x3E, 0x00, 0x3F]);
    }

    #[test]
    fn encodes_application_values_that_decode_back() {
        let values = [
            WriteValue::Boolean(true),
            WriteValue::Unsigned(0),
            WriteValue::Unsigned(70000),
            WriteValue::Signed(-1),
            WriteValue::Signed(-300),
            WriteValue::Signed(127),
            WriteValue::Signed(128),
            WriteValue::Double(2.5),
            WriteValue::Enumerated(1),
            WriteValue::String("Comfort".into()),
            WriteValue::String("a longer string than four".into()),
        ];
        let want = [
            json!(true),
            json!(0),
            json!(70000),
            json!(-1),
            json!(-300),
            json!(127),
            json!(128),
            json!(2.5),
            json!(1),
            json!("Comfort"),
            json!("a longer string than four"),
        ];
        for (value, want) in values.iter().zip(want) {
            let mut bytes = Vec::new();
            push_application(&mut bytes, value);
            let mut pos = 0;
            assert_eq!(decode_sequence(&bytes, &mut pos, true).unwrap(), vec![want], "{value:?}");
        }
    }

    #[test]
    fn chooses_the_value_type_from_the_object() {
        let w = |ot, prop, v: Value, hint: &str| write_value(ot, prop, &v, hint);
        assert_eq!(w(1, 85, json!(72.5), "auto"), Ok(WriteValue::Real(72.5)));
        assert_eq!(w(4, 85, json!(true), ""), Ok(WriteValue::Enumerated(1)));
        assert_eq!(w(4, 85, json!("inactive"), "auto"), Ok(WriteValue::Enumerated(0)));
        assert_eq!(w(19, 85, json!(3), "auto"), Ok(WriteValue::Unsigned(3)));
        assert_eq!(w(19, 85, json!(1.5), "auto"), Err("cannot write 1.5 as Unsigned".into()));
        assert_eq!(w(1, 85, Value::Null, "auto"), Ok(WriteValue::Null));
        assert_eq!(w(0, 28, json!("text"), "auto"), Ok(WriteValue::String("text".into())));
        assert_eq!(w(2, 81, json!(true), "auto"), Ok(WriteValue::Boolean(true)));
        assert_eq!(w(2, 85, json!(5), "double"), Ok(WriteValue::Double(5.0)));
        assert!(w(2, 85, json!("x"), "real").is_err());
        assert!(w(2, 85, json!(1), "nope").is_err());
    }

    #[test]
    fn encodes_subscribe_cov() {
        assert_eq!(
            subscribe_cov_body(1, 0, 1, false, 60),
            vec![0x09, 0x01, 0x1C, 0x00, 0x00, 0x00, 0x01, 0x29, 0x00, 0x39, 0x3C]
        );
    }

    #[test]
    fn decodes_cov_notifications() {
        let data = [
            0x09, 0x01, 0x1C, 0x02, 0x00, 0x00, 0x05, 0x2C, 0x00, 0x00, 0x00, 0x01, 0x39, 0x3C, 0x4E, 0x09, 0x55, 0x2E,
            0x44, 0x42, 0x90, 0x00, 0x00, 0x2F, 0x09, 0x6F, 0x2E, 0x82, 0x04, 0x00, 0x2F, 0x4F,
        ];
        let want = CovEvent {
            process_id: 1,
            object_type: 0,
            instance: 1,
            time_remaining: 60,
            values: vec![(85, json!(72.0)), (111, json!([false, false, false, false]))],
        };
        assert_eq!(decode_cov_notification(&data), Some(want.clone()));
        let mut unconfirmed = vec![0x10, 0x02];
        unconfirmed.extend_from_slice(&data);
        assert_eq!(classify_cov_frame(&unconfirmed), CovFrame::Notification(want.clone(), None));
        let mut confirmed = vec![0x00, 0x75, 9, 0x01];
        confirmed.extend_from_slice(&data);
        assert_eq!(classify_cov_frame(&confirmed), CovFrame::Notification(want, Some(vec![0x20, 9, 1])));
        assert_eq!(classify_cov_frame(&[0x10, 0x00, 1, 2]), CovFrame::Other);
        assert_eq!(property_name(85), "presentValue");
        assert_eq!(property_name(9999), "9999");
    }
}
