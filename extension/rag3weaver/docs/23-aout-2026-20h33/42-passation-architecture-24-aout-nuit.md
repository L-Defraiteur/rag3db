# Doc 42 — Passation : architecture et intention, telles que je les connais (24 août, nuit)

Remplace le doc 30 sur les points qui ont bougé. Compagnons :
[41 — progression](41-passation-progression-24-aout-nuit.md) ·
[43 — tests et points critiques](43-passation-tests-et-points-critiques.md) ·
[36 — vision](36-vision-agents-comme-graphe-et-workflow.md) ·
[37 — multi-tenant](37-conception-org-project-multi-tenant.md).

## L'intention

**rag3weaver = un moteur RAG / workflow / agentique tout terrain, embarquable,
packageable.** Le critère qui tranche depuis février : *zéro Docker, zéro
Python, embarqué* — c'est lui qui a choisi burn (wgpu/Vulkan, un code pour
AMD/NVIDIA/Apple/navigateur) contre candle (oracle de parité seulement), le
fork kuzu contre Postgres, et qui a fait retirer BM42. Horizon (doc 36) :
l'agent comme **sous-graphe qui se compile en workflow**, ses sessions et
traces dans la même base, des agents qui construisent des agents par RAG sur
les agents existants ; le RAG embarqué comme *substrat* distribué où un onglet
de navigateur est un nœud (shards lucistore).

Cible immédiate : un agent de code exposé en MCP, sur base embarquée (le
serveur MCP est le seul propriétaire de la base — kuzu verrouille au niveau
processus).

## La pile

```
rag3db  (fork Kuzu v0.11.2.2, LRSL v1.2, seuil 100 k€/an)
├── extension/vector     HNSW C++ — vivant ; QUERY_VECTOR_INDEX ignore les graphes projetés (canari)
├── extension/geo        R-tree — orphelin, jamais testé E2E
└── extension/rag3weaver ~45 k lignes Rust — le produit (plus aucune extension C++ à Rust)

lucivy  (~/git_workspaces/lucivy, arbre vivant, par chemin — VOULU, c'est l'usage naïf
         de rag3weaver qui valide la v3 non publiée ; ne jamais rediriger vers le submodule)
├── lucivy_core      ShardedHandle (4 shards), SFX v3, contains/parse/boolean, filtres natifs (string fast fields), search_filtered routé par shard, node_ids_of
├── luciole          scheduler d'acteurs (Pool, Reply, DAG) — AUSSI le moteur du search DAG de rag3weaver (une seule copie !)
├── lucistore        BlobStore, shard_storage, delta, snapshot, sync_server — la persistance commune
└── sparse_vector    index sparse WAND original (MIT chez eux), SparseHandle + ShardedSparseHandle, sur lucistore
```

## rag3weaver, par couches

**Catalog** (`catalog.rs`, ~5 300 lignes) — la seule surface publique voulue :
`register_entity` / `register_kb` / `create` / `ingest_entities` / `update` /
`delete` / `drain` / `search` / `reindex` / `shutdown`, `set_embedder` /
`set_sparse_embedder` / `set_dual_embedder` / **`set_reranker`** /
**`set_scope`**. Deux modes d'entité : **KB** (`{KB}_Index` + `_Index_Chunk`,
agrégation cross-entités par `KBUpdateNode`) et **simple** (`Entity` +
`Entity_Chunk`). `Drop` ferme les handles FTS (cellule courante + garées) et
flushe le blob store sous `catch_unwind` — jamais de panique qui s'échappe.

**Multi-tenant** (`scope.rs`, doc 37) — `Scope { org, project }`, deux axes
orthogonaux. Chaque ligne porte `_org`/`_project` ; chaque cellule a ses index
FTS (`Lucivy_{table}__{org}__{project}`) et sparse ; `set_scope` gare les
handles de la cellule quittée (`parked_fts`/`parked_sparse`) et reprend ceux
de la nouvelle ; `SearchOptions.scope` (autre cellule) et `scopes` (fan-out +
fusion par rang). Vecteur : HNSW par table + **post-filtre par colonnes**
(sur-fetch ×4) tant que kuzu ignore la projection. Mono-tenant = cellule
`default/default`, index `Lucivy_{table}` d'avant, zéro coût.

