# 02 — Plan d'integration Lucivy dans le Cypher natif rag3db

## Contexte

Actuellement, rag3weaver utilise lucivy_fts via des `CALL` explicites + multiples round-trips Cypher.
L'objectif : integrer FTS directement dans le Cypher pour eliminer les round-trips, pousser les predicats, et simplifier l'API.

## Etat actuel : pipeline rag3weaver pour 1 search BM25

```
Requete 1: CALL QUERY_LUCIVY_INDEX('Document', jsonQuery, limit)
           → (offset, score, highlights)

Requete 2: MATCH (n:Document) WHERE OFFSET(id(n)) IN [offsets]
           RETURN OFFSET(id(n)), n._uuid
           → resolution offset → UUID

Requete 3: MATCH (p:Document)-[:HAS_CHUNK]->(c:Document_Chunk)
           WHERE p._uuid IN [uuids]
           RETURN p._uuid, c._uuid, c._text, c._index, ...
           → chunk resolution (si chunked)

Requete 4: MATCH (n:Document) WHERE n._uuid IN [uuids]
           RETURN n._uuid, n.title, n.body, ...
           → data enrichment
```

**4 requetes sequentielles** pour un seul search. En hybrid (BM25 + vector), c'est 6-8 requetes.

### Usages detailles rag3weaver

| Operation | Cypher actuel | Frequence |
|-----------|--------------|-----------|
| **CREATE index** | `CALL CREATE_LUCIVY_INDEX('T', ['f1','f2'], filter_fields := ['ff1'])` | 1x init |
| **BM25 search** | `CALL QUERY_LUCIVY_INDEX('T', jsonQuery, limit) RETURN node_id, score` | chaque search |
| **BM25 raw** | `...RETURN node_id, score, highlights` | chaque search chunked |
| **Offset → UUID** | `MATCH (n:T) WHERE OFFSET(id(n)) IN [...] RETURN OFFSET(id(n)), n._uuid` | chaque search |
| **Chunk resolution** | `MATCH (p:T)-[:T_HAS_CHUNK]->(c:T_Chunk) WHERE p._uuid IN [...] RETURN ...` | si chunked |
| **Data enrichment** | `MATCH (n:T) WHERE n._uuid IN [...] RETURN n._uuid, n.title, ...` | chaque search |
| **Filter pre-fetch** | `MATCH (n:T) WHERE n.year >= 2024 RETURN OFFSET(id(n))` | si filtres Cypher |
| **DROP index** | (pas utilise par rag3weaver) | — |

### 4 modes de query JSON

| Mode | JSON passe a QUERY_LUCIVY_INDEX |
|------|----------------------------------|
| Parse | `{"type":"parse","fields":["title","body"],"value":"rust"}` |
| Contains | `{"type":"contains","field":"body","value":"rust","distance":1}` |
| ContainsSplit | `{"type":"boolean","should":[{"type":"contains",...},...]}`|
| Regex | `{"type":"contains","field":"body","value":"prog.*","regex":true}` |

### Highlights

Retournes en JSON : `{"body":[[100,200],[300,350]],"title":[[5,15]]}` — par champ, intervalles [start, end] en caracteres.

Utilises pour le chunk resolution : on matche les intervalles highlight avec les bornes `_start_char/_end_char` des chunks.

---

## Architecture interne rag3db (Kuzu v0.11.2.2)

### Query planner — index integration actuelle

Le seul index integre au planner = **PrimaryKeyIndex** (hash index).

```
FilterPushDownOptimizer::visitScanNodeTableReplace()
  → predicateSet.popNodePKEqualityComparison(*nodeID)
  → si WHERE node.pk = <constant> :
      scan.setScanType(PRIMARY_KEY_SCAN)
      scan.setExtraInfo(PrimaryKeyScanInfo{key})
```

Pipeline complet : `MATCH → LogicalScanNodeTable(SCAN) → Optimizer → PRIMARY_KEY_SCAN → PrimaryKeyScanNodeTable → NodeTable::lookupPK()`

