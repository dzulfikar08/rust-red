use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct ModbusServerConfig {
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_coil_count")]
    coil_count: u16,
    #[serde(default = "default_register_count")]
    register_count: u16,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    5020
}
fn default_coil_count() -> u16 {
    100
}
fn default_register_count() -> u16 {
    100
}

#[derive(Debug)]
#[flow_node("modbus-server", red_name = "modbus-server", module = "node-red")]
struct ModbusServerNode {
    base: BaseFlowNodeState,
    config: ModbusServerConfig,
}

impl ModbusServerNode {
    fn build(
        _flow: &Flow,
        state: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let cfg = ModbusServerConfig::deserialize(&config.rest)?;
        Ok(Box::new(ModbusServerNode { base: state, config: cfg }))
    }

    fn make_event_msg(&self, topic: &str, payload: Variant, session_id: &str, remote_addr: &str) -> MsgHandle {
        let mut body = std::collections::BTreeMap::new();
        body.insert("topic".to_string(), Variant::String(topic.to_string()));
        body.insert("payload".to_string(), payload);
        let mut session = std::collections::BTreeMap::new();
        session.insert("type".to_string(), Variant::String("modbus".to_string()));
        session.insert("id".to_string(), Variant::String(session_id.to_string()));
        session.insert("remoteAddress".to_string(), Variant::String(remote_addr.to_string()));
        body.insert("_session".to_string(), Variant::Object(session));
        MsgHandle::with_properties(body)
    }

