# Doc 19 — Findings : Simplification filtres + ResultMode

**Date** : 3 mars 2026
**Branche** : `feature/kb-index-architecture`

---

## Contexte

Analyse approfondie du flow de search/filtres en vue de deux changements :
1. Supprimer `FilterCompiler::split()` — tout router via `allowed_ids`
2. Ajouter `ResultMode` (aggregated vs source-resolved)

Ce document capture tous les findings de l'exploration pour que la prochaine session d'implémentation soit efficace.

---

## 1. Architecture actuelle du search — flow complet

### Entrée : `Catalog::search()` (`catalog.rs:1057-1370`)

```
search(kb_name, query, SearchOptions) →
  1. Consistency check (drain/flush si nécessaire)
  2. Résoudre entity = "{KB}_Index", vector_entity = "{KB}_Index_Chunk"
  3. BM25 fields fixés : ["_title", "_content"]
  4. Parser les filtres (dual-path, voir section 2)
  5. Embedder la query (dense/sparse/dual selon signals)
  6. Lancer les searches en parallèle (BM25, vector, sparse)
  7. Résoudre chunks → parents (resolve_vector_chunks)
  8. Fusionner (RRF ou weighted)
  9. Paginer
  10. Enrichir les résultats manquants
  11. Retourner SearchResponse
```

### Variables clés

- `entity` = `"{kb_name}_Index"` (ligne 1097) — table cible pour BM25/FTS
- `vector_entity` = `"{kb_name}_Index_Chunk"` (ligne 1098) — table cible pour HNSW/sparse
- `bm25_fields` = `["_title", "_content"]` (ligne 1100) — toujours ces deux champs
- `is_chunked` = `true` (ligne 1191) — toujours true car toute KB a des Index_Chunk
- `enrich_fields` = `["_title", "_content", "_source_entity", "_source_uuid", "_content_hash"]` (ligne 1194)

---

## 2. Dual-path filtres actuel (à supprimer)

### Path A : Vector search (`catalog.rs:1112-1130`)

Tous les filtres → `FilterParser::parse_condition(cond, entity="TreeKB_Index", alias="n")` → Cypher WHERE.
Passé à `search_vector()` comme `extra_where`, `extra_params`, `extra_match`.
Le vector search utilise `PROJECT_GRAPH_CYPHER` pour créer un sous-graphe filtré puis HNSW dessus.

### Path B : BM25 search (`catalog.rs:1133-1188`)

`FilterCompiler::split(cond)` sépare en deux :

**Split lucivy** : conditions simples (Eq, Lt, Gt, In, Between, Contains, StartsWith, Neq, NotIn) sur champs non-cross-entity → JSON `FilterClause` array injecté dans la query Lucivy.

**Split kuzu** : conditions complexes (IsNull, cross-entity avec ".", HasAny/HasAll) → Cypher `MATCH (n:{entity}) WHERE ... RETURN OFFSET(id(n))` → `allowed_ids` passés à Lucivy.

### Problème

Le split lucivy route des champs comme `page_count`, `status` vers Lucivy. Mais l'index FTS est sur `{KB}_Index` qui n'a PAS ces colonnes — il n'a que `_title`, `_content`, `_source_entity`. Donc si un filtre `page_count > 10` arrive, Lucivy cherche un field inexistant → crash potentiel.

### La solution : tout passer par allowed_ids

Puisque `allowed_ids` fonctionne déjà comme fallback pour tous les filtres, on supprime le split et on route tout par ce chemin. Le seul "coût" est 1 round-trip Kuzu en plus, ce qui est négligeable (<1ms pour des volumes réalistes).

---

## 3. FilterCompiler — code à supprimer

**Fichier** : `src/filter.rs`

### Structs/fonctions à supprimer

| Élément | Lignes | Description |
|---------|--------|-------------|
| `SplitResult` struct | 808-815 | Résultat du split (lucivy + kuzu) |
| `FilterCompiler::is_lucivy_op()` | 824-838 | Classifie les ops comme Lucivy-compatible |
| `FilterCompiler::is_cross_entity()` | 842-844 | Détecte les clés avec "." |
| `FilterCompiler::is_field_lucivy()` | 847-856 | Classifie un champ complet |
| `FilterCompiler::split()` | 865-912 | Le split principal |
| `FilterCompiler::is_all_lucivy()` | 916-925 | Check récursif |
| `FilterCompiler::wrap_must()` | 927-933 | Helper pour rewrap |
| `FilterCompiler::to_lucivy_json()` | 939-1042 | Convertit en JSON FilterClause |
| `FilterCompiler::field_to_json()` | 980-1015 | Convertit un champ en JSON |
| `cypher_to_json()` | 1044-1060 | Helper CypherValue → serde_json::Value |
| Tests associés | ~1100+ | Tests du split et du to_lucivy_json |

**Note** : vérifier si `FilterCompiler` a d'autres méthodes utiles avant de supprimer le struct entièrement. Si c'est le cas, garder le struct et ne supprimer que les méthodes listées.

### Exports à nettoyer

