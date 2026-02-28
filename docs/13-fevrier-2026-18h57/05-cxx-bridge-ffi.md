# Migration FFI : de `extern "C"` + JSON vers `cxx` bridge

> Remplacement de la couche FFI actuelle (13 fonctions extern C, cbindgen, JSON partout)
> par un bridge cxx typé avec structs partagés Rust ↔ C++.

## Statut : TERMINÉ

**Commits** sur `ld-tantivy:main` (13 février 2026) :
1. `127c15b` — ajout du bridge cxx (9 structs, 15 fonctions, ~350 lignes)
2. `9daf73e` — suppression de l'ancien API extern "C" (-1571 lignes)

**Fichiers finaux** :
- `tantivy_fts/rust/Cargo.toml` — `cxx = "1.0"` + `cxx-build = "1.0"`
- `tantivy_fts/rust/build.rs` — `cxx_build::bridge("src/bridge.rs")`
- `tantivy_fts/rust/src/bridge.rs` — le bridge complet (9 structs, 15 fonctions)
- `tantivy_fts/rust/src/lib.rs` — déclarations de modules uniquement

**Supprimé** :
- `tantivy_fts/include/tantivy_fts.h` — ancien header C (cbindgen)
- `tantivy_fts/test/test_ffi.c` — anciens 153 tests C
- 13 fonctions `extern "C"` dans lib.rs, helpers (`cstr_to_str`, `error_json`, `free_string`)
- Code mort dans query.rs (`execute_search`, `collect_results`, `SearchResult`)
- Code mort dans handle.rs (`raw_field_name`)

**Tests** : 1015 ld-tantivy = tout vert, 0 warning.

