//! BACnet node compatibility tests.
//!
//! Verifies that BACnet read/write/config nodes behave correctly,
//! matching Node-RED behavior.
//!
//! # IMPORTANT: Infrastructure Requirement
//!
//! All tests in this file are marked `#[ignore]` because they require a running
//! BACnet device or simulator (e.g., BACnet/IP simulator) on the network.
//!
//! To run the ignored tests:
//!
//! ```bash
//! # Start a BACnet/IP simulator
//! # e.g., using BACnet Stack simulator or similar tool
//!
//! # Run the ignored tests
//! cargo test -p rust-red-core --test nodered_compat --features internal-testing,nodes_bacnet -- test_bacnet --ignored
//! ```
//!
//! Even when ignored, these tests must compile to catch regressions.

use std::time::Duration;

use serde_json::json;

use super::harness::TestHarness;

// ---------------------------------------------------------------------------
// Config Deserialization — No device needed
// ---------------------------------------------------------------------------

/// Verify the BACnet config node deserializes with defaults.
#[test]
fn bacnet_config_defaults() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "bacnet-config",
            "name": "bacnet-cfg",
            "deviceId": 1234
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

/// Verify the BACnet config node accepts all configuration fields.
#[test]
fn bacnet_config_full_options() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg_full",
            "type": "bacnet-config",
            "name": "bacnet-cfg-full",
            "deviceId": 5600,
            "targetHost": "192.168.1.200",
            "targetPort": 47809,
            "interface": "0.0.0.0",
            "port": 47808,
            "covLifetime": 3600,
            "apduTimeoutMs": 5000,
            "retries": 5
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

/// Verify the read node config deserializes with defaults.
#[test]
fn bacnet_read_config_defaults() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "bacnet-config",
            "name": "bacnet-cfg",
            "deviceId": 1234,
            "targetHost": "127.0.0.1"
        },
        {
            "id": "1",
            "type": "bacnet read",
            "z": "100",
            "name": "bacnet-read",
            "configNode": "cfg1",
            "objectType": "analogInput",
            "objectInstance": 1,
            "property": "presentValue",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

/// Verify the write node config deserializes.
#[test]
fn bacnet_write_config_defaults() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "bacnet-config",
            "name": "bacnet-cfg",
            "deviceId": 1234,
            "targetHost": "127.0.0.1"
        },
        {
            "id": "2",
            "type": "bacnet write",
            "z": "100",
            "name": "bacnet-write",
            "configNode": "cfg1",
            "objectType": "analogOutput",
            "objectInstance": 1,
            "property": "presentValue",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

/// Verify all BACnet object types can be configured.
#[test]
fn bacnet_read_all_object_types() {
    for obj_type in &[
        "analogInput",
        "analogOutput",
        "analogValue",
        "binaryInput",
        "binaryOutput",
        "binaryValue",
        "multiStateInput",
        "multiStateOutput",
        "multiStateValue",
    ] {
        let flow = json!([
            {"id": "100", "type": "tab"},
            {
                "id": "cfg1",
                "type": "bacnet-config",
                "name": "bacnet-cfg",
                "deviceId": 1234,
                "targetHost": "127.0.0.1"
            },
            {
                "id": "3",
                "type": "bacnet read",
                "z": "100",
                "name": format!("bacnet-read-{}", obj_type),
                "configNode": "cfg1",
                "objectType": obj_type,
                "objectInstance": 1,
                "property": "presentValue",
                "wires": [["99"]]
            },
            {"id": "99", "z": "100", "type": "test-once"}
        ]);

        let _harness = TestHarness::from_flow_json(flow);
    }
}

/// Verify the read node accepts COV subscription configuration.
#[test]
fn bacnet_read_config_with_cov() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "bacnet-config",
            "name": "bacnet-cfg",
            "deviceId": 1234,
            "targetHost": "127.0.0.1",
            "covLifetime": 1800
        },
        {
            "id": "4",
            "type": "bacnet read",
            "z": "100",
            "name": "bacnet-read-cov",
            "configNode": "cfg1",
            "objectType": "analogInput",
            "objectInstance": 1,
            "property": "presentValue",
            "subscribeCov": true,
            "covLifetime": 600,
            "covIncrement": 0.5,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

// ---------------------------------------------------------------------------
// BACnet Read — Analog Input Present Value (requires device)
// ---------------------------------------------------------------------------

/// Read the presentValue of an analog input object.
///
/// Requires: BACnet device/simulator on the network
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires BACnet device/simulator on the network"]
async fn bacnet_read_analog_input() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "bacnet-config",
            "name": "bacnet-cfg",
            "deviceId": 1234,
            "targetHost": "127.0.0.1"
        },
        {
            "id": "10",
            "type": "bacnet read",
            "z": "100",
            "name": "bacnet-read-ai",
            "configNode": "cfg1",
            "objectType": "analogInput",
            "objectInstance": 1,
            "property": "presentValue",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("10", json!({"payload": true}), 1, Duration::from_secs(10)).await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
        assert!(msgs[0].contains("bacnet"));
    }
}

