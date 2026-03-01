# 09 — Cahier des charges : Tests rag3weaver complets

## Objectif

Tester **toutes** les fonctionnalités du framework rag3weaver de bout en bout, sur deux plateformes :
- **Natif** (Rust E2E avec `rag3db-native`) — tests rapides, isolent les bugs du framework
- **WASM** (Playwright browser) — valident que tout fonctionne dans l'environnement cible

Principe : **chaque fonctionnalité doit être testée en natif d'abord**, puis en WASM.

---

## Inventaire des fonctionnalités

### A. Cycle de vie des entités

| # | Fonctionnalité | Testé natif ? | Testé WASM ? |
|---|----------------|:---:|:---:|
| A1 | `create()` — UUID random | ✅ | ✅ |
| A2 | `create()` — UUID hashsafe (dédup) | ✅ | ❌ |
| A3 | `drain()` — pipeline complet | ✅ (sans KB) | ✅ (sans KB) |
| A4 | `get()` / `get_many()` | ✅ | ❌ |
| A5 | `exists()` | ✅ | ❌ |
| A6 | `count()` | ✅ | ✅ |
| A7 | `link()` — relations | ✅ | ✅ |
| A8 | `update()` — détection changement (hash) | ✅ | ❌ |
| A9 | `delete()` — cascade chunks + relations | ✅ | ❌ |
| A10 | `getUuid()` — résolution ref → uuid | ❌ | ✅ |

### B. Knowledge Bases & Ingestion

| # | Fonctionnalité | Testé natif ? | Testé WASM ? |
|---|----------------|:---:|:---:|
| B1 | KB avec `titleFor` + `contentFor` | ❌ | ❌ |
| B2 | Drain avec KB → embeddings calculés | ❌ | ❌ (drain OK, search KO) |
| B3 | Drain avec KB → FTS indexé | ❌ | ❌ |
| B4 | Chunking (Text long → chunks) | ❌ | ❌ |
| B5 | Chunk cascade (delete parent → delete chunks) | ❌ | ❌ |
| B6 | Multi-KB (même entité, 2 KBs) | ❌ | ❌ |
| B7 | Sparse embeddings (BM42) | ❌ | ❌ |
| B8 | Update → re-embed si contenu changé | ❌ | ❌ |
| B9 | Filter fields (String/Int64/Double dans Tantivy) | ❌ | ❌ |

### C. Search

| # | Fonctionnalité | Testé natif ? | Testé WASM ? |
|---|----------------|:---:|:---:|
| C1 | BM25 seul (mode `fulltext`) | ❌ | ❌ |
| C2 | Vector seul (mode `semantic`) | ❌ | ❌ |
| C3 | Hybrid dense+BM25 (mode `hybrid`) | ❌ | ❌ |
| C4 | Sparse seul (via sparse_vector extension) | ❌ | ❌ |
| C5 | 3-way hybrid (dense+BM25+sparse) | ❌ | ❌ |
| C6 | BM25 fuzzy (distance > 0) | ❌ | ❌ |
| C7 | Filtres natifs Tantivy (pre-filtering) | ❌ | ❌ |
| C8 | Filtres Cypher (post-filtering WHERE) | ❌ | ❌ |
| C9 | Filtres combinés (Tantivy + Cypher) | ❌ | ❌ |
| C10 | Search sur chunks (résultats = chunks, pas parents) | ❌ | ❌ |
| C11 | Search avec `consistency: immediate` | ❌ | ❌ |
| C12 | Explore (search + graph traversal) | ❌ | ❌ |

### D. Fusion

| # | Fonctionnalité | Testé natif ? | Testé WASM ? |
|---|----------------|:---:|:---:|
| D1 | Boost fusion | ❌ | ❌ |
| D2 | RRF fusion | ❌ | ❌ |
| D3 | Weighted fusion | ❌ | ❌ |

### E. Embedders

| # | Fonctionnalité | Testé natif ? | Testé WASM ? |
|---|----------------|:---:|:---:|
| E1 | MockEmbedder (zéro vectors) | ✅ (implicite) | ✅ |
| E2 | CandleEmbedder MiniLM-L6 (22MB, 384d) | ✅ (unit) | ✅ (candle_embed.spec) |
| E3 | CandleEmbedder Multilingual-L12 (471MB, 384d) | ❌ | ❌ (spec existe, search KO) |
| E4 | BM42Embedder (sparse) | ❌ | ❌ |
| E5 | BgeM3Embedder (dense+sparse natif) | ✅ (unit, 1/9) | N/A |
| E6 | CallbackEmbedder (closure user) | ❌ | ❌ |
| E7 | `setEmbedder()` hot-swap WASM | N/A | ❌ (spec existe, search KO) |

