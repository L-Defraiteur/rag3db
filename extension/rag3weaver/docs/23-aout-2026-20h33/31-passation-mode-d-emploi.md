# Doc 31 — Passation : mode d'emploi (tests, outils, pièges, où regarder)

Compagnons : [29 — progression](29-passation-progression-24-aout-soir.md) ·
[30 — architecture](30-passation-architecture-et-intention.md).

## Build natif et E2E

```bash
cd extension/rag3weaver && bash run_e2e.sh --build-only --no-cuda   # cmake, long la 1ère fois
B=/home/lucied/git_workspaces/rag3db/build/native-test/src
export RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR="$B" RAG3DB_INCLUDE_DIR="$B" \
       RAG3DB_ROOT=/home/lucied/git_workspaces/rag3db LD_LIBRARY_PATH="$B"
```

**`default = []` depuis le 24 août : nommer les features est obligatoire.**

```bash
# suites sans modèle (rapides, mock embedder)
cargo test --features rag3db-native --test e2e_symbol_search -- --ignored --test-threads=1
cargo test --features rag3db-native --test e2e_idempotent_registration -- --ignored --test-threads=1
# vrais modèles = burn (poids dans ~/.cache/rag3weaver/{bge-m3,minilm}/, voir generated/README.md)
# — sans `burn-embedder`, 25 des 38 tests de e2e_search DISPARAISSENT sans erreur
cargo test --features rag3db-native,burn-embedder --test e2e_search -- --ignored --test-threads=1
cargo test --features rag3db-native,burn-embedder --test e2e_idempotent_registration -- --ignored --test-threads=1
cargo test --features rag3db-native,burn-embedder --test e2e_simple_entity -- --ignored --test-threads=1
cargo test --features rag3db-native,burn-embedder --test e2e_burn_embedder -- --ignored --test-threads=1
cargo test --features rag3db-native,burn-embedder --test e2e_burn_minilm   -- --ignored --test-threads=1
cargo test --features rag3db-native,burn-embedder --test e2e_burn_multilingual_minilm -- --ignored --test-threads=1   # dense multilingue (~/.cache/rag3weaver/multilingual-minilm/)
cargo test --features rag3db-native,burn-embedder --test e2e_burn_reranker -- --ignored --test-threads=1   # cross-encoder (~/.cache/rag3weaver/msmarco-minilm/)
cargo test --features rag3db-native,burn-embedder --test e2e_burn_xlmr_reranker -- --ignored --test-threads=1   # mmarco-mMiniLMv2 + bge-reranker-v2-m3 (~/.cache/rag3weaver/{mmarco-minilm,bge-reranker-v2-m3}/)
cargo test --features rag3db-native --test e2e_rerank -- --ignored --test-threads=1                   # crochet de rerank, reranker mock
# candle n'est PLUS une feature des E2E (depuis le 24 août au soir) : oracle de parité
# seulement, via examples/*_reference.rs et examples/burn_*_vs_candle.rs
# profil (drain par phase, moteur seul vs chaîne, N croissant)
cargo test --features rag3db-native --test e2e_profile_overhead -- --ignored --nocapture
```

`RAG3DB_SHARED=1` est indispensable (sinon `undefined symbol: IndexAuxInfo`).
Toutes les suites E2E (`tests/e2e_*.rs`) compilent et passent sur burn — passe complète : voir doc 29 (chiffre du 24 au soir).
`run_e2e.sh` passe `rag3db-native,burn-embedder` (`--no-cuda` accepté, sans effet). Les embedders partagés des E2E sont dans `tests/common/mod.rs`.

## Matrice de features — une combinaison par appel, jamais `--all-features`

L'unification cargo rend `--all-features` aveugle. Boucle de référence
(zsh : fonction avec `"$@"`, **pas** de variable non quotée) :

```bash
chk() { local l="$1"; shift; cargo check -q --lib "$@" && echo "✓ $l" || echo "✗ $l"; }
chk default; chk candle --features candle-embedder; chk bge-m3 --features bge-m3
chk candle-wasm --no-default-features --features candle-wasm
chk burn --features burn-embedder; chk postgres --features postgres
chk wasm --features wasm-emscripten; chk native --features rag3db-native
cargo check --examples --features candle-embedder   # idem bge-m3, burn-embedder
cargo test --lib; cargo test --lib --features candle-embedder; cargo test --lib --features burn-embedder
```

Le crate est en `[lints.rust] warnings = "deny"` : un import mort casse une combinaison.

## Variables d'environnement utiles

| Variable | Effet |
|---|---|
| `RAG3W_BLOB_TRACE=1` | trace des flushs du blob store (saves reçus/poussés, octets, ms) |
| `RAG3W_NO_BATCH_SAVE=1` | un `MERGE` par blob au lieu de l'`UNWIND` — change le timing, sert à isoler les courses |
| `RAG3DB_MAX_DB_SIZE=$((1<<30))` | réservation VM kuzu (défaut 8 TiB — le défaut Rust `u32::MAX` est le sentinel `-1u`) ; **requis sous valgrind** ; puissance de 2 |
| `RAG3DB_BUFFER_POOL_SIZE` | idem, buffer pool |
| `LUCIOLE_REPLY_TRACE=1` | backtrace à chaque `Reply` lâchée sans `send` (luciole) |
| `RAG3W_VEC_TRACE=1` | affiche le Cypher de projection envoyé à `QUERY_VECTOR_INDEX` (filtres vectoriels) |
| `RAG3WEAVER_BGE_M3_BPK` / `_TOKENIZER`, `RAG3WEAVER_MINILM_BPK` / `_TOKENIZER` | chemins des poids burn |

