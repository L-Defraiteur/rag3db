//! E2E : **un graphe de formes confié entier au dialecte**, sur une base
//! rag3db vivante, fondu par le dialecte puis découpé (un nœud, une
//! instruction) : les deux rendent les mêmes lignes.
//!
//! Page de conception : `docs/11-octobre-2026-embarquements/01-chainage-des-formes.md`.
//! L'oracle de chaque graphe est la même suite de formes exécutée à la main,
//! une par une, comme le code au-dessus le faisait avant.
//!
//! Run with: ./run_e2e.sh --test e2e_chainage

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rag3weaver::config::{EntityConfig, FieldType, SimpleFieldDef};
use rag3weaver::connection::{CypherValue, QueryParam, QueryResult};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::plan::{PlanError, PlanOptions, PlanReport};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};
use rag3weaver_ir::{Column, Count, Direction, Form, Hop, Plan, Predicate, Select, Write};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn entite(champs: &[&str]) -> EntityConfig {
    let mut fields = HashMap::new();
    for (i, c) in champs.iter().enumerate() {
        fields.insert(c.to_string(), SimpleFieldDef { field_type: FieldType::String, is_title: i == 0, is_content: i == 0, ..Default::default() });
    }
    EntityConfig { fields, ..Default::default() }
}

fn ligne(paires: &[(&str, &str)]) -> BTreeMap<String, CypherValue> {
    paires.iter().map(|(k, v)| (k.to_string(), CypherValue::String(v.to_string()))).collect()
}

/// Deux fichiers, quatre symboles. `f1` définit `a` et `b`, `f2` définit
/// `c` ; `c` utilise `a` et `b`, `d` utilise `a`.
struct Scene {
    cat: Catalog,
    f1: String,
    a: String,
    b: String,
}

