//! **La vue de lecture** (page 12-octobre-2026-arbre-principal/02, lot L2) :
//! ce qu'une lecture d'outil demande au catalogue, pris sans son mutex.
//!
//! Le mutex du catalogue protégeait trois choses à la fois : la connexion,
//! la forme (entités, relations, dialecte) et la cohérence d'un fichier
//! réindexé. La connexion de lecture (L1) règle la première ; cette vue règle
//! la deuxième : une copie immuable de la forme, publiée par le catalogue
//! quand elle change, qu'une lecture prend par une copie d'`Arc` sous un
//! `RwLock` tenu le temps de cette copie. La troisième est L3.
//!
//! Les fonctions de lecture (usages, impact, callees, liens, l'ordre des
//! verrous) prennent un [`ReadCatalog`] : le `Catalog` lui-même (sous son
//! mutex, comme avant) ou cette vue (sans lui).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, RwLock};

use crate::catalog::{Catalog, CatalogError};
use crate::config::{EntityConfig, EntityDef, RelationDef};
use crate::connection::{CypherValue, DbConnection, QueryParam, QueryResult};
use crate::dataflow::ServiceRegistry;
use crate::dialect::SchemaDialect;
use crate::events::{CatalogEvent, EventBus};
use crate::filter::{FilterCondition, FilterParser};
use crate::search;

/// Le nom du service qui porte la cellule de la vue.
pub const READ_VIEW_SERVICE: &str = "read_view";

/// **Ce qu'une lecture d'outil demande au catalogue.** Neuf méthodes, celles
/// que les nœuds de lecture emploient : rien qui écrive.
pub trait ReadCatalog {
    fn dialect_arc(&self) -> Arc<dyn SchemaDialect>;
    fn execute_read_with_params(&self, cypher: &str, params: &[QueryParam]) -> Result<QueryResult, CatalogError>;
    fn run_read_plan(&self, plan: &rag3weaver_ir::Plan, options: crate::plan::PlanOptions) -> Result<crate::plan::PlanReport, crate::plan::PlanError>;
    fn entity_config(&self, name: &str) -> Option<&EntityConfig>;
    fn get_relation_def(&self, name: &str) -> Option<&RelationDef>;
    fn entity_uuid(&self, entity_name: &str, data: &BTreeMap<String, CypherValue>) -> Result<String, CatalogError>;
    fn read_meta_key(&self, key: &str) -> Result<Option<String>, CatalogError>;
    fn two_step_hops(&self) -> bool;
    fn emit_event(&self, event: CatalogEvent);
    /// Une ligne d'une entité par son uuid (lot L4 : les outils de code).
    fn get(&self, entity_name: &str, uuid: &str) -> Result<Option<BTreeMap<String, CypherValue>>, CatalogError>;
    /// Les lignes d'une entité dont un champ vaut `value`, avec ces champs.
    fn find_by_field(&self, entity_name: &str, field: &str, value: CypherValue, fields: &[&str]) -> Result<Vec<BTreeMap<String, CypherValue>>, CatalogError>;
    /// Les noms des relations déclarées, triés.
    fn relation_names(&self) -> Vec<String>;
}

