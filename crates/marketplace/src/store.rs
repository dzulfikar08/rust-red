//! In-memory plugin store with analytics.

use std::collections::BTreeMap;

use chrono::Utc;
use dashmap::DashMap;
use sha2::{Digest, Sha256};

use crate::config::MarketplaceConfig;
use crate::error::MarketplaceError;
use crate::models::*;

/// In-memory store for all marketplace data.
///
/// Uses `DashMap` for concurrent access. Plugin binaries are held as `Vec<u8>`
/// keyed by `"{plugin_id}@{version}"`.
///
/// Feature-gate note: when a persistent backend (S3, SQLite) is added, this
/// struct will delegate to it behind the same trait interface.
pub struct PluginStore {
    /// Plugin records keyed by plugin ID.
    plugins: DashMap<String, PluginRecord>,

    /// Name-to-ID index for fast name lookups.
    name_index: DashMap<String, String>,

    /// WASM binaries keyed by "{plugin_id}@{version}".
    binaries: DashMap<String, Vec<u8>>,

    /// Rating aggregates keyed by plugin ID.
    ratings: DashMap<String, RatingAggregate>,

    /// Marketplace configuration.
    config: MarketplaceConfig,
}

impl PluginStore {
    /// Create a new empty store.
    pub fn new(config: &MarketplaceConfig) -> Self {
        Self {
            plugins: DashMap::new(),
            name_index: DashMap::new(),
            binaries: DashMap::new(),
            ratings: DashMap::new(),
            config: config.clone(),
        }
    }

    /// Return a reference to the config.
    pub fn config(&self) -> &MarketplaceConfig {
        &self.config
    }

    // ---- Publish ----

    /// Publish a new plugin version. Creates the plugin record if it does not
    /// yet exist. Returns the plugin ID.
    pub fn publish(&self, meta: &PluginMetadata, wasm_bytes: Vec<u8>) -> Result<String, MarketplaceError> {
        // Size check
        let size = wasm_bytes.len() as u64;
        if size > self.config.max_plugin_size_bytes {
            return Err(MarketplaceError::PluginTooLarge(size, self.config.max_plugin_size_bytes));
        }

        // Semver validation
        let incoming_ver =
            semver::Version::parse(&meta.version).map_err(|_| MarketplaceError::InvalidSemver(meta.version.clone()))?;

        // Check if plugin already exists
        let plugin_id = if let Some(existing_id) = self.name_index.get(&meta.name) {
            let existing_id = existing_id.clone();
            // Verify version does not already exist
            let plugin = self
                .plugins
                .get(&existing_id)
                .ok_or_else(|| MarketplaceError::Internal("name_index / plugins desync".into()))?;

            if plugin.versions.contains_key(&meta.version) {
                return Err(MarketplaceError::VersionConflict(meta.name.clone(), meta.version.clone()));
            }

            // Enforce that the new version is newer than every existing version
            let max_existing = plugin.versions.keys().filter_map(|v| semver::Version::parse(v).ok()).max();
            if let Some(existing_max) = max_existing {
                if incoming_ver <= existing_max {
                    return Err(MarketplaceError::VersionNotNewer(meta.version.clone(), existing_max.to_string()));
                }
            }

            drop(plugin);
            existing_id
        } else {
            // New plugin
            let id = uuid::Uuid::new_v4().to_string();
            let now = Utc::now();

            let record = PluginRecord {
                id: id.clone(),
                name: meta.name.clone(),
                author: meta.author.clone(),
                description: meta.description.clone(),
                category: meta.category.clone(),
                tags: meta.tags.clone(),
                license: meta.license.clone(),
                node_types: meta.node_types.clone(),
                permissions: meta.permissions.clone(),
                versions: BTreeMap::new(),
                downloads: 0,
                created_at: now,
                updated_at: now,
            };

            self.plugins.insert(id.clone(), record);
            self.name_index.insert(meta.name.clone(), id.clone());
            id
        };

        // Compute checksum
        let mut hasher = Sha256::new();
        hasher.update(&wasm_bytes);
        let checksum = hex::encode(hasher.finalize());

        // Store binary
        let bin_key = format!("{}@{}", plugin_id, meta.version);
        self.binaries.insert(bin_key, wasm_bytes);

        // Insert version record
        let now = Utc::now();
        let version_record =
            PluginVersion { version: meta.version.clone(), checksum, size_bytes: size, published_at: now };

        if let Some(mut plugin) = self.plugins.get_mut(&plugin_id) {
            plugin.versions.insert(meta.version.clone(), version_record);
            plugin.updated_at = now;
            // Update mutable metadata fields
            plugin.description = meta.description.clone();
            plugin.tags = meta.tags.clone();
            plugin.node_types = meta.node_types.clone();
            plugin.permissions = meta.permissions.clone();
        }

        log::info!("Published plugin {} v{} ({} bytes)", meta.name, meta.version, size,);

        Ok(plugin_id)
    }

    // ---- Unpublish ----

