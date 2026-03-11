# Doc 06 — Debug : e2e_drain_unified échoue (6/6 tests)

Date : 11 mars 2026

## Contexte

Phase 3 FAIT (build_ingestion_graph câblé, 537 unit + 120 e2e existants passent).
Ajouté `enqueue_update()` / `enqueue_delete()` dans catalog.rs (sync, push dans pending).
Créé `tests/e2e_drain_unified.rs` avec 6 tests.

## Symptôme

Les 6 tests échouent avec `processed=0, failed=N`. Les `delete_results` et `update_results` sont vides (len=0).

Exemples de sortie :
```
drain delete: processed=0, failed=2
delete_results: []

mixed drain: processed=0, failed=3
  delete_results: 0
  update_results: 0

batch update results:
(empty)
```

Le drain retourne `FlushResult { processed: 0, failed: N }` → il passe dans le branch `Err(e)` de drain().

## Diagnostic probable

Le graph dataflow échoue pendant `runtime.execute()`. L'erreur n'est pas visible dans les tests car drain() attrape l'erreur et retourne `failed: op_count`.

### Hypothèse 1 : Service manquant

Les nœuds DeleteRecordNode et UpdateRecordNode ont besoin de services :
- `conn`, `config`, `kb_metadata` → enregistrés toujours ✓
- `node_id_cache` → enregistré toujours ✓
- `entity_configs` → enregistré seulement si `has_deletes || has_updates || needs_kb` ✓
- `pending_aggregates` → enregistré toujours ✓
- `delete_results` / `update_results` → enregistrés toujours ✓

Ceux-ci devraient tous être présents. Mais à vérifier.

### Hypothèse 2 : Erreur dans le nœud lui-même

DeleteRecordNode/UpdateRecordNode exécutent des requêtes Cypher complexes (UNWIND MATCH DELETE, etc.). Une erreur de syntaxe ou un problème de schéma pourrait faire échouer le nœud.

### Hypothèse 3 : Problème d'ordering / trigger manquant

Le graph avec seulement des deletes (pas d'entities/relations) n'a peut-être pas le bon chaînage de triggers. Si UpdateRecordNode attend un trigger de DeleteRecordNode mais que DeleteRecordNode n'est pas dans le graph, le nœud pourrait ne jamais s'exécuter.

Vérifions : quand `has_deletes=true, has_updates=false, has_entities=false, has_relations=false` :
- DeleteRecordNode("deletes") est ajouté avec initial input → devrait s'exécuter
- Pas de trigger nécessaire (c'est un root node avec initial input)
- ✓ devrait fonctionner

Quand `has_updates=true` sans `has_deletes` :
- UpdateRecordNode("updates") ajouté avec initial input
- Pas de trigger connect (has_deletes=false)
- ✓ devrait fonctionner

### Hypothèse 4 : Problème de types dans les Cypher queries

Les nœuds Phase 2 (record_nodes.rs) ont été compilés et passent les unit tests, mais n'ont jamais été testés avec une vraie DB. Il est probable qu'une requête Cypher échoue à l'exécution (ex: table inexistante, colonne manquante, syntaxe rag3db incompatible).

### Hypothèse 5 : empty graph quand seulement updates/deletes

`build_ingestion_graph()` retourne early si `pending.is_empty()`. Vérifions que `PendingWork::is_empty()` considère bien les champs `updates` et `deletes` — c'était fait en Phase 1 mais à confirmer.

## Actions de debug

1. **Rendre l'erreur visible** : dans les tests, modifier pour capturer l'erreur du drain. Options :
   - Ajouter un `subscribe()` au event bus pour voir les CatalogEvent::Error émis
   - Ou temporairement modifier drain() pour eprintln l'erreur avant de la swallow

2. **Tester un nœud isolément** : créer un test minimal qui exécute DeleteRecordNode seul avec un mock ou une vraie DB, pour voir l'erreur exacte.

3. **Vérifier PendingWork::is_empty()** : confirmer qu'il retourne false quand updates/deletes sont non-vides.

4. **Vérifier les requêtes Cypher** : les queries dans DeleteRecordNode/UpdateRecordNode utilisent des patterns comme `UNWIND $items AS item MATCH (n:{entity} {_uuid: item.uuid}) DETACH DELETE n`. Vérifier la syntaxe rag3db (qui diffère de neo4j sur certains points).

## Fichiers pertinents

- `tests/e2e_drain_unified.rs` — les 6 tests qui échouent
- `src/catalog.rs:942-975` — enqueue_update() / enqueue_delete()
- `src/catalog.rs:2015-2200` — build_ingestion_graph() (Phase 3)
- `src/catalog.rs:2202-2256` — drain() avec extraction résultats
- `src/dataflow/record_nodes.rs:2878-3158` — DeleteRecordNode
- `src/dataflow/record_nodes.rs:3173-3500+` — UpdateRecordNode
- `src/records.rs:374-399` — PendingWork (is_empty, total_count)

## État du code

- `enqueue_update()` / `enqueue_delete()` : compilent, ajoutés juste avant les méthodes legacy
- `e2e_drain_unified.rs` : utilise `ingest_entities()` pour le setup (crée entities + chunks), puis `enqueue_*` + `drain()` pour tester le pipeline
- Les 120 e2e existants passent toujours (l'ancien chemin inline n'est pas touché)