impl ReadCatalog for Catalog {
    fn dialect_arc(&self) -> Arc<dyn SchemaDialect> {
        Catalog::dialect_arc(self)
    }
    fn execute_read_with_params(&self, cypher: &str, params: &[QueryParam]) -> Result<QueryResult, CatalogError> {
        Catalog::execute_read_with_params(self, cypher, params)
    }
    fn run_read_plan(&self, plan: &rag3weaver_ir::Plan, options: crate::plan::PlanOptions) -> Result<crate::plan::PlanReport, crate::plan::PlanError> {
        Catalog::run_read_plan(self, plan, options)
    }
    fn entity_config(&self, name: &str) -> Option<&EntityConfig> {
        Catalog::entity_config(self, name)
    }
    fn get_relation_def(&self, name: &str) -> Option<&RelationDef> {
        Catalog::get_relation_def(self, name)
    }
    fn entity_uuid(&self, entity_name: &str, data: &BTreeMap<String, CypherValue>) -> Result<String, CatalogError> {
        Catalog::entity_uuid(self, entity_name, data)
    }
    fn read_meta_key(&self, key: &str) -> Result<Option<String>, CatalogError> {
        Catalog::read_meta_key(self, key)
    }
    fn two_step_hops(&self) -> bool {
        Catalog::two_step_hops(self)
    }
    fn emit_event(&self, event: CatalogEvent) {
        Catalog::emit_event(self, event)
    }
    fn get(&self, entity_name: &str, uuid: &str) -> Result<Option<BTreeMap<String, CypherValue>>, CatalogError> {
        Catalog::get(self, entity_name, uuid)
    }
    fn find_by_field(&self, entity_name: &str, field: &str, value: CypherValue, fields: &[&str]) -> Result<Vec<BTreeMap<String, CypherValue>>, CatalogError> {
        Catalog::find_by_field(self, entity_name, field, value, fields)
    }
    fn relation_names(&self) -> Vec<String> {
        Catalog::relation_names(self)
    }
}

/// Le catalogue tenu par son mutex, comme les appelants le passent.
impl ReadCatalog for std::sync::MutexGuard<'_, Catalog> {
    fn dialect_arc(&self) -> Arc<dyn SchemaDialect> {
        Catalog::dialect_arc(self)
    }
    fn execute_read_with_params(&self, cypher: &str, params: &[QueryParam]) -> Result<QueryResult, CatalogError> {
        Catalog::execute_read_with_params(self, cypher, params)
    }
    fn run_read_plan(&self, plan: &rag3weaver_ir::Plan, options: crate::plan::PlanOptions) -> Result<crate::plan::PlanReport, crate::plan::PlanError> {
        Catalog::run_read_plan(self, plan, options)
    }
    fn entity_config(&self, name: &str) -> Option<&EntityConfig> {
        Catalog::entity_config(self, name)
    }
    fn get_relation_def(&self, name: &str) -> Option<&RelationDef> {
        Catalog::get_relation_def(self, name)
    }
    fn entity_uuid(&self, entity_name: &str, data: &BTreeMap<String, CypherValue>) -> Result<String, CatalogError> {
        Catalog::entity_uuid(self, entity_name, data)
    }
    fn read_meta_key(&self, key: &str) -> Result<Option<String>, CatalogError> {
        Catalog::read_meta_key(self, key)
    }
    fn two_step_hops(&self) -> bool {
        Catalog::two_step_hops(self)
    }
    fn emit_event(&self, event: CatalogEvent) {
        Catalog::emit_event(self, event)
    }
    fn get(&self, entity_name: &str, uuid: &str) -> Result<Option<BTreeMap<String, CypherValue>>, CatalogError> {
        Catalog::get(self, entity_name, uuid)
    }
    fn find_by_field(&self, entity_name: &str, field: &str, value: CypherValue, fields: &[&str]) -> Result<Vec<BTreeMap<String, CypherValue>>, CatalogError> {
        Catalog::find_by_field(self, entity_name, field, value, fields)
    }
    fn relation_names(&self) -> Vec<String> {
        Catalog::relation_names(self)
    }
}

/// **La forme d'un catalogue, empruntée** (lot L4, 18 octobre 2026) : ce que
/// les résolutions de la recherche lisent (la cible, les filtres, les ids d'un
/// filtre, les parents d'un résultat), au catalogue sous son mutex comme à la
/// vue de lecture sans lui. Les corps vivent ici une seule fois ; ceux du
/// catalogue délèguent. La connexion est celle de qui l'emprunte : la
/// principale pour le catalogue, la connexion de lecture pour la vue.
pub struct Forme<'a> {
    pub(crate) entity_configs: &'a HashMap<String, EntityConfig>,
    pub(crate) relations: &'a HashMap<String, RelationDef>,
    pub(crate) dialect: &'a Arc<dyn SchemaDialect>,
    pub(crate) conn: &'a Arc<dyn DbConnection>,
    pub(crate) multi_cell: bool,
}

