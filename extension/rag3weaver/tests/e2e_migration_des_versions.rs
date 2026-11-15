//! **La migration 1b** : une base de code de la forme d'avant passe à la forme à versions, sans
//! réembarquer ce dont le texte n'a pas changé. Fixtures synthétiques seulement ; aucune base de
//! service n'est lue.
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::path::Path;
use std::sync::{Arc, Mutex};

use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema, register_code_schema_sans_versions, scope_form, ScopeForm};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::DbConnection;
use rag3weaver::embedder::{EmbedError, Embedder, HashEmbedder};
use rag3weaver::migration_des_versions::migrer_vers_les_versions;
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

/// Un catalogue sur une base et un plein texte en fichiers, l'embarqueur compté.
fn ouvrir(dir: &Path) -> (Catalog, Arc<Mutex<Vec<String>>>) {
    let conn = Rag3dbConnection::new(dir.join("code.rag3db")).unwrap();
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
    let texts = Arc::new(Mutex::new(Vec::new()));
    let mut cat = Catalog::new(Box::new(conn), Box::new(Compteur { inner: HashEmbedder::new(8), texts: texts.clone() }), CatalogConfig {
        allow_mock_embedder: true, embedding_dim: 8, ..Default::default()
    });
    cat.set_fts_storage(rag3weaver::fts_handle::FtsStorage::Files { base_path: dir.join("code.rag3db.fts").to_string_lossy().into_owned() });
    cat.initialize().unwrap();
    (cat, texts)
}

fn compte(cat: &Catalog, cypher: &str) -> i64 {
    cat.execute_raw(cypher).unwrap_or_else(|e| panic!("{cypher} : {e}")).rows[0][0].as_i64().unwrap()
}

const PARTAGEE: &str = "pub fn partagee() {\n    let marque_contenu_partage = 41;\n    let _ = marque_contenu_partage + 1;\n}\n";
const SEULE: &str = "pub fn seule() {\n    let marque_seule = 7;\n    let _ = marque_seule * 2;\n}\n";

/// Les occurrences d'une base, telles qu'un lecteur les voit : uuid, fichier, lignes.
fn occurrences(cat: &Catalog) -> Vec<(String, String, i64, i64)> {
    cat.execute_raw("MATCH (s:Scope) RETURN s._uuid, s.file_path, s.start_line, s.end_line ORDER BY s._uuid")
        .unwrap()
        .rows
        .iter()
        .map(|r| (r[0].as_str().unwrap().to_string(), r[1].as_str().unwrap_or("").to_string(), r[2].as_i64().unwrap_or(-1), r[3].as_i64().unwrap_or(-1)))
        .collect()
}

