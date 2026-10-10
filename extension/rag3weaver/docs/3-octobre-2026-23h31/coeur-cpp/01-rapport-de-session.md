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

## A3′ en cours (10 octobre 2026, 19 h) — état au nettoyage du disque

Interrompue par l'orchestration (le disque sature, tous les bâtis vont être effacés) ; tout est
poussé en branche, **rien n'est fusionné**.

- **Code** : branche `verrous-a3` de rag3db-moteur (un commit sur `ff9bad960`), message complet
  dans le commit. Ce qu'elle fait : verrou de clé à l'insertion (index partagé + clé exclusive),
  verrou d'index exclusif du COPY pris sur le fil du client avant l'ordonnancement, unicité
  contre le dernier état validé avec **une seule** visibilité (insertion et validation), fil de
  remplacement de l'ordonnanceur pendant une attente de verrou (`TaskScheduler::BlockingWait`),
  témoin d'un fil `insert_lock_test.cpp` (7 cas), journal à doublon d'avant A3′ gardé au dépôt
  (`test/transaction/journal_with_duplicate_key`), deux témoins du banc réécrits
  (`RollbackOfACopy` à deux fils, `RecoveryOfAJournalWithADuplicateKey…` sur le journal gardé).
  Page 07 §5 : ce que le code a appris.
- **Joué** : sur le dernier bâti, transaction_test (InsertLockTest + LockManagerTest) 25 verts ;
  sur le bâti d'avant le fil de remplacement, C1 ×6, C1_SnapshotPredatesCommit ×3,
  LockBench.SameKey ×2 verts, plus de SIGSEGV dans C7. **Pas joué** : le filtre A3′ du banc et
  C7 ×3 sur le dernier bâti (chaîne tuée au nettoyage), la liste complète, la relecture du banc,
  la mesure du coût par ligne (rag3db-6f).
- **À faire à la reprise** : rebâtir (`build/moteur` effacé) ; rejouer `LockBench.SameKey*:
  *C1_*:*RecoveryOfAJournalWithADuplicateKey*:*RollbackOfACopy*` puis `*C7_RandomMix*` ×3
  (attendu : RollbackOfACopy ×2 verts, plus de garde expirée ; C7 Hot/Reopen restent
  probabilistes, Crash attendu vert) ; appliquer le script de `known_red.txt`
  (`~/.cache/rag3db-moteur-notes/a3/known_red.py` : retire C1 ×6, Snapshot ×3, C7 Crash,
  SameKey ×2, RollbackOfACopy ; garde la ligne de la reprise) — si C7 Crash ne verdit pas
  toujours, `probabilistic.txt` ; liste complète ; relecture du banc ; push en avance rapide
  avec le compte des rouges (59 avant A3′) ; fermer le ticket « L'annulation d'un COPY efface
  les lignes… » (corrigé par le verrou) et le message de commit « Passe : INCOMPLÈTE » se
  corrige par un second commit, pas par un amend poussé.
- **Faute du jour** : un `pidof concurrence_test` nu a tué un test du banc ; règle notée (pids
  lancés par soi, vérifiés par `/proc/<pid>/exe`).
