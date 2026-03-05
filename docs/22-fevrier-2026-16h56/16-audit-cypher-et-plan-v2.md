# 16 — Audit Cypher & plan V2 rag3weaver

## Contexte

Apres la V1 du sparse index (doc 15), on fait un audit complet du code rag3weaver pour identifier tous les anti-patterns de performance, notamment les boucles contenant des queries Cypher. Ce document inventorie les problemes et les pistes de resolution AVANT implementation.

---

## Inventaire des anti-patterns Cypher

### CRITIQUE : Boucles avec queries

| # | Fichier | Fonction | Pattern | Queries/appel | Fix propose |
|---|---------|----------|---------|---------------|-------------|
| 1 | `catalog.rs:1432-1464` | `EmbedProcessor::process()` | `for (work, vector)` → 1 SET par embedding | N (batch_size=32 max) | UNWIND groupé par entity |
| 2 | `catalog.rs:1531-1563` | `SparseEmbedProcessor::process()` | `for (work, sv)` → 1 SET par sparse vector | N (batch_size=32 max) | UNWIND groupé par entity |
| 3 | `search.rs:586-653` | `explore_bfs()` | `for uuid × for rel` → 1 MATCH par (uuid, rel) | O(frontier × relations) par niveau | Batch frontier en 1 query/direction |
| 4 | `catalog.rs:1127-1166` | `rebuild_sparse_index()` | `for entity` → 1 MATCH par type d'entité | N_entities par KB | UNION ALL en 1 query |

### MOYEN : Queries séquentielles combinables

| # | Fichier | Fonction | Pattern | Queries | Fix propose |
|---|---------|----------|---------|---------|-------------|
| 5 | `catalog.rs:491-542` | `update()` | 1 MATCH pour hash check + 1 MATCH pour SET | 2 | Combiner en 1 : SET + RETURN old_hash |
| 6 | `catalog.rs:612-641` | `delete()` | 1 DELETE chunks + 1 DETACH DELETE entity | 2 | Tenter OPTIONAL MATCH chain (Kuzu compat ?) |

### BAS : Allocations inutiles

| # | Fichier | Fonction | Pattern | Impact |
|---|---------|----------|---------|--------|
| 7 | `catalog.rs:1416` | `EmbedProcessor` | `texts.clone()` pour collect en Vec | N strings copiees par batch |
| 8 | `catalog.rs:1515` | `SparseEmbedProcessor` | `texts.clone()` idem | N strings copiees par batch |
| 9 | `catalog.rs` create/update | EmbedOp + SparseEmbedOp | `.clone()` des textes entre les deux ops | 2x memoire textes |

---

## Detail des problemes et pistes

### 1. EmbedProcessor — N queries en boucle

**Code actuel** (simplifie) :
```rust
// Phase 3 : 1 query PAR embedding
for (work, vector) in works.iter().zip(vectors.iter()) {
    let cypher = format!(
        "MATCH (n:{} {{_uuid: $uuid}}) SET n.{} = $embedding",
        work.entity_name, work.embedding_col
    );
    self.conn.execute_with_params(&cypher, &params).await?;
}
```

**Piste** : Grouper par `entity_name`, une seule query UNWIND par groupe :
```cypher
UNWIND $items AS item
MATCH (n:Document {_uuid: item.uuid})
SET n.main_embedding = item.embedding
```

**Question ouverte** : Est-ce que Kuzu supporte `SET n[item.col] = item.val` (propriete dynamique) ? Si non, on doit generer 1 query par colonne d'embedding au lieu de par entite (toujours mieux que par row).

### 2. SparseEmbedProcessor — meme probleme

Meme pattern que #1 mais avec 2 colonnes (indices + weights). Meme fix UNWIND.

### 3. explore_bfs — explosion quadratique

**Code actuel** :
```rust
for uuid in &frontier {                    // N uuids
    for rel in outgoing_relations {        // M relations
        explore_relation(conn, uuid, rel, "outgoing").await?;  // 1 query
    }
    for rel in incoming_relations {        // P relations
        explore_relation(conn, uuid, rel, "incoming").await?;  // 1 query
    }
}
// Total : N × (M + P) queries par niveau de BFS
```

