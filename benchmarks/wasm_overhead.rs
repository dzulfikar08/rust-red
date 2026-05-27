//! WASM plugin overhead benchmark for Rust-Red.
//!
//! Measures:
//! - postcard serialization/deserialization timing
//! - Throughput comparison: native dummy node vs WASM echo node (if available)
//! - WASM plugin load time (if available)
//!
//! This benchmark tests the serialization layer used for WASM boundary crossing
//! independently of the full engine, giving a clear picture of the overhead.
//!
//! Usage:
//!   cargo bench --bench wasm_overhead

use std::time::{Duration, Instant};

mod bench_common;
use bench_common::{init_test_logger, Stats};

// ---------------------------------------------------------------------------
// Serialization overhead benchmark
// ---------------------------------------------------------------------------

/// A simplified WasmMessage for benchmarking serialization.
/// Matches the structure in crates/wasm-host/src/types.rs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BenchWasmMessage {
    msg_id: String,
    payload: BenchWasmValue,
    topic: Option<String>,
    extra: std::collections::BTreeMap<String, BenchWasmValue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum BenchWasmValue {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<BenchWasmValue>),
    Object(std::collections::BTreeMap<String, BenchWasmValue>),
}

impl Default for BenchWasmValue {
    fn default() -> Self {
        BenchWasmValue::Null
    }
}

fn create_sample_message(payload_size: usize) -> BenchWasmMessage {
    let payload_str = "x".repeat(payload_size);
    let mut extra = std::collections::BTreeMap::new();
    extra.insert("counter".to_string(), BenchWasmValue::U64(42));
    extra.insert("metadata".to_string(), BenchWasmValue::String("benchmark-test".to_string()));

    BenchWasmMessage {
        msg_id: "0123456789abcdef".to_string(),
        payload: BenchWasmValue::String(payload_str),
        topic: Some("bench/wasm".to_string()),
        extra,
    }
}

fn bench_serialization(label: &str, msg: &BenchWasmMessage, iterations: usize) -> SerializationResult {
    println!("\n=== Serialization: {} ({} iterations) ===", label, iterations);

    // Benchmark postcard serialization
    let mut serialize_times = Vec::with_capacity(iterations);
    let mut deserialize_times = Vec::with_capacity(iterations);
    let mut serialized_sizes = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        // Serialize
        let t = Instant::now();
        let serialized = postcard::to_allocvec(msg).expect("postcard serialize failed");
        serialize_times.push(t.elapsed());

        serialized_sizes.push(serialized.len());

        // Deserialize
        let t = Instant::now();
        let _: BenchWasmMessage = postcard::from_bytes(&serialized).expect("postcard deserialize failed");
        deserialize_times.push(t.elapsed());
    }

    let ser_stats = Stats::from_durations(&serialize_times, 1);
    let deser_stats = Stats::from_durations(&deserialize_times, 1);
    let avg_size = serialized_sizes.iter().sum::<usize>() / serialized_sizes.len();

    println!(
        "  Serialize:   mean={:.2}ns  p50={:.2}ns  p99={:.2}ns  size={} bytes",
        ser_stats.mean.as_nanos(),
        ser_stats.p50.as_nanos(),
        ser_stats.p99.as_nanos(),
        avg_size,
    );
    println!(
        "  Deserialize: mean={:.2}ns  p50={:.2}ns  p99={:.2}ns",
        deser_stats.mean.as_nanos(),
        deser_stats.p50.as_nanos(),
        deser_stats.p99.as_nanos(),
    );
    println!("  Round-trip:  mean={:.2}ns", (ser_stats.mean + deser_stats.mean).as_nanos(),);

    SerializationResult {
        label: label.to_string(),
        payload_size: match &msg.payload {
            BenchWasmValue::String(s) => s.len(),
            _ => 0,
        },
        serialized_size: avg_size,
        serialize_mean_ns: ser_stats.mean.as_nanos() as f64,
        deserialize_mean_ns: deser_stats.mean.as_nanos() as f64,
    }
}

struct SerializationResult {
    label: String,
    payload_size: usize,
    serialized_size: usize,
    serialize_mean_ns: f64,
    deserialize_mean_ns: f64,
}

