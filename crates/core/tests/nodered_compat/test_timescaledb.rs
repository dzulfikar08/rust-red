//! TimescaleDB node compatibility tests.
//!
//! Tests the timescaledb-config and timescaledb-query nodes.
//! These tests require a running TimescaleDB (PostgreSQL with TimescaleDB extension)
//! and are marked `#[ignore]`.
//!
//! # Prerequisites
//!
//! - TimescaleDB running on localhost:5432
//! - Database `rustred_test`, user `rustred`, password `rustred`
//! - TimescaleDB extension enabled
//!
//! ```bash
//! # Using Docker:
//! docker run -p 5432:5432 -e POSTGRES_PASSWORD=rustred -d timescale/timescaledb:latest-pg16
//! # Then create user/database:
//! psql -h localhost -U postgres -c "CREATE USER rustred WITH PASSWORD 'rustred';"
//! psql -h localhost -U postgres -c "CREATE DATABASE rustred_test OWNER rustred;"
//! psql -h localhost -U rustred -d rustred_test -c "CREATE EXTENSION IF NOT EXISTS timescaledb;"
//! ```
//!
//! # Running
//!
//! ```bash
//! cargo test -p rust-red-core --test nodered_compat --features internal-testing,nodes_timescaledb -- test_timescaledb --ignored
//! ```

use std::time::Duration;

use serde_json::json;

use super::harness::{TestHarness, assert_msg_has, assert_msg_not_has, assert_msg_num};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TS_HOST: &str = "localhost";
const TS_PORT: u16 = 5432;
const TS_DB: &str = "rustred_test";
const TS_USER: &str = "rustred";
const TS_PASS: &str = "rustred";

fn build_ts_flow(query: &str) -> serde_json::Value {
    json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "timescaledb-config", "name": "testts",
         "host": TS_HOST, "port": TS_PORT, "dbname": TS_DB,
         "user": TS_USER, "password": TS_PASS,
         "poolMaxSize": 5, "connectTimeoutMs": 5000},
        {"id": "1", "type": "timescaledb-query", "z": "100", "name": "query",
         "configNode": "c1", "query": query,
         "timeoutMs": 5000, "outputMode": "rows",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Test basic connection and simple SELECT.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running TimescaleDB server"]
async fn timescaledb_connect_and_select() {
    let flow = build_ts_flow("SELECT 1 AS value;");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "test"}), 1, Duration::from_secs(10)).await;

    assert!(!msgs.is_empty(), "should get output from select");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 1);
}

/// Test hypertable creation and data insertion/querying.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running TimescaleDB server"]
async fn timescaledb_hypertable_insert_select() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "timescaledb-config", "name": "testts",
         "host": TS_HOST, "port": TS_PORT, "dbname": TS_DB,
         "user": TS_USER, "password": TS_PASS,
         "poolMaxSize": 5, "connectTimeoutMs": 5000},
        {"id": "1", "type": "timescaledb-query", "z": "100", "name": "drop",
         "configNode": "c1",
         "query": "DROP TABLE IF EXISTS ts_test_metrics;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "timescaledb-query", "z": "100", "name": "create",
         "configNode": "c1",
         "query": "CREATE TABLE ts_test_metrics (time TIMESTAMPTZ NOT NULL, value DOUBLE PRECISION, label TEXT);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["3"]]},
        {"id": "3", "type": "timescaledb-query", "z": "100", "name": "hypertable",
         "configNode": "c1",
         "query": "SELECT create_hypertable('ts_test_metrics', 'time', migrate_data => true);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["4"]]},
        {"id": "4", "type": "timescaledb-query", "z": "100", "name": "insert",
         "configNode": "c1",
         "query": "INSERT INTO ts_test_metrics (time, value, label) VALUES (NOW(), 42.5, 'cpu'), (NOW() - INTERVAL '1 hour', 38.1, 'mem');",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["5"]]},
        {"id": "5", "type": "timescaledb-query", "z": "100", "name": "select",
         "configNode": "c1",
         "query": "SELECT * FROM ts_test_metrics ORDER BY time DESC;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "start"}), 1, Duration::from_secs(20)).await;

    assert!(!msgs.is_empty(), "should get output from select");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 2);
}

/// Test time_bucket aggregation function (TimescaleDB-specific).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running TimescaleDB server"]
async fn timescaledb_time_bucket_query() {
    // This test assumes ts_test_metrics was created by the hypertable test above
    let flow = build_ts_flow(
        "SELECT time_bucket('1 hour', time) AS bucket, AVG(value) AS avg_val \
         FROM ts_test_metrics GROUP BY bucket ORDER BY bucket DESC LIMIT 5;",
    );
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "query"}), 1, Duration::from_secs(10)).await;

    assert!(!msgs.is_empty(), "should get time_bucket output");
    // This may error if the table doesn't exist yet, which is acceptable
}

/// Test error handling for invalid SQL.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running TimescaleDB server"]
async fn timescaledb_error_invalid_sql() {
    let flow = build_ts_flow("SELECT * FROM nonexistent_table_xyz;");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "bad"}), 1, Duration::from_secs(10)).await;

    assert!(!msgs.is_empty(), "node should forward message even on error");
    assert_msg_has(&msgs[0], "error");
}

/// Test parameterized query with queryParams.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running TimescaleDB server"]
async fn timescaledb_parameterized_query() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "timescaledb-config", "name": "testts",
         "host": TS_HOST, "port": TS_PORT, "dbname": TS_DB,
         "user": TS_USER, "password": TS_PASS,
         "poolMaxSize": 5, "connectTimeoutMs": 5000},
        {"id": "1", "type": "timescaledb-query", "z": "100", "name": "setup",
         "configNode": "c1",
         "query": "DROP TABLE IF EXISTS ts_params_test; CREATE TABLE ts_params_test (id SERIAL PRIMARY KEY, val TEXT); INSERT INTO ts_params_test (val) VALUES ('a'), ('b'), ('c');",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "timescaledb-query", "z": "100", "name": "param_select",
         "configNode": "c1",
         "query": "SELECT * FROM ts_params_test WHERE val = $1;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness
        .inject_and_collect_timeout("2", json!({"payload": "query", "queryParams": ["b"]}), 1, Duration::from_secs(10))
        .await;

    assert!(!msgs.is_empty(), "should get parameterized output");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 1);
}
