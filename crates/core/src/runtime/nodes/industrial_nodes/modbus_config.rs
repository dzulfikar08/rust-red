use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio_modbus::client::tcp;
use tokio_modbus::prelude::*;

use crate::runtime::engine::Engine;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct ModbusConfig {
    #[serde(default = "default_transport")]
    transport: String,
    host: String,
    #[serde(default = "default_tcp_port")]
    port: u16,
    #[serde(default = "default_unit_id")]
    unit_id: u8,
    #[serde(default = "default_timeout_ms")]
    timeout_ms: u64,
    // Serial RTU (round-trip until runtime support)
    serial_port: Option<String>,
    baud_rate: Option<u32>,
    data_bits: Option<String>,
    stop_bits: Option<String>,
    parity: Option<String>,
    // Queue options (round-trip until runtime support)
    parallel_unit_ids: Option<bool>,
    queue_log_enabled: Option<bool>,
    buffer_commands: Option<bool>,
    command_delay: Option<u64>,
    // Connection options (round-trip until runtime support)
    keep_alive: Option<bool>,
    reconnect_timeout: Option<u64>,
    auto_connect: Option<bool>,
}

fn default_transport() -> String {
    "tcp".to_string()
}
fn default_tcp_port() -> u16 {
    502
}
fn default_unit_id() -> u8 {
    1
}
fn default_timeout_ms() -> u64 {
    5000
}

/// Data type conversion helpers for Modbus register values.
///
/// Industrial PLCs store values in 16-bit registers. Multi-byte and
/// floating-point values span two or more consecutive registers. This
/// enum describes how to interpret a sequence of raw u16 words.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ModbusDataType {
    /// Raw unsigned 16-bit word (default).
    UInt16,
    /// Signed 16-bit integer (two's complement).
    Int16,
    /// Unsigned 32-bit integer stored in two registers.
    UInt32,
    /// Signed 32-bit integer stored in two registers.
    Int32,
    /// IEEE-754 single-precision float stored in two registers.
    Float,
    /// Unsigned 64-bit integer stored in four registers.
    UInt64,
    /// Signed 64-bit integer stored in four registers.
    Int64,
    /// IEEE-754 double-precision float stored in four registers.
    Double,
}

impl Default for ModbusDataType {
    fn default() -> Self {
        Self::UInt16
    }
}

impl ModbusDataType {
    /// Number of 16-bit registers required per value.
    pub(crate) fn register_count(&self) -> u16 {
        match self {
            Self::UInt16 | Self::Int16 => 1,
            Self::UInt32 | Self::Int32 | Self::Float => 2,
            Self::UInt64 | Self::Int64 | Self::Double => 4,
        }
    }

