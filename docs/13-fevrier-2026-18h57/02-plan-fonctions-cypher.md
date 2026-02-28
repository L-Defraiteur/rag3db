# Plan — Fonctions Cypher pour tantivy_fts

> Analyse de l'extension system rag3db et design des fonctions Cypher.
> Priorité : QUERY avec NgramContains + highlights.

---

## 1. Analyse de l'extension system rag3db

### Pattern d'extension

Chaque extension hérite de `extension::Extension` et implémente `load()` :

```cpp
// extension header
class TantivyFtsExtension final : public extension::Extension {
public:
    static constexpr char EXTENSION_NAME[] = "TANTIVY_FTS";
    static void load(main::ClientContext* context);
};

// registration dans load()
void TantivyFtsExtension::load(main::ClientContext* context) {
    auto& db = *context->getDatabase();
    ExtensionUtils::addTableFunc<QueryTantivyFunction>(db);              // regular
    ExtensionUtils::addStandaloneTableFunc<CreateTantivyFunction>(db);   // standalone (DDL-like)
    ExtensionUtils::addStandaloneTableFunc<DropTantivyFunction>(db);     // standalone
}
```

### Types de table functions

| Type | Signature `tableFunc` | Usage |
|------|----------------------|-------|
| **SimpleTableFunc** | `offset_t(const TableFuncMorsel&, const TableFuncInput&, DataChunk& output)` | Retourne des lignes (QUERY) |
| **Standalone** | `offset_t(const TableFuncInput&, TableFuncOutput&)` | DDL-like, peut utiliser `rewriteFunc` |
| **GDS** | via `VertexCompute` | Scan de graphe massif |

### Pattern SimpleTableFunc (pour QUERY)

Référence : `src/function/table/current_setting.cpp`

```cpp
// 1. BindData custom (stocke les params parsés)
struct MyBindData final : TableFuncBindData {
    std::string param;
    MyBindData(std::string p, binder::expression_vector columns, offset_t numRows)
        : TableFuncBindData{std::move(columns), numRows}, param{std::move(p)} {}
    std::unique_ptr<TableFuncBindData> copy() const override { ... }
};

// 2. bindFunc (parse params, define output schema)
static std::unique_ptr<TableFuncBindData> bindFunc(
    const ClientContext* context, const TableFuncBindInput* input) {
    auto param = input->getLiteralVal<std::string>(0);
    std::vector<std::string> columnNames = {"col1", "col2"};
    std::vector<LogicalType> columnTypes = {LogicalType::INT64(), LogicalType::STRING()};
    columnNames = TableFunction::extractYieldVariables(columnNames, input->yieldVariables);
    auto columns = input->binder->createVariables(columnNames, columnTypes);
    return std::make_unique<MyBindData>(param, columns, maxRows);
}

// 3. internalTableFunc (produit les lignes de résultat)
static offset_t internalTableFunc(
    const TableFuncMorsel& morsel, const TableFuncInput& input, DataChunk& output) {
    auto* bd = input.bindData->constPtrCast<MyBindData>();
    // écrire dans output.getValueVectorMutable(0), (1), etc.
    return numRowsProduced;
}

// 4. getFunctionSet (registration)
function_set MyFunction::getFunctionSet() {
    function_set fs;
    auto func = std::make_unique<TableFunction>(name,
        std::vector{LogicalTypeID::STRING});
    func->tableFunc = SimpleTableFunc::getTableFunc(internalTableFunc);
    func->bindFunc = bindFunc;
    func->initSharedStateFunc = SimpleTableFunc::initSharedState;
    func->initLocalStateFunc = TableFunction::initEmptyLocalState;
    fs.push_back(std::move(func));
    return fs;
}
```

### Pattern Standalone (pour CREATE/DROP)

Référence : `extension/fts/src/function/create_fts_index.cpp`

Le FTS extension utilise `rewriteFunc` pour générer du Cypher qui crée des tables internes (docs, terms, appears_in), puis `_CREATE_FTS_INDEX` interne fait le travail lourd (statistiques, catalogue).

Pour tantivy_fts, on n'a pas besoin de tables internes pour le moteur de recherche (Tantivy gère tout ça nativement). En revanche, on utilise **une table de registre interne** `_tantivy_indexes` pour persister les métadonnées des index créés (voir section 2.2).

### Accès au contexte

