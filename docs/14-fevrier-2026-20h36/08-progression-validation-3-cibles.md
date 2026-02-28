# Progression — Validation regex contains sur 3 cibles + fixes cmake/nodejs

Date : 14 fevrier 2026

## Statut : Natif OK, Node.js OK, WASM KO (regex retourne 0 resultats)

## Ce qui a ete fait

### 1. Fix cmake DEPENDS sur sources Rust (FAIT)

**Fichier** : `extension/tantivy_fts/CMakeLists.txt`

Le probleme historique : `add_custom_command(OUTPUT libtantivy_fts.a ...)` sans `DEPENDS` sur les `.rs` — cmake ne relancait jamais cargo meme apres modif Rust.

**Fix** : ajout de `file(GLOB_RECURSE)` + `DEPENDS` :

```cmake
file(GLOB_RECURSE RUST_SOURCES
    "${RUST_WORKSPACE_DIR}/src/*.rs"
    "${RUST_WORKSPACE_DIR}/tantivy_fts/rust/src/*.rs"
    "${RUST_WORKSPACE_DIR}/Cargo.toml"
    "${RUST_WORKSPACE_DIR}/tantivy_fts/rust/Cargo.toml"
)

add_custom_command(
    OUTPUT ${TANTIVY_STATIC_LIB}
    COMMAND ... cargo build ...
    DEPENDS ${RUST_SOURCES}
    ...
)
```

Limitation : les NOUVEAUX fichiers `.rs` ne sont detectes qu'apres `cmake` re-configuration. Mais les MODIFICATIONS de fichiers existants sont detectees immediatement. C'est largement suffisant.

Les 3 cibles (release, nodejs, wasm) ont ete reconfigurees avec ce fix.

### 2. Fix Node.js output directory (FAIT)

**Fichier** : `tools/nodejs_api/CMakeLists.txt`

Le `.node` etait builde dans `tools/nodejs_api/build/` mais le loader (`rag3db_native.js`) l'attend dans `tools/nodejs_api/src_js/`.

**Fix** : change les output directories de `build` a `src_js` :

```cmake
RUNTIME_OUTPUT_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/src_js"
LIBRARY_OUTPUT_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/src_js"
ARCHIVE_OUTPUT_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/src_js"
```

Plus besoin de `cp build/rag3dbjs.node src_js/` manuellement.

### 3. Validation natif (OK — 11/11)

```
[  PASSED  ] 11 tests.
```

Tous les tests GTest passent, y compris `TantivyRegexContainsTest` avec les 7 sub-tests (regex trigram, BM25, hybrid, no match, highlights, regression, short-literal).

### 4. Validation Node.js natif (OK — 5/5)

```
CREATE INDEX: OK
1. Regex contains: OK (3)
2. Hybrid regex+fuzzy: OK (2)
3. Short-literal regex: OK (1)
4. Regex no match: OK (0)
5. Fuzzy regression: OK (2)
```

Teste via script inline Node.js avec `getAll()` (pas `hasNext()/getNext()` — l'API NAPI a un bug avec le loop manuel sur les TableFunc, utiliser `getAll()` a la place).

### 5. Validation WASM (KO — regex retourne 0)

```
CREATE INDEX: OK
1. Regex contains: FAIL (0)
2. Hybrid regex+fuzzy: FAIL (0)
3. Short-literal regex: FAIL (0)
4. Regex no match: OK (0)
5. Fuzzy regression: OK (2)
```

Le fuzzy contains marche (regression OK), mais TOUTES les queries regex retournent 0 resultats. Le cargo WASM a bien ete recompile (on voit `Compiling tantivy-fts` et `Compiling ld-tantivy` dans la sortie cmake), le WASM est re-linke.

**Hypotheses pour le bug WASM regex** :

1. **`regex` crate en WASM** : la crate `regex` compile en wasm32 mais peut avoir des differences de comportement (Unicode tables, allocations). Verifier que `Regex::new("(?i)program[a-z]+")` compile et matche en WASM.

2. **`regex-syntax` Extractor en WASM** : l'extraction des litteraux via `Extractor::new().extract(&hir)` pourrait retourner des resultats differents en WASM. Verifier que `seq.literals()` retourne bien des litteraux.

3. **Candidate collection vide** : si l'Extractor retourne des litteraux mais que les trigrams ne trouvent pas de candidats (probleme d'encodage ou de comparaison de strings en WASM), on obtient 0 resultats.

4. **Fallback path** : meme le short-literal regex (full-scan, `0..max_doc`) retourne 0. Donc soit `verify_regex()` echoue (le regex ne matche pas le texte stocke), soit les candidats sont bien la mais le scorer ne retourne rien.

5. **Le probleme pourrait etre dans `Regex::new()`** : si la construction du regex echoue silencieusement en WASM, `build_contains_regex()` retourne une erreur qui est avalee par le C++. Verifier les logs d'erreur.

**Piste de debug prioritaire** : ajouter un test Rust minimal qui tourne en WASM (ou un print/log dans `build_contains_regex`) pour verifier que le regex compile et que l'Extractor fonctionne.

## Fichiers modifies dans cette session

| Fichier | Changement |
|---------|------------|
| `extension/tantivy_fts/CMakeLists.txt` | `file(GLOB_RECURSE)` + `DEPENDS` sur sources Rust |
| `tools/nodejs_api/CMakeLists.txt` | Output dir `build` → `src_js` |

## Commits a faire

Rien n'est commite. Les 2 fixes cmake + le fix Node.js output dir sont des modifications locales. A commiter une fois le bug WASM resolu (ou en l'etat si on decide de traiter le WASM plus tard).

## Ce qu'il reste

1. **Debug WASM regex** — comprendre pourquoi les queries regex retournent 0 en WASM
2. **Commiter** les fixes cmake DEPENDS + Node.js output dir
3. **Mettre a jour** le submodule ld-tantivy dans rag3db
4. **Mettre a jour** les BUILD.md (retirer la mention du rebuild manuel, documenter le fix)