/// **Une base d'avant migre vers les versions sans appeler le modèle** (T3, T5 du plan du 12) :
/// la même fonction dans `a.rs` et `b.rs` donne une version, une autre fonction la sienne ; les
/// occurrences gardent leur uuid et leurs lignes, chacune rattachée à une seule version ; chaque
/// morceau neuf a son vecteur, repris d'un morceau d'avant au texte identique, et aucun texte ne
/// passe par l'embarqueur. Le plein texte compte une ligne par document. Rouverte par le schéma
/// de production, la base est à versions, et une occurrence de plus ne réembarque rien.
#[test]
#[ignore]
fn une_base_d_avant_migre_vers_les_versions_sans_appel_au_modele() {
    let dir = tempfile::tempdir().unwrap();
    let avant = {
        let (mut cat, _) = ouvrir(dir.path());
        register_code_schema_sans_versions(&mut cat, default_scope_chunking()).unwrap();
        let source = Snapshot::new("ancre", [
            ("a.rs".to_string(), PARTAGEE.to_string()),
            ("b.rs".to_string(), format!("\n\n{PARTAGEE}")),
            ("c.rs".to_string(), SEULE.to_string()),
        ]);
        assert_eq!(cat.ingest_code(&analyze_source(&source).unwrap()).unwrap().failed, 0);
        assert_eq!(scope_form(&cat).unwrap(), ScopeForm::SansVersions, "le décor : une base d'avant");
        let col = cat.vector_storage("Scope_Chunk").unwrap().column;
        assert!(compte(&cat, &format!("MATCH (c:Scope_Chunk) WHERE c.{col} IS NOT NULL RETURN count(c)")) > 0, "le décor : des vecteurs d'avant");
        occurrences(&cat)
    };
    let (mut cat, texts) = ouvrir(dir.path());
    register_code_schema(&mut cat, default_scope_chunking()).unwrap();
    assert_eq!(scope_form(&cat).unwrap(), ScopeForm::SansVersions, "la base d'avant se rouvre comme avant");
    texts.lock().unwrap().clear();
    let bilan = migrer_vers_les_versions(&mut cat, default_scope_chunking()).unwrap_or_else(|e| panic!("la migration refuse : {e}"));
    assert_eq!(texts.lock().unwrap().len(), 0, "la migration a appelé le modèle : {:?}", texts.lock().unwrap());
    assert_eq!(scope_form(&cat).unwrap(), ScopeForm::Versions, "l'estampille n'est pas posée");
    assert_eq!(occurrences(&cat), avant, "les occurrences ont changé (uuid, fichier ou lignes)");
    assert_eq!(compte(&cat, "MATCH (v:ScopeContent) WHERE v.name = 'partagee' RETURN count(v)"), 1, "deux occurrences identiques, une version");
    let scopes = compte(&cat, "MATCH (s:Scope) RETURN count(s)");
    assert_eq!(compte(&cat, "MATCH (:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s:Scope) RETURN count(DISTINCT s)"), scopes, "une occurrence sans version");
    assert_eq!(compte(&cat, "MATCH (:ScopeContent)-[r:ScopeContent_DERIVED_FROM]->(:Scope) RETURN count(r)"), scopes, "une occurrence à deux versions");
    let stockage = cat.vector_storage("ScopeContent_Chunk").unwrap();
    let morceaux = compte(&cat, "MATCH (c:ScopeContent_Chunk) RETURN count(c)");
    assert!(morceaux > 0, "aucun morceau de version");
    assert_eq!(
        compte(&cat, &format!("MATCH (c:ScopeContent_Chunk) WHERE c.{} IS NOT NULL AND c.{} = c._text_hash RETURN count(c)", stockage.column, stockage.marker)),
        morceaux,
        "un morceau de version sans son vecteur repris"
    );
    assert_eq!(bilan.versions as i64, compte(&cat, "MATCH (v:ScopeContent) RETURN count(v)"), "{bilan:?}");
    assert_eq!(bilan.occurrences as i64, scopes, "{bilan:?}");
    assert!(bilan.morceaux_en_dette.values().all(|n| *n == 0), "{bilan:?}");
    for (entite, documents, lignes) in cat.comptes_du_plein_texte() {
        assert_eq!(documents, lignes, "« {entite} » : {documents} documents pour {lignes} lignes");
    }
    drop(cat);
    let (mut cat, texts) = ouvrir(dir.path());
    register_code_schema(&mut cat, default_scope_chunking()).unwrap_or_else(|e| panic!("la base migrée ne se rouvre pas : {e}"));
    assert_eq!(scope_form(&cat).unwrap(), ScopeForm::Versions);
    let source = Snapshot::new("ancre", [("d.rs".to_string(), format!("\n{PARTAGEE}"))]);
    assert_eq!(cat.ingest_code(&analyze_source(&source).unwrap()).unwrap().failed, 0);
    assert!(!texts.lock().unwrap().iter().any(|t| t.contains("marque_contenu_partage")), "une occurrence de plus réembarque une version migrée");
}

/// **Une base déjà à versions refuse la migration, sans rien changer.**
#[test]
#[ignore]
fn une_base_a_versions_refuse_la_migration() {
    let dir = tempfile::tempdir().unwrap();
    let (mut cat, _) = ouvrir(dir.path());
    register_code_schema(&mut cat, default_scope_chunking()).unwrap();
    let source = Snapshot::new("ancre", [("a.rs".to_string(), PARTAGEE.to_string())]);
    assert_eq!(cat.ingest_code(&analyze_source(&source).unwrap()).unwrap().failed, 0);
    let versions = compte(&cat, "MATCH (v:ScopeContent) RETURN count(v)");
    let refus = migrer_vers_les_versions(&mut cat, default_scope_chunking()).expect_err("une base à versions migre encore");
    assert!(refus.contains("forme"), "le refus ne nomme pas la forme : {refus}");
    assert_eq!(compte(&cat, "MATCH (v:ScopeContent) RETURN count(v)"), versions);
}

