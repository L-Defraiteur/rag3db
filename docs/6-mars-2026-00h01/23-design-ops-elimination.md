# Doc 23 — Design : Elimination des Ops — Le graphe EST le plan

Date : 6 mars 2026

## 1. Constat

Le pipeline d'ingestion a deux couches qui encodent la même information :

```
create("Document", {title, body})
  ↓
  compute_ops() → [InsertOp(Document), InsertOp(KB_Index), LinkOp(IN_KB), AggregateOp]
  ↓
  SplitOpsNode → route InsertOp vers InsertBatchNode, LinkOp vers LinkBatchNode, etc.
  ↓
  Chaque BatchNode exécute "son type" d'op
```

**Couche 1 — Les Ops** : `CatalogOp::Insert`, `CatalogOp::Link`, `CatalogOp::Embed`... disent "quoi faire".
**Couche 2 — Le Graphe** : `InsertBatchNode`, `LinkBatchNode`, `EmbedBatchNode`... disent "quoi faire".

C'est redondant. La présence d'un `EmbedBatchNode` connecté après un `InsertBatchNode` **est** l'instruction "embedder après insertion". Pas besoin d'un `EmbedOp` pour le dire — c'est l'edge qui le dit.

### Ce que les ops transportent vraiment

| Op | Données métier | Instruction |
|---|---|---|
| `InsertOp` | entity_name, data, entity_ref, resolver | "insère cette entité" |
| `LinkOp` | rel_name, from, to, properties, resolver | "crée cette relation" |
| `EmbedOp` | entity_ref, kb_name, texts | "embedde ces textes" |
| `SparseEmbedOp` | entity_ref, kb_name, texts | "embedde sparse ces textes" |
| `DualEmbedOp` | entity_ref, kb_name, texts | "embedde dual ces textes" |
| `ChunkOp` | entity_name, parent_uuid, data | "chunke cette entité" |
| `AggregateOp` | index_entry_uuid, kb_name, title_entity | "reconstruis cet index" |

La partie "instruction" est redondante avec le nœud qui traite l'op. La partie "données métier" est ce qui devrait circuler sur les ports.

### Autres vestiges pré-graphe

- **`OrderedPriority`** — Chunk=0.0, Insert=1.0, Link=2.0, Aggregate=2.5, Embed=3.0. C'était l'ordonnancement du `OperationQueue`. Le graphe encode l'ordre via les edges : insert avant link car l'edge `InsertNode.done → LinkNode.trigger` l'impose. Plus besoin de priorités numériques.

- **`OperationConfig`** — batch_size, max_retries par type d'op. Le batch_size est une propriété du nœud (pas de l'op). Les retries sont une responsabilité du runtime.

- **`SplitOpsNode`** — Existe uniquement pour démixer un `Vec<CatalogOp>` en 7 vecteurs typés. Si les données sont déjà typées à la source, le split est inutile.

- **8 `PortType` variants** — Ops, Inserts, Links, Chunks, Aggregates, Embeds, SparseEmbeds, DualEmbeds. Chacun wrappe un `Vec<XxxOp>` dans un `BatchPayload`. Si les ops disparaissent, ces variants aussi.

## 2. Vision — Le graphe est le plan d'exécution

### Principe

Les données métier (entités, relations, textes, chunks, embeddings) circulent sur les ports. La **topologie du graphe** encode les transformations. Aucune structure intermédiaire ne dit aux nœuds quoi faire — c'est leur position dans le graphe qui le détermine.

### Nouvelles structures de données

Les ops sont remplacés par des records de données pures :

```rust
/// Une entité prête à être insérée (remplace InsertOp).
pub struct EntityRecord {
    pub entity_name: String,
    pub data: BTreeMap<String, CypherValue>,
    pub entity_ref: EntityRef,
    pub resolver: Option<EntityRefResolver>,
}

/// Une relation prête à être créée (remplace LinkOp).
pub struct RelationRecord {
    pub rel_name: String,
    pub from: RefOrUuid,
    pub to: RefOrUuid,
    pub properties: BTreeMap<String, CypherValue>,
    pub relation_ref: RelationRef,
    pub resolver: Option<RelationRefResolver>,
}

/// Un index KB à reconstruire (remplace AggregateOp).
pub struct AggregateRecord {
    pub index_entry_uuid: String,
    pub kb_name: String,
    pub title_entity: String,
    pub source_uuid: String,
}
```

