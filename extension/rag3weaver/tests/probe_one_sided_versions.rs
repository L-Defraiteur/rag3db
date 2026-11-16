//! **The versions one base has and the other has not** (probe, 16 November 2026, lucivy's ticket: the migrated copy A
//! and the fresh index B′ do not search the same contents, 911 and 879 identities on one side only). On **copies**, by
//! the raw connection only (no schema registration, nothing written): for a sample of the one-sided offsets, the
//! version's name, an occurrence's path, its content's length and ends, and the version of the same name and path in
//! the other base. Measures without failing.
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::Rag3dbConnection;

fn t(v: &CypherValue) -> String {
    match v {
        CypherValue::String(s) => s.clone(),
        CypherValue::Null => "∅".into(),
        other => format!("{other:?}"),
    }
}

fn ends(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let head: String = c.iter().take(90).collect();
    let tail: String = c.iter().rev().take(50).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{} chars | {:?} … {:?}", c.len(), head, tail)
}

fn describe(conn: &Rag3dbConnection, offset: i64) -> Option<(String, String, String)> {
    let r = conn
        .execute(&format!(
            "MATCH (n:ScopeContent) WHERE OFFSET(id(n)) = {offset} OPTIONAL MATCH (n)-[:ScopeContent_DERIVED_FROM]->(s:Scope) RETURN n.name, n.content, s.file_path, s._jusqu_a LIMIT 1"
        ))
        .ok()?;
    let row = r.rows.first()?;
    Some((t(&row[0]), t(&row[1]), format!("{} (_jusqu_a {})", t(&row[2]), t(&row[3]))))
}

fn same_name_and_path(conn: &Rag3dbConnection, name: &str, path: &str) -> Vec<String> {
    let path = path.split(" (").next().unwrap_or("");
    let q = format!(
        "MATCH (n:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s:Scope) WHERE n.name = '{}' AND s.file_path = '{}' AND s._jusqu_a IS NULL RETURN DISTINCT n.content LIMIT 3",
        name.replace('\'', "\\'"),
        path.replace('\'', "\\'")
    );
    conn.execute(&q).map(|r| r.rows.iter().map(|l| ends(&t(&l[0]))).collect()).unwrap_or_else(|e| vec![format!("error: {e}")])
}

#[test]
#[ignore]
fn sample_the_one_sided_versions() {
    let var = |n: &str| std::env::var(n).ok();
    let (Some(a), Some(b), Some(only_a), Some(only_b)) = (var("PROBE_A"), var("PROBE_B"), var("PROBE_ONLY_A"), var("PROBE_ONLY_B")) else {
        eprintln!("SKIPPED: PROBE_A, PROBE_B (copies), PROBE_ONLY_A, PROBE_ONLY_B (offset files) are needed");
        return;
    };
    let n: usize = var("PROBE_N").and_then(|v| v.parse().ok()).unwrap_or(8);
    let ca = Rag3dbConnection::new(&a).expect("open A");
    let cb = Rag3dbConnection::new(&b).expect("open B");
    for (label, file, here, there) in [("only in A", &only_a, &ca, &cb), ("only in B", &only_b, &cb, &ca)] {
        let offsets: Vec<i64> = std::fs::read_to_string(file).unwrap().lines().filter_map(|l| l.trim().parse().ok()).collect();
        eprintln!("[probe] {label}: {} offsets, sampling {n}", offsets.len());
        let step = (offsets.len() / n).max(1);
        for o in offsets.iter().step_by(step).take(n) {
            let Some((name, content, path)) = describe(here, *o) else {
                eprintln!("[probe]   {o}: not found");
                continue;
            };
            eprintln!("[probe]   {o}: {name} @ {path}");
            eprintln!("[probe]     here:  {}", ends(&content));
            for other in same_name_and_path(there, &name, &path) {
                eprintln!("[probe]     there: {other}");
            }
        }
    }
}
