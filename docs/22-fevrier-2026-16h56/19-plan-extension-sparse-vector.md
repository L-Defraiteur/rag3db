# 19 — Plan : extension C++ sparse_vector (type tantivy_fts)

## Contexte

Le sparse index vit actuellement en mémoire dans rag3weaver (`sparse_index.rs`, ~250 lignes). C'est un index inversé HashMap basique : `token_id → [(uuid, weight)]`. Il fonctionne mais a des limites fondamentales :

1. **Pas de persistance** — rebuild complet depuis les colonnes DB au cold start (O(N) scan)
2. **Pas de hooks** — rag3weaver est un client Cypher, pas une extension. Pas d'INSERT/DELETE hooks automatiques. Le rebuild après `drain()` relit TOUT.
3. **Mémoire** — ~60 MB pour 100k docs, ~600 MB pour 1M docs. Ne scale pas au-delà.
4. **Pas de filtrage natif** — `search_sparse()` ignore les filtres, pas d'`allowed_ids`

Au lieu de patcher ces problèmes un par un (concessions 2A-2E du doc 17), on fait une vraie extension C++ avec le code Rust existant réutilisé via cxx bridge. Même pattern que tantivy_fts.

## Pourquoi une extension plutôt que continuer en in-memory

| Aspect | In-memory rag3weaver | Extension rag3db |
|---|---|---|
| Persistance | Colonnes DB → rebuild O(N) | Fichier dédié, chargé direct |
| Hooks INSERT/DELETE | Aucun (drain + rebuild) | Automatiques via NodeTable |
| Lazy commit | N/A | `dirty_` flag comme tantivy_fts |
| Filtrage | Pas d'accès aux attributs | `allowed_ids` natif |
| Mémoire | Tout en RAM | Index on-disk + cache LRU (futur) |
| WASM | Fonctionne (tout en Rust) | Fonctionne (même build que tantivy_fts) |
| API | Code Rust interne, opaque | Cypher : CREATE/QUERY/DROP |

**Argument décisif** : toutes les optimisations qu'on voulait faire (2A rebuild incrémental, 2C filtrage, cache) reviennent à réimplémenter les mécanismes qu'une extension fournit nativement (hooks, persistance, accès storage). Autant le faire proprement une fois.

## Architecture cible

```
┌─────────────────────────────────────────────────────┐
│ Extension C++ : sparse_vector                        │
│                                                       │
│  SparseVectorIndex : public storage::Index           │
│    ├─ rust::Box<SparseHandle> handle_                │
│    ├─ dirty_ flag                                    │
│    ├─ insert() hook   → add_document()  → dirty_=1  │
│    ├─ delete_() hook  → remove()        → dirty_=1  │
│    ├─ flushIfDirty()  → commit+reload                │
│    └─ checkpointInMemory()                           │
│                                                       │
│  Functions Cypher :                                  │
│    CREATE_SPARSE_VECTOR_INDEX(table, col_indices,    │
│      col_weights, [metric := 'dot'])                 │
│    QUERY_SPARSE_VECTOR_INDEX(table, $query_indices,  │
│      $query_weights, k, [allowed_ids])               │
│    DROP_SPARSE_VECTOR_INDEX(table)                   │
│                                                       │
│  Catalog entry : SparseVectorAuxInfo                 │
│    { indexPath, colIndices, colWeights, metric }      │
└──────────────┬──────────────────────────────────────┘
               │ cxx bridge
┌──────────────▼──────────────────────────────────────┐
│ Rust crate : sparse_fts (dans ld-tantivy ou séparé) │
│                                                       │
│  SparseHandle {                                      │
│    index: SparseIndex,          ← code existant !    │
│    path: PathBuf,                                    │
│    dirty: bool,                                      │
│  }                                                   │
│                                                       │
│  Bridge functions :                                  │
│    create_index(path, config_json) → Box<Handle>     │
│    open_index(path)                → Box<Handle>     │
│    add_document(handle, node_id, indices, weights)   │
│    remove_document(handle, node_id)                  │
│    search(handle, q_indices, q_weights, k)           │
│    search_filtered(handle, q, k, allowed_ids)        │
│    commit(handle)                                    │
│    reload(handle)  ← no-op pour in-memory v1         │
│    num_docs(handle) → u64                            │
│                                                       │
│  SparseIndex (réutilisé tel quel de rag3weaver)      │
│    ~250 lignes, HashMap posting lists                │
│    insert, remove, search, clear                     │
└─────────────────────────────────────────────────────┘
```

## Ce qui existe déjà (réutilisable)

