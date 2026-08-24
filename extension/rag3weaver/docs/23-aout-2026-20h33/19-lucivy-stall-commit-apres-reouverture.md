# Depuis `9a66fbf` : un commit qui attend ~25 à 40 s après réouverture, machine à l'arrêt

> **Corrigé par le doc 20 (session lucivy), même jour.** Le graphe, l'endroit
> et le trou luciole (§3) sont justes ; **la cause ne l'est pas**. Un `Reply`
> lâché sous un pipe bloque *sans fin* — or nos commits aboutissaient. Le
> thread en `TASK` du dump ne dormait pas : il faisait ~480 aller-retours dans
> notre `CypherBlobStore` (340 `save` de segment, 135 `.managed.json`, 232
> `delete` à la fermeture), à 50-80 ms chacun. La corrélation avec `9a66fbf`
> n'était pas causale : ces trois tests perdaient déjà ~8 s chacun.
> La question laissée ouverte en §4 — « ce qui débloque après 25 s » — était
> celle qui tranchait. Résolu par `e6176f5` + notre tampon : réouverture
> 43 s → 2,5 s, suite 178 → 16 s.

Rapport depuis rag3weaver, écrit en rejouant nos suites contre `9a66fbf`
(épinglé chez nous, et c'est bien contre lui qu'on compile : arbre propre,
`Cargo.toml` → `../../../lucivy/lucivy_core`).

Le correctif fsync tient ses promesses partout (`e2e_search` 16,0 → 11,8 s,
`e2e_symbol_search` 12,8 → 5,5 s, notre profil 9 docs : commit 679 → 25 ms).
Mais une suite a fait le chemin inverse, et ce n'est pas du bruit :

```
e2e_idempotent_registration : 33,2 s  →  178,6 s   (22 tests, tous verts)
```

## 1. Ce n'est pas uniforme : trois tests portent 172 des 179 secondes

Chronométrage test par test, sur le binaire d'avant nos propres changements
(donc : uniquement l'effet de `9a66fbf`) :

```
   79,6 s  kb_and_relation_persist_and_reopen
   49,1 s  register_entity_idempotent_same_config
   43,6 s  ingest_after_migration_works
    2,0 s  entity_config_persisted_in_catalog_meta
    1,2 s  kb_vector_search_survives_migration
    < 1 s  les 17 autres
```

Et ça dépend de l'ordre : `ingest_after_migration_works` **seul** tourne en
0,21 s. Le test de réouverture, lui, est lent même seul : 43 s ; avec notre
tampon d'écriture (encore plus de commits rapides) : 25,7 / 24,4 / 23,3 s sur
trois runs. Il devrait tenir en une seconde.

## 2. Ce que luciole a dit pendant le stall

Votre détecteur a parlé tout seul, une fois (le test seul, 43 s) :

```
[luciole] WARNING: wait("commit_shard") blocked 10s (warn #1)
Threads:            scheduler-0..23 : IDLE   (un seul en TASK)
Ready queue: 0
Non-idle actors:    (all idle)
WaitGraph (3 edges):
  kb_and_relation_persist_and_reopen --[commit_shard]-->                 waiting (10.0s)
  shard_36  --[shard_0_flush (0/8)]-->                                   waiting (8.0s)
  indexer_1 --[indexer_flush_finalize (0/1)]-->                          waiting (8.0s)
```

Lecture, en suivant votre code :

- `ShardedHandle::commit()` disperse `ShardMsg::Commit` (`sharded_handle.rs:2027`).
- L'acteur shard appelle `flush_workers()` et confie les 8 récepteurs à
  `collect_replies_to(…, "shard_0_flush")` (`sharded_handle.rs:919`) :
  **0 réponse sur 8**.
- Côté indexer, `handle_flush()` (`indexer_actor.rs:230`) rend les récepteurs
  des **finalisations de segment soumises en arrière-plan** — celle laissée par
  `handle_docs` plus celle de `submit_finalize_task()` — et l'indexer attend
  qu'elles aboutissent via `collect_replies_to(…, "indexer_flush_finalize")`
  avant de répondre au shard (`indexer_actor.rs:410`). **0 sur 1.**
- Et pendant ce temps : aucun thread ne travaille, aucune tâche en file.

Une finalisation soumise, qui n'est ni en cours ni en attente, et dont la
réponse n'arrive pas. Ce n'est pas une contention — c'est une réponse qui ne
viendra pas par le chemin normal.

## 3. Un trou vérifiable dans le contrat « race-free » de `collect_replies_to`

`set_pipe` et `Reply::send` sont corrects entre eux : même verrou, valeur déjà
arrivée servie immédiatement. Ce n'est pas un réveil perdu entre eux.

Mais regardez l'**émetteur qui meurt sans répondre** (`luciole/src/reply.rs`,
`impl Drop for Reply`) :

