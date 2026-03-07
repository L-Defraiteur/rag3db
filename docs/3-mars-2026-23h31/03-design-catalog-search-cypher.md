# Doc 03 — Design : CATALOG_SEARCH — Cypher haut niveau pour rag3weaver

**Date** : 4 mars 2026
**Branche** : `feature/kb-index-architecture`
**Statut** : Réflexion, pas planifié

---

## 1. Motivation

Aujourd'hui, `Catalog::search()` est une API Rust programmatique. L'appelant construit un `SearchOptions` struct, appelle la méthode, et reçoit un `SearchResponse`. C'est puissant mais :

- **Pas composable avec le graphe** : impossible de combiner un search avec un traversal Cypher dans une seule query
- **Pas exposable comme langage** : un agent LLM ou un utilisateur ne peut pas écrire une "requête de search" en texte
- **Chaque combinaison search+graphe nécessite du code Rust** : le domain adapter doit programmer chaque pattern

L'idée : exposer le search comme une **table function Cypher** (`CATALOG_SEARCH`) composable avec du Cypher natif.

---

## 2. Ce qui existe déjà dans rag3db

### 2.1 Table functions de search (bas niveau)

```cypher
-- BM25 (Lucivy)
CALL QUERY_LUCIVY_INDEX('ScopeKB_Index', '{"query":"auth"}', 10)
RETURN node_id, score

-- Vector (HNSW)
CALL VECTOR_SEARCH('ScopeKB_Index_Chunk', embedding, 10)
RETURN node_id, distance

-- Sparse
CALL SPARSE_SEARCH('ScopeKB_Index_Chunk', sparse_vec, 10)
RETURN node_id, score
```

Ces fonctions sont bas niveau : elles opèrent sur des tables spécifiques, retournent des `node_id` (offsets internes), et ne font pas de fusion.

### 2.2 SEARCH() dans WHERE (extension FTS_SCAN)

```cypher
MATCH (s:Scope) WHERE SEARCH(s.content, 'auth') RETURN s, SEARCH_SCORE()
```

Intégré dans le planner Cypher via `FilterPushDownOptimizer` → `FTSScanNodeTable`. Puissant mais limité au BM25 mono-champ, pas de fusion, pas de chunking.

### 2.3 Parser Cypher de rag3db

- ANTLR4 : grammaire `.g4`, lexer, parser, transformer
- API publique : `Parser::parseQuery(query) → vector<Statement>`
- Extensible via `TransformerExtension`
- C++ uniquement, pas exposé en FFI/WASM

### 2.4 Infrastructure table functions

Deux types :
- **`TableFunc`** : nécessite RETURN clause, retourne un résultat tabulaire (ex: QUERY_LUCIVY_INDEX)
- **`StandaloneTableFunc`** : pas de RETURN, effet de bord (ex: CREATE_LUCIVY_INDEX)

CATALOG_SEARCH serait une `TableFunc` avec YIELD.

---

## 3. Options évaluées

### Option A : Étendre la grammaire ANTLR

Ajouter `SEARCH ... IN ... WHERE ... MODE ...` comme clause Cypher native.

```cypher
SEARCH 'auth middleware' IN ScopeKB
WHERE scope_type = 'function'
MODE detailed
EXPLORE [PARENT_OF, DEFINED_IN] DEPTH 2
LIMIT 10
```

| Avantage | Inconvénient |
|---|---|
| Premier citoyen dans le langage | Modification du parser ANTLR + binder + planner |
| Syntaxe propre et dédiée | Fork plus loin de Kuzu upstream |
| Validation at parse time | Chaque nouvelle option = modification de la grammaire |
| | Très lourd à implémenter et maintenir |

**Verdict** : trop lourd pour le bénéfice. On s'éloigne du Cypher standard sans gain majeur.

### Option B : Table function CATALOG_SEARCH (recommandée)

Une seule `TableFunc` haut niveau qui encapsule `Catalog::search()`.

```cypher
CALL CATALOG_SEARCH('ScopeKB', 'auth middleware',
    mode := 'detailed',
    filters := '{"scope_type": "function"}',
    signals := 'bm25|vector',
    limit := 10)
YIELD uuid, score, entity, data, chunks
```

| Avantage | Inconvénient |
|---|---|
| Réutilise l'infra table function existante | Options en string/JSON (pas type-safe au parsing) |
| Composable avec Cypher natif (MATCH, WHERE, JOIN) | Messages d'erreur moins précis que la grammaire native |
| Pas de modification du parser | |
| Pattern identique à QUERY_LUCIVY_INDEX | |
| LLM peut générer naturellement | |

**Verdict** : meilleur rapport effort/valeur. Implémentation relativement simple.

