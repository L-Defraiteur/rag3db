# Rag3Weaver — Etape 0 : Squelette (events + config + traits)

Date : 15 fevrier 2026
Statut : FAIT

---

## Contexte

Rag3Weaver est le futur orchestrateur du pipeline RAG pour rag3db. Il gere l'ingestion (chunking, UUID, embeddings), le stockage batch, la recherche hybride (vector + FTS) et l'exploration de graphe. C'est un portage en Rust du module TypeScript existant (`ragforge-core-exp-kuzu/kuzu-wasm-exp/src/`).

L'Etape 0 pose le squelette : systeme d'events, structs de config serde, traits async pour l'embedder et la connexion DB, avec 35 tests unitaires.

---

## Emplacement

```
packages/rag3db/extension/rag3weaver/    ← Crate Rust standalone
├── Cargo.toml
└── src/
    ├── lib.rs            (mod declarations + re-exports)
    ├── events.rs         (CatalogEvent enum + EventBus)
    ├── config.rs         (CatalogConfig + tous les sous-types serde)
    ├── embedder.rs       (trait Embedder async + MockEmbedder)
    └── connection.rs     (trait DbConnection async + CypherValue + MockConnection)
```

**Pourquoi dans `extension/` et pas dans `ld-lucivy/`** : rag3weaver utilise rag3db ET lucivy_fts ET l'extension vector. C'est un consommateur de ces composants, pas un composant de Lucivy. Le placer dans ld-lucivy (qui est un submodule git) serait semantiquement faux et compliquerait le versioning.

**Pas encore une extension rag3db** : pour la v1, c'est une crate Rust standalone. L'integration comme extension C++ (fonctions Cypher CREATE_CATALOG, CATALOG_SEARCH) viendra en v2 si le besoin se confirme.

---

## Dependencies

```toml
[dependencies]
async-broadcast = "0.7"     # Events pub/sub, runtime-agnostic, WASM-compatible
serde = { version = "1", features = ["derive"] }
serde_json = "1"
async-trait = "0.1"          # Traits async object-safe
thiserror = "2"              # Error types propres

[dev-dependencies]
tokio = { version = "1", features = ["rt", "macros", "time"] }  # Tests async uniquement
```

**Pas de tokio en dep principale** : la crate est runtime-agnostic pour rester compatible WASM. Seuls les tests utilisent tokio.

---

## Module events.rs

### CatalogEvent — 16 variantes

| Categorie | Events |
|-----------|--------|
| Pipeline | `EntityPrepared`, `ChunksCreated`, `EmbeddingStarted`, `EmbeddingCompleted`, `EntitiesStored`, `RelationsLinked` |
| Drain | `DrainStarted` (avec tailles des queues), `DrainCompleted` (avec `DrainStats`) |
| Search | `SearchStarted`, `SearchCompleted` |
| Error | `Error { context, message }` |
| Entity lifecycle | `EntityCreated`, `EntityUpdated`, `EntityDeleted` |

### EventBus — async_broadcast wrapper

```rust
pub struct EventBus {
    sender: Sender<CatalogEvent>,
    _inactive: InactiveReceiver<CatalogEvent>,  // garde le channel ouvert
}

impl EventBus {
    pub fn new(capacity: usize) -> Self;
    pub fn subscribe(&self) -> Receiver<CatalogEvent>;
    pub fn emit(&self, event: CatalogEvent);  // synchrone, fire-and-forget
}
```

**Decisions cles** :
- `emit()` est **synchrone** via `try_broadcast()` — pas besoin d'async pour du monitoring fire-and-forget. Permet d'emettre depuis du code sync sans runtime async.
- `set_overflow(true)` : quand le buffer est plein, le plus ancien message est drop. Le pipeline ne bloque jamais a cause d'un subscriber lent.
- `InactiveReceiver` garde le channel ouvert meme sans subscribers actifs.
- async_broadcast notifie les receivers quand des messages sont perdus via `Err(Overflowed(n))`.

---

## Module config.rs

### Structs principales

| Struct | Role | Defaults |
|--------|------|----------|
| `CatalogConfig` | Config top-level | embedding_dim=384 |
| `EntityDef` | Definition d'entite | fields HashMap, hashsafe optionnel |
| `FieldDef` | Definition de champ | field_type=String, chunked=false |
| `RelationDef` | Definition de relation | from, to, properties optionnelles |
| `KBConfig` | Config knowledge base | search=Hybrid, keyword_weight=0.3, title_boost=2.0 |
| `ChunkingConfig` | Config chunking | enabled=true, max_size=1500, overlap=200, strategy=Semantic |
| `EmbeddingConfig` | Config provider | provider, model, max_input_tokens (tous optionnels) |
| `FlushConfig` | Config auto-flush | auto=true, max_count=50, max_delay=100ms, batch_size=32 |

