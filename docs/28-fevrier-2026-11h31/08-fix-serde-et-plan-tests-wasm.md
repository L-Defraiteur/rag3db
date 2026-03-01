# 08 — Fix serde FieldType + Plan tests WASM réalistes

## Bug corrigé : Tantivy schema panic

### Root cause

Dans `config.rs`, le champ `field_type` de `FieldDef` avait :
```rust
#[serde(default, rename = "type", alias = "field_type")]
pub field_type: FieldType,
```

Le `rename = "type"` **override** le `rename_all = "camelCase"` du struct. Donc serde attendait la clé JSON `"type"` ou `"field_type"`.

Mais le JS (weaver_worker.js, set_embedder_worker.js) envoyait `"fieldType"` (camelCase). Cette clé ne matchait rien → `#[serde(default)]` → `FieldType::String` (au lieu de `Text`).

Conséquence : tous les champs étaient `FieldType::String`, pas `Text` → inclus dans `filter_fields` → DDL généré avec `filter_fields := ['body', 'title']` → `title` à la fois dans fts_fields ET filter_fields → doublon dans le schema Tantivy → **PANIC**.

Le test unitaire `wasm_test_config_ddl` ne détectait pas le bug car il construisait le config directement en Rust, sans passer par la désérialisation JSON.

### Fix appliqué

**`config.rs`** — 2 changements :

1. Ajouté `alias = "fieldType"` sur le champ :
```rust
#[serde(default, rename = "type", alias = "field_type", alias = "fieldType")]
pub field_type: FieldType,
```

2. Ajouté alias PascalCase sur chaque variant de l'enum `FieldType` :
```rust
#[serde(alias = "String")]
String,
#[serde(alias = "Text")]
Text,
// etc.
```

Car le JS envoie `"Text"` (PascalCase) mais serde attend `"text"` (lowercase via `rename_all = "lowercase"`).

### Tests ajoutés

- `config.rs::field_type_pascal_case` — vérifie que tous les variants PascalCase désérialisent
- `config.rs::js_style_config_deserialization` — reproduit le JSON exact du JS, vérifie `FieldType::Text`
- `schema.rs::wasm_test_config_ddl_from_json` — vérifie que le DDL généré depuis JSON n'a PAS de filter_fields

### Logs debug nettoyés

Retirés tous les `eprintln!`/`fprintf(stderr)` de la session 07 :
- `handle.rs` — 3 eprintln! retirés
- `schema.rs` (ld-tantivy) — 1 eprintln! retiré, message panic simplifié
- `create_tantivy_index.cpp` — 8 fprintf retirés (bindFunc, bindFuncInternal, rewriteFunc, tableFunc)
- `catalog.rs` — 1 eprintln! retiré

## État après fix

- `cargo test` (rag3weaver) : **345 passed**, 0 failed
- `cargo test --lib` (ld-tantivy) : **1062 passed**, 0 failed
- Build WASM (toutes extensions : json, vector, algo, tantivy_fts, sparse_vector) : **OK**
- Playwright : **7/9 passed**
  - ✅ `rag3weaver.spec.js` — PASS (plus de panic Tantivy !)
  - ✅ `idbfs.spec.js` — PASS (phase 1 + phase 2 persistence)
  - ✅ `threading.spec.js` — PASS (3 tests)
  - ❌ `set_embedder.spec.js` — 2 FAIL (MiniLM + Multilingual)

## Problème restant : search retourne 0 résultats

Les 2 tests `set_embedder.spec.js` échouent car `search.results.length = 0`.

Le drain fonctionne (processed=3, failed=0, embeddings calculés), mais la recherche ne trouve rien : `vectorCount: 0, bm25Count: 0`.

### Causes identifiées

1. **`body` n'a pas de `contentFor`** dans les configs de test → body n'est PAS dans le KB → pas embedé, pas indexé FTS. Seul `title` est dans le KB. Config trop minimale.

2. **Jamais testé en natif** — les E2E Rust (`e2e_native.rs`) utilisent un config **sans knowledge bases** (commentaire : "Config WITHOUT knowledge bases — no extensions needed"). Search n'a JAMAIS été testé de bout en bout.

3. **Possible problème HNSW** — le vector index est créé à `initialize()` sur une table vide. Après `drain()` (insert de rows), les nouveaux vecteurs ne sont peut-être pas dans l'index HNSW. À investiguer.

## Plan : tests WASM réalistes et incrémentaux

### Objectif

Réécrire les tests WASM avec une config KB réaliste, couvrant toutes les stratégies de search selon le doc `01-strategie-modeles-embedding.md`.

### Config KB réaliste

```js
{
  name: "test-rag3weaver",
  entities: {
    Document: {
      fields: {
        title: { fieldType: "Text", titleFor: "main" },
        body: { fieldType: "Text", contentFor: "main" },
        category: { fieldType: "String" }  // filter field
      }
    }
  },
  relations: {
    REFERENCES: { from: "Document", to: "Document" }
  },
  knowledgeBases: { main: { search: "hybrid" } },
  embeddingDim: 384
}
```

### Phases de test incrémentales

| Phase | Ce qu'on teste | Embedder | Search mode | Assertions |
|-------|---------------|----------|-------------|------------|
| 1 | Create + drain + count | Mock | — | processed=N, count=N |
| 2 | BM25 seul | Mock | fulltext | results > 0, bonne pertinence |
| 3 | Dense (MiniLM-L6) | CandleEmbedder (22MB) | semantic | results > 0, ordre sémantique |
| 4 | Hybrid dense+BM25 | CandleEmbedder | hybrid | results > 0, fusion fonctionne |
| 5 | Multilingual | CandleEmbedder (multilingual, 471MB) | hybrid | cross-lingual FR→EN |
| 6 | Sparse BM42 | CandleEmbedder + BM42 | hybrid+sparse | sparseCount > 0 |

### Stratégie modèles (rappel doc 01)

| Plateforme | Dense | Sparse | Modèle |
|-----------|-------|--------|--------|
| WASM défaut | 384d | BM42 (attention hack) | multilingual-MiniLM-L12-v2 |
| WASM léger | 384d | BM42 | all-MiniLM-L6-v2 |
| Natif | 1024d | Appris (SPLADE-like) | BGE-M3 |

### Avant de coder les tests

1. **Investiguer pourquoi BM25 retourne 0** — même avec un MockEmbedder, BM25 devrait trouver des résultats textuels. Le problème est peut-être que les hooks Tantivy insert ne fonctionnent pas après `drain()` en WASM.

2. **Investiguer pourquoi vector retourne 0** — le HNSW index est créé sur table vide. Les rows insérées après `CREATE_VECTOR_INDEX` sont-elles automatiquement indexées ?

3. **Vérifier en natif d'abord** — ajouter un test E2E Rust avec KB + search pour isoler si le problème est WASM-only ou général.

## Fichiers modifiés cette session

| Fichier | Changement |
|---------|-----------|
| `extension/rag3weaver/src/config.rs` | Fix serde: alias `fieldType` + PascalCase variants + 2 tests |
| `extension/rag3weaver/src/schema.rs` | Test `wasm_test_config_ddl_from_json` |
| `extension/tantivy_fts/src/function/create_tantivy_index.cpp` | Retrait logs debug |
| `extension/tantivy/ld-tantivy/tantivy_fts/rust/src/handle.rs` | Retrait logs debug |
| `extension/tantivy/ld-tantivy/src/schema/schema.rs` | Retrait logs debug, simplification panic msg |
| `extension/rag3weaver/src/catalog.rs` | Retrait log debug initialize |
