# Doc 07 — Plan : CypherBlobStore dans rag3weaver

Date : 13 mars 2026

Ref : doc 06 (findings CypherBlobStore), doc 05 (impl BlobStore/BlobDirectory)

## Objectif

Implémenter `CypherBlobStore` dans rag3weaver : une implémentation du trait `BlobStore` qui persiste les blobs dans rag3db via la table `_index_blobs`. C'est le pont entre les abstractions Rust (lucivy BlobDirectory, sparse SparseHandle) et le storage rag3db.

## Contexte technique

### Ce qui existe déjà

| Composant | Localisation | Statut |
|-----------|-------------|--------|
| `BlobStore` trait | `lucivy_core/src/blob_store.rs` | ✅ Implémenté |
| `MemBlobStore` | `lucivy_core/src/blob_store.rs` | ✅ 3 tests |
| `BlobDirectory` | `lucivy_core/src/blob_directory.rs` | ✅ 7 tests |
| `BlobStore` trait (copie) | `sparse_vector/src/blob_store.rs` | ✅ Implémenté |
| `SparseHandle::*_with_store` | `sparse_vector/src/handle.rs` | ✅ 7 tests |
| `CypherValue::Blob(Vec<u8>)` | `rag3weaver/src/connection.rs` | ✅ Ajouté |
| Mapping `rag3db::Value::Blob` ↔ `CypherValue::Blob` | `rag3weaver/src/rag3db_connection.rs` | ✅ Ajouté |

### Problème sync/async

Le trait `BlobStore` est **sync** (imposé par lucivy `Directory` trait). `DbConnection` est **async**.

**Solution retenue** : `Rag3dbConnection` est sync en interne (`query_sync`, `query_with_params_sync`). Le CypherBlobStore n'utilise pas `DbConnection` — il prend directement une **closure sync** d'exécution de queries. Ça évite tout problème de runtime tokio.

```rust
type QueryFn = Box<dyn Fn(&str, &[QueryParam]) -> Result<QueryResult, String> + Send + Sync>;
```

Pour `Rag3dbConnection`, on wrappe `query_with_params_sync` dans cette closure. Pour WASM/HTTP, l'appelant fournit sa propre closure qui bloque sur l'async.

## Fichiers à créer/modifier

### Nouveau fichier

| Fichier | Contenu |
|---------|---------|
| `src/cypher_blob_store.rs` | `CypherBlobStore` struct + `impl BlobStore` |

### Fichiers à modifier

| Fichier | Changement |
|---------|-----------|
| `src/lib.rs` | Ajouter `pub mod cypher_blob_store;` |
| `Cargo.toml` | Ajouter dépendance `lucivy_core` (pour le trait BlobStore) |
| `src/catalog.rs` | Créer table `_index_blobs` dans `initialize()`, exposer le store |

## Design détaillé

### 1. Table `_index_blobs`

```sql
CREATE NODE TABLE IF NOT EXISTS _index_blobs (
    _key STRING,
    _data BLOB,
    PRIMARY KEY(_key)
)
```

- `_key` : `"{index_name}/{file_name}"` (ex: `"Product_fts/meta.json"`, `"Product_sparse/sparse.mmap"`)
- `_data` : BLOB natif rag3db (pas de base64, zéro overhead)
- Primary key sur `_key` pour MERGE upsert

### 2. Struct CypherBlobStore

```rust
use lucivy_core::blob_store::BlobStore;
use crate::connection::{CypherValue, QueryParam, QueryResult};

type QueryFn = Box<dyn Fn(&str, &[QueryParam]) -> Result<QueryResult, String> + Send + Sync>;

pub struct CypherBlobStore {
    query_fn: QueryFn,
}

impl CypherBlobStore {
    pub fn new(query_fn: QueryFn) -> Self { ... }

    /// Constructeur pour Rag3dbConnection (feature rag3db-native).
    #[cfg(feature = "rag3db-native")]
    pub fn from_rag3db(conn: &Rag3dbConnection) -> Self { ... }

    /// Crée la table _index_blobs si elle n'existe pas.
    pub fn ensure_table(&self) -> Result<(), String> { ... }

    fn execute(&self, cypher: &str, params: &[QueryParam]) -> Result<QueryResult, String> {
        (self.query_fn)(cypher, params)
    }
}
```

### 3. Implémentation BlobStore

```rust
impl BlobStore for CypherBlobStore {
    fn save(&self, index_name: &str, file_name: &str, data: &[u8]) -> io::Result<()> {
        let key = format!("{index_name}/{file_name}");
        self.execute(
            "MERGE (b:_index_blobs {_key: $key}) SET b._data = $data",
            &[
                QueryParam::new("key", CypherValue::String(key)),
                QueryParam::new("data", CypherValue::Blob(data.to_vec())),
            ],
        ).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(())
    }

    fn load(&self, index_name: &str, file_name: &str) -> io::Result<Vec<u8>> {
        let key = format!("{index_name}/{file_name}");
        let result = self.execute(
            "MATCH (b:_index_blobs {_key: $key}) RETURN b._data",
            &[QueryParam::new("key", CypherValue::String(key.clone()))],
        ).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        match result.rows.first().and_then(|r| r.first()) {
            Some(CypherValue::Blob(data)) => Ok(data.clone()),
            _ => Err(io::Error::new(io::ErrorKind::NotFound, format!("{key} not found"))),
        }
    }

    fn delete(&self, index_name: &str, file_name: &str) -> io::Result<()> {
        let key = format!("{index_name}/{file_name}");
        self.execute(
            "MATCH (b:_index_blobs {_key: $key}) DELETE b",
            &[QueryParam::new("key", CypherValue::String(key))],
        ).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(())
    }

    fn exists(&self, index_name: &str, file_name: &str) -> io::Result<bool> {
        let key = format!("{index_name}/{file_name}");
        let result = self.execute(
            "MATCH (b:_index_blobs {_key: $key}) RETURN count(b)",
            &[QueryParam::new("key", CypherValue::String(key))],
        ).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        match result.rows.first().and_then(|r| r.first()) {
            Some(CypherValue::Int(n)) => Ok(*n > 0),
            _ => Ok(false),
        }
    }

    fn list(&self, index_name: &str) -> io::Result<Vec<String>> {
        let prefix = format!("{index_name}/");
        let result = self.execute(
            "MATCH (b:_index_blobs) WHERE b._key STARTS WITH $prefix RETURN b._key",
            &[QueryParam::new("prefix", CypherValue::String(prefix.clone()))],
        ).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(result.rows.iter().filter_map(|r| {
            match r.first() {
                Some(CypherValue::String(key)) => Some(key.strip_prefix(&prefix)?.to_string()),
                _ => None,
            }
        }).collect())
    }
}
```

