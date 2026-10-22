//! Modbus read node.
//!
//! Serves `modbus read`, and the `modbus-read` and `modbus-getter` nodes of
//! `node-red-contrib-modbus` (see `modbus_contrib`).
//!
//! A `modbus-read` node polls at its rate and sends two messages per read: output 1 with the
//! values in `msg.payload` and the raw response in `msg.responseBuffer`, output 2 with the raw
//! response in `msg.payload`. A `modbus-getter` node does the same when a message arrives instead
//! of polling. A `modbus read` node keeps its own shape: `msg.payload` holds the values and
//! `msg.modbus` the request, on its single output.

use std::sync::Arc;
use std::time::Duration;

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
use super::modbus_contrib::{self as contrib, ContribOptions};

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
    #[serde(flatten)]
    options: ContribOptions,
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
        let rest = if contrib::is_contrib(&config.rest) {
            contrib::translate_read(&config.rest, config.type_name != "modbus-getter")?
        } else {
            config.rest.clone()
        };
        let read_config = ModbusReadConfig::deserialize(&rest)?;
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

    async fn perform_read(&self, config_node: &ModbusConfigNode) -> crate::Result<(Variant, ModbusRaw)> {
        let count = self.effective_register_count()?;
        let fc = contrib::function_code(&self.config.function_code);
        let raw = {
            let mut conn = config_node.connection.lock().await;
            conn.read_raw(self.config.options.unit_id, fc, self.config.address, count).await?
        };
        let values = match &raw {
            ModbusRaw::Bits(_) => raw.to_variant(),
            ModbusRaw::Words(words) => Variant::from(self.config.data_type.convert_batch(words)?),
        };
        Ok((values, raw))
    }

    fn request_info(&self) -> Variant {
        Variant::from(serde_json::json!({
            "functionCode": self.config.function_code,
            "address": self.config.address,
            "quantity": self.config.quantity,
        }))
    }

    fn status_text(&self) -> String {
        format!("{} @ {}", self.config.address, self.config.function_code)
    }

    async fn status(&self, fill: StatusFill, shape: StatusShape, text: String, cancel: CancellationToken) {
        self.report_status(StatusObject { fill: Some(fill), shape: Some(shape), text: Some(text) }, cancel).await;
    }

    /// With `emptyMsgOnFail`, the contrib nodes send an empty payload and the error on both outputs.
    async fn send_failure(&self, base: Msg, error: &anyhow::Error, cancel: CancellationToken) {
        if !self.config.options.empty_msg_on_fail {
            return;
        }
        let (first, second) = contrib::failure_outputs(base, error);
        if let Err(e) = contrib::send_pair(self, first, second, cancel).await {
            log::warn!("[modbus-read:{}] Cannot send the failure message: {e}", self.name());
        }
    }

    /// One poll of a `modbus-read` node, or of a `modbus read` node with a poll rate.
    async fn poll_once(&self, config_node: &ModbusConfigNode, cancel: CancellationToken) {
        match self.perform_read(config_node).await {
            Ok((values, raw)) => {
                let sent = if self.config.options.contrib {
                    let topic = self.config.options.topic.clone().unwrap_or_else(|| "polling".to_string());
                    let request = contrib::request_object(
                        self.config.options.unit_id,
                        contrib::function_code(&self.config.function_code),
                        self.config.address,
                        self.config.quantity,
                        None,
                    );
                    // The polling message of the contrib node, kept in `msg.input`.
                    let input = Variant::from([
                        ("topic", Variant::String(topic.clone())),
                        ("from", Variant::String(self.name().to_string())),
                        ("payload", request),
                    ]);
                    let (mut first, mut second) = contrib::read_outputs(Msg::default(), &topic, &raw, Variant::Null);
                    for msg in [&mut first, &mut second] {
                        msg.remove("modbusRequest");
                        msg.set("input".to_string(), input.clone());
                        msg.set("sendingNodeId".to_string(), Variant::String(self.id().to_string()));
                    }
                    contrib::send_pair(self, first, second, cancel.child_token()).await
                } else {
                    let mut msg = Msg::default();
                    msg.set("payload".to_string(), values);
                    msg.set(
                        "topic".to_string(),
                        Variant::String(self.config.options.topic.clone().unwrap_or_else(|| "modbus".to_string())),
                    );
                    msg.set("modbus".to_string(), self.request_info());
                    self.fan_out_one(Envelope { port: 0, msg: MsgHandle::new(msg) }, cancel.child_token()).await
                };
                if let Err(e) = sent {
                    log::warn!("[modbus-read:{}] Cannot send the result: {e}", self.name());
                }
                self.status(StatusFill::Green, StatusShape::Dot, self.status_text(), cancel.child_token()).await;
            }
            Err(e) => {
                log::warn!("[modbus-read:{}] Read error: {e}", self.name());
                self.send_failure(Msg::default(), &e, cancel.child_token()).await;
                self.status(StatusFill::Red, StatusShape::Ring, e.to_string(), cancel.child_token()).await;
            }
        }
    }

    /// One read triggered by an incoming message.
    async fn read_for_message(&self, config_node: &ModbusConfigNode, msg: MsgHandle, cancel: CancellationToken) {
        let result = self.perform_read(config_node).await;

        if self.config.options.contrib {
            let input = msg.read().await.clone();
            match result {
                Ok((_, raw)) => {
                    let topic = input
                        .get("topic")
                        .and_then(|t| t.as_str())
                        .map(str::to_string)
                        .unwrap_or_else(|| self.id().to_string());
                    let base = if self.config.options.keep_msg_properties { input } else { Msg::default() };
                    let request = contrib::request_object(
                        self.config.options.unit_id,
                        contrib::function_code(&self.config.function_code),
                        self.config.address,
                        self.config.quantity,
                        None,
                    );
                    let (first, second) = contrib::read_outputs(base, &topic, &raw, request);
                    if let Err(e) = contrib::send_pair(self, first, second, cancel.child_token()).await {
                        log::warn!("[modbus-read:{}] Cannot send the result: {e}", self.name());
                    }
                    self.status(StatusFill::Green, StatusShape::Dot, self.status_text(), cancel.child_token()).await;
                }
                Err(e) => {
                    log::warn!("[modbus-read:{}] Read error: {e}", self.name());
                    self.send_failure(input, &e, cancel.child_token()).await;
                    self.status(StatusFill::Red, StatusShape::Ring, e.to_string(), cancel.child_token()).await;
                }
            }
            return;
        }

        match result {
            Ok((values, _)) => {
                {
                    let mut guard = msg.write().await;
                    guard.set("payload".to_string(), values);
                    guard.set("modbus".to_string(), self.request_info());
                }
                self.status(StatusFill::Green, StatusShape::Dot, self.status_text(), cancel.child_token()).await;
            }
            Err(e) => {
                log::warn!("[modbus-read:{}] Read error: {e}", self.name());
                msg.write().await.set("error".to_string(), Variant::String(e.to_string()));
                self.status(StatusFill::Red, StatusShape::Ring, e.to_string(), cancel.child_token()).await;
            }
        }
        if let Err(e) = self.fan_out_one(Envelope { port: 0, msg }, cancel.child_token()).await {
            log::warn!("[modbus-read:{}] Cannot send the result: {e}", self.name());
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
                self.status(StatusFill::Red, StatusShape::Ring, e.to_string(), stop_token.clone()).await;
                stop_token.cancelled().await;
                return;
            }
        };

        // If polling configured, spawn a poll loop
        let poll_handle = if let Some(interval_ms) = self.config.effective_poll_interval_ms().filter(|ms| *ms > 0) {
            let this = self.clone();
            let cfg = config_node.clone();
            let cancel = stop_token.child_token();
            Some(tokio::spawn(async move {
                let period = Duration::from_millis(interval_ms);
                // A contrib node waits one period before its first read, and `delayOnStart` waits longer.
                let first_tick = if this.config.options.contrib {
                    let delay = if this.config.options.delay_on_start {
                        Duration::from_secs(this.config.options.start_delay_time.unwrap_or(10))
                    } else {
                        Duration::ZERO
                    };
                    tokio::time::Instant::now() + delay + period
                } else {
                    tokio::time::Instant::now()
                };
                let mut interval = tokio::time::interval_at(first_tick, period);
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                loop {
                    tokio::select! {
                        _ = cancel.cancelled() => break,
                        _ = interval.tick() => {
                            match downcast_modbus_config(&cfg) {
                                Ok(cfg_inner) => this.poll_once(cfg_inner, cancel.child_token()).await,
                                Err(e) => log::error!("[modbus-read:{}] {}", this.name(), e),
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
                    match downcast_modbus_config(&cfg) {
                        Ok(cfg_inner) => node.read_for_message(cfg_inner, msg, cancel.child_token()).await,
                        Err(e) => {
                            log::error!("[modbus-read:{}] {}", node.name(), e);
                            msg.write().await.set("error".to_string(), Variant::String(e.to_string()));
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
