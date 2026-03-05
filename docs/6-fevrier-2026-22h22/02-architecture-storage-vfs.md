# Architecture Storage : Extension Lucivy-FTS dans rag3db

> Suite de `01-progression.md`. Definit l'approche de stockage pour Lucivy et son integration comme extension rag3db.

---

## Principe fondamental : extension rag3db, pas kuzu-wasm

L'extension Lucivy-FTS doit fonctionner **partout ou rag3db tourne** :

| Plateforme | Directory Lucivy | Stockage | Persistence |
|------------|-------------------|----------|-------------|
| **Natif** (Linux, macOS, Windows) | `MmapDirectory` | Vrai filesystem | Automatique (fichiers sur disque) |
| **WASM** (browser, Node) | `EmscriptenDirectory` (`std::fs`) | VFS Emscripten (MEMFS) | IDBFS → IndexedDB via `FS.syncfs()` |
| **Tests** | `RamDirectory` | RAM pure | Aucune (ephemere) |

Le choix se fait a la compilation via `#[cfg()]` dans le crate Rust. Le code C++ de l'extension n'a pas besoin de savoir - il appelle la meme API C.

```rust
fn open_directory(path: &str) -> Box<dyn Directory> {
    #[cfg(target_os = "emscripten")]
    { Box::new(EmscriptenDirectory::open(path)) }

    #[cfg(not(target_os = "emscripten"))]
    { Box::new(MmapDirectory::open_path(path).expect("cannot open directory")) }
}
```

---

## Placement dans rag3db

L'extension FTS existante vit dans `packages/rag3db/extension/fts/`. La nouvelle extension Lucivy suit le meme pattern :

```
packages/rag3db/extension/
├── fts/                    ← Extension FTS actuelle (BM25 via tables internes Kuzu)
│   ├── CMakeLists.txt
│   ├── src/
│   │   ├── main/fts_extension.cpp         ← init(), enregistre fonctions Cypher
│   │   ├── function/                       ← CREATE/DROP/QUERY_FTS_INDEX
│   │   ├── index/fts_index.cpp            ← FTSIndex (stocke dans tables Kuzu)
│   │   ├── catalog/                        ← Serialisation metadata
│   │   └── utils/                          ← Tokenization, stemming
│   ├── third_party/snowball/               ← Stemming C
│   └── test/
│
├── lucivy_fts/            ← NOUVELLE extension (Lucivy, fichiers segments)
│   ├── CMakeLists.txt                      ← Link la static lib Rust + headers
│   ├── include/
│   │   └── lucivy_fts.h                  ← Header C genere (cbindgen)
│   ├── src/
│   │   ├── include/main/
│   │   │   └── lucivy_fts_extension.h    ← Header extension
│   │   ├── main/
│   │   │   ├── CMakeLists.txt             ← OBJECT library
│   │   │   └── lucivy_fts_extension.cpp  ← load(), enregistre fonctions Cypher
│   │   ├── function/                       ← CREATE/DROP/QUERY_LUCIVY_INDEX (a faire)
│   │   ├── index/lucivy_index.cpp        ← LucivyIndex wrapper (a faire)
│   │   └── catalog/                        ← Metadata dans le catalog Kuzu (a faire)
│   ├── rust/                               ← Crate Rust avec FFI C
│   │   ├── Cargo.toml
│   │   ├── cbindgen.toml
│   │   └── src/
│   │       ├── lib.rs                     ← API extern "C" (13 fonctions)
│   │       ├── directory.rs               ← StdFsDirectory (std::fs, agnostique)
│   │       ├── handle.rs                  ← LucivyHandle (index + writer + reader)
│   │       └── query.rs                   ← Parsing queries, fuzzy, regex
│   └── test/
│
├── vector/                 ← Extension Vector (HNSW)
└── ...
```

### Enregistrement dans rag3db

```cmake
# extension/CMakeLists.txt (modifie)
add_extension_if_enabled("lucivy_fts")
```

```cmake
# extension/extension_config.cmake (modifie)
# Ajoute a EXTENSION_LIST et aux blocs static link WASM/Android/Swift
add_static_link_extension(lucivy_fts)
```

### Chargement de l'extension

