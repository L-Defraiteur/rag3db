# Doc 15 — Analyse : Sparse Index V2 + considérations cloud

Date : 12 mars 2026

Réf : doc 01 (11 mars §6), doc 03 (§2.9), doc 04 (§3.3, §4)

## 1. État actuel du sparse index

### 1.1 Architecture

```
Text → [DualEmbedder] → SparseVector { indices: Vec<u32>, values: Vec<f32> }
                              ↓
                    CALL QUERY_SPARSE_VECTOR_INDEX(table, indices, values, limit)
                              ↓
                    C++ SparseVectorIndex → Rust SparseHandle → SparseIndex::search()
                              ↓
                    Vec<(node_id, score)> → resolve chunks → UnifiedResult
```

### 1.2 Structures de données (Rust, `sparse_vector/rust/src/index.rs`)

```rust
pub struct SparseIndex {
    postings: HashMap<u32, Vec<(u64, f32)>>,  // token_id → [(node_id, weight)]
    vectors: HashMap<u64, SparseVector>,       // node_id → sparse vector (pour delete/update)
}

pub struct SparseVector {
    pub indices: Vec<u32>,   // token IDs (vocab ~30k-200k)
    pub values: Vec<f32>,    // poids (f32)
}
```

**Double stockage** : chaque document est dans `postings` (éclaté par token) ET dans `vectors` (copie complète). Le `vectors` sert uniquement au delete/update (pour savoir quels posting lists nettoyer).

### 1.3 Persistance : bincode full load/save

```rust
// open() — charge TOUT en RAM
let data = std::fs::read("sparse.bin")?;
let index: SparseIndex = bincode::deserialize(&data)?;

// commit() — sérialise TOUT d'un coup
let data = bincode::serialize(&index)?;
std::fs::write("sparse.bin", data)?;
```

- Pas de mmap, pas de WAL, pas d'écriture incrémentale
- Lazy commit via `dirty_` flag en C++ (flush avant query ou checkpoint)
- **Coût** : O(N) à chaque open ET chaque commit, où N = taille totale de l'index

### 1.4 Recherche : dot product sur posting lists

```rust
fn search(&self, query: &SparseVector, limit: usize) -> Vec<(u64, f32)> {
    let mut scores: HashMap<u64, f32> = HashMap::new();
    for (i, &token_id) in query.indices.iter().enumerate() {
        if let Some(postings) = self.postings.get(&token_id) {
            for &(doc_id, weight) in postings {
                *scores.entry(doc_id).or_default() += query.values[i] * weight;
            }
        }
    }
    // sort + truncate(limit)
}
```

**Complexité** : O(K × N_avg) où K = tokens dans la query (~10-100), N_avg = longueur moyenne d'une posting list.

### 1.5 Embedders supportés

| Embedder | Méthode | Sparsité typique |
|----------|---------|-----------------|
| BGE-M3 (`bge_m3_embedder.rs`) | Linear layer + ReLU, MAX par token | 10-100 tokens/doc |
| BM42/Candle (`bm42_embedder.rs`) | CLS attention weights, SUM par token | 5-50 tokens/doc |

Les deux supportent `DualEmbedder` (dense + sparse en un forward pass).

---

## 2. Le problème : pourquoi ça ne scale pas

### 2.1 Estimation mémoire

Hypothèse : 100k documents, BGE-M3, ~50 tokens non-zéro par doc en moyenne.

**postings** :
- 100k docs × 50 tokens = 5M entrées `(u64, f32)` = 5M × 12 bytes = **60 MB** (données brutes)
- HashMap overhead (~30-50%) → **~80-90 MB**

**vectors** :
- 100k docs × (50 × 4 bytes indices + 50 × 4 bytes values) = **40 MB**
- HashMap overhead → **~55 MB**

**Total RAM** : ~150 MB pour 100k docs. C'est gérable en local.

**À 1M docs** : ~1.5 GB RAM juste pour le sparse index. Plus le temps de serialize/deserialize qui passe à **plusieurs secondes** à chaque commit.

**À 10M docs** : ~15 GB. Inviable.

### 2.2 Coûts open/commit

