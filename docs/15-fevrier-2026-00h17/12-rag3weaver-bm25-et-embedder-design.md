# Rag3Weaver — BM25 NgramContains + Embedder callback (15 fevrier 2026)

Date : 15 fevrier 2026
Statut : 257 tests, 20 modules

---

## Deux changements dans cette iteration

### 1. BM25 : `contains:` au lieu de `parse:`

Le search.rs v1 generait un appel QUERY_LUCIVY_INDEX avec `parse:query` — le query parser standard de Lucivy (AND/OR, multi-mots). Mais notre feature principale dans ld-lucivy c'est **NgramContainsQuery** : fuzzy substring + trigram + BM25 scoring.

**Avant** :
```
CALL QUERY_LUCIVY_INDEX('Document', 'parse:hello world', 10) RETURN _uuid, _score
```

**Apres** :
```
CALL QUERY_LUCIVY_INDEX('Document', '{"type":"contains","field":"body","value":"hello world","distance":1}', 10) RETURN node_id, score
```

Le format est un JSON `QueryConfig` qui correspond exactement au struct Rust `query.rs::QueryConfig` dans lucivy_fts. C'est le meme format utilise dans les 9 tests GTest E2E.

#### Changements

- **`BM25Mode` enum** : `Contains` (defaut) | `Regex`
  - `Contains` : NgramContainsQuery fuzzy — substring + trigram + Levenshtein + BM25
  - `Regex` : NgramContainsQuery regex — trigram-accelerated regex + hybrid fuzzy si distance > 0
  - `parse:` supprime — inutile dans un contexte RAG (pas de AND/OR), et problematique (les caracteres speciaux comme `c++` cassent le query parser)

- **`SearchOptions`** : +`bm25_mode: BM25Mode` et +`fuzzy_distance: u8` (defaut 1)
  - `fuzzy_distance` s'applique dans les DEUX modes (Contains + Regex)
  - En mode Regex avec distance > 0 : mode hybride (regex OU fuzzy sur les literals extraits)

- **`build_bm25_query()`** : construit le JSON QueryConfig
  - 1 champ : `{"type":"contains","field":"f","value":"q","distance":1}`
  - N champs : `{"type":"boolean","should":[...]}`
  - Mode Regex : ajoute `"regex":true`

