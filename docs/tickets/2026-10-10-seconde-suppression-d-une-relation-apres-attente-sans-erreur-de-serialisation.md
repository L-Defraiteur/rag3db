# Sous les verrous, une seconde suppression ou mise à jour de la même relation, après attente, reçoit « Write-write conflict » et non l'erreur de sérialisation

- **État** : ouvert (marche A4′, 10 octobre 2026 ; relevé à la relecture du banc).
- **Gravité** : réponse fausse de nom seulement (le refus est juste, son nom n'est pas celui de
  l'option A ; un appelant qui rejoue sur « could not serialize » ne rejouera pas sur celui-ci).
- **Atteignable en service** : non (mode multi-écrivains, éteint hors du banc).
- **Touche rag3weaver** : non tant que le mode reste éteint.

## Ce que c'est

A4′ verrouille les deux extrémités d'une relation en exclusif pour la supprimer ou la mettre à
jour, et contrôle après l'attente que les extrémités n'ont pas été supprimées par une validation
postérieure à l'instantané. Il ne contrôle pas **la relation elle-même** : si une autre transaction
l'a supprimée (ou mise à jour, même colonne) et validée pendant l'attente, la seconde la trouve
encore dans son instantané et reçoit, au moment d'écrire la version, le refus immédiat d'avant
(`VectorVersionInfo::delete_` : « Write-write conflict: deleting a row that is already deleted by
another transaction » ; `UpdateInfo::update` : « Write-write conflict of updating the same row »).
Le résultat est juste (jamais les deux) ; seul le nom de l'erreur n'est pas celui que l'option A
promet.

## Recette

Mode multi-écrivains, deux connexions, une relation validée r entre 1 et 2 : A `BEGIN`, A
`DELETE r` ; B `BEGIN`, B `DELETE r` (attend les extrémités) ; A `COMMIT` ; B reçoit
« Write-write conflict… » au lieu de « could not serialize access due to concurrent update ».

## Témoin

Aucun au banc (aucun cas de relation contre relation) ; à écrire avec la forme `holderAndWaiter`
(détenteur `DELETE r`, attendant `DELETE r`, `expectWaiterRefused(Refusal::SerializationFailure)`).

## Pour le fermer

Un `CSRNodeGroup::wasWrittenByCommitAfter(startTS, transactionID, source, rowIdx)` sur le modèle
de `ChunkedNodeGroup::wasWrittenByCommitAfter`, appelé par `RelTableData::update` et `delete_`
après `findMatchingRow`, qui lève l'erreur de sérialisation avant d'écrire la version.