### Code Rust existant (`rag3weaver/src/sparse_index.rs`)

Le `SparseIndex` et `SparseVector` sont prêts :
- `SparseVector` : `indices: Vec<u32>`, `values: Vec<f32>`, + constructeur + tests
- `SparseIndex` : `postings: HashMap<u32, Vec<(String, f32)>>`, `vectors: HashMap<String, SparseVector>`
- `insert(uuid, vector)`, `remove(uuid)`, `search(query, limit)`, `clear()`
- 9 tests unitaires

On les copie dans le nouveau crate sparse_fts (ou on les importe si on veut garder une seule source).

### Pattern C++ de tantivy_fts (à reproduire)

Le squelette C++ est identique — on copie la structure tantivy_fts et on remplace :

| tantivy_fts | sparse_vector |
|---|---|
| `TantivyIndex` | `SparseVectorIndex` |
| `TantivyHandle` | `SparseHandle` |
| `add_document_texts()` | `add_document_sparse()` |
| `search_with_highlights()` | `search_sparse()` |
| `DocFieldText` | `DocSparse { indices: Vec<u32>, weights: Vec<f32> }` |
| `SearchResultWithHighlights` | `SparseSearchResult { node_id: u64, score: f32 }` |
| textes multi-champs | 1 paire de colonnes (indices + weights) |

## Plan d'implémentation détaillé

### Phase 1 : Crate Rust `sparse_fts` + cxx bridge

**Fichiers à créer** dans `extension/tantivy/ld-tantivy/sparse_fts/` (ou `extension/sparse_vector/sparse_fts/`) :

```
sparse_fts/
├── rust/
│   ├── Cargo.toml        (crate-type = ["staticlib"], deps: cxx)
│   ├── build.rs          (cxx_build::bridge("src/bridge.rs"))
│   └── src/
│       ├── lib.rs
│       ├── bridge.rs     (cxx bridge — structs + fonctions)
│       ├── handle.rs     (SparseHandle — lifecycle + persistence)
│       └── index.rs      (copie de sparse_index.rs existant)
└── include/
    └── sparse_fts/
        └── rust/         (headers générés par cxx)
```

**`bridge.rs`** — Interface cxx :

```rust
#[cxx::bridge]
mod ffi {
    struct SparseSearchResult {
        node_id: u64,
        score: f32,
    }

    extern "Rust" {
        type SparseHandle;

        fn create_sparse_index(path: &str, config_json: &str) -> Result<Box<SparseHandle>>;
        fn open_sparse_index(path: &str) -> Result<Box<SparseHandle>>;

        fn add_document(handle: &SparseHandle, node_id: u64,
                       indices: &[u32], weights: &[f32]) -> i64;
        fn delete_document(handle: &SparseHandle, node_id: u64) -> i64;

        fn search(handle: &SparseHandle, query_indices: &[u32],
                 query_weights: &[f32], limit: u32) -> Vec<SparseSearchResult>;
        fn search_filtered(handle: &SparseHandle, query_indices: &[u32],
                          query_weights: &[f32], limit: u32,
                          allowed_ids: &[u64]) -> Vec<SparseSearchResult>;

        fn commit(handle: &SparseHandle) -> i64;
        fn num_docs(handle: &SparseHandle) -> u64;
    }
}
```

**`handle.rs`** — Persistence + lifecycle :

```rust
pub struct SparseHandle {
    index: Mutex<SparseIndex>,         // L'index en mémoire (v1)
    id_map: Mutex<BiMap<u64, String>>, // node_id ↔ uuid mapping
    path: PathBuf,                     // Dossier de persistance
}
```

**Persistance V1 (simple)** : sérialiser l'index en binaire (bincode ou custom) dans `{path}/sparse.bin`. Le `commit()` écrit sur disque, `open()` relit. C'est mieux que le scan O(N) de colonnes DB parce que :
- Le format binaire est compact (pas de parsing Cypher)
- La lecture est O(1) (mmap ou read complet), pas O(N) queries
- Le fichier ne contient que l'index inversé, pas les données brutes

**Persistance V2 (futur)** : format on-disk par token avec mmap + cache LRU. Pas pour la V1 de l'extension.

### Phase 2 : Extension C++

**Fichiers à créer** dans `extension/sparse_vector/src/` :

