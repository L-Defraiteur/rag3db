# Progression - 6 Fevrier 2026

> Suite de `00-etat-des-lieux.md`. Documente les tests de compilation effectues cette session.

---

## Contexte

On a identifie que l'approche correcte est de compiler Tantivy en **static lib** pour la target `wasm32-unknown-emscripten` (pas `wasm32-unknown-unknown`), afin de beneficier du support pthreads via SharedArrayBuffer et de pouvoir linker avec Kuzu dans un seul module WASM.

Cette session valide la faisabilite technique de cette approche.

---

## Test 1 : Summa via wasm-pack (echec attendu)

**Target :** `wasm32-unknown-unknown` (wasm-pack)
**Resultat :** Echec

Summa compile bien en WASM via `wasm-pack build --target web` (6.7MB), mais crash au runtime avec :

```
Failed to spawn segment updater thread
```

Cause : `wasm32-unknown-unknown` n'a pas `std::thread::spawn()`. Meme avec `WriterThreads::N(1)`, Tantivy tente un `thread::spawn` interne. La variante `WriterThreads::SameThread` pourrait contourner ca, mais ca ne resout pas le probleme de fond (d'autres parties de Tantivy utilisent aussi des threads).

**Conclusion :** wasm-pack seul n'est pas viable pour Tantivy avec ecriture d'index.

---

## Test 2 : fuzzy-fst pour Emscripten (succes)

**Target :** `wasm32-unknown-emscripten`
**Emplacement :** `packages/rag3db/third_party/fuzzy-fst/`

```bash
source emsdk_env.sh
cd packages/rag3db/third_party/fuzzy-fst
EMCC_CFLAGS="-pthread" cargo rustc --target wasm32-unknown-emscripten --release --crate-type staticlib
```

Note : on utilise `cargo rustc --crate-type staticlib` au lieu de `cargo build` car le Cargo.toml de fuzzy-fst definit aussi `cdylib` et `rlib`, ce qui fait echouer le link Emscripten (`undefined symbol: main`). En forcant `staticlib` seul, le probleme disparait.

**Resultat :** `libfuzzy_fst.a` (3.3MB) - archive ar compatible Emscripten.

---

## Test 3 : izihawa-tantivy pour Emscripten (succes)

**Target :** `wasm32-unknown-emscripten`
**Crate de test :** `/tmp/tantivy-emscripten-test/`

### Cargo.toml

```toml
[package]
name = "tantivy-emscripten-test"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib"]

[dependencies]
izihawa-tantivy = {
    path = ".../packages/rag3db/extension/tantivy/izihawa-tantivy",
    default-features = false,
    features = ["stopwords", "lz4-compression", "stemmer"]
}

[profile.release]
opt-level = "z"
lto = "thin"
```

### src/lib.rs

```rust
use izihawa_tantivy::schema::{Schema, TEXT};
use izihawa_tantivy::Index;

#[no_mangle]
pub extern "C" fn tantivy_test_create_index() -> i32 {
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("body", TEXT);
    let schema = schema_builder.build();
    let _index = Index::create_in_ram(schema);
    0
}
```

### Commande de build

```bash
source emsdk_env.sh
EMCC_CFLAGS="-pthread" cargo build --target wasm32-unknown-emscripten --release
```

### Probleme rencontre : tokio + feature mmap

Avec les features par defaut, la compilation echoue :

```
error: Only features sync,macros,io-util,rt,time are supported on wasm.
```

Cause : la feature `mmap` (active par defaut) depend de `tokio` avec la feature `fs`, non supportee sur WASM. La feature `mmap` active :

- `memmap2` : memory-mapped files (acces index sur disque)
- `fs4` : file locking
- `tempfile` : fichiers temporaires pendant merge
- `tokio` avec `fs` + `io-util`

**Solution :** `default-features = false` + activer seulement les features necessaires. On n'a pas besoin de `mmap` car on utilise des index en RAM (`Index::create_in_ram`). Dans l'architecture finale, c'est Kuzu qui gere le storage sur disque.

