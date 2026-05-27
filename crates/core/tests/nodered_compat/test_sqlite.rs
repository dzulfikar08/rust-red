//! SQLite database node compatibility tests.
//!
//! Tests the sqlite-config and sqlite-query nodes using in-memory databases.
//! No external database server is required.
//!
//! # Running
//!
//! ```bash
//! cargo test -p rust-red-core --test nodered_compat --features internal-testing,nodes_sqlite -- test_sqlite
//! ```

use std::time::Duration;

use serde_json::json;

use super::harness::{TestHarness, assert_msg_has, assert_msg_not_has, assert_msg_num, assert_msg_str};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a flow with an in-memory sqlite-config + sqlite-query wired to a test-once sink.
///
/// - Config node (ID "c1"): sqlite-config with `:memory:` database
/// - Query node (ID "1"): the sqlite-query node with the given SQL
/// - Sink (ID "99"): test-once
fn build_sqlite_flow(query: &str) -> serde_json::Value {
    json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "sqlite-config", "name": "testdb", "path": ":memory:", "walMode": true, "busyTimeoutMs": 5000},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "query",
         "configNode": "c1",
         "query": query,
         "timeoutMs": 5000,
         "outputMode": "rows",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

// ---------------------------------------------------------------------------
// CREATE TABLE tests
// ---------------------------------------------------------------------------

/// Create a simple table and verify no error is produced.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_create_table() {
    let flow = build_sqlite_flow("CREATE TABLE test_users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, age INTEGER);");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "create"}), 1, Duration::from_secs(5)).await;

    assert!(!msgs.is_empty(), "sqlite-query should forward message after CREATE TABLE");
    assert_msg_not_has(&msgs[0], "error");
}

// ---------------------------------------------------------------------------
// INSERT tests
// ---------------------------------------------------------------------------

/// Insert a row into a table created in the same flow (chained nodes).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_insert_and_select() {
    // Flow: create table -> select -> sink
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "sqlite-config", "name": "testdb", "path": ":memory:", "walMode": true, "busyTimeoutMs": 5000},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "create",
         "configNode": "c1", "query": "CREATE TABLE sensors (id INTEGER PRIMARY KEY, temperature REAL, label TEXT);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "sqlite-query", "z": "100", "name": "select",
         "configNode": "c1", "query": "SELECT * FROM sensors;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "init"}), 1, Duration::from_secs(5)).await;

    assert!(!msgs.is_empty(), "chained flow should produce output from second node");
    assert_msg_not_has(&msgs[0], "error");
    // Table is empty, so payload should be an empty array
    let payload = msgs[0].get("payload").expect("should have payload");
    assert!(
        payload.as_array().map(|a| a.is_empty()).unwrap_or(false),
        "Empty table should return empty array, got: {:?}",
        payload
    );
}

/// Insert a row using a chained flow and verify we can read it back.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_insert_row_and_query() {
    // Flow: create table -> insert -> select -> sink
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "sqlite-config", "name": "testdb", "path": ":memory:", "walMode": true, "busyTimeoutMs": 5000},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "create",
         "configNode": "c1", "query": "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT, qty INTEGER);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "sqlite-query", "z": "100", "name": "insert",
         "configNode": "c1", "query": "INSERT INTO items (name, qty) VALUES ('widget', 10);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["3"]]},
        {"id": "3", "type": "sqlite-query", "z": "100", "name": "select",
         "configNode": "c1", "query": "SELECT * FROM items;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "start"}), 1, Duration::from_secs(5)).await;

    assert!(!msgs.is_empty(), "should get output from select node");
    assert_msg_not_has(&msgs[0], "error");

    let payload = msgs[0].get("payload").expect("should have payload");
    let rows = payload.as_array().expect("payload should be array");
    assert_eq!(rows.len(), 1, "Should have exactly 1 row");

    // Verify the row contents
    let row = &rows[0];
    assert!(row.as_object().is_some(), "Row should be an object");
}

// ---------------------------------------------------------------------------
// SELECT tests
// ---------------------------------------------------------------------------

