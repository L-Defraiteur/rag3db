# Session 08 — SearchQueue : revue d'architecture et gaps vers Rhai

## Ce qui est fait (Phase 1 complète)

### Architecture actuelle

```
Catalog::search_with_strategy(Arc<Mutex<Catalog>>, kb, query, strategy)
  │
  └── SearchQueue (round-based, dependency tracking, async_broadcast events)
       │
       ├── Round 0: PrimarySearchProcessor    → catalog.search() → Vec<UnifiedResult>
       ├── Round 0: ExpansionProcessor        → évalue rules, émet FetchRelated ops
       │                                        emit.all(fetch_handles).then(Compose)
       ├── Round 1: FetchRelatedProcessor     → Cypher UNWIND → ChildSummary
       │                                        DeferredResolved → enqueue Compose
       └── Round 2: ComposeProcessor          → attache children aux résultats
```

### Fichiers

| Fichier | Lignes | Rôle |
|---------|--------|------|
| `src/search_strategy.rs` | ~180 | UnifiedResult, ChildSummary, SearchStrategy, ExpansionRule, source_info() |
| `src/search_queue.rs` | ~500 | SearchQueue, Emitter, OpHandle, GroupBuilder, SearchQueueEvent, SearchProcessor trait |
| `src/processors.rs` | ~350 | PrimarySearch, Expansion, FetchRelated, Compose processors |
| `src/catalog.rs` | +65 | build_search_queue(), search_with_strategy() |
| `tests/e2e_search_queue.rs` | ~530 | 5 tests E2E (all pass) |

**367 tests unitaires + 5 E2E verts.** Commit `3c1a710b4`.

### Design Pattern : Promise-like dependencies

Le pattern clé de cette implémentation. Les processeurs n'ordonnent pas leurs ops manuellement — ils déclarent des dépendances :

```rust
// Dans ExpansionProcessor::process()
let h1 = emit.op(SearchOp::FetchRelated { ... });
let h2 = emit.op(SearchOp::FetchRelated { ... });
emit.all(vec![h1, h2]).then(SearchOp::Compose);
// → Compose est enqueued automatiquement quand h1 et h2 sont completed
```

La queue résout les dépendances après chaque round : si tous les wait_for d'un deferred group sont completed → enqueue les then_ops → DeferredResolved event.

### Design Pattern : Emitter

Les processeurs n'ont pas de return value pour les ops — tout passe par `&mut Emitter` :

```rust
#[async_trait]
pub trait SearchProcessor: Send + Sync {
    fn handles(&self) -> &[&'static str];
    async fn process(
        &self,
        ops: &[SearchOp],
        context: &mut SearchContext,
        emit: &mut Emitter,
    ) -> Result<(), String>;
}
```

L'Emitter collecte :
- `emit.op(SearchOp) → OpHandle` — ops à enqueuer
- `emit.all(handles).then(op)` — dépendances
- `emit.data(key, value)` — metadata structurée → dans BatchCompleted event

### Design Pattern : handles()

Les processeurs déclarent les op types qu'ils gèrent. La queue dispatch automatiquement :

```rust
impl SearchProcessor for FetchRelatedProcessor {
    fn handles(&self) -> &[&'static str] { &[OP_FETCH_RELATED] }
}
```

`queue.register(processor)` au lieu de `queue.register_processor(op_type, processor)`.

### Déduplication à 3 niveaux

Problème rencontré : en mode Aggregated, plusieurs index entries (TreeKB_Index) pointent vers la même entité source (ex: 3 index entries → même Directory). Sans dédup :
- FetchRelated query Cypher retourne N × les mêmes children
- ComposeProcessor attache les children au premier résultat seulement (remove vs get)

Fix appliqué :
1. **ExpansionProcessor** — déduplique parents par source_uuid (HashSet)
2. **FetchRelatedProcessor** — déduplique source_uuids dans la query UNWIND (HashSet)
3. **ComposeProcessor** — `get().cloned()` au lieu de `remove()` pour que tous les résultats partageant un source_uuid reçoivent les mêmes children

