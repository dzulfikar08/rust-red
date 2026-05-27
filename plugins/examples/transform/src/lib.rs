//! Transform plugin for Rust-Red.
//!
//! Applies configurable transformations to messages:
//! - template: replace payload with a template string ({{payload}}, {{topic}}, etc.)
//! - rename: rename a message property
//! - add: add a new property to the message
//! - filter: only pass through if payload matches a condition
//!
//! Configuration is passed via the `extra` field of the on_start config message.
//! The `rules` key should contain an array of rule objects, each with an "action" field.

#![no_std]

extern crate alloc;

use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use rust_red_wasm_sdk::*;

struct TransformNode;

impl Default for TransformNode {
    fn default() -> Self {
        TransformNode
    }
}

/// Resolve a template placeholder like {{payload}}, {{topic}}, {{extra.key}}.
fn resolve_placeholder(name: &str, msg: &WasmMessage) -> String {
    match name {
        "payload" => wasm_value_to_string(&msg.payload),
        "topic" => msg.topic.as_deref().unwrap_or("").to_owned(),
        "msgId" | "msg_id" => msg.msg_id.clone(),
        _ => {
            // Try "extra." prefix for extra fields
            if let Some(key) = name.strip_prefix("extra.") {
                msg.extra.get(key).map(|v| wasm_value_to_string(v)).unwrap_or_else(|| String::new())
            } else {
                msg.extra.get(name).map(|v| wasm_value_to_string(v)).unwrap_or_else(|| String::new())
            }
        }
    }
}

