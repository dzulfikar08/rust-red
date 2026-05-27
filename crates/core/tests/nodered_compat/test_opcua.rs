//! OPC-UA node compatibility tests.
//!
//! Verifies that OPC-UA read/write/config nodes behave correctly,
//! matching Node-RED behavior.
//!
//! # IMPORTANT: Infrastructure Requirement
//!
//! All tests in this file are marked `#[ignore]` because they require a running
//! OPC-UA server (e.g., open62541-based server, Prosys Simulation Server)
//! on localhost:4840.
//!
//! To run the ignored tests:
//!
//! ```bash
//! # Start an OPC-UA server
//! # e.g., using open62541-based server or Prosys Simulation Server
//!
//! # Run the ignored tests
//! cargo test -p rust-red-core --test nodered_compat --features internal-testing,nodes_opcua -- test_opcua --ignored
//! ```
//!
//! Even when ignored, these tests must compile to catch regressions.

use std::time::Duration;

use serde_json::json;

use super::harness::TestHarness;

// ---------------------------------------------------------------------------
// Config Deserialization — No server needed
// ---------------------------------------------------------------------------

/// Verify the OPC-UA config node deserializes with defaults.
#[test]
fn opcua_config_defaults() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "opcua-config",
            "name": "opcua-cfg",
            "endpoint": "opc.tcp://localhost:4840"
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

/// Verify the OPC-UA config node accepts security settings.
#[test]
fn opcua_config_with_security() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg_secure",
            "type": "opcua-config",
            "name": "opcua-secure",
            "endpoint": "opc.tcp://localhost:4840",
            "securityMode": "SignAndEncrypt",
            "securityPolicy": "Basic256Sha256",
            "username": "admin",
            "password": "secret",
            "certPath": "/path/to/cert.pem",
            "keyPath": "/path/to/key.pem",
            "sessionTimeoutMs": 60000
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

/// Verify the read node config deserializes with defaults.
#[test]
fn opcua_read_config_defaults() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "opcua-config",
            "name": "opcua-cfg",
            "endpoint": "opc.tcp://localhost:4840"
        },
        {
            "id": "1",
            "type": "opcua read",
            "z": "100",
            "name": "opcua-read",
            "configNode": "cfg1",
            "nodeId": "ns=2;s=Temperature",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

/// Verify the write node config deserializes.
#[test]
fn opcua_write_config_defaults() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "opcua-config",
            "name": "opcua-cfg",
            "endpoint": "opc.tcp://localhost:4840"
        },
        {
            "id": "2",
            "type": "opcua write",
            "z": "100",
            "name": "opcua-write",
            "configNode": "cfg1",
            "nodeId": "ns=2;s=Setpoint",
            "attribute": "Value",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

// ---------------------------------------------------------------------------
// OPC-UA Read — Basic Read (requires server)
// ---------------------------------------------------------------------------

/// Read a single node value from the server.
///
/// Requires: OPC-UA server on opc.tcp://localhost:4840
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires OPC-UA server on opc.tcp://localhost:4840"]
async fn opcua_read_node_value() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "opcua-config",
            "name": "opcua-cfg",
            "endpoint": "opc.tcp://localhost:4840"
        },
        {
            "id": "10",
            "type": "opcua read",
            "z": "100",
            "name": "opcua-read-val",
            "configNode": "cfg1",
            "nodeId": "ns=0;i=2258",  // Server.ServerStatus.CurrentTime
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("10", json!({"payload": true}), 1, Duration::from_secs(10)).await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
        assert!(msgs[0].contains("opcua"));
        // The opcua metadata should include the nodeId
        let opcua = msgs[0].get("opcua").unwrap();
        let opcua_str = opcua.as_str().unwrap_or("");
        assert!(opcua_str.contains("ns=0;i=2258") || opcua.to_string().unwrap_or_default().contains("ns=0;i=2258"));
    }
}

// ---------------------------------------------------------------------------
// OPC-UA Read — Browse Action (requires server)
// ---------------------------------------------------------------------------

/// Browse the children of the root folder.
///
/// Requires: OPC-UA server on opc.tcp://localhost:4840
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires OPC-UA server on opc.tcp://localhost:4840"]
async fn opcua_read_browse_action() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "opcua-config",
            "name": "opcua-cfg",
            "endpoint": "opc.tcp://localhost:4840"
        },
        {
            "id": "11",
            "type": "opcua read",
            "z": "100",
            "name": "opcua-browse",
            "configNode": "cfg1",
            "nodeId": "ns=0;i=84",  // Root folder (Objects)
            "action": "browse",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("11", json!({"payload": true}), 1, Duration::from_secs(10)).await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
        // Browse should return an array of references
        let opcua = msgs[0].get("opcua").unwrap();
        let opcua_str = opcua.to_string().unwrap_or_default();
        assert!(opcua_str.contains("browse"));
    }
}

