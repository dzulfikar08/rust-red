//! MQTT Broker node compatibility tests using the embedded broker.
//!
//! These tests use the built-in MQTT broker from `rust-red-mqtt-broker` instead
//! of requiring an external broker like Mosquitto. All tests run self-contained.

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use super::harness::TestHarness;

// ---------------------------------------------------------------------------
// Helper: Start embedded broker
// ---------------------------------------------------------------------------

/// Start an embedded MQTT broker on an OS-assigned port and return the address.
/// The broker runs in a background task for the lifetime of the test.
async fn start_embedded_broker() -> std::net::SocketAddr {
    let config = rust_red_mqtt_broker::config::BrokerConfig {
        bind: "127.0.0.1:0".to_string(),
        enabled: true,
        ..Default::default()
    };
    let broker = Arc::new(rust_red_mqtt_broker::broker::MqttBroker::new(config));
    let addr = broker.clone().start_background().await.expect("embedded broker start");
    // Give the broker a moment to be ready
    tokio::time::sleep(Duration::from_millis(50)).await;
    addr
}

// ---------------------------------------------------------------------------
// Test: Engine builds with MQTT broker config node
// ---------------------------------------------------------------------------

/// Verify that the engine can build a flow with an MQTT broker config node.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mqtt_broker_config_node_builds() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "broker_1",
            "type": "mqtt-broker",
            "url": "mqtt://localhost:1883"
        }
    ]);

    let _harness = TestHarness::from_flow_json(flow);
    // Engine built successfully with mqtt-broker config node
}

// ---------------------------------------------------------------------------
// Test: Engine builds with MQTT in/out and broker nodes
// ---------------------------------------------------------------------------

/// Verify that the engine can build a complete MQTT flow with in/out and broker.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mqtt_full_flow_builds() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "1",
            "type": "mqtt in",
            "z": "100",
            "name": "mqtt-sub",
            "broker": "broker_1",
            "topic": "test/embedded",
            "qos": "0",
            "datatype": "utf8",
            "inputs": 0,
            "wires": [["99"]]
        },
        {
            "id": "2",
            "type": "mqtt out",
            "z": "100",
            "name": "mqtt-pub",
            "broker": "broker_1",
            "topic": "test/embedded",
            "qos": "0",
            "retain": false,
            "wires": []
        },
        {
            "id": "broker_1",
            "type": "mqtt-broker",
            "url": "mqtt://localhost:1883"
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
    // Engine built successfully with full MQTT flow
}

// ---------------------------------------------------------------------------
// Test: Embedded broker starts and accepts connections
// ---------------------------------------------------------------------------

/// Verify that the embedded broker starts on a random port and accepts a
/// rumqttc client connection.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn embedded_broker_accepts_connection() {
    let addr = start_embedded_broker().await;

    let mut opts = rumqttc::MqttOptions::new("test-client", addr.ip().to_string(), addr.port());
    opts.set_keep_alive(Duration::from_secs(5));
    opts.set_clean_session(true);

    let (_client, mut eventloop) = rumqttc::AsyncClient::new(opts, 10);

    // Poll for CONNACK
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut got_connack = false;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, eventloop.poll()).await {
            Ok(Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(ack)))) => {
                assert_eq!(ack.code, rumqttc::ConnectReturnCode::Success);
                got_connack = true;
                break;
            }
            Ok(Ok(_)) => continue,
            Ok(Err(e)) => panic!("MQTT connection error: {e}"),
            Err(_) => break,
        }
    }
    assert!(got_connack, "Expected successful CONNACK from embedded broker");
}

// ---------------------------------------------------------------------------
// Test: Embedded broker pub/sub with QoS 0
// ---------------------------------------------------------------------------

