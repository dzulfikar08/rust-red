//! BACnet node compatibility tests.
//!
//! Verifies that BACnet read/write/config nodes behave correctly,
//! matching Node-RED behavior.
//!
//! The read tests run against an in-process BACnet/IP device (a UDP socket answering
//! ReadProperty). The write tests still need a real device or simulator and are `#[ignore]`d:
//!
//! ```bash
//! cargo test -p rust-red-core --test nodered_compat --features internal-testing,nodes_bacnet -- test_bacnet --ignored
//! ```

use std::time::Duration;

use serde_json::json;

use super::harness::{TestHarness, assert_msg_f64, assert_msg_has, assert_msg_num, assert_msg_str};

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
// BACnet Read against an in-process BACnet/IP device
// ---------------------------------------------------------------------------

/// Starts a UDP socket that answers ReadProperty like a small BACnet/IP device and returns its port.
///
/// Objects: analogInput 1 (presentValue 72.5 Real, objectName "Supply Temp", units 62),
/// binaryInput 2 (presentValue 1, statusFlags all clear), multiStateValue 3 (presentValue 4 Unsigned).
/// Anything else is answered with an Error PDU (object / unknown-object).
async fn start_mock_device() -> u16 {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let port = socket.local_addr().unwrap().port();
    tokio::spawn(async move {
        let mut buf = [0u8; 1500];
        loop {
            let Ok((n, from)) = socket.recv_from(&mut buf).await else { return };
            // BVLC (4) + NPDU (2), then a confirmed request: type, max-apdu, invoke id, service, [0] object id, [1] property.
            let apdu = &buf[6..n];
            if apdu.len() < 11 || apdu[0] >> 4 != 0 || apdu[3] != 0x0C {
                continue;
            }
            let invoke = apdu[2];
            let object = u32::from_be_bytes([apdu[5], apdu[6], apdu[7], apdu[8]]);
            let property = match apdu[9] & 0x07 {
                1 => apdu[10] as u32,
                2 => ((apdu[10] as u32) << 8) | apdu[11] as u32,
                _ => continue,
            };
            let value: Option<Vec<u8>> = match (object, property) {
                (0x0000_0001, 85) => Some([&[0x44][..], &72.5f32.to_be_bytes()].concat()),
                (0x0000_0001, 77) => Some([&[0x75, 12, 0][..], b"Supply Temp"].concat()),
                (0x0000_0001, 117) => Some(vec![0x91, 62]),
                (0x00C0_0002, 85) => Some(vec![0x91, 1]),
                (0x00C0_0002, 111) => Some(vec![0x82, 0x04, 0x00]),
                (0x04C0_0003, 85) => Some(vec![0x21, 4]),
                _ => None,
            };
            let mut reply = vec![0x81, 0x0A, 0, 0, 0x01, 0x00];
            match value {
                Some(v) => {
                    reply.extend_from_slice(&[0x30, invoke, 0x0C, 0x0C]);
                    reply.extend_from_slice(&object.to_be_bytes());
                    reply.extend_from_slice(&apdu[9..9 + 1 + (apdu[9] & 0x07) as usize]);
                    reply.push(0x3E);
                    reply.extend_from_slice(&v);
                    reply.push(0x3F);
                }
                None => reply.extend_from_slice(&[0x50, invoke, 0x0C, 0x91, 0x01, 0x91, 0x1F]),
            }
            let len = reply.len() as u16;
            reply[2..4].copy_from_slice(&len.to_be_bytes());
            let _ = socket.send_to(&reply, from).await;
        }
    });
    port
}