    /// Convert a slice of raw u16 register values into a `Variant`.
    ///
    /// `words` must contain at least `self.register_count()` elements.
    /// Only the first value is converted (multi-value conversion is done
    /// at the node level by advancing the slice).
    pub(crate) fn words_to_variant(&self, words: &[u16]) -> crate::Result<Variant> {
        match self {
            Self::UInt16 => {
                if words.is_empty() {
                    return Err(anyhow::anyhow!("Not enough registers for UInt16"));
                }
                Ok(Variant::from(words[0] as i64))
            }
            Self::Int16 => {
                if words.is_empty() {
                    return Err(anyhow::anyhow!("Not enough registers for Int16"));
                }
                Ok(Variant::from(words[0] as i16 as i64))
            }
            Self::UInt32 => {
                if words.len() < 2 {
                    return Err(anyhow::anyhow!("Not enough registers for UInt32 (need 2)"));
                }
                // Big-endian / Modbus convention: first register = high word
                let val = ((words[0] as u32) << 16) | (words[1] as u32);
                Ok(Variant::from(val as i64))
            }
            Self::Int32 => {
                if words.len() < 2 {
                    return Err(anyhow::anyhow!("Not enough registers for Int32 (need 2)"));
                }
                let val = ((words[0] as u32) << 16) | (words[1] as u32);
                Ok(Variant::from(val as i32 as i64))
            }
            Self::Float => {
                if words.len() < 2 {
                    return Err(anyhow::anyhow!("Not enough registers for Float (need 2)"));
                }
                let bits = ((words[0] as u32) << 16) | (words[1] as u32);
                let val = f32::from_bits(bits);
                Ok(Variant::from(serde_json::Number::from_f64(val as f64).map_or(Variant::Null, Variant::Number)))
            }
            Self::UInt64 => {
                if words.len() < 4 {
                    return Err(anyhow::anyhow!("Not enough registers for UInt64 (need 4)"));
                }
                let val = ((words[0] as u64) << 48)
                    | ((words[1] as u64) << 32)
                    | ((words[2] as u64) << 16)
                    | (words[3] as u64);
                if val > i64::MAX as u64 {
                    return Err(anyhow::anyhow!(
                        "UInt64 value {} exceeds i64::MAX — JSON number cannot represent it faithfully",
                        val
                    ));
                }
                Ok(Variant::from(val as i64))
            }
            Self::Int64 => {
                if words.len() < 4 {
                    return Err(anyhow::anyhow!("Not enough registers for Int64 (need 4)"));
                }
                let val = ((words[0] as u64) << 48)
                    | ((words[1] as u64) << 32)
                    | ((words[2] as u64) << 16)
                    | (words[3] as u64);
                Ok(Variant::from(val as i64))
            }
            Self::Double => {
                if words.len() < 4 {
                    return Err(anyhow::anyhow!("Not enough registers for Double (need 4)"));
                }
                let bits = ((words[0] as u64) << 48)
                    | ((words[1] as u64) << 32)
                    | ((words[2] as u64) << 16)
                    | (words[3] as u64);
                let val = f64::from_bits(bits);
                Ok(Variant::from(serde_json::Number::from_f64(val).map_or(Variant::Null, Variant::Number)))
            }
        }
    }

    /// Convert a Variant value into raw u16 register words for writing.
    pub(crate) fn variant_to_words(&self, value: &Variant) -> crate::Result<Vec<u16>> {
        let f64_val = value.as_f64().ok_or_else(|| anyhow::anyhow!("Payload must be numeric"))?;
        match self {
            Self::UInt16 => Ok(vec![f64_val as u16]),
            Self::Int16 => Ok(vec![f64_val as i16 as u16]),
            Self::UInt32 => {
                let v = f64_val as u32;
                Ok(vec![(v >> 16) as u16, v as u16])
            }
            Self::Int32 => {
                let v = f64_val as i32 as u32;
                Ok(vec![(v >> 16) as u16, v as u16])
            }
            Self::Float => {
                let bits = (f64_val as f32).to_bits();
                Ok(vec![(bits >> 16) as u16, bits as u16])
            }
            Self::UInt64 | Self::Int64 => {
                let v = f64_val as u64;
                Ok(vec![(v >> 48) as u16, (v >> 32) as u16, (v >> 16) as u16, v as u16])
            }
            Self::Double => {
                let bits = f64_val.to_bits();
                Ok(vec![(bits >> 48) as u16, (bits >> 32) as u16, (bits >> 16) as u16, bits as u16])
            }
        }
    }

    /// Convert a batch of raw u16 registers into a Vec<Variant>, one value per
    /// data-type unit. For example, with `Float` and 6 registers, returns 3 values.
    pub(crate) fn convert_batch(&self, words: &[u16]) -> crate::Result<Vec<Variant>> {
        let stride = self.register_count() as usize;
        if words.len() % stride != 0 {
            return Err(anyhow::anyhow!(
                "Register count {} is not a multiple of data-type width {}",
                words.len(),
                stride
            ));
        }
        words.chunks(stride).map(|chunk| self.words_to_variant(chunk)).collect()
    }
}

pub(crate) struct ModbusConnection {
    context: Option<tokio_modbus::client::Context>,
    config: ModbusConfig,
}

impl std::fmt::Debug for ModbusConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModbusConnection")
            .field("connected", &self.context.is_some())
            .field("config", &self.config)
            .finish()
    }
}

impl ModbusConnection {
    pub(crate) async fn ensure_connected(&mut self) -> crate::Result<()> {
        if self.context.is_some() {
            return Ok(());
        }
        let auto_connect = self.config.auto_connect.unwrap_or(true);
        if !auto_connect {
            return Err(anyhow::anyhow!("Not connected and auto_connect is disabled"));
        }
        if let Some(reconnect_ms) = self.config.reconnect_timeout {
            tokio::time::sleep(std::time::Duration::from_millis(reconnect_ms)).await;
        }
        self.connect().await
    }

