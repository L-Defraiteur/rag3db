# Réponse au doc 27 : le double free est fermé (valgrind : 0), le `Reply` lâché est nommé, et il reste un underflow dans la recherche de shard

`3c282c7` épinglé, tout rejoué.

## 1. Valgrind contre votre correctif : 0 erreur, 0 `free` invalide

Même repro que le doc 26 (`phase0`+`phase1`, `RAG3W_NO_BATCH_SAVE=1`,
`RAG3DB_MAX_DB_SIZE=1 GiB`) :

```
ERROR SUMMARY: 0 errors from 0 contexts     (était : 158 errors, 23 contexts, 2 Invalid free)
```

Le double free est mort. Merci pour la vitesse.

## 2. Le `Reply` lâché — le dernier maillon que vous demandiez

Avec `LUCIOLE_REPLY_TRACE=1`, **une** occurrence par run, toujours la même :

```
[luciole] WARNING: Reply dropped without send() — the waiter gets 'actor died without replying'
   0: <luciole::reply::Reply<()> as Drop>::drop              reply.rs:185
   2: drop_glue<luciole::pool::DrainMsg>
   3: drop_glue<lucivy_core::sharded_handle::ShardMsg>
   4: drop_glue<flume::SendError<ShardMsg>>
   6: <luciole::pool::Pool<ShardMsg>>::drain                  pool.rs:149
   7: <ShardedHandle>::drain_pipeline                          sharded_handle.rs:1823
   8: <ShardedHandle>::close                                   sharded_handle.rs:2174
   9: rag3weaver::catalog::Catalog::close_fts_handles
  10: <Catalog as Drop>::drop
```

Ce n'est ni une recherche après `close()`, ni un `close()` concurrent d'une
recherche. C'est **`close()` lui-même** : `Pool::drain` fait

```rust
let _ = worker.send(DrainMsg(tx).into());   // pool.rs:149
…
scheduler.wait(rx, label);                  // panique si tx a été lâché
```

Quand un worker est déjà parti, `send` échoue, le `SendError` emporte `tx`,
`tx` est lâché — et `scheduler.wait(rx)` panique « actor died without replying
(drain_shards) ». Le correctif est local : ignorer le récepteur dont l'envoi a
échoué (ou `try_wait`). Un `close()` ne devrait jamais paniquer ; c'est un
destructeur chez tous ses appelants.

## 3. Ce que ça faisait chez nous, et ce qu'on a blindé

Sous valgrind, l'enchaînement complet était : la recherche échoue proprement
(`Err`, votre correctif n° 2 marche), le test panique sur son `unwrap`, et
pendant le déroulement de pile **notre `Drop` appelle `close()` qui panique à
son tour** → « panic in a destructor during cleanup » → abort du processus.

`Catalog::close_fts_handles` passe maintenant `close()` et `commit()` sous
`catch_unwind` : une panique devient une entrée d'échec rapportée. Un
destructeur ne doit jamais laisser passer une panique, quelle que soit sa
provenance. Résultat : abort non déterministe → **12/13 déterministe, 3 runs
sur 3**, avec un message.

## 4. Le message : un underflow dans la recherche de shard

```
search DAG: node 'search_2' failed: attempt to subtract with overflow
```

C'est le vrai coupable de toute la chaîne : le nœud `search_2` (shard 2 sur
4) **panique** sur une soustraction `usize`, l'acteur meurt (d'où les « actor
died » qui suivent), et avant `3675c3d` cette panique traversait `ptr::read` et
devenait le double free.

Deux observations qui en font une **course**, pas un cas limite arithmétique :

- Le même test **seul** passe. Il ne tombe qu'en 12ᵉ position dans le
  processus (11 cycles create/search/close avant lui).
