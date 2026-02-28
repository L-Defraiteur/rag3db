# 01 — Plan L5 : Code RAG domain-specific dans rag3weaver Rust

## Contexte

rag3weaver Rust implemente L0-L4 (Database, SchemaBuilder, DocumentStore/Chunking, Catalog, Orchestrator). Le niveau L5 — la couche domain-specific qui transforme la sortie codeparsers en entites/relations du graphe — existe uniquement en JavaScript dans `kuzu-wasm-exp/l5/code-rag/`. L'objectif est d'implementer L5 en Rust natif, directement dans la crate rag3weaver, pour avoir un pipeline 100% Rust : `codeparsers → L5 → catalog.create()/link() → drain()`.

## Ce qui existe deja

### L5 JS (kuzu-wasm-exp) — 4 fichiers

| Fichier | Contenu | Lignes |
|---|---|---|
| `schema.js` | CODE_SCHEMA : 3 entities (File, Scope, Library), 10 relations, 3 KBs | ~80 |
| `index.js` | `codeparsersToEntities()` + `codeparsersRelationships()` + enrichissement containers | ~240 |
| `hooks.js` | `enrichClassContent()` (pre-ingest), `enrichCodeResult()` (post-search), `composeHooks()` | ~100 |
| `presets.js` | 5 presets (CODE, IMPLEMENTATION, TYPE, BROAD, CHUNK) avec `boostIf` | ~60 |

### codeparsers Rust — deja pret

La crate `codeparsers` produit exactement les memes structures que le TS :
- `ProjectAnalysis { files, relationships, stats, errors }`
- `ScopeInfo { name, type, signature, content, content_dedented, parameters, heritage_clauses, decorator_details, identifier_references, parent, children, ... }`
- `ResolvedRelationship { type, from_uuid, to_uuid, from_name, to_name, metadata }`
- `FileInfo { uuid, path, absolute_path }`
- `ExternalLibraryInfo { uuid, name, symbols }`

Le pont est direct : pas de conversion de format necessaire.

### rag3weaver Rust L0-L4 — deja pret

- `CatalogConfig` avec entities/relations/knowledge_bases
- `catalog.create(entity_name, data)` → InsertOp + EmbedOp
- `catalog.link(rel_name, from, to, props)` → LinkOp
- `catalog.drain()` → flush tout
- `catalog.search(kb, query, options)` → hybrid search
- `catalog.search_with_explore(kb, query, explore_options)` → search + BFS graph

## Plan d'implementation

### Phase 1 : CODE_SCHEMA en Rust

Nouveau fichier `src/l5/code_schema.rs` :

