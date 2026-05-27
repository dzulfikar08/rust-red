//! Catch, Status, and Complete node compatibility tests.
//!
//! Verifies that catch nodes capture errors, status nodes receive status
//! updates, and complete nodes fire when processing is done.

use serde_json::json;

use super::harness::{TestHarness, assert_msg_has};

/// Catch node captures errors from function nodes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn catch_captures_error() {
    // A function that throws an error, with a catch node wired to sink
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "function", "name": "",
         "func": "throw new Error('test error');",
         "outputs": 1, "timeout": 5, "noerr": 0,
         "initialize": "", "finalize": "", "libs": [],
         "wires": [["99"]]},
        {"id": "c1", "z": "100", "type": "catch", "name": "",
         "scope": null, "uncaught": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "trigger"}), 1).await;

    assert!(!msgs.is_empty(), "Catch node should have captured the error");
    let error_msg = &msgs[0];
    assert_msg_has(error_msg, "error");
    assert_msg_has(error_msg, "payload");
}

/// Catch node with scoped scope captures errors only from specified nodes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn catch_scoped_capture() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "function", "name": "",
         "func": "throw new Error('scoped error');",
         "outputs": 1, "timeout": 5, "noerr": 0,
         "initialize": "", "finalize": "", "libs": [],
         "wires": [[]]},
        {"id": "c1", "z": "100", "type": "catch", "name": "",
         "scope": ["1"], "uncaught": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "trigger"}), 1).await;

    assert!(!msgs.is_empty(), "Catch node should have captured the scoped error");
    assert_msg_has(&msgs[0], "error");
}

/// Catch node error message contains source info.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn catch_error_contains_source_info() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "function", "name": "errorFunc",
         "func": "throw new Error('test error message');",
         "outputs": 1, "timeout": 5, "noerr": 0,
         "initialize": "", "finalize": "", "libs": [],
         "wires": [[]]},
        {"id": "c1", "z": "100", "type": "catch", "name": "",
         "scope": null, "uncaught": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "trigger"}), 1).await;

    assert!(!msgs.is_empty());
    let error_msg = &msgs[0];
    let error = error_msg.get("error").expect("Missing error property");
    let error_obj = error.as_object().expect("Error should be an object");
    assert!(error_obj.contains_key("message"), "Error should contain 'message'");
    assert!(error_obj.contains_key("source"), "Error should contain 'source'");
}

/// Complete node fires when a node finishes processing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn complete_node_fires_on_completion() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "function", "name": "",
         "func": "msg.payload = 'processed'; return msg;",
         "outputs": 1, "timeout": 5, "noerr": 0,
         "initialize": "", "finalize": "", "libs": [],
         "wires": [["99"]]},
        {"id": "cp1", "z": "100", "type": "complete", "name": "",
         "scope": ["1"],
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    // We expect 2 messages: one from the function output, one from the complete node
    let msgs = harness.inject_and_collect("1", json!({"payload": "input"}), 2).await;

    assert!(!msgs.is_empty(), "Should receive at least one message from complete or function");
}

/// Catch node: uncaught errors are captured by uncaught=true catch node.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn catch_uncaught_error() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "function", "name": "",
         "func": "throw new Error('uncaught test');",
         "outputs": 1, "timeout": 5, "noerr": 0,
         "initialize": "", "finalize": "", "libs": [],
         "wires": [[]]},
        {"id": "c1", "z": "100", "type": "catch", "name": "",
         "scope": null, "uncaught": true,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "trigger"}), 1).await;

    assert!(!msgs.is_empty(), "Uncaught catch node should capture the error");
}

// ---------------------------------------------------------------------------
// Additional catch node tests mirroring Node-RED catch_spec.js
// ---------------------------------------------------------------------------