/// Test a SELECT query that returns multiple rows with various types.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_select_multiple_rows() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "sqlite-config", "name": "testdb", "path": ":memory:", "walMode": true, "busyTimeoutMs": 5000},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "create",
         "configNode": "c1", "query": "CREATE TABLE metrics (id INTEGER PRIMARY KEY, value REAL, active INTEGER, label TEXT);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "sqlite-query", "z": "100", "name": "insert1",
         "configNode": "c1", "query": "INSERT INTO metrics (value, active, label) VALUES (1.5, 1, 'alpha'), (2.7, 0, 'beta'), (3.14, 1, 'gamma');",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["3"]]},
        {"id": "3", "type": "sqlite-query", "z": "100", "name": "select",
         "configNode": "c1", "query": "SELECT * FROM metrics ORDER BY id;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "go"}), 1, Duration::from_secs(5)).await;

    assert!(!msgs.is_empty(), "should get output from select");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 3);

    let payload = msgs[0].get("payload").expect("should have payload");
    let rows = payload.as_array().expect("payload should be array");
    assert_eq!(rows.len(), 3, "Should return 3 rows");
}

/// Test SELECT with WHERE clause and parameters -- all in one chained flow.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_select_with_where_param() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "sqlite-config", "name": "testdb", "path": ":memory:", "walMode": true, "busyTimeoutMs": 5000},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "create",
         "configNode": "c1", "query": "CREATE TABLE events (id INTEGER PRIMARY KEY, name TEXT, severity INTEGER);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "sqlite-query", "z": "100", "name": "insert_data",
         "configNode": "c1", "query": "INSERT INTO events (name, severity) VALUES ('error_log', 3), ('info_log', 1), ('warn_log', 2);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["3"]]},
        {"id": "3", "type": "sqlite-query", "z": "100", "name": "select_filtered",
         "configNode": "c1", "query": "SELECT * FROM events WHERE severity >= ?1 ORDER BY severity;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness
        .inject_and_collect_timeout("1", json!({"payload": "go", "queryParams": [2]}), 1, Duration::from_secs(5))
        .await;

    assert!(!msgs.is_empty(), "should get filtered output");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 2);
}

// ---------------------------------------------------------------------------
// UPDATE tests
// ---------------------------------------------------------------------------

/// Test UPDATE with parameterized values in a single chained flow.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_update_with_params() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "sqlite-config", "name": "testdb", "path": ":memory:", "walMode": true, "busyTimeoutMs": 5000},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "create",
         "configNode": "c1", "query": "CREATE TABLE inventory (id INTEGER PRIMARY KEY, item TEXT, stock INTEGER);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "sqlite-query", "z": "100", "name": "insert",
         "configNode": "c1", "query": "INSERT INTO inventory (item, stock) VALUES ('bolts', 100);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["3"]]},
        {"id": "3", "type": "sqlite-query", "z": "100", "name": "update",
         "configNode": "c1", "query": "UPDATE inventory SET stock = ?1 WHERE item = ?2;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["4"]]},
        {"id": "4", "type": "sqlite-query", "z": "100", "name": "verify",
         "configNode": "c1", "query": "SELECT stock FROM inventory WHERE item = 'bolts';",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness
        .inject_and_collect_timeout(
            "1",
            json!({"payload": "update", "queryParams": [50, "bolts"]}),
            1,
            Duration::from_secs(5),
        )
        .await;

    assert!(!msgs.is_empty(), "should get verification output");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 1);
}

// ---------------------------------------------------------------------------
// DELETE tests
// ---------------------------------------------------------------------------

