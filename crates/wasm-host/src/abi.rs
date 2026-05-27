//! Host import functions that are linked into the WASM guest.
//!
//! These are the functions the guest calls back into the host:
//! - `host_log`
//! - `host_send_msg`
//! - `host_set_status`
//! - `host_alloc`
//! - `host_report_error`

use wasmtime::Linker;

use crate::memory::{read_from_guest, read_string_from_guest};
use crate::state::WasmNodeState;

/// Register all core host imports into the given Linker.
pub fn register_core_imports(linker: &mut Linker<WasmNodeState>) -> anyhow::Result<()> {
    linker.func_wrap("env", "host_log", host_log)?;
    linker.func_wrap("env", "host_send_msg", host_send_msg)?;
    linker.func_wrap("env", "host_set_status", host_set_status)?;
    linker.func_wrap("env", "host_alloc", host_alloc_fn)?;
    linker.func_wrap("env", "host_report_error", host_report_error)?;
    Ok(())
}

/// `host_log(level: u32, msg_ptr: u32, msg_len: u32)`
///
/// Log a message from the guest at the given level:
/// 0 = error, 1 = warn, 2 = info, 3 = debug, 4 = trace.
fn host_log(mut caller: wasmtime::Caller<'_, WasmNodeState>, level: u32, msg_ptr: u32, msg_len: u32) {
    let memory = match get_memory(&mut caller) {
        Some(m) => m,
        None => {
            log::error!("host_log: guest has no exported memory");
            return;
        }
    };

    let msg = match read_string_from_guest(&caller, &memory, msg_ptr, msg_len) {
        Ok(s) => s,
        Err(e) => {
            log::error!("host_log: failed to read guest string: {e}");
            return;
        }
    };

    match level {
        0 => log::error!("[wasm] {}", msg),
        1 => log::warn!("[wasm] {}", msg),
        2 => log::info!("[wasm] {}", msg),
        3 => log::debug!("[wasm] {}", msg),
        4 => log::trace!("[wasm] {}", msg),
        _ => log::info!("[wasm] {}", msg),
    }
}

/// `host_send_msg(port: u32, msg_ptr: u32, msg_len: u32)`
///
/// Queue a message for output on the given port.
fn host_send_msg(mut caller: wasmtime::Caller<'_, WasmNodeState>, port: u32, msg_ptr: u32, msg_len: u32) {
    let memory = match get_memory(&mut caller) {
        Some(m) => m,
        None => {
            log::error!("host_send_msg: guest has no exported memory");
            return;
        }
    };

    let bytes = match read_from_guest(&caller, &memory, msg_ptr, msg_len) {
        Ok(b) => b,
        Err(e) => {
            log::error!("host_send_msg: failed to read guest memory: {e}");
            return;
        }
    };

    match postcard::from_bytes::<crate::types::WasmMessage>(&bytes) {
        Ok(msg) => {
            caller.data_mut().push_output(port, msg);
        }
        Err(e) => {
            log::error!("host_send_msg: failed to deserialize WasmMessage: {e}");
        }
    }
}

/// `host_set_status(fill: u32, shape: u32, text_ptr: u32, text_len: u32)`
///
/// Set the node status indicator.
fn host_set_status(
    mut caller: wasmtime::Caller<'_, WasmNodeState>,
    fill: u32,
    shape: u32,
    text_ptr: u32,
    text_len: u32,
) {
    use rust_red_core::runtime::nodes::{StatusFill, StatusObject, StatusShape};

    let memory = match get_memory(&mut caller) {
        Some(m) => m,
        None => return,
    };

    let text = read_string_from_guest(&caller, &memory, text_ptr, text_len).ok();

    let status_fill = match fill {
        0 => Some(StatusFill::Red),
        1 => Some(StatusFill::Green),
        2 => Some(StatusFill::Yellow),
        3 => Some(StatusFill::Grey),
        4 => Some(StatusFill::Blue),
        _ => None,
    };

    let status_shape = match shape {
        0 => Some(StatusShape::Ring),
        1 => Some(StatusShape::Dot),
        _ => None,
    };

    caller.data_mut().status = Some(StatusObject { fill: status_fill, shape: status_shape, text });
}

/// `host_alloc(size: u32) -> u32`
///
/// Allocate `size` bytes in guest linear memory.
/// This is a fallback allocator — the guest should export its own `rust_red_alloc`.
fn host_alloc_fn(mut caller: wasmtime::Caller<'_, WasmNodeState>, size: u32) -> u32 {
    const WASM_PAGE_SIZE: usize = 65536;

    let memory = match get_memory(&mut caller) {
        Some(m) => m,
        None => return 0,
    };

    let current_pages = memory.size(&caller) as usize;
    let current_size = current_pages * WASM_PAGE_SIZE;

    // If we need more memory, grow
    let needed = size as usize;
    if current_size < needed {
        let extra_pages = (needed - current_size + WASM_PAGE_SIZE - 1) / WASM_PAGE_SIZE;
        if memory.grow(&mut caller, extra_pages as u64).is_err() {
            return 0;
        }
    }

    // Return offset at the end of old memory area
    let ptr = current_size as u32;
    ptr
}

/// `host_report_error(msg_ptr: u32, msg_len: u32)`
fn host_report_error(mut caller: wasmtime::Caller<'_, WasmNodeState>, msg_ptr: u32, msg_len: u32) {
    let memory = match get_memory(&mut caller) {
        Some(m) => m,
        None => return,
    };

    match read_string_from_guest(&caller, &memory, msg_ptr, msg_len) {
        Ok(msg) => {
            log::error!("[wasm] error reported: {}", msg);
            caller.data_mut().push_error(msg);
        }
        Err(e) => {
            log::error!("host_report_error: failed to read error message: {e}");
        }
    }
}

/// Helper to extract the exported `memory` from the caller.
fn get_memory(caller: &mut wasmtime::Caller<'_, WasmNodeState>) -> Option<wasmtime::Memory> {
    caller.get_export("memory").and_then(|e| e.into_memory())
}
