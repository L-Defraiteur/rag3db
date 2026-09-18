//! Record-based ingestion nodes (Phase B — doc 23).
//!
//! These replace the op-based batch nodes with typed record inputs.
//! The graph topology encodes the execution plan — records carry only data.
//! All nodes are **idempotent** — safe to re-run after crash or partial execution.
//!
//! - [`InsertRecordNode`] — UNWIND MERGE on `_uuid` from `Vec<EntityRecord>`
//! - [`LinkRecordNode`] — UNWIND MATCH+MERGE from `Vec<RelationRecord>`
//! - [`ChunkRecordNode`] — parallel chunking for entities (entity_configs)
//! - [`MarquerDecoupeNode`] — marks `_chunked_hash` once the chunks are linked
//! - [`EmbedNode`] — embedding with `_embed_hash` skip (configurable columns)
//! - [`FlushNode`] / [`SparseCommitNode`] — commit the FTS and sparse indexes
//! - [`DeleteRecordNode`] — batch cascade-delete entities + chunks from Vec<DeleteRecord>
//! - [`UpdateRecordNode`] — batch field update + change detection from Vec<UpdateRecord>
//! - [`RechunkDeleteNode`] — delete old chunks before re-chunking (pass-through)

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};


use crate::events::{CatalogEvent, EventBus};
use crate::chunker::{Chunker, ChunkerConfig};
use crate::config::CatalogConfig;
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::embedder::{lot_budget, souffler, stable_batches};
use crate::embedder::{DualEmbedder, Embedder, SparseEmbedder};
use crate::hash::content_hash;
use crate::node_id_cache::{InternalNodeId, NodeIdCache};
use crate::disponibilite::Disponibilites;
use crate::records::{
    DeleteRecord, DeleteResult, EchecDeGroupe, EntityRecord, RefOrUuid, RelationRecord,
    UpdateRecord, UpdateResult, UpdateStatus, SERVICE_ECHECS,
};
use crate::refs::{EntityRef, RelationRef};
use crate::search;
use crate::sparse_index::SparseVector;
use crate::uuid::chunk_uuid;

use std::any::Any;

use super::node::{Node, NodeContext};
use super::port::{BatchPayload, PortDef, PortType, PortValue};

// ─── InsertRecordNode ───────────────────────────────────────────────────────

/// Batch INSERT from `Vec<EntityRecord>`: UNWIND MERGE on `_uuid` grouped by
/// `(entity_name, column_set)`, resolves EntityRefs, caches node IDs.
/// Idempotent: re-running with the same `_uuid` updates instead of duplicating.
///
/// **Input**: `entities` — `BatchPayload<EntityRecord>` (PortType::Entities)
/// **Output**: `done` — Empty signal, `inserted` — entities with resolved refs
/// **Services**: `conn` (DbConnection), `node_id_cache` (RwLock<NodeIdCache>)
pub struct InsertRecordNode {
    name: String,
    mode: InsertMode,
    undo_data: Option<serde_json::Value>,
    // Stored during execute() for undo()
    conn: Option<Arc<dyn DbConnection>>,
    dialect: Option<Arc<dyn crate::dialect::SchemaDialect>>,
}

/// **Le chemin d'écriture d'`InsertRecordNode`.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InsertMode {
    /// `UNWIND … MERGE` : idempotent, rend les identifiants. Le défaut.
    #[default]
    Upsert,
    /// `COPY … FROM` : le chargement en masse du moteur, réservé à une
    /// première ingestion — la table est vide, aucune clé ne peut heurter.
    /// Mesuré le 6 septembre 2026 sur le cœur C++ de rag3db : 18 140 scopes
    /// par MERGE en 2,5 s, 20 132 chunks avec leur vecteur posés après en
    /// 2 + 7,7 s. Un moteur sans chemin de masse retombe sur `Upsert`.
    Copy,
}

impl InsertRecordNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), mode: InsertMode::Upsert, undo_data: None, conn: None, dialect: None }
    }

    pub fn with_mode(mut self, mode: InsertMode) -> Self {
        self.mode = mode;
        self
    }
}

/// La table `uuid → identifiant interne` rendue par un `batch_upsert`
/// (`RETURN ID(n), item._uuid`) ou par [`SchemaDialect::select_node_ids`]
/// (`RETURN n._uuid, ID(n)`) — l'ordre des colonnes diffère, on lit par type.
fn identifiants_par_uuid(rows: &[Vec<CypherValue>], uuid_en_premier: bool) -> HashMap<String, String> {
    let mut table = HashMap::with_capacity(rows.len());
    for row in rows {
        let (Some(a), Some(b)) = (row.first(), row.get(1)) else { continue };
        // **L'identifiant n'a pas le même type selon le backend.**
        // `node_id_expr` rend `ID(n)` sur rag3db — une chaîne
        // `"table:offset"` — et `_row_id` sur PostgreSQL, un entier.
        // Ne lire que la chaîne laissait cette table **vide** sur
        // tout backend SQL, et avec elle le cache d'identifiants et
        // l'indexation lucivy : un index se créait, se commitait, et
        // ne contenait aucun document. `MoteurTexte::Lucivy` était
        // donc inutilisable sur PostgreSQL, sans une erreur nulle part.
        let lire = |v: &CypherValue| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string()));
        let (uuid, id) = if uuid_en_premier { (a, b) } else { (b, a) };
        if let (Some(uuid), Some(id)) = (uuid.as_str(), lire(id)) {
            table.insert(uuid.to_string(), id);
        }
    }
    table
}


/// **Les tables qu'on écrit sans l'index plein texte qu'elles réclament.**
///
/// `Catalog::open_fts_handles_for` porte depuis longtemps le commentaire qui
/// décrit ce défaut sans pouvoir l'empêcher : sans handle ouvert, l'indexation
/// est sautée, la recherche rend zéro, et rien ne le signale. Un commentaire
/// n'est pas un contrat — voici la règle, et elle est appelée par le nœud
/// qui écrit les entités (`InsertRecordNode`).
///
/// Elle est **exacte**, donc sans fausse alerte :
///
/// - sur le chemin **natif** il n'y a aucun handle à ouvrir — l'index vit avec
///   les données — donc jamais d'alarme ;
/// - `reclamees` ne contient que ce qui déclare le signal BM25 ; une table de
///   chunks n'y est pas (son index vit sur la table parente), une entité sans
///   plein texte non plus.
///
/// Le résultat est trié et dédoublonné : une alarme par table, pas par ligne.
pub(crate) fn tables_sans_index_plein_texte<T>(
    natif: bool,
    reclamees: impl IntoIterator<Item = String>,
    ouvertes: &HashMap<String, T>,
) -> Vec<String> {
    if natif {
        return Vec::new();
    }
    let mut manquantes: Vec<String> = reclamees
        .into_iter()
        .filter(|t| !ouvertes.contains_key(t))
        .collect();
    manquantes.sort();
    manquantes.dedup();
    manquantes
}

/// **Consigne un groupe qui n'a pas abouti**, sans faire tomber le graphe.
///
/// Le canal est le service [`SERVICE_ECHECS`], ouvert par l'appelant du
/// graphe et relu par lui. Sans le service — un montage de test, un graphe
/// monté à la main — l'échec va au journal du nœud, pour ne jamais se taire.
pub(crate) fn consigner_l_echec(
    ctx: &mut NodeContext,
    noeud: &str,
    table: &str,
    operations: usize,
    perdu: Disponibilites,
    cause: String,
) {
    match ctx.service::<Arc<Mutex<Vec<EchecDeGroupe>>>>(SERVICE_ECHECS).cloned() {
        Some(canal) => {
            if let Ok(mut v) = canal.lock() {
                v.push(EchecDeGroupe {
                    noeud: noeud.to_string(),
                    table: table.to_string(),
                    operations,
                    perdu,
                    cause,
                });
                return;
            }
            ctx.warn(&format!("{noeud} : « {table} » — {operations} opération(s) en échec : {cause}"));
        }
        None => ctx.warn(&format!(
            "{noeud} : « {table} » — {operations} opération(s) en échec : {cause} \
             (aucun canal d'échecs monté : ceci n'est pas compté)"
        )),
    }
}

impl Node for InsertRecordNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "InsertRecordNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::InsertRecordNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::InsertRecordNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut items: Vec<EntityRecord> = ctx.take_input("entities")
            .and_then(|pv| pv.take::<BatchPayload>())
            .and_then(|bp| bp.take::<EntityRecord>())
            .ok_or("InsertRecordNode: missing 'entities' input")?;

        // Multi-tenant (doc 37) : toute ligne écrite ici — entité, chunk, ligne
        // d'index — porte la cellule courante. Un seul point de choc.
        let scope = ctx.service::<crate::scope::Scope>("scope").cloned().unwrap_or_default();
        for rec in &mut items {
            scope.stamp(&mut rec.data);
        }

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn").cloned()
            .ok_or("InsertRecordNode: 'conn' service not registered")?;
        let node_id_cache = ctx.service::<Arc<RwLock<NodeIdCache>>>("node_id_cache").cloned()
            .ok_or("InsertRecordNode: 'node_id_cache' service not registered")?;
        // Index FTS : présent seulement si des handles ont été ouverts. Absent
        // pour les tables de chunks (l'index vit sur la table parente).
        let fts_handles = ctx
            .service::<HashMap<String, Arc<lucivy_core::sharded_handle::ShardedHandle>>>("fts_handles")
            .cloned();

        // **Le chemin de masse dédoublonne d'abord.** `COPY` refuse tout le
        // fichier sur une clé en double là où `MERGE` mettait à jour : la
        // dernière occurrence d'un uuid gagne, les autres sont résolues sans
        // être écrites.
        let mut ecartes: HashSet<usize> = HashSet::new();
        if self.mode == InsertMode::Copy {
            let mut vus: HashSet<String> = HashSet::with_capacity(items.len());
            for i in (0..items.len()).rev() {
                let uuid = items[i].data.get("_uuid").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if !vus.insert(uuid.clone()) {
                    ecartes.insert(i);
                    if let Some(r) = items[i].take_resolver() {
                        r.resolve(uuid);
                    }
                }
            }
        }

        // Group by (entity_name, sorted column_set) for UNWIND batching. En
        // masse, la colonne du vecteur dense fait partie de la clé du groupe :
        // elle est une colonne du CSV comme les autres.
        let mut groups: HashMap<(String, Vec<String>), Vec<usize>> = HashMap::new();
        for (i, rec) in items.iter().enumerate() {
            if ecartes.contains(&i) {
                continue;
            }
            let mut columns: Vec<String> = rec.data.keys().cloned().collect();
            if let Some(dense) = rec.vectors.as_ref().and_then(|v| v.dense.as_ref()) {
                if !rec.data.contains_key(&dense.column) {
                    columns.push(dense.column.clone());
                }
            }
            columns.sort();
            groups
                .entry((rec.entity_name.clone(), columns))
                .or_default()
                .push(i);
        }

        // **L'indexation sautée en silence.** `Catalog::open_fts_handles_for`
        // porte depuis longtemps le commentaire qui décrit ce défaut sans
        // pouvoir l'empêcher : sans handle ouvert, l'indexation est sautée, la
        // recherche rend zéro, et rien ne le signale. Un commentaire n'est pas
        // un contrat — voici l'alarme. Elle atteint désormais l'appelant :
        // `drain` et `ingest_entities` ramassent les avertissements des nœuds
        // dans leur `FlushResult`.
        //
        // La règle est exacte, donc sans fausse alerte : on ne se plaint que si
        // le moteur n'est **pas** natif (sur le chemin natif il n'y a aucun
        // handle à ouvrir, l'index vit avec les données) **et** que l'entité
        // déclare le signal BM25. Une table de chunks n'est pas dans
        // `entity_configs`, elle ne déclenche donc rien — c'est voulu, son index
        // vit sur la table parente.
        let natif = ctx.service::<bool>("plein_texte_natif").copied().unwrap_or(false);
        {
            let configs = ctx
                .service::<HashMap<String, crate::config::EntityConfig>>("entity_configs")
                .cloned();
            if let (Some(handles), Some(configs)) = (fts_handles.as_ref(), configs) {
                let reclamees = groups
                    .keys()
                    .map(|(nom, _)| nom.clone())
                    .filter(|nom| configs.get(nom).is_some_and(|c| c.signals.bm25()));
                for nom in tables_sans_index_plein_texte(natif, reclamees, handles) {
                    ctx.warn(&format!(
                        "aucun index plein texte ouvert pour « {nom} », qui déclare \
                         pourtant le signal BM25 : ses champs texte ne sont pas indexés \
                         et une recherche rendra zéro sans autre explication. Il manque \
                         un open_fts_handles_for() sur ce point d'entrée d'ingestion."
                    ));
                }
            }
        }

        ctx.metric("items", items.len() as f64);
        ctx.metric("groups", groups.len() as f64);
        ctx.info(&format!("group_summary: {}", groups.iter()
            .map(|((name, _), idxs)| format!("{}×{}", name, idxs.len()))
            .collect::<Vec<_>>().join(", ")));

        let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect").cloned()
            .ok_or("InsertRecordNode: 'dialect' service not registered")?;
        let sparse_handles = ctx
            .service::<HashMap<String, Arc<sparse_vector::handle::SparseHandle>>>("sparse_handles")
            .cloned();
        let mut copied = 0usize;

        for ((entity_name, columns), indices) in &groups {
            let col_refs: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();
            // Ce que la relecture des identifiants doit servir : le cache et
            // lucivy (par table), le handle sparse (par enregistrement).
            let a_indexer = fts_handles.as_ref().is_some_and(|h| h.contains_key(entity_name.as_str()));
            let porte_du_sparse = indices.iter().any(|&i| items[i].vectors.as_ref().is_some_and(|v| v.sparse.is_some()));

            // **En masse, par COPY**, sur une première ingestion : le moteur
            // charge le fichier d'un bloc, sans MERGE ligne à ligne. Refusé,
            // le groupe repasse par le chemin de toujours.
            let mut uuid_to_node_id: Option<HashMap<String, String>> = None;
            if self.mode == InsertMode::Copy {
                match copier_les_noeuds(conn.as_ref(), dialect.as_ref(), entity_name, &col_refs, indices, &items, a_indexer || porte_du_sparse) {
                    Ok(Some(ids)) => {
                        copied += indices.len();
                        uuid_to_node_id = Some(ids);
                    }
                    Ok(None) => {}
                    Err(cause) => {
                        ctx.warn(&format!("insertion dans « {entity_name} » : chargement en masse refusé ({cause}), retour au MERGE"));
                    }
                }
            }

            let uuid_to_node_id = match uuid_to_node_id {
                Some(ids) => ids,
                None => {
                    // Build batch upsert via dialect (idempotent MERGE/INSERT ON CONFLICT)
                    let cypher = dialect.batch_upsert(entity_name, &col_refs);

                    // Build items list param. Un vecteur porté par
                    // l'enregistrement se pose avec la ligne, ici aussi.
                    let items_param = CypherValue::List(
                        indices
                            .iter()
                            .map(|&i| {
                                let rec = &items[i];
                                let mut map = BTreeMap::new();
                                for col in &col_refs {
                                    let valeur = rec.data.get(*col).cloned().or_else(|| {
                                        rec.vectors.as_ref().and_then(|v| v.dense.as_ref())
                                            .filter(|d| d.column == *col)
                                            .map(|d| CypherValue::List(d.values.iter().map(|&f| CypherValue::Float(f as f64)).collect()))
                                    });
                                    map.insert(col.to_string(), valeur.unwrap_or(CypherValue::Null));
                                }
                                CypherValue::Map(map)
                            })
                            .collect(),
                    );

                    // **Un groupe qui échoue ne tue plus le graphe.** Il se compte,
                    // ses refs sont résolus en échec — un lien vers une de ces lignes
                    // échouera tout de suite et se comptera, au lieu d'attendre
                    // trente secondes — et les autres groupes passent.
                    let result = match conn.execute_with_params(
                        &cypher,
                        &[QueryParam { name: "items".to_string(), value: items_param }],
                    ) {
                        Ok(r) => r,
                        Err(e) => {
                            let cause = e.to_string();
                            for &i in indices {
                                if let Some(r) = items[i].take_resolver() {
                                    r.fail(format!("insertion dans « {entity_name} » échouée : {cause}"));
                                }
                            }
                            consigner_l_echec(
                                ctx, "InsertRecordNode", entity_name, indices.len(), Disponibilites::TOUT, cause,
                            );
                            continue;
                        }
                    };
                    identifiants_par_uuid(&result.rows, false)
                }
            };

            // Resolve refs + cache node IDs
            for &i in indices {
                let rec = &mut items[i];
                let uuid = rec
                    .data
                    .get("_uuid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if let Some(id_str) = uuid_to_node_id.get(&uuid) {
                    // **Un identifiant illisible n'est pas une absence.** Tout
                    // ce qui suit en dépend — le cache d'identifiants et
                    // l'indexation lucivy — et le sauter sans le dire donne un
                    // index vide qui se commite sans erreur.
                    if InternalNodeId::parse(id_str).is_none() {
                        ctx.warn(&format!(
                            "InsertRecordNode: identifiant « {id_str} » illisible pour \
                             {entity_name} — ni cache d'identifiants ni index FTS pour \
                             cette ligne"
                        ));
                    }
                    if let Some(node_id) = InternalNodeId::parse(id_str) {
                        if let Ok(mut cache) = node_id_cache.write() {
                            cache.insert(&uuid, node_id);
                        }

                        // Indexation FTS par offset, à l'identique du sparse.
                        //
                        // On passe TOUTES les valeurs texte du record :
                        // `index_document` ne retient que les champs présents au
                        // schéma, qui est donc l'unique source de vérité sur ce
                        // qui est indexé. Et c'est exactement la valeur écrite en
                        // base, donc celle que le chunker verra — condition
                        // nécessaire pour que les offsets de highlight s'alignent
                        // sur les spans de chunk.
                        if let Some(ref handles) = fts_handles {
                            if let Some(handle) = handles.get(entity_name) {
                                let text_fields: Vec<(String, String)> = rec
                                    .data
                                    .iter()
                                    .filter_map(|(k, v)| {
                                        v.as_str().map(|s| (k.clone(), s.to_string()))
                                    })
                                    .collect();
                                // Une ligne posée mais non indexée : la donnée
                                // est là, le plein texte non. Ça se dit comme une
                                // disponibilité perdue, pas comme une opération.
                                if let Err(e) = crate::fts_handle::upsert_document(
                                    handle,
                                    &text_fields,
                                    node_id.offset,
                                ) {
                                    consigner_l_echec(
                                        ctx, "InsertRecordNode", entity_name, 0,
                                        Disponibilites::PLEIN_TEXTE,
                                        format!("indexation FTS de la ligne {uuid} : {e}"),
                                    );
                                }
                            }
                        }

                        // Le vecteur sparse porté par l'enregistrement, dans le
                        // handle, au décalage de la ligne — ce qu'`EmbedNode`
                        // fait après coup quand la ligne le précède.
                        if let Some(sv) = rec.vectors.as_mut().and_then(|v| v.sparse.take()) {
                            if let Some(handle) = sparse_handles.as_ref().and_then(|h| h.get(entity_name.as_str())) {
                                let sv_idx = sparse_vector::index::SparseVector::new(sv.indices, sv.values);
                                if let Err(e) = handle.insert(node_id.offset, &sv_idx) {
                                    consigner_l_echec(
                                        ctx, "InsertRecordNode", entity_name, 0,
                                        Disponibilites::SPARSE,
                                        format!("vecteur sparse de la ligne {uuid} : {e}"),
                                    );
                                }
                            }
                        }
                    }
                }

                if let Some(resolver) = rec.take_resolver() {
                    resolver.resolve(uuid);
                }
            }
        }
        if copied > 0 {
            ctx.metric("copied", copied as f64);
        }

        // Capture undo data: entity_name → [uuid] for DELETE
        let mut undo_groups: HashMap<String, Vec<String>> = HashMap::new();
        for rec in &items {
            if let Some(uuid) = rec.data.get("_uuid").and_then(|v| v.as_str()) {
                undo_groups.entry(rec.entity_name.clone()).or_default().push(uuid.to_string());
            }
        }
        self.undo_data = Some(serde_json::json!(undo_groups));

        // Store services for undo
        self.conn = Some(conn.clone());
        let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect").cloned()
            .ok_or("InsertRecordNode: 'dialect' service not registered")?;
        self.dialect = Some(dialect.clone());

        ctx.trigger("done");
        ctx.set_output("inserted", PortValue::new(
            BatchPayload::new(PortType::Entities, items),
        ));
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<Box<dyn Any + Send>> {
        self.undo_data.clone().map(|v| Box::new(v) as Box<dyn Any + Send>)
    }

    fn undo(&mut self, undo_ctx: Box<dyn Any + Send>) -> Result<(), String> {
        let undo_ctx = *undo_ctx.downcast::<serde_json::Value>().map_err(|_| "bad undo ctx")?;
        let conn = self.conn.as_ref()
            .ok_or("InsertRecordNode undo: 'conn' not stored")?;
        let dialect = self.dialect.as_ref()
            .ok_or("InsertRecordNode undo: 'dialect' not stored")?;

        let groups = undo_ctx.as_object()
            .ok_or("InsertRecordNode undo: expected object")?;

        for (entity_name, uuids) in groups {
            let uuid_list: Vec<&str> = uuids.as_array()
                .ok_or("InsertRecordNode undo: expected array of uuids")?
                .iter()
                .filter_map(|v| v.as_str())
                .collect();

            if uuid_list.is_empty() { continue; }

            let uuid_params = CypherValue::List(
                uuid_list.iter().map(|u| CypherValue::String(u.to_string())).collect()
            );
            let cypher = dialect.batch_cascade_delete(entity_name);
            conn.execute_with_params(
                &cypher,
                &[QueryParam { name: "uuids".into(), value: uuid_params }],
            ).map_err(|e| format!("InsertRecordNode undo failed: {e}"))?;
        }
        Ok(())
    }
}

