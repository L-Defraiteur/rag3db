//! **La migration 1b** : une base de code de la forme d'avant (les morceaux et les vecteurs portés
//! par `Scope`) passe à la forme à versions (`ScopeContent`, dérivée par contenu, ses morceaux et ses
//! vecteurs), sans rien réembarquer de ce dont le texte n'a pas changé.
//!
//! Le contrat est celui de la page 29 (`docs/arbre-principal/13-11-2026/29-contrat-de-la-version-de-scope.md`,
//! § 5) et du plan du 12 novembre : une base à la fois, sur une copie d'abord ; le creux est refusé
//! avant tout effet ; seules les versions au texte changé restent à embarquer, en dette, que l'index
//! ordinaire solde ensuite.

use std::collections::BTreeMap;

use crate::config::ChunkingConfig;
use crate::Catalog;

/// Ce que la migration a fait, pour le rapport.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bilan {
    /// Les `Scope` rattachés à une version.
    pub occurrences: usize,
    /// Les `ScopeContent` posés.
    pub versions: usize,
    /// Les morceaux de `ScopeContent` posés.
    pub morceaux: usize,
    /// Par modèle (son slug) : les vecteurs repris d'un morceau d'avant au texte identique.
    pub vecteurs_repris: BTreeMap<String, usize>,
    /// Par modèle : les morceaux neufs sans vecteur d'avant, laissés en dette d'embarquement.
    pub morceaux_en_dette: BTreeMap<String, usize>,
}

/// **Migrer une base de code de la forme d'avant vers la forme à versions.** Le catalogue est ouvert
/// sur la base, son schéma de code enregistré (une base d'avant garde alors sa forme). Refuse, avant
/// tout effet, une base qui n'est pas de la forme d'avant, et une base qui porte du creux.
///
/// Les pas :
/// 1. **les refus d'abord** : la forme, puis le creux (déclaré sur `Scope` ou posé sur un morceau) ;
/// 2. **la bascule** : `Scope` devient occurrence (index public `ScopeContent`, sans morceaux ni
///    signaux) par la seule voie déclarée, et `ScopeContent` s'enregistre (la forme du binaire) ;
/// 3. **la dérivation de toutes les occurrences**, par le drain ordinaire, jusqu'au plein texte :
///    versions, liens `ScopeContent_DERIVED_FROM`, morceaux, plein texte ; aucun embarquement ;
/// 4. **le transfert des vecteurs**, modèle par modèle : un morceau neuf reprend le vecteur d'un
///    morceau d'avant d'une de ses occurrences, au même champ, au même rang, au même texte et au
///    même titre, et dont le marqueur dit qu'il a été calculé sur ce texte. L'index vectoriel du
///    modèle courant se rebâtit après ;
/// 5. **la vérification**, puis **l'estampille** de la forme, en dernier : une base interrompue
///    avant elle ne s'ouvre pas comme une base à versions.
///
/// Les morceaux et le plein texte d'avant (`Scope_Chunk`, `Lucivy_Scope`) restent sur le disque :
/// leur retrait est une décision à part, après validation.
pub fn migrer_vers_les_versions(catalog: &mut Catalog, scope_chunking: ChunkingConfig) -> Result<Bilan, String> {
    migrer_d_un_seul_lot(catalog, scope_chunking)
}

/// Les occurrences dérivées par lot, et les versions dont on transfère les vecteurs par lot : la
/// mémoire d'une migration reste bornée (la copie de `code-rag3db-depot`, 14 novembre : un lot
/// unique a dépassé 32 Gio en trois minutes).
pub const LOT: usize = 2000;

/// [`migrer_vers_les_versions`], au lot donné.
pub fn migrer_vers_les_versions_par_lots(catalog: &mut Catalog, scope_chunking: ChunkingConfig, lot: usize) -> Result<Bilan, String> {
    let _ = (catalog, scope_chunking, lot);
    Err("la migration par lots n'existe pas encore".into())
}

