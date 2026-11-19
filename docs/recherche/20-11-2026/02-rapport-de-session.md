# Rapport de session recherche, du 19 au 20 novembre 2026

Ce qui est fait, ce qui attend, ce qu'on a appris. Les lots sont nommés par leur id au pousseur.

## Atterri, ou en file

| Lot | Ce qu'il fait | État au 20 nov., 00 h 30 |
|---|---|---|
| `recherche-revert-c-2` | le revert de C (relations avant les dérivations), qui rougissait 4 cas `en_fichiers` | sur master, 4c0728e703, PR 19 |
| `recherche-c-durable-3` | C revient, durable : marque `derivations_held` dans `_catalog_meta` ; un écrivain rend ce qu'une passe morte a retenu ; « à rebâtir » tant que la marque reste. `usages` en release : 23,9 → 12,2 s sur rag3weaver/src, 74,9 → 40,5 s sur le dépôt | en file |
| `recherche-cas-de-campagne` | deux cas de campagne : la déclaration d'un conteneur retiré, la lecture qui se taisait pendant l'index | sur master (PR 27) |
| `recherche-nouveaute-usages` | la ligne d'usages sous générations dans les nouveautés | sur master (PR 42) |
| `recherche-residus-4b` | les quatre dernières suites à résidus propres, et l'écrivain affamé qui écrit dans un TempDir du cache | en file, derrière `recherche-residus-e2e-code` |
| `recherche-interruption-2` | un arrêt de run coupe la requête de son instantané, jamais la connexion partagée ; `RevocableConnection` relaie `interrupt` | en file |
| `recherche-attente-vecteurs` | une recherche qui lit les vecteurs attend aussi les vecteurs dus (texte et vecteurs à part) ; sans service, pas d'attente et la cause dite | en file, sur `arbre-estimation-hote` |
| `recherche-notification-index` | une seule notification quand l'index d'une vue « pas prête » est prêt : veille du client mince, `POST /index_state`, `notifications/message` | en file |
| `recherche-notification-e2e` | le témoin par le binaire réel (faux hôte par route, `--daemon`) | en file |
| `recherche-filtre-vide` | un filtre e2e qui ne désigne aucun test est refusé | sur master, f6eca5f135, PR 82 |
| `recherche-cas-de-campagne-03` | troisième cas, sans lib : le filtre e2e qui ne désigne rien | en file ; le lanceur apprend les témoins en script (archi) |

## Ce qu'on a appris

- **Un témoin sous le chemin de l'hôte peut être vert pendant que le service ne fait rien.** L'interruption était
  verte sur l'instantané, mais par `call_in` puis `stop_run`, l'appel allait au bout. L'enveloppe `RevocableConnection`
  ne relayait pas `interrupt`. Seul le témoin par l'hôte l'a vu.
- **Un rouge doit l'être pour la bonne raison.** Un `count(*)` de deux UNWIND finissait en 42 ms, avant
  l'interruption ; un `range(1, 2e9)` se tient en mémoire et faisait tuer le moteur. La requête retenue somme
  `x * y % 13` sur quatre milliards de lignes.
- **Deux passes ne partagent pas un worktree** : une mesure de l'écrivain affamé a joué les commits d'une autre passe
  qui y faisait son checkout.
- **On ne tue que l'arbre de ses propres pids**, jamais par un motif de commande que d'autres peuvent porter.
- **Effacer : la règle du 18 novembre prime** (au oui de Lucie), même pour des résidus de nos propres tests.
- **Les worktrees vont sous `~/git_workspaces/<session>/<jj-mm-aaaa>-<lot>`** et partent à l'atterrissage du lot.
  `git worktree remove --force`, qu'exige le sous-module, est refusé à cette session : son chemin va à c7.

## Ce qui attend

- **Le rouge intermittent d'`e2e_mcp_hote_designe`** (« descendants laissés », tests verts) : ticket ouvert, journal
  gardé à luciepc (`~/.cache/rag3weaver-build/gardes-recherche/2026-11-19-e2e-mcp-hote-designe-descendants`).
  Départager à la prochaine occurrence.
- **L'attente courte des vecteurs qui aboutit** (des vecteurs calculés pendant l'attente) n'a pas de témoin : aucun
  embarqueur ne tourne en fond dans le backend de test.
