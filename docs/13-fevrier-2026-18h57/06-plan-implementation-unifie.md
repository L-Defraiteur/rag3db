# Plan d'implémentation unifié — Extension Tantivy dans rag3db

> Consolidation des docs 02, 03, 04, 05.
> Ce document remplace les docs précédents comme référence pour l'implémentation.

---

## Statut des docs précédents

| Doc | Contenu | Statut |
|-----|---------|--------|
| 01 | État des lieux | Toujours valide |
| 02 | Fonctions Cypher (registre, HandleMap, FFI C) | **Partiellement obsolète** — les patterns Cypher sont valides, mais le registre `_tantivy_indexes` et le `TantivyHandleMap` sont remplacés par l'infra native `storage::Index` (doc 04) |
| 03 | Hooks incrémentaux dans le core | **Entièrement obsolète** — doc 04 montre que les hooks existent déjà |
| 04 | Infra d'index existante (storage::Index) | **Valide** — source de vérité pour l'architecture |
| 05 | Migration cxx bridge | **TERMINÉ** (commits `127c15b`, `9daf73e`) |

---

## Architecture unifiée

### Avant (doc 02) vs Maintenant (doc 04 + 05)

| Aspect | Doc 02 (initial) | Maintenant |
|--------|------------------|------------|
| Persistance métadonnées | Table registre `_tantivy_indexes` | `IndexStorageInfo` natif (sérialisé avec la NodeTable) |
| Cache handles | `TantivyHandleMap` singleton | `TantivyIndex` instances dans `NodeTable::indexes` |
| FFI | 13 fonctions extern "C" + JSON | cxx bridge (9 structs, 15 fonctions typées) |
| Incrémental | Manuel (ADD_TANTIVY_DOC, SYNC) | Automatique (hooks `insert`/`delete_`/`checkpoint`) |
| Modification core | Possible (doc 03) | **Aucune** |

### Vue d'ensemble

```
Cypher                    C++ Extension              cxx bridge              Rust
──────                    ─────────────              ──────────              ────
CREATE_TANTIVY_INDEX ──→ TantivyIndex::create() ──→ create_index()      ──→ TantivyHandle::create()
                          scan nodes              ──→ add_document_texts() ──→ TantivyHandle writer
                                                  ──→ commit()            ──→ IndexWriter::commit()
                          nodeTable.addIndex()

QUERY_TANTIVY_INDEX  ──→ bindFunc: get TantivyIndex ──→ search_with_highlights() ──→ TopDocs + HighlightSink
                          tableFunc: emit results

DROP_TANTIVY_INDEX   ──→ TantivyIndex::drop()     ──→ (drop Box<TantivyHandle>) ──→ automatic cleanup
                          nodeTable.removeIndex()
                          rm -rf index dir

INSERT (:doc {...})  ──→ TantivyIndex::insert()   ──→ add_document_texts() ──→ buffered in writer
(automatique)             NodeTable::checkpoint()  ──→ commit()             ──→ flush segments
                                                   ──→ reload_reader()      ──→ visible aux searchers

DELETE (d:doc)       ──→ TantivyIndex::delete_()  ──→ delete_by_node_id()  ──→ buffered
(automatique)             NodeTable::checkpoint()  ──→ commit() + reload    ──→ visible
```

---

## 1. TantivyIndex — Classe principale

