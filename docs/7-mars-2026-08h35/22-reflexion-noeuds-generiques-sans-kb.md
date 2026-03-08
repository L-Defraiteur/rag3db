# Doc 22 — Réflexion : Nœuds génériques sans concept KB

Date : 8 mars 2026

## 1. Le problème

Le framework dataflow a deux couches :

**Couche basse (générique)** — InsertRecordNode, LinkRecordNode, ChunkRecordNode : fonctionnent sur n'importe quelle entité. Pas de concept KB.

**Couche haute (KB-spécifique)** — GatherKBNode, UpdateKBNode, ChunkKBNode, FlushFTSNode, PrimarySearchNode : hardcodés sur l'architecture `{KB}_Index` / `{KB}_Index_Chunk`.

Le problème : on ne peut pas construire un pipeline simple "entités → chunks → embed → search" sans passer par l'abstraction KB. Un utilisateur qui veut juste :

```
Insérer des documents → les chunker → les embedder → les rechercher
```

...est forcé de créer une KB, définir un title_entity, des content_fields, des relations, etc. C'est de la complexité inutile pour les cas simples.

---

## 2. État actuel : couplage KB par nœud

| Nœud | Générique ? | Couplage KB | Ce qu'il faudrait changer |
|------|-------------|-------------|---------------------------|
| InsertRecordNode | Oui | Aucun | Rien |
| LinkRecordNode | Oui | Aucun | Rien |
| ChunkRecordNode | Oui | Aucun | Rien |
| EmbedRecordNode | Partiel | Utilise `_kb_name` pour les noms de colonnes d'embedding (`{kb}_embedding`, `{kb}_sparse`) et pour résoudre les signaux | Rendre le nom de colonne configurable |
| GatherKBNode | Non | Total (title_entity, content_fields, relations) | Non réutilisable tel quel |
| UpdateKBNode | Non | Total (`{KB}_Index`, `{KB}_Index_Chunk`) | Non réutilisable tel quel |
| ChunkKBNode | Non | Total (`{KB}_Index_Chunk`) | Non réutilisable tel quel |
| FlushFTSNode | Non | Total (FLUSH_LUCIVY_INDEX sur `{KB}_Index`) | Rendre le nom de table configurable |
| QuerySourceNode | Partiel | Prend un `kb_name` | Rendre optionnel ou ajouter `entity_name` |
| PrimarySearchNode | Non | Appelle `Catalog::search()` hardcodé KB | Ajouter un mode de recherche directe |
| FetchRelatedNode | Oui | Aucun | Rien |
| ComposeNode | Oui | Aucun | Rien |

**Score** : 5 nœuds génériques, 4 nœuds KB-only, 3 nœuds partiellement couplés.

---

## 3. Vision : pipeline standard sans KB

### 3.1 Ingestion simple

Un utilisateur a des `Document` avec un champ `content`. Il veut les insérer, chunker, embedder.

```
graph LR
    insert["InsertRecordNode"]
    chunk["ChunkRecordNode"]
    chunk_insert["InsertRecordNode"]
    chunk_link["LinkRecordNode"]
    embed["EmbedNode(entity='Document_Chunk', text_field='content', embedding_col='embedding')"]

    insert -->|inserted:entities| chunk
    chunk -->|entities| chunk_insert
    chunk -->|relations| chunk_link
    chunk_insert -->|done:trigger| chunk_link
    chunk_insert -->|inserted:entities| embed
```

**Différence avec le pipeline KB** :
- Pas de GatherKB/UpdateKB/ChunkKB — on chunke directement les entités insérées
- Pas de `{KB}_Index` intermédiaire — les chunks sont liés directement à l'entité source
- L'embedding est stocké sur les chunks directement (`Document_Chunk.embedding`) au lieu de `{kb}_embedding`
- Pas de FlushFTS (ou un FlushFTS configurable sur la bonne table)

### 3.2 Search simple

L'utilisateur veut chercher dans ses Documents via les embeddings des chunks.

