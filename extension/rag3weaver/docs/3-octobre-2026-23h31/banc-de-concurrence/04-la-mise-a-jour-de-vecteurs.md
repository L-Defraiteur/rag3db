# La mise à jour de vecteurs dans l'index HNSW — une page avant le code

5 octobre 2026, session du banc. Ticket :
`docs/tickets/2026-10-04-mise-a-jour-massive-de-vecteurs-lignes-injoignables.md`. Cette page
s'appuie sur une lecture de l'extension vector et des opérateurs SET et DELETE, à `31ad712c5`
(**[lu]** : vu dans le code ; **[déduit]** : tiré de cette lecture, non exécuté).

## 1. Le défaut

Quand une instruction remplace les vecteurs de beaucoup de lignes d'une table indexée, des
lignes deviennent injoignables : une recherche exhaustive ne les rend plus, ou une ligne ne
sort pas première pour son propre vecteur. Cela arrive un essai sur deux. La réponse fausse,
c'est une ligne absente des résultats.

rag3weaver n'y passe presque pas. Ses embarquements posent un vecteur sur une ligne qui n'en
avait pas, et son réembarquement repasse par NULL ; ces deux chemins sont verts.

## 2. Ce que fait le moteur aujourd'hui

### La mise à jour d'une ligne

`SingleLabelNodeSetExecutor::set` crée un état de mise à jour neuf à CHAQUE ligne
(`set_executor.cpp:54-57`) **[lu]**. `OnDiskHNSWIndex::update` (`hnsw_index.cpp:994-1089`)
procède en trois temps **[lu]** :

1. **Le retrait de l'ancienne position.** `deleteFromGraph` retire les arêtes SORTANTES de la
   ligne ; ses arêtes entrantes restent, et désignent désormais la nouvelle position. Puis
   `repairEntryPoint`.
2. **La réinsertion du nouveau vecteur.** `insertInternal` → `insertToLayer` →
   `createRels` (la ligne vers ses voisins, ses voisins vers elle). Un voisin dont le degré
   déborde est élagué tout de suite par `shrinkForNode`, ou mis en attente dans
   `lowerNodesToShrink`. Cet ensemble est jeté avec l'état, à la fin de la ligne.
3. **Le contrôle des anciens voisins.** `c8fdaf196` y faisait une recherche par ancien voisin
   (`keepNodeReachable`). `110a65f15`, un commit d'optimisation (la mise à jour était 60 fois
   plus lente), l'a remplacée par une heuristique : un voisin compte comme joignable si la
   ligne réinsérée pointe vers lui, ou si l'un de ses propres voisins le fait.

Cette heuristique suppose les arêtes réciproques, et c'est justement l'élagage qui casse la
réciprocité **[déduit]**. Elle ne regarde ni les nœuds que `shrinkForNode` a écartés (rien ne
les note), ni l'effet d'une ligne sur les suivantes de la même instruction.

### La suppression

La suppression fait mieux **[lu]**. L'état vit le temps de l'instruction : l'exécuteur DELETE le
crée une fois, et `NodeTable::initDeleteStates` initialise une fois les états d'index. À
l'épuisement de l'enfant, `DeleteNode` appelle une fois `finalize`, qui mène à
`OnDiskHNSWIndex::finalizeDelete`. Celui-ci :
1. réécrit les arêtes des voisins survivants (`cleanEdgesForNode`) ;
2. répare les points d'entrée ;
3. appelle `keepNodeReachable` pour chaque survivant : il cherche le nœud depuis le point
   d'entrée, et s'il ne le trouve pas, le relie dans les deux sens à ses plus proches.

### Trois points relevés en route

- **Une ligne créée dans la transaction est insérée deux fois** **[déduit]**. `NodeTable::update`
  appelle `index->update` sans tester si la ligne est locale, puis le commit l'insère de
  nouveau (`commitInsert`, `needCommitInsert` vrai). Le correctif `35d09c466` (SIGSEGV dans
  `shrinkForNode`) est passé par ce chemin.
- **`shrinkForNode` écarte toujours le plus proche voisin, sur disque** **[lu ; effet déduit]**.
  Sa boucle de sélection commence à `i = 1` (`hnsw_index.cpp:1567`, héritée de l'amont
  `444deac67`). En mémoire, `nbrs[0]` est le nœud lui-même. Sur disque, il n'y a jamais
  d'arête vers soi : `nbrs[0]` est le vrai plus proche voisin, toujours écarté. Une ligne
  mise à jour qui est le plus proche voisin d'un nœud qu'on élague perd donc son arête
  entrante à coup sûr.
- **Une ligne mise à jour plusieurs fois dans l'instruction** refait chaque fois le retrait,
  la réinsertion et le contrôle, sur un état neuf. Témoin rouge connu :
  `OneRowUpdatedManyTimes`.

## 3. Ce que font les moteurs établis

