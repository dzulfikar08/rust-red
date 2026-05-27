// Licensed under the Apache License, Version 2.0
// Copyright Rust-Red contributors
// Based on Node-RED 70-HTML.js HTML node

//! HTML Parser Node
//!
//! This node is compatible with Node-RED's HTML node. It can:
//! - Extract elements from HTML documents using CSS selectors
//! - Return extracted content as HTML markup or plain text
//! - Support both single-message (array) and multi-message output modes
//! - Accept selector from configuration or dynamically via `msg.select`
//!
//! Configuration:
//! - `tag`: CSS selector for extracting elements
//! - `ret`: Return format - "html" for markup, "text" for text content
//! - `as`: Output mode - "single" for one message with array, "multi" for separate messages
//!
//! Behavior matches Node-RED:
//! - Parses HTML string in `msg.payload`
//! - Uses CSS selectors (via `scraper` crate) to find matching elements
//! - Sets `msg.parts` for multi-message output (split/join compatibility)

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::Number;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

#[cfg(feature = "nodes_html")]
use scraper::{Html, Selector};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
enum HtmlRetFormat {
    #[serde(rename = "html")]
    #[default]
    Html, // Return HTML markup

    #[serde(rename = "text")]
    Text, // Return text content
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
enum HtmlOutputMode {
    #[serde(rename = "single")]
    #[default]
    Single, // Single message with array payload

    #[serde(rename = "multi")]
    Multi, // Multiple messages, one per element
}

/// HTML Parser Node
///
/// Extracts elements from an HTML document held in `msg.payload` using a CSS selector.
///
/// Configuration:
/// - `tag`: CSS selector string (e.g. "h1", ".classname", "#id")
/// - `ret`: Return format - "html" (default) or "text"
/// - `as`: Output mode - "single" (default) for array, "multi" for separate messages
/// - `property`: Property to operate on (default: "payload")
///
/// Dynamic selector:
/// - If `tag` is not configured, the selector can be provided via `msg.select`
///
/// Output:
/// - Single mode: `msg.payload` contains an array of matched elements
/// - Multi mode: Sends separate messages for each matched element with `msg.parts` set
#[derive(Debug)]
#[flow_node("html", red_name = "HTML")]
struct HtmlNode {
    base: BaseFlowNodeState,
    config: HtmlNodeConfig,
}

impl HtmlNode {
    fn build(
        _flow: &Flow,
        state: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let html_config = HtmlNodeConfig::deserialize(&config.rest)?;
        let node = HtmlNode { base: state, config: html_config };
        Ok(Box::new(node))
    }
}

#[derive(Deserialize, Debug)]
struct HtmlNodeConfig {
    /// CSS selector for extracting elements
    #[serde(default)]
    tag: String,

    /// Return format: "html" for markup, "text" for text content
    #[serde(default)]
    ret: HtmlRetFormat,

    /// Output mode: "single" for array, "multi" for separate messages
    #[serde(rename = "as")]
    #[serde(default)]
    output: HtmlOutputMode,

    /// Property name to operate on (default: "payload")
    #[serde(default = "default_property")]
    property: String,