```
graph LR
    query["SearchSourceNode(entity='Document_Chunk', query='$query')"]
    search["VectorSearchNode(entity='Document_Chunk', embedding_col='embedding', limit=10)"]
    resolve["ResolveSourceNode(relation='CHUNKED_FROM')"]

    query -->|query| search
    search -->|results| resolve
```

Ou avec BM25 :

```
graph LR
    query["SearchSourceNode(entity='Document', query='$query')"]
    bm25["BM25SearchNode(entity='Document', fields='title,content', limit=10)"]

    query -->|query| bm25
```

**Différence avec le pipeline KB** :
- Pas de Catalog::search() — recherche directe via Cypher sur la table spécifiée
- Pas de KB metadata à résoudre
- Le nœud de recherche connaît juste la table, la colonne d'embedding, et le query

---

## 4. Nouveaux nœuds nécessaires

### 4.1 EmbedNode (refactor d'EmbedRecordNode ou nouveau)

L'actuel `EmbedRecordNode` utilise `_kb_name` pour :
- Déterminer les signaux (BM25, Vector, Sparse) via `kb_metadata`
- Nommer les colonnes : `{kb}_embedding`, `{kb}_sparse_indices`, `{kb}_sparse_weights`

**Option A — Refactor EmbedRecordNode** : rendre `_kb_name` optionnel, avec fallback sur des noms de colonnes configurés.

**Option B — Nouveau EmbedNode** : plus simple, config explicite :

```
embed["EmbedNode(entity='Document_Chunk', text_field='content', embedding_col='embedding', signals='vector')"]
```

| Config | Type | Défaut | Description |
|--------|------|--------|-------------|
| `entity` | String | requis | Table cible |
| `text_field` | String | `"content"` | Champ texte à embedder |
| `embedding_col` | String | `"embedding"` | Colonne pour le vecteur dense |
| `sparse_col` | String | `""` | Colonne pour le sparse (si vide, pas de sparse) |
| `signals` | String | `"vector"` | Quels embeddings : `vector`, `bm25`, `sparse`, `hybrid` |
| `gpu_batch_size` | usize | 32 | Taille des batches GPU |

**Recommandation : Option B** — un nouveau nœud plus simple. L'ancien EmbedRecordNode reste pour le pipeline KB. Pas de refactor risqué.

### 4.2 VectorSearchNode (nouveau)

Recherche vectorielle directe sur une table d'entités.

```rust
pub struct VectorSearchNode {
    entity: String,       // "Document_Chunk"
    embedding_col: String, // "embedding"
    limit: usize,
}
```

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `query` | in | Query | Query texte + options |
| `results` | out | Results | Vec<UnifiedResult> |

**Exécution** :
1. Embedder le query texte
2. Cypher : `MATCH (n:{entity}) WHERE n.{embedding_col} IS NOT NULL RETURN n._uuid, n.{embedding_col}` + calcul de similarité cosinus
3. Trier par score, limiter
4. Retourner comme Vec<UnifiedResult>

**Alternative** : utiliser la fonction `VECTOR_SEARCH()` de rag3db si elle existe, ou un Cypher custom.

### 4.3 BM25SearchNode (nouveau)

Recherche BM25 directe via Tantivy/Lucivy.

```rust
pub struct BM25SearchNode {
    entity: String,       // "Document"
    fields: Vec<String>,  // ["title", "content"]
    limit: usize,
}
```

**Exécution** :
1. Appeler `QUERY_TANTIVY_INDEX('{entity}', '{field}', '{query}')` pour chaque champ
2. Fusionner les résultats
3. Retourner comme Vec<UnifiedResult>

### 4.4 ResolveSourceNode (nouveau)

Résoudre les chunks vers leurs entités sources (remonter via une relation).

```rust
pub struct ResolveSourceNode {
    relation: String,     // "CHUNKED_FROM"
    direction: ExpansionDirection,
}
```

| Port | Direction | Type |
|------|-----------|------|
| `results` | in | Results |
| `results` | out | Results |