fn scene() -> Scene {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    assert!(std::path::Path::new(&ext).exists(), "vector extension not found at {ext} — ./run_e2e.sh --build-only");
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("chainage-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut cat = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    cat.initialize().unwrap();
    cat.register_entity("File", entite(&["path"])).unwrap();
    cat.register_entity("Symbol", entite(&["name", "note"])).unwrap();
    cat.register_relation("defines", "File", "Symbol").unwrap();
    cat.register_relation("uses", "Symbol", "Symbol").unwrap();

    let fichiers = [ligne(&[("path", "f1")]), ligne(&[("path", "f2")])];
    let symboles: Vec<_> = ["a", "b", "c", "d"].iter().map(|n| ligne(&[("name", n), ("note", "x")])).collect();
    let uf: Vec<String> = fichiers.iter().map(|l| cat.entity_uuid("File", l).unwrap()).collect();
    let us: Vec<String> = symboles.iter().map(|l| cat.entity_uuid("Symbol", l).unwrap()).collect();
    cat.ingest_entities("File", fichiers.to_vec()).unwrap();
    cat.ingest_entities("Symbol", symboles).unwrap();
    for (rel, de, vers) in [
        ("defines", &uf[0], &us[0]),
        ("defines", &uf[0], &us[1]),
        ("defines", &uf[1], &us[2]),
        ("uses", &us[2], &us[0]),
        ("uses", &us[2], &us[1]),
        ("uses", &us[3], &us[0]),
    ] {
        cat.link(rel, de.as_str(), vers.as_str(), BTreeMap::new()).unwrap();
    }
    Scene { f1: uf[0].clone(), a: us[0].clone(), b: us[1].clone(), cat }
}

fn uuids(v: &[&str]) -> QueryParam {
    QueryParam::new("uuids", CypherValue::List(v.iter().map(|u| CypherValue::String(u.to_string())).collect()))
}

fn sel_file() -> Select {
    Select::by_uuids("File", vec![Column::Node("_uuid".into())])
}
fn hop_defines() -> Hop {
    Hop::new("File", "defines", "Symbol", Direction::Outgoing)
}
fn deg_uses() -> Count {
    Count::Edges { start: "Symbol".into(), relation: "uses".into(), direction: Direction::Incoming }
}

/// Les lignes, comme un ensemble de textes : l'ordre des lignes n'est pas
/// promis, ni par le moteur ni par le graphe.
fn ensemble(r: &QueryResult) -> BTreeSet<String> {
    r.rows.iter().map(|l| format!("{l:?}")).collect()
}

/// Le graphe fondu, puis découpé : chaque sortie a les mêmes lignes.
fn deux_fois(cat: &Catalog, plan: &Plan, options: PlanOptions) -> (PlanReport, PlanReport) {
    let fondu = cat.run_plan(plan, options).unwrap_or_else(|e| panic!("fondu : {e}"));
    let decoupe = cat.run_plan(plan, PlanOptions { split: true, ..options }).unwrap_or_else(|e| panic!("découpé : {e}"));
    for (nom, lignes) in &fondu.outputs {
        assert_eq!(ensemble(lignes), ensemble(&decoupe.outputs[nom]), "sortie « {nom} » : la fusion rend les lignes du repli");
    }
    (fondu, decoupe)
}

/// L'oracle : la même suite exécutée à la main, forme par forme.
fn a_la_main(cat: &Catalog, f: &str) -> QueryResult {
    let d = cat.dialect_arc();
    let s = cat.execute_raw_with_params(&d.select(&sel_file()).unwrap(), &[uuids(&[f])]).unwrap();
    let fichiers: Vec<String> = s.rows.iter().map(|r| r[0].as_str().unwrap().to_string()).collect();
    let h = cat.execute_raw_with_params(&d.hop(&hop_defines()).unwrap(), &[uuids(&fichiers.iter().map(String::as_str).collect::<Vec<_>>())]).unwrap();
    let mut atteints: Vec<String> = h.rows.iter().map(|r| r[1].as_str().unwrap().to_string()).collect();
    atteints.dedup();
    cat.execute_raw_with_params(&d.count(&deg_uses()).unwrap(), &[uuids(&atteints.iter().map(String::as_str).collect::<Vec<_>>())]).unwrap()
}

/// **Une chaîne** `Select` → `Hop` → `Count` : les lignes de l'oracle, trois
/// instructions dans une transaction, atomique et dit.
#[test]
#[ignore]
fn une_chaine_rend_les_lignes_de_l_oracle_et_dit_sa_decoupe() {
    let s = scene();
    let plan = Plan::new()
        .node("sel", Form::Select(sel_file()), vec![uuids(&[&s.f1])])
        .node("hop", Form::Hop(hop_defines()), vec![])
        .node("deg", Form::Count(deg_uses()), vec![])
        .edge("sel", 0, "hop")
        .edge("hop", 1, "deg")
        .output("deg");
    let (r, decoupe) = deux_fois(&s.cat, &plan, PlanOptions { require_atomic: true, ..Default::default() });
    let attendu = a_la_main(&s.cat, &s.f1);
    assert_eq!(ensemble(&r.outputs["deg"]), ensemble(&attendu), "les lignes du graphe sont celles de l'oracle");
    let degres: BTreeMap<String, i64> =
        r.outputs["deg"].rows.iter().map(|l| (l[0].as_str().unwrap().to_string(), l[1].as_i64().unwrap())).collect();
    assert_eq!(degres, BTreeMap::from([(s.a.clone(), 2), (s.b.clone(), 1)]), "a est utilisé deux fois, b une fois");
    assert_eq!(r.instructions, 1, "fondu en une instruction : {r:?}");
    assert_eq!(r.fused, vec![vec!["sel".to_string(), "hop".to_string(), "deg".to_string()]]);
    assert!(r.atomic && r.atomic_reason.is_none(), "{r:?}");
    assert_eq!(decoupe.instructions, 3);
    assert_eq!(decoupe.fused, vec![vec!["sel".to_string()], vec!["hop".to_string()], vec!["deg".to_string()]]);
    assert!(decoupe.atomic, "trois instructions dans une transaction");
}

/// **Un nœud qui en nourrit deux** : le saut et le degré de `File` lisent
/// tous deux la sélection ; les deux sorties sont rendues.
#[test]
#[ignore]
fn un_noeud_qui_en_nourrit_deux_rend_deux_sorties() {
    let s = scene();
    let deg_defines = Count::Edges { start: "File".into(), relation: "defines".into(), direction: Direction::Outgoing };
    let plan = Plan::new()
        .node("sel", Form::Select(sel_file()), vec![uuids(&[&s.f1])])
        .node("hop", Form::Hop(hop_defines()), vec![])
        .node("deg", Form::Count(deg_defines.clone()), vec![])
        .edge("sel", 0, "hop")
        .edge("sel", 0, "deg")
        .output("hop")
        .output("deg");
    let (r, _) = deux_fois(&s.cat, &plan, PlanOptions::default());
    let d = s.cat.dialect_arc();
    let hop = s.cat.execute_raw_with_params(&d.hop(&hop_defines()).unwrap(), &[uuids(&[&s.f1])]).unwrap();
    let deg = s.cat.execute_raw_with_params(&d.count(&deg_defines).unwrap(), &[uuids(&[&s.f1])]).unwrap();
    assert_eq!(ensemble(&r.outputs["hop"]), ensemble(&hop));
    assert_eq!(ensemble(&r.outputs["deg"]), ensemble(&deg));
    assert_eq!(r.outputs["deg"].rows[0][1].as_i64(), Some(2), "f1 définit deux symboles");
    assert_eq!(r.instructions, 3, "sel nourrit deux nœuds : rien ne se fond");
}

fn note_de(cat: &Catalog, u: &str) -> CypherValue {
    let q = cat.dialect_arc().select(&Select::by_uuids("Symbol", vec![Column::Node("note".into())])).unwrap();
    let r = cat.execute_raw_with_params(&q, &[uuids(&[u])]).unwrap();
    r.rows[0][1].clone()
}

/// **Une écriture dans le graphe**, puis un nœud qui échoue : la
/// transaction est défaite, la marque n'est pas restée.
#[test]
#[ignore]
fn un_noeud_qui_echoue_defait_l_ecriture_d_avant() {
    let s = scene();
    let sel = Select::by_uuids("Symbol", vec![Column::Node("_uuid".into())]);
    let marquer = Write::Mark { table: "Symbol".into(), set: vec![("note".into(), None)] };
    let fautif = Select::filtered("Symbol", Predicate::Compiled("m.`nope` = 1".into()), vec![Column::Node("_uuid".into())]);
    let plan = Plan::new()
        .node("sel", Form::Select(sel.clone()), vec![uuids(&[&s.a])])
        .node("marquer", Form::Write(marquer.clone()), vec![])
        .node("fautif", Form::Select(fautif), vec![])
        .edge("sel", 0, "marquer");
    for split in [false, true] {
        match s.cat.run_plan(&plan, PlanOptions { split, ..Default::default() }) {
            Err(PlanError::Failed { node, rolled_back, .. }) => {
                assert_eq!(node, "fautif", "split = {split}");
                assert!(rolled_back, "la transaction est défaite (split = {split})");
            }
            autre => panic!("échec attendu sur « fautif » (split = {split}), rendu {autre:?}"),
        }
        assert_eq!(note_de(&s.cat, &s.a), CypherValue::String("x".into()), "la marque n'est pas restée (split = {split})");
    }

    // Le même graphe sans le nœud fautif écrit, et le dit atomique.
    let plan = Plan::new()
        .node("sel", Form::Select(sel), vec![uuids(&[&s.a])])
        .node("marquer", Form::Write(marquer), vec![])
        .edge("sel", 0, "marquer");
    let r = s.cat.run_plan(&plan, PlanOptions { require_atomic: true, ..Default::default() }).unwrap();
    assert!(r.atomic);
    assert_eq!(r.fused, vec![vec!["sel".to_string(), "marquer".to_string()]], "la lecture et la marque en une instruction");
    assert!(note_de(&s.cat, &s.a).is_null(), "la marque a vidé le champ");
    assert_eq!(note_de(&s.cat, &s.b), CypherValue::String("x".into()), "et seulement pour a");
}

/// **Une entrée vide** n'envoie pas d'instruction : un fichier sans symbole
/// laisse le degré sans ligne, et le rapport ne compte que ce qui est parti.
#[test]
#[ignore]
fn une_entree_vide_n_envoie_rien() {
    let s = scene();
    let plan = Plan::new()
        .node("sel", Form::Select(sel_file()), vec![uuids(&["aucun"])])
        .node("hop", Form::Hop(hop_defines()), vec![])
        .node("deg", Form::Count(deg_uses()), vec![])
        .edge("sel", 0, "hop")
        .edge("hop", 1, "deg")
        .output("deg");
    let (r, decoupe) = deux_fois(&s.cat, &plan, PlanOptions::default());
    assert!(r.outputs["deg"].rows.is_empty());
    assert_eq!(r.instructions, 1, "fondu : une instruction, qui ne trouve rien : {r:?}");
    assert_eq!(decoupe.instructions, 1, "découpé : seule la sélection est partie : {decoupe:?}");
}
