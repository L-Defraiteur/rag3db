# Findings — Crates Rust pour Rag3Weaver

Date : 14 fevrier 2026

---

## Stack recommandee

| Composant | Crate | Stars | WASM | Justification |
|-----------|-------|:-----:|:----:|---------------|
| Events/pub-sub | `async-broadcast` | 191 | Oui | Runtime-agnostic (smol-rs), zero dep tokio, broadcast multi-consumer |
| Events (fondation) | `event-listener` | 504 | Oui | Teste WASM (`wasm-bindgen-test` dans les devdeps), `no_std` |
| Chunking texte | `text-splitter` | 562 | Partiel | Markdown + code (tree-sitter) + texte. Maj 13 fev 2026 |
| Embeddings natif | `async-openai` | 1771 | Non | Full OpenAI API, compatible TEI (OpenAI-compatible endpoint) |
| Embeddings WASM | `async-openai-wasm` | 21 | Oui | Fork identique API, cible `wasm32-unknown-unknown` |
| UUID deterministe | `uuid` v5 | 1179 | Oui | `Uuid::new_v5(namespace, content)` — SHA-1, zero randomness |
| Content hashing | `blake3` | 6056 | Oui | 3x SHA-256, WASM SIMD (`wasm32_simd` feature), `no_std` |
| Score fusion | Roll-your-own | — | — | RRF = 20 lignes. Crate `rrf` 0.1.0 immature |

---

## Detail par composant

### 1. Events/Pub-sub — `async-broadcast`

**Pourquoi** : on a besoin de broadcast (chaque subscriber recoit tous les events). `tokio::sync::broadcast` ne compile pas en WASM browser. `async-broadcast` est runtime-agnostic, construit sur `event-listener` qui est explicitement teste WASM.

**Alternatives ecartees** :
- `tokio::sync::broadcast` : pas WASM
- `pharos` : stale (dernier commit oct 2022)
- `emitter-rs` : 1 star, utilise serde pour les payloads (overhead)

**Usage prevu** :

```rust
use async_broadcast::{broadcast, Sender, Receiver};

#[derive(Clone, Debug)]
pub enum CatalogEvent {
    EntityPrepared { entity: String, uuid: String },
    ChunksCreated { entity: String, count: usize },
    EmbeddingCompleted { batch_size: usize, duration_ms: u64 },
    DrainCompleted { stats: DrainStats },
    SearchCompleted { kb: String, results: usize, duration_ms: u64 },
    Error { context: String, message: String },
}

let (tx, rx) = broadcast::<CatalogEvent>(128);
```

**Impact architecture** : les events ne dependent plus de tokio. Le pipeline peut utiliser tokio (natif) ou `wasm-bindgen-futures` (WASM) pour l'async, mais le pub/sub est decouple du runtime.

---

### 2. Chunking — `text-splitter`

**Pourquoi** : le plus actif (maj hier), supporte markdown (CommonMark-aware), code (tree-sitter), et texte. Maximise la taille des chunks tout en respectant les limites semantiques.

**Ce qu'il fait** :
- Decoupe aux frontieres semantiques (phrases, paragraphes, mots)
- Feature `markdown` : respecte les headers, code blocks, listes
- Feature `code` : tree-sitter, decoupe par fonctions/classes
- Sizing par caracteres OU par tokens (tiktoken, HuggingFace)

