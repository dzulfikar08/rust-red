//! WasmNodeShim — bridges the host's `FlowNodeBehavior` trait to the WASM guest.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use rust_red_core::runtime::context::Context;
use rust_red_core::runtime::model::*;
use rust_red_core::runtime::nodes::*;
use tokio::select;
use tokio_util::sync::CancellationToken;

use crate::state::WasmNodeState;
use crate::types::{ProcessResult, WasmMessage, WasmValue};

/// The shim struct that implements `FlowNodeBehavior` and delegates to the WASM guest.
pub struct WasmNodeShim {
    pub base: BaseFlowNodeState,
    pub instance: wasmtime::Instance,
    pub store: std::sync::Mutex<wasmtime::Store<WasmNodeState>>,
    pub memory: wasmtime::Memory,
    pub process_fn: wasmtime::TypedFunc<(u32, u32), u32>,
    pub on_start_fn: wasmtime::TypedFunc<(u32, u32), u32>,
    pub on_stop_fn: wasmtime::TypedFunc<(), u32>,
    pub alloc_fn: wasmtime::TypedFunc<u32, u32>,
    pub result_len_fn: Option<wasmtime::TypedFunc<(), u32>>,
}

impl WasmNodeShim {
    pub fn new(
        base: BaseFlowNodeState,
        instance: wasmtime::Instance,
        store: wasmtime::Store<WasmNodeState>,
        memory: wasmtime::Memory,
        process_fn: wasmtime::TypedFunc<(u32, u32), u32>,
        on_start_fn: wasmtime::TypedFunc<(u32, u32), u32>,
        on_stop_fn: wasmtime::TypedFunc<(), u32>,
        alloc_fn: wasmtime::TypedFunc<u32, u32>,
        result_len_fn: Option<wasmtime::TypedFunc<(), u32>>,
    ) -> Self {
        Self {
            base,
            instance,
            store: std::sync::Mutex::new(store),
            memory,
            process_fn,
            on_start_fn,
            on_stop_fn,
            alloc_fn,
            result_len_fn,
        }
    }

    fn call_guest_on_start(&self) {
        let mut store_guard = self.store.lock().unwrap();
        store_guard.set_epoch_deadline(1_000_000);
        // Send empty config as a minimal WasmMessage
        let empty_config =
            WasmMessage { msg_id: String::new(), payload: WasmValue::Null, topic: None, extra: BTreeMap::new() };
        let config_bytes = postcard::to_allocvec(&empty_config).unwrap_or_default();

        let len = config_bytes.len() as u32;
        let ptr = self.alloc_fn.call(&mut *store_guard, len).unwrap_or(0);

        if ptr == 0 {
            log::error!("WasmNodeShim: guest alloc returned null for on_start config");
            return;
        }

        self.memory.data_mut(&mut *store_guard)[ptr as usize..][..len as usize].copy_from_slice(&config_bytes);

        match self.on_start_fn.call(&mut *store_guard, (ptr, len)) {
            Ok(0) => log::debug!("WasmNodeShim: guest on_start returned success"),
            Ok(code) => log::warn!("WasmNodeShim: guest on_start returned code {code}"),
            Err(e) => log::error!("WasmNodeShim: guest on_start failed: {e}"),
        }
    }

    fn call_guest_on_stop(&self) {
        let mut store_guard = self.store.lock().unwrap();
        store_guard.set_epoch_deadline(1_000_000);
        match self.on_stop_fn.call(&mut *store_guard, ()) {
            Ok(0) => log::debug!("WasmNodeShim: guest on_stop returned success"),
            Ok(code) => log::warn!("WasmNodeShim: guest on_stop returned code {code}"),
            Err(e) => log::error!("WasmNodeShim: guest on_stop failed: {e}"),
        }
    }

