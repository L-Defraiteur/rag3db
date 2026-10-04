//! **La cohésion, mesurée** — un candidat relié à d'autres candidats dans le
//! graphe du code remonte-t-il la bonne réponse, ou une grappe de tests et de
//! code généré qui se citent entre eux ?
//!
//! Un graphe hybride (BM25 + vecteur → fusion), la cohésion (`CohesionNode`)
//! branchée en `boost` de la fusion avec le poids W. W = 0 est le contrôle :
//! la même fusion, multipliée par 1 — la base de CE harnais, pas celle du
//! banc étagé (qui appelle l'API sans graphe). Les questions, le corpus et la
//! mesure sont copiés de `e2e_banc_etage.rs` — deux binaires de test ne se
//! prêtent rien.
//!
//! Run with: ./run_e2e.sh --test e2e_banc_cohesion
//!   RAG3WEAVER_BANC_MODELE=granite-278m   # les chiffres qui vaillent ; défaut : HashEmbedder, montage à vide
//!   RAG3WEAVER_COHESION_POIDS=0,0.2,0.5,1 # les poids joués

#![cfg(all(feature = "rag3db-native", feature = "code"))]

mod common;

use std::sync::{Arc, Mutex};

use rag3weaver::code::{default_scope_chunking, read_sources, register_code_schema};
use rag3weaver::dataflow::graph_tool::GraphTool;
use rag3weaver::dataflow::node_factories::register_builtins;
use rag3weaver::dataflow::node_registry::NodeRegistry;
use rag3weaver::dataflow::ServiceRegistry;
use rag3weaver::embedder::{Embedder, HashEmbedder};
use rag3weaver::search::{Consistency, SearchOptions};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

const QUESTIONS: &[(&str, &[&str])] = &[
    // français
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
    ("fusionner les résultats de plusieurs signaux de recherche", &["fuse_signals"]),
    ("construire la requête BM25 à partir du texte tapé par l'utilisateur", &["build_bm25_query"]),
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
    // anglais
    ("split a list of texts into batches under a character budget", &["budget_batches", "stable_batches"]),
    ("load burnpack weights with a precision adapter", &["charger_burnpack"]),
    ("start the embedding daemon and listen on an address", &["servir"]),
    ("attach to a running daemon or spawn one", &["assurer"]),
    ("how are the results from several search signals merged", &["fuse_signals"]),
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

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}
fn manifest() -> String {
    std::env::var("CARGO_MANIFEST_DIR").unwrap()
}
fn embarqueur() -> (Arc<dyn Embedder>, String) {
    let voulu = std::env::var("RAG3WEAVER_BANC_MODELE").unwrap_or_default();
    match voulu.as_str() {
        #[cfg(feature = "burn-embedder")]
        "granite-278m" => (common::burn::GRANITE_278M.clone(), "granite-278m".into()),
        #[cfg(feature = "burn-embedder")]
        "granite-107m" => (common::burn::GRANITE_107M.clone(), "granite-107m".into()),
        // BGE-M3 porte le creux : c'est le modèle de la mesure de fusion
        // avec le signal sparse (lot 2, demande de Lucie du 1er octobre).
        #[cfg(feature = "burn-embedder")]
        "bge-m3" => (common::burn::BGE_M3.clone() as Arc<dyn Embedder>, "bge-m3".into()),
        "" => (Arc::new(HashEmbedder::new(64)), "HashEmbedder (montage à vide)".into()),
        autre => panic!("RAG3WEAVER_BANC_MODELE={autre} : granite-278m, granite-107m, bge-m3, ou rien (HashEmbedder)"),
    }
}
fn base(embedder: Arc<dyn Embedder>, nom: &str) -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).expect("charger l'extension vecteur");
    let config = CatalogConfig { name: Some(nom.into()), embedding_dim: embedder.dim(), ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(embedder), config);
    catalog.initialize().unwrap();
    catalog
}

/// Une liste de noms classés → MRR, R@1, R@5 sur les questions (45 jusqu'au 2 octobre 2026 ; 43 depuis le retrait
/// d'`explore_bfs`, dont les deux questions n'avaient plus de cible).
#[derive(Default)]
struct Mesure {
    mrr: f64,
    r1: usize,
    r5: usize,
}
impl Mesure {
    fn noter(&mut self, classes: &[String], attendus: &[&str]) {
        if let Some(i) = classes.iter().position(|n| attendus.contains(&n.as_str())) {
            self.mrr += 1.0 / (i as f64 + 1.0);
            self.r1 += usize::from(i == 0);
            self.r5 += usize::from(i < 5);
        }
    }
    fn ligne(&self, etiquette: &str) -> String {
        self.ligne_sur(etiquette, QUESTIONS.len())
    }
    /// La même ligne, sur un autre jeu : la ligne I (identifiants) n'a pas
    /// le dénominateur des 43 questions.
    fn ligne_sur(&self, etiquette: &str, n: usize) -> String {
        format!("| {etiquette} | {:.3} | {} | {} |", self.mrr / n as f64, self.r1, self.r5)
    }
}

