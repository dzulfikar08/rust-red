use anyhow::{Context, Result};
use wasmtime::Memory;

use crate::state::WasmNodeState;

/// Write data into guest linear memory by calling the guest's `rust_red_alloc`
/// and then copying the bytes.
///
/// Returns the guest pointer where data was written.
pub fn write_to_guest(
    store: &mut impl wasmtime::AsContextMut<Data = WasmNodeState>,
    memory: &Memory,
    alloc_fn: &wasmtime::TypedFunc<u32, u32>,
    data: &[u8],
) -> Result<u32> {
    let len = data.len() as u32;
    let ptr = alloc_fn.call(&mut *store, len).map_err(|e| anyhow::anyhow!("guest alloc failed: {e}"))?;

    memory.data_mut(&mut *store)[ptr as usize..][..len as usize].copy_from_slice(data);

    Ok(ptr)
}

/// Read a slice of bytes from guest linear memory.
pub fn read_from_guest(
    store: &impl wasmtime::AsContext<Data = WasmNodeState>,
    memory: &Memory,
    ptr: u32,
    len: u32,
) -> Result<Vec<u8>> {
    let ptr = ptr as usize;
    let len = len as usize;
    let mem_data = memory.data(store);
    if ptr + len > mem_data.len() {
        anyhow::bail!("read_from_guest: out-of-bounds access ptr={} len={} mem_size={}", ptr, len, mem_data.len());
    }
    Ok(mem_data[ptr..][..len].to_vec())
}

/// Read a string from guest linear memory.
pub fn read_string_from_guest(
    store: &impl wasmtime::AsContext<Data = WasmNodeState>,
    memory: &Memory,
    ptr: u32,
    len: u32,
) -> Result<String> {
    let bytes = read_from_guest(store, memory, ptr, len)?;
    String::from_utf8(bytes).context("guest string is not valid UTF-8")
}
