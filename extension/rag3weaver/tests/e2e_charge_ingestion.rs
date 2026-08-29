//! **Le banc qui mesure ce qu'une ingestion coûte à la machine.**
//!
//! Il manquait, et son absence a coûté trois manches : `e2e_burn_embedder`
//! embarque **trois documents de cent cinquante caractères**, donc le budget
//! par lot n'y mord jamais et les « rafales GPU » qu'on y mesurait étaient en
//! réalité la mise en route du modèle. Régler un bouton sur un banc qui ne le
//! sollicite pas donne un tableau parfaitement plat, et un tableau plat
//! ressemble beaucoup à « ce bouton ne sert à rien ».
//!
//! Ici on ingère **notre propre `src/`** avec le vrai BGE-M3 : des milliers de
//! scopes, des centaines de lots. C'est le seul régime où la longueur d'une
//! rafale veut dire quelque chose.
//!
//! Ce qu'on cherche à placer, ce sont deux curseurs qui ne règlent pas la même
//! chose :
//!
//! - `RAG3WEAVER_EMBED_CHAR_BUDGET` — **combien de temps d'affilée** la carte
//!   ne rend pas la main. Plancher : la taille d'un chunk (1 000 pour du code),
//!   parce qu'un élément qui dépasse le budget forme son propre lot.
//! - `RAG3WEAVER_GPU_DUTY` — **quelle fraction du temps** elle t'appartient.
//!   Écarte les rafales sans les raccourcir ; coûte du temps total,
//!   proportionnellement.
//!
//! Le test n'affirme rien sur les durées — elles dépendent de la carte. Il
//! affirme que l'ingestion **aboutit** et imprime de quoi placer les curseurs.
//!
//! Run with: ./run_e2e.sh --test e2e_charge_ingestion
#![cfg(all(feature = "rag3db-native", feature = "burn-embedder", feature = "code"))]

mod common;

use std::sync::Arc;
use std::time::Instant;

use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema};
use rag3weaver::code_tools::{FileSource, WorkingTree};
use rag3weaver::embedder::{DualEmbedder, HashEmbedder};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

/// Occupation d'une carte, en pourcentage, si le noyau la publie.
///
/// On lit `sysfs` plutôt que de lancer un outil : pas de processus à tenir, pas
/// de sortie à analyser, et l'échantillon coûte une lecture de fichier — assez
/// peu pour en prendre un entre deux lots sans fausser ce qu'on mesure.
fn occupation(carte: &str) -> Option<u32> {
    std::fs::read_to_string(format!("/sys/class/drm/{carte}/device/gpu_busy_percent"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn cartes() -> Vec<String> {
    let Ok(dir) = std::fs::read_dir("/sys/class/drm") else { return Vec::new() };
    let mut out: Vec<String> = dir
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("card") && !n.contains('-'))
        .filter(|n| occupation(n).is_some())
        .collect();
    out.sort();
    out
}

#[test]
#[ignore]
fn ingerer_notre_propre_source_avec_le_vrai_embedder() {
    let budget = std::env::var("RAG3WEAVER_EMBED_CHAR_BUDGET").unwrap_or_else(|_| "8192 (défaut)".into());
    let duty = std::env::var("RAG3WEAVER_GPU_DUTY").unwrap_or_else(|_| "100 (défaut)".into());
    eprintln!("[charge] budget par lot = {budget} · rapport cyclique = {duty} %");

    let dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("src");
    let source: Arc<dyn FileSource> = Arc::new(WorkingTree::new(&dir));

    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();

    let config = CatalogConfig { name: Some("charge".into()), embedding_dim: 1024, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(1024)), config);
    catalog.initialize().unwrap();
    let bge: Arc<dyn DualEmbedder> = common::burn::BGE_M3.clone();
    catalog.set_dual_embedder(bge);
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();

    let t = Instant::now();
    let analysis = analyze_source(source.as_ref()).unwrap();
    let analyse_ms = t.elapsed().as_millis();
    eprintln!(
        "[charge] analysé {} fichiers, {} scopes en {analyse_ms} ms",
        analysis.files.len(), analysis.scopes.len()
    );
    assert!(analysis.scopes.len() > 500, "banc trop petit : {} scopes", analysis.scopes.len());

    // L'échantillonnage se fait depuis un fil à côté : l'ingestion est
    // synchrone, et on veut voir ce qui se passe *pendant*.
    let cartes = cartes();
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let echantillons: Arc<std::sync::Mutex<Vec<(String, u32)>>> = Arc::default();
    let guetteur = {
        let (stop, echantillons, cartes) = (stop.clone(), echantillons.clone(), cartes.clone());
        std::thread::spawn(move || {
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                for c in &cartes {
                    if let Some(v) = occupation(c) {
                        echantillons.lock().unwrap().push((c.clone(), v));
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        })
    };

    let t = Instant::now();
    let report = catalog.ingest_code(&analysis).unwrap();
    let ingest_ms = t.elapsed().as_millis();

    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    guetteur.join().ok();

    eprintln!("[charge] ingéré {report:?}");
    eprintln!("[charge] ingestion : {ingest_ms} ms");

    // **La longueur d'une rafale**, carte par carte : le nombre de suites
    // consécutives au-dessus de 90 %, la plus longue et la moyenne. C'est ce
    // chiffre-là, et pas l'occupation instantanée, qui dit si un bureau
    // saccade — 100 % pendant 20 ms ne se voit pas, 100 % pendant 500 ms si.
    let tous = echantillons.lock().unwrap();
    for carte in &cartes {
        let suite: Vec<u32> = tous.iter().filter(|(c, _)| c == carte).map(|(_, v)| *v).collect();
        if suite.is_empty() {
            continue;
        }
        let (mut rafales, mut plus_longue, mut total, mut courante) = (0usize, 0usize, 0usize, 0usize);
        for v in &suite {
            if *v >= 90 {
                courante += 1;
            } else if courante > 0 {
                rafales += 1;
                total += courante;
                plus_longue = plus_longue.max(courante);
                courante = 0;
            }
        }
        if courante > 0 {
            rafales += 1;
            total += courante;
            plus_longue = plus_longue.max(courante);
        }
        let haut = suite.iter().filter(|v| **v >= 90).count();
        let mut triee = suite.clone();
        triee.sort_unstable();
        let p = |q: f64| triee[((triee.len() - 1) as f64 * q) as usize];
        let moyenne: f64 = suite.iter().map(|v| f64::from(*v)).sum::<f64>() / suite.len() as f64;
        eprintln!(
            "[charge] {carte} : {} échantillons · moyenne {moyenne:.0} % · médiane {} % · p90 {} % · p99 {} % · max {} %",
            suite.len(), p(0.5), p(0.90), p(0.99), triee[triee.len() - 1],
        );
        eprintln!(
            "[charge] {carte} : {:.1} % des échantillons ≥ 90 % · {rafales} rafales · la plus longue {} ms · moyenne {} ms",
            100.0 * haut as f64 / suite.len() as f64,
            plus_longue * 20,
            if rafales > 0 { total * 20 / rafales } else { 0 },
        );
    }

    assert_eq!(report.failed, 0, "l'ingestion ne doit rien perdre : {report:?}");
}
