# 02 — Usages réels envisagés pour rag3weaver

Vision complète des cas d'usage concrets à mettre en place, basée sur les expérimentations passées (ragforge-core/Neo4j, kuzu-wasm-exp, agent-configurator) et la roadmap rag3weaver.

---

## A. Ingestion GitHub — Code RAG structuré

### Ce qui existait (ragforge-core + Neo4j)

L'endpoint `POST /ingest/github` dans community-docs :
1. **Clone** le repo (git clone + submodules) dans un répertoire temporaire
2. **Scan** les fichiers code via `getCodeFilesFromDir()` (filtres par extension)
3. **Parse** avec `@luciformresearch/codeparsers` (TypeScript, worker threads) :
   - `ProjectParser.parseProject({ root, files, resolveRelationships: true })`
   - Extraction AST via tree-sitter → `ScopeFileAnalysis` par fichier
   - Résolution des relations inter-fichiers via `RelationshipResolver`
4. **Ingestion** dans Neo4j via `UnifiedProcessor` + `orchestrator.ingestVirtualSimplified()`
5. **Streaming SSE** pour le progrès en temps réel

**Entités créées dans Neo4j :**

| Entité | Champs clés |
|--------|-------------|
| `File` | path, name, extension, language, fileHash, size |
| `Scope` | name, type (function/class/method/...), signature, content, docstring, startLine, endLine, linesOfCode |
| `ExternalLibrary` | name (package importé) |
| `VueSFC` | componentName, templateSource, imports, usedComponents |
| `SvelteComponent` | componentName, scriptSource, imports |
| `MarkdownDocument` | file, content, path |
| `Stylesheet` | file, source, ruleCount, variableCount |
| `WebDocument` | file, content, title |
| `PackageJson` | name, version, description, dependencies |

**Relations créées :**

| Relation | From → To | Signification |
|----------|-----------|---------------|
| `DEFINED_IN` | Scope → File | Le scope vit dans ce fichier |
| `CONSUMES` | Scope → Scope | Appelle / utilise un autre scope |
| `CONSUMED_BY` | Scope → Scope | Inverse (optionnel) |
| `PARENT_OF` | Scope → Scope | Container → enfant (class → method) |
| `INHERITS_FROM` | Class → Class | Héritage |
| `IMPLEMENTS` | Class → Interface | Implémentation |
| `DECORATED_BY` | Scope → Decorator | Décorateur appliqué |
| `USES_LIBRARY` | Scope → ExternalLibrary | Import externe |

**Stats typiques :** sur codeparsers lui-même (15 fichiers TS), ~120 scopes, ~200 relations, en ~2s.

### Ce qui existe déjà (codeparsers Rust dans rag3weaver)

Le crate `codeparsers` dans `extension/rag3weaver/codeparsers/` est la **version Rust transpilée** (via codeparsers-transpiler) avec :

**Langages supportés :** TypeScript/JavaScript, Python, Rust, Go, C, C++, C#

**Pipeline :** `ProjectParser::parse_project()` → parsing Rayon parallèle → `ScopeFileAnalysis` + `RelationshipResolutionResult`

**Extraction de scopes :** 12 types — Class, Interface, Function, Method, Enum, TypeAlias, Namespace, Module, Variable, Lambda, Constant, Block

**Chaque scope contient :**
- `signature`, `content`, `content_dedented`, `docstring`
- `scope_start_line`, `scope_end_line`, `body_start_line`, `body_end_line`
- `parameters` (avec types, optionnels, valeurs par défaut)
- `return_type`, `generic_parameters`, `heritage_clauses`
- `decorator_details`, `members` (pour classes)
- `imports`, `exports`, `dependencies`
- `identifier_references` (avec kind : Import/LocalScope/Builtin/Unknown)
- `complexity`, `lines_of_code`, `ast_valid`, `ast_issues`

**Relations résolues :** CONSUMES, CONSUMED_BY, INHERITS_FROM, IMPLEMENTS, PARENT_OF, HAS_PARENT, DECORATES, DECORATED_BY, DEFINED_IN, USES_LIBRARY — avec `ResolvedRelationship` contenant from_uuid, to_uuid, from_file, to_file, metadata (via_import, import_path, clause, etc.)

