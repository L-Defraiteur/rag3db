# Plan — Fonctions Cypher pour lucivy_fts

> Analyse de l'extension system rag3db et design des fonctions Cypher.
> Priorité : QUERY avec NgramContains + highlights.

---

## 1. Analyse de l'extension system rag3db

### Pattern d'extension

Chaque extension hérite de `extension::Extension` et implémente `load()` :

```cpp
// extension header
class LucivyFtsExtension final : public extension::Extension {
public:
    static constexpr char EXTENSION_NAME[] = "LUCIVY_FTS";
    static void load(main::ClientContext* context);
};

// registration dans load()
void LucivyFtsExtension::load(main::ClientContext* context) {
    auto& db = *context->getDatabase();
    ExtensionUtils::addTableFunc<QueryLucivyFunction>(db);              // regular
    ExtensionUtils::addStandaloneTableFunc<CreateLucivyFunction>(db);   // standalone (DDL-like)
    ExtensionUtils::addStandaloneTableFunc<DropLucivyFunction>(db);     // standalone
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

Pour lucivy_fts, on n'a pas besoin de tables internes pour le moteur de recherche (Lucivy gère tout ça nativement). En revanche, on utilise **une table de registre interne** `_lucivy_indexes` pour persister les métadonnées des index créés (voir section 2.2).

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

### LucivyHandleMap — Stockage global des handles

```cpp
// lucivy_handle_map.h
#pragma once
#include "lucivy_fts.h"
#include <mutex>
#include <string>
#include <unordered_map>

namespace rag3db {
namespace lucivy_fts_extension {

class LucivyHandleMap {
public:
    static LucivyHandleMap& instance();

    // Open or create an index, returns the handle.
    // Thread-safe. Opens only once per indexPath.
    LucivyHandlePtr getOrOpen(const std::string& indexPath);
    LucivyHandlePtr getOrCreate(const std::string& indexPath, const std::string& schemaJson);

    // Get an existing handle (returns nullptr if not found).
    LucivyHandlePtr get(const std::string& indexPath);

    // Close and remove from map.
    void close(const std::string& indexPath);

    // Close all handles (called on extension unload).
    void closeAll();

private:
    LucivyHandleMap() = default;
    std::mutex mutex_;
    std::unordered_map<std::string, LucivyHandlePtr> handles_;
};

} // namespace lucivy_fts_extension
} // namespace rag3db
```

### Convention de chemin d'index

```
{databasePath}/lucivy_indexes/{tableName}/
```

Exemple : `/data/mydb/lucivy_indexes/doc/`

### 2.2 Table de registre `_lucivy_indexes`

Table interne (nœud) qui persiste les métadonnées de chaque index Lucivy créé. Source de vérité persistante ; le `LucivyHandleMap` en est le cache mémoire.

#### Schema

```cypher
CREATE NODE TABLE _lucivy_indexes (
    table_name STRING PRIMARY KEY,
    index_path STRING,
    fields STRING,       -- JSON array : '["title","body"]'
    stemmer STRING,      -- "english" ou "" si pas de stemming
    num_docs UINT64,
    created_at STRING    -- ISO 8601 : "2026-02-13T19:30:00"
);
```

**Clé primaire `table_name`** : un seul index Lucivy par table pour l'instant. Si on veut plusieurs index par table plus tard, on passera à un PK composite ou SERIAL.

#### Lifecycle

| Moment | Action |
|--------|--------|
| **Extension load** | Si `_lucivy_indexes` existe dans le catalogue → lire toutes les lignes → peupler `LucivyHandleMap` (lazy open : on note les paths, on ouvre les handles à la première requête) |
| **CREATE_LUCIVY_INDEX** | Si `_lucivy_indexes` n'existe pas → la créer. Insérer une ligne avec les métadonnées. |
| **QUERY_LUCIVY_INDEX** | Lire la ligne pour trouver `index_path`. Si absent → erreur "No index on table X". |
| **DROP_LUCIVY_INDEX** | Supprimer la ligne. Si table vide → optionnellement la dropper. |
| **SHOW_LUCIVY_INDEXES** | `MATCH (i:_lucivy_indexes) RETURN i.*` (ou table func dédiée) |

#### Avantages

1. **Persistance** — après un restart, on sait quels index existent sans scanner le filesystem
2. **Discoverabilité** — `MATCH (i:_lucivy_indexes) RETURN i.*` pour lister les index
3. **Validation** — QUERY vérifie dans le registre avant d'essayer d'ouvrir un fichier
4. **Synchronisation** — comparer `num_docs` avec le count actuel de la table pour détecter des changements
5. **Métadonnées** — savoir quels champs sont indexés, quel stemmer, quand l'index a été créé

#### Implémentation dans `rewriteFunc`

CREATE_LUCIVY_INDEX utilise `rewriteFunc` pour générer le Cypher de gestion du registre :

```cypher
-- Créer la table de registre si elle n'existe pas (vérifié via catalog dans bindFunc)
CREATE NODE TABLE _lucivy_indexes (table_name STRING PRIMARY KEY, ...);

