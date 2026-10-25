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

/// Des fiches et leurs apports (25 octobre 2026, l'ordre par extrême) : g1,
/// ancienne (10), reçoit un L1 neuf (100) ; g2, à 50, n'a aucun apport ; g3, à
/// 30, a un L1 ancien (20) et un L2 neuf (200).
fn base_des_apports(dir: &std::path::Path) -> Rag3dbConnection {
    let c = Rag3dbConnection::new(&dir.join("apports.rag3db")).unwrap();
    for q in [
        "CREATE NODE TABLE Memory(_uuid STRING, kind STRING, created_at INT64, updated_at INT64, PRIMARY KEY(_uuid))",
        "CREATE NODE TABLE Apport(_uuid STRING, niveau STRING, created_at INT64, PRIMARY KEY(_uuid))",
        "CREATE REL TABLE A_POUR_APPORT(FROM Memory TO Apport)",
        "CREATE (:Memory {_uuid: 'g1', kind: 'rule', created_at: 1, updated_at: 10})",
        "CREATE (:Memory {_uuid: 'g2', kind: 'rule', created_at: 2, updated_at: 50})",
        "CREATE (:Memory {_uuid: 'g3', kind: 'rule', created_at: 3, updated_at: 30})",
        "CREATE (:Apport {_uuid: 'p1', niveau: 'L1', created_at: 100})",
        "CREATE (:Apport {_uuid: 'p3a', niveau: 'L1', created_at: 20})",
        "CREATE (:Apport {_uuid: 'p3b', niveau: 'L2', created_at: 200})",
        "MATCH (m:Memory {_uuid: 'g1'}), (a:Apport {_uuid: 'p1'}) CREATE (m)-[:A_POUR_APPORT]->(a)",
        "MATCH (m:Memory {_uuid: 'g3'}), (a:Apport {_uuid: 'p3a'}) CREATE (m)-[:A_POUR_APPORT]->(a)",
        "MATCH (m:Memory {_uuid: 'g3'}), (a:Apport {_uuid: 'p3b'}) CREATE (m)-[:A_POUR_APPORT]->(a)",
    ] {
        c.execute(q).unwrap_or_else(|e| panic!("{q} : {e}"));
    }
    c
}

/// Les fiches, ordonnées par `cle` puis `_uuid`, paginées, avec la liste de
/// leurs apports (l'extrême et la liste dans la même requête) et leur total.
fn par_extreme(cle: serde_json::Value) -> DeclaredPlan {
    DeclaredPlan::from_json(
        &json!({
            "forms": {
                "fiches": {"select": {
                    "table": "Memory",
                    "filter": {"equals": {"field": "kind", "param": "k"}},
                    "returns": ["_uuid",
                        {"collect": {"as": "apports", "relation": "A_POUR_APPORT", "direction": "outgoing", "end": "Apport", "fields": ["niveau"]}}],
                    "order_by": [cle, {"field": "_uuid"}],
                    "limit": {"param": "limit", "max": 10},
                    "offset": {"param": "offset"}
                }},
                "total": {"count": {"matching": {"table": "Memory", "filter": {"equals": {"field": "kind", "param": "k"}}}}}
            },
            "outputs": {"rows": "fiches", "total": "total"}
        })
        .to_string(),
    )
    .unwrap()
}

fn apport_le_plus_recent(filtre: bool) -> serde_json::Value {
    let mut max = json!({"relation": "A_POUR_APPORT", "direction": "outgoing", "end": "Apport", "field": "created_at"});
    if filtre {
        max["filter"] = json!({"optional": {"equals": {"field": "niveau", "param": "niveau"}}});
    }
    max
}

