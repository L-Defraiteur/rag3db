# Le côté « pas » du socle d'agent de code : la conception

15 novembre 2026, 17 h. C'est la part d'archi du [socle](07-le-socle-d-un-agent-de-code-dans-une-tache.md),
celle qui ne dépend pas des primitives d'ED (c7). La référence est la
[vision du 15 novembre](../../orchestration/visions/15-11-2026-l-agent-est-un-agent-de-code-qui-gere-ses-worktrees.md).
Le lot est à part, posé sur la garde de l'équipier (`une-demande-attend-une-ligne`, `a1a74f661`).
Il n'est pas poussé avant son tour.

## La conception, en quatre lignes

1. **Le droit d'écrire d'une tâche se lit, il ne se fige pas (d), et son refus le dit (c).**
   - `surcharges_de` recalcule `ecrit` à chaque ouverture, sur le manifeste parent du moment
     (`Tache.parent_manifest` : `!read_only || task_writes`). Le registre n'en garde qu'un
     souvenir.
   - Dans une tâche, le refus de `ReadOnly` dit « cette tâche n'a pas le droit d'écrire : son
     backend parent ne le permet pas (workspace.read_only sans task_writes) », et ne conseille
     jamais `POST /ide/taches`. Le message d'aujourd'hui reste celui du backend principal.
2. **Le bac Landlock d'une tâche lit `info/` du dossier git commun (b)**, en plus de `objects`,
   `refs`, `HEAD`, `config`, `packed-refs` et `shallow` : `git status` y lit `info/exclude`. Le
   dossier commun entier reste fermé, puisqu'il contient les autres worktrees.
3. **Le pas parle comme un agent de code.**
   - L'invite d'un pas commence par celle de l'agent de code (`apps/code/chat.json`) et dit :
     « tu parles à l'utilisateur ; une réponse par tour, en texte, sans te citer ni dire
     « rapport fait » ». La boucle de l'équipier (plans, `step_done`, la boîte) vient ensuite, plus
     courte.
   - Pour un backend de code, les outils de déclaration (`declare`, `declare_set`, `propose`)
     passent en bas de la liste.
   - La description de `run_command` dit de lancer `git status` nu, sans inventer d'option
     (`git -c …`).
4. **Rouge d'abord, un témoin par point** :
   - (d) une tâche ouverte quand le parent interdisait d'écrire, puis rouverte après
     `task_writes` : elle écrit ;
   - (c) le texte du refus dans une tâche ;
   - (b) `git status` sous le bac d'un worktree, code 0 ;
   - l'ordre de l'invite et sa phrase ;
   - l'ordre des outils ;
   - la description de `run_command`.

   La passe : ces témoins, ceux de l'équipier et du chat, la lib entière, check_features et
   e2e_equipier, sous `timeout`.
