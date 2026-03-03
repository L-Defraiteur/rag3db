# 13 — Tests E2E Phase 0b (3 mars 2026)

## Objectif

Valider la chaîne complète de Phase 0b avec de vrais embeddings et une vraie DB Kuzu. Les tests unitaires actuels (362) utilisent des mocks (MockConnection, MockEmbedder) — ils ne valident pas que le Cypher exécuté est correct, que les chunks sont réellement créés, ni que la résolution highlight→chunk fonctionne de bout en bout.

---

## Infrastructure de test nécessaire

### Config multi-entity (TreeKB)

```yaml
entities:
  Directory:
    hashsafe: [absolute_path]
    fields:
      name:          { type: text, title_for: TreeKB }
      absolute_path: { type: text, content_for: [TreeKB] }
  File:
    hashsafe: [absolute_path]
    fields:
      name:          { type: text, title_for: FileKB, content_for: [TreeKB] }
      absolute_path: { type: text, content_for: [TreeKB] }
      body:          { type: text, content_for: [FileKB], chunked: true }

relations:
  HAS_FILE: { from: Directory, to: File }

knowledge_bases:
  TreeKB:  { signals: [bm25] }           # multi-entity, pas de vector
  FileKB:  { signals: [bm25, vector] }   # single-entity, avec vector + chunks
```

**Pourquoi cette config :**
- `TreeKB` = multi-entity (Directory + File), BM25 only → teste `_content_offset`, highlight resolution, SOURCED rels
- `FileKB` = single-entity, vector + BM25, chunked body → teste chunks, embeddings, standard flow

### Embedder

Vrai embedder ou mock déterministe retournant des vecteurs distincts par texte (ex: hash du texte → vecteur). L'important c'est que les vecteurs soient distincts pour que le HNSW retourne le bon résultat.

### DB

Vraie instance Kuzu (in-memory ou temp dir). `Catalog::initialize()` doit créer le schema complet.

---

## Test 1 : Ingestion complète + schema validation

**`test_e2e_ingest_and_schema`**

1. `initialize()` — pas d'erreur
2. Vérifier que les tables existent :
   - `Directory`, `File` (entity tables)
   - `HAS_FILE` (user rel)
   - `TreeKB_Index`, `TreeKB_Index_Chunk` (multi-entity KB)
   - `FileKB_Index`, `FileKB_Index_Chunk` (single-entity KB)
   - `Directory_IN_TreeKB`, `File_SOURCED_TreeKB`, `Directory_SOURCED_TreeKB`
   - `File_IN_FileKB`, `File_SOURCED_FileKB`
   - `TreeKB_Index_HAS_CHUNK`, `FileKB_Index_HAS_CHUNK`
3. `create("Directory", { name: "src", absolute_path: "/repo/src/" })`
4. `create("File", { name: "auth.ts", absolute_path: "/repo/src/auth.ts", body: "export function authenticate(req: Request) { ... long body ... }" })`
5. `link("HAS_FILE", dir_ref, file_ref)`
6. `drain()` — 0 erreurs
7. Vérifier :
   - 1 `TreeKB_Index` entry (title entity = Directory)
   - Ses `_title` = "src", `_content` contient "/repo/src/" et "auth.ts" et "/repo/src/auth.ts"
   - `_content_hash` != "" (hash sentinel remplacé par le vrai hash)
   - N chunks `TreeKB_Index_Chunk` avec `_content_offset` correct
   - SOURCED rels : `Directory_SOURCED_TreeKB` et `File_SOURCED_TreeKB` existent
   - 1 `FileKB_Index` entry, title = "auth.ts"
   - N chunks `FileKB_Index_Chunk` (body chunké)
   - Embeddings non nuls sur `FileKB_Index_Chunk`

---

## Test 2 : BM25 highlight → chunk resolution (multi-entity)

**`test_e2e_bm25_highlight_to_chunk_multi_entity`**

Après ingestion du test 1 :

