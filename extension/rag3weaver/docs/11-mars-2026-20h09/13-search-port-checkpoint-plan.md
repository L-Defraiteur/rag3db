# Doc 13 — Search port checkpoint : plan d'implémentation

Date : 12 mars 2026

## Contexte

Le checkpoint sauvegarde les outputs de chaque nœud pour crash recovery. Actuellement, seuls les **PortTypes d'ingestion** sont checkpointables. Les **ports search** échouent avec une erreur hard qui stoppe l'exécution.

## État actuel

| PortType | Serialize | Deserialize | Checkpoint | Utilisé par |
|----------|-----------|-------------|------------|-------------|
| Empty | ✓ | ✓ | ✓ | Tous (signaux done/trigger) |
| Entities (Batch) | ✓ | ✓ | ✓ | Insert, Chunk, Rechunk |
| Relations (Batch) | ✓ | ✓ | ✓ | Link, KBChunk |
| Aggregates (Batch) | ✓ | ✓ | ✓ | KBGather |
| KBContent (Batch) | ✓ | ✓ | ✓ | KBUpdate, KBChunk |
| Updates (Batch) | ✓ | ✓ | ✓ | UpdateRecordNode |
| Deletes (Batch) | ✓ | ✓ | ✓ | DeleteRecordNode |
| **Results** | ✓ | ✗ | ✗ | Search, Compose, Expand |
| **Children** | ✓ | ✗ | ✗ | Expand |
| **Uuids** | ✓ | ✗ | ✗ | Expand |
| **Meta** | ✓ | ✗ | ✗ | Search |
| **Query** | ⚠️ partial | ✗ | ✗ | Search entry point |
| **Rules** | ✓ | ✗ | ✗ | Expand |
| **Map** | ✓ | ✓ | ✗ (non testé) | Générique |
| **Any** | ✓ | ✓ | ✗ (non testé) | Générique |

## Problème

`checkpoint.rs:516` fait `port_value_to_checkpoint(value)?` — erreur hard. Si un nœud search produit un `PortValue::Results`, l'exécution échoue.

En pratique, les pipelines search n'utilisent PAS le checkpoint (pas de `execute_with_checkpoint()`). Mais si on voulait les rendre checkpointables (pour des pipelines mixtes ingestion+search), il faut :

## Ce qu'il faut faire

### Étape 1 — Ajouter `Deserialize` aux types search

Types concernés (tous ont déjà `Serialize`, il manque `Deserialize`) :

| Type | Fichier | Ligne |
|------|---------|-------|
| `UnifiedResult` | `search_strategy.rs` | ~21 |
| `ChildSummary` | `search_strategy.rs` | ~81 |
| `ExpansionRule` | `search_strategy.rs` | ~? |
| `ExpansionDirection` | `search_strategy.rs` | ~? |
| `SearchMeta` | `search.rs` | ~482 |
| `ChunkInfo` | `search.rs` | ~? |
| `AttributedChunk` | `search.rs` | ~? |
| `SearchDiagnostics` | `search.rs` | ~? |
| `ExploreGraph` | `search.rs` | ~? |

**Action** : ajouter `Deserialize` au derive de chaque struct. Vérifier que tous les champs sont eux-mêmes Deserialize (notamment `CypherValue` dans les `BTreeMap<String, CypherValue>`).

### Étape 2 — Sérialiser `SearchTarget`

`SearchTarget` n'a **aucun** derive Serialize/Deserialize. Il contient :
```rust
pub struct SearchTarget {
    pub name: String,
    pub parent_table: String,
    pub chunk_table: String,
    pub relation_field: String,
    pub is_kb: bool,
    pub kb_config: Option<KBConfig>,
    pub entity_config: Option<EntityConfig>,
}
```

**Action** : ajouter `#[derive(Serialize, Deserialize)]`. Vérifier `KBConfig` et `EntityConfig`.

**Impact sur `Query` PortValue** : actuellement `options` et `target` sont `#[serde(skip)]`. Après cette étape, retirer les `skip` pour permettre la sérialisation complète.

### Étape 3 — Implémenter `deserialize_non_batch_port_value()`

Actuellement (checkpoint.rs:211-216), cette fonction retourne toujours une erreur :
```rust
fn deserialize_non_batch_port_value(port_type: PortType, _json: &str) -> Result<PortValue, String> {
    Err("checkpoint deserialization not yet supported...")
}
```

**Action** : implémenter le dispatch par PortType :
```rust
fn deserialize_non_batch_port_value(port_type: PortType, json: &str) -> Result<PortValue, String> {
    match port_type {
        PortType::Results => Ok(PortValue::Results(serde_json::from_str(json)?)),
        PortType::Children => Ok(PortValue::Children(serde_json::from_str(json)?)),
        PortType::Uuids => Ok(PortValue::Uuids(serde_json::from_str(json)?)),
        PortType::Meta => Ok(PortValue::Meta(serde_json::from_str(json)?)),
        PortType::Query => { /* reconstruct from JSON */ },
        PortType::Rules => Ok(PortValue::Rules(serde_json::from_str(json)?)),
        PortType::Map => Ok(PortValue::Map(serde_json::from_str(json)?)),
        PortType::Any => Ok(PortValue::Any(serde_json::from_str(json)?)),
        _ => Err(format!("unsupported: {port_type:?}")),
    }
}
```

## Cascade de dépendances

```
Étape 1 (Deserialize sur types search)
  → nécessite: CypherValue Deserialize (vérifier)
  → nécessite: tous les sous-types Deserialize

Étape 2 (SearchTarget serializable)
  → nécessite: KBConfig, EntityConfig Serialize+Deserialize
  → permet: retirer #[serde(skip)] sur Query.options/target

Étape 3 (deserialize_non_batch_port_value)
  → nécessite: étapes 1 + 2
  → permet: checkpoint complet pour pipelines search
```

## Fichiers concernés

| Fichier | Modification |
|---------|-------------|
| `src/search_strategy.rs` | Ajouter Deserialize aux structs |
| `src/search.rs` | Ajouter Deserialize aux structs + SearchTarget |
| `src/dataflow/checkpoint.rs` | Implémenter `deserialize_non_batch_port_value()` |
| `src/dataflow/port.rs` | Retirer `#[serde(skip)]` sur Query (après étape 2) |
| `src/config.rs` (?) | Vérifier EntityConfig, KBConfig serde |
| `src/connection.rs` (?) | Vérifier CypherValue Deserialize |

## Risques

- **CypherValue** : si ce type n'a pas Deserialize, c'est un bloqueur (il est utilisé partout dans les types search). Vérifier en priorité.
- **SearchTarget reconstruction** : `KBConfig` et `EntityConfig` peuvent contenir des types non-sérialisables (closures, Arc, etc.). À vérifier.
- **Volume de données** : un `PortValue::Results` peut contenir des centaines de résultats avec data enrichie. Le JSON checkpointé peut devenir volumineux.

## Priorité

**Basse.** Les pipelines search ne passent pas par `execute_with_checkpoint()` actuellement. Ce chantier n'est nécessaire que si on veut des pipelines mixtes (ingestion + search dans le même graph checkpointé) ou la crash recovery sur des search pipelines.
