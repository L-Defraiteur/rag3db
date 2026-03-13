# Doc 06 — Findings : CypherBlobStore dans rag3weaver

Date : 13 mars 2026

Ref : doc 04 (findings blob store), doc 05 (impl BlobDirectory + BlobStore sparse)

## Objectif

Comprendre comment rag3weaver interagit avec rag3db pour designer un `CypherBlobStore` — une implémentation du trait `BlobStore` (défini dans lucivy_core et sparse_vector) qui persiste les blobs via des queries Cypher dans rag3db.

---

## 1. Architecture de connexion rag3weaver → rag3db

### 1a. Trait DbConnection (async)

```rust
// connection.rs
#[async_trait]
pub trait DbConnection: Send + Sync {
    async fn execute(&self, cypher: &str) -> Result<QueryResult, DbError>;
    async fn execute_with_params(
        &self,
        cypher: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError>;
}
```

**Implémentations existantes** :
- `Rag3dbConnection` : connexion native in-process (feature `rag3db-native`)
- `CallbackConnection` : closure async fournie par l'appelant (WASM, HTTP)
- `MockConnection` : retourne des résultats vides (tests)

### 1b. Types de valeurs

```rust
pub enum CypherValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<CypherValue>),
    Map(BTreeMap<String, CypherValue>),
}
```

**Pas de variant `Blob`/`Bytes`.** Le type BLOB de rag3db existe (kuzu le supporte nativement), mais `CypherValue` ne le mappe pas. Dans `rag3db_connection.rs`, les BLOBs tombent dans le fallback `other => CypherValue::String(format!("{other}"))`.

### 1c. QueryParam

```rust
pub struct QueryParam {
    pub name: String,
    pub value: CypherValue,
}
```

Utilisé avec `execute_with_params` pour les prepared statements.

---

## 2. Pattern de persistence existant : `_catalog_meta`

Rag3weaver utilise déjà une table key-value `_catalog_meta` pour persister les configurations :

```
CREATE NODE TABLE _catalog_meta (_key STRING, _value STRING, PRIMARY KEY(_key))
```

### Écriture (MERGE upsert)

```rust
async fn persist_meta_key(&self, key: &str, value: &str) -> Result<(), CatalogError> {
    self.conn.execute_with_params(
        "MERGE (m:_catalog_meta {_key: $key}) SET m._value = $value",
        &[
            QueryParam::new("key", CypherValue::String(key.to_string())),
            QueryParam::new("value", CypherValue::String(value.to_string())),
        ],
    ).await?;
    Ok(())
}
```

### Lecture (MATCH + filtre prefix)

```rust
let result = self.conn.execute(
    "MATCH (m:_catalog_meta) WHERE m._key STARTS WITH 'entity_config:' RETURN m._key, m._value"
).await?;
```

### Usage actuel

| Clé | Valeur | Usage |
|-----|--------|-------|
| `entity_config:{name}` | JSON EntityConfig | register_entity idempotent |
| `relation:{name}` | JSON RelationConfig | register_relation |
| `kb_config:{name}` | JSON KBConfig | register_kb |

**Tout est JSON dans des colonnes STRING.**

---

## 3. Problème central : sync vs async

### Le trait BlobStore est sync

```rust
// lucivy_core/src/blob_store.rs & sparse_vector/src/blob_store.rs
pub trait BlobStore: Send + Sync + 'static {
    fn load(&self, index_name: &str, file_name: &str) -> io::Result<Vec<u8>>;
    fn save(&self, index_name: &str, file_name: &str, data: &[u8]) -> io::Result<()>;
    fn delete(&self, index_name: &str, file_name: &str) -> io::Result<()>;
    fn exists(&self, index_name: &str, file_name: &str) -> io::Result<bool>;
    fn list(&self, index_name: &str) -> io::Result<Vec<String>>;
}
```

**Pourquoi sync ?** Le trait `Directory` de lucivy (que `BlobDirectory` implémente) est entièrement sync. Lucivy n'est pas async — l'IndexWriter, le SegmentUpdater, le GC appellent `Directory` de façon synchrone.

### DbConnection est async

```rust
async fn execute_with_params(&self, cypher: &str, params: &[QueryParam]) -> Result<...>;
```

### Solutions possibles

| Option | Mécanisme | Pro | Con |
|--------|-----------|-----|-----|
| **A. `block_on` dans BlobStore** | `tokio::runtime::Handle::current().block_on(async_query)` | Simple, pas de changement de trait | Panic si appelé depuis un contexte async (nested runtime) |
| **B. Runtime dédié** | Le CypherBlobStore possède son propre `tokio::Runtime` et fait `self.rt.block_on(...)` | Pas de conflit de runtime | Coût d'un runtime supplémentaire |
| **C. Sync DbConnection** | Ajouter une méthode `execute_sync` à `DbConnection` ou un trait `SyncDbConnection` | Propre | Change l'interface rag3weaver |
| **D. Rag3dbConnection est déjà sync** | `Rag3dbConnection` utilise `query_sync` en interne (le `async` est un wrapper) | Zéro overhead | Couplé à l'implémentation native |

