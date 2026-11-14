//! **La recherche vectorielle filtrée, seule, et la mémoire qu'elle garde** (2a′-2). La sonde par étage
//! du 14 novembre 2026 voit le dense seul, sur une base à versions, retenir environ 3,7 Gio par
//! `search_code`. Ticket `docs/everything-declarative/tickets/2026-11-14-une-recherche-a-versions-prend-des-gio-et-des-dizaines-de-secondes.md`.
//!
//! Le dense filtré passe par `Rag3dbSearchBackend::vector_search_filtered` :
//! 1. le filtre compilé (`compile_filter_for_vector_in`), écrit en clair dans un Cypher ;
//! 2. `PROJECT_GRAPH_CYPHER` sur ce Cypher ;
//! 3. `QUERY_VECTOR_INDEX` sur le graphe projeté ;
//! 4. `DROP_PROJECTED_GRAPH`.
//!
//! Sur une **copie** à versions (`RAG3WEAVER_SONDE_BASE`, `RAG3WEAVER_SONDE_MANIFESTE`), hors de
//! `search_code`, `RAG3WEAVER_SONDE_REPETITIONS` fois chacune (6 par défaut), le RSS après chaque tour :
//! - **V1** : le cycle entier, par `vector_search_filtered`, avec le masque des versions comme filtre ;
//! - **V2** : la projection puis le retrait, sans la requête vectorielle ;
//! - **V3** : `vector_search` sans filtre, en témoin.
//! `RAG3WEAVER_SONDE_VARIANTES=V2,V3,V1` choisit et ordonne les variantes (par défaut V3, V2, V1 : le
//! témoin d'abord, la plus gourmande en dernier).

use rag3weaver::backend::PreparedBackend;
use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::filter::{FilterCondition, FilterValue};
use rag3weaver::search_backend::SearchBackend;
use rag3weaver::Rag3dbConnection;
use std::time::Instant;

fn rss_mio() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("VmRSS:")).and_then(|l| l.split_whitespace().nth(1)).and_then(|k| k.parse::<f64>().ok()))
        .map_or(f64::NAN, |k| k / 1024.0)
}

/// La substitution d'un paramètre, pour la variante V2 : une liste de chaînes, ou une chaîne.
fn en_clair(v: &CypherValue) -> String {
    match v {
        CypherValue::String(s) => format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'")),
        CypherValue::List(l) => format!("[{}]", l.iter().map(en_clair).collect::<Vec<_>>().join(", ")),
        CypherValue::Int(i) => i.to_string(),
        autre => panic!("paramètre non prévu par la sonde : {autre:?}"),
    }
}