Features conservees :
- `stopwords` : filtrage mots vides
- `lz4-compression` : compression segments
- `stemmer` : stemming (running -> run)

### Resultat

```
Finished `release` profile [optimized] target(s) in 2m 38s
```

**`libtantivy_emscripten_test.a` (4.8MB)** - archive ar compatible Emscripten.

---

## Bilan tests de compilation

| Composant | Target | Resultat | Taille .a |
|-----------|--------|----------|-----------|
| fuzzy-fst | `wasm32-unknown-emscripten` | Succes | 3.3 MB |
| izihawa-tantivy (test minimal) | `wasm32-unknown-emscripten` | Succes | 4.8 MB |
| Summa (wasm-pack) | `wasm32-unknown-unknown` | Echec (threads) | N/A |

La Phase 1 de la roadmap (verification compilation) est **validee**. Les deux libs Rust compilent en static libs compatibles Emscripten.

---

## Crate tantivy-fts : API C FFI complete (succes)

**Emplacement :** `packages/rag3db/extension/tantivy_fts/rust/`

Apres avoir valide la compilation, on a ecrit le crate complet qui expose l'API C FFI pour Tantivy. Ce crate est concu comme une extension rag3db, pas comme un composant specifique a WASM.

### Structure du crate

```
extension/tantivy_fts/rust/
├── Cargo.toml
├── cbindgen.toml
└── src/
    ├── lib.rs          ← 13 fonctions extern "C" (API publique)
    ├── directory.rs    ← StdFsDirectory (impl Directory via std::fs)
    ├── handle.rs       ← TantivyHandle (index + writer + reader)
    └── query.rs        ← Parsing JSON → Query Tantivy, execution search
```

### API C exposee (13 fonctions)

```c
// Lifecycle
TantivyHandle* tantivy_create_index(const char* path, const char* schema_json);
TantivyHandle* tantivy_open_index(const char* path);
void            tantivy_close_index(TantivyHandle* handle);

// Ecriture (incrementale, segment-based)
int64_t  tantivy_add_document(TantivyHandle* handle, const char* doc_json);
int64_t  tantivy_delete_by_term(TantivyHandle* handle, const char* field, const char* value);
int64_t  tantivy_commit(TantivyHandle* handle);
void     tantivy_rollback(TantivyHandle* handle);

// Lecture
char*    tantivy_search(TantivyHandle* handle, const char* query_json, uint32_t limit);
char*    tantivy_search_filtered(TantivyHandle* handle, const char* query_json,
                                  uint32_t limit, const uint64_t* allowed_ids, uint32_t num_ids);
void     tantivy_reload_reader(TantivyHandle* handle);
void     tantivy_free_string(char* ptr);

// Info
char*    tantivy_get_schema(TantivyHandle* handle);
uint64_t tantivy_num_docs(TantivyHandle* handle);
```

### Champ `_node_id` auto-ajoute

Chaque schema Tantivy inclut automatiquement un champ `_node_id: u64` avec les flags `FAST | INDEXED`. Ce champ permet :

1. **Mapping Kuzu → Tantivy** : chaque document Tantivy porte l'ID du noeud Kuzu correspondant
2. **Filtered search** : `tantivy_search_filtered()` prend un tableau d'IDs autorises et utilise `FilterCollector` pour ne scorer que les documents dont le `_node_id` est dans le set
3. **Delete par node ID** : suppression ciblee via `tantivy_delete_by_term("_node_id", id)`

Flow de recherche filtree :
```
Cypher: MATCH (n:Document) WHERE n.year > 2020 RETURN n._node_id
    → [42, 87, 103, 256]
    → tantivy_search_filtered(handle, query, limit, [42,87,103,256], 4)
    → FTS uniquement sur ces 4 documents
```

### StdFsDirectory : agnostique de la plateforme

Au lieu d'utiliser `RamDirectory` (RAM seule) ou `MmapDirectory` (necessite la feature `mmap` + tokio), on a implemente un `StdFsDirectory` qui utilise `std::fs` :