**Parsers non-code :** Markdown, HTML, CSS/SCSS, SVG (via `NonCodeProjectParser` / modules dédiés)

### Ce qu'il faut faire pour rag3weaver

**Objectif :** Reproduire le pipeline `POST /ingest/github` mais nativement en Rust dans rag3weaver, sans Neo4j.

```
gh API / git clone
    ↓
codeparsers::ProjectParser::parse_project()
    ↓
Conversion en entités rag3weaver:
    File    → catalog.create("File", { path, language, linesOfCode })
    Scope   → catalog.create("Scope", { name, scopeType, signature, content, docstring, ... })
    Library → catalog.create("Library", { name, importPath })
    ↓
Relations rag3weaver:
    catalog.link("DEFINED_IN", scope_ref, file_ref)
    catalog.link("CONSUMES", scope_ref1, scope_ref2)
    catalog.link("INHERITS_FROM", class_ref, parent_ref)
    catalog.link("USES_LIBRARY", scope_ref, lib_ref)
    ↓
catalog.drain() → chunk, embed, index FTS, index sparse, store
    ↓
Search:
    catalog.search("ScopeKB", "error handling middleware", signals: BM25|Vector|Sparse)
    catalog.search_with_explore("ScopeKB", query, depth=2, relations=["CONSUMES", "DEFINED_IN"])
```

**Knowledge Bases envisagées :**

| KB | titleFor | contentFor | Strategy |
|----|----------|------------|----------|
| ScopeKB | signature | content + docstring (chunked) | hybrid (BM25 + vector + sparse) |
| FileKB | path | — | fulltext only |
| LibraryKB | name | importPath | fulltext only |

**Tâches concrètes :**
1. Fonction `ingest_github(url, branch, options)` dans rag3weaver
2. Clone via `git2` crate ou shell `git clone`
3. Scan des fichiers par extension (réutiliser `EXTENSION_TO_LANGUAGE`)
4. Appel `ProjectParser::parse_project()` avec le content_map
5. Conversion `ProjectAnalysis` → séries de `catalog.create()` + `catalog.link()`
6. `catalog.drain()` pour pipeline complet
7. Expose en Node.js via FFI pour l'API REST (SSE streaming)

---

## B. Ingestion documents binaires — OCR + extraction

### Ce qui existait (ragforge-core)

Un pipeline complet de parsing de documents avec OCR multi-provider :

**Formats supportés :**
- **PDF** : extraction texte (pdfjs-dist) + fallback OCR (Tesseract offline → Gemini Vision si confidence < 60%)
- **DOCX** : extraction via mammoth (texte + HTML + images embarquées)
- **XLSX/XLS** : parsing via xlsx (feuilles, headers, données)
- **CSV** : parsing direct
- **Images** : PNG, JPG, GIF, WebP, SVG, BMP, TIFF — avec analyse Vision optionnelle
- **3D** : GLTF/GLB — métadonnées + rendu multi-vues + description synthétisée

**Providers OCR (3 implémentés) :**

| Provider | Modèle | Usage |
|----------|--------|-------|
| Tesseract.js | — | Gratuit, offline, primaire pour PDF image-only |
| Gemini Vision | gemini-2.0-flash | Fallback qualité si Tesseract < 60% confidence |
| Claude Vision | claude-3-5-haiku | Alternative, batch concurrent |
| Replicate | deepseek-vl2 (DeepSeek-OCR) | 97% accuracy, layout-aware |

**Stratégie free-first :**
1. PDF avec texte → extraction directe (coût zéro)
2. PDF image-only → Tesseract OCR (offline, gratuit)
3. Si confidence < 60% → flag `needsGeminiVision` (lazy, pas automatique)
4. Images embarquées → Vision on-demand

**Structure 3-tiers dans Neo4j :**

```
File (metadata original)
  └─ DERIVED_FROM → MarkdownDocument (container parsé)
       └─ IN_DOCUMENT → MarkdownSection[] (sections avec contenu)
```

**MarkdownSection** : file, title, titleLevel, content, index, startLine, endLine, pageNum, type — chunking 4000 chars / 400 overlap.

