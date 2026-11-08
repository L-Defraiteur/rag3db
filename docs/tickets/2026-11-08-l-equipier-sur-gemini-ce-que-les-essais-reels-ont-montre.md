# L'équipier sur Gemini : ce que les essais réels ont montré

- **État** : ouvert (8 novembre 2026), porteur everything declarative. Relevé par c7 pendant les essais 3 et 4 de l'équipier (page `extension/rag3weaver/docs/08-novembre-2026-orchestration/01-essais-de-l-equipier.md`).
- **Famille** : produit
- **Gravité** : une tâche réelle s'arrête ou tourne à vide. Attendre le superviseur la tue après trois pas, et chaque pas d'attente coûte un appel au modèle et du quota.
- **Atteignable en service** : oui, par toute tâche confiée à l'équipier depuis Claude Code ou l'IDE.
- **Touche rag3weaver** : oui, la boucle de l'équipier (`src/daemon_host/teammate.rs`), le juge des bilans, le crochet d'après edit_file.

## Ce qui est déjà corrigé

Une ligne postée pendant le dernier pas d'un lancement se perdait. La boucle finissait sans la
lire, la fin marquait toute la boîte comme lue, et la poste ne relançait rien parce que
l'équipier tenait encore sa clé. Le lot des commandes par la porte déclarée l'a corrigé :
la boucle ne finit pas sur une nouvelle non lue, le curseur reste où la boucle l'a laissé, et
une fin « plus rien à faire » relance l'équipier sur une ligne arrivée après sa dernière
lecture. Le témoin est `a_line_posted_during_the_last_step_is_read_before_the_teammate_ends`.
C'est probablement la cause de l'answer non lue de l'essai 4 (point 6 ci-dessous), à vérifier
dans le journal du run.

## Ce qui reste

1. **Attendre le superviseur tue la tâche** (essai 4, racine-disparue). L'équipier a demandé à
   c7 de jouer un témoin, puis a marqué ses pas d'attente `succeeded: false` (« raté : en
   attente de la sortie de cargo test par le superviseur »). Au troisième, l'hôte a arrêté la
   tâche : « 3 pas de suite sans élément réussi : je m'arrête ». Attendre une réponse n'est pas
   un pas raté. Il faut un état d'attente : la question est posée, le pas est suspendu, et le
   modèle n'est pas appelé avant la réponse. Un pas d'attente ne compte ni comme réussi ni comme
   raté.
2. **Il répète le même message pendant l'attente.** Quatre lignes identiques en une minute
   (report, question, step_done, report), chacune un appel à Gemini.
3. **Le quota frappe l'équipier seul** (essai 3). Des 429 RESOURCE_EXHAUSTED entre 01:20 et
   01:26 alors que e2e_juge, e2e_equipier et le juge des bilans passaient sur le même projet. À
   01:37 : « le pas 6 n'a pas abouti : quota dépassé après 2 reprises en 186 s ». Les journaux
   de pas font 87 à 130 Ko. Hypothèse : un quota de jetons d'entrée par minute, et l'équipier
   renvoie tout son fil à chaque pas. Un pas devrait repartir du plan, de la boîte et du résumé
   des pas faits. **À mesurer d'abord** : la taille des requêtes par pas dans le journal de
   l'hôte. Avec lui : un 429 ou un 503 ne compte pas dans `max_failed_steps`, et la reprise
   attend au moins une fenêtre de quota par minute.
4. **Le juge note chaque pas comme la tâche entière** (essai 3). Le run du pas 1
   (`agent-1a1b876c759-887`, « lire le ticket et le code ») a reçu 2/10, objectif non : « n'a pu
   accomplir aucune action corrective ni aucun test ». Le juge en a déduit que seuls read_file,
   grep_files et edit_plan étaient déclarés, ce qui est faux : edit_file et step_done l'étaient.
   Il faut donner au juge le but du pas (l'élément du plan) et la boîte, ou juger la tâche à sa
   fin. Trace : la fiche de la mémoire longue « bilan : tâche réelle… », apport du 8 novembre
   00:10 UTC.
5. **Une consigne de la boîte ne retient pas la clôture** (essai 3). c7 avait écrit « ne
   déclare pas la tâche accomplie avant ma confirmation du commit », et l'équipier a clos le
   plan (« plus rien à faire »). Dans le même essai, il a écrit « le témoin constate l'échec »
   sans avoir joué le témoin.
6. **L'answer à sa question n'a pas été lue** (essai 4). Il a réagi à la `request` suivante.
   À vérifier dans le journal du run `agent-1a1b8c5652d-1857` et les précédents de la tâche,
   à la lumière de la course corrigée plus haut.
7. **La reprise d'une tâche finie ne se dit pas** (essai 4). Une `request` sur une tâche
   `ended` l'a relancée à 02:35:23, ce qui est voulu. La réponse du POST et la boîte doivent
   dire « tâche reprise ».
8. **Le crochet d'après edit_file ne trouve pas son gabarit** (essai 3, 01:25:58) : «
   RenderResultsNode: gabarit 'motif-ailleurs' : ni un nom fourni (default, compact), ni du
   Jinja ». Un nom de gabarit se lit dans `templates/render/<nom>.md.jinja` relatif au
   répertoire courant du processus, ou à `RAG3WEAVER_RENDER_TEMPLATES`
   (`src/dataflow/render_nodes.rs`), pas relatif au manifeste. Un hôte lancé ailleurs que dans
   `extension/rag3weaver` ne le trouve pas. Deux formes : résoudre depuis le dossier du
   manifeste ou du crate, ou fournir `motif-ailleurs` comme `compact`.
9. **« argument requis manquant : 'old' »** (essai 3, 01:33:54), le modèle ayant appelé
   edit_file sans `old`. Vérifier que le résultat d'edit_file le dit au modèle.

## Pour le fermer

Un lot par point ou par famille de points, chacun avec son témoin. L'état d'attente (points 1
et 2) passe en premier : c'est lui qui arrête les tâches réelles et qui tire sur le quota.
