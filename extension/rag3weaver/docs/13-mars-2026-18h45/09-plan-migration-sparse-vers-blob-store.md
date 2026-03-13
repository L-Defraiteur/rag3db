# Doc 09 — Plan de migration sparse vers BlobStore (Catalog intégration)

Date : 13 mars 2026

Ref : doc 08 (rapport BlobStore complet), `sparse_vector/rust/src/handle.rs`, `catalog.rs`, `search.rs`

## Objectif

Remplacer les appels à l'extension C++ `CREATE_SPARSE_VECTOR_INDEX` / `QUERY_SPARSE_VECTOR_INDEX` par l'usage direct du crate Rust `sparse_vector` avec `SparseHandle` + `CypherBlobStore`.

## Problème clé : bridge sync/async

### Le problème

`CypherBlobStore::from_connection()` crée un closure qui fait :
```rust
tokio::runtime::Handle::current().block_on(async { conn.execute_with_params(...).await })
```

`Handle::block_on()` **panic si appelé depuis un runtime tokio**. Or :
- `Catalog::initialize()` est `async` (dans un runtime tokio)
- `Catalog::drain()` est `async` (dataflow runtime)
- `SparseHandle::create_with_store()` appelle `commit()` → `BlobStore::save()` → le closure → **panic**

### Pourquoi ça ne pose pas problème avec `Rag3dbConnection`

`Rag3dbConnection` est **100% sync en interne**. Ses méthodes `async`:
```rust
async fn execute(&self, cypher: &str) -> Result<QueryResult, DbError> {
    self.query_sync(cypher)  // sync direct, pas de vrai await
}
```

Le `block_on` dans `from_connection` est un wrapper inutile quand le backend est sync.

### Solutions possibles

| Solution | Pros | Cons |
|----------|------|------|
| **A. Dedicated runtime** : créer un `tokio::Runtime` séparé dans CypherBlobStore | Marche toujours, thread-safe | Overhead d'un runtime supplémentaire, complexité |
| **B. `block_in_place`** : wrapper la closure avec `tokio::task::block_in_place(block_on(...))` | Simple | Ne fonctionne qu'avec multi-thread runtime, `block_in_place` ne supprime pas le contexte runtime |
| **C. QueryFn sync direct** : bypasser le trait `DbConnection` async, passer directement les méthodes sync | Zero overhead, pas de runtime bridge | Nécessite que le backend soit sync (OK pour Rag3dbConnection) |
| **D. `spawn_blocking`** aux call sites | Pas de changement au BlobStore | Chaque call site doit être adapté, rend le code plus complexe |

### Solution retenue : C — QueryFn sync direct

**Justification** : `Rag3dbConnection` est sync, WASM utilise `MemoryDirectory` (pas de BlobStore), il n'y a pas de backend async en vue. Pas de raison de payer le coût d'un bridge async→sync quand tout est déjà sync.

**Implémentation** : Ajouter une méthode `from_sync_connection` (ou modifier `from_connection`) qui prend une `Arc<Rag3dbConnection>` directement et appelle `query_with_params_sync` sans tokio :

```rust
impl CypherBlobStore {
    pub fn from_sync_connection(conn: Arc<Rag3dbConnection>) -> Self {
        let query_fn: QueryFn = Arc::new(move |cypher, params| {
            conn.query_with_params_sync(cypher, params)
                .map_err(|e| e.to_string())
        });
        Self { query_fn }
    }
}
```

**Alternative** : trait `SyncDbConnection` dans rag3weaver avec `execute_sync` / `execute_with_params_sync`, implémenté par `Rag3dbConnection`. Le `CypherBlobStore::from_connection` prend un `Arc<dyn SyncDbConnection>`.

**Point design** : Le `from_sync_connection` nécessite `rag3db-native` feature car il type directement sur `Rag3dbConnection`. Si on veut rester générique, le trait `SyncDbConnection` est préférable. Mais concrètement, le seul backend natif est `Rag3dbConnection`.

→ **Décision** : trait `SyncDbConnection` générique. Plus extensible, pas de `#[cfg(feature)]` nécessaire.

## Architecture cible

