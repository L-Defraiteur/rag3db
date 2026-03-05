# 02 — Plan : ingestion des chunks

## Décision d'architecture

**Un document = minimum 1 chunk.** La recherche vector fonctionne uniquement via les chunks. Pas deux chemins (entité vs chunks), un seul.

- Texte court → 1 chunk = le texte complet
- Texte long → N chunks via le Chunker (semantic/markdown/fixed, avec overlap)
- Le vector search cible toujours `{Entity}_Chunk`, jamais `{Entity}`
- BM25 reste sur `{Entity}` (texte complet, pas de coupure mid-word)
- L'embedding entité (`Document.main_embedding`) : à décider si on le garde pour l'explore graph ou si on le supprime

## Embedding contextualisé

Au moment d'embedder un chunk, on préfixe avec le titre du document pour donner du contexte au modèle. Mais le chunk **stocké** en DB garde son texte original et ses offsets intacts.

```
Stocké en DB (_text) :          "discovered in 1942 by researchers at MIT"
Envoyé à l'embedder :           "Machine Learning\n---\ndiscovered in 1942 by researchers at MIT"
```

Les offsets (start_byte, end_byte, start_line, end_line) correspondent toujours au texte original du champ, pas au texte préfixé.

## Pas de déduplication

Si 3 chunks d'un même document matchent dans le top-K, les 3 sont retournés. C'est une config de search qu'on pourra ajouter plus tard en 5 min.

## Ce qui existe déjà

| Composant | Statut |
|---|---|
| `Chunker` (semantic/markdown/fixed, overlap, offsets) | ✅ Prêt |
| `ChunkingConfig` sur `KBConfig` (enabled, max_size, overlap, strategy) | ✅ Prêt |
| `chunked: bool` sur `FieldDef` | ✅ Prêt |
| Schema `{Entity}_Chunk` table + `HAS_CHUNK` rel + HNSW index | ✅ Prêt |
| `SearchResult.chunk: Option<ChunkInfo>` | ✅ Prêt (toujours None) |
| `InsertOp`, `LinkOp`, `EmbedOp` dans la queue | ✅ Prêt |
| Chunk creation dans `create()` | ❌ Manquant |
| Chunk deletion dans `delete()` | ✅ Déjà fait (DETACH DELETE) |
| Chunk re-creation dans `update()` | ❌ Manquant |

## Plan d'ingestion : `create()`

### Flow actuel (sans chunks, embedding sur entité)

```
create("Document", {title: "ML", body: "long text..."})
  ├─ InsertOp(Document, {_uuid, title, body, _content_hash})
  ├─ EmbedOp(Document, "main", ["ML", "long text..."])       ← SUPPRIMÉ
  └─ SparseEmbedOp(Document, "main", ["ML", "long text..."])  ← SUPPRIMÉ
```

### Flow cible

```
create("Document", {title: "ML", body: "long text..."})
  │
  │  1. Chunker
  │  ─────────
  │  Pour chaque champ avec chunked:true (ex: body) :
  │    chunks = Chunker.chunk(body)  →  [Chunk0, Chunk1, Chunk2]
  │    Si body est court : chunks = [Chunk{text: body, index: 0, ...}]
  │
  │  2. Ops entité (inchangé sauf embedding)
  │  ───────────
  ├─ InsertOp(Document, {_uuid: "doc-1", title, body, _content_hash})
  │  Plus d'EmbedOp sur l'entité — seuls les chunks sont embeddes.
  │
  │  3. Ops chunks
  │  ─────────────
  │  Pour chaque chunk :
  │    chunk_uuid = chunk_uuid("doc-1", "body", chunk.index)
  │
  ├─ InsertOp(Document_Chunk, {
  │      _uuid: chunk_uuid,
  │      _parent_uuid: "doc-1",
  │      _parent_field: "body",
  │      _kb_name: "main",
  │      _text: chunk.text,              ← texte brut, offsets originaux
  │      _text_hash: hash(chunk.text),
  │      _index: chunk.index,
  │      _start_char: chunk.start_byte,
  │      _end_char: chunk.end_byte,
  │      _start_line: chunk.start_line,
  │      _end_line: chunk.end_line,
  │      _core_start_char: ...,          ← zone sans overlap (à calculer)
  │      _core_end_char: ...,
  │      _core_start_line: ...,
  │      _core_end_line: ...,
  │  })
  │
  ├─ LinkOp(Document_HAS_CHUNK, "doc-1" → chunk_uuid)
  │
  ├─ EmbedOp(Document_Chunk, chunk_uuid, ["ML\n---\n" + chunk.text])
  │                                        ↑ préfixe titre pour contexte
  ├─ SparseEmbedOp(Document_Chunk, chunk_uuid, ["ML\n---\n" + chunk.text])
  │                                              (si KB.sparse = true)
  │
  │  (répété pour chaque chunk)
```

### Détails importants

**UUID des chunks** : `chunk_uuid(parent_uuid, field_name, index)` — déjà implémenté dans `uuid.rs`. Déterministe, permet l'idempotence.

**Core offsets** : La zone "core" d'un chunk est la partie sans overlap. Pour un chunk qui overlap avec le précédent :
- `_start_char` = début du chunk (avec overlap)
- `_core_start_char` = début après l'overlap
- `_end_char` = fin du chunk (avec overlap)
- `_core_end_char` = fin avant l'overlap avec le suivant

Ces offsets permettent au frontend de highlight la partie "originale" du chunk sans les zones de contexte overlap.

