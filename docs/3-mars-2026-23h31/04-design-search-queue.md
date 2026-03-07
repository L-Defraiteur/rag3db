# Doc 04 — Design : SearchQueue — Pipeline de recherche réactif

**Date** : 4 mars 2026
**Branche** : `feature/kb-index-architecture`
**Statut** : Réflexion, pas planifié
**Dépend de** : Doc 01 (ResultMode), Doc 02 (Abstractions)

---

## 1. Insight fondamental

Rag3weaver a déjà un système de queue à priorités pour l'ingestion :

```
create() → InsertOp (prio 1)
         → ChunkOp  (prio 0)  ← ChunkProcessor émet InsertOp + LinkOp + EmbedOp downstream
drain() exécute dans l'ordre des priorités
```

**Le même pattern s'applique au search.** Une recherche n'est pas une opération atomique — c'est un pipeline où chaque étape peut émettre des ops downstream en fonction des résultats :

```
SearchOp → [execute] → résultats
         → ExpandContainerOp (downstream, conditionnel)
         → FetchChunksOp (downstream)
         → FetchSiblingsOp (downstream)
compose() → résultat final enrichi
```

### Pourquoi c'est mieux qu'un DAG statique (node editor)

| | DAG statique | Queue réactive |
|---|---|---|
| Défini | Avant l'exécution | Pendant l'exécution |
| Branchement conditionnel | Non (tous les chemins sont évalués) | Oui (ops émises si condition vraie) |
| Dépend des résultats | Non | Oui |
| Pattern | "Calcule tout, fusionne" | "Cherche, regarde, approfondi" |
| Analogie | Shader graph | Agent qui réfléchit |

Le DAG dit "fais BM25 + Vector, fusionne, limite". La queue dit "cherche, et si tu trouves une classe, va chercher ses enfants". C'est le pattern d'un agent de recherche.

---

## 2. Le cas d'usage concret : expansion de containers

### Le problème

Une recherche "auth middleware" dans ScopeKB retourne `class AuthService` (match via member_summary ou docstring). Le résultat est un index entry avec un seul best chunk. L'utilisateur veut :

1. La classe elle-même (signature + docstring)
2. Les méthodes qui matchent "auth" avec leurs meilleurs chunks de code
3. Les autres méthodes juste avec leur signature (pour le contexte)

Aujourd'hui, le client doit faire 3 queries séparées après le search initial. Avec L5, le hook `enrichCodeResult` faisait ça en N+1 queries.

### La solution : ops downstream

```
SearchOp("ScopeKB", "auth middleware", limit=10)
    │
    ↓ SearchProcessor exécute
    │
    résultats: [AuthService (class, score=0.95), handleAuth (fn, score=0.80), ...]
    │
    ↓ ContainerExpansionProcessor voit AuthService est un container
    │
    ├── émet SearchChildrenOp {
    │     origin_uuid: "scope-auth",
    │     relation: "PARENT_OF",
    │     query: "auth middleware",     ← même query
    │     limit: 5,
    │     mode: detailed               ← avec chunks
    │   }
    │
    └── émet FetchSiblingsOp {
          origin_uuid: "scope-auth",
          relation: "PARENT_OF",
          exclude: [résultats de SearchChildrenOp],  ← exclusion
          fields: ["signature", "scope_type"],       ← juste les signatures
        }
    │
    ↓ SearchChildrenProcessor exécute
    │
    enfants matchants: [handleAuth (chunks: [...]), verifyToken (chunks: [...])]
    │
    ↓ FetchSiblingsProcessor exécute
    │
    autres enfants: [constructor(), logout(), getSession()]  ← signatures only
    │
    ↓ ComposeProcessor assemble
    │
    résultat final:
      AuthService {
        score: 0.95,
        signature: "class AuthService",
        docstring: "Service d'auth JWT...",
        matched_children: [
          { name: "handleAuth", chunks: [{text: "async fn handleAuth..."}] },
          { name: "verifyToken", chunks: [{text: "fn verify(token)..."}] },
        ],
        other_children: [
          { signature: "constructor(opts: Options)" },
          { signature: "logout(): void" },
          { signature: "getSession(): Session" },
        ],
      }
```

### Pourquoi c'est générique

Le même pattern "expansion" fonctionne pour tous les domaines :