`lib.rs:45` : `pub use filter::{FilterBuilder, FilterCompiler, FilterCondition, FilterOp, FilterParser, FilterValue, ParsedFilter, SplitResult};`
→ retirer `SplitResult`, potentiellement `FilterCompiler` si vide.

---

## 4. search.rs — signatures à nettoyer

### `build_bm25_query()` (~ligne 1207-1258)

Actuellement accepte `lucivy_filters: Option<&[serde_json::Value]>` et injecte dans le JSON query :
```rust
if let Some(filters) = lucivy_filters {
    obj["filters"] = serde_json::json!(filters);
}
```
→ Supprimer le paramètre et cette injection.

### `search_bm25()` (~ligne 1300-1363)

Signature actuelle :
```rust
pub async fn search_bm25(
    conn, entity, query, fields, mode, fuzzy_distance, limit,
    lucivy_filters: Option<&[serde_json::Value]>,  // ← supprimer
    allowed_ids: Option<&[u64]>,                     // ← garder
    return_fields,
)
```

### `search_bm25_chunked()` (~ligne 1625-1759)

Même changement : retirer `lucivy_filters`.

### `search_bm25_raw()` — vérifier aussi

Potentiellement une version bas-niveau qui passe les filtres. À vérifier.

---

## 5. Nouveau flow filtres — résolution via title entity

### Le Cypher cible