```cpp
// lucivy_fts_extension.cpp - meme pattern que fts_extension.cpp
void LucivyFtsExtension::load(main::ClientContext* context) {
    auto& db = *context->getDatabase();

    // Fonctions Cypher
    ExtensionUtils::addStandaloneTableFunc<CreateLucivyIndexFunction>(db);
    ExtensionUtils::addStandaloneTableFunc<DropLucivyIndexFunction>(db);
    ExtensionUtils::addTableFunc<QueryLucivyIndexFunction>(db);

    // Type d'index
    ExtensionUtils::registerIndexType(db, LucivyIndex::getIndexType());

    // Charger les index existants
    initLucivyEntries(context, *db.getCatalog());
}
```

---

## Stockage des segments Lucivy

### Difference cle avec l'extension FTS actuelle

L'extension FTS actuelle stocke tout dans des **tables internes Kuzu** :
- `__fts_{tableID}_{indexName}_docs` (noeuds)
- `__fts_{tableID}_{indexName}_terms` (noeuds)
- `__fts_{tableID}_{indexName}_appears_in` (relations)

Lucivy gere ses propres **fichiers segments** sur disque. L'extension Lucivy-FTS stocke donc :
- **Metadata** (config, schema, stats) → dans le catalog Kuzu (comme FTS)
- **Fichiers segments** → dans un sous-repertoire du data directory de Kuzu

### Emplacement des fichiers

```
{kuzu_db_path}/
├── nodes.kz                       ← Donnees Kuzu (existant)
├── rels.kz
├── catalog.kz
└── lucivy/                        ← Repertoire cree par l'extension
    ├── {table_id}_{index_name}/   ← Un dossier par index Lucivy
    │   ├── meta.json              ← Metadata Lucivy (segment list, schema)
    │   ├── {seg_id}.idx           ← Inverted index
    │   ├── {seg_id}.pos           ← Term positions
    │   ├── {seg_id}.store         ← Document store
    │   ├── {seg_id}.fast          ← Fast fields (columnar)
    │   ├── {seg_id}.fn            ← Field norms
    │   └── {seg_id}.del           ← Alive bitset (deletions)
    └── {autre_table}_{autre_index}/
        └── ...
```

Le path est construit par l'extension C++ :

```cpp
std::string LucivyIndex::getIndexPath() const {
    // db_path vient du StorageManager de Kuzu
    return storageManager->getDataDir() + "/lucivy/"
         + std::to_string(tableID) + "_" + indexName;
}
```

Ce path est passe a la FFI Rust qui cree le bon Directory selon la plateforme.

### En natif vs WASM

- **Natif** : `{db_path}/lucivy/...` sont de vrais fichiers sur disque. `MmapDirectory` les mappe en memoire. Persistence automatique.
- **WASM** : le meme chemin existe dans le VFS Emscripten. `EmscriptenDirectory` y accede via `std::fs`. IDBFS peut persister vers IndexedDB si monte sur `/lucivy/`.

L'extension ne fait aucune distinction - c'est le crate Rust qui choisit le bon Directory a la compilation.

---

## Architecture des segments Lucivy (rappel)

Lucivy utilise un modele **segments immutables** :

```
Commit 1 → [Segment A: doc1, doc2, doc3]
Commit 2 → [Segment A] + [Segment B: doc4, doc5]        ← A n'est pas modifie
Commit 3 → [Segment A] + [Segment B] + [Segment C: doc6]
   ... merge automatique (background) ...
           → [Segment AB: doc1-5] + [Segment C: doc6]    ← fusion, vieux segments supprimes
```

- **Ajout** : nouveaux documents → nouveau segment au commit
- **Suppression** : alive bitset marque les docs supprimes (`.del` file)
- **Merge** : la merge policy (LogMergePolicy) fusionne les petits segments en background
- **Pas de rebuild** : on n'a jamais besoin de reconstruire l'index entier

La merge policy est du Rust pur (pas de dependance tokio), les threads de merge utilisent pthreads (supportes sur Emscripten via SharedArrayBuffer).

---

## API C FFI

L'API est agnostique de la plateforme. Le `path` est un chemin filesystem (reel en natif, VFS en WASM).

