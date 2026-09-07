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
    embed_query, enrich_results_with_data, fuse_signals,
    search_bm25_chunked, search_sparse, search_vector, search_vector_via_backend, BM25Mode,
    FusionConfig, FusionStrategy, ResultMode, SearchOptions, SearchResult, SearchTarget,
    SignalConfig, SignalRole, DEFAULT_RRF_K,
};
use crate::search_strategy::UnifiedResult;

use super::node::{Node, NodeContext};
use super::port::{take_or_clone, PortDef, PortValue, QueryPayload};
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
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::SearchSourceNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::SearchSourceNodeFactory).1
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
        // **Le filtre hérité descend aussi.** `filters` (le `HashMap`) était
        // ignoré sur ce chemin : seul `filter_condition` était lu. Même
        // précédence que le monolithe — le précis (`filter_condition`)
        // l'emporte sur le grossier (`filters`) — B5 de la réconciliation.
        if options.filter_condition.is_none() && !options.filters.is_empty() {
            options.filter_condition = Some(options.filters.clone().into());
        }
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

        // **La consigne de cohérence, enfin appliquée sur ce chemin.** Elle
        // vivait dans le corps de `Catalog::search`, que ce graphe n'emprunte
        // pas : l'outil `search` des agents traversait donc zéro des trois
        // branches, et `Consistency::Strict` n'était construit nulle part.
        // C'est ici le seul endroit qui convienne — après la résolution de la
        // cible, avant que les signaux ne lisent quoi que ce soit.
        let (reste_en_file, partiel, mut avertissements, embedding, sparse) = {
            let mut cat = catalog.lock().unwrap();
            let mut w: Vec<String> = Vec::new();
            let (exige, attendre_ailleurs) = options.ce_qui_doit_etre_pret();
            // Bornée à la fermeture de la cible : ce que d'autres entités ont
            // en file n'attend pas cette recherche, et ne la rend pas partielle.
            let (reste, partiel) = cat.appliquer_la_consigne_pour(
                &self.target_name, exige, attendre_ailleurs, options.timeout_ms, &mut w,
            );
            let signaux = options.signals.unwrap_or(target.default_signals);
            // **L'index plein texte s'ouvre ici, paresseusement**, comme dans le
            // monolithe : c'est le seul endroit qui connaisse à la fois la
            // table, ses champs et le verrou. Un handle non ouvert était une
            // erreur dure sur ce chemin — B9 de la réconciliation.
            if signaux.bm25() && !cat.plein_texte_natif() {
                cat.ensure_fts_handle(
                    &target.parent_table,
                    &target.bm25_fields,
                    &crate::scope::fts_filter_fields(),
                );
            }
            // **La requête s'embarque une fois**, dual si les deux signaux sont
            // demandés, avec le cache du catalogue — B6. Un échec ne casse pas
            // la recherche : les nœuds savent embarquer eux-mêmes, et ça se dit.
            let (embedding, sparse) = if signaux.vector() || signaux.sparse() {
                match cat.embarquer_la_requete(&self.query, signaux.vector(), signaux.sparse()) {
                    Ok((e, s)) => ((!e.is_empty()).then_some(e), s),
                    Err(e) => {
                        w.push(format!(
                            "embarquement de la requête impossible ici ({e}) : les signaux \
                             vectoriels embarqueront eux-mêmes"
                        ));
                        (None, None)
                    }
                }
            } else {
                (None, None)
            };
            (reste, partiel, w, embedding, sparse)
        };
        for a in &avertissements {
            ctx.warn(a);
        }

        ctx.set_output(
            "query",
            PortValue::new(QueryPayload {
                target_name: self.target_name.clone(),
                query: self.query.clone(),
                options: options.clone(),
                target: Some(target.clone()),
                embedding,
                sparse,
            }),
        );

        // Le journal du nœud ne va nulle part pour un agent — c'est ce qu'on a
        // découvert avec l'avertissement de filtre du nœud vectoriel. La méta
        // est le seul canal qui remonte jusqu'à la fiche rendue, et
        // `merge_port_values` sait déjà fondre deux `SearchMeta` (les
        // avertissements se concatènent, `partial` s'ajoute par `|=`).
        ctx.set_output(
            "meta",
            PortValue::new(crate::search::SearchMeta {
                query: self.query.clone(),
                target: target.name.clone(),
                signals: crate::search::SearchSignals::NONE,
                consistency: options.consistency,
                partial: partiel,
                pending_count: reste_en_file,
                vector_count: 0,
                bm25_count: 0,
                sparse_count: 0,
                fused_count: 0,
                reranked_count: 0,
                warnings: std::mem::take(&mut avertissements),
                search_time_ms: 0,
                diagnostics: None,
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
    /// `None` : le budget vient de la requête (`budget_de_recherche`).
    limit: Option<usize>,
    result_mode: ResultMode,
    signal: Option<String>,
}

impl VectorSearchNode {
    pub fn new(name: &str, limit: usize) -> Self {
        Self::avec_limite(name, Some(limit))
    }

    /// Le nœud dont le budget vient de la requête — la forme du gabarit
    /// (`budget_de_recherche`).
    pub fn depuis_la_requete(name: &str) -> Self {
        Self::avec_limite(name, None)
    }

    fn avec_limite(name: &str, limit: Option<usize>) -> Self {
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
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::VectorSearchNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::VectorSearchNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let debut = std::time::Instant::now();
        let (query_str, target, options, vecteurs) = extract_query_and_target(ctx, "VectorSearchNode")?;
        let limite = budget_de_recherche(self.limit, &options);

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

        // Ce que l'agent doit entendre. `ctx.warn` va dans le journal du nœud,
        // que personne ne lit du côté de l'appelant : ce qui touche à la
        // justesse d'un résultat passe par la méta.
        let mut node_warnings: Vec<String> = Vec::new();

        // Le vecteur ne se pré-filtre pas par offsets — le HNSW ne connaît
        // pas nos identités — mais par du Cypher sur l'entité parente. Le
        // catalogue sait le compiler, cellule comprise.
        let (filter_where, filter_params, filter_match) = match &options.filter_condition {
            None => (None, vec![], None),
            Some(cond) => match ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned() {
                Some(catalog) => {
                    let compiled = catalog.lock().unwrap().compile_filter_for_vector(&target.parent_table, Some(cond));
                    match compiled {
                        Ok(c) => c,
                        Err(e) => {
                            node_warnings.push(format!(
                                "VectorSearchNode: filtre non compilé ({e}) — les \
                                 résultats ne sont PAS restreints au domaine demandé"
                            ));
                            (None, vec![], None)
                        }
                    }
                }
                None => {
                    node_warnings.push(
                        "VectorSearchNode: un filtre est demandé mais le service \
                         'catalog' manque — les résultats ne sont PAS restreints au \
                         domaine demandé"
                            .to_string(),
                    );
                    (None, vec![], None)
                }
            },
        };

        // Le vecteur de la requête, embarqué une fois par la source ; sinon
        // on l'embarque ici — le montage minimal des tests.
        let embedding = match vecteurs.0 {
            Some(e) => e,
            None => {
                let mut cache = HashMap::new();
                embed_query(&*embedder, &query_str, &mut cache)
                    .map_err(|e| format!("VectorSearchNode: embed failed: {e}"))?
            }
        };

        // **Le stockage du modèle courant sur cette table**, résolu depuis la
        // méta : l'index pour rag3db, la colonne pour pgvector. Un modèle
        // absent refuse en nommant ceux qui sont là — jamais zéro en silence.
        // Sans catalogue (montage minimal), les noms d'avant.
        let (index_name, column) = {
            let models = ctx
                .service::<Vec<crate::embedding_storage::EmbeddingModelEntry>>("embedding_models")
                .cloned();
            let slug = ctx.service::<String>("embedding_slug").cloned();
            match (models, slug) {
                (Some(m), Some(s)) => {
                    let entry = m.iter().find(|e| e.slug() == s).ok_or_else(|| {
                        format!("VectorSearchNode: {}", crate::embedding_storage::unavailable_message(&s, &m))
                    })?;
                    let st = crate::embedding_storage::VectorStorage::resolve(&target.chunk_table, entry);
                    (st.index, st.column)
                }
                _ => (format!("{}_vec", target.chunk_table), "embedding".to_string()),
            }
        };

        let backend = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .and_then(|c| c.lock().unwrap().search_backend());
        let chunk_results = match backend {
            Some(backend) => search_vector_via_backend(
                backend.as_ref(),
                &target.chunk_table,
                &index_name,
                &column,
                &embedding,
                limite,
                filter_where.as_deref(),
                &filter_params,
                filter_match.as_deref(),
                &mut node_warnings,
            ),
            None => search_vector(
                &*conn,
                &target.chunk_table,
                &index_name,
                &embedding,
                limite,
                filter_where.as_deref(),
                &filter_params,
                filter_match.as_deref(),
            ),
        }
        .map_err(|e| format!("VectorSearchNode: search failed: {e}"))?;

        // Resolve chunk-level results → parent-level with data enrichment
        // **Le dialecte du service, pas rag3db en dur** : sans lui, ce chemin
        // résolvait les chunks en Cypher sur PostgreSQL — B2 de la
        // réconciliation du 6 septembre 2026.
        let dialect = ctx
            .service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
            .cloned()
            .ok_or("'dialect' service not found")?;
        let results = crate::search::resolve_vector_chunks_with_dialect(
            &*conn,
            &target,
            chunk_results,
            &target.enrich_fields,
            self.result_mode,
            dialect.as_ref(),
        )
        .map_err(|e| format!("VectorSearchNode: resolve chunks failed: {e}"))?;

        let label = self.signal.clone().unwrap_or_else(|| self.node_name.clone());
        let unified = finish_signal(ctx, "VectorSearchNode", &target, results, self.result_mode, &label)?;
        for w in &node_warnings {
            ctx.warn(w);
        }
        let nombre = unified.len();

        // **Un signal muet dit pourquoi.** Zéro résultat vectoriel peut vouloir
        // dire « ça n'existe pas » ou « ce n'est pas encore embarqué ».
        // L'appelant ne peut pas distinguer, et c'est le chemin que les agents
        // empruntent. Le compte n'est fait que dans ce cas-là.
        if nombre == 0 {
            if let Some(cat) = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned() {
                if let Ok(c) = cat.lock() {
                    c.expliquer_le_silence_d_un_signal(
                        &target.chunk_table,
                        crate::search::SearchSignals::VECTOR,
                        &mut node_warnings,
                    );
                }
            }
        }

        ctx.set_output("results", PortValue::new(unified));
        ctx.set_output(
            "meta",
            PortValue::new(crate::search::SearchMeta {
                query: query_str.clone(),
                target: target.name.clone(),
                signals: crate::search::SearchSignals::VECTOR,
                consistency: options.consistency,
                partial: false,
                pending_count: 0,
                vector_count: nombre,
                bm25_count: 0,
                sparse_count: 0,
                fused_count: nombre,
                reranked_count: 0,
                warnings: node_warnings,
                search_time_ms: debut.elapsed().as_millis() as u64,
                diagnostics: None,
            }),
        );
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
    /// `None` : le budget vient de la requête (`budget_de_recherche`).
    limit: Option<usize>,
    /// `None` : celui de la requête (`options.fuzzy_distance`).
    fuzzy_distance: Option<u8>,
    result_mode: ResultMode,
    /// `None` : celui de la requête (`options.bm25_mode`, `Auto` par défaut).
    mode: Option<BM25Mode>,
    fields: Option<Vec<String>>,
    signal: Option<String>,
}

impl BM25SearchNode {
    pub fn new(name: &str, limit: usize) -> Self {
        Self::avec_limite(name, Some(limit))
    }

    /// Le nœud dont le budget vient de la requête — la forme du gabarit
    /// (`budget_de_recherche`).
    pub fn depuis_la_requete(name: &str) -> Self {
        Self::avec_limite(name, None)
    }

    fn avec_limite(name: &str, limit: Option<usize>) -> Self {
        Self {
            node_name: name.to_string(),
            limit,
            fuzzy_distance: None,
            result_mode: ResultMode::Aggregated,
            mode: None,
            fields: None,
            signal: None,
        }
    }

    pub fn with_fuzzy(mut self, distance: u8) -> Self {
        self.fuzzy_distance = Some(distance);
        self
    }

    pub fn with_result_mode(mut self, mode: ResultMode) -> Self {
        self.result_mode = mode;
        self
    }

    pub fn with_mode(mut self, mode: BM25Mode) -> Self {
        self.mode = Some(mode);
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
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::BM25SearchNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::BM25SearchNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let debut = std::time::Instant::now();
        let (query_str, target, options, _vecteurs) = extract_query_and_target(ctx, "BM25SearchNode")?;
        // Même règle que pour le vecteur et le sparse : une cible qui ne
        // déclare pas BM25 rend vide et le dit, elle ne casse pas le graphe.
        if !declares(&target, &options, "bm25") {
            ctx.warn(&format!(
                "BM25SearchNode: '{}' ne déclare pas le signal 'bm25' — aucun résultat plein texte",
                target.name
            ));
            ctx.set_output("results", PortValue::new(Vec::<UnifiedResult>::new()));
            return Ok(());
        }
        let limite = budget_de_recherche(self.limit, &options);

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
            // Le domaine de travail descend en SQL ici, pas en offsets : c'est
            // la base qui cherche. Même exigence que sur l'autre chemin — s'il
            // ne peut pas descendre, on le **dit**.
            let filtre = filtre_utilisateur_for(ctx, "BM25SearchNode", &target, &options);
            let cellule: Option<crate::scope::Scope> = options
                .scope
                .clone()
                .or_else(|| ctx.service::<crate::scope::Scope>("cellule").cloned());
            let results = crate::search::search_texte_natif(
                backend.as_ref(),
                &target,
                &query_str,
                limite,
                &target.enrich_fields,
                self.result_mode,
                // `options.scope` prime : c'est une recherche explicitement
                // dirigée vers une autre cellule. Sinon, celle du catalogue.
                cellule.as_ref().map(|s| (s.org.as_str(), s.project.as_str())),
                filtre.as_ref().map(|(j, w, _)| (j.as_str(), w.as_str())),
                filtre.as_ref().map(|(_, _, p)| p.as_slice()).unwrap_or(&[]),
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

        // Le chemin lucivy, lui, veut des offsets. Résolus ici et pas plus
        // haut : le chemin natif est déjà parti avec son propre filtre, et
        // résoudre pour rien ferait un aller de base par recherche.
        let allowed = allowed_ids_for(ctx, "BM25SearchNode", &target, &options);

        let dialect = ctx
            .service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
            .cloned()
            .ok_or("BM25SearchNode: 'dialect' service not registered")?;
        let results = search_bm25_chunked(
            &*conn,
            dialect.as_ref(),
            &target,
            &query_str,
            fields,
            // Le gabarit s'il a parlé, sinon la requête — dont les défauts sont
            // ceux du monolithe (`Auto`, distance 1). Le nœud disait
            // `Contains` et 0 : celui-là même qui rend zéro sur une phrase
            // française — B10 de la réconciliation.
            self.mode.unwrap_or(options.bm25_mode),
            self.fuzzy_distance.unwrap_or(options.fuzzy_distance),
            limite,
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
    /// `None` : le budget vient de la requête (`budget_de_recherche`).
    limit: Option<usize>,
    result_mode: ResultMode,
    signal: Option<String>,
}

impl SparseSearchNode {
    pub fn new(name: &str, limit: usize) -> Self {
        Self::avec_limite(name, Some(limit))
    }

    /// Le nœud dont le budget vient de la requête — la forme du gabarit
    /// (`budget_de_recherche`).
    pub fn depuis_la_requete(name: &str) -> Self {
        Self::avec_limite(name, None)
    }

    fn avec_limite(name: &str, limit: Option<usize>) -> Self {
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
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::SparseSearchNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::SparseSearchNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let debut = std::time::Instant::now();
        let (query_str, target, options, vecteurs) = extract_query_and_target(ctx, "SparseSearchNode")?;
        let limite = budget_de_recherche(self.limit, &options);

        // Ce que l'agent doit entendre — par la méta, pas par le journal du
        // nœud, que personne ne lit du côté de l'appelant. Même règle que
        // `VectorSearchNode`, appliquée ici avec un jour de retard : le
        // demi-portage que la réconciliation du 6 septembre a relevé.
        let mut node_warnings: Vec<String> = Vec::new();

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
        let sparse_vec = if let Some(sv) = vecteurs.1 {
            // Embarqué une fois par la source, dual si le dense était demandé
            // aussi : pas de seconde passe avant ici.
            sv
        } else if let Some(dual) = dual_emb {
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
                limite,
                &[],
                Some(ids),
            ),
            (Some(_), None) => {
                // Un filtre non appliqué touche à la justesse du résultat :
                // il passe par la méta, où l'agent l'entend.
                node_warnings.push(
                    "SparseSearchNode: un filtre est demandé mais aucun backend de \
                     recherche — les résultats ne sont PAS restreints au domaine demandé"
                        .to_string(),
                );
                search_sparse(handle, &*conn, &target.chunk_table, &sparse_vec, limite, &[])
            }
            (None, _) => search_sparse(
                handle,
                &*conn,
                &target.chunk_table,
                &sparse_vec,
                limite,
                &[], // empty fields for chunked entities (fields are on parent table)
            ),
        }
        .map_err(|e| format!("SparseSearchNode: search failed: {e}"))?;

        // Resolve chunk-level results → parent-level with data enrichment
        // **Le dialecte du service, pas rag3db en dur** : sans lui, ce chemin
        // résolvait les chunks en Cypher sur PostgreSQL — B2 de la
        // réconciliation du 6 septembre 2026.
        let dialect = ctx
            .service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
            .cloned()
            .ok_or("'dialect' service not found")?;
        let results = crate::search::resolve_vector_chunks_with_dialect(
            &*conn,
            &target,
            chunk_results,
            &target.enrich_fields,
            self.result_mode,
            dialect.as_ref(),
        )
        .map_err(|e| format!("SparseSearchNode: resolve chunks failed: {e}"))?;

        let label = self.signal.clone().unwrap_or_else(|| self.node_name.clone());
        let unified = finish_signal(ctx, "SparseSearchNode", &target, results, self.result_mode, &label)?;
        for w in &node_warnings {
            ctx.warn(w);
        }
        let nombre = unified.len();

        // **Un signal muet dit pourquoi.** Zéro résultat sparse peut vouloir
        // dire « ça n'existe pas » ou « ce n'est pas encore embarqué » ; le
        // marqueur `_sparse_hash` sait répondre, et le compte n'est fait que
        // dans ce cas-là. `Catalog::search` le faisait pour les deux signaux ;
        // ce chemin ne le faisait que pour le vecteur.
        if nombre == 0 {
            if let Some(cat) = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned() {
                if let Ok(c) = cat.lock() {
                    c.expliquer_le_silence_d_un_signal(
                        &target.chunk_table,
                        crate::search::SearchSignals::SPARSE,
                        &mut node_warnings,
                    );
                }
            }
        }

        ctx.set_output("results", PortValue::new(unified));
        ctx.set_output(
            "meta",
            PortValue::new(crate::search::SearchMeta {
                query: query_str.clone(),
                target: target.name.clone(),
                signals: crate::search::SearchSignals::SPARSE,
                consistency: options.consistency,
                partial: false,
                pending_count: 0,
                vector_count: 0,
                bm25_count: 0,
                sparse_count: nombre,
                fused_count: nombre,
                reranked_count: 0,
                warnings: node_warnings,
                search_time_ms: debut.elapsed().as_millis() as u64,
                diagnostics: None,
            }),
        );
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

    /// La configuration d'un signal, sur une base donnée.
    ///
    /// `gabarit_decide` : la base est le défaut du moteur, et ce que le
    /// gabarit a posé (poids, rôle) s'applique par-dessus. Sinon la base est
    /// une **déclaration** — celle de l'appelant ou d'une base de
    /// connaissances — et le gabarit ne la retouche pas ; seule sa
    /// troncature (`top_k`) reste, ce n'est pas un poids.
    fn signal_config(&self, label: &str, base: &FusionConfig, gabarit_decide: bool) -> SignalConfig {
        let mut cfg = base.signal_config(label);
        if gabarit_decide {
            if let Some(w) = self.weights.get(label) {
                cfg.weight = *w;
            }
            if self.boost.contains(label) {
                cfg.role = SignalRole::Boost;
            }
        }
        if self.top_k.is_some() {
            cfg.top_k = self.top_k;
        }
        cfg
    }

    /// **D'où viennent les poids.** L'appelant d'abord (`options.fusion`), puis
    /// la déclaration d'une base de connaissances (son `fusion` dans la
    /// config), puis le gabarit, puis le défaut du moteur. Le monolithe
    /// faisait `options.fusion.unwrap_or(target.default_fusion)` ; ce nœud
    /// ignorait les deux — B4 de la réconciliation du 6 septembre 2026.
    ///
    /// Une entité simple n'a pas de fusion déclarée (`default_fusion` y est le
    /// défaut du moteur) : c'est le gabarit qui décide pour elle.
    fn base_de_fusion(qp: Option<&QueryPayload>) -> (FusionConfig, bool) {
        match qp {
            Some(qp) if qp.options.fusion.is_some() => (qp.options.fusion.clone().unwrap(), false),
            Some(qp) if qp.target.as_ref().is_some_and(|t| t.has_source_refs) => {
                (qp.target.as_ref().unwrap().default_fusion.clone(), false)
            }
            _ => (FusionConfig::default(), true),
        }
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
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::FuseResultsNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::FuseResultsNodeFactory).1
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

        // La requête, si le gabarit l'a câblée : c'est elle qui dit d'où
        // viennent les poids.
        let qp = ctx.take_input("query").and_then(|pv| take_or_clone::<QueryPayload>(pv));
        let (base, gabarit_decide) = Self::base_de_fusion(qp.as_ref());
        let (strategy, rrf_k) = if gabarit_decide {
            (self.strategy, self.rrf_k)
        } else {
            (base.strategy, base.rrf_k)
        };

        // Convert UnifiedResult → SearchResult for fuse_signals()
        let lists: Vec<(Vec<SearchResult>, SignalConfig)> = groups
            .iter()
            .map(|(label, v)| {
                (
                    v.iter().cloned().map(SearchResult::from).collect(),
                    self.signal_config(label, &base, gabarit_decide),
                )
            })
            .collect();
        let borrowed: Vec<(&[SearchResult], SignalConfig)> =
            lists.iter().map(|(l, c)| (l.as_slice(), *c)).collect();
        let fused_sr = fuse_signals(&borrowed, strategy, rrf_k);

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
    /// `None` : le pool vient de la requête (`options.rerank`), `0` = passe.
    candidates: Option<usize>,
    service: String,
    signal: Option<String>,
    keep_signal: bool,
}

impl RerankNode {
    pub const DEFAULT_CANDIDATES: usize = 20;

    pub fn new(name: &str) -> Self {
        Self {
            node_name: name.to_string(),
            candidates: None,
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
        self.candidates = Some(n);
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
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::RerankNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::RerankNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut results = ctx.take_input("results")
            .and_then(|pv| take_or_clone::<Vec<UnifiedResult>>(pv))
            .ok_or("RerankNode: missing 'results' input")?;
        let qp = ctx.take_input("query")
            .and_then(|pv| take_or_clone::<QueryPayload>(pv))
            .ok_or("RerankNode: missing 'query' input")?;

        // Le pool : celui du nœud si le gabarit l'a posé, sinon celui de la
        // requête (`options.rerank`), sinon zéro.
        let candidats = self
            .candidates
            .unwrap_or_else(|| qp.options.rerank.as_ref().map(|r| r.candidates).unwrap_or(0));

        // Zéro candidat : on ne fait rien, et on ne dit rien non plus. Pas de
        // service consulté, pas d'avertissement, pas d'étiquette changée —
        // sinon un outil qui porte un cross-encoder éteint remplirait ses
        // journaux d'une absence voulue.
        if candidats == 0 {
            ctx.set_output("results", PortValue::new(results));
            return Ok(());
        }
        let label = self.signal.clone().unwrap_or_else(|| self.node_name.clone());

        // Ce que l'agent doit entendre passe par la méta, pas par le journal
        // du nœud : « aucun reranker configuré » n'atteignait jamais
        // l'appelant sur ce chemin — B7 de la réconciliation.
        let mut node_warnings: Vec<String> = Vec::new();
        let mut reranked_count = 0usize;
        let emettre_meta = |ctx: &mut NodeContext, warnings: Vec<String>, reranked: usize| {
            ctx.set_output(
                "meta",
                PortValue::new(crate::search::SearchMeta {
                    query: qp.query.clone(),
                    target: qp.target_name.clone(),
                    signals: crate::search::SearchSignals::NONE,
                    consistency: qp.options.consistency,
                    partial: false,
                    pending_count: 0,
                    vector_count: 0,
                    bm25_count: 0,
                    sparse_count: 0,
                    fused_count: 0,
                    reranked_count: reranked,
                    warnings,
                    search_time_ms: 0,
                    diagnostics: None,
                }),
            );
        };

        let reranker = ctx.service::<Arc<dyn Reranker>>(&self.service).cloned();
        let Some(reranker) = reranker else {
            node_warnings.push(format!(
                "rerank demandé, aucun reranker configuré (service '{}') — ordre de fusion conservé",
                self.service
            ));
            for w in &node_warnings {
                ctx.warn(w);
            }
            if !self.keep_signal {
                retag(&mut results, &label);
            }
            ctx.set_output("results", PortValue::new(results));
            emettre_meta(ctx, node_warnings, 0);
            return Ok(());
        };

        // **Le plancher du pool** : au moins `limit + offset`, sinon la
        // pagination coupe dans des résultats que le cross-encoder n'a pas vus.
        let pool = candidats
            .max(qp.options.limit + qp.options.offset)
            .min(results.len());
        let tail = results.split_off(pool);

        // **Le pool a besoin de texte.** Un résultat sans chunk ni `_content`
        // ne donne rien au cross-encoder ; le monolithe allait le chercher
        // avant de constituer les passages, ce nœud avertissait et laissait
        // passer.
        if let Some(target) = qp.target.as_ref() {
            if results.iter().any(|r| r.data.is_none()) && !target.enrich_fields.is_empty() {
                let backend = ctx
                    .service::<Arc<Mutex<Catalog>>>("catalog")
                    .and_then(|c| c.lock().ok().and_then(|c| c.search_backend()));
                let conn = ctx.service::<ConnService>("conn").map(|c| c.0.clone());
                let signaux: Vec<Option<String>> = results.iter().map(|r| r.signal.clone()).collect();
                let mut plats: Vec<SearchResult> =
                    results.iter().cloned().map(SearchResult::from).collect();
                let issue = match (backend, conn) {
                    (Some(b), _) => crate::search::enrich_results_with_data_via_backend(
                        b.as_ref(), &target.parent_table, &target.enrich_fields, &mut plats,
                    ),
                    (None, Some(c)) => enrich_results_with_data(
                        &*c, &target.parent_table, &target.enrich_fields, &mut plats,
                    ),
                    (None, None) => Ok(()),
                };
                match issue {
                    Ok(()) => {
                        results = plats
                            .into_iter()
                            .zip(signaux)
                            .map(|(r, sig)| {
                                let mut u = UnifiedResult::from(r);
                                u.signal = sig;
                                u
                            })
                            .collect();
                    }
                    Err(e) => node_warnings.push(format!(
                        "RerankNode: enrichissement du pool impossible ({e}) — le cross-encoder \
                         travaille sur ce qu'il a"
                    )),
                }
            }
        }

        let passages: Vec<String> = results
            .iter()
            .map(|u| passage_text(&SearchResult::from(u.clone())))
            .collect();

        if !passages.is_empty() && passages.iter().all(|p| p.is_empty()) {
            node_warnings.push(
                "RerankNode: aucun texte de passage disponible (ni chunk, ni _content) — ordre d'entrée conservé"
                    .to_string(),
            );
            for w in &node_warnings {
                ctx.warn(w);
            }
            results.extend(tail);
            if !self.keep_signal {
                retag(&mut results, &label);
            }
            ctx.set_output("results", PortValue::new(results));
            emettre_meta(ctx, node_warnings, 0);
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
                reranked_count = reordered.len();
                reordered.extend(tail);
                results = reordered;
            }
            Ok(scores) => {
                node_warnings.push(format!(
                    "RerankNode ({}): {} scores pour {} passages — ordre d'entrée conservé",
                    reranker.name(),
                    scores.len(),
                    results.len()
                ));
                results.extend(tail);
            }
            Err(e) => {
                node_warnings.push(format!(
                    "RerankNode ({}): {e} — ordre d'entrée conservé",
                    reranker.name()
                ));
                results.extend(tail);
            }
        }
        for w in &node_warnings {
            ctx.warn(w);
        }
        if !self.keep_signal {
            retag(&mut results, &label);
        }
        ctx.set_output("results", PortValue::new(results));
        emettre_meta(ctx, node_warnings, reranked_count);
        Ok(())
    }
}

// ─── PaginateNode ────────────────────────────────────────────────────────────

/// **La page demandée, et rien de plus.** `offset` puis `limit`, lus dans la
/// requête — après le rerank, avant la résolution, comme le monolithe.
///
/// Sans lui, le chemin composable rendait l'union des branches (jusqu'à
/// `2 × limit`) et `offset` était inerte — B3 de la réconciliation du
/// 6 septembre 2026. Il n'a pas de configuration : ce que la page vaut est
/// une décision de l'appelant, elle voyage avec la requête.
pub struct PaginateNode {
    node_name: String,
}

impl PaginateNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string() }
    }
}

impl Node for PaginateNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "PaginateNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::PaginateNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::PaginateNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut results = ctx.take_input("results")
            .and_then(|pv| take_or_clone::<Vec<UnifiedResult>>(pv))
            .ok_or("PaginateNode: missing 'results' input")?;
        let qp = ctx.take_input("query")
            .and_then(|pv| take_or_clone::<QueryPayload>(pv))
            .ok_or("PaginateNode: missing 'query' input")?;
        let (offset, limit) = (qp.options.offset, qp.options.limit);
        ctx.metric("avant", results.len() as f64);
        if offset >= results.len() {
            results.clear();
        } else if offset > 0 {
            results = results.split_off(offset);
        }
        results.truncate(limit);
        ctx.metric("apres", results.len() as f64);
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
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::ResolveParentNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::ResolveParentNodeFactory).1
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

        // **`parent_table`, pas `name`.** Pour une base de connaissances, le nom
        // est `MaKB` et la table qui porte les lignes est `MaKB_Index` :
        // enrichir sur le nom faisait un `MATCH (n:MaKB)` sur une table qui
        // n'existe pas. Le monolithe passait la table ; ce nœud passait le
        // nom — B1 de la réconciliation du 6 septembre 2026.
        //
        // **Et par le backend quand il y en a un** : l'enrichissement direct
        // est du Cypher, qui ne parle à aucune base SQL. Le monolithe passait
        // par `enrich_results_with_data_via_backend` ; ce nœud non, et sur
        // PostgreSQL il échouait sur `MATCH`. Sans service `catalog`, le
        // Cypher direct reste le chemin — c'est le montage minimal des tests.
        let backend = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .and_then(|c| c.lock().ok().and_then(|c| c.search_backend()));
        match backend {
            Some(b) => crate::search::enrich_results_with_data_via_backend(
                b.as_ref(), &target.parent_table, return_fields, &mut search_results,
            ),
            None => enrich_results_with_data(&*conn, &target.parent_table, return_fields, &mut search_results),
        }
        .map_err(|e| format!("ResolveParentNode: enrich failed: {e}"))?;

        // **Vers l'entité source, après la page** (`SourceResolved`), avec la
        // déduplication du monolithe : deux lignes d'index du même document
        // ne rendent qu'un document. La provenance suit par la source : c'est
        // le signal du meilleur original qui reste.
        let signals: Vec<Option<String>> = if qp.options.result_mode == ResultMode::SourceResolved
            && target.has_source_refs
        {
            let mut par_source: HashMap<String, Option<String>> = HashMap::new();
            for (r, sig) in search_results.iter().zip(signals.iter()) {
                if let Some(src) = r.data.as_ref().and_then(|d| d.get("_source_uuid")).and_then(|v| v.as_str()) {
                    par_source.entry(src.to_string()).or_insert_with(|| sig.clone());
                }
            }
            let catalog = ctx
                .service::<Arc<Mutex<Catalog>>>("catalog")
                .cloned()
                .ok_or("ResolveParentNode: result_mode=source_resolved needs the 'catalog' service")?;
            catalog
                .lock()
                .unwrap()
                .resolve_to_source_entities(&mut search_results)
                .map_err(|e| format!("ResolveParentNode: source resolution failed: {e}"))?;
            search_results
                .iter()
                .map(|r| par_source.get(&r.uuid).cloned().flatten())
                .collect()
        } else {
            signals
        };

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
/// Les vecteurs de la requête, quand la source les a embarqués.
type VecteursDeRequete = (Option<Vec<f32>>, Option<crate::sparse_index::SparseVector>);

fn extract_query_and_target(
    ctx: &mut NodeContext,
    node_type: &str,
) -> Result<(String, SearchTarget, crate::search::SearchOptions, VecteursDeRequete), String> {
    let qp = ctx.take_input("query")
        .and_then(|pv| take_or_clone::<QueryPayload>(pv))
        .ok_or_else(|| format!("{node_type}: missing 'query' input"))?;
    let vecteurs = (qp.embedding, qp.sparse);
    match qp.target {
        // Les options **voyagent avec la requête**. Elles étaient jetées ici
        // jusqu'au 27 août : un graphe composé à la main filtrait ou ne
        // filtrait pas selon le nœud branché, sans rien dire
        // (`e2e_code::the_per_signal_path_drops_the_search_options_today`).
        Some(t) => Ok((qp.query, t, qp.options, vecteurs)),
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
    // `parent_table` : la table qui porte les lignes, `MaKB_Index` pour une
    // base de connaissances — voir `ResolveParentNode`.
    let resolved = catalog.lock().unwrap().resolve_filter_to_ids(&target.parent_table, condition, target);
    match resolved {
        Ok(ids) => ids,
        Err(e) => {
            ctx.warn(&format!("{node_type}: filtre non résolu ({e}) — la recherche n'est pas restreinte"));
            None
        }
    }
}

/// Le **domaine de travail** rendu en SQL, pour le chemin texte natif.
///
/// Jumeau de [`allowed_ids_for`] : là-bas le filtre devient des offsets lucivy,
/// ici il devient une jointure et une condition que la base applique
/// elle-même. Même règle dans les deux cas — si le filtre ne peut pas
/// descendre, on l'annonce plutôt que de rendre des lignes que l'appelant
/// croyait exclues.
fn filtre_utilisateur_for(
    ctx: &mut NodeContext,
    node_type: &str,
    target: &SearchTarget,
    options: &crate::search::SearchOptions,
) -> Option<(String, String, Vec<crate::connection::QueryParam>)> {
    let condition = options.filter_condition.as_ref()?;
    let Some(catalog) = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned() else {
        ctx.warn(&format!(
            "{node_type}: un filtre est demandé mais le service 'catalog' manque — \
             la recherche n'est pas restreinte"
        ));
        return None;
    };
    let compile = catalog
        .lock()
        .unwrap()
        .compile_filter_utilisateur(&target.parent_table, Some(condition));
    match compile {
        Ok((Some(w), params, Some(j))) => Some((j, w, params)),
        Ok(_) => None,
        Err(e) => {
            ctx.warn(&format!(
                "{node_type}: filtre non compilé ({e}) — la recherche n'est pas restreinte"
            ));
            None
        }
    }
}

/// **Combien un signal va chercher.** La limite du nœud si le gabarit l'a
/// posée ; sinon le **sur-fetch** du monolithe — `(limit + offset) × 2`,
/// relevé au pool du rerank — pour que la fusion et le cross-encoder aient
/// de quoi travailler avant la pagination. B3 de la réconciliation du
/// 6 septembre 2026 : sans ça, `offset` était inerte et le pool appauvri.
pub(crate) fn budget_de_recherche(limite_du_noeud: Option<usize>, options: &SearchOptions) -> usize {
    if let Some(l) = limite_du_noeud {
        return l;
    }
    let base = (options.limit + options.offset).saturating_mul(2).max(1);
    match options.rerank {
        Some(ref rk) => base.max(rk.candidates),
        None => base,
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
    _node_type: &str,
    target: &SearchTarget,
    results: Vec<SearchResult>,
    result_mode: ResultMode,
    label: &str,
) -> Result<Vec<UnifiedResult>, String> {
    // La résolution vers l'entité **source** ne se fait plus ici, par signal
    // et avant la fusion : elle se fait dans `ResolveParentNode`, après la
    // page, comme le monolithe — sinon la déduplication par source n'était
    // pas celle de la liste rendue (B11 de la réconciliation).
    let _ = (result_mode, target, ctx);
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
    use super::super::port::PortType;
    use super::*;
    use std::collections::BTreeMap;

    use crate::connection::CypherValue;
    use crate::search::SearchOptions;

    // ── Port tests ───────────────────────────────────────────────────────

    #[test]
    fn search_source_node_ports() {
        let node = SearchSourceNode::new("src", "Product", "test", SearchOptions::default());
        assert_eq!(node.inputs().len(), 0);
        assert_eq!(node.outputs().len(), 2);
        assert_eq!(node.outputs()[0].name, "query");
        assert_eq!(node.outputs()[0].port_type, PortType::Query);
        // Le second port dit ce que la consigne de cohérence a trouvé. Sans
        // lui, un agent ne peut pas distinguer « rien à trouver » de « des
        // écritures sont encore en file ».
        assert_eq!(node.outputs()[1].name, "meta");
        assert_eq!(node.outputs()[1].port_type, PortType::Meta);
        assert_eq!(node.node_type(), "SearchSourceNode");
    }

    #[test]
    fn vector_search_node_ports() {
        let node = VectorSearchNode::new("vec", 10);
        assert_eq!(node.inputs().len(), 1);
        assert_eq!(node.inputs()[0].name, "query");
        assert_eq!(node.outputs().len(), 2);
        assert_eq!(node.outputs()[0].name, "results");
        assert_eq!(node.outputs()[0].port_type, PortType::Results);
        // Le canal des avertissements, comme sur BM25 : sans lui, « les
        // résultats ne sont PAS restreints » restait dans le journal du nœud.
        assert_eq!(node.outputs()[1].name, "meta");
        assert_eq!(node.outputs()[1].port_type, PortType::Meta);
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
        assert_eq!(node.outputs().len(), 2);
        assert_eq!(node.outputs()[0].name, "results");
        // Le canal des avertissements, comme sur le vecteur et BM25. C'est
        // par lui qu'un zéro sparse dit s'il est une dette d'embarquement ou
        // une absence — le troisième signal était le seul à ne pas le dire.
        assert_eq!(node.outputs()[1].name, "meta");
        assert_eq!(node.outputs()[1].port_type, PortType::Meta);
        assert_eq!(node.node_type(), "SparseSearchNode");
    }

    #[test]
    fn fuse_results_node_ports() {
        let node = FuseResultsNode::new("fuse");
        assert_eq!(node.inputs().len(), 5);
        assert_eq!(node.inputs()[0].name, "vector");
        assert_eq!(node.inputs()[1].name, "bm25");
        assert_eq!(node.inputs()[2].name, "sparse");
        assert_eq!(node.inputs()[3].name, "signals");
        // La requête, facultative : d'où viennent les poids (B4).
        assert_eq!(node.inputs()[4].name, "query");
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
            embedding: None,
            sparse: None,
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
            embedding: None,
            sparse: None,
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

    /// **Les poids de l'appelant priment sur ceux du gabarit.** Le gabarit
    /// pèse tout sur `bm25` ; la requête porte une fusion qui pèse tout sur
    /// `vector` : c'est la requête qui décide. Sans requête câblée, le gabarit.
    #[test]
    fn la_fusion_prend_les_poids_de_l_appelant_avant_ceux_du_gabarit() {
        use crate::search::{FusionConfig, SignalConfig};
        let monter = |avec_requete: bool| -> Vec<String> {
            let mut ctx = NodeContext::new();
            ctx.set_input("bm25", PortValue::new(vec![tagged("b", 0.9, "bm25")]));
            ctx.set_input("vector", PortValue::new(vec![tagged("v", 0.9, "vector")]));
            if avec_requete {
                let mut qp = query_payload("q");
                qp.options.fusion = Some(FusionConfig {
                    bm25: SignalConfig { weight: 0.0, ..SignalConfig::default() },
                    vector: SignalConfig { weight: 1.0, ..SignalConfig::default() },
                    ..FusionConfig::default()
                });
                ctx.set_input("query", PortValue::new(qp));
            }
            let mut node = FuseResultsNode::new("fuse")
                .with_weight("bm25", 1.0)
                .with_weight("vector", 0.0);
            node.execute(&mut ctx).unwrap();
            results_of(&mut ctx).into_iter().map(|r| r.uuid).collect()
        };
        assert_eq!(monter(false), vec!["b", "v"], "sans requête, le gabarit décide");
        assert_eq!(monter(true), vec!["v", "b"], "avec, l'appelant décide");
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
        QueryPayload { target_name: "T".into(), query: q.into(), options: SearchOptions::default(), target: None, embedding: None, sparse: None }
    }

    /// **Le budget d'un signal** : la limite du nœud si le gabarit l'a
    /// posée, sinon le sur-fetch du monolithe — `(limit + offset) × 2`,
    /// relevé au pool du rerank.
    #[test]
    fn le_budget_de_recherche_suit_la_requete_sauf_si_le_gabarit_decide() {
        let mut o = SearchOptions::default(); // limit 10, offset 0
        assert_eq!(budget_de_recherche(Some(7), &o), 7, "le gabarit décide");
        assert_eq!(budget_de_recherche(None, &o), 20, "(10 + 0) × 2");
        o.offset = 5;
        assert_eq!(budget_de_recherche(None, &o), 30, "(10 + 5) × 2");
        o.rerank = Some(crate::search::RerankOptions { candidates: 50 });
        assert_eq!(budget_de_recherche(None, &o), 50, "relevé au pool du rerank");
    }

    /// **La page, et rien de plus** : `offset` puis `limit`, lus dans la
    /// requête. Un `offset` au-delà de la liste rend vide, pas une erreur.
    #[test]
    fn paginate_node_coupe_la_page_demandee() {
        let liste = || vec![
            with_text("a", 0.9, "a"), with_text("b", 0.8, "b"),
            with_text("c", 0.7, "c"), with_text("d", 0.6, "d"),
        ];
        let page = |offset: usize, limit: usize| -> Vec<String> {
            let mut ctx = NodeContext::with_services(Arc::new(super::super::services::ServiceRegistry::new()));
            ctx.set_input("results", PortValue::new(liste()));
            let mut qp = query_payload("q");
            qp.options.offset = offset;
            qp.options.limit = limit;
            ctx.set_input("query", PortValue::new(qp));
            PaginateNode::new("page").execute(&mut ctx).unwrap();
            results_of(&mut ctx).into_iter().map(|r| r.uuid).collect()
        };
        assert_eq!(page(0, 2), vec!["a", "b"]);
        assert_eq!(page(1, 2), vec!["b", "c"]);
        assert_eq!(page(3, 10), vec!["d"]);
        assert!(page(4, 10).is_empty(), "au-delà de la liste : vide, sans erreur");
    }

    /// **« Aucun reranker configuré » atteint l'agent** : par la méta, pas
    /// par le journal du nœud. Et le pool vient de la requête quand le
    /// gabarit ne le fixe pas.
    #[test]
    fn le_rerank_dit_dans_sa_meta_qu_il_n_a_pas_de_reranker() {
        let mut ctx = NodeContext::with_services(Arc::new(super::super::services::ServiceRegistry::new()));
        ctx.set_input("results", PortValue::new(vec![with_text("a", 0.9, "x"), with_text("b", 0.8, "y")]));
        let mut qp = query_payload("q");
        qp.options.rerank = Some(crate::search::RerankOptions { candidates: 5 });
        ctx.set_input("query", PortValue::new(qp));
        RerankNode::new("rerank").execute(&mut ctx).unwrap();
        let sorties = ctx.drain_outputs();
        let meta = sorties
            .get("meta")
            .and_then(|pv| pv.downcast::<crate::search::SearchMeta>())
            .cloned()
            .expect("une méta");
        assert_eq!(meta.reranked_count, 0);
        assert!(
            meta.warnings.iter().any(|w| w.contains("aucun reranker")),
            "{:?}", meta.warnings
        );
        let n = sorties.get("results").and_then(|pv| pv.downcast::<Vec<UnifiedResult>>()).map(|v| v.len());
        assert_eq!(n, Some(2), "l'ordre d'entrée est conservé, rien n'est perdu");
    }

    /// Sans pool — ni dans le gabarit, ni dans la requête — le rerank est un
    /// passe-plat exact : pas de méta, pas de mot.
    #[test]
    fn le_rerank_sans_pool_est_un_passe_plat() {
        let mut ctx = NodeContext::with_services(Arc::new(super::super::services::ServiceRegistry::new()));
        ctx.set_input("results", PortValue::new(vec![with_text("a", 0.9, "x")]));
        ctx.set_input("query", PortValue::new(query_payload("q")));
        RerankNode::new("rerank").execute(&mut ctx).unwrap();
        let sorties = ctx.drain_outputs();
        assert!(sorties.get("meta").is_none(), "un passe-plat ne dit rien");
        let n = sorties.get("results").and_then(|pv| pv.downcast::<Vec<UnifiedResult>>()).map(|v| v.len());
        assert_eq!(n, Some(1));
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
        // **Le pool a un plancher** : au moins `limit + offset`, pour que la
        // pagination ne coupe pas dans des résultats que le cross-encoder n'a
        // pas vus. Avec le `limit` par défaut (10), les quatre seraient dans
        // le pool ; ici la page fait deux, et `candidates` (3) l'emporte.
        let mut qp = query_payload("rust memory safety");
        qp.options.limit = 2;
        ctx.set_input("query", PortValue::new(qp));

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
        assert_eq!(node.limit, Some(20));
        assert_eq!(node.fuzzy_distance, Some(2));
        assert!(matches!(node.result_mode, ResultMode::Detailed));
    }

    #[test]
    fn resolve_parent_with_return_fields() {
        let node = ResolveParentNode::new("resolve")
            .with_return_fields(vec!["name".into(), "description".into()]);
        assert_eq!(node.return_fields, vec!["name", "description"]);
    }
}