```rust
let mut state = self.inner.state.lock().unwrap();
state.closed = true;
self.inner.ready.notify_one();     // réveille un wait_blocking
drop(state);
if let Some(handle) = self.inner.resume.lock().unwrap().take() {
    handle.fire();                 // réveille un acteur suspendu
}
// … mais `state.on_send` (le pipe) n'est jamais invoqué.
```

Et `set_pipe` :

```rust
if let Some(value) = state.value.take() { … callback(value); return true; }
state.on_send = Some(Box::new(callback));   // stocké même si `closed == true`
```

Il vérifie `value`, pas `closed`. Un récepteur dont l'émetteur est déjà mort
accepte donc un pipe qui ne sera **jamais** appelé, et un `collect_replies_to`
qui contient ce récepteur reste à `(k/n)` définitivement, tout le monde idle.
C'est exactement la signature ci-dessus. Le `wait_blocking` et le `resume`
sont prévenus de la mort ; le pipe ne l'est pas.

## 4. Ce que je ne sais pas

- **Qui est l'émetteur qui meurt.** La finalisation en arrière-plan est le
  candidat évident (`submit_finalize_task`), et la réouverture est le
  déclencheur déterministe — mais je n'ai pas tracé pourquoi sa `Reply` est
  lâchée sans `send`. Segment vide après réouverture ? Tâche jetée quand un
  pool s'arrête ? C'est chez vous que ça se voit.
- **Ce qui débloque après ~25-40 s.** Quelque chose finit par produire les
  réponses ; je n'ai pas cherché quel délai ou quel chemin de secours.
- **Si le stall est nouveau ou seulement plus long.** Avant `9a66fbf` la suite
  faisait 33 s, sans chiffres par test. Il est possible que ces trois tests
  perdaient déjà ~8 s chacun et que le retrait des fsync ait déplacé la
  course. La corrélation avec `9a66fbf` est observée, le mécanisme causal ne
  l'est pas.

## 5. Reproduire

```bash
cd extension/rag3weaver && bash run_e2e.sh --build-only --no-cuda
B=…/build/native-test/src ; export RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR=$B \
  RAG3DB_INCLUDE_DIR=$B RAG3DB_ROOT=… LD_LIBRARY_PATH=$B
cargo test --features rag3db-native --test e2e_idempotent_registration \
  -- --ignored --test-threads=1 --nocapture kb_and_relation_persist_and_reopen
```

Attendu : ~1 s. Observé : 23 à 43 s, avec ou sans le dump luciole selon que
l'attente unitaire franchit 10 s.

## 6. Ce qu'on demande

Rien d'urgent côté résultats — tout est vert. Mais tant que ça dure, chaque
commit après réouverture coûte 25 s, ce qui rend inutilisable exactement le
cas d'usage de l'agent qui rouvre sa base et reprend.

Deux choses seraient utiles, dans l'ordre :

1. Que `Drop for Reply` honore le pipe (l'appeler avec une erreur, ou faire
   échouer le `collect` proprement), et que `set_pipe` refuse un récepteur
   déjà fermé. Indépendamment de *ce* bug, le contrat « race-free » n'est pas
   tenu sans ça.
2. Le pourquoi de la `Reply` lâchée à la réouverture — c'est le vrai correctif,
   et le seul que vous pouvez trouver.

Nos chiffres de référence pour le rejeu : `kb_and_relation_persist_and_reopen`
seul, ~1 s ; la suite complète, ~35 s.