**Exécution** :
1. Pour chaque résultat (un chunk), suivre la relation inverse
2. Remplacer l'uuid/entity par ceux de l'entité source
3. Dédup si plusieurs chunks pointent vers la même source (garder meilleur score)

C'est essentiellement ce que `ResultMode::SourceResolved` fait dans le Catalog, mais en nœud composable.

### 4.5 FuseResultsNode (nouveau, optionnel)

Fusionner les résultats de plusieurs recherches (vector + BM25 → RRF ou weighted).

```rust
pub struct FuseResultsNode {
    strategy: FusionStrategy, // RRF ou Weighted
}
```

| Port | Direction | Type |
|------|-----------|------|
| `results` | in | Results | (fan-in : merge automatique de plusieurs sources)
| `results` | out | Results |

---

## 5. Nouveaux templates

### 5.1 `simple_ingestion.mmd` — Insert + Chunk + Embed

```
graph LR
    insert["InsertRecordNode"]
    chunk["ChunkRecordNode"]
    chunk_insert["InsertRecordNode"]
    chunk_link["LinkRecordNode"]
    embed["EmbedNode(text_field='$text_field', embedding_col='$embedding_col', signals='$signals')"]

    insert -->|inserted:entities| chunk
    chunk -->|entities| chunk_insert
    chunk -->|relations| chunk_link
    chunk_insert -->|done:trigger| chunk_link
    chunk_insert -->|inserted:entities| embed
```

### 5.2 `simple_search.mmd` — Vector search + resolve source

```
graph LR
    source["SearchSourceNode(query='$query')"]
    vector["VectorSearchNode(entity='$entity_chunk', embedding_col='$embedding_col', limit=$limit)"]
    resolve["ResolveSourceNode(relation='CHUNKED_FROM')"]

    source -->|query| vector
    vector -->|results| resolve
```

### 5.3 `hybrid_search.mmd` — Vector + BM25 + fusion

```
graph LR
    source["SearchSourceNode(query='$query')"]
    vector["VectorSearchNode(entity='$entity_chunk', embedding_col='$embedding_col', limit=$limit)"]
    bm25["BM25SearchNode(entity='$entity', fields='$fields', limit=$limit)"]
    fuse["FuseResultsNode(strategy='rrf')"]
    resolve["ResolveSourceNode(relation='CHUNKED_FROM')"]

    source -->|query| vector
    source -->|query| bm25
    vector -->|results| fuse
    bm25 -->|results| fuse
    fuse -->|results| resolve
```

---

## 6. Relation avec les nœuds KB existants

Les nœuds KB ne sont **pas remplacés** — ils restent pour les cas où l'abstraction KB apporte de la valeur (agrégation multi-source, FTS automatique, content change detection).

La vision est deux niveaux de templates :

**Templates "simples" (nouveaux)** : pour les cas directs
- `simple_ingestion.mmd` — entity → chunk → embed
- `simple_search.mmd` — vector search direct
- `hybrid_search.mmd` — vector + BM25

**Templates "KB" (existants)** : pour les cas avancés
- `ingestion.mmd` — pipeline KB complet avec agrégation
- `search.mmd` / `search_expansion.mmd` — recherche via KB Index

Les nœuds génériques (Insert, Link, Chunk) sont partagés entre les deux niveaux.

---

## 7. Relation avec Rhai/ScriptNode (Phase 5)

Les nouveaux nœuds sont **complémentaires** à ScriptNode, pas concurrents :

```
graph LR
    source["SearchSourceNode(query='$query')"]
    vector["VectorSearchNode(entity='Product_Chunk', embedding_col='embedding', limit=20)"]
    filter["ScriptNode(script='filter by category', in_results='Results', out_results='Results')"]
    resolve["ResolveSourceNode(relation='CHUNKED_FROM')"]

    source -->|query| vector
    vector -->|results| filter
    filter -->|results| resolve
```

