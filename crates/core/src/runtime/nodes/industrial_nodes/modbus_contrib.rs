//! Compatibility with `node-red-contrib-modbus`.
//!
//! The contrib package has its own node types and property names. Flows exported from it are
//! served by the nodes in this folder:
//!
//! | contrib type        | served by                 |
//! |---------------------|---------------------------|
//! | `modbus-client`     | `modbus-config`           |
//! | `modbus-read`       | `modbus read`             |
//! | `modbus-getter`     | `modbus read`, no polling |
//! | `modbus-write`      | `modbus write`            |
//! | `modbus-flex-getter`| `modbus-flex-getter`      |
//! | `modbus-flex-write` | `modbus-flex-writer`      |
//!
//! This module translates the contrib properties into the ones our nodes read, and builds the
//! output messages in the shape the contrib nodes use: two outputs, the first with
//! `msg.payload` holding the values and `msg.responseBuffer` the raw response, the second with
//! `msg.payload` holding the raw response and `msg.values` the values.

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::modbus_config::ModbusRaw;
use crate::runtime::model::*;
use crate::runtime::nodes::*;

/// Properties shared by the nodes that understand contrib flows.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ContribOptions {
    /// The node was read from a contrib flow: use the contrib message shapes.
    #[serde(default)]
    pub(crate) contrib: bool,
    /// Unit (slave) id for this node, instead of the one of the client config.
    #[serde(default, rename = "unitId")]
    pub(crate) unit_id: Option<u8>,
    #[serde(default)]
    pub(crate) topic: Option<String>,
    #[serde(default, rename = "emptyMsgOnFail")]
    pub(crate) empty_msg_on_fail: bool,
    #[serde(default, rename = "keepMsgProperties")]
    pub(crate) keep_msg_properties: bool,
    #[serde(default, rename = "delayOnStart")]
    pub(crate) delay_on_start: bool,
    #[serde(default, rename = "startDelayTime")]
    pub(crate) start_delay_time: Option<u64>,
}

/// Contrib nodes name their client config in `server`. Ours use `configNode`.
pub(crate) fn is_contrib(rest: &Value) -> bool {
    rest.get("server").is_some()
}

/// A property that holds a number, which the editor often saves as a string.
fn number(rest: &Value, key: &str) -> crate::Result<Option<i64>> {
    match rest.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_i64()
            .or_else(|| n.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64))
            .map(Some)
            .ok_or_else(|| anyhow::anyhow!("'{key}' must be a whole number, got {n}")),
        Some(Value::String(s)) if s.trim().is_empty() => Ok(None),
        Some(Value::String(s)) => {
            s.trim().parse::<i64>().map(Some).map_err(|_| anyhow::anyhow!("'{key}' must be a whole number, got '{s}'"))
        }
        Some(other) => Err(anyhow::anyhow!("'{key}' must be a whole number, got {other}")),
    }
}

