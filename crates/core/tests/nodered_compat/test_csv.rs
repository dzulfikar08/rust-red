//! CSV node compatibility tests.
//!
//! Verifies that the CSV node parses and stringifies CSV correctly,
//! matching Node-RED behavior.

use std::time::Duration;

use serde_json::json;

use super::harness::TestHarness;

/// CSV: parse CSV with headers into array of objects.
///
/// Input: `"a,b\n1,2"` with `hdrin: true`
/// Output: payload is an array with one object `{a: 1, b: 2}` (numbers because
/// `strings` defaults to true, which parses numeric strings).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn csv_parse_with_headers() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "csv", "hdrin": true, "multi": "mult", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "a,b\n1,2"}), 1).await;

    assert_eq!(msgs.len(), 1, "Expected 1 output message");
    let payload = msgs[0].get("payload").expect("Missing payload");
    let arr = payload.as_array().expect("Payload should be an array");
    assert_eq!(arr.len(), 1, "Expected 1 row");

    let row = arr[0].as_object().expect("Row should be an object");
    // strings defaults to true, so "1" and "2" are parsed as numbers
    assert_eq!(row.get("a").unwrap().as_number().unwrap().as_i64().unwrap(), 1);
    assert_eq!(row.get("b").unwrap().as_number().unwrap().as_i64().unwrap(), 2);
}

/// CSV: parse CSV without headers into array (auto-generated col1, col2).
///
/// Without `hdrin`, columns are auto-named col1, col2, ...
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn csv_parse_without_headers() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "csv", "multi": "mult", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "hello,world\nfoo,bar"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let arr = payload.as_array().expect("Payload should be an array");
    assert_eq!(arr.len(), 2);

    let row0 = arr[0].as_object().expect("Row 0 should be an object");
    assert_eq!(row0.get("col1").unwrap().as_str().unwrap(), "hello");
    assert_eq!(row0.get("col2").unwrap().as_str().unwrap(), "world");
}

/// CSV: convert array of objects back to CSV string.
///
/// Input: payload is `[{a: "1", b: "2"}]` with `hdrout: "all"`
/// Output: CSV string with header line and data line.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn csv_stringify() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "csv", "hdrout": "all", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": [{"a": "1", "b": "2"}]}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let csv_str = payload.as_str().expect("Payload should be a string");

    // Should contain header line and data line
    assert!(csv_str.contains("a,b"), "CSV output should contain header 'a,b', got: {csv_str}");
    assert!(csv_str.contains("1,2"), "CSV output should contain data '1,2', got: {csv_str}");
}

/// CSV: multi-part message (parts/sequence) handling.
///
/// When input msg has `parts` with index < count-1, the CSV node accumulates
/// and does not emit output. When the last part arrives, it emits the combined result.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn csv_multi_part() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "csv", "multi": "mult", "hdrin": true, "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    // Inject a first-part message (index 0 of 2, meaning more parts coming)
    let msgs = harness
        .inject_and_collect_timeout(
            "1",
            json!({
                "payload": "a,b\n1,2",
                "parts": {"id": "p1", "index": 0, "count": 2}
            }),
            1,
            Duration::from_secs(2),
        )
        .await;

    // First part should produce no output (accumulating)
    assert!(msgs.is_empty(), "First part should produce no output, got {} msgs", msgs.len());
}

/// CSV: empty string input produces empty output.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn csv_empty_input() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "csv", "multi": "mult", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": ""}), 1, Duration::from_secs(2)).await;

    // Empty CSV should produce an empty array payload (or no output)
    if !msgs.is_empty() {
        let payload = msgs[0].get("payload").expect("Missing payload");
        let arr = payload.as_array();
        assert!(arr.is_none() || arr.unwrap().is_empty(), "Expected empty array or no output for empty CSV");
    }
}

/// CSV: parse with numbers enabled (strings: true).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn csv_parse_numbers() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "csv", "hdrin": true, "multi": "mult", "strings": true, "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "x,y\n42,3.14"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let arr = payload.as_array().expect("Payload should be an array");
    let row = arr[0].as_object().expect("Row should be an object");

    // x should be parsed as integer 42
    let x_val = row.get("x").unwrap();
    assert!(x_val.is_number(), "x should be a number");
    assert_eq!(x_val.as_number().unwrap().as_i64().unwrap(), 42);
}
