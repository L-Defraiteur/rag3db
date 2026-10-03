//! **La mise de côté avant purge** (`keepFor`, 3 octobre 2026).
//!
//! Une fin de synchronisation qui retire des lignes les retire vraiment — rien
//! ne reste dans les tables vivantes ni dans les index. Avant, elle en copie
//! l'essentiel dans `_snapshot_aside` : la ligne entière, et les vecteurs
//! denses de ses chunks avec le `_text_hash` qu'ils embarquent.
//!
//! Une ligne qui reparaît avec la **même identité et le même
//! `_content_hash`** s'ingère par le chemin habituel — le découpage est
//! déterministe et bon marché — mais ses chunks retrouvent leurs vecteurs au
//! lieu d'être réembarqués : `EmbedNode` reçoit les vecteurs connus
//! ([`KnownVectors`]) et ne recalcule que ce qui manque (le creux, qui ne vit
//! que dans l'index lucistore et ne se relit pas). La copie est consommée.
//!
//! Une fin appliquée devient réversible tant que ses copies vivent :
//! [`Catalog::undo_snapshot_finish`] réingère les lignes retirées et rend leur
//! état d'avant aux lignes transitionnées. Les copies plus vieilles que
//! `keepFor` sont purgées par passes bornées, **vidées par SET** : rag3db ne
//! récupère pas la place d'une ligne supprimée.
//!
//! Page de conception : `docs/3-octobre-2026-15h04/01-mise-de-cote-avant-purge.md`.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use super::synchronisation::SnapshotFinish;
use super::{Catalog, CatalogError};
use crate::config::OnMissing;
use crate::connection::{CypherValue, QueryParam};

/// Une purge vide au plus ce nombre de copies par passe : elle ne bloque
/// jamais une écriture.
const PURGE_BATCH: usize = 512;

/// Le vecteur dense d'un chunk, tel qu'il était au retrait.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AsideChunk {
    uuid: String,
    text_hash: String,
    vector: Vec<f32>,
}

/// Les vecteurs denses des chunks mis de côté, pour un modèle.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AsideChunks {
    /// Le modèle qui les a calculés : un vecteur d'un autre modèle ne se rend
    /// pas.
    slug: String,
    chunks: Vec<AsideChunk>,
}

/// **Les vecteurs connus**, rendus à `EmbedNode` pendant une ingestion : par
/// uuid de chunk, le `_text_hash` qu'ils embarquent et le vecteur. Un chunk
/// dont le texte a changé n'y trouve rien.
#[derive(Debug, Clone, Default)]
pub struct KnownVectors {
    pub slug: String,
    pub by_chunk: HashMap<String, (String, Vec<f32>)>,
}

/// Ce que la mise de côté rend à une ingestion.
#[derive(Debug, Default)]
pub(crate) struct AsideReturn {
    /// Les lignes rendues (même contenu qu'au retrait).
    pub restored: Vec<String>,
    pub known: KnownVectors,
    /// Les copies lues — rendues ou périmées — à vider après l'écriture.
    pub consumed: Vec<String>,
}

/// Le service qui porte les [`KnownVectors`] d'une ingestion.
pub const SERVICE_KNOWN_VECTORS: &str = "known_vectors";

/// Ce qu'une annulation en bloc a fait — une absence se nomme.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotUndo {
    pub entity: String,
    pub scope: BTreeMap<String, CypherValue>,
    pub session: String,
    /// Les lignes retirées par la fin, rendues.
    pub restored: Vec<String>,
    /// Les lignes transitionnées par la fin, revenues à leur état d'avant.
    pub reverted: Vec<String>,
    /// Ce qui n'a pas été rendu, avec la raison.
    pub kept: Vec<(String, String)>,
    pub warnings: Vec<String>,
}

/// Une fin appliquée, gardée pour son annulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FinishRecord {
    report: SnapshotFinish,
    #[serde(default)]
    undone: bool,
}

pub(super) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn aside_key(entity_name: &str, uuid: &str) -> String {
    format!("{entity_name}:{uuid}")
}