### Enums

- `FieldType` : String, Text, Int64, Integer, Double, Number, Boolean, Timestamp, Json, Tags, Choice
- `SearchMode` : Hybrid, Semantic, Fulltext
- `ChunkStrategy` : Semantic, Fixed, Sentence

### Serde dual camelCase/snake_case

Toutes les structs utilisent `#[serde(rename_all = "camelCase")]` comme format primaire, avec `#[serde(alias = "snake_case")]` sur chaque champ pour accepter les deux formats en input. Compatible avec le TypeScript qui utilise les deux.

### Custom deserializer pour content_for

Le champ `content_for` du TS accepte `"main"` (string) ou `["main", "summary"]` (array). Un deserializer custom `deserialize_string_or_vec` gere les deux cas.

---

## Module embedder.rs

### trait Embedder

```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError>;
    fn dim(&self) -> usize;
}
```

### EmbedError (thiserror)

- `ProviderError(String)` — erreur du provider HTTP
- `DimensionMismatch { expected, got }` — vecteur de mauvaise taille
- `BatchTooLarge { max, got }` — trop de textes dans le batch
- `Timeout` — timeout de la requete

### MockEmbedder

Retourne des vecteurs zero de la bonne dimension. Dans le crate principal (pas dev-only) pour etre reutilisable par les tests downstream.

---

## Module connection.rs

### CypherValue — union typee JSON-compatible

```rust
#[serde(untagged)]
pub enum CypherValue {
    Null, Bool(bool), Int(i64), Float(f64),
    String(String), List(Vec<CypherValue>), Map(HashMap<String, CypherValue>),
}
```

**`#[serde(untagged)]`** pour du JSON naturel : `42` au lieu de `{"Int": 42}`. L'ordre des variantes est important : Int avant Float pour que `42` parse en i64.

Helpers : `as_str()`, `as_i64()`, `as_f64()`, `as_bool()`, `is_null()`.
Conversions From : `&str`, `String`, `i64`, `f64`, `bool`, `Vec<T>`.

### trait DbConnection

```rust
#[async_trait]
pub trait DbConnection: Send + Sync {
    async fn execute(&self, cypher: &str) -> Result<QueryResult, DbError>;
    async fn execute_with_params(&self, cypher: &str, params: &[QueryParam]) -> Result<QueryResult, DbError>;
}
```

### MockConnection

Retourne `QueryResult::default()` (vide) pour tout. Reutilisable par les tests downstream.

---

## Tests — 35 au total

| Module | Tests | Couverture |
|--------|:-----:|------------|
| events | 5 | emit/receive, multi-subscribers, overflow, no-subscriber, DrainStats |
| config | 11 | defaults, roundtrip, snake_case, content_for (absent/string/array), field types, chunking, flush, relations |
| embedder | 5 | dimensions, zero vectors, empty batch, error display, trait object |
| connection | 14 | CypherValue (null/string/int/float/bool/list/map/serde), QueryParam, QueryResult, MockConnection, trait object, DbError display |

```bash
cd packages/rag3db/extension/rag3weaver && cargo test
# 35 passed, 0 failed, 0.00s
```

---

## Prochaines etapes

### Etape 1 — Logique pure (zero DB, zero async)
- `schema.rs` : config → Cypher DDL
- `chunker.rs` : SemanticChunker (offsets, overlaps, strategies)
- `uuid.rs` : HASHSAFE (SHA256 → UUID deterministe)
- `hash.rs` : content hashing (blake3)
- `filter.rs` : config filtres → WHERE Cypher
- `fusion.rs` : scoring (boost, RRF, weighted)

### Etape 2 — Catalog CRUD + pipeline async + persistence
### Etape 3 — Search + Explore via Lucivy direct
### Etape 4 — Providers embedding + code parsing (callbacks TS)
### Etape 5 — WASM + Tests E2E

Docs de design detailles : `packages/rag3db/docs/14-fevrier-2026-22h57/01-rag3weaver-port-to-rag3db.md`
Findings crates : `packages/rag3db/docs/14-fevrier-2026-22h57/03-findings-crates-ecosystem.md`
