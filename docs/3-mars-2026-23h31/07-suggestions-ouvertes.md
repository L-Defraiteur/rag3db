# Doc 07 — Suggestions ouvertes et points à clarifier

**Date** : 4 mars 2026
**Branche** : `feature/kb-index-architecture`
**Statut** : À discuter
**Contexte** : Revue critique des Docs 01-06

---

## 1. Cache intra-drain

### Le problème

Si 3 classes apparaissent dans les résultats, chacune déclenche une expansion (SearchRelated + FetchRelated via PARENT_OF). On va potentiellement querier les mêmes entités plusieurs fois dans le même drain — par exemple si deux classes partagent un enfant via une relation.

Plus concrètement : `get_entity()`, `count_related()`, `get_related()` dans les scripts Rhai peuvent être appelés pour le même UUID par plusieurs résultats.

### Solution proposée

Un cache par drain dans le `SearchContext` :

```rust
struct SearchContext {
    conn: Arc<Connection>,
    catalog: Arc<Catalog>,
    query: String,

    // Cache peuplé au fil des ops, vidé à la fin du drain
    entity_cache: RwLock<HashMap<String, CypherValue>>,   // uuid → data
    count_cache: RwLock<HashMap<(String, String), i64>>,   // (uuid, relation) → count
}

impl SearchContext {
    async fn get_entity_cached(&self, uuid: &str) -> CypherValue {
        if let Some(cached) = self.entity_cache.read().get(uuid) {
            return cached.clone();
        }
        let result = /* fetch from DB */;
        self.entity_cache.write().insert(uuid.to_string(), result.clone());
        result
    }

    fn count_related_cached(&self, uuid: &str, relation: &str) -> Option<i64> {
        self.count_cache.read().get(&(uuid.to_string(), relation.to_string())).copied()
    }
}
```

Les builtins Rhai (`get_related`, `count_related`, `get_entity`) passent par le cache automatiquement. Transparent pour l'utilisateur.

### Impact

- Pas de changement d'API
- Réduit les aller-retours DB dans les cas multi-expansion
- Le cache vit uniquement pendant un drain — pas de stale data

---

## 2. Observabilité et debugging

### Le problème

On a `QueueEvents` pour l'ingestion queue (Doc enrichi en task #100). La SearchQueue a besoin du même genre de tracing, surtout pour debugger les scripts Rhai et les chaînes de `then` callbacks.

### Solution proposée

#### 2.1 SearchQueueEvents

```rust
enum SearchQueueEvent {
    /// Une op a été ajoutée à la queue.
    OpEnqueued {
        op_type: String,          // "Search", "SearchRelated", "FetchRelated", ...
        source: String,           // "ExpansionProcessor", "script.rhai:12", "then:on_children_found"
        priority: f32,
    },

    /// Une op a été exécutée.
    OpExecuted {
        op_type: String,
        duration_ms: f64,
        result_count: usize,
    },

    /// Un callback `then` a été invoqué.
    CallbackInvoked {
        fn_name: String,
        depth: usize,
        emitted_ops: usize,       // combien d'ops le callback a émises
    },

    /// Un script a échoué.
    ScriptError {
        source: String,           // nom du fichier ou "inline"
        line: Option<usize>,
        error: String,
    },

    /// Le drain est terminé.
    DrainComplete {
        total_ops: usize,
        total_rounds: usize,
        total_duration_ms: f64,
        result_count: usize,
    },
}
```

#### 2.2 Mode dry_run

Un mode qui montre les ops sans les exécuter — utile pour développer des scripts :

```rust
pub struct SearchStrategy {
    // ...
    pub dry_run: bool,  // défaut: false
}
```

En dry_run :
- PrimarySearchProcessor exécute normalement (on a besoin des vrais résultats)
- Les ops downstream sont loguées mais pas exécutées
- Le retour inclut la liste des ops qui auraient été exécutées

```json
{
  "results": [ /* résultats primaires normaux */ ],
  "planned_ops": [
    { "type": "SearchRelated", "origin": "scope-auth", "relation": "PARENT_OF", "source": "ExpansionProcessor" },
    { "type": "FetchRelated", "origin": "scope-auth", "relation": "PARENT_OF", "source": "script.rhai:on_result" },
    { "type": "SearchRelated", "origin": "scope-mw", "relation": "IMPORTS", "source": "script.rhai:on_result" }
  ]
}
```

Permet de voir exactement ce que le script ferait, sans coût d'exécution.

#### 2.3 Script REPL (futur)

Un mode interactif pour itérer sur un script avec des vrais résultats :

