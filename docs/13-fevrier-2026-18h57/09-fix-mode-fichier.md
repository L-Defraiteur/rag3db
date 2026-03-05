# 09 — Fix mode fichier : 2 bugs corrigés

Session du 14 février 2026, ~01h30–02h15.
Reprend là où 08-phase-b-tests-e2e-cpp.md s'est arrêté.

## Contexte

Les tests E2E (Phase B) ne tournaient qu'en mode in-memory (`IN_MEM_MODE=true`). En mode fichier, rag3db crashait au démarrage avec `*** stack smashing detected ***` dans `DatabaseHeader::deserialize`.

## Bug 1 : Buffer overflow dans `validateMagicBytes`

**Fichier** : `src/storage/database_header.cpp:35`

**Symptôme** : `*** stack smashing detected ***: terminated` à chaque ouverture d'une DB fichier.

**Stack trace** :
```
DatabaseHeader::deserialize(Deserializer&)
  └─ validateMagicBytes(Deserializer&)
       └─ écrit 6 octets dans un buffer de 4 → stack smashing
```

**Cause racine** : Lors du rename kuzu → rag3db, `MAGIC_BYTES` est passé de `"KUZU"` (4 chars) à `"RAG3DB"` (6 chars) dans `storage_version_info.h`. Mais le buffer de lecture dans `validateMagicBytes` est resté `uint8_t magicBytes[4]` — la boucle écrit 6 octets dans un buffer de 4, overflow de 2 octets sur la pile.

**Fix** :
```cpp
// Avant (bug)
uint8_t magicBytes[4];

// Après
uint8_t magicBytes[8];
KU_ASSERT(numMagicBytes <= sizeof(magicBytes));
```

## Bug 2 : Index path invalide en mode fichier

**Fichier** : `extension/lucivy_fts/src/function/create_lucivy_index.cpp:161`

**Symptôme** : `filesystem error: cannot create directories: Not a directory [/.../db.kz/lucivy_indexes/doc/]`

**Cause racine** : `getDatabasePath()` retourne le chemin du **fichier** de la base de données (ex: `/tmp/rag3db/ApiTest.../db.kz`), pas le répertoire parent. Le code faisait `<dbPath>/lucivy_indexes/doc/` → essayait de créer un sous-répertoire sous un fichier.

**Fix** :
```cpp
// Avant (bug)
basePath = context.clientContext->getDatabasePath();

// Après
basePath = std::filesystem::path(
    context.clientContext->getDatabasePath()).parent_path().string();
```

**Note** : Seul `create_lucivy_index.cpp` construit le chemin initial. Les fonctions `query` et `drop` le lisent depuis le catalogue (`LucivyIndexAuxInfo.indexPath`), donc pas de changement nécessaire ailleurs.

## Fichiers modifiés

| Fichier | Modification |
|---------|-------------|
| `src/storage/database_header.cpp` | Buffer `magicBytes[4]` → `[8]` + assert + ajout `#include "common/assert.h"` |
| `extension/lucivy_fts/src/function/create_lucivy_index.cpp` | `getDatabasePath()` → `parent_path()` du database path |

## Build & vérification

```bash
cd packages/rag3db/build/release

# Rebuilder l'extension shared + le core + le test
cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)
cmake --build . --target lucivy_fts_test -j$(nproc)

# Mode fichier (auparavant crash)
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu \
  ./extension/lucivy_fts/test/lucivy_fts_test

# Mode in-memory (vérification régression zéro)
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu IN_MEM_MODE=true \
  ./extension/lucivy_fts/test/lucivy_fts_test
```

## Résultat

```
[==========] Running 4 tests from 1 test suite.
[       OK ] ApiTest.LucivyCreateAndQueryTest (408 ms)
[       OK ] ApiTest.LucivyDropTest (328 ms)
[       OK ] ApiTest.LucivyErrorTest (311 ms)
[       OK ] ApiTest.LucivyStemmerTest (463 ms)
[  PASSED  ] 4 tests.
```

Les 4 tests passent dans les **deux** modes (fichier et in-memory).

## Notes techniques

- **Extension dynamique** : lucivy_fts est compilée en shared lib (`.rag3db_extension`), pas en static link. Le test la charge via `LOAD EXTENSION`. Il faut rebuilder la target `rag3db_lucivy_fts_extension` en plus de `lucivy_fts_test` quand on modifie le code de l'extension.
- **`EXTENSION_STATIC_LINK_LIST`** est commentée dans `extension_config.cmake` → `__STATIC_LINK_EXTENSION_TEST__` n'est pas défini → les tests chargent l'extension dynamiquement.

## Mise à jour du doc 08

La note "Mode fichier crash — bug pré-existant" du doc 08 est maintenant obsolète. Les tests peuvent tourner sans `IN_MEM_MODE=true`.

## Prochaines étapes

- Phase C : Intégration Rag3Weaver
- Tests de persistance (redémarrage DB fichier → index Lucivy toujours présent)