/// Render a template string by replacing {{key}} placeholders.
fn render_template(template: &str, msg: &WasmMessage) -> String {
    let mut result = String::new();
    let chars: Vec<char> = template.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '{' && i + 1 < len && chars[i + 1] == '{' {
            i += 2; // skip opening {{
            let mut key = String::new();
            let mut closed = false;
            while i < len {
                if chars[i] == '}' && i + 1 < len && chars[i + 1] == '}' {
                    i += 2; // skip closing }}
                    closed = true;
                    break;
                }
                key.push(chars[i]);
                i += 1;
            }
            if closed {
                result.push_str(&resolve_placeholder(key.trim(), msg));
            } else {
                result.push_str("{{");
                result.push_str(&key);
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Convert a WasmValue to a display string.
fn wasm_value_to_string(v: &WasmValue) -> String {
    match v {
        WasmValue::Null => String::from("null"),
        WasmValue::Bool(b) => {
            if *b {
                String::from("true")
            } else {
                String::from("false")
            }
        }
        WasmValue::I64(n) => format!("{}", n),
        WasmValue::U64(n) => format!("{}", n),
        WasmValue::F64(n) => format!("{}", n),
        WasmValue::String(s) => s.clone(),
        WasmValue::Bytes(b) => format!("[bytes:{}]", b.len()),
        WasmValue::Array(arr) => {
            let items: Vec<String> = arr.iter().map(wasm_value_to_string).collect();
            format!("[{}]", items.join(", "))
        }
        WasmValue::Object(obj) => {
            let items: Vec<String> = obj.iter().map(|(k, v)| format!("{}:{}", k, wasm_value_to_string(v))).collect();
            format!("{{{}}}", items.join(", "))
        }
    }
}

/// Extract a rule configuration value as a string from extra fields.
/// Rules are stored as an array of objects: extra["rules"] = Array([Object({...})])
fn get_rule_str(rule: &BTreeMap<String, WasmValue>, key: &str) -> Option<String> {
    rule.get(key).and_then(|v| v.as_str().map(|s| s.to_owned()))
}

/// Apply template transformation: replace payload with rendered template string.
fn apply_template(msg: &mut WasmMessage, template: &str) {
    let rendered = render_template(template, msg);
    msg.payload = WasmValue::String(rendered);
}

/// Apply rename transformation: move value from one property to another.
fn apply_rename(msg: &mut WasmMessage, from: &str, to: &str) {
    // Handle special fields: payload, topic
    match from {
        "payload" => {
            let val = core::mem::replace(&mut msg.payload, WasmValue::Null);
            set_property(msg, to, val);
        }
        "topic" => {
            let val = msg.topic.take();
            if let Some(s) = val {
                set_property(msg, to, WasmValue::String(s));
            }
        }
        _ => {
            if let Some(val) = msg.extra.remove(from) {
                set_property(msg, to, val);
            }
        }
    }
}

/// Set a property on the message (payload, topic, or extra).
fn set_property(msg: &mut WasmMessage, key: &str, val: WasmValue) {
    match key {
        "payload" => {
            msg.payload = val;
        }
        "topic" => {
            msg.topic = Some(val.as_str().unwrap_or("").to_owned());
        }
        _ => {
            msg.extra.insert(key.to_owned(), val);
        }
    }
}

/// Apply add transformation: add a new property with a literal or computed value.
fn apply_add(msg: &mut WasmMessage, key: &str, value_expr: &str) {
    let val = match value_expr {
        "timestamp" | "$timestamp" => {
            // WASM has no clock access; use a counter-like value from msg_id as placeholder
            WasmValue::String(format!("ts-{}", msg.msg_id))
        }
        "null" => WasmValue::Null,
        "true" => WasmValue::Bool(true),
        "false" => WasmValue::Bool(false),
        s => {
            // Try parsing as integer, else use as literal string
            if let Ok(n) = s.parse::<i64>() {
                WasmValue::I64(n)
            } else if let Ok(n) = s.parse::<u64>() {
                WasmValue::U64(n)
            } else if let Ok(f) = s.parse::<f64>() {
                WasmValue::F64(f)
            } else {
                // Treat as template for variable substitution
                let rendered = render_template(s, msg);
                WasmValue::String(rendered)
            }
        }
    };
    set_property(msg, key, val);
}

/// Evaluate a filter condition against a message payload.
/// Returns true if the message should pass through.
fn eval_filter(msg: &WasmMessage, condition: &str) -> bool {
    match condition {
        "nonnull" | "notnull" | "non-null" => !matches!(msg.payload, WasmValue::Null),
        "truthy" => match &msg.payload {
            WasmValue::Null => false,
            WasmValue::Bool(b) => *b,
            WasmValue::I64(n) => *n != 0,
            WasmValue::U64(n) => *n != 0,
            WasmValue::F64(n) => *n != 0.0,
            WasmValue::String(s) => !s.is_empty(),
            _ => true,
        },
        "falsy" => !eval_filter(msg, "truthy"),
        "isstring" | "is_string" => matches!(msg.payload, WasmValue::String(_)),
        "isnumber" | "is_number" => {
            matches!(msg.payload, WasmValue::I64(_) | WasmValue::U64(_) | WasmValue::F64(_))
        }
        "isempty" | "is_empty" => match &msg.payload {
            WasmValue::Null => true,
            WasmValue::String(s) => s.is_empty(),
            WasmValue::Array(a) => a.is_empty(),
            WasmValue::Object(o) => o.is_empty(),
            _ => false,
        },
        // Exact string match: "equals:somevalue"
        s if s.starts_with("equals:") => {
            let expected = &s[7..];
            msg.payload.as_str().map(|p| p == expected).unwrap_or(false)
        }
        // Contains: "contains:substr"
        s if s.starts_with("contains:") => {
            let needle = &s[9..];
            msg.payload.as_str().map(|p| p.contains(needle)).unwrap_or(false)
        }
        _ => true, // Unknown condition passes through
    }
}

/// Extract rules from the config message's extra["rules"] field.
/// Returns a Vec of rule BTreeMaps.
fn extract_rules(config: &WasmMessage) -> Vec<BTreeMap<String, WasmValue>> {
    let mut rules = vec![];
    if let Some(WasmValue::Array(arr)) = config.extra.get("rules") {
        for item in arr {
            if let WasmValue::Object(obj) = item {
                rules.push(obj.clone());
            }
        }
    }
    rules
}

/// Apply a single rule to a message. Returns true if the message should be
/// swallowed (filter reject).
fn apply_rule(msg: &mut WasmMessage, rule: &BTreeMap<String, WasmValue>) -> RuleResult {
    let action = match get_rule_str(rule, "action") {
        Some(a) => a,
        None => return RuleResult::Pass,
    };

    match action.as_str() {
        "template" => {
            if let Some(tmpl) = get_rule_str(rule, "template") {
                apply_template(msg, &tmpl);
            }
            RuleResult::Pass
        }
        "rename" => {
            let from = get_rule_str(rule, "from").unwrap_or_default();
            let to = get_rule_str(rule, "to").unwrap_or_default();
            if !from.is_empty() && !to.is_empty() {
                apply_rename(msg, &from, &to);
            }
            RuleResult::Pass
        }
        "add" => {
            let key = get_rule_str(rule, "key").unwrap_or_default();
            let value = get_rule_str(rule, "value").unwrap_or_default();
            if !key.is_empty() {
                apply_add(msg, &key, &value);
            }
            RuleResult::Pass
        }
        "filter" => {
            let condition = get_rule_str(rule, "condition").unwrap_or_else(|| "nonnull".to_owned());
            if eval_filter(msg, &condition) {
                RuleResult::Matched
            } else {
                RuleResult::Unmatched
            }
        }
        _ => RuleResult::Pass,
    }
}

enum RuleResult {
    /// Continue processing (no filter involved).
    Pass,
    /// Filter matched — message should go to port 0.
    Matched,
    /// Filter unmatched — message should go to port 1.
    Unmatched,
}

impl WasmNodeHandler for TransformNode {
    fn info() -> WasmNodeInfo {
        WasmNodeInfo {
            node_type: String::from("example/transform"),
            red_name: String::from("transform"),
            module: String::from("example"),
            version: String::from("1.0.0"),
            inputs: 1,
            outputs: 2, // port 0 = matched/transformed, port 1 = unmatched (filter miss)
            color: Some(String::from("#E2D96E")),
            icon: Some(String::from("arrow-right.svg")),
            label: Some(String::from("transform")),
            label_style: None,
            palette_label: None,
            align: None,
            editor_template: None,
            capabilities: vec![],
        }
    }

    fn on_start(_config: WasmMessage) {
        log("info", "TransformNode started");
    }

    fn process(mut msg: WasmMessage) -> ProcessResult {
        log("debug", "TransformNode: processing message");

        // For demonstrating rules, we use built-in defaults when no config is available.
        // In production, rules would come from on_start config. Since the WASM process()
        // is stateless (static handler), we apply rules encoded in msg.extra["_rules"].
        //
        // If no rules are specified, we do a default transform:
        // add a "processed" flag and pass through.

        let has_rules = msg.extra.contains_key("_rules");

        if has_rules {
            // Rules passed inline in the message for per-message control
            let rules = extract_rules(&msg);
            // Remove the meta-field so it doesn't leak into output
            msg.extra.remove("_rules");

            let mut filter_result: Option<bool> = None;

            for rule in &rules {
                match apply_rule(&mut msg, rule) {
                    RuleResult::Pass => {}
                    RuleResult::Matched => {
                        filter_result = Some(true);
                    }
                    RuleResult::Unmatched => {
                        filter_result = Some(false);
                    }
                }
            }

            match filter_result {
                Some(true) => ProcessResult::to_port(0, msg, 2),
                Some(false) => ProcessResult::to_port(1, msg, 2),
                None => ProcessResult::to_port(0, msg, 2),
            }
        } else {
            // Default behavior: pass through with a "processed" stamp
            msg.extra.insert(String::from("processed"), WasmValue::Bool(true));
            ProcessResult::to_port(0, msg, 2)
        }
    }

    fn on_stop() {
        log("info", "TransformNode stopped");
    }
}

export_node!(TransformNode);