```cpp
// Dans tableFunc :
auto& context = *input.context;                    // ExecutionContext
auto* clientContext = context.clientContext;         // ClientContext
std::string dbPath = clientContext->getDatabasePath(); // chemin base de données
auto* db = clientContext->getDatabase();             // Database*
```

---

## 2. Infrastructure commune

### TantivyHandleMap — Stockage global des handles

```cpp
// tantivy_handle_map.h
#pragma once
#include "tantivy_fts.h"
#include <mutex>
#include <string>
#include <unordered_map>

namespace rag3db {
namespace tantivy_fts_extension {

class TantivyHandleMap {
public:
    static TantivyHandleMap& instance();

    // Open or create an index, returns the handle.
    // Thread-safe. Opens only once per indexPath.
    TantivyHandlePtr getOrOpen(const std::string& indexPath);
    TantivyHandlePtr getOrCreate(const std::string& indexPath, const std::string& schemaJson);

    // Get an existing handle (returns nullptr if not found).
    TantivyHandlePtr get(const std::string& indexPath);

    // Close and remove from map.
    void close(const std::string& indexPath);

    // Close all handles (called on extension unload).
    void closeAll();

private:
    TantivyHandleMap() = default;
    std::mutex mutex_;
    std::unordered_map<std::string, TantivyHandlePtr> handles_;
};

} // namespace tantivy_fts_extension
} // namespace rag3db
```

### Convention de chemin d'index

```
{databasePath}/tantivy_indexes/{tableName}/
```

Exemple : `/data/mydb/tantivy_indexes/doc/`

### 2.2 Table de registre `_tantivy_indexes`

Table interne (nœud) qui persiste les métadonnées de chaque index Tantivy créé. Source de vérité persistante ; le `TantivyHandleMap` en est le cache mémoire.

#### Schema

```cypher
CREATE NODE TABLE _tantivy_indexes (
    table_name STRING PRIMARY KEY,
    index_path STRING,
    fields STRING,       -- JSON array : '["title","body"]'
    stemmer STRING,      -- "english" ou "" si pas de stemming
    num_docs UINT64,
    created_at STRING    -- ISO 8601 : "2026-02-13T19:30:00"
);
```

**Clé primaire `table_name`** : un seul index Tantivy par table pour l'instant. Si on veut plusieurs index par table plus tard, on passera à un PK composite ou SERIAL.

#### Lifecycle

| Moment | Action |
|--------|--------|
| **Extension load** | Si `_tantivy_indexes` existe dans le catalogue → lire toutes les lignes → peupler `TantivyHandleMap` (lazy open : on note les paths, on ouvre les handles à la première requête) |
| **CREATE_TANTIVY_INDEX** | Si `_tantivy_indexes` n'existe pas → la créer. Insérer une ligne avec les métadonnées. |
| **QUERY_TANTIVY_INDEX** | Lire la ligne pour trouver `index_path`. Si absent → erreur "No index on table X". |
| **DROP_TANTIVY_INDEX** | Supprimer la ligne. Si table vide → optionnellement la dropper. |
| **SHOW_TANTIVY_INDEXES** | `MATCH (i:_tantivy_indexes) RETURN i.*` (ou table func dédiée) |

#### Avantages

1. **Persistance** — après un restart, on sait quels index existent sans scanner le filesystem
2. **Discoverabilité** — `MATCH (i:_tantivy_indexes) RETURN i.*` pour lister les index
3. **Validation** — QUERY vérifie dans le registre avant d'essayer d'ouvrir un fichier
4. **Synchronisation** — comparer `num_docs` avec le count actuel de la table pour détecter des changements
5. **Métadonnées** — savoir quels champs sont indexés, quel stemmer, quand l'index a été créé

#### Implémentation dans `rewriteFunc`

CREATE_TANTIVY_INDEX utilise `rewriteFunc` pour générer le Cypher de gestion du registre :

```cypher
-- Créer la table de registre si elle n'existe pas (vérifié via catalog dans bindFunc)
CREATE NODE TABLE _tantivy_indexes (table_name STRING PRIMARY KEY, ...);

-- Insérer les métadonnées
CREATE (i:_tantivy_indexes {
    table_name: 'doc',
    index_path: '/data/mydb/tantivy_indexes/doc/',
    fields: '["title","body"]',
    stemmer: 'english',
    num_docs: 0,
    created_at: '2026-02-13T19:30:00'
});

-- Appeler la fonction interne qui fait le travail lourd (scan + index)
CALL _CREATE_TANTIVY_INDEX('doc', ['title', 'body']);

-- Mettre à jour num_docs après indexation
MATCH (i:_tantivy_indexes {table_name: 'doc'}) SET i.num_docs = <count>;
```

