//! YAML node compatibility tests.
//!
//! Verifies that the YAML node parses YAML strings to objects and
/// converts objects back to YAML strings, matching Node-RED behavior.
use serde_json::json;

use super::harness::TestHarness;

/// YAML: parse a simple YAML string to an object (auto mode).
///
/// Input: `"key: value\nnum: 42"`
/// Output: object `{key: "value", num: 42}`
#[cfg(feature = "nodes_yaml")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn yaml_parse_to_object() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "yaml", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "key: value\nnum: 42"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let obj = payload.as_object().expect("Payload should be an object");
    assert_eq!(obj.get("key").unwrap().as_str().unwrap(), "value");
    assert_eq!(obj.get("num").unwrap().as_number().unwrap().as_i64().unwrap(), 42);
}

/// YAML: convert an object to YAML string (auto mode).
///
/// Input: object `{name: "test", count: 5}`
/// Output: YAML string containing "name: test" and "count: 5".
#[cfg(feature = "nodes_yaml")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn yaml_stringify() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "yaml", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": {"name": "test", "count": 5}}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let yaml_str = payload.as_str().expect("Payload should be a string");
    assert!(yaml_str.contains("name: test"), "YAML output should contain 'name: test', got: {yaml_str}");
    assert!(yaml_str.contains("count: 5"), "YAML output should contain 'count: 5', got: {yaml_str}");
}

/// YAML: parse a YAML array.
///
/// Input: `"- one\n- two\n- three"`
/// Output: array `["one", "two", "three"]`
#[cfg(feature = "nodes_yaml")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn yaml_parse_array() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "yaml", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "- one\n- two\n- three"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let arr = payload.as_array().expect("Payload should be an array");
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].as_str().unwrap(), "one");
    assert_eq!(arr[1].as_str().unwrap(), "two");
    assert_eq!(arr[2].as_str().unwrap(), "three");
}