### F. Types de champs

| # | Fonctionnalité | Testé natif ? | Testé WASM ? |
|---|----------------|:---:|:---:|
| F1 | Text | ✅ | ✅ |
| F2 | String | ❌ | ❌ |
| F3 | Int64 / Integer | ❌ | ❌ |
| F4 | Double / Number | ❌ | ❌ |
| F5 | Boolean | ❌ | ❌ |
| F6 | Timestamp | ❌ | ❌ |
| F7 | Json | ❌ | ❌ |
| F8 | Tags | ❌ | ❌ |
| F9 | Choice | ❌ | ❌ |

### G. Divers

| # | Fonctionnalité | Testé natif ? | Testé WASM ? |
|---|----------------|:---:|:---:|
| G1 | Event bus (subscribe → events reçus) | ❌ | ❌ |
| G2 | Persistance IDBFS (close/reopen) | N/A | ✅ |
| G3 | Persistance opérations (crash recovery) | ❌ | ❌ |
| G4 | Drain async (drainAsyncStart/Poll) | N/A | ✅ |
| G5 | Search async (searchAsyncStart/Poll) | N/A | ❌ |
| G6 | Threading WASM (thread/rayon/futures) | N/A | ✅ |
| G7 | Relations avec propriétés | ❌ | ❌ |

---

## Plan de tests incrémental

### Principes

1. **Natif d'abord** — chaque phase a un test Rust E2E avant le test WASM
2. **Incrémental** — chaque phase s'appuie sur les précédentes
3. **Config réaliste** — une seule config partagée, représentative d'un vrai usage
4. **Pas de mock pour le search** — MockEmbedder uniquement pour les phases sans search sémantique

### Config de référence

```json
{
  "name": "test-rag3weaver",
  "entities": {
    "Document": {
      "fields": {
        "title":    { "fieldType": "Text",    "titleFor": "main" },
        "body":     { "fieldType": "Text",    "contentFor": "main", "chunked": true },
        "summary":  { "fieldType": "Text",    "contentFor": "main" },
        "category": { "fieldType": "String" },
        "year":     { "fieldType": "Integer" },
        "score":    { "fieldType": "Double" },
        "archived": { "fieldType": "Boolean" }
      },
      "hashsafe": ["title"]
    },
    "Author": {
      "fields": {
        "name":  { "fieldType": "Text", "titleFor": "authors" },
        "bio":   { "fieldType": "Text", "contentFor": "authors" }
      }
    }
  },
  "relations": {
    "WRITTEN_BY": { "from": "Document", "to": "Author" },
    "REFERENCES": { "from": "Document", "to": "Document" },
    "CITES": {
      "from": "Document", "to": "Document",
      "properties": { "context": { "fieldType": "Text" } }
    }
  },
  "knowledgeBases": {
    "main": {
      "search": "hybrid",
      "chunking": { "enabled": true, "maxSize": 500, "overlap": 50, "strategy": "markdown" },
      "sparse": true,
      "sparseWeight": 0.2
    },
    "authors": {
      "search": "semantic"
    }
  },
  "embeddingDim": 384
}
```

Cette config couvre :
- **2 entités** (Document, Author) → multi-entité
- **2 KBs** (main, authors) → multi-KB
- **7 types de champs** → String, Text, Integer, Double, Boolean + titleFor/contentFor/chunked
- **3 relations** dont 1 avec propriétés → relations simples et typées
- **Hashsafe** sur Document.title → déduplication
- **Chunking** activé sur main → chunks testés
- **Sparse** activé sur main → 3-way hybrid
- **Filter fields** : category (String), year (Integer), score (Double), archived (Boolean) → filtrage natif Tantivy

### Jeu de données

