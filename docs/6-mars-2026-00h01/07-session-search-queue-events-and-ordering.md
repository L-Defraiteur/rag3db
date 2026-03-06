# Session 07 — SearchQueue : événements et problème d'ordonnancement

## Ce qui est fait

### Fichiers créés/modifiés

| Fichier | État | Détail |
|---------|------|--------|
| `src/search_strategy.rs` | COMPLET | UnifiedResult, ChildSummary, SearchStrategy, ExpansionRule, source_info(). 5 tests unitaires. |
| `src/search_queue.rs` | COMPLET | SearchOp, SearchProcessor trait, SearchContext, SearchQueue round-based. SearchQueueEvent via async_broadcast. 7 tests unitaires (dont 2 pour events). |
| `src/processors.rs` | COMPLET (sauf fix ci-dessous) | PrimarySearchProcessor, ExpansionProcessor, FetchRelatedProcessor, ComposeProcessor. 6 tests unitaires. |
| `src/catalog.rs` | MODIFIÉ | `build_search_queue()` + `search_with_strategy()` associated functions avec `Arc<Mutex<Catalog>>`. |
| `src/search.rs` | MODIFIÉ | Ajouté `#[derive(Serialize)]` sur ExploreGraph, GraphNode, GraphEdge. |
| `src/lib.rs` | MODIFIÉ | 3 nouveaux modules (search_queue, search_strategy, processors) + pub use exports. |
| `tests/e2e_search_queue.rs` | EN COURS | 5 tests E2E. 3 passent (no_expansion, entity_filter, max_rounds_guard). 2 échouent (expand_has_file, child_data). |

**365 tests unitaires passent, 0 échoués.**

### SearchQueueEvent — système d'événements

Implémenté avec le même pattern que `OperationQueue` :
- `async_broadcast::broadcast(128)` avec `set_overflow(true)`
- `subscribe()` via `_inactive_rx.activate_cloned()`
- `emit()` via `try_broadcast()`

Événements définis :

```rust
pub enum SearchQueueEvent {
    Enqueued { id, op_type },
    RoundStarted { round, pending_count },
    ProcessingGroup { round, op_type, count },
    GroupCompleted { round, op_type, count, emitted_count },
    GroupFailed { round, op_type, count, error },
    Injected { round, source_op_type, ops: Vec<&'static str> },
    Completed { rounds, total_ops },
    Failed { rounds, error },
}
```

**API exposée :**
- `build_search_queue()` — crée la queue configurée, le caller peut `subscribe()` avant `process()`
- `search_with_strategy()` — wrapper haut niveau (pas de subscribe possible)

### Compromis sur les événements (à revoir)

1. **Pas de `SearchOp::summary()` dans les events** — Contrairement à `QueueEvent` qui inclut des `Vec<OpSummary>` dans ProcessingBatch/BatchCompleted, les `SearchQueueEvent` n'incluent que `op_type` et `count`. On pourrait ajouter un `Vec<String>` de summaries pour plus de détail (SearchOp a déjà une méthode `summary() -> String`).

2. **Pas d'event_sender passé aux processors** — `OperationQueue` donne un `Sender<QueueEvent>` aux processors (via `event_sender()`) pour émettre des events fins (GpuBatchCompleted, DbWriteCompleted). SearchQueue expose `event_sender()` mais aucun processor ne l'utilise encore. FetchRelatedProcessor pourrait émettre un event avec le nombre de children fetchés, le cypher exécuté, etc.

3. **Pas de timing dans les events** — Les events n'incluent pas de `duration_ms`. Il faudrait mesurer le temps de chaque processor et l'inclure dans GroupCompleted.

4. **Pas de données de contexte dans les events** — Après PrimarySearchProcessor, on pourrait émettre un event avec le nombre de root_results trouvés. Après FetchRelatedProcessor, le nombre de children par parent. Ce serait très utile pour le debug.

## Problème : ordonnancement Compose vs FetchRelated

### Constat (via events E2E)

```
Round 0: PrimarySearch → 3 results
Round 0: Expansion → émet 1 FetchRelated
Round 0: Compose → tourne MAINTENANT (children map vide → rien attaché)
Round 1: FetchRelated → peuple children map avec 2 enfants
→ Compose a déjà terminé, les enfants ne sont jamais attachés aux résultats
```

### Cause

`build_search_queue()` enqueue les 3 ops au départ :
1. PrimarySearch
2. Expansion (si rules)
3. Compose

