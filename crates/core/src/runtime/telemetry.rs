//! OpenTelemetry tracing and metrics integration for Rust-Red.
//!
//! All functionality is gated behind the `otel` feature flag. When the feature
//! is not enabled, the public API collapses to no-op stubs so that call-sites
//! compile unconditionally.

use std::time::Duration;

// ---------------------------------------------------------------------------
// Feature-gated implementation
// ---------------------------------------------------------------------------
#[cfg(feature = "otel")]
mod inner {
    use super::*;
    use once_cell::sync::OnceCell;
    use opentelemetry::KeyValue;
    use opentelemetry::global;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::metrics::SdkMeterProvider;
    use opentelemetry_sdk::resource::Resource;
    use opentelemetry_sdk::trace::SdkTracerProvider;

    /// Stored tracer provider for graceful shutdown.
    static TRACER_PROVIDER: OnceCell<SdkTracerProvider> = OnceCell::new();
    /// Stored meter provider for graceful shutdown.
    static METER_PROVIDER: OnceCell<SdkMeterProvider> = OnceCell::new();

    /// Initialise the OTLP tracer provider and meter provider.
    ///
    /// Call this once during application startup *before* any spans / metrics
    /// are created.  Returns `Ok(())` on success.
    pub fn init_telemetry(config: &super::TelemetryConfig) -> crate::Result<()> {
        let resource = Resource::builder().with_service_name(config.service_name.clone()).build();

        // -- Tracer (gRPC / OTLP on port 4317) --
        let span_exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(config.endpoint.clone())
            .with_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create OTLP span exporter: {e}"))?;

        let tracer_provider = SdkTracerProvider::builder()
            .with_batch_exporter(span_exporter)
            .with_resource(resource.clone())
            .with_sampler(opentelemetry_sdk::trace::Sampler::TraceIdRatioBased(config.trace_ratio))
            .build();

        global::set_tracer_provider(tracer_provider.clone());
        let _ = TRACER_PROVIDER.set(tracer_provider);

        // -- Meter provider --
        let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_tonic()
            .with_endpoint(config.endpoint.clone())
            .with_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create OTLP metric exporter: {e}"))?;

        let meter_provider =
            SdkMeterProvider::builder().with_periodic_exporter(metric_exporter).with_resource(resource).build();

        global::set_meter_provider(meter_provider.clone());
        let _ = METER_PROVIDER.set(meter_provider);

        log::info!("OpenTelemetry initialised — endpoint={}, service={}", config.endpoint, config.service_name);
        Ok(())
    }

    /// Gracefully flush and shut down telemetry providers.
    pub fn shutdown_telemetry() {
        if let Some(tp) = TRACER_PROVIDER.get() {
            let _: Result<(), _> = tp.force_flush();
            let _: Result<(), _> = tp.shutdown();
        }
        if let Some(mp) = METER_PROVIDER.get() {
            let _: Result<(), _> = mp.shutdown();
        }
        log::info!("OpenTelemetry shut down.");
    }

    // -- Helper wrappers used by instrumented call-sites ----------------------

    pub fn record_message(node_type: &str) {
        let meter = global::meter("rust-red");
        let counter = meter.u64_counter("rust_red_messages_total").build();
        counter.add(1, &[KeyValue::new("node_type", node_type.to_string())]);
    }

    pub fn record_message_duration(node_type: &str, duration: Duration) {
        let meter = global::meter("rust-red");
        let histogram = meter.f64_histogram("rust_red_message_duration_seconds").build();
        histogram.record(duration.as_secs_f64(), &[KeyValue::new("node_type", node_type.to_string())]);
    }

    pub fn record_flow_deployment() {
        let meter = global::meter("rust-red");
        let counter = meter.u64_counter("rust_red_flow_deployments_total").build();
        counter.add(1, &[]);
    }

    pub fn record_active_flows(count: i64) {
        let meter = global::meter("rust-red");
        let gauge = meter.i64_up_down_counter("rust_red_active_flows").build();
        // Using add with the delta is intentional -- callers should pass +1/-1.
        gauge.add(count, &[]);
    }
}

// ---------------------------------------------------------------------------
// No-op fallbacks (feature disabled)
// ---------------------------------------------------------------------------
#[cfg(not(feature = "otel"))]
mod inner {
    use super::*;

    pub fn init_telemetry(_config: &TelemetryConfig) -> crate::Result<()> {
        Ok(())
    }

    pub fn shutdown_telemetry() {}

    pub fn record_message(_node_type: &str) {}

    pub fn record_message_duration(_node_type: &str, _duration: Duration) {}

    pub fn record_flow_deployment() {}

    pub fn record_active_flows(_count: i64) {}
}

// ---------------------------------------------------------------------------
// Public re-exports (always compiled)
// ---------------------------------------------------------------------------

/// Configuration for the OpenTelemetry integration.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Whether telemetry is enabled.
    pub enabled: bool,
    /// OTLP gRPC endpoint (default `http://localhost:4317`).
    pub endpoint: String,
    /// Logical service name (default `rust-red`).
    pub service_name: String,
    /// Trace sampling ratio (0.0 -- 1.0, default 1.0).
    pub trace_ratio: f64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://localhost:4317".to_string(),
            service_name: "rust-red".to_string(),
            trace_ratio: 1.0,
        }
    }
}

impl TelemetryConfig {
    /// Build a `TelemetryConfig` from the application `config::Config`.
    ///
    /// Expected TOML layout:
    /// ```toml
    /// [telemetry]
    /// enabled = true
    /// endpoint = "http://localhost:4317"
    /// service_name = "rust-red"
    /// trace_ratio = 1.0
    /// ```
    pub fn from_config(cfg: &config::Config) -> Self {
        Self {
            enabled: cfg.get_bool("telemetry.enabled").unwrap_or(false),
            endpoint: cfg.get_string("telemetry.endpoint").unwrap_or_else(|_| "http://localhost:4317".to_string()),
            service_name: cfg.get_string("telemetry.service_name").unwrap_or_else(|_| "rust-red".to_string()),
            trace_ratio: cfg.get_float("telemetry.trace_ratio").unwrap_or(1.0),
        }
    }
}

/// Initialise OpenTelemetry providers. No-op when the `otel` feature is disabled.
pub fn init_telemetry(config: &TelemetryConfig) -> crate::Result<()> {
    inner::init_telemetry(config)
}

/// Shut down OpenTelemetry providers. No-op when the `otel` feature is disabled.
pub fn shutdown_telemetry() {
    inner::shutdown_telemetry()
}

/// Increment `rust_red_messages_total` counter (label: `node_type`).
pub fn record_message(node_type: &str) {
    inner::record_message(node_type);
}

/// Record `rust_red_message_duration_seconds` histogram (label: `node_type`).
pub fn record_message_duration(node_type: &str, duration: Duration) {
    inner::record_message_duration(node_type, duration);
}

/// Increment `rust_red_flow_deployments_total` counter.
pub fn record_flow_deployment() {
    inner::record_flow_deployment();
}

/// Adjust `rust_red_active_flows` gauge (pass +1 on start, -1 on stop).
pub fn record_active_flows(count: i64) {
    inner::record_active_flows(count);
}
