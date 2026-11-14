# Les dix premiers de search_code rendent plusieurs copies d'un même contenu

- **État** : ouvert (14 novembre 2026, banc, à la demande de l'orchestration), pour arbre principal.
- **Gravité** : coût. La réponse n'est pas fausse, mais une recherche qui rend plusieurs fois le
  même script perd autant de places parmi ses dix premiers.
- **Atteignable en service** : oui. Le dépôt rag3db garde de nombreux scripts recopiés d'un dossier
  de docs à l'autre (`docs/<session>/<jour>/<essai>/…py`).
- **Touche rag3weaver** : oui (la présentation des résultats de `search_code`).
- **Famille** : produit

## Ce que c'est

Parmi les dix premiers résultats d'une requête, plusieurs lignes portent le même contenu
(`_content_hash`) dans des fichiers différents. Le cas est plus fréquent sur la base à versions
que sur la base d'avant : les occurrences d'un même contenu y partagent la même version, donc le
même score lexical, et se suivent dans le classement.

## Les chiffres

Le juge de parité de `search_code` (branche `banc-parite-search-code`), sur le jeu commun de
2a′-2 : 10 requêtes, 3 formes (sans filtre, filtrée par `src/include/`, filtrée par `python`),
k = 10. Les deux bases sont R, la base d'avant les versions (instantané avant-835d5298f), et la base
migrée par 1b (migree-20261114-1641). Les relevés sont à
`luciepc:~/.cache/rag3weaver-build/banc-parite/sorties/{ref,migree}.json`.

| base | places des dix premiers prises par une copie d'un contenu déjà rendu dans la même liste | cas touchés |
|---|---|---|
| R | 16 sur 289 | 5 sur 30 |
| migrée | 29 sur 289 | 9 sur 30 |

Exemple sur R, « value », filtré par langage python : le même contenu (`values`) revient depuis
`docs/arbre-principal/12-11-2026/preuves-faits-lies-v1/passe.py`, `…-v4/passe.py`,
`docs/arbre-principal/12-11-2026/preuves-observation-ro/globale.py` et
`docs/arbre-principal/13-11-2026/raccord-physique/globale.py`.

## La forme proposée (par l'orchestration)

Avec les versions, rendre **une version avec ses occurrences dessous** plutôt que chaque occurrence
comme une ligne. C'est la projection version → occurrences de 1b, du côté de la présentation : les
dix premiers deviennent dix contenus distincts, chacun avec la liste de ses chemins.

## Témoin

À écrire avec le correctif : sur la base migrée et le jeu commun, aucune liste des dix premiers ne
porte deux fois le même `_content_hash`, et chaque résultat rend toutes ses occurrences. Le juge de
parité, au régime « migration », dit ce que le changement fait à l'ensemble.

## Pour le fermer

La forme choisie par arbre principal, le témoin vert, et le relevé du juge de parité avant et après.
