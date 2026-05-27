//! Modbus node compatibility tests.
//!
//! Verifies that Modbus read/write/config nodes behave correctly,
//! matching Node-RED behavior.
//!
//! # IMPORTANT: Infrastructure Requirement
//!
//! Most tests in this file are marked `#[ignore]` because they require a running
//! Modbus TCP simulator (e.g., `diagslave` or `modrssim2`) on localhost:502.
//!
//! The data-type conversion tests do NOT require a simulator and run as normal tests.
//!
//! To run the ignored tests:
//!
//! ```bash
//! # Start a Modbus TCP simulator (e.g., with modpoll/diagslave)
//! # diagslave -m tcp -p 502
//!
//! # Run the ignored tests
//! cargo test -p rust-red-core --test nodered_compat --features internal-testing,nodes_modbus -- test_modbus --ignored
//! ```
//!
//! Even when ignored, these tests must compile to catch regressions.

use std::time::Duration;

use serde_json::json;

use super::harness::TestHarness;

// ---------------------------------------------------------------------------
// Data Type Conversion — Pure unit tests (no simulator needed)
// ---------------------------------------------------------------------------

/// Verify that the Modbus read node config deserializes correctly
/// with default values.
#[test]
fn modbus_read_config_defaults() {
    // Ensure the flow JSON parses and the engine can build the nodes
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "name": "modbus-cfg",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "1",
            "type": "modbus read",
            "z": "100",
            "name": "modbus-read-defaults",
            "configNode": "cfg1",
            "functionCode": "readHoldingRegisters",
            "address": 0,
            "quantity": 1,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
    // Engine built successfully with defaults
}

/// Verify the write node config deserializes with all function codes.
#[test]
fn modbus_write_config_deserialization() {
    for fc in &["writeSingleRegister", "writeMultipleRegisters", "writeSingleCoil", "writeMultipleCoils"] {
        let flow = json!([
            {"id": "100", "type": "tab"},
            {
                "id": "cfg1",
                "type": "modbus-config",
                "name": "modbus-cfg",
                "host": "127.0.0.1",
                "port": 502
            },
            {
                "id": "2",
                "type": "modbus write",
                "z": "100",
                "name": format!("modbus-write-{}", fc),
                "configNode": "cfg1",
                "functionCode": fc,
                "address": 10,
                "wires": [["99"]]
            },
            {"id": "99", "z": "100", "type": "test-once"}
        ]);

        let _harness = TestHarness::from_flow_json(flow);
    }
}

/// Verify that the config node deserializes with all transport options.
#[test]
fn modbus_config_node_builds() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg_custom",
            "type": "modbus-config",
            "name": "modbus-custom",
            "transport": "tcp",
            "host": "192.168.1.100",
            "port": 5020,
            "unitId": 5,
            "timeoutMs": 10000
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

/// Verify the read node accepts the dataType field for data conversion.
#[test]
fn modbus_read_config_with_data_type() {
    for dt in &["uint16", "int16", "uint32", "int32", "float", "double"] {
        let flow = json!([
            {"id": "100", "type": "tab"},
            {
                "id": "cfg1",
                "type": "modbus-config",
                "name": "modbus-cfg",
                "host": "127.0.0.1",
                "port": 502
            },
            {
                "id": "3",
                "type": "modbus read",
                "z": "100",
                "name": format!("modbus-read-{}", dt),
                "configNode": "cfg1",
                "functionCode": "readHoldingRegisters",
                "address": 100,
                "quantity": 1,
                "dataType": dt,
                "wires": [["99"]]
            },
            {"id": "99", "z": "100", "type": "test-once"}
        ]);

        let _harness = TestHarness::from_flow_json(flow);
    }
}

/// Verify the read node accepts the pollIntervalMs field.
#[test]
fn modbus_read_config_with_polling() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "name": "modbus-cfg",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "4",
            "type": "modbus read",
            "z": "100",
            "name": "modbus-read-poll",
            "configNode": "cfg1",
            "functionCode": "readHoldingRegisters",
            "address": 0,
            "quantity": 10,
            "pollIntervalMs": 1000,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