**Limitations** :
- Seulement egalite exacte sur PK (`WHERE n.id = 5`)
- Pas de support : `IN`, `>`, `<`, ni predicats sur colonnes secondaires
- Zone maps existent pour range queries mais limites

### Extensions vector et lucivy — aucune integration planner

Les deux utilisent un pattern identique : **TABLE FUNCTION** avec `CALL`.

| Aspect | Vector (HNSW) | Lucivy (FTS) |
|--------|--------------|---------------|
| Create | `CALL CREATE_VECTOR_INDEX(...)` | `CALL CREATE_LUCIVY_INDEX(...)` |
| Query | `CALL QUERY_VECTOR_INDEX(...)` | `CALL QUERY_LUCIVY_INDEX(...)` |
| Retour | `(internal_id, distance)` | `(node_id_offset, score, highlights)` |
| Filtres | SemiMask via Cypher filter graph | `allowed_ids` pre-compute OU JSON filters |
| Planner | Aucune integration | Aucune integration |

Le vector extension a un mecanisme elegant : il accepte un `filter := 'MATCH (n:T) WHERE ... RETURN n'` qui est parse, planifie, et execute comme un SemiMask pre-search.

---

## Niveaux d'integration proposes

### Niveau 0 : Etat actuel (reference)

4-5 requetes par search. JSON pour la query. Offset resolution manuelle.

**Cout** : 4+ round-trips Cypher, latence cumulee, complexite rag3weaver.

### Niveau 1 : Composition Cypher unique (zero changement moteur)

**Principe** : combiner CALL + MATCH + enrichment en une seule requete Cypher composee.

```cypher
-- BM25 + resolution UUID + enrichment en 1 requete
CALL QUERY_LUCIVY_INDEX('Document', $query_json, $limit)
WITH node_id, score, highlights
MATCH (d:Document) WHERE OFFSET(id(d)) = node_id
RETURN d._uuid, d.title, d.body, score, highlights
ORDER BY score DESC
```

```cypher
-- BM25 + chunks + enrichment en 1 requete
CALL QUERY_LUCIVY_INDEX('Document', $query_json, $limit)
WITH node_id, score, highlights
MATCH (d:Document) WHERE OFFSET(id(d)) = node_id
OPTIONAL MATCH (d)-[:Document_HAS_CHUNK]->(c:Document_Chunk)
RETURN d._uuid, d.title, score, highlights,
       COLLECT({
         uuid: c._uuid, text: c._text,
         start_char: c._start_char, end_char: c._end_char
       }) AS chunks
ORDER BY score DESC
```

```cypher
-- Avec filtres Cypher (post-filter au lieu de pre-fetch)
CALL QUERY_LUCIVY_INDEX('Document', $query_json, $over_fetch_limit)
WITH node_id, score, highlights
MATCH (d:Document) WHERE OFFSET(id(d)) = node_id
  AND d.year >= 2024 AND d.category = 'programming'
RETURN d._uuid, d.title, score, highlights
ORDER BY score DESC LIMIT $limit
```

**Gains** :
- 4 requetes → 1 requete
- Elimination totale du round-trip offset → UUID
- Enrichment integre (plus de requete separee)
- Chunk resolution dans le meme pipeline

**Limites** :
- Filtres Cypher = post-filter (pas pushdown vers Lucivy)
- Over-fetch necessaire si filtres (demander plus que `limit` a Lucivy)
- JSON query toujours present

**Effort** : rag3weaver seulement (Rust), zero changement C++/moteur.

**Impact** : immediat et significatif. C'est le quick win.

### Niveau 1b : Filtres Lucivy-natifs + composition

Combiner les filtres Lucivy JSON (pre-filter efficace au niveau segment) avec la composition Cypher :

