# Doc 43 — Passation : tests, outils, points critiques (24 août, nuit)

Remplace le doc 31 là où ils divergent. Compagnons :
[41 — progression](41-passation-progression-24-aout-nuit.md) ·
[42 — architecture](42-passation-architecture-24-aout-nuit.md).

## 1. Build natif et environnement

```bash
cd extension/rag3weaver && bash run_e2e.sh --build-only --no-cuda   # cmake vector;geo (long la 1ère fois)
B=/home/lucied/git_workspaces/rag3db/build/native-test/src
export RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR="$B" RAG3DB_INCLUDE_DIR="$B" \
       RAG3DB_ROOT=/home/lucied/git_workspaces/rag3db LD_LIBRARY_PATH="$B"
```

`RAG3DB_SHARED=1` est indispensable. **`default = []` : nommer les features
est obligatoire** — sans `burn-embedder`, 25 des 38 tests d'`e2e_search`
*disparaissent* sans erreur. Si l'arbre lucivy ne compile pas (ils sont en
plein travail), une copie figée par `git archive <sha>` dans le scratchpad et
un `sed` temporaire des path deps de `Cargo.toml` débloque — **à remettre
avant tout commit**.

## 2. Les suites E2E (toutes `#[ignore]`, `--test-threads=1`)

| Suite | Tests | Features | Sujet |
|---|---|---|---|
| `e2e_search` | 38 | `rag3db-native,burn-embedder` (13 sans burn) | BM25/vecteur/sparse/hybride, KB, modes BM25 |
| `e2e_scope` | 9 | `rag3db-native` | multi-tenant : stamp, index par cellule, isolation, fan-out, migration, **canari kuzu** |
| `e2e_rerank` | 3 | `rag3db-native` | crochet de rerank avec reranker mock, pool, pagination, avertissements |
| `e2e_burn_reranker` | 5 | + `burn-embedder` | ms-marco-MiniLM (Berlin, déterminisme, lots, Catalog) |
| `e2e_burn_xlmr_reranker` | 8 | + `burn-embedder` | mmarco-mMiniLMv2 + bge-reranker-v2-m3, EN/FR/croisé |
| `e2e_burn_minilm` / `e2e_burn_multilingual_minilm` / `e2e_burn_embedder` | 3 / 5 / 4 | + `burn-embedder` | embedders burn (EN, multilingue FR→EN, BGE-M3 trois signaux) |
| `e2e_burn_ocr` | 4 | `burn-ocr` (sans rag3db-native) | PP-OCRv6 tiny sur `tests/fixtures/ocr/hello.png`, `OcrNode` avec le vrai modèle |
| `openai_llm_sse` / `openai_llm_luciole` | 6 / 3 (+1 `#[ignore]`) | `openai-llm` | client SSE sur socket réelle (serveur factice local), tool calls en deltas, `Flow::Stop` qui coupe, non-fuite du secret ; règle d'interblocage luciole |
| `e2e_symbol_search` | 12 | `rag3db-native` | `BM25Mode::Symbol`, séparateurs, emoji, `parse` booléen |
| `e2e_idempotent_registration` | 22 | `rag3db-native` | enregistrement KB/entités dans tous les ordres, réouverture |
| `e2e_simple_entity` | 15 | `rag3db-native` | pipeline simple, update partiel (**régression FTS**), delete |
| `e2e_undo` | 4 | `rag3db-native` (+burn pour KB) | undo delete/update depuis checkpoint, FTS ré-indexé |
| `e2e_dataflow_observe` / `e2e_search_queue` / `e2e_generic_search` | 7 / 5 / 8 | `rag3db-native` (+burn) | observabilité, `search_with_strategy`, nœuds génériques |
| `e2e_highlight_long_text`, `e2e_result_mode`, `e2e_phase0b`, `e2e_drain_unified`, `e2e_batch_observe`, `e2e_checkpoint`, `e2e_native` | 8/10/14/6/2/3/11 | `rag3db-native` | chunks, modes de résultat, drain, checkpoints |
| `e2e_profile_overhead` | 4 | `rag3db-native` (`--nocapture`) | drain par phase, plancher de commit, drain(N) |

Passe complète (toutes les suites, ~2,5 min sur burn) :

