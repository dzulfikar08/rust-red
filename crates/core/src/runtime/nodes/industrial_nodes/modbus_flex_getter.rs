//! Modbus flex getter node.
//!
//! Serves `modbus-flex-getter` as written by `node-red-contrib-modbus` and by RustRED.
//!
//! A contrib node (it names its client in `server`) takes the request from `msg.payload`, an
//! object `{ fc, unitid, address, quantity }` or the same as JSON text, and sends two messages:
//! output 1 with the values in `msg.payload` and the raw response in `msg.responseBuffer`,
//! output 2 with the raw response in `msg.payload`. A RustRED node takes `msg.address`,
//! `msg.quantity` and `msg.functionCode` and sends the values in `msg.payload`.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::modbus_config::{
    ModbusConfigNode, ModbusDataType, ModbusRaw, downcast_modbus_config, resolve_modbus_config,
};
use super::modbus_contrib::{self as contrib, ContribOptions, FlexRequest};

#[derive(Deserialize, Debug, Clone)]
struct ModbusFlexGetterConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    #[serde(default, rename = "dataType")]
    data_type: ModbusDataType,
    #[serde(flatten)]
    options: ContribOptions,
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
        let rest = if contrib::is_contrib(&config.rest) {
            contrib::translate_flex(&config.rest)?
        } else {
            config.rest.clone()
        };
        let cfg = ModbusFlexGetterConfig::deserialize(&rest)?;
        Ok(Box::new(ModbusFlexGetterNode { base: base_node, config: cfg }))
    }

    fn extract_u16(guard: &Msg, key: &str, default: u16) -> u16 {
        guard.get(key).and_then(|v| v.as_f64()).map(|f| f as u16).unwrap_or(default)
    }

    fn extract_string(guard: &Msg, key: &str, default: &str) -> String {
        guard.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| default.to_string())
    }

    async fn status(&self, fill: StatusFill, shape: StatusShape, text: String, cancel: CancellationToken) {
        self.report_status(StatusObject { fill: Some(fill), shape: Some(shape), text: Some(text) }, cancel).await;
    }

    /// Contrib node: the request is in `msg.payload`.
    async fn get_contrib(&self, config_node: &ModbusConfigNode, msg: MsgHandle, cancel: CancellationToken) {
        let input = msg.read().await.clone();

        let result = async {
            let payload = input.get("payload").ok_or_else(|| anyhow::anyhow!("Invalid message on input."))?;
            let request = FlexRequest::parse(payload, &[1, 2, 3, 4], true)?;
            let raw = {
                let mut conn = config_node.connection.lock().await;
                conn.read_raw(
                    request.unit.or(self.config.options.unit_id),
                    request.fc,
                    request.address,
                    request.quantity,
                )
                .await?
            };
            Ok::<_, anyhow::Error>((request, raw))
        }
        .await;

        match result {
            Ok((request, raw)) => {
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
                let (first, second) = contrib::read_outputs(base, &topic, &raw, info);
                if let Err(e) = contrib::send_pair(self, first, second, cancel.child_token()).await {
                    log::warn!("[modbus-flex-getter:{}] Cannot send the result: {e}", self.name());
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
                log::warn!("[modbus-flex-getter:{}] {e}", self.name());
                if self.config.options.empty_msg_on_fail {
                    let (first, second) = contrib::failure_outputs(input, &e);
                    if let Err(e) = contrib::send_pair(self, first, second, cancel.child_token()).await {
                        log::warn!("[modbus-flex-getter:{}] Cannot send the failure message: {e}", self.name());
                    }
                }
                self.status(StatusFill::Red, StatusShape::Ring, e.to_string(), cancel.child_token()).await;
            }
        }
    }

    /// RustRED node: the request is in `msg.address`, `msg.quantity` and `msg.functionCode`.
    async fn get_native(&self, config_node: &ModbusConfigNode, msg: MsgHandle, cancel: CancellationToken) {
        let (address, quantity, fc) = {
            let guard = msg.read().await;
            (
                Self::extract_u16(&guard, "address", 0),
                Self::extract_u16(&guard, "quantity", 1),
                Self::extract_string(&guard, "functionCode", "readHoldingRegisters"),
            )
        };

        let result: crate::Result<Variant> = async {
            let reg_count = match fc.as_str() {
                "readCoils" | "readDiscreteInputs" => quantity,
                _ => quantity
                    .checked_mul(self.config.data_type.register_count())
                    .ok_or_else(|| anyhow::anyhow!("quantity * data type width overflows"))?,
            };
            let raw = {
                let mut conn = config_node.connection.lock().await;
                conn.read_raw(self.config.options.unit_id, contrib::function_code(&fc), address, reg_count).await?
            };
            Ok(match &raw {
                ModbusRaw::Bits(_) => raw.to_variant(),
                ModbusRaw::Words(words) => Variant::from(self.config.data_type.convert_batch(words)?),
            })
        }
        .await;

        match result {
            Ok(values) => {
                {
                    let mut guard = msg.write().await;
                    guard.set("payload".to_string(), values);
                    guard.set(
                        "modbus".to_string(),
                        Variant::from(serde_json::json!({
                            "functionCode": fc,
                            "address": address,
                            "quantity": quantity,
                        })),
                    );
                }
                self.status(StatusFill::Green, StatusShape::Dot, format!("{address} @ {fc}"), cancel.child_token())
                    .await;
            }
            Err(e) => {
                log::warn!("[modbus-flex-getter:{}] Read error: {e}", self.name());
                msg.write().await.set("error".to_string(), Variant::String(e.to_string()));
                self.status(StatusFill::Red, StatusShape::Ring, e.to_string(), cancel.child_token()).await;
            }
        }
        if let Err(e) = self.fan_out_one(Envelope { port: 0, msg }, cancel).await {
            log::warn!("[modbus-flex-getter:{}] Cannot send the result: {e}", self.name());
        }
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
                            node.get_contrib(cfg_inner, msg, cancel.child_token()).await
                        }
                        Ok(cfg_inner) => node.get_native(cfg_inner, msg, cancel.child_token()).await,
                        Err(e) => {
                            log::error!("[modbus-flex-getter:{}] {}", node.name(), e);
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
