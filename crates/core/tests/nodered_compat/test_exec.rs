//! Exec node compatibility tests (Unix only: they run `echo` and `printf`).
//!
//! Verifies the exact output payloads of the exec node, matching Node-RED.

#![cfg(unix)]

use std::time::Duration;

use serde_json::json;

use super::harness::{TestHarness, assert_msg_str};

fn exec_flow(command: &str, addpay: &str, append: &str, use_spawn: bool) -> serde_json::Value {
    json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "exec", "z": "100", "command": command, "addpay": addpay,
         "append": append, "useSpawn": use_spawn.to_string(), "oldrc": "false",
         "wires": [["99"], [], []]},
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

/// Exec mode: stdout is forwarded as the command wrote it, trailing newline included.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn exec_keeps_trailing_newline() {
    let harness = TestHarness::from_flow_json(exec_flow("echo", "payload", "", false));
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "hi"}), 1, Duration::from_secs(5)).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "hi\n");
}

/// Spawn mode: stdout is forwarded as raw chunks, not split into lines with the newline stripped.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn exec_spawn_forwards_raw_chunks() {
    let harness = TestHarness::from_flow_json(exec_flow("printf", "", "a\\nb\\n", true));
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": ""}), 1, Duration::from_secs(5)).await;

    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "a\nb\n");
}

/// Output that is not valid UTF-8 is delivered as a buffer, not lossily converted to text.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn exec_binary_output_is_a_buffer() {
    let harness = TestHarness::from_flow_json(exec_flow("printf", "", "'\\377'", false));
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": ""}), 1, Duration::from_secs(5)).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("payload");
    assert_eq!(payload.as_bytes().expect("payload must be a buffer"), &[0xffu8][..]);
}