### Option C : Mini-DSL Rust (pest/nom)

Parser Rust léger pour un DSL textuel dédié.

```
search ScopeKB "auth middleware"
  where scope_type = "function"
  mode detailed
  signals bm25 | vector
  limit 10
  explore PARENT_OF, DEFINED_IN depth 2
```

| Avantage | Inconvénient |
|---|---|
| 100% Rust, léger | Encore un langage à apprendre |
| Exactement les features qu'on veut | Pas composable avec Cypher |
| Validation fine | Doublon conceptuel avec FilterCondition |

**Verdict** : intéressant pour un CLI ou une API REST, mais pas composable avec le graphe.

### Option D : JSON structuré

Pas de nouveau parser, juste serde :

```json
{
  "kb": "ScopeKB",
  "query": "auth middleware",
  "mode": "detailed",
  "filters": { "scope_type": { "$in": ["function", "method"] } },
  "signals": ["bm25", "vector"],
  "limit": 10
}
```

| Avantage | Inconvénient |
|---|---|
| Zéro parser à écrire | Pas du Cypher, pas composable |
| Facile pour les agents LLM | Verbose |
| serde_json fait tout | Pas de navigation graphe intégrée |

**Verdict** : utile comme format d'entrée pour une API REST, mais pas comme langage de query.

---

## 4. Design détaillé : CATALOG_SEARCH (Option B)

### 4.1 Signature

```
CATALOG_SEARCH(kb_name STRING, query STRING, [options...])
YIELD uuid, score, entity, data, chunks
```

**Paramètres nommés (tous optionnels) :**

| Paramètre | Type | Défaut | Description |
|---|---|---|---|
| `mode` | STRING | `'aggregated'` | `'aggregated'`, `'source_resolved'`, `'detailed'` |
| `limit` | INT64 | `10` | Nombre max de résultats |
| `offset` | INT64 | `0` | Pagination |
| `signals` | STRING | (config KB) | `'bm25'`, `'vector'`, `'bm25\|vector'`, `'bm25\|vector\|sparse'` |
| `filters` | STRING (JSON) | `'{}'` | Filtres au format JSON (FilterCondition sérialisé) |
| `bm25_mode` | STRING | `'contains'` | `'contains'`, `'contains_split'`, `'regex'`, `'parse'` |
| `fuzzy_distance` | INT64 | `1` | Distance Levenshtein |

**Colonnes YIELD :**

| Colonne | Type | Description |
|---|---|---|
| `uuid` | STRING | UUID du résultat (index entry ou entité source selon mode) |
| `score` | DOUBLE | Score fusionné |
| `entity` | STRING | Type d'entité (`"ScopeKB_Index"` ou `"Scope"` selon mode) |
| `data` | STRING (JSON) | Données de l'entité (sérialisé JSON car types hétérogènes) |
| `chunks` | STRING (JSON) | Chunks attribués en mode detailed, `null` sinon |

**Note sur `data` et `chunks`** : rag3db supporte les types `MAP` et `STRUCT`, mais les données des entités ont des schémas variables selon le type d'entité. Le JSON est le format le plus flexible ici. Alternative future : utiliser `STRUCT` avec des colonnes dynamiques si rag3db le supporte.

### 4.2 Exemples d'usage

#### Recherche simple

```cypher
CALL CATALOG_SEARCH('ScopeKB', 'auth middleware')
YIELD uuid, score, entity, data
RETURN uuid, score, entity, data
ORDER BY score DESC
```

#### Recherche + navigation graphe

```cypher
-- "Fonctions d'auth dans des fichiers TypeScript du dossier src/api/"
CALL CATALOG_SEARCH('ScopeKB', 'authentication handler',
    mode := 'source_resolved',
    filters := '{"scope_type": {"$in": ["function", "method"]}}')
YIELD uuid, score, data
MATCH (s:Scope {_uuid: uuid})-[:DEFINED_IN]->(f:File)
MATCH (d:Directory)-[:HAS_FILE]->(f)
WHERE f.extension = '.ts'
  AND d.absolute_path STARTS WITH '/repo/src/api/'
RETURN s.name, s.signature, score, f.path
ORDER BY score DESC
LIMIT 5
```

#### Analyse de dépendances

```cypher
-- "Quelles libraries utilisent les fonctions qui matchent 'database'?"
CALL CATALOG_SEARCH('ScopeKB', 'database connection', limit := 20)
YIELD uuid
MATCH (s:Scope {_uuid: uuid})-[:USES_LIBRARY]->(l:Library)
RETURN l.name, count(*) AS usage_count
ORDER BY usage_count DESC
```

#### Cross-KB (deux recherches enchaînées)

