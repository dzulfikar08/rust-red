//! Flow versioning support for Rust-Red.
//!
//! Provides automatic version snapshots on deploy, diffing between versions,
//! and rollback. Versions stored as JSON files in a `versions/` directory.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Versioning configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersioningConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_versions")]
    pub max_versions: usize,
}

impl Default for VersioningConfig {
    fn default() -> Self {
        Self { enabled: true, max_versions: 50 }
    }
}

fn default_true() -> bool {
    true
}
fn default_max_versions() -> usize {
    50
}

/// Metadata for a single version snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMeta {
    pub id: String,
    pub timestamp: String,
    pub user: Option<String>,
    pub checksum: String,
    pub node_count: usize,
}

/// Result of diffing two flow snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowDiff {
    pub from_version: String,
    pub to_version: String,
    pub added: Vec<serde_json::Value>,
    pub removed: Vec<serde_json::Value>,
    pub modified: Vec<NodeDiff>,
}

/// Per-node change record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDiff {
    pub id: String,
    pub changes: BTreeMap<String, FieldChange>,
}

/// A single field change within a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    pub from: serde_json::Value,
    pub to: serde_json::Value,
}

/// File-based store for flow version snapshots.
///
/// Directory layout (relative to the flows file parent):
/// ```text
/// <parent>/versions/
///   versions.json        <- index of VersionMeta entries
///   v_000001.json         <- canonical flow snapshot
///   v_000002.json
/// ```
pub struct FlowVersionStore {
    base_dir: PathBuf,
    config: VersioningConfig,
}

impl FlowVersionStore {
    pub fn new(flows_file_path: &Path, config: &VersioningConfig) -> Self {
        let base_dir = flows_file_path.parent().unwrap_or_else(|| Path::new(".")).join("versions");
        Self { base_dir, config: config.clone() }
    }

    pub async fn save_version(
        &self,
        flows: &[serde_json::Value],
        user: Option<String>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled {
            return Err("Versioning is disabled".into());
        }

        tokio::fs::create_dir_all(&self.base_dir).await?;

        let canonical = canonicalize_flows(flows);
        let checksum = compute_checksum(&canonical);

        let mut index = self.read_index().await?;
        let next_id = match index.last() {
            Some(last) => last.id.parse::<u64>().unwrap_or(0) + 1,
            None => 1,
        };
        let version_id = format!("{next_id:06}");

        let snapshot_path = self.base_dir.join(format!("v_{version_id}.json"));
        tokio::fs::write(&snapshot_path, &canonical).await?;

        let meta = VersionMeta {
            id: version_id.clone(),
            timestamp: Utc::now().to_rfc3339(),
            user,
            checksum,
            node_count: flows.len(),
        };
        index.push(meta);

        self.prune_old_versions(&mut index).await?;
        self.write_index(&index).await?;

        log::info!("Saved flow version v_{version_id}");
        Ok(version_id)
    }

    pub async fn list_versions(
        &self,
        page: usize,
        per_page: usize,
    ) -> Result<Vec<VersionMeta>, Box<dyn std::error::Error + Send + Sync>> {
        let index = self.read_index().await?;
        let reversed: Vec<VersionMeta> = index.into_iter().rev().collect();
        let start = page.saturating_mul(per_page);
        if start >= reversed.len() {
            return Ok(vec![]);
        }
        let end = std::cmp::min(start + per_page, reversed.len());
        Ok(reversed[start..end].to_vec())
    }

