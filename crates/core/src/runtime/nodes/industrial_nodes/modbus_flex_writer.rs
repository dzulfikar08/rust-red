//! Modbus flex writer node.
//!
//! Serves `modbus-flex-writer` as written by RustRED, and `modbus-flex-write` of
//! `node-red-contrib-modbus`.
//!
//! A contrib node (it names its client in `server`) takes the request from `msg.payload`, an
//! object `{ fc, unitid, address, quantity, value }` with `fc` 5, 6, 15 or 16, or the same as
//! JSON text, and sends two messages: output 1 with the request in `msg.payload`, output 2 with
//! the device's acknowledgement in `msg.payload`. A RustRED node takes `msg.address`,
//! `msg.functionCode` and the value in `msg.payload`.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::modbus_config::{ModbusConfigNode, ModbusDataType, downcast_modbus_config, resolve_modbus_config};
use super::modbus_contrib::{self as contrib, ContribOptions, FlexRequest};

#[derive(Deserialize, Debug, Clone)]
struct ModbusFlexWriterConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    #[serde(default, rename = "dataType")]
    data_type: ModbusDataType,
    #[serde(flatten)]
    options: ContribOptions,
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
        let rest = if contrib::is_contrib(&config.rest) {
            contrib::translate_flex(&config.rest)?
        } else {
            config.rest.clone()
        };
        let cfg = ModbusFlexWriterConfig::deserialize(&rest)?;
        Ok(Box::new(ModbusFlexWriterNode { base: base_node, config: cfg }))
    }

    async fn status(&self, fill: StatusFill, shape: StatusShape, text: String, cancel: CancellationToken) {
        self.report_status(StatusObject { fill: Some(fill), shape: Some(shape), text: Some(text) }, cancel).await;
    }

    /// Contrib node: the request is in `msg.payload`.
    async fn write_contrib(&self, config_node: &ModbusConfigNode, msg: MsgHandle, cancel: CancellationToken) {
        let input = msg.read().await.clone();

        let result = async {
            let payload = input.get("payload").ok_or_else(|| anyhow::anyhow!("Invalid message on input."))?;
            let request = FlexRequest::parse(payload, &[5, 6, 15, 16], false)?;
            let value = request.value.clone().ok_or_else(|| anyhow::anyhow!("msg.payload.value is missing"))?;
            let ack = {
                let mut conn = config_node.connection.lock().await;
                conn.write_raw(request.unit.or(self.config.options.unit_id), request.fc, request.address, &value)
                    .await?
            };
            Ok::<_, anyhow::Error>((request, ack))
        }
        .await;

        match result {
            Ok((request, ack)) => {
                let topic = input
                    .get("topic")
                    .and_then(|t| t.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| self.id().to_string());
                let base = if self.config.options.keep_msg_properties { input } else { Msg::default() };
                let info = contrib::request_object(
                    request.unit,
                    request.fc,
                    request.address,
                    request.quantity,
                    request.value.as_ref(),
                );
                let (first, second) = contrib::write_outputs(base, &topic, info, ack);
                if let Err(e) = contrib::send_pair(self, first, second, cancel.child_token()).await {
                    log::warn!("[modbus-flex-writer:{}] Cannot send the result: {e}", self.name());
                }
                self.status(
                    StatusFill::Green,
                    StatusShape::Dot,
                    format!("{} @ FC{}", request.address, request.fc),
                    cancel.child_token(),
                )
                .await;
            }
            Err(e) => {
                log::warn!("[modbus-flex-writer:{}] {e}", self.name());
                if self.config.options.empty_msg_on_fail {
                    let (first, second) = contrib::failure_outputs(input, &e);
                    if let Err(e) = contrib::send_pair(self, first, second, cancel.child_token()).await {
                        log::warn!("[modbus-flex-writer:{}] Cannot send the failure message: {e}", self.name());
                    }
                }
                self.status(StatusFill::Red, StatusShape::Ring, e.to_string(), cancel.child_token()).await;
            }
        }
    }

    /// RustRED node: `msg.address`, `msg.functionCode` and the value in `msg.payload`.
    async fn write_native(&self, config_node: &ModbusConfigNode, msg: MsgHandle, cancel: CancellationToken) {
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

        let result: crate::Result<Variant> = async {
            let mut conn = config_node.connection.lock().await;
            conn.select_unit(self.config.options.unit_id);
            Ok(match fc.as_str() {
                "writeSingleCoil" => {
                    let val = payload
                        .and_then(|v| v.as_bool())
                        .ok_or_else(|| anyhow::anyhow!("payload must be boolean for writeSingleCoil"))?;
                    conn.write_single_coil(address, val).await?;
                    Variant::Bool(val)
                }
                "writeMultipleCoils" => {
                    let arr = payload
                        .and_then(|v| v.as_array().map(|a| a.to_vec()))
                        .ok_or_else(|| anyhow::anyhow!("payload must be array of booleans"))?;
                    let coils: Vec<bool> = arr
                        .iter()
                        .enumerate()
                        .map(|(i, v)| v.as_bool().ok_or_else(|| anyhow::anyhow!("payload[{}] is not boolean", i)))
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
                            all.extend(self.config.data_type.variant_to_words(item)?);
                        }
                        all
                    } else {
                        self.config.data_type.variant_to_words(&pv)?
                    };
                    let count = words.len();
                    conn.write_multiple_registers(address, &words).await?;
                    Variant::from(count as i64)
                }
                _ => {
                    let pv = payload.ok_or_else(|| anyhow::anyhow!("payload must be numeric"))?;
                    let words = self.config.data_type.variant_to_words(&pv)?;
                    if words.len() == 1 {
                        conn.write_single_register(address, words[0]).await?;
                    } else {
                        conn.write_multiple_registers(address, &words).await?;
                    }
                    Variant::from(words.len() as i64)
                }
            })
        }
        .await;

        match result {
            Ok(written) => {
                {
                    let mut guard = msg.write().await;
                    guard.set("payload".to_string(), written);
                    guard.set(
                        "modbus".to_string(),
                        Variant::from(serde_json::json!({
                            "functionCode": fc,
                            "address": address,
                        })),
                    );
                }
                self.status(StatusFill::Green, StatusShape::Dot, format!("{address} @ {fc}"), cancel.child_token())
                    .await;
            }
            Err(e) => {
                log::warn!("[modbus-flex-writer:{}] Write error: {e}", self.name());
                msg.write().await.set("error".to_string(), Variant::String(e.to_string()));
                self.status(StatusFill::Red, StatusShape::Ring, e.to_string(), cancel.child_token()).await;
            }
        }
        if let Err(e) = self.fan_out_one(Envelope { port: 0, msg }, cancel).await {
            log::warn!("[modbus-flex-writer:{}] Cannot send the result: {e}", self.name());
        }
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
                    self.status(StatusFill::Red, StatusShape::Ring, e.to_string(), stop_token.clone()).await;
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
                    match downcast_modbus_config(&cfg) {
                        Ok(cfg_inner) if node.config.options.contrib => {
                            node.write_contrib(cfg_inner, msg, cancel.child_token()).await
                        }
                        Ok(cfg_inner) => node.write_native(cfg_inner, msg, cancel.child_token()).await,
                        Err(e) => {
                            log::error!("[modbus-flex-writer:{}] {}", node.name(), e);
                            msg.write().await.set("error".to_string(), Variant::String(e.to_string()));
                        }
                    }
                    Ok(())
                }
            })
            .await;
        }
    }
}
