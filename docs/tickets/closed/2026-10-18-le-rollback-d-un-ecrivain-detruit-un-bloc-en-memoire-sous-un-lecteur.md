# Le rollback d'un écrivain détruit un bloc en mémoire sous un lecteur : le lecteur se fige sur un verrou mort

- **État** : corrigé le 18 octobre 2026 par le commit qui ferme ce ticket (cœur C++) : blocs et
  groupes passent en propriété partagée (`GroupCollection` en `shared_ptr`, `getGroupShared`) et
  tout lecteur qui utilise un bloc ou un groupe hors du verrou de sa collection en garde une
  référence le temps de l'usage (les deux balayages en mémoire du CSR, `isVisible`,
  `wasWrittenByCommitAfter`, et l'état de balayage de table d'un appel à l'autre). Le rollback ne
  change pas : il retire les blocs et groupes de queue vidés comme avant, et l'objet survit tant
  qu'un lecteur le tient. Témoin `test/transaction/rollback_under_readers_test.cpp`, cinq cas ;
  celui du lecteur en boucle sous soixante COPY annulés d'un autre écrivain est rouge sur master
  (le fils meurt d'un signal 11 après 64 s en Release ; heap-use-after-free dans
  `ChunkedNodeGroup::scan` sous `-fsanitize=address`) et vert sur le correctif.
- **Gravité** : blocage définitif du processus (un worker du moteur attend pour toujours un
  verrou dont l'objet est détruit ; l'hôte ne sort plus sur TERM), ou lecture après libération.
- **Atteignable en service** : oui, en mode multi-écrivains (le défaut depuis le 11 octobre) :
  il suffit d'un lecteur qui balaie les relations en mémoire d'un nœud pendant qu'un autre
  écrivain annule une insertion dans la même table.
- **Touche rag3weaver** : oui, c'est l'hôte de luciepc qui s'est figé (18 octobre 2026, pid
  1043752, binaire 6041f9194, lib 0fc7dd10f ; pile capturée par Lucie à 16 h 26 :
  `~/.cache/rag3weaver-notes-c7/hote-bloque-18-oct-stack1043752.txt`).
- **Ouvert le** : 18 octobre 2026, session cœur C++, sur demande de l'orchestration.
- **Pour** : cœur C++, après le lot « BEGIN refusé », avant le ticket du COMMIT au point de reprise
  en échec.

## Ce qu'on voit

Le thread principal de l'hôte attend l'arrêt du réacteur ; le réacteur attend une requête de
`oublier_les_aretes_du_fichier` dans `TaskScheduler::scheduleTaskAndWaitOrError` ; un seul
worker du moteur est dans le stockage, arrêté dans `__pthread_rwlock_rdlock` sous
`ChunkedNodeGroup::scan` ← `CSRNodeGroup::scanCommittedInMemSequential` ← `scanCommittedInMem`
← `CSRNodeGroup::scan` ← `RelTable::scanInternal` ← `ScanRelTable`. Les 80 autres threads sont
inactifs ou hors du moteur : personne ne tient ce verrou en écriture. Tous les verrous du moteur
sont RAII : un verrou tenu sans thread n'existe pas. Le verrou lu est donc celui d'un objet
détruit, dont la mémoire réemployée ressemble à « pris en écriture ».

## La cause (lue)

`CSRNodeGroup::scanCommittedInMemSequential` (`csr_node_group.cpp:278-303`) prend le pointeur du
bloc sous le verrou de la collection, **le rend**, puis balaie sous `inMemAppendMtx` partagé :

```cpp
{ const auto lock = chunkedGroups.lock(); chunkedGroup = chunkedGroups.getGroup(lock, chunkIdx); }
{ const std::shared_lock appendGuard{inMemAppendMtx}; chunkedGroup->scan(...); }
```

`NodeGroup::rollbackInsert` (`node_group.cpp:426-437`) prend la collection puis `inMemAppendMtx`
exclusif, et **détruit les blocs de queue vides** (`removeTrailingGroups`, `groups.erase`).
Entre les deux accolades du lecteur, l'annulation d'un autre écrivain passe, et le bloc que le
lecteur va balayer est libéré : la liste CSR en mémoire du nœud (`inMemCSRList`, copiée du
`CSRIndex`) contient les lignes de l'insertion non validée (le filtre de versions les écarte
ensuite), donc le lecteur désigne bien ce bloc. `scanCommittedInMemRandom` a le même motif, par
construction (le verrou est rendu « le temps de changer de bloc », pour éviter l'inversion
d'ordre avec l'écrivain, qui tient la collection puis `inMemAppendMtx`).