// ─── LinkRecordNode ─────────────────────────────────────────────────────────

/// Batch LINK from `Vec<RelationRecord>`: resolves from/to refs,
/// UNWIND MATCH+MERGE grouped by `(rel_name, property_keys)`.
/// Idempotent: re-running with the same from/to/rel_name skips existing relations.
///
/// **Input**: `relations` — `BatchPayload<RelationRecord>` (PortType::Relations)
/// **Output**: `done` — Empty signal
/// **Services**: `conn` (DbConnection)
pub struct LinkRecordNode {
    name: String,
    undo_data: Option<serde_json::Value>,
    conn: Option<Arc<dyn DbConnection>>,
    dialect: Option<Arc<dyn crate::dialect::SchemaDialect>>,
}

impl LinkRecordNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), undo_data: None, conn: None, dialect: None }
    }
}


/// Un lien dont les deux bouts sont résolus, et son rang dans le lot.
struct ResolvedLink {
    from_uuid: String,
    to_uuid: String,
    index: usize,
}

/// Au-delà de ce nombre d'arêtes dans une relation, le lot part par COPY.
const COPY_SEUIL: usize = 2_000;

/// **Un fichier CSV à nous seuls.** Le processus, un compteur, et l'instant :
/// deux graphes dans le même processus — deux tests, deux fils — qui posent
/// la même table à la même milliseconde se volaient le fichier (6 septembre
/// 2026 : un lot de trois lignes en laissait une).
fn fichier_csv(prefixe: &str, table: &str) -> std::path::PathBuf {
    static COMPTEUR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COMPTEUR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "rag3weaver-{prefixe}-{}-{n}-{table}-{}.csv",
        std::process::id(),
        crate::dataflow::checkpoint::timestamp_ms()
    ))
}

/// Une valeur Cypher dans une cellule CSV, entre guillemets doublés au besoin.
fn cellule_csv(v: &CypherValue) -> String {
    let brut = match v {
        CypherValue::Null => return String::new(),
        // La cellule vide est une chaîne vide, pas un NULL — `_embed_hash` à
        // la naissance d'un chunk en est une, et la lire NULL la ferait
        // réécrire. C'est le `null_strings` du COPY des nœuds qui le garantit.
        CypherValue::String(s) if s.is_empty() => return "\"\"".to_string(),
        CypherValue::String(s) => s.clone(),
        CypherValue::Int(i) => i.to_string(),
        CypherValue::Float(f) => f.to_string(),
        CypherValue::Bool(b) => b.to_string(),
        CypherValue::List(items) => {
            let cases: Vec<String> = items.iter().map(|x| match x {
                CypherValue::Int(i) => i.to_string(),
                CypherValue::Float(f) => f.to_string(),
                CypherValue::Bool(b) => b.to_string(),
                CypherValue::Null => String::new(),
                autre => format!("{autre:?}"),
            }).collect();
            format!("[{}]", cases.join(","))
        }
        autre => format!("{autre:?}"),
    };
    if brut.contains([',', '"', '\n', '\r']) {
        // Entre guillemets : la barre d'abord (sinon celles produites par
        // les deux suivantes seraient redoublées), puis les sauts de ligne
        // en deux caractères — c'est l'option `ESCAPED_NEWLINES` du moteur
        // (7 septembre 2026), qui garde le lecteur CSV **parallèle** là où
        // un saut de ligne physique entre guillemets le refusait.
        let echappee = brut.replace('\\', "\\\\").replace('\n', "\\n").replace('\r', "\\r").replace('"', "\"\"");
        format!("\"{echappee}\"")
    } else {
        brut
    }
}

/// Écrire une chaîne en cellule CSV sans allouer quand elle n'a rien à
/// échapper — le cas des uuids, deux par arête, 205 000 arêtes.
fn ecrire_cellule_texte(w: &mut impl std::io::Write, texte: &str) -> Result<(), String> {
    if texte.contains([',', '"', '\n', '\r']) || texte.is_empty() {
        w.write_all(cellule_csv(&CypherValue::String(texte.to_string())).as_bytes()).map_err(|e| e.to_string())
    } else {
        w.write_all(texte.as_bytes()).map_err(|e| e.to_string())
    }
}

/// Une valeur qu'on sait écrire en CSV pour le moteur : les scalaires, et
/// les listes de nombres. Une liste de chaînes ou une carte a une syntaxe
/// qu'on n'a pas éprouvée, et un NULL se lit différemment selon le type de
/// la colonne, que l'écrivain ne connaît pas — leur groupe reste sur le MERGE.
fn csv_sait_ecrire(v: &CypherValue) -> bool {
    match v {
        CypherValue::String(_) | CypherValue::Int(_) | CypherValue::Float(_) | CypherValue::Bool(_) => true,
        CypherValue::List(items) => items.iter().all(|x| matches!(x, CypherValue::Int(_) | CypherValue::Float(_) | CypherValue::Bool(_) | CypherValue::Null)),
        _ => false,
    }
}