-- Insérer les métadonnées
CREATE (i:_lucivy_indexes {
    table_name: 'doc',
    index_path: '/data/mydb/lucivy_indexes/doc/',
    fields: '["title","body"]',
    stemmer: 'english',
    num_docs: 0,
    created_at: '2026-02-13T19:30:00'
});

-- Appeler la fonction interne qui fait le travail lourd (scan + index)
CALL _CREATE_LUCIVY_INDEX('doc', ['title', 'body']);

-- Mettre à jour num_docs après indexation
MATCH (i:_lucivy_indexes {table_name: 'doc'}) SET i.num_docs = <count>;
```

DROP_LUCIVY_INDEX :

```cypher
-- Appeler la fonction interne (close handle + rm dir)
CALL _DROP_LUCIVY_INDEX('doc');

-- Supprimer du registre
MATCH (i:_lucivy_indexes {table_name: 'doc'}) DELETE i;
```

---

## 3. QUERY_LUCIVY_INDEX — Priorité #1

### Syntaxe Cypher

```cypher
CALL QUERY_LUCIVY_INDEX('doc', '{"type":"contains","field":"body","value":"c++","highlight":true}', 10)
RETURN node_id, score, highlights;
```

| Param | Type | Description |
|-------|------|-------------|
| `tableName` | STRING | Nom de la table de nœuds |
| `queryJson` | STRING | JSON de requête (format QueryConfig de lucivy_fts) |
| `limit` | INT64 | Nombre max de résultats (optionnel, default 10) |

### Output columns

| Column | Type | Description |
|--------|------|-------------|
| `node_id` | INT64 | Offset du nœud dans la table (= `_node_id` Lucivy) |
| `score` | DOUBLE | Score BM25 |
| `highlights` | STRING | JSON : `{"body":[[5,16],[20,25]]}` ou `{}` |

### Flux d'exécution

```
bindFunc:
  1. Parse params (tableName, queryJson, limit)
  2. Vérifier que l'index existe :
     a. Chercher dans LucivyHandleMap (cache mémoire)
     b. Si absent → chercher dans _lucivy_indexes via catalog API
     c. Si absent → erreur "No Lucivy index on table 'X'. Use CREATE_LUCIVY_INDEX first."
  3. Récupérer indexPath (depuis map ou registre)
  4. Ouvrir/réutiliser handle via LucivyHandleMap::getOrOpen(indexPath)
  5. Appeler lucivy_search(handle, queryJson, limit)
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

Le pattern SimpleTableFunc itère par morsels (chunks de DEFAULT_VECTOR_CAPACITY = 2048 lignes). Comme `lucivy_search` retourne tous les résultats d'un coup (JSON array), il est plus simple de :
1. Exécuter la recherche une seule fois dans `bindFunc`
2. Stocker les résultats parsés dans le BindData
3. Les distribuer par morsel dans `internalTableFunc`

Alternative : exécuter dans `initSharedState` ou dans le premier appel de `tableFunc` avec un flag `done`. Le `bindFunc` est le plus simple.

### Accès au registre depuis `bindFunc`