    async fn connect(&mut self) -> crate::Result<()> {
        match self.config.transport.as_str() {
            "tcp" => self.connect_tcp().await,
            #[cfg(feature = "nodes_modbus_serial")]
            "serial" | "rtu" => self.connect_rtu().await,
            #[cfg(not(feature = "nodes_modbus_serial"))]
            "serial" | "rtu" => Err(anyhow::anyhow!("Serial/RTU transport requires the 'nodes_modbus_serial' feature")),
            _ => Err(anyhow::anyhow!("Unsupported transport: '{}'", self.config.transport)),
        }
    }

    async fn connect_tcp(&mut self) -> crate::Result<()> {
        let socket_addr = format!("{}:{}", self.config.host, self.config.port);
        let addr: std::net::SocketAddr =
            socket_addr.parse().map_err(|e: std::net::AddrParseError| anyhow::anyhow!("Invalid address: {e}"))?;

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(self.config.timeout_ms),
            tcp::connect_slave(addr, Slave(self.config.unit_id)),
        )
        .await;

        match result {
            Ok(Ok(mut ctx)) => {
                ctx.set_slave(Slave(self.config.unit_id));
                self.context = Some(ctx);
                log::info!("[modbus-config] Connected to {}", socket_addr);
                Ok(())
            }
            Ok(Err(e)) => {
                log::warn!("[modbus-config] Connection failed to {}: {}", socket_addr, e);
                Err(anyhow::anyhow!("Connection failed: {}", e))
            }
            Err(_) => {
                log::warn!("[modbus-config] Connection timeout to {}", socket_addr);
                Err(anyhow::anyhow!("Connection timeout after {}ms", self.config.timeout_ms))
            }
        }
    }

    #[cfg(feature = "nodes_modbus_serial")]
    async fn connect_rtu(&mut self) -> crate::Result<()> {
        use tokio_modbus::client::rtu;

        let serial_port = self
            .config
            .serial_port
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("serial_port is required for RTU transport"))?;
        let baud_rate = self.config.baud_rate.unwrap_or(9600);

        let builder = tokio_serial::new(serial_port, baud_rate);
        let port = tokio_serial::SerialStream::open(&builder)
            .map_err(|e| anyhow::anyhow!("Failed to open serial port '{}': {}", serial_port, e))?;

        let ctx = rtu::attach_slave(port, Slave(self.config.unit_id));
        self.context = Some(ctx);

        log::info!(
            "[modbus-config] Connected RTU to {} @ {} baud, unit {}",
            serial_port,
            baud_rate,
            self.config.unit_id
        );
        Ok(())
    }

    fn disconnect(&mut self) {
        if self.context.take().is_some() {
            log::info!("[modbus-config] Disconnected");
        }
    }

    /// Attempt to reconnect after a failure. Returns true if reconnection succeeded.
    async fn try_reconnect(&mut self) -> bool {
        self.disconnect();
        if !self.config.auto_connect.unwrap_or(true) {
            return false;
        }
        if let Some(reconnect_ms) = self.config.reconnect_timeout {
            tokio::time::sleep(std::time::Duration::from_millis(reconnect_ms)).await;
        }
        match self.connect().await {
            Ok(()) => true,
            Err(_) => false,
        }
    }

    /// Returns true if the connection appears to be alive.
    pub(crate) fn is_connected(&self) -> bool {
        self.context.is_some()
    }

    /// FC1 – Read Coils (bits).
    pub(crate) async fn read_coils(&mut self, address: u16, quantity: u16) -> crate::Result<Vec<bool>> {
        self.ensure_connected().await?;
        let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        match ctx.read_coils(address, quantity).await {
            Ok(Ok(response)) => Ok(response),
            err => {
                if self.try_reconnect().await {
                    let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
                    ctx.read_coils(address, quantity)
                        .await
                        .map_err(|e| anyhow::anyhow!("Read failed: {}", e))
                        .and_then(|r| r.map_err(|e| anyhow::anyhow!("Read error: {}", e)))
                } else {
                    map_modbus_err(err)
                }
            }
        }
    }

    /// FC2 – Read Discrete Inputs (read-only bits).
    pub(crate) async fn read_discrete_inputs(&mut self, address: u16, quantity: u16) -> crate::Result<Vec<bool>> {
        self.ensure_connected().await?;
        let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        match ctx.read_discrete_inputs(address, quantity).await {
            Ok(Ok(response)) => Ok(response),
            err => {
                if self.try_reconnect().await {
                    let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
                    ctx.read_discrete_inputs(address, quantity)
                        .await
                        .map_err(|e| anyhow::anyhow!("Read discrete inputs failed: {}", e))
                        .and_then(|r| r.map_err(|e| anyhow::anyhow!("Read discrete inputs error: {}", e)))
                } else {
                    map_modbus_err(err)
                }
            }
        }
    }

    /// FC3 – Read Holding Registers.
    pub(crate) async fn read_holding_registers(&mut self, address: u16, quantity: u16) -> crate::Result<Vec<u16>> {
        self.ensure_connected().await?;
        let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        match ctx.read_holding_registers(address, quantity).await {
            Ok(Ok(response)) => Ok(response),
            err => {
                if self.try_reconnect().await {
                    let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
                    ctx.read_holding_registers(address, quantity)
                        .await
                        .map_err(|e| anyhow::anyhow!("Read failed: {}", e))
                        .and_then(|r| r.map_err(|e| anyhow::anyhow!("Read error: {}", e)))
                } else {
                    map_modbus_err(err)
                }
            }
        }
    }

    /// FC4 – Read Input Registers (read-only).
    pub(crate) async fn read_input_registers(&mut self, address: u16, quantity: u16) -> crate::Result<Vec<u16>> {
        self.ensure_connected().await?;
        let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        match ctx.read_input_registers(address, quantity).await {
            Ok(Ok(response)) => Ok(response),
            err => {
                if self.try_reconnect().await {
                    let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
                    ctx.read_input_registers(address, quantity)
                        .await
                        .map_err(|e| anyhow::anyhow!("Read failed: {}", e))
                        .and_then(|r| r.map_err(|e| anyhow::anyhow!("Read error: {}", e)))
                } else {
                    map_modbus_err(err)
                }
            }
        }
    }

    /// FC5 – Write Single Coil.
    pub(crate) async fn write_single_coil(&mut self, address: u16, value: bool) -> crate::Result<()> {
        self.ensure_connected().await?;
        let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        match ctx.write_single_coil(address, value).await {
            Ok(Ok(())) => Ok(()),
            err => {
                if self.try_reconnect().await {
                    let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
                    ctx.write_single_coil(address, value)
                        .await
                        .map_err(|e| anyhow::anyhow!("Write coil failed: {}", e))
                        .and_then(|r| r.map_err(|e| anyhow::anyhow!("Write coil error: {}", e)))
                } else {
                    map_modbus_err_unit(err)
                }
            }
        }
    }

    /// FC6 – Write Single Register.
    pub(crate) async fn write_single_register(&mut self, address: u16, value: u16) -> crate::Result<()> {
        self.ensure_connected().await?;
        let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        match ctx.write_single_register(address, value).await {
            Ok(Ok(())) => Ok(()),
            err => {
                if self.try_reconnect().await {
                    let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
                    ctx.write_single_register(address, value)
                        .await
                        .map_err(|e| anyhow::anyhow!("Write failed: {}", e))
                        .and_then(|r| r.map_err(|e| anyhow::anyhow!("Write error: {}", e)))
                } else {
                    map_modbus_err_unit(err)
                }
            }
        }
    }

    /// FC15 – Write Multiple Coils.
    pub(crate) async fn write_multiple_coils(&mut self, address: u16, values: &[bool]) -> crate::Result<()> {
        self.ensure_connected().await?;
        let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        match ctx.write_multiple_coils(address, values).await {
            Ok(Ok(())) => Ok(()),
            err => {
                if self.try_reconnect().await {
                    let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
                    ctx.write_multiple_coils(address, values)
                        .await
                        .map_err(|e| anyhow::anyhow!("Write multiple coils failed: {}", e))
                        .and_then(|r| r.map_err(|e| anyhow::anyhow!("Write multiple coils error: {}", e)))
                } else {
                    map_modbus_err_unit(err)
                }
            }
        }
    }

    /// FC16 – Write Multiple Registers.
    pub(crate) async fn write_multiple_registers(&mut self, address: u16, values: &[u16]) -> crate::Result<()> {
        self.ensure_connected().await?;
        let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        match ctx.write_multiple_registers(address, values).await {
            Ok(Ok(())) => Ok(()),
            err => {
                if self.try_reconnect().await {
                    let ctx = self.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
                    ctx.write_multiple_registers(address, values)
                        .await
                        .map_err(|e| anyhow::anyhow!("Write failed: {}", e))
                        .and_then(|r| r.map_err(|e| anyhow::anyhow!("Write error: {}", e)))
                } else {
                    map_modbus_err_unit(err)
                }
            }
        }
    }
}