Note : **pas de ChunkRecord ni d'EmbedRecord au niveau de l'API**. Les chunks sont produits *par* le ChunkNode. Les embeddings sont produits *par* l'EmbedNode. L'utilisateur ne dit jamais "chunke ceci" ou "embedde cela" — c'est le graphe qui décide en fonction de la config.

### Nouveau graphe d'ingestion

```
                    EntityRecords
                         │
                    InsertNode ──────→ done
                         │ (entities insérées)
                         │
              ┌──────────┼──────────┐
              ↓          ↓          ↓
         ChunkNode   LinkNode   AggregateNode
              │                      │
              ↓                      ↓
        EmbedNode              (re-chunk + re-embed via DynamicNode)
```

Comparé à avant :

```
AVANT:                              APRÈS:
Vec<CatalogOp> (mixed)              EntityRecords + RelationRecords (typés)
       ↓                                    ↓              ↓
  SplitOpsNode (routeur)            InsertNode         LinkNode
   ↓  ↓  ↓  ↓  ↓  ↓  ↓                 ↓
  7 BatchNodes typés                ChunkNode → EmbedNode
```

**SplitOpsNode disparaît** — les données sont typées dès la source.

### Ce que fait chaque nœud

| Nœud | Input | Output | Rôle |
|---|---|---|---|
| **InsertNode** | `entities: Vec<EntityRecord>` | `done: Empty`, `inserted: Vec<EntityRecord>` (avec refs résolues) | CREATE Cypher, resolve refs, cache node_id |
| **LinkNode** | `relations: Vec<RelationRecord>` | `done: Empty` | MATCH+CREATE Cypher, resolve refs |
| **ChunkNode** | `entities: Vec<EntityRecord>` | `chunks: Vec<EntityRecord>` (les chunks), `chunk_links: Vec<RelationRecord>` | Split textes, produit chunks + relations HAS_CHUNK. DynamicNode : émet EmbedNode(s) downstream |
| **EmbedNode** | `entities: Vec<EntityRecord>` | `done: Empty` | Appelle Embedder, UNWIND SET. Config interne : dense/sparse/dual selon les signals KB |
| **AggregateNode** | `aggregates: Vec<AggregateRecord>` | `done: Empty` | DynamicNode : query graph, rebuild content, émet InsertNode + ChunkNode + EmbedNode |

### Ce qui change côté `create()` / `link()`

```rust
// AVANT — create() construit des ops
pub fn create(&mut self, entity_name: &str, data: BTreeMap<...>) -> Result<EntityRef, ...> {
    let insert_op = CatalogOp::Insert(InsertOp::new(...));
    let mut ops = vec![insert_op];
    // Aussi : InsertOp(KB_Index), LinkOp(IN_KB), AggregateOp
    for kb in entity_kbs { ops.push(CatalogOp::Insert(...)); ops.push(CatalogOp::Link(...)); ... }
    self.pending_ops.extend(ops);
}

// APRÈS — create() pousse des records typés
pub fn create(&mut self, entity_name: &str, data: BTreeMap<...>) -> Result<EntityRef, ...> {
    let (entity_ref, resolver) = EntityRef::new(entity_name);
    let entity = EntityRecord { entity_name, full_data, entity_ref, resolver };
    self.pending.entities.push(entity);
    // KB Index entries + relations + aggregates
    for kb in entity_kbs {
        self.pending.entities.push(EntityRecord { /* KB_Index */ });
        self.pending.relations.push(RelationRecord { /* IN_KB */ });
        self.pending.aggregates.push(AggregateRecord { ... });
    }
}
```

