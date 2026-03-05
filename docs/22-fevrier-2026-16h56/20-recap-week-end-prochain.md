# 20 — Récap : deux chantiers pour le week-end prochain

Deux optimisations majeures à faire. Les deux suivent le même principe : rag3db a déjà les mécanismes (extensions vector et hooks), on ne les utilise juste pas encore.

---

## 1. ULTRA PRIO — Brancher HNSW au lieu du brute-force vector

**Le problème** : `search_vector()` dans `search.rs` fait un scan brute-force O(N) avec `array_cosine_similarity` sur TOUS les noeuds. C'est un full scan. Pour 50k docs ça prend ~25ms par query au lieu de <1ms.

**La solution existe déjà** : l'extension `vector` de rag3db a un index HNSW complet avec `CREATE_VECTOR_INDEX` / `QUERY_VECTOR_INDEX`. Support INSERT/DELETE incrémental, metric cosine. On l'utilise juste pas.

```cypher
-- AVANT (brute-force O(N)) :
MATCH (n:Document)
WHERE n.main_embedding IS NOT NULL
WITH n, array_cosine_similarity(n.main_embedding, $embedding) AS sim
ORDER BY sim DESC LIMIT 10
RETURN n._uuid, sim

-- APRÈS (HNSW O(log N)) :
CALL QUERY_VECTOR_INDEX('Document', 'doc_embedding_idx', $query_vec, 10)
RETURN node._uuid, distance
```

**Ce qu'il faut faire** :
1. `initialize()` / `generate_full_schema()` : ajouter `CALL CREATE_VECTOR_INDEX(table, index_name, col, metric := 'cosine')` pour chaque entité × KB qui a un embedding
2. `search_vector()` (`search.rs`) : remplacer le MATCH + array_cosine_similarity par `CALL QUERY_VECTOR_INDEX(...)`
3. Vérifier le support des filtres (WHERE clause ou post-filtre ?) dans `extension/vector/test/test_files/filter.test`

**Impact** : O(N) → O(log N). Pour 50k docs : ~25ms → <1ms.

**Fichiers** : `extension/vector/src/function/create_hnsw_index.cpp`, `query_hnsw_index.cpp`, tests dans `extension/vector/test/test_files/`

**Effort** : ~2-3h. C'est surtout du wiring Cypher, pas de code complexe.

---

## 2. Extension C++ `sparse_vector` (type lucivy_fts)

**Le problème** : le sparse index vit en mémoire dans rag3weaver. Pas de persistance (rebuild O(N) au cold start), pas de hooks INSERT/DELETE, pas de filtrage, ne scale pas au-delà de 100k docs.

**La solution** : une extension C++ avec le pattern exact de lucivy_fts. Le code Rust du SparseIndex existe déjà (~250 lignes), il suffit d'ajouter :
- Un cxx bridge (structs typés, ~15 fonctions)
- Un wrapper C++ avec hooks NodeTable + dirty_ flag + lazy commit
- Une persistance binaire simple (1 fichier)
- L'API Cypher : `CREATE_SPARSE_VECTOR_INDEX`, `QUERY_SPARSE_VECTOR_INDEX`, `DROP_SPARSE_VECTOR_INDEX`

**Architecture résumée** :
```
Extension C++ (sparse_vector)
  └─ SparseVectorIndex : storage::Index
       ├─ hooks INSERT/DELETE automatiques
       ├─ dirty_ + flushIfDirty() (lazy commit)
       └─ rust::Box<SparseHandle> via cxx bridge
              └─ SparseIndex existant (~250 LOC)
              └─ Persistance binaire (commit → fichier, open → reload)
```

**Gains** :
- Zéro rebuild O(N) après drain (hooks automatiques)
- Persistance native (cold start = lire 1 fichier, pas scanner toute la DB)
- Filtrage `allowed_ids` natif
- Base pour V2 on-disk + cache LRU (scaling > 1M docs)

**Détail complet** : doc 19

**Effort** : ~11-14h (un week-end)

---

## Ordre recommandé

1. **HNSW d'abord** (~2-3h) — gain immédiat énorme, peu de code, zéro risque
2. **Extension sparse ensuite** (~11-14h) — plus gros chantier, mais le pattern est connu (copier lucivy_fts)

Les deux sont indépendants.