    async fn process_and_forward(&self, msg: MsgHandle) -> anyhow::Result<Vec<Envelope>> {
        let mut envelopes = Vec::new();

        let wasm_msg = Self::msg_to_wasm(msg).await;
        let msg_bytes = postcard::to_allocvec(&wasm_msg).map_err(|e| {
            rust_red_core::RustRedError::InvalidOperation(format!("postcard serialization failed: {e}"))
        })?;

        let mut store_guard = self.store.lock().unwrap();
        store_guard.set_epoch_deadline(1_000_000);

        let len = msg_bytes.len() as u32;
        let guest_ptr = self
            .alloc_fn
            .call(&mut *store_guard, len)
            .map_err(|e| rust_red_core::RustRedError::InvalidOperation(format!("guest alloc failed: {e}")))?;

        self.memory.data_mut(&mut *store_guard)[guest_ptr as usize..][..len as usize].copy_from_slice(&msg_bytes);

        let result_ptr = self
            .process_fn
            .call(&mut *store_guard, (guest_ptr, len))
            .map_err(|e| rust_red_core::RustRedError::InvalidOperation(format!("guest process_msg failed: {e}")))?;

        let result = if result_ptr == 0 {
            ProcessResult { output: None }
        } else {
            let result_len = if let Some(ref len_fn) = self.result_len_fn {
                len_fn.call(&mut *store_guard, ()).unwrap_or(0)
            } else {
                4096
            };

            if result_len == 0 {
                ProcessResult { output: None }
            } else {
                let mem_data = self.memory.data(&*store_guard);
                let ptr = result_ptr as usize;
                let end = ptr.saturating_add(result_len as usize);
                if end > mem_data.len() {
                    return Err(rust_red_core::RustRedError::InvalidOperation(
                        "guest result pointer out of bounds".into(),
                    )
                    .into());
                }
                let result_bytes = &mem_data[ptr..][..result_len as usize];

                postcard::from_bytes::<ProcessResult>(result_bytes).map_err(|e| {
                    rust_red_core::RustRedError::InvalidOperation(format!(
                        "postcard deserialization of ProcessResult failed: {e}"
                    ))
                })?
            }
        };

        if let Some(outputs) = result.output {
            for (port_idx, port_msgs) in outputs.iter().enumerate() {
                for wasm_msg in port_msgs.iter().flatten() {
                    let msg_handle = Self::wasm_to_msg(wasm_msg);
                    envelopes.push(Envelope { port: port_idx, msg: msg_handle });
                }
            }
        }

        let pending = store_guard.data_mut().drain_outputs();
        for pending_output in pending {
            let msg_handle = Self::wasm_to_msg(&pending_output.msg);
            envelopes.push(Envelope { port: pending_output.port as usize, msg: msg_handle });
        }

        Ok(envelopes)
    }

    async fn msg_to_wasm(msg: MsgHandle) -> WasmMessage {
        let guard = msg.read().await;
        let mut msg_id = String::new();
        let mut payload = WasmValue::Null;
        let mut topic = None;
        let mut extra = BTreeMap::new();

        let obj = guard.as_variant_object();
        for (k, v) in obj.iter() {
            match k.as_str() {
                "_msgid" => {
                    msg_id = v.to_string().unwrap_or_default();
                }
                "payload" => {
                    payload = Self::variant_to_wasm_value(v);
                }
                "topic" => {
                    topic = v.as_str().map(|s: &str| s.to_string());
                }
                _ => {
                    extra.insert(k.clone(), Self::variant_to_wasm_value(v));
                }
            }
        }

        WasmMessage { msg_id, payload, topic, extra }
    }

    fn wasm_to_msg(wasm_msg: &WasmMessage) -> MsgHandle {
        let mut body = BTreeMap::new();

        body.insert("_msgid".to_string(), Variant::String(wasm_msg.msg_id.clone()));
        body.insert("payload".to_string(), Self::wasm_value_to_variant(&wasm_msg.payload));

        if let Some(ref topic) = wasm_msg.topic {
            body.insert("topic".to_string(), Variant::String(topic.clone()));
        }

        for (k, v) in &wasm_msg.extra {
            body.insert(k.clone(), Self::wasm_value_to_variant(v));
        }

        MsgHandle::with_properties(body)
    }

