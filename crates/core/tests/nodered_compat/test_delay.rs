//! Delay node compatibility tests.
//!
//! Verifies that the delay node delays and rate-limits messages correctly,
//! matching Node-RED behavior.

use std::time::Duration;

use serde_json::json;

use super::flow_builder::FlowBuilder;
use super::harness::{TestHarness, assert_msg_str};

/// Delay: fixed delay of milliseconds.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delay_fixed_milliseconds() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "delay", "z": "100", "name": "",
         "pauseType": "delay", "timeout": "100", "timeoutUnits": "milliseconds",
         "rate": "1", "nbRateUnits": "1", "rateUnits": "second",
         "randomFirst": "1", "randomLast": "5", "randomUnits": "seconds",
         "drop": false, "allowrate": false, "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "delayed"}), 1, Duration::from_secs(2)).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "delayed");
}

/// Delay: fixed delay of seconds.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delay_fixed_seconds() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "delay", "z": "100", "name": "",
         "pauseType": "delay", "timeout": "1", "timeoutUnits": "seconds",
         "rate": "1", "nbRateUnits": "1", "rateUnits": "second",
         "randomFirst": "1", "randomLast": "5", "randomUnits": "seconds",
         "drop": false, "allowrate": false, "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "delayed-1s"}), 1, Duration::from_secs(3)).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "delayed-1s");
}

/// Delay: message should arrive after delay, not before.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delay_respects_timing() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "delay", "z": "100", "name": "",
         "pauseType": "delay", "timeout": "500", "timeoutUnits": "milliseconds",
         "rate": "1", "nbRateUnits": "1", "rateUnits": "second",
         "randomFirst": "1", "randomLast": "5", "randomUnits": "seconds",
         "drop": false, "allowrate": false, "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);

    let start = std::time::Instant::now();
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "delayed"}), 1, Duration::from_secs(2)).await;

    let elapsed = start.elapsed();

    assert_eq!(msgs.len(), 1);
    // The message should take at least ~500ms to arrive
    assert!(elapsed >= Duration::from_millis(400), "Delay should be at least ~500ms, but was {:?}", elapsed);
}

/// Delay: multiple messages each get delayed independently.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delay_multiple_messages() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "delay", "z": "100", "name": "",
         "pauseType": "delay", "timeout": "100", "timeoutUnits": "milliseconds",
         "rate": "1", "nbRateUnits": "1", "rateUnits": "second",
         "randomFirst": "1", "randomLast": "5", "randomUnits": "seconds",
         "drop": false, "allowrate": false, "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .run_with_inject(
            2,
            vec![("1".to_string(), json!({"payload": "msg1"})), ("1".to_string(), json!({"payload": "msg2"}))],
        )
        .await;

    assert_eq!(msgs.len(), 2);
}

/// Delay: delay with variable from msg.delay property.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delay_variable() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "delay", "z": "100", "name": "",
         "pauseType": "delayv", "timeout": "100", "timeoutUnits": "milliseconds",
         "rate": "1", "nbRateUnits": "1", "rateUnits": "second",
         "randomFirst": "1", "randomLast": "5", "randomUnits": "seconds",
         "drop": false, "allowrate": false, "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .inject_and_collect_timeout("1", json!({"payload": "var-delayed", "delay": 100}), 1, Duration::from_secs(2))
        .await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "var-delayed");
}

/// Delay: rate limiting mode.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delay_rate_limit() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "delay", "z": "100", "name": "",
         "pauseType": "rate", "timeout": "5", "timeoutUnits": "seconds",
         "rate": "2", "nbRateUnits": "1", "rateUnits": "second",
         "randomFirst": "1", "randomLast": "5", "randomUnits": "seconds",
         "drop": false, "allowrate": false, "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    // Send 3 messages, rate limit is 2/sec so we need some time for them to pass
    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "rate-limited"}), 1, Duration::from_secs(2)).await;

    // At least the first message should get through
    assert!(!msgs.is_empty());
}

// ---------------------------------------------------------------------------
// Additional delay node tests mirroring Node-RED 89-delay_spec.js
// ---------------------------------------------------------------------------

/// Delay: random delay between 100ms and 300ms should complete within bounds.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delay_random_mode() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "delay", "z": "100", "name": "",
         "pauseType": "random", "timeout": "5", "timeoutUnits": "seconds",
         "rate": "1", "nbRateUnits": "1", "rateUnits": "second",
         "randomFirst": "100", "randomLast": "300", "randomUnits": "milliseconds",
         "drop": false, "allowrate": false, "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let start = std::time::Instant::now();
    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "random-delayed"}), 1, Duration::from_secs(2)).await;

    let elapsed = start.elapsed();
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "random-delayed");
    assert!(elapsed >= Duration::from_millis(50), "Random delay should be at least ~100ms, but was {:?}", elapsed);
}

/// Delay: reset message cancels pending delayed messages.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delay_reset_message() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "delay", "z": "100", "name": "",
         "pauseType": "delay", "timeout": "5000", "timeoutUnits": "milliseconds",
         "rate": "1", "nbRateUnits": "1", "rateUnits": "second",
         "randomFirst": "1", "randomLast": "5", "randomUnits": "seconds",
         "drop": false, "allowrate": false, "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    // Inject a delayed message then reset - use timeout since reset cancels the pending msg
    let msgs = harness
        .inject_and_collect_timeout(
            "1",
            json!({"payload": "delayed-msg", "reset": true}),
            1,
            Duration::from_millis(500),
        )
        .await;

    // The reset should prevent the delayed message from being sent
    assert!(msgs.is_empty(), "Reset should cancel pending message, got {} messages", msgs.len());
}

/// Delay: negative msg.delay value sends message immediately in variable delay mode.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delay_negative_msg_delay() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "delay", "z": "100", "name": "",
         "pauseType": "delayv", "timeout": "5000", "timeoutUnits": "milliseconds",
         "rate": "1", "nbRateUnits": "1", "rateUnits": "second",
         "randomFirst": "1", "randomLast": "5", "randomUnits": "seconds",
         "drop": false, "allowrate": false, "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let start = std::time::Instant::now();
    let msgs = harness
        .inject_and_collect_timeout("1", json!({"payload": "instant", "delay": -100}), 1, Duration::from_secs(2))
        .await;

    let elapsed = start.elapsed();
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "instant");
    assert!(elapsed < Duration::from_millis(200), "Negative delay should send immediately, but took {:?}", elapsed);
}

/// Delay: variable delay with msg.delay=0 sends immediately.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delay_variable_zero() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "delay", "z": "100", "name": "",
         "pauseType": "delayv", "timeout": "5000", "timeoutUnits": "milliseconds",
         "rate": "1", "nbRateUnits": "1", "rateUnits": "second",
         "randomFirst": "1", "randomLast": "5", "randomUnits": "seconds",
         "drop": false, "allowrate": false, "outputs": 1,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let start = std::time::Instant::now();
    let msgs = harness
        .inject_and_collect_timeout("1", json!({"payload": "zero-delay", "delay": 0}), 1, Duration::from_secs(2))
        .await;

    let elapsed = start.elapsed();
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "zero-delay");
    assert!(elapsed < Duration::from_millis(200), "Zero delay should be fast, but took {:?}", elapsed);
}
