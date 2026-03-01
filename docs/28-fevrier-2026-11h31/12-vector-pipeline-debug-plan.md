# 12 — Debug pipeline vector Catalog : diagnostic complet + plan de fix

## Résumé du problème

Les tests Phase 2 (vector HNSW via `Catalog.search()`) échouent avec `vector_count=0`. Le raw pipeline fonctionne parfaitement (test `phase2_raw_vector_pipeline` ✅).

## Diagnostic (3 bugs trouvés et corrigés dans cette session)

### Bug 1 : `chunked: false` → aucun EmbedOp généré

**Cause** : la config de test Phase 2 avait `chunking.enabled: false` et tous les fields avec `chunked: false`. Or les embeddings ne sont générés que pour les chunks (dans `compute_chunk_ops()`), jamais pour l'entité parent directement.

**Fix appliqué** : supprimé `chunked: bool` de `FieldDef` et `enabled: bool` de `ChunkingConfig`. Ajouté `FieldDef::is_chunked()` qui retourne `content_for.is_some()`. Un field est chunké ssi il est content pour un KB. Plus de confusion possible.

**Fichiers modifiés** :
- `extension/rag3weaver/src/config.rs` — supprimé `chunked`, supprimé `ChunkingConfig.enabled`, ajouté `is_chunked()`
- `extension/rag3weaver/src/schema.rs` — `entity_has_chunks()` utilise `f.is_chunked()`
- `extension/rag3weaver/src/catalog.rs` — `field_def.is_chunked()`
- `extension/rag3weaver/src/validator.rs`, `search.rs` — retiré `chunked:` des constructions FieldDef
- `extension/rag3weaver/tests/e2e_search.rs`, `e2e_native.rs` — idem

### Bug 2 : search vector ciblait `Document` au lieu de `Document_Chunk`

**Cause** : dans `Catalog::search()`, la recherche vector utilisait `entity = kb.title.entity` (= `"Document"`), mais les embeddings sont stockés sur `Document_Chunk`. L'index HNSW est sur la table chunk, pas la table parent.

**Fix appliqué** : ajouté `vector_entity` qui résout à `"{entity}_Chunk"` quand `entity_has_chunks()` est vrai. Utilisé pour `search_vector()` et `search_sparse_cypher()`.

**Fichier modifié** : `extension/rag3weaver/src/catalog.rs` (dans la méthode `search()`, lignes ~830-980)

### Bug 3 : SET sur colonne indexée HNSW = impossible (LE BLOQUEUR)

**Cause** : l'extension vector bloque **tout SET** sur une colonne qui a un index HNSW (`initUpdateState()` throw RuntimeException). Le flow actuel est :
1. `initialize()` → crée tables + crée index HNSW sur `Document_Chunk.kb_embedding`
2. `create()` → enqueue InsertOp (chunk créé avec embedding NULL) + EmbedOp
3. `drain()` → InsertProcessor crée le chunk (embedding NULL), puis EmbedProcessor fait `SET n.kb_embedding = [...]` → **ERREUR**

**Erreur exacte** : `"Cannot set property vec in table embeddings because it is used in one or more indexes. Try delete and then insert."`

**Confirmé par le pub/sub queue** (voir section ci-dessous) :
```
ProcessingBatch { op_type: "embed", priority: 3, items: ["opi_9", "opi_12", "opi_15"] }
BatchFailed { op_type: "embed", priority: 3, error: "Cannot set property vec in table embeddings because it is used in one or more indexes..." }
```

