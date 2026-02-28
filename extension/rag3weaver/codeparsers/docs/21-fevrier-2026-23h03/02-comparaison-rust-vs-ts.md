# 02 - Comparaison Rust vs TypeScript codeparsers

## Contexte

Comparaison du parsing du projet `ragforge-core/packages/codeparsers/src` (79 fichiers .ts) par :
- **TS** : le codeparsers original TypeScript (ProjectParser via workers)
- **Rust** : la traduction Rust (codeparsers crate)

## Corrections effectuees avant la comparaison

### 1. Tree-sitter grammars manquants
Tous les `set_language()` etaient commentes et aucun grammar crate n'etait dans Cargo.toml.

Ajoute dans `Cargo.toml` :
```toml
tree-sitter-typescript = "0.23"
tree-sitter-python = "0.23"
tree-sitter-rust = "0.23"
tree-sitter-go = "0.23"
tree-sitter-c = "0.23"
tree-sitter-cpp = "0.23"
tree-sitter-c-sharp = "0.23"
tree-sitter-css = "0.23"
tree-sitter-html = "0.23"
tree-sitter-scss = "1"
```

Decommente `set_language()` dans :
- `base_scope_extraction_parser.rs` (7 langages)
- `python_scope_extraction_parser.rs`
- `css_parser.rs`
- `scss_parser.rs` (attention : `tree_sitter_scss::language().into()`, pas `LANGUAGE`)
- `syntax_highlighting_parser.rs`

### 2. NodeTypeConfig non initialise pour TS/JS
`BaseScopeExtractionParser::new()` utilisait `NodeTypeConfig::default()` (vecteurs vides) pour tous les langages. Resultat : 1 scope par fichier (que le file_scope).

Fix : initialiser `node_types`, `stop_words`, `builtin_identifiers` selon le langage dans `new()`.

### 3. Defaults du RelationshipResolver
Rust avait `unwrap_or(false)` pour toutes les options, TS avait `true` par defaut.
- `include_contains`, `include_decorators`, `include_defined_in`, `include_uses_library`, `include_inverse` : tous passes a `unwrap_or(true)`.

### 4. Performance regex : macro `cached_regex!`
`detect_relationship_type()` compilait 2 `Regex::new()` a chaque appel x milliers de relations = 4456ms.

Cree `src/utils/regex_cache.rs` avec macro :
```rust
#[macro_export]
macro_rules! cached_regex {
    ($pattern:expr) => {{
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        RE.get_or_init(|| regex::Regex::new($pattern).unwrap())
    }};
}
```

Remplace 148 appels `Regex::new(r"...").unwrap()` par `cached_regex!(r"...")` dans 15 fichiers.
4 appels dynamiques `Regex::new(&variable)` gardes tels quels (non cachables).

## Resultats performance (release)

| Metrique | Rust | TypeScript | Ratio |
|---|---|---|---|
| Parse time | 397ms | 1309ms | **3.3x plus rapide** |
| Relationships | 44ms | 1932ms | **44x plus rapide** |
| **Total** | **~441ms** | **~3241ms** | **7.4x plus rapide** |

## Resultats fonctionnels

### Scopes : Rust 1127 vs TS 1125 (+2)

Seuls 2 fichiers different :

| Fichier | Rust | TS | Difference |
|---|---|---|---|
| `scope-extraction/BaseScopeExtractionParser.ts` | 89 | 88 | +1 Block `BaseScopeExtractionParser-scope-02` (L3408-L3416) |
| `wasm/types.ts` | 5 | 4 | +1 Module `file_scope_01` (L51-L53) |

### Relationships : Rust 21321 vs TS 21336 (-15)

Par type :

| Type | Rust | TS | Diff |
|---|---|---|---|
| CONSUMED_BY | 8772 | 8783 | -11 |
| CONSUMES | 8772 | 8783 | -11 |
| DEFINED_IN | 1127 | 1125 | +2 |
| HAS_PARENT | 749 | 748 | +1 |
| IMPLEMENTS | 585 | 585 | = |
| INHERITS_FROM | 372 | 369 | +3 |
| PARENT_OF | 749 | 748 | +1 |
| USES_LIBRARY | 195 | 195 | = |

### Differences detaillees

**25 relations only in Rust :**
- 8 viennent du scope-02 supplementaire (BaseScopeExtractionParser)
- `INHERITS_FROM: globalRegistry -> ParserRegistry` (TS dit CONSUMES)
- `INHERITS_FROM: CSHARP_BUILTIN_IDENTIFIERS -> BUILTIN_IDENTIFIERS` (TS dit CONSUMES, direction inversee)
- `INHERITS_FROM: GO_BUILTIN_IDENTIFIERS -> BUILTIN_IDENTIFIERS` (idem)
- Relations VueParser dans wasm/types.ts (scope supplementaire)

**36 relations only in TS :**
- `isLocalImport` detecte comme CONSUMES dans 7 scopes (Rust ne le detecte pas) — **a investiguer en priorite**
- `parseFile` relations dans markdown (extractCodeBlocks, extractLinks, etc.) — TS les detecte, Rust non
- `BaseLanguageParser -> initialize`/`parseFile` — CONSUMES detecte par TS pas Rust
- Direction inversee : CONSUMES vs INHERITS_FROM pour constantes (GO_BUILTIN_IDENTIFIERS, etc.)

## Fichiers modifies

- `Cargo.toml` — 10 grammar crates ajoutes
- `src/utils/regex_cache.rs` — cree (macro cached_regex!)
- `src/utils/mod.rs` — ajoute `pub mod regex_cache`
- 15 fichiers `.rs` — `use crate::cached_regex;` + remplacement regex
- `src/scope_extraction/base_scope_extraction_parser.rs` — node_types fix, set_language
- `src/relationship_resolution/relationship_resolver.rs` — defaults fix
- `examples/compare.rs` — benchmark Rust avec JSON normalise
- `examples/compare.mjs` — script de diff Rust vs TS

## Outils de comparaison

```bash
# Generer JSON Rust
cd packages/rag3db/extension/rag3weaver/codeparsers
cargo run --release --example compare -- <src_dir> 2>/dev/null > /tmp/rust_output.json

# Generer JSON TS
cd packages/ragforge-core/packages/codeparsers
node export-json.mjs  # ecrit /tmp/ts_output.json

# Comparer
node packages/rag3db/extension/rag3weaver/codeparsers/examples/compare.mjs
```

## Prochaines etapes

1. **Investiguer `isLocalImport`** : 7 scopes TS detectent des CONSUMES vers `isLocalImport` que Rust ne trouve pas — determiner qui a raison
2. **Relations markdown** : `parseFile -> extract*` — verifier la detection de references dans le Rust
3. **Direction INHERITS_FROM vs CONSUMES** : constantes globales (GO_BUILTIN_IDENTIFIERS = [...BUILTIN_IDENTIFIERS]) — clarifier la semantique
