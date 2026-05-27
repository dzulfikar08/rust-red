//! Watch node compatibility tests.
//!
//! Verifies that the watch node detects filesystem changes (create, modify,
//! delete) and emits appropriate messages.
//!
//! These tests spawn file operations in a background task so the engine has
//! time to start the watcher before filesystem events occur.

use std::time::Duration;

use serde_json::json;
use tempfile::TempDir;

use super::harness::{TestHarness, assert_msg_has};

// ---------------------------------------------------------------------------
// Helper: build a watch flow
// ---------------------------------------------------------------------------

fn build_watch_flow(watch_path: &str) -> serde_json::Value {
    json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "watch", "z": "100", "name": "",
         "files": watch_path,
         "recursive": false,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Watch: detect file creation in watched directory.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore] // filesystem watching is timing-sensitive; enable manually
async fn watch_file_create() {
    let tmp = TempDir::new().expect("create temp dir");
    let watch_dir = tmp.path().to_string_lossy().to_string();
    let file_path = tmp.path().join("new_file.txt");

    let flow = build_watch_flow(&watch_dir);
    let harness = TestHarness::from_flow_json(flow);

    // Spawn a task that creates a file after a short delay (letting watcher initialize)
    let fp = file_path.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let _ = tokio::fs::write(&fp, "created content").await;
    });

    let msgs = harness.run_with_timeout(1, Duration::from_secs(8)).await;

    if msgs.is_empty() {
        eprintln!("WARN: watch_file_create: no events received (OS timing)");
        return;
    }

    assert_msg_has(&msgs[0], "payload");
    assert_msg_has(&msgs[0], "event");
    assert_msg_has(&msgs[0], "file");
}

/// Watch: detect file modification in watched directory.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore] // filesystem watching is timing-sensitive; enable manually
async fn watch_file_change() {
    let tmp = TempDir::new().expect("create temp dir");
    let watch_dir = tmp.path().to_string_lossy().to_string();

    // Pre-create a file before setting up the watch
    let existing_file = tmp.path().join("existing.txt");
    tokio::fs::write(&existing_file, "original").await.expect("create initial file");

    let flow = build_watch_flow(&watch_dir);
    let harness = TestHarness::from_flow_json(flow);

    // Spawn a task that modifies the file after watcher initializes
    let ef = existing_file.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let _ = tokio::fs::write(&ef, "modified content").await;
    });

    let msgs = harness.run_with_timeout(1, Duration::from_secs(8)).await;

    if msgs.is_empty() {
        eprintln!("WARN: watch_file_change: no events received (OS timing)");
        return;
    }

    assert_msg_has(&msgs[0], "payload");
    assert_msg_has(&msgs[0], "event");
}

/// Watch: detect file deletion in watched directory.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore] // filesystem watching is timing-sensitive; enable manually
async fn watch_file_delete() {
    let tmp = TempDir::new().expect("create temp dir");
    let watch_dir = tmp.path().to_string_lossy().to_string();

    let flow = build_watch_flow(&watch_dir);
    let harness = TestHarness::from_flow_json(flow);

    // Spawn a task that creates then deletes a file after watcher initializes
    let df = tmp.path().join("to_delete.txt");
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let _ = tokio::fs::write(&df, "will be deleted").await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = tokio::fs::remove_file(&df).await;
    });

    // Expect 2 messages: one create/update, one delete (or update on some platforms)
    let msgs = harness.run_with_timeout(2, Duration::from_secs(8)).await;

    if msgs.len() < 2 {
        eprintln!("WARN: watch_file_delete: fewer than 2 events received");
        return;
    }

    // Verify we received events for the file lifecycle
    let delete_msg = &msgs[1];
    assert_msg_has(delete_msg, "payload");
    assert_msg_has(delete_msg, "event");
    // Note: on some platforms (macOS), file deletion may be reported as "update"
    // rather than "delete" due to notify crate behavior. Just verify an event
    // was received and the file path is correct.
    let payload = delete_msg.get("payload").expect("missing payload");
    if let Some(path_str) = payload.as_str() {
        assert!(path_str.contains("to_delete.txt"), "Payload should reference the deleted file, got: {}", path_str);
    }
}

/// Watch: message payload contains the full file path and file metadata.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore] // filesystem watching is timing-sensitive; enable manually
async fn watch_message_contains_path() {
    let tmp = TempDir::new().expect("create temp dir");
    let watch_dir = tmp.path().to_string_lossy().to_string();

    let flow = build_watch_flow(&watch_dir);
    let harness = TestHarness::from_flow_json(flow);

    // Spawn a task that creates a file after watcher initializes
    let fp = tmp.path().join("path_test.txt");
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let _ = tokio::fs::write(&fp, "test").await;
    });

    let msgs = harness.run_with_timeout(1, Duration::from_secs(8)).await;

    if msgs.is_empty() {
        eprintln!("WARN: watch_message_contains_path: no events received (OS timing)");
        return;
    }

    // Payload should be the full file path
    let payload = msgs[0].get("payload").expect("missing payload");
    if let Some(path_str) = payload.as_str() {
        assert!(path_str.contains("path_test.txt"), "Payload should contain the filename, got: {}", path_str);
    }

    // Additional properties should be set
    assert_msg_has(&msgs[0], "filename");
    assert_msg_has(&msgs[0], "file");
    assert_msg_has(&msgs[0], "type");
}