fn migrer_d_un_seul_lot(catalog: &mut Catalog, scope_chunking: ChunkingConfig) -> Result<Bilan, String> {
    use crate::code::{scope_form, ScopeForm, SCOPE, SCOPE_CONTENT, SCOPE_FORM_KEY, SCOPE_FORM_VERSIONS};
    let e = |e: crate::catalog::CatalogError| e.to_string();

    // 1. Les refus, avant tout effet.
    let forme = scope_form(catalog).map_err(e)?;
    if forme != ScopeForm::SansVersions {
        return Err(format!("la migration vers les versions prend une base de la forme d'avant ; celle-ci est de la forme {forme:?}"));
    }
    let avant = catalog.entity_config(SCOPE).cloned().ok_or_else(|| format!("« {SCOPE} » n'est pas enregistré : le schéma de code d'abord"))?;
    if avant.signals.sparse() {
        return Err(format!("« {SCOPE} » déclare le creux : son transfert sans calcul n'est pas prouvé, la migration refuse avant tout effet"));
    }
    let creux = compter(catalog, "MATCH (c:Scope_Chunk) WHERE c._sparse_hash IS NOT NULL AND c._sparse_hash <> '' RETURN count(c)")?;
    if creux > 0 {
        return Err(format!("{creux} morceaux de « {SCOPE} » portent du creux : son transfert sans calcul n'est pas prouvé, la migration refuse avant tout effet"));
    }
    let modeles = catalog.registered_embedding_models().map_err(e)?;

    // 2. La bascule : la forme du binaire.
    let (occurrence, contenu) = crate::code::scope_version_configs(crate::code::scope_config(scope_chunking.clone()), scope_chunking);
    catalog.basculer_l_index_public_par_migration(SCOPE, occurrence).map_err(e)?;
    catalog.register_entity(SCOPE_CONTENT, contenu).map_err(e)?;

    // 3. Toutes les occurrences dérivées, jusqu'au plein texte : aucun embarquement.
    catalog.rederiver_tout(SCOPE_CONTENT).map_err(e)?;
    let drain = catalog.drain_jusqu_a(crate::disponibilite::Disponibilites::RECHERCHE_TEXTE);
    if drain.failed > 0 {
        return Err(format!("la dérivation des versions a échoué ({} échecs) : {}", drain.failed, drain.warnings.join(" ; ")));
    }

    // 4. Les vecteurs, modèle par modèle, sous l'index du modèle courant retiré puis rebâti.
    let mut bilan = Bilan::default();
    let transfert = catalog.bulk_vector_index(&[SCOPE_CONTENT], |cat| -> Result<(BTreeMap<String, usize>, BTreeMap<String, usize>), String> {
        let mut repris = BTreeMap::new();
        let mut dette = BTreeMap::new();
        for modele in &modeles {
            let (n, reste) = transferer_les_vecteurs(cat, modele)?;
            repris.insert(modele.slug(), n);
            dette.insert(modele.slug(), reste);
        }
        Ok((repris, dette))
    }).map_err(e)??;
    (bilan.vecteurs_repris, bilan.morceaux_en_dette) = transfert;

    // 5. La vérification, puis l'estampille.
    bilan.occurrences = compter(catalog, "MATCH (s:Scope) RETURN count(s)")? as usize;
    let rattachees = compter(catalog, "MATCH (:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s:Scope) RETURN count(DISTINCT s)")? as usize;
    let liens = compter(catalog, "MATCH (:ScopeContent)-[r:ScopeContent_DERIVED_FROM]->(:Scope) RETURN count(r)")? as usize;
    if rattachees != bilan.occurrences || liens != bilan.occurrences {
        return Err(format!(
            "{} occurrences, {rattachees} rattachées à une version, {liens} liens : chaque occurrence doit avoir exactement une version ; la base n'est pas estampillée",
            bilan.occurrences
        ));
    }
    bilan.versions = compter(catalog, "MATCH (v:ScopeContent) RETURN count(v)")? as usize;
    bilan.morceaux = compter(catalog, "MATCH (c:ScopeContent_Chunk) RETURN count(c)")? as usize;
    catalog.persist_meta_key(SCOPE_FORM_KEY, SCOPE_FORM_VERSIONS).map_err(e)?;
    Ok(bilan)
}

fn compter(catalog: &Catalog, cypher: &str) -> Result<i64, String> {
    catalog
        .execute_raw(cypher)
        .map_err(|e| format!("{cypher} : {e}"))?
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_i64())
        .ok_or_else(|| format!("{cypher} : compte illisible"))
}

