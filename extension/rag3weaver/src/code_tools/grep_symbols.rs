//! Contexte optionnel du grep : un Plan et une préparation commune par fichier.
use super::{FileSource, GrepMatch, GrepResult, ScopeRef};
use crate::read_view::ReadCatalog;
use crate::symbol_navigation::PreparedNavigation;
use serde_json::{json, Value};

pub(super) const MAX_BYTES: usize = 96 * 1024;
const MAX_FILES: usize = 8;
const MAX_NODES: usize = crate::symbol_navigation::MAX_NODES;
#[derive(Default)]
pub(super) struct Budget {
    files: usize,
    nodes: usize,
    limited: bool,
}
impl Budget {
    pub fn summary(&self, depth: usize) -> Value {
        json!({"depth":depth,"relation":"range_containment","position_precision":"line_ranges",
            "scope_depth":0,"max_files":MAX_FILES,"files_queried":self.files,
            "max_nodes":MAX_NODES,"nodes_before_output_budget":self.nodes,
            "max_bytes":MAX_BYTES,"truncated":self.limited,"coverage":"partial_indexed_scopes",
            "match_precision":"first_regex_match_per_line","not_relations":["AST_parent","inheritance","declared_membership"]})
    }
}
/// Même contenu et même curseur que la recherche, sans seconde lecture de disque.
struct Observed<'a> {
    cursor: String,
    path: &'a str,
    content: &'a str,
}
impl FileSource for Observed<'_> {
    fn cursor(&self) -> String {
        self.cursor.clone()
    }
    fn list(&self) -> Result<Vec<String>, String> {
        Ok(vec![self.path.into()])
    }
    fn read(&self, path: &str) -> Result<Option<String>, String> {
        Ok((path == self.path).then(|| self.content.to_owned()))
    }
}
fn index(
    catalog: &dyn ReadCatalog,
    path: &str,
    source: &str,
) -> Result<(Vec<Value>, usize), String> {
    use rag3weaver_ir::{Column, DeclaredPlan, Form};
    let declared: DeclaredPlan = serde_json::from_str(include_str!(
        "../../templates/backends/code/plans/symbol_navigation.json"
    ))
    .map_err(|e| e.to_string())?;
    declared.check().map_err(|e| e.to_string())?;
    let params = std::collections::BTreeMap::from([
        (
            "path".into(),
            serde_json::from_value(json!(path)).map_err(|e| e.to_string())?,
        ),
        (
            "source".into(),
            serde_json::from_value(json!(source)).map_err(|e| e.to_string())?,
        ),
    ]);
    let bound = declared.bind(&params).map_err(|e| e.to_string())?;
    let report = catalog
        .run_read_plan(
            &bound.plan,
            crate::plan::PlanOptions {
                no_transaction: true,
                ..Default::default()
            },
        )
        .map_err(|e| format!("{e:?}"))?;
    let rows_form = bound.rows.as_ref().ok_or("Plan sans rows")?;
    let Some(Form::Select(select)) = bound.plan.get(rows_form).map(|n| &n.form) else {
        return Err("rows n'est pas une sélection".into());
    };
    let names: Vec<&str> = select
        .returns
        .iter()
        .map(|c| match c {
            Column::Node(n) => Ok(n.as_str()),
            Column::Collect(c) => Ok(c.alias.as_str()),
            _ => Err("colonne non nommée"),
        })
        .collect::<Result<_, _>>()?;
    let rows = report
        .outputs
        .get(rows_form)
        .ok_or("sortie rows absente")?
        .rows
        .iter()
        .map(|r| {
            if r.len() != names.len() {
                return Err("arité des lignes du Plan incohérente".into());
            }
            let pairs = r
                .iter()
                .zip(&names)
                .map(|(v, n)| {
                    serde_json::to_value(v)
                        .map(|v| ((*n).to_owned(), v))
                        .map_err(|e| e.to_string())
                })
                .collect::<Result<serde_json::Map<_, _>, String>>()?;
            Ok(Value::Object(pairs))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let count_form = bound.total.as_ref().ok_or("Plan sans total")?;
    let n = report
        .outputs
        .get(count_form)
        .and_then(|r| r.rows.first())
        .and_then(|r| r.first());
    let Some(crate::connection::CypherValue::Int(n)) = n else {
        return Err("total non entier".into());
    };
    let total = usize::try_from(*n).map_err(|_| "total invalide")?;
    Ok((rows, total))
}

pub(super) fn enrich_file(
    source: &dyn FileSource,
    catalog: Option<&dyn ReadCatalog>,
    path: &str,
    content: &str,
    matches: &mut [GrepMatch],
    spans: &[(usize, usize)],
    depth: usize,
    budget: &mut Budget,
) {
    let (src, indexed_path) = super::indexed_name(source, path);
    let observed = Observed {
        cursor: source.cursor(),
        path: &indexed_path,
        content,
    };
    let available = budget.files < MAX_FILES && budget.nodes < MAX_NODES;
    // Aucun Plan au-delà du budget, ni pour un grep sans correspondance retenue.
    let facts = if !available {
        Ok((vec![], 0))
    } else if let Some(c) = catalog {
        budget.files += 1;
        index(c, &indexed_path, &src)
    } else {
        Ok((vec![], 0))
    };
    let preparation = match &facts {
        Ok((rows, total)) if available && catalog.is_some() => Some(PreparedNavigation::new(
            rows,
            *total,
            Some(&observed),
            &indexed_path,
            &src,
        )),
        _ => None,
    };
    for (m, &(start, end)) in matches.iter_mut().zip(spans) {
        let remaining = MAX_NODES.saturating_sub(budget.nodes);
        let mut nav = if !available || remaining == 0 {
            budget.limited = true;
            json!({"status":"context_budget_exceeded","truncated":true})
        } else if catalog.is_none() {
            json!({"status":"index_unavailable"})
        } else {
            match &preparation {
                Some(Ok(p)) => p.parents_at(start, end, depth, remaining).unwrap_or_else(
                    |e| json!({"status":"index_context_error","reason":super::clip(&e,512)}),
                ),
                Some(Err(e)) => json!({"status":"index_context_error","reason":super::clip(e,512)}),
                None => {
                    json!({"status":"index_context_error","reason":super::clip(facts.as_ref().err().map(String::as_str).unwrap_or("préparation absente"),512)})
                }
            }
        };
        budget.nodes += nav["nodes"].as_array().map(Vec::len).unwrap_or(0)
            + nav["candidates"].as_array().map(Vec::len).unwrap_or(0);
        if nav["truncated"] == true {
            budget.limited = true;
        }
        m.stale = match nav["status"].as_str() {
            Some("stale_index") => Some(true),
            Some("ready") => Some(false),
            _ => None,
        };
        if nav["status"] == "ready" {
            if let Some(r) = nav["nodes"].as_array().and_then(|a| a.first()) {
                m.scope = Some(ScopeRef {
                    name: r["name"].as_str().unwrap_or("").into(),
                    scope_type: r["kind"].as_str().unwrap_or("").into(),
                    start_line: r["start_line"].as_u64().unwrap_or(0) as usize,
                    end_line: r["end_line"].as_u64().unwrap_or(0) as usize,
                    uuid: r["ref"]["uuid"].as_str().map(String::from),
                });
            }
        }
        nav["match_ref"] = json!({"source":src,"path":indexed_path,"line":m.line,"start_byte":start,"end_byte":end,"precision":"first_regex_match_per_line"});
        m.symbol_context = Some(nav);
    }
}

pub(super) fn markdown(r: &GrepResult) -> String {
    let Some(summary) = &r.symbol_hierarchy else {
        return String::new();
    };
    let mut out=format!("\n## Contexte des symboles\n\nContenant puis au plus {} parent(s) ; plages de lignes, couverture partielle, pas AST ni héritage.\n",summary["depth"]);
    if summary["truncated"] == true {
        out.push_str("**Tronqué** : budget de fichiers/nœuds/volume ou profondeur ; les correspondances restantes restent exploitables.\n");
    }
    if summary["header_omitted"] == true {
        out.push_str("Métadonnées d'entrée omises pour respecter le budget ; aucune identité tronquée n'est présentée comme exacte.\n");
    }
    for m in &r.matches {
        if let Some(c) = &m.symbol_context {
            let p = &c["match_ref"];
            out.push_str(&format!(
                "\n### `{}` : {} — source `{}` · octets {}..{}\n",
                super::escape_md(&m.path),
                m.line,
                super::escape_md(p["source"].as_str().unwrap_or("")),
                p["start_byte"],
                p["end_byte"]
            ));
            if c.get("provenance").is_some() {
                out.push_str(&crate::symbol_navigation::markdown(c));
            } else {
                out.push_str(&format!(
                    "Contexte indisponible : {}. {}\n",
                    c["status"].as_str().unwrap_or("unknown"),
                    c["reason"].as_str().unwrap_or("")
                ));
            }
        }
    }
    out
}
/// Un plafond commun aux deux représentations, seulement pour l'option positive.
pub(super) fn bound(r: &mut GrepResult) {
    loop {
        let fits = serde_json::to_vec(&*r).is_ok_and(|v| v.len() <= MAX_BYTES)
            && r.to_markdown().len() <= MAX_BYTES;
        if fits {
            break;
        }
        let s = r.symbol_hierarchy.as_mut().expect("enrichissement demandé");
        s["truncated"] = json!(true);
        s["output_budget_exceeded"] = json!(true);
        if !r.matches.is_empty() {
            let keep = r.matches.len().saturating_sub((r.matches.len() / 8).max(1));
            r.matches.truncate(keep);
            r.returned = keep;
            continue;
        }
        if r.related.pop().is_some() {
            continue;
        }
        // Une identité d'entrée démesurée ne devient jamais une fausse référence.
        r.pattern = super::clip(&r.pattern, 256);
        r.cursor.clear();
        r.hint = None;
        r.symbol_hierarchy.as_mut().unwrap()["header_omitted"] = json!(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_tools::{grep_files, GrepOptions, Snapshot};
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Counted {
        inner: Snapshot,
        reads: AtomicUsize,
    }
    impl FileSource for Counted {
        fn cursor(&self) -> String {
            self.inner.cursor()
        }
        fn list(&self) -> Result<Vec<String>, String> {
            self.inner.list()
        }
        fn read(&self, p: &str) -> Result<Option<String>, String> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            self.inner.read(p)
        }
    }
    #[test]
    fn default_keeps_the_legacy_shape_and_positive_missing_index_keeps_utf8_matches() {
        let s = Counted {
            inner: Snapshot::new("grep", [("a.py".into(), "é λNEEDLE\r\nλNEEDLE".into())]),
            reads: AtomicUsize::new(0),
        };
        let old = grep_files(&s, None, "λNEEDLE", &GrepOptions::default()).unwrap();
        let value = serde_json::to_value(&old).unwrap();
        assert!(value.get("symbol_hierarchy").is_none());
        assert!(value["matches"][0].get("symbol_context").is_none());
        assert_eq!(old.to_markdown(),"**Pattern:** `λNEEDLE` | **Files:** 1 searched | **Matches:** 2\n\n| File | Line | Scope | Match |\n|------|------|-------|-------|\n| a.py | 1 | — | `é λNEEDLE` |\n| a.py | 2 | — | `λNEEDLE` |\n");
        s.reads.store(0, Ordering::SeqCst);
        let result = grep_files(
            &s,
            None,
            "λNEEDLE",
            &GrepOptions {
                symbol_hierarchy_depth: 2,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            s.reads.load(Ordering::SeqCst),
            1,
            "aucune seconde lecture des corps"
        );
        assert_eq!((result.total_found, result.returned), (2, 2));
        let a = result.matches[0].symbol_context.as_ref().unwrap();
        let b = result.matches[1].symbol_context.as_ref().unwrap();
        assert_eq!(a["status"], "index_unavailable");
        assert_eq!(a["match_ref"]["start_byte"], 3);
        assert_eq!(a["match_ref"]["end_byte"], 11);
        assert_eq!(b["match_ref"]["start_byte"], 13);
        assert_eq!(b["match_ref"]["end_byte"], 21);
        assert!(result.to_markdown().contains("index_unavailable"));
        assert!(grep_files(
            &s,
            None,
            "NEEDLE",
            &GrepOptions {
                symbol_hierarchy_depth: 17,
                ..Default::default()
            }
        )
        .is_err());
    }
    #[test]
    fn the_global_enriched_budget_bounds_both_formats_and_preserves_counts() {
        let path = format!("{}.py", "x".repeat(1024));
        let s = Snapshot::new(
            "budget",
            [(path, format!("needle {}\n", "λ".repeat(100)).repeat(200))],
        );
        let r = grep_files(
            &s,
            None,
            "needle",
            &GrepOptions {
                symbol_hierarchy_depth: 1,
                max_results: 200,
                context_lines: 5,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.total_found, 200);
        assert!(r.returned > 0 && r.returned < 200);
        assert_eq!(r.symbol_hierarchy.as_ref().unwrap()["truncated"], true);
        assert!(serde_json::to_vec(&r).unwrap().len() <= MAX_BYTES);
        assert!(r.to_markdown().len() <= MAX_BYTES);
        assert!(r.to_markdown().contains("Tronqué"));
    }
}
