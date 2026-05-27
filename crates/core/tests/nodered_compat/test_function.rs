//! Function node compatibility tests.
//!
//! Verifies that the function node executes JavaScript code via rquickjs
//! and produces correct message outputs, matching Node-RED behavior.

use std::time::Duration;

use serde_json::json;

use super::harness::{TestHarness, assert_msg_has, assert_msg_str};

/// Helper to build a minimal function node flow.
fn function_flow(func: &str) -> serde_json::Value {
    json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "function", "name": "",
         "func": func,
         "outputs": 1, "timeout": 5, "noerr": 0,
         "initialize": "", "finalize": "", "libs": [],
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

/// Helper to build a multi-output function node flow.
fn function_flow_multi_output(func: &str, outputs: usize) -> serde_json::Value {
    let mut wires = Vec::with_capacity(outputs);
    for _ in 0..outputs {
        wires.push(json!(["99"]));
    }
    json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "function", "name": "",
         "func": func,
         "outputs": outputs, "timeout": 5, "noerr": 0,
         "initialize": "", "finalize": "", "libs": [],
         "wires": wires},
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

/// Function: basic modify payload.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_basic_modify_payload() {
    let flow = function_flow("msg.payload = 'modified'; return msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "modified");
}

/// Function: return null should produce no output.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_return_null() {
    let flow = function_flow("return null;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_millis(300)).await;
    assert_eq!(msgs.len(), 0, "Expected no output when function returns null");
}

/// Function: access msg.topic and forward as payload.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_access_topic() {
    let flow = function_flow("msg.payload = msg.topic; return msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .inject_and_collect_timeout("1", json!({"payload": "x", "topic": "hello"}), 1, Duration::from_secs(2))
        .await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "hello");
}

/// Function: set a new property on msg.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_set_new_property() {
    let flow = function_flow("msg.newprop = 'added'; return msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_has(&msgs[0], "newprop");
    assert_msg_str(&msgs[0], "newprop", "added");
}

/// Function: throw error should produce no output at sink.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_throw_error() {
    let flow = function_flow("throw new Error('test error');");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 0, "Expected no output when function throws");
}

/// Function: numeric operations on payload.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_numeric_operations() {
    let flow = function_flow("msg.payload = msg.payload * 2 + 1; return msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": 5}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let num = payload.as_f64().unwrap_or_else(|| panic!("Payload is not a number: {:?}", payload));
    assert_eq!(num, 11.0, "Expected 5 * 2 + 1 = 11");
}

/// Function: conditional logic — payload > 10 passes, else filtered.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_conditional_logic_pass() {
    let flow = function_flow("if (msg.payload > 10) { return msg; } return null;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": 15}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_conditional_logic_filtered() {
    let flow = function_flow("if (msg.payload > 10) { return msg; } return null;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": 5}), 1, Duration::from_millis(300)).await;
    assert_eq!(msgs.len(), 0, "Expected no output when payload <= 10");
}

/// Function: array manipulation with map.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_array_manipulation() {
    let flow = function_flow("msg.payload = msg.payload.map(x => x * 2); return msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": [1, 2, 3]}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    // Verify the payload is an array [2, 4, 6]
    let payload = msgs[0].get("payload").expect("Missing payload");
    match payload {
        rust_red_core::runtime::model::Variant::Array(arr) => {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0].as_f64().unwrap(), 2.0);
            assert_eq!(arr[1].as_f64().unwrap(), 4.0);
            assert_eq!(arr[2].as_f64().unwrap(), 6.0);
        }
        other => panic!("Expected array payload, got {:?}", other),
    }
}

/// Function: JSON.parse on a string payload.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_json_parse() {
    let flow = function_flow("msg.payload = JSON.parse(msg.payload); return msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .inject_and_collect_timeout("1", json!({"payload": "{\"key\":\"val\"}"}), 1, Duration::from_secs(2))
        .await;
    assert_eq!(msgs.len(), 1);
    // payload should be an object with key="val"
    let payload = msgs[0].get("payload").expect("Missing payload");
    let obj = payload.as_object().unwrap_or_else(|| panic!("Expected object payload, got {:?}", payload));
    let val = obj.get("key").expect("Missing 'key' in parsed object");
    assert_eq!(val.as_str().unwrap(), "val");
}

/// Function: string toUpperCase.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_string_methods() {
    let flow = function_flow("msg.payload = msg.payload.toUpperCase(); return msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "hello"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "HELLO");
}

// ---------------------------------------------------------------------------
// AC1: Date.now() usage in function nodes
// ---------------------------------------------------------------------------

/// Function: msg.payload = Date.now() should produce a numeric timestamp.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_date_now_timestamp() {
    let flow = function_flow("msg.payload = Date.now(); return msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let ts = payload.as_f64().unwrap_or_else(|| panic!("Payload should be a number, got {:?}", payload));
    // Timestamp should be a reasonable value (after year 2020 in milliseconds)
    assert!(ts > 1577836800000.0, "Date.now() should return a millisecond timestamp, got {ts}");
}

/// Function: new Date() construction and method calls work.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_date_object_creation() {
    let flow = function_flow("msg.payload = new Date().getFullYear(); return msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let year = payload.as_f64().unwrap_or_else(|| panic!("Payload should be a number, got {:?}", payload));
    // Year should be current year (2020-2030 range for a safe test)
    assert!(year >= 2020.0 && year <= 2035.0, "getFullYear() should return current year, got {year}");
}

