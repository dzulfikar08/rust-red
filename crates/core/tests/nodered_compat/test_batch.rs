//! Batch node compatibility tests.
//!
//! Verifies that the batch node correctly groups messages into batches,
//! matching Node-RED behavior for count mode, overlap, honour_parts,
//! and reset handling.

use std::time::Duration;

use serde_json::json;

use super::harness::{TestHarness, assert_msg_has};

/// Batch: batch messages by count (group of N messages).
// Mirrors: Node-RED 19-batch_spec.js "should batch messages by count"
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn batch_by_count() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "batch", "z": "100", "name": "",
         "mode": "count", "count": 3, "overlap": 0,
         "interval": 1, "allow_empty_sequence": false,
         "topics": [], "honour_parts": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .run_with_inject(
            3,
            vec![
                ("1".to_string(), json!({"payload": "a"})),
                ("1".to_string(), json!({"payload": "b"})),
                ("1".to_string(), json!({"payload": "c"})),
            ],
        )
        .await;

    assert_eq!(msgs.len(), 3, "Should produce 3 messages for batch of 3");
    for msg in &msgs {
        assert_msg_has(msg, "parts");
    }
    assert_eq!(msgs[0].get("payload").unwrap().as_str().unwrap(), "a");
    assert_eq!(msgs[1].get("payload").unwrap().as_str().unwrap(), "b");
    assert_eq!(msgs[2].get("payload").unwrap().as_str().unwrap(), "c");
}

/// Batch: batch messages by count with overlap.
// Mirrors: Node-RED 19-batch_spec.js "should batch messages with overlap"
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn batch_with_overlap() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "batch", "z": "100", "name": "",
         "mode": "count", "count": 3, "overlap": 1,
         "interval": 1, "allow_empty_sequence": false,
         "topics": [], "honour_parts": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .run_with_inject(
            6,
            vec![
                ("1".to_string(), json!({"payload": "a"})),
                ("1".to_string(), json!({"payload": "b"})),
                ("1".to_string(), json!({"payload": "c"})),
                ("1".to_string(), json!({"payload": "d"})),
                ("1".to_string(), json!({"payload": "e"})),
            ],
        )
        .await;

    assert_eq!(msgs.len(), 6, "Should produce 6 messages from 2 batches of 3");
    assert_eq!(msgs[0].get("payload").unwrap().as_str().unwrap(), "a");
    assert_eq!(msgs[1].get("payload").unwrap().as_str().unwrap(), "b");
    assert_eq!(msgs[2].get("payload").unwrap().as_str().unwrap(), "c");
    assert_eq!(msgs[3].get("payload").unwrap().as_str().unwrap(), "c");
    assert_eq!(msgs[4].get("payload").unwrap().as_str().unwrap(), "d");
    assert_eq!(msgs[5].get("payload").unwrap().as_str().unwrap(), "e");
}

/// Batch: reset should clear pending messages.
// Mirrors: Node-RED 19-batch_spec.js "should reset and clear pending"
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn batch_reset_clears_pending() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "batch", "z": "100", "name": "",
         "mode": "count", "count": 3, "overlap": 0,
         "interval": 1, "allow_empty_sequence": false,
         "topics": [], "honour_parts": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"reset": true}), 1, Duration::from_millis(500)).await;

    assert!(msgs.is_empty(), "Reset should produce no output");
}

/// Batch: honour_parts triggers flush on end-of-sequence.
// Mirrors: Node-RED 19-batch_spec.js "should flush on end-of-sequence when honour_parts is true"
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn batch_honour_parts() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "batch", "z": "100", "name": "",
         "mode": "count", "count": 100, "overlap": 0,
         "interval": 1, "allow_empty_sequence": false,
         "topics": [], "honour_parts": true,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .run_with_inject(
            3,
            vec![
                ("1".to_string(), json!({"payload": "x", "parts": {"id": "g1", "index": 0, "count": 3}})),
                ("1".to_string(), json!({"payload": "y", "parts": {"id": "g1", "index": 1, "count": 3}})),
                ("1".to_string(), json!({"payload": "z", "parts": {"id": "g1", "index": 2, "count": 3}})),
            ],
        )
        .await;

    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0].get("payload").unwrap().as_str().unwrap(), "x");
    assert_eq!(msgs[1].get("payload").unwrap().as_str().unwrap(), "y");
    assert_eq!(msgs[2].get("payload").unwrap().as_str().unwrap(), "z");
}

/// Batch: single-message batches (count=1).
// Mirrors: Node-RED 19-batch_spec.js "should emit every message as a batch of 1"
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn batch_count_one() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "batch", "z": "100", "name": "",
         "mode": "count", "count": 1, "overlap": 0,
         "interval": 1, "allow_empty_sequence": false,
         "topics": [], "honour_parts": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .run_with_inject(
            2,
            vec![("1".to_string(), json!({"payload": "first"})), ("1".to_string(), json!({"payload": "second"}))],
        )
        .await;

    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].get("payload").unwrap().as_str().unwrap(), "first");
    assert_eq!(msgs[1].get("payload").unwrap().as_str().unwrap(), "second");
}
