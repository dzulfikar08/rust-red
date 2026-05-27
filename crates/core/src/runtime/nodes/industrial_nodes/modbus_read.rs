use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::modbus_config::{ModbusConfigNode, ModbusDataType, downcast_modbus_config, resolve_modbus_config};

#[derive(Deserialize, Debug, Clone)]
struct ModbusReadConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    #[serde(default = "default_fc", rename = "functionCode")]
    function_code: String,
    #[serde(default)]
    address: u16,
    #[serde(default = "default_quantity")]
    quantity: u16,
    /// Optional data type for register-to-value conversion.
    /// Only applies to register-based function codes (FC3/FC4).
    /// Defaults to "uint16" (raw register words).
    #[serde(default, rename = "dataType")]
    data_type: ModbusDataType,
    #[serde(default, rename = "pollIntervalMs")]
    poll_interval_ms: Option<u64>,
    #[serde(default, rename = "pollRate")]
    poll_rate: Option<u64>,
    #[serde(default, rename = "pollRateUnit")]
    poll_rate_unit: Option<String>,
}

impl ModbusReadConfig {
    fn effective_poll_interval_ms(&self) -> Option<u64> {
        if let Some(rate) = self.poll_rate {
            let unit = self.poll_rate_unit.as_deref().unwrap_or("ms");
            let multiplier = match unit {
                "s" => 1000,
                "min" => 60_000,
                "hr" => 3_600_000,
                _ => 1, // "ms"
            };
            Some(rate * multiplier)
        } else {
            self.poll_interval_ms
        }
    }
}

fn default_fc() -> String {
    "readHoldingRegisters".to_string()
}
fn default_quantity() -> u16 {
    1
}

#[derive(Debug)]
#[flow_node("modbus read", red_name = "modbus-read", module = "node-red")]
struct ModbusReadNode {
    base: BaseFlowNodeState,
    config: ModbusReadConfig,
}

impl ModbusReadNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let read_config = ModbusReadConfig::deserialize(&config.rest)?;
        Ok(Box::new(ModbusReadNode { base: base_node, config: read_config }))
    }

    async fn resolve_config_node(&self) -> crate::Result<Arc<dyn GlobalNodeBehavior>> {
        resolve_modbus_config(self.flow().as_ref(), &self.config.config_node).await
    }

    /// Compute the actual number of registers to request, accounting for the
    /// data-type width. For coil/discrete operations this is simply `quantity`.
    fn effective_register_count(&self) -> crate::Result<u16> {
        match self.config.function_code.as_str() {
            "readCoils" | "readDiscreteInputs" => Ok(self.config.quantity),
            _ => {
                // Register-based: quantity values * registers-per-value
                let width = self.config.data_type.register_count();
                self.config.quantity.checked_mul(width).ok_or_else(|| {
                    anyhow::anyhow!("quantity ({}) * data_type width ({}) overflows u16", self.config.quantity, width)
                })
            }
        }
    }

    async fn perform_read(&self, config_node: &ModbusConfigNode) -> crate::Result<Variant> {
        let mut conn = config_node.connection.lock().await;
        let address = self.config.address;
        let reg_count = self.effective_register_count()?;
        match self.config.function_code.as_str() {
            "readCoils" => {
                // FC1
                let values: Vec<bool> = conn.read_coils(address, self.config.quantity).await?;
                Ok(Variant::from(values.iter().map(|&b| Variant::from(b)).collect::<Vec<_>>()))
            }
            "readDiscreteInputs" => {
                // FC2
                let values: Vec<bool> = conn.read_discrete_inputs(address, self.config.quantity).await?;
                Ok(Variant::from(values.iter().map(|&b| Variant::from(b)).collect::<Vec<_>>()))
            }
            "readInputRegisters" => {
                // FC4
                let words: Vec<u16> = conn.read_input_registers(address, reg_count).await?;
                let values = self.config.data_type.convert_batch(&words)?;
                Ok(Variant::from(values))
            }
            _ => {
                // FC3 – readHoldingRegisters (default)
                let words: Vec<u16> = conn.read_holding_registers(address, reg_count).await?;
                let values = self.config.data_type.convert_batch(&words)?;
                Ok(Variant::from(values))
            }
        }
    }
}