    fn fc_name(fc: u8) -> &'static str {
        match fc {
            0x01 => "readCoils",
            0x02 => "readDiscreteInputs",
            0x03 => "readHoldingRegisters",
            0x04 => "readInputRegisters",
            0x05 => "writeSingleCoil",
            0x06 => "writeSingleRegister",
            0x0F => "writeMultipleCoils",
            0x10 => "writeMultipleRegisters",
            _ => "unknown",
        }
    }

    async fn handle_connection(
        self: &Arc<Self>,
        mut stream: tokio::net::TcpStream,
        session_id: String,
        remote_addr: String,
        coils: Arc<std::sync::Mutex<Vec<bool>>>,
        registers: Arc<std::sync::Mutex<Vec<u16>>>,
        stop_token: CancellationToken,
    ) {
        let mut buf = [0u8; 260];

        loop {
            // Read MBAP header (7 bytes): transaction_id(2) + protocol_id(2) + length(2) + unit_id(1)
            let n = tokio::select! {
                _ = stop_token.cancelled() => break,
                result = stream.read(&mut buf[..7]) => result,
            };
            match n {
                Ok(0) => break, // connection closed
                Ok(n) if n < 7 => {
                    // Try to read remaining header bytes
                    let mut total = n;
                    while total < 7 {
                        tokio::select! {
                            _ = stop_token.cancelled() => return,
                            result = stream.read(&mut buf[total..7]) => {
                                match result {
                                    Ok(0) => return,
                                    Ok(m) => total += m,
                                    Err(_) => return,
                                }
                            }
                        }
                    }
                }
                Err(_) => break,
                Ok(_) => {}
            }

            let tx_id = u16::from_be_bytes([buf[0], buf[1]]);
            let protocol_id = u16::from_be_bytes([buf[2], buf[3]]);
            let length = u16::from_be_bytes([buf[4], buf[5]]) as usize;
            let unit_id = buf[6];

            if protocol_id != 0 {
                break;
            }

            // PDU length = length - 1 (unit_id already read)
            let pdu_len = length.saturating_sub(1);
            if pdu_len == 0 || pdu_len > 253 {
                break;
            }

            // Read PDU
            let mut pdu = vec![0u8; pdu_len];
            let mut pdu_read = 0;
            while pdu_read < pdu_len {
                tokio::select! {
                    _ = stop_token.cancelled() => return,
                    result = stream.read(&mut pdu[pdu_read..]) => {
                        match result {
                            Ok(0) => return,
                            Ok(m) => pdu_read += m,
                            Err(_) => return,
                        }
                    }
                }
            }

            let fc = pdu[0];

            // Build event payload for the request
            let request_payload = self.build_request_payload(fc, &pdu);

            // Process the request and build response
            let response = self.process_request(fc, &pdu, &coils, &registers);

            // Emit request event to output
            let fc_name = Self::fc_name(fc);
            let mut payload_map = std::collections::BTreeMap::new();
            payload_map.insert("functionCode".to_string(), Variant::String(fc_name.to_string()));
            if let Some(ref req) = request_payload {
                for (k, v) in req {
                    payload_map.insert(k.clone(), v.clone());
                }
            }
            let event_payload = Variant::Object(payload_map);
            let event_msg = self.make_event_msg("modbus/request", event_payload, &session_id, &remote_addr);
            if let Err(e) = self.fan_out_one(Envelope { port: 0, msg: event_msg }, stop_token.clone()).await {
                log::debug!("[modbus-server] failed to emit event: {e}");
            }

            // Send response
            if let Some(resp_pdu) = response {
                let resp_len = (1 + resp_pdu.len()) as u16; // unit_id + pdu
                let mut frame = Vec::with_capacity(7 + resp_pdu.len());
                frame.extend_from_slice(&tx_id.to_be_bytes());
                frame.extend_from_slice(&0u16.to_be_bytes()); // protocol_id
                frame.extend_from_slice(&resp_len.to_be_bytes());
                frame.push(unit_id);
                frame.extend_from_slice(&resp_pdu);

                tokio::select! {
                    _ = stop_token.cancelled() => return,
                    result = stream.write_all(&frame) => {
                        if result.is_err() { return; }
                    }
                }
                let _ = stream.flush().await;
            }
        }

        // Emit disconnect event
        let disconnect_msg =
            self.make_event_msg("modbus/disconnect", Variant::String(remote_addr.clone()), &session_id, &remote_addr);
        let _ = self.fan_out_one(Envelope { port: 0, msg: disconnect_msg }, stop_token).await;
    }

    fn build_request_payload(&self, fc: u8, pdu: &[u8]) -> Option<std::collections::BTreeMap<String, Variant>> {
        let mut m = std::collections::BTreeMap::new();
        match fc {
            0x01 | 0x02 | 0x03 | 0x04 => {
                if pdu.len() >= 5 {
                    let addr = u16::from_be_bytes([pdu[1], pdu[2]]);
                    let qty = u16::from_be_bytes([pdu[3], pdu[4]]);
                    m.insert("address".to_string(), Variant::from(addr as i64));
                    m.insert("quantity".to_string(), Variant::from(qty as i64));
                }
            }
            0x05 => {
                if pdu.len() >= 5 {
                    let addr = u16::from_be_bytes([pdu[1], pdu[2]]);
                    let val = u16::from_be_bytes([pdu[3], pdu[4]]);
                    m.insert("address".to_string(), Variant::from(addr as i64));
                    m.insert("value".to_string(), Variant::Bool(val == 0xFF00));
                }
            }
            0x06 => {
                if pdu.len() >= 5 {
                    let addr = u16::from_be_bytes([pdu[1], pdu[2]]);
                    let val = u16::from_be_bytes([pdu[3], pdu[4]]);
                    m.insert("address".to_string(), Variant::from(addr as i64));
                    m.insert("value".to_string(), Variant::from(val as i64));
                }
            }
            0x0F => {
                if pdu.len() >= 6 {
                    let addr = u16::from_be_bytes([pdu[1], pdu[2]]);
                    let qty = u16::from_be_bytes([pdu[3], pdu[4]]);
                    m.insert("address".to_string(), Variant::from(addr as i64));
                    m.insert("quantity".to_string(), Variant::from(qty as i64));
                }
            }
            0x10 => {
                if pdu.len() >= 6 {
                    let addr = u16::from_be_bytes([pdu[1], pdu[2]]);
                    let qty = u16::from_be_bytes([pdu[3], pdu[4]]);
                    m.insert("address".to_string(), Variant::from(addr as i64));
                    m.insert("quantity".to_string(), Variant::from(qty as i64));
                }
            }
            _ => return None,
        }
        Some(m)
    }

    /// Process a Modbus request and return the response PDU (function code + data).
    /// Returns None for unsupported function codes.
    fn process_request(
        &self,
        fc: u8,
        pdu: &[u8],
        coils: &Arc<std::sync::Mutex<Vec<bool>>>,
        registers: &Arc<std::sync::Mutex<Vec<u16>>>,
    ) -> Option<Vec<u8>> {
        match fc {
            0x01 | 0x02 => {
                // Read Coils / Read Discrete Inputs
                if pdu.len() < 5 {
                    return Some(Self::exception(fc, 0x02));
                }
                let addr = u16::from_be_bytes([pdu[1], pdu[2]]) as usize;
                let qty = u16::from_be_bytes([pdu[3], pdu[4]]) as usize;
                let end = addr.saturating_add(qty);
                let c = coils.lock().unwrap();
                if end > c.len() {
                    return Some(Self::exception(fc, 0x02));
                }
                let byte_count = (qty + 7) / 8;
                let mut data = vec![0u8; byte_count];
                for i in 0..qty {
                    if c[addr + i] {
                        data[i / 8] |= 1 << (i % 8);
                    }
                }
                let mut resp = vec![fc, byte_count as u8];
                resp.extend_from_slice(&data);
                Some(resp)
            }
            0x03 | 0x04 => {
                // Read Holding Registers / Read Input Registers
                if pdu.len() < 5 {
                    return Some(Self::exception(fc, 0x02));
                }
                let addr = u16::from_be_bytes([pdu[1], pdu[2]]) as usize;
                let qty = u16::from_be_bytes([pdu[3], pdu[4]]) as usize;
                let end = addr.saturating_add(qty);
                let r = registers.lock().unwrap();
                if end > r.len() {
                    return Some(Self::exception(fc, 0x02));
                }
                let byte_count = (qty * 2) as u8;
                let mut resp = vec![fc, byte_count];
                for i in 0..qty {
                    resp.extend_from_slice(&r[addr + i].to_be_bytes());
                }
                Some(resp)
            }
            0x05 => {
                // Write Single Coil
                if pdu.len() < 5 {
                    return Some(Self::exception(fc, 0x02));
                }
                let addr = u16::from_be_bytes([pdu[1], pdu[2]]) as usize;
                let val = u16::from_be_bytes([pdu[3], pdu[4]]);
                let mut c = coils.lock().unwrap();
                if addr >= c.len() {
                    return Some(Self::exception(fc, 0x02));
                }
                c[addr] = val == 0xFF00;
                Some(vec![fc, pdu[1], pdu[2], pdu[3], pdu[4]])
            }
            0x06 => {
                // Write Single Register
                if pdu.len() < 5 {
                    return Some(Self::exception(fc, 0x02));
                }
                let addr = u16::from_be_bytes([pdu[1], pdu[2]]) as usize;
                let val = u16::from_be_bytes([pdu[3], pdu[4]]);
                let mut r = registers.lock().unwrap();
                if addr >= r.len() {
                    return Some(Self::exception(fc, 0x02));
                }
                r[addr] = val;
                Some(vec![fc, pdu[1], pdu[2], pdu[3], pdu[4]])
            }
            0x0F => {
                // Write Multiple Coils
                if pdu.len() < 6 {
                    return Some(Self::exception(fc, 0x02));
                }
                let addr = u16::from_be_bytes([pdu[1], pdu[2]]) as usize;
                let qty = u16::from_be_bytes([pdu[3], pdu[4]]) as usize;
                let byte_count = pdu[5] as usize;
                if pdu.len() < 6 + byte_count {
                    return Some(Self::exception(fc, 0x03));
                }
                let end = addr.saturating_add(qty);
                let mut c = coils.lock().unwrap();
                if end > c.len() {
                    return Some(Self::exception(fc, 0x02));
                }
                for i in 0..qty {
                    c[addr + i] = (pdu[6 + i / 8] >> (i % 8)) & 1 == 1;
                }
                Some(vec![fc, pdu[1], pdu[2], pdu[3], pdu[4]])
            }
            0x10 => {
                // Write Multiple Registers
                if pdu.len() < 6 {
                    return Some(Self::exception(fc, 0x02));
                }
                let addr = u16::from_be_bytes([pdu[1], pdu[2]]) as usize;
                let qty = u16::from_be_bytes([pdu[3], pdu[4]]) as usize;
                let byte_count = pdu[5] as usize;
                if pdu.len() < 6 + byte_count {
                    return Some(Self::exception(fc, 0x03));
                }
                let end = addr.saturating_add(qty);
                let mut r = registers.lock().unwrap();
                if end > r.len() {
                    return Some(Self::exception(fc, 0x02));
                }
                for i in 0..qty {
                    let off = 6 + i * 2;
                    r[addr + i] = u16::from_be_bytes([pdu[off], pdu[off + 1]]);
                }
                Some(vec![fc, pdu[1], pdu[2], pdu[3], pdu[4]])
            }
            _ => Some(Self::exception(fc, 0x01)),
        }
    }

    fn exception(fc: u8, code: u8) -> Vec<u8> {
        vec![fc | 0x80, code]
    }
}

