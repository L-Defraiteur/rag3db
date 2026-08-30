//! Generic search nodes for composable search pipelines.
//!
//! Each node wraps a primitive from [`search`](crate::search) and can be composed
//! via Mermaid templates to build custom search pipelines (BM25-only, vector-only,
//! hybrid, hybrid+sparse) without modifying Rust code.
//!
//! - [`SearchSourceNode`] — resolves SearchTarget + emits Query
//! - [`VectorSearchNode`] — vector similarity search on chunk embeddings
//! - [`BM25SearchNode`] — full-text BM25 search with highlight→chunk resolution
//! - [`SparseSearchNode`] — sparse vector search (SPLADE/BGE-M3)
//! - [`FuseResultsNode`] — fusion N-aire de signaux étiquetés (RRF ou pondérée)
//! - [`RerankNode`] — cross-encoder sur la tête des résultats
//! - [`ResolveParentNode`] — resolve chunks → parent entities with data enrichment
//!
//! # Signaux étiquetés
//!
//! Chaque nœud de recherche étiquette ses résultats (`UnifiedResult::signal`)
//! avec son nom, ou la config `signal`. `FuseResultsNode` accepte en plus de
//! ses trois ports historiques (`vector`, `bm25`, `sparse`) un port `signals`
//! en fan-in : N branches y arrivent concaténées, et sont retrouvées par leur
//! étiquette. La pondération est alors une topologie et des poids nommés —
//! deux BM25 sur deux champs, un vecteur, un reranker en `boost` — au lieu
//! d'un réglage figé dans la configuration du catalogue.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use std::sync::Mutex;

use crate::catalog::Catalog;
use crate::embedder::{DualEmbedder, Embedder, SparseEmbedder};
use crate::reranker::{passage_text, Reranker};
use crate::search::{
    embed_query, enrich_results_with_data, fuse_signals, resolve_vector_chunks,
    search_bm25_chunked, search_sparse, search_vector, search_vector_via_backend, BM25Mode,
    FusionConfig, FusionStrategy, ResultMode, SearchOptions, SearchResult, SearchTarget,
    SignalConfig, SignalRole, DEFAULT_RRF_K,
};
use crate::search_strategy::UnifiedResult;

use super::node::{Node, NodeContext};
use super::port::{take_or_clone, PortDef, PortType, PortValue, QueryPayload};
use super::services::ConnService;

// ─── SearchSourceNode ────────────────────────────────────────────────────────

/// Resolves a `SearchTarget` from the catalog and emits a Query with it.
///
/// Unlike [`KBQuerySourceNode`](super::search_nodes::KBQuerySourceNode) which emits
/// a raw query without resolving the target, this node resolves table/column names
/// so downstream nodes can use them directly.
pub struct SearchSourceNode {
    node_name: String,
    target_name: String,
    query: String,
    options: SearchOptions,
}

impl SearchSourceNode {
    pub fn new(name: &str, target_name: &str, query: &str, options: SearchOptions) -> Self {
        Self {
            node_name: name.to_string(),
            target_name: target_name.to_string(),
            query: query.to_string(),
            options,
        }
    }
}


impl Node for SearchSourceNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "SearchSourceNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "target_name": self.target_name,
            "query": self.query,
        })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "query",
            port_type: PortType::Query,
            required: false,
        }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let catalog = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog").cloned()
            .ok_or("SearchSourceNode: 'catalog' service not found")?;

        let target = {
            let catalog = catalog.lock().unwrap();
            catalog
                .resolve_search_target(&self.target_name)
                .map_err(|e| format!("SearchSourceNode: {e}"))?
        };

        // Le domaine de travail, s'il y en a un dans le registre : la vision
        // de l'agent rétrécit la recherche, par câblage et sans que la fiche
        // ait rien à déclarer. Un filtre déjà posé par l'appelant l'emporte —
        // il est plus précis que la vision générale.
        let mut options = self.options.clone();
        if options.filter_condition.is_none() {
            if let Some(domain) = ctx.service::<Arc<crate::work_domain::WorkDomain>>(crate::work_domain::WORK_DOMAIN_SERVICE).cloned() {
                let fields = {
                    let catalog = catalog.lock().unwrap();
                    catalog.entity_configs().get(&self.target_name).map(|c| c.fields.keys().cloned().collect::<std::collections::HashSet<_>>())
                };
                let has = |f: &str| fields.as_ref().is_some_and(|s| s.contains(f));
                let path_field = if has("file_path") { "file_path" } else { "path" };
                if domain.applies_to(has, path_field) {
                    options.filter_condition = domain.to_filter(path_field);
                } else if !domain.is_everything() {
                    // Ne jamais rétrécir en silence, et ne jamais faire
                    // semblant de rétrécir : `Symbol` n'a ni dépôt ni
                    // fichier, le domaine ne peut rien y faire, et il faut
                    // que ça se sache.
                    ctx.warn(&format!(
                        "SearchSourceNode: le domaine « {} » ne s'applique pas à {} (champs manquants : {})",
                        domain.name,
                        self.target_name,
                        domain.required_fields(path_field).into_iter().filter(|f| !has(f)).collect::<Vec<_>>().join(", ")
                    ));
                }
            }
        }

        ctx.set_output(
            "query",
            PortValue::new(QueryPayload {
                target_name: self.target_name.clone(),
                query: self.query.clone(),
                options,
                target: Some(target),
            }),
        );
        Ok(())
    }
}

// ─── VectorSearchNode ────────────────────────────────────────────────────────

/// Vector similarity search on chunk embeddings.
///
/// Embeds the query string, then searches the chunk table. Passe par le
/// `SearchBackend` du catalogue quand le service `catalog` en expose un
/// (même chemin que `Catalog::search`, agnostique du moteur) ; sans catalogue,
/// retombe sur le chemin Cypher direct.
pub struct VectorSearchNode {
    node_name: String,
    limit: usize,
    result_mode: ResultMode,
    signal: Option<String>,
}

impl VectorSearchNode {
    pub fn new(name: &str, limit: usize) -> Self {
        Self {
            node_name: name.to_string(),
            limit,
            result_mode: ResultMode::Aggregated,
            signal: None,
        }
    }

    pub fn with_result_mode(mut self, mode: ResultMode) -> Self {
        self.result_mode = mode;
        self
    }

    /// Étiquette des résultats (défaut : le nom du nœud).
    pub fn with_signal(mut self, signal: impl Into<String>) -> Self {
        self.signal = Some(signal.into());
        self
    }
}


