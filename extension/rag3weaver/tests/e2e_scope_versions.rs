//! Versions de contenu sur l'ancre : un contenu partagé, des occurrences distinctes.
//! Témoins du lot 1a ; aucun accès aux bases de service.
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::sync::{Arc, Mutex};

use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::DbConnection;
use rag3weaver::embedder::{EmbedError, Embedder, HashEmbedder};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

struct Compteur {
    inner: HashEmbedder,
    texts: Arc<Mutex<Vec<String>>>,
}

impl Embedder for Compteur {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        self.texts.lock().unwrap().extend(texts.iter().cloned());
        self.inner.embed(texts)
    }
    fn dim(&self) -> usize { self.inner.dim() }
    fn is_mock(&self) -> bool { true }
    fn name(&self) -> &str { self.inner.name() }
}

fn catalogue() -> (Catalog, Arc<Mutex<Vec<String>>>) {
    let conn = Rag3dbConnection::in_memory().expect("base privée en mémoire");
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
    let texts = Arc::new(Mutex::new(Vec::new()));
    let embedder = Compteur { inner: HashEmbedder::new(8), texts: texts.clone() };
    let mut cat = Catalog::new(Box::new(conn), Box::new(embedder), CatalogConfig {
        allow_mock_embedder: true, embedding_dim: 8, ..Default::default()
    });
    cat.initialize().unwrap();
    register_code_schema(&mut cat, default_scope_chunking()).unwrap();
    // Déclaration privée du ticket 1a : les API communes, aucun changement du
    // schéma code par défaut ni activation d'une base existante.
    // docs/arbre-principal/tickets/2026-11-11-deux-occurrences-identiques-reembarquent-le-meme-contenu.md
    assert_eq!(cat.execute_raw("MATCH (s:Scope) RETURN count(s)").unwrap().rows[0][0].as_i64(), Some(0));
    let original = rag3weaver::code::scope_config(default_scope_chunking());
    let mut occurrence = original.clone();
    occurrence.search_index = Some("ScopeContent".into());
    occurrence.chunked = Some(false);
    // L'occurrence fournit les données et les positions ; seul ScopeContent
    // porte ici les index physiques, avec les signaux du schéma d'origine.
    occurrence.signals = rag3weaver::search::SearchSignals::NONE;
    cat.register_entity("Scope", occurrence).unwrap();
    let names = ["name", "signature", "content", "docstring", "scope_type", "language"];
    let fields = names.iter().map(|name| (name.to_string(), original.fields[*name].clone())).collect();
    let render = names.iter().map(|name| (name.to_string(), format!("{{{{ root.{name} }}}}"))).collect();
    cat.register_entity("ScopeContent", rag3weaver::EntityConfig {
        fields, chunking: default_scope_chunking(), signals: original.signals,
        derived: Some(rag3weaver::config::DerivedConfig {
            from: "Scope".into(), identity: rag3weaver::config::DerivedIdentity::Content { format: "code-scope-v1".into() },
            gather: vec![], render,
        }), ..Default::default()
    }).unwrap();
    (cat, texts)
}

const FONCTION: &str = "pub fn partagee() {\n    let marque_contenu_partage = 41;\n    let _ = marque_contenu_partage + 1;\n}\n";

/// Même fonction dans deux fichiers et à deux positions : on partage son contenu,
/// jamais l'identité ni les coordonnées des occurrences.
#[test]
#[ignore]
fn deux_occurrences_partagent_un_embarquement_et_gardent_leurs_positions() {
    let (mut cat, texts) = catalogue();
    let source = Snapshot::new("ancre", [
        ("a.rs".to_string(), FONCTION.to_string()),
        ("b.rs".to_string(), format!("\n\n{FONCTION}")),
    ]);
    cat.ingest_code(&analyze_source(&source).unwrap()).unwrap();
    let r = cat.execute_raw("MATCH (s:Scope) WHERE s.name = 'partagee' RETURN s.file_path, s.start_line ORDER BY s.file_path").unwrap();
    assert_eq!(r.rows.len(), 2, "les occurrences ont été fusionnées : {:?}", r.rows);
    assert_eq!(r.rows[0][0].as_str(), Some("a.rs"));
    assert_eq!(r.rows[1][0].as_str(), Some("b.rs"));
    let a = r.rows[0][1].as_i64().unwrap();
    let b = r.rows[1][1].as_i64().unwrap();
    assert_eq!(b - a, 2, "les positions de chaque occurrence doivent être conservées");
    let vus = texts.lock().unwrap();
    let embarques: Vec<_> = vus.iter().filter(|s| s.contains("marque_contenu_partage")).collect();
    assert!(!embarques.is_empty(), "le compteur n'a vu aucun contenu de fonction");
    assert_eq!(embarques.len(), 1, "le contenu identique de deux occurrences est réembarqué : {embarques:?}");
    drop(vus);
    assert_eq!(cat.execute_raw("MATCH (v:ScopeContent) WHERE v.name = 'partagee' RETURN count(v)").unwrap().rows[0][0].as_i64(), Some(1));
    assert_eq!(cat.execute_raw("MATCH (c:Scope_Chunk) RETURN count(c)").unwrap().rows[0][0].as_i64(), Some(0));
    let moved = Snapshot::new("ancre", [
        ("a.rs".to_string(), format!("\n\n\n\n{FONCTION}")),
        ("b.rs".to_string(), format!("\n\n{FONCTION}")),
    ]);
    assert_eq!(cat.ingest_code(&analyze_source(&moved).unwrap()).unwrap().failed, 0);
    assert_eq!(texts.lock().unwrap().iter().filter(|text| text.contains("marque_contenu_partage")).count(), 1, "le déplacement ne réembarque pas");

}