/// **« Les fiches triées par la date de leur dernier L1 »** (la forme de la
/// mémoire) : `coalesce(max(L1.created_at), updated_at, created_at)` décroissant.
/// Une fiche ancienne qui reçoit un L1 neuf passe devant ; une fiche sans L1
/// retombe sur sa date ; le filtre de l'extrême écarte le L2, et absent (sous
/// `optional`), il les compte tous.
#[test]
fn le_dernier_apport_ordonne_les_fiches_et_une_fiche_sans_apport_retombe_sur_sa_date() {
    let dir = tempfile::tempdir().unwrap();
    let c = base_des_apports(dir.path());
    let p = par_extreme(json!({"coalesce": [{"max": apport_le_plus_recent(true)}, "updated_at", "created_at"], "desc": true}));
    // L1 seulement : g1 (100), g2 (50, sa date), g3 (20, son L1).
    let (lignes, total) = jouer(&c, &p, json!({"k": "rule", "niveau": "L1", "limit": 10, "offset": 0}));
    assert_eq!(uuids(&lignes), ["g1", "g2", "g3"]);
    assert_eq!(total, 3);
    // La liste et l'extrême dans la même requête : g3 garde ses deux apports.
    let CypherValue::List(apports) = &lignes[2][1] else { panic!("{:?}", lignes[2]) };
    assert_eq!(apports.len(), 2, "{apports:?}");
    let CypherValue::List(aucun) = &lignes[1][1] else { panic!("{:?}", lignes[1]) };
    assert!(aucun.is_empty(), "g2 n'a aucun apport : {aucun:?}");
    // Sans niveau (l'optional retiré) : le L2 de g3 (200) compte, g3 passe devant.
    let (lignes, _) = jouer(&c, &p, json!({"k": "rule", "limit": 10, "offset": 0}));
    assert_eq!(uuids(&lignes), ["g3", "g1", "g2"]);
    // Paginé : la seconde page suit la première, sans saut ni répétition.
    let (page, _) = jouer(&c, &p, json!({"k": "rule", "limit": 2, "offset": 1}));
    assert_eq!(uuids(&page), ["g1", "g2"]);
}