/// Le graphe joué : celui de `search_workspace.mmd` en mode index, et la
/// cohésion des candidats des deux signaux, en `boost` de la fusion.
const GRAPHE: &str = r#"%% tool: banc_cohesion
%% description: Banc : recherche hybride, la cohésion en boost de la fusion au poids $w.
%% param: query string! -- La question.
%% param: options json! -- SearchOptions.
%% param: w string = "0" -- Le poids de la cohésion.
%% result: resolve.results

graph LR
    source["SearchSourceNode(target_name=Scope, query=$query, options=$options, mode=indexed)"]
    bm25["BM25SearchNode"]
    vector["VectorSearchNode"]
    cohesion["CohesionNode(entity=Scope, relations='CONSUMES|INHERITS_FROM|IMPLEMENTS', max_hops=2, sources=40)"]
    fuse["FuseResultsNode(weights='bm25:0.5,vector:0.5,cohesion:$w', boost='cohesion')"]
    paginate["PaginateNode"]
    resolve["ResolveParentNode"]
    source -->|query| bm25
    source -->|query| vector
    source -->|query| fuse
    source -->|query| paginate
    source -->|query| resolve
    bm25 -->|results:bm25| fuse
    vector -->|results:vector| fuse
    bm25 -->|results| cohesion
    vector -->|results| cohesion
    cohesion -->|results:signals| fuse
    fuse -->|results| paginate
    paginate -->|results| resolve
"#;

fn noms(rendu: &str) -> Vec<String> {
    let v: serde_json::Value = serde_json::from_str(rendu).unwrap_or(serde_json::Value::Null);
    v.as_array()
        .map(|l| l.iter().filter_map(|r| r.pointer("/data/name").and_then(|n| n.as_str()).map(String::from)).collect())
        .unwrap_or_default()
}

#[test]
#[ignore]
fn banc_cohesion() {
    let (embedder, modele) = embarqueur();
    let reel = Arc::new(Mutex::new(base(embedder, "cohesion")));
    register_code_schema(&mut reel.lock().unwrap(), default_scope_chunking()).unwrap();
    let racine = manifest();
    let sources: Vec<(String, String)> = read_sources(&format!("{racine}/src")).expect("lire src/").into_iter().map(|(rel, c)| (format!("src/{rel}"), c)).collect();
    let rapport = reel.lock().unwrap().ingest_code(&rag3weaver::code::analyze(&racine, sources)).expect("ingérer src/");
    eprintln!("[cohésion] modèle {modele} : {} scopes", rapport.scopes);

    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(GRAPHE).unwrap().bind(&registry).unwrap();
    let options = serde_json::to_value(SearchOptions { limit: 10, consistency: Consistency::Immediate, ..Default::default() }).unwrap();
    let poids: Vec<String> = std::env::var("RAG3WEAVER_COHESION_POIDS").unwrap_or_else(|_| "0,0.2,0.5,1".into()).split(',').map(|s| s.trim().to_string()).collect();

    let mut lignes = Vec::new();
    let mut classements: Vec<Vec<Vec<String>>> = Vec::new();
    for w in &poids {
        let mut m = Mesure::default();
        let mut par_question = Vec::new();
        let t = std::time::Instant::now();
        for (q, attendus) in QUESTIONS {
            let mut services = ServiceRegistry::new();
            reel.lock().unwrap().register_search_services(&mut services);
            services.register("catalog", reel.clone());
            let rendu = tool.execute(&registry, Arc::new(services), &serde_json::json!({ "query": q, "options": options, "w": w })).expect("recherche");
            let n = noms(&rendu);
            m.noter(&n, attendus);
            par_question.push(n);
        }
        lignes.push(format!("{} — {} ms par question", m.ligne(&format!("cohésion W = {w}")), t.elapsed().as_millis() / QUESTIONS.len() as u128));
        classements.push(par_question);
    }
    eprintln!("\n| ligne | MRR | R@1 | R@5 |\n|---|---|---|---|");
    for l in &lignes {
        eprintln!("{l}");
    }
    // Ce qui bouge, question par question, entre le contrôle et chaque poids.
    for (i, w) in poids.iter().enumerate().skip(1) {
        eprintln!("\n### W = {w} contre W = {}", poids[0]);
        for (k, (q, attendus)) in QUESTIONS.iter().enumerate() {
            let (a, b) = (&classements[0][k], &classements[i][k]);
            if a.iter().take(5).ne(b.iter().take(5)) {
                let rang = |l: &Vec<String>| l.iter().position(|n| attendus.contains(&n.as_str())).map(|p| (p + 1).to_string()).unwrap_or("-".into());
                eprintln!("- « {q} » attendu {attendus:?} : rang {} → {}\n    avant {:?}\n    après {:?}", rang(a), rang(b), &a[..a.len().min(5)], &b[..b.len().min(5)]);
            }
        }
    }
}