Un étage plus haut, même chose pour les groupes de nœuds : `NodeGroupCollection::rollbackInsert`
(`node_group_collection.cpp:253-258`, appelé par `RelTableData` ligne 343 et par la table de
nœuds) détruit les groupes de queue vides sous le seul verrou de la collection, alors que
`NodeTable::scan` et `RelTableData` prennent un pointeur de groupe (`getNodeGroup`) et le gardent
dans l'état de balayage pour toute la fenêtre.

Ni la compaction du CSR (c44af8031 : elle raccourcit des listes, ne détruit aucun bloc) ni E10
(87a362e30 : il retire une colonne au rollback, sous les verrous de la collection et du groupe)
ne touchent à ces chemins ; master 87a362e30 a exactement le même code que la lib de l'hôte sur
ce point.

## Recette

Pas déterministe sans point d'arrêt : la fenêtre est entre deux prises de verrou. À jouer sous
`-fsanitize=address` : un lecteur qui balaie en boucle les relations d'un nœud (`MATCH
(a:A {id: 0})-[r:R]->(b) RETURN count(r)`) pendant qu'un second écrivain, dans une autre
connexion, `BEGIN ; CREATE` 2 000 relations depuis ce nœud ; `ROLLBACK`, en boucle. ASAN rend
« heap-use-after-free » dans `ChunkedNodeGroup::scan` en quelques tours ; en Release, c'est le
blocage de l'hôte ou une lecture silencieuse.

## Témoin

Aucun encore. À écrire dans `test/transaction/concurrence/` (deux fils, la recette), avec sa
preuve sous ASAN ; en Release il sert de garde contre le blocage (délai).

## Forme retenue

La propriété partagée, et non « ne rien détruire au rollback ». La première forme essayée (garder
les blocs et groupes vidés en place) rendait les lignes annulées visibles : le balayage d'un
groupe borne par les lignes de chaque bloc et non par le compte du groupe, un groupe de nœuds
vide sans aucun bloc était balayé hors tableau, et les listes CSR désignaient encore les lignes
du bloc survivant. Prolonger la vie des objets tenus par un lecteur ne change aucune règle de
visibilité : c'est la forme sûre. Le coût : un compteur atomique par prise de référence dans les
balayages en mémoire du CSR (une par bloc, pas par ligne).

## Le second défaut, lu puis vu (cœur C++, 18 octobre 2026)

Les listes CSR en mémoire (`CSRIndex::indices[nœud].rowIndices`) reçoivent les lignes à l'append
(`updateCSRIndex`) et rien ne les en retirait au rollback. Un CREATE annulé ne le montre pas (il
vit dans le stockage local), ni un COPY annulé dans une table vide (ses lignes vont au bloc
persistant, retrouvées par l'en-tête CSR). Il se voit dès qu'un COPY annulé s'ajoute à des
relations déjà en mémoire ou persistantes d'un même nœud : c'est le témoin du lecteur en boucle,
une fois le bloc gardé vivant par la garde partagée, qui l'a montré (265 lectures fausses sur
268 : la liste du nœud 0 désignait les 1 500 lignes du COPY annulé, tronquées du bloc, et le
balayage les lisait au-delà des lignes du bloc). Deux pièces dans le même commit :
`CSRNodeGroup::rollbackInsert` (`rollbackInsert` devient virtuel) retire des listes, sous
`csrIndexMtx` exclusif, toute ligne à partir de `startRow` ; et les deux balayages en mémoire
bornent par les lignes du bloc (`getNumRows`), pour une liste copiée avant l'annulation : tout ce
qui dépasse est annulé. Les témoins `ARolledBackRelInsertLeavesNoRowInTheNodesList` et
`ARolledBackFirstRelInsertIsStillInvisibleToACopiedList` (table vide) passent sur master et
restent comme gardes ; `AReaderSurvivesTheRollbackOfAnotherWriter` juge les deux défauts
(`reader-survives-rollback`, `reader-sees-only-committed-rows`), et
`ARolledBackCopyDoesNotComeBackWithTheNextCopy` le second sans concurrence : relations d'avant
en mémoire, un COPY annulé puis un COPY validé comptaient 3 010 au lieu de 1 510 sur master (les
1 500 lignes annulées revenaient avec le suivant).

Résidu, noté sans correctif : une transaction qui tient une copie de liste prise avant une
annulation et qui réemploie elle-même ces emplacements dans la même instruction peut lire ses
propres lignes sous le nœud d'avant ; la garde des copies en vol prévue pour la lecture en
mémoire (page `coeur-cpp/13` §6) le fermerait.