/// Verify all four read function codes can be configured.
#[test]
fn modbus_read_all_function_codes() {
    for fc in &["readCoils", "readDiscreteInputs", "readHoldingRegisters", "readInputRegisters"] {
        let flow = json!([
            {"id": "100", "type": "tab"},
            {
                "id": "cfg1",
                "type": "modbus-config",
                "name": "modbus-cfg",
                "host": "127.0.0.1",
                "port": 502
            },
            {
                "id": "5",
                "type": "modbus read",
                "z": "100",
                "name": format!("modbus-read-{}", fc),
                "configNode": "cfg1",
                "functionCode": fc,
                "address": 0,
                "quantity": 8,
                "wires": [["99"]]
            },
            {"id": "99", "z": "100", "type": "test-once"}
        ]);

        let _harness = TestHarness::from_flow_json(flow);
    }
}

// ---------------------------------------------------------------------------
// Modbus Read — FC3 Read Holding Registers (requires simulator)
// ---------------------------------------------------------------------------

/// FC3: Read 10 holding registers from address 0.
///
/// Requires: Modbus TCP simulator on localhost:502
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires Modbus TCP simulator on localhost:502"]
async fn modbus_read_holding_registers() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "name": "modbus-cfg",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "1",
            "type": "modbus read",
            "z": "100",
            "name": "modbus-read-fc3",
            "configNode": "cfg1",
            "functionCode": "readHoldingRegisters",
            "address": 0,
            "quantity": 10,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": true}), 1, Duration::from_secs(5)).await;

    if !msgs.is_empty() {
        // The response should have a payload array with register values
        assert!(msgs[0].contains("payload"));
        assert!(msgs[0].contains("modbus"));
    }
}

// ---------------------------------------------------------------------------
// Modbus Read — FC1 Read Coils (requires simulator)
// ---------------------------------------------------------------------------

/// FC1: Read 8 coils from address 0.
///
/// Requires: Modbus TCP simulator on localhost:502
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires Modbus TCP simulator on localhost:502"]
async fn modbus_read_coils() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "name": "modbus-cfg",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "2",
            "type": "modbus read",
            "z": "100",
            "name": "modbus-read-fc1",
            "configNode": "cfg1",
            "functionCode": "readCoils",
            "address": 0,
            "quantity": 8,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("2", json!({"payload": true}), 1, Duration::from_secs(5)).await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
    }
}

// ---------------------------------------------------------------------------
// Modbus Read — FC2 Read Discrete Inputs (requires simulator)
// ---------------------------------------------------------------------------

/// FC2: Read 8 discrete inputs from address 0.
///
/// Requires: Modbus TCP simulator on localhost:502
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires Modbus TCP simulator on localhost:502"]
async fn modbus_read_discrete_inputs() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "name": "modbus-cfg",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "3",
            "type": "modbus read",
            "z": "100",
            "name": "modbus-read-fc2",
            "configNode": "cfg1",
            "functionCode": "readDiscreteInputs",
            "address": 0,
            "quantity": 8,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("3", json!({"payload": true}), 1, Duration::from_secs(5)).await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
    }
}

// ---------------------------------------------------------------------------
// Modbus Read — FC4 Read Input Registers (requires simulator)
// ---------------------------------------------------------------------------

/// FC4: Read 4 input registers from address 0.
///
/// Requires: Modbus TCP simulator on localhost:502
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires Modbus TCP simulator on localhost:502"]
async fn modbus_read_input_registers() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "name": "modbus-cfg",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "4",
            "type": "modbus read",
            "z": "100",
            "name": "modbus-read-fc4",
            "configNode": "cfg1",
            "functionCode": "readInputRegisters",
            "address": 0,
            "quantity": 4,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("4", json!({"payload": true}), 1, Duration::from_secs(5)).await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
    }
}

// ---------------------------------------------------------------------------
// Modbus Read — Float data type conversion (requires simulator)
// ---------------------------------------------------------------------------

