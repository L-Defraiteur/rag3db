# Doc 06 — Phases d'implémentation : SearchQueue + Rhai

**Date** : 4 mars 2026
**Branche** : `feature/kb-index-architecture`
**Statut** : Plan, pas commencé
**Résume** : Docs 01, 04, 05

---

## Vue d'ensemble

```
Phase 0 : ResultMode (Doc 01)               ← prérequis, task #105
    ↓
Phase 1 : SearchQueue minimale (Doc 04)      ← infra, le plus gros morceau
    ↓
Phase 2 : Rhai core (Doc 05)                 ← scripts fire-and-forget
    ↓
Phase 3 : Rhai avancé (Doc 05)              ← then, run_parallel, priorités
    ↓
Phase 4 : Polish + presets                   ← DX, sécurité, domaines
```

Chaque phase est indépendamment shippable — le système fonctionne à chaque étape.

---

## Phase 0 : ResultMode (Doc 01)

**Prérequis de tout le reste.** Sans SourceResolved, la SearchQueue ne peut pas résoudre vers les entités sources.

### Livrables

- `ResultMode` enum (`Aggregated`, `SourceResolved`) dans `search.rs`
- `result_mode` champ dans `SearchOptions`
- `resolve_to_source_entities()` dans `catalog.rs` — quand `SourceResolved`, remplacer uuid/entity/data par ceux de l'entité source
- Dédupliquer si plusieurs index entries pointent vers la même source (garder meilleur score)

### Fichiers

| Fichier | Changement |
|---|---|
| `src/search.rs` | +ResultMode enum, +SearchOptions.result_mode |
| `src/catalog.rs` | +resolve_to_source_entities(), appel conditionnel dans search() |

### Tests

- E2E : search avec `ResultMode::SourceResolved` retourne des `Directory`/`File` au lieu de `TreeKB_Index`
- Vérifier la dédupplication (même source, scores différents → garder le max)

### Effort : ~1-2 jours

### Référence : Doc 01 (`01-design-result-mode.md`)

### Statut : task #105 (pending)

---

## Phase 1 : SearchQueue minimale (Doc 04)

**L'infra.** La queue réactive avec le déclaratif, sans scripting.

### Livrables

#### 1.1 Types de base

```rust
// search_queue.rs (nouveau fichier)

enum SearchOp {
    Search { kb: String, query: String, options: SearchOptions },
    SearchRelated { origin_uuid: String, relation: String, query: String, options: SearchOptions },
    FetchRelated { origin_uuid: String, relation: String, exclude_uuids: Vec<String>, fields: Vec<String>, limit: Option<usize> },
    FetchChunks { kb: String, origin_uuid: String, query: Option<String>, limit: usize },
    Explore { uuid: String, depth: usize, outgoing: Vec<String>, incoming: Vec<String> },
}

struct QueuedOp {
    priority: f32,
    op: SearchOp,
}

struct SearchQueue {
    ops: VecDeque<QueuedOp>,
    processors: Vec<Box<dyn SearchProcessor>>,
    results: Vec<IntermediateResult>,
    max_rounds: usize,
}
```

#### 1.2 Trait SearchProcessor

```rust
trait SearchProcessor: Send + Sync {
    fn handles(&self, op: &SearchOp) -> bool;
    async fn process(&self, op: SearchOp, context: &SearchContext) -> ProcessResult;
}

struct ProcessResult {
    results: Vec<IntermediateResult>,
    downstream: Vec<QueuedOp>,
}
```

#### 1.3 Processors built-in

| Processor | Gère | Fait quoi |
|---|---|---|
| `PrimarySearchProcessor` | `Search` | Appelle `Catalog::search()` |
| `RelatedSearchProcessor` | `SearchRelated` | Filtre par relation + `Catalog::search()` |
| `FetchRelatedProcessor` | `FetchRelated` | Cypher MATCH par relation, projection champs |
| `FetchChunksProcessor` | `FetchChunks` | Fetch chunks d'un index entry |
| `ExploreProcessor` | `Explore` | Réutilise `Catalog::explore()` existant |

