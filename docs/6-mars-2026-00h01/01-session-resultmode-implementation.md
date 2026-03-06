# Doc 01 — Session : Implémentation ResultMode (Phase 0)

**Date** : 6 mars 2026
**Branche** : `feature/kb-index-architecture`
**Statut** : Implémentation terminée, tests E2E à écrire

---

## Résumé

Implémentation complète de `ResultMode` (Aggregated / SourceResolved / Detailed) dans rag3weaver. Trois modes de présentation des résultats de search, sans changement de comportement par défaut.

**Build** : clean, 346/346 tests lib passent, 1 warning pré-existant (`maybe_enqueue_chunk_op` dead_code).

---

## Changements par fichier

### `src/schema.rs`

**2 colonnes ajoutées** au chunk DDL (`generate_index_chunk_table_ddl()`) :
- `_source_entity STRING` — type de l'entité source (ex: "File", "Directory")
- `_source_uuid STRING` — UUID de l'entité source

Ajoutées après `_source_field`, avant `_text`. Le chunk passe de 17 à 19 colonnes fixes (+ embeddings dynamiques).

### `src/catalog.rs`

**AggregateProcessor** (~ligne 2170) : 2 inserts ajoutés dans `chunk_data` :
```rust
chunk_data.insert("_source_entity", CypherValue::String(source.entity_name.clone()));
chunk_data.insert("_source_uuid", CypherValue::String(source.entity_uuid.clone()));
```
`source` est le `SourceContent` qui a déjà `entity_name` et `entity_uuid` — zéro query supplémentaire.

**`resolve_to_source_entities()`** — nouvelle méthode privée sur `Catalog` :
1. Lit `_source_entity` et `_source_uuid` depuis `result.data` pour chaque résultat
2. Groupe par entity type → `HashMap<String, Vec<String>>`
3. Batch fetch : `MATCH (n:{entity}) WHERE n._uuid IN [...] RETURN n`
4. Remplace `uuid`, `entity`, `data` dans chaque résultat
5. Déduplique par UUID source (garde meilleur score)

**`search()`** — branchement ajouté après enrichissement :
```rust
if options.result_mode == ResultMode::SourceResolved {
    self.resolve_to_source_entities(&mut fused).await?;
}
```

**Appels mis à jour** : `search_bm25_chunked()` et `resolve_vector_chunks()` reçoivent `options.result_mode`.

### `src/search.rs`

**Nouveaux types** :

| Type | Description |
|------|-------------|
| `ResultMode` enum | `Aggregated` (défaut), `SourceResolved`, `Detailed` |
| `AttributedChunk` struct | Chunk avec `source_entity`, `source_uuid`, `source_field` |

**Structs modifiées** :

| Struct | Champ ajouté |
|--------|-------------|
| `SearchOptions` | `result_mode: ResultMode` (défaut `Aggregated`) |
| `SearchResult` | `chunks: Option<Vec<AttributedChunk>>` (None sauf Detailed) |
| `ChunkRecord` | `source_entity: String`, `source_uuid: String` |

**`resolve_and_enrich_chunked()`** — Cypher étendu :
- Ajout de `c._source_entity` et `c._source_uuid` au RETURN
- Mappés vers `ChunkRecord.source_entity` et `ChunkRecord.source_uuid`

**`resolve_vector_chunks()`** — paramètre `result_mode` ajouté :
- Cypher étendu avec `c._source_entity`, `c._source_uuid`, `c._source_field`
- Mode Aggregated : collapse au best chunk par parent (inchangé)
- Mode Detailed : groupe tous les chunks par parent en `Vec<AttributedChunk>`

**`search_bm25_chunked()`** — paramètre `result_mode` ajouté :
- Mode Aggregated : un SearchResult par chunk intersectant (inchangé)
- Mode Detailed : un SearchResult par parent avec `chunks: Some(vec![AttributedChunk...])` contenant tous les chunks avec overlap

