# 08 — Étude de kuzu-wasm-exp : Vision Universelle et Code RAG

## Contexte

`kuzu-wasm-exp` est le prototype exploratoire qui a précédé rag3weaver Rust. Tout était en JavaScript pur, au-dessus de Kuzu WASM. Le code est dispersé mais contient deux idées architecturales fortes :

1. **Un framework RAG universel** — pas juste pour le code, mais pour n'importe quel domaine
2. **Un pipeline code RAG complet** avec codeparsers, relations, explore, et enrichissement

Ce document synthétise ces deux axes.

### Racine du prototype

```
packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/
├── l5/code-rag/           # L5 : instanciation domaine code
│   ├── schema.js          # CODE_SCHEMA (entités, relations, KBs)
│   ├── index.js           # codeparsersToEntities + codeparsersRelationships + re-exports
│   ├── hooks.js           # enrichClassContent, enrichCodeResult, composeHooks
│   └── presets.js         # 5 presets de recherche
├── src/lib/catalog/       # Catalog system (L3+)
│   ├── index.ts           # Catalog principal
│   ├── types.ts           # Types TS
│   └── modules/
│       ├── CatalogSearch.ts   # searchWithExplore + formatExploreAsMarkdown
│       ├── CatalogCRUD.ts     # CRUD Cypher
│       └── CatalogSchema.ts   # Schema management
├── test-l5-full.mjs       # Test d'intégration complet (parse → ingest → search → explore)
├── docs/architecture/
│   └── universal-catalog-schema.md  # Schema YAML universel multi-domaines
└── RECONCILIATION-universal-to-code.md  # Pont universel → Code RAG
```

---

## 1. La vision "universelle"

### Le problème

L3 (Catalog) dans le prototype était hardcodé pour le code : `PARENT_OF`, `Scope`, `scopeType`, `signature`... partout dans le code. La vision universelle dit : **le même framework doit servir pour des voitures, de l'immobilier, des FAQ, du e-commerce, ET du code.** Le code n'est qu'une instanciation parmi d'autres.

### L'architecture en couches (L0-L5)

```
L5 — Domain-Specific (Code RAG, Cars RAG, Docs RAG...)
     Instantiation du schema + hooks + presets + formatters

L4 — Orchestrator
     Accumulator pattern, batching, events, parallel embedding
     create() → [prepare] → [embedding] → [store] → [linking]

L3 — Catalog (domain-agnostic, schema-driven)
     Multi-entités, relations, KB cross-entités, hybrid search

L2 — DocumentStore
     Auto-chunking, embeddings, HASHSAFE UUID, content hashing

L1 — SchemaBuilder (Fluent API)
     Type-safe schema, prepared statements, query builder, Cypher gen

L0 — Kuzu WASM (Raw)
     Direct Cypher queries
```

Le principe : **L0-L4 ne connaissent RIEN du domaine**. Tout ce qui est spécifique (entités, relations, champs, formatage, hooks de search) vit en L5.

### Le schema universel (`schema.yaml`)

> **Fichier** : `packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/docs/architecture/universal-catalog-schema.md`

Le document `universal-catalog-schema.md` définit un format déclaratif pour n'importe quel catalogue :

```yaml
catalog:
  name: "Mon Garage Auto"

knowledge_bases:
  CarKB:
    search: hybrid
    title_boost: 2.5
    keyword_weight: 0.4
  ReviewsKB:
    search: semantic
    content_boost: 2.0

sources:
  primary: items.csv
  reviews:
    file: reviews.csv
    link: car_id → primary.id

fields:
  name:        { type: text, title_for: CarKB }
  description: { type: text, content_for: CarKB }
  prix:        { type: number, filter: range, sort: true }
  marque:      { type: choice, values: auto, filter: multi-select }
  carburant:   { type: choice, values: [essence, diesel, électrique, hybride] }
  options:     { type: tags, filter: has-any, content_for: CarKB }
  photos:      { type: images, analyze: true, content_for: CarKB }
  review_text: { source: reviews, type: text, content_for: ReviewsKB }
```