**Option D est la plus pragmatique.** En regardant `rag3db_connection.rs`, les queries sont synchrones en interne (`query_sync`, `query_with_params_sync`). Le trait async est là pour supporter WASM/HTTP. Pour le `CypherBlobStore` natif, on peut directement utiliser la connexion rag3db sync.

**Approche recommandée** : le `CypherBlobStore` prend un `Arc<dyn DbConnection>` + un `Handle` tokio (ou un `Runtime`), et utilise `block_on` pour les appels. Ou mieux : il prend directement une closure sync `Fn(&str, &[QueryParam]) -> Result<QueryResult>` qui wrappe la connexion.

---

## 4. Stockage BLOB : STRING base64 vs BLOB natif

### Option A : Base64 dans STRING

```sql
CREATE NODE TABLE _index_blobs (
    _key STRING,        -- "{index_name}/{file_name}"
    _data STRING,       -- base64-encoded binary data
    PRIMARY KEY(_key)
)
```

**Pro** : fonctionne avec le CypherValue actuel (pas de changement à connection.rs).
**Con** : +33% taille en mémoire et sur disque. Un index lucivy de 100 MB → 133 MB en base64.

### Option B : BLOB natif rag3db

```sql
CREATE NODE TABLE _index_blobs (
    _key STRING,
    _data BLOB,
    PRIMARY KEY(_key)
)
```

**Pro** : pas d'overhead d'encodage, taille native.
**Con** : nécessite d'ajouter `CypherValue::Blob(Vec<u8>)` et de mapper le type BLOB de rag3db dans `rag3db_connection.rs`. Aussi, les paramètres Cypher pour BLOB nécessitent un encoding spécifique (hex ou `\x` prefix dans Kuzu).

### Option C : Clé composite

```sql
CREATE NODE TABLE _index_blobs (
    _index_name STRING,
    _file_name STRING,
    _data BLOB,
    PRIMARY KEY(_index_name, _file_name)
)
```

**Con** : rag3db (Kuzu) ne supporte qu'une seule clé primaire. Il faudrait une clé composée `_key = "{index_name}/{file_name}"`.

### Recommandation

**Phase 1 : base64 dans STRING.** Ça marche immédiatement sans toucher à connection.rs. Les index en dev/test sont petits.

**Phase 2 : BLOB natif.** Ajouter `CypherValue::Blob(Vec<u8>)`, mapper le type BLOB de rag3db, et migrer la colonne `_data` de STRING à BLOB. Pour la production avec des index de taille conséquente.

---

## 5. Design CypherBlobStore

### Structure

```rust
use lucivy_core::blob_store::BlobStore;
// ou: use crate::blob_store::BlobStore; si on copie le trait

pub struct CypherBlobStore {
    /// Closure sync pour exécuter des queries Cypher.
    query_fn: Box<dyn Fn(&str, &[QueryParam]) -> Result<QueryResult, String> + Send + Sync>,
}
```

Alternative avec `Arc<dyn DbConnection>` + runtime :

```rust
pub struct CypherBlobStore {
    conn: Arc<dyn DbConnection>,
    rt: tokio::runtime::Handle,
}
```

### Table

```sql
CREATE NODE TABLE IF NOT EXISTS _index_blobs (
    _key STRING,
    _data STRING,
    PRIMARY KEY(_key)
)
```

### Implémentation BlobStore

```rust
impl BlobStore for CypherBlobStore {
    fn save(&self, index_name: &str, file_name: &str, data: &[u8]) -> io::Result<()> {
        let key = format!("{index_name}/{file_name}");
        let b64 = base64::encode(data);
        self.execute(
            "MERGE (b:_index_blobs {_key: $key}) SET b._data = $data",
            &[
                QueryParam::new("key", CypherValue::String(key)),
                QueryParam::new("data", CypherValue::String(b64)),
            ],
        )?;
        Ok(())
    }

    fn load(&self, index_name: &str, file_name: &str) -> io::Result<Vec<u8>> {
        let key = format!("{index_name}/{file_name}");
        let result = self.execute(
            "MATCH (b:_index_blobs {_key: $key}) RETURN b._data",
            &[QueryParam::new("key", CypherValue::String(key))],
        )?;
        match result.rows.first().and_then(|r| r.first()) {
            Some(CypherValue::String(b64)) => {
                base64::decode(b64).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            }
            _ => Err(io::Error::new(io::ErrorKind::NotFound, "blob not found")),
        }
    }

    fn delete(&self, index_name: &str, file_name: &str) -> io::Result<()> {
        let key = format!("{index_name}/{file_name}");
        self.execute(
            "MATCH (b:_index_blobs {_key: $key}) DELETE b",
            &[QueryParam::new("key", CypherValue::String(key))],
        )?;
        Ok(())
    }

    fn exists(&self, index_name: &str, file_name: &str) -> io::Result<bool> {
        let key = format!("{index_name}/{file_name}");
        let result = self.execute(
            "MATCH (b:_index_blobs {_key: $key}) RETURN count(b)",
            &[QueryParam::new("key", CypherValue::String(key))],
        )?;
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
        )?;
        Ok(result.rows.iter().filter_map(|r| {
            match r.first() {
                Some(CypherValue::String(key)) => Some(key.strip_prefix(&prefix)?.to_string()),
                _ => None,
            }
        }).collect())
    }
}
```

