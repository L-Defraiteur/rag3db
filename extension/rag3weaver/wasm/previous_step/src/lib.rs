//! **The teammate's « Previous step » block, as a WebAssembly node**: the same text as `StepWork::block` (Rust) and
//! `previous_step.ts`, from the same JSON input `{inputs: {work}, config: {style}}`, rendered as `{block}`.
//!
//! The ABI the host expects (`rag3weaver::script::wasm`): `memory`, `alloc(len) -> ptr`, and `run(ptr, len) -> u64`,
//! the output's pointer and length packed as `ptr << 32 | len`. No import: the host offers none.
use serde_json::{json, Value};

#[no_mangle]
pub extern "C" fn alloc(len: u32) -> u32 {
    let mut v = Vec::<u8>::with_capacity(len as usize);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p as u32
}

#[no_mangle]
pub extern "C" fn run(ptr: u32, len: u32) -> u64 {
    let input = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
    let v: Value = serde_json::from_slice(input).unwrap_or(Value::Null);
    let style = v["config"]["style"].as_str().unwrap_or("full");
    let out = serde_json::to_vec(&json!({ "block": block(&v["inputs"]["work"], style) })).unwrap_or_default();
    let out = out.into_boxed_slice();
    let n = out.len() as u64;
    let p = Box::into_raw(out) as *mut u8 as u64;
    (p << 32) | n
}

fn block(w: &Value, style: &str) -> String {
    let strings = |k: &str| -> Vec<String> {
        w[k].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
    };
    let (read, wrote) = (strings("read"), strings("wrote"));
    let last = w["last_text"].as_str().unwrap_or("").trim().to_string();
    let budget = w["budget"].as_u64().unwrap_or(8000) as usize;
    let command = w["command"]
        .as_object()
        .map(|c| (c["command"].as_str().unwrap_or("").to_string(), c["output"].as_str().unwrap_or("").to_string()));
    if read.is_empty() && wrote.is_empty() && command.is_none() && last.is_empty() {
        return String::new();
    }
    let mut b = format!("\n## Previous step (step {})\n", w["number"].as_u64().unwrap_or(0));
    if !read.is_empty() {
        b.push_str(&format!("Read: {}\n", read.join(", ")));
    }
    if !wrote.is_empty() {
        b.push_str(&format!("Wrote: {}\n", wrote.join(", ")));
    }
    if let Some((c, out)) = command {
        if style == "compact" {
            b.push_str(&format!("Last command: {c}\n"));
        } else {
            b.push_str(&format!("Last command: {c}\nIts output ends with:\n{out}\n"));
        }
    }
    if !last.is_empty() {
        b.push_str(&format!("Your last text: {last}\n"));
    }
    b.push_str("Lines you read in that step can be replaced now without reading them again.\n");
    if b.chars().count() > budget {
        let cut: String = b.chars().take(budget.saturating_sub(80)).collect();
        b = format!("{cut}\n[Previous step cut at {budget} characters: teammate.carry_over_chars]\n");
    }
    b
}
