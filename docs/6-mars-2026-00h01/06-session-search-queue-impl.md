# Session 06 — Implémentation SearchQueue Phase 1 : état et notes

## Ce qui est fait

### search_strategy.rs — CRÉÉ, compile, tests unitaires passent

Fichier : `src/search_strategy.rs` (~210 lignes)

Types créés :
- `UnifiedResult` — type plat avec tous les champs (SearchResult + relation + children + graph)
- `ChildSummary` — uuid, entity, relation, data
- `SearchStrategy` — search: SearchOptions, expansions: Vec<ExpansionRule>, max_rounds: usize
- `ExpansionRule` — relation, direction, source_entity filter, limit
- `ExpansionDirection` — Outgoing / Incoming
- `SearchStrategyResponse` — results: Vec<UnifiedResult>, meta: SearchMeta
- `source_info()` helper — extrait (entity_type, entity_uuid) que ce soit Aggregated ou SourceResolved
- `From<SearchResult> for UnifiedResult` + `From<UnifiedResult> for SearchResult`

5 tests unitaires passent (conversions, source_info aggregated/resolved/no_data, strategy default).

Module déclaré dans `lib.rs` : `pub mod search_strategy;`

### Bug à fixer : ExploreGraph n'a pas Serialize

`ExploreGraph`, `GraphNode`, `GraphEdge` dans search.rs n'ont que `#[derive(Debug, Clone)]`.
Le champ `graph: Option<ExploreGraph>` sur UnifiedResult est actuellement `#[serde(skip)]`.

**Action requise** : ajouter `#[derive(Serialize)]` à ExploreGraph, GraphNode, GraphEdge dans search.rs (lignes ~449-473). CypherValue a déjà Serialize donc ça devrait compiler. Ensuite remplacer `#[serde(skip)]` par `#[serde(skip_serializing_if = "Option::is_none")]` sur le champ graph.

## Ce qui reste à implémenter

### Décision architecture : Arc<Mutex<Catalog>>

On a choisi `Arc<tokio::sync::Mutex<Catalog>>` pour que les processors puissent appeler `catalog.search()` (nécessaire pour SearchRelated futur). `search_with_strategy()` est une **associated function**, pas une méthode `&mut self`.

Le plan a été mis à jour dans le fichier plan (`federated-dancing-firefly.md`).

### search_queue.rs — À CRÉER

Fichier : `src/search_queue.rs` (~250 lignes)

```rust
// Constantes op_type
pub const OP_PRIMARY_SEARCH: &str = "primary_search";
pub const OP_EXPANSION: &str = "expansion";
pub const OP_FETCH_RELATED: &str = "fetch_related";
pub const OP_COMPOSE: &str = "compose";

// SearchOp — 4 variantes
pub enum SearchOp {
    PrimarySearch { kb_name: String, query: String, options: SearchOptions },
    Expansion { rules: Vec<ExpansionRule> },
    FetchRelated {
        parents: Vec<(String, String)>,  // (source_uuid, result_uuid)
        relation: String,
        direction: ExpansionDirection,
        limit: usize,
    },
    Compose,
}

// SearchOpItem — wrapper simple
pub struct SearchOpItem { pub id: usize, pub op: SearchOp, pub completed: bool }

// SearchProcessor trait — plus simple que queue.rs::Processor
#[async_trait]
pub trait SearchProcessor: Send + Sync {
    async fn process(
        &self,
        items: &mut [SearchOpItem],
        context: &mut SearchContext,
    ) -> Result<Vec<SearchOp>, String>;
}

// SearchContext — état partagé entre processors
pub struct SearchContext {
    pub root_results: Vec<UnifiedResult>,
    pub children: HashMap<String, Vec<ChildSummary>>,  // source_uuid → children
    pub meta: Option<SearchMeta>,
}

// SearchQueue — round-based, pas de priorités
pub struct SearchQueue {
    items: Vec<SearchOpItem>,
    processors: HashMap<&'static str, Arc<dyn SearchProcessor>>,
    pub context: SearchContext,
    counter: usize,
    max_rounds: usize,
}
```

`process()` algo :
1. Loop : prend items pending, groupe par op_type
2. Appelle processor.process() pour chaque groupe
3. New ops retournées → ajoutées aux items
4. Guard max_rounds

SearchOp.op_type() retourne la constante str. Pas de round() method — les ops sont traitées dans l'ordre d'insertion (FIFO). L'Expansion est enqueued après PrimarySearch, FetchRelated est injecté par Expansion, Compose est enqueued en dernier.

### processors.rs — À CRÉER

