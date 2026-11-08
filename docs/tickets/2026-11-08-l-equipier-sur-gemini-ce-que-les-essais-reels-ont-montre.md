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

**L'attente du superviseur** (points 1 et 2 ci-dessous) est corrigée par le lot de l'état
d'attente. Une question ou une commande demandée, sans réponse ni ligne nouvelle adressée à
l'équipier, le suspend : aucun appel au modèle, la boîte dit une fois « en attente du
superviseur depuis HH:MM », et le pas ne compte ni réussi, ni raté, ni dans les 40. La réponse
le réveille. L'attente se lit dans la boîte, elle survit donc à un redémarrage de l'hôte. Le
témoin est `waiting_for_the_supervisor_suspends_the_teammate_without_failing_the_task`, rouge
sans le lot.

**L'hôte s'arrêtait pour inactivité sous un équipier au travail** (hôte de Lucie, 8 novembre
4 h 46 : « inactif depuis 1804 s sans travail en cours », alors que l'équipier de la tâche
tests-bases-dans-tmp venait d'appeler edit_file et read_file ; systemd l'a relancé par la
socket, la tâche a repris à la frontière, le pas en cours perdu, puis un 429). Corrigé : un
équipier en route, au travail ou en attente du superviseur, retient l'arrêt comme un index en
cours. Témoin : l'équipier suspendu de `waiting_for_the_supervisor_suspends_the_teammate_without_failing_the_task`
retient `travail_en_cours`.

**Le fil par pas** (point 3) : un pas repart d'une session neuve, avec la demande d'origine,
les step_done des pas faits, la boîte et les plans. Témoin :
`each_step_starts_from_a_fresh_thread_with_the_request_and_the_steps_done`. La mesure après,
sur un essai réel, reste à prendre.

**Un refus du fournisseur n'est plus un pas raté** (point 3, sa seconde moitié). Un quota
dépassé (429) ou un service en panne (5xx, la connexion) fait attendre `teammate.quota_wait_s`
(60 s), doublé à chaque refus d'affilée jusqu'à dix fois, puis rejoue le même pas, ses
nouvelles rendues ; la boîte dit chaque attente, un `stop` la coupe, et
`teammate.max_unavailable_steps` (10) refus d'affilée arrêtent en le disant. Un quota épuisé
(par jour, de dépense) arrête tout de suite. Témoin :
`a_provider_refusal_waits_a_window_and_does_not_fail_the_step` (trois 429, autant que
`max_failed_steps`, et l'équipier finit).

**Un hôte rendu à un binaire d'avant refusait les arbres du gabarit neuf** (8 novembre, 9 h :
G a reposé le binaire eb154bf5f après deux SIGSEGV de la lib dadbf0041, et l'arbre d'essais de
c7 ne s'ouvrait plus, « unknown field `env` »). `deny_unknown_fields` reste : un champ de bac à
sable ignoré en silence serait un bac à sable qui ne dit pas ce qu'il fait. Désormais :

- le refus d'un champ inconnu nomme le commit du binaire qui lit, et la raison probable (un
  manifeste écrit pour un binaire plus récent) ;
- **la règle pour un repli d'hôte** : avant de rendre un hôte à un binaire d'avant, jouer
  `rag3weaver-backend check-manifests` avec ce binaire-là ; il lit les manifestes des arbres
  inscrits sans ouvrir de base, dit lesquels ne s'ouvriront pas, et rend le code 1 s'il y en a.
  Elle ne vaut que pour les binaires qui portent la commande, à partir de ce lot.

**Le gabarit code borne cargo** : `CARGO_BUILD_JOBS=8` dans `sandbox.env`, la règle de Lucie
(huit au plus, deux cœurs libres) ; `code-tree add` le recopie avec le reste du workspace.

**Le crochet d'après edit_file** (points 8 et 9). Un gabarit nommé introuvable dans le dossier
des gabarits se lit dans le gabarit livré du même nom, que le binaire embarque : un hôte lancé
hors du crate trouve `motif-ailleurs` (témoin `a_named_template_falls_back_to_the_shipped_one`).
Un crochet dont l'appel ne porte pas un paramètre requis (edit_file qui écrit le contenu entier,
sans `old`) se tait et le dit au journal, au lieu d'une erreur.

