//! E2E : **un Plan IR déclaré, exécuté par `PlanNode`** — la spécification
//! exécutable de la forme fixée avec l'arbre principal
//! (`docs/18-octobre-2026-memoire-longue/03-la-forme-json-d-un-plan.md`), et de
//! l'outil `cited_by` qu'elle rend possible (`…/01-une-fiche-qui-vise-un-symbole.md`).
//!
//! **Rouges sur l'IR du 18 octobre 2026** : `PlanNode` n'existe pas, les formes
//! n'ont pas de serde. Écrits d'avance, à la demande de l'arbre principal, pour
//! servir de spécification ; ils passent au vert un par un avec le lot de l'IR
//! (StartsWith, Through, Order, offset, Count sur le même prédicat, Collect,
//! Optional, serde + validate) et le `PlanNode`. Ils ne touchent aucun type de
//! l'IR à la compilation : tout passe par un manifeste et des appels d'outil, si
//! bien que le binaire e2e compile aujourd'hui.
//!
//! Run with: ./run_e2e.sh --test e2e_plan_declare

#![cfg(feature = "rag3db-native")]

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::Rag3dbConnection;

/// `Memory`, `Ref`, `Subject` recopiées du gabarit `memory`, sans signaux, avec
/// `CITE` et `ANCHORED_TO`, et un outil `plan_essai` qui exécute
/// `plans/essai.json` avec les paramètres qu'on lui donne.
fn manifeste(dir: &Path, plan: &Value) -> PathBuf {
    let memoire = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/memory");
    let gabarit: Value = serde_json::from_slice(&std::fs::read(memoire.join("backend.json")).unwrap()).unwrap();
    let recopier = |nom: &str| {
        let mut e = gabarit["entities"][nom].clone();
        let schema = e["schema"].as_str().unwrap().to_string();
        e["schema"] = json!(memoire.join(schema).display().to_string());
        e["config"]["signals"] = json!([]);
        e
    };
    std::fs::create_dir_all(dir.join("plans")).unwrap();
    std::fs::write(dir.join("plans/essai.json"), serde_json::to_vec_pretty(plan).unwrap()).unwrap();
    std::fs::write(
        dir.join("plan_essai.mmd"),
        "%% tool: plan_essai\n\
         %% description: Exécute plans/essai.json (témoin du PlanNode).\n\
         %% param: params json = {} -- les paramètres du Plan\n\
         %% result: plan.rows\n\
         graph LR\n    plan[\"PlanNode(plan=@plans/essai.json, params=$params)\"]\n",
    )
    .unwrap();
    let m = json!({
        "version": 1, "name": "plan-declare", "database": "base.rag3db",
        "embeddings": {"address": "127.0.0.1:9", "model": "none", "dimensions": 8},
        "vector_extension": crate::common::vector_extension(),
        "entities": {"Memory": recopier("Memory"), "Ref": recopier("Ref"), "Subject": recopier("Subject")},
        "relations": {
            "CITE": {"from": "Memory", "to": "Ref"},
            "ANCHORED_TO": {"from": "Memory", "to": "Subject"}
        },
        "tools": {
            "plan_essai": {"graph": "plan_essai.mmd", "metadata": [{"node": "plan", "port": "total"}]}
        }
    });
    let chemin = dir.join("backend.json");
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    chemin
}

fn ouvrir(chemin: &Path) -> Result<Backend, String> {
    let prepared = PreparedBackend::load(chemin)?;
    let base = prepared.path(&prepared.manifest.database);
    prepared.open(Box::new(Rag3dbConnection::new(&base).expect("la base")), None)
}

/// Le Plan refusé quelque part avant les lignes : au chargement, à l'ouverture
/// ou à l'appel — la forme le permet aux trois (validate() au chargement, le
/// schéma au liage). Rend le texte du refus.
fn refus(plan: Value, params: Value) -> String {
    let dir = tempfile::tempdir().expect("dossier");
    match ouvrir(&manifeste(dir.path(), &plan)) {
        Err(e) => e,
        Ok(b) => match b.call("plan_essai", json!({"params": params})) {
            Err(e) => e,
            Ok(r) => panic!("le Plan devait être refusé, il a rendu : {r}"),
        },
    }
}

