//! **Le banc de pondération du texte brut** — deux bancs, un tableau, aucun
//! seuil qui casse : un banc mesure, il n'échoue pas.
//!
//! Depuis le 30 août 2026, un fichier sans grammaire entre dans l'index comme
//! un scope `texte_brut`. Du texte libre est avantagé par BM25 comme par le
//! vecteur, et nos questions sont en prose : la recherche du code rend-elle
//! encore les mêmes fonctions en tête ? Conception :
//! `docs/18-septembre-2026-20h22/01-le-banc-de-ponderation-du-texte-brut.md`.
//!
//! - **Banc A** — la recherche ne recule pas. Un seul index de `src/` et d'un
//!   dossier de prose française ; deux lectures, `sans` (le filtre
//!   `scope_type != 'texte_brut'` reconstitue la recherche d'avant) et `avec` ;
//!   trois signaux ; MRR, R@1, R@5 sur les 45 questions du banc de qualité,
//!   et le nombre de fois où un texte brut sort **devant** la fonction attendue.
//! - **Banc B** — le gain existe. Cinq questions dont la réponse n'est que
//!   dans un texte brut : zéro en `sans` par construction, et c'est le point.
//!
//! Run with: ./run_e2e.sh --test e2e_banc_texte_brut
//!   RAG3WEAVER_BANC_MODELE=granite-278m   # les chiffres qui vaillent ; défaut : HashEmbedder, montage à vide

#![cfg(all(feature = "rag3db-native", feature = "code"))]

mod common;

use std::sync::Arc;

