# Doc 22 — Réflexion : Nœuds génériques sans concept KB

Date : 8 mars 2026

## 1. Le problème

Le framework dataflow a deux couches :

**Couche basse (générique)** — InsertRecordNode, LinkRecordNode, ChunkRecordNode : fonctionnent sur n'importe quelle entité. Pas de concept KB.

**Couche haute (KB-spécifique)** — KBGatherNode, KBUpdateNode, KBChunkNode, KBEmbedNode, KBQuerySourceNode, KBSearchNode : hardcodés sur l'architecture `{KB}_Index` / `{KB}_Index_Chunk`.

Le problème : on ne peut pas construire un pipeline simple "entités → chunks → embed → search" sans passer par l'abstraction KB. Un utilisateur qui veut juste :

```
Insérer des documents → les chunker → les embedder → les rechercher
```

...est forcé de créer une KB, définir un title_entity, des content_fields, des relations, etc. C'est de la complexité inutile pour les cas simples.

---

## 2. État actuel : couplage KB par nœud

> **Mise à jour (8 mars 2026)** : Le renommage KB a été effectué. Les nœuds KB-spécifiques sont préfixés `KB`, FlushNode a été rendu générique (config `tables`), EmbedRecordNode renommé KBEmbedNode.

| Nœud | Générique ? | Couplage KB | Statut |
|------|-------------|-------------|--------|
| InsertRecordNode | ✅ Oui | Aucun | — |
| LinkRecordNode | ✅ Oui | Aucun | — |
| ChunkRecordNode | ✅ Oui | Aucun | — |
| FlushNode | ✅ Oui | Aucun (config `tables` explicite) | ✅ Refactoré (ex-FlushFTSNode) |
| FetchRelatedNode | ✅ Oui | Aucun | — |
| ComposeNode | ✅ Oui | Aucun | — |
| KBEmbedNode | ❌ Non | `_kb_name`, `{kb}_embedding`, `config.knowledge_bases` | ✅ Renommé (ex-EmbedRecordNode) |
| KBGatherNode | ❌ Non | Total (title_entity, content_fields, relations) | ✅ Renommé |
| KBUpdateNode | ❌ Non | Total (`{KB}_Index`, `{KB}_Index_Chunk`) | ✅ Renommé |
| KBChunkNode | ❌ Non | Total (`{KB}_Index_Chunk`) | ✅ Renommé |
| KBQuerySourceNode | ❌ Non | Prend un `kb_name` | ✅ Renommé |
| KBSearchNode | ❌ Non | Appelle `Catalog::search()` hardcodé KB | ✅ Renommé |

**Score** : 6 nœuds génériques, 6 nœuds KB-only (clairement séparés par préfixe).

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

> **Principe : chunks toujours présents.** L'ingestion crée toujours au moins 1 chunk par entité (même pour un texte court). Pas de branche "sans chunks".

> **BM25 sur le contenu complet, vector sur les chunks.** Comme dans le pipeline KB : BM25 cherche sur l'entité parent (contenu complet → meilleur TF-IDF), puis résout les highlights vers les chunks. Vector cherche directement sur les chunks. RRF fusionne au niveau chunk.

**Vector search** — cherche sur les chunks :
```
graph LR
    query["SearchSourceNode(query='$query')"]
    search["VectorSearchNode(entity='Document_Chunk', embedding_col='embedding', limit=10)"]
    resolve["ResolveSourceNode(relation='CHUNKED_FROM')"]

    query -->|query| search
    search -->|results| resolve
```

**BM25 search** — cherche sur l'entité (contenu complet), résout par highlights vers les chunks :
```
graph LR
    query["SearchSourceNode(query='$query')"]
    bm25["BM25SearchNode(entity='Document', fields='description,name', chunk_entity='Document_Chunk', limit=10)"]
    resolve["ResolveSourceNode(relation='CHUNKED_FROM')"]

    query -->|query| bm25
    bm25 -->|results| resolve
```

