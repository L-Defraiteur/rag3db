# Une recherche vectorielle pendant un DROP/CREATE de l'index rend une exception brute de la bibliothèque standard

- **État** : ouvert
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

À instruire. Les deux messages sont ceux de `std::bad_optional_access` (un `.value()` sur un
optionnel vide) et de `std::out_of_range` (un `.at()` sur une clé absente). Ce sont des lectures
de l'état de l'index qui supposent qu'il existe entre le moment où la recherche le trouve au
catalogue et celui où elle le lit.

## Pour le fermer

Soit la recherche voit l'index entier ou pas du tout (sous le verrou d'index, ou par un instantané
du catalogue tenu jusqu'à la lecture). Soit elle traduit ces deux cas en refus nommé (« index …
does not exist »). Le témoin au vert, rejoué ×10.

Ce que l'exploration écarte, pour l'hypothèse du gel de luciepc : ce chemin ne fige pas le moteur.