```cypher
-- "Fichiers qui parlent d'auth ET contiennent des fonctions d'auth"
CALL CATALOG_SEARCH('ScopeKB', 'authentication', limit := 10)
YIELD uuid AS scope_uuid, score AS scope_score
MATCH (s:Scope {_uuid: scope_uuid})-[:DEFINED_IN]->(f:File)
RETURN f.path, scope_score, s.name
ORDER BY scope_score DESC
```

#### Résultats détaillés avec chunks

```cypher
CALL CATALOG_SEARCH('ScopeKB', 'JWT token validation',
    mode := 'detailed', limit := 5)
YIELD uuid, score, chunks
RETURN uuid, score, chunks
```

Le client parse le JSON `chunks` pour obtenir les `AttributedChunk` avec `sourceEntity`, `sourceField`, etc.

### 4.3 Implémentation

#### Côté C++ (extension rag3weaver)

```
extension/rag3weaver_ext/src/
├── function/
│   ├── catalog_search.cpp      ← TableFunc CATALOG_SEARCH
│   └── catalog_search.h
```

La table function :
1. Parse les paramètres nommés
2. Convertit le JSON `filters` en `FilterCondition`
3. Construit un `SearchOptions`
4. Appelle `Catalog::search()` (via FFI Rust→C++ ou via le pont cxx existant)
5. Sérialise les résultats en colonnes tabulaires

#### Pont Rust ↔ C++

Deux approches possibles :

**A. Via la connection DB existante :**
La table function exécute le search via des queries internes (le même chemin que `Catalog::search()` fait avec `QUERY_LUCIVY_INDEX` + `VECTOR_SEARCH` + fusion). Pas de nouveau FFI.

**B. Via un nouveau pont cxx :**
Exposer `Catalog::search()` directement en C++ via cxx. Plus propre mais plus de travail de bridge.

L'approche A est plus pragmatique — CATALOG_SEARCH serait essentiellement un "macro" qui génère et exécute les queries de bas niveau en interne.

### 4.4 Schéma d'exécution

```
CALL CATALOG_SEARCH('ScopeKB', 'auth', mode := 'detailed', ...)
    │
    ↓ (C++ TableFunc)
    │
    ├── Parse params → SearchOptions
    │
    ├── Résolution filtres → allowed_ids
    │   (MATCH title_entity WHERE ... → OFFSET(id(idx)))
    │
    ├── BM25 search (QUERY_LUCIVY_INDEX interne)
    ├── Vector search (HNSW interne)
    ├── Sparse search (SPARSE_SEARCH interne)
    │
    ├── Fusion (RRF / Weighted)
    │
    ├── ResultMode resolution
    │   ├── Aggregated: tel quel
    │   ├── SourceResolved: fetch source entities
    │   └── Detailed: collect all chunks + attribution
    │
    └── YIELD uuid, score, entity, data, chunks
            │
            ↓ (retour au Cypher engine)
            │
            MATCH (s:Scope {_uuid: uuid}) ...
            WHERE ...
            RETURN ...
```

---

## 5. Cas d'usage avancés rendus possibles

### 5.1 Agent LLM : génération de queries

Un agent peut générer du Cypher avec CATALOG_SEARCH sans code custom :

```
User: "Trouve les fonctions d'authentification qui utilisent JWT dans le projet"

Agent generates:
CALL CATALOG_SEARCH('ScopeKB', 'authentication JWT',
    mode := 'source_resolved',
    filters := '{"scope_type": {"$in": ["function", "method"]}}',
    limit := 10)
YIELD uuid, score, data
MATCH (s:Scope {_uuid: uuid})-[:USES_LIBRARY]->(l:Library)
WHERE l.name CONTAINS 'jwt'
RETURN json_extract(data, '$.name') AS name,
       json_extract(data, '$.signature') AS signature,
       score, l.name AS library
ORDER BY score DESC
```

L'agent n'a besoin de connaître que :
1. Les noms des KBs
2. Les noms des entités et relations
3. La syntaxe CATALOG_SEARCH

### 5.2 Dashboard / API REST

Un endpoint REST peut accepter du Cypher brut avec CATALOG_SEARCH :

```
POST /api/query
{
  "cypher": "CALL CATALOG_SEARCH('ProductKB', $query, mode := 'source_resolved', filters := $filters) YIELD uuid, score, data RETURN *",
  "params": {
    "query": "chaussures running",
    "filters": "{\"price_max\": {\"$lte\": 100}}"
  }
}
```

### 5.3 Agrégations sur les résultats de search