impl Node for VectorSearchNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "VectorSearchNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "limit": self.limit,
            "result_mode": self.result_mode,
            "signal": self.signal,
        })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "query",
            port_type: PortType::Query,
            required: true,
        }]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "results",
            port_type: PortType::Results,
            required: false,
        }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let (query_str, target, options) = extract_query_and_target(ctx, "VectorSearchNode")?;

        // Une cible sans vecteurs n'est pas une panne, c'est une cible sans
        // vecteurs. On rend une liste vide en le disant, plutôt que d'échouer :
        // sinon un outil hybride serait inutilisable sur `Symbol`, déclaré
        // BM25 seul, et le seul recours serait de rendre `search` borgne pour
        // tout le monde — ce qu'on a fait pendant des mois sans le voir.
        if !declares(&target, &options, "vector") {
            ctx.warn(&format!(
                "VectorSearchNode: '{}' ne déclare pas le signal 'vector' — aucun résultat vectoriel",
                target.name
            ));
            ctx.set_output("results", PortValue::new(Vec::<UnifiedResult>::new()));
            return Ok(());
        }

        let conn = ctx
            .service::<ConnService>("conn")
            .ok_or("VectorSearchNode: 'conn' service not found")?
            .0.clone();
        let embedder = ctx
            .service::<Arc<dyn Embedder>>("embedder").cloned()
            .ok_or("VectorSearchNode: 'embedder' service not found")?;

        // Le vecteur ne se pré-filtre pas par offsets — le HNSW ne connaît
        // pas nos identités — mais par du Cypher sur l'entité parente. Le
        // catalogue sait le compiler, cellule comprise.
        let (filter_where, filter_params, filter_match) = match &options.filter_condition {
            None => (None, vec![], None),
            Some(cond) => match ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned() {
                Some(catalog) => {
                    let compiled = catalog.lock().unwrap().compile_filter_for_vector(&target.name, Some(cond));
                    match compiled {
                        Ok(c) => c,
                        Err(e) => {
                            ctx.warn(&format!("VectorSearchNode: filtre non compilé ({e}) — résultats non restreints"));
                            (None, vec![], None)
                        }
                    }
                }
                None => {
                    ctx.warn("VectorSearchNode: un filtre est demandé mais le service 'catalog' manque — résultats non restreints");
                    (None, vec![], None)
                }
            },
        };

        let mut cache = HashMap::new();
        let embedding = embed_query(&*embedder, &query_str, &mut cache)
            .map_err(|e| format!("VectorSearchNode: embed failed: {e}"))?;

        let backend = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .and_then(|c| c.lock().unwrap().search_backend());
        let chunk_results = match backend {
            Some(backend) => search_vector_via_backend(
                backend.as_ref(),
                &target.chunk_table,
                &embedding,
                self.limit,
                filter_where.as_deref(),
                &filter_params,
                filter_match.as_deref(),
            ),
            None => search_vector(
                &*conn,
                &target.chunk_table,
                &target.name,
                &embedding,
                self.limit,
                filter_where.as_deref(),
                &filter_params,
                filter_match.as_deref(),
            ),
        }
        .map_err(|e| format!("VectorSearchNode: search failed: {e}"))?;

        // Resolve chunk-level results → parent-level with data enrichment
        let results = resolve_vector_chunks(
            &*conn,
            &target,
            chunk_results,
            &target.enrich_fields,
            self.result_mode,
        )
        .map_err(|e| format!("VectorSearchNode: resolve chunks failed: {e}"))?;

        let label = self.signal.clone().unwrap_or_else(|| self.node_name.clone());
        let unified = finish_signal(ctx, "VectorSearchNode", &target, results, self.result_mode, &label)?;
        ctx.set_output("results", PortValue::new(unified));
        Ok(())
    }
}

// ─── BM25SearchNode ──────────────────────────────────────────────────────────

/// BM25 full-text search with highlight→chunk resolution.
///
/// `fields` restreint la recherche à certains champs de l'index (par défaut
/// tous ceux de la cible). C'est ce qui permet deux branches BM25 sur `_title`
/// et `_content`, pesées séparément à la fusion — le « boost de titre » sans
/// pondération par champ dans le moteur.
pub struct BM25SearchNode {
    node_name: String,
    limit: usize,
    fuzzy_distance: u8,
    result_mode: ResultMode,
    mode: BM25Mode,
    fields: Option<Vec<String>>,
    signal: Option<String>,
}

impl BM25SearchNode {
    pub fn new(name: &str, limit: usize) -> Self {
        Self {
            node_name: name.to_string(),
            limit,
            fuzzy_distance: 0,
            result_mode: ResultMode::Aggregated,
            mode: BM25Mode::Contains,
            fields: None,
            signal: None,
        }
    }

    pub fn with_fuzzy(mut self, distance: u8) -> Self {
        self.fuzzy_distance = distance;
        self
    }

    pub fn with_result_mode(mut self, mode: ResultMode) -> Self {
        self.result_mode = mode;
        self
    }

    pub fn with_mode(mut self, mode: BM25Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Champs interrogés, à la place de ceux de la cible.
    pub fn with_fields(mut self, fields: Vec<String>) -> Self {
        self.fields = Some(fields);
        self
    }

    /// Étiquette des résultats (défaut : le nom du nœud).
    pub fn with_signal(mut self, signal: impl Into<String>) -> Self {
        self.signal = Some(signal.into());
        self
    }
}


impl Node for BM25SearchNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "BM25SearchNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "limit": self.limit,
            "fuzzy_distance": self.fuzzy_distance,
            "result_mode": self.result_mode,
            "mode": self.mode,
            "fields": self.fields,
            "signal": self.signal,
        })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "query",
            port_type: PortType::Query,
            required: true,
        }]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                name: "results",
                port_type: PortType::Results,
                required: false,
            },
            // **Le canal qui manquait.** Les avertissements du moteur — « la
            // recherche floue ignore les séparateurs », un regex sans littéral,
            // une attribution de chunk douteuse — partaient dans le journal du
            // nœud et s'arrêtaient là. Un avertissement qui ne remonte pas est
            // un avertissement qui n'existe pas : l'agent voyait « aucun
            // résultat » sans jamais savoir qu'on n'avait pas cherché ce qu'il
            // croyait (issue 02 du 29 août 2026).
            PortDef {
                name: "meta",
                port_type: PortType::Meta,
                required: false,
            },
        ]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let debut = std::time::Instant::now();
        let (query_str, target, options) = extract_query_and_target(ctx, "BM25SearchNode")?;

        let conn = ctx
            .service::<ConnService>("conn")
            .ok_or("BM25SearchNode: 'conn' service not found")?
            .0.clone();

        // Le handle FTS de la table parente : d'abord l'instantané du service
        // `fts_handles`, sinon le catalogue **vivant** — une entité
        // enregistrée après la construction des services (une trace, un
        // message) a son index dans le catalogue, pas dans l'instantané.
        let fts_handle = ctx
            .service::<std::collections::HashMap<String, std::sync::Arc<lucivy_core::sharded_handle::ShardedHandle>>>("fts_handles")
            .and_then(|h| h.get(&target.parent_table).cloned())
            .or_else(|| {
                ctx.service::<Arc<Mutex<Catalog>>>("catalog")
                    .and_then(|c| c.lock().ok())
                    .and_then(|c| c.fts_handle(&target.parent_table))
            });

        let fields: &[String] = match &self.fields {
            Some(f) => {
                if let Some(unknown) = f.iter().find(|x| !target.bm25_fields.contains(x)) {
                    return Err(format!(
                        "BM25SearchNode: field '{unknown}' is not indexed on '{}' (indexed: {:?})",
                        target.name, target.bm25_fields
                    ));
                }
                f
            }
            None => &target.bm25_fields,
        };

        let allowed = allowed_ids_for(ctx, "BM25SearchNode", &target, &options);

        let mut node_warnings: Vec<String> = Vec::new();

        // **Le plein texte servi par la base**, quand elle sait le faire.
        //
        // On demande au catalogue plutôt qu'au service `fts_handles` : c'est
        // lui qui porte l'option (`MoteurTexte`), et c'est la même décision
        // qu'à l'ingestion — sinon on chercherait dans un index qu'on n'a pas
        // écrit.
        // Le service n'existe que si le catalogue a choisi ce chemin. S'il
        // manque alors qu'il devrait être là, la retombée sur lucivy échoue
        // bruyamment — « aucun index FTS ouvert » — parce qu'on n'aura pas
        // ouvert d'index non plus. Le défaut de câblage se voit au lieu de se
        // déguiser en zéro résultat.
        let natif = ctx
            .service::<Arc<dyn crate::search_backend::SearchBackend>>("texte_natif")
            .cloned();

        if let Some(backend) = natif {
            let results = crate::search::search_texte_natif(
                backend.as_ref(),
                &target,
                &query_str,
                self.limit,
                &target.enrich_fields,
                self.result_mode,
                None,
                &mut node_warnings,
            )
            .map_err(|e| format!("BM25SearchNode: recherche native: {e}"))?;
            for w in &node_warnings {
                ctx.warn(w);
            }
            let label = self.signal.clone().unwrap_or_else(|| self.node_name.clone());
            let unified =
                finish_signal(ctx, "BM25SearchNode", &target, results, self.result_mode, &label)?;
            let nombre = unified.len();
            ctx.set_output("results", PortValue::new(unified));
            ctx.set_output(
                "meta",
                PortValue::new(crate::search::SearchMeta {
                    query: query_str.clone(),
                    target: target.name.clone(),
                    signals: crate::search::SearchSignals::BM25,
                    consistency: options.consistency,
                    partial: false,
                    pending_count: 0,
                    vector_count: 0,
                    bm25_count: nombre,
                    sparse_count: 0,
                    fused_count: nombre,
                    reranked_count: 0,
                    warnings: node_warnings.clone(),
                    search_time_ms: debut.elapsed().as_millis() as u64,
                    diagnostics: None,
                }),
            );
            return Ok(());
        }

        let results = search_bm25_chunked(
            &*conn,
            &target,
            &query_str,
            fields,
            self.mode,
            self.fuzzy_distance,
            self.limit,
            allowed.as_deref(),
            &target.enrich_fields,
            self.result_mode,
            None,
            // Ce nœud n'a pas encore de canal de sortie pour les avertissements ;
            // ils sont collectés puis journalisés plutôt que perdus en silence.
            &mut node_warnings,
            // Handle FTS de la table parente si le service l'expose ; sinon on
            // reste sur le chemin C++.
            fts_handle.as_deref(),
        )
        .map_err(|e| format!("BM25SearchNode: search failed: {e}"))?;

        for w in &node_warnings {
            ctx.warn(w);
        }

        let label = self.signal.clone().unwrap_or_else(|| self.node_name.clone());
        let unified = finish_signal(ctx, "BM25SearchNode", &target, results, self.result_mode, &label)?;
        let nombre = unified.len();
        ctx.set_output("results", PortValue::new(unified));

        // **Une fiche honnête de ce que *ce* nœud a fait.** Les compteurs des
        // autres signaux sont à zéro parce qu'il ne les a pas exécutés — c'est
        // vrai, pas une omission. Le port est facultatif : un graphe qui ne le
        // branche pas se comporte exactement comme avant.
        ctx.set_output(
            "meta",
            PortValue::new(crate::search::SearchMeta {
                query: query_str.clone(),
                target: target.name.clone(),
                signals: crate::search::SearchSignals::BM25,
                consistency: options.consistency,
                partial: false,
                pending_count: 0,
                vector_count: 0,
                bm25_count: nombre,
                sparse_count: 0,
                fused_count: nombre,
                reranked_count: 0,
                search_time_ms: debut.elapsed().as_millis() as u64,
                warnings: node_warnings.clone(),
                diagnostics: None,
            }),
        );
        Ok(())
    }
}