1. `search("TreeKB", "auth")` avec signal BM25
2. Le résultat doit contenir des chunks avec :
   - `chunk.start_char` / `chunk.end_char` relatifs au champ source (pas au `_content` concaténé)
   - Le texte `chunk.text` doit contenir "auth" (substring du champ source)
3. Vérifier que les chunks proviennent bien du File (la recherche "auth" matche "auth.ts" et "/repo/src/auth.ts" qui sont des champs du File)
4. Vérifier que `chunk.start_char` permet de retrouver le texte correct :
   ```
   source_field_value[chunk.start_char..chunk.end_char] == chunk.text
   ```

**Ce que ça valide :** Bug A (clé `"_content"` au lieu de `parent_field`), Bug B (`_content_offset` translation), et que les offsets sont cohérents.

---

## Test 3 : BM25 highlight → chunk resolution (single-entity)

**`test_e2e_bm25_highlight_to_chunk_single_entity`**

1. Créer un File avec un body long (>1500 chars, plusieurs chunks) contenant le mot "authentication" à une position connue
2. `drain()`
3. `search("FileKB", "authentication")` avec signal BM25
4. Vérifier :
   - Au moins 1 chunk retourné
   - Le chunk qui contient "authentication" a des offsets corrects
   - `body[chunk.start_char..chunk.end_char]` contient "authentication"
   - `chunk.start_line` / `chunk.end_line` correspondent aux vraies lignes du body

---

## Test 4 : Vector search + chunk-to-source entity resolution

**`test_e2e_vector_chunk_to_source_entity`**

1. Créer 2 Files avec des bodies distincts :
   - File A : body sur l'authentification
   - File B : body sur le logging
2. `drain()`
3. `search("FileKB", "authenticate user login")` avec signal vector
4. Vérifier :
   - File A apparaît en premier (plus pertinent sémantiquement)
   - Le résultat a `chunk` non null, avec `uuid`, `text`, et offsets corrects
   - Le `uuid` dans le résultat correspond au File A (résolution parent)
5. Vérifier la résolution via SOURCED :
   ```cypher
   MATCH (f:File)-[:File_SOURCED_FileKB]->(c:FileKB_Index_Chunk {_uuid: $chunk_uuid})
   RETURN f._uuid, f.name
   ```
   Doit retourner File A

---

## Test 5 : `_content_offset` vérifié arithmétiquement

**`test_e2e_content_offset_arithmetic`**

1. Créer Directory "src" (absolute_path: "/app/src/")
2. Créer File "main.rs" (name: "main.rs", absolute_path: "/app/src/main.rs")
3. `link("HAS_FILE", dir, file)`
4. `drain()`
5. Query `TreeKB_Index` pour récupérer `_content` concaténé
6. Query `TreeKB_Index_Chunk` pour récupérer tous les chunks avec `_content_offset`, `_start_char`, `_end_char`, `_source_field`
7. Pour chaque chunk, vérifier :
   ```
   _content[chunk._content_offset + chunk._start_char .. chunk._content_offset + chunk._end_char]
   == chunk._text
   ```
   (aux whitespace de trimming près si le chunker trim)

**Ce que ça valide :** `_content_offset` est calculé correctement par l'AggregateProcessor, les offsets des chunks sont cohérents avec le `_content` concaténé.

---

## Test 6 : Delete contentFor-only entity → re-aggregate

**`test_e2e_delete_content_for_only`**

1. Créer Directory "src" + File "auth.ts" + link HAS_FILE
2. `drain()` → TreeKB_Index a `_content` contenant les champs de File
3. `delete("File", file_uuid)`
4. `drain()` — l'AggregateOp enqueué doit re-aggregate l'index entry du Directory
5. Vérifier :
   - `TreeKB_Index._content` ne contient **plus** "auth.ts" ni "/repo/src/auth.ts"
   - Les chunks `TreeKB_Index_Chunk` ne référencent **plus** le File (pas de `File_SOURCED_TreeKB` pour ces chunks)
   - `_content_hash` a changé
6. `search("TreeKB", "auth")` → 0 résultats (le mot "auth" a disparu)

---

## Test 7 : Update contentFor-only entity → re-aggregate

