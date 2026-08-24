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
# vrais modèles candle (oracle) — sinon 25 des 38 tests DISPARAISSENT sans erreur
cargo test --features rag3db-native,candle-embedder,bge-m3 --test e2e_search -- --ignored --test-threads=1
# burn (poids dans ~/.cache/rag3weaver/{bge-m3,minilm}/, voir generated/README.md)
cargo test --features rag3db-native,burn-embedder --test e2e_burn_embedder -- --ignored --test-threads=1
cargo test --features rag3db-native,burn-embedder --test e2e_burn_minilm   -- --ignored --test-threads=1
# profil (drain par phase, moteur seul vs chaîne, N croissant)
cargo test --features rag3db-native --test e2e_profile_overhead -- --ignored --nocapture
```

`RAG3DB_SHARED=1` est indispensable (sinon `undefined symbol: IndexAuxInfo`).
Les 12 suites vertes (hors `simple_entity` 12/13 obsolète) font ~90 s.
`run_e2e.sh` passe déjà `rag3db-native,candle-embedder,bge-m3[,cuda]`.

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
- Le submodule `extension/lucivy/ld-lucivy` ne sert qu'au build C++ (`lucivy_fts`, code mort). Épingler : `cd extension/lucivy/ld-lucivy && git fetch && git checkout <sha>`, puis `git add extension/lucivy/ld-lucivy`.
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
| Embedders burn | `src/burn_bge_m3_embedder.rs`, `src/burn_minilm_embedder.rs`, `generated/README.md` |
| Enregistrement / migration d'entités et KB | `src/catalog.rs` (`register_entity`, `create_kb_tables`), `src/schema.rs` (`resolve_kb_title_entities`) |
| Connexion native, config kuzu | `src/rag3db_connection.rs` |
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

## Hygiène

- Commits sans mention de Claude ni trailer d'attribution (mémoire).
- `.vault/hf.env` contient `HF_ACCESS_TOKEN` (gitignoré) ; upload HF via un venv jetable `huggingface_hub` (`hf upload`), jamais afficher le token.
- Docs de session : `extension/rag3weaver/docs/<date>/NN-*.md` pour rag3weaver ; `docs/` racine pour le fork kuzu.
- `git config user.email luciedefraiteur@gmail.com` local ; routage SSH par URL de remote, pas `gh auth switch`.