```
extension/sparse_vector/
├── CMakeLists.txt
├── src/
│   ├── main/
│   │   └── sparse_vector_extension.cpp/.h   (load, init, extern "C")
│   ├── function/
│   │   ├── create_sparse_vector_index.cpp/.h
│   │   ├── query_sparse_vector_index.cpp/.h
│   │   └── drop_sparse_vector_index.cpp/.h
│   ├── index/
│   │   └── sparse_vector_index.cpp/.h       (hooks, dirty_, flush)
│   ├── catalog/
│   │   └── sparse_vector_catalog_entry.cpp/.h
│   └── include/
│       └── ... (headers)
└── test/
    └── sparse_vector_test.cpp               (GTest E2E)
```

**`SparseVectorIndex`** (le coeur) :

```cpp
class SparseVectorIndex final : public storage::Index {
private:
    rust::Box<SparseHandle> handle_;
    uint32_t indicesPropertyID_;   // ID colonne sparse_indices
    uint32_t weightsPropertyID_;   // ID colonne sparse_weights
    bool dirty_ = false;

public:
    // Hook INSERT : extrait indices+weights des ValueVectors, appelle add_document
    void insert(Transaction*, const ValueVector& nodeIDVector,
                const std::vector<ValueVector*>& propertyVectors, InsertState&) override {
        for (auto idx : sel) {
            auto nodeID = nodeIDVector.getValue<internalID_t>(idx).offset;
            auto& indicesVec = ...; // extraire INT64[] → Vec<u32>
            auto& weightsVec = ...; // extraire DOUBLE[] → Vec<f32>
            add_document(*handle_, nodeID, indices, weights);
        }
        dirty_ = true;
    }

    // Hook DELETE
    void delete_(Transaction*, const ValueVector& nodeIDVector, DeleteState&) override {
        for (auto nodeID : ...) {
            delete_document(*handle_, nodeID);
        }
        dirty_ = true;
    }

    // Lazy commit (appelé avant QUERY)
    void flushIfDirty() {
        if (!dirty_) return;
        commit(*handle_);
        dirty_ = false;
    }
};
```

**`CREATE_SPARSE_VECTOR_INDEX`** :

```cypher
CALL CREATE_SPARSE_VECTOR_INDEX('Document', 'main_sparse_indices', 'main_sparse_weights')
```

Flow :
1. Valider que les colonnes existent (INT64[] et DOUBLE[])
2. Créer le dossier `sparse_indexes/{tableName}/`
3. Appeler `create_sparse_index(path, config)`
4. Scanner la table, insérer tous les documents existants
5. Commit
6. Enregistrer dans le catalogue + `nodeTable.addIndex()`

**`QUERY_SPARSE_VECTOR_INDEX`** :

```cypher
CALL QUERY_SPARSE_VECTOR_INDEX('Document', $query_indices, $query_weights, 10)
RETURN node._uuid, score
-- ou avec filtre :
CALL QUERY_SPARSE_VECTOR_INDEX('Document', $query_indices, $query_weights, 10,
     allowed_ids := $ids)
RETURN node._uuid, score
```

Flow :
1. `flushIfDirty()`
2. Appeler `search()` ou `search_filtered()` via cxx bridge
3. Retourner `(node_id, score)` comme TableFunction

### Phase 3 : Intégration rag3weaver

Modifier `search.rs` et `catalog.rs` pour utiliser l'extension au lieu du SparseIndex in-memory :

**Avant** (rag3weaver gère tout) :
```
create() → enqueue SparseEmbedOp → drain() → SparseEmbedProcessor store en DB → rebuild_sparse_index (scan tout)
search() → sparse_indexes.get(kb).search(query, k) ← in-memory
```

