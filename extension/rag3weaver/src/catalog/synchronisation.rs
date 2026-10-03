//! **La synchronisation par périmètre** (`SnapshotConfig`, 3 octobre 2026).
//!
//! Un instantané complet d'un périmètre arrive en plusieurs lots, marqués d'un
//! même identifiant de session (`mark_snapshot`, colonne `_snapshot`). L'appel
//! de fin (`finish_snapshot`) retire — ou fait passer par la transition
//! déclarée — les lignes du périmètre qu'aucun lot de la session n'a portées.
//! Le moteur ne connaît aucun nom de champ : le périmètre est déclaré par
//! l'entité.
//!
//! Les garde-fous : rien n'est retiré avant la fin ; un instantané qui n'a
//! rien vu dans le périmètre est refusé ; au-delà de `maxMissingRatio` du
//! périmètre, la fin refuse sans `force` ; le rapport nomme tout ce qui a été
//! retiré, transitionné ou gardé.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use super::{Catalog, CatalogError};
use crate::config::OnMissing;
use crate::connection::{CypherValue, QueryParam};

/// Les deux échappatoires des garde-fous, toujours explicites.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SnapshotFinishOptions {
    /// Accepter une session qui n'a vu aucune ligne du périmètre (le
    /// périmètre est vraiment devenu vide).
    pub allow_empty: bool,
    /// Passer outre `maxMissingRatio`.
    pub force: bool,
}

/// Ce qu'une fin de synchronisation a fait — une absence se nomme.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotFinish {
    pub entity: String,
    pub session: String,
    /// Les lignes du périmètre en base au moment de la fin.
    pub in_scope: usize,
    /// Celles qu'un lot de la session a portées.
    pub seen: usize,
    /// Les absentes (uuids), quoi qu'il leur soit arrivé ensuite.
    pub missing: Vec<String>,
    /// Retirées (`onMissing: delete`).
    pub removed: Vec<String>,
    /// Passées par la transition (`onMissing: {transition}`).
    pub transitioned: Vec<String>,
    /// Déjà dans l'état où mène la transition : rien à faire.
    pub already: Vec<String>,
    /// Absentes gardées, avec la raison (la transition ne part pas de leur
    /// état).
    pub kept: Vec<(String, String)>,
    /// Les relations emportées avec les lignes retirées ; `None` quand le
    /// dialecte ne sait pas les compter.
    pub relations_removed: Option<usize>,
    /// Les avertissements de l'écriture.
    pub warnings: Vec<String>,
}

impl Catalog {
    /// **Marquer des lignes comme portées par une session** de
    /// synchronisation — un `SET` par appel, quel que soit le nombre de
    /// lignes (mesuré : 53 lots de 512 en 324 ms). L'entité doit déclarer
    /// `snapshot`.
    pub fn mark_snapshot(
        &mut self,
        entity_name: &str,
        session: &str,
        uuids: &[String],
    ) -> Result<(), CatalogError> {
        self.check_initialized()?;
        self.check_ecriture("mark_snapshot")?;
        self.check_entity(entity_name)?;
        self.snapshot_config(entity_name)?;
        if session.is_empty() {
            return Err(CatalogError::ValidationFailed("snapshot : la session ne peut pas être vide".into()));
        }
        if uuids.is_empty() {
            return Ok(());
        }
        let cypher = self.dialect.marquer_session(entity_name);
        self.conn
            .execute_with_params(&cypher, &[
                QueryParam::new("uuids", CypherValue::List(uuids.iter().cloned().map(CypherValue::String).collect())),
                QueryParam::new("session", CypherValue::String(session.to_string())),
            ])
            .map_err(|e| CatalogError::DbError(e.to_string()))?;
        Ok(())
    }

