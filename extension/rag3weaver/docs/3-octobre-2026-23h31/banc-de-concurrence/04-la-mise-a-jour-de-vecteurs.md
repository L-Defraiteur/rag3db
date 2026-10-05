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
