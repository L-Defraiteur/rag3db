# Doc 02 — Plan : migration FTS de l'extension C++ vers Rust (LucivyHandle direct)

Date : 15 mars 2026

## Motivation

1. **Bug BM25 actuel** : les champs `._ngram`/`._raw` ne sont pas alimentés lors des insertions via les hooks C++ du NodeTable → le mode Contains ne fonctionne pas
2. **Cohérence** : sparse est déjà migré vers `handle.insert()`/`handle.search()` direct — FTS devrait suivre le même pattern
3. **Simplification** : plus besoin des fonctions C++ `CREATE_LUCIVY_INDEX`/`FLUSH_LUCIVY_INDEX` pour le pipeline rag3weaver
4. **Contrôle** : rag3weaver gère directement le cycle de vie des index (create/insert/commit/search/close)

## État actuel

### Flux FTS actuel (via extension C++)
```
register_entity/register_kb
  → DDL: CALL CREATE_LUCIVY_INDEX('{table}', [...])  // extension C++

drain (EmbedNode)
  → INSERT into entity table  // hooks C++ NodeTable::insert → LucivyIndex::insert

drain (FlushNode)
  → CALL FLUSH_LUCIVY_INDEX('{table}')  // extension C++

search
  → CALL QUERY_LUCIVY_INDEX('{table}', ...)  // extension C++ → TableFunc

shutdown
  → CALL CLOSE_LUCIVY_INDEX('{table}')  // extension C++
```

### Flux sparse (déjà migré vers Rust)
```
initialize
  → ensure_sparse_handle(table)  // SparseHandle::create_with_store ou open_with_store

drain (EmbedNode)
  → handle.insert(offset, sparse_vector)  // Rust direct

drain (SparseCommitNode)
  → handle.commit_inner()  // Rust direct

search
  → handle.search(&query_vec, limit)  // Rust direct → resolve offsets via Cypher

shutdown
  → handle.commit_inner() + drop  // Rust direct
```

## Flux FTS cible (même pattern que sparse)

```
initialize
  → ensure_fts_handle(table)  // LucivyHandle::create ou open (via BlobStore ou StdFsDirectory)

drain (EmbedNode ou InsertNode)
  → handle.add_document(offset, fields)  // Rust direct via lucivy_core

drain (FtsCommitNode)  // nouveau, même pattern que SparseCommitNode
  → handle.commit() + handle.reload_reader()

search
  → handle.search(query_config, limit)  // Rust direct → resolve offsets via Cypher

shutdown
  → handle.close() + drop
```

## Phases

### Phase 1 : FTS handles dans le Catalog
- Ajouter `fts_handles: HashMap<String, Arc<LucivyHandle>>` au Catalog
- `ensure_fts_handle()` : create ou open via StdFsDirectory (même path que l'extension C++ : `{db_parent}/lucivy_indexes/{table}/`)
- Garder `CREATE_LUCIVY_INDEX` DDL dans schema pour backward compat mais skip si handle existe

### Phase 2 : search_fts direct
- Remplacer `QUERY_LUCIVY_INDEX` dans `search.rs` par `handle.search()` direct
- Créer `search_fts(handle, conn, entity, query, limit, return_fields)` — même pattern que `search_sparse`
- Résolution des offsets `_node_id` → entity data via Cypher

### Phase 3 : insertion directe
- Dans EmbedNode/KBEmbedNode : au lieu d'écrire dans les colonnes de la table (hooks C++), appeler `handle.add_document()` directement
- Supprimer la dépendance aux hooks NodeTable pour FTS
- Créer `FtsCommitNode` (même pattern que `SparseCommitNode`)

### Phase 4 : cleanup
- Supprimer les appels C++ (`CREATE_LUCIVY_INDEX`, `FLUSH_LUCIVY_INDEX`, `QUERY_LUCIVY_INDEX`) du code rag3weaver
- Garder `CLOSE_LUCIVY_INDEX` comme fallback pour les DBs legacy
- Cleanup des colonnes orphelines

## Différences avec la migration sparse

| Aspect | Sparse | FTS |
|--------|--------|-----|
| Stockage | BlobStore (CypherBlobStore ou MemBlobStore) | Filesystem (StdFsDirectory) |
| Schema | Simple (indices + weights) | Multi-champs (text + filter fields) |
| Insertion | `handle.insert(offset, vector)` | `handle.add_document(offset, fields)` — multiple fields |
| Search | `handle.search(vec, limit)` → offsets | `handle.search(query, limit)` → offsets + highlights |
| Highlights | N/A | À conserver — le search FTS retourne des snippets |
| Commit | `commit_inner()` | `commit()` + `reload_reader()` |
| Hooks C++ | Aucun (déjà supprimés) | `NodeTable::insert` → `LucivyIndex::insert` (à supprimer) |

## Points d'attention

1. **Filter fields** : l'extension C++ gère les filter fields natifs (INT64, DOUBLE, etc.) dans les insertions. Le bridge Rust (`add_document_mixed`) le fait aussi — vérifier la parité
2. **Highlights** : le search FTS retourne des highlights via `get_highlights()`. À conserver dans le search direct
3. **Schema JSON** : `LucivyHandle::create()` prend un `SchemaConfig` JSON — le même que `CREATE_LUCIVY_INDEX` construit côté C++. Réutiliser la même génération
4. **Persistance** : FTS utilise le filesystem direct (pas BlobStore) — le path `{db_parent}/lucivy_indexes/{table}/` est déjà stable
5. **In-memory DB** : pour les DBs in-memory, le path est `$TMPDIR/rag3db_lucivy/{db_id}/` — gérer ce cas
