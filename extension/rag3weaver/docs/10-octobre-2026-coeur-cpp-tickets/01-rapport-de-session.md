# Cœur C++, correctifs et tickets — rapport de session

Seconde session cœur C++, ouverte le 10 octobre 2026 : les correctifs et les tickets du moteur,
pour que la session « coeur c++ » reste sur la stèle. Elle ne touche pas `src/transaction/` ni
`src/storage/` (journal, tampon, verrous) sans demander à « coeur c++ » ; elle ne corrige dans
`extension/vector/` qu'avec l'accord du banc.
**Dernière mise à jour : 11 octobre 2026, 0 h 30 — la nuit du 10 au 11 : quatre lots dans master (analyze, IGNORE_ERRORS, B8 fermé, contrôle de bâti), trois tickets requalifiés.**

## L'arbre (depuis le 10 octobre au soir)

- **Le cœur C++ travaille sur luciepc** (décision de Lucie) : un worktree par lot,
  `~/git_workspaces/rag3db-tickets` là-bas (`git -C ~/git_workspaces/rag3db worktree add …`),
  bâti Release `vector;geo` sous SON `poste lourd`, `-j 8` ; ici, un worktree du même nom pour écrire
  et pousser. **Après chaque fusion, worktree et build effacés** des deux côtés (règle de Lucie),
  après vérification par `/proc/<pid>/cwd`. Les notes vivent hors des worktrees :
  `~/.cache/rag3db-tickets-notes` (sondes jetables, script de liste, journaux), sur les deux postes.
- **La liste complète** : `~/.cache/rag3db-tickets-notes/liste-cpp.sh` sur luciepc, qui prend le
  verrou étape par étape — bâti et suites en `poste lourd`, la comparaison du banc en `poste mesure`
  (annoncée aux autres sessions), un `timeout` par étape ; ne pas l'envelopper dans un `poste`.
  Lancée par `~/git_workspaces/rag3db-tickets-liste.sh`, codes dans `…/liste/codes`. Une fois à la
  fusion ; pendant le travail, les tests du changement.
- **Qui pousse du C++ sur master rebâtit la lib commune de luciepc** (`rag3db-lourd`,
  `cmake --build build/lecteurs-csv -j 8`, lib et extension ensemble) et annonce la tête.

Les noms des sessions (10 oct. au soir) : orchestration rag3db-c7, cœur C++ (stèle) « coeur c++ »
(rag3db-91 un temps), banc rag3db-10, arbre principal rag3db-73, paquet npm rag3db-90.

## Les lots, dans l'ordre de l'orchestration

1. (a) `CopyTest.RelCopyBMExceptionRecoverySameConnection` — **fait**, dans master (`33c3d42de`, puis `fe7f95fdd` qui pose les deux chemins explicitement après le basculement `ff9bad960`) : le test réglé sur la taille du COPY, joué aussi journalisé, et un témoin neuf ; `copy_tests` entier vert (23), les 17 tests de `transaction_test` qui utilisent la doublure verts.
2. (b) Le plantage HNSW sur des centaines de vecteurs identiques : une lecture de rag3db-9c
   (« risque de plantage »), jamais vu. Le lot commence par la recette nue : le ticket ne
   s'écrit que si elle rougit. Accord du banc (rag3db-77) pour `extension/vector/`.
   **Confié au banc** (10 oct.) : il a déjà écrit la recette en témoin, `SameVectorForEveryRow`
   (`vector_index_update_test.cpp`, branche `banc-elagage-en-cours`, huit cas, 1 402 lignes en
   `FLOAT[384]`, cosinus), et la joue sur les deux règles d'élagage ; il me passe le résultat,
   et le ticket est pour moi s'il rougit hors de l'élagage.
   **Dissipé** (résultat du banc, 10 oct.) : aucun plantage, sur les deux règles et dans les huit
   cas ; le cosinus de simsimd rend 0 pour deux normes nulles, la recherche rend ses 10 résultats.
   Reste un défaut d'élagage, des lignes injoignables (master : 1 190 à 1 298 joignables sur
   1 402, nul ou non ; règle classique : 1 402 index après, 942 à 1 357 index avant) : chantier du
   banc, ticket avec son lot. Pas de ticket de plantage.
