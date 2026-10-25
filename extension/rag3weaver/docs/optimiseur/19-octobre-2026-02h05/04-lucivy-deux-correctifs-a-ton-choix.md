# lucivy : deux correctifs prêts, à ton choix (G, 19 octobre 2026)

Deux défauts de **lucivy-core 4.3.0** ont été trouvés cette nuit depuis rag3db. Leurs correctifs
sont dans ton dépôt lucivy, sur une branche locale **non poussée** : `correctifs-rag3db-19-octobre`,
dans le worktree `~/git_workspaces/lucivy-figement-G`, détaché de `v4.3.0`. Deux commits, rien
d'autre.

## 1. Le commit se fige sous des recherches (le plus grave)

**Ce qu'on voit.** Pendant une réindexation que des agents lisent en même temps, la montre ne
réindexe plus rien, sans erreur. L'arbre principal l'a vu trois fois sur trois en mesurant L6 :
l'acteur d'un shard reste « BUSY » des minutes.

**La cause**, lue sur les piles du processus figé : un **interblocage par réentrance**.
- La tâche qui finalise un commit (`ShardMsg::FlushDone`, `sharded_handle.rs:950-982`) tient le
  verrou de l'écrivain pendant `prepared.commit()`.
- Ce commit attend **de façon coopérative** (`SegmentUpdater::schedule_commit_with_rebuild`, puis
  `Scheduler::wait`, puis `run_one_step`) : en attendant, le fil exécute d'autres messages.
- S'il prend un travail du **même shard** qui redemande ce verrou, il s'attend lui-même. Deux
  chemins y mènent : un `Insert`, un `Delete` ou un `Commit` (une recherche en envoie un pour voir
  les écritures en cours) ; ou la finalisation d'un second commit entrelacé.
- Sans lecteurs, rien n'arrive au shard pendant son commit : c'est pourquoi les recherches le
  déclenchent.

**Le correctif** : un seul commit en vol par shard, de `Commit` à `CommitDone`. Les écritures et
commits suivants sont différés dans leur ordre, puis rejoués à la fin, erreurs comprises. L'acteur
ne se bloque plus jamais sur un verrou que sa propre tâche tient.

**Les preuves** :
- Recette réduite dans lucivy-core seul (`test_commit_under_concurrent_searches_does_not_freeze`) :
  quatre lecteurs en boucle pendant 30 commits. **Sur v4.3.0, figé au tour 1.**
- Un premier correctif, qui ne différait que pendant la finalisation, figeait encore au tour 5.
  Les piles ont montré le second chemin, que ce correctif couvre aussi.
- Avec le correctif : 30 commits en 5 s. **Les 94 tests de lucivy-core sont verts.**
- Le témoin réel de rag3db (`e2e_lectures::mesure_des_instantanes_sur_un_vrai_depot`, 219 fichiers, quatre lecteurs) **passe ses trois tours** avec ce lucivy-core, là où master se fige au tour 1 trois fois sur trois. Lancé avec `RAG3WEAVER_READ_POOL=0` : tours de 2,9 s, 527 s et 358 s ; test vert en 985 s. Le binaire mesuré contient bien `ShardActor::finish_commit`.

  **Mais la mémoire, à part** : avec quatre lecteurs, le processus du test monte en ligne droite jusqu'à 32 Go (RssAnon + VmSwap). Avec le pool d'instantanés de L6, il atteint le plafond de 40 Go et le noyau le tue au tour 2. Sans lecteurs, il plafonne à 7,2 Go sur lucivy publié et à 7,3 Go sur lucivy patché : le correctif n'y change rien. La croissance vient donc des lecteurs. Rien ne l'impute au correctif, mais rien ne l'en disculpe non plus avec quatre lecteurs, puisque la version publiée se fige avant qu'on puisse comparer. Je la départage à part : avec `MALLOC_ARENA_MAX=2`, la même passe finit verte à 29,1 Go, donc ce n'est pas la fragmentation des arènes de glibc. Qui tient cette mémoire est la question suivante (G, avec les blocs de 32 Mio relevés par le banc). Elle ne change rien au choix ci-dessous, puisque le figement, lui, est réglé.

## 2. Une écriture perdue rendait `Ok`

Un document envoyé à un shard dont l'écrivain est lâché était jeté, une suppression sautée, et
l'erreur d'`add_document` jetée (`let _ =`). `add_document` puis `commit` rendaient `Ok`.
L'appelant croyait l'écriture faite. Ce n'est pas atteignable aujourd'hui depuis rag3db, qui garde
ses écrivains ouverts, mais ça le devient dès qu'un appelant en lâche un.

**Le correctif** : l'acteur garde chaque écriture qu'il n'a pas pu appliquer, et le `Commit`
suivant les dit en erreur (« N écriture(s) non appliquée(s) depuis le dernier commit — … »).
Témoin `test_writes_to_a_shard_without_writer_are_reported_by_commit` : rouge sur v4.3.0, vert avec
le correctif.

## Les deux voies

rag3db prend lucivy par sa version publiée, et rien n'est sur master pour lucivy. Donc :

1. **Tu publies une 4.3.1 de lucivy-core** avec ces deux commits, avec tes OTP. rag3db la prend
   aussitôt (un changement de version, la passe complète). C'est la voie propre.
2. **Ou tu autorises un `[patch.crates-io]` provisoire** dans rag3db vers ces deux commits, jusqu'à
   ta version. Il enlève le figement tout de suite, mais rag3db ne prendrait plus lucivy par sa
   version publiée pendant ce temps.

Je ne vois pas de contournement sûr dans rag3db seul. Il faudrait empêcher toute recherche pendant
un commit d'écriture, ce qui défait les lectures sans attente de L4 et L6.

Ticket : `docs/tickets/2026-10-19-le-plein-texte-lucivy-se-fige-dans-son-commit-pendant-une-reindexation-lue.md`
(à mon nom), et pour le second
`docs/tickets/2026-10-19-un-shard-de-lucivy-sans-ecrivain-perd-les-ecritures-en-silence.md`.
