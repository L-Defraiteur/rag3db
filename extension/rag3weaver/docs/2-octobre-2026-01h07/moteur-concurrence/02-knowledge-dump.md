# Knowledge dump — le cœur C++ : transactions, journal, point de reprise, lecteurs et écrivains

Session « cœur C++ », arrêt du 2 octobre 2026. Ce document dit ce que je sais
du moteur, orienté sur ma partie. Il ne remplace pas le rapport de fond
`docs/2-octobre-2026-00h17/01-ecritures-paralleles-vela-et-le-chemin.md`
(663 lignes, 13 sections) : il le complète et y renvoie.

Trois étiquettes, partout :

- **[exécuté]** — prouvé par un test ou une mesure que j'ai fait tourner ;
- **[lu]** — lu dans le code, à la ligne citée, sans l'avoir fait tourner ;
- **[déduit]** — raisonnement à partir de ce qui est lu ; à traiter comme une
  hypothèse.

Les numéros de ligne valent pour la branche
`reprise-apres-panne-index-cle-primaire` (`origin/master` `df1d2c6d5` + le
correctif `df4838542`). Ce qui n'a ni ligne ni étiquette [exécuté] vient de
lectures plus anciennes de la session : à revérifier avant de s'appuyer dessus.

---

## 1. Les arbres, les branches, les remotes

| Quoi | Où |
|---|---|
| Mon arbre de travail | `/home/lucied/git_workspaces/rag3db-moteur` (worktree), dossier de build `build/moteur` |
| L'arbre principal | `/home/lucied/git_workspaces/rag3db` — **à une autre session**, je n'y écris rien |
| Le banc de concurrence | `/home/lucied/git_workspaces/rag3db-banc`, branche `banc-de-concurrence` (`37a44e351`), à la session « banc » |
| Remote `origin` | `git@github.com:L-Defraiteur/rag3db.git` |
| Remote `vela` | `https://github.com/Vela-Engineering/kuzu.git` |

Tags posés pour figer ce qui a été étudié :

- `vela-master-2026-09-03` (`2efa20b67`) — leur branche principale ;
- `vela-checkpoint-2026-09-03` (`4945229bb`) — leur branche
  `storage/concurrent-checkpoint-recovery` ;
- `ladybug-main-2026-08-31` — LadybugDB.

Notre fork part de Kuzu `89f0263cc` ; le renommage global `kuzu` → `rag3db`
est le commit `c647fbb33`. Ce renommage est ce qui rend toute fusion d'un
autre fork coûteuse : chaque fichier diffère par ses identifiants avant même
de différer par son contenu (rapport, §4).

Mes branches, toutes poussées sur origin :

| Branche | Contenu | État |
|---|---|---|
| `reprise-apres-panne-index-cle-primaire` | `df4838542`, le correctif du tableau sur disque et son test ; puis `a486fde9c`, la panne *pendant* la phase de stockage (test de balayage, `FlakyBufferManager` sorti dans un en-tête partagé) | le premier commit est accepté par l'orchestrateur et en cours de livraison sur master par la session « produit » ; le second attend sa relecture |
| `lecteur-reverifie-a-l-ouverture` | la marche 1 : `b254776d2` (crochet de test et tests rouges), `6bc8ce632` (la revérification, **commit « en cours »**) | à finir, voir le rapport de session |

---

## 2. Construire et tester

**Configuration [exécuté].** `build/moteur` : Ninja, `CMAKE_BUILD_TYPE=Release`,
`BUILD_TESTS=TRUE`, `BUILD_EXTENSIONS="vector;geo"`.

**Jamais plus de huit tâches** : `ninja -C build/moteur -j8 <cibles>`. Deux
autres sessions compilent sur le même poste ; `-j$(nproc)` le fige.

**Cibles et durées [exécuté], Release, 2 oct. 2026 :**

| Cible | Binaire | Tests | Durée |
|---|---|---|---|
| `transaction_test` | `build/moteur/test/transaction/transaction_test` | 56 | 27 s |
| `api_test` | `build/moteur/test/api/api_test` | 102 | 21 s |
| `copy_tests` | `build/moteur/test/copy/copy_tests` | 19 | 31 s |
| stockage : `buffer_manager_test`, `column_chunk_metadata_test`, `compression_test`, `local_hash_index_test`, `node_insertion_deletion_test`, `node_update_test`, `rel_tests`, `string_finalize_test` | `build/moteur/test/storage/…` | 77 | < 2 s |
| `e2e_test` | `build/moteur/test/runner/e2e_test` | — | `E2E_TEST_FILES_DIRECTORY=<dossier> ./…/e2e_test --gtest_filter='*'` |

