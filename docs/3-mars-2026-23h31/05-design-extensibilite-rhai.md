# Doc 05 — Design : Extensibilité SearchQueue via Rhai

**Date** : 4 mars 2026
**Branche** : `feature/kb-index-architecture`
**Statut** : Réflexion, pas planifié
**Dépend de** : Doc 04 (SearchQueue)

---

## 1. Le problème

Rag3weaver est en Rust, consommé via Node.js natif, WASM, et C++ (Cypher). Les utilisateurs écrivent du JS/TS. Ils ne peuvent pas implémenter `trait SearchProcessor` en Rust.

Question : comment permettre aux appelants de coder **leurs propres processors** et **leurs propres ops** pour la SearchQueue, sans modifier le code Rust de rag3weaver ?

---

## 2. Options évaluées

### Option A : Déclaratif pur (enrichir ExpansionConfig)

Pousser le design du Doc 04 plus loin — les ops + triggers forment un vocabulaire :

```json
{
  "trigger": {
    "and": [
      { "source_entity_field": { "field": "scope_type", "values": ["class"] } },
      { "score_above": 0.7 },
      { "result_has_relation": "PARENT_OF" }
    ]
  },
  "emit": [
    { "search_related": { "relation": "PARENT_OF", "limit": 5 } },
    { "fetch_related": { "relation": "PARENT_OF", "fields": ["signature"], "exclude_matched": true } }
  ]
}
```

| | |
|---|---|
| **Pro** | Zéro code, sérialisable JSON, stockable en DB, fonctionne partout (Node/WASM/C++) |
| **Con** | Limité à ce qu'on pré-implémente côté triggers et ops |
| **Couverture** | ~80-90% des cas si le vocabulaire est assez riche |

### Option B : Rhai (scripting Rust-natif)

