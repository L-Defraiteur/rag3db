# Le témoin de la carte de progression tombe sous la charge d'une lib entière

- **État** : ouvert (22 novembre 2026), arbre principal ; lot `progress-card-fiable`.
- **Famille** : produit
- **Gravité** : un faux rouge au pousseur. Le lot `recherche-search-words-built` est tombé à 07 h 01 sur ce seul test,
  sans y toucher, et chaque lot Rust de la nuit peut retomber dessus.
- **Atteignable en service** : non (un test de la lib).
- **Touche rag3weaver** : oui (`src/daemon_host/worktrees.rs`,
  `a_progress_card_reaches_the_page_and_never_the_model`).
- **Pour** : arbre principal, au mot de c7 du 22 novembre (« même famille que view_follows »).
- **Signalé par** : c7, journal `luciepc:~/.cache/rag3weaver-build/pusher-20261122-063932-690-recherche-search-words-built/lib.log`.

## Ce que c'est

- Le témoin attend bien un événement : l'entrée de la vraie passe de vecteurs dans l'embarqueur contrôlé, qui la
  retient derrière une barrière le temps des assertions sur la carte. Mais il ne lui laisse que 10 s, et ces 10 s
  couvrent aussi l'attache du dépôt et tout le plein texte d'avant. Sous trois réindexations en parallèle, la passe
  n'arrivait pas à la barrière à temps : « la vraie passe de vecteurs a atteint la barrière: Timeout »
  (`worktrees.rs:2247`).
- La même borne de 10 s attendait la fin de l'index après la libération, et l'embarqueur contrôlé rendait `Timeout` au
  bout de 30 s de barrière fermée : deux autres chutes possibles sous la même charge.

## Ce qu'il faut

- Garder ce que le témoin prouve : la carte va à la page, jamais au modèle, pendant qu'un vrai index tourne.
- Ne borner le temps que pour ne pas pendre : 120 s pour l'arrivée à la barrière et pour la fin de l'index, et 300 s
  de barrière fermée dans l'embarqueur contrôlé.