**Ce qui fonctionne avec l'extension vector** :
- INSERT après création d'index ✅ (le graph HNSW se met à jour dynamiquement via `commitInsert()`)
- SET/UPDATE sur colonne indexée ❌ (bloqué par design)
- DELETE ✅ (le graph n'est pas mis à jour, lazy)

**Le code bloqueur** : `extension/vector/src/include/index/hnsw_index.h` lignes 109-113, `initUpdateState()` throw inconditionnellement.

## Plan de fix pour la prochaine session

### La solution : fusionner InsertOp + EmbedOp pour les chunks

Au lieu du flow actuel (insert NULL → SET embedding), faire un seul CREATE avec l'embedding dedans. L'INSERT après index fonctionne, c'est le SET qui ne fonctionne pas.

**Approche concrète** :

1. **Modifier `compute_chunk_ops()`** (catalog.rs ~1216-1361) :
   - Au lieu d'émettre `CatalogOp::Insert(chunk_data)` + `CatalogOp::Embed(texts)` séparément
   - Émettre un **nouveau op** `CatalogOp::InsertWithEmbed` qui contient les données chunk + le texte à embedder
   - OU : ajouter un champ `embed_text: Option<String>` sur `InsertOp` pour que l'InsertProcessor puisse embedder inline

2. **Modifier l'InsertProcessor ou créer un InsertEmbedProcessor** :
   - Reçoit l'item, appelle `embedder.embed()`, met l'embedding dans `chunk_data["kb_embedding"]` AVANT le CREATE
   - Le CREATE inclut l'embedding directement → pas de SET nécessaire

3. **Alternative plus simple : différer la création d'index** :
   - Séparer `schema.indexes` en `fts_indexes` (créés dans initialize) et `vector_indexes` (créés après le premier drain)
   - Avantage : aucun changement dans le flow InsertOp/EmbedOp
   - Inconvénient : ne résout pas le problème pour les drains incrémentaux suivants (les updates d'embedding via SET resteront bloqués)

**Recommandation** : option 1 ou 2 (fusionner insert + embed) car c'est compatible avec l'incrémentalité. Le flow UPDATE d'un chunk existant dans le Catalog est déjà DELETE anciens chunks + INSERT nouveaux, donc pas de SET nécessaire.

### Modifications nécessaires

| Fichier | Action |
|---------|--------|
| `extension/rag3weaver/src/ops.rs` | Ajouter champ `embed_text: Option<(String, String)>` (kb_name, text) sur `InsertOp`, ou nouveau variant `InsertEmbed` |
| `extension/rag3weaver/src/catalog.rs` `compute_chunk_ops()` | Ne plus émettre `CatalogOp::Embed` séparé pour les chunks, mettre le texte dans l'InsertOp |
| `extension/rag3weaver/src/catalog.rs` `InsertProcessor` | Si `embed_text` présent, appeler `self.embedder.embed()` et ajouter l'embedding dans `data` avant le CREATE |
| `extension/rag3weaver/src/catalog.rs` `InsertProcessor` | Ajouter `embedder: Arc<dyn Embedder>` et `embedding_dim: usize` au struct |

### Étapes détaillées

1. Ajouter `embed_texts: Vec<(String, String)>` (vec de (kb_name, text)) sur `InsertOp`
2. Dans `compute_chunk_ops()`, au lieu de push `CatalogOp::Embed`, mettre `(kb_name, embed_text)` dans le `InsertOp.embed_texts`
3. Dans `InsertProcessor`, si `embed_texts` non vide :
   - Collecter tous les textes du batch
   - Appeler `embedder.embed()` en une seule fois
   - Mettre chaque embedding dans `insert.data["{kb_name}_embedding"]` comme `CypherValue::List(floats)`
   - Le CREATE contiendra l'embedding directement
4. Passer `self.embedder.clone()` à l'InsertProcessor dans `initialize()`
5. Retirer l'`EmbedProcessor` (plus nécessaire pour les chunks), OU le garder pour d'éventuels cas sans chunk

### Ce qui restera à faire après le fix

- Les 9 tests Phase 2 via Catalog (3 MiniLM, 3 Multilingual, 3 BGE-M3) devraient passer
- Le test `phase2_raw_vector_pipeline` continue de fonctionner (bypass Catalog)
- Retirer les logs debug de `setup_vector_catalog` (execute_raw, queue events)
- Phases 3-10 du cahier des charges (doc 09)

## Ajouts utiles faits dans cette session

### `Catalog.conn()` et `Catalog.execute_raw()`

Ajoutés pour faciliter le debug dans les tests :
```rust
pub fn conn(&self) -> &dyn DbConnection { self.conn.as_ref() }
pub async fn execute_raw(&self, cypher: &str) -> Result<QueryResult, CatalogError> { ... }
```

### Queue pub/sub (`QueueEvent`)

Système d'events sur la queue pour tracer le cycle de vie des opérations :

```rust
pub enum QueueEvent {
    Enqueued { id, op_type, priority },
    ProcessingBatch { op_type, priority, items: Vec<String> },
    BatchCompleted { op_type, priority, items: Vec<String> },
    BatchFailed { op_type, priority, items: Vec<String>, error },
    Injected { count, source_priority, ops: Vec<(op_type, priority)> },
}
```

Usage :
```rust
let mut queue_rx = catalog.subscribe_queue();
// ... drain ...
while let Ok(event) = queue_rx.try_recv() {
    eprintln!("[queue] {:?}", event);
}
```

C'est ce qui nous a permis de trouver le bug 3 immédiatement : on voyait le `BatchFailed` sur l'embed avec l'erreur exacte.

## État des tests

| Phase | Résultat |
|-------|----------|
| Phase 0 (6 tests CRUD) | ✅ tous passent |
| Phase 1 (6 tests BM25) | ✅ tous passent |
| Phase 2 raw pipeline | ✅ passe |
| Phase 2 via Catalog (9 tests) | ❌ bloqués par bug 3 (à fixer) |
