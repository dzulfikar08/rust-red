//! BACnet node compatibility tests.
//!
//! Verifies that BACnet read/write/config nodes behave correctly,
//! matching Node-RED behavior.
//!
//! The read, write and COV tests run against an in-process BACnet/IP device (`MockDevice`, a UDP socket that
//! answers ReadProperty and WriteProperty and sends COV notifications).

use std::time::Duration;

use serde_json::json;

use super::harness::{TestHarness, assert_msg_bool, assert_msg_f64, assert_msg_has, assert_msg_num, assert_msg_str};

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

const AI1: u32 = 0x0000_0001;
const AO1: u32 = 0x0040_0001;
const BI2: u32 = 0x00C0_0002;
const BO1: u32 = 0x0100_0001;
const MSV3: u32 = 0x04C0_0003;
const PRESENT_VALUE: u32 = 85;

/// One WriteProperty the mock device accepted.
#[derive(Debug, Clone, PartialEq)]
struct Written {
    object: u32,
    property: u32,
    /// The application-tagged value, as sent.
    value: Vec<u8>,
    priority: Option<u8>,
}

struct Subscriber {
    addr: std::net::SocketAddr,
    process: u32,
    object: u32,
}

/// A BACnet/IP device on a UDP socket: ReadProperty, WriteProperty (values are kept and read back) and
/// SubscribeCOV (an UnconfirmedCOVNotification follows the SimpleAck, more come from `notify`).
///
/// Objects: analogInput 1 (presentValue 72.5 Real, objectName "Supply Temp", units 62), analogOutput 1,
/// binaryInput 2 (presentValue 1, statusFlags all clear), binaryOutput 1, multiStateValue 3 (presentValue 4).
/// Anything else is answered with an Error PDU (object / unknown-object).
struct MockDevice {
    port: u16,
    socket: std::sync::Arc<tokio::net::UdpSocket>,
    writes: std::sync::Arc<std::sync::Mutex<Vec<Written>>>,
    subscribers: std::sync::Arc<std::sync::Mutex<Vec<Subscriber>>>,
    subscribe_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

fn real(v: f32) -> Vec<u8> {
    [&[0x44][..], &v.to_be_bytes()].concat()
}

/// Context-tagged unsigned: tag number, length 1 to 4, minimal big-endian value.
fn ctx_unsigned(tag: u8, value: u32) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let skip = bytes.iter().take(3).take_while(|b| **b == 0).count();
    let mut out = vec![(tag << 4) | 0x08 | (4 - skip) as u8];
    out.extend_from_slice(&bytes[skip..]);
    out
}

fn cov_notification(process: u32, object: u32, present_value: &[u8]) -> Vec<u8> {
    let mut apdu = vec![0x10, 0x02];
    apdu.extend_from_slice(&ctx_unsigned(0, process));
    apdu.extend_from_slice(&[0x1C, 0x02, 0x00, 0x04, 0xD2, 0x2C]);
    apdu.extend_from_slice(&object.to_be_bytes());
    apdu.extend_from_slice(&[0x39, 0x3C, 0x4E, 0x09, 0x55, 0x2E]);
    apdu.extend_from_slice(present_value);
    apdu.extend_from_slice(&[0x2F, 0x09, 0x6F, 0x2E, 0x82, 0x04, 0x00, 0x2F, 0x4F]);
    let mut frame = vec![0x81, 0x0A, 0, 0, 0x01, 0x00];
    frame.extend_from_slice(&apdu);
    let len = frame.len() as u16;
    frame[2..4].copy_from_slice(&len.to_be_bytes());
    frame
}