/// **Un extrême seul : son NULL se range après les autres, dans les deux
/// sens** — g2, sans apport, est dernier en `max` décroissant comme en `min`
/// croissant.
#[test]
fn un_extreme_seul_range_la_fiche_sans_apport_en_dernier_dans_les_deux_sens() {
    let dir = tempfile::tempdir().unwrap();
    let c = base_des_apports(dir.path());
    let max = par_extreme(json!({"max": apport_le_plus_recent(false), "desc": true}));
    let (lignes, _) = jouer(&c, &max, json!({"k": "rule", "limit": 10, "offset": 0}));
    assert_eq!(uuids(&lignes), ["g3", "g1", "g2"], "max décroissant : 200, 100, puis le NULL");
    let min = par_extreme(json!({"min": apport_le_plus_recent(false)}));
    let (lignes, _) = jouer(&c, &min, json!({"k": "rule", "limit": 10, "offset": 0}));
    assert_eq!(uuids(&lignes), ["g3", "g1", "g2"], "min croissant : 20, 100, puis le NULL");
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

/// Rouge connu tant que la lib `courant` n'a pas le correctif du cœur : le
/// `through` (un EXISTS paramétré) et `kinds` (un autre paramètre, dehors)
/// dans un même WHERE ne rendent rien (le total vaut 0 au lieu de 1).
#[test]
#[ignore = "rouge connu : docs/tickets/closed/2026-10-19-un-exists-parametre-avec-un-autre-parametre-dehors-ne-rend-rien.md — vert avec la lib du correctif du cœur (8b98f19f7)"]
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

/// L'isolement du compte faux (exploration, 19 octobre 2026) : les variantes
/// qui ont départagé le défaut des EXISTS paramétrés, chacune nue sur la
/// connexion ; elles impriment ce qu'elles rendent. Corrigé par 8b98f19f7 ; la
/// recette reste, pour la rejouer sur une autre lib.
#[test]
#[ignore]
fn isoler_le_compte_sous_une_liste_en_parametre() {
    use crate::connection::QueryParam;
    let dir = tempfile::tempdir().unwrap();
    let c = base(dir.path());
    let kinds = || vec![QueryParam::new("kinds", CypherValue::List(vec![CypherValue::String("rule".into())]))];
    let v = || vec![QueryParam::new("value", CypherValue::String("d:src".into())), QueryParam::new("value_dir", CypherValue::String("d:src/".into()))];
    let ex = "EXISTS { MATCH (m)-[:`CITE`]->(t1:`Ref`) WHERE t1.`valeur` = $value OR t1.`valeur` STARTS WITH $value_dir }";
    let cas: Vec<(&str, String, Vec<QueryParam>)> = vec![
        ("in seul, lignes", "MATCH (m:`Memory`) WHERE m.`kind` IN $kinds RETURN m.`_uuid`".into(), kinds()),
        ("in seul, compte", "MATCH (m:`Memory`) WHERE m.`kind` IN $kinds RETURN count(m)".into(), kinds()),
        ("in littéral, compte", "MATCH (m:`Memory`) WHERE m.`kind` IN ['rule'] RETURN count(m)".into(), vec![]),
        ("exists seul, compte", format!("MATCH (m:`Memory`) WHERE ({ex}) RETURN count(m)"), v()),
        ("exists et in, lignes", format!("MATCH (m:`Memory`) WHERE ({ex}) AND (m.`kind` IN $kinds) RETURN m.`_uuid`"), [v(), kinds()].concat()),
        ("exists et in, compte", format!("MATCH (m:`Memory`) WHERE ({ex}) AND (m.`kind` IN $kinds) RETURN count(m)"), [v(), kinds()].concat()),
        ("exists et in littéral, compte", format!("MATCH (m:`Memory`) WHERE ({ex}) AND (m.`kind` IN ['rule']) RETURN count(m)"), v()),
        ("in puis exists, compte", format!("MATCH (m:`Memory`) WHERE (m.`kind` IN $kinds) AND ({ex}) RETURN count(m)"), [v(), kinds()].concat()),
        ("exists et égal, compte", format!("MATCH (m:`Memory`) WHERE ({ex}) AND (m.`kind` = 'rule') RETURN count(m)"), v()),
        ("exists et in, WITH puis compte", format!("MATCH (m:`Memory`) WHERE ({ex}) AND (m.`kind` IN $kinds) WITH m RETURN count(m)"), [v(), kinds()].concat()),
        ("exists sans paramètre et in, lignes", "MATCH (m:`Memory`) WHERE (EXISTS { MATCH (m)-[:`CITE`]->(t1:`Ref`) WHERE t1.`valeur` STARTS WITH 'd:src/' }) AND (m.`kind` IN $kinds) RETURN m.`_uuid`".into(), kinds()),
        ("exists et list_contains, lignes", format!("MATCH (m:`Memory`) WHERE ({ex}) AND list_contains($kinds, m.`kind`) RETURN m.`_uuid`"), [v(), kinds()].concat()),
        ("in et autre paramètre sans exists, lignes", "MATCH (m:`Memory`) WHERE m.`kind` IN $kinds AND m.`created_at` >= $x RETURN m.`_uuid`".into(), [kinds(), vec![QueryParam::new("x", CypherValue::Int(0))]].concat()),
        ("exists et égal en paramètre, lignes", format!("MATCH (m:`Memory`) WHERE ({ex}) AND m.`kind` = $k RETURN m.`_uuid`"), [v(), vec![QueryParam::new("k", CypherValue::String("rule".into()))]].concat()),
        ("exists sans paramètre et égal en paramètre, lignes", "MATCH (m:`Memory`) WHERE (EXISTS { MATCH (m)-[:`CITE`]->(t1:`Ref`) WHERE t1.`valeur` STARTS WITH 'd:src/' }) AND m.`kind` = $k RETURN m.`_uuid`".into(), vec![QueryParam::new("k", CypherValue::String("rule".into()))]),
        ("deux MATCH à la façon du FilterParser, lignes", "MATCH (m:`Memory`) MATCH (m)-[:`CITE`]->(e1:`Ref`) WHERE e1.`valeur` STARTS WITH $value_dir AND m.`kind` = $k RETURN DISTINCT m.`_uuid`".into(), vec![QueryParam::new("value_dir", CypherValue::String("d:src/".into())), QueryParam::new("k", CypherValue::String("rule".into()))]),
        ("WITH puis exists, lignes", "MATCH (m:`Memory`) WHERE m.`kind` = $k WITH m WHERE EXISTS { MATCH (m)-[:`CITE`]->(t1:`Ref`) WHERE t1.`valeur` STARTS WITH $value_dir } RETURN m.`_uuid`".into(), vec![QueryParam::new("value_dir", CypherValue::String("d:src/".into())), QueryParam::new("k", CypherValue::String("rule".into()))]),
        ("exists puis WITH, lignes", "MATCH (m:`Memory`) WHERE EXISTS { MATCH (m)-[:`CITE`]->(t1:`Ref`) WHERE t1.`valeur` STARTS WITH $value_dir } WITH m WHERE m.`kind` = $k RETURN m.`_uuid`".into(), vec![QueryParam::new("value_dir", CypherValue::String("d:src/".into())), QueryParam::new("k", CypherValue::String("rule".into()))]),
        ("WITH puis exists, compte", "MATCH (m:`Memory`) WHERE m.`kind` = $k WITH m WHERE EXISTS { MATCH (m)-[:`CITE`]->(t1:`Ref`) WHERE t1.`valeur` STARTS WITH $value_dir } RETURN count(m)".into(), vec![QueryParam::new("value_dir", CypherValue::String("d:src/".into())), QueryParam::new("k", CypherValue::String("rule".into()))]),
        ("deux exists paramétrés, un WHERE, lignes", "MATCH (m:`Memory`) WHERE (EXISTS { MATCH (m)-[:`CITE`]->(t1:`Ref`) WHERE t1.`valeur` STARTS WITH $value_dir }) AND (EXISTS { MATCH (m)-[:`ANCHORED_TO`]->(t2:`Subject`) WHERE t2.`name` = $subject }) RETURN m.`_uuid`".into(), vec![QueryParam::new("value_dir", CypherValue::String("d:src/".into())), QueryParam::new("subject", CypherValue::String("stockage".into()))]),
        ("deux exists paramétrés, séparés par WITH, lignes", "MATCH (m:`Memory`) WHERE EXISTS { MATCH (m)-[:`CITE`]->(t1:`Ref`) WHERE t1.`valeur` STARTS WITH $value_dir } WITH m WHERE EXISTS { MATCH (m)-[:`ANCHORED_TO`]->(t2:`Subject`) WHERE t2.`name` = $subject } RETURN m.`_uuid`".into(), vec![QueryParam::new("value_dir", CypherValue::String("d:src/".into())), QueryParam::new("subject", CypherValue::String("stockage".into()))]),
        ("deux exists paramétrés, séparés par WITH, compte", "MATCH (m:`Memory`) WHERE EXISTS { MATCH (m)-[:`CITE`]->(t1:`Ref`) WHERE t1.`valeur` STARTS WITH $value_dir } WITH m WHERE EXISTS { MATCH (m)-[:`ANCHORED_TO`]->(t2:`Subject`) WHERE t2.`name` = $subject } RETURN count(m)".into(), vec![QueryParam::new("value_dir", CypherValue::String("d:src/".into())), QueryParam::new("subject", CypherValue::String("stockage".into()))]),
        ("exists paramétré puis OPTIONAL MATCH paramétré, lignes", "MATCH (m:`Memory`) WHERE EXISTS { MATCH (m)-[:`CITE`]->(t1:`Ref`) WHERE t1.`valeur` STARTS WITH $value_dir } OPTIONAL MATCH (m)-[:`CITE`]->(c0:`Ref`) WHERE c0.`valeur` STARTS WITH $value_dir WITH m, coalesce(collect(c0.`valeur`), []) AS v RETURN m.`_uuid`, v".into(), vec![QueryParam::new("value_dir", CypherValue::String("d:src/".into()))]),
        ("WITH puis exists, puis OPTIONAL MATCH avec un autre paramètre, lignes", "MATCH (m:`Memory`) WHERE m.`kind` = $k WITH m WHERE EXISTS { MATCH (m)-[:`CITE`]->(t1:`Ref`) WHERE t1.`valeur` STARTS WITH $value_dir } OPTIONAL MATCH (m)-[:`ANCHORED_TO`]->(c1:`Subject`) WHERE c1.`name` = $subject WITH m, coalesce(collect(c1.`name`), []) AS n RETURN m.`_uuid`, n".into(), vec![QueryParam::new("value_dir", CypherValue::String("d:src/".into())), QueryParam::new("k", CypherValue::String("rule".into())), QueryParam::new("subject", CypherValue::String("stockage".into()))]),
        ("exists un paramètre et in, lignes", "MATCH (m:`Memory`) WHERE (EXISTS { MATCH (m)-[:`CITE`]->(t1:`Ref`) WHERE t1.`valeur` STARTS WITH $value_dir }) AND (m.`kind` IN $kinds) RETURN m.`_uuid`".into(), [vec![QueryParam::new("value_dir", CypherValue::String("d:src/".into()))], kinds()].concat()),
    ];
    for (nom, q, p) in cas {
        let r = c.execute_with_params(&q, &p);
        eprintln!("[isoler] {nom} : {:?}", r.map(|r| r.rows));
    }
}