**Ce qu'il ne fait PAS** (on devra wrapper) :
- Pas d'overlaps natifs — on doit implementer le sliding window
- Pas de tracking d'offsets (start_char, end_char, start_line, end_line) — on doit calculer les positions
- Pas de distinction core vs overlap offsets (pour l'affichage vs la recherche)

**WASM** : le mode caracteres fonctionne. Le backend `tiktoken-rs` a des problemes WASM. Le backend `tokenizers` (HuggingFace) a un feature `unstable_wasm` experimental. Pour WASM, on utilisera le sizing par caracteres.

**Alternatives ecartees** :
- `semchunk-rs` : overlap support mais moins mature, moins d'options
- `code-splitter` : stale (sept 2024), tree-sitter en C = probleme WASM
- `memchunk` : ultra-rapide mais basique, pas configurable
- Custom : possible mais `text-splitter` fait 80% du travail

**Usage prevu** :

```rust
use text_splitter::TextSplitter;

// Wrapper qui ajoute overlaps + offset tracking
pub struct SemanticChunker {
    splitter: TextSplitter,
    overlap: usize,
    // ...
}
```

---

### 3. Embeddings — `async-openai` + `async-openai-wasm`

**Pourquoi** : meme API pour natif et WASM, dual-target resolu via `cfg`. Compatible avec tout endpoint OpenAI-compatible (TEI, vLLM, Ollama, etc.).

**Pattern dual-target** :

```rust
// Cargo.toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
async-openai = "0.x"

[target.'cfg(target_arch = "wasm32")'.dependencies]
async-openai-wasm = "0.x"

// embedder.rs
#[cfg(not(target_arch = "wasm32"))]
use async_openai as openai;
#[cfg(target_arch = "wasm32")]
use async_openai_wasm as openai;
```

**Alternatives ecartees** :
- `genai` (652 stars) : multi-provider mais WASM status inconnu, principalement chat
- `fastembed` (756 stars) : embeddings locaux (ONNX), pas WASM, gros binaire
- `embed_anything` (1155 stars) : multi-modal, complexe, pas WASM

**Note TEI** : TEI expose un endpoint compatible OpenAI (`/v1/embeddings`). Donc `async-openai` fonctionne directement avec TEI sans client dedie.

---

### 4. UUID — `uuid` v5

**Pourquoi** : standard, deterministe, WASM natif (pas de randomness), 1179 stars, mis a jour hier.

**HASHSAFE pattern** :

```rust
use uuid::Uuid;

const RAG3WEAVER_NS: Uuid = Uuid::new_v5(&Uuid::NAMESPACE_URL, b"rag3weaver");

fn hashsafe_uuid(entity: &str, fields: &[&str]) -> Uuid {
    let input = format!("{}:{}", entity, fields.join(":"));
    Uuid::new_v5(&RAG3WEAVER_NS, input.as_bytes())
}
```

v5 utilise SHA-1 (pas securitaire mais parfait pour le content-addressing). Si on veut plus fort, on peut hasher avec blake3 et construire un UUID v8 custom :

```rust
let hash = blake3::hash(content.as_bytes());
let bytes: [u8; 16] = hash.as_bytes()[..16].try_into().unwrap();
let id = uuid::Builder::from_custom_bytes(bytes).into_uuid();
```

---

### 5. Content hashing — `blake3`

**Pourquoi** : 3x plus rapide que SHA-256, WASM SIMD accelere, `no_std`, 6056 stars.

**Usage** : deduplication de contenu (`_content_hash` dans les tables).

```rust
fn content_hash(text: &str) -> String {
    blake3::hash(text.as_bytes()).to_hex().to_string()
}
```

**WASM** : feature `wasm32_simd` active les instructions SIMD pour les navigateurs modernes. Fallback pur Rust sinon.

---

### 6. Score fusion — Custom (20 lignes)

Le crate `rrf` est en 0.1.0 avec 67% de docs. Pas worth une dependance.

```rust
pub fn rrf_fuse(ranked_lists: &[Vec<(String, f32)>], k: f32) -> Vec<(String, f32)> {
    let mut scores: HashMap<String, f32> = HashMap::new();
    for list in ranked_lists {
        for (rank, (id, _)) in list.iter().enumerate() {
            *scores.entry(id.clone()).or_default() += 1.0 / (k + rank as f32 + 1.0);
        }
    }
    let mut results: Vec<_> = scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    results
}

pub fn weighted_fuse(vector_score: f32, bm25_score: f32, keyword_weight: f32) -> f32 {
    (1.0 - keyword_weight) * vector_score + keyword_weight * bm25_score
}

pub fn boost_fuse(vector_score: f32, bm25_normalized: f32, boost_factor: f32) -> f32 {
    vector_score * (1.0 + bm25_normalized * boost_factor)
}
```

---

### 7. RAG framework — Inspiration seulement

**`rig-core`** (5935 stars, WASM compatible) est le plus gros framework RAG Rust. Utile pour :
- S'inspirer des traits (`EmbeddingModel`, `VectorStoreIndex`)
- Voir comment ils gerent le dual-target WASM/natif
- Patterns d'abstraction provider

**On ne l'utilise PAS comme dependance** car :
- On a notre propre stack (rag3db + lucivy_fts)
- Ses vector stores sont des backends externes (Qdrant, LanceDB) — on a deja HNSW dans rag3db
- Ajouter rig-core tirerait un arbre de deps enorme

**`swiftide`** (661 stars) : bon modele pour le pipeline d'indexation (streaming, parallel, async) mais pas WASM.

---

## Impact sur les decisions prises

### Decision 3 (Async) — Ajustement

On avait choisi "tokio d'emblee". Avec `async-broadcast` qui est runtime-agnostic, on peut decoupler :
- **Events** : `async-broadcast` (fonctionne avec n'importe quel runtime)
- **Pipeline** : `tokio` en natif, `wasm-bindgen-futures` en WASM
- **HTTP** : `reqwest` (tokio natif, fetch en WASM)

Concretement : le code metier (events, fusion, chunking) n'importe pas tokio. Seul le code I/O (pipeline, HTTP, timers) utilise le runtime, isole derriere des traits.

### Nouvelle contrainte WASM

Le dual-target WASM/natif impose des `cfg(target_arch)` a 3 endroits :
1. `async-openai` vs `async-openai-wasm`
2. Timers : `tokio::time` vs `gloo-timers`
3. I/O : `std::fs` vs IDBFS/MemFS

Tout le reste (`async-broadcast`, `text-splitter` chars, `uuid` v5, `blake3`, fusion) est naturellement cross-platform.

---

## Cargo.toml prevu

```toml
[package]
name = "rag3weaver"
version = "0.1.0"
edition = "2021"

[dependencies]
# Events (runtime-agnostic)
async-broadcast = "0.7"

# Config
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Chunking
text-splitter = { version = "0.18", features = ["markdown"] }

# UUID + Hashing
uuid = { version = "1", features = ["v5", "serde"] }
blake3 = "1"

# Lucivy (meme workspace)
lucivy-fts = { path = "../lucivy_fts/rust" }

# Code parsing (JS runtime embarque pour Rust standalone)
rquickjs = { version = "0.11", features = ["full-async"], optional = true }

# Async
futures-core = "0.3"

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
async-openai = "0.26"
tokio = { version = "1", features = ["rt", "time", "sync"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
async-openai-wasm = "0.26"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
gloo-timers = { version = "0.3", features = ["futures"] }

[features]
default = ["embedded-js"]
embedded-js = ["rquickjs"]  # Pour Rust standalone (pas de host JS)
```

Note : `rquickjs` est optionnel. En WASM/Node.js, le code parsing passe par callbacks
vers le host JS (V8 natif). `rquickjs` n'est necessaire que pour le mode Rust standalone
sans host JS (~1 MB d'overhead, QuickJS embarque).