#### 1.4 ExpansionProcessor (déclaratif)

```rust
struct ExpansionProcessor {
    configs: Vec<ExpansionConfig>,
}

// Après PrimarySearchProcessor, pour chaque résultat :
// - évalue les triggers (SourceEntityField, ScoreAbove, TopN, Always)
// - émet SearchRelated + FetchRelated en downstream
```

#### 1.5 ComposeProcessor + EnrichedResult

```rust
struct EnrichedResult {
    pub uuid: String,
    pub score: f64,
    pub entity: Option<String>,
    pub data: Option<BTreeMap<String, CypherValue>>,
    pub chunks: Option<Vec<AttributedChunk>>,
    pub matched_children: Option<Vec<EnrichedResult>>,
    pub other_children: Option<Vec<ChildSummary>>,
    pub graph: Option<ExploreGraph>,
}
```

Le `ComposeProcessor` assemble les `IntermediateResult` en arbre `EnrichedResult` en matchant les `origin_uuid`.

#### 1.6 API

```rust
impl Catalog {
    // Existant — inchangé
    pub async fn search(&self, kb: &str, query: &str, options: SearchOptions) -> Result<SearchResponse>;

    // Nouveau — avec stratégie d'expansion
    pub async fn search_with_strategy(&self, kb: &str, query: &str, strategy: SearchStrategy) -> Result<Vec<EnrichedResult>>;
}
```

`SearchStrategy::default()` = pas d'expansion = comportement identique à `search()`.

### Fichiers

| Fichier | Changement |
|---|---|
| `src/search_queue.rs` | **Nouveau** — SearchOp, SearchQueue, QueuedOp, ProcessResult |
| `src/processors/mod.rs` | **Nouveau** — SearchProcessor trait |
| `src/processors/primary.rs` | **Nouveau** — PrimarySearchProcessor |
| `src/processors/related.rs` | **Nouveau** — RelatedSearchProcessor + FetchRelatedProcessor |
| `src/processors/chunks.rs` | **Nouveau** — FetchChunksProcessor |
| `src/processors/expansion.rs` | **Nouveau** — ExpansionProcessor |
| `src/processors/compose.rs` | **Nouveau** — ComposeProcessor → EnrichedResult |
| `src/search.rs` | +SearchStrategy, +ExpansionConfig, +ExpansionTrigger, +EnrichedResult, +ChildSummary |
| `src/catalog.rs` | +search_with_strategy() |
| `src/lib.rs` | +mod search_queue, +mod processors |

### Tests

- Unitaire : SearchQueue drain avec des ops mockées
- Unitaire : ExpansionTrigger évaluation (SourceEntityField, ScoreAbove, TopN)
- E2E : search_with_strategy sur TreeKB avec expansion `Always` + relation `Directory_IN_TreeKB`
- E2E : search_with_strategy sans expansion = mêmes résultats que search()

### Effort : ~3-4 jours

### Référence : Doc 04 (`04-design-search-queue.md`)

---

## Phase 2 : Rhai core (Doc 05)

**Scripts fire-and-forget.** Les utilisateurs peuvent écrire des processors custom en Rhai.

### Prérequis : Phase 1

### Livrables

#### 2.1 Intégration Rhai

```toml
# Cargo.toml
[dependencies]
rhai = { version = "1.x", features = ["sync", "serde"] }
```

#### 2.2 Builtins Tier 1 + 2

| Builtin | Tier | Description |
|---|---|---|
| `emit(op)` | 1 | Enqueue une op downstream |
| `get_related(result, rel)` | 2 | Entités liées par une relation domaine |
| `count_related(result, rel)` | 2 | Compter les entités liées |
| `has_relation(result, rel)` | 2 | Vérifier si une relation existe |
| `get_chunks(kb, result)` | 2 | Chunks d'une entité (abstrait `_SOURCED` / `_Chunk`) |
| `resolve_source(kb, result)` | 2 | Index entry → entité source |
| `kb_meta(kb)` | 2 | Metadata KB (title_entity, entities, relations) |
| `query_cypher(cypher, params)` | 2 | Escape hatch : Cypher brut |

