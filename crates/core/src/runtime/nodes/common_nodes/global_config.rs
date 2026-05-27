use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::Deserialize;

use crate::runtime::engine::Engine;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct GlobalConfigEnv {
    name: String,
    value: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct GlobalConfigNodeConfig {
    #[serde(default)]
    env: Vec<GlobalConfigEnv>,
}

#[derive(Debug)]
#[global_node("global-config", red_name = "global-config", module = "node-red")]
#[allow(dead_code)]
struct GlobalConfigNode {
    base: BaseGlobalNodeState,
    #[allow(dead_code)]
    env_vars: BTreeMap<String, String>,
}

impl GlobalConfigNode {
    fn build(
        engine: &Engine,
        config: &RedGlobalNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn GlobalNodeBehavior>> {
        let gc_config = GlobalConfigNodeConfig::deserialize(&config.rest).unwrap_or_default();
        let env_vars: BTreeMap<String, String> =
            gc_config.env.iter().map(|e| (e.name.clone(), e.value.clone())).collect();

        if !env_vars.is_empty() {
            log::info!("[global-config:{}] Loaded {} environment variables", config.name, env_vars.len());
        }

        let context = engine.get_context_manager().new_context(engine.context(), config.id.to_string());
        let node = Self {
            base: BaseGlobalNodeState {
                id: config.id,
                name: config.name.clone(),
                type_str: "global-config",
                ordering: config.ordering,
                context,
                disabled: config.disabled,
            },
            env_vars,
        };
        Ok(Box::new(node))
    }
}

#[async_trait]
impl GlobalNodeBehavior for GlobalConfigNode {
    fn get_base(&self) -> &BaseGlobalNodeState {
        &self.base
    }
}