Fichier : `src/processors.rs` (~300 lignes)

**PrimarySearchProcessor** :
```rust
pub struct PrimarySearchProcessor {
    catalog: Arc<tokio::sync::Mutex<Catalog>>,
}
```
- Lock catalog, appelle `catalog.search(kb_name, query, options)`
- Convertit SearchResponse.results → Vec<UnifiedResult> via From
- Stocke dans context.root_results + context.meta

**ExpansionProcessor** :
```rust
pub struct ExpansionProcessor;
```
- Pour chaque rule, filtre root_results par source_entity via `source_info()`
- Collecte (source_uuid, result_uuid) des parents matchants
- Émet SearchOp::FetchRelated par rule

**FetchRelatedProcessor** :
```rust
pub struct FetchRelatedProcessor {
    conn: Arc<dyn DbConnection>,
}
```
- Cypher UNWIND (pattern explore_relation_batch, search.rs:2410) :
  - Outgoing : `UNWIND $uuids AS uid MATCH (n {_uuid: uid})-[:REL]->(m) RETURN uid, m._uuid, label(m), m`
  - Incoming : `UNWIND $uuids AS uid MATCH (n {_uuid: uid})<-[:REL]-(m) RETURN uid, m._uuid, label(m), m`
- Parse rows → ChildSummary, stocke dans context.children[source_uuid]
- Truncate par parent si limit > 0

**ComposeProcessor** :
```rust
pub struct ComposeProcessor;
```
- Pour chaque root_result, lookup source_uuid via source_info()
- Move context.children[source_uuid] → result.other_children

### catalog.rs — MODIFIER

Ajouter associated function :
```rust
pub async fn search_with_strategy(
    catalog: Arc<tokio::sync::Mutex<Catalog>>,
    kb_name: &str,
    query: &str,
    strategy: SearchStrategy,
) -> Result<SearchStrategyResponse, CatalogError>
```

Nécessite exposer `conn` : ajouter `pub(crate) fn conn_arc(&self) -> Arc<dyn DbConnection>` ou rendre le champ `pub(crate)`.

Pipeline :
1. Crée SearchQueue
2. Register 4 processors (PrimarySearch avec catalog.clone(), Expansion, FetchRelated avec conn, Compose)
3. Enqueue PrimarySearch + Expansion (si rules) + Compose
4. queue.process().await
5. Retourne SearchStrategyResponse

### lib.rs — MODIFIER

Ajouter :
```rust
pub mod search_queue;
pub mod processors;

pub use search_strategy::{
    UnifiedResult, ChildSummary, SearchStrategy, SearchStrategyResponse,
    ExpansionRule, ExpansionDirection,
};
```

### tests/e2e_search_queue.rs — À CRÉER

5 tests E2E avec schema Directory + File + HAS_FILE (réutilise e2e_phase0b pattern) :

1. **strategy_no_expansion** — pas d'expansions, same as search(), other_children=None
2. **strategy_expand_has_file** — 1 Dir + 2 Files, expansion HAS_FILE outgoing, Dir a 2 File children
3. **strategy_entity_filter** — search "auth" → File match, expansion source_entity="Directory" → Files NOT expanded
4. **strategy_child_data** — vérifier ChildSummary.data contient champs File (name, absolute_path, body)
5. **strategy_max_rounds_guard** — max_rounds=0 → erreur

## Notes importantes

### FetchRelated utilise source_uuid, pas result.uuid

En mode Aggregated, result.uuid = index entry UUID. Pour traverser le graphe, on a besoin du source entity UUID. source_info() le résout :
- SourceResolved : result.uuid est déjà le bon
- Aggregated : lire data._source_uuid

### Le pattern UNWIND de explore_relation_batch (search.rs:2410-2478)

```rust
let cypher = format!(
    "UNWIND $uuids AS uid MATCH (n {{_uuid: uid}})-[:{relation}]->(m) \
     RETURN uid, m._uuid, label(m), m"
);
```
Retourne : (parent_uuid, child_uuid, child_entity_label, child_data_as_map).
Le `label(m)` donne le nom de la node table (= entity type).

### execute_with_params existe sur DbConnection

Signature : `async fn execute_with_params(&self, cypher: &str, params: &[QueryParam]) -> Result<QueryResult, String>`
QueryParam : `{ name: String, value: CypherValue }`
CypherValue::List(Vec<CypherValue>) pour passer la liste d'UUIDs.

### Compilation checkpoints

```bash
# Après chaque fichier :
cargo check --lib

# Tests unitaires :
cargo test --lib

# E2E :
./run_e2e.sh --test e2e_search_queue
```