```cpp
// extension/tantivy_fts/src/include/index/tantivy_index.h

#include "storage/index/index.h"
#include "tantivy_fts/rust/src/bridge.rs.h"  // cxx-generated header

namespace rag3db {
namespace tantivy_fts_extension {

struct TantivyStorageInfo final : storage::IndexStorageInfo {
    std::string indexPath;    // chemin sur disque
    std::string schemaJson;   // schéma Tantivy (JSON, une fois à la création)
    std::string stemmer;      // "english", "" si pas de stemming
    common::offset_t numCheckpointedNodes;

    TantivyStorageInfo(std::string indexPath, std::string schemaJson,
        std::string stemmer, common::offset_t numCheckpointedNodes);

    std::shared_ptr<common::BufferWriter> serialize() const override;
    static std::unique_ptr<storage::IndexStorageInfo> deserialize(
        std::unique_ptr<common::BufferReader> reader);
};

class TantivyIndex final : public storage::Index {
public:
    // Construction (après create_index ou open_index)
    TantivyIndex(storage::IndexInfo indexInfo,
        std::unique_ptr<TantivyStorageInfo> storageInfo,
        rust::Box<TantivyHandle> handle);

    // Registration dans l'extension
    static storage::IndexType getIndexType();

    // Chargement au restart (appelé par IndexHolder::load)
    static std::unique_ptr<storage::Index> load(main::ClientContext* context,
        storage::StorageManager* storageManager, storage::IndexInfo indexInfo,
        std::span<uint8_t> storageInfoBuffer);

    // ── Hooks CRUD (appelés automatiquement par NodeTable) ──

    std::unique_ptr<InsertState> initInsertState(main::ClientContext* context,
        visible_func isVisible) override;

    // Ajoute les documents au writer Tantivy (buffered, pas encore visible)
    void insert(transaction::Transaction* transaction,
        const common::ValueVector& nodeIDVector,
        const std::vector<common::ValueVector*>& propertyVectors,
        InsertState& insertState) override;

    std::unique_ptr<DeleteState> initDeleteState(
        const transaction::Transaction* transaction,
        storage::MemoryManager* mm, visible_func isVisible) override;

    // Supprime par node_id (buffered)
    void delete_(transaction::Transaction* transaction,
        const common::ValueVector& nodeIDVector,
        DeleteState& deleteState) override;

    // Mode immédiat (comme FTS, pas différé comme HNSW)
    bool needCommitInsert() const override { return false; }

    // ── Persistence ──

    // Flush segments Tantivy → disque, rend les changements visibles
    void checkpointInMemory() override;

    // Sérialise TantivyStorageInfo
    void checkpoint(main::ClientContext* context,
        storage::PageAllocator& pageAllocator) override;

    // Rollback les changements non-committed
    void rollbackCheckpoint() override;

    // Rattrapage au restart (indexe les nœuds ajoutés depuis le dernier checkpoint)
    void finalize(main::ClientContext* context) override;

    // ── Accesseur pour QUERY ──

    const TantivyHandle& getHandle() const { return *handle_; }

private:
    rust::Box<TantivyHandle> handle_;
};

} // namespace tantivy_fts_extension
} // namespace rag3db
```

### Détails des hooks

**`insert()`** — appelé par `NodeTable::insert()` pour chaque batch de nœuds insérés :

```cpp
void TantivyIndex::insert(Transaction* transaction,
    const ValueVector& nodeIDVector,
    const std::vector<ValueVector*>& propertyVectors,
    InsertState& insertState) {
    for (auto i = 0u; i < nodeIDVector.state->getSelSize(); i++) {
        auto pos = nodeIDVector.state->getSelVector()[i];
        auto nodeID = nodeIDVector.getValue<nodeID_t>(pos);
        auto nodeOffset = nodeID.offset;

        // Construire les DocFieldText pour chaque propriété texte
        rust::Vec<DocFieldText> fields;
        for (size_t f = 0; f < propertyVectors.size(); f++) {
            if (propertyVectors[f]->isNull(pos)) continue;
            auto text = propertyVectors[f]->getValue<ku_string_t>(pos).getAsString();
            fields.push_back(DocFieldText{
                static_cast<uint32_t>(fieldIds_[f]),  // field_id du schema Tantivy
                rust::String(text)
            });
        }
        add_document_texts(*handle_, nodeOffset, fields);
    }
    // Note: pas de commit ici — sera fait dans checkpointInMemory()
    auto& si = storageInfo->cast<TantivyStorageInfo>();
    si.numCheckpointedNodes = nodeIDVector.getValue<nodeID_t>(
        nodeIDVector.state->getSelVector()[0]).offset + 1;
}
```

**`delete_()`** — appelé par `NodeTable::delete_()` :

```cpp
void TantivyIndex::delete_(Transaction* transaction,
    const ValueVector& nodeIDVector, DeleteState& deleteState) {
    for (auto i = 0u; i < nodeIDVector.state->getSelSize(); i++) {
        auto pos = nodeIDVector.state->getSelVector()[i];
        auto nodeOffset = nodeIDVector.getValue<nodeID_t>(pos).offset;
        delete_by_node_id(*handle_, nodeOffset);
    }
}
```

