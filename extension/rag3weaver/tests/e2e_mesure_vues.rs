//! **Mesures du modèle des vues** (M3, M4, M5 de
//! `docs/07-novembre-2026-arbre/01-ce-que-le-moteur-doit-au-modele-des-vues.md`), en Cypher brut
//! sur une **copie** d'une vraie base de code — jamais une base en service :
//!
//! - M3 : le coût d'un filtre de forme vue sur chaque recherche dense (la projection, la recherche
//!   sous masque, le retrait), contre la recherche nue ; et la résolution des offsets admis que
//!   lucivy reçoit ;
//! - M4 : le rappel des k rendus sous un masque de tâche (5, 50, 500 fichiers) contre les k exacts
//!   d'un balayage complet des morceaux admis ;
//! - M5 (`MESURE_VUES_ECRIRE=1`) : des vues qui naissent et meurent (copies des morceaux de 50
//!   fichiers, puis leur effacement), et ce que leurs lignes mortes font à la recherche.
//!
//! Aucun seuil : le témoin imprime des chiffres (`MESURE\t…`) et ne juge que ce qui empêcherait
//! de mesurer. Sans `MESURE_VUES_BASE`, il le dit et rend la main.
//!
//! Run with: MESURE_VUES_BASE=<copie>/code.rag3db ./run_e2e.sh --test e2e_mesure_vues

#![cfg(feature = "rag3db-native")]

use std::time::Instant;

use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::{DbConnection, Rag3dbConnection};

const K: usize = 10;
const LIMITE: usize = 20; // le budget du produit : (limit + offset) × 2