/// Les vecteurs d'un modèle, des morceaux d'avant vers les morceaux neufs. Rend le nombre de
/// vecteurs repris et le nombre de morceaux neufs restés sans vecteur (en dette).
fn transferer_les_vecteurs(catalog: &Catalog, modele: &crate::embedding_storage::EmbeddingModelEntry) -> Result<(usize, usize), String> {
    use crate::connection::CypherValue;
    use crate::embedding_storage::VectorStorage;
    let avant = VectorStorage::resolve("Scope_Chunk", modele);
    let neuf = VectorStorage::resolve("ScopeContent_Chunk", modele);
    // Le premier morceau d'avant compatible, par uuid d'occurrence : un choix déterministe quand
    // deux occurrences au même texte ont chacune leur vecteur.
    let cypher = format!(
        "MATCH (n:ScopeContent_Chunk)-[:ScopeContent_CHUNKED_FROM]->(:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s:Scope)<-[:Scope_CHUNKED_FROM]-(o:Scope_Chunk) \
         WHERE o._parent_field = n._parent_field AND o._index = n._index AND o._text_hash = n._text_hash \
           AND coalesce(o._title, '') = coalesce(n._title, '') \
           AND o.{col} IS NOT NULL AND o.{marque} = o._text_hash \
         RETURN n._uuid, n._text_hash, o.{col} ORDER BY n._uuid, s._uuid",
        col = avant.column,
        marque = avant.marker,
    );
    let lu = catalog.execute_raw(&cypher).map_err(|e| format!("les vecteurs d'avant de « {} » : {e}", modele.slug()))?;
    let mut items = Vec::new();
    let mut dernier: Option<String> = None;
    for r in &lu.rows {
        let Some(uuid) = r.first().and_then(|v| v.as_str()) else { continue };
        if dernier.as_deref() == Some(uuid) {
            continue;
        }
        dernier = Some(uuid.to_string());
        let mut item = std::collections::BTreeMap::new();
        item.insert("uuid".to_string(), CypherValue::String(uuid.to_string()));
        item.insert("hash".to_string(), r.get(1).cloned().unwrap_or(CypherValue::Null));
        item.insert("emb".to_string(), r.get(2).cloned().unwrap_or(CypherValue::Null));
        items.push(CypherValue::Map(item));
    }
    let repris = items.len();
    let (conn, dialect) = catalog.connexion_et_dialecte();
    crate::dataflow::record_nodes::write_vectors(conn.as_ref(), dialect.as_ref(), "ScopeContent_Chunk", &neuf.column, &neuf.marker, items)
        .map_err(|e| format!("l'écriture des vecteurs de « {} » : {e}", modele.slug()))?;
    let reste = compter(catalog, &format!("MATCH (c:ScopeContent_Chunk) WHERE c.{} IS NULL OR c.{} IS NULL OR c.{} <> c._text_hash RETURN count(c)", neuf.column, neuf.marker, neuf.marker))?;
    Ok((repris, reste as usize))
}

/// **Migrer une base par son chemin**, hors hôte : la base (`<nom>.rag3db`), son plein texte en
/// fichiers à côté (`<nom>.rag3db.fts`), l'extension vectorielle du processus. Le catalogue s'ouvre
/// avec un embarqueur qui **refuse tout appel**, au nom et à la dimension du premier modèle
/// enregistré dans la base : une migration qui voudrait embarquer échoue au lieu de calculer.
/// À jouer sur une copie, sans autre ouvreur.
#[cfg(feature = "rag3db-native")]
pub fn migrer_la_base(chemin: &std::path::Path) -> Result<Bilan, String> {
    use crate::connection::DbConnection;
    use crate::{CatalogConfig, Rag3dbConnection};
    let ouvrir = || -> Result<Rag3dbConnection, String> {
        let conn = Rag3dbConnection::new(chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        let extension = crate::backend::vector_extension_of_this_process()?;
        conn.execute(&format!("LOAD EXTENSION {}", crate::connection::cypher_path_literal(&extension)))
            .map_err(|e| format!("l'extension vectorielle : {e}"))?;
        Ok(conn)
    };
    let fts = crate::fts_handle::FtsStorage::Files { base_path: format!("{}.fts", chemin.display()) };
    // Les modèles de la base, lus sans modèle ni initialisation.
    let modeles = Catalog::without_embedding_model(Box::new(ouvrir()?), CatalogConfig::default())
        .registered_embedding_models()
        .map_err(|e| format!("les modèles enregistrés : {e}"))?;
    let modele = modeles.first().ok_or("aucun modèle dense enregistré dans la base : rien à transférer, la migration ordinaire suffit")?;
    let refus = Box::new(Refus { nom: modele.name.clone(), dim: modele.dim });
    let mut catalog = Catalog::new(Box::new(ouvrir()?), refus, CatalogConfig { embedding_dim: modele.dim, ..Default::default() });
    catalog.set_fts_storage(fts);
    catalog.initialize().map_err(|e| format!("l'ouverture du catalogue : {e}"))?;
    crate::code::register_code_schema(&mut catalog, crate::code::default_scope_chunking()).map_err(|e| format!("le schéma de code : {e}"))?;
    migrer_vers_les_versions(&mut catalog, crate::code::default_scope_chunking())
}

/// L'embarqueur d'une migration : le nom et la dimension d'un modèle de la base, et un refus à
/// chaque appel. La migration ne calcule rien ; un appel serait un défaut, qu'il faut voir.
#[cfg(feature = "rag3db-native")]
struct Refus {
    nom: String,
    dim: usize,
}

#[cfg(feature = "rag3db-native")]
impl crate::embedder::Embedder for Refus {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, crate::embedder::EmbedError> {
        Err(crate::embedder::EmbedError::ProviderError(format!(
            "la migration vers les versions n'embarque rien ; {} textes demandés à « {} »",
            texts.len(),
            self.nom
        )))
    }
    fn dim(&self) -> usize {
        self.dim
    }
    fn name(&self) -> &str {
        &self.nom
    }
}
