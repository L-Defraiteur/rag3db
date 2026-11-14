//! **Un `_uuid IN [liste]` résolu en offsets, sur une base à versions** (le ticket
//! `docs/everything-declarative/tickets/2026-11-14-un-in-sur-la-cle-primaire-balaie-toute-la-table.md`
//! et sa réécriture par accès à la clé). Autonome : seulement des API de master, sans migration.
//!
//! Sur une **copie** d'une base à versions (`RAG3WEAVER_SONDE_BASE`, `RAG3WEAVER_SONDE_MANIFESTE`),
//! jamais la base en service :
//! - le masque des versions sans filtre, comme `occurrence_mask_options` : toutes les racines, puis
//!   leurs versions par `derived_references`, par lots de 5 000 ;
//! - **la mesure** : `Catalog::resolve_filter_to_ids` sur la table des versions avec
//!   `_uuid IN [toutes les versions]`, c'est-à-dire le chemin de `allowed_ids_for` (aujourd'hui 5 008 ms
//!   par balayage, la cible sous 200 ms) ;
//! - la même résolution écrite à la main par `UNWIND` et accès par clé, en témoin.
//!
//! `RAG3WEAVER_SONDE_CHAUFFE` tours de chauffe (1), puis `RAG3WEAVER_SONDE_TOURS` tours (5) : la médiane
//! et le maximum. La sonde dit « SOUS 200 ms » ou « AU-DESSUS de 200 ms » ; elle n'échoue pas.

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
fn un_in_sur_la_cle_se_resout_par_la_cle() {
    let (Some(base), Some(manifeste)) = (std::env::var_os("RAG3WEAVER_SONDE_BASE"), std::env::var_os("RAG3WEAVER_SONDE_MANIFESTE")) else {
        eprintln!("SAUTÉ un_in_sur_la_cle_se_resout_par_la_cle : RAG3WEAVER_SONDE_BASE ou RAG3WEAVER_SONDE_MANIFESTE manque (une copie, jamais la base en service)");
        return;
    };
    let (chauffe, tours) = (env_n("RAG3WEAVER_SONDE_CHAUFFE", 1), env_n("RAG3WEAVER_SONDE_TOURS", 5));
    let prepared = PreparedBackend::load(std::path::Path::new(&manifeste)).expect("le manifeste");
    let backend = prepared.open(Box::new(Rag3dbConnection::new(&base).expect("ouvrir la copie")), None).expect("ouvrir le backend");
    let cat = backend.catalog.lock().unwrap();
    let cible = cat.resolve_search_target("Scope").expect("la cible Scope");
    let projection = cible.occurrence_projection.clone().expect("une base à versions projette ses occurrences");
    let conn = cat.conn_arc();
    let dialecte = cat.dialect_arc();

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
    eprintln!("[sonde] {} racines, {} versions dans {}", racines.len(), versions.len(), cible.parent_table);

    let filtre = FilterCondition::Field { key: "_uuid".into(), value: FilterValue::List(versions.iter().cloned().map(CypherValue::String).collect()) };
    let (m, x, n) = chrono(chauffe, tours, || cat.resolve_filter_to_ids(&cible.parent_table, &filtre, &cible).unwrap().map_or(0, |o| o.len()));
    eprintln!("[sonde] resolve_filter_to_ids `_uuid IN [...]`   médiane {m:>8.1} ms   max {x:>8.1} ms   {n} offsets");
    let par_cle = format!("UNWIND $ids AS u MATCH (n:`{}` {{_uuid: u}}) RETURN OFFSET(id(n))", cible.parent_table);
    let liste = QueryParam::new("ids", CypherValue::List(versions.iter().cloned().map(CypherValue::String).collect()));
    let (mc, xc, nc) = chrono(chauffe, tours, || conn.execute_with_params(&par_cle, std::slice::from_ref(&liste)).unwrap().rows.len());
    eprintln!("[sonde] témoin UNWIND et clé, à la main           médiane {mc:>8.1} ms   max {xc:>8.1} ms   {nc} offsets");
    eprintln!("[sonde] VERDICT {} 200 ms (resolve_filter_to_ids, médiane {m:.1} ms)", if m < 200.0 { "SOUS" } else { "AU-DESSUS de" });
}