```
Documents :
  D1: "Rust Programming"      / body: long markdown about Rust safety, ownership, lifetimes (~2000 chars)
                               / category: "programming", year: 2024, score: 9.5, archived: false
  D2: "Python Data Science"   / body: long markdown about Python, pandas, numpy (~2000 chars)
                               / category: "programming", year: 2023, score: 8.0, archived: false
  D3: "French Cuisine"        / body: long markdown about cuisine française (~2000 chars)
                               / category: "cooking", year: 2022, score: 7.5, archived: false
  D4: "Machine Learning"      / body: long markdown about neural networks, deep learning (~2000 chars)
                               / category: "programming", year: 2024, score: 9.0, archived: false
  D5: "Archived Article"      / body: short text
                               / category: "misc", year: 2020, score: 3.0, archived: true

Authors :
  A1: "Alice" / bio: "Expert in Rust and systems programming"
  A2: "Bob"   / bio: "Data scientist specializing in Python and ML"

Relations :
  D1 -[WRITTEN_BY]-> A1
  D2 -[WRITTEN_BY]-> A2
  D4 -[WRITTEN_BY]-> A2
  D4 -[REFERENCES]-> D2
  D4 -[CITES { context: "foundational work on data preprocessing" }]-> D2
  D1 -[REFERENCES]-> D4
```

---

### Phase 0 — Fondations (natif uniquement)

**But :** valider create + drain + CRUD avec la config KB réaliste.

**Tests :**
| Test | Action | Assertion |
|------|--------|-----------|
| 0.1 | Initialize avec config complète | Pas d'erreur, DDL exécuté |
| 0.2 | Create 5 Documents + 2 Authors + drain | processed=7, failed=0 |
| 0.3 | count("Document") / count("Author") | 5 / 2 |
| 0.4 | get(D1.uuid) | title="Rust Programming", tous les champs présents |
| 0.5 | exists(D1.uuid) = true, exists("fake") = false | |
| 0.6 | Hashsafe : re-create D1 (même title) | Même UUID retourné |
| 0.7 | Link relations + drain | 6 relations créées |
| 0.8 | Update D1 (body changé) | status=Updated, reembedded=true |
| 0.9 | Update D1 (même data) | status=Unchanged |
| 0.10 | Delete D5 | chunks + entity supprimés, count=4 |

**Embedder :** MockEmbedder (on ne teste pas le search ici)

---

### Phase 1 — BM25 seul (natif puis WASM)

**But :** valider que le FTS Tantivy fonctionne via Catalog.search().

**Prérequis :** MockEmbedder (BM25 ne nécessite pas de vrais embeddings).

**Config KB :** `search: "fulltext"` (override pour cette phase).

**Tests :**
| Test | Action | Assertion |
|------|--------|-----------|
| 1.1 | search("main", "Rust safety") | results.length > 0, D1 en premier |
| 1.2 | search("main", "neural networks") | results.length > 0, D4 en premier |
| 1.3 | search("main", "cuisine française") | results.length > 0, D3 en premier |
| 1.4 | Fuzzy : search("main", "programing") | results.length > 0 (typo corrigée) |
| 1.5 | Pas de résultat : search("main", "xyznonexistent") | results.length = 0 |
| 1.6 | meta.bm25Count > 0, meta.vectorCount = 0 | BM25 seul |
| 1.7 | meta.searchType = "BM25Only" ou "Fulltext" | |

**Ce que ça valide :**
- `CREATE_TANTIVY_INDEX` fonctionne après drain (hooks insert)
- `QUERY_TANTIVY_INDEX` retourne des résultats
- Résolution offset → UUID fonctionne
- contentFor fait que body est bien dans le FTS index

---

### Phase 2 — Vector seul (natif puis WASM)

**But :** valider HNSW vector search via Catalog.search().

**Embedder :** CandleEmbedder MiniLM-L6 (22MB, 384d).

**Config KB :** `search: "semantic"`.

**Tests :**
| Test | Action | Assertion |
|------|--------|-----------|
| 2.1 | search("main", "systems programming language") | results.length > 0, D1 en top 3 |
| 2.2 | search("main", "data analysis with Python") | results.length > 0, D2 en top 3 |
| 2.3 | Pertinence sémantique : "cooking recipes" | D3 en premier (pas de match lexical "cooking" dans body) |
| 2.4 | meta.vectorCount > 0, meta.bm25Count = 0 | Vector seul |
| 2.5 | Scores décroissants | results[0].score >= results[1].score |
| 2.6 | search("authors", "Rust expert") | A1 en premier |

**Ce que ça valide :**
- `CREATE_VECTOR_INDEX` + HNSW indexe les rows insérées APRÈS create index
- embed_query() + QUERY_VECTOR_INDEX retourne des résultats
- Multi-KB fonctionne (main + authors)
- Cosine similarity scores cohérents

---

### Phase 3 — Hybrid dense + BM25 (natif puis WASM)

**But :** valider la fusion BM25 + vector.

**Embedder :** CandleEmbedder MiniLM-L6.

**Config KB :** `search: "hybrid"`.

