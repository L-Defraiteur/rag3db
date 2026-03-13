# Doc 10 — Design : reindex() après changement de schéma

Date : 12 mars 2026

Réf : doc 07 (design registration idempotente), doc 09 (plan implémentation)

## 1. Problème

Quand `register_entity()` détecte un changement de content/title fields, il :
- Rebuild l'index FTS (vide)
- Flag `needs_reindex:{entity}` dans `_catalog_meta`
- Log un warning

Tant que `reindex()` n'est pas appelé, le search est dégradé (FTS vide, embeddings partiels).

## 2. Deux pipelines, deux mécanismes

### 2.1 Simple entity (`is_content`/`is_title`)

```
UpdateRecordNode (détecte hash mismatch)
  → RechunkDeleteNode (supprime anciens chunks)
  → ChunkRecordNode (re-chunk)
  → InsertRecordNode (insert chunks)
  → LinkRecordNode (CHUNKED_FROM)
  → EmbedNode (re-embed)
  → FlushNode (rebuild FTS)
```

Le hash est sur l'entité : `_content_hash` = blake3 de `content_fields().join("\n\n")`.

Après ajout d'un content field, `content_fields()` retourne un set différent → le hash calculé à partir des données actuelles (nouveau champ = `''`) diffère de l'ancien → UpdateRecordNode détecte le changement → rechunk.

### 2.2 KB entity (`title_for`/`content_for`)

```
AggregateRecord (trigger)
  → KBGatherNode (re-gather depuis toutes les entités)
  → KBUpdateNode (update {KB}_Index + delete old chunks)
  → KBChunkNode (re-chunk contenu agrégé)
  → InsertRecordNode (insert chunks)
  → LinkRecordNode (HAS_CHUNK + SOURCED)
  → EmbedNode (re-embed)
  → FlushNode (rebuild FTS)
```

Le hash est sur `{KB}_Index._content_hash` = hash du contenu agrégé (titre + contenu de toutes les entités participantes).

KBGatherNode lit les données depuis les tables entités, résout les champs `content_for`/`title_for`, et calcule un nouveau hash. Si le set de champs a changé (nouveau field), le hash diffère → re-chunk.

### 2.3 Subtilité : `content_for` sans `title_for`

Si l'entité A a `content_for: ["docs"]` mais PAS `title_for`, elle n'a pas d'entrée directe dans `{KB}_Index`. C'est l'entité B (qui a `title_for: "docs"`) qui crée les entrées.

Pour reindex A :
1. Trouver les KBs auxquels A contribue
2. Pour chaque KB, trouver les title entities liées à A via relation
3. Générer des AggregateRecords pour ces title entities (pas pour A)
4. KBGatherNode re-gather → inclut les données de A avec le nouveau champ

### 2.4 Cas mixte

Une entité peut avoir :
- `is_content: true` sur un champ (pipeline simple)
- `content_for: ["docs"]` sur un autre champ (pipeline KB)

Les deux pipelines doivent être déclenchés.

## 3. Approche

```rust
pub async fn reindex(&mut self, entity_name: &str) -> Result<ReindexStats, CatalogError>
```

### 3.1 Étapes

1. Vérifier que `needs_reindex:{entity}` est flagué (ou forcer avec un param optionnel)
2. Query tous les UUIDs + données de l'entité
3. Déterminer les pipelines à déclencher :
   - Simple ? (au moins un champ `is_content` ou `is_title`)
   - KB ? (au moins un champ `title_for` ou `content_for`)
4. Pour chaque pipeline, enqueue les records appropriés
5. `drain()`
6. Clear `needs_reindex:{entity}` dans `_catalog_meta`

### 3.2 Pipeline simple

```
for each record (uuid, data):
    new_content_hash = build_content_text(entity, data) → content_hash()
    pending.updates.push(UpdateRecord { entity_name, uuid, data, new_content_hash })
```

À drain(), UpdateRecordNode compare avec `_content_hash` en DB → mismatch → rechunk.

### 3.3 Pipeline KB

```
for each KB where entity has title_for:
    for each record uuid:
        index_uuid = hashsafe_uuid("{KB}_Index", [entity_name, uuid])
        pending.aggregates.push(AggregateRecord {
            index_entry_uuid: index_uuid,
            kb_name, title_entity: entity_name, source_uuid: uuid
        })

for each KB where entity has content_for (but NOT title_for):
    query linked title entities via relation
    for each (title_entity_name, title_uuid):
        index_uuid = hashsafe_uuid("{KB}_Index", [title_entity_name, title_uuid])
        pending.aggregates.push(AggregateRecord {
            index_entry_uuid: index_uuid,
            kb_name, title_entity: title_entity_name, source_uuid: title_uuid
        })
```

À drain(), KBGatherNode re-gather le contenu incluant le nouveau champ → hash mismatch → rechunk.

### 3.4 Lecture des données

Pour le pipeline simple, on a besoin de toutes les données de l'entité (pour calculer le nouveau content hash et pour que UpdateRecordNode puisse émettre des EntityRecords).

Query :
```cypher
MATCH (n:{Entity}) RETURN n._uuid, n.field1, n.field2, ...
```

Pour le pipeline KB, on n'a besoin que des UUIDs (KBGatherNode re-lit les données lui-même).

Pour le cas content_for-only, on a besoin des title entities liées. Query :
```cypher
MATCH (n:{Entity})-[:{Relation}]-(t:{TitleEntity}) RETURN DISTINCT t._uuid
```

### 3.5 Batching

Si l'entité a beaucoup de records, on batch :
- Lire les UUIDs par batch de 1000
- Enqueue + drain par batch
- Avantage : mémoire bornée

Ou plus simple : tout enqueue, un seul drain(). drain() est déjà batch-aware (InsertRecordNode fait des UNWIND groupés). Le batching est surtout nécessaire si on veut afficher un progressbar.

Pour une v1, un seul drain() suffit.

## 4. Retour (`ReindexStats`)

```rust
pub struct ReindexStats {
    pub entity: String,
    pub records_processed: usize,
    pub simple_rechunked: usize,
    pub kb_aggregates_enqueued: usize,
}
```

## 5. Vérification

`reindex()` est idempotent : re-appeler ne cause pas de dommage (KBGatherNode et UpdateRecordNode skip si hash n'a pas changé).

## 6. Limitations v1

- Pas de batching intermédiaire (tout en un drain)
- Pas de progressbar
- `content_for`-only : nécessite une relation existante vers la title entity (sinon on ne peut pas trouver les title entities liées — erreur explicite)
- Reindex une seule entité à la fois (pas de `reindex_all()`)

## 7. Tests

- `reindex_simple_entity` — ajouter un content field, reindex → chunks recréés
- `reindex_kb_title_entity` — ajouter un content_for field sur title entity, reindex → KB index entries rechunkées
- `reindex_kb_content_entity` — ajouter un content_for field sur content-only entity, reindex → KB index entries rechunkées via title entity
- `reindex_clears_flag` — après reindex, `needs_reindex:{entity}` supprimé
- `reindex_idempotent` — deux appels = même résultat
