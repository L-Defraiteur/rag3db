//! Record types for the dataflow ingestion pipeline.
//!
//! These replace the `CatalogOp` enum with typed data records that flow through
//! the dataflow graph. The graph topology encodes the execution plan — records
//! carry only business data, not instructions.
//!
//! See doc 23 — "Design: Elimination des Ops — Le graphe EST le plan".

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::connection::CypherValue;
use crate::refs::{EntityRef, EntityRefResolver, RefError, RelationRef, RelationRefResolver};

// ─── RefOrUuid ──────────────────────────────────────────────────────────────

/// Reference to an entity: either an unresolved `EntityRef` or a known UUID.
///
/// Used by `Catalog::link()` for `from`/`to` endpoints: the caller can pass
/// either an `EntityRef` (whose UUID will be resolved at drain time) or a
/// direct UUID string (for entities already in the DB).
#[derive(Debug, Clone)]
pub enum RefOrUuid {
    Ref(EntityRef),
    Uuid(String),
}

impl RefOrUuid {
    pub fn try_resolve(&self) -> Result<String, RefError> {
        match self {
            Self::Uuid(s) => Ok(s.clone()),
            Self::Ref(r) => r.uuid(),
        }
    }

    pub fn resolve(&mut self) -> Result<String, RefError> {
        match self {
            Self::Uuid(s) => Ok(s.clone()),
            Self::Ref(r) => r.ready(),
        }
    }
}

impl From<EntityRef> for RefOrUuid {
    fn from(r: EntityRef) -> Self {
        Self::Ref(r)
    }
}

impl From<String> for RefOrUuid {
    fn from(s: String) -> Self {
        Self::Uuid(s)
    }
}

impl From<&str> for RefOrUuid {
    fn from(s: &str) -> Self {
        Self::Uuid(s.to_string())
    }
}

// ─── UpdateResult / DeleteResult ───────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Updated,
    Unchanged,
    /// La mise à jour n'a pas abouti — la cause est dans
    /// [`FlushResult::warnings`] et dans le canal d'échecs. Avant le
    /// 6 septembre 2026, ce cas n'existait pas : une ligne refusée faisait
    /// tomber tout le graphe, ou passait pour `Updated`.
    Failed,
}

#[derive(Debug)]
pub struct UpdateResult {
    pub uuid: String,
    pub entity: String,
    pub status: UpdateStatus,
    pub reembedded: bool,
    pub chunks_created: usize,
    pub chunks_deleted: usize,
}

#[derive(Debug)]
pub struct DeleteResult {
    pub uuid: String,
    pub entity: String,
    pub chunks_deleted: usize,
    pub relations_deleted: usize,
    /// `Some(cause)` : la suppression n'a pas abouti.
    pub echec: Option<String>,
}

/// **Un groupe d'enregistrements qui n'a pas abouti**, dit par le nœud qui
/// l'a vu — c'est le canal d'échec par groupe (réconciliation, A5).
///
/// Onze nœuds sur douze n'avaient que deux façons de finir : `?`, qui fait
/// tomber tout le graphe sans annuler ce qui est déjà écrit, ou `ctx.warn`,
/// que personne ne compte. Ici un nœud dit *ce* groupe, *combien*
/// d'opérations, *ce qui* est perdu, *pourquoi* — et continue les autres.
///
/// `operations` compte des opérations **en file** (insertion, lien, mise à
/// jour, suppression, agrégat) : celles-là entrent dans `FlushResult::failed`.
/// Un échec **dérivé** — commit d'index, découpage, embarquement — n'en
/// compte aucune (`operations: 0`) mais retire `perdu` de `rendu_pret` : un
/// drain dont le commit plein texte a raté ne dit plus `textsearch`.
#[derive(Debug, Clone)]
pub struct EchecDeGroupe {
    pub noeud: String,
    pub table: String,
    pub operations: usize,
    pub perdu: crate::disponibilite::Disponibilites,
    pub cause: String,
}

/// Le nom du service qui porte le canal d'échecs :
/// `Arc<Mutex<Vec<EchecDeGroupe>>>`, ouvert par l'appelant du graphe et
/// relu après — le même patron que `update_results` et `delete_results`,
/// écrit **pendant** l'exécution, donc survivant à l'abandon d'une phase.
pub const SERVICE_ECHECS: &str = "echecs";

