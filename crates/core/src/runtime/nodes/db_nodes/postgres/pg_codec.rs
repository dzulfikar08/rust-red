//! Parameter binding and row decoding for the PostgreSQL query node.
//!
//! Parameters are bound by the type the server infers for each placeholder, so a JSON number
//! can go into an `int4`, `float8` or `numeric` column alike. Rows are decoded the way
//! `node-postgres` hands them to Node-RED: numbers for integers and floats, text for `numeric`,
//! ISO 8601 text for timestamps, parsed JSON for `json`/`jsonb`.

use std::collections::BTreeMap;
use std::error::Error;

use bytes::{BufMut, BytesMut};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde_json::Value;
use tokio_postgres::types::{FromSql, IsNull, ToSql, Type, to_sql_checked};

use crate::runtime::model::*;

type BoxError = Box<dyn Error + Sync + Send>;

// ---------------------------------------------------------------------------
// numeric
// ---------------------------------------------------------------------------

const NUMERIC_POS: u16 = 0x0000;
const NUMERIC_NEG: u16 = 0x4000;
const NUMERIC_NAN: u16 = 0xC000;

/// Encode a decimal text such as `-12.50`, `3`, `1e-3` or `NaN` in the PostgreSQL binary `numeric` format.
fn encode_numeric(text: &str, out: &mut BytesMut) -> Result<(), BoxError> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("nan") {
        out.put_i16(0);
        out.put_i16(0);
        out.put_u16(NUMERIC_NAN);
        out.put_u16(0);
        return Ok(());
    }

    let (negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (mantissa, exponent) = match unsigned.split_once(['e', 'E']) {
        Some((m, e)) => (m, e.parse::<i32>().map_err(|_| format!("invalid numeric exponent in '{text}'"))?),
        None => (unsigned, 0),
    };
    let (int_digits, frac_digits) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if int_digits.is_empty() && frac_digits.is_empty()
        || !int_digits.bytes().all(|b| b.is_ascii_digit())
        || !frac_digits.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(format!("invalid numeric value '{text}'").into());
    }

    // Move the decimal point by the exponent.
    let mut int_part = int_digits.to_string();
    let mut frac_part = frac_digits.to_string();
    if exponent > 0 {
        let shift = exponent as usize;
        while frac_part.len() < shift {
            frac_part.push('0');
        }
        int_part.push_str(&frac_part[..shift]);
        frac_part = frac_part[shift..].to_string();
    } else if exponent < 0 {
        let shift = exponent.unsigned_abs() as usize;
        while int_part.len() < shift {
            int_part.insert(0, '0');
        }
        let split = int_part.len() - shift;
        frac_part = format!("{}{}", &int_part[split..], frac_part);
        int_part.truncate(split);
    }

    let int_part = int_part.trim_start_matches('0').to_string();
    let dscale = frac_part.len();

    // Group into base 10000 digits: integer part aligned on the right, fraction on the left.
    let mut padded_int = int_part;
    while !padded_int.len().is_multiple_of(4) {
        padded_int.insert(0, '0');
    }
    let mut padded_frac = frac_part;
    while !padded_frac.len().is_multiple_of(4) {
        padded_frac.push('0');
    }
    let mut groups: Vec<i16> = padded_int
        .as_bytes()
        .chunks(4)
        .chain(padded_frac.as_bytes().chunks(4))
        .map(|chunk| std::str::from_utf8(chunk).unwrap().parse::<i16>().unwrap())
        .collect();
    let int_groups = padded_int.len() / 4;

    let mut weight = int_groups as i32 - 1;
    // Drop leading zero groups (they only shift the weight) and trailing ones.
    let leading = groups.iter().take_while(|g| **g == 0).count();
    groups.drain(..leading);
    weight -= leading as i32;
    while groups.last() == Some(&0) {
        groups.pop();
    }
    if groups.is_empty() {
        weight = 0;
    }

    let sign = if negative && !groups.is_empty() { NUMERIC_NEG } else { NUMERIC_POS };
    out.put_i16(groups.len() as i16);
    out.put_i16(weight as i16);
    out.put_u16(sign);
    out.put_u16(dscale as u16);
    for group in groups {
        out.put_i16(group);
    }
    Ok(())
}