/// La docstring est un champ de contenu distinct : une déduplication par corps seul
/// ne doit pas réutiliser la documentation d'une autre occurrence.
#[test]
#[ignore]
fn deux_docstrings_distinctes_sont_conservees_et_embarquees() {
    let (mut cat, texts) = catalogue();
    let source = Snapshot::new("ancre", [
        ("a.rs".to_string(), format!("/// documentation_alpha_unique\n{FONCTION}")),
        ("b.rs".to_string(), format!("/// documentation_beta_unique\n{FONCTION}")),
    ]);
    cat.ingest_code(&analyze_source(&source).unwrap()).unwrap();
    let r = cat.execute_raw("MATCH (s:Scope) WHERE s.name = 'partagee' RETURN s.file_path, s.docstring ORDER BY s.file_path").unwrap();
    assert_eq!(r.rows.len(), 2, "{:?}", r.rows);
    assert!(r.rows[0][1].as_str().unwrap().contains("documentation_alpha_unique"));
    assert!(r.rows[1][1].as_str().unwrap().contains("documentation_beta_unique"));
    let vus = texts.lock().unwrap();
    for mot in ["documentation_alpha_unique", "documentation_beta_unique"] {
        assert!(vus.iter().any(|t| t.contains(mot)), "documentation jamais embarquée : {mot}");
    }
}


#[test]
#[ignore]
fn le_schema_prive_garde_la_recherche_scope_et_les_aretes_des_occurrences() {
    use rag3weaver::search::{Consistency, ResultMode, SearchOptions, SearchSignals};
    use rag3weaver::filter::{FilterCondition, FilterValue};
    let (mut cat, _) = catalogue();
    let code = format!("fn aide() {{}}\n{}", FONCTION.replace("let marque_contenu_partage", "aide();\n    let marque_contenu_partage"));
    let snapshot = Snapshot::new("ancre", [
        ("a.rs".to_string(), code.clone()),
        ("b.rs".to_string(), format!("\n\n{code}")),
    ]);
    assert_eq!(cat.ingest_code(&analyze_source(&snapshot).unwrap()).unwrap().failed, 0);
    let edges = cat.execute_raw("MATCH (a:Scope)-[:CONSUMES]->(b:Scope) WHERE a.name = 'partagee' AND b.name = 'aide' RETURN a.file_path, b.file_path ORDER BY a.file_path").unwrap().rows;
    assert_eq!(edges.len(), 2, "{edges:?}");
    assert_eq!(edges[0][0].as_str(), Some("a.rs"));
    assert_eq!(edges[0][1].as_str(), Some("a.rs"));
    assert_eq!(edges[1][0].as_str(), Some("b.rs"));
    assert_eq!(edges[1][1].as_str(), Some("b.rs"));
    let shared = Arc::new(Mutex::new(cat));
    for signal in [SearchSignals::VECTOR, SearchSignals::BM25] {
        for mode in [ResultMode::Aggregated, ResultMode::Detailed, ResultMode::SourceResolved] {
            let result = Catalog::rechercher(&shared, "Scope", "marque_contenu_partage", SearchOptions {
                signals: Some(signal), result_mode: mode, consistency: Consistency::Immediate, strict_signals: true,
                filter_condition: Some(FilterCondition::Field { key: "name".into(), value: FilterValue::Direct("partagee".into()) }),
                ..Default::default()
            }).unwrap();
            assert_eq!(result.results.len(), 2, "signal={signal:?}, mode={mode:?} : {:?}", result.meta);
            let mut files = Vec::new();
            for hit in result.results {
                assert_eq!(hit.entity.as_deref(), Some("Scope"));
                let data = hit.data.unwrap();
                files.push(data["file_path"].as_str().unwrap().to_string());
                assert!(data["start_line"].as_i64().unwrap() > 0);
            }
            files.sort();
            assert_eq!(files, vec!["a.rs", "b.rs"]);
        }
    }
}

