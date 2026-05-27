//! MSSQL (SQL Server) database node compatibility tests.
//!
//! Tests the mssql-config and mssql-query nodes.
//! These tests require a running SQL Server instance and are marked `#[ignore]`.
//!
//! # Prerequisites
//!
//! - SQL Server running on localhost:1433
//! - Database `rustred_test`, user `sa`, password `RustRedTest123!`
//!
//! ```bash
//! # Using Docker:
//! docker run -e 'ACCEPT_EULA=Y' -e 'MSSQL_SA_PASSWORD=RustRedTest123!' \
//!   -p 1433:1433 -d mcr.microsoft.com/mssql/server:2022-latest
//! ```
//!
//! # Running
//!
//! ```bash
//! cargo test -p rust-red-core --test nodered_compat --features internal-testing,nodes_mssql -- test_mssql --ignored
//! ```

use std::time::Duration;

use serde_json::json;

use super::harness::{TestHarness, assert_msg_has, assert_msg_not_has, assert_msg_num};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const MSSQL_HOST: &str = "localhost";
const MSSQL_PORT: u16 = 1433;
const MSSQL_DB: &str = "rustred_test";
const MSSQL_USER: &str = "sa";
const MSSQL_PASS: &str = "RustRedTest123!";

fn build_mssql_flow(query: &str) -> serde_json::Value {
    json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "mssql-config", "name": "testmssql",
         "host": MSSQL_HOST, "port": MSSQL_PORT, "database": MSSQL_DB,
         "user": MSSQL_USER, "password": MSSQL_PASS,
         "encrypt": false, "trustServerCertificate": true,
         "poolMaxSize": 5, "connectTimeoutMs": 5000},
        {"id": "1", "type": "mssql-query", "z": "100", "name": "query",
         "configNode": "c1", "query": query, "timeoutMs": 5000,
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Test basic connection and simple SELECT.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running SQL Server"]
async fn mssql_connect_and_select() {
    let flow = build_mssql_flow("SELECT 1 AS value;");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "test"}), 1, Duration::from_secs(10)).await;

    assert!(!msgs.is_empty(), "should get output from select");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 1);
}

/// Test CREATE TABLE, INSERT, SELECT roundtrip.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running SQL Server"]
async fn mssql_create_insert_select() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "mssql-config", "name": "testmssql",
         "host": MSSQL_HOST, "port": MSSQL_PORT, "database": MSSQL_DB,
         "user": MSSQL_USER, "password": MSSQL_PASS,
         "encrypt": false, "trustServerCertificate": true,
         "poolMaxSize": 5, "connectTimeoutMs": 5000},
        {"id": "1", "type": "mssql-query", "z": "100", "name": "create",
         "configNode": "c1",
         "query": "IF OBJECT_ID('mssql_test_items', 'U') IS NOT NULL DROP TABLE mssql_test_items; CREATE TABLE mssql_test_items (id INT IDENTITY PRIMARY KEY, name NVARCHAR(100), qty INT);",
         "timeoutMs": 5000, "wires": [["2"]]},
        {"id": "2", "type": "mssql-query", "z": "100", "name": "insert",
         "configNode": "c1",
         "query": "INSERT INTO mssql_test_items (name, qty) VALUES ('bolt', 100), ('nut', 200);",
         "timeoutMs": 5000, "wires": [["3"]]},
        {"id": "3", "type": "mssql-query", "z": "100", "name": "select",
         "configNode": "c1",
         "query": "SELECT * FROM mssql_test_items ORDER BY id;",
         "timeoutMs": 5000, "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "start"}), 1, Duration::from_secs(15)).await;

    assert!(!msgs.is_empty(), "should get output from select");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 2);
}

/// Test parameterized query with queryParams using tiberius binding.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running SQL Server"]
async fn mssql_parameterized_query() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "mssql-config", "name": "testmssql",
         "host": MSSQL_HOST, "port": MSSQL_PORT, "database": MSSQL_DB,
         "user": MSSQL_USER, "password": MSSQL_PASS,
         "encrypt": false, "trustServerCertificate": true,
         "poolMaxSize": 5, "connectTimeoutMs": 5000},
        {"id": "1", "type": "mssql-query", "z": "100", "name": "setup",
         "configNode": "c1",
         "query": "IF OBJECT_ID('mssql_test_events', 'U') IS NOT NULL DROP TABLE mssql_test_events; CREATE TABLE mssql_test_events (id INT IDENTITY PRIMARY KEY, name NVARCHAR(50), severity INT); INSERT INTO mssql_test_events (name, severity) VALUES ('info', 1), ('warn', 2), ('error', 3);",
         "timeoutMs": 5000, "wires": [["2"]]},
        {"id": "2", "type": "mssql-query", "z": "100", "name": "param_select",
         "configNode": "c1",
         "query": "SELECT * FROM mssql_test_events WHERE severity >= @P1 ORDER BY severity;",
         "timeoutMs": 5000, "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness
        .inject_and_collect_timeout("2", json!({"payload": "query", "queryParams": [2]}), 1, Duration::from_secs(10))
        .await;

    assert!(!msgs.is_empty(), "should get parameterized output");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 2);
}

/// Test error handling for invalid SQL.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running SQL Server"]
async fn mssql_error_invalid_sql() {
    let flow = build_mssql_flow("SELECT * FROM nonexistent_table_xyz;");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "bad"}), 1, Duration::from_secs(10)).await;

    assert!(!msgs.is_empty(), "node should forward message even on error");
    assert_msg_has(&msgs[0], "error");
}