/// Decode a binary `numeric` into its decimal text.
fn decode_numeric(raw: &[u8]) -> Result<String, BoxError> {
    if raw.len() < 8 {
        return Err("numeric value is too short".into());
    }
    let word = |i: usize| i16::from_be_bytes([raw[i], raw[i + 1]]);
    let ndigits = word(0) as usize;
    let weight = word(2) as i32;
    let sign = u16::from_be_bytes([raw[4], raw[5]]);
    let dscale = u16::from_be_bytes([raw[6], raw[7]]) as usize;
    if raw.len() < 8 + ndigits * 2 {
        return Err("numeric value is truncated".into());
    }
    if sign == NUMERIC_NAN {
        return Ok("NaN".to_string());
    }
    let digits: Vec<i16> = (0..ndigits).map(|i| word(8 + i * 2)).collect();

    // digits[k] has the weight `weight - k`, so the integer groups are digits[0..=weight].
    let mut int_text = String::new();
    if weight >= 0 {
        for k in 0..=weight as usize {
            let group = digits.get(k).copied().unwrap_or(0);
            if k == 0 {
                int_text.push_str(&group.to_string());
            } else {
                int_text.push_str(&format!("{group:04}"));
            }
        }
    } else {
        int_text.push('0');
    }

    // Fraction groups start at weight -1.
    let mut frac_text = String::new();
    let mut w = -1;
    while frac_text.len() < dscale {
        let digit_index = weight - w;
        let group = if digit_index >= 0 { digits.get(digit_index as usize).copied().unwrap_or(0) } else { 0 };
        frac_text.push_str(&format!("{group:04}"));
        w -= 1;
    }
    frac_text.truncate(dscale);

    let mut text = String::new();
    if sign == NUMERIC_NEG {
        text.push('-');
    }
    text.push_str(&int_text);
    if dscale > 0 {
        text.push('.');
        text.push_str(&frac_text);
    }
    Ok(text)
}

struct PgNumeric(String);

impl<'a> FromSql<'a> for PgNumeric {
    fn from_sql(_: &Type, raw: &'a [u8]) -> Result<Self, BoxError> {
        decode_numeric(raw).map(PgNumeric)
    }

    fn accepts(ty: &Type) -> bool {
        *ty == Type::NUMERIC
    }
}

// ---------------------------------------------------------------------------
// parameters
// ---------------------------------------------------------------------------

/// One query parameter, converted to whatever type the server expects for its placeholder.
#[derive(Debug)]
pub(super) struct PgParam(pub(super) Value);

fn number_text(value: &Value) -> Result<String, BoxError> {
    match value {
        Value::Number(n) => Ok(n.to_string()),
        Value::String(s) => Ok(s.trim().to_string()),
        Value::Bool(b) => Ok(if *b { "1" } else { "0" }.to_string()),
        other => Err(format!("cannot use {other} as a number").into()),
    }
}

fn as_f64(value: &Value) -> Result<f64, BoxError> {
    match value {
        Value::Number(n) => n.as_f64().ok_or_else(|| "number out of range".into()),
        other => number_text(other)?.parse::<f64>().map_err(|e| format!("invalid number {other}: {e}").into()),
    }
}

fn as_i64(value: &Value) -> Result<i64, BoxError> {
    match value {
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_f64().filter(|f| f.fract() == 0.0 && f.abs() < 9.2e18).map(|f| f as i64))
            .ok_or_else(|| format!("{n} is not a whole number").into()),
        other => {
            let text = number_text(other)?;
            text.parse::<i64>()
                .or_else(|_| {
                    text.parse::<f64>()
                        .ok()
                        .filter(|f| f.fract() == 0.0 && f.abs() < 9.2e18)
                        .map(|f| f as i64)
                        .ok_or_else(|| format!("'{text}' is not a whole number"))
                })
                .map_err(Into::into)
        }
    }
}

fn as_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
        other => super::template::js_string(other),
    }
}

