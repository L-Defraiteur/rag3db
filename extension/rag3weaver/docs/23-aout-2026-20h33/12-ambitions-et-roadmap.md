# Doc 12 — Ambitions et feuille de route

Écrit parce que les dossiers de session antérieurs sont lointains et que
l'intention d'ensemble s'était perdue. Reconstitue **ce qui était ambitionné**
(à partir du code, des commits et des docs de mars/mai) puis **ce qui reste**.

Compagnons : [11 — état des lieux](11-etat-des-lieux-24-aout.md) ·
[13 — knowledge dump](13-knowledge-dump.md)

---

## L'ambition de fond

Faire de **rag3weaver un moteur de RAG / workflow / agentique tout terrain**,
dont le différenciateur est la **portabilité, l'embarquabilité et la
packageabilité** : un utilisateur ne doit avoir ni Python à installer, ni Docker
à lancer, ni GPU d'une marque particulière.

C'est le critère qui a tranché plusieurs arbitrages cette semaine, et qui doit
trancher les suivants. Sans lui, rag3weaver n'est « qu'un RAG de plus sur
pgvector ».

## Ce qui était ambitionné avant (reconstitué)

**Une base multi-index unifiée pour le RAG.** L'idée directrice de mars : au lieu
d'exposer chaque index par des table functions séparées, **tout se branche dans
le `WHERE` Cypher** via un mécanisme d'optimiseur générique. Trois commits en
portent l'intention :

```
b5cc6f4db  1 mars  SEARCH() in WHERE — native FTS integration
0aeb9fbea  2 mars  refactor: generalize FTS_SCAN → INDEX_SCAN
68a980050  2 mars  SPARSE_SEARCH + VECTOR_SEARCH in WHERE
```

Le contrat partagé est `src/include/common/index_search_types.h` : une fonction
scalaire pose `isIndexScanPredicate = true`, l'optimiseur bascule le scan en
`INDEX_SCAN` et injecte des expressions virtuelles (`SEARCH_SCORE()`,
`VECTOR_DISTANCE()`, `SPARSE_SCORE()`). Un seul mécanisme, trois extensions.

**Puis, à partir de mars, un pivot vers le Rust** : abstraction backend
(`DbConnection`, `SchemaDialect`, `SearchBackend`, `BlobStore`), migration
async→sync, migration vers luciole pour l'exécution parallèle du DAG. Le C++ a
cessé d'évoluer à ce moment-là.

**Ce qui restait explicitement noté au 17 mai** : FTS en Rust direct (fait
aujourd'hui), sparse segments WORM, search DAG parallèle, incremental sync pour
le WASM offline.

## Feuille de route — par ordre de valeur

### 1. Finir la migration FTS (court, débloque le reste)

- **Débrancher le repli C++.** Aujourd'hui `search_bm25_chunked` et `FlushNode`
  retombent sur `CALL *_LUCIVY_INDEX` quand aucun handle Rust n'est ouvert. Le
  garder était juste tant qu'on doutait ; maintenant que les E2E sont verts, il
  masque les cas où le handle ne s'ouvre pas (c'est exactement ce qui a caché
  trois bugs aujourd'hui). À retirer après une passe sur corpus réel.
- **Supprimer l'extension C++ `lucivy_fts`** une fois le repli parti, et le
  submodule avec.

### 2. Sortir de candle (le différenciateur en dépend)

candle n'a pas de backend ROCm : sur la machine actuelle il est CPU-only. burn
couvre AMD/NVIDIA/Apple/navigateur depuis **une seule** implémentation.

| Modèle | Route | État |
|---|---|---|
| BGE-M3 | ONNX BAAI → burn-onnx | ✅ fait, parité prouvée |
| MiniLM | `sentence-transformers/all-MiniLM-L6-v2`, `onnx/model.onnx` **86 Mo** | à faire — plus simple que BGE-M3 : sous la limite de 2 Go du protobuf, donc **burn-onnx 0.21 stable suffit** |
| BM42 | export ONNX à produire avec `output_attentions=True` | le seul point dur — PyTorch **au build**, l'artefact livré restant 100 % Rust |

**Garder candle comme outillage de test, pas comme dépendance produit** :
`examples/bge_m3_reference.rs` est l'oracle qui prouve la parité à chaque montée
de version de burn. Derrière une feature optionnelle, jamais par défaut.

**Décision prise** : MiniLM sera le défaut navigateur (90 Mo contre 2,2 Go). Mais
avec `LoadStrategy::Bytes`, pas `Embedded` — c'est JS qui fournit les octets et
les met en IDBFS, comme le fait déjà `catalog_set_embedder`.

### 3. Étendre les E2E aux vrais embedders

Les E2E utilisent déjà `CandleEmbedder` (9×) et `BgeM3Embedder` (12×) à côté de
`MockEmbedder` (63×). Manquent :

- **`BurnBgeM3Embedder` — le trou le plus risqué.** Validé en isolation, **jamais
  dans le `Catalog`**. C'est là que se joue ce que l'isolation ne teste pas : le
  `DualEmbedder` en un seul forward pendant le drain, l'alimentation du
  `SparseHandle` en vrais poids, et la **qualité de la fusion RRF** à trois
  signaux réels.
- `Bm42Embedder` : aucun usage E2E.

⚠️ Ces jobs sont lourds (`burn-embedder` tire ~700 crates, les poids font
2,2 Go). À réserver à un déclenchement manuel, pas à chaque push.

### 4. Résorber les trois doublons

| Doublon | Origine | Sortie |
|---|---|---|
| 2 moteurs de DAG | migration luciole bloquée sur l'absence de checkpoint dans luciole | exposer `NodeContext::set_input/drain_outputs` (c'est ton repo), puis supprimer `dataflow/runtime.rs`, `graph.rs`, `services.rs` |
| 2 chemins de search | `Catalog::search()` utilise les `_via_backend`, les nodes du DAG appellent les legacy avec `&conn` brut | faire pointer les nodes sur `_via_backend` → le pipeline DAG devient portable Postgres |
| repli C++ FTS | transitoire assumé | voir §1 |

