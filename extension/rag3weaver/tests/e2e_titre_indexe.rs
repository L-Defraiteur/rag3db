//! **La garantie des lignes exactes, titre indexé** (question de Lucie,
//! 4 octobre 2026) : le titre entre dans lucivy comme un champ À PART —
//! rien n'est préfixé ni mêlé au texte du contenu, donc un extrait trouvé
//! par mots garde ses lignes au fichier près, avec et sans touche sur le
//! titre, avec et sans boost. Éprouvé sur du code (un dépôt en mémoire,
//! sans disque — le cas qu'elle cite) et sur un document.
//!
//! Run with: ./run_e2e.sh --test e2e_titre_indexe

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use rag3weaver::code::{default_scope_chunking, register_code_schema, SCOPE};
use rag3weaver::config::{EntityConfig, FieldType, SimpleFieldDef};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn base() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).expect("extension vecteur");
    let config = CatalogConfig { name: Some("titre-indexe".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    catalog
}

fn options_bm25() -> SearchOptions {
    SearchOptions {
        limit: 10,
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        ..Default::default()
    }
}

/// Le dépôt en mémoire : un seul fichier, des positions connues.
const SOURCE: &str = "//! Un module de démonstration, trois scopes.

fn preambule() {
    let _ = 1;
}

/// La cible : son corps parle de pollinisation croisée des abeilles.
fn cible_du_test() {
    let pollinisation = \"croisée des abeilles\";
    let _ = pollinisation;
}

fn epilogue() {
    let _ = 3;
}
";

/// Les lignes `start..=end` (1-based) du texte, telles qu'au fichier.
fn lignes(texte: &str, start: usize, end: usize) -> String {
    texte
        .lines()
        .skip(start.saturating_sub(1))
        .take(end.saturating_sub(start) + 1)
        .collect::<Vec<_>>()
        .join("\n")
}

fn scope_au_boost(cat: &Arc<Mutex<Catalog>>, boost: Option<f64>) {
    let mut config = rag3weaver::code::scope_config(default_scope_chunking());
    if let Some(d) = config.fields.get_mut("name") {
        d.boost = boost;
    }
    cat.lock().unwrap().register_entity(SCOPE, config).expect("Scope re-déclaré");
}

#[test]
fn le_code_garde_ses_lignes_au_fichier_pres() {
    let mut catalog = base();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    // Racine absolue d'un dépôt qui n'existe pas sur disque : le contenu
    // vient de content_map, c'est le cas « dépôt en mémoire » de Lucie.
    let analysis = rag3weaver::code::analyze(
        "/depot-en-memoire",
        vec![("src/demo.rs".to_string(), SOURCE.to_string())],
    );
    eprintln!(
        "[titre] analyse: {} fichiers, {} scopes, écartés {:?}",
        analysis.files.len(),
        analysis.scopes.len(),
        analysis.skipped
    );
    let rapport = catalog.ingest_code(&analysis).expect("ingérer le dépôt en mémoire");
    eprintln!("[titre] {} scopes, {} en échec", rapport.scopes, rapport.failed);
    assert!(rapport.scopes > 0, "l'analyse du dépôt en mémoire n'a rendu aucun scope");
    let cat = Arc::new(Mutex::new(catalog));

    let garanties = |etiquette: &str| {
        // 1. Par les MOTS du corps : l'extrait rendu désigne les lignes du
        // fichier, à la ligne près — le texte relu à ces lignes porte le mot
        // ET la définition.
        let r = Catalog::rechercher(&cat, SCOPE, "pollinisation", options_bm25()).expect("mots du corps");
        let rendus: Vec<String> = r
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
        let hit = r
            .results
            .iter()
            .find(|x| {
                x.data.as_ref().and_then(|d| d.get("name")).and_then(|v| v.as_str())
                    == Some("cible_du_test")
            })
            .unwrap_or_else(|| panic!("{etiquette}: cible_du_test absent des résultats — rendus: {rendus:?}"));
        let (debut, fin) = match (
            hit.data.as_ref().and_then(|d| d.get("start_line")).and_then(|v| v.as_i64()),
            hit.data.as_ref().and_then(|d| d.get("end_line")).and_then(|v| v.as_i64()),
        ) {
            (Some(a), Some(b)) => (a as usize, b as usize),
            autre => panic!("{etiquette}: lignes absentes des données: {autre:?}"),
        };
        let extrait = lignes(SOURCE, debut, fin);
        assert!(
            extrait.contains("pollinisation") && extrait.contains("fn cible_du_test"),
            "{etiquette}: les lignes {debut}..{fin} du fichier ne portent pas la cible:\n{extrait}"
        );

        // 2. Par le NOM (la touche sur le titre) : même chose entière, mêmes
        // lignes exactes — une touche titre n'a pas d'extrait à elle, elle
        // désigne la définition et son étendue déclarée.
        let r = Catalog::rechercher(&cat, SCOPE, "cible_du_test", options_bm25()).expect("par le nom");
        let hit = r
            .results
            .iter()
            .find(|x| {
                x.data.as_ref().and_then(|d| d.get("name")).and_then(|v| v.as_str())
                    == Some("cible_du_test")
            })
            .unwrap_or_else(|| panic!("{etiquette}: cible_du_test introuvable par son nom"));
        let (d2, f2) = (
            hit.data.as_ref().and_then(|d| d.get("start_line")).and_then(|v| v.as_i64()).unwrap() as usize,
            hit.data.as_ref().and_then(|d| d.get("end_line")).and_then(|v| v.as_i64()).unwrap() as usize,
        );
        assert_eq!((d2, f2), (debut, fin), "{etiquette}: le nom et les mots désignent les mêmes lignes");
        (debut, fin)
    };

    let sans_boost = garanties("sans boost");

    // 3. Le boost ne touche pas les positions : mêmes lignes, et la
    // définition nommée passe en tête de la recherche par nom.
    scope_au_boost(&cat, Some(2.0));
    let avec_boost = garanties("boost 2,0");
    assert_eq!(sans_boost, avec_boost, "le boost ne déplace aucune ligne");

    let r = Catalog::rechercher(&cat, SCOPE, "cible_du_test", options_bm25()).expect("nom, boost");
    let premier = r.results.first().and_then(|x| x.data.as_ref()).and_then(|d| d.get("name")).and_then(|v| v.as_str());
    assert_eq!(premier, Some("cible_du_test"), "au boost 2,0 la définition nommée est première");
}

#[test]
fn un_document_garde_ses_lignes_dans_son_corps() {
    let mut catalog = base();
    let mut fields = HashMap::new();
    fields.insert("title".into(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("body".into(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
    let config = EntityConfig { fields, ..Default::default() };
    catalog.register_entity("Note", config).unwrap();

    let body = "Premier paragraphe, rien d'utile.\n\
                Deuxième ligne encore neutre.\n\
                Le savoir est ici : la ruche hiverne sous le givre.\n\
                Dernière ligne, neutre aussi.";
    let mut data = BTreeMap::new();
    data.insert("title".to_string(), CypherValue::String("Grimoire des pollens".into()));
    data.insert("body".to_string(), CypherValue::String(body.into()));
    catalog.ingest_entities("Note", vec![data]).unwrap();
    let cat = Arc::new(Mutex::new(catalog));

    // Par les mots du corps : l'extrait rendu, relu aux lignes dites du
    // corps, porte le mot cherché.
    let r = Catalog::rechercher(&cat, "Note", "hiverne", options_bm25()).expect("mots du corps");
    let hit = r.results.first().expect("un résultat");
    let chunk = hit.chunk.as_ref().expect("un extrait");
    let extrait = lignes(body, chunk.start_line.max(1), chunk.end_line.max(1));
    assert!(
        extrait.contains("hiverne"),
        "les lignes {}..{} du corps ne portent pas le mot:\n{extrait}",
        chunk.start_line, chunk.end_line
    );

    // Par le titre : le document entier, et l'extrait n'invente rien — son
    // texte est un morceau du corps, pas du titre collé au corps.
    let r = Catalog::rechercher(&cat, "Note", "Grimoire des pollens", options_bm25()).expect("par le titre");
    let hit = r.results.first().expect("le document par son titre");
    assert_eq!(
        hit.data.as_ref().and_then(|d| d.get("title")).and_then(|v| v.as_str()),
        Some("Grimoire des pollens")
    );
    if let Some(chunk) = hit.chunk.as_ref() {
        assert!(
            body.contains(chunk.text.trim()),
            "l'extrait d'une touche titre doit rester un morceau du corps: {:?}",
            chunk.text
        );
    }
}