Tous se lancent depuis la racine du worktree. Un changement dans `src/`
recompile la bibliothèque puis relie chaque binaire : compter deux à trois
minutes à `-j8`.

**Rouge connu de master [exécuté] :**
`LecteursConcurrents.CeQueLeLecteurVoitEstCoherent` (dans `api_test`), rouge
cinq fois sur cinq sur master. C'est la course du lecteur d'un autre processus
(§8) ; la marche 1 le rend vert.

**Les doublures de test.**

- `testing::FlakyCheckpointer` (`test/transaction/checkpoint_test.cpp:18` sur
  master ; sorti dans `test/transaction/flaky_checkpointer.h` sur la branche de
  la marche 1). Il installe une fabrique de `Checkpointer` dans
  `TransactionManager::initCheckpointerFunc`. Ce champ est **privé** : seule
  cette classe est amie (`transaction_manager.h:26-28`), donc toute doublure
  passe par elle. On dérive `Checkpointer` et on surcharge une phase
  (`checkpointStorage`, `serializeCatalogAndMetadata`, `writeDatabaseHeader`,
  `logCheckpointAndApplyShadowPages`) : la doublure ne sait s'arrêter
  **qu'entre deux phases**.
- `testing::FlakyBufferManager` (`test/copy/copy_test.cpp:23` sur master ;
  `test/include/test_helper/flaky_buffer_manager.h` à partir de `a486fde9c`). Il refuse une réservation de mémoire sur `failureFrequency`, dans
  les phases où on l'y autorise ; après un refus il double la fréquence. C'est
  la seule doublure qui échoue **au milieu** d'une phase. Pour viser la k-ième
  allocation d'une requête : `reserveCount = 0; failureFrequency = k;` juste
  avant.
- Le crochet du lecteur, `WALReplayer::setReadOnlyOpenHookForTesting`
  (marche 1) : une fonction appelée à deux moments d'une ouverture en lecture
  seule, `JOURNAL_SCANNED` et `DATA_FILE_READ`. Il permet de faire écrire un
  écrivain **du même processus** exactement entre deux lectures du lecteur, et
  rend la course déterministe.

**Montage des tests privés.** `PrivateApiTest` (`test/include/api_test/`)
ouvre une base et charge un jeu de données ; `getInputDir()` rendant `"empty"`
donne une base vide. `createDBAndConn()` rouvre la même base : c'est ainsi
qu'on simule une panne (avec `CALL force_checkpoint_on_close=false;`, sans quoi
la fermeture fait un point de reprise et efface la panne). `CALL
auto_checkpoint=false` pour décider soi-même des points de reprise.

---

## 3. La couche transactionnelle et le MVCC

**Un seul écrivain par défaut [lu].** `TransactionManager::beginTransaction`
refuse une deuxième transaction d'écriture si `enableMultiWrites` est faux
(`src/transaction/transaction_manager.cpp:35`, message « Cannot start a new
write transaction in the system »). Le réglage s'appelle
`debug_enable_multi_writes` (`src/include/main/settings.h:100`) : le nom dit ce
que l'amont en pense.

**Deux verrous dans le gestionnaire [lu]** : `mtxForSerializingPublicFunctionCalls`
(toutes les fonctions publiques) et `mtxForStartingNewTransactions` (pris par
le point de reprise pour empêcher toute entrée,
`transaction_manager.cpp:119-120`).

**Le commit [lu]** : `Transaction::commit` fait `localStorage->commit()` puis
`undoBuffer->commit(commitTS)` (`src/transaction/transaction.cpp:63-65`). Les
insertions d'une transaction vivent dans un stockage local jusqu'au commit ;
les mises à jour et suppressions marquent les versions en place et s'annulent
par le tampon d'annulation.

**Les conflits détectés aujourd'hui [lu]** — trois messages, et seulement
ceux-là :

