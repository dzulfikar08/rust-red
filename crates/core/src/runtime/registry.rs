use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use crate::runtime::nodes::*;

inventory::collect!(MetaNode);

/// Node types written by other Node-RED packages that are served by one of our own nodes.
///
/// The left side is the `type` found in an exported flow, the right side is the type
/// we register. The node factories look at the original `type` to pick the field mapping.
const TYPE_ALIASES: &[(&str, &str)] = &[
    // node-red-contrib-modbus
    ("modbus-client", "modbus-config"),
    ("modbus-read", "modbus read"),
    ("modbus-getter", "modbus read"),
    ("modbus-write", "modbus write"),
    ("modbus-flex-write", "modbus-flex-writer"),
    // node-red-contrib-postgresql
    ("postgreSQLConfig", "postgres-config"),
    ("postgresql", "postgres-query"),
];

/// Resolve an exported node `type` to the type we register, if it is an alias.
pub fn resolve_type_alias(type_name: &str) -> &str {
    TYPE_ALIASES.iter().find(|(alias, _)| *alias == type_name).map(|(_, target)| *target).unwrap_or(type_name)
}

pub trait Registry: 'static + Send + Sync {
    fn all(&self) -> &HashMap<&'static str, &'static MetaNode>;
    fn get(&self, type_name: &str) -> Option<&'static MetaNode>;
}

#[derive(Debug, Clone)]
pub struct RegistryHandle(Arc<dyn Registry>);

impl Deref for RegistryHandle {
    type Target = Arc<dyn Registry>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
struct RegistryImpl {
    meta_nodes: Arc<HashMap<&'static str, &'static MetaNode>>,
}

#[derive(Debug)]
pub struct RegistryBuilder {
    meta_nodes: HashMap<&'static str, &'static MetaNode>,
}

impl Default for RegistryBuilder {
    fn default() -> Self {
        Self::new().with_builtins()
    }
}

impl RegistryBuilder {
    pub fn new() -> Self {
        Self { meta_nodes: HashMap::new() }
    }

    pub fn register(mut self, meta_node: &'static MetaNode) -> Self {
        self.meta_nodes.insert(meta_node.type_, meta_node);
        self
    }

    /// Register a dynamically-created MetaNode (e.g., from a WASM plugin).
    /// The caller is responsible for ensuring the MetaNode and its string
    /// fields have a 'static lifetime (e.g., via Box::leak).
    pub fn register_dynamic(&mut self, meta_node: &'static MetaNode) {
        self.meta_nodes.insert(meta_node.type_, meta_node);
    }

    pub fn with_builtins(mut self) -> Self {
        for meta in inventory::iter::<MetaNode> {
            log::debug!("[REGISTRY] Available built-in Node: '{}'", meta.type_);
            self.meta_nodes.insert(meta.type_, meta);
        }
        self
    }

    pub fn build(self) -> crate::Result<RegistryHandle> {
        if self.meta_nodes.is_empty() {
            log::warn!("There are no meta node in the Registry!");
        }

        let result = RegistryHandle(Arc::new(RegistryImpl { meta_nodes: Arc::new(self.meta_nodes) }));
        Ok(result)
    }
}

impl RegistryImpl {}

impl Registry for RegistryImpl {
    fn all(&self) -> &HashMap<&'static str, &'static MetaNode> {
        &self.meta_nodes
    }

    fn get(&self, type_name: &str) -> Option<&'static MetaNode> {
        self.meta_nodes.get(type_name).or_else(|| self.meta_nodes.get(resolve_type_alias(type_name))).copied()
    }
}

impl std::fmt::Debug for dyn Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry").field("meta_nodes", self.all()).finish()
    }
}
