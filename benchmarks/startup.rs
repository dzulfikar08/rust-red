//! Startup time benchmark for Rust-Red.
//!
//! Measures:
//! - Cold start time (process launch to ready)
//! - Flow load time (parsing and building flows)
//! - Engine start time (starting all node tasks)
//!
//! Usage:
//!   cargo bench --bench startup

use std::time::{Duration, Instant};

use rust_red_core::runtime::engine::Engine;
use rust_red_core::runtime::registry::RegistryBuilder;

mod bench_common;
use bench_common::{init_test_logger, Stats};

// Force the dummy node plugin to be linked
extern crate rust_red_nodes_dummy;

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
// Measurement
// ---------------------------------------------------------------------------

struct StartupMetrics {
    label: String,
    registry_time: Duration,
    flow_load_time: Duration,
    engine_start_time: Duration,
    total_time: Duration,
    node_count: usize,
}

fn measure_startup(label: &str, flow_json: &str, n_nodes: usize) -> StartupMetrics {
    println!("\n=== Startup: {} ({} nodes) ===", label, n_nodes);

    let iterations = 10;
    let mut registry_times = Vec::with_capacity(iterations);
    let mut load_times = Vec::with_capacity(iterations);
    let mut start_times = Vec::with_capacity(iterations);
    let mut total_times = Vec::with_capacity(iterations);

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    for _ in 0..iterations {
        let t_total = Instant::now();

        // Measure registry creation
        let t_reg = Instant::now();
        let reg = RegistryBuilder::default().build().expect("registry build failed");
        let reg_time = t_reg.elapsed();

        // Measure flow loading
        let t_load = Instant::now();
        let engine = Engine::with_json_string(&reg, flow_json.to_string(), None).expect("engine build failed");
        let load_time = t_load.elapsed();

        // Measure engine start
        let start_time_result = rt.block_on(async {
            let t_start = Instant::now();
            engine.start().await.expect("engine start failed");
            let start_dur = t_start.elapsed();

            tokio::time::sleep(Duration::from_millis(10)).await;
            engine.stop().await.expect("engine stop failed");
            start_dur
        });

        let total = t_total.elapsed();

        registry_times.push(reg_time);
        load_times.push(load_time);
        start_times.push(start_time_result);
        total_times.push(total);
    }

    let reg_stats = Stats::from_durations(&registry_times, 1);
    let load_stats = Stats::from_durations(&load_times, 1);
    let start_stats = Stats::from_durations(&start_times, 1);
    let total_stats = Stats::from_durations(&total_times, 1);

    println!(
        "  Registry:  mean={:.2}us  p50={:.2}us",
        reg_stats.mean.as_secs_f64() * 1e6,
        reg_stats.p50.as_secs_f64() * 1e6,
    );
    println!(
        "  Flow Load: mean={:.2}us  p50={:.2}us",
        load_stats.mean.as_secs_f64() * 1e6,
        load_stats.p50.as_secs_f64() * 1e6,
    );
    println!(
        "  Engine Start: mean={:.2}us  p50={:.2}us",
        start_stats.mean.as_secs_f64() * 1e6,
        start_stats.p50.as_secs_f64() * 1e6,
    );
    println!(
        "  Total:     mean={:.2}us  p50={:.2}us",
        total_stats.mean.as_secs_f64() * 1e6,
        total_stats.p50.as_secs_f64() * 1e6,
    );

    StartupMetrics {
        label: label.to_string(),
        registry_time: reg_stats.mean,
        flow_load_time: load_stats.mean,
        engine_start_time: start_stats.mean,
        total_time: total_stats.mean,
        node_count: n_nodes,
    }
}

fn main() {
    init_test_logger();

    println!("========================================");
    println!("  Rust-Red Startup Benchmark");
    println!("========================================");

    let mut all_metrics: Vec<StartupMetrics> = Vec::new();

    for &n_nodes in &[0_usize, 10, 100] {
        let label = format!("flow-{}-nodes", n_nodes);
        let flow_json = generate_chain_flow(n_nodes);
        all_metrics.push(measure_startup(&label, &flow_json, n_nodes));
    }

    // Print comparison table
    println!("\n========================================");
    println!("  Startup Summary");
    println!("========================================");
    println!(
        "{:<20} {:>5} {:>12} {:>12} {:>12} {:>12}",
        "Configuration", "Nodes", "Registry(us)", "Load(us)", "Start(us)", "Total(us)"
    );
    println!("{}", "-".repeat(65));
    for m in &all_metrics {
        println!(
            "{:<20} {:>5} {:>12.1} {:>12.1} {:>12.1} {:>12.1}",
            m.label,
            m.node_count,
            m.registry_time.as_secs_f64() * 1e6,
            m.flow_load_time.as_secs_f64() * 1e6,
            m.engine_start_time.as_secs_f64() * 1e6,
            m.total_time.as_secs_f64() * 1e6,
        );
    }

    // Node-RED comparison instructions
    println!("\n--- Node-RED Comparison ---");
    println!("To measure Node-RED's startup time for comparison:");
    println!("  1. time node-red -p 1880 -- -v   # Measure cold start");
    println!("  2. Or programmatically:");
    println!("     const t0 = Date.now();");
    println!("     const RED = require('node-red');");
    println!("     RED.init(server, {{}});");
    println!("     RED.start().then(() => console.log(Date.now() - t0, 'ms'));");

    // Output JSON results
    println!("\n--- JSON Results ---");
    let json_results: Vec<serde_json::Value> = all_metrics
        .iter()
        .map(|m| {
            serde_json::json!({
                "label": m.label,
                "node_count": m.node_count,
                "registry_us": m.registry_time.as_secs_f64() * 1e6,
                "flow_load_us": m.flow_load_time.as_secs_f64() * 1e6,
                "engine_start_us": m.engine_start_time.as_secs_f64() * 1e6,
                "total_us": m.total_time.as_secs_f64() * 1e6,
            })
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&json_results).unwrap());
}