```bash
for f in tests/e2e_*.rs; do t=$(basename $f .rs);
  cargo test --features rag3db-native,burn-embedder --test $t -- --ignored --test-threads=1 > /tmp/$t.log 2>&1;
  echo "$t $(grep '^test result' /tmp/$t.log)"; done
```

Poids attendus dans `~/.cache/rag3weaver/{minilm,multilingual-minilm,bge-m3,msmarco-minilm,mmarco-minilm,bge-reranker-v2-m3}/{model.bpk,tokenizer.json}`
et `~/.cache/rag3weaver/ppocrv6-tiny/{det.bpk,rec.bpk,dict.txt}` (`RAG3WEAVER_PPOCR_DIR`)
(ou `RAG3WEAVER_<MODEL>_BPK` / `_TOKENIZER`) — `generated/README.md` dit d'où
les télécharger (HF `Lucie666/*-burnpack` + tokenizer amont).

## 3. Matrice `--lib` et exemples — une combinaison par appel

```bash
chk() { local l="$1"; shift; cargo check -q --lib "$@" && echo "✓ $l" || echo "✗ $l"; }
chk default; chk burn --features burn-embedder; chk candle --features candle-embedder
chk bge-m3 --features bge-m3; chk candle-wasm --no-default-features --features candle-wasm
chk wasm --features wasm-emscripten; chk postgres --features postgres; chk native --features rag3db-native
chk ocr --features ocr; chk burn-ocr --features burn-ocr; chk both --features burn-embedder,burn-ocr
cargo test --lib; cargo test --lib --features burn-embedder; cargo test --lib --features wasm-emscripten wasm_ffi
cargo test --lib --no-run --features wasm-emscripten,candle-wasm    # le trou bouché ce soir
cargo check --examples --features burn-embedder; cargo check --examples --features candle-embedder
cargo check --tests --features rag3db-native,burn-embedder; cargo check --tests --features rag3db-native
```

`[lints.rust] warnings = "deny"` : un import mort casse une combinaison ; les
imports utilisés seulement par des tests gatés vont sous le même `cfg`.

**Piège de méthode, payé le 25 :** pour compter *nos* diagnostics dans le bruit
de lucivy, `grep -c "rag3weaver/src/"` **ne trouve jamais rien** — cargo affiche
les chemins du paquet local en **relatif** (`--> src/foo.rs`) et ne met en
absolu que les dépendances par chemin. Une vérification écrite comme ça rend
« 0 » même avec de vraies erreurs. Le bon filtre :

```bash
cargo check ... 2>&1 | grep -cE '^\s+--> (src|tests|examples)/'
```

## 4. Parité burn / candle (à rejouer après toute régénération)

```bash
cargo run --example minilm_reference --features candle-embedder -- /tmp/ref.json
cargo run --example burn_minilm_vs_candle --features burn-embedder -- /tmp/ref.json          # cosine ≥ 0.9999
cargo run --example reranker_reference --features candle-embedder -- /tmp/rr.json
cargo run --example burn_reranker_vs_candle --features burn-embedder -- /tmp/rr.json          # |Δ| logit < 1e-3
cargo run --example xlmr_reranker_reference --features candle-embedder -- /tmp/x.json mmarco  # ou bge
cargo run --example burn_xlmr_reranker_vs_candle --features burn-embedder -- /tmp/x.json mmarco
cargo run --example multilingual_minilm_reference --features candle-embedder -- /tmp/pml.json
cargo run --example burn_multilingual_minilm_vs_candle --features burn-embedder -- /tmp/pml.json
```

Publication HF : venv jetable `huggingface_hub` (`scratchpad/hfvenv/bin/hf upload`),
jeton dans `.vault/hf.env` (`HF_ACCESS_TOKEN`, gitignoré, **jamais affiché**),
puis `curl … | sha256sum` du fichier servi avant d'écrire l'empreinte dans la
fiche et dans `generated/README.md`.

## 5. Variables d'environnement