```cypher
-- Filtres sur colonnes indexees Lucivy = pre-filter segment-level
-- Filtres Cypher restants = post-filter sur resultats
CALL QUERY_LUCIVY_INDEX('Document',
  '{"type":"parse","fields":["title","body"],"value":"rust",
    "filters":[{"field":"year","op":"gte","value":2024}]}',
  $limit)
WITH node_id, score, highlights
MATCH (d:Document) WHERE OFFSET(id(d)) = node_id
  AND d.archived = false  -- filtre Cypher restant (pas dans Lucivy)
RETURN d._uuid, d.title, score, highlights
ORDER BY score DESC
```

**Gains supplementaires** : split intelligent des filtres = pre-filter Lucivy (rapide, segment-level) + post-filter Cypher (flexible).

rag3weaver fait deja ce split (FilterCompiler). Il suffit de l'integrer dans la composition.

### Niveau 2 : QUERY_LUCIVY_INDEX ameliore (changements C++ extension)

**Principe** : QUERY_LUCIVY_INDEX retourne directement des node IDs (pas des offsets) et fait le lookup properties en interne.

```cypher
-- QUERY_LUCIVY_INDEX retourne des nœuds, pas des offsets
CALL QUERY_LUCIVY_INDEX('Document', $query_json, $limit,
    return_fields := ['_uuid', 'title', 'body'])
RETURN _uuid, title, body, score, highlights
ORDER BY score DESC
```

**Changements C++** :
1. `query_lucivy_index.cpp` : apres search Lucivy, faire un NodeTable::lookup pour chaque hit
2. Ajouter les colonnes demandees au output schema
3. Retourner `(field1, field2, ..., score, highlights)` au lieu de `(node_id, score, highlights)`

**Gains** :
- Plus besoin de MATCH apres le CALL
- Un seul operateur fait tout : search + property fetch
- Pattern identique a ce que fait le vector extension (retourne internal_id)

**Option bonus** : supporter un parametre `filter` comme le vector extension :
```cypher
CALL QUERY_LUCIVY_INDEX('Document', $query_json, $limit,
    filter := 'MATCH (n:Document) WHERE n.year >= 2024 RETURN n',
    return_fields := ['_uuid', 'title'])
RETURN _uuid, title, score, highlights
```

Cela permettrait le SemiMask pattern (filter Cypher → mask → Lucivy search avec mask).

### Niveau 3 : Predicat FTS dans WHERE (changements optimizer)

**Principe** : reconnaitre un predicat `SEARCH(node, query)` dans WHERE et le convertir en index scan.

```cypher
MATCH (d:Document)
WHERE SEARCH(d, 'rust safety')
  AND d.year >= 2024
RETURN d, SEARCH_SCORE(d) AS score, SEARCH_HIGHLIGHTS(d) AS hl
ORDER BY score DESC LIMIT 10
```

**Architecture** :

```
1. Planner : MATCH (d:Document) WHERE SEARCH(d, 'rust') AND d.year >= 2024
   → LogicalScanNodeTable(SCAN) + LogicalFilter(SEARCH(...) AND year >= 2024)

2. FilterPushDownOptimizer (etendu) :
   → Detecte SEARCH(d, 'rust') dans les predicats
   → Extrait : {table: Document, query: 'rust', mode: 'parse'}
   → Detecte d.year >= 2024 comme filtre pushable vers Lucivy
   → scan.setScanType(FTS_SCAN)
   → scan.setExtraInfo(FTSScanInfo{query, lucivyFilters, cypherFilters})

3. PlanMapper :
   → Cree FTSScanNodeTable (nouvel operateur physique)

4. FTSScanNodeTable::getNextTuples() :
   → flushIfDirty()
   → search_filtered_with_highlights(query, limit, filters)
   → Pour chaque hit : NodeTable::lookup → properties
   → Expose : node properties + score + highlights
   → Applique les filtres Cypher restants en post-filter
```

**Nouveaux composants** :

