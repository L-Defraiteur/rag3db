# Après un CREATE sous écritures, la recherche entre dans un nuage de lignes déplacées

- **État** : corrigé par le commit qui ouvre et ferme ce ticket (19 octobre 2026, seconde session
  cœur C++, forme tranchée par l'orchestration) ; voir « Ce qui a été fait », à la fin.
- **Gravité** : réponse fausse. Une recherche, même exhaustive, n'atteignait plus que les lignes
  d'un nuage : 146 lignes sur 19 984.
- **Atteignable en service** : non. Le rattrapage n'agit que sur des écritures validées pendant
  le bâti d'un `CREATE_VECTOR_INDEX`. Or rag3weaver lance ses deux seuls CREATE
  (`rebuild_vector_index`, `ensure_vector_index`) sous le `Mutex<Catalog>` de la connexion
  d'écriture, ou à l'ouverture avant que le catalogue soit partagé. La montre, l'index en fond et
  `CodeIngestNode` prennent ce même verrou. Le SET ordinaire, même de N vecteurs en une
  instruction, n'a pas ce défaut : `NodeTable::update` met l'index à jour ligne par ligne, avant
  d'écrire cette ligne dans la table, si bien que le graphe et la table restent d'accord à chaque
  pas.
- **Touche rag3weaver** : non, tant que le produit ne crée pas d'index sous écritures. (A), le
  CREATE concurrent (page 07), l'y autorisera.
- **Ouvert le** : 19 octobre 2026, par hotfixs. Le rejeu ciblé de son lot du témoin de (A) a
  trouvé un rouge nouveau.

## Ce qu'on voit

`VectorIndexUpdate.ChangesCommittedDuringACreateAreSeen` est le témoin de (δ). Une table de
20 000 lignes reçoit un CREATE pendant qu'une autre connexion valide en boucle : des insertions,
la suppression de lignes paires, le déplacement de lignes impaires vers un nuage lointain
(`[500 + j, j % 7, 0, …, -1]`). Joué 20 puis 40 fois sur master, il rougit environ une fois sur
douze. Dans ce cas, la recherche exhaustive n'atteint qu'une poignée de lignes :

```
create under writes: … 136/19986 reached …
create under writes: … 253/19973 reached, … 112/112 moved rows found at their new vector
```

La bissection est innocentée. cf71c929d, le dernier commit de (δ), est déjà rouge (3 sur 40).
Les deux lots du matin (R1, VMRegion) n'y sont pour rien : le premier 0 sur 20 de cf71c929d
était de la chance.

## La cause

Elle est prouvée par trace, en deux temps.

**Les points d'entrée relevés.** Sur 40 passes, les 3 rouges ont pour point d'entrée de la
couche haute une ligne du nuage (offsets 47, 3, 3). Les 37 verts partent d'une ligne ordinaire.

**Les écritures du point d'entrée.** Une instrumentation jetable trace chacune de ces écritures.
Sur un rouge à 146/19984 :

```
catchUp start upper=17 lower=0 del=97 upd=64 ins=81
deleteFromGraph upper 17 -> 21 (32 nbrs)
deleteFromGraph upper 21 -> 17 (32 nbrs)
catchUp after updates upper=17
```

Le rattrapage (`OnDiskHNSWIndex::catchUpCommittedAfter`) reprenait les lignes déplacées une à
une, par `updateInternal`. Mais il les reprenait contre une table où **toutes portaient déjà
leur nouveau vecteur** :

1. La ligne 17, point d'entrée haut, est déplacée. `deleteFromGraph` la remplace par son
   premier voisin vivant, 21. C'est une impaire pas encore traitée, encore à son ancienne place
   dans le graphe.
2. Réinsérée, 17 se lie à 21, « proche » au nuage puisque la table porte déjà le nouveau vecteur
   de 21.
3. 21 est déplacée à son tour, et son premier voisin vivant est 17, déjà dans le nuage.

Les premiers nœuds de la couche haute se lient entre eux au bâti, et ce sont les premières
lignes d'un groupe de nœuds : un point d'entrée déplacé a donc souvent pour voisins d'autres
lignes déplacées. La recherche entre alors dans le nuage, et la couche basse n'en ressort pas.

**Un défaut voisin, indépendant.** `insertToLayer`, devant un point d'entrée haut invalide,
faisait du nœud inséré le point d'entrée sans aucune arête, « comme si la couche était vide ».
Toute recherche entrait alors sur lui seul. La couche basse, elle, vérifiait par `findLiveNode`.
Un point d'entrée haut devient invalide quand une suppression ne laisse survivre aucun de ses
voisins hauts, ou quand on annule la ligne qui le portait (`rollbackInsert`).

## Les témoins

Les deux sont dans `vector_index_update_test.cpp`, et tous deux sont rouges 3 fois sur 3 sur
master c2ef8a941.

- `RowsMovedTogetherDuringACreateStayReachable`. Un écrivain, ouvert avant le CREATE et validé
  pendant son bâti, déplace les 150 premières lignes de chaque groupe de 2 048 vers une grille
  serrée. Il y a six tours, chacun sur une table neuve. Sur master, la moitié des tours rougit,
  à « 900/12288 atteintes » : exactement le nuage.
- `AnInvalidUpperEntryPointIsTakenFromTheUpperLayer`. Le point d'entrée haut est rendu invalide
  par les internes, puis des lignes s'insèrent jusqu'à ce que l'une monte dans la couche haute.
  Sur master, c'est cette ligne neuve qui devient le point d'entrée (2004, 2013).

## Ce qui a été fait

**Le rattrapage sort ensemble les lignes déplacées et les supprimées.** Elles passent par
`deleteInternal`, puis `finalizeDelete`, et chacune rentre ensuite à son nouveau vecteur. Aucune
ne peut plus remplacer un point d'entrée. Ce qui reste du graphe est à la place de son vecteur.

**La recherche ne suit plus une arête vers une ligne sortie, tant que la ligne n'est pas
rentrée** (`HNSWSearchState::outOfGraph`). Les tables d'arêtes de l'index ne stockent que le sens
avant (`storage_direction='fwd'`). Une survivante garde donc ses arêtes sans retour vers une
ligne sortie, et `finalizeDelete` ne réécrit pas ces arêtes. Sans cet écart, les réinsertions se
liaient à ces impasses : 2 tours sur 30 ne retrouvaient que 889 ou 890 lignes déplacées sur 900.
Or la même grille, posée par un SET ordinaire ou bâtie d'un coup, en retrouve 900 sur 900 sur
20 tours.

**Un point d'entrée haut invalide se prend dans la couche haute** (`findLiveUpperNode` : un
nœud vivant qui y a des arêtes), dans `insertToLayer`. Le repli de `repairEntryPoint` garde la
conduite de master : il laisse le point d'entrée haut invalide, et la recherche part du point
d'entrée bas, réparé par balayage. Une première version y prenait aussi un nœud de la couche
haute. Après une suppression massive, ce nœud pouvait n'avoir plus d'arêtes hautes que vers des
lignes retirées, et la couche basse, depuis lui, n'atteignait pas toutes les survivantes :
`RegionDelete/SetNullFirstAllButFive` a rougi 2 fois sur 26 (4 survivantes atteintes sur 5),
contre 0 sur 25 sur master. Ramené à master, il est vert 25 fois sur 25.

