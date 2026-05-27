//! Switch node compatibility tests.
//!
//! Verifies that the switch node routes messages to the correct output
//! ports based on rules, matching Node-RED behavior.

use std::time::Duration;

use serde_json::json;

use super::flow_builder::{FlowBuilder, switch_rule};
use super::harness::{TestHarness, assert_msg_has};

/// Switch: equality (==) rule with string.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_eq_string() {
    let flow = FlowBuilder::new()
        .switch(
            "1",
            "payload",
            vec![switch_rule::eq("Hello", "str"), switch_rule::else_rule()],
            true,
            2,
            json!([["99"], ["99"]]),
        )
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "Hello"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["payload"], "Hello".into());
}

/// Switch: not equal (!=) rule.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_neq_string() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::neq("Hello", "str")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "Goodbye"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["payload"], "Goodbye".into());
}

/// Switch: less than (<) rule with numbers.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_lt_number() {
    let flow = FlowBuilder::new()
        .switch(
            "1",
            "payload",
            vec![switch_rule::lt("10", "num"), switch_rule::else_rule()],
            true,
            2,
            json!([["99"], ["99"]]),
        )
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 5}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["payload"], 5.into());
}

/// Switch: greater than (>) rule.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_gt_number() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::gt("10", "num")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 15}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["payload"], 15.into());
}

/// Switch: greater than or equal (>=) rule.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_gte_number() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::gte("10", "num")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 10}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: less than or equal (<=) rule.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_lte_number() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::lte("10", "num")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 10}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: between rule (inclusive).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_between() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::btwn("3", "num", "5", "num")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 4}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["payload"], 4.into());
}

/// Switch: between rule - boundary check (lower bound).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_between_lower_bound() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::btwn("3", "num", "5", "num")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 3}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: contains rule.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_contains() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::cont("ello")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "Hello World"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["payload"], "Hello World".into());
}

/// Switch: is null rule.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_is_null() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::is_null()], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": null}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: is not null rule.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_is_not_null() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::is_not_null()], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "exists"}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: is true rule.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_is_true() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::is_true()], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": true}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: is false rule.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_is_false() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::is_false()], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": false}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: is empty rule.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_is_empty() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::is_empty()], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": ""}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: is not empty rule.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_is_not_empty() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::is_not_empty()], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "data"}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: else (default) rule - matches when no other rule matches.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_else_rule() {
    let flow = FlowBuilder::new()
        .switch(
            "1",
            "payload",
            vec![switch_rule::eq("specific", "str"), switch_rule::else_rule()],
            true,
            2,
            json!([["99"], ["99"]]),
        )
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "other"}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["payload"], "other".into());
}

/// Switch: type check with istype (string).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_istype_string() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::istype("string")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "a string"}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: type check with istype (number).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_istype_number() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::istype("number")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 42}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: regex rule.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_regex() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::regex("[abc]+")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "abc"}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: multiple rules with checkall=true.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_multiple_rules_checkall() {
    let flow = FlowBuilder::new()
        .switch(
            "1",
            "payload",
            vec![switch_rule::gt("5", "num"), switch_rule::lt("20", "num")],
            true,
            2,
            json!([["99"], ["99"]]),
        )
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 10}), 2).await;

    assert_eq!(msgs.len(), 2);
}

/// Switch: no matching rule - message should not pass through.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_no_match() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::eq("exact", "str")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let result =
        harness.inject_and_collect_timeout("1", json!({"payload": "other"}), 1, Duration::from_millis(200)).await;

    assert!(result.is_empty(), "Expected no messages when no rule matches");
}

// ---------------------------------------------------------------------------
// Additional switch node tests mirroring Node-RED 10-switch_spec.js
// ---------------------------------------------------------------------------

/// Switch: hask (has key) rule checks if an object has a specific key.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_hask_object_has_key() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![json!({"t": "hask", "v": "mykey", "vt": "str"})], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": {"mykey": "value"}}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: hask rule does not match when object lacks the key.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_hask_object_missing_key() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![json!({"t": "hask", "v": "missing", "vt": "str"})], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let result = harness
        .inject_and_collect_timeout("1", json!({"payload": {"other": "value"}}), 1, Duration::from_millis(200))
        .await;

    assert!(result.is_empty(), "hask should not match when key is missing");
}

/// Switch: between rule matches at upper bound (inclusive).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_between_upper_bound() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::btwn("3", "num", "5", "num")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 5}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: between rule still matches when bounds are reversed (commutative).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_between_reversed_bounds() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::btwn("5", "num", "3", "num")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 4}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: regex rule with case-insensitive flag.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_regex_case_insensitive() {
    let flow = FlowBuilder::new()
        .switch(
            "1",
            "payload",
            vec![json!({"t": "regex", "v": "hello", "vt": "str", "case": true})],
            true,
            1,
            json!([["99"]]),
        )
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "HELLO WORLD"}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: istype rule for boolean type.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_istype_boolean() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::istype("boolean")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": true}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: istype rule for array type.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_istype_array() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::istype("array")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": [1, 2, 3]}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: istype rule for object type.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_istype_object() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::istype("object")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": {"key": "val"}}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: istype rule for null type.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_istype_null() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::istype("null")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": null}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: checkall=false stops after first matching rule.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_checkall_false_stops_at_first() {
    let flow = FlowBuilder::new()
        .switch(
            "1",
            "payload",
            vec![switch_rule::gt("5", "num"), switch_rule::gt("3", "num")],
            false,
            2,
            json!([["99"], ["99"]]),
        )
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 10}), 1).await;

    assert_eq!(msgs.len(), 1, "checkall=false should stop at first matching rule");
}

/// Switch: eq rule comparing payload against msg.topic (vt: "msg").
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_eq_msg_property() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![json!({"t": "eq", "v": "topic", "vt": "msg"})], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "hello", "topic": "hello"}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: is_empty rule matches empty array.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_is_empty_array() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::is_empty()], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": []}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: is_empty rule matches empty object.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_is_empty_object() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::is_empty()], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": {}}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: is_not_empty rule matches non-empty array.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_is_not_empty_array() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::is_not_empty()], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": [1]}), 1).await;

    assert_eq!(msgs.len(), 1);
}

/// Switch: contains rule does not match when substring is absent.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_contains_not_matching() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![switch_rule::cont("xyz")], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let result =
        harness.inject_and_collect_timeout("1", json!({"payload": "Hello World"}), 1, Duration::from_millis(200)).await;

    assert!(result.is_empty(), "contains should not match when substring is absent");
}

/// Switch: prev value comparison - matching values pass after first message sets prev.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switch_prev_value_eq() {
    let flow = FlowBuilder::new()
        .switch("1", "payload", vec![json!({"t": "eq", "v": "", "vt": "prev"})], true, 1, json!([["99"]]))
        .test_sink("99")
        .to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .run_with_inject(
            1,
            vec![("1".to_string(), json!({"payload": "hello"})), ("1".to_string(), json!({"payload": "hello"}))],
        )
        .await;

    // First message sets prev, second matches prev
    assert_eq!(msgs.len(), 1, "Second message should match prev value");
}
