# Un point de reprise automatique meurt dans le dictionnaire d'une colonne de chaînes

- **État** : ouvert (14 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : plantage (SIGSEGV du processus entier)
- **Atteignable en service** : à établir. Il faut un point de reprise automatique, demandé par un
  COMMIT, sous vingt écrivains et des DDL.
- **Touche rag3weaver** : par le moteur (l'hôte meurt)
- **Signalé par** : cœur c++ — `TwentyWriters.DdlEveryMinute` ×10 sous profil, sur le correctif de
  l'annulation sous le verrou
- **Pour** : cœur c++

## Ce qu'on voit

`TwentyWriters.DdlEveryMinute`, tour 6 sur 10, joué à luciepc le 14 novembre sur `9b0e73c9c`
(branche `annulation-garde-les-autres-2`, master `6b60d77aa` plus le correctif de l'annulation
sous le verrou). Bâti Release dans `~/git_workspaces/rag3db-colonnes/build/moteur`, sous
`RAG3DB_PROFILE_CHECKPOINT=1`.

Le processus meurt d'un SIGSEGV après 1 min 40, **sans aucun refus du pool** et sans aucune
annulation d'un COMMIT refusé : le chemin neuf du correctif ne s'est pas exécuté. Le core est
gardé par systemd-coredump à luciepc (pid 3980144, 14:17:21).

```
#0 std::_Hash_bytes
#1 std::_Hashtable<DictionaryChunk::DictionaryEntry, …>::find
#2 DictionaryChunk::appendString
#3 StringChunkData::write
#4 Column::checkpointSegment
#5 ColumnChunk::checkpoint
#6 NodeGroup::checkpointInMemAndOnDisk
#7 NodeGroup::checkpoint
#8 NodeGroupCollection::checkpoint
#9 NodeTable::checkpoint
#10 StorageManager::checkpoint
#11 Checkpointer::writeCheckpoint
#12 TransactionManager::checkpointNoLock
#13 TransactionManager::writeRequestedCheckpointIfLastNoLock
#14 TransactionManager::commit
```

Le point de reprise automatique, demandé par un COMMIT (A8), écrit une colonne de chaînes. En
recopiant une chaîne dans le dictionnaire du segment, il en hache une qui n'est plus lisible.

Les autres fils, lus avec le binaire de `9b0e73c9c` encore en place : trace complète à luciepc,
`~/.cache/rag3db-moteur-notes/core-3980144.txt` (`thread apply all bt 8`).

- Un seul fil écrit le point de reprise : le fil 1, celui qui meurt.
- 20 fils attendent à `BEGIN` (`waitWhileCheckpointPendingNoLock`), ce qui est normal : un
  point de reprise est en cours.
- Des lecteurs admis sous V2 balaient pendant ce temps : `ScanRelTable`, `PrimaryKeyScanNodeTable`,
  et le fil 3 dans `Column::scanSegment` → `BufferManager::optimisticRead` → `decompressFromPage`.
  C'est une lecture optimiste ordinaire, qui n'est pas en cause d'après sa pile.
- La chaîne illisible est lue par le point de reprise dans un bloc **en mémoire** de la colonne
  (`StringChunkData::write` à partir des données du bloc). Ce tour n'a eu aucune annulation d'un
  COMMIT refusé. Les échecs ordinaires du scénario (clé dupliquée, accès concurrent) s'annulent
  dans le stockage local de leur transaction.

## Ce qu'on ne sait pas

- **Quelle table.** Note a `texte STRING`, Doc a `titre STRING`. Le dictionnaire peut lire une
  chaîne d'une page rendue, ou d'un bloc réécrit sous lui.
- **Si c'est le SIGSEGV de la nuit sur `f41c0ba5b`** (tour 6, 02:34:46, pid 3562319). Sa pile ne
  se lit plus : le binaire a été rebâti depuis, et ses symboles ne correspondent plus.
- **Si c'est la même famille** que `2026-11-08-un-sigsegv-dans-optimisticread-sous-lecture-
  concurrente-pendant-une-indexation.md`, ou que le lecteur du 13 novembre mort dans l'index de clé
  primaire.

## Ce qu'il faut pour le fermer

1. Lire la pile entière du core 3980144 (fil, colonne, table) avec le binaire de `9b0e73c9c`,
   rebâti à l'identique si celui de l'arbre a changé.
2. Un témoin qui meure de même, puis le correctif.
3. `DdlEveryMinute` ×10 sans SIGSEGV.

## La lecture du core 3980144 (hotfixs, 14 novembre 2026)

Le core a été lu à luciepc avec le binaire de `9b0e73c9c` encore en place, en Release sans `-g`, donc
sans variables locales.

- **Le signal est `SEGV_ACCERR`, à l'adresse `0x7ef93bfff000`, un début de page.** D'après les
  segments `LOAD` du core, cette page est la seule page sans droits entre deux régions RW de 64 Mio
  (`0x7ef938000000-0x7ef93bfff000`, puis `0x7ef93c000000…`). C'est la forme d'une arène de `malloc`
  de glibc par fil, avec sa queue pas encore ouverte par `mprotect`. `_Hash_bytes` lisait donc une
  chaîne qui sort du bout d'un tampon alloué par `malloc`. Ce n'est ni une page du pool de tampons, ni
  une page relue du disque. Les octets juste avant (0x11, 0x10, 0x01 puis des zéros, par mots de 8)
  ressemblent à des longueurs ou à des en-têtes, pas à du texte.
- **La chaîne est lue dans `segmentCheckpointState.chunkData`**, la colonne en mémoire des écritures
  non reprises : `Column::checkpointSegment` (`column.cpp:418`) appelle
  `fresh->write(&segmentCheckpointState.chunkData, startRowInData, offsetInSegment, numRows)`, qui
  appelle `StringChunkData::write`, puis `srcStringChunk.getValue<std::string_view>(srcPos)`, puis
  `appendString`.
- **Deux lectures possibles :**
  - un index de dictionnaire de cette colonne qui dépasse son tampon de chaînes (`srcPos` au-delà des
    valeurs vraiment écrites, ou un tampon réduit sous l'index) ;
  - un `chunkData` libéré puis réutilisé.

## Le témoin réduit : vert sous ASAN (hotfixs, 14 novembre 2026)

La branche `chaine-au-point-de-reprise` ajoute trois cas à
`test/transaction/concurrence/string_rewrite_test.cpp`, dans la fixture `StringRewriteAcrossAddColumn` :
- `RewriteThenAddThenCheckpoint` : une réécriture des chaînes, un `ADD … DEFAULT`, une seconde
  réécriture, puis CHECKPOINT et réouverture ;
- `AnOpenWriterCommitsAfterAnAdd` : une transaction ouverte réécrit 5 000 lignes et en crée 500,
  l'`ADD` passe par une autre connexion, puis la transaction valide ;
- `AutomaticCheckpointsBetweenAddsAndDrops` : `auto_checkpoint` et `checkpoint_threshold=4096`,
  cinq lots avec des `ADD … DEFAULT` et des `DROP` entre eux, sous un lecteur qui balaie.

Ils ont été joués dans le `build/asan` du cœur (`rag3db-bissect`, RelWithDebInfo + ASAN, rebâti
incrémentalement sur la branche) : **4 sur 4 verts, aucun rapport ASAN.** Le plantage n'est donc pas
reproduit par ces formes. Il manque au témoin, par rapport au scénario, les vingt écrivains
simultanés, les MERGE par clé avec relations et vecteur, et `CREATE_VECTOR_INDEX`. Ces cas
apprennent en revanche l'isolement du catalogue, qui fait l'objet d'un ticket à part :
`docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-14-un-add-column-passe-sous-une-transaction-d-ecriture-ouverte.md`.

## Reproduit sur master le 16 novembre 2026, binaire gardé

`TwentyWriters.DdlEveryMinute`, tour 8 sur 12, sur master `2385b5f616` (le pool corrigé), sous un
binaire sonde hors dépôt : le pool du scénario réduit à 200 Mio, et un diagnostic du contrôle de
schéma qui n'a rien écrit. Core 3079782 (02:55:41), binaire à côté :
`~/.cache/rag3db-moteur-notes/stuck-probe/concurrence_test.sonde`. La même pile qu'au 14 novembre :

```
#0 std::_Hash_bytes
#1 std::_Hashtable<DictionaryChunk::DictionaryEntry, …>::find
#2 DictionaryChunk::appendString
#3 StringChunkData::finalize
#4 Column::checkpointSegment
#5 ColumnChunk::checkpoint
#6 NodeGroup::checkpointInMemAndOnDisk
…
#12 TransactionManager::checkpointNoLock
#13 TransactionManager::writeRequestedCheckpointIfLastNoLock
#14 TransactionManager::commit
```

Un point de reprise demandé par un COMMIT. 1 tour sur 12 sous 200 Mio ; 0 sur 10 sous 2 Gio avec le
pool corrigé. La pression du pool semble l'aider. La suite reste la passe sous ASan, en fenêtre.

## Le 18 novembre : le petit test de Codex, mesuré

Codex avait vu une fois le SIGSEGV de cette famille dans
`StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops` (branche
`cx-coeur-dictionary-append-atomic`, pages 15, 21, 24), avec la pile `Hashtable<DictionaryEntry>::_M_find_tr`
← `DictionaryChunk::appendString` ← `StringChunkData::write` ← `Column::checkpointSegment`. Cela
désigne une écriture de chaîne pendant le point de reprise, et non `finalize`.

Le même test joué à luciepc sur master `05c54aab8` (le correctif des offsets `9fa16affff` compris),
chaque tour dans son propre processus, journaux sous `~/.cache/rag3db-moteur-notes/c1-rate/` :

| variante | tours | verts | rouges | morts |
|---|---|---|---|---|
| tel quel (fil lecteur, ADD et DROP de colonnes, points de reprise automatiques à 4 Kio) | 100 | 100 | 0 | 0 |
| sans le fil lecteur | 100 | 100 | 0 | 0 |
| sans les DROP | 100 | 100 | 0 | 0 |
| tel quel, sous l'environnement de Codex (MAX_NUM_THREADS=2, OMP et OPENBLAS à 5, `taskset -c 0-4`) | 100 | 100 | 0 | 0 |

Rien en 400 tours : le petit test ne le reproduit pas à une fréquence utile. Le défaut reste rare
partout : une fois sur 12 sous `DdlEveryMinute` à 200 Mio, une fois chez Codex, jamais sous ASan
(7 tours) ni sous TSan (6 tours). C'est cohérent avec une course qui demande un entrelacement précis.

## Revu vivant sur master le 19 novembre (cœur c++)

`TwentyWriters.DdlEveryMinute` dix fois de suite à luciepc, sur le moteur de la pile C++ du 19 novembre (`4ab4ab8150`,
Release, offsets de `9fa16affff` compris) : **deux SIGSEGV, aux tours 3 et 8**, chaque fois dans les deux premières
minutes. La pile est celle de C1 dans les deux cores (pids 3923328 et 1542664, systemd-coredump ; copie
`~/.cache/rag3db-moteur-notes/ddl-x10/cores/core.1542664`) :

`TransactionManager::commit` → `writeRequestedCheckpointIfLastNoLock` → `Checkpointer::writeCheckpoint` →
`NodeTable::checkpoint` → `NodeGroup::checkpointInMemAndOnDisk` → `ColumnChunk::checkpoint` →
`Column::checkpointSegment` → `StringChunkData::write` → `DictionaryChunk::appendString` →
`unordered_set::_M_find_tr<string_view>` → `std::_Hash_bytes`, SEGV_ACCERR.

Ce que dit le core : la faute est le hachage de la chaîne **source** (`indexTable.find(val)`, la recherche
hétérogène ne hache que la clé). L'adresse fautive `0x7ff4fc809000` est la première page non engagée d'un tas
d'arène de malloc (`0x7ff4fc000000`, partie lisible jusqu'à `0x7ff4fc809000`, le reste réservé sans droit) : la vue
de la chaîne source déborde le tampon de son dictionnaire. Soit sa longueur ou son début sont faux (offsets
incohérents dans le dictionnaire du morceau source), soit le tampon a été libéré ou réalloué sous elle.

Le petit test de Codex, vert 400 fois, ne le voit pas ; `DdlEveryMinute` le voit deux fois sur dix. Ce qui les
sépare : des écrivains et des DDL concurrents pendant les points de reprise automatiques. Suite : `DdlEveryMinute`
sous TSan et sous ASan à luciepc.

## Sous TSan, sous ASan, puis sondé (19 et 20 novembre, cœur c++)

- **TSan**, `DdlEveryMinute` ×3 (`~/.cache/rag3db-moteur-notes/c1-ddl/tsan-*.log` à luciepc) : 18, 16 et 17 rapports,
  aucun dans `DictionaryChunk` ni `StringChunkData`. Trois familles : le compteur de relations lu par le
  planificateur, `setTimestamp` du catalogue (B1), et une course réelle des groupes de morceaux sous un lecteur
  par clé primaire, corrigée à part (lot `course-des-groupes`, ticket du lecteur qui meurt dans l'index de clé
  primaire).
- **ASan**, `DdlEveryMinute` ×3 : aucune erreur, aucun SIGSEGV. (Un premier essai TSan et ASan n'avait rien jugé :
  l'extension vector est chargée depuis `extension/vector/build/`, que chaque arbre de bâti écrase ; elle se bâtit
  maintenant dans l'arbre du sanitizer juste avant ses tours.)
- **Sonde en Release**, branche `c1-probe` (`ba5ccce329`, jamais pour master) : avant chaque copie de chaîne dans
  `StringChunkData::write`, la chaîne source doit tenir dans le tampon de son dictionnaire, sinon l'état du morceau
  source sur stderr puis abort. `DdlEveryMinute` ×10 sur master du 19 au soir : **aucun déclenchement, aucun
  SIGSEGV** (7 rouges de bornes connues : `create-rel-table-holds-writers-at-most-its-duration` ×7,
  `catalog-as-expected` ×3). Journal `~/.cache/rag3db-moteur-notes/c1-sonde/`.

À 2 sur 10 la veille, 0 sur 10 arrive environ une fois sur neuf par hasard : rien n'est conclu. La sonde reste à
rejouer (×20) sur la lib qui porte le correctif des groupes, pour départager un lien entre les deux.

## Le core avec symboles (20 novembre, cœur c++) : le tampon des offsets porte du texte

Lib épinglée `4ad394f4d2` (avec le correctif de la course des groupes), bâtie en RelWithDebInfo à luciepc
(`~/git_workspaces/coeur/20-11-2026-bati/build/rwdi`) : `DdlEveryMinute` meurt au tour 4 (pid 204417, 13 h 02 ; core
`~/.cache/rag3db-moteur-notes/c1-boucle/cores/core.204417`). C1 n'est donc pas la course des groupes.

La faute tombe dans `DictionaryChunk::appendString` (`dictionary_chunk.cpp:80`, l'`insert` de la nouvelle entrée), au
hachage de l'entrée (`StringOps::operator()`, `entry.get(*dict)`), avec une longueur de 2^64 − 100 615 : négative.
Le pointeur `dict` de `StringOps` est bien le dictionnaire courant. L'état de ce dictionnaire (de destination, celui
du point de reprise) :

- `offsetChunk` : 8 832 valeurs, capacité 16 384 (un tampon de 131 072 octets) ;
- `stringDataChunk` : 4 991 octets, capacité 131 072 ;
- `indexTable` : 0 élément ;
- **les offsets ne sont pas des offsets** : les premiers valent 29 801, 12 985, 101 704…, et les octets du tampon des
  offsets commencent par `'i' 't' 0 0 …`. Ce sont des fragments du texte des données (« …titre 2 1139titre 12185… »,
  le début du tampon de chaînes est « 83titre 2 1139titre 12185titre 12186… »).

Le tampon des offsets de ce dictionnaire est donc occupé par les octets d'un tampon de chaînes. Un tampon de 128 Kio
n'est pas une page du pool (`TEMP_PAGE_SIZE` = 256 Kio) : `MemoryManager::allocateBuffer` le prend par `malloc` à sa
taille exacte et `freeBlock` le rend par `free`. L'hypothèse : **une lecture et une écriture après libération** —
le tampon des offsets a été rendu (un `resize`, une remise à vide, la destruction d'un morceau) pendant qu'un morceau
le tient encore, et `malloc` l'a redonné au tampon de chaînes d'un autre morceau. Le premier core du 19 (arène de malloc)
va dans le même sens. ASan doit le voir dès un tour qui tombe : ses trois tours du 19 n'en ont pas eu (environ un tour
sur cinq plante). Suite : `DdlEveryMinute` ×10 sous ASan (`quarantine_size_mb=1024`), arrêt au premier rapport,
arbre `20-11-2026-bati/build/asan`.

## Treize tours ASan sans C1, et ce que dit l'adresse des tampons (20 novembre au soir, cœur c++)

**ASan.** `DdlEveryMinute` sous ASan, RelWithDebInfo, `quarantine_size_mb=1024`, sur la lib `4ad394f4d2` : dix tours à
luciepc (13 h 32 → 23 h 13, `~/.cache/rag3db-moteur-notes/c1-asan/`), plus les trois du 19 : **0 rapport, 0
plantage** en treize tours ; les rouges sont des bornes du test (`create-rel-table-holds-writers-at-most-its-duration`,
`catalog-as-expected`). Le Release, avec ou sans symboles, plante environ un tour sur cinq. Dix tours de plus tournent
à la tablette (`~/.cache/rag3db-moteur-notes/c1-asan-tablette/`).

**Le core de 13 h 02 (pid 204417) relu.** Ce n'est pas une lecture après libération, c'est un **débordement** :

- le tampon de chaînes du dictionnaire fautif est à `0x7ff684e07d80`, 131 072 octets : il finit à `0x7ff684e27d80` ;
- son tampon d'offsets est à `0x7ff684e27d90`, seize octets plus loin (l'en-tête de malloc) : le bloc voisin ;
- le texte trouvé dans les offsets (« it… », « titre 12185… ») est la suite d'une chaîne écrite au-delà de la fin du
  tampon de chaînes de **ce même dictionnaire**.

Un seul fil travaille dans ce dictionnaire (`thread apply all bt`, 36 fils) : le point de reprise
(`Checkpointer::writeCheckpoint` → `NodeGroup::checkpointInMemAndOnDisk` → `Column::checkpointSegment` →
`StringChunkData::write`, `srcOffsetInChunk = 0`, `dstOffsetInChunk = 0`, `numValuesToCopy = 8831`), au tout
premier appel (`pos = 0`). Le dictionnaire de destination n'était pourtant pas vide à ce moment : 8 831 offsets
déjà là, une table de hachage vide, 131 072 octets de capacité. Il a été rempli avant `write`, sans sa table : c'est
la forme d'un remplissage par balayage du disque, et non par `appendString`.

**La piste à lire maintenant** : `DictionaryColumn::scan` (`dictionary_column.cpp:39-70`) agrandit le tampon de
chaînes d'après `dataMetadata.numValues`, puis le remplit par `scanSegment` en relisant ce même `numValues`. Si les
métadonnées du segment lues là ne sont pas celles que le balayage écrit (un `numValues` qui change entre les deux
lectures, ou des métadonnées d'un autre segment), il écrit plus que ce qu'il a réservé. `appendString` vérifie sa
place, ce chemin-là non. À lire avec les remises à vide (`setToInMemory`, qui vide la table mais garde offsets et
octets ; `resetToEmpty`) et les échanges de morceaux de `Column::checkpointSegment`.

## La lecture du chemin (20 novembre, soir, cœur c++) : une hypothèse, pas encore prouvée

`Column::checkpointSegment` (`column.cpp:402`) crée un morceau neuf, y relit le segment persistant
(`scanSegment` → `DictionaryColumn::scan`, qui remplit offsets et octets sans bâtir la table de hachage), puis y
fusionne les écritures par `StringChunkData::write` → `appendString`. Le core montre le dictionnaire juste après
cette relecture : **8 831 offsets pour 4 991 octets**, ce qui est impossible pour des chaînes « titre N » de sept
octets au moins, même dédupliquées. Les offsets et les octets relus ne décrivent donc pas la même version du
dictionnaire : des offsets pointent au-delà des octets, `getString` lit hors du tampon, et `appendString`, qui
calcule sa place sur cette base fausse, écrit au-delà de la fin du tampon de chaînes, jusque dans le bloc voisin.

**Hypothèse** (non vérifiée dans le code, non reproduite) : les métadonnées de l'enfant OFFSET et celles de l'enfant
DATA d'un même segment persistant ne décrivent pas la même version, parce qu'un point de reprise précédent réécrit
les deux enfants séparément (flush de chacun, scission par `shouldSplit`/`split`) et peut publier l'un sans l'autre.
À tracer d'abord dans `StringChunkData::flush`, `split` et la publication des métadonnées des enfants.

**Le témoin**, en branche, rouge d'abord : une colonne de chaînes ; deux points de reprise de suite, avec des mises à
jour qui font grandir le dictionnaire jusqu'à le scinder ; puis, pour chaque segment persistant, le contrôle que le
dernier offset tient dans les octets.

**Le correctif, deux pièces** (c7, 20 novembre) :
1. la publication des deux enfants d'un segment ensemble : une version par segment, pas par enfant ;
2. à la relecture (`DictionaryColumn::scan`), une garde qui refuse en le nommant un segment dont le dernier offset
   dépasse ses octets, au lieu de déborder en silence.

## La trace de la publication (21 novembre, matin, cœur c++) : l'hypothèse s'affaiblit

- `StringChunkData::flush` écrit l'index, puis le dictionnaire (`DictionaryChunk::flush` : les octets, puis les
  offsets), sur le même fil, dans le même objet ; le segment neuf ne devient visible que par le remplacement du
  segment entier (`Column::checkpointSegment` rend la liste des segments, l'ancien est retiré). Je n'y trouve pas de
  chemin qui publie un enfant sans l'autre.
- La scission d'un segment de chaînes (`ColumnChunkData::split`, le recul pour les chaînes) passe par `append` →
  `appendString`, puis `truncate` et `finalize`, qui reconstruit le dictionnaire par `appendString` : tous vérifient
  leur place.
- **Un défaut sûr, en passant** : `DictionaryChunk::sanityCheck` (`dictionary_chunk.cpp:93-95`) rend
  `offsetChunk->getNumValues() <= offsetChunk->getNumValues()`, toujours vrai. Il devait comparer le dernier offset aux
  octets ; c'est la garde que le correctif doit rendre réelle (sa deuxième pièce).

L'incohérence relue (8 831 offsets pour 4 991 octets) reste un fait du core, mais sa naissance n'est pas trouvée. La
sonde de hotfixs (à la relecture, dans `DictionaryColumn::scan`, les métadonnées des deux enfants à l'abort) dira
lequel des deux est en retard, et de combien. C'est elle qui départagera.

