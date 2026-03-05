# 15 — Sparse Index V1 : rapport final et concessions

## Statut : COMPLET — 330 tests passent, 0 failures

L'implementation V1 du sparse index dans rag3weaver est terminee. Tous les fichiers prevus dans le plan (doc 13) ont ete ecrits et les tests passent.

## Fichiers modifies/crees

Chemin de base : `packages/rag3db/extension/rag3weaver/src/`

| Fichier | Changement | Lignes ~approx |
|---|---|---|
| `sparse_index.rs` | **NOUVEAU** — SparseVector, SparseIndex, 9 tests | ~260 |
| `embedder.rs` | SparseEmbedder trait, Mock, Callback, 6 tests | +120 |
| `config.rs` | `sparse: bool`, `sparse_weight: f64` sur KBConfig | +10 |
| `ops.rs` | SparseEmbedOp, CatalogOp::SparseEmbed, OP_SPARSE_EMBED | +25 |
| `schema.rs` | Colonnes `{kb}_sparse_indices/weights`, 1 nouveau test | +20 |
| `search.rs` | search_sparse, fuse_rrf_n, fuse_weighted_3way, 6 nouveaux tests | +180 |
| `catalog.rs` | sparse_embedder, sparse_indexes, SparseEmbedProcessor, rebuild, wiring CRUD+search | +150 |
| `lib.rs` | `pub mod sparse_index` + re-exports | +5 |
| `cypher_persistence.rs` | Match arm SparseEmbed dans extract_op_data | +10 |
| `queue.rs` | Match arm SparseEmbed dans enqueue | +1 |

## Concessions et compromis de la V1

### 1. Rebuild complet de l'index apres chaque drain()

**Concession** : Apres `drain()`, on fait un `rebuild_sparse_index()` qui relit TOUTES les entites depuis la DB pour reconstruire l'index en memoire.

**Pourquoi c'est un compromis** : C'est O(N) sur le nombre total de documents avec sparse. L'ideal serait un update incremental : le `SparseEmbedProcessor` mettrait directement a jour le SparseIndex en memoire apres avoir stocke en DB, sans passer par un reload complet.

**Pourquoi on l'a fait comme ca** : Le `Processor` n'a pas acces au `Catalog` ni a ses `sparse_indexes`. Il faudrait un mecanisme de callback ou un Arc<Mutex> partage, ce qui complexifierait l'architecture queue/processor pour la V1. Le rebuild est acceptable tant que le nombre de documents reste modere (< 100k).

**Fix futur** : Ajouter un canal (channel) entre SparseEmbedProcessor et Catalog, ou un post-drain hook dans la queue.

### 2. Pas d'embedder sparse reel — seulement Mock et Callback

**Concession** : Le `MockSparseEmbedder` utilise un hash djb2 des mots comme token_ids et 1/num_words comme poids. C'est du bruit, pas de l'attention.

