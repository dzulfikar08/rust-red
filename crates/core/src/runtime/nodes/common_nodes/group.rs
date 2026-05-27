use std::sync::Arc;

use serde::Deserialize;

use crate::runtime::flow::Flow;
use crate::runtime::model::json::RedFlowNodeConfig;
use crate::runtime::nodes::*;
use rust_red_macro::*;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GroupStyle {
    #[serde(default)]
    fill: Option<String>,
    #[serde(default)]
    stroke: Option<String>,
    #[serde(default)]
    label: Option<bool>,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GroupConfig {
    #[serde(default)]
    name: String,
    #[serde(default)]
    style: Option<GroupStyle>,
}

#[derive(Debug)]
#[flow_node("group", red_name = "group")]
struct GroupNode {
    base: BaseFlowNodeState,
    #[allow(dead_code)]
    config: GroupConfig,
}

impl GroupNode {
    fn build(
        _flow: &Flow,
        base: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let group_config = GroupConfig::deserialize(&config.rest)?;
        let node = GroupNode { base, config: group_config };
        Ok(Box::new(node))
    }
}

#[async_trait]
impl FlowNodeBehavior for GroupNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        while !stop_token.is_cancelled() {
            stop_token.cancelled().await;
        }
    }
}
