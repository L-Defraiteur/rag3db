//! **The parity of an embedding service between batches, measured by a client script**
//! (ticket `docs/embarquement/tickets/2026-11-16-la-preuve-de-parite-des-services-n-a-pas-de-tolerance-mesuree.md`).
//!
//! `scripts/embed_parity.py` embeds a fixed set of texts alone, then in batches of other sizes and orders, and
//! reports the gap to the vector computed alone. Here it runs against the deterministic provider of the witnesses
//! (`tests/fixtures/residence/fournisseur.py`, CPU, no model) : every gap must be 0 and every vector bit exact.
//! The real measure, on the 7878 and the 7879, is run by hand and written in the embarquement's rules page.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

/// **On the deterministic fixture, the parity is bit exact** in every batch shape.
#[test]
#[ignore]
fn embed_parity_on_the_fixture_is_bit_exact() {
    let root = env!("CARGO_MANIFEST_DIR");
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut fixture = Command::new("python3")
        .args(["-I", &format!("{root}/tests/fixtures/residence/fournisseur.py"), "--adresse", "127.0.0.1:0", "--generation", "parity", "--journal"])
        .arg(dir.path().join("journal.jsonl"))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the fixture provider starts");
    let mut line = String::new();
    let mut out = BufReader::new(fixture.stdout.take().expect("its stdout"));
    let address = loop {
        line.clear();
        if out.read_line(&mut line).expect("a line from the fixture") == 0 {
            let _ = fixture.kill();
            panic!("the fixture ended without saying its address");
        }
        if let Some(a) = line.trim().strip_prefix("adresse liée : ") {
            break a.to_string();
        }
    };
    let report = dir.path().join("parity.json");
    let run = Command::new("python3")
        .args(["-I", &format!("{root}/scripts/embed_parity.py"), "--address", &address, "--batches", "2,8", "--out"])
        .arg(&report)
        .output()
        .expect("python3 runs");
    let _ = fixture.kill();
    let _ = fixture.wait();
    assert!(run.status.success(), "embed_parity.py failed : {}", String::from_utf8_lossy(&run.stderr));
    let r: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&report).expect("the report")).expect("JSON");
    assert_eq!(r["texts"], 100, "the fixed set has 100 texts : {r}");
    assert_eq!(r["runs"].as_array().map(Vec::len), Some(7), "alone again, then 2 sizes × 3 orders : {r}");
    for key in ["same_shape", "between_batches"] {
        assert_eq!(r[key]["max_abs"], 0.0, "{key} : {r}");
        assert_eq!(r[key]["bit_exact_share"], 1.0, "{key} : {r}");
        assert_eq!(r[key]["min_cosine"], 1.0, "{key} : {r}");
    }
}