**Dataflow** (`dataflow/`) — DAG typé, 26 nœuds, checkpoints avec undo,
observabilité (taps, `ExecutionReport`, enregistreur JSONL). **Deux runtimes** :
`DataflowRuntime` (ingestion, `search_with_strategy` via `execute_via_luciole`)
et luciole. Contrats : `PortValue::take()` panique en fan-out → nos nœuds
consomment avec `take_or_clone` ; un nœud frais rejoue un `undo` après
`bind_services(conn, dialect)` (+ `bind_fts` pour `UpdateRecordNode`) ; deux
conventions de service `conn` (`ConnService` pour la recherche,
`Arc<dyn DbConnection>` nu pour l'ingestion). Un seul point d'écriture de
lignes : `InsertRecordNode` (+ `KBUpdateNode` pour les lignes d'index) — c'est
là que le stamp de scope se fait.

**Recherche** (`search.rs`) — trois signaux (BM25 via lucivy, vecteur via
kuzu HNSW, sparse via SparseHandle) fusionnés par RRF, **puis rerank optionnel
du pool avant la pagination** (`SearchOptions.rerank`, `RerankOptions {
candidates: 50 }`), puis enrichissement. `meta.warnings` toujours peuplé et
honnête (rerank sans reranker, sur-fetch épuisé, fan-out, attribution de
chunks). Cinq modes BM25 : `Contains` (défaut — **une seule chaîne**, une
requête multi-mots veut `ContainsSplit`), `ContainsSplit`, `Regex`, `Parse`
(opt-in humain), `Symbol` (exact, séparateurs inclus). Invariant : highlights ↔
chunks par recouvrement d'intervalles d'**octets** ; BM25 indexe le document
entier ; `_content` = `join("\n")` sans terminateur. `FilterCondition` →
`allowed_ids` pour BM25 **et sparse** (branché ce soir), `WHERE` pour le
vecteur.

**Backends** — `DbConnection`, `SchemaDialect` (rag3db + Postgres, jamais
exécuté), `SearchBackend` (vecteur + résolution). `CypherBlobStore`
(`_index_blobs`, MERGE/UNWIND) derrière `BufferedBlobStore` (write-back,
flush aux frontières : drain, **ingest**, reindex, shutdown, Drop). Les index
vivent **dans la base** : un fichier à sauvegarder ; le cache mmap est jetable.

**Modèles burn** (`generated/README.md` = registre : provenance, empreintes,
parité) — code généré par burn-onnx 0.22.0-pre.1 commité dans `generated/`,
poids en burnpack sur HF `Lucie666/*-burnpack`, jamais dans git :

| rôle | modèle | struct | taille |
|---|---|---|---|
| dense EN / navigateur | all-MiniLM-L6-v2 | `BurnMiniLmEmbedder` | 90 Mo |
| dense multilingue | paraphrase-multilingual-MiniLM-L12-v2 | `BurnMultilingualMiniLmEmbedder` | 470 Mo |
| dense + sparse appris | BGE-M3 | `BurnBgeM3Embedder` | 2,2 Go |
| reranker EN | ms-marco-MiniLM-L-6-v2 | `BurnMiniLmReranker` | 90 Mo |
| reranker multilingue (défaut) | mmarco-mMiniLMv2-L12-H384-v1 | `BurnMMiniLmReranker` | 470 Mo |
| reranker qualité | bge-reranker-v2-m3 | `BurnBgeRerankerV2M3` | 2,2 Go |

Recette figée : ONNX fp32 → build.rs jetable `ModelGen … LoadStrategy::Bytes` →
`generated/<m>_onnx.rs` (en-tête scrubbé) + `.bpk` → exemple `*_reference.rs`
(candle) + `burn_*_vs_candle.rs` (seuil) → E2E → fiche HF avec empreinte
vérifiée *après* téléchargement. Pièges : le vrai `Model::forward` est le
dernier ; `LinearLayout::Col` impose burn ≥ 0.22.0-pre.2 ; `.bpk` non
reproductible octet à octet ; XLM-R = pad **1**, pas de `token_type_ids`,
paire `<s> q </s></s> p </s>` ; le MiniLM multilingue = corps BERT à
vocabulaire XLM-R, `token_type_ids` à zéro, troncature 128 héritée.

**FFI** (`wasm_ffi.rs`, C ABI) — `create`, `search_async`, `drain*`, `count`,
`set_embedder`, `set_scope`/`get_scope` ; options JSON par
`parse_search_options` (camelCase, lit aussi `scope`, `scopes`,
`filterCondition`, `filters`) ; **toutes** les réponses par `serde_json`.
Liaisons emscripten dans `tools/wasm/src_cpp/weaver_bindings.cpp`.

## Ce qui dort et qui compte

`codeparsers/` (24 555 lignes, tree-sitter, 10 langages, résolution d'imports
et de relations — la brique qui produit les arêtes que le graphe sait
parcourir), l'éval (on ne mesure pas la pertinence), les documents réels
(OCR d'abord en nœud unitaire), le streaming (ports en flux), le YAML
universel (= sérialisation de l'agent-sous-graphe).
