# L'insertion de vecteurs sous index HNSW — le coût, sa carte, les pas

Lot du cœur C++ ouvert le 12 octobre 2026 à 5 h 45 (orchestration), avec hotfixs (le DELETE d'une
ligne indexée) : un seul instrument, celui du banc (`vector_index_cost_test.cpp`, master
baf76c474), un seul profil (échantillonnage de piles du banc, `~/.cache/rag3db-banc-notes/`).

## 1. Le fait **[lu]**

- Produit : un index entier du dépôt passe 309 s sur 329 à écrire 15 631 vecteurs, ~20 ms par
  vecteur (SET par lots de 64 sur des lignes créées sans vecteur, UNWIND d'une liste de maps en
  paramètre, une transaction par lot, aucun point de reprise).
- Banc nu (768 dimensions, paramètre) : SET depuis NULL 1 à 4 ms par vecteur sous index, 0,25 ms
  sans ; le coût monte d'un lot à l'autre (1,02, 1,08, 1,88, 2,32 ms par lots de 500) ; 1, 8 et 32
  fils donnent les mêmes chiffres ; un CHECKPOINT entre les lots ne change pas la montée.
  CREATE_VECTOR_INDEX sur 15 000 vecteurs : 9 à 12 s (0,6 à 0,8 ms par vecteur).
- Le CHECKPOINT après ~15 000 SET depuis NULL : 24,9 s.

## 2. Le profil du banc (12 octobre, 5 h 50) **[lu]**

- CHECKPOINT : `ColumnChunkData::resize` 80 % en temps propre, par
  `scanCommittedUpdatesForColumn → iterateVectorInfo → applyCommittedUpdates → ListChunkData::write`
  — un `resize` par mise à jour appliquée sur la colonne FLOAT[768], chacun recopiant tout le
  tampon : quadratique.
- Insertion : `insertToLayer` 90 % ; `scanCommittedInMemRandom` 66 % → `ChunkedNodeGroup::lookup`
  57 % ; `shrinkForNode → detachDelete → findMatchingRow` 29 % ; rwlock 40 % en temps propre ;
  `VersionInfo::isSelected` 12 %.

## 3. La carte (lecture du code, 12 octobre) **[lu]**

Pour un SET d'une ligne sans vecteur sous index (ml 60, ef 200, seuil de rétrécissement 75) :
- à l'instruction : `initUpdateState` par ligne (un ensemble visité de N octets, deux graphes, six
  DataChunks, deux lookups de catalogue — jeté après la ligne) ; verrou INDEX exclusif et verrou de
  ligne ; `OnDiskHNSWIndex::update` : deux `detachDelete` (même sans ancien vecteur), puis
  `insertInternal` : une descente en couche haute, une recherche KNN basse à ef 200 (des milliers
  de lectures de vecteurs, chacune un lookup de colonne avec verrou partagé, version et la chaîne
  `UpdateInfo` parcourue en entier), puis `createRels` : 60 arêtes, et pour chacun des 60 voisins une
  arête inverse avec `prepareRelScan + scanFwd + count` (61 états de balayage par couche) ; au-delà
  de 75 arêtes, `shrinkForNode` : balayage, 75 vecteurs, `detachDelete` (par arête,
  `findMatchingRow` relit toute la liste : O(L²)), ≤ 60 réinsertions — ≈ 4 fois par vecteur ;
- au COMMIT : `RelTable::commit` ajoute chaque arête une à une au CSR validé en mémoire
  (`updateCSRIndex` retrie la liste entière du nœud par arête) ; les arêtes ne vont pas au journal ;
- au point de reprise : les mises à jour de la colonne s'appliquent une à une (le `resize`).
- La construction entière partage ce que l'incrémental refait par ligne : un ensemble visité par
  fil, les vecteurs contigus en mémoire (un pointeur par lecture), le graphe en tableaux, les arêtes
  écrites en bloc, aucun état de balayage.
