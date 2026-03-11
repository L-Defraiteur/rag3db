# Doc 03 — Progression Phase 1 + Spécifications Phase 2

Date : 11 mars 2026
Réf : Doc 02 (plan queue/drain unifié)

## Phase 1 : FAIT

### Changements

**`records.rs`** :
- `UpdateRecord { entity_name, uuid, data, new_content_hash }` — Serialize/Deserialize
- `DeleteRecord { entity_name, uuid }` — Serialize/Deserialize
- `PendingWork` étendu avec `updates: Vec<UpdateRecord>`, `deletes: Vec<DeleteRecord>`
- `is_empty()` et `total_count()` mis à jour pour les 5 vecs
- 3 nouveaux tests unitaires (serialization roundtrip, PendingWork mixte)

**`port.rs`** :
- `PortType::Updates` et `PortType::Deletes`
- 2 nouveaux tests (compatibilité, BatchPayload)

**`lib.rs`** :
- Export `UpdateRecord`, `DeleteRecord`

**Résultats** : 544 tests (vs 539 avant), zéro régression.

---

## Phase 2 : Spécifications des nouveaux nœuds

### Patterns extraits des nœuds existants

Chaque nœud suit le pattern `DataflowNode` trait :
```rust
fn name(&self) -> &str;
fn inputs(&self) -> &[PortDef];
fn outputs(&self) -> &[PortDef];
async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String>;
fn can_undo(&self) -> bool;
fn undo_context(&self) -> Option<serde_json::Value>;
async fn undo(&mut self, ctx: &mut NodeContext, undo_ctx: Value) -> Result<(), String>;
```

**Input pattern** :
```rust
let items: Vec<T> = match ctx.take_input("port") {
    Some(PortValue::Batch(payload)) => payload.take::<T>()
        .ok_or("failed to extract")?,
    _ => return Err("missing input".into()),
};
```

**Service pattern** :
```rust
let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
    .ok_or("'conn' service not registered")?;
```

**UNWIND batch pattern** (utilisé par InsertRecordNode, LinkRecordNode, KBUpdateNode, etc.) :
```rust
// Group by entity_name (ou key composite)
let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
for (i, rec) in items.iter().enumerate() {
    groups.entry(rec.entity_name.clone()).or_default().push(i);
}

// Pour chaque group: build CypherValue::List of Maps, UNWIND + MATCH + mutation
let items_param = CypherValue::List(
    indices.iter().map(|&i| {
        let mut m = BTreeMap::new();
        m.insert("uuid".into(), CypherValue::String(items[i].uuid.clone()));
        // ... autres champs
        CypherValue::Map(m)
    }).collect()
);
let cypher = format!("UNWIND $items AS item MATCH ... SET/DELETE ...");
conn.execute_with_params(&cypher, &[QueryParam::new("items", items_param)]).await?;
```

**Output pattern** :
```rust
ctx.set_output("done", PortValue::Empty);
ctx.set_output("entities", PortValue::Batch(
    BatchPayload::new(PortType::Entities, records),
));
```

---

### Nœud 1 : DeleteRecordNode

#### Signature

```rust
pub struct DeleteRecordNode {
    name: String,
    undo_data: Option<serde_json::Value>,
}
```

| Port | Direction | Type | Required |
|------|-----------|------|----------|
| `deletes` | input | `PortType::Deletes` | true |
| `trigger` | input | `PortType::Empty` | false |
| `done` | output | `PortType::Empty` | false |

Services requis : `conn`, `node_id_cache`, `entity_configs`, `config`, `kb_metadata`

Shared services en écriture :
- `pending_aggregates: Arc<Mutex<Vec<AggregateRecord>>>` — pour les AggregateRecords générés (KB contentFor)
- `delete_results: Arc<Mutex<Vec<DeleteResult>>>` — pour extraction post-drain

#### Exécution (extraction de `batch_delete()`)

```
1. Take input deletes: Vec<DeleteRecord>
2. Group par entity_name
3. Pour chaque entity_name:
   a. Résoudre entity_kbs via config + kb_metadata
   b. KB titleFor:
      - Compute index_uuids = hash(kb_name + "_Index", [entity_name, uuid])
      - UNWIND DELETE chunks:
        UNWIND $idx_uuids AS idx_uuid
        MATCH (c:{chunk_table} {_parent_uuid: idx_uuid})
        DETACH DELETE c RETURN idx_uuid, count(c) AS cnt
      - UNWIND DELETE index entries:
        UNWIND $idx_uuids AS idx_uuid
        MATCH (idx:{index_table} {_uuid: idx_uuid})
        DETACH DELETE idx
   c. KB contentFor:
      - UNWIND DELETE SOURCED chunks:
        UNWIND $uuids AS uuid
        MATCH (e:{entity} {_uuid: uuid})-[:{sourced_rel}]->(c:{chunk_table})
        DETACH DELETE c RETURN uuid, count(c) AS cnt
      - Find title entities à re-agréger:
        UNWIND $uuids AS uuid
        MATCH (t:{title_entity})-[:{rel}]->(e:{entity} {_uuid: uuid})
        RETURN t._uuid
      - Dedup + push AggregateRecords → pending_aggregates service
   d. Simple entity (entity_configs.contains && entity_kbs.is_empty):
      - UNWIND DELETE chunks:
        UNWIND $uuids AS uuid
        MATCH (c:{chunk_table} {_parent_uuid: uuid})
        DETACH DELETE c RETURN uuid, count(c) AS cnt
   e. UNWIND DELETE entities:
      UNWIND $uuids AS uuid
      MATCH (n:{entity_name} {_uuid: uuid})
      DETACH DELETE n
   f. Batch remove from node_id_cache
4. Push DeleteResults → delete_results service
5. Set output done=Empty
```

