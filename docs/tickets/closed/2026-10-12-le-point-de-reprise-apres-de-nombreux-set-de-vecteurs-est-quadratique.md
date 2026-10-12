# Le point de reprise qui suit de nombreux SET d'une colonne de vecteurs coûte un temps quadratique

- **État** : corrigé (le commit qui déplace ce fichier dans `closed/`, 12 octobre 2026, cœur C++, page coeur-cpp/13) — `ColumnChunkData::resize` grandit au moins du double quand il doit grandir (la capacité suit le tampon, le disque reçoit numValues) : le point de reprise applique les mises à jour une valeur à la fois sans plus recopier tout le tampon à chacune. Témoin du banc `Columns/CheckpointAfterUpdates.ScalesWithTheUpdatedRows` sorti du known_red. Ouvert le 12 octobre 2026 ;  ; le correctif est écrit dans le lot de l'insertion du cœur C++ (branche
  insertion-vecteurs, pas 1)
- **Gravité** : lenteur sévère. Un CHECKPOINT après 15 000 SET de FLOAT[768] prend 24 s, contre
  0,5 s pour le même volume écrit par COPY
- **Atteignable en service** : oui. rag3weaver pose chaque vecteur par un SET depuis NULL
  (`embed_set`) ; le premier index d'un dépôt en écrit des dizaines de milliers avant le point de
  reprise
- **Touche rag3weaver** : oui. Le point de reprise automatique ou explicite qui suit
  l'embarquement tient la base pendant ce temps
- **Ouvert le** : 12 octobre 2026, session du banc (tiré du témoin du coût vectoriel : le
  CHECKPOINT de la forme du produit prenait 19 à 25 s)
- **Pour** : le cœur C++

## Ce que c'est

Une table chargée par COPY avec une colonne FLOAT[768] à NULL, puis un SET de chaque ligne par lots
de 64, sans index et sans point de reprise entre-temps. Le CHECKPOINT qui suit coûte un temps qui
croît comme le carré du nombre de lignes mises à jour. Une colonne STRING ou INT64 ne le fait pas.

## Mesuré (master 2a3f57f0d, poste principal, sous `poste lourd`)

| colonne | lignes mises à jour | SET par lots de 64 | CHECKPOINT après | référence : CHECKPOINT après COPY |
|---|---|---|---|---|
| FLOAT[768] | 1 500 | 95 ms | 153 ms | 55 ms |
| FLOAT[768] | 15 000 | 795 ms | **23 993 ms** (×156) | 542 ms |
| STRING (200 octets) | 1 500 → 15 000 | 17 → 199 ms | 3,9 → 35 ms (×9) | 5,5 → 33 ms |
| INT64 | 1 500 → 15 000 | 13 → 183 ms | 0,7 → 2,0 ms | 3,2 → 4,3 ms |

Le profil par échantillonnage du point de reprise de la forme du produit : `ColumnChunkData::resize`
fait 80 % du temps propre, sous `NodeGroup::checkpointInMemAndOnDisk`,
`scanCommittedUpdatesForColumn`, `UpdateInfo::iterateVectorInfo`,
`LazySegmentScanner::applyCommittedUpdates` et `ListChunkData::write`.

## Cause (lue dans le code, confirmée par le cœur C++)

Au point de reprise, `LazySegmentScanner::applyCommittedUpdates` (`lazy_segment_scanner.cpp`)
applique les mises à jour validées une ligne à la fois (`segmentData->write(..., 1)`). Pour une
colonne de listes ou de tableaux, `ListChunkData::write(const ValueVector*, ...)`
(`list_chunk_data.cpp`) agrandit la donnée au plus juste,
`dataColumnChunk->resize(getNumValues() + appendSize)`, et `ColumnChunkData::resize`
(`column_chunk_data.cpp`) réalloue et recopie tout le tampon chaque fois. N mises à jour coûtent
donc de l'ordre de N² octets copiés.

La colonne STRING passe par `DictionaryChunk::appendString` puis le même `resize`, mais elle reste
linéaire ici : à instruire, si le correctif ne la couvre pas d'une autre façon.

## Témoin

`Columns/CheckpointAfterUpdates.ScalesWithTheUpdatedRows`
(`test/transaction/concurrence/checkpoint_after_updates_test.cpp`), contrôle
`checkpoint-linear-in-updated-rows` : le CHECKPOINT après 15 000 SET au plus vingt fois celui après
1 500, plus 50 ms (un coût linéaire fait environ ×10, un coût quadratique environ ×100). Le cas
`Vector768` est en known_red, rouge deux passes sur deux (153 puis 130 ms après 1 500 SET, 23 993 puis 24 309 ms après 15 000) ; `String` et `Int64` sont verts.

## Pour le fermer

Le correctif du cœur : `ColumnChunkData::resize` grandit au moins du double quand il doit grandir.
Le témoin vert, et le CHECKPOINT de la forme du produit (`VectorIndexCost.ProductShapeDeleteCost`,
qui l'imprime) remesuré.