**`checkpointInMemory()`** — flush Tantivy :

```cpp
void TantivyIndex::checkpointInMemory() {
    commit(*handle_);
    reload_reader(*handle_);
}
```

**`finalize()`** — rattrapage au restart :

```cpp
void TantivyIndex::finalize(ClientContext* context) {
    auto& si = storageInfo->cast<TantivyStorageInfo>();
    auto* storageManager = StorageManager::Get(*context);
    auto& nodeTable = storageManager->getTable(indexInfo.tableID)->cast<NodeTable>();
    auto numTotalRows = nodeTable.getNumTotalRows(&DUMMY_CHECKPOINT_TRANSACTION);
    if (numTotalRows == si.numCheckpointedNodes) {
        return;  // Déjà à jour
    }
    // Scanner les nœuds manquants et les indexer
    // (pattern identique à FTSIndex::finalize et HNSW::finalize)
    for (auto offset = si.numCheckpointedNodes; offset < numTotalRows; offset++) {
        // Lire les propriétés du nœud, construire les DocFieldText, add_document_texts
    }
    commit(*handle_);
    reload_reader(*handle_);
    si.numCheckpointedNodes = numTotalRows;
}
```

### TantivyStorageInfo — Sérialisation

Pattern identique à FTS/HNSW :

```cpp
std::shared_ptr<BufferWriter> TantivyStorageInfo::serialize() const {
    auto writer = std::make_shared<BufferWriter>();
    auto ser = Serializer(writer);
    ser.writeString(indexPath);
    ser.writeString(schemaJson);
    ser.writeString(stemmer);
    ser.write<offset_t>(numCheckpointedNodes);
    return writer;
}

std::unique_ptr<IndexStorageInfo> TantivyStorageInfo::deserialize(
    std::unique_ptr<BufferReader> reader) {
    Deserializer deSer{std::move(reader)};
    std::string indexPath, schemaJson, stemmer;
    offset_t numCheckpointedNodes;
    deSer.deserializeValue(indexPath);
    deSer.deserializeValue(schemaJson);
    deSer.deserializeValue(stemmer);
    deSer.deserializeValue(numCheckpointedNodes);
    return std::make_unique<TantivyStorageInfo>(
        std::move(indexPath), std::move(schemaJson),
        std::move(stemmer), numCheckpointedNodes);
}
```

### IndexType Registration

```cpp
IndexType TantivyIndex::getIndexType() {
    static const IndexType TANTIVY_INDEX_TYPE{
        "TANTIVY",
        IndexConstraintType::SECONDARY_NON_UNIQUE,
        IndexDefinitionType::EXTENSION,
        TantivyIndex::load
    };
    return TANTIVY_INDEX_TYPE;
}
```

### Chargement au restart

```cpp
std::unique_ptr<Index> TantivyIndex::load(ClientContext* context, StorageManager*,
    IndexInfo indexInfo, std::span<uint8_t> storageInfoBuffer) {
    auto reader = std::make_unique<BufferReader>(
        storageInfoBuffer.data(), storageInfoBuffer.size());
    auto storageInfo = TantivyStorageInfo::deserialize(std::move(reader));
    auto& si = storageInfo->cast<TantivyStorageInfo>();
    // Rouvrir l'index Tantivy sur disque
    auto handle = open_index(si.indexPath);
    return std::make_unique<TantivyIndex>(
        std::move(indexInfo), std::move(storageInfo), std::move(handle));
}
```

---

## 2. CREATE_TANTIVY_INDEX

### Syntaxe Cypher

```cypher
CALL CREATE_TANTIVY_INDEX('doc', ['title', 'body']);
CALL CREATE_TANTIVY_INDEX('doc', ['title', 'body'], stemmer := 'english');
```

### Architecture : Standalone + rewriteFunc + Internal

Comme HNSW/FTS : fonction publique avec `rewriteFunc` qui appelle la fonction interne.

**Publique** (`CREATE_TANTIVY_INDEX`) — standalone :
- `bindFunc`: valide la table et les colonnes, construit les params
- `rewriteFunc`: génère le Cypher pour appeler `_CREATE_TANTIVY_INDEX`