ScriptNode s'insère entre les nœuds génériques pour la logique custom (filtre, reranking, transformation). Les nœuds génériques fournissent les briques de base.

---

## 8. API SimpleCatalog

### 8.1 Le besoin

Les nœuds séparés sont pour les power users qui composent des templates Mermaid. Mais l'utilisateur Node.js/WASM typique ne va pas écrire du Mermaid pour faire une recherche — il veut une API simple.

```typescript
// Aujourd'hui (KB obligatoire)
catalog.createKB("Products", {
    titleEntity: "Product",
    contentFields: ["name", "description"],
    relations: [{ name: "HAS_CATEGORY", direction: "Outgoing" }],
    // ...
});
await catalog.ingest("Products", entities);
const results = await catalog.search("Products", "red shoes");

// Simple (pas de KB, pas de setup)
await catalog.ingestEntities("Product", entities, { chunk: true, embed: true });
const results = await catalog.searchEntities("Product", "red shoes");
```

### 8.2 registerEntity — config par champ

Pas besoin d'un "SimpleCatalog" séparé. Nouvelle méthode `registerEntity` sur le Catalog existant.

La config est essentiellement un `EntityDef` (qui existe déjà) avec `isTitle`/`isContent` au lieu de `title_for`/`content_for` (qui référencent un KB). **Les types des champs sont déclarés** — Kuzu est typé statiquement, `generate_node_table_ddl()` en a besoin pour le `CREATE NODE TABLE`.

```rust
/// Config d'une entité pour le pipeline simple.
/// Réutilise FieldType existant (String, Text, Int64, Double, Boolean, etc.)
pub struct SimpleFieldDef {
    /// Type du champ (String, Text, Int64, Double, Boolean, Timestamp, etc.)
    pub field_type: FieldType,
    /// Champ titre (contexte pour les chunks). Un seul par entité.
    pub is_title: bool,
    /// Champ contenu (concatené pour chunking/embedding). Plusieurs possibles.
    pub is_content: bool,
}

pub struct EntityConfig {
    pub fields: HashMap<String, SimpleFieldDef>,
    /// Signaux d'embedding. Défaut: Hybrid (vector + BM25).
    pub signals: SearchSignals,
}
```

