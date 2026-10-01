# Le WAL illisible : un bug d'écriture des enregistrements de plus de 4 Kio

**1er octobre 2026.** Analyse menée en lecture seule par un agent de la
session « rag3weaver archi », relue par elle dans le code ; correctif et test
par la session Products Experiments, sur la branche
`correctif-wal-enregistrements-longs`.

**Ce qui a été fait (étape D).**
- Test `WalTest.LongRecordsReplayWithTheirValues`
  (`test/transaction/wal_test.cpp`) : des chaînes de 4 097, 8 193, 20 000 et
  100 000 caractères, seules ou à plusieurs dans une transaction, en CREATE et
  en SET ; réouverture avec `throwOnWalReplayFailure=true` ; égalité des
  valeurs relues. **Rouge avant le correctif** : « Corrupted wal file. Read out
  invalid WAL record type. » (le début perdu se lisait comme un type 0).
- Correctif : `resizeBufferIfNeeded` recopie les octets déjà accumulés dans le
  nouveau tampon, côté écrivain (`checksum_writer.cpp`) et côté lecteur
  (`checksum_reader.cpp`). Format du journal inchangé.
- `wal_record.cpp` : un type hors énumération lève « Corrupted wal file. Read
  out unknown WAL record type N. » au lieu de `KU_UNREACHABLE`.
- `transaction_test` complet : 50 tests verts.

**Ce qui ne change pas.** Un journal déjà écrit avec le bug reste
irrécupérable : son début est perdu à l'écriture. C'est le cas du `.wal` de la
base MTG du 28 septembre (30 enregistrements illisibles) : on repart de la
dernière copie ou d'une ré-ingestion.

**Décision de Lucie, 1er octobre 2026 (étape E, à venir).** Une fin
réellement déchirée (le rejeu bute sur la fin du fichier au milieu d'un
enregistrement, ou un BEGIN sans COMMIT) n'empêche plus d'ouvrir : on rejoue
jusqu'au dernier COMMIT / CHECKPOINT complet, on écarte le reste en le
copiant à côté et en le disant. Une corruption au milieu d'un journal qui
continue reste un refus d'ouvrir.

