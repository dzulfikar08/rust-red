//! Flow context builder for AI assistance.
//!
//! Gathers relevant runtime information (current flow JSON, node types, debug
//! output, etc.) so that the AI provider can produce context-aware responses.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A snapshot of the current flow context, ready to be injected into an AI prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowContext {
    /// The current flow JSON (pretty-printed).
    pub flow_json: String,

    /// Summary of all node types present in the flow.
    pub node_types: Vec<NodeTypeSummary>,

    /// Number of tabs / flow groups.
    pub flow_count: usize,

    /// Number of total nodes.
    pub node_count: usize,

    /// Recent debug messages (if available).
    pub recent_debug_output: Vec<String>,

    /// Any error messages from the runtime.
    pub errors: Vec<String>,
}

/// Summary of a node type present in the flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTypeSummary {
    pub node_type: String,
    pub count: usize,
    pub sample_names: Vec<String>,
}

/// Builder that assembles a [`FlowContext`] from various runtime sources.
pub struct FlowContextBuilder {
    flow_json: Option<String>,
    debug_output: Vec<String>,
    errors: Vec<String>,
}

impl FlowContextBuilder {
    pub fn new() -> Self {
        Self { flow_json: None, debug_output: Vec::new(), errors: Vec::new() }
    }

    /// Set the raw flow JSON (typically the array of node objects).
    pub fn with_flow_json(mut self, json: &str) -> Self {
        // Pretty-print if it's valid JSON
        self.flow_json = match serde_json::from_str::<serde_json::Value>(json) {
            Ok(val) => Some(serde_json::to_string_pretty(&val).unwrap_or_else(|_| json.to_string())),
            Err(_) => Some(json.to_string()),
        };
        self
    }

    /// Add a debug message to the context.
    pub fn with_debug_message(mut self, msg: &str) -> Self {
        self.debug_output.push(msg.to_string());
        self.errors.truncate(20); // Keep last 20 messages
        self
    }

    /// Add an error message to the context.
    pub fn with_error(mut self, msg: &str) -> Self {
        self.errors.push(msg.to_string());
        self.errors.truncate(20); // Keep last 20 errors
        self
    }

    /// Build the final context.
    pub fn build(self) -> FlowContext {
        let flow_json = self.flow_json.unwrap_or_default();
        let (node_types, node_count, flow_count) = Self::analyze_flow(&flow_json);

        FlowContext {
            flow_json,
            node_types,
            flow_count,
            node_count,
            recent_debug_output: self.debug_output,
            errors: self.errors,
        }
    }

    /// Analyze flow JSON to extract node type summaries.
    fn analyze_flow(flow_json: &str) -> (Vec<NodeTypeSummary>, usize, usize) {
        let mut type_map: HashMap<String, (usize, Vec<String>)> = HashMap::new();
        let mut node_count = 0usize;
        let mut flow_count = 0usize;

        // Try to parse as array of nodes
        if let Ok(nodes) = serde_json::from_str::<Vec<serde_json::Value>>(flow_json) {
            for node in &nodes {
                node_count += 1;

                let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");

                if node_type == "tab" {
                    flow_count += 1;
                    continue;
                }

                let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();

                let entry = type_map.entry(node_type.to_string()).or_insert((0, Vec::new()));
                entry.0 += 1;
                if entry.1.len() < 3 && !name.is_empty() {
                    entry.1.push(name);
                }
            }
        }

        let node_types: Vec<NodeTypeSummary> = type_map
            .into_iter()
            .map(|(node_type, (count, sample_names))| NodeTypeSummary { node_type, count, sample_names })
            .collect();

        (node_types, node_count, flow_count)
    }