    pub async fn get_version(
        &self,
        version_id: &str,
    ) -> Result<Option<(VersionMeta, Vec<serde_json::Value>)>, Box<dyn std::error::Error + Send + Sync>> {
        let index = self.read_index().await?;
        let meta = match index.iter().find(|m| m.id == version_id) {
            Some(m) => m.clone(),
            None => return Ok(None),
        };

        let snapshot_path = self.base_dir.join(format!("v_{version_id}.json"));
        if !snapshot_path.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&snapshot_path).await?;
        let flows: Vec<serde_json::Value> = serde_json::from_str(&content)?;
        Ok(Some((meta, flows)))
    }

    pub async fn total_versions(&self) -> usize {
        self.read_index().await.map(|i| i.len()).unwrap_or(0)
    }

    pub async fn diff_versions(
        &self,
        from_id: &str,
        to_id: &str,
    ) -> Result<FlowDiff, Box<dyn std::error::Error + Send + Sync>> {
        let from = self.get_version(from_id).await?.ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Version {from_id} not found").into()
        })?;
        let to = self.get_version(to_id).await?.ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Version {to_id} not found").into()
        })?;

        let mut diff = compute_diff(&from.1, &to.1);
        diff.from_version = from_id.to_string();
        diff.to_version = to_id.to_string();
        Ok(diff)
    }

    pub async fn load_version_flows(
        &self,
        version_id: &str,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let snapshot_path = self.base_dir.join(format!("v_{version_id}.json"));
        if !snapshot_path.exists() {
            return Err(format!("Version {version_id} not found").into());
        }
        let content = tokio::fs::read_to_string(&snapshot_path).await?;
        let flows: Vec<serde_json::Value> = serde_json::from_str(&content)?;
        Ok(flows)
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    async fn read_index(&self) -> Result<Vec<VersionMeta>, Box<dyn std::error::Error + Send + Sync>> {
        let index_path = self.base_dir.join("versions.json");
        if !index_path.exists() {
            return Ok(vec![]);
        }
        let content = tokio::fs::read_to_string(&index_path).await?;
        let index: Vec<VersionMeta> = serde_json::from_str(&content)?;
        Ok(index)
    }

    async fn write_index(&self, index: &[VersionMeta]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let index_path = self.base_dir.join("versions.json");
        let json = serde_json::to_string_pretty(index)?;
        tokio::fs::write(&index_path, json).await?;
        Ok(())
    }

    async fn prune_old_versions(
        &self,
        index: &mut Vec<VersionMeta>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let max = self.config.max_versions;
        if index.len() <= max {
            return Ok(());
        }
        let to_remove_count = index.len() - max;
        let to_remove: Vec<String> = index[..to_remove_count].iter().map(|m| m.id.clone()).collect();
        for id in &to_remove {
            let path = self.base_dir.join(format!("v_{id}.json"));
            let _ = tokio::fs::remove_file(&path).await;
        }
        index.drain(..to_remove_count);
        log::info!("Pruned {} old version(s)", to_remove_count);
        Ok(())
    }
}

fn canonicalize_flows(flows: &[serde_json::Value]) -> String {
    let sorted = sort_value(serde_json::Value::Array(flows.to_vec()));
    serde_json::to_string_pretty(&sorted).unwrap_or_default()
}

fn sort_value(v: serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<String, serde_json::Value> =
                map.into_iter().map(|(k, v)| (k, sort_value(v))).collect();
            let mut object = serde_json::Map::new();
            for (k, v) in sorted {
                object.insert(k, v);
            }
            serde_json::Value::Object(object)
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(arr.into_iter().map(sort_value).collect()),
        other => other,
    }
}

fn compute_checksum(canonical_json: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_json.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn compute_diff(from: &[serde_json::Value], to: &[serde_json::Value]) -> FlowDiff {
    let from_map = build_node_map(from);
    let to_map = build_node_map(to);

    let mut added: Vec<serde_json::Value> = Vec::new();
    let mut removed: Vec<serde_json::Value> = Vec::new();
    let mut modified: Vec<NodeDiff> = Vec::new();

    let all_keys: std::collections::BTreeSet<&String> = from_map.keys().chain(to_map.keys()).collect();

    for key in all_keys {
        match (from_map.get(key), to_map.get(key)) {
            (None, Some(node)) => added.push((*node).clone()),
            (Some(node), None) => removed.push((*node).clone()),
            (Some(old), Some(new)) => {
                let changes = diff_node_fields(old, new);
                if !changes.is_empty() {
                    modified.push(NodeDiff { id: key.clone(), changes });
                }
            }
            (None, None) => {}
        }
    }

    FlowDiff { from_version: String::new(), to_version: String::new(), added, removed, modified }
}

fn build_node_map(flows: &[serde_json::Value]) -> std::collections::HashMap<String, &serde_json::Value> {
    flows.iter().filter_map(|v| v.get("id").and_then(|id| id.as_str()).map(|id| (id.to_string(), v))).collect()
}

fn diff_node_fields(old: &serde_json::Value, new: &serde_json::Value) -> BTreeMap<String, FieldChange> {
    let mut changes = BTreeMap::new();
    let old_obj = match old.as_object() {
        Some(o) => o,
        None => return changes,
    };
    let new_obj = match new.as_object() {
        Some(o) => o,
        None => return changes,
    };

    let all_keys: std::collections::BTreeSet<&String> = old_obj.keys().chain(new_obj.keys()).collect();
    for key in all_keys {
        let old_val = old_obj.get(key).cloned().unwrap_or(serde_json::Value::Null);
        let new_val = new_obj.get(key).cloned().unwrap_or(serde_json::Value::Null);
        if old_val != new_val {
            changes.insert(key.clone(), FieldChange { from: old_val, to: new_val });
        }
    }
    changes
}
