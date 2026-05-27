//! Comment node compatibility tests.
//!
//! Verifies that the comment node loads correctly and does not interfere
//! with message flow. The comment node is a no-op terminal node in Node-RED.

use serde_json::json;

use super::flow_builder::FlowBuilder;
use super::harness::{TestHarness, assert_msg_str};

/// Comment node loads in a flow without errors.
/// It does not participate in message passing, so it has no wires.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn comment_loads() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "comment", "name": "A comment",
         "info": "This is a test comment"},
        {"id": "2", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "hello", "vt": "str"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "hello", "payloadType": "str",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "hello");
}

/// Multiple comment nodes in a flow do not interfere with message routing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn comment_multiple() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "z": "100", "type": "comment", "name": "Note 1",
         "info": "First comment"},
        {"id": "c2", "z": "100", "type": "comment", "name": "Note 2",
         "info": "Second comment"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "multi-comment", "vt": "str"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "multi-comment", "payloadType": "str",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "multi-comment");
}

/// Comment node with no name or info loads fine.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn comment_minimal() {
    let flow = FlowBuilder::new()
        .raw_node(json!({"id": "c1", "z": "100", "type": "comment"}))
        .inject_once("1", "minimal", "str", json!([["99"]]))
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "minimal");
}