| Domaine | Trigger | Relation | Résultat expansé |
|---|---|---|---|
| **Code** | Résultat = container (class, module) | `PARENT_OF` | Méthodes matchantes (chunks) + autres (signatures) |
| **Documents** | Résultat = document | `IN_DOCUMENT` | Sections matchantes (chunks) + autres (titres) |
| **Mail** | Résultat = thread | `IN_THREAD` | Messages matchants (extraits) + autres (subject+date) |
| **Shopify** | Résultat = collection | `IN_COLLECTION` | Produits matchants (description) + autres (titre+prix) |
| **Notion** | Résultat = page parent | `HAS_CHILD` | Sous-pages matchantes (contenu) + autres (titres) |

La logique est toujours :
1. **Trigger** : le résultat a des enfants via une relation
2. **Search children** : re-chercher avec la même query, limité aux enfants
3. **Fetch siblings** : récupérer les enfants non matchants avec des champs minimaux
4. **Compose** : assembler le résultat enrichi

---

## 3. Architecture : SearchQueue

### 3.1 Types d'ops

```rust
enum SearchOp {
    /// Recherche primaire dans une KB.
    Search {
        kb: String,
        query: String,
        options: SearchOptions,
    },

    /// Recherche limitée aux entités liées à un résultat.
    /// Re-search avec la même query mais filtré aux enfants/liés.
    SearchRelated {
        origin_uuid: String,
        relation: String,        // "PARENT_OF", "IN_DOCUMENT", etc.
        query: String,
        options: SearchOptions,
    },

    /// Fetch des entités liées sans search (juste les données).
    /// Pour les "autres membres" qu'on veut afficher en signature-only.
    FetchRelated {
        origin_uuid: String,
        relation: String,
        exclude_uuids: Vec<String>,  // exclure les déjà trouvés par SearchRelated
        fields: Vec<String>,         // ["signature", "scope_type"] — projection
        limit: Option<usize>,
    },

    /// Fetch des chunks d'un résultat spécifique.
    FetchChunks {
        kb: String,
        origin_uuid: String,     // UUID de l'index entry
        query: Option<String>,   // si Some, trier par pertinence
        limit: usize,
    },

    /// Exploration graphe (BFS) depuis un résultat.
    Explore {
        uuid: String,
        depth: usize,
        outgoing: Vec<String>,
        incoming: Vec<String>,
    },
}
```

### 3.2 Processors

Un processor exécute un type d'op et peut émettre des ops downstream :

```rust
trait SearchProcessor {
    /// Quels types d'ops ce processor gère.
    fn handles(&self, op: &SearchOp) -> bool;

    /// Exécute l'op, retourne des résultats intermédiaires + ops downstream.
    async fn process(
        &self,
        op: SearchOp,
        context: &SearchContext,
    ) -> ProcessResult;
}

struct ProcessResult {
    /// Résultats intermédiaires à ajouter au pool.
    results: Vec<IntermediateResult>,
    /// Ops downstream à enqueue.
    downstream: Vec<SearchOp>,
}
```

### 3.3 Processors génériques (built-in)

```rust
/// Exécute un Search via Catalog::search().
struct PrimarySearchProcessor;

/// Exécute un SearchRelated via Catalog::search() + filtre par relation.
struct RelatedSearchProcessor;

/// Fetch des entités liées sans search.
struct FetchRelatedProcessor;

/// Fetch les chunks d'un index entry.
struct FetchChunksProcessor;

/// Expansion conditionnelle : quand un résultat matche une condition,
/// émet des SearchRelated + FetchRelated downstream.
struct ExpansionProcessor {
    /// Condition pour déclencher l'expansion.
    trigger: ExpansionTrigger,
    /// Relation à suivre pour trouver les enfants.
    relation: String,
    /// Champs à fetcher pour les non-matchants.
    sibling_fields: Vec<String>,
    /// Limite d'enfants searchés.
    search_limit: usize,
}
```

### 3.4 ExpansionTrigger : quand déclencher l'expansion

```rust
enum ExpansionTrigger {
    /// Expand si l'entité source a un certain type.
    /// Ex: scope_type IN ["class", "interface", "module"]
    SourceEntityField {
        field: String,
        values: Vec<String>,
    },

    /// Expand si le résultat a un score au-dessus d'un seuil.
    ScoreAbove(f64),

    /// Expand les N premiers résultats.
    TopN(usize),

    /// Toujours expand.
    Always,

    /// Combinaison AND.
    All(Vec<ExpansionTrigger>),
}
```

### 3.5 SearchQueue : orchestration

