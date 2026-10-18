# L'arrêt de l'hôte attend sans borne une réaction figée : TERM ne le fait jamais sortir

- **État** : corrigé (18 octobre 2026, G, le lot de l'hôte par socket) — l'arrêt des réacteurs est borné (`ReactorHandle::stop_within`, `RAG3WEAVER_STOP_WAIT_S`, 30 s) ; au-delà, le réacteur figé est nommé et lâché, et le backend ne ferme pas le catalogue qu'il tient peut-être (« catalogue laissé ouvert, rejoué à la prochaine ouverture ») ; l'hôte sort. Témoin `reactor::tests::a_frozen_reaction_does_not_hold_a_bounded_stop`. L'unité porte `TimeoutStopSec=90` en filet. L'interblocage du moteur reste dans son propre ticket
- **Gravité** : blocage. L'hôte ne sert plus et ne se remplace pas ; seul un `kill -9` le fait
  sortir, au prix du journal d'écriture des bases ouvertes.
- **Atteignable en service** : oui, dès qu'une réaction de la montre se fige.
- **Touche rag3weaver** : oui (`Backend::shutdown`, `stop_reactions`, `ReactorHandle::stop`).

## Ce que c'est

`Backend::shutdown` (`extension/rag3weaver/src/backend.rs`) appelle `stop_reactions`, qui lève le
drapeau d'arrêt puis attend chaque réacteur par `ReactorHandle::stop`
(`src/dataflow/reactor.rs`). Cette attente n'a **aucune borne**. Le drapeau ne se lit qu'« au
prochain fichier » : une réaction figée au milieu d'un fichier ne le lit jamais, et l'arrêt attend
pour toujours. L'unité systemd reste alors en `deactivating (stop-sigterm)`.

## Le relevé (luciepc, 18 octobre)

- `systemctl --user stop rag3weaver-daemon` à 15 h 39 min 05 s, hôte pid 1043752 (binaire
  `3ea1d5a7`, master `6041f9194`, lib `0fc7dd10f`). Aucun client MCP connecté ; le dernier index
  (`code-principal`) avait fini à 15 h 06, et le `.wal` de `code-principal` datait de 15 h 08.
- À 16 h 05 et après : toujours en `stop-sigterm`. 81 fils, tous en `futex_wait`, rss 55 Mo.
- La pile (Lucie) : le réacteur est figé dans une réindexation de la montre (`oublier les arêtes
  du fichier`), sur un rwlock de `ChunkedNodeGroup` que plus aucun fil vivant ne tient, et
  l'arrêt attend ce réacteur. Le figement date sans doute d'après 15 h 08, donc d'avant l'arrêt.
- **Sortie : `kill -9` par Lucie à 16 h 27 min 48 s**, après 48 min d'attente. Le nouvel hôte
  (binaire `da216050`, lib `87a362e30`) a démarré dans la même seconde, pid 1292809. À la
  réouverture, `code-principal` s'ouvre sans erreur avec son `.wal` de 6,4 Mo (15 h 08 min 38 s) ;
  `usages stop_reactions` rend 1 définition et 20 appels, et les index File, Library, Scope et
  Symbol sont `ready`. Aucune réindexation n'a été nécessaire.

## Recette

Sans Cypher. Figer une réaction (dans un témoin : une réaction qui attend un verrou qu'on ne rend
pas), puis appeler `Backend::shutdown` : l'appel ne rend jamais la main.

## Témoin

À écrire, dans `backend_reactions_tests.rs` : une réaction bloquée sur un canal jamais servi, puis
`shutdown` sous un délai. Le témoin attend un retour dans le délai et une erreur nommée.

## Ce que serait le correctif

- Borner l'attente de `ReactorHandle::stop` dans `shutdown`. Au-delà du délai (de l'ordre de la
  minute, réglable), dire la réaction figée et ce qu'elle faisait, puis fermer quand même ce qui
  peut l'être sans elle.
- Ou rendre l'arrêt de la réaction interruptible ailleurs qu'entre deux fichiers.
- La base dont la réaction est figée ne se ferme pas proprement : le dire, et la laisser à la
  reprise du prochain démarrage, au lieu d'attendre.
- L'unité de luciepc pourrait aussi porter un `TimeoutStopSec` explicite : c'est un filet, pas
  un correctif, puisqu'il finit en SIGKILL.

## Pour le fermer

Le témoin vert sous délai ; un arrêt qui rend la main en dépit d'une réaction figée, en la nommant.
L'interblocage du moteur se ferme dans son propre ticket.