#[test]
#[ignore]
fn la_recherche_vectorielle_filtree_seule() {
    let (Some(base), Some(manifeste)) = (std::env::var_os("RAG3WEAVER_SONDE_BASE"), std::env::var_os("RAG3WEAVER_SONDE_MANIFESTE")) else {
        eprintln!("SAUTÉ la_recherche_vectorielle_filtree_seule : RAG3WEAVER_SONDE_BASE ou RAG3WEAVER_SONDE_MANIFESTE manque (une copie, jamais la base en service)");
        return;
    };
    let n: usize = std::env::var("RAG3WEAVER_SONDE_REPETITIONS").ok().and_then(|v| v.parse().ok()).unwrap_or(6);
    let variantes: Vec<String> = std::env::var("RAG3WEAVER_SONDE_VARIANTES").unwrap_or_else(|_| "V3,V2,V1".into()).split(',').map(|s| s.trim().to_string()).collect();
    let prepared = PreparedBackend::load(std::path::Path::new(&manifeste)).expect("le manifeste");
    let (embarqueur, _) = prepared.connect_embedder().expect("l'embarqueur du manifeste");
    let requete = embarqueur.embed(&["rebuild the vector index".to_string()]).expect("embarquer la requête").remove(0);
    let backend = prepared.open(Box::new(Rag3dbConnection::new(&base).expect("ouvrir la copie")), Some(embarqueur)).expect("ouvrir le backend");
    let mut cat = backend.catalog.lock().unwrap();
    if rag3weaver::code::scope_form(&cat).unwrap() == rag3weaver::code::ScopeForm::Versions {
        rag3weaver::code::migrer_vers_les_generations(&mut cat).expect("la migration");
    }
    let cible = cat.resolve_search_target("Scope").expect("la cible Scope");
    let projection = cible.occurrence_projection.clone().expect("une base à versions projette ses occurrences");
    let conn = cat.conn_arc();
    let dialecte = cat.dialect_arc();
    let stockage = cat.vector_storage(&cible.chunk_table).expect("le stockage du modèle courant");
    eprintln!("[sonde] morceaux {} ; index {} ; parent {} ; RSS {:.0} Mio", cible.chunk_table, stockage.index, cible.parent_table, rss_mio());

    // Le masque des versions, comme `occurrence_mask_options` sans filtre.
    let racines: Vec<String> = conn.execute(&dialecte.select_all(&projection.entity, &["_uuid"], None)).unwrap().rows.into_iter().filter_map(|l| l.first().and_then(|v| v.as_str()).map(str::to_string)).collect();
    let masque = dialecte.derived_references(&cible.parent_table, &projection.entity, &projection.relation);
    let mut versions = std::collections::BTreeSet::new();
    for lot in racines.chunks(5_000) {
        let p = QueryParam::new("uuids", CypherValue::List(lot.iter().cloned().map(CypherValue::String).collect()));
        for l in conn.execute_with_params(&masque, &[p]).unwrap().rows {
            if let Some(id) = l.get(1).and_then(|v| v.as_str()) {
                versions.insert(id.to_string());
            }
        }
    }
    let condition = FilterCondition::Field { key: "_uuid".into(), value: FilterValue::List(versions.iter().cloned().map(CypherValue::String).collect()) };
    let (filtre_where, filtre_params, filtre_match) = cat.compile_filter_for_vector_in(&cat.scope().clone(), &cible.parent_table, Some(&condition)).expect("le filtre compilé");
    eprintln!("[sonde] {} versions ; filtre : match {:?}, where {:?}, {} paramètre(s) ; RSS {:.0} Mio", versions.len(), filtre_match, filtre_where, filtre_params.len(), rss_mio());
    let moteur = rag3weaver::rag3db_search_backend::Rag3dbSearchBackend::new(conn.clone());

    for v in &variantes {
        let depart = rss_mio();
        eprintln!("[sonde] == {v}, RSS au départ {depart:.0} Mio");
        for i in 1..=n {
            let d = Instant::now();
            let rendu = match v.as_str() {
                "V1" => moteur
                    .vector_search_filtered(&cible.chunk_table, &stockage.index, &stockage.column, &requete, 10, filtre_match.as_deref(), filtre_where.as_deref(), &filtre_params)
                    .map(|h| h.len()),
                "V2" => {
                    let mut cypher = format!("MATCH (n:`{}`)", cible.chunk_table);
                    if let Some(m) = &filtre_match {
                        cypher.push(' ');
                        cypher.push_str(m);
                    }
                    if let Some(w) = &filtre_where {
                        cypher.push_str(" WHERE ");
                        cypher.push_str(w);
                    }
                    cypher.push_str(" RETURN n");
                    for p in &filtre_params {
                        cypher = cypher.replace(&format!("${}", p.name), &en_clair(&p.value));
                    }
                    if i == 1 {
                        eprintln!("[sonde] V2 : Cypher projeté de {} octets", cypher.len());
                    }
                    let echappe = cypher.replace('\\', "\\\\").replace('\'', "\\'");
                    let _ = conn.execute("CALL DROP_PROJECTED_GRAPH('_vf_sonde', skip_if_not_exists := true)");
                    let r = conn.execute(&format!("CALL PROJECT_GRAPH_CYPHER('_vf_sonde', '{echappe}')")).map(|_| 0).map_err(|e| e.to_string());
                    let _ = conn.execute("CALL DROP_PROJECTED_GRAPH('_vf_sonde', skip_if_not_exists := true)");
                    r
                }
                "V3" => moteur.vector_search(&cible.chunk_table, &stockage.index, &stockage.column, &requete, 10).map(|h| h.len()),
                autre => panic!("variante inconnue : {autre}"),
            };
            eprintln!(
                "[sonde] {v} tour {i:>2} : {:>8.1} ms, {:?}, RSS {:.0} Mio ({:+.0} depuis le départ de la variante)",
                d.elapsed().as_secs_f64() * 1000.0,
                rendu,
                rss_mio(),
                rss_mio() - depart
            );
        }
    }
}