fn text(rest: &Value, key: &str) -> Option<String> {
    match rest.get(key) {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

fn flag(rest: &Value, key: &str) -> bool {
    match rest.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s == "true",
        _ => false,
    }
}

/// Map the properties of a `modbus-client` onto the ones of `modbus-config`.
pub(crate) fn translate_client(rest: &Value) -> crate::Result<Value> {
    let client_type = text(rest, "clienttype").unwrap_or_else(|| "tcp".to_string());
    let transport = match client_type.as_str() {
        "tcp" => "tcp",
        "serial" | "simpleser" | "rtu" => "serial",
        other => {
            return Err(anyhow::anyhow!("Modbus client type '{other}' is not supported, only tcp and serial are"));
        }
    };
    if transport == "serial" && text(rest, "serialType").is_some_and(|t| t.to_ascii_uppercase().starts_with("ASCII")) {
        return Err(anyhow::anyhow!("Modbus ASCII serial framing is not supported, use RTU"));
    }

    let mut out = Map::new();
    out.insert("transport".into(), json!(transport));
    out.insert("host".into(), json!(text(rest, "tcpHost").unwrap_or_else(|| "127.0.0.1".to_string())));
    out.insert("port".into(), json!(number(rest, "tcpPort")?.unwrap_or(502)));
    out.insert("unitId".into(), json!(number(rest, "unit_id")?.unwrap_or(1)));
    out.insert("timeoutMs".into(), json!(number(rest, "clientTimeout")?.filter(|t| *t > 0).unwrap_or(1000)));
    if let Some(port) = text(rest, "serialPort") {
        out.insert("serialPort".into(), json!(port));
    }
    if let Some(baud) = number(rest, "serialBaudrate")? {
        out.insert("baudRate".into(), json!(baud));
    }
    for (from, to) in [("serialDatabits", "dataBits"), ("serialStopbits", "stopBits"), ("serialParity", "parity")] {
        if let Some(value) = text(rest, from) {
            out.insert(to.into(), json!(value));
        }
    }
    out.insert(
        "parallelUnitIds".into(),
        json!(rest.get("parallelUnitIdsAllowed").is_none_or(|_| flag(rest, "parallelUnitIdsAllowed"))),
    );
    out.insert("queueLogEnabled".into(), json!(flag(rest, "queueLogEnabled")));
    out.insert("bufferCommands".into(), json!(rest.get("bufferCommands").is_none_or(|_| flag(rest, "bufferCommands"))));
    if let Some(delay) = number(rest, "commandDelay")?.filter(|d| *d >= 0) {
        out.insert("commandDelay".into(), json!(delay));
    }
    if rest.get("reconnectOnTimeout").is_none_or(|_| flag(rest, "reconnectOnTimeout"))
        && let Some(delay) = number(rest, "reconnectTimeout")?.filter(|d| *d >= 0)
    {
        out.insert("reconnectTimeout".into(), json!(delay));
    }
    out.insert("autoConnect".into(), json!(true));
    Ok(Value::Object(out))
}

/// Function code of a contrib read `dataType`.
fn read_function(data_type: &str) -> crate::Result<(&'static str, u8)> {
    match data_type {
        "Coil" => Ok(("readCoils", 1)),
        "Input" => Ok(("readDiscreteInputs", 2)),
        "HoldingRegister" => Ok(("readHoldingRegisters", 3)),
        "InputRegister" => Ok(("readInputRegisters", 4)),
        other => Err(anyhow::anyhow!("Unknown Modbus read type '{other}'")),
    }
}

/// Function code of a contrib write `dataType`.
fn write_function(data_type: &str) -> crate::Result<(&'static str, u8)> {
    match data_type {
        "Coil" => Ok(("writeSingleCoil", 5)),
        "HoldingRegister" => Ok(("writeSingleRegister", 6)),
        "MCoils" => Ok(("writeMultipleCoils", 15)),
        "MHoldingRegisters" => Ok(("writeMultipleRegisters", 16)),
        other => Err(anyhow::anyhow!("Unknown Modbus write type '{other}'")),
    }
}

/// Function code number for the names our nodes use.
pub(crate) fn function_code(name: &str) -> u8 {
    match name {
        "readCoils" => 1,
        "readDiscreteInputs" => 2,
        "readInputRegisters" => 4,
        "writeSingleCoil" => 5,
        "writeSingleRegister" => 6,
        "writeMultipleCoils" => 15,
        "writeMultipleRegisters" => 16,
        _ => 3,
    }
}

fn common_options(rest: &Value, out: &mut Map<String, Value>) -> crate::Result<()> {
    out.insert("contrib".into(), json!(true));
    out.insert("configNode".into(), json!(text(rest, "server").unwrap_or_default()));
    if let Some(unit) = number(rest, "unitid")? {
        out.insert("unitId".into(), json!(unit));
    }
    if let Some(topic) = text(rest, "topic") {
        out.insert("topic".into(), json!(topic));
    }
    for key in ["emptyMsgOnFail", "keepMsgProperties", "delayOnStart"] {
        out.insert(key.into(), json!(flag(rest, key)));
    }
    if let Some(delay) = number(rest, "startDelayTime")?.filter(|d| *d >= 0) {
        out.insert("startDelayTime".into(), json!(delay));
    }
    Ok(())
}

/// `modbus-read` (with `polling`) and `modbus-getter` (without).
pub(crate) fn translate_read(rest: &Value, polling: bool) -> crate::Result<Value> {
    let mut out = Map::new();
    common_options(rest, &mut out)?;
    let (name, _) = read_function(text(rest, "dataType").as_deref().unwrap_or(""))?;
    out.insert("functionCode".into(), json!(name));
    out.insert("address".into(), json!(number(rest, "adr")?.unwrap_or(0)));
    out.insert("quantity".into(), json!(number(rest, "quantity")?.unwrap_or(1)));
    if polling && let Some(rate) = number(rest, "rate")?.filter(|r| *r > 0) {
        let unit = match text(rest, "rateUnit").as_deref() {
            Some("s") => "s",
            Some("m") => "min",
            Some("h") => "hr",
            _ => "ms",
        };
        out.insert("pollRate".into(), json!(rate));
        out.insert("pollRateUnit".into(), json!(unit));
    }
    Ok(Value::Object(out))
}

/// `modbus-write`.
pub(crate) fn translate_write(rest: &Value) -> crate::Result<Value> {
    let mut out = Map::new();
    common_options(rest, &mut out)?;
    let (name, _) = write_function(text(rest, "dataType").as_deref().unwrap_or(""))?;
    out.insert("functionCode".into(), json!(name));
    out.insert("address".into(), json!(number(rest, "adr")?.unwrap_or(0)));
    out.insert("quantity".into(), json!(number(rest, "quantity")?.unwrap_or(1)));
    Ok(Value::Object(out))
}

/// `modbus-flex-getter` and `modbus-flex-write`: everything comes with the message.
pub(crate) fn translate_flex(rest: &Value) -> crate::Result<Value> {
    let mut out = Map::new();
    common_options(rest, &mut out)?;
    Ok(Value::Object(out))
}

fn message_id() -> String {
    format!("{:024x}", rand::random::<u128>() & ((1u128 << 96) - 1))
}

/// The request as the contrib nodes put it in `msg.modbusRequest`.
pub(crate) fn request_object(
    unit: Option<u8>,
    fc: u8,
    address: u16,
    quantity: u16,
    value: Option<&Variant>,
) -> Variant {
    let mut map = std::collections::BTreeMap::from([
        ("fc".to_string(), Variant::from(fc as i64)),
        ("address".to_string(), Variant::from(address as i64)),
        ("quantity".to_string(), Variant::from(quantity as i64)),
        ("messageId".to_string(), Variant::String(message_id())),
    ]);
    if let Some(unit) = unit {
        map.insert("unitid".to_string(), Variant::from(unit as i64));
    }
    if let Some(value) = value {
        map.insert("value".to_string(), value.clone());
    }
    Variant::Object(map)
}

/// The raw response, as `msg.responseBuffer`: `{ data: [...], buffer: <bytes> }`.
pub(crate) fn response_object(raw: &ModbusRaw) -> Variant {
    Variant::from([("data", raw.to_variant()), ("buffer", Variant::Bytes(raw.to_bytes()))])
}

/// Both outputs of a read.
///
/// `base` is the incoming message when `keepMsgProperties` is on, otherwise an empty one.
/// Output 1 carries the values in `msg.payload`. Output 2 carries the raw response in
/// `msg.payload` and the values in `msg.values`.
pub(crate) fn read_outputs(base: Msg, topic: &str, raw: &ModbusRaw, request: Variant) -> (Msg, Msg) {
    let values = raw.to_variant();
    let response = response_object(raw);

    let mut first = base;
    first.set("modbusRequest".to_string(), request);
    first.set("payload".to_string(), values.clone());
    first.set("topic".to_string(), Variant::String(topic.to_string()));
    first.set("responseBuffer".to_string(), response.clone());

    let mut second = first.clone();
    second.set("payload".to_string(), response);
    second.set("values".to_string(), values);
    second.remove("responseBuffer");
    (first, second)
}

/// Both outputs of a write. Output 1 carries the request in `msg.payload`, output 2 the
/// acknowledgement of the device in `msg.payload` and the request in `msg.values`.
pub(crate) fn write_outputs(base: Msg, topic: &str, request: Variant, ack: Variant) -> (Msg, Msg) {
    let mut first = base;
    first.set("payload".to_string(), request.clone());
    first.set("topic".to_string(), Variant::String(topic.to_string()));

    let mut second = first.clone();
    second.set("payload".to_string(), ack);
    second.set("values".to_string(), request);
    (first, second)
}

/// The messages sent on a failure when `emptyMsgOnFail` is on: an empty payload and the error.
pub(crate) fn failure_outputs(mut msg: Msg, error: &anyhow::Error) -> (Msg, Msg) {
    msg.set("payload".to_string(), Variant::String(String::new()));
    msg.set(
        "error".to_string(),
        Variant::from([
            ("message", Variant::String(error.to_string())),
            ("name", Variant::String("Error".to_string())),
        ]),
    );
    (msg.clone(), msg)
}

/// Send `first` on output 1 and `second` on output 2, when the node has a second output wired.
pub(crate) async fn send_pair<N: FlowNodeBehavior + ?Sized>(
    node: &N,
    first: Msg,
    second: Msg,
    cancel: CancellationToken,
) -> crate::Result<()> {
    node.fan_out_one(Envelope { port: 0, msg: MsgHandle::new(first) }, cancel.clone()).await?;
    if node.get_base().ports.len() > 1 {
        node.fan_out_one(Envelope { port: 1, msg: MsgHandle::new(second) }, cancel).await?;
    }
    Ok(())
}

/// What a flex node asks for: `msg.payload` as an object, or as JSON text.
#[derive(Debug, Clone)]
pub(crate) struct FlexRequest {
    pub(crate) unit: Option<u8>,
    pub(crate) fc: u8,
    pub(crate) address: u16,
    pub(crate) quantity: u16,
    pub(crate) value: Option<Variant>,
}

impl FlexRequest {
    /// Parse `msg.payload` of a flex node. `valid_fcs` lists the function codes the node accepts.
    /// Missing `fc`, `address` and `quantity` default to 3, 0 and 1 for reads, as in the contrib nodes.
    pub(crate) fn parse(payload: &Variant, valid_fcs: &[u8], defaults: bool) -> crate::Result<Self> {
        let json: Value = match payload {
            Variant::String(s) => {
                serde_json::from_str(s).map_err(|e| anyhow::anyhow!("msg.payload is not valid JSON: {e}"))?
            }
            other => serde_json::to_value(other)?,
        };
        let Value::Object(object) = &json else {
            return Err(anyhow::anyhow!("msg.payload must be an object with fc, address and quantity"));
        };

        let field = |key: &str| -> crate::Result<Option<i64>> { number(&json, key) };
        let fc = field("fc")?.or(defaults.then_some(3));
        let fc = fc
            .filter(|fc| valid_fcs.iter().any(|v| i64::from(*v) == *fc))
            .ok_or_else(|| anyhow::anyhow!("FC Not Valid: use one of {valid_fcs:?}"))? as u8;
        let address = field("address")?.or(defaults.then_some(0)).unwrap_or(-1);
        let address = u16::try_from(address).map_err(|_| anyhow::anyhow!("Address Not Valid: 0 to 65535"))?;
        let quantity = field("quantity")?.or(defaults.then_some(1)).unwrap_or(-1);
        let quantity = u16::try_from(quantity)
            .ok()
            .filter(|q| *q >= 1)
            .ok_or_else(|| anyhow::anyhow!("Quantity Not Valid: 1 to 65535"))?;
        let unit = field("unitid")?
            .map(u8::try_from)
            .transpose()
            .map_err(|_| anyhow::anyhow!("Unit id Not Valid: 0 to 255"))?;

        // Values that arrive as text, for instance from an HTTP request: "true", "false", "[1,2,3]" or "1,2,3".
        let value = object.get("value").map(|v| match v {
            Value::String(s) if s == "true" || s == "false" => Variant::Bool(s == "true"),
            Value::String(s) if s.contains(',') => serde_json::from_str::<Value>(s)
                .or_else(|_| serde_json::from_str::<Value>(&format!("[{s}]")))
                .map(Variant::from)
                .unwrap_or_else(|_| Variant::String(s.clone())),
            other => Variant::from(other.clone()),
        });
        Ok(Self { unit, fc, address, quantity, value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_translation_reads_stock_tcp_properties() {
        let out = translate_client(&json!({
            "name": "plc", "clienttype": "tcp", "bufferCommands": true, "stateLogEnabled": false,
            "queueLogEnabled": false, "failureLogEnabled": true, "tcpHost": "10.0.0.5", "tcpPort": "1502",
            "tcpType": "DEFAULT", "serialPort": "/dev/ttyUSB", "serialType": "RTU-BUFFERD", "serialBaudrate": "9600",
            "serialDatabits": "8", "serialStopbits": "1", "serialParity": "none", "serialConnectionDelay": "100",
            "unit_id": "3", "commandDelay": "1", "clientTimeout": "2000", "reconnectOnTimeout": true,
            "reconnectTimeout": "5000", "parallelUnitIdsAllowed": true
        }))
        .unwrap();
        assert_eq!(out["transport"], "tcp");
        assert_eq!(out["host"], "10.0.0.5");
        assert_eq!(out["port"], 1502);
        assert_eq!(out["unitId"], 3);
        assert_eq!(out["timeoutMs"], 2000);
        assert_eq!(out["reconnectTimeout"], 5000);
        assert_eq!(out["baudRate"], 9600);
        assert_eq!(out["dataBits"], "8");
    }

    #[test]
    fn client_translation_rejects_unsupported_framing() {
        assert!(translate_client(&json!({"clienttype": "udp"})).is_err());
        assert!(translate_client(&json!({"clienttype": "serial", "serialType": "ASCII"})).is_err());
        assert!(translate_client(&json!({"clienttype": "serial", "serialType": "RTU"})).is_ok());
    }

    #[test]
    fn read_translation_maps_type_rate_and_unit() {
        let rest = json!({
            "server": "c1", "dataType": "InputRegister", "adr": "10", "quantity": "4",
            "rate": "2", "rateUnit": "m", "unitid": "7", "topic": "plc", "showErrors": false
        });
        let out = translate_read(&rest, true).unwrap();
        assert_eq!(out["functionCode"], "readInputRegisters");
        assert_eq!((out["address"].as_i64(), out["quantity"].as_i64()), (Some(10), Some(4)));
        assert_eq!(out["pollRate"], 2);
        assert_eq!(out["pollRateUnit"], "min");
        assert_eq!(out["unitId"], 7);
        assert_eq!(out["configNode"], "c1");
        assert!(translate_read(&rest, false).unwrap().get("pollRate").is_none());
        assert!(translate_read(&json!({"server": "c", "dataType": "Nope"}), true).is_err());
    }

    #[test]
    fn write_translation_maps_types() {
        for (data_type, name) in [
            ("Coil", "writeSingleCoil"),
            ("HoldingRegister", "writeSingleRegister"),
            ("MCoils", "writeMultipleCoils"),
            ("MHoldingRegisters", "writeMultipleRegisters"),
        ] {
            let out =
                translate_write(&json!({"server": "c", "dataType": data_type, "adr": "0", "quantity": "2"})).unwrap();
            assert_eq!(out["functionCode"], name);
        }
    }

    #[test]
    fn coil_bytes_are_packed_lowest_bit_first() {
        let raw = ModbusRaw::Bits(vec![true, false, true, true, false, false, false, false, true]);
        assert_eq!(raw.to_bytes(), vec![0b0000_1101, 0b0000_0001]);
        assert_eq!(ModbusRaw::Words(vec![0x0102, 0xFFFE]).to_bytes(), vec![1, 2, 0xFF, 0xFE]);
    }

    #[test]
    fn read_outputs_have_the_contrib_shape() {
        let raw = ModbusRaw::Words(vec![7, 8]);
        let (first, second) = read_outputs(Msg::default(), "polling", &raw, request_object(Some(1), 3, 0, 2, None));
        assert_eq!(first.get("payload"), Some(&Variant::from(serde_json::json!([7, 8]))));
        assert!(first.get("responseBuffer").is_some());
        assert!(first.get("modbusRequest").is_some());
        assert_eq!(first.get("topic"), Some(&Variant::String("polling".into())));
        assert_eq!(second.get("values"), Some(&Variant::from(serde_json::json!([7, 8]))));
        assert!(second.get("responseBuffer").is_none());
        assert!(matches!(second.get("payload"), Some(Variant::Object(_))));
    }

    #[test]
    fn flex_request_parses_object_and_json_text() {
        let payload = Variant::from(json!({"fc": 3, "unitid": "2", "address": "100", "quantity": 4}));
        let request = FlexRequest::parse(&payload, &[1, 2, 3, 4], true).unwrap();
        assert_eq!((request.unit, request.fc, request.address, request.quantity), (Some(2), 3, 100, 4));

        let text = Variant::String(r#"{"address": 5}"#.into());
        let request = FlexRequest::parse(&text, &[1, 2, 3, 4], true).unwrap();
        assert_eq!((request.fc, request.address, request.quantity), (3, 5, 1));

        assert!(FlexRequest::parse(&Variant::from(json!({"fc": 9})), &[1, 2, 3, 4], true).is_err());
        assert!(FlexRequest::parse(&Variant::from(json!({"fc": 6, "address": 1})), &[5, 6, 15, 16], false).is_err());
    }

    #[test]
    fn flex_request_converts_text_values() {
        let parse = |value: Value| {
            FlexRequest::parse(
                &Variant::from(json!({"fc": 15, "address": 0, "quantity": 2, "value": value})),
                &[15],
                false,
            )
            .unwrap()
            .value
        };
        assert_eq!(parse(json!("true")), Some(Variant::Bool(true)));
        assert_eq!(parse(json!("1,0")), Some(Variant::from(json!([1, 0]))));
        assert_eq!(parse(json!([true, false])), Some(Variant::from(json!([true, false]))));
    }
}
