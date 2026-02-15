# Rag3Weaver — Single WASM : Concessions, limites et pistes (15 février 2026)

Date : 15 février 2026

---

## 1. Fuite mémoire sur chaque appel FFI (`return_string_to_c`)

**Fichier** : `src/wasm_ffi.rs:764`

```rust
fn return_string_to_c(s: String) -> *const c_char {
    let c = CString::new(s).unwrap_or_else(|_| CString::new("").unwrap());
    let ptr = c.as_ptr();
    std::mem::forget(c); // ← fuite : jamais libéré
    ptr
}
```

Chaque appel à `create()`, `drain()`, `count()` alloue une `CString` côté Rust et la
`forget()`. Le C++ copie immédiatement dans un `std::string`, mais l'allocation Rust
n'est jamais libérée. Sur des milliers d'appels, ça accumule.

### Correction prévue

Thread-local qui garde la dernière `CString`. L'ancienne est droppée quand on en écrit
une nouvelle :

```rust
thread_local! {
    static LAST_RETURN: std::cell::RefCell<CString> =
        std::cell::RefCell::new(CString::new("").unwrap());
}

fn return_string_to_c(s: String) -> *const c_char {
    let c = CString::new(s).unwrap_or_else(|_| CString::new("").unwrap());
    LAST_RETURN.with(|cell| {
        let ptr = c.as_ptr();
        *cell.borrow_mut() = c;  // drop l'ancien, garde le nouveau vivant
        ptr
    })
}
```

Le pointeur reste valide jusqu'au prochain appel FFI. Le C++ copie avant, donc pas de
use-after-free. Zéro fuite, zéro changement de l'API externe.

---

## 2. UUID vide au retour de `create()` → handle opaque

**Fichier** : `src/wasm_ffi.rs:858`

```rust
let uuid = entity_ref.uuid().unwrap_or_default(); // → "" (Pending)
```

`create()` retourne toujours `{"uuid":""}` parce que `EntityRef.uuid()` est en état
`Pending` — l'UUID réel n'est résolu qu'après `drain()`.

### Analyse : l'appelant n'a pas besoin de l'UUID

En réalité, l'UUID retourné par `create()` n'est utile dans **aucun** flow courant :

- **Ingestion** : `create()` → `link()` → `drain()`. Le `Catalog` Rust utilise des
  `EntityRef` pour `link()`, pas des UUID. Un handle opaque suffit.
- **Recherche** : l'UUID vient des résultats de `search()`, pas du `create()`.
- **Update/Delete** : l'UUID vient d'un query Cypher ou d'une recherche, pas du `create()`.

Le flow naturel est :
```
create → handle → link(handle, handle) → drain → search → UUID (dans les résultats)
```

### Correction prévue : handle entier

`create()` retourne un **index entier** (handle opaque). L'`EntityRef` correspondant
est stocké côté Rust dans un `Vec<EntityRef>` sur le contexte du Catalog.

```rust
// Rust FFI
fn rag3weaver_create(ctx, entity_type, fields_json) -> i64 {
    let entity_ref = catalog.create(...).await;
    let handle = ctx.refs.len() as i64;
    ctx.refs.push(entity_ref);
    handle  // 0, 1, 2, ...
}
```

```cpp
// C++ embind
int create(std::string entityType, std::string fieldsJson) {
    return (int)rag3weaver_create(ctx_, entityType.c_str(), fieldsJson.c_str());
}
```

```js
// JS — usage
const docHandle = weaver.create("Document", JSON.stringify({ title: "Rust Guide" }));
const chunkHandle = weaver.create("Chunk", JSON.stringify({ text: "..." }));
weaver.link(docHandle, chunkHandle, "HAS_CHUNK");
weaver.drain();
// Plus tard : search() retourne les UUID finaux
```

**Avantages** :
- Impossible de confondre temp_uuid / uuid — il n'y a ni l'un ni l'autre
- Le handle n'est utilisable que pour `link()`, pas pour des requêtes DB
- Zéro JSON à parser pour un simple create
- API impossible à mal utiliser

