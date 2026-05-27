//! Link node compatibility tests.
//!
//! Verifies that link in, link out, and link call nodes correctly route
//! messages between nodes. Link nodes connect flows across tabs or within
//! the same tab without direct wires.

use std::time::Duration;

use serde_json::json;

use super::harness::{TestHarness, assert_msg_str};

/// link out -> link in on the same tab (same flow).
/// Inject sends to link out, which forwards to link in, which wires to test-once.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn link_in_out_same_flow() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "same-tab", "vt": "str"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "same-tab", "payloadType": "str",
         "wires": [["2"]]},
        {"id": "2", "z": "100", "type": "link out", "name": "link-out",
         "links": ["3"], "mode": "link"},
        {"id": "3", "z": "100", "type": "link in", "name": "link-in",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "same-tab");
}

/// link out on tab A connects to link in on tab B (cross-tab linking).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn link_in_out_cross_tab() {
    let flow = json!([
        {"id": "100", "type": "tab", "label": "Tab A"},
        {"id": "200", "type": "tab", "label": "Tab B"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "cross-tab", "vt": "str"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "cross-tab", "payloadType": "str",
         "wires": [["2"]]},
        {"id": "2", "z": "100", "type": "link out", "name": "link-out-a",
         "links": ["3"], "mode": "link"},
        {"id": "3", "z": "200", "type": "link in", "name": "link-in-b",
         "wires": [["99"]]},
        {"id": "99", "z": "200", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "cross-tab");
}

/// link out fan-out: one link out connects to multiple link in nodes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn link_out_fan_out() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "fanout", "vt": "str"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "fanout", "payloadType": "str",
         "wires": [["2"]]},
        {"id": "2", "z": "100", "type": "link out", "name": "link-out",
         "links": ["3", "4"], "mode": "link"},
        {"id": "3", "z": "100", "type": "link in", "name": "link-in-0",
         "wires": [["99"]]},
        {"id": "4", "z": "100", "type": "link in", "name": "link-in-1",
         "wires": [["98"]]},
        {"id": "99", "z": "100", "type": "test-once"},
        {"id": "98", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(2).await;

    assert_eq!(msgs.len(), 2);
    for msg in &msgs {
        assert_msg_str(msg, "payload", "fanout");
    }
}

/// link call -> link in -> link out (return) -> back to link call output.
/// The message should travel: inject -> link call -> link in -> link out (return)
/// -> link call (output) -> test-once.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn link_call_return() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "call-return", "vt": "str"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "call-return", "payloadType": "str",
         "wires": [["2"]]},
        {"id": "2", "z": "100", "type": "link call", "name": "caller",
         "links": ["3"], "linkType": "static", "timeout": "5",
         "wires": [["99"]]},
        {"id": "3", "z": "100", "type": "link in", "name": "callee",
         "wires": [["4"]]},
        {"id": "4", "z": "100", "type": "link out", "name": "returner",
         "mode": "return", "links": []},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run_with_timeout(1, Duration::from_secs(3)).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "call-return");
}

/// Message passes through link nodes unchanged — no mutation of payload or topic.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn link_passthrough() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [
            {"p": "payload", "v": "unchanged", "vt": "str"},
            {"p": "topic", "v": "link-test", "vt": "str"}
         ],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "link-test", "payload": "unchanged", "payloadType": "str",
         "wires": [["2"]]},
        {"id": "2", "z": "100", "type": "link out", "name": "lo",
         "links": ["3"], "mode": "link"},
        {"id": "3", "z": "100", "type": "link in", "name": "li",
         "wires": [["4"]]},
        {"id": "4", "z": "100", "type": "link out", "name": "lo2",
         "links": ["5"], "mode": "link"},
        {"id": "5", "z": "100", "type": "link in", "name": "li2",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run(1).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "unchanged");
    assert_msg_str(&msgs[0], "topic", "link-test");
}

/// link call with an intermediary function node that modifies the payload,
/// then returns via link out (return). Verifies the modified message arrives
/// at the link call output.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn link_call_with_modification() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "inject", "name": "",
         "props": [{"p": "payload", "v": "original", "vt": "str"}],
         "repeat": "", "crontab": "", "once": true, "onceDelay": 0,
         "topic": "", "payload": "original", "payloadType": "str",
         "wires": [["2"]]},
        {"id": "2", "z": "100", "type": "link call", "name": "caller",
         "links": ["3"], "linkType": "static", "timeout": "5",
         "wires": [["99"]]},
        {"id": "3", "z": "100", "type": "link in", "name": "callee",
         "wires": [["10"]]},
        {"id": "10", "z": "100", "type": "change", "name": "",
         "rules": [{"t": "set", "p": "payload", "pt": "msg", "to": "modified", "tot": "str"}],
         "wires": [["4"]]},
        {"id": "4", "z": "100", "type": "link out", "name": "returner",
         "mode": "return", "links": []},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run_with_timeout(1, Duration::from_secs(3)).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "modified");
}