```rust
struct SearchQueue {
    ops: VecDeque<(f32, SearchOp)>,  // priority queue
    processors: Vec<Box<dyn SearchProcessor>>,
    results: Vec<IntermediateResult>,
    max_rounds: usize,  // safety: éviter les boucles infinies
}

impl SearchQueue {
    /// Exécute la queue jusqu'à ce qu'elle soit vide ou max_rounds atteint.
    async fn drain(&mut self, context: &SearchContext) -> ComposedResult {
        let mut round = 0;
        while let Some((_, op)) = self.ops.pop_front() {
            if round >= self.max_rounds { break; }
            round += 1;

            for processor in &self.processors {
                if processor.handles(&op) {
                    let result = processor.process(op, context).await;
                    self.results.extend(result.results);
                    for downstream_op in result.downstream {
                        self.ops.push_back((next_priority(), downstream_op));
                    }
                    break;
                }
            }
        }
        self.compose()
    }
}
```

---

## 4. Résultat composé : la structure de sortie

### 4.1 Le problème de la composition

Les résultats ne sont plus des `Vec<SearchResult>` plats. Ils sont **hiérarchiques** :

```
SearchResult (AuthService, class)
├── matched_children: [SearchResult (handleAuth), SearchResult (verifyToken)]
│   └── chunks: [AttributedChunk, ...]
├── other_children: [ChildSummary (constructor), ChildSummary (logout), ...]
└── graph: Option<ExploreGraph>
```

### 4.2 Structure enrichie

```rust
/// Résultat enrichi par la SearchQueue.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichedResult {
    // Résultat de base (identique à SearchResult)
    pub uuid: String,
    pub score: f64,
    pub entity: Option<String>,
    pub data: Option<BTreeMap<String, CypherValue>>,

    // Enrichissements optionnels (ajoutés par les processors downstream)
    /// Chunks attribués (mode Detailed ou FetchChunks).
    pub chunks: Option<Vec<AttributedChunk>>,

    /// Enfants qui matchent la query (SearchRelated).
    pub matched_children: Option<Vec<EnrichedResult>>,  // récursif !

    /// Autres enfants (FetchRelated) — données minimales.
    pub other_children: Option<Vec<ChildSummary>>,

    /// Graphe d'exploration (Explore).
    pub graph: Option<ExploreGraph>,
}

/// Résumé minimal d'un enfant non-matchant.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildSummary {
    pub uuid: String,
    pub entity: String,
    pub fields: BTreeMap<String, CypherValue>,  // ex: {"signature": "logout(): void"}
}
```

### 4.3 Exemple de sortie JSON (cas Code)

```json
[
  {
    "uuid": "scope-auth",
    "score": 0.95,
    "entity": "Scope",
    "data": {
      "name": "AuthService",
      "scopeType": "class",
      "signature": "class AuthService",
      "docstring": "Service d'authentification JWT, gère login/logout et validation de tokens."
    },
    "chunks": [
      { "text": "Service d'authentification JWT...", "sourceField": "docstring", "score": 0.92 }
    ],
    "matchedChildren": [
      {
        "uuid": "scope-handle",
        "score": 0.88,
        "entity": "Scope",
        "data": { "name": "handleAuth", "signature": "async handleAuth(req: Request): Response" },
        "chunks": [
          { "text": "async fn handleAuth(req) {\n  let token = req.header('Authorization')...", "score": 0.88 }
        ]
      },
      {
        "uuid": "scope-verify",
        "score": 0.82,
        "entity": "Scope",
        "data": { "name": "verifyToken", "signature": "verifyToken(token: string): Claims" },
        "chunks": [
          { "text": "fn verify(token: &str) -> Result<Claims> {\n  jwt::decode(token, &key)...", "score": 0.82 }
        ]
      }
    ],
    "otherChildren": [
      { "uuid": "scope-ctor", "entity": "Scope", "fields": { "signature": "constructor(opts: Options)" } },
      { "uuid": "scope-logout", "entity": "Scope", "fields": { "signature": "logout(): void" } },
      { "uuid": "scope-session", "entity": "Scope", "fields": { "signature": "getSession(): Session" } }
    ]
  },
  {
    "uuid": "scope-mw",
    "score": 0.80,
    "entity": "Scope",
    "data": { "name": "authMiddleware", "scopeType": "function", "signature": "function authMiddleware(req, res, next)" },
    "chunks": [
      { "text": "function authMiddleware(req, res, next) {\n  const token = req.cookies.auth...", "score": 0.80 }
    ]
  }
]
```

Le premier résultat (AuthService) a été **expansé** parce que c'est un container. Le second (authMiddleware) est une simple fonction — pas d'expansion.

---

## 5. Configuration : SearchStrategy

### 5.1 Définition

Au lieu de coder les processors en dur, on les configure via une `SearchStrategy` :

