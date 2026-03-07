# Doc 02 — Design : Abstractions cross-domain et cas L5 Code RAG

**Date** : 3 mars 2026
**Branche** : `feature/kb-index-architecture`
**Dépend de** : Doc 01 (ResultMode)

---

## 1. Problème

L'expérience L5 JS (`kuzu-wasm-exp/l5/code-rag/`) a validé le pipeline complet Code RAG (parse → ingest → search → enrich). Mais l'enrichissement post-search repose sur des **hooks JS domaine-spécifiques** :

```js
// hooks.js — L5
async function enrichCodeResult(result, context) {
  result.node = await catalog._fetchNodeDetails('Scope', result._uuid);           // 1
  result.relevantChildren = await catalog.searchRelated(uuid, 'PARENT_OF', ...);  // 2
  result.relevantChunks = await catalog.getRelevantChunks(uuid, query, ...);      // 3
}
```

Ces 3 opérations sont des **workarounds** pour des limitations du search actuel :

| Hook L5 | Limitation actuelle | Solution rag3weaver |
|---|---|---|
| `_fetchNodeDetails()` | Le résultat contient `{KB}_Index` data, pas l'entité source | **ResultMode::SourceResolved** |
| `getRelevantChunks()` | Un seul chunk (best) par résultat | **ResultMode::Detailed** |
| `searchRelated()` | Pas de navigation graphe post-search | **Explore** (déjà implémenté) |

**Question centrale** : peut-on éliminer les hooks domaine-spécifiques et offrir les mêmes capacités via des mécanismes génériques ? Si oui, le même search fonctionne pour Code, Documents, Shopify, Gmail sans code custom.

---

## 2. Anatomie du L5 : ce qui est générique vs domaine-spécifique

### 2.1 Ce qui est purement domaine-spécifique (et doit le rester)

| Composant L5 | Rôle | Pourquoi domaine-spécifique |
|---|---|---|
| `codeparsersToEntities()` | Parse result → entités rag3weaver | Conversion de format, propre à codeparsers |
| `codeparsersRelationships()` | Parse result → relations | Idem |
| `enrichClassContent()` | Construit `member_summary` pour containers | Logique sémantique du code (quoi résumer, comment formater) |
| `CODE_SCHEMA` | Définition entités/relations/KBs | Propre au domaine Code |

**Principe** : la transformation des données sources en entités/relations est toujours domaine-spécifique. C'est le **domain adapter**. Rag3weaver ne doit pas en savoir quoi que ce soit.

### 2.2 Ce qui devrait être générique (et ne l'est pas encore dans L5)

| Composant L5 | Ce que rag3weaver fournit/fournira |
|---|---|
| `enrichCodeResult` (fetch node) | `ResultMode::SourceResolved` |
| `enrichCodeResult` (relevant chunks) | `ResultMode::Detailed` + `AttributedChunk` |
| `enrichCodeResult` (relevant children) | `Explore` (depth=1, relations=["PARENT_OF"]) |
| `CODE_SEARCH_PRESET` (boostIf) | `SearchOptions` : filtres + `_source_entity` (déjà possible via FilterCondition) |
| `CHUNK_SEARCH_PRESET` (returnChunks) | `ResultMode::Detailed` |

### 2.3 Interface cible

```
Domain Adapter (Code, Shopify, Documents...)
    │
    │  catalog.create("Scope", data)
    │  catalog.link("DEFINED_IN", scope_ref, file_ref)
    │  catalog.drain()
    │
    ↓
Rag3weaver (100% générique)
    │
    │  catalog.search("ScopeKB", query, {
    │      result_mode: Detailed,       // tous les chunks avec attribution
    │      filters: { "_source_entity": "Scope" },
    │      ...
    │  })
    │  catalog.search_with_explore("ScopeKB", query, {
    │      result_mode: SourceResolved, // entités sources dans les résultats
    │      explore: { depth: 1, relations: ["PARENT_OF", "DEFINED_IN"] },
    │  })
    │
    ↓
Client (UI, agent, API)
    │
    │  Formatage, groupement, affichage
    │  (aucune query DB supplémentaire)
```

