# L'élagage de l'index vectoriel peut couper le dernier pont vers une région

- **État** : ouvert
- **Gravité** : réponse fausse (des lignes que la recherche exhaustive ne trouve plus)
- **Atteignable en service** : en principe oui, après des mises à jour ou des insertions massives. Vu seulement sur une variante d'essai ; jamais sur master, en 13 essais de `TenThousandRowsInBatchesOf512`.
- **Touche rag3weaver** : non vu
- **Ouvert le** : 11 octobre 2026, session du banc (page 04 du banc, §10.1)

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
