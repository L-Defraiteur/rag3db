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
//! Le module est neuf exprès : `code.rs` et `code_tools.rs` sont indexés par
//! le banc de recherche (corpus vivant).

use std::collections::BTreeMap;

use crate::catalog::{Catalog, CatalogError, SnapshotFinishOptions};
use crate::code::SCOPE;
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
            let fin = catalog
                .finish_snapshot(SCOPE, &grain, &open.session, SnapshotFinishOptions { allow_empty: true, force: true })
                .map_err(|e| e.to_string())?;
            deleted = fin.removed.len();
        }
        Err(CatalogError::SnapshotRefused(raison)) => deferred_to = Some(raison),
        Err(e) => return Err(e.to_string()),
    }
    let report = catalog.ingest_code_jusqu_a(&analysis, exige).map_err(|e| e.to_string())?;
    Ok(ReingestReport {
        scopes_upserted: report.scopes,
        scopes_deleted: deleted,
        relations: report.relations,
        failed: report.failed,
        deferred_to,
    })
}
