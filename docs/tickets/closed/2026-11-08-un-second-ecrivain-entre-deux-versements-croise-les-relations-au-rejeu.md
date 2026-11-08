# Un second écrivain entre deux versements croise les relations au rejeu

- **État** : corrigé (lot `versement-verrou-2`, 8 novembre 2026, session hotfixs)
- **Gravité** : réponse fausse — après un rejeu, des relations désignent les nœuds d'un autre
  écrivain, sans refus ni avertissement
- **Atteignable en service** : sous le mode multi-écrivains, quand deux écrivains écrivent dans la
  même table de nœuds pendant qu'une transaction verse ses nœuds locaux (un COPY de relations vers
  des nœuds qu'elle a créés), puis que la base meurt et se rejoue. L'hôte n'a aujourd'hui qu'un
  écrivain par base.
- **Touche rag3weaver** : par le moteur

## Ce que c'est

Depuis G1 (c4f8610b9, 7 novembre), IndexLookup verse les nœuds locaux d'une transaction dans leur
table avant de résoudre les clés d'un COPY de relations. Les nœuds y prennent leurs décalages
définitifs **en milieu de transaction**. Un second écrivain qui validait ensuite des nœuds dans la
même table les plaçait derrière eux.

En mémoire, tout est juste. Au rejeu, les transactions repassent dans l'ordre de leurs commits : les
nœuds du second écrivain, validé avant la première transaction, prennent les décalages que celle-ci
avait en service. Ses relations, journalisées avec ces décalages, désignent alors les nœuds de
l'autre. Mesuré par le témoin : en mémoire 1000→1001 et 1000→1105 ; après le rejeu 3000→3001 et
3000→1105.

Un COPY de nœuds en est protégé : il tient l'index de sa table en exclusif jusqu'à sa validation.

## La correction

- `IndexLookup::acquireLocksBeforeExecution` prend `LockResource::index(table)` en EXCLUSIVE sur
  chaque table de nœuds où la transaction a des nœuds locaux, avant l'exécution, comme
  `NodeBatchInsert`. Le verrou tient jusqu'à la fin de la transaction : le second écrivain attend,
  ou est refusé par le délai.
- `QueryProcessor::acquireLocksBeforeExecution` visite tous les opérateurs d'un pipeline, et non plus
  son seul puits : IndexLookup, au milieu du COPY de relations, n'était jamais appelé.

## Les témoins

`test/transaction/concurrence/provisional_offsets_test.cpp` :
- `ProvisionalOffsets.TwoFlushesInOneTransactionKeepEveryRelOnItsNodesThroughTheJournal` : deux
  versements, un second écrivain entre les deux, des relations locales avant et après, des mises à
  jour, puis une mort base ouverte et le rejeu. Rouge avant (`second-writer-waits`), vert après : le
  second écrivain est refusé par le délai, et chaque relation est sur ses nœuds.
- `ACopyOfRelsToLocalNodesKeepsItsEndpointsWhenAnotherWriterCommitsAfterIt` change de prémisse, pas
  d'attente. Le second écrivain écrit dans un fil, qui doit attendre la validation de la transaction,
  puis passe après elle.