> BM25SearchNode encapsule le flow `search_bm25_raw()` + `resolve_bm25_to_chunks()` : BM25 sur l'entité parent → parse highlights → match byte ranges aux chunks → retourne des résultats chunk-level. Même logique que `search_bm25_chunked()` dans le Catalog KB.

**Différence avec le pipeline KB** :
- Pas de Catalog::search() — recherche directe via Cypher/Lucivy
- Pas de KB metadata à résoudre
- BM25SearchNode fait la résolution highlights→chunks en interne (comme `search_bm25_chunked()`)
- ResolveSourceNode remplace le `ResultMode::SourceResolved` du Catalog

---

## 4. Nouveaux nœuds nécessaires

### 4.1 EmbedNode (nouveau nœud générique)

> **Décision** : Nouveau nœud séparé de KBEmbedNode (ex-EmbedRecordNode, renommé). KBEmbedNode reste inchangé pour le pipeline KB. La logique d'embedding partagée (appel embedder, batching GPU) sera extraite dans des helpers communs.

Le nœud opère toujours sur les chunks (`{Entity}_Chunk`). Le champ texte est `_text` par convention (rempli par ChunkRecordNode).

```
embed["EmbedNode(text_field='_text', embedding_col='embedding', signals='vector')"]
```

| Config | Type | Défaut | Description |
|--------|------|--------|-------------|
| `text_field` | String | `"_text"` | Champ texte à embedder (convention chunks) |
| `embedding_col` | String | `"embedding"` | Colonne pour le vecteur dense |
| `sparse_col` | String | `""` | Colonne pour le sparse (si vide, pas de sparse) |
| `signals` | String | `"vector"` | Quels embeddings : `vector`, `bm25`, `sparse`, `hybrid` |
| `gpu_batch_size` | usize | 32 | Taille des batches GPU |

L'entity n'est pas en config — elle est déduite des entités reçues sur le port `entities` (comme InsertRecordNode).

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

Recherche BM25 sur le **contenu complet de l'entité parent** (pas les chunks), puis résolution des highlights vers les chunks. Même logique que `search_bm25_chunked()` dans le Catalog KB.

L'index FTS est créé sur l'entité parent (ex: `Product`) car BM25 a besoin du contenu complet pour un bon scoring TF-IDF. Les highlights (byte offsets des matches) sont ensuite matchés aux chunk byte ranges pour obtenir des résultats chunk-level, ce qui permet la fusion RRF avec les résultats vector (eux aussi chunk-level).

```rust
pub struct BM25SearchNode {
    entity: String,        // "Document" — table parent (contenu complet)
    chunk_entity: String,  // "Document_Chunk" — table chunks (résolution)
    fields: Vec<String>,   // ["description", "name"] — champs FTS de l'entité
    limit: usize,
}
```

**Exécution** (encapsule `search_bm25_raw` + `resolve_bm25_to_chunks`) :
1. `QUERY_LUCIVY_INDEX('{entity}', query, limit)` → scores + highlights par champ
2. Parse highlights : `HashMap<field, Vec<(start_byte, end_byte)>>`
3. Fetch chunks de chaque parent hit : `MATCH (p:{entity})-[:CHUNKED_FROM]-(c:{chunk_entity}) WHERE p._uuid IN [...]`
4. Match highlights aux chunks via `_content_offset + _start_char/_end_char` (overlap)
5. Retourner comme Vec<UnifiedResult> au **niveau chunk** (prêt pour RRF)

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

### 5.2 `simple_search.mmd` — Vector search sur chunks + resolve

```
graph LR
    source["SearchSourceNode(query='$query')"]
    vector["VectorSearchNode(entity='$entity_chunk', embedding_col='$embedding_col', limit=$limit)"]
    resolve["ResolveSourceNode(relation='CHUNKED_FROM')"]

    source -->|query| vector
    vector -->|results| resolve
```

### 5.3 `hybrid_search.mmd` — Vector + BM25 + fusion + resolve

> Vector cherche sur `$entity_chunk` (embeddings), BM25 cherche sur `$entity` (contenu complet) et résout les highlights vers `$entity_chunk`. Les deux produisent des résultats chunk-level → RRF fusionne → resolve vers l'entité source.

