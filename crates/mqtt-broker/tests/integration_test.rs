//! Integration tests for the embedded MQTT broker.
//!
//! These tests start a real MQTT broker on an OS-assigned port and use
//! rumqttc clients to connect, subscribe, publish, and verify message delivery.

use std::sync::Arc;
use std::time::Duration;

use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use rust_red_mqtt_broker::broker::MqttBroker;
use rust_red_mqtt_broker::config::BrokerConfig;

/// Helper: start a broker on a random port and return its address.
async fn start_test_broker() -> (Arc<MqttBroker>, std::net::SocketAddr) {
    let mut config = BrokerConfig::default();
    config.bind = "127.0.0.1:0".to_string();
    config.enabled = true;
    let broker = Arc::new(MqttBroker::new(config));
    let addr = broker.clone().start_background().await.expect("broker start");
    // Give the broker a moment to start listening
    tokio::time::sleep(Duration::from_millis(50)).await;
    (broker, addr)
}

/// Helper: create a rumqttc client connected to the given address.
async fn create_client(addr: std::net::SocketAddr, client_id: &str) -> (AsyncClient, rumqttc::EventLoop) {
    let mut opts = MqttOptions::new(client_id, addr.ip().to_string(), addr.port());
    opts.set_keep_alive(Duration::from_secs(5));
    opts.set_clean_session(true);
    AsyncClient::new(opts, 10)
}

/// Wait for a specific incoming event type from the event loop.
async fn wait_for_event(
    eventloop: &mut rumqttc::EventLoop,
    timeout: Duration,
    predicate: impl Fn(&Incoming) -> bool,
) -> Option<Incoming> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return None;
        }
        match tokio::time::timeout(remaining, eventloop.poll()).await {
            Ok(Ok(Event::Incoming(incoming))) => {
                if predicate(&incoming) {
                    return Some(incoming);
                }
            }
            Ok(Ok(Event::Outgoing(_))) => continue,
            Ok(Err(e)) => {
                eprintln!("Event loop error: {e}");
                return None;
            }
            Err(_) => return None, // timeout
        }
    }
}

// ---------------------------------------------------------------------------
// Test 1: Broker starts and client connects
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn broker_starts_and_client_connects() {
    let (_broker, addr) = start_test_broker().await;

    let (_client, mut eventloop) = create_client(addr, "test-connect").await;

    let connack = wait_for_event(&mut eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    assert!(connack.is_some(), "Expected CONNACK from broker");
}

// ---------------------------------------------------------------------------
// Test 2: Subscribe and receive a published message
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subscribe_and_receive_message() {
    let (_broker, addr) = start_test_broker().await;

    // Create subscriber
    let (sub_client, mut sub_eventloop) = create_client(addr, "test-sub").await;

    // Wait for CONNACK
    let connack =
        wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
    assert!(connack.is_some());

    // Subscribe to topic
    sub_client.subscribe("test/hello", QoS::AtMostOnce).await.unwrap();

    // Wait for SUBACK
    let suback =
        wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::SubAck(_))).await;
    assert!(suback.is_some());

    // Create publisher
    let (pub_client, mut pub_eventloop) = create_client(addr, "test-pub").await;

    // Wait for publisher CONNACK
    let pub_connack =
        wait_for_event(&mut pub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
    assert!(pub_connack.is_some());

    // Publish a message
    pub_client.publish("test/hello", QoS::AtMostOnce, false, "hello world".as_bytes()).await.unwrap();

    // Drive the publisher's event loop so the message is actually sent
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match pub_eventloop.poll().await {
                Ok(Event::Outgoing(_)) => continue,
                Ok(_) => break,
                Err(_) => break,
            }
        }
    })
    .await;

    // Wait for the subscriber to receive the PUBLISH
    let publish =
        wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::Publish(_))).await;

    assert!(publish.is_some(), "Subscriber should receive a PUBLISH message");

    if let Some(Incoming::Publish(p)) = publish {
        assert_eq!(p.topic, "test/hello");
        assert_eq!(&p.payload[..], b"hello world");
    }
}

