//! Compile and export a search chain without opening the database or using MCP.
use rag3weaver::{backend::PreparedBackend, dataflow::search_chain::SearchProgram};
use serde_json::json;
use std::{env, path::Path};

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: search_chain_plan <backend.json> <search-program.json>".into());
    }
    let prepared = PreparedBackend::load(Path::new(&args[0]))?;
    let program: SearchProgram =
        serde_json::from_slice(&std::fs::read(&args[1]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let plan = prepared.compile_search_program(&program)?;
    let template = plan.template();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "plan": plan, "template": template,
        }))
        .map_err(|e| e.to_string())?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