DROP_TANTIVY_INDEX :

```cypher
-- Appeler la fonction interne (close handle + rm dir)
CALL _DROP_TANTIVY_INDEX('doc');

-- Supprimer du registre
MATCH (i:_tantivy_indexes {table_name: 'doc'}) DELETE i;
```

---

## 3. QUERY_TANTIVY_INDEX — Priorité #1

### Syntaxe Cypher

```cypher
CALL QUERY_TANTIVY_INDEX('doc', '{"type":"contains","field":"body","value":"c++","highlight":true}', 10)
RETURN node_id, score, highlights;
```

| Param | Type | Description |
|-------|------|-------------|
| `tableName` | STRING | Nom de la table de nœuds |
| `queryJson` | STRING | JSON de requête (format QueryConfig de tantivy_fts) |
| `limit` | INT64 | Nombre max de résultats (optionnel, default 10) |

### Output columns

| Column | Type | Description |
|--------|------|-------------|
| `node_id` | INT64 | Offset du nœud dans la table (= `_node_id` Tantivy) |
| `score` | DOUBLE | Score BM25 |
| `highlights` | STRING | JSON : `{"body":[[5,16],[20,25]]}` ou `{}` |

### Flux d'exécution

```
bindFunc:
  1. Parse params (tableName, queryJson, limit)
  2. Vérifier que l'index existe :
     a. Chercher dans TantivyHandleMap (cache mémoire)
     b. Si absent → chercher dans _tantivy_indexes via catalog API
     c. Si absent → erreur "No Tantivy index on table 'X'. Use CREATE_TANTIVY_INDEX first."
  3. Récupérer indexPath (depuis map ou registre)
  4. Ouvrir/réutiliser handle via TantivyHandleMap::getOrOpen(indexPath)
  5. Appeler tantivy_search(handle, queryJson, limit)
  6. Parser le JSON résultat → stocker dans BindData
  7. Définir output columns (node_id INT64, score DOUBLE, highlights STRING)
  8. numRows = nombre de résultats

internalTableFunc (appelé par morsel):
  1. Lire le BindData
  2. Pour chaque résultat dans le morsel :
     - output[0].setValue(pos, result.node_id)      // INT64
     - output[1].setValue(pos, result.score)         // DOUBLE
     - output[2].setValue(pos, result.highlights)    // STRING (JSON)
  3. Retourner le nombre de lignes produites
```

### Pourquoi exécuter la recherche dans `bindFunc` ?

Le pattern SimpleTableFunc itère par morsels (chunks de DEFAULT_VECTOR_CAPACITY = 2048 lignes). Comme `tantivy_search` retourne tous les résultats d'un coup (JSON array), il est plus simple de :
1. Exécuter la recherche une seule fois dans `bindFunc`
2. Stocker les résultats parsés dans le BindData
3. Les distribuer par morsel dans `internalTableFunc`

Alternative : exécuter dans `initSharedState` ou dans le premier appel de `tableFunc` avec un flag `done`. Le `bindFunc` est le plus simple.

### Accès au registre depuis `bindFunc`

`bindFunc` reçoit un `ClientContext*`. Pour vérifier si l'index existe sans exécuter du Cypher, on utilise l'API catalogue interne :

```cpp
auto* catalog = catalog::Catalog::Get(*context);
auto* transaction = transaction::Transaction::Get(*context);
if (catalog->containsTable(transaction, "_tantivy_indexes")) {
    // Table existe → lire la propriété index_path du nœud avec table_name = tableName
    // Via StorageManager::getTable() + scan direct, ou via TantivyHandleMap déjà peuplé au load
}
```

En pratique, le plus simple : au `load()` de l'extension, lire le registre et peupler le `TantivyHandleMap` avec les `(tableName → indexPath)`. Ensuite `bindFunc` fait juste `TantivyHandleMap::get(tableName)`.

### FFI utilisée