    fn variant_to_wasm_value(v: &Variant) -> WasmValue {
        match v {
            Variant::Null => WasmValue::Null,
            Variant::Bool(b) => WasmValue::Bool(*b),
            Variant::Number(n) => {
                if let Some(i) = n.as_i64() {
                    WasmValue::I64(i)
                } else if let Some(u) = n.as_u64() {
                    WasmValue::U64(u)
                } else {
                    WasmValue::F64(n.as_f64().unwrap_or(0.0))
                }
            }
            Variant::String(s) => WasmValue::String(s.clone()),
            Variant::Bytes(b) => WasmValue::Bytes(b.clone()),
            Variant::Array(arr) => WasmValue::Array(arr.iter().map(Self::variant_to_wasm_value).collect()),
            Variant::Object(map) => {
                let mut bt = BTreeMap::new();
                for (k, v) in map.iter() {
                    bt.insert(k.clone(), Self::variant_to_wasm_value(v));
                }
                WasmValue::Object(bt)
            }
            Variant::Date(_) => WasmValue::Null,
            Variant::Regexp(_) => WasmValue::Null,
        }
    }

    fn wasm_value_to_variant(v: &WasmValue) -> Variant {
        match v {
            WasmValue::Null => Variant::Null,
            WasmValue::Bool(b) => Variant::Bool(*b),
            WasmValue::I64(i) => Variant::Number(serde_json::Number::from(*i)),
            WasmValue::U64(u) => Variant::Number(serde_json::Number::from(*u)),
            WasmValue::F64(f) => {
                Variant::Number(serde_json::Number::from_f64(*f).unwrap_or_else(|| serde_json::Number::from(0)))
            }
            WasmValue::String(s) => Variant::String(s.clone()),
            WasmValue::Bytes(b) => Variant::Bytes(b.clone()),
            WasmValue::Array(arr) => Variant::Array(arr.iter().map(Self::wasm_value_to_variant).collect()),
            WasmValue::Object(map) => {
                let mut bt = BTreeMap::new();
                for (k, v) in map.iter() {
                    bt.insert(k.clone(), Self::wasm_value_to_variant(v));
                }
                Variant::Object(bt)
            }
        }
    }
}

impl FlowsElement for WasmNodeShim {
    fn id(&self) -> ElementId {
        self.base.id
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn type_str(&self) -> &'static str {
        self.base.type_str
    }

    fn ordering(&self) -> usize {
        self.base.ordering
    }

    fn is_disabled(&self) -> bool {
        self.base.disabled
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn parent_element(&self) -> Option<ElementId> {
        self.base.flow.upgrade().map(|arc| arc.id())
    }

    fn get_path(&self) -> String {
        match self.base.flow.upgrade() {
            Some(flow) => format!("{}/{}", flow.get_path(), self.id()),
            None => self.id().to_string(),
        }
    }
}

impl ContextHolder for WasmNodeShim {
    fn context(&self) -> &Context {
        &self.base.context
    }
}

#[async_trait]
impl FlowNodeBehavior for WasmNodeShim {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        self.call_guest_on_start();

        loop {
            select! {
                _ = stop_token.cancelled() => {
                    log::debug!("WasmNodeShim: stop token cancelled");
                    break;
                }
                result = self.recv_msg(stop_token.clone()) => {
                    match result {
                        Ok(msg) => {
                            match self.process_and_forward(msg).await {
                                Ok(envelopes) => {
                                    for envelope in envelopes {
                                        if let Err(e) = self.fan_out_one(envelope, stop_token.clone()).await {
                                            log::error!("WasmNodeShim: fan_out_one error: {e}");
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("WasmNodeShim: process_and_forward error: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            if let Some(rust_red_core::RustRedError::TaskCancelled) =
                                e.downcast_ref::<rust_red_core::RustRedError>()
                            {
                                break;
                            }
                            log::warn!("WasmNodeShim: recv_msg error: {e}");
                            break;
                        }
                    }
                }
            }
        }

        self.call_guest_on_stop();
    }
}