```rust
/// Stratégie de recherche : quels processors appliquer et comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStrategy {
    /// Options de recherche primaire.
    pub search: SearchOptions,

    /// Expansions conditionnelles à appliquer sur les résultats.
    pub expansions: Vec<ExpansionConfig>,

    /// Nombre maximum de rounds (sécurité anti-boucle).
    pub max_rounds: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionConfig {
    /// Condition de déclenchement.
    pub trigger: ExpansionTrigger,

    /// Relation à suivre.
    pub relation: String,

    /// Rechercher les enfants avec la query originale.
    pub search_children: bool,
    pub search_limit: usize,

    /// Fetcher les enfants non-matchants.
    pub fetch_siblings: bool,
    pub sibling_fields: Vec<String>,
    pub sibling_limit: Option<usize>,
}
```

### 5.2 Presets par domaine

```rust
// Code Domain
fn code_search_strategy() -> SearchStrategy {
    SearchStrategy {
        search: SearchOptions {
            result_mode: ResultMode::SourceResolved,
            limit: 10,
            ..Default::default()
        },
        expansions: vec![
            ExpansionConfig {
                trigger: ExpansionTrigger::SourceEntityField {
                    field: "scope_type".into(),
                    values: vec!["class", "interface", "module", "enum", "namespace"]
                        .into_iter().map(String::from).collect(),
                },
                relation: "PARENT_OF".into(),
                search_children: true,
                search_limit: 5,
                fetch_siblings: true,
                sibling_fields: vec!["signature".into(), "scope_type".into()],
                sibling_limit: Some(20),
            },
        ],
        max_rounds: 3,
    }
}

// Document Domain
fn document_search_strategy() -> SearchStrategy {
    SearchStrategy {
        search: SearchOptions {
            result_mode: ResultMode::SourceResolved,
            limit: 10,
            ..Default::default()
        },
        expansions: vec![
            ExpansionConfig {
                trigger: ExpansionTrigger::Always,
                relation: "IN_DOCUMENT".into(),
                search_children: true,
                search_limit: 3,
                fetch_siblings: true,
                sibling_fields: vec!["title".into(), "title_level".into()],
                sibling_limit: Some(10),
            },
        ],
        max_rounds: 2,
    }
}

// Mail Domain
fn mail_search_strategy() -> SearchStrategy {
    SearchStrategy {
        search: SearchOptions {
            result_mode: ResultMode::SourceResolved,
            limit: 10,
            ..Default::default()
        },
        expansions: vec![
            ExpansionConfig {
                trigger: ExpansionTrigger::Always,
                relation: "IN_THREAD".into(),
                search_children: true,
                search_limit: 3,
                fetch_siblings: true,
                sibling_fields: vec!["subject".into(), "date".into(), "from".into()],
                sibling_limit: Some(10),
            },
        ],
        max_rounds: 2,
    }
}
```

### 5.3 La stratégie par défaut = pas d'expansion

```rust
impl Default for SearchStrategy {
    fn default() -> Self {
        Self {
            search: SearchOptions::default(),
            expansions: vec![],  // pas d'expansion = comportement actuel
            max_rounds: 1,
        }
    }
}
```

`Catalog::search()` avec `SearchStrategy::default()` = exactement le comportement actuel. Zéro changement breaking.

---

## 6. API : comment l'exposer

### 6.1 Rust

```rust
// Simple (comportement actuel)
let response = catalog.search("ScopeKB", "auth", SearchOptions::default()).await?;

// Avec stratégie d'expansion
let response = catalog.search_with_strategy(
    "ScopeKB",
    "auth middleware",
    code_search_strategy(),
).await?;
```

### 6.2 CATALOG_SEARCH (Cypher, cf. Doc 03)

```cypher
-- Simple
CALL CATALOG_SEARCH('ScopeKB', 'auth middleware')
YIELD uuid, score, entity, data

-- Avec expansion (strategy en JSON)
CALL CATALOG_SEARCH('ScopeKB', 'auth middleware',
    strategy := '{
      "expansions": [{
        "trigger": {"source_entity_field": {"field": "scope_type", "values": ["class"]}},
        "relation": "PARENT_OF",
        "search_children": true,
        "search_limit": 5,
        "fetch_siblings": true,
        "sibling_fields": ["signature"]
      }]
    }')
YIELD uuid, score, entity, data, matched_children, other_children
```

### 6.3 JSON (API REST / agents LLM)

```json
{
  "kb": "ScopeKB",
  "query": "auth middleware",
  "strategy": {
    "search": { "limit": 10, "result_mode": "source_resolved" },
    "expansions": [
      {
        "trigger": { "source_entity_field": { "field": "scope_type", "values": ["class", "interface"] } },
        "relation": "PARENT_OF",
        "search_children": true,
        "search_limit": 5,
        "fetch_siblings": true,
        "sibling_fields": ["signature", "scope_type"],
        "sibling_limit": 20
      }
    ]
  }
}
```

