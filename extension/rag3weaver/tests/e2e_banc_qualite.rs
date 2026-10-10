//! **Le banc de qualité des modèles d'embedding sur des requêtes de code.**
//!
//! La vitesse est mesurée (`e2e_banc_bge_m3`) ; ici on mesure ce que chaque
//! modèle *retrouve*. Le corpus est un jeu de scopes **réels** de rag3weaver,
//! lus dans `src/` au moment du test et découpés comme l'indexation le fait
//! (une fonction avec sa doc, coupée à 1 500 caractères) ; les requêtes sont
//! des questions de développeur, en français et en anglais, avec la fonction
//! attendue. Par modèle : MRR, rappel à 1 et à 5, et la taille d'index pour
//! 20 000 chunks. Deux variantes : le code tel quel (les commentaires sont en
//! français, ce qui avantage le français), et le code sans commentaires (ce
//! qu'on trouverait dans un dépôt commenté en anglais, ou pas du tout).
//!
//! Un seul processus à la fois sur la carte, et **`RAG3WEAVER_SANS_DEMON=1`** :
//! le démon en place peut appartenir à une autre session (il sert peut-être
//! un autre modèle), et un test qui le trouve périmé le remplacerait.
//!
//! **Ce que ce banc a cessé de couvrir (10 octobre 2026).** Il lit des
//! FICHIERS sous `src/`, pas des réexports. La crate `rag3weaver-ir`
//! (`41b869ba4`) a déplacé quatre de ses aiguilles hors de `src/` : le commit
//! dit vrai — « aucun site d'usage ne bouge » — mais un banc à corpus vivant
//! ne lit pas les usages, il lit l'arborescence, et les réexports lui sont
//! invisibles. Les quatre sont remplacées par des fonctions restées dans
//! `src/`, nommées comme remplacement à leur ligne ; le banc **ne couvre donc
//! plus** `Scope::validate_id`, `Scope::index_name`, `Scope::stamp` (passées
//! dans `ir/src/scope.rs`) ni `is_valid_identifier` (passée dans
//! `ir/src/value.rs`). Les quatre mêmes remplacements sont faits dans
//! `e2e_banc_etage.rs`, qui copie ce corpus : **les deux ne doivent pas
//! diverger**, c'est tout l'intérêt de la copie.
//!
//! **Un MRR d'avant cette date ne se compare pas à un MRR d'après** : deux des
//! questions changent de cible (celles qui visaient `validate_id` et
//! `index_name`), à nombre de questions égal. Pour comparer deux modèles, les
//! deux se mesurent sur ce banc-ci.
//!
//! Quand étendre le corpus à `ir/src/` : le jour où la crate portera les
//! **formes** du dialecte (`Hop` est là, `Count` et `Select` suivent), parce
//! que ce sera du vocabulaire qu'un agent cherche. En une fois, en rejouant
//! toutes les références (F, 10 octobre).
//!
//! ```text
//! RAG3WEAVER_SANS_DEMON=1 cargo test --features burn-embedder,daemon \
//!     --test e2e_banc_qualite -- --ignored --nocapture
//! RAG3WEAVER_BANC_MODELES=granite-107m,bge-m3   # pour n'en mesurer que certains
//! ```
#![cfg(feature = "burn-embedder")]

mod common;

use rag3weaver::embedder::Embedder;
use std::path::Path;

/// **Le corpus** : (fichier, aiguille) — la première ligne de `src/<fichier>`
/// qui contient l'aiguille ouvre le scope. Les cibles des questions y sont,
/// et autant de voisins qui leur ressemblent (même module, même vocabulaire)
/// pour que retrouver la bonne ne soit pas gratuit.
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

/// **Les questions**, avec le nom de la fonction attendue (plusieurs quand
/// deux voisines répondent aussi bien). Écrites comme on les taperait, sans
/// recopier le nom de la fonction.
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

/// La limite de l'indexation (façon ragforge : 1 500 caractères par chunk).
const CHUNK_CHARS: usize = 1500;

struct Scope {
    nom: String,
    texte: String,
}

/// Le nom de la fonction dans l'aiguille : ce qui suit `fn ` jusqu'à `(` ou `<`.
fn nom_de(aiguille: &str) -> String {
    let apres = &aiguille[aiguille.find("fn ").unwrap() + 3..];
    apres.split(|c: char| c == '(' || c == '<').next().unwrap().to_string()
}

