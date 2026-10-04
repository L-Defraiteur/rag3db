//! **Sonde : la racine des receveurs sans type lu.** Les appels qualifiés
//! (`x.f()`, `x.a().f()`, `self.c.lock().unwrap().f()`) que l'analyseur
//! laisse sans `qualifier_type` sont reliés par le seul nom au rendez-vous.
//! D'où part leur qualificatif — un champ de `self`, le retour d'une
//! fonction, une variable — dit ce que le palier 3 (le type différé à texte
//! complet) rapporterait.
//!
//! Corpus : `src/` et `tests/` de rag3weaver, analysés avec les options de
//! `code::analyze_in_project` (fichier seul). Aucune base.
//!
//! Run with: cargo test --features code --test sonde_racines -- --ignored --nocapture

#![cfg(feature = "code")]

use std::collections::{BTreeMap, HashMap};

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipResolverOptions;
use codeparsers::scope_extraction::types::{DeferredType, UsageKind};

/// La racine d'un qualificatif : ce qui précède le premier `.` ou `(`.
fn racine(q: &str, differe: Option<&DeferredType>) -> &'static str {
    let q = q.trim();
    if q == "self" || q == "Self" || q == "this" {
        return "self (type englobant)";
    }
    let tete: String = q.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':').collect();
    let reste = q[tete.len()..].trim_start();
    if tete.is_empty() {
        return "autre (parenthèse, littéral…)";
    }
    if tete == "self" {
        return match differe {
            Some(DeferredType::FieldOf { .. }) => "champ de self — différé lu",
            _ => "chaîne depuis un champ de self (self.c.….f())",
        };
    }
    if reste.is_empty() && tete.contains("::") {
        return "chemin de module (m::f())";
    }
    if reste.is_empty() {
        return match differe {
            Some(DeferredType::ReturnOf { .. }) => "variable liée à un retour — différé lu",
            Some(DeferredType::FieldOf { .. }) => "variable, champ — différé lu",
            None => "variable seule, non typée",
        };
    }
    if reste.starts_with('(') {
        return if tete.contains("::") { "chaîne depuis un appel par chemin (T::f()…)" } else { "chaîne depuis un retour de fonction (f()…)" };
    }
    if reste.starts_with('.') || reste.starts_with('?') {
        return "chaîne depuis une variable non typée (v.….f())";
    }
    "autre (parenthèse, littéral…)"
}

#[test]
#[ignore]
fn racines_des_receveurs_sans_type() {
    let racine_crate = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut files = Vec::new();
    let mut contenus = HashMap::new();
    for dir in ["src", "tests"] {
        for e in walk(&format!("{racine_crate}/{dir}")) {
            if e.ends_with(".rs") {
                contenus.insert(e.clone(), std::fs::read_to_string(&e).unwrap());
                files.push(e);
            }
        }
    }
    let r = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: racine_crate.clone(),
        files,
        content_map: Some(contenus),
        resolve_relationships: Some(true),
        resolver_options: Some(RelationshipResolverOptions {
            include_file_level_refs: Some(false),
            include_child_refs: Some(false),
            resolve_cross_file: Some(false),
            ..Default::default()
        }),
    });
    let (mut types, mut sans) = (0usize, 0usize);
    let mut parts: BTreeMap<&'static str, (usize, Vec<String>)> = BTreeMap::new();
    for fa in r.files.values() {
        for s in &fa.scopes {
            for x in &s.identifier_references {
                let (Some(q), Some(UsageKind::Call)) = (x.qualifier.as_deref(), x.usage.as_ref()) else { continue };
                if x.qualifier_type.is_some() {
                    types += 1;
                    continue;
                }
                sans += 1;
                let k = racine(q, x.qualifier_deferred.as_ref());
                let e = parts.entry(k).or_default();
                e.0 += 1;
                if e.1.len() < 3 {
                    e.1.push(format!("`{}.{}` ({})", q.replace('\n', " ").chars().take(60).collect::<String>(), x.identifier, s.name));
                }
            }
        }
    }
    eprintln!("\nAppels qualifiés : {types} avec un type lu, {sans} sans.\n");
    eprintln!("| racine du receveur (appels sans type lu) | appels | part |\n|---|---|---|");
    let mut tri: Vec<_> = parts.iter().collect();
    tri.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    for (k, (n, _)) in &tri {
        eprintln!("| {k} | {n} | {:.0} % |", 100.0 * *n as f64 / sans.max(1) as f64);
    }
    for (k, (_, ex)) in &tri {
        eprintln!("\n**{k}**");
        for e in ex {
            eprintln!("- {e}");
        }
    }
}

fn walk(dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p.to_string_lossy()));
        } else {
            out.push(p.to_string_lossy().to_string());
        }
    }
    out
}
