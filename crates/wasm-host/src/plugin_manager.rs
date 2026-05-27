//! PluginManager — loads .wasm plugin files, validates exports,
//! extracts node info, and registers them into the runtime Registry.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;

use dashmap::DashMap;
use notify::Watcher;
use rust_red_core::runtime::flow::Flow;
use rust_red_core::runtime::model::json::RedFlowNodeConfig;
use rust_red_core::runtime::nodes::*;
use rust_red_core::runtime::registry::RegistryBuilder;
use sha2::{Digest, Sha256};

use crate::abi;
use crate::shim::WasmNodeShim;
use crate::state::WasmNodeState;
use crate::types::WasmNodeInfo;

/// Global registry of WASM modules and engines, keyed by node type string.
/// This is needed because NodeFactory::Flow takes a bare fn pointer (not a closure),
/// so we can't capture the engine/module state directly.
static WASM_MODULE_REGISTRY: OnceLock<DashMap<String, WasmModuleEntry>> = OnceLock::new();

#[derive(Debug)]
struct WasmModuleEntry {
    engine: wasmtime::Engine,
    module: wasmtime::Module,
}

/// A loaded WASM plugin module.
#[derive(Debug)]
pub struct LoadedPlugin {
    /// The compiled wasmtime module.
    pub module: wasmtime::Module,
    /// Extracted node metadata.
    pub info: WasmNodeInfo,
    /// Source file path.
    pub source_path: PathBuf,
    /// SHA-256 checksum of the .wasm file.
    pub checksum: Vec<u8>,
    /// When this plugin was loaded.
    pub loaded_at: SystemTime,
}

/// Manages loading, validation, and registration of WASM node plugins.
pub struct PluginManager {
    /// Shared wasmtime engine (async + epoch interruption).
    pub engine: wasmtime::Engine,
    /// Loaded plugins keyed by node_type.
    pub plugins: DashMap<String, LoadedPlugin>,
    /// Directory to scan for .wasm files.
    pub plugin_dir: PathBuf,
}

/// Configuration for creating a new PluginManager.
#[derive(Debug, Clone)]
pub struct PluginManagerConfig {
    /// Directory path for .wasm plugins.
    pub plugin_dir: PathBuf,
    /// Maximum fuel per process_msg call.
    pub max_fuel: u64,
    /// Maximum memory pages per instance.
    pub max_memory_pages: u32,
}

impl Default for PluginManagerConfig {
    fn default() -> Self {
        Self { plugin_dir: PathBuf::from("./plugins"), max_fuel: 10_000_000, max_memory_pages: 16 }
    }
}

fn global_registry() -> &'static DashMap<String, WasmModuleEntry> {
    WASM_MODULE_REGISTRY.get_or_init(DashMap::new)
}

impl PluginManager {
    /// Create a new PluginManager with a wasmtime engine configured for async + epoch interruption.
    pub fn new(config: &PluginManagerConfig) -> anyhow::Result<Self> {
        let mut wasm_config = wasmtime::Config::new();
        wasm_config.epoch_interruption(true);
        wasm_config.wasm_multi_memory(true);
        let engine = wasmtime::Engine::new(&wasm_config)?;

        Ok(Self { engine, plugins: DashMap::new(), plugin_dir: config.plugin_dir.clone() })
    }

    /// Scan the plugin directory and load all .wasm files.
    pub async fn load_all(&self, plugin_dir: &Path) -> anyhow::Result<Vec<WasmNodeInfo>> {
        let mut infos = Vec::new();
        if !plugin_dir.exists() {
            log::warn!("Plugin directory does not exist: {:?}", plugin_dir);
            return Ok(infos);
        }

        let mut entries = tokio::fs::read_dir(plugin_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension() == Some(OsStr::new("wasm")) {
                match self.load_plugin(&path).await {
                    Ok(info) => {
                        log::info!("Loaded WASM plugin: {} ({})", info.node_type, info.red_name);
                        infos.push(info);
                    }
                    Err(e) => {
                        log::error!("Failed to load WASM plugin {:?}: {e}", path);
                    }
                }
            }
        }
        Ok(infos)
    }

