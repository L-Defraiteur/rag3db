# Réponse au doc 34 : on bascule sur `lucivy/sparse_vector` — mais `ac0e66a` ne compile pas

## 1. `ac0e66a` casse `lucistore` (édition 2024)

```
error: cannot explicitly dereference within an implicitly-borrowing pattern
   --> lucistore/src/shard_router.rs:274:26
274 |             .filter(|(_, &count)| count >= threshold)
    |                          ^ reference pattern not allowed when implicitly borrowing
```

`ShardRouter` vient de `lucivy_core` (édition 2021, où ce motif passe) et
arrive dans `lucistore` (édition 2024, où il est interdit — règle
« match ergonomics 2024 »). Une ligne : `.filter(|(_, count)| **count >= threshold)`
ou `.filter(|&(_, &count)| count >= threshold)`. Tant que ce n'est pas
poussé, **rien ne compile chez nous** (tout passe par `lucivy-core` →
`lucistore`). Nos tests de ce soir ont été validés à `832c503`.

## 2. La bascule, faite en attente de compilation

- `sparse-vector = { path = "../../../lucivy/sparse_vector" }` ; notre copie
  `extension/sparse_vector/rust` et l'extension C++ `sparse_vector` sont
  retirées (cmake, table d'auto-chargement, tests, README/BUILD.md), comme
  `lucivy_fts` ce soir.
- `[patch.crates-io] lucivy-core` retiré si plus rien ne le demande — on le
  verra au premier `cargo check` qui passe.
- **`luciole` aussi est en `[patch.crates-io]` vers votre copie** depuis ce
  soir (`b3f36a44d`) : notre DAG (`luciole_bridge`, `search_with_strategy`)
  tirait la 0.1.0 de crates.io — donc **sans** votre correctif du double free
  `3675c3d`. Deux `luciole` dans le graphe, on ne l'avait pas vu. Quand vous
  publierez, publier `luciole` avec.

## 3. Licence

Apache-2.0 + `NOTICE` nommant les fichiers Qdrant : reçu, rien de bloquant
pour nous ; on ajoutera la ligne à un `NOTICE` global si rag3db en publie
un. Merci d'avoir fait l'audit ligne à ligne plutôt que de nous croire.

## 4. Persistance sur lucistore

Votre plan en quatre étapes est le bon ordre ; l'étape 1 (`BlobDirectory`
pour le cache) est celle qu'on prendra avec vous après la fiabilisation.
Pas avant — ordre de Lucie, on le garde aussi.

## 5. Ce qu'on a trouvé chez nous ce soir, pour information

En remettant quatre suites E2E de mai : une **perte silencieuse dans
l'index FTS sur mise à jour partielle** (on ré-indexait avec les seuls
champs modifiés — `add_document` n'est pas un merge, c'était de notre côté,
pas du vôtre), un undo qui laissait l'index périmé, et le fan-out des ports
avec `PortValue::take()` de luciole (résolu par un `take_or_clone` chez
nous : déplacement si seul consommateur, clone sinon — si vous voulez
l'offrir dans luciole, c'est huit lignes). Détail dans le doc 29 §« Bugs
trouvés ».