**Tests :**
| Test | Action | Assertion |
|------|--------|-----------|
| 3.1 | search("main", "Rust programming") | results.length > 0 |
| 3.2 | meta.vectorCount > 0 ET meta.bm25Count > 0 | Les deux sources contribuent |
| 3.3 | meta.fusedCount > 0 | Fusion a eu lieu |
| 3.4 | Boost fusion (défaut) | D1 en premier (match lexical ET sémantique) |
| 3.5 | RRF fusion (hybridStrategy: "rrf") | Résultats ordonnés, scores > 0 |
| 3.6 | Weighted fusion (hybridStrategy: "weighted") | Résultats ordonnés |
| 3.7 | keyword_weight=1.0 → quasi BM25 seul | Résultats proches de Phase 1 |
| 3.8 | keyword_weight=0.0 → quasi vector seul | Résultats proches de Phase 2 |

---

### Phase 4 — Filtres (natif puis WASM)

**But :** valider le filtrage natif Tantivy + filtrage Cypher.

**Embedder :** CandleEmbedder MiniLM-L6.

**Tests :**
| Test | Action | Assertion |
|------|--------|-----------|
| 4.1 | filter: { category: "programming" } | Seulement D1, D2, D4 (pas D3 cooking) |
| 4.2 | filter: { year: { $gte: 2024 } } | Seulement D1, D4 |
| 4.3 | filter: { archived: false } | Pas D5 |
| 4.4 | filter: { score: { $gte: 9.0 } } | D1 (9.5) et D4 (9.0) |
| 4.5 | Combiné : category=programming + year>=2024 | D1 et D4 |
| 4.6 | FilterCondition (AND/OR imbriqué) | Résultats corrects |
| 4.7 | Filtre sans résultat : category="nonexistent" | results.length = 0 |

**Ce que ça valide :**
- `filter_fields` passés à Tantivy → pre-filtering segment-level
- FilterCompiler → Cypher WHERE génère les bonnes clauses
- SplitResult sépare correctement Tantivy-natif vs post-filtre

---

### Phase 5 — Chunking (natif puis WASM)

**But :** valider le text splitting + search sur chunks.

**Embedder :** CandleEmbedder MiniLM-L6.

**Tests :**
| Test | Action | Assertion |
|------|--------|-----------|
| 5.1 | Après drain, D1 a des chunks | count("Document_Chunk") > 0 pour D1 |
| 5.2 | Chunks ont _text, _index, _parent_uuid | Champs présents |
| 5.3 | Chunks couvrent le body complet | Pas de trou dans le texte |
| 5.4 | Overlap entre chunks adjacents | chunk[i].end > chunk[i+1].start |
| 5.5 | Search retourne des ChunkInfo | result.chunk.text contient le passage pertinent |
| 5.6 | Delete parent → chunks supprimés | count("Document_Chunk") diminue |
| 5.7 | Update body → re-chunk | Nouveaux chunks, anciens supprimés |

---

### Phase 6 — Sparse BM42 (natif puis WASM)

**But :** valider sparse embeddings + 3-way fusion.

**Embedder dense :** CandleEmbedder MiniLM-L6.
**Embedder sparse :** BM42Embedder (réutilise le même modèle BERT).

**Config KB :** `sparse: true, sparseWeight: 0.2`.

**Tests :**
| Test | Action | Assertion |
|------|--------|-----------|
| 6.1 | Après drain, sparse_indices et sparse_weights non vides | Champs populés |
| 6.2 | search("main", "Rust") | meta.sparseCount > 0 |
| 6.3 | 3-way fusion | meta.vectorCount > 0, bm25Count > 0, sparseCount > 0 |
| 6.4 | Sparse améliore la pertinence lexicale | Mots-clés exacts mieux rankés |

---

### Phase 7 — Multilingual (WASM uniquement)

**But :** valider cross-lingual search avec multilingual-MiniLM-L12.

**Embedder :** CandleEmbedder Multilingual-L12 (471MB).

**Tests :**
| Test | Action | Assertion |
|------|--------|-----------|
| 7.1 | search("main", "programming language") en anglais | D1, D2, D4 pertinents |
| 7.2 | search("main", "langage de programmation") en français | Mêmes docs pertinents (cross-lingual) |
| 7.3 | search("main", "recettes de cuisine") en français | D3 pertinent |
| 7.4 | Cosine similarity FR↔EN > seuil | sim("Rust programming", "Programmation Rust") > 0.5 |

---

### Phase 8 — Explore / Graph traversal (natif puis WASM)

**But :** valider search_with_explore() — search + expansion de graphe.

