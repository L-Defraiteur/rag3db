# tty-22 : le mode plain attend un run que l'hôte reprend après sa fin

Session tty, 18 novembre 2026. Ticket `docs/tickets/closed/2026-11-16-le-mode-plain-et-la-fin-d-un-pas-relance.md`, ouvert le 16 après le correctif T1 (tty-20).

## Le cas réel

En lisant l'hôte : la relance « ta demande attend une réponse » vient avant `ended`, dans la boucle de l'équipier ; elle ne trompait pas le mode texte. Le cas qui le trompe est autre. Une ligne du superviseur, arrivée pendant le dernier pas, fait repartir l'équipier juste après son `ended`, et la tâche repasse à `running`. Le mode `--plain` s'arrêtait au premier `ended`, et le journal perdait la réponse du run repris.

## Le correctif

Dans `src/main.rs`, la relecture de la boîte ne s'arrête plus au premier `ended`. Elle garde l'heure du dernier `ended` vu ; une ligne qui le suit l'efface. La demande est finie quand ce `ended` est la dernière ligne depuis 2 s et que la tâche n'est plus `running` (`GET /ide/teammate/list?supervisor=supervisor`, la tâche de ce dépôt), ou que l'hôte ne sait pas le dire. La règle est la fonction `run_is_over`, avec son test.

## Les témoins

- `mesures/plain_waits_for_a_resumed_run.py`, sur la route neuve `POST /faux/end-then-resume` du faux hôte (un `ended`, 3 s de tâche `running`, une réponse, puis le `ended` final). Vu rouge sur le binaire de master 81aa3d0d1e (« the journal does not carry the answer of the resumed run »), puis vert.
- `mesures/plain_stays_attached.py` (T1) reste vert.
- `run_end_tests::an_end_is_the_end_only_after_the_grace_and_when_the_task_no_longer_runs`.
- Le crate passe 114 + 2 tests, et 22 sans vue ; clippy `-D warnings` ne signale rien.