**La comptabilité de `createIndexUnderWrites`.** Une ligne « déplacée puis supprimée » dont le
DELETE validait juste après la fin du CREATE restait attendue à son nouveau vecteur. Les manques
d'une ligne (40/41, 64/65), vus sur master comme avec le correctif, venaient de là. Les quatre
ids relevés (139, 149, 169, 179) sont tous d'indice j % 5 = 4, et introuvables même à efs 2000.

**La mise en place recommence.** Sur un poste chargé, un CREATE pouvait finir avant qu'une
écriture de chaque genre ait validé (« no row moved then deleted during the CREATE »).
`createIndexUnderWrites` recommence alors sur une table neuve, jusqu'à cinq fois. Un CREATE en
échec reste un rouge.

Avec le correctif, sur luciepc chargé :

- `AnInvalidUpperEntryPointIsTakenFromTheUpperLayer` : 13 verts sur 13, puis 3 sur 3 avec le
  repli ramené à master ;
- `ChangesCommittedDuringACreateAreSeen` : 30 passes, toutes les lignes atteintes ;
- `RowsMovedTogetherDuringACreateStayReachable` : 205 tours, toutes les lignes atteintes. Un seul
  tour n'a retrouvé que 83 lignes déplacées sur 90 à la recherche courte ; il ne s'est pas
  reproduit en 144 tours sous diagnostic. Un manque écrit désormais la ligne perdue, la ligne
  rendue à sa place et ce que rend une recherche à efs 2000.

Le témoin du rattrapage dure environ 30 s : il est rangé dans `long.txt`.