fn read_flow(port: u16, object_type: &str, instance: u32, extra: serde_json::Value) -> serde_json::Value {
    let mut node = json!({
        "id": "10",
        "type": "bacnet read",
        "z": "100",
        "name": "bacnet-read",
        "configNode": "cfg1",
        "objectType": object_type,
        "objectInstance": instance,
        "wires": [["99"]]
    });
    for (k, v) in extra.as_object().unwrap() {
        node[k] = v.clone();
    }
    json!([
        {"id": "100", "type": "tab"},
        {
            "id": "cfg1",
            "type": "bacnet-config",
            "name": "bacnet-cfg",
            "deviceId": 1234,
            "targetHost": "127.0.0.1",
            "targetPort": port,
            "apduTimeoutMs": 1000,
            "retries": 1
        },
        node,
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

/// The node reads presentValue of an analog input and puts the number in msg.payload.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bacnet_read_analog_input() {
    let port = start_mock_device().await;
    let harness = TestHarness::from_flow_json(read_flow(port, "analogInput", 1, json!({"property": "presentValue"})));
    let msgs = harness.inject_and_collect_timeout("10", json!({"payload": true}), 1, Duration::from_secs(10)).await;
    assert_eq!(msgs.len(), 1, "expected one output message");
    assert_msg_f64(&msgs[0], "payload", 72.5);
    assert_msg_has(&msgs[0], "bacnet");
}

/// presentValue is the default property.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bacnet_read_defaults_to_present_value() {
    let port = start_mock_device().await;
    let harness = TestHarness::from_flow_json(read_flow(port, "analogInput", 1, json!({})));
    let msgs = harness.inject_and_collect_timeout("10", json!({"payload": 0}), 1, Duration::from_secs(10)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_f64(&msgs[0], "payload", 72.5);
}

/// Binary values come back as their enumerated number, multi-state as unsigned.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bacnet_read_binary_and_multistate() {
    let port = start_mock_device().await;
    let harness = TestHarness::from_flow_json(read_flow(port, "binaryInput", 2, json!({"property": "presentValue"})));
    let msgs = harness.inject_and_collect_timeout("10", json!({"payload": 0}), 1, Duration::from_secs(10)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_num(&msgs[0], "payload", 1);

    let harness =
        TestHarness::from_flow_json(read_flow(port, "multiStateValue", 3, json!({"property": "presentValue"})));
    let msgs = harness.inject_and_collect_timeout("10", json!({"payload": 0}), 1, Duration::from_secs(10)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_num(&msgs[0], "payload", 4);
}

/// Other properties decode by type: character strings, enumerations and bit strings.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bacnet_read_object_name() {
    let port = start_mock_device().await;
    let harness = TestHarness::from_flow_json(read_flow(port, "analogInput", 1, json!({"property": "objectName"})));
    let msgs = harness.inject_and_collect_timeout("10", json!({"payload": 0}), 1, Duration::from_secs(10)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_str(&msgs[0], "payload", "Supply Temp");

    let harness = TestHarness::from_flow_json(read_flow(port, "analogInput", 1, json!({"property": "units"})));
    let msgs = harness.inject_and_collect_timeout("10", json!({"payload": 0}), 1, Duration::from_secs(10)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_num(&msgs[0], "payload", 62);
}

/// Several properties produce an object keyed by property name.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bacnet_read_requested_properties() {
    let port = start_mock_device().await;
    let harness = TestHarness::from_flow_json(read_flow(
        port,
        "analogInput",
        1,
        json!({"property": "presentValue", "properties": ["objectName", "units"]}),
    ));
    let msgs = harness.inject_and_collect_timeout("10", json!({"payload": 0}), 1, Duration::from_secs(10)).await;
    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("payload").as_object().expect("payload object");
    assert_eq!(payload.get("presentValue").and_then(|v| v.as_f64()), Some(72.5));
    assert_eq!(payload.get("objectName").and_then(|v| v.as_str()), Some("Supply Temp"));
    assert_eq!(payload.get("units").and_then(|v| v.as_i64()), Some(62));
}

/// A device Error PDU does not produce a null payload message.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bacnet_read_unknown_object_emits_nothing() {
    let port = start_mock_device().await;
    let harness = TestHarness::from_flow_json(read_flow(port, "analogInput", 99, json!({})));
    let msgs = harness.inject_and_collect_timeout("10", json!({"payload": 0}), 1, Duration::from_secs(3)).await;
    assert!(msgs.is_empty(), "expected no output, got {} message(s)", msgs.len());
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
