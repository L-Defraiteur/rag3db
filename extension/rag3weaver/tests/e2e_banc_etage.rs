//! **L'étage qui perd la moitié** — 0,35 de MRR par la recherche contre 0,84
//! au cosinus nu, mêmes 43 questions, même modèle. Trois mesures, une chose
//! changée à la fois, pour isoler l'étage. Conception : §9 de
//! `docs/18-septembre-2026-20h22/01-le-banc-de-ponderation-du-texte-brut.md`.
//!
//! | ligne | ce qui change |
//! |---|---|
//! | tel quel | `Catalog::search`, vecteur seul, sur `src/` ingéré par le chemin réel |
//! | M1 | **le texte** : les 63 scopes à l'aiguille du banc de qualité, un texte par fonction, un chunk par texte |
//! | M2 | **l'index** : cosinus exact contre tous les chunks au lieu du HNSW, même résolution |
//! | M3 | **la résolution** : les 20 chunks bruts du HNSW avant résolution au parent |
//! | M1b | **le texte seul** : les 4 819 scopes de `src/`, chacun un texte `nom + doc + signature + corps` — même corpus que tel quel, même texte que M1 |
//! | G | **le genre** : tel quel, `scope_type` filtré sur `function | method` — ce que pèsent les scopes de fichier entier et les espaces de noms |
//!
//! **Ce que ce banc a cessé de couvrir (10 octobre 2026).** Il lit des
//! FICHIERS sous `src/`, pas des réexports. La crate `rag3weaver-ir`
//! (`41b869ba4`) a déplacé quatre de ses aiguilles hors de `src/` : le commit
//! dit vrai — « aucun site d'usage ne bouge » — mais un banc à corpus vivant
//! ne lit pas les usages, il lit l'arborescence. Les quatre sont remplacées
//! par des fonctions restées dans `src/`, chacune nommée comme remplacement à
//! sa ligne, et le banc **ne couvre donc plus** `Scope::validate_id`,
//! `Scope::index_name`, `Scope::stamp` (passées dans `ir/src/scope.rs`) ni
//! `is_valid_identifier` (passée dans `ir/src/value.rs`).
//!
//! **Et ses chiffres d'avant ne sont pas comparables à ceux d'après**, pas à
//! cause de cette réparation mais à cause du déménagement lui-même : les
//! lignes « tel quel » et M1b indexent `src/`, qui a perdu des fichiers. Deux
//! des 43 questions changent aussi de cible (celles qui visaient `validate_id`
//! et `index_name`), le compte restant à 43. Une mesure « après » se rejoue
//! donc contre un « avant » rejoué sur ce banc-ci, jamais contre un chiffre
//! d'avant le 10 octobre.
//!
//! **Quand étendre le corpus à `ir/src/`, et pas avant** (F, 10 octobre) :
//! aujourd'hui `ir` ne porte que du vocabulaire trivial pour une recherche —
//! valider un identifiant, nommer un index, estamper. Le jour où la crate
//! portera les **formes** du dialecte (`Hop` est là, `Count` et `Select`
//! suivent), ce sera du vocabulaire qu'un agent cherche vraiment, et l'étendre
//! aura du sens. À faire **en une fois** à ce moment-là, en rejouant toutes les
//! lignes de référence : une recalibration n'est pas une réparation, et elle ne
//! se fait pas au milieu d'une fusion. Rien ne doit quitter `src/` avant
//! `Select` ; `Hop` et `Count` ajoutent dans `ir` sans rien retirer d'ici.
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
use std::sync::{Arc, Mutex};

use rag3weaver::code::{default_scope_chunking, read_sources, register_code_schema, SCOPE};
use rag3weaver::config::{ChunkStrategy, ChunkingConfig, EntityConfig, FieldType, SimpleFieldDef};
use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::filter::{FilterCondition, FilterValue};
use rag3weaver::embedder::{Embedder, HashEmbedder};
use rag3weaver::search::{Consistency, FieldWeight, FusionConfig, SearchOptions, SearchSignals, SignalConfig};
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
    // scope.rs — trois aiguilles remplacées, les parties nommées en regard
    ("scope.rs", "pub fn is_scope_column("),   // remplace validate_id, passée dans ir/src/scope.rs
    ("scope.rs", "pub fn fts_filter_fields("), // remplace index_name, passée dans ir/src/scope.rs
    ("scope.rs", "pub fn scope_columns("),
    // search.rs
    ("search.rs", "pub fn embed_query("),
    ("search.rs", "pub fn search_vector("),
    ("search.rs", "pub fn build_bm25_query("),
    ("search.rs", "pub fn search_sparse("),
    ("search.rs", "pub fn search_texte_natif("),
    ("search.rs", "pub fn fuse_signals("),
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
    ("filter.rs", "pub fn parse_condition("),  // remplace scope.rs::stamp, passée dans ir/src/scope.rs
    ("filter.rs", "pub fn has_any("),          // remplace is_valid_identifier, passée dans ir/src/value.rs
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
    // remplace « vérifier qu'un identifiant … est valide » (visait validate_id, partie dans ir)
    ("savoir si une colonne est une colonne système de périmètre", &["is_scope_column"]),
    // remplace « le nom de l'index pour un scope donné » (visait index_name, partie dans ir)
    ("les champs de filtre du plein texte pour le périmètre", &["fts_filter_fields"]),
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
    // remplace « validate an organization or project identifier » (idem, en anglais)
    ("tell whether a column is a system scope column", &["is_scope_column"]),
    ("embed the user query before searching vectors", &["embed_query"]),
    ("round a batch size so that batch shapes repeat", &["stable_count", "stable_batches"]),
    ("which burn device for a given model role", &["for_role"]),
    ("check whether the running daemon was built from the same binary", &["est_a_jour_avec"]),
];

fn nom_de(aiguille: &str) -> String {
    let apres = &aiguille[aiguille.find("fn ").unwrap() + 3..];
    apres.split(|c: char| c == '(' || c == '<').next().unwrap().to_string()
}

