# 08 — Phase B : Tests E2E C++ (GTest)

Session du 14 février 2026, ~00h30–01h15.
Reprend là où 07-fix-query-e2e-tests.md s'est arrêté.

## Objectif

Écrire des tests E2E automatisés en C++ (GTest) pour l'extension tantivy_fts, suivant le pattern des extensions existantes (FTS, vector, algo).

## Fichiers créés/modifiés

| Fichier | Action |
|---------|--------|
| `extension/tantivy_fts/test/CMakeLists.txt` | **Créé** — garde `BUILD_EXTENSION_TESTS` |
| `extension/tantivy_fts/test/tantivy_fts_test.cpp` | **Créé** — 4 tests GTest |
| `extension/tantivy_fts/CMakeLists.txt` | **Modifié** — ajouté `add_subdirectory(test)` |

## Pattern utilisé

- `TEST_F(ApiTest, ...)` — hérite de `BaseGraphTest`, charge tinysnb automatiquement (ne gêne pas)
- Chargement conditionnel de l'extension via `#ifndef __STATIC_LINK_EXTENSION_TEST__`
- Chaque test crée sa propre table `doc` (ID UINT64, title STRING, body STRING)
- `QUERY_TANTIVY_INDEX` est un `TableFunc` (pas standalone) → nécessite `RETURN node_id, score, highlights`

## Les 4 tests

### TantivyCreateAndQueryTest

Crée 3 documents puis teste les 6 types de query :

| Query | JSON | Résultat attendu |
|-------|------|-----------------|
| contains | `{"type":"contains","field":"body","value":"programming"}` | 2 résultats |
| term | `{"type":"term","field":"body","value":"programming"}` | 2 résultats |
| fuzzy | `{"type":"fuzzy","field":"body","value":"programing","distance":1}` | 2 résultats |
| phrase | `{"type":"phrase","field":"body","terms":["systems","programming"]}` | 1 résultat |
| parse | `{"type":"parse","field":"body","value":"rust AND programming"}` | 1 résultat |
| contains c++ | `{"type":"contains","field":"title","value":"c++"}` | 1 résultat |

### TantivyDropTest

1. Crée table + index + vérifie query OK
2. `CALL DROP_TANTIVY_INDEX('doc')` → succès
3. Query après drop → `ASSERT_FALSE(result->isSuccess())`

### TantivyErrorTest

1. Query sur table sans index → erreur
2. CREATE avec propriété inexistante → erreur
3. CREATE avec propriété non-STRING (ID = UINT64) → erreur

### TantivyStemmerTest

1. Crée index avec `stemmer := 'french'`
2. Contains query "programming" → fonctionne (mots anglais non affectés par stemmer français)

## Build & exécution

```bash
# Build ciblé (évite les erreurs GLIBCXX des autres tests)
cd packages/rag3db/build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="tantivy_fts" \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . --target tantivy_fts_test -j$(nproc)

# Exécuter en mode in-memory (mode fichier crash — bug pré-existant)
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu IN_MEM_MODE=true \
  ./extension/tantivy_fts/test/tantivy_fts_test
```

## Résultat

```
[==========] Running 4 tests from 1 test suite.
[  PASSED  ] 4 tests.
```

## Notes

- **miniconda pollue LD_LIBRARY_PATH** : le `libstdc++.so.6` de conda est trop ancien (manque GLIBCXX_3.4.30/31/32). Il faut forcer `LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu`.
- **`cmake --build . --target tantivy_fts_test`** : build ciblé car le build global échoue sur `gtest_discover_tests` des autres targets (api_test, c_api_test) à cause du même problème GLIBCXX.
- **Mode fichier** : le crash est dans `DatabaseHeader::deserialize` (core rag3db), pas dans l'extension. Tous les tests tournent en in-memory.

## Prochaines étapes

- Phase C : Intégration Rag3Weaver