## Ce qui reste

1. **Attendre le superviseur tue la tâche** (corrigé, voir plus haut) (essai 4, racine-disparue). L'équipier a demandé à
   c7 de jouer un témoin, puis a marqué ses pas d'attente `succeeded: false` (« raté : en
   attente de la sortie de cargo test par le superviseur »). Au troisième, l'hôte a arrêté la
   tâche : « 3 pas de suite sans élément réussi : je m'arrête ». Attendre une réponse n'est pas
   un pas raté. Il faut un état d'attente : la question est posée, le pas est suspendu, et le
   modèle n'est pas appelé avant la réponse. Un pas d'attente ne compte ni comme réussi ni comme
   raté.
2. **Il répète le même message pendant l'attente** (corrigé avec le point 1). Quatre lignes identiques en une minute
   (report, question, step_done, report), chacune un appel à Gemini.
3. **Le quota frappe l'équipier seul** (essai 3). Des 429 RESOURCE_EXHAUSTED entre 01:20 et
   01:26 alors que e2e_juge, e2e_equipier et le juge des bilans passaient sur le même projet. À
   01:37 : « le pas 6 n'a pas abouti : quota dépassé après 2 reprises en 186 s ». Les journaux
   de pas font 87 à 130 Ko. Hypothèse : un quota de jetons d'entrée par minute, et l'équipier
   renvoie tout son fil à chaque pas. Un pas devrait repartir du plan, de la boîte et du résumé
   des pas faits. **Mesuré le 8 novembre** sur les journaux des runs de la tâche
   (`runs/928c89429e76`, les `prompt_tokens` de chaque appel) : l'hypothèse tient. Chaque pas
   repart de tout le fil des pas d'avant.

   | pas | heure | appels | entrée du 1er appel | entrée max | entrée cumulée |
   |---|---|---|---|---|---|
   | 1 | 01:02:34 | 12 | 5 275 | 24 301 | 179 257 |
   | 2 | 01:06:17 | 11 | 25 735 | 41 967 | 356 777 |
   | 3 | 01:12:57 | 12 | 43 197 | 58 079 | 616 223 |
   | 4 | 01:15:48 | 12 | 62 737 | 73 010 | 815 709 |
   | 5 | 01:17:00 | 12 | 74 466 | 83 060 | 950 822 |
   | 6 | 01:19:58 | 11 | 84 192 | 92 188 | 962 379 |
   | 10 | 01:45:07 | 3 | 122 242 | 122 688 | 367 512 |

   Le pas 5 envoie environ 950 000 jetons d'entrée en trois minutes, et le pas 6 tombe sur les
   429 avec deux reprises. Un pas qui repartirait du plan, de la boîte et du résumé des pas
   faits garderait son entrée près de celle du pas 1. Avec lui : un 429 ou un 503 ne compte pas dans `max_failed_steps`, et la reprise
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
8. **Le crochet d'après edit_file ne trouve pas son gabarit** (corrigé, voir plus haut) (essai 3, 01:25:58) : «
   RenderResultsNode: gabarit 'motif-ailleurs' : ni un nom fourni (default, compact), ni du
   Jinja ». Un nom de gabarit se lit dans `templates/render/<nom>.md.jinja` relatif au
   répertoire courant du processus, ou à `RAG3WEAVER_RENDER_TEMPLATES`
   (`src/dataflow/render_nodes.rs`), pas relatif au manifeste. Un hôte lancé ailleurs que dans
   `extension/rag3weaver` ne le trouve pas. Deux formes : résoudre depuis le dossier du
   manifeste ou du crate, ou fournir `motif-ailleurs` comme `compact`.
9. **« argument requis manquant : 'old' »** (corrigé, voir plus haut) (essai 3, 01:33:54), le modèle ayant appelé
   edit_file sans `old`. Vérifier que le résultat d'edit_file le dit au modèle.

## Pour le fermer

Un lot par point ou par famille de points, chacun avec son témoin. L'état d'attente (points 1
et 2) passe en premier : c'est lui qui arrête les tâches réelles et qui tire sur le quota.