D'après le ticket de la ligne lointaine, où sont lues hnswlib `ca426729`, pgvector `f37c13f6`
et Qdrant `6ab21cac`, aucun ne garantit une arête entrante à l'élagage, ni ne répare les
orphelins de l'élagage. Les deux points suivants viennent de ma connaissance générale, non
vérifiée dans le dépôt :
- **pgvector** n'a pas de mise à jour sur place. Un UPDATE est un nouveau tuple, inséré comme
  un élément neuf ; l'ancien reste dans le graphe jusqu'au VACUUM, qui le retire et recalcule
  les voisins de ceux qui pointaient vers lui. C'est l'équivalent de notre `finalizeDelete`.
- **hnswlib** a `updatePoint`. Il réécrit le vecteur, recalcule les listes des voisins à un
  saut, puis reconnecte le point (`repairConnectionsForUpdate`). Il ne vérifie pas la
  joignabilité des nœuds écartés.

Nous ferions donc mieux que les deux en garantissant la joignabilité à la fin de
l'instruction, comme notre suppression le fait déjà.

## 4. La proposition

Faire pour la mise à jour ce que la suppression fait déjà :

1. **Un état par instruction.** L'exécuteur SET garde son `NodeTableUpdateState`, comme
   l'exécuteur DELETE garde le sien (créé au premier `set`, une map par table en
   multi-label). Les états d'index sont initialisés une fois.
