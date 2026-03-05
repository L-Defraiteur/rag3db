# Approche : Index hooks dans NodeTable pour indexation incrémentale

> Idée d'architecture pour que Lucivy se mette à jour automatiquement
> quand des nœuds sont insérés/supprimés dans rag3db.

---

## Problème

Actuellement, `CREATE_LUCIVY_INDEX` fait un bulk scan de toute la table. Si l'utilisateur ajoute des nœuds après, l'index est périmé. Options manuelles (ADD_LUCIVY_DOC, SYNC) = friction pour l'utilisateur, erreur-prone.

## Idée : hooks dans `storage::Index`

rag3db a déjà une infra d'index :
- `NodeTable::addIndex(std::unique_ptr<Index>)` — existe, utilisé par FTS
- `storage::Index` — classe de base
- Mais les index sont **statiques** — créés une fois, jamais mis à jour

### Modification du core rag3db

#### 1. Nouvelles méthodes virtuelles dans `Index`

```cpp
class Index {
    // ... existant ...

    // Appelé par NodeTable après insertion de lignes
    virtual void onInsert(const std::vector<nodeID_t>& nodeIDs,
                          const std::vector<ValueVector*>& propertyVectors) {}

    // Appelé par NodeTable après suppression de lignes
    virtual void onDelete(const std::vector<nodeID_t>& nodeIDs) {}

    // Appelé au commit de la transaction
    virtual void onCommit() {}

    // Appelé au rollback
    virtual void onRollback() {}
};
```

Méthodes vides par défaut → zéro impact sur FTS existant et les autres index.

#### 2. Branchement dans NodeTable

```cpp
void NodeTable::insert(/* ... */) {
    // ... logique existante ...

    // Notifier les index enregistrés
    for (auto& index : indexes_) {
        index->onInsert(nodeIDs, propertyVectors);
    }
}

void NodeTable::delete(/* ... */) {
    // ... logique existante ...

    for (auto& index : indexes_) {
        index->onDelete(nodeIDs);
    }
}
```

#### 3. Branchement transactionnel

Dans le gestionnaire de transactions, au commit/rollback, appeler les hooks sur les index des tables modifiées :

```cpp
void Transaction::commit() {
    // ... logique existante ...

    // Notifier les index
    for (auto& table : modifiedTables_) {
        for (auto& index : table->getIndexes()) {
            index->onCommit();
        }
    }
}
```

### Implémentation LucivyIndex (dans l'extension)

```cpp
class LucivyIndex : public Index {
    LucivyHandlePtr handle_;
    std::vector<std::string> indexedFields_;  // noms des propriétés indexées

    void onInsert(const std::vector<nodeID_t>& nodeIDs,
                  const std::vector<ValueVector*>& props) override {
        for (auto i = 0u; i < nodeIDs.size(); i++) {
            nlohmann::json doc;
            doc["_node_id"] = nodeIDs[i].offset;
            for (size_t f = 0; f < indexedFields_.size(); f++) {
                doc[indexedFields_[f]] = props[f]->getAsString(i);
            }
            lucivy_add_document(handle_, doc.dump().c_str());
            // Pas de commit — buffered dans le heap Lucivy (50MB)
        }
    }

    void onDelete(const std::vector<nodeID_t>& nodeIDs) override {
        for (auto& nid : nodeIDs) {
            lucivy_delete_by_term(handle_, "_node_id",
                std::to_string(nid.offset).c_str());
        }
    }

    void onCommit() override {
        lucivy_commit(handle_);
        lucivy_reload_reader(handle_);
        // TODO: mettre à jour num_docs dans le registre _lucivy_indexes
    }

    void onRollback() override {
        lucivy_rollback(handle_);
    }
};
```

## Avantages

| Aspect | Bénéfice |
|--------|----------|
| **Transparent** | `CREATE (:doc {...})` met à jour l'index automatiquement |
| **Transactionnel** | Rollback Lucivy si rollback rag3db → cohérence |
| **Batché** | Buffer 50MB Lucivy, commit une seule fois par transaction |
| **Générique** | Mécanisme réutilisable pour tout type d'index custom |
| **Minimal** | ~20 lignes dans le core rag3db, le reste dans l'extension |
| **Compatible COPY** | COPY passe par `NodeTable::insert()` → indexé automatiquement |

## Impact sur le plan Cypher

- **CREATE_LUCIVY_INDEX** : bulk initial (VertexCompute) + `NodeTable::addIndex(lucivyIndex)` → après ça, tout INSERT/DELETE est automatique
- **Plus besoin** de fonctions `ADD_LUCIVY_DOC` ou `SYNC_LUCIVY_INDEX`
- **QUERY_LUCIVY_INDEX** : inchangé
- **DROP_LUCIVY_INDEX** : retire l'index de NodeTable + close handle + rm dir

## Points à explorer avant implémentation

1. **NodeTable::insert() / delete()** — où exactement sont-ils implémentés ? Les `ValueVector*` contiennent-ils les données texte des propriétés qu'on veut indexer ?
2. **Cycle transactionnel** — comment brancher `onCommit`/`onRollback` ? Depuis `Transaction::commit()` ou un autre point ?
3. **COPY bulk** — passe-t-il par le même `NodeTable::insert()` ou un chemin séparé (batch insert) ?
4. **Reconstruction au restart** — au `load()` de l'extension, les index sont-ils rechargés automatiquement via `NodeTable::addIndex()`, ou faut-il le refaire manuellement depuis le registre `_lucivy_indexes` ?
5. **Thread safety** — `onInsert` peut-il être appelé depuis plusieurs threads en parallèle ? Si oui, le `lucivy_add_document` est déjà thread-safe (writer derrière Mutex).
6. **UPDATE** — Kuzu/rag3db fait-il un DELETE+INSERT ou un update in-place ? Si in-place, il faudra un hook `onUpdate` aussi.