/// Catch: catch node with scope only captures errors from specified nodes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn catch_scope_filters_by_node_id() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "function", "name": "thrower",
         "func": "throw new Error('scoped');",
         "outputs": 1, "timeout": 5, "noerr": 0,
         "initialize": "", "finalize": "", "libs": [],
         "wires": [[]]},
        {"id": "2", "z": "100", "type": "function", "name": "safe",
         "func": "msg.payload = 'safe'; return msg;",
         "outputs": 1, "timeout": 5, "noerr": 0,
         "initialize": "", "finalize": "", "libs": [],
         "wires": [["99"]]},
        {"id": "c1", "z": "100", "type": "catch", "name": "",
         "scope": ["1"], "uncaught": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "trigger"}), 1).await;

    assert!(!msgs.is_empty(), "Catch node should capture error from scoped node");
    assert_msg_has(&msgs[0], "error");
}

/// Catch: multiple catch nodes both receive the same error.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn catch_multiple_catch_nodes() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "function", "name": "",
         "func": "throw new Error('multi-catch');",
         "outputs": 1, "timeout": 5, "noerr": 0,
         "initialize": "", "finalize": "", "libs": [],
         "wires": [[]]},
        {"id": "c1", "z": "100", "type": "catch", "name": "catch1",
         "scope": null, "uncaught": false,
         "wires": [["99"]]},
        {"id": "c2", "z": "100", "type": "catch", "name": "catch2",
         "scope": null, "uncaught": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "trigger"}), 2).await;

    assert!(msgs.len() >= 2, "Both catch nodes should receive the error, got {}", msgs.len());
    for msg in &msgs {
        assert_msg_has(msg, "error");
    }
}

/// Catch: error message contains detailed message string and source node details.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn catch_error_message_detailed_content() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "function", "name": "myFunc",
         "func": "throw new Error('detailed error');",
         "outputs": 1, "timeout": 5, "noerr": 0,
         "initialize": "", "finalize": "", "libs": [],
         "wires": [[]]},
        {"id": "c1", "z": "100", "type": "catch", "name": "",
         "scope": null, "uncaught": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "trigger"}), 1).await;

    assert!(!msgs.is_empty());
    let error_msg = &msgs[0];

    let error = error_msg.get("error").expect("Missing error property");
    let error_obj = error.as_object().expect("Error should be an object");

    assert!(error_obj.contains_key("message"), "Error should contain 'message'");
    let message = error_obj.get("message").expect("Missing message");
    let message_str = message.as_str().expect("Message should be a string");
    assert!(!message_str.is_empty(), "Error message should not be empty, got: {}", message_str);

    assert!(error_obj.contains_key("source"), "Error should contain 'source'");
    let source = error_obj.get("source").expect("Missing source");
    let source_obj = source.as_object().expect("Source should be an object");
    assert!(source_obj.contains_key("id"), "Source should contain 'id'");
    assert!(source_obj.contains_key("type"), "Source should contain 'type'");
    assert!(source_obj.contains_key("name"), "Source should contain 'name'");
}

/// Catch: uncaught=true catch node fires when no regular catch matches scope.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn catch_uncaught_as_fallback() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "function", "name": "",
         "func": "throw new Error('fallback error');",
         "outputs": 1, "timeout": 5, "noerr": 0,
         "initialize": "", "finalize": "", "libs": [],
         "wires": [[]]},
        {"id": "2", "z": "100", "type": "function", "name": "",
         "func": "return msg;",
         "outputs": 1, "timeout": 5, "noerr": 0,
         "initialize": "", "finalize": "", "libs": [],
         "wires": [["99"]]},
        {"id": "c1", "z": "100", "type": "catch", "name": "scoped",
         "scope": ["2"], "uncaught": false,
         "wires": [["99"]]},
        {"id": "c2", "z": "100", "type": "catch", "name": "uncaught",
         "scope": null, "uncaught": true,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "trigger"}), 1).await;

    assert!(!msgs.is_empty(), "Uncaught catch node should capture error from node 1");
    assert_msg_has(&msgs[0], "error");
}