/// Cinq fiches, des Refs de chemins et de symboles, deux sujets.
/// `updated_at` croît avec le rang : la plus récente est `f5`.
fn semer(b: &Backend) {
    let cat = b.catalog.lock().unwrap();
    let x = |q: &str| {
        cat.execute_raw_with_params(q, &[]).unwrap_or_else(|e| panic!("{q} : {e}"));
    };
    for (i, kind) in [(1, "fact"), (2, "rule"), (3, "fact"), (4, "rule"), (5, "fact")] {
        // f3 n'a pas d'`updated_at` : `coalesce(updated_at, created_at)` le range par
        // son `created_at` (1003), donc après les autres.
        let maj = if i == 3 { String::new() } else { format!(", updated_at: {}", 2000 + i) };
        x(&format!(
            "CREATE (:Memory {{_uuid: 'f{i}', claim: 'fiche {i}', why: 'parce que', kind: '{kind}', origin: 'said', \
             reach: 'person', state: 'current', created_at: {c}{maj}}})",
            c = 1000 + i
        ));
    }
    let refs = [
        ("r1", "path", "git:github.com/x/y:src/a.rs"),
        ("r2", "symbol", "git:github.com/x/y:src/a.rs#Catalog::open"),
        ("r3", "path", "git:github.com/x/y:src/b/c.rs"),
        ("r4", "path", "git:github.com/x/y:docs/d.md"),
        ("r5", "path", "git:github.com/x/y:srcx/e.rs"),
    ];
    for (u, genre, valeur) in refs {
        x(&format!("CREATE (:Ref {{_uuid: '{u}', genre: '{genre}', valeur: '{valeur}', dit_comme: '', empreinte: '', etat: 'vue'}})"));
    }
    for (s, nom) in [("s1", "stockage"), ("s2", "documentation")] {
        x(&format!("CREATE (:Subject {{_uuid: '{s}', name: '{nom}', about: '', state: 'current'}})"));
    }
    // f1 → a.rs ; f2 → a.rs#Catalog::open ; f3 → b/c.rs ; f4 → d.md ; f5 → a.rs et srcx/e.rs
    for (f, r) in [("f1", "r1"), ("f2", "r2"), ("f3", "r3"), ("f4", "r4"), ("f5", "r1"), ("f5", "r5")] {
        x(&format!("MATCH (m:Memory {{_uuid: '{f}'}}), (r:Ref {{_uuid: '{r}'}}) CREATE (m)-[:CITE]->(r)"));
    }
    for (f, s) in [("f1", "s1"), ("f2", "s1"), ("f4", "s2")] {
        x(&format!("MATCH (m:Memory {{_uuid: '{f}'}}), (s:Subject {{_uuid: '{s}'}}) CREATE (m)-[:ANCHORED_TO]->(s)"));
    }
}

fn ouvrir_seme(plan: &Value) -> (tempfile::TempDir, Backend) {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(&manifeste(dir.path(), plan)).unwrap_or_else(|e| panic!("le Plan doit charger : {e}"));
    semer(&b);
    (dir, b)
}

fn uuids(r: &Value) -> Vec<String> {
    r["result"].as_array().unwrap_or_else(|| panic!("des lignes : {r}")).iter().map(|l| l["_uuid"].as_str().unwrap().to_string()).collect()
}

fn total(r: &Value) -> i64 {
    r["metadata"]["plan.total"].as_i64().unwrap_or_else(|| panic!("le total : {r}"))
}

/// Toutes les fiches, filtrées par un `kind` facultatif, ordre total.
fn plan_par_kind(filtre: Value) -> Value {
    json!({
        "forms": {
            "lignes": {"select": {"table": "Memory", "filter": filtre, "returns": ["_uuid", "kind"],
                                  "order_by": [{"field": "_uuid"}]}},
            "total": {"count": {"matching": {"table": "Memory", "filter": filtre}}}
        },
        "outputs": {"rows": "lignes", "total": "total"}
    })
}

#[test]
#[ignore = "rouge jusqu'au lot de l'IR (Optional) et au PlanNode"]
fn un_optional_est_retire_par_un_parametre_absent_et_par_une_liste_vide() {
    let (_d, b) = ouvrir_seme(&plan_par_kind(json!({"optional": {"in": {"field": "kind", "param": "kinds"}}})));
    let tout = vec!["f1", "f2", "f3", "f4", "f5"];
    for params in [json!({}), json!({"kinds": []})] {
        let r = b.call("plan_essai", json!({"params": params})).expect("l'appel");
        assert_eq!(uuids(&r), tout, "{params} : le filtre facultatif est retiré");
        assert_eq!(total(&r), 5);
    }
    let r = b.call("plan_essai", json!({"params": {"kinds": ["rule"]}})).expect("l'appel");
    assert_eq!(uuids(&r), vec!["f2", "f4"], "donné, il filtre");
    assert_eq!(total(&r), 2);
}