### Events (async_broadcast)

```rust
pub enum SearchQueueEvent {
    Enqueued { id, op_type },
    RoundStarted { round, pending_count },
    BatchStarted { round, op_type, count },
    BatchCompleted { round, op_type, count, emitted_count, deferred_count, metadata },
    BatchFailed { round, op_type, error },
    DeferredResolved { round, ops: Vec<&'static str> },
    Completed { rounds, total_ops },
    Failed { rounds, error },
}
```

Metadata dans BatchCompleted — chaque processeur émet des données structurées :
- PrimarySearch: `root_results=3`
- Expansion: `fetch_ops_emitted=1`
- FetchRelated: `fetched_children=2`
- Compose: `attached_children=2`

---

## Gaps vers le design Rhai (Doc 05)

### Gap 1 : Pas de callback avec résultats (STRUCTUREL)

**Priorité : HAUTE — c'est le gap le plus fondamental.**

Notre `.then()` actuel :
```rust
emit.all(handles).then(SearchOp::Compose);
// → quand les handles sont completed, enqueue Compose (statiquement)
```

Ce que le doc Rhai veut :
```rust
emit(op, { then: "on_children_found" });
// → quand l'op est completed, appelle on_children_found(context, results)
// → le callback REÇOIT les résultats et DÉCIDE quoi émettre ensuite
```

Le cas d'usage concret : le pattern **exclude**. Chercher les enfants d'une classe qui matchent la query (SearchRelated), puis fetcher les AUTRES enfants (FetchRelated avec exclude des matchés). Le callback a besoin des UUIDs trouvés par SearchRelated pour les exclure du FetchRelated.

```javascript
// Rhai — impossible sans callback avec résultats
fn on_result(result, query) {
    emit(#{ search_related: #{ origin: result, relation: "PARENT_OF", query: query } },
         #{ then: "exclude_and_fetch" });
}

fn exclude_and_fetch(context, matched_children) {
    emit(#{ fetch_related: #{
        origin: context.origin,
        relation: context.relation,
        exclude: matched_children  // ← besoin des résultats de l'étape précédente
    }});
}
```

**Impact sur l'architecture :**

Le `.then()` actuel est un "enqueue statique". Il faudrait un `.then_with(callback)` où le callback reçoit les résultats de l'op completée et peut émettre dynamiquement.

Options :
- A) `GroupBuilder::then_with(Box<dyn FnOnce(Vec<SearchOp>) -> Vec<SearchOp>>)` — callback Rust
- B) `GroupBuilder::then_with(callback_name: String)` — callback Rhai identifié par nom
- C) Les deux : `OpCallback::Internal(Box<dyn FnOnce(...)>)` | `OpCallback::Script { fn_name, ast, depth }`

Le doc 07 (suggestions ouvertes, section 3) propose exactement l'option C — un enum `OpCallback` unifié.

**Implications queue :** le processeur doit pouvoir passer ses résultats au deferred group. Actuellement `process()` écrit dans `SearchContext` (shared state). Les résultats sont "dans le contexte", pas retournés au caller. Pour un callback, il faudrait soit :
- Passer le contexte au callback (il lit ce qui a changé)
- Avoir un return value typé par op (complexe)
- Stocker les résultats intermédiaires par op ID dans le contexte

### Gap 2 : Pas de priorité

**Priorité : MOYENNE — pas bloquant pour Phase 1, nécessaire pour Phase 2+.**

Le doc envisionne :
```javascript
emit(#{ search_related: ... }, #{ priority: 10.0 }); // haute prio
emit(#{ fetch_related: ... }, #{ priority: 5.0 });   // basse prio
```

Notre queue traite les ops par round, groupées par type dans l'ordre d'insertion. Pas de priorité.

