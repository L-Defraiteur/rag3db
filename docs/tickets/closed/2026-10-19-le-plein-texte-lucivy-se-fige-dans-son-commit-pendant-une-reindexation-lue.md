# Le plein texte lucivy se fige dans son commit pendant une réindexation lue

- **État** : **corrigé** dans lucivy 4.3.1, publiée par Lucie le 25 octobre 2026, et prise par rag3weaver à sa version publiée. La cause était plus large que la réentrance du commit : une panique de l'IDF (bm25.rs, `assert!(doc_count >= doc_freq)`) sous commits concurrents tuait l'acteur d'un shard. La 4.3.1 borne l'IDF, protège le Search (catch_unwind), garde un seul commit en vol et rejoue ses différés sur place, Drain compris. Preuves : la recette sharded_handle 80 fois sur 80 sous charge, et le témoin réel à quatre lecteurs en 3 tours.
- **Suivi par** : G (optimiseur).
- **Gravité** : blocage — la montre ne réindexe plus rien, sans erreur ; les lectures continuent sur l'état d'avant (« Une écriture de l'index est en cours »).
- **Atteignable en service** : oui, a priori. C'est le régime ordinaire d'un hôte de code : un agent lit pendant que des fichiers changent sur le disque.
- **Touche rag3weaver** : oui (la montre, `reindex_on_change`) ; le défaut et le correctif sont dans lucivy (`lucivy-core` 4.3.0, `sharded_handle.rs`).

## Ce que c'est

Un dossier de code réel (`extension/rag3weaver/src`, 213 fichiers) indexé par le gabarit de code
livré, servi par un `DaemonHost`. Quatre lecteurs bouclent sur `search_code` et `usages`, et tous
les fichiers sont modifiés deux fois de suite.

- Le premier tour se réindexe en 2,8 s.
- Au second, la montre réindexe fichier par fichier (environ 1 s de verrou chacun), puis s'arrête
  après une soixantaine de fichiers. Le journal répète, toutes les 10 s jusqu'à la fin du délai
  du test (600 s) :

```
[luciole] WARNING: wait("flush_shard") blocked 530s (warn #53)
  ActorId(68) shard: BUSY processing (529.2s) q:0
WaitGraph (10 edges):
  unknown --[flush_shard]--> waiting
  shard_68 --[shard_0_commit]--> waiting
  scheduler-8 --[schedule_commit]--> waiting
  rag3weaver-graphes --[drain_shards]--> waiting
```

L'acteur du shard 68 est occupé sans fin. Il attend `shard_0_commit`, le flush attend le shard, et
le drain du graphe de réindexation attend `drain_shards`.

## La cause (piles du processus figé, puis recette réduite)

Un **interblocage par réentrance** dans l'acteur d'un shard de `lucivy-core` 4.3.0.

- La tâche qui finalise un commit (`ShardMsg::FlushDone`, `sharded_handle.rs:950-982`) tient le
  verrou de l'écrivain pendant `prepared.commit()`.
- Ce commit attend de façon **coopérative** (`schedule_commit_with_rebuild`, puis
  `Scheduler::wait`, puis `run_one_step`). En attendant, le fil exécute d'autres messages.
- S'il prend un travail du même shard qui redemande ce verrou, il s'attend lui-même. Ce travail
  peut être un `Insert`, un `Delete` ou un `Commit` : une recherche en envoie un pour voir les
  écritures en cours. Ce peut aussi être la finalisation d'un second commit entrelacé.

**Départage** : les lecteurs sont nécessaires. Sans eux, rien n'arrive au shard pendant son
commit. La recette réduite dans lucivy-core le montre :
`test_commit_under_concurrent_searches_does_not_freeze`, quatre lecteurs en boucle pendant
30 commits de 50 documents.

- Sur v4.3.0, elle **se fige au tour 1**.
- Un premier correctif, qui ne différait que pendant la finalisation, se figeait encore au tour 5.
  C'est ainsi que le second chemin est apparu.

## Le correctif (dans lucivy, non poussé)

Un seul commit en vol par shard, de `Commit` à `CommitDone`. Les `Insert`, `Delete` et `Commit`
qui arrivent pendant ce temps sont différés dans leur ordre, puis rejoués à la fin, y compris sur
erreur.

Il se trouve dans le dépôt lucivy de Lucie : branche locale `correctifs-rag3db-19-octobre`
(worktree `~/git_workspaces/lucivy-figement-G`, sur `v4.3.0`), commit `dbb7d6e`. Le second
commit de la branche, `bb07627`, est le correctif du ticket
`2026-10-19-un-shard-de-lucivy-sans-ecrivain-perd-les-ecritures-en-silence.md`, pour une seule
version.

Résultats :
- la recette passe ses 30 commits en 5 s ;
- les 94 tests de lucivy-core sont verts ;
- le témoin réel de rag3db (`e2e_lectures::mesure_des_instantanes_sur_un_vrai_depot`, 219 fichiers, quatre lecteurs) **passe ses trois tours** avec ce lucivy-core, là où master se fige au tour 1 trois fois sur trois. Lancé avec `RAG3WEAVER_READ_POOL=0` : tours de 2,9 s, 527 s et 358 s ; test vert en 985 s. Le binaire mesuré contient bien `ShardActor::finish_commit`.

  **Mais la mémoire, à part** : avec quatre lecteurs, le processus du test monte en ligne droite jusqu'à 32 Go (RssAnon + VmSwap). Avec le pool d'instantanés de L6, il atteint le plafond de 40 Go et le noyau le tue au tour 2. Sans lecteurs, il plafonne à 7,2 Go sur lucivy publié et à 7,3 Go sur lucivy patché : le correctif n'y change rien. La croissance vient donc des lecteurs. Rien ne l'impute au correctif, mais rien ne l'en disculpe non plus avec quatre lecteurs, puisque la version publiée se fige avant qu'on puisse comparer. Je la départage à part : avec `MALLOC_ARENA_MAX=2`, la même passe finit verte à 29,1 Go, donc ce n'est pas la fragmentation des arènes de glibc. Qui tient cette mémoire est la question suivante (G, avec les blocs de 32 Mio relevés par le banc). Elle ne change rien au choix ci-dessous, puisque le figement, lui, est réglé.

**Aucun contournement sûr côté rag3weaver** : il faudrait interdire toute recherche pendant un
commit d'écriture, ce qui défait les lectures sans attente.

## La recette

Le témoin ci-dessous, sur luciepc :

- lib cf71c929d ;
- services d'embarquement 7878-7880 ;
- `RAG3WEAVER_MESURE_ARBRE` pointant sur un dossier de code.

Le rouge se reproduit trois fois sur trois. La recette déterministe est celle de lucivy-core,
plus haut.

## Le témoin

`e2e_lectures::mesure_des_instantanes_sur_un_vrai_depot` (arrive avec L6), qui échoue sur « le
tour 1 n'est pas réindexé ». Il se saute sans `RAG3WEAVER_MESURE_ARBRE` et le dit.

## Pour le fermer

Lucie choisit sa voie : une 4.3.1 publiée, prise aussitôt ici ; ou un `[patch.crates-io]`
provisoire, qu'elle autorise. Le témoin de mesure doit ensuite passer ses trois tours sur la lib
qui porte la version corrigée.

## Ses filets

- `extension/rag3weaver/Cargo.toml`, bloc `[patch.crates-io]` (lucivy, à `a925d94`, la 4.3.1 de Lucie, depuis le 25 octobre au soir) : rag3weaver prend
  ce correctif sous un patch temporaire depuis le 25 octobre 2026, jusqu'à la publication par
  Lucie. Son retrait a son ticket : `2026-10-25-retirer-le-patch-lucivy-a-la-publication.md`.
