# La place d'une génération gardée du plein texte

Session lucivy, 21 novembre 2026, à la question de Lucie : « si lucivy savait partager un index entre plusieurs
générations, ce serait fou, pour économiser la place disque ».

## Ce que fait lucivy 4.4

Il partage déjà, par construction. Une génération gardée n'est qu'un `meta.<g>.json` de quelques Ko, qui nomme ses
segments. Un segment ne change jamais une fois écrit : tant que les générations suivantes le gardent, c'est **le même
fichier** pour toutes. Une génération gardée ne retient en plus que ce que les suivantes ont remplacé :
- les segments que les fusions et la compaction ont réécrits ;
- les anciens fichiers de retraits (`.del`) d'un segment qui a reçu des retraits ;
- l'ancienne génération du dictionnaire de shard (`dict-<g>`), que la compaction du dictionnaire réécrit.

Entre worktrees, c'est l'autre direction : un index par dépôt, des vues par worktree.

## La mesure

Sur une copie de l'index ScopeContent du dépôt (instantané de G, 08:37, hôte arrêté), avec lucivy 4.4.0 seule, par une
sonde jamais commitée :

| Étape | Taille | Écart |
|---|---|---|
| Départ | 1 015 Mo (104 917 documents) | |
| Un `commit_tagged`, rien changé | 1 021 Mo | +6,3 Mo |
| Une maintenance avec compaction, la génération 1 gardée | 2 015 Mo | +993 Mo, en 31 s |
| La génération 1 relâchée (`release_below`, puis un commit) | 1 006 Mo | |

## Ce que cela dit

- **Garder une génération quand peu change coûte peu** : quelques Mo, le méta, les statistiques et ce que le commit
  réécrit.
- **Une compaction réécrit tout l'index** : tant que la génération d'avant est gardée, elle retient l'index entier
  (~1 Go ici). La place revient au relâchement.
- D'où la règle de la voie B : `release_below` dès que la base a validé de façon durable. Avec la génération par
  paquet (décision (A) du 21 novembre), la fenêtre se compte en secondes, et un paquet ordinaire ajoute quelques Mo.
- La compaction est le pire cas. Elle est rare : maintenance nocturne, ou explicite. À la faire tourner, il faut garder
  de la place pour un index de plus, le temps d'une génération.