/// **Poser un groupe de lignes par COPY.** `Ok(None)` : pas de chemin de
/// masse ici (moteur sans COPY, ou une valeur qu'on ne sait pas écrire en
/// CSV) — l'appelant reste sur le MERGE. `Ok(Some(ids))` : posé ; la table
/// `uuid → identifiant` est relue seulement si `relire_les_identifiants`,
/// c'est-à-dire quand lucivy ou le handle sparse en ont besoin. Vide sinon.
fn copier_les_noeuds(
    conn: &dyn crate::connection::DbConnection,
    dialect: &dyn crate::dialect::SchemaDialect,
    table: &str,
    columns: &[&str],
    indices: &[usize],
    items: &[EntityRecord],
    relire_les_identifiants: bool,
) -> Result<Option<HashMap<String, String>>, String> {
    let chemin = fichier_csv("lignes", table);
    let Some(copie) = dialect.copy_nodes_from_csv(table, columns, &chemin.to_string_lossy()) else {
        return Ok(None);
    };
    if indices.iter().any(|&i| items[i].data.values().any(|v| !csv_sait_ecrire(v))) {
        return Ok(None);
    }
    let profil = std::env::var_os("RAG3WEAVER_INGEST_PROFILE").is_some();
    let t0 = std::time::Instant::now();

    {
        use std::io::Write;
        let fichier = std::fs::File::create(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        let mut w = std::io::BufWriter::with_capacity(1 << 20, fichier);
        let mut ligne = String::new();
        for &i in indices {
            let rec = &items[i];
            ligne.clear();
            for (k, col) in columns.iter().enumerate() {
                if k > 0 {
                    ligne.push(',');
                }
                match rec.data.get(*col) {
                    Some(v) => ligne.push_str(&cellule_csv(v)),
                    None => {
                        // La colonne du vecteur dense : `[f1,f2,…]` entre
                        // guillemets, écrit depuis le `f32` sans passer par
                        // une liste de `CypherValue`.
                        if let Some(d) = rec.vectors.as_ref().and_then(|v| v.dense.as_ref()).filter(|d| d.column == *col) {
                            ligne.push_str("\"[");
                            for (j, f) in d.values.iter().enumerate() {
                                if j > 0 {
                                    ligne.push(',');
                                }
                                use std::fmt::Write as _;
                                let _ = write!(ligne, "{f}");
                            }
                            ligne.push_str("]\"");
                        }
                    }
                }
            }
            ligne.push('\n');
            w.write_all(ligne.as_bytes()).map_err(|e| e.to_string())?;
        }
        w.flush().map_err(|e| e.to_string())?;
    }

    let t_csv = t0.elapsed().as_millis();
    let t1 = std::time::Instant::now();
    let resultat = conn.execute(&copie).map_err(|e| e.to_string());
    let _ = std::fs::remove_file(&chemin);
    let t_copy = t1.elapsed().as_millis();
    let t2 = std::time::Instant::now();
    if profil {
        eprintln!(
            "[copy-profile] {table} : {} lignes, csv {t_csv} ms, COPY {t_copy} ms{}",
            indices.len(),
            resultat.as_ref().err().map(|e| format!(" — refusé : {e}")).unwrap_or_default()
        );
    }
    resultat?;

    let mut ids = HashMap::new();
    if relire_les_identifiants {
        let uuids: Vec<&str> = indices.iter().filter_map(|&i| items[i].data.get("_uuid").and_then(|v| v.as_str())).collect();
        for tranche in uuids.chunks(5_000) {
            let param = CypherValue::List(tranche.iter().map(|u| CypherValue::String(u.to_string())).collect());
            let lu = conn
                .execute_with_params(&dialect.select_node_ids(table), &[QueryParam { name: "uuids".into(), value: param }])
                .map_err(|e| e.to_string())?;
            ids.extend(identifiants_par_uuid(&lu.rows, true));
        }
        if profil {
            eprintln!("[copy-profile] {table} : identifiants relus en {} ms", t2.elapsed().as_millis());
        }
    }
    Ok(Some(ids))
}

/// **Poser un lot d'arêtes par COPY.** Rend `Ok(true)` si le lot est posé,
/// `Ok(false)` si le moteur n'a pas de chemin de masse.
#[allow(clippy::too_many_arguments)]
fn copier_les_liens(
    ctx: &mut NodeContext,
    conn: &dyn crate::connection::DbConnection,
    dialect: &dyn crate::dialect::SchemaDialect,
    rel_name: &str,
    ends: (&str, &str),
    prop_keys: &[String],
    prop_refs: &[&str],
    indices: &[usize],
    resolved: &[ResolvedLink],
    items: &[RelationRecord],
    presents_connus: &mut HashMap<String, HashSet<String>>,
) -> Result<bool, String> {
    let chemin = fichier_csv("liens", rel_name);
    let Some(copie) = dialect.copy_links_from_csv(rel_name, ends, prop_refs, &chemin.to_string_lossy()) else {
        return Ok(false);
    };
    let profil = std::env::var_os("RAG3WEAVER_INGEST_PROFILE").is_some();
    let t0 = std::time::Instant::now();
    // Les paires déjà posées, seulement si la table a quelque chose.
    let mut deja: HashSet<(String, String)> = HashSet::new();
    let compte = conn.execute(&dialect.count_links(rel_name)).map_err(|e| e.to_string())?;
    let vide = matches!(compte.rows.first().and_then(|r| r.first()), Some(CypherValue::Int(0)));
    if !vide {
        let mut froms: Vec<String> = indices.iter().map(|&ri| resolved[ri].from_uuid.clone()).collect();
        froms.sort_unstable();
        froms.dedup();
        for tranche in froms.chunks(5_000) {
            let param = CypherValue::List(tranche.iter().map(|f| CypherValue::String(f.clone())).collect());
            let lu = conn
                .execute_with_params(&dialect.existing_links(rel_name, ends), &[QueryParam { name: "froms".into(), value: param }])
                .map_err(|e| e.to_string())?;
            for row in &lu.rows {
                if let (Some(a), Some(b)) = (row.first().and_then(|v| v.as_str()), row.get(1).and_then(|v| v.as_str())) {
                    deja.insert((a.to_string(), b.to_string()));
                }
            }
        }
    }
    // **Les bouts qui existent.** COPY refuse tout le fichier dès qu'une
    // clé manque (« Unable to find primary key value ») là où MERGE sautait
    // la paire en silence : un PARENT_OF vers une fermeture repliée, un
    // MENTIONS vers un symbole absent. On vérifie les uuids par table, en
    // une requête par tranche, et on n'écrit que les paires dont les deux
    // bouts sont là ; les autres sont comptées, pas perdues en silence.
    // **Chaque uuid n'est vérifié qu'une fois par drain** : six relations
    // sur les mêmes scopes demandaient six fois la même liste.
    let mut verifier = |table: &str, uuids: &mut Vec<&str>| -> Result<(), String> {
        let connus = presents_connus.entry(table.to_string()).or_default();
        uuids.retain(|u| !connus.contains(*u));
        for tranche in uuids.chunks(5_000) {
            let param = CypherValue::List(tranche.iter().map(|u| CypherValue::String(u.to_string())).collect());
            let lu = conn
                .execute_with_params(&dialect.select_by_uuids(table, &["_uuid"]), &[QueryParam { name: "uuids".into(), value: param }])
                .map_err(|e| e.to_string())?;
            for row in &lu.rows {
                if let Some(u) = row.first().and_then(|v| v.as_str()) {
                    connus.insert(u.to_string());
                }
            }
        }
        Ok(())
    };
    let mut froms_tous: Vec<&str> = indices.iter().map(|&ri| resolved[ri].from_uuid.as_str()).collect();
    froms_tous.sort_unstable();
    froms_tous.dedup();
    let mut tos_tous: Vec<&str> = indices.iter().map(|&ri| resolved[ri].to_uuid.as_str()).collect();
    tos_tous.sort_unstable();
    tos_tous.dedup();
    verifier(ends.0, &mut froms_tous)?;
    verifier(ends.1, &mut tos_tous)?;
    let vide_connus = HashSet::new();
    let froms_presents = presents_connus.get(ends.0).unwrap_or(&vide_connus);
    let tos_presents = presents_connus.get(ends.1).unwrap_or(&vide_connus);
    let t_existence = t0.elapsed();
    let t_csv0 = std::time::Instant::now();
    let mut absents = 0usize;

    let mut vues: HashSet<(&str, &str)> = HashSet::with_capacity(indices.len());
    let mut ecrites = 0usize;
    {
        use std::io::Write;
        let f = std::fs::File::create(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        let mut w = std::io::BufWriter::with_capacity(1 << 20, f);
        for &ri in indices {
            let rl = &resolved[ri];
            let paire = (rl.from_uuid.as_str(), rl.to_uuid.as_str());
            if !froms_presents.contains(paire.0) || !tos_presents.contains(paire.1) {
                absents += 1;
                continue;
            }
            if !vues.insert(paire) || (!deja.is_empty() && deja.contains(&(paire.0.to_string(), paire.1.to_string()))) {
                continue;
            }
            let rel = &items[rl.index];
            ecrire_cellule_texte(&mut w, &rl.from_uuid)?;
            w.write_all(b",").map_err(|e| e.to_string())?;
            ecrire_cellule_texte(&mut w, &rl.to_uuid)?;
            for key in prop_keys {
                w.write_all(b",").map_err(|e| e.to_string())?;
                w.write_all(cellule_csv(rel.properties.get(key).unwrap_or(&CypherValue::Null)).as_bytes()).map_err(|e| e.to_string())?;
            }
            w.write_all(b"\n").map_err(|e| e.to_string())?;
            ecrites += 1;
        }
        w.flush().map_err(|e| e.to_string())?;
    }
    let t_csv = t_csv0.elapsed();
    let t1 = std::time::Instant::now();
    let resultat = if ecrites > 0 { conn.execute(&copie).map(|_| ()).map_err(|e| e.to_string()) } else { Ok(()) };
    let _ = std::fs::remove_file(&chemin);
    if profil {
        eprintln!(
            "[link-profile] {rel_name} : {} arêtes, {} déjà là ou en double, {absents} sans bout, existence {} ms, csv {} ms, COPY {} ms{}",
            ecrites,
            indices.len() - ecrites - absents,
            t_existence.as_millis(),
            t_csv.as_millis(),
            t1.elapsed().as_millis(),
            resultat.as_ref().err().map(|e| format!(" — refusé : {e}")).unwrap_or_default()
        );
    }
    resultat?;
    ctx.metric("copied", ecrites as f64);
    ctx.metric("already_linked", (indices.len() - ecrites - absents) as f64);
    ctx.metric("dangling", absents as f64);
    Ok(true)
}

impl Node for LinkRecordNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "LinkRecordNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::LinkRecordNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::LinkRecordNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut items: Vec<RelationRecord> = ctx.take_input("relations")
            .and_then(|pv| pv.take::<BatchPayload>())
            .and_then(|bp| bp.take::<RelationRecord>())
            .ok_or("LinkRecordNode: missing 'relations' input")?;

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn").cloned()
            .ok_or("LinkRecordNode: 'conn' service not registered")?;

        // Resolve all refs first (should be instant — InsertRecordNode already completed)
        let t_resolution = std::time::Instant::now();
        let mut resolved: Vec<ResolvedLink> = Vec::with_capacity(items.len());
        for (i, rel) in items.iter_mut().enumerate() {
            // Un bout qui ne se résout pas — l'insertion de sa ligne a échoué —
            // ne fait plus tomber tous les liens du lot : celui-ci se compte,
            // son ref est résolu en échec, les autres partent.
            let bouts = rel
                .from
                .resolve()
                .map_err(|e| format!("bout source : {e}"))
                .and_then(|de| {
                    rel.to
                        .resolve()
                        .map(|vers| (de, vers))
                        .map_err(|e| format!("bout cible : {e}"))
                });
            match bouts {
                Ok((from_uuid, to_uuid)) => resolved.push(ResolvedLink { from_uuid, to_uuid, index: i }),
                Err(cause) => {
                    let nom = rel.rel_name.clone();
                    if let Some(r) = rel.take_resolver() {
                        r.fail(format!("lien « {nom} » : {cause}"));
                    }
                    consigner_l_echec(ctx, "LinkRecordNode", &nom, 1, Disponibilites::TOUT, cause);
                }
            }
        }

        // Group by (rel_name, sorted property keys) for UNWIND batching.
        let mut groups: HashMap<(String, Vec<String>), Vec<usize>> = HashMap::new();
        for (ri, rl) in resolved.iter().enumerate() {
            let rel = &items[rl.index];
            let mut prop_keys: Vec<String> = rel.properties.keys().cloned().collect();
            prop_keys.sort();
            groups
                .entry((rel.rel_name.clone(), prop_keys))
                .or_default()
                .push(ri);
        }

        ctx.metric("items", items.len() as f64);
        ctx.metric("groups", groups.len() as f64);
        ctx.metric("resolve_ms", t_resolution.elapsed().as_millis() as f64);
        ctx.info(&format!("group_summary: {}", groups.iter()
            .map(|((name, _), idxs)| format!("{}×{}", name, idxs.len()))
            .collect::<Vec<_>>().join(", ")));
        // Les uuids dont l'existence est acquise, par table, pour tout ce drain.
        let mut presents_connus: HashMap<String, HashSet<String>> = HashMap::new();

        for ((rel_name, prop_keys), indices) in &groups {
            // Build batch link via dialect (idempotent — skip if relation already exists)
            let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
                .cloned()
                .ok_or("LinkRecordNode: 'dialect' service not registered")?;
            let prop_refs: Vec<&str> = prop_keys.iter().map(|s| s.as_str()).collect();
            // Les bouts déclarés de la relation, pour un MATCH étiqueté — et
            // ceux d'un lien de chunk, que le nom porte : `X_CHUNKED_FROM`
            // va de `X_Chunk` à `X`.
            let ends = ctx
                .service::<crate::config::CatalogConfig>("config")
                .and_then(|c| c.relations.get(rel_name.as_str()).map(|d| (d.from.clone(), d.to.clone())))
                .or_else(|| {
                    rel_name
                        .strip_suffix("_CHUNKED_FROM")
                        .map(|entity| (format!("{entity}_Chunk"), entity.to_string()))
                });
            let cypher = dialect.batch_link_labeled(rel_name, ends.as_ref().map(|(f, t)| (f.as_str(), t.as_str())), &prop_refs);

            // **En masse, par COPY**, quand le lot est gros, que la relation
            // a ses deux étiquettes et que le moteur sait le faire : 200 000
            // arêtes en 47 ms au lieu de 158 s (6 septembre 2026). La
            // sémantique de MERGE est gardée : dédoublonnage dans le lot, et
            // les paires déjà posées écartées quand la table n'est pas vide.
            if indices.len() >= COPY_SEUIL {
                if let Some((from, to)) = ends.as_ref() {
                    match copier_les_liens(ctx, conn.as_ref(), dialect.as_ref(), rel_name, (from, to), prop_keys, &prop_refs, indices, &resolved, &items, &mut presents_connus) {
                        Ok(true) => {
                            for &ri in indices {
                                let rl = &resolved[ri];
                                if let Some(resolver) = items[rl.index].take_resolver() {
                                    resolver.resolve(rl.from_uuid.clone(), rl.to_uuid.clone());
                                }
                            }
                            continue;
                        }
                        Ok(false) => {}
                        Err(cause) => {
                            ctx.warn(&format!("lien « {rel_name} » : chargement en masse refusé ({cause}), retour au chemin par lots"));
                        }
                    }
                }
            }

            // **Par tranches.** Un seul UNWIND de 225 000 éléments prenait
            // 97 s là où 30 000 en prenaient 1,2 s : superlinéaire au-delà de
            // quelques dizaines de milliers (6 septembre 2026, cœur C++ de
            // rag3db). Cinq mille par requête.
            for tranche in indices.chunks(5_000) {
            let items_param = CypherValue::List(
                tranche
                    .iter()
                    .map(|&ri| {
                        let rl = &resolved[ri];
                        let rel = &items[rl.index];
                        let mut map = BTreeMap::new();
                        map.insert(
                            "from_uuid".to_string(),
                            CypherValue::String(rl.from_uuid.clone()),
                        );
                        map.insert(
                            "to_uuid".to_string(),
                            CypherValue::String(rl.to_uuid.clone()),
                        );
                        for key in prop_keys {
                            map.insert(
                                key.clone(),
                                rel.properties.get(key).cloned().unwrap_or(CypherValue::Null),
                            );
                        }
                        CypherValue::Map(map)
                    })
                    .collect(),
            );

            if let Err(e) = conn.execute_with_params(
                &cypher,
                &[QueryParam { name: "items".to_string(), value: items_param }],
            ) {
                let cause = e.to_string();
                for &ri in tranche {
                    if let Some(r) = items[resolved[ri].index].take_resolver() {
                        r.fail(format!("lien « {rel_name} » : {cause}"));
                    }
                }
                consigner_l_echec(ctx, "LinkRecordNode", rel_name, tranche.len(), Disponibilites::TOUT, cause);
                continue;
            }

            // Resolve relation refs
            for &ri in tranche {
                let rl = &resolved[ri];
                let rel = &mut items[rl.index];
                if let Some(resolver) = rel.take_resolver() {
                    resolver.resolve(rl.from_uuid.clone(), rl.to_uuid.clone());
                }
            }
            }
        }

        // Capture undo data: rel_name → [{from, to}]
        let mut undo_groups: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
        for rl in &resolved {
            let rel = &items[rl.index];
            undo_groups.entry(rel.rel_name.clone()).or_default().push(
                serde_json::json!({"from": rl.from_uuid, "to": rl.to_uuid})
            );
        }
        self.undo_data = Some(serde_json::json!(undo_groups));

        // Store services for undo
        self.conn = Some(conn.clone());
        let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect").cloned()
            .ok_or("LinkRecordNode: 'dialect' service not registered")?;
        self.dialect = Some(dialect.clone());

        ctx.trigger("done");
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<Box<dyn Any + Send>> {
        self.undo_data.clone().map(|v| Box::new(v) as Box<dyn Any + Send>)
    }

    fn undo(&mut self, undo_ctx: Box<dyn Any + Send>) -> Result<(), String> {
        let undo_ctx = *undo_ctx.downcast::<serde_json::Value>().map_err(|_| "bad undo ctx")?;
        let conn = self.conn.as_ref()
            .ok_or("LinkRecordNode undo: 'conn' not stored")?;
        let dialect = self.dialect.as_ref()
            .ok_or("LinkRecordNode undo: 'dialect' not stored")?;

        let groups = undo_ctx.as_object()
            .ok_or("LinkRecordNode undo: expected object")?;

        for (rel_name, links) in groups {
            let link_list = links.as_array()
                .ok_or("LinkRecordNode undo: expected array of links")?;

            let items_param = CypherValue::List(
                link_list.iter().filter_map(|link| {
                    let from = link["from"].as_str()?;
                    let to = link["to"].as_str()?;
                    let mut m = BTreeMap::new();
                    m.insert("from".into(), CypherValue::String(from.to_string()));
                    m.insert("to".into(), CypherValue::String(to.to_string()));
                    Some(CypherValue::Map(m))
                }).collect()
            );

            let cypher = dialect.batch_delete_relation(rel_name);
            conn.execute_with_params(
                &cypher,
                &[QueryParam { name: "items".into(), value: items_param }],
            ).map_err(|e| format!("LinkRecordNode undo failed: {e}"))?;
        }
        Ok(())
    }
}

// ─── ChunkRecordNode (simple entities) ──────────────────────────────────────

/// Parallel chunking for simple entities (registerEntity API).
/// Uses `entity_configs` to find content fields.
///
/// **Input**: `entities` — `BatchPayload<EntityRecord>` (PortType::Entities)
/// **Output**: `done` — Empty, `chunks` — chunk entities, `chunk_links` — CHUNKED_FROM relations
/// **Services**: `config`, `entity_configs`, `chunker_cache`
pub struct ChunkRecordNode {
    name: String,
}

impl ChunkRecordNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Compute chunks for one simple entity, producing EntityRecords (chunks) and
    /// RelationRecords (CHUNKED_FROM links). Pure CPU work — no DB queries.
    fn compute_chunks(
        entity_name: &str,
        parent_uuid: &str,
        entity_ref: &EntityRef,
        data: &BTreeMap<String, CypherValue>,
        entity_configs: &HashMap<String, crate::config::EntityConfig>,
        chunker_cache: &HashMap<ChunkerConfig, Chunker>,
    ) -> (Vec<EntityRecord>, Vec<RelationRecord>) {
        let entity_config = match entity_configs.get(entity_name) {
            Some(cfg) => cfg,
            None => return (vec![], vec![]),
        };

        // Entité déclarée sans chunks : rien à découper, rien à lier. Elle
        // reste écrite et indexée en plein texte — cet index vit sur la
        // table parente. `EntityConfig::validate` a déjà refusé cette
        // déclaration si un signal vecteur ou sparse était demandé.
        if entity_config.chunked == Some(false) {
            return (vec![], vec![]);
        }

        let content_fields = entity_config.content_fields();
        if content_fields.is_empty() {
            return (vec![], vec![]);
        }

        // Get chunker from cache
        let chunker_key = ChunkerConfig::from(&entity_config.chunking);
        let chunker = match chunker_cache.get(&chunker_key) {
            Some(c) => c,
            None => return (vec![], vec![]),
        };

        // Get title from parent entity
        let title = entity_config.title_field()
            .and_then(|f| data.get(f))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let chunk_table = format!("{entity_name}_Chunk");
        let rel_name = format!("{entity_name}_CHUNKED_FROM");

        let mut chunk_entities: Vec<EntityRecord> = Vec::new();
        let mut chunk_relations: Vec<RelationRecord> = Vec::new();

        // Track _content_offset: offset of each field in the concatenation of all content fields
        // Concatenation order = content_fields (sorted alphabetically)
        // Separator = "\n\n" (2 chars) between fields
        let mut content_offset: i64 = 0;

        for (field_idx, field_name) in content_fields.iter().enumerate() {
            let field_text = match data.get(*field_name).and_then(|v| v.as_str()) {
                Some(s) if !s.is_empty() => s,
                _ => {
                    // Empty field still advances offset (0 chars + separator)
                    if field_idx > 0 {
                        content_offset += 2; // "\n\n" separator
                    }
                    continue;
                },
            };

            // Add separator offset for fields after the first
            if field_idx > 0 {
                content_offset += 2; // "\n\n" separator
            }

            let chunks = chunker.chunk(field_text);
            if chunks.is_empty() {
                content_offset += field_text.len() as i64;
                continue;
            }

            for chunk in &chunks {
                let c_uuid = chunk_uuid(parent_uuid, field_name, chunk.index);

                let mut chunk_data = BTreeMap::new();
                chunk_data.insert("_uuid".into(), CypherValue::String(c_uuid.clone()));
                chunk_data.insert("_parent_uuid".into(), CypherValue::String(parent_uuid.to_string()));
                chunk_data.insert("_parent_field".into(), CypherValue::String(field_name.to_string()));
                chunk_data.insert("_text".into(), CypherValue::String(chunk.text.clone()));
                chunk_data.insert("_title".into(), CypherValue::String(title.clone()));
                chunk_data.insert("_text_hash".into(), CypherValue::String(content_hash(&chunk.text)));
                chunk_data.insert("_embed_hash".into(), CypherValue::String(String::new()));
                // Le pendant sparse, vide comme lui : deux marqueurs, deux
                // disponibilités, et aucun des deux n'est acquis à la naissance.
                chunk_data.insert("_sparse_hash".into(), CypherValue::String(String::new()));
                chunk_data.insert("_index".into(), CypherValue::Int(chunk.index as i64));
                chunk_data.insert("_start_char".into(), CypherValue::Int(chunk.start_byte as i64));
                chunk_data.insert("_end_char".into(), CypherValue::Int(chunk.end_byte as i64));
                chunk_data.insert("_start_line".into(), CypherValue::Int(chunk.start_line as i64));
                chunk_data.insert("_end_line".into(), CypherValue::Int(chunk.end_line as i64));
                chunk_data.insert("_core_start_char".into(), CypherValue::Int(chunk.core_start_byte as i64));
                chunk_data.insert("_core_end_char".into(), CypherValue::Int(chunk.core_end_byte as i64));
                chunk_data.insert("_core_start_line".into(), CypherValue::Int(chunk.core_start_line as i64));
                chunk_data.insert("_core_end_line".into(), CypherValue::Int(chunk.core_end_line as i64));
                chunk_data.insert("_content_offset".into(), CypherValue::Int(content_offset));
                // Une ligne dérivée passe sa source à ses chunks (doc du 7 septembre 2026).
                for cle in [crate::config::DerivedConfig::SOURCE_ENTITY, crate::config::DerivedConfig::SOURCE_UUID] {
                    if let Some(v) = data.get(cle) {
                        chunk_data.insert(cle.into(), v.clone());
                    }
                }

                let (chunk_ref, chunk_resolver) = EntityRef::new(&chunk_table);
                chunk_resolver.resolve(c_uuid.clone());

                chunk_entities.push(EntityRecord {
                    entity_name: chunk_table.clone(),
                    data: chunk_data,
                    entity_ref: chunk_ref,
                    resolver: None,
                    vectors: None,
                });

                let (link_ref, link_resolver) = RelationRef::new(&rel_name);
                chunk_relations.push(RelationRecord {
                    rel_name: rel_name.clone(),
                    from: RefOrUuid::Uuid(c_uuid),
                    to: RefOrUuid::Ref(entity_ref.clone()),
                    properties: BTreeMap::new(),
                    relation_ref: link_ref,
                    resolver: Some(link_resolver),
                });
            }

            content_offset += field_text.len() as i64;
        }

        (chunk_entities, chunk_relations)
    }
}


impl Node for ChunkRecordNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "ChunkRecordNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::ChunkRecordNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::ChunkRecordNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        use rayon::prelude::*;

        let items: Vec<EntityRecord> = ctx.take_input("entities")
            .and_then(|pv| pv.take::<BatchPayload>())
            .and_then(|bp| bp.take::<EntityRecord>())
            .ok_or("ChunkRecordNode: missing 'entities' input")?;

        let entity_configs = ctx.service::<HashMap<String, crate::config::EntityConfig>>("entity_configs").cloned()
            .ok_or("ChunkRecordNode: 'entity_configs' service not registered")?;
        let chunker_cache = ctx.service::<Arc<HashMap<ChunkerConfig, Chunker>>>("chunker_cache").cloned()
            .ok_or("ChunkRecordNode: 'chunker_cache' service not registered")?;

        // Parallel chunking via rayon
        let all_results: Vec<(Vec<EntityRecord>, Vec<RelationRecord>)> = items
            .par_iter()
            .map(|rec| {
                let parent_uuid = rec.data
                    .get("_uuid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Self::compute_chunks(
                    &rec.entity_name,
                    parent_uuid,
                    &rec.entity_ref,
                    &rec.data,
                    &entity_configs,
                    &chunker_cache,
                )
            })
            .collect();

        let mut all_chunk_entities: Vec<EntityRecord> = Vec::new();
        let mut all_chunk_relations: Vec<RelationRecord> = Vec::new();
        for (entities, relations) in all_results {
            all_chunk_entities.extend(entities);
            all_chunk_relations.extend(relations);
        }

        ctx.metric("entities", items.len() as f64);
        ctx.metric("chunks", all_chunk_entities.len() as f64);
        ctx.metric("chunk_links", all_chunk_relations.len() as f64);

        // L'autre moitié de `UpdateResult` : combien de chunks ont été créés,
        // par parent. Le service n'existe que pendant un drain qui contient
        // des mises à jour ; `drain` n'applique ensuite ces comptes qu'aux
        // uuid qu'il a effectivement mis à jour, donc un autre producteur de
        // chunks ne peut pas les fausser.
        if let Some(compte) = ctx
            .service::<Arc<Mutex<HashMap<String, (usize, usize)>>>>("chunk_counts")
            .cloned()
        {
            let mut c = compte.lock().map_err(|e| format!("chunk_counts lock: {e}"))?;
            for chunk in &all_chunk_entities {
                if let Some(parent) = chunk.data.get("_parent_uuid").and_then(|v| v.as_str()) {
                    c.entry(parent.to_string()).or_default().1 += 1;
                }
            }
        }

        ctx.trigger("done");
        ctx.set_output("chunks", PortValue::new(
            BatchPayload::new(PortType::Entities, all_chunk_entities),
        ));
        ctx.set_output("chunk_links", PortValue::new(
            BatchPayload::new(PortType::Relations, all_chunk_relations),
        ));
        // Les parents, tels quels : c'est ce que `MarquerDecoupeNode` attend
        // pour poser `_chunked_hash` une fois les chunks posés et liés.
        ctx.set_output("parents", PortValue::new(
            BatchPayload::new(PortType::Entities, items),
        ));
        Ok(())
    }

    // Read-only: nothing to undo, but must return true so rollback doesn't fail
    fn can_undo(&self) -> bool { true }
}

// ─── MarquerDecoupeNode ─────────────────────────────────────────────────────

