//! **Un Plan déclaré, joué sur le moteur** (19 octobre 2026) : le Cypher que
//! rend le dialecte rag3db pour `starts_with`, `through` (`EXISTS`), `collect`
//! (`OPTIONAL MATCH` + `collect`), l'ordre par `coalesce` décroissant,
//! `SKIP`/`LIMIT` et un compte sans condition, sur un petit schéma de la
//! mémoire. Le `PlanNode` et ses témoins de bout en bout (`e2e_plan_declare`)
//! s'appuient sur ce rendu ; ici, on prouve que le moteur l'accepte et rend
//! ce qu'il faut.

use std::collections::BTreeMap;

use rag3weaver_ir::{DeclaredPlan, Value};
use serde_json::json;

use crate::connection::{CypherValue, DbConnection};
use crate::dialect::Rag3dbDialect;
use crate::plan::{run_plan, PlanOptions};
use crate::Rag3dbConnection;

fn base(dir: &std::path::Path) -> Rag3dbConnection {
    let c = Rag3dbConnection::new(&dir.join("plan.rag3db")).unwrap();
    for q in [
        "CREATE NODE TABLE Memory(_uuid STRING, kind STRING, created_at INT64, updated_at INT64, PRIMARY KEY(_uuid))",
        "CREATE NODE TABLE Ref(_uuid STRING, genre STRING, valeur STRING, PRIMARY KEY(_uuid))",
        "CREATE NODE TABLE Subject(_uuid STRING, name STRING, PRIMARY KEY(_uuid))",
        "CREATE REL TABLE CITE(FROM Memory TO Ref)",
        "CREATE REL TABLE ANCHORED_TO(FROM Memory TO Subject)",
        // f1 : a.rs, 10 ; f2 : un symbole de a.rs, 30, ancrée « stockage » ;
        // f3 : a.rs, sans updated_at (créée à 20) ; f4 : ailleurs (srcx/) ;
        // f5 : le dossier src/, 40.
        "CREATE (:Memory {_uuid: 'f1', kind: 'fact', created_at: 1, updated_at: 10})",
        "CREATE (:Memory {_uuid: 'f2', kind: 'rule', created_at: 2, updated_at: 30})",
        "CREATE (:Memory {_uuid: 'f3', kind: 'fact', created_at: 20})",
        "CREATE (:Memory {_uuid: 'f4', kind: 'rule', created_at: 4, updated_at: 50})",
        "CREATE (:Memory {_uuid: 'f5', kind: 'fact', created_at: 5, updated_at: 40})",
        "CREATE (:Ref {_uuid: 'r1', genre: 'fichier', valeur: 'd:src/a.rs'})",
        "CREATE (:Ref {_uuid: 'r2', genre: 'symbole', valeur: 'd:src/a.rs#m::f'})",
        "CREATE (:Ref {_uuid: 'r4', genre: 'fichier', valeur: 'd:srcx/b.rs'})",
        "CREATE (:Ref {_uuid: 'r5', genre: 'dossier', valeur: 'd:src/'})",
        "CREATE (:Subject {_uuid: 's1', name: 'stockage'})",
        "MATCH (m:Memory {_uuid: 'f1'}), (r:Ref {_uuid: 'r1'}) CREATE (m)-[:CITE]->(r)",
        "MATCH (m:Memory {_uuid: 'f2'}), (r:Ref {_uuid: 'r2'}) CREATE (m)-[:CITE]->(r)",
        "MATCH (m:Memory {_uuid: 'f3'}), (r:Ref {_uuid: 'r1'}) CREATE (m)-[:CITE]->(r)",
        "MATCH (m:Memory {_uuid: 'f4'}), (r:Ref {_uuid: 'r4'}) CREATE (m)-[:CITE]->(r)",
        "MATCH (m:Memory {_uuid: 'f5'}), (r:Ref {_uuid: 'r5'}) CREATE (m)-[:CITE]->(r)",
        "MATCH (m:Memory {_uuid: 'f5'}), (r:Ref {_uuid: 'r4'}) CREATE (m)-[:CITE]->(r)",
        "MATCH (m:Memory {_uuid: 'f2'}), (s:Subject {_uuid: 's1'}) CREATE (m)-[:ANCHORED_TO]->(s)",
    ] {
        c.execute(q).unwrap_or_else(|e| panic!("{q} : {e}"));
    }
    c
}