// ─── SparseSearchNode ────────────────────────────────────────────────────────

/// Sparse vector search (SPLADE / BGE-M3).
pub struct SparseSearchNode {
    node_name: String,
    limit: usize,
    result_mode: ResultMode,
    signal: Option<String>,
}

impl SparseSearchNode {
    pub fn new(name: &str, limit: usize) -> Self {
        Self {
            node_name: name.to_string(),
            limit,
            result_mode: ResultMode::Aggregated,
            signal: None,
        }
    }

    pub fn with_result_mode(mut self, mode: ResultMode) -> Self {
        self.result_mode = mode;
        self
    }

    /// Étiquette des résultats (défaut : le nom du nœud).
    pub fn with_signal(mut self, signal: impl Into<String>) -> Self {
        self.signal = Some(signal.into());
        self
    }
}


impl Node for SparseSearchNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "SparseSearchNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "limit": self.limit,
            "result_mode": self.result_mode,
            "signal": self.signal,
        })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "query",
            port_type: PortType::Query,
            required: true,
        }]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "results",
            port_type: PortType::Results,
            required: false,
        }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let (query_str, target, options) = extract_query_and_target(ctx, "SparseSearchNode")?;

        // Même règle que pour le vecteur : une cible qui ne déclare pas
        // `sparse` rend vide, elle ne casse pas le graphe qui la traverse.
        if !declares(&target, &options, "sparse") {
            ctx.warn(&format!(
                "SparseSearchNode: '{}' ne déclare pas le signal 'sparse' — aucun résultat épars",
                target.name
            ));
            ctx.set_output("results", PortValue::new(Vec::<UnifiedResult>::new()));
            return Ok(());
        }

        let conn = ctx
            .service::<ConnService>("conn")
            .ok_or("SparseSearchNode: 'conn' service not found")?
            .0.clone();

        // Le pré-filtre sparse est **exact** : pas de statistique de corpus,
        // donc un filtre ne peut retirer que des lignes, jamais changer un
        // score (doc 09 §1, prouvé par leur `test_filter_truth.rs`). Et il
        // n'est jamais perdant — au pire 30 % au-dessus d'une recherche
        // complète, gagnant sous 1 % du corpus (doc 09 §2.3). On le pose
        // sans arrière-pensée.
        let allowed = allowed_ids_for(ctx, "SparseSearchNode", &target, &options);

        // Try dual embedder first, then sparse embedder
        let dual_emb = ctx.service::<Arc<dyn DualEmbedder>>("dual_embedder").cloned();
        let sparse_emb = ctx.service::<Arc<dyn SparseEmbedder>>("sparse_embedder").cloned();
        let sparse_vec = if let Some(dual) = dual_emb {
            let (_, sparse_vecs) = dual
                .embed_dual(&[query_str.clone()])
                .map_err(|e| format!("SparseSearchNode: dual embed failed: {e}"))?;
            sparse_vecs.into_iter().next().unwrap()
        } else if let Some(sparse) = sparse_emb {
            let vecs = sparse
                .embed_sparse(&[query_str.clone()])
                .map_err(|e| format!("SparseSearchNode: sparse embed failed: {e}"))?;
            vecs.into_iter().next().unwrap()
        } else {
            return Err("SparseSearchNode: no 'dual_embedder' or 'sparse_embedder' service".into());
        };

        let handles = ctx
            .service::<HashMap<String, Arc<sparse_vector::handle::SparseHandle>>>("sparse_handles").cloned()
            .ok_or("SparseSearchNode: 'sparse_handles' service not found")?;

        let handle = handles.get(&target.chunk_table)
            .ok_or_else(|| format!("SparseSearchNode: no sparse handle for '{}'", target.chunk_table))?;

        let backend = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .and_then(|c| c.lock().unwrap().search_backend());
        let chunk_results = match (&allowed, backend) {
            // Le chemin filtré passe par le backend : c'est lui qui expose
            // `search_filtered`.
            (Some(ids), Some(backend)) => crate::search::search_sparse_via_backend(
                handle,
                backend.as_ref(),
                &target.chunk_table,
                &sparse_vec,
                self.limit,
                &[],
                Some(ids),
            ),
            (Some(_), None) => {
                ctx.warn("SparseSearchNode: un filtre est demandé mais aucun backend de recherche — résultats non restreints");
                search_sparse(handle, &*conn, &target.chunk_table, &sparse_vec, self.limit, &[])
            }
            (None, _) => search_sparse(
                handle,
                &*conn,
                &target.chunk_table,
                &sparse_vec,
                self.limit,
                &[], // empty fields for chunked entities (fields are on parent table)
            ),
        }
        .map_err(|e| format!("SparseSearchNode: search failed: {e}"))?;

        // Resolve chunk-level results → parent-level with data enrichment
        let results = resolve_vector_chunks(
            &*conn,
            &target,
            chunk_results,
            &target.enrich_fields,
            self.result_mode,
        )
        .map_err(|e| format!("SparseSearchNode: resolve chunks failed: {e}"))?;

        let label = self.signal.clone().unwrap_or_else(|| self.node_name.clone());
        let unified = finish_signal(ctx, "SparseSearchNode", &target, results, self.result_mode, &label)?;
        ctx.set_output("results", PortValue::new(unified));
        Ok(())
    }
}