/// **Toutes les aiguilles manquantes d'un coup, et un seul échec.**
///
/// `extraire` paniquait sur la première introuvable, donc un déménagement qui
/// en emportait quatre n'en annonçait qu'une : on corrigeait, on rejouait
/// 167 secondes, on en découvrait une autre. Quatre tours pour un seul défaut.
/// C'est la même leçon que le filet des gabarits — essayer tous les cas avant
/// de rendre — et elle vaut davantage ici, vu le prix de la passe.
///
/// Le relevé est statique : il ne lit que les fichiers, aucune base, aucun
/// embarquement. Il tourne donc avant la mesure et ne lui coûte rien.
fn verifier_les_aiguilles(src_dir: &std::path::Path) {
    let mut manquantes: Vec<String> = Vec::new();
    for (fichier, aiguille) in CORPUS {
        match std::fs::read_to_string(src_dir.join(fichier)) {
            Ok(source) => {
                if !source.lines().any(|l| l.contains(aiguille)) {
                    manquantes.push(format!("{fichier} :: {aiguille} — aiguille absente du fichier"));
                }
            }
            Err(e) => manquantes.push(format!("{fichier} — illisible ({e})")),
        }
    }
    // **Les deux comptes sont épinglés.** La prose de ce banc annonçait 66
    // aiguilles et 45 questions ; il en avait 63 et 43, et personne ne l'avait
    // vu — un nombre écrit dans un commentaire dérive, celui-là avait dérivé
    // avant la réparation du 10 octobre. Ces deux lignes ne jugent pas le
    // contenu : elles exigent qu'un ajout ou un retrait soit DÉCIDÉ, et que la
    // prose change dans le même commit.
    assert_eq!(
        CORPUS.len(),
        63,
        "le corpus à l'aiguille a changé ({} entrées) : corrigez ce compte ET la prose \
         en tête du banc dans le même commit, en disant ce qui entre ou sort",
        CORPUS.len()
    );
    assert_eq!(
        QUESTIONS.len(),
        43,
        "les questions ont changé ({} entrées) : le MRR se compare à nombre de questions \
         égal — corrigez ce compte, la prose, et dites à quelle mesure « avant » ces \
         chiffres restent comparables",
        QUESTIONS.len()
    );
    assert!(
        manquantes.is_empty(),
        "{} aiguilles du corpus vivant sur {} ne sont plus dans src/ — une fonction \
         renommée ou déménagée les emporte, et ce banc lit des FICHIERS, pas des \
         réexports. Remplacez-les par des fonctions restées dans src/, en nommant \
         la partie en regard, et dites en tête du banc ce qu'il a cessé de couvrir \
         (la réparation du 10 octobre 2026 en est l'exemple) :\n  {}",
        manquantes.len(),
        CORPUS.len(),
        manquantes.join("\n  ")
    );
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

fn options_vecteur() -> SearchOptions {
    SearchOptions {
        limit: 10,
        consistency: Consistency::Immediate,
        exige: Some(rag3weaver::disponibilite::Disponibilites::DENSE),
        signals: Some(SearchSignals::VECTOR),
        ..Default::default()
    }
}

/// L'entité de M1 et M1b : un nom, un champ de contenu, un chunk par texte
/// (`Fixed` à 4 000 > 1 500 — `chunked = false` refuse le vecteur, c'est le
/// même effet), vecteur seul.
fn fonction_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("texte".to_string(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
    EntityConfig {
        fields,
        signals: SearchSignals::VECTOR,
        chunking: ChunkingConfig { max_size: 4_000, overlap: 0, strategy: ChunkStrategy::Fixed, ..Default::default() },
        ..Default::default()
    }
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

/// **Les quatre lignes**, vecteur seul, sur les 43 questions.
#[test]
#[ignore]
fn banc_etage_qui_perd() {
    let (embedder, modele) = embarqueur();

    // ── Tel quel : src/ par le chemin réel ──────────────────────────────
    let reel = Arc::new(Mutex::new(base(embedder.clone(), "etage-reel")));
    register_code_schema(&mut reel.lock().unwrap(), default_scope_chunking()).unwrap();
    // Le signal creux est indépendant du dense : RAG3WEAVER_BANC_CREUX=bge-m3
    // l'active quel que soit le modèle dense — la configuration que le
    // produit aurait naturellement est granite-278m dense + creux bge-m3
    // (models.embed et models.sparse sur deux modèles), et un default_weights
    // avec du creux ne se pose pas sur la foi d'une mesure faite avec un
    // autre dense que celui du défaut (orchestration, 4 octobre). Avec
    // RAG3WEAVER_BANC_MODELE=bge-m3, le creux s'active tout seul. Scope est
    // ré-enregistré avec sparse (le banc éprouve une déclaration que le
    // produit n'a pas encore) et les embarqueurs creux/dual sont posés AVANT
    // toute ingestion. NB : avec deux modèles, le dual bge sert le CREUX des
    // documents ; le dense des documents reste celui de l'embarqueur du
    // catalogue (granite) — c'est le montage visé.
    #[cfg(feature = "burn-embedder")]
    let avec_creux = modele == "bge-m3"
        || std::env::var("RAG3WEAVER_BANC_CREUX").as_deref() == Ok("bge-m3");
    #[cfg(not(feature = "burn-embedder"))]
    let avec_creux = false;
    #[cfg(feature = "burn-embedder")]
    if avec_creux {
        let bge = common::burn::BGE_M3.clone();
        let mut cat = reel.lock().unwrap();
        let mut config = rag3weaver::code::scope_config(default_scope_chunking());
        config.signals = SearchSignals::HYBRID | SearchSignals::SPARSE;
        cat.register_entity(SCOPE, config).expect("Scope avec le creux");
        cat.set_sparse_embedder(bge.clone());
        // Le dual — dense ET creux en une passe — seulement quand le dense
        // EST bge : sur un catalogue granite, il écrirait le dense de bge
        // (1024) dans la colonne granite (768).
        if modele == "bge-m3" {
            cat.set_dual_embedder(bge);
        }
    }
    let racine = manifest();
    let sources: Vec<(String, String)> = read_sources(&format!("{racine}/src"))
        .expect("lire src/")
        .into_iter()
        .map(|(rel, c)| (format!("src/{rel}"), c))
        .collect();
    let analysis = rag3weaver::code::analyze(&racine, sources);
    let rapport = reel.lock().unwrap().ingest_code(&analysis).expect("ingérer src/");
    eprintln!("[étage] modèle {modele} · tel quel : {} scopes, {} en échec", rapport.scopes, rapport.failed);

    let mut tel_quel = Mesure::default();
    for (q, attendus) in QUESTIONS {
        let r = Catalog::rechercher(&reel, SCOPE, q, options_vecteur()).expect("recherche");
        tel_quel.noter(&noms(&r), attendus);
    }

    // ── G : le genre — tel quel, `scope_type` dans function | method ─────
    // M1b a renvoyé l'écart au corpus : 4 820 scopes dont les `tests
    // (namespace)` et les `file_scope_NN (module)` de fichier entier, qui
    // occupent le haut des listes sans jamais être une réponse. Ici on ne
    // change rien au texte ni à l'index : on ne garde que les genres qui
    // peuvent répondre. Ce que la ligne gagne, c'est ce que pèse la pollution.
    let mut par_genre = Mesure::default();
    for (q, attendus) in QUESTIONS {
        let mut o = options_vecteur();
        o.filter_condition = Some(FilterCondition::Should(vec![
            FilterCondition::Field { key: "scope_type".into(), value: FilterValue::Direct(CypherValue::String("function".into())) },
            FilterCondition::Field { key: "scope_type".into(), value: FilterValue::Direct(CypherValue::String("method".into())) },
        ]));
        let r = Catalog::rechercher(&reel, SCOPE, q, o).expect("recherche par genre");
        par_genre.noter(&noms(&r), attendus);
    }

    // ── P : la pondération par genre — un poids, pas un filtre (pas C) ──
    // Deux formes, trois valeurs, dans le même run : la demande (fichier
    // entier et espace de noms dévalués à x) et l'ombre pondérée de G
    // (function/method à 1, tout le reste à x par `default`). L'étage
    // appelant applique exactement la même table que la déclaration
    // d'entité : la mesure vaut pour les deux sans toucher `Scope`.
    let mut par_poids: Vec<(String, Mesure)> = Vec::new();
    {
        // La demande (file et namespace dévalués) : une valeur représentative —
        // la mesure du 3 octobre est identique à 0,8, 0,6 et 0,4 (les scores
        // RRF sont serrés, toute dévaluation déclasse le bloc entier).
        let x = 0.8;
        let mut p_demande = Mesure::default();
        for (q, attendus) in QUESTIONS {
            let mut o = options_vecteur();
            o.field_weights = vec![FieldWeight {
                field: "scope_type".into(),
                weights: [("file".to_string(), x), ("namespace".to_string(), x)].into_iter().collect(),
                default: 1.0,
            }];
            let r = Catalog::rechercher(&reel, SCOPE, q, o).expect("recherche pondérée");
            p_demande.noter(&noms(&r), attendus);
        }
        par_poids.push((format!("P(file,ns → {x}) — la demande, pondérée pas filtrée"), p_demande));
    }
    // L'ombre de G, des valeurs douces aux dures : à 0,8 un fichier de rang 1
    // retombe derrière une quinzaine de fonctions (1/61 × 0,8 ≈ 1/76) — on
    // cherche la valeur la plus douce qui atteint encore G. Aucune question
    // du banc n'a un fichier ou un module pour bonne réponse : le banc ne
    // voit pas ce que la dévaluation leur coûte, seulement ce qu'elle rend.
    for x in [0.95, 0.9, 0.85, 0.8, 0.6, 0.4] {
        let mut p_ombre = Mesure::default();
        for (q, attendus) in QUESTIONS {
            let mut o = options_vecteur();
            o.field_weights = vec![FieldWeight {
                field: "scope_type".into(),
                weights: [("function".to_string(), 1.0), ("method".to_string(), 1.0)].into_iter().collect(),
                default: x,
            }];
            let r = Catalog::rechercher(&reel, SCOPE, q, o).expect("recherche pondérée");
            p_ombre.noter(&noms(&r), attendus);
        }
        par_poids.push((format!("P(default → {x}) — l'ombre pondérée de G"), p_ombre));
    }

    // ── T : la marque de test, dévaluée — même geste que P (test_role) ──
    // La pile codeparsers fait des fonctions de `mod tests` des scopes :
    // 415 de plus sur src/dataflow, dont 350 fonctions de test. Sur
    // `merge_port_values`, deux tests qui l'appellent passent devant sa
    // définition (e2e_code:331). Aucune question du banc n'a un test pour
    // bonne réponse : la ligne mesure ce que la dévaluation rend aux
    // définitions, pas ce qu'elle coûte à « où est le test de X » — c'est
    // pour ces questions-là que la valeur doit rester douce. e2e_code fixe
    // le plafond (w < 0,60 sur les scores mesurés) ; le banc dit si une
    // valeur plus douce suffit déjà, et ce qu'on y gagne.
    for x in [0.7, 0.6, 0.55, 0.5, 0.4] {
        let mut t = Mesure::default();
        for (q, attendus) in QUESTIONS {
            let mut o = options_vecteur();
            o.field_weights = vec![FieldWeight {
                field: "test_role".into(),
                weights: [
                    ("case".to_string(), x),
                    ("suite".to_string(), x),
                    ("support".to_string(), x),
                ]
                .into_iter()
                .collect(),
                // La valeur vide — un scope qui n'est pas un test — reste
                // pleine : hors table, elle prend `default`.
                default: 1.0,
            }];
            let r = Catalog::rechercher(&reel, SCOPE, q, o).expect("recherche pondérée test_role");
            t.noter(&noms(&r), attendus);
        }
        par_poids.push((format!("T(case,suite,support → {x}) — les tests dévalués"), t));
    }

    // ── M : le seuil du « même motif » — une distribution, pas une note ──
    // Le crochet d'edit_file (apres-motif-ailleurs) montre les voisins de
    // l'ANCIEN texte au-dessus d'un seuil de similarité vectorielle. Pour le
    // poser : un échantillon de scopes, requête = leur propre texte, vecteur
    // seul, et l'on regarde le MEILLEUR voisin hors de leur fichier. Les
    // percentiles disent où vivent les voisins quelconques (le seuil doit
    // être au-dessus), les exemples nommés disent à quoi ressemble le haut
    // (les vrais motifs répétés — impl Node boilerplate — que la section
    // doit montrer). Mesure, pas d'assert : un banc mesure.
    {
        let rows = reel
            .lock()
            .unwrap()
            .execute_raw_with_params(
                "MATCH (s:Scope) WHERE s.scope_type = 'function' OR s.scope_type = 'method' \
                 RETURN s.name, s.file_path, s.content ORDER BY s.file_path, s.name",
                &[],
            )
            .expect("échantillon de scopes")
            .rows;
        let scopes: Vec<(String, String, String)> = rows
            .iter()
            .filter_map(|l| {
                Some((
                    l.first()?.as_str()?.to_string(),
                    l.get(1)?.as_str()?.to_string(),
                    l.get(2)?.as_str()?.to_string(),
                ))
            })
            .collect();
        // Un sur 37 : déterministe, ~60 requêtes sur src/, pas tout le tas.
        let mut meilleurs: Vec<(f64, String, String)> = Vec::new();
        for (i, (nom, fichier, contenu)) in scopes.iter().enumerate() {
            if i % 37 != 0 || contenu.trim().is_empty() {
                continue;
            }
            // Le budget de l'embarquement de requête : comme le crochet.
            let q: String = contenu.chars().take(4096).collect();
            let r = Catalog::rechercher(&reel, SCOPE, &q, options_vecteur())
                .expect("recherche motif");
            let voisin = r.results.iter().find(|x| {
                x.data
                    .as_ref()
                    .and_then(|d| d.get("file_path"))
                    .and_then(|v| v.as_str())
                    .is_some_and(|f| f != fichier)
            });
            if let Some(v) = voisin {
                let vnom = v
                    .data
                    .as_ref()
                    .and_then(|d| d.get("name"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("?")
                    .to_string();
                meilleurs.push((v.score, nom.clone(), vnom));
            }
        }
        meilleurs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let pct = |p: f64| -> f64 {
            if meilleurs.is_empty() {
                return 0.0;
            }
            let i = ((meilleurs.len() - 1) as f64 * p) as usize;
            meilleurs[i].0
        };
        eprintln!(
            "| M (seuil motif) — meilleur voisin hors fichier, {} requêtes | max {:.3} | p10 {:.3} | p25 {:.3} | p50 {:.3} | p90 {:.3} |",
            meilleurs.len(),
            pct(0.0),
            pct(0.10),
            pct(0.25),
            pct(0.50),
            pct(0.90),
        );
        for (score, de, vers) in meilleurs.iter().take(8) {
            eprintln!("|   motif : `{de}` ↔ `{vers}` ({score:.3}) |");
        }
    }

    // ── H et I : les poids de fusion en hybride, sur les deux versants ──
    // La mesure que Lucie attend depuis le 18 septembre, affinée le
    // 3 octobre : « favorise légèrement les identifiants » se cherche entre
    // 0,6/0,4 (protège les identifiants, coûte aux phrases) et 0,3/0,7
    // (l'inverse). H joue les 43 questions en langue naturelle ; I joue des
    // identifiants exacts du corpus — le versant qui n'avait qu'un « ça
    // passe », chiffré. Le sparse n'entre pas : granite-278m n'a pas de
    // sortie sparse — dit plutôt que laissé croire mesuré. Depuis la
    // pondération par genre déclarée dans `Scope` (3 octobre), ces lignes se
    // mesurent pondération comprise : cohérentes entre elles, pas avec les
    // chiffres d'avant la déclaration.
    const IDENTIFIANTS: [&str; 10] = [
        "merge_port_values",
        "fuse_signals",
        "resolve_search_target",
        "register_search_services",
        "embarquer_la_requete",
        "base_de_fusion",
        "appliquer_la_consigne_pour",
        "search_bm25_chunked",
        "parse_mermaid_template",
        "rendre_le_retard",
    ];
    let hybride_de = |b: f64, v: f64| -> SearchOptions {
        let mut o = options_vecteur();
        o.signals = Some(SearchSignals::HYBRID);
        o.fusion = Some(FusionConfig {
            bm25: SignalConfig { weight: b, ..SignalConfig::default() },
            vector: SignalConfig { weight: v, ..SignalConfig::default() },
            ..FusionConfig::default()
        });
        o
    };
    let mut hybrides: Vec<(String, Mesure, Mesure)> = Vec::new();
    for (etiquette, b, v) in [
        ("0,6/0,4 — le gabarit", 0.6, 0.4),
        ("0,55/0,45", 0.55, 0.45),
        ("0,5/0,5", 0.5, 0.5),
        ("0,45/0,55", 0.45, 0.55),
        ("0,4/0,6", 0.4, 0.6),
        ("0,3/0,7 — l'ancien moteur", 0.3, 0.7),
    ] {
        let mut phrases = Mesure::default();
        for (q, attendus) in QUESTIONS {
            let r = Catalog::rechercher(&reel, SCOPE, q, hybride_de(b, v)).expect("recherche hybride");
            phrases.noter(&noms(&r), attendus);
        }
        let mut idents = Mesure::default();
        for nom in IDENTIFIANTS {
            let r = Catalog::rechercher(&reel, SCOPE, nom, hybride_de(b, v)).expect("recherche d'identifiant");
            idents.noter(&noms(&r), &[nom]);
        }
        hybrides.push((etiquette.to_string(), phrases, idents));
    }
    // ── HS/IS : la fusion AVEC le signal creux (bge-m3 seulement) ───────
    // La demande de Lucie du 1er octobre : « mesurer avant de choisir… et
    // avec le signal sparse ». Le couple retenu provisoirement (0,45/0,55)
    // contre l'actuel du gabarit (0,5/0,5), sans creux (témoins ci-dessus)
    // et avec, à deux poids. Les poids RRF sont relatifs : pas de
    // renormalisation.
    if avec_creux {
        let hybride_creux_de = |b: f64, v: f64, sp: f64| -> SearchOptions {
            let mut o = options_vecteur();
            o.signals = Some(SearchSignals::HYBRID | SearchSignals::SPARSE);
            o.fusion = Some(FusionConfig {
                bm25: SignalConfig { weight: b, ..SignalConfig::default() },
                vector: SignalConfig { weight: v, ..SignalConfig::default() },
                sparse: SignalConfig { weight: sp, ..SignalConfig::default() },
                ..FusionConfig::default()
            });
            o
        };
        for (etiquette, b, v, sp) in [
            ("0,5/0,5 + creux 0,2", 0.5, 0.5, 0.2),
            ("0,5/0,5 + creux 0,4", 0.5, 0.5, 0.4),
            ("0,45/0,55 + creux 0,2", 0.45, 0.55, 0.2),
            ("0,45/0,55 + creux 0,4", 0.45, 0.55, 0.4),
        ] {
            let mut phrases = Mesure::default();
            for (q, attendus) in QUESTIONS {
                let r = Catalog::rechercher(&reel, SCOPE, q, hybride_creux_de(b, v, sp))
                    .expect("recherche hybride + creux");
                phrases.noter(&noms(&r), attendus);
            }
            let mut idents = Mesure::default();
            for nom in IDENTIFIANTS {
                let r = Catalog::rechercher(&reel, SCOPE, nom, hybride_creux_de(b, v, sp))
                    .expect("identifiant hybride + creux");
                idents.noter(&noms(&r), &[nom]);
            }
            hybrides.push((format!("HS {etiquette}"), phrases, idents));
        }
    }

    // ── A : l'ablation complète (expérience 1 de l'optimiseur) ─────────
    // Chaque voie seule, chaque paire, le trio — poids ÉGAUX entre les
    // voies actives : on mesure les APPORTS, pas un réglage. Elle dit si
    // le creux apporte par-dessus le plein texte ou le double (l'hypothèse
    // du doc optimiseur : notre gain viendrait de la DÉCOUPE du creux, pas
    // d'un signal neuf — un second plein texte). Sans creux (passes « b »,
    // découpeur en variante), seules les combinaisons sans SPARSE jouent.
    {
        let combo = |b: bool, v: bool, sp: bool| -> SearchOptions {
            let mut o = options_vecteur();
            let mut s = SearchSignals::NONE;
            if b { s = s | SearchSignals::BM25; }
            if v { s = s | SearchSignals::VECTOR; }
            if sp { s = s | SearchSignals::SPARSE; }
            o.signals = Some(s);
            let w = |on: bool| SignalConfig { weight: if on { 1.0 } else { 0.0 }, ..SignalConfig::default() };
            o.fusion = Some(FusionConfig {
                bm25: w(b),
                vector: w(v),
                sparse: w(sp),
                ..FusionConfig::default()
            });
            o
        };
        for (etiquette, b, v, sp) in [
            ("A plein texte seul", true, false, false),
            ("A dense seul", false, true, false),
            ("A creux seul", false, false, true),
            ("A texte+dense", true, true, false),
            ("A texte+creux", true, false, true),
            ("A dense+creux", false, true, true),
            ("A trio", true, true, true),
        ] {
            if sp && !avec_creux {
                continue;
            }
            let mut phrases = Mesure::default();
            for (q, attendus) in QUESTIONS {
                let r = Catalog::rechercher(&reel, SCOPE, q, combo(b, v, sp)).expect("ablation");
                phrases.noter(&noms(&r), attendus);
            }
            let mut idents = Mesure::default();
            for nom in IDENTIFIANTS {
                let r = Catalog::rechercher(&reel, SCOPE, nom, combo(b, v, sp)).expect("ablation id");
                idents.noter(&noms(&r), &[nom]);
            }
            hybrides.push((etiquette.to_string(), phrases, idents));
        }
    }

    // ── N : le titre pesé (chantier du 4 octobre) ───────────────────────
    // Le nom est indexé pour toute entité depuis ce jour ; un hit dont le
    // titre matche multiplie son score par le boost déclaré
    // (SimpleFieldDef.boost du champ titre). Chaque ligne ré-enregistre
    // Scope avec le boost — sans réindexer : le champ est déjà dans
    // l'index, seul le SearchTarget change. La ligne 1,0 est le témoin
    // « indexé sans poids » à comparer au 0,243/0,417 d'avant (non indexé).
    {
        let scope_au_boost = |b: f64| {
            let mut cat = reel.lock().unwrap();
            let mut config = rag3weaver::code::scope_config(default_scope_chunking());
            if avec_creux {
                config.signals = SearchSignals::HYBRID | SearchSignals::SPARSE;
            }
            if let Some(d) = config.fields.get_mut("name") {
                d.boost = Some(b);
            }
            cat.register_entity(SCOPE, config).expect("Scope au titre pesé");
        };
        let options_bm25 = || {
            let mut o = options_vecteur();
            o.signals = Some(SearchSignals::BM25);
            o.fusion = None;
            o
        };
        for b in [1.0, 1.5, 2.0, 3.0] {
            scope_au_boost(b);
            let mut phrases = Mesure::default();
            for (q, attendus) in QUESTIONS {
                let r = Catalog::rechercher(&reel, SCOPE, q, options_bm25()).expect("titre pesé");
                phrases.noter(&noms(&r), attendus);
            }
            let mut idents = Mesure::default();
            for nom in IDENTIFIANTS {
                let r = Catalog::rechercher(&reel, SCOPE, nom, options_bm25()).expect("titre pesé id");
                idents.noter(&noms(&r), &[nom]);
            }
            hybrides.push((format!("N texte seul, titre ×{b}"), phrases, idents));

            let mut ph = Mesure::default();
            for (q, attendus) in QUESTIONS {
                let r = Catalog::rechercher(&reel, SCOPE, q, hybride_de(0.45, 0.55)).expect("titre pesé H");
                ph.noter(&noms(&r), attendus);
            }
            let mut ih = Mesure::default();
            for nom in IDENTIFIANTS {
                let r = Catalog::rechercher(&reel, SCOPE, nom, hybride_de(0.45, 0.55)).expect("titre pesé IH");
                ih.noter(&noms(&r), &[nom]);
            }
            hybrides.push((format!("N H 0,45/0,55, titre ×{b}"), ph, ih));
        }
        // La config de base revient pour les sections qui suivent.
        scope_au_boost(1.0);
    }

    // ── U : la note d'utilité comme relecteur (essai 3 de l'optimiseur) ──
    // Par-dessus la fusion du produit (0,45/0,55) : chaque résultat est noté
    // sur une grille à quatre niveaux par le modèle de décision (JevK5-4B,
    // `score` = Σ pᵢ·i au-dessus de `decide`), les résultats se reclassent
    // par note (tri stable : l'égalité garde l'ordre de fusion), les notes
    // sous 1,5 partent en queue. Rien d'allumé au produit — une voie de plus
    // au banc, jouée seulement si RAG3WEAVER_BANC_UTILITE donne l'adresse du
    // serveur (http://127.0.0.1:7982). La grille est à nous, pas un copié de
    // jevbox (dépôt sans licence).
    if let Ok(adresse) = std::env::var("RAG3WEAVER_BANC_UTILITE") {
        use rag3weaver::decider::{score, LlamaServerDecider};
        let decider = LlamaServerDecider::new(&adresse, "JevK5-4B").expect("adresse du décideur");
        let grille: Vec<String> = [
            "sans rapport avec la question",
            "du même sujet, mais n'aide pas à y répondre",
            "répond en partie, ou donne des faits qui aident",
            "contient précisément ce qui est demandé",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let vivant = score(&decider, "La mer est-elle bleue ?", &grille, 1.0);
        if let Err(e) = &vivant {
            eprintln!("[U] décideur muet ({adresse}) : {e} — section sautée");
        } else {
            let mut appels = 0usize;
            let mut duree = std::time::Duration::ZERO;
            let mut releu = |requete: &str, attendus: &[&str], avant: &mut Mesure, apres: &mut Mesure| {
                let r = Catalog::rechercher(&reel, SCOPE, requete, hybride_de(0.45, 0.55))
                    .expect("fusion du produit");
                let noms_bruts: Vec<String> = r
                    .results
                    .iter()
                    .map(|x| {
                        x.data
                            .as_ref()
                            .and_then(|d| d.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                            .to_string()
                    })
                    .collect();
                avant.noter(&noms_bruts, attendus);
                let t0 = std::time::Instant::now();
                let notes: Vec<f64> = r
                    .results
                    .iter()
                    .map(|x| {
                        let extrait: String = x
                            .chunk
                            .as_ref()
                            .map(|c| c.text.chars().take(1200).collect())
                            .unwrap_or_default();
                        if extrait.is_empty() {
                            return 1.5; // sans extrait on ne juge pas : neutre
                        }
                        appels += 1;
                        score(
                            &decider,
                            &format!("Question : {requete}
Extrait :
{extrait}"),
                            &grille,
                            1.0,
                        )
                        .unwrap_or(1.5)
                    })
                    .collect();
                duree += t0.elapsed();
                let mut ordre: Vec<usize> = (0..noms_bruts.len()).collect();
                // Tri stable par note décroissante ; les écartés (< 1,5) en
                // queue, dans leur ordre de fusion.
                ordre.sort_by(|&a, &b| {
                    let (ea, eb) = (notes[a] < 1.5, notes[b] < 1.5);
                    eb.cmp(&ea).reverse().then(
                        notes[b].partial_cmp(&notes[a]).unwrap_or(std::cmp::Ordering::Equal),
                    )
                });
                let noms_releus: Vec<String> =
                    ordre.iter().map(|&i| noms_bruts[i].clone()).collect();
                apres.noter(&noms_releus, attendus);
                // Montées et descentes nommées, sur l'attendu.
                let rang = |liste: &[String]| {
                    liste.iter().position(|n| attendus.contains(&n.as_str()))
                };
                if let (Some(a), Some(b)) = (rang(&noms_bruts), rang(&noms_releus)) {
                    if a != b {
                        let sens = if b < a { "monte" } else { "descend" };
                        eprintln!(
                            "[U] « {} » : l'attendu {} → {} ({sens})",
                            requete.chars().take(50).collect::<String>(),
                            a + 1,
                            b + 1
                        );
                    }
                }
            };
            let (mut ph_avant, mut ph_apres) = (Mesure::default(), Mesure::default());
            for (q, attendus) in QUESTIONS {
                releu(q, attendus, &mut ph_avant, &mut ph_apres);
            }
            let (mut id_avant, mut id_apres) = (Mesure::default(), Mesure::default());
            for nom in IDENTIFIANTS {
                releu(nom, &[nom], &mut id_avant, &mut id_apres);
            }
            hybrides.push(("U avant relecture (0,45/0,55)".into(), ph_avant, id_avant));
            hybrides.push(("U après relecture par la note".into(), ph_apres, id_apres));
            let n_requetes = QUESTIONS.len() + IDENTIFIANTS.len();
            eprintln!(
                "[U] coût : {appels} appels au modèle, {:.2} s par requête en moyenne ({} requêtes)",
                duree.as_secs_f64() / n_requetes as f64,
                n_requetes
            );
        }
    }

    // Le témoin : les identifiants en vecteur seul — le chiffre du problème.
    let mut idents_vecteur = Mesure::default();
    for nom in IDENTIFIANTS {
        let r = Catalog::rechercher(&reel, SCOPE, nom, options_vecteur()).expect("identifiant au vecteur");
        idents_vecteur.noter(&noms(&r), &[nom]);
    }

    // ── M2 : cosinus exact contre tous les chunks, même résolution ──────
    // Le vecteur de la requête est celui du catalogue ; la comparaison est
    // exhaustive au lieu du HNSW ; les 20 meilleurs chunks remontent à leur
    // parent, dédoublonnés au meilleur rang — la résolution d'aujourd'hui.
    let stockage = reel.lock().unwrap().vector_storage("Scope_Chunk").expect("stockage du modèle courant");
    let mut m2 = Mesure::default();
    for (q, attendus) in QUESTIONS {
        let (qvec, _) = reel.lock().unwrap().embarquer_la_requete(q, true, false).expect("embarquer la requête");
        let cypher = format!(
            "MATCH (c:Scope_Chunk) WHERE c.{col} IS NOT NULL \
             WITH c, array_cosine_similarity(c.{col}, $q) AS sim ORDER BY sim DESC LIMIT 20 \
             MATCH (c)-[:Scope_CHUNKED_FROM]->(p:Scope) RETURN p.name, sim ORDER BY sim DESC",
            col = stockage.column
        );
        let q_val = CypherValue::List(qvec.iter().map(|&f| CypherValue::Float(f as f64)).collect());
        let r = reel.lock().unwrap().execute_raw_with_params(&cypher, &[QueryParam::new("q", q_val)]).expect("cosinus exact");
        let classes = parents_uniques(r.rows.iter().filter_map(|l| l.first()?.as_str().map(str::to_string)));
        m2.noter(&classes, attendus);
    }

    // ── M3 : les 20 chunks bruts du HNSW, avant résolution ──────────────
    // Ce que l'index rend vraiment, et si le bon parent y est — à quel rang.
    let backend = reel.lock().unwrap().search_backend().expect("backend de recherche");
    let mut m3 = Mesure::default();
    let mut bon_parent_dans_les_20 = 0usize;
    for (q, attendus) in QUESTIONS {
        let (qvec, _) = reel.lock().unwrap().embarquer_la_requete(q, true, false).expect("embarquer la requête");
        let hits = backend
            .vector_search("Scope_Chunk", &stockage.index, &stockage.column, &qvec, 20)
            .expect("HNSW brut");
        let uuids = CypherValue::List(hits.iter().map(|h| CypherValue::String(h.uuid.clone())).collect());
        let r = reel
            .lock()
            .unwrap()
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
    // Les 63 scopes à l'aiguille, entité d'un seul champ de contenu, un chunk
    // par texte (Fixed, 4 000 > 1 500), vecteur seul.
    let embedder_m1b = embedder.clone();
    let nu = Arc::new(Mutex::new(base(embedder, "etage-m1")));
    nu.lock().unwrap().register_entity("Fonction", fonction_config()).expect("enregistrer Fonction");
    let src_dir = std::path::Path::new(&racine).join("src");
    verifier_les_aiguilles(&src_dir);
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
    let r = nu.lock().unwrap().ingest_entities("Fonction", lignes).expect("ingérer les scopes à l'aiguille");
    eprintln!("[étage] M1 : {} scopes à l'aiguille, {} en échec", r.processed, r.failed);
    let mut m1 = Mesure::default();
    for (q, attendus) in QUESTIONS {
        let r = Catalog::rechercher(&nu, "Fonction", q, options_vecteur()).expect("recherche M1");
        m1.noter(&noms(&r), attendus);
    }

    // ── M1b : le texte de M1, sur le corpus de tel quel ─────────────────
    // M1 change le texte *et* la taille du corpus (63 contre 4 819). Ici les
    // mêmes 4 819 scopes que tel quel, chacun embarqué comme un texte assemblé
    // côté test depuis `analysis.scopes` — nom, doc, signature, corps, coupé
    // comme le cosinus nu — sans rien changer à `EntityConfig`. Si M1b tient
    // près de M1, le texte explique tout ; s'il retombe vers tel quel, c'est
    // la taille du corpus.
    let plein = Arc::new(Mutex::new(base(embedder_m1b, "etage-m1b")));
    plein.lock().unwrap().register_entity("Fonction", fonction_config()).expect("enregistrer Fonction");
    let lignes: Vec<std::collections::BTreeMap<String, CypherValue>> = analysis
        .scopes
        .iter()
        .map(|sc| {
            let mut texte = String::new();
            for morceau in [sc.name.as_str(), sc.docstring.as_str(), sc.signature.as_str(), sc.content.as_str()] {
                if !morceau.is_empty() {
                    texte.push_str(morceau);
                    texte.push('\n');
                }
            }
            if texte.len() > CHUNK_CHARS {
                let mut coupe = CHUNK_CHARS;
                while !texte.is_char_boundary(coupe) {
                    coupe -= 1;
                }
                texte.truncate(coupe);
            }
            let mut d = std::collections::BTreeMap::new();
            d.insert("name".to_string(), CypherValue::String(sc.name.clone()));
            d.insert("texte".to_string(), CypherValue::String(texte));
            d
        })
        .collect();
    let r = plein.lock().unwrap().ingest_entities("Fonction", lignes).expect("ingérer les scopes assemblés");
    eprintln!("[étage] M1b : {} scopes assemblés, {} en échec", r.processed, r.failed);
    let mut m1b = Mesure::default();
    for (q, attendus) in QUESTIONS {
        let r = Catalog::rechercher(&plein, "Fonction", q, options_vecteur()).expect("recherche M1b");
        m1b.noter(&noms(&r), attendus);
    }

    // ── Le tableau ──────────────────────────────────────────────────────
    eprintln!("\n## L'étage qui perd — {modele}, vecteur seul, {} questions\n", QUESTIONS.len());
    eprintln!("| ligne | MRR | R@1 | R@5 |");
    eprintln!("|---|---:|---:|---:|");
    eprintln!("{}", tel_quel.ligne("tel quel — Catalog::search sur src/"));
    eprintln!("{}", m1.ligne("M1 — même texte que le cosinus nu, un chunk par fonction"));
    eprintln!("{}", m2.ligne("M2 — cosinus exact au lieu du HNSW, même résolution"));
    eprintln!("{}", m3.ligne("M3 — les 20 chunks bruts du HNSW, avant résolution"));
    eprintln!("{}", m1b.ligne("M1b — le texte de M1 sur les 4 819 scopes de src/"));
    eprintln!("{}", par_genre.ligne("G — tel quel, scope_type dans function | method"));
    for (etiquette, m) in &par_poids {
        eprintln!("{}", m.ligne(etiquette));
    }
    for (etiquette, phrases, idents) in &hybrides {
        eprintln!("{}", phrases.ligne(&format!("H phrases ({etiquette})")));
        eprintln!("{}", idents.ligne_sur(&format!("I identifiants ({etiquette})"), IDENTIFIANTS.len()));
    }
    eprintln!("{}", idents_vecteur.ligne_sur("I identifiants (vecteur seul — témoin)", IDENTIFIANTS.len()));
    eprintln!("(hybride sans sparse : granite-278m n'a pas de sortie sparse — le poids sparse reste à mesurer avec un modèle qui en a une)");
    eprintln!("\nM3 : le bon parent est dans les 20 chunks bruts pour {bon_parent_dans_les_20}/{} questions.", QUESTIONS.len());

    assert!(rapport.scopes > 0, "rien n'a été indexé");
}

/// **La cohésion par-dessus la fusion du produit** (session codeparsers,
/// demande de l'orchestration du 4 octobre). La fusion du produit
/// (bm25 0,45 / vector 0,55, sans creux), puis `SearchOptions.cohesion` à
/// W = 0 / 0,2 / 0,5 : un candidat relié dans le graphe du code (CONSUMES,
/// INHERITS_FROM, IMPLEMENTS, deux sauts, carrefours exclus) aux autres
/// candidats fusionnés remonte. W = 0 doit égaler la ligne « 0,45/0,55 » de
/// la section H (même base fraîche). Un test à part, avec sa propre base,
/// pour itérer sur W sans rejouer les autres sections. Un banc mesure, il
/// n'échoue pas : les montées et descentes sont nommées, pas assertées.
#[test]
#[ignore]
fn banc_cohesion_produit() {
    let (embedder, modele) = embarqueur();
    let reel = Arc::new(Mutex::new(base(embedder.clone(), "etage-cohesion")));
    register_code_schema(&mut reel.lock().unwrap(), default_scope_chunking()).unwrap();
    let racine = manifest();
    let sources: Vec<(String, String)> = read_sources(&format!("{racine}/src"))
        .expect("lire src/")
        .into_iter()
        .map(|(rel, c)| (format!("src/{rel}"), c))
        .collect();
    let rapport = reel.lock().unwrap().ingest_code(&rag3weaver::code::analyze(&racine, sources)).expect("ingérer src/");
    eprintln!("[cohésion] modèle {modele} · {} scopes", rapport.scopes);

    // Copiés de la section H — deux tests ne se prêtent rien.
    const IDENTIFIANTS: [&str; 10] = [
        "merge_port_values",
        "fuse_signals",
        "resolve_search_target",
        "register_search_services",
        "embarquer_la_requete",
        "base_de_fusion",
        "appliquer_la_consigne_pour",
        "search_bm25_chunked",
        "parse_mermaid_template",
        "rendre_le_retard",
    ];
    let options = |w: f64| -> SearchOptions {
        let mut o = options_vecteur();
        o.signals = Some(SearchSignals::HYBRID);
        o.fusion = Some(FusionConfig {
            bm25: SignalConfig { weight: 0.45, ..SignalConfig::default() },
            vector: SignalConfig { weight: 0.55, ..SignalConfig::default() },
            ..FusionConfig::default()
        });
        if w != 0.0 {
            o.cohesion = Some(rag3weaver::search::CohesionOptions {
                weight: w,
                relations: vec!["CONSUMES".into(), "INHERITS_FROM".into(), "IMPLEMENTS".into()],
                ..Default::default()
            });
        }
        o
    };
    let poids = [0.0, 0.2, 0.5];
    let mut lignes = Vec::new();
    let mut classements: Vec<(Vec<Vec<String>>, Vec<Vec<String>>)> = Vec::new();
    for w in poids {
        let (mut ph, mut id) = (Mesure::default(), Mesure::default());
        let (mut cp, mut ci) = (Vec::new(), Vec::new());
        let t = std::time::Instant::now();
        for (q, attendus) in QUESTIONS {
            let n = noms(&Catalog::rechercher(&reel, SCOPE, q, options(w)).expect("recherche"));
            ph.noter(&n, attendus);
            cp.push(n);
        }
        for nom in IDENTIFIANTS {
            let n = noms(&Catalog::rechercher(&reel, SCOPE, nom, options(w)).expect("recherche d'identifiant"));
            id.noter(&n, &[nom]);
            ci.push(n);
        }
        let ms = t.elapsed().as_millis() as usize / (QUESTIONS.len() + IDENTIFIANTS.len());
        lignes.push(format!("{} / I {:.3} {}/{} — {ms} ms par recherche", ph.ligne(&format!("produit 0,45/0,55, cohésion W = {w}")), id.mrr / IDENTIFIANTS.len() as f64, id.r1, id.r5));
        classements.push((cp, ci));
    }
    eprintln!("\n| ligne | MRR | R@1 | R@5 | identifiants (MRR R@1/R@5) |\n|---|---|---|---|---|");
    for l in &lignes {
        eprintln!("{l}");
    }
    let rang = |l: &Vec<String>, attendus: &[&str]| l.iter().position(|n| attendus.contains(&n.as_str()));
    for (i, w) in poids.iter().enumerate().skip(1) {
        let (mut montees, mut descentes) = (Vec::new(), Vec::new());
        let jeux: Vec<(String, Vec<&str>, &Vec<String>, &Vec<String>)> = QUESTIONS
            .iter()
            .enumerate()
            .map(|(k, (q, a))| (q.to_string(), a.to_vec(), &classements[0].0[k], &classements[i].0[k]))
            .chain(IDENTIFIANTS.iter().enumerate().map(|(k, nom)| (format!("[id] {nom}"), vec![*nom], &classements[0].1[k], &classements[i].1[k])))
            .collect();
        for (q, attendus, avant, apres) in jeux {
            let (a, b) = (rang(avant, &attendus), rang(apres, &attendus));
            let f = |r: Option<usize>| r.map(|x| (x + 1).to_string()).unwrap_or("-".into());
            let ligne = format!("« {q} » : {} → {} (avant {:?} ; après {:?})", f(a), f(b), &avant[..avant.len().min(3)], &apres[..apres.len().min(3)]);
            match (a.unwrap_or(99), b.unwrap_or(99)) {
                (x, y) if y < x => montees.push(ligne),
                (x, y) if y > x => descentes.push(ligne),
                _ => {}
            }
        }
        eprintln!("\n### W = {w} contre W = 0 : {} montées, {} descentes", montees.len(), descentes.len());
        for l in montees {
            eprintln!("- ↑ {l}");
        }
        for l in descentes {
            eprintln!("- ↓ {l}");
        }
    }
}