| Docs | Taille sparse.bin (estimée) | Temps open() | Temps commit() |
|------|-----------------------------|-------------|----------------|
| 10k | ~15 MB | ~50ms | ~50ms |
| 100k | ~150 MB | ~500ms | ~500ms |
| 1M | ~1.5 GB | ~5s | ~5s |
| 10M | ~15 GB | ~50s+ | ~50s+ |

Chaque `drain()` → `commit()` = sérialise tout. Chaque réouverture de session = charge tout.

### 2.3 Pas de concurrence

Un seul `Mutex<SparseIndex>` protège tout. Pendant un `search()`, aucune écriture possible, et vice versa. Avec mmap on aurait read-only views parallèles.

---

## 3. Solutions possibles — V2 sparse

### 3.1 Option A : mmap + posting lists on-disk (recommandé docs précédents)

**Principe** : format binaire custom, posting lists stockées contiguës, accès via mmap.

**Format on-disk** :
```
[Header: magic, version, num_tokens, num_docs]
[Token directory: token_id → (offset, length) dans le fichier]
[Posting lists: séquences contiguës de (doc_id: u64, weight: f32)]
[Vectors section: doc_id → (offset, length) pour les SparseVector complètes]
```

**Accès** :
- `open()` = mmap le fichier, lire le header + token directory en RAM (petit)
- `search()` = pour chaque token de la query, seek direct dans le fichier mmappé → lire la posting list
- `commit()` = écriture incrémentale des nouvelles entrées + compaction périodique

**Avantages** :
- Open() quasi instantané (mmap, pas de deserialize)
- RAM = seulement le working set (pages touchées), géré par l'OS
- Lecture parallèle naturelle (mmap = read-only concurrent)

**Inconvénients** :
- Format custom = complexe à implémenter (corruption, versioning)
- Compaction nécessaire (les deletes laissent des trous)
- WASM : mmap pas supporté (fallback nécessaire)

**Effort estimé** : 3-5 jours (doc 01)

### 3.2 Option B : posting lists dans rag3db (Cypher/Kuzu tables)

**Principe** : stocker les posting lists comme des node tables Kuzu.

```
Table _SparsePosting_{entity} (token_id: INT64, doc_id: INT64, weight: DOUBLE)
  → Index sur token_id pour lookup rapide
```

**Search** = `MATCH (p:_SparsePosting_Product) WHERE p.token_id IN [123, 456, 789] RETURN p.doc_id, SUM(p.weight * $query_weight) AS score ORDER BY score DESC LIMIT 10`

**Avantages** :
- Zéro code de persistance custom (Kuzu gère tout : WAL, crash recovery, mmap interne)
- Backup/restore = backup la DB, tout vient avec
- Requêtes filtrées = jointure Cypher naturelle avec les autres tables

**Inconvénients** :
- Performance : Kuzu n'est pas optimisé pour ce pattern (scan de posting lists + agrégation). Benchmark nécessaire.
- Overhead Cypher : parse + plan + execute vs. accès direct HashMap
- Pas transférable à pgvector (point 2 de ta question initiale)

**Effort estimé** : 2-3 jours

### 3.3 Option C : backend de stockage externe (RocksDB / LMDB)

**Principe** : key-value store optimisé pour les accès disque.

```
Key: token_id (u32, big-endian)
Value: Vec<(doc_id: u64, weight: f32)> sérialisé (bincode ou custom)
```

**Avantages** :
- Éprouvé (RocksDB = LSM-tree, LMDB = B-tree + mmap)
- Write-ahead log, compaction, snapshots gratuits
- Bonnes perfs en lecture concurrente (LMDB) ou écriture intensive (RocksDB)
- Petite empreinte RAM (cache configurable)

**Inconvénients** :
- Dépendance lourde (RocksDB = ~10MB de lib, compile time significatif)
- LMDB plus léger mais limite de taille DB (2GB par défaut, configurable)
- WASM : ni RocksDB ni LMDB ne compilent en WASM
- Pas de gain conceptuel vs. Option A (on échange notre format custom contre leur format custom)

**Effort estimé** : 2-4 jours

### 3.4 Option D : chez le voisin (pgvector / Supabase)