Calcul : pour le chunk `i` avec overlap `O` :
- `core_start = if i == 0 { start } else { start + O/2 }` (approximation, le vrai calcul dépend de l'overlap réel qui peut varier avec les boundaries sémantiques)
- En pratique : `core_start = chunks[i].start_byte`, `core_end = chunks[i].end_byte` si pas d'overlap, sinon le milieu de la zone overlappée avec le voisin.

**Quel champ chunker ?** : tous les champs `content_for` du KB qui ont `chunked: true`. Le titre n'est jamais chunké.

**Multi-champ** : si une entité a `body` (chunked) et `summary` (pas chunked), seul `body` est chunké. `summary` est inclus dans l'embedding entité seulement.

**Multi-KB** : si un champ est `content_for: ["main", "code"]`, les chunks sont créés une seule fois (dans la même table `Document_Chunk`) mais avec `_kb_name` indiquant le KB. L'embedding est fait pour chaque KB séparément.

### Où modifier le code

**`catalog.rs` — `create()`** (lignes 268-345) :
- Après la construction de `full_data` et `entity_ref`
- Pour chaque KB → pour chaque champ chunked → run Chunker
- Générer les InsertOps, LinkOps, EmbedOps pour les chunks
- Les ajouter à `ops` avant `self.queue.enqueue_all(ops)`

**`catalog.rs` — `build_embed_texts()`** n'a pas besoin de changer pour les entités. Pour les chunks, on construit les textes directement dans `create()` avec le préfixe titre.

## Plan d'ingestion : `update()`

### Flow cible

```
update("Document", "doc-1", {body: "new long text..."})
  │
  │  1. Si contenu changé (hash différent) :
  │  ─────────────────────────────────────
  ├─ DELETE anciens chunks :
  │    MATCH (c:Document_Chunk {_parent_uuid: "doc-1"}) DETACH DELETE c
  │
  │  2. Re-chunker + re-insérer (même flow que create)
  │  ──────────────────────────────────────────────────
  ├─ InsertOps pour nouveaux chunks
  ├─ LinkOps pour HAS_CHUNK
  ├─ EmbedOps pour chunks (avec préfixe titre)
```

La logique de delete des chunks existe déjà dans `delete()`. On la réutilise dans `update()`.

## Plan d'ingestion : `delete()`

Déjà fait :
```rust
// catalog.rs lignes 613-621
let chunk_table = format!("{entity_name}_Chunk");
let cypher = format!(
    "MATCH (c:{chunk_table} {{_parent_uuid: $uuid}}) DETACH DELETE c RETURN count(c) AS cnt"
);
```

Rien à changer.

## Décisions tranchées

### 1. Embedding entité : SUPPRIMÉ

L'embedding sur `Document.main_embedding` n'est plus utile. Le search passe par les chunks. L'explore graph traverse les relations à partir de l'entité — il n'utilise pas l'embedding entité.

**Conséquences** :
- `create()` : ne plus enqueue d'`EmbedOp` pour l'entité, seulement pour les chunks
- `schema.rs` : ne plus créer d'index HNSW sur la table entité (garder seulement sur `{Entity}_Chunk`)
- La colonne `{kb}_embedding` sur l'entité peut rester dans le schéma (pas de migration cassante) mais ne sera plus peuplée
- À terme on pourra la retirer du DDL

### 2. Lucivy sur les chunks : NON

BM25 reste sur l'entité parent (texte complet). Un chunk coupé au milieu d'un mot ou d'une phrase casse le `contains`. Pas d'index Lucivy sur `{Entity}_Chunk`.

### 3. Sparse sur les chunks : OUI

Les embedders sparse (BM42/SPLADE) sont basés sur BERT, max ~512 tokens. Les chunks (1500 chars ≈ 300-400 tokens) rentrent parfaitement. Le sparse marche mieux sur des textes courts que sur des longs documents — les poids d'attention sont plus précis. Donc si `KBConfig.sparse = true`, on fait aussi un `SparseEmbedOp` par chunk.

## Fichiers à modifier

| Fichier | Changement |
|---|---|
| `catalog.rs` — `create()` | Chunker + InsertOps/LinkOps/EmbedOps pour chunks |
| `catalog.rs` — `update()` | Delete anciens chunks + re-chunk + re-insert |
| `catalog.rs` | Helper `build_chunk_ops()` pour factoriser la logique |
| `chunker.rs` | Peut-être ajouter les core offsets (zone sans overlap) |

## Note sur la fusion BM25 ↔ vector/sparse

BM25 cherche sur l'entité (texte complet), vector et sparse cherchent sur les chunks. À la fusion (RRF ou autre stratégie), il faudra **mapper les highlights BM25 sur les ranges des chunks**.

Concrètement : si BM25 retourne un highlight à `char_start=4200, char_end=4235` sur le document complet, et que le chunk 3 couvre `[4000..5500]`, alors ce highlight tombe dans le chunk 3. Ce mapping est nécessaire pour que la fusion puisse associer des scores BM25 aux bons chunks et construire un résultat cohérent.

C'est un problème de **search/fusion**, pas d'ingestion — les offsets `_start_char`/`_end_char` stockés sur chaque chunk fournissent déjà toute l'info nécessaire pour faire ce mapping.

## Prochaine étape

Implémenter l'ingestion dans `create()` et `update()`, puis valider que les chunks sont bien insérés en DB avec les bons offsets et embeddings. Le search viendra après.
