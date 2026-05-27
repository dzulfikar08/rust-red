//! Test harness for Node-RED compatibility tests.
//!
//! Provides `TestHarness` which wraps the Engine and simplifies
//! creating flows, injecting messages, and capturing output.

use std::time::Duration;

use rust_red_core::runtime::engine::Engine;
use rust_red_core::runtime::model::{ElementId, Msg, MsgHandle, Variant};
use serde::Deserialize;
use serde_json::json;

/// Default timeout for test runs (500ms).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(500);

/// Node IDs used as conventions in test flows.
pub mod node_ids {
    /// The tab/flow node ID.
    pub const TAB: &str = "100";
    /// First processing node ID.
    pub const NODE_A: &str = "1";
    /// Second processing node ID.
    pub const NODE_B: &str = "2";
    /// Third processing node ID.
    pub const NODE_C: &str = "3";
    /// Fourth processing node ID.
    pub const NODE_D: &str = "4";
    /// The test-once sink node ID.
    pub const SINK: &str = "99";
    /// Catch node ID.
    pub const CATCH: &str = "c1";
    /// Status node ID.
    pub const STATUS: &str = "s1";
    /// Complete node ID.
    pub const COMPLETE: &str = "cp1";
}

/// A test harness that wraps an Engine and provides convenience methods
/// for building and running test flows.
pub struct TestHarness {
    engine: Engine,
}

impl TestHarness {
    /// Build a test engine from a raw Node-RED flow JSON array.
    ///
    /// The JSON should be an array of node config objects, exactly as
    /// Node-RED would export them. The engine is created but NOT started.
    pub fn from_flow_json(json: serde_json::Value) -> Self {
        let engine = Engine::with_json(
            &rust_red_core::runtime::registry::RegistryBuilder::default().build().unwrap(),
            json,
            None,
        )
        .expect("Failed to build test engine");
        Self { engine }
    }

    /// Build a test engine from a JSON string.
    pub fn from_flow_json_str(json_str: &str) -> Self {
        let json: serde_json::Value = serde_json::from_str(json_str).expect("Invalid JSON string");
        Self::from_flow_json(json)
    }

    /// Run the engine, expecting `expected_count` messages to arrive at
    /// the `test-once` sink nodes. Messages are automatically injected
    /// via the inject nodes (if any have `once: true`).
    ///
    /// Returns the captured messages.
    pub async fn run(&self, expected_count: usize) -> Vec<Msg> {
        self.engine.run_once(expected_count, DEFAULT_TIMEOUT).await.expect("Engine run_once failed")
    }

    /// Run the engine with a timeout, expecting `expected_count` messages.
    pub async fn run_with_timeout(&self, expected_count: usize, timeout: Duration) -> Vec<Msg> {
        self.engine.run_once(expected_count, timeout).await.expect("Engine run_once failed")
    }

    /// Run the engine, injecting messages manually into specific nodes.
    ///
    /// `msgs_to_inject` is a vec of `(node_id_str, msg_body_json)` tuples.
    /// The node_id_str identifies which node to inject into.
    pub async fn run_with_inject(
        &self,
        expected_count: usize,
        msgs_to_inject: Vec<(String, serde_json::Value)>,
    ) -> Vec<Msg> {
        let inject_data: Vec<(ElementId, Msg)> = msgs_to_inject
            .into_iter()
            .map(|(id, val)| {
                let eid: ElementId = id.parse().expect("Invalid node ID");
                let msg: Msg = Msg::deserialize(val).expect("Failed to deserialize msg");
                (eid, msg)
            })
            .collect();

        let inject_json: serde_json::Value =
            inject_data.iter().map(|(eid, msg)| json!([eid.to_string(), msg])).collect();

        let inject_list: Vec<(ElementId, Msg)> =
            Vec::deserialize(inject_json).expect("Failed to deserialize inject list");

        self.engine
            .run_once_with_inject(expected_count, DEFAULT_TIMEOUT, inject_list)
            .await
            .expect("Engine run_once_with_inject failed")
    }

