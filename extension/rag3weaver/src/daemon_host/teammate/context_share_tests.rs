//! **A step's context bound follows the model's window** (the night of 24 Nov 2026: a step of Gemini 3 Pro, a window
//! of a million tokens, was stopped at 64 000 in the middle of its search, and the next step invented the names it had
//! found). The bound is the larger of `teammate.step_context_tokens` and a declared share of the window
//! (`teammate.step_context_share`).

use super::*;
use serde_json::json;

fn lire() -> ToolCall {
    ToolCall::new("c1", "read_file", json!({ "path": "src/lib.rs" }).to_string())
}

#[test]
fn a_step_of_a_large_window_is_not_stopped_at_the_fixed_bound() {
    let d = tempfile::tempdir().unwrap();
    let s = Step::new(TeammateFiles::new(d.path().join("boite"), "t"), "t", "r", None, 1);
    let m = s.context_meter();
    record_context(&m, &json!({ "event": "generation_end", "prompt_tokens": 100_000, "completion_tokens": 500, "context_window": 1_048_576 }));
    assert!(s.refusal(&lire()).is_none(), "100 500 tokens of a 1 048 576-token window: a quarter of it is the bound, not 64 000");
    record_context(&m, &json!({ "event": "generation_end", "prompt_tokens": 300_000, "completion_tokens": 500, "context_window": 1_048_576 }));
    assert!(s.refusal(&lire()).is_some(), "past a quarter of the window, the step is full");
}

#[test]
fn a_small_or_unknown_window_keeps_the_fixed_bound() {
    let d = tempfile::tempdir().unwrap();
    let s = Step::new(TeammateFiles::new(d.path().join("boite"), "t"), "t", "r", None, 1);
    let m = s.context_meter();
    record_context(&m, &json!({ "event": "generation_end", "prompt_tokens": 70_000, "completion_tokens": 0 }));
    assert!(s.refusal(&lire()).is_some(), "no window said: the fixed bound, 64 000");
    record_context(&m, &json!({ "event": "generation_end", "prompt_tokens": 60_000, "completion_tokens": 0, "context_window": 131_072 }));
    assert!(s.refusal(&lire()).is_none(), "under the fixed bound");
}
