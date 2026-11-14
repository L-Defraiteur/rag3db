//! **L'intérieur du nœud lexical sur une base à versions** (2a′-2 ; la mesure par étage du 14 novembre
//! 2026 donne `BM25SearchNode` à 6 928 ms sans filtre, contre 874 ms avant les versions). Page
//! `docs/everything-declarative/14-11-2026/04-le-pas-2a-prime-2-l-ancre-immuable.md`, § 2.
//!
//! Le nœud fait trois choses avant de chercher. La sonde les rejoue une à une, sur une **copie**
//! (`RAG3WEAVER_SONDE_BASE`, `RAG3WEAVER_SONDE_MANIFESTE`), par les mêmes fonctions :
//! 1. **les racines** : sans filtre, tous les `_uuid` de `Scope` (`select_all`, le repli
//!    d'`occurrence_roots_for`) ;
//! 2. **le masque** : `derived_references` par lots de 5 000 racines, les versions admises ;
//! 3. **la résolution du filtre `_uuid IN [versions]`** en offsets lucivy (`resolve_filter_to_ids`
//!    sur la table parente, ce que fait `allowed_ids_for`).
//! Le reste du temps du nœud est la recherche lucivy elle-même.
//!
//! Avec `RAG3WEAVER_SONDE_ROLE=reference`, la sonde dit seulement l'état des index de `Scope` (pour le
//! dense à 0 ms de la référence). `RAG3WEAVER_SONDE_CHAUFFE` (1) et `RAG3WEAVER_SONDE_TOURS` (5).

use rag3weaver::backend::PreparedBackend;
use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::filter::{FilterCondition, FilterValue};
use rag3weaver::Rag3dbConnection;
use std::time::Instant;

fn env_n(nom: &str, defaut: usize) -> usize {
    std::env::var(nom).ok().and_then(|v| v.parse().ok()).unwrap_or(defaut)
}

fn chrono<T>(chauffe: usize, tours: usize, mut f: impl FnMut() -> T) -> (f64, f64, T) {
    for _ in 0..chauffe {
        f();
    }
    let mut t = Vec::new();
    let mut dernier = None;
    for _ in 0..tours {
        let d = Instant::now();
        dernier = Some(f());
        t.push(d.elapsed().as_secs_f64() * 1000.0);
    }
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (t[t.len() / 2], t[t.len() - 1], dernier.unwrap())
}

#[test]
#[ignore]
fn l_interieur_du_noeud_lexical() {
    let (Some(base), Some(manifeste)) = (std::env::var_os("RAG3WEAVER_SONDE_BASE"), std::env::var_os("RAG3WEAVER_SONDE_MANIFESTE")) else {
        eprintln!("SAUTÉ l_interieur_du_noeud_lexical : RAG3WEAVER_SONDE_BASE ou RAG3WEAVER_SONDE_MANIFESTE manque (une copie, jamais la base en service)");
        return;
    };
    let role = std::env::var("RAG3WEAVER_SONDE_ROLE").unwrap_or_else(|_| "versions".into());
    let (chauffe, tours) = (env_n("RAG3WEAVER_SONDE_CHAUFFE", 1), env_n("RAG3WEAVER_SONDE_TOURS", 5));
    let prepared = PreparedBackend::load(std::path::Path::new(&manifeste)).expect("le manifeste");
    let backend = prepared.open(Box::new(Rag3dbConnection::new(&base).expect("ouvrir la copie")), None).expect("ouvrir le backend");
    let mut cat = backend.catalog.lock().unwrap();
    let forme = rag3weaver::code::scope_form(&cat).unwrap();
    eprintln!("[sonde] rôle {role} ; forme {forme:?}");
    for e in ["Scope", "ScopeContent"] {
        match cat.index_state_for(e) {
            Ok(s) => eprintln!("[sonde] état de {e} : {s:?}"),
            Err(err) => eprintln!("[sonde] état de {e} : {err}"),
        }
    }
    if role != "versions" {
        return;
    }
    if forme == rag3weaver::code::ScopeForm::Versions {
        rag3weaver::code::migrer_vers_les_generations(&mut cat).expect("la migration");
    }
    let cible = cat.resolve_search_target("Scope").expect("la cible Scope");
    let projection = cible.occurrence_projection.clone().expect("une base à versions projette ses occurrences");
    let conn = cat.conn_arc();
    let dialecte = cat.dialect_arc();

    // 1. Les racines.
    let tout = dialecte.select_all(&projection.entity, &["_uuid"], None);
    let (m1, x1, racines): (f64, f64, Vec<String>) = chrono(chauffe, tours, || {
        conn.execute(&tout).unwrap().rows.into_iter().filter_map(|l| l.first().and_then(|v| v.as_str()).map(str::to_string)).collect()
    });
    eprintln!("[sonde] 1 racines            médiane {m1:>8.1} ms   max {x1:>8.1} ms   {} racines", racines.len());
    // 2. Le masque.
    let masque = dialecte.derived_references(&cible.parent_table, &projection.entity, &projection.relation);
    let mut tri = racines.clone();
    tri.sort();
    let (m2, x2, versions): (f64, f64, Vec<String>) = chrono(chauffe, tours, || {
        let mut v = std::collections::HashSet::new();
        for lot in tri.chunks(5_000) {
            let p = QueryParam::new("uuids", CypherValue::List(lot.iter().cloned().map(CypherValue::String).collect()));
            for l in conn.execute_with_params(&masque, &[p]).unwrap().rows {
                if let Some(id) = l.get(1).and_then(|v| v.as_str()) {
                    v.insert(id.to_string());
                }
            }
        }
        let mut v: Vec<String> = v.into_iter().collect();
        v.sort();
        v
    });
    eprintln!("[sonde] 2 masque             médiane {m2:>8.1} ms   max {x2:>8.1} ms   {} versions", versions.len());
    // 3. La résolution du filtre `_uuid IN [versions]` en offsets, comme `allowed_ids_for`.
    let filtre = FilterCondition::Field {
        key: "_uuid".into(),
        value: FilterValue::List(versions.iter().cloned().map(CypherValue::String).collect()),
    };
    let (m3, x3, offsets) = chrono(chauffe, tours, || cat.resolve_filter_to_ids(&cible.parent_table, &filtre, &cible).unwrap());
    eprintln!(
        "[sonde] 3 filtre en offsets   médiane {m3:>8.1} ms   max {x3:>8.1} ms   {} offsets",
        offsets.map_or(0, |o| o.len())
    );
    eprintln!("[sonde] RÉSUMÉ 1+2+3 = {:.1} ms (le nœud lexical sans filtre : 6 928 ms en médiane à la mesure par étage)", m1 + m2 + m3);
}
