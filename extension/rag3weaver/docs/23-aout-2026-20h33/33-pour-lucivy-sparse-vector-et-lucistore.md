# Pour la session lucivy : `sparse_vector` est le jumeau de `lucivy_fts` — à unifier sur lucistore ?

Pas une demande de travail immédiat (l'ordre de Lucie : fiabilisation d'abord).
Un état des lieux pour que vous ayez la carte sous les yeux quand vous
toucherez à `lucistore`.

## 1. Ce qui vient de se passer côté FTS

L'extension C++ `lucivy_fts` (v2, cxx bridge, `CREATE_LUCIVY_INDEX` /
`QUERY_LUCIVY_INDEX` / `SEARCH`) est **supprimée** de rag3db ce soir. Depuis le
24 août, rag3weaver indexe et cherche en Rust, dans son processus, via
`lucivy-core` v3 (`ShardedHandle` sur `BlobShardStorage`) — la couche C++
n'était plus appelée par personne, et le submodule `extension/lucivy/ld-lucivy`
ne servait plus qu'à la construire.

## 2. `sparse_vector` a exactement la même anatomie

| | `lucivy_fts` (mort) | `sparse_vector` (vivant, même forme) |
|---|---|---|
| Extension C++ rag3db | `CREATE_LUCIVY_INDEX`, `SEARCH`… | `CREATE_SPARSE_VECTOR_INDEX`, `SPARSE_SEARCH`, `SPARSE_SCORE` |
| Appelée par rag3weaver ? | non | **non** — seuls ses propres tests C++ l'appellent |
| Chemin réel de rag3weaver | `lucivy_core::ShardedHandle` | `sparse_vector::handle::SparseHandle::{open,create}_with_store` (`catalog.rs:281`) |
| Crate Rust | `lucivy_core` (chez vous) | `extension/sparse_vector/rust` (ici, 2 695 lignes, inspiré de l'index sparse Qdrant : postings + WAND, remapping de dimensions, mmap) |
| Persistance | `BlobShardStorage` / lucistore | **son propre mécanisme** : 3 blobs par index (`sparse.mmap`, `sparse_vectors.bin`, `sparse_dims.bin`) écrits dans le `BlobStore` au `commit`, un tmpdir local comme cache mmap, nettoyé au `Drop` |

Le crate dépend de `lucivy-core = "2.0.0"` (crates.io) **uniquement** pour
ré-exporter le trait `BlobStore` (`blob_store.rs`, 5 lignes). Comme vous avez
depuis déplacé ce trait dans `lucistore` (`lucivy_core::blob_store` n'est plus
qu'un `pub use lucistore::blob_store::*`), rag3weaver porte un
`[patch.crates-io] lucivy-core = { path = … }` pour qu'il n'y ait qu'un seul
trait `BlobStore` dans le graphe — sinon `CypherBlobStore` n'implémenterait pas
celui que sparse attend. Ce patch tombe dès que `lucistore` et `lucivy-core`
2.1.0 sont publiés, à condition que `sparse-vector` dépende alors de
`lucistore` directement.

## 3. Les questions, pour quand vous y serez

1. **Un seul mécanisme de persistance ?** Le sparse réinvente en petit ce que
   lucistore fait en grand : cache local jetable, matérialisation mmap, commit
   vers le store. Il n'a ni shards, ni delta, ni snapshot, ni routage
   `.managed.json` ; et il a probablement le même `fsync` sur cache jetable
   que vous avez retiré du FTS (`9a66fbf`) — nous n'avons jamais profilé son
   commit. Si `shard_storage` / `blob_cache` de lucistore sont assez
   génériques pour porter un index qui n'est pas un segment tantivy, le
   sparse y gagnerait tout d'un coup (et nous, une seule politique de commit,
   un seul cache, un seul lieu où chercher les bugs).
2. **Où vit le crate ?** « Shared persistence, sync and shard infrastructure
   for lucivy *and friends* » — le sparse est un candidat évident au rang de
   *friend*. Chez vous (crate frère, publié avec le reste) ou ici (rag3db,
   dépendance sur lucistore) : les deux marchent, mais la première évite un
   deuxième `[patch]` à chaque évolution du trait.
3. **L'extension C++ `sparse_vector`** subira le sort de `lucivy_fts` (code
   mort pour rag3weaver) — sauf si vous voyez une raison de garder
   `SPARSE_SEARCH` en Cypher natif. Dites-le avant qu'on la retire.

## 4. Pointeurs

- `extension/sparse_vector/rust/src/handle.rs` — `create_with_store` l.114,
  `open_with_store` l.145, `commit_inner` l.378 (noms des blobs l.23-27).
- `extension/rag3weaver/src/catalog.rs:281` — l'unique point d'entrée côté
  rag3weaver ; `src/sparse_index.rs` — le type `SparseVector` partagé avec les
  embedders (BGE-M3 sparse appris, burn).
- `extension/rag3weaver/Cargo.toml` § `[patch.crates-io]` — le palliatif à
  retirer.
