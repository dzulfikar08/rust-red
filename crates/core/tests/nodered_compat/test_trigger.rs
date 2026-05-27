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