// ─── FlushResult ────────────────────────────────────────────────────────────

/// Result of a drain/flush cycle.
#[derive(Debug, Default)]
pub struct FlushResult {
    pub processed: usize,
    pub failed: usize,
    /// Combien, parmi les `processed`, n'ont demandé **aucun travail** : déjà en
    /// base, identiques, artefacts dérivés présents.
    ///
    /// Sans ce compte, le court-circuit de l'inchangé rendait exactement ce que
    /// rend une vraie ingestion de la même taille — l'appelant ne pouvait pas
    /// distinguer « j'ai tout réécrit » de « je n'ai rien eu à faire ». C'est
    /// aussi ce que la synchronisation d'un catalogue doit savoir dire :
    /// *93 inchangés, 2 modifiés, 5 nouveaux, 3 disparus*.
    ///
    /// Un booléen n'aurait pas suffi : sur un lot, les deux cas coexistent.
    pub unchanged: usize,
    /// Ce que les **nœuds** ont signalé pendant l'exécution : un lot sauté, un
    /// service absent, une indexation impossible.
    ///
    /// Ils le disaient déjà par `ctx.warn`, ce qui devenait un
    /// `DataflowEvent::NodeLog` qu'aucun appelant du drain n'écoutait. Un drain
    /// pouvait donc rendre un compte plein et un silence complet — c'est la
    /// même correction que pour la recherche, où un avertissement moteur ne
    /// remontait nulle part.
    ///
    /// Le canal des événements a une capacité de 128 et **écarte le plus
    /// ancien** quand il déborde : sur un très gros drain, des lignes peuvent
    /// manquer. Quand ça arrive, c'est dit ici aussi.
    pub warnings: Vec<String>,
    /// **Ce que ce verbe a rendu prêt**, nommément.
    ///
    /// `None` : ce verbe ne le dit pas — file vide, échec, ou chemin qui n'a
    /// pas encore appris à répondre. Dire « je ne sais pas » est un état ; le
    /// deviner n'en est pas un.
    ///
    /// C'est la moitié écriture des quatre disponibilités : un acquittement qui
    /// annonce sa portée cesse de laisser croire qu'il a tout fait. Un verbe qui
    /// rend `data + textsearch` a laissé une dette d'embarquement dans la base,
    /// et `Catalog::embarquer_le_retard` la soldera.
    pub rendu_pret: Option<crate::disponibilite::Disponibilites>,
    pub update_results: Vec<UpdateResult>,
    pub delete_results: Vec<DeleteResult>,
}

impl FlushResult {
    /// **Absorbe le canal d'échecs** : les opérations ratées quittent
    /// `processed` pour `failed`, les disponibilités perdues quittent
    /// `rendu_pret`, et chaque échec se dit dans `warnings`.
    pub fn absorber_les_echecs(&mut self, echecs: &[EchecDeGroupe]) {
        for e in echecs {
            self.failed += e.operations;
            self.processed = self.processed.saturating_sub(e.operations);
            if e.operations == 0 {
                if let Some(pret) = self.rendu_pret.as_mut() {
                    *pret = pret.sans(e.perdu);
                }
            }
            self.warnings.push(if e.operations == 0 {
                format!(
                    "{} : « {} » — {} ({}) : {}",
                    e.noeud, e.table, "disponibilité perdue", e.perdu, e.cause
                )
            } else {
                format!(
                    "{} : « {} » — {} opération(s) en échec : {}",
                    e.noeud, e.table, e.operations, e.cause
                )
            });
        }
    }
}

// ─── DrainStats ─────────────────────────────────────────────────────────────

/// Snapshot of drain statistics.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DrainStats {
    pub pending: usize,
    pub failed: usize,
    pub total_queued: usize,
    pub total_processed: usize,
    pub total_failed: usize,
    pub flush_count: usize,
}

// ─── EntityRecord ────────────────────────────────────────────────────────────

/// An entity ready to be inserted (replaces InsertOp).
///
/// Carries the entity name, data columns, and a ref/resolver pair for
/// cross-node resolution (InsertNode resolves the ref, LinkNode awaits it).
pub struct EntityRecord {
    pub entity_name: String,
    pub data: BTreeMap<String, CypherValue>,
    pub entity_ref: EntityRef,
    pub resolver: Option<EntityRefResolver>,
}