    /// Build a system prompt for the AI that includes the flow context.
    pub fn build_system_prompt(context: &FlowContext) -> String {
        let mut prompt = String::from(
            "You are an AI assistant for Rust-Red, a Rust reimplementation of Node-RED. \
            You help users build, debug, and understand their visual flow-based programs.\n\n",
        );

        if !context.flow_json.is_empty() {
            prompt.push_str(&format!(
                "## Current Flow\nThe user has {} node(s) across {} flow(s).\n",
                context.node_count, context.flow_count,
            ));

            if !context.node_types.is_empty() {
                prompt.push_str("Node types in use:\n");
                for nt in &context.node_types {
                    prompt.push_str(&format!(
                        "- {} ({} instances{})\n",
                        nt.node_type,
                        nt.count,
                        if nt.sample_names.is_empty() {
                            String::new()
                        } else {
                            format!(": {}", nt.sample_names.join(", "))
                        },
                    ));
                }
            }

            if !context.errors.is_empty() {
                prompt.push_str("\n## Recent Errors\n");
                for err in &context.errors {
                    prompt.push_str(&format!("- {err}\n"));
                }
            }

            if !context.recent_debug_output.is_empty() {
                prompt.push_str("\n## Recent Debug Output\n");
                for msg in &context.recent_debug_output {
                    prompt.push_str(&format!("- {msg}\n"));
                }
            }
        }

        prompt.push_str("\n\nGuidelines:\n");
        prompt.push_str("- Be concise and practical\n");
        prompt.push_str("- When suggesting flows, provide valid Node-RED JSON\n");
        prompt.push_str("- Reference specific node types by their exact type names\n");
        prompt.push_str("- Mention wire connections when relevant\n");

        prompt
    }

    /// Build a compact context string for the suggest/explain endpoints.
    pub fn build_compact_context(context: &FlowContext) -> String {
        let mut ctx = String::new();

        if !context.node_types.is_empty() {
            ctx.push_str("Node types: ");
            ctx.push_str(
                &context
                    .node_types
                    .iter()
                    .map(|nt| format!("{}x{}", nt.count, nt.node_type))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            ctx.push('\n');
        }

        if !context.errors.is_empty() {
            ctx.push_str("Errors: ");
            ctx.push_str(&context.errors.join("; "));
            ctx.push('\n');
        }

        if ctx.is_empty() {
            ctx.push_str("(no active flow context)");
        }

        ctx
    }
}

impl Default for FlowContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_builder() {
        let ctx = FlowContextBuilder::new().build();
        assert!(ctx.flow_json.is_empty());
        assert!(ctx.node_types.is_empty());
        assert_eq!(ctx.node_count, 0);
        assert_eq!(ctx.flow_count, 0);
    }

    #[test]
    fn test_analyze_flow() {
        let flow_json = r#"[
            {"id": "tab1", "type": "tab", "label": "Flow 1"},
            {"id": "n1", "type": "inject", "name": "timestamp", "wires": [["n2"]]},
            {"id": "n2", "type": "debug", "name": "msg.payload", "wires": []},
            {"id": "n3", "type": "inject", "name": "hello", "wires": [["n2"]]}
        ]"#;

        let ctx = FlowContextBuilder::new().with_flow_json(flow_json).build();
        assert_eq!(ctx.flow_count, 1);
        assert_eq!(ctx.node_count, 4); // 3 nodes + 1 tab
        assert_eq!(ctx.node_types.len(), 2); // inject + debug

        let inject_type = ctx.node_types.iter().find(|nt| nt.node_type == "inject").unwrap();
        assert_eq!(inject_type.count, 2);
    }

    #[test]
    fn test_system_prompt() {
        let ctx = FlowContextBuilder::new()
            .with_flow_json(r#"[{"id":"n1","type":"inject","name":"test"}]"#)
            .with_error("Something went wrong")
            .build();

        let prompt = FlowContextBuilder::build_system_prompt(&ctx);
        assert!(prompt.contains("Rust-Red"));
        assert!(prompt.contains("inject"));
        assert!(prompt.contains("Something went wrong"));
    }
}
