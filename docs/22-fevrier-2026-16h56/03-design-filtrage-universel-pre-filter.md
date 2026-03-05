# 03 — Design : Filtrage universel pré-filter

## Contexte

On a implémenté (02) les filtres avancés Qdrant-like dans rag3weaver :
- `FilterOp` étendu (IsNull, Between, StartsWith, Contains, NotIn, ValuesCount, etc.)
- `FilterCondition` : composition Must/Should/MustNot récursive
- `FilterBuilder` : API ergonomique
- Branchement dans `catalog.rs::search()` → Cypher WHERE

**Problème** : pour BM25, le filtrage est en post-filter (Lucivy retourne K résultats, on filtre après → perte de résultats). Pour vector, c'est déjà du pré-filter (WHERE Cypher).

## Décision : tout pré-filter, zéro post-filter

### Principe

Chaque filtre est résolu **avant** le scoring, jamais après. Deux mécanismes complémentaires :

1. **Lucivy-natif** : les ops scalaires sont compilés en `FilterClause` JSON, injectés dans `QueryConfig.filters`. Élagage au niveau segment, le plus rapide.

2. **allowed_ids** : les ops impossibles en Lucivy (listes Kuzu, null, cross-entity) sont résolus par un MATCH Cypher → liste d'IDs → passés au `FilterCollector` via `search_filtered_with_highlights`. HashSet O(1) par doc.

### Split des ops

| Catégorie | Ops | Cible |
|---|---|---|
| Scalaire comparaison | Eq, Neq, Lt, Lte, Gt, Gte, Between | Lucivy FilterClause |
| Scalaire ensemble | In, NotIn | Lucivy FilterClause |
| Texte | StartsWith, Contains | Lucivy FilterClause (RegexQuery / prefix) |
| Composition | Must, Should, MustNot | Lucivy BooleanQuery (sur les clauses Lucivy-compatibles) |
| Liste Kuzu | HasAny, HasAll, HasNone, IsEmpty, IsNotEmpty, ValuesCount | Cypher → allowed_ids |
| Null | IsNull, IsNotNull | Cypher → allowed_ids |
| Cross-entity | Author.name = "John" | Cypher MATCH → allowed_ids |

### Flow BM25

```
FilterCondition
      │
      split()
      ├─── lucivy_part ──► FilterClause JSON ──► QueryConfig.filters
      │                                               │
      └─── kuzu_part ──► MATCH Cypher ──► [id1, id2, ...]
                                               │
                                               ▼
                              search_filtered_with_highlights(
                                  query_json + lucivy_filters,
                                  limit,
                                  allowed_ids
                              )
```

### Flow Vector

```
FilterCondition
      │
      to_cypher()  ──► WHERE clause + params
      │
      ▼
  MATCH (n:Entity) WHERE n.embedding IS NOT NULL AND {filters}
  WITH n, array_cosine_similarity(...) AS sim
  ORDER BY sim DESC LIMIT K
  RETURN n._uuid, sim
```

Pas de changement pour vector : tout passe par Cypher WHERE, c'est déjà pré-filter.

### Flow mixte (kuzu_part non vide + BM25)

```cypher
-- Étape 1 : résoudre les IDs via Kuzu
MATCH (n:Document) WHERE size(n.tags) > 0 AND n.deleted_at IS NULL
RETURN id(n) AS nid

-- Étape 2 : Lucivy avec allowed_ids + FilterClause natifs
search_filtered_with_highlights(
    '{"type":"contains", "field":"body", "value":"rust", "filters":[{"field":"status","op":"eq","value":"active"}]}',
    100,
    [nid1, nid2, ...]
)
```

## Choix architecturaux

### 1. Indexer tous les champs dans Lucivy

Actuellement seuls `title_for`/`content_for` d'une KB sont indexés. On indexe désormais **tous les champs** de l'entité :
- `FieldType::Text` → champ texte Lucivy (stemmed + raw + ngram)
- `FieldType::String` → champ string Lucivy (exact match, pas de tokenisation)
- `FieldType::Int64` / `Float64` / etc. → filter fields (déjà fait)

Avantage : `StartsWith("Dr.")` / `Contains("important")` sur n'importe quel champ texte → requête Lucivy native au lieu de scan Cypher.

### 2. FilterCompiler : deux backends, un type

`FilterCondition` reste le type utilisateur unique. Deux méthodes de compilation :

```rust
impl FilterCompiler {
    /// Sépare en (lucivy_compatible, kuzu_only)
    fn split(condition: &FilterCondition) -> (FilterCondition, FilterCondition);

    /// Compile en FilterClause JSON pour Lucivy
    fn to_lucivy_json(condition: &FilterCondition) -> Vec<serde_json::Value>;

    /// Compile en Cypher WHERE + params (existant, refactoré depuis FilterParser)
    fn to_cypher(condition: &FilterCondition, ...) -> ParsedFilter;
}
```

### 3. Seuil allowed_ids

Si le filtre Kuzu est très permissif (retourne > N IDs), on peut :
- Option A : skip le filtre et laisser Lucivy scorer tout (comme Qdrant fait avec les filtres à haute cardinalité)
- Option B : toujours filtrer (HashSet reste O(1), c'est la taille du transfert qui coûte)

Décision : **Option B pour l'instant** (toujours filtrer). On optimisera si profiling montre un bottleneck.

### 4. Pas de double compilation pour FilterCondition mixtes

Si un `Must([scalar_op, list_op])`, le split produit :
- lucivy_part = `Must([scalar_op])`
- kuzu_part = `Must([list_op])`

Les deux sont appliqués en pré-filter chacun de leur côté. Le résultat est l'intersection (ce qui est correct pour Must/AND).

Pour `Should([scalar_op, list_op])` c'est plus délicat : on ne peut pas splitter un OR entre deux systèmes. Dans ce cas, tout le `Should` tombe en kuzu_part (Cypher → allowed_ids). C'est correct mais moins optimal. Acceptable en V1.

## Fichiers impactés

| Fichier | Changement |
|---|---|
| `ld-lucivy/lucivy_fts/rust/src/query.rs` | Ajouter ops between, not_in, starts_with, contains à build_filter_clause(). Ajouter composition must/should/must_not sur FilterClause. |
| `rag3weaver/src/filter.rs` | Ajouter FilterCompiler avec split() + to_lucivy_json(). Refactorer to_cypher() depuis FilterParser. |
| `rag3weaver/src/search.rs` | Modifier search_bm25() pour injecter lucivy_filters dans le JSON et passer allowed_ids. |
| `rag3weaver/src/catalog.rs` | Orchestrer : split → pré-résolution Cypher → appels search. |
| `extension/lucivy_fts/src/function/create_lucivy_index.cpp` | Indexer tous les champs de l'entité (pas seulement content_for/title_for). |

## Ce qu'on ne fait PAS (V1)

- Pas de vector search via Lucivy (reste Cypher cosine similarity)
- Pas de seuil adaptatif sur allowed_ids
- Pas d'optimisation Should mixte (tout tombe en Cypher si un seul op n'est pas Lucivy-compatible)
- Pas de cache de résultats de pré-résolution Kuzu
