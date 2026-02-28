# Prochaines Etapes - Integration tantivy-fts dans rag3db

> Suite de `02-architecture-storage-vfs.md`. Plan d'integration de la static lib Rust dans le build CMake de rag3db, puis extension C++ et tests.

---

## Etat actuel (fin Phase 1)

- Crate `tantivy-fts` : 13 fonctions C FFI, 63/63 tests natifs
- Architecture dual-field (stemmed + `._raw`) avec routing transparent
- Compilation validee : natif (26 MB) et Emscripten (17 MB)
- Header C genere (`include/tantivy_fts.h`)
- API publique definie : 4 modes Cypher (parse, fuzzy, regex, exact)

---

## Etape 1 : Linker tantivy-fts.a dans CMake rag3db (natif) — FAIT

**Objectif :** Valider que la static lib Rust link correctement dans le build CMake existant de rag3db, sans undefined symbols.

### Ce qui a ete fait

1. Renommage `extension/tantivy-fts/` → `extension/tantivy_fts/` (convention CMake : underscores)

2. Fichiers crees :
   - `extension/tantivy_fts/CMakeLists.txt` : `add_custom_command` pour `cargo build --release`, declare `libtantivy_fts.a` comme IMPORTED, link `-lpthread -lm -ldl`
   - `extension/tantivy_fts/src/main/CMakeLists.txt` : OBJECT library
   - `extension/tantivy_fts/src/include/main/tantivy_fts_extension.h` : header extension
   - `extension/tantivy_fts/src/main/tantivy_fts_extension.cpp` : stub `load()` qui verifie les symboles FFI

3. Fichiers modifies :
   - `extension/extension_config.cmake` : ajout `tantivy_fts` dans EXTENSION_LIST + blocs static link
   - `extension/CMakeLists.txt` : ajout `add_extension_if_enabled("tantivy_fts")`

4. Build natif :
   ```bash
   cd packages/rag3db && mkdir -p build && cd build
   cmake .. -DBUILD_EXTENSIONS="tantivy_fts"
   make -j$(nproc)
   ```

### Resultat

- `make` passe sans erreur de link (exit code 0)
- `libtantivy_fts.kuzu_extension` produit (15 KB, extension dynamique)
- Symboles FFI Rust resolus correctement

---

## Etape 2 : Build Emscripten (kuzu-wasm) — FAIT

**Objectif :** Meme chose mais pour la target `wasm32-unknown-emscripten`. Verifier que le `.a` Emscripten link dans le build WASM de rag3db.

### Ce qui a ete fait

Le CMakeLists.txt de tantivy_fts detecte deja Emscripten via `if(EMSCRIPTEN)` et ajuste la target cargo et les env vars.

Build WASM :
```bash
source .../emsdk/emsdk_env.sh
cd packages/rag3db/build-wasm
emcmake cmake .. -DBUILD_EXTENSIONS="json;vector;algo;tantivy_fts" -DBUILD_WASM=FALSE
source .../emsdk/emsdk_env.sh && emmake make -j$(nproc)
```

**Note :** On passe `-DBUILD_WASM=FALSE` car `BUILD_WASM=TRUE` force le static linking de TOUTES les extensions (via `extension_config.cmake`), y compris FTS qui a un bug de compilation (`DOC_FREQUENCY_PROP_NAME` undeclared). En passant FALSE + BUILD_EXTENSIONS explicite, on controle exactement quelles extensions sont incluses.

### Resultat

- `emmake make` passe sans erreur (exit code 0)
- `libkuzu.a` produit : **36 MB** avec 4 extensions linkees statiquement
- Extensions incluses : json, vector, algo, tantivy_fts
- Extension FTS originale exclue (remplacee par tantivy_fts)
- Aucun conflit pthreads ni symbole manquant

### Risques identifies et resolus

| Risque initial | Resultat |
|----------------|----------|
| Conflits pthreads | Aucun conflit — la lib Rust et Emscripten cohabitent |
| Symbols manquants libc | Aucun symbole manquant |
| Extension FTS bug compile | Contourne en excluant FTS du build |
| Taille WASM | 36 MB (libkuzu.a) — taille finale .wasm a mesurer apres link JS |

---

## Etape 3 : Extension C++ complete

**Objectif :** Ecrire le wrapper C++ qui expose Tantivy via des fonctions Cypher.

### Structure

```
extension/tantivy_fts/src/
├── main/
│   └── tantivy_fts_extension.cpp    ← load(), enregistre fonctions Cypher
├── function/
│   ├── create_tantivy_index.cpp     ← CREATE_TANTIVY_INDEX
│   ├── drop_tantivy_index.cpp       ← DROP_TANTIVY_INDEX
│   └── query_tantivy_index.cpp      ← QUERY_TANTIVY_INDEX (4 modes)
├── index/
│   └── tantivy_index.cpp            ← TantivyIndex (wrapper FFI, insert/delete/checkpoint)
└── catalog/
    └── tantivy_catalog_entry.cpp    ← Serialisation metadata dans catalog Kuzu
```