    /// Remove a specific version. If it was the last version, the entire
    /// plugin record is removed.
    pub fn unpublish_version(&self, plugin_id: &str, version: &str) -> Result<(), MarketplaceError> {
        let plugin =
            self.plugins.get_mut(plugin_id).ok_or_else(|| MarketplaceError::NotFound(plugin_id.to_string()))?;

        if !plugin.versions.contains_key(version) {
            return Err(MarketplaceError::VersionNotFound(plugin_id.to_string(), version.to_string()));
        }

        // Remove binary
        let bin_key = format!("{}@{}", plugin_id, version);
        drop(plugin); // release borrow before touching binaries
        self.binaries.remove(&bin_key);

        // Remove version from record
        let mut plugin = self.plugins.get_mut(plugin_id).unwrap();
        plugin.versions.remove(version);
        let should_delete = plugin.versions.is_empty();
        let name = plugin.name.clone();
        drop(plugin);

        if should_delete {
            self.plugins.remove(plugin_id);
            self.name_index.remove(&name);
            self.ratings.remove(plugin_id);
        }

        log::info!("Unpublished {}@{}", name, version);
        Ok(())
    }

    // ---- Read ----

    /// Get plugin record by ID.
    pub fn get_plugin(&self, id: &str) -> Option<PluginRecord> {
        self.plugins.get(id).map(|r| r.clone())
    }

    /// Get plugin ID by name.
    pub fn get_plugin_id_by_name(&self, name: &str) -> Option<String> {
        self.name_index.get(name).map(|r| r.clone())
    }

    /// Download a specific version's WASM binary, incrementing the download counter.
    pub fn download(&self, plugin_id: &str, version: &str) -> Result<Vec<u8>, MarketplaceError> {
        let bin_key = format!("{}@{}", plugin_id, version);

        let binary = self.binaries.get(&bin_key).map(|b| b.clone()).ok_or_else(|| {
            // Distinguish between plugin-not-found and version-not-found
            if self.plugins.contains_key(plugin_id) {
                MarketplaceError::VersionNotFound(plugin_id.to_string(), version.to_string())
            } else {
                MarketplaceError::NotFound(plugin_id.to_string())
            }
        })?;

        // Increment download counter
        if let Some(mut plugin) = self.plugins.get_mut(plugin_id) {
            plugin.downloads += 1;
        }

        Ok(binary)
    }

    /// List plugins with optional filters. Returns a paginated result.
    pub fn list_plugins(&self, query: &ListPluginsQuery) -> PluginListResponse {
        let page = query.page.max(1);
        let page_size = query.page_size.clamp(1, 100);

        let mut matches: Vec<PluginSummary> = self
            .plugins
            .iter()
            .filter(|entry| {
                let p = entry.value();
                if let Some(ref cat) = query.category {
                    if p.category != *cat {
                        return false;
                    }
                }
                if let Some(ref tag) = query.tag {
                    if !p.tags.contains(tag) {
                        return false;
                    }
                }
                if let Some(ref author) = query.author {
                    if p.author != *author {
                        return false;
                    }
                }
                if let Some(ref search) = query.search {
                    let s = search.to_lowercase();
                    if !p.name.to_lowercase().contains(&s) && !p.description.to_lowercase().contains(&s) {
                        return false;
                    }
                }
                true
            })
            .map(|entry| {
                let p = entry.value();
                let rating = self.ratings.get(&p.id);
                PluginSummary {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    author: p.author.clone(),
                    description: p.description.clone(),
                    category: p.category.clone(),
                    tags: p.tags.clone(),
                    latest_version: p.versions.keys().last().cloned(),
                    downloads: p.downloads,
                    rating_avg: rating.as_ref().and_then(|r| r.average()),
                    rating_count: rating.as_ref().map_or(0, |r| r.count),
                    created_at: p.created_at,
                    updated_at: p.updated_at,
                }
            })
            .collect();

        matches.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        let total = matches.len();
        let start = ((page - 1) * page_size) as usize;
        let page_items: Vec<_> = matches.into_iter().skip(start).take(page_size as usize).collect();

        PluginListResponse { plugins: page_items, total, page, page_size }
    }

    // ---- Rating ----

    /// Add a rating (1-5) for a plugin. Returns the new average.
    pub fn rate(&self, plugin_id: &str, rating: u8) -> Result<f64, MarketplaceError> {
        if !(1..=5).contains(&rating) {
            return Err(MarketplaceError::BadRequest("rating must be between 1 and 5".into()));
        }

        if !self.plugins.contains_key(plugin_id) {
            return Err(MarketplaceError::NotFound(plugin_id.to_string()));
        }

        let mut agg = self.ratings.entry(plugin_id.to_string()).or_default();
        agg.total_score += rating as u64;
        agg.count += 1;

        Ok(agg.average().unwrap_or(0.0))
    }

    /// Get rating aggregate for a plugin.
    pub fn get_rating(&self, plugin_id: &str) -> RatingAggregate {
        self.ratings.get(plugin_id).map(|r| r.clone()).unwrap_or_default()
    }

    /// Convert a `PluginRecord` to a public `PluginDetail`.
    pub fn record_to_detail(&self, record: &PluginRecord) -> PluginDetail {
        let rating = self.get_rating(&record.id);
        PluginDetail {
            id: record.id.clone(),
            name: record.name.clone(),
            author: record.author.clone(),
            description: record.description.clone(),
            category: record.category.clone(),
            tags: record.tags.clone(),
            license: record.license.clone(),
            node_types: record.node_types.clone(),
            permissions: record.permissions.clone(),
            versions: record
                .versions
                .values()
                .map(|v| VersionSummary {
                    version: v.version.clone(),
                    checksum: v.checksum.clone(),
                    size_bytes: v.size_bytes,
                    published_at: v.published_at,
                })
                .collect(),
            latest_version: record.versions.keys().last().cloned(),
            downloads: record.downloads,
            rating_avg: rating.average(),
            rating_count: rating.count,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}