**Étape E, faite (branche `fin-de-journal-dechiree`).**
- `EndOfFileException` (sous-type de `RuntimeException`) : une lecture au-delà
  de la fin du fichier. `BufferedFileReader::read` la lève avant de copier
  quoi que ce soit (la branche par pages copiait des octets périmés au-delà
  d'une dernière page courte ; la branche directe acceptait un `pread` court).
  `pread` lui-même n'est pas changé : ses autres appelants (pages de données)
  comptent sur la lecture courte en fin de fichier.
- `dryReplay` : une fin de fichier au milieu d'un enregistrement, ou des
  enregistrements sans COMMIT terminés sur une frontière, rouvrent au dernier
  COMMIT / CHECKPOINT quel que soit `throwOnWalReplayFailure`. Toute autre
  erreur (somme fausse, type inconnu) garde son sens : refus par défaut.
- **Toute troncature** met de côté ce qu'elle retire dans
  `<journal>.ecarte-<ms>`, copié par blocs de 1 Mio, et le dit sur stderr ; en
  lecture seule, rien n'est écrit, la fin est laissée en place et signalée.

**La limite, écrite en clair.** Le format n'a pas de longueur par
enregistrement. Une longueur abîmée au milieu du journal (celle d'une chaîne,
par exemple) fait demander plus que le fichier : elle se lit comme une fin
déchirée, et la base rouvre en écartant ce qui suit — **transactions validées
comprises**. On ne peut pas le distinguer sans changer le format. Ce qui est
écarté est copié à l'octet près (le test
`CorruptedLengthReadsAsATornEndAndIsSetAsideWhole` le fixe : journal tronqué +
fichier écarté = journal d'origine), et le message de stderr ne prétend pas
que ces octets étaient non validés. La base MTG est exactement dans ce cas
possible : ses enregistrements abîmés par l'ancien bug d'écriture peuvent se
lire comme une fin de fichier. **Elle reste à ne pas ouvrir** (Lucie, 1er
octobre : « on la laisse tranquille pour le moment »).

---

## Le rapport de l'agent, tel quel

RAPPORT — WAL illisible après arrêt brutal (analyse en lecture seule, rien compilé, rien modifié)

CONCLUSION : le journal n'est ni tronqué ni déchiré. Il est corrompu À L'ÉCRITURE, bien avant le kill, par un bug de `ChecksumWriter` : tout enregistrement de plus de 4096 octets perd son début (dont l'octet de type). L'arrêt brutal ne fait que forcer le rejeu d'un journal déjà faux (un arrêt propre fait un checkpoint et supprime le .wal, database.cpp:140-146). Les quatre fichiers examinés le confirment.

ALERTE : le .wal vivant à côté de la base actuelle (`experiments/mtga/data/engine-search-lucivy43-recovered.rag3db.wal`, 3 546 580 o, 28 sept. 08:56, aucun processus rag3weaver en cours) est dans le même état : 30 enregistrements à type invalide, le premier à l'offset 2796. La prochaine ouverture échouera. Ne pas ouvrir avec `throw_on_wal_replay_failure=false` : le rejeu tronquerait le fichier (wal_replayer.cpp:144).

## 1. Mécanisme

Format et écriture
- En-tête : UUID 16 o + enableChecksums 1 o + somme 8 o (wal.cpp:71-77, wal_replayer.cpp:42-53).
- Enregistrement : type 1 o + corps + somme 8 o. Pas de longueur, pas de magic, pas de LSN (wal_record.cpp:16-19, checksum_writer.cpp:50-56).
- Chaque transaction écrit dans un `LocalWAL` en mémoire (local_wal.cpp:16-19, 104-110). Au commit : `logCommit` puis `logCommittedWAL` (transaction.cpp:68-69), qui sous mutex recopie tout dans un `BufferedFileWriter` de 4096 o, puis flush + fsync (wal.cpp:28-37, 61-64 ; buffered_file.cpp:50-62 ; local_file_system.cpp:446).
- Le fsync à chaque commit et la somme par enregistrement existent donc déjà.
- Fichier ouvert CREATE|READ|WRITE, écritures à offset explicite repris à la taille du fichier (wal.cpp:83-85, 103).

Le bug
- `resizeBufferIfNeeded` (checksum_writer.cpp:17-24) : si la taille demandée dépasse le tampon (4096 au départ, ligne 10), il alloue un tampon neuf par `allocateBuffer(false, bit_ceil(...))`, soit un malloc non initialisé (memory_manager.cpp:54-73), sans recopier les octets déjà accumulés.
- `write` (lignes 26-31) continue à `currentEntrySize` : le début de l'enregistrement devient du contenu de tas.
- `onObjectEnd` (50-56) calcule la somme sur ce tampon faux : elle est donc valide et ne détecte rien.
- Même défaut côté lecture (checksum_reader.cpp:20-36) : un enregistrement > 4096 o correctement écrit échouerait en « Checksum verification failed » (46-53).

Pourquoi la ligne 79
- `WALRecord::deserialize` lit le type et aiguille AVANT la vérification de somme, qui n'a lieu qu'à `onObjectEnd` (wal_record.cpp:25-27, 29-81, 83).
- Type hors énumération : `default: KU_UNREACHABLE` (78-80), qui lève `InternalException` (assert.h:9-15, 30-32).
- Type 0 : `RuntimeException("Corrupted wal file…")` (75-76). C'est le même bug avec un autre message.

Ce que sait faire le rejeu
- `replay` (wal_replayer.cpp:84-157) : fsync du WAL à l'ouverture (105), passe à blanc `dryReplay` qui retient l'offset du dernier COMMIT/CHECKPOINT (159-201), rejeu jusque-là (137-141), puis troncature (144, 577-585).
- Il sait donc s'arrêter à la dernière transaction complète, mais seulement si `throwOnWalReplayFailure=false` (193-199). Le défaut est true partout (database.h:69, tools/rust_api/src/database.rs:59) : toute exception de la passe à blanc fait échouer l'ouverture, y compris une vraie fin tronquée (« Reading past the end », buffered_file.cpp:101-106).
- En mode false, aucune distinction entre fin déchirée et corruption au milieu : tout ce qui suit le premier enregistrement illisible est abandonné puis tronqué, transactions validées comprises.

Défauts secondaires (sans rapport avec l'incident)
- `BufferedFileReader::read` copie `remaining` octets sans vérifier que la dernière page les contient (buffered_file.cpp:86-93).
- `pread` court en fin de fichier accepté sans erreur (local_file_system.cpp:380-382).
- `replayShadowPageRecords` ne fait pas de fsync du fichier de données avant suppression du WAL (shadow_file.cpp:132-138, wal_replayer.cpp:116) : risque coupure de courant seulement.

Checkpoint (checkpointer.cpp:147-164, 94-95) : shadow flush + fsync (shadow_file.cpp:141-161), enregistrement CHECKPOINT + fsync, application des pages, troncature puis suppression du WAL. Rien d'anormal vu ; shadow_file.cpp seulement survolé.

## 2. Pièces à conviction

Méthode : faute de longueur dans le format, j'ai cherché chaque frontière e telle que checksum(data[s:e]) == data[e:e+8]. Scripts : `<scratchpad>/walscan.py` et `walstat.py`. Les quatre fichiers se découpent intégralement jusqu'au dernier octet et finissent sur un COMMIT : aucune troncature, aucune somme fausse, aucune plage d'octets nuls sur disque.

| Fichier (sous experiments/mtga/data/) | Taille | Enreg. | ≤ 4096 o à type invalide | > 4096 o | dont type invalide | Premier illisible |
|---|---|---|---|---|---|---|
| backup-20260927-wal/….wal (identique à ….wal.illisible à la racine) | 13 456 | 109 | 0 | 1 | 1 | offset 5708, len 4121, type 0x7f → ligne 79 |
| wal-illisible-20260927-1415/….wal | 16 777 437 | 73 128 | 0 sur 72 827 | 301 | 45 (+1 lu « 16 » par hasard) | offset 550, len 20030, type 136 → ligne 79 |
| wal-illisible-20260927-1539/….wal | 15 439 | 173 | 0 | 1 | 1 | offset 7594, len 4471, type 0 → ligne 76 |
| ….rag3db.wal (vivant) | 3 546 580 | 6 428 | 0 sur 6 380 | 48 | 30 | offset 2796, len 4578, type 0 → ligne 76 |

- Fichier 1, offset 5708 : 49 octets de déchets (motif de structures de tas `7f0f0000 1d0b0000 fb00…`), puis la chaîne JSON intacte à +49, exactement là où elle commence dans un NODE_UPDATE sain (offset 5011). 49 + 4072 = 4121 : l'écriture de la chaîne a franchi 4096 et les 49 octets déjà tamponnés (type, table, colonne, offset, type de donnée, longueur) ont été perdus.
- Les enregistrements > 4096 o lus correctement (type 32) suivent un enregistrement plus gros de la même transaction : le tampon était déjà agrandi (ex. fichier 1415, 1149492 juste après 924958 de 224 526 o).
- Fichier 1415 : taille = seuil de 16 Mio (database.h:68) + 221, avec un .shadow de 0 o : le processus a été tué au début d'un auto-checkpoint. Sans incidence sur la cause.
- Il y a donc eu trois épisodes le 27 (13:28, 14:15, 14:42), le troisième avec le message de la ligne 76.

## 3. L'amont

- `master..vela/storage/concurrent-checkpoint-recovery` : 46 commits, 134 fichiers, +8056/−1005. `vela/master` : 109 fichiers, +3545/−561. Base commune 89f0263cc (2025-10-10).
- Aucune des deux branches ne touche `checksum_writer.cpp`, `checksum_reader.cpp` ni `buffered_file.cpp` (0 ligne hors renommage kuzu→rag3db). Le bug y est intact. Il vient de 5b78870ea « Checksum WAL records (#5940) » et figure encore au tag local `ladybug-main-2026-08-31`.
- La branche de récupération traite autre chose : rotation du WAL pendant le checkpoint, enregistrements CHECKPOINT_BEGIN / V2 avec identité (dfee83bf3), fsync des entrées de répertoire (645d68973), publication des commits après sync (a96f7a7d9), reprise de checkpoints partiels (b73e093c0, 373f40945), tests de frontières de crash (4945229bb, 27 sept.).
- Son `dryReplay` garde le même `catch(...)` et devient plus strict (wal_replayer.cpp:420-431 de la branche ; tests renommés « FailsClosed »). Le `default: KU_UNREACHABLE` y est toujours (wal_record.cpp:87-89).
- Réponse : non, l'amont n'a corrigé ni ce bug ni la tolérance à une fin déchirée. Ces commits ne se reportent pas isolément : ils dépendent de la chaîne checkpoint concurrent (#1-#4, #10-#12) et chaque fichier porte le bruit du renommage.
- Seul commit apparenté : 21b4ad5cd « Store WAL record lengths » (Ladybug, 2026-06-05, atteignable par tags seulement). Il ajoute une longueur par enregistrement, mais touche 23 fichiers dans une arborescence restructurée et ne corrige pas le redimensionnement.

## 4. Pistes, de la moins à la plus invasive

A. Recopier l'existant dans `resizeBufferIfNeeded`, dans l'écrivain ET le lecteur (environ 6 lignes, format inchangé).
   - Garantit : enregistrements > 4096 o écrits et relus correctement, donc WAL rejouable après kill.
   - Ne garantit pas : les WAL déjà écrits restent irrécupérables (le début est perdu à l'écriture, il faut réingérer depuis le dernier checkpoint) ; une vraie fin tronquée échoue toujours par défaut.
B. Remplacer `default: KU_UNREACHABLE` par une exception « WAL corrompu » (wal_record.cpp:78-80).
   - Garantit un diagnostic clair. Ne répare rien seul.
C. Tolérance à la fin déchirée par défaut : dans `dryReplay`, s'arrêter au dernier COMMIT et tronquer si l'échec est dans la dernière transaction non validée, refuser d'ouvrir sinon. Corriger au passage les deux lectures courtes du point 1.
   - Sans longueur d'enregistrement, on ne peut pas sauter un enregistrement illisible pour voir ce qui suit ; l'heuristique fiable est « échec par fin de fichier ».
   - C'est un changement de sémantique de récupération : à décider par toi.
D. Longueur par enregistrement et somme vérifiée avant d'interpréter le type (esprit de 21b4ad5cd).
   - Rend C rigoureux. Change le format du WAL, donc compatibilité à gérer.
E. fsync à la validation et somme par enregistrement : déjà en place, rien à gagner. La somme est aveugle à ce bug.
F. Reporter l'amont vela : ne traite pas le problème.

Recommandation : A + B tout de suite, avec le test du point 5. C'est la cause unique des trois épisodes et c'est le plus petit correctif. C ensuite comme décision séparée : même sans ce bug, un kill pendant `logCommittedWAL` laisse une fin partielle, et le défaut refuse alors d'ouvrir.

Tests existants : `test/transaction/wal_test.cpp` (588 lignes, cible `transaction_test`, test/transaction/CMakeLists.txt:5). Fin tronquée de 10 o : lignes 259-335, 436, 488, toutes avec `throwOnWalReplayFailure=false` (fixture, ligne 18). Sommes fausses : 150-207. `test/transaction/checkpoint_test.cpp` couvre le checkpoint. Aucun test ne rejoue un enregistrement > 4096 o.

## 5. Reproduire sans tuer de processus

L'infrastructure le permet déjà : fixture `WalTest` (ApiTest), `CALL force_checkpoint_on_close=false`, `createDBAndConn()` pour rouvrir, `std::filesystem::resize_file` pour tronquer. Deux tests à poser dans `test/transaction/wal_test.cpp` :

1. Cause racine, sans troncature :
   - créer une table avec une colonne STRING, faire un CREATE puis un SET avec des chaînes de 4097, 8193 et ~100 000 caractères, dont plusieurs gros enregistrements dans une même transaction ;
   - rouvrir avec `throwOnWalReplayFailure=true` et relire les valeurs ;
   - affirmer l'égalité des valeurs, pas un message : le début perdu est du tas non initialisé, le message varie (ligne 79, ligne 76 ou somme fausse). Doit échouer aujourd'hui.
2. Fin déchirée :
   - écrire quelques transactions, puis pour chaque longueur L (au milieu d'un corps, juste après un octet de type, au milieu d'une somme, entre BEGIN et COMMIT) copier le .wal tronqué à L et rouvrir ;
   - attendre l'état de la dernière transaction complète. Avec le défaut true, échoue aujourd'hui ; c'est le test de la piste C.

## Non vérifié

- Rien n'a été compilé ni exécuté : le message exact par fichier est déduit du code. Aucune base n'a été ouverte, y compris la base vivante.
- État de l'amont au-delà des refs locales (pas de fetch) : Kuzu ou Ladybug ont pu corriger depuis.
- shadow_file.cpp survolé seulement. Le dossier `….checkpoints/drain-…` (14:15:14) n'a pas d'origine dans src/ ; non examiné.
- Aucune fin réellement déchirée n'a été observée : ce mode de panne est déduit du code, pas des fichiers.