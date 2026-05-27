//! Split and Join node compatibility tests.
//!
//! Verifies that the split node breaks messages into parts and the join
//! node reassembles them, matching Node-RED behavior.

use std::time::Duration;

use serde_json::json;

use super::flow_builder::FlowBuilder;
use super::harness::{TestHarness, assert_msg_has};
use rust_red_core::runtime::model::Variant;

/// Split: split an array into individual messages.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn split_array_into_messages() {
    let flow = FlowBuilder::new().split("1", json!([["99"]])).test_sink("99").to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": ["a", "b", "c"]}), 3, Duration::from_secs(2)).await;

    assert_eq!(msgs.len(), 3, "Should produce 3 messages from array of 3");
    assert_eq!(msgs[0]["payload"], "a".into());
    assert_eq!(msgs[1]["payload"], "b".into());
    assert_eq!(msgs[2]["payload"], "c".into());
}

/// Split: each output message has a parts property with index.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn split_array_has_parts() {
    let flow = FlowBuilder::new().split("1", json!([["99"]])).test_sink("99").to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": [10, 20]}), 2, Duration::from_secs(2)).await;

    assert_eq!(msgs.len(), 2);

    for (i, msg) in msgs.iter().enumerate() {
        assert_msg_has(msg, "parts");
        let parts = msg.get("parts").expect("Missing parts");
        let parts_obj = parts.as_object().expect("parts should be an object");
        assert!(parts_obj.contains_key("index"), "parts[{}] should contain 'index'", i);
        assert!(parts_obj.contains_key("id"), "parts[{}] should contain 'id'", i);
    }
}

/// Split: split a string by newlines.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn split_string_by_newlines() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "split", "z": "100", "name": "",
         "splt": "\\n", "spltType": "str",
         "arraySplt": 1, "arraySpltType": "len",
         "stream": false, "addname": "", "property": "payload",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .inject_and_collect_timeout("1", json!({"payload": "line1\nline2\nline3"}), 3, Duration::from_secs(2))
        .await;

    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0]["payload"], "line1".into());
    assert_eq!(msgs[1]["payload"], "line2".into());
    assert_eq!(msgs[2]["payload"], "line3".into());
}

/// Split: split a string by comma.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn split_string_by_delimiter() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "split", "z": "100", "name": "",
         "splt": ",", "spltType": "str",
         "arraySplt": 1, "arraySpltType": "len",
         "stream": false, "addname": "", "property": "payload",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "a,b,c,d"}), 4, Duration::from_secs(2)).await;

    assert_eq!(msgs.len(), 4);
    assert_eq!(msgs[0]["payload"], "a".into());
    assert_eq!(msgs[1]["payload"], "b".into());
    assert_eq!(msgs[2]["payload"], "c".into());
    assert_eq!(msgs[3]["payload"], "d".into());
}

/// Join: join messages back into an array (auto mode).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn join_auto_array() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "split", "z": "100", "name": "",
         "splt": "\\n", "spltType": "str",
         "arraySplt": 1, "arraySpltType": "len",
         "stream": false, "addname": "", "property": "payload",
         "wires": [["2"]]},
        {"id": "2", "type": "join", "z": "100", "name": "",
         "mode": "auto", "build": "array",
         "property": "payload", "propertyType": "msg",
         "joinChar": "\\n", "accumulate": false,
         "timeout": "", "count": "",
         "reduce": false, "reduceExp": "", "reduceInit": "",
         "reduceFixup": "", "reduceInitType": "", "reduceExpType": "",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": ["x", "y", "z"]}), 1, Duration::from_secs(2)).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    assert!(payload.is_array(), "Joined payload should be an array");
    let arr = payload.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0], Variant::from("x"));
    assert_eq!(arr[1], Variant::from("y"));
    assert_eq!(arr[2], Variant::from("z"));
}

/// Join: manual join with specified count.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn join_manual_count() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "join", "z": "100", "name": "",
         "mode": "custom", "build": "array",
         "property": "payload", "propertyType": "msg",
         "joinChar": "\\n", "accumulate": false,
         "timeout": "", "count": "3",
         "reduce": false, "reduceExp": "", "reduceInit": "",
         "reduceFixup": "", "reduceInitType": "", "reduceExpType": "",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .run_with_inject(
            1,
            vec![
                ("1".to_string(), json!({"payload": "first"})),
                ("1".to_string(), json!({"payload": "second"})),
                ("1".to_string(), json!({"payload": "third"})),
            ],
        )
        .await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    assert!(payload.is_array(), "Joined payload should be an array");
    let arr = payload.as_array().unwrap();
    assert_eq!(arr.len(), 3);
}

/// Join: join into a string with separator.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn join_into_string() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "join", "z": "100", "name": "",
         "mode": "custom", "build": "string",
         "property": "payload", "propertyType": "msg",
         "joinChar": ",", "accumulate": false,
         "timeout": "", "count": "3",
         "reduce": false, "reduceExp": "", "reduceInit": "",
         "reduceFixup": "", "reduceInitType": "", "reduceExpType": "",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .run_with_inject(
            1,
            vec![
                ("1".to_string(), json!({"payload": "a"})),
                ("1".to_string(), json!({"payload": "b"})),
                ("1".to_string(), json!({"payload": "c"})),
            ],
        )
        .await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    assert!(payload.is_string(), "Joined payload should be a string");
    let s = payload.as_str().unwrap();
    assert_eq!(s, "a,b,c");
}

/// Split: single element array produces one message.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn split_single_element_array() {
    let flow = FlowBuilder::new().split("1", json!([["99"]])).test_sink("99").to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": ["only"]}), 1, Duration::from_secs(2)).await;

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["payload"], "only".into());
}

/// Split: empty array produces no messages.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn split_empty_array() {
    let flow = FlowBuilder::new().split("1", json!([["99"]])).test_sink("99").to_json();

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": []}), 1, Duration::from_millis(300)).await;

    assert!(msgs.is_empty(), "Empty array should produce no messages");
}