```
Catalog
  ├── blob_store: Arc<CypherBlobStore>        (déjà fait)
  ├── sparse_handles: HashMap<String, Arc<SparseHandle>>  (nouveau)
  │     key = "{table}:{indices_col}:{weights_col}"
  │     ou plus simplement key = index_name passé au BlobStore
  │
  ├── register_entity() / create_entity_tables()
  │     → SparseHandle::create_with_store(blob_store, index_name)
  │     → stocké dans sparse_handles
  │
  ├── initialize()
  │     → SparseHandle::open_with_store(blob_store, index_name)
  │     → pour chaque index sparse déjà enregistré
  │
  ├── drain() / EmbedNode
  │     → handle.insert(node_id, vector)
  │     → handle.commit() à la fin du batch
  │
  └── search()
        → handle.search(query_vector, limit)
        → plus besoin de QUERY_SPARSE_VECTOR_INDEX
```

## Points design à résoudre

### 1. Nommage des index dans le BlobStore

Le BlobStore utilise `(index_name, file_name)`. Quel `index_name` pour chaque sparse ?

Options :
- **A.** `"{entity}_Chunk_sparse"` pour simple pipeline, `"{kb}_Index_Chunk_sparse"` pour KB
- **B.** `"sparse:{entity}_Chunk"` / `"sparse:{kb}_Index_Chunk"`
- **C.** Utiliser le même nom que la table cible : `"{entity}_Chunk"` / `"{kb}_Index_Chunk"`

→ **Décision** : préfixe `"Sparse_"` appliqué par la lib sparse elle-même (constante `BLOB_PREFIX`). L'appelant passe `"Product_Chunk"`, le BlobStore voit `"Sparse_Product_Chunk"`. Zéro collision possible avec FTS ou autre, sans que l'appelant ait à y penser.

### 2. Colonnes sparse sur les chunk tables

Actuellement les sparse embeddings sont stockées en DEUX endroits :
1. Colonnes `sparse_indices INT64[]` + `sparse_weights DOUBLE[]` sur la chunk table
2. Index sparse de l'extension C++ (fichiers mmap)

Avec SparseHandle + BlobStore, l'index vit dans `_index_blobs`. Question : **garder les colonnes aussi ?**

| Approche | Pros | Cons |
|----------|------|------|
| **Garder les colonnes** | Backup, reindex possible, inspection SQL | Duplication, écriture double |
| **Supprimer les colonnes** | Plus simple, single source of truth | Reindex nécessite re-embed (lent) |

→ **Décision** : supprimer les colonnes. C'est un doublon — le SparseHandle via BlobStore est la seule source de vérité. Le seul cas d'usage des colonnes (rebuild sans re-embed) ne justifie pas la duplication. Le reindex en cas de corruption relance le modèle, cas rare et acceptable.

### 3. SparseVector type mismatch

- `rag3weaver::SparseVector` : `indices: Vec<u32>`, `values: Vec<f32>`
- `sparse_vector::SparseVector` : `indices: Vec<u32>`, `values: Vec<f32>`

Même structure mais types différents. Options :
- **A.** Convertir à chaque call site (simple cast, zero-cost)
- **B.** Re-exporter le type de sparse_vector dans rag3weaver
- **C.** Garder les deux, conversion triviale

→ Option A pour commencer. Conversion triviale :
```rust
let sv = sparse_vector::SparseVector::new(query.indices.clone(), query.values.clone());
```

### 4. Lifetime des handles

Les `SparseHandle` doivent vivre aussi longtemps que le Catalog. Au shutdown, les handles sont droppés → tmpdir cache nettoyé automatiquement.

- Pas besoin de `CLOSE_SPARSE_VECTOR_INDEX` — le Drop suffit
- Les handles sont `Arc<SparseHandle>` pour pouvoir être partagés avec les dataflow nodes

### 5. Commit timing

