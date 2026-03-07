# 10 — lucivy-core extraction terminee

## Quoi

Le code partage de `lucivy-fts` (handle, query, tokenizer, directory) a ete extrait dans un nouveau crate `lucivy-core`. Tous les bindings (cpp, wasm, python, nodejs) dependent maintenant de `lucivy-core` au lieu de `lucivy-fts`.

`LucivyHandle::create` et `::open` prennent un `impl Directory` au lieu d'un path string. La config `_config.json` passe par `Directory::atomic_write/read`.

Detail complet : `extension/lucivy/ld-lucivy/docs/7-mars-2026-08h35/04-lucivy-core-extraction.md`

## Impact sur rag3weaver

`lucivy-fts` n'exporte plus `handle`, `query`, `tokenizer`, `directory`. Ces modules sont dans `lucivy-core`.

`rag3weaver` depend de `lucivy-fts` via le bridge CXX — le bridge (`bridge.rs`) a deja ete migre pour importer depuis `lucivy_core::`. Donc **rag3weaver ne devrait pas etre impacte directement**, le bridge CXX est la seule interface.

## Tests e2e a relancer

Les tests e2e (`e2e_native.rs`, `e2e_phase0b.rs`) sont derriere `#[cfg(feature = "rag3db-native")]` et necessitent l'environnement C++ complet :

```bash
RAG3DB_SHARED=1 \
RAG3DB_LIBRARY_DIR=.../build/release/src \
RAG3DB_INCLUDE_DIR=.../build/release/src \
LD_LIBRARY_PATH=.../build/release/src \
cargo test --features rag3db-native --test e2e_native --test e2e_phase0b -- --ignored
```

Ces tests n'ont pas pu etre lances depuis l'instance lucivy (pas d'acces au build C++). A faire cote rag3weaver pour confirmer que le bridge CXX fonctionne de bout en bout apres l'extraction.

## Verification deja faite

- `cargo test -p lucivy-core` : 48/48 OK
- `cargo build -p lucivy-fts --features cxx-bridge` : OK
- `cargo check` : les 6 crates compilent (lucivy-core, lucivy-fts, lucivy-cpp, lucivy-wasm, lucivy, lucivy-napi)