**Principe** : dans un scénario cloud avec pgvector, les sparse vectors seraient stockées dans Postgres.

```sql
-- Extension pgvector ne supporte PAS nativement le sparse. Mais :
CREATE TABLE sparse_postings (
    token_id INT,
    doc_id BIGINT,
    weight REAL,
    PRIMARY KEY (token_id, doc_id)
);
CREATE INDEX ON sparse_postings (token_id);

-- Search
SELECT doc_id, SUM(weight * query_weight) as score
FROM sparse_postings
WHERE token_id = ANY($1)
GROUP BY doc_id
ORDER BY score DESC
LIMIT $2;
```

Ou avec l'extension `pgvector` 0.8+ qui supporte `sparsevec` :
```sql
-- pgvector sparse support (depuis 0.8)
ALTER TABLE chunks ADD COLUMN sparse_embedding sparsevec(30522);
-- Mais : pas d'index HNSW sur sparsevec, seulement scan séquentiel
-- → inutilisable à grande échelle pour la recherche
```

**Réalité pgvector sparse** : `sparsevec` existe mais il n'y a **pas d'index inversé** dessus. pgvector fait du scan séquentiel + distance cosine/dot sur les sparse vectors. C'est pire que notre HashMap actuel.

**Conclusion** : pour un backend Postgres/Supabase, l'option posting lists en table SQL (pas pgvector sparsevec) est la bonne approche. C'est exactement le pattern Option B mais en SQL au lieu de Cypher.

---

## 4. Comparaison

| Critère | A (mmap) | B (Kuzu tables) | C (RocksDB) | D (Postgres) |
|---------|----------|-----------------|-------------|--------------|
| Perf search | ★★★★★ | ★★★ | ★★★★ | ★★★ |
| Perf write | ★★★★ | ★★★★ | ★★★★★ | ★★★★ |
| Open() speed | ★★★★★ | ★★★★★ | ★★★★★ | ★★★★★ |
| Implem effort | ★★ | ★★★★ | ★★★ | ★★★★ |
| WASM compat | ★★ (fallback) | ★★★★★ | ★ | N/A (cloud) |
| Cloud-ready | ★★ | ★★★ | ★★ | ★★★★★ |
| Maintenance | ★★ | ★★★★★ | ★★★ | ★★★★★ |
| Backup/restore | ★★ | ★★★★★ | ★★★ | ★★★★★ |

## 5. Recommandation

### Court terme : Option B (Kuzu tables)

**Pourquoi** :
- Zéro nouvelle dépendance, zéro format custom
- La persistance est déjà gérée (WAL, checkpoint, crash recovery)
- Cohérent avec le reste de l'architecture (tout passe par la DB)
- Facile à implémenter : on remplace `SparseIndex::search()` par un `MATCH ... WHERE token_id IN [...] RETURN ...`
- Le pattern est **identique** à ce qu'on ferait en SQL pour un backend Postgres → le code de conversion Kuzu→Postgres sera minimal

**Trade-off perf** : probablement 2-5x plus lent que HashMap in-memory pour la recherche pure. Mais :
1. Le sparse search n'est qu'un signal parmi 3 (BM25 + vector + sparse) → l'impact sur le temps total est dilué
2. On élimine le coût O(N) de open/commit qui est bien pire à grande échelle
3. On peut toujours ajouter un LRU cache in-memory des posting lists chaudes

**Migration** : l'interface `SparseHandle` (create/open/insert/delete/search/commit) ne change pas côté C++. On remplace l'implémentation Rust interne.

### Moyen terme : variante SQL pour le backend Postgres/Supabase

Quand on aura un backend Postgres, le `SparseSearchNode` alternatif fera :
```sql
SELECT doc_id, SUM(p.weight * v.query_weight) as score
FROM sparse_postings p
JOIN unnest($token_ids, $query_weights) AS v(token_id, query_weight)
  ON p.token_id = v.token_id
GROUP BY p.doc_id
ORDER BY score DESC
LIMIT $limit;
```

C'est exactement le même algorithme (dot product sur posting lists inversées) mais en SQL. Le `SparseSearchNode` Postgres sera ~50 lignes.

### Ce qu'on ne fait PAS

