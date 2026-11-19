# Un rang déplacé pendant un CREATE de l'index vectoriel n'est pas toujours rendu premier à son nouveau vecteur

- **État** : ouvert, le 7 novembre 2026 (seconde session cœur C++, hotfixs). Le cas est inscrit à
  `test/transaction/concurrence/probabilistic.txt` ; le correctif entre en tête du lot G6 (HNSW hors
  verrous), avec un témoin déterministe s'il s'en trouve un.
- **Gravité** : réponse fausse en silence (le 19 novembre : environ une fois sur deux sur une recette sans concurrence ; voir plus bas) : un rang mis à jour pendant la construction
  de l'index est trouvé, mais pas au premier rang, par une recherche à son vecteur exact.
- **Atteignable en service** : oui, par un CREATE_VECTOR_INDEX sous écritures (le rattrapage de
  (δ), 6fe9ffe9b, et le CREATE concurrent (A), 97daa41ed).
- **Touche rag3weaver** : oui, par la recherche dense.
- **Pour** : hotfixs (rattrapage de (δ)).
- **Famille** : moteur

## Ce qu'on voit

`VectorIndexCreateInMemory.ChangesCommittedDuringACreateAreSeen` (`vector_index_update_test.cpp`),
check `rows-moved-during-create-found`. Pendant le bâti, des écrivains insèrent, suppriment et
déplacent des rangs. Après le CREATE, chaque rang déplacé est cherché à son nouveau vecteur et
doit sortir premier :

```
moved row 1 not found first at [500, 0 % 7, 0, …, -1]: got 3, at efs 2000 1|0.000000
create under writes: 333 inserted, 333 deleted, 267 moved, 66 moved then deleted during the build;
19934/19934 reached, 0 dead returned, 0 returned twice, 266/267 moved rows found at their new vector
```

Le rang 1 est trouvé, mais au troisième rang, alors que la distance à son vecteur est nulle.

## Les passes

À luciepc, le 7 novembre 2026, binaire du banc de `csr-listes-temoin-3` (moteur de master
ddb856e21) :

| Passe | Rouges |
|---|---|
| le banc complet (comparaison à known_red, charge ~15) | 1 sur 1 (266/267) |
| la famille `VectorIndexCreate*` répétée cinq fois (charge 15) | 1 sur 5 (466/467) |
| le cas seul, dix fois | 0 sur 10 |

Un rouge sur trois passes sous charge, aucun seul : l'entrelacement des écrivains avec le bâti
décide.

### Le 15 novembre : une fois sur quatre, le cas seul

Mesuré par cœur c++ à luciepc, bâti Release de `rag3db-colonnes/build/moteur` sous `poste lourd`,
le cas seul répété (`--gtest_repeat=96`) :

| tête | ce qu'elle porte | rouges |
|---|---|---|
| `a2f9c9b65` | master | 25 sur 96 (2 sur 24 au premier relevé) |
| `d5f5325f0` | master + le lot du verrou de schéma (`alter-attend-les-ecrivains`) | 27 sur 96 (6 sur 24) |

Le défaut se joue désormais sans charge autour, environ une fois sur quatre, sur master comme
avec le lot du verrou de schéma : celui-ci ne l'aggrave pas. long.txt le dit encore « rouge
environ une fois sur douze », à reprendre avec ce chiffre.

### Le 18 novembre : les rangs manqués restent à leur ancienne place

Session hotfixs, à luciepc, Release de `insert-after-forced-copy-keeps-rows-scanned` (32a07f5c0e)
sous `poste lourd`, sans autre charge. `VectorIndexCreateInMemory.ChangesCommittedDuringACreateAreSeen`
répété 96 fois avec une sonde non commitée : pour chaque rang manqué, ses arêtes dans les deux
couches. **3 rouges sur 96** (0 sur 20 au relevé d'avant ; sous ASan, 4 sur 10 pour la variante en
mémoire et 0 sur 10 pour celle sur disque, `VectorIndexUpdate.…`).