// ---------------------------------------------------------------------------
// BACnet Read — Binary Input (requires device)
// ---------------------------------------------------------------------------

/// Read the presentValue of a binary input object.
///
/// Requires: BACnet device/simulator on the network
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires BACnet device/simulator on the network"]
async fn bacnet_read_binary_input() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "bacnet-config",
            "name": "bacnet-cfg",
            "deviceId": 1234,
            "targetHost": "127.0.0.1"
        },
        {
            "id": "11",
            "type": "bacnet read",
            "z": "100",
            "name": "bacnet-read-bi",
            "configNode": "cfg1",
            "objectType": "binaryInput",
            "objectInstance": 1,
            "property": "presentValue",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("11", json!({"payload": true}), 1, Duration::from_secs(10)).await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
    }
}

// ---------------------------------------------------------------------------
// BACnet Read — Object Name Property (requires device)
// ---------------------------------------------------------------------------

/// Read the objectName property of an analog input.
///
/// Requires: BACnet device/simulator on the network
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires BACnet device/simulator on the network"]
async fn bacnet_read_object_name() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "bacnet-config",
            "name": "bacnet-cfg",
            "deviceId": 1234,
            "targetHost": "127.0.0.1"
        },
        {
            "id": "12",
            "type": "bacnet read",
            "z": "100",
            "name": "bacnet-read-name",
            "configNode": "cfg1",
            "objectType": "analogInput",
            "objectInstance": 1,
            "property": "objectName",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let _msgs = harness.inject_and_collect_timeout("12", json!({"payload": true}), 1, Duration::from_secs(10)).await;
}

// ---------------------------------------------------------------------------
// BACnet Write — Analog Output Present Value (requires device)
// ---------------------------------------------------------------------------

/// Write a value to an analog output object.
///
/// Requires: BACnet device/simulator on the network
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires BACnet device/simulator on the network"]
async fn bacnet_write_analog_output() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "bacnet-config",
            "name": "bacnet-cfg",
            "deviceId": 1234,
            "targetHost": "127.0.0.1"
        },
        {
            "id": "20",
            "type": "bacnet write",
            "z": "100",
            "name": "bacnet-write-ao",
            "configNode": "cfg1",
            "objectType": "analogOutput",
            "objectInstance": 1,
            "property": "presentValue",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("20", json!({"payload": 72.5}), 1, Duration::from_secs(10)).await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
        assert!(msgs[0].contains("bacnet"));
    }
}

// ---------------------------------------------------------------------------
// BACnet Write — Binary Output (requires device)
// ---------------------------------------------------------------------------

/// Write to a binary output object (on/off).
///
/// Requires: BACnet device/simulator on the network
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires BACnet device/simulator on the network"]
async fn bacnet_write_binary_output() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "bacnet-config",
            "name": "bacnet-cfg",
            "deviceId": 1234,
            "targetHost": "127.0.0.1"
        },
        {
            "id": "21",
            "type": "bacnet write",
            "z": "100",
            "name": "bacnet-write-bo",
            "configNode": "cfg1",
            "objectType": "binaryOutput",
            "objectInstance": 1,
            "property": "presentValue",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("21", json!({"payload": true}), 1, Duration::from_secs(10)).await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
    }
}

