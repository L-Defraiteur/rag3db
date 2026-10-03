//! **Le code sur la synchronisation déclarée** (3 octobre 2026).
//!
//! L'entité `Scope` déclare son périmètre — la source — et un grain fin — le
//! fichier (`SnapshotConfig { scope: [source], fine_scope: [file_path] }`). Ce
//! module en fait les verbes du code ; le mécanisme, lui, ne connaît aucun nom
//! de champ ([`crate::catalog`], `synchronisation.rs`).
//!
//! - [`reingest_file`] : une édition connaît tout le contenu de son fichier.
//!   C'est une **fin immédiate** sur le grain fin — ouvrir, marquer les scopes
//!   que l'analyse produit encore, finir —, **avant** `ingest_code` : un scope
//!   déplacé dans le même fichier (même nom, autre parent) aurait sinon deux
//!   définisseurs au moment de la résolution par `Symbol`, qui s'abstient sur
//!   un nom ambigu. `allowEmpty` (un fichier vidé de ses scopes est légitime)
//!   et `force` (renommer un scope sur deux est une édition ordinaire) : le
//!   contenu entier est connu, les garde-fous d'un instantané partiel n'ont
//!   rien à protéger ici.
//! - Si une synchronisation de la source entière tient le périmètre large,
//!   le grain fin est refusé : l'édition **écrit simplement**, la marque à
//!   l'écriture la compte pour la session large, et les scopes disparus du
//!   fichier partent à la fin de celle-ci.
//!
//! - [`sync_source`] : la **synchronisation d'une source entière**. Elle ouvre
//!   une session sur le grain large de `Scope` et de `File`, ingère la source
//!   par paquets de fichiers en marquant ce qu'elle porte, puis finit : ce qui
//!   a disparu de la source — scopes, et fichiers supprimés — part, par la
//!   mise de côté. C'est la brique que les produits appellent pour « indexer
//!   ce dépôt » et le tenir à jour.
//!
//! Le module est neuf exprès : `code.rs` et `code_tools.rs` sont indexés par
//! le banc de recherche (corpus vivant).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::catalog::{Catalog, CatalogError, SnapshotFinish, SnapshotFinishOptions};
use crate::code::{FILE, SCOPE};
use crate::code_tools::{FileSource, ReingestReport};
use crate::connection::CypherValue;
use crate::disponibilite::Disponibilites;

/// Le grain fin d'un fichier : sa source et son nom indexé.
fn grain_du_fichier(source_id: &str, indexed_path: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([
        ("source".to_string(), CypherValue::String(source_id.to_string())),
        ("file_path".to_string(), CypherValue::String(indexed_path.to_string())),
    ])
}