// ---------------------------------------------------------------------------
// Test 3: QoS 1 publish and subscribe
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn qos1_publish_and_subscribe() {
    let (_broker, addr) = start_test_broker().await;

    // Create subscriber
    let (sub_client, mut sub_eventloop) = create_client(addr, "test-sub-qos1").await;
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    // Subscribe with QoS 1
    sub_client.subscribe("test/qos1", QoS::AtLeastOnce).await.unwrap();
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::SubAck(_))).await;

    // Create publisher and publish with QoS 1
    let (pub_client, mut pub_eventloop) = create_client(addr, "test-pub-qos1").await;
    wait_for_event(&mut pub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    pub_client.publish("test/qos1", QoS::AtLeastOnce, false, "qos1 message".as_bytes()).await.unwrap();

    // Drive publisher event loop
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match pub_eventloop.poll().await {
                Ok(Event::Incoming(Incoming::PubAck(_))) => break,
                Ok(Event::Outgoing(_)) => continue,
                Ok(_) => continue,
                Err(_) => break,
            }
        }
    })
    .await;

    // Subscriber receives the message
    let publish =
        wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::Publish(_))).await;

    assert!(publish.is_some());
    if let Some(Incoming::Publish(p)) = publish {
        assert_eq!(p.topic, "test/qos1");
        assert_eq!(&p.payload[..], b"qos1 message");
    }
}

// ---------------------------------------------------------------------------
// Test 4: Multiple subscribers receive the same message
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multiple_subscribers() {
    let (_broker, addr) = start_test_broker().await;

    // Create two subscribers
    let (sub1_client, mut sub1_eventloop) = create_client(addr, "test-sub1-multi").await;
    wait_for_event(&mut sub1_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
    sub1_client.subscribe("test/multi", QoS::AtMostOnce).await.unwrap();
    wait_for_event(&mut sub1_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::SubAck(_))).await;

    let (sub2_client, mut sub2_eventloop) = create_client(addr, "test-sub2-multi").await;
    wait_for_event(&mut sub2_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
    sub2_client.subscribe("test/multi", QoS::AtMostOnce).await.unwrap();
    wait_for_event(&mut sub2_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::SubAck(_))).await;

    // Publisher
    let (pub_client, mut pub_eventloop) = create_client(addr, "test-pub-multi").await;
    wait_for_event(&mut pub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    pub_client.publish("test/multi", QoS::AtMostOnce, false, "broadcast".as_bytes()).await.unwrap();

    // Drive publisher
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match pub_eventloop.poll().await {
                Ok(Event::Outgoing(_)) => continue,
                Ok(_) => break,
                Err(_) => break,
            }
        }
    })
    .await;

    // Both subscribers should receive the message
    let msg1 =
        wait_for_event(&mut sub1_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::Publish(_))).await;
    let msg2 =
        wait_for_event(&mut sub2_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::Publish(_))).await;

    assert!(msg1.is_some(), "Subscriber 1 should receive the message");
    assert!(msg2.is_some(), "Subscriber 2 should receive the message");
}

// ---------------------------------------------------------------------------
// Test 5: Wildcard topic matching (#)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wildcard_topic_matching() {
    let (_broker, addr) = start_test_broker().await;

    // Subscribe to "sensors/#" (multi-level wildcard)
    let (sub_client, mut sub_eventloop) = create_client(addr, "test-wildcard").await;
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
    sub_client.subscribe("sensors/#", QoS::AtMostOnce).await.unwrap();
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::SubAck(_))).await;

    // Publisher
    let (pub_client, mut pub_eventloop) = create_client(addr, "test-pub-wild").await;
    wait_for_event(&mut pub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    // Publish to "sensors/temperature/living" - should match "sensors/#"
    pub_client.publish("sensors/temperature/living", QoS::AtMostOnce, false, "22.5".as_bytes()).await.unwrap();

    // Drive publisher
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match pub_eventloop.poll().await {
                Ok(Event::Outgoing(_)) => continue,
                Ok(_) => break,
                Err(_) => break,
            }
        }
    })
    .await;

    let msg = wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::Publish(_))).await;

    assert!(msg.is_some(), "Subscriber should receive message matching wildcard");
    if let Some(Incoming::Publish(p)) = msg {
        assert_eq!(p.topic, "sensors/temperature/living");
        assert_eq!(&p.payload[..], b"22.5");
    }
}

