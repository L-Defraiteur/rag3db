# 09 — Récap session Phase 0b (3 mars 2026)

## Ce qui a été fait

### 1. schema.rs — Architecture KB Index (FAIT, 35 tests schema, 361 total)

Refactoring complet de la génération DDL. Les entity tables sont du pur stockage (pas d'embedding). Chaque KB a ses propres tables d'index.

**Fonctions ajoutées :**
- `KBSchemaInfo` + `resolve_kb_title_entities(config)` — résout quelle entité est titleFor de chaque KB
- `generate_index_table_ddl(kb_name, kb_config, embedding_dim)` → `{KB}_Index` avec `_uuid`, `_source_entity`, `_source_uuid`, `_content_hash`, `_title`, `_content`, `{KB}_embedding`, sparse si activé
- `generate_index_chunk_table_ddl(kb_name, kb_config, embedding_dim)` → `{KB}_Index_Chunk` avec colonnes chunk standard + `_source_field STRING` + embeddings
- `generate_index_chunk_rel_ddl(kb_name)` → `{KB}_Index_HAS_CHUNK(FROM {KB}_Index TO {KB}_Index_Chunk)`
- `generate_index_rel_ddl(title_entity, kb_name)` → `{TitleEntity}_IN_{KB}(FROM {TitleEntity} TO {KB}_Index)`
- `generate_source_rel_ddl(entity_name, kb_name)` → `{Entity}_SOURCED_{KB}(FROM {Entity} TO {KB}_Index_Chunk)` — une par entité contribuant (titleFor ou contentFor) à la KB

**Fonctions modifiées :**
- `generate_node_table_ddl(entity_name, entity_def)` — signature simplifiée, plus de `embedding_dim`/`kb_configs`, plus de colonnes embedding
- `generate_full_schema()` — réécrit : meta → entity tables → user rels → pour chaque KB : index table + chunk table + rels (HAS_CHUNK, IN, SOURCED) + FTS + HNSW

**Fonctions supprimées :**
- `generate_chunk_table_ddl()` — remplacé par `generate_index_chunk_table_ddl()`
- `generate_chunk_rel_ddl()` — remplacé par `generate_index_chunk_rel_ddl()` + `generate_index_rel_ddl()`

**Conservé pour backward compat :**
- `entity_has_chunks()` — encore utilisé par catalog.rs, à retirer quand catalog sera migré

**Tests :** 35 tests schema (anciens adaptés + nouveaux : `resolve_kb_title_entities_basic`, `resolve_kb_title_entities_multi_entity`, `node_table_no_embedding_even_with_kb`, `index_table_basic`, `index_table_with_sparse`, `index_chunk_table_basic`, `index_chunk_table_with_sparse`, `index_chunk_rel_ddl`, `index_rel_ddl`, `index_rel_ddl_tree_kb`, `source_rel_ddl`, `source_rel_ddl_single_entity`, `full_schema_fts_on_kb_index`, `full_schema_multi_entity_kb`, `full_schema_wasm_config`, `full_schema_wasm_config_from_json`)

### 2. ops.rs — AggregateOp (FAIT)

**Ajouté :**
- `AggregateOp` struct : `index_entry_uuid`, `kb_name`, `title_entity`, `source_uuid`
- `OP_AGGREGATE` constante : prio 2.5, batch_size 50, max_retries 3
- Variant `CatalogOp::Aggregate(AggregateOp)` dans l'enum
- `priority()`, `operation_type()`, `config()` mis à jour pour le nouveau variant
- Test `aggregate_op_priority`

### 3. queue.rs — Match corrigé (FAIT)

- Ajout de `CatalogOp::Aggregate(_)` dans le match de `enqueue()` (ligne 327)

### 4. cypher_persistence.rs — Sérialisation AggregateOp (FAIT)

- Ajout du cas `CatalogOp::Aggregate` dans `extract_op_data()` : sérialise `kb_name`, `index_entry_uuid`, `title_entity`, `source_uuid` en JSON

---

## Flow d'ingestion clarifié

```
create("Directory", data)
  → InsertOp(1.0) pour l'entité
  → InsertOp(1.0) pour {KB}_Index entry (titre + content propres)
  → LinkOp(2.0) pour {TitleEntity}_IN_{KB}
  → AggregateOp(2.5) pour cette index entry

link("HAS_FILE", dir, file)
  → LinkOp(2.0)
  → AggregateOp(2.5) même UUID → dédupliqué

drain():
  1.0  InsertOps     → entités + index entries (données brutes, PAS de chunks)
  2.0  LinkOps       → toutes les relations (user rels + _IN_ rels)
  2.5  AggregateOps  → UN SEUL par index entry (dédupliqué par UUID)
                       → construit _content final (propre + agrégé)
                       → compare _content_hash → skip si inchangé
                       → chunk PAR CHAMP SOURCE (pas le texte concaténé)
                       → crée InsertOps chunks + LinkOps SOURCED + EmbedOps
  3.0  EmbedOps      → embedding UNE SEULE FOIS, sur le contenu final
```

**Pas de double embed.** Le create() n'enqueue PAS d'EmbedOp. C'est l'AggregateOp qui produit les chunks + EmbedOps.

---

## Décision architecturale : source tracking via relations, PAS colonnes

**Problème :** chunker le `_content` concaténé rend les `_start_char`/`_end_char` incohérents pour les multi-entity KBs — les offsets ne correspondent à aucun champ source réel.

**Décision :** les chunks sont créés **par champ source d'entité**, pas sur le texte concaténé. Le lien chunk → entité source se fait via **relations** `{Entity}_SOURCED_{KB}`, pas via des colonnes `_source_entity`/`_source_uuid` sur le chunk.

### Modèle final

- **`{KB}_Index._content`** = texte concaténé (pour BM25 full document, IDF cohérent). Pas de chunks dessus.
- **`{KB}_Index_Chunk`** = chunks créés par champ source individuel :
  - `_source_field STRING` — colonne sur le chunk, indique le champ d'origine ("absolute_path", "content")
  - `_start_char`/`_end_char` etc. — relatifs au champ source → **cohérents**
- **`{Entity}_SOURCED_{KB}`** = relation `(Entity) → ({KB}_Index_Chunk)` — une par entité contribuant à la KB. Tracke quelle entité a produit quel chunk (traversable, pas de données dupliquées).

### Relations par KB

```
(Directory)-[:Directory_IN_TreeKB]->(TreeKB_Index)           ← title entity owns index entry
(TreeKB_Index)-[:TreeKB_Index_HAS_CHUNK]->(TreeKB_Index_Chunk)  ← index entry → chunks
(Directory)-[:Directory_SOURCED_TreeKB]->(TreeKB_Index_Chunk)   ← source entity → chunks it produced
(File)-[:File_SOURCED_TreeKB]->(TreeKB_Index_Chunk)             ← source entity → chunks it produced
```

### Impact sur l'AggregateOp

L'AggregateOp fait deux choses distinctes :
1. **Reconstruit `_content`** sur `{KB}_Index` (concaténation pour BM25) + update `_content_hash`
2. **Gère les chunks par source** : pour chaque entité content liée, chunk ses champs individuellement → InsertOp(`{KB}_Index_Chunk`) + LinkOp(`{KB}_Index_HAS_CHUNK`) + LinkOp(`{Entity}_SOURCED_{KB}`) + EmbedOp

Il ne chunk PAS le texte concaténé.

### Impact sur le BM25 highlight resolution

Le BM25 indexe `_content` (concaténé). Les highlight offsets sont relatifs au texte concaténé. Pour résoudre vers les chunks (qui sont par champ source), il faut un mapping :
- Connaître l'offset de chaque champ source dans le `_content` concaténé
- `{KB}_Index` pourrait stocker ce mapping (ex: `_content_offsets JSON`)
- Ou recalculer à la volée au search time

**Alternative plus simple :** stocker `_content_offset INT64` sur chaque chunk = position du début du champ source dans le `_content` concaténé. Ainsi : `highlight_offset_in_concat - chunk._content_offset = offset_in_source_field`.

---

## Ce qui reste à faire

### A. catalog.rs — Migration `create()`

1. Après l'InsertOp de l'entité, pour chaque KB où l'entité a `titleFor` :
   - InsertOp pour `{KB}_Index` (avec `_title`, `_content` = champs propres, `_source_entity`, `_source_uuid`, `_content_hash`)
   - LinkOp pour `{TitleEntity}_IN_{KB}`
   - AggregateOp pour cette index entry
   - **NE PAS** créer de ChunkOp ni EmbedOp (c'est l'AggregateOp qui s'en charge)
2. Supprimer l'appel à `maybe_enqueue_chunk_op()` dans `create()`

### B. catalog.rs — Migration `link()`

3. Détecter si la relation relie une content entity à une title entity pour une KB → enqueue AggregateOp

### C. AggregateProcessor — Nouveau processor

4. Créer `AggregateProcessor` (implémente `Processor`) :
   - Déduplique les AggregateOps par `index_entry_uuid`
   - Pour chaque entry unique : query le graphe pour reconstruire `_content` agrégé
   - Compare `_content_hash` → skip si inchangé
   - Si changé : UPDATE `{KB}_Index`, delete anciens chunks, re-chunk **par champ source**, créer InsertOps(`{KB}_Index_Chunk`) + LinkOps(`{KB}_Index_HAS_CHUNK` + `{Entity}_SOURCED_{KB}`) + EmbedOps
5. Enregistrer `AggregateProcessor` dans `initialize()` à côté des autres processors

### D. catalog.rs — Migration `search()`

6. `entity` = `"{KB}_Index"` au lieu de `kb.title.entity`
7. `vector_entity` = `"{KB}_Index_Chunk"` au lieu de `"{Entity}_Chunk"`
8. `bm25_fields` = `["_title", "_content"]` (fixe)
9. Résolution : `_source_uuid` + SOURCED rels → entité source pour enrichissement
10. Filters sur `{KB}_Index` (`_source_entity`), plus sur l'entité

### E. catalog.rs — Update/Delete propagation

11. `update()` : si l'entité a titleFor → AggregateOp pour ses index entries. Si content entity → AggregateOp pour les index entries liées.
12. `delete()` : supprimer les index entries + chunks associés. Si content entity → AggregateOp pour les title entities liées.

### F. catalog.rs — Cleanup

13. Supprimer `maybe_enqueue_chunk_op()` — remplacé par AggregateProcessor
14. Refactorer ou supprimer `compute_chunk_ops()` — la logique de chunking passe dans AggregateProcessor, cible `{KB}_Index_Chunk`
15. Retirer import `entity_has_chunks` quand plus utilisé

### G. initialize() — Mise à jour

16. La boucle sparse vector index (l.300-327) cible `{Entity}_Chunk` → doit cibler `{KB}_Index_Chunk`

---

## État des fichiers modifiés

| Fichier | État |
|---------|------|
| `schema.rs` | MIGRÉ — KB Index architecture + SOURCED rels, 35 tests |
| `ops.rs` | MIGRÉ — AggregateOp ajouté, tests OK |
| `queue.rs` | Match corrigé pour Aggregate |
| `cypher_persistence.rs` | Sérialisation Aggregate ajoutée |
| `catalog.rs` | PAS ENCORE MODIFIÉ — prochaine étape (sections A-G ci-dessus) |
| `validator.rs` | PAS MODIFIÉ — fonctionne déjà (entities.len() > 1) |

## Tests : 361 passent, 0 échecs, 13 ignorés