impl Forme<'_> {
    pub fn expliquer_le_silence_d_un_signal(
        &self,
        chunk_table: &str,
        dense_marker: &str,
        signals: search::SearchSignals,
        warnings: &mut Vec<String>,
    ) {
        let compte = |marqueur: &str| -> usize {
            let requete = self.dialect.count_marqueur_manquant(chunk_table, marqueur);
            self.conn
                .execute(&requete)
                .ok()
                .and_then(|r| r.rows.first().and_then(|l| l.first()).and_then(|v| v.as_i64()))
                .unwrap_or(0) as usize
        };
        for (demande, marqueur, nom) in [
            (signals.vector(), dense_marker, "dense"),
            (signals.sparse(), "_sparse_hash", "sparse"),
        ] {
            if !demande {
                continue;
            }
            let dus = compte(marqueur);
            if dus > 0 {
                warnings.push(format!(
                    "le signal {nom} n'a rien rendu, et {dus} chunk(s) de « {chunk_table} » \
                     n'ont pas encore été embarqués : ce n'est pas la preuve que le contenu \
                     n'existe pas. Exigez « {nom} » pour attendre leur embarquement."
                ));
            }
        }
    }

    pub fn resolve_search_target(&self, name: &str) -> Result<search::SearchTarget, CatalogError> {
        // Try simple entity (must have simple pipeline — KB-only entities are not searchable directly)
        if let Some(ec) = self.entity_configs.get(name) {
            if !ec.has_simple_pipeline() {
                // Les dérivées qui la rassemblent, pour une erreur qui aide.
                let mut kb_names: Vec<&String> = self
                    .entity_configs
                    .iter()
                    .filter(|(_, c)| {
                        c.derived.as_ref().is_some_and(|d| {
                            d.from == name
                                || d.gather.iter().any(|r| {
                                    self.relations.get(&r.relation).is_some_and(|rel| match r.direction {
                                        crate::config::GatherDirection::Out => rel.to == name,
                                        crate::config::GatherDirection::In => rel.from == name,
                                    })
                                })
                        })
                    })
                    .map(|(n, _)| n)
                    .collect();
                kb_names.sort();
                let suggestion = if kb_names.is_empty() {
                    String::new()
                } else {
                    format!(" — search on KB {} instead", kb_names.iter().map(|n| format!("'{n}'")).collect::<Vec<_>>().join(", "))
                };
                return Err(CatalogError::SchemaError(
                    format!("Entity '{name}' has no simple pipeline (KB-only){suggestion}")
                ));
            }
            let chunk_table = format!("{name}_Chunk");
            let mut enrich_fields: Vec<String> = ec.content_fields().into_iter().map(|s| s.to_string()).collect();
            if let Some(title) = ec.title_field() {
                let title_owned = title.to_string();
                if !enrich_fields.contains(&title_owned) {
                    enrich_fields.push(title_owned);
                }
            }
            if let Some(extra) = &ec.return_fields {
                for f in extra {
                    if !enrich_fields.contains(f) {
                        enrich_fields.push(f.clone());
                    }
                }
            }
            enrich_fields.push("_content_hash".to_string());
            let mut bm25_fields: Vec<String> = ec.content_fields().into_iter().map(|s| s.to_string()).collect();
            // Une entité dérivée rend sa racine (`_source_entity`, `_source_uuid`),
            // se filtre par elle, et porte ses poids de fusion. Son titre, rendu
            // par gabarit, se cherche en plein texte comme celui de l'ancienne
            // ligne d'index (`_title` + `_content`).
            let derivee = ec.derived.as_ref();
            // **Le titre est un champ de recherche pour TOUTE entité**, plus
            // seulement les dérivées (la sonde du 4 octobre : le nom du Scope
            // n'était pas indexé, la définition ne portait pas son nom). Un
            // index existant ne gagne le champ qu'en étant rebâti — la
            // recherche filtre par le schéma du handle, voir search_bm25_chunked.
            let mut title_field = None;
            let mut title_boost = 1.0_f64;
            if let Some(title) = ec.title_field() {
                if !bm25_fields.iter().any(|f| f == title) {
                    bm25_fields.push(title.to_string());
                }
                title_field = Some(title.to_string());
                if let Some(b) = ec.fields.get(title).and_then(|d| d.boost) {
                    title_boost = b;
                }
            }
            if derivee.is_some() {
                for f in [crate::config::DerivedConfig::SOURCE_ENTITY, crate::config::DerivedConfig::SOURCE_UUID] {
                    if !enrich_fields.iter().any(|e| e == f) {
                        enrich_fields.push(f.to_string());
                    }
                }
            }
            return Ok(search::SearchTarget {
                name: name.to_string(),
                parent_table: name.to_string(),
                chunk_table,
                chunk_rel: format!("{name}_CHUNKED_FROM"),
                chunk_rel_fwd: false,
                bm25_fields,
                enrich_fields,
                default_signals: ec.signals,
                default_fusion: ec.fusion.clone(),
                field_weights: ec.field_weights.clone(),
                has_source_refs: derivee.is_some(),
                filter_indirection: derivee.map(|d| (d.from.clone(), crate::schema::derived_rel_name(name))),
                title_field,
                title_boost,
            });
        }

        Err(CatalogError::UnknownKB(name.to_string()))
    }

    pub fn compile_filter_utilisateur(
        &self,
        entity: &str,
        condition: Option<&FilterCondition>,
    ) -> Result<(Option<String>, Vec<QueryParam>, Option<String>), CatalogError> {
        let Some(cond) = condition else {
            return Ok((None, vec![], None));
        };
        let mut parser = FilterParser::new(&self.relations, self.dialect.as_ref());
        let parsed = parser
            .parse_condition(cond, entity, "p")
            .map_err(|e| CatalogError::FilterError(e.to_string()))?;
        let w = (!parsed.where_clauses.is_empty()).then(|| parsed.combine_where());
        let mut clauses: Vec<String> = Vec::new();
        if w.is_some() {
            // La jointure chunk→parent d'abord : les jointures inter-entités
            // qui suivent s'accrochent à `p`.
            clauses.push(self.dialect.chunk_parent_join("n", "p", entity));
        }
        clauses.extend(parsed.match_clauses.iter().cloned());
        let m = (!clauses.is_empty()).then(|| clauses.join(" "));
        Ok((w, parsed.params, m))
    }

    pub fn compile_filter_for_vector_in(
        &self,
        cell: &crate::scope::Scope,
        entity: &str,
        condition: Option<&FilterCondition>,
    ) -> Result<(Option<String>, Vec<QueryParam>, Option<String>), CatalogError> {
        // `n` est la table de **chunks** : c'est elle que le HNSW indexe. Un
        // filtre porte pourtant sur les champs du **parent** (`file_path`,
        // `repo`, `language`…), qui n'existent pas sur un chunk. On compile
        // donc la condition sur un alias `p`, et on joint.
        //
        // Sans cette jointure, un filtre sur un champ parent ne rendait pas
        // moins de résultats : il **plantait**, « Cannot find property
        // file_path for n » — sur `Catalog::search` comme sur le nœud. Trouvé
        // le 27 août en câblant le domaine de travail au chemin vectoriel.
        let (where_str, mut params, match_str) =
            self.compile_filter_utilisateur(entity, condition)?;

        if !self.multi_cell {
            return Ok((where_str, params, match_str));
        }
        let scope_where = format!(
            "n.{} = $_scope_org AND n.{} = $_scope_project",
            crate::scope::ORG_COLUMN,
            crate::scope::PROJECT_COLUMN
        );
        params.push(QueryParam::new("_scope_org", CypherValue::String(cell.org.clone())));
        params.push(QueryParam::new("_scope_project", CypherValue::String(cell.project.clone())));
        let combined = match where_str {
            Some(w) => format!("({w}) AND {scope_where}"),
            None => scope_where,
        };
        Ok((Some(combined), params, match_str))
    }

    pub fn resolve_filter_to_ids(
        &self,
        entity: &str,
        condition: &FilterCondition,
        target: &crate::search::SearchTarget,
    ) -> Result<Option<Vec<u64>>, CatalogError> {
        let (filter_entity, filter_alias, join_from): (&str, &str, Option<(&str, &str, &str)>) =
            if let Some((ref title_entity, ref in_rel)) = target.filter_indirection {
                (title_entity.as_str(), "t", Some(("t", title_entity.as_str(), in_rel.as_str())))
            } else {
                (entity, "n", None)
            };

        let mut parser = FilterParser::new(&self.relations, self.dialect.as_ref());
        let parsed = parser
            .parse_condition(condition, filter_entity, filter_alias)
            .map_err(|e| CatalogError::FilterError(e.to_string()))?;

        if parsed.where_clauses.is_empty() {
            return Ok(None);
        }

        let resolve_alias = if join_from.is_some() { "idx" } else { "n" };
        let query = self.dialect.filter_resolve_offsets(
            entity,
            resolve_alias,
            &parsed.match_clauses,
            &parsed.combine_where(),
            join_from,
        );
        let result = if parsed.params.is_empty() {
            self.conn.execute(&query).map_err(|e| CatalogError::DbError(e.to_string()))?
        } else {
            self.conn
                .execute_with_params(&query, &parsed.params)
                .map_err(|e| CatalogError::DbError(e.to_string()))?
        };
        let mut ids: Vec<u64> = result
            .rows
            .iter()
            .filter_map(|r| r.first().and_then(|v| v.as_i64()).map(|i| i as u64))
            .collect();
        // **Contrat d'API de lucivy** (doc 09, §2.4) : un ensemble trié et
        // sans doublon est lu sur place, sans allocation ; un ensemble
        // quelconque est copié, trié et dédupliqué **à chaque requête**. Ils
        // l'ont mesuré : 540 000 ids passent de 6,004 ms à 0,220 ms. La
        // requête les rend probablement déjà triés — on ne parie pas dessus,
        // ça coûte un tri une fois contre un tri par requête.
        ids.sort_unstable();
        ids.dedup();
        Ok(Some(ids))
    }

    pub fn search_target_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .entity_configs
            .iter()
            .filter(|(_, ec)| ec.has_simple_pipeline())
            .map(|(name, _)| name.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    pub fn resolve_to_source_entities(
        &self,
        results: &mut Vec<search::SearchResult>,
    ) -> Result<(), CatalogError> {
        use crate::connection::CypherValue;

        // 1. Group by entity type → [source_uuid]
        let mut by_entity: HashMap<String, Vec<String>> = HashMap::new();
        for r in results.iter() {
            if let Some(ref data) = r.data {
                let entity = data.get("_source_entity").and_then(|v| v.as_str());
                let uuid = data.get("_source_uuid").and_then(|v| v.as_str());
                if let (Some(e), Some(u)) = (entity, uuid) {
                    by_entity.entry(e.to_string()).or_default().push(u.to_string());
                }
            }
        }

        // 2. Batch fetch source entity data
        let mut source_data: HashMap<String, (String, BTreeMap<String, CypherValue>)> = HashMap::new();
        for (entity_name, uuids) in &by_entity {
            let deduped: HashSet<&str> = uuids.iter().map(|s| s.as_str()).collect();
            let uuid_param = CypherValue::List(
                deduped.iter().map(|u| CypherValue::String(u.to_string())).collect(),
            );
            let cypher = self.dialect.select_entity_all_by_uuids(entity_name);
            let result = self.conn
                .execute_with_params(
                    &cypher,
                    &[QueryParam { name: "uuids".into(), value: uuid_param }],
                )
                .map_err(|e| CatalogError::DbError(e.to_string()))?;

            for row in &result.rows {
                if let Some(CypherValue::Map(map)) = row.first() {
                    if let Some(uuid) = map.get("_uuid").and_then(|v| v.as_str()) {
                        source_data.insert(
                            uuid.to_string(),
                            (entity_name.clone(), map.clone()),
                        );
                    }
                }
            }
        }

        // 3. Replace uuid/entity/data for each result
        for r in results.iter_mut() {
            let source_uuid = r.data.as_ref()
                .and_then(|d| d.get("_source_uuid"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            if let Some(su) = source_uuid {
                if let Some((entity_name, data)) = source_data.get(&su) {
                    r.uuid = su;
                    r.entity = Some(entity_name.clone());
                    r.data = Some(data.clone());
                }
            }
        }

        // 4. Deduplicate by UUID (same source entity), keep highest score
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut to_remove: Vec<usize> = Vec::new();
        for (i, r) in results.iter().enumerate() {
            if let Some(&prev_idx) = seen.get(&r.uuid) {
                if r.score > results[prev_idx].score {
                    to_remove.push(prev_idx);
                    seen.insert(r.uuid.clone(), i);
                } else {
                    to_remove.push(i);
                }
            } else {
                seen.insert(r.uuid.clone(), i);
            }
        }
        to_remove.sort_unstable();
        to_remove.dedup();
        for idx in to_remove.into_iter().rev() {
            results.remove(idx);
        }

        Ok(())
    }
}

impl Forme<'_> {
    /// Les relations déclarées, `(nom, de, vers)`, triées.
    pub fn relation_summaries(&self) -> Vec<(String, String, String)> {
        let mut rels: Vec<(String, String, String)> =
            self.relations.iter().map(|(name, def)| (name.clone(), def.from.clone(), def.to.clone())).collect();
        rels.sort();
        rels
    }

    /// La configuration d'une entité.
    pub fn entity_config(&self, name: &str) -> Option<&EntityConfig> {
        self.entity_configs.get(name)
    }
}

/// **Une ligne rendue en dictionnaire** : un nœud entier (`RETURN n`) tel
/// quel, sinon ses colonnes. Le corps de `Catalog::row_to_map`, partagé avec
/// la vue (un autre nom : `e2e_impact` suit `row_to_map` par son nom).
pub(crate) fn ligne_en_dictionnaire(columns: &[String], row: &[CypherValue]) -> BTreeMap<String, CypherValue> {
    // rag3db returns a whole node in one column (`RETURN n`), while SQL
    // returns its columns. Unwrap only this known node shape, not arbitrary
    // object-valued projections such as `RETURN n.payload`.
    if columns.len() == 1 && columns[0] == "n" && row.len() == 1 {
        if let CypherValue::Map(node) = &row[0] {
            if node.contains_key("_label") && node.contains_key("_uuid") {
                return node.clone();
            }
        }
    }
    let mut data = BTreeMap::new();
    for (i, col) in columns.iter().enumerate() {
        if i < row.len() {
            data.insert(col.clone(), row[i].clone());
        }
    }
    data
}

/// **Où le modèle courant range ses vecteurs**, d'après une liste de modèles
/// enregistrés : le corps de `Catalog::vector_storage`, pour la vue, qui porte
/// la liste et le modèle courant dans ses services.
pub fn vector_storage_from(
    chunk_table: &str,
    slug: &str,
    registered: &[crate::embedding_storage::EmbeddingModelEntry],
) -> Result<crate::embedding_storage::VectorStorage, CatalogError> {
    match registered.iter().find(|e| e.slug() == slug) {
        Some(entry) => Ok(crate::embedding_storage::VectorStorage::resolve(chunk_table, entry)),
        None => Err(CatalogError::EmbeddingModelUnavailable(crate::embedding_storage::unavailable_message(slug, registered))),
    }
}

/// **La forme du catalogue, figée**, et la connexion de lecture.
pub struct ReadView {
    pub(crate) dialect: Arc<dyn SchemaDialect>,
    pub(crate) conn: Arc<dyn DbConnection>,
    pub(crate) entity_configs: HashMap<String, EntityConfig>,
    pub(crate) entity_defs: HashMap<String, EntityDef>,
    pub(crate) relations: HashMap<String, RelationDef>,
    pub(crate) scope: crate::scope::Scope,
    pub(crate) fanout: Option<bool>,
    pub(crate) two_step_hops: bool,
    /// Plusieurs cellules en base : le filtre vectoriel y ajoute la cellule.
    pub(crate) multi_cell: bool,
    /// Le moteur de recherche posé du dehors (Elasticsearch…), s'il y en a
    /// un ; `None` pour celui du catalogue, que la vue rebâtit sur sa
    /// connexion de lecture.
    pub(crate) search_backend_externe: Option<Arc<dyn crate::search_backend::SearchBackend>>,
    pub(crate) event_bus: EventBus,
    /// Les services de recherche de la cellule du catalogue, tout prêts :
    /// `build_services` les pose en parent du registre d'un appel.
    pub search_services: Arc<ServiceRegistry>,
}

impl ReadView {
    /// Une entité déclarée, comme `Catalog::check_entity` ; une base à
    /// rouvrir se refuse comme au catalogue.
    fn entite_connue(&self, name: &str) -> Result<&EntityDef, CatalogError> {
        if let Some(reason) = self.conn.must_reopen() {
            return Err(CatalogError::MustReopen(reason));
        }
        self.entity_defs.get(name).ok_or_else(|| CatalogError::UnknownEntity(name.to_string()))
    }

    /// La forme de la vue, sur sa connexion de lecture.
    pub fn forme(&self) -> Forme<'_> {
        Forme { entity_configs: &self.entity_configs, relations: &self.relations, dialect: &self.dialect, conn: &self.conn, multi_cell: self.multi_cell }
    }

    /// La cellule de la vue.
    pub fn scope(&self) -> &crate::scope::Scope {
        &self.scope
    }

    /// Les configurations d'entités de la vue.
    pub fn entity_configs(&self) -> &HashMap<String, EntityConfig> {
        &self.entity_configs
    }
}

