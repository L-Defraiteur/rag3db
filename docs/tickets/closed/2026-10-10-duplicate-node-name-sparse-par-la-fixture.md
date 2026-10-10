# « duplicate node name: sparse » à la première recherche de test_backend_sparse

- **État** : corrigé `413e7ba93`
- **Gravité** : réponse fausse (la suite rougissait sur un défaut qui n'existait pas dans le moteur ni dans rag3weaver)
- **Atteignable en service** : non — le défaut vivait dans la fixture de la suite, pas dans un chemin de production
- **Touche rag3weaver** : les tests seulement (`scripts/test_backend_sparse.py`, et le filet ajouté dans `src/dataflow/graph_tool.rs`)

## Ce que c'est

La fixture de `test_backend_sparse.py` lisait le gabarit livré
`templates/tools/search_structured.mmd` et y **injectait** une branche sparse
par remplacements de texte — écrits quand le gabarit n'en avait pas. Depuis
que le gabarit livré porte SA branche sparse, l'injection la doublait :
« le graphe n'a pas pu être construit : duplicate node name: sparse » à la
première `search_notes`. Même classe que la fixture d'impact du 10 octobre :
un corpus vivant a bougé sous un collage. (Ouvert par la session de l'arbre
principal pendant l'enquête du ticket
`2026-10-10-tampon-de-256-mio-plein-a-la-premiere-ecriture`, sans fichier
propre ; celui-ci est né fermé.)

## La recette minimale

Le script d'avant (`git show 413e7ba93~1:extension/rag3weaver/scripts/test_backend_sparse.py`)
joué sur l'arbre du 11 octobre rougit mot pour mot
`duplicate node name: sparse` ; pas de Cypher en cause.

## Le témoin

- La suite réparée passe entière (`test_backend_sparse.py`, PASS, même arbre
  et même binaire que la preuve rouge).
- Le garde inverse dans la fixture : elle rougit si le gabarit livré PERD sa
  branche sparse.
- Le filet de classe : `chaque_gabarit_d_outil_livre_se_construit`
  (`src/dataflow/graph_tool.rs`) — les 31 fiches de `templates/tools` se
  construisent avec le registre réel d'un backend.

## La cause

Une fixture-collage sur corpus vivant : le gabarit a gagné sa branche sparse
(10 octobre) et les `str.replace` de la fixture en ajoutaient une seconde du
même nom. Le remplacement des poids (`bm25:0.5,vector:0.5`) ne correspondait
d'ailleurs plus à rien — dérive silencieuse du même collage.

## Ce qu'il a fallu pour le fermer

Prendre le gabarit livré TEL QUEL dans la fixture (il porte la branche), et
garder seulement le garde ; plus le filet de classe ci-dessus.