### Fonctions Cypher

```sql
-- Creation
CALL CREATE_TANTIVY_INDEX('Table', 'index_name', ['field1', 'field2'],
    stemmer := 'english');

-- Recherche (4 modes)
CALL QUERY_TANTIVY_INDEX('Table', 'index_name', 'query text',
    mode := 'parse')          -- defaut, stemmed
RETURN node, score;

CALL QUERY_TANTIVY_INDEX('Table', 'index_name', 'query text',
    mode := 'fuzzy', distance := 1)
RETURN node, score;

CALL QUERY_TANTIVY_INDEX('Table', 'index_name', 'pattern.*',
    mode := 'regex')
RETURN node, score;

CALL QUERY_TANTIVY_INDEX('Table', 'index_name', 'exactTerm',
    mode := 'exact')           -- reroute vers regex .*exactterm.*
RETURN node, score;

-- Avec filtre graph
MATCH (n:Table) WHERE n.year > 2020
WITH collect(n._node_id) AS ids
CALL QUERY_TANTIVY_INDEX('Table', 'index_name', 'query',
    filter_ids := ids)
RETURN node, score;

-- Suppression
CALL DROP_TANTIVY_INDEX('Table', 'index_name');
```

### Mapping mode → FFI

Le wrapper C++ traduit les modes publics en appels FFI :

| Mode Cypher | Construction query_json | Fonction FFI |
|-------------|------------------------|--------------|
| `parse` (defaut) | `{"type":"parse","fields":[...],"value":"..."}` | `tantivy_search` |
| `fuzzy` | `{"type":"fuzzy","field":"...","value":"...","distance":N}` | `tantivy_search` |
| `regex` | `{"type":"regex","field":"...","pattern":"..."}` | `tantivy_search` |
| `exact` | `{"type":"regex","field":"...","pattern":".*{value}.*"}` | `tantivy_search` |
| (tout) + `filter_ids` | idem | `tantivy_search_filtered` |

### Insertion/suppression incrementale

```cpp
// TantivyIndex herite de storage::Index
void TantivyIndex::insert(...) {
    // Construit le doc JSON depuis les colonnes Kuzu
    // Ajoute _node_id
    // Appelle tantivy_add_document()
}

void TantivyIndex::delete_(...) {
    // Appelle tantivy_delete_by_term("_node_id", id)
}

void TantivyIndex::checkpoint() {
    tantivy_commit(handle);
    tantivy_reload_reader(handle);
}
```

---

## Etape 4 : Tests end-to-end

### Natif

1. Build rag3db complet avec extension tantivy-fts
2. Creer une base Kuzu, creer une table, inserer des noeuds
3. `CREATE_TANTIVY_INDEX` sur la table
4. `QUERY_TANTIVY_INDEX` dans les 4 modes
5. Verifier scores, filtrage, delete/reindex

### WASM (kuzu-wasm)

1. Build WASM complet
2. Charger dans Node.js ou browser
3. Memes tests via l'API JS de Kuzu WASM
4. Verifier persistence VFS (IDBFS si browser)

### Metriques a mesurer

- Taille `.wasm` finale (Kuzu + Tantivy + Vector)
- Temps de creation d'index (1K, 10K, 100K documents)
- Temps de recherche (par mode)
- Memoire utilisee

---

## Etape 5 : Integration Rag3Weaver

1. Modifier `CatalogSearch` pour utiliser `QUERY_TANTIVY_INDEX` au lieu de `QUERY_FTS_INDEX`
2. Ajouter les options `mode` et `distance` dans les parametres de search
3. Tests end-to-end avec le pipeline complet (ingestion → search → results)

---

## Resume

| Etape | Risque | Effort | Bloque par | Status |
|-------|--------|--------|------------|--------|
| 1. CMake link natif | ~~Eleve~~ | ~~Moyen~~ | Rien | **FAIT** — 15 KB extension, link OK |
| 2. CMake link WASM | ~~Moyen~~ | ~~Faible~~ | ~~Etape 1~~ | **FAIT** — 36 MB libkuzu.a, 4 extensions |
| 3. Extension C++ | Faible (pattern connu) | **Eleve** | ~~Etapes 1+2~~ | A faire |
| 4. Tests end-to-end | Faible | Moyen | Etape 3 | A faire |
| 5. Rag3Weaver | Faible | Faible | Etape 4 | A faire |

Les etapes 1 et 2 sont terminees. Le point de bascule (link) est passe avec succes. Le reste est du code applicatif.
