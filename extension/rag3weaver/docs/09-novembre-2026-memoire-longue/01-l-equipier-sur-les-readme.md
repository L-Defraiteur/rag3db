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
