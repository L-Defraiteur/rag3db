# 12 — Extension Sparse Vector : findings et orientation

## Contexte

Suite au doc 11 (reflexion sparse vectors) et a la preuve que candle tourne en WASM (test Playwright passe, all-MiniLM-L6-v2 384d, 10s total), on decide de creer une extension dediee pour les sparse vectors plutot que de passer par Lucivy.

## Pourquoi pas Lucivy ?

Lucivy fait du full-text search avec sa propre tokenisation (stemming, stop words, lowercase, etc.). Les sparse vectors de type BM42 travaillent sur des tokens **WordPiece** bruts issus du transformer — ce sont des sous-mots comme `["hel", "##lo", "wor", "##ld"]`, pas des mots stemmes.

Mettre des tokens WordPiece dans Lucivy poserait des problemes :
- Lucivy applique ses analyzers (stemming, stopwords) → corrompt les tokens
- Les poids d'attention BM42 sont des floats arbitraires, pas du TF-IDF
- L'index inverse de Lucivy est optimise pour BM25 (term frequency + doc frequency), pas pour un dot product sparse generique
- On melangerait deux responsabilites differentes dans le meme index

## Pourquoi une nouvelle extension (pas dans vector) ?

L'extension `vector` existante fait du **HNSW** (Hierarchical Navigable Small World) — un algorithme d'approximate nearest neighbor pour vecteurs denses. Le sparse c'est fondamentalement different :

| | Dense (HNSW) | Sparse |
|---|---|---|
| Dimension | Fixe (384 ou 768) | Variable (~30k vocab, ~100-300 non-zero) |
| Stockage | Array dense `FLOAT[N]` | Paires `(token_id, weight)` |
| Structure | Graph multi-couches (CSR) | Index inverse (posting lists) |
| Search | Approximate NN (greedy traversal) | Exact dot product (posting list scan) |
| Complexite | O(log N) approx | O(Q * avg_posting_len) exact |

Fusionner les deux dans une seule extension ajouterait de la complexite sans benefice reel — la fusion des scores se fait deja dans rag3weaver (RRF / Weighted).

## Architecture de l'extension vector existante (reference)

```
extension/vector/src/
├── main/vector_extension.cpp      — Entry point, registration des fonctions
├── catalog/
│   └── hnsw_index_catalog_entry.cpp — Persistence metadata dans le catalog
├── function/
│   ├── create_hnsw_index.cpp       — CREATE_VECTOR_INDEX (StandaloneTableFunc)
│   ├── query_hnsw_index.cpp        — QUERY_VECTOR_INDEX (TableFunc, RETURN requis)
│   └── drop_hnsw_index.cpp         — DROP_VECTOR_INDEX (StandaloneTableFunc)
└── index/
    ├── hnsw_index.cpp              — InMemHNSWIndex / OnDiskHNSWIndex
    ├── hnsw_graph.cpp              — Layers upper/lower, CSR format
    └── hnsw_config.cpp             — Config parsing (mu, ml, metric, etc.)
```

**Patterns a reutiliser :**
- Registration via `addTableFunc` / `addStandaloneTableFunc`
- Catalog entry (AuxInfo serialise) pour persistence metadata
- Le cycle de vie : CREATE (in-memory build) → checkpoint (persist) → load (from disk) → QUERY
- Le pattern bind/init/exec des TableFunc
- IndexStorageInfo pour stocker les IDs des rel tables

**Patterns a remplacer :**
- HNSW graph → index inverse (posting lists)
- Dense `FLOAT[N]` → sparse `(u32, f32)[]`
- Cosine/L2 metric → dot product sparse
- Two-layer graph → flat inverted index

## Architecture proposee : extension sparse_vector

```
extension/sparse_vector/src/
├── main/sparse_vector_extension.cpp   — Entry point, registration
├── catalog/
│   └── sparse_index_catalog_entry.cpp — Catalog persistence
├── function/
│   ├── create_sparse_index.cpp        — CREATE_SPARSE_INDEX
│   ├── query_sparse_index.cpp         — QUERY_SPARSE_INDEX
│   └── drop_sparse_index.cpp          — DROP_SPARSE_INDEX
└── index/
    ├── sparse_index.cpp               — InMemSparseIndex / OnDiskSparseIndex
    ├── inverted_index.cpp             — Posting lists (token_id → doc_id + weight)
    └── sparse_config.cpp              — Config parsing
```

### SQL API

```sql
-- Creer un index sparse sur une colonne qui contient des sparse vectors
-- Format colonne : STRUCT(indices INT32[], values FLOAT[]) ou serialise JSON
CALL CREATE_SPARSE_INDEX('Document', 'doc_sparse_idx', 'sparse_embedding');

-- Rechercher : passer un sparse vector query, retourne top-k
CALL QUERY_SPARSE_INDEX('Document', 'doc_sparse_idx',
    {indices: [1234, 5678, 9012], values: [0.8, 0.3, 0.1]}, 10)
RETURN node, score;

-- Supprimer
CALL DROP_SPARSE_INDEX('Document', 'doc_sparse_idx');
```

### Structure de donnees : index inverse a poids

L'index inverse est le coeur. Pour chaque `token_id` (dimension non-zero), on stocke une **posting list** triee par `doc_id` :

```
token_id=1234 → [(doc_0, 0.82), (doc_3, 0.45), (doc_7, 0.91), ...]
token_id=5678 → [(doc_1, 0.33), (doc_3, 0.67), ...]
token_id=9012 → [(doc_0, 0.12), (doc_5, 0.55), ...]
```

