use rust_red_core::runtime::nodes::MetaNode;
use rust_red_core::runtime::registry::{RegistryBuilder, RegistryHandle};
use std::collections::BTreeMap;

#[cfg(feature = "wasm_plugins")]
use std::path::PathBuf;
#[cfg(feature = "wasm_plugins")]
use rust_red_wasm_host::{PluginManager, PluginManagerConfig};

// Type aliases to simplify complex nested types
type NodeEntry<'a> = (&'a str, &'a MetaNode);
type NodeList<'a> = Vec<NodeEntry<'a>>;
type RedNameMap<'a> = BTreeMap<&'a str, NodeList<'a>>;
type ModuleMap<'a> = BTreeMap<&'a str, RedNameMap<'a>>;

pub fn create_registry() -> rust_red_core::Result<RegistryHandle> {
    log::info!("Discovering all nodes...");
    log::info!("Loading node registry...");
    RegistryBuilder::default().build()
}

/// Create registry with WASM plugin support.
/// Scans the plugin directory for .wasm files and registers them.
#[cfg(feature = "wasm_plugins")]
pub async fn create_registry_with_plugins(
    plugin_dir: Option<PathBuf>,
) -> rust_red_core::Result<(RegistryHandle, Option<PluginManager>)> {
    log::info!("Discovering all nodes...");
    log::info!("Loading node registry...");

    let mut builder = RegistryBuilder::default();

    let plugin_mgr = if let Some(dir) = plugin_dir {
        log::info!("Loading WASM plugins from: {:?}", dir);
        let config = PluginManagerConfig { plugin_dir: dir.clone(), ..Default::default() };
        match PluginManager::new(&config) {
            Ok(mgr) => {
                match mgr.load_all(&dir).await {
                    Ok(infos) => {
                        log::info!("Loaded {} WASM plugin(s)", infos.len());
                        for info in &infos {
                            log::info!("  - {} ({}) v{}", info.node_type, info.red_name, info.version);
                        }
                    }
                    Err(e) => log::error!("Failed to load WASM plugins: {e}"),
                }
                mgr.register_into(&mut builder).map_err(|e| {
                    rust_red_core::RustRedError::InvalidOperation(format!("WASM plugin registration failed: {e}"))
                })?;
                Some(mgr)
            }
            Err(e) => {
                log::error!("Failed to create PluginManager: {e}");
                None
            }
        }
    } else {
        None
    };

    let registry = builder.build()?;
    Ok((registry, plugin_mgr))
}

pub async fn list_available_nodes() -> anyhow::Result<()> {
    // Create a registry to discover all nodes
    let registry = RegistryBuilder::default().build()?;
    let all_nodes = registry.all();

    println!("Available Node Types in Rust-Red:");
    println!("==================================");

    // Group nodes by module first, then by red_name
    let mut modules: ModuleMap = BTreeMap::new();

    for (type_name, meta_node) in all_nodes.iter() {
        modules
            .entry(meta_node.module)
            .or_default()
            .entry(meta_node.red_name)
            .or_default()
            .push((type_name, meta_node));
    }

    for (module, red_names) in modules.iter() {
        println!("\nModule: `{module}`\n");

        for (red_name, nodes) in red_names.iter() {
            // Sort nodes by type_name within each group
            let mut sorted_nodes = nodes.clone();
            sorted_nodes.sort_by_key(|(type_name, _)| *type_name);

            println!("{red_name}:");

            for (_, meta_node) in sorted_nodes {
                // Build flags string
                let mut flags = Vec::new();
                if meta_node.local {
                    flags.push("local");
                }
                if meta_node.user {
                    flags.push("user");
                }
                let flags_str = if flags.is_empty() { String::new() } else { format!("[{}]", flags.join(", ")) };

                println!(
                    "\t{:<26} {}/{}\t\t{} {}",
                    meta_node.red_name, meta_node.module, meta_node.type_, meta_node.version, flags_str
                );
            }
        }
    }

    println!("\nTotal: {} node types available", all_nodes.len());

    Ok(())
}
