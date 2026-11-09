# L'équipier sur les README — l'essai du 9 novembre 2026

Une tâche réelle et bornée, posée par l'orchestration (c7) pour mesurer l'équipier par pas sur
Gemini : le rattrapage des README depuis `docs/nouveautes.md` (Lucie, 8 novembre : « le README
suit ce que l'utilisateur voit »). Superviseur : la session mémoire (96). Pour everything
declarative, qui tient l'équipier.

## Le cadre

- L'hôte du portable (7970), sur la lib 29a7bc52a depuis 12 h 23 ; `gemini-3.8-flash`, projet
  Vertex de Lucie, région `global`.
- L'arbre de déclarations de c7 (`code-essais-c7`, backend e4d8635ecfe7) tel quel : `edit_file`,
  `run_command` derrière une approbation ; son worktree `rag3db-essais-c7` avancé à master
  (74519fc65) par avance rapide.
- La tâche `readme-nouveautes-novembre`, postée à 12 h 58 min 12 s. Sa base à elle (backend
  852ca016d10f), son worktree sur `agent/readme-nouveautes-novembre`.
- La consigne : lire les lignes de nouveautés du 1er au 9 novembre, dire pour chacune « interne »,
  « déjà dit » ou « à ajouter » dans un report, puis une question par ajout avec le texte exact,
  le oui du superviseur, et l'édition ; rien hors des deux README ; aucune commande ; ne pas se
  dire fini avant la réponse au report final.

## Ce qui s'est passé

| | |
|---|---|
| pas | 1 |
| appels au modèle | 12 (dont 3 avec reprises : 1, 1 et 4) |
| durée du pas | 20 min 50 s (12 h 58 min 14 s → 13 h 19 min 04 s) |
| appels à reprises | 3, 1 165 s en tout ; l'appel 8 seul 1 004 s (16 min 44 s, 4 reprises) |
| jetons d'entrée, premier et dernier appel | 5 593 → 38 036 |
| outils | 4 `read_file`, 7 `grep_files`, 1 `step_done` |
| lignes de l'équipier dans la boîte | 1 (le `step_done`) |
| report de verdicts, questions, propositions, éditions | 0 |
| commandes demandées | 0 |
| fin | `ended : fait sans plan`, posée par l'hôte |

**Ce qu'il a lu, et bien.** Les deux README et la section de novembre de nouveautes.md en quatre
`read_file`, sans relire. Puis il a cherché dans les README les noms que les nouveautés
introduisent : `task_writes`, `git_init` (l'équipier sans git) ; `attach_worktree`,
`open_branch`, `list_worktrees` (l'atelier) ; `extension_error` (l'index vectoriel rouvert) ;
`daemon_status` et « disparu » (la racine disparue) ; `daemon.log` ; « quota », « reprise »,
« 503 », « 429 » (la reprise du client). Six lignes de nouveautés sur 31 vérifiées, en sept
recherches `grep_files` sur le bon fichier : c'était la bonne méthode, et rien d'inventé dans ce qu'il a
appelé.

**Où il a buté : pas chez lui.**
1. *Vingt minutes sans un mot.* De 12 h 58 min 51 s à 13 h 18 min 20 s : aucune ligne dans la
   boîte, rien dans `daemon.log`, aucun run sur le disque. Le fil `teammate-readme` dormait en
   `hrtimer_nanosleep` : le recul du client du modèle après des refus de Vertex. Ticket
   `2026-11-09-le-recul-du-client-du-modele-ne-se-voit-pas-et-n-a-pas-de-borne-de-pas`.
2. *La cause n'est écrite nulle part.* Le run compte les reprises, pas leurs codes (429 ou 503) ;
   on ne peut pas dire si le quota du projet était atteint. Ticket
   `2026-11-09-le-code-d-un-refus-du-modele-n-est-garde-nulle-part`.
3. *L'hôte a clos la tâche après le premier pas.* L'équipier n'a pas ouvert de plan ; son
   `step_done` disait `remaining : « Validation du verdict et soumission des ajouts un par un pour
   accord avant modification »`, c'est-à-dire exactement la suite demandée. L'hôte a lu « aucun
   plan vivant, un pas déclaré » et a écrit « fait sans plan ». Ticket
   `2026-11-09-une-tache-sans-plan-se-dit-faite-sur-un-step-done-qui-dit-ce-qui-reste`.

**Ce qui est à lui.** Il n'a pas écrit le report demandé au point (3) avant de déclarer son pas,
alors que la consigne le mettait avant tout ajout ; il a regroupé « analyse et verdict » en un pas
au lieu de le livrer dans la boîte. Sans le défaut 3, le pas suivant l'aurait peut-être écrit ;
l'essai ne permet pas de le dire.

## Pour l'orchestration et pour ED

- Les jetons d'entrée montent de 5 593 à 38 036 sur douze appels d'un seul pas : le fil par pas
  (une session neuve à chaque pas) ne joue pas à l'intérieur d'un pas. La mesure par pas d'ED
  (`mesurer_fil_par_pas.py`, runs/<backend> <tâche>) demande plusieurs pas : celle-ci n'en a qu'un.
- Deux choses vues à côté, sans ticket : `/ide/etat` étiquette le backend de la tâche avec la
  branche `essais-c7` alors que son worktree est sur `agent/readme-nouveautes-novembre` ; et les
  vecteurs de File et Scope du backend d'essais sont restés à « never » après la réouverture de
  12 h 24 (le défaut connu du rattrapage, correctif d'ED en attente de Lucie).
- Le lot des README n'existe pas : aucune proposition. Il se rejoue quand les tickets 1 et 3 sont
  corrigés, avec la même consigne.

## La référence de la prochaine tentative (fait à la main le 9 novembre, l'après-midi)

Le rattrapage que l'équipier n'a pas fait, fait par la session mémoire sur les 33 lignes de
novembre (les 31 de la tâche et les deux du 9). Les deux README sont **en anglais** : la consigne
« au ton des README » voulait dire en anglais, ce qu'elle ne disait pas ; la prochaine le dira.
Verdicts : **interne** (une correction que l'utilisateur ne voit pas comme une capacité), **déjà
dit** (le README le dit, section), **ajouté** (README, section).

| ligne de nouveautés | verdict |
|---|---|
| 9 nov. — un modèle qui refuse se voit attendre | ajouté : rag3weaver, *The agent* (la puce du modèle), avec la règle d'une minute du 8 |
| 9 nov. — l'atelier lit et écrit | déjà dit : rag3weaver, *The workshop* |
| 8 nov. — JavaScript, locales de blocs voisins | interne (justesse des usages) |
| 8 nov. — l'équipier sans git | ajouté : rag3weaver, *Working alone*, puce « A task without git » |
| 8 nov. — l'atelier ouvre un worktree ou une branche | déjà dit pour `attach_worktree` et `open_branch` ; ajouté : `list_worktrees`, la commande git dite, rien cloné ni poussé |
| 8 nov. — un quota, une minute puis le double | ajouté avec la ligne du 9 |
| 8 nov. — mes recherches | déjà dit : rag3weaver, *The IDE* |
| 8 nov. — le score dit d'où il vient | déjà dit : rag3weaver, *Search* (`bestSignal`, `chunk.signal`) |
| 8 nov. — le code du chat aux couleurs du dépôt | déjà dit : rag3weaver, *The IDE* (`POST /highlight`) |
| 8 nov. — un journal qui recopie puis met à jour une relation | déjà dit : racine, *Hardened write paths* |
| 8 nov. — l'index vectoriel retrouvé, `extension_error` | ajouté : racine, la puce de `CREATE_VECTOR_INDEX` |
| 8 nov. — une phrase collée, le surlignage sans modèle, le second écrivain | déjà dit pour les deux premiers (*Search*) ; ajouté : le second écrivain refusé avec qui tient la base (rag3weaver, après les binaires) |
| 8 nov. — les fichiers d'annulation hors de `/tmp` | interne |
| 8 nov. — un gros COPY de blobs | déjà dit : racine, *Hardened write paths* |
| 8 nov. — la racine disparue | ajouté : rag3weaver, *The agent*, la puce du manifeste |
| 7 nov. — un appel dans une garde de `match` | interne (justesse des usages) |
| 7 nov. — le flux de l'IDE ne perd plus un événement | interne ; la barre par `kind` est dans la ligne « la barre parle en dépôts » |
| 7 nov. — le chat tient l'ordre d'arrivée | ajouté : rag3weaver, *The IDE* |
| 7 nov. — la barre parle en dépôts | ajouté : rag3weaver, *The IDE* |
| 7 nov. — le journal de l'hôte dit l'heure et l'auteur | ajouté : rag3weaver, après les binaires |
| 7 nov. — deux écrivains sous un COPY de relations | déjà dit : racine, *Several writers at once* |
| 7 nov. — la recherche surligne le vecteur, filtre juste, prompt collé | déjà dit : rag3weaver, *Search* |
| 7 nov. — une lecture survit à l'échec d'une instruction | déjà dit : racine, *Reads that hold together* |
| 8 nov. — le code se colore dans le terminal | déjà dit : rag3weaver, *The IDE and the terminal* |
| 7 nov. — le Daemon Code Summoning | ajouté : rag3weaver, *The IDE and the terminal* |
| 7 nov. — le terminal de l'agent a sa vue | déjà dit |
| 7 nov. — l'équipier depuis un terminal (`--plain`) | déjà dit |
| 7 nov. — chercher sans avoir ouvert un dossier | ajouté : rag3weaver, *The IDE* |
| 7 nov. — l'IDE entier en React | déjà dit |
| 7 nov. — un lecteur ne voit plus les lignes annulées | déjà dit : racine, *Readers that never wait* |
| 7 nov. — un appel C++ ne voit que ce qu'il inclut | déjà dit : rag3weaver, tableau et *The code graph* |
| 7 nov. — la mémoire parle à Claude Code | déjà dit : rag3weaver, *Claude Code and Codex* |
| 7 nov. — Claude parle au trait Llm | déjà dit : rag3weaver, tableau et *The agent* |

**Le compte attendu d'un équipier** : 12 lignes à ajout, dont 2 en partie déjà dites, pour
11 ajouts (le quota du 8 avec la ligne du 9) ; 17 « déjà dit » ; 4 « interne ». Les noms cités (`task_writes`, `git_init`,
`list_worktrees`, `extension_error`, « racine absente », `decisions.jsonl`, `~/rw-code`)
sont vérifiés dans le code. Une tentative qui propose un ajout pour une ligne « déjà dit », ou un
nom absent du code, se compte en faute.

