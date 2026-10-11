# Cœur C++ — rapport de session

Session « cœur C++ » : le moteur (fork de Kuzu), son journal, sa reprise après arrêt,
l'index vectoriel, les lecteurs et écrivains concurrents, les verrous à venir.
Mis à jour sur place. **Dernière mise à jour : 10 octobre 2026 — pause pour le redémarrage du poste.**

Le registre commun est `docs/journal-des-chantiers.md` (§1 pour l'ordre et les
livraisons, §4 pour les décisions, §6 pour les défauts). Ce fichier dit ce que le journal
ne dit pas : comment reprendre, et pourquoi les choses sont dans cet ordre.

## Ce qui est livré (tout est sur `master`)

| Quoi | Commits | Ce que ça ferme |
|---|---|---|
| Marche T0 | `88f57e2ba`, `4c13619d1`, `a34538c9a` | après une erreur dans `BEGIN … COMMIT`, tout est refusé jusqu'au `ROLLBACK` |
| Marche A2 | `e81423a28`, `875da1772` | les offsets provisoires d'une transaction sont remappés au commit |
| Champ nul d'une liste de structures | `17c1ae41d`, `909c2573c` | `UNWIND $rows … CREATE` avec un champ `NULL` |
| Index vectoriel, deux défauts | `e64839489`, `13284a0fe` | un `DELETE` annulé ne rend plus l'index muet ; un survivant reste atteignable |
| Marche A5 | `1175cc0a2`, `e415e0277` | lecteurs contre suppression : informations de version et index des relations en mémoire sous verrou |
| Marche A5 bis | `7487fae08` | l'ajout en mémoire ne réalloue plus un bloc sous une recherche vectorielle |
| `DROP` d'index au rejeu | `f5acca417` | le rejeu retire aussi l'index de la table |
| Garde 1 de la reprise | `fcd9a7882` | une base ne plante plus à l'ouverture ; l'index se dit « en retard » |
| Garde 2 de la reprise ; borne du contrôle des voisins | `15fcc3474`, `110a65f15` | après un arrêt brutal l'index vectoriel reste juste sans être rebâti (`<base>.extensions`) ; la mise à jour d'un vecteur ne lance plus une recherche par ancien voisin (dix mille lignes par lots : 42 s au lieu d'environ 800) |
| Perte de relations au point de reprise | `80e3f2c32` | un point de reprise ne libère plus les relations des régions qu'il n'a pas réécrites (perte silencieuse, défaut d'origine) ; le plantage à la lecture après une relation créée puis supprimée |
| Gardes de mémoire ; taille exacte pour l'index vectoriel | `1ea49837f` | trois refus nommés à la place de trois écritures hors bloc (tableau des visités, graphe en mémoire, décalages du dictionnaire) ; l'index se dimensionne par le nombre de lignes, plus par la cardinalité estimée. Sans test neuf : voir le ticket |
| Après un `COPY` refusé | `05788a868` | le point de reprise n'écrit plus hors de son bloc (corruption de tas, défaut d'origine) ; la ligne d'origine d'une clé en double reste dans l'index de clé primaire (résultat faux silencieux, défaut d'origine) |
| Chargement journalisé, étape 2 (les nœuds) ; refus du `COPY` après des insertions | `0f4a54b2c` | derrière `force_checkpoint_on_copy=false`, un `COPY` de nœuds écrit ses lignes au journal et ne force plus de point de reprise ; dans une transaction, un `COPY` dans une table où elle a déjà inséré est refusé par son nom (il rendait des résultats faux, défaut d'origine) |
| Gestionnaire de verrous, marche V1 | `022c78402` | le gestionnaire seul, sans câblage : ressources ligne et index, partagé et exclusif, attente, interblocage à la prise, délai, interruption, prises groupées, `CALL lock_timeout` ; aucune écriture ne prend encore de verrou |
| Double ouverture en écriture dans un processus | `57c8389b4` | la cause de la corruption d'`e2e_code` : refus nommé `Database::ALREADY_OPEN_FOR_WRITING` avant de toucher à un fichier ; un lecteur du même processus reste permis |
| Clés fantômes après deux `COPY` annulés | `5c8507577` | régression de `05788a868` trouvée par l'arbre principal : l'annulation retire maintenant de l'index les clés de toutes les lignes non validées, et seulement elles |
| Chronométrage du point de reprise | `5771f0afb` | `RAG3DB_PROFILE_CHECKPOINT=1` : le découpage d'un commit et de son point de reprise sur la sortie d'erreur ; muet sinon |
| Relire ses relations ; voisins d'un vecteur mis à jour | `c8fdaf196` | une transaction relit juste ses relations après en avoir supprimé (défaut d'origine, par Cypher) ; la mise à jour d'un vecteur garde ses anciens voisins joignables |
| Chargement journalisé, étape 3 (les relations) | `1cfba2d6a` | derrière le même réglage, un `COPY` de relations écrit ses relations au journal (par le partitionneur, sous le verrou qui réserve leurs identités) ; un chargement entier est durable par le journal seul. Le témoin ne prouve pas l'ordre entre plusieurs fils |
| Sonde des deux sens | `168a63901` | une relation introuvable dans l'un de ses deux rangements fait refuser sa mise à jour ou sa suppression (`RelTable::REL_NOT_FOUND_IN_ONE_DIRECTION`) ; sans test, la condition ne se fabrique pas par l'interface |
| Chaînes permutées au point de reprise | `25b3b45dc` | défaut d'origine, résultat faux écrit sur disque : la relecture partielle d'un segment de chaînes échangeait les chaînes des lignes d'une région (propriétés `STRING`, `BLOB`, `STRUCT` à chaîne des relations ; listes de chaînes des relations **et des nœuds**). Garde `DICTIONARY_INDEX_OUT_OF_RANGE`. Ne répare pas : une base écrite avant se réindexe ; `tools/check_rel_directions` dit si des relations sont atteintes |
| Un `COPY` après des insertions de la même transaction | `f1d8c7190` | le `COPY` verse d'abord dans la table les lignes locales de sa transaction (`LocalStorage::flushNodeTable` : le commit d'une table, appelé plus tôt) ; le refus nommé du 4 octobre disparaît ; les quatre tests Cypher d'origine reprennent leur forme. Non prouvé sous plusieurs écrivains |
| L'annulation prévient les index | `e1049934e` | après un `COPY` annulé sur une table à index vectoriel, l'index ne se croit plus en avance (la recherche échouait, puis le `COPY` suivant n'était pas relié, en silence) ; crochet `Index::rollbackInsert` ; refus « is behind its table » si le compte dépasse quand même la table. Le refus n'a pas de témoin (le banc l'écrit par les internes) |
| Le plantage de la reprise | `35d09c466` | poser par `SET` le vecteur d'une ligne créée dans la même transaction, sur une table indexée, plantait (`shrinkForNode`) : la lecture groupée des vecteurs des voisins rendait un tableau plus court que demandé. Défaut d'origine, sans rapport avec l'annulation ; c'est la reprise ordinaire de rag3weaver |
| Transactions à point de reprise forcé | `37608cf4b` | une transaction qui porte un `COPY` hors journal n'écrit plus rien au journal : entière ou pas du tout. Deux défauts d'origine du chemin par défaut : la transaction « écritures puis `COPY` » revenue à moitié après une mort pendant son point de reprise ; la base qui ne se rouvrait plus après un `COPY` à lignes écartées |
| Journal d'un `COPY` après un `DROP` | `47a80373e` | le `COPY` journalisé écrit les propriétés du catalogue, pas les colonnes du stockage (plantage du `COPY` de relations après `ALTER … DROP` puis `ADD`, mon défaut de l'étape 3) |
| Point de reprise sans fin après un `COPY` annulé | `836edfc29` | après un gros `COPY` annulé, refusé ou à court de mémoire, le point de reprise suivant ne finissait plus et prenait des gigaoctets : le compte de l'index de clé en mémoire recule au retrait d'une clé, sa réservation ne passe plus sous zéro ; le compte de lignes et le curseur de réservation de la table reculent avec l'annulation. Deux défauts d'origine. Seuil mesuré : 16 000 lignes passent, 18 000 non |
| La clé perdue par l'index qui grandit | `eb2d78e46` | défaut d'origine de `HashIndex::splitSlots` : une lecture en mémoire libérée quand une part de l'index fait plus que doubler laissait une clé sur disque sous une mauvaise empreinte — ligne présente, introuvable par sa clé, pour toujours. Trouvé par l'arbre principal (deux `COPY` de 50 000 symboles). Une base déjà touchée se réindexe |
| Statistiques d'un `COPY` dans sa transaction (étape 4, lot 1) | `1177f5794` | la cardinalité et les comptes de distincts d'un `COPY` ne rejoignent la table qu'à la validation ; un `COPY` annulé ou refusé ne les gonfle plus ; des insertions suivies d'un `COPY` dans la même transaction ne sont plus comptées deux fois (deux défauts de `f1d8c7190`) |
| Forme compacte des tableaux au journal | `1c232f318` | un tableau de taille fixe de numériques s'écrit en octets bruts : un `FLOAT[768]` pèse 3,1 Ko au journal au lieu de 9,3, par `COPY` comme par `SET` ; trois numéros d'enregistrement neufs (40, 42, 45), l'ancien décodage gardé ; un journal neuf n'est pas lisible par un moteur d'avant |
| Sonde du poids du journal | `c1c2f9dfc` | `RAG3DB_PROFILE_JOURNAL=1` : à chaque validation, les octets du journal de la transaction par type d'enregistrement et par table, sur la sortie d'erreur |
| Repli d'un `COPY` journalisé (étape 4, lot 2) | `71cffbc4b` | au-delà de `copy_journal_threshold` (défaut : un huitième du tampon, plafonné à 256 Mio), la transaction vide son journal et redevient durable par son point de reprise ; compteur `copy_journal_fallbacks`. Inerte tant que `force_checkpoint_on_copy` vaut `true` |
| Le fichier de la base a une étendue connue | `0aed3c4b5` | les pages d'un `COPY` tué ou rejoué ne sont plus perdues : l'étendue du fichier dans l'en-tête au point de reprise (version de stockage 40), l'excédent rendu à l'espace libre à l'ouverture en écriture ; 559 pages perdues par COPY de 200 000 lignes tué, avant |
| **Le COPY journalisé devient le défaut** (condition 2 de la stèle) | `ff9bad960` | `force_checkpoint_on_copy=false` par défaut ; huit tests adaptés (un `CHECKPOINT` après le COPY qu'ils regardent ; l'attente du délai ne vaut que pour un COPY forcé par le réglage ; un jumeau journalisé qui n'attend pas un lecteur ouvert) ; au banc, `ExplicitCopyCommitWhileATransactionIsOpen` en deux formes, `LockBench.TwoCommits…` verdi |
| Transaction forcée sans journal en mémoire | `fb98852e1` | une transaction à point de reprise forcé (tout `COPY` d'aujourd'hui) ne sérialise plus son journal en mémoire pour le jeter : 411 Ko puis 823 Ko gardés avant, 0 après |

A5, A5 bis et la garde 1 corrigent des défauts **atteignables en service avec un seul
écrivain**, pas seulement sous le mode multi-écrivains (qui reste éteint hors du banc).

## Ce qui est en cours

### Ce qu'une seconde session cœur C++ doit savoir (10 octobre 2026)

Lucie ouvre une seconde session cœur C++ pour les correctifs et les tickets ; celle-ci garde
la stèle (fuite de pages → basculement du COPY journalisé → verrous → écritures parallèles) et
tient `src/transaction/` et `src/storage/` (journal, tampon, verrous) : demander avant d'y
toucher ; le reste est à l'autre.

1. **Un arbre à soi**, worktree de `rag3db` (jamais l'arbre principal
   `/home/lucied/git_workspaces/rag3db`, ni `rag3db-moteur` qui est celui-ci, ni `rag3db-banc`) ;
   `git config user.email luciedefraiteur@gmail.com` ; sous-modules à initialiser
   (`third_party/fuzzy-fst` pour l'extension fts). Un bâti Release avec les tests :
   `cmake -B build/moteur -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTS=ON -DBUILD_EXTENSIONS="vector;geo;fts"`,
   puis **toujours** `~/.cache/rag3weaver-build/poste lourd cmake --build build/moteur -j 8`
   (jamais `ninja` nu : alias `-j32`, le poste fige). Un rebâti complet : 20 à 30 min ;
   `transaction_test` seul : 4 min. Un seul rebâti exclusif à la fois à trois (banc, celle-ci,
   elle) : le dire par message avant de lancer.
2. **La liste complète** : `~/.cache/rag3db-moteur-notes/copy-journalise/liste-cpp.sh`
   (douze suites gtest, le banc comparé à `known_red.txt`, vector disque et mémoire, Cypher
   1866) sous `poste lourd`, ~40 min ; ses codes dans `copy-journalise/liste/codes`. Une fois
   avant la fusion, pas à chaque rebase ; pendant le travail, les témoins du changement.
   Jamais de rebâti dans un arbre où une liste tourne. `poste mesure` seulement pour ce qui
   mesure.
3. **Les pièges** : une assertion dans une fonction auxiliaire (`ok()`) n'arrête pas le test —
   lire la première erreur d'un rouge ; un COPY n'écrit ses pages avant la validation qu'à
   131 072 lignes ; le tampon des tests est petit (un index vectoriel de 140 000 lignes le
   déborde : `systemConfig->bufferPoolSize`) ; un témoin de mort tue dans la portée de la
   `Database` et exige un `.wal` non vide ; pas d'`EXPECT_EQ` entre deux grands textes ; un
   programme nu lié à `build/moteur/src` (annexes, `essai-*.cpp`) mesure plus vite qu'un
   témoin.
4. **Les règles** : commit par chemins explicites, message en français sans attribution, push
   en avance rapide seulement, pas de `git stash` (patch), un ticket par défaut non corrigé
   dans `docs/tickets/`, le rapport de session tenu dans ce dossier (le sien à part), rien vers
   les amonts.
5. **Ce qui lui est confié** (par l'orchestration) : `RelCopyBMExceptionRecoverySameConnection`
   (lire d'abord pourquoi le repli ne s'est pas déclenché sous un tampon minuscule), le
   plantage HNSW sur vecteurs identiques (recette de l'arbre principal, avec le banc), la voie
   (a) des statistiques, les tickets confort. Les tickets ouverts sont listés plus bas.

**Où j'en suis (10 octobre 2026, soir).** **La condition 2 de la stèle est fermée** : le
COPY journalisé est le défaut du moteur, `ff9bad960` sur `master` (liste complète finale verte
sous ce défaut, contrôle de fuite de pages compris ; série de confirmation des embarquements :
fichiers 80 → 81 s, blobs 89 → 87 s ; relecture du banc). La fuite de pages est fermée
(`0aed3c4b5`). Toutes les sessions sont prévenues : leur lib se rebâtit sur ce master, et le
défaut a changé. Mon arbre `rag3db-moteur` est propre sur `copy-journalise-par-defaut`, égale à
`master`. **La suite** : la page courte des verrous — ce que V2, A3′, A4′ et l'index au commit
changent réellement, dans l'ordre que je propose — avant de coder (note de conception du 3
octobre : `docs/3-octobre-2026-15h47/01-note-de-conception-les-verrous.md`) ; puis les écritures
parallèles. La seconde session (hotfixs/tickets) tient les correctifs ; elle m'a demandé et obtenu
trois retouches dans `src/storage` (un commentaire, le recalage sur les lignes vivantes avec deux
gardes, et `ColumnStats::update` qui lisait les hachages à la mauvaise position sous une
sélection filtrée). Piège du jour, deux fois : sous la porte `poste`, `cmake --build` a répondu
« rien à faire » après un changement d'en-tête, et une liste a tourné sur l'ancien binaire —
vérifier le nombre d'étapes du bâti avant de croire une liste ; en direct, `ninja -n` voyait le
travail.

**Pause du 10 octobre 2026 (redémarrage du poste, noyau mis à jour).** Reprise de la
veille : chantier B du plan `../../8-octobre-2026-16h29/orchestration/01-plan-de-reprise.md`.

### Où j'en suis exactement

- **Branche `etendue-du-fichier`**, poussée sur `origin` (hash dans le message « prêt » à
  l'orchestration ; `git log origin/etendue-du-fichier`), basée sur `master` à `27eeb6a7f`.
  **Pas à fusionner dans `master`** : ni liste complète, ni relecture du banc, ni témoins joués
  sur le correctif.
- Elle porte : le correctif de la page 06 (l'étendue du fichier dans l'en-tête, version de
  stockage 40, l'excédent rendu à l'ouverture en écriture — `DatabaseHeader::numDataPages`,
  `Checkpointer::returnOwnerlessPages`) ; les neuf témoins `OwnerlessPagesTest` ; sept contrôles
  de fuite ajoutés aux témoins du point de reprise interrompu (`checkpoint_test.cpp`) ; la base
  de la version 39 (`test/transaction/database_before_extent/`, compressée, lue par `gzip -dc`) ;
  et la copie de `NodeTableDeleteState` interdite (chantier G).
- **Vérifié** : sans harnais, sur la bibliothèque corrigée, un COPY de 200 000 lignes tué avant
  sa validation laisse 4 pages occupées au lieu de 559 ; replié, 4 au lieu de 560 ; journalisé
  validé puis rejoué, 4 au lieu de 559. L'ancien moteur refuse une base en version 40 :
  « Trying to read a database file with a different version. Database file version: 40 ».
  Les quatre premiers témoins ont été vus **rouges** sur le moteur d'avant : 640, 638, 637 et
  1 269 pages occupées pour 9.
- **Pas vérifié** : les neuf témoins et les sept contrôles n'ont pas été joués sur le correctif
  (le bâti de `transaction_test` a échoué sur une liste d'initialisation du témoin de parité,
  corrigée depuis ; le rebâti a été arrêté pour le redémarrage). Le témoin du plein texte
  (`TheReplayReadsNothingBeyondTheExtentWithAFullTextIndex`) se saute tant que l'extension fts
  n'est pas bâtie : le sous-module `third_party/fuzzy-fst` est initialisé dans l'arbre, mais la
  reconfiguration `cmake -DBUILD_EXTENSIONS="vector;geo;fts"` a échoué avant lui et a laissé
  `build/moteur` en configuration incomplète — **à refaire d'abord** (ou revenir à
  `vector;geo`), puis rebâtir.
- **À la reprise, dans l'ordre** : reconfigurer et rebâtir `build/moteur` sous `poste lourd` ;
  jouer `OwnerlessPagesTest.*`, `FlakyCheckpointerTest.*`, et les suites de la reprise ; le
  témoin plein texte ; la liste complète ; le contrôle de fuite de la suite Cypher sur toute la
  liste **défaut basculé** (exigence c) ; relecture du banc ; rebase et avance rapide.
- Deux exigences de l'orchestration en plus du correctif : (a) le rejeu ne lit rien au-delà de
  l'étendue — témoigné par la réutilisation de l'excédent et la parité avec une base témoin,
  pour id/texte/nombre/FLOAT[4], avec index vectoriel, avec index plein texte ; (b) le point de
  reprise interrompu après sa marque — les sept contrôles.
- Le ticket est reclassé (défaut du moteur d'aujourd'hui, hors stèle, corrigé maintenant) ;
  page 06 et relevé complétés (seuil du groupe plein : 131 072 lignes).

**Mise en pause (5 octobre 2026, 10 h).** Lucie reprend le week-end ou un soir. Cette section
suffit pour reprendre ; les sections plus bas sont plus anciennes.

### L'état exact

- `master` est à `fb98852e1` (moteur) ; les docs suivent. Tout ce qui est fini est poussé.
- L'arbre `rag3db-moteur` est **propre**, sur la branche locale `copy-journalise-etape-4`, égale
  à `master`. Son `build/moteur` est **à moitié rebâti** (un rebâti interrompu à la pause) :
  commencer par `~/.cache/rag3weaver-build/poste lourd cmake --build build/moteur -j 8`.
- Rien n'est en cours d'exécution de mon côté : ni liste, ni agent.
- Un brouillon non commité est gardé en patch : les quatre témoins de la fuite de pages,
  `annexes/etape-4/temoins-pages-sans-proprietaire.patch` (à appliquer sur
  `test/transaction/journaled_copy_test.cpp`). Ils n'ont **jamais été bâtis ni joués**.

### Ce qui a été fait le 5 octobre (dans l'ordre)

1. **Deux bloquants de la stèle fermés, sur l'index de clé primaire** : le point de reprise sans
   fin après un `COPY` annulé (`836edfc29`), et la clé perdue quand l'index grandit
   (`eb2d78e46`, une lecture en mémoire libérée dans `HashIndex::splitSlots`, trouvée par l'arbre
   principal sur deux `COPY` de 50 000 symboles). Une base déjà touchée se réindexe.
2. **L'étape 4 du chargement journalisé**, en lots : les statistiques d'un `COPY` dans sa
   transaction (`1177f5794`) ; la forme compacte des tableaux au journal (`1c232f318`) ; la sonde
   `RAG3DB_PROFILE_JOURNAL` (`c1c2f9dfc`) ; le repli au seuil (`71cffbc4b`) ; la transaction
   forcée sans journal en mémoire (`fb98852e1`).
3. **Deux marches sans retour**, acceptées par l'orchestration : un journal écrit par le moteur
   d'aujourd'hui n'est plus lisible par un moteur d'avant `1c232f318` (« unknown WAL record
   type 40 ») ; et le correctif à venir de la fuite de pages changera la version de stockage.

### Ce que les mesures ont appris

- **Le `COPY` journalisé ne fait pas gagner de temps** sur une première indexation (mesure des
  embarquements, 7 014 fichiers) : 80 s contre 78–79 en mode fichiers ; 117 s contre 78–89 quand
  le plein texte vit en base, parce que ses blobs sont alors écrits deux fois. Le basculement se
  justifie par la stèle (plus de point de reprise forcé, plus d'attente à la validation,
  plusieurs `COPY` dans une transaction), pas par la vitesse.
- **Ce qui pèse au journal** (sonde, mode fichiers, 466 Mio pour 62 Mo de texte) : les relations
  168 Mio ; le texte écrit deux fois, dans `Scope` et `Scope_Chunk`, environ 124 Mio (un fait de
  schéma de rag3weaver) ; le coût par ligne hors texte, environ 175 Mio. La plus grosse
  transaction est le chargement final des relations (158 Mio), pas un paquet ; le plus gros
  paquet de 2 048 fichiers pèse 114,5 Mio.
- **Décisions de l'orchestration** : seuil par défaut confirmé (256 Mio au plus) ; le mode à
  blobs demande lui-même le point de reprise forcé, côté rag3weaver (l'arbre principal le pose
  dans sa branche des nouveaux défauts) ; le coût par cellule au journal est un ticket de
  confort, après le basculement.

### Ce qui bloque le basculement du défaut

**La fuite de pages après une réouverture** — ticket
`docs/tickets/2026-10-05-pages-d-un-copy-journalise-perdues-au-rejeu.md`, page de conception
`06-les-pages-sans-proprietaire.md`, **acceptée par l'orchestration, rien de codé**.

- Mesuré : après un `COPY` journalisé et une réouverture sans point de reprise, 403 pages
  occupées pour 11 attendues, 949 pour 11, 232 pour 5 (le contrôle de fuite de la suite Cypher,
  défaut basculé dans l'arbre).
- La cause, lue et non rejouée : le fichier de la base n'a pas d'étendue connue — le nombre de
  pages en mémoire est la taille du fichier à l'ouverture. Par raisonnement, **non mesuré**, le
  `COPY` forcé d'aujourd'hui tué avant son point de reprise fuit de la même façon.
- Le correctif accepté : écrire l'étendue du fichier dans l'en-tête à chaque point de reprise
  (nouvelle version de stockage) ; à l'ouverture en écriture, rendre à l'espace libre tout ce qui
  dépasse ; pas de troncature.
- Exigences de l'orchestration : témoigner que le rejeu ne lit rien au-delà de l'étendue (`COPY`
  journalisé, index vectoriel, plein texte) ; le point de reprise interrompu après sa marque ;
  le contrôle de fuite joué sur toute la liste ; une base du moteur d'avant qui s'ouvre et
  n'est pas abîmée ; relecture du banc.

### La suite, dans l'ordre

1. Rebâtir `build/moteur`. Appliquer le patch des témoins de la fuite, les bâtir, **les voir
   rouges** (c'est la mesure qui manque : le `COPY` forcé tué, le `COPY` replié tué, deux morts de
   suite). Si le `COPY` forcé tué fuit, l'écrire au ticket : c'est un défaut du défaut actuel.
2. Coder le correctif de la page 06, ses huit témoins, la liste complète, la relecture du banc.
3. `CopyTest.RelCopyBMExceptionRecoverySameConnection`, rouge sous le défaut basculé : sous un
   tampon minuscule le `COPY` de relations échoue vingt fois ; regarder d'abord pourquoi le repli
   ne s'est pas déclenché (un plancher du seuil ?) avant d'accuser le test.
4. Rejouer la liste « défaut basculé » (`sed` sur `forceCheckpointOnCopy` dans
   `src/include/main/client_config.h`, **sans le commiter**), le banc compris — il s'arrêtait à
   `ExplicitCopyCommitWhileATransactionIsOpen`, un défaut de son harnais que le banc corrige.
5. Adapter les huit tests qui supposaient le point de reprise d'un `COPY` (liste au message de
   tri rendu à l'orchestration : `CopySegmentTest`, `DeleteAllRels`, `FSMReclaimRelNewTable`,
   `FSMReuseFreePageForNewTable`, `DropNodeTableReclaim`, `DropNodeTableRollbackRecovery`,
   `AutoCheckpointTimeoutErrorTest`, `JournaledCopyTest.ByDefaultACopyStillForcesItsCheckpoint`),
   un par un, les « Reclaim » lus avec soupçon.
6. Dire aux embarquements de jouer leur série d'avant basculement (sur ≥ `fb98852e1` : fichiers
   à ±3 % sous `COPY` journalisé, blobs à leur temps d'avant), puis basculer le défaut.
7. Ensuite : les verrous.

### Ce que d'autres sessions me doivent, ou attendent de moi

- **Le banc** : il ajoute la mort pendant le point de reprise d'une transaction repliée (ses sept
  points de mort) et corrige la portée des résultats dans son harnais. Il m'enverra à relire son
  correctif de l'élagage de l'index vectoriel (`shrinkForNode` : la règle classique, avec ses
  chiffres) — **je lui dois cette relecture**, rejeu et chemin disque d'abord.
- **Les embarquements** : leur série d'avant basculement attend mon signal.
- **L'arbre principal** : il pose le point de reprise forcé pour le mode à blobs, et rejoue ses
  témoins sur les correctifs de l'index.

### Tickets ouverts le 5 octobre, non traités

- la fuite de pages (bloque le basculement) ;
- le journal d'une transaction sans borne hors `COPY` ;
- le coût par cellule au journal (confort) ;
- un `COPY` de relations annulé gonfle l'estimation du nombre de relations (confort) ;
- une extension chargée au rejeu sans contrôle de bâti (robustesse du produit) ;
- à ouvrir : des pages jamais rendues à l'annulation dans une session qui continue (création
  d'un index plein texte annulée, `ALTER … ADD` annulé, point de reprise annulé) — lu dans les
  commentaires du code, non mesuré.

### Mes erreurs du jour, pour ne pas les refaire

- Un quatrième cas de témoin annoncé « rouge » alors qu'il échouait pour une instruction refusée
  plus haut (une assertion dans une fonction auxiliaire n'arrête pas le test) : lire la première
  erreur d'un rouge avant de le compter.
- Un rebâti lancé dans l'arbre pendant qu'une liste y bâtissait : la liste est partie de travers.
  Ne rien bâtir dans un arbre où une liste tourne.
- Un rebâti complet parti hors de `poste`.
- Un « core dump » annoncé à l'arbre principal sans l'avoir regardé : c'était mon essai, qui
  chargeait une extension d'un autre bâti.

**Ce que les lots de l'index de clé laissent ouvert :**
- la comparaison du banc a rendu une fois « Segmentation fault, no result » (sous un plafond de
  12 Go, avant le correctif de `splitSlots`) ; jamais revue en quatre comparaisons depuis. Cause
  non prouvée ; la lecture en mémoire libérée de `splitSlots` est l'hypothèse ;
- après le dernier rebase (sur `62f5c9817`) je n'ai rejoué que `transaction_test`, `copy_tests`,
  `local_hash_index_test` et la comparaison du banc ; la liste complète était verte juste avant ;
- quatre suspects faibles relevés par le banc en chassant le même motif (rapport du banc,
  `13d2dff01`), pas encore en tickets : `mergeSlot` (un pointeur dans une page que l'ajout peut
  désépingler), `disk_array.cpp:48`, `dictionary_chunk.cpp:63-66`, et
  `OverflowFileHandle::setStringOverflow` si la lecture optimiste rejoue sa fonction ;
- TSan : les tests ouvrent la base avec `MAX_DB_SIZE=68719476736`, sinon la réservation de 8 Tio
  du gestionnaire de tampon est refusée (« Mmap for size 8796093022208 failed ») et la passe ne
  prouve rien — c'est arrivé une fois, vu au compte des rapports resté à zéro avec onze échecs.

*(Les paragraphes qui suivent datent d'avant ces deux lots.)*


**Où j'en suis (5 octobre, 1 h 30).** Tout est poussé, l'arbre `rag3db-moteur` est propre, sur
la branche locale `copy-apres-insertions` (égale à `master`, `e1049934e`). La suite, dans l'ordre
de l'orchestration : l'étape 4 du chargement journalisé avec le banc (prévenir l'orchestration
avant de retirer le remède 2a) ; la forme compacte des vecteurs au journal ; l'étape 5 ; puis
le câblage des verrous.

**Le plantage de l'arbre principal est réglé** (`35d09c466`) ; il rejoue sa sonde.

**Les lignes introuvables après des `COPY` annulés** (témoin du banc) : ce n'est pas
l'annulation. La ligne manque avant le premier `COPY` annulé, une passe sur deux, et
l'annulation rend exactement l'état d'avant (huit passes dans un seul processus). C'est une
ligne injoignable dès la construction par des `COPY` successifs — l'élagage des voisins à
l'insertion ne garantit pas qu'une ligne garde une arête entrante. Tout est au ticket
`docs/tickets/2026-10-04-ligne-lointaine-injoignable-index-bati-d-un-coup.md` : la recette,
les huit passes, ce que font hnswlib, pgvector et Qdrant (aucun ne garantit la
joignabilité), et la décision de l'orchestration — mesurer d'abord sur un corpus réel
(arbre principal), puis une passe de rattrapage hors du chemin chaud si le taux n'est pas
nul. **Rien à coder avant la mesure.** Le filet « is behind its table » et le remappage du
versement sous deux écrivains ont maintenant leurs témoins au banc (`efd76bdc9`).

**L'étape 4 du chargement journalisé est en cours, et elle est plus grosse que prévu.**
Basculer le défaut et jouer la liste d'origine a rendu des défauts que le réglage éteint
cachait ; l'état, l'ordre et le compte des passes sont à la page
`04-le-chargement-en-masse-journalise.md`, §7 bis, et à la stèle (§3.2, point 4). **Rien
n'est basculé sur master.** Branche locale : `copy-journalise-etape-4`, égale à `master`
(`47a80373e`), arbre propre. Dans l'ordre :
1. le plantage de `concurrence_test` sous le défaut basculé — à identifier : bâtir avec
   `forceCheckpointOnCopy = false` dans `src/include/main/client_config.h`, lancer le
   binaire seul sous `poste`, lire le dernier `[ RUN ]` ;
2. mon contrôle du journal qui masque une erreur de tampon plein
   (`NodeTable::logInsertedRowsToWAL`, « 0 rows were written to the journal ») ;
3. la borne de mémoire du journal d'une transaction (4b) : au-delà d'une taille, le journal
   local déborde dans un fichier à lui à côté de la base, recopié à la validation, jeté à
   l'annulation et à l'ouverture, dans la liste des fichiers d'une copie ;
4. les pages perdues au rejeu (ticket du banc, sa piste) ;
5. la liste défaut basculé, jusqu'au bout ; puis le basculement, avec les deux témoins du
   banc qui exigeaient l'attente (le prévenir avant), les miens, les tests d'origine adaptés
   (chacun dit pourquoi son attendu change), la description du réglage (transitoire, ce qui
   le fera retirer).
Reste aussi, non décidé : journaliser le `COPY` qui écarte des lignes (plus aucun `COPY`
forcé) — il faut une forme de journal pour un trou ; et la graine réglable de l'index
vectoriel, à ajouter en repassant dans l'extension.

Un défaut d'origine trouvé en passant, en ticket, non corrigé : retirer une colonne déclarée
avant la clé primaire casse l'insertion ordinaire
(`2026-10-05-drop-d-une-colonne-declaree-avant-la-cle-primaire.md`).

Ce qui reste ouvert des lots de la nuit :
- le refus « is behind its table » du compte en avance n'a aucun témoin ;
- mon essai jetable sur la base de l'arbre principal a planté à la fermeture (appel d'un
  pointeur nul dans `NodeTable::serialize`, au point de reprise de `~Database`) — peut-être un
  artefact de l'essai (deux `Database` dans le processus de test), non poursuivi ;
- aucun témoin du chargement journalisé (étapes 2 et 3) ne porte d'index vectoriel : le trou
  est dit, le banc y ajoute deux formes à sa famille `ExtensionIndexRecovery` ;
- le compte de l'index est un entier simple lu et écrit sans verrou : pour la passe TSan de
  l'extension, jamais faite ;
- trois tickets ouverts par lecture, non exécutés ou sans effet avec un seul écrivain : `COPY`
  de relations vers des nœuds locaux sans remappage (A3′), RTree de geo (confort), et le
  remappage du versement sous plusieurs écrivains (dans le code) ;
- du lot des chaînes : Vela non regardé ; `expectSameRows` jamais vue en rouge ; un rouge
  isolé jamais revu (`transaction~ddl~ddl_tinysnb.AddInt64PropertyWithoutDefaultRollbackRecovery`,
  « Cannot open file …db.kz.shadow »).

**La panne de 22 h 51.** systemd-oomd a tué toutes les sessions. Cause établie : un
`EXPECT_EQ` rouge de gtest entre deux textes de 400 000 lignes (un test neuf du banc, joué
sur le moteur non corrigé, hors de `poste`) — gtest calcule la différence dans une table
quadratique. Mesuré sous plafond : 4 000 lignes, 186 Mio ; 20 000 lignes, plus de 4 Gio.
Le moteur n'y est pour rien (le même scénario sans gtest : 438 Mio). Depuis, `poste` met
chaque commande dans sa portée avec un plafond (`POSTE_MEM_MAX`, 40 Gio par défaut).

**La corruption de mémoire qui tue `e2e_code`** — tout est dans le ticket
`docs/tickets/2026-10-04-memoire-corrompue-dans-e2e-code.md` : les mesures (une mort sur 60
avec la bibliothèque de master, donc antérieure à la garde 2), la pile d'AddressSanitizer
(une écriture de 4 096 octets après un bloc de 32 768, dans `DictionaryColumn::scanValue`,
la lecture d'une colonne de chaînes longues par un balayage), ce qui a été essayé sans effet.

Où j'en suis exactement (16 h 40, seconde limite de l'orchestration atteinte : huit passes) :
- **Fait nouveau** : avec l'extension vector bâtie elle aussi sous ASan, un second débordement,
  dans l'extension — `VisitedState::contains` lit l'indice 255 d'un tableau de 211 octets
  dimensionné par la cardinalité estimée de la table (`query_hnsw_index.cpp:293`) ; son
  pendant `add` écrit un octet hors tableau. Même test, même base sur disque. Un rapport sur
  six passes réelles ; le rapport de `scanValue` n'est pas revenu.
- **La condition n'est pas établie.** Un contrôle « cardinalité < nombre de lignes » n'a
  jamais parlé en quatre passes : soit le retard est rare, soit c'est le voisin rendu par le
  graphe qui est faux. Détail, lectures et suite au ticket.
- L'arbre est propre (`110a65f15`), l'extension ordinaire est rebâtie. **`build/asan` porte
  encore la bibliothèque instrumentée** : la rebâtir avant de s'en servir. L'instrumentation
  est en patch dans `annexes/instrumentation-traque-4-octobre.patch`, avec
  `build-traque.sh` et `boucle-traque.sh`.
- Leçon : un contrôle provisoire se joue une fois seul avant d'entrer dans une boucle
  comptée — deux passes sur huit sont mortes de mes propres contrôles.
- Piège : l'extension vector sort dans `extension/vector/build/`, commun à tous les dossiers
  de build de l'arbre ; ninja ne la refait pas si le fichier d'un autre dossier est plus
  récent : supprimer le fichier puis rebâtir la cible.

**La stèle du moteur** (`docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md`, décision de
Lucie) : le point d'arrêt de version du moteur. J'y ai écrit le tri des tickets (§3.1) et
corrigé le §2. Leçon du jour, à garder : un contrôle plus strict que nécessaire (« la clé
mène dans la plage annulée ») a fait planter `copy_tests` — la liste complète l'a attrapé
avant le push ; ne jamais pousser sur les seuls tests du changement.

**La corruption d'`e2e_code` est close côté moteur** : deux exemplaires de la base ouverts
en écriture dans le même processus (ticket, avec la méthode qui l'a trouvée). Les outils de
la traque sont dans `annexes/` : `sonde-base-fautive/` (rouvrir une base abîmée, relire ses
tables, relever les plages de pages). Le journal des pages, lui, n'a pas été gardé en patch
(retiré de l'arbre sans copie, ma faute) ; il se refait en une demi-heure : une fonction qui
imprime l'opération, la plage et la pile (`backtrace`, adresses relatives par `dladdr`, à
résoudre par `addr2line`) quand le chemin du fichier porte un fragment donné par une variable
d'environnement, appelée dans `PageManager::allocatePageRange`, `freePageRange`,
`freeImmediatelyRewritablePageRange` et `finalizeCheckpoint`, dans
`FileHandle::writePagesToFile` et `removePageIdxAndTruncateIfNecessary`, et dans
`ShadowFile::applyShadowPages`.

**L'ordre, revu par Lucie le 4 octobre à 20 h 40 (stèle §3.2)** : le `COPY` hors journal
est la cause commune de ce qui a été payé ce jour-là ; **le chargement en masse journalisé
passe avant le câblage des verrous**. Donc : V1 (fait, `022c78402`) → le chargement
journalisé → A3′, A4′, V2, la maintenance de l'index au commit → les écritures parallèles.
Les autres tickets « bloque » sont au banc.

**Le lot en cours : le chargement en masse journalisé.** Tout est dans
`04-le-chargement-en-masse-journalise.md`, §7 bis pour l'avancement : étapes 1 et 2 faites,
**étape 3 (les relations) à faire**. Ordre décidé par l'orchestration pour la suite : étape 3,
puis le vrai correctif du `COPY` après des insertions (ticket « bloque la stèle » : verser les
lignes locales dans la table au début du `COPY`, ce qui retire le refus et rend quatre tests
Cypher d'origine à leur forme), puis l'étape 4 avec le banc (retrait du forcé et de son
remède 2a), la forme compacte des vecteurs avec une version du journal, l'étape 5.

Pour l'étape 3, ce qu'il faut savoir : `RelBatchInsert::initGlobalStateInternal` force
aujourd'hui le point de reprise ; la journalisation des relations doit suivre l'ordre du
stockage comme pour les nœuds ; le rejeu existant est `replayRelTableInsertRecord`. Le banc
prépare deux témoins (mort au milieu et juste après un `COPY` validé). Tenir le compte des
passes au fil de l'eau, erreurs comprises : l'orchestration le rend à Lucie.

**V1, ce qu'il faut savoir pour la suite** : `src/transaction/lock_manager.{h,cpp}`, un
`LockManager` par `TransactionManager`, `releaseAll` appelé dans `clearTransactionNoLock`.
Le genre de ressource « base » envisagé pour l'attente du départ des autres n'est pas
construit (voir l'ajout daté de la note de conception des verrous). Tests :
`test/transaction/lock_manager_test.cpp`, à rejouer sous ThreadSanitizer (`build/tsan`,
cible `transaction_test`) à chaque changement du gestionnaire.

**Le point de reprise de `COPY`, voie (c) : rendu.** Le découpage (mesure de la session des
embarquements avec `RAG3DB_PROFILE_CHECKPOINT=1`) : les 5 s par paquet sont celles d'une
seule table, `_index_blobs`, réécrite en entier à chaque point de reprise (48,9 s sur 75 à
une validation par paquet, 23,8 sur 28 à une validation tous les quatre). Le remède est
parti côté rag3weaver (pousser les blobs une fois, à la fin) ; côté moteur c'est un ticket
d'optimisation, hors stèle. La demi-page reste `03-le-point-de-reprise-de-copy.md` ; ses
voies (b) et (a) attendent.

**Leçons de la soirée, à garder.**
- Mon premier correctif de l'index (`05788a868`) reposait sur une lecture fausse de ce que
  l'annulation balaie. Avant de corriger un chemin d'annulation, imprimer ce qu'il relit
  réellement (plages, lignes rendues) : dix minutes d'instrument auraient évité la
  régression.
- Avant de poser un refus sur un chemin, chercher si la batterie d'origine le tient pour
  permis (`grep` dans `test/test_files`) : quatre tests Cypher inséraient puis copiaient dans
  une transaction, et une liste complète a été perdue à le découvrir.
- Un balayage de table (`NodeGroup::scan`) attend une sélection neuve à chaque bloc ; et le
  masque ne filtre pas les lignes d'un bloc qui porte des informations de version. Filtrer
  soi-même par décalage.
- Quand un défaut intermittent abîme un fichier, la première chose à obtenir est le fichier
  abîmé : des refus nommés à la place des plantages l'ont laissé sur disque, et vingt minutes
  de lecture de pages brutes ont dit plus que la journée d'hypothèses sur le code.
- Un témoin ne doit pas vérifier la seule ligne qui a révélé le défaut : celui de la clé en
  double ne regardait que la clé 5, et cachait que toutes les voisines étaient perdues.

## Les amonts et la licence (4 octobre 2026)

Décision de Lucie : on lit les amonts, on ne leur transmet rien ; tout reste sous LRSL ;
par défaut, **lire, ne pas copier**.
- Ladybug est sous MIT (« Copyright (c) 2022-2025 Kùzu Inc. — Copyright (c) 2025-2026
  Ladybug Memory Inc. », `LICENSE` au tag `ladybug-main-2026-08-31`). Notre `LICENSE` est la
  LRSL et renvoie à `NOTICE` pour la notice MIT de Kuzu ; `NOTICE` ne mentionne pas Ladybug.
- Ce qui a été repris, et comment : les gardes des index non chargés (`d2db8acb4`,
  `1ba0cc540`) — lues puis réécrites (`NodeTable::isWritableIndex`, l'index détaché, le refus
  nommé sont à nous) ; la ligne de `LocalRelTable::scan` (`ffe855873`) — écrite ici avant
  d'avoir lu la leur, retrouvée identique ensuite ; le `DROP` d'index au rejeu — correction
  différente de la leur. Aucun bloc copié ; non vérifié par un outil de comparaison.
- La règle : le message de commit dit ce qui vient d'où (« lu puis réécrit », commit de
  l'amont cité). Copier un bloc — le port de `e92346c97` serait le premier cas tentant — se
  demande avant ; c'est Lucie qui tranche l'ajout d'une ligne à `NOTICE`.

## L'ordre, et pourquoi

1. **La garde 2 de la reprise** : que le rejeu ait l'extension avant de rejouer. Sans elle,
   chaque mort pendant une écriture dans une table indexée coûte un index entier à rebâtir,
   sur le chemin de chaque première indexation.
2. **La mise à jour massive de vecteurs** : rare chez nous et contournable ; avant V1
   seulement si l'invariant côté produit rougit.
3. **Les verrous** (décision de Lucie : avant H4) : V1, A3′, A4′, V2, puis la maintenance de
   l'index vectoriel au commit — une marche du plan, pas un affinage : sans elle deux
   écrivains sur la même table indexée s'attendent du début à la fin.
4. **H4**, ce qu'il en reste (le doublon de clé qui empêche de rouvrir : A3′ et une reprise
   qui met de côté la transaction fautive).
5. Le port de la recherche par clé par ligne de Ladybug ; la marche A5 ter.

## Ce qui attend quelqu'un

- **Session de l'arbre principal** : reconnaître `is behind its table` à l'ouverture, faire
  `DROP_VECTOR_INDEX` puis `CREATE_VECTOR_INDEX`, et le dire dans le reçu d'ouverture.
  Regarder si `e2e_idempotent_registration` ferme vraiment avant de rouvrir.
- **Lucie** : supprimer sur `origin` les branches d'avant rebase
  (`a5-suppression-sure-entre-fils`, `-2`, `-3`, et celles listées au journal §1).
- **Une mesure au calme** du coût d'A5 bis sur `e2e_search` et l'ingestion (marche A5 ter) :
  jamais faite, le poste était trop chargé.

## Comment reprendre

**Les arbres.**
- `~/git_workspaces/rag3db-moteur` : mon arbre de travail. Builds : `build/moteur` (Release,
  tests, extensions `vector;geo`), `build/lecteurs-csv` (bibliothèque partagée pour Rust),
  `build/tsan` (ThreadSanitizer, cibles `concurrence_test` et `transaction_test`). Target
  cargo dans `extension/rag3weaver/target`.
- `~/git_workspaces/rag3db-docs-note` : arbre détaché, pour pousser les docs vers `master`.
- Ne rien écrire dans l'arbre principal `~/git_workspaces/rag3db`.

**Livrer une marche du moteur** (la liste, dans l'ordre) :
1. La liste C++ : `annexes/liste-cpp.sh` (build, `transaction_test`, `api_test`,
   `c_api_test`, `copy_tests`, les huit suites de stockage, le banc comparé à
   `known_red.txt`, les tests de l'extension vector sur disque et en mémoire, la batterie
   Cypher). Avec ThreadSanitizer quand la marche touche à la concurrence (la version d'A5
   bis du script le fait).
2. La passe Rust, une fois : `annexes/passe-rust-1.sh` puis `-2.sh` (lib, dix-huit suites
   e2e, binaires, neuf scripts Python). Annoncer cargo à l'orchestration avant.
3. Reposer sur `origin/master` sous un **nom de branche neuf** (jamais de push en force),
   vérifier par `git diff --stat` ce que master a apporté, pousser en avance rapide :
   `git push origin <branche>:master`.
4. Le journal des chantiers, depuis l'arbre des docs.

**Les pièges, tous rencontrés.**
- `ctest -R concurrence_test.known_red` depuis la racine du build ne trouve aucun test et
  sort 0. Lancer `compare_known_red.cmake` directement (voir `liste-cpp.sh`).
- Les binaires Rust demandent la feature `code` ; sans elle la construction échoue et les
  scripts Python passent quand même, sur d'anciens binaires. Lire le code de sortie.
- Un test de reprise qui ferme la base avec son point de reprise final ne rejoue rien :
  exiger un journal non vide juste avant de rouvrir. `CREATE_VECTOR_INDEX` et `COPY`
  écrivent eux-mêmes un point de reprise ; la reprise aussi, à la fin du rejeu.
- Un processus qui a déjà chargé l'extension vector ne voit pas les défauts du rejeu sans
  extension : rouvrir dans un processus neuf (`exec`).
- Un témoin vert avant comme après un correctif ne prouve rien : celui des relations relues
  de travers était vert parce que ses relations étaient encore en mémoire — il manquait un
  `CHECKPOINT`. Le jouer d'abord sans le correctif.
- Un compte de lignes joignables dans l'index varie d'une passe à l'autre : seuil « toutes
  joignables », plusieurs passes.
- La pile de `git stash` est commune à tous les arbres du dépôt : ne pas s'en servir.
- **Un chemin derrière un réglage éteint n'est éprouvé que par ses propres témoins.**
  Basculer le défaut et jouer la liste d'origine AVANT d'écrire quoi que ce soit : c'est la
  liste d'origine qui fait le travail de témoin (deux plantages et quatorze rouges que mes
  étapes 2 et 3 n'avaient pas vus).
- Un correctif d'une ligne se joue chez celui qui a le témoin le plus dur avant d'être
  poussé : le banc a montré que « la base se rouvre » laissait une transaction à moitié.
- Ne pas ajouter un essai à un fichier de tests pendant qu'une liste attend son tour : il
  entre dans son bâti et, s'il plante, tronque la suite.
- Un témoin de diagnostic se vérifie à sa PREMIÈRE étape avant d'être lu : six passes du
  témoin des lignes injoignables étaient nulles (fichiers `.batchN` sans `.csv`, aucun `COPY`
  ne passait, « 128 manques sur 128 » partout).
- Une corrélation entre deux fins de test n'est pas une cause : « rouge avec annulation, vert
  avec mort » venait du rejeu qui rebâtit le graphe, pas du `ROLLBACK`. Mesurer AVANT
  l'opération accusée, dans le même processus.
- Ne pas rebâtir l'extension vector pendant qu'une liste tourne : les suites la chargent à
  l'exécution (`extension/vector/build/`), le rebâti la remplace sous elles. Écrire le code,
  bâtir après.
- Un test écrit pour une hypothèse se joue avant d'être annoncé comme témoin : mon filet « is
  behind its table » était annoncé comme la réponse au plantage de l'arbre principal avant
  d'avoir vu sa base, qui ne le déclenche pas.
- Deux commits qui se partagent un fichier se découpent par un patch inverse des fichiers du
  second, le premier commité, puis le patch réappliqué (`git apply -R`, jamais `git stash`).
- **Jamais d'`EXPECT_EQ` entre deux grands textes à plusieurs lignes** : comparer par `==` et
  écrire un résumé borné (`expectSameRows` dans `rel_string_property_checkpoint_test.cpp`).
  Rouge, gtest réserve (lignes + 1)² cases.
- Le pic de mémoire d'une passe : `/usr/bin/time` n'existe pas sur ce poste ; lire
  `memory.peak` du cgroup de la portée (`annexes/borne.sh`, à lancer sous `poste`).
- Laquelle des deux formes de requête (`->` ou `<-`) lit quel rangement d'une table de
  relations dépend du planificateur : aucune n'est une référence, on compare les deux.
- Une étendue annoncée d'après ses propres variantes n'est pas l'étendue du défaut : « nœuds
  et listes indemnes » était faux, la relecture du banc a montré l'appelant manquant
  (`ListColumn::scanSegment`). Chercher les appelants du code fautif avant d'annoncer.
- **Le verrou de poste** (depuis le 4 octobre) : tout ce qui est lourd — build, liste C++,
  passe Rust, boucle — se lance sous `flock -s ~/.cache/rag3weaver-build/poste.lock` ; une
  mesure de durée ou de mémoire sous `flock -x`. Deux mesures d'une autre session ont été
  faussées par une boucle et un build lancés sans prévenir.
- `compare_known_red.cmake` prend un argument de plus, `-DLONG_FILE=…/long.txt` ; sans lui
  il sort en erreur. Les cas longs du banc sont à part (`CONCURRENCE_LONG=1`).
- Reposer une branche pendant qu'un script de passe tourne : le script compile un arbre en
  conflit. Finir le rebase d'abord, lancer ensuite.
- Une ligne très loin des autres est injoignable dans un index bâti d'un coup : ne pas
  s'en servir comme sonde dans un test qui prouve autre chose.
- Le poste est partagé par plusieurs sessions : une mesure de temps faite sous charge ne
  départage rien. Noter la charge (`uptime`) à côté de chaque chiffre.
- Dans un nouvel arbre ou après un rebase : `git submodule update --init
  extension/rag3weaver/codeparsers`, et vérifier `git config user.email`.
- Le mode de permission peut refuser un push vers `master` : s'arrêter et le dire à Lucie,
  ne pas contourner. Un appel refusé a pu s'exécuter en partie : vérifier l'arbre.
- Mes messages aux autres sessions donnent des faits avec `fichier:ligne` ; ce qui n'est pas
  vérifié est dit non vérifié. Deux constats faux ont circulé ce soir (H4 « seulement sur
  une table indexée », le « second défaut » du `MERGE`) : tous deux corrigés au journal.

**Les annexes** (`annexes/`) : les scripts de livraison et de mesure, les tests brouillons
qui ont servi aux mesures (à remettre dans `test/transaction/` et dans son `CMakeLists.txt`
pour les rejouer), et les patchs essayés puis écartés. Ils vivaient dans
`~/.cache/rag3db-moteur-notes/`, qui n'est pas durable ; les journaux d'exécution y sont
restés.

## A3′ faite (10 octobre 2026, 20 h 30) — sur master en avance rapide

Trois commits sur master : `21b6cb370` (le câblage), `79b7be75f` (known_red, le témoin de la
famine, deux témoins du banc adaptés), `9a57a17b2` (les trois témoins demandés à la relecture, la
forme forcée des deux validations dans known_red, la mesure). Relu par le banc (rag3db-10) à
chaque étape ; liste complète jouée deux fois : sur le poste principal au premier lot, puis sur
**luciepc** sur la branche rebasée (transaction_test 221, api 104, c_api 136, copy 23, stockage
81, banc conforme à known_red 50 rouges comparés / 197 verts, vector 74 et 63, Cypher 1867).

Ce qu'A3′ fait, en une ligne chacune (le détail : page 07 §5, le knowledge dump §4) :
- l'insertion prend l'index de sa table en partagé et sa clé en exclusif, jusqu'à la fin de la
  transaction ; le COPY prend l'index en exclusif, **sur le fil du client avant
  l'ordonnancement** ; une seconde insertion de la même clé attend, puis reçoit l'erreur de clé en
  double ou passe, selon que la première a validé ou annulé ;
- l'unicité se contrôle contre le dernier état validé, avec **une seule visibilité** à
  l'insertion et à la validation ;
- une attente de verrou sur un fil ouvrier lance un **fil de remplacement** de l'ordonnanceur
  (`TaskScheduler::BlockingWait`) ; le témoin de la famine est rouge sans lui (59,8 s) et vert
  avec (216 ms) ;
- tout cela sous le mode multi-écrivains seulement, **éteint hors du banc**.

Ce que le banc compte : 59 rouges attendus avant A3′, 56 après le rebase sur master (mes 10
lignes sorties, une ligne entrée pour la forme forcée des deux validations, C7 après arrêt brutal
qui change d'étiquettes et passe sous A4′ ; les lignes ajoutées par le banc sur master entre-temps
s'y ajoutent). Les messages des commits disent 50 puis 52 : ils comptaient avant le rebase.

La mesure (luciepc, un million de lignes par `UNWIND … CREATE` dans une transaction, pool de 8
fils, trois passes, tenue exclusive, **non comparable aux chiffres du poste principal**) :

| mode | insertion (ms) | validation (ms) |
|---|---|---|
| multi-écrivains éteint (le produit) | 1 073 / 790 / 722 | 242 / 233 / 244 |
| allumé (un verrou de clé par ligne) | 1 479 / 1 437 / 1 408 | 465 / 485 / 476 |

Sous les verrous : +0,65 µs par ligne à l'insertion, +0,23 µs à la validation (rendre un million
de verrous). Le mode éteint est le chemin d'avant plus un test booléen par ligne ; la comparaison
avec `ff9bad960` sur la même machine reste à faire quand un bâti d'avant y existera (confort :
la clé en octets bruts, la libération par lot, pour le jour où le mode s'allume dans le produit).

Ce qu'A3′ ne fait pas, et où ça va : la part **reprise** d'un journal à doublon (le témoin du
banc reste rouge sur `database-reopens`, il rejoue le journal gardé au dépôt,
`test/transaction/journal_with_duplicate_key`) — à faire avec A4′ ou juste après ; le COPY de
relations ; **A4′** (mises à jour, suppressions, relations) ensuite, puis V2 (la `CALL`, à
prévenir rag3db-50 avant `src/function`), puis l'index au commit.

Tickets : fermé « L'annulation d'un COPY efface les lignes d'un autre écrivain » (par le verrou) ;
ouvert « Une erreur pendant la validation, après l'ajout des lignes aux groupes, laisse la
transaction à moitié validée » (demandé par le banc) ; signalé à rag3db-50 un rouge probabiliste
hors du lot (IGNORE_ERRORS et la ligne de clé en double gardée).

Le cœur C++ travaille désormais sur **luciepc** (décision de Lucie, 10 octobre au soir) :
worktree `~/git_workspaces/rag3db-moteur` là-bas, `build/moteur` gardé entre les lots ; ici, le
worktree moteur n'a plus de bâti ; les notes restent dans `~/.cache/rag3db-moteur-notes/a3/`.

Faute du jour, notée en mémoire : un `pidof concurrence_test` nu a tué un test du banc ; on ne
tue que les pids qu'on a lancés.

## A4′ (10 octobre 2026, nuit) — ce qui est fait, ce qui reste

Branche `verrous-a4-2` (rebasée sur master `e0fad1325` : IGNORE_ERRORS, l'index vectoriel, analyze),
six commits, relue par le banc au fil de l'eau ; les sémantiques d'un coup d'œil : page 07 §0 ;
ce que le code a appris : page 07 §6. En une ligne chacune, sous le mode multi-écrivains :
la ligne en exclusif (clé) à la mise à jour et à la suppression, puis l'erreur de sérialisation
si un autre a validé une écriture de la ligne après l'instantané ; les deux extrémités d'une
relation en partagé pour créer, en exclusif pour écrire ; `DELETE` refusé par
`has connected edges` sur le dernier état validé ; `DETACH DELETE` qui détache aussi ce qu'il
n'a pas vu à son instantané (Neo4j) ; les écritures internes d'un index sans verrou.

Trouvé en route : H2 validait à moitié (les arêtes de l'index prenaient les extrémités) ;
les huit cas Cypher `WWConflict*` de l'amont attendent désormais le verrou (délai de 300 ms,
erreur nommée) ; deux lignes de V1 (interblocage, interruption) verdissent avec A4′.

**Reste nommé** : C7_RandomMix après arrêt brutal (relations à une extrémité manquante qui
n'existent qu'après le rejeu ; le témoin « détachement sur le dernier état validé, mort base
ouverte, rejeu » est vert, le chemin n'est pas trouvé) — ticket, à chercher avec le banc ; la
seconde suppression d'une même relation après attente nommée « Write-write conflict » (ticket) ;
les témoins {1,0} et C5 réécrits par le banc sur sa branche, à relire.

**Sur master** (avance rapide, 11 commits de `9904fefc0` à `67ff3816b`, le 10 octobre à 23 h 40 ;
les cinq derniers sont les témoins du banc, relus). Passe, sur luciepc, bâti de la branche
rebasée : transaction_test 233, api 104, c_api 136, copy 23, stockage 81 + column_stats 2,
vector 74 et 63, Cypher 1867 (les huit `WWConflict*` adaptés compris) ; banc de concurrence en
tenue exclusive, conforme à `known_red.txt` : 15 rouges comparés, 238 verts (21 lignes attendues,
dont 6 mises de côté par `long.txt` et `probabilistic.txt`) — 56 lignes avant A4′, 21 après.
Les deux premières comparaisons, jouées pendant qu'un autre banc tournait là-bas, avaient
montré des « waited » de charge : c'est ce qui a donné la règle du soir (le banc entier en tenue
exclusive, annoncée). Le message complet du lot : `~/.cache/rag3db-moteur-notes/a4/message.txt`.

## V2 (11 octobre 2026, 0 h 15) — l'annonce en tête, faite

Branche `verrous-v2` (page 08 ; `CALL acquire_locks('Table', [clés])` en tête d'une transaction
explicite : une prise groupée, index partagé et clés exclusives, puis l'instantané repris après
l'attente ; refus nommés en auto-commit, après une écriture, deux fois ; rien hors du mode
multi-écrivains). Témoins : trois d'un fil verts, `AnnouncedLocksNeverDeadlockAndBothCommitInTurn`
du banc vert (39 ms), sa ligne sortie de `known_red.txt` (20 attendus). La liste de fusion joue
sur luciepc (suites en lourd, banc en tenue exclusive annoncée) ; push en avance rapide ensuite,
lib commune rebâtie, worktree effacé. Suite : l'index au commit (page 07 §2, dépend de la page 04
du banc), et les tickets ouverts d'A4′ (C7 après arrêt brutal, la seconde suppression d'une
relation), la part reprise du journal à doublon.

Passe, sur luciepc, bâti de la branche rebasée sur master (`97acd579b`, le contrôle de bâti des
extensions compris) : transaction_test 238, api 104, c_api 136, copy 23, stockage 81 +
column_stats 2, vector 74 et 63, Cypher 1867 ; banc de concurrence en tenue exclusive conforme à
`known_red.txt`, 13 rouges comparés, 240 verts (19 lignes attendues). Une course de témoin
trouvée en route et bornée avec l'accord du banc : « l'une après l'autre » se mesure au début du
commit du premier (le verrou est rendu pendant le COMMIT, avant sa marque).

## I1 (11 octobre 2026, 2 h 30) — le genre « index », fait

Branche `verrous-i1-2` (page 09 §0 le découpage, §5 ce que le code a tranché) : tout écrivain
d'une table indexée tient l'index en exclusif de sa première écriture à sa validation,
l'instantané repris après l'attente. Témoins : six d'un fil, et au banc IndexLockBench ×3, H3 ×2
verts (sortis de `known_red`, 15 attendus), H2 ×4 et H5 ×4 réécrits par le banc verts. Tickets
ouverts en route : la recherche qui ne rend plus qu'une ligne après annulations puis mises à jour
(au banc), le CALL nu qui rend des nœuds supprimés (ticket du banc). I2 (la maintenance au
commit) attend la mesure du banc.

Le cas Cypher `insert.TwoOpenTransactionsInsertVectors` de l'extension vector (un seul fil, deux
transactions ouvertes qui insèrent dans une table indexée) supposait deux écrivains simultanés :
adapté à la règle (délai nommé de 300 ms, annulation, insertion une fois la première validée),
vert dans les deux modes. Rebasée sur master `6044723a5` (`bfc1434e1` du banc dedans, sans
conflit ; `known_red` : 11 attendus, les quatre VectorIndexUpdate sortis par le banc) et poussée
`verrous-i1-3` (`b549e607f`). Sur ce bâti, le témoin composé de la recherche à une seule ligne
(`liveRowsByIndex == 100`) est **vert** : le ticket est fermé par les correctifs du banc
(`6b13c6610`, stèle §2 à cinq). Le banc en mesure, lancé puis **retiré** : hotfixs pousse le
lecteur affamé (du C++ dans `src/storage`) avant ma fusion, et un rebase qui apporte du C++ se
rejoue — une seule passe, après son « poussé ».

Passe, sur luciepc, bâti de `verrous-i1-4` (`27b90442c`, rebasée sur master `5c5feb527` : le lecteur
affamé d'hotfixs dedans, 67 étapes, lib et extension datées après le checkout) : banc de
concurrence en tenue exclusive conforme à `known_red.txt`, 9 rouges comparés, 246 verts (11 lignes
attendues) ; api 104, c_api 136, copy 23, stockage 83 + column_stats 2, vector 74 et 63, Cypher
1867 ; transaction_test 246 verts et **un rouge** : `OwnerlessPagesTest.TheReplayReadsNothingBeyondTheExtent`
(après la mort d'un COPY journalisé, rejeu + point de reprise : la colonne DOUBLE des dernières lignes
lit zéro). Seul ×2 vert, vert dans la liste précédente ; probabiliste (le fils finit son COPY puis se
tue : le hasard est ailleurs, l'ordre des fils du COPY ou l'étendue rendue à l'ouverture) ; pas
I1 (hors du mode, rien ne change) ; ticket ouvert
(`docs/tickets/2026-10-11-rejeu-d-un-copy-tue-la-colonne-score-des-dernieres-lignes-a-zero.md`),
hotfixs bissecte entre son `e4198f862` et son parent. **Fusionnée** sur ordre de l'orchestration :
master `3ad09c8cc` (avance rapide, `83fdf22f3`…`3ad09c8cc`, sur `8b098eb3b` ; le dernier rebase
n'apportait que du rag3weaver et des docs). Lib commune de luciepc rebâtie dessus, worktree
`rag3db-i1` effacé.

## La suite : I2 (la maintenance de l'index au commit)

Page 09 §0 et §3 : I2 n'ouvre qu'avec la mesure du banc sur l'état par instruction (ce que coûte la
fin d'instruction, et pourquoi) ; **la page avant le code est écrite** (3 h 30, page 10 :
`10-i2-la-maintenance-de-l-index-au-commit.md` — un état par transaction appliqué au commit sous
`INDEX` pris là ; l'index est l'état validé, comme pour les insertions ; ce que la mesure du banc
tranche : son essai `essai-fin-d-instruction-filtre`, `53c742c0b`, en cours sur luciepc). Tickets du
moteur encore à moi : C7 après un arrêt brutal, la seconde suppression d'une relation, la part
reprise du journal à doublon.

## Le COPY journalisé qui lisait des zéros (11 octobre 2026, 3 h 10 à 4 h 30) — priorité de l'orchestration

Relais de l'arbre principal (sa sonde `~/.cache/rag3weaver-notes-A/etendue/variantes.cpp` : un COPY
journalisé de 200 000 lignes, deux fils, `FLOAT[4]` à côté d'une `DOUBLE`, relu **sans mort ni
rejeu** : 134 à 285 scores à 0 en fin de table, 3 sur 3 ; juste à un fil, sans tableau, sans seuil,
ou forcé ; juste après un `CHECKPOINT` dans le même processus). Le rouge probabiliste
`OwnerlessPagesTest.TheReplayReadsNothingBeyondTheExtent` de ma liste d'I1 en était l'ombre au rejeu.

**Comment la cause a été trouvée** (worktree `rag3db-copie` sur luciepc, moteur instrumenté, sonde
rebâtie contre lui) : (1) reproduit 3 sur 4 ; les « 0.000000 » n'étaient pas toujours 0,0 à la
lecture par `WHERE d.score = 0.0` — c'était la carte de zones qui sautait le bloc, piège ; (2) une
relecture **par la clé** rend la bonne valeur là où le balayage lit 0 : les pages sont bonnes, la
lecture vectorisée est fausse ; (3) une sonde dans `ColumnChunk::scan` : les zéros sortent du **groupe
plein** écrit sur disque pendant le COPY (ON_DISK, 131 072 valeurs, **deux segments** 99 840 +
31 232), à la première valeur du second segment, sous une **sélection filtrée** (les versions de
l'insertion du COPY, `CHECK_VERSION`, tant qu'aucun point de reprise ne les efface) ; le second
segment porte des **exceptions ALP** aux positions 0..N-1, N = 69, 223, 241 = exactement le nombre de
lignes fausses ; (4) le code : `patchFloatExceptions` passe au filtre l'index absolu dans le vecteur
(`startOffsetInResult + position`) là où `DefaultColumnReadWriter::readCompressedValues` lui donne
l'index relatif au balayage — pour tout segment après le premier d'un bloc, rien n'est rapiécé, les
valeurs restent à leur encodage ALP, 0. Code de l'amont (mai 2025) ; rien vers l'amont.

**Correctif** : une ligne (`src/storage/table/column_reader_writer.cpp`, l'index relatif au filtre),
branche `copie-journalisee-double`. **Témoin** : la sonde en gtest
(`JournaledCopyReadBackTest.TwoThreadsAndAnArrayColumnReadEveryScoreBack`, trois COPY de suite, la
configuration de la sonde), rouge sur master (212 faux au premier essai), vert sur le correctif ; la
sonde elle-même 0 faux ×3 sur le bâti corrigé. Passe, sur luciepc (`rag3db-copie`, bâti de 3 h 52) :
transaction_test 248, api 104, c_api 136, copy 23, stockage 83 + column_stats 2, vector 74 et 63,
Cypher 1867 ; banc en tenue exclusive conforme sauf la course de témoin
`AnnouncedWritersOfOneIndexedTableWaitThenBothCommit` (`announce:done` avant `commit:done`), bornée à
`commit:start` par la ligne du banc, prise dans ce lot (`60d74a4e2`) ; relu par hotfixs (le banc
dormait). Rebasée sur master `fc636f7e6` (tests seulement) : `copie-journalisee-double-2`,
`10a796166` témoin, `eae3863ab` correctif, `60d74a4e2` borne ; OwnerlessPages et le témoin neuf
rejoués ×20 avant l'avance rapide.

Leçons : un `KU_ASSERT` muet en Release ne prouve rien, la sonde doit imprimer ; un bâti de cible
(`--target rag3db`) ne relie pas la lib partagée : prouver par la date et par `strings` ; relire par un
autre chemin (la clé) départage « pages fausses » et « lecture fausse » en une requête.
## La seconde écriture d'une relation (11 octobre 2026, 3 h 20) — le petit ticket d'A4′, fermé

Branche `verrous-rel2`, en attendant la mesure d'I2 (ordre de l'orchestration). A4′ contrôlait les
extrémités d'une relation, pas la relation : une relation vue par l'instantané puis supprimée ou mise
à jour et validée par une autre transaction recevait « Write-write conflict » à la place de l'erreur
de sérialisation nommée. `CSRNodeGroup::wasWrittenByCommitAfter` (la source et la ligne de
`findMatchingRow`), contrôlé par `RelTableData::update` et `delete_`, sous le mode seulement. Témoin
d'un fil rouge d'abord (trois refus mal nommés), puis vert ; une relation née après l'instantané
n'est ni vue ni refusée ; une écriture annulée ne refuse rien. Le cas à deux fils (l'attente puis
l'erreur) reste au banc dans sa forme `holderAndWaiter`, s'il le veut. Passe, sur luciepc
(`rag3db-rel2`), rejouée deux fois parce que master apportait du C++ entre-temps (le COPY corrigé,
puis le point de reprise interrompu d'hotfixs) : transaction_test 249, api 104, c_api 136, copy 23 ;
banc en tenue exclusive conforme (10 rouges / 248 verts, 5 h 09) — le premier banc avait montré la
course de témoin `AnnouncedWritersOfOneIndexedTable` (bornée à `commit:start` par le banc, prise
dans le lot du COPY). Sur master en avance rapide : `81f74f86c` témoin, `69f3ff53e` correctif
(5 h 10). Lib commune de luciepc rebâtie dessus, worktree effacé.

## Arrêt sur rapport propre (11 octobre 2026, 5 h 20) — la nuit du 10 au 11, et comment reprendre

Ordre de l'orchestration : I2 porte une règle visible (page 10 §1, « l'index est l'état validé »)
que Lucie peut renverser, et pèse deux à trois I1 — elle s'ouvre au matin sur son mot.

**Entré sur master cette nuit** (tout en avance rapide, listes complètes vertes, banc en tenue
exclusive conforme, relectures croisées) :

- A3′ (`21b6cb370`…`9a57a17b2`), A4′ (`9904fefc0`…`67ff3816b`), V2 (`95ced051e`…`11887dc21`),
  **I1** (`83fdf22f3`…`3ad09c8cc`) : la condition 1 de la stèle est à une marche (I2) de sa fin ;
- **la corruption du COPY journalisé** (`eae3863ab`, témoin `10a796166`) : un défaut de lecture
  de l'amont révélé par le COPY journalisé, trouvé en une heure par sonde — la condition 2 reste
  fermée ; le filet rag3weaver est retiré par l'arbre principal (`f9bf4574d`) ;
- **rel2** (`69f3ff53e`), A4′ pour les relations ;
- relus pour d'autres : le lecteur affamé (hotfixs, `e4198f862`), le point de reprise interrompu,
  les segments de chaînes bornés (hotfixs, poussés derrière moi) ; le contrôle de bâti des
  extensions (`458ff7157`) tient depuis V2.

**Le banc de concurrence** : de 56 lignes attendues rouges au départ des verrous à **10** après rel2
(248 verts). Ce qui reste rouge y est nommé ligne à ligne (`known_red.txt`).

**Ce qui reste, dans l'ordre** :

1. **I2**, la maintenance de l'index au commit : page 10 tranchée sur la mesure du banc (« pertes
   seules filtrée », l'état d'instruction devient l'état de transaction, bornes `commit:start`), avec
   la contrainte du pont coupé (§3 bis : une recherche par région détachée, à mesurer). Premier jet
   quand Lucie dit oui ; le banc réécrit ses témoins sur une branche partie de la mienne à ce
   moment-là — le lui dire.
2. C7 après un arrêt brutal (relations pendantes nées au rejeu, ticket ouvert).
3. La part reprise du journal à doublon (`LockBench.RecoveryOfAJournalWithADuplicateKey…`, fixture
   `test/transaction/journal_with_duplicate_key`).

**Les arbres** : ici, `rag3db-moteur` (branches `verrous-i1`, `verrous-rel2`, `copie-journalisee-double`,
toutes sur master, sans bâti) et `rag3db-docs-note` (`docs-i1`, poussée) ; sur luciepc, seul
`rag3db-lourd` reste, lib commune sur `3ed14644e` (librag3db.so 05:10:32 md5 `4598f15f67b1`,
libvector `4c12f0d4577a`) — un worktree neuf par lot, effacé à la fusion. Notes :
`~/.cache/rag3db-moteur-notes/{i1,rel2,copie}/` (scripts de passe détachés `passe*.sh`, journaux).
Rien ne tourne.

## La condition 3, les écritures parallèles (11 octobre 2026, matin) — la page, rien à coder

Décision de Lucie au réveil : I2 n'est pas bloquante (page 10 prête, I1 reste le produit) ; le
chantier est la condition 3, « oui jusqu'à B, et A d'abord ». Page 11
(`11-condition-3-les-ecritures-paralleles.md`, `1151b55b1`) : ce qui est (un refus sous un mutex,
l'état MVCC en mémoire par processus), A = allumer le mode et fermer T0, A7, A3′ reprise, C7, plus
A8 (le point de reprise sous écrivains permanents) et A9 (la mesure du coût, puis le mode par
défaut) ; B = le lecteur (fait), le relais au fichier (B2), le démon (B3, la voie Neo4j) ; le vrai
multi-processus est une refonte que je ne recommande pas ; quatre écarts rendus à Lucie. Rien à
coder avant son mot.

## T0, la transaction en échec garde ses verrous (11 octobre 2026, 10 h 30) — condition 3, marche 1

Branche `verrous-t0`. Ce qui existait : après l'échec d'une instruction dans `BEGIN … COMMIT`, la
connexion refusait déjà tout jusqu'au `ROLLBACK` (`throwIfManualTransactionFailed`), mais la
transaction était annulée **et fermée** en dessous — ses verrous rendus à l'échec ; celui qui
attendait derrière écrivait pendant que le bloc était encore ouvert côté client (rouge `waited` du
banc). T0 : la transaction reste active, ses effets annulés tout de suite
(`Transaction::failAfterStatement` : rollback, tampon d'annulation vidé — il ne s'applique qu'une
fois —, journal local vidé, drapeau `failed`), ses verrous tenus ; `ROLLBACK` la ferme ; `COMMIT` la
ferme en annulant et lève l'erreur nommée. **Écart choisi** : PostgreSQL répond au `COMMIT` d'un
bloc en échec par `ROLLBACK` sans erreur, Neo4j refuse le commit — Neo4j, pour qu'un client ne
croie pas avoir validé. **Conséquence en mode un seul écrivain** (relecture d'hotfixs) : un bloc en
échec non fermé garde la place de l'écrivain jusqu'au `ROLLBACK` — voulu, c'est ce que vaut une
transaction ouverte ; rag3weaver ferme déjà ses blocs en échec (`code_sync.rs`), dit à l'arbre
principal. `cleanUp()` du contexte appelé deux fois (à l'échec, à la fermeture) : il ne fait que
nettoyer les fichiers du contexte dans chaque système de fichiers, idempotent. Témoins : d'un fil
`AFailedTransactionKeepsItsLocksUntilItIsClosed` (rouge sur master : 0 verrou après l'échec, l'autre
n'attend pas ; vert après — la transaction garde aussi les verrous de l'instruction qui a échoué) ;
au banc `FailedTransactionReleasesItsLocksAtRollback` 5 sur 5, sa ligne sort de `known_red` (11
attendus). Passe, sur luciepc (`rag3db-t0`) : transaction_test 251, api 104, c_api 136, copy 23,
stockage 83 + column_stats 2 + string_split 2, vector 74 et 63, Cypher 1867 ; banc en tenue
exclusive conforme (9 rouges / 249 verts). Deux témoins existants adaptés à la règle
(`FailedTransactionTest` : un `BEGIN`/`CHECKPOINT` refusé dans un bloc en échec ne le ferme plus ;
en mode un seul écrivain, l'autre connexion lit et son écriture attend le `ROLLBACK`). Cas limite
nommé par hotfixs, laissé tel quel : un `ROLLBACK` qui lèverait lui-même dans un bloc en échec
laisse le bloc ouvert jusqu'à un second `ROLLBACK` ou la fin de la connexion — rien n'est perdu.
Sur master en avance rapide : `e963fb220`…`82a5f4851` (11 h 40).

## A3′ reprise, le journal à doublon (11 octobre 2026, 11 h 30 à 12 h 30) — condition 3, marche 3

Branche `verrous-a3-reprise`. Le rejeu butait sur « Found duplicated primary key value 7 » et la base
ne se rouvrait plus (journal fabriqué par le moteur d'avant le verrou de clé, sous le mode
multi-écrivains : deux transactions validées sur la même clé, mort base ouverte).

**Première forme, retirée** : sauter la transaction fautive jusqu'à son `COMMIT` et copier ses octets
à côté. Relecture d'hotfixs : les enregistrements du journal désignent les lignes **par décalage**
(mise à jour, suppression, extrémités des relations) ; sauter k insertions décale toutes les lignes
insertées après dans la même table, et une mise à jour ou une relation journalisée après tombe sur la
mauvaise ligne, sans erreur. Réponse fausse silencieuse — j'ai refait.

**La pierre tombale** : rien n'est sauté. Au rejeu, `NodeTable::insert` ne contrôle plus l'unicité
(le journal l'a fait quand il a été écrit) : la ligne prend son décalage ; au commit,
`PrimaryKeyIndex::commitInsert` note le doublon (décalage, clé) au lieu de lever, en RECOVERY
seulement — hors reprise, `DuplicatedPrimaryKeyException` (sous-classe de `RuntimeException`, même
message). À la fin du rejeu, `WALReplayer::settleReplayedDuplicates`, dans une transaction de
reprise (en mémoire en lecture seule, comme le reste) : si l'index mène à une **autre** ligne vivante
de la clé, la ligne rejouée est détachée de ses relations (toutes les tables de relations dont la
table est une extrémité, les deux sens) puis supprimée par décalage, l'index non touché (il ne la
connaît pas) ; sinon — la première ligne supprimée après coup — elle est gardée et reçoit son entrée
d'index (les trois cas d'hotfixs : décider à la fin, nommer ce que la pierre tombale emporte, la
lecture seule en mémoire). Chaque cas est nommé sur stderr (clé, table, décalage, côtés détachés).
Toute autre erreur de rejeu reste une corruption : la base ne s'ouvre pas.

**Témoins** : trois fixtures fabriquées par `ff9bad960` (worktree `rag3db-vieux` sur luciepc, la cible
`single_file_header` pour `rag3db.hpp`) dans `test/transaction/journal_with_duplicate_key/` : le
doublon seul ; B insère 7 et 8, C insère 9, la met à jour, la relie à 1 (9 porte sa mise à jour et
sa relation, par la clé comme par le balayage) ; comme la 2 puis D supprime la 7 de A (la 7 de B
survit, trouvée par la clé, mise à jour, relue après point de reprise). Rouges propres sur master,
verts après ; au banc `RecoveryOfAJournalWithADuplicateKeyKeepsTheDatabaseOpen` 3/3, sa ligne sort
de `known_red`. Pièges : `git commit -- <chemin>` refuse un fichier neuf non ajouté ; `git reset
--hard` vers un commit qui ne porte pas un fichier l'efface de l'arbre ; un `ASSERT` dans une aide
ne stoppe pas le test (plantage au lieu de rouge). Passe, sur luciepc (`rag3db-a3r`, rebasée sur
T0) : transaction_test 254, api 104, c_api 136, copy 23, stockage 83 + column_stats 2 +
string_split 2, vector 74 et 63, Cypher 1867 ; banc en tenue exclusive : 8 rouges / 250 verts, le
seul écart avec `known_red` étant la ligne qui verdit, sortie dans le commit final. Relu par
hotfixs (quatre points, dont la lecture seule : une transaction de reprise ne journalise jamais).
Sur master en avance rapide : `f8795e215` témoins, `b08396988` correctif, `b49423a49` known_red
(12 h 35). **Le banc est à 10 lignes rouges attendues.**

## C7 après arrêt brutal (11 octobre 2026, 12 h 40 à 13 h 30) — condition 3, marche 6 ; un défaut de rejeu à un seul écrivain

Branche `verrous-c7`. Méthode : un patch d'enquête sur luciepc (une trace du rejeu par enregistrement
sous `RAG3DB_TRACE_REPLAY`, le harnais gardant la base, le journal et le dump vivant avant le rejeu
sous `C7_KEEP`), `C7_RandomMix/Thread_Crash` rouge au premier essai ; puis un petit programme qui
rejoue la base gardée seule, et deux traces de plus (la liste CSR du nœud détaché, la visibilité
ligne à ligne). Lecture : la relation pendante (54, 95→60) est créée par la transaction juste avant
celle qui détache et supprime 60 ; le détachement du côté arrière de 60 voit deux relations visibles
dans le même lot (49 et 54) et n'en supprime qu'une. **Cause** :
`WALReplayer::replayRelDetachDeletionRecord` branchait les vecteurs voisin et relation sur l'état du
vecteur **source** (plat) ; le balayage y écrit la sélection du lot, et `RelTableData::delete_` lit le
source à la position de la relation — nulle au-delà de 0 — et rend faux en silence. L'exécuteur
vivant donne à voisin et relation un état à eux (`NodeTableDeleteInfo`) : la forme reprise. **Rejeu
seulement, un seul écrivain suffit** : un nœud à deux relations, `DETACH DELETE`, mort avant le
point de reprise — atteignable en service, condition 4, le ticket passe d'« inatteignable » à fermé.
Les trois pistes du ticket n'y étaient pour rien. Témoin `PendingRelAfterReplayTest` (deux relations
vers un nœud, détaché, mort, rejeu ; sous le mode aussi), rouge sur master, déterministe — un premier
témoin à UNE relation était vert : le motif demandait deux relations dans le même lot. Relecture
d'hotfixs : **le même défaut était dans ma pierre tombale** d'A3′ reprise (un seul état partagé) —
corrigé dans le même commit, avec une fixture 4 (la clé en doublon à deux relations par sens, toutes
retirées). Le ticket C7 est déplacé dans `closed/` par le commit qui corrige (règle de
l'orchestration), avec sa cause. Passe, sur luciepc (`rag3db-c7`, rebasée sur A7) : six témoins
rouges sur master puis verts, `C7_RandomMix/Thread_Crash` ×10 puis ×3 verts, transaction_test 257,
api 104, c_api 136, copy 23, stockage 83 + column_stats 2 + string_split 2, vector 74 et 63, Cypher
1867 ; banc en tenue exclusive : un premier passage où une ligne vectorielle du ticket des copies
passait au vert par hasard, un second conforme (6 rouges comparés / 252 verts, 8 lignes attendues).
Sur master en avance rapide : `edb411858` témoins, `2c7704b27` correctif (13 h 35). **Le banc est à
8 lignes rouges attendues** : A7 (hotfixs) et C7 sortis.

## A9, la mesure du coût du mode multi-écrivains (11 octobre 2026, 13 h 40 à 14 h 10) — condition 3, marche 4, première moitié

Branche `verrous-a9`, worktree `rag3db-a9` sur luciepc. Un témoin de mesure,
`test/transaction/concurrence/lock_cost_test.cpp` (`Mode/LockCost.*`, `TEST_P` sur le mode, sauté
sans `CONCURRENCE_MEASURE=1`), qui imprime chaque chiffre avec l'identifiant de bâti du moteur
mesuré (`common/build_id.h`, le dossier de bâti ajouté aux inclusions de `concurrence_test`).
Protocole et chiffres dans la page 11 §5 : sur le COPY de 200 000 lignes et les 100 000 insertions
ligne à ligne, aucun écart mesurable (moins d'un pour cent, sous le bruit) ; sur le SET sous index
HNSW, un bruit de ±30 % dans les deux modes, sans sens. **Le coût du mode est nul** : la condition
« plus de quelques pour cent » n'est pas atteinte, la clé imprimée reste. Avant la fusion : les six
cas sautés sans la variable, 18 verts avec, la comparaison du banc sur le binaire qui porte la
mesure. Lu avant le push : banc conforme (6 rouges comparés / 252 verts, 8 lignes attendues, 13 h 57 à 14 h 10), le rebase n'apportait aucun C++. Sur master en avance rapide : `87137a766` (14 h 12) ; `rag3db-a9` effacé. Suite : la bascule du défaut en lot à part, sur le mot
de Lucie (la page 11 dit la forme et le renversement).

## A9b, la bascule du défaut — bloquée par une permission de la session (11 octobre 2026, 14 h 30)

L'orchestration a rendu la règle de Lucie du matin : la bascule se fait sur le coût dit, et il est
nul. Le lot est prêt (worktree `rag3db-a9b` bâti sur luciepc, forme : `enable_multi_writes` sans
`debug_`, `true` par défaut dans `DBConfig`, `CALL enable_multi_writes=false` pour l'ancienne
sémantique, 18 fichiers de tests et `.test` à renommer, deux commentaires côté Rust), mais le
renommage en masse a été **refusé par le classifieur de permissions** de la session (« Modify
Shared Resources »), et la consigne du refus interdit d'atteindre le même résultat par un autre
outil. Rien n'a été modifié (arbre vérifié propre). Il faut la main de Lucie (une règle de
permission, ou le `sed` fait par elle sur la branche). Dit à l'orchestration et à hotfixs.

## Les bases gtest sous un parent par passe (11 octobre 2026, 14 h 35 à 15 h 55) — le ticket de rag3db-97

Branche `tests-tmp-par-passe`, worktree `rag3db-a9b` réemployé. Forme : `TestHelper::getRootTempDir()`
rend `<commun>/<pid du lanceur>` (pid pris à l'initialisation statique, avant tout fork, donc hérité
par les enfants du banc) ; `getSharedTempRoot()` est l'ancien `/tmp/rag3db` ; un `::testing::Environment`
global (`TempDirPerRun`, `test_helper.cpp`, enregistré à l'initialisation statique) efface le parent au
début et à la fin du lanceur (jamais depuis un enfant : `getpid()` comparé), et au début les parents
des passes dont le lanceur n'existe plus (`kill(pid, 0)`, noms numériques seulement). `cleanup_test`
efface le dossier commun. Témoin `HarnessMechanics.WhatAKilledChildLeavesLivesUnderTheRunsParent` :
un enfant fork ouvre une base puis `_exit` ; la base est sous le parent de la passe ; un parent nommé
du pid d'un enfant fini est effacé, le nôtre reste (témoin et forme dans le même commit : sans l'API
il ne compile pas). Relecture du banc (rag3db-10) : rien ne casse dans le harnais, le processus
neuf de `probeOpenInFreshProcess` se prend pour un lanceur sans dégât. **Ce qui reste à la main,
une fois** : les dossiers d'avant à la racine de `/tmp/rag3db`, quand tous les arbres sont rebâtis.
Passe sur luciepc (`rag3db-a9b`) : témoin vert (HarnessMechanics ×4), transaction_test 257, api 104, c_api 136, copy 23, stockage 83 + column_stats 2 + string_split 2, vector 74 et 63, Cypher 1867, banc conforme (6 rouges comparés / 253 verts : le témoin neuf en plus) ; après chaque suite, aucun parent numérique laissé dans `/tmp/rag3db` (les entrées non numériques qui y sont apparues pendant la passe venaient de la liste d'A8 d'hotfixs, jouée en même temps avec un binaire d'avant). Sur master en avance rapide : `dd9413d5e` (15 h 58 ; le rebase n'apportait que du code de test, le lecteur du journal du banc). `rag3db-a9b` effacé ; la lib commune de luciepc n'a pas à être rebâtie (aucun C++ du moteur).

## A9b, la bascule du défaut (11 octobre 2026, 16 h 10 à PASSE_A9B_FIN) — condition 3, marche 4, seconde moitié

Lucie a levé la permission en séance (« vas-y je t'autorise explicitement »). Branche `verrous-a9b`,
worktree `rag3db-a9b` refait sur luciepc. Forme : `EnableMVCCSetting::name` = `enable_multi_writes`
(plus de `debug_`), `DBConfig::enableMultiWrites{true}`, commentaire du réglage ; les 18 fichiers de
tests et `.test` qui le nommaient renommés ; les témoins de l'ancienne sémantique la posent par
`CALL enable_multi_writes=false` — `ddl_serial_execution.test` (le refus du second BEGIN),
`FailedTransactionTest.AutoCommitAndOtherConnectionsAreNotAffected` (première moitié), et, sur
relecture du banc (page 05 §4 du banc), les trois `InsertLockTest.WithoutMultiWrites…` qui
comptaient sur le défaut sans le poser (deux auraient rougi, le troisième aurait testé le mode sous
un nom qui dit le contraire). Deux commentaires Rust (`daemon/db.rs`, `e2e_prise_atomique.rs`)
disent le défaut neuf. Page 11 : la phrase de Lucie et la mesure qui déclenche. Commits
`f1db9d9aa` (la bascule), `63e6a26bf` (les trois témoins). Preuves avant le hash : ma liste
complète et le banc (PASSE_A9B), le protocole référence/branche du banc (BANC_A9B), les e2e du
produit qui ouvrent un backend rejoués par l'arbre principal sur son bâti de la branche (E2E_A9B).

**Ce que la liste a fait sortir (16 h 50)** : sur 63e6a26bf, tout vert sauf (1) un `.test` de
l'extension vector qui nommait encore l'ancien réglage — le renommage n'avait pris que `src` et
`test` (`1a34a4315`) ; (2) **cinq tests Cypher `ddl`/`rel_group` rouges, un vrai défaut de la
bascule** : `CREATE NODE TABLE Person2 AS LOAD FROM …` → « Person2 does not exist in catalog ».
Cause : depuis A3′, sous le mode, `NodeBatchInsert::acquireLocksBeforeExecution` prend le verrou
d'index de sa table **avant l'exécution du plan** (sur le fil du client, pour ne pas geler
l'ordonnanceur) — pour `CREATE TABLE … AS`, la table est créée par le même plan, elle n'est pas
encore au catalogue. Forme : si la table n'est pas au catalogue à ce moment, pas de prise ; la
prise se fait dans `initGlobalStateInternal` (la reprise d'un verrou tenu est sans effet), et
pour une table créée par le plan elle n'attend jamais (identifiant neuf, personne d'autre ne la
voit). Témoin `InsertLockTest.CreateTableAsTakesTheIndexOfTheTableItCreates` (`ebf81a632`),
correctif `6a4ebaa89`. Le banc et l'arbre principal prévenus : leurs passes sur 63e6a26bf valent
première passe, la preuve est sur la tête finale. Seconde passe sur luciepc : bâti, témoins,
transaction_test, copy, vector, Cypher, banc — **tout vert** (17 h 05, luciepc) : témoins 42, transaction_test 260, copy 23, vector 74 et 63, Cypher 1867, banc conforme 6 rouges / 253 verts. Tête finale `6a4ebaa89`, dite au banc et à l'arbre principal pour leurs passes de preuve.

**17 h 45 — A8 sur master, la bascule rebasée.** A8 d'hotfixs est sur master (f024578d5, lib
commune 481afb550) ; la bascule rebasée dessus sans conflit sous un nom neuf, `verrous-a9b-2`,
tête `4ee70e386` ; rejeu sur luciepc dans `rag3db-a9b` (bâti 66 étapes, témoins 42 verts, puis
transaction_test, ddl, banc). Le banc et l'arbre principal rejouent leurs preuves sur cette tête
(leur première passe sur 6a4ebaa89 : banc transaction 260 et e2e 27 verts ; arbre principal 116
e2e du produit verts, aucune attente ni erreur de sérialisation, artefact prouvé par md5).
**Trois rouges 10/10 sur master, hors bascule, relevés par le banc** (probabilistes rejoués dix
fois sur fd10b8c27) : H4_IndexedRandomMix Hot/Crash — « HNSW index: node offset 310 is beyond the
visited set » et « Row 314 of table Doc has no primary key to lock: it is not visible to this
transaction » (ce dernier est un message de mes verrous : une ligne supprimée et validée par un
autre écrivain entre le MATCH et le SET devrait donner l'erreur nommée de sérialisation, pas un
refus inattendu — à prendre après la bascule) ; JournaledCopyDeath `data-file-bounded` (le
fichier de données grossit à chaque mort au milieu d'un COPY). Tickets par le banc.

**17 h 50 — rejeu sur 4ee70e386 (bascule + A8), luciepc** : bâti 66 étapes, témoins 42, transaction_test 260, e2e ddl 72, banc conforme 6 rouges / 254 verts (le témoin d'A8 en plus). L'arbre principal « fini » : 116 e2e du produit verts sur cette tête, artefact prouvé (md5 par ldd), aucune attente ni erreur de sérialisation, durées comparables. Reste le « fini » du banc (protocole entier sur 4ee70e386 contre master 481afb550), puis `poste push`.

**18 h 15 — la bascule est sur master, `a0a79016e`.** Le banc « fini » : son protocole sur 4ee70e386
contre master 481afb550 ne montre aucun écart (lib/extension md5 distincts ; transaction_test 259 /
260 — le témoin CreateTableAs en plus ; e2e de concurrence 27/27 ; probabilistes ×10 : 70 verts
30 rouges des deux côtés, les trois mêmes cas de master ; banc conforme 6 rouges / 254 verts des
deux côtés, les six mêmes). Push par `poste push` : premier tour refusé par ma faute (la branche
rebasée poussée sous son ancien nom, non-fast-forward, rien poussé — le script pousse désormais
sous un nom neuf à chaque tour) ; second tour : rebase sur 3f13b1657 (rien du moteur), bâti chaud
33 étapes, témoins 42, e2e ddl 72, banc 6/254, master immobile, push. Trois contrôles lus.
**Le mode multi-écrivains est le défaut du moteur** ; `CALL enable_multi_writes=false` rend
l'ancienne sémantique. Lib commune de luciepc rebâtie dessus, `rag3db-a9b` effacé. Reste à moi :
B2, et le ticket de mes verrous relevé par le banc (ligne supprimée par un autre écrivain entre
le MATCH et le SET → l'erreur nommée de sérialisation, pas un refus inattendu).