    /// **Vérifier qu'un lot appartient bien au périmètre annoncé** : chaque
    /// ligne porte, pour chaque champ du périmètre, la même valeur. Rend ces
    /// valeurs (vide pour un périmètre vide).
    pub fn snapshot_scope_of(
        &self,
        entity_name: &str,
        rows: &[BTreeMap<String, CypherValue>],
    ) -> Result<BTreeMap<String, CypherValue>, CatalogError> {
        let config = self.snapshot_config(entity_name)?;
        let mut scope = BTreeMap::new();
        for field in &config.scope {
            let mut value: Option<&CypherValue> = None;
            for (i, row) in rows.iter().enumerate() {
                let v = row.get(field).filter(|v| !matches!(v, CypherValue::Null)).ok_or_else(|| {
                    CatalogError::ValidationFailed(format!("snapshot : ligne {i} sans valeur pour le champ de périmètre '{field}'"))
                })?;
                match value {
                    None => value = Some(v),
                    Some(first) if first == v => {}
                    Some(first) => {
                        return Err(CatalogError::ValidationFailed(format!(
                            "snapshot : un lot porte un seul périmètre — '{field}' vaut {first:?} puis {v:?} (ligne {i})"
                        )))
                    }
                }
            }
            if let Some(v) = value {
                scope.insert(field.clone(), v.clone());
            }
        }
        Ok(scope)
    }

