//! Change node compatibility tests.
//!
//! Verifies that the change node modifies message properties correctly,
//! matching Node-RED behavior for set, delete, change, and move operations.

use serde_json::json;

use super::flow_builder::{FlowBuilder, change_rule};
use super::harness::{TestHarness, assert_msg_not_has, assert_msg_str};
use rust_red_core::runtime::model::Variant;

/// Change: set property to a string value.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_set_string() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::set("payload", "msg", "new-value", "str")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "old"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "new-value");
}

/// Change: set property to a number value.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_set_number() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::set("payload", "msg", "42", "num")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "old"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    assert_eq!(payload, &Variant::from(42));
}

/// Change: set property to a boolean value.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_set_boolean() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::set("payload", "msg", "true", "bool")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "old"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["payload"], true.into());
}

/// Change: set property from another message property.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_set_from_another_property() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::set("payload", "msg", "topic", "msg")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "old", "topic": "source-value"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "source-value");
}

/// Change: set a nested property.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_set_nested_property() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::set("result", "msg", "hello", "str")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "data"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "result", "hello");
    assert_msg_str(&msgs[0], "payload", "data");
}

/// Change: delete a property.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_delete_property() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::delete("topic", "msg")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "data", "topic": "to-delete"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_not_has(&msgs[0], "topic");
    assert_msg_str(&msgs[0], "payload", "data");
}

/// Change: delete payload property.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_delete_payload() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::delete("payload", "msg")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "to-delete", "topic": "keep"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_not_has(&msgs[0], "payload");
    assert_msg_str(&msgs[0], "topic", "keep");
}

/// Change: move property from one key to another.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_move_property() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::move_rule("source", "msg", "dest", "msg")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"source": "moved-value", "payload": "keep"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "dest", "moved-value");
    assert_msg_not_has(&msgs[0], "source");
    assert_msg_str(&msgs[0], "payload", "keep");
}

/// Change: string replacement (change rule).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_string_replacement() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::change("payload", "msg", "old", "str", "new", "str")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "replace old value"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "replace new value");
}

/// Change: multiple rules applied in sequence.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_multiple_rules() {
    let flow = FlowBuilder::new()
        .change(
            "1",
            vec![
                change_rule::set("a", "msg", "1", "str"),
                change_rule::set("b", "msg", "2", "str"),
                change_rule::delete("payload", "msg"),
            ],
            json!([["99"]]),
        )
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "original"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_not_has(&msgs[0], "payload");
    assert_msg_str(&msgs[0], "a", "1");
    assert_msg_str(&msgs[0], "b", "2");
}

/// Change: set property to JSON object.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_set_json_object() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::set("payload", "msg", "{\"key\":\"value\"}", "json")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "old"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let obj = payload.as_object().expect("Payload should be an object");
    assert_eq!(obj.get("key"), Some(&Variant::from("value")));
}

/// Change: set then delete the same property.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_set_then_delete() {
    let flow = FlowBuilder::new()
        .change(
            "1",
            vec![change_rule::set("temp", "msg", "temporary", "str"), change_rule::delete("temp", "msg")],
            json!([["99"]]),
        )
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "data"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_not_has(&msgs[0], "temp");
}

// ---------------------------------------------------------------------------
// Additional change node tests mirroring Node-RED 15-change_spec.js
// ---------------------------------------------------------------------------

/// Change: set a multi-level nested property like "result.data".
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_set_multi_level_property() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::set("result.data", "msg", "nested-value", "str")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "original"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let result = msgs[0].get("result").expect("Missing result property");
    let obj = result.as_object().expect("result should be an object");
    assert_eq!(obj.get("data"), Some(&Variant::from("nested-value")), "result.data should be 'nested-value'");
    assert_msg_str(&msgs[0], "payload", "original");
}

/// Change: set property from a nested msg property like "source.value".
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_set_from_nested_msg_property() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::set("payload", "msg", "source.value", "msg")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "old", "source": {"value": "from-nested"}}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "from-nested");
}

/// Change: change rule with regex from type replaces pattern matches.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_with_regex() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::change("payload", "msg", "\\d+", "re", "NUM", "str")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "order123"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "orderNUM");
}

/// Change: move a top-level property to a nested destination.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_move_to_sub_property() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::move_rule("source", "msg", "target.inner", "msg")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"source": "moved-value", "payload": "keep"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_not_has(&msgs[0], "source");
    let target = msgs[0].get("target").expect("Missing target property");
    let target_obj = target.as_object().expect("target should be an object");
    assert_eq!(target_obj.get("inner"), Some(&Variant::from("moved-value")));
    assert_msg_str(&msgs[0], "payload", "keep");
}

/// Change: move a nested property to a top-level destination.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_move_from_sub_property() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::move_rule("source.inner", "msg", "dest", "msg")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"source": {"inner": "deep-value"}, "payload": "keep"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "dest", "deep-value");
    assert_msg_str(&msgs[0], "payload", "keep");
}

/// Change: complex chaining of set, change, delete, and move in sequence.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_complex_multi_rule_chain() {
    let flow = FlowBuilder::new()
        .change(
            "1",
            vec![
                change_rule::set("header", "msg", "topic", "msg"),
                change_rule::change("payload", "msg", "hello", "str", "world", "str"),
                change_rule::move_rule("temp", "msg", "result", "msg"),
                change_rule::delete("topic", "msg"),
            ],
            json!([["99"]]),
        )
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .inject_and_collect(
            "1",
            json!({
                "payload": "say hello there",
                "topic": "my-topic",
                "temp": "temp-value"
            }),
            1,
        )
        .await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "say world there");
    assert_msg_str(&msgs[0], "header", "my-topic");
    assert_msg_str(&msgs[0], "result", "temp-value");
    assert_msg_not_has(&msgs[0], "topic");
    assert_msg_not_has(&msgs[0], "temp");
}

/// Change: deleting a non-existent property does not error.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_delete_non_existent() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::delete("nonexistent", "msg")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "data"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "data");
}

/// Change: replace a numeric payload with another number when values match exactly.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_replace_number() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::change("payload", "msg", "5", "num", "10", "num")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 5}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["payload"], 10.into());
}

/// Change: replace a boolean payload value when values match exactly.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn change_replace_boolean() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::change("payload", "msg", "true", "bool", "false", "bool")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": true}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["payload"], false.into());
}
