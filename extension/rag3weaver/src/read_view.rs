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

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

use crate::catalog::{Catalog, CatalogError};
use crate::config::{EntityConfig, EntityDef, RelationDef};
use crate::connection::{CypherValue, DbConnection, QueryParam, QueryResult};
use crate::dataflow::ServiceRegistry;
use crate::dialect::SchemaDialect;
use crate::events::{CatalogEvent, EventBus};

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
    pub(crate) event_bus: EventBus,
    /// Les services de recherche de la cellule du catalogue, tout prêts :
    /// `build_services` les pose en parent du registre d'un appel.
    pub search_services: Arc<ServiceRegistry>,
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

    pub(crate) fn set(&self, view: Arc<ReadView>) {
        if let Ok(mut v) = self.0.write() {
            *v = Some(view);
        }
    }
}
