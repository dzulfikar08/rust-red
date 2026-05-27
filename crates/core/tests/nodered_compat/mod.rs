//! Node-RED Compatibility Test Suite
//!
//! Integration tests that verify RustRed behaves identically to Node-RED
//! for all node types. Each test creates a flow, injects messages, and
//! asserts that output messages match expected Node-RED behavior.
//!
//! # Running
//!
//! These tests require the `pymod` feature to access test-only engine methods:
//!
//! ```bash
//! cargo test -p rust-red-core --test nodered_compat_tests --features pymod
//! ```
//!
//! To list all tests without running them:
//!
//! ```bash
//! cargo test -p rust-red-core --test nodered_compat_tests --features pymod -- --list
//! ```

mod flow_builder;
mod harness;

mod test_batch;
mod test_catch;
mod test_change;
mod test_comment;
mod test_debug;
mod test_delay;
mod test_file;
mod test_function;
mod test_http_request;
mod test_inject;
mod test_json;
mod test_link;
mod test_range;
mod test_rbe;
mod test_sort;
mod test_split_join;
mod test_switch;
mod test_template;
mod test_trigger;
mod test_watch;

mod test_flow_import;

mod test_csv;
mod test_html;
mod test_xml;
mod test_yaml;

mod test_mqtt;
mod test_mqtt_broker;
mod test_mqtt_flow;
mod test_tcp;
mod test_udp;
mod test_websocket;

// Database driver tests
#[cfg(feature = "nodes_influxdb")]
mod test_influxdb;
#[cfg(feature = "nodes_mssql")]
mod test_mssql;
#[cfg(feature = "nodes_postgres")]
mod test_postgres;
#[cfg(feature = "nodes_sqlite")]
mod test_sqlite;
#[cfg(feature = "nodes_timescaledb")]
mod test_timescaledb;

// Industrial protocol tests
#[cfg(feature = "nodes_bacnet")]
mod test_bacnet;
#[cfg(feature = "nodes_modbus")]
mod test_modbus;
#[cfg(feature = "nodes_opcua")]
mod test_opcua;