`bindFunc` reçoit un `ClientContext*`. Pour vérifier si l'index existe sans exécuter du Cypher, on utilise l'API catalogue interne :

```cpp
auto* catalog = catalog::Catalog::Get(*context);
auto* transaction = transaction::Transaction::Get(*context);
if (catalog->containsTable(transaction, "_lucivy_indexes")) {
    // Table existe → lire la propriété index_path du nœud avec table_name = tableName
    // Via StorageManager::getTable() + scan direct, ou via LucivyHandleMap déjà peuplé au load
}
```

En pratique, le plus simple : au `load()` de l'extension, lire le registre et peupler le `LucivyHandleMap` avec les `(tableName → indexPath)`. Ensuite `bindFunc` fait juste `LucivyHandleMap::get(tableName)`.

### FFI utilisée

```c
// Ouvrir l'index (si pas déjà en mémoire)
LucivyHandlePtr lucivy_open_index(const char* path);

// Rechercher
char* lucivy_search(LucivyHandlePtr handle, const char* query_json, uint32_t limit);

// Libérer le résultat
void lucivy_free_string(char* ptr);
```

Le JSON retourné par `lucivy_search` :
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

## 4. CREATE_LUCIVY_INDEX

### Syntaxe Cypher

```cypher
CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body']);
-- Avec options :
CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'], stemmer := 'english');
```

| Param | Type | Description |
|-------|------|-------------|
| `tableName` | STRING | Nom de la table de nœuds |
| `fields` | LIST(STRING) | Propriétés à indexer |
| `stemmer` | STRING (optionnel) | Stemmer ("english", "french"...), default "english" |

### Architecture : Standalone avec `rewriteFunc` + Internal

Comme le FTS extension, on sépare en deux fonctions :

1. **`CREATE_LUCIVY_INDEX`** (publique, standalone) : `rewriteFunc` génère le Cypher pour le registre + appelle la fonction interne
2. **`_CREATE_LUCIVY_INDEX`** (interne) : fait le travail lourd (scan nodes, FFI Lucivy)

#### rewriteFunc — Gestion du registre

```cpp
std::string createLucivyIndexQuery(ClientContext& context, const TableFuncBindData& bindData) {
    auto* bd = bindData.constPtrCast<CreateLucivyBindData>();
    context.setUseInternalCatalogEntry(true);
    std::string query;

    // 1. Créer la table de registre si elle n'existe pas
    auto* catalog = catalog::Catalog::Get(context);
    auto* txn = transaction::Transaction::Get(context);
    if (!catalog->containsTable(txn, "_lucivy_indexes")) {
        query += "CREATE NODE TABLE _lucivy_indexes ("
                 "table_name STRING PRIMARY KEY, "
                 "index_path STRING, "
                 "fields STRING, "
                 "stemmer STRING, "
                 "num_docs UINT64, "
                 "created_at STRING);";
    }

    // 2. Insérer les métadonnées
    query += stringFormat(
        "CREATE (i:_lucivy_indexes {{"
        "table_name: '{}', index_path: '{}', fields: '{}', "
        "stemmer: '{}', num_docs: 0, created_at: '{}'}});",
        bd->tableName, bd->indexPath, bd->fieldsJson,
        bd->stemmer, bd->createdAt);

    // 3. Appeler la fonction interne
    query += stringFormat(
        "CALL _CREATE_LUCIVY_INDEX('{}', {}, stemmer := '{}');",
        bd->tableName, bd->fieldsLiteral, bd->stemmer);

    // 4. Mettre à jour num_docs (sera calculé par la fonction interne,
    //    stocké dans une variable globale ou re-compté via lucivy_num_docs)
    query += stringFormat(
        "RETURN 'Lucivy index created on table {}' AS result;",
        bd->tableName);

    return query;
}
```

#### _CREATE_LUCIVY_INDEX — Scan + Indexation

Le challenge : scanner TOUS les nœuds de la table et les insérer dans l'index Lucivy via le FFI C. Deux options :

**Option A — VertexCompute (scalable, recommandé)**

