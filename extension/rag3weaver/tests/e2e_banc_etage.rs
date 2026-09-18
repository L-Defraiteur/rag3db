//! **L'étage qui perd la moitié** — 0,35 de MRR par la recherche contre 0,84
//! au cosinus nu, mêmes 45 questions, même modèle. Trois mesures, une chose
//! changée à la fois, pour isoler l'étage. Conception : §9 de
//! `docs/18-septembre-2026-20h22/01-le-banc-de-ponderation-du-texte-brut.md`.
//!
//! | ligne | ce qui change |
//! |---|---|
//! | tel quel | `Catalog::search`, vecteur seul, sur `src/` ingéré par le chemin réel |
//! | M1 | **le texte** : les 66 scopes à l'aiguille du banc de qualité, un texte par fonction, un chunk par texte |
//! | M2 | **l'index** : cosinus exact contre tous les chunks au lieu du HNSW, même résolution |
//! | M3 | **la résolution** : les 20 chunks bruts du HNSW avant résolution au parent |
//!
//! Un banc mesure, il n'échoue pas. Les aiguilles, l'extraction et les
//! questions sont copiées de `e2e_banc_qualite.rs` — deux binaires de test ne
//! se prêtent rien ; si les deux divergent, c'est ici qu'on le verra.
//!
//! Run with: ./run_e2e.sh --test e2e_banc_etage
//!   RAG3WEAVER_BANC_MODELE=granite-278m   # les chiffres qui vaillent ; défaut : HashEmbedder, montage à vide

#![cfg(all(feature = "rag3db-native", feature = "code"))]

mod common;

use std::collections::HashMap;
use std::sync::Arc;

use rag3weaver::code::{default_scope_chunking, read_sources, register_code_schema, SCOPE};
use rag3weaver::config::{ChunkStrategy, ChunkingConfig, EntityConfig, FieldType, SimpleFieldDef};
use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::embedder::{Embedder, HashEmbedder};
use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

