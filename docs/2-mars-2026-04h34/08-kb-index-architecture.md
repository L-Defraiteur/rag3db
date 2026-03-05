# 08 — Architecture KB Index : tables de recherche découplées des entités

## Motivation

L'architecture actuelle place les colonnes d'embedding directement sur les tables entité (`File.FileContentKB_embedding`) et les tables chunk (`File_Chunk.FileContentKB_embedding`). Les index FTS/HNSW sont aussi créés sur les tables entité.

**Problèmes :**
1. **Cross-entity KBs impossibles** — une KB partagée (TreeKB : Directory + File) ne peut pas avoir un seul index FTS cohérent (IDF partagé). Fusionner des scores venant d'index séparés produit des résultats incomparables.
2. **Tables entité polluées** — `File` porte `FileContentKB_embedding`, `TreeKB_embedding`, etc. Les colonnes d'embedding n'ont rien à faire dans la table de données.
3. **Code path divergents** — single-entity et multi-entity KBs suivent des chemins complètement différents dans le code (schema, ingestion, search), augmentant la complexité et les bugs potentiels.

**Solution :** chaque Knowledge Base a ses propres tables d'index, possédées par l'entité titre. Les tables entité deviennent du pur stockage de données.

---

## Principe central : l'entité titre possède l'index

**L'entité qui a `titleFor` sur une KB est propriétaire des index entries de cette KB.** Il y a une `{KB}_Index` entry par instance de l'entité titre. Le contenu des entités liées (via relations directes) est **agrégé** dans cette entry au moment du `link()`.

```
  create("Directory")                    link("HAS_FILE", dir, file)
         │                                        │
         ▼                                        ▼
  ┌─────────────────┐                    Update TreeKB_Index entry:
  │ Directory table │                    append File.name + File.absolute_path
  │  name: "src"    │                    to _content, then re-chunk + re-embed
  │  path: "/src/"  │
  └────────┬────────┘
           │ create() détecte titleFor: TreeKB
           ▼
  ┌────────────────────────────┐
  │ TreeKB_Index               │
  │  _title: "src"             │  ← Directory.name (titleFor)
  │  _content: "/src/"         │  ← Directory.absolute_path (contentFor)
  │  _source_uuid: dir_uuid    │     + après link: "\nauth.ts\n/src/auth.ts"
  └────────────┬───────────────┘     (File.name + File.absolute_path)
               │ chunks
  ┌────────────▼───────────────┐
  │ TreeKB_Index_Chunk (×N)    │
  │  _text: "...", embedding   │
  └────────────────────────────┘
```