// ─── FuseResultsNode ─────────────────────────────────────────────────────────

/// Fusion N-aire de signaux étiquetés.
///
/// Entrées : les trois ports historiques `vector`, `bm25`, `sparse` (une liste
/// chacun, étiquetée par le nom du port), et le port `signals` en **fan-in** :
/// tout ce qui y arrive est regroupé par `UnifiedResult::signal`, dans l'ordre
/// de première apparition. Une étiquette présente des deux côtés est fusionnée
/// en une seule liste.
///
/// Poids : `weights` par étiquette ; sans entrée, `vector`/`bm25`/`sparse`
/// gardent les défauts de [`FusionConfig`] (0,7 / 0,3 / 0,2) et toute autre
/// étiquette vaut 1,0. `boost` nomme les étiquettes en rôle `Boost` : elles ne
/// participent pas à la fusion mais modulent le score fusionné — c'est ainsi
/// qu'un [`RerankNode`] se **mélange** au lieu de remplacer.
pub struct FuseResultsNode {
    node_name: String,
    strategy: FusionStrategy,
    rrf_k: f64,
    weights: HashMap<String, f64>,
    boost: HashSet<String>,
    top_k: Option<usize>,
    signal: Option<String>,
}

impl FuseResultsNode {
    pub fn new(name: &str) -> Self {
        Self {
            node_name: name.to_string(),
            strategy: FusionStrategy::Rrf,
            rrf_k: DEFAULT_RRF_K,
            weights: HashMap::new(),
            boost: HashSet::new(),
            top_k: None,
            signal: None,
        }
    }

    pub fn with_strategy(mut self, strategy: FusionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_rrf_k(mut self, k: f64) -> Self {
        self.rrf_k = k;
        self
    }

    /// Poids d'une étiquette.
    pub fn with_weight(mut self, label: impl Into<String>, weight: f64) -> Self {
        self.weights.insert(label.into(), weight);
        self
    }

    /// Étiquette en rôle `Boost` (module le score fusionné au lieu d'y entrer).
    pub fn with_boost(mut self, label: impl Into<String>) -> Self {
        self.boost.insert(label.into());
        self
    }

    /// Troncature de chaque liste avant fusion.
    pub fn with_top_k(mut self, k: usize) -> Self {
        self.top_k = Some(k);
        self
    }

    /// Étiquette des résultats fusionnés (défaut : le nom du nœud).
    pub fn with_signal(mut self, signal: impl Into<String>) -> Self {
        self.signal = Some(signal.into());
        self
    }

    fn signal_config(&self, label: &str) -> SignalConfig {
        let mut cfg = FusionConfig::default().signal_config(label);
        if let Some(w) = self.weights.get(label) {
            cfg.weight = *w;
        }
        if self.boost.contains(label) {
            cfg.role = SignalRole::Boost;
        }
        if self.top_k.is_some() {
            cfg.top_k = self.top_k;
        }
        cfg
    }
}


impl Node for FuseResultsNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "FuseResultsNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        let mut boost: Vec<&String> = self.boost.iter().collect();
        boost.sort();
        Some(Box::new(serde_json::json!({
            "strategy": self.strategy,
            "rrf_k": self.rrf_k,
            "weights": self.weights.iter().collect::<std::collections::BTreeMap<_, _>>(),
            "boost": boost,
            "top_k": self.top_k,
            "signal": self.signal,
        })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                name: "vector",
                port_type: PortType::Results,
                required: false,
            },
            PortDef {
                name: "bm25",
                port_type: PortType::Results,
                required: false,
            },
            PortDef {
                name: "sparse",
                port_type: PortType::Results,
                required: false,
            },
            PortDef {
                name: "signals",
                port_type: PortType::Results,
                required: false,
            },
        ]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "results",
            port_type: PortType::Results,
            required: false,
        }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        // Listes étiquetées, dans l'ordre : ports nommés, puis fan-in par
        // étiquette (ordre de première apparition).
        let mut groups: Vec<(String, Vec<UnifiedResult>)> = Vec::new();
        fn push(groups: &mut Vec<(String, Vec<UnifiedResult>)>, label: String, r: UnifiedResult) {
            match groups.iter_mut().find(|(l, _)| *l == label) {
                Some((_, v)) => v.push(r),
                None => groups.push((label, vec![r])),
            }
        }
        for port in ["vector", "bm25", "sparse"] {
            for r in take_results(ctx, port) {
                push(&mut groups, port.to_string(), r);
            }
        }
        for r in take_results(ctx, "signals") {
            let label = r.signal.clone().unwrap_or_else(|| "signals".to_string());
            push(&mut groups, label, r);
        }

        let label_out = self.signal.clone().unwrap_or_else(|| self.node_name.clone());

        // Convert UnifiedResult → SearchResult for fuse_signals()
        let lists: Vec<(Vec<SearchResult>, SignalConfig)> = groups
            .iter()
            .map(|(label, v)| {
                (
                    v.iter().cloned().map(SearchResult::from).collect(),
                    self.signal_config(label),
                )
            })
            .collect();
        let borrowed: Vec<(&[SearchResult], SignalConfig)> =
            lists.iter().map(|(l, c)| (l.as_slice(), *c)).collect();
        let fused_sr = fuse_signals(&borrowed, self.strategy, self.rrf_k);

        for (label, v) in &groups {
            ctx.metric(&format!("signal.{label}"), v.len() as f64);
        }

        // Build a lookup from all input results to preserve rich data, **et
        // qui l'a trouvé**. Jusqu'au 27 août 2026 la fusion écrasait
        // `signal` par son propre nom : la provenance mourait ici, et une
        // trace ne pouvait plus dire si un résultat venait du plein texte,
        // du vecteur, ou des deux. C'est exactement la question qu'on s'est
        // posée en relisant un artefact.
        let mut all_by_uuid: HashMap<String, UnifiedResult> = HashMap::new();
        let mut from_by_uuid: HashMap<String, Vec<String>> = HashMap::new();
        for (label, v) in groups {
            for r in v {
                let seen = from_by_uuid.entry(r.uuid.clone()).or_default();
                if !seen.iter().any(|l| *l == label) {
                    seen.push(label.clone());
                }
                all_by_uuid.entry(r.uuid.clone()).or_insert(r);
            }
        }

        // Reconstruct UnifiedResult with fused scores
        let fused: Vec<UnifiedResult> = fused_sr
            .into_iter()
            .map(|sr| {
                let mut u = all_by_uuid
                    .get(&sr.uuid)
                    .cloned()
                    .unwrap_or_else(|| UnifiedResult::from(sr.clone()));
                u.score = sr.score;
                // Une étiquette explicite est un choix de l'appelant et prime.
                // Sinon : les signaux qui ont contribué, dans l'ordre des
                // listes — `bm25+vector` se lit tout seul.
                u.signal = Some(match &self.signal {
                    Some(explicit) => explicit.clone(),
                    None => match from_by_uuid.get(&sr.uuid) {
                        Some(labels) if !labels.is_empty() => labels.join("+"),
                        _ => label_out.clone(),
                    },
                });
                u
            })
            .collect();

        ctx.set_output("results", PortValue::new(fused));
        Ok(())
    }
}