/// Un scope tel que l'indexation le verrait : la doc au-dessus, la
/// signature, le corps jusqu'à l'accolade qui ferme, coupé à
/// [`CHUNK_CHARS`]. `sans_commentaires` retire toute ligne `//…`.
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

/// **Toutes les aiguilles manquantes d'un coup, et un seul échec.**
///
/// `extraire` paniquait sur la première introuvable — et c'est ce qui rendait
/// `le_corpus_se_lit` muet sur le vrai sujet : il meurt dans `corpus()` avant
/// d'atteindre ses propres gardes, qui vérifient pourtant que chaque question
/// a sa cible. Un déménagement qui emportait quatre aiguilles n'en annonçait
/// donc qu'une, et la garde qui aurait tout dit ne s'exécutait jamais.
///
/// Le relevé est statique : il ne lit que les fichiers, aucun modèle, aucune
/// carte. Il ne coûte rien à la mesure.
fn verifier_les_aiguilles(racine: &Path) {
    let mut manquantes: Vec<String> = Vec::new();
    for (fichier, aiguille) in CORPUS {
        match std::fs::read_to_string(racine.join(fichier)) {
            Ok(source) => {
                if !source.lines().any(|l| l.contains(aiguille)) {
                    manquantes.push(format!("{fichier} :: {aiguille} — aiguille absente du fichier"));
                }
            }
            Err(e) => manquantes.push(format!("{fichier} — illisible ({e})")),
        }
    }
    assert!(
        manquantes.is_empty(),
        "{} aiguilles du corpus vivant sur {} ne sont plus dans src/ — une fonction \
         renommée ou déménagée les emporte, et ce banc lit des FICHIERS, pas des \
         réexports. Remplacez-les par des fonctions restées dans src/, en nommant la \
         partie en regard, dites en tête ce que le banc a cessé de couvrir, et faites \
         le MÊME remplacement dans e2e_banc_etage.rs — les deux corpus ne doivent pas \
         diverger (réparation du 10 octobre 2026) :\n  {}",
        manquantes.len(),
        CORPUS.len(),
        manquantes.join("\n  ")
    );
}

fn corpus(sans_commentaires: bool) -> Vec<Scope> {
    let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    verifier_les_aiguilles(&racine);
    CORPUS.iter().map(|(fichier, aiguille)| {
        let source = std::fs::read_to_string(racine.join(fichier)).unwrap_or_else(|e| panic!("{fichier} : {e}"));
        Scope { nom: nom_de(aiguille), texte: extraire(&source, aiguille, sans_commentaires) }
    }).collect()
}

fn cos(a: &[f32], b: &[f32]) -> f32 {
    let (mut ab, mut aa, mut bb) = (0f32, 0f32, 0f32);
    for (x, y) in a.iter().zip(b) { ab += x * y; aa += x * x; bb += y * y; }
    ab / (aa.sqrt() * bb.sqrt()).max(1e-12)
}

#[derive(Default)]
struct Mesure {
    mrr: f64,
    r1: f64,
    r5: f64,
    /// Les questions ratées à 5, pour lire *ce que* le modèle ne voit pas.
    ratees: Vec<String>,
}

fn mesurer(e: &dyn Embedder, scopes: &[Scope]) -> Mesure {
    let textes: Vec<String> = scopes.iter().map(|s| s.texte.clone()).collect();
    let vecs = e.embed(&textes).expect("embed corpus");
    let questions: Vec<String> = QUESTIONS.iter().map(|(q, _)| q.to_string()).collect();
    let qvecs = e.embed(&questions).expect("embed questions");
    let mut m = Mesure::default();
    for (i, (q, attendus)) in QUESTIONS.iter().enumerate() {
        let mut classement: Vec<(usize, f32)> = vecs.iter().enumerate().map(|(j, v)| (j, cos(&qvecs[i], v))).collect();
        classement.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let rang = classement.iter().position(|(j, _)| attendus.contains(&scopes[*j].nom.as_str())).unwrap() + 1;
        m.mrr += 1.0 / rang as f64;
        if rang <= 1 { m.r1 += 1.0; }
        if rang <= 5 { m.r5 += 1.0; } else {
            let premiers: Vec<&str> = classement.iter().take(3).map(|(j, _)| scopes[*j].nom.as_str()).collect();
            m.ratees.push(format!("  rang {rang:>2} · « {q} » → {} ; vus : {}", attendus[0], premiers.join(", ")));
        }
    }
    let n = QUESTIONS.len() as f64;
    m.mrr /= n; m.r1 /= n; m.r5 /= n;
    m
}

