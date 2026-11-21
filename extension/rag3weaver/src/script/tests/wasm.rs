//! The witness of the lot `wasm-node-trial` (21 November 2026): a WebAssembly script is bounded and isolated.
use super::*;

/// A module whose `run` loops forever, one whose `run` grows its memory until refused, one that imports a file
/// reader: built from text by `wat` (the trial's bench, `ed-wasm-essai/wat2hex` at luciepc).
const SPIN: &str = "0061736d01000000010c0260017f017f60027f7f017e03030200010503010001071803066d656d6f7279020005616c6c6f6300000372756e00010a110205004180080b090003400c000b42000b000d046e616d65030601010100016c";
const HOG: &str = "0061736d01000000010c0260017f017f60027f7f017e03030200010503010001071803066d656d6f7279020005616c6c6f6300000372756e00010a1c0205004180080b1400034041104000417f460440000b0c000b42000b000d046e616d65030601010100016c";
const IMPORTS_A_FILE: &str = "0061736d01000000010c0260017f017f60027f7f017e02110103656e7609726561645f66696c65000003030200010503010001071803066d656d6f7279020005616c6c6f6300010372756e00020a0c0205004180080b040042000b";

/// **A WebAssembly script is bounded and isolated** (the trial of 21 November 2026): a loop that never ends stops on
/// its fuel, said as such; a memory that never stops growing stops at the host's ceiling, said as such; a module that
/// imports a file reader is refused at its preparation, naming the import.
#[test]
fn a_wasm_script_is_bounded_by_its_fuel_and_memory_and_isolated() {
    let input = serde_json::json!({"inputs": {}, "config": {}});
    let limits = ScriptLimits { fuel: 10_000_000, ..Default::default() };
    let e = prepare("wasm", SPIN, &limits).unwrap().call(&input, &limits).err().expect("the loop is stopped");
    assert_eq!(e.kind, ScriptErrorKind::Timeout, "{e}");
    assert!(e.to_string().contains("fuel"), "{e}");
    let limits = ScriptLimits { memory_bytes: 4 * 1024 * 1024, fuel: u64::MAX / 2, ..Default::default() };
    let e = prepare("wasm", HOG, &limits).unwrap().call(&input, &limits).err().expect("the memory is stopped");
    assert_eq!(e.kind, ScriptErrorKind::Memory, "{e}");
    let e = prepare("wasm", IMPORTS_A_FILE, &ScriptLimits::default()).err().expect("an import is refused");
    assert_eq!(e.kind, ScriptErrorKind::Forbidden("env::read_file".into()), "{e}");
}