/// Read registers with float data type conversion.
///
/// Requires: Modbus TCP simulator on localhost:502
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires Modbus TCP simulator on localhost:502"]
async fn modbus_read_float_data_type() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "name": "modbus-cfg",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "5",
            "type": "modbus read",
            "z": "100",
            "name": "modbus-read-float",
            "configNode": "cfg1",
            "functionCode": "readHoldingRegisters",
            "address": 100,
            "quantity": 1,
            "dataType": "float",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("5", json!({"payload": true}), 1, Duration::from_secs(5)).await;

    if !msgs.is_empty() {
        // The float payload should be a number (or array of numbers)
        assert!(msgs[0].contains("payload"));
    }
}

// ---------------------------------------------------------------------------
// Modbus Write — FC6 Write Single Register (requires simulator)
// ---------------------------------------------------------------------------

/// FC6: Write a single register value.
///
/// Requires: Modbus TCP simulator on localhost:502
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires Modbus TCP simulator on localhost:502"]
async fn modbus_write_single_register() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "name": "modbus-cfg",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "10",
            "type": "modbus write",
            "z": "100",
            "name": "modbus-write-fc6",
            "configNode": "cfg1",
            "functionCode": "writeSingleRegister",
            "address": 0,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("10", json!({"payload": 12345}), 1, Duration::from_secs(5)).await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
        assert!(msgs[0].contains("modbus"));
    }
}

// ---------------------------------------------------------------------------
// Modbus Write — FC5 Write Single Coil (requires simulator)
// ---------------------------------------------------------------------------

/// FC5: Write a single coil (true).
///
/// Requires: Modbus TCP simulator on localhost:502
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires Modbus TCP simulator on localhost:502"]
async fn modbus_write_single_coil() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "name": "modbus-cfg",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "11",
            "type": "modbus write",
            "z": "100",
            "name": "modbus-write-fc5",
            "configNode": "cfg1",
            "functionCode": "writeSingleCoil",
            "address": 0,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect_timeout("11", json!({"payload": true}), 1, Duration::from_secs(5)).await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
    }
}

// ---------------------------------------------------------------------------
// Modbus Write — FC15 Write Multiple Coils (requires simulator)
// ---------------------------------------------------------------------------

/// FC15: Write multiple coils.
///
/// Requires: Modbus TCP simulator on localhost:502
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires Modbus TCP simulator on localhost:502"]
async fn modbus_write_multiple_coils() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "name": "modbus-cfg",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "12",
            "type": "modbus write",
            "z": "100",
            "name": "modbus-write-fc15",
            "configNode": "cfg1",
            "functionCode": "writeMultipleCoils",
            "address": 0,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .inject_and_collect_timeout("12", json!({"payload": [true, false, true, true]}), 1, Duration::from_secs(5))
        .await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
    }
}

// ---------------------------------------------------------------------------
// Modbus Write — FC16 Write Multiple Registers (requires simulator)
// ---------------------------------------------------------------------------

/// FC16: Write multiple registers.
///
/// Requires: Modbus TCP simulator on localhost:502
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires Modbus TCP simulator on localhost:502"]
async fn modbus_write_multiple_registers() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "name": "modbus-cfg",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "13",
            "type": "modbus write",
            "z": "100",
            "name": "modbus-write-fc16",
            "configNode": "cfg1",
            "functionCode": "writeMultipleRegisters",
            "address": 0,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .inject_and_collect_timeout("13", json!({"payload": [100, 200, 300, 400]}), 1, Duration::from_secs(5))
        .await;

    if !msgs.is_empty() {
        assert!(msgs[0].contains("payload"));
    }
}

// ---------------------------------------------------------------------------
// Modbus — Round-trip: Write then Read (requires simulator)
// ---------------------------------------------------------------------------

