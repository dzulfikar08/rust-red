//! Sort node compatibility tests.
//!
//! Verifies that the sort node correctly sorts array payloads and message
//! sequences, matching Node-RED behavior.

use std::time::Duration;

use serde_json::json;

use super::harness::TestHarness;

/// Sort: sort an array of numbers in ascending order.
// Mirrors: Node-RED 18-sort_spec.js "should sort payload in ascending order"
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sort_array_numbers_ascending() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "sort", "z": "100", "name": "",
         "order": "ascending", "as_num": true,
         "target": "payload", "target_type": "msg", "key": "payload",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": [5, 3, 1, 4, 2]}), 1).await;

    assert_eq!(msgs.len(), 1);
    let arr = msgs[0].get("payload").unwrap().as_array().unwrap();
    let values: Vec<i64> = arr.iter().map(|v| v.as_i64().unwrap()).collect();
    assert_eq!(values, vec![1, 2, 3, 4, 5]);
}

/// Sort: sort an array of numbers in descending order.
// Mirrors: Node-RED 18-sort_spec.js "should sort payload in descending order"
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sort_array_numbers_descending() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "sort", "z": "100", "name": "",
         "order": "descending", "as_num": true,
         "target": "payload", "target_type": "msg", "key": "payload",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": [5, 3, 1, 4, 2]}), 1).await;

    assert_eq!(msgs.len(), 1);
    let arr = msgs[0].get("payload").unwrap().as_array().unwrap();
    let values: Vec<i64> = arr.iter().map(|v| v.as_i64().unwrap()).collect();
    assert_eq!(values, vec![5, 4, 3, 2, 1]);
}

/// Sort: sort an array of strings in ascending alphabetical order.
// Mirrors: Node-RED 18-sort_spec.js "should sort strings as strings"
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sort_array_strings_ascending() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "sort", "z": "100", "name": "",
         "order": "ascending", "as_num": false,
         "target": "payload", "target_type": "msg", "key": "payload",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": ["cherry", "apple", "banana"]}), 1).await;

    assert_eq!(msgs.len(), 1);
    let arr = msgs[0].get("payload").unwrap().as_array().unwrap();
    let values: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(values, vec!["apple", "banana", "cherry"]);
}

/// Sort: sort an array of objects by a key property (ascending, numeric).
// Mirrors: Node-RED 18-sort_spec.js "should sort array of objects by key property"
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sort_array_by_key() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "sort", "z": "100", "name": "",
         "order": "ascending", "as_num": true,
         "target": "payload", "target_type": "msg", "key": "age",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .inject_and_collect(
            "1",
            json!({"payload": [
                {"name": "Charlie", "age": 35},
                {"name": "Alice", "age": 25},
                {"name": "Bob", "age": 30}
            ]}),
            1,
        )
        .await;

    assert_eq!(msgs.len(), 1);
    let arr = msgs[0].get("payload").unwrap().as_array().unwrap();
    let names: Vec<&str> = arr.iter().map(|v| v.as_object().unwrap().get("name").unwrap().as_str().unwrap()).collect();
    assert_eq!(names, vec!["Alice", "Bob", "Charlie"]);
}

/// Sort: sort a message sequence (messages with parts) in ascending order.
// Mirrors: Node-RED 18-sort_spec.js "should sort message parts in ascending order"
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sort_sequence_ascending() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "sort", "z": "100", "name": "",
         "order": "ascending", "as_num": true,
         "target": "payload", "target_type": "seq", "key": "payload",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .run_with_inject(
            3,
            vec![
                ("1".to_string(), json!({"payload": 30, "parts": {"id": "g1", "index": 0, "count": 3}})),
                ("1".to_string(), json!({"payload": 10, "parts": {"id": "g1", "index": 1, "count": 3}})),
                ("1".to_string(), json!({"payload": 20, "parts": {"id": "g1", "index": 2, "count": 3}})),
            ],
        )
        .await;

    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0].get("payload").unwrap().as_i64().unwrap(), 10);
    assert_eq!(msgs[1].get("payload").unwrap().as_i64().unwrap(), 20);
    assert_eq!(msgs[2].get("payload").unwrap().as_i64().unwrap(), 30);
}

/// Sort: sort a message sequence in descending order.
// Mirrors: Node-RED 18-sort_spec.js "should sort message parts in descending order"
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sort_sequence_descending() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "sort", "z": "100", "name": "",
         "order": "descending", "as_num": true,
         "target": "payload", "target_type": "seq", "key": "payload",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .run_with_inject(
            3,
            vec![
                ("1".to_string(), json!({"payload": 10, "parts": {"id": "g1", "index": 0, "count": 3}})),
                ("1".to_string(), json!({"payload": 30, "parts": {"id": "g1", "index": 1, "count": 3}})),
                ("1".to_string(), json!({"payload": 20, "parts": {"id": "g1", "index": 2, "count": 3}})),
            ],
        )
        .await;

    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0].get("payload").unwrap().as_i64().unwrap(), 30);
    assert_eq!(msgs[1].get("payload").unwrap().as_i64().unwrap(), 20);
    assert_eq!(msgs[2].get("payload").unwrap().as_i64().unwrap(), 10);
}

/// Sort: sending a reset message clears pending messages without output.
// Mirrors: Node-RED 18-sort_spec.js "should handle reset correctly"
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sort_reset_clears_pending() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "sort", "z": "100", "name": "",
         "order": "ascending", "as_num": true,
         "target": "payload", "target_type": "seq", "key": "payload",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"reset": true}), 1, Duration::from_millis(500)).await;

    assert!(msgs.is_empty(), "Reset should produce no output");
}