impl EntityRecord {
    pub fn new(
        entity_name: String,
        data: BTreeMap<String, CypherValue>,
        resolver: EntityRefResolver,
        entity_ref: EntityRef,
    ) -> Self {
        Self {
            entity_name,
            data,
            entity_ref,
            resolver: Some(resolver),
        }
    }

    /// **Un enregistrement dont l'identité est déjà connue**, donc sans
    /// résolveur à consommer.
    ///
    /// La ligne existe déjà en base : c'est le cas d'une passe de rattrapage,
    /// qui relit des chunks pour leur donner l'embarquement qu'ils doivent. Il
    /// n'y a rien à résoudre, seulement à traverser des nœuds qui savent
    /// attendre un `EntityRef` — et un ref déjà prêt les traverse sans attendre.
    pub fn deja_resolu(
        entity_name: String,
        data: BTreeMap<String, CypherValue>,
        uuid: &str,
    ) -> Self {
        Self {
            entity_name,
            data,
            entity_ref: EntityRef::pre_resolved(&String::new(), uuid, uuid),
            resolver: None,
        }
    }

    /// Take the resolver out (consumed once on success by InsertNode).
    pub fn take_resolver(&mut self) -> Option<EntityRefResolver> {
        self.resolver.take()
    }
}

// ─── RelationRecord ──────────────────────────────────────────────────────────

/// A relation ready to be created (replaces LinkOp).
///
/// Endpoints (`from`/`to`) are `RefOrUuid` — either an already-known UUID
/// or an `EntityRef` that will be resolved by InsertNode before LinkNode runs.
pub struct RelationRecord {
    pub rel_name: String,
    pub from: RefOrUuid,
    pub to: RefOrUuid,
    pub properties: BTreeMap<String, CypherValue>,
    pub relation_ref: RelationRef,
    pub resolver: Option<RelationRefResolver>,
}

impl RelationRecord {
    pub fn new(
        rel_name: String,
        from: RefOrUuid,
        to: RefOrUuid,
        properties: BTreeMap<String, CypherValue>,
        resolver: RelationRefResolver,
        relation_ref: RelationRef,
    ) -> Self {
        Self {
            rel_name,
            from,
            to,
            properties,
            relation_ref,
            resolver: Some(resolver),
        }
    }

    /// Take the resolver out (consumed once on success by LinkNode).
    pub fn take_resolver(&mut self) -> Option<RelationRefResolver> {
        self.resolver.take()
    }
}

// ─── AggregateRecord ─────────────────────────────────────────────────────────

/// A KB Index entry to rebuild (replaces AggregateOp).
///
/// Quasi-identical to AggregateOp — the "instruction" was already implicit
/// in the old AggregateOp (rebuild = query graph + re-chunk + re-embed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateRecord {
    pub index_entry_uuid: String,
    pub kb_name: String,
    pub title_entity: String,
    pub source_uuid: String,
}

// ─── KBContentRecord ────────────────────────────────────────────────────────

/// Content collected from a single source field of a contributing entity.
///
/// Used by KBGatherNode to collect content from DB, and by KBChunkNode
/// to produce chunk entities with correct _source_entity / _source_uuid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordSourceContent {
    pub entity_name: String,
    pub entity_uuid: String,
    pub field_name: String,
    pub text: String,
}

/// A KB Index entry whose content has changed and needs re-chunking.
///
/// Produced by KBGatherNode (Steps 1-4: read DB, detect changes),
/// consumed by KBUpdateNode (Steps 5-6: update index, delete old chunks)
/// and KBChunkNode (Step 7: generate chunk records).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KBContentRecord {
    /// UUID of the {KB}_Index entity
    pub index_entry_uuid: String,
    /// Knowledge base name
    pub kb_name: String,
    /// Title entity name (needed for MERGE on {KB}_Index + IN_{KB} rel)
    pub source_entity: String,
    /// Source entity UUID (the title entity's _uuid)
    pub source_uuid: String,
    /// Title text (truncated)
    pub title_text: String,
    /// Aggregated content text (for SET on index)
    pub content_text: String,
    /// New content hash (title + content)
    pub new_hash: String,
    /// Source fields with text — needed for per-source chunking + SOURCED relations
    pub sources: Vec<RecordSourceContent>,
}

