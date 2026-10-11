# L'arrêt de l'hôte attend la fin d'une réaction de la montre

- **État** : ouvert (11 octobre 2026) — constaté sur l'hôte de luciepc (session codeparsers)
- **Gravité** : blocage (un redémarrage prend des minutes, sans borne ; tous les backends de l'hôte restent indisponibles pendant ce temps)
- **Atteignable en service** : oui, dès qu'un arbre indexé change beaucoup d'un coup (un worktree effacé, un `git checkout` d'une autre branche)
- **Touche rag3weaver** : oui (hôte, réactions, `ReindexFileNode`)

## Ce que c'est

Le worktree `rag3db-bascule`, indexé par le backend `code-bascule`, a été effacé. La montre a vu
~6 000 fichiers disparaître, et la réaction `reindex_on_change` les retirait par lots en tenant
le catalogue : `usages` répondait « Catalogue occupé » pendant ce temps. Le manifeste retiré,
`systemctl --user restart rag3weaver-daemon` à 17 h 58 : SIGTERM reçu (« arrêt propre, comme
POST /arret »), puis l'unité est restée « deactivating » plus de six minutes, l'hôte écrivant
toujours dans `code-bascule.rag3db` (`TimeoutStopUSec=infinity`).

Une indexation s'arrête au prochain paquet validé (`stop_indexings_of`) ; une réaction
(`stop_reactions`), elle, semble aller au bout de son lot d'événements (jusqu'à 1 000) avant que
l'hôte ne ferme.

## Ce qu'il faut pour le fermer

- `ReindexFileNode` lit un drapeau d'arrêt entre deux fichiers (comme l'indexation entre deux
  paquets) ; les événements non traités restent dans le curseur, la réaction reprendra là.
- Une réaction qui retire plus de N fichiers d'un coup (un arbre disparu) ne retire pas fichier
  par fichier : elle constate que la racine n'existe plus, le dit, et suspend le backend.
- Témoin : un arbre indexé, la montre active, la racine effacée, puis SIGTERM ; l'hôte s'arrête en
  quelques secondes et la base se rouvre.
