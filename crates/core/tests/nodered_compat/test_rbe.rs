//! RBE (Report by Exception) node compatibility tests.
//!
//! Verifies that the RBE node blocks repeated values, allows changed values,
//! and handles reset messages, matching Node-RED behavior.

use serde_json::json;

use super::harness::{TestHarness, assert_msg_num};

/// RBE: same value sent twice — second should be blocked.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rbe_basic_block_repeat() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "rbe",
         "func": "rbe", "gap": "", "start": "",
         "inout": "out", "property": "payload",
         "septopics": true, "topi": "topic",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    // Send the same value twice — only the first should pass through
    let msgs = harness
        .run_with_inject(1, vec![("1".to_string(), json!({"payload": 42})), ("1".to_string(), json!({"payload": 42}))])
        .await;

    // Only the first message should pass; the second is blocked as a repeat
    assert_eq!(msgs.len(), 1, "Second identical value should be blocked");
    assert_msg_num(&msgs[0], "payload", 42);
}

/// RBE: different values should pass through.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rbe_allow_change() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "rbe",
         "func": "rbe", "gap": "", "start": "",
         "inout": "out", "property": "payload",
         "septopics": true, "topi": "topic",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    // Send different values — both should pass through
    let msgs = harness
        .run_with_inject(2, vec![("1".to_string(), json!({"payload": 10})), ("1".to_string(), json!({"payload": 20}))])
        .await;

    assert_eq!(msgs.len(), 2, "Both different values should pass through");
    assert_msg_num(&msgs[0], "payload", 10);
    assert_msg_num(&msgs[1], "payload", 20);
}

/// RBE: reset message clears previous state, allowing repeat value.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rbe_reset() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "rbe",
         "func": "rbe", "gap": "", "start": "",
         "inout": "out", "property": "payload",
         "septopics": true, "topi": "topic",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    // Send value, then reset, then same value again — all three operations,
    // but only 2 messages should arrive (first value + value after reset)
    let msgs = harness
        .run_with_inject(
            2,
            vec![
                ("1".to_string(), json!({"payload": 42})),
                ("1".to_string(), json!({"reset": true})),
                ("1".to_string(), json!({"payload": 42})),
            ],
        )
        .await;

    assert_eq!(msgs.len(), 2, "After reset, the same value should be allowed through again");
    assert_msg_num(&msgs[0], "payload", 42);
    assert_msg_num(&msgs[1], "payload", 42);
}

/// RBE: rbei mode — ignore the first message (block until a previous value exists).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rbe_ignore_first() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "rbe",
         "func": "rbei", "gap": "", "start": "",
         "inout": "out", "property": "payload",
         "septopics": true, "topi": "topic",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    // In rbei mode, the first message is stored but not forwarded.
    // The second message with a different value should pass through.
    let msgs = harness
        .run_with_inject(1, vec![("1".to_string(), json!({"payload": 10})), ("1".to_string(), json!({"payload": 20}))])
        .await;

    assert_eq!(msgs.len(), 1, "rbei should block first message, allow second if different");
    assert_msg_num(&msgs[0], "payload", 20);
}