fn q(c: &Rag3dbConnection, cypher: &str) -> rag3weaver::connection::QueryResult {
    c.execute(cypher).unwrap_or_else(|e| panic!("{cypher} : {e}"))
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// Médiane et p95 d'une série, en ms.
fn resume(nom: &str, base: &str, v: &mut Vec<f64>) {
    if v.is_empty() {
        return;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let med = v[v.len() / 2];
    let p95 = v[((v.len() as f64 * 0.95) as usize).min(v.len() - 1)];
    println!("MESURE\t{base}\t{nom}\tn={}\tmédiane={med:.2} ms\tp95={p95:.2} ms\tmin={:.2}\tmax={:.2}", v.len(), v[0], v[v.len() - 1]);
}

/// Un tirage déterministe (LCG) de `n` éléments distincts.
fn tirer<T: Clone>(v: &[T], n: usize, graine: u64) -> Vec<T> {
    let mut s = graine;
    let mut idx: Vec<usize> = (0..v.len()).collect();
    let mut out = Vec::new();
    for i in 0..n.min(v.len()) {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = i + (s >> 33) as usize % (idx.len() - i);
        idx.swap(i, j);
        out.push(v[idx[i]].clone());
    }
    out
}

fn liste_cypher(chemins: &[String]) -> String {
    let items: Vec<String> = chemins.iter().map(|p| format!("'{}'", p.replace('\\', "\\\\").replace('\'', "\\'"))).collect();
    format!("[{}]", items.join(", "))
}

fn vecteur(v: &CypherValue) -> Vec<f32> {
    match v {
        CypherValue::List(l) => l.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect(),
        autre => panic!("un vecteur attendu, pas {autre:?}"),
    }
}

fn distance_cosinus(a: &[f32], b: &[f32]) -> f32 {
    let (mut ab, mut aa, mut bb) = (0f32, 0f32, 0f32);
    for (x, y) in a.iter().zip(b) {
        ab += x * y;
        aa += x * x;
        bb += y * y;
    }
    1.0 - ab / (aa.sqrt() * bb.sqrt()).max(1e-12)
}

struct Base {
    nom: String,
    c: Rag3dbConnection,
    index: String,
    colonne: String,
}

impl Base {
    /// La recherche dense, nue ou sous la projection du produit (DROP, PROJECT, QUERY, DROP) ;
    /// rend (uuids, ms de la projection, ms de la recherche, ms des deux retraits).
    fn dense(&self, emb: &[f32], filtre: Option<&str>, limite: usize) -> (Vec<String>, f64, f64, f64) {
        let p = [QueryParam { name: "embedding".into(), value: CypherValue::List(emb.iter().map(|&f| CypherValue::Float(f as f64)).collect()) }];
        let (cible, mut t_proj, mut t_drop) = ("Scope_Chunk".to_string(), 0.0, 0.0);
        let cible = if let Some(w) = filtre {
            let t = Instant::now();
            let _ = self.c.execute("CALL DROP_PROJECTED_GRAPH('_vf_mesure', skip_if_not_exists := true)");
            t_drop += ms(t);
            let cy = format!("MATCH (n:Scope_Chunk)-[:Scope_CHUNKED_FROM]->(p:Scope) WHERE {w} RETURN n").replace('\\', "\\\\").replace('\'', "\\'");
            let t = Instant::now();
            q(&self.c, &format!("CALL PROJECT_GRAPH_CYPHER('_vf_mesure', '{cy}')"));
            t_proj = ms(t);
            "_vf_mesure".to_string()
        } else {
            cible
        };
        let t = Instant::now();
        let r = self
            .c
            .execute_with_params(&format!("CALL QUERY_VECTOR_INDEX('{cible}', '{}', $embedding, {limite}) RETURN node._uuid, distance", self.index), &p)
            .unwrap_or_else(|e| panic!("QUERY_VECTOR_INDEX sur {cible} : {e}"));
        let t_q = ms(t);
        if filtre.is_some() {
            let t = Instant::now();
            let _ = self.c.execute("CALL DROP_PROJECTED_GRAPH('_vf_mesure', skip_if_not_exists := true)");
            t_drop += ms(t);
        }
        (r.rows.iter().filter_map(|l| l.first().and_then(|v| v.as_str()).map(str::to_string)).collect(), t_proj, t_q, t_drop)
    }

    fn compte(&self) -> (i64, i64) {
        let r = q(&self.c, "MATCH (n:Scope_Chunk) RETURN count(*), max(OFFSET(id(n)))");
        (r.rows[0][0].as_i64().unwrap_or(-1), r.rows[0][1].as_i64().unwrap_or(-1))
    }
}

/// M3 sur un jeu de requêtes : nu, vue (le principal moins 50 chemins), tâche (5 chemins).
fn m3(b: &Base, requetes: &[Vec<f32>], vue: &[String], tache: &[String], etiquette: &str) {
    let f_vue = format!("NOT p.file_path IN {}", liste_cypher(vue));
    let f_tache = format!("p.file_path IN {}", liste_cypher(tache));
    let (mut nu, mut v_proj, mut v_q, mut v_tot, mut t_proj, mut t_q, mut t_tot) = (vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    for e in requetes {
        let t = Instant::now();
        b.dense(e, None, LIMITE);
        nu.push(ms(t));
        let t = Instant::now();
        let (_, p, qq, _) = b.dense(e, Some(&f_vue), LIMITE);
        v_tot.push(ms(t));
        v_proj.push(p);
        v_q.push(qq);
        let t = Instant::now();
        let (_, p, qq, _) = b.dense(e, Some(&f_tache), LIMITE);
        t_tot.push(ms(t));
        t_proj.push(p);
        t_q.push(qq);
    }
    let n = &b.nom;
    resume(&format!("{etiquette} dense nu"), n, &mut nu);
    resume(&format!("{etiquette} dense vue (−50 chemins) total"), n, &mut v_tot);
    resume(&format!("{etiquette} dense vue projection"), n, &mut v_proj);
    resume(&format!("{etiquette} dense vue recherche sous masque"), n, &mut v_q);
    resume(&format!("{etiquette} dense tâche (5 chemins) total"), n, &mut t_tot);
    resume(&format!("{etiquette} dense tâche projection"), n, &mut t_proj);
    resume(&format!("{etiquette} dense tâche recherche sous masque"), n, &mut t_q);
    // La résolution des offsets admis que lucivy reçoit (BM25), sur le parent.
    let mut res = vec![];
    let mut admis = 0;
    for _ in 0..requetes.len().min(10) {
        let t = Instant::now();
        let r = q(&b.c, &format!("MATCH (p:Scope) WHERE {f_vue} RETURN OFFSET(id(p))"));
        res.push(ms(t));
        admis = r.rows.len();
    }
    resume(&format!("{etiquette} lucivy : offsets admis d'une vue ({admis} scopes)"), n, &mut res);
}

#[test]
#[ignore]
fn mesures_du_modele_des_vues() {
    let Ok(chemin) = std::env::var("MESURE_VUES_BASE") else {
        eprintln!("non mesuré : MESURE_VUES_BASE ne désigne aucune copie de base");
        return;
    };
    let ecrire = std::env::var("MESURE_VUES_ECRIRE").as_deref() == Ok("1");
    // En écriture même pour M3 et M4 : `PROJECT_GRAPH_CYPHER` est refusé sur une base ouverte en
    // lecture seule (« Cannot execute write operations in a read-only database! », 7 nov., la
    // première passe) — c'est une copie, rien ne s'y écrit hors de M5.
    let c = Rag3dbConnection::new(&chemin).expect("la copie s'ouvre");
    c.execute(&format!("LOAD EXTENSION {}", rag3weaver::connection::cypher_path_literal(std::path::Path::new(&crate::common::vector_extension()))))
        .expect("l'extension vector");
    let nom = std::path::Path::new(&chemin).file_stem().unwrap().to_string_lossy().into_owned();

    // L'index du modèle, par SHOW_INDEXES.
    let idx = q(&c, "CALL SHOW_INDEXES() RETURN *");
    for l in &idx.rows {
        println!("INDEX\t{nom}\t{l:?}");
    }
    let ligne = idx.rows.iter().find(|l| l.first().and_then(|v| v.as_str()) == Some("Scope_Chunk") && l.iter().any(|v| v.as_str().is_some_and(|s| s.to_uppercase().contains("HNSW"))))
        .expect("un index HNSW sur Scope_Chunk");
    let index = ligne[1].as_str().unwrap().to_string();
    let colonne = ligne.iter().find_map(|v| match v {
        CypherValue::List(l) => l.first().and_then(|x| x.as_str()).map(str::to_string),
        CypherValue::String(s) if s.starts_with("embedding") => Some(s.clone()),
        _ => None,
    }).expect("la colonne de l'index");
    let b = Base { nom: nom.clone(), c, index, colonne };
    let (morceaux, max_off) = b.compte();
    let scopes = q(&b.c, "MATCH (p:Scope) RETURN count(*)").rows[0][0].as_i64().unwrap();
    let chemins: Vec<String> = q(&b.c, "MATCH (p:Scope) RETURN DISTINCT p.file_path").rows.iter().filter_map(|l| l[0].as_str().map(str::to_string)).collect();
    println!("MESURE\t{nom}\ttaille\tmorceaux={morceaux}\toffset max={max_off}\tscopes={scopes}\tfichiers={}\tindex={}\tcolonne={}", chemins.len(), b.index, b.colonne);

    // Trente vecteurs de requête pris dans la base, répartis.
    let pas = (morceaux / 30).max(1);
    let requetes: Vec<Vec<f32>> = (0..30)
        .filter_map(|i| {
            let r = q(&b.c, &format!("MATCH (n:Scope_Chunk) WHERE n.{} IS NOT NULL RETURN n.{} SKIP {} LIMIT 1", b.colonne, b.colonne, i * pas));
            r.rows.first().map(|l| vecteur(&l[0]))
        })
        .collect();
    assert!(requetes.len() >= 10, "au moins dix vecteurs de requête ({})", requetes.len());
    // Un premier tour à froid, non compté.
    let vue = tirer(&chemins, 50, 7);
    let tache = tirer(&chemins, 5, 11);
    for e in requetes.iter().take(3) {
        b.dense(e, None, LIMITE);
        b.dense(e, Some(&format!("NOT p.file_path IN {}", liste_cypher(&vue))), LIMITE);
    }

    // M3.
    m3(&b, &requetes, &vue, &tache, "M3");

    // M4 : le rappel sous des masques de tâche de 5, 50, 500 fichiers, et nu sur 5 000 morceaux tirés.
    for (n_fichiers, graine) in [(5usize, 11u64), (50, 13), (500, 17)] {
        let admis_chemins = tirer(&chemins, n_fichiers, graine);
        let f = format!("p.file_path IN {}", liste_cypher(&admis_chemins));
        let r = q(&b.c, &format!("MATCH (n:Scope_Chunk)-[:Scope_CHUNKED_FROM]->(p:Scope) WHERE {f} AND n.{} IS NOT NULL RETURN n._uuid, n.{}", b.colonne, b.colonne));
        let admis: Vec<(String, Vec<f32>)> = r.rows.iter().map(|l| (l[0].as_str().unwrap().to_string(), vecteur(&l[1]))).collect();
        if admis.len() < K {
            println!("MESURE\t{nom}\tM4 {n_fichiers} fichiers\tadmis={} < k : non mesuré", admis.len());
            continue;
        }
        let (mut rappels, mut rendus_moins_que_k) = (vec![], 0);
        for e in &requetes {
            let mut exact: Vec<(f32, &str)> = admis.iter().map(|(u, v)| (distance_cosinus(e, v), u.as_str())).collect();
            exact.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let vrais: std::collections::HashSet<&str> = exact.iter().take(K).map(|x| x.1).collect();
            let (rendus, ..) = b.dense(e, Some(&f), LIMITE);
            let pris: Vec<&String> = rendus.iter().take(K).collect();
            if pris.len() < K {
                rendus_moins_que_k += 1;
            }
            let bons = pris.iter().filter(|u| vrais.contains(u.as_str())).count();
            rappels.push(bons as f64 / K as f64);
        }
        rappels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let moyen = rappels.iter().sum::<f64>() / rappels.len() as f64;
        println!(
            "MESURE\t{nom}\tM4 rappel@{K} sous masque de {n_fichiers} fichiers\tadmis={} morceaux ({:.4} de la table)\tmoyen={moyen:.3}\tmin={:.2}\trequêtes rendant moins de k={rendus_moins_que_k}/{}",
            admis.len(),
            admis.len() as f64 / morceaux.max(1) as f64,
            rappels[0],
            rappels.len()
        );
    }

    // M5 : des vues qui naissent et meurent.
    if !ecrire {
        println!("MESURE\t{nom}\tM5\tnon mesuré : MESURE_VUES_ECRIRE n'est pas posé");
        return;
    }
    let colonnes: Vec<String> = q(&b.c, "CALL TABLE_INFO('Scope_Chunk') RETURN *").rows.iter().filter_map(|l| l.get(1).and_then(|v| v.as_str()).map(str::to_string)).collect();
    println!("COLONNES\t{nom}\t{colonnes:?}");
    let cycles: usize = std::env::var("MESURE_VUES_CYCLES").ok().and_then(|s| s.parse().ok()).unwrap_or(20);
    let mut nees = 0usize;
    for cycle in 1..=cycles {
        let fichiers = tirer(&chemins, 50, 100 + cycle as u64);
        let props: Vec<String> = colonnes.iter().map(|k| if k == "_uuid" { format!("_uuid: n._uuid + '-v{cycle}'") } else { format!("{k}: n.{k}") }).collect();
        let t = Instant::now();
        let r = q(&b.c, &format!(
            "MATCH (n:Scope_Chunk)-[:Scope_CHUNKED_FROM]->(p:Scope) WHERE p.file_path IN {} CREATE (m:Scope_Chunk {{{}}}) RETURN count(m)",
            liste_cypher(&fichiers), props.join(", ")
        ));
        let n_nes = r.rows[0][0].as_i64().unwrap_or(0) as usize;
        let t_creer = ms(t);
        let t = Instant::now();
        q(&b.c, &format!("MATCH (m:Scope_Chunk) WHERE m._uuid ENDS WITH '-v{cycle}' DETACH DELETE m"));
        let t_effacer = ms(t);
        let t = Instant::now();
        q(&b.c, "CHECKPOINT");
        let t_ckpt = ms(t);
        nees += n_nes;
        let (vivants, max_off) = b.compte();
        println!(
            "MESURE\t{nom}\tM5 cycle {cycle}\tnés puis effacés={n_nes}\tcréer={t_creer:.0} ms\teffacer={t_effacer:.0} ms ({:.2} ms par ligne)\tcheckpoint={t_ckpt:.0} ms\tvivants={vivants}\toffset max={max_off}\tmorts cumulés={nees}",
            if n_nes > 0 { t_effacer / n_nes as f64 } else { 0.0 }
        );
        if cycle % 5 == 0 || cycle == cycles {
            m3(&b, &requetes[..10.min(requetes.len())], &vue, &tache, &format!("M5 après {cycle} cycles ({nees} morts)"));
        }
    }
}