```rust
pub fn code_schema() -> CatalogConfig {
    CatalogConfig {
        name: Some("code-rag".into()),
        entities: hashmap! {
            "File" => EntityDef {
                fields: hashmap! {
                    "path" => FieldDef { field_type: String, title_for: Some("FileKB"), .. },
                    "absolutePath" => FieldDef { field_type: String, .. },
                    "language" => FieldDef { field_type: Choice, .. },
                    "linesOfCode" => FieldDef { field_type: Int64, .. },
                },
                hashsafe: Some(vec!["path"]),
            },
            "Scope" => EntityDef {
                fields: hashmap! {
                    "name" => FieldDef { field_type: String, title_for: Some("ScopeKB"), .. },
                    "scopeType" => FieldDef { field_type: Choice, .. },
                    "signature" => FieldDef { field_type: Text, content_for: Some(vec!["ScopeKB"]), boost: Some(2.5), .. },
                    "content" => FieldDef { field_type: Text, content_for: Some(vec!["ScopeKB"]), chunked: true, .. },
                    "docstring" => FieldDef { field_type: Text, content_for: Some(vec!["ScopeKB"]), boost: Some(0.5), .. },
                    "startLine" => FieldDef { field_type: Int64, .. },
                    "endLine" => FieldDef { field_type: Int64, .. },
                    "absolutePath" => FieldDef { field_type: String, .. },
                    "parentName" => FieldDef { field_type: String, .. },
                },
                hashsafe: Some(vec!["absolutePath", "name", "scopeType", "startLine"]),
            },
            "Library" => EntityDef {
                fields: hashmap! {
                    "name" => FieldDef { field_type: String, title_for: Some("LibraryKB"), .. },
                    "importPath" => FieldDef { field_type: String, .. },
                },
                hashsafe: Some(vec!["name"]),
            },
        },
        relations: hashmap! {
            "DEFINED_IN"    => RelationDef { from: "Scope", to: "File" },
            "CONSUMES"      => RelationDef { from: "Scope", to: "Scope" },
            "CONSUMED_BY"   => RelationDef { from: "Scope", to: "Scope" },
            "INHERITS_FROM" => RelationDef { from: "Scope", to: "Scope" },
            "IMPLEMENTS"    => RelationDef { from: "Scope", to: "Scope" },
            "PARENT_OF"     => RelationDef { from: "Scope", to: "Scope" },
            "HAS_PARENT"    => RelationDef { from: "Scope", to: "Scope" },
            "DECORATES"     => RelationDef { from: "Scope", to: "Scope" },
            "USES_LIBRARY"  => RelationDef { from: "Scope", to: "Library" },
            "IMPORTS"       => RelationDef { from: "File", to: "File" },
        },
        knowledge_bases: hashmap! {
            "FileKB"    => KBConfig { search: Fulltext, chunking: ChunkingConfig { enabled: false, .. }, .. },
            "ScopeKB"   => KBConfig { search: Hybrid, keyword_weight: 0.3, chunking: ChunkingConfig { max_size: 1000, overlap: 100, .. }, .. },
            "LibraryKB" => KBConfig { search: Hybrid, .. },
        },
        embedding_dim: 384,
        ..Default::default()
    }
}
```

**Effort** : ~80 lignes, 0 dependance nouvelle.

### Phase 2 : codeparsers_to_entities()

Nouveau fichier `src/l5/code_ingest.rs` :

```rust
pub struct CodeEntities {
    pub files: Vec<(String, HashMap<String, CypherValue>)>,      // (entity_ref_key, data)
    pub scopes: Vec<(String, HashMap<String, CypherValue>)>,
    pub libraries: Vec<(String, HashMap<String, CypherValue>)>,
}

pub fn codeparsers_to_entities(analysis: &ProjectAnalysis) -> CodeEntities {
    // 1. Convertir FileInfo → File entities
    //    { path, absolutePath, language, linesOfCode }

    // 2. Convertir scopes (flatten depuis relationships.uuid_mapping)
    //    { name, scopeType, signature, content, docstring, startLine, endLine, absolutePath, parentName }

    // 3. Enrichir containers avec "Members:" section
    //    Pour class/interface/enum/namespace/module/struct/trait :
    //    - Trouver enfants par parent == scope.name
    //    - Builder section "Members:\n  - signature (L{start}-{end})\n  ..."
    //    - Ajouter au content

    // 4. Convertir ExternalLibraryInfo → Library entities
    //    { name, importPath }
}
```

**Enrichissement containers** — le point crucial :
```rust
fn enrich_container_content(scope: &ScopeInfo, children: &[&ScopeInfo]) -> String {
    let mut content = scope.content_dedented.clone();
    if children.is_empty() { return content; }

    content.push_str("\n\nMembers:\n");
    let mut sorted_children: Vec<_> = children.iter().collect();
    sorted_children.sort_by_key(|c| c.scope_start_line);

    for child in sorted_children {
        if child.r#type == ScopeInfoType::Block { continue; }
        let sig = if child.signature.is_empty() { &child.name } else { &child.signature };
        let truncated = if sig.len() > 120 { &sig[..120] } else { sig };
        content.push_str(&format!("  - {} (L{}-L{})\n",
            truncated, child.scope_start_line, child.scope_end_line));
    }
    content
}
```

**Effort** : ~200 lignes.

### Phase 3 : codeparsers_relationships()

Dans le meme fichier :

