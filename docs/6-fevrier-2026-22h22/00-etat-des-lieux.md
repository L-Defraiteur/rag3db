# Etat des Lieux - 6 Fevrier 2026

> **Objectif final :** Remplacer Neo4j par Kuzu (embedded) + Lucivy (FTS fuzzy/regex) dans un seul module WASM, pour alimenter le framework Rag3Weaver puis ragforge-core et community-docs.

---

## Ce qu'on a

### 1. rag3db (Fork Kuzu v0.11.2.2)

**Emplacement :** `packages/rag3db/`
**Repo :** https://github.com/L-Defraiteur/rag3db
**License :** MIT (Kuzu upstream) + nos modifs Source Available

Un fork de Kuzu avec :
- Le code source C++ complet (graph DB, Cypher, storage, extensions)
- L'extension FTS native (BM25, stemming Snowball, tokenization)
- L'extension Vector (HNSW index)
- Build WASM via Emscripten avec support **pthreads** (`-s USE_PTHREADS`, `SharedArrayBuffer`)

### 2. fuzzy-fst (Lib Rust standalone)

**Emplacement :** `packages/rag3db/third_party/fuzzy-fst/` (submodule)
**Repo :** https://github.com/L-Defraiteur/fuzzy-fst
**License :** MIT

Lib Rust complete pour fuzzy string matching :
- FST (fst-rs) + Levenshtein automaton (levenshtein-automata)
- Exports : Rust natif, C FFI (`fuzzy_fst.h`), WASM (wasm-bindgen)
- Benchmarks, tests, serialization
- **Status : code complet, pas encore integre dans rag3db FTS**

### 3. Lucivy / Summa (moteur FTS complet)

**Emplacement :** `packages/rag3db/extension/lucivy/`

Trois versions clonees :
- `izihawa-lucivy/` : Fork Lucivy v0.26.0 avec support WASM
- `lucivy-latest/` : Version officielle (reference)
- `summa/` : Wrapper izihawa qui ajoute protobuf API, memory indexes, query parsing

**Ce qu'on a verifie :**
- Summa compile en WASM via `wasm-pack` (6.7MB)
- FuzzyQuery ajoute au proto et au query parser
- Schema parsing JSON fonctionne
- **Bloqueur en mode wasm-pack :** `std::thread::spawn()` crash car `wasm32-unknown-unknown` n'a pas de threads

### 4. Rag3Weaver (Framework RAG TypeScript)

**Emplacement :** `packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/src/`

Framework complet au-dessus de Kuzu WASM :
- L1 : QueryBuilder, SchemaBuilder (prepared statements)
- L2 : Chunker, DocumentStore, UUIDGenerator
- L3 : FilterParser, SemanticChunker, Ref, EventEmitter
- Catalog : CRUD, Search (hybrid vector+BM25, fusion RRF), Schema
- Queue : OperationQueue (priority PERSIST > INSERT > LINK > EMBED)

**Status : fonctionnel, mais search sans fuzzy**

### 5. Kuzu WASM (bindings existants)

**Emplacement :** `packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/`

Build Emscripten de Kuzu avec :
- `emsdk/` installe localement
- CMakeLists.txt avec `-s USE_PTHREADS`
- Tests browser (l1-test, l2-test, l3-test) avec verification SharedArrayBuffer
- Headers COOP/COEP documentes pour le deploiement

---

## Le probleme resolu aujourd'hui

**Question :** Comment integrer Lucivy (Rust) dans un environnement WASM qui supporte les threads ?

| Approche | Target WASM | Threads | Modules | Verdict |
|----------|------------|---------|---------|---------|
| wasm-pack seul | `wasm32-unknown-unknown` | Pas de `std::thread` | 2 WASM separes | **Echec** - crash au spawn |
| C FFI + Emscripten | `wasm32-unknown-emscripten` | pthreads via SharedArrayBuffer | 1 WASM unifie | **La bonne approche** |

La solution : compiler Lucivy/Summa en **static lib C** (via `cbindgen` / C FFI), puis **linker avec Kuzu** dans le meme build Emscripten. Resultat : un seul `.wasm` avec graph DB + FTS fuzzy + vector search.

---

## Architecture cible

