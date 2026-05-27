//! File and file-in node compatibility tests.
//!
//! Verifies that the file (write/append/delete) and file-in (read) nodes
//! behave identically to Node-RED's file node.

use std::time::Duration;

use serde_json::json;
use tempfile::TempDir;

use super::harness::{TestHarness, assert_msg_has, assert_msg_str};

// ---------------------------------------------------------------------------
// Helper: build a file write flow (inject -> file -> test-once sink)
// ---------------------------------------------------------------------------

/// Build a flow with a file node (write/append/delete) wired to a test-once sink.
/// The file node is node "1", sink is "99".
fn build_file_write_flow(filename: &str, overwrite: &str, create_dir: bool) -> serde_json::Value {
    json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "file", "z": "100", "name": "",
         "filename": filename,
         "filenameType": "str",
         "appendNewline": false,
         "overwriteFile": overwrite,
         "createDir": create_dir,
         "encoding": "none",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

/// Build a flow with a file-in node (read) wired to a test-once sink.
fn build_file_read_flow(filename: &str, format: &str) -> serde_json::Value {
    json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "file in", "z": "100", "name": "",
         "filename": filename,
         "filenameType": "str",
         "format": format,
         "encoding": "none",
         "allProps": false,
         "sendError": true,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

/// Build a flow where file node reads filename from msg.filename.
fn build_file_write_msg_filename_flow() -> serde_json::Value {
    json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "file", "z": "100", "name": "",
         "filename": "filename",
         "filenameType": "msg",
         "appendNewline": false,
         "overwriteFile": "true",
         "createDir": false,
         "encoding": "none",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

/// Build a flow with file-in reading from msg.filename.
fn build_file_read_msg_filename_flow() -> serde_json::Value {
    json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "file in", "z": "100", "name": "",
         "filename": "filename",
         "filenameType": "msg",
         "format": "utf8",
         "encoding": "none",
         "allProps": false,
         "sendError": true,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// File write: write payload string to a new file.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_write_basic() {
    let tmp = TempDir::new().expect("create temp dir");
    let file_path = tmp.path().join("output.txt");
    let file_path_str = file_path.to_string_lossy().to_string();

    let flow = build_file_write_flow(&file_path_str, "true", false);
    let harness = TestHarness::from_flow_json(flow);

    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "hello world"}), 1, Duration::from_secs(2)).await;

    assert!(!msgs.is_empty(), "File node should forward the message");
    assert_msg_str(&msgs[0], "payload", "hello world");

    // Verify file contents on disk
    let contents = tokio::fs::read_to_string(&file_path).await.expect("read file");
    assert_eq!(contents, "hello world");
}

/// File read: read file contents into msg.payload.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_read_basic() {
    let tmp = TempDir::new().expect("create temp dir");
    let file_path = tmp.path().join("input.txt");
    let file_path_str = file_path.to_string_lossy().to_string();

    // Pre-create the file with known content
    tokio::fs::write(&file_path, "test content here").await.expect("write test file");

    let flow = build_file_read_flow(&file_path_str, "utf8");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "trigger"}), 1, Duration::from_secs(2)).await;

    assert!(!msgs.is_empty(), "File-in node should output a message");
    assert_msg_str(&msgs[0], "payload", "test content here");
    assert_msg_str(&msgs[0], "filename", &file_path_str);
}

/// File append: append to an existing file.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_append() {
    let tmp = TempDir::new().expect("create temp dir");
    let file_path = tmp.path().join("append.txt");
    let file_path_str = file_path.to_string_lossy().to_string();

    // Pre-create with initial content
    tokio::fs::write(&file_path, "first").await.expect("write initial content");

    let flow = build_file_write_flow(&file_path_str, "false", false);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "second"}), 1, Duration::from_secs(2)).await;

    assert!(!msgs.is_empty(), "File node should forward appended message");

    let contents = tokio::fs::read_to_string(&file_path).await.expect("read file");
    assert_eq!(contents, "firstsecond", "File should contain appended content");
}

/// File delete: delete a file using overwriteFile="delete".
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_delete() {
    let tmp = TempDir::new().expect("create temp dir");
    let file_path = tmp.path().join("to_delete.txt");
    let file_path_str = file_path.to_string_lossy().to_string();

    // Pre-create file
    tokio::fs::write(&file_path, "delete me").await.expect("write file to delete");
    assert!(file_path.exists(), "File should exist before delete");

    let flow = build_file_write_flow(&file_path_str, "delete", false);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "anything"}), 1, Duration::from_secs(2)).await;

    assert!(!msgs.is_empty(), "File delete node should forward message");
    assert!(!file_path.exists(), "File should be deleted after node processes");
}

