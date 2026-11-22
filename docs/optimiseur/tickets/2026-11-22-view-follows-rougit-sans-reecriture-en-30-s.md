# Le témoin de la vue suivie rougit sans aucune réécriture en 30 s

**Ouvert le 22 novembre 2026, par G (optimiseur).**

## Le constat

`daemon_host::invocation::tests::view_follows::a_write_in_a_views_worktree_is_followed_by_a_kernel_event_and_seen_under_the_view`
rougit de temps en temps dans la lib, sous le pousseur, sur des lots qui ne touchent ni les vues ni l'invocation :

| passe du pousseur | lot |
|---|---|
| `pusher-20261122-015851-539-user-token-2` | `user-token-2` |
| `pusher-20261122-070304-263-recherche-search-words-built` | `recherche-search-words-built` |
| `pusher-20261122-124515-243-secret-reads` | `secret-reads` (rendu rouge, à redéposer tel quel) |

Le message est toujours le même : « the view's layer is not rewritten within 30s of a write in its worktree ».

## Ce que cela dit

Le test compte les réécritures de la couche, par événement noyau et par relevé, et attend jusqu'à 30 s. Il n'en voit
**aucune**, alors que le relevé lent tourne lui-même toutes les 30 s. Ce n'est donc pas une lenteur sous charge, comme la
borne de 2 s relâchée le 21 novembre : l'événement est perdu. Une hypothèse à départager : la surveillance (inotify) est
armée après l'écriture du test, ou à un moment où le fil du suiveur n'est pas encore prêt. Le relevé de 30 s, de son côté,
ne tombe pas toujours dans la fenêtre.

## Ce qu'il faut

1. Un témoin qui attend que la surveillance soit armée avant d'écrire (un état « suit » rendu par l'hôte), ou qui rejoue
   l'écriture tant que rien n'est vu.
2. D'ici là, ce test n'est pas une raison de rendre un lot rouge : le dire au lot qui le rencontre et le redéposer tel quel.