/// **Une base se migre par son chemin, hors hôte** (l'entrée de la copie réelle) : le catalogue
/// fermé, `migrer_la_base` ouvre la base et son plein texte en fichiers, et migre sans embarqueur
/// réel (le sien refuse tout appel). Rouverte par le schéma de production, la base est à versions,
/// ses occurrences intactes, chaque morceau avec son vecteur.
#[test]
#[ignore]
fn une_base_se_migre_par_son_chemin_hors_hote() {
    let dir = tempfile::tempdir().unwrap();
    let avant = {
        let (mut cat, _) = ouvrir(dir.path());
        register_code_schema_sans_versions(&mut cat, default_scope_chunking()).unwrap();
        let source = Snapshot::new("ancre", [
            ("a.rs".to_string(), PARTAGEE.to_string()),
            ("b.rs".to_string(), format!("\n\n{PARTAGEE}")),
            ("c.rs".to_string(), SEULE.to_string()),
        ]);
        assert_eq!(cat.ingest_code(&analyze_source(&source).unwrap()).unwrap().failed, 0);
        occurrences(&cat)
    };
    let bilan = rag3weaver::migration_des_versions::migrer_la_base(&dir.path().join("code.rag3db"))
        .unwrap_or_else(|e| panic!("la migration par le chemin refuse : {e}"));
    assert!(bilan.vecteurs_repris.values().sum::<usize>() > 0, "{bilan:?}");
    assert!(bilan.morceaux_en_dette.values().all(|n| *n == 0), "{bilan:?}");
    let (mut cat, texts) = ouvrir(dir.path());
    register_code_schema(&mut cat, default_scope_chunking()).unwrap_or_else(|e| panic!("la base migrée ne se rouvre pas : {e}"));
    assert_eq!(scope_form(&cat).unwrap(), ScopeForm::Versions);
    assert_eq!(occurrences(&cat), avant, "les occurrences ont changé");
    let stockage = cat.vector_storage("ScopeContent_Chunk").unwrap();
    let morceaux = compte(&cat, "MATCH (c:ScopeContent_Chunk) RETURN count(c)");
    assert!(morceaux > 0);
    assert_eq!(compte(&cat, &format!("MATCH (c:ScopeContent_Chunk) WHERE c.{} = c._text_hash RETURN count(c)", stockage.marker)), morceaux);
    for (entite, documents, lignes) in cat.comptes_du_plein_texte() {
        assert_eq!(documents, lignes, "« {entite} » : {documents} documents pour {lignes} lignes");
    }
    assert_eq!(texts.lock().unwrap().len(), 0, "la réouverture a embarqué : {:?}", texts.lock().unwrap());
}

/// **La migration se fait par lots bornés, au même résultat** : des lots d'une seule occurrence
/// donnent les mêmes versions, les mêmes liens et un vecteur repris par morceau, sans appel au modèle.
#[test]
#[ignore]
fn une_migration_par_lots_d_une_occurrence_donne_le_meme_resultat() {
    let dir = tempfile::tempdir().unwrap();
    {
        let (mut cat, _) = ouvrir(dir.path());
        register_code_schema_sans_versions(&mut cat, default_scope_chunking()).unwrap();
        let source = Snapshot::new("ancre", [
            ("a.rs".to_string(), PARTAGEE.to_string()),
            ("b.rs".to_string(), format!("\n\n{PARTAGEE}")),
            ("c.rs".to_string(), SEULE.to_string()),
        ]);
        assert_eq!(cat.ingest_code(&analyze_source(&source).unwrap()).unwrap().failed, 0);
    }
    let (mut cat, texts) = ouvrir(dir.path());
    register_code_schema(&mut cat, default_scope_chunking()).unwrap();
    texts.lock().unwrap().clear();
    let bilan = rag3weaver::migration_des_versions::migrer_vers_les_versions_par_lots(&mut cat, default_scope_chunking(), 1)
        .unwrap_or_else(|e| panic!("la migration par lots refuse : {e}"));
    assert_eq!(texts.lock().unwrap().len(), 0, "la migration a appelé le modèle");
    let scopes = compte(&cat, "MATCH (s:Scope) RETURN count(s)");
    assert!(scopes >= 3);
    assert_eq!(compte(&cat, "MATCH (:ScopeContent)-[r:ScopeContent_DERIVED_FROM]->(:Scope) RETURN count(r)"), scopes);
    assert_eq!(compte(&cat, "MATCH (v:ScopeContent) WHERE v.name = 'partagee' RETURN count(v)"), 1);
    let stockage = cat.vector_storage("ScopeContent_Chunk").unwrap();
    let morceaux = compte(&cat, "MATCH (c:ScopeContent_Chunk) RETURN count(c)");
    assert_eq!(compte(&cat, &format!("MATCH (c:ScopeContent_Chunk) WHERE c.{} = c._text_hash RETURN count(c)", stockage.marker)), morceaux);
    assert_eq!(bilan.vecteurs_repris.values().sum::<usize>() as i64, morceaux, "{bilan:?}");
    // **Rien ne tient en mémoire plus qu'un lot** : la borne se lit dans le bilan.
    assert_eq!(bilan.plus_gros_lot, 1, "un lot de plus d'une unité a été tenu en mémoire : {bilan:?}");
}