**Search (dot product) :**
1. Pour chaque `(token_id, q_weight)` dans la query sparse vector
2. Lookup la posting list de `token_id`
3. Pour chaque `(doc_id, d_weight)` dans la posting list
4. Accumuler `scores[doc_id] += q_weight * d_weight`
5. Retourner top-k par score decroissant

C'est **exact** (pas approximatif), et rapide grace a la sparsity (~100-300 non-zero sur ~30k dims).

### Stockage sur disque

Deux options :

**Option 1 : Rel tables (comme vector)**
- Une rel table `_<tableID>_<indexName>_POSTINGS`
- Chaque edge : `token_id → doc_id` avec propriete `weight: FLOAT`
- Avantage : reutilise l'infra rag3db, transactions, checkpoints
- Inconvenient : overhead d'une rel table pour des posting lists

**Option 2 : Fichier binaire dedie**
- Fichier custom dans `parent_path(db)/sparse_indexes/<table>/<index>/`
- Format : header + token_id offsets + posting lists contiguës
- Avantage : compact, lecture sequentielle rapide, mmap possible
- Inconvenient : gestion lifecycle custom (pas de transactions rag3db)

**Recommandation :** Option 1 d'abord (rel tables) pour rester coherent avec l'extension vector. On peut optimiser vers un fichier binaire plus tard si les perfs le demandent.

### Integration avec le pipeline rag3weaver

```
Ingestion:
  texte → candle forward pass → dense (384d) + sparse BM42 (attention weights)
  dense → extension vector (HNSW)
  sparse → extension sparse_vector (inverted index)

Search:
  query → candle → dense_query (384d) + sparse_query (attention weights)
  QUERY_VECTOR_INDEX(dense_query, k=20)  → dense_scores
  QUERY_SPARSE_INDEX(sparse_query, k=20) → sparse_scores
  BM25 Lucivy(text_query, k=20)         → bm25_scores
  RRF fusion(dense, sparse, bm25)        → final_ranked_results
```

Fusion 3-way avec RRF : `score(d) = Σ 1/(k + rank_i(d))` pour i ∈ {dense, sparse, bm25}.

## Candle en WASM : preuve de concept FAIT

**Test Playwright reussi** (22 fevrier 2026) :
- all-MiniLM-L6-v2 charge depuis HuggingFace CDN (config.json + tokenizer.json + model.safetensors ~90MB)
- Forward pass via candle en wasm32-unknown-emscripten
- Resultat : dim=384, nonZero=384, norm=1.0, 10.4s total
- Feature `candle-wasm` dans Cargo.toml (candle sans hf-hub)
- `CandleEmbedder::from_bytes()` pour chargement in-memory (pas de filesystem)

Cela prouve que la generation BM42 (extraction attention weights) fonctionnera aussi en WASM — c'est le meme forward pass, on extrait juste des tenseurs differents.

## Etapes d'implementation

### V1 : Extension sparse_vector (stockage + search)
**Effort : ~500-800 lignes C++**

1. Copier le squelette de l'extension vector
2. Remplacer HNSW par un index inverse
3. API : CREATE_SPARSE_INDEX, QUERY_SPARSE_INDEX, DROP_SPARSE_INDEX
4. Format colonne : STRUCT(indices INT32[], values FLOAT[])
5. Tests E2E GTest avec sparse vectors manuels
6. Support DELETE/UPDATE via hooks (comme lucivy_fts)

### V2 : Generation BM42 dans candle_embedder
**Effort : ~200 lignes Rust**

1. Modifier le forward pass BERT pour extraire les attention weights du [CLS]
2. Merger les sous-mots WordPiece (somme des poids par mot)
3. Normaliser les poids (softmax ou L1)
4. Nouveau trait `SparseEmbedder` ou extension du trait `Embedder`
5. Un seul forward pass → dense Vec<f32> + sparse Vec<(u32, f32)>
6. Tests unitaires comparant avec valeurs de reference

### V3 : Fusion 3-way dans rag3weaver
**Effort : ~50 lignes Rust**

1. Etendre `search.rs` pour appeler QUERY_SPARSE_INDEX
2. Etendre `fusion.rs` pour supporter 3+ listes dans RRF
3. Config : poids par signal (dense_weight, sparse_weight, bm25_weight)
4. Tests d'integration

## Questions ouvertes

1. **Format de colonne sparse** : `STRUCT(indices INT32[], values FLOAT[])` ou `MAP(INT32, FLOAT)` ou `STRING` (JSON) ? Le STRUCT est le plus type-safe.

2. **IDF** : BM42 utilise `score = Σ IDF(qi) * attention(qi)`. L'IDF est calculable depuis les posting lists (nombre de docs contenant le token / nombre total de docs). Le stocker dans l'index ou le calculer au query time ?

3. **Vocabulaire** : les token_ids WordPiece vont de 0 a ~30k. Comment les indexer efficacement ? HashMap<u32, Vec<(u64, f32)>> en memoire, serialise en posting lists sur disque.

4. **Incremental** : comment gerer INSERT/DELETE incrementalement ? L'index inverse est naturellement incrementable (ajouter/retirer des entries dans les posting lists).

## Fichiers de reference

| Composant | Fichier |
|---|---|
| Extension vector (modele) | `extension/vector/src/` |
| Extension lucivy_fts (modele hooks) | `extension/lucivy_fts/src/` |
| CandleEmbedder + from_bytes() | `extension/rag3weaver/src/candle_embedder.rs` |
| Trait Embedder | `extension/rag3weaver/src/embedder.rs` |
| Fusion RRF existante | `extension/rag3weaver/src/fusion.rs` |
| Search pipeline | `extension/rag3weaver/src/search.rs` |
| Test Playwright candle WASM | `tools/wasm/test/browser/candle_embed.spec.js` |
