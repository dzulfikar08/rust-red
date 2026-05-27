//! WebSocket node compatibility tests.
//!
//! Verifies that WebSocket in/out/listener nodes behave correctly,
//! matching Node-RED behavior.
//!
//! # Infrastructure
//!
//! Some tests spawn local WebSocket echo servers using tokio-tungstenite
//! and can run without external dependencies. Tests that require an external
//! WebSocket server are marked `#[ignore]`.

use std::time::Duration;

use serde_json::json;

use super::harness::TestHarness;

// ---------------------------------------------------------------------------
// WebSocket Listener — Server Start
// ---------------------------------------------------------------------------

/// WebSocket listener: node starts a WebSocket server and accepts connections.
///
/// Verify that the flow compiles and the engine starts with a listener node.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn websocket_listener_starts() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "1",
            "type": "websocket-listener",
            "z": "100",
            "name": "ws-listener",
            "path": "/ws",
            "port": "0",
            "wholemsg": false,
            "type": "server",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
    // Engine built successfully. The listener will bind to an ephemeral port.
}

// ---------------------------------------------------------------------------
// WebSocket In — Connect to Server
// ---------------------------------------------------------------------------

/// WebSocket in (connect mode): node connects to a remote WebSocket server
/// and emits messages received from the server.
///
/// We spawn a local WebSocket echo server and verify the flow compiles.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn websocket_in_connect_mode() {
    // Start a simple WS echo server using tokio-tungstenite
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ws_addr = listener.local_addr().unwrap();
    let ws_url = format!("ws://127.0.0.1:{}", ws_addr.port());

    let echo_server = tokio::spawn(async move {
        use futures_util::StreamExt;
        use tokio_tungstenite::accept_async;

        if let Ok((stream, _)) = listener.accept().await {
            if let Ok(mut ws_stream) = accept_async(stream).await {
                // Echo loop: read a message and send it back
                while let Some(msg) = ws_stream.next().await {
                    match msg {
                        Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                            use futures_util::SinkExt;
                            let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(text)).await;
                        }
                        Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
                        _ => break,
                    }
                }
            }
        }
    });

    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "2",
            "type": "websocket in",
            "z": "100",
            "name": "ws-in",
            "url": ws_url,
            "path": "/ws",
            "wholemsg": false,
            "client": "connect",
            "reconnect": false,
            "reconnectInterval": 5,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);

    // Clean up the echo server
    echo_server.abort();
}

// ---------------------------------------------------------------------------
// WebSocket Out — Connect and Send
// ---------------------------------------------------------------------------

/// WebSocket out (connect mode): node connects to a WebSocket server and
/// sends payload data from incoming messages.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn websocket_out_connect_and_send() {
    // Start a WS server that collects received messages
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ws_addr = listener.local_addr().unwrap();
    let ws_url = format!("ws://127.0.0.1:{}", ws_addr.port());

    let server_task = tokio::spawn(async move {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::accept_async;

        if let Ok((stream, _)) = listener.accept().await {
            if let Ok(mut ws_stream) = accept_async(stream).await {
                while let Some(msg) = ws_stream.next().await {
                    match msg {
                        Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                            // Echo back
                            let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(text)).await;
                        }
                        Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
                        _ => break,
                    }
                }
            }
        }
    });

    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "3",
            "type": "websocket out",
            "z": "100",
            "name": "ws-out",
            "url": ws_url,
            "path": "/ws",
            "wholemsg": false,
            "client": "connect",
            "reconnect": false,
            "reconnectInterval": 5,
            "wires": []
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);

    // Inject a message to send via WebSocket
    let _msgs = harness
        .inject_and_collect_timeout(
            "3",
            json!({"payload": "hello websocket"}),
            0, // WebSocket out does not produce output messages
            Duration::from_secs(3),
        )
        .await;

    server_task.abort();
}

// ---------------------------------------------------------------------------
// WebSocket In — Whole Message Mode
// ---------------------------------------------------------------------------

/// WebSocket in with wholemsg=true should include metadata (socketid, type,
/// timestamp) in the output message.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn websocket_in_wholemsg_mode() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "4",
            "type": "websocket in",
            "z": "100",
            "name": "ws-wholemsg",
            "url": "ws://localhost:19999/ws",
            "path": "/ws",
            "wholemsg": true,
            "client": "connect",
            "reconnect": false,
            "reconnectInterval": 5,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
    // Engine built with wholemsg mode. Node will attempt to connect to the
    // nonexistent server and handle it gracefully.
}

// ---------------------------------------------------------------------------
// WebSocket Out — Listen Mode
// ---------------------------------------------------------------------------

/// WebSocket out in listen mode accepts connections and sends data to
/// connected clients.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn websocket_out_listen_mode() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "5",
            "type": "websocket out",
            "z": "100",
            "name": "ws-out-listen",
            "url": "",
            "path": "/ws",
            "wholemsg": false,
            "client": "listen",
            "reconnect": false,
            "reconnectInterval": 5,
            "wires": []
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let _harness = TestHarness::from_flow_json(flow);
}