- Ce qui grossit avec les SET depuis le point de reprise : la chaîne `UpdateInfo` (≈ 32 versions
  par vecteur de 2 048 lignes avec des lots de 64, borne indépendante de N — hotfixs), les lignes
  mortes du CSR en mémoire (chaque rétrécissement en marque 75 et en ajoute 60 : scans,
  `findMatchingRow` et tris les relisent), le stockage local de la transaction.

## 4. Les pas, et leurs mesures

**Mesure maison, en 768 dimensions, lots de 64 sous transaction, requête en texte** (3 200
vecteurs ; `~/.cache/rag3weaver-build/coeur-cpp/insertion/insertion.cpp`) : sans index 8,26 ms par
vecteur — **le texte d'une liste de 64 × 768 flottants coûte à lui seul ~8 ms par vecteur à
analyser et lier** ; le produit passe par un paramètre (`UNWIND $rows`), l'instrument du banc aussi
(0,25 ms sans index) : la mesure maison ne vaut que par sa différence avec et sans index.

| mesure maison (texte) | avant (master 41d282843) | après les pas 1-3 |
|---|---|---|
| avec index, lot 1 → lot 50 | 8,3 → 15,2 ms/vecteur | 9,6 → 11,9 ms/vecteur |
| total 3 200 vecteurs | 13,52 ms/vecteur | 10,90 ms/vecteur |
| part de l'index (moins les 8,26 sans index) | ~5,3 ms/vecteur | ~2,6 ms/vecteur |
| CHECKPOINT final (3 200 vecteurs) | 488 ms | 232 ms |

Les pas 1 à 3 (branche `insertion-vecteurs`) :
1. **`ColumnChunkData::resize` grandit au moins du double** quand il doit grandir (la capacité suit
   le tampon ; le disque reçoit toujours `numValues`) : le CHECKPOINT quadratique.
2. **`CSRNodeGroup::scanCommittedInMemRandom` prend le verrou partagé une fois par fenêtre**, pas
   par ligne, rendu le temps de changer de bloc (l'écrivain tient `chunkedGroups` puis
   `inMemAppendMtx` ; un lecteur ne tient jamais les deux).
3. **`detachDelete` supprime l'arête du sens direct par la ligne que son balayage rend déjà**
   (colonne ROW_IDX, `RelTableData::delete_(…, source, rowIdx)`), sans `findMatchingRow` ; le sens
   inverse garde le repérage.

4. **Le commit des arêtes d'un nœud par suites contiguës** (`RelTable::prepareCommitForNodeGroup`) :
   un append et une mise à jour de l'index CSR par suite — un seul tri de la liste — au lieu d'une
   ligne et d'un tri complet par arête (`updateCSRIndex`).

Et la garde de `NodeTable::lookupOffsets` (balayage sous masque) : un décalage masqué sans groupe
est passé, non déréférencé (SIGSEGV de `.~delete.DeleteAllThenCreate` trouvé par hotfixs : le
masqueur d'une recherche vectorielle retient des décalages que le balayage par fenêtres ne lisait
jamais).

**Mesure de l'instrument du banc** (`VectorIndexCost.MeasureInsert`, 768 dimensions, lots de 500 en
paramètre, 1 fil ; avant sur master 50f2591ba bâti en entier, après sur le lot, rien bâti entre) :

| ms par vecteur, lots de 500 | lot 0 | lot 1 | lot 2 | lot 3 |
|---|---|---|---|---|
| début du remplissage, avant | 1,56 | 2,08 | 3,10 | 4,01 |
| début du remplissage, **après** | 1,56 | 2,00 | 2,49 | **2,85** |
| fin du remplissage (13 000 déjà), avant | 1,02 | 1,12 | 1,93 | 2,39 |
| fin du remplissage, **après** | 0,97 | 1,06 | 1,53 | **1,79** |
| sans index (référence) | 0,22 | 0,22 | 0,22 | 0,22 |