- **`search_bm25()`** : nouvelle signature
  ```rust
  pub async fn search_bm25(
      conn, entity, query, fields: &[String], mode: BM25Mode,
      fuzzy_distance: u8, limit,
  ) -> Result<Vec<SearchResult>>
  ```
  Les `fields` sont extraits du KBMetadata (title + content fields pour l'entite).

- **`Catalog::search()`** : extrait les champs texte du KB et les passe a search_bm25

#### Tests ajoutes (+5)

| Test | Verifie |
|------|---------|
| `search_bm25_empty_fields` | fields vide → resultat vide |
| `build_bm25_query_single_field_contains` | JSON correct : type, field, value, distance |
| `build_bm25_query_single_field_regex` | JSON correct avec `"regex":true` |
| `build_bm25_query_multi_field_boolean` | JSON boolean+should, 2 clauses, distance=2 |
| `build_bm25_query_multi_field_regex` | Multi-field + regex:true sur chaque clause |

---

### 2. Embedder : architecture callback, zero dep ML

#### Decision de design

**La crate rag3weaver n'a AUCUNE dependance sur un provider d'embeddings.**

Pas de candle, pas d'async-openai, pas de reqwest pour les embeddings. La lib est purement un orchestrateur. L'utilisateur fournit son propre `Embedder`.

#### Pourquoi

- **WASM + natif** : en browser, on veut Transformers.js (JS) ou candle (WASM). En natif, on veut une API (TEI, OpenAI) ou candle local. Impossible de satisfaire tous les cas avec une seule dep.
- **Taille binaire** : candle + tokenizers = ~2-3MB WASM. Si l'utilisateur utilise deja Transformers.js cote JS, c'est du poids mort.
- **Flexibilite** : l'utilisateur peut utiliser n'importe quel provider (candle, ort, API custom, Transformers.js via wasm-bindgen).
- **Decouplage** : la lib ne se casse pas quand un provider change d'API.

#### Le trait Embedder (existant)

```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError>;
    fn dim(&self) -> usize;
}
```

C'est deja l'abstraction callback. L'utilisateur implemente ce trait sur son struct.

#### CallbackEmbedder (nouveau)

Raccourci pour ne pas forcer l'utilisateur a creer un struct + impl :

```rust
let embedder = CallbackEmbedder::new(384, |texts| {
    Box::pin(async move {
        // candle, API, Transformers.js, whatever
        Ok(texts.iter().map(|_| vec![0.0f32; 384]).collect())
    })
});
let catalog = Catalog::new(conn, Box::new(embedder), config);
```

Types exposes :
- `CallbackEmbedder` — struct qui wraps un `EmbedFn`
- `EmbedFn` — type alias pour la closure async
- `EmbedError` — enum d'erreurs (deja existant, maintenant re-exporte)

#### Implementations prevues (hors de la crate)

| Impl | Ou | Contexte | Dep |
|------|----|----------|-----|
| `MockEmbedder` | dans la crate | Tests | aucune |
| `CallbackEmbedder` | dans la crate | Usage rapide | aucune |
| Candle embedder | crate separee ou exemple | Local natif + WASM | candle, tokenizers |
| API embedder | crate separee ou exemple | TEI, OpenAI, Ollama | reqwest |
| JS embedder | exemple WASM | Browser via Transformers.js | wasm-bindgen |

L'experimentation browser existante (`rag-demo.html`) utilisait Transformers.js + onnxruntime-web avec les modeles :
- `all-MiniLM-L6-v2` (23MB, dim 384, rapide)
- `bge-small-en-v1.5` (33MB, meilleure qualite)
- `gte-small` (30MB, bon equilibre)
- `multilingual-e5-small` (118MB, multilingue)

Tous ces modeles sont utilisables via candle (format safetensors) ou Transformers.js (format ONNX). Le choix se fait a l'initialisation du Catalog, pas dans la lib.

#### Tests ajoutes (+4)

| Test | Verifie |
|------|---------|
| `callback_embedder_basic` | Construction + embed retourne les bons vecteurs |
| `callback_embedder_error` | La closure peut retourner une erreur |
| `callback_embedder_as_trait_object` | Compatible Box<dyn Embedder> |
| `callback_embedder_empty_batch` | Batch vide → resultat vide |

---

## Bilan

```
20 modules, 257 tests (+9 depuis L3c search)
```

| Changement | Tests |
|------------|:-----:|
| BM25 NgramContains + build_bm25_query | +5 |
| CallbackEmbedder | +4 |
| **Total session** | **+9** |

### Re-exports ajoutes a lib.rs

```rust
pub use embedder::{CallbackEmbedder, EmbedError, EmbedFn, Embedder};
pub use search::BM25Mode;
```

---

## Fichiers modifies

| Fichier | Action |
|---------|--------|
| `src/search.rs` | Modifie — BM25Mode enum, build_bm25_query(), search_bm25() reecrit, SearchOptions +bm25_mode +fuzzy_distance, +5 tests |
| `src/catalog.rs` | Modifie — bm25_fields extraction, passage des nouveaux params a search_bm25 |
| `src/embedder.rs` | Modifie — CallbackEmbedder + EmbedFn + doc module, +4 tests |
| `src/lib.rs` | Modifie — re-exports BM25Mode, CallbackEmbedder, EmbedError, EmbedFn |

---

## Prochaines etapes

- **Phase C** : Integration Node.js / WASM — le vrai `DbConnection` qui parle a rag3db
- **Exemples** : candle embedder, API embedder, JS embedder (hors crate)
- **Filters dans search** : integrer FilterParser pour generer des WHERE clauses
- **search_time_ms** : feature flag pour std::time::Instant vs fallback 0