**Interne** (`_CREATE_TANTIVY_INDEX`) :
- `tableFunc`: scan les nœuds, crée l'index, appelle le cxx bridge

### rewriteFunc

```cpp
static std::string rewriteFunc(ClientContext& context, const TableFuncBindData& bindData) {
    context.setUseInternalCatalogEntry(true);
    auto* bd = bindData.constPtrCast<CreateTantivyBindData>();
    // Pas de tables internes à créer (contrairement à FTS).
    // Juste appeler la fonction interne.
    return stringFormat(
        "CALL _CREATE_TANTIVY_INDEX('{}', {}, stemmer := '{}');"
        "RETURN 'Tantivy index created on table {}' AS result;",
        bd->tableName, bd->fieldsLiteral, bd->stemmer, bd->tableName);
}
```

### _CREATE_TANTIVY_INDEX — tableFunc

```
1. Construire indexPath = "{dbPath}/tantivy_indexes/{tableName}/"
2. Construire schemaJson depuis les colonnes de la table
   - STRING → type "text" (tri-field: stemmed + raw + ngram)
   - INT64  → type "i64" (fast field)
   - etc. (mapping complet en section 4)
3. create_index(indexPath, schemaJson) → handle
4. get_field_ids(handle) → mapping nom→field_id
5. Scanner TOUS les nœuds de la table :
   - Pour chaque nœud : add_document_texts(handle, node_offset, fields)
6. commit(handle) + reload_reader(handle)
7. Créer IndexCatalogEntry dans le catalogue
8. Créer TantivyIndex avec le handle et le storageInfo
9. nodeTable->addIndex(std::move(tantivyIndex))
10. transaction->setForceCheckpoint()
```

### Scan des nœuds — approche simple (pas VertexCompute)

VertexCompute (pattern FTS/HNSW) est optimal pour le parallélisme mais complexe. Pour v1, on utilise un scan séquentiel direct via NodeTable :

```cpp
auto transaction = Transaction::Get(*context);
auto numRows = nodeTable.getNumTotalRows(transaction);
// Construire un scan state pour lire les colonnes indexées
auto scanState = nodeTable.constructScanState(...columnIDs...);
for (offset_t offset = 0; offset < numRows; offset++) {
    // Lire les propriétés du nœud
    nodeTable.initScanState(transaction, scanState, tableID, offset);
    nodeTable.lookup(transaction, scanState);
    // Extraire les valeurs, construire DocFieldText, add_document_texts
}
```

Si les performances sont insuffisantes, on migrera vers VertexCompute.

---

## 3. QUERY_TANTIVY_INDEX

### Syntaxe Cypher

```cypher
CALL QUERY_TANTIVY_INDEX('doc',
    '{"type":"contains","field":"body","value":"c++"}', 10)
RETURN node_id, score, highlights;
```

### Architecture : SimpleTableFunc

C'est la plus simple des trois fonctions. Pas de standalone, pas de rewriteFunc.

### Output columns

| Column | Type | Description |
|--------|------|-------------|
| `node_id` | UINT64 | Offset du nœud dans la table |
| `score` | DOUBLE | Score BM25 |
| `highlights` | STRING | JSON : `{"body":[[5,16],[20,25]]}` ou `{}` |

### bindFunc

```cpp
static std::unique_ptr<TableFuncBindData> bindFunc(
    ClientContext* context, const TableFuncBindInput* input) {
    auto tableName = input->getLiteralVal<std::string>(0);
    auto queryJson = input->getLiteralVal<std::string>(1);
    auto limit = input->getNumParams() > 2 ?
        input->getLiteralVal<int64_t>(2) : 10;

    // Trouver la table et l'index
    auto* catalog = catalog::Catalog::Get(*context);
    auto* transaction = transaction::Transaction::Get(*context);
    auto* tableEntry = catalog->getTableCatalogEntry(transaction, tableName);
    auto* storageManager = StorageManager::Get(*context);
    auto& nodeTable = storageManager->getTable(tableEntry->getTableID())
        ->cast<NodeTable>();

    // Récupérer le TantivyIndex depuis la NodeTable
    auto indexOpt = nodeTable.getIndex("tantivy");  // ou par type
    if (!indexOpt.has_value()) {
        throw BinderException("No Tantivy index on table '" + tableName + "'");
    }
    auto& tantivyIndex = indexOpt.value()->cast<TantivyIndex>();

    // Exécuter la recherche dans bindFunc (résultats en mémoire)
    auto results = search_with_highlights(
        tantivyIndex.getHandle(), queryJson, static_cast<uint32_t>(limit));

    // Stocker les résultats dans le BindData
    // ... (voir ci-dessous)
}
```

