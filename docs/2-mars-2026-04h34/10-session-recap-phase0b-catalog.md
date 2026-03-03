# 10 — Récap session Phase 0b — Migration catalog.rs (3 mars 2026)

## Ce qui a été fait dans cette session

### 1. schema.rs — SOURCED rels (FAIT, 361 tests)

- `generate_source_rel_ddl(entity_name, kb_name)` → `{Entity}_SOURCED_{KB}(FROM {Entity} TO {KB}_Index_Chunk)`
- Intégré dans `generate_full_schema()` : pour chaque KB, itère sur toutes les entités, si l'entité contribue (titleFor ou contentFor) → génère la SOURCED rel
- `_source_field STRING` ajouté comme colonne sur `{KB}_Index_Chunk`
- 2 nouveaux tests : `source_rel_ddl`, `source_rel_ddl_single_entity`
- Tests existants mis à jour : `full_schema_order` vérifie `Document_SOURCED_main`, `full_schema_multi_entity_kb` vérifie `Directory_SOURCED_TreeKB` + `File_SOURCED_TreeKB`

### 2. catalog.rs — `create()` migré (FAIT, 361 tests)

Ancien flow :
```
create("Document", data) → InsertOp(entity) + ChunkOp
```

Nouveau flow :
```
create("Document", data)
  → InsertOp(entity)                    prio 1.0
  → Pour chaque KB où entity a titleFor :
    → InsertOp({KB}_Index)              prio 1.0
    → LinkOp({Entity}_IN_{KB})          prio 2.0
    → AggregateOp                       prio 2.5
```

**Code modifié :** `create()` (anciennement l.335) — remplacé `maybe_enqueue_chunk_op()` par la boucle `resolve_entity_kbs()` qui crée les ops KB Index.

**Import ajouté :** `AggregateOp` et `resolve_entity_kbs` dans les imports catalog.rs.

### 3. catalog.rs — AggregateProcessor stub (FAIT)

- Struct `AggregateProcessor` créée avec : `conn`, `config`, `kb_metadata`, `chunker_cache`, `has_sparse`, `has_dual`
- Implémente `Processor` trait — **no-op pour l'instant** (marque les items comme processed, déduplique par `index_entry_uuid`)
- Enregistré dans `initialize()` avec `"aggregate"` comme clé, juste après le LinkProcessor
- Le chunker_cache est construit via `warm_chunker_cache()` + `std::mem::take()` (comme ChunkProcessor)

### 4. catalog.rs — `initialize()` sparse index migré (FAIT)

Ancien code : itérait sur `kb_meta.entities` → `{Entity}_Chunk` ou `entity` pour créer les sparse vector indexes.

Nouveau code : une seule table par KB → `{KB}_Index_Chunk` directement.

### 5. catalog.rs — `search()` migré (FAIT)