[Rhai](https://rhai.rs) — langage de script conçu pour être embarqué dans du Rust :

```javascript
fn on_result(result, query) {
    if result.entity == "Scope" && result.data.scope_type == "class" {
        emit(#{ search_related: #{ relation: "PARENT_OF", limit: 5 } });
    }
}
```

| | |
|---|---|
| **Pro** | Expressif, sandboxé, cross-plateforme (Node/WASM/C++), syntaxe JS-like |
| **Con** | Dépendance ~2MB, un langage en plus, debug plus difficile |
| **Couverture** | ~99% |

### Option C : Callbacks JS via Node.js bridge

```typescript
catalog.searchWithStrategy("ScopeKB", "auth", {
  onResult: async (result, query) => {
    if (result.data.scope_type === "class") {
      return [{ searchRelated: { relation: "PARENT_OF", limit: 5 } }];
    }
    return [];
  }
});
```

| | |
|---|---|
| **Pro** | Langage natif des utilisateurs, pas de nouveau langage |
| **Con** | Traverse la frontière Rust↔JS à chaque résultat (~1ms/call), ne marche pas en WASM/C++ |
| **Couverture** | 100% mais limité au contexte Node.js |

### Option D : WASM plugins

Les utilisateurs compilent leurs processors en WASM, rag3weaver les charge.

| | |
|---|---|
| **Pro** | Language-agnostic, sandboxé |
| **Con** | Extrêmement lourd à implémenter, mauvais DX |
| **Couverture** | 100% en théorie, irréaliste en pratique |

---

## 3. Choix : Rhai

Rhai est le meilleur compromis pour rag3weaver :

- **Cross-plateforme** : même script fonctionne en Node.js, WASM, C++
- **Sandboxé mais extensible** : par défaut rien, on expose ce qu'on veut
- **Syntaxe familière** : JS-like, pas de courbe d'apprentissage raide
- **Pur Rust** : pas de dépendance système, compile partout
- **Précédent solide** : même pattern que Lua dans Redis (EVAL), Nginx (OpenResty), game engines

Le déclaratif (Option A) reste le **Layer 1 par défaut** — on n'écrit du Rhai que quand le déclaratif ne suffit pas. Les deux cohabitent dans la même SearchStrategy.

---

## 4. Le modèle de sandbox Rhai

### 4.1 Additif, pas soustractif

Le modèle Rhai est **additif** : par défaut le script ne peut rien faire (pas de FS, pas de réseau, pas d'accès système). On **enregistre** explicitement les fonctions Rust accessibles :

```rust
let mut engine = Engine::new();

// Chaque register_fn expose UNE capacité au script
engine.register_fn("query_cypher", |cypher: &str, params: Map| -> Array { ... });
engine.register_fn("search_bm25", |kb: &str, query: &str, limit: i64| -> Array { ... });
engine.register_fn("emit", |op: Map| { ... });
```

Le script a accès à **exactement** ce qu'on expose. Si on n'expose pas `call_http`, le script ne peut pas faire de réseau. Si on l'expose, il peut — c'est notre choix.

### 4.2 Analogie : Lua dans Redis

Redis expose `redis.call()` aux scripts Lua. Avec cette seule fonction, les scripts peuvent faire n'importe quoi dans Redis. Mais ils ne peuvent pas accéder au FS ou au réseau.

Notre équivalent : on expose `query_cypher()`, `search_bm25()`, `emit()`. Avec ces 3 fonctions, un script peut implémenter n'importe quel processor. Mais il ne peut pas accéder à quoi que ce soit d'autre.

---

## 5. Builtins exposés

### 5.1 Tiers de builtins

```
┌──────────────────────────────────────────────────┐
│  Tier 4 : Externe (optionnel, opt-in)            │
│  call_http(url, method, body) → réponse          │
│  log(message)                                    │
├──────────────────────────────────────────────────┤
│  Tier 3 : Search (réutilise le moteur)           │
│  search_bm25(kb, query, limit) → résultats       │
│  search_vector(kb, query, limit) → résultats     │
├──────────────────────────────────────────────────┤
│  Tier 2 : Graph haut-niveau (KB-aware)           │
│  get_related(result, rel) → entités liées        │
│  count_related(result, rel) → int                │
│  has_relation(result, rel) → bool                │
│  get_chunks(kb, result) → chunks                 │
│  resolve_source(kb, result) → entité source      │
│  kb_meta(kb) → metadata KB                       │
│  query_cypher(cypher, params) → escape hatch     │
├──────────────────────────────────────────────────┤
│  Tier 1.5 : Parallélisme (orchestration)         │
│  run_parallel(ops) → résultats groupés           │
├──────────────────────────────────────────────────┤
│  Tier 1 : Ops (toujours disponible)              │
│  emit(op) → fire-and-forget                      │
│  emit(op, {priority, then}) → avec options       │
│  result.uuid, result.score, result.data, ...     │
└──────────────────────────────────────────────────┘
```

### 5.2 Détail des builtins

```rust
// === Tier 1 : Ops ===

/// Enqueue une op downstream dans la SearchQueue.
/// L'op est un Map Rhai avec une clé = type d'op.
/// Le script n'attend pas le résultat — la queue l'exécutera plus tard.
fn emit(op: Map);

/// Variante avec options : priorité custom et/ou callback.
/// options.priority : f64 — priorité custom (plus haut = exécuté en premier)
/// options.then : String — nom de la fonction Rhai à appeler avec les résultats
fn emit(op: Map, options: Map);

// Le résultat courant est passé en argument à on_result()
// result.uuid      → String
// result.score     → f64
// result.entity    → String
// result.data      → Map (champs de l'entité)


// === Tier 1.5 : Parallélisme ===

/// Exécute plusieurs builtins en parallèle et retourne tous les résultats.
/// Chaque entrée est un Map avec une clé = nom du builtin à appeler.
/// Utile quand le script a besoin des résultats pour prendre une décision.
fn run_parallel(ops: Array) -> Array;


// === Tier 2 : Graph haut-niveau (KB-aware) ===
// Tous les builtins Tier 2 acceptent un result handle (pas un UUID brut).
// Le handle est le résultat passé à on_result() ou retourné par les builtins.

/// Entités liées par une relation domaine.
/// direction : "outgoing" (défaut), "incoming", ou "both".
fn get_related(result: Map, relation: String) -> Array;
fn get_related(result: Map, relation: String, direction: String) -> Array;

/// Compter les entités liées.
/// Même support de direction que get_related.
fn count_related(result: Map, relation: String) -> i64;
fn count_related(result: Map, relation: String, direction: String) -> i64;

/// Vérifier si une relation existe.
/// Même support de direction que get_related.
fn has_relation(result: Map, relation: String) -> bool;
fn has_relation(result: Map, relation: String, direction: String) -> bool;

/// Chunks d'une entité dans une KB.
/// Abstrait les tables internes (_SOURCED, _Chunk).
fn get_chunks(kb: String, result: Map) -> Array;

/// Index entry → entité source (pour multi-entity KBs).
fn resolve_source(kb: String, result: Map) -> Map;

/// Metadata d'une KB (title_entity, entities, relations).
fn kb_meta(kb: String) -> Map;

/// Escape hatch : requête Cypher brute en lecture.
fn query_cypher(cypher: String, params: Map) -> Array;


// === Tier 3 : Search ===

/// Recherche BM25 dans une KB. Retourne les résultats simplifiés.
fn search_bm25(kb: String, query: String, limit: i64) -> Array;

/// Recherche vectorielle dans une KB.
fn search_vector(kb: String, query: String, limit: i64) -> Array;


// === Tier 4 : Externe (opt-in) ===

/// Appel HTTP sortant. Désactivé par défaut.
/// Doit être explicitement activé dans la config.
fn call_http(url: String, method: String, body: String) -> Map;

/// Log de debug (écrit dans le log rag3weaver, pas stdout).
fn log(message: String);
```

### 5.3 Implémentation Rust des builtins

```rust
use rhai::{Engine, Map, Array, Dynamic};

fn register_builtins(engine: &mut Engine, catalog: Arc<Catalog>, conn: Arc<Connection>) {
    // Tier 1 : emit(op) et emit(op, options)
    let ops_queue = Arc::new(Mutex::new(Vec::<EmittedOp>::new()));

    // emit(op) — priorité par défaut, pas de callback
    let q = ops_queue.clone();
    engine.register_fn("emit", move |op: Map| {
        q.lock().unwrap().push(EmittedOp {
            op: map_to_search_op(&op),
            priority: None,        // → priorité par défaut du round
            then: None,
        });
    });

    // emit(op, options) — priorité custom + callback optionnel
    let q = ops_queue.clone();
    engine.register_fn("emit", move |op: Map, options: Map| {
        let priority = options.get("priority").and_then(|v| v.as_float().ok());
        let then = options.get("then").and_then(|v| v.clone().into_string().ok());
        q.lock().unwrap().push(EmittedOp {
            op: map_to_search_op(&op),
            priority,
            then,   // nom de la fonction Rhai à appeler avec les résultats
        });
    });

    // Tier 2 : Graph haut-niveau (KB-aware)
    // Les builtins acceptent un result handle (Map avec uuid + entity),
    // pas un UUID brut. Ça abstrait les tables internes de la KB.

    let c = conn.clone();
    engine.register_fn("get_related", move |result: Map, relation: String| -> Array {
        get_related_impl(&c, &result, &relation, "outgoing")
    });

    let c = conn.clone();
    engine.register_fn("get_related", move |result: Map, relation: String, direction: String| -> Array {
        get_related_impl(&c, &result, &relation, &direction)
    });

    // ...

    fn get_related_impl(conn: &Connection, result: &Map, relation: &str, direction: &str) -> Array {
        let uuid = result.get("uuid").unwrap().clone().into_string().unwrap();
        tokio::task::block_in_place(|| {
            Handle::current().block_on(async {
                let cypher = match direction {
                    "outgoing" => format!("MATCH (a)-[:{relation}]->(b) WHERE a._uuid = $uuid RETURN b"),
                    "incoming" => format!("MATCH (a)<-[:{relation}]-(b) WHERE a._uuid = $uuid RETURN b"),
                    "both"     => format!("MATCH (a)-[:{relation}]-(b) WHERE a._uuid = $uuid RETURN DISTINCT b"),
                    _ => panic!("direction must be 'outgoing', 'incoming', or 'both'"),
                };
                let rows = conn.execute_with_params(&cypher, &[("uuid", &uuid)]).await.unwrap();
                rows_to_rhai_array(&rows.rows)
            })
        })
    }

    let c = conn.clone();
    engine.register_fn("count_related", move |result: Map, relation: String| -> i64 {
        let uuid = result.get("uuid").unwrap().clone().into_string().unwrap();
        tokio::task::block_in_place(|| {
            Handle::current().block_on(async {
                let cypher = format!(
                    "MATCH (a)-[:{relation}]->(b) WHERE a._uuid = $uuid RETURN count(b) AS cnt"
                );
                let rows = c.execute_with_params(&cypher, &[("uuid", &uuid)]).await.unwrap();
                rows.rows[0].first().and_then(|v| v.as_i64()).unwrap_or(0)
            })
        })
    });

    let c = conn.clone();
    engine.register_fn("has_relation", move |result: Map, relation: String| -> bool {
        let uuid = result.get("uuid").unwrap().clone().into_string().unwrap();
        tokio::task::block_in_place(|| {
            Handle::current().block_on(async {
                let cypher = format!(
                    "MATCH (a)-[:{relation}]->() WHERE a._uuid = $uuid RETURN count(*) > 0 AS exists"
                );
                let rows = c.execute_with_params(&cypher, &[("uuid", &uuid)]).await.unwrap();
                rows.rows[0].first().and_then(|v| v.as_bool()).unwrap_or(false)
            })
        })
    });

    let cat = catalog.clone();
    engine.register_fn("get_chunks", move |kb: String, result: Map| -> Array {
        let uuid = result.get("uuid").unwrap().clone().into_string().unwrap();
        let entity = result.get("entity").unwrap().clone().into_string().unwrap();
        tokio::task::block_in_place(|| {
            Handle::current().block_on(async {
                // Résout automatiquement : {entity}_SOURCED_{kb} → {kb}_Chunk
                let sourced_rel = format!("{entity}_SOURCED_{kb}");
                let chunk_table = format!("{kb}_Chunk");
                let cypher = format!(
                    "MATCH (e)-[:{sourced_rel}]->(c:{chunk_table}) WHERE e._uuid = $uuid RETURN c"
                );
                let rows = cat.conn.execute_with_params(&cypher, &[("uuid", &uuid)]).await.unwrap();
                rows_to_rhai_array(&rows.rows)
            })
        })
    });

    let cat = catalog.clone();
    engine.register_fn("kb_meta", move |kb: String| -> Map {
        let meta = cat.kb_metadata.get(&kb).unwrap();
        let mut map = Map::new();
        map.insert("title_entity".into(), Dynamic::from(meta.title.entity.clone()));
        map.insert("entities".into(), Dynamic::from(
            meta.entities.iter().map(|e| Dynamic::from(e.name.clone())).collect::<Array>()
        ));
        // ... relations, etc.
        map
    });

    // Escape hatch : Cypher brut
    let c = conn.clone();
    engine.register_fn("query_cypher", move |cypher: String, params: Map| -> Array {
        tokio::task::block_in_place(|| {
            Handle::current().block_on(async {
                let rhai_params = map_to_cypher_params(&params);
                let result = c.execute_with_params(&cypher, &rhai_params).await.unwrap();
                rows_to_rhai_array(&result.rows)
            })
        })
    });

    // Tier 1.5 : Parallélisme
    let cat2 = catalog.clone();
    let c2 = conn.clone();
    engine.register_fn("run_parallel", move |ops: Array| -> Array {
        tokio::task::block_in_place(|| {
            Handle::current().block_on(async {
                let futures: Vec<_> = ops.into_iter().map(|op| {
                    let map = op.cast::<Map>();
                    execute_builtin_async(&cat2, &c2, map)
                }).collect();
                let results = futures::future::join_all(futures).await;
                results.into_iter().map(|r| Dynamic::from(r)).collect()
            })
        })
    });

    // Tier 3
    let cat = catalog.clone();
    engine.register_fn("search_bm25", move |kb: String, query: String, limit: i64| -> Array {
        tokio::task::block_in_place(|| {
            Handle::current().block_on(async {
                let opts = SearchOptions { limit: limit as usize, ..Default::default() };
                let response = cat.search(&kb, &query, opts).await.unwrap();
                results_to_rhai_array(&response.results)
            })
        })
    });
}

/// Dispatch un appel builtin depuis run_parallel().
/// Chaque entrée du tableau est un Map avec une clé = nom du builtin.
async fn execute_builtin_async(
    catalog: &Catalog,
    conn: &Connection,
    op: Map,
) -> Dynamic {
    if let Some(args) = op.get("query_cypher") {
        let args = args.clone().cast::<Map>();
        let cypher = args.get("cypher").unwrap().clone().into_string().unwrap();
        let params = args.get("params").unwrap().clone().cast::<Map>();
        let result = conn.execute_with_params(&cypher, &map_to_cypher_params(&params)).await.unwrap();
        Dynamic::from(rows_to_rhai_array(&result.rows))
    } else if let Some(args) = op.get("search_bm25") {
        let args = args.clone().cast::<Map>();
        let kb = args.get("kb").unwrap().clone().into_string().unwrap();
        let query = args.get("query").unwrap().clone().into_string().unwrap();
        let limit = args.get("limit").unwrap().as_int().unwrap();
        let opts = SearchOptions { limit: limit as usize, ..Default::default() };
        let response = catalog.search(&kb, &query, opts).await.unwrap();
        Dynamic::from(results_to_rhai_array(&response.results))
    } else if let Some(args) = op.get("search_vector") {
        // ... même pattern ...
        Dynamic::UNIT
    } else if let Some(args) = op.get("get_entity") {
        let uuid = args.clone().into_string().unwrap();
        let cypher = "MATCH (n) WHERE n._uuid = $uuid RETURN n LIMIT 1";
        let result = conn.execute_with_params(cypher, &[("uuid", &uuid)]).await.unwrap();
        Dynamic::from(row_to_rhai_map(&result.rows[0]))
    } else {
        Dynamic::UNIT
    }
}
```

---

## 6. Exemples de processors custom

### 6.1 Container expansion (remplace l'ExpansionConfig déclaratif)

```javascript
// container_expansion.rhai
// Équivalent de l'ExpansionConfig du Doc 04, mais en script

fn on_result(result, query) {
    let containers = ["class", "interface", "module", "enum", "namespace"];

    if result.entity == "Scope" && containers.contains(result.data.scope_type) {
        // Chercher les enfants qui matchent la query
        emit(#{
            search_related: #{
                origin: result,
                relation: "PARENT_OF",
                query: query,
                limit: 5
            }
        });

        // Fetcher les autres enfants (signatures uniquement)
        emit(#{
            fetch_related: #{
                origin: result,
                relation: "PARENT_OF",
                fields: ["signature", "scope_type"],
                exclude_matched: true,
                limit: 20
            }
        });
    }
}
```

### 6.2 Expansion conditionnelle basée sur le graph

```javascript
// smart_expansion.rhai
// Expansion seulement si l'entité a assez d'enfants pour que ça vaille le coup

fn on_result(result, query) {
    if result.entity != "Scope" || result.data.scope_type != "class" {
        return;
    }

    // Builtin haut-niveau — pas de Cypher !
    let child_count = count_related(result, "PARENT_OF");

    if child_count > 3 {
        // Assez d'enfants → expansion complète
        emit(#{ search_related: #{
            origin: result,
            relation: "PARENT_OF",
            query: query,
            limit: 5
        }});
        emit(#{ fetch_related: #{
            origin: result,
            relation: "PARENT_OF",
            fields: ["signature", "scope_type"],
            exclude_matched: true
        }});
    } else if child_count > 0 {
        // Peu d'enfants → juste fetch tout
        emit(#{ fetch_related: #{
            origin: result,
            relation: "PARENT_OF",
            fields: ["signature", "scope_type", "docstring"]
        }});
    }
    // 0 enfants → rien
}
```

### 6.3 Cross-KB : enrichir avec des données d'une autre KB

```javascript
// cross_kb_enrichment.rhai
// Chercher un fichier trouvé dans TreeKB aussi dans ScopeKB pour avoir les scopes

fn on_result(result, query) {
    if result.entity != "File" {
        return;
    }

    // Builtin haut-niveau — vérifie si la relation existe
    if has_relation(result, "IN_FILE") {
        emit(#{ search_related: #{
            origin: result,
            relation: "IN_FILE",
            query: query,
            limit: 5,
            kb: "ScopeKB"
        }});
    }
}
```

### 6.4 LLM re-ranking (Tier 4 : appel externe)

```javascript
// llm_rerank.rhai
// Re-ranker les résultats via un appel LLM externe

fn on_compose(results, query) {
    // Construire le prompt
    let prompt = `Given the query "${query}", rank these results by relevance:\n`;
    for (i, r) in results {
        prompt += `${i+1}. [${r.entity}] ${r.data.name}: ${r.data.signature}\n`;
    }

    // Appel externe (Tier 4, opt-in)
    let response = call_http(
        "https://api.llm.local/rerank",
        "POST",
        #{ prompt: prompt, model: "fast" }
    );

    // Réordonner les résultats selon le ranking LLM
    let ranking = response.order;  // [3, 1, 5, 2, 4, ...]
    reorder(results, ranking);
}
```

---

## 7. Philosophie : `emit()` vs appels directs vs `run_parallel()`

### 7.1 Trois modes d'interaction avec la queue

Un script Rhai a trois façons d'agir :

| Mode | Méthode | Le script attend le résultat ? | Parallélisme |
|---|---|---|---|
| **Fire-and-forget** | `emit(op)` | Non — la queue l'exécutera plus tard | Géré par la queue (ops de même priorité en parallèle) |
| **Fire-and-continue** | `emit(op, {then: "fn"})` | Non — mais le callback sera appelé avec les résultats | Géré par la queue + continuation |
| **Synchrone** | `search_bm25()`, `query_cypher()`, etc. | Oui — bloque jusqu'au résultat | Séquentiel (un appel à la fois) |
| **Synchrone parallèle** | `run_parallel([...])` | Oui — bloque jusqu'à ce que tout soit fini | Parallèle (un seul `block_in_place`, N futures) |

### 7.2 Recommandation : `emit()` par défaut

Le modèle recommandé est `emit()` — le script décrit ce qu'il veut, la queue orchestre l'exécution :

```javascript
// RECOMMANDÉ : emit() — fire and forget
fn on_result(result, query) {
    emit(#{ search_related: #{ origin: result, relation: "PARENT_OF", query: query, limit: 5 } });
    emit(#{ fetch_related: #{ origin: result, relation: "PARENT_OF", fields: ["signature"] } });
    // La queue exécute ces 2 ops en parallèle (même priorité), le script est déjà terminé.
}
```

Avantages :
- Le script est instantané (~10µs) — pas de `block_in_place`
- La queue parallélise naturellement les ops de même priorité
- Les résultats sont composés par le `ComposeProcessor` en fin de drain
- Le script reste simple et déclaratif

### 7.3 Priorités custom sur `emit()`

Par défaut, les ops émises prennent la priorité suivante dans la queue. On peut overrider :

```javascript
fn on_result(result, query) {
    // Priorité haute → s'exécute en premier
    emit(#{ search_related: #{ origin: result, relation: "PARENT_OF", query: query } },
         #{ priority: 10.0 });

    // Priorité basse → s'exécute après
    emit(#{ fetch_related: #{ origin: result, relation: "PARENT_OF", fields: ["signature"] } },
         #{ priority: 5.0 });
}
```

Signature de `emit()` :

```rust
/// emit(op)              → priorité par défaut
/// emit(op, options)     → options: { priority, then }
fn emit(op: Map);
fn emit(op: Map, options: Map);
```

Sans `priority`, la queue utilise la priorité par défaut du round courant (décroissante, comme pour l'ingestion queue).

### 7.4 Callback `then` : continuation après exécution

Le pattern le plus puissant de `emit()`. Le callback est appelé quand l'op a fini, avec ses résultats :

```javascript
fn on_result(result, query) {
    // Étape 1 : chercher les enfants qui matchent
    emit(#{ search_related: #{
            origin: result,
            relation: "PARENT_OF",
            query: query,
            limit: 5
         }},
         #{ priority: 10.0, then: "on_children_found" });
}

// Étape 2 : appelé quand SearchRelated a fini
fn on_children_found(context, children) {
    // context = { origin, relation, query, ... } (l'op qui a été exécutée)
    // children = résultats du SearchRelated

    // Maintenant on peut exclure les matchés
    emit(#{ fetch_related: #{
            origin: context.origin,
            relation: context.relation,
            exclude: children,
            fields: ["signature", "scope_type"]
         }});
}
```

C'est exactement ce que fait `ExpansionProcessor` (Doc 04) en dur — mais le script le fait de façon custom. Le `then` transforme une séquence fire-and-forget en **pipeline réactif scriptable**.

### 7.5 Cas d'usage concret du `then` : le pattern exclude

Le cas le plus fréquent — impossible à faire avec un simple `emit()` car on a besoin des UUIDs trouvés pour exclure :

```javascript
// SANS then : impossible d'exclure les matchés
fn on_result(result, query) {
    emit(#{ search_related: #{ origin: result, relation: "PARENT_OF" } });
    emit(#{ fetch_related: #{
        origin: result,
        relation: "PARENT_OF",
        exclude: ???  // On ne connaît pas encore les résultats !
    }});
}

// AVEC then : le callback reçoit les résultats de l'étape précédente
fn on_result(result, query) {
    emit(#{ search_related: #{ origin: result, relation: "PARENT_OF", query: query } },
         #{ then: "exclude_and_fetch" });
}

fn exclude_and_fetch(context, matched_children) {
    emit(#{ fetch_related: #{
        origin: context.origin,
        relation: context.relation,
        exclude: matched_children,
        fields: ["signature", "scope_type"]
    }});
}
```

### 7.6 Garde-fou : `max_callback_depth`

Un `then` peut émettre un op avec un autre `then`, qui peut émettre... Pour éviter les chaînes infinies :

```rust
struct SearchQueue {
    // ...
    max_callback_depth: usize,  // défaut: 3
}
```

| Depth | Qui appelle |
|---|---|
| 0 | `on_result()` — le hook initial |
| 1 | `on_children_found()` — callback du premier emit |
| 2 | `on_grandchildren()` — callback du callback |
| 3 | **STOP** — l'op est exécutée mais le `then` est ignoré |

En pratique, depth 2 couvre tous les cas réalistes (search → exclude → fetch). Depth 3 est une marge de sécurité.

### 7.7 Quand utiliser les appels directs ou `run_parallel()`

Les appels synchrones sont nécessaires quand le script a besoin du **résultat immédiatement pour une condition** (pas pour chaîner des ops) :

```javascript
// NÉCESSAIRE : le script a besoin de compter avant de décider
fn on_result(result, query) {
    // Builtin haut-niveau — pas de Cypher
    let child_count = count_related(result, "PARENT_OF");

    if child_count > 3 {
        emit(#{ search_related: #{ origin: result, relation: "PARENT_OF", query: query } });
    }
}
```

`run_parallel()` est utile quand le script a besoin de **plusieurs résultats indépendants** pour décider :

```javascript
// UTILE : besoin de 2 infos indépendantes pour décider
fn on_result(result, query) {
    let infos = run_parallel([
        #{ count_related: #{ result: result, relation: "PARENT_OF" } },
        #{ count_related: #{ result: result, relation: "IMPORTS" } },
    ]);

    let child_count = infos[0];
    let import_count = infos[1];

    if child_count > 3 {
        emit(#{ search_related: #{ origin: result, relation: "PARENT_OF", query: query, limit: 5 } });
    }
    if import_count > 0 {
        emit(#{ fetch_related: #{ origin: result, relation: "IMPORTS", fields: ["name", "path"] } });
    }
}
```

### 7.8 Résumé : quand utiliser quoi

```
Pas besoin du résultat              →  emit(op)
Besoin du résultat pour chaîner     →  emit(op, {then: "fn"})
Besoin du résultat pour une condition  →  appel direct (count_related, has_relation, ...)
Besoin de N résultats pour décider  →  run_parallel([...])
```

Les 4 modes forment un spectre de contrôle croissant :

```
emit()          le plus simple, la queue gère tout
     ↓
emit(+then)     fire-and-forget avec continuation
     ↓
appel direct    synchrone, une requête
     ↓
run_parallel()  synchrone, N requêtes en parallèle
```

En pratique, `emit()` + `emit(+then)` couvrent ~95% des cas. Les appels synchrones sont pour les conditions basées sur le graph (compter des enfants, vérifier une relation).

---

## 8. Intégration dans SearchQueue

### 8.1 EmittedOp : op + metadata

```rust
/// Une op émise par un script, avec ses options.
struct EmittedOp {
    /// L'opération à exécuter.
    op: SearchOp,
    /// Priorité custom (None = priorité par défaut du round).
    priority: Option<f32>,
    /// Nom de la fonction Rhai à appeler avec les résultats (None = fire-and-forget).
    then: Option<String>,
}
```

Quand la queue exécute une `EmittedOp` avec un `then` :
1. Exécute l'op normalement via le processor approprié
2. Récupère les résultats
3. Appelle `engine.call_fn(&ast, &then, (context, results))`
4. Le callback peut émettre de nouvelles ops → re-enqueue

```rust
// Dans SearchQueue::drain()
for emitted in script_emitted_ops {
    let priority = emitted.priority.unwrap_or(current_round_priority);

    if let Some(then_fn) = &emitted.then {
        // Enqueue avec callback attaché
        self.ops.push(QueuedOp {
            priority,
            op: emitted.op,
            callback: Some(Callback {
                script_ast: ast.clone(),
                fn_name: then_fn.clone(),
                depth: current_depth + 1,
            }),
        });
    } else {
        // Fire-and-forget classique
        self.ops.push(QueuedOp { priority, op: emitted.op, callback: None });
    }
}

// Quand on exécute une op avec callback :
let results = processor.process(op).await;
if let Some(cb) = callback {
    if cb.depth < self.max_callback_depth {
        // Appeler le callback Rhai avec les résultats
        engine.call_fn(&cb.script_ast, &cb.fn_name, (op_as_context, results))?;
        // Les emit() du callback sont collectés et re-enqueue
    }
}
```

### 8.2 ScriptProcessor

Un nouveau processor built-in qui exécute du Rhai :

```rust
/// Processor qui exécute un script Rhai pour chaque résultat.
struct ScriptProcessor {
    engine: Engine,
    ast: AST,          // script compilé (cache)
    hook: ScriptHook,  // quel hook le script implémente
}

enum ScriptHook {
    /// Appelé pour chaque résultat de la recherche primaire.
    /// fn on_result(result, query) → émet des ops via emit()
    OnResult,

    /// Appelé après composition de tous les résultats.
    /// fn on_compose(results, query) → peut réordonner/filtrer
    OnCompose,
}
```

### 8.3 SearchStrategy avec scripts

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStrategy {
    /// Options de recherche primaire.
    pub search: SearchOptions,

    /// Expansions déclaratives (Layer 1 — Doc 04).
    pub expansions: Vec<ExpansionConfig>,

    /// Scripts Rhai (Layer 2 — ce doc).
    /// Exécutés après les expansions déclaratives.
    pub scripts: Vec<ScriptConfig>,

    /// Nombre maximum de rounds.
    pub max_rounds: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    /// Le code Rhai (inline ou référence à un fichier).
    pub source: ScriptSource,

    /// Quel hook le script implémente.
    pub hook: ScriptHook,

    /// Quels tiers de builtins activer.
    /// Par défaut : [Tier1, Tier2, Tier3].
    /// Tier4 (externe) doit être explicitement opt-in.
    pub allowed_tiers: Vec<BuiltinTier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScriptSource {
    /// Code Rhai inline (dans la SearchStrategy JSON).
    Inline(String),
    /// Référence à un fichier .rhai sur le FS.
    File(PathBuf),
}
```

### 8.4 Ordre d'exécution dans la queue

```
1. PrimarySearchProcessor    → exécute Search, produit résultats
2. ExpansionProcessor(s)     → expansions déclaratives (Doc 04)
3. ScriptProcessor(OnResult) → scripts Rhai par résultat
4. [downstream ops exécutées par leurs processors respectifs]
5. ComposeProcessor          → assemble les résultats enrichis
6. ScriptProcessor(OnCompose)→ post-processing Rhai (optionnel)
```

Les scripts Rhai s'exécutent **après** les expansions déclaratives. Si un script émet des ops, elles sont enqueue et exécutées par les processors built-in (pas par le script).

Le script ne touche jamais à l'exécution — il émet des **instructions** (SearchOps) que les processors built-in exécutent.

### 8.5 Diagramme

```
                     SearchStrategy
                    ┌──────────────────────────┐
                    │  search: SearchOptions    │
                    │  expansions: [...]        │  ← Layer 1 (déclaratif)
                    │  scripts: [...]           │  ← Layer 2 (Rhai)
                    └──────────┬───────────────┘
                               │
                    SearchQueue::drain()
                               │
                    ┌──────────▼───────────────┐
                    │  PrimarySearchProcessor   │
                    │  (exécute Search op)      │
                    └──────────┬───────────────┘
                               │ résultats
                    ┌──────────▼───────────────┐
                    │  ExpansionProcessor       │
                    │  (déclaratif, Doc 04)     │──→ émet SearchRelated, FetchRelated
                    └──────────┬───────────────┘
                               │
                    ┌──────────▼───────────────┐
                    │  ScriptProcessor          │
                    │  (Rhai, on_result)        │──→ émet n'importe quel SearchOp
                    └──────────┬───────────────┘
                               │
                    ┌──────────▼───────────────┐
                    │  [downstream processors]  │
                    │  RelatedSearch, Fetch...  │
                    └──────────┬───────────────┘
                               │
                    ┌──────────▼───────────────┐
                    │  ComposeProcessor         │
                    └──────────┬───────────────┘
                               │
                    ┌──────────▼───────────────┐
                    │  ScriptProcessor          │
                    │  (Rhai, on_compose)       │──→ réordonne, filtre, enrichit
                    └──────────┴───────────────┘
```

---

## 9. Le point technique : async dans Rhai

### 9.1 Le problème

Rhai est **synchrone**. Les builtins DB (`query_cypher`, `search_bm25`) sont **async** en Rust.

### 9.2 La solution : block_in_place

```rust
engine.register_fn("query_cypher", move |cypher: String, params: Map| -> Array {
    tokio::task::block_in_place(|| {
        Handle::current().block_on(async {
            let result = conn.execute_with_params(&cypher, &params).await?;
            Ok(rows_to_rhai_array(&result.rows))
        })
    })
});
```

`block_in_place` dit à Tokio "je vais bloquer ce thread, déplace les autres tâches". C'est safe tant que :
- Le script est rapide (<100ms typiquement)
- On ne le fait pas en parallèle sur des dizaines de threads

C'est exactement le pattern de Redis + Lua : le script bloque le thread pendant son exécution, mais un script typique prend <10ms (le gros du temps est dans les builtins DB, pas dans le script lui-même).

### 9.3 Alternative évaluée et rejetée

**Script qui retourne des intentions** : le script ne fait que déclarer ce qu'il veut (`emit()` accumule dans un vec), l'exécution async se fait après. Problème : le script ne peut pas faire `query_cypher()` pour prendre une décision conditionnelle (cf. exemple 6.2). Trop limitant.

---

## 10. Sécurité et limites

### 10.1 Sandbox par défaut

| Accès | Par défaut | Opt-in |
|---|---|---|
| Variables du résultat | Oui (Tier 1) | — |
| emit() | Oui (Tier 1) | — |
| Graph haut-niveau (get_related, count_related, get_chunks, ...) | Oui (Tier 2) | — |
| Cypher brut (query_cypher) | Oui (Tier 2) | — |
| Search (BM25, vector) | Oui (Tier 3) | — |
| Réseau (call_http) | **Non** | `allowed_tiers: [Tier4]` |
| Filesystem | **Non** | Jamais |
| Écriture graph | **Non** | Jamais |

Un script ne peut **jamais** écrire dans le graph ni accéder au filesystem. Les tiers 1-3 sont en lecture seule. Le tier 4 (réseau) nécessite un opt-in explicite.

### 10.2 Limites d'exécution

```rust
// Protection contre les scripts malveillants ou buggés
engine.set_max_operations(100_000);    // ~10ms de CPU
engine.set_max_call_levels(32);        // profondeur de récursion
engine.set_max_string_size(1_000_000); // 1MB max par string
engine.set_max_array_size(10_000);     // 10K éléments max par array
engine.set_max_map_size(1_000);        // 1K entrées max par map
```

### 10.3 Erreurs

Un script qui échoue (erreur Rhai, timeout, builtin qui retourne une erreur) :
- Log l'erreur avec le contexte (script source, ligne, résultat qui a déclenché)
- **Ne fait pas échouer la recherche** — le résultat est retourné sans enrichissement
- Compteur d'erreurs par script pour monitoring

---

## 11. DX : comment les utilisateurs écrivent des scripts

### 11.1 Inline dans la SearchStrategy JSON

```json
{
  "kb": "ScopeKB",
  "query": "auth",
  "strategy": {
    "scripts": [
      {
        "source": { "inline": "fn on_result(r, q) { if r.data.scope_type == \"class\" { emit(#{ search_related: #{ origin: r, relation: \"PARENT_OF\", query: q, limit: 5 }}); } }" },
        "hook": "on_result"
      }
    ]
  }
}
```

Pas idéal pour des scripts longs, mais pratique pour des one-liners.

### 11.2 Fichier .rhai référencé

```json
{
  "strategy": {
    "scripts": [
      { "source": { "file": "./processors/container_expansion.rhai" }, "hook": "on_result" }
    ]
  }
}
```

Les fichiers `.rhai` peuvent être versionnés dans le projet de l'utilisateur.

### 11.3 Presets avec scripts intégrés

```rust
fn code_search_strategy() -> SearchStrategy {
    SearchStrategy {
        search: SearchOptions { result_mode: ResultMode::SourceResolved, ..default() },
        expansions: vec![/* ... déclaratif ... */],
        scripts: vec![
            ScriptConfig {
                source: ScriptSource::Inline(include_str!("scripts/code_expansion.rhai").into()),
                hook: ScriptHook::OnResult,
                allowed_tiers: vec![Tier1, Tier2, Tier3],
            },
        ],
        max_rounds: 3,
    }
}
```

Les scripts des presets built-in sont embarqués dans le binaire via `include_str!`.

---

## 12. Relation avec les layers

```
┌─────────────────────────────────────────────────────┐
│  Layer 3 : Callbacks JS (Node.js bridge)            │
│  Pas implémenté. Rhai couvre le besoin.              │
│  Si besoin futur : le callback JS retourne des ops  │
│  (même vocabulaire SearchOp).                       │
├─────────────────────────────────────────────────────┤
│  Layer 2 : Rhai scripts (ce doc)                    │
│  Cross-plateforme, expressif, sandboxé.             │
│  Accès aux builtins : Cypher, search, emit.         │
│  Pour la logique conditionnelle complexe.           │
├─────────────────────────────────────────────────────┤
│  Layer 1 : Déclaratif (ExpansionConfig, Doc 04)     │
│  Zéro code. JSON sérialisable.                      │
│  Couvre 80-90% des cas standards.                    │
│  Prioritaire à l'implémentation.                    │
└─────────────────────────────────────────────────────┘
```

Le déclaratif est le **défaut recommandé**. On n'écrit du Rhai que quand on a besoin de :
- Conditions basées sur le graph (query_cypher dans le trigger)
- Logique multi-branche complexe (if/else imbriqués)
- Cross-KB enrichment
- Post-processing custom (on_compose)

Le Layer 3 (callbacks JS) n'est pas nécessaire si Rhai est implémenté. Si demandé plus tard, le callback JS retournerait des SearchOps — même vocabulaire, même IR que le Rhai.

---

## 13. Relation avec les docs précédents

| Doc | Relation |
|---|---|
| **Doc 01 (ResultMode)** | Rhai peut émettre des `FetchChunks` ops → équivalent de ResultMode::Detailed mais conditionnel |
| **Doc 02 (Abstractions)** | Les domain adapters pourraient inclure des scripts Rhai prédéfinis pour leur domaine |
| **Doc 03 (CATALOG_SEARCH)** | Le paramètre `strategy` de CATALOG_SEARCH accepte des scripts inline → Rhai accessible depuis Cypher |
| **Doc 04 (SearchQueue)** | Rhai est le Layer 2 d'extensibilité. Le ScriptProcessor est un processor comme les autres dans la SearchQueue. Les ops émises par Rhai sont les mêmes SearchOps que celles émises par ExpansionProcessor. |

---

## 14. Implémentation

### 14.1 Dépendance Cargo

```toml
[dependencies]
rhai = { version = "1.x", features = ["sync", "serde"] }
# "sync" : types Send+Sync pour usage multi-thread
# "serde" : conversion automatique Rhai Map ↔ Rust structs
```

### 14.2 Effort estimé

| Composant | Effort |
|---|---|
| Intégration Rhai engine + builtins Tier 1-2 | ~2 jours |
| ScriptProcessor dans SearchQueue | ~1 jour |
| Builtins Tier 3 (search_bm25, search_vector) | ~1 jour |
| Tests + exemples de scripts | ~1 jour |
| **Total** | **~5 jours** |

### 14.3 Phase dans le plan global

```
Phase 1 : ResultMode (task #105)          ← en cours
Phase 2 : SearchQueue minimal (Doc 04)    ← nécessaire avant Rhai
Phase 3 : Rhai extensibility (ce doc)     ← après SearchQueue
Phase 4 : Presets domaine avec scripts
Phase 5 : CATALOG_SEARCH + strategy
```

Rhai arrive en **Phase 3** — après que la SearchQueue existe et fonctionne avec le déclaratif. On ajoute le scripting quand le déclaratif montre ses limites sur des cas réels.