```
> load script.rhai
> search ScopeKB "auth middleware"
[3 results]
> run on_result results[0]
emitted: SearchRelated { origin: scope-auth, relation: PARENT_OF }
emitted: FetchRelated { origin: scope-auth, relation: PARENT_OF }
> run on_result results[1]
(no ops emitted)
```

Pas prioritaire, mais très utile pour le dev. Pourrait être un CLI tool séparé.

---

## 3. Pattern exclude en mode déclaratif

### Le problème

Dans le Doc 04 (SearchQueue), `ExpansionConfig` a un champ `fetch_siblings: true` avec l'idée implicite que les siblings sont "les enfants moins les matchés". Mais pour exclure les matchés du FetchRelated, il faut que le SearchRelated ait fini d'abord.

En Rhai, c'est résolu par `then` — le callback reçoit les résultats du SearchRelated et émet le FetchRelated avec `exclude`.

En déclaratif, le `ExpansionProcessor` doit faire la même chose en coulisse.

### Solution proposée

L'`ExpansionProcessor` traite un `ExpansionConfig` en **deux étapes internes** :

```rust
impl ExpansionProcessor {
    async fn expand(&self, result: &SearchResult, config: &ExpansionConfig, context: &SearchContext) -> Vec<QueuedOp> {
        let mut ops = vec![];

        if config.search_children {
            // Étape 1 : émettre SearchRelated avec prio haute
            ops.push(QueuedOp {
                priority: HIGH,
                op: SearchOp::SearchRelated { /* ... */ },
                // Callback interne (pas Rhai) qui émettra FetchRelated
                internal_then: if config.fetch_siblings {
                    Some(Box::new(move |matched_results| {
                        vec![QueuedOp {
                            priority: LOW,
                            op: SearchOp::FetchRelated {
                                origin_uuid: result.uuid.clone(),
                                relation: config.relation.clone(),
                                exclude_uuids: matched_results.iter().map(|r| r.uuid.clone()).collect(),
                                fields: config.sibling_fields.clone(),
                                limit: config.sibling_limit,
                            },
                            ..
                        }]
                    }))
                } else { None },
            });
        } else if config.fetch_siblings {
            // Pas de search children → fetch tous les siblings sans exclusion
            ops.push(QueuedOp {
                priority: LOW,
                op: SearchOp::FetchRelated { exclude_uuids: vec![], /* ... */ },
                ..
            });
        }

        ops
    }
}
```