La file d'attente change de :
```rust
// AVANT
pending_ops: Vec<CatalogOp>  // tout mélangé, trié par priorité

// APRÈS
pending: PendingWork {
    entities: Vec<EntityRecord>,
    relations: Vec<RelationRecord>,
    aggregates: Vec<AggregateRecord>,
}
```

### Ce qui change côté `build_ingestion_graph()`

```rust
// AVANT — prend un Vec<CatalogOp>, split par type, route vers BatchNodes
fn build_ingestion_graph(&self) -> (DataflowGraph, ServiceRegistry, usize) {
    let ops = std::mem::take(&mut self.pending_ops);
    // SplitOpsNode + 7 conditional BatchNodes
    graph.add_node(Box::new(SplitOpsNode::new("split")));
    graph.set_initial_input("split", "ops", PortValue::Batch(BatchPayload::new(PortType::Ops, ops)));
    // ... 100 lines de routing conditionnel ...
}

// APRÈS — prend des records typés, crée le graphe directement
fn build_ingestion_graph(&self) -> (DataflowGraph, ServiceRegistry, usize) {
    let work = std::mem::take(&mut self.pending);
    let mut graph = DataflowGraph::new();

    // Entities → InsertNode → ChunkNode → EmbedNode
    if !work.entities.is_empty() {
        graph.add_node(Box::new(InsertNode::new("insert")));
        graph.set_initial_input("insert", "entities", work.entities.into());
        // ChunkNode si la config a du chunking
        if self.has_chunking() {
            graph.add_dynamic_node(Box::new(ChunkNode::new("chunk")));
            graph.connect("insert", "inserted", "chunk", "entities");
        }
    }
    // Relations → LinkNode (trigger après insert)
    if !work.relations.is_empty() {
        graph.add_node(Box::new(LinkNode::new("link")));
        graph.set_initial_input("link", "relations", work.relations.into());
        graph.connect("insert", "done", "link", "trigger");
    }
    // Aggregates → AggregateNode (trigger après link)
    if !work.aggregates.is_empty() {
        graph.add_dynamic_node(Box::new(AggregateNode::new("aggregate")));
        graph.set_initial_input("aggregate", "aggregates", work.aggregates.into());
        graph.connect("link", "done", "aggregate", "trigger");
    }
}
```

Plus de SplitOpsNode. Plus de routing par type d'op. Le graphe est construit directement à partir des données typées.

## 3. Impact sur les PortType / PortValue

### Variants supprimés (8)

```
Ops, Inserts, Links, Chunks, Aggregates, Embeds, SparseEmbeds, DualEmbeds
```

### Variants ajoutés (3)

```rust
pub enum PortType {
    // Existants (search)
    Results, Children, Uuids, Meta, Query, Rules, Map, Any, Empty,
    // Nouveaux (ingestion)
    Entities,     // Vec<EntityRecord>
    Relations,    // Vec<RelationRecord>
    Aggregates,   // Vec<AggregateRecord>
}
```

Les embeddings n'ont pas de PortType dédié — l'EmbedNode prend des `Entities` en input (il sait quels champs embedder via la config KB).

