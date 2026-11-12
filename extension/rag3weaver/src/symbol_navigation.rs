//! Navigation bornée dans les faits de l'index. Les plages sont à granularité
//! ligne : leur inclusion est une relation dérivée, jamais une preuve AST,
//! d'appartenance à un type ou d'héritage.
use crate::code_tools::{indexed_name, FileSource};
use serde_json::{json, Value};

pub const MAX_INPUT: usize = 2000;
pub const MAX_NODES: usize = 200;
pub const MAX_DEPTH: usize = 16;
const MAX_BYTES: usize = 96 * 1024;

pub struct Navigation<'a> {
    pub path: &'a str,
    pub source: &'a str,
    pub mode: &'a str,
    pub symbol: &'a str,
    pub depth: usize,
    pub limit: usize,
}
fn text<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}
fn number(v: &Value, k: &str) -> Option<u64> {
    v[k].as_u64()
}
fn clipped(s: &str, max: usize) -> String {
    let mut t: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        t.push('…');
    }
    t
}
fn node(row: &Value, depth: usize, parent: Option<&str>) -> Value {
    json!({"ref": {"uuid": row["_uuid"], "source": row["source"], "path": row["file_path"]},
        "name": clipped(text(row,"name"),256), "kind": row["scope_type"],
        "signature": clipped(text(row,"signature"),512), "start_line": row["start_line"],
        "end_line": row["end_line"], "depth":depth, "parent_uuid":parent,
        "declared_parent_name":row["parent_name"]})
}
fn interval(row: &Value) -> Option<(u64, u64)> {
    let (a, b) = (number(row, "start_byte")?, number(row, "end_byte")?);
    (a < b).then_some((a, b))
}
fn contains(parent: &Value, child: &Value) -> bool {
    match (interval(parent), interval(child)) {
        (Some((a, b)), Some((c, d))) => a <= c && d <= b && (a, b) != (c, d),
        _ => false,
    }
}
/// Tous les statuts passent par le même plafond, y compris les refus et
/// les candidats ambigus. Les références retirées ne sont pas tronquées.
fn bounded(mut out: Value) -> Value {
    for key in ["nodes", "members", "candidates"] {
        if let Some(a) = out[key].as_array_mut() {
            if a.len() > MAX_NODES {
                a.truncate(MAX_NODES);
                out["truncated"] = json!(true);
            }
        }
    }
    while out.to_string().len() > MAX_BYTES {
        let mut removed = false;
        for key in ["members", "nodes", "candidates"] {
            if out[key].as_array_mut().is_some_and(|a| a.pop().is_some()) {
                removed = true;
                break;
            }
        }
        if !removed {
            removed = out["coverage"]["ambiguous_ranges"]
                .as_array_mut()
                .is_some_and(|a| a.pop().is_some());
        }
        if !removed {
            return json!({"status":"output_budget_exceeded", "status_before_budget":clipped(text(&out,"status"),64),
                "nodes":[], "members":[], "candidates":[], "truncated":true});
        }
        out["truncated"] = json!(true);
    }
    out
}

pub fn navigate(
    rows: &[Value],
    total: usize,
    source: Option<&dyn FileSource>,
    q: Navigation<'_>,
) -> Result<Value, String> {
    navigate_inner(rows, total, source, q).map(bounded)
}

