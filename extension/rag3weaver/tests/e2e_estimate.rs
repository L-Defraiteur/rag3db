//! **Estimer ce dépôt avant de l'indexer** — le verbe `estimate`, sur le vrai
//! dépôt et un vrai embarqueur.
//!
//! Compte le dépôt entier par la politique d'ingestion, sonde le débit de
//! l'embarqueur attaché (le service de `RAG3WEAVER_EMBED_SERVICE` s'il est
//! posé, la carte d'ici sinon), et imprime ce qu'un agent lirait. N'écrit
//! rien, n'ouvre aucune base.
//!
//! Run with: ./run_e2e.sh --test e2e_estimate
#![cfg(all(feature = "rag3db-native", feature = "burn-embedder", feature = "code"))]

mod common;

use std::sync::Arc;

use rag3weaver::code_tools::WorkingTree;
use rag3weaver::embedder::Embedder;
use rag3weaver::estimate::{code_policy, estimate_here, probe_rate, working_tree_files};

fn repository_root() -> std::path::PathBuf {
    std::env::var("RAG3DB_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

/// Une soixantaine de morceaux de 1 200 caractères pris dans la crate : la
/// matière d'une ingestion.
fn samples() -> Vec<String> {
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    for name in ["catalog.rs", "code.rs", "search.rs"] {
        let text = std::fs::read_to_string(src.join(name)).expect("source de la crate");
        let chars: Vec<char> = text.chars().collect();
        out.extend(chars.chunks(1_200).take(22).map(|c| c.iter().collect::<String>()));
    }
    out
}

#[test]
#[ignore]
fn ce_depot_s_estime_avant_de_s_indexer() {
    let tree = WorkingTree::new(repository_root());
    let t = std::time::Instant::now();
    let (files, excluded) = working_tree_files(&tree).expect("liste du dépôt");
    eprintln!("[estimate] {} fichiers listés en {:?}, sans en lire un seul ; {} écartés par la source", files.len(), t.elapsed(), excluded.len());
    // Le dossier pris tel quel, ignorés compris, n'est pas le dépôt : 307 541
    // fichiers le 3 octobre 2026, dont 278 759 retenus (2,46 Go).
    assert!(files.len() < 50_000, "la source respecte les règles d'exclusion du dossier : {}", files.len());
    let samples = samples();

    let embedders: [(&str, Arc<dyn Embedder>); 2] =
        [("granite-278m", common::burn::GRANITE_278M.clone()), ("granite-107m", common::burn::GRANITE_107M.clone())];
    for (name, embedder) in embedders {
        let rate = probe_rate(embedder.as_ref(), &samples).expect("sonde");
        let estimate = estimate_here(&files, &excluded, code_policy, rate, embedder.distant());
        eprintln!(
            "[estimate] sonde {name} ({}) : {:.0} caractères/s",
            if embedder.distant() { "service" } else { "carte d'ici" },
            rate.map(|r| r.chars_per_second).unwrap_or(0.0)
        );
        eprintln!("[estimate] {}", estimate.text().replace('\n', "\n[estimate] "));

        assert!(estimate.survey.files > 1_000, "le dépôt a plus de mille fichiers retenus : {:?}", estimate.survey);
        assert!(estimate.survey.skipped_files() > 0, "et des données qu'on écarte");
        assert!(estimate.vectors_seconds.is_some(), "une sonde qui répond donne une durée");
        assert!(!estimate.model_reason.is_empty());
    }
}

/// **Cherchable par mots avant ses vecteurs** — la mesure que la proposition
/// n'avait pu que calculer. Le dépôt entier (fichiers suivis, retenus par la
/// politique) est synchronisé en exigeant seulement le plein texte ; les
/// vecteurs restent en dette, et l'avancement le dit.
///
/// Par défaut le corpus est `src/` de la crate — une à deux minutes, ce
/// qu'une batterie supporte. `RAG3WEAVER_ESTIMATE_REPO=1` prend le dépôt
/// entier : mesuré le 3 octobre 2026 (6 735 fichiers, 59,6 Mo, 121 820
/// morceaux, binaire de test non optimisé, granite-278m par le service
/// distant), **1 798 s** pour être cherchable par mots — les 1 024 premiers
/// fichiers en 19 s, puis l'insertion des relations coûte de plus en plus
/// cher à mesure que la base grossit (418 761 relations ; confié à la session
/// cœur C++).
///
/// `RAG3WEAVER_ESTIMATE_VECTORS=1` solde ensuite la dette et chronomètre :
/// c'est ce que l'estimation avait prévu, confronté à ce qui arrive. Même
/// mesure : 710 s pour 121 820 morceaux, 569 s prévus par la sonde.
#[test]
#[ignore]
fn ce_depot_est_cherchable_par_mots_avant_ses_vecteurs() {
    use rag3weaver::code::{default_scope_chunking, register_code_schema, SCOPE};
    use rag3weaver::code_sync::{sync_source, SourceSyncOptions, SourceSyncProgress};
    use rag3weaver::code_tools::Snapshot;
    use rag3weaver::disponibilite::Disponibilites as D;
    use rag3weaver::estimate::Kept;
    use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
    use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};
    use std::sync::Mutex;
    use std::time::Instant;

    let root = repository_root();
    let whole = std::env::var_os("RAG3WEAVER_ESTIMATE_REPO").is_some();
    let corpus = if whole { root.clone() } else { std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src") };
    let (listed, _) = working_tree_files(&WorkingTree::new(&corpus)).expect("liste du corpus");
    let kept: Vec<(String, String)> = listed
        .iter()
        .filter(|(p, b)| matches!(code_policy(p, *b), Kept::Yes(_)))
        .filter_map(|(p, _)| std::fs::read_to_string(corpus.join(p)).ok().map(|c| (p.clone(), c)))
        .collect();
    let chars: usize = kept.iter().map(|(_, c)| c.len()).sum();
    eprintln!("[mots] {} fichiers, {:.1} Mo à synchroniser", kept.len(), chars as f64 / 1e6);

    let embedder: Arc<dyn Embedder> = common::burn::GRANITE_278M.clone();
    let rate = probe_rate(embedder.as_ref(), &samples()).expect("sonde");
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    boxed
        .execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", root.display()))
        .expect("extension vector");
    let config = CatalogConfig { name: Some("estimate".into()), embedding_dim: embedder.dim(), ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(embedder), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();

    let options = SourceSyncOptions { batch_files: 64, exige: D::RECHERCHE_TEXTE, ..Default::default() };
    let t = Instant::now();
    let mut last = 0usize;
    let report = sync_source(&mut catalog, &Snapshot::new("rag3db", kept), &options, &mut |p: SourceSyncProgress| {
        if p.files_done >= last + 1_000 || p.files_done == p.files_total {
            last = p.files_done;
            eprintln!("[mots] {} / {} fichiers, {} scopes, {:.0} s", p.files_done, p.files_total, p.scopes_written, t.elapsed().as_secs_f64());
        }
    })
    .expect("synchronisation jusqu'au plein texte");
    let text_seconds = t.elapsed().as_secs_f64();
    eprintln!("[mots] cherchable par mots en {text_seconds:.0} s — {} fichiers, {} scopes, {} relations", report.files_ingested, report.scopes_written, report.relations);

    let progress = catalog.index_progress().expect("avancement");
    let chars_per_chunk = chars / progress.chunks().max(1);
    eprintln!("[mots] {}", progress.line(rate, chars_per_chunk));
    assert!(progress.chunks() > 1_000, "{progress:?}");
    assert_eq!(progress.dense_missing(), progress.chunks(), "aucun vecteur n'a été calculé : {progress:?}");

    let catalog = Arc::new(Mutex::new(catalog));
    let found = Catalog::rechercher(&catalog, SCOPE, "embarquer_le_retard", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        ..Default::default()
    })
    .expect("recherche par mots");
    eprintln!("[mots] « embarquer_le_retard » : {} résultats par mots, vecteurs à {} %", found.results.len(), progress.dense_percent());
    assert!(!found.results.is_empty(), "le dépôt doit être cherchable par mots avant ses vecteurs");

    if std::env::var_os("RAG3WEAVER_ESTIMATE_VECTORS").is_some() {
        let predicted = rate.map(|r| r.duration_for((progress.dense_missing() * chars_per_chunk) as u64).as_secs());
        let t = Instant::now();
        let mut done = 0usize;
        let mut next = 10_000usize;
        loop {
            let n = catalog.lock().unwrap().embarquer_le_retard(D::TOUT, 512, None).expect("rattrapage");
            if n == 0 {
                break;
            }
            done += n;
            if done >= next {
                next += 10_000;
                let p = catalog.lock().unwrap().index_progress().unwrap();
                eprintln!("[vecteurs] {:.0} s — {}", t.elapsed().as_secs_f64(), p.line(rate, chars_per_chunk));
            }
        }
        let p = catalog.lock().unwrap().index_progress().unwrap();
        eprintln!("[vecteurs] {done} morceaux embarqués en {:.0} s (prévu : {predicted:?} s) — {}", t.elapsed().as_secs_f64(), p.line(rate, chars_per_chunk));
        assert_eq!(p.dense_missing(), 0, "{p:?}");
    }
}

/// **`index` en fond, de bout en bout** : un dossier est indexé par
/// `spawn_index`, le journal dit le plein texte puis les vecteurs, finit par
/// la ligne que `wait` attend, et l'index est complet. Le débit constaté est
/// noté pour l'estimation suivante.
#[test]
#[ignore]
fn l_indexation_en_fond_ecrit_son_journal_jusqu_au_bout() {
    use rag3weaver::code::{default_scope_chunking, register_code_schema, SCOPE};
    use rag3weaver::catalog::Level;
    use rag3weaver::code_tools::FileSource;
    use rag3weaver::dataflow::index_nodes::{new_index_journal, spawn_index, DONE_LINE, FAILED_PREFIX};
    use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
    use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};
    use std::sync::Mutex;

    let corpus = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/daemon");
    let tree = WorkingTree::new(&corpus);
    let (files, excluded) = working_tree_files(&tree).expect("liste");
    let estimate = estimate_here(&files, &excluded, code_policy, None, true);

    let embedder: Arc<dyn Embedder> = common::burn::GRANITE_278M.clone();
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    boxed
        .execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", repository_root().display()))
        .expect("extension vector");
    let config = CatalogConfig { name: Some("index".into()), embedding_dim: embedder.dim(), ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(embedder), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    let before = catalog.index_state().expect("état d'un index vide");
    assert_eq!((before.text, before.vectors), (Level::Never, Level::Never), "{before:?}");
    let catalog = Arc::new(Mutex::new(catalog));

    let journal = new_index_journal().expect("journal");
    let source: Arc<dyn FileSource> = Arc::new(tree);
    let t = std::time::Instant::now();
    spawn_index(catalog.clone(), source, journal.clone(), estimate.survey.bytes).join().expect("le fil d'indexation");
    let lines: Vec<String> = std::fs::read_to_string(&journal).expect("journal").lines().map(String::from).collect();
    eprintln!("[index] {} lignes en {:.0} s :", lines.len(), t.elapsed().as_secs_f64());
    for l in lines.iter().take(3).chain(lines.iter().rev().take(4).rev()) {
        eprintln!("[index]   {l}");
    }
    assert!(!lines.iter().any(|l| l.starts_with(FAILED_PREFIX)), "{lines:?}");
    assert_eq!(lines.last().map(String::as_str), Some(DONE_LINE), "la ligne que `wait` attend");
    let ready = lines.iter().position(|l| l.starts_with("plein texte prêt :")).expect("le plein texte est annoncé prêt");
    let first_vectors = lines.iter().position(|l| l.contains("· vecteurs")).expect("puis les vecteurs");
    assert!(ready < first_vectors, "le plein texte d'abord : {lines:?}");
    assert!(lines[lines.len() - 2].contains("vecteurs prêts"), "{lines:?}");

    let progress = catalog.lock().unwrap().index_progress().expect("avancement");
    assert!(progress.complete() && progress.chunks() > 100, "{progress:?}");
    let after = catalog.lock().unwrap().index_state().expect("état");
    eprintln!("[index] état lu par la recherche : {}", serde_json::to_string(&after).unwrap());
    assert_eq!((after.text, after.vectors, after.vectors_percent), (Level::Ready, Level::Ready, 100), "{after:?}");
    let found = Catalog::rechercher(&catalog, SCOPE, "EmbedDaemon", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25 | SearchSignals::VECTOR),
        ..Default::default()
    })
    .expect("recherche");
    assert!(!found.results.is_empty());
    if progress.chunks() >= 512 {
        let rate = catalog.lock().unwrap().known_embedding_rate().expect("débit");
        eprintln!("[index] débit constaté et noté : {:?}", rate.map(|r| r.chars_per_second as u64));
        assert!(rate.is_some(), "une indexation note ce qu'elle a mesuré");
    }
    let _ = std::fs::remove_file(&journal);
}