Les builtins Tier 2 sont **KB-aware** : ils abstraient les tables internes (`_Index`, `_SOURCED`, `_Chunk`). L'utilisateur n'a pas besoin de connaître le schéma interne — juste le nom de la KB et les relations domaine.

Les builtins acceptent des **result handles** (le Map passé à `on_result()`) plutôt que des UUIDs bruts.

Implémentation : `register_fn` sur le Rhai Engine, `block_in_place` pour les appels async.

#### 2.3 ScriptProcessor

```rust
struct ScriptProcessor {
    engine: Engine,
    ast: AST,
    hook: ScriptHook,  // OnResult
}
```

- S'insère dans la SearchQueue après ExpansionProcessor
- Pour chaque résultat : appelle `engine.call_fn(&ast, "on_result", (result, query))`
- Collecte les ops émises via `emit()`, les enqueue

#### 2.4 ScriptConfig dans SearchStrategy

```rust
pub struct SearchStrategy {
    pub search: SearchOptions,
    pub expansions: Vec<ExpansionConfig>,
    pub scripts: Vec<ScriptConfig>,       // ← nouveau
    pub max_rounds: usize,
}

pub struct ScriptConfig {
    pub source: ScriptSource,  // Inline(String)
    pub hook: ScriptHook,
    pub allowed_tiers: Vec<BuiltinTier>,
}
```

Pour cette phase, seul `ScriptSource::Inline` et `ScriptHook::OnResult` sont supportés.

### Fichiers

| Fichier | Changement |
|---|---|
| `Cargo.toml` | +rhai dependency |
| `src/scripting/mod.rs` | **Nouveau** — Engine setup, register builtins |
| `src/scripting/builtins.rs` | **Nouveau** — Tier 1 + 2 builtins |
| `src/processors/script.rs` | **Nouveau** — ScriptProcessor |
| `src/search.rs` | +ScriptConfig, +ScriptSource, +ScriptHook, +BuiltinTier |
| `src/search_queue.rs` | Intégrer ScriptProcessor dans la pipeline |

### Tests

- Unitaire : Rhai engine + builtins Tier 1-2 isolés
- E2E : SearchStrategy avec un script inline qui émet un `SearchRelated` → vérifie que les enfants apparaissent dans le résultat
- E2E : script qui fait `query_cypher()` pour une condition → vérifie que l'expansion est conditionnelle

### Effort : ~2 jours

### Référence : Doc 05 (`05-design-extensibilite-rhai.md`, sections 3-6, 8)

---

## Phase 3 : Rhai avancé (Doc 05)

**Priorités, callbacks, parallélisme.** Le spectre complet d'interaction avec la queue.

### Prérequis : Phase 2

### Livrables

#### 3.1 `emit(op, options)` — priorités + then

```rust
struct EmittedOp {
    op: SearchOp,
    priority: Option<f32>,
    then: Option<String>,  // nom de fonction Rhai
}
```

- Overload `emit` avec 2 args : `emit(op, #{ priority: 10.0, then: "fn_name" })`
- Quand la queue exécute une op avec `then` : appeler le callback Rhai avec les résultats
- `max_callback_depth` (défaut 3) pour éviter les chaînes infinies

#### 3.2 `run_parallel()`

```rust
engine.register_fn("run_parallel", move |ops: Array| -> Array {
    tokio::task::block_in_place(|| {
        Handle::current().block_on(async {
            let futures = ops.into_iter().map(|op| execute_builtin_async(op));
            futures::future::join_all(futures).await
        })
    })
});
```

Un seul `block_in_place`, N requêtes en parallèle via `join_all`.

#### 3.3 Builtins Tier 3

| Builtin | Description |
|---|---|
| `search_bm25(kb, query, limit)` | Recherche BM25 |
| `search_vector(kb, query, limit)` | Recherche vectorielle |

#### 3.4 Hook `on_compose`

- `ScriptHook::OnCompose` : appelé après composition, reçoit tous les résultats
- Peut réordonner, filtrer, enrichir les résultats finaux