---

## 3. Clé JSON `"pending"` au lieu de `"persisted"` dans drain — CORRIGÉ

**Fichier** : `src/wasm_ffi.rs:876`

Bug déjà corrigé dans cette session. La clé JSON est maintenant `"persisted"`.

---

## 4. `catalog_new` retourne NULL sans message d'erreur

**Fichier** : `src/wasm_ffi.rs:798-820`

Trois points de sortie retournent `std::ptr::null_mut()` sans message :
1. Config JSON invalide (parse error)
2. Connexion DB échouée (path invalide)
3. `initialize()` échoué (erreur schema)

Le C++ throw un `std::runtime_error("rag3weaver_catalog_new failed")` générique.

### Correction prévue

Thread-local `LAST_ERROR` (même pattern que `LAST_RETURN`) + nouvelle fonction FFI :

```rust
thread_local! {
    static LAST_ERROR: std::cell::RefCell<CString> =
        std::cell::RefCell::new(CString::new("").unwrap());
}

fn set_last_error(msg: String) {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = CString::new(msg).unwrap_or_default();
    });
}

#[no_mangle]
pub extern "C" fn rag3weaver_last_error() -> *const c_char {
    LAST_ERROR.with(|cell| cell.borrow().as_ptr())
}
```

Le C++ :
```cpp
Weaver(std::string config, std::string path) {
    ctx_ = rag3weaver_catalog_new(config.c_str(), path.c_str());
    if (!ctx_) {
        std::string err = rag3weaver_last_error();
        throw std::runtime_error(err.empty() ? "unknown error" : err);
    }
}
```

---

## 5. Embeddings en WASM : candle impossible, `@xenova/transformers` via callback

### Pourquoi pas candle ?

Candle (le backend natif de rag3weaver) ne compile pas pour `wasm32-unknown-emscripten` :
- Dépend de BLAS/LAPACK ou fallback CPU lourd
- Les poids du modèle (23-120 MB) nécessitent un chargement spécialisé
- Pas de SIMD emscripten supporté par candle
- Le build.rs de candle ne gère pas emscripten

### La bonne approche : `@xenova/transformers` (Transformers.js)

Déjà validé dans `ragforge-core-exp-kuzu/kuzu-wasm-exp/dist/test/rag-demo.html` :

```js
import { BrowserRAG } from '/browser-rag.js';
// ← utilise @xenova/transformers (ONNX Runtime Web)
// ← modèles : all-MiniLM-L6-v2, bge-small, gte-small, multilingual-e5
// ← backends : WebGL, WebGPU, WASM
```

Cette démo prouve que les embeddings browser fonctionnent (40k docs benchmarkés).

### Intégration avec Weaver : callback FFI bidirectionnel

Le trait `Embedder` est déjà batch-ready :

```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError>;
    fn dim(&self) -> usize;
}
```

Et `CallbackEmbedder` wraps une closure :

```rust
pub type EmbedFn = Box<
    dyn Fn(&[String]) -> Pin<Box<dyn Future<Output = Result<Vec<Vec<f32>>, EmbedError>> + Send>>
        + Send + Sync,
>;
```

Le flow WASM serait :
1. JS initialise Transformers.js (`pipeline("feature-extraction", "all-MiniLM-L6-v2")`)
2. JS passe une callback au Weaver via un nouvel entry point FFI
3. Pendant `drain()`, le Rust appelle la callback → FFI → JS → Transformers.js → vecteurs → retour

**Difficulté** : c'est un callback **bidirectionnel** (Rust appelle JS pendant une
exécution initiée par JS). Faisable avec `emscripten_call_function_*` ou un mécanisme
de polling, mais plus complexe que le FFI unidirectionnel actuel.

**Alternative plus simple** : l'appelant JS embedde lui-même en amont, puis passe les
vecteurs pré-calculés à `create()` comme champ spécial. Le Weaver stocke juste le
vecteur. Pas de callback FFI nécessaire — mais ça bypasse le pipeline queue/batch.