Tous en round 0. Le round traite les ops **groupées par type dans l'ordre d'insertion**. Donc PrimarySearch → Expansion → Compose, le tout dans le même round. FetchRelated est injecté par Expansion mais n'est traité qu'au round 1, après que Compose ait déjà fini.

### Solutions possibles

**Option A : Compose injecté par FetchRelatedProcessor**
- Ne pas enqueuer Compose au départ
- FetchRelatedProcessor émet `SearchOp::Compose` après avoir peuplé les children
- Pro : simple, explicite
- Con : si pas d'expansion (no rules), Compose n'est jamais émis → il faut quand même l'enqueuer quand il n'y a pas d'expansion. Aussi, si plusieurs FetchRelated sont émis (plusieurs rules), chacun émet un Compose → multiples Compose

**Option B : Compose vérifie et se ré-enqueue**
- Compose regarde s'il y a des items pending dans la queue (FetchRelated)
- Si oui, il se ré-enqueue pour le prochain round
- Pro : robuste
- Con : Compose ne voit pas les items de la queue (il n'a accès qu'au context). Nécessiterait de passer la queue ou un flag.

**Option C : Compose enqueued après les expansions dans le flux**
- ExpansionProcessor émet [FetchRelated, ..., Compose] dans cet ordre
- Les FetchRelated sont traités en round 1, Compose en round 1 aussi mais après les FetchRelated (même round, groupés par type, FetchRelated avant Compose dans l'ordre d'insertion)
- Pro : pas de changement d'architecture
- Con : ça dépend de l'ordre d'insertion dans new_ops. Si Expansion émet [FetchRelated1, FetchRelated2, Compose], le groupement met tous les FetchRelated ensemble puis Compose → ça marche. Mais si un FetchRelated émet lui aussi un Compose, double Compose.

**Option D : Compose n'est pas un op, c'est un post-processing**
- Retirer ComposeProcessor de la queue
- Faire le compose dans `search_with_strategy()` après `queue.process()`
- Pro : le plus simple, pas de problème d'ordonnancement
- Con : casse le pattern "tout est un processor"

**Option E : Sémantique "round barrier"**
- Les ops émises par un processor ne sont traitées qu'au round suivant (c'est déjà le cas)
- Mais les ops du même round sont traitées dans l'ordre des groupes
- Compose doit être dans un round APRÈS FetchRelated
- Solution : ne pas enqueuer Compose au départ, mais le faire enqueuer par une "finalizer" logic dans la queue après chaque round s'il reste des résultats à composer

### Recommandation

**Option C** semble la plus propre sans changer l'architecture :
- Ne pas enqueuer Compose dans `build_search_queue()` quand il y a des expansions
- ExpansionProcessor émet `[...FetchRelated..., Compose]` — les FetchRelated puis Compose en dernier
- Le groupement par type dans le même round garantit que tous les FetchRelated tournent avant Compose
- Sans expansion, Compose est enqueued normalement dans `build_search_queue()`

Mais il faut aussi gérer le cas où ExpansionProcessor ne trouve aucun parent matchant (0 FetchRelated émis) → il faut quand même émettre Compose.

Donc : **ExpansionProcessor émet toujours Compose en dernier dans sa liste d'ops retournées**, que ce soit avec ou sans FetchRelated. Et `build_search_queue()` n'enqueue Compose que s'il n'y a PAS d'expansions.

### Fix tentée (à valider/revert)

J'ai commencé à faire émettre Compose par FetchRelatedProcessor (Option A partielle) dans `processors.rs` :
```rust
// After all fetches, emit Compose to attach children to results
if !context.children.is_empty() {
    return Ok(vec![SearchOp::Compose]);
}
```

**Ce changement est incomplet** — il ne gère pas le cas sans children et doit être revu selon l'option choisie.

## État du code à reprendre

- `processors.rs` a un changement partiel (FetchRelated émet Compose) → **à revoir selon la décision d'ordonnancement**
- `tests/e2e_search_queue.rs` test `strategy_expand_has_file` utilise `build_search_queue()` + `subscribe()` pour tracer les events → garder ce pattern pour le debug
- Les 3 tests qui passent (no_expansion, entity_filter, max_rounds_guard) ne sont pas affectés par le problème d'ordonnancement car ils n'ont pas d'expansion effective ou pas de children attendus
