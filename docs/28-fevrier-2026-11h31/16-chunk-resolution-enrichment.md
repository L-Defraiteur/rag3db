# 16 — Chunk Resolution + Enrichment des résultats de recherche

## Résumé

Suite au doc 15 (HNSW DELETE/UPDATE finalisé, 64/64 tests vector), on est revenu à rag3weaver pour valider les tests Phase 2 vector via Catalog.search(). **Le fix HNSW fonctionne** — les embed ops passent (BatchCompleted au lieu de BatchFailed). Mais les résultats de search avaient `data: None` et `chunk: None` → les titres étaient vides.

Implémenté la résolution chunk→parent + enrichissement des données parent. **8/10 tests Phase 2 passent** (vs 1/10 avant). Les 2 échecs restants sont un bug HNSW avec UNWIND SET.

## Ce qui a été fait

### 1. Diagnostic initial

Lancé les 10 tests Phase 2 après rebuild native-test avec les changements HNSW. Résultat :
- `phase2_raw_vector_pipeline` : ✅ (comme avant)
- Les 9 tests Catalog : ❌ tous échouent avec `Top result: '' (score=0.60)` — title vide

**Cause** : `parse_hnsw_results()` (search.rs:486) met `data: None, chunk: None`. Le vector search retourne juste `{uuid, score}` sans données. Et les UUIDs sont des chunk UUIDs (`Document_Chunk`), pas des parent UUIDs (`Document`).

### 2. Étendre ChunkInfo (search.rs)

Ajouté les bornes lignes/chars au struct :
```rust
pub struct ChunkInfo {
    pub uuid: String,
    pub text: String,
    pub index: usize,
    pub score: f64,
    pub start_line: usize,   // NOUVEAU
    pub end_line: usize,     // NOUVEAU
    pub start_char: usize,   // NOUVEAU
    pub end_char: usize,     // NOUVEAU
}
```

### 3. `resolve_chunk_results()` — méthode abstraite (search.rs)

Nouvelle fonction réutilisable par vector ET sparse. Convertit des résultats chunk-level en résultats parent-level avec ChunkInfo.

```rust
pub async fn resolve_chunk_results(
    conn: &dyn DbConnection,
    chunk_entity: &str,   // "Document_Chunk"
    parent_entity: &str,  // "Document"
    results: Vec<SearchResult>,
) -> Result<Vec<SearchResult>, CatalogError>
```

**Algorithme :**
1. Collecter les chunk UUIDs distincts
2. Batch query : `MATCH (c:{chunk_entity}) WHERE c._uuid IN [...] RETURN c._uuid, c._parent_uuid, c._text, c._index, c._start_line, c._end_line, c._start_char, c._end_char`
3. Construire `HashMap<chunk_uuid, ChunkMeta>`
4. Grouper par parent_uuid, garder le chunk avec le meilleur score par parent
5. Retourner `Vec<SearchResult>` avec `uuid=parent_uuid`, `entity=parent_entity`, `chunk=Some(ChunkInfo)`

### 4. `enrich_results_with_data()` (search.rs)

Nouvelle fonction qui batch-fetch les données parent et popule `result.data`.

```rust
pub async fn enrich_results_with_data(
    conn: &dyn DbConnection,
    entity: &str,
    fields: &[String],
    results: &mut [SearchResult],
) -> Result<(), CatalogError>
```

Construit dynamiquement le RETURN clause depuis les champs de l'entity config : `RETURN n._uuid AS _uuid, n.title AS title, n.body AS body, ...`

### 5. Câblage dans Catalog::search() (catalog.rs)

**Avant fusion** : résolution chunk→parent pour vector et sparse :
```rust
let vector_results = if is_chunked && !vector_results.is_empty() {
    search::resolve_chunk_results(conn, &vector_entity, &entity, vector_results).await?
} else { vector_results };
// Idem pour sparse_results
```

**Après pagination** : enrichissement données parent :
```rust
search::enrich_results_with_data(conn, &entity, &enrich_fields, &mut fused).await?;
```

### 6. Préservation ChunkInfo dans la fusion

`fuse_results()` reconstruisait les SearchResult avec `chunk: None`. Corrigé en :
1. Avant fusion : construire `HashMap<uuid, ChunkInfo>` depuis tous les inputs (best chunk par UUID)
2. Après fusion : réattacher les ChunkInfo aux résultats fused

### 7. Fix EmbedProcessor : SET individuel au lieu de UNWIND (EN COURS)

Remplacé le UNWIND batch SET par des SET individuels par chunk. Le UNWIND + SET sur colonne indexée HNSW sautait silencieusement certains items (toujours le texte français avec accents pour BGE-M3/Multilingual).

**Note** : ce fix est un workaround. Le vrai problème est dans le HNSW `update()` qui ne gère pas correctement le UNWIND (plusieurs SET dans une seule transaction). C'est un bug à investiguer dans l'extension vector C++.

## Bug découvert : UNWIND SET + HNSW index skip silencieux