    /// Number of outputs (usually 1)
    #[serde(default = "default_outputs")]
    #[allow(dead_code)]
    outputs: usize,
}

fn default_property() -> String {
    "payload".to_string()
}

fn default_outputs() -> usize {
    1
}

#[cfg(feature = "nodes_html")]
impl HtmlNode {
    async fn process_html(&self, msg: MsgHandle) -> crate::Result<()> {
        let msg_guard = msg.read().await;

        // Get the HTML string from the configured property
        let html_string = match msg_guard.get(&self.config.property) {
            Some(Variant::String(s)) => s.clone(),
            Some(Variant::Bytes(bytes)) => match String::from_utf8(bytes.clone()) {
                Ok(s) => s,
                Err(_) => {
                    drop(msg_guard);
                    return Err(crate::RustRedError::InvalidOperation(
                        "HTML node: payload contains invalid UTF-8 bytes".to_string(),
                    )
                    .into());
                }
            },
            Some(_) => {
                drop(msg_guard);
                return Err(crate::RustRedError::InvalidOperation(
                    "HTML node: payload must be a string or buffer".to_string(),
                )
                .into());
            }
            None => {
                // No payload - pass through
                drop(msg_guard);
                return self.fan_out_one(Envelope { port: 0, msg }, CancellationToken::new()).await;
            }
        };

        // Determine the selector: use configured tag or fall back to msg.select
        let selector_str = if !self.config.tag.is_empty() {
            self.config.tag.clone()
        } else if let Some(Variant::String(sel)) = msg_guard.get("select") {
            sel.clone()
        } else {
            drop(msg_guard);
            return Err(crate::RustRedError::InvalidOperation(
                "HTML node: no CSS selector configured and msg.select not provided".to_string(),
            )
            .into());
        };

        // Store values before dropping the read guard
        let msg_id = msg_guard.id().unwrap_or_default().to_string();

        // Drop the read guard before parsing HTML (scraper::Html is not Send,
        // so we must not hold it across await points)
        drop(msg_guard);

        // Parse the CSS selector
        let selector = match Selector::parse(&selector_str) {
            Ok(s) => s,
            Err(e) => {
                return Err(crate::RustRedError::InvalidOperation(format!(
                    "HTML node: invalid CSS selector '{selector_str}': {e}"
                ))
                .into());
            }
        };

        // Parse the HTML document and extract results in a fully synchronous block
        // to ensure the non-Send scraper::Html is dropped before any await
        let results: Vec<String> = {
            let document = Html::parse_document(&html_string);
            document
                .select(&selector)
                .map(|element| match self.config.ret {
                    HtmlRetFormat::Html => element.html(),
                    HtmlRetFormat::Text => element.text().collect::<String>(),
                })
                .collect()
        };

        // Build the response message
        let mut response_msg = msg.read().await.clone();

        match self.config.output {
            HtmlOutputMode::Single => {
                // Return a single message with array payload
                let variants: Vec<Variant> = results.into_iter().map(Variant::String).collect();
                response_msg.set("payload".to_string(), Variant::Array(variants));

                let response_handle = MsgHandle::new(response_msg);
                self.fan_out_one(Envelope { port: 0, msg: response_handle }, CancellationToken::new()).await?;
            }
            HtmlOutputMode::Multi => {
                // Send separate messages for each matched element
                if results.is_empty() {
                    // No matches - send empty array
                    response_msg.set("payload".to_string(), Variant::Array(vec![]));
                    let response_handle = MsgHandle::new(response_msg);
                    self.fan_out_one(Envelope { port: 0, msg: response_handle }, CancellationToken::new()).await?;
                } else {
                    let count = results.len();
                    for (i, result) in results.into_iter().enumerate() {
                        let mut individual_msg = response_msg.clone();
                        individual_msg.set("payload".to_string(), Variant::String(result));

                        // Add parts information for split/join compatibility
                        let mut parts = BTreeMap::new();
                        parts.insert("id".to_string(), Variant::String(msg_id.clone()));
                        parts.insert("index".to_string(), Variant::Number(Number::from(i)));
                        parts.insert("count".to_string(), Variant::Number(Number::from(count)));
                        individual_msg.set("parts".to_string(), Variant::Object(parts));

                        let individual_handle = MsgHandle::new(individual_msg);
                        self.fan_out_one(Envelope { port: 0, msg: individual_handle }, CancellationToken::new())
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(not(feature = "nodes_html"))]
impl HtmlNode {
    async fn process_html(&self, _msg: MsgHandle) -> crate::Result<()> {
        log::error!("HTML node is not available. Please enable the 'nodes_html' feature.");
        Err(crate::RustRedError::InvalidOperation("HTML node requires 'nodes_html' feature to be enabled".to_string())
            .into())
    }
}

#[async_trait::async_trait]
impl FlowNodeBehavior for HtmlNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        while !stop_token.is_cancelled() {
            let node = self.clone();

            with_uow(node.as_ref(), stop_token.clone(), |node, msg| async move { node.process_html(msg).await }).await;
        }

        log::debug!("HtmlNode terminated.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_html_config_deserialize_defaults() {
        let config: HtmlNodeConfig = serde_json::from_value(json!({})).unwrap();
        assert!(config.tag.is_empty());
        assert_eq!(config.ret, HtmlRetFormat::Html);
        assert_eq!(config.output, HtmlOutputMode::Single);
        assert_eq!(config.property, "payload");
        assert_eq!(config.outputs, 1);
    }

    #[test]
    fn test_html_config_deserialize_full() {
        let config: HtmlNodeConfig = serde_json::from_value(json!({
            "tag": "h1",
            "ret": "text",
            "as": "multi",
            "property": "payload",
            "outputs": 1
        }))
        .unwrap();
        assert_eq!(config.tag, "h1");
        assert_eq!(config.ret, HtmlRetFormat::Text);
        assert_eq!(config.output, HtmlOutputMode::Multi);
    }

    #[cfg(feature = "nodes_html")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_html_node_extract_text_single() {
        let flows_json = json!([
            {"id": "100", "type": "tab"},
            {"id": "1", "type": "html", "z": "100", "wires": [["2"]], "tag": "h1", "ret": "text"},
            {"id": "2", "z": "100", "type": "test-once"},
        ]);

        let html_payload = "<html><body><h1>Hello World</h1><p>Some text</p></body></html>";
        let msgs_to_inject_json = json!([
            ["1", {"payload": html_payload}],
        ]);

        let engine = crate::runtime::engine::build_test_engine(flows_json).unwrap();
        let msgs_to_inject = Vec::<(ElementId, Msg)>::deserialize(msgs_to_inject_json).unwrap();
        let msgs =
            engine.run_once_with_inject(1, std::time::Duration::from_secs_f64(0.2), msgs_to_inject).await.unwrap();

        let payload = msgs[0]["payload"].as_array().expect("payload should be array");
        assert_eq!(payload.len(), 1);
        assert_eq!(payload[0].as_str().unwrap(), "Hello World");
    }

    #[cfg(feature = "nodes_html")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_html_node_extract_html_single() {
        let flows_json = json!([
            {"id": "100", "type": "tab"},
            {"id": "1", "type": "html", "z": "100", "wires": [["2"]], "tag": "p", "ret": "html"},
            {"id": "2", "z": "100", "type": "test-once"},
        ]);

        let html_payload = "<html><body><p>First</p><p>Second</p></body></html>";
        let msgs_to_inject_json = json!([
            ["1", {"payload": html_payload}],
        ]);

        let engine = crate::runtime::engine::build_test_engine(flows_json).unwrap();
        let msgs_to_inject = Vec::<(ElementId, Msg)>::deserialize(msgs_to_inject_json).unwrap();
        let msgs =
            engine.run_once_with_inject(1, std::time::Duration::from_secs_f64(0.2), msgs_to_inject).await.unwrap();

        let payload = msgs[0]["payload"].as_array().expect("payload should be array");
        assert_eq!(payload.len(), 2);
        assert!(payload[0].as_str().unwrap().contains("First"));
        assert!(payload[1].as_str().unwrap().contains("Second"));
    }

    #[cfg(feature = "nodes_html")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_html_node_extract_multi_with_parts() {
        let flows_json = json!([
            {"id": "100", "type": "tab"},
            {"id": "1", "type": "html", "z": "100", "wires": [["2"]], "tag": "li", "ret": "text", "as": "multi"},
            {"id": "2", "z": "100", "type": "test-once"},
        ]);

        let html_payload = "<ul><li>One</li><li>Two</li><li>Three</li></ul>";
        let msgs_to_inject_json = json!([
            ["1", {"payload": html_payload}],
        ]);

        let engine = crate::runtime::engine::build_test_engine(flows_json).unwrap();
        let msgs_to_inject = Vec::<(ElementId, Msg)>::deserialize(msgs_to_inject_json).unwrap();
        let msgs =
            engine.run_once_with_inject(3, std::time::Duration::from_secs_f64(0.2), msgs_to_inject).await.unwrap();

        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0]["payload"].as_str().unwrap(), "One");
        assert_eq!(msgs[1]["payload"].as_str().unwrap(), "Two");
        assert_eq!(msgs[2]["payload"].as_str().unwrap(), "Three");