#### Undo

Pas d'undo dans un premier temps (`can_undo() = false`). L'entity est supprimée — la recréer nécessiterait de stocker toutes les données + chunks, trop complexe pour le MVP.

---

### Nœud 2 : UpdateRecordNode

#### Signature

```rust
pub struct UpdateRecordNode {
    name: String,
    undo_data: Option<serde_json::Value>,
}
```

| Port | Direction | Type | Required |
|------|-----------|------|----------|
| `updates` | input | `PortType::Updates` | true |
| `trigger` | input | `PortType::Empty` | false |
| `done` | output | `PortType::Empty` | false |
| `rechunk_entities` | output | `PortType::Entities` | false |

Services requis : `conn`, `entity_configs`, `config`, `kb_metadata`

Shared services en écriture :
- `pending_aggregates: Arc<Mutex<Vec<AggregateRecord>>>` — pour KB re-aggregation
- `update_results: Arc<Mutex<Vec<UpdateResult>>>` — pour extraction post-drain

#### Exécution (extraction de `batch_update()`)

```
1. Take input updates: Vec<UpdateRecord>
2. Group par entity_name
3. Pour chaque entity_name:
   a. Batch-read old hashes:
      UNWIND $uuids AS uuid
      MATCH (n:{entity_name} {_uuid: uuid})
      RETURN n._uuid, n._content_hash
   b. Detect content changes:
      changed[i] = (old_hash[i] != new_content_hash[i])
   c. Batch SET all fields + _content_hash:
      UNWIND $items AS item
      MATCH (n:{entity_name} {_uuid: item._uuid})
      SET n.field1 = item.field1, ..., n._content_hash = item._content_hash
   d. Pour les changed items:
      - KB titleFor: compute AggregateRecords → pending_aggregates service
      - KB contentFor: find linked title entities (UNWIND MATCH) → AggregateRecords
      - Simple entity: read full data via MATCH RETURN n, unwrap {"n": Map({...})},
        build EntityRecord avec EntityRef::pre_resolved() → rechunk_entities output
   e. Build UpdateResults (status, reembedded flag)
4. Push UpdateResults → update_results service
5. Set output rechunk_entities = BatchPayload<EntityRecord> (simple entities changed)
6. Set output done = Empty
```

#### Optimisation UNWIND SET

Actuellement `batch_update()` construit le SET clause à partir du premier item, en assumant que tous les items ont les mêmes champs. Même approche ici mais en groupant par `(entity_name, sorted_field_keys)` :

```rust
let mut groups: HashMap<(String, Vec<String>), Vec<usize>> = HashMap::new();
for (i, rec) in items.iter().enumerate() {
    let mut keys: Vec<String> = rec.data.keys().cloned().collect();
    keys.sort();
    groups.entry((rec.entity_name.clone(), keys)).or_default().push(i);
}
```

Cela permet de supporter des updates avec des colonnes différentes dans le même drain (ex: un update qui change `description`, un autre qui change `price`).

#### Undo

Stocker les anciennes valeurs avant le SET :
```rust
// Avant le SET, pour chaque group:
UNWIND $items AS item
MATCH (n:{entity_name} {_uuid: item._uuid})
RETURN n  // capture toutes les valeurs actuelles

// Undo: restaurer les anciennes valeurs
UNWIND $items AS item
MATCH (n:{entity_name} {_uuid: item._uuid})
SET n.field1 = item.old_field1, ...
```

#### Node wrapping

`RETURN n` retourne `{"n": Map({_uuid, description, ...})}`. L'unwrap est nécessaire :
```rust
let props = match row_map.get("n") {
    Some(CypherValue::Map(m)) => m.clone(),
    _ => row_map,
};
```

Ce pattern est déjà utilisé dans `update()` et `batch_update()` actuels (fix Bug 3, Doc 15).

---

### Nœud 3 : RechunkDeleteNode

#### Signature

```rust
pub struct RechunkDeleteNode {
    name: String,
}
```

| Port | Direction | Type | Required |
|------|-----------|------|----------|
| `entities` | input | `PortType::Entities` | true |
| `entities` | output | `PortType::Entities` | false |

