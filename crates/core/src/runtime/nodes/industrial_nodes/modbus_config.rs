use std::sync::Arc;
use std::time::Duration;

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
#[allow(dead_code)]
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
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ModbusDataType {
    /// Raw unsigned 16-bit word (default).
    #[default]
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
                Ok(serde_json::Number::from_f64(val as f64).map_or(Variant::Null, Variant::Number))
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
                Ok(serde_json::Number::from_f64(val).map_or(Variant::Null, Variant::Number))
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
        if !words.len().is_multiple_of(stride) {
            return Err(anyhow::anyhow!(
                "Register count {} is not a multiple of data-type width {}",
                words.len(),
                stride
            ));
        }
        words.chunks(stride).map(|chunk| self.words_to_variant(chunk)).collect()
    }
}

/// Raw data of a read, before any data type conversion.
#[derive(Debug, Clone)]
pub(crate) enum ModbusRaw {
    Bits(Vec<bool>),
    Words(Vec<u16>),
}

impl ModbusRaw {
    /// The values as plain numbers or booleans, like the `data` array of the contrib nodes.
    pub(crate) fn to_variant(&self) -> Variant {
        match self {
            Self::Bits(bits) => Variant::Array(bits.iter().map(|b| Variant::Bool(*b)).collect()),
            Self::Words(words) => Variant::Array(words.iter().map(|w| Variant::from(*w as i64)).collect()),
        }
    }

    /// The bytes of the response body: big endian registers, or coils packed eight to a byte, lowest bit first.
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Bits(bits) => {
                let mut bytes = vec![0u8; bits.len().div_ceil(8)];
                for (index, bit) in bits.iter().enumerate() {
                    if *bit {
                        bytes[index / 8] |= 1 << (index % 8);
                    }
                }
                bytes
            }
            Self::Words(words) => words.iter().flat_map(|w| w.to_be_bytes()).collect(),
        }
    }
}

pub(crate) struct ModbusConnection {
    context: Option<tokio_modbus::client::Context>,
    config: ModbusConfig,
    /// Unit (slave) id used for the next request. Reset per request by `select_unit`.
    unit: u8,
}

impl std::fmt::Debug for ModbusConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModbusConnection")
            .field("connected", &self.context.is_some())
            .field("config", &self.config)
            .finish()
    }
}

/// Run one request with the configured timeout. A timeout counts as a transport error.
async fn timed<T>(limit: Duration, call: impl Future<Output = tokio_modbus::Result<T>>) -> tokio_modbus::Result<T> {
    tokio::time::timeout(limit, call).await.unwrap_or_else(|_| {
        Err(tokio_modbus::Error::Transport(std::io::Error::new(std::io::ErrorKind::TimedOut, "request timed out")))
    })
}

/// Send a request on the connection. A transport failure or timeout drops the connection,
/// reconnects and tries once more. A Modbus exception response is returned as an error
/// right away, because the device did answer and the link is fine.
macro_rules! modbus_op {
    ($conn:expr, $label:expr, |$ctx:ident| $call:expr) => {{
        let conn: &mut ModbusConnection = $conn;
        conn.ensure_connected().await?;
        conn.apply_unit();
        let limit = Duration::from_millis(conn.config.timeout_ms);
        let first = {
            let $ctx = conn.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
            timed(limit, $call).await
        };
        match first {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(code)) => Err(anyhow::anyhow!("{} failed: Modbus exception {code:?}", $label)),
            Err(error) => {
                log::warn!("[modbus-config] {} failed ({error}), reconnecting", $label);
                if conn.try_reconnect().await {
                    conn.apply_unit();
                    let $ctx = conn.context.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
                    match timed(limit, $call).await {
                        Ok(Ok(value)) => Ok(value),
                        Ok(Err(code)) => Err(anyhow::anyhow!("{} failed: Modbus exception {code:?}", $label)),
                        Err(error) => Err(anyhow::anyhow!("{} failed: {error}", $label)),
                    }
                } else {
                    Err(anyhow::anyhow!("{} failed: {error}", $label))
                }
            }
        }
    }};
}

impl ModbusConnection {
    pub(crate) async fn ensure_connected(&mut self) -> crate::Result<()> {
        if self.context.is_some() {
            return Ok(());
        }
        if !self.config.auto_connect.unwrap_or(true) {
            return Err(anyhow::anyhow!("Not connected and auto_connect is disabled"));
        }
        self.connect().await
    }

    /// Use `unit` for the next requests, or the unit id of the config node when it is `None`.
    pub(crate) fn select_unit(&mut self, unit: Option<u8>) {
        self.unit = unit.unwrap_or(self.config.unit_id);
    }

