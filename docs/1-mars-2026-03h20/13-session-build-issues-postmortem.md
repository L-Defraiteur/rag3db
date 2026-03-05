# 13 — Session : Build Issues — Postmortem & Solutions

## Contexte

Après la refacto Option A (doc 12), les 9 tests SearchInWhere segfaultaient (exit 139) alors que les 15 tests basiques passaient. Le debug a pris du temps à cause de plusieurs problèmes cumulés.

## Problème 1 : Disque plein pendant la compilation

**Symptôme** : segfault sur les tests SearchInWhere après un build "réussi".

**Cause** : le disque `/` (nvme0n1p3, 203G) était à 100% pendant la compilation. Les `.o` écrits sur un disque plein peuvent être tronqués/corrompus.

**Résolution** :
- Swap réduit de 113G → 4G (fichier sur `/`)
- Docker data-root déplacé vers `/home` (1.8T, 731G libres)
- Crash reports nettoyés (`/var/lib/apport`, 22G)
- Résultat : 67G libres sur `/`

**Leçon** : monitorer l'espace disque avant les builds lourds. Le swap de 113G sur `/` était excessif.

## Problème 2 : `.rag3db_extension` dans le source tree (pas le build tree)

**Symptôme** : `dynamic_cast<IndexSearchBindData*>` retourne NULL → segfault dans `visitScanNodeTableReplace`.

**Cause racine** : cmake `set_extension_properties` configure `LIBRARY_OUTPUT_DIRECTORY` vers `${PROJECT_SOURCE_DIR}/extension/lucivy_fts/build/`, c'est-à-dire dans le **source tree**. Quand on fait `rm -rf extension/lucivy_fts/` depuis le **build tree**, ça ne touche pas le `.rag3db_extension` réel.

**Conséquence** : le test charge un ancien `.rag3db_extension` compilé avec l'ancien `FTSSearchBindData` (sans `virtualExprSpecs`). Le core fait `dynamic_cast<IndexSearchBindData*>` sur un objet créé dans l'ancien `.so` → RTTI mismatch → NULL → crash.

**Résolution** : builder explicitement le target `rag3db_lucivy_fts_extension` :
```bash
cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)
```

## Problème 3 : `lucivy_fts_test` ne dépend pas de `rag3db_lucivy_fts_extension`

**Symptôme** : `cmake --build . --target lucivy_fts_test` ne rebuild PAS l'extension `.so`.

**Cause** : `add_rag3db_test` ne fait que :
```cmake
target_link_libraries(${TEST_NAME} PRIVATE test_helper test_runner graph_test)
```
Le test charge l'extension via `LOAD EXTENSION` (dlopen), donc cmake ne voit pas la dépendance.

**Le `.rag3db_extension` peut être stale** : le test compilera et linkera sans erreur même si l'extension `.so` est vieille de jours/semaines.

## Problème 4 : `lucivy_fts_extension_function` non inclus dans `lucivy_fts_test`

**Symptôme** : `find . -name "search_function.cpp.o"` → rien après le build du test.

**Cause** : les `.o` de l'extension sont des OBJECT libraries cmake. Ils sont compilés uniquement quand le target `rag3db_lucivy_fts_extension` (la shared lib) est demandé. Le test ne les inclut pas.

## Solutions recommandées

### Solution immédiate : script de build

Toujours builder les deux targets :
```bash
cmake --build . --target rag3db_lucivy_fts_extension --target lucivy_fts_test -j$(nproc)
```

Ou un alias :
```bash
alias build-lucivy='cmake --build . --target rag3db_lucivy_fts_extension --target lucivy_fts_test -j$(nproc)'
```

### Solution cmake (optionnelle, plus propre)

Ajouter une dépendance cmake dans `extension/lucivy_fts/test/CMakeLists.txt` :
```cmake
if (${BUILD_EXTENSION_TESTS})
    add_rag3db_test(lucivy_fts_test lucivy_fts_test.cpp)
    add_dependencies(lucivy_fts_test rag3db_lucivy_fts_extension)
endif ()
```

Ainsi, `cmake --build . --target lucivy_fts_test` rebuildera automatiquement le `.so` si nécessaire.

### Checklist debug pour segfaults futurs

1. **Espace disque** : `df -h /` — vérifier > 10G libres
2. **Extension .so** : `ls -la ../../extension/lucivy_fts/build/liblucivy_fts.rag3db_extension` — vérifier le timestamp
3. **Rebuild extension** : `cmake --build . --target rag3db_lucivy_fts_extension`
4. **RTTI cross-library** : si `dynamic_cast` échoue, vérifier que le `.so` et l'exécutable sont compilés avec les mêmes headers

## Résultat final

Après rebuild correct de l'extension + test :
```
[==========] Running 24 tests from 1 test suite.
[  PASSED  ] 24 tests.
```

24/24 tests verts — la refacto Option A (FTS → Index) est validée.

## Fichiers impliqués

| Fichier | Rôle |
|---------|------|
| `extension/CMakeLists.txt` | `set_extension_properties` → output dans source tree |
| `extension/lucivy_fts/CMakeLists.txt` | `build_extension_lib` → crée la shared lib |
| `extension/lucivy_fts/test/CMakeLists.txt` | `add_rag3db_test` → pas de dépendance sur la shared lib |
| `extension/lucivy_fts/build/liblucivy_fts.rag3db_extension` | Le .so chargé par les tests |