- `Write-write conflict of updating the same row`
  (`src/storage/table/update_info.cpp:37`) : deux mises à jour de la même
  ligne **dans la même colonne**. L'information de mise à jour est tenue par
  colonne et par vecteur ; ce que devient la même ligne modifiée dans deux
  colonnes différentes par deux transactions est une question ouverte de la
  note sur les verrous — **non vérifié**.
- `Write-write conflict: deleting a row that is already deleted by another
  transaction` (`src/storage/table/version_info.cpp:106` et `:117`).
- `Write-write conflict on creating catalog entry` (`src/catalog/catalog_set.cpp:91`).

**Ce qui n'est pas détecté sous plusieurs écrivains** — c'est tout le sujet de
la cible A (rapport, §7) :

- deux transactions insèrent la même clé primaire : chacune vérifie contre ce
  qu'elle voit, aucune validation au commit → doublon silencieux ;
- une relation créée vers un nœud qu'une autre transaction supprime → relation
  pendante ;
- les offsets locaux des nœuds créés ne sont pas remappés au commit quand une
  autre transaction a validé entre-temps → relations vers de mauvais nœuds
  (Vela a le remappage, c'est la marche A2).

Le banc de la session « banc » (`37a44e351`) dit avoir rendu ces trois
corruptions réelles (C1 à C3, liste `known_red.txt`). **Je ne l'ai pas encore
relu** : c'est le point 3 de la reprise.

---

## 4. Le journal (WAL) et le rejeu

**Écriture [lu].** Une transaction accumule ses enregistrements dans un
journal local, versé d'un bloc au commit (`WAL::logCommittedWAL`,
`src/storage/wal/wal.cpp:28`). Le point de reprise écrit un enregistrement
CHECKPOINT (`WAL::logAndFlushCheckpoint`, `:39`), puis le journal est vidé
(`WAL::clear`, `:48`) et réinitialisé (`WAL::reset`, `:53`).

**Rejeu à l'ouverture [lu + exécuté].** `WALReplayer::replay`
(`src/storage/wal/wal_replayer.cpp`) :

1. pas de journal → on lit le point de reprise, c'est tout ;
2. journal vide → idem ;
3. sinon `dryReplay` (`:169`) parcourt le journal sans rien appliquer et rend
   la position du dernier enregistrement valide, `isLastRecordCheckpoint`, et
   `tornEnd` (fin déchirée, travail de l'étape E d'une autre session) ;
4. si le dernier enregistrement est CHECKPOINT : le point de reprise était
   journalisé mais ses pages pas toutes appliquées → on rejoue **le fichier
   fantôme** ; en lecture seule c'est refusé (« Couldn't replay shadow pages
   under read-only mode ») ;
5. sinon : `readCheckpoint()` charge catalogue et métadonnées depuis le fichier
   de données, puis on rejoue le journal jusqu'au dernier COMMIT, puis on
   tronque.

Ce sont **trois lectures à trois instants différents** (parcours du journal,
lecture du fichier de données, rejeu du journal). Pour un écrivain seul, sans
importance. Pour un lecteur d'un autre processus, c'est la course du §8.

---

## 5. Le point de reprise et le fichier fantôme

**La séquence [lu]**, `Checkpointer::writeCheckpoint`
(`src/storage/checkpointer.cpp:64`) :