Comme le FTS extension : `OnDiskGraph` + `VertexCompute` pour itérer par batches. Chaque batch appelle `lucivy_add_document()`.

```cpp
class LucivyIndexCompute final : public VertexCompute {
    LucivyHandlePtr handle_;
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
            lucivy_add_document(handle_, doc.dump().c_str());
        }
    }
};
```

**Option B — rewriteFunc + MATCH + collect (simple mais tout en mémoire)**

Problème : charge tous les docs en mémoire. Mauvais pour les grosses tables.

**On part sur Option A.**

#### Flux complet

```
bindFunc (CREATE_LUCIVY_INDEX — publique):
  1. Valider que la table existe et que les propriétés sont bien des STRING
  2. Vérifier qu'il n'y a pas déjà un index sur cette table (catalogue + registre)
  3. Construire indexPath, fieldsJson, stemmer
  4. Retourner CreateLucivyBindData

rewriteFunc:
  1. Créer _lucivy_indexes si nécessaire
  2. Insérer la ligne de registre
  3. Appeler _CREATE_LUCIVY_INDEX
  4. Retourner message

tableFunc (_CREATE_LUCIVY_INDEX — interne):
  1. Construire indexPath et schemaJson
  2. lucivy_create_index(indexPath, schemaJson) → handle
  3. Scanner les nœuds via VertexCompute → lucivy_add_document() par batch
  4. lucivy_commit(handle) + lucivy_reload_reader(handle)
  5. Stocker handle dans LucivyHandleMap
  6. Mettre à jour num_docs dans le registre :
     MATCH (i:_lucivy_indexes {table_name: '...'}) SET i.num_docs = lucivy_num_docs(handle)
  7. Retourner 0
```

---

## 5. DROP_LUCIVY_INDEX

### Syntaxe Cypher

```cypher
CALL DROP_LUCIVY_INDEX('doc');
```

### Architecture : Standalone avec `rewriteFunc` + Internal

Même pattern que CREATE : publique + interne.

#### rewriteFunc

```cpp
std::string dropLucivyIndexQuery(ClientContext& context, const TableFuncBindData& bindData) {
    auto* bd = bindData.constPtrCast<DropLucivyBindData>();
    context.setUseInternalCatalogEntry(true);
    std::string query;

    // 1. Appeler la fonction interne (close handle + rm dir)
    query += stringFormat("CALL _DROP_LUCIVY_INDEX('{}');", bd->tableName);

    // 2. Supprimer du registre
    query += stringFormat(
        "MATCH (i:_lucivy_indexes {{table_name: '{}'}}) DELETE i;",
        bd->tableName);

    query += stringFormat(
        "RETURN 'Lucivy index dropped on table {}' AS result;",
        bd->tableName);

    return query;
}
```

#### _DROP_LUCIVY_INDEX — Nettoyage

```
tableFunc:
  1. Récupérer indexPath depuis LucivyHandleMap ou le registre
  2. LucivyHandleMap::close(indexPath) → lucivy_close_index(handle)
  3. std::filesystem::remove_all(indexPath) → supprimer le dossier d'index
  4. Retourner 0
```

#### Flux `bindFunc`

```
bindFunc (DROP_LUCIVY_INDEX — publique):
  1. Vérifier que l'index existe (LucivyHandleMap ou registre)
  2. Si absent → erreur "No Lucivy index on table 'X'"
  3. Retourner DropLucivyBindData
```

---

## 6. Fichiers à créer/modifier

### Nouveaux fichiers

```
extension/lucivy_fts/
├── src/
│   ├── include/
│   │   ├── main/lucivy_fts_extension.h          (existe déjà)
│   │   └── function/
│   │       ├── lucivy_handle_map.h               ← NEW
│   │       ├── query_lucivy_index.h              ← NEW
│   │       ├── create_lucivy_index.h             ← NEW
│   │       └── drop_lucivy_index.h               ← NEW
│   ├── main/
│   │   ├── lucivy_fts_extension.cpp              (modifier)
│   │   └── CMakeLists.txt                         (existe déjà)
│   └── function/
│       ├── lucivy_handle_map.cpp                 ← NEW
│       ├── query_lucivy_index.cpp                ← NEW
│       ├── create_lucivy_index.cpp               ← NEW
│       ├── drop_lucivy_index.cpp                 ← NEW
│       └── CMakeLists.txt                         ← NEW
└── CMakeLists.txt                                 (modifier)
```