```c
// Ouvrir l'index (si pas déjà en mémoire)
TantivyHandlePtr tantivy_open_index(const char* path);

// Rechercher
char* tantivy_search(TantivyHandlePtr handle, const char* query_json, uint32_t limit);

// Libérer le résultat
void tantivy_free_string(char* ptr);
```

Le JSON retourné par `tantivy_search` :
```json
[
  {"score": 1.23, "doc": {"body": "Rust programming", "_node_id": 42}, "highlights": {"body": [[5,16]]}},
  {"score": 0.87, "doc": {"body": "C++ guide", "_node_id": 7}, "highlights": {"body": [[0,3]]}}
]
```

On extrait `_node_id`, `score`, et `highlights` pour les output columns.

### Parsing JSON en C++

`nlohmann/json` est disponible dans `third_party/nlohmann_json/`. Include : `#include "nlohmann/json.hpp"`.

---

## 4. CREATE_TANTIVY_INDEX

### Syntaxe Cypher

```cypher
CALL CREATE_TANTIVY_INDEX('doc', ['title', 'body']);
-- Avec options :
CALL CREATE_TANTIVY_INDEX('doc', ['title', 'body'], stemmer := 'english');
```

| Param | Type | Description |
|-------|------|-------------|
| `tableName` | STRING | Nom de la table de nœuds |
| `fields` | LIST(STRING) | Propriétés à indexer |
| `stemmer` | STRING (optionnel) | Stemmer ("english", "french"...), default "english" |

### Architecture : Standalone avec `rewriteFunc` + Internal

Comme le FTS extension, on sépare en deux fonctions :

1. **`CREATE_TANTIVY_INDEX`** (publique, standalone) : `rewriteFunc` génère le Cypher pour le registre + appelle la fonction interne
2. **`_CREATE_TANTIVY_INDEX`** (interne) : fait le travail lourd (scan nodes, FFI Tantivy)

#### rewriteFunc — Gestion du registre

```cpp
std::string createTantivyIndexQuery(ClientContext& context, const TableFuncBindData& bindData) {
    auto* bd = bindData.constPtrCast<CreateTantivyBindData>();
    context.setUseInternalCatalogEntry(true);
    std::string query;

    // 1. Créer la table de registre si elle n'existe pas
    auto* catalog = catalog::Catalog::Get(context);
    auto* txn = transaction::Transaction::Get(context);
    if (!catalog->containsTable(txn, "_tantivy_indexes")) {
        query += "CREATE NODE TABLE _tantivy_indexes ("
                 "table_name STRING PRIMARY KEY, "
                 "index_path STRING, "
                 "fields STRING, "
                 "stemmer STRING, "
                 "num_docs UINT64, "
                 "created_at STRING);";
    }

    // 2. Insérer les métadonnées
    query += stringFormat(
        "CREATE (i:_tantivy_indexes {{"
        "table_name: '{}', index_path: '{}', fields: '{}', "
        "stemmer: '{}', num_docs: 0, created_at: '{}'}});",
        bd->tableName, bd->indexPath, bd->fieldsJson,
        bd->stemmer, bd->createdAt);

    // 3. Appeler la fonction interne
    query += stringFormat(
        "CALL _CREATE_TANTIVY_INDEX('{}', {}, stemmer := '{}');",
        bd->tableName, bd->fieldsLiteral, bd->stemmer);

    // 4. Mettre à jour num_docs (sera calculé par la fonction interne,
    //    stocké dans une variable globale ou re-compté via tantivy_num_docs)
    query += stringFormat(
        "RETURN 'Tantivy index created on table {}' AS result;",
        bd->tableName);

    return query;
}
```

#### _CREATE_TANTIVY_INDEX — Scan + Indexation

Le challenge : scanner TOUS les nœuds de la table et les insérer dans l'index Tantivy via le FFI C. Deux options :

**Option A — VertexCompute (scalable, recommandé)**

Comme le FTS extension : `OnDiskGraph` + `VertexCompute` pour itérer par batches. Chaque batch appelle `tantivy_add_document()`.