    fn apply_unit(&mut self) {
        if let Some(ctx) = self.context.as_mut() {
            ctx.set_slave(Slave(self.unit));
        }
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
        let target = format!("{}:{}", self.config.host, self.config.port);
        let limit = Duration::from_millis(self.config.timeout_ms);

        let connect = async {
            let addr = tokio::net::lookup_host(&target)
                .await
                .map_err(|e| anyhow::anyhow!("Cannot resolve '{target}': {e}"))?
                .next()
                .ok_or_else(|| anyhow::anyhow!("Cannot resolve '{target}'"))?;
            tcp::connect_slave(addr, Slave(self.unit)).await.map_err(|e| anyhow::anyhow!("Connection failed: {e}"))
        };

        match tokio::time::timeout(limit, connect).await {
            Ok(Ok(mut ctx)) => {
                ctx.set_slave(Slave(self.unit));
                self.context = Some(ctx);
                log::info!("[modbus-config] Connected to {target}");
                Ok(())
            }
            Ok(Err(e)) => {
                log::warn!("[modbus-config] {e} ({target})");
                Err(e)
            }
            Err(_) => {
                log::warn!("[modbus-config] Connection timeout to {target}");
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

        let ctx = rtu::attach_slave(port, Slave(self.unit));
        self.context = Some(ctx);

        log::info!("[modbus-config] Connected RTU to {} @ {} baud, unit {}", serial_port, baud_rate, self.unit);
        Ok(())
    }

    fn disconnect(&mut self) {
        if self.context.take().is_some() {
            log::info!("[modbus-config] Disconnected");
        }
    }

    /// Drop the connection and connect again, after the configured reconnect delay.
    /// Returns true if reconnection succeeded.
    async fn try_reconnect(&mut self) -> bool {
        self.disconnect();
        if !self.config.auto_connect.unwrap_or(true) {
            return false;
        }
        if let Some(reconnect_ms) = self.config.reconnect_timeout {
            tokio::time::sleep(Duration::from_millis(reconnect_ms)).await;
        }
        self.connect().await.is_ok()
    }

    /// Returns true if the connection appears to be alive.
    #[allow(dead_code)]
    pub(crate) fn is_connected(&self) -> bool {
        self.context.is_some()
    }

    /// FC1 – Read Coils (bits).
    pub(crate) async fn read_coils(&mut self, address: u16, quantity: u16) -> crate::Result<Vec<bool>> {
        modbus_op!(self, "Read coils", |ctx| ctx.read_coils(address, quantity))
    }

    /// FC2 – Read Discrete Inputs (read-only bits).
    pub(crate) async fn read_discrete_inputs(&mut self, address: u16, quantity: u16) -> crate::Result<Vec<bool>> {
        modbus_op!(self, "Read discrete inputs", |ctx| ctx.read_discrete_inputs(address, quantity))
    }

    /// FC3 – Read Holding Registers.
    pub(crate) async fn read_holding_registers(&mut self, address: u16, quantity: u16) -> crate::Result<Vec<u16>> {
        modbus_op!(self, "Read holding registers", |ctx| ctx.read_holding_registers(address, quantity))
    }

    /// FC4 – Read Input Registers (read-only).
    pub(crate) async fn read_input_registers(&mut self, address: u16, quantity: u16) -> crate::Result<Vec<u16>> {
        modbus_op!(self, "Read input registers", |ctx| ctx.read_input_registers(address, quantity))
    }

    /// FC5 – Write Single Coil.
    pub(crate) async fn write_single_coil(&mut self, address: u16, value: bool) -> crate::Result<()> {
        modbus_op!(self, "Write coil", |ctx| ctx.write_single_coil(address, value))
    }

    /// FC6 – Write Single Register.
    pub(crate) async fn write_single_register(&mut self, address: u16, value: u16) -> crate::Result<()> {
        modbus_op!(self, "Write register", |ctx| ctx.write_single_register(address, value))
    }

    /// FC15 – Write Multiple Coils.
    pub(crate) async fn write_multiple_coils(&mut self, address: u16, values: &[bool]) -> crate::Result<()> {
        modbus_op!(self, "Write multiple coils", |ctx| ctx.write_multiple_coils(address, values))
    }

    /// FC16 – Write Multiple Registers.
    pub(crate) async fn write_multiple_registers(&mut self, address: u16, values: &[u16]) -> crate::Result<()> {
        modbus_op!(self, "Write multiple registers", |ctx| ctx.write_multiple_registers(address, values))
    }

    /// Read by function code number (1 to 4) from `unit`, or the config node's unit when `None`.
    pub(crate) async fn read_raw(
        &mut self,
        unit: Option<u8>,
        fc: u8,
        address: u16,
        quantity: u16,
    ) -> crate::Result<ModbusRaw> {
        self.select_unit(unit);
        match fc {
            1 => {
                let mut bits = self.read_coils(address, quantity).await?;
                bits.truncate(quantity as usize);
                Ok(ModbusRaw::Bits(bits))
            }
            2 => {
                let mut bits = self.read_discrete_inputs(address, quantity).await?;
                bits.truncate(quantity as usize);
                Ok(ModbusRaw::Bits(bits))
            }
            3 => Ok(ModbusRaw::Words(self.read_holding_registers(address, quantity).await?)),
            4 => Ok(ModbusRaw::Words(self.read_input_registers(address, quantity).await?)),
            other => Err(anyhow::anyhow!("Function code {other} is not a read function (use 1 to 4)")),
        }
    }

    /// Write by function code number (5, 6, 15 or 16) the way the contrib write nodes do.
    /// Returns the acknowledgement as an object: `{address, state}` (FC5), `{address, value}` (FC6)
    /// or `{address, length}` (FC15 and FC16).
    pub(crate) async fn write_raw(
        &mut self,
        unit: Option<u8>,
        fc: u8,
        address: u16,
        value: &Variant,
    ) -> crate::Result<Variant> {
        fn truthy(value: &Variant) -> bool {
            match value {
                Variant::Bool(b) => *b,
                Variant::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
                Variant::String(s) => matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1"),
                _ => false,
            }
        }
        fn word(value: &Variant) -> crate::Result<u16> {
            match value {
                Variant::Number(n) => n
                    .as_f64()
                    .map(|f| f as i64 as u16)
                    .ok_or_else(|| anyhow::anyhow!("Value {n} cannot be written to a register")),
                Variant::String(s) => s
                    .trim()
                    .parse::<f64>()
                    .map(|f| f as i64 as u16)
                    .map_err(|_| anyhow::anyhow!("Value '{s}' is not a number")),
                Variant::Bool(b) => Ok(*b as u16),
                other => Err(anyhow::anyhow!("Value {other:?} cannot be written to a register")),
            }
        }
        fn items(value: &Variant) -> Vec<Variant> {
            match value {
                Variant::Array(items) => items.clone(),
                single => vec![single.clone()],
            }
        }

        self.select_unit(unit);
        let ack = |key: &str, value: Variant| Variant::from([("address", Variant::from(address as i64)), (key, value)]);
        match fc {
            5 => {
                let state = truthy(value);
                self.write_single_coil(address, state).await?;
                Ok(ack("state", Variant::Bool(state)))
            }
            6 => {
                let first = items(value).into_iter().next().ok_or_else(|| anyhow::anyhow!("No value to write"))?;
                let word = word(&first)?;
                self.write_single_register(address, word).await?;
                Ok(ack("value", Variant::from(word as i64)))
            }
            15 => {
                let coils: Vec<bool> = items(value).iter().map(truthy).collect();
                if coils.is_empty() {
                    return Err(anyhow::anyhow!("No values to write"));
                }
                self.write_multiple_coils(address, &coils).await?;
                Ok(ack("length", Variant::from(coils.len() as i64)))
            }
            16 => {
                let words = items(value).iter().map(word).collect::<crate::Result<Vec<u16>>>()?;
                if words.is_empty() {
                    return Err(anyhow::anyhow!("No values to write"));
                }
                self.write_multiple_registers(address, &words).await?;
                Ok(ack("length", Variant::from(words.len() as i64)))
            }
            other => Err(anyhow::anyhow!("Function code {other} is not a write function (use 5, 6, 15 or 16)")),
        }
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
    #[allow(dead_code)]
    pub(crate) queue: Arc<ModbusRequestQueue>,
}

impl ModbusConfigNode {
    pub fn build(
        engine: &Engine,
        config: &RedGlobalNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn GlobalNodeBehavior>> {
        // `modbus-client` is the config node of node-red-contrib-modbus.
        let rest = if config.type_name == "modbus-client" {
            super::modbus_contrib::translate_client(&config.rest)?
        } else {
            config.rest.clone()
        };
        let modbus_config = ModbusConfig::deserialize(&rest)?;
        let connection = ModbusConnection { context: None, unit: modbus_config.unit_id, config: modbus_config.clone() };
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
    let engine = flow.and_then(|f| f.engine()).ok_or_else(|| anyhow::anyhow!("No engine available"))?;

    engine
        .find_global_node_by_ref(config_node_id)
        .ok_or_else(|| anyhow::anyhow!("Config node '{config_node_id}' not found, or it failed to load"))
}

/// Downcast a global node to ModbusConfigNode, returning a descriptive error on mismatch.
pub(crate) fn downcast_modbus_config(global: &Arc<dyn GlobalNodeBehavior>) -> crate::Result<&ModbusConfigNode> {
    global
        .as_any()
        .downcast_ref::<ModbusConfigNode>()
        .ok_or_else(|| anyhow::anyhow!("Config node is not a modbus-config (got '{}')", global.type_str()))
}