// ---------------------------------------------------------------------------
// Test 6: Retained messages
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retained_message_delivery() {
    let (_broker, addr) = start_test_broker().await;

    // Publisher sends a retained message
    let (pub_client, mut pub_eventloop) = create_client(addr, "test-pub-retain").await;
    wait_for_event(&mut pub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    pub_client.publish("test/retained", QoS::AtMostOnce, true, "retained payload".as_bytes()).await.unwrap();

    // Drive publisher
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match pub_eventloop.poll().await {
                Ok(Event::Outgoing(_)) => continue,
                Ok(_) => break,
                Err(_) => break,
            }
        }
    })
    .await;

    // New subscriber connects AFTER the retained message was published
    let (sub_client, mut sub_eventloop) = create_client(addr, "test-sub-retain").await;
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    sub_client.subscribe("test/retained", QoS::AtMostOnce).await.unwrap();
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::SubAck(_))).await;

    // The subscriber should immediately receive the retained message
    let msg = wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::Publish(_))).await;

    assert!(msg.is_some(), "Subscriber should receive retained message");
    if let Some(Incoming::Publish(p)) = msg {
        assert_eq!(p.topic, "test/retained");
        assert_eq!(&p.payload[..], b"retained payload");
        assert!(p.retain, "Retained flag should be set");
    }
}

// ---------------------------------------------------------------------------
// Test 7: Unsubscribe stops message delivery
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsubscribe_stops_delivery() {
    let (_broker, addr) = start_test_broker().await;

    let (sub_client, mut sub_eventloop) = create_client(addr, "test-unsub").await;
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    sub_client.subscribe("test/unsub", QoS::AtMostOnce).await.unwrap();
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::SubAck(_))).await;

    // Unsubscribe
    sub_client.unsubscribe("test/unsub").await.unwrap();
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::UnsubAck(_))).await;

    // Publisher sends a message
    let (pub_client, mut pub_eventloop) = create_client(addr, "test-pub-unsub").await;
    wait_for_event(&mut pub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    pub_client.publish("test/unsub", QoS::AtMostOnce, false, "after unsub".as_bytes()).await.unwrap();

    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match pub_eventloop.poll().await {
                Ok(Event::Outgoing(_)) => continue,
                Ok(_) => break,
                Err(_) => break,
            }
        }
    })
    .await;

    // Wait a bit and check that subscriber does NOT receive the message
    let msg =
        wait_for_event(&mut sub_eventloop, Duration::from_millis(500), |ev| matches!(ev, Incoming::Publish(_))).await;

    assert!(msg.is_none(), "Subscriber should NOT receive messages after unsubscribe");
}