```
                        Build time (Emscripten)
                        =======================

  ┌──────────────────────────────────────────────────────────┐
  │                                                          │
  │  Lucivy/Summa (Rust)                                    │
  │  - Full-text search (BM25)                               │
  │  - Fuzzy search (Levenshtein automaton)                  │
  │  - Regex search                                          │
  │  - Phrase queries                                        │
  │                                                          │
  │  Compile: cargo build --target wasm32-unknown-emscripten │
  │  Output: liblucivy_fts.a (static lib C)                 │
  │                                                          │
  └───────────────────────────┬──────────────────────────────┘
                              │ C FFI link
                              ▼
  ┌──────────────────────────────────────────────────────────┐
  │                                                          │
  │  rag3db (Kuzu fork, C++)                                 │
  │  - Graph database (Cypher)                               │
  │  - Storage engine (columnar, CSR)                        │
  │  - Vector index (HNSW)                                   │
  │  - Extension FTS → appelle Lucivy via C FFI             │
  │                                                          │
  │  Compile: emcc + CMake                                   │
  │  Output: rag3db.wasm + rag3db.js + worker.js             │
  │                                                          │
  └───────────────────────────┬──────────────────────────────┘
                              │
                              ▼
                        Runtime (Browser/Node)
                        =====================

  ┌──────────────────────────────────────────────────────────┐
  │                                                          │
  │  Rag3Weaver (TypeScript)                                 │
  │  - L1-L3 abstractions                                    │
  │  - Catalog (CRUD, Search, Schema, Queue)                 │
  │  - Hybrid search: Vector + BM25 + Fuzzy via Cypher       │
  │                                                          │
  └───────────────────────────┬──────────────────────────────┘
                              │
                              ▼
  ┌──────────────────────────────────────────────────────────┐
  │                                                          │
  │  ragforge-core → community-docs                          │
  │  - Ingestion, search, entity extraction                  │
  │  - Remplace Neo4j (zero Docker, embedded)                │
  │                                                          │
  └──────────────────────────────────────────────────────────┘
```

---

## Avantages de cette approche

1. **Un seul module WASM** - pas de communication JS entre deux modules
2. **Threads supportes** - Emscripten gere pthreads via SharedArrayBuffer
3. **FTS superieur** - Lucivy offre fuzzy, regex, phrase queries (vs BM25 exact-match de Kuzu natif)
4. **Controle total** - on possede le fork, on peut iterer librement
5. **Embedded** - zero Docker, zero serveur externe, tourne dans le browser
6. **Filtrage graph→FTS** - pre-filtrage par node IDs Kuzu avant FTS, via `FilterCollector` et champ `_node_id` fast

---

## Ce qui reste a faire

### Phase 1 : Lucivy C FFI (Rust → static lib) — FAIT

- [x] Definir l'API C minimale pour FTS (13 fonctions extern "C", dont filtered search)
- [x] Compiler Lucivy avec target `wasm32-unknown-emscripten` (17 MB)
- [x] Compiler en natif (26 MB)
- [x] Implementer `StdFsDirectory` (agnostique plateforme)
- [x] Verifier que la static lib `.a` est compatible Emscripten
- [x] Generer le header C via cbindgen (`include/lucivy_fts.h`)
- [x] Test natif du cycle complet : 63/63 tests (create → add → commit → search → filtered → delete → reopen → dual-field stemming)
- [x] Architecture dual-field : champs stemmes + `._raw` pour routing transparent des queries

> **Crate :** `packages/rag3db/extension/lucivy_fts/rust/`
> **Details :** voir `01-progression.md` et `02-architecture-storage-vfs.md`

### Phase 2 : Integration CMake + Build — FAIT

- [x] Renommer `extension/lucivy-fts/` → `extension/lucivy_fts/` (convention CMake underscore)
- [x] Creer `LucivyFtsExtension::load()` (stub minimal, meme pattern que extension/fts/)
- [x] CMakeLists.txt qui link la static lib Rust (cargo build via add_custom_command)
- [x] Ajouter lucivy_fts dans extension_config.cmake + extension/CMakeLists.txt
- [x] Build natif OK (`liblucivy_fts.kuzu_extension` produit)
- [x] Build Emscripten OK (libkuzu.a WASM 36 MB avec json+vector+algo+lucivy_fts)
- [ ] Wrapper C++ `LucivyIndex` qui appelle la FFI Rust
- [ ] Fonctions Cypher : `CREATE_LUCIVY_INDEX`, `DROP_LUCIVY_INDEX`, `QUERY_LUCIVY_INDEX`
- [ ] Serialisation catalog entry (metadata dans Kuzu, segments sur filesystem)