| Composant | Fichier | Description |
|-----------|---------|-------------|
| `LogicalScanNodeTableType::FTS_SCAN` | `logical_scan_node_table.h` | Nouveau type de scan |
| `FTSScanInfo` | `logical_scan_node_table.h` | Query, filtres, config |
| `FTSScanNodeTable` | Nouveau operateur physique | Execute le search Lucivy |
| Extension `FilterPushDownOptimizer` | `filter_push_down_optimizer.cpp` | Detection SEARCH() |
| `SEARCH()` scalar function | Nouvelle function enregistree | Marqueur pour l'optimizer |
| `SEARCH_SCORE()` | Nouvelle function | Acces au score FTS |
| `SEARCH_HIGHLIGHTS()` | Nouvelle function | Acces aux highlights |

**Challenges** :
- `SEARCH_SCORE(d)` et `SEARCH_HIGHLIGHTS(d)` sont des "colonnes virtuelles" qui n'existent que dans le contexte d'un FTS_SCAN. Il faut un mecanisme pour les transporter dans le pipeline.
- Le LIMIT doit etre pousse vers le scan (sinon Lucivy retourne tout).
- Le ORDER BY score doit aussi etre pousse (Lucivy retourne deja ordonne).
- La coexistence avec d'autres predicats (graph patterns, joins) est complexe.

**Effort** : significatif. Modification du core rag3db (optimizer, planner, mapper, nouvel operateur).

### Niveau 4 : Search unifie (BM25 + vector + sparse en 1 query)

**Vision finale** :

```cypher
-- Hybrid search en 1 seul MATCH
MATCH (d:Document)
WHERE SEARCH(d, 'rust safety', kb := 'main')
RETURN d._uuid, d.title,
       SEARCH_SCORE(d) AS score,
       SEARCH_BM25_SCORE(d) AS bm25,
       SEARCH_VECTOR_SCORE(d) AS vector,
       SEARCH_HIGHLIGHTS(d) AS hl,
       SEARCH_CHUNKS(d) AS chunks
ORDER BY score DESC LIMIT 10

-- Graph-aware search
MATCH (d:Document)-[:WRITTEN_BY]->(a:Author)
WHERE SEARCH(d, 'rust') AND a.name = 'Alice'
RETURN d, a, SEARCH_SCORE(d) AS score
ORDER BY score DESC LIMIT 10
```

**Cela impliquerait** :
- Fusion BM25 + vector + sparse dans un seul operateur
- Chunk resolution integree
- Graph traversal apres search
- Le planner optimise automatiquement l'ordre (search first, then graph join)

**Effort** : tres important. C'est un objectif a long terme.

---

## Analyse d'impact pour rag3weaver

### Ce que chaque niveau change dans rag3weaver (search.rs + catalog.rs)

| Niveau | search.rs | catalog.rs | Queries/search | Changement moteur |
|--------|-----------|------------|----------------|-------------------|
| 0 (actuel) | build JSON → CALL → resolve offsets → chunks → enrich | 5 fonctions orchestrent | 4-5 | aucun |
| 1 | single composed Cypher | 1 seul execute() | **1** | aucun |
| 1b | + split filtres dans JSON | idem + FilterCompiler | **1** | aucun |
| 2 | simplified CALL avec return_fields | plus simple | **1** | C++ extension |
| 3 | `MATCH WHERE SEARCH()` | tres simplifie | **1** | C++ core |
| 4 | `MATCH WHERE SEARCH() + graph` | minimal | **1** | C++ core majeur |

### Gains de performance estimes

| Niveau | Latence relative | Round-trips | Complexite rag3weaver |
|--------|-----------------|-------------|----------------------|
| 0 | 1x (reference) | 4-5 | haute |
| 1 | ~0.3x | 1 | moyenne |
| 1b | ~0.25x | 1 | moyenne |
| 2 | ~0.2x | 1 | faible |
| 3 | ~0.15x | 1 | tres faible |
| 4 | ~0.1x | 1 | minimale |

Le gain principal vient de l'elimination des round-trips (niveau 1). Les niveaux suivants reduisent le overhead interne.

---

## Plan d'execution recommande

### Phase A : Niveau 1 — Composition Cypher (immediat)

**Effort** : 1-2 sessions. Zero changement moteur.