### Ce qu'il faut faire pour rag3weaver

**Objectif :** Pipeline d'ingestion documents binaires → entités rag3weaver avec search hybride.

```
Document binaire (PDF, DOCX, image)
    ↓
Format detection + parsing:
    PDF  → pdfjs / pdf-extract (Rust) → texte pages
    DOCX → docx-rs → texte + images
    Image → OCR provider → texte
    ↓
Sectionnement:
    Markdown heading detection → sections
    Ou chunking fixe si pas de headings
    ↓
Entités rag3weaver:
    Document → catalog.create("Document", { title, sourceFormat, pageCount, author })
    Section  → catalog.create("Section", { title, content, startPage, endPage })
    ↓
Relations:
    catalog.link("IN_DOCUMENT", section_ref, doc_ref)
    catalog.link("HAS_FILE", directory_ref, doc_ref)  // si arborescence
    ↓
catalog.drain() → chunk sections, embed, index FTS
```

**Knowledge Bases :**

| KB | titleFor | contentFor | Strategy |
|----|----------|------------|----------|
| DocumentKB | title | section.content (chunked) | hybrid |

**OCR côté Rust — options :**
1. **Tesseract binding** (`leptess` crate) — gratuit, offline
2. **API Vision** — appels HTTP vers Gemini/Claude (même stratégie free-first)
3. **pdf-extract / lopdf** — extraction texte PDF en Rust natif

**Tâches concrètes :**
1. Crate `document-parser` dans rag3weaver (ou module interne)
2. Détection format par extension/magic bytes
3. Extraction texte selon format (PDF Rust natif, DOCX via zip + XML, etc.)
4. OCR fallback configurable (Tesseract local ou API Vision)
5. Sectionnement intelligent (headings, paragraphes)
6. Conversion en entités rag3weaver

---

## C. Composio — Connecteurs SaaS (Shopify, Google Drive, Gmail...)

### Ce qui existait (agent-configurator backend)

L'agent-configurator utilise **Composio** (SDK Python) pour connecter des services SaaS via OAuth, avec un agent **Google ADK** (Gemini 2.5 Flash) qui orchestre les tool calls.

**Architecture :**
```
User → Agent (Google ADK + Gemini)
         ↓
    Tool assembly:
    ├── Composio tools (MCP protocol) — OAuth-connected
    │   └── SHOPIFY_GRAPH_QL_QUERY, GMAIL_SEND, etc.
    ├── Qdrant tools — search_products()
    ├── Memory tools — conversation history
    └── Custom tools — business logic
```

**25+ toolkits Composio :** github, gitlab, jira, linear, sentry, slack, discord, gmail, outlook, notion, trello, asana, google_docs, google_sheets, hubspot, salesforce, google_calendar, google_drive, dropbox, serpapi, firecrawl, shopify, spotify...

**Flow OAuth :**
1. `POST /api/tools/{tool}/connect` → Composio `initiate()` → redirect_url
2. User complète OAuth dans le browser
3. `GET /api/oauth/callback` → trigger indexation en background
4. Agent peut maintenant utiliser le toolkit connecté

### Shopify spécifiquement

**Ingestion produits (déjà implémenté en Python) :**
1. GraphQL via `SHOPIFY_GRAPH_QL_QUERY` (cursor pagination, 250/batch)
2. Extraction : id, title, vendor, type, tags, prix, variantes, images, inventaire
3. Conversion variantes → sizes, colors, prix min/max, promotions

**Dual ingestion Qdrant :**
- **Document titre** : `product.title + vendor + type` (1 doc/produit)
- **Document contenu** : `description + tags + variant info` (chunked si > 512 mots)

**Métadonnées filtrables :** product_id, title, price_min, price_max, on_sale, inventory, vendor, product_type, tags, sizes, colors, image_url

**Search :** hybride semantic + keyword, filtres par prix/vendor/tags/stock, tri optionnel.

### Ce qu'il faut faire pour rag3weaver

**Objectif :** Ingestion Shopify (et autres SaaS) directement dans rag3db, pas Qdrant.

