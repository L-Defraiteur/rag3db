# Découverte : l'infra d'index incrémental existe déjà dans rag3db

> Exploration du 13 février 2026.
> Conclusion : pas besoin de modifier le core. On implémente `Index` comme FTS et HNSW.

---

## Résumé

On pensait devoir ajouter des hooks `onInsert`/`onDelete`/`onCommit` dans le core rag3db (voir 03). En fait, **tout existe déjà**. La classe `storage::Index` définit les hooks, et `NodeTable` les appelle automatiquement sur INSERT/UPDATE/DELETE.

---

## 1. Classe `Index` — Interface complète

**Fichier** : `src/include/storage/index/index.h`

```cpp
class Index {
    // --- Init states (appelés avant les opérations) ---
    virtual std::unique_ptr<InsertState> initInsertState(ClientContext*, visible_func) = 0;
    virtual std::unique_ptr<UpdateState> initUpdateState(ClientContext*, column_id_t, visible_func);
    virtual std::unique_ptr<DeleteState> initDeleteState(Transaction*, MemoryManager*, visible_func) = 0;

    // --- Opérations CRUD ---
    virtual void insert(Transaction*, const ValueVector& nodeIDs,
                        const std::vector<ValueVector*>& propertyVectors, InsertState&);
    virtual void update(Transaction*, const ValueVector& nodeIDs,
                        ValueVector& newValues, UpdateState&);
    virtual void delete_(Transaction*, const ValueVector& nodeIDs, DeleteState&) = 0;

    // --- Hooks transactionnels ---
    virtual bool needCommitInsert() const { return false; }
    virtual void commitInsert(Transaction*, const ValueVector& nodeIDs,
                              const std::vector<ValueVector*>& dataVectors, InsertState&);

    // --- Persistence ---
    virtual void checkpointInMemory();
    virtual void checkpoint(ClientContext*, PageAllocator&);
    virtual void rollbackCheckpoint();
    virtual void finalize(ClientContext*);  // catch-up au restart
};
```

### Deux modes d'insertion

| Mode | `needCommitInsert()` | Quand l'insert effectif a lieu | Utilisé par |
|------|---------------------|-------------------------------|-------------|
| **Immédiat** | `false` | Dans `insert()` pendant la transaction | FTS, PrimaryKey |
| **Différé** | `true` | Dans `commitInsert()` au commit | HNSW |

Pour Tantivy, le mode **différé** est idéal : on accumule les `tantivy_add_document()` pendant la transaction (buffer 50MB), puis `tantivy_commit()` au commit.

---

## 2. NodeTable — Appels automatiques aux index

**Fichier** : `src/storage/store/node_table.cpp`

### INSERT (ligne ~419-447)

```cpp
void NodeTable::insert(Transaction* transaction, TableInsertState& insertState) {
    // ... insert data dans la table ...

    // Notifie TOUS les index enregistrés
    for (auto i = 0u; i < indexes.size(); i++) {
        auto index = indexes[i].getIndex();
        std::vector<ValueVector*> indexedPropertyVectors;
        for (const auto columnID : index->getIndexInfo().columnIDs) {
            indexedPropertyVectors.push_back(insertState.propertyVectors[columnID]);
        }
        index->insert(transaction, nodeInsertState.nodeIDVector,
            indexedPropertyVectors, *nodeInsertState.indexInsertStates[i]);
    }
}
```

**Important** : seules les `columnIDs` de l'index sont passées → on reçoit uniquement les propriétés qu'on a déclaré vouloir indexer.

### UPDATE (ligne ~466-508)

```cpp
void NodeTable::update(Transaction* transaction, TableUpdateState& updateState) {
    // ... update data ...

    for (auto i = 0u; i < indexes.size(); i++) {
        auto index = indexes[i].getIndex();
        if (!nodeUpdateState.needToUpdateIndex(i)) {
            continue; // Skip si l'index n'est pas sur cette colonne
        }
        index->update(transaction, nodeUpdateState.nodeIDVector,
            nodeUpdateState.propertyVector, *nodeUpdateState.indexUpdateState[i]);
    }
}
```

### DELETE (ligne ~510-546)

```cpp
bool NodeTable::delete_(Transaction* transaction, TableDeleteState& deleteState) {
    // Notifie les index AVANT de supprimer
    for (auto& index : indexes) {
        auto indexDeleteState = index.getIndex()->initDeleteState(...);
        index.getIndex()->delete_(transaction, nodeDeleteState.nodeIDVector, *indexDeleteState);
    }
    // ... puis supprime de la table ...
}
```