### 4. Trait BlobStore : quelle copie utiliser ?

Le trait `BlobStore` est identique dans `lucivy_core` et `sparse_vector`. Le CypherBlobStore implémente celui de `lucivy_core` (dépendance ajoutée dans Cargo.toml).

Pour sparse_vector, deux options :
- **A. sparse_vector dépend de lucivy_core** pour le trait → un seul trait, un seul impl
- **B. Wrapper/adaptateur** dans rag3weaver qui convertit `lucivy_core::BlobStore` → `sparse_vector::BlobStore`

**Option A recommandée** : ajouter `lucivy_core` en dépendance de sparse_vector, ne garder le trait que dans lucivy_core, et réexporter depuis sparse_vector. Élimine la duplication.

**Alternative temporaire** : implémenter les deux traits dans CypherBlobStore (le code est identique, juste deux `impl` blocks).

### 5. Intégration dans Catalog

Dans `Catalog::initialize()`, après les DDL et avant le chargement des configs :

```rust
// Create _index_blobs table for BlobStore persistence
self.conn.execute(
    "CREATE NODE TABLE IF NOT EXISTS _index_blobs (_key STRING, _data BLOB, PRIMARY KEY(_key))"
).await.map_err(|e| CatalogError::DbError(e.to_string()))?;
```

Exposer le store via un getter :

```rust
impl Catalog {
    /// Get a CypherBlobStore backed by this catalog's DB connection.
    /// Used by rag3weaver to create BlobDirectory (lucivy) and
    /// SparseHandle::create_with_store (sparse).
    pub fn blob_store(&self) -> Arc<CypherBlobStore> {
        // ... construit à partir de self.conn
    }
}
```

### 6. Construction du CypherBlobStore depuis Catalog

Le `Catalog` a `conn: Arc<dyn DbConnection>`. Pour le mode natif, on sait que c'est un `Rag3dbConnection` (sync en interne). La closure sync :

```rust
// Option propre : trait downcast ou sync wrapper
let conn = self.conn.clone();
let rt = tokio::runtime::Handle::current();
let query_fn = move |cypher: &str, params: &[QueryParam]| -> Result<QueryResult, String> {
    rt.block_on(conn.execute_with_params(cypher, params))
        .map_err(|e| e.to_string())
};
```

**Attention** : `block_on` dans un contexte async peut paniquer. Mais `Rag3dbConnection::execute_with_params` est en réalité sync (appelle `query_with_params_sync`), donc le block_on retourne immédiatement. Pas de deadlock.

Pour WASM : le CypherBlobStore n'est pas utilisé (WASM utilise le filesystem via StdFsDirectory/path). Pas de problème.

## Tests

### Tests unitaires (pas de DB)

```rust
#[test]
fn cypher_blob_store_with_mock() {
    // Mock query_fn qui simule _index_blobs en mémoire
    // Teste save/load/delete/exists/list
}
```

### Tests d'intégration (feature rag3db-native)

```rust
#[tokio::test]
#[ignore] // nécessite rag3db build
async fn cypher_blob_store_roundtrip() {
    let conn = Rag3dbConnection::in_memory().unwrap();
    // CREATE TABLE _index_blobs
    // save, load, exists, list, delete
}

#[tokio::test]
#[ignore]
async fn cypher_blob_store_with_blob_directory() {
    // CypherBlobStore → BlobDirectory → lucivy IndexWriter → commit → reopen → search
}

#[tokio::test]
#[ignore]
async fn cypher_blob_store_with_sparse_handle() {
    // CypherBlobStore → SparseHandle::create_with_store → insert → commit → reopen → search
}
```

## Ce qu'on ne fait PAS dans cette étape

- Migrer Catalog pour utiliser BlobDirectory/SparseHandle directement (étape suivante)
- Supprimer les appels Cypher `CALL CREATE_LUCIVY_INDEX(...)` (gardés pour l'instant)
- Implémenter CypherBlobStore pour WASM
- Compresser les blobs (zstd) avant stockage
- GC des blobs orphelins

## Ordre d'exécution

```
1. Cargo.toml : ajouter lucivy_core en dépendance
2. src/cypher_blob_store.rs : struct + impl BlobStore + ensure_table + tests mock
3. src/lib.rs : pub mod cypher_blob_store
4. Tests unitaires avec mock query_fn
5. (optionnel) Tests intégration avec Rag3dbConnection::in_memory()
```

## Vérification

```bash
# Compilation
cargo check

# Tests unitaires
cargo test --lib -- cypher_blob_store

# Tests intégration (si rag3db buildé)
cargo test --lib --features "rag3db-native" -- cypher_blob_store --ignored
```