- **Natif** : `std::fs` → vrai filesystem, fichiers segments sur disque
- **Emscripten** : `std::fs` → VFS Emscripten (MEMFS), persistable via IDBFS

Le Directory suit le meme pattern que `RamDirectory` dans izihawa-tantivy : `FileSlice` comme `FileHandle`, `FsWriter` avec buffer en memoire + flush vers fichier.

### Types de queries supportes

Le `query_json` accepte :
- `"type": "term"` : recherche exacte par terme → champ **raw** (precision)
- `"type": "fuzzy"` : recherche fuzzy (Levenshtein, distance configurable) → champ **raw**
- `"type": "phrase"` : recherche de phrase (auto-tokenized) → champ **stemmed** (recall)
- `"type": "regex"` : recherche par expression reguliere → champ **raw**
- `"type": "boolean"` : combinaison must/should/must_not
- `"type": "parse"` : query parser natif Tantivy → champ **stemmed** (recall)

### Stemming et architecture dual-field

Configurable par langue dans le schema JSON (`"stemmer": "english"`). Langues supportees : english, french, german, spanish, italian, portuguese, dutch, russian.

Quand un stemmer est actif, chaque champ "text" genere **deux champs Tantivy** :
- `{name}` : tokenizer "stemmed" (SimpleTokenizer + LowerCaser + Stemmer) — pour les queries orientees recall
- `{name}._raw` : tokenizer "default" (SimpleTokenizer + LowerCaser) — pour les queries orientees precision

Le routing est **transparent** : l'utilisateur reference toujours le nom de base (ex: "body"). Le crate Rust redirige automatiquement vers le bon champ selon le type de query.

| Type de query | Champ utilise | Raison |
|---------------|---------------|--------|
| `term` | `._raw` | Match exact sur la forme originale (lowercased) |
| `fuzzy` | `._raw` | Distance de Levenshtein sur la forme originale |
| `regex` | `._raw` | Pattern sur la forme originale |
| `phrase` | stemmed | Auto-tokenize les termes → match malgre les flexions |
| `parse` | stemmed | Query parser utilise le pipeline du tokenizer |
| `boolean` | depend des sous-clauses | Chaque sous-clause est routee independamment |

**Exemple concret** (stemmer english) :
- `term "programming"` → match exact sur "programming" dans `body._raw` ✓
- `term "programs"` → **pas de match** avec "programming" (mots differents en raw) ✗
- `parse "programs"` → match via stemming ("programs" → "program", "programming" → "program") ✓
- `fuzzy "programing" d=1` → match "programming" (1 lettre de difference) sur `body._raw` ✓
- `regex "program.*"` → match "programming" sur `body._raw` ✓

### Persistence de la configuration

Le schema et les options de stemming sont persistes dans `_config.json` a cote des fichiers segments de l'index. Cela permet a `tantivy_open_index()` de re-enregistrer le bon tokenizer et de reconstruire les `raw_field_pairs` sans information externe.

### Compilation

| Target | Resultat | Taille .a | Temps |
|--------|----------|-----------|-------|
| **Natif** (x86_64-linux) | Succes | 26 MB | 13s |
| **Emscripten** (wasm32) | Succes | 17 MB | 40s |

Les deux compilent sans erreur.

### Adaptations API izihawa-tantivy

Quelques differences par rapport a l'API Tantivy standard :
- `Index::create()` prend un 3e argument `IndexSettings` (pas dans Tantivy upstream)
- `TopDocs` n'implemente pas `Collector` directement → il faut `.order_by_score()`
- `TantivyDocument::parse_json(schema, json)` au lieu de `Schema::parse_document(json)`
- `WatchCallbackList` n'implemente pas `Debug` → impl manuelle pour `StdFsDirectory`
- `FilterCollector::new()` prend le nom du champ en `String`, un predicat `Fn(u64) -> bool`, et un collector interieur
- Phrase query avec stemmer : resolu par `tokenize_for_field()` qui passe les termes dans le pipeline du tokenizer avant construction de la query