Avec le système de dépendances (`.then()`), les priorités sont moins critiques — l'ordonnancement est dicté par les dépendances, pas par les numéros. Mais les priorités restent utiles pour :
- Ordonner des ops indépendantes dans le même round
- Permettre aux scripts de contrôler l'ordre d'exécution

**Impact :** Ajouter un champ `priority: Option<f32>` sur les ops emittées. Trier les groupes par priorité dans chaque round. Peu de changement structurel.

### Gap 3 : SearchOp est un enum fermé

**Priorité : MOYENNE — impacte l'extensibilité Rhai.**

Notre enum :
```rust
pub enum SearchOp {
    PrimarySearch { ... },
    Expansion { ... },
    FetchRelated { ... },
    Compose,
}
```

Ajouter `SearchRelated` = modifier l'enum Rust + ajouter un processor + modifier `op_type()` + ajouter une constante.

Le doc Rhai veut que les scripts émettent des ops via des Maps dynamiques :
```javascript
emit(#{ search_related: #{ origin: result, relation: "PARENT_OF" } });
emit(#{ custom_op: #{ whatever: "data" } });
```

Options :
- A) Garder l'enum fermé, ajouter des variants au fur et à mesure (SearchRelated, FetchChunks, etc.)
- B) Ajouter un variant `Custom { op_type: String, data: Map }` pour les ops Rhai
- C) Remplacer l'enum par un trait object `dyn SearchOp`

**Recommandation : Option A + B.** Garder les variants typés pour les ops built-in (safety, pattern matching), ajouter `Custom(String, Map)` pour l'extensibilité Rhai. Le ScriptProcessor convertit les Maps Rhai en `SearchOp::Custom` que des processors custom peuvent handler via `handles()`.

### Gap 4 : Pas de SearchRelated

**Priorité : HAUTE pour le domaine Code — pas bloquant pour Phase 1.**

`SearchRelated` = chercher "auth" parmi les enfants de `AuthService` via PARENT_OF. C'est une **sub-search** (BM25/vector) filtrée aux enfants, pas un simple traversal graph (FetchRelated).

```rust
SearchOp::SearchRelated {
    origin: (source_uuid, result_uuid),
    relation: String,
    direction: ExpansionDirection,
    query: String,           // ← re-search avec la query
    kb_name: String,
    limit: usize,
}
```

Le processor correspondant :
1. Traverse le graph pour trouver les UUIDs enfants
2. Filtre la recherche BM25/vector à ces UUIDs (via `allowed_ids`)
3. Les résultats deviennent `matched_children` sur le parent

**Impact sur l'Emitter :** L'ExpansionProcessor devrait pouvoir émettre SearchRelated comme il émet FetchRelated. Avec les dépendances :
```rust
let h_search = emit.op(SearchOp::SearchRelated { ... });
let h_fetch = emit.op(SearchOp::FetchRelated { ... });
emit.all(vec![h_search, h_fetch]).then(SearchOp::Compose);
```

Notre architecture supporte déjà ça — il suffit d'ajouter le variant et le processor.

### Gap 5 : Pas de cache intra-drain

**Priorité : BASSE — optimisation, pas structurel.**

Doc 07 section 1 : si 3 classes matchent, chacune déclenche un `count_related()` qui fait un Cypher. Un cache `(uuid, relation) → count` dans le `SearchContext` éviterait les queries redondantes.

```rust
pub struct SearchContext {
    pub root_results: Vec<UnifiedResult>,
    pub children: HashMap<String, Vec<ChildSummary>>,
    pub meta: Option<SearchMeta>,
    // futur:
    // pub entity_cache: HashMap<String, CypherValue>,
    // pub count_cache: HashMap<(String, String), i64>,
}
```

Pas de changement d'architecture — juste ajouter des caches dans SearchContext et les utiliser dans les builtins Rhai.

---

## Compatibilité avec le design Rhai

### Ce qui marche déjà

