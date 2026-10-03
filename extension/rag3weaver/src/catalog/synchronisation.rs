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

use super::{Catalog, CatalogError, LifecycleVerdict};
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
    /// Les relations déclarées que le retrait emportera, **comptées au plan,
    /// avant l'écriture** : une estimation d'avant, pas un constat — une
    /// ligne dont la suppression échoue garde les siennes. `None` quand le
    /// dialecte ne sait pas les compter.
    pub relations_to_remove: Option<usize>,
    /// Les avertissements de l'écriture.
    pub warnings: Vec<String>,
    /// `false` : un plan (`plan_snapshot_finish`), rien n'est encore écrit ;
    /// `removed` et `transitioned` disent alors ce qui le sera.
    pub applied: bool,
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
        let cypher = self.dialect.mark_snapshot_session(entity_name);
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
    /// toute écriture. C'est `plan_snapshot_finish` puis
    /// `apply_snapshot_finish`.
    pub fn finish_snapshot(
        &mut self,
        entity_name: &str,
        scope: &BTreeMap<String, CypherValue>,
        session: &str,
        options: SnapshotFinishOptions,
    ) -> Result<SnapshotFinish, CatalogError> {
        let plan = self.plan_snapshot_finish(entity_name, scope, session, options)?;
        self.apply_snapshot_finish(plan)
    }

    /// **Ce qu'une fin ferait, sans rien écrire** : les comptes, les
    /// garde-fous (qui refusent ici déjà), et pour chaque absente ce qui lui
    /// arrivera. Le plan est relu et appliqué par `apply_snapshot_finish` ;
    /// entre les deux, la base peut changer — l'application le rapporte.
    pub fn plan_snapshot_finish(
        &self,
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
        let cypher = self.dialect.select_snapshot_scope(entity_name, &scope_fields, &[]);
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
                // Les relations déclarées au catalogue qui touchent l'entité :
                // les liens internes (chunks, dérivées) ne sont pas des
                // relations au sens de l'utilisateur.
                let mut rels: Vec<String> = self
                    .config
                    .relations
                    .iter()
                    .filter(|(_, d)| d.from == entity_name || d.to == entity_name)
                    .map(|(name, _)| name.clone())
                    .collect();
                rels.sort();
                let list = CypherValue::List(report.missing.iter().cloned().map(CypherValue::String).collect());
                let mut total = Some(0usize);
                for rel in &rels {
                    let Some(cypher) = self.dialect.count_relations_of(entity_name, rel) else {
                        total = None;
                        break;
                    };
                    let res = self
                        .conn
                        .execute_with_params(&cypher, &[QueryParam::new("uuids", list.clone())])
                        .map_err(|e| CatalogError::DbError(e.to_string()))?;
                    let n = res.rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
                    total = total.map(|t| t + n);
                }
                report.relations_to_remove = total;
                report.removed = report.missing.clone();
            }
            OnMissing::Transition(name) => {
                let lc = entity_config.lifecycle.as_ref().ok_or_else(|| {
                    CatalogError::SchemaError(format!("snapshot : la transition '{name}' sans lifecycle"))
                })?;
                // Un nom désigne un seul couple (from, to) : `Lifecycle::validate`
                // refuse deux transitions du même nom. Le `find` en dépend.
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
                    let Some(row) = current.get(&uuid) else {
                        report.kept.push((uuid, "introuvable à la relecture".into()));
                        continue;
                    };
                    // Un état vide est un état inconnu (une machine déclarée
                    // sur une entité déjà en service) : l'état initial.
                    let state = row
                        .get(&lc.field)
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .unwrap_or(&lc.initial)
                        .to_string();
                    match Catalog::lifecycle_verdict(entity_name, lc, &uuid, &transition.to, Some(&state)) {
                        LifecycleVerdict::Same => {
                            report.already.push(uuid);
                            continue;
                        }
                        LifecycleVerdict::Refused(cause) => {
                            report.kept.push((
                                uuid,
                                format!("transition '{}' impossible depuis '{state}' (elle part de '{}') — {cause}", transition.name, transition.from),
                            ));
                            continue;
                        }
                        LifecycleVerdict::Allowed => report.transitioned.push(uuid),
                    }
                }
            }
        }
        Ok(report)
    }

    /// **Appliquer un plan de fin.** Les retraits et les transitions planifiés
    /// sont posés puis drainés — une fin est une unité, elle rend une fois
    /// tout posé, quel que soit le régime d'écriture. La base a pu changer
    /// depuis le plan : une ligne disparue entre-temps est gardée et nommée,
    /// et une transition refusée par la garde à l'écriture aussi, avec sa
    /// cause. Le rapport ne dit que ce qui a eu lieu.
    pub fn apply_snapshot_finish(&mut self, plan: SnapshotFinish) -> Result<SnapshotFinish, CatalogError> {
        let mut report = plan;
        if report.applied {
            return Err(CatalogError::ValidationFailed("snapshot : ce plan a déjà été appliqué".into()));
        }
        report.applied = true;
        let entity_name = report.entity.clone();
        self.check_ecriture("finish_snapshot")?;
        for uuid in &report.removed {
            self.mettre_en_file_la_suppression(&entity_name, uuid)?;
        }
        if !report.transitioned.is_empty() {
            let entity_config = self
                .entity_configs()
                .get(&entity_name)
                .cloned()
                .ok_or_else(|| CatalogError::UnknownEntity(entity_name.clone()))?;
            let (lc, name) = match (&entity_config.lifecycle, entity_config.snapshot.as_ref().map(|s| &s.on_missing)) {
                (Some(lc), Some(OnMissing::Transition(name))) => (lc.clone(), name.clone()),
                _ => return Err(CatalogError::SchemaError("snapshot : transitions planifiées sans transition déclarée".into())),
            };
            let to = lc
                .transitions
                .iter()
                .find(|t| t.name == name)
                .map(|t| t.to.clone())
                .ok_or_else(|| CatalogError::SchemaError(format!("snapshot : transition '{name}' non déclarée")))?;
            let current: HashMap<String, BTreeMap<String, CypherValue>> = self
                .get_many(&entity_name, &report.transitioned)?
                .into_iter()
                .filter_map(|row| Some((row.get("_uuid")?.as_str()?.to_string(), row)))
                .collect();
            let planned = std::mem::take(&mut report.transitioned);
            for uuid in planned {
                let Some(row) = current.get(&uuid) else {
                    report.kept.push((uuid, "introuvable à la relecture".into()));
                    continue;
                };
                // La ligne entière, sans ses colonnes internes : le hash de
                // contenu se calcule sur ce qu'on écrit. La garde de la
                // machine à états juge à l'écriture.
                let mut data: BTreeMap<String, CypherValue> = row
                    .iter()
                    .filter(|(k, _)| !k.starts_with('_') && !crate::scope::is_scope_column(k))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                data.insert(lc.field.clone(), CypherValue::String(to.clone()));
                self.mettre_en_file_la_mise_a_jour(&entity_name, &uuid, data)?;
                report.transitioned.push(uuid);
            }
        }
        if !(report.removed.is_empty() && report.transitioned.is_empty()) {
            let res = self.drain();
            // Le rapport ne dit que ce qui a eu lieu : une transition refusée
            // au drain (l'état relu à un autre instant) quitte `transitioned`
            // pour `kept`, avec la cause que porte `UpdateStatus::Failed` ;
            // une suppression qui échoue quitte `removed` de même — une
            // ligne qu'on croit retirée ne serait jamais retentée. Le refus à
            // l'écriture est éprouvé par `e2e_synchronisation` (plan, état
            // changé, application) ; l'échec d'une suppression ne se provoque
            // pas aujourd'hui : la boucle n'est pas couverte, elle n'est pas
            // morte pour autant.
            for del in &res.delete_results {
                if let Some(cause) = &del.echec {
                    if let Some(pos) = report.removed.iter().position(|u| u == &del.uuid) {
                        let uuid = report.removed.remove(pos);
                        report.kept.push((uuid, format!("suppression refusée : {cause}")));
                    }
                }
            }
            for update in &res.update_results {
                if let crate::records::UpdateStatus::Failed(cause) = &update.status {
                    if let Some(pos) = report.transitioned.iter().position(|u| u == &update.uuid) {
                        let uuid = report.transitioned.remove(pos);
                        report.kept.push((uuid, format!("refusée à l'écriture : {cause}")));
                    }
                }
            }
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
