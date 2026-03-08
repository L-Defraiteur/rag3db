# Doc 14 — Debug FTS batch indexing : findings

Date : 8 mars 2026
Réf : Doc 13 (debug FTS post-delete)

## Résumé

Le bug FTS n'est NI un problème de delete, NI un problème de lowercase dans `scoring_utils.rs`. C'est un **bug d'indexation batch** : seul le premier produit d'un batch UNWIND MERGE a son champ `description` indexé dans le FTS. Les champs `details` sont indexés correctement pour tous les produits.

## Correction du Doc 13

Le Doc 13 identifiait 2 bugs :
1. ~~FTS query format dans les diagnostics~~ → Faux, le vrai problème est l'indexation batch
2. **Deadlock dataflow** dans `rechunk_simple_entities()` → Toujours valide, fix appliqué (voir ci-dessous)

## Finding 1 : Le deadlock dataflow est fixé

Fix appliqué dans `record_nodes.rs` : `ChunkRecordNode` et `KBChunkRecordNode` set maintenant TOUJOURS leurs outputs `chunks` et `chunk_links`, même quand les vecteurs sont vides. Avant, les outputs n'étaient pas set si vides → scheduler deadlock.

```rust
// AVANT (deadlock si 0 chunks) :
if !all_chunk_entities.is_empty() {
    ctx.set_output("chunks", ...);
}

// APRÈS (toujours set, même vide) :
ctx.set_output("chunks", PortValue::Batch(
    BatchPayload::new(PortType::Entities, all_chunk_entities),
));
```

Fichier : `src/dataflow/record_nodes.rs`, 2 occurrences (KBChunkRecordNode + ChunkRecordNode).

## Finding 2 : Le lowercase multi-token dans ld-lucivy est un faux positif

L'agent dans ld-lucivy a trouvé que `generate_trigrams()` ne faisait pas `.to_lowercase()`. Un fix a été appliqué dans `scoring_utils.rs:204`. Mais :

- Le test unitaire Rust dans lucivy-core passe (le chemin `build_query → NgramContainsQuery` gère correctement le case)
- Après rebuild de l'extension C++ avec le fix, le problème persiste
- Le fix lowercase est un bon hardening mais ce n'est PAS la cause racine

## Finding 3 : Le VRAI bug — indexation FTS batch, seul le premier row est indexé pour `description`

### Preuve

Diagnostics raw `QUERY_LUCIVY_INDEX` AVANT tout delete, sur 3 produits ingérés en un seul batch :

| Query | Champ | Résultat | Attendu |
|-------|-------|----------|---------|
| description:'alpha' | description | 1 hit, id=0 | ✓ |
| description:'advanced' | description | 1 hit, id=0 | ✓ |
| description:'technology' | description | 1 hit, id=0 | ✓ |
| description:'beta' | description | **0 hits** | ✗ devrait trouver id=1 |
| description:'engineering' | description | **0 hits** | ✗ devrait trouver id=1 |
| description:'gamma' | description | **0 hits** | ✗ devrait trouver id=2 |
| description:'precision' | description | **0 hits** | ✗ devrait trouver id=2 |
| details:'alpha' | details | 1 hit, id=0 | ✓ |
| details:'beta' | details | 2 hits, ids=[1,2] | ✓ (fuzzy via "details") |
| details:'gamma' | details | 1 hit, id=2 | ✓ |

**Pattern clair** : le champ `description` n'est indexé que pour id=0 (premier produit du batch). Le champ `details` est indexé pour tous.

### Données de test

```
id=0  Alpha Widget  desc="Advanced alpha technology for computing"       det="Alpha details here"
id=1  Beta Gadget   desc="Beta engineering and manufacturing process"    det="Beta details here"
id=2  Gamma Tool    desc="Gamma precision instruments for research"      det="Gamma details here"
```

### Pourquoi les 10 tests BM25 existants passent

Ils ne cherchent que des mots du **premier** produit ingesté (ex: "programming language" dans la description du Rust Book = premier produit). Ou bien ils cherchent des mots communs qui apparaissent dans `details` de tous les produits. Le bug est invisible pour ces tests.

