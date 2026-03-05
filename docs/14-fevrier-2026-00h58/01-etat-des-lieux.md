# État des lieux — 14 février 2026, 01h00 (mis à jour ~05h00)

> Consolidation de toutes les sessions du 13-14 février 2026.

---

## Vue d'ensemble

Le projet intègre un moteur de recherche full-text (Lucivy) dans **rag3db** (fork de Kuzu), pour servir de backend à **Rag3Weaver** (framework RAG TypeScript).

### Repos

| Repo | Branche | Description |
|------|---------|-------------|
| `rag3db` | `feature/fuzzy-fts` | Fork Kuzu v0.11.2.2, renommé rag3db |
| `ld-lucivy` | `main` | Fork Lucivy v0.26.0, submodule de rag3db |

---

## Travail complété — résumé par phase

### Phases 1-8 (1er-13 février) — Moteur Lucivy

Détails dans `docs/13-fevrier-2026-18h57/01-etat-des-lieux.md`.

- **Crate Rust FFI** `lucivy_fts` — bridge cxx typé (9 structs, 15 fonctions)
- **Tri-field layout** automatique : `body` → stemmed + `._raw` + `._ngram`
- **Cascade 4 niveaux** : exact → fuzzy → substring → fuzzy substring
- **ContainsScorer** (multi-token) + **ContainsSingleScorer** (résout "c++")
- **FuzzySubstringAutomaton** — NFA `.*{levenshtein(token,d)}.*`
- **HighlightSink** — byte offsets pour tous les types de query
- **WithFreqsAndPositionsAndOffsets** — offsets dans les postings (21 fichiers)
- **Rename kuzu → rag3db** (2538 fichiers)
- **Tests** : 1015 ld-lucivy = tout vert

### Phase A (13-14 février) — Extension C++ `lucivy_fts`

Détails dans `docs/13-fevrier-2026-18h57/06-plan-implementation-unifie.md` et `07-fix-query-e2e-tests.md`.

3 fonctions Cypher implémentées :

```cypher
-- Créer un index FTS sur une table de nœuds
CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body']);
CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'], stemmer := 'french');
CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'], filter_fields := ['category', 'score']);

-- Rechercher (6 types : contains, term, fuzzy, phrase, parse)
CALL QUERY_LUCIVY_INDEX('doc', '{"type":"contains","field":"body","value":"c++"}', 10)
RETURN node_id, score, highlights;

-- Rechercher avec filtrage par node IDs
CALL QUERY_LUCIVY_INDEX('doc', '{"type":"contains","field":"body","value":"test"}', 10,
  allowed_ids := [CAST(0, 'UINT64'), CAST(2, 'UINT64')])
RETURN node_id, score, highlights;

-- Rechercher avec filtres natifs sur colonnes non-texte
CALL QUERY_LUCIVY_INDEX('doc', '{"type":"contains","field":"body","value":"programming",
  "filters":[{"field":"category","op":"eq","value":1},{"field":"score","op":"gt","value":8.0}]}', 10)
RETURN node_id, score, highlights;

-- Supprimer l'index
CALL DROP_LUCIVY_INDEX('doc');
```

**Architecture** :
- `LucivyIndex` hérite de `storage::Index` — hooks automatiques (`insert`/`delete_`/`checkpoint`)
- Indexation incrémentale native (pas de SYNC manuel)
- Persistance via `IndexStorageInfo` sérialisé avec la NodeTable (pas de table registre)
- `QUERY` = `TableFunc` (nécessite RETURN), `CREATE`/`DROP` = `StandaloneTableFunc`
- Recherche exécutée dans `bindFunc`, résultats distribués par morsel dans `tableFunc`

**Fichiers créés** (dans `extension/lucivy_fts/`) :
```
src/include/index/lucivy_index.h
src/include/catalog/lucivy_index_catalog_entry.h
src/include/function/create_lucivy_index.h
src/include/function/query_lucivy_index.h
src/include/function/drop_lucivy_index.h
src/index/lucivy_index.cpp
src/catalog/lucivy_index_catalog_entry.cpp
src/function/create_lucivy_index.cpp
src/function/query_lucivy_index.cpp
src/function/drop_lucivy_index.cpp
```

### Phase B (14 février) — Tests E2E GTest

7 tests GTest automatisés, tous verts en mode fichier ET in-memory :

