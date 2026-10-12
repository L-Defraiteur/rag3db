# Un index retiré ou recréé change dans le stockage pour toutes les transactions, avant sa validation

- **État** : ouvert (forme arrêtée avec le cœur C++ : la rétention, comme les tables)
- **Gravité** : refus d'une lecture valide, et travail perdu. Une recherche liée avant un DROP
  reçoit un refus nommé au lieu de voir l'ancien index. Un CREATE annulé bloque le suivant
  (« Index with name … already exists »). Aucune perte ni réponse fausse depuis le pas 1 du ticket
  fermé.
- **Atteignable en service** : oui, par `ajuster_l_index_pour_le_retard` (DROP, rattrapage,
  CREATE) pendant que d'autres connexions cherchent ou écrivent.
- **Touche rag3weaver** : oui (refus transitoires pendant une reconstruction).
- **Ouvert le** : 12 octobre 2026, session cœur C++ hotfixs, suite de
  `closed/2026-10-12-une-recherche-vectorielle-pendant-drop-create-de-l-index-rend-une-exception-brute.md`.
- **Pour** : hotfixs.

## Ce que c'est

Le catalogue est versionné. La liste des index du stockage (`NodeTable::indexes`) ne l'est pas.
- Le DROP retire l'index du stockage tout de suite, pour tout le monde, avant sa validation.
  Une annulation ne le rend pas.
- Le CREATE l'y ajoute avant sa validation et avant de remplir ses tables de relations.
  Une annulation ne l'en retire pas.
- Ni l'un ni l'autre ne prend le verrou INDEX de la table : ils ne se sérialisent pas avec les
  écrivains, qui parcourent la liste (`getIndexes()`) sans son verrou.

Depuis le pas 1, la recherche est sûre : elle capture l'index et refuse par un nom ce qui n'est
pas celui de son instantané. Mais elle refuse là où un lecteur d'avant devrait voir l'ancien index
entier.

## Forme arrêtée (avec le cœur C++)

Comme `reclaimDroppedTables` pour les tables :
- **Le DROP** marque le holder retiré (sa transaction, puis son horodatage de validation), sans
  l'effacer. La suppression physique se fait au point de reprise, quand plus aucune transaction
  d'avant n'est ouverte. Une annulation démarque.
- **Le CREATE** ajoute un holder lié à sa nouvelle entrée de catalogue. Il n'est visible que par
  cette entrée, donc invisible aux autres jusqu'à la validation. Une annulation le retire.
- **La résolution par l'entrée** (pas 1) fait trouver au lecteur d'avant l'ancien holder entier,
  à celui d'après le nouveau ou le refus nommé.
- **Le holder retiré** est invisible à `hasLoadedSecondaryIndex`, `lockIndexForWrite` et
  `commitInsert` pour la transaction qui l'a retiré et pour celles qui commencent après sa
  validation. Les écrivains d'avant continuent de l'entretenir : c'est du travail perdu, pas une
  faute.
- **Le DROP et le CREATE** prennent le verrou INDEX exclusif de la table : ils se sérialisent
  avec les écrivains, pas avec les lecteurs. Un écrivain qui sort de l'attente et trouve l'index
  retiré rafraîchit son instantané (I1) et passe sans index.

Un trou à fermer avec le verrou : update et delete sur une table sans index secondaire ne prennent
pas le verrou INDEX (`lockIndexForWrite` rend la main), et parcourent pourtant la liste (la clé
primaire seule). Un CREATE peut y faire un `push_back` pendant leur parcours. Le CREATE prendra
INDEX exclusif, et ces écritures INDEX partagé même sans index secondaire. Prendre plutôt le verrou
de la liste dans les parcours des écrivains n'est pas sûr : `insert` et `lockIndexForWrite` le
reprennent par `hasLoadedSecondaryIndex` (un `shared_mutex` repris s'interbloque derrière un DDL
qui attend), et `acquireLocks` attendrait jusqu'à `lock_timeout` en le tenant.

## Témoins à écrire

- Le lecteur d'avant voit l'ancien index entier : une transaction en lecture ouverte avant un
  DROP validé cherche, et trouve les résultats d'avant.
- Annuler un CREATE, puis le refaire : pas de « already exists ».
- Le témoin du banc (`VectorIndexRebuild.SearchesAcrossDropAndCreateAreConsistentOrNamed`)
  reste vert, rejoué 10 fois.

## Pour le fermer

La forme ci-dessus, avec les trois témoins verts.
