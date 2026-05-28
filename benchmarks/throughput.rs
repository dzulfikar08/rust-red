//! Message throughput benchmark for Rust-Red.
//!
//! Measures messages processed per second through various flow topologies:
//! - Simple passthrough (inject -> debug)
//! - 10-node chain
//! - 100-node chain
//! - Fan-out (inject -> 10 parallel debug nodes)
//!
//! Strategy: Messages are injected directly into the first node in the chain
//! using `engine.inject_msg()`. The debug nodes at the end of each path
//! publish to the engine's debug_channel (broadcast). We subscribe to the
//! debug channel and count received messages to measure throughput.
//!
//! Usage:
//!   cargo bench --bench throughput

use std::collections::BTreeMap;
use std::str::FromStr;
use std::time::{Duration, Instant};

use tokio_util::sync::CancellationToken;

use rust_red_core::runtime::engine::Engine;
use rust_red_core::runtime::model::wellknown::MSG_ID_PROPERTY;
use rust_red_core::runtime::model::*;
use rust_red_core::runtime::registry::RegistryBuilder;

mod bench_common;
use bench_common::{init_test_logger, Stats};

// Force the dummy node plugin to be linked (inventory::submit! items need this)
extern crate rust_red_nodes_dummy;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_engine(json: &str) -> Engine {
    let reg = RegistryBuilder::default().build().expect("registry build failed");
    Engine::with_json_string(&reg, json.to_string(), None).expect("engine build failed")
}

fn parse_eid(hex: &str) -> ElementId {
    ElementId::from_str(hex).expect("invalid element id")
}

/// Create a simple MsgHandle with a string payload.
fn make_msg_handle(payload: &str) -> MsgHandle {
    let mut body = BTreeMap::new();
    body.insert(MSG_ID_PROPERTY.to_string(), Msg::generate_id_variant());
    body.insert("payload".to_string(), Variant::String(payload.to_string()));
    MsgHandle::with_properties(body)
}

// ---------------------------------------------------------------------------
// Flow JSON generators
// ---------------------------------------------------------------------------

/// Generate a chain flow: inject -> N dummy nodes -> debug
/// The inject node has `once: false` and is not used for actual injection.
/// Messages are injected directly into the first dummy (or debug) node.
fn generate_chain_flow(n_nodes: usize) -> String {
    use serde_json::json;

    let flow_id = format!("{:016x}", 0xf100000000000001u64);
    let inject_id = format!("{:016x}", 0xb100000000000001u64);
    let debug_id = format!("{:016x}", 0xc100000000000001u64);

    let mut nodes = vec![
        json!({
            "id": flow_id,
            "label": format!("Bench Chain {}", n_nodes),
            "type": "tab"
        }),
        json!({
            "id": inject_id,
            "name": "bench-inject",
            "type": "inject",
            "z": flow_id,
            "once": false,
            "onceDelay": 0,
            "payload": "",
            "payloadType": "str",
            "repeat": "",
            "crontab": "",
            "props": [{"p": "payload"}],
            "topic": "",
            "wires": [[if n_nodes > 0 {
                format!("{:016x}", 0xd100000000000001u64)
            } else {
                debug_id.clone()
            }]]
        }),
    ];

    for i in 0..n_nodes {
        let node_id = format!("{:016x}", 0xd100000000000001u64 + i as u64);
        let next_id =
            if i < n_nodes - 1 { format!("{:016x}", 0xd100000000000001u64 + i as u64 + 1) } else { debug_id.clone() };

        nodes.push(json!({
            "id": node_id,
            "name": format!("pass-{}", i),
            "type": "dummy",
            "z": flow_id,
            "wires": [[next_id]]
        }));
    }

    nodes.push(json!({
        "id": debug_id,
        "name": "bench-debug",
        "type": "debug",
        "z": flow_id,
        "active": true,
        "console": false,
        "tosidebar": true,
        "tostatus": false,
        "complete": "payload",
        "targetType": "msg",
        "statusType": "auto",
        "statusVal": "",
        "wires": []
    }));

    serde_json::to_string(&nodes).unwrap()
}

/// Generate a fan-out flow: inject -> hub dummy -> N parallel debug nodes
/// A dummy "hub" node is used as the injection point because inject nodes
/// don't accept external messages -- they only produce them.
fn generate_fanout_flow(n_paths: usize) -> String {
    use serde_json::json;

    let flow_id = format!("{:016x}", 0xf200000000000001u64);
    let inject_id = format!("{:016x}", 0xb200000000000001u64);
    let hub_id = format!("{:016x}", 0xd200000000000001u64);

    let debug_ids: Vec<String> = (0..n_paths).map(|i| format!("{:016x}", 0xc200000000000001u64 + i as u64)).collect();

    let mut nodes = vec![
        json!({
            "id": flow_id,
            "label": format!("Bench Fan-Out {}", n_paths),
            "type": "tab"
        }),
        // Dummy hub node: fans out one input to N debug outputs
        json!({
            "id": hub_id,
            "name": "bench-hub",
            "type": "dummy",
            "z": flow_id,
            "wires": [debug_ids]
        }),
    ];

    for (i, debug_id) in debug_ids.iter().enumerate() {
        nodes.push(json!({
            "id": debug_id,
            "name": format!("bench-debug-{}", i),
            "type": "debug",
            "z": flow_id,
            "active": true,
            "console": false,
            "tosidebar": true,
            "tostatus": false,
            "complete": "payload",
            "targetType": "msg",
            "statusType": "auto",
            "statusVal": "",
            "wires": []
        }));
    }

    nodes.push(json!({
        "id": inject_id,
        "name": "bench-inject",
        "type": "inject",
        "z": flow_id,
        "once": false,
        "onceDelay": 0,
        "payload": "",
        "payloadType": "str",
        "repeat": "",
        "crontab": "",
        "props": [{"p": "payload"}],
        "topic": "",
        "wires": [[hub_id]]
    }));

    serde_json::to_string(&nodes).unwrap()
}

