//! Template node compatibility tests.
//!
//! Verifies that the template node renders mustache templates correctly,
//! matching Node-RED behavior.

use std::time::Duration;

use serde_json::json;

use super::flow_builder::FlowBuilder;
use super::harness::{TestHarness, assert_msg_str};
use rust_red_core::runtime::model::Variant;

/// Template: basic mustache substitution with payload.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_basic_mustache() {
    let flow = FlowBuilder::new()
        .template("1", "Hello {{payload}}!", "payload", "str", json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "World"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "Hello World!");
}

/// Template: mustache with topic property.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_with_topic() {
    let flow = FlowBuilder::new()
        .template("1", "Topic: {{topic}}, Data: {{payload}}", "payload", "str", json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "hello", "topic": "greeting"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "Topic: greeting, Data: hello");
}

/// Template: store result in a different field.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_custom_field() {
    let flow = FlowBuilder::new()
        .template("1", "Value: {{payload}}", "result", "str", json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "42"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "42");
    assert_msg_str(&msgs[0], "result", "Value: 42");
}

/// Template: JSON output mode.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_json_output() {
    let flow = FlowBuilder::new()
        .template("1", "{\"greeting\": \"Hello {{payload}}\"}", "result", "json", json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "World"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let result = msgs[0].get("result").expect("Missing result field");
    let obj = result.as_object().expect("Result should be an object");
    assert_eq!(obj.get("greeting"), Some(&Variant::from("Hello World")));
}

/// Template: plain syntax (no substitution).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_plain_syntax() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "template", "z": "100", "wires": [["99"]],
         "template": "payload={{payload}}", "field": "payload",
         "output": "str", "syntax": "plain"},
        {"id": "99", "z": "100", "type": "test-once"},
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "foo"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "payload={{payload}}");
}

/// Template: empty template string does not crash.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_empty_template() {
    let flow = FlowBuilder::new().template("1", "", "payload", "str", json!([["99"]])).test_sink("99").into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "foo"}), 1, Duration::from_millis(200)).await;

    assert!(msgs.is_empty());
}

/// Template: mustache with numeric payload.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_numeric_payload() {
    let flow = FlowBuilder::new()
        .template("1", "The answer is {{payload}}", "payload", "str", json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 42}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "The answer is 42");
}

/// Template: mustache with nested object property.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_nested_property() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "template", "z": "100", "wires": [["99"]],
         "template": "Name: {{payload.name}}, Age: {{payload.age}}",
         "field": "result", "output": "str"},
        {"id": "99", "z": "100", "type": "test-once"},
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": {"name": "Alice", "age": "30"}}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "result", "Name: Alice, Age: 30");
}
