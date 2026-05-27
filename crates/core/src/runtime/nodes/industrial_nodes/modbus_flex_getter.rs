use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::modbus_config::{ModbusDataType, downcast_modbus_config, resolve_modbus_config};

#[derive(Deserialize, Debug, Clone)]
struct ModbusFlexGetterConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    #[serde(default, rename = "dataType")]
    data_type: ModbusDataType,
}

#[derive(Debug)]
#[flow_node("modbus-flex-getter", red_name = "modbus-flex-getter", module = "node-red")]
struct ModbusFlexGetterNode {
    base: BaseFlowNodeState,
    config: ModbusFlexGetterConfig,
}

impl ModbusFlexGetterNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let cfg = ModbusFlexGetterConfig::deserialize(&config.rest)?;
        Ok(Box::new(ModbusFlexGetterNode { base: base_node, config: cfg }))
    }

    fn extract_u16(guard: &Msg, key: &str, default: u16) -> u16 {
        guard.get(key).and_then(|v| v.as_f64()).map(|f| f as u16).unwrap_or(default)
    }

    fn extract_string(guard: &Msg, key: &str, default: &str) -> String {
        guard.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| default.to_string())
    }
}

#[async_trait]
impl FlowNodeBehavior for ModbusFlexGetterNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> =
            match resolve_modbus_config(self.flow().as_ref(), &self.config.config_node).await {
                Ok(n) => n,
                Err(e) => {
                    log::error!("[modbus-flex-getter:{}] {}", self.name(), e);
                    self.report_status(
                        StatusObject {
                            fill: Some(StatusFill::Red),
                            shape: Some(StatusShape::Ring),
                            text: Some(e.to_string()),
                        },
                        stop_token.clone(),
                    )
                    .await;
                    stop_token.cancelled().await;
                    return;
                }
            };

        while !stop_token.is_cancelled() {
            let cancel = stop_token.child_token();
            let this = self.clone();
            let cfg = config_node.clone();
            with_uow(this.as_ref(), cancel.child_token(), |node, msg| {
                let cfg = cfg.clone();
                async move {
                    let cfg_inner = match downcast_modbus_config(&cfg) {
                        Ok(c) => c,
                        Err(e) => {
                            let mut guard = msg.write().await;
                            guard.set("error".to_string(), Variant::String(e.to_string()));
                            return Ok(());
                        }
                    };

                    let (address, quantity, fc) = {
                        let guard = msg.read().await;
                        let address = Self::extract_u16(&guard, "address", 0);
                        let quantity = Self::extract_u16(&guard, "quantity", 1);
                        let fc = Self::extract_string(&guard, "functionCode", "readHoldingRegisters");
                        (address, quantity, fc)
                    };

                    let reg_count = match fc.as_str() {
                        "readCoils" | "readDiscreteInputs" => quantity,
                        _ => quantity * node.config.data_type.register_count(),
                    };

                    let read_result = {
                        let mut conn = cfg_inner.connection.lock().await;
                        match fc.as_str() {
                            "readCoils" => {
                                let values: Vec<bool> = conn.read_coils(address, quantity).await?;
                                Variant::from(values.iter().map(|&b| Variant::from(b)).collect::<Vec<_>>())
                            }
                            "readDiscreteInputs" => {
                                let values: Vec<bool> = conn.read_discrete_inputs(address, quantity).await?;
                                Variant::from(values.iter().map(|&b| Variant::from(b)).collect::<Vec<_>>())
                            }
                            "readInputRegisters" => {
                                let words: Vec<u16> = conn.read_input_registers(address, reg_count).await?;
                                node.config.data_type.convert_batch(&words)?.into_iter().collect::<Vec<_>>().into()
                            }
                            _ => {
                                let words: Vec<u16> = conn.read_holding_registers(address, reg_count).await?;
                                node.config.data_type.convert_batch(&words)?.into_iter().collect::<Vec<_>>().into()
                            }
                        }
                    };

                    {
                        let mut guard = msg.write().await;
                        guard.set("payload".to_string(), read_result);
                        guard.set(
                            "modbus".to_string(),
                            Variant::from(serde_json::json!({
                                "functionCode": fc,
                                "address": address,
                                "quantity": quantity,
                            })),
                        );
                    }

                    node.report_status(
                        StatusObject {
                            fill: Some(StatusFill::Green),
                            shape: Some(StatusShape::Dot),
                            text: Some(format!("{} @ {}", address, fc)),
                        },
                        cancel.child_token(),
                    )
                    .await;

                    Ok(())
                }
            })
            .await;
        }
    }
}