Le mécanisme de `then` (callback après exécution d'une op) existe donc aussi en interne côté Rust — pas seulement pour Rhai. Le `then` Rhai est la version exposée au script de ce même pattern interne.

### Implication

Ça veut dire que la queue doit supporter les callbacks nativement (pas juste pour Rhai). La struct `QueuedOp` a :

```rust
struct QueuedOp {
    priority: f32,
    op: SearchOp,
    callback: Option<OpCallback>,  // Rust closure OU Rhai fn_name
}

enum OpCallback {
    /// Callback Rust interne (utilisé par ExpansionProcessor).
    Internal(Box<dyn FnOnce(Vec<SearchResult>) -> Vec<QueuedOp>>),
    /// Callback Rhai (utilisé par ScriptProcessor).
    Script { fn_name: String, ast: Arc<AST>, depth: usize },
}
```

C'est un seul mécanisme unifié pour les deux layers.

---

## 4. Stratégie de déduplication

### Le problème

Avec les expansions, une même entité peut apparaître à plusieurs endroits :

- **Cas 1** : L'entité X est un résultat primaire (score 0.8) ET un matched_child de la classe Y (score 0.6)
- **Cas 2** : L'entité X apparaît dans deux expansions différentes (via PARENT_OF et via IMPORTS)
- **Cas 3** : L'entité X apparaît dans matched_children ET other_children du même parent

### Options

| Stratégie | Comportement | Quand |
|---|---|---|
| `keep_best` | Garder à la meilleure position, supprimer les autres | Défaut, le plus intuitif |
| `merge` | Merger : apparaît une fois au top level + référencé comme child | Quand le contexte hiérarchique est important |
| `allow_duplicates` | Laisser les doublons | Le client décide |

### Solution proposée

```rust
#[derive(Debug, Clone, Copy, Default)]
pub enum DedupStrategy {
    /// Garder l'occurrence avec le meilleur score. Supprimer les autres.
    #[default]
    KeepBest,
    /// Merger : garder au top level (meilleur score), mais aussi comme child reference.
    Merge,
    /// Ne pas dédupliquer. Le client gère.
    AllowDuplicates,
}

pub struct SearchStrategy {
    // ...
    pub dedup: DedupStrategy,
}
```

Pour le **Cas 3** (matched_children vs other_children du même parent) : si une entité est dans matched_children, elle ne devrait jamais apparaître dans other_children. C'est exactement le pattern `exclude` du FetchRelated — déjà géré par le mécanisme de callback (section 3).

### Quand dédupliquer

La déduplication se fait dans le `ComposeProcessor`, après que tous les résultats intermédiaires sont collectés et avant l'assemblage en arbre `EnrichedResult`.

---

## 5. Testing DX pour les scripts Rhai

### Le problème

Comment les utilisateurs testent leurs scripts `.rhai` ? Sans harness de test, le cycle de dev est : modifier le script → relancer la recherche → regarder les résultats → deviner ce qui s'est passé.

### Solution proposée

#### 5.1 Test harness Rust

```rust
/// Simule l'exécution d'un script sur des résultats fictifs.
/// Retourne les ops émises sans les exécuter.
pub fn test_script(
    script: &str,
    fake_results: Vec<SearchResult>,
    query: &str,
) -> Vec<EmittedOp> {
    let engine = setup_rhai_engine_with_mocks();
    let ast = engine.compile(script).unwrap();

    let mut all_ops = vec![];
    for result in &fake_results {
        let ops = engine.call_fn(&ast, "on_result", (result_to_map(result), query.to_string())).unwrap();
        all_ops.extend(ops);
    }
    all_ops
}

// Usage dans un test :
#[test]
fn test_container_expansion() {
    let results = vec![
        fake_result("Scope", "class", "AuthService", 0.95),
        fake_result("Scope", "function", "authMiddleware", 0.80),
    ];

    let ops = test_script(include_str!("scripts/container_expansion.rhai"), results, "auth");

    // Seul le premier résultat (class) devrait déclencher une expansion
    assert_eq!(ops.len(), 2);  // SearchRelated + FetchRelated
    assert!(ops[0].is_search_related());
    assert!(ops[1].is_fetch_related());
}
```

#### 5.2 Mocks pour les builtins

Les builtins Tier 2+ doivent être mockables pour les tests :

```rust
fn setup_rhai_engine_with_mocks() -> Engine {
    let mut engine = Engine::new();

    // count_related retourne toujours 5 (ou configurable)
    engine.register_fn("count_related", |_result: Map, _rel: String| -> i64 { 5 });

    // get_related retourne une liste vide
    engine.register_fn("get_related", |_result: Map, _rel: String| -> Array { vec![] });

    // has_relation retourne toujours true
    engine.register_fn("has_relation", |_result: Map, _rel: String| -> bool { true });

    // emit collecte dans un vec global
    // ...

    engine
}
```

Ou mieux : un `MockSearchContext` injectable qui permet de configurer les réponses attendues.

#### 5.3 Script assertions intégrées

Rhai supporte les assertions nativement :

```javascript
// test_container_expansion.rhai
// Tests intégrés au script

fn test_class_triggers_expansion() {
    let fake = #{ uuid: "test-1", entity: "Scope", score: 0.95,
                  data: #{ scope_type: "class", name: "AuthService" } };
    on_result(fake, "auth");
    // Les ops émises sont vérifiables via le test harness
}

fn test_function_no_expansion() {
    let fake = #{ uuid: "test-2", entity: "Scope", score: 0.80,
                  data: #{ scope_type: "function", name: "authMiddleware" } };
    on_result(fake, "auth");
    // Aucune op ne devrait être émise
}
```

---

## 6. Résumé et priorité

| Suggestion | Impact | Effort | Quand |
|---|---|---|---|
| **Cache intra-drain** | Perf (évite queries redondantes) | Faible (~0.5j) | Phase 1 (SearchQueue) |
| **Observabilité (events)** | DX, debugging | Moyen (~1j) | Phase 1 (SearchQueue) |
| **dry_run** | DX, dev de scripts | Faible (~0.5j) | Phase 2 (Rhai core) |
| **Exclude déclaratif unifié** | Architecture (un seul mécanisme callback) | Moyen (~1j) | Phase 1 (SearchQueue) |
| **Déduplication** | Correctness | Moyen (~1j) | Phase 1 (ComposeProcessor) |
| **Test harness** | DX, qualité des scripts | Moyen (~1j) | Phase 2 (Rhai core) |
| **Script REPL** | DX (nice to have) | Élevé (~2-3j) | Phase 4+ (futur) |

Les 4 premiers devraient être intégrés dans les phases existantes du Doc 06 — ce n'est pas du travail supplémentaire massif, c'est des raffinements qui s'intègrent naturellement dans l'implémentation.
