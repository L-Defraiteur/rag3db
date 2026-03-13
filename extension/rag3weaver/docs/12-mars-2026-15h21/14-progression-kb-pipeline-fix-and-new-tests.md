# Doc 14 — Progression : KB pipeline fix + 5 nouveaux tests E2E

Date : 12 mars 2026

Réf : doc 12, doc 13

## Bugs corrigés

### Bug 1 : `KBUpdateNode` utilisait MATCH au lieu de MERGE

**Symptôme** : `docs_Index` = 0 records après `ingest_entities()` pour KB-only entity, mais `docs_Index_Chunk` = 2 (chunks orphelins).

**Cause** : `KBUpdateNode` (record_nodes.rs ~ligne 2530) faisait `MATCH (idx:{index_table} {_uuid: item.uuid}) SET ...`. Pour des NOUVEAUX KB Index records (première ingestion), le MATCH ne trouvait rien → SET sur 0 rows → no-op silencieux.

**Fix** :
- `KBUpdateNode` : remplacé `MATCH` par `MERGE` avec `ON CREATE SET` (tous les champs : `_title`, `_content`, `_content_hash`, `_source_entity`, `_source_uuid`) et `ON MATCH SET` (juste title, content, hash)
- Ajouté step 5b : `MERGE` des relations `{Entity}_IN_{KB}` après la création des Index records
- Ajouté `source_entity` et `source_uuid` à `KBContentRecord` (records.rs) pour que `KBUpdateNode` ait les infos nécessaires pour le `ON CREATE SET`
- `KBGatherNode` : peuple les nouveaux champs depuis le `title_entity` et `source_uuid` du `AggregateRecord`

**Fichiers** : `src/records.rs`, `src/dataflow/record_nodes.rs`

### Bug 2 : Entité composite (is_content + content_for) — KB pipeline ne se déclenchait pas

**Symptôme** : Pour une entité avec À LA FOIS `is_content` (simple pipeline) ET `content_for` (KB), le simple pipeline marchait mais `{KB}_Index` = 0.