#[test]
#[ignore = "rouge jusqu'au lot de l'IR (règle de l'absent et du vide) et au PlanNode"]
fn une_liste_vide_hors_optional_est_refusee_en_la_nommant() {
    let e = refus(plan_par_kind(json!({"in": {"field": "kind", "param": "kinds"}})), json!({"kinds": []}));
    assert!(e.contains("kinds"), "le refus nomme le paramètre : {e}");
}

#[test]
#[ignore = "rouge jusqu'au lot de l'IR (validate : ordre total) et au PlanNode"]
fn une_pagination_sans_uuid_final_est_refusee() {
    let plan = json!({
        "forms": {"lignes": {"select": {"table": "Memory", "returns": ["_uuid"],
                                        "order_by": [{"field": "updated_at", "desc": true}],
                                        "limit": {"param": "limit", "max": 50}}}},
        "outputs": {"rows": "lignes"}
    });
    let e = refus(plan, json!({"limit": 2}));
    assert!(e.contains("_uuid"), "le refus dit qu'il faut finir l'ordre par _uuid : {e}");
}

#[test]
#[ignore = "rouge jusqu'au lot de l'IR (validate : limit sans max) et au PlanNode"]
fn un_limit_lie_sans_plafond_est_refuse() {
    let plan = json!({
        "forms": {"lignes": {"select": {"table": "Memory", "returns": ["_uuid"],
                                        "order_by": [{"field": "_uuid"}], "limit": {"param": "limit"}}}},
        "outputs": {"rows": "lignes"}
    });
    let e = refus(plan, json!({"limit": 2}));
    assert!(e.contains("max"), "le refus dit qu'il manque le plafond : {e}");
}

#[test]
#[ignore = "rouge jusqu'au lot de l'IR (offset, Count dans le même Plan) et au PlanNode"]
fn le_total_concorde_avec_les_lignes_quand_on_pagine() {
    let plan = json!({
        "forms": {
            "lignes": {"select": {"table": "Memory", "returns": ["_uuid"],
                                  "order_by": [{"coalesce": ["updated_at", "created_at"], "desc": true}, {"field": "_uuid"}],
                                  "limit": {"param": "limit", "max": 50}, "offset": {"param": "offset"}}},
            "total": {"count": {"matching": {"table": "Memory"}}}
        },
        "outputs": {"rows": "lignes", "total": "total"}
    });
    let (_d, b) = ouvrir_seme(&plan);
    let mut vues = Vec::new();
    for offset in [0, 2, 4] {
        let r = b.call("plan_essai", json!({"params": {"limit": 2, "offset": offset}})).expect("l'appel");
        assert_eq!(total(&r), 5, "le total ne dépend pas de la page");
        vues.extend(uuids(&r));
    }
    let mut tries = vues.clone();
    tries.sort();
    tries.dedup();
    assert_eq!(tries.len(), 5, "trois pages : cinq fiches, aucune sautée ni répétée : {vues:?}");
}

#[test]
#[ignore = "rouge jusqu'au lot de l'IR (Through, validation contre le schéma) et au PlanNode"]
fn un_through_vers_un_bout_qui_n_est_pas_celui_de_la_relation_est_refuse() {
    let filtre = json!({"through": {"relation": "CITE", "direction": "outgoing", "end": "Subject",
                                    "filter": {"equals": {"field": "name", "param": "nom"}}}});
    let e = refus(plan_par_kind(filtre), json!({"nom": "stockage"}));
    assert!(e.contains("CITE") && e.contains("Subject"), "le refus nomme la relation et le bout : {e}");
}

#[test]
#[ignore = "rouge jusqu'au lot de l'IR (serde : Compiled jamais désérialisé) et au PlanNode"]
fn du_cypher_brut_dans_le_plan_est_refuse_a_la_lecture() {
    let e = refus(plan_par_kind(json!({"compiled": "m.kind = 'fact'"})), json!({}));
    assert!(e.contains("compiled"), "le refus nomme la variante interdite : {e}");
}