- **mmap custom** (Option A) : trop de code à maintenir pour un gain marginal vs. Kuzu tables
- **RocksDB** (Option C) : dépendance lourde, pas WASM-compatible, gain discutable
- **pgvector sparsevec** : pas d'index inversé, scan séquentiel → inutilisable

---

## 6. Plan d'implémentation Option B (si on y va)

### Étape 1 : Tables posting lists dans Kuzu (~0.5j)

À `register_entity()` (si signal SPARSE) :
```cypher
CREATE NODE TABLE IF NOT EXISTS _SparsePosting_{entity_chunk} (
    _id SERIAL PRIMARY KEY,
    _token_id INT64,
    _doc_id INT64,
    _weight DOUBLE
)
```

Note : `_doc_id` = l'offset interne Kuzu du chunk node (ce qu'on passe déjà à l'index sparse actuel).

### Étape 2 : Remplacement insert/delete/search (~1.5j)

**Insert** (dans EmbedNode, après embedding) :
```cypher
UNWIND $items AS item
CREATE (p:_SparsePosting_{chunk_table} {_token_id: item.token_id, _doc_id: item.doc_id, _weight: item.weight})
```

**Delete** (dans RechunkDeleteNode ou DeleteRecordNode) :
```cypher
MATCH (p:_SparsePosting_{chunk_table}) WHERE p._doc_id IN $doc_ids
DELETE p
```

**Search** :
```cypher
MATCH (p:_SparsePosting_{chunk_table})
WHERE p._token_id IN $token_ids
WITH p._doc_id AS doc_id,
     SUM(p._weight * CASE p._token_id WHEN $t0 THEN $w0 WHEN $t1 THEN $w1 ... END) AS score
ORDER BY score DESC
LIMIT $limit
RETURN doc_id, score
```

Ou plus proprement via UNWIND + join si Kuzu le supporte.

### Étape 3 : Retirer l'extension sparse_vector C++/Rust (~0.5j)

- Plus besoin de `CREATE_SPARSE_VECTOR_INDEX`, `QUERY_SPARSE_VECTOR_INDEX`, `DROP_SPARSE_VECTOR_INDEX`
- Plus de `sparse.bin`, plus de bincode, plus de `SparseHandle`
- Le sparse devient une feature pure Kuzu (tables normales)

### Étape 4 : Benchmark (~0.5j)

- Comparer search latency : ancienne (HashMap) vs. nouvelle (Kuzu tables) sur 1k, 10k, 100k docs
- Si trop lent : ajouter un index Kuzu sur `_token_id` ou un cache in-memory des top-K tokens

**Effort total estimé : 3 jours**

---

## 7. Impact sur le point 2 (variante Supabase/pgvector)

Le passage à Option B (posting lists en tables) rend la variante Postgres triviale :
- Même schéma, même queries, juste SQL au lieu de Cypher
- Le `SparseSearchNode` Postgres = copier-coller du Kuzu avec `$1` au lieu de `$items`
- Pas besoin de pgvector pour le sparse — des tables SQL standard suffisent

Le vrai travail pour Supabase/pgvector concerne les **16 autres nœuds** qui parlent Cypher (doc 04 §2.2). Le sparse est le plus facile à porter car c'est un index inversé pur, sans relations graph.

---

## 8. Questions ouvertes

1. **Kuzu supporte-t-il un index sur une propriété non-PK ?** Si oui, `CREATE INDEX ON _SparsePosting(token_id)` accélérerait énormément le lookup. Si non, scan séquentiel = potentiellement lent.

2. **Taille des posting lists** : avec BGE-M3 (vocab 250k tokens), la table `_SparsePosting` aura ~50 × N_docs rows. Pour 100k docs = 5M rows. Kuzu gère ça sans problème en lecture, mais les JOINs avec UNWIND restent à benchmarker.

3. **WASM** : le sparse actuel (bincode in-memory) fonctionne en WASM. L'Option B (Kuzu tables) aussi puisque Kuzu tourne déjà en WASM. Pas de régression.

4. **Sub-word merging** : toujours pertinent quelle que soit l'option. Réduirait le nombre de posting list entries de ~30-50%. Orthogonal au stockage.
