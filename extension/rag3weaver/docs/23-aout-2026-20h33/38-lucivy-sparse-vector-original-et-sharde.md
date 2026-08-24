# `sparse-vector` : réécrit, MIT, shardé — ce qui change pour rag3weaver

Suite du doc 34. Depuis, le crate a été réécrit et étendu ; voici l'état à
épingler et ce qu'il faut savoir en le consommant.

## 1. Le code dérivé de Qdrant n'existe plus

Le cœur (listes de postings, WAND, top-k) a été **réécrit sur spécification**
dans `sparse_vector/src/wand/` — sans ouvrir les fichiers dérivés — puis
branché à la place ; `posting_list.rs`, `posting_list_common.rs`,
`search_context.rs`, `scores_memory_pool.rs`, `top_k.rs` sont supprimés.
Audit ligne à ligne contre l'arbre Qdrant, sur tout le crate : 0-10 % de
lignes communes, toutes triviales. Le crate est donc **MIT** (comme le reste
du workspace), avec un `NOTICE` « design inspiré de Qdrant, aucun code
dérivé ». Le doc 34 §2 (Apache-2.0) est caduc.

Preuves de justesse : égalité avec un produit scalaire brut sur 200
requêtes × 12 configurations (ids et scores), élagage activé et désactivé,
poids négatifs, filtres, aller-retour mmap, invariant des plafonds sous
2 000 mutations aléatoires. 61 tests.

## 2. Perf : mieux qu'avant, et l'insertion n'est plus quadratique

50 000 records, 2 000 dimensions, 30 non-nuls, popularité en Zipf, médiane
par requête top-10 :

| | avant | après |
|---|---|---|
| recherche RAM | 147 µs | **137 µs** |
| recherche mmap | 154 µs | **127 µs** |
| insertion de 50 000 vecteurs | 3,2 s | **139 ms** |

L'insertion réécrivait tout le préfixe d'une liste à chaque ajout ; elle ne
répare plus que ce qui change (O(1) amorti en ordre croissant d'ids).

## 3. API : `SparseHandle` inchangé, trois précisions

`create` / `open` / `create_with_store` / `open_with_store` / `insert` /
`remove` / `search` / `search_filtered` / `len` / `commit_inner` : mêmes
signatures. Trois comportements à connaître :

- **Poids nuls** : une dimension de poids `0.0` n'est plus stockée dans les
  postings (elle ne contribue à aucun score). Un vecteur dont tous les poids
  sont nuls est donc introuvable par la recherche — c'était déjà le cas par
  lots avant, mais pas sur le chemin mono-liste ; c'est maintenant uniforme.
- **Dimensions dupliquées dans une requête** : fusionnées (poids sommés)
  avant la recherche. Avant, les deux chemins comptaient deux fois.
- **Ordre des résultats** : score décroissant puis id croissant, déterministe.

## 4. Nouveau : `ShardedSparseHandle` (`sparse_vector::sharded`)

Le pendant sparse du `ShardedHandle` FTS, sur la même infrastructure
(`lucistore::ShardRouter`, `luciole::Pool`, storages lucistore) :

```rust
use sparse_vector::sharded::{ShardedSparseHandle, ShardedSparseConfig};

let cfg = ShardedSparseConfig::new(4);            // shards ; balance_weight 1.0 par défaut
let h = ShardedSparseHandle::create_with_store(store.clone(), "vectors", &cache_base, &cfg)?;
h.insert(node_id, &vector)?;                       // routé par dimensions, fire-and-forget
h.commit()?;                                       // remonte la première erreur d'insert par shard
let hits = h.search(&query, 10)?;                  // scatter + fusion k-way
let hits = h.search_filtered(&query, 10, &allowed_ids)?;
h.remove(node_id)?;                                // routé par node_id (broadcast si inconnu)
h.close()?;                                        // commit, acteurs arrêtés, handle inerte
// ou : h.drop_index()?  — close puis destruction du storage
let h = ShardedSparseHandle::open_with_store(store, "vectors", &cache_base)?;
```

Blobs : `Sparse_{name}/shard_{i}` par shard, fichiers racine
(`_sparse_config.json`, `_sparse_router.bin`) sous `Sparse_{name}` ; votre
`CypherBlobStore` convient tel quel. Un `node_id` réinséré reste sur son
shard (upsert, pas doublon). Après `close()`, tout appel rend
`"handle is closed"`. Pas de statistiques globales à échanger pour le
distribué : un produit scalaire est local, la fusion se fait sur des
`(id, score)`.

Ce qui n'y est pas encore : deltas LUCIDS (un shard modifié repart entier),
partage du seuil top-k entre shards, plafonds par bloc pour que l'élagage
morde sur les très longues listes (`docs/24-08-2026/04-sparse-sharding-design.md`).

## 5. Chez vous

- Épingler le HEAD de `v3-recovery` (`git log -1 -- sparse_vector`).
- `sparse-vector = { path = "../../../lucivy/sparse_vector" }` ; supprimer
  l'ancienne copie `extension/sparse_vector/rust` et l'extension C++.
- Le `[patch.crates-io] lucivy-core` n'a plus de raison d'être côté sparse.
- Si vous passez au shardé : `catalog.rs:281` est l'unique point d'entrée ;
  `SparseHandle` reste disponible pour le mono-shard.
