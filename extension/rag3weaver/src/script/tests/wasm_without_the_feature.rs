//! The witness of the lot `wasm-node-trial` (21 November 2026) in a build without the feature `wasm-nodes`, the
//! service's: the language exists, and refuses by naming what it needs.
use super::*;

/// **Without the feature, a WebAssembly script is refused, said so**: a module, even a valid one, is `Unsupported`
/// and the message names the feature; a source that is not hexadecimal is still a syntax error, named.
#[test]
fn a_wasm_script_without_the_feature_names_the_feature() {
    // The smallest valid module: the magic number and the version.
    let e = prepare("wasm", "0061736d01000000", &ScriptLimits::default()).err().expect("refused without the engine");
    assert_eq!(e.kind, ScriptErrorKind::Unsupported, "{e}");
    assert!(e.to_string().contains("wasm-nodes"), "{e}");
    let e = prepare("wasm", "not hexadecimal", &ScriptLimits::default()).err().expect("refused");
    assert_eq!(e.kind, ScriptErrorKind::Syntax, "{e}");
    assert!(e.to_string().contains("hexadecimal"), "{e}");
}
