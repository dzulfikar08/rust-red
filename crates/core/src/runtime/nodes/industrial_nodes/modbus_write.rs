//! Modbus write node.
//!
//! Serves `modbus write` and the `modbus-write` node of `node-red-contrib-modbus`
//! (see `modbus_contrib`).
//!
//! A `modbus-write` node takes the value from `msg.payload` (a number, a boolean, an array, or
//! an object with a `value`), writes it with the function code of its type, and sends two
//! messages: output 1 with the request in `msg.payload`, output 2 with the device's
//! acknowledgement in `msg.payload`. A `modbus write` node keeps its own shape: `msg.payload`
//! holds the number of values written and `msg.modbus` the request.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::modbus_config::{ModbusConfigNode, ModbusDataType, downcast_modbus_config, resolve_modbus_config};
use super::modbus_contrib::{self as contrib, ContribOptions};

#[derive(Deserialize, Debug, Clone)]
struct ModbusWriteConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    #[serde(rename = "functionCode")]
    #[serde(default = "default_fc")]
    function_code: String,
    #[serde(default)]
    address: u16,
    #[serde(default = "default_quantity")]
    quantity: u16,
    /// Data type for register conversion on write operations.
    /// Only applies to register-based function codes.
    #[serde(default, rename = "dataType")]
    data_type: ModbusDataType,
    #[serde(flatten)]
    options: ContribOptions,
}

fn default_fc() -> String {
    "writeSingleRegister".to_string()
}
fn default_quantity() -> u16 {
    1
}

#[derive(Debug)]
#[flow_node("modbus write", red_name = "modbus-write", module = "node-red")]
struct ModbusWriteNode {
    base: BaseFlowNodeState,
    config: ModbusWriteConfig,
}

/// Values that arrive as text, for instance from an HTTP request: `"true"`, `"false"` or `"1,2,3"`.
fn normalize_text_value(value: &Variant) -> Variant {
    match value {
        Variant::String(s) if s == "true" || s == "false" => Variant::Bool(s == "true"),
        Variant::String(s) if s.contains(',') => serde_json::from_str::<serde_json::Value>(s)
            .or_else(|_| serde_json::from_str::<serde_json::Value>(&format!("[{s}]")))
            .map(Variant::from)
            .unwrap_or_else(|_| value.clone()),
        other => other.clone(),
    }
}

impl ModbusWriteNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let rest = if contrib::is_contrib(&config.rest) {
            contrib::translate_write(&config.rest)?
        } else {
            config.rest.clone()
        };
        let write_config = ModbusWriteConfig::deserialize(&rest)?;
        Ok(Box::new(ModbusWriteNode { base: base_node, config: write_config }))
    }

    async fn resolve_config_node(&self) -> crate::Result<Arc<dyn GlobalNodeBehavior>> {
        resolve_modbus_config(self.flow().as_ref(), &self.config.config_node).await
    }

    async fn status(&self, fill: StatusFill, shape: StatusShape, text: String, cancel: CancellationToken) {
        self.report_status(StatusObject { fill: Some(fill), shape: Some(shape), text: Some(text) }, cancel).await;
    }

    async fn perform_write(&self, config_node: &ModbusConfigNode, msg: &MsgHandle) -> crate::Result<Variant> {
        let mut conn = config_node.connection.lock().await;
        conn.select_unit(self.config.options.unit_id);
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

    /// A `modbus-write` node: write `msg.payload` and send the two contrib messages.
    async fn write_contrib(&self, config_node: &ModbusConfigNode, msg: MsgHandle, cancel: CancellationToken) {
        let input = msg.read().await.clone();
        let fc = contrib::function_code(&self.config.function_code);

        let result = async {
            let payload = input.get("payload").ok_or_else(|| anyhow::anyhow!("msg.payload is missing"))?;
            let value = match payload {
                Variant::Object(object) if object.contains_key("value") => normalize_text_value(&object["value"]),
                other => normalize_text_value(other),
            };
            let mut conn = config_node.connection.lock().await;
            let ack = conn.write_raw(self.config.options.unit_id, fc, self.config.address, &value).await?;
            Ok::<_, anyhow::Error>((value, ack))
        }
        .await;

        match result {
            Ok((value, ack)) => {
                let topic = input
                    .get("topic")
                    .and_then(|t| t.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| self.id().to_string());
                let base = if self.config.options.keep_msg_properties { input } else { Msg::default() };
                let request = contrib::request_object(
                    self.config.options.unit_id,
                    fc,
                    self.config.address,
                    self.config.quantity,
                    Some(&value),
                );
                let (first, second) = contrib::write_outputs(base, &topic, request, ack);
                if let Err(e) = contrib::send_pair(self, first, second, cancel.child_token()).await {
                    log::warn!("[modbus-write:{}] Cannot send the result: {e}", self.name());
                }
                self.status(
                    StatusFill::Green,
                    StatusShape::Dot,
                    format!("{} @ {}", self.config.address, fc),
                    cancel.child_token(),
                )
                .await;
            }
            Err(e) => {
                log::warn!("[modbus-write:{}] Write error: {e}", self.name());
                if self.config.options.empty_msg_on_fail {
                    let (first, second) = contrib::failure_outputs(input, &e);
                    if let Err(e) = contrib::send_pair(self, first, second, cancel.child_token()).await {
                        log::warn!("[modbus-write:{}] Cannot send the failure message: {e}", self.name());
                    }
                }
                self.status(StatusFill::Red, StatusShape::Ring, e.to_string(), cancel.child_token()).await;
            }
        }
    }

    async fn write_native(&self, config_node: &ModbusConfigNode, msg: MsgHandle, cancel: CancellationToken) {
        match self.perform_write(config_node, &msg).await {
            Ok(value) => {
                let mut guard = msg.write().await;
                guard.set("payload".to_string(), value);
                guard.set(
                    "modbus".to_string(),
                    Variant::from(serde_json::json!({
                        "functionCode": self.config.function_code,
                        "address": self.config.address,
                    })),
                );
            }
            Err(e) => {
                log::warn!("[modbus-write:{}] Write error: {}", self.name(), e);
                msg.write().await.set("error".to_string(), Variant::String(e.to_string()));
            }
        }
        if let Err(e) = self.fan_out_one(Envelope { port: 0, msg }, cancel).await {
            log::warn!("[modbus-write:{}] Cannot send the result: {e}", self.name());
        }
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
                            log::error!("[modbus-write:{}] {}", node.name(), e);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_values_are_normalized() {
        assert_eq!(normalize_text_value(&Variant::String("true".into())), Variant::Bool(true));
        assert_eq!(normalize_text_value(&Variant::String("1,2,3".into())), Variant::from(serde_json::json!([1, 2, 3])));
        assert_eq!(normalize_text_value(&Variant::String("abc".into())), Variant::String("abc".into()));
    }
}