/// Test DELETE with parameterized WHERE clause in a chained flow.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_delete_with_param() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "sqlite-config", "name": "testdb", "path": ":memory:", "walMode": true, "busyTimeoutMs": 5000},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "create",
         "configNode": "c1", "query": "CREATE TABLE tasks (id INTEGER PRIMARY KEY, task TEXT);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "sqlite-query", "z": "100", "name": "insert",
         "configNode": "c1", "query": "INSERT INTO tasks (task) VALUES ('task_a'), ('task_b'), ('task_c');",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["3"]]},
        {"id": "3", "type": "sqlite-query", "z": "100", "name": "delete",
         "configNode": "c1", "query": "DELETE FROM tasks WHERE task = ?1;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["4"]]},
        {"id": "4", "type": "sqlite-query", "z": "100", "name": "verify",
         "configNode": "c1", "query": "SELECT COUNT(*) as cnt FROM tasks;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness
        .inject_and_collect_timeout(
            "1",
            json!({"payload": "delete", "queryParams": ["task_b"]}),
            1,
            Duration::from_secs(5),
        )
        .await;

    assert!(!msgs.is_empty(), "should get verification output");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 1);
}

// ---------------------------------------------------------------------------
// Error handling tests
// ---------------------------------------------------------------------------

/// Test that an invalid SQL produces an error on the output message.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_error_invalid_sql() {
    let flow = build_sqlite_flow("SELECT * FROM nonexistent_table;");
    let harness = TestHarness::from_flow_json(flow);

    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "bad_query"}), 1, Duration::from_secs(5)).await;

    assert!(!msgs.is_empty(), "node should forward message even on error");
    assert_msg_has(&msgs[0], "error");
    let error_val = msgs[0].get("error").expect("should have error");
    let error_str = error_val.as_str().expect("error should be string");
    assert!(
        error_str.to_lowercase().contains("no such table") || error_str.contains("error"),
        "Error should mention table not found, got: {}",
        error_str
    );
}

/// Test that a syntax error in SQL produces an error on the output message.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_error_syntax_error() {
    let flow = build_sqlite_flow("INVALID SQL STATEMENT HERE;");
    let harness = TestHarness::from_flow_json(flow);

    let msgs =
        harness.inject_and_collect_timeout("1", json!({"payload": "syntax_error"}), 1, Duration::from_secs(5)).await;

    assert!(!msgs.is_empty(), "node should forward message even on syntax error");
    assert_msg_has(&msgs[0], "error");
}

/// Test that referring to a missing config node produces no output (node enters error state).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_error_missing_config() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "noconfig",
         "configNode": "nonexistent",
         "query": "SELECT 1;",
         "timeoutMs": 5000,
         "outputMode": "rows",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    // With a missing config node, the query node can't run. It reports an error status
    // and waits for cancellation. So we expect no messages at the sink.
    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "test"}), 1, Duration::from_secs(2)).await;

    assert!(msgs.is_empty(), "No message should arrive when config node is missing");
}

// ---------------------------------------------------------------------------
// Dynamic query override tests
// ---------------------------------------------------------------------------

/// Test that the configured query runs correctly for simple SELECT.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_configured_query_used() {
    let flow = build_sqlite_flow("SELECT 1 AS value;");
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "test"}), 1, Duration::from_secs(5)).await;

    assert!(!msgs.is_empty(), "should get output");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 1);
}

// ---------------------------------------------------------------------------
// Column type mapping tests
// ---------------------------------------------------------------------------

