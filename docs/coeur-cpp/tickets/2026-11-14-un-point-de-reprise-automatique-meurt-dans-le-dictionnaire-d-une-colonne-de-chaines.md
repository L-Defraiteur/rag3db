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

## Reprise du 16 novembre : diagnostic des offsets, sans attribution du crash

Branche cx-coeur-c1-dictionary-diagnostic, base927941a67a ; [diagnostic et recette](../16-11-2026/02-c1-dictionnaire-diagnostic-et-temoin-des-offsets.md). Témoin seul publié2e25c850ecb627182f7ee64da13e9627e1339032 : refus48octets des offsets après copie du texte, préservation des anciennes chaînes et compteurs. NON COMPILÉ/NON JOUÉ, aucun rouge acquis ; correctif256 non repris. TSan existant tours1–4 finis0dictionnaire, tour5 encoreactif au relevé, pasFIN ; aucune relance. Fenêtre G prioritaire, qualification seulement sur feu neuf après sa fin ; aucun master autorisé.

Complément témoin seul b82c06f2572892d2124c71dce87eba287f5e9f2b : BUILD_ID émis par la cible/en-tête généré, aucune lib posée. Recette enrichie sorties/arrêt/quiescence/espace32G, plafond1800 proposé neuf, lanceur encore à publier après coordination G. ObservationTSan partielle datée/SHA conservée, aucun verdict final ni run neuf.

Première qualification C1 privée : bâti1000étapes0 puis assertionrunnerBUILD_ID erronée avant test (IDmoteur59ac distinct source/testb82), unité144,848s. **0testjoué, aucunrougedutémoin** ; preuves arret-01, poste rendu/MainPID0. Aucun header/binaire conservé avantassert,tmpfsdisparu : limite notée. Delta runner publié pour revue seulement, aucun rerun après instruction C7, aucunfix256.

## Rouge isolé du refus des offsets, 16 novembre

[Résultat et limites](../16-11-2026/03-c1-refus-offsets-rouge-et-limites.md) : sourceb82/engine59ac,1casjoué1échec0skip, réservation48octets atteinte/refusée ; anciennegamma→gammadelta et bytecount14→19,3offsetsrestants. Défaut local de garantie d’exception prouvé ; causalité SIGSEGVC1 NONétablie, aucuncorrectif256. BUILD_ID émis/ELF+archive+header sauvés et rehashés2postes. Unité155,730s, quiescenceMainPID0/cgroupvide/aucunrésidu, poste rendu ; aucunretry/fix. Ticket reste ouvert, TSan existant toujours indépendant.

Défaut isolé de garantie d’exception suivi séparément par [ticket fils](2026-11-16-un-ajout-au-dictionnaire-refuse-change-la-derniere-chaine.md), rouge physique publié avant correctif. Le présent ticket C1 reste ouvert : aucune causalité SIGSEGV déduite. Branche fillecx-coeur-dictionary-append-atomic pour témoin renforcé/hash/réessai puis correctif minimal distinct, pas256.

Ticket fils offsets : ciblés5/5 verts sur033ffb92068f31cdae6338db876d23e6bced3567 ([preuves](../16-11-2026/05-refus-offsets-cinq-cibles-vertes.md)), aucune qualification globale ni attribution du SIGSEGVC1. Le présent ticket reste ouvert. TSan existant toujours indépendant, aucun tour neuf lancé.

## Fin TSan historique : six tours, aucune relance

[Rapport final et bruts](../16-11-2026/10-c1-tsan-six-tours-termines.md) : FIN12:38:29,0processusattribuérestant,6codes66/111warnings, aucun nom DictionaryChunk/StringChunkData ni segv seloncontrôleur. Cinq toursGTest1passed, tour2GTestfailedcatalog-as-expected (doc_vecprésentaprèsCREATErefusépoolplein,compteslignescorrects), [ticketdistinct](2026-11-16-le-scenario-tsan-revoit-un-index-dont-la-creation-a-ete-refusee.md) pour départageroracle/étatNOTEDnoninterrogé. Source64eee/engine59acTSanémis, pas033/888. TicketC1resteouvert ; prochaineétapelecturecheckpoint/mémoire, pas256aveugle.

## Core du cas isolé, lecture admise le16novembre

[Analyse bornée du core existant](../16-11-2026/21-core-lu-pile-du-checkpoint-dictionnaire-conservee.md) : sourceed0/moteur888, cas AutomaticCheckpointsBetweenAddsAndDrops isolé, SIGSEGV confirmé ; pile Hashtable find → DictionaryChunk::appendString → StringChunkData::write → Column::checkpointSegment → NodeGroup::checkpointInMemAndOnDisk → Checkpointer::writeCheckpoint. Un autre thread lit DictionaryColumn/StringColumn. Pas de finalize dans les12frames acquis, pas de variables DWARF ; rapprochement de trajet avec C1, pas preuve de cause commune. Sonde chaînes non activée par le pilote50e2, aucune valeur détaillée. Aucun nouvel inférieur/test lancé par cette lecture, poste rendu5,511915s. Le parent reste ouvert/non attribué, comme le ticket propre du SIGSEGV isolé.

## Référence isolée75/59, une exécution verte

[Cas chaînes seul sans sonde](../16-11-2026/24-reference-chaines-verte-une-fois-sans-attribution-causale.md) sous profil effectif du crash ed0/888 :1/1passé,922ms, source/test/ELF/hash conservés, aucun candidat rejoué. Un vert unique de référence ne démontre pas une régression du correctif append ou une cause C1. Poste rendu225,250341s ; parent ouvert, aucune boucle/rebuild/GDB supplémentaire.
