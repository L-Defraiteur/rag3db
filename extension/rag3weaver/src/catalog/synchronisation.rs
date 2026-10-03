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
//!
//! **La marque à l'écriture** (3 octobre 2026) : une écriture ordinaire —
//! ingestion, création, mise à jour, hors de toute session — dans un
//! périmètre dont une session est ouverte prend la marque de cette session
//! (`{session}+w`) : écrire une ligne, c'est dire que la source l'a. Sans
//! elle, une ligne écrite pendant la session, ou recréée entre le plan et
//! l'application, gardait une marque qui n'était pas la sienne et la fin la
//! retirait. Le rapport compte à part ce que la session a porté (`seen`) et ce
//! que des écritures ont marqué (`written`) ; la marque monte, elle ne descend
//! jamais (portée puis réécrite reste portée) ; les écritures de la fin
//! elle-même ne marquent pas. **Le prix, assumé** : un écrivain qui réécrit
//! périodiquement une ligne que la source n'a plus empêche son retrait, sans
//! bruit — la rétention plutôt que la suppression à tort. Une écriture en
//! Cypher brut (`execute_raw`) contourne le catalogue et ne marque pas.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use super::{Catalog, CatalogError, LifecycleVerdict};
use crate::config::OnMissing;
use crate::connection::{CypherValue, QueryParam};

/// Le suffixe de la marque d'une **écriture** pendant une session :
/// `{session}+w`. Un identifiant de session qui le contiendrait est refusé à
/// l'ouverture : la lecture de la marque reste sans ambiguïté par construction.
const WRITTEN_SUFFIX: &str = "+w";

/// **Ce que dit la marque `_snapshot` d'une ligne**, pour une session — lue
/// en un seul endroit ([`mark_verdict`]), comme `LifecycleVerdict` pour la
/// machine à états : une égalité oubliée quelque part retirerait une ligne
/// présente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkVerdict {
    /// Un lot de la session l'a portée.
    Carried,
    /// Une écriture ordinaire l'a écrite pendant la session.
    Written,
    /// Ni l'un ni l'autre : absente de la session.
    Unseen,
}

pub(crate) fn mark_verdict(mark: &str, session: &str) -> MarkVerdict {
    match mark.strip_prefix(session) {
        Some("") => MarkVerdict::Carried,
        Some(rest) if rest == WRITTEN_SUFFIX => MarkVerdict::Written,
        _ => MarkVerdict::Unseen,
    }
}

/// La raison d'une ligne gardée parce qu'elle a quitté le périmètre de la
/// session entre le plan et l'application.
const QUITTE_LE_PERIMETRE: &str = "a quitté le périmètre depuis le plan";

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

/// **Une session ouverte** sur un périmètre, rendue par `begin_snapshot`.
/// L'identifiant est généré par le moteur : un appelant ne peut ni le
/// réutiliser ni le partager par mégarde.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSession {
    pub session: String,
    /// Millisecondes depuis l'époque Unix.
    pub opened_at: i64,
    /// La session abandonnée que `takeover` a remplacée, s'il y en avait une.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced: Option<String>,
}