Au lieu de résoudre les filtres sur `{KB}_Index` (qui n'a pas les champs entité), on résout sur la **title entity** puis on JOINe vers l'index :

```cypher
-- Filtre simple sur la title entity
MATCH (t:Directory)-[:Directory_IN_TreeKB]->(idx:TreeKB_Index)
WHERE t.depth > 2
RETURN OFFSET(id(idx))

-- Filtre cross-entity
MATCH (t:Directory)-[:Directory_IN_TreeKB]->(idx:TreeKB_Index)
MATCH (t)-[:HAS_FILE]->(f:File)
WHERE f.extension = '.ts'
RETURN OFFSET(id(idx))
```

### Comment construire ce Cypher

1. Récupérer `kb_meta.title.entity` → ex: "Directory"
2. Construire `in_rel` = `"{title_entity}_IN_{kb_name}"` → ex: "Directory_IN_TreeKB"
3. Appeler `FilterParser::parse_condition(cond, title_entity, "t")` — le parser résout les champs sur Directory, et les cross-entity via les relations du graph
4. Assembler : `MATCH (t:{title_entity})-[:{in_rel}]->(idx:{index_table}) {parsed.match_clauses} WHERE {parsed.where_clauses} RETURN OFFSET(id(idx))`
5. Exécuter → `Vec<u64>` d'offsets

### Infos disponibles dans KBMetadata

```rust
pub struct KBMetadata {           // catalog.rs:34
    pub name: String,
    pub title: KBFieldRef,        // { entity: "Directory", field: "name" }
    pub content: Vec<KBFieldRef>, // [{ entity: "Directory", field: "absolute_path" }, ...]
    pub entities: HashSet<String>,// {"Directory", "File"}
    pub signals: SearchSignals,
    pub keyword_weight: f64,
    pub title_boost: f64,
    pub content_boost: f64,
    pub chunking: ChunkingConfig,
}

pub struct KBFieldRef {           // validator.rs:19
    pub entity: String,
    pub field: String,
}
```

### Cas spécial : `_source_entity`

`_source_entity` est un champ de `{KB}_Index`, pas de la title entity. Si un filtre contient `_source_entity = "File"`, il faut l'appliquer sur `idx`, pas sur `t`.

**Approche proposée** : avant le FilterParser, scanner le condition tree pour extraire les champs qui commencent par `_` (champs système de l'index). Les appliquer comme `AND idx._source_entity = $val` dans le Cypher final. Passer le reste au FilterParser normalement.

---

## 6. FilterParser — comment il fonctionne

### Entrée

```rust
FilterParser::parse_condition(
    condition: &FilterCondition,
    result_entity: &str,   // ex: "Directory" (la title entity)
    result_alias: &str,    // ex: "t"
) -> Result<ParsedFilter, FilterError>
```

### Résolution des champs

- **Sans "."** : `depth` → résolu sur `result_entity` (Directory) → `WHERE t.depth > 2`
- **Avec "."** : `File.extension` → lookup relation entre Directory et File → génère `MATCH (t)-[:HAS_FILE]->(e1:File)` + `WHERE e1.extension = '.ts'`

### Sortie

```rust
pub struct ParsedFilter {
    pub where_clauses: Vec<String>,      // ["t.depth > $filter_p0"]
    pub match_clauses: Vec<String>,      // ["MATCH (t)-[:HAS_FILE]->(e1:File)"]
    pub params: Vec<QueryParam>,         // [QueryParam { name: "filter_p0", value: 2 }]
    pub aliases: HashMap<String, String>,
}

impl ParsedFilter {
    pub fn combine_where(&self) -> String {
        self.where_clauses.join(" AND ")
    }
}
```

### Relations disponibles

Le parser a accès à `self.relations: &HashMap<String, RelationDef>` (passé au constructeur). Il peut trouver les relations entre n'importe quelles entités du schema.

---

## 7. ResultMode — design détaillé

### SearchResult actuel

```rust
pub struct SearchResult {      // search.rs:279
    pub uuid: String,          // UUID du {KB}_Index entry
    pub score: f64,
    pub entity: Option<String>,// "{KB}_Index"
    pub data: Option<BTreeMap<String, CypherValue>>,
    pub chunk: Option<ChunkInfo>,
}
```

### Mode Aggregated (défaut, actuel)

- `uuid` = UUID de `{KB}_Index` entry
- `entity` = `"{KB}_Index"`
- `data` = `{ _title, _content, _source_entity, _source_uuid, _content_hash }`
- `chunk` = meilleur chunk matché (si chunked search)

### Mode SourceResolved (nouveau)

- `uuid` = `_source_uuid` (UUID de l'entité source originale)
- `entity` = `_source_entity` (ex: "Directory", "File")
- `data` = champs de l'entité source (récupérés via MATCH)
- `chunk` = même chunk, mais les offsets sont relatifs au champ source

### Implémentation : `resolve_to_source_entities()`

Appelée après fusion + pagination dans `search()` :

1. Pour chaque résultat, lire `_source_entity` et `_source_uuid` depuis `data`
2. Grouper par entity type : `HashMap<String, Vec<String>>` (entity_name → [uuids])
3. Pour chaque groupe, query batch :
   ```cypher
   MATCH (n:Directory) WHERE n._uuid IN ['dir-123', 'dir-456'] RETURN n
   ```
4. Remplacer uuid/entity/data dans les résultats
5. Dédupliquer : si plusieurs index entries pointent vers la même source (ex: même Directory via des KBs différents), garder le meilleur score

### Déduplications possibles

Dans TreeKB, chaque Directory a **un seul** index entry (1:1 via `{TitleEntity}_IN_{KB}`), donc pas de doublon pour une KB donnée. Mais si on cherche cross-KB un jour, il pourrait y en avoir.

---

## 8. Impact sur les tests E2E existants

### Tests qui utilisent search avec filtres

Aucun des 14 tests E2E phase0b ne teste les filtres sur search. Les filtres sont testés dans les tests unitaires de `filter.rs` et dans `e2e_search.rs`.

### Tests à vérifier après le changement

- `e2e_search.rs` : tous les tests qui utilisent `filter_condition` ou `filters` dans SearchOptions
- Les tests unitaires de `FilterCompiler` dans `filter.rs` → à supprimer avec le code

### Nouveaux tests à ajouter

- Test E2E : search TreeKB avec filtre `depth > 1` (title entity filter)
- Test E2E : search TreeKB avec filtre `_source_entity = "File"` (index filter)
- Test E2E : search TreeKB avec `ResultMode::SourceResolved` → vérifie entity types + données sources
- Test unitaire : `resolve_to_source_entities()` avec mix Directory + File

---

## 9. Ordre d'implémentation recommandé

```
1. catalog.rs : remplacer le bloc dual-path (lignes 1133-1188) par single allowed_ids
2. search.rs : retirer lucivy_filters des signatures (search_bm25, search_bm25_chunked, build_bm25_query)
3. filter.rs : supprimer FilterCompiler::split/to_lucivy_json + SplitResult + tests
4. lib.rs : nettoyer exports
5. cargo test --lib → vérifier compilation + tests unitaires
6. search.rs : ajouter ResultMode enum + SearchOptions.result_mode
7. catalog.rs : ajouter resolve_to_source_entities() + appel dans search()
8. tests E2E : ajouter tests filtres + ResultMode
9. ./run_e2e.sh --test e2e_phase0b → vérifier non-régression
```

---

## 10. Points d'attention

1. **FilterParser et `{KB}_Index` fields** : le parser ne connaît pas les champs système de l'index (`_source_entity`, `_title`, etc.). Il faut les extraire avant de passer au parser, ou les traiter comme un cas spécial.

2. **Vector search filter path** : le vector search utilise déjà un chemin pur Kuzu (`extra_where` + `PROJECT_GRAPH_CYPHER`). Actuellement il résout sur `entity = "{KB}_Index"`. Il faut aussi le router sur la title entity pour que les filtres marchent.

3. **Sparse search** : `search_sparse_cypher()` n'accepte pas de filtres actuellement. Si on veut des filtres sur sparse, il faudra ajouter `allowed_ids` (ou un pré-filtre Kupher).

4. **Performance allowed_ids avec beaucoup d'IDs** : si une KB a 100K index entries et le filtre en exclut peu, le `allowed_ids` array sera énorme. Lucivy utilise un bitset en interne donc c'est OK côté Lucivy, mais le Cypher `RETURN OFFSET(id(idx))` produit 100K lignes. Pas un problème immédiat mais à surveiller.

5. **Champs "body" sans titleFor/contentFor** : dans l'ancien schéma, un champ `body: Text` sans annotation titleFor/contentFor était indexé dans Lucivy quand même. Dans le nouveau schéma, seuls `_title` et `_content` sont indexés. Les champs non-annotés ne sont pas searchables — c'est un changement de comportement voulu mais à documenter.