/// Les lignes/total viennent d'un seul Plan dans la vue de l'appel. Aucun
/// voisin n'est inventé depuis un nom de parent ; aucun corps n'est rendu.
fn navigate_inner(
    rows: &[Value],
    total: usize,
    source: Option<&dyn FileSource>,
    q: Navigation<'_>,
) -> Result<Value, String> {
    if !["file", "parents", "children", "members"].contains(&q.mode) {
        return Err("mode: file | parents | children | members".into());
    }
    if q.depth > MAX_DEPTH || q.limit == 0 || q.limit > MAX_NODES {
        return Err(format!("depth: 0..{MAX_DEPTH}, limit: 1..{MAX_NODES}"));
    }
    if q.mode != "file" && q.symbol.is_empty() {
        return Err("symbol requis (UUID ou nom exact dans path/source)".into());
    }
    let mut out = json!({"mode":q.mode,"path":q.path,"source":q.source,"status":"ready","depth":q.depth,
        "nodes":[],"members":[],"candidates":[],"total_indexed_scopes":total,"total_selected":0,
        "truncated":false,"provenance":{"relation":"range_containment","position_precision":"line_ranges",
        "freshness":"unverified","not_relations":["declared_membership","inheritance"]},
        "coverage":{"complete":false,"limits":["Scopes indexés seulement ; lambdas repliées et membres non extraits peuvent manquer.",
        "Inclusion de plages à granularité ligne, pas parent AST garanti ; méthodes hors classe non rattachées par ce parcours.",
        "Membres : scopes directs de plages, déclarations méthode/fonction et propriétés typées ; faits séparés, aucun inventaire exhaustif."]}});
    // Une tranche de scopes ne permet pas de décider les racines/parents.
    if total > MAX_INPUT || rows.len() < total {
        out["status"] = json!("input_truncated");
        out["truncated"] = json!(true);
        return Ok(out);
    }
    let mut selected: Vec<&Value> = rows
        .iter()
        .filter(|r| q.source.is_empty() || text(r, "source") == q.source)
        .collect();
    let sources: std::collections::BTreeSet<_> =
        selected.iter().map(|r| text(r, "source")).collect();
    if sources.len() > 1 {
        out["status"] = json!("ambiguous_source");
        out["candidates"] = json!(sources);
        return Ok(out);
    }
    if selected.is_empty() {
        out["status"] = json!("no_indexed_scopes");
        return Ok(out);
    }
    let src = text(selected[0], "source");
    out["source"] = json!(src);
    // Le hash de File est porté par DEFINED_IN dans la même lecture du Plan.
    let Some(files) = selected[0]["indexed_file"].as_array() else {
        out["status"] = json!("unverified_file");
        return Ok(out);
    };
    if files.len() != 1 {
        out["status"] = json!("unverified_file");
        return Ok(out);
    }
    let file = &files[0];
    let hash = text(file, "content_hash");
    let coherent = selected.iter().all(|r| {
        text(r, "file_path") == q.path
            && text(r, "source") == src
            && r["indexed_file"]
                .as_array()
                .is_some_and(|fs| fs.len() == 1 && fs[0] == *file)
    });
    if !coherent || hash.is_empty() {
        out["status"] = json!("unverified_file");
        return Ok(out);
    }
    out["provenance"]["indexed_hash"] = json!(hash);
    out["provenance"]["indexed_cursor"] = file["cursor"].clone();
    let Some(source) = source else {
        out["status"] = json!("source_unavailable");
        return Ok(out);
    };
    let (actual_src, actual_path) = indexed_name(source, q.path);
    if actual_src != src || actual_path != q.path {
        out["status"] = json!("source_unavailable");
        return Ok(out);
    }
    out["provenance"]["source_cursor"] = json!(source.cursor());
    let Some(content) = source.read(q.path)? else {
        out["status"] = json!("file_missing");
        return Ok(out);
    };
    let actual = crate::hash::content_hash(&content);
    out["provenance"]["actual_hash"] = json!(actual);
    if actual != hash {
        out["status"] = json!("stale_index");
        out["provenance"]["freshness"] = json!("stale");
        return Ok(out);
    }
    out["provenance"]["freshness"] = json!("matches_observed_content");
    if selected.iter().any(|r| {
        text(r, "_uuid").is_empty() || interval(r).is_none_or(|(_, end)| end > content.len() as u64)
    }) {
        out["status"] = json!("invalid_positions");
        return Ok(out);
    }
    selected.sort_by_key(|r| {
        (
            number(r, "start_byte"),
            std::cmp::Reverse(number(r, "end_byte")),
            text(r, "_uuid"),
        )
    });
    let mut parents = vec![None; selected.len()];
    let mut ambiguous = Vec::new();
    let mut comparisons = 0usize;
    // Deux scans par scope, pas une comparaison de chaque paire de parents
    // pour chaque enfant. Un candidat minimal ne suffit que si tous les
    // autres le contiennent ; des plages croisées restent ambiguës.
    for (i, child) in selected.iter().enumerate() {
        let mut candidates = Vec::new();
        for (j, parent) in selected.iter().enumerate() {
            comparisons += 1;
            if i != j && contains(parent, child) {
                candidates.push(j);
            }
            if i != j && interval(parent) == interval(child) {
                ambiguous.push(text(child, "_uuid"));
            }
        }
        if let Some(&best) = candidates.iter().min_by_key(|&&j| {
            let (a, b) = interval(selected[j]).unwrap();
            b - a
        }) {
            let mut unique = true;
            for &j in &candidates {
                comparisons += 1;
                if j != best && !contains(selected[j], selected[best]) {
                    unique = false;
                }
            }
            if unique {
                parents[i] = Some(best);
            } else {
                ambiguous.push(text(child, "_uuid"));
            }
        }
    }
    ambiguous.sort_unstable();
    ambiguous.dedup();
    out["coverage"]["ambiguous_range_count"] = json!(ambiguous.len());
    out["coverage"]["ambiguous_ranges"] = json!(ambiguous);
    out["coverage"]["parent_comparisons"] = json!(comparisons);
    let root = if q.mode == "file" {
        None
    } else {
        let found: Vec<usize> = selected
            .iter()
            .enumerate()
            .filter(|(_, r)| text(r, "_uuid") == q.symbol || text(r, "name") == q.symbol)
            .map(|(i, _)| i)
            .collect();
        if found.len() != 1 {
            out["status"] = json!(if found.is_empty() {
                "symbol_not_found"
            } else {
                "ambiguous_symbol"
            });
            out["candidates"] = json!(found
                .iter()
                .take(q.limit)
                .map(|&i| node(selected[i], 0, None))
                .collect::<Vec<_>>());
            out["truncated"] = json!(found.len() > q.limit);
            return Ok(out);
        }
        Some(found[0])
    };
    let mut nodes = Vec::new();
    if q.mode == "file" {
        nodes.push(json!({"kind":"file","path":q.path,"source":src,"depth":0}));
    }
    // La stricte inclusion diminue la plage à chaque descente : pas de cycle.
    // La visite garde néanmoins un ensemble pour borner toute évolution future.
    let mut queue: std::collections::VecDeque<(usize, usize)> = match root {
        Some(i) => [(i, 0)].into(),
        None => parents
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_none())
            .map(|(i, _)| (i, 1))
            .collect(),
    };
    let mut visited = std::collections::BTreeSet::new();
    let mut omitted_depth = false;
    while let Some((i, depth)) = queue.pop_front() {
        if !visited.insert(i) {
            continue;
        }
        if depth > q.depth {
            omitted_depth = true;
            continue;
        }
        nodes.push(node(
            selected[i],
            depth,
            parents[i].map(|p| text(selected[p], "_uuid")),
        ));
        if q.mode == "parents" {
            if let Some(p) = parents[i] {
                queue.push_back((p, depth + 1));
            }
        } else if q.mode != "members" || depth == 0 {
            queue.extend(
                parents
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| **p == Some(i))
                    .map(|(j, _)| (j, depth + 1)),
            );
        }
    }
    let mut members = Vec::new();
    if q.mode == "members" && q.depth > 0 {
        let r = selected[root.unwrap()];
        for (field, origin) in [
            ("declarations", "indexed_declaration"),
            ("field_types", "indexed_typed_property"),
        ] {
            if let Ok(Value::Array(facts)) = serde_json::from_str::<Value>(text(r, field)) {
                for fact in facts {
                    // Identité de graphe et position absentes ne sont jamais fabriquées.
                    members.push(json!({"name":clipped(text(&fact,"name"),256),"kind":if field=="field_types"{"property"}else{text(&fact,"kind")},
                        "signature":clipped(text(&fact,"signature"),512),"type":fact["type"],"line":fact["line"],
                        "owner_ref":node(r,0,None)["ref"],"provenance":origin}));
                }
            } else if !text(r, field).is_empty() {
                out["coverage"]["malformed_member_facts"] = json!(true);
            }
        }
    }
    let total_selected = nodes.len() + members.len();
    while nodes.len() + members.len() > q.limit {
        if !members.is_empty() {
            members.pop();
        } else {
            nodes.pop();
        }
    }
    out["nodes"] = json!(nodes);
    out["members"] = json!(members);
    out["total_selected"] = json!(total_selected);
    out["truncated"] = json!(total_selected > q.limit || omitted_depth);
    Ok(out)
}