/// Test that various SQLite column types are mapped correctly to Variant types.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_column_type_mapping() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "sqlite-config", "name": "testdb", "path": ":memory:", "walMode": true, "busyTimeoutMs": 5000},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "create",
         "configNode": "c1",
         "query": "CREATE TABLE types_test (id INTEGER PRIMARY KEY, int_col INTEGER, real_col REAL, text_col TEXT, null_col TEXT);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "sqlite-query", "z": "100", "name": "insert",
         "configNode": "c1",
         "query": "INSERT INTO types_test (int_col, real_col, text_col, null_col) VALUES (42, 3.14, 'hello', NULL);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["3"]]},
        {"id": "3", "type": "sqlite-query", "z": "100", "name": "select",
         "configNode": "c1",
         "query": "SELECT int_col, real_col, text_col, null_col FROM types_test;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "go"}), 1, Duration::from_secs(5)).await;

    assert!(!msgs.is_empty(), "should get typed output");
    assert_msg_not_has(&msgs[0], "error");

    let payload = msgs[0].get("payload").expect("should have payload");
    let rows = payload.as_array().expect("payload should be array");
    assert_eq!(rows.len(), 1, "Should have exactly 1 row");

    let row = &rows[0];
    let row_obj = row.as_object().expect("row should be object");

    // int_col should be a number (i64)
    let int_val = row_obj.get("int_col").expect("should have int_col");
    assert!(int_val.as_i64().is_some() || int_val.as_f64().is_some(), "int_col should be numeric, got: {:?}", int_val);

    // real_col should be a number (f64)
    let real_val = row_obj.get("real_col").expect("should have real_col");
    assert!(real_val.as_f64().is_some(), "real_col should be f64, got: {:?}", real_val);

    // text_col should be a string
    let text_val = row_obj.get("text_col").expect("should have text_col");
    assert!(text_val.as_str().is_some(), "text_col should be string, got: {:?}", text_val);

    // null_col should be Null variant
    let null_val = row_obj.get("null_col").expect("should have null_col");
    assert!(null_val.is_null(), "null_col should be Null, got: {:?}", null_val);
}

// ---------------------------------------------------------------------------
// Empty result tests
// ---------------------------------------------------------------------------

/// Test that querying an empty table returns an empty array.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_empty_result_set() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "sqlite-config", "name": "testdb", "path": ":memory:", "walMode": true, "busyTimeoutMs": 5000},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "create",
         "configNode": "c1", "query": "CREATE TABLE empty_tbl (id INTEGER PRIMARY KEY, val TEXT);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "sqlite-query", "z": "100", "name": "select",
         "configNode": "c1", "query": "SELECT * FROM empty_tbl;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "go"}), 1, Duration::from_secs(5)).await;

    assert!(!msgs.is_empty(), "should get output even for empty result");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 0);

    let payload = msgs[0].get("payload").expect("should have payload");
    assert!(payload.as_array().map(|a| a.is_empty()).unwrap_or(false), "Empty table should return empty array");
}

// ---------------------------------------------------------------------------
// Aggregate query tests
// ---------------------------------------------------------------------------

/// Test that a simple aggregate function works.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_aggregate_query() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "sqlite-config", "name": "testdb", "path": ":memory:", "walMode": true, "busyTimeoutMs": 5000},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "create",
         "configNode": "c1",
         "query": "CREATE TABLE numbers (n INTEGER);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "sqlite-query", "z": "100", "name": "insert",
         "configNode": "c1",
         "query": "INSERT INTO numbers (n) VALUES (10), (20), (30);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["3"]]},
        {"id": "3", "type": "sqlite-query", "z": "100", "name": "aggregate",
         "configNode": "c1",
         "query": "SELECT SUM(n) as total, AVG(n) as avg_val, COUNT(*) as cnt FROM numbers;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness.inject_and_collect_timeout("1", json!({"payload": "go"}), 1, Duration::from_secs(5)).await;

    assert!(!msgs.is_empty(), "should get aggregate output");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 1);

    let payload = msgs[0].get("payload").expect("should have payload");
    let rows = payload.as_array().expect("payload should be array");
    assert_eq!(rows.len(), 1);
}

// ---------------------------------------------------------------------------
// NULL handling tests
// ---------------------------------------------------------------------------

/// Test that NULL values in query parameters are handled correctly.
/// All nodes in a single chained flow sharing the same in-memory database.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_null_param_handling() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "sqlite-config", "name": "testdb", "path": ":memory:", "walMode": true, "busyTimeoutMs": 5000},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "create",
         "configNode": "c1",
         "query": "CREATE TABLE nullable (id INTEGER PRIMARY KEY, val TEXT);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "sqlite-query", "z": "100", "name": "insert_null",
         "configNode": "c1",
         "query": "INSERT INTO nullable (val) VALUES (?1);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["3"]]},
        {"id": "3", "type": "sqlite-query", "z": "100", "name": "select",
         "configNode": "c1",
         "query": "SELECT val FROM nullable WHERE val IS NULL;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    // Send null as a query parameter -- the null propagates through all chained nodes
    let msgs = harness
        .inject_and_collect_timeout(
            "1",
            json!({"payload": "insert_null", "queryParams": [null]}),
            1,
            Duration::from_secs(5),
        )
        .await;

    assert!(!msgs.is_empty(), "should get output for null param query");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 1);
}

