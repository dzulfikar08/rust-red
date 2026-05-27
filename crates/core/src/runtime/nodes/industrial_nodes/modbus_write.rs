use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::modbus_config::{ModbusConfigNode, ModbusDataType, downcast_modbus_config, resolve_modbus_config};

#[derive(Deserialize, Debug, Clone)]
struct ModbusWriteConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    #[serde(rename = "functionCode")]
    #[serde(default = "default_fc")]
    function_code: String,
    #[serde(default)]
    address: u16,
    /// Data type for register conversion on write operations.
    /// Only applies to register-based function codes.
    #[serde(default, rename = "dataType")]
    data_type: ModbusDataType,
}

fn default_fc() -> String {
    "writeSingleRegister".to_string()
}

#[derive(Debug)]
#[flow_node("modbus write", red_name = "modbus-write", module = "node-red")]
struct ModbusWriteNode {
    base: BaseFlowNodeState,
    config: ModbusWriteConfig,
}

impl ModbusWriteNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let write_config = ModbusWriteConfig::deserialize(&config.rest)?;
        Ok(Box::new(ModbusWriteNode { base: base_node, config: write_config }))
    }

    async fn resolve_config_node(&self) -> crate::Result<Arc<dyn GlobalNodeBehavior>> {
        resolve_modbus_config(self.flow().as_ref(), &self.config.config_node).await
    }

    async fn perform_write(&self, config_node: &ModbusConfigNode, msg: &MsgHandle) -> crate::Result<Variant> {
        let mut conn = config_node.connection.lock().await;
        let address = self.config.address;

        let guard = msg.read().await;
        let payload = guard.get("payload");
        let value = match self.config.function_code.as_str() {
            "writeSingleCoil" => {
                // FC5
                let val = payload
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| anyhow::anyhow!("payload must be a boolean for writeSingleCoil"))?;
                conn.write_single_coil(address, val).await?;
                Variant::Bool(val)
            }
            "writeMultipleCoils" => {
                // FC15
                let values = payload
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| anyhow::anyhow!("payload must be an array of booleans for writeMultipleCoils"))?;
                let coils: Vec<bool> = values
                    .iter()
                    .enumerate()
                    .map(|(i, v)| v.as_bool().ok_or_else(|| anyhow::anyhow!("payload[{}] is not a boolean", i)))
                    .collect::<crate::Result<Vec<_>>>()?;
                let count = coils.len();
                conn.write_multiple_coils(address, &coils).await?;
                Variant::from(count as i64)
            }
            "writeMultipleRegisters" => {
                // FC16
                let payload_val =
                    payload.ok_or_else(|| anyhow::anyhow!("payload required for writeMultipleRegisters"))?;
                // Accept either an array of numbers or a single value
                // (single value gets converted via data_type into multiple registers)
                let words = if let Some(arr) = payload_val.as_array() {
                    // Array of individual values: each converted via data_type
                    let mut all_words = Vec::new();
                    for item in arr {
                        all_words.extend(self.config.data_type.variant_to_words(item)?);
                    }
                    all_words
                } else {
                    // Single value
                    self.config.data_type.variant_to_words(payload_val)?
                };
                let count = words.len();
                conn.write_multiple_registers(address, &words).await?;
                Variant::from(count as i64)
            }
            _ => {
                // FC6 – writeSingleRegister (default)
                let payload_val =
                    payload.ok_or_else(|| anyhow::anyhow!("payload must be a number for writeSingleRegister"))?;
                let words = self.config.data_type.variant_to_words(payload_val)?;
                // For single-register data types (UInt16/Int16) write FC6
                // For wider types fall through to FC16 semantics
                if words.len() == 1 {
                    conn.write_single_register(address, words[0]).await?;
                } else {
                    conn.write_multiple_registers(address, &words).await?;
                }
                Variant::from(words.len() as i64)
            }
        };
        drop(guard);
        Ok(value)
    }
}

#[async_trait]
impl FlowNodeBehavior for ModbusWriteNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> = match self.resolve_config_node().await {
            Ok(n) => n,
            Err(e) => {
                log::error!("[modbus-write:{}] {}", self.name(), e);
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
                    match node.perform_write(cfg_inner, &msg).await {
                        Ok(value) => {
                            let mut guard = msg.write().await;
                            guard.set("payload".to_string(), value);
                            guard.set(
                                "modbus".to_string(),
                                Variant::from(serde_json::json!({
                                    "functionCode": node.config.function_code,
                                    "address": node.config.address,
                                })),
                            );
                        }
                        Err(e) => {
                            log::warn!("[modbus-write:{}] Write error: {}", node.name(), e);
                            let mut guard = msg.write().await;
                            guard.set("error".to_string(), Variant::String(e.to_string()));
                        }
                    }
                    Ok(())
                }
            })
            .await;
        }
    }
}