1. `checkpointStorage()` — chaque table écrit ses colonnes puis ses index
   (`NodeTable::checkpoint`, `src/storage/table/node_table.cpp:676` : les
   groupes de nœuds d'abord, **les index en dernier**) ;
2. `serializeCatalogAndMetadata` — catalogue, métadonnées, gestionnaire
   d'espace libre, tout par le fichier fantôme ;
3. `writeDatabaseHeader` — la page 0, par le fichier fantôme ;
4. `logCheckpointAndApplyShadowPages` (`:150`) : vider le fantôme sur disque,
   **journaliser CHECKPOINT** (le point de non-retour), recopier les pages
   fantômes dans le fichier de données, vider journal et fantôme ;
5. `finalizeCheckpoint` (`:83`) : c'est seulement ici que les pages libérées
   deviennent réutilisables.

**Le principe [lu].** Avant l'enregistrement CHECKPOINT, le fichier de données
doit rester celui du point de reprise précédent : une panne à ce stade se
répare en rejouant le journal par-dessus. Donc toute page **référencée par
l'état sur disque** se modifie dans le fichier fantôme ; seules les pages
**neuves**, que rien sur disque ne référence, peuvent s'écrire directement.

**Qui écrit comment** — relevé par un agent, lecture seule ; les lignes ✔ sont
celles que j'ai revérifiées moi-même :

| Site | Chemin | Critère |
|---|---|---|
| colonnes en place, nulls, exceptions ALP (`column_reader_writer.cpp:438`) | fantôme | inconditionnel |
| chunks de colonnes neufs (`compression_flush_buffer.cpp`) | direct | page allouée juste avant (`column_chunk_data.cpp:316`, `column.cpp:185`) |
| débordement des chaînes de l'index ✔ (`overflow_file.cpp:221-231`) | direct si `newPage`, sinon fantôme | `newPage` posé juste après `allocatePage` |
| en-têtes de l'index (`hash_index.cpp:639`), des tableaux (`disk_array_collection.cpp:55`), catalogue, métadonnées, page 0 | fantôme | inconditionnel |
| pages des tableaux de l'index ✔ (`disk_array.cpp`) | fantôme si la page existait au dernier point de reprise, direct sinon | **c'était le défaut**, §6 |

**Seules les transactions de type CHECKPOINT lisent le fantôme [lu]**
(`ShadowUtils::getFileHandleAndPhysicalPageIdxToPin`,
`src/storage/shadow_utils.cpp:42-46`). Conséquence, **[exécuté]** : après un
point de reprise **échoué**, tout ce qu'il a mis dans le fantôme est invisible
aux transactions suivantes, alors que les structures en mémoire ont déjà
avancé. Voir le piège n° 1.

**Le retour arrière d'un point de reprise échoué [lu]** est mince :
`TransactionManager::checkpointNoLock` attrape l'exception, appelle
`checkpointer->rollback()` et lève `CheckpointException`
(`transaction_manager.cpp:179-182`) ; `Checkpointer::rollback`
(`checkpointer.cpp:166-174`) ne fait que `StorageManager::rollbackCheckpoint`
(`storage_manager.cpp:199-207`), qui ne restaure **que les index des tables de
nœuds** et vide la liste des pages libérées en attente. Le fantôme n'est pas
vidé. `ClientContext` se contente de `clearTransaction()` et relance
(`src/main/client_context.cpp:588`).

---

## 6. `DiskArray` et l'index de hachage

**La structure [lu].** L'index de clé primaire, `PrimaryKeyIndex`, est fait de
`NUM_HASH_INDEXES` index de hachage ; chacun a deux tableaux sur disque,
`pSlots` (cases primaires) et `oSlots` (cases de débordement)
(`src/storage/index/hash_index.cpp:40-41`), plus un fichier de débordement pour
les chaînes. Un tableau sur disque (`DiskArrayInternal`) est une suite de pages
de données (AP) retrouvées par des pages d'index de pages (PIP). Il a **deux
en-têtes** : celui de lecture (l'état du dernier point de reprise) et celui
d'écriture (l'état en cours).

**La vie d'une clé [lu].** Au commit, les clés insérées vont dans un stockage
local **en mémoire** de l'index. C'est le point de reprise qui les fond dans la
structure sur disque : `HashIndex::checkpoint` (`hash_index.cpp:75`) →
`mergeBulkInserts` (`:266`). Puis `PrimaryKeyIndex::checkpoint` écrit les
en-têtes, le débordement, **vide les pages sales sur disque**
(`flushAllDirtyPagesInFrames`, `:694`) et appelle `checkpointInMemory()`
(`:695`) — pendant la phase de stockage, donc avant qu'on sache si le point de
reprise réussira.

**`bypassShadowing` [lu].** Les tableaux de l'index sont créés avec
`bypassShadowing = true` (`hash_index.cpp:487`, `:512`) : leurs pages neuves
s'écrivent directement, par le gestionnaire de tampons, pour ne pas doubler
chaque écriture d'un gros chargement.

**Le défaut corrigé [exécuté]** — commit `df4838542`. Pour savoir si une page
du tableau existait au dernier point de reprise, l'ancien code comparait le
numéro de la page **dans le fichier** à un seuil en cache, `lastPageOnDisk`.
Faux deux fois :

- le seuil était recalculé (`HashIndex::checkpointInMemory`, `:104`) **avant**
  que `PrimaryKeyIndex::checkpointInMemory` ne recopie les en-têtes d'écriture
  dans ceux de lecture (`:621-623`) : il décrivait donc l'avant-dernier point
  de reprise, et restait à 0 pour un tableau né vide. Mesuré par trace : 0 du
  début à la fin, 261 écritures directes sur 438 sur des pages existantes ;
- il supposait les pages du tableau rangées par ordre croissant dans le
  fichier, ce qui est faux dès que le gestionnaire d'espace libre réutilise
  des pages.

Le critère est maintenant l'**indice dans le tableau** contre l'en-tête de
lecture :

```cpp
bool existedAtLastCheckpoint(common::page_idx_t apIdx) const {
    return !bypassShadowing || apIdx < getNumAPs(header);
}
```

Symptôme avant correctif : une table qui a déjà connu des points de reprise,
un point de reprise interrompu pendant ou après sa phase de stockage → la base
ne se rouvre plus (« Found duplicated primary key value 278 »). Les tests
amont n'interrompaient que le **premier** point de reprise d'une table neuve,
où le défaut ne se voit pas. Le fichier était identique à l'amont : **Kuzu,
Vela et Ladybug portent ce défaut** (fichiers comparés dans les arbres
extraits le 2 octobre). Le message du commit est écrit pour être envoyé tel
quel à l'amont.

**Une piste fausse à ne pas reprendre [exécuté]** : garder le seuil par numéro
de page de fichier en prenant le maximum sur toutes les pages du tableau
laisse le test rouge.

---

## 7. Le gestionnaire d'espace libre

**La garantie dont dépend toute écriture directe [lu, revérifié]** : une page
libérée pendant un point de reprise ne peut pas être réallouée pendant ce même
point de reprise.

- `freePageRange` range la page dans `uncheckpointedFreePageRanges`
  (`src/storage/free_space_manager.cpp:47`) ;
- `popFreePages` ne puise que dans `freeLists` (`:56`) ;
- le transfert se fait dans `finalizeCheckpoint` (`:194`), appelé **après**
  l'application des pages fantômes (`checkpointer.cpp:77` puis `:83`).

Seule exception : `freeImmediatelyRewritablePageRange`
(`src/storage/page_manager.cpp:38`), appelé seulement par le retour arrière de
l'allocateur optimiste, sur des pages que l'état sur disque ne référence pas.

**Fuite connue [lu]** : après un point de reprise échoué, les pages qu'il avait
allouées et écrites directement ne sont rendues à personne en mémoire
(`rollbackCheckpoint`, `:51`, vide seulement la liste en attente). Elles
redeviennent libres au redémarrage. L'amont le sait : `TODO(Royi) prevent
leaking of allocated pages during checkpoint rollback`
(`test/copy/copy_test.cpp`).

---

## 8. Le verrou de fichier et le lecteur d'un autre processus

**Le verrou [lu].** À l'ouverture, le fichier de données est verrouillé par
`fcntl(F_SETLK)` (`src/common/file_system/local_file_system.cpp:135-142`) :
verrou d'écriture pour un écrivain, de lecture pour un lecteur
(`src/storage/file_handle.cpp:36` et `:41`), sauf si `isLockRequired()` est
faux. Depuis le port de trois lignes repris de Vela (`4b6f9baa2`), **une
ouverture en lecture seule ne prend aucun verrou** : un lecteur d'un autre
processus peut ouvrir pendant qu'un écrivain écrit.

**Ce que fait ce lecteur [lu + exécuté].** Il rejoue **en mémoire** le journal
de l'écrivain vivant, par les trois lectures du §4. Entre deux points de
reprise, l'écrivain n'ajoute qu'à la fin du journal et n'écrit que des pages
que le dernier point de reprise ne référence pas : les trois lectures
s'accordent. Un point de reprise, lui, réécrit la page 0 et des pages
référencées, puis tronque le journal.

**La course [exécuté]**, rendue déterministe par le crochet : un point de
reprise qui passe entre le parcours du journal et la lecture du fichier de
données donne, selon l'instant, « Reading past the end of the file », « Found
duplicated primary key », « Cannot open file »… ou **aucune erreur et un
contenu faux** : un lecteur a vu 18 relations là où il y en a 9
(`ReadOnlyOpenTest.RelationshipsAreNotDoubledByAHalfDoneCheckpoint`).

**La revérification (marche 1) [exécuté].** Le lecteur retient au début la
page 0 brute et la taille + somme de contrôle du journal ; il revérifie après
la lecture du fichier de données et après le rejeu, et aussi **avant de
relancer toute erreur** survenue pendant l'ouverture. Si un point de reprise
est passé, il refuse par une erreur nommée et stable :

> `The database was checkpointed by another process while this read-only open
> was reading it` (constante `CHECKPOINT_CROSSED_READ_ONLY_OPEN`), suivie de
> « … Retry the open. »

Un journal qui a seulement **grandi** de nouveaux commits n'est pas un refus.

**Ce que la revérification ne couvre pas, par construction** — il faut une
époque écrite dans le fichier, c'est la **marche 5** :

- un lecteur qui **reste ouvert** pendant qu'un point de reprise passe : il lit
  ses pages à la demande avec les métadonnées chargées à l'ouverture ;
- deux points de reprise complets dans une seule ouverture qui laissent une
  page 0 identique et un journal vide des deux côtés.

L'en-tête de base n'a aujourd'hui aucune époque : `catalogPageRange`,
`metadataPageRange`, `databaseID` (`src/include/storage/database_header.h:13-18`).

**Rectification [exécuté]** : ma mesure du 18 septembre (pic de 567 ms, « 2/80 »
côté Rust) attribuait les refus à une famine d'ordonnancement. Le test de ce
jour-là comptait les refus **sans lire leur message** ; c'était très
probablement cette course. Le titre du commit `20a8f6ee8` n'est plus soutenu.

**Question ouverte [exécuté, une fois]** : sous des points de reprise très
fréquents, une passe du test de charge a compté **zéro** ouverture réussie —
le lecteur refuse honnêtement, mais peut ne jamais entrer. À mesurer après la
marche 1.

---

## 9. Le mode multi-écrivains et les deux cibles

Tout est au rapport de fond ; l'essentiel :

- **Cible A** — plusieurs transactions d'écriture en parallèle dans un
  processus. Marches A1 à A8 (rapport, §7) : A1 le banc (fait par la session
  « banc »), A2 remappage des offsets locaux au commit (repris de Vela), A3
  unicité de clé primaire, A4 intégrité des relations, A5 sûreté de la
  suppression entre fils, A6 allumer le mode, A7 point de reprise qui attend
  les transactions, A8 file d'attente optionnelle.
- **Cible B** — plusieurs processus écrivains. Forme retenue : journal partagé
  (rapport, §8). Ce que A construit est réutilisé par B, sauf A7.
- **Décisions de Lucie (2 oct.)** : « on fait ce qu'il faut pour écritures
  parallèles, peu importe ce que ça coûte » ; « oui jusque B, et A d'abord si
  dans même chemin » ; piste de plusieurs fichiers par genre d'index (rapport,
  §9) ; **le second écrivain attend au lieu d'échouer, sur un verrou par clé** —
  ce qui change A3 et A4 de « valider au commit, le second échoue » en
  « verrouiller par clé, le second attend ». D'où la note de conception sur
  les verrous, due avant tout code sur A3/A4.
- **Principe de Lucie pour toute conception du moteur** : faire comme les
  moteurs établis (PostgreSQL, Neo4j), sauf quand on trouve mieux ; vérifier
  leur comportement **dans leur documentation**, pas de mémoire ; ne lui rendre
  en options que les écarts.

---

## 10. L'index vectoriel (HNSW) sous plusieurs écrivains

Rapport d'un agent, lecture seule, **non revérifié par moi** :

- l'insertion se fait **au commit** (`needCommitInsert() = true`), sous le
  verrou de commit, avec les offsets définitifs — c'est l'amont, et c'est sain ;
- nos ajouts (suppression et mise à jour) travaillent **pendant** la
  transaction : ils modifient les points d'entrée dans `HNSWStorageInfo` sans
  verrou, et ne sont pas annulés par un rollback ;
- `shrinkForNode` peut lever « Write-write conflict » au milieu d'un commit ;
- aucun test de concurrence sur l'extension.

La section à écrire dessus reste due (dernier point de la reprise).

---

## 11. Vela et Ladybug : ce qu'ils valent pour nous

- **Vela** (`Vela-Engineering/kuzu`). Leur « concurrent writes » : le mode
  multi-écrivains de Kuzu rendu utilisable — remappage des offsets au commit,
  point de reprise en fil de fond qui attend les transactions, reprise après
  panne d'un point de reprise concurrent sur leur branche. Rapport, §1 à §5 :
  ce qu'on reprend (A2, A7, les trois lignes du lecteur sans verrou, déjà
  faites), ce qu'on ne reprend pas, et le coût de fusion — on porte à la main,
  on ne fusionne pas.
- **Ladybug** : sonde de faisabilité du 3 septembre
  (`docs/3-septembre-2026-14h42/05`, `06`, `07`).
- Les trois arbres portent le défaut du §6.

Les arbres extraits pour lecture étaient dans mon scratchpad (`etude-vela/`),
donc dans `/tmp` : ils disparaissent au redémarrage. Ils se régénèrent par
`git archive <tag> | tar -x -C <dossier>` à partir des tags du §1.

---

## 12. Autres morceaux du cœur que j'ai touchés

- **Lecteur CSV** : option `ESCAPED_NEWLINES` (`d2b48ea68`).
- **Dépôt d'extensions** : refus de `RAG3DB_EXTENSION_REPO` (`72df62584`).
- **Tests des lecteurs concurrents** : `test/api/lecteurs_concurrents_test.cpp`
  (`945cd6706`), trois tests, le troisième mesure les fenêtres de refus.

---

## 13. Les pièges

1. **Après un point de reprise échoué, le processus ne doit pas continuer
   [exécuté].** Sur master, avec ou sans mon correctif : mêmes douze cycles,
   panne après la phase de stockage, sans rouvrir → `COUNT(a)` = 300 juste,
   mais `COUNT(DISTINCT a.id)` = 275 et `SUM(a.id)` = 37 675 au lieu de
   44 850 (les 25 dernières lignes ont perdu leur clé, en silence), puis
   SIGSEGV dans `StringColumn::scanUnfiltered`. La reprise par réouverture,
   elle, est juste. rag3weaver subit les points de reprise automatiques au
   commit et continue sur la même `Database` : **il est exposé**. Le test
   d'expérience est rangé à côté de ce document :
   `experience-meme-processus-apres-point-de-reprise-echoue.patch`.
2. **Un test de panne qui ne désactive pas `force_checkpoint_on_close`** ne
   teste rien : la fermeture refait le point de reprise.
3. **Un point de reprise complet dans le crochet du lecteur tronque le
   journal** : le test devient rouge pour une autre raison (« Reading past the
   end of the file »). Pour obtenir la clé en double ou les relations doublées,
   il faut un point de reprise **à moitié fait** (doublure de `Checkpointer`).
4. **Une table écrit son index en dernier** : pour qu'une panne *pendant* la
   phase de stockage trouve des pages d'index déjà sur disque, il faut **deux
   tables**. Le test de balayage de `a486fde9c` le fait : 180 points de
   reprise interrompus, 174 pendant la phase de stockage, rouge sans le
   correctif, vert avec.
5. **`initCheckpointerFunc` est privé** : passer par `FlakyCheckpointer`.
6. **Un commentaire C++ qui finit par une barre oblique inverse** commente la
   ligne suivante (vécu sur `ESCAPED_NEWLINES` : un `push_back` avait disparu).
7. **fish** ne découpe pas les variables et refuse les globs non protégés
   (`--include='*.cpp'` entre apostrophes).
8. **`sleep` au premier plan est bloqué** dans cet environnement ; lancer les
   longues commandes en arrière-plan, sortie redirigée vers un fichier.
9. **Arbre partagé** : commiter par chemins (`git commit -- <chemins>`), jamais
   `-A` ; vérifier `git config user.email` avant le premier commit d'un
   worktree (l'adresse personnelle, locale au dépôt).
10. **Ne pas relâcher un seuil ni conclure sur un compte de refus sans lire
    les messages** : c'est l'erreur du 18 septembre (§8).