### COMMIT (ligne ~625-640)

```cpp
void NodeTable::commit(ClientContext* context, TableCatalogEntry* tableEntry,
    LocalTable* localTable) {
    // 1. Append local storage → persistent
    nodeGroups->append(transaction, columnIDsToCommit, localNodeTable.getNodeGroups());

    // 2. Pour les index en mode différé
    for (auto& index : indexes) {
        if (!index.needCommitInsert()) {
            continue;  // FTS skip ici (mode immédiat)
        }
        // HNSW (et notre TantivyIndex) passe ici
        UncommittedIndexInserter indexInserter{startNodeOffset, this, index.getIndex(),
            getVisibleFunc(transaction)};
        scanIndexColumns(context, indexInserter, localNodeTable.getNodeGroups());
    }
}
```

### COPY (bulk insert)

COPY passe par le même chemin que INSERT → les index sont notifiés automatiquement. Pas de chemin séparé à gérer.

---

## 3. Les 3 types d'index existants

### PrimaryKeyIndex (HASH) — Builtin

**Fichiers** :
- `src/include/storage/index/hash_index.h` — déclaration
- `src/storage/index/hash_index.cpp` — implémentation

| Propriété | Valeur |
|-----------|--------|
| Type name | `"HASH"` |
| Constraint | `PRIMARY` |
| Definition | `BUILTIN` |
| Incrémental | Oui, complet (insert/delete/lookup) |
| `needCommitInsert()` | `false` |

Toujours présent sur chaque node table (obligatoire, sur le PK).

### FTS Index — Extension

**Fichiers** :
- `extension/fts/src/include/index/fts_index.h` — déclaration
- `extension/fts/src/index/fts_index.cpp` — implémentation
- `extension/fts/src/function/create_fts_index.cpp` — création
- `extension/fts/src/function/query_fts_index.cpp` — requêtage
- `extension/fts/src/function/drop_fts_index.cpp` — suppression

| Propriété | Valeur |
|-----------|--------|
| Type name | `"FTS"` |
| Constraint | `SECONDARY_NON_UNIQUE` |
| Definition | `EXTENSION` |
| Incrémental | Oui, complet |
| `needCommitInsert()` | `false` (insert immédiat) |
| Stockage | Tables internes (docs, terms, appearsIn) |

### HNSW Vector Index — Extension

**Fichiers** :
- `extension/vector/src/include/index/hnsw_index.h` — déclaration (InMem + OnDisk)
- `extension/vector/src/index/hnsw_index.cpp` — implémentation
- `extension/vector/src/include/index/hnsw_config.h` — configuration
- `extension/vector/src/function/create_hnsw_index.cpp` — création
- `extension/vector/src/function/query_hnsw_index.cpp` — requêtage
- `extension/vector/src/function/drop_hnsw_index.cpp` — suppression
- `extension/vector/src/main/vector_extension.cpp` — registration

| Propriété | Valeur |
|-----------|--------|
| Type name | `"HNSW"` |
| Constraint | `SECONDARY_NON_UNIQUE` |
| Definition | `EXTENSION` |
| Incrémental | Oui, différé |
| `needCommitInsert()` | `true` (commit-time insert) |
| Stockage | 2 rel tables internes (upper graph, lower graph) |

**Deux modes** :
- `InMemHNSWIndex` — construit en batch pendant CREATE (en mémoire)
- `OnDiskHNSWIndex` — supporte les inserts incrémentaux après création

**Mécanisme de rattrapage** (`finalize()`) :
```cpp
void OnDiskHNSWIndex::finalize(ClientContext* context) {
    const auto numTotalRows = nodeTable.getNumTotalRows(...);
    if (numTotalRows == hnswStorageInfo.numCheckpointedNodes) {
        return;  // Déjà à jour
    }
    // Indexer les nœuds manquants
    for (auto offset = numCheckpointedNodes; offset < numTotalRows; offset++) {
        insertInternal(transaction, offset, embedding, *insertState);
    }
    hnswStorageInfo.numCheckpointedNodes = numTotalRows;
}
```

Stocke un `numCheckpointedNodes` pour savoir où il en est → rattrapage au restart.

---

## 4. Registration d'un type d'index

**Fichier** : `src/include/extension/extension.h`