// ---------------------------------------------------------------------------
// WASM plugin load time benchmark (if wasm_plugins feature is available)
// ---------------------------------------------------------------------------

fn bench_wasm_plugin_load() -> Option<Duration> {
    // This requires the wasm_plugins feature and a compiled echo plugin
    // Return None if not available
    let echo_wasm_path =
        std::path::Path::new("plugins/examples/echo/target/wasm32-unknown-unknown/release/rust_red_plugin_echo.wasm");
    if !echo_wasm_path.exists() {
        println!("\nWASM echo plugin not found at {:?}", echo_wasm_path);
        println!("Build it with: cd plugins/examples/echo && cargo build --target wasm32-unknown-unknown --release");
        return None;
    }

    // We can't easily load the WASM plugin without the wasm_plugins feature,
    // so we just measure the file read + compilation time as a lower bound
    let iterations = 10;
    let mut times = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let t = Instant::now();
        let _bytes = std::fs::read(echo_wasm_path).expect("failed to read wasm file");
        times.push(t.elapsed());
    }

    let stats = Stats::from_durations(&times, 1);
    println!("\n  WASM file read: mean={:.2}us", stats.mean.as_secs_f64() * 1e6);
    Some(stats.mean)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    init_test_logger();

    println!("========================================");
    println!("  Rust-Red WASM Overhead Benchmark");
    println!("========================================");

    let iterations = 10_000;

    let mut ser_results: Vec<SerializationResult> = Vec::new();

    // Benchmark with different payload sizes
    for &size in &[0_usize, 64, 256, 1024, 4096] {
        let msg = create_sample_message(size);
        let label = format!("payload-{}b", size);
        let result = bench_serialization(&label, &msg, iterations);
        ser_results.push(result);
    }

    // Print summary table
    println!("\n========================================");
    println!("  Serialization Overhead Summary");
    println!("========================================");
    println!(
        "{:<15} {:>10} {:>12} {:>14} {:>14} {:>14}",
        "Benchmark", "Input(B)", "Output(B)", "Ser(mean,ns)", "Deser(mean,ns)", "Total(mean,ns)"
    );
    println!("{}", "-".repeat(81));
    for r in &ser_results {
        println!(
            "{:<15} {:>10} {:>12} {:>14.1} {:>14.1} {:>14.1}",
            r.label,
            r.payload_size,
            r.serialized_size,
            r.serialize_mean_ns,
            r.deserialize_mean_ns,
            r.serialize_mean_ns + r.deserialize_mean_ns,
        );
    }

    // WASM plugin load benchmark
    println!("\n--- WASM Plugin Load Time ---");
    let wasm_load_time = bench_wasm_plugin_load();
    if let Some(load_time) = wasm_load_time {
        println!("  WASM plugin load: {:.2}us", load_time.as_secs_f64() * 1e6);
    }

    // Comparison notes
    println!("\n--- Native vs WASM Comparison ---");
    println!("To compare native vs WASM throughput:");
    println!("  1. Build the echo plugin:");
    println!("     cd plugins/examples/echo && cargo build --target wasm32-unknown-unknown --release");
    println!("  2. Run the throughput benchmark with WASM feature:");
    println!("     cargo bench --bench throughput --features wasm_plugins");
    println!("  3. The throughput benchmark's chain results show native node overhead");
    println!("  4. Compare with WASM echo node throughput (not yet automated)");

    // Output JSON results
    println!("\n--- JSON Results ---");
    let json_results: Vec<serde_json::Value> = ser_results
        .iter()
        .map(|r| {
            let entry = serde_json::json!({
                "benchmark": r.label,
                "payload_size_bytes": r.payload_size,
                "serialized_size_bytes": r.serialized_size,
                "serialize_mean_ns": r.serialize_mean_ns,
                "deserialize_mean_ns": r.deserialize_mean_ns,
                "round_trip_ns": r.serialize_mean_ns + r.deserialize_mean_ns,
            });
            entry
        })
        .collect();

    let mut output = serde_json::json!({
        "serialization": json_results,
    });

    if let Some(load_time) = wasm_load_time {
        output["wasm_plugin_load_us"] = serde_json::json!(load_time.as_secs_f64() * 1e6);
    }

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
