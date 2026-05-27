//! InfluxDB node compatibility tests.
//!
//! Tests the influxdb-config, influxdb-in, and influxdb-out nodes.
//! These tests require a running InfluxDB v2 server and are marked `#[ignore]`.
//!
//! # Prerequisites
//!
//! - InfluxDB v2 running on localhost:8086
//! - Organization `rustred`, bucket `test`, token configured
//!
//! ```bash
//! # Using Docker:
//! docker run -p 8086:8086 -d influxdb:2 \
//!   influxd --reporting-disabled
//! # Then set up via http://localhost:8086
//! ```
//!
//! # Running
//!
//! ```bash
//! cargo test -p rust-red-core --test nodered_compat --features internal-testing,nodes_influxdb -- test_influxdb --ignored
//! ```

use std::time::Duration;

use serde_json::json;

use super::harness::{TestHarness, assert_msg_has, assert_msg_not_has};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const INFLUX_URL: &str = "http://localhost:8086";
const INFLUX_TOKEN: &str = "rustred-test-token";
const INFLUX_ORG: &str = "rustred";
const INFLUX_BUCKET: &str = "test";

fn build_influxdb_config_node(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "type": "influxdb-config",
        "name": "testinflux",
        "url": INFLUX_URL,
        "token": INFLUX_TOKEN,
        "org": INFLUX_ORG,
        "bucket": INFLUX_BUCKET,
        "version": "v2"
    })
}

fn build_influxdb_in_flow(measurement: &str) -> serde_json::Value {
    json!([
        {"id": "100", "type": "tab"},
        build_influxdb_config_node("c1"),
        {"id": "1", "type": "influxdb-in", "z": "100", "name": "write",
         "configNode": "c1",
         "measurement": measurement,
         "tagColumns": ["host"],
         "fieldColumns": ["value"],
         "timestampColumn": "",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

fn build_influxdb_out_flow(query: &str) -> serde_json::Value {
    json!([
        {"id": "100", "type": "tab"},
        build_influxdb_config_node("c1"),
        {"id": "1", "type": "influxdb-out", "z": "100", "name": "query",
         "configNode": "c1",
         "query": query,
         "timeoutMs": 5000,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

// ---------------------------------------------------------------------------
// influxdb-in tests
// ---------------------------------------------------------------------------

/// Test writing a single data point to InfluxDB.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running InfluxDB v2 server"]
async fn influxdb_in_write_point() {
    let flow = build_influxdb_in_flow("test_measurement");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness
        .inject_and_collect_timeout(
            "1",
            json!({
                "payload": {"value": 42.5},
                "host": "server01"
            }),
            1,
            Duration::from_secs(10),
        )
        .await;

    assert!(!msgs.is_empty(), "influxdb-in should forward message");
    assert_msg_not_has(&msgs[0], "error");
}

/// Test writing multiple field values from payload object.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running InfluxDB v2 server"]
async fn influxdb_in_write_multiple_fields() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        build_influxdb_config_node("c1"),
        {"id": "1", "type": "influxdb-in", "z": "100", "name": "write_multi",
         "configNode": "c1",
         "measurement": "multi_fields",
         "tagColumns": ["location"],
         "fieldColumns": ["temperature", "humidity"],
         "timestampColumn": "",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness
        .inject_and_collect_timeout(
            "1",
            json!({
                "location": "room1",
                "temperature": 22.5,
                "humidity": 65.0
            }),
            1,
            Duration::from_secs(10),
        )
        .await;

    assert!(!msgs.is_empty(), "influxdb-in should forward message");
    assert_msg_not_has(&msgs[0], "error");
}

/// Test writing with invalid credentials produces an error.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running InfluxDB v2 server"]
async fn influxdb_in_auth_error() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "influxdb-config", "name": "badinflux",
         "url": INFLUX_URL, "token": "invalid_token",
         "org": INFLUX_ORG, "bucket": INFLUX_BUCKET, "version": "v2"},
        {"id": "1", "type": "influxdb-in", "z": "100", "name": "write",
         "configNode": "c1",
         "measurement": "test",
         "tagColumns": [], "fieldColumns": ["value"], "timestampColumn": "",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": {"value": 1.0}}), 1, Duration::from_secs(10)).await;

    assert!(!msgs.is_empty(), "should get message with auth error");
    assert_msg_has(&msgs[0], "error");
}

// ---------------------------------------------------------------------------
// influxdb-out tests
// ---------------------------------------------------------------------------

/// Test querying InfluxDB with a Flux query.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running InfluxDB v2 server"]
async fn influxdb_out_flux_query() {
    let flow = build_influxdb_out_flow("from(bucket: \"test\") |> range(start: -1h) |> limit(n: 10)");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "query"}), 1, Duration::from_secs(10)).await;

    assert!(!msgs.is_empty(), "influxdb-out should forward query result");
    assert_msg_not_has(&msgs[0], "error");
}

/// Test querying with mustache template substitution.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running InfluxDB v2 server"]
async fn influxdb_out_template_query() {
    let flow = build_influxdb_out_flow("from(bucket: \"{{bucket}}\") |> range(start: -1h) |> limit(n: 10)");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness
        .inject_and_collect_timeout("1", json!({"payload": "query", "bucket": "test"}), 1, Duration::from_secs(10))
        .await;

    assert!(!msgs.is_empty(), "influxdb-out should forward templated query result");
    assert_msg_not_has(&msgs[0], "error");
}

/// Test error handling for malformed Flux query.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running InfluxDB v2 server"]
async fn influxdb_out_invalid_query() {
    let flow = build_influxdb_out_flow("INVALID FLUX QUERY SYNTAX");
    let harness = TestHarness::from_flow_json(flow);

    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "bad_query"}), 1, Duration::from_secs(10)).await;

    assert!(!msgs.is_empty(), "should get message with query error");
    assert_msg_has(&msgs[0], "error");
}
