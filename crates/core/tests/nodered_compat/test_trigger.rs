//! Trigger node compatibility tests.
//!
//! Verifies that the trigger node sends op1 immediately and op2 after a delay,
//! and that reset messages cancel pending triggers, matching Node-RED behavior.

use std::time::Duration;

use serde_json::json;

use super::harness::{TestHarness, assert_msg_str};

/// Trigger: send op1 immediately, then send op2 after duration (wait-then-send).
/// Default config: op1="1", op2="0", duration=250ms.
/// With op1type=nul (null), only op2 is sent after the delay.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn trigger_send_then_wait() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "trigger",
         "op1": "1", "op2": "0", "op1type": "str", "op2type": "str",
         "duration": "100", "units": "ms",
         "extend": false, "overrideDelay": false,
         "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "test"}), 1, Duration::from_secs(2)).await;

    // Should get at least the first message (op1="1") immediately
    assert!(!msgs.is_empty(), "Expected at least one message from trigger");
    assert_msg_str(&msgs[0], "payload", "1");
}

/// Trigger: wait-then-send — op1type is nul so no immediate message,
/// then op2 is sent after duration.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn trigger_wait_then_send() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "trigger",
         "op1": "", "op2": "0", "op1type": "nul", "op2type": "str",
         "duration": "100", "units": "ms",
         "extend": false, "overrideDelay": false,
         "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "test"}), 1, Duration::from_secs(2)).await;

    assert!(!msgs.is_empty(), "Expected op2 message after delay");
    assert_msg_str(&msgs[0], "payload", "0");
}

/// Trigger: both op1 and op2 are sent (op1 immediately, op2 after delay).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn trigger_both_op1_and_op2() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "trigger",
         "op1": "first", "op2": "second", "op1type": "str", "op2type": "str",
         "duration": "100", "units": "ms",
         "extend": false, "overrideDelay": false,
         "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "test"}), 2, Duration::from_secs(2)).await;

    assert_eq!(msgs.len(), 2, "Expected both op1 and op2 messages");
    assert_msg_str(&msgs[0], "payload", "first");
    assert_msg_str(&msgs[1], "payload", "second");
}

/// Trigger: reset message cancels pending trigger.
/// Send a message that starts a trigger, then immediately send reset.
/// The second op2 should not arrive because reset cancels the timer.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn trigger_reset() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "trigger",
         "op1": "1", "op2": "0", "op1type": "str", "op2type": "str",
         "duration": "5000", "units": "ms",
         "extend": false, "overrideDelay": false,
         "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    // Inject message with reset=true — this should trigger op1 but also cancel due to reset
    // In Node-RED, msg.reset cancels the pending timer. If reset is present, the node
    // cancels any pending timeout and returns without sending.
    let msgs = harness
        .inject_and_collect_timeout("1", json!({"payload": "test", "reset": true}), 1, Duration::from_millis(500))
        .await;

    // With reset=true, the trigger node should cancel the pending event and not send
    assert!(msgs.is_empty(), "Reset should cancel pending trigger, got {} messages", msgs.len());
}

/// Trigger: extend flag extends the timer on each message.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn trigger_extend() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "trigger",
         "op1": "1", "op2": "0", "op1type": "str", "op2type": "str",
         "duration": "100", "units": "ms",
         "extend": true, "overrideDelay": false,
         "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "test"}), 2, Duration::from_secs(2)).await;

    // With extend, first message sends op1, then after duration sends op2
    assert!(!msgs.is_empty(), "Expected messages with extend");
    assert_msg_str(&msgs[0], "payload", "1");
}

fn trigger_flow(op1type: &str, op1: &str, extra: serde_json::Value) -> serde_json::Value {
    let mut node = json!({
        "id": "1", "z": "100", "type": "trigger",
        "op1": op1, "op2": "0", "op1type": op1type, "op2type": "str",
        "duration": "100", "units": "ms",
        "extend": false, "overrideDelay": false,
        "outputs": 1,
        "wires": [["99"]]
    });
    for (k, v) in extra.as_object().unwrap() {
        node[k] = v.clone();
    }
    json!([
        {"id": "100", "type": "tab", "env": [{"name": "TRIG_ENV", "type": "str", "value": "from-env"}]},
        node,
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

/// Trigger: the `jsonata` value type evaluates an expression against the message.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn trigger_op1_jsonata() {
    let harness = TestHarness::from_flow_json(trigger_flow("jsonata", "payload & \"!\"", json!({})));
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "hi"}), 1, Duration::from_secs(2)).await;

    assert!(!msgs.is_empty(), "Expected op1 message");
    assert_msg_str(&msgs[0], "payload", "hi!");
}

/// Trigger: the `bin` value type builds a buffer from a JSON array.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn trigger_op1_bin() {
    let harness = TestHarness::from_flow_json(trigger_flow("bin", "[1,2,3]", json!({})));
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "x"}), 1, Duration::from_secs(2)).await;

    assert!(!msgs.is_empty(), "Expected op1 message");
    let payload = msgs[0].get("payload").expect("payload");
    assert_eq!(payload.as_bytes().expect("payload must be a buffer"), &[1u8, 2, 3][..]);
}

/// Trigger: the `env` value type reads an environment variable.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn trigger_op1_env() {
    let harness = TestHarness::from_flow_json(trigger_flow("env", "TRIG_ENV", json!({})));
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "x"}), 1, Duration::from_secs(2)).await;

    assert!(!msgs.is_empty(), "Expected op1 message");
    assert_msg_str(&msgs[0], "payload", "from-env");
}

/// Trigger: the `date` value type honours the requested format instead of always using the
/// epoch milliseconds.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn trigger_op1_date_iso() {
    let harness = TestHarness::from_flow_json(trigger_flow("date", "iso", json!({})));
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "x"}), 1, Duration::from_secs(2)).await;

    assert!(!msgs.is_empty(), "Expected op1 message");
    let payload = msgs[0].get("payload").and_then(|v| v.as_str()).expect("iso date must be a string").to_string();
    assert!(payload.contains('T'), "not an ISO date: {payload}");
}

/// Trigger: with `overrideDelay`, `msg.delay` is in milliseconds (it used to be multiplied by 1000).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn trigger_override_delay_is_milliseconds() {
    let flow = trigger_flow("nul", "", json!({"overrideDelay": true, "duration": "10", "units": "s"}));
    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "x", "delay": 100}), 1, Duration::from_secs(3)).await;

    assert!(!msgs.is_empty(), "Expected op2 after the 100ms override delay");
    assert_msg_str(&msgs[0], "payload", "0");
}
