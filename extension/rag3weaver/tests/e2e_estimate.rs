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

/// **Ce que la règle des fichiers générés retire de ce dépôt** : la mesure,
/// et deux gardes — les générés connus sortent, le code écrit à la main reste.
#[test]
#[ignore]
fn ce_depot_dit_ses_fichiers_generes() {
    use rag3weaver::estimate::{generated_among, Kept};
    use rag3weaver::generated::GeneratedPolicy;
    let tree = WorkingTree::new(repository_root());
    let (files, _) = working_tree_files(&tree).expect("liste du dépôt");
    let t = std::time::Instant::now();
    let generated = generated_among(&tree, &files, &GeneratedPolicy::default(), code_policy);
    let lu = t.elapsed();
    let kept: Vec<&(String, u64)> = files.iter().filter(|(p, b)| matches!(code_policy(p, *b), Kept::Yes(_))).collect();
    let mut by_reason: std::collections::BTreeMap<&str, (usize, u64)> = Default::default();
    let mut heaviest: Vec<(u64, &str)> = Vec::new();
    for (path, bytes) in &files {
        if let Some(reason) = generated.get(path) {
            let e = by_reason.entry(reason).or_default();
            e.0 += 1;
            e.1 += bytes;
            heaviest.push((*bytes, path));
        }
    }
    heaviest.sort_by(|a, b| b.cmp(a));
    let total: u64 = kept.iter().map(|(_, b)| *b).sum();
    eprintln!("[générés] {} fichiers retenus par la politique ({:.1} Mo) ; têtes lues en {lu:?}", kept.len(), total as f64 / 1e6);
    for (reason, (n, bytes)) in &by_reason {
        eprintln!("[générés] {n} fichiers, {:.1} Mo : {reason}", *bytes as f64 / 1e6);
    }
    for (bytes, path) in heaviest.iter().take(12) {
        eprintln!("[générés]   {:>5} Ko  {path}", bytes / 1024);
    }
    for expected in ["third_party/antlr4_cypher/cypher_parser.cpp", "extension/rag3weaver/generated/bge_m3_onnx.rs"] {
        assert!(generated.contains_key(expected), "{expected} est généré, et doit sortir");
    }
    for written in ["extension/rag3weaver/src/catalog.rs", "extension/rag3weaver/src/generated.rs", "src/include/binder/bound_import_database.h"] {
        assert!(!generated.contains_key(written), "{written} est écrit à la main, et doit rester");
    }
    let outside: Vec<&String> =
        generated.keys().filter(|p| !p.contains("third_party/") && !p.contains("/generated/")).collect();
    eprintln!("[générés] hors third_party et generated : {outside:?}");
}