// ─── Checkpoint types ────────────────────────────────────────────────────────

/// Serializable snapshot of an EntityRef or RelationRef state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRefState {
    pub type_name: String,
    pub temp_uuid: String,
    pub status: CheckpointRefStatus,
}

/// Resolution status of a ref at checkpoint time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointRefStatus {
    Pending,
    Ready { uuid: String },
    Failed { error: String },
    ReadyRel { from_uuid: String, to_uuid: String },
}

/// Serializable form of EntityRecord (no channels, no resolver).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointEntityRecord {
    pub entity_name: String,
    pub data: BTreeMap<String, CypherValue>,
    pub ref_state: CheckpointRefState,
}

/// Serializable form of RelationRecord (no channels, no resolver).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRelationRecord {
    pub rel_name: String,
    pub from_uuid: Option<String>,
    pub to_uuid: Option<String>,
    pub from_temp_uuid: Option<String>,
    pub to_temp_uuid: Option<String>,
    pub properties: BTreeMap<String, CypherValue>,
    pub ref_state: CheckpointRefState,
}

impl EntityRecord {
    /// Convert to a serializable checkpoint form.
    pub fn to_checkpoint(&self) -> CheckpointEntityRecord {
        let status = match self.entity_ref.uuid() {
            Ok(uuid) => CheckpointRefStatus::Ready { uuid },
            Err(RefError::Pending) => CheckpointRefStatus::Pending,
            Err(RefError::Failed(e)) => CheckpointRefStatus::Failed { error: e },
        };
        CheckpointEntityRecord {
            entity_name: self.entity_name.clone(),
            data: self.data.clone(),
            ref_state: CheckpointRefState {
                type_name: self.entity_ref.entity().to_string(),
                temp_uuid: self.entity_ref.cle_de_correlation().to_string(),
                status,
            },
        }
    }
}

impl CheckpointEntityRecord {
    /// Reconstruct an EntityRecord from checkpoint data.
    ///
    /// The EntityRef is pre-resolved (channel initialized to Ready).
    /// The resolver is None (already consumed in the original execution).
    pub fn into_entity_record(self) -> EntityRecord {
        let entity_ref = match &self.ref_state.status {
            CheckpointRefStatus::Ready { uuid } => {
                EntityRef::pre_resolved(
                    &self.ref_state.type_name,
                    &self.ref_state.temp_uuid,
                    uuid,
                )
            }
            _ => {
                // Pending or Failed: create a new unresolved ref with the same temp_uuid.
                // This shouldn't happen in practice (checkpoint saves after node completes),
                // but we handle it for robustness.
                EntityRef::pre_resolved(
                    &self.ref_state.type_name,
                    &self.ref_state.temp_uuid,
                    "",
                )
            }
        };
        EntityRecord {
            entity_name: self.entity_name,
            data: self.data,
            entity_ref,
            resolver: None,
        }
    }
}

impl RelationRecord {
    /// Convert to a serializable checkpoint form.
    pub fn to_checkpoint(&self) -> CheckpointRelationRecord {
        let (from_uuid, from_temp_uuid) = match &self.from {
            RefOrUuid::Uuid(s) => (Some(s.clone()), None),
            RefOrUuid::Ref(r) => (r.uuid().ok(), Some(r.cle_de_correlation().to_string())),
        };
        let (to_uuid, to_temp_uuid) = match &self.to {
            RefOrUuid::Uuid(s) => (Some(s.clone()), None),
            RefOrUuid::Ref(r) => (r.uuid().ok(), Some(r.cle_de_correlation().to_string())),
        };
        let status = match self.relation_ref.resolved() {
            Ok(r) => CheckpointRefStatus::ReadyRel {
                from_uuid: r.from_uuid,
                to_uuid: r.to_uuid,
            },
            Err(RefError::Pending) => CheckpointRefStatus::Pending,
            Err(RefError::Failed(e)) => CheckpointRefStatus::Failed { error: e },
        };
        CheckpointRelationRecord {
            rel_name: self.rel_name.clone(),
            from_uuid,
            to_uuid,
            from_temp_uuid,
            to_temp_uuid,
            properties: self.properties.clone(),
            ref_state: CheckpointRefState {
                type_name: self.relation_ref.relation().to_string(),
                temp_uuid: self.relation_ref.cle_de_correlation().to_string(),
                status,
            },
        }
    }
}