```
Composio OAuth → Shopify GraphQL
    ↓
Fetch products (cursor pagination)
    ↓
Entités rag3weaver:
    Product → catalog.create("Product", {
        title, description, vendor, productType,
        priceMin, priceMax, onSale, inventory,
        tags: ["GPS", "Premium", ...],
        imageUrl
    })
    Variant → catalog.create("Variant", {
        title, price, sku, inventoryQuantity,
        size, color
    })
    Collection → catalog.create("Collection", { title, description })
    ↓
Relations:
    catalog.link("HAS_VARIANT", product_ref, variant_ref)
    catalog.link("IN_COLLECTION", product_ref, collection_ref)
    ↓
catalog.drain()
```

**Knowledge Bases :**

| KB | titleFor | contentFor | Strategy |
|----|----------|------------|----------|
| ProductKB | title | description + tags (hybrid) | hybrid 3-way |
| CollectionKB | title | description | hybrid |

**Filter fields Lucivy :**
- `vendor` (String) — `category="Nike"`
- `product_type` (String) — `category="shoes"`
- `price_min`, `price_max` (Double) — range queries
- `inventory` (Int64) — stock filtering
- `on_sale` (Bool) — promotions only

**Avantage rag3db vs Qdrant :** relations de graphe natives (Product→Variant, Product→Collection, Collection→SubCollection), explore BFS pour naviguer le catalogue, filtres Lucivy natifs en pré-filter.

**Pattern agentique :**
L'agent peut faire : `search("ProductKB", "chaussures de running rouges", filters: { price_max: 100, in_stock: true })` puis explorer le graphe `Product→Collection` pour recommander des produits similaires dans la même collection.

---

## D. Google Drive / arborescences fichiers

### Vision

Ingestion d'une arborescence complète (Google Drive, Dropbox, ou filesystem local) avec les relations hiérarchiques préservées.

```
Google Drive API / local filesystem
    ↓
Scan arborescence récursif
    ↓
Entités:
    Directory → catalog.create("Directory", { name, path, depth })
    File      → catalog.create("File", { name, path, extension, size, mimeType, modifiedAt })
    ↓
Relations hiérarchiques:
    catalog.link("CONTAINS", parent_dir_ref, child_dir_ref)
    catalog.link("HAS_FILE", dir_ref, file_ref)
    ↓
Pour chaque fichier texte/document:
    Extraction contenu → catalog.update(file_ref, { content: extractedText })
    Si document binaire → OCR pipeline (voir section B)
    ↓
Détection mentions dans le contenu:
    URLs   → catalog.create("URL", { url, domain }) + catalog.link("MENTIONS_URL", file, url)
    Emails → catalog.create("Contact", { email, name }) + catalog.link("MENTIONS_CONTACT", file, contact)
    ↓
catalog.drain()
```

**Knowledge Bases :**

| KB | titleFor | contentFor | Strategy |
|----|----------|------------|----------|
| FileContentKB | name | content (chunked) | hybrid |
| DirectoryKB | path | — | fulltext |

**Search patterns :**
- "tous les fichiers qui parlent d'authentification" → BM25+vector sur FileContentKB
- "documents modifiés cette semaine dans /src/" → filtres Lucivy (date, path prefix)
- "quel fichier mentionne ce contact ?" → explore BFS File→MENTIONS_CONTACT→Contact

---

## E. Gmail / Outlook — Ingestion mails

### Vision

```
Gmail API / Outlook API (via Composio)
    ↓
Fetch threads/messages (pagination, date range)
    ↓
Entités:
    Mail → catalog.create("Mail", {
        subject, body, date, threadId,
        from, to, cc, hasAttachments
    })
    Contact → catalog.create("Contact", { email, name, domain })
    Attachment → catalog.create("Attachment", { filename, mimeType, size })
    ↓
Relations:
    catalog.link("FROM", mail_ref, contact_ref)
    catalog.link("TO", mail_ref, contact_ref)
    catalog.link("CC", mail_ref, contact_ref)
    catalog.link("HAS_ATTACHMENT", mail_ref, attachment_ref)
    catalog.link("IN_THREAD", mail_ref, thread_ref)  // conversation threading
    catalog.link("MENTIONS_URL", mail_ref, url_ref)   // URLs dans le body
    ↓
Pour les attachments documents:
    Extraction contenu via pipeline OCR (section B)
    catalog.link("DERIVED_FROM", document_ref, attachment_ref)
    ↓
catalog.drain()
```