pub fn markdown(v: &Value) -> String {
    let mut out = format!(
        "**Navigation {}** `{}` — {}\n\n",
        text(v, "mode"),
        text(v, "path"),
        text(v, "status")
    );
    out.push_str(&format!(
        "Source `{}` · fraîcheur {}.\n",
        text(v, "source"),
        text(&v["provenance"], "freshness")
    ));
    if v["coverage"]["ambiguous_range_count"].as_u64().unwrap_or(0) > 0 {
        out.push_str(&format!("Parent non déterminable pour {} plage(s) égale(s) ou croisée(s) ; aucun parent choisi arbitrairement.\n", v["coverage"]["ambiguous_range_count"]));
    }
    if v["coverage"]["malformed_member_facts"] == true {
        out.push_str("Faits de membres invalides : inventaire incomplet.\n");
    }
    out.push_str("Plages de lignes dérivées, pas parent AST, appartenance ni héritage. Couverture partielle.\n");
    for row in v["nodes"].as_array().into_iter().flatten() {
        let depth = row["depth"].as_u64().unwrap_or(0) as usize;
        if row["kind"] == "file" {
            out.push_str(&format!("- fichier `{}`\n", text(row, "path")));
            continue;
        }
        out.push_str(&format!(
            "{}- `{}` {} {}:{}-{} · uuid={}\n",
            "  ".repeat(depth),
            text(row, "name"),
            text(row, "kind"),
            text(&row["ref"], "path"),
            row["start_line"],
            row["end_line"],
            text(&row["ref"], "uuid")
        ));
    }
    for row in v["members"].as_array().into_iter().flatten() {
        out.push_str(&format!(
            "- {} `{}` {} · {}\n",
            text(row, "kind"),
            text(row, "name"),
            text(row, "signature"),
            text(row, "provenance")
        ));
    }
    if v["truncated"] == true {
        out.push_str("Résultat tronqué par profondeur/limite/budget ; voir format=json.\n");
    }
    if v["status"] != "ready" {
        out.push_str(&format!("{}\n", v));
    }
    if out.len() > MAX_BYTES {
        let mut compact = String::new();
        for line in out.lines() {
            if compact.len() + line.len() + 100 > MAX_BYTES {
                break;
            }
            compact.push_str(line);
            compact.push('\n');
        }
        compact.push_str(
            "Sortie tronquée par le budget de rendu ; utiliser format=json et réduire limit.\n",
        );
        return compact;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_tools::Snapshot;
    fn fixture() -> (Snapshot, Vec<Value>) {
        let content="class A:\n    def run(self):\n        pass\nclass B:\n    def run(self):\n        pass\n";
        let src = Snapshot::new("nav", [("a.py".into(), content.into())]);
        let h = crate::hash::content_hash(content);
        let file = json!([{"source":"snapshot:nav","path":"a.py","content_hash":h,"cursor":"snapshot:nav"}]);
        let mk = |id: &str, name: &str, kind: &str, start: u64, end: u64, line: u64| {
            json!({"_uuid":id,"name":name,"scope_type":kind,
            "source":"snapshot:nav","file_path":"a.py","start_byte":start,"end_byte":end,"start_line":line,"end_line":line+2,
            "signature":format!("{name}(…)"),"parent_name":"","indexed_file":file,"declarations":"","field_types":""})
        };
        (
            src,
            vec![
                mk("a", "A", "class", 0, 40, 1),
                mk("ar", "run", "method", 9, 40, 2),
                mk("b", "B", "class", 41, 81, 4),
                mk("br", "run", "method", 50, 81, 5),
            ],
        )
    }
    fn call(
        src: &Snapshot,
        rows: &[Value],
        mode: &str,
        symbol: &str,
        depth: usize,
        limit: usize,
    ) -> Value {
        navigate(
            rows,
            rows.len(),
            Some(src),
            Navigation {
                path: "a.py",
                source: "",
                mode,
                symbol,
                depth,
                limit,
            },
        )
        .unwrap()
    }
    #[test]
    fn depths_and_qualified_roundtrip_without_bodies() {
        let (src, rows) = fixture();
        let zero = call(&src, &rows, "file", "", 0, 80);
        assert_eq!(zero["nodes"].as_array().unwrap().len(), 1);
        let one = call(&src, &rows, "file", "", 1, 80);
        assert_eq!(one["nodes"].as_array().unwrap().len(), 3);
        let two = call(&src, &rows, "file", "", 2, 80);
        assert_eq!(two["nodes"].as_array().unwrap().len(), 5);
        assert!(!two.to_string().contains("pass"));
        let child = call(&src, &rows, "children", "a", 1, 80);
        assert_eq!(child["nodes"][1]["ref"]["uuid"], "ar");
        let parent = call(&src, &rows, "parents", "ar", 1, 80);
        assert_eq!(parent["nodes"][1]["ref"]["uuid"], "a");
        assert_eq!(parent["provenance"]["relation"], "range_containment");
    }
    #[test]
    fn names_sources_and_equal_ranges_never_choose_arbitrarily() {
        let (src, mut rows) = fixture();
        assert_eq!(
            call(&src, &rows, "parents", "run", 1, 80)["status"],
            "ambiguous_symbol"
        );
        let mut other = rows[0].clone();
        other["source"] = json!("another");
        rows.push(other);
        assert_eq!(
            call(&src, &rows, "file", "", 2, 80)["status"],
            "ambiguous_source"
        );
        rows.pop();
        rows[1]["start_byte"] = rows[0]["start_byte"].clone();
        let v = call(&src, &rows, "parents", "ar", 2, 80);
        assert_eq!(v["nodes"].as_array().unwrap().len(), 1);
        assert!(!v["coverage"]["ambiguous_ranges"]
            .as_array()
            .unwrap()
            .is_empty());
    }
    #[test]
    fn stale_missing_and_unverified_files_do_not_claim_current_neighbors() {
        let (src, mut rows) = fixture();
        src.insert("a.py", "changed");
        let v = call(&src, &rows, "children", "a", 2, 80);
        assert_eq!(v["status"], "stale_index");
        assert_eq!(v["nodes"], json!([]));
        src.remove("a.py");
        assert_eq!(
            call(&src, &rows, "file", "", 2, 80)["status"],
            "file_missing"
        );
        rows[0]["indexed_file"] = json!([]);
        assert_eq!(
            call(&src, &rows, "file", "", 2, 80)["status"],
            "unverified_file"
        );
    }
    #[test]
    fn limits_refuse_partial_input_and_bound_the_result() {
        let (src, rows) = fixture();
        let v = navigate(
            &rows,
            MAX_INPUT + 1,
            Some(&src),
            Navigation {
                path: "a.py",
                source: "",
                mode: "file",
                symbol: "",
                depth: 2,
                limit: 2,
            },
        )
        .unwrap();
        assert_eq!(v["status"], "input_truncated");
        assert_eq!(v["nodes"], json!([]));
        let v = call(&src, &rows, "file", "", 2, 2);
        assert_eq!(v["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(v["truncated"], true);
    }
    #[test]
    fn ambiguous_results_and_markdown_share_the_budget() {
        let (src, rows) = fixture();
        let mut many = Vec::new();
        for i in 0..256 {
            let mut r = rows[0].clone();
            r["_uuid"] = json!(format!("scope-{i}"));
            r["name"] = json!("same");
            r["signature"] = json!("é".repeat(512));
            many.push(r);
        }
        let v = call(&src, &many, "parents", "same", 1, 200);
        assert_eq!(v["status"], "ambiguous_symbol");
        assert!(v.to_string().len() <= MAX_BYTES);
        assert_eq!(v["truncated"], true);
        assert!(markdown(&v).len() <= MAX_BYTES);
        for (i, r) in many.iter_mut().enumerate() {
            r["source"] = json!(format!("{i}{}", "é".repeat(512)));
        }
        let v = call(&src, &many, "file", "", 2, 200);
        assert_eq!(v["status"], "ambiguous_source");
        assert!(v.to_string().len() <= MAX_BYTES);
        assert_eq!(v["truncated"], true);
    }
    #[test]
    fn parent_work_is_quadratic_and_default_render_exposes_unknowns() {
        let (src, rows) = fixture();
        let content = "x".repeat(256);
        src.insert("a.py", &content);
        let mut chain = Vec::new();
        for i in 0..64 {
            let mut r = rows[0].clone();
            r["_uuid"] = json!(format!("scope-{i}"));
            r["start_byte"] = json!(i);
            r["end_byte"] = json!(256 - i);
            r["indexed_file"][0]["content_hash"] = json!(crate::hash::content_hash(&content));
            chain.push(r);
        }
        let v = call(&src, &chain, "children", "scope-0", 16, 200);
        assert!(v["coverage"]["parent_comparisons"].as_u64().unwrap() <= 2 * 64 * 64);
        let (src, mut rows) = fixture();
        rows[1]["start_byte"] = rows[0]["start_byte"].clone();
        rows[0]["field_types"] = json!("bad-json");
        let v = call(&src, &rows, "members", "a", 1, 80);
        let rendered = markdown(&v);
        assert!(rendered.contains("Parent non déterminable"));
        assert!(rendered.contains("Faits de membres invalides"));
    }
    #[test]
    fn member_facts_are_partial_without_fabricated_ids_or_positions() {
        let (src, mut rows) = fixture();
        rows[0]["declarations"] =
            json!(r#"[{"name":"run","line":2,"signature":"run(self)","kind":"method"}]"#);
        rows[0]["field_types"] = json!(r#"[{"name":"typed","type":"int"}]"#);
        let v = call(&src, &rows, "members", "a", 1, 80);
        assert!(
            v["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["ref"]["uuid"] == "ar"),
            "une méthode indexée sans déclaration doit rester explorable : {v}"
        );
        assert_eq!(v["members"].as_array().unwrap().len(), 2);
        assert_eq!(v["coverage"]["complete"], false);
        assert!(v["members"][1]["line"].is_null());
        assert!(v["members"][1].get("ref").is_none());
        assert_eq!(
            call(&src, &rows, "members", "a", 0, 80)["members"],
            json!([])
        );
    }
}

#[cfg(all(test, feature = "rag3db-native", feature = "daemon"))]
mod integration {
    use super::*;
    use crate::daemon_host::DaemonHost;
    use crate::llm::{CallbackLlm, Llm, LlmError, MockLlm, Turn};
    use std::{
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };
    fn payload(mut v: Value) -> Value {
        for _ in 0..8 {
            if v["status"].is_string() {
                return v;
            }
            if let Some(x) = v.get("result") {
                v = x.clone();
            } else if let Some(s) = v.as_str() {
                v = serde_json::from_str(s)
                    .unwrap_or_else(|_| panic!("résultat navigation non JSON : {s}"));
            } else {
                panic!("résultat navigation absent : {v}");
            }
        }
        panic!("enveloppes excessives");
    }
    fn manifest(d: &Path, kind: &str) -> PathBuf {
        let p = d.join(format!("templates/backends/{kind}/backend.json"));
        let mut m: Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
        m["database"] = json!(d.join(format!("{kind}.rag3db")));
        m["workspace"]["root"] = json!(d.join("ws"));
        m["vector_extension"] = json!(crate::backend::vector_extension_of_this_process()
            .unwrap()
            .display()
            .to_string());
        std::fs::write(&p, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        p
    }
    #[test]
    fn actual_chat_and_contextual_relay_navigate_the_index_without_bodies() {
        let d = tempfile::tempdir().unwrap();
        crate::code_tree::copy(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"),
            &d.path().join("templates"),
            false,
        )
        .unwrap();
        std::fs::create_dir_all(d.path().join("ws")).unwrap();
        std::fs::write(
            d.path().join("ws/a.py"),
            "class A:\n    typed: int\n    def run(self):\n        return 'BODY_NEVER_DELIVERED'\n",
        )
        .unwrap();
        let code = manifest(d.path(), "code");
        let atelier = manifest(d.path(), "atelier");
        let observed = Arc::new(Mutex::new(Vec::<Value>::new()));
        let seen = observed.clone();
        let model = Arc::new(
            move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
                let seen = seen.clone();
                Ok(Arc::new(CallbackLlm::new(
                    "navigation-fixture",
                    32768,
                    move |turns: &[Turn], opts, sink| {
                        if !opts.tools.iter().any(|t| t.name == "navigate_symbols") {
                            return Err(LlmError::Model(
                                "navigate_symbols non exposé au vrai chat".into(),
                            ));
                        }
                        let tools: Vec<_> = turns
                            .iter()
                            .filter(|t| {
                                t.role == "tool"
                                    && t.tool_name.as_deref() == Some("navigate_symbols")
                            })
                            .collect();
                        let args = if let Some(t) = tools.last() {
                            if t.content.contains("BODY_NEVER_DELIVERED") {
                                return Err(LlmError::Model("corps divulgué".into()));
                            }
                            let result = payload(serde_json::from_str(&t.content).unwrap());
                            seen.lock().unwrap().push(result.clone());
                            if tools.len() >= 2 {
                                return MockLlm::new("Navigation vérifiée.")
                                    .generate(turns, opts, sink);
                            }
                            let root = result["nodes"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .find(|n| n["name"] == "A")
                                .unwrap();
                            json!({"path":"a.py","mode":"members","symbol":root["ref"]["uuid"],"depth":1,"format":"json"})
                        } else {
                            json!({"path":"a.py","depth":2,"format":"json"})
                        };
                        MockLlm::new("")
                            .with_tool_calls(vec![(
                                "navigate_symbols".to_string(),
                                args.to_string(),
                            )])
                            .generate(turns, opts, sink)
                    },
                )))
            },
        );
        let host = DaemonHost::new()
            .with_runs_root(d.path().join("runs"))
            .with_chats_root(d.path().join("chats"))
            .with_racines_de_code(crate::code_tree::Roots {
                config: d.path().join("config"),
                data: d.path().join("data"),
            })
            .with_chat_model(model);
        let session = host.open_session("fixture-navigation", None, vec!["dev".into()], None);
        let opened = host.open(Some(&session), &code);
        assert_eq!(opened["ok"], true, "{opened}");
        let id = opened["result"]["id"].as_str().unwrap();
        let started = host.call(Some(&session), &code, "index", json!({"confirm":true}));
        assert_eq!(started["ok"], true, "{started}");
        let mut result = &started;
        while let Some(v) = result.get("result") {
            result = v;
        }
        let path = result
            .as_str()
            .and_then(|s| s.lines().find_map(|l| l.strip_prefix("journal : ")))
            .expect("journal réel d'index");
        let deadline = Instant::now() + Duration::from_secs(45);
        while !std::fs::read_to_string(path)
            .unwrap_or_default()
            .contains("indexation terminée")
        {
            assert!(Instant::now() < deadline, "index fixture trop long");
            std::thread::sleep(Duration::from_millis(20));
        }
        host.chat_turn(
            id,
            "Explore la structure puis les membres de A sans lire les corps.",
        )
        .unwrap();
        let seen = observed.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0]["status"], "ready");
        assert_eq!(seen[1]["status"], "ready");
        assert!(
            seen[1]["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["name"] == "run"),
            "méthode indexée réellement accessible : {}",
            seen[1]
        );
        assert!(seen[1]["members"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["name"] == "typed"));
        drop(seen);
        let a = host.open(Some(&session), &atelier);
        assert_eq!(a["ok"], true, "{a}");
        let relay = host.call(
            Some(&session),
            &atelier,
            "navigate_symbols",
            json!({"context":{"target":{"kind":"backend","id":id}},"path":"a.py","format":"json"}),
        );
        assert_eq!(relay["ok"], true, "{relay}");
        assert_eq!(payload(relay)["status"], "ready");
        let refused=host.call(Some(&session),&atelier,"navigate_symbols",json!({"context":{"target":{"kind":"backend","id":id},"view":"unavailable"},"path":"a.py","format":"json"}));
        assert!(
            refused.to_string().contains("view") || refused.to_string().contains("vue"),
            "{refused}"
        );
        host.shutdown();
    }
}
