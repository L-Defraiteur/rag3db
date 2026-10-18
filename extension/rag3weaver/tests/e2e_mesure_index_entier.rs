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
        edge_mark: rag3weaver::dataflow::graph_walk::EdgeMark { field: "resolution".into(), guessed: vec!["nom".into(), "dossier".into()], doubtful: vec!["ambigu".into()] },
        // Comme le gabarit (`templates/tools/usages.mmd`).
        qualifier_field: Some("parent_name".into()),
    };
    let decl = DeclarationsConfig { relation: "HAS_PARENT".into(), field: "declarations".into() };
    // Les deux formes du saut, l'une après l'autre : où va le temps de chacune.
    for deux in [false, true] {
    catalog.set_two_step_hops(Some(deux));
    let forme = if deux { "en deux temps" } else { "en un temps" };
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
            "[usages] {forme}, {nom} (path « {chemin} ») : {usages} usages, médiane {:.1} ms, {} instructions par appel ; prepare {:.1} ms, execute {:.1} ms, lignes {:.1} ms par appel",
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
    catalog.set_two_step_hops(None);
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
    let liste = CypherValue::List(vec![symbole.clone()]);
    let saut = "UNWIND $uuids AS u MATCH (d:`Symbol` {_uuid: u})<-[r:`MENTIONS`]-(m:`Scope`) \
                RETURN u, m.`_uuid`, m.`name`, m.`scope_type`, m.`repo_path`, m.`file_path`, m.`start_line`";
    let voisins: Vec<CypherValue> = conn
        .execute_with_params(saut, &[QueryParam::new("uuids", liste.clone())])
        .unwrap()
        .rows
        .iter()
        .map(|r| r[1].clone())
        .collect();
    // Le saut d'origine, puis la lecture par clé du saut en deux temps.
    let par_cle = "UNWIND $uuids AS u MATCH (m:`Scope` {_uuid: u}) \
                   RETURN u, m.`name`, m.`scope_type`, m.`repo_path`, m.`file_path`, m.`start_line`";
    // Les mêmes requêtes écrites autrement : la liste en filtre (`IN`), et un
    // seul uuid en paramètre scalaire ; médiane de sept, et les lignes que sort
    // le balayage de la table d'arrivée.
    let champs = "m.`name`, m.`scope_type`, m.`repo_path`, m.`file_path`, m.`start_line`";
    let variantes = [
        ("saut, UNWIND", saut.to_string(), QueryParam::new("uuids", liste.clone())),
        ("saut, IN", format!("MATCH (d:`Symbol`)<-[r:`MENTIONS`]-(m:`Scope`) WHERE d.`_uuid` IN $uuids RETURN d.`_uuid`, m.`_uuid`, {champs}"), QueryParam::new("uuids", liste.clone())),
        ("saut, scalaire", format!("MATCH (d:`Symbol` {{_uuid: $u}})<-[r:`MENTIONS`]-(m:`Scope`) RETURN d.`_uuid`, m.`_uuid`, {champs}"), QueryParam::new("u", symbole.clone())),
        ("par clé, UNWIND", par_cle.to_string(), QueryParam::new("uuids", CypherValue::List(voisins.clone()))),
        ("par clé, IN", format!("MATCH (m:`Scope`) WHERE m.`_uuid` IN $uuids RETURN m.`_uuid`, {champs}"), QueryParam::new("uuids", CypherValue::List(voisins.clone()))),
    ];
    for (nom, q, p) in &variantes {
        let mut t = Vec::new();
        let mut n = 0;
        for _ in 0..7 {
            let d = std::time::Instant::now();
            n = conn.execute_with_params(q, std::slice::from_ref(p)).unwrap_or_else(|e| panic!("{nom} : {e}")).rows.len();
            t.push(d.elapsed().as_secs_f64() * 1e3);
        }
        t.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let profil = conn.execute_with_params(&format!("PROFILE {q}"), std::slice::from_ref(p)).unwrap();
        let texte = profil.rows.iter().flatten().map(|v| match v { CypherValue::String(t) => t.clone(), _ => String::new() }).collect::<String>();
        let balayages: Vec<String> = texte.lines().filter(|l| l.contains("NumOutputTuples")).map(|l| l.replace(['│', '┌', '┐', '└', '┘', '─'], "").split_whitespace().filter(|x| x.parse::<u64>().is_ok()).collect::<Vec<_>>().join("/")).collect();
        let mut lignes: Vec<String> = conn.execute_with_params(q, std::slice::from_ref(p)).unwrap().rows.iter().map(|r| format!("{r:?}")).collect();
        lignes.sort();
        let empreinte = { use std::hash::{Hash, Hasher}; let mut h = std::collections::hash_map::DefaultHasher::new(); lignes.hash(&mut h); h.finish() };
        eprintln!("[variante] {nom} : médiane {:.1} ms, {n} lignes, empreinte {empreinte:016x} ; tuples par opérateur {}", t[3], balayages.join(" "));
    }
    // Un saut de Scope vers Scope (départ et arrivée dans la même table), depuis
    // le premier voisin : la forme d'IMPLEMENTS, INHERITS_FROM et HAS_PARENT.
    let saut_scope = "UNWIND $uuids AS u MATCH (d:`Scope` {_uuid: u})-[r:`HAS_PARENT`]->(m:`Scope`) \
                      RETURN u, m.`_uuid`, m.`name`, m.`repo_path`, m.`file_path`, m.`declarations`";
    for (nom, q, p) in [
        ("saut", saut, liste.clone()),
        ("par clé", par_cle, CypherValue::List(voisins.clone())),
        ("saut Scope vers Scope", saut_scope, CypherValue::List(voisins[..1].to_vec())),
    ] {
        for prefixe in ["EXPLAIN", "PROFILE"] {
            let r = conn
                .execute_with_params(&format!("{prefixe} {q}"), &[QueryParam::new("uuids", p.clone())])
                .unwrap_or_else(|e| panic!("{prefixe} : {e}"));
            eprintln!("[plan] {nom}, {prefixe} : {} ligne(s)", r.rows.len());
            for ligne in &r.rows {
                for v in ligne {
                    match v {
                        CypherValue::String(t) => eprintln!("[plan] {nom}, {prefixe}\n{t}"),
                        autre => eprintln!("[plan] {nom}, {prefixe} : {autre:?}"),
                    }
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

/// **Ce qui distingue les morceaux vides des pleins** : pour chaque colonne de
/// `Scope_Chunk`, la part de valeurs présentes dans chaque groupe et, pour les
/// nombres, le minimum, la moyenne et le maximum ; et la place des vides dans
/// leur scope (le premier morceau, le dernier, au-delà de quel offset).
#[test]
#[ignore]
fn ce_qui_distingue_les_morceaux_vides() {
    use rag3weaver::connection::CypherValue;
    let Some(base) = base() else { return };
    let conn = Rag3dbConnection::new(&base).expect("ouvrir la copie");
    let info = conn.execute("CALL TABLE_INFO('Scope_Chunk') RETURN *").unwrap_or_else(|e| panic!("{e}"));
    let colonnes: Vec<(String, String)> = info
        .rows
        .iter()
        .filter_map(|r| Some((r.get(1)?.as_str()?.to_string(), r.get(2)?.as_str()?.to_string())))
        .collect();
    eprintln!("[distingue] colonnes : {:?}", colonnes);
    let vide = "(n.`_text` IS NULL OR n.`_text` = '')";
    for (nom, genre) in &colonnes {
        if nom == "_text" {
            continue;
        }
        let c = format!("n.`{}`", nom.replace('`', ""));
        let nombre = ["INT64", "INT32", "DOUBLE", "UINT64", "INT16"].iter().any(|t| genre.starts_with(t))
            || (genre.starts_with("FLOAT") && !genre.contains('['));
        let q = if nombre {
            format!("MATCH (n:`Scope_Chunk`) RETURN {vide} AS v, count(n), count({c}), min({c}), avg({c}), max({c}) ORDER BY v")
        } else {
            format!("MATCH (n:`Scope_Chunk`) RETURN {vide} AS v, count(n), count({c}) ORDER BY v")
        };
        match conn.execute(&q) {
            Ok(r) => {
                for l in &r.rows {
                    let groupe = if matches!(l.first(), Some(CypherValue::Bool(true))) { "vides " } else { "pleins" };
                    let (n, presents) = (l[1].as_i64().unwrap_or(0), l[2].as_i64().unwrap_or(0));
                    let stats = if nombre { format!(" ; min {:?} moy {:?} max {:?}", l.get(3), l.get(4), l.get(5)) } else { String::new() };
                    eprintln!("[distingue] {nom:<22} {groupe} : présents {presents}/{n} ({:.1} %){stats}", 100.0 * presents as f64 / n.max(1) as f64);
                }
            }
            Err(e) => eprintln!("[distingue] {nom} : {e}"),
        }
    }
    // La place dans le scope : le rang du morceau parmi ceux de son scope.
    let place = conn.execute(&format!(
        "MATCH (n:`Scope_Chunk`)-[:`Scope_CHUNKED_FROM`]->(s:`Scope`) \
         WITH s, collect({{d: n.`_start_char`, v: {vide}}}) AS ms \
         UNWIND ms AS m \
         WITH s, m, size(ms) AS total, size([x IN ms WHERE x.d < m.d]) AS rang \
         RETURN m.v AS v, count(*), avg(total), \
                count(CASE WHEN rang = 0 THEN 1 END), count(CASE WHEN rang = total - 1 THEN 1 END), \
                count(CASE WHEN total = 1 THEN 1 END) ORDER BY v"
    ));
    match place {
        Ok(r) => {
            for l in &r.rows {
                let groupe = if matches!(l.first(), Some(CypherValue::Bool(true))) { "vides " } else { "pleins" };
                eprintln!(
                    "[distingue] place {groupe} : {:?} morceaux, scopes de {:?} morceaux en moyenne ; premiers {:?}, derniers {:?}, seuls de leur scope {:?}",
                    l.get(1), l.get(2), l.get(3), l.get(4), l.get(5)
                );
            }
        }
        Err(e) => eprintln!("[distingue] place : {e}"),
    }
    // Les scopes : tout vide, tout plein, ou mêlés ?
    let melange = conn.execute(&format!(
        "MATCH (n:`Scope_Chunk`)-[:`Scope_CHUNKED_FROM`]->(s:`Scope`) \
         WITH s, count(CASE WHEN {vide} THEN 1 END) AS v, count(n) AS t \
         RETURN count(CASE WHEN v = 0 THEN 1 END), count(CASE WHEN v = t THEN 1 END), count(CASE WHEN v > 0 AND v < t THEN 1 END)"
    ));
    if let Ok(r) = melange {
        eprintln!("[distingue] scopes sans vide {:?}, tout vides {:?}, mêlés {:?}", r.rows[0].first(), r.rows[0].get(1), r.rows[0].get(2));
    }
}

/// **Les fichiers des scopes vides** : pour chaque fichier, ses scopes tout
/// vides et ses scopes en tout, écrits dans `RAG3WEAVER_MESURE_SORTIE` (une
/// ligne par fichier, tabulée), pour être croisés avec l'historique git ; et
/// les colonnes de `File`.
#[test]
#[ignore]
fn les_fichiers_des_scopes_vides() {
    let Some(base) = base() else { return };
    let conn = Rag3dbConnection::new(&base).expect("ouvrir la copie");
    if let Ok(r) = conn.execute("CALL TABLE_INFO('File') RETURN *") {
        let c: Vec<String> = r.rows.iter().filter_map(|l| l.get(1).and_then(|v| v.as_str()).map(|s| s.to_string())).collect();
        eprintln!("[fichiers] colonnes de File : {c:?}");
    }
    let r = conn
        .execute(
            "MATCH (n:`Scope_Chunk`)-[:`Scope_CHUNKED_FROM`]->(s:`Scope`) \
             WITH s, count(CASE WHEN n.`_text` IS NULL OR n.`_text` = '' THEN 1 END) AS v, count(n) AS t \
             RETURN s.`file_path`, count(CASE WHEN v = t THEN 1 END), count(s)",
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let sortie = std::env::var("RAG3WEAVER_MESURE_SORTIE").unwrap_or_else(|_| "/dev/stderr".into());
    let texte: String = r
        .rows
        .iter()
        .map(|l| format!("{}\t{}\t{}\n", l[0].as_str().unwrap_or(""), l[1].as_i64().unwrap_or(0), l[2].as_i64().unwrap_or(0)))
        .collect();
    std::fs::write(&sortie, texte).unwrap();
    eprintln!("[fichiers] {} fichiers écrits dans {sortie}", r.rows.len());
    // Les champs de File, un fichier par ligne, à côté.
    let f = conn
        .execute("MATCH (f:`File`) RETURN f.`absolute_path`, f.`path`, f.`revision`, f.`_snapshot`, f.`cursor`, f.`language`, f.`size_bytes`, f.`_absent_since`")
        .unwrap_or_else(|e| panic!("{e}"));
    let texte: String = f
        .rows
        .iter()
        .map(|l| l.iter().map(|v| match v { rag3weaver::connection::CypherValue::String(s) => s.clone(), autre => format!("{autre:?}") }).collect::<Vec<_>>().join("\t") + "\n")
        .collect();
    if sortie.starts_with("/dev/") {
        eprintln!("[fichiers] RAG3WEAVER_MESURE_SORTIE absent : les champs de File ne sont pas écrits");
        return;
    }
    std::fs::write(format!("{sortie}.files"), texte).unwrap();
    eprintln!("[fichiers] {} lignes de File écrites dans {sortie}.files", f.rows.len());
}

/// **Ce que porte le texte des scopes tout vides** : la part de scopes dont
/// `content` contient un accent grave, un guillemet, une barre oblique inverse,
/// ou est lui-même vide ; leur longueur — chez les scopes tout vides et chez
/// ceux qui n'ont aucun morceau vide.
#[test]
#[ignore]
fn ce_que_porte_le_texte_des_scopes_vides() {
    use rag3weaver::connection::CypherValue;
    let Some(base) = base() else { return };
    let conn = Rag3dbConnection::new(&base).expect("ouvrir la copie");
    // En deux temps : le classement des scopes sans leur texte, puis leur
    // texte par paquets, par clé (la même requête en un temps fait tomber le
    // moteur : SIGSEGV, le 12 octobre).
    let classes = conn
        .execute(
            "MATCH (n:`Scope_Chunk`)-[:`Scope_CHUNKED_FROM`]->(s:`Scope`) \
             WITH s, count(CASE WHEN n.`_text` IS NULL OR n.`_text` = '' THEN 1 END) AS v, count(n) AS t \
             WHERE v = 0 OR v = t \
             RETURN v > 0 AS vide, s.`_uuid`",
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let vide_de: std::collections::HashMap<String, bool> = classes
        .rows
        .iter()
        .filter_map(|l| Some((l.get(1)?.as_str()?.to_string(), matches!(l.first(), Some(CypherValue::Bool(true))))))
        .collect();
    let uuids: Vec<String> = vide_de.keys().cloned().collect();
    let mut lignes = Vec::new();
    for paquet in uuids.chunks(500) {
        let liste = CypherValue::List(paquet.iter().map(|u| CypherValue::String(u.clone())).collect());
        let r = conn
            .execute_with_params("UNWIND $uuids AS u MATCH (s:`Scope` {_uuid: u}) RETURN u, s.`content`, s.`docstring`", &[QueryParam::new("uuids", liste)])
            .unwrap_or_else(|e| panic!("{e}"));
        lignes.extend(r.rows);
    }
    // [scopes, grave, guillemet, barre, content vide, longueur totale, docstring non vide]
    let mut g = [[0u64; 7]; 2];
    for l in &lignes {
        let k = l.first().and_then(|v| v.as_str()).and_then(|u| vide_de.get(u)).copied().unwrap_or(false) as usize;
        let c = l.get(1).and_then(|v| v.as_str()).unwrap_or("");
        let d = l.get(2).and_then(|v| v.as_str()).unwrap_or("");
        g[k][0] += 1;
        g[k][1] += c.contains('`') as u64;
        g[k][2] += c.contains('"') as u64;
        g[k][3] += c.contains('\\') as u64;
        g[k][4] += c.is_empty() as u64;
        g[k][5] += c.chars().count() as u64;
        g[k][6] += (!d.is_empty()) as u64;
    }
    for (k, nom) in [(0, "pleins    "), (1, "tout vides")] {
        let n = g[k][0].max(1) as f64;
        eprintln!(
            "[texte] {nom} : {} scopes ; accent grave {:.1} % ; guillemet {:.1} % ; barre inverse {:.1} % ; content vide {:.1} % ; longueur moyenne {:.0} ; docstring {:.1} %",
            g[k][0],
            100.0 * g[k][1] as f64 / n,
            100.0 * g[k][2] as f64 / n,
            100.0 * g[k][3] as f64 / n,
            100.0 * g[k][4] as f64 / n,
            g[k][5] as f64 / n,
            100.0 * g[k][6] as f64 / n
        );
    }
}

/// **Deux lectures pour départager les chemins** (relecture du proto) : chez
/// les morceaux vides et chez les pleins, le marqueur dense est-il égal à
/// `_text_hash` ; et parmi les scopes tout vides, combien ont un autre scope de
/// même clé (`name`, `parent_name`, fichier).
#[test]
#[ignore]
fn deux_lectures_pour_departager_les_chemins() {
    use rag3weaver::connection::CypherValue;
    use std::collections::HashMap;
    let Some(base) = base() else { return };
    let conn = Rag3dbConnection::new(&base).expect("ouvrir la copie");
    let vide = "(n.`_text` IS NULL OR n.`_text` = '')";
    let r = conn
        .execute(&format!(
            "MATCH (n:`Scope_Chunk`) RETURN {vide} AS v, count(n), \
             count(CASE WHEN n.`_embed_hash__granite_278m` = n.`_text_hash` THEN 1 END), \
             count(CASE WHEN n.`_embed_hash` = n.`_text_hash` THEN 1 END), \
             count(CASE WHEN n.`_sparse_hash` = n.`_text_hash` THEN 1 END) ORDER BY v"
        ))
        .unwrap_or_else(|e| panic!("{e}"));
    for l in &r.rows {
        let g = if matches!(l.first(), Some(CypherValue::Bool(true))) { "vides " } else { "pleins" };
        let n = l[1].as_i64().unwrap_or(0).max(1) as f64;
        let p = |i: usize| 100.0 * l[i].as_i64().unwrap_or(0) as f64 / n;
        eprintln!(
            "[chemins] {g} : {:?} morceaux ; _embed_hash__granite_278m = _text_hash {:.1} % ; _embed_hash = _text_hash {:.1} % ; _sparse_hash = _text_hash {:.1} %",
            l.get(1), p(2), p(3), p(4)
        );
    }
    let classes = conn
        .execute(&format!(
            "MATCH (n:`Scope_Chunk`)-[:`Scope_CHUNKED_FROM`]->(s:`Scope`) \
             WITH s, count(CASE WHEN {vide} THEN 1 END) AS v, count(n) AS t \
             RETURN s.`_uuid`, v = t AS tout_vide, s.`name`, s.`parent_name`, s.`file_path`"
        ))
        .unwrap_or_else(|e| panic!("{e}"));
    let texte = |v: Option<&CypherValue>| v.and_then(|x| x.as_str()).unwrap_or("").to_string();
    let mut par_cle: HashMap<(String, String, String), usize> = HashMap::new();
    for l in &classes.rows {
        *par_cle.entry((texte(l.get(2)), texte(l.get(3)), texte(l.get(4)))).or_default() += 1;
    }
    // Aussi parmi tous les scopes, même ceux sans morceau.
    let (mut vides, mut vides_doubles, mut pleins, mut pleins_doubles) = (0u64, 0u64, 0u64, 0u64);
    for l in &classes.rows {
        let double = par_cle[&(texte(l.get(2)), texte(l.get(3)), texte(l.get(4)))] > 1;
        if matches!(l.get(1), Some(CypherValue::Bool(true))) {
            vides += 1;
            vides_doubles += double as u64;
        } else {
            pleins += 1;
            pleins_doubles += double as u64;
        }
    }
    eprintln!(
        "[chemins] scopes tout vides : {vides}, dont avec un autre scope de même clé (name, parent_name, fichier) : {vides_doubles} ({:.1} %)",
        100.0 * vides_doubles as f64 / vides.max(1) as f64
    );
    eprintln!(
        "[chemins] autres scopes : {pleins}, dont avec un autre scope de même clé : {pleins_doubles} ({:.1} %)",
        100.0 * pleins_doubles as f64 / pleins.max(1) as f64
    );
}

/// **Les morceaux vides sont-ils alignés sur les groupes de nœuds du moteur**
/// (2 048 lignes) ? Pour chaque groupe `offset(id(n)) / 2048` de Scope_Chunk :
/// ses vides et ses lignes. Des groupes entiers vides désigneraient le moteur
/// (une colonne chaîne perdue par groupe) ; rien d'aligné, un autre bâti.
#[test]
#[ignore]
fn les_morceaux_vides_et_les_groupes_de_noeuds() {
    let Some(base) = base() else { return };
    let conn = Rag3dbConnection::new(&base).expect("ouvrir la copie");
    let r = conn
        .execute(
            "MATCH (n:`Scope_Chunk`) \
             RETURN offset(id(n)) / 2048 AS g, \
                    count(CASE WHEN n.`_text` IS NULL OR n.`_text` = '' THEN 1 END), count(n) ORDER BY g",
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let (mut tout_vides, mut mêlés, mut sans, mut vides_dans_mêlés) = (0, 0, 0, 0i64);
    let mut lignes = Vec::new();
    for l in &r.rows {
        let (g, v, t) = (l[0].as_i64().unwrap_or(-1), l[1].as_i64().unwrap_or(0), l[2].as_i64().unwrap_or(0));
        if v == 0 {
            sans += 1;
        } else if v == t {
            tout_vides += 1;
        } else {
            mêlés += 1;
            vides_dans_mêlés += v;
        }
        lignes.push(format!("{g}:{v}/{t}"));
    }
    eprintln!("[groupes] {} groupes de 2 048 : tout vides {tout_vides}, mêlés {mêlés} ({vides_dans_mêlés} vides dedans), sans vide {sans}", r.rows.len());
    eprintln!("[groupes] par groupe (offset/2048 : vides/lignes) : {}", lignes.join(" "));
}

/// **Le saut en deux temps, sur usages** (page 06, la règle des 100 ms) : sur
/// la même copie et le même catalogue, `usages` en un temps puis en deux
/// (`Catalog::set_two_step_hops`), dix appels chacun ; le rendu (Markdown,
/// lignes triées) doit être le même, et la médiane dit le gain.
#[test]
#[ignore]
fn usages_en_un_temps_et_en_deux() {
    use rag3weaver::dataflow::usage_nodes::{declarations_of, usages_of, DeclarationsConfig, UsagesConfig};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
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
        edge_mark: rag3weaver::dataflow::graph_walk::EdgeMark { field: "resolution".into(), guessed: vec!["nom".into(), "dossier".into()], doubtful: vec!["ambigu".into()] },
        qualifier_field: Some("parent_name".into()),
    };
    let decl = DeclarationsConfig { relation: "HAS_PARENT".into(), field: "declarations".into() };
    for (nom, chemin) in [("update", "src/storage/table/node_table"), ("update", ""), ("getTableID", "")] {
        let mut resultats = Vec::new();
        for deux in [false, true] {
            catalog.set_two_step_hops(Some(deux));
            let appel = || {
                let r = usages_of(&catalog, &cfg, nom, chemin).unwrap();
                let d = declarations_of(&catalog, &cfg, &decl, nom, &r.definitions).unwrap();
                r.markdown_with("all", 200, &d)
            };
            let _ = appel();
            let mut temps = Vec::new();
            let mut rendu = String::new();
            for _ in 0..10 {
                let t = Instant::now();
                rendu = appel();
                temps.push(t.elapsed().as_secs_f64() * 1e3);
            }
            temps.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mut lignes: Vec<&str> = rendu.lines().collect();
            lignes.sort();
            let mut h = DefaultHasher::new();
            lignes.hash(&mut h);
            let empreinte = h.finish();
            eprintln!(
                "[deux temps] {nom} (path « {chemin} ») en {} : médiane {:.1} ms, empreinte {empreinte:016x}",
                if deux { "deux temps" } else { "un temps  " },
                temps[5]
            );
            resultats.push(empreinte);
        }
        assert_eq!(resultats[0], resultats[1], "{nom} : le rendu est le même en un temps et en deux");
    }
    catalog.set_two_step_hops(None);
}
