# Doc 13 — Progression : register_entity / register_kb order-independent

Date : 12 mars 2026

Réf : doc 11, doc 12

## Ce qui a été fait

### 1. config.rs — helpers sur EntityConfig

```rust
pub fn has_simple_pipeline(&self) -> bool  // a des is_content fields
pub fn has_kb_participation(&self) -> bool  // a des content_for/title_for vers un KB (pas "self")
```

### 2. catalog.rs — register_entity() relaxé

- **Validation** : accepte si `has_simple_pipeline()` OU `has_kb_participation()` (avant : exigeait `is_content`)
- **create_entity_tables()** : si KB-only (pas de simple pipeline), crée SEULEMENT la table entité node. Skip chunk table, FTS, vector, sparse.
- **Re-trigger KBs** : après persist, scanne les `content_for`/`title_for` du config. Pour chaque KB mentionnée qui existe déjà dans `self.kb_metadata` → re-appelle `register_kb(kb_name, existing_config)`.
- **Docstring** mis à jour.

### 3. catalog.rs — register_kb() mise à jour KB existante

Avant : no-op si KB existait déjà. Maintenant :
- Compare les `content_refs` (old vs new)
- Si changés → DROP + CREATE FTS sur `{KB}_Index`
- Reconstruit `KBMetadata` avec les nouveaux content_refs

### 4. catalog.rs — resolve_search_target()

Pour les entities KB-only (pas de simple pipeline) : retourne une erreur claire avec suggestion des KBs disponibles :
```
"Entity 'Article' has no simple pipeline (KB-only) — search on KB 'docs' instead"
```

### 5. catalog.rs — migrate_entity()

- FTS rebuild sur entity table : seulement si `has_simple_pipeline()` sur la nouvelle config
- Create missing indexes (vector, sparse) : seulement si `has_simple_pipeline()`
- `needs_reindex` flag : toujours posé (pour simple ET KB)

### 6. catalog.rs — is_simple_entity() / is_registered_entity()

- `is_simple_entity()` : retourne true seulement si l'entity a `has_simple_pipeline()`
- `is_registered_entity()` : nouveau, retourne true si l'entity est dans `entity_configs` (simple ou KB-only)

### 7. catalog.rs — build_content_text() fix

Avant : utilisait `content_fields()` (= `is_content` fields) pour TOUTES les entities dans `entity_configs`.
Pour une KB-only entity, ça retournait "" → content hash = hash("") → UpdateRecordNode ne détectait jamais de changement.

Fix : si `content_fields()` est vide mais l'entity a des KB fields → utilise les champs `title_for`/`content_for` pour construire le content text.

### 8. catalog.rs — ingest_entities() KB trigger

Après le simple pipeline graph, si l'entity a KB participation :
1. Clone les données des records avant le graph (car EntityRecord n'est pas Clone)
2. Crée des `UpdateRecord` avec les données (sans les champs `_*` internes pour éviter le "Cannot SET primary key")
3. Appelle `drain()` → UpdateRecordNode détecte KB participation → enqueue AggregateRecords → KB pipeline

### 9. Tests E2E ajoutés (tests/e2e_idempotent_registration.rs)

6 nouveaux tests (11-16) :

| # | Test | Feature gate | Statut |
|---|------|-------------|--------|
| 11 | `hybrid_search_survives_migration_and_reindex` | candle-embedder | ✅ PASSE |
| 12 | `kb_migration_and_reindex` | — | ❌ EN COURS |
| 13 | `kb_vector_search_survives_migration` | candle-embedder | ❌ EN COURS |
| 14 | `kb_and_relation_persist_and_reopen` | — | ❌ EN COURS |
| 15 | `wild_mix_progressive_kb_and_simple` | — | ❌ EN COURS |
| 16 | `double_reindex_no_corruption` | — | ✅ PASSE |

Aussi ajouté `BM25Mode::ContainsSplit` à TOUS les SearchOptions BM25 (sinon BM25 ne trouve rien).

Les 10 tests originaux (1-10) passent tous.
Le test 11 (HYBRID simple entity avec MiniLM réel) passe.
Le test 16 (double reindex) passe.

## Bug en cours de debug (tests KB 12-15)

### Symptôme

Après `ingest_entities("Article", ...)` avec une entity KB-only → `register_kb("docs", ...)` :
- `Article` table : 2 records ✅
- `docs_Index` table : **0 records** ❌
- `docs_Index_Chunk` table : 2 records ✅ (bon contenu !)
- `docs_Index_HAS_CHUNK` rels : 0 ❌
- `Article_IN_docs` rels : 0 ❌
- `Article_SOURCED_docs` rels : 2 ✅

### Analyse

Le pipeline KB crée des chunks (`docs_Index_Chunk`) et des `SOURCED` rels, mais pas les `docs_Index` records ni les `HAS_CHUNK` / `IN_docs` rels. Ça pointe vers un problème dans `KBUpdateNode` ou `KBGatherNode`.

Le flow :
1. `ingest_entities()` insère les Article records ✅
2. `ingest_entities()` crée des UpdateRecords et appelle `drain()` ✅
3. `UpdateRecordNode` détecte le content hash change (fix #7) ✅ (`reembedded: true` dans les events)
4. `UpdateRecordNode` enqueue des `AggregateRecords` dans `pending_aggregates` service
5. `KBGatherNode` lit les `pending_aggregates`, fait des queries MATCH pour re-gather le contenu
6. `KBUpdateNode` crée/update les `{KB}_Index` records + `IN_{KB}` rels
7. `KBChunkNode` crée les chunks + `HAS_CHUNK` rels

**Le step 4 semble OK** car les chunks sont créés avec le bon contenu.
**Le step 6 semble échouer** — les `docs_Index` records ne sont pas créés.

### Hypothèse

Les chunks orphelins dans `docs_Index_Chunk` sans `docs_Index` parent suggèrent que :
- Soit `KBGatherNode` produit des `KBContentRecord` sans title/content (donc `KBUpdateNode` skip l'insertion)
- Soit la gather query ne trouve pas les données source (le MATCH sur Article ne retourne pas les bons champs)
- Soit il y a un problème de timing/ordering dans le dataflow graph

### Prochaines étapes

1. Ajouter du debug dans `KBGatherNode::gather_batch()` pour voir les queries exécutées et les résultats
2. Vérifier que `resolve_entity_kbs()` retourne le bon mapping pour l'EntityDef créée par `entity_config_to_entity_def()`
3. Possiblement le problème est que `KBGatherNode` n'arrive pas à faire le MATCH car le `title_entity` ou `source_uuid` ne correspond pas

## Fichiers modifiés

| Fichier | Changements |
|---------|------------|
| `src/config.rs` | +`has_simple_pipeline()`, +`has_kb_participation()` |
| `src/catalog.rs` | Validation relaxée, create_entity_tables conditionnel, re-trigger KBs, register_kb update, resolve_search_target KB-only error, migrate_entity conditionnel, is_registered_entity, build_content_text fix, ingest_entities KB trigger |
| `tests/e2e_idempotent_registration.rs` | +6 tests, +imports (Arc, KBConfig, BM25Mode, CandleEmbedder), +MINILM lazy static, bm25_mode sur tous les SearchOptions |
| `docs/.../12-design-register-order-independent.md` | Design doc |

## État de la compilation

- `cargo check` : ✅
- `cargo test --lib` : 544 tests passent ✅
- E2E tests 1-10 + 11 + 16 : ✅
- E2E tests 12-15 (KB) : ❌ en cours de debug