### internalTableFunc

```cpp
static offset_t internalTableFunc(
    const TableFuncMorsel& morsel, const TableFuncInput& input, DataChunk& output) {
    auto* bd = input.bindData->constPtrCast<QueryTantivyBindData>();
    auto count = std::min(morsel.endOffset - morsel.startOffset,
        static_cast<offset_t>(bd->results.size() - morsel.startOffset));
    for (offset_t i = 0; i < count; i++) {
        auto& result = bd->results[morsel.startOffset + i];
        output.getValueVectorMutable(0).setValue(i, result.node_id);   // UINT64
        output.getValueVectorMutable(1).setValue(i, (double)result.score); // DOUBLE
        // Convertir highlights en JSON string
        output.getValueVectorMutable(2).setValue(i, highlightsToJson(result.highlights));
    }
    return count;
}
```

### Conversion highlights → JSON

Les highlights viennent du cxx bridge comme `Vec<FieldHighlights>`. On les convertit en JSON string pour l'output :

```cpp
static std::string highlightsToJson(const rust::Vec<FieldHighlights>& highlights) {
    if (highlights.empty()) return "{}";
    std::string json = "{";
    for (size_t i = 0; i < highlights.size(); i++) {
        if (i > 0) json += ",";
        json += "\"" + std::string(highlights[i].field_name) + "\":[";
        for (size_t j = 0; j < highlights[i].ranges.size(); j++) {
            if (j > 0) json += ",";
            json += "[" + std::to_string(highlights[i].ranges[j].start)
                + "," + std::to_string(highlights[i].ranges[j].end) + "]";
        }
        json += "]";
    }
    json += "}";
    return json;
}
```

---

## 4. DROP_TANTIVY_INDEX

### Syntaxe Cypher

```cypher
CALL DROP_TANTIVY_INDEX('doc');
```

### Architecture : Standalone + rewriteFunc + Internal

Même pattern que CREATE.

### _DROP_TANTIVY_INDEX — tableFunc

```
1. Trouver l'index sur la NodeTable
2. nodeTable.removeIndex("tantivy")  → drop du Box<TantivyHandle> (automatique via cxx)
3. catalog->dropIndex(transaction, tableID, indexName)
4. std::filesystem::remove_all(indexPath)  → supprimer les fichiers
5. Retourner message de confirmation
```

---

## 5. Extension Load + Wiring

### tantivy_fts_extension.cpp

```cpp
void TantivyFtsExtension::load(ClientContext* context) {
    auto& db = *context->getDatabase();

    // Table functions
    ExtensionUtils::addTableFunc<QueryTantivyFunction>(db);

    // Standalone (DDL-like)
    ExtensionUtils::addStandaloneTableFunc<CreateTantivyFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalCreateTantivyFunction>(db);
    ExtensionUtils::addStandaloneTableFunc<DropTantivyFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalDropTantivyFunction>(db);

    // Register index type (pour load/finalize au restart)
    ExtensionUtils::registerIndexType(db, TantivyIndex::getIndexType());
}
```

### Séquence au restart

1. `NodeTable::deserialize()` → crée les `IndexHolder` non-chargés (storageInfoBuffer)
2. Extension `load()` → appelle `registerIndexType("TANTIVY", loadFunc)`
3. Quand l'index est accédé → `IndexHolder::load()` → appelle `TantivyIndex::load()` → `open_index()`
4. `IndexHolder::finalize()` → `TantivyIndex::finalize()` → rattrape les nœuds manquants

---

## 6. Mapping types rag3db → Tantivy

| Type rag3db | Type Tantivy (schema JSON) | Options | Usage |
|-------------|---------------------------|---------|-------|
| STRING (champ FTS) | `"text"` | `stored: true` | Tri-field auto (stemmed + raw + ngram) |
| STRING (champ filtre) | `"string"` | `stored: true, indexed: true` | Filtrage valeur exacte |
| INT64 | `"i64"` | `stored: true, fast: true` | Filtrage range |
| UINT64 | `"u64"` | `stored: true, fast: true` | Filtrage range |
| DOUBLE | `"f64"` | `stored: true, fast: true` | Filtrage range |