#[async_trait]
impl FlowNodeBehavior for ModbusServerNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let coils = Arc::new(std::sync::Mutex::new(vec![false; self.config.coil_count as usize]));
        let registers = Arc::new(std::sync::Mutex::new(vec![0u16; self.config.register_count as usize]));
        let addr = format!("{}:{}", self.config.host, self.config.port);

        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                log::error!("[modbus-server:{}] Bind failed: {e}", self.name());
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

        log::info!("[modbus-server:{}] Listening on {addr}", self.name());
        self.report_status(
            StatusObject {
                fill: Some(StatusFill::Green),
                shape: Some(StatusShape::Dot),
                text: Some(format!("listening :{}", self.config.port)),
            },
            stop_token.clone(),
        )
        .await;

        let conn_counter = Arc::new(tokio::sync::Mutex::new(0u64));

        loop {
            tokio::select! {
                _ = stop_token.cancelled() => {
                    log::info!("[modbus-server:{}] Stopped", self.name());
                    break;
                }
                accept = listener.accept() => {
                    match accept {
                        Ok((stream, peer)) => {
                            let mut counter = conn_counter.lock().await;
                            *counter += 1;
                            let session_id = format!("modbus_conn_{}", *counter);
                            drop(counter);

                            let remote_addr = peer.to_string();

                            // Emit connect event
                            let connect_msg = self.make_event_msg(
                                "modbus/connect",
                                Variant::String(remote_addr.clone()),
                                &session_id,
                                &remote_addr,
                            );
                            if let Err(e) = self.fan_out_one(
                                Envelope { port: 0, msg: connect_msg },
                                stop_token.clone(),
                            ).await {
                                log::debug!("[modbus-server] Failed to emit connect event: {e}");
                            }

                            // Spawn connection handler
                            let this = self.clone();
                            let coils = coils.clone();
                            let registers = registers.clone();
                            let cancel = stop_token.child_token();
                            tokio::spawn(async move {
                                this.handle_connection(
                                    stream, session_id, remote_addr,
                                    coils, registers, cancel,
                                ).await;
                            });
                        }
                        Err(e) => {
                            log::warn!("[modbus-server:{}] Accept error: {e}", self.name());
                        }
                    }
                }
                // Handle input messages — write to simulator state
                result = self.base.msg_rx.recv_msg(stop_token.clone()) => {
                    if let Ok(msg) = result {
                        let guard = msg.read().await;
                        let topic = guard.get("topic").and_then(|v| v.as_str()).unwrap_or("");
                        if topic == "write" || topic.is_empty() {
                            if let Some(payload) = guard.get("payload") {
                                if let Some(obj) = payload.as_object() {
                                    let addr_val = obj.get("address").and_then(|v| v.as_f64()).map(|f| f as usize);
                                    let value = obj.get("value");
                                    if let (Some(a), Some(v)) = (addr_val, value) {
                                        if let Some(b) = v.as_bool() {
                                            let mut c = coils.lock().unwrap();
                                            if a < c.len() { c[a] = b; }
                                        } else if let Some(n) = v.as_f64() {
                                            let mut r = registers.lock().unwrap();
                                            if a < r.len() { r[a] = n as u16; }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