impl CheckpointRelationRecord {
    /// Reconstruct a RelationRecord from checkpoint data.
    pub fn into_relation_record(self) -> RelationRecord {
        let from = match self.from_uuid {
            Some(uuid) => RefOrUuid::Uuid(uuid),
            None => RefOrUuid::Uuid(String::new()),
        };
        let to = match self.to_uuid {
            Some(uuid) => RefOrUuid::Uuid(uuid),
            None => RefOrUuid::Uuid(String::new()),
        };
        let relation_ref = match &self.ref_state.status {
            CheckpointRefStatus::ReadyRel { from_uuid, to_uuid } => {
                RelationRef::pre_resolved(
                    &self.ref_state.type_name,
                    &self.ref_state.temp_uuid,
                    from_uuid,
                    to_uuid,
                )
            }
            _ => RelationRef::pre_resolved(
                &self.ref_state.type_name,
                &self.ref_state.temp_uuid,
                "",
                "",
            ),
        };
        RelationRecord {
            rel_name: self.rel_name,
            from,
            to,
            properties: self.properties,
            relation_ref,
            resolver: None,
        }
    }
}

// ─── UpdateRecord ───────────────────────────────────────────────────────────

/// An entity update queued for drain processing.
///
/// `update()` pushes these into `PendingWork`. At drain time,
/// `UpdateRecordNode` reads old hashes, detects content changes, applies
/// field updates, and emits re-chunk requests for changed simple entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRecord {
    pub entity_name: String,
    pub uuid: String,
    pub data: BTreeMap<String, CypherValue>,
    /// Pre-computed content hash (from `build_content_text()` at enqueue time).
    pub new_content_hash: String,
}

// ─── DeleteRecord ───────────────────────────────────────────────────────────

/// An entity deletion queued for drain processing.
///
/// `delete()` pushes these into `PendingWork`. At drain time,
/// `DeleteRecordNode` cascades chunk/index deletion, removes entities,
/// and emits `AggregateRecord`s for affected KB indexes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRecord {
    pub entity_name: String,
    pub uuid: String,
}

// ─── PendingWork ─────────────────────────────────────────────────────────────

/// Typed pending work queue (replaces `Vec<CatalogOp>`).
///
/// `create()` and `link()` push records here. `update()` and
/// `delete()` push update/delete records. `build_ingestion_graph()`
/// drains them into the dataflow graph as typed inputs.
///
/// Processing order at drain: deletes → updates → inserts → links → KB aggregation.
#[derive(Default)]
pub struct PendingWork {
    pub entities: Vec<EntityRecord>,
    pub relations: Vec<RelationRecord>,
    pub aggregates: Vec<AggregateRecord>,
    pub updates: Vec<UpdateRecord>,
    pub deletes: Vec<DeleteRecord>,
}

