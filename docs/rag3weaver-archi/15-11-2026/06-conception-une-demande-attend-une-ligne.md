# Une demande attend au moins une ligne de l'équipier : la conception

15 novembre 2026, 13 h. Ce lot répond au
[ticket rouvert](../tickets/2026-11-14-l-equipier-rend-un-compte-rendu-au-lieu-du-contenu-demande.md)
(relevé de tty, run `agent-1a1df060b51-346`) et suit la forme de c7 (12 h 4x) : après deux outils
refusés, l'équipier finit sur un `step_done` qui dit « exemple TypeScript fourni », sans rien dans
la boîte.

## La conception, en quatre lignes

1. **Le défaut change.** Une demande sans `expects` attend au moins une ligne de l'équipier dans la
   boîte depuis elle : un `report`, une `question` ou une `approval`. Le texte d'un pas, acheminé
   par le correctif du 14, arrive en `report` et compte. Un `step_done` ne compte pas.
   `expects: "none"`, déclaré, garde le comportement silencieux d'aujourd'hui : `EXPECTS` passe à
   trois valeurs (`report`, `approval`, `none`), et une autre valeur reste refusée en les nommant.
2. **Le refus est une relance, une fois par demande.** Dans la branche de fin
   (`teammate.rs`, après `missing_deliverable`), si rien de l'équipier n'est dans la boîte, l'hôte
   écrit une ligne : « relance : ta demande attend une réponse dans la boîte ; ton step_done dit
   « <done> », et rien n'y est arrivé ; écris le contenu demandé (report) ou dis pourquoi
   (question) ». Le pas suivant part avec cette ligne dans son message de frontière.
   - La relance ne compte ni comme un pas raté (`failed` est inchangé) ni comme un pas sans suite
     (`stalled` aussi).
   - La tâche ne finit pas sur elle.
3. **Une seconde fois sans rien** : la garde du livrable prend le relais. La boîte dit « fait sans
   le livrable demandé (une ligne de l'équipier dans la boîte) : j'attends le superviseur ».
   L'équipier attend sans rappeler le modèle, et une ligne du superviseur le relance, comme sous
   `expects: "report"`.
4. **Rouge d'abord, sur le faux modèle** (`CallbackLlm`, `teammate_tests.rs`) :
   - le cas du relevé : deux `read_file` refusés, puis un `step_done` sans texte. La relance est
     dite et cite le `step_done`. Au pas suivant, un texte arrive dans la boîte, puis la tâche
     finit « fait sans plan », sans pas compté raté ;
   - toujours rien après la relance : l'attente du superviseur, et aucun appel au modèle pendant
     trois secondes ;
   - `expects: "none"` : la tâche finit sans ligne et sans relance ;
   - `expects` accepte ses trois valeurs et refuse une autre en les nommant.

   La passe : ces témoins, la lib entière, check_features, puis e2e_equipier. Le changement de
   défaut va dans les nouveautés. tty rejoue son relevé sur le binaire d'après.

## Les deux relevés réels

| | Relevé de tty | Relevé de Lucie |
|---|---|---|
| Quand | 15 novembre, matin | 15 novembre, après-midi, vu par Lucie elle-même |
| Tâche | `essai-tty`, hôte 7970 du portable, binaire 26a091474 | `essai-tty`, hôte 7970 du portable, binaire 5b8f8994f |
| Run | `agent-1a1df060b51-346` (`runs/3ab508104e74/`) | `agent-1a1dfaad199-200` (`runs/3ab508104e74/`) |
| Demande | lire deux fichiers, puis « réponds-moi … avec un court exemple de classe TypeScript » | « qu'est-ce qu'on sait sur les orphelins du plein texte ? » |
| Ce que fait l'équipier | deux `read_file` refusés, huit explorations | 12 appels d'outils, des lectures (le ticket lucivy du plein texte, `catalog.rs` lignes 6945-6979…) |
| Le `step_done` | « Lecture … tentée (fichiers absents du dépôt) et exemple TypeScript fourni. » | « fait : Synthèse documentaire détaillée sur les orphelins du plein texte (définition, mesures chiffrées par base, causes identifiées et questions ouvertes). » |
| Ce qui est dans la boîte depuis la demande | rien | rien : la boîte (`taches/12389e156742/essai-tty/mailbox.jsonl`) n'a que la demande, le `step_done` et « fait sans plan » |
| Vérifié par archi dans le run | aucun `report`, aucun bloc typescript, aucune classe | 12 appels, 0 `report`, 0 `edit_file` |

Dans les deux relevés, l'équipier n'a rien laissé : ni ligne dans la boîte, ni fichier écrit, ni
commande, ni plan clos. C'est le cas que la forme B retient, et rien de plus large.

## Ce qui a changé depuis la première forme (c7, 14 h 4x)

- **La forme B** : la relance ne joue que si l'équipier n'a rien laissé depuis la demande. La trace
  (un fichier écrit, une commande, un plan clos) se lit dans la politique que le manifeste déclare
  pour chaque outil abouti (`write_files`, `run_commands`, `attach_worktrees`). Elle est **sur
  disque**, dans `request.trace` à côté du curseur de la boîte : un redémarrage de l'hôte ne la perd
  pas.
- **L'attente du superviseur est bornée** par `teammate.reply_wait_s` (1800 s par défaut). Au-delà,
  la tâche finit « sans réponse de l'équipier … (teammate.reply_wait_s) », un échec nommé.
- **Cinq témoins d'avant déclarent `expects: "none"`** :
  `a_backend_closes_between_two_tool_calls_of_a_step`, `a_provider_refusal_waits_a_window…`,
  `a_failed_step_does_not_end…`, `a_step_done_that_says_what_remains…` et
  `a_teammate_resumes_after_the_host_restarts`. Leur faux modèle suit, par construction, le motif
  des relevés (des lectures, puis un `step_done` sans rien), et leur sujet est ailleurs : une
  fermeture, un refus, un compte, ce qui reste, un redémarrage. C'est leur contrat réel.