```
graph LR
    source["SearchSourceNode(query='$query')"]
    vector["VectorSearchNode(entity='$entity_chunk', embedding_col='$embedding_col', limit=$limit)"]
    bm25["BM25SearchNode(entity='$entity', chunk_entity='$entity_chunk', fields='$fields', limit=$limit)"]
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
// KB (agrégation multi-source)
catalog.createKB("Products", {
    titleEntity: "Product",
    contentFields: ["name", "description"],
    relations: [{ name: "HAS_CATEGORY", direction: "Outgoing" }],
});
await catalog.ingest("Products", entities);
const results = await catalog.search("Products", "red shoes");

// Simple (une seule entité, pas de KB)
catalog.registerEntity("Product", {
    fields: { name: { type: "text", isTitle: true }, description: { type: "text", isContent: true } },
    signals: "hybrid",
});
await catalog.ingestEntities("Product", entities);
const results = await catalog.search("Product", "red shoes");  // même API !
```

> **`catalog.search()` est unifié** — fonctionne pour les KB et les entités simples. Le catalog sait en interne si le nom correspond à un KB ou une entité simple, et résout les bons noms de tables. La logique de recherche (BM25 → highlights → chunks, vector → chunks, RRF fusion) est identique.

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
2. `CREATE NODE TABLE IF NOT EXISTS Product_Chunk(_uuid STRING, _text STRING, _title STRING, _embed_hash STRING, _start_char INT64, _end_char INT64, _content_offset INT64, _index INT64, _parent_field STRING, embedding DOUBLE[], sparse_indices INT64[], sparse_weights DOUBLE[], PRIMARY KEY(_uuid))`
3. `CREATE REL TABLE IF NOT EXISTS Product_CHUNKED_FROM(FROM Product_Chunk TO Product)`
4. `CREATE_LUCIVY_INDEX('Product', ['description', 'details'])` — FTS sur l'**entité** (contenu complet), pas les chunks
5. Stocke la config en mémoire (comme `kb_metadata` pour les KB)