−29 % au lot 3 ; **la montée d'un lot à l'autre reste** (1,56 → 2,85 en 2 000 vecteurs) : ce sont les
lignes mortes du CSR validé en mémoire (chaque rétrécissement en marque 75 et en ajoute 60, que
chaque balayage de voisins, `isSelected` compris, relit) — le pas suivant est la compaction de la
liste d'un nœud (ou un index de ses lignes vivantes), lot à part. Le CHECKPOINT quadratique a son
témoin au banc (`Columns/CheckpointAfterUpdates.ScalesWithTheUpdatedRows`, ×156 avant), sorti du
known_red avec le pas 1. Les pas restants côté extension (hotfixs) : `initUpdateState` par ligne
(un ensemble visité de N octets par SET), les 61 `prepareRelScan` par `createRels`, et (A) pour le
DELETE.

Leçons de mesure : `LD_LIBRARY_PATH` ne remplace pas la lib d'un binaire de test (RPATH prime) ; un
`cmake --build` pendant une mesure réécrit l'extension vector sous elle ; un « avant » se joue sur un
checkout de master bâti en entier, l'« après » sur la branche, rien entre.

**Vérification du banc sur master 9da274476 (8 h 25)** : `Columns/CheckpointAfterUpdates` vert
dans ses trois cas — FLOAT[768] : CHECKPOINT après 15 000 SET **576 ms** (24 s avant le lot ; la
référence par COPY fait 550 ms : à égalité), STRING 38 ms, INT64 2,1 ms ; dans la forme du produit
(`VectorIndexCost.ProductShapeDeleteCost`), le CHECKPOINT passe de 19-25 s à **1,1 s**. Le DELETE
direct reste rouge (218 contre 5,4 ms) : la part d'hotfixs.

## 5. La pente sans point de reprise : compacter les lignes mortes du CSR en mémoire **[mesuré]**