1. **LucivyCreateAndQueryTest** — 6 types de query (contains, term, fuzzy, phrase, parse, c++)
2. **LucivyDropTest** — CREATE → query OK → DROP → query échoue
3. **LucivyErrorTest** — erreurs (pas d'index, propriété inexistante, type non-STRING)
4. **LucivyStemmerTest** — stemmer français
5. **LucivyFilteredSearchTest** — filtrage par node IDs (`allowed_ids`)
6. **LucivyFilterFieldsTest** — filter fields natifs (INT64, DOUBLE) avec opérateurs (eq, gt, gte, multi-filtres)
7. **LucivyPersistenceTest** — create → close DB → reopen → query = mêmes résultats

### Fix mode fichier (14 février)

Détails dans `docs/13-fevrier-2026-18h57/09-fix-mode-fichier.md`.

2 bugs corrigés :
1. **Buffer overflow `magicBytes[4]`** — MAGIC_BYTES="RAG3DB" (6 chars) mais buffer hardcodé à 4 → stack smashing. Fix : `[8]` + assert.
2. **Index path invalide** — `getDatabasePath()` retourne le fichier DB, pas le répertoire → `parent_path()`.

### Persistance (14 février) — NOUVEAU

Test E2E validant le cycle complet : create DB file-mode → create index → insert docs → query OK → close DB → reopen DB → query → mêmes résultats.

Valide : sérialisation `IndexStorageInfo` → désérialisation → `LucivyIndex::load()` → `open_index()` → `finalize()`.

Le test crée sa propre DB at temp path (indépendant du fixture `ApiTest`) pour garantir le mode fichier même quand `IN_MEM_MODE=true`.

### Filtrage par node IDs (14 février) — NOUVEAU

Câblage de `search_filtered_with_highlights` (déjà disponible côté Rust) dans `QUERY_LUCIVY_INDEX` via paramètre optionnel Cypher.

**Syntaxe** :
```cypher
CALL QUERY_LUCIVY_INDEX('doc', '{"type":"contains",...}', 10,
  allowed_ids := [CAST(0, 'UINT64'), CAST(2, 'UINT64')])
RETURN node_id, score, highlights;
```

**Changement** : `query_lucivy_index.cpp` — extraction du paramètre optionnel `allowed_ids` via `NestedVal`, dispatch vers `search_filtered_with_highlights` quand présent.

Utile pour le pré-filtrage Cypher : `MATCH ... WHERE ... WITH collect(id) AS ids CALL QUERY_LUCIVY_INDEX(...)`.

### Filter fields natifs v2 (14 février) — NOUVEAU

Colonnes non-texte (INT64, DOUBLE, UINT64, FLOAT, STRING exacte) indexées directement dans Lucivy pour filtrage **pendant** le scoring — pas de pré-filtrage par IDs nécessaire.

#### Côté C++ (create_lucivy_index.cpp)

- `FilterFieldInfo` struct (name, propertyID, lucivyType)
- `mapLogicalTypeToLucivy()` — INT64/INT32/INT16/INT8 → "i64", UINT64/32/16/8 → "u64", DOUBLE/FLOAT → "f64", STRING → "string"
- `resolveFilterFields()` — validation + résolution des propriétés
- Filter fields inclus dans le schema JSON avec `indexed:true, fast:true`
- `add_document_mixed()` utilisé au lieu de `add_document_texts()` quand des filter fields sont présents
- Filter fields inclus dans `IndexInfo.columnIDs` et `IndexCatalogEntry.propertyIDs`

#### Côté C++ (lucivy_index.cpp)

- `fieldTypes_` vector + `hasMixedFields_` flag dans `LucivyIndex`
- `insert()` dispatch vers `add_document_mixed` pour les tables avec filter fields
- `finalize()` reconstruit les types corrects pour chaque colonne

#### Côté Rust (query.rs)

- `FilterClause` struct (field, op, value)
- `filters` champ optionnel dans `QueryConfig`
- `build_query()` wraps la query texte dans un `BooleanQuery` avec `Occur::Must` quand des filtres sont présents
- `json_to_term()` — conversion JSON → `Term` selon le type du champ dans le schema
- `build_filter_clause()` — 7 opérateurs supportés : `eq`, `ne`, `lt`, `lte`, `gt`, `gte`, `in`
- `eq`/`ne` : `TermQuery`
- `lt`/`lte`/`gt`/`gte` : `RangeQuery` avec bornes inclusives/exclusives
- `in` : `BooleanQuery` avec `Occur::Should` (OR)

#### Côté Rust (handle.rs)

- Fix : ajout du type `"f64"` manquant dans `build_schema()` (causait "unknown field type: f64")

#### Syntaxe

```cypher
-- Création avec filter fields
CALL CREATE_LUCIVY_INDEX('article', ['title', 'body'],
  filter_fields := ['category', 'score']);

-- Query avec filtres natifs dans le JSON
CALL QUERY_LUCIVY_INDEX('article',
  '{"type":"contains","field":"body","value":"programming",
    "filters":[
      {"field":"category","op":"eq","value":1},
      {"field":"score","op":"gt","value":8.0}
    ]}', 10)
RETURN node_id, score, highlights;
```

---

## État actuel des tests

| Suite | Résultat | Commande |
|-------|----------|----------|
| ld-lucivy lib | 1015 pass | `cargo test --lib` |
| Extension E2E (GTest) | **7 pass** (fichier + in-memory) | `./lucivy_fts_test` |

---

## Ce qui fonctionne

| Fonctionnalité | Statut |
|---------------|--------|
| CREATE_LUCIVY_INDEX (bulk scan + indexation) | OK |
| QUERY_LUCIVY_INDEX (6 types de query + highlights) | OK |
| DROP_LUCIVY_INDEX (cleanup complet) | OK |
| Indexation incrémentale (INSERT → index auto) | OK (visible après checkpoint) |
| Mode fichier | OK |
| Mode in-memory | OK |
| Persistance (close DB → reopen → query) | OK |
| Filtrage par node IDs (`allowed_ids`) | OK |
| Filter fields natifs (INT64, DOUBLE, etc.) | OK |
| Filtres dans query JSON (eq, ne, lt, lte, gt, gte, in) | OK |
| cxx bridge typé (zéro JSON sur hot path) | OK |
| Stemmer configurable | OK |

---

## Ce qui reste

### 1. Phase C : Intégration Rag3Weaver

Adapter le framework RAG TypeScript pour utiliser rag3db comme backend :
- Wrapper Node.js pour les fonctions Cypher
- Pipeline d'ingestion → indexation automatique
- API de recherche exposant les highlights

### 2. Améliorations futures

- **DELETE support** dans `LucivyIndex::delete_()` (implémenté mais non testé E2E)
- **UPDATE support** (delete + re-insert)
- **Opérateur `in`** pour filter fields (supporté côté Rust, pas encore testé E2E)
- **Opérateur `ne`** (not equal) pour filter fields (supporté côté Rust, pas encore testé E2E)

---

## Arborescence actuelle

```
packages/rag3db/
├── extension/lucivy/
│   └── ld-lucivy/                    ← Submodule (fork Lucivy v0.26.0)
│       ├── src/                       ← Moteur Lucivy modifié (1015 tests)
│       └── lucivy_fts/rust/src/
│           ├── bridge.rs              ← cxx bridge (9 structs, 15 fonctions)
│           ├── handle.rs              ← LucivyHandle, tri-field layout, f64 support
│           └── query.rs               ← build_query, FilterClause, build_filter_clause
├── extension/lucivy_fts/
│   ├── src/
│   │   ├── main/lucivy_fts_extension.cpp  ← Point d'entrée extension
│   │   ├── index/lucivy_index.cpp         ← LucivyIndex + mixed fields support
│   │   ├── catalog/                        ← LucivyIndexAuxInfo
│   │   └── function/
│   │       ├── create_lucivy_index.cpp    ← filter_fields + add_document_mixed
│   │       ├── query_lucivy_index.cpp     ← allowed_ids + highlights
│   │       └── drop_lucivy_index.cpp      ← Standalone + Internal
│   ├── test/
│   │   └── lucivy_fts_test.cpp            ← 7 tests GTest E2E
│   └── CMakeLists.txt                      ← Build (Rust + C++ + cxx glue)
├── src/                               ← Code C++ rag3db (ex-Kuzu)
└── build/release/                     ← Build vérifié OK
```

---

## Commandes de build

```bash
# Tests ld-lucivy (1015 tests)
cd packages/rag3db/extension/lucivy/ld-lucivy && cargo test --lib

# Build rag3db + extension + tests
cd packages/rag3db/build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="lucivy_fts" -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . --target lucivy_fts_test -j$(nproc)
cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)

# Run tests (les deux modes fonctionnent)
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu ./extension/lucivy_fts/test/lucivy_fts_test
IN_MEM_MODE=true LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu ./extension/lucivy_fts/test/lucivy_fts_test
```

---

## Historique des docs

| Dossier | Contenu |
|---------|---------|
| `13-fevrier-2026-18h57/01` | État des lieux consolidé (phases 1-8) |
| `13-fevrier-2026-18h57/02` | Plan fonctions Cypher (partiellement obsolète — remplacé par 06) |
| `13-fevrier-2026-18h57/03` | Hooks incrémentaux (obsolète — voir 04) |
| `13-fevrier-2026-18h57/04` | Infra d'index existante (storage::Index) — **référence architecture** |
| `13-fevrier-2026-18h57/05` | Migration cxx bridge — **TERMINÉ** |
| `13-fevrier-2026-18h57/06` | Plan implémentation unifié — **référence implémentation** |
| `13-fevrier-2026-18h57/07` | Fix QUERY + tests E2E manuels |
| `13-fevrier-2026-18h57/08` | Phase B — Tests E2E GTest |
| `13-fevrier-2026-18h57/09` | Fix mode fichier (2 bugs) |
| **`14-fevrier-2026-00h58/01`** | **Ce document** — état des lieux final (persistance + filtres) |