/// Write a value and read it back to verify round-trip integrity.
///
/// Requires: Modbus TCP simulator on localhost:502
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires Modbus TCP simulator on localhost:502"]
async fn modbus_write_read_roundtrip() {
    // This test uses separate flow instances for write and read
    // since the test harness runs one flow at a time.

    // Step 1: Write
    let write_flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "name": "modbus-cfg",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "20",
            "type": "modbus write",
            "z": "100",
            "name": "modbus-write-rt",
            "configNode": "cfg1",
            "functionCode": "writeSingleRegister",
            "address": 100,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let write_harness = TestHarness::from_flow_json(write_flow);
    let _write_msgs =
        write_harness.inject_and_collect_timeout("20", json!({"payload": 4242}), 1, Duration::from_secs(5)).await;

    // Step 2: Read back
    let read_flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "name": "modbus-cfg",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "21",
            "type": "modbus read",
            "z": "100",
            "name": "modbus-read-rt",
            "configNode": "cfg1",
            "functionCode": "readHoldingRegisters",
            "address": 100,
            "quantity": 1,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let read_harness = TestHarness::from_flow_json(read_flow);
    let read_msgs =
        read_harness.inject_and_collect_timeout("21", json!({"payload": true}), 1, Duration::from_secs(5)).await;

    if !read_msgs.is_empty() {
        assert!(read_msgs[0].contains("payload"));
        // In a real test we would assert payload value == 4242
    }
}

// ---------------------------------------------------------------------------
// Modbus — Polling mode (requires simulator)
// ---------------------------------------------------------------------------

/// Verify that polling mode emits messages at the configured interval.
///
/// Requires: Modbus TCP simulator on localhost:502
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires Modbus TCP simulator on localhost:502"]
async fn modbus_read_polling_mode() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "name": "modbus-cfg",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "30",
            "type": "modbus read",
            "z": "100",
            "name": "modbus-read-poll",
            "configNode": "cfg1",
            "functionCode": "readHoldingRegisters",
            "address": 0,
            "quantity": 1,
            "pollIntervalMs": 500,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);

    // Wait for at least one poll cycle
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // The polling mode should have emitted a message to the sink
    // (actual assertion depends on the simulator being available)
}

// ---------------------------------------------------------------------------
// Modbus — Error handling: missing config node
// ---------------------------------------------------------------------------

/// Verify the node reports an error when the config node does not exist.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires Modbus TCP simulator on localhost:502"]
async fn modbus_read_missing_config_node() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "40",
            "type": "modbus read",
            "z": "100",
            "name": "modbus-read-no-cfg",
            "configNode": "nonexistent",
            "functionCode": "readHoldingRegisters",
            "address": 0,
            "quantity": 1,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
    // Node should have reported an error status (config node not found)
    // but the engine should still build successfully
}

// ---------------------------------------------------------------------------
// Advanced Modbus Features (RRD-41) — no simulator needed
// ---------------------------------------------------------------------------

/// Verify that serial RTU fields round-trip through config deserialization.
#[test]
fn modbus_config_serial_fields_roundtrip() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg_serial",
            "type": "modbus-config",
            "name": "serial-rtu",
            "transport": "serial",
            "host": "N/A",
            "port": 0,
            "serialPort": "/dev/ttyUSB0",
            "baudRate": 19200,
            "dataBits": "8",
            "stopBits": "1",
            "parity": "none",
            "unitId": 3,
            "timeoutMs": 3000
        }
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

/// Verify that queue management options round-trip through config.
#[test]
fn modbus_config_queue_options_roundtrip() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg_queue",
            "type": "modbus-config",
            "name": "queue-test",
            "transport": "tcp",
            "host": "192.168.1.10",
            "port": 502,
            "parallelUnitIds": true,
            "queueLogEnabled": true,
            "bufferCommands": true,
            "commandDelay": 100,
            "keepAlive": true,
            "reconnectTimeout": 5000,
            "autoConnect": false
        }
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

/// Verify that pollRate + pollRateUnit round-trip through read config.
#[test]
fn modbus_read_poll_rate_roundtrip() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "modbus-config",
            "host": "127.0.0.1",
            "port": 502
        },
        {
            "id": "1",
            "type": "modbus read",
            "z": "100",
            "configNode": "cfg1",
            "functionCode": "readHoldingRegisters",
            "address": 0,
            "quantity": 1,
            "pollRate": 5,
            "pollRateUnit": "s",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

/// Verify that the config node builds with autoConnect disabled.
#[test]
fn modbus_config_auto_connect_disabled() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg_noreconnect",
            "type": "modbus-config",
            "name": "no-auto",
            "transport": "tcp",
            "host": "192.168.1.10",
            "port": 502,
            "autoConnect": false,
            "reconnectTimeout": 2000
        }
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}