```cpp
class TantivyIndexCompute final : public VertexCompute {
    TantivyHandlePtr handle_;
    std::vector<std::string> fieldNames_;

    void vertexCompute(const graph::VertexScanState::Chunk& chunk) override {
        auto nodeIDs = chunk.getNodeIDs();
        for (auto i = 0u; i < nodeIDs.size(); i++) {
            nlohmann::json doc;
            doc["_node_id"] = nodeIDs[i].offset;
            for (size_t f = 0; f < fieldNames_.size(); f++) {
                auto val = chunk.getProperties<ku_string_t>(f);
                doc[fieldNames_[f]] = std::string(val[i].getData(), val[i].len);
            }
            tantivy_add_document(handle_, doc.dump().c_str());
        }
    }
};
```

**Option B — rewriteFunc + MATCH + collect (simple mais tout en mémoire)**

Problème : charge tous les docs en mémoire. Mauvais pour les grosses tables.

**On part sur Option A.**

#### Flux complet

```
bindFunc (CREATE_TANTIVY_INDEX — publique):
  1. Valider que la table existe et que les propriétés sont bien des STRING
  2. Vérifier qu'il n'y a pas déjà un index sur cette table (catalogue + registre)
  3. Construire indexPath, fieldsJson, stemmer
  4. Retourner CreateTantivyBindData

rewriteFunc:
  1. Créer _tantivy_indexes si nécessaire
  2. Insérer la ligne de registre
  3. Appeler _CREATE_TANTIVY_INDEX
  4. Retourner message

tableFunc (_CREATE_TANTIVY_INDEX — interne):
  1. Construire indexPath et schemaJson
  2. tantivy_create_index(indexPath, schemaJson) → handle
  3. Scanner les nœuds via VertexCompute → tantivy_add_document() par batch
  4. tantivy_commit(handle) + tantivy_reload_reader(handle)
  5. Stocker handle dans TantivyHandleMap
  6. Mettre à jour num_docs dans le registre :
     MATCH (i:_tantivy_indexes {table_name: '...'}) SET i.num_docs = tantivy_num_docs(handle)
  7. Retourner 0
```

---

## 5. DROP_TANTIVY_INDEX

### Syntaxe Cypher

```cypher
CALL DROP_TANTIVY_INDEX('doc');
```

### Architecture : Standalone avec `rewriteFunc` + Internal

Même pattern que CREATE : publique + interne.

#### rewriteFunc

```cpp
std::string dropTantivyIndexQuery(ClientContext& context, const TableFuncBindData& bindData) {
    auto* bd = bindData.constPtrCast<DropTantivyBindData>();
    context.setUseInternalCatalogEntry(true);
    std::string query;

    // 1. Appeler la fonction interne (close handle + rm dir)
    query += stringFormat("CALL _DROP_TANTIVY_INDEX('{}');", bd->tableName);

    // 2. Supprimer du registre
    query += stringFormat(
        "MATCH (i:_tantivy_indexes {{table_name: '{}'}}) DELETE i;",
        bd->tableName);

    query += stringFormat(
        "RETURN 'Tantivy index dropped on table {}' AS result;",
        bd->tableName);

    return query;
}
```

#### _DROP_TANTIVY_INDEX — Nettoyage

```
tableFunc:
  1. Récupérer indexPath depuis TantivyHandleMap ou le registre
  2. TantivyHandleMap::close(indexPath) → tantivy_close_index(handle)
  3. std::filesystem::remove_all(indexPath) → supprimer le dossier d'index
  4. Retourner 0
```

#### Flux `bindFunc`

```
bindFunc (DROP_TANTIVY_INDEX — publique):
  1. Vérifier que l'index existe (TantivyHandleMap ou registre)
  2. Si absent → erreur "No Tantivy index on table 'X'"
  3. Retourner DropTantivyBindData
```

---

## 6. Fichiers à créer/modifier

### Nouveaux fichiers

```
extension/tantivy_fts/
├── src/
│   ├── include/
│   │   ├── main/tantivy_fts_extension.h          (existe déjà)
│   │   └── function/
│   │       ├── tantivy_handle_map.h               ← NEW
│   │       ├── query_tantivy_index.h              ← NEW
│   │       ├── create_tantivy_index.h             ← NEW
│   │       └── drop_tantivy_index.h               ← NEW
│   ├── main/
│   │   ├── tantivy_fts_extension.cpp              (modifier)
│   │   └── CMakeLists.txt                         (existe déjà)
│   └── function/
│       ├── tantivy_handle_map.cpp                 ← NEW
│       ├── query_tantivy_index.cpp                ← NEW
│       ├── create_tantivy_index.cpp               ← NEW
│       ├── drop_tantivy_index.cpp                 ← NEW
│       └── CMakeLists.txt                         ← NEW
└── CMakeLists.txt                                 (modifier)
```

