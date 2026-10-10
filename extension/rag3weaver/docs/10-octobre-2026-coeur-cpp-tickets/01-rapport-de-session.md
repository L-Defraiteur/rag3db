# Cœur C++, correctifs et tickets — rapport de session

Seconde session cœur C++, ouverte le 10 octobre 2026 : les correctifs et les tickets du moteur,
pour que la session « coeur c++ » reste sur la stèle. Elle ne touche pas `src/transaction/` ni
`src/storage/` (journal, tampon, verrous) sans demander à « coeur c++ » ; elle ne corrige dans
`extension/vector/` qu'avec l'accord du banc.
**Dernière mise à jour : 10 octobre 2026, lot (a) commité (`90882b6d0`, branche `relcopy-sous-refus`).**

## L'arbre

- `/home/lucied/git_workspaces/rag3db-tickets`, worktree détaché sur `origin/master`
  (`d2bd94630`), `user.email` gmail en local. Bâti prévu : `build/moteur` (Release, tests,
  `vector;geo;fts`), toujours sous `~/.cache/rag3weaver-build/poste lourd … -j 8`.

**Au prochain rebase** (sur `0aed3c4b5` ou plus, version de stockage 40) : l'extension fts de
l'amont est retirée (`1275099c2`) et `fuzzy-fst` avec elle ; `rm -rf extension/fts
third_party/fuzzy-fst` si les dossiers restent, puis reconfigurer avec
`BUILD_EXTENSIONS="vector;geo"`.

Les noms des sessions depuis le plantage de Codium (10 oct.) : orchestration rag3db-c7, cœur C++
(stèle) rag3db-91, banc rag3db-10, arbre principal rag3db-73.

## Les lots, dans l'ordre de l'orchestration

1. (a) `CopyTest.RelCopyBMExceptionRecoverySameConnection` — **fait** (`90882b6d0`) : le test réglé sur la taille du COPY, joué aussi journalisé, et un témoin neuf ; `copy_tests` entier vert (23), les 17 tests de `transaction_test` qui utilisent la doublure verts.
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
4. (d) Les tickets confort du moteur, le plus petit d'abord.

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