#[async_trait]
impl FlowNodeBehavior for ModbusReadNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> = match self.resolve_config_node().await {
            Ok(n) => n,
            Err(e) => {
                log::error!("[modbus-read:{}] {}", self.name(), e);
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

        // If polling configured, spawn a poll loop
        let poll_handle = if let Some(interval_ms) = self.config.effective_poll_interval_ms() {
            let this = self.clone();
            let cfg = config_node.clone();
            let cancel = stop_token.child_token();
            Some(tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
                loop {
                    tokio::select! {
                        _ = cancel.cancelled() => break,
                        _ = interval.tick() => {
                            let cfg_inner = match downcast_modbus_config(&cfg) {
                                Ok(c) => c,
                                Err(e) => {
                                    log::error!("[modbus-read:{}] {}", this.name(), e);
                                    continue;
                                }
                            };
                            match this.perform_read(cfg_inner).await {
                                Ok(payload) => {
                                    let msg_body = Variant::from([
                                        ("payload", payload),
                                        ("topic", Variant::String("modbus".into())),
                                        ("modbus", Variant::from(serde_json::json!({
                                            "functionCode": this.config.function_code,
                                            "address": this.config.address,
                                            "quantity": this.config.quantity,
                                        }))),
                                    ]);
                                    let envelope = Envelope {
                                        port: 0,
                                        msg: MsgHandle::with_body(msg_body),
                                    };
                                    let _ = this.fan_out_one(envelope, cancel.child_token()).await;
                                    let _ = this.report_status(StatusObject {
                                        fill: Some(StatusFill::Green),
                                        shape: Some(StatusShape::Dot),
                                        text: Some(format!("{} @ {}", this.config.address, this.config.function_code)),
                                    }, cancel.child_token()).await;
                                }
                                Err(e) => {
                                    log::warn!("[modbus-read:{}] Read error: {}", this.name(), e);
                                    let _ = this.report_status(StatusObject {
                                        fill: Some(StatusFill::Red),
                                        shape: Some(StatusShape::Ring),
                                        text: Some(format!("{}", e)),
                                    }, cancel.child_token()).await;
                                }
                            }
                        }
                    }
                }
            }))
        } else {
            None
        };

        // Also handle input messages for one-shot reads
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
                    match node.perform_read(cfg_inner).await {
                        Ok(payload) => {
                            let mut guard = msg.write().await;
                            guard.set("payload".to_string(), payload);
                            guard.set(
                                "modbus".to_string(),
                                Variant::from(serde_json::json!({
                                    "functionCode": node.config.function_code,
                                    "address": node.config.address,
                                    "quantity": node.config.quantity,
                                })),
                            );
                            drop(guard);
                            node.report_status(
                                StatusObject {
                                    fill: Some(StatusFill::Green),
                                    shape: Some(StatusShape::Dot),
                                    text: Some(format!("{} @ {}", node.config.address, node.config.function_code)),
                                },
                                cancel.child_token(),
                            )
                            .await;
                        }
                        Err(e) => {
                            log::warn!("[modbus-read:{}] Read error: {}", node.name(), e);
                            let mut guard = msg.write().await;
                            guard.set("error".to_string(), Variant::String(e.to_string()));
                            drop(guard);
                            node.report_status(
                                StatusObject {
                                    fill: Some(StatusFill::Red),
                                    shape: Some(StatusShape::Ring),
                                    text: Some(format!("{}", e)),
                                },
                                cancel.child_token(),
                            )
                            .await;
                        }
                    }
                    Ok(())
                }
            })
            .await;
        }

        if let Some(handle) = poll_handle {
            handle.abort();
        }
    }
}