```cypher
-- Distribution des types de scope dans les résultats
CALL CATALOG_SEARCH('ScopeKB', 'error handling', limit := 50)
YIELD uuid
MATCH (s:Scope {_uuid: uuid})
RETURN s.scopeType, count(*) AS cnt
ORDER BY cnt DESC
```

```cypher
-- Fichiers les plus "denses" en résultats pour une query
CALL CATALOG_SEARCH('ScopeKB', 'database', limit := 100)
YIELD uuid
MATCH (s:Scope {_uuid: uuid})-[:DEFINED_IN]->(f:File)
RETURN f.path, count(*) AS matches
ORDER BY matches DESC
LIMIT 10
```

### 5.4 Comparaison cross-KB

```cypher
-- Résultats ScopeKB vs résultats dans les docstrings
CALL CATALOG_SEARCH('ScopeKB', 'authentication', limit := 10)
YIELD uuid AS scope_uuid, score AS scope_score
MATCH (s:Scope {_uuid: scope_uuid})
RETURN 'code' AS source, s.name, scope_score
UNION ALL
CALL CATALOG_SEARCH('LibraryKB', 'authentication', limit := 5)
YIELD uuid AS lib_uuid, score AS lib_score
MATCH (l:Library {_uuid: lib_uuid})
RETURN 'library' AS source, l.name, lib_score
```

---

## 6. Relation avec les autres docs

| Doc | Lien |
|---|---|
| **Doc 01 (ResultMode)** | Le paramètre `mode` de CATALOG_SEARCH mappe directement sur `ResultMode` enum |
| **Doc 02 (Abstractions)** | CATALOG_SEARCH élimine le besoin de hooks L5 ET donne la composabilité graphe |
| **Doc 03 abstractions (cross-KB search)** | Le pattern UNION ALL ci-dessus est une implémentation de cross-KB search via Cypher natif, sans API dédiée `search_across_kbs()` |

---

## 7. Effort et priorité

### Prérequis

1. **ResultMode** (task #105) — CATALOG_SEARCH en dépend pour le paramètre `mode`
2. **_source_entity + _source_uuid sur chunks** — nécessaire pour le mode detailed

### Estimation

| Composant | Effort | Fichiers |
|---|---|---|
| TableFunc C++ `CATALOG_SEARCH` | Moyen | `catalog_search.cpp/h` |
| Pont vers `Catalog::search()` | Petit (via queries internes) ou Moyen (via cxx) | Bridge existant ou nouveau |
| Sérialisation résultats → colonnes | Petit | Dans la TableFunc |
| JSON `data` extraction | Petit | Utiliser l'extension JSON existante de rag3db |
| Tests | Moyen | GTest + E2E rag3weaver |

**Priorité** : basse. C'est un bonus d'ergonomie, pas un bloquant. L'API Rust programmatique couvre tous les cas fonctionnels. CATALOG_SEARCH ajoute la composabilité Cypher et l'accessibilité pour les agents LLM.

### Ordre dans la roadmap

```
Phase 1 : ResultMode (task #105)                    ← en cours
Phase 2 : Domain Adapter Code (task #95)             ← priorité 1
Phase 3 : CATALOG_SEARCH table function              ← ce doc
Phase 4 : Améliorations (boosting, cross-KB natif)   ← futur
```

---

## 8. Questions ouvertes

1. **Format du `data` YIELD** : JSON string ou type MAP rag3db ? Le JSON est plus flexible (schéma variable par entité) mais nécessite `json_extract()` pour accéder aux champs. Le MAP serait plus naturel en Cypher mais impose un schéma fixe.

2. **YIELD dynamique** : pourrait-on permettre `YIELD uuid, score, name, signature` directement (colonnes de l'entité source) au lieu de tout mettre dans `data` ? Nécessiterait un YIELD dynamique basé sur le schéma de l'entité — possible mais complexe côté planner.

3. **Explore intégré** : faut-il un paramètre `explore_depth` + `explore_relations` dans CATALOG_SEARCH, ou laisser l'utilisateur faire le traversal en Cypher natif (MATCH après YIELD) ? Le Cypher natif est plus flexible et explicite — probablement suffisant.

4. **Caching** : si la même CATALOG_SEARCH est appelée dans plusieurs branches d'un UNION, les résultats sont-ils cachés ? Probablement pas par défaut — à voir si c'est un problème en pratique.

5. **Transactions** : CATALOG_SEARCH lit des index Lucivy + HNSW qui vivent en dehors du moteur transactionnel de rag3db. Le lazy commit (`flushIfDirty()`) garantit la cohérence lecture-après-écriture au sein d'une session, mais pas l'isolation entre sessions concurrentes. C'est le comportement actuel de toutes les search functions — CATALOG_SEARCH hérite du même modèle.