/// Une durée lisible : c'est sur elle qu'un appelant décide d'un `takeover`.
fn elapsed_since(opened_at: i64) -> String {
    let secs = ((now_ms() - opened_at).max(0) / 1000) as u64;
    match secs {
        0..=59 => format!("{secs} s"),
        60..=3599 => format!("{} min {} s", secs / 60, secs % 60),
        3600..=86399 => format!("{} h {} min", secs / 3600, (secs % 3600) / 60),
        _ => format!("{} j {} h", secs / 86400, (secs % 86400) / 3600),
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Ce qu'une fin de synchronisation a fait — une absence se nomme.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotFinish {
    pub entity: String,
    /// Le périmètre de la session.
    pub scope: BTreeMap<String, CypherValue>,
    pub session: String,
    /// Les lignes du périmètre en base au moment de la fin.
    pub in_scope: usize,
    /// Celles qu'un lot de la session a portées.
    pub seen: usize,
    /// Celles qu'une **écriture** ordinaire a marquées pendant la session,
    /// sans qu'un lot les porte : présentes elles aussi, comptées à part pour
    /// que `seen` dise ce que la session a vraiment porté.
    #[serde(default)]
    pub written: usize,
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
    // ── Les sessions : une à la fois par périmètre ───────────────────────
    //
    // Comme le verrou d'état de Terraform ou la table de verrou de Flyway :
    // une session ouverte par périmètre, prise au début, rendue à la fin ou à
    // l'abandon ; une session abandonnée se reprend par un geste explicite
    // (`takeover`), jamais par un délai qui expire seul — un mécanisme qui
    // supprime ne devine pas qu'un appelant est parti. Deux périmètres
    // différents se synchronisent en même temps sans se gêner.

    /// La clé de la session ouverte : la **cellule courante** (`_org`,
    /// `_project`), l'entité, les valeurs du périmètre. Deux cellules ne se
    /// touchent pas : elles synchronisent le même périmètre en même temps.
    fn session_key(&self, entity_name: &str, scope: &BTreeMap<String, CypherValue>) -> String {
        let scope = serde_json::to_string(scope).unwrap_or_default();
        format!("snapshot_session:{}/{}:{entity_name}:{scope}", self.scope.org, self.scope.project)
    }

    /// La session ouverte sur ce périmètre, s'il y en a une.
    pub fn open_snapshot_session(
        &self,
        entity_name: &str,
        scope: &BTreeMap<String, CypherValue>,
    ) -> Result<Option<SnapshotSession>, CatalogError> {
        match self.read_meta_key(&self.session_key(entity_name, scope))? {
            Some(v) if !v.is_empty() => serde_json::from_str(&v)
                .map(Some)
                .map_err(|e| CatalogError::DbError(format!("session de synchronisation illisible : {e}"))),
            _ => Ok(None),
        }
    }

    fn check_scope_declared(
        &self,
        entity_name: &str,
        scope: &BTreeMap<String, CypherValue>,
    ) -> Result<(), CatalogError> {
        let config = self.snapshot_config(entity_name)?;
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
        Ok(())
    }

    /// Refuse une session qui n'est pas celle ouverte sur ce périmètre.
    fn check_open_session(
        &self,
        entity_name: &str,
        scope: &BTreeMap<String, CypherValue>,
        session: &str,
    ) -> Result<SnapshotSession, CatalogError> {
        match self.open_snapshot_session(entity_name, scope)? {
            Some(open) if open.session == session => Ok(open),
            Some(open) => Err(CatalogError::SnapshotRefused(format!(
                "{entity_name} : la session '{session}' n'est pas celle ouverte sur ce périmètre ('{}', ouverte depuis {}) — \
                 périmée, reprise par une autre, ou d'un autre périmètre",
                open.session, elapsed_since(open.opened_at)
            ))),
            None => Err(CatalogError::SnapshotRefused(format!(
                "{entity_name} : aucune session ouverte sur ce périmètre — begin_snapshot d'abord (la session '{session}' est fermée ou n'a jamais existé)"
            ))),
        }
    }

    /// **Ouvrir une session** sur un périmètre. Refusé si une session y est
    /// déjà ouverte, en disant laquelle et depuis quand ; `takeover` la
    /// remplace (un appelant qui reprend une synchronisation abandonnée).
    pub fn begin_snapshot(
        &mut self,
        entity_name: &str,
        scope: &BTreeMap<String, CypherValue>,
        takeover: bool,
    ) -> Result<SnapshotSession, CatalogError> {
        self.check_initialized()?;
        self.check_ecriture("begin_snapshot")?;
        self.check_entity(entity_name)?;
        self.check_scope_declared(entity_name, scope)?;
        let replaced = match self.open_snapshot_session(entity_name, scope)? {
            Some(open) if !takeover => {
                return Err(CatalogError::SnapshotRefused(format!(
                    "{entity_name} : une session est déjà ouverte sur ce périmètre ('{}', ouverte depuis {}) ; \
                     une seule à la fois — la finir, l'abandonner, ou takeover pour la reprendre",
                    open.session, elapsed_since(open.opened_at)
                )))
            }
            Some(open) => Some(open.session),
            None => None,
        };
        let opened_at = now_ms();
        let seed = format!("{entity_name}|{:?}|{opened_at}|{}|{:?}", scope, std::process::id(), replaced);
        let session = format!("{opened_at}-{}", &blake3::hash(seed.as_bytes()).to_hex()[..12]);
        if session.contains(WRITTEN_SUFFIX) {
            return Err(CatalogError::DbError(format!(
                "identifiant de session « {session} » : il contient « {WRITTEN_SUFFIX} », réservé à la marque d'une écriture"
            )));
        }
        let open = SnapshotSession { session, opened_at, replaced };
        let value = serde_json::to_string(&SnapshotSession { replaced: None, ..open.clone() })
            .map_err(|e| CatalogError::DbError(e.to_string()))?;
        self.persist_meta_key(&self.session_key(entity_name, scope), &value)?;
        self.oublier_les_sessions(entity_name);
        Ok(open)
    }

    /// **Abandonner une session** : elle se ferme, rien n'est retiré.
    pub fn abort_snapshot(
        &mut self,
        entity_name: &str,
        scope: &BTreeMap<String, CypherValue>,
        session: &str,
    ) -> Result<(), CatalogError> {
        self.check_initialized()?;
        self.check_ecriture("abort_snapshot")?;
        self.check_entity(entity_name)?;
        self.check_open_session(entity_name, scope, session)?;
        self.persist_meta_key(&self.session_key(entity_name, scope), "")?;
        self.oublier_les_sessions(entity_name);
        Ok(())
    }

    /// **Marquer des lignes comme portées par la session ouverte** sur leur
    /// périmètre — un `SET` par appel, quel que soit le nombre de lignes
    /// (mesuré : 53 lots de 512 en 324 ms). Une ligne qui reparaît perd sa
    /// marque d'absence.
    pub fn mark_snapshot(
        &mut self,
        entity_name: &str,
        scope: &BTreeMap<String, CypherValue>,
        session: &str,
        uuids: &[String],
    ) -> Result<(), CatalogError> {
        self.check_initialized()?;
        self.check_ecriture("mark_snapshot")?;
        self.check_entity(entity_name)?;
        self.check_open_session(entity_name, scope, session)?;
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
        self.check_scope_declared(entity_name, scope)?;
        self.check_open_session(entity_name, scope, session)?;

        let entity_config = self
            .entity_configs()
            .get(entity_name)
            .cloned()
            .ok_or_else(|| CatalogError::UnknownEntity(entity_name.to_string()))?;
        // Le périmètre, **dans la cellule courante** : une synchronisation ne
        // voit ni ne retire les lignes d'une autre cellule.
        let mut scope_fields: Vec<&str> = config.scope.iter().map(String::as_str).collect();
        let mut values: Vec<CypherValue> = config.scope.iter().map(|f| scope[f].clone()).collect();
        scope_fields.extend(["_org", "_project"]);
        values.push(CypherValue::String(self.scope.org.clone()));
        values.push(CypherValue::String(self.scope.project.clone()));
        let cypher = self.dialect.select_snapshot_scope(entity_name, &scope_fields, &[]);
        let params: Vec<QueryParam> = values
            .into_iter()
            .enumerate()
            .map(|(i, v)| QueryParam::new(&format!("p{i}"), v))
            .collect();
        let rows = self
            .conn
            .execute_with_params(&cypher, &params)
            .map_err(|e| CatalogError::DbError(e.to_string()))?
            .rows;

        let mut report = SnapshotFinish {
            entity: entity_name.to_string(),
            scope: scope.clone(),
            session: session.to_string(),
            in_scope: rows.len(),
            ..Default::default()
        };
        for row in &rows {
            let uuid = row.first().and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let mark = row.get(1).and_then(|v| v.as_str()).unwrap_or_default();
            match mark_verdict(mark, session) {
                MarkVerdict::Carried => report.seen += 1,
                MarkVerdict::Written => report.written += 1,
                MarkVerdict::Unseen => report.missing.push(uuid),
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
        self.check_initialized()?;
        self.check_ecriture("finish_snapshot")?;
        self.check_entity(&entity_name)?;
        // La session est toujours celle ouverte : un plan d'une session
        // reprise ou fermée entre-temps ne s'applique pas.
        self.check_open_session(&entity_name, &report.scope, &report.session)?;
        // Une ligne planifiée pour le retrait que la session a portée depuis
        // le plan (un lot arrivé entre les deux) a reparu : elle reste. Une
        // ligne déjà partie n'est pas annoncée retirée par cette fin.
        if !report.removed.is_empty() {
            // La marque, et l'appartenance au périmètre relue : une ligne que
            // l'autre session d'un autre périmètre a portée depuis le plan vit
            // ailleurs, sa marque n'est simplement plus la nôtre.
            let marks: HashMap<String, (String, bool)> = self
                .get_many(&entity_name, &report.removed)?
                .into_iter()
                .filter_map(|row| {
                    let uuid = row.get("_uuid")?.as_str()?.to_string();
                    let mark = row.get("_snapshot").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                    let dedans = self.dans_le_perimetre(&row, &report.scope);
                    Some((uuid, (mark, dedans)))
                })
                .collect();
            let planned = std::mem::take(&mut report.removed);
            for uuid in planned {
                match marks.get(&uuid) {
                    None => report.kept.push((uuid, "introuvable à la relecture".into())),
                    Some((mark, _)) if mark_verdict(mark, &report.session) != MarkVerdict::Unseen => {
                        report.kept.push((uuid, "reparue depuis le plan (portée ou écrite pendant la session)".into()))
                    }
                    Some((_, false)) => report.kept.push((uuid, QUITTE_LE_PERIMETRE.into())),
                    Some(_) => {
                        self.mettre_en_file_la_suppression(&entity_name, &uuid)?;
                        report.removed.push(uuid);
                    }
                }
            }
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
                if !self.dans_le_perimetre(row, &report.scope) {
                    report.kept.push((uuid, QUITTE_LE_PERIMETRE.into()));
                    continue;
                }
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
            // Les écritures de la fin elle-même — ses transitions — ne sont
            // pas des vues : elles ne marquent pas. Sinon la fin suivante
            // croirait présentes les lignes qu'elle vient d'archiver.
            self.dans_une_fin = true;
            let res = self.drain();
            self.dans_une_fin = false;
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
            report.warnings.extend(res.warnings);
        }
        // La marque d'absence : posée sur les lignes que l'absence a fait
        // changer d'état, à la première absence constatée seulement.
        if !report.transitioned.is_empty() {
            let cypher = self.dialect.mark_absent_since(&entity_name);
            self.conn
                .execute_with_params(&cypher, &[
                    QueryParam::new("uuids", CypherValue::List(report.transitioned.iter().cloned().map(CypherValue::String).collect())),
                    QueryParam::new("since", CypherValue::Int(now_ms())),
                ])
                .map_err(|e| CatalogError::DbError(e.to_string()))?;
        }
        // La fin ferme la session.
        self.persist_meta_key(&self.session_key(&entity_name, &report.scope), "")?;
        self.oublier_les_sessions(&entity_name);
        Ok(report)
    }

    // ── La marque à l'écriture ──────────────────────────────────────────
    //
    // Une écriture dans un périmètre dont une session est ouverte prend la
    // marque de cette session (`{session}+w`) : écrire une ligne, c'est dire
    // que la source l'a. Le prix, assumé : un écrivain qui réécrit
    // périodiquement une ligne que la source n'a plus empêche son retrait,
    // sans bruit — la rétention plutôt que la suppression à tort.

    fn cle_des_sessions(&self, entity_name: &str) -> String {
        format!("{}/{}:{entity_name}", self.scope.org, self.scope.project)
    }

    fn oublier_les_sessions(&mut self, entity_name: &str) {
        let cle = self.cle_des_sessions(entity_name);
        self.sessions_ouvertes.remove(&cle);
    }

    /// Les sessions ouvertes de l'entité dans la cellule courante, par
    /// valeurs de périmètre (JSON, comme dans la clé de session). Une lecture
    /// de `_catalog_meta` au premier besoin, puis le cache.
    fn sessions_ouvertes_de(&mut self, entity_name: &str) -> Result<HashMap<String, String>, CatalogError> {
        let cle = self.cle_des_sessions(entity_name);
        if let Some(s) = self.sessions_ouvertes.get(&cle) {
            return Ok(s.clone());
        }
        let prefixe = format!("snapshot_session:{cle}:");
        let res = self
            .conn
            .execute_with_params(
                &self.dialect.load_meta_by_prefix("prefix"),
                &[QueryParam::new("prefix", CypherValue::String(prefixe.clone()))],
            )
            .map_err(|e| CatalogError::DbError(e.to_string()))?;
        let mut ouvertes = HashMap::new();
        for row in res.rows {
            let (Some(k), Some(v)) = (row.first().and_then(|v| v.as_str()), row.get(1).and_then(|v| v.as_str())) else {
                continue;
            };
            let Some(scope_json) = k.strip_prefix(&prefixe) else { continue };
            if let Ok(open) = serde_json::from_str::<SnapshotSession>(v) {
                ouvertes.insert(scope_json.to_string(), open.session);
            }
        }
        self.sessions_ouvertes.insert(cle, ouvertes.clone());
        Ok(ouvertes)
    }

    /// **Marquer les lignes écrites** (`uuids`, de l'entité) de la session
    /// ouverte sur leur périmètre, s'il y en a une. Aucun coût — ni lecture de
    /// ligne, ni écriture — quand l'entité ne déclare pas `snapshot` ou
    /// qu'aucune session n'est ouverte sur elle dans cette cellule ; sinon,
    /// une relecture des lignes écrites et un SET par session touchée.
    pub(crate) fn marquer_les_ecritures(&mut self, entity_name: &str, uuids: &[String]) -> Result<(), CatalogError> {
        if uuids.is_empty() || self.dans_une_fin {
            return Ok(());
        }
        let Some(config) = self.entity_configs().get(entity_name).and_then(|c| c.snapshot.clone()) else {
            return Ok(());
        };
        let ouvertes = self.sessions_ouvertes_de(entity_name)?;
        if ouvertes.is_empty() {
            return Ok(());
        }
        let mut par_session: HashMap<String, Vec<String>> = HashMap::new();
        for row in self.get_many(entity_name, uuids)? {
            let Some(uuid) = row.get("_uuid").and_then(|v| v.as_str()) else { continue };
            let cellule = |champ: &str, attendu: &str| row.get(champ).and_then(|v| v.as_str()) == Some(attendu);
            if !(cellule("_org", &self.scope.org) && cellule("_project", &self.scope.project)) {
                continue;
            }
            let perimetre: BTreeMap<String, CypherValue> = config
                .scope
                .iter()
                .filter_map(|f| Some((f.clone(), row.get(f)?.clone())))
                .collect();
            let json = serde_json::to_string(&perimetre).unwrap_or_default();
            if let Some(session) = ouvertes.get(&json) {
                par_session.entry(session.clone()).or_default().push(uuid.to_string());
            }
        }
        for (session, uuids) in par_session {
            self.conn
                .execute_with_params(&self.dialect.mark_written_session(entity_name), &[
                    QueryParam::new("uuids", CypherValue::List(uuids.into_iter().map(CypherValue::String).collect())),
                    QueryParam::new("mark", CypherValue::String(format!("{session}{WRITTEN_SUFFIX}"))),
                    QueryParam::new("session", CypherValue::String(session)),
                ])
                .map_err(|e| CatalogError::DbError(e.to_string()))?;
        }
        Ok(())
    }

    /// **La ligne est-elle encore dans ce périmètre**, dans la cellule
    /// courante ? Relu à l'application : entre le plan et elle, une ligne a pu
    /// changer de périmètre — portée par la session d'un autre, ou mise à jour.
    fn dans_le_perimetre(&self, row: &BTreeMap<String, CypherValue>, scope: &BTreeMap<String, CypherValue>) -> bool {
        let cellule = |champ: &str, attendu: &str| row.get(champ).and_then(|v| v.as_str()) == Some(attendu);
        scope.iter().all(|(champ, valeur)| row.get(champ) == Some(valeur))
            && cellule("_org", &self.scope.org)
            && cellule("_project", &self.scope.project)
    }

    fn snapshot_config(&self, entity_name: &str) -> Result<&crate::config::SnapshotConfig, CatalogError> {
        self.entity_configs()
            .get(entity_name)
            .and_then(|c| c.snapshot.as_ref())
            .ok_or_else(|| CatalogError::ValidationFailed(format!("{entity_name} ne déclare pas de synchronisation (snapshot)")))
    }
}
