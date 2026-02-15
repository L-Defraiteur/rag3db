# Rag3Weaver — Plan exemples Embedder + candle par defaut (15 fevrier 2026)

Date : 15 fevrier 2026
Statut : reflexion / plan

---

## Contexte

On a le trait `Embedder` + `CallbackEmbedder` dans rag3weaver. La lib est zero dep ML. L'utilisateur fournit son propre embedder. Mais si l'utilisateur n'en specifie pas, il se retrouve sans rien — mauvaise DX.

## Ambition

### Principe : candle integre comme provider par defaut

On integre candle + tokenizers directement dans rag3weaver, derriere un **feature flag** (active par defaut). Ca donne un embedder local pret a l'emploi — zero config, zero API, fonctionne natif ET WASM.

```toml
[features]
default = ["candle-embedder"]
candle-embedder = ["dep:candle-core", "dep:candle-nn", "dep:candle-transformers", "dep:tokenizers", "dep:hf-hub"]
```

L'utilisateur qui veut pas de candle fait :
```toml
rag3weaver = { version = "0.1", default-features = false }
```

### Comportement souhaite

```rust
// Option 1 : l'utilisateur specifie rien → candle par defaut
let catalog = Catalog::builder(config)
    .connection(conn)
    .build(); // utilise CandleEmbedder::default() (all-MiniLM-L6-v2, dim 384)

// Option 2 : l'utilisateur fournit son propre embedder (API, callback, whatever)
let catalog = Catalog::builder(config)
    .connection(conn)
    .embedder(Box::new(my_custom_embedder))
    .build();

// Option 3 : callback rapide
let catalog = Catalog::builder(config)
    .connection(conn)
    .embedder_fn(384, |texts| Box::pin(async move { /* ... */ }))
    .build();
```

Si le feature `candle-embedder` est active et que l'utilisateur ne specifie pas d'embedder, on utilise `CandleEmbedder` avec `all-MiniLM-L6-v2` (384 dims, ~23MB de modele). Le modele est telecharge au premier appel (via hf-hub en natif, ou charge depuis une URL en WASM).

Si le feature est desactive et que l'utilisateur ne specifie pas d'embedder → erreur a l'init.

---

## Les 3 exemples concrets

### Environnement de test

- **TEI Docker** : `ragforge-tei` sur port 8081, modele `BAAI/bge-base-en-v1.5` (dim 768, float16)
- **Endpoint** : `http://localhost:8081/v1/embeddings` (compatible OpenAI)

### Exemple 1 : `examples/tei_reqwest.rs` — reqwest direct vers TEI

Le plus simple. Un POST JSON vers `/v1/embeddings`. Pas de dep SDK, juste reqwest.

```rust
struct TeiEmbedder {
    client: reqwest::Client,
    url: String,    // "http://localhost:8081/v1/embeddings"
    model: String,  // "BAAI/bge-base-en-v1.5"
    dim: usize,     // 768
}
```

- Dep : `reqwest = { version = "0.12", features = ["json"] }`
- ~50 lignes de code
- Construit le JSON `{"input": [...], "model": "..."}`, parse la reponse
- Gestion erreur HTTP → `EmbedError::ProviderError`

### Exemple 2 : `examples/tei_openai.rs` — async-openai vers TEI

Meme TEI mais via le SDK async-openai. Montre que TEI est compatible OpenAI API.

```rust
struct OpenAIEmbedder {
    client: async_openai::Client<async_openai::config::OpenAIConfig>,
    model: String,
    dim: usize,
}
```

- Dep : `async-openai = "0.30"`
- ~40 lignes de code
- `client.embeddings().create(request).await`
- Montre le pattern pour OpenAI, Azure, ou tout endpoint compatible

### Exemple 3 : `examples/candle_local.rs` — candle local

Embeddings locaux, aucun serveur requis. Charge un modele sentence-transformers en safetensors.

```rust
struct CandleEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    dim: usize,     // 384 pour all-MiniLM-L6-v2
    device: Device,
}
```

- Deps : `candle-core`, `candle-nn`, `candle-transformers`, `tokenizers`, `hf-hub`
- ~100-150 lignes de code
- Telecharge le modele via hf-hub au premier lancement
- Tokenize → forward pass → mean pooling → normalize
- Fonctionne natif ET WASM (candle compile en WASM)
- **C'est cet exemple qui deviendra le `CandleEmbedder` integre dans la lib**

---

## Cargo.toml prevu

```toml
[package]
name = "rag3weaver"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core (toujours present)
async-broadcast = "0.7"
blake3 = "1"
text-splitter = { version = "0.28", features = ["markdown"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
async-trait = "0.1"
thiserror = "2"
tokio = { version = "1", features = ["sync"] }

# Candle embedder (feature flag, active par defaut)
candle-core = { version = "0.8", optional = true }
candle-nn = { version = "0.8", optional = true }
candle-transformers = { version = "0.8", optional = true }
tokenizers = { version = "0.21", optional = true, default-features = false }
hf-hub = { version = "0.4", optional = true }

[features]
default = ["candle-embedder"]
candle-embedder = ["dep:candle-core", "dep:candle-nn", "dep:candle-transformers", "dep:tokenizers", "dep:hf-hub"]

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
reqwest = { version = "0.12", features = ["json"] }
async-openai = "0.30"

[[example]]
name = "tei_reqwest"

[[example]]
name = "tei_openai"

[[example]]
name = "candle_local"
required-features = ["candle-embedder"]
```

Les exemples reqwest et openai sont en `dev-dependencies` (pas dans la lib finale). Candle est en `dependencies` optionnelle (dans la lib quand le feature est active).

---

## Plan d'execution

1. **Exemple tei_reqwest** : le plus simple, valide que TEI repond, sert de smoke test
2. **Exemple tei_openai** : meme test via async-openai, valide la compatibilite
3. **Exemple candle_local** : le plus complexe, charge le modele, tokenize, inference
4. **Integration candle dans la lib** : promouvoir l'exemple en `src/candle_embedder.rs` derriere le feature flag, avec `CandleEmbedder::default()` qui charge all-MiniLM-L6-v2
5. **Tests reels** : utiliser les exemples comme base pour des tests d'integration (embed vrai texte, verifier dimensions, verifier que cosine similarity fonctionne)

---

## Decisions ouvertes

- **Version candle** : 0.8.x (stable) ou 0.9.x (alpha) ? 0.8.3 est la derniere stable.
- **Modele par defaut** : `all-MiniLM-L6-v2` (384 dims, 23MB) semble le bon choix — petit, rapide, qualite correcte. Mais `bge-base-en-v1.5` (768 dims) est ce qu'on a dans TEI.
- **WASM + candle** : le telechargemement du modele en WASM passe par fetch (pas hf-hub). Il faudra un cfg(target_arch) pour le loading, pas pour l'inference.
- **Tokenizers WASM** : le crate `tokenizers` a un feature `unstable_wasm`. A tester.
