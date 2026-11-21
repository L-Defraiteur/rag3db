# Une suppression sous un index vectoriel peut couper des voisins du graphe

- **État** : fermé (21 novembre 2026, hotfixs), corrigé sur la branche `hnsw-delete-reachability` (319730f984 avant
  la repose). La cause : `keepFormerNeighboursReachable` (étape 3a) tenait un ancien voisin pour joignable dès qu'un
  nœud du dehors pointait vers lui, alors que ce nœud pouvait n'être joint que par lui ; un îlot dont la seule entrée
  venait des lignes supprimées se déclarait joignable sans recherche. Le correctif : seule l'ancre prouve ; le reste
  passe par la recherche depuis l'entrée, et ce qu'elle visite est marqué joignable. La preuve : le témoin déterministe
  `VectorIndexUpdate.AnIslandOfFormerNeighboursStaysReachableAfterADelete` (graphe posé à la main : 0 → D, D → X,
  X ↔ Y ; D supprimé), **rouge sur le moteur de e22dede600** (2 injoignables, leur propre recherche rend 7 et 15),
  **vert avec le correctif**. Sur les vrais vecteurs de code-lourd, `RealVectorsDeletedInWaves` (douze vagues de
  suppressions par blocs contigus, deux tours) : rouge une vague sur 48 sans correctif (l'amas de quatre quasi-copies,
  coupé par un bloc lointain), 0 sur 24 avec ; les DELETE ne sont pas plus lents (42 à 83 s avec, 51 à 92 s sans, sous
  charge).
- **Famille** : moteur
- **Gravité** : réponse fausse. Une ligne vivante, voisine d'une ligne supprimée, n'est plus rendue par aucune
  recherche, pas même celle de son propre vecteur à efs 10000.
- **Atteignable en service** : oui, par toute suppression de lignes indexées, à faible taux : 1 tour sur 16, 2 lignes
  sur 124 341.
- **Touche rag3weaver** : par le moteur (les suppressions de morceaux à la resynchronisation d'un fichier)
- **Signalé par** : hotfixs, le parcours de la couche basse (`CONCURRENCE_LOWER_GRAPH`), branche
  `hnsw-lower-graph-bridge`
- **Pour** : hotfixs

## Le constat

`VectorIndexBridgeMeasure.RealVectorsUpdatedAndDeleted`, sur les 127 198 vecteurs de `Scope_Chunk` de code-lourd
(granite-278m, cosinus). Chaque tour fait trois étapes : le CREATE, 20 000 mises à jour par lots de 512 (le vecteur
renversé), puis la suppression d'une ligne sur sept parmi les ids 20 000 à 40 000. La couche basse est parcourue en
largeur depuis toutes ses entrées après chaque étape.

Le 20 novembre, au tour 1 du second jugement du correctif du bâti (tête 49afee87fe), le parcours trouve 0
injoignable après le CREATE et 0 après les mises à jour, mais **2 après les suppressions** : les ids 22046 et 22048
(décalages 10864 et 10865), vivants (ni l'un ni l'autre n'est multiple de sept), dans la plage où des voisins ont été
supprimés. Aucun des deux n'est rendu par son propre vecteur à efs 10000. Les quinze autres tours de vrais vecteurs
avec le correctif n'ont rien coupé aux suppressions.

## Où regarder

`OnDiskHNSWIndex::deleteInternal` → `deleteFromGraph`, puis `keepFormerNeighboursReachable` (`hnsw_index.cpp`,
autour de 995-1050) : le commentaire dit « No anchor here, and a deleted node proves nothing ». Les voisins d'un nœud
supprimé reçoivent ses voisins vivants, sont réélagués (`lowerNeighborsToShrink`), et un écarté peut se retrouver sans
arête entrante. C'est la forme du ticket `docs/banc/tickets/2026-10-11-l-elagage-peut-couper-le-dernier-pont-vers-une-region.md`,
cette fois sur le chemin de suppression.

## Témoin

Le même que celui du bâti, qui est rouge ici 1 tour sur 16 ; il faut donc plus de tours, ou un témoin qui isole les
suppressions. Les commandes sont dans le ticket du pont (section « Le témoin sur vrais vecteurs »). Le parcours de la
couche basse peut aussi se jouer sur toute la suite de l'index vectoriel, `CONCURRENCE_LOWER_GRAPH=1`.

## Pour le fermer

Après `deleteFromGraph`, un recontrôle de la joignabilité des voisins réélagués, à la manière de la passe de fin de
bâti (`keepEveryNodeReachable`), limité aux voisins des nœuds supprimés. Puis ce témoin vert sur assez de tours pour un
taux de 1 sur 16.