> **Pourquoi FTS sur l'entité et pas les chunks ?** BM25 a besoin du contenu complet pour un scoring TF-IDF correct. Les highlights (byte offsets) sont ensuite résolus vers les chunks via `_content_offset + _start_char/_end_char`, ce qui permet la fusion RRF avec les résultats vector (chunk-level). C'est exactement ce que fait le pipeline KB avec `search_bm25_raw()` + `resolve_bm25_to_chunks()`.

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
    /// Crée toujours la table de chunks + relation + index FTS sur l'entité.
    pub async fn register_entity(
        &mut self,
        entity: &str,
        config: EntityConfig,
    ) -> Result<(), CatalogError>;

    /// Ingestion directe : insert + chunk (toujours, 1 minimum) + embed, sans KB.
    ///
    /// Construit et exécute un DataflowGraph en interne :
    ///   InsertRecordNode → ChunkRecordNode → InsertRecordNode (chunks)
    ///                                      → LinkRecordNode (CHUNKED_FROM)
    ///                                      → EmbedNode (sur les chunks)
    pub async fn ingest_entities(
        &self,
        entity: &str,
        records: Vec<EntityRecord>,
    ) -> Result<IngestResult, CatalogError>;

    /// search() existant — déjà implémenté pour les KB.
    /// Étendu pour supporter aussi les entités simples (registerEntity).
    ///
    /// Le catalog résout en interne :
    ///   - KB "Products"  → tables: Products_Index / Products_Index_Chunk
    ///   - Entity "Product" → tables: Product / Product_Chunk
    ///
    /// La logique de recherche est identique :
    ///   BM25 sur parent (contenu complet) → highlights → chunks
    ///   Vector sur chunks (embeddings)
    ///   RRF fusion → resolve vers source
    pub async fn search(
        &self,
        name: &str,  // KB name ou entity name
        query: &str,
        opts: SearchOptions,
    ) -> Result<Vec<SearchResult>, CatalogError>;
}
```

**Filtrage à la recherche** — tout champ est filtrable via `SearchOptions.filters`, sans déclaration :
```typescript
const results = await catalog.search("Product", "red shoes", {
    filters: {
        price: [{ gte: 10 }, { lte: 100 }],
        category: "shoes",
    },
});
// → Cypher pré-filtre sur Product → allowed_ids
// → BM25 sur Product (contenu complet) → highlights → Product_Chunk
// → vector search sur Product_Chunk avec allowed_ids
// → RRF fusion → resolve vers Product
```

### 8.3 Résolution interne unifiée

> `catalog.search(name, query)` fonctionne pour les KB et les entités simples. Le catalog résout les noms de tables en interne.

| | KB (`createKB("Products")`) | Simple (`registerEntity("Product")`) |
|---|---|---|
| Table parent (BM25) | `Products_Index` | `Product` |
| Table chunks (vector) | `Products_Index_Chunk` | `Product_Chunk` |
| Relation chunks | `Products_Index_HAS_CHUNK` | `Product_CHUNKED_FROM` |
| FTS index | sur `Products_Index` | sur `Product` |

La logique de recherche est identique dans les deux cas :
1. **BM25** sur table parent → highlights → résolution vers chunks
2. **Vector** sur table chunks → résultats chunk-level
3. **RRF** fusion au niveau chunk
4. Résolution vers entité source si demandé

```typescript
// Les deux passent par catalog.search() — même code
const kbResults = await catalog.search("Products", "red shoes");
const simpleResults = await catalog.search("Product", "red shoes");
```

### 8.4 Deux modes, une seule API de recherche

| | KB (`createKB`) | Simple (`registerEntity`) |
|---|---|---|
| **Setup** | `createKB()` + schéma EntityDef | `registerEntity()` self-contained |
| **Ingestion** | `ingest()` — agrégation multi-source | `ingestEntities()` — insert + chunk + embed direct |
| **Recherche** | **`search()`** | **`search()`** — même API ! |
| **FTS** | Sur `{KB}_Index` (contenu agrégé) | Sur `{Entity}` (contenu complet) |
| **Content tracking** | `_content_hash` sur `{KB}_Index` | `_embed_hash` sur les chunks |
| **Complexité** | Haute (title_entity, relations) | Basse (fields + signals) |
| **Cas d'usage** | Agrégation multi-source | Entités simples, prototypage rapide |

L'utilisateur commence avec `registerEntity` + `search` (simple). S'il a besoin d'agrégation multi-source, il migre vers `createKB` — le `search()` ne change pas.

### 8.5 Impact Node.js / WASM

Nouvelles méthodes à exposer dans les bindings :

**Node.js** (rag3dbjs) :
```typescript
interface Catalog {
    // Existant (inchangé)
    createKB(name: string, config: KBConfig): Promise<void>;
    ingest(kb: string, records: EntityRecord[]): Promise<IngestResult>;
    search(name: string, query: string, opts?: SearchOptions): Promise<SearchResult[]>;  // unifié KB + simple