---

## 3. Comment ResultMode remplace les hooks L5

### 3.1 Cas 1 : `_fetchNodeDetails` → SourceResolved

**L5 hook :**
```js
result.node = await catalog._fetchNodeDetails('Scope', result._uuid);
// result.node = { name: "parseProject", scopeType: "function", content: "...", ... }
```

**Rag3weaver avec SourceResolved :**
```rust
let response = catalog.search("ScopeKB", "scope extraction", SearchOptions {
    result_mode: ResultMode::SourceResolved,
    ..Default::default()
});
// result.uuid = "scope-123" (uuid du Scope, pas de l'index)
// result.entity = "Scope"
// result.data = { name: "parseProject", scopeType: "function", content: "...", ... }
```

Zéro query supplémentaire. L'entité source est directement dans le résultat.

### 3.2 Cas 2 : `getRelevantChunks` → Detailed

**L5 hook :**
```js
result.relevantChunks = await catalog.getRelevantChunks(uuid, query, { limit: 3 });
// relevantChunks = [{ text: "async function...", score: 0.88 }, ...]
```

**Rag3weaver avec Detailed :**
```rust
let response = catalog.search("ScopeKB", "auth middleware", SearchOptions {
    result_mode: ResultMode::Detailed,
    ..Default::default()
});
// result.chunks = [
//   { uuid: "c1", text: "async function...", score: 0.88,
//     source_entity: "Scope", source_uuid: "scope-42", source_field: "content" },
//   { uuid: "c2", text: "Handles JWT validation...", score: 0.75,
//     source_entity: "Scope", source_uuid: "scope-42", source_field: "docstring" },
// ]
```

Tous les chunks pertinents avec leur attribution, sans query supplémentaire. Le client groupe par `source_field` s'il veut séparer code vs docstring.

### 3.3 Cas 3 : `searchRelated` → Explore

**L5 hook :**
```js
result.relevantChildren = await catalog.searchRelated(uuid, 'PARENT_OF', query, { limit: 5 });
```

**Rag3weaver (déjà implémenté) :**
```rust
let response = catalog.search_with_explore("ScopeKB", "auth", ExploreOptions {
    search: SearchOptions { result_mode: ResultMode::SourceResolved, ..Default::default() },
    depth: 1,
    outgoing_relations: vec!["PARENT_OF".into()],
    incoming_relations: vec!["DEFINED_IN".into()],
    ..Default::default()
});
// response.results = [{ uuid: "scope-1", entity: "Scope", data: {...} }]
// response.graph.nodes = [scope-1, method-a, method-b, file-x]
// response.graph.edges = [scope-1→method-a (PARENT_OF), scope-1→method-b (PARENT_OF), ...]
```

---

## 4. Presets de recherche : de L5 JS à rag3weaver Rust

### 4.1 L5 presets (JS)

```js
const CODE_SEARCH_PRESET = {
  limit: 10,
  boostIf: { "scopeType IN ['class', 'interface', 'enum']": 0.8 },
};

const IMPLEMENTATION_SEARCH_PRESET = {
  limit: 10,
  boostIf: { "scopeType IN ['function', 'method']": 0.85 },
};
```

### 4.2 Équivalent rag3weaver

Les presets L5 combinent deux choses :
1. **Filtrage** : limiter par type d'entité → `FilterCondition`
2. **Boosting** : augmenter le score de certains types → pas encore dans rag3weaver