`BatchPayload` reste nécessaire (les records contiennent `EntityRefResolver` qui n'est ni Clone ni Serialize).

## 4. Impact sur l'EmbedNode

Changement majeur : **l'EmbedNode ne reçoit plus d'instructions explicites** (pas de "embedde ce texte avec tel embedder"). Il reçoit des `EntityRecord` et décide lui-même :

1. Regarde l'entity_name dans la config
2. Pour chaque KB qui touche cette entity :
   - Si KB a dense → appelle embedder
   - Si KB a sparse → appelle sparse_embedder
   - Si KB a dual → appelle dual_embedder
3. Les textes à embedder sont extraits des données de l'entity (title + content fields)

Ça élimine les 3 types d'ops embed (`EmbedOp`, `SparseEmbedOp`, `DualEmbedOp`) et les 3 nœuds séparés (`EmbedBatchNode`, `SparseEmbedBatchNode`, `DualEmbedBatchNode`). Un seul `EmbedNode` qui sait tout faire.

**Trade-off** : le nœud est plus gros (gère dense+sparse+dual) mais élimine 3 types d'ops, 3 nœuds, 3 PortType variants, et le routing conditionnel dans build_ingestion_graph().

## 5. Impact sur le ChunkNode (DynamicNode)

Avant : `ChunkBatchNode` reçoit `Vec<ChunkOp>`, appelle `compute_chunk_ops()` qui retourne des `Vec<CatalogOp>` (InsertOp + LinkOp + EmbedOp mélangés), puis les route vers des nœuds émis.

Après : `ChunkNode` reçoit `Vec<EntityRecord>` (les entités insérées), produit :
- `Vec<EntityRecord>` — les chunks (sur port `chunks`)
- `Vec<RelationRecord>` — les HAS_CHUNK (sur port `chunk_links`)

Puis émet dynamiquement un `InsertNode` pour les chunks, un `LinkNode` pour les relations, et un `EmbedNode` pour les embeddings. Les données circulent directement — pas d'intermédiaire CatalogOp.

`compute_chunk_ops()` est remplacé par une fonction `compute_chunks()` qui retourne `(Vec<EntityRecord>, Vec<RelationRecord>)` au lieu de `Vec<CatalogOp>`.

## 6. Impact sur l'AggregateNode (DynamicNode)

Même simplification. L'AggregateNode :
1. Reçoit `Vec<AggregateRecord>`
2. Query le graph DB pour reconstruire le contenu
3. Produit des `EntityRecord` (chunks) + `RelationRecord` (HAS_CHUNK)
4. Émet dynamiquement InsertNode + LinkNode + EmbedNode

Plus besoin de `PRIO_POST_AGG_INSERT` (2.6) ni `PRIO_POST_AGG_LINK` (2.7) — l'ordre est imposé par les edges, pas par des priorités numériques.

## 7. Batching systématique — dette historique

> **Statut : FAIT** (doc 24). InsertBatchNode et LinkBatchNode réécrits avec UNWIND.
> Validé E2E (e2e_batch_observe) : 15 inserts → 3 UNWIND, 10 links → 2 UNWIND.
> Observabilité ajoutée sur les 7 nœuds batch (eprintln! avec group counts).

### Constat

Vérifié via git history : les INSERT et LINK n'ont **jamais** été batchés. Dès le premier commit (`67a87123f`), le code fait 1 Cypher par entité / 1 Cypher par relation. Les embed nodes utilisent UNWIND depuis le début, mais pas les inserts ni les links.

Pour 500 entités + 500 relations + 500 chunks = **1500 round-trips DB** au lieu de ~10.

### Règle : tout ce qui peut être fait en batch DOIT l'être

Chaque nœud doit traiter son `Vec<Record>` en un minimum de queries Cypher, en groupant par clé logique et en utilisant `UNWIND`.

### 7.1 — InsertNode : UNWIND CREATE groupé par (entity_name, column_set)

```rust
// AVANT — 1 query par entity (N round-trips)
for entity in entities {
    let cypher = format!("CREATE (n:{} {{ ... }}) RETURN ID(n)", entity.entity_name);
    conn.execute_with_params(&cypher, &params).await;
}

// APRÈS — 1 query par groupe (entity_name, column_set)
// Grouper : HashMap<(entity_name, Vec<col>), Vec<EntityRecord>>
for ((entity_name, columns), group) in &groups {
    let items_param = CypherValue::List(
        group.iter().map(|e| {
            let mut map = BTreeMap::new();
            for col in columns {
                map.insert(col.clone(), e.data[col].clone());
            }
            CypherValue::Map(map)
        }).collect()
    );

    let set_clauses: String = columns.iter()
        .map(|c| format!("{c}: item.{c}"))
        .collect::<Vec<_>>()
        .join(", ");

    let cypher = format!(
        "UNWIND $items AS item \
         CREATE (n:{entity_name} {{{set_clauses}}}) \
         RETURN ID(n), item._uuid"
    );

    let result = conn.execute_with_params(&cypher, &[QueryParam::new("items", items_param)]).await?;

    // Batch resolve : matcher chaque row retournée à son EntityRecord
    for (row, entity) in result.rows.iter().zip(group.iter()) {
        // Cache node_id
        if let Some(node_id) = InternalNodeId::parse(row[0].as_str().unwrap()) {
            cache.insert(&entity.uuid(), node_id);
        }
        // Resolve ref
        if let Some(resolver) = entity.take_resolver() {
            resolver.resolve(entity.uuid());
        }
    }
}
```

**Gain** : 500 Document + 500 KB_Index = **2 queries** au lieu de 1000.

**Note** : les entities d'un même type ont généralement le même column set (create() produit les mêmes colonnes pour un entity_name donné). Donc en pratique 1 groupe = 1 entity type.

**Compatibilité KB** : le pattern KB (InsertNode → LinkNode → AggregateNode) fonctionne car :
- Les UUIDs sont pré-calculés dans `create()` (hashsafe), pas à l'INSERT — le UNWIND ne change rien
- Tous les resolvers d'un groupe sont résolus après le UNWIND, avant que LinkNode ne démarre
- AggregateNode lit la DB après que tous les inserts et links soient faits (trigger edges)

**Sécurité d'ordre** : ne pas supposer que `result.rows[i]` correspond à `items[i]`. Utiliser `RETURN ID(n), item._uuid` et matcher par UUID pour résoudre les resolvers. Coût négligeable (HashMap lookup).

### 7.2 — LinkNode : UNWIND MATCH+CREATE groupé par (rel_name, has_properties)

```rust
// AVANT — 1 query par relation (N round-trips)
for link in relations {
    let cypher = format!("MATCH (a {{_uuid: $from}}), (b {{_uuid: $to}}) CREATE (a)-[:{}]->(b)", rel_name);
    conn.execute_with_params(&cypher, &params).await;
}

// APRÈS — 1 query par rel_name
for (rel_name, group) in &groups {
    let items_param = CypherValue::List(
        group.iter().map(|r| {
            let mut map = BTreeMap::new();
            map.insert("from_uuid".into(), CypherValue::String(r.from_uuid()));
            map.insert("to_uuid".into(), CypherValue::String(r.to_uuid()));
            // ajouter les properties si elles existent
            for (k, v) in &r.properties {
                map.insert(k.clone(), v.clone());
            }
            CypherValue::Map(map)
        }).collect()
    );

    let cypher = if has_properties {
        format!(
            "UNWIND $items AS item \
             MATCH (a {{_uuid: item.from_uuid}}), (b {{_uuid: item.to_uuid}}) \
             CREATE (a)-[:{rel_name} {{{prop_clauses}}}]->(b)"
        )
    } else {
        format!(
            "UNWIND $items AS item \
             MATCH (a {{_uuid: item.from_uuid}}), (b {{_uuid: item.to_uuid}}) \
             CREATE (a)-[:{rel_name}]->(b)"
        )
    };

    conn.execute_with_params(&cypher, &[QueryParam::new("items", items_param)]).await?;

    // Batch resolve
    for relation in group {
        if let Some(resolver) = relation.take_resolver() {
            resolver.resolve(relation.from_uuid(), relation.to_uuid());
        }
    }
}
```

**Gain** : 500 `Document_IN_KB` + 500 `HAS_CHUNK` = **2 queries** au lieu de 1000.

### 7.3 — EmbedNode : déjà batché (UNWIND SET) — à conserver

L'embed est déjà batché via UNWIND depuis le premier jour. Le nouveau `EmbedNode` unifié conserve ce pattern :

```rust
// 1 appel GPU pour tous les textes
let vectors = embedder.embed(&all_texts).await?;

// 1 UNWIND SET par groupe (entity_name, embedding_col)
for ((entity_name, col), group) in &groups {
    let cypher = format!(
        "UNWIND $items AS item \
         MATCH (n:{entity_name} {{_uuid: item.uuid}}) \
         SET n.{col} = item.emb"
    );
    conn.execute_with_params(&cypher, &[items_param]).await?;
}
```

Dense, sparse, et dual peuvent tourner en **parallèle** (tokio::join!) puisqu'ils sont maintenant dans le même nœud.

### 7.4 — ChunkNode : rayon parallèle — déjà batché

Le chunking est CPU-bound, déjà parallélisé via rayon. Pas de DB ici. Inchangé.

### 7.5 — AggregateNode : batch les queries de lecture

Aujourd'hui, chaque AggregateOp fait ~3-5 queries DB séquentielles (read title entity, read content entities, read current index). On peut batch la phase de lecture :

```rust
// AVANT — N queries séquentielles par aggregate
for agg in aggregates {
    let title = conn.execute("MATCH (n {_uuid: $uuid}) RETURN n.*", ...).await?;
    let contents = conn.execute("MATCH (n)-[:REL]->(c) RETURN c.*", ...).await?;
    // ...
}

// APRÈS — 1 UNWIND query pour toutes les lectures titre
let all_title_uuids: Vec<String> = aggregates.iter().map(|a| a.source_uuid.clone()).collect();
let cypher = "UNWIND $uuids AS uuid MATCH (n {_uuid: uuid}) RETURN n.*";
let all_titles = conn.execute_with_params(&cypher, &[QueryParam::new("uuids", ...)]).await?;

// Puis 1 query pour tous les contenus liés (si même type de relation)
// ...
```

**Gain** : 50 aggregates × 3 queries = **150 → ~3 queries**.

### 7.6 — DELETE dans AggregateNode : UNWIND DETACH DELETE

```rust
// AVANT — 1 DELETE par stale chunk
for chunk_uuid in stale_chunks {
    conn.execute("MATCH (n {_uuid: $uuid}) DETACH DELETE n", ...).await?;
}

// APRÈS — 1 DELETE pour tous les stale chunks
let cypher = "UNWIND $uuids AS uuid MATCH (n {_uuid: uuid}) DETACH DELETE n";
conn.execute_with_params(&cypher, &[QueryParam::new("uuids", all_stale_uuids)]).await?;
```

### 7.7 — UPDATE dans AggregateNode : UNWIND SET

```rust
// AVANT — 1 UPDATE par index entry
for (uuid, new_content, new_hash) in updates {
    conn.execute("MATCH (n {_uuid: $uuid}) SET n._content = $c, n._content_hash = $h", ...).await?;
}

// APRÈS — 1 UNWIND pour tous les updates
let cypher = "UNWIND $items AS item \
    MATCH (n {_uuid: item.uuid}) \
    SET n._content = item.content, n._content_hash = item.hash, n._title = item.title";
conn.execute_with_params(&cypher, &[QueryParam::new("items", items)]).await?;
```

### Résumé du gain de batching

| Nœud | Avant (queries) | Après (queries) | Pour 500 entities |
|---|---|---|---|
| InsertNode | N (1/entity) | ~2 (1/entity_type) | 1000 → 2 |
| LinkNode | N (1/relation) | ~3 (1/rel_name) | 1000 → 3 |
| EmbedNode | ~3 (déjà UNWIND) | ~3 (inchangé) | 3 → 3 |
| AggregateNode reads | N×3 | ~3 (UNWIND reads) | 150 → 3 |
| AggregateNode deletes | N (1/chunk) | ~1 (UNWIND DELETE) | 500 → 1 |
| AggregateNode updates | N (1/index) | ~1 (UNWIND SET) | 50 → 1 |
| **Total** | **~2700** | **~13** | **×200 moins de round-trips** |

## 8. Ce qui est supprimé

| Supprimé | Lignes | Raison |
|---|---|---|
| `CatalogOp` enum | ~50 | Remplacé par records typés |
| `InsertOp`, `LinkOp` | ~90 | → `EntityRecord`, `RelationRecord` |
| `EmbedOp`, `SparseEmbedOp`, `DualEmbedOp` | ~25 | Absorbés par EmbedNode |
| `ChunkOp` | ~10 | Le ChunkNode travaille sur EntityRecords |
| `AggregateOp` | ~10 | → `AggregateRecord` (quasi identique) |
| `OrderedPriority` | ~40 | Plus d'ordonnancement par priorité |
| `OperationConfig` | ~50 | batch_size → config nœud, retries → runtime |
| `OpSummary` | ~60 | Remplacé par DataflowEvent (déjà existant) |
| `SplitOpsNode` | ~80 | Plus de routing par type d'op |
| `EmbedBatchNode` | ~80 | Fusionné dans EmbedNode |
| `SparseEmbedBatchNode` | ~60 | Fusionné dans EmbedNode |
| `DualEmbedBatchNode` | ~100 | Fusionné dans EmbedNode |
| `InsertBatchNode` | ~60 | → InsertNode (même logique, nouveau type d'input) |
| `LinkBatchNode` | ~50 | → LinkNode (même logique, nouveau type d'input) |
| 8 PortType variants | ~16 | → 3 nouveaux variants |
| `compute_chunk_ops()` | ~120 | → `compute_chunks()` retournant des records |
| **Total estimé** | **~900 lignes** | |

## 9. Résolution des références inter-nœuds

### Le mécanisme EntityRef / Resolver est inchangé

Le pattern `EntityRef` / `EntityRefResolver` reste identique. C'est un mécanisme de communication asynchrone entre nœuds, indépendant des ops :

```rust
// À l'enqueue (dans create())
let (entity_ref, resolver) = EntityRef::new("Document");
// entity_ref = Arc<Mutex<Option<String>>> partagé, initialement None

// EntityRecord porte les deux :
EntityRecord { entity_ref: entity_ref.clone(), resolver: Some(resolver), ... }

// RelationRecord pointe vers le même entity_ref :
RelationRecord { from: RefOrUuid::Ref(entity_ref.clone()), ... }
```

### Comment InsertNode débloque LinkNode

```
InsertNode                              LinkNode
─────────                              ────────
1. CREATE (n:Doc {...}) RETURN n._uuid
2. resolver.resolve(uuid)  ──────────→  entity_ref résolu
3. set_output("done", Empty) ─────────→  trigger satisfait → LinkNode ready
                                       4. from.resolve().await → retourne immédiatement
                                       5. MATCH+CREATE relation
```

**L'ordonnancement est garanti par les edges du graphe**, pas par les priorités :

```
InsertNode ──done──→ LinkNode.trigger
InsertNode ──done──→ ChunkNode.trigger
LinkNode   ──done──→ AggregateNode.trigger
```

Le runtime ne lance `LinkNode` que quand `InsertNode` est completed. Donc quand `LinkNode` appelle `from.resolve().await`, le ref est **toujours déjà résolu** — le `await` retourne immédiatement.

### Pourquoi on garde quand même le resolve().await

Le `resolve().await` dans `LinkNode` est techniquement redondant (les edges garantissent l'ordre). On le garde comme **safety net** :

1. **Défense en profondeur** — si un bug dans le runtime exécute un nœud trop tôt, le `await` bloque au lieu de crasher avec un UUID None
2. **Coût zéro** — un `await` sur un ref déjà résolu retourne immédiatement (un seul check atomique)
3. **DynamicNodes** — les nœuds émis par ChunkNode/AggregateNode peuvent avoir des dépendances plus complexes. Le `resolve().await` reste le filet de sécurité universel

### Ce qui change vs ops

| Aspect | Avant (ops) | Après (records) |
|---|---|---|
| Le ref vit dans | `InsertOp.entity_ref` | `EntityRecord.entity_ref` |
| Le resolver vit dans | `InsertOp.resolver` | `EntityRecord.resolver` |
| Le waiter pointe vers | `LinkOp.from: RefOrUuid::Ref(entity_ref)` | `RelationRecord.from: RefOrUuid::Ref(entity_ref)` |
| L'ordre est garanti par | `OrderedPriority(1.0)` < `OrderedPriority(2.0)` | Edge `InsertNode.done → LinkNode.trigger` |
| Le resolve() est appelé par | `LinkBatchNode` | `LinkNode` |

**Conclusion** : le mécanisme de résolution est strictement identique. Seul le véhicule change (Record au lieu d'Op) et la garantie d'ordonnancement passe des priorités numériques aux edges du graphe — ce qui est plus explicite et plus robuste.

## 10. Ce qui reste

| Conservé | Raison |
|---|---|
| `EntityRef` / `EntityRefResolver` | Cross-node reference resolution (inchangé) |
| `RelationRef` / `RelationRefResolver` | Idem |
| `RefOrUuid` | Endpoints de relations (inchangé) |
| `Hashsafe` | UUID déterministe (inchangé) |
| `BatchPayload` | Records contiennent des Resolvers (pas Clone) |
| `ServiceRegistry` | Services partagés (conn, embedder, etc.) |
| `DataflowRuntime` | Exécution du graphe (inchangé) |
| Observabilité (tap, report, record) | Fonctionne sur les events du runtime (inchangé) |

## 11. Migration

> **Pré-requis FAIT** : le batching UNWIND est implémenté sur les nœuds actuels (doc 24).
> Les phases A-D ci-dessous portent le vrai comportement graphe : élimination des ops.

### Phase A — Nouveaux records + PendingWork

1. Créer `EntityRecord`, `RelationRecord`, `AggregateRecord` dans un nouveau fichier `records.rs`
2. Créer `PendingWork { entities, relations, aggregates }`
3. Ajouter 3 PortType variants : `Entities`, `Relations`, `Aggregates`
4. Migrer `create()` et `link()` pour pousser des records au lieu d'ops (garde les deux systèmes en parallèle)

### Phase B — Nouveaux nœuds

1. `InsertNode` — même logique que `InsertBatchNode` mais prend `Vec<EntityRecord>` au lieu de `Vec<InsertOp>`
2. `LinkNode` — même logique, prend `Vec<RelationRecord>`
3. `EmbedNode` — fusionne les 3 embed nodes, prend `Vec<EntityRecord>`, décide dense/sparse/dual via config
4. `ChunkNode` — même logique que `ChunkBatchNode`, prend `Vec<EntityRecord>`, produit records
5. `AggregateNode` — même logique, prend `Vec<AggregateRecord>`, produit records

### Phase C — Nouveau build_ingestion_graph()

1. Réécrire `build_ingestion_graph()` pour utiliser les nouveaux nœuds + `PendingWork`
2. Réécrire `drain()` et `flush_insertions()`
3. Migrer `update()` et `delete()` vers le nouveau système

### Phase D — Suppression

1. Supprimer les ops (`InsertOp`, `LinkOp`, `EmbedOp`, etc.)
2. Supprimer `CatalogOp`, `OrderedPriority`, `OperationConfig`, `OpSummary`
3. Supprimer les anciens nœuds (`InsertBatchNode`, `SplitOpsNode`, etc.)
4. Supprimer les 8 PortType variants ingestion
5. Supprimer `compute_chunk_ops()` → `compute_chunks()`
6. Nettoyer tests

### Validation à chaque phase

- `cargo test --lib` — 385+ unit tests
- 6 suites E2E — 84+ tests
- Aucune régression

## 12. Résumé

**Avant** : `create()` pré-calcule un plan d'exécution sous forme d'ops → le graphe dispatche ces ops vers des nœuds typés.

**Après** : `create()` pousse des données métier → le graphe **est** le plan d'exécution. Les nœuds savent quoi faire par leur position dans le graphe et par la config.

Le graphe passe de "routeur d'instructions" à "pipeline de transformations de données". C'est le shift fondamental : les ops disaient **quoi faire**, le graphe dit **comment les données circulent**.
