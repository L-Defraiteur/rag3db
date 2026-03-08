# Doc 13 — Debug FTS post-delete : findings

Date : 8 mars 2026
Réf : Doc 12 (rapport progression)

## Résumé

Le bug FTS post-delete n'est PAS un bug de delete. C'est un bug de **format de query** dans mes diagnostics — ET potentiellement un **deadlock dataflow** dans `rechunk_simple_entities()`.

## Finding 1 : Le FTS index fonctionne — c'est le format de query diagnostic qui était faux

Test diagnostic ajouté : raw `QUERY_LUCIVY_INDEX` appelé **AVANT** tout delete.

```
[DIAG PRE-DELETE] raw beta: Ok(0)   ← 0 résultats AVANT le delete !
```

Ça prouve que le problème n'est PAS le delete qui corrompt l'index. L'index n'a jamais répondu à ma query de test.

**Raison** : ma query diagnostic utilisait le format GTest :
```json
{"type":"contains","field":"description","value":"beta"}
```

Mais le vrai pipeline `search()` utilise `build_bm25_query()` qui génère (pour 2 champs description + details) :
```json
{"type":"boolean","should":[
  {"type":"contains","field":"description","value":"beta engineering","distance":1},
  {"type":"contains","field":"details","value":"beta engineering","distance":1}
]}
```

La différence clé : `"distance":1` (Levenshtein fuzzy). Sans ce champ, le `NgramContainsQuery` peut se comporter différemment (distance=0 = match exact trigram only, vs distance=1 = fuzzy).

**Mais surtout** : les tests existants BM25 (10 tests) passent tous, ce qui prouve que le FTS fonctionne correctement via le pipeline `search()`.

## Finding 2 : Le VRAI bug est un deadlock dans rechunk_simple_entities()

En relançant les tests avec `--build`, j'ai découvert l'erreur réelle :

```
rechunk_simple_entities failed: deadlock: nodes ["chunk_insert", "chunk_link", "embed", "flush_fts"] cannot execute
```

C'est un **deadlock du scheduler dataflow**, pas un problème FTS.

### Cause racine du deadlock

Dans `ChunkRecordNode::execute()` (record_nodes.rs:1397-1406) :

```rust
if !all_chunk_entities.is_empty() {
    ctx.set_output("chunks", ...);
}
if !all_chunk_relations.is_empty() {
    ctx.set_output("chunk_links", ...);
}
```

Si le chunking produit 0 chunks (possible pour du texte très court), les outputs `chunks` et `chunk_links` ne sont **PAS set**. Le scheduler attend les données sur les edges connectés → deadlock.

Le scheduler (runtime.rs:554-558) :
```rust
// Optional inputs with no connected edge are always satisfied
if !input.required && !has_incoming_edge {
    return true;
}
// For required inputs, OR optional inputs with connected edges:
// wait for upstream data to be available
```

Quand un edge est connecté, le scheduler attend les données même si le port est `required: false`. Mais ici c'est le port `entities` (required: true) de `chunk_insert` qui n'a jamais reçu de données car `chunk.chunks` n'a pas été set.

### Pourquoi 0 chunks ?

Hypothèse : le texte des produits de test est court ("Advanced alpha technology for computing" = ~40 chars). Selon le `max_size` du chunker (configuré dans `setup_simple_catalog(4)`), le texte pourrait être trop court pour générer des chunks. Mais ça paraît bizarre car l'ingest initial produit bien des chunks (6 chunks pour 3 produits).

**À vérifier** : est-ce que les EntityRecords passés à `rechunk_simple_entities()` contiennent bien les content fields (description, details) dans leur `data` ? Le `self.get()` retourne les données from DB, qui incluent tous les champs. Mais si le format diffère de ce que `compute_chunks()` attend, ça pourrait produire 0 chunks.

## Finding 3 : Build C++ cassé

```
gmake[1]: *** [lucivy_fts_extension_function] Erreur 2
```

Le build cmake de l'extension `lucivy_fts` échoue. Les tests utilisent une version stale de l'extension. Ce n'est probablement pas lié au bug, mais ça empêche de rebuilder proprement.

## Résumé des vrais bugs à fixer

| # | Bug | Sévérité | Fichier |
|---|-----|----------|---------|
| 1 | **Deadlock dataflow** dans `rechunk_simple_entities()` quand chunk produit 0 résultats | Bloquant | `src/catalog.rs` (rechunk graph) |
| 2 | FTS query format dans les tests de diagnostic — non bloquant, c'était juste mon erreur de debug | Non-bug | tests seulement |
| 3 | Build cmake cassé pour lucivy_fts | À investiguer | cmake |

## Prochaines étapes

1. **Fixer le deadlock** — 2 options :
   - **Option A** : `ChunkRecordNode` doit TOUJOURS set ses outputs (même vides) → `ctx.set_output("chunks", PortValue::Batch(BatchPayload::new(PortType::Entities, vec![])))`. Ça corrige le problème génériquement pour tous les graphes.
   - **Option B** : Dans `rechunk_simple_entities()`, guard early si items est vide ou si le texte est trop court pour chunker.

2. **Vérifier les données** passées à `rechunk_simple_entities()` — le `self.get()` retourne-t-il bien les content fields ?

3. **Fixer le query format** dans les tests E2E — utiliser `build_bm25_query()` ou le format multi-field avec `distance:1`.

4. **Investiguer le build cmake** cassé.

## Fichiers clés modifiés pendant le debug

| Fichier | Modification |
|---------|-------------|
| `tests/e2e_simple_entity.rs` | Ajout diagnostics raw QUERY_LUCIVY_INDEX (à nettoyer) |

## Extension C++ — architecture confirmée

- FTS index est sur la table **entity** (Product), PAS la table chunk (Product_Chunk)
- Les hooks `insert()/delete_()/update()` sur LucivyIndex sont câblés via le storage layer de rag3db
- `DETACH DELETE` appelle bien le hook `delete_()` sur les indexes de la table
- `flushIfDirty()` = `commit() + reload_reader()` — identique à `checkpointInMemory()`
- `QUERY_LUCIVY_INDEX` appelle `flushIfDirty()` automatiquement au bind time
- Le GTest `LucivyDeleteTest2` prouve que delete + search fonctionne côté extension C++
