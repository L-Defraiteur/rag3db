//! **La note des comptes du plein texte après une maintenance nocturne** (ticket du 21 novembre 2026, « la maintenance
//! nocturne compte les non-candidates comme des écritures perdues »). La garde de fin d'index compare les documents aux
//! lignes candidates ; la maintenance réécrivait la note avec toutes les lignes de la table, et l'état disait « des
//! écritures ont été perdues » pour chaque conteneur que la règle des non-candidats écarte.

use super::tests::{backend_de_code, fixture_code_avec};
use super::*;
use serde_json::json;
use std::path::Path;

#[test]
fn the_nightly_maintenance_does_not_count_the_non_candidates_as_lost_writes() {
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/tools");
    let (dir, p) = fixture_code_avec("working_tree", false, json!({}), &|m| {
        m["tools"]["search_code"] =
            json!({"graph": "search_workspace.mmd", "bindings": {"target": "Scope"}, "policy": {"read_files": true}});
    });
    drop(p);
    std::fs::copy(tools.join("search_workspace.mmd"), dir.path().join("search_workspace.mmd")).unwrap();
    for i in 0..12 {
        std::fs::write(dir.path().join(format!("sources/h{i}.h")), format!("#pragma once\n\nnamespace zorg {{\nclass Thing{i} {{\n  int a{i};\n}};\n}}\n")).unwrap();
    }
    let prepared = PreparedBackend::load(&dir.path().join("backend.json")).expect("le manifeste se charge");
    let backend = backend_de_code(dir.path(), prepared);
    let source = backend.file_source.clone().unwrap();
    // Under the declared rule: the index sets the folded containers aside, and its guard sees no gap.
    let ecartees = {
        let mut cat = backend.catalog.lock().unwrap();
        crate::code_sync::sync_source(&mut cat, source.as_ref(), &Default::default(), &mut |_| {}).expect("the index under the rule").non_candidates_set_aside
    };
    let ecart = |backend: &Backend| -> Vec<String> {
        let etats = backend.index_states().expect("index_states").to_string();
        let mut v: Vec<String> = Vec::new();
        if etats.contains("des écritures ont été perdues") {
            v.push(etats.split("des écritures ont été perdues").next().unwrap_or("").rsplit("plein texte de").next().unwrap_or("").to_string());
        }
        v
    };
    assert!(ecartees >= 10, "the rule sets the folded containers aside: {ecartees}");
    let comptes = backend.catalog.lock().unwrap().comptes_du_plein_texte();
    assert!(comptes.iter().all(|(_, d, l)| d == l), "the end-of-index guard sees no gap: {comptes:?}");
    assert!(ecart(&backend).is_empty(), "no gap said after the index");
    // The nightly maintenance rewrites the noted counts: the containers set aside are no lost write.
    let maintenant = jiff::civil::date(2026, 11, 21).at(3, 30, 0, 0);
    let note = backend.catalog.lock().unwrap().fts_maintenance_nocturne(maintenant, 0.5, false).expect("a full-text maintenance");
    assert!(note.to_string().contains("ScopeContent"), "the maintenance read ScopeContent: {note}");
    let gaps = ecart(&backend);
    assert!(gaps.is_empty(), "the state says lost writes after the maintenance, with {ecartees} containers set aside by the rule: {gaps:?}");
}