// ─── RerankNode ──────────────────────────────────────────────────────────────

/// Cross-encoder sur la tête des résultats.
///
/// Re-score les `candidates` premiers résultats avec le service `service`
/// (`Arc<dyn Reranker>`, `"reranker"` par défaut) et laisse passer la queue
/// inchangée. Sa sortie est un signal comme un autre : placé après la fusion il
/// **remplace** l'ordre ; branché sur le port `signals` d'un `FuseResultsNode`
/// avec `boost='<son étiquette>'`, il **module** l'ordre fusionné.
///
/// Il a besoin du texte des passages (chunk retrouvé, ou `_content` enrichi) :
/// s'il n'y en a aucun, ou si aucun reranker n'est configuré, il avertit et
/// laisse passer — comme `Catalog::search`.
pub struct RerankNode {
    node_name: String,
    candidates: usize,
    service: String,
    signal: Option<String>,
    keep_signal: bool,
}

impl RerankNode {
    pub const DEFAULT_CANDIDATES: usize = 20;

    pub fn new(name: &str) -> Self {
        Self {
            node_name: name.to_string(),
            candidates: Self::DEFAULT_CANDIDATES,
            service: "reranker".to_string(),
            signal: None,
            keep_signal: false,
        }
    }

    /// Taille du pool re-scoré (le reste passe inchangé).
    ///
    /// **`0` est un passe-plat exact** : ni service consulté, ni étiquette
    /// touchée, ni journal. C'est ce qui permet à un graphe figé de porter un
    /// cross-encoder que l'appelant allume ou non — un graphe-outil n'a pas de
    /// conditionnelle, mais un nœud peut avoir un zéro qui veut dire « passe ».
    pub fn with_candidates(mut self, n: usize) -> Self {
        self.candidates = n;
        self
    }

    /// Garder l'étiquette d'origine des résultats au lieu de la remplacer.
    ///
    /// Par défaut le nœud ré-étiquette (c'est ce qui permet à une fusion en
    /// aval de le reconnaître par son nom et de l'utiliser en `boost`). Dans
    /// une chaîne où le rerank est la dernière étape, la provenance —
    /// `bm25+vector` — vaut plus que le nom du dernier nœud traversé.
    pub fn with_keep_signal(mut self, keep: bool) -> Self {
        self.keep_signal = keep;
        self
    }

    /// Clé du service `Arc<dyn Reranker>` à utiliser.
    pub fn with_service(mut self, key: impl Into<String>) -> Self {
        self.service = key.into();
        self
    }

    /// Étiquette des résultats (défaut : le nom du nœud).
    pub fn with_signal(mut self, signal: impl Into<String>) -> Self {
        self.signal = Some(signal.into());
        self
    }
}

impl Node for RerankNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "RerankNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "candidates": self.candidates,
            "service": self.service,
            "signal": self.signal,
            "keep_signal": self.keep_signal,
        })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                name: "results",
                port_type: PortType::Results,
                required: true,
            },
            PortDef {
                name: "query",
                port_type: PortType::Query,
                required: true,
            },
        ]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "results",
            port_type: PortType::Results,
            required: false,
        }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut results = ctx.take_input("results")
            .and_then(|pv| take_or_clone::<Vec<UnifiedResult>>(pv))
            .ok_or("RerankNode: missing 'results' input")?;
        let qp = ctx.take_input("query")
            .and_then(|pv| take_or_clone::<QueryPayload>(pv))
            .ok_or("RerankNode: missing 'query' input")?;

        // Zéro candidat : on ne fait rien, et on ne dit rien non plus. Pas de
        // service consulté, pas d'avertissement, pas d'étiquette changée —
        // sinon un outil qui porte un cross-encoder éteint remplirait ses
        // journaux d'une absence voulue.
        if self.candidates == 0 {
            ctx.set_output("results", PortValue::new(results));
            return Ok(());
        }
        let label = self.signal.clone().unwrap_or_else(|| self.node_name.clone());

        let reranker = ctx.service::<Arc<dyn Reranker>>(&self.service).cloned();
        let Some(reranker) = reranker else {
            ctx.warn(&format!(
                "RerankNode: aucun service '{}' — ordre d'entrée conservé",
                self.service
            ));
            if !self.keep_signal {
                retag(&mut results, &label);
            }
            ctx.set_output("results", PortValue::new(results));
            return Ok(());
        };

        let pool = self.candidates.min(results.len());
        let tail = results.split_off(pool);
        let passages: Vec<String> = results
            .iter()
            .map(|u| passage_text(&SearchResult::from(u.clone())))
            .collect();

        if !passages.is_empty() && passages.iter().all(|p| p.is_empty()) {
            ctx.warn("RerankNode: aucun texte de passage disponible (ni chunk, ni _content) — ordre d'entrée conservé");
            results.extend(tail);
            if !self.keep_signal {
                retag(&mut results, &label);
            }
            ctx.set_output("results", PortValue::new(results));
            return Ok(());
        }

        match reranker.rerank(&qp.query, &passages) {
            Ok(scores) if scores.len() == results.len() => {
                let mut idx: Vec<usize> = (0..results.len()).collect();
                idx.sort_by(|&a, &b| {
                    scores[b]
                        .partial_cmp(&scores[a])
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(a.cmp(&b))
                });
                let mut reordered: Vec<UnifiedResult> = idx
                    .into_iter()
                    .map(|i| {
                        let mut r = results[i].clone();
                        r.score = scores[i] as f64;
                        r
                    })
                    .collect();
                ctx.metric("reranked", reordered.len() as f64);
                reordered.extend(tail);
                results = reordered;
            }
            Ok(scores) => {
                ctx.warn(&format!(
                    "RerankNode ({}): {} scores pour {} passages — ordre d'entrée conservé",
                    reranker.name(),
                    scores.len(),
                    results.len()
                ));
                results.extend(tail);
            }
            Err(e) => {
                ctx.warn(&format!(
                    "RerankNode ({}): {e} — ordre d'entrée conservé",
                    reranker.name()
                ));
                results.extend(tail);
            }
        }
        if !self.keep_signal {
            retag(&mut results, &label);
        }
        ctx.set_output("results", PortValue::new(results));
        Ok(())
    }
}

// ─── ResolveParentNode ───────────────────────────────────────────────────────

/// Resolves chunk results → parent entities with data enrichment.
///
/// Takes `results` and optionally `query` (for the SearchTarget). If no query
/// input is provided, the SearchTarget must be registered as a service.
pub struct ResolveParentNode {
    node_name: String,
    return_fields: Vec<String>,
}

impl ResolveParentNode {
    pub fn new(name: &str) -> Self {
        Self {
            node_name: name.to_string(),
            return_fields: vec![],
        }
    }