**Le filtrage existe déjà** grâce à la simplification all-allowed_ids (task #104). Tous les filtres passent par la title entity via Kuzu :

```rust
// "Que les fonctions et méthodes"
let options = SearchOptions {
    filter_condition: Some(FilterCondition::Or(vec![
        FilterCondition::Field {
            field: "scope_type".into(),
            op: FilterOp::Eq,
            value: FilterValue::String("function".into()),
        },
        FilterCondition::Field {
            field: "scope_type".into(),
            op: FilterOp::Eq,
            value: FilterValue::String("method".into()),
        },
    ])),
    ..Default::default()
};
```

**Le boosting conditionnel est une feature future.** Pour l'instant, le filtre strict suffit. Le boosting (modifier les scores sans exclure) nécessiterait un pass de reranking post-fusion — c'est le "graph-aware reranking" du doc 03, mais appliqué sur des propriétés d'entité plutôt que sur la topologie. C'est une optimisation, pas un bloquant.

### 4.3 Filtrage par `_source_entity`

Cas important pour les KBs cross-entity (TreeKB avec Directory + File) :

```rust
// "Que les fichiers dans TreeKB"
let options = SearchOptions {
    filter_condition: Some(FilterCondition::Field {
        field: "_source_entity".into(),
        op: FilterOp::Eq,
        value: FilterValue::String("File".into()),
    }),
    ..Default::default()
};
```

Ce filtre est résolu sur `{KB}_Index._source_entity` (colonne déjà existante sur l'index entry). Il passe par le chemin all-allowed_ids sans problème.

---

## 5. Le pattern Container : member_summary + chunks enfants

### 5.1 Le problème Container dans le Code Domain

Un container (class, interface, module) agrège du contenu de plusieurs sources :

```
Container "AuthService" (ScopeKB_Index entry)
├── _title: "class AuthService"
└── _content (concaténé):
    ├── [offset 0]     Scope "AuthService".content → "class AuthService { ... }"
    ├── [offset 500]   Scope "AuthService".docstring → "Service d'authentification JWT"
    ├── [offset 600]   Scope "AuthService".member_summary → "Members:\n  - login() ...\n  - verify() ..."
    └── [offset 800]   Scope "handleAuth".content → "async fn handleAuth() { ... }" (via PARENT_OF)
                        ↑ Ce dernier n'existe PAS aujourd'hui
```

**Aujourd'hui** : seuls les champs `contentFor` de l'entité titre et des entités liées **dans la config** contribuent au `_content`. Les enfants (méthodes) via `PARENT_OF` ne sont pas automatiquement agrégés — c'est pourquoi L5 construit un `member_summary` manuellement dans le domain adapter.

**Faut-il changer ça ?** Non. Le member_summary est la bonne approche :

| Approche | Avantages | Inconvénients |
|---|---|---|
| **Agrégation automatique via relations** | Zéro code domain | Explose le `_content` (tout le code de toutes les méthodes), chunks de mauvaise qualité, index entry énorme |
| **member_summary dans le domain adapter** | Résumé concis et pertinent, taille contrôlée, qualité chunks/embeddings | Logique dans le domain adapter, recalcul en incrémental |

Le member_summary est une **transformation sémantique** ("quels champs résumer, comment formater") qui appartient au domain adapter. Rag3weaver n'a pas à savoir ce qu'est un "member" ou une "signature".

### 5.2 Ce que ResultMode::Detailed apporte aux containers

Sans ResultMode::Detailed, l'appelant reçoit un container avec son best chunk (peut-être un bout de member_summary, peut-être un bout de content). Pas moyen de savoir d'où ça vient.

Avec Detailed :

```json
{
  "uuid": "idx-auth",
  "entity": "ScopeKB_Index",
  "score": 0.95,
  "data": { "_title": "class AuthService", "_source_entity": "Scope" },
  "chunks": [
    {
      "uuid": "c1", "text": "Service d'auth JWT, gère login et logout...",
      "score": 0.92,
      "sourceEntity": "Scope", "sourceUuid": "scope-auth", "sourceField": "docstring"
    },
    {
      "uuid": "c2", "text": "Members:\n  - login(email, password) (L16-30)\n  - verify(token) (L32-45)",
      "score": 0.88,
      "sourceEntity": "Scope", "sourceUuid": "scope-auth", "sourceField": "member_summary"
    },
    {
      "uuid": "c5", "text": "async fn handleAuth(req: Request) { let token = req.header(...) }",
      "score": 0.85,
      "sourceEntity": "Scope", "sourceUuid": "scope-auth", "sourceField": "content"
    }
  ]
}
```

Le client voit :
- `docstring` → description haut-niveau (chunk c1)
- `member_summary` → vue d'ensemble des membres (chunk c2)
- `content` → code source pertinent (chunk c5)

Il peut grouper par `sourceField`, prendre le top-2 de chaque, et présenter un résultat structuré — **sans connaître le domaine Code**.

### 5.3 Et si les méthodes enfants étaient dans des index entries séparées ?

Les scopes enfants (méthodes) sont aussi des entités Scope avec leurs propres index entries dans ScopeKB. Donc une recherche "handleAuth" les trouve directement, indépendamment du container parent.

Le container a son propre index entry avec `member_summary` comme signal additionnel. C'est complémentaire :
- Search "AuthService" → match container (via member_summary ou content)
- Search "handleAuth" → match méthode directement
- Search "auth JWT login" → match les deux, fusion choisit les plus pertinents

---

## 6. Cross-entity KB : le cas TreeKB et les futurs domaines

### 6.1 Rappel du problème

TreeKB agrège Directory + File. Aujourd'hui, le search ne query que la `title.entity` (Directory). Un File qui match ne sera trouvé que via les chunks qu'il contribue à l'index entry du Directory parent.

### 6.2 Architecture existante qui supporte déjà le multi-entity

Le schema rag3weaver supporte déjà `KBMetadata.entities: HashSet<String>`. L'AggregateProcessor collecte les contenus de toutes les entités contributrices. L'index Lucivy agrège tout dans un seul `{KB}_Index`.

Ce qui manque : le filtre de résolution. Quand on a un filtre `depth > 2`, il est résolu sur la title entity (Directory). Mais si on cherche `extension = ".ts"`, c'est un champ de File, pas de Directory. Le FilterParser avec la notation "." supporte ça (`File.extension`), mais il faut que l'entité File soit accessible via une relation depuis la title entity.

### 6.3 ResultMode et cross-entity

En mode **SourceResolved**, les résultats TreeKB montrent le Directory (title entity). Mais l'utilisateur cherche peut-être un File.

Solution : avec `_source_entity` sur l'index entry, le client sait si le match principal vient d'un Directory ou d'un File. En mode **Detailed**, les `AttributedChunk` indiquent exactement quelle entité a produit chaque chunk :

```json
{
  "uuid": "idx-src-auth",
  "entity": "TreeKB_Index",
  "data": { "_title": "auth", "_source_entity": "Directory" },
  "chunks": [
    { "sourceEntity": "Directory", "sourceField": "absolute_path",
      "text": "/repo/src/auth/" },
    { "sourceEntity": "File", "sourceField": "name",
      "text": "middleware.ts" },
    { "sourceEntity": "File", "sourceField": "absolute_path",
      "text": "/repo/src/auth/middleware.ts" }
  ]
}
```

Le client voit que le match vient à la fois du répertoire et de ses fichiers, sans connaître la structure TreeKB.

---

## 7. Mapping complet : L5 → rag3weaver natif

### 7.1 Pipeline d'ingestion

| Étape L5 (JS) | Rag3weaver Rust | Statut |
|---|---|---|
| `ProjectParser.parseProject()` | `codeparsers::ProjectParser::parse_project()` | Fait (crate Rust transpilé) |
| `codeparsersToEntities()` | Domain adapter Rust (à écrire) | A faire |
| `codeparsersRelationships()` | Domain adapter Rust (à écrire) | A faire |
| `enrichClassContent()` (member_summary) | Domain adapter Rust — `build_member_summary()` | A faire |
| `catalog.create()` / `catalog.relate()` / `catalog.drain()` | Identique : `catalog.create()` / `catalog.link()` / `catalog.drain()` | Fait |

### 7.2 Pipeline de recherche

| Étape L5 (JS) | Rag3weaver Rust | Statut |
|---|---|---|
| `catalog.search('ScopeKB', query)` | `catalog.search("ScopeKB", query, options)` | Fait |
| `onResultEnrich: enrichCodeResult` (fetch node) | `ResultMode::SourceResolved` | A faire (task #105) |
| `onResultEnrich: enrichCodeResult` (relevant chunks) | `ResultMode::Detailed` + `AttributedChunk` | A faire (task #105) |
| `onResultEnrich: enrichCodeResult` (children) | `catalog.search_with_explore()` | Fait |
| `CODE_SEARCH_PRESET` (boostIf) | `SearchOptions::filter_condition` (filtrage strict) | Fait (filtre, pas boost) |
| `catalog.searchWithExplore()` | `catalog.search_with_explore()` | Fait |

### 7.3 Ce qui reste à implémenter dans rag3weaver

| Feature | Fichiers | Effort | Bloquant pour L5 ? |
|---|---|---|---|
| `ResultMode` enum + `SearchOptions.result_mode` | `search.rs` | Petit | Oui |
| `_source_entity` + `_source_uuid` sur chunks | `schema.rs`, `catalog.rs` | 2 lignes chacun | Oui (Detailed) |
| `AttributedChunk` struct | `search.rs` | Petit | Oui (Detailed) |
| `SourceResolved` resolution dans `search()` | `catalog.rs` | Moyen | Non (client peut le faire) |
| `Detailed` multi-chunk resolution | `search.rs`, `catalog.rs` | Moyen | Non (client peut le faire) |
| Domain adapter Code (Rust) | Nouveau module | Moyen | Non (L5 JS fonctionne) |
| Boosting conditionnel (boostIf) | `search.rs` | Moyen | Non (filtre suffit) |

---

## 8. Généricité : même pattern pour tous les domaines

### 8.1 Mapping par domaine

| Domaine | Title entity | Content entities | Chunks sources multiples ? | Bénéfice Detailed |
|---|---|---|---|---|
| **Code (ScopeKB)** | Scope | Scope (content, docstring, member_summary) | Oui (3 champs) | Distinguer code vs doc vs résumé |
| **Code (TreeKB)** | Directory | Directory + File (paths, names) | Oui (2 entités) | Distinguer dir vs file match |
| **Documents** | Document | Section (content) | Oui (N sections) | Voir quelle section match |
| **Shopify** | Product | Product (description, tags) | Non (1 entité) | Pas critique |
| **Gmail** | Mail | Mail (body) + Attachment (content) | Oui (2 entités) | Distinguer body vs pièce jointe |
| **Notion** | Page | Page (content) + SubPage (content) | Oui (N niveaux) | Voir quelle sous-page match |

### 8.2 Le même search pour tous

```rust
// Code : "quelles fonctions gèrent l'authentification ?"
catalog.search("ScopeKB", "auth middleware", SearchOptions {
    result_mode: ResultMode::Detailed,
    filter_condition: Some(/* scope_type = "function" */),
    ..Default::default()
});

// Shopify : "chaussures running rouges pas chères"
catalog.search("ProductKB", "chaussures running rouges", SearchOptions {
    result_mode: ResultMode::SourceResolved,
    filter_condition: Some(/* price_max < 100 AND in_stock = true */),
    ..Default::default()
});

// Documents : "clauses de confidentialité"
catalog.search("DocumentKB", "confidentialité", SearchOptions {
    result_mode: ResultMode::Detailed,  // voir quelles sections matchent
    ..Default::default()
});

// Gmail : "facture Shopify mars"
catalog.search("MailKB", "facture shopify mars", SearchOptions {
    result_mode: ResultMode::Detailed,  // body vs attachment
    filter_condition: Some(/* date range */),
    ..Default::default()
});
```

Aucun code domaine-spécifique dans le search. Toute la spécificité est dans :
1. Le **schema** (quelles entités, quels champs, quelles KBs)
2. Le **domain adapter** (comment transformer les données sources en entités/relations)
3. Le **client** (comment afficher les résultats selon le domaine)

---

## 9. Architecture Domain Adapter

### 9.1 Interface

Un domain adapter n'est pas un trait Rust formel — c'est un **pattern d'usage** du Catalog :

```
┌─────────────────────────────────────────────────┐
│  Domain Adapter (Code, Shopify, Documents...)   │
│                                                 │
│  1. Définit un CatalogConfig (schema)           │
│  2. Transforme source data → create() + link()  │
│  3. Appelle drain()                             │
│                                                 │
│  Optionnel :                                    │
│  4. Fournit des SearchOptions "presets"          │
│  5. Fournit des ExploreOptions "presets"         │
└─────────────────────────────────────────────────┘
         │ create(), link(), drain()
         ↓
┌─────────────────────────────────────────────────┐
│  Rag3weaver Catalog (100% générique)            │
│                                                 │
│  Ingestion : queue, chunk, embed, store, link   │
│  Search : BM25 + vector + sparse + fusion       │
│  ResultMode : Aggregated / SourceResolved /     │
│               Detailed                          │
│  Explore : BFS graph traversal                  │
│  Filtres : title entity → allowed_ids           │
└─────────────────────────────────────────────────┘
```

### 9.2 Exemple : Code Domain Adapter (Rust)

```rust
// Pseudo-code — ce que le domain adapter Code ferait

pub struct CodeDomainAdapter;

impl CodeDomainAdapter {
    /// Config schema pour le Code Domain
    pub fn schema() -> CatalogConfig { /* File, Scope, Library, ScopeKB, TreeKB, ... */ }

    /// Convertit le résultat codeparsers en opérations catalog
    pub async fn ingest(
        catalog: &Catalog,
        parse_result: &ProjectAnalysis,
    ) -> Result<FlushResult, Error> {
        // 1. Entités
        for file in &parse_result.files { catalog.create("File", file.to_data()); }
        for scope in &parse_result.scopes {
            let mut data = scope.to_data();
            if is_container(scope) {
                data.insert("member_summary", build_member_summary(scope, &parse_result));
            }
            catalog.create("Scope", data);
        }
        for lib in &parse_result.libraries { catalog.create("Library", lib.to_data()); }

        // 2. Relations
        for rel in &parse_result.relationships {
            catalog.link(&rel.rel_type, rel.from_ref, rel.to_ref);
        }

        // 3. Drain
        catalog.drain().await
    }

    /// Presets de recherche
    pub fn implementation_search() -> SearchOptions {
        SearchOptions {
            filter_condition: Some(FilterCondition::Or(vec![
                FilterCondition::field_eq("scope_type", "function"),
                FilterCondition::field_eq("scope_type", "method"),
            ])),
            result_mode: ResultMode::Detailed,
            ..Default::default()
        }
    }
}
```

### 9.3 Exemple : Shopify Domain Adapter

```rust
pub struct ShopifyDomainAdapter;

impl ShopifyDomainAdapter {
    pub fn schema() -> CatalogConfig { /* Product, Variant, Collection, ProductKB, ... */ }

    pub async fn ingest(catalog: &Catalog, products: &[ShopifyProduct]) -> Result<...> {
        for product in products {
            let prod_ref = catalog.create("Product", product.to_data());
            for variant in &product.variants {
                let var_ref = catalog.create("Variant", variant.to_data());
                catalog.link("HAS_VARIANT", prod_ref, var_ref);
            }
        }
        catalog.drain().await
    }

    pub fn price_range_search(min: f64, max: f64) -> SearchOptions {
        SearchOptions {
            filter_condition: Some(FilterCondition::And(vec![
                FilterCondition::field_gte("price_min", min),
                FilterCondition::field_lte("price_max", max),
            ])),
            result_mode: ResultMode::SourceResolved,
            ..Default::default()
        }
    }
}
```

---

## 10. Cas avancé : Detailed + Explore combinés

Le cas le plus riche est une recherche Code avec Detailed (chunks attributés) ET Explore (graphe de dépendances) :

```rust
let response = catalog.search_with_explore("ScopeKB", "database connection", ExploreOptions {
    search: SearchOptions {
        result_mode: ResultMode::Detailed,
        limit: 5,
        ..Default::default()
    },
    depth: 2,
    outgoing_relations: vec!["CONSUMES".into(), "USES_LIBRARY".into()],
    incoming_relations: vec!["PARENT_OF".into(), "DEFINED_IN".into()],
    ..Default::default()
});
```

Le résultat contient :
1. **Résultats** avec chunks attributés (docstring vs code vs member_summary)
2. **Graphe** : scope → CONSUMES → autre scope, scope → DEFINED_IN → file, scope → USES_LIBRARY → library

Le client peut construire une vue riche :
```
📦 class DatabasePool (score: 0.95)
   📝 docstring: "Connection pool avec retry et circuit breaker..."
   💻 content:  "async fn get_connection(&self) -> Connection { ... }"
   📋 members:  "- get_connection() (L15-30)\n- release() (L32-40)"
   ├── CONSUMES → fn retry_with_backoff
   ├── USES_LIBRARY → tokio
   └── DEFINED_IN → src/db/pool.rs
```

Tout ça sans code domaine-spécifique. Uniquement : schema + search options + formatage client.

---

## 11. Ce qu'on ne fait PAS (décisions conscientes)

### 11.1 Pas de hooks post-search dans rag3weaver

L5 avait `onResultEnrich` comme callback. On ne le reproduit pas. Raisons :
- Chaque hook fait N+1 queries (1 par résultat) → lent
- La logique est opaque (l'appelant ne sait pas ce que le hook fait)
- ResultMode + Explore couvrent tous les cas sans queries supplémentaires

Si un cas très spécifique nécessite un enrichissement custom, le client le fait après avoir reçu les résultats.

### 11.2 Pas d'agrégation automatique des enfants

On ne traverse pas automatiquement `PARENT_OF` pour agréger le contenu des méthodes dans le container. Le domain adapter décide quoi mettre dans `member_summary`. Rag3weaver chunk et indexe ce qu'on lui donne.

### 11.3 Pas de boosting conditionnel (pour l'instant)

`boostIf` de L5 modifie les scores post-search. C'est une feature de reranking qui peut être ajoutée plus tard (cf. doc 03 §6). Pour l'instant, le filtrage strict via `FilterCondition` suffit — on exclut ce qu'on ne veut pas plutôt que de booster ce qu'on préfère.

### 11.4 Pas de cross-KB search (pour l'instant)

`search_across_kbs(["ScopeKB", "FileKB", "LibraryKB"], query)` est une feature future (doc 03 §2). Chaque KB est searchée indépendamment pour l'instant. L'appelant peut faire N recherches parallèles et fusionner côté client.

---

## 12. Ordre d'implémentation recommandé

```
Phase 1 : ResultMode dans rag3weaver (task #105)
├── 1a. _source_entity + _source_uuid sur chunks (schema.rs + catalog.rs)
├── 1b. ResultMode enum + AttributedChunk struct (search.rs)
├── 1c. SourceResolved resolution (catalog.rs)
├── 1d. Detailed multi-chunk resolution (search.rs + catalog.rs)
└── 1e. Tests E2E (task #106)

Phase 2 : Domain Adapter Code (task #95)
├── 2a. Schema config Code (File, Scope, Library, ScopeKB, TreeKB)
├── 2b. codeparsers → entities conversion (Rust natif)
├── 2c. member_summary builder
├── 2d. Presets de recherche Code
└── 2e. Tests E2E ingestion + search Code

Phase 3 : Améliorations futures (non bloquantes)
├── Boosting conditionnel (reranking post-fusion)
├── Cross-KB search (search_across_kbs)
└── Graph-aware reranking
```

---

## 13. Résumé

| Aspect | L5 JS (hooks domaine) | Rag3weaver Rust (générique) |
|---|---|---|
| Fetch entité source | `_fetchNodeDetails()` | `ResultMode::SourceResolved` |
| Chunks pertinents | `getRelevantChunks()` | `ResultMode::Detailed` |
| Navigation graphe | `searchRelated()` | `search_with_explore()` |
| Filtrage par type | `boostIf` (boost score) | `filter_condition` (filtrage strict) |
| Member summary | Hook pré-ingestion | Domain adapter (inchangé) |
| Presets search | Objets JS exportés | `SearchOptions` Rust pré-configurés |
| Enrichissement post-search | N+1 queries via hooks | Zéro query : tout dans le résultat |

**Conclusion** : les 3 mécanismes génériques (ResultMode, Explore, FilterCondition) remplacent entièrement les hooks domaine-spécifiques de L5. La seule logique domaine-spécifique qui reste est la transformation des données sources en entités/relations (domain adapter), ce qui est exactement là où elle doit être.