**Conséquences :**
- Un search sur TreeKB retourne des **Directory** (l'entité titre), pas des File
- Pour trouver les Files d'un résultat, on suit la relation `HAS_FILE` depuis le Directory
- Le `_content` d'une entry TreeKB contient les données agrégées de tous les File enfants
- `_source_entity` est toujours l'entité titre (ici "Directory") — colonne conservée pour le code générique

---

## Tables

### `{KB}_Index` — document complet pour BM25

```sql
CREATE NODE TABLE IF NOT EXISTS {KB}_Index(
    _uuid STRING,              -- hashsafe(kb_name + title_entity + source_uuid)
    _source_entity STRING,     -- entité titre ("Directory", "File", "Scope"...)
    _source_uuid STRING,       -- UUID de l'instance titre
    _title STRING,             -- valeur du champ titleFor
    _content STRING,           -- concaténation des champs contentFor (propres + agrégés)
    {KB}_embedding FLOAT[384],
    {KB}_sparse_indices INT64[],   -- si sparse activé
    {KB}_sparse_weights DOUBLE[],
    PRIMARY KEY(_uuid)
)
```

- **BM25 indexe cette table** (champs `_title` + `_content`). Document entier. IDF partagé sur toute la KB.
- **`_source_entity`** est un filter field Lucivy (toujours le même pour une KB donnée, mais utile en code générique).
- **L'embedding** du document complet est stocké ici (re-ranking, comparaison document-level).

### `{KB}_Index_Chunk` — chunks pour dense/sparse/highlights

```sql
CREATE NODE TABLE IF NOT EXISTS {KB}_Index_Chunk(
    _uuid STRING,
    _parent_uuid STRING,       -- réf vers {KB}_Index._uuid
    _parent_field STRING,      -- "_content"
    _kb_name STRING,
    _text STRING,
    _text_hash STRING,
    _index INT64,
    _start_char INT64,
    _end_char INT64,
    _start_line INT64,
    _end_line INT64,
    _core_start_char INT64,
    _core_end_char INT64,
    _core_start_line INT64,
    _core_end_line INT64,
    {KB}_embedding FLOAT[384],
    {KB}_sparse_indices INT64[],
    {KB}_sparse_weights DOUBLE[],
    PRIMARY KEY(_uuid)
)
```

**Rôles des chunks :**
- **Dense (vector)** : recherche sémantique, résolution chunk → parent index entry → source entity
- **Sparse** : idem
- **BM25 highlight resolution** : le BM25 indexe le document complet, mais les highlight offsets sont résolus vers les chunks pour localiser le match. Même mécanique que `resolve_bm25_to_chunks()`.

**Tous les signaux créent des chunks systématiquement.** Même le BM25, qui n'utilise pas les chunks pour l'indexation, les crée pour la résolution de highlights.

### Relations

```sql
-- Entité titre → Index entry (1:1 par instance)
CREATE REL TABLE IF NOT EXISTS {TitleEntity}_IN_{KB}(
    FROM {TitleEntity} TO {KB}_Index
)

-- Index entry → Chunks
CREATE REL TABLE IF NOT EXISTS {KB}_Index_HAS_CHUNK(
    FROM {KB}_Index TO {KB}_Index_Chunk
)
```

Note : pour les KBs multi-entity, seule l'entité titre a un `_IN_{KB}`. Les entités content sont reliées à l'entité titre via les relations user (`HAS_FILE`, etc.).

### Index de recherche

```sql
-- FTS sur le document complet
CALL CREATE_LUCIVY_INDEX('{KB}_Index', ['_title', '_content'])

-- Vector HNSW sur les chunks
CALL CREATE_VECTOR_INDEX('{KB}_Index_Chunk', '{KB}_Index_Chunk_vec',
    '{KB}_embedding', metric := 'cosine', skip_if_exists := true)

-- Sparse vector sur les chunks (si sparse activé)
CALL CREATE_SPARSE_VECTOR_INDEX('{KB}_Index_Chunk',
    '{KB}_sparse_indices', '{KB}_sparse_weights')
```

---

## Mécanisme d'indexation par signal

| Signal | Table cible | Ce qui est indexé |
|--------|------------|-------------------|
| **BM25** | `{KB}_Index` | Document complet (`_title` + `_content`) |
| **Dense (vector)** | `{KB}_Index_Chunk` | Chunks du content |
| **Sparse** | `{KB}_Index_Chunk` | Chunks du content |

---

## Propriété de l'index et agrégation

### Règle

L'entité qui a `titleFor: {KB}` **possède** l'index de cette KB. Chaque instance de cette entité produit exactement une `{KB}_Index` entry.

Les champs `contentFor: [{KB}]` qui sont **sur l'entité titre elle-même** sont inclus dans `_content` dès le `create()`.

Les champs `contentFor: [{KB}]` qui sont **sur d'autres entités** sont agrégés dans `_content` au moment du `link()`, à condition qu'une **relation directe** existe entre l'entité titre et l'entité content.

### Contrainte : relation directe

Le content d'une KB ne peut venir que d'entités reliées directement (1 hop) à l'entité titre. Raison : à l'ingestion (`link()`), on doit pouvoir identifier immédiatement quelle index entry mettre à jour.

Le validator vérifie déjà cette contrainte : si une KB a des `contentFor` venant d'une entité différente du titre, il exige une relation directe entre elles.

### Quand l'agrégation se déclenche

| Événement | Enqueue |
|-----------|---------|
| `create(TitleEntity, data)` | IndexEntryOp (crée l'entry avec champs propres) + ChunkOp + EmbedOps |
| `link(Rel, title, content)` | LinkOp + **AggregateOp(index_entry_uuid)** |
| `update(TitleEntity, data)` | AggregateOp (re-construit title + content propres + agrégés) |
| `update(ContentEntity, data)` | AggregateOp pour chaque entry titre liée |
| `delete(TitleEntity)` | Suppression index entry + chunks |
| `unlink(Rel, title, content)` | AggregateOp (re-construit sans le content retiré) |
| `delete(ContentEntity)` | AggregateOp pour chaque entry titre liée |

### Batching : un seul rebuild par entry

Le `link()` n'exécute **rien** immédiatement sur l'index. Il enqueue un `AggregateOp` avec l'UUID de l'index entry. Au `drain()`, les AggregateOps sont **dédupliqués par index entry UUID** : même si 100 Files sont linkés au même Directory, l'index entry n'est reconstruite **qu'une seule fois**.

```
link("HAS_FILE", dir1, file1)  → LinkOp + AggregateOp(dir1_treekb_uuid)
link("HAS_FILE", dir1, file2)  → LinkOp + AggregateOp(dir1_treekb_uuid)  ← même UUID
link("HAS_FILE", dir1, file3)  → LinkOp + AggregateOp(dir1_treekb_uuid)  ← même UUID

drain():
  prio 1.0 : InsertOps (entities + index entries + chunks)
  prio 2.0 : LinkOps (toutes les relations)
  prio 2.5 : AggregateOps (DEDUP → 1 seul pour dir1)
  prio 3.0 : EmbedOps (chunks)
```

### Comment AggregateOp reconstruit le `_content`

L'AggregateOp est **idempotent** : il relit l'état actuel du graphe et reconstruit à partir de zéro.

```
1. Query le graphe pour trouver toutes les entités content liées :
   MATCH (d:Directory)-[:HAS_FILE]->(f:File)
   WHERE d._uuid = $source_uuid
   RETURN f.name, f.absolute_path

2. Reconstruit _content :
   _content = concat(
       Directory.absolute_path,              // champs propres
       "\n",
       for each File:                        // champs agrégés
           File.name + "\n" + File.absolute_path
   )

3. UPDATE l'index entry (SET _content = ...)
4. DELETE anciens chunks
5. Re-chunk le nouveau _content → InsertOps chunks
6. Re-embed les chunks → EmbedOps (injectés dans la queue)
```

Les EmbedOps injectés par l'AggregateOp à prio 2.5 seront traités à prio 3.0 (déjà dans le batch courant).

---

## Flow d'ingestion

### `create("Directory", { name: "src", absolute_path: "/repo/src/" })`

```
1. InsertOp         → Directory table (données brutes)
2. IndexEntryOp     → TreeKB_Index {
                        _title: "src",
                        _content: "/repo/src/",   ← seulement les champs propres
                        _source_entity: "Directory",
                        _source_uuid: dir_uuid
                      }
3. LinkOp           → Directory_IN_TreeKB
4. ChunkOp          → chunks de _content dans TreeKB_Index_Chunk
5. EmbedOps         → embeddings sur les chunks
```

### `create("File", { name: "auth.ts", absolute_path: "/repo/src/auth.ts", content: "..." })`

```
1. InsertOp         → File table (données brutes)
2. IndexEntryOp     → FileContentKB_Index {
                        _title: "auth.ts",
                        _content: "export function authenticate...",
                        _source_entity: "File",
                        _source_uuid: file_uuid
                      }
3. LinkOp           → File_IN_FileContentKB
4. ChunkOp          → chunks dans FileContentKB_Index_Chunk
5. EmbedOps         → embeddings sur les chunks

⚠ PAS d'index entry TreeKB — File n'a pas titleFor: TreeKB
```

### `link("HAS_FILE", dir_ref, file_ref)`

```
1. LinkOp           → enqueued (prio 2.0)
2. Détection :      File a contentFor: [TreeKB], et TreeKB.titleFor = Directory
3. AggregateOp      → enqueued pour dir_ref's TreeKB_Index entry (prio 2.5)
                      ⚠ PAS exécuté maintenant — juste enqueued, dédupliqué au drain

Au drain() : l'AggregateOp (dédupliqué si N links vers le même Directory) :
  → MATCH (d:Directory)-[:HAS_FILE]->(f:File) WHERE d._uuid = $uuid
  → reconstruit _content avec tous les Files actuellement liés
  → delete anciens chunks, re-chunk, inject EmbedOps (prio 3.0)
```

### Priorités (OrderedPriority f32)

```
0.0  ChunkOp          (génère les chunks, injecte les ops suivantes)
1.0  InsertOp          (insert entity + index entries + chunks)
2.0  LinkOp            (relie entity → index, index → chunks)
2.5  AggregateOp       (link-triggered: update index entry, re-chunk, re-embed)
3.0  EmbedOp/DualEmbed (embeddings sur chunks)
3.5  EmbedOp touched   (re-embed lors d'un update incrémental)
```

---

## Flow de recherche

### BM25

```
1. QUERY_LUCIVY_INDEX('{KB}_Index', query, fields=['_title', '_content'])
   → retourne des {KB}_Index entries avec scores + highlight offsets
2. Résolution highlights → chunks :
   pour chaque highlight offset dans le document,
   trouver le chunk dans {KB}_Index_Chunk dont [start_char, end_char] contient l'offset
3. Enrichissement : via _source_uuid, MATCH (n:{TitleEntity}) WHERE n._uuid = _source_uuid
   → retourne les champs de l'entité titre
```

### Dense (vector)

```
1. Embed la query
2. HNSW search sur {KB}_Index_Chunk → chunks avec distances
3. Résolution chunk → parent : _parent_uuid → {KB}_Index._source_uuid
4. Enrichissement : joindre les champs de l'entité titre
```

### Sparse

Identique à dense, avec SPARSE_SEARCH.

### Fusion

Identique à l'existant (RRF ou weighted). Tables cibles changent :
- BM25 : `{KB}_Index`
- Dense/Sparse : `{KB}_Index_Chunk`

---

## Exemple concret : Code Domain

### Config

```yaml
entities:
  File:
    fields:
      name:          { type: text, title_for: FileContentKB, content_for: [TreeKB] }
      absolute_path: { type: text, content_for: [TreeKB] }
      content:       { type: text, content_for: [FileContentKB] }
      extension:     { type: string }
      language:      { type: string }
      content_hash:  { type: string }

  Directory:
    fields:
      name:          { type: text, title_for: TreeKB }
      absolute_path: { type: text, content_for: [TreeKB] }
      depth:         { type: int64 }

  Scope:
    fields:
      signature:     { type: text, title_for: ScopeKB }
      content:       { type: text, content_for: [ScopeKB] }
      docstring:     { type: text, content_for: [ScopeKB] }
      # ... autres champs

relations:
  HAS_FILE:    { from: Directory, to: File }
  DEFINED_IN:  { from: Scope, to: File }

knowledge_bases:
  FileContentKB: { signals: [bm25, vector] }
  TreeKB:        { signals: [bm25, vector] }
  ScopeKB:       { signals: [bm25, vector, sparse] }
```

### Tables générées

```sql
-- Entités (données pures, PAS d'embedding)
File(_uuid, _content_hash, name, absolute_path, content, extension, language, content_hash)
Directory(_uuid, _content_hash, name, absolute_path, depth)
Scope(_uuid, _content_hash, signature, content, docstring, ...)

-- KB: FileContentKB (title entity = File)
FileContentKB_Index(_uuid, _source_entity, _source_uuid, _title, _content,
                    FileContentKB_embedding)
FileContentKB_Index_Chunk(_uuid, _parent_uuid, ..., FileContentKB_embedding)
FileContentKB_Index_HAS_CHUNK(FROM FileContentKB_Index TO FileContentKB_Index_Chunk)
File_IN_FileContentKB(FROM File TO FileContentKB_Index)

-- KB: TreeKB (title entity = Directory, content from File via HAS_FILE)
TreeKB_Index(_uuid, _source_entity, _source_uuid, _title, _content,
             TreeKB_embedding)
TreeKB_Index_Chunk(_uuid, ..., TreeKB_embedding)
TreeKB_Index_HAS_CHUNK(FROM TreeKB_Index TO TreeKB_Index_Chunk)
Directory_IN_TreeKB(FROM Directory TO TreeKB_Index)

-- KB: ScopeKB (title entity = Scope)
ScopeKB_Index(_uuid, _source_entity, _source_uuid, _title, _content,
              ScopeKB_embedding, ScopeKB_sparse_indices, ScopeKB_sparse_weights)
ScopeKB_Index_Chunk(_uuid, ..., ScopeKB_embedding, ScopeKB_sparse_indices, ScopeKB_sparse_weights)
ScopeKB_Index_HAS_CHUNK(FROM ScopeKB_Index TO ScopeKB_Index_Chunk)
Scope_IN_ScopeKB(FROM Scope TO ScopeKB_Index)

-- Relations user
HAS_FILE(FROM Directory TO File)
DEFINED_IN(FROM Scope TO File)
```

### Scénario d'ingestion

```
create("Directory", { name: "src", absolute_path: "/repo/src/", depth: 1 })
  → Directory row
  → TreeKB_Index { _title: "src", _content: "/repo/src/" }
  → TreeKB_Index_Chunk (1 chunk, contenu court)
  → embed chunk

create("File", { name: "auth.ts", absolute_path: "/repo/src/auth.ts",
                 content: "export function authenticate() { ... }", ... })
  → File row
  → FileContentKB_Index { _title: "auth.ts", _content: "export function..." }
  → FileContentKB_Index_Chunk (N chunks du code)
  → embed chunks
  ⚠ PAS de TreeKB entry

link("HAS_FILE", dir_ref, file_ref)
  → HAS_FILE relation
  → TreeKB_Index entry du Directory mise à jour :
    _content: "/repo/src/\nauth.ts\n/repo/src/auth.ts"
  → re-chunk + re-embed

search("TreeKB", "auth")
  → BM25 hit sur le Directory (car _content contient "auth.ts")
  → résultat : Directory { name: "src", absolute_path: "/repo/src/" }
  → pour trouver les fichiers : follow HAS_FILE
```

---

## Impact sur le code existant

### `schema.rs`

- `generate_node_table_ddl()` : **retirer toutes** les colonnes embedding. Les entity tables ne portent plus d'embedding.
- `generate_chunk_table_ddl()` / `generate_chunk_rel_ddl()` : **supprimés** — remplacés par `generate_index_chunk_table_ddl(kb_name)` et `generate_index_chunk_rel_ddl(kb_name)`
- `generate_index_table_ddl()` : déjà ajouté, à utiliser pour **toutes** les KBs
- `generate_index_rel_ddl()` : déjà ajouté, simplifié — une seule rel `{TitleEntity}_IN_{KB}`
- `entity_has_chunks()` : **supprimé** — le chunking est toujours sur les index entries, pas sur les entités
- `collect_multi_entity_kbs()` : **supprimé** — plus de distinction single/multi, même code path
- `generate_full_schema()` : itérer sur les KBs (au lieu des entités) pour générer index tables + chunks + rels + indexes

### `catalog.rs`

- `create()` : pour chaque KB où cette entité a `titleFor`, créer l'index entry + chunks
- `link()` : détecter si l'entité content a des `contentFor` pour une KB possédée par l'autre endpoint → agrégation
- `compute_chunk_ops()` : les chunks vont dans `{KB}_Index_Chunk`, parent = `{KB}_Index._uuid`
- `search()` : `entity = "{KB}_Index"` (BM25) / `"{KB}_Index_Chunk"` (vector/sparse). `_source_uuid` pour résolution.
- `update()` / `delete()` : propager aux index entries possédées + re-agrégation si nécessaire

### `ops.rs`

- Nouveau `AggregateOp` (ou `IndexUpdateOp`) : déclenché par `link()`, reconstruit le `_content` agrégé et re-chunk/re-embed
- `ChunkOp` cible `{KB}_Index_Chunk` au lieu de `{Entity}_Chunk`

### Tables supprimées

- `{Entity}_Chunk` — les chunks vivent dans `{KB}_Index_Chunk`
- `{Entity}_HAS_CHUNK` — remplacé par `{KB}_Index_HAS_CHUNK`
- Colonnes `{KB}_embedding` sur entity tables
- `{Entity}_IN_{KB}` pour les entités content (seule l'entité titre a un `_IN_{KB}`)

### Tables ajoutées (par KB)

- `{KB}_Index`
- `{KB}_Index_Chunk`
- `{KB}_Index_HAS_CHUNK`
- `{TitleEntity}_IN_{KB}`

---

## Résumé des règles

1. **Les tables entité sont du pur stockage** — pas d'embedding, pas d'index de recherche
2. **Chaque KB a ses propres tables** — `{KB}_Index` + `{KB}_Index_Chunk`, uniformément
3. **L'entité titre possède l'index** — une entry par instance, créée au `create()`
4. **Le contenu des entités liées est agrégé au `link()`** — via relations directes uniquement
5. **Tous les signaux créent des chunks** — BM25 pour la résolution de highlights, dense/sparse pour l'ingestion des embeddings
6. **BM25 indexe le document complet** (`{KB}_Index`), jamais les chunks
7. **Dense et sparse indexent les chunks** (`{KB}_Index_Chunk`)

---

## État du code

- **Phase 0a (float priority) : FAIT** — `OrderedPriority(f32)` dans ops.rs, queue.rs, persistence.rs, cypher_persistence.rs. 352 tests passent.
- **schema.rs : intact** — aucune modification Phase 0b. La ré-implémentation partira du code existant (embeddings sur entity tables, `{Entity}_Chunk`, etc.) et le remplacera par l'architecture KB Index décrite ci-dessus.
- Le plan d'implémentation sera dans un doc séparé.