**Deux pistes a verifier sur un vrai dataset Kuzu** :

Piste A — `WHERE n._uuid IN $uuids` :
```cypher
MATCH (n)-[r:REL1|REL2|REL3]->(m)
WHERE n._uuid IN $uuids
RETURN n._uuid AS from_uuid, type(r) AS rel, m._uuid, label(m), m
```
- Avantage : potentiellement optimise par le planner (index scan sur _uuid)
- Risque : Kuzu pourrait faire un full scan si pas d'index sur IN

Piste B — `UNWIND` :
```cypher
UNWIND $uuids AS uid
MATCH (n {_uuid: uid})-[r:REL1|REL2|REL3]->(m)
RETURN uid AS from_uuid, type(r) AS rel, m._uuid, label(m), m
```
- Avantage : explicite, chaque uid est un lookup
- Risque : potentiellement N lookups sequentiels

**A verifier** : `type(r)` est-il supporte dans Kuzu ? `MATCH -[r:A|B|C]->` multi-label est-il supporte ? Si non, fallback sur N queries (1 par relation type) mais batch les uuids.

### 4. rebuild_sparse_index — N queries par entite

**Code actuel** :
```rust
for entity in &kb_meta.entities {
    let cypher = format!(
        "MATCH (n:{entity}) WHERE n.{indices_col} IS NOT NULL ..."
    );
    conn.execute(&cypher).await?;
}
```

**Fix simple** : UNION ALL
```cypher
MATCH (n:Document) WHERE n.main_sparse_indices IS NOT NULL
RETURN n._uuid, n.main_sparse_indices, n.main_sparse_weights
UNION ALL
MATCH (n:Article) WHERE n.main_sparse_indices IS NOT NULL
RETURN n._uuid, n.main_sparse_indices, n.main_sparse_weights
```

Pas de question ouverte — UNION ALL est standard Cypher, Kuzu le supporte.

### 5. update() — 2 round-trips

**Code actuel** : 1 MATCH pour lire `_content_hash`, comparer, puis 1 MATCH SET pour ecrire.

**Fix** :
```cypher
MATCH (n:Document {_uuid: $uuid})
WITH n, n._content_hash AS old_hash
SET n.title = $title, n.body = $body, n._content_hash = $new_hash
RETURN old_hash
```

Le code Rust compare ensuite `old_hash != new_hash` pour decider de re-embedder.

**Question** : Kuzu gere-t-il `WITH` + `SET` + `RETURN` dans la meme query ? A verifier. Sinon, on garde 2 queries (impact faible car update est appele 1 fois par entite).

### 6. delete() — 2 round-trips

**Code actuel** : 1 query delete chunks + count, 1 query DETACH DELETE entity.

**Kuzu ne supporte PAS `FOREACH`**. Piste : OPTIONAL MATCH chain.

```cypher
OPTIONAL MATCH (c:Document_Chunk {_parent_uuid: $uuid})
WITH count(c) AS chunk_count
OPTIONAL MATCH (c2:Document_Chunk {_parent_uuid: $uuid})
DETACH DELETE c2
WITH chunk_count
MATCH (n:Document {_uuid: $uuid})
DETACH DELETE n
RETURN chunk_count
```

**Risque** : Kuzu pourrait ne pas supporter DELETE + WITH chain. A tester. Si ca echoue, garder 2 queries — delete est rare (operation ponctuelle, pas batch).

---

## Concessions V1 — plan de resolution

| Concession (doc 15) | Resolution V2 | Priorite |
|---|---|---|
| #1 Rebuild complet apres drain | Arc<RwLock<>> + update incremental dans processor | HAUTE |
| #2 Pas d'embedder sparse reel | Hors scope V2 — BM42 candle est un projet separe | - |
| #3 Boost + sparse = RRF | Garder tel quel — RRF est meilleur pour 3 signaux | - |
| #4 Pas de compression postings | Garder — premature avant 1M+ docs | - |
| #5 Pas de cache sparse | Ajouter FIFO cache meme pattern que dense | MOYENNE |
| #6 INT64/DOUBLE colonnes | Contrainte Kuzu — rien a faire | - |
| #7 Sparse ignore filtres | Ajouter `allowed_uuids` sur search_sparse | MOYENNE |
| #8 Textes clones | Arc<Vec<String>> partage entre ops | BASSE |