2. **Noter ce qu'il faut recontrôler, sans rien rechercher par ligne** :
   - les anciens voisins (déjà connus à l'appel) ;
   - les lignes mises à jour, avec leur vecteur final ;
   - les nœuds que `shrinkForNode` écarte. Il les connaît : il suffit de les ajouter à un
     ensemble de l'état d'insertion.
3. **Une fin d'instruction.** `SetNodeProperty` (et MERGE, qui réutilise les exécuteurs SET)
   appelle une fois `executor->finalize` à l'épuisement de l'enfant, comme `DeleteNode`. Puis
   `NodeTable::finalizeUpdate`, puis un nouveau `Index::finalizeUpdate`, vide par défaut. Pour
   HNSW, dans cet ordre :
   1. les élagages différés ;
   2. `repairEntryPoint` ;
   3. `keepNodeReachable` sur l'union dédupliquée des lignes notées.
   Le contrôle porte sur le graphe FINAL : il couvre l'effet d'une ligne sur les suivantes,
   et une ligne mise à jour dix fois n'est contrôlée qu'une fois.
4. **Le rejeu du journal** appelle la même fin, à la fin d'un enregistrement de mise à jour.
5. **Les lignes de la transaction** : `update` les saute, puisque le commit les insérera avec
   leur valeur finale. C'est l'insertion double du §2.

Hors de cette proposition, à décider à part : le `i = 1` de `shrinkForNode`. Le changer modifie
le comportement de l'amont pour toutes les insertions, y compris la construction ; il mérite
ses propres mesures, avec les témoins de la ligne lointaine.

## 5. Le coût

`keepNodeReachable` fait une recherche par nœud contrôlé. C'est le coût que `110a65f15` a
voulu éviter : environ 800 s pour 10 000 lignes par lots de 512, d'après son message. En fin
d'instruction, l'ensemble est dédupliqué. Dans une mise à jour massive, beaucoup de lignes
sont voisines entre elles : l'union est bien plus petite que « lignes × voisins »
**[déduit, à mesurer]**.

Les témoins existent, rouges ou probabilistes : `SetToAnotherVectorInBatchesOf512`,
`TwentyRowsToDistinctVectorsLineByLine`, `OneRowUpdatedManyTimes`, `UpdateThenDelete`,
`SetToAnotherVectorLineByLine`, `TenThousandRowsInBatchesOf512`. Le chemin de rag3weaver
(`SetFromNull*`, `ReplaceThroughNull*`) doit rester vert, et la mesure de durée de
`TenThousandRowsInBatchesOf512` dira le prix, avant et après.

## 6. Ce que je demande avant de coder

- **À l'orchestration** :
  - la proposition du §4 ;
  - un plafond de coût acceptable sur `TenThousandRowsInBatchesOf512` ;
  - si les lignes de la transaction sautées par `update` vont dans ce lot.
- **À la session cœur C++** :
  - les fichiers que je toucherai : `set_executor`, `set.cpp`, `merge.cpp`, `node_table`,
    `index.h`, `hnsw_index` et `wal_replayer`. `node_table.cpp` est encore le point de
    rencontre ;
  - si elle voit un piège à garder un état d'index le temps d'une instruction : verrous,
    lignes locales, rejeu.

## 7. État au 10 octobre, avant le code

**Ce qui a changé depuis le 5 octobre.**
- L'élagage est redressé sur master (`2da041058`, `c841d507c`). Le `i = 1` du §2 n'existe plus.
  `shrinkForNode` suit la règle classique, avec les places libres reprises par le plus lointain
  et les copies bornées. Le « hors de cette proposition » du §4 est donc fait.
- **Les décisions de l'orchestration** (5 octobre) :
  - la proposition du §4 est acceptée ;
  - plafond de coût : ×2 sur la durée de `TenThousandRowsInBatchesOf512`, prise SUR LE NOUVEL
    élagage ; entre ×2 et ×5, je rends le chiffre avant de pousser ; au-delà, non ;
  - **mesurer d'abord** la taille de l'union à recontrôler par lot de 512, avant d'écrire la fin
    d'instruction ;
  - le chemin de rag3weaver (`SetFromNull*`, `ReplaceThroughNull*`) ne doit pas bouger ;
  - les lignes de la transaction sautées par `update` partent dans **un commit à part, avec son
    témoin**.
- **Les quatre pièges relevés par la session cœur C++** (5 octobre), à tenir dans le code :
  1. **la frontière du rejeu, c'est le COMMIT**, pas l'enregistrement. Une instruction est
     écrite au journal en plusieurs enregistrements de mise à jour : la fin d'instruction au rejeu
     se fait quand la transaction rejouée valide, pas à chaque enregistrement (le §4.4 est à
     corriger en ce sens) ;
  2. **les lignes versées** : une ligne locale versée dans la table par un COPY (`f1d8c7190`) n'est
     plus locale ; la fin d'instruction doit la traiter comme validée ;
  3. **l'échec en cours d'instruction** : si l'instruction échoue après quelques lignes, l'état
     noté est jeté avec l'annulation, sans contrôle de fin ;
  4. **le parallélisme de l'opérateur** : si l'exécuteur SET tourne sur plusieurs fils, l'état
     de l'instruction est partagé et doit être protégé, ou bien un par fil puis fusionné à la fin.

**L'insertion double, vérifiée à la source le 10 octobre** [lu] : `NodeTable::update`
(`node_table.cpp:~617-623`) appelle `index->update` pour chaque index sans tester si la ligne
est locale à la transaction, et `OnDiskHNSWIndex::update` réinsère la ligne dans le graphe. Le
commit l'insère ensuite de nouveau par `commitInsert` (`needCommitInsert`). Une ligne créée puis
mise à jour dans la même transaction entre donc deux fois dans le graphe. À ne pas confondre avec
le versement par COPY, mesuré le 10 octobre : là, une seule insertion par ligne.

**L'ordre de travail.**
1. Le commit à part : `update` saute les lignes locales, avec un témoin (une ligne créée puis
   mise à jour dans une transaction : une seule insertion, une sonde qui compte les insertions par
   ligne, la recherche juste après le COMMIT et après réouverture).
2. La mesure de l'union par lot de 512 sur `TenThousandRowsInBatchesOf512`, et la référence de
   durée prise sur le nouvel élagage, dans une tenue « mesure ».
3. La fin d'instruction du §4, avec les quatre pièges, puis les témoins du §5 et le coût.

## 8. Ce qui est fait, et la mesure de l'union (10 octobre au soir)

**L'étape 1 est faite** : `a162b43e3`. Une ligne créée puis mise à jour dans la même transaction
n'entre plus deux fois dans le graphe. Avant le correctif, 741 arêtes en double pour dix lignes
(0 au contrôle), et rien de visible par la recherche. Témoins `Kinds/RowsCreatedInATransaction.*`
et `RowsFlushedByACopyThenUpdatedFollowTheirVector`.

**L'étape 2, mesurée** sur le poste principal en tenue « mesure », avec l'extension prouvée
(bâtie de sa source, md5 au résumé). Une sonde temporaire dans `OnDiskHNSWIndex::update` et
`shrinkForNode` comptait, par transaction, les lignes mises à jour, leurs anciens voisins et les
nœuds que l'élagage écarte (le patch est gardé dans les notes du banc, pas dans le dépôt).
Sur `TenThousandRowsInBatchesOf512` (20 instructions de 512 lignes) :

| | par lot de 512 | sur 10 000 lignes |
|---|---|---|
| lignes mises à jour | 512 | 10 000 |
| anciens voisins (distincts) | 667 à 756 | — |
| nœuds écartés par l'élagage | 497 à 695 | — |
| **union à recontrôler** | **680 à 910** | **16 051**, soit 1,6 par ligne |
| somme naïve (lignes × anciens voisins) | ~31 000 | 617 989 |

L'union est **38 fois plus petite** que la somme ligne par ligne, comme le §5 le supposait : les
lignes d'un même lot sont voisines entre elles.

**La référence de durée** de `TenThousandRowsInBatchesOf512` sur le nouvel élagage : 19,5 s,
24,3 s et 20,1 s (charge de 5 à 8). Le plafond ×2 se situe donc vers 40 à 48 s.

**Le coût attendu, une estimation à vérifier** : `110a65f15` donnait ~800 s pour la recherche
naïve par ligne, ~617 000 contrôles, soit ~1,3 ms par contrôle. 16 000 contrôles en fin
d'instruction feraient ~21 s de plus, **à peu près ×2, à la limite du plafond**. Si la mesure
dépasse, le premier levier : ne recontrôler que les nœuds qui ont PERDU une arête entrante
(anciens voisins et écartés), et non les lignes mises à jour elles-mêmes, que leur réinsertion
vient de relier.

**Une remarque du cœur C++ pour l'étape 3** (relecture du 10 octobre) : la même question qu'à
l'étape 1 se pose à `NodeTable::delete_`. Une ligne locale supprimée avant le COMMIT n'a jamais
été dans l'index vectoriel, et `index->delete_` sur elle est au mieux un non-sens silencieux.