impl MockDevice {
    async fn start() -> MockDevice {
        use std::collections::HashMap;
        use std::sync::atomic::Ordering;
        use std::sync::{Arc, Mutex};
        let socket = Arc::new(tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let port = socket.local_addr().unwrap().port();
        let writes = Arc::new(Mutex::new(Vec::new()));
        let subscribers = Arc::new(Mutex::new(Vec::<Subscriber>::new()));
        let subscribe_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut values: HashMap<(u32, u32), Vec<u8>> = HashMap::new();
        values.insert((AI1, 85), real(72.5));
        values.insert((AI1, 77), [&[0x75, 12, 0][..], b"Supply Temp"].concat());
        values.insert((AI1, 117), vec![0x91, 62]);
        values.insert((AO1, 85), real(0.0));
        values.insert((BI2, 85), vec![0x91, 1]);
        values.insert((BI2, 111), vec![0x82, 0x04, 0x00]);
        values.insert((BO1, 85), vec![0x91, 0]);
        values.insert((MSV3, 85), vec![0x21, 4]);
        let task = (socket.clone(), writes.clone(), subscribers.clone(), subscribe_count.clone());
        tokio::spawn(async move {
            let (socket, writes, subscribers, subscribe_count) = task;
            let mut buf = [0u8; 1500];
            loop {
                let Ok((n, from)) = socket.recv_from(&mut buf).await else { return };
                // BVLC (4) + NPDU (2), then a confirmed request: type, max-apdu, invoke id, service, service data.
                let apdu = &buf[6..n];
                if apdu.len() < 11 || apdu[0] >> 4 != 0 {
                    continue;
                }
                let invoke = apdu[2];
                let service = apdu[3];
                let object = u32::from_be_bytes([apdu[5], apdu[6], apdu[7], apdu[8]]);
                let mut after_object = 9;
                let reply = |body: &[u8]| {
                    let mut reply = vec![0x81, 0x0A, 0, 0, 0x01, 0x00];
                    reply.extend_from_slice(body);
                    let len = reply.len() as u16;
                    reply[2..4].copy_from_slice(&len.to_be_bytes());
                    reply
                };
                let error = reply(&[0x50, invoke, service, 0x91, 0x01, 0x91, 0x1F]);
                match service {
                    0x0C | 0x0F => {
                        let plen = (apdu[after_object] & 0x07) as usize;
                        let property = apdu[after_object + 1..after_object + 1 + plen]
                            .iter()
                            .fold(0u32, |a, b| (a << 8) | *b as u32);
                        after_object += 1 + plen;
                        if service == 0x0C {
                            let answer = match values.get(&(object, property)) {
                                Some(v) => {
                                    let mut body = vec![0x30, invoke, 0x0C, 0x0C];
                                    body.extend_from_slice(&object.to_be_bytes());
                                    body.extend_from_slice(&apdu[9..after_object]);
                                    body.push(0x3E);
                                    body.extend_from_slice(v);
                                    body.push(0x3F);
                                    reply(&body)
                                }
                                None => error,
                            };
                            let _ = socket.send_to(&answer, from).await;
                            continue;
                        }
                        // WriteProperty: opening tag 3, one primitive, closing tag 3, optional priority [4].
                        let start = after_object + 1;
                        let head = apdu[start];
                        let (header, len) = match head & 0x07 {
                            5 => (2, apdu[start + 1] as usize),
                            _ if head >> 4 == 1 => (1, 0),
                            l => (1, l as usize),
                        };
                        let value = apdu[start..start + header + len].to_vec();
                        let mut rest = start + header + len + 1;
                        let priority = (apdu.len() > rest && apdu[rest] == 0x49).then(|| apdu[rest + 1]);
                        if priority.is_some() {
                            rest += 2;
                        }
                        let _ = rest;
                        let answer = if values.contains_key(&(object, property)) {
                            values.insert((object, property), value.clone());
                            writes.lock().unwrap().push(Written { object, property, value, priority });
                            reply(&[0x20, invoke, 0x0F])
                        } else {
                            error
                        };
                        let _ = socket.send_to(&answer, from).await;
                    }
                    0x05 => {
                        // SubscribeCOV: [0] process id, [1] object id, [2] confirmed, [3] lifetime.
                        let plen = (apdu[4] & 0x07) as usize;
                        let process = apdu[5..5 + plen].iter().fold(0u32, |a, b| (a << 8) | *b as u32);
                        let at = 5 + plen + 1;
                        let object = u32::from_be_bytes([apdu[at], apdu[at + 1], apdu[at + 2], apdu[at + 3]]);
                        if !values.contains_key(&(object, 85)) {
                            let _ = socket.send_to(&error, from).await;
                            continue;
                        }
                        subscribe_count.fetch_add(1, Ordering::SeqCst);
                        {
                            let mut subs = subscribers.lock().unwrap();
                            subs.retain(|s| !(s.addr == from && s.process == process && s.object == object));
                            subs.push(Subscriber { addr: from, process, object });
                        }
                        let _ = socket.send_to(&reply(&[0x20, invoke, 0x05]), from).await;
                        let _ = socket.send_to(&cov_notification(process, object, &values[&(object, 85)]), from).await;
                    }
                    _ => {}
                }
            }
        });
        MockDevice { port, socket, writes, subscribers, subscribe_count }
    }

    /// Send an UnconfirmedCOVNotification with this present value to every subscriber of `object`.
    async fn notify(&self, object: u32, present_value: &[u8]) {
        let targets: Vec<(std::net::SocketAddr, u32)> = self
            .subscribers
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.object == object)
            .map(|s| (s.addr, s.process))
            .collect();
        for (addr, process) in targets {
            let _ = self.socket.send_to(&cov_notification(process, object, present_value), addr).await;
        }
    }

