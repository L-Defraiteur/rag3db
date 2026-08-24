# Réponse au doc 33 : `sparse-vector` vit maintenant dans le workspace lucivy, sous sa propre licence

Vos trois questions, dans l'ordre — et une chose que vous n'aviez pas
demandée mais qu'il fallait faire.

## 1. Où vit le crate : chez nous, `lucivy/sparse_vector/`

Copié depuis `extension/sparse_vector/rust` (votre `6392c29a1`), **sans le
pont C++** (`bridge.rs`, `build.rs`, `cxx`, `staticlib`) : plus de client.
La seule dépendance à `lucivy-core` était le ré-export de `BlobStore` ; le
crate dépend maintenant de `lucistore` par chemin. Commit `e344b93` sur `v3-recovery`. 34 tests verts.

Chez vous, tout de suite, pour ne pas vivre avec deux copies :

```toml
sparse-vector = { path = "../../../lucivy/sparse_vector" }
```

et le `[patch.crates-io] lucivy-core = …` peut tomber dès que plus rien
d'autre ne l'exige — il n'y a plus qu'un `BlobStore`, celui de `lucistore`.
Votre `CypherBlobStore` l'implémente déjà. Le code rag3db de
`extension/sparse_vector/rust` devient l'ancienne copie ; à supprimer avec
l'extension C++ (§3).

## 2. La licence — le point que Lucie a soulevé

Le crate se déclarait **MIT**. L'audit (comparaison ligne à ligne avec le
dépôt Qdrant) dit autre chose : `posting_list.rs` est à 77 % verbatim de
`lib/sparse/src/index/posting_list.rs`, `search_context.rs` à 59 %,
`posting_list_common.rs` à 53 % — tous trois portaient déjà l'en-tête
Qdrant. `handle.rs`, `index.rs`, `mmap_index.rs`, `top_k.rs` sont originaux
(2-12 %, le mmap compris). C'est donc une œuvre dérivée : le crate est
maintenant **Apache-2.0**, avec le texte de licence et un `NOTICE` qui
nomme les fichiers dérivés.

Ce que ça change pour vous : rien de bloquant. Apache-2.0 est compatible
avec un projet MIT dans les deux sens, à condition de conserver les
notices — c'est ce que le crate fait lui-même. Les crates MIT du workspace
(`ld-lucivy`, `lucivy-core`, `luciole`, `lucistore`) n'en dépendent pas ;
rag3weaver, qui en dépend, embarque une dépendance Apache-2.0 comme il en
embarque déjà des dizaines. Si rag3db publie un `NOTICE` global, y ajouter
la ligne Qdrant est propre ; ce n'est pas une obligation tant que le crate
porte la sienne.

## 3. Unifier la persistance sur lucistore : oui, et voilà le plan

Le sparse a la même anatomie que le FTS mais réinventée en petit :
tmpdir cache, trois fichiers, `store.save` au commit. Pas de fsync (c'est
`fs::write`, donc pas le piège de `9a66fbf`), mais pas de lazy, pas de
delta, pas de shards, et une fermeture qui `remove_dir_all` sans passer par
personne. Étapes, dans l'ordre où elles rapportent :

1. **`BlobDirectory` pour le cache** — le sparse écrit ses trois fichiers
   dans un `BlobDirectory` (celui du FTS, générique sur `BlobStore`) au
   lieu de son tmpdir : matérialisation Eager/Lazy, `load_range`, drop
   propre, `.managed.json` hors sujet (pas de segments tantivy). Gain
   immédiat : ouverture lazy et un seul cache à surveiller.
2. **`ShardStorage`** — `SparseHandle::{create,open}_with_storage(Box<dyn
   ShardStorage>)` comme `ShardedHandle`, donc `BlobShardStorage` et
   `FsShardStorage` sans code chez vous.
3. **Sharding** — N `SparseHandle` derrière un routeur, top-k fusionné ;
   le routage par `node_id` et `search_filtered(allowed_ids)` du FTS se
   transposent tels quels.
4. **Delta / snapshot** — LUCID/LUCIDS sur des fichiers, pas sur des
   segments : `lucistore::delta` y est presque, il faudra un exporteur qui
   n'assume pas `meta.json`.

Le point 1 est petit et sans risque ; 2 suit ; 3-4 se décident quand vous
en avez besoin. Rien de tout ça ne commence avant la fin de la
fiabilisation — c'est l'ordre de Lucie, on le garde.

## 4. L'extension C++ `sparse_vector`

Aucune raison de la garder de notre côté : `SPARSE_SEARCH` en Cypher natif
n'a pas de client, et le pont n'existe plus dans le crate. Retirez-la avec
l'ancienne copie du crate Rust.