/// Le Plan de `cited_by`, ramené à ce petit schéma : les fiches qui citent
/// une Ref valant `value`, ou commençant par `value_dir` ou `value_sym`.
fn cited_by() -> DeclaredPlan {
    DeclaredPlan::from_json(
        &json!({
            "let": {
                "ref_qui_repond": {"any_of": [
                    {"equals": {"field": "valeur", "param": "value"}},
                    {"optional": {"starts_with": {"field": "valeur", "param": "value_dir"}}},
                    {"optional": {"starts_with": {"field": "valeur", "param": "value_sym"}}}
                ]},
                "fiche_gardee": {"all_of": [
                    {"through": {"relation": "CITE", "direction": "outgoing", "end": "Ref", "filter": {"let": "ref_qui_repond"}}},
                    {"optional": {"in": {"field": "kind", "param": "kinds"}}},
                    {"optional": {"through": {"relation": "ANCHORED_TO", "direction": "outgoing", "end": "Subject",
                                              "filter": {"equals": {"field": "name", "param": "subject"}}}}}
                ]}
            },
            "forms": {
                "fiches": {"select": {
                    "table": "Memory",
                    "filter": {"let": "fiche_gardee"},
                    "returns": ["_uuid", "kind",
                        {"collect": {"as": "refs", "relation": "CITE", "direction": "outgoing", "end": "Ref",
                                     "fields": ["genre", "valeur"], "filter": {"let": "ref_qui_repond"}}},
                        {"collect": {"as": "subjects", "relation": "ANCHORED_TO", "direction": "outgoing", "end": "Subject", "fields": ["name"]}}],
                    "order_by": [{"coalesce": ["updated_at", "created_at"], "desc": true}, {"field": "_uuid"}],
                    "limit": {"param": "limit", "max": 50},
                    "offset": {"param": "offset"}
                }},
                "total": {"count": {"matching": {"table": "Memory", "filter": {"let": "fiche_gardee"}}}}
            },
            "outputs": {"rows": "fiches", "total": "total"}
        })
        .to_string(),
    )
    .unwrap()
}

fn jouer(c: &Rag3dbConnection, p: &DeclaredPlan, params: serde_json::Value) -> (Vec<Vec<CypherValue>>, i64) {
    p.check().unwrap();
    let params: BTreeMap<String, Value> = serde_json::from_value(params).unwrap();
    let lie = p.bind(&params).unwrap();
    let rapport = run_plan(&Rag3dbDialect, c, &lie.plan, PlanOptions { no_transaction: true, ..Default::default() })
        .unwrap_or_else(|e| panic!("{e:?}"));
    for n in &lie.plan.nodes {
        let texte = match &n.form {
            rag3weaver_ir::Form::Select(f) => crate::dialect::SchemaDialect::select(&Rag3dbDialect, f),
            rag3weaver_ir::Form::Count(f) => crate::dialect::SchemaDialect::count(&Rag3dbDialect, f),
            _ => unreachable!(),
        };
        eprintln!("[plan] {} : {texte:?}\n       {:?}", n.id, n.params);
    }
    let lignes = rapport.outputs["fiches"].rows.clone();
    let total = match &rapport.outputs["total"].rows[0][0] {
        CypherValue::Int(n) => *n,
        autre => panic!("{autre:?}"),
    };
    (lignes, total)
}

fn uuids(lignes: &[Vec<CypherValue>]) -> Vec<String> {
    lignes.iter().map(|l| match &l[0] { CypherValue::String(s) => s.clone(), autre => panic!("{autre:?}") }).collect()
}

#[test]
fn un_dossier_rend_ce_qui_est_dessous_par_date_avec_ses_refs_et_son_total() {
    let dir = tempfile::tempdir().unwrap();
    let c = base(dir.path());
    let p = cited_by();
    let (lignes, total) = jouer(&c, &p, json!({"value": "d:src", "value_dir": "d:src/", "limit": 10, "offset": 0}));
    // f5 (40), f2 (30), f3 (créée à 20), f1 (10) ; f4 cite srcx/, pas src/.
    assert_eq!(uuids(&lignes), ["f5", "f2", "f3", "f1"]);
    assert_eq!(total, 4);
    // Les refs de f5 : seulement celle qui répond (pas srcx/b.rs).
    let refs = &lignes[0][2];
    let CypherValue::List(refs) = refs else { panic!("{refs:?}") };
    assert_eq!(refs.len(), 1, "{refs:?}");
    // f1 n'a pas de sujet : une liste vide, pas une liste d'un NULL.
    assert_eq!(lignes[3][3], CypherValue::List(vec![]), "{:?}", lignes[3]);
    // f2 : son sujet.
    let CypherValue::List(sujets) = &lignes[1][3] else { panic!() };
    assert_eq!(sujets.len(), 1);
}

/// Le `through` (un EXISTS paramétré) et `kinds` (un autre paramètre, dehors)
/// dans un même WHERE : avant le correctif du cœur (8b98f19f7), ils ne
/// rendaient rien, le total valait 0 au lieu de 1
/// (docs/tickets/closed/2026-10-19-un-exists-parametre-avec-un-autre-parametre-dehors-ne-rend-rien.md).
#[test]
fn la_pagination_et_les_filtres_facultatifs() {
    let dir = tempfile::tempdir().unwrap();
    let c = base(dir.path());
    let p = cited_by();
    let (page, total) = jouer(&c, &p, json!({"value": "d:src", "value_dir": "d:src/", "limit": 2, "offset": 2}));
    assert_eq!(uuids(&page), ["f3", "f1"]);
    assert_eq!(total, 4, "le total ne dépend pas de la page");
    let (lignes, total) = jouer(&c, &p, json!({"value": "d:src", "value_dir": "d:src/", "kinds": ["rule"], "limit": 10, "offset": 0}));
    assert_eq!(uuids(&lignes), ["f2"]);
    assert_eq!(total, 1);
    let (lignes, _) = jouer(&c, &p, json!({"value": "d:src", "value_dir": "d:src/", "subject": "stockage", "limit": 10, "offset": 0}));
    assert_eq!(uuids(&lignes), ["f2"]);
    // Le symbole seul : value exacte, sans préfixe.
    let (lignes, _) = jouer(&c, &p, json!({"value": "d:src/a.rs#m::f", "limit": 10, "offset": 0}));
    assert_eq!(uuids(&lignes), ["f2"]);
}
