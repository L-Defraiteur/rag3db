# Un nom tapé exactement n'est promu que s'il est déjà dans la liste bornée de la fusion

- **État** : fermé le 20 novembre 2026, par le lot `exact-name-by-name`.
- **Famille** : produit (la recherche)
- **Gravité** : une recherche par le nom exact d'une déclaration ne la rend pas, quand ce nom est courant (`lib`,
  `result`) ou que le plein texte le trouve dans beaucoup d'autres mots (`con` dans `release_connection`).
- **Atteignable en service** : oui (le dépôt de la tablette, après la re-bascule du 20 novembre).
- **Touche rag3weaver** : oui (`FuseResultsNode`, `exact_name` ; `templates/tools/search_workspace.mmd`).
- **Pour** : everything declarative
- **Signalé par** : le banc, par l'hôte de service, relayé par c7 (20 novembre 2026, soir).

## Ce qu'on voit

Cinq noms du dépôt (`__all__`, `lib`, `result`, `con`, `PORT`) sont absents du top 10, sans homonyme devant eux. Ce n'est
ni un fichier écarté, ni le rang publié, ni un plein texte retiré : une recherche bornée à leur fichier les rend
premiers. `lib` et `result` ont 13 et 32 homonymes dans le top 50 du BM25 seul, et aucun n'entre dans le top 10 par
défaut ; `con` et `__all__` ne sont même pas dans le top 50 du BM25. Une méthode `result` avec vecteur n'est pas promue
non plus : ce n'est pas la règle des déclarations courtes.

## La cause

La promotion du nom exact (décision (B), `exact_name`) ne voit que la liste fusionnée, bornée avant elle par le budget
de chaque signal (`(limit + offset) × 2`). Un nom courant, ou trouvé par le plein texte dans d'autres mots (le
`contains` à distance 1), n'y entre pas.

## Fermeture

Le nom exact se cherche par le nom : `ExactNameNode` lit directement les lignes dont le nom est la requête entière
(après trim), bornées à 20, sous le filtre de la requête (chemins, langages, celui de l'appelant), la cellule et la
génération visible, sur l'entité où vit le nom (`Scope`, l'occurrence) ; un conteneur qui replie des enfants n'est pas
lu par son nom (`leaf='folds'`). Il les donne à la fusion comme un signal (`name_exact`), et la promotion les met
devant. Déclaré dans `search_workspace.mmd`, livré par `builtin:` sans repassage.

Témoin `a_name_typed_exactly_is_found_by_the_name_beyond_the_bounded_list`, rouge avant le nœud (« con » ne rendait
pas `con` dans le top 3, quarante `release_connection_*` devant), vert avec. Les deux témoins des conteneurs repliés
restent verts. Le juge du banc, par l'hôte de service, porte les cinq noms du dépôt.