    pub fn with_return_fields(mut self, fields: Vec<String>) -> Self {
        self.return_fields = fields;
        self
    }
}


impl Node for ResolveParentNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "ResolveParentNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(if self.return_fields.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::json!({ "return_fields": self.return_fields })
        }))
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                name: "results",
                port_type: PortType::Results,
                required: true,
            },
            PortDef {
                name: "query",
                port_type: PortType::Query,
                required: false,
            },
        ]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "results",
            port_type: PortType::Results,
            required: false,
        }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let results = ctx.take_input("results")
            .and_then(|pv| take_or_clone::<Vec<UnifiedResult>>(pv))
            .ok_or("ResolveParentNode: missing 'results' input")?;

        // Get SearchTarget from query input
        let qp = ctx.take_input("query")
            .and_then(|pv| take_or_clone::<QueryPayload>(pv))
            .ok_or("ResolveParentNode: no 'query' input with resolved SearchTarget")?;
        let target = qp.target
            .ok_or("ResolveParentNode: Query has no resolved SearchTarget")?;

        if results.is_empty() {
            ctx.set_output("results", PortValue::new(Vec::<UnifiedResult>::new()));
            return Ok(());
        }

        let conn = ctx
            .service::<ConnService>("conn")
            .ok_or("ResolveParentNode: 'conn' service not found")?
            .0.clone();

        let return_fields = if self.return_fields.is_empty() {
            &target.enrich_fields
        } else {
            &self.return_fields
        };

        // Results are already parent-level (resolved by upstream nodes).
        // Enrich with data fields via UUID-based lookup. L'étiquette de signal
        // ne survit pas au passage par `SearchResult` : on la garde à part.
        let signals: Vec<Option<String>> = results.iter().map(|r| r.signal.clone()).collect();
        let mut search_results: Vec<SearchResult> =
            results.into_iter().map(SearchResult::from).collect();

        enrich_results_with_data(&*conn, &target.name, return_fields, &mut search_results)
            .map_err(|e| format!("ResolveParentNode: enrich failed: {e}"))?;

        let enriched: Vec<UnifiedResult> = search_results
            .into_iter()
            .zip(signals)
            .map(|(r, signal)| {
                let mut u = UnifiedResult::from(r);
                u.signal = signal;
                u
            })
            .collect();

        ctx.set_output("results", PortValue::new(enriched));
        Ok(())
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Extract query string and resolved SearchTarget from a Query input.
fn extract_query_and_target(
    ctx: &mut NodeContext,
    node_type: &str,
) -> Result<(String, SearchTarget, crate::search::SearchOptions), String> {
    let qp = ctx.take_input("query")
        .and_then(|pv| take_or_clone::<QueryPayload>(pv))
        .ok_or_else(|| format!("{node_type}: missing 'query' input"))?;
    match qp.target {
        // Les options **voyagent avec la requête**. Elles étaient jetées ici
        // jusqu'au 27 août : un graphe composé à la main filtrait ou ne
        // filtrait pas selon le nœud branché, sans rien dire
        // (`e2e_code::the_per_signal_path_drops_the_search_options_today`).
        Some(t) => Ok((qp.query, t, qp.options)),
        None => Err(format!("{node_type}: Query has no resolved SearchTarget (use SearchSourceNode upstream)")),
    }
}

/// **Le pré-filtre du chemin par signal.**
///
/// La condition portée par la requête — celle de l'appelant, ou celle qu'un
/// domaine de travail a posée — devient des offsets lucivy. Ce n'est pas un
/// tri après coup : le jeu d'ids descend jusqu'aux résolveurs, et la
/// `doc_freq` est comptée sur le sous-ensemble. Un document score donc comme
/// si l'index ne contenait que ce qui est autorisé.
///
/// Sans catalogue dans le registre, on ne peut pas résoudre : on le **dit**
/// plutôt que de rendre un résultat trop large en silence.
fn allowed_ids_for(
    ctx: &mut NodeContext,
    node_type: &str,
    target: &SearchTarget,
    options: &crate::search::SearchOptions,
) -> Option<Vec<u64>> {
    let condition = options.filter_condition.as_ref()?;
    let Some(catalog) = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned() else {
        ctx.warn(&format!("{node_type}: un filtre est demandé mais le service 'catalog' manque — la recherche n'est pas restreinte"));
        return None;
    };
    let resolved = catalog.lock().unwrap().resolve_filter_to_ids(&target.name, condition, target);
    match resolved {
        Ok(ids) => ids,
        Err(e) => {
            ctx.warn(&format!("{node_type}: filtre non résolu ({e}) — la recherche n'est pas restreinte"));
            None
        }
    }
}

/// Take optional Results from a port, defaulting to empty vec.
fn take_results(ctx: &mut NodeContext, port: &str) -> Vec<UnifiedResult> {
    ctx.take_input(port)
        .and_then(|pv| take_or_clone::<Vec<UnifiedResult>>(pv))
        .unwrap_or_default()
}

/// **Ce que la cible déclare**, options du tour comprises.
///
/// `SearchOptions.signals` prime sur `SearchTarget.default_signals` — l'appelant
/// a le dernier mot, le schéma a le mot par défaut.
fn declares(target: &SearchTarget, options: &SearchOptions, signal: &str) -> bool {
    let signals = options.signals.unwrap_or(target.default_signals);
    match signal {
        "vector" => signals.vector(),
        "sparse" => signals.sparse(),
        _ => signals.bm25(),
    }
}

/// Finition commune des nœuds de signal : résolution vers l'entité source si
/// `SourceResolved` sur une cible qui a des références source (KB), puis
/// étiquetage. C'est la résolution vers la source qui rend deux KB fusionnables
/// — leurs lignes d'index diffèrent, leurs entités sont les mêmes.
fn finish_signal(
    ctx: &mut NodeContext,
    node_type: &str,
    target: &SearchTarget,
    mut results: Vec<SearchResult>,
    result_mode: ResultMode,
    label: &str,
) -> Result<Vec<UnifiedResult>, String> {
    if result_mode == ResultMode::SourceResolved && target.has_source_refs {
        let catalog = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog").cloned()
            .ok_or_else(|| format!("{node_type}: result_mode=source_resolved needs the 'catalog' service"))?;
        catalog
            .lock()
            .unwrap()
            .resolve_to_source_entities(&mut results)
            .map_err(|e| format!("{node_type}: source resolution failed: {e}"))?;
    }
    let mut unified: Vec<UnifiedResult> = results.into_iter().map(UnifiedResult::from).collect();
    retag(&mut unified, label);
    Ok(unified)
}

