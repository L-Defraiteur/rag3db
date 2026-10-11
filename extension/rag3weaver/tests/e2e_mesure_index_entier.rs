//! Mesure : **les outils de code sur un index entier**, une copie de l'index
//! de l'agent de code (`RAG3WEAVER_MESURE_BASE`, prise à part : une base rag3db
//! n'a qu'un écrivain, et l'original est tenu). Pages
//! `docs/11-octobre-2026-embarquements/06-ou-va-le-temps-d-un-aller-retour.md`
//! et le ticket du semi-masque. **Sans la copie, chaque test le dit et ne mesure
//! rien** : ce sont des mesures, pas des preuves de justesse ; ne les compter
//! comme vertes que si leurs lignes `[usages]`, `[plan]`, `[forme]` paraissent.
//!
//! Run with: RAG3WEAVER_MESURE_BASE=<copie>/code-rag3db-depot.rag3db RUST_TEST_THREADS=1 \
//!   poste mesure ./run_e2e.sh --test e2e_mesure_index_entier

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::time::Instant;

use rag3weaver::code::{default_scope_chunking, register_code_schema};
use rag3weaver::connection::{DbConnection, QueryParam};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

/// La copie de l'index, ou `None` en le disant.
fn base() -> Option<String> {
    let b = std::env::var("RAG3WEAVER_MESURE_BASE").ok();
    if b.is_none() {
        eprintln!("[index entier] NON MESURÉ : RAG3WEAVER_MESURE_BASE absent (une copie de l'index, jamais l'original)");
    }
    b
}