```c
// === Lifecycle ===
// Cree un nouvel index. Le repertoire est cree si necessaire.
LucivyHandle* lucivy_create_index(const char* path, const char* schema_json);

// Ouvre un index existant (charge les segments depuis le path).
LucivyHandle* lucivy_open_index(const char* path);

// Ferme l'index, libere les ressources.
void lucivy_close_index(LucivyHandle* handle);


// === Ecriture (incrementale) ===
// Ajoute un document. Retourne l'opstamp (identifiant monotone de l'operation).
int64_t lucivy_add_document(LucivyHandle* handle, const char* doc_json);

// Supprime les documents matchant le terme. Effectif au prochain commit.
int64_t lucivy_delete_by_term(LucivyHandle* handle, const char* field, const char* value);

// Finalise les operations en cours. Cree un nouveau segment.
// Retourne l'opstamp du commit.
int64_t lucivy_commit(LucivyHandle* handle);

// Annule les operations non commitees.
void lucivy_rollback(LucivyHandle* handle);


// === Lecture ===
// Recherche. query_json contient le type de query (term, phrase, fuzzy, regex, boolean).
// Retourne un JSON avec les resultats [{score, doc}, ...].
char* lucivy_search(LucivyHandle* handle, const char* query_json, uint32_t limit);

// Recherche filtree par node IDs. Seuls les documents dont _node_id est dans
// allowed_ids seront scores. Flow : Cypher WHERE → node IDs → FTS filtre.
// Utilise FilterCollector de Lucivy sur le champ _node_id (u64 FAST).
char* lucivy_search_filtered(LucivyHandle* handle, const char* query_json,
                               uint32_t limit, const uint64_t* allowed_ids, uint32_t num_ids);

// Recharge le reader pour voir les derniers commits.
void lucivy_reload_reader(LucivyHandle* handle);

// Libere une string allouee par le crate Rust.
void lucivy_free_string(char* ptr);


// === Info ===
char* lucivy_get_schema(LucivyHandle* handle);
uint64_t lucivy_num_docs(LucivyHandle* handle);
```

### Format du query_json (API interne FFI — 6 types)

```json
// Recherche exacte → champ ._raw (precision)
{"type": "term", "field": "body", "value": "function"}

// Recherche fuzzy (Levenshtein) → champ ._raw (precision)
{"type": "fuzzy", "field": "body", "value": "fonctoin", "distance": 2}

// Recherche phrase (auto-tokenized) → champ stemmed (recall)
{"type": "phrase", "field": "body", "terms": ["hello", "world"]}

// Recherche regex → champ ._raw (precision)
{"type": "regex", "field": "body", "pattern": "func.*ion"}

// Recherche booleenne (combinaison)
{"type": "boolean", "must": [...], "should": [...], "must_not": [...]}

// Query parser (syntaxe utilisateur) → champ stemmed (recall)
{"type": "parse", "field": "body", "value": "hello world"}
```

**Routing dual-field** : quand un stemmer est actif, le routing est transparent. L'utilisateur reference toujours le nom de base ("body"), le crate redirige vers `body` (stemmed) ou `body._raw` selon le type de query.

### Architecture dual-field (stemming actif)

Quand `"stemmer": "english"` est specifie dans le schema, chaque champ "text" genere deux champs Lucivy :

```
body        → tokenizer "stemmed" (SimpleTokenizer + LowerCaser + Stemmer)
body._raw   → tokenizer "default" (SimpleTokenizer + LowerCaser)
```

- Les champs `._raw` ne sont **pas stockes** (pas de duplication dans les resultats de recherche)
- La duplication des valeurs dans `._raw` est faite automatiquement par `lucivy_add_document()`
- La config (stemmer + champs) est persistee dans `_config.json` a cote des segments

| Type query | Champ utilise | Pourquoi |
|------------|---------------|----------|
| `term` | `._raw` | Match exact sur forme originale lowercased |
| `fuzzy` | `._raw` | Levenshtein sur forme originale |
| `regex` | `._raw` | Pattern sur forme originale |
| `phrase` | stemmed | Auto-tokenize → match malgre flexions |
| `parse` | stemmed | Pipeline tokenizer complet |

### Format du schema_json

```json
{
  "fields": [
    {"name": "body", "type": "text", "stored": true, "indexed": true},
    {"name": "title", "type": "text", "stored": true, "indexed": true},
    {"name": "doc_id", "type": "u64", "stored": true, "indexed": false, "fast": true}
  ],
  "tokenizer": "simple",
  "stemmer": "english"
}
```

**Note :** Le champ `_node_id` (u64 FAST + INDEXED) est automatiquement ajoute a tout schema par le crate Rust. Il n'a pas besoin d'etre specifie dans le `schema_json`. C'est le lien entre les noeuds Kuzu et les documents Lucivy.