**`fuse_results()`** — fusion des chunks Detailed :
- Nouveau `chunks_map: HashMap<String, Vec<AttributedChunk>>` collecté depuis tous les signaux
- Déduplique par chunk UUID (garde meilleur score)
- Re-attache `chunks` aux résultats fusionnés

### `src/lib.rs`

Exports ajoutés : `ResultMode`, `AttributedChunk`, `ChunkInfo`.

---

## Comportement par mode

### Aggregated (défaut, inchangé)

```json
{
  "uuid": "idx-abc",
  "entity": "TreeKB_Index",
  "score": 0.95,
  "data": { "_title": "src", "_source_entity": "Directory", "_source_uuid": "dir-123" },
  "chunk": { "uuid": "chunk-1", "text": "...", "score": 0.88 },
  "chunks": null
}
```

### SourceResolved

```json
{
  "uuid": "dir-123",
  "entity": "Directory",
  "score": 0.95,
  "data": { "name": "src", "absolute_path": "/repo/src/", "depth": 1 },
  "chunk": { "uuid": "chunk-1", "text": "...", "score": 0.88 },
  "chunks": null
}
```

### Detailed

```json
{
  "uuid": "idx-abc",
  "entity": "TreeKB_Index",
  "score": 0.95,
  "data": { "_title": "src" },
  "chunk": null,
  "chunks": [
    { "uuid": "chunk-1", "text": "...", "score": 0.88, "sourceEntity": "Directory", "sourceUuid": "dir-123", "sourceField": "name" },
    { "uuid": "chunk-5", "text": "...", "score": 0.72, "sourceEntity": "File", "sourceUuid": "file-456", "sourceField": "content" }
  ]
}
```

---

## Ce qui reste

### Tests E2E (task #116)

À écrire dans `tests/e2e_result_mode.rs` ou ajouter aux tests existants :

1. **Aggregated non-régression** — search par défaut retourne les mêmes résultats qu'avant
2. **SourceResolved** — vérifie `entity == "Directory"`, `uuid == source_uuid`, `data` contient les champs de Directory (pas `_title`/`_content`)
3. **Detailed** — vérifie `chunks.is_some()`, chaque `AttributedChunk` a `source_entity`/`source_uuid`/`source_field` corrects
4. **Detailed multi-signal** — search avec BM25+Vector, vérifie que les chunks des deux signaux sont mergés et dédupliqués
5. **SourceResolved dédup** — si pertinent (normalement 1:1 pour une KB donnée)

### Migration des KBs existantes

Pas de migration — les KBs existantes seront re-créées (pré-production). Les nouvelles colonnes `_source_entity`/`_source_uuid` sur les chunks seront remplies à la prochaine ingestion.

### Note sur le `_source_field` dans AttributedChunk

En mode Detailed pour BM25, le `source_field` de l'AttributedChunk vient de `ChunkRecord.parent_field` (qui est `_parent_field` du chunk, identique à `_source_field`). C'est correct car les deux sont remplis avec `source.field_name` à l'ingestion.

---

## Décisions prises

1. **Colonnes sur chunks plutôt que query SOURCED** — ajout de `_source_entity`/`_source_uuid` directement sur le chunk (recommandation section 7.5 du doc 01-design). Zéro query SOURCED nécessaire au search time.

2. **Pas de `max_chunks_per_result`** — pas ajouté pour l'instant, le client filtre côté appelant. À ajouter si les volumes deviennent un problème.

3. **`chunks: None` en Aggregated/SourceResolved** — le champ `chunks` est toujours `None` sauf en mode Detailed, pour ne pas casser la sérialisation existante.

4. **fuse_results ne connaît pas le result_mode** — il merge les chunks de manière transparente. Les fonctions de résolution en amont (resolve_vector_chunks, search_bm25_chunked) décident de peupler `chunks` ou `chunk` selon le mode.
