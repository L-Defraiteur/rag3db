# 07 — Fix QUERY_TANTIVY_INDEX + Tests E2E

Session du 14 février 2026, ~00h00–00h30.
Reprend là où SAVE_CONTEXT_13_fevrier_23h08.md s'est arrêté.

## Bugs corrigés (3)

### Bug 1 : `bindFunc` partagée entre public et internal

**Symptôme** : `QUERY_TANTIVY_INDEX` → "unknown field: body". Le schema Tantivy ne contenait que `_node_id`.

**Root cause** : `create_tantivy_index.cpp` utilisait la même `bindFunc` pour `CREATE_TANTIVY_INDEX` (2 params : tableName, propertyList) et `_CREATE_TANTIVY_INDEX` (3 params : tableName, indexName, propertyList). Le `rewriteFunc` du public générait un appel à l'interne avec 3 params, mais `bindFunc` lisait `param(1)` comme la propertyList — or pour l'interne, `param(1)` est l'indexName (STRING "doc"), pas la liste. Résultat : `propertyNames` vide → schema `{"fields":[],"stemmer":"english"}`.

**Fix** : Séparation en `bindFunc` (public, 2 params) et `bindFuncInternal` (internal, 3 params) + extraction d'un helper `resolveProperties()`.

### Bug 2 : `stringFormat` et `{{` → JSON invalide

**Symptôme** : `invalid schema JSON: key must be a string at line 1 column 13`

**Root cause** : Le code utilisait `stringFormat("{{\"name\":\"{}\",...}}", propName)` en supposant que `{{` produirait `{` littéral (comme fmt/Python). Mais `stringFormat` de rag3db ne gère pas cet échappement → produit `{{"name":"title",...}}` (double accolade = JSON invalide).

**Fix** : Construction du JSON par concaténation directe de strings.

### Bug 3 : `_node_id` non STORED → node_id toujours 0

**Symptôme** : Tous les résultats de recherche retournaient `node_id: 0`.

**Root cause** : Dans `handle.rs`, le champ `_node_id` était déclaré `FAST | INDEXED` sans `STORED`. `extract_node_id()` dans `bridge.rs` lit le stored document via `doc.get_first()` → champ absent → `unwrap_or(0)`.

**Fix** : `FAST | INDEXED | STORED` dans `handle.rs:178`.

## Fichiers modifiés

| Fichier | Modification |
|---------|-------------|
| `extension/tantivy_fts/src/function/create_tantivy_index.cpp` | Séparation bindFunc/bindFuncInternal, helper resolveProperties(), fix JSON |
| `extension/tantivy_fts/src/function/query_tantivy_index.cpp` | Retiré debug fprintf |
| `extension/tantivy_fts/src/index/tantivy_index.cpp` | Retiré debug fprintf |
| `extension/tantivy/ld-tantivy/tantivy_fts/rust/src/handle.rs` | `_node_id` : ajouté STORED |
| `extension/tantivy/ld-tantivy/tantivy_fts/rust/src/bridge.rs` | Retiré debug_handle_info temporaire |

## Tests E2E — Résultats

Tous en mode in-memory (le shell crash en mode fichier — bug pré-existant).

| Test | Query JSON | Résultat |
|------|-----------|----------|
| contains | `{"type":"contains","field":"body","value":"programming"}` | node_id 0,2 + highlights OK |
| term | `{"type":"term","field":"body","value":"programming"}` | node_id 0,2 OK |
| fuzzy | `{"type":"fuzzy","field":"body","value":"programing","distance":1}` | node_id 0,2 OK |
| phrase | `{"type":"phrase","field":"body","terms":["systems","programming"]}` | node_id 0 seul, OK |
| parse | `{"type":"parse","field":"body","value":"rust AND programming"}` | node_id 0 seul, OK |
| contains c++ | `{"type":"contains","field":"title","value":"c++"}` | node_id 2 via regex fallback, OK |
| DROP | `CALL DROP_TANTIVY_INDEX('doc')` | OK, query après = erreur attendue |
| Incrémental | INSERT après CREATE_INDEX | Doc ajouté au writer mais pas visible avant checkpoint (comportement attendu) |

## Note sur l'incrémental

`TantivyIndex::insert()` appelle `add_document_texts()` mais PAS `commit()`/`reload_reader()`. Le reader ne voit les nouveaux docs qu'après `checkpointInMemory()`, déclenché par le checkpoint de rag3db. C'est volontaire pour la performance.

## Tests Rust

- 1015 tests ld-tantivy : OK (après ajout de STORED sur _node_id)

## Prochaines étapes

- Phase B : Tests E2E via API C (pas le shell) pour tester le mode fichier
- Phase C : Intégration Rag3Weaver