/// File create directory: write to a path that requires creating parent dirs.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_create_directory() {
    let tmp = TempDir::new().expect("create temp dir");
    let nested_path = tmp.path().join("subdir").join("nested").join("output.txt");
    let file_path_str = nested_path.to_string_lossy().to_string();

    let flow = build_file_write_flow(&file_path_str, "true", true);
    let harness = TestHarness::from_flow_json(flow);

    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "nested content"}), 1, Duration::from_secs(2)).await;

    assert!(!msgs.is_empty(), "File node should forward message");
    assert!(nested_path.exists(), "File should exist at nested path");

    let contents = tokio::fs::read_to_string(&nested_path).await.expect("read file");
    assert_eq!(contents, "nested content");
}

/// File binary mode: write string payload and read back as buffer (bytes).
/// JSON arrays are deserialized as Variant::Array (not Variant::Bytes), so
/// binary roundtrip requires reading back via file-in buffer format.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_binary_mode() {
    let tmp = TempDir::new().expect("create temp dir");
    let file_path = tmp.path().join("binary.bin");
    let file_path_str = file_path.to_string_lossy().to_string();

    // Write a known string payload
    let write_flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "file", "z": "100", "name": "",
         "filename": file_path_str,
         "filenameType": "str",
         "appendNewline": false,
         "overwriteFile": "true",
         "createDir": false,
         "encoding": "none",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let write_harness = TestHarness::from_flow_json(write_flow);

    // Use non-ASCII UTF-8 characters as binary-like content
    let binary_content = "binary\u{00e2}\u{009c}\u{0082}data\u{00c3}\u{00bc}";
    let msgs = write_harness
        .inject_and_collect_timeout("1", json!({"payload": binary_content}), 1, Duration::from_secs(2))
        .await;

    assert!(!msgs.is_empty(), "File node should forward message");

    // Verify file contains expected bytes
    let disk_contents = tokio::fs::read(&file_path).await.expect("read binary file");
    assert_eq!(disk_contents, binary_content.as_bytes());

    // Now read back using file-in with buffer format (empty string = buffer mode)
    let read_flow = json!([
        {"id": "200", "type": "tab"},
        {"id": "10", "type": "file in", "z": "200", "name": "",
         "filename": file_path_str,
         "filenameType": "str",
         "format": "",
         "encoding": "none",
         "allProps": false,
         "sendError": true,
         "wires": [["98"]]},
        {"id": "98", "z": "200", "type": "test-once"}
    ]);
    let read_harness = TestHarness::from_flow_json(read_flow);
    let read_msgs =
        read_harness.inject_and_collect_timeout("10", json!({"payload": "read"}), 1, Duration::from_secs(2)).await;

    assert!(!read_msgs.is_empty(), "File-in should output binary data");
    let payload = read_msgs[0].get("payload").expect("missing payload");
    match payload {
        rust_red_core::runtime::model::Variant::Bytes(bytes) => {
            assert_eq!(bytes, &binary_content.as_bytes().to_vec());
        }
        other => panic!("Expected Bytes variant, got: {:?}", other),
    }
}

/// File read nonexistent: reading a nonexistent file should produce an error message.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_read_nonexistent() {
    let tmp = TempDir::new().expect("create temp dir");
    let file_path = tmp.path().join("does_not_exist.txt");
    let file_path_str = file_path.to_string_lossy().to_string();

    let flow = build_file_read_flow(&file_path_str, "utf8");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "trigger"}), 1, Duration::from_secs(2)).await;

    assert!(!msgs.is_empty(), "File-in should emit error message for nonexistent file");
    // Node-RED: on error, msg.error is set with the error description
    assert_msg_has(&msgs[0], "error");
}

/// File write with filename from msg.filename: use msg.filename to set the target path.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_write_with_filename_from_msg() {
    let tmp = TempDir::new().expect("create temp dir");
    let file_path = tmp.path().join("dynamic_name.txt");
    let file_path_str = file_path.to_string_lossy().to_string();

    let flow = build_file_write_msg_filename_flow();
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness
        .inject_and_collect_timeout(
            "1",
            json!({
                "payload": "dynamic content",
                "filename": file_path_str
            }),
            1,
            Duration::from_secs(2),
        )
        .await;

    assert!(!msgs.is_empty(), "File node should forward message");
    assert!(file_path.exists(), "File should be created at dynamic path");

    let contents = tokio::fs::read_to_string(&file_path).await.expect("read file");
    assert_eq!(contents, "dynamic content");
}

// ---------------------------------------------------------------------------
// Additional file node tests
// ---------------------------------------------------------------------------

