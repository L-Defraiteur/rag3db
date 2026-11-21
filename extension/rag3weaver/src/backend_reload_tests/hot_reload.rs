//! The witness of the lot `scripted-nodes-hot-reload` (21 November 2026): a declared node edited in `nodes/` serves at
//! the next call of an open backend, without a reload asked and without reading its files at every call.
use super::*;

/// **An edited node serves at the next call, and an unchanged folder reloads nothing** (Lucie, 21 November 2026:
/// « que le code devienne un peu de la pâte à modeler » ; the trial of the scripted node: an open backend never read
/// its `nodes/` again). The node's `.ts` is edited: the next call doubles by three, no `reload()` asked. Two calls with
/// nothing changed reload nothing. A broken edit is refused, and the version before keeps serving.
#[test]
fn an_edited_node_serves_at_the_next_call_without_a_reload() {
    let (dir, backend) = toy_with_nodes();
    assert_eq!(doubled(&backend, 21), json!({"n": 42}));
    write(&dir.path().join("nodes"), "double.ts", &DOUBLE_TS.replace("* 2", "* 3"));
    assert_eq!(doubled(&backend, 21), json!({"n": 63}), "the edited node serves at the next call");
    let before = backend.automatic_reloads();
    assert_eq!(doubled(&backend, 21), json!({"n": 63}));
    assert_eq!(doubled(&backend, 21), json!({"n": 63}));
    assert_eq!(backend.automatic_reloads(), before, "a call with nothing changed reloads nothing");
    write(&dir.path().join("nodes"), "double.ts", "function run({ inputs }) { return { doubled: ( }; }");
    assert_eq!(doubled(&backend, 21), json!({"n": 63}), "a broken edit is refused, the version before serves");
}
