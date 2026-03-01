# 20 — Root Cause & Fix : HashMap non-déterministe corrompt LIST<STRUCT>

## Résumé

Le bug UNWIND + MATCH (docs 17-19) n'était **NI** dans `selectFunc`, **NI** dans `StructVector::copyFromVectorData`, **NI** dans UNWIND. Les données étaient corrompues **avant même que UNWIND ne copie quoi que ce soit**.

**Root cause** : `CypherValue::Map(HashMap<String, CypherValue>)` — chaque `HashMap::new()` en Rust utilise une graine aléatoire différente (`RandomState`). L'ordre d'itération des clés varie entre instances du même programme. Quand on construit une `LIST<STRUCT>` à partir de plusieurs Maps :
1. Le type STRUCT est inféré du **premier** élément (ex: `STRUCT(id: STRING, emb: DOUBLE[])`)
2. Les éléments suivants peuvent avoir un ordre différent (ex: `STRUCT(emb: DOUBLE[], id: STRING)`)
3. `copyFromValue` pour STRUCT copie le i-ème child de la Value dans le i-ème child vector — sans vérifier les noms
4. → la valeur `emb` (LIST<DOUBLE>) est copiée dans le child vector STRING → `value.strVal` est vide → `""` silencieusement

**Fix** : `HashMap` → `BTreeMap` dans `CypherValue::Map`. Ordre alphabétique garanti, problème éliminé structurellement.

## Preuve directe

Debug ajouté dans `Unwind::copyTuplesToOutVector` — dump des STRING values dans le list data vector SOURCE :

```
[UNWIND] copyTuplesToOutVector startPos=0 endPos=3
  SRC field[0] type=STRING
    SRC[0] str="aaa" ✅
    SRC[1] str=""     ❌  (devrait être "bbb" — déjà corrompu AVANT la copie !)
    SRC[2] str="ccc" ✅
```

Chaque query crée un `make_items_param()` frais. Les pointeurs des vecteurs sont tous différents entre queries. La corruption est dans la matérialisation du paramètre.

Avec le fix BTreeMap :
```
  SRC[0] str="aaa" ✅
  SRC[1] str="bbb" ✅
  SRC[2] str="ccc" ✅
```

## Chemin de la corruption

```
Rust: CypherValue::List(vec![
    CypherValue::Map(HashMap { "id" → "aaa", "emb" → [1.0] }),  ← HashMap itère: [id, emb]
    CypherValue::Map(HashMap { "id" → "bbb", "emb" → [2.0] }),  ← HashMap itère: [emb, id] !!
    CypherValue::Map(HashMap { "id" → "ccc", "emb" → [3.0] }),  ← HashMap itère: [id, emb]
])

→ cypher_to_rag3db_value:
  item 0 → Value::Struct([("id","aaa"), ("emb",[1.0])])    ← type inféré: STRUCT(id, emb)
  item 1 → Value::Struct([("emb",[2.0]), ("id","bbb")])    ← ORDRE INVERSÉ !
  item 2 → Value::Struct([("id","ccc"), ("emb",[3.0])])

→ Value::List(elem_type=STRUCT(id:STRING, emb:DOUBLE[]), items)

→ copyFromValue pour LIST<STRUCT>:
  data_vector type = STRUCT(id:STRING, emb:DOUBLE[])
  child[0] = STRING vector (id)
  child[1] = DOUBLE[] vector (emb)

  item 0: child[0].copy(pos=0, value.children[0]="aaa")  ✅  STRING ← STRING
  item 1: child[0].copy(pos=1, value.children[0]=[2.0])  ❌  STRING ← LIST<DOUBLE> !
           → value.strVal = "" (vide pour un non-STRING) → position 1 = ""
  item 2: child[0].copy(pos=2, value.children[0]="ccc")  ✅  STRING ← STRING
```

## Fix appliqué

### `extension/rag3weaver/src/connection.rs`
```rust
// AVANT:
Map(HashMap<String, CypherValue>)

// APRÈS:
Map(BTreeMap<String, CypherValue>)
```

### Propagation dans tout le crate rag3weaver
Tous les fichiers qui construisent ou destructurent `CypherValue::Map` ont été mis à jour :
- `connection.rs` — définition enum + test
- `rag3db_connection.rs` — conversion rag3db Value ↔ CypherValue
- `catalog.rs` — create/link/update/get/search
- `ops.rs` — InsertOp/LinkOp/ChunkOp data fields
- `search.rs` — SearchResult.data, GraphNode.data, GraphEdge.properties
- `wasm_ffi.rs` — convert_node/rel/struct/map
- `cypher_persistence.rs` — tests
- `queue.rs` — tests
- `tests/e2e_search.rs` — tous les helpers de test
- `tests/e2e_native.rs` — helpers de test

Les `HashMap` internes (caches, configs, indexes) n'ont **pas** été changés — seuls ceux liés à `CypherValue::Map` utilisent `BTreeMap`.

## Debug C++ retiré

Tous les fprintf debug ajoutés pendant l'investigation (docs 17-19) ont été retirés :
- `src/expression_evaluator/function_evaluator.cpp`
- `src/processor/operator/unwind.cpp`
- `src/processor/operator/filter.cpp`
- `src/processor/operator/flatten.cpp`
- `src/processor/operator/cross_product.cpp`
- `src/storage/table/node_table.cpp`
- `extension/vector/src/index/hnsw_index.cpp`
- `extension/rag3weaver/src/catalog.rs` (eprintln debug)

## Vérification

```bash
cd packages/rag3db/extension/rag3weaver
bash run_e2e.sh debug_unwind
# test debug_unwind_match_set ... ok
# Diag 2: 3 rows (UNWIND + MATCH → tous trouvés)
# Diag 3: 3 nœuds avec dim=4 (UNWIND + MATCH + SET → tous updatés)
# Diag 5: 4 nœuds avec dim=4, 0 nulls
```

## Leçons

1. **Ne jamais utiliser HashMap pour des données qui doivent avoir un ordre déterministe** — surtout quand elles passent à travers une API typée par position (comme les STRUCT fields de Kuzu)
2. **`copyFromValue` pour STRUCT ne vérifie PAS les noms de champs** — il copie par index, donc l'appelant DOIT garantir l'ordre
3. **La corruption était silencieuse** — pas de crash, pas d'erreur, juste des strings vides et des matches manqués
4. **Le debug dans l'opérateur intermédiaire (UNWIND) a été la clé** — en montrant que la SOURCE était déjà corrompue, on a pu remonter au paramètre
