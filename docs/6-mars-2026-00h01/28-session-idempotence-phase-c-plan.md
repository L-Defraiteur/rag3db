# Doc 28 — Session : Idempotence des Record Nodes + Plan Phase C

Date : 7 mars 2026

## Contexte

Le doc 27 a implémenté les static nodes, le système de metrics, et le NodeEventFilter. Il a aussi posé le design pour l'idempotence et le checkpoint. Cette session implémente l'idempotence.

## Travail effectué — Idempotence des Record Nodes

### 1. InsertRecordNode — CREATE → MERGE

**Avant** :
```cypher
UNWIND $items AS item
CREATE (n:Entity {col1: item.col1, col2: item.col2, ...})
RETURN ID(n), item._uuid
```

**Après** :
```cypher
UNWIND $items AS item
MERGE (n:Entity {_uuid: item._uuid})
SET n.col1 = item.col1, n.col2 = item.col2, ...
RETURN ID(n), item._uuid
```

Le MERGE matche sur `_uuid` uniquement. Si l'entité existe déjà, les colonnes sont mises à jour (pas de doublon). Les colonnes `_uuid` sont extraites du SET (pas de `SET n._uuid = item._uuid` redondant).

### 2. LinkRecordNode — CREATE → MERGE

**Avant** :
```cypher
UNWIND $items AS item
MATCH (a {_uuid: item.from_uuid}), (b {_uuid: item.to_uuid})
CREATE (a)-[:REL {prop1: item.prop1}]->(b)
```

**Après** :
```cypher
UNWIND $items AS item
MATCH (a {_uuid: item.from_uuid}), (b {_uuid: item.to_uuid})
MERGE (a)-[r:REL]->(b)
SET r.prop1 = item.prop1, ...
```

Le MERGE matche sur (from, to, rel_type). Si la relation existe, les propriétés sont mises à jour via SET.

### 3. EmbedRecordNode — Hash-check avant GPU

Ajout d'un champ `_embed_hash` persisté sur chaque entité après embedding. Au prochain run :

1. Collecte tous les work items (dense + sparse + dual)
2. Query DB par entité pour lire les `_embed_hash` existants
3. `retain(is_changed)` — filtre les work items dont le hash n'a pas changé
4. Seuls les items changés passent au GPU
5. Après embedding, persiste `_embed_hash = content_hash(embed_text)` via SET

**Détails** :
- `EmbedWork` a un nouveau champ `text_hash: String`
- Le hash est calculé via `content_hash()` (même fonction que pour `_text_hash` des chunks)
- La query DB utilise `WHERE n._embed_hash IS NOT NULL` pour ne récupérer que les existants
- Les 4 chemins d'écriture (dense standalone, sparse standalone, dual dense, dual sparse) persistent tous `_embed_hash`
- Metric `skipped_unchanged` rapporte le nombre d'items filtrés avant le batch GPU

### 4. ChunkRecordNode — Déjà idempotent

UUIDs déterministes via `chunk_uuid(parent_uuid, field_name, index)`. Relancer produit les mêmes UUIDs → le MERGE en aval (InsertRecordNode) gère la dédup.

### 5. AggregateRecordNode — Déjà idempotent

Compare `_content_hash` avant de recalculer. Skip les entrées inchangées.

### Tableau récapitulatif

| Nœud | Mécanisme | Coût du replay |
|---|---|---|
| InsertRecordNode | MERGE sur `_uuid` | 1 query/groupe (pas de doublon, update si changé) |
| LinkRecordNode | MERGE sur (from, to, type) | 1 query/groupe (skip si existe) |
| EmbedRecordNode | `_embed_hash` check → skip GPU | 1 query DB de lecture (économise N×GPU) |
| ChunkRecordNode | UUIDs déterministes | CPU only (pas de DB) |
| AggregateRecordNode | `_content_hash` check | 1 query DB de lecture |

## Observation — ChunkRecordNode entity-level

Discussion sur l'utilité de `ChunkRecordNode` : il produit des chunks par entité (`File_Chunk`, `DocPage_Chunk`) mais la recherche ne cible que `{KB}_Index_Chunk` (produit par `AggregateRecordNode`). Les entity-level chunks ne sont consommés par aucun chemin de recherche.