fn parse_timestamp(value: &Value) -> Result<DateTime<Utc>, BoxError> {
    match value {
        Value::Number(n) => {
            let ms = n.as_i64().ok_or("timestamp must be a whole number of milliseconds")?;
            DateTime::from_timestamp_millis(ms).ok_or_else(|| "timestamp out of range".into())
        }
        Value::String(s) => {
            let s = s.trim();
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Ok(dt.with_timezone(&Utc));
            }
            for format in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"] {
                if let Ok(naive) = NaiveDateTime::parse_from_str(s, format) {
                    return Ok(naive.and_utc());
                }
            }
            if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                return Ok(date.and_time(NaiveTime::MIN).and_utc());
            }
            Err(format!("cannot read '{s}' as a timestamp").into())
        }
        other => Err(format!("cannot use {other} as a timestamp").into()),
    }
}

impl ToSql for PgParam {
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, BoxError> {
        if self.0.is_null() {
            return Ok(IsNull::Yes);
        }
        let value = &self.0;
        match *ty {
            Type::BOOL => {
                let flag = match value {
                    Value::Bool(b) => *b,
                    Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
                    Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
                        "true" | "t" | "1" | "yes" | "on" => true,
                        "false" | "f" | "0" | "no" | "off" => false,
                        other => return Err(format!("cannot read '{other}' as a boolean").into()),
                    },
                    other => return Err(format!("cannot use {other} as a boolean").into()),
                };
                flag.to_sql(ty, out)
            }
            Type::INT2 => i16::try_from(as_i64(value)?).map_err(|_| "value out of range for smallint")?.to_sql(ty, out),
            Type::INT4 => i32::try_from(as_i64(value)?).map_err(|_| "value out of range for integer")?.to_sql(ty, out),
            Type::INT8 => as_i64(value)?.to_sql(ty, out),
            Type::OID => u32::try_from(as_i64(value)?).map_err(|_| "value out of range for oid")?.to_sql(ty, out),
            Type::FLOAT4 => (as_f64(value)? as f32).to_sql(ty, out),
            Type::FLOAT8 => as_f64(value)?.to_sql(ty, out),
            Type::NUMERIC => {
                encode_numeric(&number_text(value)?, out)?;
                Ok(IsNull::No)
            }
            Type::JSON | Type::JSONB => match value {
                // A string is already JSON text only if it parses; otherwise it is a JSON string value.
                Value::String(s) => serde_json::from_str::<Value>(s).unwrap_or_else(|_| value.clone()).to_sql(ty, out),
                other => other.to_sql(ty, out),
            },
            Type::TIMESTAMPTZ => parse_timestamp(value)?.to_sql(ty, out),
            Type::TIMESTAMP => parse_timestamp(value)?.naive_utc().to_sql(ty, out),
            Type::DATE => parse_timestamp(value)?.date_naive().to_sql(ty, out),
            Type::UUID => {
                let text = as_text(value);
                uuid::Uuid::parse_str(text.trim()).map_err(|e| format!("invalid uuid '{text}': {e}"))?.to_sql(ty, out)
            }
            Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME | Type::UNKNOWN => as_text(value).to_sql(ty, out),
            _ => Err(format!("unsupported parameter type '{ty}'").into()),
        }
    }

    fn accepts(_ty: &Type) -> bool {
        true
    }

    to_sql_checked!();
}

/// Turn `$name` placeholders into `$1`, `$2`, ... and collect the values from `named`,
/// like `msg.queryParameters` of `node-red-contrib-postgresql`. Placeholders are numbered in
/// alphabetical order of their names, and a placeholder without a value is an error.
pub(super) fn convert_named(sql: &str, named: &serde_json::Map<String, Value>) -> crate::Result<(String, Vec<Value>)> {
    let bytes = sql.as_bytes();
    let mut tokens: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && bytes.get(i + 1).is_some_and(|b| b.is_ascii_alphabetic()) {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            let token = &sql[start..end];
            if !tokens.contains(&token) {
                tokens.push(token);
            }
            i = end;
        } else {
            i += 1;
        }
    }

    let missing: Vec<&str> = tokens.iter().copied().filter(|t| !named.contains_key(*t)).collect();
    if !missing.is_empty() {
        return Err(anyhow::anyhow!("Missing Parameters: {}", missing.join(", ")));
    }
    tokens.sort_unstable();

    let mut text = String::with_capacity(sql.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && bytes.get(i + 1).is_some_and(|b| b.is_ascii_alphabetic()) {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            let position = tokens.iter().position(|t| *t == &sql[start..end]).unwrap();
            text.push_str(&format!("${}", position + 1));
            i = end;
        } else {
            let ch = sql[i..].chars().next().unwrap();
            text.push(ch);
            i += ch.len_utf8();
        }
    }
    let values = tokens.iter().map(|t| named[*t].clone()).collect();
    Ok((text, values))
}