// ---------------------------------------------------------------------------
// Benchmark runner
// ---------------------------------------------------------------------------

fn benchmark_flow_throughput(flow_json: &str, label: &str, msg_count: usize) -> Option<Stats> {
    println!("\n=== Throughput: {} ({} msgs) ===", label, msg_count);

    let engine = build_engine(flow_json);
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    let result = rt.block_on(async {
        engine.start().await.expect("engine start failed");

        // Small delay to let all nodes start their run loops
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Determine the first node to inject into
        let inject_target_id = if label.starts_with("fan-out") {
            // For fan-out, inject into the hub dummy node
            parse_eid("d200000000000001")
        } else if label == "passthrough" {
            // No intermediate nodes, inject directly into debug node
            parse_eid("c100000000000001")
        } else {
            // Inject into the first dummy node in the chain
            parse_eid("d100000000000001")
        };

        // For fan-out, each injected message fans out to N debug nodes,
        // so we expect msg_count * fan_out_factor debug messages.
        let fan_out_factor = if label.starts_with("fan-out") { 10 } else { 1 };
        let expected_msgs = msg_count * fan_out_factor;

        let iterations = 5;
        let mut durations = Vec::with_capacity(iterations);

        for iter in 0..iterations {
            // Subscribe to the debug channel BEFORE injecting
            let mut debug_rx = engine.debug_channel().subscribe();

            let cancel = CancellationToken::new();
            let cancel_clone = cancel.clone();
            let engine_clone = engine.clone();
            let target_id = inject_target_id;
            let count = msg_count;

            // Spawn the inject pump
            let inject_handle = tokio::spawn(async move {
                for _ in 0..count {
                    let msg = make_msg_handle("bench");
                    if engine_clone.inject_msg(&target_id, msg, cancel_clone.clone()).await.is_err() {
                        break;
                    }
                }
            });

            // Measure time from inject start to all messages received via debug channel
            let start = Instant::now();
            let mut received = 0;

            let recv_result = tokio::time::timeout(Duration::from_secs(30), async {
                while received < expected_msgs {
                    tokio::select! {
                        msg_result = debug_rx.recv() => {
                            match msg_result {
                                Ok(_) => received += 1,
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                    received += n as usize;
                                }
                                Err(_) => break,
                            }
                        }
                        _ = cancel.cancelled() => break,
                    }
                }
            })
            .await;

            let elapsed = start.elapsed();
            cancel.cancel();

            inject_handle.await.ok();

            if recv_result.is_err() {
                eprintln!("  Iteration {}: TIMEOUT after {:?}", iter, elapsed);
            }

            if received < expected_msgs {
                eprintln!("  Iteration {}: received {}/{} messages in {:?}", iter, received, expected_msgs, elapsed);
            }

            durations.push(elapsed);

            // Small pause between iterations
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        engine.stop().await.expect("engine stop failed");
        durations
    });

    let stats = Stats::from_durations(&result, msg_count);
    stats.print_summary(label);
    Some(stats)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    init_test_logger();

    println!("========================================");
    println!("  Rust-Red Throughput Benchmark Suite");
    println!("========================================");

    let msg_count = 1000;

    let mut all_results: Vec<(&str, Stats)> = Vec::new();

    // Chain benchmarks
    for &n_nodes in &[0_usize, 10, 100] {
        let flow_json = generate_chain_flow(n_nodes);
        let label = if n_nodes == 0 { "passthrough" } else { Box::leak(format!("chain-{}", n_nodes).into_boxed_str()) };
        if let Some(stats) = benchmark_flow_throughput(&flow_json, label, msg_count) {
            all_results.push((label, stats));
        }
    }

    // Fan-out benchmark
    {
        let flow_json = generate_fanout_flow(10);
        let label = "fan-out-10";
        if let Some(stats) = benchmark_flow_throughput(&flow_json, label, msg_count) {
            all_results.push((label, stats));
        }
    }

    // Print comparison table
    println!("\n========================================");
    println!("  Throughput Summary");
    println!("========================================");
    println!(
        "{:<15} {:>12} {:>12} {:>12} {:>12} {:>12}",
        "Benchmark", "msgs/sec", "mean (us)", "p50 (us)", "p99 (us)", "max (us)"
    );
    println!("{}", "-".repeat(77));
    for (label, stats) in &all_results {
        println!(
            "{:<15} {:>12.0} {:>12.1} {:>12.1} {:>12.1} {:>12.1}",
            label,
            stats.throughput_per_sec,
            stats.mean.as_secs_f64() * 1e6,
            stats.p50.as_secs_f64() * 1e6,
            stats.p99.as_secs_f64() * 1e6,
            stats.max.as_secs_f64() * 1e6,
        );
    }

    // Output JSON for machine-readable results
    println!("\n--- JSON Results ---");
    let json_results: Vec<serde_json::Value> = all_results
        .iter()
        .map(|(label, stats)| {
            serde_json::json!({
                "benchmark": label,
                "messages": msg_count,
                "throughput_per_sec": stats.throughput_per_sec,
                "mean_us": stats.mean.as_secs_f64() * 1e6,
                "p50_us": stats.p50.as_secs_f64() * 1e6,
                "p99_us": stats.p99.as_secs_f64() * 1e6,
                "max_us": stats.max.as_secs_f64() * 1e6,
                "min_us": stats.min.as_secs_f64() * 1e6,
            })
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&json_results).unwrap());
}
