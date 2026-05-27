//! Helper for building Node-RED flow JSON programmatically.
//!
//! Instead of writing raw JSON arrays, use `FlowBuilder` to construct
//! test flows with a fluent API.

use serde_json::{Value, json};

/// Builder for constructing Node-RED flow JSON arrays.
pub struct FlowBuilder {
    nodes: Vec<Value>,
}

#[allow(dead_code)]
impl FlowBuilder {
    /// Create a new flow builder with a default tab node.
    pub fn new() -> Self {
        Self { nodes: vec![json!({"id": "100", "type": "tab", "label": "Test Flow"})] }
    }

    /// Create a new flow builder with a custom tab ID.
    #[allow(dead_code)]
    pub fn with_tab_id(tab_id: &str) -> Self {
        Self { nodes: vec![json!({"id": tab_id, "type": "tab", "label": "Test Flow"})] }
    }

    /// Add an inject node that fires once on deploy.
    ///
    /// Sets `once: true`, `onceDelay: 0`.
    /// `wires` is a JSON value like `json!([["99"]])` or `json!([["a"], ["b"]])`.
    pub fn inject_once(mut self, id: &str, payload: &str, payload_type: &str, wires: Value) -> Self {
        self.nodes.push(json!({
            "id": id,
            "type": "inject",
            "z": "100",
            "name": "",
            "props": [
                {"p": "payload", "v": payload, "vt": payload_type},
                {"p": "topic", "vt": "str"}
            ],
            "repeat": "",
            "crontab": "",
            "once": true,
            "onceDelay": 0,
            "topic": "",
            "payload": payload,
            "payloadType": payload_type,
            "wires": wires,
        }));
        self
    }

    /// Add an inject node that fires once with multiple properties.
    pub fn inject_once_with_props(mut self, id: &str, props: Vec<(&str, &str, &str)>, wires: Value) -> Self {
        let props_json: Vec<Value> = props
            .iter()
            .map(|(p, v, vt)| {
                if *vt == "str" || *vt == "num" || *vt == "bool" || *vt == "json" {
                    json!({"p": p, "v": v, "vt": vt})
                } else {
                    json!({"p": p, "v": "", "vt": vt})
                }
            })
            .collect();

        self.nodes.push(json!({
            "id": id,
            "type": "inject",
            "z": "100",
            "name": "",
            "props": props_json,
            "repeat": "",
            "crontab": "",
            "once": true,
            "onceDelay": 0,
            "topic": "",
            "wires": wires,
        }));
        self
    }

    /// Add a function node with JavaScript code.
    pub fn function(mut self, id: &str, code: &str, outputs: usize, wires: Value) -> Self {
        self.nodes.push(json!({
            "id": id,
            "type": "function",
            "z": "100",
            "name": "",
            "func": code,
            "outputs": outputs,
            "timeout": 0,
            "noerr": 0,
            "initialize": "",
            "finalize": "",
            "libs": [],
            "wires": wires,
        }));
        self
    }

    /// Add a switch node.
    pub fn switch(
        mut self,
        id: &str,
        property: &str,
        rules: Vec<Value>,
        check_all: bool,
        outputs: usize,
        wires: Value,
    ) -> Self {
        self.nodes.push(json!({
            "id": id,
            "type": "switch",
            "z": "100",
            "name": "",
            "property": property,
            "propertyType": "msg",
            "rules": rules,
            "checkall": check_all,
            "repair": false,
            "outputs": outputs,
            "wires": wires,
        }));
        self
    }

    /// Add a change node.
    pub fn change(mut self, id: &str, rules: Vec<Value>, wires: Value) -> Self {
        self.nodes.push(json!({
            "id": id,
            "type": "change",
            "z": "100",
            "name": "",
            "rules": rules,
            "wires": wires,
        }));
        self
    }