```rust
const SUPPORTED_RELATIONSHIP_TYPES: &[RelationshipType] = &[
    RelationshipType::DefinedIn,
    RelationshipType::Consumes,
    RelationshipType::ConsumedBy,
    RelationshipType::InheritsFrom,
    RelationshipType::Implements,
    RelationshipType::ParentOf,
    RelationshipType::HasParent,
    RelationshipType::Decorates,
    RelationshipType::UsesLibrary,
];

pub struct CodeRelationship {
    pub rel_type: String,
    pub from_uuid: String,
    pub to_uuid: String,
}

pub fn codeparsers_relationships(analysis: &ProjectAnalysis) -> Vec<CodeRelationship> {
    analysis.relationships.as_ref()
        .map(|r| r.relationships.iter()
            .filter(|rel| SUPPORTED_RELATIONSHIP_TYPES.contains(&rel.r#type))
            .map(|rel| CodeRelationship {
                rel_type: rel.r#type.to_string(),  // "CONSUMES", "INHERITS_FROM", etc.
                from_uuid: rel.from_uuid.clone(),
                to_uuid: rel.to_uuid.clone(),
            })
            .collect()
        )
        .unwrap_or_default()
}
```

**Effort** : ~40 lignes.

### Phase 4 : ingest_code_project() — orchestrateur L5

Fonction haut-niveau qui fait le lien complet :

```rust
pub async fn ingest_code_project(
    catalog: &mut Catalog,
    analysis: &ProjectAnalysis,
) -> Result<IngestStats, CatalogError> {
    let entities = codeparsers_to_entities(analysis);
    let relationships = codeparsers_relationships(analysis);

    // 1. Create entities
    let mut file_handles: HashMap<String, EntityRef> = HashMap::new();
    for (key, data) in &entities.files {
        let handle = catalog.create("File", data.clone()).await?;
        file_handles.insert(key.clone(), handle);
    }
    // ... idem pour scopes et libraries

    // 2. Create relationships
    for rel in &relationships {
        catalog.link(&rel.rel_type,
            RefOrUuid::Uuid(rel.from_uuid.clone()),
            RefOrUuid::Uuid(rel.to_uuid.clone()),
            HashMap::new(),
        ).await?;
    }

    // 3. Drain
    catalog.drain().await?;

    Ok(IngestStats { files: entities.files.len(), scopes: entities.scopes.len(), ... })
}
```

**Effort** : ~80 lignes.

### Phase 5 : Hooks system

Nouveau fichier `src/l5/hooks.rs` :

```rust
/// Hook trait — called during search result processing
#[async_trait]
pub trait SearchHook: Send + Sync {
    async fn enrich(&self, result: &mut SearchResult, context: &SearchContext) -> Result<(), CatalogError>;
}

pub struct SearchContext<'a> {
    pub query: &'a str,
    pub catalog: &'a Catalog,
    pub kb: &'a str,
}

/// Pre-built hook: enrich container results with relevant children
pub struct EnrichCodeResultHook;

#[async_trait]
impl SearchHook for EnrichCodeResultHook {
    async fn enrich(&self, result: &mut SearchResult, ctx: &SearchContext) -> Result<(), CatalogError> {
        // 1. Fetch node details si data est None
        // 2. Pour container types : search related via PARENT_OF
        // 3. Pour tous : get relevant chunks
        // Stocker dans result.data["_relevantChildren"] et result.data["_relevantChunks"]
    }
}

/// Compose multiple hooks
pub fn compose_hooks(hooks: Vec<Box<dyn SearchHook>>) -> Box<dyn SearchHook> { ... }
```

**Changement necessaire dans search.rs** : ajouter `hooks: Option<Vec<Box<dyn SearchHook>>>` a `SearchOptions` ou a `Catalog`, et les appeler apres fusion mais avant return.

**Effort** : ~120 lignes.

### Phase 6 : Search presets + boostIf

Nouveau fichier `src/l5/presets.rs` :

```rust
pub struct SearchPreset {
    pub name: &'static str,
    pub options: SearchOptions,
    pub boost_rules: Vec<BoostRule>,
}

pub struct BoostRule {
    pub field: String,       // "scopeType"
    pub values: Vec<String>, // ["class", "interface", "enum"]
    pub multiplier: f64,     // 1.1 = 10% boost
}

pub fn code_search_preset() -> SearchPreset { ... }
pub fn implementation_search_preset() -> SearchPreset { ... }
pub fn type_search_preset() -> SearchPreset { ... }
pub fn broad_search_preset() -> SearchPreset { ... }
pub fn chunk_search_preset() -> SearchPreset { ... }
```

