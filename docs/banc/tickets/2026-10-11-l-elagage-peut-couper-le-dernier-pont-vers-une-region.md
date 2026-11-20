# L'élagage de l'index vectoriel peut couper le dernier pont vers une région

- **État** : ouvert ; correctif du bâti prêt (hotfixs, 20 novembre 2026, branche `hnsw-lower-graph-bridge`), en
  attente de son train C++. Atteint sur les vrais vecteurs **dès le CREATE** ; la part des suppressions a son ticket,
  `docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-20-une-suppression-sous-un-index-vectoriel-peut-couper-des-voisins-du-graphe.md`.
  Mesures : `docs/coeur-cpp-hotfixs-tickets/20-11-2026/04-le-graphe-hnsw-parcouru-sur-des-vecteurs-reels.md`.
- **Gravité** : réponse fausse (des lignes qu'aucune recherche ne rend, pas même celle de leur propre vecteur)
- **Atteignable en service** : oui. Sur les 127 198 vecteurs de `Scope_Chunk` de code-lourd, de 0 à 6 lignes
  injoignables par bâti, 8 tours sur 10 (20 novembre). Toujours vert sur master en 100 tours de
  `TenThousandRowsInBatchesOf512` (dimension 4), où le parcours de la couche basse ne trouve rien.
- **Touche rag3weaver** : non vu
- **Ouvert le** : 11 octobre 2026, session du banc (page 04 du banc, §10.1)
- **Famille** : moteur

## Le témoin sur vrais vecteurs (20 novembre 2026, hotfixs)

Une mesure sous `poste lourd`, pas une suite : 8 minutes par tour, et 900 Mo de vecteurs hors du dépôt.

1. **Les vecteurs** : une copie par reflink d'un instantané d'une base de code (base fermée), ouverte en lecture
   seule, puis `COPY (MATCH (n:Scope_Chunk) RETURN n.embedding__granite_278m AS vec) TO '<dossier>/vec.csv'
   (header=false);` par le shell (`rag3db -r`). Une ligne « [x,…] » par vecteur.
2. **Le lancement**, dans un arbre bâti en Release avec l'extension vector :
   `CONCURRENCE_MEASURE=1 CONCURRENCE_VECTOR_RUNS=10 BANC_VECTORS_CSV=<dossier>/vec.csv POSTE_MEM_MAX=32G
   poste lourd timeout 150m test/transaction/concurrence/concurrence_test
   --gtest_filter='VectorIndexBridgeMeasure.*'`.
   Chaque tour : COPY, CREATE en cosinus, 20 000 vecteurs renversés par lots de 512, une suppression sur sept parmi
   20 000 lignes ; la couche basse est parcourue en largeur depuis toutes ses entrées après chaque étape, et chaque
   ligne injoignable est cherchée par son propre vecteur à efs 10000. Vert : 0 injoignable à chaque tour.
3. **La suite entière** : `CONCURRENCE_LOWER_GRAPH=1` ajoute le parcours au contrôle de tous les tests de
   `VectorIndexUpdate`, en quelques minutes.

Tout lot qui touche l'élagage, le bâti ou les suppressions de l'index vectoriel rejoue les deux.

## Ce que c'est

Dans la couche basse du graphe HNSW, l'élagage d'un nœud (`shrinkForNode`) retire les arêtes que
la règle de sélection juge couvertes. Quand l'arête retirée était la dernière qui menait d'une
région de l'espace à une autre, toute la seconde région devient injoignable depuis l'entrée, alors
que chacun de ses nœuds garde des arêtes entrantes venues de sa propre région.

## Recette minimale

Aucune de déterministe. La coupure a été vue sur la variante d'essai « union »
(`essai-fin-d-instruction-filtre`, `bb50fb768`), dans `TenThousandRowsInBatchesOf512` :
- d'abord 1 essai sur 10, puis 1 sur 30 ;
- lignes 0 à 2 032 perdues, 7 967 sur 10 000 atteintes ;
- dans la couche basse, **0** arête de la région atteinte vers la région perdue (sur 537 306),
  et 219 dans l'autre sens ;
- environ 67 arêtes sortantes par nœud, de part et d'autre.

## Témoin

À écrire : un contrôle du graphe qui, pour les lignes que la recherche exhaustive perd, compte les
arêtes de la couche basse qui y entrent depuis la région atteinte. Zéro signe la coupure. La sonde
qui l'a montré est dans les notes du banc (`sonde-coupure.py`, posée dans le `check` de
`vector_index_update_test.cpp`).

## Cause

Le mécanisme est établi par la sonde. Ce qui suit est déduit :
- Master ne recontrôle que les anciens voisins d'une ligne mise à jour, et pas les nœuds que
  l'élagage écarte. La coupure y est donc possible en principe.
- Le filtre des essais de fin d'instruction ne peut pas la voir : il tient pour joignable un nœud
  désigné par un voisin qui n'a rien perdu, et tous les nœuds de la région coupée le sont.
- Les liens de rattrapage posés sans élagage (`keepNodeReachable`) chargent les nœuds jusqu'au
  seuil, et rendent les élagages plus fréquents.

## Correctif de l'amont (à lire, ne pas copier)

Aucun : Ladybug n'a ni mise à jour ni suppression dans son HNSW.

## Pour le fermer

Un recontrôle par la recherche d'au moins un nœud de chaque région que l'élagage détache. Ou bien
une règle d'élagage qui ne retire jamais la dernière arête vers un nœud qui n'en a pas d'autre
venue d'ailleurs. Le témoin du graphe ci-dessus, joué sur `TenThousandRowsInBatchesOf512` et sur
les vrais vecteurs, assez d'essais pour un taux de 1 sur 30.