// ---------------------------------------------------------------------------
// Test 8: Multiple topics and selective matching
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multiple_topics_selective_matching() {
    let (_broker, addr) = start_test_broker().await;

    let (sub_client, mut sub_eventloop) = create_client(addr, "test-selective").await;
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    // Subscribe only to "test/topicA"
    sub_client.subscribe("test/topicA", QoS::AtMostOnce).await.unwrap();
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::SubAck(_))).await;

    // Publisher
    let (pub_client, mut pub_eventloop) = create_client(addr, "test-pub-sel").await;
    wait_for_event(&mut pub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    // Publish to topicB (subscriber should NOT receive)
    pub_client.publish("test/topicB", QoS::AtMostOnce, false, "wrong topic".as_bytes()).await.unwrap();

    // Drive publisher
    let _ = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            match pub_eventloop.poll().await {
                Ok(Event::Outgoing(_)) => continue,
                Ok(_) => break,
                Err(_) => break,
            }
        }
    })
    .await;

    // Check subscriber does NOT receive topicB
    let no_msg =
        wait_for_event(&mut sub_eventloop, Duration::from_millis(500), |ev| matches!(ev, Incoming::Publish(_))).await;
    assert!(no_msg.is_none(), "Should not receive message from unsubscribed topic");

    // Publish to topicA (subscriber SHOULD receive)
    pub_client.publish("test/topicA", QoS::AtMostOnce, false, "correct topic".as_bytes()).await.unwrap();

    let _ = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            match pub_eventloop.poll().await {
                Ok(Event::Outgoing(_)) => continue,
                Ok(_) => break,
                Err(_) => break,
            }
        }
    })
    .await;

    let msg = wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::Publish(_))).await;
    assert!(msg.is_some(), "Should receive message from subscribed topic");
}

// ---------------------------------------------------------------------------
// Test 9: Single-level wildcard (+) matching
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn single_level_wildcard_matching() {
    let (_broker, addr) = start_test_broker().await;

    let (sub_client, mut sub_eventloop) = create_client(addr, "test-plus-wild").await;
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    sub_client.subscribe("device/+/status", QoS::AtMostOnce).await.unwrap();
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::SubAck(_))).await;

    let (pub_client, mut pub_eventloop) = create_client(addr, "test-pub-plus").await;
    wait_for_event(&mut pub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    // Should match: device/sensor1/status
    pub_client.publish("device/sensor1/status", QoS::AtMostOnce, false, "online".as_bytes()).await.unwrap();

    drive_publisher(&mut pub_eventloop).await;

    let msg = wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::Publish(_))).await;
    assert!(msg.is_some(), "Should match single-level wildcard");
    if let Some(Incoming::Publish(p)) = msg {
        assert_eq!(p.topic, "device/sensor1/status");
        assert_eq!(&p.payload[..], b"online");
    }
}