Pour v1, seuls les champs texte FTS sont supportés. Les filter fields (string, i64, u64, f64) sont pour v2.

---

## 7. Build — CMakeLists.txt

### Headers cxx

Le `cargo build` de tantivy_fts produit maintenant :
- `libtantivy_fts.a` — lib statique Rust (comme avant)
- `target/cxxbridge/tantivy-fts/src/bridge.rs.h` — header C++ généré par cxx
- `target/cxxbridge/rust/cxx.h` — runtime cxx (types `rust::String`, `rust::Vec`, etc.)

### Modifications CMakeLists.txt

```cmake
# Ajouter les headers cxx (remplace l'ancien include cbindgen)
set(CXX_BRIDGE_DIR ${RUST_WORKSPACE_DIR}/target/cxxbridge)
include_directories(
    ${CXX_BRIDGE_DIR}/tantivy-fts/src/  # bridge.rs.h
    ${CXX_BRIDGE_DIR}/rust/             # cxx.h runtime
)

# Ajouter les sources C++ de l'extension
add_subdirectory(src/index)
add_subdirectory(src/function)
```

### Source supplémentaire : cxx glue

cxx génère aussi un fichier source C++ qu'il faut compiler :
```cmake
# Le fichier glue cxx (généré par cargo build)
set(CXX_GLUE_SRC ${RUST_WORKSPACE_DIR}/target/cxxbridge/tantivy-fts/src/bridge.rs.cc)
# L'ajouter aux sources de l'extension
```

Note: le glue file est déjà compilé dans `libtantivy_fts.a` via `cxx-build` dans `build.rs`. Vérifier s'il faut le recompiler côté CMake ou s'il est linké automatiquement.

---

## 8. Fichiers à créer / modifier

### Nouveaux fichiers (dans `extension/tantivy_fts/`)

```
src/include/index/
    tantivy_index.h                 ← TantivyIndex + TantivyStorageInfo
src/index/
    tantivy_index.cpp               ← Implémentation des hooks + load
    CMakeLists.txt                  ← Build
src/include/function/
    create_tantivy_index.h          ← CreateTantivyFunction + InternalCreateTantivyFunction
    query_tantivy_index.h           ← QueryTantivyFunction
    drop_tantivy_index.h            ← DropTantivyFunction + InternalDropTantivyFunction
src/function/
    create_tantivy_index.cpp        ← Scan + indexation + registration
    query_tantivy_index.cpp         ← SimpleTableFunc + search_with_highlights
    drop_tantivy_index.cpp          ← Cleanup + removeIndex
    CMakeLists.txt                  ← Build
```

### Fichiers modifiés

```
src/main/tantivy_fts_extension.cpp  ← Remplacer stub par registration complète
CMakeLists.txt                      ← Ajouter subdirectories + headers cxx
```

### Total : 9 fichiers à créer, 2 à modifier

---

## 9. Plan d'implémentation (ordre)

### Étape 1 : Build infrastructure

1. Modifier `CMakeLists.txt` : ajouter headers cxx, subdirectories
2. Créer `src/index/CMakeLists.txt` et `src/function/CMakeLists.txt`
3. Vérifier que le build compile (sans code métier)

### Étape 2 : TantivyIndex + StorageInfo

1. Créer `tantivy_index.h` / `.cpp`
2. Implémenter : constructeur, `getIndexType()`, `load()`, `serialize()`/`deserialize()`
3. Stubs pour `insert()`, `delete_()`, `checkpoint()`, `finalize()`
4. Build test

### Étape 3 : QUERY_TANTIVY_INDEX