Le banc (témoin `VectorIndexInsertSlope.CostPerBatchSettles`, master cb6c1a2cb, ticket moteur
« sans point de reprise, l'insertion sous HNSW ralentit lot après lot ») : sans point de reprise le
coût par lot de 64 monte en ligne droite (~1,1 ms par lot, 78 → 165 ms sur 64 lots), avec un
point de reprise tous les 8 lots il reste plat (~105 ms) ; profil propre : `scanCommittedInMemRandom`
4 → 29 %, `isSelected` 8 % — les lignes mortes relues ; la chaîne des versions 9 % ;
`findMatchingRow` < 1 % (le pas 3 a fait son travail). Borne `insert-cost-without-checkpoint-stays-close`
(sans ≤ 1,2 × avec, + 2 ms) ; le rapport plateau / début (×1,2 à ×1,4, les rétrécissements) est
un ticket extension pour hotfixs.

**Ce que fait le code** (lecture du 12 octobre, 10 h) : `NodeCSRIndex{isSequential, rowIndices}`
par nœud dans `CSRIndex` (csr_node_group.h:31-98), sous `csrIndexMtx` (exclusif aux ajouts du commit
via `updateCSRIndex`, partagé le temps que `scanCommittedInMem` **copie** la liste du nœud —
ligne 256 — avant de la balayer ligne à ligne) ; une suppression validée en mémoire n'est qu'un
marqueur (`deletedVersions[row] = commitTS` au commit), la liste garde la ligne ; le point de
reprise filtre par `isDeleted` puis jette tout (`csrIndex.reset()`), sûr parce que
`stopNewTransactionsAndWaitForOthers` a vidé les transactions. **Il n'existe aucun « plus ancien
instantané actif »** : le TransactionManager a `activeTransactions` sous son mutex public, tenu
pendant tout commit, où `min(startTS)` se calcule en O(#actifs).

**La forme** : (1) un filigrane `oldestActiveStartTS` calculé sous le mutex du TransactionManager
au commit (hors la transaction qui valide ; `lastTimestamp` si personne), posé sur la transaction ;
(2) `CSRNodeGroup::delete_` reçoit l'offset du nœud (il manque à `VersionRecord`) et compte les
mortes par nœud (`deadHints`, hors de `NodeCSRIndex` que les lecteurs copient) ; (3) au commit,
dans `CSRNodeGroup::append` (déjà sous `csrIndexMtx` exclusif, sur le mutex public), quand un nœud
a des mortes, sa liste est reconstruite sans les lignes `isDeleted(oldestActiveStartTS, …)` — tout
lecteur actif ou futur les voit supprimées ; les blocs ne sont pas libérés (un lecteur en cours garde
sa copie et ses lignes adressables) ; ordre de verrous `csrIndexMtx` → `chunkedGroups` → `versionInfo`
partagé, jamais inversé ; les suppressions de la transaction qui valide portent encore son identifiant
(l'annulation est validée après) : elles partent au commit suivant qui touche le nœud. Gains : la
copie, les recherches de `scanCommittedInMemRandom`, le tri d'`updateCSRIndex` sur les vivantes.
Limite : un nœud jamais réalimenté garde ses mortes jusqu'au point de reprise. Témoin : celui du banc
(rapport sans/avec sous 1,2), plus un lecteur à instantané ancien ouvert pendant les commits qui
doit encore voir ses arêtes.

**Codé et mesuré (12 octobre, 12 h à 15 h, branche `compaction-csr`, sur master `c44af8031`).** La forme ci-dessus, à deux
différences près : le filigrane se lit sur `snapshotFloorTS` (startTS tel que le gestionnaire le
connaît : la `LatestCommittedView` ne change `startTS` que sur son fil, le temps d'un balayage, et
le lire depuis le commit d'un autre serait une course), et la compaction a deux occasions, pas
une : la validation qui ajoute au nœud (`compactDeadRows` dans `append`) **et la fin de la
validation qui a supprimé** (`UndoBuffer::compactDeadRowsAfterCommit`, une fois les suppressions
datées) — sans la seconde, un rétrécissement HNSW (supprimer puis recréer dans la même
transaction) laissait ses mortes au commit suivant qui touche le nœud, et un nœud touché une fois
les gardait jusqu'au point de reprise. Les notes (`deadHints`, un octet par nœud du groupe, et la
liste des nœuds notés) vivent hors de `NodeCSRIndex`.

Témoins `CSRDeadRows` (`test/transaction/csr_dead_rows_test.cpp`, longueur de la liste lue par
`CSRNodeGroup::getNumInMemRows`) : sous un lecteur ancien, les listes gardent leurs mortes (1 000 →
1 600 en trois tours) et il garde ses 1 000 relations et leur somme ; lui parti, la validation
suivante ramène 1 000, une suppression seule 800 ; la forme du rétrécissement (30 tours) reste à
64, grandit de 64 par tour sous un lecteur, revient à 64 dès la validation qui suit son départ ;
mêmes comptes après CHECKPOINT et réouverture.

Le témoin du banc, sous `poste mesure` sur la lib du lot :

| 5 derniers lots | sans point de reprise | avec (tous les 8 lots) | rapport |
|---|---|---|---|
| master (banc, cinq passes) | 163 à 230 ms | 108 à 137 ms | ×1,5 à ×1,74 |
| master (ma passe de 12 h 20, sous charge) | 197 ms | 139 ms | ×1,42 |
| **le lot**, passe 1 | **154 ms** | **131 ms** | **×1,17** (vert, borne ×1,2) |
| le lot, passe 2 | 169 ms | 109 ms | ×1,55 (rouge) |
| le lot, passe 3 | 152 ms | 126 ms | ×1,20 (vert, à la borne) |
| le lot rebasé sur master 0c0af5bf5 (avec 2bd65cbb8 de hotfixs), passe 4 | 154 ms | 113 ms | ×1,37 (rouge) |

Un rouge sur trois : le cas ne sort pas de known_red pour un vert, il passe en probabilistic.txt
avec ses comptes (la règle du banc : trois passes vertes avant de sortir un cas), et le ticket
reste ouvert sur ce qui reste.

Ce qui reste de pente sans point de reprise (135 → 160 ms du lot 22 au lot 63, contre
110 → 130 avec) n'est plus le CSR, et la passe 4 montre que ce n'est pas non plus la chaîne des
versions seule (2bd65cbb8 de hotfixs est dans ce bâti). Lecture : c'est le coût propre du chemin
de lecture en mémoire — les relations et les vecteurs écrits depuis le dernier point de reprise se
lisent ligne à ligne (`ChunkedNodeGroup::lookup`, `isSelected`, l'info de mise à jour) là où le
chemin persistant balaie des colonnes ; sans point de reprise, la part des voisins lus par ce
chemin croît avec chaque lot. À profiler (eu-stack échantillonné sur les derniers lots, comme le
banc l'a fait) avant d'y toucher ; le cas est en probabilistic.txt, le ticket ouvert sur cette
part.

## 6. La lecture en mémoire ligne à ligne **[en cours, 12 octobre 15 h 35 à …]**

Le lot de l'orchestration après la compaction (profil du banc sur master c44af8031 :
`scanCommittedInMemRandom` 4 → 27 %, `VersionInfo::isSelected` 0 → 18 % entre les lots 0-15 et
50-63 ; chaîne des versions et chemin persistant stables). Branche `lecture-en-memoire`, mesure
par la pente du banc sous `poste mesure` : **avant, master 65cb9a1f3 bâti entier : 147,5 / 109,6 ms,
×1,35**.

**Ce que fait le code (lu).** Une liste CSR non séquentielle (compactée, ou touchée par un
rétrécissement) se lit ligne à ligne : `ChunkedNodeGroup::lookup` par ligne, soit un verrou
partagé `versionInfoMtx`, `getQuotientRemainder`, le vecteur de versions et ses deux switches,
puis `ColumnChunk::lookup` par colonne (segments, puis l'info de mise à jour). Aucun résumé de
bloc n'existe : le raccourci `ALWAYS_INSERTED` de `VectorVersionInfo` n'est jamais posé en mémoire
(`commitInsert` écrit les TS sans changer le statut). Le balayage de plage (`ChunkedNodeGroup::scan`)
écrit toujours à partir de la position 0 de la sortie.

**Première forme — le résumé par bloc** (`VersionInfo::allRowsVisibleTo` : insertions toutes
validées au plus tard à l'instantané, aucune suppression, résumé invalide s'il vient du disque)
et **les suites contiguës lues d'un bloc quand le bloc est tout visible** (`scanRun`, `dstOffset`
ajouté à `ColumnChunk::scan` et `UpdateInfo::scan`). Témoins `InMemoryReadShortcut` 2/2 (visibilité
inchangée, suites et DELETE par numéro de ligne), suites complètes vertes. **Pente après : 163 /
111, ×1,47 : rien.** Les blocs font 1 024 lignes (CHUNKED_NODE_GROUP_CAPACITY = 2^10) : chaque
rétrécissement supprime partout, `anyDeletion` exclut tout.

**Deuxième forme — les suppressions encore listées** : `listedDeletions` (delete_ +1, rollback −n)
que la compaction décrémente quand elle retire la ligne morte de sa liste (`forgetDeletion`), avec
une garde : la compaction d'un groupe ne retire rien tant qu'une copie de liste est en vol
(`CSRNodeGroupScanState::listCopiedFrom`, `CSRNodeGroup::inFlightListCopies`) — sinon un lecteur
à copie ancienne relirait la ligne morte que le résumé rendrait vivante. **Pente après : 168 / 111,
×1,52 : rien.** La trace (`RAG3DB_TRACE_INMEM`) le dit : 6,9 M lignes lues par suite, 28,7 M
ligne à ligne, 21 M suites refusées, toutes pour « suppressions listées », zéro compaction
sautée : ce sont les suppressions **de la transaction elle-même** (les rétrécissements du lot en
cours), que rien ne peut oublier avant sa validation, et le lecteur est l'écrivain.

**Troisième forme — les suites lues en bloc avec leur filtre** : `scanRun(…, runSel)` prend le
verrou une fois par suite, calcule la sélection de la suite par `getSelVectorToScan`, copie les
colonnes en bloc à la position de sortie, et `scanCommittedInMemRandom` accumule les positions des
lignes visibles dans le tampon de la sélection de sortie (filtrée à la fin si une ligne d'une suite
ne l'était pas ; l'identité pendant la fenêtre). Bâtie ; témoins, trace et pente après 3 à jouer
après la pause de Lucie (16 h 30).
