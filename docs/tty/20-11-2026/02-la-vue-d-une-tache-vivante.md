# La vue d'une tâche vivante, et ses accords répondus depuis la page

Session tty, 20 novembre 2026, à la demande de c7. C'est le lot qui précède le kanban des tickets.

## L'invariant de Lucie

Une session au terminal et l'IDE se retrouvent toujours dans le même backend, dans les deux sens, avec l'hôte pour seule source. Un run lancé au terminal est visible et joignable dans l'IDE ; un run de l'IDE l'est au terminal.

## Ce que fait le lot

- **La page d'une tâche suit son équipier au flux.** L'hôte publie chaque ligne de boîte (`MailboxLine`) sur le sujet `mailbox`, et le worker de l'IDE la reçoit désormais. Pour chaque ligne de sa tâche (le manifeste de la tâche, et `task`), la page relit `GET /ide/teammate` : la boîte, les plans, et ce qui attend. Elle ne fait aucune requête au repos.
- **Ce qui attend le superviseur vient de l'hôte.** `GET /ide/teammate` rend `waiting`, la question ou l'approbation ouverte, calculée par `teammate::open_question`, le même calcul que l'équipier lui-même. La page ne recalcule rien.
- **Une carte pour ce qui attend.** Une approbation devient une carte (`#attente-equipier`) avec la commande et les boutons « Oui » et « Non ». Une question devient une carte avec un champ de réponse. La page poste une `answer` (`reply_to`, et `approved` pour une approbation), exactement comme le o / n du terminal. La carte part une fois répondue.

## Le témoin

`daemon_host::teammate_tests::the_page_of_a_task_follows_its_teammate_and_answers_its_approval`, un témoin navigateur, sur le banc de la commande retenue par la porte :
1. la page de la tâche « doc » est ouverte avant la demande ;
2. l'équipier lance `echo accorde-moi`, que la porte retient ;
3. sans rechargement, la carte d'accord paraît, et la ligne `approval` est dans la boîte ;
4. le « Oui » de la page fait jouer la commande au pas suivant, et la carte part.

- **Rouge** à luciepc (9240978d68) : la page ne montrait pas la commande retenue.
- **Vert** deux fois sur deux, après correction du témoin : l'apostrophe de « l'accord » fermait la chaîne JavaScript de son sélecteur, et la condition ne pouvait jamais être vraie. Sont verts aussi `ide_par_le_flux`, le contrôle du bundle, les deux fenêtres, le compte des accords, et tout `daemon_host::teammate_tests`.

## Ce qui reste

- La fenêtre « mémoire à côté » : /memory et /turns dans l'IDE (`/ide/l1`, `/ide/turns/read`).
- Les pas de l'équipier en liste qui se coche (fait, en cours, à venir), emprunt de c7 à Claude Tag. Les plans s'affichent aujourd'hui en cartes.
- Trouver l'agent qui tient un run : le registre d'archi (son lot 6) et le troisième préalable du kanban.
- Le distant (le téléphone) : une écoute authentifiée ou un tunnel, à cadrer avec G, jamais un second backend.