**boostIf en Cypher** : dans le JS, c'est une string evaluee. En Rust, on peut faire mieux avec un match sur les `data` du SearchResult :

```rust
fn apply_boost_rules(results: &mut [SearchResult], rules: &[BoostRule]) {
    for result in results.iter_mut() {
        if let Some(data) = &result.data {
            for rule in rules {
                if let Some(CypherValue::String(val)) = data.get(&rule.field) {
                    if rule.values.contains(val) {
                        result.score *= rule.multiplier;
                    }
                }
            }
        }
    }
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
}
```

**Effort** : ~100 lignes.

### Phase 7 : format_explore_as_markdown()

Nouveau fichier `src/l5/format.rs` :

```rust
pub struct FormatOptions {
    pub include_snippets: bool,
    pub max_snippet_lines: usize,
    pub group_by_relation: bool,
    pub show_depth: bool,
}

pub fn format_explore_as_markdown(result: &ExploreResult, options: &FormatOptions) -> String {
    // ## Search Results for "{query}"
    //
    // ### 1. {label} — score: {score}
    // ```{lang}
    // {snippet tronque a max_snippet_lines}
    // ```
    //
    // #### Outgoing:
    // +-- [CONSUMES] target_label
    // +-- [USES_LIBRARY] lib_name
    //
    // #### Incoming:
    // +-- [CONSUMED_BY] caller_label
    // ---
}
```

**Effort** : ~100 lignes.

## Structure des fichiers

```
src/
  l5/
    mod.rs              // pub mod code_schema, code_ingest, hooks, presets, format;
    code_schema.rs      // CODE_SCHEMA comme CatalogConfig (~80 lignes)
    code_ingest.rs      // codeparsers_to_entities + codeparsers_relationships + ingest_code_project (~320 lignes)
    hooks.rs            // SearchHook trait + EnrichCodeResultHook + compose_hooks (~120 lignes)
    presets.rs          // 5 presets + BoostRule + apply_boost_rules (~100 lignes)
    format.rs           // format_explore_as_markdown (~100 lignes)
  lib.rs                // + pub mod l5;
```

**Total estime** : ~720 lignes de Rust.

## Dependance cle : codeparsers

La crate `codeparsers` est un workspace member dans le meme Cargo.toml. L5 depend de ses types (`ProjectAnalysis`, `ScopeInfo`, `ResolvedRelationship`, etc.). Il faudra ajouter :

```toml
[dependencies]
codeparsers = { path = "codeparsers" }
```

**Attention** : codeparsers depend de tree-sitter (natif C). En WASM, ca ne compile pas directement — il faudra feature-gater L5 :

```toml
[features]
l5-code = ["codeparsers"]
```

## Bilan

| Phase | Livrable | Effort | Depend de |
|---|---|---|---|
| 1 | `code_schema()` | ~80 lignes | - |
| 2 | `codeparsers_to_entities()` + enrichissement | ~200 lignes | Phase 1, codeparsers |
| 3 | `codeparsers_relationships()` | ~40 lignes | codeparsers |
| 4 | `ingest_code_project()` | ~80 lignes | Phase 2+3 |
| 5 | Hooks system | ~120 lignes | search.rs modifie |
| 6 | Presets + boostIf | ~100 lignes | - |
| 7 | `format_explore_as_markdown()` | ~100 lignes | search types |

Phases 1-4 sont le chemin critique vers un pipeline E2E fonctionnel.
Phases 5-7 sont des enrichissements qui ameliorent la qualite des resultats.

## Tests prevus

| Test | Verifie |
|---|---|
| `code_schema_is_valid` | `validate_schema(code_schema())` passe |
| `entities_from_analysis` | scope count, file count, library count corrects |
| `container_enrichment` | classes ont "Members:" dans content |
| `relationships_filtered` | seuls les types supportes passent |
| `ingest_e2e` | full pipeline avec MockConnection (create + link + drain) |
| `boost_rules_applied` | classes boostees au-dessus des fonctions a score egal |
| `explore_markdown_format` | output contient headers, snippets, tree |