1. Refactorer `search_bm25()` pour composer une seule requete CALL + WITH + MATCH + RETURN
2. Refactorer `search_bm25_raw()` pour inclure le chunk OPTIONAL MATCH
3. Integrer `enrich_results_with_data()` dans la meme requete
4. Adapter le FilterCompiler pour generer les filtres JSON (Lucivy) + WHERE (Cypher) dans la meme requete
5. Supprimer les fonctions separees `resolve_offsets_to_uuids()` et `enrich_results_with_data()`
6. Tests : re-run cahier des charges phases 0-2

**Queries avant/apres** :

Avant (non-chunked) :
```
1. CALL QUERY_LUCIVY_INDEX(...) RETURN node_id, score
2. MATCH (n:T) WHERE OFFSET(id(n)) IN [...] RETURN ..., n._uuid
3. MATCH (n:T) WHERE n._uuid IN [...] RETURN n._uuid, n.title, ...
```

Apres (non-chunked) :
```
1. CALL QUERY_LUCIVY_INDEX(...) WITH node_id, score
   MATCH (d:T) WHERE OFFSET(id(d)) = node_id
   RETURN d._uuid, d.title, d.body, score
   ORDER BY score DESC
```

Avant (chunked) :
```
1. CALL QUERY_LUCIVY_INDEX(...) RETURN node_id, score, highlights
2. MATCH (n:T) WHERE OFFSET(id(n)) IN [...] RETURN ..., n._uuid
3. MATCH (p:T)-[:T_HAS_CHUNK]->(c:T_Chunk) WHERE p._uuid IN [...]
   RETURN p._uuid, c._uuid, c._text, c._start_char, c._end_char, ...
4. MATCH (n:T) WHERE n._uuid IN [...] RETURN n._uuid, n.title, ...
```

Apres (chunked) :
```
1. CALL QUERY_LUCIVY_INDEX(...) WITH node_id, score, highlights
   MATCH (d:T) WHERE OFFSET(id(d)) = node_id
   OPTIONAL MATCH (d)-[:T_HAS_CHUNK]->(c:T_Chunk)
   RETURN d._uuid, d.title, score, highlights,
          COLLECT({uuid: c._uuid, text: c._text,
                   start_char: c._start_char, end_char: c._end_char,
                   start_line: c._start_line, end_line: c._end_line,
                   index: c._index, parent_field: c._parent_field}) AS chunks
   ORDER BY score DESC
```

### Phase B : Niveau 2 — QUERY_LUCIVY_INDEX ameliore

**Effort** : 2-3 sessions. Changement C++ extension lucivy_fts.

1. Ajouter parametre `return_fields` a QUERY_LUCIVY_INDEX
2. Faire le NodeTable::lookup dans le tableFunc (apres search Lucivy)
3. Ajouter parametre `filter` (pattern SemiMask comme vector extension)
4. Retourner `(field1, field2, ..., score, highlights)` au lieu de `(node_id, score, highlights)`
5. Adapter rag3weaver pour utiliser la nouvelle API

### Phase C : Niveau 3 — Integration optimizer (long terme)

**Effort** : 5-10 sessions. Changement core rag3db.

1. Enregistrer `SEARCH()`, `SEARCH_SCORE()`, `SEARCH_HIGHLIGHTS()` comme fonctions
2. Etendre `FilterPushDownOptimizer` pour detecter `SEARCH(node, query)`
3. Nouveau `LogicalScanNodeTableType::FTS_SCAN`
4. Nouveau operateur physique `FTSScanNodeTable`
5. Integration LIMIT/ORDER BY pushdown
6. Adapter rag3weaver pour syntaxe `MATCH WHERE SEARCH()`

---

## Considerations techniques

### Le probleme UNWIND + MATCH (doc 17-20)

Le bug corrige (HashMap → BTreeMap) affectait la composition Cypher. Maintenant corrige, la composition au niveau 1 est fiable.

