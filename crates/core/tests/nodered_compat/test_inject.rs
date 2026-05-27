//! Inject node compatibility tests.
//!
//! Verifies that the inject node produces messages matching Node-RED behavior
//! for different payload types and configurations.

use serde_json::json;

use super::flow_builder::FlowBuilder;
use super::harness::{TestHarness, assert_msg_bool, assert_msg_has, assert_msg_num, assert_msg_str};
use rust_red_core::runtime::model::Variant;

/// Inject with string payload.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inject_string_payload() {
    let flow = FlowBuilder::new().inject_once("1", "hello world", "str", json!([["99"]])).test_sink("99").into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "hello world");
}

/// Inject with numeric payload.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inject_numeric_payload() {
    let flow = FlowBuilder::new().inject_once("1", "42", "num", json!([["99"]])).test_sink("99").into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_num(&msgs[0], "payload", 42);
}

/// Inject with boolean payload (true).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inject_boolean_true_payload() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "true", "vt": "bool"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "true", "payloadType": "bool",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_bool(&msgs[0], "payload", true);
}

/// Inject with boolean payload (false).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inject_boolean_false_payload() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "false", "vt": "bool"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "false", "payloadType": "bool",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_bool(&msgs[0], "payload", false);
}

/// Inject with JSON object payload.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inject_json_object_payload() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "{\"a\":1,\"b\":2}", "vt": "json"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "{\"a\":1,\"b\":2}", "payloadType": "json",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let obj = payload.as_object().expect("Payload should be an object");
    assert_eq!(obj.get("a"), Some(&Variant::from(1)));
    assert_eq!(obj.get("b"), Some(&Variant::from(2)));
}

/// Inject with a topic property.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inject_with_topic() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [
            {"p": "payload", "v": "test-data", "vt": "str"},
            {"p": "topic", "v": "my-topic", "vt": "str"}
         ],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "my-topic", "payload": "test-data", "payloadType": "str",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "test-data");
    assert_msg_str(&msgs[0], "topic", "my-topic");
}

/// Inject with timestamp (date type).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inject_timestamp_payload() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "", "vt": "date"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "", "payloadType": "date",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    assert!(payload.is_number(), "Timestamp payload should be a number");
    let ts = payload.as_number().unwrap().as_f64().unwrap();
    assert!(ts > 1577836800000.0, "Timestamp should be after 2020");
}

/// Inject should generate a _msgid.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inject_generates_msgid() {
    let flow = FlowBuilder::new().inject_once("1", "test", "str", json!([["99"]])).test_sink("99").into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_has(&msgs[0], "_msgid");
}

/// Inject with multiple properties (payload and custom).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inject_multiple_properties() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [
            {"p": "payload", "v": "hello", "vt": "str"},
            {"p": "topic", "v": "test-topic", "vt": "str"},
            {"p": "count", "v": "5", "vt": "num"}
         ],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "test-topic", "payload": "hello", "payloadType": "str",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "hello");
    assert_msg_str(&msgs[0], "topic", "test-topic");
    assert_msg_num(&msgs[0], "count", 5);
}