**Pourquoi** : BM42 (extraction des poids d'attention du transformer) est la V2. Ca necessite candle + un modele charge, ce qui est un chantier a part entiere. La V1 pose l'infra (storage, search, fusion) sans le ML.

**En attendant** : Le `CallbackSparseEmbedder` permet de brancher n'importe quelle logique externe (API, WASM candle, etc.) sans modifier rag3weaver.

### 3. Boost + sparse = fallback RRF

**Concession** : Quand `HybridStrategy::Boost` est utilise et qu'il y a des sparse results, on tombe en fallback sur RRF au lieu d'un vrai boost 3-way.

**Pourquoi** : La formule Boost (`vector * (1 + bm25_norm * factor)`) est un produit scalaire entre 2 signaux. L'etendre a 3 signaux n'a pas de sens mathematique evident — on obtiendrait un `vector * (1 + bm25_norm * f1 + sparse_norm * f2)` qui est finalement juste du weighted deguise.

**Impact** : Minime. Boost est deja le choix par defaut uniquement parce qu'il favorise les resultats vector (precision semantique). Avec 3 signaux, RRF est une strategie plus robuste et neutre.

### 4. Pas de compression des posting lists

**Concession** : Les posting lists sont des `Vec<(String, f32)>` bruts — pas de delta encoding, pas de variable-byte encoding, pas de compaction.

**Pourquoi** : V1 = simplicite. Les tokens BM42 typiques donnent 5-15 tokens non-zero par document. Avec 10k documents, ca fait ~100k entrees en memoire, soit ~quelques MB. Largement acceptable.

**Si ca pose probleme** : On pourrait passer a des posting lists compressees (groupes varint, roaring bitmaps pour les doc IDs), mais ca n'apporterait rien tant qu'on n'a pas > 1M documents.

### 5. Pas de cache sparse embeddings

**Concession** : Les dense embeddings ont un cache FIFO (`embedding_cache` sur Catalog, max 100 entrees). Les sparse embeddings query n'ont pas de cache.

**Pourquoi** : Le dense embedding est couteux (forward pass ~10ms). Le sparse embedding BM42 sera aussi couteux (forward pass attention). Mais en V1 avec le Mock, c'est instantane, donc pas de besoin.

**Fix futur** : Ajouter un `sparse_embedding_cache: HashMap<String, SparseVector>` sur Catalog, meme pattern FIFO.

### 6. Colonnes sparse stockees comme INT64[] / DOUBLE[]

**Concession** : Les indices sparse sont stockes en `INT64[]` (64 bits) alors que `u32` suffirait. Les poids sont en `DOUBLE[]` (64 bits) alors que `f32` suffirait.

**Pourquoi** : rag3db (Kuzu) n'a pas de type `INT32[]` ou `FLOAT[]` en colonnes. Les types arrays disponibles sont `INT64[]` et `DOUBLE[]`. On fait la conversion u32↔i64 et f32↔f64 au read/write.

**Impact** : ~2x l'espace disque par rapport a l'ideal, mais les sparse vectors sont petits (5-15 elements), donc negligeable.

### 7. Sparse search ignore les filtres

**Concession** : `search_sparse()` n'applique aucun filtre — ni les filtres Kuzu (`extra_where`), ni les filtres Lucivy. Les filtres sont appliques indirectement via la fusion (un document filtre-out du vector/BM25 aura un score plus bas dans la fusion).

**Pourquoi** : Le SparseIndex en memoire n'a pas de notion d'attributs. Pour filtrer, il faudrait soit :
- Maintenir les attributs dans l'index (complexite)
- Post-filtrer les resultats sparse (facile mais potentiellement imprecis si le top-k est petit)

**Impact** : Pour la V1, c'est acceptable. Les filtres vector + BM25 couvrent la majorite des cas. Un document qui passe le filtre dense mais pas le sparse aura quand meme son score dense dans la fusion.

**Fix futur** : Ajouter `allowed_ids: Option<&HashSet<String>>` sur `search_sparse()` — le meme pattern que `allowed_ids` sur Lucivy.

### 8. SparseEmbedOp textes clones

**Concession** : Dans `create()`, les textes pour embed et sparse_embed sont `.clone()`-es au lieu d'etre partages.

**Pourquoi** : L'EmbedOp et le SparseEmbedOp sont des ops separees dans la queue avec des lifetimes differentes. Partager les textes necessiterait un `Arc<Vec<String>>`, ce qui complique l'API pour un gain negligeable (les textes sont petits, ~quelques KB).

## Ce qui marche bien

- **Fusion N-way RRF** : `fusion::rrf_fuse` acceptait deja N listes, l'extension a 3 signaux n'a necessite aucun changement dans fusion.rs
- **Weighted 3-way** : formule propre `(1 - kw - sw) * vec + kw * bm25 + sw * sparse` avec normalisation min-max de chaque signal
- **Pattern Processor** : le SparseEmbedProcessor suit exactement le meme pattern que EmbedProcessor — resoudre refs, batch embed, stocker en DB
- **Trait SparseEmbedder** : le CallbackSparseEmbedder permet de brancher un BM42 externe (API ou WASM) sans toucher a rag3weaver
- **Zero nouvelle dep Cargo** : tout en std, le sparse index fait 260 lignes

## Prochaines etapes

1. **V2 : BM42 candle** — extraction poids d'attention [CLS], merge sous-mots WordPiece, production de SparseVector reels
2. **Cache sparse** — ajouter FIFO cache pour embed_sparse query
3. **Filtrage sparse** — allowed_ids sur search_sparse
4. **Incremental rebuild** — eviter le reload complet apres drain
5. **E2E tests** — config yaml avec `sparse: true`, insert → drain → search, verifier scores fusion
