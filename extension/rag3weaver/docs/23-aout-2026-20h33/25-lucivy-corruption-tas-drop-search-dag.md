# Corruption du tas à la destruction du DAG de recherche — pile gdb, repro, et ce qu'on ne sait pas

Rapport depuis rag3weaver, contre `8f14edc` (`parse` booléen). Le contrat `parse`
tient (nos gardes 12/12, highlights sur les deux formes, refus remontés tels quels).
Mais en rejouant `e2e_search`, le processus **aborte ou segfaulte de façon non
déterministe**. La pile est chez vous, et elle est nette.

## La pile (gdb, thread fautif)

```
#0  pthread_kill / raise / abort                        glibc: "double free or corruption (out)"
#7  alloc::raw_vec::RawVec<ActorRef<ShardMsg>>::drop    ← libération du tampon du Vec
#10 drop_glue<luciole::pool::Pool<ShardMsg>>
#11 drop_glue<lucivy_core::search_dag::SearchShardNode>
#12 drop_glue<Box<dyn luciole::node::Node>>
#13 drop_glue<luciole::dag::DagNodeEntry>
#19 drop_glue<luciole::dag::Dag>
#20 ShardedHandle::search_internal                     sharded_handle.rs:1871  (fin de fonction, `dag` sort de portée)
#21 ShardedHandle::search                              sharded_handle.rs:1825
#22 rag3weaver::fts_handle::search_hits
#23 rag3weaver::search::search_bm25_chunked            mode ContainsSplit, "Rust safety"
```

Au moment du crash, **aucun des 71 autres threads** ne touche `Pool`, `ShardMsg`,
`SearchShardNode` ni `search_dag` (dump complet disponible). Ce n'est donc pas deux
threads qui libèrent ensemble.

`Pool::clone` clone bien le `Vec<ActorRef>` (`pool.rs:21`) et `SearchShardNode::new`
reçoit un `shard_pool.clone()` (`search_dag.rs:636`) — pas de copie bit à bit visible.

## Ce que l'allocateur dit, et qui change la lecture

- **Sous `MALLOC_CHECK_=3 MALLOC_PERTURB_=165`, la suite complète passe 38/38**
  (250 s). `MALLOC_CHECK_=3` **attrape un vrai double free à coup sûr**. Qu'il ne
  voie rien suggère plutôt une **écriture hors limites** ailleurs, qui corrompt les
  métadonnées du chunk voisin ; glibc s'en aperçoit au `free` suivant — ici celui
  du `Vec` du pool. **Le pool serait la victime, pas le coupable.** Sous
  `MALLOC_CHECK_`, la disposition des chunks change et l'écriture tombe ailleurs.
- Les deux signaux vus (`SIGABRT` « double free or corruption (out) », `SIGSEGV`)
  sont cohérents avec une corruption qui dépend de la disposition du tas.
- gdb : 1 crash sur 2 essais. Sans gdb : 2 sur 2 dans la configuration ci-dessous,
  0 sur 2 dans une configuration voisine — voir « Repro ».

## Repro

```bash
# build natif, variables d'env habituelles (RAG3DB_SHARED=1, LD_LIBRARY_PATH…)
cd extension/rag3weaver
RAG3W_NO_BATCH_SAVE=1 cargo test --features rag3db-native --test e2e_search \
  -- --ignored --test-threads=1
```

`RAG3W_NO_BATCH_SAVE=1` fait faire à notre `CypherBlobStore` un `MERGE` par blob
(~114 par drain) au lieu d'un seul `UNWIND` : ça ne touche pas lucivy, ça change le
**timing et la disposition du tas**, et ça rend le crash quasi déterministe (2/2
contre 0/2 sans). C'est un amplificateur, pas une cause — la version `UNWIND` a aussi
planté une fois sur trois runs.

Pour la pile :

```bash
BIN=$(ls -t target/debug/deps/e2e_search-* | grep -v '\.d$' | head -1)
RAG3W_NO_BATCH_SAVE=1 gdb -q -batch -ex run -ex bt -ex "thread apply all bt 12" \
  --args "$BIN" --ignored --test-threads=1
```

Le crash survient au 12ᵉ test (`phase1_bm25_split_distant_words`), après 11 créations
et destructions de `Catalog` dans le même processus — donc après ~11 cycles
create/open/search/close de `ShardedHandle`, 2 shards, `MemBlobStore` non : notre
`CypherBlobStore` derrière un tampon.

## Ce qu'on ne sait pas

- **Depuis quand.** À `36b1edd`, `e2e_search` a tourné 20/20 au moins quatre fois
  aujourd'hui (features par défaut, donc 13 de ces tests + candle) sans crash. À
  `8f14edc` : crash sur 3 runs sur 5 sans gdb. Mais c'est une corruption
  sensible à la disposition, et entre les deux nous avons aussi changé des choses
  (aucun embedder par défaut, imports gardés) — la corrélation n'est pas une
  preuve. J'ai voulu compiler contre `36b1edd` dans un worktree : collision de
  lockfile (`ld-lucivy` arrive par deux chemins). Dater proprement demande de
  déplacer votre arbre de travail, je ne le fais pas sans vous.
- **Où est l'écriture hors limites**, si c'en est une. valgrind n'est pas encore
  installé ici (miroirs pacman en 404, en cours). Dès qu'il l'est, je le passe sur
  le binaire natif — 13 tests, aucun modèle, ~10 s hors valgrind — et je vous
  envoie la première écriture invalide avec sa pile.

## Ce qu'on demande

La pile pointe sur la fin de `search_internal` : le `Dag` construit par
`build_search_dag` détruit ses `SearchShardNode`, chacun avec son clone de pool.
Si un chemin `unsafe` de luciole (mailbox, pool, `box_clone`) ou des lectures mmap
SFX v3 (`.posmap` / `.bytemap`, résolution par offsets) peut écrire un octet de
trop quelque part entre deux recherches, c'est là que je regarderais en premier.
Si vous avez ASan ou valgrind sous la main avant moi, ce test (`phase0`+`phase1`,
13 tests, sans modèle) est le plus court chemin.

Rien n'est perdu côté données — c'est un crash de processus à la recherche, pas une
corruption d'index. Mais c'est un SIGSEGV dans une lib Rust, et on a la même règle
que vous : ça ne devrait pas exister.
