# Sous écrivains simultanés, l'index vectoriel refuse une instruction : « node offset N is beyond the visited set of N nodes »

- **État** : corrigé (session cœur C++ hotfixs, 11 octobre 2026 au soir ; voir « Correctif »)
- **Gravité** : refus d'une instruction valide (aucune perte vue : l'intégrité reste juste après le refus et après une réouverture)
- **Atteignable en service** : oui, depuis la bascule du défaut (`a0a79016e`), dès que deux écrivains insèrent ou mettent à jour une même table indexée par HNSW
- **Touche rag3weaver** : oui en principe, puisque ses tables de morceaux portent un index vectoriel et que le mode est maintenant le défaut. Non vu en usage.
- **Ouvert le** : 11 octobre 2026, session du banc (preuve de la bascule)
- **Pour** : à attribuer (extension vector sous le mode multi-écrivains)

## Ce que c'est

Avec plusieurs écrivains sur une table indexée, des instructions échouent sur :

> Runtime exception: HNSW index: node offset 310 is beyond the visited set of 310 nodes. The index
> graph or the row count of its table is wrong; drop the index and build it again.

Le message demande de rebâtir l'index. Le vérificateur d'intégrité ne trouve pourtant aucune
violation, ni dans le processus écrivain ni après fermeture et réouverture.

## Recette minimale

Aucune de déterministe pour l'instant.

## Témoin

`Launchers/ConcurrencyBench.H4_IndexedRandomMix/Thread_Hot` et `/Thread_Crash` (le mélange de C7 sur
une table indexée), contrôle `no-unexpected-error`. Ils rougissent **10 fois sur 10 sur master
`fd10b8c27` et `481afb550`, hors bascule**, et autant sur `4ee70e386`. L'erreur vient de plusieurs
écrivains par passe, avec un offset chaque fois égal à la taille de l'ensemble visité : 307, 310,
329. Le même témoin montrait aussi un refus des verrous (« no primary key to lock: it is not
visible »), corrigé par le cœur C++ sur `verrous-pk-lock` : après ce correctif, ce ticket suffit à
garder H4 rouge.

Le témoin minimal, écrit ensuite par le banc :
`VectorIndexUpdate.AnUpdateWaitingBehindAnInsertSeesItsNewRows`
(`test/transaction/concurrence/vector_index_update_test.cpp`). Un SET de vecteur attend l'index
derrière une insertion de 50 lignes non validée, puis reprend après la validation. Rouge 3 fois
sur 3 sur master `2fa7502e7` : « node offset 321 (329, 324) is beyond the visited set of 300
nodes ».

## Cause

L'hypothèse du cœur C++ était la bonne. L'état d'une écriture sur le graphe (`initUpdateState`,
`initInsertState`) est créé par l'opérateur avant que `NodeTable::update` prenne le verrou de
l'index (`lockIndexForWrite`). Son ensemble visité a la taille de la table à ce moment-là. Sous
le mode multi-écrivains, l'attente du verrou reprend l'instantané (`refreshSnapshot`) : le graphe
atteint alors, par des arêtes validées entre-temps, les lignes de l'autre écrivain, au-delà de
l'ensemble. Le contrôle de borne le refusait au lieu de lire hors du tableau.

## Correctif

`VisitedState::ensureSize` agrandit l'ensemble en gardant ses marques, sans jamais le rétrécir.
`OnDiskHNSWIndex::fitVisitedToTable` l'ajuste au nombre de lignes de la table
(`getNumTotalRows`, lignes locales comprises) au début de chaque opération sur le graphe :
`update`, `delete_`, `finalizeDelete` et `commitInsert`, et par précaution `search` (la relecture
du cœur C++ le demandait ; un témoin, un SET qui attend en amont d'un `QUERY_VECTOR_INDEX` dans la
même instruction, passe déjà sur master : il n'a pas été gardé). Le contrôle de borne reste, pour
un graphe qui désignerait une ligne hors de la table. On n'a pas retenu « ignorer un voisin au-delà
de l'ensemble » : la recherche aurait perdu des voisins valides.

Sur master `cad384056`, qui porte déjà le correctif des verrous du cœur (« no primary key to
lock ») : le témoin passe 5 fois sur 5 avec le correctif ; H4 (Thread_Hot et Thread_Crash) aussi,
5 fois sur 5, sans aucune des deux erreurs. Le témoin et les deux lignes de H4 sortent de
`known_red.txt`. Sans le correctif, sur la même base, H4 rougit 3 fois sur 3, et seulement sur
« beyond the visited set » (offsets 310 à 322, chaque fois égaux à la taille de l'ensemble) : le
vert revient à ce correctif, pas à celui des verrous.