/// End-to-end test: publish a message and verify the subscriber receives it
/// using the embedded broker.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn embedded_broker_pubsub_qos0() {
    let addr = start_embedded_broker().await;

    // Create subscriber
    let mut sub_opts = rumqttc::MqttOptions::new("sub-qos0", addr.ip().to_string(), addr.port());
    sub_opts.set_keep_alive(Duration::from_secs(5));
    sub_opts.set_clean_session(true);
    let (sub_client, mut sub_eventloop) = rumqttc::AsyncClient::new(sub_opts, 10);

    // Wait for subscriber CONNACK
    wait_for_connack(&mut sub_eventloop).await;

    // Subscribe
    sub_client.subscribe("test/embedded/qos0", rumqttc::QoS::AtMostOnce).await.unwrap();
    wait_for_suback(&mut sub_eventloop).await;

    // Create publisher
    let mut pub_opts = rumqttc::MqttOptions::new("pub-qos0", addr.ip().to_string(), addr.port());
    pub_opts.set_keep_alive(Duration::from_secs(5));
    pub_opts.set_clean_session(true);
    let (pub_client, mut pub_eventloop) = rumqttc::AsyncClient::new(pub_opts, 10);

    wait_for_connack(&mut pub_eventloop).await;

    // Publish
    pub_client.publish("test/embedded/qos0", rumqttc::QoS::AtMostOnce, false, b"hello embedded").await.unwrap();

    // Drive publisher event loop
    drain_eventloop(&mut pub_eventloop, Duration::from_secs(2)).await;

    // Subscriber should receive the message
    let msg = wait_for_publish(&mut sub_eventloop, Duration::from_secs(5)).await;
    assert!(msg.is_some(), "Subscriber should receive message");
    let pub_msg = msg.unwrap();
    assert_eq!(pub_msg.topic, "test/embedded/qos0");
    assert_eq!(&pub_msg.payload[..], b"hello embedded");
}

// ---------------------------------------------------------------------------
// Test: Embedded broker wildcard subscription
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn embedded_broker_wildcard_subscription() {
    let addr = start_embedded_broker().await;

    // Subscriber with wildcard
    let mut sub_opts = rumqttc::MqttOptions::new("sub-wild", addr.ip().to_string(), addr.port());
    sub_opts.set_keep_alive(Duration::from_secs(5));
    sub_opts.set_clean_session(true);
    let (sub_client, mut sub_eventloop) = rumqttc::AsyncClient::new(sub_opts, 10);

    wait_for_connack(&mut sub_eventloop).await;
    sub_client.subscribe("sensors/#", rumqttc::QoS::AtMostOnce).await.unwrap();
    wait_for_suback(&mut sub_eventloop).await;

    // Publisher
    let mut pub_opts = rumqttc::MqttOptions::new("pub-wild", addr.ip().to_string(), addr.port());
    pub_opts.set_keep_alive(Duration::from_secs(5));
    pub_opts.set_clean_session(true);
    let (pub_client, mut pub_eventloop) = rumqttc::AsyncClient::new(pub_opts, 10);
    wait_for_connack(&mut pub_eventloop).await;

    // Publish to a deep topic that matches "sensors/#"
    pub_client.publish("sensors/temp/living/room1", rumqttc::QoS::AtMostOnce, false, b"23.5").await.unwrap();
    drain_eventloop(&mut pub_eventloop, Duration::from_secs(2)).await;

    let msg = wait_for_publish(&mut sub_eventloop, Duration::from_secs(5)).await;
    assert!(msg.is_some(), "Subscriber should receive wildcard-matched message");
    let pub_msg = msg.unwrap();
    assert_eq!(pub_msg.topic, "sensors/temp/living/room1");
    assert_eq!(&pub_msg.payload[..], b"23.5");
}

// ---------------------------------------------------------------------------
// Test: Embedded broker retained message
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn embedded_broker_retained_message() {
    let addr = start_embedded_broker().await;

    // Publisher sends a retained message
    let mut pub_opts = rumqttc::MqttOptions::new("pub-retain", addr.ip().to_string(), addr.port());
    pub_opts.set_keep_alive(Duration::from_secs(5));
    pub_opts.set_clean_session(true);
    let (pub_client, mut pub_eventloop) = rumqttc::AsyncClient::new(pub_opts, 10);
    wait_for_connack(&mut pub_eventloop).await;

    pub_client.publish("test/retained", rumqttc::QoS::AtMostOnce, true, b"retained msg").await.unwrap();
    drain_eventloop(&mut pub_eventloop, Duration::from_secs(2)).await;

    // Late subscriber connects and subscribes
    let mut sub_opts = rumqttc::MqttOptions::new("sub-retain", addr.ip().to_string(), addr.port());
    sub_opts.set_keep_alive(Duration::from_secs(5));
    sub_opts.set_clean_session(true);
    let (sub_client, mut sub_eventloop) = rumqttc::AsyncClient::new(sub_opts, 10);
    wait_for_connack(&mut sub_eventloop).await;
    sub_client.subscribe("test/retained", rumqttc::QoS::AtMostOnce).await.unwrap();
    wait_for_suback(&mut sub_eventloop).await;

    // Should immediately receive the retained message
    let msg = wait_for_publish(&mut sub_eventloop, Duration::from_secs(5)).await;
    assert!(msg.is_some(), "Subscriber should receive retained message");
    let pub_msg = msg.unwrap();
    assert_eq!(pub_msg.topic, "test/retained");
    assert_eq!(&pub_msg.payload[..], b"retained msg");
}