/// **Ré-ingère un seul fichier**, par la synchronisation déclarée : analyse
/// seule (références locales et `DEFINED_IN` ; l'inter-fichiers attend la
/// résolution contre la base), retrait des scopes disparus par une fin
/// immédiate sur le grain du fichier, puis upsert du reste.
pub fn reingest_file(
    catalog: &mut Catalog,
    source: &dyn FileSource,
    path: &str,
    content: &str,
    exige: Disponibilites,
) -> Result<ReingestReport, String> {
    let cursor = source.cursor();
    let (root, virtual_source) = match cursor.strip_prefix("worktree:") {
        Some(root) => (root.to_string(), false),
        None => ("/".to_string(), true),
    };
    let mut analysis = crate::code::analyze_with(&root, vec![(path.to_string(), content.to_string())], &cursor);
    for f in &mut analysis.files {
        f.cursor = cursor.clone();
        if virtual_source {
            f.absolute_path.clear();
        }
    }
    let (source_id, indexed_path) = crate::code_tools::indexed_name(source, path);
    let grain = grain_du_fichier(&source_id, &indexed_path);

    // La fin immédiate sur le grain du fichier — ou rien, si la source entière
    // est en cours de synchronisation : alors elle s'en charge.
    let mut deleted = 0usize;
    let mut deferred_to = None;
    let mut noms_retires = Vec::new();
    match catalog.begin_snapshot(SCOPE, &grain, true) {
        Ok(open) => {
            let uuids = analysis
                .scopes
                .iter()
                .map(|s| {
                    catalog.entity_uuid(SCOPE, &BTreeMap::from([("key".to_string(), CypherValue::String(s.key.clone()))]))
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            catalog.mark_snapshot(SCOPE, &grain, &open.session, &uuids).map_err(|e| e.to_string())?;
            let plan = catalog
                .plan_snapshot_finish(SCOPE, &grain, &open.session, SnapshotFinishOptions { allow_empty: true, force: true })
                .map_err(|e| e.to_string())?;
            noms_retires = noms_des_scopes(catalog, &plan.removed)?;
            let fin = catalog.apply_snapshot_finish(plan).map_err(|e| e.to_string())?;
            deleted = fin.removed.len();
        }
        Err(CatalogError::SnapshotRefused(raison)) => deferred_to = Some(raison),
        Err(e) => return Err(e.to_string()),
    }
    let report = catalog.ingest_code_jusqu_a(&analysis, exige).map_err(|e| e.to_string())?;
    // Un nom dont un définisseur vient de partir a pu redevenir unique.
    catalog.resoudre_les_symboles(&noms_retires, exige).map_err(|e| e.to_string())?;
    Ok(ReingestReport {
        scopes_upserted: report.scopes,
        scopes_deleted: deleted,
        relations: report.relations,
        failed: report.failed,
        deferred_to,
    })
}

/// Ce qu'on demande à une synchronisation de source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SourceSyncOptions {
    /// Fichiers par paquet d'ingestion.
    pub batch_files: usize,
    /// S'arrêter au plan : la source est ingérée (c'est l'avancement), mais
    /// rien n'est retiré et les sessions sont abandonnées. Le rapport dit ce
    /// qui le serait — l'estimation avant de confirmer.
    pub plan_only: bool,
    /// Reprendre une synchronisation abandonnée de la même source.
    pub takeover: bool,
    /// Les échappatoires des garde-fous de la fin, explicites.
    pub allow_empty: bool,
    pub force: bool,
    /// Ce qui doit être prêt quand un paquet rend (`RECHERCHE_TEXTE` laisse
    /// la dette de vecteurs en base).
    #[serde(skip, default = "tout")]
    pub exige: Disponibilites,
}

fn tout() -> Disponibilites {
    Disponibilites::TOUT
}

impl Default for SourceSyncOptions {
    fn default() -> Self {
        Self { batch_files: 64, plan_only: false, takeover: false, allow_empty: false, force: false, exige: Disponibilites::TOUT }
    }
}

/// L'avancement d'une synchronisation, rendu après chaque paquet.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSyncProgress {
    pub files_done: usize,
    pub files_total: usize,
    pub scopes_written: usize,
}

/// Ce qu'une synchronisation de source a fait.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSyncReport {
    pub source: String,
    /// Fichiers listés par la source, et ceux retenus pour l'analyse.
    pub files_listed: usize,
    pub files_ingested: usize,
    pub scopes_written: usize,
    pub relations: usize,
    pub failed: usize,
    /// La fin, par entité : plan seul si `plan_only`, appliquée sinon.
    pub scopes: SnapshotFinish,
    pub files: SnapshotFinish,
}

/// **Synchroniser une source entière** : voir le module. Les sessions sont
/// abandonnées si quoi que ce soit échoue avant la fin — rien n'est retiré
/// d'une synchronisation incomplète.
pub fn sync_source(
    catalog: &mut Catalog,
    source: &dyn FileSource,
    options: &SourceSyncOptions,
    progress: &mut dyn FnMut(SourceSyncProgress),
) -> Result<SourceSyncReport, String> {
    let cursor = source.cursor();
    let source_id = crate::code::source_id(&cursor);
    let grain = BTreeMap::from([("source".to_string(), CypherValue::String(source_id.clone()))]);
    let s_scopes = catalog.begin_snapshot(SCOPE, &grain, options.takeover).map_err(|e| e.to_string())?.session;
    let s_files = match catalog.begin_snapshot(FILE, &grain, options.takeover) {
        Ok(open) => open.session,
        Err(e) => {
            let _ = catalog.abort_snapshot(SCOPE, &grain, &s_scopes);
            return Err(e.to_string());
        }
    };
    let resultat = synchroniser(catalog, source, options, progress, &grain, &s_scopes, &s_files, source_id);
    if resultat.is_err() || options.plan_only {
        let _ = catalog.abort_snapshot(SCOPE, &grain, &s_scopes);
        let _ = catalog.abort_snapshot(FILE, &grain, &s_files);
    }
    resultat
}