---

## Questions ouvertes a verifier avant implementation

1. **UNWIND + SET dynamique** : Kuzu supporte-t-il `SET n[item.col] = item.val` (propriete dynamique via variable) ? Si non, on genere des queries par colonne au lieu de par row.

2. **Multi-label relationship** : Kuzu supporte-t-il `MATCH (n)-[r:A|B|C]->(m)` ? Et `type(r)` retourne-t-il le type de la relation ?

3. **WHERE IN + index** : Kuzu utilise-t-il l'index primaire sur `_uuid` pour `WHERE n._uuid IN [...]` ?

4. **WITH + SET + RETURN** : Kuzu supporte-t-il un pipeline `MATCH → WITH → SET → RETURN` dans une seule query ?

5. **DELETE dans un WITH chain** : Kuzu supporte-t-il `DETACH DELETE` suivi de `WITH` pour continuer la query ?

---

## Table recapitulative : avant → apres

| Operation | Avant (queries) | Apres (queries) | Gain |
|---|---|---|---|
| Embed 32 entites | 32 | 1-2 (par entity type) | ~16-32x |
| Sparse embed 32 entites | 32 | 1-2 (par entity type) | ~16-32x |
| BFS niveau (10 nodes × 5 rels) | 50 | 2 (outgoing + incoming) | ~25x |
| Rebuild sparse (3 entity types) | 3 | 1 (UNION ALL) | 3x |
| Update 1 entite | 2 | 1 | 2x |
| Delete 1 entite + chunks | 2 | 1 (si supporte) | 2x |
| Drain + sparse refresh | N (full rebuild) | 0 (incremental) | infini |

---

## Reflexion strategique : on maitrise la stack complete

### Le constat

On controle tout : rag3db (fork Kuzu), l'extension lucivy_fts (C++/Rust), et rag3weaver (Rust). Ca signifie qu'au lieu de contourner les limitations Kuzu cote client, **on peut penser les optimisations en amont, cote DB**.

### Etat des lieux des IDs

Il y a actuellement **deux systemes d'identification** en parallele :

| Systeme | Type | Ou | Usage |
|---|---|---|---|
| `_uuid` | STRING (PRIMARY KEY) | Colonnes des tables, rag3weaver | Toutes les queries Cypher : MATCH, WHERE, SET |
| `table_id:offset` | u64 interne | Kuzu storage layer | Lucivy `allowed_ids`, internal node refs |

**Aujourd'hui** :
- `_uuid` est PRIMARY KEY → hash lookup O(1) dans Kuzu pour les MATCH simples
- Lucivy utilise les offsets internes (u64) pour `allowed_ids` — deja optimal
- Mais les UNWIND/batch ops passent par `_uuid` STRING → hash lookup par row

### Piste : exposer les internal IDs pour le batch path

L'idee : quand on fait un UNWIND sur N items, chaque `MATCH (n {_uuid: item.uuid})` fait un hash lookup. C'est O(1) par lookup, mais avec overhead de hashing string. Si on avait les internal offsets, on ferait du direct addressing O(1) sans hash.

**Concretement** :

```
Aujourd'hui (string path) :
  UNWIND $items AS item
  MATCH (n:Doc {_uuid: item.uuid})  ← hash lookup par string, N fois
  SET n.embedding = item.emb

Avec internal IDs (offset path) :
  UNWIND $items AS item
  MATCH (n:Doc) WHERE ID(n) = item.node_id  ← direct offset, O(1) sans hash
  SET n.embedding = item.emb
```

### Ce qu'il faudrait cote rag3db

1. **Retourner l'internal ID dans les resultats** : Quand on fait un INSERT ou MATCH, Kuzu pourrait retourner `ID(n)` (l'offset). rag3weaver le stockerait en memoire a cote du _uuid.