// ─── Le schéma de production (lot 1a, 13 novembre 2026) ──────────────────────
// docs/arbre-principal/13-11-2026/29-contrat-de-la-version-de-scope.md

fn compteur() -> (Compteur, Arc<Mutex<Vec<String>>>) {
    let texts = Arc::new(Mutex::new(Vec::new()));
    (Compteur { inner: HashEmbedder::new(8), texts: texts.clone() }, texts)
}

fn ouvrir(conn: Rag3dbConnection, embedder: Compteur) -> Catalog {
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
    let mut cat = Catalog::new(Box::new(conn), Box::new(embedder), CatalogConfig {
        allow_mock_embedder: true, embedding_dim: 8, ..Default::default()
    });
    cat.initialize().unwrap();
    cat
}

/// Une base neuve, par le seul schéma de production : aucune déclaration privée.
fn catalogue_de_production() -> (Catalog, Arc<Mutex<Vec<String>>>) {
    let (embedder, texts) = compteur();
    let mut cat = ouvrir(Rag3dbConnection::in_memory().expect("base privée en mémoire"), embedder);
    register_code_schema(&mut cat, default_scope_chunking()).unwrap();
    (cat, texts)
}

#[test]
#[ignore]
fn le_schema_de_production_partage_le_contenu_de_deux_fichiers() {
    let (mut cat, texts) = catalogue_de_production();
    let source = Snapshot::new("ancre", [
        ("a.rs".to_string(), FONCTION.to_string()),
        ("b.rs".to_string(), format!("\n\n{FONCTION}")),
    ]);
    assert_eq!(cat.ingest_code(&analyze_source(&source).unwrap()).unwrap().failed, 0);
    let r = cat.execute_raw("MATCH (s:Scope) WHERE s.name = 'partagee' RETURN s.file_path, s.start_line ORDER BY s.file_path").unwrap();
    assert_eq!(r.rows.len(), 2, "les occurrences ont été fusionnées : {:?}", r.rows);
    assert_eq!(r.rows[1][1].as_i64().unwrap() - r.rows[0][1].as_i64().unwrap(), 2, "chaque occurrence garde ses lignes");
    let embarques = texts.lock().unwrap().iter().filter(|s| s.contains("marque_contenu_partage")).count();
    assert_eq!(embarques, 1, "le schéma de production réembarque un contenu identique");
}

#[test]
#[ignore]
fn une_base_neuve_recoit_l_estampille_des_versions() {
    let (cat, _) = catalogue_de_production();
    assert_eq!(rag3weaver::code::scope_form(&cat).unwrap(), rag3weaver::code::ScopeForm::Versions);
    assert!(cat.entity_configs().contains_key(rag3weaver::code::SCOPE_CONTENT), "pas d'entité de contenu sur une base neuve");
}

/// Une base de la forme d'avant, peuplée ou vide : elle ouvre comme avant, garde
/// ses morceaux sur `Scope`, et se dit à migrer. Aucune bascule silencieuse.
#[test]
#[ignore]
fn une_base_d_avant_ouvre_comme_avant_et_se_dit_a_migrer() {
    for peuplee in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let chemin = dir.path().join("code.rag3db");
        {
            let (embedder, _) = compteur();
            let mut cat = ouvrir(Rag3dbConnection::new(&chemin).unwrap(), embedder);
            rag3weaver::code::register_code_schema_sans_versions(&mut cat, default_scope_chunking()).unwrap();
            if peuplee {
                let source = Snapshot::new("ancre", [("a.rs".to_string(), FONCTION.to_string())]);
                assert_eq!(cat.ingest_code(&analyze_source(&source).unwrap()).unwrap().failed, 0);
            }
        }
        let (embedder, texts) = compteur();
        let mut cat = ouvrir(Rag3dbConnection::new(&chemin).unwrap(), embedder);
        register_code_schema(&mut cat, default_scope_chunking())
            .unwrap_or_else(|e| panic!("peuplée={peuplee} : la base d'avant ne s'ouvre plus : {e}"));
        assert_eq!(rag3weaver::code::scope_form(&cat).unwrap(), rag3weaver::code::ScopeForm::SansVersions, "peuplée={peuplee}");
        assert!(!cat.entity_configs().contains_key(rag3weaver::code::SCOPE_CONTENT), "peuplée={peuplee} : bascule silencieuse");
        let source = Snapshot::new("ancre", [("b.rs".to_string(), format!("\n{FONCTION}"))]);
        assert_eq!(cat.ingest_code(&analyze_source(&source).unwrap()).unwrap().failed, 0, "peuplée={peuplee}");
        let morceaux = cat.execute_raw("MATCH (c:Scope_Chunk) RETURN count(c)").unwrap().rows[0][0].as_i64().unwrap();
        assert!(morceaux > 0, "peuplée={peuplee} : les morceaux restent sur Scope dans la forme d'avant");
        assert!(texts.lock().unwrap().iter().any(|t| t.contains("marque_contenu_partage")), "peuplée={peuplee}");
    }
}