/// **Pose `_chunked_hash = _content_hash`** sur les parents dont les chunks
/// viennent d'être posés et liés — après, jamais avant : marquer avant que
/// les chunks existent serait le mensonge que `_embed_hash` a appris à ne
/// plus dire.
///
/// C'est la dette de découpage qui devient une ligne de base (réconciliation,
/// C5) : une mise à jour posée au niveau donnée laisse `_content_hash` neuf
/// et `_chunked_hash` ancien, et `Catalog::rattraper_le_decoupage` retrouve
/// ces parents par une requête, sans rien avoir gardé en mémoire.
///
/// **Input**: `entities` — les parents (`EntityRecord`, avec `_uuid` et
/// `_content_hash` dans `data`) ; `trigger` — quand les chunks sont liés.
/// **Output**: `done`.
pub struct MarquerDecoupeNode {
    name: String,
}

impl MarquerDecoupeNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Node for MarquerDecoupeNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "MarquerDecoupeNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::MarquerDecoupeNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::MarquerDecoupeNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let items: Vec<EntityRecord> = ctx.take_input("entities")
            .and_then(|pv| pv.take::<BatchPayload>())
            .and_then(|bp| bp.take::<EntityRecord>())
            .unwrap_or_default();
        let conn = ctx.service::<Arc<dyn DbConnection>>("conn").cloned()
            .ok_or("MarquerDecoupeNode: 'conn' service not registered")?;
        let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect").cloned()
            .ok_or("MarquerDecoupeNode: 'dialect' service not registered")?;

        let mut par_table: HashMap<String, Vec<CypherValue>> = HashMap::new();
        for rec in &items {
            let (Some(uuid), Some(hash)) = (
                rec.data.get("_uuid").and_then(|v| v.as_str()),
                rec.data.get("_content_hash").and_then(|v| v.as_str()),
            ) else {
                continue;
            };
            let mut m = BTreeMap::new();
            m.insert("_uuid".to_string(), CypherValue::String(uuid.to_string()));
            m.insert("_chunked_hash".to_string(), CypherValue::String(hash.to_string()));
            par_table.entry(rec.entity_name.clone()).or_default().push(CypherValue::Map(m));
        }
        let mut marques = 0usize;
        for (table, lignes) in par_table {
            let n = lignes.len();
            let cypher = dialect.batch_update_fields(&table, &["_chunked_hash"]);
            // Un marquage raté laisse la dette visible : ces parents seront
            // redécoupés une fois de trop, jamais une fois de moins. Ce n'est
            // pas une disponibilité perdue, c'est un travail en double.
            if let Err(e) = conn.execute_with_params(
                &cypher,
                &[QueryParam { name: "items".into(), value: CypherValue::List(lignes) }],
            ) {
                ctx.warn(&format!(
                    "MarquerDecoupeNode: « {table} » — {n} parent(s) non marqués ({e}) : ils \
                     seront redécoupés une fois de trop"
                ));
                continue;
            }
            marques += n;
        }
        ctx.metric("marques", marques as f64);
        ctx.trigger("done");
        Ok(())
    }
}

// ─── EmbedNode (simple entities) ────────────────────────────────────────────

/// Embedding node for simple entities (registerEntity API).
/// Configurable column names, signals on the node. No KB dependency.
///
/// **Input**: `entities` — `BatchPayload<EntityRecord>` (PortType::Entities)
/// **Output**: `done` — Empty
/// **Services**: `conn`, `embedder`, `embedding_dim`, optionally `sparse_embedder`, `dual_embedder`
pub struct EmbedNode {
    name: String,
    text_field: String,
    embedding_col: String,
    sparse_col: String,
    signals: search::SearchSignals,
    gpu_batch_size: usize,
    mode: EmbedMode,
    undo_data: Option<serde_json::Value>,
    conn: Option<Arc<dyn DbConnection>>,
    dialect: Option<Arc<dyn crate::dialect::SchemaDialect>>,
}

/// **Ce qu'`EmbedNode` fait de ses vecteurs.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmbedMode {
    /// Les lignes existent : on vérifie leurs marqueurs, on pose les
    /// vecteurs par `SET`, le sparse dans son handle au décalage relu. Le défaut.
    #[default]
    Persist,
    /// Les lignes n'existent pas encore : les vecteurs et leurs marqueurs
    /// s'attachent aux enregistrements (`EntityRecord::vectors`, `_embed_hash`,
    /// `_sparse_hash`), et l'insertion qui suit les pose avec la ligne. Sans
    /// vérification de marqueurs — rien n'est en base — et sans rien à défaire.
    Enrich,
}

impl EmbedNode {
    pub fn new(
        name: impl Into<String>,
        signals: search::SearchSignals,
        gpu_batch_size: usize,
    ) -> Self {
        Self {
            name: name.into(),
            text_field: "_text".into(),
            embedding_col: "embedding".into(),
            sparse_col: "sparse".into(),
            signals,
            gpu_batch_size,
            mode: EmbedMode::Persist,
            undo_data: None,
            conn: None,
            dialect: None,
        }
    }

