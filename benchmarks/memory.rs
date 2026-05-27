//! Memory footprint benchmark for Rust-Red.
//!
//! Measures RSS (Resident Set Size) with various flow sizes:
//! - Empty engine (no flows)
//! - 10-node flow
//! - 100-node flow
//!
//! Usage:
//!   cargo bench --bench memory

use std::process::Command;
use std::time::Duration;

use rust_red_core::runtime::engine::Engine;
use rust_red_core::runtime::registry::RegistryBuilder;

mod bench_common;
use bench_common::init_test_logger;

// Force the dummy node plugin to be linked
extern crate rust_red_nodes_dummy;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_engine(json: &str) -> Engine {
    let reg = RegistryBuilder::default().build().expect("registry build failed");
    Engine::with_json_string(&reg, json.to_string(), None).expect("engine build failed")
}

/// Get current process RSS in bytes.
/// Works on macOS and Linux.
fn get_rss_bytes() -> u64 {
    // Try /proc/self/status first (Linux)
    if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
        for line in content.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return kb * 1024;
                    }
                }
            }
        }
    }

    // Fallback: use `ps` command (macOS/Linux)
    let pid = std::process::id();
    let output = Command::new("ps").args(["-o", "rss=", "-p", &pid.to_string()]).output().expect("failed to run ps");

    let rss_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    rss_str.parse::<u64>().unwrap_or(0) * 1024 // ps reports in KB
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

// ---------------------------------------------------------------------------
// Flow JSON generators
// ---------------------------------------------------------------------------

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
            "name": format!("node-{}", i),
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
        "active": false,
        "console": false,
        "tosidebar": false,
        "tostatus": false,
        "complete": "payload",
        "targetType": "msg",
        "statusType": "auto",
        "statusVal": "",
        "wires": []
    }));

    serde_json::to_string(&nodes).unwrap()
}

// ---------------------------------------------------------------------------
// Benchmark
// ---------------------------------------------------------------------------

struct MemorySnapshot {
    label: String,
    rss_bytes: u64,
    rss_delta_bytes: u64,
}

fn measure_memory(label: &str, flow_json: Option<&str>) -> MemorySnapshot {
    let rss_before = get_rss_bytes();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let result = rt.block_on(async {
        if let Some(json) = flow_json {
            let engine = build_engine(json);
            engine.start().await.expect("engine start failed");
            // Let the engine settle
            tokio::time::sleep(Duration::from_millis(500)).await;
            let rss_after = get_rss_bytes();
            engine.stop().await.expect("engine stop failed");
            rss_after
        } else {
            // Just the registry, no engine
            let _reg = RegistryBuilder::default().build().expect("registry build failed");
            tokio::time::sleep(Duration::from_millis(100)).await;
            get_rss_bytes()
        }
    });

    MemorySnapshot { label: label.to_string(), rss_bytes: result, rss_delta_bytes: result.saturating_sub(rss_before) }
}

fn main() {
    init_test_logger();

    println!("========================================");
    println!("  Rust-Red Memory Benchmark");
    println!("========================================\n");

    let mut snapshots: Vec<MemorySnapshot> = Vec::new();

    // Measure baseline (registry only)
    println!("Measuring baseline (registry only)...");
    snapshots.push(measure_memory("baseline", None));

    // Measure with various flow sizes
    for &n_nodes in &[0_usize, 10, 100] {
        let label = format!("flow-{}-nodes", n_nodes);
        println!("Measuring {}...", label);
        let flow_json = generate_chain_flow(n_nodes);
        snapshots.push(measure_memory(&label, Some(&flow_json)));
    }

    // Print results table
    println!("\n========================================");
    println!("  Memory Summary");
    println!("========================================");
    println!("{:<20} {:>15} {:>15}", "Configuration", "RSS", "RSS Delta");
    println!("{}", "-".repeat(52));
    for snap in &snapshots {
        println!("{:<20} {:>15} {:>15}", snap.label, format_bytes(snap.rss_bytes), format_bytes(snap.rss_delta_bytes),);
    }

    // Node-RED comparison instructions
    println!("\n--- Node-RED Comparison ---");
    println!("To measure Node-RED's memory for comparison:");
    println!("  1. Start Node-RED: node-red -p 1880");
    println!("  2. Import a flow with N nodes");
    println!("  3. Measure RSS: ps -o rss= -p $(pgrep -f 'node-red')");
    println!("  4. Or use: node -e 'process.memoryUsage().rss / 1024 / 1024 + \" MB\"'");

    // Output JSON results
    println!("\n--- JSON Results ---");
    let json_results: Vec<serde_json::Value> = snapshots
        .iter()
        .map(|snap| {
            serde_json::json!({
                "label": snap.label,
                "rss_bytes": snap.rss_bytes,
                "rss_human": format_bytes(snap.rss_bytes),
                "rss_delta_bytes": snap.rss_delta_bytes,
                "rss_delta_human": format_bytes(snap.rss_delta_bytes),
            })
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&json_results).unwrap());
}