---

## Fonctions Cypher exposees

Meme pattern que l'extension FTS actuelle.

### API publique (4 modes)

L'API publique simplifie les 6 types internes FFI en **4 modes** destines aux utilisateurs et aux LLMs :

| Mode Cypher | Type FFI interne | Champ | Description |
|-------------|-----------------|-------|-------------|
| `parse` (defaut) | `parse` | stemmed | Recherche en langage naturel, stemming actif |
| `fuzzy` | `fuzzy` | raw | Tolerant aux fautes de frappe (Levenshtein) |
| `regex` | `regex` | raw | Pattern matching sur les termes |
| `exact` | `regex` (avec `.*{term}.*`) | raw | Match exact d'un terme connu, reroute vers regex |

**`exact`** n'est pas un type FFI a part : le wrapper C++ transforme `exact "myFunction"` en `regex ".*myfunction.*"` (lowercased). C'est utile pour un agent/LLM qui connait le terme exact et veut le trouver dans le texte.

Les types internes `term`, `phrase` et `boolean` restent disponibles dans l'API FFI pour des cas avances, mais ne sont pas exposes directement en Cypher.

### Exemples Cypher

```sql
-- Creer un index Lucivy sur une table
CALL CREATE_LUCIVY_INDEX('Article', 'article_fts', ['title', 'body'],
    stemmer := 'english');

-- Recherche full-text (defaut : mode parse, stemmed)
CALL QUERY_LUCIVY_INDEX('Article', 'article_fts', 'running programs')
RETURN node, score;
-- → Trouve aussi "ran", "program", "programming" via stemming

-- Recherche fuzzy (tolerant aux typos)
CALL QUERY_LUCIVY_INDEX('Article', 'article_fts', 'programing',
    mode := 'fuzzy', distance := 1)
RETURN node, score;
-- → Trouve "programming" (1 lettre de difference)

-- Recherche regex
CALL QUERY_LUCIVY_INDEX('Article', 'article_fts', 'func.*ion',
    mode := 'regex')
RETURN node, score;

-- Recherche exact (pour un agent/LLM qui connait le terme)
CALL QUERY_LUCIVY_INDEX('Article', 'article_fts', 'myFunction',
    mode := 'exact')
RETURN node, score;
-- → Reroute vers regex ".*myfunction.*" en interne

-- Recherche filtree (graph WHERE → FTS sur subset)
MATCH (n:Article) WHERE n.year > 2020
WITH collect(n._node_id) AS ids
CALL QUERY_LUCIVY_INDEX('Article', 'article_fts', 'search query',
    filter_ids := ids)
RETURN node, score;

-- Supprimer un index
CALL DROP_LUCIVY_INDEX('Article', 'article_fts');
```

### Insertion/suppression incrementale

Quand un noeud est insere/supprime dans Kuzu, l'extension intercepte via le systeme d'index :

```cpp
// LucivyIndex herite de storage::Index
void LucivyIndex::insert(Transaction* tx, const ValueVector& nodeIDs,
                           const vector<ValueVector*>& indexVectors,
                           InsertState& state) {
    // Pour chaque noeud : appel FFI lucivy_add_document()
    // Le commit est fait par checkpoint()
}

void LucivyIndex::delete_(Transaction* tx, const ValueVector& nodeIDs,
                            DeleteState& state) {
    // Pour chaque noeud : appel FFI lucivy_delete_by_term()
}

void LucivyIndex::checkpoint() {
    // Appel FFI lucivy_commit()
    // En WASM : le JS peut ensuite appeler FS.syncfs() pour persister
}
```

---

## Synergies avec Rag3Weaver

### Constat actuel

Rag3Weaver (`kuzu-wasm-exp/src/`) est 100% string-centric :
- Pas de gestion de fichiers (ni blob, ni binaire, ni reference externe)
- Pas de content extraction (suppose que le texte arrive deja extrait)
- Pas de storage abstraction au-dela de Kuzu

### Ce que l'extension apporte

Avec Lucivy comme extension rag3db, Rag3Weaver beneficie de fuzzy/regex FTS **via Cypher**, sans rien changer a son code :

```typescript
// CatalogSearch peut simplement utiliser QUERY_LUCIVY_INDEX au lieu de QUERY_FTS_INDEX
const results = await conn.execute(`
    CALL QUERY_LUCIVY_INDEX('Article', 'article_fts', $query, fuzzy := 2)
    RETURN node._uuid AS uuid, score
    ORDER BY score DESC LIMIT $limit