// ---------------------------------------------------------------------------
// Test 10: Retained message replaced by new value
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retained_message_replaced() {
    let (_broker, addr) = start_test_broker().await;

    // Publish retained v1
    let (pub1, mut pub1_ev) = create_client(addr, "pub-retain-v1").await;
    wait_for_event(&mut pub1_ev, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
    pub1.publish("test/replace", QoS::AtMostOnce, true, "v1".as_bytes()).await.unwrap();
    drive_publisher(&mut pub1_ev).await;

    // Publish retained v2 (replaces v1)
    let (pub2, mut pub2_ev) = create_client(addr, "pub-retain-v2").await;
    wait_for_event(&mut pub2_ev, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
    pub2.publish("test/replace", QoS::AtMostOnce, true, "v2".as_bytes()).await.unwrap();
    drive_publisher(&mut pub2_ev).await;

    // New subscriber should get v2
    let (sub, mut sub_ev) = create_client(addr, "sub-retain-replace").await;
    wait_for_event(&mut sub_ev, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
    sub.subscribe("test/replace", QoS::AtMostOnce).await.unwrap();
    wait_for_event(&mut sub_ev, Duration::from_secs(5), |ev| matches!(ev, Incoming::SubAck(_))).await;

    let msg = wait_for_event(&mut sub_ev, Duration::from_secs(5), |ev| matches!(ev, Incoming::Publish(_))).await;
    assert!(msg.is_some());
    if let Some(Incoming::Publish(p)) = msg {
        assert_eq!(&p.payload[..], b"v2");
    }
}

// ---------------------------------------------------------------------------
// Test 11: Retained message cleared by empty payload
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retained_message_cleared_by_empty_payload() {
    let (_broker, addr) = start_test_broker().await;

    // Set retained
    let (pub1, mut pub1_ev) = create_client(addr, "pub-retain-set").await;
    wait_for_event(&mut pub1_ev, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
    pub1.publish("test/clear", QoS::AtMostOnce, true, "data".as_bytes()).await.unwrap();
    drive_publisher(&mut pub1_ev).await;

    // Clear retained with empty payload
    let (pub2, mut pub2_ev) = create_client(addr, "pub-retain-clear").await;
    wait_for_event(&mut pub2_ev, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
    pub2.publish("test/clear", QoS::AtMostOnce, true, "".as_bytes()).await.unwrap();
    drive_publisher(&mut pub2_ev).await;

    // New subscriber should NOT get retained message
    let (sub, mut sub_ev) = create_client(addr, "sub-retain-clear").await;
    wait_for_event(&mut sub_ev, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
    sub.subscribe("test/clear", QoS::AtMostOnce).await.unwrap();
    wait_for_event(&mut sub_ev, Duration::from_secs(5), |ev| matches!(ev, Incoming::SubAck(_))).await;

    let msg = wait_for_event(&mut sub_ev, Duration::from_millis(500), |ev| matches!(ev, Incoming::Publish(_))).await;
    assert!(msg.is_none(), "Retained message should be cleared");
}

// ---------------------------------------------------------------------------
// Test 12: Authentication with username/password
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn authentication_rejects_wrong_credentials() {
    let mut config = BrokerConfig::default();
    config.bind = "127.0.0.1:0".to_string();
    config.enabled = true;
    config.auth =
        rust_red_mqtt_broker::config::AuthConfig { username: Some("admin".into()), password: Some("secret".into()) };
    let broker = Arc::new(MqttBroker::new(config));
    let addr = broker.clone().start_background().await.expect("broker start");
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Client with wrong credentials
    let mut opts = MqttOptions::new("bad-auth", addr.ip().to_string(), addr.port());
    opts.set_keep_alive(Duration::from_secs(5));
    opts.set_credentials("admin", "wrong");
    let (_client, mut eventloop) = AsyncClient::new(opts, 10);

    // The broker should reject — rumqttc may report ConnAck with failure or error
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(ack))) => {
                    return ack.code != rumqttc::ConnectReturnCode::Success;
                }
                Ok(Event::Outgoing(_)) => continue,
                Ok(_) => continue,
                Err(_) => return true, // connection error = rejected
            }
        }
    })
    .await;

    assert!(result.is_ok_and(|rejected| rejected), "Broker should reject wrong credentials");
}

// ---------------------------------------------------------------------------
// Test 13: Large payload delivery
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn large_payload_delivery() {
    let mut config = BrokerConfig::default();
    config.bind = "127.0.0.1:0".to_string();
    config.enabled = true;
    config.max_packet_size = 200_000;
    let broker = Arc::new(MqttBroker::new(config));
    let addr = broker.clone().start_background().await.expect("broker start");
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut sub_opts = MqttOptions::new("test-sub-large", addr.ip().to_string(), addr.port());
    sub_opts.set_keep_alive(Duration::from_secs(5));
    sub_opts.set_clean_session(true);
    sub_opts.set_max_packet_size(200_000, 200_000);
    let (sub_client, mut sub_eventloop) = AsyncClient::new(sub_opts, 10);
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
    sub_client.subscribe("test/large", QoS::AtMostOnce).await.unwrap();
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::SubAck(_))).await;

    // 100KB payload
    let large_payload: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();

    let mut pub_opts = MqttOptions::new("test-pub-large", addr.ip().to_string(), addr.port());
    pub_opts.set_keep_alive(Duration::from_secs(5));
    pub_opts.set_clean_session(true);
    pub_opts.set_max_packet_size(200_000, 200_000);
    let (pub_client, mut pub_eventloop) = AsyncClient::new(pub_opts, 10);
    wait_for_event(&mut pub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    pub_client.publish("test/large", QoS::AtMostOnce, false, large_payload.clone()).await.unwrap();
    drive_publisher(&mut pub_eventloop).await;

    let msg = wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::Publish(_))).await;

    assert!(msg.is_some(), "Should receive large payload");
    if let Some(Incoming::Publish(p)) = msg {
        assert_eq!(p.payload.len(), 100_000);
        assert_eq!(&p.payload[..], &large_payload[..]);
    }
}