    async fn wait_for_subscriber(&self) {
        for _ in 0..100 {
            if !self.subscribers.lock().unwrap().is_empty() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("nobody subscribed");
    }

    fn writes(&self) -> Vec<Written> {
        self.writes.lock().unwrap().clone()
    }
}

async fn start_mock_device() -> u16 {
    let device = MockDevice::start().await;
    let port = device.port;
    // The tasks of the device keep running on their own; the handle is not needed by the read tests.
    std::mem::forget(device);
    port
}

fn read_flow(port: u16, object_type: &str, instance: u32, extra: serde_json::Value) -> serde_json::Value {
    node_flow(port, "bacnet read", object_type, instance, extra)
}

fn node_flow(
    port: u16,
    node_type: &str,
    object_type: &str,
    instance: u32,
    extra: serde_json::Value,
) -> serde_json::Value {
    let mut node = json!({
        "id": "10",
        "type": node_type,
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
// BACnet Write
// ---------------------------------------------------------------------------

async fn write_once(
    device: &MockDevice,
    object_type: &str,
    instance: u32,
    extra: serde_json::Value,
    msg: serde_json::Value,
) -> Vec<rust_red_core::runtime::model::Msg> {
    let harness = TestHarness::from_flow_json(node_flow(device.port, "bacnet write", object_type, instance, extra));
    harness.inject_and_collect_timeout("10", msg, 1, Duration::from_secs(5)).await
}

/// An analog output is written as Real at the configured priority, and the value reads back.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bacnet_write_analog_output_with_priority() {
    let device = MockDevice::start().await;
    let msgs = write_once(&device, "analogOutput", 1, json!({"priority": 8}), json!({"payload": 55.5})).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_bool(&msgs[0], "payload", true);
    assert_msg_has(&msgs[0], "bacnet");
    assert_eq!(
        device.writes(),
        vec![Written { object: AO1, property: PRESENT_VALUE, value: real(55.5), priority: Some(8) }]
    );

    let harness = TestHarness::from_flow_json(read_flow(device.port, "analogOutput", 1, json!({})));
    let msgs = harness.inject_and_collect_timeout("10", json!({"payload": 0}), 1, Duration::from_secs(5)).await;
    assert_eq!(msgs.len(), 1);
    assert_msg_f64(&msgs[0], "payload", 55.5);
}

/// Binary outputs take Enumerated 0/1 from booleans and strings, with no priority unless one is set (also as a string).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bacnet_write_binary_and_multistate() {
    let device = MockDevice::start().await;
    assert_eq!(write_once(&device, "binaryOutput", 1, json!({}), json!({"payload": true})).await.len(), 1);
    assert_eq!(
        write_once(&device, "binaryOutput", 1, json!({"priority": "16"}), json!({"payload": "inactive"})).await.len(),
        1
    );
    assert_eq!(
        write_once(&device, "multiStateValue", 3, json!({"priority": ""}), json!({"payload": 2})).await.len(),
        1
    );
    assert_eq!(
        device.writes(),
        vec![
            Written { object: BO1, property: PRESENT_VALUE, value: vec![0x91, 1], priority: None },
            Written { object: BO1, property: PRESENT_VALUE, value: vec![0x91, 0], priority: Some(16) },
            Written { object: MSV3, property: PRESENT_VALUE, value: vec![0x21, 2], priority: None },
        ]
    );
}

/// Null and `relinquish` write NULL at the priority; `msg.priority` overrides the node setting.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bacnet_write_relinquish_and_priority_override() {
    let device = MockDevice::start().await;
    write_once(&device, "analogOutput", 1, json!({"priority": 8}), json!({"payload": null})).await;
    write_once(&device, "analogOutput", 1, json!({"priority": 9, "relinquish": true}), json!({"payload": 5})).await;
    write_once(&device, "analogOutput", 1, json!({"priority": 9}), json!({"payload": 1.5, "priority": 3})).await;
    write_once(&device, "analogOutput", 1, json!({}), json!({"payload": 1.5, "relinquish": true, "priority": 10}))
        .await;
    assert_eq!(
        device.writes(),
        vec![
            Written { object: AO1, property: PRESENT_VALUE, value: vec![0x00], priority: Some(8) },
            Written { object: AO1, property: PRESENT_VALUE, value: vec![0x00], priority: Some(9) },
            Written { object: AO1, property: PRESENT_VALUE, value: real(1.5), priority: Some(3) },
            Written { object: AO1, property: PRESENT_VALUE, value: vec![0x00], priority: Some(10) },
        ]
    );
}

/// Other properties follow the JSON type; `valueType` forces one.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bacnet_write_value_types() {
    let device = MockDevice::start().await;
    write_once(&device, "analogInput", 1, json!({"property": "objectName"}), json!({"payload": "Return"})).await;
    write_once(&device, "analogOutput", 1, json!({"valueType": "double"}), json!({"payload": 2.5})).await;
    assert_eq!(
        device.writes(),
        vec![
            Written { object: AI1, property: 77, value: [&[0x75, 7, 0][..], b"Return"].concat(), priority: None },
            Written {
                object: AO1,
                property: PRESENT_VALUE,
                value: [&[0x55, 8][..], &2.5f64.to_be_bytes()].concat(),
                priority: None
            },
        ]
    );
}

