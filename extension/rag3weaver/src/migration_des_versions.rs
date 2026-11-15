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
    /// **Le plus gros lot tenu en mémoire**, en occurrences (à la dérivation) ou en versions (au
    /// transfert). Rien ne tient en mémoire plus qu'un lot : c'est la borne d'une passe de masse.
    pub plus_gros_lot: usize,
    /// Le plus grand nombre de lignes lues en une requête de transfert (un morceau neuf par
    /// occurrence de sa version).
    pub plus_de_lignes_lues: usize,
}

/// **Migrer une base de code de la forme d'avant vers la forme à versions.** Le catalogue est ouvert
/// sur la base, son schéma de code enregistré (une base d'avant garde alors sa forme). Refuse, avant
/// tout effet, une base qui n'est pas de la forme d'avant, et une base qui porte du creux.
///
/// Les pas :
/// 1. **les refus d'abord** : la forme, puis le creux (déclaré sur `Scope` ou posé sur un morceau) ;
/// 2. **la bascule** : `Scope` devient occurrence (index public `ScopeContent`, sans morceaux ni
///    signaux) par la seule voie déclarée, et `ScopeContent` s'enregistre (la forme du binaire) ;
/// 3. **la dérivation des occurrences par lots** ([`LOT`]), chacun drainé jusqu'au plein texte
///    avant le suivant :
///    versions, liens `ScopeContent_DERIVED_FROM`, morceaux, plein texte ; aucun embarquement ;
/// 4. **le transfert des vecteurs**, par lots de versions et modèle par modèle : un morceau neuf reprend le vecteur d'un
///    morceau d'avant d'une de ses occurrences, au même champ, au même rang, au même texte et au
///    même titre, et dont le marqueur dit qu'il a été calculé sur ce texte. L'index vectoriel du
///    modèle courant se rebâtit après ;
/// 5. **la vérification**, puis **l'estampille** de la forme, en dernier : une base interrompue
///    avant elle ne s'ouvre pas comme une base à versions.
///
/// Les morceaux et le plein texte d'avant (`Scope_Chunk`, `Lucivy_Scope`) restent sur le disque :
/// leur retrait est une décision à part, après validation.
pub fn migrer_vers_les_versions(catalog: &mut Catalog, scope_chunking: ChunkingConfig) -> Result<Bilan, String> {
    migrer_vers_les_versions_par_lots(catalog, scope_chunking, LOT)
}

/// Les occurrences dérivées par lot, et les versions dont on transfère les vecteurs par lot : la
/// mémoire d'une migration reste bornée (la copie de `code-rag3db-depot`, 14 novembre : un lot
/// unique a dépassé 32 Gio en trois minutes).
pub const LOT: usize = 2000;