**Décision** : garder `ChunkRecordNode` dans le code mais ne pas le connecter dans le graphe par défaut. Il servira quand les templates Mermaid permettront de choisir entre deux topologies :
- **Simple** : Insert → Chunk → Embed (pas de KB Index, entity-level chunks directement)
- **MetaKB** : Insert → Link → Aggregate → Insert → Embed (KB Index avec agrégation cross-entity)

Commentaire ajouté dans le module header de `record_nodes.rs`.

## Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/dataflow/record_nodes.rs` | MERGE dans InsertNode + LinkNode, hash-check dans EmbedNode, doc comments |

## Validation

- `cargo check --lib` : compile clean
- `cargo test --lib` : **392 pass**, 0 fail (0 régression)

## Plan — Phase C : Câbler les Record Nodes dans le pipeline

### Objectif

Remplacer `build_ingestion_graph()` pour qu'il consomme `PendingWork` (entities, relations, aggregates) au lieu de `pending_ops` (Vec<CatalogOp>). Les record nodes deviennent le pipeline actif.

### Sous-tâches

| # | Tâche | Détail |
|---|---|---|
| C.1 | `build_record_ingestion_graph()` | Nouvelle fonction qui construit le graphe record-based depuis `PendingWork`. Topologie : InsertRecordNode → LinkRecordNode → AggregateRecordNode → InsertRecordNode("agg_inserts") → EmbedRecordNode("agg_embeds"). ChunkRecordNode non connecté (commentaire: futur Mermaid template). |
| C.2 | `drain()` redirigé | `drain()` appelle `build_record_ingestion_graph()` au lieu de `build_ingestion_graph()`. L'ancien graphe batch reste en place temporairement (Phase D le supprimera). |
| C.3 | Services registry | Vérifier que tous les services nécessaires sont enregistrés (conn, node_id_cache, embedder, embedding_dim, config, kb_metadata, chunker_cache, sparse_embedder, dual_embedder, has_sparse, has_dual). |
| C.4 | `create()` / `link()` réécrit | `create()` pousse dans `self.pending.entities` + `self.pending.relations` + `self.pending.aggregates` au lieu de `self.pending_ops`. `link()` pousse dans `self.pending.relations`. |
| C.5 | `update()` / `delete()` adaptés | Même migration pour update (qui génère des InsertOps + LinkOps + AggregateOps). |
| C.6 | Unit tests | Vérifier `cargo test --lib` — 392+ tests pass |
| C.7 | E2E tests | `./run_e2e.sh` — tous les E2E passent (e2e_native, e2e_search, e2e_phase0b, e2e_result_mode, e2e_batch_observe, e2e_dataflow_observe, e2e_search_queue) |

### Topologie du nouveau graphe

```
PendingWork.entities ──→ InsertRecordNode("inserts")
                              │
                         ──inserted──→ EmbedRecordNode("entity_embeds")
                         ──done──────→ LinkRecordNode("links") ←── PendingWork.relations
                                           │
                                      ──done──→ AggregateRecordNode("aggregate") ←── PendingWork.aggregates
                                                     │
                                                ──entities──→ InsertRecordNode("agg_inserts")
                                                ──relations─→ LinkRecordNode("agg_links")
                                                                  agg_inserts ──done──→ EmbedRecordNode("agg_embeds")
```

Note : pas de ChunkRecordNode. Les entity-level chunks seront activables via un futur template Mermaid.

### Risques

- **EntityRef resolution** : les record nodes utilisent `EntityRef`/`RelationRef` pour la résolution cross-nœud. Il faut que `create()` les construise correctement et que `InsertRecordNode` les resolve avant que `LinkRecordNode` n'en ait besoin (garanti par l'edge `done`).
- **PendingWork vide** : si aucune entité/relation/aggregate, ne pas créer le nœud correspondant (même pattern conditionnel que l'ancien `build_ingestion_graph()`).
- **E2E break** : les tests E2E testent le pipeline entier (create → drain → search). Si la topologie change, les résultats de recherche doivent rester identiques.

## Prochaine étape

Implémenter Phase C (C.1 à C.7), valider avec `cargo test --lib` + `./run_e2e.sh`, puis Phase D (cleanup).