fn retag(results: &mut [UnifiedResult], label: &str) {
    for r in results {
        r.signal = Some(label.to_string());
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::connection::CypherValue;
    use crate::search::SearchOptions;

    // ── Port tests ───────────────────────────────────────────────────────

    #[test]
    fn search_source_node_ports() {
        let node = SearchSourceNode::new("src", "Product", "test", SearchOptions::default());
        assert_eq!(node.inputs().len(), 0);
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.outputs()[0].name, "query");
        assert_eq!(node.outputs()[0].port_type, PortType::Query);
        assert_eq!(node.node_type(), "SearchSourceNode");
    }

    #[test]
    fn vector_search_node_ports() {
        let node = VectorSearchNode::new("vec", 10);
        assert_eq!(node.inputs().len(), 1);
        assert_eq!(node.inputs()[0].name, "query");
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.outputs()[0].name, "results");
        assert_eq!(node.outputs()[0].port_type, PortType::Results);
        assert_eq!(node.node_type(), "VectorSearchNode");
    }

    #[test]
    fn bm25_search_node_ports() {
        let node = BM25SearchNode::new("bm25", 10);
        assert_eq!(node.inputs().len(), 1);
        assert_eq!(node.inputs()[0].name, "query");
        // Deux sorties : les résultats, et ce que le moteur a dit d'eux.
        assert_eq!(node.outputs().len(), 2);
        assert_eq!(node.outputs()[0].name, "results");
        assert_eq!(node.outputs()[1].name, "meta");
        assert_eq!(node.node_type(), "BM25SearchNode");
    }

    #[test]
    fn sparse_search_node_ports() {
        let node = SparseSearchNode::new("sparse", 10);
        assert_eq!(node.inputs().len(), 1);
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.node_type(), "SparseSearchNode");
    }

    #[test]
    fn fuse_results_node_ports() {
        let node = FuseResultsNode::new("fuse");
        assert_eq!(node.inputs().len(), 4);
        assert_eq!(node.inputs()[0].name, "vector");
        assert_eq!(node.inputs()[1].name, "bm25");
        assert_eq!(node.inputs()[2].name, "sparse");
        assert_eq!(node.inputs()[3].name, "signals");
        assert!(node.inputs().iter().all(|p| !p.required));
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.outputs()[0].name, "results");
        assert_eq!(node.node_type(), "FuseResultsNode");
    }

    #[test]
    fn resolve_parent_node_ports() {
        let node = ResolveParentNode::new("resolve");
        assert_eq!(node.inputs().len(), 2);
        assert_eq!(node.inputs()[0].name, "results");
        assert_eq!(node.inputs()[0].port_type, PortType::Results);
        assert_eq!(node.inputs()[1].name, "query");
        assert_eq!(node.inputs()[1].port_type, PortType::Query);
        assert!(!node.inputs()[1].required);
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.node_type(), "ResolveParentNode");
    }

    // ── Functional tests ─────────────────────────────────────────────────

    fn make_unified_result(uuid: &str, score: f64) -> UnifiedResult {
        UnifiedResult {
            signal: None,
            uuid: uuid.into(),
            score,
            entity: Some("TestEntity".into()),
            data: Some(BTreeMap::from([(
                "_offset".into(),
                CypherValue::Int(1),
            )])),
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
        }
    }

    #[test]
    fn fuse_empty_inputs_returns_empty() {
        let mut node = FuseResultsNode::new("fuse");
        let mut ctx = NodeContext::new();
        // No inputs set — all empty

        node.execute(&mut ctx).unwrap();

        let outputs = ctx.drain_outputs();
        let results = outputs.get("results")
            .and_then(|pv| pv.downcast::<Vec<UnifiedResult>>())
            .expect("expected Results output");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn fuse_single_input_passthrough() {
        let mut node = FuseResultsNode::new("fuse");
        let mut ctx = NodeContext::new();

        ctx.set_input(
            "bm25",
            PortValue::new(vec![
                make_unified_result("a", 0.9),
                make_unified_result("b", 0.7),
            ]),
        );

        node.execute(&mut ctx).unwrap();

        let outputs = ctx.drain_outputs();
        let results = outputs.get("results")
            .and_then(|pv| pv.downcast::<Vec<UnifiedResult>>())
            .expect("expected Results output");
        assert_eq!(results.len(), 2);
        // Single input → passthrough, scores re-ranked by RRF
        assert_eq!(results[0].uuid, "a");
        assert_eq!(results[1].uuid, "b");
    }

    #[test]
    fn fuse_two_inputs_merges() {
        let mut node = FuseResultsNode::new("fuse");
        let mut ctx = NodeContext::new();

        ctx.set_input(
            "vector",
            PortValue::new(vec![
                make_unified_result("a", 0.9),
                make_unified_result("c", 0.5),
            ]),
        );
        ctx.set_input(
            "bm25",
            PortValue::new(vec![
                make_unified_result("b", 0.8),
                make_unified_result("a", 0.6),
            ]),
        );

        node.execute(&mut ctx).unwrap();

        let outputs = ctx.drain_outputs();
        let results = outputs.get("results")
            .and_then(|pv| pv.downcast::<Vec<UnifiedResult>>())
            .expect("expected Results output");
        // "a" appears in both → highest fused score
        assert!(results.len() >= 2);
        // "a" should be first (appears in both signals)
        assert_eq!(results[0].uuid, "a");
    }

    fn tagged(uuid: &str, score: f64, signal: &str) -> UnifiedResult {
        let mut r = make_unified_result(uuid, score);
        r.signal = Some(signal.into());
        r
    }

    fn results_of(ctx: &mut NodeContext) -> Vec<UnifiedResult> {
        ctx.drain_outputs()
            .get("results")
            .and_then(|pv| pv.downcast::<Vec<UnifiedResult>>())
            .expect("expected Results output")
            .clone()
    }

    /// **Le cross-encoder éteint est un passe-plat exact.**
    ///
    /// C'est ce qui permet à `search` — un graphe figé, sans conditionnelle —
    /// de porter un `RerankNode` que l'appelant allume au coup par coup avec
    /// `rerank=N`. Éteint, il ne consulte pas le service, ne ré-étiquette
    /// rien, et n'écrit pas dans les journaux : une absence voulue n'est pas
    /// un incident.
    #[test]
    fn a_cross_encoder_at_zero_changes_nothing_at_all() {
        let mut ctx = NodeContext::new();
        ctx.set_input("results", PortValue::new(vec![tagged("a", 0.9, "bm25+vector"), tagged("b", 0.5, "vector")]));
        ctx.set_input("query", PortValue::new(QueryPayload {
            target_name: "Product".into(),
            query: "comment un nœud signale son échec".into(),
            options: SearchOptions::default(),
            target: None,
        }));
        let mut node = RerankNode::new("rerank").with_candidates(0);
        node.execute(&mut ctx).unwrap();
        let out = results_of(&mut ctx);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].uuid, "a");
        // La provenance traverse : elle n'est pas remplacée par « rerank ».
        assert_eq!(out[0].signal.as_deref(), Some("bm25+vector"));
        assert_eq!(out[1].signal.as_deref(), Some("vector"));
    }

    /// Sans service de rerank, l'ordre est conservé — et `keep_signal` décide
    /// si la provenance l'est aussi. C'est ce que `search` demande : le rerank
    /// y est la dernière étape, donc « qui a trouvé ça » vaut mieux que « quel
    /// nœud est passé en dernier ».
    #[test]
    fn keep_signal_decides_whether_provenance_survives_the_reranker() {
        let payload = || QueryPayload {
            target_name: "Product".into(),
            query: "une vraie question".into(),
            options: SearchOptions::default(),
            target: None,
        };

        let mut ctx = NodeContext::new();
        ctx.set_input("results", PortValue::new(vec![tagged("a", 0.9, "bm25+vector")]));
        ctx.set_input("query", PortValue::new(payload()));
        RerankNode::new("rerank").with_candidates(20).with_keep_signal(true).execute(&mut ctx).unwrap();
        assert_eq!(results_of(&mut ctx)[0].signal.as_deref(), Some("bm25+vector"));

        // Le défaut ré-étiquette : c'est ce qui permet à une fusion en aval de
        // reconnaître le rerank par son nom et de s'en servir en `boost`.
        let mut ctx = NodeContext::new();
        ctx.set_input("results", PortValue::new(vec![tagged("a", 0.9, "bm25+vector")]));
        ctx.set_input("query", PortValue::new(payload()));
        RerankNode::new("rerank").with_candidates(20).execute(&mut ctx).unwrap();
        assert_eq!(results_of(&mut ctx)[0].signal.as_deref(), Some("rerank"));
    }

    /// Le port `signals` regroupe par étiquette : deux branches BM25 arrivent
    /// concaténées et sont pesées séparément. Poids 0 sur une branche = elle
    /// ne compte plus.
    #[test]
    fn fuse_signals_port_groups_by_label_and_weights_apply() {
        let mut ctx = NodeContext::new();
        // Fan-in simulé : `title` puis `body`, concaténés sur un seul port.
        let mut fanned = vec![tagged("a", 0.9, "title"), tagged("b", 0.8, "title")];
        fanned.extend([tagged("c", 0.9, "body"), tagged("d", 0.8, "body")]);
        ctx.set_input("signals", PortValue::new(fanned));

        let mut node = FuseResultsNode::new("fuse")
            .with_weight("title", 1.0)
            .with_weight("body", 0.0);
        node.execute(&mut ctx).unwrap();
        let out = results_of(&mut ctx);

        assert_eq!(out.len(), 4);
        assert_eq!(out[0].uuid, "a");
        assert_eq!(out[1].uuid, "b");
        assert!(out[2].score == 0.0 && out[3].score == 0.0, "body branch weighs nothing");
        // **La provenance survit à la fusion.** Sans étiquette explicite sur le
        // nœud, un résultat sort en disant quels signaux l'ont trouvé, pas le
        // nom du nœud qui les a mêlés — sinon une trace ne peut plus répondre
        // à « est-ce le plein texte ou le vecteur qui a vu ça ? ».
        assert_eq!(out[0].signal.as_deref(), Some("title"));
        assert_eq!(out[2].signal.as_deref(), Some("body"));

        // Un même document trouvé des deux côtés les porte tous les deux.
        let mut ctx = NodeContext::new();
        ctx.set_input("bm25", PortValue::new(vec![tagged("a", 0.9, "bm25")]));
        ctx.set_input("vector", PortValue::new(vec![tagged("a", 0.7, "vector")]));
        let mut node = FuseResultsNode::new("fuse");
        node.execute(&mut ctx).unwrap();
        assert_eq!(results_of(&mut ctx)[0].signal.as_deref(), Some("vector+bm25"));

        // Une étiquette demandée reste un choix de l'appelant et prime.
        let mut ctx = NodeContext::new();
        ctx.set_input("bm25", PortValue::new(vec![tagged("a", 0.9, "bm25")]));
        let mut node = FuseResultsNode::new("fuse").with_signal("hybride");
        node.execute(&mut ctx).unwrap();
        assert_eq!(results_of(&mut ctx)[0].signal.as_deref(), Some("hybride"));
    }

    /// Une étiquette en `boost` ne participe pas à la fusion : elle module.
    /// Ici un « reranker » qui préfère `b` fait passer `b` devant `a`.
    #[test]
    fn fuse_boost_label_modulates_instead_of_fusing() {
        let mut ctx = NodeContext::new();
        ctx.set_input("bm25", PortValue::new(vec![tagged("a", 0.9, "bm25"), tagged("b", 0.5, "bm25")]));
        ctx.set_input("vector", PortValue::new(vec![tagged("a", 0.9, "vector"), tagged("b", 0.5, "vector")]));
        ctx.set_input("signals", PortValue::new(vec![tagged("b", 1.0, "rerank"), tagged("a", 0.0, "rerank")]));

        let mut node = FuseResultsNode::new("fuse").with_boost("rerank").with_weight("rerank", 5.0);
        node.execute(&mut ctx).unwrap();
        let out = results_of(&mut ctx);
        assert_eq!(out[0].uuid, "b", "boosted b overtakes a: {:?}", out.iter().map(|r| (&r.uuid, r.score)).collect::<Vec<_>>());
    }

    #[test]
    fn rerank_node_ports() {
        let node = RerankNode::new("rerank");
        assert_eq!(node.inputs().len(), 2);
        assert_eq!(node.inputs()[0].name, "results");
        assert!(node.inputs()[0].required);
        assert_eq!(node.inputs()[1].name, "query");
        assert_eq!(node.outputs()[0].name, "results");
        assert_eq!(node.node_type(), "RerankNode");
    }

    fn with_text(uuid: &str, score: f64, text: &str) -> UnifiedResult {
        let mut r = make_unified_result(uuid, score);
        r.chunk = Some(crate::search::ChunkInfo {
            uuid: format!("{uuid}-chunk"),
            text: text.into(),
            index: 0,
            score,
            start_line: 0,
            end_line: 0,
            start_char: 0,
            end_char: 0,
        });
        r
    }

    fn query_payload(q: &str) -> QueryPayload {
        QueryPayload { target_name: "T".into(), query: q.into(), options: SearchOptions::default(), target: None }
    }

    /// Le reranker re-score la tête (`candidates`) et laisse la queue en place.
    #[test]
    fn rerank_node_rescores_head_keeps_tail() {
        let mut services = super::super::services::ServiceRegistry::new();
        services.register::<Arc<dyn Reranker>>("reranker", Arc::new(crate::reranker::MockReranker));
        let mut ctx = NodeContext::with_services(Arc::new(services));
        ctx.set_input("results", PortValue::new(vec![
            with_text("a", 0.9, "nothing relevant"),
            with_text("b", 0.8, "rust memory safety"),
            with_text("c", 0.7, "rust"),
            with_text("d", 0.1, "rust memory safety too"), // hors pool
        ]));
        ctx.set_input("query", PortValue::new(query_payload("rust memory safety")));

        let mut node = RerankNode::new("rerank").with_candidates(3);
        node.execute(&mut ctx).unwrap();
        let out = results_of(&mut ctx);
        let order: Vec<&str> = out.iter().map(|r| r.uuid.as_str()).collect();
        assert_eq!(order, vec!["b", "c", "a", "d"], "head reordered, d stays last");
        assert!((out[0].score - 1.0).abs() < 1e-6, "score replaced by the reranker's");
        assert_eq!(out[0].signal.as_deref(), Some("rerank"));
    }

    /// Sans service reranker : avertissement, ordre conservé, jamais d'échec.
    #[test]
    fn rerank_node_without_service_passes_through() {
        let mut ctx = NodeContext::new();
        ctx.set_input("results", PortValue::new(vec![with_text("a", 0.9, "x"), with_text("b", 0.8, "y")]));
        ctx.set_input("query", PortValue::new(query_payload("q")));
        let mut node = RerankNode::new("rerank");
        node.execute(&mut ctx).unwrap();
        let out = results_of(&mut ctx);
        assert_eq!(out.iter().map(|r| r.uuid.as_str()).collect::<Vec<_>>(), vec!["a", "b"]);
    }

    #[test]
    fn bm25_node_builder_methods() {
        let node = BM25SearchNode::new("bm25", 20)
            .with_fuzzy(2)
            .with_result_mode(ResultMode::Detailed);
        assert_eq!(node.limit, 20);
        assert_eq!(node.fuzzy_distance, 2);
        assert!(matches!(node.result_mode, ResultMode::Detailed));
    }

    #[test]
    fn resolve_parent_with_return_fields() {
        let node = ResolveParentNode::new("resolve")
            .with_return_fields(vec!["name".into(), "description".into()]);
        assert_eq!(node.return_fields, vec!["name", "description"]);
    }
}