Ce que la sonde montre :
- **Ce n'est pas une égalité** : chaque rang déplacé a son propre vecteur (`[500 + j, j % 7, 0, …,
  -1]`), et la recherche large (efs 2000) trouve le rang manqué à distance 0, sauf dans le pire
  relevé.
- **Le rang manqué est dans le graphe à son ancien vecteur.** Ses arêtes de la couche basse, en
  sortie (60) comme en entrée (32 à 138), vont toutes vers le nuage d'origine (premier coordonné de
  0 à 16), aucune vers les autres rangs déplacés (500 et plus). Il n'a pas été repris par le
  rattrapage, et le bâti l'a placé à l'ancien vecteur.
- **Ce sont les premiers déplacés de la fenêtre.** Le pire relevé : 24 manqués sur 308 (j = 0 à 33,
  rangs 1 à 67), le rang 1 introuvable même à efs 2000 (« got 43 » à distance 21). C'est une perte
  de rappel, pas un écart à la marge.
- **Sans écrivain concurrent, en mémoire, le bâti voit bien le SET** : au shell, un SET du rang 1
  puis le CREATE, la recherche le rend premier à distance 0. La colonne mise en cache applique les
  mises à jour validées.

Lecture : un SET validé tôt dans la fenêtre échappe aux deux chemins à la fois. Le bâti lit
l'ancien vecteur, donc la colonne en cache a été lue avant que ce SET ne lui soit visible. Et le
rattrapage ne le voit pas validé après `buildStartTS` (`hasCommittedUpdateAfter`). L'instant de
lecture du cache et `buildStartTS` ne sont donc pas le même pour ces rangs ; à départager entre
T_A (NOTED, qui ne reçoit pas les écritures), l'attente `_WAIT_HNSW_WRITERS` et le début de T_B, et
pourquoi seulement en mémoire (sur disque, la validation de T_A est un point de reprise forcé).

### Le 18 novembre au soir : les lectures sont justes, le lien manque

Une sonde moteur (non commitée, `RAG3DB_PROBE_E3`), 96 répétitions deux fois (4 puis 2 rouges),
date chaque étape pour les décalages impairs 1 à 69 :
- **Le départ de T_B** (`buildStartTS`) vaut 9 à 12 : l'écrivain n'a encore rien validé, sauf
  parfois son premier SET (le rang 1, j = 0). Tous les autres SET des rangs déplacés valident après
  (`hasCommittedUpdateAfter` vrai).
- **La colonne mise en cache** lit le rang 1 à 500, son nouveau vecteur, quand son SET a validé
  avant T_B ; les autres à leur ancienne valeur. C'est juste : c'est l'instantané de T_B.
- **Le rattrapage** réinsère chaque rang déplacé après `buildStartTS` à son nouveau vecteur (501,
  502…). Juste aussi.

Le défaut n'est donc ni dans la lecture du cache ni dans celle du rattrapage, mais dans le lien. Au
bâti, le rang 1 est seul à `[500, 0, …, -1]`, et ses voisins les plus proches sont dans le nuage
d'origine : le bâti l'y relie, à juste titre. Le rattrapage bâtit ensuite le nuage des rangs
déplacés à côté de lui, à distance 1,4, sans jamais le relier : ni arête sortante du rang 1 vers ce
nuage, ni arête entrante depuis lui. Une recherche qui entre dans le nuage s'arrête au rang 3. Le
même schéma vaut pour les rangs 25 et 27 d'un autre relevé, réinsérés eux aussi.

Les questions suivantes : la recherche d'insertion du rattrapage atteint-elle le rang 1 (les
voisins du nuage d'origine qui pointent vers lui) ? Si oui, pourquoi l'arête est-elle perdue, à la
sélection des voisins ou à un élagage (`shrinkForNode`) ? Et pourquoi seulement en mémoire ?

### Le 19 novembre : sans concurrence ni CREATE, une fois sur deux

**Le défaut n'est pas une course du CREATE.** Une sonde dans `insertToLayer` (couche basse, rattrapage) le montre :
la première réinsertion, celle du rang 3 à `[501, 1, 0, …, -1]`, cherche depuis le nœud 10452 et rend 60 voisins,
tous dans le nuage d'origine à environ 490. Le rang 1, pourtant à 1,41, n'en est pas. Chaque rang déplacé se relie
ensuite aux précédents, et le nuage se forme sans lui.

**La recette sans concurrence.** Au shell, master 9906f44d69, six essais par case :
1. 20 000 lignes ;
2. le SET du rang 1 à `[500, 0, …, -1]` ;
3. CREATE_VECTOR_INDEX ;
4. 150 SET (ou 150 insertions) à `[500 + j, j % 7, 0, …, -1]` ;
5. `QUERY_VECTOR_INDEX(…, [500, 0, …, -1], 1, efs := 64)`.

| nuage bâti par | en mémoire | sur disque |
|---|---|---|
| SET | 3 manqués sur 6 | 4 sur 6 |
| insertions | 4 sur 6 | 3 sur 6 |

**Lecture.** Au bâti, le rang 1 est seul loin du nuage d'origine. Toutes ses distances au nuage sont proches de 490,
et la recherche ne les départage pas. Il se relie (60 arêtes sortantes, 32 entrantes) à des nœuds du nuage qui ne
sont pas ceux par où passent les recherches venues de sa direction. Les points insérés ensuite près de lui ne le
trouvent pas, et ne s'y relient pas. La recherche exacte de son vecteur s'arrête dans le nouveau nuage, au plus
proche voisin, à 1,41.

C'est une faiblesse du graphe HNSW autour d'un point isolé, par n'importe quel chemin (bâti, insertion, mise à jour,
rattrapage), en mémoire comme sur disque. **Gravité** : une réponse fausse, la recherche exacte d'un vecteur présent
ne le rend pas premier. Le cas du banc (`probabilistic.txt`) en est une forme.

**Question à Lucie (19 novembre, par c7).** Corriger, c'est toucher la conception de l'insertion : par exemple
relier un nouveau point aussi aux voisins de ses voisins, ou garder les arêtes entrantes d'un point isolé. Ou bien
on tient cela pour la limite d'un index approché, et le témoin du banc dit une borne de rappel plutôt qu'un premier
rang.

## Pour avancer

1. ~~Lire pourquoi le rang n'est rendu qu'au troisième rang~~ : il est resté à son ancien vecteur
   (18 novembre, ci-dessus). Reste à dater, pour un rang manqué, son SET contre la lecture du cache
   et `buildStartTS`.
2. Un témoin déterministe : un rang déplacé à un instant choisi du bâti (après le dernier lot du
   balayage, avant le rattrapage), puis la recherche à son vecteur.
3. Le correctif, puis la ligne de `probabilistic.txt` retirée dans son commit.
