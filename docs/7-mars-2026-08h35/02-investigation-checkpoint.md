# Doc 02 — Investigation : Checkpoint & Crash Recovery

Date : 7 mars 2026

## Problème

Le pipeline d'ingestion (drain) exécute un graphe de ~10 nœuds séquentiels. Si crash mid-execution (GPU timeout, DB down, OOM) :
- Les nœuds déjà exécutés ont écrit en DB (entités, relations, embeddings)
- Les EntityRefResolver (oneshot channels) sont perdus
- Pas de moyen de reprendre — il faut re-ingérer tout depuis le début

## État actuel de l'idempotence

Tous les nœuds record-based sont **déjà idempotents** :

| Noeud | Mécanisme | Fichier:ligne |
|---|---|---|
| InsertRecordNode | `MERGE (n:{entity} {_uuid: item._uuid})` | record_nodes.rs:121 |
| LinkRecordNode | `MERGE (a)-[r:{rel}]->(b)` sur from/to UUIDs | record_nodes.rs:300 |
| EmbedRecordNode | Compare `_embed_hash` vs `_text_hash`, skip si égaux | record_nodes.rs:526-553 |
| ChunkRecordNode | UUIDs déterministes via `chunk_uuid()` | (hérité d'InsertRecordNode) |
| GatherKBNode | Compare `_content_hash`, skip si inchangé | record_nodes.rs |

Donc : re-jouer un nœud déjà exécuté est **safe** — MERGE ne duplique pas, les hash-checks évitent le travail GPU inutile.

## Sérialisabilité des port data

### PortValue — partiellement sérialisable

```rust
#[derive(Debug, Clone, Serialize)]       // ← Serialize OK
pub enum PortValue {
    Results(Vec<UnifiedResult>),          // ✓ sérialisable
    Children(HashMap<String, Vec<...>>),  // ✓
    Meta(SearchMeta),                     // ✓
    Query { ... #[serde(skip)] options }, // ✓ (options skippé)
    Empty,                                // ✓
    Batch(BatchPayload),                  // ⚠ sérialise seulement {batch_type, count}
}
```

### BatchPayload — données NON sérialisables

```rust
pub struct BatchPayload {
    batch_type: PortType,
    count: usize,
    data: Arc<Mutex<Option<Box<dyn Any + Send>>>>,  // ← opaque, pas Serialize
}
```

Les records (EntityRecord, RelationRecord, AggregateRecord, KBContentRecord) n'implémentent pas Serialize. Ils contiennent des `EntityRef`/`RelationRef` (oneshot channels) qui ne sont pas sérialisables par nature.

### Conséquence

On **ne peut pas** persister les données intermédiaires entre nœuds telles quelles. Deux options :

## Option A — Checkpoint complet (port_data persisté)

1. Ajouter `#[derive(Serialize, Deserialize)]` aux records (sans les Ref/Resolver)
2. Créer un `CheckpointPortValue` qui sérialise les batch data
3. Après chaque nœud : persister `completed_nodes` + `port_data` sérialisé
4. Au restart : charger, recréer le graphe, injecter les port_data, skip les nœuds complétés

**Avantage** : Reprise exacte, pas de re-calcul
**Inconvénient** : Complexe (sérialisation des records, reconstruction des Refs), beaucoup de code

## Option B — Replay idempotent (recommandé)

1. Après chaque nœud : persister seulement `completed_nodes` (HashSet<String>)
2. Au restart : reconstruire le graphe depuis PendingWork, re-exécuter tout
3. Chaque nœud skip ce qui est déjà fait grâce à l'idempotence (MERGE, hash-check)

**Avantage** : Simple, ~100 lignes de code, pas de sérialisation de records
**Inconvénient** : Re-exécute les requêtes DB (MERGE = fast si déjà existant), re-lit les hashes

En pratique le surcoût est minimal : les MERGE sont rapides (O(1) sur `_uuid` indexé), et le hash-check dans EmbedRecordNode évite les appels GPU.

## Design Option B

### CheckpointStore trait

```rust
#[async_trait]
pub trait CheckpointStore: Send + Sync {
    async fn save(&self, execution_id: &str, completed: &HashSet<String>) -> Result<(), String>;
    async fn load(&self, execution_id: &str) -> Result<Option<HashSet<String>>, String>;
    async fn delete(&self, execution_id: &str) -> Result<(), String>;
}
```

Implémentation via la DB existante (table `_DataflowCheckpoint`) ou fichier JSON local.

### Intégration dans le runtime

```rust
// Dans DataflowRuntime
pub async fn execute_with_checkpoint(
    &self,
    graph: &mut DataflowGraph,
    checkpoint_store: &dyn CheckpointStore,
    execution_id: &str,
) -> Result<DataflowOutput, String> {
    // 1. Charger checkpoint existant
    let previously_completed = checkpoint_store.load(execution_id).await?
        .unwrap_or_default();

    // ... boucle d'exécution normale ...

    // 2. Avant chaque nœud : skip si dans previously_completed
    if previously_completed.contains(&node_name) {
        // Skip — mais on n'a pas les port_data
        // → les nœuds downstream recevront Empty/pas d'input
        // → ils re-exécuteront en mode idempotent
        completed.insert(node_name);
        continue;
    }

    // 3. Après chaque nœud complété : sauvegarder
    completed.insert(node_name.clone());
    checkpoint_store.save(execution_id, &completed).await?;

    // 4. Fin OK : supprimer le checkpoint
    checkpoint_store.delete(execution_id).await?;
}
```

### Problème : port_data manquant au restart

Si on skip les nœuds complétés, les nœuds downstream n'ont pas leurs inputs (les port_data sont perdus). Deux sous-options :

**B1 — Skip intelligent** : Ne skip que les nœuds "terminaux" (qui n'ont pas de downstream). Les nœuds intermédiaires sont re-exécutés — l'idempotence garantit la sécurité. En pratique on ne skip rien et on re-exécute tout, le checkpoint sert juste à savoir qu'on doit reprendre.

**B2 — Re-exécution complète** : Ignorer le checkpoint au niveau du runtime. Le `drain()` dans catalog.rs reconstruit `PendingWork` et re-exécute tout. Les nœuds sont idempotents donc le résultat est correct. Le "checkpoint" est implicite dans la DB (ce qui est MERGE'd existe déjà).

**B2 est le plus simple** : le seul changement nécessaire est que `catalog.drain()` puisse être appelé de nouveau sur les mêmes données sans effet de bord. C'est déjà le cas si `PendingWork` est reconstruit depuis la source (le caller qui appelle create/link).

### Mais on perd PendingWork au crash...

C'est le vrai problème. `PendingWork` vit en mémoire dans `Catalog`. Si le process crash, les `EntityRecord`/`RelationRecord` sont perdus. Le caller (Node.js, WASM) doit re-appeler `create()`/`link()` pour reconstruire le PendingWork.

Solutions :
1. **Côté caller** : Le caller persiste son intention (fichiers à ingérer) et re-appelle create/link au restart. C'est déjà le pattern typique (le caller a la source de vérité).
2. **Côté rag3weaver** : Persister PendingWork avant drain(). Plus complexe (sérialisation des records).
3. **Hybride** : Persister juste les paramètres de create/link (entity_name, data BTreeMap) — pas les Refs/Resolvers — et reconstruire PendingWork au restart.

## Recommandation

**Option B2 + solution côté caller** :
- Pas de changement dans le runtime
- Le caller (orchestrateur Node.js) re-appelle create()/link()/drain() si le drain précédent a échoué
- L'idempotence garantit la correction
- Documenter le pattern de retry dans l'API

**Si on veut aller plus loin (Option B avec checkpoint store)** :
- `CheckpointStore` trait + implémentation DB
- Persister les arguments de create/link (pas les records compilés)
- `Catalog::drain_with_retry()` qui reconstruit PendingWork depuis le checkpoint

## Fichiers concernés

| Fichier | Changement |
|---|---|
| `src/dataflow/checkpoint.rs` (NEW) | CheckpointStore trait + CypherCheckpointStore |
| `src/dataflow/runtime.rs` | execute_with_checkpoint() |
| `src/dataflow/mod.rs` | Export checkpoint module |
| `src/catalog.rs` | drain() with retry logic |
| `src/records.rs` | Optionnel : Serialize pour les records (si Option A) |