## Chasser un crash mémoire (méthode qui a marché)

1. Reproduire en boucle : `for i in 1 2 3; do cargo test … ; done` — un run ne prouve rien.
2. Pile : `gdb -q -batch -ex run -ex bt -ex "thread apply all bt 12" --args <bin> --ignored --test-threads=1`.
3. **valgrind**, pas `MALLOC_CHECK_` (glibc ne suit pas l'identité des blocs — il m'a fait écrire un faux diagnostic) :
   `RAG3DB_MAX_DB_SIZE=$((1<<30)) RAG3DB_BUFFER_POOL_SIZE=$((256<<20)) valgrind --tool=memcheck --error-limit=no --num-callers=24 --log-file=v.log <bin> --ignored --test-threads=1 phase0 phase1`
4. Pour une panique précise (ex. overflow) : `gdb -ex "rbreak panic_const_sub_overflow" -ex run -ex "bt 30"` — `break rust_panic` n'est **pas** un symbole résolu.
5. Prendre le binaire **par empreinte**, pas `ls -t` (une autre passe peut en avoir recompilé un autre).

## Relation avec lucivy — deux copies, un seul chemin qui compte

- rag3weaver compile `lucivy-core` par **path dep** : `../../../lucivy/lucivy_core` = `~/git_workspaces/lucivy`, **l'arbre de travail de la session lucivy**. Vérifier `git -C ~/git_workspaces/lucivy log -1` et `status` avant toute mesure. Ne jamais le déplacer sans leur accord.
- Le submodule `extension/lucivy/ld-lucivy` n'est qu'une **référence épinglée** (le CI clone lucivy à cette révision pour les path deps). Épingler à chaque avancée : `cd extension/lucivy/ld-lucivy && git fetch && git checkout <sha>`, puis `git add extension/lucivy/ld-lucivy`. Path deps depuis l'arbre vivant : `lucivy-core`, `luciole`, `sparse-vector` (→ `lucistore`).
- Compiler contre un autre commit lucivy via worktree **ne marche pas** (collision de lockfile `ld-lucivy` par `sparse-vector`).
- Le dialogue se fait par docs numérotés dans ce dossier (01 → 28 aujourd'hui). Leurs réponses arrivent comme fichiers ; Lucie relaie les nôtres.

## Où regarder dans le code

| Sujet | Fichier |
|---|---|
| API publique, cycle de vie, Drop | `src/catalog.rs` (`initialize`, `drain`, `flush_blob_store`, `close_fts_handles`) |
| FTS Rust (schéma, index, recherche, offsets) | `src/fts_handle.rs` |
| Requêtes BM25, modes, warnings, attribution chunks | `src/search.rs` (`build_bm25_query`, `search_bm25_chunked`, `finish_bm25_chunked`, `ChunkAttributionMiss`) |
| Blob store | `src/cypher_blob_store.rs`, `src/buffered_blob_store.rs` |
| Nœuds d'ingestion, KB, FlushNode | `src/dataflow/record_nodes.rs` (`gather_batch`, `KBUpdateNode`) |
| Embedders burn | `src/burn_bge_m3_embedder.rs`, `src/burn_minilm_embedder.rs`, `src/burn_multilingual_minilm_embedder.rs`, `generated/README.md` (provenance, empreintes, parité de chaque modèle) |
| Reranking (cross-encoder) | `src/reranker.rs` (trait, mock, `passage_text`), `src/burn_reranker.rs` (MiniLM EN), `src/burn_xlmr_reranker.rs` (mMiniLMv2 + bge-reranker-v2-m3, XLM-R : pad 1, pas de token_type_ids), crochet dans `Catalog::search` (avant la pagination), `examples/{reranker,xlmr_reranker}_reference.rs` + `burn_*_vs_candle.rs` |
| Enregistrement / migration d'entités et KB | `src/catalog.rs` (`register_entity`, `create_kb_tables`), `src/schema.rs` (`resolve_kb_title_entities`) |
| Connexion native, config kuzu | `src/rag3db_connection.rs` |
| Index sparse (WAND, 3 blobs par index) | crate `sparse-vector` dans le workspace **lucivy** (`~/git_workspaces/lucivy/sparse_vector`), ouvert dans `catalog.rs` (`SparseHandle::*_with_store`) |
| FFI WASM | `src/wasm_ffi.rs` (`catalog_set_embedder` gardé par feature) |
| CI | `.github/workflows/rag3weaver-workflow.yml` (matrice explicite) |

## Pièges payés aujourd'hui (ne pas les repayer)

- **`pgrep -f` / `pkill -f` sur un motif présent dans un autre script** : deux attentes se sont attendues mutuellement, puis un `pkill` a tué sa propre commande. Utiliser un fichier sentinelle ou `pgrep -x cargo`.
- **Le `cd` persiste entre appels de l'outil Bash** ; les chemins relatifs suivants partent du mauvais dossier. Chemins absolus.
- **`cargo test -q` supprime les lignes `test … ok`** : impossible de chronométrer par test. Sans `-q`, avec timestamps.
- **`out=$(…)` + grep vide** = souvent une erreur de compilation avalée. Capturer dans un fichier et lire `^error`.
- **Recompiler pendant qu'une passe tourne** casse la passe (cargo réécrit les binaires). Attendre ou changer de features.
- **`default = []` fait disparaître des tests en silence** si on oublie les features.
- **Un destructeur ne laisse jamais passer une panique** (abort pendant un déroulement). Idem pour `close()` de toute lib.
- **Un doc de rapport doit dire ce qu'il ne sait pas** ; les docs 19 et 25 ont affirmé des causes fausses, corrigées par erratum. Un « déterministe » vérifié sur 3 runs en était à 3/4.

## Multi-tenant (doc 37) — comment s'en servir

```rust
catalog.set_scope(Scope::new("acme", "search"))?;   // cellule courante : ingestion + recherche
catalog.ingest_entities("Doc", rows)?;              // lignes estampillées _org/_project, index de la cellule
catalog.search("Doc", "q", SearchOptions { scope: Some(Scope::new("acme", "billing")), ..Default::default() })?;
catalog.search("Doc", "q", SearchOptions { scopes: vec![a, b], ..Default::default() })?;   // fan-out, fusion par rang
```

- Sans `set_scope` : cellule `default/default`, index `Lucivy_{table}` d'avant, zéro coût.
- Chaque cellule a ses index (`Lucivy_{table}__{org}__{project}` dans `_index_blobs`) ; `set_scope` gare/reprend les handles.
- Le vecteur est post-filtré par colonnes (sur-fetch ×4) tant que le canari kuzu tient ; `meta.warnings` signale un sur-fetch épuisé.
- Ids : `[A-Za-z0-9_.-/]`, ≤ 128 ; `/` pour une hiérarchie par convention (`starts_with`).

## Contrats à connaître avant de toucher au dataflow

- **Ports partagés** : `PortValue::take()` (luciole) panique en fan-out. Dans un nœud, consommer avec `take_or_clone::<T>(pv)` (`dataflow/port.rs`) — déplacement si seul consommateur, clone sinon. Le runtime déplace la valeur vers le *dernier* consommateur et clone pour les autres.
- **Undo depuis un checkpoint sur un nœud frais** : `DeleteRecordNode`/`UpdateRecordNode::bind_services(conn, dialect)`, plus `UpdateRecordNode::bind_fts(fts_handles, node_id_cache, entity_configs)` — sinon `undo` répond « 'conn' not stored ». Accesseurs : `Catalog::{conn_arc, dialect_arc, fts_handles, sparse_handles, node_id_cache, entity_configs}`.
- **Ré-indexation FTS** : toujours relire la ligne entière avec *tous* les champs indexés de l'entité (`entity_indexed_fields` → `reindex_fts_rows`), jamais le sous-ensemble modifié.
- **Graphe générique construit hors du `Catalog`** (BM25/Vector/Sparse/Resolve nodes) : enregistrer `conn` en `ConnService`, `embedder`/`sparse_embedder`/`dual_embedder` en `Arc<dyn …>` tels quels, et **`fts_handles` + `sparse_handles`** (les nœuds cherchent dans les index Rust). Modèle : `build_services` dans `tests/e2e_generic_search.rs`.
- **Services `conn`** : deux conventions coexistent — `ConnService(Arc<dyn DbConnection>)` pour les nœuds de recherche, `Arc<dyn DbConnection>` nu pour les nœuds d'enregistrement/migration. Ne pas les confondre.
- **`luciole`** est une dépendance **par chemin** (`../../../lucivy/luciole`), comme `lucivy-core` et `sparse-vector` — plus de `[patch.crates-io]` : quand lucivy a bumpé 0.1.0 → 0.2.0, le patch ne satisfaisait plus la contrainte et cargo retombait silencieusement sur crates.io. **Une seule entrée `luciole` doit exister dans `Cargo.lock`** — à vérifier après chaque épinglage (`grep -c 'name = "luciole"' Cargo.lock`).

## Hygiène

- Commits sans mention de Claude ni trailer d'attribution (mémoire).
- `.vault/hf.env` contient `HF_ACCESS_TOKEN` (gitignoré) ; upload HF via un venv jetable `huggingface_hub` (`hf upload`), jamais afficher le token.
- Docs de session : `extension/rag3weaver/docs/<date>/NN-*.md` pour rag3weaver ; `docs/` racine pour le fork kuzu.
- `git config user.email luciedefraiteur@gmail.com` local ; routage SSH par URL de remote, pas `gh auth switch`.