Services requis : `conn`

#### Exécution (première étape de `rechunk_simple_entities()`)

```
1. Take input entities: Vec<EntityRecord>
2. Group par entity_name
3. Pour chaque entity_name:
   - Collecter les UUIDs (_parent_uuid)
   - UNWIND DELETE old chunks:
     UNWIND $uuids AS uuid
     MATCH (c:{entity_name}_Chunk {_parent_uuid: uuid})
     DETACH DELETE c RETURN uuid, count(c) AS cnt
4. Pass-through: set output entities = mêmes EntityRecords
```

Pas d'undo (les chunks sont recréés par ChunkRecordNode en aval).

---

### Shared Services (nouveaux)

Phase 3 ajoutera ces services dans `build_ingestion_graph()` :

```rust
// Agrégats collectés par DeleteRecordNode + UpdateRecordNode
let pending_aggregates: Arc<Mutex<Vec<AggregateRecord>>> = Arc::new(Mutex::new(
    std::mem::take(&mut pending.aggregates)  // seed avec les aggregates existants
));
services.register::<Mutex<Vec<AggregateRecord>>>("pending_aggregates", pending_aggregates);

// Résultats pour extraction post-drain
let update_results: Arc<Mutex<Vec<UpdateResult>>> = Arc::new(Mutex::new(Vec::new()));
services.register::<Mutex<Vec<UpdateResult>>>("update_results", update_results.clone());

let delete_results: Arc<Mutex<Vec<DeleteResult>>> = Arc::new(Mutex::new(Vec::new()));
services.register::<Mutex<Vec<DeleteResult>>>("delete_results", delete_results.clone());
```

KBGatherNode sera modifié pour lire depuis `pending_aggregates` service au lieu de (ou en plus de) son input port.

---

### Factories (node_factories.rs)

```rust
named_factory!(
    DeleteRecordNodeFactory,
    DeleteRecordNode,
    "DeleteRecordNode",
    "Batch cascade-delete entities + chunks from Vec<DeleteRecord>",
    &[
        PortDef { name: "deletes", port_type: PortType::Deletes, required: true },
        PortDef { name: "trigger", port_type: PortType::Empty, required: false },
    ],
    &[
        PortDef { name: "done", port_type: PortType::Empty, required: false },
    ],
);

named_factory!(
    UpdateRecordNodeFactory,
    UpdateRecordNode,
    "UpdateRecordNode",
    "Batch field update + change detection from Vec<UpdateRecord>",
    &[
        PortDef { name: "updates", port_type: PortType::Updates, required: true },
        PortDef { name: "trigger", port_type: PortType::Empty, required: false },
    ],
    &[
        PortDef { name: "done", port_type: PortType::Empty, required: false },
        PortDef { name: "rechunk_entities", port_type: PortType::Entities, required: false },
    ],
);

named_factory!(
    RechunkDeleteNodeFactory,
    RechunkDeleteNode,
    "RechunkDeleteNode",
    "Delete old chunks before re-chunking",
    &[
        PortDef { name: "entities", port_type: PortType::Entities, required: true },
    ],
    &[
        PortDef { name: "entities", port_type: PortType::Entities, required: false },
    ],
);
```

Et dans `register_builtins()` :
```rust
registry.register(Box::new(DeleteRecordNodeFactory));
registry.register(Box::new(UpdateRecordNodeFactory));
registry.register(Box::new(RechunkDeleteNodeFactory));
```

---

### Fonctions helpers à extraire de `catalog.rs`

Pour éviter de dupliquer du code entre les nœuds et catalog.rs, extraire ces helpers :

1. **`resolve_entity_kbs()`** — déjà dans `schema.rs:81`, accessible via `config` + `kb_metadata` services
2. **`find_relation_to_entity()`** — dans `catalog.rs:2936`, à extraire vers `schema.rs` (prend config en paramètre)
3. **`hashsafe_uuid()`** / `chunk_uuid()`— déjà dans `uuid.rs`, publics
4. **`row_to_map()`** — dans `catalog.rs:2997`, trivial (zip columns + values)
5. **`build_content_text()`** — dans `catalog.rs:2948`, dépend de `entity_configs`. Soit passer en paramètre, soit extraire.

L'idée est que les nœuds n'importent PAS `Catalog` mais accèdent aux mêmes helpers via des fonctions libres + services.

---

## Plan d'action Phase 2

1. Extraire `find_relation_to_entity()` vers `schema.rs` (fonction libre)
2. Implémenter `RechunkDeleteNode` (le plus simple des 3)
3. Implémenter `DeleteRecordNode` (logique de `batch_delete()`)
4. Implémenter `UpdateRecordNode` (logique de `batch_update()`)
5. Ajouter les 3 factories dans `node_factories.rs`
6. Tests unitaires avec mock DbConnection
7. Compilation + run 544+ unit tests