#[allow(clippy::too_many_arguments)]
fn synchroniser(
    catalog: &mut Catalog,
    source: &dyn FileSource,
    options: &SourceSyncOptions,
    progress: &mut dyn FnMut(SourceSyncProgress),
    grain: &BTreeMap<String, CypherValue>,
    s_scopes: &str,
    s_files: &str,
    source_id: String,
) -> Result<SourceSyncReport, String> {
    let cursor = source.cursor();
    let (root, virtual_source) = match cursor.strip_prefix("worktree:") {
        Some(root) => (root.to_string(), false),
        None => ("/".to_string(), true),
    };
    let listed = source.list()?;
    // Le tri au nom seul, comme `analyze_source` : ne pas lire un gros
    // fichier binaire pour l'écarter ensuite.
    let retenus: Vec<String> = listed
        .iter()
        .filter(|p| !matches!(crate::code::verdict(p, 0), crate::code::Verdict::Ecarte(_)))
        .cloned()
        .collect();
    let mut report = SourceSyncReport { source: source_id, files_listed: listed.len(), ..Default::default() };
    let mut avancement = SourceSyncProgress { files_total: retenus.len(), ..Default::default() };
    for paquet in retenus.chunks(options.batch_files.max(1)) {
        let mut sources = Vec::with_capacity(paquet.len());
        for path in paquet {
            if let Some(content) = source.read(path)? {
                sources.push((path.clone(), content));
            }
        }
        let mut analysis = crate::code::analyze_with(&root, sources, &cursor);
        for f in &mut analysis.files {
            f.cursor = cursor.clone();
            if virtual_source {
                f.absolute_path.clear();
            }
        }
        let ingere = catalog.ingest_code_jusqu_a(&analysis, options.exige).map_err(|e| e.to_string())?;
        // Ce que le paquet porte, marqué de la session : vu, pas seulement
        // écrit pendant elle.
        let uuids_scopes = analysis
            .scopes
            .iter()
            .map(|s| catalog.entity_uuid(SCOPE, &BTreeMap::from([("key".to_string(), CypherValue::String(s.key.clone()))])))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let uuids_files = analysis
            .files
            .iter()
            .map(|f| catalog.entity_uuid(FILE, &f.data()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        catalog.mark_snapshot(SCOPE, grain, s_scopes, &uuids_scopes).map_err(|e| e.to_string())?;
        catalog.mark_snapshot(FILE, grain, s_files, &uuids_files).map_err(|e| e.to_string())?;
        report.files_ingested += analysis.files.len();
        report.scopes_written += ingere.scopes;
        report.relations += ingere.relations;
        report.failed += ingere.failed;
        avancement.files_done += paquet.len();
        avancement.scopes_written = report.scopes_written;
        progress(avancement);
    }
    let garde = SnapshotFinishOptions { allow_empty: options.allow_empty, force: options.force };
    let plan_scopes = catalog.plan_snapshot_finish(SCOPE, grain, s_scopes, garde).map_err(|e| e.to_string())?;
    let plan_files = catalog.plan_snapshot_finish(FILE, grain, s_files, garde).map_err(|e| e.to_string())?;
    if options.plan_only {
        report.scopes = plan_scopes;
        report.files = plan_files;
        return Ok(report);
    }
    // Les scopes d'abord : un fichier supprimé emporte ses `DEFINED_IN`, ses
    // scopes sont déjà partis.
    let noms_retires = noms_des_scopes(catalog, &plan_scopes.removed)?;
    report.scopes = catalog.apply_snapshot_finish(plan_scopes).map_err(|e| e.to_string())?;
    report.files = catalog.apply_snapshot_finish(plan_files).map_err(|e| e.to_string())?;
    // Un nom dont un définisseur vient de partir a pu redevenir unique : ses
    // mentionneurs, qu'aucun paquet ne repasse, gagnent leur arête.
    let resolu = catalog.resoudre_les_symboles(&noms_retires, options.exige).map_err(|e| e.to_string())?;
    report.relations += resolu.linked_across_batches;
    Ok(report)
}

/// Les noms des scopes `uuids`, relus avant leur retrait.
fn noms_des_scopes(catalog: &Catalog, uuids: &[String]) -> Result<Vec<String>, String> {
    if uuids.is_empty() {
        return Ok(Vec::new());
    }
    let mut noms: Vec<String> = catalog
        .get_many(SCOPE, uuids)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter_map(|r| r.get("name").and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    noms.sort();
    noms.dedup();
    Ok(noms)
}
