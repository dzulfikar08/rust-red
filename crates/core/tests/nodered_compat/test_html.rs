//! HTML node compatibility tests.
//!
//! Verifies that the HTML node extracts elements from HTML documents
//! using CSS selectors, matching Node-RED behavior.

use serde_json::json;

use super::harness::TestHarness;

/// HTML: extract text content from `<p>` elements.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_extract_text() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "tag": "p", "ret": "text", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "<html><body><p>Hello</p></body></html>"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let arr = payload.as_array().expect("Payload should be an array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].as_str().unwrap(), "Hello");
}

/// HTML: extract HTML markup (default ret="html").
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_extract_html_markup() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "tag": "p", "ret": "html", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "<html><body><p>Hello</p></body></html>"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let arr = payload.as_array().expect("Payload should be an array");
    assert_eq!(arr.len(), 1);
    let html_str = arr[0].as_str().unwrap();
    assert!(html_str.contains("Hello"), "HTML output should contain 'Hello', got: {html_str}");
    assert!(html_str.contains("<p>"), "HTML output should contain '<p>', got: {html_str}");
}

/// HTML: extract multiple matching elements returns array.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_extract_multiple() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "tag": "li", "ret": "text", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect("1", json!({"payload": "<ul><li>One</li><li>Two</li><li>Three</li></ul>"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let arr = payload.as_array().expect("Payload should be an array");
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].as_str().unwrap(), "One");
    assert_eq!(arr[1].as_str().unwrap(), "Two");
    assert_eq!(arr[2].as_str().unwrap(), "Three");
}

/// HTML: selector matches nothing returns empty array.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_no_match() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "tag": "h2", "ret": "text", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect("1", json!({"payload": "<html><body><p>No h2 here</p></body></html>"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let arr = payload.as_array().expect("Payload should be an array");
    assert!(arr.is_empty(), "Expected empty array when no elements match");
}

/// HTML: multi-output mode sends separate messages per element with parts.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_multi_output() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "tag": "li", "ret": "text", "as": "multi", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "<ul><li>A</li><li>B</li></ul>"}), 2).await;

    assert_eq!(msgs.len(), 2, "Expected 2 separate messages in multi mode");
    assert_eq!(msgs[0].get("payload").unwrap().as_str().unwrap(), "A");
    assert_eq!(msgs[1].get("payload").unwrap().as_str().unwrap(), "B");
}