```cpp
ExtensionUtils::registerIndexType(db, MyIndex::getIndexType());
```

**Fichier** : `src/include/storage/index/index.h`

```cpp
struct IndexType {
    std::string typeName;              // "HNSW", "FTS", notre "TANTIVY"
    IndexConstraintType constraintType; // SECONDARY_NON_UNIQUE
    IndexDefinitionType definitionType; // EXTENSION
    create_index_func_t createFunc;     // appelé par CREATE INDEX
    load_index_func_t loadFunc;         // appelé au restart (désérialisation)
};
```

Exemple HNSW :
```cpp
// extension/vector/src/index/hnsw_index.cpp
IndexType OnDiskHNSWIndex::getIndexType() {
    return IndexType{
        "HNSW",
        IndexConstraintType::SECONDARY_NON_UNIQUE,
        IndexDefinitionType::EXTENSION,
        OnDiskHNSWIndex::create,  // création
        OnDiskHNSWIndex::load     // chargement au restart
    };
}
```

---

## 5. Sérialisation / Désérialisation

**Fichier** : `src/storage/store/node_table.cpp` (ligne ~815-853)

### Sérialisation (checkpoint)

```cpp
void NodeTable::serialize(Serializer& serializer) const {
    nodeGroups->serialize(serializer);
    serializer.write<uint64_t>(indexes.size());
    for (auto& index : indexes) {
        index.serialize(serializer);  // IndexInfo + StorageInfo binaire
    }
}
```

### Désérialisation (restart)

```cpp
void NodeTable::deserialize(ClientContext* context, StorageManager* storageManager,
    Deserializer& deSer) {
    // ...
    for (uint64_t i = 0; i < numIndexes; ++i) {
        IndexInfo indexInfo = IndexInfo::deserialize(deSer);
        // Lire le buffer StorageInfo
        auto storageInfoBuffer = ...;
        // Créer IndexHolder (pas encore chargé)
        indexes.push_back(IndexHolder(indexInfo, std::move(storageInfoBuffer), storageInfoSize));

        // Charger immédiatement les index builtin
        if (indexInfo.isBuiltin) {
            indexes[i].load(context, storageManager);
        }
        // Les index d'extension sont lazy-loaded
    }
}
```

### Lazy loading des index d'extension

**Fichier** : `src/storage/index/index.cpp` (ligne ~90-103)

```cpp
void IndexHolder::load(ClientContext* context, StorageManager* storageManager) {
    if (loaded) return;

    auto indexType = StorageManager::Get(*context)->getIndexType(indexInfo.indexType);
    // Appelle loadFunc du type d'index enregistré
    index = indexType.loadFunc(context, storageManager, indexInfo,
        std::span(storageInfoBuffer.get(), storageInfoBufferSize));
    loaded = true;
}
```

**Séquence au restart** :
1. `NodeTable::deserialize()` → crée les `IndexHolder` non-chargés
2. L'extension se charge (`load()`) → appelle `registerIndexType()`
3. Quand l'index est accédé → `IndexHolder::load()` → appelle `loadFunc` de l'extension
4. Puis `finalize()` pour rattraper les nœuds non-indexés

---

## 6. Fichier récapitulatif — Où regarder

| Concept | Fichier |
|---------|---------|
| Interface `Index` | `src/include/storage/index/index.h` |
| `IndexType`, `IndexInfo`, `IndexHolder` | `src/include/storage/index/index.h` |
| `IndexHolder::load()` | `src/storage/index/index.cpp` |
| `NodeTable::insert()` (appel index) | `src/storage/store/node_table.cpp:419-447` |
| `NodeTable::update()` (appel index) | `src/storage/store/node_table.cpp:466-508` |
| `NodeTable::delete_()` (appel index) | `src/storage/store/node_table.cpp:510-546` |
| `NodeTable::commit()` (commitInsert) | `src/storage/store/node_table.cpp:625-640` |
| `NodeTable::addIndex()` | `src/storage/store/node_table.cpp:773-779` |
| `NodeTable::serialize()` | `src/storage/store/node_table.cpp:815-821` |
| `NodeTable::deserialize()` | `src/storage/store/node_table.cpp:823-853` |
| `ExtensionUtils::registerIndexType()` | `src/extension/extension.cpp:166-167` |
| FTS Index (référence insert immédiat) | `extension/fts/src/index/fts_index.cpp` |
| HNSW Index (référence insert différé) | `extension/vector/src/index/hnsw_index.cpp` |
| HNSW `getIndexType()` | `extension/vector/src/index/hnsw_index.cpp` |
| HNSW `finalize()` (rattrapage) | `extension/vector/src/index/hnsw_index.cpp:606-635` |
| HNSW `commitInsert()` | `extension/vector/src/index/hnsw_index.cpp:585-604` |
| HNSW create function | `extension/vector/src/function/create_hnsw_index.cpp` |
| HNSW StorageInfo | `extension/vector/src/include/index/hnsw_index.h:60-82` |
| Vector extension load | `extension/vector/src/main/vector_extension.cpp` |