    // Nouveau
    registerEntity(entity: string, config: EntityConfig): Promise<void>;
    ingestEntities(entity: string, records: EntityRecord[]): Promise<IngestResult>;
    // search() fonctionne déjà pour les entités simples — pas de searchEntities() !
}
```

**WASM** : même interface via wasm-bindgen.

---

## 9. Priorisation

> **Prérequis fait** : Renommage KB-spécifiques (préfixe KB) + FlushNode générique. Voir rapport session `02-rapport-session-renommage-kb.md`.

| Étape | Quoi | Effort | Dépend de | Statut |
|-------|------|--------|-----------|--------|
| **—** | Renommage KB + FlushNode générique | — | — | ✅ Fait |
| **A** | Deserialize sur types search | ~0.5j | Rien | À faire |
| **B** | EmbedNode (version simple) | ~1j | Rien | À faire |
| **C** | VectorSearchNode | ~1-2j | Deserialize (A) | À faire |
| **D** | BM25SearchNode | ~1j | Idem | À faire |
| **E** | ResolveSourceNode | ~0.5j | Rien | À faire |
| **F** | FuseResultsNode | ~0.5j | Rien | À faire |
| **G** | Templates simples (.mmd) | ~0.5j | B, C, D, E | À faire |
| **H** | Catalog::register_entity / ingest_entities + unifier search() | ~1-2j | B, C, D, E, F | À faire |
| **I** | ScriptNode (Rhai) | ~2j | A | À faire |
| **J** | HttpNode | ~1j | Rien | À faire |

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

### Q1 — EmbedNode vs refactor EmbedRecordNode ? → **Nouveau nœud** ✅ Décidé

Nouveau EmbedNode séparé. KBEmbedNode (ex-EmbedRecordNode, renommé) reste inchangé pour le pipeline KB. La logique d'embedding partagée (appel embedder, batching GPU) sera extraite dans des helpers communs.

### Q2 — VectorSearchNode : comment faire la similarité cosinus ? → **Réutiliser le code existant**

`Catalog::search()` fait déjà du vector search via `search_vector()`. Extraire cette logique en helper réutilisable (embed query → Cypher avec cosinus → tri par score) que VectorSearchNode appelle directement, sans passer par le Catalog. Même performance que le pipeline KB.

### Q3 — SearchSourceNode vs QuerySourceNode ? → **Nouveau SearchSourceNode**

Nouveau nœud `SearchSourceNode` séparé. QuerySourceNode reste inchangé pour le pipeline KB. SearchSourceNode émet un Query avec `entity` au lieu de `kb_name`. Ça implique probablement un nouveau variant pour PortValue::Query ou un champ optionnel, à déterminer à l'implémentation.

### Q4 — FlushFTSNode générique ? → ✅ Fait (FlushNode refactoré)

**Implémenté** : FlushFTSNode a été refactoré en `FlushNode` générique avec config `tables: Vec<String>` dans le constructeur. Pas de "nouveau + ancien" — l'ancien a été transformé. Le service `flush_kb_names` a été supprimé. Les tables sont passées directement au constructeur. `node_config()` sérialise pour checkpoint/restore. Factory `FlushNodeFactory` accepte `table` (single) ou `tables` (array).

**Renommage systématique** ✅ Fait :

| Ancien nom | Nouveau nom | Statut |
|------------|-------------|--------|
| GatherKBNode | KBGatherNode | ✅ |
| UpdateKBNode | KBUpdateNode | ✅ |
| ChunkKBNode | KBChunkNode | ✅ |
| FlushFTSNode | FlushNode (générique) | ✅ Refactoré |
| EmbedRecordNode | KBEmbedNode | ✅ |
| QuerySourceNode | KBQuerySourceNode | ✅ |
| PrimarySearchNode | KBSearchNode | ✅ |

489 tests unitaires + 89 tests E2E : 0 régressions.

### Q5 — Détection automatique dans search_entities ? → **Conventions, pas détection** ✅ Simplifié

Les chunks sont toujours présents (1 minimum). Pas de détection "si chunks existent" — les conventions sont fixées par `registerEntity` :
- Table de chunks : `{Entity}_Chunk` (embeddings, offsets)
- Embedding : `{Entity}_Chunk.embedding`
- FTS : index Lucivy sur **`{Entity}`** (contenu complet, pas les chunks) — pour un scoring BM25 correct
- Relation : `{Entity}_CHUNKED_FROM`
- Résolution BM25 : highlights (byte offsets) → match aux chunks via `_content_offset + _start_char/_end_char`

Les `SearchSignals` (BM25, Vector, Sparse) déterminent quels nœuds de recherche sont activés. Défaut : Hybrid.

### Q6 — Content change tracking sans KB ? → **Réutiliser `_embed_hash`**

EmbedRecordNode utilise déjà `_embed_hash` pour skip les entités dont le contenu n'a pas changé. Le nouveau EmbedNode fait pareil sur les chunks : hasher le contenu texte, comparer avec `_embed_hash` stocké, skip si identique. Même mécanisme, appliqué aux chunks au lieu des entités KB.