/// **`cited_by`** sur un dossier : le Plan de la page 03, tel quel, sur les
/// fiches semées. Le préfixe dit « dessous » (`src/` et `src/a.rs#…`), pas
/// `srcx/` ; les Refs rassemblées sont celles qui ont répondu ; l'ordre est
/// celui des dates, décroissant.
#[test]
#[ignore = "rouge jusqu'au lot de l'IR (StartsWith, Through, Collect, Order, offset, Count, Optional, let) et au PlanNode"]
fn cited_by_sur_un_dossier_rend_ce_qui_est_dessous_par_date() {
    let ref_qui_repond = json!({"all_of": [
        {"optional": {"in": {"field": "genre", "param": "genres"}}},
        {"any_of": [
            {"equals": {"field": "valeur", "param": "value"}},
            {"optional": {"starts_with": {"field": "valeur", "param": "value_dir"}}},
            {"optional": {"starts_with": {"field": "valeur", "param": "value_sym"}}}
        ]}
    ]});
    let plan = json!({
        "let": {
            "ref_qui_repond": ref_qui_repond,
            "fiche_gardee": {"all_of": [
                {"through": {"relation": "CITE", "direction": "outgoing", "end": "Ref", "filter": {"let": "ref_qui_repond"}}},
                {"optional": {"in": {"field": "kind", "param": "kinds"}}},
                {"optional": {"through": {"relation": "ANCHORED_TO", "direction": "outgoing", "end": "Subject",
                                          "filter": {"equals": {"field": "name", "param": "subject"}}}}}
            ]}
        },
        "forms": {
            "fiches": {"select": {
                "table": "Memory", "filter": {"let": "fiche_gardee"},
                "returns": ["_uuid", "claim", "kind", "updated_at",
                    {"collect": {"as": "refs", "relation": "CITE", "direction": "outgoing", "end": "Ref",
                                 "fields": ["genre", "valeur", "etat"], "filter": {"let": "ref_qui_repond"}}},
                    {"collect": {"as": "subjects", "relation": "ANCHORED_TO", "direction": "outgoing", "end": "Subject",
                                 "fields": ["name"]}}],
                "order_by": [{"coalesce": ["updated_at", "created_at"], "desc": true}, {"field": "_uuid"}],
                "limit": {"param": "limit", "max": 200}, "offset": {"param": "offset"}}},
            "total": {"count": {"matching": {"table": "Memory", "filter": {"let": "fiche_gardee"}}}}
        },
        "outputs": {"rows": "fiches", "total": "total", "results": {"form": "fiches", "uuid": "_uuid", "entity": "Memory"}}
    });
    let (_d, b) = ouvrir_seme(&plan);
    let v = "git:github.com/x/y:src";
    let r = b
        .call("plan_essai", json!({"params": {"value": v, "value_dir": format!("{v}/"), "value_sym": format!("{v}#"),
                                              "limit": 20, "offset": 0}}))
        .expect("l'appel");
    // f5 (2005), f2 (2002), f1 (2001), puis f3 (pas d'updated_at : son created_at, 1003).
    assert_eq!(uuids(&r), vec!["f5", "f2", "f1", "f3"], "dessous src/, par date décroissante, pas srcx/ ni docs/ : {r}");
    assert_eq!(total(&r), 4);
    let f5 = &r["result"][0];
    let valeurs: Vec<&str> = f5["refs"].as_array().expect("les refs").iter().map(|x| x["valeur"].as_str().unwrap()).collect();
    assert_eq!(valeurs, vec!["git:github.com/x/y:src/a.rs"], "seules les Refs qui ont répondu (pas srcx/e.rs) : {f5}");
    let f2 = &r["result"][1];
    assert_eq!(f2["subjects"][0]["name"], "stockage", "les sujets d'ancrage sont rassemblés : {f2}");

    // Le symbole seul, sans « dessous » : une fiche.
    let s = "git:github.com/x/y:src/a.rs#Catalog::open";
    let r = b.call("plan_essai", json!({"params": {"value": s, "limit": 20, "offset": 0}})).expect("l'appel");
    assert_eq!(uuids(&r), vec!["f2"]);

    // Filtré par sujet.
    let r = b
        .call("plan_essai", json!({"params": {"value": v, "value_dir": format!("{v}/"), "value_sym": format!("{v}#"),
                                              "subject": "stockage", "limit": 20, "offset": 0}}))
        .expect("l'appel");
    assert_eq!(uuids(&r), vec!["f2", "f1"]);
}
