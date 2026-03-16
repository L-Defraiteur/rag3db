# Doc 01 — Rapport session : finalisation sparse + fix LockBusy + migrations

Date : 15 mars 2026

## Résumé

Session de finalisation de la migration sparse (Phases 1-3 commitées en session précédente) + debug et fix du LockBusy sur DB reopen + investigation des migrations destructives.

## Commits de cette session

### `99fd8d76` — fix: MemBlobStore fallback + sparse_handles service registration
- Type `blob_store` : `Option<Arc<CypherBlobStore>>` → `Option<Arc<dyn BlobStore>>`
- Fallback `MemBlobStore` quand `sync_conn` est None (tests in-memory)
- Registration du service `sparse_handles` dans les 3 paths (drain simple, drain unified, reindex)

### `db867275` — fix: move blob_store init before ensure_sparse_handle in initialize()
- Bug d'ordering : `ensure_sparse_handle()` était appelé avant `blob_store` init → sparse handles jamais créés
- Déplacé blob_store init (étape 8) avant ensure_sparse_handle (étape 9)

### `6057ac9a` — fix: LockBusy on DB reopen + shutdown events + shared Database sync conn
- **`CREATE_LUCIVY_INDEX` idempotent** : skip si index déjà chargé par `initLucivyEntries`
- **`~LucivyIndex()` destructeur** : appelle `close()` pour relâcher le writer lock
- **`Catalog::shutdown()`** : `&mut self`, drain sparse handles, émet events
- **`ShutdownStarted`/`ShutdownCompleted`** events dans EventBus
- **`Rag3dbConnection`** : `Box<Database>` → `Arc<Database>`, ajout `create_sync_connection()`
- **Phase6 test** : utilise sync conn partagée pour CypherBlobStore persistent

## Résultats E2E

```
e2e_search                      36 passed, 2 FAILED
```

Les 2 fails sont BM25 pré-existants (`phase1_bm25_contains_exact`, `phase3_hybrid_3way`), pas liés au sparse. Cause identifiée : les champs `._ngram`/`._raw` ne sont pas alimentés par l'extension C++ lors des insertions via hooks NodeTable. Le contains fonctionne parfaitement dans lucivy_core (test ajouté par l'instance ld-lucivy).

## Investigation LockBusy — résumé

### Cause racine
Session 2 `load_extensions` → `initLucivyEntries()` ouvre l'index existant avec un writer. Puis `initialize()` → `CREATE_LUCIVY_INDEX` essaie de créer un AUTRE writer au même path → **conflit avec soi-même** (pas avec session 1).

### Fix
`CREATE_LUCIVY_INDEX` vérifie maintenant si l'index existe déjà sur le NodeTable et fait un early return.

### Fix secondaire
`~LucivyIndex()` appelle `close()` en destructeur — filet de sécurité si `CLOSE_LUCIVY_INDEX` n'est jamais appelé (ex: crash, `~Database()` qui ne cascade pas).

### `Rag3dbConnection::create_sync_connection()`
Le `sync_conn` doit partager le même `Database` que la connexion principale (sinon les tables comme `_index_blobs` ne sont pas visibles). Refactor `Box<Database>` → `Arc<Database>` pour permettre ça.

## État des migrations destructives

Ref : `docs/12-mars-2026-15h21/08-todo-migrations-destructives.md`

### Ce qui est géré (code actuel dans `migrate_entity()`)

| Cas | Comportement | Code |
|-----|-------------|------|
| **Ajout de champ** | `ALTER TABLE ADD` + reindex si content/title changé | ✅ lignes 610-625 |
| **Changement d'annotation** (is_content, title_for) | Détecté → `needs_reindex` + drop/recreate FTS | ✅ lignes 627-661 |
| **Ajout de signal** (ex: +vector) | Crée l'index manquant | ✅ lignes 663-671 |
| **Suppression de champ** | **Refusé avec erreur** | ✅ lignes 589-596 |
| **Changement de type** | **Refusé avec erreur** | ✅ lignes 598-607 |

### Ce qui n'est PAS géré

| Cas | Impact | Risque |
|-----|--------|--------|
| **Suppression de champ** | L'utilisateur ne peut pas retirer un champ du schema | Bloquant pour l'évolution du schema — mais safe |
| **Changement de type** | Idem | Bloquant — mais safe |
| **Changement d'embedding_dim** | Non détecté — les colonnes FLOAT[N] existantes restent avec l'ancien N | **Silencieusement cassé** si on change de modèle |
| **Retrait d'un signal** (ex: sparse → plus de sparse) | Non détecté — le sparse handle existe mais pas utilisé | Pas grave, juste du waste |
| **Colonnes sparse supprimées du schema** | Les anciennes tables ont encore `sparse_indices`/`sparse_weights` | **Non impactant** — colonnes ignorées, sparse est maintenant via SparseHandle |

### Verdict sur les colonnes sparse

La migration sparse (Phase 3) a supprimé les colonnes `sparse_indices`/`sparse_weights` du DDL généré. Les anciennes DBs qui ont ces colonnes les garderont comme colonnes orphelines — elles ne sont plus lues ni écrites. Le sparse fonctionne maintenant via `SparseHandle` + `BlobStore`.

Pour une DB existante qui est réouverte après la migration :
- `initialize()` → `CREATE TABLE IF NOT EXISTS` → no-op (table existe déjà avec les vieilles colonnes)
- Les vieilles colonnes restent mais sont ignorées
- Le sparse fonctionne via les handles Rust
- Pas de perte de données, pas de corruption

**TODO** : ajouter un nettoyage des colonnes orphelines (`ALTER TABLE DROP sparse_indices`, `ALTER TABLE DROP sparse_weights`) lors de la migration. Même pattern à prévoir pour FTS quand on migrera les insertions FTS vers Rust (colonnes/index C++ qui deviendront obsolètes).

### Recommandation

1. **Embedding dim change** : ajouter une détection dans `initialize()` qui compare `config.embedding_dim` avec la dim des colonnes existantes. Si mismatch → erreur + message "run catalog.rebuild()"
2. **Suppression de champ** : garder le refus pour l'instant, c'est safe
3. **Colonnes orphelines (sparse + futur FTS)** : ajouter un step de cleanup dans `migrate_entity()` qui drop les colonnes connues comme obsolètes (`sparse_indices`, `sparse_weights`)

## Prochaine étape : migration FTS vers Rust

Même pattern que sparse — utiliser `LucivyHandle` directement depuis rag3weaver au lieu de passer par les hooks C++ du NodeTable. Ça réglerait le bug BM25 (champs `._ngram`/`._raw` non alimentés) et simplifierait l'architecture.

Scope à définir dans un doc dédié.
