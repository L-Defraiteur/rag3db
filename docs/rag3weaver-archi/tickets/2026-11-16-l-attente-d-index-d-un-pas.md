# L'attente d'index d'un pas : synchrone sous six secondes, une notification au-dessus

- **État** : ouvert, le 16 novembre 2026 ; à faire par Codex (passation d'archi).
- **Famille** : produit
- **Pour** : archi (passation à Codex)
- **Gravité** : confort : Lucie ne veut pas être re-notifiée à chaque changement que l'agent cause lui-même.
- **Atteignable en service** : non, à construire.
- **Touche rag3weaver** : oui
- **Signalé par** : Lucie et c7, point 7 de la vision du 15 novembre

## Ce qu'il faut faire

La conception : `docs/rag3weaver-archi/15-11-2026/12-conception-l-attente-d-index-du-pas.md`. Le
contrat de recherche est sur master (`8b7686025e`) : `crate::catalog::retard::Lag::estimate(&cell)`
et `Lag::wait_caught_up(&cell, deadline)`, seuil `manifest.search.index_wait_s` (6). La part à faire
est le pas : sous le seuil, attendre en silence ; au-dessus, répondre `index not ready, about N s` et
**une seule** ligne de l'hôte à l'index prêt ; rien pour les réindexations des écritures du pas.

## Comment le prouver

Les trois témoins de la page, avec une estimation fournie par le témoin. Passe comme le plafond, plus
`e2e_lectures`.

## Taille et blocage

1 jour. Aucun (le contrat existe).