// ---------------------------------------------------------------------------
// Test: Multiple clients receive the same published message
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn embedded_broker_fanout() {
    let addr = start_embedded_broker().await;

    // Two subscribers
    let mut sub1_opts = rumqttc::MqttOptions::new("fanout-sub1", addr.ip().to_string(), addr.port());
    sub1_opts.set_keep_alive(Duration::from_secs(5));
    sub1_opts.set_clean_session(true);
    let (sub1_client, mut sub1_el) = rumqttc::AsyncClient::new(sub1_opts, 10);
    wait_for_connack(&mut sub1_el).await;
    sub1_client.subscribe("fanout/topic", rumqttc::QoS::AtMostOnce).await.unwrap();
    wait_for_suback(&mut sub1_el).await;

    let mut sub2_opts = rumqttc::MqttOptions::new("fanout-sub2", addr.ip().to_string(), addr.port());
    sub2_opts.set_keep_alive(Duration::from_secs(5));
    sub2_opts.set_clean_session(true);
    let (sub2_client, mut sub2_el) = rumqttc::AsyncClient::new(sub2_opts, 10);
    wait_for_connack(&mut sub2_el).await;
    sub2_client.subscribe("fanout/topic", rumqttc::QoS::AtMostOnce).await.unwrap();
    wait_for_suback(&mut sub2_el).await;

    // Publisher
    let mut pub_opts = rumqttc::MqttOptions::new("fanout-pub", addr.ip().to_string(), addr.port());
    pub_opts.set_keep_alive(Duration::from_secs(5));
    pub_opts.set_clean_session(true);
    let (pub_client, mut pub_el) = rumqttc::AsyncClient::new(pub_opts, 10);
    wait_for_connack(&mut pub_el).await;
    pub_client.publish("fanout/topic", rumqttc::QoS::AtMostOnce, false, b"fanout msg").await.unwrap();
    drain_eventloop(&mut pub_el, Duration::from_secs(2)).await;

    // Both should receive
    let msg1 = wait_for_publish(&mut sub1_el, Duration::from_secs(5)).await;
    let msg2 = wait_for_publish(&mut sub2_el, Duration::from_secs(5)).await;
    assert!(msg1.is_some(), "Subscriber 1 should receive");
    assert!(msg2.is_some(), "Subscriber 2 should receive");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn wait_for_connack(eventloop: &mut rumqttc::EventLoop) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            panic!("Timeout waiting for CONNACK");
        }
        match tokio::time::timeout(remaining, eventloop.poll()).await {
            Ok(Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_)))) => return,
            Ok(Ok(_)) => continue,
            Ok(Err(e)) => panic!("Event loop error: {e}"),
            Err(_) => panic!("Timeout waiting for CONNACK"),
        }
    }
}

async fn wait_for_suback(eventloop: &mut rumqttc::EventLoop) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            panic!("Timeout waiting for SUBACK");
        }
        match tokio::time::timeout(remaining, eventloop.poll()).await {
            Ok(Ok(rumqttc::Event::Incoming(rumqttc::Packet::SubAck(_)))) => return,
            Ok(Ok(_)) => continue,
            Ok(Err(e)) => panic!("Event loop error: {e}"),
            Err(_) => panic!("Timeout waiting for SUBACK"),
        }
    }
}

async fn wait_for_publish(eventloop: &mut rumqttc::EventLoop, timeout: Duration) -> Option<rumqttc::Publish> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return None;
        }
        match tokio::time::timeout(remaining, eventloop.poll()).await {
            Ok(Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(p)))) => return Some(p),
            Ok(Ok(_)) => continue,
            Ok(Err(_)) => return None,
            Err(_) => return None,
        }
    }
}

async fn drain_eventloop(eventloop: &mut rumqttc::EventLoop, timeout: Duration) {
    let _ = tokio::time::timeout(timeout, async {
        loop {
            match eventloop.poll().await {
                Ok(rumqttc::Event::Outgoing(_)) => continue,
                Ok(_) => break,
                Err(_) => break,
            }
        }
    })
    .await;
}