### Fichiers

| Fichier | Changement |
|---|---|
| `src/scripting/builtins.rs` | +emit overload, +run_parallel, +Tier 3 builtins |
| `src/search_queue.rs` | +EmittedOp, +callback dispatch, +max_callback_depth |
| `src/processors/script.rs` | +OnCompose hook |
| `src/processors/compose.rs` | Appel OnCompose après composition |

### Tests

- Unitaire : emit avec priorité → ordre d'exécution correct
- E2E : script avec `then` callback → pattern exclude (search children → fetch siblings excluant les matchés)
- E2E : `run_parallel` avec 2 queries → résultats corrects
- E2E : `on_compose` qui réordonne les résultats

### Effort : ~1-2 jours

### Référence : Doc 05 (`05-design-extensibilite-rhai.md`, sections 7-8)

---

## Phase 4 : Polish + presets

**DX, sécurité, presets domaine.** Rendre le tout production-ready.

### Prérequis : Phase 3

### Livrables

#### 4.1 Sécurité

```rust
engine.set_max_operations(100_000);
engine.set_max_call_levels(32);
engine.set_max_string_size(1_000_000);
engine.set_max_array_size(10_000);
engine.set_max_map_size(1_000);
```

- Script qui échoue → log l'erreur, retourne le résultat sans enrichissement
- Compteur d'erreurs par script pour monitoring

#### 4.2 Builtins Tier 4 (opt-in)

| Builtin | Description |
|---|---|
| `call_http(url, method, body)` | Appel HTTP sortant |
| `log(message)` | Debug dans le log rag3weaver |

Nécessite `allowed_tiers: [Tier4]` explicite dans `ScriptConfig`.

#### 4.3 ScriptSource::File

```rust
ScriptSource::File(PathBuf)  // chargement depuis le FS
```

Les scripts peuvent être des fichiers `.rhai` versionnés dans le projet de l'utilisateur.

#### 4.4 Presets domaine

```rust
// Scripts embarqués via include_str!
fn code_search_strategy() -> SearchStrategy { ... }
fn document_search_strategy() -> SearchStrategy { ... }
fn mail_search_strategy() -> SearchStrategy { ... }
```

Chaque preset combine des `ExpansionConfig` déclaratifs + des scripts Rhai optionnels pour les cas plus complexes.

### Fichiers

| Fichier | Changement |
|---|---|
| `src/scripting/builtins.rs` | +Tier 4 (call_http, log) |
| `src/scripting/mod.rs` | +limites d'exécution, +gestion d'erreurs |
| `src/scripting/loader.rs` | **Nouveau** — chargement ScriptSource::File |
| `src/presets/mod.rs` | **Nouveau** — presets par domaine |
| `src/presets/code.rs` | **Nouveau** — code_search_strategy() |
| `src/presets/document.rs` | **Nouveau** — document_search_strategy() |
| `scripts/code_expansion.rhai` | **Nouveau** — script embarqué pour Code domain |

### Effort : ~2-3 jours (au fil du temps, pas nécessairement d'un bloc)

---

## Résumé

| Phase | Quoi | Effort | Dépend de | Ce qui marche après |
|---|---|---|---|---|
| **0** | ResultMode | ~1-2j | rien (task #105) | `SourceResolved` dans search() |
| **1** | SearchQueue minimale | ~3-4j | Phase 0 | Expansion déclarative, EnrichedResult |
| **2** | Rhai core | ~2j | Phase 1 | Scripts on_result, emit(), query_cypher() |
| **3** | Rhai avancé | ~1-2j | Phase 2 | then, run_parallel, priorités, on_compose |
| **4** | Polish + presets | ~2-3j | Phase 3 | Sécurité, presets domaine, fichiers .rhai |
| | **Total** | **~10-13j** | | |

Chaque phase est shippable — le système fonctionne et apporte de la valeur à chaque étape. On n'est pas obligé d'aller jusqu'à Phase 4 pour que ce soit utile : Phase 0+1 seules donnent déjà l'expansion déclarative qui couvre 80-90% des cas.