/// A device error, a wrong value and an unknown object produce no output message.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bacnet_write_failures_emit_nothing() {
    let device = MockDevice::start().await;
    assert!(write_once(&device, "analogOutput", 9, json!({}), json!({"payload": 1})).await.is_empty());
    assert!(write_once(&device, "analogOutput", 1, json!({}), json!({"payload": "warm"})).await.is_empty());
    assert!(write_once(&device, "analogOutput", 1, json!({}), json!({"payload": 1, "priority": 17})).await.is_empty());
    assert!(
        write_once(&device, "analogOutput", 1, json!({"relinquish": true}), json!({"payload": 1})).await.is_empty()
    );
    assert!(device.writes().is_empty());
}

// ---------------------------------------------------------------------------
// BACnet COV subscription
// ---------------------------------------------------------------------------

/// The node subscribes, emits the initial notification and every later one, and keeps the other properties in msg.bacnet.values.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bacnet_cov_notifications_become_messages() {
    let device = std::sync::Arc::new(MockDevice::start().await);
    let harness = TestHarness::from_flow_json(read_flow(
        device.port,
        "analogInput",
        1,
        json!({"subscribeCov": true, "covLifetime": 60}),
    ));
    let pusher = {
        let device = device.clone();
        tokio::spawn(async move {
            device.wait_for_subscriber().await;
            tokio::time::sleep(Duration::from_millis(300)).await;
            device.notify(AI1, &real(80.0)).await;
            device.notify(AI1, &real(81.5)).await;
        })
    };
    let msgs = harness.run_with_timeout(3, Duration::from_secs(10)).await;
    pusher.await.unwrap();
    assert_eq!(msgs.len(), 3);
    assert_msg_f64(&msgs[0], "payload", 72.5);
    assert_msg_f64(&msgs[1], "payload", 80.0);
    assert_msg_f64(&msgs[2], "payload", 81.5);
    let bacnet = msgs[2].get("bacnet").and_then(|b| b.as_object()).expect("msg.bacnet");
    assert_eq!(bacnet.get("cov").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(bacnet.get("objectType").and_then(|v| v.as_str()), Some("analogInput"));
    let values = bacnet.get("values").and_then(|v| v.as_object()).expect("values");
    let flags = values.get("statusFlags").and_then(|v| v.as_array()).expect("statusFlags");
    assert_eq!(flags.len(), 4);
}

/// covIncrement drops changes smaller than the increment (the first one is always emitted).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bacnet_cov_increment_filters_small_changes() {
    let device = std::sync::Arc::new(MockDevice::start().await);
    let harness = TestHarness::from_flow_json(read_flow(
        device.port,
        "analogInput",
        1,
        json!({"subscribeCov": true, "covLifetime": 60, "covIncrement": 5.0}),
    ));
    let pusher = {
        let device = device.clone();
        tokio::spawn(async move {
            device.wait_for_subscriber().await;
            tokio::time::sleep(Duration::from_millis(300)).await;
            device.notify(AI1, &real(73.0)).await;
            device.notify(AI1, &real(80.0)).await;
            device.notify(AI1, &real(82.0)).await;
        })
    };
    let msgs = harness.run_with_timeout(2, Duration::from_secs(10)).await;
    pusher.await.unwrap();
    assert_eq!(msgs.len(), 2);
    assert_msg_f64(&msgs[0], "payload", 72.5);
    assert_msg_f64(&msgs[1], "payload", 80.0);
}

/// The subscription is renewed at half its lifetime.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bacnet_cov_subscription_is_renewed() {
    use std::sync::atomic::Ordering;
    let device = MockDevice::start().await;
    let harness = TestHarness::from_flow_json(read_flow(
        device.port,
        "analogInput",
        1,
        json!({"subscribeCov": true, "covLifetime": 2}),
    ));
    let _ = harness.run_with_timeout(4, Duration::from_millis(3500)).await;
    assert!(
        device.subscribe_count.load(Ordering::SeqCst) >= 3,
        "renewals: {}",
        device.subscribe_count.load(Ordering::SeqCst)
    );
}