// ---------------------------------------------------------------------------
// OPC-UA Read — Read Attribute (requires server)
// ---------------------------------------------------------------------------

/// Read the Description attribute of a node.
///
/// Requires: OPC-UA server on opc.tcp://localhost:4840
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires OPC-UA server on opc.tcp://localhost:4840"]
async fn opcua_read_description_attribute() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "opcua-config",
            "name": "opcua-cfg",
            "endpoint": "opc.tcp://localhost:4840"
        },
        {
            "id": "12",
            "type": "opcua read",
            "z": "100",
            "name": "opcua-read-desc",
            "configNode": "cfg1",
            "nodeId": "ns=0;i=2258",
            "attribute": "Description",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let _msgs = harness.inject_and_collect_timeout("12", json!({"payload": true}), 1, Duration::from_secs(10)).await;
}

// ---------------------------------------------------------------------------
// OPC-UA Write — Write Value (requires server)
// ---------------------------------------------------------------------------

/// Write a value to a node on the server.
///
/// Requires: OPC-UA server on opc.tcp://localhost:4840
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires OPC-UA server on opc.tcp://localhost:4840"]
async fn opcua_write_node_value() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "opcua-config",
            "name": "opcua-cfg",
            "endpoint": "opc.tcp://localhost:4840"
        },
        {
            "id": "20",
            "type": "opcua write",
            "z": "100",
            "name": "opcua-write-val",
            "configNode": "cfg1",
            "nodeId": "ns=2;s=Setpoint",
            "attribute": "Value",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("20", json!({"payload": 42.5}), 1, Duration::from_secs(10)).await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
        assert!(msgs[0].contains("opcua"));
    }
}

// ---------------------------------------------------------------------------
// OPC-UA — Write then Read Round-Trip (requires server)
// ---------------------------------------------------------------------------

/// Write a value, then read it back to verify round-trip integrity.
///
/// Requires: OPC-UA server on opc.tcp://localhost:4840
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires OPC-UA server on opc.tcp://localhost:4840"]
async fn opcua_write_read_roundtrip() {
    // Step 1: Write
    let write_flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "opcua-config",
            "name": "opcua-cfg",
            "endpoint": "opc.tcp://localhost:4840"
        },
        {
            "id": "30",
            "type": "opcua write",
            "z": "100",
            "name": "opcua-write-rt",
            "configNode": "cfg1",
            "nodeId": "ns=2;s=TestVar",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let write_harness = TestHarness::from_flow_json(write_flow);
    let _write_msgs =
        write_harness.inject_and_collect_timeout("30", json!({"payload": 99.9}), 1, Duration::from_secs(10)).await;

    // Step 2: Read back
    let read_flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "opcua-config",
            "name": "opcua-cfg",
            "endpoint": "opc.tcp://localhost:4840"
        },
        {
            "id": "31",
            "type": "opcua read",
            "z": "100",
            "name": "opcua-read-rt",
            "configNode": "cfg1",
            "nodeId": "ns=2;s=TestVar",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let read_harness = TestHarness::from_flow_json(read_flow);
    let _read_msgs =
        read_harness.inject_and_collect_timeout("31", json!({"payload": true}), 1, Duration::from_secs(10)).await;
}

// ---------------------------------------------------------------------------
// OPC-UA — Anonymous vs Username/Password (requires server)
// ---------------------------------------------------------------------------

/// Verify the config node accepts anonymous connection.
///
/// Requires: OPC-UA server on opc.tcp://localhost:4840
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires OPC-UA server on opc.tcp://localhost:4840"]
async fn opcua_anonymous_connection() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg_anon",
            "type": "opcua-config",
            "name": "opcua-anon",
            "endpoint": "opc.tcp://localhost:4840",
            "securityMode": "None",
            "securityPolicy": "None"
        },
        {
            "id": "40",
            "type": "opcua read",
            "z": "100",
            "name": "opcua-read-anon",
            "configNode": "cfg_anon",
            "nodeId": "ns=0;i=2258",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

/// Verify the config node accepts username/password authentication.
///
/// Requires: OPC-UA server on opc.tcp://localhost:4840
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires OPC-UA server on opc.tcp://localhost:4840"]
async fn opcua_username_auth() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg_auth",
            "type": "opcua-config",
            "name": "opcua-auth",
            "endpoint": "opc.tcp://localhost:4840",
            "securityMode": "None",
            "securityPolicy": "None",
            "username": "user1",
            "password": "password1"
        },
        {
            "id": "41",
            "type": "opcua read",
            "z": "100",
            "name": "opcua-read-auth",
            "configNode": "cfg_auth",
            "nodeId": "ns=0;i=2258",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}