use rag3weaver::code::{default_scope_chunking, read_sources, register_code_schema, SCOPE};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::{Embedder, HashEmbedder};
use rag3weaver::filter::{FilterCondition, FilterValue};
use rag3weaver::search::{Consistency, SearchOptions, SearchResponse, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

/// **Les 45 questions du banc de qualité**, avec la fonction attendue. Copiées
/// de `e2e_banc_qualite.rs` plutôt qu'importées : deux binaires de test ne se
/// prêtent rien, et les déplacer dans `common` toucherait le banc de
/// l'optimiseur. Si les deux divergent un jour, c'est ici qu'on le verra.
const QUESTIONS: &[(&str, &[&str])] = &[
    ("comment découper une liste de textes en lots selon un budget de caractères ?", &["budget_batches", "stable_batches"]),
    ("quel lot demander à un modèle qui dit combien de séquences le saturent ?", &["lot_budget"]),
    ("arrondir le nombre d'éléments d'un lot pour que les formes se répètent", &["stable_count", "stable_batches"]),
    ("laisser souffler la carte graphique entre deux lots de travail", &["souffler", "pause_pour"]),
    ("quelle carte burn choisir selon le rôle du modèle", &["for_role"]),
    ("charger les poids d'un burnpack dans la précision voulue", &["charger_burnpack"]),
    ("la précision flottante demandée par une variable d'environnement", &["float_dtype_voulu", "precision_par_defaut"]),
    ("lancer le serveur d'embeddings et écouter sur une adresse", &["servir"]),
    ("se connecter au démon, ou le démarrer s'il n'est pas là", &["assurer"]),
    ("savoir si le démon qui tourne vient du même binaire que nous", &["est_a_jour_avec"]),
    ("vérifier qu'un identifiant d'organisation ou de projet est valide", &["validate_id"]),
    ("le nom de l'index pour un scope donné", &["index_name"]),
    ("fusionner les résultats de plusieurs signaux de recherche", &["fuse_signals", "fuse_results"]),
    ("construire la requête BM25 à partir du texte tapé par l'utilisateur", &["build_bm25_query"]),
    ("parcourir le graphe en largeur à partir d'un nœud", &["explore_bfs"]),
    ("une similarité entre deux chaînes qui tolère les fautes de frappe", &["jaro_winkler", "jaro"]),
    ("hacher le contenu d'un texte pour repérer les doublons", &["content_hash"]),
    ("fabriquer l'identifiant d'un chunk à partir de celui de son parent", &["chunk_uuid"]),
    ("trier les lignes reconnues par l'OCR dans l'ordre de lecture", &["sort_reading_order"]),
    ("décider si un fichier mérite d'être indexé d'après son chemin et sa taille", &["verdict"]),
    ("lire tous les fichiers source sous une racine", &["read_sources"]),
    ("obtenir un jeton d'accès Google Cloud depuis un compte de service", &["token"]),
    ("détecter les interblocages entre des agents qui s'attendent mutuellement", &["deadlocks"]),
    ("quelle carte graphique est la moins regardée, pour y mettre le modèle", &["least_watched_card"]),
    ("le taux d'occupation accordé à la carte selon le régime", &["duty", "gpu_duty"]),
    ("split a list of texts into batches under a character budget", &["budget_batches", "stable_batches"]),
    ("load burnpack weights with a precision adapter", &["charger_burnpack"]),
    ("start the embedding daemon and listen on an address", &["servir"]),
    ("attach to a running daemon or spawn one", &["assurer"]),
    ("how are the results from several search signals merged", &["fuse_signals", "fuse_results"]),
    ("breadth-first traversal of the graph from a node", &["explore_bfs"]),
    ("fuzzy string similarity that tolerates typos", &["jaro_winkler", "jaro"]),
    ("hash text content for deduplication", &["content_hash"]),
    ("sort OCR lines in reading order", &["sort_reading_order"]),
    ("should this file be indexed given its path and size", &["verdict"]),
    ("get a Google Cloud access token from a service account", &["token"]),
    ("detect deadlocks between agents waiting on each other", &["deadlocks"]),
    ("pick the GPU the desktop uses the least", &["least_watched_card"]),
    ("register the code schema tables in the catalog", &["register_code_schema"]),
    ("the name of the full-text index for a table", &["fts_index_name"]),
    ("validate an organization or project identifier", &["validate_id"]),
    ("embed the user query before searching vectors", &["embed_query"]),
    ("round a batch size so that batch shapes repeat", &["stable_count", "stable_batches"]),
    ("which burn device for a given model role", &["for_role"]),
    ("check whether the running daemon was built from the same binary", &["est_a_jour_avec"]),
];

/// **Cinq questions dont la réponse n'est que dans un texte brut** — un
/// fragment du chemin où elle vit. Zéro en `sans` par construction.
const QUESTIONS_TEXTE: &[(&str, &str)] = &[
    ("pourquoi les grammaires tree-sitter sont figées en 0.23 et pas 0.25", "codeparsers/Cargo.toml"),
    ("comment lancer les tests e2e en forçant la reconstruction du moteur", "run_e2e.sh"),
    ("l'union des scopes d'un fichier doit couvrir le fichier entier", "01-cahier-des-charges"),
    ("un passage non compris ne doit pas se présenter comme une fonction", "02-ce-que-ca-change-en-aval"),
    ("comment on saura que la couverture des fichiers marche", "03-comment-on-saura"),
];

/// Les dossiers de prose du 30 août : ce qui est resté ici, et ce qui est
/// parti avec codeparsers le jour même — les docs 01 et 03 y vivent, et deux
/// questions du banc B les visent (trouvé à la première passe 278m : elles
/// étaient introuvables par construction).
const PROSE_DIRS: &[&str] = &["docs/30-aout-2026-06h00", "codeparsers/docs/30-aout-2026-06h00"];

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn manifest() -> String {
    std::env::var("CARGO_MANIFEST_DIR").unwrap()
}

/// L'embarqueur du banc : `HashEmbedder` pour le montage à vide, un vrai
/// modèle par `RAG3WEAVER_BANC_MODELE` pour des chiffres qui vaillent.
fn embarqueur() -> (Arc<dyn Embedder>, String) {
    let voulu = std::env::var("RAG3WEAVER_BANC_MODELE").unwrap_or_default();
    match voulu.as_str() {
        #[cfg(feature = "burn-embedder")]
        "granite-278m" => (common::burn::GRANITE_278M.clone(), "granite-278m".into()),
        #[cfg(feature = "burn-embedder")]
        "granite-107m" => (common::burn::GRANITE_107M.clone(), "granite-107m".into()),
        "" => (Arc::new(HashEmbedder::new(64)), "HashEmbedder (montage à vide)".into()),
        autre => panic!("RAG3WEAVER_BANC_MODELE={autre} : granite-278m, granite-107m, ou rien (HashEmbedder) — burn-embedder requis pour un vrai modèle"),
    }
}

/// **Le corpus** : `src/` par le chemin réel — `read_sources` applique le
/// `verdict`, donc ce que `src/` contient de non-Rust y entre en texte brut —
/// plus un dossier de prose française, pour que le banc B ait ses réponses et
/// que le banc A ait du bruit plausible. Les deux sous la même racine.
fn corpus() -> (String, Vec<(String, String)>, usize, usize) {
    let racine = manifest();
    let mut sources = Vec::new();
    for sous in std::iter::once("src").chain(PROSE_DIRS.iter().copied()) {
        for (rel, contenu) in read_sources(&format!("{racine}/{sous}")).expect("lire les sources") {
            sources.push((format!("{sous}/{rel}"), contenu));
        }
    }
    // Le `Cargo.toml` de codeparsers et `run_e2e.sh` : deux textes bruts que le
    // banc B interroge, hors de `src/`.
    for rel in ["codeparsers/Cargo.toml", "run_e2e.sh"] {
        if let Ok(c) = std::fs::read_to_string(format!("{racine}/{rel}")) {
            sources.push((rel.to_string(), c));
        }
    }
    let code = sources.iter().filter(|(p, _)| p.ends_with(".rs")).count();
    let texte = sources.len() - code;
    (racine, sources, code, texte)
}

fn setup(embedder: Arc<dyn Embedder>) -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).expect("charger l'extension vecteur");
    let config = CatalogConfig { name: Some("banc-texte-brut".into()), embedding_dim: embedder.dim(), ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(embedder), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

/// `sans` : la recherche d'avant le 30 août, reconstituée par un filtre.
fn sans_texte_brut() -> FilterCondition {
    FilterCondition::MustNot(vec![FilterCondition::Field {
        key: "scope_type".into(),
        value: FilterValue::Direct(CypherValue::String("texte_brut".into())),
    }])
}

fn options(signaux: SearchSignals, sans: bool) -> SearchOptions {
    SearchOptions {
        limit: 10,
        consistency: Consistency::Immediate,
        exige: Some(rag3weaver::disponibilite::Disponibilites::DENSE),
        signals: Some(signaux),
        filter_condition: sans.then(sans_texte_brut),
        ..Default::default()
    }
}

/// Un résultat, tel que le banc le lit : nom, genre, chemin.
fn lignes(r: &SearchResponse) -> Vec<(String, String, String)> {
    r.results
        .iter()
        .filter_map(|x| {
            let d = x.data.as_ref()?;
            let s = |k: &str| d.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
            Some((s("name"), s("scope_type"), s("file_path")))
        })
        .collect()
}

#[derive(Default)]
struct Mesure {
    mrr: f64,
    r1: usize,
    r5: usize,
    /// Un texte brut est sorti **devant** la fonction attendue.
    texte_devant: usize,
    ratees: Vec<String>,
}

fn mesurer(catalog: &mut Catalog, signaux: SearchSignals, sans: bool) -> Mesure {
    let mut m = Mesure::default();
    for (q, attendus) in QUESTIONS {
        let r = catalog.search(SCOPE, q, options(signaux, sans)).expect("recherche");
        let l = lignes(&r);
        let rang = l.iter().position(|(nom, _, _)| attendus.contains(&nom.as_str()));
        let premier_texte = l.iter().position(|(_, genre, _)| genre == "texte_brut");
        match rang {
            Some(i) => {
                m.mrr += 1.0 / (i as f64 + 1.0);
                m.r1 += usize::from(i == 0);
                m.r5 += usize::from(i < 5);
                if premier_texte.is_some_and(|t| t < i) {
                    m.texte_devant += 1;
                }
            }
            None => {
                if premier_texte.is_some_and(|t| t < 5) {
                    m.texte_devant += 1;
                }
                let tete: Vec<String> = l.iter().take(3).map(|(n, g, _)| format!("{n} ({g})")).collect();
                m.ratees.push(format!("  ✗ « {q} » → {}", tete.join(" · ")));
            }
        }
    }
    m.mrr /= QUESTIONS.len() as f64;
    m
}

/// **Banc A et banc B, en une passe**, sur le même index. Un tableau par
/// signal, `sans` et `avec` côte à côte ; les chiffres vont dans le doc avec la
/// date et le modèle.
#[test]
#[ignore]
fn banc_texte_brut() {
    let (embedder, modele) = embarqueur();
    let mut catalog = setup(embedder);
    let (racine, sources, code, texte) = corpus();
    let analysis = rag3weaver::code::analyze(&racine, sources);
    let t = std::time::Instant::now();
    let rapport = catalog.ingest_code(&analysis).expect("ingérer");
    eprintln!(
        "[banc texte brut] modèle {modele} · {code} fichiers de code + {texte} de texte · {} scopes, {} en échec · ingestion {:.1} s",
        rapport.scopes, rapport.failed, t.elapsed().as_secs_f64()
    );
    let bruts = catalog
        .execute_raw("MATCH (s:Scope) WHERE s.scope_type = 'texte_brut' RETURN count(s)")
        .ok()
        .and_then(|r| r.rows.first().and_then(|l| l.first()).and_then(|v| v.as_i64()))
        .unwrap_or(-1);
    eprintln!("[banc texte brut] {bruts} scopes texte_brut dans l'index");

    // ── Banc A ──────────────────────────────────────────────────────────
    eprintln!("\n## Banc A — la recherche ne recule pas ({} questions)\n", QUESTIONS.len());
    eprintln!("| signal | lecture | MRR | R@1 | R@5 | texte brut devant |");
    eprintln!("|---|---|---:|---:|---:|---:|");
    let mut ratees: Vec<String> = Vec::new();
    for (etiquette, signaux) in [("vecteur", SearchSignals::VECTOR), ("plein texte", SearchSignals::BM25), ("hybride", SearchSignals::HYBRID)] {
        for (lecture, sans) in [("sans", true), ("avec", false)] {
            let m = mesurer(&mut catalog, signaux, sans);
            eprintln!("| {etiquette} | {lecture} | {:.3} | {} | {} | {} |", m.mrr, m.r1, m.r5, m.texte_devant);
            if !m.ratees.is_empty() {
                ratees.push(format!("{etiquette}, {lecture}, ratées à 10 :\n{}", m.ratees.join("\n")));
            }
        }
    }
    for r in &ratees {
        eprintln!("\n{r}");
    }

    // ── Banc B ──────────────────────────────────────────────────────────
    eprintln!("\n## Banc B — le gain existe ({} questions, réponse dans un texte brut)\n", QUESTIONS_TEXTE.len());
    eprintln!("| signal | lecture | R@1 | R@5 |");
    eprintln!("|---|---|---:|---:|");
    for (etiquette, signaux) in [("vecteur", SearchSignals::VECTOR), ("plein texte", SearchSignals::BM25), ("hybride", SearchSignals::HYBRID)] {
        for (lecture, sans) in [("sans", true), ("avec", false)] {
            let (mut r1, mut r5) = (0, 0);
            let mut detail: Vec<String> = Vec::new();
            for (q, chemin) in QUESTIONS_TEXTE {
                let r = catalog.search(SCOPE, q, options(signaux, sans)).expect("recherche");
                let l = lignes(&r);
                let rang = l.iter().position(|(_, _, f)| f.contains(chemin));
                r1 += usize::from(rang == Some(0));
                r5 += usize::from(rang.is_some_and(|i| i < 5));
                if !sans {
                    let tete = l.first().map(|(n, g, f)| format!("{n} ({g}, {f})")).unwrap_or_default();
                    detail.push(format!("  {} « {q} » → {}", if rang.is_some_and(|i| i < 5) { "✓" } else { "✗" }, tete));
                }
            }
            eprintln!("| {etiquette} | {lecture} | {r1} | {r5} |");
            for d in detail {
                eprintln!("{d}");
            }
        }
    }

    // Un banc mesure, il n'échoue pas — sauf si le montage n'a rien indexé,
    // auquel cas les zéros ne diraient rien.
    assert!(rapport.scopes > 0, "rien n'a été indexé");
}