**Knowledge Bases :**

| KB | titleFor | contentFor | Strategy |
|----|----------|------------|----------|
| MailKB | subject | body (chunked pour longs threads) | hybrid |
| ContactKB | name | email + domain | fulltext |

**Filter fields :** date (range), from_domain (String), has_attachments (Bool), thread_id (String)

**Search patterns :**
- "mails de Jean sur le contrat Shopify" → hybrid MailKB + filtre contact
- "toutes les pièces jointes PDF du mois dernier" → filtre Lucivy date + mimeType
- "qui a répondu au thread sur le budget ?" → explore Mail→IN_THREAD + Mail→FROM→Contact

---

## F. Schéma universel — Catalogues configurables

### Vision originale (kuzu-wasm-exp)

Le document `universal-catalog-schema.md` définit un schéma YAML universel pour **tout type de catalogue** (voitures, immobilier, restaurants, jobs, événements, e-commerce) sans code spécifique :

```yaml
catalog:
  name: "Mon Garage Auto"
  type: vehicles

knowledge_bases:
  AnnoncesKB:
    search: hybrid
    keyword_weight: 0.4
  AvisKB:
    search: semantic

fields:
  titre:       { type: text, title_for: AnnoncesKB }
  description: { type: text, content_for: AnnoncesKB }
  prix:        { type: number, filter: range, sort: true }
  marque:      { type: choice, values: auto, filter: multi-select }
  carburant:   { type: choice, values: [essence, diesel, électrique, hybride] }
  options:     { type: tags, filter: has-any, content_for: AnnoncesKB }
  photos:      { type: images, analyze: true, content_for: AnnoncesKB }
```

**Types de champs :** text, number, choice, boolean, date, tags, location, images, url, price

**Filtres :** range, gte, lte, exact, single-select, multi-select, has-any, has-all, has-none, toggle, search, radius

**Domaines testés en schema YAML :** Véhicules, Immobilier, Restaurant/Menu, Jobs/Emploi, Événements/Billetterie, E-commerce générique — chacun avec 2+ KBs, relations, filtres spécifiques.

**Mode zero-config :** CSV sans schema → auto-détection des types (< 20 valeurs uniques → choice, contient "|" → tags, long texte → text/fulltext, etc.) + proposition interactive.

### Ce que ça devient dans rag3weaver

Le schéma YAML pilote la configuration de `Catalog` :
- Les `fields` mappent vers les colonnes rag3db + filter fields Lucivy
- Les `knowledge_bases` mappent vers les KB rag3weaver avec `titleFor` / `contentFor`
- Les `filter` mappent vers `FilterCompiler` (pre-filter Lucivy pour number/choice/boolean, pre-resolution Cypher pour relations)
- Les `sources` avec `link: X → Y` mappent vers `catalog.link()`

**Priorité basse** — le code-first est plus pragmatique pour l'instant, mais le schéma YAML reste la vision cible pour l'adoption non-développeur.

---

## G. Architecture L0-L5 (vision originale → rag3weaver Rust)

### Mapping des couches

| Couche JS (kuzu-wasm-exp) | Équivalent Rust (rag3weaver) | État |
|---|---|---|
| L0 — Raw Kuzu Connection | `rag3db::Database` + `Connection` | ✅ natif |
| L1 — SchemaBuilder (fluent API) | Schema Cypher dans `schema.rs` | ✅ |
| L2 — DocumentStore (chunking, embeddings) | `chunker.rs` + `EmbedBatchOp` + `catalog.drain()` | ✅ |
| L3 — Catalog (multi-KB, hybrid search, queues) | `catalog.rs` + `search.rs` + `filter.rs` + `fusion.rs` | ✅ |
| L4 — Orchestrator (events, batching) | `event_bus.rs` + `drain()` pipeline | ✅ partiellement |
| L5 — Domain-Specific (Code RAG, Cars...) | Pas encore séparé — config inline | ❌ à faire |