fn modeles() -> Vec<(&'static str, std::sync::Arc<dyn Embedder>)> {
    let voulus = std::env::var("RAG3WEAVER_BANC_MODELES").ok();
    let tous: Vec<(&str, fn() -> std::sync::Arc<dyn Embedder>)> = vec![
        ("granite-107m", || common::burn::GRANITE_107M.clone()),
        ("granite-278m", || common::burn::GRANITE_278M.clone()),
        ("bge-m3", || common::burn::BGE_M3.clone()),
        ("minilm", || common::burn::MINILM.clone()),
        ("multilingual-minilm", || common::burn::MULTILINGUAL_MINILM.clone()),
    ];
    tous.into_iter()
        .filter(|(nom, _)| voulus.as_deref().map_or(true, |v| v.split(',').any(|x| x.trim() == *nom)))
        .map(|(nom, f)| (nom, f()))
        .collect()
}

/// **Le tableau** : modèle × variante × (MRR, R@1, R@5), plus la taille de
/// l'index. Chaque modèle est chargé une fois et mesuré sur les deux corpus.
#[test]
#[ignore]
fn banc_qualite() {
    let _ = env_logger::try_init();
    let avec = corpus(false);
    let sans = corpus(true);
    let chars: usize = avec.iter().map(|s| s.texte.len()).sum();
    eprintln!("[banc qualité] {} scopes ({} caractères, {} sans commentaires), {} questions ({} fr, {} en)",
        avec.len(), chars, sans.iter().map(|s| s.texte.len()).sum::<usize>(), QUESTIONS.len(), 25, QUESTIONS.len() - 25);
    let mut lignes = Vec::new();
    let mut ratees = Vec::new();
    for (nom, e) in modeles() {
        let t = std::time::Instant::now();
        let ma = mesurer(e.as_ref(), &avec);
        let ms = mesurer(e.as_ref(), &sans);
        let mo = e.dim() * 4 * 20_000 / 1_000_000;
        lignes.push(format!("| {nom:<20} | {:>4} | {:.3} | {:.2} | {:.2} | {:.3} | {:.2} | {:.2} | {mo:>4} Mo | {:.1} s |",
            e.dim(), ma.mrr, ma.r1, ma.r5, ms.mrr, ms.r1, ms.r5, t.elapsed().as_secs_f64()));
        if !ma.ratees.is_empty() { ratees.push(format!("{nom}, avec commentaires, ratées à 5 :\n{}", ma.ratees.join("\n"))); }
        if !ms.ratees.is_empty() { ratees.push(format!("{nom}, sans commentaires, ratées à 5 :\n{}", ms.ratees.join("\n"))); }
    }
    eprintln!("\n| modèle | dim | MRR | R@1 | R@5 | MRR sans // | R@1 | R@5 | index 20 k | temps |");
    eprintln!("|---|---|---|---|---|---|---|---|---|---|");
    for l in &lignes { eprintln!("{l}"); }
    eprintln!();
    for r in &ratees { eprintln!("{r}\n"); }
}

/// Le corpus se lit bien : chaque aiguille trouve un scope non vide qui
/// commence par sa doc ou sa signature, et chaque question a une cible.
#[test]
fn le_corpus_se_lit() {
    let avec = corpus(false);
    let noms: Vec<&str> = avec.iter().map(|s| s.nom.as_str()).collect();
    for s in &avec {
        assert!(s.texte.len() > 40, "{} : {:?}", s.nom, s.texte);
        assert!(s.texte.contains(&format!("fn {}", s.nom)), "{} : {:?}", s.nom, s.texte);
    }
    for (q, attendus) in QUESTIONS {
        for a in *attendus {
            assert!(noms.contains(a), "« {q} » attend {a}, absent du corpus");
        }
    }
    let sans = corpus(true);
    assert!(sans.iter().all(|s| !s.texte.lines().any(|l| l.trim_start().starts_with("//"))));
}