/// **Cherchable par mots avant ses vecteurs** — la mesure que la proposition
/// n'avait pu que calculer. Le dépôt entier (fichiers suivis, retenus par la
/// politique) est synchronisé en exigeant seulement le plein texte ; les
/// vecteurs restent en dette, et l'avancement le dit.
///
/// Par défaut le corpus est `src/` de la crate — une à deux minutes, ce
/// qu'une batterie supporte. `RAG3WEAVER_ESTIMATE_REPO=1` prend le dépôt
/// entier. Mesuré le 3 octobre 2026 au soir (6 799 fichiers, 60,6 Mo,
/// 123 750 morceaux, 384 790 relations, binaire de test non optimisé,
/// granite-278m par le service distant, relations chargées en masse) :
/// **514 s** pour les mots, **10 s** pour les relations. Avant le chargement
/// en masse, la même passe prenait 1 798 s.
///
/// `RAG3WEAVER_ESTIMATE_VECTORS=1` solde ensuite la dette et chronomètre :
/// c'est ce que l'estimation avait prévu, confronté à ce qui arrive. Même
/// mesure : 692 s pour 123 750 morceaux, 651 s prévus par la sonde.
#[test]
#[ignore]
fn ce_depot_est_cherchable_par_mots_avant_ses_vecteurs() {
    use rag3weaver::code::{default_scope_chunking, register_code_schema, SCOPE};
    use rag3weaver::code_sync::{sync_source, SourceSyncOptions, SourceSyncProgress, SyncPhase};
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
    // La mémoire résidente du moment, en Mo : de quoi dire où elle monte.
    let rss = || {
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .map(|s| {
                // Le total, puis ce qui est anonyme (tas, tampon du moteur) et
                // ce qui est partagé (fichiers mmap du tmpfs, cache lucivy).
                let field = |name: &str| {
                    s.lines()
                        .find(|l| l.starts_with(name))
                        .and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse::<u64>().ok()))
                        .map(|kb| kb / 1024)
                        .unwrap_or(0)
                };
                format!("{} (anonyme {}, partagé {})", field("VmRSS:"), field("RssAnon:"), field("RssShmem:"))
            })
            .unwrap_or_default()
    };
    eprintln!("[mémoire] {} Mo — les sources lues ({:.0} Mo de texte)", rss(), chars as f64 / 1e6);

    let embedder: Arc<dyn Embedder> = common::burn::GRANITE_278M.clone();
    let reopen_embedder = embedder.clone();
    let rate = probe_rate(embedder.as_ref(), &samples()).expect("sonde");
    // `RAG3WEAVER_ESTIMATE_DB_DIR` : la base sur disque, dans ce dossier (vidé
    // d'abord) — le cas réel d'un utilisateur. Sans elle : en mémoire.
    let db_dir = std::env::var_os("RAG3WEAVER_ESTIMATE_DB_DIR").map(std::path::PathBuf::from);
    let conn = match &db_dir {
        Some(dir) => {
            let _ = std::fs::remove_dir_all(dir);
            std::fs::create_dir_all(dir).expect("dossier de la base");
            eprintln!("[mots] base sur disque : {}", dir.display());
            Rag3dbConnection::new(dir.join("estimate.rag3db")).expect("base sur disque")
        }
        None => Rag3dbConnection::in_memory().expect("base en mémoire"),
    };
    eprintln!(
        "[mots] tampon du moteur : {}",
        rag3weaver::connection::DbConnection::buffer_pool(&conn).map(rag3weaver::connection::describe_buffer_pool).unwrap_or_else(|| "non dit par la connexion".into())
    );
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    boxed
        .execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", root.display()))
        .expect("extension vector");
    // `RAG3WEAVER_ESTIMATE_AUTO_CHECKPOINT=0` : le point de reprise
    // automatique du moteur est coupé, et un seul est demandé à la fin.
    // `RAG3WEAVER_ESTIMATE_CHECKPOINT_THRESHOLD` : son seuil, en octets de
    // journal (défaut du moteur : 16 Mio).
    let auto_checkpoint = std::env::var("RAG3WEAVER_ESTIMATE_AUTO_CHECKPOINT").map(|v| v.trim() != "0").unwrap_or(true);
    if !auto_checkpoint {
        boxed.execute("CALL auto_checkpoint=false").expect("couper le point de reprise automatique");
    }
    if let Some(threshold) = std::env::var("RAG3WEAVER_ESTIMATE_CHECKPOINT_THRESHOLD").ok().and_then(|v| v.trim().parse::<u64>().ok()) {
        boxed.execute(&format!("CALL checkpoint_threshold={threshold}")).expect("seuil du point de reprise");
    }
    // `RAG3WEAVER_ESTIMATE_COPY_JOURNALISE=1` : un COPY écrit ses lignes au
    // journal au lieu de forcer son point de reprise (réglage du moteur
    // `force_checkpoint_on_copy`) — pour peser au journal un paquet réel.
    if std::env::var("RAG3WEAVER_ESTIMATE_COPY_JOURNALISE").as_deref() == Ok("1") {
        boxed.execute("CALL force_checkpoint_on_copy=false").expect("COPY journalisé");
        eprintln!("[mots] COPY journalisé (force_checkpoint_on_copy=false)");
    }
    // Depuis `ff9bad960` le COPY journalisé est le défaut du moteur :
    // `RAG3WEAVER_ESTIMATE_COPY_JOURNALISE=0` rend le COPY forcé d'avant.
    if std::env::var("RAG3WEAVER_ESTIMATE_COPY_JOURNALISE").as_deref() == Ok("0") {
        boxed.execute("CALL force_checkpoint_on_copy=true").expect("COPY forcé");
        eprintln!("[mots] COPY forcé (force_checkpoint_on_copy=true)");
    }
    eprintln!("[mots] point de reprise automatique : {auto_checkpoint} ; seuil : {:?}", std::env::var("RAG3WEAVER_ESTIMATE_CHECKPOINT_THRESHOLD").ok());
    // Les points de reprise réellement posés, comptés par la taille du
    // journal : elle ne retombe que quand le moteur le replie (le guetteur de
    // `e2e_arret_brutal`). Un échantillon toutes les 2 ms.
    let journal_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let journal_watch = db_dir.as_ref().map(|dir| {
        let (dir, stop) = (dir.clone(), journal_stop.clone());
        std::thread::spawn(move || {
            use std::sync::atomic::Ordering;
            let wal_size = || -> u64 {
                std::fs::read_dir(&dir)
                    .map(|entries| {
                        entries
                            .flatten()
                            .filter(|e| e.file_name().to_string_lossy().ends_with(".wal"))
                            .filter_map(|e| e.metadata().ok())
                            .map(|m| m.len())
                            .sum()
                    })
                    .unwrap_or(0)
            };
            let (mut previous, mut folds, mut largest, mut folded_bytes) = (0u64, 0usize, 0u64, 0u64);
            while !stop.load(Ordering::Relaxed) {
                let size = wal_size();
                if size < previous {
                    folds += 1;
                    folded_bytes += previous;
                }
                largest = largest.max(size);
                previous = size;
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            (folds, largest, folded_bytes)
        })
    });
    let config = CatalogConfig { name: Some("estimate".into()), embedding_dim: embedder.dim(), ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(embedder), config);
    // `RAG3WEAVER_ESTIMATE_FTS=localfs` ou `fichiers` (base sur disque
    // seulement) : le plein texte dans des fichiers à côté de la base,
    // `<base>.fts/`, au lieu de blobs dans la base — `FtsStorage::LocalFs`
    // (un `fsync` par fichier) ou `FtsStorage::Files` (un par génération).
    let fts_storage = || match (&db_dir, std::env::var("RAG3WEAVER_ESTIMATE_FTS").as_deref()) {
        (Some(dir), Ok("localfs")) => Some(rag3weaver::fts_handle::FtsStorage::LocalFs {
            base_path: dir.join("estimate.rag3db.fts").to_string_lossy().to_string(),
        }),
        (Some(dir), Ok("fichiers")) => Some(rag3weaver::fts_handle::FtsStorage::Files {
            base_path: dir.join("estimate.rag3db.fts").to_string_lossy().to_string(),
        }),
        _ => None,
    };
    if let Some(storage) = fts_storage() {
        eprintln!("[mots] plein texte : {storage:?}");
        catalog.set_fts_storage(storage);
    }
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();

    // `RAG3WEAVER_ESTIMATE_BATCH_FILES` : la taille des paquets (64 par défaut).
    // À 100 000, toute la source passe en un paquet — le plancher du « tout
    // d'un coup » avec le code d'aujourd'hui.
    let batch_files = std::env::var("RAG3WEAVER_ESTIMATE_BATCH_FILES").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(64);
    eprintln!("[mots] paquets de {batch_files} fichiers");
    let options = SourceSyncOptions { batch_files, exige: D::RECHERCHE_TEXTE, ..Default::default() };
    // Le pic au fil de l'eau, avec l'étape en cours quand il arrive : un
    // échantillon de la mémoire résidente toutes les 100 ms.
    let step = Arc::new(Mutex::new(String::from("avant le premier paquet")));
    let peak_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let peak_watch = {
        let (step, stop) = (step.clone(), peak_stop.clone());
        std::thread::spawn(move || {
            let mut peak = (0u64, String::new());
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                let rss = std::fs::read_to_string("/proc/self/status")
                    .ok()
                    .and_then(|s| s.lines().find(|l| l.starts_with("VmRSS:")).and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse::<u64>().ok())))
                    .unwrap_or(0);
                if rss > peak.0 {
                    peak = (rss, step.lock().unwrap().clone());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            peak
        })
    };
    let t = Instant::now();
    let mut last = 0usize;
    // Les trois temps, séparés : les mots (les paquets), les relations (le
    // chargement final, en masse), puis — plus bas — les vecteurs.
    let mut words_seconds: Option<f64> = None;
    // **La première chose cherchable** : sous la transaction par paquet, rien
    // n'est visible avant la première validation, qui tombe après
    // taille × K fichiers (K = RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION).
    let par_validation: usize = std::env::var("RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(1);
    let premier_seuil = if par_validation == 0 { usize::MAX } else { batch_files.saturating_mul(par_validation) };
    let mut premier_vu = false;
    let report = sync_source(&mut catalog, &Snapshot::new("rag3db", kept), &options, &mut |p: SourceSyncProgress| {
        *step.lock().unwrap() = match p.phase {
            SyncPhase::Nodes => format!("paquet suivant {} fichiers faits, {} liens en file", p.files_done, p.relations_pending),
            other => format!("{other:?}, {} liens à poser", p.relations_pending),
        };
        if p.phase == SyncPhase::Relations && words_seconds.is_none() {
            words_seconds = Some(t.elapsed().as_secs_f64());
            eprintln!("[mots] les mots sont là en {:.0} s ; relations : {} liens à poser", t.elapsed().as_secs_f64(), p.relations_pending);
            eprintln!("[mémoire] {} Mo — les mots posés, {} liens en file", rss(), p.relations_pending);
        }
        if p.phase == SyncPhase::Nodes && !premier_vu && (p.files_done >= premier_seuil.min(p.files_total)) {
            premier_vu = true;
            eprintln!("[mots] première chose cherchable à {:.0} s (après {} fichiers sur {})", t.elapsed().as_secs_f64(), p.files_done, p.files_total);
        }
        if p.phase == SyncPhase::Nodes && (p.files_done >= last + 1_000 || p.files_done == p.files_total) {
            last = p.files_done;
            eprintln!("[mots] {} / {} fichiers, {} scopes, {} liens en file, {:.0} s", p.files_done, p.files_total, p.scopes_written, p.relations_pending, t.elapsed().as_secs_f64());
            eprintln!("[mémoire] {} Mo — {} fichiers faits", rss(), p.files_done);
        }
    })
    .expect("synchronisation jusqu'au plein texte");
    let text_seconds = t.elapsed().as_secs_f64();
    eprintln!(
        "[mots] synchronisé en {text_seconds:.0} s — mots {:.0} s, relations {:.0} s ({:?}, chargement final {} ms) — {} fichiers, {} scopes, {} relations",
        words_seconds.unwrap_or(text_seconds),
        text_seconds - words_seconds.unwrap_or(text_seconds),
        report.relations_mode,
        report.relations_bulk_ms,
        report.files_ingested,
        report.scopes_written,
        report.relations
    );

    eprintln!("[mémoire] {} Mo — synchronisation finie", rss());
    peak_stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let (peak_kb, peak_step) = peak_watch.join().expect("guetteur du pic");
    eprintln!("[mémoire] pic au fil de l'eau : {} Mo, pendant : {peak_step}", peak_kb / 1024);
    if !auto_checkpoint {
        let t = Instant::now();
        catalog.execute_raw("CHECKPOINT").expect("point de reprise final");
        eprintln!("[journal] point de reprise final, demandé : {:.1} s", t.elapsed().as_secs_f64());
    }
    journal_stop.store(true, std::sync::atomic::Ordering::Relaxed);
    if let Some(watch) = journal_watch {
        let (folds, largest, folded_bytes) = watch.join().expect("guetteur du journal");
        eprintln!(
            "[journal] {folds} replis du journal vus (points de reprise) ; plus gros journal : {:.0} Mo ; journal replié au total : {:.0} Mo",
            largest as f64 / 1e6,
            folded_bytes as f64 / 1e6
        );
    }
    // Les relations par type : de quoi comparer deux tailles de paquet.
    match catalog.execute_raw("MATCH ()-[r]->() RETURN label(r) AS type, count(*) AS n ORDER BY type") {
        Ok(result) => {
            for row in &result.rows {
                eprintln!("[relations] {:?} : {:?}", row[0], row[1]);
            }
        }
        Err(e) => eprintln!("[relations] décompte par type impossible : {e}"),
    }
    if let Some(dir) = &db_dir {
        fn size(dir: &std::path::Path) -> u64 {
            std::fs::read_dir(dir)
                .map(|entries| {
                    entries
                        .flatten()
                        .map(|e| match e.metadata() {
                            Ok(m) if m.is_dir() => size(&e.path()),
                            Ok(m) => m.len(),
                            Err(_) => 0,
                        })
                        .sum()
                })
                .unwrap_or(0)
        }
        let fts = dir.join("estimate.rag3db.fts");
        eprintln!(
            "[mots] taille de la base sur disque : {:.0} Mo (dont dossier du plein texte {:.0} Mo)",
            size(dir) as f64 / 1e6,
            size(&fts) as f64 / 1e6
        );
    }
    for (reason, n) in &report.files_set_aside {
        eprintln!("[mots] {n} fichiers écartés : {reason}");
    }
    eprintln!("[mots] chargements en masse refusés (bulk_load_refused) : {} {:?}", report.bulk_load_refused.len(), report.bulk_load_refused);
    // Les replis d'un COPY journalisé (`71cffbc4b`) : au-delà du seuil, la
    // transaction vide son journal et repasse par le point de reprise. Inerte
    // sous le COPY forcé ; un moteur plus ancien ne connaît pas le réglage.
    match catalog.conn().execute("CALL current_setting('copy_journal_fallbacks') RETURN *") {
        Ok(r) => eprintln!("[mots] replis du COPY journalisé (copy_journal_fallbacks) : {:?}", r.rows.first().and_then(|row| row.first())),
        Err(e) => eprintln!("[mots] replis du COPY journalisé : illisibles ({e})"),
    }

    // Le pic de mémoire résidente du processus : la borne à tenir sur un
    // poste modeste, et l'inconnue d'un chargement de bout en bout.
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        if let Some(peak) = status.lines().find(|l| l.starts_with("VmHWM:")) {
            eprintln!("[mots] pic de mémoire résidente : {}", peak.trim_start_matches("VmHWM:").trim());
        }
    }
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

    // **La réouverture** : un processus qui rouvre la base et cherche. En
    // `BlobBacked`, l'index entier se recopie depuis la base ; en `LocalFs`,
    // il se lit en place.
    if let Some(dir) = &db_dir {
        drop(catalog);
        // `RAG3WEAVER_ESTIMATE_FORCER_REBATI=1` (mode fichiers) : le dossier
        // du plein texte est retiré avant la réouverture — ce que laisse un
        // dossier perdu : la réouverture le rebâtit depuis les lignes.
        if std::env::var_os("RAG3WEAVER_ESTIMATE_FORCER_REBATI").is_some() {
            let _ = std::fs::remove_dir_all(dir.join("estimate.rag3db.fts"));
            eprintln!("[réouverture] dossier du plein texte retiré : la réouverture doit le rebâtir");
        }
        let t = Instant::now();
        let conn = Rag3dbConnection::new(dir.join("estimate.rag3db")).expect("base rouverte");
        let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
        boxed
            .execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", root.display()))
            .expect("extension vector");
        let config = CatalogConfig { name: Some("estimate".into()), embedding_dim: reopen_embedder.dim(), ..Default::default() };
        let mut reopened = Catalog::new(boxed, Box::new(reopen_embedder), config);
        if let Some(storage) = fts_storage() {
            reopened.set_fts_storage(storage);
        }
        reopened.initialize().unwrap();
        register_code_schema(&mut reopened, default_scope_chunking()).unwrap();
        let opened = t.elapsed();
        // La garde des comptes, sur le dépôt entier, comme une synchronisation
        // la ferait en tête : son coût.
        let tg = Instant::now();
        let ecarts = reopened.check_fts_counts().expect("garde des comptes");
        eprintln!("[réouverture] garde des comptes du plein texte : {:.0} ms, {} écarts", tg.elapsed().as_secs_f64() * 1e3, ecarts.len());
        let reopened = Arc::new(Mutex::new(reopened));
        let found = Catalog::rechercher(&reopened, SCOPE, "embarquer_le_retard", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        .expect("recherche après réouverture");
        let etat = reopened.lock().unwrap().index_state_for(SCOPE).ok();
        eprintln!(
            "[réouverture] base rouverte en {:.1} s, première recherche par mots rendue à {:.1} s : {} résultats ; état des scopes : {:?}",
            opened.as_secs_f64(),
            t.elapsed().as_secs_f64(),
            found.results.len(),
            etat.map(|e| (e.text, e.text_percent))
        );
        // Un rebâti en fond : attendre qu'il ait fini, en relevant
        // l'avancement, puis chercher à nouveau.
        if etat.is_some_and(|e| e.text_percent.is_some()) {
            let debut = Instant::now();
            let mut vu = 0u8;
            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let e = reopened.lock().unwrap().index_state_for(SCOPE).ok();
                match e.and_then(|e| e.text_percent) {
                    Some(p) => {
                        if p >= vu + 10 {
                            vu = p;
                            eprintln!("[réouverture] rebâti en fond : {p} % à {:.0} s", debut.elapsed().as_secs_f64());
                        }
                    }
                    None => break,
                }
            }
            eprintln!("[réouverture] rebâti en fond fini en {:.1} s (vu depuis la recherche)", debut.elapsed().as_secs_f64());
            rag3weaver::ingest_profile::publish();
            let found = Catalog::rechercher(&reopened, SCOPE, "embarquer_le_retard", SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            })
            .expect("recherche après le rebâti");
            eprintln!("[réouverture] après le rebâti : {} résultats", found.results.len());
            assert!(!found.results.is_empty(), "après le rebâti, le dépôt est cherchable par mots");
        } else {
            assert!(!found.results.is_empty(), "après réouverture, le dépôt reste cherchable par mots");
        }
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
    spawn_index(catalog.clone(), source, journal.clone(), estimate.survey.bytes, Default::default()).join().expect("le fil d'indexation");
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
    // Par entité : `Scope` est prêt des deux côtés ; `Symbol`, plein texte
    // seul, est cherchable par mots, et ses vecteurs sont « non déclarés »
    // (`NotDeclared` depuis le 10 octobre 2026 : « jamais » se lisait comme une
    // panne) ; une entité inconnue est une erreur, pas un « jamais ».
    {
        use rag3weaver::code::SYMBOL;
        let guard = catalog.lock().unwrap();
        let scope = guard.index_state_for(SCOPE).expect("état de Scope");
        assert_eq!((scope.text, scope.vectors, scope.vectors_percent), (Level::Ready, Level::Ready, 100), "{scope:?}");
        let symbol = guard.index_state_for(SYMBOL).expect("état de Symbol");
        assert_eq!((symbol.text, symbol.vectors), (Level::Ready, Level::NotDeclared), "{symbol:?}");
        assert!(guard.index_state_for("Inconnue").is_err());
    }
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