---

## Header C genere (cbindgen)

**Emplacement :** `extension/tantivy_fts/include/tantivy_fts.h`

Genere via `cbindgen --config cbindgen.toml --crate tantivy-fts --output ../include/tantivy_fts.h`.

La constante `TANTIVY_NODE_ID_FIELD` ("_node_id") est ajoutee manuellement (cbindgen ne supporte pas les `&str` Rust).

Le header contient les 13 fonctions avec leurs doc comments et les types opaques (`TantivyHandle`, `TantivyHandlePtr`).

---

## Test natif C : 63/63

**Emplacement :** `extension/tantivy_fts/test/test_ffi.c`

Test complet du cycle FFI via un programme C linke avec la static lib :

```bash
cargo build --release --manifest-path=../rust/Cargo.toml
cc -o test_ffi test_ffi.c -I../include -L../rust/target/release -ltantivy_fts -lpthread -lm -ldl
./test_ffi
```

### Tests couverts

| Categorie | Tests | Details |
|-----------|-------|---------|
| Lifecycle | 3 | create, close, reopen (open) |
| Schema | 6 | get_schema, verification _node_id/title/body + title._raw/body._raw |
| Ecriture | 8 | 5x add_document, commit, delete_by_term, commit apres delete |
| Term search | 3 | recherche "rust" dans body |
| Fuzzy search | 2 | "rast" distance=1 → trouve "rust" |
| Phrase search | 3 | "lazy dog" auto-tokenized (stemmer transparent) |
| Regex search | 2 | `.*program.*` dans title |
| Parse query | 2 | "graph database" multi-champs |
| Boolean query | 3 | must=rust + must_not=search |
| Filtered search | 6 | miss (IDs sans match), hit (IDs avec match), single node |
| Dual-field stemming | 9 | term exact vs parse stemmed, fuzzy raw, regex raw, phrase auto-stemmed |
| Delete | 4 | delete + commit + num_docs + verification absence |
| Reopen | 3 | close → open → num_docs + search |
| Error handling | 5 | null handle/path/schema |
| Cleanup | 4 | close + rm index dir + rm reopen dir + final |
| **Total** | **63** | |

### Tests dual-field (section ajoutee)

La section "dual-field stemming" valide le routing automatique :

- `term "programs"` sur body → **0 resultat** (exact match, "programs" ≠ "programming" en raw)
- `term "programming"` sur body → **1 resultat** (exact match en raw)
- `parse "programs"` sur body → **1+ resultats** (stemming : "programs" → "program" ≈ "programming")
- `fuzzy "programing" d=1` sur body → **1 resultat** (distance 1 de "programming" en raw)
- `regex "program.*"` sur body → **1+ resultats** (pattern sur raw)
- `phrase ["jumped", "over"]` sur body → **1 resultat** (auto-stemmed, trouve "The quick brown fox jumped over the lazy dog")
- `parse "lazy dogs"` sur body → **1+ resultats** (stemming : "lazy" → "lazi", "dogs" → "dog")

---

## Prochaines etapes

- [x] Valider la compilation Emscripten (fuzzy-fst + izihawa-tantivy)
- [x] Definir l'architecture (extension rag3db, pas kuzu-wasm-specific)
- [x] Ecrire le crate Rust FFI avec API C
- [x] Compiler natif + Emscripten
- [x] Ajouter `_node_id` (u64 FAST) + `tantivy_search_filtered()` (13e fonction FFI)
- [x] **Generer le header C** via cbindgen (`include/tantivy_fts.h`)
- [x] **Test natif complet** : 63/63 tests (cycle create → add → commit → search → filtered → delete → reopen → dual-field)
- [x] **Architecture dual-field** : champs stemmes + `._raw` (lowercase only) avec routing transparent
- [x] **Persistence config** : `_config.json` pour re-registration tokenizer au reopen
- [x] **CMakeLists.txt (Phase 2)** : linker la static lib Rust dans le build rag3db
  - Renommage `tantivy-fts/` → `tantivy_fts/` (convention CMake)
  - Stub C++ `TantivyFtsExtension::load()` qui verifie les symboles FFI
  - Build natif OK : `libtantivy_fts.kuzu_extension` (15 KB)
  - Build WASM OK : `libkuzu.a` (36 MB) avec extensions json+vector+algo+tantivy_fts
  - Extension FTS originale droppee du build (bug `DOC_FREQUENCY_PROP_NAME`)