`, { query, limit });
```

### Futur : stockage de documents bruts

Le meme mecanisme de fichiers (sous-repertoire du data directory) peut servir pour le stockage de documents bruts dans une future extension :

```
{kuzu_db_path}/
├── lucivy/           ← Extension lucivy_fts (segments FTS)
├── documents/         ← Future extension document-store (fichiers bruts)
│   ├── {uuid}.pdf
│   ├── {uuid}.md
│   └── ...
└── ...
```

En natif : vrais fichiers. En WASM : VFS Emscripten. Meme pattern, meme code. Le VFS n'est pas une feature specifique a WASM - c'est juste que le filesystem fonctionne differemment selon la plateforme, et l'abstraction est geree au niveau le plus bas (Rust `std::fs` / Emscripten libc).

---

## Comparaison avec l'extension FTS actuelle

| Aspect | Extension FTS (actuelle) | Extension Lucivy-FTS (nouvelle) |
|--------|-------------------------|----------------------------------|
| Stockage index | Tables internes Kuzu | Fichiers segments (filesystem) |
| Moteur | Custom BM25 sur tables | Lucivy (BM25 natif) |
| Fuzzy | Via fuzzy-fst (expansion termes) | Levenshtein automaton natif |
| Regex | Non | Oui |
| Phrase queries | Non | Oui |
| Incremental | Oui (insert/delete par noeud) | Oui (segments immutables + merge) |
| Merge | Non applicable | Automatique (LogMergePolicy) |
| Stemming | Snowball (C) | rust-stemmers |
| Tokenization | Simple / Jieba | Simple (extensible) |
| Plateforme | Partout (C++ pur) | Partout (Rust FFI + cfg) |

A terme, l'extension Lucivy-FTS pourrait **remplacer** l'extension FTS existante, mais les deux peuvent coexister pendant la transition.

---

## Prochaines etapes

### Phase 1 : Crate Rust FFI — FAIT

- [x] Crate `lucivy-fts` cree dans `extension/lucivy_fts/rust/`
- [x] `StdFsDirectory` implemente (agnostique plateforme, ~100 lignes)
- [x] 13 fonctions `extern "C"` (create/open/close, add/delete/commit/rollback, search/search_filtered/reload, schema/num_docs/free)
- [x] Support queries : term, fuzzy, phrase, regex, boolean, parse
- [x] Compilation native : 26 MB, 13s
- [x] Compilation Emscripten : 17 MB, 40s
- [x] Generer le header C via cbindgen (`include/lucivy_fts.h`)
- [x] Test natif du cycle complet : 63/63 (create → add → commit → search → filtered → delete → reopen → dual-field)
- [x] Architecture dual-field : stemmed + `._raw` avec routing transparent par type de query
- [x] Persistence config : `_config.json` pour reopen avec bon tokenizer

### Phase 2 : Integration CMake + Extension C++ — EN COURS

**Build (FAIT) :**
- [x] Renommage `lucivy-fts/` → `lucivy_fts/` (convention CMake)
- [x] `LucivyFtsExtension::load()` stub (verifie link FFI)
- [x] CMakeLists.txt qui link la static lib Rust (cargo build via add_custom_command)
- [x] Build natif OK : `liblucivy_fts.kuzu_extension` (15 KB)
- [x] Build WASM OK : `libkuzu.a` (36 MB) avec json+vector+algo+lucivy_fts
- [x] Extension FTS originale exclue du build WASM (bug `DOC_FREQUENCY_PROP_NAME`)

**Extension C++ (A FAIRE) :**
- [ ] Wrapper C++ `LucivyIndex` qui appelle la FFI
- [ ] Fonctions Cypher CREATE/DROP/QUERY_LUCIVY_INDEX
- [ ] Serialisation catalog entry

### Phase 3 : Tests

- [ ] Tests unitaires (Rust + C++ + Cypher)
- [ ] Tests integration avec Rag3Weaver
- [ ] Mesurer taille WASM finale et performance

### Phase 4 : Integration Rag3Weaver

- [ ] Modifier CatalogSearch pour utiliser QUERY_LUCIVY_INDEX
- [ ] Ajouter options fuzzy/regex dans les parametres de search
- [ ] Tests end-to-end