// ---------------------------------------------------------------------------
// Test 14: Concurrent publishers, single subscriber
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_publishers() {
    let (_broker, addr) = start_test_broker().await;

    // Single subscriber
    let (sub_client, mut sub_eventloop) = create_client(addr, "test-sub-conc").await;
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
    sub_client.subscribe("conc/#", QoS::AtMostOnce).await.unwrap();
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::SubAck(_))).await;

    // Spawn 5 publishers concurrently
    let mut handles = Vec::new();
    for i in 0..5 {
        let addr_clone = addr;
        handles.push(tokio::spawn(async move {
            let (client, mut ev) = create_client(addr_clone, &format!("pub-conc-{i}")).await;
            wait_for_event(&mut ev, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
            client
                .publish(&format!("conc/from/{i}"), QoS::AtMostOnce, false, format!("msg-{i}").as_bytes())
                .await
                .unwrap();
            drive_publisher(&mut ev).await;
        }));
    }

    for h in handles {
        let _ = h.await;
    }

    // Subscriber should receive all 5 messages
    let mut received = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while received < 5 {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, sub_eventloop.poll()).await {
            Ok(Ok(Event::Incoming(Incoming::Publish(_)))) => received += 1,
            Ok(Ok(Event::Outgoing(_))) => continue,
            _ => continue,
        }
    }
    assert_eq!(received, 5, "Subscriber should receive all 5 messages from concurrent publishers");
}

// ---------------------------------------------------------------------------
// Test 15: Root-level wildcard (#) matches everything
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn root_wildcard_matches_all() {
    let (_broker, addr) = start_test_broker().await;

    let (sub_client, mut sub_eventloop) = create_client(addr, "test-root-hash").await;
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;
    sub_client.subscribe("#", QoS::AtMostOnce).await.unwrap();
    wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::SubAck(_))).await;

    let (pub_client, mut pub_eventloop) = create_client(addr, "test-pub-root").await;
    wait_for_event(&mut pub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    pub_client.publish("any/deep/topic/here", QoS::AtMostOnce, false, "matched".as_bytes()).await.unwrap();
    drive_publisher(&mut pub_eventloop).await;

    let msg = wait_for_event(&mut sub_eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::Publish(_))).await;
    assert!(msg.is_some(), "Root # should match any topic");
    if let Some(Incoming::Publish(p)) = msg {
        assert_eq!(p.topic, "any/deep/topic/here");
    }
}

// ---------------------------------------------------------------------------
// Test 16: Ping/pong keeps connection alive
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ping_response() {
    let (_broker, addr) = start_test_broker().await;

    let mut opts = MqttOptions::new("test-ping", addr.ip().to_string(), addr.port());
    opts.set_keep_alive(Duration::from_secs(1));
    opts.set_clean_session(true);
    let (_client, mut eventloop) = AsyncClient::new(opts, 10);

    // Wait for CONNACK
    wait_for_event(&mut eventloop, Duration::from_secs(5), |ev| matches!(ev, Incoming::ConnAck(_))).await;

    // With 1s keep-alive, rumqttc will send PingReq within ~1.5s
    let alive = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::PingResp)) => return true,
                Ok(Event::Outgoing(rumqttc::Outgoing::PingReq)) => continue,
                Ok(_) => continue,
                Err(_) => return false,
            }
        }
    })
    .await;

    assert!(alive.is_ok_and(|ok| ok), "Should receive PingResp from broker");
}

// ---------------------------------------------------------------------------
// Helper: drain the publisher event loop
// ---------------------------------------------------------------------------

async fn drive_publisher(eventloop: &mut rumqttc::EventLoop) {
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match eventloop.poll().await {
                Ok(Event::Outgoing(_)) => continue,
                Ok(_) => break,
                Err(_) => break,
            }
        }
    })
    .await;
}