/// **Une base migrée par 1b monte aux générations par son chemin** (la bascule des bases de Lucie,
/// `migration generations --base`) : la forme passe de « versions » à « generations », la génération
/// 0 de `backend` est posée, les occurrences restent les mêmes, et la base se rouvre par le schéma de
/// production à la forme des générations. Les lectures voient ensuite le rang en attente (1), que le
/// repassage de l'écrivain écrit.
#[test]
#[ignore]
fn une_base_a_versions_monte_aux_generations_par_son_chemin() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("code.rag3db");
    let avant = {
        let (mut cat, _) = ouvrir(dir.path());
        register_code_schema_sans_versions(&mut cat, default_scope_chunking()).unwrap();
        let source = Snapshot::new("ancre", [("a.rs".to_string(), PARTAGEE.to_string()), ("c.rs".to_string(), SEULE.to_string())]);
        assert_eq!(cat.ingest_code(&analyze_source(&source).unwrap()).unwrap().failed, 0);
        occurrences(&cat)
    };
    let refus = rag3weaver::migration_des_versions::migrer_la_base_aux_generations(&base).expect_err("une base d'avant monte aux générations");
    assert!(refus.contains("forme"), "le refus d'une base d'avant ne nomme pas la forme : {refus}");
    rag3weaver::migration_des_versions::migrer_la_base(&base).unwrap_or_else(|e| panic!("1b refuse : {e}"));
    let bilan = rag3weaver::migration_des_versions::migrer_la_base_aux_generations(&base).unwrap_or_else(|e| panic!("la montée refuse : {e}"));
    assert_eq!(
        bilan,
        rag3weaver::migration_des_versions::BilanDesGenerations {
            forme_avant: "versions".into(),
            forme_apres: "generations".into(),
            generation: Some(("backend".into(), 0)),
            rang_lu: Some(1),
        }
    );
    let (mut cat, texts) = ouvrir(dir.path());
    register_code_schema(&mut cat, default_scope_chunking()).unwrap_or_else(|e| panic!("la base montée ne se rouvre pas : {e}"));
    assert_eq!(scope_form(&cat).unwrap(), ScopeForm::Generations);
    assert_eq!(occurrences(&cat), avant, "les occurrences ont changé");
    assert_eq!(texts.lock().unwrap().len(), 0, "la réouverture a embarqué");
    drop(cat);
    let refus = rag3weaver::migration_des_versions::migrer_la_base_aux_generations(&base).expect_err("une base aux générations remonte");
    assert!(refus.contains("forme"), "le second passage ne nomme pas la forme : {refus}");
}

/// **Une base tenue par un autre écrivain vivant ne monte pas** : sa marque `<base>.ecrivain` (ici,
/// celle du processus parent, vivant) vaut un refus qui dit qui la tient, avant toute ouverture.
#[test]
#[ignore]
fn une_base_tenue_par_un_autre_ne_monte_pas_aux_generations() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("code.rag3db");
    {
        let (mut cat, _) = ouvrir(dir.path());
        register_code_schema(&mut cat, default_scope_chunking()).unwrap();
    }
    let marque = serde_json::json!({
        "pid": std::os::unix::process::parent_id(), "started": null, "role": "hote", "outil": "un hôte de test", "depuis_ms": 0
    });
    std::fs::write(rag3weaver::marque_d_ecrivain::chemin(&base), marque.to_string()).unwrap();
    let refus = rag3weaver::migration_des_versions::migrer_la_base_aux_generations(&base).expect_err("une base tenue monte");
    assert!(refus.contains("tenue"), "le refus ne dit pas que la base est tenue : {refus}");
}