    pub fn with_mode(mut self, mode: EmbedMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_columns(
        mut self,
        text_field: impl Into<String>,
        embedding_col: impl Into<String>,
        sparse_col: impl Into<String>,
    ) -> Self {
        self.text_field = text_field.into();
        self.embedding_col = embedding_col.into();
        self.sparse_col = sparse_col.into();
        self
    }
}

/// Internal work item for simple embedding.
struct SimpleEmbedWork {
    uuid: String,
    text: String,
    text_hash: String,
    entity_name: String,
}


/// **Embarquer en pipeline.** Des fils de fond envoient les lots au modèle,
/// avec au plus deux lots d'avance ; le fil appelant écrit chaque lot rendu,
/// dans l'ordre où il revient. Le modèle ne voit pas les écritures, la base
/// ne voit pas le modèle — sans ça les deux s'attendaient l'un l'autre
/// (6 septembre 2026 : carte à 36 % pendant une ingestion). Une erreur d'un
/// côté arrête l'autre.
///
/// **Deux producteurs par défaut** (`RAG3WEAVER_EMBED_THREADS`) : en local,
/// `embed` tokenise sur le processeur puis calcule sur la carte, l'un après
/// l'autre — avec deux fils, l'un tokenise pendant que l'autre calcule. Par
/// le démon, deux requêtes en vol font la même chose de son côté.
/// Ce que le pipeline a mesuré : le temps passé dans le modèle (cumulé sur
/// les producteurs) et dans les écritures.
#[derive(Debug, Default, Clone, Copy)]
struct PipelineStats {
    embed_ms: u64,
    write_ms: u64,
}

fn embed_pipeline<W: Sync, V: Send>(
    works: &[W],
    plages: Vec<std::ops::Range<usize>>,
    text_of: impl Fn(&W) -> &str + Sync + Send,
    distant: bool,
    embed: &(dyn Fn(&[String]) -> Result<V, String> + Sync),
    mut write: impl FnMut(&[W], V) -> Result<(), String>,
) -> Result<PipelineStats, String> {
    let embed_ms = std::sync::atomic::AtomicU64::new(0);
    let mut write_ms = 0u64;
    let producteurs = std::env::var("RAG3WEAVER_EMBED_THREADS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(2)
        .min(plages.len().max(1));
    let (tx, rx) = std::sync::mpsc::sync_channel::<Result<(std::ops::Range<usize>, V), String>>(2);
    let suivant = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|s| {
        let text_of = &text_of;
        let plages = &plages;
        let suivant = &suivant;
        let embed_ms = &embed_ms;
        for _ in 0..producteurs {
            let tx = tx.clone();
            s.spawn(move || loop {
                let i = suivant.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(plage) = plages.get(i).cloned() else { return };
                let texts: Vec<String> = works[plage.clone()].iter().map(|w| text_of(w).to_string()).collect();
                let t = std::time::Instant::now();
                let rendu = embed(&texts);
                embed_ms.fetch_add(t.elapsed().as_millis() as u64, std::sync::atomic::Ordering::Relaxed);
                // **Celui qui touche la carte souffle.** Voir `Embedder::distant`.
                if !distant {
                    souffler(t.elapsed());
                }
                let echec = rendu.is_err();
                if tx.send(rendu.map(|v| (plage, v))).is_err() || echec {
                    return;
                }
            });
        }
        drop(tx);
        for message in rx {
            let (plage, v) = message?;
            let t = std::time::Instant::now();
            write(&works[plage], v)?;
            write_ms += t.elapsed().as_millis() as u64;
        }
        Ok(PipelineStats { embed_ms: embed_ms.load(std::sync::atomic::Ordering::Relaxed), write_ms })
    })
}

impl Node for EmbedNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "EmbedNode"
    }
    fn node_config(&self) -> Option<Box<dyn Any + Send>> {
        Some(Box::new(serde_json::json!({
            "gpu_batch_size": self.gpu_batch_size,
            "text_field": self.text_field,
            "embedding_col": self.embedding_col,
            "sparse_col": self.sparse_col,
        })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::EmbedNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::EmbedNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut items: Vec<EntityRecord> = ctx.take_input("entities")
            .and_then(|pv| pv.take::<BatchPayload>())
            .and_then(|bp| bp.take::<EntityRecord>())
            .ok_or("EmbedNode: missing 'entities' input")?;

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn").cloned()
            .ok_or("EmbedNode: 'conn' service not registered")?;
        let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect").cloned()
            .ok_or("EmbedNode: 'dialect' service not registered")?;
        let embedder = ctx.service::<Arc<dyn Embedder>>("embedder").cloned()
            .ok_or("EmbedNode: 'embedder' service not registered")?;
        let embedding_dim = ctx.service::<usize>("embedding_dim").copied()
            .ok_or("EmbedNode: 'embedding_dim' service not registered")?;

        // **Le stockage du modèle courant, résolu depuis la méta** (7 septembre
        // 2026). Un index porte plusieurs modèles ; celui-ci écrit dans sa
        // colonne et juge la fraîcheur par son marqueur. Sans catalogue — le
        // montage minimal d'un test — on garde `embedding_col` et `_embed_hash`.
        let models = ctx
            .service::<Vec<crate::embedding_storage::EmbeddingModelEntry>>("embedding_models")
            .cloned();
        let slug = ctx.service::<String>("embedding_slug").cloned();
        let current_entry = match (&models, &slug) {
            (Some(m), Some(s)) => Some(m.iter().find(|e| e.slug() == *s).cloned().ok_or_else(|| {
                format!("EmbedNode: {}", crate::embedding_storage::unavailable_message(s, m))
            })?),
            _ => None,
        };
        // La dimension est celle du modèle, pas d'une config globale.
        let embedding_dim = current_entry.as_ref().map(|e| e.dim).unwrap_or(embedding_dim);
        let default_col = self.embedding_col.clone();
        let storage_for = |chunk_table: &str| -> (String, String) {
            match &current_entry {
                Some(e) => {
                    let s = crate::embedding_storage::VectorStorage::resolve(chunk_table, e);
                    (s.column, s.marker)
                }
                None => (default_col.clone(), "_embed_hash".to_string()),
            }
        };
        let has_sparse_svc = ctx.service::<bool>("has_sparse").copied().unwrap_or(false);
        let has_dual_svc = ctx.service::<bool>("has_dual").copied().unwrap_or(false);
        let sparse_embedder = ctx.service::<Arc<dyn SparseEmbedder>>("sparse_embedder").cloned();
        let dual_embedder = ctx.service::<Arc<dyn DualEmbedder>>("dual_embedder").cloned();

        // Per-entity signals: read from entity_configs if available, fallback to self.signals.
        let entity_configs = ctx.service::<HashMap<String, crate::config::EntityConfig>>("entity_configs").cloned();
        let resolve_signals = |entity_name: &str| -> search::SearchSignals {
            entity_configs.as_ref()
                .and_then(|cfgs| cfgs.get(entity_name))
                .map(|ec| ec.signals)
                .unwrap_or(self.signals)
        };

        // Build work items from chunk entities, grouped by per-entity signals
        let mut dense_works: Vec<SimpleEmbedWork> = Vec::new();
        let mut sparse_works: Vec<SimpleEmbedWork> = Vec::new();
        let mut dual_works: Vec<SimpleEmbedWork> = Vec::new();

        for rec in &mut items {
            let uuid = rec
                .entity_ref
                .ready()
                .map_err(|e| format!("embed ref resolution failed: {e}"))?;

            let embed_text = match rec.data.get(&self.text_field).and_then(|v| v.as_str()) {
                Some(t) if !t.is_empty() => t.to_string(),
                _ => continue,
            };
            let text_hash = rec.data.get("_text_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| content_hash(&embed_text));

            // Resolve signals for the parent entity (chunk entity_name is "{Parent}_Chunk")
            let parent_entity = rec.entity_name.strip_suffix("_Chunk").unwrap_or(&rec.entity_name);
            let signals = resolve_signals(parent_entity);
            let want_vector = signals.vector();
            let want_sparse = signals.sparse() && has_sparse_svc;

            if has_dual_svc && want_vector && want_sparse && dual_embedder.is_some() {
                dual_works.push(SimpleEmbedWork {
                    uuid: uuid.clone(),
                    text: embed_text,
                    text_hash,
                    entity_name: rec.entity_name.clone(),
                });
            } else {
                if want_vector {
                    dense_works.push(SimpleEmbedWork {
                        uuid: uuid.clone(),
                        text: embed_text.clone(),
                        text_hash: text_hash.clone(),
                        entity_name: rec.entity_name.clone(),
                    });
                }
                if want_sparse && sparse_embedder.is_some() {
                    sparse_works.push(SimpleEmbedWork {
                        uuid: uuid.clone(),
                        text: embed_text,
                        text_hash,
                        entity_name: rec.entity_name.clone(),
                    });
                }
            }
        }

        // ── Idempotence: skip chunks whose text hasn't changed ──
        let all_uuids: HashSet<&str> = dense_works.iter()
            .chain(sparse_works.iter())
            .chain(dual_works.iter())
            .map(|w| w.uuid.as_str())
            .collect();

        // Un marqueur par signal : `_embed_hash` pour le dense, `_sparse_hash`
        // pour le sparse. Un chunk peut être à jour pour l'un et devoir l'autre.
        let mut hash_dense: HashMap<String, String> = HashMap::new();
        let mut hash_sparse: HashMap<String, String> = HashMap::new();
        let enrich = self.mode == EmbedMode::Enrich;
        // En mode Enrich, rien n'est en base : tout est à faire.
        if !enrich && !all_uuids.is_empty() {
            let mut by_entity: HashMap<&str, Vec<&str>> = HashMap::new();
            for w in dense_works.iter().chain(sparse_works.iter()).chain(dual_works.iter()) {
                by_entity.entry(&w.entity_name).or_default().push(&w.uuid);
            }
            for (entity_name, uuids) in &by_entity {
                let unique: HashSet<&&str> = uuids.iter().collect();
                let items_param = CypherValue::List(
                    unique.iter().map(|&&u| {
                        let mut m = BTreeMap::new();
                        m.insert("uuid".into(), CypherValue::String(u.to_string()));
                        CypherValue::Map(m)
                    }).collect()
                );
                let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
                    .ok_or("EmbedNode: 'dialect' service not registered")?;
                let (_, marker) = storage_for(entity_name);
                let cypher = dialect.embed_check_hashes(entity_name, &marker);
                if let Ok(result) = conn.execute_with_params(
                    &cypher,
                    &[QueryParam { name: "items".into(), value: items_param }],
                ) {
                    for row in &result.rows {
                        let Some(uuid) = row.first().and_then(|v| v.as_str()) else { continue };
                        // Vide vaut absent : c'est la valeur qu'un chunk neuf
                        // porte à sa naissance. Nul aussi : la colonne d'un
                        // modèle ajouté après coup n'existe pas sur les lignes
                        // d'avant, et un chunk inséré sans elle la laisse nulle.
                        if let Some(h) = row.get(1).and_then(|v| v.as_str()).filter(|h| !h.is_empty()) {
                            hash_dense.insert(uuid.to_string(), h.to_string());
                        }
                        if let Some(h) = row.get(2).and_then(|v| v.as_str()).filter(|h| !h.is_empty()) {
                            hash_sparse.insert(uuid.to_string(), h.to_string());
                        }
                    }
                }
            }
        }

        // **Chaque signal juge par son propre marqueur.** Un seul les jugeait
        // tous les trois : un chunk embarqué en dense était donc réputé fait
        // pour le sparse aussi, et n'y passait jamais. C'est le défaut que la
        // colonne `_sparse_hash` du schéma v3 existe pour supprimer.
        let a_jour = |carte: &HashMap<String, String>, uuid: &str, texte: &str| -> bool {
            carte.get(uuid).is_some_and(|pose| pose == texte)
        };
        let dense_a_faire = |w: &SimpleEmbedWork| !a_jour(&hash_dense, &w.uuid, &w.text_hash);
        let sparse_a_faire = |w: &SimpleEmbedWork| !a_jour(&hash_sparse, &w.uuid, &w.text_hash);

        let pre_filter = dense_works.len() + sparse_works.len() + dual_works.len();
        dense_works.retain(dense_a_faire);
        sparse_works.retain(sparse_a_faire);
        // Le dual produit les deux : il repasse si **l'un des deux** manque.
        dual_works.retain(|w| dense_a_faire(w) || sparse_a_faire(w));
        let skipped = pre_filter - (dense_works.len() + sparse_works.len() + dual_works.len());

        ctx.metric("entities", items.len() as f64);
        ctx.metric("dense", dense_works.len() as f64);
        ctx.metric("sparse", sparse_works.len() as f64);
        ctx.metric("dual", dual_works.len() as f64);
        ctx.metric("skipped_unchanged", skipped as f64);

        // Les vecteurs gardés en mémoire (mode Enrich), par uuid.
        let mut dense_done: HashMap<String, Vec<f32>> = HashMap::new();
        let mut sparse_done: HashMap<String, SparseVector> = HashMap::new();

        // ── Dense embedding (GPU mini-batches) ──
        if !dense_works.is_empty() {
            // **Par longueur, pour que le lot ne rembourre presque rien.** Le
            // tokenizer rembourre au plus long du lot ; dans l'ordre d'arrivée un
            // lot mêle des chunks de 50 et de 1000 caractères et la carte calcule
            // les blancs — mesuré le 6 septembre 2026 sur `src/dataflow`. Stable :
            // les résultats se relisent par position.
            dense_works.sort_by_key(|w| w.text.len());
            let lens: Vec<usize> = dense_works.iter().map(|w| w.text.len()).collect();
            // **En pipeline** : la carte calcule le lot suivant pendant que ce
            // fil écrit le précédent en base. Mesuré le 6 septembre 2026 : la
            // carte était à 36 % en moyenne pendant une ingestion, le reste
            // du temps elle attendait le JSON et les écritures.
            let plages = stable_batches(&lens, lot_budget(embedder.budget_conseille(), self.gpu_batch_size));
            let embed_dense = |texts: &[String]| embedder.embed(texts).map_err(|e| format!("dense embedding failed: {e}"));
            let stats = embed_pipeline(&dense_works, plages, |w| &w.text, embedder.distant(), &embed_dense, |chunk, vectors| {
                if vectors.len() != chunk.len() {
                    return Err(format!(
                        "embedder returned {} vectors for {} texts",
                        vectors.len(), chunk.len()
                    ));
                }
                if enrich {
                    for (work, vector) in chunk.iter().zip(vectors.into_iter()) {
                        if vector.len() != embedding_dim {
                            return Err(format!("embedding dimension mismatch: expected {}, got {}", embedding_dim, vector.len()));
                        }
                        dense_done.insert(work.uuid.clone(), vector);
                    }
                    return Ok(());
                }

                // Group by entity_name for batch UNWIND
                let mut groups: HashMap<&str, Vec<(&SimpleEmbedWork, &Vec<f32>)>> = HashMap::new();
                for (work, vector) in chunk.iter().zip(vectors.iter()) {
                    if vector.len() != embedding_dim {
                        return Err(format!(
                            "embedding dimension mismatch: expected {}, got {}",
                            embedding_dim, vector.len()
                        ));
                    }
                    groups.entry(&work.entity_name).or_default().push((work, vector));
                }

                for (entity_name, group) in &groups {
                    let items_param = CypherValue::List(
                        group.iter().map(|(work, vec)| {
                            let mut map = BTreeMap::new();
                            map.insert("uuid".into(), CypherValue::String(work.uuid.clone()));
                            map.insert("hash".into(), CypherValue::String(work.text_hash.clone()));
                            map.insert("emb".into(), CypherValue::List(
                                vec.iter().map(|&f| CypherValue::Float(f as f64)).collect(),
                            ));
                            CypherValue::Map(map)
                        }).collect(),
                    );

                    let (col, marker) = storage_for(entity_name);
                    let cypher = dialect.embed_set(entity_name, &col, &marker);

                    conn.execute_with_params(
                        &cypher,
                        &[QueryParam { name: "items".into(), value: items_param }],
                    ).map_err(|e| e.to_string())?;
                }
                Ok(())
            })?;
            ctx.metric("model_ms", stats.embed_ms as f64);
            ctx.metric("write_ms", stats.write_ms as f64);
        }

        // ── Sparse embedding (GPU mini-batches) → insert into SparseHandle ──
        let sparse_handles = ctx.service::<HashMap<String, Arc<sparse_vector::handle::SparseHandle>>>("sparse_handles").cloned();
        if !sparse_works.is_empty() {
            if let Some(ref sparse_emb) = sparse_embedder {
                // **Par longueur, pour que le lot ne rembourre presque rien.** Le
                // tokenizer rembourre au plus long du lot ; dans l'ordre d'arrivée un
                // lot mêle des chunks de 50 et de 1000 caractères et la carte calcule
                // les blancs — mesuré le 6 septembre 2026 sur `src/dataflow`. Stable :
                // les résultats se relisent par position.
                sparse_works.sort_by_key(|w| w.text.len());
                let lens: Vec<usize> = sparse_works.iter().map(|w| w.text.len()).collect();
                let plages = stable_batches(&lens, lot_budget(embedder.budget_conseille(), self.gpu_batch_size));
                let appel = |texts: &[String]| sparse_emb.embed_sparse(texts).map_err(|e| format!("sparse embedding failed: {e}"));
                let stats = embed_pipeline(&sparse_works, plages, |w| &w.text, sparse_emb.distant(), &appel, |chunk, sparse_vecs| {
                    if sparse_vecs.len() != chunk.len() {
                        return Err(format!(
                            "sparse embedder returned {} vectors for {} texts",
                            sparse_vecs.len(), chunk.len()
                        ));
                    }
                    if enrich {
                        for (work, sv) in chunk.iter().zip(sparse_vecs.into_iter()) {
                            sparse_done.insert(work.uuid.clone(), sv);
                        }
                        return Ok(());
                    }

                    let mut groups: HashMap<&str, Vec<(&SimpleEmbedWork, &SparseVector)>> =
                        HashMap::new();
                    for (work, sv) in chunk.iter().zip(sparse_vecs.iter()) {
                        groups.entry(&work.entity_name).or_default().push((work, sv));
                    }

                    for (entity_name, group) in &groups {
                        // Même inversion qu'au chemin sparse pur, même
                        // correction : on lit le décalage sans rien poser.
                        //
                        // **Mais ici elle ne suffit pas**, et il faut le dire :
                        // sur le chemin dual, l'écriture dense pose le *même*
                        // `_embed_hash` un peu plus bas. Un vecteur sparse
                        // perdu reste donc marqué par le dense. Un marqueur
                        // unique ne peut pas répondre séparément « dense
                        // prêt ? » et « sparse prêt ? » — c'est ce que les
                        // quatre disponibilités vont demander, et ça réclame
                        // une seconde colonne. Migration, donc décision.
                        let items_param = CypherValue::List(
                            group.iter().map(|(work, _)| {
                                let mut map = BTreeMap::new();
                                map.insert("uuid".into(), CypherValue::String(work.uuid.clone()));
                                map.insert("hash".into(), CypherValue::String(work.text_hash.clone()));
                                CypherValue::Map(map)
                            }).collect(),
                        );

                        let cypher = dialect.embed_get_offset(entity_name);

                        let result = conn.execute_with_params(
                            &cypher,
                            &[QueryParam { name: "items".into(), value: items_param }],
                        ).map_err(|e| e.to_string())?;

                        // Insert into SparseHandle using offsets
                        if let Some(ref handles) = sparse_handles {
                            if let Some(handle) = handles.get(*entity_name) {
                                let uuid_to_sv: HashMap<&str, &SparseVector> = group.iter()
                                    .map(|(w, sv)| (w.uuid.as_str(), *sv))
                                    .collect();
                                for row in &result.rows {
                                    if let (Some(uuid), Some(offset)) = (
                                        row.first().and_then(|v| v.as_str()),
                                        row.get(1).and_then(|v| v.as_i64()),
                                    ) {
                                        if let Some(sv) = uuid_to_sv.get(uuid) {
                                            let sv_idx = sparse_vector::index::SparseVector::new(
                                                sv.indices.clone(), sv.values.clone(),
                                            );
                                            handle.insert(offset as u64, &sv_idx)
                                                .map_err(|e| format!("sparse insert failed: {e}"))?;
                                        }
                                    }
                                }

                                // Le marqueur sparse, après son vecteur. Le
                                // dense posera le sien plus bas, sur sa propre
                                // colonne : c'est ce qui rend les deux
                                // disponibilités indépendantes.
                                let marques = CypherValue::List(
                                    group.iter().map(|(work, _)| {
                                        let mut m = BTreeMap::new();
                                        m.insert("_uuid".into(),
                                            CypherValue::String(work.uuid.clone()));
                                        m.insert("_sparse_hash".into(),
                                            CypherValue::String(work.text_hash.clone()));
                                        CypherValue::Map(m)
                                    }).collect(),
                                );
                                let pose = dialect
                                    .batch_update_fields(entity_name, &["_sparse_hash"]);
                                conn.execute_with_params(
                                    &pose,
                                    &[QueryParam { name: "items".into(), value: marques }],
                                ).map_err(|e| e.to_string())?;
                            }
                        }
                    }
                    Ok(())
                })?;
            ctx.metric("model_ms", stats.embed_ms as f64);
            ctx.metric("write_ms", stats.write_ms as f64);
            }
        }

        // ── Dual embedding (GPU mini-batches) ──
        if !dual_works.is_empty() {
            if let Some(ref dual_emb) = dual_embedder {
                let mut dense_results: Vec<(&SimpleEmbedWork, Vec<f32>)> = Vec::with_capacity(dual_works.len());
                let mut sparse_results: Vec<(&SimpleEmbedWork, SparseVector)> = Vec::with_capacity(dual_works.len());

                // **Par longueur, pour que le lot ne rembourre presque rien.** Le

                // tokenizer rembourre au plus long du lot ; dans l'ordre d'arrivée un

                // lot mêle des chunks de 50 et de 1000 caractères et la carte calcule

                // les blancs — mesuré le 6 septembre 2026 sur `src/dataflow`. Stable :

                // les résultats se relisent par position.

                dual_works.sort_by_key(|w| w.text.len());

                let lens: Vec<usize> = dual_works.iter().map(|w| w.text.len()).collect();
                let plages = stable_batches(&lens, lot_budget(embedder.budget_conseille(), self.gpu_batch_size));
                let appel = |texts: &[String]| dual_emb.embed_dual(texts).map_err(|e| format!("dual embed failed: {e}"));
                let stats = embed_pipeline(&dual_works, plages, |w| &w.text, dual_emb.distant(), &appel, |chunk, (dense_vecs, sparse_vecs)| {
                    if dense_vecs.len() != chunk.len() || sparse_vecs.len() != chunk.len() {
                        return Err(format!(
                            "dual embedder returned {}/{} vectors for {} texts",
                            dense_vecs.len(), sparse_vecs.len(), chunk.len()
                        ));
                    }

                    let base_idx = dense_results.len();
                    for (i, (dense, sparse)) in
                        dense_vecs.into_iter().zip(sparse_vecs.into_iter()).enumerate()
                    {
                        dense_results.push((&dual_works[base_idx + i], dense));
                        sparse_results.push((&dual_works[base_idx + i], sparse));
                    }
                    Ok(())
                })?;
            ctx.metric("model_ms", stats.embed_ms as f64);
            ctx.metric("write_ms", stats.write_ms as f64);

                if enrich {
                    for (work, dense) in dense_results.drain(..) {
                        if dense.len() != embedding_dim {
                            return Err(format!("embedding dimension mismatch: expected {}, got {}", embedding_dim, dense.len()));
                        }
                        dense_done.insert(work.uuid.clone(), dense);
                    }
                    for (work, sparse) in sparse_results.drain(..) {
                        sparse_done.insert(work.uuid.clone(), sparse);
                    }
                }

                // UNWIND dense (sets embedding + _embed_hash)
                {
                    let mut groups: HashMap<&str, Vec<(&SimpleEmbedWork, &Vec<f32>)>> = HashMap::new();
                    for (work, vec) in &dense_results {
                        if vec.len() != embedding_dim {
                            return Err(format!(
                                "embedding dimension mismatch: expected {}, got {}",
                                embedding_dim, vec.len()
                            ));
                        }
                        groups.entry(&work.entity_name).or_default().push((work, vec));
                    }

                    for (entity_name, group) in &groups {
                        let items_param = CypherValue::List(
                            group.iter().map(|(work, vec)| {
                                let mut map = BTreeMap::new();
                                map.insert("uuid".into(), CypherValue::String(work.uuid.clone()));
                                map.insert("hash".into(), CypherValue::String(work.text_hash.clone()));
                                map.insert("emb".into(), CypherValue::List(
                                    vec.iter().map(|&f| CypherValue::Float(f as f64)).collect(),
                                ));
                                CypherValue::Map(map)
                            }).collect(),
                        );

                        let (col, marker) = storage_for(entity_name);
                        let cypher = dialect.embed_set(entity_name, &col, &marker);

                        conn.execute_with_params(
                            &cypher,
                            &[QueryParam { name: "items".into(), value: items_param }],
                        ).map_err(|e| e.to_string())?;
                    }
                }

                // Insert sparse into SparseHandle (offsets resolved via MATCH)
                {
                    let mut groups: HashMap<&str, Vec<(&SimpleEmbedWork, &SparseVector)>> =
                        HashMap::new();
                    for (work, sv) in &sparse_results {
                        groups.entry(&work.entity_name).or_default().push((work, sv));
                    }

                    for (entity_name, group) in &groups {
                        // Get offsets for each uuid
                        let items_param = CypherValue::List(
                            group.iter().map(|(work, _)| {
                                let mut map = BTreeMap::new();
                                map.insert("uuid".into(), CypherValue::String(work.uuid.clone()));
                                CypherValue::Map(map)
                            }).collect(),
                        );

                        let cypher = dialect.embed_get_offset(entity_name);

                        let result = conn.execute_with_params(
                            &cypher,
                            &[QueryParam { name: "items".into(), value: items_param }],
                        ).map_err(|e| e.to_string())?;

                        if let Some(ref handles) = sparse_handles {
                            if let Some(handle) = handles.get(*entity_name) {
                                let uuid_to_sv: HashMap<&str, &SparseVector> = group.iter()
                                    .map(|(w, sv)| (w.uuid.as_str(), *sv))
                                    .collect();
                                for row in &result.rows {
                                    if let (Some(uuid), Some(offset)) = (
                                        row.first().and_then(|v| v.as_str()),
                                        row.get(1).and_then(|v| v.as_i64()),
                                    ) {
                                        if let Some(sv) = uuid_to_sv.get(uuid) {
                                            let sv_idx = sparse_vector::index::SparseVector::new(
                                                sv.indices.clone(), sv.values.clone(),
                                            );
                                            handle.insert(offset as u64, &sv_idx)
                                                .map_err(|e| format!("sparse insert failed: {e}"))?;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // **Les vecteurs s'attachent aux enregistrements** (mode Enrich) :
        // le marqueur dans `data`, le vecteur à côté, et l'insertion qui
        // suit pose tout avec la ligne.
        if enrich {
            let mut attaches = 0usize;
            for rec in &mut items {
                let Some(uuid) = rec.data.get("_uuid").and_then(|v| v.as_str()).map(|s| s.to_string()) else { continue };
                // Par `get` et non `remove` : un uuid en double dans le lot
                // (la dernière occurrence gagne à l'insertion) doit être
                // enrichi à chaque occurrence, sinon c'est la nue qui reste.
                let dense = dense_done.get(&uuid).cloned();
                let sparse = sparse_done.get(&uuid).cloned();
                if dense.is_none() && sparse.is_none() {
                    continue;
                }
                let hash = rec.data.get("_text_hash").and_then(|v| v.as_str()).map(|s| s.to_string())
                    .or_else(|| rec.data.get(&self.text_field).and_then(|v| v.as_str()).map(content_hash))
                    .unwrap_or_default();
                let vectors = rec.vectors.get_or_insert_with(Default::default);
                if let Some(values) = dense {
                    let (col, marker) = storage_for(&rec.entity_name);
                    rec.data.insert(marker, CypherValue::String(hash.clone()));
                    vectors.dense = Some(crate::records::DenseVector { column: col, values });
                }
                if let Some(sv) = sparse {
                    rec.data.insert("_sparse_hash".into(), CypherValue::String(hash));
                    vectors.sparse = Some(sv);
                }
                attaches += 1;
            }
            ctx.metric("enriched", attaches as f64);
        } else {
            // Capture undo data
            let mut undo_groups: HashMap<&str, Vec<&str>> = HashMap::new();
            for w in dense_works.iter().chain(sparse_works.iter()).chain(dual_works.iter()) {
                undo_groups.entry(&w.entity_name).or_default().push(&w.uuid);
            }
            // Le marqueur voyage avec les uuids : l'undo n'a pas de contexte
            // pour le résoudre, et remettre `_embed_hash` à nul sur un chunk
            // marqué `_embed_hash__granite_278m` ne défairait rien.
            let undo_map: HashMap<String, serde_json::Value> = undo_groups.into_iter()
                .map(|(k, v)| {
                    let (_, marker) = storage_for(k);
                    (k.to_string(), serde_json::json!({ "marker": marker, "uuids": v }))
                })
                .collect();
            if !undo_map.is_empty() {
                self.undo_data = Some(serde_json::json!(undo_map));
            }
        }

        // Store services for undo
        self.conn = Some(conn.clone());
        self.dialect = Some(dialect.clone());

        ctx.trigger("done");
        ctx.set_output("embedded", PortValue::new(
            BatchPayload::new(PortType::Entities, items),
        ));
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<Box<dyn Any + Send>> {
        self.undo_data.clone().map(|v| Box::new(v) as Box<dyn Any + Send>)
    }

    fn undo(&mut self, undo_ctx: Box<dyn Any + Send>) -> Result<(), String> {
        let undo_ctx = *undo_ctx.downcast::<serde_json::Value>().map_err(|_| "bad undo ctx")?;
        let conn = self.conn.as_ref()
            .ok_or("EmbedNode undo: 'conn' not stored")?;
        let dialect = self.dialect.as_ref()
            .ok_or("EmbedNode undo: 'dialect' not stored")?;

        let groups = undo_ctx.as_object()
            .ok_or("EmbedNode undo: expected object")?;

        for (entity_name, payload) in groups {
            let marker = payload.get("marker").and_then(|m| m.as_str()).unwrap_or("_embed_hash").to_string();
            let uuid_list: Vec<&str> = payload.get("uuids").and_then(|u| u.as_array())
                .ok_or("EmbedNode undo: expected array of uuids")?
                .iter()
                .filter_map(|v| v.as_str())
                .collect();

            if uuid_list.is_empty() { continue; }

            let uuid_params = CypherValue::List(
                uuid_list.iter().map(|u| CypherValue::String(u.to_string())).collect()
            );
            // **Les deux marqueurs, ou l'undo ment à moitié.** Annuler un
            // embarquement en ne remettant que `_embed_hash` laisserait
            // `_sparse_hash` posé : le chunk serait réembarqué en dense et
            // jamais en sparse, sans que rien ne le signale.
            for colonne in [marker.as_str(), "_sparse_hash"] {
                let cypher = dialect.batch_set_null(entity_name, colonne);
                conn.execute_with_params(
                    &cypher,
                    &[QueryParam { name: "uuids".into(), value: uuid_params.clone() }],
                ).map_err(|e| format!("EmbedNode undo failed: {e}"))?;
            }
        }
        Ok(())
    }
}

// ─── FlushNode ─────────────────────────────────────────────────────────────

/// Flushes Lucivy FTS indexes for configured tables.
///
/// Generic node — works with any table that has a Lucivy index.
/// Runs `CALL FLUSH_LUCIVY_INDEX('{table}')` to commit + reload the reader
/// so subsequent searches don't pay the lazy-flush cost.
///
/// **Config**: `tables` — list of table names to flush
/// **Input**: `trigger` — Empty signal (optional)
/// **Output**: `done` — Empty signal
/// **Services**: `conn` (DbConnection)
pub struct FlushNode {
    name: String,
    tables: Vec<String>,
    undo_data: Option<serde_json::Value>,
    // Les handles Rust, comme le fait déjà `SparseCommitNode`. Auparavant ce
    // nœud gardait la connexion pour rejouer `CALL FLUSH_LUCIVY_INDEX` à l'undo.
    fts_handles: Option<HashMap<String, Arc<lucivy_core::sharded_handle::ShardedHandle>>>,
}

impl FlushNode {
    pub fn new(name: impl Into<String>, tables: Vec<String>) -> Self {
        Self { name: name.into(), tables, undo_data: None, fts_handles: None }
    }
}


impl Node for FlushNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "FlushNode"
    }
    fn node_config(&self) -> Option<Box<dyn Any + Send>> {
        Some(Box::new(serde_json::json!({ "tables": self.tables })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::FlushNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::FlushNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        // Commit des index FTS Rust. Le repli `CALL FLUSH_LUCIVY_INDEX` est
        // débranché : il rendait un commit manqué indiscernable d'un succès,
        // puisqu'il flushait l'index C++ pendant que le nôtre restait sale.
        let fts_handles = ctx
            .service::<HashMap<String, Arc<lucivy_core::sharded_handle::ShardedHandle>>>("fts_handles")
            .cloned();

        let mut flushed: usize = 0;
        for table in &self.tables {
            let Some(handle) = fts_handles.as_ref().and_then(|h| h.get(table)) else {
                // Une table sans handle n'a pas d'index FTS : rien à committer.
                continue;
            };
            // `commit()` est idempotent : il flush et recharge la lecture.
            // Les merges partent en tâche de fond selon la policy.
            match handle.commit() {
                Ok(()) => flushed += 1,
                // Une table qui ne commite pas ne tue plus les autres : le
                // plein texte de celle-ci est perdu, et ça se dit comme tel.
                Err(e) => consigner_l_echec(
                    ctx, "FlushNode", table, 0, Disponibilites::PLEIN_TEXTE,
                    format!("commit FTS échoué : {e}"),
                ),
            }
        }

        // Capture tables for undo (re-flush)
        self.undo_data = Some(serde_json::json!(self.tables));
        self.fts_handles = fts_handles;

        ctx.metric("table_count", self.tables.len() as f64);
        ctx.metric("flushed", flushed as f64);
        ctx.trigger("done");
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<Box<dyn Any + Send>> {
        self.undo_data.clone().map(|v| Box::new(v) as Box<dyn Any + Send>)
    }

    fn undo(&mut self, undo_ctx: Box<dyn Any + Send>) -> Result<(), String> {
        let undo_ctx = *undo_ctx.downcast::<serde_json::Value>().map_err(|_| "bad undo ctx")?;
        let handles = self.fts_handles.as_ref()
            .ok_or("FlushNode undo: 'fts_handles' not stored")?;

        let tables = undo_ctx.as_array()
            .ok_or("FlushNode undo: expected array of table names")?;

        // Un commit est idempotent : le rejouer est le seul « undo » qui ait un
        // sens ici (on ne dé-commite pas un index). Best-effort, comme avant.
        for t in tables {
            if let Some(handle) = t.as_str().and_then(|table| handles.get(table)) {
                let _ = handle.commit();
            }
        }
        Ok(())
    }
}

// ─── SparseCommitNode ─────────────────────────────────────────────────────

/// Commits dirty sparse vector indexes for configured tables.
///
/// Same pattern as [`FlushNode`] for FTS — explicit commit via dataflow node.
/// Calls `handle.commit_inner()` on each configured table's `SparseHandle`.
///
/// **Config**: `tables` — list of table names to commit
/// **Input**: `trigger` — Empty signal (optional)
/// **Output**: `done` — Empty signal
/// **Services**: `sparse_handles` (HashMap<String, Arc<SparseHandle>>)
pub struct SparseCommitNode {
    name: String,
    tables: Vec<String>,
    undo_data: Option<serde_json::Value>,
    sparse_handles: Option<Arc<HashMap<String, Arc<sparse_vector::handle::SparseHandle>>>>,
}

impl SparseCommitNode {
    pub fn new(name: impl Into<String>, tables: Vec<String>) -> Self {
        Self { name: name.into(), tables, undo_data: None, sparse_handles: None }
    }
}


impl Node for SparseCommitNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "SparseCommitNode"
    }
    fn node_config(&self) -> Option<Box<dyn Any + Send>> {
        Some(Box::new(serde_json::json!({ "tables": self.tables })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::SparseCommitNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::SparseCommitNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let handles = ctx.service::<HashMap<String, Arc<sparse_vector::handle::SparseHandle>>>("sparse_handles").cloned()
            .ok_or("SparseCommitNode: 'sparse_handles' service not registered")?;

        let mut committed: usize = 0;
        for table in &self.tables {
            if let Some(handle) = handles.get(table) {
                handle.commit_inner()
                    .map_err(|e| format!("SparseCommitNode: commit '{table}' failed: {e}"))?;
                committed += 1;
            }
        }

        self.undo_data = Some(serde_json::json!(self.tables));
        self.sparse_handles = Some(Arc::new(handles));
        ctx.metric("table_count", self.tables.len() as f64);
        ctx.metric("committed", committed as f64);
        ctx.trigger("done");
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<Box<dyn Any + Send>> {
        self.undo_data.clone().map(|v| Box::new(v) as Box<dyn Any + Send>)
    }

    fn undo(&mut self, undo_ctx: Box<dyn Any + Send>) -> Result<(), String> {
        let undo_ctx = *undo_ctx.downcast::<serde_json::Value>().map_err(|_| "bad undo ctx")?;
        let handles = self.sparse_handles.as_ref()
            .ok_or("SparseCommitNode undo: 'sparse_handles' not stored")?;

        let tables = undo_ctx.as_array()
            .ok_or("SparseCommitNode undo: expected array of table names")?;

        for t in tables {
            if let Some(table) = t.as_str() {
                if let Some(handle) = handles.get(table) {
                    handle.commit_inner().ok(); // Best-effort
                }
            }
        }
        Ok(())
    }
}

// ─── RechunkDeleteNode ─────────────────────────────────────────────────────

/// Delete old chunks for entities about to be re-chunked.
///
/// Batch-deletes `{Entity}_Chunk` nodes by `_parent_uuid`, then passes the
/// same `Vec<EntityRecord>` through to output for downstream ChunkRecordNode.
///
/// **Input**: `entities` — `BatchPayload<EntityRecord>` (PortType::Entities)
/// **Output**: `entities` — same records (pass-through)
/// **Services**: `conn` (DbConnection)
pub struct RechunkDeleteNode {
    name: String,
}

impl RechunkDeleteNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}


impl Node for RechunkDeleteNode {
    fn name(&self) -> &str { &self.name }
    fn node_type(&self) -> &'static str { "RechunkDeleteNode" }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::RechunkDeleteNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::RechunkDeleteNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let items: Vec<EntityRecord> = ctx.take_input("entities")
            .and_then(|pv| pv.take::<BatchPayload>())
            .and_then(|bp| bp.take::<EntityRecord>())
            .ok_or("RechunkDeleteNode: missing 'entities' input")?;

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn").cloned()
            .ok_or("RechunkDeleteNode: 'conn' service not registered")?;

        // Group by entity_name
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        for rec in &items {
            let uuid = rec.data.get("_uuid")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            groups.entry(rec.entity_name.clone()).or_default().push(uuid);
        }

        // **Le compte par uuid, pas seulement le total.** La requête rend
        // `uuid, count(c)` : le détail était là et partait dans une somme. Il
        // sert à `UpdateResult.chunks_deleted`, qui était un zéro écrit en dur
        // — un nombre présenté comme une mesure et qui n'en était pas une.
        let par_uuid = ctx
            .service::<Arc<Mutex<HashMap<String, (usize, usize)>>>>("chunk_counts")
            .cloned();

        let mut total_deleted: usize = 0;
        for (entity_name, uuids) in &groups {
            let chunk_table = format!("{entity_name}_Chunk");
            let uuid_list = CypherValue::List(
                uuids.iter().map(|u| CypherValue::String(u.clone())).collect(),
            );
            let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
                .ok_or("RechunkDeleteNode: 'dialect' service not registered")?;
            let cypher = dialect.batch_cascade_delete_returning_count(&chunk_table, "_parent_uuid");
            let result = conn
                .execute_with_params(
                    &cypher,
                    &[QueryParam { name: "uuids".into(), value: uuid_list }],
                )
                .map_err(|e| e.to_string())?;
            for row in &result.rows {
                let Some(cnt) = row.get(1).and_then(|v| v.as_i64()) else { continue };
                total_deleted += cnt as usize;
                if let (Some(uuid), Some(compte)) = (row.first().and_then(|v| v.as_str()), &par_uuid) {
                    compte
                        .lock()
                        .map_err(|e| format!("chunk_counts lock: {e}"))?
                        .entry(uuid.to_string())
                        .or_default()
                        .0 += cnt as usize;
                }
            }
        }

        ctx.metric("entities", items.len() as f64);
        ctx.metric("groups", groups.len() as f64);
        ctx.metric("chunks_deleted", total_deleted as f64);

        ctx.set_output("entities", PortValue::new(
            BatchPayload::new(PortType::Entities, items),
        ));
        Ok(())
    }

    // Deletes old chunks before re-chunking; undo is a no-op because
    // re-ingestion will recreate chunks from the restored entity data.
    fn can_undo(&self) -> bool { true }
}

/// **L'entité a-t-elle une table de chunks ?** Seules les entités à pipeline
/// simple et découpées en ont une ; une entité de données seules (que des
/// dérivées rassemblent) ou déclarée `chunked = false` n'en a pas, et lui
/// demander ses chunks fait échouer le groupe entier.
fn a_des_chunks(entity_configs: &HashMap<String, crate::config::EntityConfig>, entity_name: &str) -> bool {
    entity_configs
        .get(entity_name)
        .is_some_and(|c| c.has_simple_pipeline() && c.chunked != Some(false))
}

// ─── DeleteRecordNode ──────────────────────────────────────────────────────

/// Batch cascade-delete entities + their chunks from `Vec<DeleteRecord>`.
///
/// For each entity group, deletes the `{entity}_Chunk` rows, then the
/// entities themselves, removes them from node_id_cache and de-indexes them
/// from the FTS handles.
///
/// **Input**: `deletes` — `BatchPayload<DeleteRecord>` (PortType::Deletes)
/// **Output**: `done` — Empty signal
/// **Services**: `conn`, `node_id_cache`, `config`, `entity_configs`,
///              `delete_results`, optionally `fts_handles`, `event_bus`
pub struct DeleteRecordNode {
    name: String,
    undo_data: Option<serde_json::Value>,
    conn: Option<Arc<dyn DbConnection>>,
    dialect: Option<Arc<dyn crate::dialect::SchemaDialect>>,
}

impl DeleteRecordNode {
    /// Lie une connexion et un dialecte à un nœud frais, pour rejouer `undo`
    /// depuis un checkpoint sans passer par `execute()` (reprise après crash).
    /// Pendant un `execute()`, les deux sont capturés depuis les services.
    pub fn bind_services(
        &mut self,
        conn: Arc<dyn DbConnection>,
        dialect: Arc<dyn crate::dialect::SchemaDialect>,
    ) {
        self.conn = Some(conn);
        self.dialect = Some(dialect);
    }

    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), undo_data: None, conn: None, dialect: None }
    }
}


impl Node for DeleteRecordNode {
    fn name(&self) -> &str { &self.name }
    fn node_type(&self) -> &'static str { "DeleteRecordNode" }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::DeleteRecordNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::DeleteRecordNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let items: Vec<DeleteRecord> = ctx.take_input("deletes")
            .and_then(|pv| pv.take::<BatchPayload>())
            .and_then(|bp| bp.take::<DeleteRecord>())
            .ok_or("DeleteRecordNode: missing 'deletes' input")?;

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn").cloned()
            .ok_or("DeleteRecordNode: 'conn' service not registered")?;
        let node_id_cache = ctx.service::<Arc<RwLock<NodeIdCache>>>("node_id_cache").cloned()
            .ok_or("DeleteRecordNode: 'node_id_cache' service not registered")?;
        let fts_handles = ctx
            .service::<HashMap<String, Arc<lucivy_core::sharded_handle::ShardedHandle>>>("fts_handles")
            .cloned();
        let config = ctx.service::<CatalogConfig>("config").cloned()
            .ok_or("DeleteRecordNode: 'config' service not registered")?;
        let entity_configs = ctx.service::<HashMap<String, crate::config::EntityConfig>>("entity_configs").cloned()
            .ok_or("DeleteRecordNode: 'entity_configs' service not registered")?;
        let results_svc = ctx.service::<Arc<Mutex<Vec<DeleteResult>>>>("delete_results").cloned()
            .ok_or("DeleteRecordNode: 'delete_results' service not registered")?;
        let event_bus = ctx.service::<Arc<EventBus>>("event_bus").cloned();

        // Group by entity_name
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        for rec in &items {
            groups.entry(rec.entity_name.clone()).or_default().push(rec.uuid.clone());
        }

        let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect").cloned()
            .ok_or("DeleteRecordNode: 'dialect' service not registered")?;

        ctx.metric("items", items.len() as f64);
        ctx.metric("groups", groups.len() as f64);

        let mut all_results: Vec<DeleteResult> = Vec::new();
        let mut undo_groups: HashMap<String, Vec<BTreeMap<String, CypherValue>>> = HashMap::new();

        for (entity_name, uuids) in &groups {
            if !config.entities.contains_key(entity_name) {
                // Un groupe entier disparaissait ici : ni résultat, ni mot.
                let cause = format!("entité « {entity_name} » absente de la configuration");
                consigner_l_echec(
                    ctx, "DeleteRecordNode", entity_name, uuids.len(), Disponibilites::TOUT, cause.clone(),
                );
                for uuid in uuids {
                    all_results.push(DeleteResult {
                        uuid: uuid.clone(),
                        entity: entity_name.clone(),
                        chunks_deleted: 0,
                        relations_deleted: 0,
                        echec: Some(cause.clone()),
                    });
                }
                continue;
            }
            let derivee = config.entities.get(entity_name).is_some_and(|d| d.derived_from.is_some());
            // **Le groupe en fermeture** : un `?` dedans n'abandonne que ce
            // groupe, qui se consigne ; les autres entités du lot passent.
            let issue: Result<(), String> = (|| {
            let mut per_uuid_chunks: HashMap<String, usize> = HashMap::new();

            // Cascade-delete the entity's chunks
            if a_des_chunks(&entity_configs, entity_name) {
                let chunk_table = format!("{entity_name}_Chunk");
                let uuid_list = CypherValue::List(
                    uuids.iter().map(|u| CypherValue::String(u.clone())).collect(),
                );
                let del_chunks = dialect.batch_cascade_delete_returning_count(&chunk_table, "_parent_uuid");
                let result = conn
                    .execute_with_params(
                        &del_chunks,
                        &[QueryParam { name: "uuids".into(), value: uuid_list }],
                    )
                    .map_err(|e| e.to_string())?;
                for row in &result.rows {
                    if let (Some(uuid), Some(cnt)) = (
                        row.get(0).and_then(|v| v.as_str()),
                        row.get(1).and_then(|v| v.as_i64()),
                    ) {
                        *per_uuid_chunks.entry(uuid.to_string()).or_default() += cnt as usize;
                    }
                }
            }

            // Read full entity data before delete (for undo + existence check)
            let uuid_list = CypherValue::List(
                uuids.iter().map(|u| CypherValue::String(u.clone())).collect(),
            );
            let existing: HashSet<String> = {
                let read = conn.execute_with_params(
                    &dialect.select_entity_all_by_uuids(entity_name),
                    &[QueryParam { name: "uuids".into(), value: uuid_list.clone() }],
                ).map_err(|e| e.to_string())?;
                let mut found = HashSet::new();
                for row in &read.rows {
                    if let Some(CypherValue::Map(props)) = row.first() {
                        if let Some(uuid) = props.get("_uuid").and_then(|v| v.as_str()) {
                            found.insert(uuid.to_string());
                            // Strip internal rag3db properties (_id, _label)
                            let clean: BTreeMap<String, CypherValue> = props.iter()
                                .filter(|(k, _)| k.as_str() != "_id" && k.as_str() != "_label")
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect();
                            // **Une ligne dérivée ne se restaure pas** : elle se
                            // re-rend depuis sa racine à la prochaine écriture de
                            // celle-ci. La restaurer figerait un `_render_hash`
                            // sans chunks ni index derrière (e2e_undo, 18 sept.).
                            if !derivee {
                                undo_groups.entry(entity_name.clone())
                                    .or_default()
                                    .push(clean);
                            }
                        }
                    }
                }
                found
            };

            // Warn for nonexistent UUIDs
            for uuid in uuids.iter().filter(|u| !existing.contains(u.as_str())) {
                ctx.warn(&format!("{entity_name} with uuid '{uuid}' not found, skipping"));
            }

            // Delete entities themselves
            let del_entities = dialect.batch_cascade_delete(entity_name);
            conn.execute_with_params(
                &del_entities,
                &[QueryParam { name: "uuids".into(), value: uuid_list }],
            )
            .map_err(|e| e.to_string())?;

            // Retrait du cache uuid→offset, et désindexation FTS au passage :
            // `remove` rend l'InternalNodeId, donc l'offset, qui est exactement
            // la clé sous laquelle le document a été indexé.
            //
            // Les hooks C++ faisaient ça implicitement sur les mutations Cypher ;
            // en Rust direct, c'est à nous de le faire explicitement, sinon
            // l'index garde des documents fantômes qui ressortiraient en
            // recherche avec des offsets ne résolvant plus vers rien.
            let mut removed_offsets: Vec<u64> = Vec::new();
            if let Ok(mut cache) = node_id_cache.write() {
                for uuid in uuids {
                    if let Some(id) = cache.remove(uuid.as_str()) {
                        removed_offsets.push(id.offset);
                    }
                }
            }
            if let Some(ref handles) = fts_handles {
                if let Some(handle) = handles.get(entity_name) {
                    for offset in &removed_offsets {
                        handle
                            .delete_by_node_id(*offset)
                            .map_err(|e| format!("désindexation FTS de {entity_name}: {e}"))?;
                    }
                }
            }

            // Build DeleteResults + emit EntityDeleted events
            for uuid in uuids {
                let chunks_deleted = per_uuid_chunks.get(uuid).copied().unwrap_or(0);
                all_results.push(DeleteResult {
                    uuid: uuid.clone(),
                    entity: entity_name.clone(),
                    chunks_deleted,
                    relations_deleted: 0,
                    echec: None,
                });
                if existing.contains(uuid.as_str()) {
                    if let Some(ref bus) = event_bus {
                        bus.emit(CatalogEvent::EntityDeleted {
                            entity: entity_name.clone(),
                            uuid: uuid.clone(),
                            chunks_deleted,
                        });
                    }
                }
            }
            Ok(())
            })();
            if let Err(cause) = issue {
                consigner_l_echec(
                    ctx, "DeleteRecordNode", entity_name, uuids.len(), Disponibilites::TOUT, cause.clone(),
                );
                for uuid in uuids {
                    all_results.push(DeleteResult {
                        uuid: uuid.clone(),
                        entity: entity_name.clone(),
                        chunks_deleted: 0,
                        relations_deleted: 0,
                        echec: Some(cause.clone()),
                    });
                }
            }
        }

        // Push results to shared service
        results_svc.lock().map_err(|e| format!("delete_results lock: {e}"))?
            .extend(all_results);

        // Capture undo data
        if !undo_groups.is_empty() {
            self.undo_data = Some(
                serde_json::to_value(&undo_groups)
                    .map_err(|e| format!("DeleteRecordNode: failed to serialize undo data: {e}"))?
            );
        }

        // Store services for undo
        self.conn = Some(conn.clone());
        self.dialect = Some(dialect.clone());

        ctx.trigger("done");
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<Box<dyn Any + Send>> {
        self.undo_data.clone().map(|v| Box::new(v) as Box<dyn Any + Send>)
    }

    fn undo(&mut self, undo_ctx: Box<dyn Any + Send>) -> Result<(), String> {
        let undo_ctx = *undo_ctx.downcast::<serde_json::Value>().map_err(|_| "bad undo ctx")?;
        let conn = self.conn.as_ref()
            .ok_or("DeleteRecordNode undo: 'conn' not stored")?;
        let dialect = self.dialect.as_ref()
            .ok_or("DeleteRecordNode undo: 'dialect' not stored")?;

        let groups: HashMap<String, Vec<BTreeMap<String, CypherValue>>> =
            serde_json::from_value(undo_ctx)
                .map_err(|e| format!("DeleteRecordNode undo: failed to deserialize: {e}"))?;

        for (entity_name, items) in &groups {
            if items.is_empty() { continue; }

            let columns: Vec<&str> = items[0].keys().map(|k| k.as_str()).collect();
            let cypher = dialect.batch_upsert(entity_name, &columns);

            let items_param = CypherValue::List(
                items.iter().map(|m| CypherValue::Map(m.clone())).collect()
            );
            conn.execute_with_params(
                &cypher,
                &[QueryParam { name: "items".into(), value: items_param }],
            ).map_err(|e| format!("DeleteRecordNode undo failed: {e}"))?;
        }
        Ok(())
    }
}

// ─── UpdateRecordNode ──────────────────────────────────────────────────────

/// Batch field update + change detection from `Vec<UpdateRecord>`.
///
/// Groups by (entity_name, sorted_field_keys), batch-reads old hashes,
/// detects content changes, batch SETs all fields. For changed items,
/// reads full data and builds EntityRecords → `rechunk_entities`.
///
/// **Input**: `updates` — `BatchPayload<UpdateRecord>` (PortType::Updates)
/// **Output**: `done` — Empty signal, `rechunk_entities` — EntityRecords to re-chunk
/// **Services**: `conn`, `node_id_cache`, `config`, `entity_configs`, `dialect`,
///              `update_results`, optionally `fts_handles`
pub struct UpdateRecordNode {
    name: String,
    undo_data: Option<serde_json::Value>,
    conn: Option<Arc<dyn DbConnection>>,
    dialect: Option<Arc<dyn crate::dialect::SchemaDialect>>,
    /// Capturés à l'exécution (ou liés par `bind_fts`) : l'undo doit
    /// ré-indexer les colonnes restaurées, sinon la recherche renvoie encore
    /// le contenu annulé.
    fts_handles: Option<HashMap<String, Arc<lucivy_core::sharded_handle::ShardedHandle>>>,
    node_id_cache: Option<Arc<RwLock<NodeIdCache>>>,
    entity_configs: Option<HashMap<String, crate::config::EntityConfig>>,
}

impl UpdateRecordNode {
    /// Lie une connexion et un dialecte à un nœud frais, pour rejouer `undo`
    /// depuis un checkpoint sans passer par `execute()` (reprise après crash).
    /// Pendant un `execute()`, les deux sont capturés depuis les services.
    pub fn bind_services(
        &mut self,
        conn: Arc<dyn DbConnection>,
        dialect: Arc<dyn crate::dialect::SchemaDialect>,
    ) {
        self.conn = Some(conn);
        self.dialect = Some(dialect);
    }

    /// Lie les index FTS et le cache d'identifiants à un nœud frais (reprise),
    /// pour que `undo` ré-indexe les lignes restaurées.
    pub fn bind_fts(
        &mut self,
        fts_handles: HashMap<String, Arc<lucivy_core::sharded_handle::ShardedHandle>>,
        node_id_cache: Arc<RwLock<NodeIdCache>>,
        entity_configs: HashMap<String, crate::config::EntityConfig>,
    ) {
        self.fts_handles = Some(fts_handles);
        self.node_id_cache = Some(node_id_cache);
        self.entity_configs = Some(entity_configs);
    }

    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), undo_data: None, conn: None, dialect: None, fts_handles: None, node_id_cache: None, entity_configs: None }
    }
}


impl Node for UpdateRecordNode {
    fn name(&self) -> &str { &self.name }
    fn node_type(&self) -> &'static str { "UpdateRecordNode" }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::UpdateRecordNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::UpdateRecordNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let raw_items: Vec<UpdateRecord> = ctx.take_input("updates")
            .and_then(|pv| pv.take::<BatchPayload>())
            .and_then(|bp| bp.take::<UpdateRecord>())
            .ok_or("UpdateRecordNode: missing 'updates' input")?;

        // ─── Merge duplicate updates for same (entity_name, uuid) ───
        let items: Vec<UpdateRecord> = {
            let mut seen: HashMap<(String, String), usize> = HashMap::new();
            let mut merged: Vec<UpdateRecord> = Vec::new();
            let mut merge_count = 0usize;
            for update in raw_items {
                let key = (update.entity_name.clone(), update.uuid.clone());
                if let Some(&idx) = seen.get(&key) {
                    merged[idx].data.extend(update.data);
                    merged[idx].new_content_hash = String::new(); // sentinel: force re-chunk
                    merge_count += 1;
                } else {
                    seen.insert(key, merged.len());
                    merged.push(update);
                }
            }
            if merge_count > 0 {
                ctx.info(&format!("merged {merge_count} duplicate update(s) into existing records"));
            }
            merged
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn").cloned()
            .ok_or("UpdateRecordNode: 'conn' service not registered")?;
        let node_id_cache = ctx.service::<Arc<RwLock<NodeIdCache>>>("node_id_cache").cloned()
            .ok_or("UpdateRecordNode: 'node_id_cache' service not registered")?;
        let fts_handles = ctx
            .service::<HashMap<String, Arc<lucivy_core::sharded_handle::ShardedHandle>>>("fts_handles")
            .cloned();
        let config = ctx.service::<CatalogConfig>("config").cloned()
            .ok_or("UpdateRecordNode: 'config' service not registered")?;
        let entity_configs = ctx.service::<HashMap<String, crate::config::EntityConfig>>("entity_configs").cloned()
            .ok_or("UpdateRecordNode: 'entity_configs' service not registered")?;
        let results_svc = ctx.service::<Arc<Mutex<Vec<UpdateResult>>>>("update_results").cloned()
            .ok_or("UpdateRecordNode: 'update_results' service not registered")?;
        
        // Group by entity_name
        let mut entity_groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, rec) in items.iter().enumerate() {
            entity_groups.entry(rec.entity_name.clone()).or_default().push(i);
        }

        ctx.metric("items", items.len() as f64);
        ctx.metric("entity_groups", entity_groups.len() as f64);

        let mut all_results: Vec<UpdateResult> = Vec::new();
        let mut all_rechunk_entities: Vec<EntityRecord> = Vec::new();
        let mut undo_snapshots: HashMap<String, Vec<BTreeMap<String, CypherValue>>> = HashMap::new();

        for (entity_name, entity_indices) in &entity_groups {
            if !config.entities.contains_key(entity_name) {
                // Un groupe entier disparaissait ici : ni résultat, ni mot, et
                // `drainer` le comptait quand même dans `processed`.
                let cause = format!("entité « {entity_name} » absente de la configuration");
                consigner_l_echec(
                    ctx, "UpdateRecordNode", entity_name, entity_indices.len(), Disponibilites::TOUT,
                    cause.clone(),
                );
                for &i in entity_indices.iter() {
                    all_results.push(UpdateResult {
                        uuid: items[i].uuid.clone(),
                        entity: entity_name.clone(),
                        status: UpdateStatus::Failed(cause.clone()),
                        reembedded: false,
                        chunks_created: 0,
                        chunks_deleted: 0,
                    });
                }
                continue;
            }
            // Les lignes refusées une à une (transition non déclarée) : elles
            // sortent du groupe, se comptent, et les autres passent.
            // La cause voyage avec l'indice refusé : la publier dans le canal
            // d'échecs et la jeter ici obligeait l'appelant à réapparier des
            // chaînes libres avec des lignes, ce qui est faux dès deux entités.
            let mut rejetes: HashMap<usize, String> = HashMap::new();
            // **Le groupe en fermeture** : un `?` dedans n'abandonne que ce
            // groupe, qui se consigne ; les autres entités du lot passent.
            let issue: Result<(), String> = (|| {

            // 1. Batch-read old entity data (for hashes + undo)
            let uuid_list = CypherValue::List(
                entity_indices.iter()
                    .map(|&i| CypherValue::String(items[i].uuid.clone()))
                    .collect(),
            );
            let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect").cloned()
                .ok_or("UpdateRecordNode: 'dialect' service not registered")?;
            let old_read_query = dialect.select_entity_all_by_uuids(entity_name);
            let old_result = conn
                .execute_with_params(
                    &old_read_query,
                    &[QueryParam { name: "uuids".into(), value: uuid_list }],
                )
                .map_err(|e| e.to_string())?;

            let mut old_hashes: HashMap<String, String> = HashMap::new();
            // L'état d'avant, quand l'entité en déclare un. C'est **le seul
            // endroit** où on l'a : vérifier plus tôt demanderait une lecture
            // de plus et mentirait sur les mises à jour du même lot.
            let lifecycle = entity_configs.get(entity_name).and_then(|c| c.lifecycle.as_ref());
            let mut old_states: HashMap<String, String> = HashMap::new();
            for row in &old_result.rows {
                if let Some(CypherValue::Map(props)) = row.first() {
                    if let (Some(uuid), Some(hash)) = (
                        props.get("_uuid").and_then(|v| v.as_str()),
                        props.get("_content_hash").and_then(|v| v.as_str()),
                    ) {
                        if let Some(lc) = lifecycle {
                            if let Some(etat) = props.get(&lc.field).and_then(|v| v.as_str()) {
                                old_states.insert(uuid.to_string(), etat.to_string());
                            }
                        }
                        old_hashes.insert(uuid.to_string(), hash.to_string());
                        // Strip internal rag3db properties (_id, _label)
                        let clean: BTreeMap<String, CypherValue> = props.iter()
                            .filter(|(k, _)| k.as_str() != "_id" && k.as_str() != "_label")
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        undo_snapshots.entry(entity_name.clone())
                            .or_default()
                            .push(clean);
                    }
                }
            }

            // Warn for UUIDs not found in DB
            for &i in entity_indices {
                if !old_hashes.contains_key(&items[i].uuid) {
                    ctx.warn(&format!(
                        "{entity_name} with uuid '{}' not found, update is a no-op",
                        items[i].uuid
                    ));
                }
            }

            // 1 bis. **Une transition non déclarée ne passe pas.**
            //
            // Jusqu'ici `Lifecycle` était vérifié à la déclaration et
            // n'empêchait rien à l'écriture — une déclaration vérifiée mais
            // non appliquée est un piège si on l'oublie.
            //
            // Trois cas laissés passer, chacun pour une raison :
            //
            // - la mise à jour ne touche pas au champ d'état : ce n'est pas
            //   une transition ;
            // - l'état ne change pas : écrire `draft` sur `draft` n'est pas
            //   un passage ;
            // - **on ne connaît pas l'état d'avant** — ligne absente, ou
            //   champ vide parce que la machine a été déclarée après coup.
            //   Refuser bloquerait toutes les lignes existantes le jour où on
            //   ajoute un `Lifecycle` à une entité qui tourne. On le dit, et
            //   on laisse passer.
            if let Some(lc) = lifecycle {
                for &i in entity_indices {
                    let rec = &items[i];
                    let Some(vers) = rec.data.get(&lc.field).and_then(|v| v.as_str()) else {
                        continue;
                    };
                    let Some(depuis) = old_states.get(&rec.uuid) else {
                        if old_hashes.contains_key(&rec.uuid) {
                            ctx.warn(&format!(
                                "{entity_name} '{}' : état d'avant inconnu, transition vers '{vers}' non vérifiée",
                                rec.uuid
                            ));
                        }
                        continue;
                    };
                    if depuis == vers {
                        continue;
                    }
                    if lc.allows(depuis, vers).is_none() {
                        // Dire ce qui **est** permis : sans ça, l'erreur est un
                        // mur, et l'appelant doit aller relire la déclaration.
                        let permis: Vec<String> = lc
                            .next_from(depuis)
                            .iter()
                            .map(|t| format!("{} → {}", t.name, t.to))
                            .collect();
                        let permis = if permis.is_empty() {
                            format!("'{depuis}' est un état terminal")
                            } else {
                            format!("depuis '{depuis}' : {}", permis.join(", "))
                        };
                        // Une ligne refusée ne fait plus tomber l'ingestion :
                        // elle se compte, se dit, et les autres passent. La
                        // cause est dite **une fois** et sert aux deux publics :
                        // le canal d'échecs pour le compte rendu, le résultat
                        // de la ligne pour qui décide quoi reprendre.
                        let cause = format!(
                            "{entity_name} '{}' : transition '{depuis}' → '{vers}' non déclarée ({permis})",
                            rec.uuid
                        );
                        consigner_l_echec(
                            ctx, "UpdateRecordNode", entity_name, 1, Disponibilites::TOUT,
                            cause.clone(),
                        );
                        rejetes.insert(i, cause);
                    }
                }
            }

            // Ce qui reste du groupe une fois les refus retirés.
            let retenus: Vec<usize> =
                entity_indices.iter().copied().filter(|i| !rejetes.contains_key(i)).collect();
            let entity_indices = &retenus;

            // 2. Detect content changes
            let changed: Vec<bool> = entity_indices.iter()
                .map(|&i| {
                    let rec = &items[i];
                    old_hashes.get(&rec.uuid)
                        .map_or(true, |old| old != &rec.new_content_hash)
                })
                .collect();

            // 3. Batch SET via UNWIND — group by sorted field keys
            let mut set_groups: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
            for &i in entity_indices {
                let mut keys: Vec<String> = items[i].data.keys().cloned().collect();
                keys.sort();
                set_groups.entry(keys).or_default().push(i);
            }

            for (field_keys, indices) in &set_groups {
                let items_param = CypherValue::List(
                    indices.iter().map(|&i| {
                        let rec = &items[i];
                        let mut m = BTreeMap::new();
                        m.insert("_uuid".to_string(), CypherValue::String(rec.uuid.clone()));
                        m.insert("_content_hash".to_string(), CypherValue::String(rec.new_content_hash.clone()));
                        for (k, v) in &rec.data {
                            m.insert(k.clone(), v.clone());
                        }
                        CypherValue::Map(m)
                    }).collect(),
                );

                let mut update_cols: Vec<&str> = field_keys.iter().map(|s| s.as_str()).collect();
                update_cols.push("_content_hash");
                let set_cypher = dialect.batch_update_fields(entity_name, &update_cols);
                conn.execute_with_params(
                    &set_cypher,
                    &[QueryParam { name: "items".into(), value: items_param }],
                )
                .map_err(|e| e.to_string())?;

                // Ré-indexation FTS des entités modifiées.
                //
                // On RELIT la ligne entière plutôt que de réutiliser `rec.data` :
                // celui-ci ne porte que les champs modifiés, or `add_document`
                // n'est pas un merge — ré-indexer un sous-ensemble ferait
                // disparaître silencieusement les champs texte inchangés.
                if let Some(ref handles) = fts_handles {
                    if let Some(handle) = handles.get(entity_name) {
                        // Si aucun champ modifié n'est indexé, l'index est déjà à jour.
                        let touched = crate::fts_handle::indexed_text_fields(handle, field_keys);
                        if !touched.is_empty() {
                            // Candidats = TOUS les champs de l'entité, pas seulement les
                            // modifiés : ré-indexer avec le sous-ensemble modifié effaçait
                            // silencieusement les autres champs texte du document.
                            let schema_fields =
                                entity_indexed_fields(handle, &entity_configs, entity_name, field_keys);
                            let uuids: Vec<CypherValue> = indices
                                .iter()
                                .map(|&i| CypherValue::String(items[i].uuid.clone()))
                                .collect();
                            // Les champs sont posés ; si l'index ne suit pas,
                            // c'est le plein texte qui est perdu, pas la
                            // mise à jour.
                            if let Err(e) = reindex_fts_rows(
                                conn.as_ref(),
                                dialect.as_ref(),
                                handle,
                                &node_id_cache,
                                entity_name,
                                uuids,
                                &schema_fields,
                            ) {
                                consigner_l_echec(
                                    ctx, "UpdateRecordNode", entity_name, 0,
                                    Disponibilites::PLEIN_TEXTE,
                                    format!("réindexation plein texte : {e}"),
                                );
                            }
                        }
                    }
                }
            }

            // 4. Handle content changes
            let changed_uuids: HashSet<&str> = entity_indices.iter()
                .zip(changed.iter())
                .filter_map(|(&i, &c)| if c { Some(items[i].uuid.as_str()) } else { None })
                .collect();

            if !changed_uuids.is_empty() {
                // Read full data → build EntityRecords for rechunking
                if a_des_chunks(&entity_configs, entity_name) {
                    let uuid_param = CypherValue::List(
                        changed_uuids.iter().map(|u| CypherValue::String(u.to_string())).collect(),
                    );
                    let read_cypher = dialect.select_entity_all_by_uuids(entity_name);
                    let read_result = conn
                        .execute_with_params(
                            &read_cypher,
                            &[QueryParam { name: "uuids".into(), value: uuid_param }],
                        )
                        .map_err(|e| e.to_string())?;

                    for row in &read_result.rows {
                        if let Some(first) = row.first() {
                            let props = match first {
                                CypherValue::Map(m) => m.clone(),
                                _ => continue,
                            };
                            let uuid = match props.get("_uuid").and_then(|v| v.as_str()) {
                                Some(u) => u.to_string(),
                                None => continue,
                            };
                            all_rechunk_entities.push(EntityRecord {
                                entity_name: entity_name.clone(),
                                data: props,
                                entity_ref: EntityRef::pre_resolved(entity_name, &uuid, &uuid),
                                resolver: None,
                                vectors: None,
                            });
                        }
                    }
                }
            }

            // 5. Build UpdateResults + emit EntityUpdated events
            for (idx_in_group, &i) in entity_indices.iter().enumerate() {
                let content_changed = changed[idx_in_group];
                let uuid = &items[i].uuid;
                let found = old_hashes.contains_key(uuid);
                let reembedded = found && content_changed && a_des_chunks(&entity_configs, entity_name);
                let status = if !found {
                    UpdateStatus::Updated // no old hash → considered "changed" (no-op in DB)
                } else if content_changed {
                    UpdateStatus::Updated
                } else {
                    UpdateStatus::Unchanged
                };
                // Les deux comptes restent à zéro **ici** et c'est exact : le
                // rechunkage a lieu en aval (`rechunk_delete`, `rechunk_chunk`),
                // ce nœud ne peut pas les connaître. `Catalog::drain` les
                // recolle avant de rendre le résultat, et c'est lui qui émet
                // l'événement — avec les vrais nombres.
                //
                // Avant, ces zéros étaient rendus tels quels et l'événement
                // partait avec : deux champs présentés comme des mesures qui
                // n'en étaient pas.
                all_results.push(UpdateResult {
                    uuid: uuid.clone(),
                    entity: entity_name.clone(),
                    status,
                    reembedded,
                    chunks_created: 0,
                    chunks_deleted: 0,
                });
            }
            Ok(())
            })();

            let en_echec = |all_results: &mut Vec<UpdateResult>, i: usize, cause: String| {
                all_results.push(UpdateResult {
                    uuid: items[i].uuid.clone(),
                    entity: entity_name.clone(),
                    status: UpdateStatus::Failed(cause),
                    reembedded: false,
                    chunks_created: 0,
                    chunks_deleted: 0,
                });
            };
            for (&i, cause) in rejetes.iter() {
                en_echec(&mut all_results, i, cause.clone());
            }
            if let Err(cause) = issue {
                // Le groupe s'est arrêté en route. Ce qui a pu être posé avant
                // l'arrêt l'est ; on le compte quand même comme échoué — dire
                // moins que fait est le sens sûr, l'inverse est le mensonge.
                let restants: Vec<usize> =
                    entity_indices.iter().copied().filter(|i| !rejetes.contains_key(i)).collect();
                consigner_l_echec(
                    ctx, "UpdateRecordNode", entity_name, restants.len(), Disponibilites::TOUT,
                    cause.clone(),
                );
                for i in restants {
                    en_echec(&mut all_results, i, cause.clone());
                }
            }
        }

        // Push results to shared service
        results_svc.lock().map_err(|e| format!("update_results lock: {e}"))?
            .extend(all_results);

        // Capture undo data (old entity snapshots)
        if !undo_snapshots.is_empty() {
            self.undo_data = Some(
                serde_json::to_value(&undo_snapshots)
                    .map_err(|e| format!("UpdateRecordNode: failed to serialize undo data: {e}"))?
            );
        }

        // Store services for undo
        self.conn = Some(conn.clone());
        self.fts_handles = fts_handles.clone();
        self.node_id_cache = Some(node_id_cache.clone());
        self.entity_configs = Some(entity_configs.clone());
        let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect").cloned()
            .ok_or("UpdateRecordNode: 'dialect' service not registered")?;
        self.dialect = Some(dialect.clone());

        ctx.metric("rechunk_entities", all_rechunk_entities.len() as f64);

        // Always emit rechunk_entities (even empty) so downstream rechunk pipeline
        // receives its input and doesn't deadlock.
        ctx.set_output("rechunk_entities", PortValue::new(
            BatchPayload::new(PortType::Entities, all_rechunk_entities),
        ));
        ctx.trigger("done");
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<Box<dyn Any + Send>> {
        self.undo_data.clone().map(|v| Box::new(v) as Box<dyn Any + Send>)
    }

    fn undo(&mut self, undo_ctx: Box<dyn Any + Send>) -> Result<(), String> {
        let undo_ctx = *undo_ctx.downcast::<serde_json::Value>().map_err(|_| "bad undo ctx")?;
        let conn = self.conn.as_ref()
            .ok_or("UpdateRecordNode undo: 'conn' not stored")?;
        let dialect = self.dialect.as_ref()
            .ok_or("UpdateRecordNode undo: 'dialect' not stored")?;

        let groups: HashMap<String, Vec<BTreeMap<String, CypherValue>>> =
            serde_json::from_value(undo_ctx)
                .map_err(|e| format!("UpdateRecordNode undo: failed to deserialize: {e}"))?;

        for (entity_name, items) in &groups {
            if items.is_empty() { continue; }

            let columns: Vec<&str> = items[0].keys().map(|k| k.as_str()).collect();
            let other_cols: Vec<&str> = columns.iter()
                .filter(|c| **c != "_uuid")
                .copied()
                .collect();
            if other_cols.is_empty() { continue; }

            let cypher = dialect.batch_update_fields(entity_name, &other_cols);

            let items_param = CypherValue::List(
                items.iter().map(|m| CypherValue::Map(m.clone())).collect()
            );
            conn.execute_with_params(
                &cypher,
                &[QueryParam { name: "items".into(), value: items_param }],
            ).map_err(|e| format!("UpdateRecordNode undo failed: {e}"))?;

            // L'index FTS doit suivre les colonnes restaurées.
            if let (Some(handles), Some(cache)) = (self.fts_handles.as_ref(), self.node_id_cache.as_ref()) {
                if let Some(handle) = handles.get(entity_name) {
                    let restored: Vec<String> = other_cols.iter().map(|c| c.to_string()).collect();
                    let touched = crate::fts_handle::indexed_text_fields(handle, &restored);
                    if !touched.is_empty() {
                        let schema_fields = match self.entity_configs.as_ref() {
                            Some(cfgs) => entity_indexed_fields(handle, cfgs, entity_name, &restored),
                            None => touched,
                        };
                        let uuids: Vec<CypherValue> = items
                            .iter()
                            .filter_map(|m| m.get("_uuid").cloned())
                            .collect();
                        reindex_fts_rows(
                            conn.as_ref(),
                            dialect.as_ref(),
                            handle,
                            cache,
                            entity_name,
                            uuids,
                            &schema_fields,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Les champs indexés d'une entité, en partant de sa configuration complète
/// (repli sur `fallback` si l'entité n'est pas configurée).
fn entity_indexed_fields(
    handle: &lucivy_core::sharded_handle::ShardedHandle,
    entity_configs: &HashMap<String, crate::config::EntityConfig>,
    entity_name: &str,
    fallback: &[String],
) -> Vec<String> {
    let mut all: Vec<String> = match entity_configs.get(entity_name) {
        Some(cfg) => cfg.fields.keys().cloned().collect(),
        None => fallback.to_vec(),
    };
    all.sort();
    crate::fts_handle::indexed_text_fields(handle, &all)
}

/// Ré-indexe dans lucivy les documents `uuids` d'`entity_name` en relisant la
/// ligne entière (`add_document` n'est pas un merge : ré-indexer un sous-ensemble
/// ferait disparaître les champs texte inchangés). Partagé par `execute()` et
/// `undo()` d'`UpdateRecordNode`.
fn reindex_fts_rows(
    conn: &dyn DbConnection,
    dialect: &dyn crate::dialect::SchemaDialect,
    handle: &lucivy_core::sharded_handle::ShardedHandle,
    node_id_cache: &Arc<RwLock<NodeIdCache>>,
    entity_name: &str,
    uuids: Vec<CypherValue>,
    schema_fields: &[String],
) -> Result<(), String> {
    let mut cols: Vec<&str> = vec!["_uuid"];
    cols.extend(schema_fields.iter().map(|s| s.as_str()));
    let sel = dialect.select_by_uuids(entity_name, &cols);
    let rows = conn
        .execute_with_params(
            &sel,
            &[QueryParam { name: "uuids".into(), value: CypherValue::List(uuids) }],
        )
        .map_err(|e| e.to_string())?;

    for row in &rows.rows {
        let Some(uuid) = row.first().and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(offset) = node_id_cache
            .read()
            .ok()
            .and_then(|c| c.get(uuid))
            .map(|id| id.offset)
        else {
            continue;
        };
        let values: Vec<(String, String)> = schema_fields
            .iter()
            .enumerate()
            .filter_map(|(i, name)| {
                row.get(i + 1)
                    .and_then(|v| v.as_str())
                    .map(|s| (name.clone(), s.to_string()))
            })
            .collect();
        crate::fts_handle::reindex_document(handle, &values, offset)
            .map_err(|e| format!("ré-indexation FTS de {entity_name}: {e}"))?;
    }
    Ok(())
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::tables_sans_index_plein_texte as sans_index;
    use std::collections::HashMap;

    fn ouvertes(noms: &[&str]) -> HashMap<String, ()> {
        noms.iter().map(|n| ((*n).to_string(), ())).collect()
    }

    fn veut(noms: &[&str]) -> Vec<String> {
        noms.iter().map(|n| (*n).to_string()).collect()
    }

    /// Sur le chemin natif, l'index vit avec les données : il n'y a aucun
    /// handle à ouvrir, donc jamais d'alarme. Sans cette clause, chaque
    /// ingestion native crierait au loup.
    #[test]
    fn le_chemin_natif_ne_crie_jamais() {
        assert!(sans_index(true, veut(&["Doc", "Product"]), &ouvertes(&[])).is_empty());
    }

    /// Le cas qu'on veut attraper : une entité qui réclame le plein texte et
    /// qu'on écrit sans index ouvert. La recherche rendra zéro, et c'était le
    /// seul indice.
    #[test]
    fn une_table_reclamee_sans_handle_est_nommee() {
        let vus = sans_index(false, veut(&["Doc", "Product"]), &ouvertes(&["Product"]));
        assert_eq!(vus, vec!["Doc".to_string()]);
    }

    /// Une alarme par table, pas par ligne : les groupes d'un même lot
    /// répètent le nom autant de fois qu'il y a de jeux de colonnes.
    #[test]
    fn une_alarme_par_table_et_triee() {
        let vus = sans_index(false, veut(&["Zebre", "Doc", "Doc", "Alpha"]), &ouvertes(&[]));
        assert_eq!(vus, vec!["Alpha".to_string(), "Doc".to_string(), "Zebre".to_string()]);
    }

    /// Et rien à dire quand tout est en place — le silence est correct ici.
    #[test]
    fn tout_ouvert_ne_dit_rien() {
        assert!(sans_index(false, veut(&["Doc"]), &ouvertes(&["Doc"])).is_empty());
    }

    use super::*;
    use crate::config::{EntityConfig, SimpleFieldDef, FieldType};
    use crate::chunker::{Chunker, ChunkerConfig};

    fn make_entity_config() -> EntityConfig {
        let mut fields = HashMap::new();
        fields.insert("name".into(), SimpleFieldDef {
            field_type: FieldType::String,
            is_title: true,
            is_content: false,
            ..Default::default()
        });
        fields.insert("description".into(), SimpleFieldDef {
            field_type: FieldType::Text,
            is_title: false,
            is_content: true,
            ..Default::default()
        });
        fields.insert("details".into(), SimpleFieldDef {
            field_type: FieldType::Text,
            is_title: false,
            is_content: true,
            ..Default::default()
        });
        fields.insert("price".into(), SimpleFieldDef {
            field_type: FieldType::Double,
            is_title: false,
            is_content: false,
            ..Default::default()
        });
        EntityConfig {
            fields,
            ..Default::default()
        }
    }

    fn make_entity_configs() -> HashMap<String, EntityConfig> {
        let mut map = HashMap::new();
        map.insert("Product".into(), make_entity_config());
        map
    }

    fn make_chunker_cache(config: &EntityConfig) -> HashMap<ChunkerConfig, Chunker> {
        let key = ChunkerConfig::from(&config.chunking);
        let mut cache = HashMap::new();
        cache.insert(key.clone(), Chunker::new(key));
        cache
    }

    fn make_product_data(name: &str, description: &str, details: &str) -> BTreeMap<String, CypherValue> {
        let mut data = BTreeMap::new();
        data.insert("_uuid".into(), CypherValue::String("test-uuid-123".into()));
        data.insert("name".into(), CypherValue::String(name.into()));
        data.insert("description".into(), CypherValue::String(description.into()));
        data.insert("details".into(), CypherValue::String(details.into()));
        data.insert("price".into(), CypherValue::Float(29.99));
        data
    }

    // ── ChunkRecordNode::compute_chunks tests ──

    #[test]
    fn chunk_simple_entity_produces_chunks() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Red Shoes", "A nice pair of red shoes.", "Made in Italy.");
        let (chunks, links) = ChunkRecordNode::compute_chunks(
            "Product", "test-uuid-123", &entity_ref, &data, &configs, &cache,
        );

        // Should produce chunks for both content fields (description + details)
        assert!(!chunks.is_empty(), "should produce at least one chunk");
        assert_eq!(chunks.len(), links.len(), "each chunk should have a link");
    }

    #[test]
    fn chunk_entity_names_correct() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Shoes", "Description text.", "Details text.");
        let (chunks, links) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        for chunk in &chunks {
            assert_eq!(chunk.entity_name, "Product_Chunk");
        }
        for link in &links {
            assert_eq!(link.rel_name, "Product_CHUNKED_FROM");
        }
    }

    #[test]
    fn chunk_has_title_from_parent() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Red Shoes", "A product description.", "Some details.");
        let (chunks, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        for chunk in &chunks {
            let title = chunk.data.get("_title").and_then(|v| v.as_str()).unwrap();
            assert_eq!(title, "Red Shoes");
        }
    }

    #[test]
    fn chunk_has_embed_hash_empty() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Shoes", "Description.", "Details.");
        let (chunks, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        for chunk in &chunks {
            let embed_hash = chunk.data.get("_embed_hash").and_then(|v| v.as_str()).unwrap();
            assert_eq!(embed_hash, "", "_embed_hash should be empty initially");
        }
    }

    #[test]
    fn chunk_has_text_hash() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Shoes", "Some description text.", "");
        let (chunks, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        for chunk in &chunks {
            let hash = chunk.data.get("_text_hash").and_then(|v| v.as_str()).unwrap();
            assert!(!hash.is_empty(), "_text_hash should not be empty");
        }
    }

    #[test]
    fn chunk_parent_field_set_correctly() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Shoes", "Description text.", "Details text.");
        let (chunks, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        let fields: HashSet<String> = chunks.iter()
            .filter_map(|c| c.data.get("_parent_field").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect();

        // content_fields are sorted: ["description", "details"]
        assert!(fields.contains("description"), "should have description chunks");
        assert!(fields.contains("details"), "should have details chunks");
    }

    #[test]
    fn chunk_content_offset_multi_fields() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        // content_fields sorted = ["description", "details"]
        let desc = "A short description.";
        let details = "Some details here.";
        let data = make_product_data("Shoes", desc, details);
        let (chunks, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        // description chunks should have _content_offset = 0
        let desc_chunks: Vec<_> = chunks.iter()
            .filter(|c| c.data.get("_parent_field").and_then(|v| v.as_str()) == Some("description"))
            .collect();
        for c in &desc_chunks {
            let offset = c.data.get("_content_offset").and_then(|v| v.as_i64()).unwrap();
            assert_eq!(offset, 0, "description chunks should have offset 0");
        }

        // details chunks should have _content_offset = len(description) + 2 ("\n\n")
        let expected_details_offset = desc.len() as i64 + 2;
        let details_chunks: Vec<_> = chunks.iter()
            .filter(|c| c.data.get("_parent_field").and_then(|v| v.as_str()) == Some("details"))
            .collect();
        for c in &details_chunks {
            let offset = c.data.get("_content_offset").and_then(|v| v.as_i64()).unwrap();
            assert_eq!(offset, expected_details_offset,
                "details chunks should have offset = {} + 2 = {}", desc.len(), expected_details_offset);
        }
    }

    #[test]
    fn chunk_unknown_entity_returns_empty() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Unknown");

        let data = make_product_data("X", "text", "text");
        let (chunks, links) = ChunkRecordNode::compute_chunks(
            "Unknown", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        assert!(chunks.is_empty());
        assert!(links.is_empty());
    }

    #[test]
    fn chunk_empty_content_returns_empty() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Shoes", "", "");
        let (chunks, links) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        assert!(chunks.is_empty(), "empty content should produce no chunks");
        assert!(links.is_empty());
    }

    #[test]
    fn chunk_uuid_deterministic() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);

        let data = make_product_data("Shoes", "Description.", "Details.");

        let (entity_ref1, _) = EntityRef::new("Product");
        let (chunks1, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref1, &data, &configs, &cache,
        );

        let (entity_ref2, _) = EntityRef::new("Product");
        let (chunks2, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref2, &data, &configs, &cache,
        );

        assert_eq!(chunks1.len(), chunks2.len());
        for (c1, c2) in chunks1.iter().zip(chunks2.iter()) {
            let uuid1 = c1.data.get("_uuid").and_then(|v| v.as_str()).unwrap();
            let uuid2 = c2.data.get("_uuid").and_then(|v| v.as_str()).unwrap();
            assert_eq!(uuid1, uuid2, "chunk UUIDs should be deterministic");
        }
    }

    #[test]
    fn chunk_has_all_required_fields() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Shoes", "A product description.", "");
        let (chunks, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        assert!(!chunks.is_empty());
        let chunk = &chunks[0];
        let required = [
            "_uuid", "_parent_uuid", "_parent_field", "_text", "_title",
            "_text_hash", "_embed_hash", "_index", "_start_char", "_end_char",
            "_start_line", "_end_line", "_core_start_char", "_core_end_char",
            "_core_start_line", "_core_end_line", "_content_offset",
        ];
        for field in &required {
            assert!(chunk.data.contains_key(*field), "missing field: {field}");
        }
    }

    // ── EmbedNode construction tests ──

    #[test]
    fn embed_node_default_columns() {
        let node = EmbedNode::new("test", search::SearchSignals::HYBRID, 64);
        assert_eq!(node.node_type(), "EmbedNode");
        assert_eq!(node.name(), "test");

        let config = *node.node_config()
            .and_then(|b| b.downcast::<serde_json::Value>().ok())
            .expect("expected serde_json::Value config");
        assert_eq!(config["text_field"], "_text");
        assert_eq!(config["embedding_col"], "embedding");
        assert_eq!(config["sparse_col"], "sparse");
        assert_eq!(config["gpu_batch_size"], 64);
    }

    #[test]
    fn embed_node_custom_columns() {
        let node = EmbedNode::new("e", search::SearchSignals::VECTOR, 128)
            .with_columns("content", "my_emb", "my_sparse");
        let config = *node.node_config()
            .and_then(|b| b.downcast::<serde_json::Value>().ok())
            .expect("expected serde_json::Value config");
        assert_eq!(config["text_field"], "content");
        assert_eq!(config["embedding_col"], "my_emb");
        assert_eq!(config["sparse_col"], "my_sparse");
    }

    #[test]
    fn embed_node_ports() {
        let node = EmbedNode::new("e", search::SearchSignals::HYBRID, 64);
        assert_eq!(node.inputs().len(), 2); // entities + trigger
        assert_eq!(node.outputs().len(), 2); // done + embedded
        assert_eq!(node.inputs()[0].name, "entities");
        assert_eq!(node.outputs()[0].name, "done");
        assert_eq!(node.outputs()[1].name, "embedded");
    }

}