### Initialisation (dans Catalog::initialize)

Ajouter après la création des tables système :

```rust
// Create _index_blobs table if not exists
self.conn.execute(
    "CREATE NODE TABLE IF NOT EXISTS _index_blobs (_key STRING, _data STRING, PRIMARY KEY(_key))"
).await?;
```

---

## 6. Dépendance : quel trait BlobStore importer ?

Le trait `BlobStore` est défini dans **deux endroits** (copie identique) :
- `lucivy_core::blob_store::BlobStore`
- `sparse_vector::blob_store::BlobStore`

### Options

| Option | Pro | Con |
|--------|-----|-----|
| **A. Implémenter les deux** | Un seul CypherBlobStore implémente les deux traits | Même code, deux `impl` blocks — mais les traits sont identiques donc Rust ne permettra pas ça facilement |
| **B. Dépendre de lucivy_core seulement** | Un seul trait | sparse_vector ne reconnaît pas l'impl |
| **C. Créer un crate partagé `blob-store`** | Un seul trait, une seule impl | Nouveau crate, refactor des dépendances |
| **D. Copier le trait dans rag3weaver** | Indépendant | Troisième copie du même trait |
| **E. Wrapper générique** | `CypherBlobStore` n'implémente aucun trait, mais fournit les mêmes méthodes. On construit un adaptateur pour chaque crate | Flexible mais plus de boilerplate |

**Recommandation : Option C** à terme (crate `blob-store` partagé). **Pour l'instant : Option A** — le CypherBlobStore peut implémenter `lucivy_core::blob_store::BlobStore`, et pour sparse on utilise un wrapper/adaptateur simple puisque les signatures sont identiques.

Alternativement, sparse_vector pourrait dépendre de lucivy_core juste pour le trait BlobStore (dépendance légère, pas de fonctionnalités lourdes importées).

---

## 7. Questions ouvertes

1. **Sync/async bridge** : quelle approche choisir ? Runtime dédié (safe) vs block_on (simple) vs closure sync (flexible) ?

2. **Taille des blobs** : les index lucivy en production peuvent faire 100+ MB de segments. Base64 dans STRING double la pression mémoire (data + encoded). Faut-il implémenter BLOB natif avant de déployer en prod ?

3. **Transactions** : quand BlobDirectory fait `commit` (atomic_write meta.json + save segments), faut-il wrapper ça dans une transaction rag3db pour l'atomicité cross-fichiers ? Actuellement Kuzu ne supporte pas les transactions multi-statements.

4. **CREATE NODE TABLE IF NOT EXISTS** : cette syntaxe est-elle supportée par rag3db ? Sinon, il faut un try/catch ou une vérification préalable.

5. **GC des blobs orphelins** : si le process crash entre l'écriture d'un segment et la mise à jour de meta.json, des blobs orphelins restent dans `_index_blobs`. Faut-il un mécanisme de GC (lister les blobs, comparer avec meta.json) ?

6. **Crate partagé** : faut-il créer `blob-store` maintenant ou vivre avec deux copies identiques du trait pour l'instant ?

---

## 8. Ordre d'implémentation suggéré

```
1. Ajouter CypherValue::Blob(Vec<u8>) à connection.rs (optionnel, phase 2)

2. CypherBlobStore dans rag3weaver (phase 1 : base64 STRING)
   a. Struct + constructeur (prend Arc<dyn DbConnection>)
   b. Méthode ensure_table() pour créer _index_blobs si absent
   c. impl BlobStore (5 méthodes)
   d. Tests unitaires avec MockConnection ou MemBlobStore pattern

3. Intégration dans Catalog
   a. Catalog::initialize() crée _index_blobs
   b. Catalog expose un CypherBlobStore pour les extensions

4. Tests E2E
   a. CypherBlobStore roundtrip (save/load/delete/list)
   b. BlobDirectory + CypherBlobStore (lucivy persistence)
   c. SparseHandle + CypherBlobStore (sparse persistence)
   d. Full pipeline : register_entity → ingest → shutdown → reopen → search
```