Commencer par QUERY (priorité #1) pour pouvoir tester rapidement :
1. Créer `query_tantivy_index.h` / `.cpp`
2. SimpleTableFunc : `bindFunc` + `internalTableFunc`
3. Build test
4. Test manuel : créer un index Tantivy à la main (via test Rust), puis QUERY depuis rag3db

### Étape 4 : CREATE_TANTIVY_INDEX

1. Créer `create_tantivy_index.h` / `.cpp`
2. Standalone + rewriteFunc + Internal
3. Scan séquentiel des nœuds + indexation
4. Registration de l'index sur la NodeTable
5. Build test

### Étape 5 : DROP_TANTIVY_INDEX

1. Créer `drop_tantivy_index.h` / `.cpp`
2. Standalone + rewriteFunc + Internal
3. Cleanup fichiers + catalogue
4. Build test

### Étape 6 : Extension wiring

1. Modifier `tantivy_fts_extension.cpp` : registrations + registerIndexType
2. Build complet : `cmake --build build/release`

### Étape 7 : Hooks incrémentaux

1. Implémenter `insert()` et `delete_()` dans TantivyIndex
2. Implémenter `checkpointInMemory()` et `checkpoint()`
3. Implémenter `finalize()` pour le rattrapage au restart
4. Tests : INSERT → QUERY → vérifier que les nouveaux docs sont visibles

### Étape 8 : Tests E2E

```cypher
-- Setup
CREATE NODE TABLE doc (id INT64, title STRING, body STRING, PRIMARY KEY(id));
CREATE (d:doc {id: 1, title: "Rust Guide", body: "Rust programming is great"});
CREATE (d:doc {id: 2, title: "C++ Guide", body: "C++ systems programming language"});

-- Create index
CALL CREATE_TANTIVY_INDEX('doc', ['title', 'body']);

-- Query contains + highlights
CALL QUERY_TANTIVY_INDEX('doc',
    '{"type":"contains","field":"body","value":"programming"}', 10)
RETURN node_id, score, highlights;

-- Query "c++" (separator validation)
CALL QUERY_TANTIVY_INDEX('doc',
    '{"type":"contains","field":"body","value":"c++"}', 10)
RETURN node_id, score, highlights;

-- Incremental insert
CREATE (d:doc {id: 3, title: "Python Guide", body: "Python scripting language"});

-- Verify new doc is searchable (after checkpoint)
CALL QUERY_TANTIVY_INDEX('doc',
    '{"type":"contains","field":"body","value":"python"}', 10)
RETURN node_id, score, highlights;

-- Drop
CALL DROP_TANTIVY_INDEX('doc');
```

---

## 10. Décisions de design

### Mode immédiat vs différé

On utilise le mode **immédiat** (`needCommitInsert() = false`), comme FTS :
- `insert()` ajoute les docs au writer Tantivy (buffered dans le heap 50MB)
- `checkpointInMemory()` flush (commit + reload_reader)
- Plus simple que le mode différé (pas de re-scan au commit)

### Visibilité des changements

Les documents insérés ne sont visibles aux recherches qu'après `checkpointInMemory()`, qui est appelé lors du checkpoint de la NodeTable. En pratique, rag3db force un checkpoint après la plupart des opérations DDL et DML en auto-commit.

**Limitation v1** : dans une transaction multi-statements, les inserts ne sont pas visibles à QUERY tant que la transaction n'est pas committée et checkpointée.

### Pas de table registre

Contrairement au plan initial (doc 02), on n'utilise PAS de table `_tantivy_indexes`. Les métadonnées sont dans `TantivyStorageInfo`, sérialisées avec la NodeTable. Avantages :
- Zéro table interne à gérer
- Atomic avec le checkpoint de la NodeTable
- Pattern identique à FTS et HNSW

### Handle ownership

Le `Box<TantivyHandle>` vit dans `TantivyIndex`, qui vit dans `IndexHolder` de `NodeTable::indexes`. Drop automatique quand l'index est supprimé ou la DB fermée.

### Pas de HandleMap

Chaque `TantivyIndex` possède son propre handle. Pas de singleton global. Thread safety : le writer est derrière un `Mutex<IndexWriter>` côté Rust, le reader est lock-free.

### Recherche dans bindFunc

Comme FTS, on exécute la recherche dans `bindFunc` (une seule fois) et on stocke les résultats dans le BindData. Le `internalTableFunc` les distribue par morsel.

### Convention de chemin

```
{databasePath}/tantivy_indexes/{tableName}/
```

Stocké dans `TantivyStorageInfo::indexPath`. Créé par CREATE, supprimé par DROP.