/// **`usages` sur l'index entier de rag3db** (Lucie : « les usages c'est juste
/// des relations, ça ne devrait pas prendre 6 secondes »). La base est une
/// copie de l'index de l'agent de code (`RAG3WEAVER_MESURE_BASE`, prise à part :
/// une base rag3db n'a qu'un écrivain) ; `usages` comme le gabarit
/// (`templates/tools/usages.mmd`), dix appels, le temps de chacun, les phases,
/// et les instructions qui ont pris le plus de temps.
#[test]
#[ignore]
fn ou_va_le_temps_d_usages_sur_l_index_entier() {
    use rag3weaver::dataflow::usage_nodes::{declarations_of, usages_of, DeclarationsConfig, UsagesConfig};
    use rag3weaver::rag3db_connection::phases;
    let Some(base) = base() else { return };
    let conn = Rag3dbConnection::new(&base).expect("ouvrir la copie");
    let config = CatalogConfig { name: Some("code-rag3db".into()), embedding_dim: 768, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(768)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    let cfg = UsagesConfig {
        pivot: "Symbol".into(),
        key: "name".into(),
        defined_by: Some("DEFINES".into()),
        used_by: Some("MENTIONS".into()),
        direct: vec!["CONSUMES".into(), "INHERITS_FROM".into(), "IMPLEMENTS".into()],
        group_by: "usage".into(),
        usages_field: "usages".into(),
        line: "line".into(),
        title: "name".into(),
        kind_field: "scope_type".into(),
        path_fields: vec!["repo_path".into(), "file_path".into()],
        line_field: "start_line".into(),
        edge_mark: rag3weaver::dataflow::graph_walk::EdgeMark { field: "resolution".into(), guessed: vec!["nom".into()] },
        // Comme le gabarit (`templates/tools/usages.mmd`).
        qualifier_field: Some("parent_name".into()),
    };
    let decl = DeclarationsConfig { relation: "HAS_PARENT".into(), field: "declarations".into() };
    for (nom, chemin) in [("update", "src/storage/table/node_table"), ("update", ""), ("getTableID", "")] {
        let appel = || {
            let r = usages_of(&catalog, &cfg, nom, chemin).unwrap();
            let d = declarations_of(&catalog, &cfg, &decl, nom, &r.definitions).unwrap();
            (r, d)
        };
        let _ = appel(); // chauffe
        phases::demarrer();
        let mut temps = Vec::new();
        let mut usages = 0;
        for _ in 0..10 {
            let t = Instant::now();
            let (r, _) = appel();
            temps.push(t.elapsed().as_secs_f64() * 1e3);
            usages = r.usages.len();
        }
        let p = phases::arreter();
        temps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        eprintln!(
            "[usages] {nom} (path « {chemin} ») : {usages} usages, médiane {:.1} ms, {} instructions par appel ; prepare {:.1} ms, execute {:.1} ms, lignes {:.1} ms par appel",
            temps[5],
            p.appels / 10,
            p.prepare_ns as f64 / 1e7,
            p.execute_ns as f64 / 1e7,
            p.lignes_ns as f64 / 1e7
        );
        for (texte, appels, ns, lignes) in phases::par_texte(5) {
            let court: String = texte.chars().take(160).collect();
            eprintln!("[usages]   {:>7.1} ms / appel, ×{}, {} lignes : {court}", ns as f64 / 1e6 / 10.0, appels / 10, lignes / 10);
        }
    }
}

/// Le plan du moteur pour le saut le plus coûteux d'`usages` (MENTIONS
/// entrant vers Symbol), sur la copie de l'index : `EXPLAIN` et `PROFILE`.
#[test]
#[ignore]
fn le_plan_du_saut_d_usages_sur_l_index_entier() {
    use rag3weaver::connection::CypherValue;
    let Some(base) = base() else { return };
    let conn = Rag3dbConnection::new(&base).expect("ouvrir la copie");
    let symbole = conn
        .execute("MATCH (s:`Symbol`) WHERE s.`name` = 'update' RETURN s.`_uuid` LIMIT 1")
        .unwrap()
        .rows[0][0]
        .clone();
    let saut = "MATCH (d:`Symbol` {_uuid: $u})<-[r:`MENTIONS`]-(m:`Scope`) RETURN m.`_uuid`, m.`name`";
    for prefixe in ["EXPLAIN", "PROFILE"] {
        let r = conn.execute_with_params(&format!("{prefixe} {saut}"), &[QueryParam::new("u", symbole.clone())]).unwrap_or_else(|e| panic!("{prefixe} : {e}"));
        for ligne in &r.rows {
            for v in ligne {
                if let CypherValue::String(t) = v {
                    eprintln!("[plan] {prefixe}\n{t}");
                }
            }
        }
    }
}

/// **Les formes du saut d'`usages`, mesurées** sur la copie de l'index
/// (sonde de mesure, en Cypher brut) : le saut MENTIONS → Symbol de `update`
/// tel qu'aujourd'hui (V1), sans propriété du voisin (V2), puis la lecture des
/// champs par identifiant interne (V3) ou par `_uuid` (V4).
#[test]
#[ignore]
fn les_formes_du_saut_d_usages_sur_l_index_entier() {
    use rag3weaver::connection::CypherValue;
    let Some(base) = base() else { return };
    let conn = Rag3dbConnection::new(&base).expect("ouvrir la copie");
    let sym = conn.execute("MATCH (s:`Symbol`) WHERE s.`name` = 'update' RETURN s.`_uuid` LIMIT 1").unwrap().rows[0][0].clone();
    let liste = |v: Vec<CypherValue>| CypherValue::List(v);
    let champs = "m.`_uuid`, m.`name`, m.`scope_type`, m.`repo_path`, m.`file_path`, m.`start_line`";
    let mediane = |q: &str, p: &[QueryParam]| -> (f64, usize) {
        let mut t = Vec::new();
        let mut n = 0;
        for _ in 0..7 {
            let d = Instant::now();
            n = conn.execute_with_params(q, p).unwrap_or_else(|e| panic!("{e}\n{q}")).rows.len();
            t.push(d.elapsed().as_secs_f64() * 1e3);
        }
        t.sort_by(|a, b| a.partial_cmp(b).unwrap());
        (t[3], n)
    };
    let uuids = [QueryParam::new("uuids", liste(vec![sym.clone()]))];
    let v1 = format!("UNWIND $uuids AS u MATCH (d:`Symbol` {{_uuid: u}})<-[r:`MENTIONS`]-(m:`Scope`) RETURN u, {champs}");
    let (t1, n1) = mediane(&v1, &uuids);
    eprintln!("[forme] V1 saut tel quel, 6 colonnes du voisin : {t1:.1} ms, {n1} lignes");
    let v2 = "UNWIND $uuids AS u MATCH (d:`Symbol` {_uuid: u})<-[r:`MENTIONS`]-(m:`Scope`) RETURN u, id(m)";
    let (t2, n2) = mediane(v2, &uuids);
    eprintln!("[forme] V2 saut sans propriété du voisin (id interne) : {t2:.1} ms, {n2} lignes");
    let v2b = "UNWIND $uuids AS u MATCH (d:`Symbol` {_uuid: u})<-[r:`MENTIONS`]-(m:`Scope`) RETURN u, m.`_uuid`";
    let (t2b, n2b) = mediane(v2b, &uuids);
    eprintln!("[forme] V2b saut qui ne rend que m._uuid : {t2b:.1} ms, {n2b} lignes");
    // Le saut en deux temps : V2b puis V4, et ses lignes comparées à V1.
    let deux_temps = || -> (f64, std::collections::BTreeSet<String>) {
        let d = Instant::now();
        let premiers = conn.execute_with_params(v2b, &uuids).unwrap();
        let voisins: Vec<CypherValue> = premiers.rows.iter().map(|r| r[1].clone()).collect();
        let q = format!("UNWIND $uuids AS u MATCH (m:`Scope` {{_uuid: u}}) RETURN {champs}");
        let champs_lus = conn.execute_with_params(&q, &[QueryParam::new("uuids", liste(voisins))]).unwrap();
        let ms = d.elapsed().as_secs_f64() * 1e3;
        (ms, champs_lus.rows.iter().map(|r| format!("{r:?}")).collect())
    };
    let mut t = Vec::new();
    let mut lignes = Default::default();
    for _ in 0..7 {
        let (ms, l) = deux_temps();
        t.push(ms);
        lignes = l;
    }
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let reference: std::collections::BTreeSet<String> = conn.execute_with_params(&v1, &uuids).unwrap().rows.iter().map(|r| format!("{:?}", &r[1..])).collect();
    eprintln!("[forme] deux temps (V2b puis V4) : {:.1} ms, mêmes lignes que V1 : {}", t[3], lignes == reference);
    let ids: Vec<CypherValue> = conn.execute_with_params(v2, &uuids).unwrap().rows.iter().map(|r| r[1].clone()).collect();
    let v3 = format!("UNWIND $ids AS i MATCH (m:`Scope`) WHERE id(m) = i RETURN {champs}");
    match conn.execute_with_params(&v3, &[QueryParam::new("ids", liste(ids.clone()))]) {
        Ok(_) => {
            let (t3, n3) = mediane(&v3, &[QueryParam::new("ids", liste(ids))]);
            eprintln!("[forme] V3 les champs par id interne : {t3:.1} ms, {n3} lignes");
        }
        Err(e) => eprintln!("[forme] V3 refusée : {e}"),
    }
    let voisins: Vec<CypherValue> = conn.execute_with_params(&v1, &uuids).unwrap().rows.iter().map(|r| r[1].clone()).collect();
    let v4 = format!("UNWIND $uuids AS u MATCH (m:`Scope` {{_uuid: u}}) RETURN {champs}");
    let (t4, n4) = mediane(&v4, &[QueryParam::new("uuids", liste(voisins))]);
    eprintln!("[forme] V4 les champs par _uuid (clé primaire) : {t4:.1} ms, {n4} lignes");
    let p3 = conn.execute_with_params(&format!("PROFILE {v4}"), &[QueryParam::new("uuids", liste(conn.execute_with_params(&v1, &uuids).unwrap().rows.iter().map(|r| r[1].clone()).collect()))]);
    if let Ok(r) = p3 {
        for l in &r.rows {
            if let Some(CypherValue::String(t)) = l.first() {
                for x in t.lines().filter(|x| x.contains("SCAN") || x.contains("NumOutputTuples")) {
                    eprintln!("[forme] V4 plan {}", x.replace(['│', '┌', '┐', '└', '┘', '─'], "").trim());
                }
            }
        }
    }
}

/// **Les morceaux à texte vide** (le gel de luciepc : une boucle de
/// `run_index` sur eux) : leur nombre, et les vingt premiers fichiers qui en
/// portent, par le scope dont ils sont issus.
#[test]
#[ignore]
fn les_morceaux_a_texte_vide() {
    let Some(base) = base() else { return };
    let conn = Rag3dbConnection::new(&base).expect("ouvrir la copie");
    let n = conn
        .execute("MATCH (n:`Scope_Chunk`) WHERE n.`_text` IS NULL OR n.`_text` = '' RETURN count(n)")
        .unwrap_or_else(|e| panic!("{e}"));
    eprintln!("[vides] morceaux à texte vide : {:?}", n.rows.first());
    let total = conn.execute("MATCH (n:`Scope_Chunk`) RETURN count(n)").unwrap();
    eprintln!("[vides] morceaux en tout : {:?}", total.rows.first());
    let par_fichier = conn.execute(
        "MATCH (n:`Scope_Chunk`)-[r]->(s:`Scope`) WHERE n.`_text` IS NULL OR n.`_text` = '' \
         RETURN s.`file_path` AS f, label(r) AS rel, count(n) AS c ORDER BY c DESC LIMIT 20",
    );
    match par_fichier {
        Ok(r) => r.rows.iter().for_each(|l| eprintln!("[vides]   {:?}", l)),
        Err(e) => eprintln!("[vides] par fichier refusé : {e}"),
    }
}

/// **Le texte des morceaux vides existait quand il a été haché** : parmi les
/// morceaux à texte vide, ceux dont `_text_hash` n'est ni absent ni l'empreinte
/// d'une chaîne vide. Leur texte s'est perdu entre le hachage et l'écriture.
#[test]
#[ignore]
fn les_morceaux_vides_ont_le_hash_d_un_texte() {
    let Some(base) = base() else { return };
    let conn = Rag3dbConnection::new(&base).expect("ouvrir la copie");
    const SHA256_VIDE: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let r = conn
        .execute(&format!(
            "MATCH (n:`Scope_Chunk`) WHERE n.`_text` IS NULL OR n.`_text` = '' \
             RETURN count(n), \
                    count(CASE WHEN n.`_text_hash` IS NULL OR n.`_text_hash` = '' THEN 1 END), \
                    count(CASE WHEN n.`_text_hash` = '{SHA256_VIDE}' THEN 1 END)"
        ))
        .unwrap_or_else(|e| panic!("{e}"));
    let l = &r.rows[0];
    let (total, sans, vide) = (l[0].as_i64().unwrap_or(0), l[1].as_i64().unwrap_or(0), l[2].as_i64().unwrap_or(0));
    eprintln!(
        "[vides] morceaux à texte vide : {total} ; sans hash : {sans} ; hash d'une chaîne vide : {vide} ; hash d'un vrai texte : {}",
        total - sans - vide
    );
}