| Variable | Effet |
|---|---|
| `RAG3W_BLOB_TRACE=1` | trace des flushs du blob store |
| `RAG3W_NO_BATCH_SAVE=1` | un `MERGE` par blob (change le timing — outil de course) |
| `RAG3W_VEC_TRACE=1` | Cypher de projection envoyé à `QUERY_VECTOR_INDEX` |
| `RAG3DB_MAX_DB_SIZE=$((1<<30))`, `RAG3DB_BUFFER_POOL_SIZE` | réservation VM kuzu (8 TiB par défaut) ; **requis sous valgrind** |
| `LUCIOLE_REPLY_TRACE=1` | backtrace à chaque `Reply` lâchée |
| `RAG3WEAVER_*_BPK` / `_TOKENIZER` | chemins des poids burn |

## 6. Points critiques — où regarder, et ce qu'il ne faut pas casser

| Sujet | Fichier / fonction | Invariant |
|---|---|---|
| Écriture des lignes | `record_nodes.rs` `InsertRecordNode::execute`, `KBUpdateNode` | seul point d'écriture ; stamp `_org`/`_project` ici |
| Ré-indexation FTS | `record_nodes.rs` `reindex_fts_rows`, `entity_indexed_fields` | relire **tous** les champs indexés (`add_document` n'est pas un merge) |
| Undo | `DeleteRecordNode`/`UpdateRecordNode::{bind_services, bind_fts, undo}` | un nœud frais doit être lié avant `undo` |
| Ports partagés | `dataflow/port.rs` `take_or_clone` | jamais `PortValue::take()` sur un port à plusieurs consommateurs |
| Handles FTS/sparse | `catalog.rs` `ensure_fts_handle`, `ensure_sparse_handle`, `set_scope`, `close_fts_handles` | nom d'index = `scope.index_name(...)` ; fermer aussi les cellules garées |
| Flush du blob store | `catalog.rs` `flush_blob_store` | frontières : drain, ingest, reindex, shutdown, Drop |
| Recherche | `catalog.rs` `search` → fusion → **rerank** → pagination → enrichissement | rerank avant `truncate` ; `search_warnings` porte tout ce qui a été dégradé |
| Vecteur filtré | `catalog.rs` `scope_post_filter` ; `rag3db_search_backend.rs` `vector_search_filtered` | le graphe projeté **n'est pas respecté** par kuzu (canari) |
| Migration de schéma | `catalog.rs` `migrate_scope_columns` (`schema_version` = 2) | tolère « already has property » (base neuve avec KB en config) |
| FFI | `wasm_ffi.rs` `err_json` / `error_only_json` / `parse_search_options` | jamais de JSON interpolé à la main |
| Lucivy | `Cargo.toml` : `lucivy-core`, `ld-lucivy`, `luciole`, `sparse-vector` **par chemin** | `grep -c 'name = "luciole"' Cargo.lock` = 1 |

## 7. Pièges payés (ne pas les repayer)

- Deux agents qui éditent `lib.rs` / `tests/common` / `Cargo.toml` en même temps se marchent dessus : **séquencer**, commiter entre deux.
- Un agent qui « attend une notification » ne reprend pas : lui demander d'attendre **de façon synchrone** (`until … ; do sleep 5; done`).
- Le shell ne découpe pas `$var` en mots (`set -- $pair` échoue) ; le `cd` persiste entre appels ; `grep -v token` avale les vraies erreurs — filtrer sur la valeur du secret, pas sur le mot.
- Recompiler pendant qu'une passe tourne casse la passe.
- `BM25Mode::Contains` cherche la chaîne entière : `ContainsSplit` pour du multi-mots.
- Un test de régression se prouve par **contre-épreuve** (il doit échouer sans le correctif) ; un « déterministe » se mesure sur ≥ 3 runs.
- Erreurs C++ de kuzu : « already has property », « not found », « does not exist » — les messages qu'on tolère dans la migration.
- Docs : les numéros collisionnent avec la session lucivy (24, 29, 35, 36, 40 ce soir) — renuméroter le leur, le dire dans le commit.

## 8. Hygiène

Commits sans mention d'IA ni trailer d'attribution ; `user.email` local
`luciedefraiteur@gmail.com`, routage SSH par URL de remote ; `.gitmodules` jamais
commité ; docs de session dans `extension/rag3weaver/docs/<date>/NN-*.md` ;
repérage par agents Explore plutôt que lecture par tranches (mémoire).
