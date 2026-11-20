# Un scope `variable` recopie la ligne de ses frères

- **État** : fermé le 20 novembre 2026, **faux défaut**. Voir « Fermeture ».
- **Famille** : produit (l'analyse du code)
- **Gravité** : un texte embarqué deux fois, et une ligne de bruit qui gagne des recherches par le sens. Après
  l'exclusion des déclarations courtes (lot `variable-not-candidate`, règles 13), c'est la copie longue qui remonte.
- **Atteignable en service** : oui, sur la base de code-principal (luciepc).
- **Touche rag3weaver** : oui (les scopes que l'analyse écrit).
- **Pour** : codeparsers
- **Signalé par** : le banc (juge du 20 novembre 2026), lu par ED.

## Ce qu'on voit

Une ligne Python qui enchaîne des affectations (`manifest=…;cards=…;owned=…`) porte plusieurs scopes `variable` sur
la même ligne, de deux sortes :
- chaque affectation a son scope, avec **son seul texte** : 26 à 59 caractères pour
  `experiments/mtga/scripts/prepare_engine_relations.py:14` ; 19, 22 et 109 pour
  `experiments/mtga/scripts/build_plant_variant.py:32` ;
- mais un scope `variable` a pour contenu **la ligne entière** : 220 caractères et 152 caractères.

La ligne entière recopie le texte de ses frères. Elle est embarquée en plus d'eux, et sous la règle des déclarations
courtes, c'est elle qui gagne :
- « rebuild the vector index » (python) rend en tête `build_plant_variant.py:32`, la ligne du 88 % de luciepc ;
- « parse a json manifest » rend `prepare_engine_relations.py:14`.

Les relevés sont à luciepc : `banc-parite/sorties/feuille-5d-20261120/trois-premiers.json`. L'analyse est dans
`docs/everything-declarative/20-11-2026/04-les-trois-premiers-changes-par-le-sens.md`.

## Ce qu'il faut pour le fermer

Savoir d'où vient le scope de la ligne entière (une affectation multiple vue comme un seul `variable` ? un repli de
l'analyseur Python ?), et qu'aucun scope ne porte le texte de ses frères : un témoin rouge sur la ligne
`index = …; s = index.read_text(); …`, qui ne doit avoir que des scopes à texte propre.

## Fermeture (20 novembre 2026, soir, session everything declarative)

**Faux défaut.** Le banc a corrigé sa lecture : les scopes de la ligne entière (220 et 152 caractères) sont des
**versions fermées** d'une analyse antérieure au bornage aux octets (`_depuis` 0, `_jusqu_a` 1), et ils n'ont pas été
rendus. Les résultats en tête étaient des déclarations courantes et courtes (`manifest`, `index`). Codeparsers a joué
l'analyse courante sur les deux fichiers : aucun scope ne recopie la ligne de ses frères. Les générations 0, 1 et 2 sont
publiées, et la recherche ne lit pas de version fermée.

La vraie cause est le revers de la décision (A) : ticket
`2026-11-20-une-declaration-courte-prend-la-tete-par-un-mot.md`, fermé par (A'). La page
`20-11-2026/04-les-trois-premiers-changes-par-le-sens.md` est corrigée.