/// [`migrer_vers_les_versions`], au lot donné.
pub fn migrer_vers_les_versions_par_lots(catalog: &mut Catalog, scope_chunking: ChunkingConfig, lot: usize) -> Result<Bilan, String> {
    use crate::code::{scope_form, ScopeForm, SCOPE, SCOPE_CONTENT, SCOPE_FORM_KEY, SCOPE_FORM_VERSIONS};
    let e = |e: crate::catalog::CatalogError| e.to_string();
    let lot = lot.max(1);
    let debut = std::time::Instant::now();

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
    let occurrences = uuids(catalog, "MATCH (s:Scope) RETURN s._uuid ORDER BY s._uuid")?;
    journal(debut, "départ", &format!("{} occurrences, {} modèles, lots de {lot}", occurrences.len(), modeles.len()));

    // 2. La bascule : la forme du binaire.
    let (occurrence, contenu) = crate::code::scope_version_configs(crate::code::scope_config(scope_chunking.clone()), scope_chunking);
    catalog.basculer_l_index_public_par_migration(SCOPE, occurrence).map_err(e)?;
    catalog.register_entity(SCOPE_CONTENT, contenu).map_err(e)?;
    journal(debut, "bascule", "Scope en occurrence, ScopeContent enregistré");

    // 3. Les occurrences dérivées par lots, chacun drainé jusqu'au plein texte avant le suivant.
    let mut bilan = Bilan::default();
    for (n, paquet) in occurrences.chunks(lot).enumerate() {
        catalog.mettre_en_file_les_derivations(
            paquet.iter().map(|u| crate::records::Derivation { entity: SCOPE_CONTENT.to_string(), root_uuid: u.clone() }).collect(),
        );
        let drain = catalog.drain_jusqu_a(crate::disponibilite::Disponibilites::RECHERCHE_TEXTE);
        if drain.failed > 0 {
            return Err(format!("la dérivation du lot {n} a échoué ({} échecs) : {}", drain.failed, drain.warnings.join(" ; ")));
        }
        bilan.plus_gros_lot = bilan.plus_gros_lot.max(paquet.len());
        if n % 5 == 0 || (n + 1) * lot >= occurrences.len() {
            journal(debut, "dérivation", &format!("{} / {} occurrences", ((n + 1) * lot).min(occurrences.len()), occurrences.len()));
        }
    }

    // 4. Les vecteurs, par lots de versions et modèle par modèle, sous l'index du modèle courant
    //    retiré puis rebâti.
    let versions = uuids(catalog, "MATCH (v:ScopeContent) RETURN v._uuid ORDER BY v._uuid")?;
    let transfert = catalog.bulk_vector_index(&[SCOPE_CONTENT], |cat| -> Result<(BTreeMap<String, usize>, usize, usize), String> {
        let mut repris = BTreeMap::new();
        let mut plus_de_lignes = 0usize;
        let mut plus_gros = 0usize;
        for (n, paquet) in versions.chunks(lot).enumerate() {
            for modele in &modeles {
                let (pris, lignes) = transferer_les_vecteurs(cat, modele, paquet)?;
                *repris.entry(modele.slug()).or_insert(0) += pris;
                plus_de_lignes = plus_de_lignes.max(lignes);
            }
            plus_gros = plus_gros.max(paquet.len());
            if n % 5 == 0 || (n + 1) * lot >= versions.len() {
                journal(debut, "vecteurs", &format!("{} / {} versions", ((n + 1) * lot).min(versions.len()), versions.len()));
            }
        }
        Ok((repris, plus_de_lignes, plus_gros))
    }).map_err(e)??;
    bilan.vecteurs_repris = transfert.0;
    bilan.plus_de_lignes_lues = transfert.1;
    bilan.plus_gros_lot = bilan.plus_gros_lot.max(transfert.2);
    for modele in &modeles {
        let neuf = crate::embedding_storage::VectorStorage::resolve("ScopeContent_Chunk", modele);
        let reste = compter(catalog, &format!("MATCH (c:ScopeContent_Chunk) WHERE c.{} IS NULL OR c.{} IS NULL OR c.{} <> c._text_hash RETURN count(c)", neuf.column, neuf.marker, neuf.marker))?;
        bilan.morceaux_en_dette.insert(modele.slug(), reste as usize);
    }
    journal(debut, "vecteurs", &format!("repris {:?}, en dette {:?}", bilan.vecteurs_repris, bilan.morceaux_en_dette));

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
    journal(debut, "estampille", &format!("{} occurrences, {} versions, {} morceaux", bilan.occurrences, bilan.versions, bilan.morceaux));
    Ok(bilan)
}

/// Une ligne de journal par étape ou par lot : le temps depuis le départ, l'étape, ce qu'elle
/// tient, et la mémoire résidente du processus.
fn journal(debut: std::time::Instant, etape: &str, quoi: &str) {
    let rss = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("VmRSS:")).map(|l| l.trim_start_matches("VmRSS:").trim().to_string()))
        .unwrap_or_else(|| "?".into());
    eprintln!("[migration 1b] {:>7.1} s · {etape} · {quoi} · rss {rss}", debut.elapsed().as_secs_f64());
}