---

## 7. Implications pour TantivyIndex

On n'a **pas besoin de modifier le core rag3db**. On implémente :

```cpp
class TantivyIndex : public storage::Index {
    // IndexType registration
    static storage::IndexType getIndexType();
    static std::unique_ptr<Index> create(ClientContext*, ...);
    static std::unique_ptr<Index> load(ClientContext*, ..., std::span<uint8_t> storageInfo);

    // CRUD (mode différé, comme HNSW)
    bool needCommitInsert() const override { return true; }
    void insert(...) override;        // tantivy_add_document (buffered)
    void delete_(...) override;       // tantivy_delete_by_term
    void commitInsert(...) override;  // tantivy_commit + reload_reader

    // Persistence
    void checkpoint(...) override;         // sérialiser StorageInfo
    void rollbackCheckpoint() override;    // tantivy_rollback
    void finalize(ClientContext*) override; // rattraper les nœuds non-indexés (comme HNSW)
};

struct TantivyStorageInfo : storage::IndexStorageInfo {
    std::string indexPath;
    std::string fieldsJson;
    std::string stemmer;
    common::offset_t numCheckpointedNodes;
};
```

Ceci remplace la table de registre `_tantivy_indexes` du doc 02 : les métadonnées sont sérialisées **avec la NodeTable** via le mécanisme standard de checkpoint. Plus besoin de table interne dédiée.

---

## 8. Filtrage natif Tantivy — Indexer toutes les colonnes

### Le constat

`NodeTable::insert()` ne passe à l'index que les colonnes listées dans `IndexInfo.columnIDs`. On pourrait croire que c'est limitant, mais **c'est nous qui définissons cette liste**. Rien n'empêche de déclarer toutes les colonnes de la table :

```cpp
// Dans CREATE_TANTIVY_INDEX :
IndexInfo info;
info.columnIDs = {
    title_col_id,      // text → tri-field FTS (stemmed + raw + ngram)
    body_col_id,       // text → tri-field FTS
    category_col_id,   // string → fast field (filtrage par valeur exacte)
    created_at_col_id, // int64 → fast field (filtrage par range)
};
```

`NodeTable::insert()` nous passera alors TOUTES ces colonnes dans `propertyVectors`. Zéro modification du core.

### Mapping types rag3db → types Tantivy

| Type rag3db | Type Tantivy | Options | Usage |
|-------------|-------------|---------|-------|
| `STRING` (champ texte FTS) | `"text"` | `stored: true` | Tri-field : stemmed + raw + ngram |
| `STRING` (champ filtre) | `"string"` | `stored: true, indexed: true, fast: true` | Filtrage par valeur exacte |
| `INT64` | `"i64"` | `stored: true, fast: true` | Filtrage par range |
| `UINT64` | `"u64"` | `stored: true, fast: true` | Filtrage par range |
| `DOUBLE` | `"f64"` | `stored: true, fast: true` | Filtrage par range |
| `BOOL` | `"u64"` (0/1) | `fast: true` | Filtrage boolean |

Le FFI `tantivy_create_index` supporte déjà tous ces types dans le schema JSON.

### Syntaxe Cypher envisagée

```cypher
-- Créer un index FTS sur title+body, avec filtres rapides sur category et created_at
CALL CREATE_TANTIVY_INDEX('doc',
    ['title', 'body'],                    -- champs FTS
    filter_fields := ['category', 'created_at']  -- champs de filtrage rapide
);

-- Recherche avec filtre natif Tantivy (appliqué PENDANT le scoring, pas après)
CALL QUERY_TANTIVY_INDEX('doc',
    '{"type":"contains","field":"body","value":"rust programming",
      "highlight":true,
      "filters":[
        {"field":"category","op":"eq","value":"tutorial"},
        {"field":"created_at","op":"range","min":1700000000}
      ]}',
    10)
RETURN node_id, score, highlights;
```

