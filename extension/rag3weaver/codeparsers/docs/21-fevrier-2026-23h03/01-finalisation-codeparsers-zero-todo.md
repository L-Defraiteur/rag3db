# 01 — Finalisation codeparsers : 0 todo, blake3 UUID, Rayon parallel (session 21 février 2026, 23h)

## Résumé

Cette session a finalisé le crate `codeparsers` (transpilé de TypeScript → Rust). On est passé de **59 todo dans 6 fichiers** à **0 todo, 0 erreurs, 0 warnings**.

4 blocs de travail :
1. **Déplacement** du crate vers son emplacement final dans rag3weaver
2. **Centralisation UUID blake3** — module `utils::hash`, remplacement des 7 `generate_uuid()` non-déterministes
3. **Nettoyage HTML** — suppression dom_tree, stub du parser
4. **Implémentation parallel/** — Rayon `par_iter()`, cache parsers `thread_local!`

## 1. Déplacement du crate

```
AVANT : packages/codeparsers-transpiler/output/
APRÈS : packages/rag3db/extension/rag3weaver/codeparsers/
```

Le crate est maintenant un sous-répertoire de rag3weaver (son consommateur principal).

## 2. Centralisation UUID blake3

### Problème

7 fichiers contenaient un `generate_uuid()` basé sur `SystemTime::now().as_nanos()` :
- **Non déterministe** : UUID différent à chaque exécution pour le même fichier
- **Collisions** : deux appels au même nanoseconde = même UUID
- **`simple_hash()`** utilisait `DefaultHasher` (SipHash) — non stable entre versions Rust

### Solution

Nouveau module `utils/hash.rs` avec 3 fonctions centralisées :

```rust
pub fn content_hash(text: &str) -> String {
    blake3::hash(text.as_bytes()).to_hex().to_string()
}

pub fn hash_to_uuid(hash_hex: &str) -> String {
    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}

pub fn blake3_uuid(input: &str) -> String {
    hash_to_uuid(&content_hash(input))
}
```

Cohérent avec `rag3weaver/src/uuid.rs` et `rag3weaver/src/hash.rs` (même pattern blake3).

### Fichiers modifiés

| Fichier | `generate_uuid()` remplacé par | `simple_hash()` remplacé par |
|---|---|---|
| `css/css_parser.rs` | `blake3_uuid("css:{path}")` | `content_hash()` |
| `scss/scss_parser.rs` | `blake3_uuid("scss:{path}")` | `content_hash()` |
| `markdown/markdown_parser.rs` | `blake3_uuid("md:{path}")` pour le doc, `blake3_uuid("md:{path}:{title}:{line}")` pour les sections | `content_hash()` |
| `generic/generic_code_parser.rs` | `blake3_uuid("generic:{path}:{name}:{line}")` (4 call sites) | `content_hash()` |
| `vue/vue_parser.rs` | `blake3_uuid("vue:{path}")` | `content_hash()` |
| `svelte/svelte_parser.rs` | `blake3_uuid("svelte:{path}")` | `content_hash()` |
| `html/html_document_parser.rs` | `blake3_uuid(input)` (le todo!() remplacé) | (n'existait pas) |

Également factorisé dans `relationship_resolver.rs` : les fonctions locales `hash_to_uuid()` et `blake3_uuid()` supprimées, remplacées par `use crate::utils::hash::blake3_uuid`.

Imports retirés de tous ces fichiers : `DefaultHasher`, `Hash`, `Hasher`, `SystemTime`.

### Modification signature markdown

`extract_sections(&self, lines, start_line)` → `extract_sections(&self, lines, start_line, file_path)` pour pouvoir générer des UUID déterministes par section.

## 3. Nettoyage module HTML

### Constat

Le module HTML (31 todo) faisait du parsing DOM complet avec tree-sitter — pas le métier de codeparsers. À remplacer par un convertisseur HTML→Markdown dédié pour ingestion LLM.

### Modifications

- **Supprimé** : `html/dom_tree.rs` (19 todo, 400+ lignes)
- **Remplacé** : `html/html_document_parser.rs` → stub de 15 lignes qui retourne `Err("HTML parsing not implemented")`
- **Conservé** : `html/types.rs` (utilisé par generic_worker et non_code_project_parser pour compilation)
- **Mis à jour** : `html/mod.rs` — retiré `pub mod dom_tree;`

## 4. Implémentation parallel/ avec Rayon

### Avant

4 fichiers, 28 `todo!()`, squelette vide avec source TS en commentaires. Architecture TS originale : Piscina worker pool (Node.js threads), file-by-file dispatch.

### Architecture Rust

Pas besoin de Piscina — **Rayon** `par_iter()` avec work-stealing natif.

#### `parser_worker.rs` — Parse un fichier code

```rust
thread_local! {
    static CACHE: RefCell<ParserCache> = RefCell::new(ParserCache::new());
}

struct ParserCache {
    typescript: Option<BaseScopeExtractionParser>,
    python: Option<PythonScopeExtractionParser>,
    rust: Option<RustScopeExtractionParser>,
    go: Option<GoScopeExtractionParser>,
    c: Option<CScopeExtractionParser>,
    cpp: Option<CppScopeExtractionParser>,
    csharp: Option<CSharpScopeExtractionParser>,
}
```

- `parse_file(task) -> ScopeFileAnalysis` — dispatch selon `task.language`
- Parsers cachés par thread (1 instance par langue par thread Rayon)
- Pour 10k fichiers sur 8 cores → ~8 instances au lieu de 10k

#### `generic_worker.rs` — Parse un fichier non-code

Même pattern `thread_local!` avec cache pour : Markdown, CSS, SCSS, Vue, Svelte, Generic.
HTML retourne `Err(...)`.

#### `project_parser.rs` — Orchestrateur code

```rust
pub fn parse_project(&self, options) -> ProjectAnalysis {
    // Phase 1: Lire les fichiers (séquentiel, I/O)
    // Phase 2: Parser en parallèle (Rayon par_iter + catch_unwind)
    // Phase 3: RelationshipResolver (séquentiel, cross-file)
}
```

- `EXTENSION_TO_LANGUAGE` mappe vers `SupportedLanguage` enum (pas des strings)
- `catch_unwind` pour ne pas crasher si un fichier panic
- `detect_language_from_path()`, `is_code_parser_supported()`, `get_supported_code_extensions()`

#### `non_code_project_parser.rs` — Orchestrateur non-code

```rust
pub fn parse_files(&self, options) -> NonCodeParseAnalysis {
    // Build tasks → par_iter → collect into typed HashMaps
}
```

- `EXTENSION_TO_PARSER` mappe vers `NonCodeParserType` enum
- Résultats dispatchés dans des HashMap typées (markdown_files, css_files, etc.)

### Nettoyage async

Retiré `async` de 5 parsers non-code — résidu du transpileur TS, aucun I/O réel :
- `css_parser.rs` : `async fn initialize/parse_file` → `fn`
- `scss_parser.rs` : idem
- `vue_parser.rs` : idem
- `svelte_parser.rs` : idem
- `generic_code_parser.rs` : idem

Supprimé les méthodes wrapper `fn generate_uuid(&self)` dans generic, vue, svelte (appelaient la free function supprimée).

### Dépendance ajoutée

```toml
rayon = "1"  # Cargo.toml
```

## Bilan

| Métrique | Avant session | Après session |
|---|---|---|
| `todo!()` | 59 | **0** |
| Fichiers avec todo | 6 | **0** |
| Compilation | 0 err, 0 warn | **0 err, 0 warn** |
| UUID non-déterministes | 7 fichiers | **0** (tout blake3) |
| `async` inutiles | 5 parsers | **0** |
| Parallélisme | aucun | **Rayon par_iter + thread_local cache** |

### Fichiers créés
- `src/utils/mod.rs`
- `src/utils/hash.rs`

### Fichiers supprimés
- `src/html/dom_tree.rs`

### Fichiers réécrits (0 todo)
- `src/parallel/parser_worker.rs`
- `src/parallel/generic_worker.rs`
- `src/parallel/project_parser.rs`
- `src/parallel/non_code_project_parser.rs`
- `src/html/html_document_parser.rs`

### Fichiers modifiés
- `Cargo.toml` (+rayon)
- `src/lib.rs` (+pub mod utils)
- `src/html/mod.rs` (-dom_tree)
- `src/css/css_parser.rs` (blake3, -async)
- `src/scss/scss_parser.rs` (blake3, -async)
- `src/markdown/markdown_parser.rs` (blake3, +file_path dans extract_sections)
- `src/generic/generic_code_parser.rs` (blake3, -async, +file_path dans extract_scopes/extract_chunks)
- `src/vue/vue_parser.rs` (blake3, -async)
- `src/svelte/svelte_parser.rs` (blake3, -async)
- `src/relationship_resolution/relationship_resolver.rs` (factorisé vers utils::hash)

## Prochaines étapes

Le crate `codeparsers` est **complet** (0 todo, compile, parallélisé).

Reste à faire hors codeparsers :
1. **Phase C rag3weaver** : Intégrer codeparsers comme dépendance de rag3weaver
2. **HTML→Markdown** : Convertisseur dédié (hors codeparsers) pour ingestion LLM, préservant liens et images markdown
3. **Tests** : Écrire des tests unitaires/intégration pour les parsers et le parallel
4. **Benchmarks** : Mesurer le gain Rayon vs séquentiel sur un vrai projet
