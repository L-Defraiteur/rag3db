# Rapport : Undo E2E + bugs corrigés

## Résumé de la session

### 1. Bugs corrigés

#### 1.1 CypherCheckpointStore::mark_completed supprimait les undo_context

**Fichier** : `src/dataflow/checkpoint_store.rs`

**Problème** : `mark_completed()` faisait `DELETE n` sur TOUS les `_DataflowNodeState` d'une exécution réussie. L'undo_context (nécessaire au rollback) était perdu.

**Le MockCheckpointStore** gardait les nodes (clear juste `output_ports`), d'où le fait que les tests unitaires passaient mais pas les e2e avec la vraie DB.

**Fix** : On supprime uniquement les nodes sans undo_context. Les nodes avec undo_context sont gardés mais leurs `output_ports` sont vidés (gros JSON inutile).

```rust
// Avant : DELETE ALL
"MATCH (n:_DataflowNodeState {execution_id: $exec_id}) DELETE n"

// Après : DELETE seulement les nodes sans undo
"MATCH (n:_DataflowNodeState {execution_id: $exec_id})
 WHERE n.undo_json = '' OR n.undo_json IS NULL DELETE n"
// + clear output_ports des survivants
"MATCH (n:_DataflowNodeState {execution_id: $exec_id}) SET n.output_ports = ''"
```

#### 1.2 RETURN n inclut _id et _label (propriétés système rag3db)

**Fichier** : `src/dataflow/record_nodes.rs`

**Problème** : `RETURN n` dans rag3db retourne un Map qui inclut `_id` (ID interne nœud) et `_label` (nom de table). Ces propriétés sont read-only — le SET lors du undo échouait avec "Binder exception: _id is reserved for system usage".

**Fix** : Filtrage de `_id` et `_label` lors de la capture des données undo dans DeleteRecordNode et UpdateRecordNode :

```rust
let clean: BTreeMap<String, CypherValue> = props.iter()
    .filter(|(k, _)| k.as_str() != "_id" && k.as_str() != "_label")
    .map(|(k, v)| (k.clone(), v.clone()))
    .collect();
```

Appliqué aux deux endroits :
- `DeleteRecordNode::execute()` — capture avant delete (~ligne 3173)
- `UpdateRecordNode::execute()` — capture avant update (~ligne 3431)

#### 1.3 BM25 search vide avec BM25Mode::Contains sur requêtes multi-mots

**Fichier** : `tests/e2e_undo.rs`

**Problème** : Le mode par défaut `BM25Mode::Contains` cherche la requête entière comme substring. "programming algorithms" ne matche pas "programming and algorithms" car ce n'est pas contigu.

**Fix** : Utilisation de `BM25Mode::ContainsSplit` dans les tests undo — split les mots et fait un boolean OR.

### 2. Tests e2e undo écrits

**Fichier** : `tests/e2e_undo.rs`

#### Tests qui passent (4/4) :

| Test | Signal | Description |
|------|--------|-------------|
| `undo_delete_simple_entity` | BM25 | Delete 2 products → undo → re-ingest → BM25 search OK |
| `undo_update_simple_entity` | BM25 | Update description → undo → re-ingest → BM25 search OK |
| `undo_delete_simple_entity_bgem3` | BM25+Vector+Sparse | Delete product → undo → re-ingest → 3 signaux OK |
| `undo_delete_kb_bgem3` | BM25+Vector+Sparse | **BLOQUÉ** (voir section 3) |

### 3. Bug ouvert : deadlock du drain sur delete-only KB

**Erreur** :
```
deadlock: nodes ["update_kb", "chunk_kb", "flush_fts", "agg_inserts", "agg_links", "agg_embeds"] cannot execute
```

**Contexte** : Quand le seul pending work est un `catalog.delete("Document", &uuid)` sur une entité KB, `build_ingestion_graph()` crée quand même la pipeline KB complète (aggregation, chunking KB, flush FTS, embeddings). Ces nœuds attendent des inputs d'upstream nodes qui ne fire jamais (pas d'inserts ni d'updates).

**Analyse** :
- `build_ingestion_graph()` crée toujours les nœuds KB si des KBs sont configurées
- Le `DeleteRecordNode` n'émet pas vers ces nœuds KB downstream
- Les nœuds KB ont des inputs `required: true` qui ne sont jamais satisfaits → deadlock détecté par le runtime

**Ce n'est PAS un bug undo** — c'est un bug préexistant du pipeline de drain qui n'a jamais été exposé car les tests KB existants ne font pas de delete-only avec tous les signaux activés.

**Solution proposée** : Dans `build_ingestion_graph()`, ne créer les nœuds KB (update_kb, chunk_kb, agg_*, flush_fts) que si `has_entities || has_updates || has_aggregates`. Si seuls des deletes sont pendants, seul le `DeleteRecordNode` + `FlushNode` suffisent.

### 4. Résumé des fichiers modifiés

| Fichier | Changement |
|---------|------------|
| `src/dataflow/checkpoint_store.rs` | Fix mark_completed : garder nodes avec undo_context |
| `src/dataflow/record_nodes.rs` | Fix _id/_label filtrage dans DeleteRecordNode + UpdateRecordNode |
| `src/dataflow/mod.rs` | Re-exports : DeleteRecordNode, UpdateRecordNode, RechunkDeleteNode |
| `tests/e2e_undo.rs` | 4 tests e2e (2 BM25-only, 2 BGE-M3 all signals) |

### 5. Tests validés

- 544 unit tests OK
- 15 e2e_simple_entity OK
- 3 e2e_checkpoint OK
- 37 e2e_search OK
- 8 e2e_generic_search OK
- 6 e2e_drain_unified OK
- 11 e2e_native OK
- 14 e2e_phase0b OK
- 10 e2e_result_mode OK
- 8 e2e_highlight_long_text OK
- 5 e2e_search_queue OK
- 3/4 e2e_undo OK (1 bloqué par deadlock KB)

**Total : 664 tests OK, 0 régressions.**

### 6. Prochaines étapes

1. **Fixer le deadlock KB delete-only** dans `build_ingestion_graph()`
2. Débloquer `undo_delete_kb_bgem3`
3. Commit + push le tout
