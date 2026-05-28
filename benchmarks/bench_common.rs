//! Common utilities for Rust-Red benchmarks.

use std::time::Duration;

/// Initialize a minimal logger for benchmarks (suppresses most output).
pub fn init_test_logger() {
    let stderr = log4rs::append::console::ConsoleAppender::builder()
        .target(log4rs::append::console::Target::Stderr)
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new("[{l}] {m}{n}")))
        .build();

    let config = log4rs::Config::builder()
        .appender(log4rs::config::Appender::builder().build("stderr", Box::new(stderr)))
        .build(log4rs::config::Root::builder().appender("stderr").build(log::LevelFilter::Warn))
        .unwrap();

    let _ = log4rs::init_config(config);
}

/// Statistics computed from a set of timing measurements.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Stats {
    pub mean: Duration,
    pub min: Duration,
    pub max: Duration,
    pub p50: Duration,
    pub p90: Duration,
    pub p99: Duration,
    pub throughput_per_sec: f64,
    pub iterations: usize,
}

#[allow(dead_code)]
impl Stats {
    /// Compute statistics from a slice of durations.
    /// `msg_count` is the number of messages processed per iteration,
    /// used to compute throughput.
    pub fn from_durations(durations: &[Duration], msg_count: usize) -> Self {
        assert!(!durations.is_empty(), "need at least one duration");

        let mut sorted: Vec<Duration> = durations.to_vec();
        sorted.sort();

        let n = sorted.len();
        let sum: Duration = sorted.iter().sum();
        let mean = sum / n as u32;

        let p50 = sorted[n * 50 / 100];
        let p90 = sorted[n * 90 / 100];
        let p99_idx = std::cmp::max(n.saturating_sub(1), n * 99 / 100);
        let p99 = sorted[p99_idx];
        let min = sorted[0];
        let max = sorted[n - 1];

        let throughput_per_sec = if mean.as_secs_f64() > 0.0 { msg_count as f64 / mean.as_secs_f64() } else { 0.0 };

        Stats { mean, min, max, p50, p90, p99, throughput_per_sec, iterations: n }
    }

    /// Print a human-readable summary of the stats.
    pub fn print_summary(&self, label: &str) {
        println!("  {} ({} iterations):", label, self.iterations);
        println!(
            "    mean={:.2}us  p50={:.2}us  p90={:.2}us  p99={:.2}us  min={:.2}us  max={:.2}us",
            self.mean.as_secs_f64() * 1e6,
            self.p50.as_secs_f64() * 1e6,
            self.p90.as_secs_f64() * 1e6,
            self.p99.as_secs_f64() * 1e6,
            self.min.as_secs_f64() * 1e6,
            self.max.as_secs_f64() * 1e6,
        );
        println!("    throughput={:.0} msgs/sec", self.throughput_per_sec);
    }
}