### Implémentation côté Rust FFI

Ajout d'un champ `filters` à `QueryConfig` (~50 lignes) :

```rust
#[derive(Deserialize)]
struct FilterClause {
    field: String,
    op: String,       // "eq", "range", "in"
    value: Option<serde_json::Value>,  // pour "eq"
    min: Option<serde_json::Value>,    // pour "range"
    max: Option<serde_json::Value>,    // pour "range"
    values: Option<Vec<serde_json::Value>>, // pour "in"
}

// Dans QueryConfig :
#[serde(default)]
filters: Vec<FilterClause>,
```

Construction du query combiné :

```rust
fn apply_filters(fts_query: Box<dyn Query>, filters: &[FilterClause], schema: &Schema)
    -> Box<dyn Query>
{
    if filters.is_empty() {
        return fts_query;
    }
    let mut clauses = vec![(Occur::Must, fts_query)];
    for filter in filters {
        let field = schema.get_field(&filter.field)?;
        match filter.op.as_str() {
            "eq" => {
                let term = Term::from_field_text(field, value_str);
                clauses.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
            }
            "range" => {
                clauses.push((Occur::Must, Box::new(RangeQuery::new_i64(field, min..max))));
            }
            "in" => {
                // OR de plusieurs TermQuery
                let sub: Vec<_> = values.iter()
                    .map(|v| (Occur::Should, Box::new(TermQuery::new(...)) as Box<dyn Query>))
                    .collect();
                clauses.push((Occur::Must, Box::new(BooleanQuery::new(sub))));
            }
            _ => {} // ignorer les ops inconnues
        }
    }
    Box::new(BooleanQuery::new(clauses))
}
```

### Pourquoi c'est mieux que `tantivy_search_filtered`

`tantivy_search_filtered(allowed_ids)` fait du **pré-filtrage** : on passe une liste d'IDs et Tantivy ne score que ceux-là. Ça marche, mais :

| Aspect | `search_filtered(ids)` | `filters` natif |
|--------|----------------------|-----------------|
| Filtrage | Côté rag3db (Cypher WHERE) puis IDs passés | Côté Tantivy (pendant le scoring) |
| Mémoire | Tous les IDs matchants en mémoire | Aucune allocation, filtre à la volée |
| Performance | O(n) pour construire le HashSet d'IDs | O(1) lookup dans les fast fields |
| Combinabilité | Difficile de combiner FTS + filtre efficacement | BooleanQuery natif, optimisé par Tantivy |
| Complexité Cypher | `MATCH ... WHERE ... WITH collect(...) CALL ...` | Un seul CALL avec le JSON |

Les deux approches restent disponibles — `search_filtered` est utile pour des filtres complexes basés sur le graphe (traversées, chemins), tandis que `filters` est optimal pour les filtres simples sur des propriétés directes.

### Effort total

| Pièce | Lignes | Modif core ? |
|-------|--------|-------------|
| Déclarer toutes les colonnes dans `IndexInfo.columnIDs` | ~5 C++ | Non |
| Mapper types rag3db → types Tantivy dans le schema JSON | ~30 C++ | Non |
| Ajouter `filters` + `FilterClause` à `QueryConfig` (serde) | ~20 Rust | Non |
| Builder les filter clauses en `BooleanQuery` | ~50 Rust | Non |
| Supporter `eq`, `range`, `in` (les 3 ops utiles) | ~30 Rust | Non |

**Total : ~135 lignes, zéro modification du core rag3db.**

### Bonus : UPDATE propre

En déclarant toutes les colonnes dans `columnIDs`, `NodeTable::update()` nous passe le nouveau vecteur de la colonne modifiée. Mais surtout, comme on reçoit TOUTES les colonnes indexées lors de l'INSERT initial, on peut reconstruire un document complet dans Tantivy. Pour un UPDATE :

1. `delete_()` reçoit le `nodeIDVector` → `tantivy_delete_by_term("_node_id", offset)`
2. Le prochain `commitInsert()` (ou `insert()` si mode immédiat) re-scanne le nœud avec toutes ses colonnes → `tantivy_add_document()` avec le doc complet

Ce mécanisme est identique à celui du HNSW qui gère aussi les updates via delete + re-insert.
