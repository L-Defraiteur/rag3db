# 10 - Refactor scope lines + content body-only (EN COURS)

## Objectif

Deux changements combines :
1. **Renommer `start_line`/`end_line`** en `scope_start_line`/`scope_end_line` et ajouter `signature_start_line`/`signature_end_line` + `body_start_line`/`body_end_line`
2. **`scope.content`** ne doit plus contenir la signature — seulement le body

## Design adopte

```
scope_start_line / scope_end_line       → le scope entier (ancien start_line/end_line)
signature_start_line / signature_end_line → la signature seulement
body_start_line / body_end_line          → le body seulement (Option<usize>, None si absent)
content                                  → body-only text (plus de duplicat de signature)
content_dedented                         → derive de content (se corrige auto)
```

## Etat d'avancement

### Etape 1 : Modification du struct ScopeInfo — FAIT

**Fichier** : `src/scope_extraction/types.rs`

Ancien :
```rust
pub start_line: usize,
pub end_line: usize,
```

Nouveau :
```rust
pub scope_start_line: usize,
pub scope_end_line: usize,
pub signature_start_line: usize,
pub signature_end_line: usize,
pub body_start_line: Option<usize>,
pub body_end_line: Option<usize>,
```

### Etape 2 : Rename dans les scope_extraction parsers — FAIT (7 agents paralleles)

7 agents lances en parallele, tous termines avec succes :

| Fichier | ScopeInfo inits | .start_line accesses | Status |
|---|---|---|---|
| `base_scope_extraction_parser.rs` | ~6 blocs | ~27 accesses | FAIT (agent aea0c8e) |
| `cpp_scope_extraction_parser.rs` | 4 blocs (namespace, class, method, enum) | 1 (sort_by_key) | FAIT (agent a1b70d0) |
| `python_scope_extraction_parser.rs` | 4 blocs (class, function, lambda, variable) | 7 accesses | FAIT (agent adc5ec3) |
| `go_scope_extraction_parser.rs` | 3 blocs (type, function, method) | 1 (sort_by_key) | FAIT (agent ae782ea) |
| `c_scope_extraction_parser.rs` | 4 blocs (function, class, enum, type_alias) | 1 (sort_by_key) | FAIT (agent a253dc1) |
| `c_sharp_scope_extraction_parser.rs` | 7 blocs (namespace, class, record, interface, enum, method, constructor) | 1 (sort_by_key) | FAIT (agent a640f0a) |
| `rust_scope_extraction_parser.rs` | 6 blocs (module, impl, struct, trait, enum, function) | 1 (sort_by_key) | FAIT (agent acdcbd3) |

Pattern applique partout : `start_line,` → `scope_start_line: start_line, signature_start_line: start_line, signature_end_line: start_line, body_start_line: None, body_end_line: None,` et `end_line,` → `scope_end_line: end_line,`

### Etape 3 : Rename dans les fichiers secondaires — FAIT

| Fichier | Changement |
|---|---|
| `relationship_resolution/relationship_resolver.rs` | `scope.start_line` → `scope.scope_start_line` (3 sites) |
| `typescript/type_script_language_parser.rs` | `scope.start_line` / `scope.end_line` → `scope.scope_start_line` / `scope.scope_end_line` |
| `python/python_language_parser.rs` | idem |
| `go/go_language_parser.rs` | idem |
| `c/c_language_parser.rs` | idem |
| `cpp/cpp_language_parser.rs` | idem |
| `csharp/c_sharp_language_parser.rs` | idem |
| `rust/rust_language_parser.rs` | idem |

**Note** : `ScopeMappingEntry` dans `relationship_resolution/types.rs` garde ses propres `start_line`/`end_line` (type interne, pas ScopeInfo).

**Note** : `UniversalScopeInfo` dans `base/universal_types.rs` garde ses propres `start_line`/`end_line` (type interne separé).

**Note** : css, vue, svelte, markdown, generic, scss — ces parsers ont leurs propres types avec `start_line`/`end_line` qui ne sont PAS `ScopeInfo`, donc pas affectes.

### Etape 4 : Verifier compilation — A FAIRE

```bash
cd packages/rag3db/extension/rag3weaver/codeparsers && cargo check
```

Le base parser (agent aea0c8e) n'avait pas encore fini quand le rapport a ete demande. Il est probablement termine maintenant.

### Etape 5 : Body extraction (content = body-only) — A FAIRE

Pour chaque parser, remplacer :
```rust
let node_content = self.base.get_node_text(Some(node), content);
```
par :
```rust
let node_content = node.child_by_field_name("body")
    .map(|body| self.base.get_node_text(Some(body), content))
    .unwrap_or_else(|| self.base.get_node_text(Some(node), content));
```

Et peupler les body lines :
```rust
let (body_start_line, body_end_line) = node.child_by_field_name("body")
    .map(|b| (Some(b.start_position().row + 1), Some(b.end_position().row + 1)))
    .unwrap_or((None, None));
```

Et peupler signature_end_line correctement :
```rust
let signature_end_line = body_start_line
    .map(|bl| if bl > start_line { bl - 1 } else { start_line })
    .unwrap_or(end_line);
```

### Mapping body nodes par langage (verifie par test AST)

| Langage | Node type | child_by_field_name("body") |
|---|---|---|
| TypeScript | class_declaration | class_body |
| TypeScript | function_declaration | statement_block |
| TypeScript | method_definition | statement_block |
| TypeScript | enum_declaration | enum_body |
| TypeScript | interface_declaration | interface_body |
| Python | class_definition | block |
| Python | function_definition | block |
| Go | function_declaration | block |
| Go | method_declaration | block |
| Go | type_spec | **no_body** (fallback) |
| C | struct_specifier | field_declaration_list |
| C | function_definition | compound_statement |
| C | enum_specifier | enumerator_list |
| C++ | class_specifier | field_declaration_list |
| C++ | function_definition | compound_statement |
| C++ | enum_specifier | enumerator_list |
| C++ | namespace_definition | declaration_list |
| C# | class_declaration | declaration_list |
| C# | struct_declaration | declaration_list |
| C# | method_declaration | block |
| C# | interface_declaration | declaration_list |
| C# | enum_declaration | enum_member_declaration_list |
| Rust | struct_item | field_declaration_list |
| Rust | function_item | block |
| Rust | impl_item | declaration_list |
| Rust | trait_item | declaration_list |
| Rust | enum_item | enum_variant_list |

### Etape 6 : Tests — A FAIRE

Les tests existants (49 tests relationships.rs) n'assertent PAS sur `content` ni sur `start_line`/`end_line`. Ils devraient passer sans modification.

Ajouter des tests specifiques pour verifier :
- `content` ne contient pas la signature
- `body_start_line`/`body_end_line` sont corrects
- `signature_start_line`/`signature_end_line` sont corrects

## Commits effectues avant ce refactor

- `55e7276` — feat: codeparsers Rust crate + cross-language coverage fixes (49/49 tests)
- `3585615` — chore: add .gitignore for codeparsers, remove target/ from tracking

Point de retour propre en cas de besoin de revert.