    /// Add a template node.
    pub fn template(mut self, id: &str, template: &str, field: &str, output: &str, wires: Value) -> Self {
        self.nodes.push(json!({
            "id": id,
            "type": "template",
            "z": "100",
            "name": "",
            "template": template,
            "field": field,
            "fieldType": "msg",
            "output": output,
            "syntax": "mustache",
            "wires": wires,
        }));
        self
    }

    /// Add a delay node.
    pub fn delay(mut self, id: &str, delay: &str, delay_units: &str, wires: Value) -> Self {
        self.nodes.push(json!({
            "id": id,
            "type": "delay",
            "z": "100",
            "name": "",
            "pauseType": "delay",
            "timeout": delay,
            "timeoutUnits": delay_units,
            "rate": "1",
            "nbRateUnits": "1",
            "rateUnits": "second",
            "randomFirst": "1",
            "randomLast": "5",
            "randomUnits": "seconds",
            "drop": false,
            "allowrate": false,
            "outputs": 1,
            "wires": wires,
        }));
        self
    }

    /// Add a JSON node.
    pub fn json_node(mut self, id: &str, action: &str, wires: Value) -> Self {
        self.nodes.push(json!({
            "id": id,
            "type": "json",
            "z": "100",
            "name": "",
            "property": "payload",
            "action": action,
            "pretty": false,
            "wires": wires,
        }));
        self
    }

    /// Add a split node.
    pub fn split(mut self, id: &str, wires: Value) -> Self {
        self.nodes.push(json!({
            "id": id,
            "type": "split",
            "z": "100",
            "name": "",
            "splt": "\\n",
            "spltType": "str",
            "arraySplt": 1,
            "arraySpltType": "len",
            "stream": false,
            "addname": "",
            "property": "payload",
            "wires": wires,
        }));
        self
    }

    /// Add a join node.
    pub fn join(mut self, id: &str, mode: &str, build: &str, count: Option<usize>, wires: Value) -> Self {
        let mut node = json!({
            "id": id,
            "type": "join",
            "z": "100",
            "name": "",
            "mode": mode,
            "build": build,
            "property": "payload",
            "propertyType": "msg",
            "joinChar": "\\n",
            "accumulate": false,
            "timeout": "",
            "reduce": false,
            "reduceExp": "",
            "reduceInit": "",
            "reduceFixup": "",
            "reduceInitType": "",
            "reduceExpType": "",
            "wires": wires,
        });
        if let Some(c) = count {
            node["count"] = json!(c);
        }
        self.nodes.push(node);
        self
    }

    /// Add a catch node.
    pub fn catch(mut self, id: &str, scope: Option<Vec<&str>>, wires: Value) -> Self {
        let scope_json = match scope {
            Some(s) => json!(s),
            None => json!(null),
        };
        self.nodes.push(json!({
            "id": id,
            "type": "catch",
            "z": "100",
            "name": "",
            "scope": scope_json,
            "uncaught": false,
            "wires": wires,
        }));
        self
    }

    /// Add a status node.
    pub fn status(mut self, id: &str, scope: Option<Vec<&str>>, wires: Value) -> Self {
        let scope_json = match scope {
            Some(s) => json!(s),
            None => json!(null),
        };
        self.nodes.push(json!({
            "id": id,
            "type": "status",
            "z": "100",
            "name": "",
            "scope": scope_json,
            "wires": wires,
        }));
        self
    }

    /// Add a complete node.
    pub fn complete(mut self, id: &str, scope: Vec<&str>, wires: Value) -> Self {
        self.nodes.push(json!({
            "id": id,
            "type": "complete",
            "z": "100",
            "name": "",
            "scope": scope,
            "uncaught": false,
            "wires": wires,
        }));
        self
    }

    /// Add a debug node.
    pub fn debug(mut self, id: &str) -> Self {
        self.nodes.push(json!({
            "id": id,
            "type": "debug",
            "z": "100",
            "name": "",
            "active": true,
            "tosidebar": true,
            "console": false,
            "tostatus": false,
            "complete": "payload",
            "targetType": "msg",
            "wires": [],
        }));
        self
    }