fn uuids(catalog: &Catalog, cypher: &str) -> Result<Vec<String>, String> {
    Ok(catalog
        .execute_raw(cypher)
        .map_err(|e| format!("{cypher} : {e}"))?
        .rows
        .iter()
        .filter_map(|r| r.first().and_then(|v| v.as_str()).map(str::to_string))
        .collect())
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

/// Les vecteurs d'un modèle pour un lot de versions, des morceaux d'avant vers les morceaux neufs.
/// Rend le nombre de vecteurs repris et le plus grand nombre de lignes lues en une requête.
///
/// **Un représentant par version** : toutes les occurrences d'une version ont le même contenu, donc
/// les mêmes morceaux ; on lit ceux d'une seule (la première par uuid). Joindre chaque morceau neuf
/// aux morceaux de **toutes** les occurrences, vecteurs compris, puis trier, faisait exploser la
/// requête sur une version partagée par des milliers d'occurrences : la copie de
/// `code-rag3db-depot` (14 novembre) a rempli le tampon de 8 Gio du moteur au lot 42 000 / 72 652,
/// avec ou sans point de reprise par lot. Trois lectures bornées par le lot, l'appariement en Rust.
fn transferer_les_vecteurs(catalog: &Catalog, modele: &crate::embedding_storage::EmbeddingModelEntry, versions: &[String]) -> Result<(usize, usize), String> {
    use std::collections::HashMap;
    use crate::connection::{CypherValue, QueryParam};
    use crate::embedding_storage::VectorStorage;
    let avant = VectorStorage::resolve("Scope_Chunk", modele);
    let neuf = VectorStorage::resolve("ScopeContent_Chunk", modele);
    let liste = |v: &[String]| CypherValue::List(v.iter().map(|u| CypherValue::String(u.clone())).collect());
    let lire = |cypher: &str, nom: &str, valeurs: &[String]| {
        catalog
            .execute_raw_with_params(cypher, &[QueryParam::new(nom, liste(valeurs))])
            .map_err(|e| format!("le transfert de « {} » : {cypher} : {e}", modele.slug()))
    };
    let texte = |v: Option<&CypherValue>| v.and_then(|v| v.as_str()).unwrap_or("").to_string();
    let rang = |v: Option<&CypherValue>| v.and_then(|v| v.as_i64()).unwrap_or(-1);

    // 1. Le représentant de chaque version.
    let paires = lire(
        "MATCH (v:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s:Scope) WHERE v._uuid IN $versions RETURN v._uuid, s._uuid",
        "versions",
        versions,
    )?;
    let mut representant: HashMap<String, String> = HashMap::new();
    for r in &paires.rows {
        let (v, s) = (texte(r.first()), texte(r.get(1)));
        representant.entry(v).and_modify(|x| if s < *x { *x = s.clone() }).or_insert(s);
    }
    let representants: Vec<String> = representant.values().cloned().collect();

    // 2. Les morceaux neufs du lot.
    let neufs = lire(
        "MATCH (n:ScopeContent_Chunk)-[:ScopeContent_CHUNKED_FROM]->(v:ScopeContent) WHERE v._uuid IN $versions \
         RETURN v._uuid, n._uuid, n._parent_field, n._index, n._text_hash, coalesce(n._title, '')",
        "versions",
        versions,
    )?;

    // 3. Les morceaux d'avant des seuls représentants, avec leur vecteur calculé sur leur texte.
    let anciens = lire(
        &format!(
            "MATCH (o:Scope_Chunk)-[:Scope_CHUNKED_FROM]->(s:Scope) WHERE s._uuid IN $representants \
               AND o.{col} IS NOT NULL AND o.{marque} = o._text_hash \
             RETURN s._uuid, o._parent_field, o._index, o._text_hash, coalesce(o._title, ''), o.{col}",
            col = avant.column,
            marque = avant.marker,
        ),
        "representants",
        &representants,
    )?;
    let lignes = paires.rows.len().max(neufs.rows.len()).max(anciens.rows.len());
    let mut vecteurs: HashMap<(String, String, i64, String, String), CypherValue> = HashMap::new();
    for r in &anciens.rows {
        let cle = (texte(r.first()), texte(r.get(1)), rang(r.get(2)), texte(r.get(3)), texte(r.get(4)));
        if let Some(v) = r.get(5) {
            vecteurs.entry(cle).or_insert_with(|| v.clone());
        }
    }
    let mut items = Vec::new();
    for r in &neufs.rows {
        let Some(rep) = representant.get(&texte(r.first())) else { continue };
        let hash = texte(r.get(4));
        let cle = (rep.clone(), texte(r.get(2)), rang(r.get(3)), hash.clone(), texte(r.get(5)));
        let Some(vecteur) = vecteurs.get(&cle) else { continue };
        let mut item = std::collections::BTreeMap::new();
        item.insert("uuid".to_string(), CypherValue::String(texte(r.get(1))));
        item.insert("hash".to_string(), CypherValue::String(hash));
        item.insert("emb".to_string(), vecteur.clone());
        items.push(CypherValue::Map(item));
    }
    let repris = items.len();
    let (conn, dialect) = catalog.connexion_et_dialecte();
    crate::dataflow::record_nodes::write_vectors(conn.as_ref(), dialect.as_ref(), "ScopeContent_Chunk", &neuf.column, &neuf.marker, items)
        .map_err(|e| format!("l'écriture des vecteurs de « {} » : {e}", modele.slug()))?;
    Ok((repris, lignes))
}

/// **Migrer une base par son chemin**, hors hôte : la base (`<nom>.rag3db`), son plein texte en
/// fichiers à côté (`<nom>.rag3db.fts`), l'extension vectorielle du processus. Le catalogue s'ouvre
/// avec un embarqueur qui **refuse tout appel**, au nom et à la dimension du premier modèle
/// enregistré dans la base : une migration qui voudrait embarquer échoue au lieu de calculer.
/// À jouer sur une copie, sans autre ouvreur.
#[cfg(feature = "rag3db-native")]
pub fn migrer_la_base(chemin: &std::path::Path) -> Result<Bilan, String> {
    let mut catalog = ouvrir_par_le_chemin(chemin)?;
    migrer_vers_les_versions(&mut catalog, crate::code::default_scope_chunking())
}

/// **Une base de code ouverte par son chemin**, pour une migration : sa connexion avec l'extension
/// vectorielle, son plein texte en fichiers, un embarqueur qui refuse tout appel au nom de son modèle,
/// et le schéma de code enregistré (une base garde alors sa forme).
#[cfg(feature = "rag3db-native")]
fn ouvrir_par_le_chemin(chemin: &std::path::Path) -> Result<Catalog, String> {
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
    Ok(catalog)
}

/// Ce que la montée aux générations a fait, pour le rapport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BilanDesGenerations {
    /// La forme de la base avant (`versions` attendue).
    pub forme_avant: String,
    /// La forme après (`generations`).
    pub forme_apres: String,
    /// La couche et le rang de la dernière génération publiée, lus dans `_generation` (`backend`, 0).
    pub generation: Option<(String, i64)>,
    /// Le rang que les lectures voient ensuite : celui en attente (1), que le repassage écrit.
    pub rang_lu: Option<i64>,
}