**Total : 8 fichiers à créer, 2 à modifier.**

### Détail des structs par fichier

#### `create_lucivy_index.h`

```cpp
struct CreateLucivyFunction {
    static constexpr const char* name = "CREATE_LUCIVY_INDEX";
    static function::function_set getFunctionSet();
};

struct InternalCreateLucivyFunction {
    static constexpr const char* name = "_CREATE_LUCIVY_INDEX";
    static function::function_set getFunctionSet();
};
```

#### `query_lucivy_index.h`

```cpp
struct QueryLucivyFunction {
    static constexpr const char* name = "QUERY_LUCIVY_INDEX";
    static function::function_set getFunctionSet();
};
```

#### `drop_lucivy_index.h`

```cpp
struct DropLucivyFunction {
    static constexpr const char* name = "DROP_LUCIVY_INDEX";
    static function::function_set getFunctionSet();
};

struct InternalDropLucivyFunction {
    static constexpr const char* name = "_DROP_LUCIVY_INDEX";
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
add_library(lucivy_fts_extension_function
    OBJECT
    lucivy_handle_map.cpp
    query_lucivy_index.cpp
    create_lucivy_index.cpp
    drop_lucivy_index.cpp)

set(LUCIVY_FTS_EXTENSION_OBJECT_FILES
    ${LUCIVY_FTS_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:lucivy_fts_extension_function>
    PARENT_SCOPE)
```

**`src/main/lucivy_fts_extension.cpp`** — Remplacer le stub :

```cpp
void LucivyFtsExtension::load(main::ClientContext* context) {
    auto& db = *context->getDatabase();

    // Table functions
    ExtensionUtils::addTableFunc<QueryLucivyFunction>(db);

    // Standalone table functions (DDL)
    ExtensionUtils::addStandaloneTableFunc<CreateLucivyFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalCreateLucivyFunction>(db);
    ExtensionUtils::addStandaloneTableFunc<DropLucivyFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalDropLucivyFunction>(db);

    // Charger le registre si il existe → peupler LucivyHandleMap
    LucivyHandleMap::instance().loadFromRegistry(context);
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

// FFI Lucivy
#include "lucivy_fts.h"

// JSON parsing
#include "nlohmann/json.hpp"

// Pour le binder
#include "binder/binder.h"
```

---

## 8. Plan d'implémentation

### Étape 1 : Infrastructure

1. Créer `lucivy_handle_map.h` / `.cpp` (avec `loadFromRegistry`)
2. Créer `src/function/CMakeLists.txt`
3. Modifier `CMakeLists.txt` racine (ajouter `add_subdirectory(src/function)`)

### Étape 2 : CREATE_LUCIVY_INDEX

1. Créer `create_lucivy_index.h` / `.cpp`
   - `CreateLucivyFunction` (publique, standalone, rewriteFunc)
   - `InternalCreateLucivyFunction` (interne, tableFunc avec VertexCompute)
   - Gestion du registre `_lucivy_indexes`
2. Ajouter au CMakeLists
3. Build test

### Étape 3 : QUERY_LUCIVY_INDEX

1. Créer `query_lucivy_index.h` / `.cpp`
   - `QueryLucivyFunction` (SimpleTableFunc)
   - Lecture du registre / handle map
   - Appel FFI `lucivy_search` + parse JSON nlohmann
2. Build test

### Étape 4 : DROP_LUCIVY_INDEX

1. Créer `drop_lucivy_index.h` / `.cpp`
   - `DropLucivyFunction` (publique, standalone, rewriteFunc)
   - `InternalDropLucivyFunction` (interne, close handle + rm dir)
   - Suppression de la ligne dans le registre
2. Build test

### Étape 5 : Extension load + wiring

1. Modifier `lucivy_fts_extension.cpp` (registrations + loadFromRegistry)
2. Build complet : `cmake --build build/release`

