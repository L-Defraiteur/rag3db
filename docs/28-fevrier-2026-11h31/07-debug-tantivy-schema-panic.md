# 07 — Debug Lucivy schema panic WASM

## Le bug

**Symptôme** : `thread panicked at src/schema/schema.rs:202: Field already exists in schema title`

Tous les tests WASM Playwright cassés (rag3weaver.spec.js ET set_embedder.spec.js). Les 15 tests natifs GTest passent. Les 341 tests cargo passent.

## Root cause trouvée

Le DDL exécuté par `initialize()` est :
```
CALL CREATE_LUCIVY_INDEX('Document', ['title'], filter_fields := ['body', 'title'])
```

Mais le code Rust `generate_fts_index_ddl()` génère :
```
CALL CREATE_LUCIVY_INDEX('Document', ['title'])
```
**SANS filter_fields** (vérifié par test unitaire `wasm_test_config_ddl` — les deux champs sont `FieldType::Text`, donc exclus des filter_fields).

## Chaîne de debug complète

1. **`[initialize]`** exécute `CALL CREATE_LUCIVY_INDEX('Document', ['title'], filter_fields := ['body', 'title'])` — le DDL contient `filter_fields` QUI N'EXISTE PAS dans le code Rust !

2. **`[bindFunc PUBLIC]`** reçoit `optionalParams.size=1` avec `filter_fields = [body, title]` — Kuzu injecte ce paramètre alors que le Cypher n'en contient pas

3. **`[rewriteFunc]`** génère `CALL _CREATE_LUCIVY_INDEX('Document', 'Document', ['title'], stemmer := 'english', filter_fields := ['body', 'title'])` — propage le filter_fields injecté

4. **`[bindFuncInternal]`** reçoit `filterFields.size=2` → body (string) + title (string)

5. **`[tableFunc]`** construit le JSON avec 3 champs : `title:text` (du propertyNames) + `body:string` + `title:string` (du filterFields) → `title` dupliqué → PANIC

## Hypothèse probable

**Le framework Kuzu/rag3db injecte automatiquement `filter_fields` comme optional param** quand il ne devrait pas. C'est un comportement WASM-only (les tests natifs passent).

Possibilités :
- Le binder Kuzu pour `StandaloneTableFunc` ajoute des params automatiquement en WASM
- Un cache/état global corrompu via `--allow-multiple-definition` qui fait qu'une fonction du binder résout mal les params
- L'`inferInputTypes` ou un autre hook ajoute des params

**Fichier clé à investiguer** : `src/binder/bind/bind_table_function.cpp` — c'est là que `optionalParams` est peuplé depuis les expressions parsées. Le code ne devrait ajouter que les params avec alias (syntaxe `key := value`), mais quelque chose ajoute `filter_fields` qui n'est pas dans le Cypher original.

## Ce qui a été fait pour débugger

### Logs ajoutés (à retirer après fix)

| Fichier | Log ajouté |
|---------|-----------|
| `ld-lucivy/lucivy_fts/rust/src/handle.rs` | `eprintln!` dans `build_schema()` : dump des champs config |
| `ld-lucivy/src/schema/schema.rs` | `eprintln!` dans `SchemaBuilder::add_field()` : dump keys existantes |
| `extension/lucivy_fts/src/function/create_lucivy_index.cpp` | `fprintf(stderr)` dans `bindFunc`, `bindFuncInternal`, `rewriteFunc`, `tableFunc` |
| `extension/rag3weaver/src/catalog.rs` | `eprintln!` dans `initialize()` pour les index DDL |

### Script rebuild.sh créé

**`tools/wasm/rebuild.sh`** — force toujours un clean configure (supprime CMakeCache.txt + generated_extension_loader), accepte `--no-sparse`, `--no-lucivy`, `--only-configure`, `--dir`, `-jN`.

### extension_config.cmake modifié

Ajouté `WASM_EXCLUDE_EXTENSIONS` variable cmake pour pouvoir exclure des extensions via `--no-sparse` etc.

### Test unitaire ajouté

`schema.rs::tests::wasm_test_config_ddl` — reproduit exactement la config WASM et vérifie que le DDL n'a PAS de filter_fields.

## Ce qui a été prouvé

1. **Ce n'est PAS sparse_vector** — le panic se produit aussi sans sparse_vector (build `--no-sparse`)
2. **Ce n'est PAS le linkage `--allow-multiple-definition`** (probablement) — le schemaJson C++ est correct mais les données en entrée sont corrompues
3. **C'est le framework Kuzu qui injecte filter_fields** — vérifié par logs : le `bindFunc PUBLIC` reçoit `filter_fields` comme optional param alors que le Cypher n'en contient pas
4. **Natif fonctionne** — 15 tests GTest passent, le même code C++ fonctionne

## Prochaines étapes

1. **Investiguer pourquoi Kuzu injecte `filter_fields`** dans les optionalParams en WASM mais pas en natif
   - Comparer le parsing du Cypher `CALL CREATE_LUCIVY_INDEX('Document', ['title'])` entre WASM et natif
   - Ajouter un log dans `src/binder/bind/bind_table_function.cpp` pour voir les expressions parsées
   - Vérifier si c'est un problème de `--allow-multiple-definition` sur le BINDER (pas le Lucivy)

2. **Alternative : filtrer côté C++** — dans `bindFunc`/`bindFuncInternal`, exclure de filterFields les champs qui sont déjà dans propertyNames (fix rapide)

3. **Alternative : dédupliquer dans build_schema Rust** — skip les champs dont le nom existe déjà (fix défensif)

## Fichiers modifiés cette session (non committés, en plus de la session 06)

| Fichier | Changement |
|---------|-----------|
| `tools/wasm/rebuild.sh` | Nouveau — script de build WASM avec clean configure |
| `extension/extension_config.cmake` | Ajouté WASM_EXCLUDE_EXTENSIONS |
| `extension/lucivy_fts/src/function/create_lucivy_index.cpp` | Logs debug (à retirer) |
| `extension/lucivy/ld-lucivy/lucivy_fts/rust/src/handle.rs` | Logs debug (à retirer) |
| `extension/lucivy/ld-lucivy/src/schema/schema.rs` | Logs debug (à retirer) |
| `extension/rag3weaver/src/catalog.rs` | Log debug dans initialize() (à retirer) |
| `extension/rag3weaver/src/schema.rs` | Test unitaire `wasm_test_config_ddl` (à garder) |

## État compilation/tests

- `cargo test` : 342 passed (1 nouveau test), 0 failed
- Build WASM (sans sparse_vector) : OK
- Tests WASM Playwright : CASSÉS (panic Lucivy — filter_fields injecté par Kuzu)
- Tests natifs GTest : 15 passed