**test-l5-full.mjs** montrait le pipeline complet :
1. `codeparsersToEntities(parseResult)` → { files, scopes, libraries }
2. `codeparsersRelationships(parseResult)` → [{ type, from, to }]
3. Ingest via `Catalog.create()` + `catalog.relate()` + `catalog.drain()`
4. Search via `catalog.search('ScopeKB', query)` + `catalog.searchWithExplore()`
5. Enrichissement via hooks (`enrichCodeResult` — fetch children, relevant chunks)

Ce pipeline est **exactement** ce que rag3weaver Rust fait déjà, mais côté WASM/JS. La version Rust native est le remplaçant.

---

## H. Récapitulatif : tous les use cases et leur priorité

### Priorité 1 — Déjà implémenté, à brancher

| Use case | Input | Entités | Relations | KBs | Effort |
|----------|-------|---------|-----------|-----|--------|
| **Code RAG (GitHub)** | Git clone → codeparsers | File, Scope, Library | DEFINED_IN, CONSUMES, INHERITS_FROM, USES_LIBRARY, PARENT_OF | ScopeKB (hybrid), FileKB (fts), LibraryKB (fts) | Moyen — wrapper `ingest_github()` |

### Priorité 2 — Pipeline à construire

| Use case | Input | Entités | Relations | KBs | Effort |
|----------|-------|---------|-----------|-----|--------|
| **Documents binaires** | PDF, DOCX, images | Document, Section | IN_DOCUMENT | DocumentKB (hybrid) | Moyen — parsing Rust + OCR |
| **Arborescence fichiers** | Filesystem / Google Drive | Directory, File | CONTAINS, HAS_FILE, MENTIONS_URL | FileContentKB (hybrid) | Petit — scan + extraction texte |
| **Shopify (Composio)** | GraphQL API | Product, Variant, Collection | HAS_VARIANT, IN_COLLECTION | ProductKB (hybrid) | Moyen — GraphQL + OAuth |

### Priorité 3 — Extensions futures

| Use case | Input | Entités | Relations | KBs | Effort |
|----------|-------|---------|-----------|-----|--------|
| **Gmail / Outlook** | API mail | Mail, Contact, Attachment | FROM, TO, CC, HAS_ATTACHMENT, IN_THREAD | MailKB (hybrid), ContactKB (fts) | Moyen |
| **Catalogue universel** | YAML + CSV | Configurable | Configurable | Configurable | Gros — schema parser |
| **GitHub Issues/PRs** | gh API | Repository, Issue, PR, Commit | CLOSES, MODIFIES, REFERENCES | IssueKB (hybrid), CommitKB (fts) | Moyen |

---

## I. Ce que ces usages réels vont stress-tester

| Aspect | Use cases qui le testent |
|--------|------------------------|
| **Variété de types** (Text, String, Int64, Double, Bool, Tags) | Shopify (prix, stock, tags, on_sale), Catalogue universel |
| **Relations riches** (hiérarchies, citations, structurelles) | Code RAG (class→method, scope→library), Mails (thread→mail→contact), Fichiers (dir→file) |
| **Chunking à l'échelle** (milliers de docs, > 100KB) | Documents binaires (PDF 50+ pages), Code RAG (gros repos) |
| **Filtres pré-filtre Lucivy** (range, string match, boolean) | Shopify (price range, vendor, in_stock), Mails (date range), Catalogue universel |
| **Multi-KB** (même entité, stratégies différentes) | Code RAG (ScopeKB hybrid + FileKB fts), Catalogue (ProduitsKB + AvisKB + FaqKB) |
| **Explore BFS** (navigation graphe post-search) | Code RAG (scope→consumes→scope→defined_in→file), Mails (mail→thread→mail→contact) |
| **Fusion 3-way** (BM25 + vector + sparse) | Code RAG (docstrings sémantiques + keywords exacts + sparse termes rares) |
| **Ingestion incrémentale** (update si contenu changé, hash) | GitHub re-ingestion, Google Drive sync |
| **OCR multi-provider** (free-first, fallback qualité) | Documents binaires (PDF image-only, photos) |
| **Filtres combinés** (Lucivy natif + Cypher allowed_ids) | Shopify (prix + collection via graphe), Code RAG (language + scope type + file path) |