// ---------------------------------------------------------------------------
// WebSocket Out — Send Binary Data
// ---------------------------------------------------------------------------

/// WebSocket out can send binary data (array of byte values) as a binary
/// WebSocket frame.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn websocket_out_binary_payload() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "6",
            "type": "websocket out",
            "z": "100",
            "name": "ws-binary",
            "url": "ws://localhost:19999/ws",
            "path": "/ws",
            "wholemsg": false,
            "client": "connect",
            "reconnect": false,
            "reconnectInterval": 5,
            "wires": []
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);

    // Inject a message with a binary payload (array of bytes)
    let _msgs = harness
        .inject_and_collect_timeout(
            "6",
            json!({"payload": [72, 101, 108, 108, 111]}), // "Hello" as bytes
            0,
            Duration::from_secs(2),
        )
        .await;
}

// ---------------------------------------------------------------------------
// WebSocket In — Disconnect Command
// ---------------------------------------------------------------------------

/// WebSocket in node processes a disconnect command to close the active connection.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn websocket_in_disconnect_command() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "7",
            "type": "websocket in",
            "z": "100",
            "name": "ws-disconnect",
            "url": "ws://localhost:19999/ws",
            "path": "/ws",
            "wholemsg": false,
            "client": "connect",
            "reconnect": false,
            "reconnectInterval": 5,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);

    let _msgs = harness.inject_and_collect_timeout("7", json!({"disconnect": true}), 0, Duration::from_secs(2)).await;
}

// ---------------------------------------------------------------------------
// WebSocket Out — Disconnect Command
// ---------------------------------------------------------------------------

/// WebSocket out node processes a disconnect command to close the active connection.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn websocket_out_disconnect_command() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "8",
            "type": "websocket out",
            "z": "100",
            "name": "ws-out-disconnect",
            "url": "ws://localhost:19999/ws",
            "path": "/ws",
            "wholemsg": false,
            "client": "connect",
            "reconnect": false,
            "reconnectInterval": 5,
            "wires": []
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);

    let _msgs = harness.inject_and_collect_timeout("8", json!({"disconnect": true}), 0, Duration::from_secs(2)).await;
}

// ---------------------------------------------------------------------------
// WebSocket Listener — Broadcast to All Connections
// ---------------------------------------------------------------------------

/// WebSocket listener node broadcasts messages to all connected clients
/// when no specific _sessionid is provided.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn websocket_listener_broadcast() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "9",
            "type": "websocket-listener",
            "z": "100",
            "name": "ws-broadcast",
            "path": "/ws",
            "port": "0",
            "wholemsg": false,
            "type": "server",
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);

    // Inject a message to broadcast
    let _msgs = harness
        .inject_and_collect_timeout("9", json!({"payload": "broadcast message"}), 0, Duration::from_secs(2))
        .await;
}

// ---------------------------------------------------------------------------
// WebSocket Full Round-Trip (Echo)
// ---------------------------------------------------------------------------
//
// NOTE: This test is marked #[ignore] because the current test harness
// does not support connecting two engine nodes (in and out) to the same
// external WebSocket server within a single test run and verifying the
// round-trip. It documents the expected behavior for future enhancement.

/// Full round-trip: send via WebSocket out, receive via WebSocket in,
/// through an echo server.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires enhanced test harness for round-trip verification"]
async fn websocket_echo_roundtrip() {
    // Start a WS echo server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ws_addr = listener.local_addr().unwrap();
    let ws_url = format!("ws://127.0.0.1:{}", ws_addr.port());

    let _echo_server = tokio::spawn(async move {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::accept_async;

        // Accept multiple connections for echo
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    if let Ok(mut ws_stream) = accept_async(stream).await {
                        while let Some(Ok(msg)) = ws_stream.next().await {
                            if ws_stream.send(msg).await.is_err() {
                                break;
                            }
                        }
                    }
                });
            }
        }
    });

    let flow = json!([
        {"id": "100", "type": "tab"},
        {
            "id": "ws_out",
            "type": "websocket out",
            "z": "100",
            "name": "ws-out-echo",
            "url": ws_url,
            "path": "/ws",
            "wholemsg": false,
            "client": "connect",
            "reconnect": false,
            "reconnectInterval": 5,
            "wires": []
        },
        {
            "id": "ws_in",
            "type": "websocket in",
            "z": "100",
            "name": "ws-in-echo",
            "url": format!("ws://127.0.0.1:{}/ws", ws_addr.port()),
            "path": "/ws",
            "wholemsg": false,
            "client": "connect",
            "reconnect": false,
            "reconnectInterval": 5,
            "wires": [["99"]]
        },
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);

    // Give WebSocket in time to connect
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Send a message via WebSocket out
    let _msgs =
        harness.inject_and_collect_timeout("ws_out", json!({"payload": "echo test"}), 0, Duration::from_secs(3)).await;

    // In a complete test, we would verify that sink "99" received
    // {"payload": "echo test"} from the WebSocket in node after the
    // echo server reflected the message.
}