// ---------------------------------------------------------------------------
// rows
// ---------------------------------------------------------------------------

fn float_value(f: f64) -> Variant {
    serde_json::Number::from_f64(f).map_or_else(|| Variant::String(f.to_string()), Variant::Number)
}

fn int8_value(v: i64) -> Variant {
    // Beyond 2^53 a JSON number is no longer exact, so hand such values over as text.
    if v.unsigned_abs() > (1u64 << 53) { Variant::String(v.to_string()) } else { Variant::from(v) }
}

fn array_of<'a, T: FromSql<'a>>(row: &'a tokio_postgres::Row, idx: usize, convert: impl Fn(T) -> Variant) -> Variant {
    match row.try_get::<_, Option<Vec<Option<T>>>>(idx) {
        Ok(Some(items)) => Variant::Array(items.into_iter().map(|i| i.map_or(Variant::Null, &convert)).collect()),
        _ => Variant::Null,
    }
}

fn timestamp_text(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn column_to_variant(row: &tokio_postgres::Row, idx: usize) -> Variant {
    let ty = row.columns()[idx].type_().clone();

    macro_rules! scalar {
        ($t:ty, $f:expr) => {
            match row.try_get::<_, Option<$t>>(idx) {
                Ok(Some(v)) => $f(v),
                Ok(None) => Variant::Null,
                Err(e) => {
                    log::warn!("Cannot read column '{}' ({ty}): {e}", row.columns()[idx].name());
                    Variant::Null
                }
            }
        };
    }

    match ty {
        Type::BOOL => scalar!(bool, Variant::Bool),
        Type::INT2 => scalar!(i16, |v| Variant::from(v as i64)),
        Type::INT4 => scalar!(i32, |v| Variant::from(v as i64)),
        Type::INT8 => scalar!(i64, int8_value),
        Type::OID => scalar!(u32, |v| Variant::from(v as i64)),
        Type::FLOAT4 => scalar!(f32, |v: f32| float_value(v.to_string().parse::<f64>().unwrap_or(v as f64))),
        Type::FLOAT8 => scalar!(f64, float_value),
        Type::NUMERIC => scalar!(PgNumeric, |v: PgNumeric| Variant::String(v.0)),
        Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME | Type::UNKNOWN => scalar!(String, Variant::String),
        Type::JSON | Type::JSONB => scalar!(Value, Variant::from),
        Type::UUID => scalar!(uuid::Uuid, |v: uuid::Uuid| Variant::String(v.to_string())),
        Type::TIMESTAMPTZ => scalar!(DateTime<Utc>, |v| Variant::String(timestamp_text(v))),
        Type::TIMESTAMP => scalar!(NaiveDateTime, |v: NaiveDateTime| Variant::String(timestamp_text(v.and_utc()))),
        Type::DATE => scalar!(NaiveDate, |v: NaiveDate| Variant::String(v.format("%Y-%m-%d").to_string())),
        Type::TIME => scalar!(NaiveTime, |v: NaiveTime| Variant::String(v.format("%H:%M:%S%.f").to_string())),
        Type::BYTEA => scalar!(Vec<u8>, Variant::Bytes),
        Type::BOOL_ARRAY => array_of::<bool>(row, idx, Variant::Bool),
        Type::INT2_ARRAY => array_of::<i16>(row, idx, |v| Variant::from(v as i64)),
        Type::INT4_ARRAY => array_of::<i32>(row, idx, |v| Variant::from(v as i64)),
        Type::INT8_ARRAY => array_of::<i64>(row, idx, int8_value),
        Type::FLOAT4_ARRAY => {
            array_of::<f32>(row, idx, |v| float_value(v.to_string().parse::<f64>().unwrap_or(v as f64)))
        }
        Type::FLOAT8_ARRAY => array_of::<f64>(row, idx, float_value),
        Type::TEXT_ARRAY | Type::VARCHAR_ARRAY => array_of::<String>(row, idx, Variant::String),
        _ => {
            log::warn!("Column '{}' has the unsupported type '{ty}', returning null", row.columns()[idx].name());
            Variant::Null
        }
    }
}

/// Decode result rows into an array of objects keyed by column name.
pub(super) fn rows_to_variant(rows: &[tokio_postgres::Row]) -> Variant {
    Variant::Array(
        rows.iter()
            .map(|row| {
                let mut map: BTreeMap<String, Variant> = BTreeMap::new();
                for (idx, column) in row.columns().iter().enumerate() {
                    map.insert(column.name().to_string(), column_to_variant(row, idx));
                }
                Variant::Object(map)
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn encoded(text: &str) -> Vec<u8> {
        let mut out = BytesMut::new();
        encode_numeric(text, &mut out).unwrap();
        out.to_vec()
    }

    #[test]
    fn numeric_round_trips() {
        for text in [
            "0",
            "1",
            "-1",
            "12.5",
            "-12.340",
            "100",
            "10000",
            "123456789.0001",
            "0.0001",
            "0.00001",
            "99999999.99",
            "1000000.5",
        ] {
            let decoded = decode_numeric(&encoded(text)).unwrap();
            assert_eq!(decoded, text, "round trip of {text}");
        }
    }

    #[test]
    fn numeric_matches_postgres_wire_format() {
        // 12.5 -> digits [12, 5000], weight 0, scale 1
        assert_eq!(encoded("12.5"), vec![0, 2, 0, 0, 0, 0, 0, 1, 0, 12, 0x13, 0x88]);
        // 0.0001 -> digits [1], weight -1, scale 4
        assert_eq!(encoded("0.0001"), vec![0, 1, 0xFF, 0xFF, 0, 0, 0, 4, 0, 1]);
        // zero has no digits
        assert_eq!(encoded("0"), vec![0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn numeric_exponents_and_errors() {
        assert_eq!(decode_numeric(&encoded("1e3")).unwrap(), "1000");
        assert_eq!(decode_numeric(&encoded("2.5e-2")).unwrap(), "0.025");
        assert_eq!(decode_numeric(&encoded("NaN")).unwrap(), "NaN");
        let mut out = BytesMut::new();
        assert!(encode_numeric("abc", &mut out).is_err());
        assert!(encode_numeric("", &mut out).is_err());
    }

    #[test]
    fn named_parameters_are_numbered_alphabetically() {
        let named = json!({"b": 2, "a": 1}).as_object().unwrap().clone();
        let (sql, values) = convert_named("SELECT $b, $a, $b FROM t WHERE x = $a", &named).unwrap();
        assert_eq!(sql, "SELECT $2, $1, $2 FROM t WHERE x = $1");
        assert_eq!(values, vec![json!(1), json!(2)]);
    }

    #[test]
    fn named_parameters_ignore_positional_and_report_missing() {
        let named = json!({"a": 1}).as_object().unwrap().clone();
        let (sql, _) = convert_named("SELECT $1, $a", &named).unwrap();
        assert_eq!(sql, "SELECT $1, $1");
        let err = convert_named("SELECT $a, $zzz", &named).unwrap_err().to_string();
        assert!(err.contains("Missing Parameters: zzz"), "{err}");
    }

    #[test]
    fn timestamps_accept_common_forms() {
        assert!(parse_timestamp(&json!("2024-05-01T10:00:00Z")).is_ok());
        assert!(parse_timestamp(&json!("2024-05-01 10:00:00.250")).is_ok());
        assert_eq!(parse_timestamp(&json!(1714557600000_i64)).unwrap().timestamp(), 1714557600);
        assert!(parse_timestamp(&json!("yesterday")).is_err());
    }
}