impl ReadCatalog for ReadView {
    fn dialect_arc(&self) -> Arc<dyn SchemaDialect> {
        self.dialect.clone()
    }
    fn execute_read_with_params(&self, cypher: &str, params: &[QueryParam]) -> Result<QueryResult, CatalogError> {
        self.conn.execute_with_params(cypher, params).map_err(|e| CatalogError::DbError(e.to_string()))
    }
    fn run_read_plan(&self, plan: &rag3weaver_ir::Plan, mut options: crate::plan::PlanOptions) -> Result<crate::plan::PlanReport, crate::plan::PlanError> {
        if options.fanout.is_none() {
            options.fanout = self.fanout;
        }
        options.no_transaction = true;
        crate::plan::run_plan(self.dialect.as_ref(), self.conn.as_ref(), plan, options)
    }
    fn entity_config(&self, name: &str) -> Option<&EntityConfig> {
        self.entity_configs.get(name)
    }
    fn get_relation_def(&self, name: &str) -> Option<&RelationDef> {
        self.relations.get(name)
    }
    fn entity_uuid(&self, entity_name: &str, data: &BTreeMap<String, CypherValue>) -> Result<String, CatalogError> {
        let def = self.entity_defs.get(entity_name).ok_or_else(|| CatalogError::UnknownEntity(entity_name.to_string()))?;
        Ok(Catalog::uuid_for_in(&self.scope, entity_name, def, data))
    }
    fn read_meta_key(&self, key: &str) -> Result<Option<String>, CatalogError> {
        Catalog::read_meta_key_on(self.dialect.as_ref(), self.conn.as_ref(), key)
    }
    fn two_step_hops(&self) -> bool {
        self.two_step_hops
    }
    fn emit_event(&self, event: CatalogEvent) {
        self.event_bus.in_scope(&self.scope).emit(event);
    }
    fn get(&self, entity_name: &str, uuid: &str) -> Result<Option<BTreeMap<String, CypherValue>>, CatalogError> {
        self.entite_connue(entity_name)?;
        let cypher = self.dialect.select_entity_all_by_uuids(entity_name);
        let result = self
            .conn
            .execute_with_params(&cypher, &[QueryParam::new("uuids", CypherValue::List(vec![CypherValue::String(uuid.to_string())]))])
            .map_err(|e| CatalogError::DbError(e.to_string()))?;
        if result.is_empty() {
            return Ok(None);
        }
        Ok(Some(ligne_en_dictionnaire(&result.columns, &result.rows[0])))
    }
    fn find_by_field(&self, entity_name: &str, field: &str, value: CypherValue, fields: &[&str]) -> Result<Vec<BTreeMap<String, CypherValue>>, CatalogError> {
        self.entite_connue(entity_name)?;
        let cypher = self.dialect.select_by_field(entity_name, field, fields);
        let result = self
            .conn
            .execute_with_params(&cypher, &[QueryParam::new("value", value)])
            .map_err(|e| CatalogError::DbError(e.to_string()))?;
        Ok(result.rows.iter().map(|row| ligne_en_dictionnaire(&result.columns, row)).collect())
    }
    fn relation_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.relations.keys().cloned().collect();
        v.sort();
        v
    }
}