    /// Load a single .wasm plugin, validate, and extract node info.
    pub async fn load_plugin(&self, path: &Path) -> anyhow::Result<WasmNodeInfo> {
        // 1. Compile WASM module
        let module = wasmtime::Module::from_file(&self.engine, path)?;

        // 2. Log imports/exports for debugging
        log::debug!("WASM module imports:");
        for import in module.imports() {
            log::debug!("  {}::{}", import.module(), import.name());
        }
        log::debug!("WASM module exports:");
        for export in module.exports() {
            log::debug!("  {}", export.name());
        }

        // 3. Validate exports
        self.validate_module(&module)?;

        // 4. Create temporary instance to extract node info
        let info = self.extract_node_info(&module).await?;

        // 5. Compute checksum
        let bytes = tokio::fs::read(path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let checksum = hasher.finalize().to_vec();

        // 6. Store in global module registry for factory function lookup
        global_registry()
            .insert(info.node_type.clone(), WasmModuleEntry { engine: self.engine.clone(), module: module.clone() });

        // 7. Store in local plugin map
        self.plugins.insert(
            info.node_type.clone(),
            LoadedPlugin {
                module,
                info: info.clone(),
                source_path: path.to_path_buf(),
                checksum,
                loaded_at: SystemTime::now(),
            },
        );

        Ok(info)
    }

    /// Validate that the module exports the required functions and memory.
    fn validate_module(&self, module: &wasmtime::Module) -> anyhow::Result<()> {
        let exports: Vec<&str> = module.exports().map(|e| e.name()).collect();

        let required = ["rust_red_node_info", "rust_red_on_start", "rust_red_process_msg", "rust_red_on_stop"];

        for req in &required {
            if !exports.iter().any(|e| e == req) {
                anyhow::bail!("WASM module missing required export: {}", req);
            }
        }

        if !exports.iter().any(|e| *e == "memory") {
            anyhow::bail!("WASM module missing required export: memory");
        }

        Ok(())
    }

    /// Create a temporary instance, call `rust_red_node_info`, and parse the result.
    async fn extract_node_info(&self, module: &wasmtime::Module) -> anyhow::Result<WasmNodeInfo> {
        let mut store = wasmtime::Store::new(&self.engine, WasmNodeState::new("temp"));
        store.set_epoch_deadline(1_000_000);

        let mut linker = wasmtime::Linker::new(&self.engine);
        abi::register_core_imports(&mut linker)?;

        log::debug!("Instantiating WASM module for info extraction...");
        let instance =
            linker.instantiate(&mut store, module).map_err(|e| anyhow::anyhow!("WASM instantiation failed: {e}"))?;
        log::debug!("WASM instance created successfully");

        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| anyhow::anyhow!("no memory export"))?;
        log::debug!("Initial memory: {} pages ({} bytes)", memory.size(&store), memory.size(&store) * 65536);

        let _alloc_fn = instance
            .get_typed_func::<u32, u32>(&mut store, "rust_red_alloc")
            .map_err(|e| anyhow::anyhow!("no rust_red_alloc export: {e}"))?;

        let info_fn = instance
            .get_typed_func::<(), u32>(&mut store, "rust_red_node_info")
            .map_err(|e| anyhow::anyhow!("no rust_red_node_info export: {e}"))?;

        log::debug!("Calling rust_red_node_info...");
        let result_ptr =
            info_fn.call(&mut store, ()).map_err(|e| anyhow::anyhow!("rust_red_node_info call failed: {e}"))?;
        log::debug!("rust_red_node_info returned ptr={}", result_ptr);

        // Get result length if exported
        let result_len_fn = instance.get_typed_func::<(), u32>(&mut store, "rust_red_result_len");
        let result_len = match &result_len_fn {
            Ok(f) => f.call(&mut store, ()).unwrap_or(0),
            Err(_) => 256,
        };
        log::debug!("result_len={}", result_len);

        if result_ptr == 0 || result_len == 0 {
            anyhow::bail!("rust_red_node_info returned null/empty");
        }

        let mem_data = memory.data(&store);
        let ptr = result_ptr as usize;
        let len = result_len as usize;
        if ptr + len > mem_data.len() {
            anyhow::bail!(
                "rust_red_node_info result pointer out of bounds (ptr={}, len={}, mem={})",
                ptr,
                len,
                mem_data.len()
            );
        }
        let slice = &mem_data[ptr..][..len];

        // Try postcard first, then fallback to msgpack/JSON for backward compat
        let info: WasmNodeInfo = postcard::from_bytes(slice)
            .or_else(|_| rmp_serde::from_slice(slice))
            .or_else(|_| serde_json::from_slice(slice))
            .map_err(|e| anyhow::anyhow!("failed to parse WasmNodeInfo: {e}"))?;

        log::info!("Extracted WASM node info: type={}, name={}", info.node_type, info.red_name);
        Ok(info)
    }

