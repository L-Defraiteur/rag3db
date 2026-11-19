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
