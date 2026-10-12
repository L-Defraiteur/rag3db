# Une recherche vectorielle pendant un DROP/CREATE de l'index rend une exception brute de la bibliothèque standard

- **État** : corrigé (session cœur C++ hotfixs, 12 octobre 2026 ; voir « Correctif »). La rétention, qui fait voir l'ancien index au lecteur d'avant, suit dans le ticket 2026-10-12-un-index-retire-ou-recree-reste-visible-a-tous-dans-le-stockage
- **Gravité** : erreur non nommée (« bad optional access », « unordered_map::at »). Aucun figement, aucun résultat faux vu
- **Atteignable en service** : oui. rag3weaver retire l'index le temps d'un long rattrapage d'embarquement, puis le recrée (`ajuster_l_index_pour_le_retard`, puis `rebuild_vector_index`), pendant que d'autres connexions du même processus cherchent
- **Touche rag3weaver** : oui. Ses recherches peuvent recevoir ces messages bruts, qu'aucun appelant ne sait reconnaître
- **Ouvert le** : 12 octobre 2026, session du banc (une exploration demandée par l'orchestration, sur l'hypothèse du gel de l'hôte de luciepc)
- **Pour** : hotfixs, qui s'est proposé

## Ce que c'est

Dans un même processus, une connexion retire puis recrée l'index vectoriel d'une table
(`DROP_VECTOR_INDEX`, des vecteurs posés sans lui, `CREATE_VECTOR_INDEX`), pendant qu'une autre
lance `QUERY_VECTOR_INDEX` en boucle. La plupart des recherches faites pendant l'absence de
l'index sont refusées par un nom (l'index n'existe pas), ce qui est attendu. Mais quelques-unes
échouent sur une exception brute de la bibliothèque standard C++, sans nom du moteur :

> bad optional access
> unordered_map::at
> vector::reserve (vue sur master 6ae69c6b1, deux passes sur trois, après le lot des fils)

Sur trois passes de 6 s chacune, il y a eu 3 à 9 « bad optional access » et 3 « unordered_map::at »
par passe, pour 30 000 à 53 000 refus nommés. Les autres contrôles restent verts :
- aucune recherche ne dépasse 41 ms : pas de figement ;
- aucun résultat incohérent ;
- la reconstruction elle-même ne rate jamais.

## Recette minimale

Le témoin ci-dessous. Une recherche qui tombe entre deux étapes du DROP ou du CREATE (catalogue et
stockage de l'index pas encore d'accord) est probablement en cause, mais ce n'est pas lu dans le
code.

## Témoin

`VectorIndexRebuild.SearchesAcrossDropAndCreateAreConsistentOrNamed`
(`test/transaction/concurrence/vector_index_rebuild_test.cpp`), contrôle
`no-unnamed-search-error`, en known_red. Il est rouge trois passes sur trois sur master
`5fa4d2489`.

## Cause

Le catalogue est versionné (MVCC), la liste des index du stockage (`NodeTable::indexes`) ne l'est
pas, et n'avait pas de verrou. La recherche lit le catalogue à son instantané mais le stockage
courant, sans prendre de verrou (les lectures n'en prennent pas).
- Le DROP (`_DROP_HNSW_INDEX`) retire l'index du stockage tout de suite, avant sa validation, et le
  détruit. Une recherche liée avant trouvait le catalogue d'accord et le stockage vide :
  `getIndex(name).value()` levait « bad optional access ». Une recherche déjà lancée gardait une
  référence pendante : elle lisait la mémoire libérée de l'index (`getTable` d'un oid au hasard,
  « unordered_map::at » ; `numTotalRows - numCheckpointedNodes`, « vector::reserve »).
- Le CREATE ajoute son index au stockage avant de valider et avant de remplir ses tables de
  relations : une recherche liée à l'ancien index pouvait lire le nouveau.
- Une annulation effaçait puis remettait une entrée du catalogue sans le verrou du `CatalogSet` :
  « unordered_map::at » à un lecteur entre son `contains` et son `at`.

## Correctif

- `NodeTable::indexes` passe sous un `shared_mutex` (accès par nom, ajout, retrait). Le holder porte
  un `shared_ptr<Index>`, et `NodeTable::getSharedIndex` le rend.
- `QUERY_VECTOR_INDEX` et `VECTOR_SEARCH` capturent l'index à l'initialisation
  (`OnDiskHNSWIndex::captureForQuery`) : il reste vivant jusqu'à la fin de la recherche, quoi que
  fasse le DROP. L'index capturé doit être celui que l'instantané désigne, reconnu à ses tables de
  relations (l'oid de la relation unique de chaque groupe). Absent, ou d'une autre génération :
  refus nommé, « Index … does not exist any more: a concurrent transaction dropped or rebuilt it
  after this query was bound ».
- La liaison distingue un index retiré par un DROP concurrent (même refus nommé) d'un index
  détaché par le rejeu (« is behind its table », comme avant). Le DROP concurrent se reconnaît au
  catalogue : la tête de la chaîne de l'entrée est une version que l'instantané du lecteur ne voit
  pas, non validée d'une autre transaction ou validée après son début
  (`Catalog::isIndexChangedAfterSnapshotOf`) : l'absence d'exemplaire ne suffit pas, puisqu'un index
  détaché, passé un point de reprise, n'en a plus du tout (la liste complète l'a montré :
  `VectorIndexCrashReopenTest.TheLateStateSurvivesACheckpoint`).
- Les extensions chargent un index sous le verrou de la liste (`NodeTable::loadIndex`).
- L'annulation du catalogue prend le verrou du `CatalogSet` (commit à part).

Témoin : vert 10 passes sur 10, sans aucune erreur brute ni résultat incohérent, contre 12
« bad optional access », 2 « unordered_map::at » et 3 « vector::reserve » par passe sur master.

Reste, au ticket de la rétention : un lecteur d'avant le DROP reçoit le refus nommé au lieu de voir
l'ancien index ; le CREATE reste visible aux écrivains avant sa validation ; le DROP ne se
sérialise pas encore avec les écrivains par le verrou INDEX ; un CREATE annulé laisse son index au
stockage (le suivant reçoit « already exists »).

## Pour le fermer

Soit la recherche voit l'index entier ou pas du tout (sous le verrou d'index, ou par un instantané
du catalogue tenu jusqu'à la lecture). Soit elle traduit ces deux cas en refus nommé (« index …
does not exist »). Le témoin au vert, rejoué ×10.

Ce que l'exploration écarte, pour l'hypothèse du gel de luciepc : ce chemin ne fige pas le moteur.