const CORPUS: &[(&str, &str)] = &[
    // embedder.rs : les lots et le rythme
    ("embedder.rs", "pub fn budget_batches("),
    ("embedder.rs", "pub fn lot_budget("),
    ("embedder.rs", "pub fn stable_batches("),
    ("embedder.rs", "pub fn stable_count("),
    ("embedder.rs", "pub fn gpu_duty("),
    ("embedder.rs", "pub fn pause_pour("),
    ("embedder.rs", "pub fn souffler("),
    ("embedder.rs", "pub fn embed_char_budget("),
    // burn_device.rs : la carte et la précision
    ("burn_device.rs", "pub fn resolve(self)"),
    ("burn_device.rs", "pub fn for_role("),
    ("burn_device.rs", "pub fn parse(s: &str)"),
    ("burn_device.rs", "pub(crate) fn charger_burnpack<"),
    ("burn_device.rs", "pub fn float_dtype_voulu("),
    ("burn_device.rs", "pub fn precision_par_defaut("),
    // daemon/embeddings.rs
    ("daemon/embeddings.rs", "pub fn servir(self"),
    ("daemon/embeddings.rs", "pub fn joindre("),
    ("daemon/embeddings.rs", "pub fn assurer("),
    ("daemon/embeddings.rs", "pub fn est_a_jour_avec("),
    ("daemon/embeddings.rs", "pub fn quitter("),
    ("daemon/embeddings.rs", "pub fn identite(&self) -> Identite"),
    // scope.rs
    ("scope.rs", "pub fn validate_id("),
    ("scope.rs", "pub fn index_name("),
    ("scope.rs", "pub fn stamp("),
    ("scope.rs", "pub fn scope_columns("),
    // search.rs
    ("search.rs", "pub fn embed_query("),
    ("search.rs", "pub fn search_vector("),
    ("search.rs", "pub fn build_bm25_query("),
    ("search.rs", "pub fn search_bm25("),
    ("search.rs", "pub fn search_sparse("),
    ("search.rs", "pub fn search_texte_natif("),
    ("search.rs", "pub fn resolve_chunk_results("),
    ("search.rs", "pub fn fuse_results("),
    ("search.rs", "pub fn fuse_signals("),
    ("search.rs", "pub fn explore_bfs("),
    // texte, hachage, identifiants
    ("jaro.rs", "pub fn jaro(a"),
    ("jaro.rs", "pub fn jaro_winkler("),
    ("jaro.rs", "pub fn meilleur_par_mot("),
    ("hash.rs", "pub fn content_hash("),
    ("uuid.rs", "pub fn hash_to_uuid("),
    ("uuid.rs", "pub fn hashsafe_uuid("),
    ("uuid.rs", "pub fn chunk_uuid("),
    ("chunker.rs", "pub fn chunk(&self"),
    // OCR
    ("ocr.rs", "pub fn sort_reading_order("),
    ("ocr.rs", "pub fn mean_confidence("),
    // code.rs
    ("code.rs", "pub fn verdict("),
    ("code.rs", "pub fn read_sources("),
    ("code.rs", "pub fn register_code_schema("),
    ("code.rs", "pub fn analyze(root"),
    ("code.rs", "pub fn source_id("),
    // gcp_auth.rs
    ("gcp_auth.rs", "pub fn token(&self)"),
    ("gcp_auth.rs", "pub fn from_env("),
    ("gcp_auth.rs", "pub fn invalidate("),
    // postures.rs
    ("postures.rs", "pub fn deadlocks("),
    ("postures.rs", "pub fn awaiting("),
    ("postures.rs", "pub fn describe_for("),
    // regime.rs
    ("regime.rs", "pub fn least_watched_card("),
    ("regime.rs", "pub fn duty(self)"),
    ("regime.rs", "pub fn carte_partagee("),
    ("regime.rs", "pub fn carte_locale("),
    ("regime.rs", "pub fn modele_agentique("),
    // divers
    ("reranker.rs", "pub fn passage_text("),
    ("template.rs", "pub fn scan("),
    ("fts_handle.rs", "pub fn fts_index_name("),
    ("fts_handle.rs", "pub fn index_document("),
    ("fts_handle.rs", "pub fn search_hits("),
    ("filter.rs", "pub fn combine_where("),
    ("filter.rs", "pub fn is_valid_identifier("),
];

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
    // anglais
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

fn nom_de(aiguille: &str) -> String {
    let apres = &aiguille[aiguille.find("fn ").unwrap() + 3..];
    apres.split(|c: char| c == '(' || c == '<').next().unwrap().to_string()
}

fn extraire(source: &str, aiguille: &str, sans_commentaires: bool) -> String {
    let lignes: Vec<&str> = source.lines().collect();
    let debut = lignes.iter().position(|l| l.contains(aiguille))
        .unwrap_or_else(|| panic!("aiguille introuvable : {aiguille}"));
    let mut premiere = debut;
    while premiere > 0 && lignes[premiere - 1].trim_start().starts_with("///") {
        premiere -= 1;
    }
    let mut prof = 0i32;
    let mut ouvert = false;
    let mut fin = debut;
    for (i, l) in lignes.iter().enumerate().skip(debut) {
        for c in l.chars() {
            match c {
                '{' => { prof += 1; ouvert = true; }
                '}' => prof -= 1,
                _ => {}
            }
        }
        fin = i;
        if ouvert && prof <= 0 { break; }
    }
    let mut texte = String::new();
    for l in &lignes[premiere..=fin] {
        let t = l.trim_start();
        if sans_commentaires && t.starts_with("//") { continue; }
        let l = if sans_commentaires { t } else { l };
        texte.push_str(l);
        texte.push('\n');
    }
    if texte.len() > CHUNK_CHARS {
        let mut coupe = CHUNK_CHARS;
        while !texte.is_char_boundary(coupe) { coupe -= 1; }
        texte.truncate(coupe);
    }
    texte
}

/// La limite de l'indexation du banc de qualité (façon ragforge).
const CHUNK_CHARS: usize = 1500;

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
        "" => (Arc::new(HashEmbedder::new(64)), "HashEmbedder (montage à vide)".into()),
        autre => panic!("RAG3WEAVER_BANC_MODELE={autre} : granite-278m, granite-107m, ou rien (HashEmbedder)"),
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