Changements :
- `entity` = `"{KB}_Index"` (était `kb.title.entity`)
- `vector_entity` = `"{KB}_Index_Chunk"` (était `"{Entity}_Chunk"`)
- `bm25_fields` = `["_title", "_content"]` fixe (était calculé depuis les champs de l'entité)
- `is_chunked` = `true` toujours (les KB Index ont toujours des chunks)
- `enrich_fields` = `["_title", "_content", "_source_entity", "_source_uuid", "_content_hash"]` (était les champs user de l'entité)

### 6. catalog.rs — `update()` migré (FAIT)

Ancien code : si content changé → delete `{Entity}_Chunk` + enqueue `ChunkOp`.

Nouveau code : si content changé → pour chaque KB où entity a `titleFor` → enqueue `AggregateOp` (qui gèrera delete chunks + re-chunk + re-embed).

**TODO :** si l'entity est contentFor (pas titleFor), il faut trouver les title entities liées via les relations et enqueue AggregateOps pour elles.

### 7. catalog.rs — `delete()` migré (FAIT)

Ancien code : delete `{Entity}_Chunk` + DETACH DELETE entity.

Nouveau code : pour chaque KB où entity a `titleFor` → delete `{KB}_Index_Chunk` (par `_parent_uuid`) + delete `{KB}_Index` entry → puis DETACH DELETE entity.

### 8. Tests mis à jour (FAIT)

- `ops_enqueued_per_create()` : `2` → `4` (1 InsertOp entity + 1 InsertOp index + 1 LinkOp IN + 1 AggregateOp)
- `ops_per_create()` : ancienne formule `1 + 1 + n*3` → `4` (2 inserts + 1 link + 1 aggregate no-op)
- `flush_insertions_only` : adapté (flush prio ≤ 1.0 = 2 InsertOps, puis drain = 1 LinkOp + 1 AggregateOp)

### 9. Doc 09 mis à jour (FAIT)

Corrigé pour refléter : 361 tests, 35 tests schema, SOURCED rels (pas colonnes), flow correct avec LinkOp(_IN_).

---

## Warnings restants (non-critiques)

```
warning: unused import: `FilterOp`                              → dans un test, trivial
warning: unused variable: `kb`                                  → dans search(), variable shadowed par les changements
warning: method `maybe_enqueue_chunk_op` is never used          → À SUPPRIMER (section F)
warning: fields `conn`, `config`, ... are never read            → AggregateProcessor stub, normal
warning: function `count_chunks` is never used                  → Helper de test obsolète, à supprimer
```

---

## État actuel : 361 tests passent, 0 échecs, 13 ignorés

---

## Ce qui reste à faire

### A. AggregateProcessor — Implémenter le vrai processing

Le stub est en place. Il faut implémenter :
1. Pour chaque AggregateOp unique (dédupliqué par `index_entry_uuid`) :
2. Query le graphe : `MATCH (idx:{KB}_Index {_uuid: $uuid})` → get current `_content_hash`
3. Traverser les relations pour collecter le content de toutes les entités liées
4. Reconstruire `_content` (concaténation pour BM25)
5. Comparer `_content_hash` → skip si inchangé
6. Si changé :
   - UPDATE `{KB}_Index` (`_content`, `_content_hash`)
   - Delete anciens chunks : `MATCH (c:{KB}_Index_Chunk {_parent_uuid: $uuid}) DETACH DELETE c`
   - Re-chunk **par champ source** (pas le texte concaténé)
   - Pour chaque chunk : InsertOp(`{KB}_Index_Chunk`) + LinkOp(`{KB}_Index_HAS_CHUNK`) + LinkOp(`{Entity}_SOURCED_{KB}`) + EmbedOp
   - Émettre via `sender.emit_all()`

### B. Cleanup catalog.rs

- Supprimer `maybe_enqueue_chunk_op()` — plus appelé
- Supprimer ou marquer `compute_chunk_ops()` comme deprecated — la logique de chunking passe dans AggregateProcessor
- Supprimer `count_chunks()` helper de test
- Retirer l'import `entity_has_chunks` si plus utilisé (vérifier)
- Fix warnings (`FilterOp`, `kb` variable)

### C. `link()` — Ingestion incrémentale

Quand un `link()` est fait **après** un drain initial (cas incrémental), il faut détecter si la relation relie une content entity à une title entity pour une KB → enqueue AggregateOp.

Pour le flow batch (create → link → drain), c'est déjà couvert par l'AggregateOp enqueué dans `create()`.

### D. `update()` — Content entity propagation

Si l'entity mise à jour est **contentFor** (pas titleFor), il faut :
1. Trouver les title entities liées via les relations de la config
2. Pour chaque title entity liée, enqueue un AggregateOp pour son index entry

### E. Search enrichment

Le `search()` retourne maintenant des données de `{KB}_Index` (`_title`, `_content`, `_source_entity`, `_source_uuid`). Il faut peut-être aussi enrichir avec les données de l'entité source (via `_source_uuid` → MATCH entity). À décider si c'est nécessaire pour la Phase 1.

### F. ChunkProcessor — Garder ou supprimer ?

Le `ChunkProcessor` + `compute_chunk_ops()` sont encore dans le code mais plus appelés (pas de ChunkOp enqueué). Options :
1. Les garder pour backward compat (si un code externe enqueue des ChunkOps)
2. Les supprimer complètement (plus propre)

Recommandation : supprimer, la logique de chunking vit maintenant dans AggregateProcessor.

---

## Fichiers modifiés (cette session)

| Fichier | Changements |
|---------|------------|
| `schema.rs` | +`generate_source_rel_ddl()`, intégré dans `generate_full_schema()`, `_source_field` sur chunk table, +2 tests |
| `catalog.rs` | `create()` migré (KB Index ops), `AggregateProcessor` stub, `initialize()` sparse migré, `search()` migré, `update()` migré, `delete()` migré, tests adaptés |
| `docs/09-session-recap-phase0b.md` | Mis à jour (361 tests, SOURCED rels, flow corrigé) |

## Fichiers NON modifiés

| Fichier | Raison |
|---------|--------|
| `ops.rs` | Déjà migré session précédente |
| `queue.rs` | Déjà migré session précédente |
| `cypher_persistence.rs` | Déjà migré session précédente |
| `validator.rs` | Fonctionne déjà |