// ---------------------------------------------------------------------------
// AC2: context.get/set, flow.get/set, global.get/set
// ---------------------------------------------------------------------------

/// Function: context.get/set at node scope.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_context_get_set() {
    let flow = function_flow("context.set('mykey', 'myvalue');\nmsg.payload = context.get('mykey');\nreturn msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "myvalue");
}

/// Function: flow.get/set at flow scope.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_flow_get_set() {
    let flow = function_flow("flow.set('flowkey', 'flowvalue');\nmsg.payload = flow.get('flowkey');\nreturn msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "flowvalue");
}

/// Function: global.get/set at global scope.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_global_get_set() {
    let flow = function_flow("global.set('gkey', 'gvalue');\nmsg.payload = global.get('gkey');\nreturn msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "gvalue");
}

/// Function: context.get returns undefined for non-existent keys.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_context_get_undefined() {
    let flow = function_flow(
        "var val = context.get('nonexistent');\nmsg.payload = (typeof val === 'undefined') ? 'yes' : 'no';\nreturn msg;",
    );
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "yes");
}

/// Function: context.set with numeric values.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_context_set_numeric() {
    let flow = function_flow("context.set('counter', 42);\nmsg.payload = context.get('counter');\nreturn msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    assert_eq!(payload.as_f64().unwrap(), 42.0);
}

/// Function: context.set with object values.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_context_set_object() {
    let flow = function_flow(
        "context.set('data', {name: 'test', count: 5});\nvar d = context.get('data');\nmsg.payload = d.name + ':' + d.count;\nreturn msg;",
    );
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "test:5");
}

// ---------------------------------------------------------------------------
// AC3: node.send() for single and multi-output
// ---------------------------------------------------------------------------

/// Function: node.send() with a single message on port 0.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_node_send_single() {
    let flow = function_flow("msg.payload = 'sent'; node.send(msg);");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "sent");
}

/// Function: node.send() with array for multi-output.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_node_send_multi_output() {
    let flow = function_flow_multi_output(
        "var msg1 = {payload: 'out1'};\nvar msg2 = {payload: 'out2'};\nnode.send([msg1, msg2]);",
        2,
    );
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 2, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 2);
}

/// Function: node.send() with null on one output port (port 0 null, port 1 has msg).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_node_send_multi_output_with_null() {
    let flow = function_flow_multi_output("node.send([null, {payload: 'only-port1'}]);", 2);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "only-port1");
}

// ---------------------------------------------------------------------------
// AC6: No segfaults or panics on malformed JS (graceful error reporting)
// ---------------------------------------------------------------------------

/// Function: syntax error in JS should not crash, just produce no output.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_malformed_syntax_error() {
    let flow = function_flow("function {;;; invalid syntax {{{");
    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_millis(500)).await;
    // The function node should fail gracefully -- no output
    assert_eq!(msgs.len(), 0, "Expected no output for malformed JS syntax");
}

/// Function: reference to undefined variable should not crash.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_undefined_reference() {
    let flow = function_flow("msg.payload = nonexistentVariable;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_millis(500)).await;
    assert_eq!(msgs.len(), 0, "Expected no output for undefined reference error");
}

/// Function: TypeError (calling non-function) should not crash.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_type_error() {
    let flow = function_flow("var x = 42; x();");
    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_millis(500)).await;
    assert_eq!(msgs.len(), 0, "Expected no output for TypeError");
}

/// Function: empty function body should not crash.
/// When func is empty string, the user script is effectively empty inside the async wrapper,
/// so no explicit return happens and no output is produced.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_empty_body() {
    let flow = function_flow("");
    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_millis(300)).await;
    // Empty body means no explicit return -> undefined -> no output
    assert_eq!(msgs.len(), 0, "Empty function body should produce no output");
}

/// Function: accessing properties of null should not crash.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_null_property_access() {
    let flow = function_flow("var x = null; msg.payload = x.foo;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_millis(500)).await;
    assert_eq!(msgs.len(), 0, "Expected no output for null property access TypeError");
}

// ---------------------------------------------------------------------------
// node.id and node.name access
// ---------------------------------------------------------------------------

/// Function: node.id and node.name should be accessible.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_node_properties() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "function", "name": "myFunc",
         "func": "msg.payload = node.id + ':' + node.name; return msg;",
         "outputs": 1, "timeout": 5, "noerr": 0,
         "initialize": "", "finalize": "", "libs": [],
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let s = payload.as_str().unwrap();
    assert!(s.contains("0000000000000001"), "node.id should contain '0000000000000001', got: {s}");
    assert!(s.contains("myFunc"), "node.name should be 'myFunc', got: {s}");
}

// ---------------------------------------------------------------------------
// node.log / node.warn / node.error / node.debug
// ---------------------------------------------------------------------------

/// Function: node.log should not crash.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_node_log() {
    let flow = function_flow("node.log('hello from log'); return msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
}

/// Function: node.warn should not crash.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_node_warn() {
    let flow = function_flow("node.warn('warning message'); return msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
}

/// Function: node.error should not crash.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_node_error() {
    let flow = function_flow("node.error('error message'); return msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
}

// ---------------------------------------------------------------------------
// env.get() access
// ---------------------------------------------------------------------------

/// Function: env.get() for environment variable access.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn function_env_get() {
    let flow = function_flow("msg.payload = env.get('TEST_ENV_VAR') || 'not-set'; return msg;");
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "input"}), 1, Duration::from_secs(2)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "not-set");
}
