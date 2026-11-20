# Le graphe HNSW parcouru sur des vecteurs réels : des lignes perdues dès le CREATE, et leur correctif

Les 20 et 21 novembre 2026, au mot de c7, pour le ticket
`docs/banc/tickets/2026-10-11-l-elagage-peut-couper-le-dernier-pont-vers-une-region.md`. Branche
`hnsw-lower-graph-bridge`, posée sur master 362caec827.

## Le témoin : la couche basse parcourue en largeur

Une recherche entre dans la couche basse par le nœud que la couche haute lui désigne, ou par son point d'entrée. Le
témoin lit les arêtes des deux couches par le catalogue interne, et parcourt en largeur la couche basse depuis toutes
ces entrées. Une ligne vivante, au vecteur non nul, reliée au graphe, qu'aucune entrée n'atteint, est perdue pour toute
recherche. Chaque ligne perdue est aussi cherchée par son propre vecteur à efs 10000 : c'est le contrôle de la sonde
elle-même (une ligne trouvée dirait que le parcours se trompe ; ce n'est jamais arrivé). Les commandes sont dans le
ticket du pont.

## Ce qu'il a trouvé

- **Dimension 4, master** : 100 tours de `TenThousandRowsInBatchesOf512`, aucune ligne injoignable.
- **Les 127 198 vecteurs de `Scope_Chunk` de code-lourd** (granite-278m, cosinus ; copie de l'instantané
  `avant-4ad394f4d-20261120`) : 4, 3, 2, puis 0, 2, puis 6, 4, 2, 4, 0 lignes injoignables. Rouge à **8 tours sur 10**,
  et toujours **dès le CREATE**, avant les mises à jour et les suppressions. Aucune n'est rendue par son propre vecteur.
- **Qui est perdu** : 7 lignes distinctes sur 5 tours. Quatre forment un amas de quasi-copies (deux paires de copies
  exactes, à distance nulle entre elles, le voisin suivant à 0,09) et reviennent presque à chaque tour. Les trois
  autres sont des points ordinaires : deux isolés (plus proche voisin à 0,10 et 0,12), et la moitié d'une paire serrée
  (0,02). Le mécanisme commun : un nœud écarté d'une liste comme « couvert » par un voisin plus proche de lui, qui ne le
  vise pas.
- **Un témoin synthétique** (500 amas de trois quasi-copies loin d'un nuage gaussien, 8 fils) est resté vert 5 tours
  sur 5 sans correctif : retiré, il ne reproduit pas le cas réel.

## Le correctif

`InMemHNSWLayer::keepEveryNodeReachable`, appelé par `finalizeHNSWTableFinalizeFunc` avant `moveToPartitionState` :
après le dernier élagage, la couche basse est parcourue en largeur depuis son point d'entrée, et chaque nœud non
atteint au vecteur non nul reçoit une arête depuis son plus proche voisin déjà atteint (parmi ses propres voisins, une
recherche depuis l'entrée en repli), dans la marge entre le degré et le seuil d'élagage. C'est `keepNodeReachable`,
pour le bâti. Une première version cherchait depuis l'entrée pour chaque nœud : le CREATE gaussien montait à 153 s,
sous charge.

## Le jugement

- **Vrais vecteurs, 20 tours avec le correctif** (deux versions) : **0 injoignable après le CREATE, à chaque tour.**
  Un tour sur les vingt a coupé deux lignes **après les suppressions** : un défaut du chemin de suppression, à part,
  `docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-20-une-suppression-sous-un-index-vectoriel-peut-couper-des-voisins-du-graphe.md`.
- **Le coût, sous `poste mesure`, les deux côtés dans les mêmes conditions** (Release, 8 fils, `seed_points` 1000, une
  passe chacun) :

| base | CREATE sans correctif | CREATE avec | rappel@10 sans → avec | points isolés dans leur top 10 |
|---|---|---|---|---|
| gaussienne (120 000, dimension 768) | 46,7 s | 47,5 s | 0,7605 → 0,7515 | 1/200 → **145/200** |
| vrais vecteurs (126 998) | 38,5 s | 38,0 s | 0,998 → 0,9985 | 200/200 → 200/200 |

L'écart de rappel gaussien tient dans la dispersion d'une passe à l'autre : de 0,7425 à 0,7605 sur toutes les passes
du jour, avec et sans correctif. Sur la base gaussienne, les points les plus isolés, que le banc voyait perdus à 1 à 4
sur 200, sont rendus à 145 sur 200.

## La leçon de mesure

La veille, une mesure de 200 points isolés pris dans un **échantillon de 2 000** avait conclu « 200/200, rien sur les
vrais vecteurs ». Elle ne pouvait pas voir des pertes de 0 à 6 lignes sur 127 000. Une absence de perte se prouve sur
toute la table, par un parcours du graphe, pas sur un échantillon.