2. **Supporter `WHERE ID(n) = $offset`** : Kuzu devrait accepter un lookup par offset interne. Ca existe peut-etre deja (a verifier dans le code Kuzu — `ID()` function).

3. **Batch SET par offsets** : Une extension C++ `BATCH_SET_BY_ID(table, [(offset, col, value), ...])` qui ecrit directement dans le storage sans passer par le query planner. Ce serait le "fast path" pour l'EmbedProcessor.

### Ce qu'il faudrait cote rag3weaver

1. **Cache uuid → internal_id** : Apres un INSERT ou MATCH, stocker le mapping `_uuid → offset` dans le Catalog. Ca evite de refaire le hash lookup lors des SET suivants.

2. **Dual-path dans les processors** : Si on a l'offset, utiliser le fast path. Sinon, fallback sur _uuid.

### Estimation d'impact

| Operation | Actuel (_uuid string) | Avec internal IDs | Gain |
|---|---|---|---|
| Hash lookup par MATCH | O(1) hash | O(1) direct | ~2-3x (pas de hash) |
| UNWIND 32 SET | 32 hash lookups | 32 direct lookups | ~2-3x |
| explore_bfs 10 nodes | 10 hash lookups | 10 direct lookups | ~2-3x |

Le gain est modeste par item (2-3x), mais **se multiplie par le nombre d'items dans un batch**. Et surtout, ca ouvre la porte a un `BATCH_SET_BY_ID` qui bypasserait entierement le query planner Cypher.

### Questions a explorer

1. **`ID(n)` existe-t-il dans Kuzu ?** — Est-ce qu'on peut recuperer et utiliser l'internal ID dans Cypher ?
2. **Stabilite des offsets** : Les offsets internes changent-ils apres un DELETE ? (compaction) Si oui, le cache uuid→offset est invalide apres un delete.
3. **BATCH_SET_BY_ID** : Quel effort pour creer cette extension C++ ? On a deja le pattern avec lucivy_fts.
4. **Alternative : virtual column** : Kuzu pourrait exposer l'offset comme une virtual column `_internal_id` sur chaque table, accessible en Cypher sans extension.

### Exploration approfondie : ce que Kuzu fait deja en interne

Apres analyse du code source rag3db :

**1. `ID(n)` existe dans Kuzu Cypher** (`src/function/pattern/id_function.cpp`)
- Retourne `internalID_t { offset: u64, tableID: u64 }`
- On peut faire `MATCH (n:Doc {_uuid: $uuid}) RETURN ID(n)` pour recuperer l'offset
- L'offset est l'adresse physique dans le storage — jamais realloue

**2. Les offsets sont STABLES apres DELETE** (`src/storage/table/node_group.cpp:370-379`)
- Delete = tombstone (flag MVCC), l'offset reste reserve a jamais
- Pas de compaction, pas de reuse — **parfait pour le caching**
- Un cache `uuid → offset` ne s'invalide JAMAIS (sauf si on truncate/drop la table)

**3. PrimaryKeyIndex mappe deja `_uuid → offset`** (`src/storage/index/hash_index.h`)
- Hash index on-disk, MVCC-aware
- Quand on fait `MATCH (n:Doc {_uuid: $uuid})`, le planner utilise deja ce hash index
- Le lookup est deja O(1) via hash — la question est : quel est le VRAI overhead ?

**4. Le vrai overhead n'est peut-etre pas le lookup**
- Pour un MATCH simple avec PRIMARY KEY, Kuzu fait : parse Cypher → plan → hash lookup → fetch → return
- Le hash lookup est O(1), mais le **parse + plan Cypher** a un cout fixe par query
- Avec UNWIND, le parse + plan se fait UNE FOIS pour N rows → deja optimal
- Le gain d'un fast path par offset serait : **eliminer le hash (string → u64)** — gain modeste par row

### Trois options architecturales cote rag3db

**Option A : Extension "uuid_offset_index" (style lucivy_fts)**