- [ ] **API publique C++ (Phase 2 suite)** : 4 modes Cypher (parse, fuzzy, regex, exact) au-dessus des 6 types FFI internes
- [ ] **Extension C++** : wrapper TantivyIndex + fonctions Cypher
- [ ] **Tests** : integration avec Kuzu, puis avec Rag3Weaver

---

## Integration CMake (Phase 2 — build)

### Renommage

`extension/tantivy-fts/` → `extension/tantivy_fts/` (convention CMake : underscores partout, identifiant = nom de repertoire).

### Fichiers crees/modifies

| Fichier | Action |
|---------|--------|
| `extension/extension_config.cmake` | Ajoute `tantivy_fts` a EXTENSION_LIST + `add_static_link_extension(tantivy_fts)` dans blocs WASM/Android/Swift |
| `extension/CMakeLists.txt` | Ajoute `add_extension_if_enabled("tantivy_fts")` |
| `extension/tantivy_fts/CMakeLists.txt` | Nouveau — cargo build via `add_custom_command`, link static lib Rust, `build_extension_lib` |
| `extension/tantivy_fts/src/main/CMakeLists.txt` | Nouveau — OBJECT library pour le C++ |
| `extension/tantivy_fts/src/include/main/tantivy_fts_extension.h` | Nouveau — header stub |
| `extension/tantivy_fts/src/main/tantivy_fts_extension.cpp` | Nouveau — stub `load()` qui verifie le link FFI |

### Build natif

```bash
cd packages/rag3db && mkdir -p build && cd build
cmake .. -DBUILD_EXTENSIONS="tantivy_fts"
make -j$(nproc)
```

**Resultat :** `libtantivy_fts.kuzu_extension` (15 KB, extension dynamique). Link OK, symboles FFI resolus.

### Build Emscripten (WASM)

```bash
source .../emsdk/emsdk_env.sh
cd packages/rag3db/build-wasm
emcmake cmake .. -DBUILD_EXTENSIONS="json;vector;algo;tantivy_fts" -DBUILD_WASM=FALSE
source .../emsdk/emsdk_env.sh && emmake make -j$(nproc)
```

**Resultat :** `libkuzu.a` (36 MB) avec 4 extensions linkees statiquement. Zero erreur de link.

**Note importante :** L'extension FTS originale est **exclue** du build WASM a cause d'un bug pre-existant (`use of undeclared identifier 'DOC_FREQUENCY_PROP_NAME'` dans `query_fts_index.cpp:325`). Elle est remplacee par tantivy_fts.

### Extensions WASM cibles

| Extension | Role | Status |
|-----------|------|--------|
| **json** | Import/export JSON | OK |
| **vector** | HNSW index (embeddings) | OK |
| **algo** | Algorithmes de graphe | OK |
| **tantivy_fts** | FTS fuzzy/regex/stemming | OK |
| ~~fts~~ | ~~BM25 exact-match~~ | Exclu (bug compile) |

---

## Toolchain utilisee

- Rust : 1.88.0
- Target natif : x86_64-unknown-linux-gnu
- Target WASM : `wasm32-unknown-emscripten` (via `rustup target add`)
- Emscripten : emsdk installe dans `kuzu-wasm-exp/emsdk/`
- Flags WASM : `EMCC_CFLAGS="-pthread"`
- izihawa-tantivy : v0.26.0 (fork izihawa, `default-features = false`)