fn options_vecteur() -> SearchOptions {
    SearchOptions {
        limit: 10,
        consistency: Consistency::Immediate,
        exige: Some(rag3weaver::disponibilite::Disponibilites::DENSE),
        signals: Some(SearchSignals::VECTOR),
        ..Default::default()
    }
}

/// Une liste de noms classés → MRR, R@1, R@5 sur les 45 questions.
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
        format!("| {etiquette} | {:.3} | {} | {} |", self.mrr / QUESTIONS.len() as f64, self.r1, self.r5)
    }
}

fn noms(r: &rag3weaver::search::SearchResponse) -> Vec<String> {
    r.results
        .iter()
        .filter_map(|x| x.data.as_ref()?.get("name")?.as_str().map(str::to_string))
        .collect()
}

/// Des parents dans l'ordre d'apparition, dédoublonnés au premier rang.
fn parents_uniques(noms: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut vus = std::collections::HashSet::new();
    noms.into_iter().filter(|n| vus.insert(n.clone())).collect()
}

/// **Les quatre lignes**, vecteur seul, sur les 45 questions.
#[test]
#[ignore]
fn banc_etage_qui_perd() {
    let (embedder, modele) = embarqueur();

    // ── Tel quel : src/ par le chemin réel ──────────────────────────────
    let mut reel = base(embedder.clone(), "etage-reel");
    register_code_schema(&mut reel, default_scope_chunking()).unwrap();
    let racine = manifest();
    let sources: Vec<(String, String)> = read_sources(&format!("{racine}/src"))
        .expect("lire src/")
        .into_iter()
        .map(|(rel, c)| (format!("src/{rel}"), c))
        .collect();
    let analysis = rag3weaver::code::analyze(&racine, sources);
    let rapport = reel.ingest_code(&analysis).expect("ingérer src/");
    eprintln!("[étage] modèle {modele} · tel quel : {} scopes, {} en échec", rapport.scopes, rapport.failed);

    let mut tel_quel = Mesure::default();
    for (q, attendus) in QUESTIONS {
        let r = reel.search(SCOPE, q, options_vecteur()).expect("recherche");
        tel_quel.noter(&noms(&r), attendus);
    }

    // ── M2 : cosinus exact contre tous les chunks, même résolution ──────
    // Le vecteur de la requête est celui du catalogue ; la comparaison est
    // exhaustive au lieu du HNSW ; les 20 meilleurs chunks remontent à leur
    // parent, dédoublonnés au meilleur rang — la résolution d'aujourd'hui.
    let stockage = reel.vector_storage("Scope_Chunk").expect("stockage du modèle courant");
    let mut m2 = Mesure::default();
    for (q, attendus) in QUESTIONS {
        let (qvec, _) = reel.embarquer_la_requete(q, true, false).expect("embarquer la requête");
        let cypher = format!(
            "MATCH (c:Scope_Chunk) WHERE c.{col} IS NOT NULL \
             WITH c, array_cosine_similarity(c.{col}, $q) AS sim ORDER BY sim DESC LIMIT 20 \
             MATCH (c)-[:Scope_CHUNKED_FROM]->(p:Scope) RETURN p.name, sim ORDER BY sim DESC",
            col = stockage.column
        );
        let q_val = CypherValue::List(qvec.iter().map(|&f| CypherValue::Float(f as f64)).collect());
        let r = reel.execute_raw_with_params(&cypher, &[QueryParam::new("q", q_val)]).expect("cosinus exact");
        let classes = parents_uniques(r.rows.iter().filter_map(|l| l.first()?.as_str().map(str::to_string)));
        m2.noter(&classes, attendus);
    }

    // ── M3 : les 20 chunks bruts du HNSW, avant résolution ──────────────
    // Ce que l'index rend vraiment, et si le bon parent y est — à quel rang.
    let backend = reel.search_backend().expect("backend de recherche");
    let mut m3 = Mesure::default();
    let mut bon_parent_dans_les_20 = 0usize;
    for (q, attendus) in QUESTIONS {
        let (qvec, _) = reel.embarquer_la_requete(q, true, false).expect("embarquer la requête");
        let hits = backend
            .vector_search("Scope_Chunk", &stockage.index, &stockage.column, &qvec, 20)
            .expect("HNSW brut");
        let uuids = CypherValue::List(hits.iter().map(|h| CypherValue::String(h.uuid.clone())).collect());
        let r = reel
            .execute_raw_with_params(
                "UNWIND $uuids AS u MATCH (c:Scope_Chunk {_uuid: u})-[:Scope_CHUNKED_FROM]->(p:Scope) RETURN u, p.name",
                &[QueryParam::new("uuids", uuids)],
            )
            .expect("parents des chunks");
        let parent_de: HashMap<String, String> = r
            .rows
            .iter()
            .filter_map(|l| Some((l.first()?.as_str()?.to_string(), l.get(1)?.as_str()?.to_string())))
            .collect();
        // Dans l'ordre du HNSW, chaque chunk remplacé par son parent.
        let classes = parents_uniques(hits.iter().filter_map(|h| parent_de.get(&h.uuid).cloned()));
        if classes.iter().any(|n| attendus.contains(&n.as_str())) {
            bon_parent_dans_les_20 += 1;
        }
        m3.noter(&classes, attendus);
    }

    // ── M1 : le même texte que le cosinus nu — un texte par fonction ────
    // Les 66 scopes à l'aiguille, entité d'un seul champ de contenu, un chunk
    // par texte (Fixed, 4 000 > 1 500), vecteur seul.
    let mut nu = base(embedder, "etage-m1");
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("texte".to_string(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
    nu.register_entity(
        "Fonction",
        EntityConfig {
            fields,
            signals: SearchSignals::VECTOR,
            chunking: ChunkingConfig { max_size: 4_000, overlap: 0, strategy: ChunkStrategy::Fixed, ..Default::default() },
            ..Default::default()
        },
    )
    .expect("enregistrer Fonction");
    let src_dir = std::path::Path::new(&racine).join("src");
    let lignes: Vec<std::collections::BTreeMap<String, CypherValue>> = CORPUS
        .iter()
        .map(|(fichier, aiguille)| {
            let source = std::fs::read_to_string(src_dir.join(fichier)).unwrap_or_else(|e| panic!("{fichier} : {e}"));
            let mut d = std::collections::BTreeMap::new();
            d.insert("name".to_string(), CypherValue::String(nom_de(aiguille)));
            d.insert("texte".to_string(), CypherValue::String(extraire(&source, aiguille, false)));
            d
        })
        .collect();
    let r = nu.ingest_entities("Fonction", lignes).expect("ingérer les 66 scopes");
    eprintln!("[étage] M1 : {} scopes à l'aiguille, {} en échec", r.processed, r.failed);
    let mut m1 = Mesure::default();
    for (q, attendus) in QUESTIONS {
        let r = nu.search("Fonction", q, options_vecteur()).expect("recherche M1");
        m1.noter(&noms(&r), attendus);
    }

    // ── Le tableau ──────────────────────────────────────────────────────
    eprintln!("\n## L'étage qui perd — {modele}, vecteur seul, {} questions\n", QUESTIONS.len());
    eprintln!("| ligne | MRR | R@1 | R@5 |");
    eprintln!("|---|---:|---:|---:|");
    eprintln!("{}", tel_quel.ligne("tel quel — Catalog::search sur src/"));
    eprintln!("{}", m1.ligne("M1 — même texte que le cosinus nu, un chunk par fonction"));
    eprintln!("{}", m2.ligne("M2 — cosinus exact au lieu du HNSW, même résolution"));
    eprintln!("{}", m3.ligne("M3 — les 20 chunks bruts du HNSW, avant résolution"));
    eprintln!("\nM3 : le bon parent est dans les 20 chunks bruts pour {bon_parent_dans_les_20}/{} questions.", QUESTIONS.len());

    assert!(rapport.scopes > 0, "rien n'a été indexé");
}
