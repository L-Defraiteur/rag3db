# 10 — Session tests E2E natif avec KB réaliste

## Ce qu'on a fait

### 1. Cahier des charges complet (doc 09)

Écrit `09-cahier-des-charges-tests.md` : inventaire exhaustif de **toutes** les fonctionnalités rag3weaver, avec 10 phases de test, ~65 tests couvrant CRUD, search (BM25, vector, hybrid, sparse, 3-way), filtres, chunking, explore, events, robustesse. Config réaliste avec 2 entités, 2 KBs, 7 types de champs, 3 relations.

### 2. Script `run_e2e.sh`

Créé `extension/rag3weaver/run_e2e.sh` — script dédié pour les tests E2E natifs :
- Configure et build dans `build/native-test/` (isolé des autres builds WASM/nodejs/release)
- Charge automatiquement les bonnes env vars (`RAG3DB_SHARED`, `LD_LIBRARY_PATH`, etc.)
- Options : `--build-only`, `--no-build`, `--test <file>`, filtre par nom de test

```bash
./run_e2e.sh                    # build + run tous les tests
./run_e2e.sh phase0             # build + run tests matching "phase0"
./run_e2e.sh --no-build phase1  # skip build, run direct
./run_e2e.sh --build-only       # juste compiler
```

### 3. Build `native-test`

Configuré et compilé `build/native-test/` avec les 3 extensions :
- `vector` (HNSW index)
- `tantivy_fts` (FTS)
- `sparse_vector`

Les `.rag3db_extension` sont placés par cmake dans `extension/*/build/` (arbre source, PAS dans le build dir).

### 4. Fichier de test `e2e_search.rs`

Créé `extension/rag3weaver/tests/e2e_search.rs` avec :
- Config KB réaliste (Document + Author, 2 KBs, filter fields, relations avec propriétés)
- Helper `load_extensions()` qui charge vector + tantivy_fts via `LOAD EXTENSION`
- Phase 0 : 6 tests CRUD
- Phase 1 : 4 tests BM25 search (prêts mais pas encore exécutés)

## État actuel des tests

### Phase 0 — Résultats

| Test | Résultat | Notes |
|------|----------|-------|
| `phase0_initialize_with_kb_config` | ✅ PASS | DDL + indexes créés, KB metadata résolue |
| `phase0_create_drain_all_field_types` | ✅ PASS | 7 entités, tous types de champs vérifiés |
| `phase0_link_relations` | ✅ PASS | 3 inserts + 3 links (dont CITES avec propriété) |
| `phase0_update_and_delete` | ✅ PASS | Update body+score, delete, verification |
| `phase0_error_cases` | ✅ PASS | UnknownEntity, UnknownRelation, UnknownKB, NotFound |
| `phase0_hashsafe_dedup` | ❌ FAIL | Lock conflict Tantivy |

### Phase 1 (BM25) — pas encore exécutée

4 tests écrits : `phase1_bm25_basic_search`, `phase1_bm25_neural_networks`, `phase1_bm25_french`, `phase1_bm25_no_results`.

## Problèmes identifiés

### Bug 1 : Tantivy lock conflict entre catalogs in-memory

**Symptôme** : quand on crée 2 `Catalog` avec `Rag3dbConnection::in_memory()` dans le même process, le 2ème `initialize()` échoue :
```
cannot create writer: Failed to acquire Lockfile: LockBusy.
"there is already an IndexWriter working on this Directory"
```

**Cause probable** : les DB in-memory utilisent toutes le même chemin par défaut pour les index Tantivy (car `getDatabasePath()` retourne un chemin fixe ou vide pour les DB in-memory). Deux instances se battent pour le même directory Tantivy sur disque.

**Impact** : empêche tout test qui crée 2 catalogs dans le même process (hashsafe dedup entre catalogs, tests de concurrence).

**Fix à investiguer** :
- Comment `getDatabasePath()` se comporte pour les DB in-memory dans rag3db ?
- Le chemin des index Tantivy est `parent_path(getDatabasePath()) + /tantivy_indexes/<table>/` — si le DB path est vide ou toujours le même, tous les indexes atterrissent au même endroit
- Solution possible : donner un temp dir unique à chaque DB in-memory, OU passer un path au lieu d'utiliser `in_memory()`

### Bug 2 : Tests parallèles → même lock conflict

**Symptôme** : sans `--test-threads=1`, TOUS les tests échouent avec le même LockBusy (sauf 1 qui gagne la course).

**Cause** : même que Bug 1 — tous les catalogs partagent le même chemin d'index Tantivy.

**Fix** : `--test-threads=1` contourne le problème MAIS c'est lent et masque le vrai bug. La vraie fix est la même que Bug 1 : des chemins uniques.

### Observation : les extensions .rag3db_extension sont dans l'arbre source

cmake place les shared libs compilées dans `extension/<name>/build/` (arbre source), PAS dans `build/native-test/extension/`. Ça veut dire que les builds WASM et natif écrasent potentiellement les mêmes fichiers. Actuellement le dernier build gagne — le build native-test a produit des `.so` ELF valides, mais un build WASM ultérieur pourrait les remplacer par des archives statiques.

**À terme** : il faudrait que cmake place les extensions dans le build dir, pas le source dir.

## Fichiers créés/modifiés

| Fichier | Action |
|---------|--------|
| `docs/28-fevrier-2026-11h31/09-cahier-des-charges-tests.md` | Créé — cahier des charges complet |
| `extension/rag3weaver/run_e2e.sh` | Créé — script de build + test dédié |
| `extension/rag3weaver/tests/e2e_search.rs` | Créé — tests E2E Phase 0 + Phase 1 |

## Prochaines étapes

1. **Fixer le lock Tantivy** (Bug 1+2) — chemin unique par DB in-memory
2. **Exécuter Phase 1 BM25** — les tests sont écrits, il faut les lancer
3. **Investiguer search 0 résultats** — BM25 et vector retournent 0 (probablement lié à contentFor manquant dans les anciens tests WASM, mais à vérifier en natif maintenant)
4. Continuer les phases 2-10 du cahier des charges
