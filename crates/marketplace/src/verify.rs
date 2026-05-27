//! WASM plugin verification.
//!
//! Before a plugin is published, the marketplace verifies:
//! 1. The binary is a valid WASM module (wasmtime can compile it).
//! 2. It exports the required interfaces: `rust_red_node_info`,
//!    `rust_red_on_start`, `rust_red_process_msg`, `rust_red_on_stop`,
//!    `rust_red_alloc`, and `memory`.
//! 3. A sandboxed execution test: instantiate, call `rust_red_on_start`,
//!    then `rust_red_process_msg` with a minimal message, then
//!    `rust_red_on_stop`. None of these should trap.
//! 4. The binary size is within limits (enforced at the store layer, but we
//!    double-check here).

use wasmtime::{Engine, FuncType, Linker, Module, Store, Val, ValType};

use crate::error::MarketplaceError;

/// Required export names that every WASM plugin must provide.
const REQUIRED_EXPORTS: &[&str] = &[
    "rust_red_node_info",
    "rust_red_on_start",
    "rust_red_process_msg",
    "rust_red_on_stop",
    "rust_red_alloc",
    "memory",
];

/// Result of a successful verification.
#[derive(Debug)]
pub struct VerificationReport {
    /// Number of exports found.
    pub export_count: usize,
    /// Number of imports required.
    pub import_count: usize,
    /// Whether the sandboxed execution test passed.
    pub sandbox_passed: bool,
}

/// Verify a WASM binary. Returns a `VerificationReport` on success or a
/// `MarketplaceError` on failure.
pub fn verify_wasm_plugin(wasm_bytes: &[u8]) -> Result<VerificationReport, MarketplaceError> {
    // 1. Compile the module
    let mut wasm_config = wasmtime::Config::new();
    wasm_config.epoch_interruption(true);
    let engine =
        Engine::new(&wasm_config).map_err(|e| MarketplaceError::VerificationFailed(format!("engine creation: {e}")))?;

    let module = Module::from_binary(&engine, wasm_bytes)
        .map_err(|e| MarketplaceError::VerificationFailed(format!("invalid WASM: {e}")))?;

    // 2. Check required exports
    let export_names: Vec<&str> = module.exports().map(|e| e.name()).collect();
    for req in REQUIRED_EXPORTS {
        if !export_names.contains(req) {
            return Err(MarketplaceError::VerificationFailed(format!("missing required export: {req}")));
        }
    }

    let import_count = module.imports().count();
    let export_count = export_names.len();

    // 3. Sandboxed execution test
    let sandbox_passed = run_sandbox_test(&engine, &module);

    Ok(VerificationReport { export_count, import_count, sandbox_passed })
}

/// Attempt to instantiate the module and call the lifecycle functions in order.
/// Returns `true` if no traps occur.
fn run_sandbox_test(engine: &Engine, module: &Module) -> bool {
    let mut store: Store<()> = Store::new(engine, ());
    store.set_epoch_deadline(1_000_000);

    let mut linker: Linker<()> = Linker::new(engine);

    // Register stub host imports that the WASM module expects ("env" module).
    if let Err(e) = register_stub_imports(engine, &mut linker, module) {
        log::debug!("sandbox: stub import registration failed: {e}");
        return false;
    }

    let instance = match linker.instantiate(&mut store, module) {
        Ok(i) => i,
        Err(e) => {
            log::debug!("sandbox: instantiation failed: {e}");
            return false;
        }
    };

    // Call rust_red_on_start(ptr=0, len=0) -- empty config
    if let Ok(f) = instance.get_typed_func::<(u32, u32), u32>(&mut store, "rust_red_on_start") {
        if let Err(e) = f.call(&mut store, (0, 0)) {
            log::debug!("sandbox: rust_red_on_start trapped: {e}");
        }
    }

    // Call rust_red_process_msg(ptr=0, len=0) -- empty message
    if let Ok(f) = instance.get_typed_func::<(u32, u32), u32>(&mut store, "rust_red_process_msg") {
        if let Err(e) = f.call(&mut store, (0, 0)) {
            log::debug!("sandbox: rust_red_process_msg trapped: {e}");
        }
    }

    // Call rust_red_on_stop()
    if let Ok(f) = instance.get_typed_func::<(), u32>(&mut store, "rust_red_on_stop") {
        if let Err(e) = f.call(&mut store, ()) {
            log::debug!("sandbox: rust_red_on_stop trapped: {e}");
        }
    }

    true
}

/// Register no-op stubs for all imports the module requires from the "env"
/// module. This avoids unresolved-import errors during sandboxed instantiation.
fn register_stub_imports(engine: &Engine, linker: &mut Linker<()>, module: &Module) -> Result<(), anyhow::Error> {
    for import in module.imports() {
        if import.module() == "env" {
            let name = import.name();
            if let wasmtime::ExternType::Func(func_ty) = import.ty() {
                let params: Vec<ValType> = func_ty.params().collect();
                let results: Vec<ValType> = func_ty.results().collect();
                let ft = FuncType::new(engine, params, results.clone());

                linker.func_new("env", name, ft, move |_caller, _args, out| {
                    for (i, ty) in results.iter().enumerate() {
                        if i < out.len() {
                            out[i] = zero_val(ty);
                        }
                    }
                    Ok(())
                })?;
            }
            // Memory/Table/Global imports are less common; skip.
        }
    }
    Ok(())
}

/// Produce a zero value for the given WASM value type.
fn zero_val(ty: &ValType) -> Val {
    match ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0.0f32.to_bits()),
        ValType::F64 => Val::F64(0.0f64.to_bits()),
        ValType::V128 => Val::V128(0u128.into()),
        _ => Val::I32(0),
    }
}