3. (c) Les comptes de distincts qui ne reculent pas (`IGNORE_ERRORS`, `DELETE`, `UPDATE`) ;
   puis, si la cause est la même, le ticket des relations annulées (lot à part).
   **Fait** : le défaut de sélection de `ColumnStats::update` (`d10b92306`) puis `CALL analyze` et
   le recalage de la cardinalité sur les lignes vivantes (`dff1adddc`, `9c528349b` ; page 03).
4. (d) Les tickets confort du moteur, le plus petit d'abord — la nuit du 10 au 11, ci-dessous.

## Lot (a) — la preuve par le code (lignes à `d2bd94630`), avant l'exécution

1. **Le seuil** : `resetDBFlaky` pose le tampon de `SystemConfig{}` (`copy_test.cpp:73`) ; le
   défaut `-1u` (`database.h:66`) devient 0,8 × la mémoire physique (`database.cpp:56-76`,
   `constants.h:55`) : 104 359 880 294 octets sur ce poste (31 848 108 pages de 4 Kio) ;
   `copyJournalThreshold` = min(tampon / 8, 256 Mio) = **268 435 456** (`transaction.cpp:63-72`).
2. **Le journal** : 3 colonnes `INTERNAL_ID` (2 + la propriété `_ID`, `partitioner.cpp:159-161`,
   `:176`) ; par cellule 1 (nullité, `value_vector.cpp:424`) + 3 (type, `types.cpp:749-755`)
   + 1 (`isNull`) + 4 (`childrenSize`) + 16 (l'identifiant) = 25 octets ; **75 par relation**,
   le chiffre mesuré le 5 octobre ; × 2 420 766 = 181 557 450 octets, sous le seuil : pas de
   repli (`partitioner.cpp:223`).
3. **Les réservations** : `InMemFileWriter::write` prend une page de 4 096 octets par
   `mm.allocateBuffer` (`in_mem_file_writer.cpp:15`, `:22`, `:74-76`) ; hors `TEMP_PAGE_SIZE`,
   c'est `mallocBuffer` → `bm->reserve` (`memory_manager.cpp:54-57`, `:70-72`), qui lève le
   message attendu par le test. ≈ **44 326 réservations par essai**.
4. **La doublure** : refus quand `(reserveCount + 1) % failureFrequency == 0`, compteur cumulé
   entre essais, fréquence doublée après un refus mais remise à `512 × (i + 15)` à chaque essai
   (`copy_test.cpp:271`) : au plus **17 408**. Les réservations du journal sont en exécution
   (`Partitioner::executeInternal`), phase où le test permet les refus.

Donc chaque essai rencontre au moins un refus, et `ASSERT_LT(i, 20)` tombe. Au défaut
d'aujourd'hui le `COPY` forcé ne journalise pas (`partitioner.cpp:221`) : le test passe.

**Mesuré** (10 oct., binaire sur `d2bd94630`, doublure instrumentée —
`~/.cache/rag3db-tickets-notes/mesure-relcopy.patch`, journaux `../relcopy-*.log`) :
- au défaut : vert au 5ᵉ essai ; 966 à 1 270 réservations de 4 Kio par essai (aucune du journal),
  ~7 000 autres, un refus par essai ;
- `COPY` journalisé (`CALL force_checkpoint_on_copy=false` sur la connexion du test) : rouge,
  `copy_test.cpp:113`, « (i) < (20), actual: 20 vs 20 » ; vingt essais, chacun deux refus dont un
  ou deux sur une page de 4 Kio ; 19 986 à 45 155 réservations de 4 Kio (le `COPY` s'arrête au
  refus et va plus loin quand la fréquence s'élargit) ; fréquence 7 680 à 17 408 ; 0 repli ;
- coût des réservations du journal : 0,7 à 1,8 ms par essai, ~40 ns la page (page 05, §5 ter) ;
- l'assertion de cohérence d'après la boucle n'est jamais atteinte : la propreté de l'annulation
  après un refus pendant la journalisation n'est pas prouvée.

Options rendues à l'orchestration ; (1b) d'abord retenue — la fréquence ne recule plus entre
essais — puis **écartée par la mesure** : au défaut, la fréquence doublée (15 360) dépassait les
~9 700 réservations d'un `COPY`, et il ne restait qu'un refus, dans le partitionnement, là où
l'ancienne formule en mettait deux dans l'insertion en masse (piles résolues par `addr2line`).

**Retenu, (1c)** (accord de l'orchestration) : le test compte les réservations d'un `COPY` entier
sans refus dans `BEGIN … ROLLBACK` (R = 9 456 au défaut, 53 869 journalisé), refuse la
réservation R − 1 024, − 768, − 512, − 256 aux quatre premiers essais, passe au cinquième, et
exige qu'au moins un refus tombe dans l'insertion en masse (toutes les identités de relations
réservées, `RelTable::reserveRelOffsets(0)`). Mesuré : dans les deux modes, quatre refus, tous
dans l'insertion en masse, vert au 5ᵉ essai. L'assertion vue rouge avec un pas de 2 000.

**Le témoin neuf** (`RelCopyRefusedOnAPageRollsBack`, `JournaledRelCopyRefusedOnAJournalPageRollsBack`) :
vert dans les deux modes. Journalisé, le refus tombe dans le journal (pile) ; au défaut, sur
l'épinglage d'une page de l'index de clé primaire. 0 relation, réouverture, `COPY` suivant
complet, aucune page perdue. Piège du harnais trouvé en chemin : rouvrir avec le tampon des tests
(≈ 73 Mio) fait échouer le `COPY` suivant dans les deux modes ; il rouvre au tampon par défaut.

Ce qu'avait écrit la première lecture, **[déduit]**, gardé pour mémoire :
- le test ne tourne pas sous un tampon minuscule : `resetDBFlaky` remet le tampon par défaut
  (`test/copy/copy_test.cpp`, `resetDBFlaky`) ; le seuil du repli vaut donc probablement son
  plafond, 256 Mio (`Transaction::copyJournalThreshold`) ;
- le journal des 2 420 766 arêtes de twitter, à ~75 octets la relation, pèse ~180 Mio : sous le
  seuil, pas de repli ;
- la doublure refuse une réservation sur `failureFrequency`, que le test remet à `512 × (i + 15)`
  à chaque essai (au plus ~17 400) ; le journal local prend ses pages de 4 Kio par réservation
  (`InMemFileWriter`, `mm.allocateBuffer`), ~44 000 de plus par essai : chaque essai croise au
  moins un refus.

Si la mesure le confirme, c'est l'hypothèse du test (un nombre borné de réservations pendant le
`COPY`) qui ne tient plus sous le `COPY` journalisé, pas le repli. L'option que préfère l'orchestration : que le test dise ce
qu'il mesure (un nombre borné de refus par essai, journal compris), sans changer le moteur. À
relever aussi pour la page 05 : ce que coûtent en temps les réservations une à une des pages du
journal local.

Premier geste : sous-modules, configuration, rebâti de `copy_tests` annoncé à « coeur c++ » et
au banc ; le test seul au défaut puis défaut basculé (`sed`, jamais commité), avec
`RAG3DB_PROFILE_JOURNAL=1`, `copy_journal_fallbacks`, et un compteur provisoire des réservations
et des refus par essai (en patch).

## Lot (c) — préparé par lecture (repérage par agent, lignes revérifiées), rien exécuté

- **Un compte de distincts ne recule pas, par construction** : `HyperLogLog::update` garde le
  maximum d'un registre (`hyperloglog.h:33`), `merge` aussi ; aucune opération de retrait. Le
  code le dit (`local_storage.h:47-48`).
- **`IGNORE_ERRORS`** : les lignes entrent aux statistiques à l'ajout
  (`node_batch_insert.cpp:180`), avant l'index de clé qui les refuse ; elles sont retirées
  par `delete_` (`node_batch_insert_error_handler.cpp:34`), qui ne touche pas aux statistiques ;
  elles partent avec les statistiques du `COPY` vers la transaction (`:296-300`).
- **`DELETE` et `SET`** ne touchent jamais `TableStats` (`NodeTable::delete_`, `update`) : après
  un `SET` les valeurs neuves ne sont pas comptées non plus (sous-estimation possible).
- **Le recalage** ne vaut que pour la cardinalité (`node_group_collection.cpp:217`, `:290`) ;
  rien ne recalcule un HyperLogLog depuis les données. Le commentaire de `:214-216` (« il fusionne
  ses statistiques avant sa validation ») est périmé depuis `1177f5794`.
- **Qui lit les distincts** : le planificateur seul, pour une égalité sur une propriété hors clé
  (`cardinality_estimator.cpp:167`, `:180-183`), et `STATS_INFO` (`stats_info.cpp:43`). Au pire
  un plan moins bon.
- Options à rendre après le lot (a) : écarter les lignes refusées à la source (n'ajouter aux
  statistiques qu'après le contrôle de la clé) ; recalculer les HyperLogLog au point de reprise
  (coût : hacher toutes les colonnes) ; ou dire l'estimation et s'en tenir là. Les relations
  n'ont aucune statistique tenue (`nextRelOffset` seul, qui est aussi l'allocateur
  d'identités) : la cause du ticket des relations est autre.

## Lot (c) — l'avancement (10 octobre, soir)

- Décision de l'orchestration : (A), `CALL analyze('Table')` explicite, un balayage, sans
  déclenchement automatique dans ce lot ; page `03-le-recalcul-des-statistiques.md`, acceptée.
- **Un défaut trouvé en chemin, corrigé** (`b180185c9`) : `ColumnStats::update` relisait les
  hachages aux premières cases sous une sélection filtrée — rouge « estimation 1 pour 1000 »,
  vert après ; relu par la session cœur C++. Les nulles comptées comme une valeur : noté au
  ticket, sans correctif.
- Accords de la session cœur C++ pour `src/storage` : le commentaire périmé et le recalage de la
  cardinalité sur les lignes vivantes, à deux conditions (ne jamais toucher `numTotalRows`, ne pas
  rebalayer au point de reprise).
- Reste : les témoins rouges de `STATS_INFO` après `IGNORE_ERRORS`, `DELETE`, `SET` ; `analyze` ;
  la réouverture ; la mesure de 100 000 lignes sous `poste mesure`.

## Arrêt du 10 octobre au soir (nettoyage du disque, demandé par Lucie)

- **Poussé en branche `statistiques`, non fusionné** : `CALL analyze('Table')`, le recalage de la
  cardinalité sur les lignes vivantes (point de reprise, ouverture), `NodeGroup::getNumLiveRows`,
  `NodeTable::replaceStats` (accords de la session cœur C++ pour `src/storage`), les témoins
  `TableAnalyzeTest`, deux tickets (`analyze` non transactionnel ; l'ouverture sans une extension
  dont une table dépend), la page 03 à jour.
- **Joué avant l'arrêt** : les quatre premiers témoins `TableAnalyzeTest`, rouges avant et verts
  après ; le témoin du recalage vu rouge sur l'ancien recalage ; la liste complète verte
  (transaction_test 217, api 104, c_api 136, copy 23, stockage, column_stats 2, banc conforme à
  `known_red` 59/181, vector 74 et 63, e2e 1867).
- **Pas joué** : le témoin du ROLLBACK (`AnAnalyzeInARolledBackTransactionLeavesAStaleEstimate`,
  écrit après la liste) ; la mesure des 100 000 lignes (arrêtée avant d'avoir eu le verrou ;
  l'essai est gardé dans `~/.cache/rag3db-tickets-notes/analyze_measure_scratch_test.cpp`, qui
  n'est pas dans un dossier effacé — à vérifier à la reprise).
- **Pas fait** : montrer le diff de `src/storage` à la session cœur C++ ; la fusion.
- **À la reprise** : un worktree neuf sur `origin/statistiques` (le mien est effacé), `cmake`
  Release avec `vector;geo`, bâti sous `poste lourd`, le témoin du ROLLBACK, la mesure sous
  `poste mesure`, le diff à la session cœur C++, rebase et avance rapide.

## Reprise sur luciepc (10 octobre, soir)

- Décision de Lucie : le cœur C++ travaille sur luciepc. Worktree `~/git_workspaces/rag3db-tickets`
  là-bas, branche `statistiques-2` (la branche reposée sur master, poussée sous un nom neuf) ;
  bâti complet en 3 min 30 (lib et binaires à 19 h 52, après le commit de 19 h 00).
- Joué là-bas : les cinq `TableAnalyzeTest`, le témoin du ROLLBACK compris (la cardinalité gardée
  vaut bien 1 000, ce que l'analyze annulé avait vu) ; la liste complète, verte sauf un rouge
  probabiliste étranger au lot (`ForcedTransactionJournalTest.OrdinaryWritesBeforeACopyThatSkipsRowsCommittedThenDead`,
  2 999 lignes pour 3 000, vu aussi par la session cœur C++ sans ce lot : ticket à écrire).
- Relecture de la session cœur C++ sur `src/storage` : d'accord, une demande — le recalage limité
  aux groupes de nœuds (un groupe CSR garde sa part persistée hors de ses blocs) : `a1bd6c8a6`,
  bâti et joué (TableAnalyze et CopyStatistics 10, rel_tests 7).
- La mesure (page 03, §5 ter) : `analyze` 4 ms sur 100 000 lignes clé + chaîne, 24 ms avec un
  `FLOAT[768]` (95 ms à froid). Deux essais nuls d'abord, dits.
- Piège de luciepc : un `scp` n'a pas gardé le droit d'exécution d'un script ; les binaires de test
  se lient à `librag3db.a`, pas au `.so` (prouver la date de la statique).

## La nuit du 10 au 11 octobre

| Quoi | Commits | État |
|---|---|---|
| `IGNORE_ERRORS` : un doublon du même COPY faisait supprimer une ligne innocente quand l'index de clé avait des entrées sur disque (perte, condition 4) | témoin `02335d429`, correctif `599f0494b` (`HashIndex::appendNoLock` rend la position du premier échec), test du doublon `721ed2898` | **corrigé**, fermé ; le choix de la ligne gardée (ordre des fils) en ticket confort |
| `CALL analyze` (après la nuit précédente) | `dff1adddc` … `5798567dc` | dans master ; le déclenchement automatique écarté sur une sonde (ticket « analyze au seuil ») ; la précision de l'HyperLogLog en ticket |
| Un `NULL` en tête d'une liste de paramètres type sa colonne en `STRING` | ticket relu `018868459` | la cause est dans rag3weaver (`cypher_to_rag3db_value`), pas le moteur ; à l'arbre principal |
| L'estimation des relations gonflée | ticket relu `018868459` | ni l'annulation ni un `DELETE` ne la reculent ; rien à coder avant un mauvais plan |
| Le RTree de geo | ticket relu `16ff4fb6d` | injoignable : l'index spatial ne se crée même pas (paramètre `LIST` sans `inferInputTypes`) ; laissé pour plus tard, geo n'a pas d'utilisateur |
| B8, le tampon de 256 Mio plein à la première écriture | fermé `c8cec6086` | non reproduit sur master, ni sur luciepc (24 fils) ni sur le poste principal (32 fils, la lib de la passe rouge) ; deux différences restantes nommées |
| L'extension chargée sans contrôle de bâti | témoin `2b6515a87`, correctif `458ff7157`, témoin du rejeu `1e2d9fcd3`, fermeture `97acd579b` | **corrigé** : identifiant de bâti régénéré à chaque bâti, refus nommé ; toute extension d'avant est refusée |

### Ce que la nuit a appris

- **La preuve par les lignes d'abord, puis l'exécution, et dire quand l'exécution contredit** :
  deux pistes de B8 lues puis réfutées par une sonde (36 cas verts) ; le déclenchement d'`analyze`
  après un COPY écarté par une sonde (la cardinalité était exacte).
- **Une sonde se vérifie à sa première étape** : la première mesure d'`analyze` mesurait une table
  vide (une compréhension de liste refusée par le dialecte), la seconde est restée vingt minutes
  sur le `COPY` d'un CSV de 400 Mo ; la troisième, minutée par étape après une sonde de 2 000
  lignes, a rendu ses chiffres.
- **Un témoin rouge peut l'être pour une autre raison que le défaut** : le témoin du rejeu du
  contrôle de bâti cherchait l'index sans recharger l'extension de ce bâti.
- **Les binaires de test se lient à `librag3db.a`** : en prouver la date, pas celle du `.so`.
- **Faute** : un `--force-with-lease` joint au push de master, sur `statistiques-3` (rien de perdu,
  ancienne tête republiée sous `statistiques-3-avant-rebase`). Après un push de master, rien
  d'autre dans la même commande.

### Pour reprendre

Les tickets confort du moteur qui restent sont surtout des décisions ou du stockage de la session
cœur C++ (le journal sans borne hors COPY, le lecteur affamé, le point de reprise sous un petit
tampon) ; geo attend un utilisateur. Rien ne tourne de mon côté, aucun worktree ouvert hors celui
des docs, effacé après ce push.

## Le lecteur affamé (11 octobre, nuit)

Ticket `2026-10-04-lecteur-affame-par-les-points-de-reprise`, confié par l'orchestration au
service de l'indexation à plusieurs sessions. Trois formes proposées (une reprise dans le
moteur, un verrou à la SQLite, la marche 5) ; la forme (2) choisie par l'orchestration, la
fenêtre étroite par la session cœur C++.

- **Le verrou des lecteurs** (`ReadersLock`, `<base>.readers`, flock ou LockFileEx) : partagé
  pour l'ouverture en lecture seule (5 s au plus, puis un refus nommé), exclusif pour le point
  de reprise, de la marque CHECKPOINT au journal supprimé (1 s au plus, puis il passe et le dit
  sur la sortie d'erreur). Le constat d'après coup reste le filet.
- **Rouge puis vert**, sur luciepc :
  - `ReadOnlyReader.SingleOpenIsNotRefusedByBackToBackCheckpoints` : 34 refus sur 80 sur
    master, 0 après ;
  - le lecteur insistant : 0 refus à 0,2 ms comme sans pause ;
  - les trois témoins des bornes dans `ReadOnlyOpenTest`.
- **Écarts acceptés par l'orchestration** :
  - le point de reprise ne compte pas les ouvertures qu'il attend (flock ne le permet pas) ;
  - une ouverture plus longue que 1 s reste refusée par le filet : c'est la marche 5.
- **Faute de méthode** : mes `timeout` enveloppaient `poste`, et comptaient donc l'attente du
  verrou. Une mesure tenait le poste, la configuration a expiré, la passe rouge n'a rien
  prouvé, et la passe enchaînée a lu le FIN de l'ancien journal pour démarrer trop tôt. Les
  bornes vont maintenant DANS `poste` (`poste lourd timeout N …`), liste comprise. Un
  enchaînement se fait dans un seul script, jamais par un guetteur qui lit un journal pas encore
  vidé.

## Ce que j'ai lu en arrivant, et ce qui m'a manqué

Lu en entier : `README.md`, `BUILD.md`, le relevé du 2 octobre
(`2-octobre-2026-01h07/moteur-concurrence/02-knowledge-dump.md`), le relevé et le rapport
cœur C++ (`3-octobre-2026-23h31/coeur-cpp/01`, `02`), ses pages 04, 05, 06, la stèle, le
README des tickets et son index, le ticket de la cardinalité, le plan de reprise du 8 octobre.
Du journal des chantiers (1 724 lignes) : l'en-tête et les titres du §6 seulement.

Ce qui m'a manqué :
- la copie du rapport cœur C++ dans l'arbre principal (`filtre-impact-fusion`) n'a pas la
  section « seconde session » : elle n'est que sur `origin/master` ; lire depuis un arbre neuf ;
- `BUILD.md` dit encore `-j$(nproc)` et ne mentionne ni `poste` ni `fts` : c'est la fiche du
  rapport qui fait foi ;
- aucun ticket ni entrée du journal pour le plantage HNSW sur des vecteurs identiques ;
- le lot (c) est donné comme « la voie (a) », que le ticket dit faite (`1177f5794`) : le reste
  n'est pas nommé.