fn as_vector(v: &CypherValue) -> Option<Vec<f32>> {
    match v {
        CypherValue::List(items) => items
            .iter()
            .map(|x| match x {
                CypherValue::Float(f) => Some(*f as f32),
                CypherValue::Int(i) => Some(*i as f32),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

impl Catalog {
    /// `keepFor` de l'entité, en millisecondes ; `0` sans synchronisation
    /// déclarée ou quand la mise de côté est coupée.
    fn keep_for_ms_of(&self, entity_name: &str) -> i64 {
        self.entity_configs()
            .get(entity_name)
            .and_then(|c| c.snapshot.as_ref())
            .and_then(|s| s.keep_for_ms().ok())
            .unwrap_or(0)
    }

    /// La clé de la fin appliquée d'une session, dans la cellule courante.
    fn finish_key(&self, entity_name: &str, session: &str) -> String {
        format!("snapshot_finish:{}/{}:{entity_name}:{session}", self.scope.org, self.scope.project)
    }

    /// **Garder une fin appliquée**, pour pouvoir l'annuler.
    pub(super) fn record_snapshot_finish(&self, report: &SnapshotFinish) -> Result<(), CatalogError> {
        let value = serde_json::to_string(&FinishRecord { report: report.clone(), undone: false })
            .map_err(|e| CatalogError::DbError(e.to_string()))?;
        self.persist_meta_key(&self.finish_key(&report.entity, &report.session), &value)
    }

    /// **Copier des lignes avant leur retrait** : la ligne entière, et les
    /// vecteurs denses à jour de leurs chunks pour le modèle courant. Rend
    /// les uuids copiés ; rien sous `keepFor: 0`.
    pub(super) fn set_aside(
        &mut self,
        entity_name: &str,
        session: &str,
        rows: &[BTreeMap<String, CypherValue>],
    ) -> Result<Vec<String>, CatalogError> {
        if rows.is_empty() || self.keep_for_ms_of(entity_name) == 0 {
            return Ok(Vec::new());
        }
        let uuids: Vec<String> = rows
            .iter()
            .filter_map(|r| r.get("_uuid").and_then(|v| v.as_str()).map(str::to_string))
            .collect();

        // Les vecteurs denses, seulement ceux qui sont à jour (marqueur égal au
        // hash du texte) : un vecteur en retard ne vaut pas d'être gardé.
        let mut by_parent: HashMap<String, Vec<AsideChunk>> = HashMap::new();
        let mut slug = String::new();
        let signals = self.entity_configs().get(entity_name).map(|c| c.signals);
        if signals.is_some_and(|s| s.vector()) {
            let chunk_table = format!("{entity_name}_Chunk");
            if let Ok(storage) = self.vector_storage(&chunk_table) {
                if let Some(cypher) = self.dialect.select_chunk_vectors(&chunk_table, &storage.column, &storage.marker) {
                    slug = self.current_embedding_slug();
                    let list = CypherValue::List(uuids.iter().cloned().map(CypherValue::String).collect());
                    let res = self
                        .conn
                        .execute_with_params(&cypher, &[QueryParam::new("uuids", list)])
                        .map_err(|e| CatalogError::DbError(e.to_string()))?;
                    for row in res.rows {
                        let (Some(uuid), Some(parent), Some(hash)) = (
                            row.first().and_then(|v| v.as_str()),
                            row.get(1).and_then(|v| v.as_str()),
                            row.get(2).and_then(|v| v.as_str()),
                        ) else {
                            continue;
                        };
                        let marker = row.get(3).and_then(|v| v.as_str()).unwrap_or_default();
                        let Some(vector) = row.get(4).and_then(as_vector) else { continue };
                        if marker != hash || vector.len() != storage.dim {
                            continue;
                        }
                        by_parent.entry(parent.to_string()).or_default().push(AsideChunk {
                            uuid: uuid.to_string(),
                            text_hash: hash.to_string(),
                            vector,
                        });
                    }
                }
            }
        }

        let at = now_ms();
        let mut items = Vec::with_capacity(rows.len());
        let mut copied = Vec::with_capacity(rows.len());
        for row in rows {
            let Some(uuid) = row.get("_uuid").and_then(|v| v.as_str()) else { continue };
            let hash = row.get("_content_hash").and_then(|v| v.as_str()).unwrap_or_default();
            let kept: BTreeMap<&String, &CypherValue> =
                row.iter().filter(|(k, _)| k.as_str() != "_id" && k.as_str() != "_label").collect();
            let row_json = serde_json::to_string(&kept).map_err(|e| CatalogError::DbError(e.to_string()))?;
            let chunks = AsideChunks { slug: slug.clone(), chunks: by_parent.remove(uuid).unwrap_or_default() };
            let chunks_json = serde_json::to_string(&chunks).map_err(|e| CatalogError::DbError(e.to_string()))?;
            let mut m = BTreeMap::new();
            m.insert("key".into(), CypherValue::String(aside_key(entity_name, uuid)));
            m.insert("entity".into(), CypherValue::String(entity_name.to_string()));
            m.insert("uuid".into(), CypherValue::String(uuid.to_string()));
            m.insert("hash".into(), CypherValue::String(hash.to_string()));
            m.insert("row".into(), CypherValue::String(row_json));
            m.insert("chunks".into(), CypherValue::String(chunks_json));
            m.insert("session".into(), CypherValue::String(session.to_string()));
            m.insert("at".into(), CypherValue::Int(at));
            items.push(CypherValue::Map(m));
            copied.push(uuid.to_string());
        }
        self.conn
            .execute_with_params(&self.dialect.upsert_aside(), &[QueryParam::new("items", CypherValue::List(items))])
            .map_err(|e| CatalogError::DbError(e.to_string()))?;
        Ok(copied)
    }

    /// Vider des copies : consommées, périmées, ou celles d'une ligne dont
    /// le retrait a échoué.
    pub(crate) fn clear_aside(&self, entity_name: &str, uuids: &[String]) -> Result<(), CatalogError> {
        if uuids.is_empty() {
            return Ok(());
        }
        let keys = CypherValue::List(uuids.iter().map(|u| CypherValue::String(aside_key(entity_name, u))).collect());
        self.conn
            .execute_with_params(&self.dialect.clear_aside(), &[QueryParam::new("keys", keys)])
            .map_err(|e| CatalogError::DbError(e.to_string()))?;
        Ok(())
    }

    /// **Le retour, décidé avant l'ingestion** : pour chaque ligne du lot
    /// qui a une copie, même `_content_hash` → ses vecteurs sont rendus à
    /// l'embarquement et elle est dite rendue ; hash différent → la copie est
    /// périmée. `records` : les paires (uuid, `_content_hash`) des lignes qui
    /// vont s'écrire. Rien n'est vidé ici : [`AsideReturn::consumed`] se vide
    /// une fois l'écriture faite — une ingestion qui échoue ne perd pas la
    /// copie.
    pub(crate) fn take_from_aside(
        &mut self,
        entity_name: &str,
        records: &[(String, String)],
    ) -> Result<AsideReturn, CatalogError> {
        let mut known = KnownVectors::default();
        let has_snapshot = self.entity_configs().get(entity_name).is_some_and(|c| c.snapshot.is_some());
        if records.is_empty() || !has_snapshot {
            return Ok(AsideReturn::default());
        }
        let keys = CypherValue::List(records.iter().map(|(u, _)| CypherValue::String(aside_key(entity_name, u))).collect());
        let res = self
            .conn
            .execute_with_params(&self.dialect.select_aside(), &[QueryParam::new("keys", keys)])
            .map_err(|e| CatalogError::DbError(e.to_string()))?;
        if res.rows.is_empty() {
            return Ok(AsideReturn::default());
        }
        let incoming: HashMap<&str, &str> = records.iter().map(|(u, h)| (u.as_str(), h.as_str())).collect();
        let current_slug = self.current_embedding_slug();
        let mut restored = Vec::new();
        let mut consumed = Vec::new();
        for row in res.rows {
            let (Some(uuid), Some(hash)) = (row.first().and_then(|v| v.as_str()), row.get(1).and_then(|v| v.as_str())) else {
                continue;
            };
            consumed.push(uuid.to_string());
            if incoming.get(uuid) != Some(&hash) {
                continue;
            }
            restored.push(uuid.to_string());
            let chunks: AsideChunks = row
                .get(3)
                .and_then(|v| v.as_str())
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            if chunks.slug.is_empty() || chunks.slug != current_slug {
                continue;
            }
            known.slug = chunks.slug;
            for c in chunks.chunks {
                known.by_chunk.insert(c.uuid, (c.text_hash, c.vector));
            }
        }
        restored.sort();
        Ok(AsideReturn { restored, known, consumed })
    }

    /// **La purge** des copies de l'entité plus vieilles que `keepFor`, par
    /// passe bornée. Rend le nombre de copies vidées.
    pub fn purge_snapshot_aside(&mut self, entity_name: &str) -> Result<usize, CatalogError> {
        self.purge_snapshot_aside_at(entity_name, now_ms())
    }

    /// [`purge_snapshot_aside`](Self::purge_snapshot_aside) à un instant
    /// donné — le temps se choisit dans un test, il ne s'attend pas.
    pub fn purge_snapshot_aside_at(&mut self, entity_name: &str, now: i64) -> Result<usize, CatalogError> {
        self.check_initialized()?;
        self.check_entity(entity_name)?;
        let before = now - self.keep_for_ms_of(entity_name);
        let mut total = 0usize;
        loop {
            let res = self
                .conn
                .execute_with_params(&self.dialect.purge_aside_before(PURGE_BATCH), &[
                    QueryParam::new("entity", CypherValue::String(entity_name.to_string())),
                    QueryParam::new("before", CypherValue::Int(before)),
                ])
                .map_err(|e| CatalogError::DbError(e.to_string()))?;
            let n = res.rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            total += n;
            if n < PURGE_BATCH {
                return Ok(total);
            }
        }
    }

    /// **Annuler une fin appliquée, en bloc** : les lignes qu'elle a retirées
    /// sont réingérées depuis leur copie (sans réembarquement), les lignes
    /// qu'elle a fait passer par une transition retrouvent leur état d'avant
    /// et perdent leur marque d'absence. Possible tant que les copies n'ont
    /// pas été purgées — au plus `keepFor` ; une fin s'annule une fois.
    ///
    /// Une ligne rendue revient **sans ses relations** : `DETACH DELETE` les a
    /// emportées, et l'autre extrémité a pu changer. C'est la synchronisation
    /// des relations qui les repose. L'état d'avant est écrit sans la garde
    /// de la machine à états — une annulation n'est pas une transition — et le
    /// document plein texte de la ligne garde l'état transitionné jusqu'à sa
    /// prochaine écriture.
    pub fn undo_snapshot_finish(
        &mut self,
        entity_name: &str,
        scope: &BTreeMap<String, CypherValue>,
        session: &str,
    ) -> Result<SnapshotUndo, CatalogError> {
        self.check_initialized()?;
        self.check_ecriture("undo_snapshot_finish")?;
        self.check_entity(entity_name)?;
        let key = self.finish_key(entity_name, session);
        let mut record: FinishRecord = match self.read_meta_key(&key)? {
            Some(v) if !v.is_empty() => serde_json::from_str(&v)
                .map_err(|e| CatalogError::DbError(format!("fin de synchronisation illisible : {e}")))?,
            _ => {
                return Err(CatalogError::SnapshotRefused(format!(
                    "{entity_name} : aucune fin appliquée connue pour la session '{session}' dans cette cellule"
                )))
            }
        };
        if &record.report.scope != scope {
            return Err(CatalogError::SnapshotRefused(format!(
                "{entity_name} : la session '{session}' a fini un autre périmètre ({:?})",
                record.report.scope
            )));
        }
        if record.undone {
            return Err(CatalogError::SnapshotRefused(format!(
                "{entity_name} : la fin de la session '{session}' est déjà annulée"
            )));
        }
        let report = record.report.clone();
        let mut undo = SnapshotUndo {
            entity: entity_name.to_string(),
            scope: scope.clone(),
            session: session.to_string(),
            ..Default::default()
        };

        // ── Les retirées : réingérées depuis leur copie ─────────────────
        if !report.set_aside.is_empty() {
            let res = self
                .conn
                .execute_with_params(&self.dialect.select_aside_by_session(), &[
                    QueryParam::new("entity", CypherValue::String(entity_name.to_string())),
                    QueryParam::new("session", CypherValue::String(session.to_string())),
                ])
                .map_err(|e| CatalogError::DbError(e.to_string()))?;
            let mut rows: HashMap<String, BTreeMap<String, CypherValue>> = HashMap::new();
            for row in res.rows {
                let (Some(uuid), Some(json)) = (row.first().and_then(|v| v.as_str()), row.get(2).and_then(|v| v.as_str())) else {
                    continue;
                };
                if let Ok(r) = serde_json::from_str::<BTreeMap<String, CypherValue>>(json) {
                    rows.insert(uuid.to_string(), r);
                }
            }
            if rows.is_empty() && report.transitioned.is_empty() {
                return Err(CatalogError::SnapshotRefused(format!(
                    "{entity_name} : rien à rendre pour la session '{session}' — ses copies ont été purgées \
                     (keepFor) ou consommées par un retour"
                )));
            }
            let mut records = Vec::new();
            let mut order = Vec::new();
            for uuid in &report.set_aside {
                let Some(row) = rows.get(uuid) else {
                    undo.kept.push((uuid.clone(), "copie purgée ou déjà consommée".into()));
                    continue;
                };
                let data: BTreeMap<String, CypherValue> = row
                    .iter()
                    .filter(|(k, v)| !k.starts_with('_') && !crate::scope::is_scope_column(k) && !matches!(v, CypherValue::Null))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                records.push(data);
                order.push(uuid.clone());
            }
            if !records.is_empty() {
                let res = self.ingest_entities(entity_name, records)?;
                undo.warnings.extend(res.warnings);
                let present: std::collections::HashSet<String> = self
                    .get_many(entity_name, &order)?
                    .into_iter()
                    .filter_map(|r| r.get("_uuid").and_then(|v| v.as_str()).map(str::to_string))
                    .collect();
                for uuid in order {
                    if present.contains(&uuid) {
                        undo.restored.push(uuid);
                    } else {
                        undo.kept.push((uuid, "réingestion refusée".into()));
                    }
                }
            }
        }

        // ── Les transitionnées : leur état d'avant ──────────────────────
        if !report.transitioned.is_empty() {
            let entity_config = self
                .entity_configs()
                .get(entity_name)
                .cloned()
                .ok_or_else(|| CatalogError::UnknownEntity(entity_name.to_string()))?;
            let (lc, name) = match (&entity_config.lifecycle, entity_config.snapshot.as_ref().map(|s| &s.on_missing)) {
                (Some(lc), Some(OnMissing::Transition(name))) => (lc.clone(), name.clone()),
                _ => return Err(CatalogError::SchemaError("snapshot : transitions à annuler sans transition déclarée".into())),
            };
            let to = lc.transitions.iter().find(|t| t.name == name).map(|t| t.to.clone()).unwrap_or_default();
            let current: HashMap<String, BTreeMap<String, CypherValue>> = self
                .get_many(entity_name, &report.transitioned)?
                .into_iter()
                .filter_map(|row| Some((row.get("_uuid")?.as_str()?.to_string(), row)))
                .collect();
            let mut items = Vec::new();
            for uuid in &report.transitioned {
                let Some(row) = current.get(uuid) else {
                    undo.kept.push((uuid.clone(), "introuvable".into()));
                    continue;
                };
                let state = row.get(&lc.field).and_then(|v| v.as_str()).unwrap_or_default();
                // Une ligne qui a bougé depuis la fin n'est pas ramenée en
                // arrière : son état d'aujourd'hui est une décision plus récente.
                if state != to {
                    undo.kept.push((uuid.clone(), format!("a changé depuis la fin (état '{state}')")));
                    continue;
                }
                let Some(before) = report.previous_states.get(uuid) else {
                    undo.kept.push((uuid.clone(), "état d'avant inconnu".into()));
                    continue;
                };
                let mut m = BTreeMap::new();
                m.insert("uuid".into(), CypherValue::String(uuid.clone()));
                m.insert("state".into(), CypherValue::String(before.clone()));
                items.push(CypherValue::Map(m));
                undo.reverted.push(uuid.clone());
            }
            if !items.is_empty() {
                self.conn
                    .execute_with_params(
                        &self.dialect.revert_lifecycle_state(entity_name, &lc.field),
                        &[QueryParam::new("items", CypherValue::List(items))],
                    )
                    .map_err(|e| CatalogError::DbError(e.to_string()))?;
            }
        }

        record.undone = true;
        let value = serde_json::to_string(&record).map_err(|e| CatalogError::DbError(e.to_string()))?;
        self.persist_meta_key(&key, &value)?;
        Ok(undo)
    }
}