**Après** (extension gère l'index) :
```
create() → enqueue SparseEmbedOp → drain() → SparseEmbedProcessor store en DB → hooks extension mettent à jour l'index automatiquement
search() → CALL QUERY_SPARSE_VECTOR_INDEX(table, $q_indices, $q_weights, k) ← via Cypher
```

Changements :
- **Supprimer** : `sparse_indexes: HashMap<String, SparseIndex>` du Catalog
- **Supprimer** : `rebuild_sparse_index()` — plus besoin, les hooks gèrent
- **Modifier** : `search()` dans catalog.rs — appeler `QUERY_SPARSE_VECTOR_INDEX` via Cypher au lieu de `sparse_index.search()`
- **Modifier** : `initialize()` — ajouter `CALL CREATE_SPARSE_VECTOR_INDEX(...)` dans la génération de schéma (comme on fera pour HNSW)
- **Garder** : `SparseEmbedProcessor` — il stocke toujours les colonnes en DB (les hooks de l'extension lisent ces colonnes)
- **Garder** : `SparseEmbedder` trait, `SparseEmbedOp`, tout le pipeline embed

## Mapping node_id ↔ uuid

Subtilité importante : l'extension travaille avec des `node_id` (offsets internes rag3db), mais rag3weaver travaille avec des `_uuid` (strings).

**Solution** : le `SparseHandle` maintient un `BiMap<u64, String>` (node_id ↔ uuid).

- `add_document(node_id, indices, weights)` → le hook C++ extrait aussi `_uuid` de la row, le passe au Rust
- `search()` retourne des `node_id` → le C++ de QUERY_SPARSE_VECTOR_INDEX fait le mapping vers les nodes rag3db (comme QUERY_VECTOR_INDEX le fait déjà)

Alternativement, on peut travailler uniquement en `node_id` côté extension et laisser le caller résoudre. C'est ce que fait `QUERY_VECTOR_INDEX` — il retourne directement un `node` pas un uuid.

## Différences clés vs tantivy_fts

| Aspect | tantivy_fts | sparse_vector |
|---|---|---|
| **Backend Rust** | Tantivy (lib externe, 50k LOC) | SparseIndex (~250 LOC, code maison) |
| **Complexité schema** | Multi-champ, stemmer, ngrams, tokenizers | 1 paire de colonnes (indices + weights) |
| **Persistance** | Segments Tantivy (fichiers multiples) | Binaire simple (1 fichier) |
| **Query** | JSON DSL complexe (term, fuzzy, phrase, boolean...) | Juste un vecteur sparse (indices + weights) |
| **Highlights** | Oui (byte offsets) | Non (scores seulement) |
| **Taille code** | ~800 lignes Rust + ~1500 C++ | ~400 lignes Rust + ~800 C++ (estimé) |

Le sparse est beaucoup plus simple que tantivy_fts. Moins de code, moins de configuration, moins de types. L'essentiel c'est les hooks + persistance + filtrage.

## Format de persistance V1

```
sparse_indexes/{tableName}/
├── sparse.bin          (index sérialisé)
└── config.json         (métadonnées)
```

**`sparse.bin`** — format binaire custom (ou bincode) :
```
[magic: 4 bytes "SPRS"]
[version: u32]
[num_docs: u64]
[num_tokens: u64]
[postings section]
  for each token_id:
    [token_id: u32]
    [num_entries: u32]
    [entries: (node_id: u64, weight: f32) × num_entries]
[vectors section]  (pour delete — reconstruire quel doc a quels tokens)
  for each doc:
    [node_id: u64]
    [nnz: u32]
    [indices: u32 × nnz]
    [values: f32 × nnz]
```

Pour la V1, on peut juste utiliser bincode/serde et optimiser le format plus tard.

## Estimation de travail

| Phase | Effort | Détail |
|---|---|---|
| Phase 1 : Crate Rust + bridge | ~3-4h | Copier sparse_index.rs, écrire handle.rs + bridge.rs, persistence bincode |
| Phase 2 : Extension C++ | ~4-5h | Copier structure tantivy_fts, adapter pour sparse, hooks, catalog entry |
| Phase 3 : Tests GTest E2E | ~2h | CREATE + INSERT + QUERY, DELETE + re-QUERY, persistence, allowed_ids |
| Phase 4 : Intégration rag3weaver | ~2h | Supprimer in-memory, wiring Cypher QUERY_SPARSE_VECTOR_INDEX |
| **Total** | **~11-14h** | Un week-end |

## Évolution future (V2+)

1. **Format on-disk par token** : Au lieu de tout charger en mémoire, stocker les posting lists par token sur disque (mmapped). Cache LRU pour les tokens hot. Permet de scaler au-delà de 1M docs sans tout garder en RAM.

2. **Quantization** : Les weights `f32` pourraient être quantisés en `u8` (256 niveaux) pour 4x moins de mémoire/disque.

3. **Posting list compression** : Delta encoding + varint pour les node_ids triés. Divise la taille par ~3 pour les longues posting lists.

4. **Batch search** : `QUERY_SPARSE_VECTOR_INDEX` avec plusieurs queries en batch (pour les cas multi-KB).

## Résumé

Le code Rust du sparse index existe déjà et marche. Ce qu'il manque c'est l'infra extension (hooks, persistance, API Cypher, filtrage). On copie le pattern tantivy_fts — c'est le même squelette C++ avec un backend Rust plus simple. L'effort est ~1 week-end. Le gain : zéro rebuild O(N), hooks INSERT/DELETE automatiques, persistance native, filtrage `allowed_ids`, et une base solide pour le scaling V2 (on-disk + LRU).

---

**Statut : À implémenter le week-end prochain.**
