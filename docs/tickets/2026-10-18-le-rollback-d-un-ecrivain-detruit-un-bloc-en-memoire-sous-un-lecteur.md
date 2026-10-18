# Le rollback d'un écrivain détruit un bloc en mémoire sous un lecteur : le lecteur se fige sur un verrou mort

- **État** : à confirmer (lu dans le code sur la pile de l'hôte figé ; pas encore reproduit)
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

## Forme proposée

Ne rien détruire au rollback. Les blocs et groupes de queue vides restent en place : le prochain
`append` les remplit (un bloc vide n'est pas plein), et le point de reprise, qui s'écrit sans
aucune transaction ouverte, est le seul à les retirer. C'est la forme la plus simple et elle
supprime les deux étages du défaut sans toucher à l'ordre des verrous. L'autre forme, différer la
destruction tant qu'un balayage tient le bloc (un compte de lecteurs en vol par groupe, comme la
garde `inFlightListCopies` prévue pour la compaction), est plus lourde et ne vaut que si garder
des blocs vides gêne quelque chose (à vérifier : `getNumEmptyTrailingGroups`, l'assertion
`numGroups >= 1` du point de reprise, la sérialisation des groupes vides).

Fermé quand le témoin passe sous ASAN et que l'hôte rejoue la réindexation de luciepc sans se
figer.
