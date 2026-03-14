# Doc 12 — Fix : MemBlobStore fallback + service registration sparse_handles

Date : 13 mars 2026

Ref : doc 10 (rapport Phase 1), doc 11 (plan Phase 2)

## Problème découvert

Les tests E2E sparse (`phase3_sparse_search_finds_results`, `phase3_hybrid_3way`) échouent avec `sparse=0` après la migration Phase 1-3. Deux causes :

### Cause 1 : `sparse_handles` non enregistré comme service

**Symptôme** : `EmbedNode` et `KBEmbedNode` cherchent `ctx.service("sparse_handles")` mais le service n'est jamais enregistré dans le `ServiceRegistry`.

**Fix appliqué** (commit `d038aecc2` + post) : ajouter dans les 3 endroits où les services sont créés (drain simple, drain unifié, reindex) :

```rust
services.register::<HashMap<String, Arc<sparse_vector::handle::SparseHandle>>>(
    "sparse_handles", Arc::new(self.sparse_handles.clone()));
```

**Fichier** : `catalog.rs`, lignes ~1354, ~1974, ~2192

**Status** : fait, mais insuffisant seul (cause 2).

### Cause 2 : `blob_store` toujours `None` sans `sync_conn`

**Symptôme** : `ensure_sparse_handle()` fait `let Some(ref blob_store) = self.blob_store else { return };` — si `blob_store` est `None`, les handles ne sont jamais créés.

`blob_store` n'est défini que si `sync_conn` est `Some` (via `set_sync_connection()`). Les tests E2E ne l'appellent jamais.

**Code concerné** (`catalog.rs`, `initialize()`) :

```rust
if let Some(ref sync_conn) = self.sync_conn {
    self.conn.execute("CREATE NODE TABLE IF NOT EXISTS _index_blobs ...").await?;
    self.blob_store = Some(Arc::new(CypherBlobStore::from_sync_connection(sync_conn.clone())));
}
// ensure_sparse_handle() appelé plus bas → blob_store est None → no-op
```

## Solution : MemBlobStore fallback

### Principe

- **Avec `sync_conn`** → `CypherBlobStore` → sparse persisté dans `_index_blobs` (production, fichier DB)
- **Sans `sync_conn`** → `MemBlobStore` → sparse en mémoire seulement (tests, in-memory DB)

Pour une DB in-memory, la persistance n'a pas de sens de toute façon (tout est perdu à la fermeture). Le `MemBlobStore` suffit.

### Changement à faire

Dans `catalog.rs`, méthode `initialize()`, remplacer :

```rust
// Avant
if let Some(ref sync_conn) = self.sync_conn {
    self.conn.execute(
        "CREATE NODE TABLE IF NOT EXISTS _index_blobs (_key STRING, _data BLOB, PRIMARY KEY(_key))"
    ).await.map_err(|e| CatalogError::DbError(e.to_string()))?;
    self.blob_store = Some(Arc::new(CypherBlobStore::from_sync_connection(sync_conn.clone())));
}
```

Par :

```rust
// Après
if let Some(ref sync_conn) = self.sync_conn {
    // Persistent blob store backed by _index_blobs table
    self.conn.execute(
        "CREATE NODE TABLE IF NOT EXISTS _index_blobs (_key STRING, _data BLOB, PRIMARY KEY(_key))"
    ).await.map_err(|e| CatalogError::DbError(e.to_string()))?;
    self.blob_store = Some(Arc::new(CypherBlobStore::from_sync_connection(sync_conn.clone())));
} else if self.blob_store.is_none() {
    // In-memory fallback — sparse indexes work but aren't persisted
    self.blob_store = Some(Arc::new(sparse_vector::blob_store::MemBlobStore::new()));
}
```

### Import nécessaire

`MemBlobStore` est re-exporté par `sparse_vector::blob_store::MemBlobStore` (vient de `lucivy_core::blob_store`).

Le type `blob_store` dans le Catalog est `Option<Arc<CypherBlobStore>>` — il faudra le changer en `Option<Arc<dyn lucivy_core::blob_store::BlobStore>>` pour accepter les deux types.

### Changement de type du champ blob_store

```rust
// Avant
blob_store: Option<Arc<CypherBlobStore>>,

// Après
blob_store: Option<Arc<dyn lucivy_core::blob_store::BlobStore>>,
```

Vérifier que tous les endroits qui utilisent `self.blob_store` fonctionnent avec `dyn BlobStore` au lieu de `CypherBlobStore` concret. L'usage principal est `ensure_sparse_handle()` qui passe `blob_store.clone()` à `SparseHandle::create_with_store` / `open_with_store` — ces fonctions prennent `Arc<dyn BlobStore>`, donc c'est compatible.

### Fichiers à modifier

| Fichier | Changement |
|---------|-----------|
| `src/catalog.rs` | Changer type `blob_store` en `dyn BlobStore`, ajouter fallback `MemBlobStore` dans `initialize()` |

### Résultat attendu

- Tests E2E in-memory : sparse fonctionne sans appeler `set_sync_connection()`
- Tests E2E fichier (phase6 persistence) : doivent toujours appeler `set_sync_connection()` pour la persistance réelle
- Production : inchangé, `set_sync_connection()` reste nécessaire pour persistance

## Tests E2E à valider après le fix

```bash
./run_e2e.sh --test e2e_search phase3 --summary    # sparse search basique
./run_e2e.sh --test e2e_search phase4 --summary    # combinaisons de signaux
./run_e2e.sh --test e2e_search phase5 --summary    # dual embedder
./run_e2e.sh --test e2e_search phase6 --summary    # persistence mmap (nécessite set_sync_connection)
```

## Changements déjà faits (non committés)

- `catalog.rs` : enregistrement `sparse_handles` service dans 3 endroits du ServiceRegistry
- `record_nodes.rs` : Phase 3 (handle.insert au lieu de colonnes sparse)
- `schema.rs` : colonnes sparse supprimées

## Ce qui est committé

- `dbcf494ca` — Phase 1 (fondation, SparseHandle + BlobStore)
- `cc5ffed5a` — Phase 2 (SparseCommitNode + search_sparse direct)
- `d038aecc2` — Phase 3 (handle.insert + suppression colonnes sparse)
