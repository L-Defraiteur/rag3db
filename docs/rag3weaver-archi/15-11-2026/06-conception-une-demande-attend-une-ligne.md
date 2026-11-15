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