Code mort à purger au passage : **`src/fusion.rs` en entier** (0 callsite hors
de ses 11 tests, mais `pub` donc sans warning), `search_vector_bruteforce`,
`resolve_chunk_results_via_backend`, `search_bm25_raw`.

### 5. Postgres : le faire tourner une fois

Il compile de nouveau mais **n'a jamais été exécuté**. Le `docker-compose.supabase.yml`
existe et n'est utilisé par rien. Un seul test d'intégration changerait son statut
de « écrit » à « fonctionne ».

À revoir au passage : `postgres_connection.rs:233` fait
`Handle::current().block_on()`, qui panique hors réacteur ou depuis un worker
tokio, et ne compile que parce que `deadpool-postgres` active `tokio/rt`
transitivement. Fragile.

### 6. Décisions en attente

**`geo`** — soit un E2E dédié (le harnais existe : `test/runner/e2e_test.cpp`
scanne un répertoire via `E2E_TEST_FILES_DIRECTORY`, `extension/vector/test/test_files/`
donne le modèle), soit le sortir de `extension_config.cmake:6` qui l'active dans
**tous** les builds natifs. Le statu quo — activé, jamais testé, deux bugs
connus — est du risque gratuit.

**Les 4 fichiers E2E en dérive** — dette de la migration luciole, jamais traitée.

**`codeparsers/`** — 22 851 LOC, tree-sitter, 14 langages, `import_resolution`,
`relationship_resolution`, `scope_extraction`. **Référencé nulle part**, ni dans
`Cargo.toml` ni dans `src/`. C'est la brique code-RAG qui dort. À câbler ou à
assumer comme projet frère.

**Topologie de stockage FTS** — `FtsStorage::BlobBacked { lazy }` est retenu.
`LocalFs` + deltas LUCIDS sera nécessaire au WASM offline (le blob-backed
rematérialise tout l'index à chaque ouverture), mais le WASM n'a jamais été
débuggé de bout en bout : c'est une cible, pas un prérequis.

**`_core_start_char` / `_core_end_char`** — écrits à l'ingestion, déclarés au
schéma, **jamais relus**. L'attribution highlight→chunk utilise le span *avec*
recouvrement, donc un highlight en zone d'overlap matche deux chunks et les deux
sortent. La zone core avait été capturée pour permettre l'attribution exclusive ;
cette moitié n'a jamais été câblée. Décision sémantique à prendre, pas un bug.

### 7. Le chantier lucivy, en parallèle

Le trou séparateurs de lucivy (chercher `->`, `};`, `foo->bar`) reste ouvert. La
piste proposée est un **sidecar byte-n-gram** (trigrammes d'octets bruts +
vérification `memmem` sur contenu mmapé, façon csearch/Zoekt), plutôt que
d'étendre le SFX. Deux classes de requêtes, deux index — c'est ce qui expliquait
le whack-a-mole des partitions v3.