---

## 6. EmbedProcessor ne batch PAS les appels embed — BUG PERF

### Le problème

La queue batch correctement : `processable.chunks_mut(batch_size)` avec `batch_size=32`
pour les ops embed (`queue.rs:307`). Le processor reçoit bien un slice de 32 items.

Mais `EmbedProcessor` boucle un par un :

```rust
// catalog.rs:1052-1113
async fn process(&self, items: &mut [OperationItem]) -> Result<(), String> {
    for item in items.iter_mut() {
        // ...
        self.embedder.embed(&[text]).await  // ← UN SEUL texte !
    }
}
```

Avec 100 documents :
- **Actuel** : 100 appels `embed(&[text])` séparés (4 batches × 25 appels chacun)
- **Correct** : 4 appels `embed(&[25 texts])` (si batch_size=32, c'est même 4 appels de ≤32)

### Correction prévue

```rust
async fn process(&self, items: &mut [OperationItem]) -> Result<(), String> {
    // 1. Collect all texts + track indices
    let mut texts = Vec::new();
    let mut indices = Vec::new();  // which items need embedding
    for (i, item) in items.iter_mut().enumerate() {
        if let CatalogOp::Embed(ref mut embed) = item.op {
            // Wait for entity ref resolution
            let _uuid = embed.entity_ref.ready().await
                .map_err(|e| format!("embed ref resolution failed: {e}"))?;
            if !embed.texts.is_empty() {
                texts.push(embed.texts.join("\n"));
                indices.push(i);
            }
        }
    }

    if texts.is_empty() { return Ok(()); }

    // 2. Single batch embed call
    let vectors = self.embedder.embed(&texts).await
        .map_err(|e| format!("embedding failed: {e}"))?;

    // 3. Distribute results back
    for (vec_idx, &item_idx) in indices.iter().enumerate() {
        if let CatalogOp::Embed(ref embed) = items[item_idx].op {
            // ... store vectors[vec_idx] on entity node
        }
    }
    Ok(())
}
```

### Impact sur le callback WASM

C'est crucial pour les embeddings browser : chaque appel embed franchit la frontière
FFI (Rust → C++ → JS → ONNX Runtime). Un seul appel batch de 32 textes est
dramatiquement plus rapide que 32 allers-retours. Transformers.js optimise en interne
(tokenisation batch, inference batch sur le GPU/WebGL).

---

## 7. `unsafe impl Send + Sync` sur WasmDbConnection

**Fichier** : `src/wasm_ffi.rs:517-518`

`WasmDbConnection` contient des raw pointers (`CDatabase`, `CConnection`) qui ne sont
pas `Send`/`Sync`. On les marque manuellement car WASM emscripten est single-threaded.

**Sécurité** : le module entier est gated derrière `#[cfg(feature = "wasm-emscripten")]`.
Ce feature n'est activé que dans le build WASM. Le code n'est jamais compilé en natif.

**Renforcement possible** : ajouter un compile_error guard :

```rust
#[cfg(all(feature = "wasm-emscripten", not(target_arch = "wasm32")))]
compile_error!("wasm-emscripten feature requires wasm32 target");
```

Cela empêcherait toute activation accidentelle en natif.

---

## 8. API tout-JSON (strings) au lieu d'objets typés

Toute la communication JS ↔ Weaver passe par des strings JSON :
- `create(entityType: string, fieldsJson: string) → string`
- `drain() → string`
- `count(entityType: string) → string`

L'appelant doit `JSON.parse()` chaque retour.

**Pourquoi** : embind ne supporte pas facilement les `emscripten::val` (objets JS) depuis
du Rust via `extern "C"`. Il faudrait une couche C++ plus épaisse qui convertit les `val`
en params C avant d'appeler le Rust.

**Deux approches possibles** :

1. **Wrapper JS/TS** (pragmatique, recommandé) : une fine couche JS au-dessus de l'embind
   qui parse automatiquement les retours JSON et expose une API typée. Zéro changement
   côté Rust/C++. Peut aussi valider les paramètres en amont.

2. **C++ plus épais** : le C++ reçoit des `emscripten::val`, les convertit, appelle le
   Rust, reconvertit les retours en `val`. Plus performant (pas de JSON serialize/parse)
   mais beaucoup plus de code C++ à maintenir.

L'approche 1 est clairement préférable pour l'instant — le bottleneck n'est pas le
JSON parse mais les opérations DB/embed.

---

## 9. Pas de `rag3weaver_query()` pour les requêtes Cypher brutes

Le Weaver n'expose pas de méthode Cypher brut. L'appelant doit utiliser `Connection`
d'embind (classe séparée).

**Problème sous-jacent** : `Catalog` encapsule sa connexion dans un `Arc<dyn DbConnection>`
privé. Il n'y a pas de getter public.

**Conséquence** : le Weaver et la Connection utilisent des connexions DB différentes.
En in-memory, ce sont carrément des bases séparées. En fichier (IDBFS), elles partagent
les mêmes données mais via des handles distincts.

**Pistes** :
- Ajouter un `Catalog::execute()` qui délègue à `self.conn` → simple, casse un peu
  l'encapsulation
- Exposer la `CDatabase*` du Weaver pour que le C++ crée une deuxième Connection sur
  la même DB → complexe mais propre
- Ou accepter que le Weaver est un ingestion-only tool, et les requêtes passent par
  Connection (qui ouvre le même fichier DB)

---

## 10. Pas de link/search dans l'API FFI

L'API actuelle expose uniquement : `create()`, `drain()`, `count()`.

Manquent : `link()`, `search()`, `delete()`, `update()`.

Les fonctions existent dans `Catalog` côté Rust. Il faut juste ajouter les entry points
`extern "C"` correspondants + les wraps C++ embind. Pattern identique à `create()`.

---

## 11. `crate-type = ["lib", "staticlib"]` toujours actif

Cargo produit toujours les deux (`.rlib` + `.a`), même en natif. Cargo ne supporte pas
`crate-type` conditionnel par feature.

**Alternative** : utiliser `cargo rustc --crate-type=staticlib` dans le CMakeLists WASM
au lieu de l'inscrire dans Cargo.toml. Mais `cargo build` ne supporte pas `--crate-type`
directement — il faudrait passer par `cargo rustc`, ce qui complique la commande cmake.

**Impact** : ~1-2s de build natif supplémentaire. Négligeable, on garde l'approche actuelle.

---

## Résumé par gravité et prochaines étapes

| # | Concession | Gravité | Fix | Priorité |
|---|-----------|---------|-----|----------|
| 3 | Clé `"pending"` → `"persisted"` | **Bug** | FAIT | — |
| 6 | EmbedProcessor pas batch | **Bug perf** | Refactor process() | Haute |
| 2 | UUID vide → handle opaque | **Trompeur** | Retourner i64 handle | Haute |
| 1 | Fuite mémoire return_string_to_c | **Mineur** | Thread-local | Moyenne |
| 4 | catalog_new sans erreur | **Mineur** | last_error pattern | Moyenne |
| 5 | MockEmbedder en WASM | **Attendu** | Callback via Transformers.js | Phase 4 |
| 10 | Pas de link/search | **Attendu** | Ajouter entry points | Phase 4 |
| 9 | Pas de Cypher via Weaver | **Design** | Catalog::execute() | À discuter |
| 8 | API tout-JSON | **Design** | Wrapper TS | Phase 4 |
| 7 | unsafe Send+Sync | **Correct** | compile_error guard | Faible |
| 11 | staticlib build natif | **Négligeable** | — | — |

### Ordre de correction suggéré

1. **Immédiat** : #2 (temp_uuid), #1 (thread-local), #4 (last_error)
2. **Court terme** : #6 (batch embed — critique pour les perfs browser)
3. **Phase 4** : #5 (callback Transformers.js), #10 (link/search), #8 (wrapper TS)