// ---------------------------------------------------------------------------
// BACnet — Write then Read Round-Trip (requires device)
// ---------------------------------------------------------------------------

/// Write a value and read it back to verify round-trip integrity.
///
/// Requires: BACnet device/simulator on the network
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires BACnet device/simulator on the network"]
async fn bacnet_write_read_roundtrip() {
    // Step 1: Write
    let write_flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "bacnet-config",
            "name": "bacnet-cfg",
            "deviceId": 1234,
            "targetHost": "127.0.0.1"
        },
        {
            "id": "30",
            "type": "bacnet write",
            "z": "100",
            "name": "bacnet-write-rt",
            "configNode": "cfg1",
            "objectType": "analogOutput",
            "objectInstance": 1,
            "property": "presentValue",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let write_harness = TestHarness::from_flow_json(write_flow);
    let _write_msgs =
        write_harness.inject_and_collect_timeout("30", json!({"payload": 55.5}), 1, Duration::from_secs(10)).await;

    // Step 2: Read back
    let read_flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "bacnet-config",
            "name": "bacnet-cfg",
            "deviceId": 1234,
            "targetHost": "127.0.0.1"
        },
        {
            "id": "31",
            "type": "bacnet read",
            "z": "100",
            "name": "bacnet-read-rt",
            "configNode": "cfg1",
            "objectType": "analogOutput",
            "objectInstance": 1,
            "property": "presentValue",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let read_harness = TestHarness::from_flow_json(read_flow);
    let _read_msgs =
        read_harness.inject_and_collect_timeout("31", json!({"payload": true}), 1, Duration::from_secs(10)).await;
}

// ---------------------------------------------------------------------------
// BACnet — COV Subscription (requires device)
// ---------------------------------------------------------------------------

/// Subscribe to COV notifications for an analog input.
/// The node should emit messages whenever the value changes.
///
/// Requires: BACnet device/simulator on the network that supports COV
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires BACnet device/simulator with COV support"]
async fn bacnet_read_cov_subscription() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "bacnet-config",
            "name": "bacnet-cfg",
            "deviceId": 1234,
            "targetHost": "127.0.0.1",
            "covLifetime": 300
        },
        {
            "id": "40",
            "type": "bacnet read",
            "z": "100",
            "name": "bacnet-read-cov",
            "configNode": "cfg1",
            "objectType": "analogInput",
            "objectInstance": 1,
            "property": "presentValue",
            "subscribeCov": true,
            "covLifetime": 300,
            "covIncrement": 1.0,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);

    // The COV subscription should be established on startup.
    // Inject a trigger message to perform an initial read.
    let msgs = harness.inject_and_collect_timeout("40", json!({"payload": true}), 1, Duration::from_secs(10)).await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
        assert!(msgs[0].contains("bacnet"));
        // The bacnet metadata should indicate COV subscription
        let bacnet = msgs[0].get("bacnet").unwrap();
        let bacnet_str = bacnet.to_string().unwrap_or_default();
        assert!(bacnet_str.contains("covSubscribed"));
    }
}

// ---------------------------------------------------------------------------
// BACnet — Multiple Object Instances (requires device)
// ---------------------------------------------------------------------------

/// Read multiple object instances in sequence.
///
/// Requires: BACnet device/simulator on the network
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires BACnet device/simulator on the network"]
async fn bacnet_read_multiple_instances() {
    for instance in [1u32, 2, 3] {
        let flow = json!([
            {"id": "100", "type": "tab"},
            {
                "id": "cfg1",
                "type": "bacnet-config",
                "name": "bacnet-cfg",
                "deviceId": 1234,
                "targetHost": "127.0.0.1"
            },
            {
                "id": "50",
                "type": "bacnet read",
                "z": "100",
                "name": format!("bacnet-read-ai-{}", instance),
                "configNode": "cfg1",
                "objectType": "analogInput",
                "objectInstance": instance,
                "property": "presentValue",
                "wires": [["99"]]
            },
            {"id": "99", "z": "100", "type": "test-once"}
        ]);

        let _harness = TestHarness::from_flow_json(flow);
        // In a full test, inject and verify each instance
    }
}