impl PendingWork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
            && self.relations.is_empty()
            && self.aggregates.is_empty()
            && self.updates.is_empty()
            && self.deletes.is_empty()
    }

    pub fn total_count(&self) -> usize {
        self.entities.len()
            + self.relations.len()
            + self.aggregates.len()
            + self.updates.len()
            + self.deletes.len()
    }

    /// **Extrait ce qui touche ces tables**, et laisse le reste en place.
    ///
    /// C'est la moitié mécanique de l'invariant de Lucie — *jamais deux
    /// ressources sans lien bloquées l'une par l'autre* : un drain n'emporte
    /// plus la file entière, il emporte la **fermeture** d'une cible
    /// ([`crate::Catalog::fermeture`] la calcule), et ce qui n'y est pas
    /// attend son propre lecteur.
    ///
    /// - une entité, une mise à jour, une suppression : par sa table ;
    /// - une relation : par les tables de ses **deux** bouts
    ///   (`bouts_d_une_relation` les donne depuis la config) — la fermeture
    ///   garantit qu'elles y sont ensemble ;
    /// - un agrégat : par la table d'index de sa base de connaissances.
    pub fn extraire_les_tables(
        &mut self,
        tables: &HashSet<String>,
        bouts_d_une_relation: &dyn Fn(&str) -> Option<(String, String)>,
    ) -> PendingWork {
        fn partager<T>(source: &mut Vec<T>, garder: impl Fn(&T) -> bool) -> Vec<T> {
            let (pris, restent): (Vec<T>, Vec<T>) =
                std::mem::take(source).into_iter().partition(|x| garder(x));
            *source = restent;
            pris
        }
        let dans = |t: &str| tables.contains(t);
        let relation_dedans = |r: &RelationRecord| {
            bouts_d_une_relation(&r.rel_name)
                .is_some_and(|(de, vers)| dans(&de) && dans(&vers))
        };
        PendingWork {
            entities: partager(&mut self.entities, |e| dans(&e.entity_name)),
            relations: partager(&mut self.relations, relation_dedans),
            aggregates: partager(&mut self.aggregates, |a| dans(&format!("{}_Index", a.kb_name))),
            updates: partager(&mut self.updates, |u| dans(&u.entity_name)),
            deletes: partager(&mut self.deletes, |d| dans(&d.entity_name)),
        }
    }

    /// Combien d'opérations en file touchent ces tables — la même règle que
    /// [`Self::extraire_les_tables`], sans rien prendre. C'est ce qu'une
    /// recherche annonce comme « encore en file » : le reste de **sa**
    /// fermeture, pas celui de la base entière.
    pub fn compter_les_tables(
        &self,
        tables: &HashSet<String>,
        bouts_d_une_relation: &dyn Fn(&str) -> Option<(String, String)>,
    ) -> usize {
        let dans = |t: &str| tables.contains(t);
        self.entities.iter().filter(|e| dans(&e.entity_name)).count()
            + self
                .relations
                .iter()
                .filter(|r| {
                    bouts_d_une_relation(&r.rel_name)
                        .is_some_and(|(de, vers)| dans(&de) && dans(&vers))
                })
                .count()
            + self.aggregates.iter().filter(|a| dans(&format!("{}_Index", a.kb_name))).count()
            + self.updates.iter().filter(|u| dans(&u.entity_name)).count()
            + self.deletes.iter().filter(|d| dans(&d.entity_name)).count()
    }

    /// Y a-t-il une mise à jour ou une suppression dans ces tables ? Poser la
    /// donnée ne suffit alors pas : leurs conséquences sur les chunks
    /// passent par le graphe de drain.
    pub fn a_des_mises_a_jour_dans(&self, tables: &HashSet<String>) -> bool {
        self.updates.iter().any(|u| tables.contains(&u.entity_name))
            || self.deletes.iter().any(|d| tables.contains(&d.entity_name))
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refs::EntityRef;

    /// **Les comptes cessent de mentir.** Une opération ratée quitte
    /// `processed` pour `failed` ; un échec dérivé retire sa disponibilité
    /// de `rendu_pret` sans toucher aux comptes ; les deux se disent.
    #[test]
    fn absorber_les_echecs_deplace_les_comptes_et_retire_les_disponibilites() {
        use crate::disponibilite::Disponibilites as D;
        let mut res = FlushResult { processed: 10, rendu_pret: Some(D::TOUT), ..Default::default() };
        res.absorber_les_echecs(&[
            EchecDeGroupe { noeud: "InsertRecordNode".into(), table: "Note".into(), operations: 3, perdu: D::TOUT, cause: "boum".into() },
            EchecDeGroupe { noeud: "FlushNode".into(), table: "Document".into(), operations: 0, perdu: D::PLEIN_TEXTE, cause: "commit".into() },
        ]);
        assert_eq!(res.failed, 3);
        assert_eq!(res.processed, 7);
        assert_eq!(res.rendu_pret, Some(D::DONNEE | D::SPARSE | D::DENSE), "le plein texte est perdu, le reste tient");
        assert_eq!(res.warnings.len(), 2);
        assert!(res.warnings[0].contains("Note") && res.warnings[0].contains("3 opération"));
        assert!(res.warnings[1].contains("disponibilité perdue"));
    }

    #[test]
    fn entity_record_take_resolver() {
        let (entity_ref, resolver) = EntityRef::new("Document");
        let mut rec = EntityRecord::new(
            "Document".to_string(),
            BTreeMap::new(),
            resolver,
            entity_ref.clone(),
        );

        assert!(rec.resolver.is_some());
        let r = rec.take_resolver();
        assert!(r.is_some());
        r.unwrap().resolve("uuid-1".to_string());
        assert_eq!(entity_ref.uuid().unwrap(), "uuid-1");

        // Second take returns None
        assert!(rec.take_resolver().is_none());
    }

    #[test]
    fn relation_record_take_resolver() {
        let (relation_ref, resolver) = RelationRef::new("HAS");
        let mut rec = RelationRecord::new(
            "HAS".to_string(),
            RefOrUuid::from("a"),
            RefOrUuid::from("b"),
            BTreeMap::new(),
            resolver,
            relation_ref.clone(),
        );

        let r = rec.take_resolver();
        assert!(r.is_some());
        r.unwrap().resolve("from-1".to_string(), "to-2".to_string());
        let res = relation_ref.resolved().unwrap();
        assert_eq!(res.from_uuid, "from-1");
        assert_eq!(res.to_uuid, "to-2");
    }

    #[test]
    fn pending_work_empty_and_count() {
        let pw = PendingWork::new();
        assert!(pw.is_empty());
        assert_eq!(pw.total_count(), 0);
    }

    #[test]
    fn pending_work_with_records() {
        let mut pw = PendingWork::new();

        let (entity_ref, resolver) = EntityRef::new("File");
        pw.entities.push(EntityRecord::new(
            "File".to_string(),
            BTreeMap::new(),
            resolver,
            entity_ref,
        ));

        pw.aggregates.push(AggregateRecord {
            index_entry_uuid: "idx-1".to_string(),
            kb_name: "TreeKB".to_string(),
            title_entity: "Directory".to_string(),
            source_uuid: "dir-1".to_string(),
        });

        assert!(!pw.is_empty());
        assert_eq!(pw.total_count(), 2);
    }

    #[test]
    fn update_record_basics() {
        let rec = UpdateRecord {
            entity_name: "Product".to_string(),
            uuid: "prod-1".to_string(),
            data: BTreeMap::from([
                ("description".to_string(), CypherValue::String("new desc".to_string())),
            ]),
            new_content_hash: "abc123".to_string(),
        };
        assert_eq!(rec.entity_name, "Product");
        assert_eq!(rec.uuid, "prod-1");
        assert_eq!(rec.new_content_hash, "abc123");

        // Serialization roundtrip
        let json = serde_json::to_string(&rec).unwrap();
        let rec2: UpdateRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(rec2.uuid, "prod-1");
    }

    #[test]
    fn delete_record_basics() {
        let rec = DeleteRecord {
            entity_name: "Product".to_string(),
            uuid: "prod-2".to_string(),
        };
        assert_eq!(rec.entity_name, "Product");
        assert_eq!(rec.uuid, "prod-2");

        // Serialization roundtrip
        let json = serde_json::to_string(&rec).unwrap();
        let rec2: DeleteRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(rec2.uuid, "prod-2");
    }

    #[test]
    fn pending_work_with_updates_and_deletes() {
        let mut pw = PendingWork::new();
        assert!(pw.is_empty());
        assert_eq!(pw.total_count(), 0);

        pw.updates.push(UpdateRecord {
            entity_name: "Product".to_string(),
            uuid: "p1".to_string(),
            data: BTreeMap::new(),
            new_content_hash: "h1".to_string(),
        });
        assert!(!pw.is_empty());
        assert_eq!(pw.total_count(), 1);

        pw.deletes.push(DeleteRecord {
            entity_name: "Product".to_string(),
            uuid: "p2".to_string(),
        });
        assert_eq!(pw.total_count(), 2);

        // Mixed with entities
        let (entity_ref, resolver) = EntityRef::new("Product");
        pw.entities.push(EntityRecord::new(
            "Product".to_string(),
            BTreeMap::new(),
            resolver,
            entity_ref,
        ));
        assert_eq!(pw.total_count(), 3);
    }
}
