//! **The night probe of the lucivy dictionaries** (20 November 2026, after the lost words of rag3weaver's code index).
//! For each index directory given (a `Lucivy_*` copy, never the live one: opening may write a lock), opens it and runs
//! `check_dictionary`, then prints one JSON line per field: `{"index", "field", "referenced", "unknown", "in_reprieve"}`.
//! An index that does not open prints `{"index", "error"}`. Exit code 0 in both cases: the judging is the script's
//! (`tools/poste/lucivy_sonde.sh`), against the known losses.

use lucivy_core::sharded_handle::ShardedHandle;

fn main() {
    for path in std::env::args().skip(1) {
        let index = std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.clone());
        let line = match ShardedHandle::open(&path).and_then(|h| h.check_dictionary()) {
            Err(e) => {
                println!("{}", serde_json::json!({ "index": index, "error": e.to_string() }));
                continue;
            }
            Ok(check) => check,
        };
        // Fields come per shard (`shard_<n>/<field>`): summed per field.
        let mut fields: std::collections::BTreeMap<String, (u64, u64, u64)> = std::collections::BTreeMap::new();
        for f in &line.fields {
            let name = f.field.rsplit('/').next().unwrap_or(&f.field).to_string();
            let e = fields.entry(name).or_default();
            e.0 += f.referenced;
            e.1 += f.unknown;
            e.2 += f.in_reprieve;
        }
        for (field, (referenced, unknown, in_reprieve)) in fields {
            println!(
                "{}",
                serde_json::json!({ "index": index, "field": field, "referenced": referenced, "unknown": unknown, "in_reprieve": in_reprieve })
            );
        }
    }
}
