# Doc 32 — Session : Fix EmbedRecordNode + E2E debug en cours

Date : 7 mars 2026

## Contexte

Suite de la session doc 31. On a voulu valider les tests E2E (`./run_e2e.sh`) et découvert que `batch_observe_multi_entity` échouait : les chunks KB n'avaient pas d'embedding (0 au lieu de 5).

## Diagnostic

### Le problème

`EmbedRecordNode("agg_embeds")` reçoit des chunks KB depuis `InsertRecordNode("agg_inserts").inserted`, mais son code cherchait les entités dans `kb_metadata` via :

```rust
for (kb_name, kb_meta) in kb_metadata.iter() {
    if !kb_meta.entities.contains(&rec.entity_name) {
        continue;  // ← les chunks "FileKB_Index_Chunk" ne sont jamais dans kb_meta.entities
    }
    // ... collecte title_for / content_for — code pensé pour des entités normales
}
```

`kb_meta.entities` contient `["File", "Document"]`, jamais `"FileKB_Index_Chunk"`. Résultat : 0 embedding produit.

### Pourquoi c'est faux conceptuellement

`EmbedRecordNode` était écrit comme s'il embeddait des entités normales en cherchant leurs champs content_for/title_for. Or :

1. **Seuls les chunks sont embeddés**, jamais les entités directement (même un texte court produit un chunk)
2. Les chunks portent déjà tout dans leur `data` : `_text`, `_kb_name`, `_text_hash`
3. La recherche vectorielle cherche sur `{KB}_Index_Chunk.{kb}_embedding` — c'est la colonne chunk qui est indexée
4. Ligne 882 de catalog.rs le confirme : *"No EmbedRecordNode on raw entities (only KB_Index_Chunk are searched)"*

### Architecture de recherche (pour référence)

| Recherche | Table cible | Colonne/Index | Résolution |
|---|---|---|---|
| Vector | `{KB}_Index_Chunk` | `{kb}_embedding` (HNSW) | chunk → parent via HAS_CHUNK |
| BM25 | `{KB}_Index` | `_title`, `_content` (Lucivy FTS) | offset → parent + chunks |
| Sparse | `{KB}_Index_Chunk` | `{kb}_sparse_indices/weights` | chunk → parent via HAS_CHUNK |

### Pipeline d'ingestion KB (correct)

```
AggregateRecord[] → GatherKBNode("gather_kb")    — lit les champs source depuis la DB
    └── kb_content → UpdateKBNode("update_kb")    — MERGE + SET sur {KB}_Index (hooks Lucivy)
        ├── kb_content → ChunkKBNode("chunk_kb")  — découpe _text en chunks
        │   ├── entities → InsertRecordNode("agg_inserts")  — UNWIND MERGE chunks
        │   │   └── inserted → EmbedRecordNode("agg_embeds")  — embed _text de chaque chunk ← ICI
        │   └── relations → LinkRecordNode("agg_links")  — HAS_CHUNK + SOURCED
        └── done → FlushFTSNode("flush_fts")  — CALL FLUSH_LUCIVY_INDEX
```

### Données portées par chaque chunk (generate_chunk_records)

```rust
// Champs dans EntityRecord.data pour un chunk :
_uuid, _parent_uuid, _parent_field, _kb_name,
_source_field, _source_entity, _source_uuid,
_text, _text_hash, _index,
_start_char, _end_char, _start_line, _end_line,
_core_start_char, _core_end_char, _core_start_line, _core_end_line,
_content_offset
```

## Fix appliqué

### Fichier modifié : `src/dataflow/record_nodes.rs`

**Changement 1** : Réécriture de la boucle de collecte de texte dans `EmbedRecordNode::execute()`.

Avant (faux) :
```rust
// Boucle sur kb_metadata, cherche entity_name dans kb_meta.entities
// Collecte title_for + content_for fields → concatène → embed
```

Après (correct) :
```rust
// Lit directement depuis les data du chunk :
let embed_text = rec.data.get("_text");      // texte à embedder
let kb_name = rec.data.get("_kb_name");      // → colonne {kb}_embedding
let text_hash = rec.data.get("_text_hash");  // → idempotence
```

**Changement 2** : Suppression du service `kb_metadata` (plus nécessaire pour ce nœud).

Le reste du code (idempotence, dense/sparse/dual embedding, UNWIND SET) est inchangé et fonctionne avec les `EmbedWork` produits.

## État après le fix

- `cargo check --lib` : ✅ compile clean
- `cargo test --lib` : ✅ 392 pass, 0 fail
- **E2E `batch_observe_multi_entity`** : ❌ FAIL — `processed=0, failed=30`

### Régression E2E

Le fix de `EmbedRecordNode` a introduit une régression pire : **tout le drain échoue** (30 ops failed, 0 processed). Avant le fix, le drain fonctionnait (processed=30) mais les embeddings étaient manquants.

L'erreur est probablement dans un nœud en amont qui échoue silencieusement. Le runtime capture l'erreur et retourne `Err(e)` dans `drain()` → `FlushResult { processed: 0, failed: op_count }`.

### Piste de debug pour la prochaine session

1. **Ajouter du logging** dans `drain()` pour afficher l'erreur `e` (actuellement émise via event_bus mais pas affichée dans les tests)
2. Vérifier si c'est un problème de **validation du graphe** (un service manquant, un port non connecté)
3. L'erreur pourrait venir de `graph.validate()` dans `build_ingestion_graph()` — vérifier si le retrait de `kb_metadata` du service registry impacte un autre nœud que EmbedRecordNode
4. Chercher quel nœud utilise `kb_metadata` comme service :
   - `GatherKBNode` — oui, lit kb_metadata
   - `ChunkKBNode` — oui, lit kb_metadata + chunker_cache
   - `EmbedRecordNode` — **plus maintenant** (retiré par le fix)
   - Vérifier que le service est toujours **enregistré** dans `build_ingestion_graph()` même si EmbedRecordNode ne l'utilise plus

## Autres changements de la session

### Test accent GTest (conservé)

Ajouté `AccentNormalization_ContainsFuzzy` dans `lucivy_fts_test.cpp` :
- Insère "tronçonneuse" dans l'index
- Cherche "tronconneuse" (sans cédille) en contains fuzzy dist 1
- Résultat : ✅ 1 match — la normalisation d'accents fonctionne

### Tests validés

- C++ GTests : **25/25 PASSED** (incluant le nouveau test accent)
- Rust unit tests : **392/392 PASSED**
- E2E : ❌ 1 FAIL (`batch_observe_multi_entity`)

## Fichiers modifiés (cette session)

| Fichier | Changement |
|---|---|
| `extension/lucivy_fts/test/lucivy_fts_test.cpp` | +test `AccentNormalization_ContainsFuzzy` |
| `extension/rag3weaver/src/dataflow/record_nodes.rs` | Réécriture collecte texte dans EmbedRecordNode (chunk-aware) |