> **Note :** L'extension FTS originale est droppee du build WASM (bug `DOC_FREQUENCY_PROP_NAME`). Extensions WASM cibles : json, vector, algo, lucivy_fts.

### Phase 3 : Tests

- [ ] Tests unitaires (Rust + C++ + Cypher)
- [ ] Mesurer taille WASM et performance

### Phase 4 : Integration Rag3Weaver

- [ ] Adapter CatalogSearch pour utiliser QUERY_LUCIVY_INDEX
- [ ] Ajouter fuzzy/regex dans les options de search
- [ ] Tests end-to-end

---

## Questions resolues

| Question | Reponse |
|----------|---------|
| On expose tout Summa ou juste le minimum ? | Minimum : 13 fonctions C (dont `lucivy_search_filtered`), pas de Summa. On utilise izihawa-lucivy directement. |
| Gestion memoire Rust/C++ ? | Handles opaques (`LucivyHandle*`), lifetime geree par create/close. Strings liberees par `lucivy_free_string`. |
| Index storage : meme directory que Kuzu ou separe ? | Sous-repertoire `{db_path}/lucivy/{table_id}_{index_name}/`. Meme filesystem, dossier separe. |
| Taille WASM ? | Static lib Lucivy FFI = 17 MB (.a). Taille finale WASM a mesurer apres link avec Kuzu. |
| Merge policy sans tokio ? | Oui, la merge policy est du Rust pur. Tokio n'etait que pour le I/O fichier async (feature `mmap`). |
| Fonctionne en natif ET WASM ? | Oui. `StdFsDirectory` utilise `std::fs` qui fonctionne partout (vrai FS en natif, VFS Emscripten en WASM). |
| Filtrage graph + FTS ? | Oui. Champ `_node_id` (u64 FAST) auto-ajoute au schema. `lucivy_search_filtered()` prend un tableau d'IDs autorises et utilise `FilterCollector` pour ne scorer que ces documents. Flow : Cypher WHERE → node IDs → FTS filtre. |
| Stemming + exact match ? | Architecture dual-field : chaque champ "text" genere `{name}` (stemmed) + `{name}._raw` (lowercase only). Routing transparent : `term/fuzzy/regex` → raw (precision), `phrase/parse` → stemmed (recall). L'utilisateur reference toujours le nom de base. |
| API publique pour le C++ ? | 4 modes exposes en Cypher : `parse` (defaut, stemmed, recall), `fuzzy` (typo tolerance, raw), `regex` (pattern, raw), `exact` (reroute vers regex `.*{term}.*`, raw). Les 6 types internes FFI (term, fuzzy, phrase, regex, boolean, parse) restent disponibles. |

---

## Fichiers de reference

| Sujet | Fichier |
|-------|---------|
| **Crate FFI Lucivy** | `packages/rag3db/extension/lucivy-fts/rust/` |
| **Header C genere** | `packages/rag3db/extension/lucivy-fts/include/lucivy_fts.h` |
| **Test natif C** | `packages/rag3db/extension/lucivy-fts/test/test_ffi.c` |
| Extension FTS existante | `packages/rag3db/extension/fts/` |
| Build WASM Kuzu | `kuzu-wasm-exp/CMakeLists.txt` |
| Prerequis COOP/COEP | `kuzu-wasm-exp/docs/guide/prerequisite.md` |
| fuzzy-fst lib | `packages/rag3db/third_party/fuzzy-fst/` |
| izihawa-lucivy fork | `packages/rag3db/extension/lucivy/izihawa-lucivy/` |
| Rag3Weaver search | `kuzu-wasm-exp/src/lib/catalog/modules/CatalogSearch.ts` |
| Session precedente | `kuzu-wasm-exp/docs/2026-02-01-08h38/` |
| Architecture storage | `docs/6-fevrier-2026-22h22/02-architecture-storage-vfs.md` |