- Nouvelle extension C++ qui registre un IndexType
- Auto-hooking sur INSERT/DELETE/UPDATE du NodeTable
- Maintient un `unordered_map<string, offset_t>` en memoire + persiste sur disque
- Expose une fonction `CALL RESOLVE_UUID('Doc', $uuid) RETURN offset`
- **Effort** : ~500 lignes C++ (copier la structure lucivy_fts)
- **Gain** : bypass le hash index pour les batch ops, lookup en memoire pure

**Option B : Fonction Cypher `BATCH_SET_BY_OFFSET`**

- Extension C++ qui expose une TableFunction
- Recoit un tableau de `[(offset, col_name, value), ...]`
- Ecrit directement dans le storage via `NodeTable::update()` sans passer par Cypher
- **Effort** : ~300 lignes C++ mais touche le storage layer directement
- **Gain** : elimine COMPLETEMENT le planner Cypher pour les batch SET

**Option C : Cache uuid→offset cote rag3weaver (Rust)**

- Apres chaque INSERT, recuperer `ID(n)` et cacher le mapping
- Pour les MATCH suivants, construire le Cypher avec `WHERE ID(n) = $internal_id`
- **Effort** : ~50 lignes Rust, zero modif rag3db
- **Gain** : elimine le hash lookup string, mais garde le planner Cypher
- **Risque** : `WHERE ID(n) = ...` est-il optimise par le planner ? A verifier.

### Analyse : ou est le vrai bottleneck ?

```
Cout par query Cypher :
  1. Parse Cypher string         ~10 us (fixe)
  2. Plan (optimiseur)           ~20 us (fixe)
  3. Hash lookup PRIMARY KEY     ~1 us  (par row)
  4. Fetch row from storage      ~2 us  (par row)
  5. Serialize result            ~5 us  (par row)
  Total single MATCH :           ~38 us

Cout UNWIND N rows :
  1. Parse + Plan                ~30 us (fixe, 1x)
  2. N × (hash + fetch + ser)   ~8 us × N
  Total UNWIND 32 rows :         ~286 us

Vs 32 queries separees :         ~1216 us (32 × 38)
```

**Le UNWIND donne deja un gain de ~4x** sur 32 rows (1 parse au lieu de 32).
Le gain supplementaire d'un fast path par offset eliminerait le hash (~1us/row) = ~32us sur 32 rows.
**C'est marginal** compare au gain UNWIND.

### Verdict revise

| Approche | Effort | Gain | Priorite |
|---|---|---|---|
| **UNWIND batch** (Cypher standard) | Faible (Rust seul) | ~4x vs N queries | **V2 — faire maintenant** |
| **Option C** (cache uuid→offset Rust) | Faible | ~10-15% en plus | **V2.5 — facile a ajouter** |
| **Option A** (extension uuid_offset) | Moyen (C++) | ~20-30% en plus | **V3 — si bench montre bottleneck** |
| **Option B** (BATCH_SET_BY_OFFSET) | Eleve (C++ storage) | bypass planner complet | **V4 — gros volumes only** |

**Recommandation** :
1. **V2** : UNWIND batch dans les processors — le plus gros gain, zero modif rag3db
2. **V2.5** : Cache `uuid→offset` cote rag3weaver (Option C) — ajouter `RETURN ID(n)` aux INSERT, cacher le mapping, utiliser dans les batch SET si le planner l'optimise
3. **V3+** : Si les benchmarks montrent un bottleneck sur le hash lookup, alors Option A ou B

---

## Ordre d'implementation

1. **Doc 16** (ce document) — inventaire et questions
2. **Verifier les questions ouvertes** — tester sur une DB Kuzu les queries UNWIND/IN/WITH
3. **2D** : Arc<Vec<String>> textes partages — change les structs, affecte tout
4. **1A + 1B** : Batch processors UNWIND — plus gros gain perf
5. **2A** : Arc<RwLock<>> incremental sparse — elimine rebuild
6. **1C** : explore_bfs batch — gros gain pour explore
7. **1D** : rebuild UNION ALL — cold start only apres 2A
8. **1E + 1F** : Combine update/delete queries
9. **2B** : Cache sparse FIFO
10. **2C** : Filtrage sparse allowed_uuids
11. **2E** : Cleanup allocations
12. **Doc 17** : rapport post-implementation
