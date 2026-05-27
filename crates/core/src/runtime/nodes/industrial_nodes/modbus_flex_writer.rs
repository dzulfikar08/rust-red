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
struct ModbusFlexWriterConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    #[serde(default, rename = "dataType")]
    data_type: ModbusDataType,
}

#[derive(Debug)]
#[flow_node("modbus-flex-writer", red_name = "modbus-flex-writer", module = "node-red")]
struct ModbusFlexWriterNode {
    base: BaseFlowNodeState,
    config: ModbusFlexWriterConfig,
}

impl ModbusFlexWriterNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let cfg = ModbusFlexWriterConfig::deserialize(&config.rest)?;
        Ok(Box::new(ModbusFlexWriterNode { base: base_node, config: cfg }))
    }
}

#[async_trait]
impl FlowNodeBehavior for ModbusFlexWriterNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> =
            match resolve_modbus_config(self.flow().as_ref(), &self.config.config_node).await {
                Ok(n) => n,
                Err(e) => {
                    log::error!("[modbus-flex-writer:{}] {}", self.name(), e);
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

                    let (address, fc, payload) = {
                        let guard = msg.read().await;
                        let address = guard.get("address").and_then(|v| v.as_f64()).map(|f| f as u16).unwrap_or(0);
                        let fc = guard
                            .get("functionCode")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "writeSingleRegister".to_string());
                        let payload = guard.get("payload").cloned();
                        (address, fc, payload)
                    };

                    let mut conn = cfg_inner.connection.lock().await;

                    let write_result = match fc.as_str() {
                        "writeSingleCoil" => {
                            let val = payload
                                .and_then(|v| v.as_bool())
                                .ok_or_else(|| anyhow::anyhow!("payload must be boolean for writeSingleCoil"))?;
                            conn.write_single_coil(address, val).await?;
                            Variant::Bool(val)
                        }
                        "writeMultipleCoils" => {
                            let arr = payload
                                .and_then(|v| v.as_array().map(|a| a.iter().cloned().collect::<Vec<_>>()))
                                .ok_or_else(|| anyhow::anyhow!("payload must be array of booleans"))?;
                            let coils: Vec<bool> = arr
                                .iter()
                                .enumerate()
                                .map(|(i, v)| {
                                    v.as_bool().ok_or_else(|| anyhow::anyhow!("payload[{}] is not boolean", i))
                                })
                                .collect::<crate::Result<Vec<_>>>()?;
                            let count = coils.len();
                            conn.write_multiple_coils(address, &coils).await?;
                            Variant::from(count as i64)
                        }
                        "writeMultipleRegisters" => {
                            let pv = payload.ok_or_else(|| anyhow::anyhow!("payload required"))?;
                            let words = if let Some(arr) = pv.as_array() {
                                let mut all = Vec::new();
                                for item in arr {
                                    all.extend(node.config.data_type.variant_to_words(item)?);
                                }
                                all
                            } else {
                                node.config.data_type.variant_to_words(&pv)?
                            };
                            let count = words.len();
                            conn.write_multiple_registers(address, &words).await?;
                            Variant::from(count as i64)
                        }
                        _ => {
                            let pv = payload.ok_or_else(|| anyhow::anyhow!("payload must be numeric"))?;
                            let words = node.config.data_type.variant_to_words(&pv)?;
                            if words.len() == 1 {
                                conn.write_single_register(address, words[0]).await?;
                            } else {
                                conn.write_multiple_registers(address, &words).await?;
                            }
                            Variant::from(words.len() as i64)
                        }
                    };
                    drop(conn);

                    {
                        let mut guard = msg.write().await;
                        guard.set("payload".to_string(), write_result);
                        guard.set(
                            "modbus".to_string(),
                            Variant::from(serde_json::json!({
                                "functionCode": fc,
                                "address": address,
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
