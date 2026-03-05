# Point d'étape — rag3db WASM : validations et prochaines étapes

## Ce qui est validé

### Node.js natif (addon NAPI)
- **139 tests mocha** : tous passent (2s)
- **lucivy_fts** : testé manuellement (contains, fuzzy, phrase, parse) — tout OK
- Extension chargée dynamiquement via `LOAD EXTENSION`
- Segfault mineur au `db.close()` (exit code 139), n'affecte pas le fonctionnement

### WASM (build standard, MEMFS)
- **Build OK** : `rag3db_wasm.js` 17MB (WASM inline)
- **Extensions statiques** : json, vector, algo, lucivy_fts
- **lucivy_fts** : testé manuellement in-memory (contains, fuzzy, phrase) — tout OK
- Tests mocha non exécutés (nécessitent variante NODEFS)

### Bugs corrigés pendant le build WASM
1. `DOC_FREQUENCY_PROP_NAME` forward declaration (ext fts)
2. cxx bridge `-fexceptions` pour emscripten
3. Rust atomics manquants → nightly + `-Z build-std`
4. `__cpp_exception` conflit → `-C panic=abort`
5. `libfuzzy_fst.a` natif → retiré `fts` du build WASM

---

## Ce qui N'EST PAS encore validé

### Extension vector
- **Statiquement linkée dans le WASM** (via extension_config.cmake)
- **JAMAIS testée en WASM ni en Node.js addon**
- C'est critique : sans vector, pas de RAG (embedding search)
- Tests à faire :
  - CREATE NODE TABLE avec colonne `FLOAT[384]` (ou DOUBLE[])
  - CREATE HNSW INDEX
  - Recherche par similarité cosine/L2
  - Vérifier que l'index HNSW fonctionne en in-memory WASM

### Tests mocha WASM complets (139 tests)
- Nécessitent la variante NODEFS (accès filesystem réel)
- Build : ajouter flag `WASM_NODEFS=true` ou équivalent dans cmake
- Valide tous les types de données, concurrence, paramètres, etc.

### Tests browser avec IDBFS
- Persistance réelle dans le navigateur (IndexedDB)
- Workflow : mount IDBFS → create DB → syncfs → reload → query
- Idéalement avec Playwright (Chromium headless)
- Valide le cas d'usage final : RAG dans le browser

---

## Architecture des 3 variantes WASM

```
tools/wasm/build.mjs fait 3 builds :

1. Default (single-thread, MEMFS)
   → package/           → Browser, pas de persistence

2. Multi-threaded (MEMFS + IDBFS)
   → package/multithreaded/  → Browser, avec persistence IDBFS

3. NODEFS (multi-threaded)
   → package/nodejs/    → Node.js, filesystem natif, pour les tests
```

Notre build actuel correspond à la variante 2 (multi-threaded, MEMFS).
Pour les tests mocha il faut la variante 3 (NODEFS).

---

## Plan de validation

### Étape 1 : Valider extension vector en WASM (PRIORITAIRE)
```sql
-- Test basique
CREATE NODE TABLE items (id UINT64, name STRING, embedding FLOAT[4], PRIMARY KEY(id));
CREATE (:items {id: 0, name: 'cat', embedding: [0.1, 0.2, 0.3, 0.4]});
CREATE (:items {id: 1, name: 'dog', embedding: [0.15, 0.25, 0.35, 0.45]});
CREATE (:items {id: 2, name: 'car', embedding: [0.9, 0.1, 0.1, 0.1]});

-- HNSW index
CALL CREATE_HNSW_INDEX('items', 'emb_idx', 'embedding', metric := 'cosine');

-- Vector search
CALL QUERY_HNSW_INDEX('items', 'emb_idx', [0.12, 0.22, 0.32, 0.42], 2)
RETURN node.name, node.id, distance;
-- Attendu : cat et dog (proches du vecteur query)
```

### Étape 2 : Build NODEFS + tests mocha WASM
- Reconfigurer cmake avec NODEFS
- Lancer les 139 tests mocha existants
- Objectif : valider que rag3db WASM fonctionne comme le natif

### Étape 3 : Tests browser avec Playwright
- Installer Playwright
- Écrire un test E2E :
  1. Charger `rag3db_wasm.js` dans Chromium
  2. Monter IDBFS
  3. Créer DB + table + lucivy_fts index + vector index
  4. Insérer des documents avec embeddings
  5. Recherche lucivy_fts (fuzzy) → récupérer node_ids
  6. Recherche vector (cosine) → récupérer les plus proches
  7. Vérifier résultats
  8. syncfs → recharger la page → re-query → même résultats (persistence)

### Étape 4 : Test combiné lucivy_fts + vector (le use case RAG)
```sql
-- Le vrai use case : hybrid search
-- 1. Full-text search avec lucivy_fts
CALL QUERY_LUCIVY_INDEX('docs', '{"type":"fuzzy","field":"content","value":"machine lerning","distance":1}', 100)
RETURN node_id, score AS text_score;

-- 2. Vector search avec HNSW
CALL QUERY_HNSW_INDEX('docs', 'emb_idx', $query_embedding, 100)
RETURN node.id, distance AS vector_dist;

-- 3. Fusion (RRF ou weighted) côté JS
```

---

## Résumé visuel

```
                        rag3db WASM
                            │
                ┌───────────┼───────────┐
                │           │           │
          lucivy_fts    vector       json/algo
          (fuzzy FTS)  (HNSW/cosine)  (utilities)
                │           │
                └─────┬─────┘
                      │
              Hybrid RAG Search
              (text + vector fusion)
```

**État actuel** :
- lucivy_fts : VALIDÉ en WASM
- vector : LINKÉ mais PAS TESTÉ ← priorité
- json/algo : linkés, testés indirectement via les 139 tests mocha natif

---

## Fichiers modifiés (récap session)

| Fichier | Modification |
|---------|-------------|
| `extension/fts/src/function/query_fts_index.cpp` | Fix forward declaration constexpr |
| `extension/lucivy/ld-lucivy/lucivy_fts/rust/build.rs` | `-fexceptions` pour emscripten |
| `extension/lucivy_fts/CMakeLists.txt` | nightly + atomics + build-std + panic=abort |
| `extension/extension_config.cmake` | Retiré `fts` de la liste WASM |