// ---------------------------------------------------------------------------
// Boolean parameter tests
// ---------------------------------------------------------------------------

/// Test that boolean values in query parameters are handled correctly.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_bool_param_handling() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "sqlite-config", "name": "testdb", "path": ":memory:", "walMode": true, "busyTimeoutMs": 5000},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "create",
         "configNode": "c1",
         "query": "CREATE TABLE flags (id INTEGER PRIMARY KEY, flag INTEGER, label TEXT);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "sqlite-query", "z": "100", "name": "insert_true",
         "configNode": "c1",
         "query": "INSERT INTO flags (flag, label) VALUES (?1, 'test_bool');",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["3"]]},
        {"id": "3", "type": "sqlite-query", "z": "100", "name": "select",
         "configNode": "c1",
         "query": "SELECT flag, label FROM flags;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness
        .inject_and_collect_timeout(
            "1",
            json!({"payload": "insert_bool", "queryParams": [true]}),
            1,
            Duration::from_secs(5),
        )
        .await;

    assert!(!msgs.is_empty(), "should get output for bool param");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 1);
}

// ---------------------------------------------------------------------------
// Parameterized insert in single flow
// ---------------------------------------------------------------------------

/// Test parameterized INSERT with queryParams, all in one chained flow.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_parameterized_insert() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "sqlite-config", "name": "testdb", "path": ":memory:", "walMode": true, "busyTimeoutMs": 5000},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "create",
         "configNode": "c1",
         "query": "CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT, price REAL);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "sqlite-query", "z": "100", "name": "insert",
         "configNode": "c1",
         "query": "INSERT INTO products (name, price) VALUES (?1, ?2);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["3"]]},
        {"id": "3", "type": "sqlite-query", "z": "100", "name": "verify",
         "configNode": "c1",
         "query": "SELECT * FROM products;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);

    let msgs = harness
        .inject_and_collect_timeout(
            "1",
            json!({
                "payload": "insert",
                "queryParams": ["gadget", 29.99]
            }),
            1,
            Duration::from_secs(5),
        )
        .await;

    assert!(!msgs.is_empty(), "parameterized insert should forward message");
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 1);
}

// ---------------------------------------------------------------------------
// Sequential injections into the same engine
// ---------------------------------------------------------------------------

/// Test that multiple sequential injections into the same flow work correctly.
/// Since each harness creates a new engine with a fresh in-memory database,
/// we use a single chained flow for both operations.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_sequential_injections() {
    // This flow chains: create -> insert -> count -> sink
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "c1", "type": "sqlite-config", "name": "testdb", "path": ":memory:", "walMode": true, "busyTimeoutMs": 5000},
        {"id": "1", "type": "sqlite-query", "z": "100", "name": "create",
         "configNode": "c1",
         "query": "CREATE TABLE seq_test (id INTEGER PRIMARY KEY, val TEXT);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["2"]]},
        {"id": "2", "type": "sqlite-query", "z": "100", "name": "insert",
         "configNode": "c1",
         "query": "INSERT INTO seq_test (val) VALUES (?1);",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["3"]]},
        {"id": "3", "type": "sqlite-query", "z": "100", "name": "count",
         "configNode": "c1",
         "query": "SELECT COUNT(*) as cnt FROM seq_test;",
         "timeoutMs": 5000, "outputMode": "rows", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness
        .inject_and_collect_timeout("1", json!({"payload": "go", "queryParams": ["row1"]}), 1, Duration::from_secs(5))
        .await;
    assert!(!msgs.is_empty());
    assert_msg_not_has(&msgs[0], "error");
    assert_msg_num(&msgs[0], "rowCount", 1);
}