- La suite complète à 38 tests, **sans** `RAG3W_NO_BATCH_SAVE` : 38/38 au
  premier essai, **37/38 au second** (même test). Avec (un `MERGE` par blob au
  lieu d'un `UNWIND` — ça ne touche que le timing), le 12ᵉ test échoue 3 fois
  sur 3. Le timing lent le rend certain ; le timing rapide le rend rare, pas
  absent.

Une soustraction qui déborde selon le timing, c'est deux compteurs lus à des
instants différents — `a - b` avec `b > a` parce que `b` a avancé entre les
deux lectures. Dans un nœud de recherche de shard, je regarderais du côté de
ce qui compte des documents ou des segments pendant qu'un autre acteur
(merge ? flush de close du handle précédent ?) les modifie.

Localisation exacte (gdb, point d'arrêt sur `rust_panic`, première panique du
processus) :

```
#0  core::panicking::panic_const_sub_overflow
#1  ld_lucivy::query::union::buffered_union::refill::{closure#0}     buffered_union.rs:72
#2  ld_lucivy::query::union::buffered_union::unordered_drain_filter   buffered_union.rs:23
#3  ld_lucivy::query::union::buffered_union::refill                   buffered_union.rs:64
#4  BufferedUnionScorer::refill                                       buffered_union.rs:119
#5  BufferedUnionScorer::build                                        buffered_union.rs:104
#6  boolean_weight::scorer_union<SumCombiner>                         boolean_weight.rs:78
#7  BooleanWeight::complex_scorer
#8  BooleanWeight::scorer
#9  BooleanWeight::per_occur_scorers
#10 BooleanWeight::complex_scorer
#11 BooleanWeight::for_each_pruning
#12 collector::sort_key::sort_by_score::collect_segment_top_k         sort_by_score.rs:41
```

`buffered_union.rs:58-72`, dans `refill` :

```rust
let horizon = min_doc + HORIZON;
…
    let doc = scorer.doc();          // l. 67
    …
    let delta = doc - min_doc;       // l. 72  ← déborde : doc < min_doc
```

Un scorer enfant de l'union rend un `doc()` **inférieur au `min_doc`** calculé
juste avant sur ces mêmes scorers. Dans le modèle tantivy c'est impossible par
construction : `refill` reçoit le minimum courant des `doc()` de ses enfants,
et un `DocSet` est monotone. Pour que ce soit faux, il faut que le `doc()` d'un
enfant ait **changé entre le calcul du minimum et la lecture** — ou qu'il rende
une valeur qui ne dépend pas de son propre curseur.

C'est là que le timing entre en jeu : les quatre `SearchShardNode` d'un niveau
tournent **en parallèle** sur les threads du scheduler. Un état mutable partagé
entre ces scorers — cache de prescan SFX v3 par segment, lecteur `.posmap` /
`.bytemap` lazy, curseur de résolution par offsets, le `HighlightSink` en `Arc`
qu'on passe à toutes les shards — suffirait. La requête est un `should` de deux
`contains` (`ContainsSplit` « Rust safety »), donc deux scorers `contains` v3
sous un `BufferedUnionScorer`, dans `complex_scorer` (`boolean_weight.rs:78`).

Ce n'est pas un cas limite de données : le même index, la même requête, seul,
passe. Je regarderais ce que le scorer `contains` v3 partage entre instances,
et je poserais un `debug_assert!(doc >= min_doc)` juste avant la ligne 72
pour attraper l'enfant fautif avec son nom.

## 5. Reproduire

```bash
cd extension/rag3weaver
RAG3W_NO_BATCH_SAVE=1 cargo test --features rag3db-native --test e2e_search \
  -- --ignored --test-threads=1
# attendu: 12 passed; 1 failed — phase1_bm25_split_distant_words,
#          "node 'search_2' failed: attempt to subtract with overflow"
```

Déterministe chez nous, 3 runs sur 3, ~3 s. Pour la pile :

```bash
BIN=$(ls -t target/debug/deps/e2e_search-* | grep -v '\.d$' | head -1)
RAG3W_NO_BATCH_SAVE=1 gdb -q -batch -ex "break rust_panic" -ex run -ex "bt 30" \
  --args "$BIN" --ignored --test-threads=1
```

## 6. Sur `MALLOC_CHECK_`

Votre §4 est retenu tel quel : valgrind d'abord, `MALLOC_CHECK_` jamais comme
preuve d'absence. Le doc 25 portait cette erreur, l'erratum y est.
