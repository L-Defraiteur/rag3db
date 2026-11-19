# Proposition de garde : un compte rendu n'est pas le contenu demandé

Session tty, 19 novembre 2026, au mot de c7, pour qu'archi conçoive dessus. Ticket : [`docs/rag3weaver-archi/tickets/2026-11-14-l-equipier-rend-un-compte-rendu-au-lieu-du-contenu-demande.md`](../../rag3weaver-archi/tickets/2026-11-14-l-equipier-rend-un-compte-rendu-au-lieu-du-contenu-demande.md).

## Le témoin

`daemon_host::teammate_tests::a_report_about_the_text_is_not_the_text`, branche `cx-equipier-compte-rendu`. Il rejoue la séance du 14 novembre. La request « Présente-toi … » n'a pas d'`expects`. Au pas 1, le faux modèle n'écrit aucun texte : il appelle `report` avec « Présentation fournie au superviseur conformément au format demandé … », puis `step_done` réussi. À partir du pas 2, il écrit la présentation, si une garde le relance.

Vu rouge : la tâche finit « fait sans plan », et le contenu n'est nulle part.

Le témoin ne choisit pas la garde. Il refuse seulement la fin silencieuse. Il est vert si le contenu arrive dans la boîte, ou si l'hôte dit dans la boîte que le contenu manque sans finir la tâche.

## Ce qui existe déjà

- `expects: "report"` : une request qui exige un report ne se clôt pas sans lui. L'équipier attend alors, et l'hôte écrit « fait sans le livrable demandé ».
- Le correctif du 14 (9009f3a3b) : un texte écrit dans le pas arrive dans la boîte, et `report` dit qu'il porte « le texte lui-même ».

Le trou : sans `expects`, rien ne regarde si le texte demandé est venu.

## La garde proposée, en trois couches

1. **Quand regarder : seulement un pas déclaré réussi qui n'a écrit aucun texte**, sous une request sans `expects`, et dont les reports sont courts. Le cas sain (un texte écrit ou rapporté) ne coûte rien.
2. **Qui juge : le décideur local**, le même que pour les commandes (`daemon_host/juge.rs`), avec une seule question : « La demande attend-elle un texte, et ce texte est-il dans la boîte ? », avec la request et les reports du pas. Il répond `livré`, `manque` ou `action` (la demande ne voulait pas de texte). Une règle de mots (« fourni », « conformément ») serait fragile : je ne la propose pas.
3. **Ce qui suit un `manque`** :
   - l'hôte écrit « fait sans le contenu demandé : <raison du juge> » ;
   - le pas repart une fois, avec une frontière qui dit « écris le texte lui-même dans ta réponse » ;
   - au second manque, l'équipier attend le superviseur, comme sous `expects: "report"`.

   Sans juge joignable, la fin n'est pas silencieuse : l'hôte écrit « fin sans texte, à vérifier ».

## Les points à trancher pour archi

- Est-ce le juge des commandes ou un appel au modèle de l'équipier lui-même (plus cher, mais sans dépendance) ?
- La relance unique : est-elle bien au compte des pas (MAX_STEPS) ?
- Le terminal (tty) et l'IDE pourraient-ils poser `expects: "report"` d'eux-mêmes quand la demande est une question ? Je ne le propose pas : le superviseur ne le sait pas mieux que le juge.

## Le correctif (19 novembre 2026, sur la conception d'archi)

Archi a tranché les trois points dans `docs/rag3weaver-archi/19-11-2026/01-garde-compte-rendu-au-lieu-du-contenu.md`. Le juge n'est pas le décideur local ; c'est le modèle de la tâche lui-même. Le code est dans `daemon_host/teammate.rs`, `DaemonHost::judge_the_content`, à la fin d'un pas déclaré réussi qui finirait la tâche.

- **Couche 1, sans appel** : le pas n'a écrit aucun texte, ses reports font moins de 300 caractères chacun, et la request n'a pas d'`expects`. Hors de ce cas, rien ne change.
- **Couche 2, le juge** : un appel au modèle de la tâche (`llm_du_chat`), hors du tour : il n'entre pas dans la session de la tâche. Température 0. La question fermée en anglais (`CONTENT_QUESTION`) porte la request et les reports. Le verdict est le premier mot de la réponse : `delivered`, `missing` ou `action`. À la relecture d'archi, le raisonnement est coupé (`ReasoningEffort::Minimal`), avec 512 jetons de place : 50 jetons, qu'un modèle qui raisonne dépense à penser (2 805 jetons de raisonnement dans D1 sur gemini-3.8-flash), laisseraient la réponse vide. La consommation de l'appel s'inscrit au compteur du chat, sur le run du pas, sous l'agent `content-judge` : l'usage du run la porte.
- **Couche 3, ce qui suit** :
  - `missing` : l'hôte écrit « ended without the requested content: … the next step starts to write it », et le pas suivant part avec une relance. Les lignes de boîte sont en anglais, comme la relance : le modèle relit la boîte à sa frontière. C'est la relance unique de la request, partagée avec la garde B (`relancee`). C'est un pas, compté dans `MAX_STEPS`.
  - un second `missing` : « ended without the requested content, after the relaunch: waiting for the supervisor », une attente bornée par `teammate.reply_wait_s`.
  - un juge en échec (une erreur, ou un premier mot qui n'est aucun des trois) : « ended without a text, to be checked (…) », et la tâche finit comme avant.
  - `delivered` et `action` : la tâche finit comme avant.

L'appel du juge se dit aussi sur la sortie d'erreur de l'hôte (« le juge du contenu dit … »). Il n'émet pas de `LlmCall` sur le bus : archi n'en veut pas, le compteur suffit.

### Les témoins

Tous dans `daemon_host::teammate_tests`, vus rouges (31603f67a2, puis d6cda0c9fc avec le faux juge), puis verts (d09dd37392) :
- `a_report_about_the_text_is_not_the_text` : le juge dit `missing`, et le contenu arrive au pas de la relance ;
- `a_report_judged_an_action_lets_the_task_end` : le juge est appelé une fois, dit `action`, et la tâche finit « fait sans plan », sans ligne de manque ;
- `a_judge_that_fails_says_the_end_is_to_be_checked` : le juge est en panne, et la boîte dit « ended without a text, to be checked » ;
- à la relecture d'archi (rouges 7c80e7ee14) : `the_verdict_is_the_judge_s_first_word` (« Delivered, not missing. » est une livraison) et `the_judge_thinks_little_and_counts_in_the_run_s_usage` (raisonnement minimal ou place pour lui, et la consommation sur le run du pas). Les témoins attendent le superviseur 3 s : un rouge qui l'attendait 1 800 s a tenu la place lourde du portable.

Les modules `daemon_host::teammate` (48) et `daemon_host::chat` (21) restent verts, sous la lib épinglée 3d706f709 avec `RAG3WEAVER_MOTEUR_ANCIEN=1`, en le sachant. Le témoin réel sous Vertex (`e2e_equipier`) n'est pas joué par tty, qui ne lit pas les identifiants : il est à la passe.

