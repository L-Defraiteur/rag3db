# Rag3Weaver — Plan bridge DbConnection réel (15 février 2026)

Date : 15 février 2026
Statut : plan / prochaine étape

---

## Contexte

rag3weaver a 21 modules, 265 tests (260 réguliers + 5 intégration candle), tout vert.
Tout tourne contre MockConnection. Le prochain gros morceau : brancher une vraie DB.

### Ce qui est fait dans cette session

1. **BM25 NgramContains** : `parse:` remplacé par `contains:`/`regex:` (JSON QueryConfig). BM25Mode enum, build_bm25_query(), fuzzy_distance exposé dans les deux modes. +5 tests.

2. **CallbackEmbedder** : EmbedFn type alias, CallbackEmbedder struct, trait object compatible. +4 tests.

3. **CandleEmbedder intégré** : feature flag `candle-embedder` (actif par défaut). DefaultModel::BgeBase (768 dims, ~110MB) par défaut, MiniLM en option (384 dims, 23MB). Téléchargement HF Hub, mean pooling + L2 normalize. +8 tests (3 unit + 5 intégration #[ignore]).

4. **3 exemples fonctionnels** :
   - `examples/tei_reqwest.rs` — reqwest direct vers TEI (port 8081, bge-base-en-v1.5)
   - `examples/tei_openai.rs` — async-openai SDK vers TEI
   - `examples/candle_local.rs` — CandleEmbedder local, teste MiniLM + BgeBase

5. **Cargo.toml mis à jour** : candle deps optionnels, reqwest/async-openai en dev-deps, feature flag, sections [[example]].

---

## Prochaine étape : bridge DbConnection réel

### Le problème

Le trait `DbConnection` (connection.rs) est l'abstraction :
```rust
#[async_trait]
pub trait DbConnection: Send + Sync {
    async fn execute(&self, cypher: &str) -> Result<QueryResult, DbError>;
    async fn execute_with_params(&self, cypher: &str, params: &[QueryParam]) -> Result<QueryResult, DbError>;
}
```

Actuellement seul `MockConnection` existe. Il faut un `Rag3dbConnection` qui parle à la vraie DB.

### Découverte clé : le crate rag3db Rust existe déjà

`packages/rag3db/tools/rust_api/` contient un crate Rust officiel (`rag3db` v0.11.1) qui :
- Utilise cxx bridge vers le C++ de rag3db
- Fournit `Database::new(path, config)` / `Database::in_memory(config)`
- Fournit `Connection::new(&db)` — `Send + Sync`
- `Connection::query(cypher)` → `QueryResult` (Iterator<Item = Vec<Value>>)
- `Connection::prepare(cypher)` + `Connection::execute(&mut stmt, params)` — prepared statements
- `Value` enum : Bool, Int64, Int32, Double, Float, String, List, Array, Struct, Node, Rel, Null, etc.
- Build via CMake (compile rag3db C++ from source) ou lien vers pre-built (RAG3DB_LIBRARY_DIR)

### Approche choisie : Option A — feature flag `rag3db-native`

rag3weaver dépend optionnellement du crate rag3db (path dependency) :

```toml
[dependencies]
rag3db = { path = "../../tools/rust_api", optional = true }

[features]
rag3db-native = ["dep:rag3db"]
```

Fournit `Rag3dbConnection` qui implémente `DbConnection` :

```rust
// src/rag3db_connection.rs (derrière #[cfg(feature = "rag3db-native")])

pub struct Rag3dbConnection {
    // Connection<'a> a un lifetime lié à Database
    // On possède les deux, Database droppée après Connection
    // Connection est Send+Sync (synchronisé côté C++)
    // query() prend &self (via UnsafeCell interne)
    db: Box<rag3db::Database>,
    conn: rag3db::Connection<'static>, // lifetime étendu, sound car db vit plus longtemps
}
```

### Mapping de types

| rag3db::Value | CypherValue |
|---------------|-------------|
| String(s) | String(s) |
| Int64(i) | Int(i) |
| Int32/16/8(i) | Int(i as i64) |
| UInt64/32/16/8(u) | Int(u as i64) |
| Double(f) | Float(f) |
| Float(f) | Float(f as f64) |
| Bool(b) | Bool(b) |
| Null(_) | Null |
| List(_, vs) | List(convert each) |
| Array(_, vs) | List(convert each) |
| Node(n) | Map(properties as HashMap) |
| Struct(fields) | Map(fields as HashMap) |
| other | String(format!("{}", v)) — fallback |

### Challenges identifiés

1. **Self-referential struct** : `Connection<'a>` emprunte `&'a Database`. Pour stocker les deux dans un struct sans lifetime, il faut un `unsafe` lifetime extension ou un pattern comme `owning_ref`. Sound car on garantit que db est droppé après conn.

2. **Sync query dans async trait** : rag3db Connection::query est sync. Le trait DbConnection est async. On wrappe simplement (pas de spawn_blocking pour l'instant, le query est rapide).

3. **Build system** : le crate rag3db compile tout rag3db via CMake. C'est lourd (~5-10 min le premier build). Alternative : pré-builder et pointer RAG3DB_LIBRARY_DIR vers `build/release/src/`.

4. **Extensions** : pour que LOAD EXTENSION marche, il faut `-rdynamic` dans le linker (build.rs du crate rag3db le fait déjà en mode static). tantivy_fts doit être loadable.

### Plan d'exécution

1. Créer `src/rag3db_connection.rs` derrière `#[cfg(feature = "rag3db-native")]`
2. Implémenter le mapping Value → CypherValue
3. Implémenter `Rag3dbConnection::new(path)` et `::in_memory()`
4. Implémenter `DbConnection` pour `Rag3dbConnection`
5. Ajouter au Cargo.toml la dep optionnelle + feature
6. Écrire des tests d'intégration (#[ignore]) qui :
   - Créent une DB in-memory
   - Créent un schema (Node tables, Rel tables)
   - Insèrent des données
   - Querent et vérifient les résultats
   - Testent les prepared statements avec params
7. Tester avec l'extension tantivy_fts chargée (LOAD EXTENSION)
8. Éventuellement : test E2E complet (Catalog + Rag3dbConnection + CandleEmbedder)

### Architecture résultante

```
rag3weaver (crate Rust)
├── src/
│   ├── catalog.rs           — orchestrateur (utilise DbConnection trait)
│   ├── search.rs            — hybrid search (BM25 + vector)
│   ├── candle_embedder.rs   — [feature candle-embedder] embedder local
│   ├── rag3db_connection.rs — [feature rag3db-native] bridge vers rag3db
│   ├── connection.rs        — trait DbConnection + MockConnection
│   ├── embedder.rs          — trait Embedder + CallbackEmbedder
│   └── ...                  — 15 autres modules
├── examples/
│   ├── tei_reqwest.rs
│   ├── tei_openai.rs
│   └── candle_local.rs
└── Cargo.toml
    features:
      default = ["candle-embedder"]
      candle-embedder = [candle deps]
      rag3db-native = [rag3db crate]
```

L'utilisateur choisit :
- `default` → candle embedder, pas de DB native (pour WASM, pour tests)
- `rag3db-native` → DB native en process (pour applis Rust standalone, pour tests E2E)
- `default-features = false` → lib pure, l'utilisateur fournit tout via traits

### Plus tard (Option B)

Si besoin de découplage total, on pourra extraire `Rag3dbConnection` dans un crate séparé `rag3weaver-rag3db`. Mais pour l'instant Option A suffit.

---

## Décisions ouvertes

- **Pre-built vs from-source** : compiler rag3db from source est lent. Pointer vers le build existant (`build/release/src/`) via env vars serait plus rapide pour le dev.
- **Self-referential struct** : quel pattern utiliser ? unsafe transmute, owning_ref, ou restructurer pour que l'utilisateur passe Database + Connection séparément ?
- **WASM** : le feature `rag3db-native` est explicitement incompatible WASM. En WASM, on utilise CallbackConnection ou un futur bridge wasm-bindgen.