**Différences vs le design initial ci-dessous** :
- `SchemaField` et `DocField` retirés (inutiles, schema reste JSON)
- `highlight` retiré de `QueryConfig` (le C++ choisit `search` vs `search_with_highlights`)
- `get_field_ids` filtre automatiquement les champs internes (`._raw`, `._ngram`)
- `add_document_texts` / `add_document_mixed` auto-dupliquent vers `._raw` et `._ngram` (le C++ ne voit que les champs user)
- `delete_by_node_id` utilise `Term::from_field_u64` (corrige un bug de l'ancien `tantivy_delete_by_term` qui utilisait `from_field_text` sur un champ u64)
- `extract_node_id` utilise `CompactDocValue::as_value().as_u64()` (pas de roundtrip JSON)

---

## Pourquoi

| Actuel (extern C + JSON) | Cible (cxx) |
|--------------------------|-------------|
| Sérialisation JSON sur le hot path (add_document) | Structs typés, zéro sérialisation |
| `tantivy_free_string()` manuel | Ownership automatique (Box, String, Vec) |
| Erreurs via `tantivy_last_error()` ou JSON `{"error":"..."}` | `Result<T>` → exceptions C++ |
| Header C généré par cbindgen | Header C++ généré par cxx |
| Pointeurs opaques (`TantivyHandlePtr`) | `Box<TantivyHandle>` avec ownership |
| Tests en C (`test_ffi.c`) | Tests en C++ (ou Rust) |

---

## Architecture cxx

### Le bridge (côté Rust)

**Fichier** : `tantivy_fts/rust/src/bridge.rs`

```rust
#[cxx::bridge]
mod ffi {

    // ── Structs partagés (visibles des deux côtés) ──

    struct SchemaField {
        name: String,
        field_type: String,    // "text", "string", "i64", "u64", "f64"
        stored: bool,
        indexed: bool,
        fast: bool,
    }

    struct DocField {
        field_id: u32,
        field_type: u8,        // 0=text, 1=u64, 2=i64, 3=f64, 4=string
    }

    struct DocFieldText {
        field_id: u32,
        value: String,
    }

    struct DocFieldU64 {
        field_id: u32,
        value: u64,
    }

    struct DocFieldI64 {
        field_id: u32,
        value: i64,
    }

    struct DocFieldF64 {
        field_id: u32,
        value: f64,
    }

    struct SearchResult {
        node_id: u64,
        score: f32,
    }

    struct HighlightRange {
        start: u32,
        end: u32,
    }

    struct FieldHighlights {
        field_name: String,
        ranges: Vec<HighlightRange>,
    }

    struct SearchResultWithHighlights {
        node_id: u64,
        score: f32,
        highlights: Vec<FieldHighlights>,
    }

    struct IndexFieldInfo {
        field_id: u32,
        name: String,
        field_type: String,
    }

    // ── Fonctions Rust exposées au C++ ──

    extern "Rust" {
        type TantivyHandle;

        // Lifecycle
        fn create_index(path: &str, schema_json: &str) -> Result<Box<TantivyHandle>>;
        fn open_index(path: &str) -> Result<Box<TantivyHandle>>;
        // close = drop du Box<TantivyHandle> (automatique)

        // Schema introspection
        fn get_field_ids(handle: &TantivyHandle) -> Vec<IndexFieldInfo>;

        // Document operations (hot path — typé, zéro JSON)
        fn add_document_texts(
            handle: &TantivyHandle,
            node_id: u64,
            fields: &[DocFieldText],
        ) -> Result<i64>;

        fn add_document_mixed(
            handle: &TantivyHandle,
            node_id: u64,
            text_fields: &[DocFieldText],
            u64_fields: &[DocFieldU64],
            i64_fields: &[DocFieldI64],
            f64_fields: &[DocFieldF64],
        ) -> Result<i64>;

        fn delete_by_node_id(handle: &TantivyHandle, node_id: u64) -> Result<i64>;

        // Transaction
        fn commit(handle: &TantivyHandle) -> Result<i64>;
        fn rollback(handle: &TantivyHandle);
        fn reload_reader(handle: &TantivyHandle);

        // Search (query reste en JSON — flexible, appelé rarement)
        fn search(
            handle: &TantivyHandle,
            query_json: &str,
            limit: u32,
        ) -> Result<Vec<SearchResult>>;

        fn search_with_highlights(
            handle: &TantivyHandle,
            query_json: &str,
            limit: u32,
        ) -> Result<Vec<SearchResultWithHighlights>>;

        fn search_filtered(
            handle: &TantivyHandle,
            query_json: &str,
            limit: u32,
            allowed_ids: &[u64],
        ) -> Result<Vec<SearchResult>>;

        fn search_filtered_with_highlights(
            handle: &TantivyHandle,
            query_json: &str,
            limit: u32,
            allowed_ids: &[u64],
        ) -> Result<Vec<SearchResultWithHighlights>>;

        // Info
        fn num_docs(handle: &TantivyHandle) -> u64;
        fn get_schema_json(handle: &TantivyHandle) -> String;
    }
}
```

### Côté C++ (généré automatiquement par cxx)

cxx génère un header `tantivy_fts/rust/src/bridge.rs.h` qu'on inclut :

```cpp
#include "bridge.rs.h"

// Utilisation directe, typée :
auto handle = create_index("/path/to/index", schema_json.c_str());

// Hot path — zéro sérialisation
rust::Vec<DocFieldText> fields;
fields.push_back(DocFieldText{0, rust::String("Rust programming is great")});
fields.push_back(DocFieldText{1, rust::String("A tutorial about Rust")});
add_document_texts(*handle, /*node_id=*/42, fields);

commit(*handle);
reload_reader(*handle);

// Search
auto results = search_with_highlights(*handle, query_json.c_str(), 10);
for (const auto& r : results) {
    // r.node_id, r.score, r.highlights — tout typé
}

// Close = juste laisser handle sortir du scope (drop automatique)
```

---

## Mapping avec les 13 fonctions actuelles

| Fonction actuelle (extern C) | Équivalent cxx | Changement |
|------------------------------|---------------|------------|
| `tantivy_create_index(path, schema_json)` | `create_index(path, schema_json)` | Retourne `Result<Box<>>` au lieu de ptr nullable |
| `tantivy_open_index(path)` | `open_index(path)` | Idem |
| `tantivy_close_index(handle)` | Drop du `Box<TantivyHandle>` | Automatique |
| `tantivy_add_document(handle, doc_json)` | `add_document_texts(handle, node_id, fields)` | **Typé, plus de JSON** |
| `tantivy_delete_by_term(handle, field, value)` | `delete_by_node_id(handle, node_id)` | Simplifié (on delete toujours par node_id) |
| `tantivy_commit(handle)` | `commit(handle)` | `Result<i64>` |
| `tantivy_rollback(handle)` | `rollback(handle)` | Identique |
| `tantivy_reload_reader(handle)` | `reload_reader(handle)` | Identique |
| `tantivy_search(handle, query_json, limit)` | `search(handle, query_json, limit)` | Retourne `Vec<SearchResult>` typé |
| `tantivy_search_filtered(handle, query_json, limit, ids, n)` | `search_filtered(handle, query_json, limit, ids)` | `&[u64]` slice au lieu de ptr+len |
| `tantivy_num_docs(handle)` | `num_docs(handle)` | Identique |
| `tantivy_get_schema(handle)` | `get_schema_json(handle)` | Retourne `String` (ownership auto) |
| `tantivy_free_string(ptr)` | **Supprimée** | Plus nécessaire — ownership automatique |

**+2 nouvelles** : `search_with_highlights`, `search_filtered_with_highlights` — les highlights sont des structs typés au lieu de JSON imbriqué.

**+1 nouvelle** : `add_document_mixed` — pour les documents avec des champs de types variés (text + u64 + i64 pour les filter fields).

**+1 nouvelle** : `get_field_ids` — retourne le mapping nom → field_id pour que le C++ sache quel field_id utiliser dans `add_document_*`.

---

## Intégration build

### Cargo.toml

```toml
[dependencies]
cxx = "1.0"

[build-dependencies]
cxx-build = "1.0"
```

### build.rs

```rust
fn main() {
    cxx_build::bridge("src/bridge.rs")
        .flag_if_supported("-std=c++17")
        .compile("tantivy_fts_cxx");
}
```

### CMakeLists.txt (extension tantivy_fts)

Le `cargo build` produit maintenant :
- `libtantivy_fts.a` — la lib statique Rust (comme avant)
- `target/cxxbridge/tantivy_fts/src/bridge.rs.h` — le header C++ généré

```cmake
# Ajouter le chemin du header cxx généré
include_directories(
    ${RUST_WORKSPACE_DIR}/target/cxxbridge/tantivy_fts/src/
    ${RUST_WORKSPACE_DIR}/target/cxxbridge/rust/  # cxx runtime headers
)
```

Le linkage reste identique (`libtantivy_fts.a` + pthread + m + dl).

---

## Migration progressive

### ~~Étape 1 : Setup cxx + fonctions de base~~ ✓ (`127c15b`)
### ~~Étape 2 : Hot path documents~~ ✓ (`127c15b`)
### ~~Étape 3 : Search~~ ✓ (`127c15b`)
### ~~Étape 4 : Nettoyage~~ ✓ (`9daf73e`)

Tout fait. Il ne reste plus que le bridge cxx, l'ancien API C est entièrement supprimé.

---

## Décisions de design

### Query JSON : on le garde

Le query reste en JSON (`&str`) :
- Appelé une fois par recherche (pas un hot path)
- Très flexible (facile d'ajouter des champs : `filters`, `highlight`, etc.)
- Le user Cypher construit la query comme une string de toute façon
- Le parsing serde côté Rust est négligeable

### Schema JSON : on le garde aussi

Même raisonnement — appelé une fois à la création de l'index.

### Documents : structs typés

C'est le hot path (milliers/millions d'appels pendant l'indexation). Deux variantes :
- `add_document_texts` — cas simple, que des champs texte + node_id
- `add_document_mixed` — cas complet avec filter fields (u64, i64, f64)

### Résultats : structs typés

Les `SearchResult` et `SearchResultWithHighlights` évitent de parser du JSON côté C++. Les highlights sont des `Vec<FieldHighlights>` avec des `Vec<HighlightRange>` — tout typé, pas de `"[[5,16],[20,25]]"` à parser.

### Ownership

- `Box<TantivyHandle>` côté C++ → drop automatique à la destruction
- `String` et `Vec` traversent la frontière avec ownership correct
- Plus de `free_string`, plus de leaks possibles

---

## Impact sur les autres docs

- **Doc 02** : QUERY_TANTIVY_INDEX utilise `search_with_highlights()` au lieu de `tantivy_search()` + JSON parse. Plus besoin de nlohmann/json pour les résultats.
- **Doc 03** : Obsolète — pas de modification du core nécessaire (voir doc 04).
- **Doc 04** : `TantivyIndex::onInsert` appelle `add_document_texts()` / `add_document_mixed()` directement. `onCommit` appelle `commit()` + `reload_reader()`. `onRollback` appelle `rollback()`.