### Étape 6 : Tests end-to-end

Tests dans rag3db shell :
```cypher
-- Setup
CREATE NODE TABLE doc (id INT64, title STRING, body STRING, PRIMARY KEY(id));
CREATE (d:doc {id: 1, title: "Rust Guide", body: "Rust programming is great"});
CREATE (d:doc {id: 2, title: "C++ Guide", body: "C++ systems programming language"});

-- Create index
CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body']);

-- Verify registry
MATCH (i:_lucivy_indexes) RETURN i.*;

-- Query with highlights
CALL QUERY_LUCIVY_INDEX('doc',
    '{"type":"contains","field":"body","value":"programming","highlight":true}', 10)
RETURN node_id, score, highlights;

-- Query "c++" (separator validation)
CALL QUERY_LUCIVY_INDEX('doc',
    '{"type":"contains","field":"body","value":"c++","highlight":true}', 10)
RETURN node_id, score, highlights;

-- Drop
CALL DROP_LUCIVY_INDEX('doc');

-- Verify cleanup
MATCH (i:_lucivy_indexes) RETURN i.*;
```

---

## 9. Résumé des FFI C utilisées

| Fonction C | Utilisée par | Purpose |
|-----------|-------------|---------|
| `lucivy_create_index` | CREATE | Crée l'index sur disque |
| `lucivy_open_index` | QUERY (lazy open) | Ouvre un index existant |
| `lucivy_close_index` | DROP | Ferme et libère le handle |
| `lucivy_add_document` | CREATE | Ajoute un document |
| `lucivy_commit` | CREATE | Commit les writes |
| `lucivy_reload_reader` | CREATE (après commit) | Rend les docs visibles |
| `lucivy_search` | QUERY | Recherche → JSON résultats |
| `lucivy_free_string` | QUERY | Libère le JSON retourné |
| `lucivy_num_docs` | — | Diagnostic |
| `lucivy_get_schema` | — | Diagnostic |

---

## 10. Points d'attention

1. **Thread safety** : `LucivyHandleMap` est mutex-protégé. Les handles Lucivy eux-mêmes sont thread-safe (writer derrière Mutex<>, reader est lock-free).

2. **Lifecycle** : Les handles restent ouverts tant que l'extension est chargée. Fermeture propre via `closeAll()` si rag3db s'arrête.

3. **In-memory databases** : `getDatabasePath()` retourne `""` ou `":memory:"`. Il faudra gérer ce cas (index temporaire en mémoire ou erreur).

4. **nlohmann/json** : Disponible dans `third_party/nlohmann_json/`. Déjà utilisé par d'autres parties de rag3db (extension httpfs, parquet, etc.).

5. **Index inexistant** : Si QUERY est appelé avant CREATE, `lucivy_open_index` retourne null. Il faudra retourner une erreur claire.

6. **Table de registre `_lucivy_indexes`** :
   - Créée au premier `CREATE_LUCIVY_INDEX` (pas au load de l'extension)
   - `useInternalCatalogEntry(true)` pour que les tables internes ne soient pas visibles dans `SHOW TABLES`
   - Le PK `table_name` empêche de créer deux index sur la même table (erreur duplicate key)
   - Si la DB est supprimée/recréée, le registre est perdu mais les fichiers Lucivy aussi → cohérent

7. **Registre vs HandleMap** :
   - Le registre (`_lucivy_indexes`) est la source de vérité **persistante** (survit aux restarts)
   - Le `LucivyHandleMap` est le cache **mémoire** (handles ouverts, accès rapide)
   - `loadFromRegistry()` au load synchronise les deux
   - Les handles sont ouverts en **lazy** : on note le path dans le map, on ouvre le handle au premier QUERY

8. **Mise à jour incrémentale** (futur) : quand des nœuds sont ajoutés/supprimés de la table source après la création de l'index, il faudra un mécanisme de re-sync. Pour le v1, on suppose que l'index est créé une fois et n'est pas mis à jour incrémentalement. On peut toujours DROP + CREATE pour reconstruire.