**`test_e2e_update_content_for_only`**

1. Créer Directory "src" + File "auth.ts" (name: "auth.ts") + link HAS_FILE
2. `drain()`
3. `update("File", file_uuid, { name: "login.ts", absolute_path: "/repo/src/login.ts" })`
4. `drain()`
5. Vérifier :
   - `TreeKB_Index._content` contient "login.ts" et "/repo/src/login.ts"
   - `TreeKB_Index._content` ne contient **plus** "auth.ts"
   - Chunks mis à jour
6. `search("TreeKB", "login")` → trouve le résultat
7. `search("TreeKB", "auth")` → 0 résultats

---

## Test 8 : Title truncation (`title_max_chars`)

**`test_e2e_title_truncation`**

1. Config avec `chunking.title_max_chars: 20`
2. Créer un File avec `name` = "a]".repeat(50) (100 chars, dépasse le max)
3. `drain()`
4. Vérifier :
   - `FileKB_Index._title` fait ≤ 20 chars
   - Les chunks ont un `embed_text` dont le préfixe titre fait ≤ 20 chars
   - Les offsets chunks ne sont pas affectés (relatifs au body, pas au titre)

---

## Test 9 : SOURCED rels multi-entity correctes

**`test_e2e_sourced_rels_multi_entity`**

1. Créer Directory + 2 Files, link les deux
2. `drain()`
3. Vérifier les SOURCED rels :
   ```cypher
   MATCH (d:Directory)-[:Directory_SOURCED_TreeKB]->(c:TreeKB_Index_Chunk)
   RETURN d.name, c._source_field, count(c)
   ```
   → chunks du Directory uniquement (absolute_path)
   ```cypher
   MATCH (f:File)-[:File_SOURCED_TreeKB]->(c:TreeKB_Index_Chunk)
   RETURN f.name, c._source_field, count(c)
   ```
   → chunks des Files (name + absolute_path de chaque File)
4. Vérifier qu'aucun chunk n'a de SOURCED vers la mauvaise entité

---

## Test 10 : Drain idempotent (hash unchanged → skip)

**`test_e2e_aggregate_skip_unchanged`**

1. Créer Directory + File + link
2. `drain()` — premier aggregate
3. Noter le `_content_hash`
4. Enqueue manuellement un `AggregateOp` pour la même index entry
5. `drain()` — deuxième aggregate
6. Vérifier que `_content_hash` est identique (l'AggregateProcessor a skip le reprocessing)

---

## Résumé

| # | Test | Valide |
|---|------|--------|
| 1 | Ingestion + schema | Tables, rels, content_hash, chunks, SOURCED, embeddings |
| 2 | BM25 highlight → chunk (multi) | Bug A + B fix, `_content_offset` translation |
| 3 | BM25 highlight → chunk (single) | Offsets corrects sur single-entity KB |
| 4 | Vector → source entity | Chunk resolution, SOURCED traversal |
| 5 | `_content_offset` arithmétique | Calcul correct de l'offset dans `_content` concaténé |
| 6 | Delete contentFor-only | Re-aggregate sans le contenu supprimé |
| 7 | Update contentFor-only | Re-aggregate avec le nouveau contenu |
| 8 | Title truncation | `title_max_chars` appliqué, offsets non affectés |
| 9 | SOURCED rels multi-entity | Chaque chunk SOURCED vers la bonne entité |
| 10 | Aggregate idempotent | Hash unchanged → skip |

## Fichiers de test

Les tests E2E nécessitent une vraie DB Kuzu + un embedder fonctionnel. Deux options :

**Option A : Tests internes au crate** (`catalog.rs` test module)
- Avantage : même fichier, accès aux internals
- Inconvénient : nécessite de builder la DB Kuzu dans le test

**Option B : Tests d'intégration** (`tests/e2e_phase0b.rs`)
- Avantage : séparation claire, test depuis l'API publique
- Inconvénient : besoin d'exporter plus de types

**Recommandation :** Option A pour le moment (comme les tests existants), avec un helper `make_real_catalog()` qui crée une DB Kuzu in-memory.
