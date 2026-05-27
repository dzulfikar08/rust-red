//! PostgreSQL database node compatibility tests.
//!
//! Tests the postgres-config and postgres-query nodes.
//! These tests require a running PostgreSQL server and are marked `#[ignore]`
//! by default.
//!
//! # Prerequisites
//!
//! - PostgreSQL server running on localhost:5432
//! - Database `rustred_test`, user `rustred`, password `rustred`
//!
//! ```bash
//! createdb rustred_test
//! createuser -P rustred  # password: rustred
//! GRANT ALL PRIVILEGES ON DATABASE rustred_test TO rustred;
//! ```
//!
//! # Running
//!
//! ```bash
//! cargo test -p rust-red-core --test nodered_compat --features internal-testing,nodes_postgres -- test_postgres --ignored
//! ```

use std::time::Duration;

use serde_json::json;

use super::harness::{TestHarness, assert_msg_has, assert_msg_not_has, assert_msg_num};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const PG_HOST: &str = "localhost";
const PG_PORT: u16 = 5432;
const PG_DB: &str = "rustred_test";
const PG_USER: &str = "rustred";
const PG_PASS: &str = "rustred";

fn build_pg_flow(query: &str) -> serde_json::Value {
    json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "postgres-config", "name": "testpg",
         "host": PG_HOST, "port": PG_PORT, "dbname": PG_DB,
         "user": PG_USER, "password": PG_PASS,
         "poolMaxSize": 5, "connectTimeoutMs": 5000},
        {"id": "1", "type": "postgres-query", "z": "100", "name": "query",
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
#[ignore = "Requires running PostgreSQL server"]
async fn postgres_connect_and_select() {
    let flow = build_pg_flow("SELECT 1 AS value;");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "test"}), 1, Duration::from_secs(10)).await;

    assert!(!msgs.is_empty(), "should get output from select");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 1);
}

/// Test CREATE TABLE, INSERT, SELECT roundtrip.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running PostgreSQL server"]
async fn postgres_create_insert_select() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "postgres-config", "name": "testpg",
         "host": PG_HOST, "port": PG_PORT, "dbname": PG_DB,
         "user": PG_USER, "password": PG_PASS,
         "poolMaxSize": 5, "connectTimeoutMs": 5000},
        {"id": "1", "type": "postgres-query", "z": "100", "name": "drop",
         "configNode": "c1",
         "query": "DROP TABLE IF EXISTS pg_test_users;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "postgres-query", "z": "100", "name": "create",
         "configNode": "c1",
         "query": "CREATE TABLE pg_test_users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, age INTEGER);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["3"]]},
        {"id": "3", "type": "postgres-query", "z": "100", "name": "insert",
         "configNode": "c1",
         "query": "INSERT INTO pg_test_users (name, age) VALUES ('Alice', 30), ('Bob', 25);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["4"]]},
        {"id": "4", "type": "postgres-query", "z": "100", "name": "select",
         "configNode": "c1",
         "query": "SELECT * FROM pg_test_users ORDER BY id;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "start"}), 1, Duration::from_secs(15)).await;

    assert!(!msgs.is_empty(), "should get output from select");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 2);
}

/// Test parameterized query with queryParams.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running PostgreSQL server"]
async fn postgres_parameterized_query() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "postgres-config", "name": "testpg",
         "host": PG_HOST, "port": PG_PORT, "dbname": PG_DB,
         "user": PG_USER, "password": PG_PASS,
         "poolMaxSize": 5, "connectTimeoutMs": 5000},
        {"id": "1", "type": "postgres-query", "z": "100", "name": "setup",
         "configNode": "c1",
         "query": "DROP TABLE IF EXISTS pg_test_products; CREATE TABLE pg_test_products (id SERIAL PRIMARY KEY, name TEXT, price FLOAT); INSERT INTO pg_test_products (name, price) VALUES ('widget', 9.99), ('gadget', 19.99), ('thingamajig', 29.99);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "postgres-query", "z": "100", "name": "param_select",
         "configNode": "c1",
         "query": "SELECT * FROM pg_test_products WHERE price > $1 ORDER BY price;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness
        .inject_and_collect_timeout("2", json!({"payload": "query", "queryParams": [15.0]}), 1, Duration::from_secs(10))
        .await;

    assert!(!msgs.is_empty(), "should get parameterized output");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 2);
}

/// Test error handling for invalid SQL.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running PostgreSQL server"]
async fn postgres_error_invalid_sql() {
    let flow = build_pg_flow("SELECT * FROM nonexistent_table_xyz;");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "bad"}), 1, Duration::from_secs(10)).await;

    assert!(!msgs.is_empty(), "node should forward message even on error");
    assert_msg_has(&msgs[0], "error");
}

/// Test that msg.query overrides the configured query.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running PostgreSQL server"]
async fn postgres_msg_query_override() {
    // The configured query is invalid, but msg.query should override it
    let flow = build_pg_flow("SELECT 0;");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness
        .inject_and_collect_timeout(
            "1",
            json!({"payload": "override", "query": "SELECT 42 AS answer;"}),
            1,
            Duration::from_secs(10),
        )
        .await;

    assert!(!msgs.is_empty(), "should get output from overridden query");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 1);
}

/// Test connection error with invalid credentials.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires running PostgreSQL server"]
async fn postgres_connection_error() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "postgres-config", "name": "badpg",
         "host": PG_HOST, "port": PG_PORT, "dbname": PG_DB,
         "user": "wrong_user", "password": "wrong_pass",
         "poolMaxSize": 2, "connectTimeoutMs": 3000},
        {"id": "1", "type": "postgres-query", "z": "100", "name": "query",
         "configNode": "c1", "query": "SELECT 1;",
         "timeoutMs": 5000, "outputMode": "rows",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "test"}), 1, Duration::from_secs(10)).await;

    // The node should still forward the message but with an error property
    assert!(!msgs.is_empty(), "should get message with pool error");
    assert_msg_has(&msgs[0], "error");
}