**Pas de `isFilter`** — le filtrage passe systématiquement par pré-filtrage Cypher → allowed_ids, puis les IDs sont passés aux moteurs de recherche (vector, BM25, sparse). Tout champ est filtrable sans déclaration préalable (Kuzu fait le type-checking à l'exécution). Les filter_fields natifs Tantivy ne sont plus utilisés.

**Exemple TypeScript** :
```typescript
catalog.registerEntity("Product", {
    fields: {
        name:        { type: "text", isTitle: true },
        description: { type: "text", isContent: true },
        details:     { type: "text", isContent: true },
        price:       { type: "double" },
        category:    { type: "string" },
        in_stock:    { type: "boolean" },
    },
    signals: "hybrid",
});

// Filtrage à la recherche — marche sur n'importe quel champ typé
const results = await catalog.searchEntities("Product", "red shoes", {
    filters: { price: [{ gte: 10 }, { lte: 100 }], category: "shoes" },
});
// → Cypher WHERE sur Product (price >= 10.0 AND price <= 100.0 AND category = 'shoes')
// → récupère les _uuid matchants → allowed_ids
// → BM25 + vector search sur Product_Chunk avec allowed_ids
```

**Ce que registerEntity fait en interne** :
1. `CREATE NODE TABLE IF NOT EXISTS Product(_uuid STRING, _content_hash STRING, name STRING, description STRING, details STRING, price DOUBLE, category STRING, in_stock BOOLEAN, PRIMARY KEY(_uuid))`
2. `CREATE NODE TABLE IF NOT EXISTS Product_Chunk(_uuid STRING, _text STRING, _title STRING, _embed_hash STRING, embedding DOUBLE[], sparse_indices INT64[], sparse_weights DOUBLE[], PRIMARY KEY(_uuid))`
3. `CREATE REL TABLE IF NOT EXISTS Product_CHUNKED_FROM(FROM Product_Chunk TO Product)`
4. `CREATE_LUCIVY_INDEX('Product_Chunk', ['_text'])`
5. Stocke la config en mémoire (comme `kb_metadata` pour les KB)

**Comparaison avec createKB** :
```typescript
// createKB — agrégation multi-source, types déjà dans EntityDef
catalog.createKB("Products", {
    titleEntity: "Product",
    contentFields: ["description"],
    relations: [
        { name: "HAS_REVIEW", fields: ["text"] }
    ],
    signals: "hybrid",
});

// registerEntity — direct, une seule entité, types dans la config
catalog.registerEntity("Product", {
    fields: {
        name: { type: "text", isTitle: true },
        description: { type: "text", isContent: true },
        price: { type: "double" },
    },
    signals: "hybrid",
});
```

La grosse différence : **createKB agrège du contenu depuis plusieurs entités liées** (Product + Reviews + Categories via relations). Il utilise les `EntityDef` déjà déclarées dans le schéma global. **registerEntity est self-contained** — il déclare l'entité et ses types en un seul appel, sans schéma global ni KB.

### 8.3 Nouvelles méthodes sur Catalog

```rust
impl Catalog {
    /// Enregistrer une entité pour le pipeline simple.
    /// Crée la table de chunks + index FTS si nécessaire.
    pub async fn register_entity(
        &mut self,
        entity: &str,
        config: EntityConfig,
    ) -> Result<(), CatalogError>;

    /// Ingestion directe : insert + chunk + embed, sans KB.
    ///
    /// Construit et exécute un DataflowGraph en interne :
    ///   InsertRecordNode → ChunkRecordNode → InsertRecordNode (chunks)
    ///                                      → LinkRecordNode (CHUNKED_FROM)
    ///                                      → EmbedNode (sur les chunks)
    /// Le chunking est toujours actif (même pour des textes courts → 1 chunk).
    pub async fn ingest_entities(
        &self,
        entity: &str,
        records: Vec<EntityRecord>,
    ) -> Result<IngestResult, CatalogError>;

    /// Recherche directe : vector + BM25 sur les chunks, sans KB.
    ///
    /// Construit et exécute un DataflowGraph en interne :
    ///   SearchSourceNode → VectorSearchNode ─┐
    ///                    → BM25SearchNode ────┤→ FuseResultsNode → ResolveSourceNode
    pub async fn search_entities(
        &self,
        entity: &str,
        query: &str,
        opts: SearchOptions,
    ) -> Result<Vec<SearchResult>, CatalogError>;
}
```

**Filtrage à la recherche** — tout champ est filtrable via `SearchOptions.filters`, sans déclaration :
```typescript
const results = await catalog.searchEntities("Product", "red shoes", {
    filters: {
        price: [{ gte: 10 }, { lte: 100 }],
        category: "shoes",
    },
});
// → Cypher pré-filtre sur Product → allowed_ids
// → BM25 + vector search sur Product_Chunk avec allowed_ids
```

impl Catalog {
    /// Ingestion directe : insert + (optionnel: chunk) + embed, sans KB.
    ///
    /// Construit et exécute un DataflowGraph en interne :
    ///   InsertRecordNode → ChunkRecordNode → InsertRecordNode (chunks)
    ///                                      → LinkRecordNode (CHUNKED_FROM)
    ///                                      → EmbedNode (sur les chunks)
    /// Le chunking est toujours actif (même pour des textes courts → 1 chunk).
    pub async fn ingest_entities(
        &self,
        entity: &str,
        records: Vec<EntityRecord>,
        opts: IngestEntityOptions,
    ) -> Result<IngestResult, CatalogError>;

    /// Recherche directe : vector + BM25 sur une table d'entités, sans KB.
    ///
    /// Construit et exécute un DataflowGraph en interne :
    ///   SearchSourceNode → VectorSearchNode ─┐
    ///                    → BM25SearchNode ────┤→ FuseResultsNode → ResolveSourceNode
    ///
    /// Si l'entité a des chunks ({Entity}_Chunk), cherche sur les chunks
    /// et résout vers l'entité source automatiquement.
    pub async fn search_entities(
        &self,
        entity: &str,
        query: &str,
        opts: SearchOptions,
    ) -> Result<Vec<SearchResult>, CatalogError>;
}
```

### 8.3 Détection automatique

`search_entities` pourrait détecter automatiquement la configuration :

1. **Table cible** : si `{Entity}_Chunk` existe → chercher sur les chunks, sinon sur l'entité directement
2. **Embedding** : détecter la colonne d'embedding disponible (par convention `embedding` ou par introspection schema)
3. **FTS** : si un index Tantivy existe sur la table → activer BM25
4. **Resolve** : si on cherche sur les chunks → ajouter ResolveSourceNode automatiquement

Ça donnerait un flow "zero config" :

```typescript
// L'utilisateur ne spécifie rien — le catalog détecte tout
const results = await catalog.searchEntities("Product", "red shoes");
// → détecte Product_Chunk, embedding col, Tantivy index → hybrid search + resolve
```

### 8.4 Relation avec le Catalog KB existant

Les deux APIs coexistent :

| | `search(kb, query)` | `searchEntities(entity, query)` |
|---|---|---|
| **Setup requis** | `createKB()` avec config complète | Rien (ou `ingestEntities()`) |
| **Agrégation** | Multi-source via `{KB}_Index` | Entité directe |
| **FTS** | Sur `{KB}_Index` | Sur l'entité ou ses chunks |
| **Content tracking** | `_content_hash` sur `{KB}_Index` | Pas de tracking (re-embed si contenu change) |
| **Complexité** | Haute (title_entity, relations, etc.) | Basse |
| **Cas d'usage** | KB multi-source, agrégation complexe | Entités simples, prototypage rapide |

L'utilisateur commence avec `searchEntities` (simple, rapide). S'il a besoin d'agrégation multi-source, il migre vers `createKB` + `search`.

### 8.5 Impact Node.js / WASM

Les nouvelles méthodes doivent être exposées dans les bindings :

**Node.js** (rag3dbjs) :
```typescript
interface Catalog {
    // Existant
    createKB(name: string, config: KBConfig): Promise<void>;
    ingest(kb: string, records: EntityRecord[]): Promise<IngestResult>;
    search(kb: string, query: string, opts?: SearchOptions): Promise<SearchResult[]>;

    // Nouveau
    ingestEntities(entity: string, records: EntityRecord[], opts?: IngestEntityOptions): Promise<IngestResult>;
    searchEntities(entity: string, query: string, opts?: SearchOptions): Promise<SearchResult[]>;
}
```

**WASM** : même interface via wasm-bindgen.

---

## 9. Priorisation

| Étape | Quoi | Effort | Dépend de |
|-------|------|--------|-----------|
| **A** | Deserialize sur types search | ~0.5j | Rien |
| **B** | EmbedNode (version simple) | ~1j | Rien |
| **C** | VectorSearchNode | ~1-2j | Deserialize (A) |
| **D** | BM25SearchNode | ~1j | Idem |
| **E** | ResolveSourceNode | ~0.5j | Rien |
| **F** | FuseResultsNode | ~0.5j | Rien |
| **G** | Templates simples (.mmd) | ~0.5j | B, C, D, E |
| **H** | Catalog::ingest_entities / search_entities | ~1-2j | B, C, D, E, F |
| **I** | ScriptNode (Rhai) | ~2j | A |
| **J** | HttpNode | ~1j | Rien |

**Ordre suggéré (briques d'abord)** :

```
A (Deserialize)
├→ B (EmbedNode) ─────────────┐
├→ E (ResolveSourceNode) ─────┤
├→ F (FuseResultsNode) ───────┤→ G (templates) → H (Catalog API)
├→ C (VectorSearchNode) ──────┤
└→ D (BM25SearchNode) ────────┘
                               └→ I (ScriptNode) → J (HttpNode)
```

Ça donne un pipeline simple testable de bout en bout (ingest → search) avant d'ajouter le scripting. Le scripting et l'API Catalog peuvent avancer en parallèle une fois les nœuds de base prêts.

---

## 10. Impact sur les PortTypes

Les nouveaux nœuds utilisent les PortTypes existants :
- `Query` — entrée de recherche
- `Results` — résultats de recherche
- `Entities` / `Relations` — données d'ingestion
- `Empty` — trigger/done

Pas de nouveau PortType nécessaire. Le PortType `Any` sert pour ScriptNode.

La seule addition nécessaire est `Deserialize` sur les types search pour que les nœuds puissent recevoir/émettre des Results via ports configurables.

---

## 11. Questions ouvertes

### Q1 — EmbedNode vs refactor EmbedRecordNode ? → **Nouveau nœud**

Nouveau EmbedNode séparé. Pas de refactor d'EmbedRecordNode — il reste tel quel pour le pipeline KB. La logique d'embedding partagée (appel embedder, batching GPU) peut être extraite dans des helpers communs.

### Q2 — VectorSearchNode : comment faire la similarité cosinus ? → **Réutiliser le code existant**

`Catalog::search()` fait déjà du vector search via `search_vector()`. Extraire cette logique en helper réutilisable (embed query → Cypher avec cosinus → tri par score) que VectorSearchNode appelle directement, sans passer par le Catalog. Même performance que le pipeline KB.

### Q3 — SearchSourceNode vs QuerySourceNode ? → **Nouveau SearchSourceNode**

Nouveau nœud `SearchSourceNode` séparé. QuerySourceNode reste inchangé pour le pipeline KB. SearchSourceNode émet un Query avec `entity` au lieu de `kb_name`. Ça implique probablement un nouveau variant pour PortValue::Query ou un champ optionnel, à déterminer à l'implémentation.

### Q4 — FlushFTSNode générique ? → **Nouveau FlushNode + renommage KB_**

Nouveau `FlushNode` avec config explicite (`table_name`). L'ancien reste pour le pipeline KB.

**Renommage systématique** : tous les nœuds KB-spécifiques sont préfixés `KB_` pour les distinguer clairement des nœuds génériques :

| Nom actuel | Nouveau nom |
|------------|-------------|
| GatherKBNode | KBGatherNode |
| UpdateKBNode | KBUpdateNode |
| ChunkKBNode | KBChunkNode |
| FlushFTSNode | KBFlushNode |
| QuerySourceNode | KBQuerySourceNode |
| PrimarySearchNode | KBSearchNode |

Les nœuds génériques gardent des noms simples : InsertRecordNode, ChunkRecordNode, EmbedNode, VectorSearchNode, BM25SearchNode, etc.

Dans les templates Mermaid, ça donne immédiatement la visibilité :
```
%% Pipeline KB
kb_search["KBSearchNode"]

%% Pipeline simple
vector["VectorSearchNode(entity='Product_Chunk')"]
```

### Q5 — Détection automatique dans search_entities ? → **Par signaux, comme le Catalog actuel**

Même modèle que le Catalog KB : les `SearchSignals` (BM25, Vector, Sparse) déterminent quels nœuds de recherche sont activés. Pas d'introspection schema — l'utilisateur configure les signaux (ou utilise le défaut Hybrid). Le reste (colonne d'embedding, table de chunks) suit la convention de nommage établie à l'ingestion.

### Q6 — Content change tracking sans KB ? → **Réutiliser `_embed_hash`**

EmbedRecordNode utilise déjà `_embed_hash` pour skip les entités dont le contenu n'a pas changé. Le nouveau EmbedNode fait pareil sur les chunks : hasher le contenu texte, comparer avec `_embed_hash` stocké, skip si identique. Même mécanisme, appliqué aux chunks au lieu des entités KB.