**Total : 8 fichiers à créer, 2 à modifier.**

### Détail des structs par fichier

#### `create_tantivy_index.h`

```cpp
struct CreateTantivyFunction {
    static constexpr const char* name = "CREATE_TANTIVY_INDEX";
    static function::function_set getFunctionSet();
};

struct InternalCreateTantivyFunction {
    static constexpr const char* name = "_CREATE_TANTIVY_INDEX";
    static function::function_set getFunctionSet();
};
```

#### `query_tantivy_index.h`

```cpp
struct QueryTantivyFunction {
    static constexpr const char* name = "QUERY_TANTIVY_INDEX";
    static function::function_set getFunctionSet();
};
```

#### `drop_tantivy_index.h`

```cpp
struct DropTantivyFunction {
    static constexpr const char* name = "DROP_TANTIVY_INDEX";
    static function::function_set getFunctionSet();
};

struct InternalDropTantivyFunction {
    static constexpr const char* name = "_DROP_TANTIVY_INDEX";
    static function::function_set getFunctionSet();
};
```

### Modifications

**`CMakeLists.txt` (racine extension)** — Ajouter :
```cmake
add_subdirectory(src/function)
```

**`src/function/CMakeLists.txt`** — Nouveau :
```cmake
add_library(tantivy_fts_extension_function
    OBJECT
    tantivy_handle_map.cpp
    query_tantivy_index.cpp
    create_tantivy_index.cpp
    drop_tantivy_index.cpp)

set(TANTIVY_FTS_EXTENSION_OBJECT_FILES
    ${TANTIVY_FTS_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:tantivy_fts_extension_function>
    PARENT_SCOPE)
```

**`src/main/tantivy_fts_extension.cpp`** — Remplacer le stub :

```cpp
void TantivyFtsExtension::load(main::ClientContext* context) {
    auto& db = *context->getDatabase();

    // Table functions
    ExtensionUtils::addTableFunc<QueryTantivyFunction>(db);

    // Standalone table functions (DDL)
    ExtensionUtils::addStandaloneTableFunc<CreateTantivyFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalCreateTantivyFunction>(db);
    ExtensionUtils::addStandaloneTableFunc<DropTantivyFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalDropTantivyFunction>(db);

    // Charger le registre si il existe → peupler TantivyHandleMap
    TantivyHandleMap::instance().loadFromRegistry(context);
}
```

---

## 7. Includes nécessaires

```cpp
// Pour les table functions
#include "function/table/table_function.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/simple_table_function.h"

// Pour le contexte
#include "main/client_context.h"
#include "processor/execution_context.h"

// Pour l'extension
#include "extension/extension.h"

// FFI Tantivy
#include "tantivy_fts.h"

// JSON parsing
#include "nlohmann/json.hpp"

// Pour le binder
#include "binder/binder.h"
```

---

## 8. Plan d'implémentation

### Étape 1 : Infrastructure

1. Créer `tantivy_handle_map.h` / `.cpp` (avec `loadFromRegistry`)
2. Créer `src/function/CMakeLists.txt`
3. Modifier `CMakeLists.txt` racine (ajouter `add_subdirectory(src/function)`)

### Étape 2 : CREATE_TANTIVY_INDEX

1. Créer `create_tantivy_index.h` / `.cpp`
   - `CreateTantivyFunction` (publique, standalone, rewriteFunc)
   - `InternalCreateTantivyFunction` (interne, tableFunc avec VertexCompute)
   - Gestion du registre `_tantivy_indexes`
2. Ajouter au CMakeLists
3. Build test

### Étape 3 : QUERY_TANTIVY_INDEX

1. Créer `query_tantivy_index.h` / `.cpp`
   - `QueryTantivyFunction` (SimpleTableFunc)
   - Lecture du registre / handle map
   - Appel FFI `tantivy_search` + parse JSON nlohmann
2. Build test

### Étape 4 : DROP_TANTIVY_INDEX

1. Créer `drop_tantivy_index.h` / `.cpp`
   - `DropTantivyFunction` (publique, standalone, rewriteFunc)
   - `InternalDropTantivyFunction` (interne, close handle + rm dir)
   - Suppression de la ligne dans le registre