        // Check parts are set
        let parts = msgs[0].parts().expect("should have parts");
        assert_eq!(parts.get("index").unwrap().as_i64().unwrap(), 0);
        assert_eq!(parts.get("count").unwrap().as_i64().unwrap(), 3);
    }

    #[cfg(feature = "nodes_html")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_html_node_dynamic_selector() {
        let flows_json = json!([
            {"id": "100", "type": "tab"},
            {"id": "1", "type": "html", "z": "100", "wires": [["2"]], "ret": "text"},
            {"id": "2", "z": "100", "type": "test-once"},
        ]);

        let html_payload = "<html><body><h1>Title</h1><p>Para</p></body></html>";
        let msgs_to_inject_json = json!([
            ["1", {"payload": html_payload, "select": "h1"}],
        ]);

        let engine = crate::runtime::engine::build_test_engine(flows_json).unwrap();
        let msgs_to_inject = Vec::<(ElementId, Msg)>::deserialize(msgs_to_inject_json).unwrap();
        let msgs =
            engine.run_once_with_inject(1, std::time::Duration::from_secs_f64(0.2), msgs_to_inject).await.unwrap();

        let payload = msgs[0]["payload"].as_array().expect("payload should be array");
        assert_eq!(payload.len(), 1);
        assert_eq!(payload[0].as_str().unwrap(), "Title");
    }
}