/// **Monter une base à versions aux générations par son chemin** (la bascule des bases, après 1b) :
/// la base ouverte comme pour [`migrer_la_base`], sans autre ouvreur, puis
/// [`crate::code::migrer_vers_les_generations`]. Refuse une base tenue par un autre écrivain vivant
/// (sa marque `<base>.ecrivain`) et une base qui n'est pas à versions, en nommant la forme.
#[cfg(feature = "rag3db-native")]
pub fn migrer_la_base_aux_generations(chemin: &std::path::Path) -> Result<BilanDesGenerations, String> {
    use crate::code::ScopeForm;
    if let Some(m) = crate::marque_d_ecrivain::lire(chemin).filter(|m| m.pid != std::process::id()) {
        #[cfg(feature = "daemon")]
        let vivant = crate::daemon_lifecycle::pid_state(m.pid) != Some(false);
        #[cfg(not(feature = "daemon"))]
        let vivant = true;
        if vivant {
            return Err(format!(
                "{} : la montée aux générations ne s'y joue pas — {}",
                chemin.display(),
                crate::marque_d_ecrivain::qui_tient(chemin).unwrap_or_else(|| format!("tenue par le pid {}", m.pid))
            ));
        }
    }
    let mut catalog = ouvrir_par_le_chemin(chemin)?;
    let nom = |f: ScopeForm| match f {
        ScopeForm::Generations => "generations",
        ScopeForm::Versions => "versions",
        ScopeForm::SansVersions => "sans versions",
        ScopeForm::Neuve => "neuve",
    };
    let forme_avant = nom(crate::code::scope_form(&catalog).map_err(|e| format!("la forme : {e}"))?).to_string();
    crate::code::migrer_vers_les_generations(&mut catalog).map_err(|e| e.to_string())?;
    let forme_apres = nom(crate::code::scope_form(&catalog).map_err(|e| format!("la forme : {e}"))?).to_string();
    let rang_lu = catalog.generations.as_ref().map(|v| v.rang);
    let couche = crate::code::COUCHE_DU_BACKEND;
    let generation = match catalog.dialect_arc().latest_generation_rank() {
        Some(q) => {
            let lu = catalog
                .execute_raw_with_params(&q, &[crate::connection::QueryParam::new("couche", crate::connection::CypherValue::String(couche.to_string()))])
                .map_err(|e| format!("la génération publiée : {e}"))?;
            lu.rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64()).map(|r| (couche.to_string(), r))
        }
        None => None,
    };
    Ok(BilanDesGenerations { forme_avant, forme_apres, generation, rang_lu })
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