**Cause** : Dans `ingest_entities()` KB trigger, `build_content_text()` utilisait les champs `is_content` (car `content_fields()` n'est pas vide pour une composite). Le hash résultant était IDENTIQUE au `_content_hash` déjà stocké par le simple pipeline → `UpdateRecordNode` voyait aucun changement → pas d'AggregateRecord → KB pipeline skip.

**Fix** : Utiliser un hash sentinel vide (`String::new()`) comme `new_content_hash` dans les UpdateRecords du KB trigger. Force `UpdateRecordNode` à toujours détecter un changement et enqueuer les AggregateRecords.

**Fichier** : `src/catalog.rs` (ingest_entities KB trigger, ~ligne 1319)

### Bug 3 : `register_kb()` avant `register_entity()` échouait

**Symptôme** : `SchemaError("KB 'library': no entity has a field with title_for=\"library\"")` quand `register_kb()` est appelé avant `register_entity()`.

**Cause** : `register_kb()` exigeait qu'un entity avec `title_for` existe déjà dans `config.entities`.

**Fix** (2 parties) :
1. `register_kb()` : si aucun entity avec `title_for` n'existe encore, persiste la config KB sans créer les tables ni le `KBMetadata`. Les tables seront créées quand `register_entity()` re-trigger `register_kb()` plus tard.
2. `register_entity()` re-trigger : vérifiait `self.kb_metadata.contains_key(kb)` pour décider de re-trigger. Changé pour vérifier AUSSI `self.config.knowledge_bases.contains_key(kb)` — car la KB peut être pré-enregistrée sans être dans `kb_metadata`.

**Fichier** : `src/catalog.rs` (register_kb ~ligne 650, register_entity re-trigger ~ligne 353)

## 5 nouveaux tests E2E (tests 17-21)

Tous dans `tests/e2e_idempotent_registration.rs`.

| # | Test | Feature gate | Statut |
|---|------|-------------|--------|
| 17 | `composite_entity_simple_and_kb_coexist` | candle-embedder | ✅ PASSE |
| 18 | `register_kb_before_entity_order_independent` | — | ✅ PASSE |
| 19 | `multi_entity_kb_partial_migration` | candle-embedder | ✅ PASSE |
| 20 | `delete_entity_cleans_kb_index` | — | ✅ PASSE |
| 21 | `kb_incremental_ingest_across_sessions` | — | ✅ PASSE |

### Test 17 — Composite entity (is_content + content_for)

Entité "Recipe" avec :
- `name` : is_title (simple pipeline)
- `summary` : is_content (simple pipeline)
- `recipe_title` : title_for "cookbook" (KB)
- `instructions` : content_for "cookbook" (KB)

Vérifie que `search("Recipe", ...)` (simple) ET `search("cookbook", ...)` (KB) fonctionnent après ingestion, migration (ajout champ `tips` content_for), et reindex. Utilise MiniLM réel pour HYBRID (BM25 + vector).

Note design : `is_title` et `title_for` sont mutuellement exclusifs **par champ**, pas par entité. Donc `name` est is_title et `recipe_title` est title_for — deux champs séparés.

### Test 18 — Ordre inversé (register_kb avant register_entity)

1. `register_kb("library")` — aucune entité n'existe encore
2. `register_entity("Book")` avec `title_for: "library"` — auto re-trigger KB
3. Ingest 2 books → KB search fonctionne
4. Register 2ème entité "Chapter" pour même KB → ingest → KB search trouve les 3

### Test 19 — Multi-entity KB, migration partielle

- Lesson (title entity pour KB "knowledge") + Exercise (content-only, relié par HAS_EXERCISE)
- Ingest lessons → KB search
- Migrate Lesson (ajout champ `prerequisites` content_for) → reindex SEULEMENT Lesson
- KB search toujours OK
- Ingest nouveau Lesson avec prerequisites → trouvable dans KB

Note : une KB a UN SEUL title entity. HashMap `resolve_kb_title_entities` écrase si deux entities ont `title_for` pour la même KB. Les entities additionnelles contribuent via `content_for` + relation.

### Test 20 — Delete entity → KB index nettoyé

- 3 notes KB-only → delete 1 → drain → 2 notes restantes
- KB search ne retourne plus le contenu supprimé
- Les 2 autres notes toujours trouvables

### Test 21 — Ingest incrémental à travers 3 sessions

- Session 1 : register + ingest 2 parts → search OK → close
- Session 2 : reopen → old data searchable → ingest 1 more → all 3 searchable → close
- Session 3 : reopen → migrate (add field) → reindex 3 → search all 3

## Résultat final

```
═══════════════════════════════════════════════
  SUMMARY
═══════════════════════════════════════════════
  e2e_idempotent_registration     21 passed
───────────────────────────────────────────────
  TOTAL                           21 passed
═══════════════════════════════════════════════
```

- `cargo test --lib` : 537 tests passent ✅
- E2E tests 1-21 : tous ✅

## Fichiers modifiés (depuis doc 13)

| Fichier | Changements |
|---------|------------|
| `src/records.rs` | +`source_entity`, +`source_uuid` sur `KBContentRecord` |
| `src/dataflow/record_nodes.rs` | KBUpdateNode: MERGE au lieu de MATCH + MERGE IN_KB rels. KBGatherNode: peuple source_entity/source_uuid |
| `src/catalog.rs` | register_kb() accepte appel sans entities. register_entity() re-trigger vérifie knowledge_bases en plus de kb_metadata. ingest_entities KB trigger: hash sentinel vide |
| `tests/e2e_idempotent_registration.rs` | +5 tests (17-21), cleanup debug eprintln du test 12 |

## Prochaines étapes possibles

- Ajouter des tests avec sparse embeddings (SPARSE signal)
- Tester dual embedder (dense + sparse simultané)
- Multi-title-entity KB (design decision : est-ce qu'on veut supporter ça ?)
- Nettoyage : vérifier que les 537 unit tests passent avec les nouvelles modifs
