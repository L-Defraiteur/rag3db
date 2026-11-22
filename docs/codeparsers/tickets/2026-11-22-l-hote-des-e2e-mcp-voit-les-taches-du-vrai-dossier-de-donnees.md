# L'hôte des e2e MCP voit les tâches du vrai dossier de données

- **État** : ouvert (22 novembre 2026)
- **Famille** : produit
- **Gravité** : moyenne. Deux témoins rougissent pour une raison qui n'est pas la leur, selon ce que le poste a créé
  avant eux. Un test d'hôte qui lit les vraies tâches de Lucie n'est pas isolé.
- **Atteignable en service** : non (les e2e) ; mais un témoin qui dépend de l'état du poste ne prouve rien
- **Touche rag3weaver** : oui (l'hôte du démon dans les e2e, son dossier de données)
- **Pour** : session archi (l'isolement du dossier de données de l'hôte dans les e2e), selon c7
- **Signalé par** : session codeparsers, à la vérification du lot `include-members-2`, à luciepc

## Constat

Le 22 novembre, entre 2 h et 5 h, à luciepc, deux témoins d'`e2e_mcp_stdio` rougissent :
- `le_meme_client_mcp_par_l_hote_du_demon` (`tests/e2e_mcp_stdio.rs:196`, « un backend ouvert par l'hôte ») ;
- `deux_sessions_mcp_partagent_un_backend_par_l_hote` (`tests/e2e_mcp_stdio.rs:222`, « un seul backend, ouvert une
  fois »).

Tous deux comptent les backends que l'hôte du test a ouverts : `left: Some(6)`, `right: Some(1)`. Les cinq en trop ont
leur manifeste sous le vrai dossier de données du poste :
`/home/lucied/.local/share/rag3weaver/taches/<id>/doc/arbre/…` (par exemple `taches/40c621863fa1/…` et
`taches/1ba6f87ec92f/…`). Ce sont des tâches créées cette nuit par d'autres passes. L'hôte du test lit donc le dossier
`taches/` réel, au lieu d'un dossier de données à lui.

Le lot vérifié ne touche ni l'hôte ni les tâches : `NeighborhoodNode` et trois descriptions de gabarit. Dans la même
passe, `un_client_mcp_voit_les_outils_du_manifeste_et_les_appelle` est vert.

## Témoin attendu

Un dossier `taches/` peuplé dans le vrai dossier de données du poste (ou une variable qui le désigne). Avec lui,
`e2e_mcp_stdio` voit un seul backend : l'hôte du test a son propre dossier de données, et ne lit jamais celui du poste.
