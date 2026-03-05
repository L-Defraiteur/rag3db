# Rapport de fin de session — 14 février 2026, ~05h30

## Ce qui a été fait cette session

### 1. Test de persistance (FAIT)
- Test GTest `LucivyPersistenceTest` : create DB file-mode → table + docs + index → query OK → close DB → reopen → query = mêmes résultats
- Crée sa propre DB à un temp path (indépendant du fixture ApiTest) pour garantir le mode fichier même quand `IN_MEM_MODE=true`

### 2. Filtrage par node IDs (FAIT)
- Paramètre optionnel `allowed_ids` sur `QUERY_LUCIVY_INDEX`
- Syntaxe : `allowed_ids := [CAST(0, 'UINT64'), CAST(2, 'UINT64')]`
- Câblage de `search_filtered_with_highlights` (déjà existant côté Rust) dans `query_lucivy_index.cpp`
- Test `LucivyFilteredSearchTest` : 4 scénarios (aucun filtre, un ID, ID sans match, plusieurs IDs)

### 3. Filter fields natifs v2 (FAIT)
- Colonnes non-texte (INT64, DOUBLE, etc.) indexées dans Lucivy pour filtrage **pendant** le scoring
- Syntaxe CREATE : `filter_fields := ['category', 'score']`
- Syntaxe QUERY (dans le JSON) : `"filters":[{"field":"category","op":"eq","value":1}]`
- 7 opérateurs : eq, ne, lt, lte, gt, gte, in
- Fichiers modifiés :
  - **Rust** : `query.rs` (FilterClause, build_filter_clause, json_to_term), `handle.rs` (ajout type f64 manquant)
  - **C++** : `create_lucivy_index.cpp` (FilterFieldInfo, mapLogicalTypeToLucivy, resolveFilterFields, add_document_mixed), `lucivy_index.h/cpp` (fieldTypes_, hasMixedFields_)
- Test `LucivyFilterFieldsTest` : eq, gt, gte, filtres multiples combinés

### 4. Update support — initié mais incomplet (EN COURS)

#### Ce qui est fait
- `initUpdateState()` et `update()` implémentés dans `LucivyIndex`
  - Pattern FTS : delete old doc → re-scan toutes les colonnes → remplacer la colonne modifiée → re-insert
  - `LucivyUpdateState` stocke : updatedColumnIdx, deleteState, storageManager, memoryManager
- Tests `LucivyDeleteTest2` et `LucivyUpdateTest` écrits
- Tout compile

#### Problème identifié : visibilité des changements Lucivy

Le DELETE et le INSERT incrémental (hooks) écrivent dans le **writer** Lucivy, mais les changements ne sont pas visibles par le **reader** tant qu'on n'a pas fait `commit() + reload_reader()`.

Ce commit/reload est fait dans `checkpointInMemory()`. Le problème : **le framework rag3db n'appelle pas `checkpointInMemory()` sur les index secondaires** entre deux transactions. Il ne l'appelle qu'au checkpoint global (fin de session / seuil WAL).

Donc : DELETE → QUERY dans deux requêtes séparées → le reader ne voit pas le delete.

FTS de Kuzu n'a pas ce problème car il utilise des tables internes rag3db (visibles via le système de transactions de rag3db).

#### Options pour résoudre

**Option A : commit+reload dans chaque hook** (insert/delete_/update)
- Simple mais **pas optimal** — un commit+reload par opération individuelle
- Lucivy commit = flush segments sur disque = coûteux pour des inserts en rafale

**Option B : commit+reload lazy dans QUERY_LUCIVY_INDEX**
- Ajouter un flag `dirty_` sur LucivyIndex
- `insert()`, `delete_()`, `update()` → mettent `dirty_ = true`
- `QUERY_LUCIVY_INDEX` → avant la recherche, vérifie `dirty_` et fait `commit + reload_reader` si nécessaire
- **Un seul commit** même après N insertions/deletions
- Le coût est payé au premier QUERY, pas à chaque mutation
- **Recommandé** — c'est le pattern "write-behind" classique pour les moteurs de recherche

**Option C : commit+reload dans `checkpointInMemory` uniquement**
- Déjà en place. Problème : le checkpoint n'arrive pas entre deux requêtes auto-commit successives
- Fonctionne pour les batch workflows (CREATE INDEX → bulk INSERT → QUERY) mais pas pour le mode interactif

**Recommandation** : Option B (lazy commit dans QUERY). Changements nécessaires :
1. Ajouter `bool dirty_ = false;` dans `LucivyIndex`
2. Mettre `dirty_ = true` dans `insert()`, `delete_()`, `update()`
3. Ajouter `flushIfDirty()` qui fait `commit + reload_reader + dirty_ = false`
4. Appeler `flushIfDirty()` au début de `search_with_highlights` / `search_filtered_with_highlights` dans `query_lucivy_index.cpp`... mais ça nécessite un accès mutable au handle depuis QUERY, qui a actuellement `const LucivyHandle&`. Il faudra changer `getHandle()` en non-const, ou ajouter une méthode `flush()` sur LucivyIndex.

---

## Résumé des tests

| Test | Statut |
|------|--------|
| LucivyCreateAndQueryTest | PASS |
| LucivyDropTest | PASS |
| LucivyErrorTest | PASS |
| LucivyStemmerTest | PASS |
| LucivyFilteredSearchTest | PASS |
| LucivyFilterFieldsTest | PASS |
| LucivyPersistenceTest | PASS |
| **LucivyDeleteTest2** | **FAIL** — reader ne voit pas le delete (pas de commit+reload) |
| **LucivyUpdateTest** | **Non testé** — même problème attendu |

Les 7 premiers tests passent en mode fichier ET in-memory, zéro régression.

---

## Fichiers modifiés cette session

### Rust (ld-lucivy/lucivy_fts/rust/src/)
- `query.rs` — FilterClause, build_filter_clause, json_to_term, filters dans QueryConfig
- `handle.rs` — ajout type f64 dans build_schema()

### C++ (extension/lucivy_fts/)
- `src/function/create_lucivy_index.cpp` — FilterFieldInfo, filter_fields, add_document_mixed
- `src/function/query_lucivy_index.cpp` — allowed_ids, search_filtered_with_highlights
- `src/include/index/lucivy_index.h` — initUpdateState, update, fieldTypes_, hasMixedFields_
- `src/index/lucivy_index.cpp` — LucivyUpdateState, initUpdateState, update, fieldTypeToLogicalType
- `test/lucivy_fts_test.cpp` — 9 tests (7 passent, 2 en attente du fix visibilité)

---

## Pour demain

1. **Résoudre la visibilité** : implémenter l'option B (lazy commit dans QUERY) — ~30 lignes
2. **Faire passer LucivyDeleteTest2 et LucivyUpdateTest**
3. **Vérifier les 9 tests** en mode fichier + in-memory
4. Mettre à jour la doc
