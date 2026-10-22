//! JSONata compatibility tests.
//!
//! JSONata is used by the change, switch and inject nodes (and by environment variables). These
//! tests check the Node-RED specific parts: the `msg` root, `$flowContext`, `$env`, the `$I`/`$N`
//! bindings of the switch `jsonata_exp` rule, and inject properties that see earlier properties.

use serde_json::json;

use super::flow_builder::{FlowBuilder, change_rule, switch_rule};
use super::harness::{TestHarness, assert_msg_num, assert_msg_str};
use rust_red_core::runtime::model::Variant;

/// Change: `jsonata` as the type of the value to set.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn jsonata_change_set_from_expression() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::set("payload", "msg", "$length(payload) + count", "jsonata")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "Hello", "count": 2}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_num(&msgs[0], "payload", 7);
}

/// Expressions written against the legacy `msg.` root still work.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn jsonata_change_legacy_msg_root() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::set("out", "msg", "msg.payload.name", "jsonata")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": {"name": "pump"}}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "out", "pump");
}

/// An expression that matches nothing (JSONata `undefined`) yields null, not an error.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn jsonata_change_undefined_is_null() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::set("out", "msg", "no.such.path", "jsonata")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 1}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].get("out"), Some(&Variant::Null));
}

/// `$flowContext(key)` reads the flow context.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn jsonata_flow_context_function() {
    let flow = FlowBuilder::new()
        .change(
            "1",
            vec![
                change_rule::set("k", "flow", "5", "num"),
                change_rule::set("payload", "msg", "$flowContext(\"k\") * 2", "jsonata"),
            ],
            json!([["99"]]),
        )
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 0}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_num(&msgs[0], "payload", 10);
}

/// `$globalContext(key)` reads the global context.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn jsonata_global_context_function() {
    let flow = FlowBuilder::new()
        .change(
            "1",
            vec![
                change_rule::set("g", "global", "7", "num"),
                change_rule::set("payload", "msg", "$globalContext(\"g\") + 1", "jsonata"),
            ],
            json!([["99"]]),
        )
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 0}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_num(&msgs[0], "payload", 8);
}

/// `$env(name)` is empty for an unknown variable, like in Node-RED.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn jsonata_env_function_unknown_is_empty() {
    let flow = FlowBuilder::new()
        .change(
            "1",
            vec![change_rule::set("payload", "msg", "$env(\"RUST_RED_NO_SUCH_VARIABLE\")", "jsonata")],
            json!([["99"]]),
        )
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 0}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "");
}

/// `$env(name)` resolves node-level environment variables.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn jsonata_env_function_reads_node_env() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "change", "name": "",
         "env": [{"name": "GREETING", "value": "hi", "type": "str"}],
         "rules": [change_rule::set("payload", "msg", "$env(\"GREETING\") & \" there\"", "jsonata")],
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 0}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "hi there");
}

/// An invalid expression is an error, not a silent pass.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn jsonata_change_invalid_expression_does_not_set() {
    let flow = FlowBuilder::new()
        .change("1", vec![change_rule::set("out", "msg", "$length(", "jsonata")], json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": 1}), 1, std::time::Duration::from_secs(2)).await;

    assert!(msgs.iter().all(|m| m.get("out").is_none()), "a broken expression must not produce a value");
}

fn jsonata_switch_flow(rule_expression: &str) -> serde_json::Value {
    FlowBuilder::new()
        .switch(
            "1",
            "payload",
            vec![switch_rule::jsonata(rule_expression), switch_rule::else_rule()],
            false,
            2,
            json!([["10"], ["11"]]),
        )
        .change("10", vec![change_rule::set("route", "msg", "match", "str")], json!([["99"]]))
        .change("11", vec![change_rule::set("route", "msg", "else", "str")], json!([["99"]]))
        .test_sink("99")
        .into_json()
}

/// Switch: a `jsonata_exp` rule matches when the expression evaluates to `true`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn jsonata_switch_exp_rule() {
    let harness = TestHarness::from_flow_json(jsonata_switch_flow("payload > 5"));
    let msgs = harness.inject_and_collect("1", json!({"payload": 10}), 1).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "route", "match");

    let harness = TestHarness::from_flow_json(jsonata_switch_flow("payload > 5"));
    let msgs = harness.inject_and_collect("1", json!({"payload": 1}), 1).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "route", "else");
}

/// Switch: `$I` and `$N` are the index and count from `msg.parts`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn jsonata_switch_exp_rule_index_and_count() {
    let expression = "$I = $N - 1";

    let harness = TestHarness::from_flow_json(jsonata_switch_flow(expression));
    let msgs = harness.inject_and_collect("1", json!({"payload": 1, "parts": {"index": 2, "count": 3}}), 1).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "route", "match");

    let harness = TestHarness::from_flow_json(jsonata_switch_flow(expression));
    let msgs = harness.inject_and_collect("1", json!({"payload": 1, "parts": {"index": 0, "count": 3}}), 1).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "route", "else");
}

/// Inject: JSONata properties work without an incoming message and see the properties before them.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn jsonata_inject_property_sees_earlier_properties() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [
            {"p": "x", "v": "3", "vt": "num"},
            {"p": "y", "v": "x + 2", "vt": "jsonata"},
            {"p": "payload", "v": "$millis() > 0", "vt": "jsonata"}
         ],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0, "topic": "",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_num(&msgs[0], "y", 5);
    assert_eq!(msgs[0].get("payload"), Some(&Variant::Bool(true)));
}
