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

## Le dénouement (11 octobre, 18 h 40)

Arrêt forcé par Lucie après 42 minutes (`kill -9` de l'hôte, toujours « deactivating ») : le seul
`.wal` était celui de `code-bascule`, la base à jeter (mise de côté dans
`code-bascule.retire-18h40/`) ; les bases gardées (`code-principal`, la boutique de G) avaient été
fermées proprement à 17 h 58 min 39 s, sans journal. L'unité relancée (MainPID 4051274, `/sante` à
18 h 40) : `code-principal` rouverte à la première demande, `usages update` sur
`node_table.cpp` rendu, et le fichier d'essai de Lucie (`essaiIndexationAChaudDixNeufHeures`) vu
par la montre. Ma session ne pouvait pas forcer l'arrêt (refus du mode automatique) : c'est
Lucie qui l'a fait.