/// **La cellule où la vue se publie** : partagée entre le catalogue (qui
/// publie) et les lecteurs (qui prennent la vue courante), sans le mutex.
#[derive(Default)]
pub struct ReadViewCell(RwLock<Option<Arc<ReadView>>>);

impl ReadViewCell {
    /// La vue courante, si le catalogue en a publié une.
    pub fn current(&self) -> Option<Arc<ReadView>> {
        self.0.read().ok().and_then(|v| v.clone())
    }

    /// Plus de vue : les lectures retombent sur le catalogue. À la
    /// fermeture, pour que la vue ne garde pas des poignées fermées.
    pub(crate) fn clear(&self) {
        if let Ok(mut v) = self.0.write() {
            *v = None;
        }
    }

    pub(crate) fn set(&self, view: Arc<ReadView>) {
        if let Ok(mut v) = self.0.write() {
            *v = Some(view);
        }
    }
}

#[cfg(all(test, feature = "rag3db-native"))]
mod tests {
    use crate::dataflow::services::ConnService;
    use crate::embedder::HashEmbedder;
    use crate::{Catalog, CatalogConfig, Rag3dbConnection};

    /// **La vue de lecture cherche par la connexion de lecture** (L4, premier
    /// commit, 18 octobre 2026) : ses services de recherche portaient la
    /// connexion principale, celle de l'écrivain. Une recherche par la vue
    /// entrait alors dans sa transaction ouverte et voyait ses lignes non
    /// validées. Une ligne validée, une seconde écrite dans une transaction
    /// ouverte : la connexion des services de la vue ne voit que la première.
    #[test]
    fn la_vue_de_lecture_ne_voit_pas_la_transaction_ouverte_de_l_ecrivain() {
        let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
        let mut cat = Catalog::new(
            Box::new(conn),
            Box::new(HashEmbedder::new(8)),
            CatalogConfig { embedding_dim: 8, allow_mock_embedder: true, ..Default::default() },
        );
        cat.initialize().unwrap();
        cat.execute_raw("CREATE NODE TABLE Essai(id INT64 PRIMARY KEY)").unwrap();
        cat.execute_raw("CREATE (:Essai {id: 1})").unwrap();
        // Comme l'ouverture d'un backend la publie.
        cat.publish_read_view();
        let vue = cat.read_view_cell().current().expect("la vue est publiée");
        let lecture = vue.search_services.get::<ConnService>("conn").expect("le service conn de la vue").0.clone();
        let compte = |c: &std::sync::Arc<dyn crate::connection::DbConnection>| {
            c.execute("MATCH (n:Essai) RETURN count(n)").unwrap().rows[0][0].as_i64().unwrap()
        };

        let ecrivain = cat.conn_arc();
        ecrivain.begin().unwrap();
        cat.execute_raw("CREATE (:Essai {id: 2})").unwrap();
        assert_eq!(compte(&ecrivain), 2, "l'écrivain voit sa propre écriture");
        assert_eq!(compte(&lecture), 1, "la connexion des services de la vue ne voit que l'état validé");
        ecrivain.commit().unwrap();
        assert_eq!(compte(&lecture), 2, "après le COMMIT, la vue voit les deux");
    }
}
