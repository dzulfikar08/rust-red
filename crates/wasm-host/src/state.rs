use rust_red_core::runtime::nodes::StatusObject;

use crate::types::WasmMessage;

/// Per-instance state stored inside the wasmtime Store.
/// Accessible from host import functions when the guest calls back.
#[derive(Debug)]
pub struct WasmNodeState {
    /// Messages queued by the guest via `host_send_msg`.
    pub pending_outputs: Vec<PendingOutput>,
    /// Status set by the guest via `host_set_status`.
    pub status: Option<StatusObject>,
    /// Errors reported by the guest.
    pub errors: Vec<String>,
    /// Node scope identifier, e.g. "node:{id}"
    pub node_scope: String,
}

/// A message queued by the guest for output on a specific port.
#[derive(Debug, Clone)]
pub struct PendingOutput {
    pub port: u32,
    pub msg: WasmMessage,
}

impl WasmNodeState {
    pub fn new(node_id: &str) -> Self {
        Self { pending_outputs: Vec::new(), status: None, errors: Vec::new(), node_scope: format!("node:{}", node_id) }
    }

    /// Push a pending output from `host_send_msg`.
    pub fn push_output(&mut self, port: u32, msg: WasmMessage) {
        self.pending_outputs.push(PendingOutput { port, msg });
    }

    /// Drain all pending outputs.
    pub fn drain_outputs(&mut self) -> Vec<PendingOutput> {
        self.pending_outputs.drain(..).collect()
    }

    /// Record an error from the guest.
    pub fn push_error(&mut self, msg: String) {
        self.errors.push(msg);
    }
}