### Les concepts clés du schema

**Knowledge Bases** — vues de recherche sur le graphe. Une KB agrège des champs de plusieurs entités via `titleFor`/`contentFor`. Chaque KB a sa propre stratégie de recherche (hybrid, semantic, fulltext).

**titleFor / contentFor** — un champ peut être titre dans une KB (poids élevé, identité) ou contenu (corps, chunkable). Un seul titre par KB, plusieurs contenus.

**boost par champ** — chaque champ a un poids indépendant : `signature: boost 2.5`, `docstring: boost 0.5`.

**Types de champs** — text, number, choice, boolean, date, tags, location, images, url, price. Chaque type a ses filtres possibles (range, multi-select, has-any, toggle...).

**HASHSAFE** — stratégie d'UUID déterministe pour les updates incrémentaux :
1. `hashsafe: [marque, modele, annee]` → UUID = hash("peugeot|308|2019") → même données = même UUID = skip si inchangé
2. Fallback sur `titleFor` si pas de hashsafe
3. Fallback sur UUID random (pas d'incrémental)

**Chunking** — configurable par KB :
```yaml
ScopeKB:
  chunking:
    enabled: true
    max_size: 1000     # chars
    overlap: 100
    fulltext_on_chunks: true   # BM25 aussi sur les chunks, pas juste embedding
```

**Sources liées** — relations 1-to-many entre sources de données (voiture → avis, produit → FAQ).

### Exemples multi-domaines documentés

Le schema a été pensé avec 6 domaines concrets :
- **Voitures** : AnnoncesKB + AvisKB, filtres prix/km/marque/carburant
- **Immobilier** : BiensKB + QuartiersKB, filtres surface/pièces/DPE, location radius
- **Restaurant** : MenuKB + RecettesKB, tags ingrédients/allergènes/régimes
- **Jobs** : OffresKB + EntrepriseKB, filtres contrat/salaire/remote/localisation
- **Événements** : EventsKB + LieuxKB, filtres date/ville/catégorie/places
- **E-commerce** : ProduitsKB + AvisKB + FaqKB, multi-source

### Mode Zero Config

Si le client fournit juste un CSV sans schema, auto-détection :
- < 20 valeurs uniques → `choice`
- Nombres avec grande variance → `number` + `range`
- Contient "|" → `tags`
- Très long texte → `text` + full-text search
- Format date reconnu → `date`

---

## 2. Le Code RAG : l'instanciation la plus avancée

### L5/code-rag : la couche domaine

> **Chemin** : `packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/l5/code-rag/`

Le dossier `l5/code-rag/` est l'instanciation du schema universel pour le code. 4 fichiers :

#### schema.js — CODE_SCHEMA

> **Fichier** : `l5/code-rag/schema.js`

```javascript
{
  name: 'CodeRAG',
  embeddingDim: 768,   // TEI default

  entities: {
    File: {
      fields: {
        path:         { type: 'string', titleFor: 'FileKB' },
        absolutePath: { type: 'string' },
        language:     { type: 'string' },
        linesOfCode:  { type: 'int64' },
      }
    },
    Scope: {
      fields: {
        name:         { type: 'string', titleFor: 'ScopeKB' },
        scopeType:    { type: 'string' },
        signature:    { type: 'string', contentFor: 'ScopeKB' },
        content:      { type: 'string', contentFor: 'ScopeKB', chunked: true },
        docstring:    { type: 'string', contentFor: 'ScopeKB' },
        startLine:    { type: 'int64' },
        endLine:      { type: 'int64' },
        absolutePath: { type: 'string' },
        parentName:   { type: 'string' },
      }
    },
    Library: {
      fields: {
        name:       { type: 'string', titleFor: 'LibraryKB' },
        importPath: { type: 'string' },
      }
    }
  },

  relations: {
    DEFINED_IN:    { from: 'Scope', to: 'File' },
    CONSUMES:      { from: 'Scope', to: 'Scope' },
    CONSUMED_BY:   { from: 'Scope', to: 'Scope' },
    INHERITS_FROM: { from: 'Scope', to: 'Scope' },
    IMPLEMENTS:    { from: 'Scope', to: 'Scope' },
    PARENT_OF:     { from: 'Scope', to: 'Scope' },
    HAS_PARENT:    { from: 'Scope', to: 'Scope' },
    DECORATES:     { from: 'Scope', to: 'Scope' },
    USES_LIBRARY:  { from: 'Scope', to: 'Library' },
    IMPORTS:       { from: 'File', to: 'File' },
  },

  knowledgeBases: {
    FileKB:    { search: 'hybrid' },
    ScopeKB:   { search: 'hybrid', chunking: { max_size: 1000, overlap: 100 } },
    LibraryKB: { search: 'hybrid' },
  }
}
```

**Observation** : `name` est titleFor ScopeKB mais `signature` est contentFor. Dans le `RECONCILIATION-universal-to-code.md`, l'idée originale était signature comme titre (boost 2.5) et content comme contenu (chunké). Le schema.js final a simplifié — name est titre, signature+content+docstring sont contenu.

**Note (22 fév 2026)** : Le refactor codeparsers Rust ajoute `scope_start_line`/`scope_end_line`, `signature_start_line`/`signature_end_line`, `body_start_line`/`body_end_line`. Le schema.js n'a que `startLine`/`endLine` — à mettre à jour lors de l'intégration.

#### index.js — codeparsersToEntities / codeparsersRelationships

> **Fichier** : `l5/code-rag/index.js` (237 lignes)

Les deux fonctions de conversion du format codeparsers vers le format rag3weaver :

**`codeparsersToEntities(parseResult)`** (lignes 48-199) :
1. Construit un lookup `fileAnalysisMap` (path → analyse) avec support des chemins relatifs et absolus (double mapping: absolu ET relatif via `relationships.files`)
2. Convertit les fichiers depuis `relationships.files` (UUID fourni par codeparsers)
3. Convertit les scopes depuis `relationships.uuidMapping` — utilise `contentDedented` en priorité sur `content` (ligne 115 : `fullScope?.contentDedented || fullScope?.content || ''`)
4. **Deuxième passe — enrichissement containers** (lignes 124-184) : constante `EXPLICIT_CONTAINER_TYPES = ['class', 'interface', 'enum', 'namespace', 'module', 'struct', 'trait']` (7 types). Pour chaque container, cherche les enfants par `parentName` + même `absolutePath`, gère aussi les parents qualifiés (`deepNested.level1.level2`). Construit une section "Members:" avec signature + line range + preview body (120 chars depuis le premier `{`).
5. Convertit les libraries externes depuis `relationships.externalLibraries`

**Détail de l'enrichissement containers** (le plus pertinent pour codeparsers) :

```javascript
// index.js lignes 148-183 — Résumé du pattern réel
const memberLines = [];
for (const child of children) {
    if (child.scopeType === 'block') continue;  // Skip les gap fillers
    const sig = child.signature || child.name;
    const lineRange = child.startLine === child.endLine
        ? `L${child.startLine}` : `L${child.startLine}-${child.endLine}`;
    // Body preview : extrait 120 chars depuis le premier '{'
    let bodyPreview = '';
    if (child.content) {
        const sigEnd = child.content.indexOf('{');
        if (sigEnd !== -1) {
            bodyPreview = child.content.substring(sigEnd).replace(/\s+/g, ' ').substring(0, 120);
            if (bodyPreview.length >= 120) bodyPreview += '...';
        }
    }
    memberLines.push(`  - ${sig} (${lineRange})`);
    if (bodyPreview) memberLines.push(`    ${bodyPreview}`);
}
// Résultat final du content enrichi :
scope.content = `${containerSig}\n\nMembers:\n${memberLines.join('\n')}`;
```

**Note (22 fév 2026)** : Maintenant que `scope.content` dans codeparsers Rust est body-only (plus de signature), le `child.content.indexOf('{')` dans le body preview ne trouvera plus le `{` d'ouverture en début de contenu (il EST le contenu). Le pattern de preview devra s'adapter.

**`codeparsersRelationships(parseResult)`** (lignes 222-236) :
- Filtre les relationships par `SUPPORTED_RELATIONSHIP_TYPES` (9 types : DEFINED_IN, CONSUMES, CONSUMED_BY, INHERITS_FROM, IMPLEMENTS, PARENT_OF, HAS_PARENT, DECORATES, USES_LIBRARY)
- Mappe vers `{ type, from: rel.fromUuid, to: rel.toUuid }`

#### hooks.js — enrichClassContent / enrichCodeResult

> **Fichier** : `l5/code-rag/hooks.js` (101 lignes)

**`enrichClassContent(scopes)`** (lignes 15-42) — enrichissement pre-ingestion.
- Constante `CONTAINER_TYPES = ['class', 'interface', 'enum', 'namespace', 'module']` (5 types — **sans** struct/trait, contrairement à `EXPLICIT_CONTAINER_TYPES` dans index.js qui en a 7).
- Pour chaque container, trouve les enfants par `parent === scope.name` + même file + lignes incluses.
- Ajoute `scope.contentWithMembers` = content original + section "Members:" avec signatures + line range.
- **Différence avec index.js** : hooks.js n'ajoute PAS de body preview, et stocke dans `contentWithMembers` (pas en remplacement de `content`). L'index.js remplace directement `scope.content`.

**`enrichCodeResult(result, context)`** (lignes 51-89) — hook de search post-résultat :
1. Fetch les détails du node (`catalog._fetchNodeDetails('Scope', result._uuid)`)
2. Si c'est un container type → `catalog.searchRelated(uuid, 'PARENT_OF', query, {limit: 5})` pour trouver les enfants pertinents → stocke dans `result.relevantChildren`
3. Pour tous les scopes → `catalog.getRelevantChunks(uuid, query, {limit: 3})` → stocke dans `result.relevantChunks`

**`composeHooks(...hooks)`** (lignes 94-101) — compose N hooks séquentiellement.

#### presets.js — 5 presets de recherche

> **Fichier** : `l5/code-rag/presets.js` (62 lignes)

```javascript
CODE_SEARCH_PRESET           // limit: 10, boost containers (class/interface/enum) × 0.8
IMPLEMENTATION_SEARCH_PRESET // limit: 10, boost functions/methods × 0.85
TYPE_SEARCH_PRESET          // limit: 10, boost types (class/interface/type/enum) × 0.7
BROAD_SEARCH_PRESET         // limit: 20, pas de boost (pure semantic/distance ranking)
CHUNK_SEARCH_PRESET         // limit: 15, returnChunks: true, includeParent: true
```

Mécanisme : `boostIf` avec expressions de filtre (`scopeType IN [...]`). Les scores sont des distances (plus petit = meilleur), donc multiplier par < 1 = booster.

---

## 3. Le test le plus complet : test-l5-full.mjs

> **Fichier** : `packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/test-l5-full.mjs` (519 lignes)

C'est le test d'intégration le plus évolué du prototype. Pipeline complet en 4 étapes :

### Étape 1 : Parse avec codeparsers

```javascript
const parser = new ProjectParser({ maxWorkers: 4, verbose: false });
const result = await parser.parseProject({
  root: projectRoot,
  files: filesToParse,
  resolveRelationships: true,  // ← active le RelationshipResolver intégré
});
```

Résultat : fichiers parsés + relationships résolues (CONSUMES, INHERITS_FROM, PARENT_OF, USES_LIBRARY...) + externalLibraries + uuidMapping.

### Étape 2 : Convert avec L5

```javascript
const entities = codeparsersToEntities(parseResult);
// → { files: [...], scopes: [...], libraries: [...] }
// Les containers sont déjà enrichis avec "Members:"

const relationships = codeparsersRelationships(parseResult);
// → [{ type: 'CONSUMES', from: uuid1, to: uuid2 }, ...]
```

### Étape 3 : Ingest dans Kuzu

```javascript
// Chargement via eval de bundles JS (pattern spécifique au prototype)
const initKuzu = (await import('./dist/kuzu-wasm.js')).default;
const kuzu = await initKuzu();
globalThis.Rag3Weaver = {};
new Function('global', fs.readFileSync('dist/rag3weaver-l1.js', 'utf-8'))(globalThis);
new Function('global', fs.readFileSync('dist/rag3weaver-l2.js', 'utf-8'))(globalThis);
new Function('global', fs.readFileSync('dist/rag3weaver-l3.js', 'utf-8'))(globalThis);
const { Catalog } = globalThis.Rag3Weaver;

const db = new kuzu.WebDatabase(':memory:', 0, 4, true, false, 256 * 1024 * 1024);
const conn = new kuzu.WebConnection(db, 4);
const catalog = await Catalog.create(conn, embedder, CODE_SCHEMA);

// Events pour le suivi
catalog.on('entities:batch_inserted', (e) => ...);
catalog.on('relations:batch_created', (e) => ...);
catalog.on('chunks:batch_inserted', (e) => ...);

// File entities
for (const file of entities.files) {
  const ref = catalog.create('File', {
    _uuid: file._uuid,          // UUID de codeparsers
    path, absolutePath, language, linesOfCode
  });
  fileRefs.set(file._uuid, ref);
}

// Scope entities
for (const scope of entities.scopes) {
  const ref = catalog.create('Scope', {
    _uuid: scope._uuid,
    name, scopeType, signature, content, docstring,
    startLine, endLine, absolutePath, parentName
  });
  scopeRefs.set(scope._uuid, ref);
}

// Library entities
for (const lib of entities.libraries) {
  const ref = catalog.create('Library', {
    _uuid: lib._uuid, name, importPath
  });
  libRefs.set(lib._uuid, ref);
}

// Relationships — lookup dans les 3 maps de refs
for (const rel of relationships) {
  const fromRef = fileRefs.get(rel.from) || scopeRefs.get(rel.from) || libRefs.get(rel.from);
  const toRef = fileRefs.get(rel.to) || scopeRefs.get(rel.to) || libRefs.get(rel.to);
  if (fromRef && toRef) catalog.relate(rel.type, fromRef, toRef);
}

await catalog.drain();
```

### Étape 4 : Search + Explore

```javascript
// Search hybride avec hook d'enrichissement
const results = await catalog.search('ScopeKB', 'scope extraction parser', {
  limit: 5,
  onResultEnrich: enrichCodeResult,   // ← hook L5
});

// Search dans LibraryKB
const libResults = await catalog.search('LibraryKB', 'tree-sitter', { limit: 5 });

// Search avec graph exploration
const exploreResult = await catalog.searchWithExplore('ScopeKB', searchQuery, {
  limit: 5,
  exploreDepth: 2,
  exploreTopK: 15,
  consumesRelations: ['CONSUMES', 'USES_LIBRARY'],
  consumedByRelations: ['CONSUMED_BY', 'PARENT_OF'],
});

// Formatage en markdown avec arbre ASCII
const markdown = await catalog.formatExploreAsMarkdown(exploreResult);
```

### L'embedder : TEI réel

Pas de mock — le test utilise un serveur TEI local :
```javascript
const TEI_URL = 'http://localhost:8081/embed';
const TEI_BATCH_SIZE = 32;
const MAX_TEXT_LENGTH = 1500;

async function embedder(texts) {
  // Truncate long texts → batch par 32 → POST /embed → embeddings 768-dim
}
```

---

## 4. Ce qui manquait au prototype (et qui existe dans rag3weaver Rust)

### Ce que rag3weaver Rust a réimplémenté nativement

| Feature | kuzu-wasm-exp (JS) | rag3weaver (Rust) |
|---|---|---|
| Graph DB | Kuzu WASM (JS bindings) | rag3db (fork Kuzu, Rust C API) |
| Fulltext search | Kuzu FTS (limité) | Tantivy (complet, fuzzy, phrase, regex) |
| Pipeline | JS queues (L4 Orchestrator) | Rust Catalog + rayon pool + drain async |
| Embedding | TEI externe | MockEmbedder (ONNX WASM à venir) |
| Chunking | JS (L2 DocumentStore) | Rust (dans Catalog) |
| Schema | JS objects (CODE_SCHEMA) | JSON config → WeaverContext |
| Hybrid search | JS (boost/rrf/weighted) | Rust (vector + BM25 fusion) |
| Relations | Cypher via JS | Cypher via Rust C API |

### Ce qui n'est PAS encore dans rag3weaver Rust

| Feature | Status |
|---|---|
| **searchWithExplore** | Pas implémenté — c'est la prochaine étape |
| **Hooks (onResultEnrich)** | Pas implémenté |
| **Presets (boostIf expressions)** | Pas implémenté |
| **formatExploreAsMarkdown** | Pas implémenté |
| **Multi-KB search routing** | Partiellement (une KB à la fois) |
| **Container enrichment (Members:)** | Côté JS seulement (codeparsersToEntities) |
| **Filter engine (range/choice/tags)** | Pas implémenté (Tantivy filter_fields partiel) |
| **Grep/Read APIs** | Pas implémenté |
| **Schema universel YAML** | Pas implémenté |

---

## 5. Le pont entre prototype et rag3weaver Rust

### Ce qu'on garde tel quel

- **CODE_SCHEMA** : la structure entités/relations/KB est directement transposable
- **codeparsersToEntities/codeparsersRelationships** : ces fonctions JS restent côté client, elles préparent les données avant ingestion via l'API WASM
- **enrichClassContent** : pré-traitement JS avant ingestion, pas besoin de le porter en Rust
- **Les presets** : restent côté JS comme arguments de search

### Ce qu'il faut porter en Rust (côté rag3weaver)

- **searchWithExplore** : graph traversal après search — c'est du Cypher (`MATCH (n)-[r]->(m) WHERE n._uuid IN $uuids`)
- **Chunk resolution vers parent** : quand un chunk matche, remonter au parent et construire matchedRange

### Ce qui reste en JS/TS (côté client)

- **Hooks** : `onResultEnrich` est du user code, il fait des appels supplémentaires au catalog
- **Formatters** : `formatExploreAsMarkdown` est de la présentation
- **Presets** : configuration de search, pas de la logique

### L'API cible pour le WASM

```javascript
// Déjà fait
const weaver = new Module.Weaver(configJson, dbPath);
const handle = weaver.create("Scope", fieldsJson);
weaver.link(h1, h2, "CONSUMES", "{}");
const drainHandle = weaver.drainAsyncStart();
const searchHandle = weaver.searchAsyncStart(kb, query, optionsJson);

// À ajouter
const exploreHandle = weaver.exploreAsyncStart(kb, query, exploreOptionsJson);
// exploreOptions = { limit, exploreDepth, exploreTopK, consumesRelations, consumedByRelations }
// Retourne: { results: [...], graph: { nodes: [...], edges: [...] } }
```

---

## 6. Récapitulatif : ce qui compte pour la suite

### Priorité 1 : searchWithExplore en WASM

> **Implémentation JS** : `src/lib/catalog/modules/CatalogSearch.ts` — `searchWithExplore()` (lignes 187-358) et `formatExploreAsMarkdown()` (lignes 513-605)

C'est la feature qui justifie le graph DB. Le prototype JS l'avait. Le Rust ne l'a pas encore. Le flow :
1. Search hybride → top N résultats avec UUIDs
2. Pour chaque résultat, traverser le graphe (Cypher multi-hop) selon les relations configurées
3. Collecter nodes + edges → retourner comme JSON
4. Côté JS : formatter, enrichir, afficher

### Priorité 2 : Pipeline codeparsers → WASM complet

Le flow end-to-end :
```
codeparsers.parseProject()
  → codeparsersToEntities() + codeparsersRelationships()   [JS, L5]
  → weaver.create() × N + weaver.link() × M               [WASM FFI]
  → weaver.drainAsyncStart() + poll                         [WASM async]
  → weaver.searchAsyncStart() + poll                        [WASM async]
  → weaver.exploreAsyncStart() + poll                       [WASM async, à faire]
  → formatExploreAsMarkdown()                               [JS, L5]
```

### Priorité 3 : Valider la généricité

Le prototype avait documenté 6 domaines. Pour prouver que rag3weaver Rust est universel, il faudra tester au moins un deuxième domaine (documents simples, FAQ, ou voitures).

---

## 7. Impact du refactor codeparsers Rust (22 fév 2026)

### Ce qui a changé dans codeparsers

Le refactor scope lines + body extraction a modifié ScopeInfo :

```
Avant                          Après
------                         ------
start_line                     scope_start_line
end_line                       scope_end_line
                               signature_start_line  (NEW)
                               signature_end_line    (NEW)
                               body_start_line       (NEW, Option<usize>)
                               body_end_line         (NEW, Option<usize>)
content = full node text       content = body-only text (sans signature)
```

### Impact sur le pipeline L5

**1. `codeparsersToEntities` (index.js)**

Le code actuel lit `fullScope?.contentDedented || fullScope?.content`. Avec le nouveau content body-only, il n'y aura plus de signature en doublon dans content. C'est un gain net.

Mais l'enrichissement containers (lignes 158-170) utilise `child.content.indexOf('{')` pour trouver le début du body preview — or le content est maintenant body-only, il commence souvent par `{`. Le body preview sera donc le content entier tronqué à 120 chars, ce qui est en fait le comportement souhaité (pas besoin de skipper la signature).

**2. `CODE_SCHEMA` (schema.js)**

Le schema n'a que `startLine`/`endLine`. Pour exploiter les nouvelles lignes :

```javascript
// Schema.js mis à jour (proposition)
Scope: {
  fields: {
    // ... existants ...
    scopeStartLine:     { type: 'int64' },
    scopeEndLine:       { type: 'int64' },
    signatureStartLine: { type: 'int64' },
    signatureEndLine:   { type: 'int64' },
    bodyStartLine:      { type: 'int64' },  // -1 si absent
    bodyEndLine:        { type: 'int64' },  // -1 si absent
  }
}
```

**3. Enrichissement classes/structs/interfaces**

La section "Members:" utilise `child.startLine`-`child.endLine` pour le line range. Avec le nouveau schema, on pourrait être plus précis :
- Afficher `L${signatureStartLine}` pour la signature
- Afficher `L${bodyStartLine}-${bodyEndLine}` pour le body range
- Calculer le nombre de lignes de body : `bodyEndLine - bodyStartLine + 1`

**4. `enrichCodeResult` (hooks.js)**

Le `CONTAINER_TYPES` dans hooks.js n'inclut PAS `struct` ni `trait`, alors que `codeparsersToEntities` les inclut. À harmoniser.

### Documents de référence liés

- `RECONCILIATION-universal-to-code.md` — Pont schema universel → Code RAG, détaille les champs Scope (startLine, endLine, etc.), les chunks comme entités, le score multi-match
- `docs/architecture/universal-catalog-schema.md` — Schema YAML universel avec 6 exemples domaines
