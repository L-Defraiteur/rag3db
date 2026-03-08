# Doc 19 — Design : Phase 5 — Rhai ScriptNode

Date : 8 mars 2026

## Contexte

Phases 1-4 du framework dataflow complètes (489 tests). Le graph exécute des pipelines typés (search, ingestion, migrations) via des nœuds Rust statiques.

Phase 5 ajoute **ScriptNode** — un nœud qui exécute du Rhai pour les cas que les nœuds built-in ne couvrent pas.

### Changement par rapport au Doc 05 (Rhai + SearchQueue)

Le Doc 05 (3 mars) décrivait Rhai dans le contexte de la SearchQueue : `on_result()` hooks, `emit()` pour enqueue des ops, callbacks `then`, builtins par tiers. Ce design ne s'applique plus tel quel car :

- La SearchQueue a été remplacée par le Dataflow graph
- Le flux est déclaratif (nœuds + edges + topo sort), pas impératif (emit + callbacks)
- Les nœuds n'émettent pas d'ops — ils lisent des inputs, écrivent des outputs

Ce qui se transpose :
- Le modèle sandbox additif (on expose uniquement ce qu'on veut)
- Les builtins `query_cypher`, `log` (via ServiceRegistry au lieu de closures)
- Les limites d'exécution (max_operations, max_call_levels, etc.)
- Le pattern `block_in_place` pour les builtins async

Ce qui disparaît :
- `emit()` / `then` / `run_parallel()` — remplacé par la topologie du graph
- ScriptHook (OnResult / OnCompose) — le script est un nœud comme un autre
- Les tiers de builtins (1-4) — simplifié en "ce que le ServiceRegistry expose"

---

## 1. ScriptNode dans le Dataflow

### 1.1 Modèle

Un ScriptNode est un nœud normal (`impl Node`) dont la logique `execute()` est un script Rhai au lieu de code Rust compilé.

```
graph LR
    create_table["CypherNode(query='CREATE NODE TABLE ...')"]
    transform["ScriptNode(file='transforms/normalize.rhai')"]
    validate["ValidateNode(query='...', assert='not_empty')"]

    create_table -->|done:trigger| transform
    transform -->|done:trigger| validate
```

Le script accède à :
- Ses **inputs** (ports) — les valeurs arrivant des edges
- Les **services** (ServiceRegistry) — `conn` pour Cypher, futur: catalog, embedder
- Ses **outputs** (ports) — les valeurs qu'il émet

### 1.2 Ports

**Question design #1 : ports statiques vs déclarés**

Option A — **Ports fixes** (comme CypherNode) :
- Input : `trigger` (Empty, optionnel)
- Outputs : `result` (Map), `done` (Empty)
- Simple, cohérent avec les autres nœuds de migration
- Le script retourne un JSON quelconque sur `result`

Option B — **Ports déclarés par annotations** (Doc 10) :
```javascript
// @input results: Results
// @output filtered: Results
fn execute(results) { ... }
```
- Plus flexible, permet des scripts de transformation de données typées
- Parsing des annotations au compile-time du script
- Complexité : conversion PortValue ↔ Rhai Dynamic pour chaque type

Option C — **Ports déclarés dans la config Mermaid** :
```
script["ScriptNode(file='filter.rhai', in_results='Results', out_filtered='Results')"]
```
- Pas d'annotation dans le script, mais verbeux dans le Mermaid
- Les ports deviennent des paramètres de config

**Recommandation : Option A pour le MVP, avec extension vers B plus tard.**

Raison : le premier use case est les migrations, où trigger/done/result suffisent. La transformation de données typées (Results, Children) est un cas futur. On peut ajouter les annotations dans une phase ultérieure sans breaking change — il suffit de faire coexister les deux modes dans ScriptNodeFactory.

### 1.3 Source du script

**Question design #2 : inline, fichier, ou les deux ?**

Option A — **Fichier uniquement** (`file='path/to/script.rhai'`)
- Lisible, versionnable, testable séparément
- Mais nécessite un fichier .rhai sur le FS au runtime

Option B — **Inline uniquement** (`script='fn execute(ctx) { ... }'`)
- Self-contained, pas de dépendance FS
- Illisible pour des scripts > 1 ligne dans le Mermaid

Option C — **Les deux** (comme CypherNode a `query` inline)
- `script='...'` pour les one-liners
- `file='path.rhai'` pour les scripts complexes
- Un seul des deux requis

**Recommandation : Option C.**

Le ScriptNodeFactory check : si `file` est set, lire le fichier et compiler. Si `script` est set, compiler l'inline. Erreur si les deux ou aucun.

---

## 2. Builtins exposés

### 2.1 MVP (Phase 5)

Le strict minimum pour être utile dans les migrations et transformations :

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `query_cypher(query)` | `fn(String) -> Array` | Exécute un Cypher en lecture, retourne les rows comme Array de Maps |
| `query_cypher(query, params)` | `fn(String, Map) -> Array` | Idem avec paramètres |
| `log(message)` | `fn(String)` | Log dans NodeContext (visible dans ExecutionReport) |
| `log(key, value)` | `fn(String, Dynamic)` | Log une métrique nommée |

Pas besoin de plus pour le MVP. Les builtins `search_bm25`, `get_related`, etc. du Doc 05 seraient ajoutés dans une phase ultérieure quand le catalog est accessible comme service.

### 2.2 Accès aux inputs dans le script

Le script a une variable `ctx` (Map) injectée avec :
- `ctx.inputs` — Map des inputs (nom_port → valeur)
- `ctx.config` — Map de la config du nœud (paramètres Mermaid)

Le script écrit ses outputs via :
- `set_output(port_name, value)` — fonction builtin

```javascript
// Exemple : migration qui normalise des noms
let rows = query_cypher("MATCH (n:Entity) RETURN n._uuid, n.name");
for row in rows {
    let normalized = row.name.to_lower();
    if normalized != row.name {
        query_cypher(`MATCH (n {_uuid: '${row._uuid}'}) SET n.name = '${normalized}'`);
        log("normalized", row._uuid);
    }
}
set_output("done", ());
```

### 2.3 Conversion Rhai ↔ Rust

Les types qui passent la frontière :

| Rhai | Rust / PortValue |
|------|------------------|
| `()` (unit) | `PortValue::Empty` |
| `Map` | `PortValue::Map(serde_json::Value)` |
| `String` | `serde_json::Value::String` |
| `i64` | `serde_json::Value::Number` |
| `f64` | `serde_json::Value::Number` |
| `bool` | `serde_json::Value::Bool` |
| `Array` | `serde_json::Value::Array` |

Pour le MVP, seuls `Empty` et `Map` sont utiles comme PortValue. Les types spécifiques (Results, Children, Entities) nécessiteraient un registered type Rhai — hors scope MVP.

---

## 3. Sandbox et limites

Modèle identique au Doc 05 section 10 :

```rust
engine.set_max_operations(100_000);     // ~10ms de CPU
engine.set_max_call_levels(32);         // profondeur de récursion
engine.set_max_string_size(1_000_000);  // 1MB max par string
engine.set_max_array_size(10_000);      // 10K éléments max par array
engine.set_max_map_size(1_000);         // 1K entrées max par map
```

Pas d'accès FS, pas d'accès réseau, pas de sleep. Uniquement les builtins enregistrés.

---

## 4. Undo

**Question design #3 : les ScriptNode sont-ils undoable ?**

Option A — **Jamais undoable** (`can_undo() = false`)
- Simple, safe
- Si le script écrit dans le graph via `query_cypher`, pas de rollback automatique
- L'utilisateur doit écrire un CypherNode avec capture pour les mutations reversibles

Option B — **Optionnel via fonctions undo dans le script**
```javascript
fn execute(ctx) {
    let old = query_cypher("MATCH (n:Foo) RETURN n._uuid, n.bar");
    set_undo_context(old);  // capture pour rollback
    query_cypher("MATCH (n:Foo) SET n.bar = 'new'");
    set_output("done", ());
}

fn undo(ctx, undo_data) {
    for row in undo_data {
        query_cypher(`MATCH (n {_uuid: '${row._uuid}'}) SET n.bar = '${row.bar}'`);
    }
}
```
- Plus puissant, cohérent avec le trait Node
- Complexité : le runtime doit compiler 2 fonctions, gérer les erreurs de chaque

**Recommandation : Option B.**

Raison : c'est cohérent avec le pattern du trait Node (can_undo, undo_context, undo). Le ScriptNode report `can_undo() = true` si le script définit `fn undo(...)`. Le runtime capture `undo_context()` comme pour les autres nœuds. C'est plus de code mais l'alternative (forcer CypherNode pour tout ce qui est reversible) frustre les power users.

---

## 5. Feature flag

**Question design #4 : rhai optionnel ou toujours inclus ?**

`rhai` ajoute ~2MB au binaire. Pour WASM c'est significatif.

**Recommandation : feature flag `rhai-script`.**

```toml
[dependencies]
rhai = { version = "1", features = ["sync", "serde"], optional = true }

[features]
rhai-script = ["dep:rhai"]
```

Le ScriptNodeFactory n'est enregistré dans `register_builtins()` que si `cfg(feature = "rhai-script")`. Sans le feature, parser un Mermaid avec un ScriptNode donne une erreur claire ("ScriptNode requires the 'rhai-script' feature").

---

## 6. Async dans Rhai

Identique au Doc 05 section 9. Les builtins DB utilisent `block_in_place` :

```rust
engine.register_fn("query_cypher", move |query: String| -> rhai::Array {
    let conn = conn.clone();
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            let result = conn.execute(&query).await
                .map_err(|e| e.to_string())?;
            Ok(rows_to_rhai_array(&result))
        })
    }).unwrap_or_default()
});
```

C'est safe car :
- Un ScriptNode s'exécute seul (pas de parallélisme intra-nœud dans le runtime)
- Le script typique prend <100ms
- `block_in_place` prévient tokio du blocage

---

## 7. Implémentation

### Fichiers

```
A  src/dataflow/script_node.rs     (~400 lignes)
     ScriptNode, ScriptNodeFactory
     register_rhai_builtins(engine, services)
     rhai_to_json(Dynamic) → serde_json::Value
     json_to_rhai(Value) → Dynamic
     cypher_result_to_rhai_array()
M  src/dataflow/node_factories.rs   (register ScriptNodeFactory, conditionally)
M  src/dataflow/mod.rs              (pub mod + exports)
M  Cargo.toml                       (rhai optional dependency + feature)
```

### Étapes

1. **Cargo.toml** — ajouter `rhai` optional + feature `rhai-script`
2. **script_node.rs** — ScriptNode struct + Node impl + Factory
3. **Builtins** — `query_cypher`, `log`, `set_output`, `set_undo_context`
4. **Conversions** — `rhai_to_json` / `json_to_rhai` bidirectionnel
5. **Undo** — détection de `fn undo(...)` dans l'AST, appel conditionnel
6. **Wire** — mod.rs exports, register_builtins conditionally
7. **Tests** (~15) :
   - Script simple (trigger → done)
   - Script avec query_cypher mock
   - Script avec set_output (result Map)
   - Script avec undo (capture + restore)
   - Script sans undo (can_undo = false)
   - Conversion rhai ↔ json roundtrip (string, int, float, bool, array, map, null)
   - Script inline vs fichier
   - Sandbox : script trop long → erreur
   - Erreur de script → node failed avec message clair
   - Factory : config manquante → erreur
   - Factory : fichier manquant → erreur
   - Parse Mermaid avec ScriptNode → graph valide
   - Template avec ScriptNode + CypherNode → chaîne complète
8. **cargo check + cargo test** → ~500+ tests pass

### Ce qu'on NE fait PAS (hors scope)

- `emit()` / dynamic graph expansion depuis Rhai (pas de ScriptDynamicNode)
- Builtins search (search_bm25, search_vector) — nécessite catalog comme service
- Builtins graph haut-niveau (get_related, count_related) — idem
- call_http (réseau) — futur
- REPL interactif
- Ports typés via annotations (@input/@output) — futur

---

## 8. Résumé des questions design

| # | Question | Recommandation | Impact si on change plus tard |
|---|----------|----------------|-------------------------------|
| 1 | Ports statiques vs déclarés | Statiques MVP (trigger/result/done) | Faible — on ajoute les annotations sans casser |
| 2 | Source inline/fichier/les deux | Les deux | Nul |
| 3 | Undo dans le script | Oui, via fn undo() optionnel | Nul — déjà dans le trait Node |
| 4 | Feature flag rhai | Oui, `rhai-script` | Nul |

Les 4 questions ont des réponses assez évidentes. Pas de choix architectural lourd — ScriptNode est un CypherNode "programmable". Le gros du travail est la plomberie Rhai ↔ Rust (conversions, builtins, sandbox).