/// File read with buffer format: read file contents as raw bytes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_read_buffer_format() {
    let tmp = TempDir::new().expect("create temp dir");
    let file_path = tmp.path().join("buffer.txt");
    let file_path_str = file_path.to_string_lossy().to_string();

    tokio::fs::write(&file_path, "buffer content").await.expect("write test file");

    let flow = build_file_read_flow(&file_path_str, "");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "trigger"}), 1, Duration::from_secs(2)).await;

    assert!(!msgs.is_empty(), "File-in buffer mode should output a message");
    // Buffer mode returns Variant::Bytes
    let payload = msgs[0].get("payload").expect("missing payload");
    match payload {
        rust_red_core::runtime::model::Variant::Bytes(bytes) => {
            assert_eq!(bytes, b"buffer content");
        }
        other => panic!("Expected Bytes variant, got: {:?}", other),
    }
}

/// File write with appendNewline=true: for single messages without parts,
/// the newline is NOT appended (it is only appended for intermediate parts
/// of multipart messages). This matches Node-RED behavior.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_write_append_newline_single_msg() {
    let tmp = TempDir::new().expect("create temp dir");
    let file_path = tmp.path().join("newline.txt");
    let file_path_str = file_path.to_string_lossy().to_string();

    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "file", "z": "100", "name": "",
         "filename": file_path_str,
         "filenameType": "str",
         "appendNewline": true,
         "overwriteFile": "true",
         "createDir": false,
         "encoding": "none",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "line"}), 1, Duration::from_secs(2)).await;

    assert!(!msgs.is_empty());

    let contents = tokio::fs::read_to_string(&file_path).await.expect("read file");
    // Single message without parts: appendNewline is true but no newline is
    // added because the message is treated as the last part.
    assert_eq!(contents, "line");
}

/// File write with appendNewline=true: for multipart messages, newline is
/// appended to intermediate parts but NOT to the last part.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_write_append_newline_multipart() {
    let tmp = TempDir::new().expect("create temp dir");
    let file_path = tmp.path().join("multipart.txt");
    let file_path_str = file_path.to_string_lossy().to_string();

    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "file", "z": "100", "name": "",
         "filename": file_path_str,
         "filenameType": "str",
         "appendNewline": true,
         "overwriteFile": "false",
         "createDir": false,
         "encoding": "none",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    // Send first part (index 0, count 3) — NOT the last part, so newline is appended
    let msgs1 = harness
        .inject_and_collect_timeout(
            "1",
            json!({
                "payload": "line1",
                "parts": {"index": 0, "count": 3}
            }),
            1,
            Duration::from_secs(2),
        )
        .await;
    assert!(!msgs1.is_empty());

    let contents = tokio::fs::read_to_string(&file_path).await.expect("read file");
    assert!(contents.ends_with('\n'), "Intermediate part should end with newline, got: {:?}", contents);
}

/// File-in read with msg.filename: read file path from message property.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_read_with_msg_filename() {
    let tmp = TempDir::new().expect("create temp dir");
    let file_path = tmp.path().join("msg_read.txt");
    let file_path_str = file_path.to_string_lossy().to_string();

    tokio::fs::write(&file_path, "from msg filename").await.expect("write test file");

    let flow = build_file_read_msg_filename_flow();
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness
        .inject_and_collect_timeout(
            "1",
            json!({"payload": "trigger", "filename": file_path_str}),
            1,
            Duration::from_secs(2),
        )
        .await;

    assert!(!msgs.is_empty(), "File-in should read using msg.filename");
    assert_msg_str(&msgs[0], "payload", "from msg filename");
}

/// File write with numeric payload: numbers are converted to string.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_write_numeric_payload() {
    let tmp = TempDir::new().expect("create temp dir");
    let file_path = tmp.path().join("numeric.txt");
    let file_path_str = file_path.to_string_lossy().to_string();

    let flow = build_file_write_flow(&file_path_str, "true", false);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": 42}), 1, Duration::from_secs(2)).await;

    assert!(!msgs.is_empty());

    let contents = tokio::fs::read_to_string(&file_path).await.expect("read file");
    assert_eq!(contents, "42");
}

/// File write then file read roundtrip: write then read back the same content.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_write_read_roundtrip() {
    let tmp = TempDir::new().expect("create temp dir");
    let file_path = tmp.path().join("roundtrip.txt");
    let file_path_str = file_path.to_string_lossy().to_string();

    // Step 1: Write the file
    let write_flow = build_file_write_flow(&file_path_str, "true", false);
    let write_harness = TestHarness::from_flow_json(write_flow);
    let write_msgs = write_harness
        .inject_and_collect_timeout("1", json!({"payload": "roundtrip data"}), 1, Duration::from_secs(2))
        .await;
    assert!(!write_msgs.is_empty());

    // Step 2: Read it back
    let read_flow = build_file_read_flow(&file_path_str, "utf8");
    let read_harness = TestHarness::from_flow_json(read_flow);
    let read_msgs =
        read_harness.inject_and_collect_timeout("1", json!({"payload": "read"}), 1, Duration::from_secs(2)).await;

    assert!(!read_msgs.is_empty());
    assert_msg_str(&read_msgs[0], "payload", "roundtrip data");
}
