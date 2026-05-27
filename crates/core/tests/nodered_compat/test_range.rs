//! Range node compatibility tests.
//!
//! Verifies that the range node scales, clamps, and wraps values correctly,
//! matching Node-RED behavior.

use serde_json::json;

use super::harness::{TestHarness, assert_msg_f64};

/// Range: scale 5 from [0,10] to [0,100] → 50.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn range_scale_basic() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "range",
         "action": "scale", "minin": 0, "maxin": 10,
         "minout": 0, "maxout": 100, "round": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 5}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_f64(&msgs[0], "payload", 50.0);
}

/// Range: scale 3 from [0,10] to [0,100] with round=true → 30.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn range_scale_with_round() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "range",
         "action": "scale", "minin": 0, "maxin": 10,
         "minout": 0, "maxout": 100, "round": true,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 3}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_f64(&msgs[0], "payload", 30.0);
}

/// Range: clamp input 15 (outside [0,10]) → clamped to 100 in [0,100] output.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn range_clamp() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "range",
         "action": "clamp", "minin": 0, "maxin": 10,
         "minout": 0, "maxout": 100, "round": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 15}), 1).await;

    assert_eq!(msgs.len(), 1);
    // 15 clamped to 10, then scaled: (10 - 0)/(10 - 0) * (100 - 0) + 0 = 100
    assert_msg_f64(&msgs[0], "payload", 100.0);
}

/// Range: wrap (roll) input 12 within [0,10] → wraps to scaled value.
/// 12 wrapped in [0,10]: ((12 - 0) % 10 + 10) % 10 + 0 = 2
/// Then scaled: (2 - 0)/(10 - 0) * (100 - 0) + 0 = 20
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn range_wrap() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "range",
         "action": "roll", "minin": 0, "maxin": 10,
         "minout": 0, "maxout": 100, "round": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 12}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_f64(&msgs[0], "payload", 20.0);
}

/// Range: drop action — value within range passes through, scaled.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn range_drop_in_range() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "range",
         "action": "drop", "minin": 0, "maxin": 10,
         "minout": 0, "maxout": 100, "round": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 5}), 1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_f64(&msgs[0], "payload", 50.0);
}

/// Range: drop action — value out of range is dropped (no output).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn range_drop_out_of_range() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "range",
         "action": "drop", "minin": 0, "maxin": 10,
         "minout": 0, "maxout": 100, "round": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    // Use timeout variant since drop produces no output, which would cause default to timeout
    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": 15}), 1, std::time::Duration::from_millis(500)).await;

    assert!(msgs.is_empty(), "Out-of-range value should be dropped, got {} messages", msgs.len());
}

/// Range: scale with fractional result and no rounding.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn range_scale_fractional() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "range",
         "action": "scale", "minin": 0, "maxin": 10,
         "minout": 0, "maxout": 100, "round": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": 1}), 1).await;

    assert_eq!(msgs.len(), 1);
    // (1 - 0)/(10 - 0) * (100 - 0) + 0 = 10
    assert_msg_f64(&msgs[0], "payload", 10.0);
}