fn map_modbus_err<T, E1: std::fmt::Display, E2: std::fmt::Display>(
    result: Result<Result<T, E1>, E2>,
) -> crate::Result<T> {
    match result {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(anyhow::anyhow!("Modbus protocol error: {}", e)),
        Err(e) => Err(anyhow::anyhow!("Modbus transport error: {}", e)),
    }
}

fn map_modbus_err_unit<E1: std::fmt::Display, E2: std::fmt::Display>(
    result: Result<Result<(), E1>, E2>,
) -> crate::Result<()> {
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(anyhow::anyhow!("Modbus protocol error: {}", e)),
        Err(e) => Err(anyhow::anyhow!("Modbus transport error: {}", e)),
    }
}

use super::modbus_queue::ModbusRequestQueue;

#[derive(Debug)]
#[global_node("modbus-config", red_name = "modbus-config", module = "node-red")]
pub(crate) struct ModbusConfigNode {
    base: BaseGlobalNodeState,
    #[allow(dead_code)]
    config: ModbusConfig,
    pub(crate) connection: Arc<Mutex<ModbusConnection>>,
    pub(crate) queue: Arc<ModbusRequestQueue>,
}

impl ModbusConfigNode {
    pub fn build(
        engine: &Engine,
        config: &RedGlobalNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn GlobalNodeBehavior>> {
        let modbus_config = ModbusConfig::deserialize(&config.rest)?;
        let connection = ModbusConnection { context: None, config: modbus_config.clone() };
        let queue =
            ModbusRequestQueue::new(modbus_config.parallel_unit_ids.unwrap_or(false), modbus_config.command_delay);
        let state = BaseGlobalNodeState {
            id: config.id,
            name: config.name.clone(),
            type_str: "modbus-config",
            ordering: config.ordering,
            context: engine.get_context_manager().new_context(engine.context(), config.id.to_string()),
            disabled: config.disabled,
        };
        Ok(Box::new(ModbusConfigNode {
            base: state,
            config: modbus_config,
            connection: Arc::new(Mutex::new(connection)),
            queue: Arc::new(queue),
        }))
    }
}

#[async_trait]
impl GlobalNodeBehavior for ModbusConfigNode {
    fn get_base(&self) -> &BaseGlobalNodeState {
        &self.base
    }
}

/// Shared helper to resolve a config_node string into a ModbusConfigNode.
/// Used by modbus_read, modbus_write, modbus_flex_getter, modbus_flex_writer.
pub(crate) async fn resolve_modbus_config(
    flow: Option<&crate::runtime::flow::Flow>,
    config_node_id: &str,
) -> crate::Result<Arc<dyn GlobalNodeBehavior>> {
    use std::str::FromStr;

    let engine = flow.and_then(|f| f.engine()).ok_or_else(|| anyhow::anyhow!("No engine available"))?;

    let eid_opt = ElementId::from_str(config_node_id).ok();
    let global = eid_opt
        .and_then(|eid| engine.find_global_node_by_id(&eid))
        .or_else(|| engine.find_global_node_by_name(config_node_id).ok().flatten())
        .ok_or_else(|| anyhow::anyhow!("Config node '{}' not found", config_node_id))?;

    Ok(global)
}

/// Downcast a global node to ModbusConfigNode, returning a descriptive error on mismatch.
pub(crate) fn downcast_modbus_config(global: &Arc<dyn GlobalNodeBehavior>) -> crate::Result<&ModbusConfigNode> {
    global
        .as_any()
        .downcast_ref::<ModbusConfigNode>()
        .ok_or_else(|| anyhow::anyhow!("Config node is not a modbus-config (got '{}')", global.type_str()))
}
