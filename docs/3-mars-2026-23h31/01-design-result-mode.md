# Doc 01 — Design : ResultMode (Aggregated / SourceResolved / Detailed)

**Date** : 3 mars 2026
**Branche** : `feature/kb-index-architecture`

---

## 1. Contexte et motivation

Après la simplification filtres (all-allowed_ids, task #104), l'étape suivante est de donner de la flexibilité sur **comment les résultats de search sont présentés**.

Actuellement, `Catalog::search()` retourne des `SearchResult` qui représentent des `{KB}_Index` entries (documents agrégés). Chaque résultat contient au mieux **un seul chunk** (le meilleur match). L'appelant n'a aucun moyen de :
- Récupérer l'entité source originale (Directory, File, Container, Scope...)
- Voir tous les chunks pertinents et savoir d'où chacun vient
- Grouper les chunks par entité/champ source

### Cas d'usage concret : Code Domain (futur)

Un `Container` (classe/module) aurait un index entry dont le `_content` concatène :
```
Container.summary   → "Service d'authentification JWT, gère login/logout..."
Scope.body (enfant) → "async fn handle_auth(req: Request) -> Response { ... }"
Scope.body (enfant) → "fn verify_token(token: &str) -> Result<Claims> { ... }"
```

Pour une recherche `"auth"`, on voudrait retourner :
- **Titre** : "AuthService"
- **2 chunks pertinents du summary** (attribution: `Container.summary`)
- **2 chunks pertinents des scopes** (attribution: `Scope.body`)

Sans code domaine-spécifique.

---

## 2. Ce qu'on a déjà dans le graph

### 2.1 Relations existantes

```
{TitleEntity}_IN_{KB}        : Directory → TreeKB_Index         (1:1, title entity → index entry)
{KB}_Index_HAS_CHUNK         : TreeKB_Index → TreeKB_Index_Chunk (1:N, index → chunks)
{Entity}_SOURCED_{KB}        : File → TreeKB_Index_Chunk         (1:N, source entity → chunks qu'elle a produit)
```

**`SOURCED` est la clé** : chaque chunk est relié à l'entité source qui l'a produit. Pour un chunk issu de `File.content`, on a : `(file:File)-[:File_SOURCED_TreeKB]->(chunk:TreeKB_Index_Chunk)`.

### 2.2 Colonnes du chunk (`{KB}_Index_Chunk`)

| Colonne | Type | Rôle |
|---|---|---|
| `_uuid` | STRING (PK) | ID unique du chunk |
| `_parent_uuid` | STRING | UUID de l'index entry parent |
| `_parent_field` | STRING | Champ source (duplicata de `_source_field`) |
| `_source_field` | STRING | **Nom du champ source** (ex: `"content"`, `"summary"`, `"absolute_path"`) |
| `_kb_name` | STRING | Nom de la KB |
| `_text` | STRING | Texte du chunk |
| `_text_hash` | STRING | Hash du texte |
| `_index` | INT64 | Index du chunk dans sa source |
| `_start_char` / `_end_char` | INT64 | Offsets char dans le champ source |
| `_start_line` / `_end_line` | INT64 | Lignes dans le champ source |
| `_core_start_char` / `_core_end_char` | INT64 | Zone core (sans overlap) |
| `_core_start_line` / `_core_end_line` | INT64 | Lignes core |
| `_content_offset` | INT64 | Offset du début de ce source field dans `_content` concaténé |
| `{kb}_embedding` | FLOAT[dim] | Embedding dense |
| `{kb}_sparse_*` | INT64[] / DOUBLE[] | Embedding sparse (optionnel) |

### 2.3 Ce qu'on peut déduire par chunk

Avec **SOURCED** + **`_source_field`**, pour chaque chunk on connaît :
- **Entité source** : via `MATCH (e)-[:Entity_SOURCED_KB]->(chunk)` → type et UUID de `e`
- **Champ source** : via `chunk._source_field` → `"content"`, `"summary"`, etc.
- **Position dans le champ** : via `_start_char` / `_end_char`
- **Position dans le `_content` global** : via `_content_offset` + `_start_char`

Aucune colonne supplémentaire nécessaire.

### 2.4 Colonnes de l'index entry (`{KB}_Index`)

| Colonne | Rôle |
|---|---|
| `_uuid` | ID unique |
| `_title` | Texte titre (ex: nom du Directory) |
| `_content` | Texte concaténé de toutes les sources |
| `_source_entity` | Type de l'entité titre (ex: `"Directory"`) |
| `_source_uuid` | UUID de l'entité titre |
| `_content_hash` | Hash du content pour détecter les changements |

---

## 3. Comportement actuel du search (rappel)

### 3.1 Trois chemins de recherche

| Chemin | Granularité | Résolution |
|---|---|---|
| **BM25 chunked** (`search_bm25_chunked`) | Index entry (FTS sur `_title` + `_content`) | Highlights → match chunks par overlap. **1 résultat = 1 index entry + meilleur chunk**. |
| **Vector** (`search_vector` + `resolve_vector_chunks`) | Chunk (HNSW sur embeddings chunk) | Multiple chunks → collapse au parent. **N chunks → 1 résultat par parent (meilleur score)**. |
| **Sparse** (`search_sparse_cypher`) | Chunk | Idem vector. |

### 3.2 Fusion (RRF / Weighted)

Après les 3 recherches, `fuse_results()` combine les scores par UUID de résultat (= UUID de l'index entry). La fusion produit un score fusionné par index entry.

### 3.3 Structures actuelles

```rust
pub struct SearchResult {
    pub uuid: String,                              // UUID de {KB}_Index
    pub score: f64,
    pub entity: Option<String>,                    // "{KB}_Index"
    pub data: Option<BTreeMap<String, CypherValue>>,  // _title, _content, _source_entity, _source_uuid
    pub chunk: Option<ChunkInfo>,                  // Meilleur chunk (un seul)
}

pub struct ChunkInfo {
    pub uuid: String,
    pub text: String,
    pub index: usize,
    pub score: f64,
    pub start_line: usize,
    pub end_line: usize,
    pub start_char: usize,
    pub end_char: usize,
}
```

**Limitation** : `chunk` est un `Option<ChunkInfo>` — un seul chunk max par résultat, sans info sur son origine (entité/champ source).

---

## 4. Design proposé : trois modes

### 4.1 `ResultMode` enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultMode {
    /// Index entry + best chunk (current behavior).
    #[default]
    Aggregated,
    /// Resolved to source entity — uuid/entity/data are the original entity's.
    SourceResolved,
    /// Index entry + ALL matched chunks with source attribution per chunk.
    Detailed,
}
```

Ajouté dans `SearchOptions` :

```rust
pub struct SearchOptions {
    // ... existing fields ...
    pub result_mode: ResultMode,
}
```

### 4.2 Mode `Aggregated` (défaut, inchangé)

Comportement actuel. Pas de changement.

```json
{
  "uuid": "idx-abc",
  "entity": "TreeKB_Index",
  "score": 0.95,
  "data": { "_title": "src", "_content": "...", "_source_entity": "Directory", "_source_uuid": "dir-123" },
  "chunk": { "uuid": "chunk-1", "text": "auth.ts\n...", "score": 0.88, ... }
}
```

### 4.3 Mode `SourceResolved`

Résout vers l'entité titre. Remplace `uuid`/`entity`/`data` par ceux de l'entité source.

**Implémentation** : après fusion + pagination, pour chaque résultat :
1. Lire `_source_entity` et `_source_uuid` depuis `data`
2. Grouper par entity type
3. Batch fetch : `MATCH (n:{entity}) WHERE n._uuid IN [...] RETURN n`
4. Remplacer `uuid`, `entity`, `data`
5. Le `chunk` (best) reste tel quel (mais perd son contexte d'attribution)

```json
{
  "uuid": "dir-123",
  "entity": "Directory",
  "score": 0.95,
  "data": { "name": "src", "absolute_path": "/repo/src/", "depth": 1 },
  "chunk": { "uuid": "chunk-1", "text": "auth.ts\n...", "score": 0.88, ... }
}
```

**Dédup** : si plusieurs index entries pointent vers la même source entity (théoriquement 1:1 pour une KB donnée, mais possible cross-KB à l'avenir), garder le meilleur score.

### 4.4 Mode `Detailed`

Retourne l'index entry avec **tous les chunks pertinents**, chacun attribué à son entité et champ source.

#### Nouvelle struct : `AttributedChunk`

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributedChunk {
    pub uuid: String,
    pub text: String,
    pub index: usize,
    pub score: f64,
    pub start_line: usize,
    pub end_line: usize,
    pub start_char: usize,
    pub end_char: usize,
    /// Source entity type (e.g. "File", "Container", "Scope")
    pub source_entity: String,
    /// Source entity UUID
    pub source_uuid: String,
    /// Source field name (e.g. "content", "summary", "body")
    pub source_field: String,
}
```

#### Modification de `SearchResult`

```rust
pub struct SearchResult {
    pub uuid: String,
    pub score: f64,
    pub entity: Option<String>,
    pub data: Option<BTreeMap<String, CypherValue>>,
    pub chunk: Option<ChunkInfo>,                    // best chunk (Aggregated/SourceResolved)
    pub chunks: Option<Vec<AttributedChunk>>,         // all chunks (Detailed mode only)
}
```

`chunks` est `None` sauf en mode `Detailed`. Le champ `chunk` reste pour compatibilité (Aggregated/SourceResolved).

#### Exemple de résultat Detailed

```json
{
  "uuid": "idx-abc",
  "entity": "CodeKB_Index",
  "score": 0.95,
  "data": { "_title": "AuthService", "_source_entity": "Container" },
  "chunk": null,
  "chunks": [
    {
      "uuid": "chunk-1", "text": "gère login/logout et validation JWT...",
      "score": 0.92, "startLine": 0, "endLine": 3,
      "sourceEntity": "Container", "sourceUuid": "container-1", "sourceField": "summary"
    },
    {
      "uuid": "chunk-5", "text": "async fn handle_auth(req: Request)...",
      "score": 0.88, "startLine": 0, "endLine": 15,
      "sourceEntity": "Scope", "sourceUuid": "scope-42", "sourceField": "body"
    },
    {
      "uuid": "chunk-8", "text": "fn verify_token(token: &str)...",
      "score": 0.85, "startLine": 0, "endLine": 10,
      "sourceEntity": "Scope", "sourceUuid": "scope-43", "sourceField": "body"
    }
  ]
}
```

Le client groupe et sélectionne sans code domaine :

```typescript
const bySource = groupBy(result.chunks, c => `${c.sourceEntity}:${c.sourceField}`);
// { "Container:summary": [chunk-1], "Scope:body": [chunk-5, chunk-8] }
const top2PerSource = mapValues(bySource, chunks => chunks.slice(0, 2));
```

---

## 5. Implémentation Detailed : résolution des sources

### 5.1 Obtenir tous les chunks matchés

**Changement principal** : au lieu de garder 1 chunk par parent, garder tous les chunks pertinents.

Pour **BM25 chunked** : `search_bm25_chunked` retourne déjà le best chunk par index entry. Pour Detailed, il faut retourner **tous les chunks qui intersectent les highlights**, pas juste le meilleur. Le code dans `resolve_bm25_to_chunks()` fait déjà ça (produit N résultats par parent, triés par overlap) — mais `search_bm25_chunked` n'utilise pas ce path. On pourrait réutiliser cette logique.

Pour **vector/sparse** : `resolve_vector_chunks()` collapse au best chunk par parent. Pour Detailed, on skip le collapse et on garde tous les chunks.

### 5.2 Attribuer les chunks à leurs sources

Une fois qu'on a les chunk UUIDs, résoudre via SOURCED :

```cypher
-- Pour chaque entity type de la KB
MATCH (e:Directory)-[:Directory_SOURCED_TreeKB]->(c:TreeKB_Index_Chunk)
WHERE c._uuid IN ['chunk-1', 'chunk-5', 'chunk-8']
RETURN c._uuid AS chunk_uuid, 'Directory' AS source_entity, e._uuid AS source_uuid
UNION ALL
MATCH (e:File)-[:File_SOURCED_TreeKB]->(c:TreeKB_Index_Chunk)
WHERE c._uuid IN ['chunk-1', 'chunk-5', 'chunk-8']
RETURN c._uuid AS chunk_uuid, 'File' AS source_entity, e._uuid AS source_uuid
```

Le `source_field` est lu directement depuis `chunk._source_field` (déjà récupéré dans les résolutions existantes).

On construit le UNION ALL dynamiquement à partir de `kb_meta.entities`.

### 5.3 Alternative : query plus simple via _source_field seulement

Si on n'a pas besoin du UUID de l'entité source (juste entity type + field), on peut éviter le UNION ALL et utiliser la convention : le type d'entité est le premier segment de la relation SOURCED. Mais la relation n'est pas stockée sur le chunk.

**Approche retenue** : faire la query SOURCED pour avoir le triplet complet (entity, uuid, field). C'est 1 round-trip Kuzu, négligeable.

---

## 6. Impact sur les chemins de recherche

### 6.1 BM25 chunked

| Étape | Aggregated (actuel) | Detailed |
|---|---|---|
| QUERY_LUCIVY_INDEX | → offsets + scores + highlights | Idem |
| resolve_and_enrich_chunked | → parent data + tous les chunks | Idem |
| Match highlights → chunks | **1 best** par parent | **Tous** les intersecting |
| Résultat | `chunk: Some(best)` | `chunks: Some(vec![attributed...])` |

### 6.2 Vector / Sparse

| Étape | Aggregated (actuel) | Detailed |
|---|---|---|
| HNSW/sparse search | → chunk offsets + scores | Idem |
| resolve_vector_chunks | → collapse best per parent | → **garder tous les chunks** |
| Résultat | `chunk: Some(best)` | `chunks: Some(vec![attributed...])` |

### 6.3 Fusion

La fusion `fuse_results()` opère par UUID d'index entry. En mode Detailed, après fusion on collecte les chunks de tous les chemins (BM25 + vector + sparse) sur le même index entry, et on déduplique par chunk UUID (un même chunk peut matcher en BM25 ET en vector).

---

## 7. Points de design ouverts

### 7.1 Limite de chunks par résultat

En mode Detailed, un index entry avec 50 chunks pourrait retourner beaucoup de données. Options :
- **Pas de limite** : le client filtre. Simple, flexible.
- **`max_chunks_per_result: usize`** dans SearchOptions (défaut: 10). Garde les top-N par score.
- **`max_chunks_per_source: usize`** dans SearchOptions. Limite par entity:field. Plus fin.

**Recommandation** : `max_chunks_per_result` avec défaut raisonnable (10). Le groupement par source est de la responsabilité du client.

### 7.2 Score des chunks en Detailed

- **BM25** : les chunks héritent du score BM25 du parent (le FTS score est document-level, pas chunk-level). L'overlap avec les highlights donne un **rank** mais pas un score différent.
- **Vector** : chaque chunk a son propre score cosine. Plus précis.
- **Fusion** : en mode Detailed multi-signal, quel score donner à chaque chunk ? Options :
  - Score du parent (fusion) pour tous les chunks → simple, cohérent
  - Score spécifique par chunk pour vector/sparse, score parent pour BM25 → plus précis mais asymétrique

**Recommandation** : chaque chunk garde son score propre quand disponible (vector/sparse), sinon le score parent (BM25). Le `SearchResult.score` reste le score fusionné du parent.

### 7.3 SourceResolved + chunks

En mode SourceResolved, `chunk` reste le best chunk. Faut-il aussi supporter SourceResolved + tous les chunks ? Cela reviendrait à un 4ème mode. Pour l'instant, non — le client peut combiner SourceResolved manuellement avec les chunk UUIDs s'il en a besoin.

### 7.4 Chunks sans intersection (BM25 title-only match)

Si un BM25 hit match uniquement dans `_title` (pas de chunk intersection), le résultat en mode Detailed aurait `chunks: Some(vec![])` (vide). Le titre est retourné via `data._title`. C'est cohérent.

### 7.5 Pré-remplir `source_entity` sans query SOURCED

On pourrait éviter la query SOURCED en ajoutant `_source_entity` et `_source_uuid` comme colonnes sur le chunk (en plus de `_source_field` qui existe déjà). Cela coûte 2 colonnes STRING supplémentaires mais économise le UNION ALL à chaque recherche Detailed.

**Trade-off** :
- **Pro** : zéro round-trip supplémentaire au search
- **Con** : duplication de données (SOURCED rel + colonnes), schéma plus lourd

**Recommandation** : ajouter `_source_entity` et `_source_uuid` sur le chunk. Le coût en stockage est négligeable, et ça simplifie drastiquement le search Detailed (pas de query SOURCED, tout est déjà dans les colonnes du chunk qu'on fetch de toute façon).

---

## 8. Schéma cible (si on adopte 7.5)

### `{KB}_Index_Chunk` — colonnes ajoutées

```diff
  _source_field STRING       // existe déjà
+ _source_entity STRING      // ex: "File", "Scope"
+ _source_uuid STRING        // UUID de l'entité source
```

À l'ingestion, dans le chunk_data du AggregateProcessor :
```rust
chunk_data.insert("_source_entity", CypherValue::String(source.entity_name.clone()));
chunk_data.insert("_source_uuid", CypherValue::String(source.entity_uuid.clone()));
```

On a déjà `source.entity_name` et `source.entity_uuid` dans `SourceContent`. Coût : 2 lignes.

### Résolution Detailed sans query SOURCED

```cypher
-- Déjà fait dans resolve_and_enrich_chunked / resolve_vector_chunks
-- On ajoute juste c._source_entity et c._source_uuid aux colonnes retournées
MATCH (n:{KB}_Index) WHERE ...
OPTIONAL MATCH (n)-[:HAS_CHUNK]->(c:{KB}_Index_Chunk)
RETURN ..., c._source_entity, c._source_uuid, c._source_field
```

Zéro query supplémentaire. L'attribution est gratuite.

---

## 9. Résumé des changements par fichier

| Fichier | Changement |
|---|---|
| `schema.rs` | Ajouter `_source_entity STRING`, `_source_uuid STRING` à `generate_index_chunk_table_ddl()` |
| `catalog.rs` | AggregateProcessor : insérer `_source_entity` et `_source_uuid` dans chunk_data |
| `search.rs` | Ajouter `ResultMode` enum, `AttributedChunk` struct, `SearchOptions.result_mode` |
| `search.rs` | `SearchResult` : ajouter `chunks: Option<Vec<AttributedChunk>>` |
| `search.rs` | Modifier `resolve_vector_chunks()` : mode Detailed → ne pas collapse, garder tous les chunks |
| `search.rs` | Modifier `search_bm25_chunked()` : mode Detailed → retourner tous les chunks intersecting |
| `search.rs` | Ajouter `resolve_and_enrich_chunked()` colonnes : `c._source_entity`, `c._source_uuid` |
| `catalog.rs` | `search()` : après fusion, si Detailed → collecter/merger chunks + attribuer. Si SourceResolved → résoudre entité source. |

---

## 10. Ordre d'implémentation proposé

```
1. schema.rs : ajouter _source_entity + _source_uuid au chunk DDL
2. catalog.rs : remplir ces colonnes dans AggregateProcessor (2 lignes)
3. search.rs : ResultMode enum + AttributedChunk struct + SearchOptions.result_mode
4. search.rs : SearchResult.chunks field
5. catalog.rs : implémenter resolve_to_source_entities() pour SourceResolved
6. search.rs : modifier resolve_vector_chunks() et search_bm25_chunked() pour Detailed
7. catalog.rs : orchestrer dans search() selon result_mode
8. Tests E2E
```
