//! Debug node compatibility tests.
//!
//! Verifies that the debug node loads correctly with various configurations
//! and does not interfere with message flow. The debug node is a terminal node
//! (sends to sidebar/console) with no output wires, so we test it by:
//!   1. Including it in the flow and verifying the engine loads without error.
//!   2. Injecting directly into it via `inject_and_collect` with a test-once
//!      sink wired from inject nodes that also exist in the flow.

use std::time::Duration;

use serde_json::json;

use super::flow_builder::FlowBuilder;
use super::harness::{TestHarness, assert_msg_str};

/// Debug node with default payload property loads in a flow without error.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn debug_default_payload() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "hello debug", "vt": "str"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "hello debug", "payloadType": "str",
         "wires": [["99"]]},
        {"id": "2", "z": "100", "type": "debug", "name": "dbg1",
         "active": true, "tosidebar": true, "console": false,
         "tostatus": false, "complete": "payload", "targetType": "msg",
         "wires": []},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "hello debug");
}

/// Debug node with `complete: "true"` (full message object) loads without error.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn debug_complete_message() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "full-msg-test", "vt": "str"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "full-msg-test", "payloadType": "str",
         "wires": [["99"]]},
        {"id": "2", "z": "100", "type": "debug", "name": "dbg-full",
         "active": true, "tosidebar": true, "console": false,
         "tostatus": false, "complete": "true", "targetType": "msg",
         "wires": []},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "full-msg-test");
}

/// Debug node with a specific property selection (e.g. "topic") loads without error.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn debug_property_selection() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [
            {"p": "payload", "v": "test", "vt": "str"},
            {"p": "topic", "v": "my-topic", "vt": "str"}
         ],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "my-topic", "payload": "test", "payloadType": "str",
         "wires": [["99"]]},
        {"id": "2", "z": "100", "type": "debug", "name": "dbg-topic",
         "active": true, "tosidebar": true, "console": false,
         "tostatus": false, "complete": "topic", "targetType": "msg",
         "wires": []},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "test");
    assert_msg_str(&msgs[0], "topic", "my-topic");
}

/// Debug node with console output enabled loads without error.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn debug_console_output() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "console-test", "vt": "str"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "console-test", "payloadType": "str",
         "wires": [["99"]]},
        {"id": "2", "z": "100", "type": "debug", "name": "dbg-console",
         "active": true, "tosidebar": true, "console": true,
         "tostatus": false, "complete": "payload", "targetType": "msg",
         "wires": []},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "console-test");
}

/// Debug node with `active: false` (disabled) loads without error.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn debug_disabled() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "inactive", "vt": "str"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "inactive", "payloadType": "str",
         "wires": [["99"]]},
        {"id": "2", "z": "100", "type": "debug", "name": "dbg-off",
         "active": false, "tosidebar": true, "console": false,
         "tostatus": false, "complete": "payload", "targetType": "msg",
         "wires": []},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "inactive");
}

/// Debug node built via FlowBuilder helper loads correctly.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn debug_via_flow_builder() {
    let flow = FlowBuilder::new()
        .inject_once("1", "builder-test", "str", json!([["99"]]))
        .debug("2")
        .test_sink("99")
        .into_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "builder-test");
}

/// Multiple debug nodes in a single flow all load without error.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn debug_multiple_nodes() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "multi", "vt": "str"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "multi", "payloadType": "str",
         "wires": [["99"]]},
        {"id": "2", "z": "100", "type": "debug", "name": "dbg-a",
         "active": true, "tosidebar": true, "console": false,
         "tostatus": false, "complete": "payload", "targetType": "msg",
         "wires": []},
        {"id": "3", "z": "100", "type": "debug", "name": "dbg-b",
         "active": true, "tosidebar": true, "console": true,
         "tostatus": false, "complete": "true", "targetType": "msg",
         "wires": []},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "multi");
}

/// Inject directly into a debug node and verify the engine handles it
/// by using inject_and_collect with a debug node that has a test-once wired.
/// Since debug has no output wires, we inject into debug and the engine
/// should not crash. We confirm via a second inject -> test-once path.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn debug_receive_message() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "sentinel", "vt": "str"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "sentinel", "payloadType": "str",
         "wires": [["99"]]},
        {"id": "2", "z": "100", "type": "debug", "name": "dbg-recv",
         "active": true, "tosidebar": true, "console": false,
         "tostatus": false, "complete": "payload", "targetType": "msg",
         "wires": []},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    // Inject into the debug node directly (no expected output from debug).
    let msgs = harness
        .inject_and_collect_timeout(
            "2",
            json!({"payload": "direct-inject"}),
            1, // expect 1 msg from the inject-once -> test-once path
            Duration::from_secs(2),
        )
        .await;

    // The debug node consumed our injected message silently.
    // The test-once got the message from the inject-once node.
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "sentinel");
}