**Embedder :** CandleEmbedder MiniLM-L6.

**Tests :**
| Test | Action | Assertion |
|------|--------|-----------|
| 8.1 | explore("main", "machine learning", depth=1) | D4 trouvé + relation REFERENCES→D2 traversée |
| 8.2 | Graph contient nœuds + arêtes | graph.nodes.length > 1, graph.edges.length > 0 |
| 8.3 | depth=2 | Plus de nœuds que depth=1 |
| 8.4 | outgoing_relations filtre | Seulement les rels spécifiées |
| 8.5 | Search results + graph nodes cohérents | Tous les search results sont dans les graph nodes |

---

### Phase 9 — Events (natif uniquement)

**But :** valider l'event bus.

**Tests :**
| Test | Action | Assertion |
|------|--------|-----------|
| 9.1 | subscribe() + create + drain | EntityPrepared, EmbeddingStarted, EmbeddingCompleted, EntitiesStored reçus |
| 9.2 | DrainStarted + DrainCompleted | stats cohérentes |
| 9.3 | search → SearchStarted + SearchCompleted | events reçus |

---

### Phase 10 — Robustesse (natif puis WASM)

**But :** edge cases et error handling.

**Tests :**
| Test | Action | Assertion |
|------|--------|-----------|
| 10.1 | Create entité inconnue | Err(UnknownEntity) |
| 10.2 | Link relation inconnue | Err(UnknownRelation) |
| 10.3 | Search KB inconnue | Err(UnknownKB) |
| 10.4 | Get UUID inexistant | None |
| 10.5 | Search query vide | results.length = 0, pas de panic |
| 10.6 | Create avec champs manquants | defaults appliqués ou erreur claire |
| 10.7 | Drain sur queue vide | processed=0, failed=0 |
| 10.8 | Double initialize() | Idempotent, pas d'erreur |

---

## Résumé des embedders par phase

| Phase | Dense embedder | Sparse embedder | Plateforme |
|-------|---------------|-----------------|------------|
| 0 | MockEmbedder | — | Natif |
| 1 | MockEmbedder | — | Natif + WASM |
| 2 | CandleEmbedder MiniLM-L6 | — | Natif + WASM |
| 3 | CandleEmbedder MiniLM-L6 | — | Natif + WASM |
| 4 | CandleEmbedder MiniLM-L6 | — | Natif + WASM |
| 5 | CandleEmbedder MiniLM-L6 | — | Natif + WASM |
| 6 | CandleEmbedder MiniLM-L6 | BM42Embedder | Natif + WASM |
| 7 | CandleEmbedder Multilingual-L12 | — | WASM |
| 8 | CandleEmbedder MiniLM-L6 | — | Natif + WASM |
| 9 | MockEmbedder | — | Natif |
| 10 | MockEmbedder | — | Natif + WASM |

## Priorité d'implémentation

1. **Phase 0** → débloquer : prouver que la config KB réaliste fonctionne en natif
2. **Phase 1** → débloquer : BM25 fonctionne via Catalog (principal bug actuel)
3. **Phase 2** → débloquer : HNSW fonctionne après drain (second bug actuel)
4. **Phase 3** → fusion fonctionne, mode hybrid complet
5. **Phase 4-5** → filtres + chunking
6. **Phase 6** → sparse (dépend de BM42 dans pipeline)
7. **Phase 7** → multilingual (gros modèle, WASM only)
8. **Phase 8-10** → explore, events, robustesse

## Fichiers à créer/modifier

| Fichier | Contenu |
|---------|---------|
| `extension/rag3weaver/tests/e2e_search.rs` | Phases 0-6, 8-10 natif (nouveau) |
| `tools/wasm/test/browser/search_worker.js` | Worker WASM pour phases 1-8 |
| `tools/wasm/test/browser/search.spec.js` | Playwright pour phases 1-8 WASM |
| `tools/wasm/test/browser/search.html` | Page HTML pour le worker |
| Modifier `set_embedder_worker.js` | Corriger config (ajouter `contentFor`) |

## Bugs à investiguer en priorité

1. **Pourquoi BM25 retourne 0 ?** — le hook Tantivy insert est-il déclenché par `drain()` ? Le `flushIfDirty()` est-il appelé ?
2. **Pourquoi HNSW retourne 0 ?** — `CREATE_VECTOR_INDEX` sur table vide → les rows insérées après sont-elles indexées ?
3. **Config actuelle des tests WASM** — `body` n'a pas `contentFor: "main"` → body n'est pas dans le KB