    /// Inject a single message into a node and expect `expected_count` output messages.
    pub async fn inject_and_collect(
        &self,
        node_id: &str,
        msg_json: serde_json::Value,
        expected_count: usize,
    ) -> Vec<Msg> {
        self.run_with_inject(expected_count, vec![(node_id.to_string(), msg_json)]).await
    }

    fn build_inject_list(&self, node_id: &str, msg_json: serde_json::Value) -> Vec<(ElementId, Msg)> {
        let eid: ElementId = node_id.parse().expect("Invalid node ID");
        let msg: Msg = Msg::deserialize(msg_json).expect("Failed to deserialize msg");
        vec![(eid, msg)]
    }

    /// Inject a single message into a node with custom timeout and expect output.
    /// Returns empty vec on timeout instead of panicking.
    pub async fn inject_and_collect_timeout(
        &self,
        node_id: &str,
        msg_json: serde_json::Value,
        expected_count: usize,
        timeout: Duration,
    ) -> Vec<Msg> {
        let inject_list = self.build_inject_list(node_id, msg_json);
        self.engine.run_once_with_inject(expected_count, timeout, inject_list).await.unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

/// Assert that a message property equals an expected Variant value.
#[track_caller]
pub fn assert_msg_eq(msg: &Msg, key: &str, expected: &Variant) {
    let actual = msg.get(key).unwrap_or_else(|| panic!("Message missing property '{key}'"));
    assert_eq!(actual, expected, "Message property '{key}': expected {:?}, got {:?}", expected, actual);
}

/// Assert that a message property equals a string value.
#[track_caller]
pub fn assert_msg_str(msg: &Msg, key: &str, expected: &str) {
    let actual = msg
        .get(key)
        .unwrap_or_else(|| panic!("Message missing property '{key}'"))
        .as_str()
        .unwrap_or_else(|| panic!("Property '{key}' is not a string"));
    assert_eq!(actual, expected, "Message property '{key}' mismatch");
}

/// Assert that a message property equals a numeric value.
#[track_caller]
pub fn assert_msg_num(msg: &Msg, key: &str, expected: i64) {
    let actual = msg.get(key).unwrap_or_else(|| panic!("Message missing property '{key}'"));
    match actual {
        Variant::Number(n) => {
            let actual_num = n.as_i64().unwrap_or_else(|| panic!("Number overflow for '{key}'"));
            assert_eq!(actual_num, expected, "Message property '{key}' mismatch");
        }
        other => panic!("Property '{key}' is not a number: {:?}", other),
    }
}

/// Assert that a message property equals a floating-point value (with tolerance).
#[track_caller]
pub fn assert_msg_f64(msg: &Msg, key: &str, expected: f64) {
    let actual = msg.get(key).unwrap_or_else(|| panic!("Message missing property '{key}'"));
    match actual {
        Variant::Number(n) => {
            let actual_num = n.as_f64().unwrap_or_else(|| panic!("Number is not f64 for '{key}'"));
            assert!(
                (actual_num - expected).abs() < 1e-6,
                "Message property '{key}': expected {expected}, got {actual_num}"
            );
        }
        other => panic!("Property '{key}' is not a number: {:?}", other),
    }
}

/// Assert that a message property equals a boolean value.
#[track_caller]
pub fn assert_msg_bool(msg: &Msg, key: &str, expected: bool) {
    let actual = msg.get(key).unwrap_or_else(|| panic!("Message missing property '{key}'"));
    match actual {
        Variant::Bool(b) => assert_eq!(*b, expected, "Message property '{key}' mismatch"),
        other => panic!("Property '{key}' is not a boolean: {:?}", other),
    }
}

/// Assert that a message has the given property.
#[track_caller]
pub fn assert_msg_has(msg: &Msg, key: &str) {
    assert!(msg.contains(key), "Message missing expected property '{key}'");
}

/// Assert that a message does NOT have the given property.
#[track_caller]
pub fn assert_msg_not_has(msg: &Msg, key: &str) {
    assert!(!msg.contains(key), "Message has unexpected property '{key}'");
}
