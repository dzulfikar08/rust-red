use std::sync::Arc;

use runtime::engine::Engine;
use runtime::registry::RegistryHandle;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use rust_red_core::runtime::model::*;
use rust_red_core::*;

use crate::cliargs::CliArgs;
use crate::env::RustRedEnv;
use crate::flows::ensure_flows_file_exists;
use crate::registry::create_registry;

#[cfg(feature = "wasm_plugins")]
use crate::registry::create_registry_with_plugins;
#[cfg(feature = "wasm_plugins")]
use rust_red_wasm_host::PluginManager;
#[cfg(feature = "wasm_plugins")]
use std::path::PathBuf;

#[cfg(feature = "cluster")]
use rust_red_cluster::{ClusterConfig, ClusterManager};

// TODO move to debug.rs
#[derive(Debug, Clone)]
pub struct MsgInjectionEntry {
    pub nid: ElementId,
    pub msg: MsgHandle,
}

pub struct App {
    _registry: RegistryHandle,
    engine: Arc<RwLock<Engine>>,
    msgs_to_inject: Mutex<Vec<MsgInjectionEntry>>,
    flows_path: String, // Store the resolved flows path
    env: Arc<RustRedEnv>,
    #[cfg(feature = "wasm_plugins")]
    _plugin_manager: Option<PluginManager>,
    #[cfg(feature = "cluster")]
    cluster_manager: Option<Arc<ClusterManager>>,
}

impl App {
    pub async fn new(
        _elargs: Arc<CliArgs>,
        env: Arc<RustRedEnv>,
        _flows_path: Option<String>,
    ) -> rust_red_core::Result<Self> {
        #[cfg(not(feature = "wasm_plugins"))]
        let reg = {
            create_registry()?
        };
        #[cfg(not(feature = "wasm_plugins"))]
        let _plugin_manager: Option<()> = None;

        #[cfg(feature = "wasm_plugins")]
        let (reg, _plugin_manager) = {
            let plugin_dir = env.config.get_string("wasm_plugin_dir").ok().map(PathBuf::from);
            match create_registry_with_plugins(plugin_dir).await {
                Ok((r, pm)) => (r, pm),
                Err(e) => {
                    log::error!("Failed to create registry with WASM plugins: {e}, falling back to builtins");
                    (create_registry()?, None)
                }
            }
        };

        let msgs_to_inject = Vec::new();

        let flows_path =
            env.config.get_string("flows_path").expect("Config must provide flows_path after normalization");
        ensure_flows_file_exists(&flows_path)?;

        log::info!("Loading flows file: {flows_path}");
        let engine = Engine::with_flows_file(&reg, &flows_path, Some(env.config.clone())).await?;

        // Initialize cluster if enabled
        #[cfg(feature = "cluster")]
        let cluster_manager = {
            let cluster_cfg = match ClusterConfig::load(&env.config) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("Failed to load cluster config: {e}, clustering disabled");
                    ClusterConfig::default()
                }
            };
            if cluster_cfg.enabled {
                let cancel = CancellationToken::new();
                let mgr = Arc::new(ClusterManager::new(cluster_cfg, cancel));
                let bridge = rust_red_cluster::bridge::ClusterPartitionerBridge::new(mgr.partition_manager.clone());
                engine.set_cluster_partitioner(Arc::new(bridge));
                Some(mgr)
            } else {
                None
            }
        };
        #[cfg(not(feature = "cluster"))]
        let _cluster_manager: Option<()> = None;

        Ok(App {
            _registry: reg,
            engine: Arc::new(RwLock::new(engine)),
            msgs_to_inject: Mutex::new(msgs_to_inject),
            flows_path: flows_path.clone(),
            env,
            #[cfg(feature = "wasm_plugins")]
            _plugin_manager,
            #[cfg(feature = "cluster")]
            cluster_manager,
        })
    }

    async fn main_flow_task(self: Arc<Self>, cancel: CancellationToken) -> crate::Result<()> {
        {
            let engine = self.engine.read().await;
            engine.start().await?;
        }

        // Inject msgs
        {
            let mut entries = self.msgs_to_inject.lock().await;
            for e in entries.iter() {
                let engine = self.engine.read().await;
                engine.inject_msg(&e.nid, e.msg.clone(), cancel.clone()).await?;
            }
            entries.clear();
        }

        cancel.cancelled().await;

        {
            let engine = self.engine.read().await;
            engine.stop().await?;
        }
        log::info!("The flows engine stopped.");
        Ok(())
    }

    async fn idle_task(self: Arc<Self>, cancel: CancellationToken) -> crate::Result<()> {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                }
                _ = cancel.cancelled() => {
                    // The token was cancelled
                    log::info!("Cancelling the idle task...");
                    break;
                }
            }
        }
        Ok(())
    }

    pub async fn run(self: Arc<Self>, cancel: CancellationToken) -> crate::Result<()> {
        let (res1, res2) = tokio::join!(
            self.clone().main_flow_task(cancel.child_token()),
            self.clone().idle_task(cancel.child_token())
        );
        res1?;
        res2?;
        Ok(())
    }

    /// Get a reference to the registry
    pub fn registry(&self) -> &RegistryHandle {
        &self._registry
    }

    /// Get a reference to the engine
    pub fn engine(&self) -> &Arc<RwLock<Engine>> {
        &self.engine
    }

    pub fn env(&self) -> &Arc<RustRedEnv> {
        &self.env
    }

    /// Get the cluster manager (if clustering is enabled)
    #[cfg(feature = "cluster")]
    pub fn cluster_manager(&self) -> &Option<Arc<ClusterManager>> {
        &self.cluster_manager
    }

    /// Restart the flow engine with updated flows from file
    pub async fn restart_engine(&self) -> crate::Result<()> {
        log::info!("Restarting flow engine...");

        // Stop the current engine (ignore errors if it's already stopped)
        {
            let engine = self.engine.read().await;
            if let Err(e) = engine.stop().await {
                log::warn!("Error stopping engine (may already be stopped): {e}");
            }
        }

        // Load new flows and create new engine
        let flows_path = &self.flows_path;
        ensure_flows_file_exists(flows_path)?;

        let new_engine = Engine::with_flows_file(&self._registry, flows_path, Some(self.env.config.clone())).await?;

        // Replace the engine
        {
            let mut engine = self.engine.write().await;
            *engine = new_engine;
        }

        // Start the new engine
        {
            let engine = self.engine.read().await;
            engine.start().await?;
        }

        log::info!("Flow engine restarted successfully");
        Ok(())
    }
}