    /// Register all loaded plugins into the EdgeLinkd node registry.
    pub fn register_into(&self, registry: &mut RegistryBuilder) -> anyhow::Result<()> {
        for entry in self.plugins.iter() {
            let LoadedPlugin { info, .. } = entry.value();

            let plugin_type = info.node_type.clone();
            let red_name = info.red_name.clone();
            let module_name = info.module.clone();
            let version = info.version.clone();

            // Box::leak to get 'static strs for MetaNode, then Box::leak the MetaNode itself
            let meta: &'static MetaNode = Box::leak(Box::new(MetaNode {
                kind: NodeKind::Flow,
                type_: Box::leak(plugin_type.clone().into_boxed_str()),
                factory: NodeFactory::Flow(wasm_node_factory),
                red_id: Box::leak(format!("{}/{}", module_name, red_name).into_boxed_str()),
                red_name: Box::leak(red_name.into_boxed_str()),
                module: Box::leak(module_name.into_boxed_str()),
                version: Box::leak(version.into_boxed_str()),
                local: false,
                user: false,
            }));

            registry.register_dynamic(meta);
        }
        Ok(())
    }

    /// Start watching the plugin directory for changes (hot reload).
    pub async fn start_watcher(&self) -> anyhow::Result<()> {
        let plugin_dir = self.plugin_dir.clone();
        let engine = self.engine.clone();
        // Wrap plugins DashMap in Arc for sharing across the watcher closure
        let plugins_ptr = &self.plugins as *const DashMap<String, LoadedPlugin>;
        let plugins_arc = Arc::new(unsafe { &*plugins_ptr });

        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if matches!(event.kind, notify::EventKind::Create(_) | notify::EventKind::Modify(_)) {
                        for path in event.paths {
                            if path.extension() == Some(OsStr::new("wasm")) {
                                log::info!("WASM plugin changed: {:?}, reloading...", path);
                                let engine = engine.clone();
                                let plugins_ref = plugins_arc.clone();
                                tokio::spawn(async move {
                                    // Load and validate the module
                                    let module = match wasmtime::Module::from_file(&engine, &path) {
                                        Ok(m) => m,
                                        Err(e) => {
                                            log::error!("Failed to compile WASM {:?}: {e}", path);
                                            return;
                                        }
                                    };
                                    // For now, just log - full hot reload requires updating the registry
                                    log::info!("WASM module recompiled: {:?}", path);
                                    let _ = (plugins_ref, module);
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Plugin watcher error: {e}");
                }
            }
        })?;

        watcher.watch(&plugin_dir, notify::RecursiveMode::NonRecursive)?;
        Box::leak(Box::new(watcher));

        Ok(())
    }
}

/// Factory function for creating WASM node instances.
/// Looks up the module in the global registry by node type string.
fn wasm_node_factory(
    _flow: &Flow,
    base: BaseFlowNodeState,
    config: &RedFlowNodeConfig,
    _settings: Option<&config::Config>,
) -> rust_red_core::Result<Box<dyn FlowNodeBehavior>> {
    let node_type = config.type_name.as_str();

    let entry = global_registry().get(node_type).ok_or_else(|| {
        rust_red_core::RustRedError::InvalidOperation(format!("WASM module not found for node type: {}", node_type))
    })?;

    let engine = &entry.engine;
    let module = &entry.module;

    let mut store = wasmtime::Store::new(engine, WasmNodeState::new(&base.id.to_string()));
    store.set_epoch_deadline(1_000_000);

    let mut linker = wasmtime::Linker::new(engine);
    abi::register_core_imports(&mut linker)
        .map_err(|e| rust_red_core::RustRedError::InvalidOperation(format!("linker setup failed: {e}")))?;

    let instance = linker
        .instantiate(&mut store, module)
        .map_err(|e| rust_red_core::RustRedError::InvalidOperation(format!("wasm instantiation failed: {e}")))?;

    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| rust_red_core::RustRedError::InvalidOperation("no memory export".into()))?;

    let process_fn = instance
        .get_typed_func::<(u32, u32), u32>(&mut store, "rust_red_process_msg")
        .map_err(|e| rust_red_core::RustRedError::InvalidOperation(format!("no rust_red_process_msg: {e}")))?;

    let on_start_fn = instance
        .get_typed_func::<(u32, u32), u32>(&mut store, "rust_red_on_start")
        .map_err(|e| rust_red_core::RustRedError::InvalidOperation(format!("no rust_red_on_start: {e}")))?;

    let on_stop_fn = instance
        .get_typed_func::<(), u32>(&mut store, "rust_red_on_stop")
        .map_err(|e| rust_red_core::RustRedError::InvalidOperation(format!("no rust_red_on_stop: {e}")))?;

    let alloc_fn = instance
        .get_typed_func::<u32, u32>(&mut store, "rust_red_alloc")
        .map_err(|e| rust_red_core::RustRedError::InvalidOperation(format!("no rust_red_alloc: {e}")))?;

    let result_len_fn = instance.get_typed_func::<(), u32>(&mut store, "rust_red_result_len").ok();

    let shim =
        WasmNodeShim::new(base, instance, store, memory, process_fn, on_start_fn, on_stop_fn, alloc_fn, result_len_fn);

    log::info!("Created WasmNodeShim for '{}'", node_type);

    Ok(Box::new(shim))
}