Quand appeler `handle.commit()` ?
- **Par insertion** : trop lent (flush mmap à chaque document)
- **Par batch** : à la fin de chaque `drain()`, après tous les inserts du batch
- **Lazy** : flag dirty, commit avant search (comme l'extension C++ actuelle)

→ **Décision** : noeud dataflow `SparseCommitNode`, même pattern que `FlushNode` pour FTS. Plaçable où on veut dans le graph. Appelle `handle.commit()` sur les handles dirty. Pas de lazy commit — on commit explicitement via le noeud.

### 6. Node ID mapping

L'extension C++ utilise les **offsets internes rag3db** comme node_id pour sparse. Le `SparseHandle` utilise `u64` comme node_id.

Actuellement `QUERY_SPARSE_VECTOR_INDEX` retourne des `node_id` = offsets rag3db, qui sont ensuite résolus via `WHERE ID(n) = offset`.

Avec le SparseHandle direct, on a deux options :
- **A.** Continuer à utiliser les offsets internes rag3db comme node_id
- **B.** Utiliser les UUIDs (string → hash → u64)

→ Option A pour compatibilité. Les offsets sont déjà disponibles dans le pipeline (InsertRecordNode retourne `ID(n)`).

## Plan d'implémentation

### Phase 1 : Fondation (ce qu'on fait maintenant)

1. **Fix sync/async** : `CypherBlobStore::from_sync_connection` ou trait `SyncDbConnection`
2. **Dépendance** : ajouter `sparse-vector` dans `rag3weaver/Cargo.toml`
3. **Catalog** : ajouter `sparse_handles: HashMap<String, Arc<SparseHandle>>`
4. **Création** : dans `create_entity_tables` / `create_kb_tables`, remplacer `CREATE_SPARSE_VECTOR_INDEX` par `SparseHandle::create_with_store`
5. **Réouverture** : dans `initialize()` / `load_entity_configs()`, `SparseHandle::open_with_store` pour les index existants

### Phase 2 : Recherche

6. **search.rs** : remplacer `search_sparse_cypher` (QUERY_SPARSE_VECTOR_INDEX) par `handle.search()` direct
7. **Catalog::search** : passer le handle au lieu de conn pour sparse

### Phase 3 : Insertion

8. **EmbedNode** : après insertion des colonnes sparse, aussi `handle.insert(node_id, vector)`
9. **FlushNode** ou post-drain : `handle.commit()` pour persister dans BlobStore
10. **DeleteRecordNode** : `handle.remove(node_id)` en plus de DELETE Cypher

### Phase 4 : Cleanup

11. Retirer les `CALL CREATE_SPARSE_VECTOR_INDEX` et `CALL QUERY_SPARSE_VECTOR_INDEX` (plus d'appels extension)
12. Optionnel : retirer les colonnes sparse des chunk tables (breaking change, pas urgent)

## Dépendances entre phases

```
Phase 1 (fondation) → Phase 2 (search) → Phase 4 (cleanup)
Phase 1 (fondation) → Phase 3 (insertion) → Phase 4 (cleanup)
```

Phases 2 et 3 sont indépendantes et peuvent être faites dans n'importe quel ordre.

## Risques

- **Offsets rag3db instables** : si rag3db réorganise les offsets internes (compaction), les node_id dans le sparse index deviennent invalides → nécessite reindex. Même risque qu'actuellement avec l'extension.
- **Concurrence** : `SparseHandle` est `Mutex<Inner>` — un seul writer à la fois. OK pour un Catalog mono-instance.
- **Taille blob** : pour de gros index (millions de docs), les 3 fichiers mmap peuvent être volumineux. Le BlobStore stocke/charge tout d'un coup. Pas de streaming. OK pour la taille actuelle des KBs.

## Question ouverte : préfixe BlobStore pour lucivy (FTS)

Sparse utilise le préfixe `"Sparse_"` (constante `BLOB_PREFIX` dans `sparse_vector/handle.rs`) appliqué automatiquement par la lib. L'appelant passe `"Product_Chunk"`, le BlobStore voit `"Sparse_Product_Chunk"`.

Quand on migrera lucivy (FTS) vers le BlobStore, il faudra décider :

- **Quel préfixe ?** `"Lucivy_"` / `"FTS_"` ? Appliquer la même convention que sparse (préfixe géré par la lib elle-même, transparent pour l'appelant).
- **Où le mettre ?** Le `BlobDirectory` dans `lucivy_core/src/blob_directory.rs` utilise déjà un `index_name` passé à l'ouverture. Faut-il ajouter un `BLOB_PREFIX` comme sparse, ou bien le préfixe est-il passé par l'appelant (rag3weaver) ?
- **Même `cache_base` ?** Le `BlobDirectory` a son propre mécanisme de cache tmpdir (`Arc<PathBuf>` + ref-counted cleanup). Faut-il l'aligner sur le pattern sparse (`{cache_base}/{pid}/{index_name}_{seq}/`) pour la cohérence, ou garder le pattern existant ?
- **Convention de nommage** : pour cohérence avec sparse, on pourrait avoir :
  - Sparse : `"Sparse_{table}"` (ex: `"Sparse_Product_Chunk"`)
  - FTS : `"Lucivy_{table}"` (ex: `"Lucivy_Product"`)
  - Ça garantit zéro collision dans `_index_blobs` même si les tables se chevauchent.

À investiguer dans lucivy_core avant la migration FTS.