2. Build test

### Étape 5 : Extension load + wiring

1. Modifier `tantivy_fts_extension.cpp` (registrations + loadFromRegistry)
2. Build complet : `cmake --build build/release`

### Étape 6 : Tests end-to-end

Tests dans rag3db shell :
```cypher
-- Setup
CREATE NODE TABLE doc (id INT64, title STRING, body STRING, PRIMARY KEY(id));
CREATE (d:doc {id: 1, title: "Rust Guide", body: "Rust programming is great"});
CREATE (d:doc {id: 2, title: "C++ Guide", body: "C++ systems programming language"});

-- Create index
CALL CREATE_TANTIVY_INDEX('doc', ['title', 'body']);

-- Verify registry
MATCH (i:_tantivy_indexes) RETURN i.*;

-- Query with highlights
CALL QUERY_TANTIVY_INDEX('doc',
    '{"type":"contains","field":"body","value":"programming","highlight":true}', 10)
RETURN node_id, score, highlights;

-- Query "c++" (separator validation)
CALL QUERY_TANTIVY_INDEX('doc',
    '{"type":"contains","field":"body","value":"c++","highlight":true}', 10)
RETURN node_id, score, highlights;

-- Drop
CALL DROP_TANTIVY_INDEX('doc');

-- Verify cleanup
MATCH (i:_tantivy_indexes) RETURN i.*;
```

---

## 9. Résumé des FFI C utilisées

| Fonction C | Utilisée par | Purpose |
|-----------|-------------|---------|
| `tantivy_create_index` | CREATE | Crée l'index sur disque |
| `tantivy_open_index` | QUERY (lazy open) | Ouvre un index existant |
| `tantivy_close_index` | DROP | Ferme et libère le handle |
| `tantivy_add_document` | CREATE | Ajoute un document |
| `tantivy_commit` | CREATE | Commit les writes |
| `tantivy_reload_reader` | CREATE (après commit) | Rend les docs visibles |
| `tantivy_search` | QUERY | Recherche → JSON résultats |
| `tantivy_free_string` | QUERY | Libère le JSON retourné |
| `tantivy_num_docs` | — | Diagnostic |
| `tantivy_get_schema` | — | Diagnostic |

---

## 10. Points d'attention

1. **Thread safety** : `TantivyHandleMap` est mutex-protégé. Les handles Tantivy eux-mêmes sont thread-safe (writer derrière Mutex<>, reader est lock-free).

2. **Lifecycle** : Les handles restent ouverts tant que l'extension est chargée. Fermeture propre via `closeAll()` si rag3db s'arrête.

3. **In-memory databases** : `getDatabasePath()` retourne `""` ou `":memory:"`. Il faudra gérer ce cas (index temporaire en mémoire ou erreur).

4. **nlohmann/json** : Disponible dans `third_party/nlohmann_json/`. Déjà utilisé par d'autres parties de rag3db (extension httpfs, parquet, etc.).

5. **Index inexistant** : Si QUERY est appelé avant CREATE, `tantivy_open_index` retourne null. Il faudra retourner une erreur claire.

6. **Table de registre `_tantivy_indexes`** :
   - Créée au premier `CREATE_TANTIVY_INDEX` (pas au load de l'extension)
   - `useInternalCatalogEntry(true)` pour que les tables internes ne soient pas visibles dans `SHOW TABLES`
   - Le PK `table_name` empêche de créer deux index sur la même table (erreur duplicate key)
   - Si la DB est supprimée/recréée, le registre est perdu mais les fichiers Tantivy aussi → cohérent

7. **Registre vs HandleMap** :
   - Le registre (`_tantivy_indexes`) est la source de vérité **persistante** (survit aux restarts)
   - Le `TantivyHandleMap` est le cache **mémoire** (handles ouverts, accès rapide)
   - `loadFromRegistry()` au load synchronise les deux
   - Les handles sont ouverts en **lazy** : on note le path dans le map, on ouvre le handle au premier QUERY

8. **Mise à jour incrémentale** (futur) : quand des nœuds sont ajoutés/supprimés de la table source après la création de l'index, il faudra un mécanisme de re-sync. Pour le v1, on suppose que l'index est créé une fois et n'est pas mis à jour incrémentalement. On peut toujours DROP + CREATE pour reconstruire.
