//! JSON node compatibility tests.
//!
//! Verifies that the JSON node parses and stringifies JSON correctly,
//! matching Node-RED behavior.

use serde_json::json;

use super::flow_builder::FlowBuilder;
use super::harness::TestHarness;
use rust_red_core::runtime::model::Variant;

/// JSON: parse a JSON string into an object (auto mode).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn json_parse_string_to_object() {
    let flow = FlowBuilder::new().json_node("1", "", json!([["99"]])).test_sink("99").to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "{\"name\":\"test\",\"value\":42}"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let obj = payload.as_object().expect("Payload should be an object");
    assert_eq!(obj.get("name"), Some(&Variant::from("test")));
    assert_eq!(obj.get("value"), Some(&Variant::from(42)));
}

/// JSON: parse a JSON string (explicit obj mode).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn json_parse_explicit() {
    let flow = FlowBuilder::new().json_node("1", "obj", json!([["99"]])).test_sink("99").to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "{\"a\":1,\"b\":\"two\"}"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let obj = payload.as_object().expect("Payload should be an object");
    assert_eq!(obj.get("a"), Some(&Variant::from(1)));
    assert_eq!(obj.get("b"), Some(&Variant::from("two")));
}

/// JSON: stringify an object to a JSON string (auto mode).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn json_stringify_object() {
    let flow = FlowBuilder::new().json_node("1", "", json!([["99"]])).test_sink("99").to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": {"name": "test", "count": 5}}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    assert!(payload.is_string(), "Payload should be a JSON string");
    let json_str = payload.as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(json_str).expect("Result should be valid JSON");
    assert_eq!(parsed["name"], "test");
    assert_eq!(parsed["count"], 5);
}

/// JSON: stringify an object (explicit str mode).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn json_stringify_explicit() {
    let flow = FlowBuilder::new().json_node("1", "str", json!([["99"]])).test_sink("99").to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": {"key": "value"}}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    assert!(payload.is_string(), "Payload should be a JSON string");
}

/// JSON: parse a JSON array.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn json_parse_array() {
    let flow = FlowBuilder::new().json_node("1", "", json!([["99"]])).test_sink("99").to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "[1,2,3]"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    assert!(payload.is_array(), "Payload should be an array");
    let arr = payload.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0], Variant::from(1));
    assert_eq!(arr[1], Variant::from(2));
    assert_eq!(arr[2], Variant::from(3));
}

/// JSON: stringify a boolean (auto mode should stringify).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn json_stringify_boolean() {
    let flow = FlowBuilder::new().json_node("1", "", json!([["99"]])).test_sink("99").to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": true}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    assert!(payload.is_string(), "Payload should be a stringified boolean");
    assert_eq!(payload.as_str().unwrap(), "true");
}

/// JSON: stringify a number (auto mode should stringify).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn json_stringify_number() {
    let flow = FlowBuilder::new().json_node("1", "", json!([["99"]])).test_sink("99").to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 42}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    assert!(payload.is_string(), "Payload should be a stringified number");
    assert_eq!(payload.as_str().unwrap(), "42");
}

/// JSON: pass through when payload is already a string and action is auto
/// (string that is NOT valid JSON should pass through).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn json_auto_invalid_string_passthrough() {
    let flow = FlowBuilder::new().json_node("1", "", json!([["99"]])).test_sink("99").to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "not-json"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert!(msgs[0].get("payload").is_some());
}

/// JSON: custom property target.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn json_custom_property() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "json", "z": "100", "name": "",
         "property": "data", "action": "", "pretty": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"data": "{\"parsed\": true}"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let data = msgs[0].get("data").expect("Missing data property");
    let obj = data.as_object().expect("data should be a parsed object");
    assert_eq!(obj.get("parsed"), Some(&Variant::from(true)));
}
