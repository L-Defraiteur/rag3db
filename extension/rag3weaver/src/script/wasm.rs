//! **WebAssembly scripts** (feature `wasm-nodes`, by wasmi; the choice is in
//! `docs/everything-declarative/21-11-2026/07-le-moteur-webassembly-wasmtime-ou-wasmi.md`).
//!
//! A module is given as its bytes in hexadecimal (the declaration reads the `.wasm` file and encodes it). It exports
//! `memory`, `alloc(len) -> ptr` and `run(ptr, len) -> u64`: the input is written as JSON at the pointer `alloc` gives,
//! and `run` returns the output's pointer and length packed in a `u64` (`ptr << 32 | len`), JSON too.
//!
//! **Isolated by construction**: a module sees only what the host offers, and this host offers nothing. A module that
//! imports anything (a file, the network, a clock) is refused at its preparation, naming the import. **Bounded at each
//! call**: a fresh instance, its memory under `memory_bytes`, its work under `fuel` (wasmi's deterministic count: the
//! same module on the same input stops at the same point, whatever the machine's load).
use super::{check_limits, Language, PreparedScript, ScriptEngine, ScriptError, ScriptErrorKind, ScriptLimits};
#[cfg(feature = "wasm-nodes")]
use super::{check_input, check_output};
#[cfg(feature = "wasm-nodes")]
use serde_json::Value;
use std::sync::Arc;

pub struct WasmEngine;

/// The bytes of a module as the text a declaration carries.
pub fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    let t = text.trim();
    if t.len() % 2 != 0 {
        return None;
    }
    (0..t.len()).step_by(2).map(|i| u8::from_str_radix(&t[i..i + 2], 16).ok()).collect()
}

impl ScriptEngine for WasmEngine {
    fn language(&self) -> Language {
        Language::Wasm
    }

    fn prepare(&self, source: &str, limits: &ScriptLimits) -> Result<Arc<dyn PreparedScript>, ScriptError> {
        check_limits(Language::Wasm, limits, "")?;
        let bytes = from_hex(source)
            .ok_or_else(|| ScriptError::new(ScriptErrorKind::Syntax, "a WebAssembly module is given as its bytes in hexadecimal"))?;
        if bytes.len() > limits.module_bytes {
            return Err(ScriptError::new(
                ScriptErrorKind::Limit,
                format!("WebAssembly module of {} bytes, over the host's {} (module_bytes)", bytes.len(), limits.module_bytes),
            ));
        }
        prepare_module(&bytes)
    }
}

#[cfg(not(feature = "wasm-nodes"))]
fn prepare_module(_bytes: &[u8]) -> Result<Arc<dyn PreparedScript>, ScriptError> {
    Err(ScriptError::new(
        ScriptErrorKind::Unsupported,
        "this build has no WebAssembly engine: it needs the feature wasm-nodes",
    ))
}

#[cfg(feature = "wasm-nodes")]
fn prepare_module(bytes: &[u8]) -> Result<Arc<dyn PreparedScript>, ScriptError> {
    let mut config = wasmi::Config::default();
    config.consume_fuel(true);
    let engine = wasmi::Engine::new(&config);
    let module = wasmi::Module::new(&engine, bytes).map_err(|e| ScriptError::new(ScriptErrorKind::Syntax, format!("WebAssembly: {e}")))?;
    if let Some(import) = module.imports().next() {
        let name = format!("{}::{}", import.module(), import.name());
        return Err(ScriptError::new(
            ScriptErrorKind::Forbidden(name.clone()),
            format!("WebAssembly: the module imports {name}, and this host offers nothing"),
        ));
    }
    for export in ["memory", "alloc", "run"] {
        if module.get_export(export).is_none() {
            return Err(ScriptError::new(
                ScriptErrorKind::MissingEntry,
                format!("WebAssembly: the module does not export `{export}` (memory, alloc(len) -> ptr, run(ptr, len) -> u64)"),
            ));
        }
    }
    Ok(Arc::new(PreparedWasm { engine, module }))
}

#[cfg(feature = "wasm-nodes")]
struct PreparedWasm {
    engine: wasmi::Engine,
    module: wasmi::Module,
}

#[cfg(feature = "wasm-nodes")]
impl PreparedScript for PreparedWasm {
    fn language(&self) -> Language {
        Language::Wasm
    }

    fn call(&self, input: &Value, limits: &ScriptLimits) -> Result<Value, ScriptError> {
        check_limits(Language::Wasm, limits, "")?;
        check_input(Language::Wasm, limits, input)?;
        let json = serde_json::to_vec(input).map_err(|e| ScriptError::new(ScriptErrorKind::Runtime, e.to_string()))?;
        let mut store = wasmi::Store::new(&self.engine, wasmi::StoreLimitsBuilder::new().memory_size(limits.memory_bytes).instances(1).trap_on_grow_failure(true).build());
        store.limiter(|l| l);
        store.set_fuel(limits.fuel).map_err(|e| ScriptError::new(ScriptErrorKind::Runtime, e.to_string()))?;
        let linker: wasmi::Linker<wasmi::StoreLimits> = wasmi::Linker::new(&self.engine);
        let runtime = |e: String| ScriptError::new(ScriptErrorKind::Runtime, format!("WebAssembly: {e}"));
        let instance = linker.instantiate_and_start(&mut store, &self.module).map_err(|e| runtime(e.to_string()))?;
        let memory = instance.get_memory(&store, "memory").ok_or_else(|| runtime("no exported memory".into()))?;
        let alloc = instance.get_typed_func::<u32, u32>(&store, "alloc").map_err(|e| runtime(e.to_string()))?;
        let run = instance.get_typed_func::<(u32, u32), u64>(&store, "run").map_err(|e| runtime(e.to_string()))?;
        // A trap is read for its cause: the fuel spent, the memory at its ceiling, or the module's own fault.
        let classify = |store: &wasmi::Store<wasmi::StoreLimits>, e: wasmi::Error| -> ScriptError {
            if store.get_fuel().is_ok_and(|f| f == 0) {
                return ScriptError::new(ScriptErrorKind::Timeout, format!("WebAssembly: all fuel consumed (fuel limit {})", limits.fuel));
            }
            // A growth the limiter refuses traps (`trap_on_grow_failure`) with its own code: read from the code, never
            // guessed from the size or the text.
            if e.as_trap_code() == Some(wasmi::TrapCode::GrowthOperationLimited) {
                return ScriptError::new(
                    ScriptErrorKind::Memory,
                    format!("WebAssembly: the memory reached the host's ceiling of {} bytes ({e})", limits.memory_bytes),
                );
            }
            runtime(e.to_string())
        };
        let len = u32::try_from(json.len()).map_err(|_| runtime("input over 4 GiB".into()))?;
        let at = alloc.call(&mut store, len).map_err(|e| classify(&store, e))?;
        memory.write(&mut store, at as usize, &json).map_err(|e| runtime(e.to_string()))?;
        let packed = run.call(&mut store, (at, len)).map_err(|e| classify(&store, e))?;
        let (ptr, n) = ((packed >> 32) as usize, (packed & 0xffff_ffff) as usize);
        let data = memory.data(&store);
        let out = data.get(ptr..ptr + n).ok_or_else(|| runtime(format!("the output ({ptr}, {n}) is outside the module's memory")))?;
        let value: Value =
            serde_json::from_slice(out).map_err(|e| ScriptError::new(ScriptErrorKind::Output, format!("WebAssembly: the output is not JSON: {e}")))?;
        check_output(Language::Wasm, limits, value)
    }
}