    /// Add the standard test-once sink node.
    pub fn test_sink(mut self, id: &str) -> Self {
        self.nodes.push(json!({
            "id": id,
            "type": "test-once",
            "z": "100",
        }));
        self
    }

    /// Add a raw node from a JSON value.
    pub fn raw_node(mut self, node: Value) -> Self {
        self.nodes.push(node);
        self
    }

    /// Convert the builder into a JSON array value suitable for `Engine::with_json`, consuming it.
    pub fn into_json(self) -> Value {
        Value::Array(self.nodes)
    }

    /// Convert the builder into a JSON string, consuming it.
    #[allow(dead_code)]
    pub fn into_json_string(self) -> String {
        serde_json::to_string(&self.into_json()).unwrap()
    }
}

impl Default for FlowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a switch rule JSON value for common operations.
pub mod switch_rule {
    use serde_json::{Value, json};

    pub fn eq(value: &str, value_type: &str) -> Value {
        json!({"t": "eq", "v": value, "vt": value_type})
    }

    pub fn neq(value: &str, value_type: &str) -> Value {
        json!({"t": "neq", "v": value, "vt": value_type})
    }

    pub fn lt(value: &str, value_type: &str) -> Value {
        json!({"t": "lt", "v": value, "vt": value_type})
    }

    pub fn lte(value: &str, value_type: &str) -> Value {
        json!({"t": "lte", "v": value, "vt": value_type})
    }

    pub fn gt(value: &str, value_type: &str) -> Value {
        json!({"t": "gt", "v": value, "vt": value_type})
    }

    pub fn gte(value: &str, value_type: &str) -> Value {
        json!({"t": "gte", "v": value, "vt": value_type})
    }

    pub fn btwn(v1: &str, v1t: &str, v2: &str, v2t: &str) -> Value {
        json!({"t": "btwn", "v": v1, "vt": v1t, "v2": v2, "v2t": v2t})
    }

    pub fn cont(value: &str) -> Value {
        json!({"t": "cont", "v": value, "vt": "str"})
    }

    pub fn regex(pattern: &str) -> Value {
        json!({"t": "regex", "v": pattern, "vt": "str"})
    }

    pub fn is_null() -> Value {
        json!({"t": "null"})
    }

    pub fn is_not_null() -> Value {
        json!({"t": "nnull"})
    }

    pub fn is_true() -> Value {
        json!({"t": "true"})
    }

    pub fn is_false() -> Value {
        json!({"t": "false"})
    }

    pub fn is_empty() -> Value {
        json!({"t": "empty"})
    }

    pub fn is_not_empty() -> Value {
        json!({"t": "nempty"})
    }

    pub fn istype(type_name: &str) -> Value {
        json!({"t": "istype", "v": type_name, "vt": "str"})
    }

    pub fn else_rule() -> Value {
        json!({"t": "else"})
    }

    #[allow(dead_code)]
    pub fn jsonata(expression: &str) -> Value {
        json!({"t": "jsonata_exp", "v": expression, "vt": "jsonata"})
    }
}

/// Create a change rule JSON value for common operations.
pub mod change_rule {
    use serde_json::{Value, json};

    pub fn set(prop: &str, prop_type: &str, to: &str, to_type: &str) -> Value {
        json!({"t": "set", "p": prop, "pt": prop_type, "to": to, "tot": to_type})
    }

    pub fn delete(prop: &str, prop_type: &str) -> Value {
        json!({"t": "delete", "p": prop, "pt": prop_type})
    }

    pub fn change(prop: &str, prop_type: &str, from: &str, from_type: &str, to: &str, to_type: &str) -> Value {
        json!({
            "t": "change",
            "p": prop,
            "pt": prop_type,
            "from": from,
            "fromt": from_type,
            "to": to,
            "tot": to_type,
        })
    }

    pub fn move_rule(prop: &str, prop_type: &str, to: &str, to_type: &str) -> Value {
        json!({"t": "move", "p": prop, "pt": prop_type, "to": to, "tot": to_type})
    }
}
