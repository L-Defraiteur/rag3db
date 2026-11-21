# C1 : la lecture du chemin des dictionnaires, et la garde qui change le plantage en erreur nommée

Le 21 novembre 2026, au partage de c7 : le cœur garde la reproduction de C1 (ses tours, sa sonde, ASan) ; hotfixs prend
la lecture du chemin et la pièce 2 du correctif. Le ticket est
`docs/coeur-cpp/tickets/2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines.md`.

## La lecture : l'hypothèse des deux versions est infirmée

L'hypothèse était que les enfants OFFSET et DATA d'un même segment de chaînes sont publiés à deux versions différentes.
Lu sur master (e13d055094) :

- `Column::checkpointSegment` (`column.cpp:402-436`) travaille en copie : il relit le segment persistant dans un morceau
  neuf, y fusionne les écritures, le flush, puis rend l'ancien (`retireStorage`). Le segment persistant n'est jamais
  modifié en place, et les segments inchangés sont partagés.
- La relecture (`DictionaryColumn::scan`, `dictionary_column.cpp:39-70`) lit DATA puis OFFSET depuis un `SegmentState`
  local, dont `ColumnChunkData::initializeScanState` (`column_chunk_data.cpp:360-372`) a copié les métadonnées par
  valeur. Personne d'autre ne l'écrit.
- `StringChunkData::flush` et `DictionaryChunk::flush` écrivent les octets, puis les offsets, sur un objet neuf, invisible
  jusqu'au remplacement du segment entier. `split` reconstruit le dictionnaire par `appendString`.
- Le seul chemin qui écrirait DATA et OFFSET séparément sur un segment publié, le commit en place
  (`DictionaryColumn::append`, `canCommitInPlace`, `dictionary_column.cpp:190-283`), n'a **aucun appelant**.
- Les pistes vérifiées en plus ne donnent rien :
  - les statistiques de compression des offsets sont recalculées au flush ;
  - `resetNumValuesFromMetadata` n'a pas d'appelant externe ;
  - un état de relecture pris sur un autre segment n'existe pas ;
  - l'ajout d'un segment entier repasse par `appendString`.

**Ce que le core rend impossible par le code.** Un dictionnaire neuf naît avec 0 octet de capacité
(`dictionary_chunk.cpp:24-33`). `appendString` ne l'agrandit qu'en doublant, et le plantage tombe à son tout premier
appel (`pos = 0`). La capacité de 131 072 octets du core vient donc de `DictionaryColumn::scan`, qui agrandit d'après
`bit_ceil(dataMetadata.numValues)`, donc avec plus de 65 536 octets annoncés. `Column::scanSegment` fixe ensuite
`numValues` à cette même valeur. Les 4 991 octets du core ne sortent d'aucun chemin du code : des champs du morceau neuf
ont été **écrasés**, et non mal publiés. Cela rejoint le débordement vers le bloc voisin relevé par le cœur, qu'ASan n'a
pas vu en treize tours. La sonde du cœur (branche `c1-scan-probe`, contrôles à la relecture et avant le premier `write`)
dira où naît l'écrasement.

## La pièce 2 : la garde (sur master, 4fcbe09fc3)

- `DictionaryChunk::sanityCheck` comparait le nombre d'offsets à lui-même, et rien ne l'appelait. Il vérifie maintenant
  que chaque offset tient dans les octets et qu'aucun ne décroît (`firstInconsistentOffset`).
- `DictionaryColumn::scan` refuse par nom un dictionnaire relu incohérent : « Corrupted string dictionary read back for
  column … : offset i of n is v (the one before is p), for b bytes of string data ».
- `DictionaryChunk::flush` refuse d'écrire un dictionnaire incohérent (« … to write … »).

Le témoin, `StringRewrite.AnInconsistentDictionaryIsRefusedByNameAtReadBack` (`concurrence_test`) :
1. il écrit 3 000 chaînes, puis fait un CHECKPOINT ;
2. par les internes, il ramène de 28 890 à 16 octets le `numValues` de l'enfant DATA du segment persistant ;
3. il fait une mise à jour, puis un CHECKPOINT dans un fils (`EXPECT_EXIT`, style threadsafe) ;
4. il rend ensuite au segment ses octets, pour que le parent ferme sur un segment sain.

**Rouge sur master e13d055094** : le fils meurt d'un SIGSEGV, la forme de C1. **Vert avec la garde** : le fils reçoit le
refus nommé. Passe de tête verte sur d66c6a65b4 ; seul y rougit DropNodeTableReclaim, admis.

La garde ne trouve pas la cause : elle change un plantage du processus entier en erreur qui nomme la colonne, et son
mot (« read back » ou « to write ») dira si l'état naît à la relecture ou avant une publication.