/// **Le journal d'une conversation ne fait pas croire le code indexé.**
///
/// Trouvé le 3 octobre 2026 dans le chat réel : le journal de la conversation
/// s'écrit dans la même base dès le premier message ; l'état global quitte
/// `never`, et plus rien ne se replie — ni la recherche qui choisit son mode,
/// ni les outils qui refusent sans index. L'état d'une entité, lui, ne compte
/// que ses lignes.
#[test]
#[ignore]
fn le_journal_d_une_conversation_ne_fait_pas_croire_le_code_indexe() {
    use rag3weaver::catalog::Level;
    use rag3weaver::code::{default_scope_chunking, register_code_schema, SCOPE};
    use rag3weaver::connection::CypherValue;
    use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};
    use std::collections::BTreeMap;

    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    boxed
        .execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", repository_root().display()))
        .expect("extension vector");
    let config = CatalogConfig { name: Some("chat".into()), embedding_dim: 8, allow_mock_embedder: true, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(rag3weaver::embedder::HashEmbedder::new(8)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    let message: rag3weaver::config::EntityConfig = serde_json::from_value(serde_json::json!({
        "fields": {"key": {"type": "string", "isTitle": true}, "text": {"type": "string", "isContent": true}},
        "hashsafe": ["key"],
        "signals": ["bm25"]
    }))
    .expect("configuration d'une entité de journal");
    catalog.register_entity("Message", message).unwrap();

    // Rien n'est indexé : tout est « jamais », globalement et pour le code.
    assert_eq!(catalog.index_state().unwrap().text, Level::Never);
    assert_eq!(catalog.index_state_for(SCOPE).unwrap().text, Level::Never);

    // L'utilisateur parle : un message entre dans le journal.
    let row = BTreeMap::from([
        ("key".to_string(), CypherValue::String("m1".into())),
        ("text".to_string(), CypherValue::String("Où est défini le régulateur de rafale ?".into())),
    ]);
    catalog.ingest_entities("Message", vec![row]).expect("le journal s'écrit");

    // Le journal, lui, est cherchable ; le code ne l'est toujours pas.
    let journal = catalog.index_state_for("Message").unwrap();
    assert_eq!(journal.text, Level::Ready, "{journal:?}");
    let code = catalog.index_state_for(SCOPE).unwrap();
    assert_eq!((code.text, code.vectors), (Level::Never, Level::Never), "le code n'est pas indexé : {code:?}");
    eprintln!("[état] après un message : Message {:?}, Scope {:?}", journal.text, code.text);

    // Et un « jamais » noté n'est pas cru sur parole : le code ingéré par un
    // autre chemin que `index` se voit à la lecture suivante.
    let files = vec![("src/lib.rs".to_string(), "pub fn regulateur() {}\n".to_string())];
    let analysis = rag3weaver::code::analyze_source(&rag3weaver::code_tools::Snapshot::new("depot", files)).expect("analyse");
    catalog.ingest_code(&analysis).expect("ingestion directe");
    let code = catalog.index_state_for(SCOPE).unwrap();
    assert_eq!(code.text, Level::Ready, "le code ingéré directement est vu : {code:?}");
}