Mais attention : la composition `CALL ... WITH ... MATCH ... WHERE OFFSET(id(d)) = node_id` utilise un pattern similaire au CROSS_PRODUCT + FLATTEN + FILTER. Il faut verifier que le bug `selectFunc` (doc 18) ne reapparait pas avec N resultats Lucivy.

**Action** : tester la composition avec 10+ resultats avant de deployer.

### Performances du pattern OFFSET(id(n)) = node_id

Ce pattern force un scan de table + filter (pas d'index scan, car ce n'est pas le PK). Pour N resultats Lucivy, c'est O(N * table_size).

**Alternative** : si QUERY_LUCIVY_INDEX retournait des internal_id (type INTERNAL_ID) au lieu d'offsets UINT64, le planner pourrait faire un index lookup direct. C'est un changement de type de retour de QUERY_LUCIVY_INDEX.

Autre alternative : utiliser `WHERE id(d) = $internal_id` avec un INTERNAL_ID construit.

### Highlights comme type structure

Actuellement les highlights sont une STRING JSON parsee cote Rust. Idealement ce serait un type MAP(STRING, LIST(LIST(INT64))) natif rag3db. Mais le parsing JSON cote Rust est deja tres rapide et cette optimisation est secondaire.

### Fusion hybrid dans le moteur vs dans rag3weaver

Pour le niveau 4, la fusion BM25 + vector devrait-elle etre dans le moteur ? Probablement pas — la logique de fusion (Boost/RRF/Weighted) est specifique a l'application. Le moteur devrait fournir les scores individuels, et l'application fusionne.

Le Cypher compose (niveau 1) permet deja de faire 2 CALL + fusion en Rust cote rag3weaver.

---

## Cahier des charges mis a jour (ref doc 09)

### Phases impactees par l'integration Cypher

| Phase CDC | Impact | Niveau min |
|-----------|--------|------------|
| Phase 1 (BM25) | Query unique au lieu de 2 | Niveau 1 |
| Phase 2 (Vector) | Idem pour vector + enrichment | Niveau 1 |
| Phase 3 (Hybrid) | 2 CALL composes + fusion Rust | Niveau 1 |
| Phase 4 (Filtres) | Split Lucivy/Cypher dans 1 query | Niveau 1b |
| Phase 5 (Chunking) | OPTIONAL MATCH chunk dans 1 query | Niveau 1 |
| Phase 8 (Explore) | Search + graph traversal en 1 query | Niveau 1 |

### Nouveaux tests a ajouter

| Test | Description |
|------|-------------|
| `composed_bm25_simple` | CALL + WITH + MATCH + RETURN, verifier resultats identiques |
| `composed_bm25_chunks` | CALL + WITH + MATCH + OPTIONAL MATCH chunks |
| `composed_bm25_filters` | CALL (filtres Lucivy) + WITH + MATCH + WHERE (filtres Cypher) |
| `composed_bm25_enriched` | Verifier que tous les champs enrichis sont presents |
| `composed_bm25_10plus` | Tester avec 10+ resultats (regression bug UNWIND) |
| `composed_hybrid_single_query` | 2 CALL (BM25 + vector) composes + fusion |
| `composed_graph_traversal` | Search + MATCH graph pattern |

---

## Resume

| Priorite | Action | Effort | Gain |
|----------|--------|--------|------|
| **1** | Niveau 1 : composition Cypher unique | 1-2 sessions | 4x moins de round-trips |
| **2** | Niveau 1b : + filtres Lucivy integres | +1 session | pre-filtering efficace |
| **3** | Niveau 2 : QUERY ameliore + return_fields | 2-3 sessions | elimine le MATCH post-CALL |
| **4** | Niveau 3 : MATCH WHERE SEARCH() | 5-10 sessions | Cypher idiomatique |
| long terme | Niveau 4 : search unifie | 10+ sessions | vision finale |

La recommandation : **commencer par le niveau 1 immediatement** — c'est le meilleur ratio effort/impact, zero changement moteur, et ca valide les compositions Cypher pour les niveaux suivants.