/// La garde des comptes du plein texte compte les lignes de la table qui porte
/// l'index (`ScopeContent`), pas celles de l'occurrence : deux occurrences d'un
/// même contenu font un document, et ce n'est pas une écriture perdue.
/// Rouge, la garde rebâtissait l'index à chaque ouverture, et sa réécriture des
/// `Scope` gardait une occurrence retirée (e2e_code_sync, nom ambigu).
#[test]
#[ignore]
fn deux_occurrences_d_un_contenu_ne_sont_pas_une_ecriture_perdue() {
    let (mut cat, _) = catalogue_de_production();
    let source = Snapshot::new("ancre", [
        ("a.rs".to_string(), FONCTION.to_string()),
        ("b.rs".to_string(), format!("\n\n{FONCTION}")),
    ]);
    assert_eq!(cat.ingest_code(&analyze_source(&source).unwrap()).unwrap().failed, 0);
    let a_rebatir = cat.check_fts_counts().unwrap();
    assert!(a_rebatir.is_empty(), "la garde voit une écriture perdue là où deux occurrences partagent un contenu : {a_rebatir:?}");
    for (entite, documents, lignes) in cat.comptes_du_plein_texte() {
        assert_eq!(documents, lignes, "« {entite} » : {documents} documents pour {lignes} lignes");
    }
}

/// Le rebâti du plein texte en fichiers lit les lignes de la table qui porte
/// l'index : l'index de `ScopeContent` se refait depuis les versions, pas depuis
/// les occurrences `Scope` (deux occurrences d'un contenu, un document). Rouge,
/// il y rangeait les lignes et les décalages de `Scope`.
#[test]
#[ignore]
fn le_plein_texte_d_une_version_se_rebatit_depuis_les_versions() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("code.rag3db");
    let fts = dir.path().join("fts");
    let ouvrir_fichiers = || {
        let (embedder, _) = compteur();
        let conn = Rag3dbConnection::new(&base).unwrap();
        conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
        let mut cat = Catalog::new(Box::new(conn), Box::new(embedder), CatalogConfig {
            allow_mock_embedder: true, embedding_dim: 8, ..Default::default()
        });
        cat.set_fts_storage(rag3weaver::fts_handle::FtsStorage::Files { base_path: fts.to_string_lossy().into_owned() });
        cat.initialize().unwrap();
        register_code_schema(&mut cat, default_scope_chunking()).unwrap();
        cat
    };
    {
        let mut cat = ouvrir_fichiers();
        let source = Snapshot::new("ancre", [
            ("a.rs".to_string(), FONCTION.to_string()),
            ("b.rs".to_string(), format!("\n\n{FONCTION}")),
            ("c.rs".to_string(), "pub fn seule() -> u32 { 7 }\n".to_string()),
        ]);
        assert_eq!(cat.ingest_code(&analyze_source(&source).unwrap()).unwrap().failed, 0);
        cat.mark_fts_pending(&["Scope"]).unwrap();
    }
    let mut cat = ouvrir_fichiers();
    cat.verify_fts_files();
    let versions = cat.execute_raw("MATCH (v:ScopeContent) RETURN count(v)").unwrap().rows[0][0].as_i64().unwrap() as u64;
    let occurrences = cat.execute_raw("MATCH (s:Scope) RETURN count(s)").unwrap().rows[0][0].as_i64().unwrap() as u64;
    assert!(occurrences > versions, "le témoin veut un contenu partagé : {occurrences} occurrences, {versions} versions");
    let comptes = cat.comptes_du_plein_texte();
    let (_, documents, _) = comptes.iter().find(|(e, _, _)| e == "Scope").cloned().expect("le plein texte de Scope");
    assert_eq!(documents, versions, "l'index de ScopeContent rebâti porte {documents} documents pour {versions} versions ({occurrences} occurrences)");
}