Le test `simple_multiple_ingestions` cherche "batch item" et attend ≥2 résultats. Il ingère en 2 batches (2+1). Seul le premier de chaque batch a `description` indexée → 2 résultats → test passe.

### Cause racine probable

Dans le hook C++ `LucivyIndex::insert()` (`lucivy_index.cpp:181-247`), le code itère sur les rows du batch :

```cpp
for (auto i = 0u; i < nodeIDVector.state->getSelSize(); i++) {
    auto pos = nodeIDVector.state->getSelVector()[i];
    // ...
    auto text = propertyVectors[f]->getValue<ku_string_t>(pos).getAsString();
}
```

**Hypothèse** : `propertyVectors[0]` (description) a un état (`state`, `selVector`, ou buffer overflow de `ku_string_t`) qui ne fonctionne correctement que pour `pos=0`. Pour `pos>0`, la valeur lue est vide ou null → skippée par le `if (propertyVectors[f]->isNull(pos)) continue;`.

`propertyVectors[1]` (details) fonctionne pour tous les pos. La différence entre les deux vecteurs pourrait être liée à :
1. L'ordre des colonnes dans le UNWIND MERGE et la façon dont Kuzu alloue les ValueVectors
2. La longueur des strings (description est plus long que details) → overflow buffer `ku_string_t`
3. Le state/selVector du DataChunk qui diffère entre propertyVectors[0] et [1]

### Piste : MERGE vs CREATE

Le `InsertRecordNode` utilise :
```sql
UNWIND $items AS item
MERGE (n:Product {_uuid: item._uuid})
SET n.description = item.description, n.details = item.details, ...
```

Possible que `MERGE` crée le node avec seulement `_uuid`, puis `SET` met à jour les autres colonnes. Le hook `insert()` se déclenche au CREATE avec `_uuid` seulement (description et details sont null). Puis le `SET` déclenche `update()`, pas `insert()`.

Mais si c'était le cas, même le premier produit n'aurait pas description indexée. Sauf si le premier row a un comportement spécial dans MERGE.

## Tests nettoyés

Le test `simple_delete_removes_chunks` a été nettoyé :
- Supprimé les diagnostics raw `QUERY_LUCIVY_INDEX` et le `return;` de la session précédente
- Restauré pour utiliser `catalog.delete()` API
- Ajouté des diagnostics per-field temporaires pour le debug (à nettoyer après fix)

## Fichiers modifiés

| Fichier | Modification |
|---------|-------------|
| `src/dataflow/record_nodes.rs` | Fix deadlock : ChunkRecordNode + KBChunkRecordNode toujours set outputs |
| `tests/e2e_simple_entity.rs` | Nettoyé diagnostics, ajouté per-field FTS debug |
| `extension/lucivy/ld-lucivy/src/query/phrase_query/scoring_utils.rs` | Fix lowercase dans `generate_trigrams()` (hardening, pas la cause racine) |

## Prochaines étapes

1. **Investiguer le hook `insert()` C++** — ajouter des `fprintf(stderr, ...)` dans `lucivy_index.cpp:189-200` pour voir les valeurs réelles lues pour chaque `(row, field)`. Vérifier si `propertyVectors[0]->isNull(pos)` retourne true pour pos>0.

2. **Tester avec INSERT au lieu de MERGE** — si `INSERT INTO Product ...` indexe correctement tous les champs, ça confirme que c'est spécifique à MERGE+SET.

3. **Tester avec 1 produit par batch** — `ingest_entities()` appelé 3 fois avec 1 produit → si description est indexée pour tous, ça confirme le bug batch.

4. **Fix potentiel** — dans `insert()`, si les propertyVectors sont vides pour pos>0, utiliser `update()` comme fallback pour re-indexer après SET. Ou bien modifier `InsertRecordNode` pour utiliser INSERT au lieu de MERGE.

5. **Après fix FTS** — relancer les 5 tests E2E CRUD et adapter les assertions si nécessaire.

## Tasks

```
#202 ✅ Fix ChunkRecordNode deadlock — always set outputs
#203 ✅ Clean up diagnostic code in simple_delete_removes_chunks test
#204 🔧 Run E2E tests — bloqué par bug FTS batch indexing
```