| Feature Rhai | Support actuel | Notes |
|---|---|---|
| `emit(op)` fire-and-forget | `emit.op(SearchOp)` | Mapping direct |
| `emit.all().then()` | Oui | Promise-like dependencies |
| ScriptProcessor comme processor | `trait SearchProcessor` | Il suffit d'implémenter le trait |
| Events / observabilité | `SearchQueueEvent` + subscribe | async_broadcast, metadata |
| Extensibilité processors | `handles()` + `register()` | N'importe quel processor peut être ajouté |
| Batch processing | `process(&[SearchOp], ...)` | Le processeur choisit batch vs one-by-one |
| Metadata structurée | `emit.data(key, value)` | Dans BatchCompleted |

### Ce qui manque

| Feature Rhai | Gap | Effort estimé |
|---|---|---|
| `emit(op, {then: "fn"})` callbacks avec résultats | `.then()` est statique, pas de résultats passés | ~2j (structurel) |
| `emit(op, {priority: N})` | Pas de priorité | ~0.5j |
| Ops custom via Maps | `SearchOp` enum fermé | ~0.5j (variant Custom) |
| `SearchRelated` | Nouveau variant + processor | ~2j |
| `count_related`, `get_related` builtins | Pas de builtins graph | ~1j (avec cache) |
| `search_bm25()`, `search_vector()` builtins | Pas de builtins search | ~1j |
| Rhai engine + sandbox | Pas commencé | ~2j |
| `on_compose` hook | ComposeProcessor ne supporte pas de post-hook | ~0.5j |
| `dry_run` mode | Pas implémenté | ~0.5j |

### Ordre de priorité recommandé

```
1. SearchRelated op + processor          ← valeur immédiate pour le domaine Code
2. Callbacks avec résultats (.then_with) ← fondation pour le pattern exclude
3. Priorités sur les ops                 ← nécessaire avant Rhai
4. Variant SearchOp::Custom              ← nécessaire pour Rhai
5. Rhai engine + builtins Tier 1-2       ← scripting
6. Builtins Tier 3 (search)              ← scripts avancés
7. Cache intra-drain                     ← optimisation
8. dry_run + on_compose                  ← DX
```

Les points 1-2 sont indépendants de Rhai et apportent de la valeur immédiate.

---

## Questions ouvertes

### Q1 : Callbacks — shared state vs return value ?

Quand un callback `.then_with()` est appelé, comment reçoit-il les résultats ?

- **Option A : Shared state (SearchContext)** — le callback lit `context.matched_children`, `context.children`, etc. C'est le pattern actuel (les processeurs écrivent dans le contexte). Simple mais implicite.
- **Option B : Return value** — chaque processeur retourne un résultat typé que le callback reçoit. Plus explicite mais complexe (types différents par op).
- **Option C : Event-based** — le processeur émet un event avec ses résultats. Le callback souscrit à l'event. Over-engineered ?

### Q2 : SearchOp extensible — enum ou trait object ?

Pour supporter des ops custom (Rhai Maps), faut-il :
- Ajouter `SearchOp::Custom(String, BTreeMap<String, CypherValue>)` (simple, suffisant)
- Rendre SearchOp un trait object `Box<dyn SearchOp>` (flexible, complexe, perd le pattern matching)

### Q3 : Rhai — quand ?

Le déclaratif (ExpansionRule) couvre le cas Tree (HAS_FILE) et bientôt Code (PARENT_OF avec SearchRelated). Faut-il implémenter Rhai maintenant ou attendre un vrai besoin que le déclaratif ne couvre pas ?

Arguments pour attendre :
- Les phases 1-2 n'ont pas besoin de scripting
- Le déclaratif couvre 80-90% des cas
- On peut toujours ajouter Rhai plus tard sans casser l'API

Arguments pour maintenant :
- L'architecture est fraîche, c'est le bon moment pour intégrer
- Les callbacks avec résultats sont nécessaires pour le pattern exclude, et c'est plus propre avec un engine de scripting qu'avec des closures Rust