    /// **La fin d'une session** : les lignes du périmètre que la session n'a
    /// pas portées sont absentes ; selon `onMissing`, elles sont retirées ou
    /// passent par la transition déclarée. Les garde-fous refusent avant
    /// toute écriture.
    pub fn finish_snapshot(
        &mut self,
        entity_name: &str,
        scope: &BTreeMap<String, CypherValue>,
        session: &str,
        options: SnapshotFinishOptions,
    ) -> Result<SnapshotFinish, CatalogError> {
        self.check_initialized()?;
        self.check_ecriture("finish_snapshot")?;
        self.check_entity(entity_name)?;
        let config = self.snapshot_config(entity_name)?.clone();
        if session.is_empty() {
            return Err(CatalogError::ValidationFailed("snapshot : la session ne peut pas être vide".into()));
        }
        // Le périmètre donné est exactement celui que l'entité déclare.
        let mut declared: Vec<&str> = config.scope.iter().map(String::as_str).collect();
        declared.sort_unstable();
        let given: Vec<&str> = scope.keys().map(String::as_str).collect();
        if declared != given {
            return Err(CatalogError::ValidationFailed(format!(
                "snapshot : le périmètre déclaré est [{}], reçu [{}]",
                declared.join(", "),
                given.join(", ")
            )));
        }

        let entity_config = self
            .entity_configs()
            .get(entity_name)
            .cloned()
            .ok_or_else(|| CatalogError::UnknownEntity(entity_name.to_string()))?;
        let scope_fields: Vec<&str> = config.scope.iter().map(String::as_str).collect();
        let cypher = self.dialect.select_perimetre(entity_name, &scope_fields, &[]);
        let params: Vec<QueryParam> = config
            .scope
            .iter()
            .enumerate()
            .map(|(i, f)| QueryParam::new(&format!("p{i}"), scope[f].clone()))
            .collect();
        let rows = self
            .conn
            .execute_with_params(&cypher, &params)
            .map_err(|e| CatalogError::DbError(e.to_string()))?
            .rows;

        let mut report = SnapshotFinish {
            entity: entity_name.to_string(),
            session: session.to_string(),
            in_scope: rows.len(),
            ..Default::default()
        };
        for row in &rows {
            let uuid = row.first().and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let mark = row.get(1).and_then(|v| v.as_str()).unwrap_or_default();
            if mark == session {
                report.seen += 1;
            } else {
                report.missing.push(uuid);
            }
        }
        report.missing.sort();

        // ── Les garde-fous, avant toute écriture ────────────────────────
        if report.seen == 0 && !options.allow_empty {
            return Err(CatalogError::SnapshotRefused(format!(
                "{entity_name} : la session '{session}' n'a porté aucune ligne du périmètre ({} en base) — \
                 instantané vide ou tronqué ; rien n'est retiré (allowEmpty pour l'accepter)",
                report.in_scope
            )));
        }
        if !report.missing.is_empty() && !options.force {
            let ratio = report.missing.len() as f64 / report.in_scope as f64;
            if ratio > config.max_missing_ratio {
                let shown: Vec<&str> = report.missing.iter().take(20).map(String::as_str).collect();
                return Err(CatalogError::SnapshotRefused(format!(
                    "{entity_name} : {} ligne(s) absente(s) sur {} dans le périmètre ({:.0} %), au-delà de \
                     maxMissingRatio ({:.0} %) ; rien n'est retiré (force pour passer outre). Absentes : {}{}",
                    report.missing.len(),
                    report.in_scope,
                    ratio * 100.0,
                    config.max_missing_ratio * 100.0,
                    shown.join(", "),
                    if report.missing.len() > shown.len() { ", …" } else { "" }
                )));
            }
        }
        if report.missing.is_empty() {
            return Ok(report);
        }

        // ── Ce que deviennent les absentes ──────────────────────────────
        match &config.on_missing {
            OnMissing::Delete => {
                report.relations_removed = match self.dialect.compter_relations_de(entity_name) {
                    Some(cypher) => {
                        let list = CypherValue::List(report.missing.iter().cloned().map(CypherValue::String).collect());
                        let res = self
                            .conn
                            .execute_with_params(&cypher, &[QueryParam::new("uuids", list)])
                            .map_err(|e| CatalogError::DbError(e.to_string()))?;
                        res.rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64()).map(|n| n as usize)
                    }
                    None => None,
                };
                for uuid in report.missing.clone() {
                    self.mettre_en_file_la_suppression(entity_name, &uuid)?;
                    report.removed.push(uuid);
                }
            }
            OnMissing::Transition(name) => {
                let lc = entity_config.lifecycle.as_ref().ok_or_else(|| {
                    CatalogError::SchemaError(format!("snapshot : la transition '{name}' sans lifecycle"))
                })?;
                let transition = lc
                    .transitions
                    .iter()
                    .find(|t| &t.name == name)
                    .ok_or_else(|| CatalogError::SchemaError(format!("snapshot : transition '{name}' non déclarée")))?
                    .clone();
                let current: HashMap<String, BTreeMap<String, CypherValue>> = self
                    .get_many(entity_name, &report.missing)?
                    .into_iter()
                    .filter_map(|row| Some((row.get("_uuid")?.as_str()?.to_string(), row)))
                    .collect();
                for uuid in report.missing.clone() {
                    let Some(row) = current.get(&uuid) else { continue };
                    let state = row.get(&lc.field).and_then(|v| v.as_str()).unwrap_or(&lc.initial).to_string();
                    if state == transition.to {
                        report.already.push(uuid);
                        continue;
                    }
                    if state != transition.from {
                        report.kept.push((
                            uuid,
                            format!("transition '{}' impossible depuis '{state}' (elle part de '{}')", transition.name, transition.from),
                        ));
                        continue;
                    }
                    // La ligne entière, sans ses colonnes internes : le hash
                    // de contenu se calcule sur ce qu'on écrit.
                    let mut data: BTreeMap<String, CypherValue> = row
                        .iter()
                        .filter(|(k, _)| !k.starts_with('_') && !crate::scope::is_scope_column(k))
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    data.insert(lc.field.clone(), CypherValue::String(transition.to.clone()));
                    self.mettre_en_file_la_mise_a_jour(entity_name, &uuid, data)?;
                    report.transitioned.push(uuid);
                }
            }
        }
        if !(report.removed.is_empty() && report.transitioned.is_empty()) {
            let exige = self.exigence_d_ecriture_par_defaut();
            let res = self.tenir_l_exigence_d_ecriture(entity_name, exige);
            report.warnings = res.warnings;
        }
        Ok(report)
    }

    fn snapshot_config(&self, entity_name: &str) -> Result<&crate::config::SnapshotConfig, CatalogError> {
        self.entity_configs()
            .get(entity_name)
            .and_then(|c| c.snapshot.as_ref())
            .ok_or_else(|| CatalogError::ValidationFailed(format!("{entity_name} ne déclare pas de synchronisation (snapshot)")))
    }
}