---

## 7. Relation avec les docs précédents

| Doc | Relation |
|---|---|
| **Doc 01 (ResultMode)** | `ResultMode::Detailed` est un cas simple de la SearchQueue : "expand tous les chunks". La SearchQueue généralise ça en expansion conditionnelle + multi-relation. |
| **Doc 02 (Abstractions)** | La SearchQueue remplace complètement les hooks L5. `enrichCodeResult` = un `ExpansionConfig` avec trigger=container, relation=PARENT_OF. Zéro code domaine-spécifique. |
| **Doc 03 (CATALOG_SEARCH)** | Le paramètre `strategy` de CATALOG_SEARCH accepte une `SearchStrategy` sérialisée. La table function instancie et drain une SearchQueue. |
| **Node editor** | Le DAG statique (node editor) définit la pipeline de la recherche primaire. La SearchQueue ajoute la dimension réactive (ops conditionnelles post-résultat). Les deux sont complémentaires — le node editor configure la recherche initiale, les expansions configurent les follow-ups. |

---

## 8. Limites et garde-fous

### 8.1 Explosion combinatoire

Si 10 résultats sont des containers, et chaque expansion fait 2 queries (SearchRelated + FetchRelated), on a 20 queries downstream pour un seul search. Avec `max_rounds = 3`, ça pourrait cascader.

**Solutions :**
- `max_rounds` : limite absolue sur le nombre de rounds
- `expansion_limit` : ne pas expand plus de N résultats (ex: top 3 seulement)
- `total_query_budget` : limite globale sur le nombre de queries exécutées
- Pas de récursion : un résultat expansé ne déclenche pas de nouvelle expansion (profondeur 1)

### 8.2 Latence

Chaque round ajoute un aller-retour DB. Pour le cas Code avec expansion containers :
- Round 1 : search primaire (~50ms)
- Round 2 : 3 SearchRelated + 3 FetchRelated (~100ms, parallélisable)
- Total : ~150ms

Acceptable pour un search interactif. Les rounds sont parallélisables au sein d'un même niveau de priorité (comme l'ingestion queue).

### 8.3 Complexité progressive

L'API est 100% rétrocompatible :
- `catalog.search()` avec `SearchOptions` = pas d'expansion = comportement actuel
- `catalog.search_with_strategy()` avec `SearchStrategy` = expansion optionnelle
- Les presets par domaine sont des `SearchStrategy` pré-configurées

Un domaine peut commencer sans expansion et l'ajouter plus tard sans changer le code de search ni le schéma.

---

## 9. Implémentation progressive

```
Phase 1 : ResultMode (task #105)
    SearchResult avec chunks attributés
    Pas de queue, pas d'expansion — juste Detailed mode
    ↓
Phase 2 : SearchQueue minimal
    SearchOp + SearchRelatedOp + FetchRelatedOp
    ExpansionProcessor avec ExpansionConfig
    EnrichedResult struct
    ↓
Phase 3 : Presets domaine
    code_search_strategy()
    document_search_strategy()
    ↓
Phase 4 : CATALOG_SEARCH avec strategy (Doc 03)
    Table function qui accepte strategy JSON
    ↓
Phase 5 : Node editor (optionnel, futur lointain)
    Frontend React Flow
    Sérialise vers SearchStrategy + SearchPipeline
```

Phase 1 est déjà planifiée (task #105). Phase 2 peut venir juste après sans casser quoi que ce soit.

---

## 10. Résumé

| Concept | Description |
|---|---|
| **SearchQueue** | Queue réactive : les ops émettent des ops downstream selon les résultats |
| **SearchOp** | Opération de recherche : Search, SearchRelated, FetchRelated, FetchChunks, Explore |
| **ExpansionProcessor** | Quand un résultat matche une condition, émet des ops pour chercher/fetcher ses enfants |
| **SearchStrategy** | Config déclarative : options de search + liste d'expansions conditionnelles |
| **EnrichedResult** | Résultat hiérarchique : données + chunks + matched_children + other_children + graph |
| **Rétrocompatible** | `SearchStrategy::default()` = pas d'expansion = `Catalog::search()` actuel |
| **Générique** | Le même ExpansionConfig fonctionne pour Code (PARENT_OF), Documents (IN_DOCUMENT), Mail (IN_THREAD), Shopify (IN_COLLECTION) |
