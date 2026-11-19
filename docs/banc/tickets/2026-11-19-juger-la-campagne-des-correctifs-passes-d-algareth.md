# Juger la campagne des correctifs passés d'Algareth

- **État** : ouvert ; la page de forme d'abord, après les mesures du banc du 19 novembre.
- **Famille** : produit
- **Gravité** : moyenne. C'est la mesure de ce que vaut l'agent de code d'aujourd'hui sur des défauts réels.
- **Atteignable en service** : oui, Algareth est l'agent de code du produit.
- **Touche rag3weaver** : non. Une mesure ; le lanceur est à la session archi.
- **Pour** : banc (le juge et la page de comparaison), archi (le lanceur), chaque session (un ou deux cas).
- **Signalé par** : c7, au mot de Lucie, 19 novembre 2026.
- **Ouvert le** : 19 novembre 2026

## La campagne

- **Les cas.** Chaque session fournit un ou deux cas tirés de ses correctifs réels :
  - le commit parent du correctif ;
  - le ticket de l'époque ;
  - le témoin du correctif, rouge sur le parent.
- **L'agent.** Algareth corrige le défaut à partir du parent et du ticket, sans git, donc sans pouvoir lire le
  correctif réel.
- **Le juge, le banc**, par le témoin : rouge sur le parent, vert après Algareth, et les suites que les chemins
  touchés exigent. Un témoin non joué ou sauté n'est pas vert.
- **La comparaison, le banc**, contre notre correctif réel :
  - l'endroit du changement (fichiers et fonctions, mêmes ou autres) ;
  - la taille (lignes ajoutées et retirées) ;
  - le coût (pas de l'agent, appels au modèle, jetons d'entrée et de sortie, part servie par le cache) ;
  - les dégâts (suites qui rougissent hors du témoin, fichiers touchés sans raison).

## Le premier pas : la page de forme

- le format d'un cas : les champs, où il vit, comment le lanceur le lit ;
- le juge : quelles suites, sous quel poste, quel verdict ;
- la grille de comparaison, et la page qui la tient.

Elle se soumet à c7 et à archi avant le premier cas.

## Pour fermer

La page de forme acceptée, la campagne jouée sur les cas fournis, et la page de comparaison écrite.