### Symptôme
Sur 3 chunks, le 3e a systématiquement un embedding NULL après `BatchCompleted` (pas d'erreur). Le chunk affecté est toujours le texte français avec accents : `'La cuisine française est mondialement reconnue...'`

### Diagnostic
- L'EmbedProcessor fait un `UNWIND $items AS item MATCH (n:...) SET n.kb_embedding = item.emb`
- Les 3 embeddings sont calculés correctement (vérification dimension OK)
- Le BatchCompleted indique que le SET n'a pas levé d'erreur
- Mais à la vérification post-drain, `size(c.kb_embedding)` retourne Null pour un chunk

### Hypothèses
1. **Bug HNSW update() avec UNWIND** : quand UNWIND déclenche plusieurs SET consécutifs sur une colonne indexée HNSW, le 2e ou 3e SET peut être ignoré. Le `HNSWUpdateState` est peut-être réutilisé sans reset entre les itérations UNWIND.
2. **Caractères spéciaux** : les accents français dans le texte ne devraient pas affecter l'embedding (qui est un tableau de floats), mais pourraient affecter le MATCH sur UUID.
3. **Pas un problème d'accents** : c'est toujours le même chunk qui échoue quel que soit l'ordre d'insertion → c'est probablement le 3e item du UNWIND qui est skippé.

### Workaround appliqué
SET individuel par chunk au lieu de UNWIND batch. Non testé encore.

### Fix définitif à faire
Investiguer `OnDiskHNSWIndex::update()` et la réutilisation de `HNSWUpdateState` entre les itérations UNWIND. Probablement un state qui n'est pas réinitialisé entre les appels successifs de `update()` dans la même transaction.

## Résultats tests Phase 2

| Test | Avant | Après |
|------|-------|-------|
| phase2_raw_vector_pipeline | ✅ | ✅ |
| phase2_vector_minilm_programming | ❌ (title vide) | ✅ |
| phase2_vector_minilm_cooking | ❌ | ✅ |
| phase2_vector_minilm_ml | ❌ | ✅ |
| phase2_vector_multilingual_programming | ❌ | ✅ |
| phase2_vector_multilingual_ml | ❌ | ✅ |
| phase2_vector_multilingual_french | ❌ | ❌ (embedding Null — UNWIND bug) |
| phase2_vector_bgem3_programming | ❌ | ✅ |
| phase2_vector_bgem3_french | ❌ | ✅ |
| phase2_vector_bgem3_ml | ❌ | ❌ (embedding Null — UNWIND bug) |

**Score : 8/10** (vs 1/10 avant cette session)

## Fichiers modifiés

| Fichier | Modifications |
|---------|--------------|
| `extension/rag3weaver/src/search.rs` | Étendu ChunkInfo (+4 champs), ajouté `resolve_chunk_results()`, ajouté `enrich_results_with_data()`, modifié `fuse_results()` pour préserver ChunkInfo |
| `extension/rag3weaver/src/catalog.rs` | Câblé resolve + enrich dans `search()`, changé EmbedProcessor de UNWIND→SET individuels |
| `extension/rag3weaver/tests/e2e_search.rs` | Amélioré debug logs chunks (uuid, parent, text snippet, dim) |

## Architecture finale search

```
Catalog::search("kb", query, options)
    │
    ▼  Search
    search_vector(conn, "Document_Chunk", ...)
        → Vec<SearchResult> {uuid=chunk_uuid, score, entity="Document_Chunk"}
    │
    ▼  Resolve chunks → parents (NOUVEAU)
    resolve_chunk_results(conn, "Document_Chunk", "Document", results)
        → Vec<SearchResult> {uuid=parent_uuid, score=best_chunk, chunk=Some(ChunkInfo)}
    │
    ▼  Fusion (si hybrid)
    fuse_results(vector, bm25, sparse)
        → Vec<SearchResult> {uuid=parent_uuid, score=fused, chunk=preserved}
    │
    ▼  Pagination
    │
    ▼  Enrich (NOUVEAU)
    enrich_results_with_data(conn, "Document", &fields, results)
        → result.data = Some({title: "Rust Programming", body: "..."})
```

## Prochaines étapes

1. **Tester le workaround SET individuel** → devrait faire passer les 2 tests restants (10/10)
2. **Investiguer le bug UNWIND + HNSW update** → probablement dans `update()` (hnsw_index.cpp), state non réinitialisé entre itérations UNWIND
3. **Phase B : BM25 highlights → chunk matching** → pour Hybrid fusion correcte :
   - Capturer `highlights` JSON depuis QUERY_LUCIVY_INDEX (colonne 3, actuellement ignorée)
   - Matcher highlight byte ranges avec chunk `_start_char/_end_char`
   - Permettre fusion BM25+vector au niveau chunk
4. **Phases suivantes du cahier des charges** (doc 09) : Sparse, Hybrid, Fusion, Filtres, Explore

## Leçons retenues

1. **UNWIND + SET sur colonne indexée HNSW peut skip des items** — le HNSW `update()` est probablement appelé N fois dans la même transaction mais le state n'est pas réinitialisé. Workaround : SET un par un.
2. **La fusion reconstruit les SearchResult** — il faut un mécanisme de préservation (chunk_map avant/après) sinon les ChunkInfo sont perdues.
3. **resolve_chunk_results() est abstraite** — même signature pour vector et sparse, utilisable pour tout search qui cible la table chunk.
