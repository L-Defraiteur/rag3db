# valgrind a parlé : double free du `Box<dyn Node>` pris par `ptr::read` dans `execute_level_parallel`

Suite du doc 25. Valgrind installé, kuzu convaincu de réserver 1 GiB au lieu de
8 TiB (`RAG3DB_MAX_DB_SIZE`, voir plus bas), et la suite native rejouée dans la
configuration qui plantait. **158 erreurs, 23 contextes, 2 `free` invalides.**
Le premier contexte suffit, et il n'est pas ambigu.

## 1. Le bloc, ses deux libérations, son allocation

Bloc de **56 octets** — un `Box<SearchShardNode>` :

```
Block was alloc'd at
   Box::new<lucivy_core::search_dag::SearchShardNode>
   luciole::dag::Dag::add_node                       dag.rs:77
   lucivy_core::search_dag::build_search_dag         search_dag.rs:640
   ShardedHandle::search_internal                    sharded_handle.rs:1855
```

**Première libération** — sur un thread du scheduler, dans la closure de la
tâche parallèle :

```
free
   <Box<dyn luciole::node::Node> as Drop>::drop      boxed.rs:2000
   luciole::runtime::execute_level_parallel::{closure#1}   runtime.rs:819
   luciole::scheduler::Scheduler::submit_task
   luciole::scheduler::run_loop                       scheduler.rs:1038
```

**Seconde libération** (lecture invalide « 8 bytes inside a block of size 56
free'd », puis `Invalid free`) — sur le thread appelant, quand le `Dag` sort de
portée :

```
   drop_glue<SearchShardNode> → drop_glue<Box<dyn Node>> → drop_glue<DagNodeEntry>
   → drop_glue<Vec<DagNodeEntry>> → drop_glue<luciole::dag::Dag>
   ShardedHandle::search_internal                    sharded_handle.rs:1871
```

Les 21 autres contextes sont les mêmes octets relus par les destructeurs en
cascade (`Pool<ShardMsg>` → `Vec<ActorRef>` → `flume::Sender` →
`Arc<WakeHandle>`). Le second `Invalid free` est le tampon du `Vec<ActorRef>`
du pool de ce même nœud, lui aussi libéré deux fois par les deux mêmes chemins.

## 2. Pourquoi : `ptr::read` laisse une copie du `Box` dans le DAG

`luciole/src/runtime.rs`, `execute_level_parallel`, vers la ligne 788 :

```rust
let entry = &mut dag.nodes_mut()[node_idx];
let node_box = unsafe {
    let ptr = &mut entry.node as *mut Box<dyn crate::node::Node>;
    std::ptr::read(ptr)     // le slot du DAG garde les mêmes octets
};
```

Le `Box` est déplacé bit à bit vers la tâche **sans vider le slot**. Le
protocole — vous le dites vous-mêmes en commentaire ligne ~815 — est que le
Box *revient toujours* dans le résultat (`Ok((…, node_box))` /
`Err((…, node_box))`) pour être réécrit dans le slot.

Ce protocole tient tant que `node_box.execute(&mut ctx)` **retourne**. S'il
**panique**, le déroulement de pile détruit `node_box` dans la closure (c'est
la première libération, attribuée à la ligne 819, fin de la closure), le slot
du DAG garde sa copie, et le `Dag` la libère à son tour. Toute panique dans un
nœud d'un niveau parallèle est une corruption du tas.

## 3. Le panic qui déclenche tout : `actor died without replying`

Le même run affiche, sur le thread du test, **deux fois** :

```
thread 'phase1_bm25_split_distant_words' panicked at luciole/src/reply.rs:236:17:
actor died without replying
```

C'est `wait_blocking_with_diag` :

```rust
if state.closed {
    panic!("actor died without replying");
}
```

— un `Reply` lâché sans `send`. C'est exactement le trou du doc 19 §3 (là,
côté pipe ; ici, côté attente bloquante). Je ne sais pas *quel* acteur lâche sa
`Reply` ni pourquoi ; c'est chez vous que ça se voit. Mais l'enchaînement est
complet : **un acteur meurt sans répondre → un nœud panique dans `execute` →
`ptr::read` transforme ce panic en double free.**

Pourquoi c'est non déterministe et pourquoi `MALLOC_CHECK_=3` ne voyait rien :
glibc ne détecte un double free que si le chunk n'a pas été réalloué entre
temps ; selon la disposition, la seconde libération tombe sur un chunk réutilisé
(silencieuse), sur un chunk encore libre (abort), ou lit des octets d'un autre
objet (SIGSEGV). Valgrind, lui, suit l'identité du bloc.

## 4. Ce qu'on demande

Deux correctifs, indépendants, dans cet ordre :

1. **Rendre `execute_level_parallel` sain sans dépendre du retour du Box.**
   Le slot doit être *réellement* vide pendant l'exécution :
   `entry.node: Option<Box<dyn Node>>` + `.take()`, ou `mem::replace` avec un
   nœud sentinelle — plus de `ptr::read`. Et un garde (`Drop` ou
   `catch_unwind`) qui réécrit le Box même si `execute` panique. Une panique de
   nœud doit remonter comme une erreur de DAG, jamais comme une corruption.
2. **Trouver l'acteur qui lâche sa `Reply`** dans ce scénario (deux shards,
   11 cycles create/search/close dans le même processus, puis une recherche
   `ContainsSplit` « Rust safety »). L'avertissement stderr ajouté par
   `e6176f5` devrait maintenant le nommer.

## 5. Reproduire chez vous, exactement

```bash
# build natif + env habituel, puis :
cd extension/rag3weaver
cargo test --features rag3db-native --test e2e_search --no-run
BIN=$(ls -t target/debug/deps/e2e_search-* | grep -v '\.d$' | head -1)
RAG3W_NO_BATCH_SAVE=1 RAG3DB_MAX_DB_SIZE=$((1<<30)) RAG3DB_BUFFER_POOL_SIZE=$((256<<20)) \
valgrind --tool=memcheck --error-limit=no --num-callers=24 --log-file=valgrind.log \
  "$BIN" --ignored --test-threads=1 phase0 phase1
```

- `RAG3DB_MAX_DB_SIZE` : kuzu réserve 8 TiB d'espace virtuel par défaut
  (`database.cpp:80`, sentinel `-1u`… que le défaut Rust `u32::MAX` déclenche,
  soit dit en passant), valgrind refuse. 1 GiB, puissance de deux, suffit.
- `RAG3W_NO_BATCH_SAVE=1` ne fait que changer le timing (un `MERGE` par blob) ;
  ça rend le déclenchement quasi certain sur cette suite.
- 13 tests, aucun modèle : ~4 minutes sous valgrind.

Le journal complet (158 erreurs) est disponible si vous le voulez ; tout ce qui
est cité ici en vient mot pour mot.
