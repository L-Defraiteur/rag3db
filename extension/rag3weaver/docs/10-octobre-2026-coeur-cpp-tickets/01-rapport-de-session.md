# Cœur C++, correctifs et tickets — rapport de session

Seconde session cœur C++, ouverte le 10 octobre 2026 : les correctifs et les tickets du moteur,
pour que la session « coeur c++ » reste sur la stèle. Elle ne touche pas `src/transaction/` ni
`src/storage/` (journal, tampon, verrous) sans demander à « coeur c++ » ; elle ne corrige dans
`extension/vector/` qu'avec l'accord du banc.
**Dernière mise à jour : 25 octobre 2026, 21 h 30 — le rattrapage et les requêtes préparées sur master ; (A) lots 1 et 3 prêts sur create-concurrently-etat-4 (base 5651ac341), la chaîne à lancer après les mesures de la fenêtre.**

## Le 25 octobre 2026 — reprise : de quoi reprendre seul

### Sur master aujourd'hui (hotfixs)
- **6fe9ffe9b** : le rattrapage d'un CREATE sort ensemble les lignes déplacées et les supprimées,
  puis les réinsère (searchState.outOfGraph, findLiveUpperNode dans insertToLayer). Le repli de
  repairEntryPoint est resté à la conduite de master : avec le lot, RegionDelete/SetNullFirstAllButFive
  rougissait 2 fois sur 26, contre 0 sur 25 sur master.
- **fb4d83b65** (témoins 05a663402) : une requête préparée porte sa requête liée
  (`shared_ptr<CachedPreparedStatement>`), et CachedPreparedStatementManager disparaît. Témoin
  réel de G : 30,0 → 7,1 Go. L'ABI change. La bascule est faite (courant = rag3db-build-04063ee06
  aux deux postes, 17 h 43).
- Tickets : 758f73fab (les lignes d'un COPY forcé validé après un CREATE restent hors du graphe),
  26db89a10 (une interruption tombée entre deux instructions d'une requête réécrite est perdue).

### (A), lots 1 et 3 fondus, et la parade du crate — à pousser
- **Branche `create-concurrently-etat-4` = ff5a7bf4f, sur master 5651ac341** (le compteur d'octets
  du cœur), 8 commits ; le diff du rebase est lu, et aucun fichier n'est commun. Les branches
  `create-concurrently-etat`, `-2` et `-3` sont des états d'avant les rebases : ne pas les rejouer.
- **Forme retenue avec le cœur.** L'état est porté au holder (READY, NOTED, MAINTAINED,
  INCOMPLETE), versionné comme createdTS. Il est persisté au format 43 et journalisé par
  INDEX_STATE. Le CREATE se fait en trois temps : T_A (_CREATE_HNSW_NOTE), puis l'attente des
  écrivains plus anciens (_WAIT_HNSW_WRITERS), puis T_B (bâti, rattrapage de (δ), READY). Un jeton
  par requête : un NOTED dont la requête est morte se lit INCOMPLETE. Les recherches sont refusées
  par le nom de l'état. Un CREATE skip_if_exists sur un INCOMPLETE est refusé par « is behind its
  table », et rag3weaver le rebâtit par son chemin existant. SHOW_INDEXES gagne une colonne
  `state` en dernier.
- **Parade du crate**, relue par la session recherche : la garde de l'index vectoriel lit
  SHOW_INDEXES par nom de colonne (`RETURN *`). Si `state` manque, il vaut READY (lib d'avant le 43).
  INCOMPLETE ou « en construction » rend la branche Indisponible, avec sa raison dite.
- **Preuves déjà acquises** (sur l'ancienne base, luciepc) :
  - les 4 témoins VectorIndexCreateStates sont rouges sur master et verts avec le lot ;
  - les voisins VectorIndexRebuild et VectorIndexRetention.RolledBackCreateCanBeRedone sont verts
    dans rag3db-dbg.
  Les passes lancées ce soir sur l'ancienne base ne valent pas pour le push : la liste finale
  (`~/.cache/rag3db-tickets-notes/etat-liste2.log`) et le témoin produit rouge/vert
  (`e2e-etat-rouge.log`, `e2e-etat-vert.log`).
- **La chaîne à lancer** (permise ce soir par c7, après les mesures, sur luciepc qui porte deux
  lourds ; le cœur peut y jouer V2-2 en même temps). Le mbox est déjà à luciepc :
  `ssh luciepc "bash -c 'setsid nohup bash ~/.cache/rag3db-tickets-notes/chaine-a.sh 5651ac341 a.mbox a > /dev/null 2>&1 < /dev/null &'"`
  Elle enchaîne quatre étapes et s'arrête au premier rouge :
  1. lib-lot.sh : git am sur la base, puis la lib build/lecteurs-csv, sans publication ;
  2. e2e-code-lot.sh : e2e_code sur cette lib, prouvée par « ▸ moteur lié » ;
  3. e2e-lectures-temoin.sh : le témoin produit la_recherche_rend_les_mots_pendant_un_create_de_l_index_vectoriel ;
  4. lot-luciepc.sh : la liste complète. Si « verdict VERT », il fait le fetch, refuse si master a
     reçu du code moteur depuis la base, rebase, pousse sans force et écrit les trois contrôles.
  Journaux : `a-chaine.log`, puis `a-liste.log`. **Au premier rouge, on s'arrête et on le dit à
  c7 : pas de second passage sans relecture.** Après le push : le hash à c7, à l'arbre principal
  et au cœur (V2-2 attend après ce lot), avec les trois contrôles dans le message.
- **Le format 43 entre dans la bascule de demain** (décision de c7). Une lib 43 écrit des bases
  qu'une lib 42 refuse. Avant la bascule, la parade du crate tourne contre courant (04063ee06)
  sans la colonne `state`. Le témoin produit, lui, exige une lib 43 et le dit par un assert clair.

### Les deux mesures de la fenêtre calme (luciepc, sur le mot de c7, vers 22 h 30)
- Les binaires sont prébâtis : rag3db-mes-a (master avec le témoin de (A)), -b (d9b67e5d5
  chronométré), -c (6fe9ffe9b chronométré). On lance :
  `ssh luciepc "bash -c 'setsid nohup bash ~/.cache/rag3db-tickets-notes/mesure-calme.sh > /dev/null 2>&1 < /dev/null &'"`
  Chaque passe tourne sous `poste mesure`, avec la charge relevée avant et après.
- Lecture de `mesure-calme.log` :
  - **le témoin** : « create under 40 writers: X ms, … the longest Y ms ». La borne est de 5 s,
    et la ligne known_red le tient pour rouge.
  - **le rattrapage** : « TCCHRONO catchUp del=… upd=N … : M ms ». On compare M/N (ms par ligne
    déplacée) entre b (ancien) et c (nouveau).
  Sous une charge de 61 à 123, ce soir, le résultat n'était pas concluant : ancien 4,3 et 6,1 ms,
  nouveau 11,3 et 4,6 ms. Si c dépasse nettement b au calme, je réécris la sortie des lignes
  déplacées avec leur réinsertion comme ancre, comme le faisait updateInternal. Sinon (A)
  l'absorbe (T_C hors validation).
- Ensuite : les chiffres à c7, puis la chaîne ci-dessus.

### Qui attend le mot de qui
- **c7** : le mot des mesures (vers 22 h 30), puis la chaîne dans la foulée.
- **Après le push** : l'arbre principal rebâtit la lib unique à la bascule de demain (format 43).
- **Le cœur** : il pousse V2-2 après mon lot, et préviendra avant tout push qui touche
  transaction_manager.
- **Lots suivants de (A)** (page 07, §6) : les notes des écrivains et le nœud absent (2), T_C
  (4), la reprise et le DROP (5), la famine du bâti (6).

### Ménage de fin de session
- **luciepc** : les worktrees rag3db-tickets, rag3db-dbg et rag3db-mes-{a,b,c} (`git -C
  ~/git_workspaces/rag3db worktree remove --force …`, après la chaîne), et le target
  `~/.cache/rag3weaver-build/target-hotfixs`.
- **Ici** : rag3db-a13 (après le push) et rag3db-cic (son témoin est repris dans le lot).
- Jamais de fichiers laissés dans l'arbre de Lucie.

### Fautes et leçons du jour
- **Mon banc tournait sous `poste mesure`, qui est exclusif**, et sept travaux attendaient
  derrière. Le comparateur passe désormais sous `poste lourd` dans tous mes scripts.
- **Un rouge connu devenu vert ne compte plus comme rouge.** La garde de liste-cpp.sh l'écrit
  « banc code 1 — seulement des rouges connus devenus verts ».
- **Un test qui mesure des temps sous charge ne tranche rien.** La pente CostPerBatchSettles est
  passée en probabilistic (banc).
- **Le premier départage de RegionDelete (0 sur 5 de chaque côté) ne voyait rien.** Vingt passes
  par côté ont trouvé le lot fautif.

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

## La suite de la nuit (11 octobre, 2 h à 4 h 15)

- **Le lecteur affamé** est fusionné : `5c5feb527` (correctif `e4198f862`). La lib commune de
  luciepc est rebâtie dessus, et les worktrees du lot sont effacés.
- **`RAG3DB_BUILD_ID_GIT_DIR`** est sur master (`274ce4fc5`) : une copie de l'arbre sans `.git`
  (la crate Rust sous Windows) lit git dans le dépôt d'origine. Le côté `build.rs` est chez la
  session du paquet npm. Sur les runners GitHub, un checkout superficiel nomme la tête au lieu du
  dernier commit du moteur : c'est plus strict, sans danger.
- **Le rouge `OwnerlessPagesTest`** a été bissecté : 1 rouge sur 30 au parent, sans le lecteur
  affamé, 3 sur 40 sur master. L'arbre principal l'a reproduit dans un processus vivant (le COPY
  journalisé), et le cœur a trouvé la cause (les exceptions ALP d'un second segment, sous une
  sélection filtrée). Le ticket porte les comptes.
- **Le point de reprise interrompu** (branche `point-de-reprise-interrompu-2`) : le refus en
  lecture seule nomme la cause, et un crochet de test lu dans l'environnement est en place
  (`RAG3DB_TEST_HOOKS=1`, `RAG3DB_TEST_DIE_AT=checkpoint-logged`, décision de l'orchestration).
  Les témoins étaient rouges, ils sont verts, et la liste complète est verte. Le lot attend la
  fusion du correctif du cœur avant d'aller sur master.
- **Le petit tampon** est reproduit sur master et localisé (ticket) : le seuil vaut un quart du
  tampon, en puissance de 2, et la pile de l'échec est dans la réécriture hors place d'un
  segment de chaînes. Le correctif attend l'accord du cœur, dont c'est le stockage.
- **Faute** : la liste complète du point de reprise interrompu a d'abord échoué, parce qu'un
  en-tête public neuf (`common/test_hooks.h`) n'était pas déclaré dans
  `scripts/headers.txt`. Tout en-tête inclus par un en-tête public y entre.

## Fin de la nuit (11 octobre, 4 h 15 à 5 h 40) — arrêt sur rapport propre

- **Le point de reprise interrompu** est sur master : `b411e7f20` (correctif `9f3725ade`). Ses
  tests ont été rejoués sur la tête rebasée sur le correctif ALP du cœur : 41/41, 121/121, 3/3.
- **Les segments de chaînes bornés** sont sur master : `4820e9154` (correctif `47a283e6c`,
  restriction `dd4fd0a35`).
  - La cause : l'estimation de taille des chaînes passait par des statistiques que
    `appendString` ne tient pas à jour. `split()` gardait toute la colonne dans un segment, que
    chaque point de reprise réécrivait par doublements.
  - Les témoins étaient rouges et sont verts. La sonde Rust du ticket passe 3 Go sous 2 Gio, en
    65 et 70 s. La liste complète est verte sur la tête finale (10 rouges connus, 248 verts au
    banc, 1 867 e2e).
  - La première version cassait les FLOAT[960] : le recul entassait des valeurs mortes dans le
    fils des listes. La liste complète l'a vu, et le recul est restreint aux chaînes.
  - Le ticket est fermé. Deux tickets sont ouverts : la confirmation par le premier index sous
    2 Gio (arbre principal, de jour) et la mémoire résidente au-delà du tampon. Le ticket voisin
    des blobs réécrits devrait tomber, non éprouvé.
- **La lib commune de luciepc** est rebâtie sur `4820e9154` (identifiant `dd4fd0a35` dans la lib
  et l'extension). Les worktrees, le target `target-tickets` et les branches de travail sont
  effacés sur les deux postes. Plus rien ne tourne de mon côté.
- **Faute répétée** : par deux fois, un bâti par cible sans `rag3db_vector_extension` a fait
  rougir les tests vector sur une extension d'un autre commit (le contrôle de bâti la refuse).
  C'est en mémoire.
- **Pour reprendre** :
  - la limite des colonnes LIST ou STRUCT de chaînes (lots de 64 sans recul), au ticket fermé ;
  - la mémoire hors du tampon, à répartir avec heaptrack sur la sonde C++
    (`~/.cache/rag3db-tickets-notes/blobs_checkpoint_probe_scratch_test.cpp`) ;
  - les deux tickets Ladybug sur les chaînes, à lire.

## Le 11 octobre, de midi au soir — la condition 3 (A7, A8) et `CALL build_id()`

- **Le COPY de blobs**, reproduit sans harnais : un seul COPY de plus d'un quart du tampon échoue
  à l'assemblage du groupe en mémoire. La forme retenue est un groupe vidé par morceaux bornés en
  octets, en allongeant les segments de la tête persistante (page 04, relue par le cœur). Le lot
  est **reporté** après la condition 3 ; le filet du produit est en place.
- **A7, la file des points de reprise** : `c0f50f453` et la page 05. La relecture du cœur a
  trouvé deux défauts (une validation de la file qui échoue reste active ; une attente non bornée),
  corrigés avant la fusion.
- **A8, le dernier partant écrit** : `f024578d5` et la page 06.
  - La première forme (attendre dans la validation) bloquait le planificateur à deux fils.
  - Le BEGIN explicite attendait lui aussi sur un fil du planificateur ; il attend maintenant dans
    le fil de l'appelant.
  - La liste complète a trouvé que sous un lecteur long, chaque écriture attendait 1 s :
    correction par une seule attente, de 250 ms.
  - Le plafond du témoin est fixé par la mesure (master de 210 à 358 Kio, correctif de 74 à 91 Kio).
- **`CALL build_id()`**, pour le démon de la mémoire : `48b74ead1`.
- **Relu pour le cœur** : T0, la pierre tombale d'A3′ reprise (avec le risque des décalages
  positionnels, puis un second défaut, l'état partagé du détachement, trouvé en relisant C7), C7.
- **Fautes** :
  - par deux fois, un bâti par cible sans `rag3db_vector_extension` (la mémoire le garde) ;
  - un témoin d'A8 insatisfiable par construction, vu par le cœur ;
  - le rebâti de la lib commune de luciepc lancé malgré un démon qui la chargeait (sans dégât ;
    le script s'arrêtera désormais).
- **Pour reprendre** :
  - le ticket « index vectoriel sous écrivains simultanés, beyond the visited set » (banc,
    témoin `VectorIndexUpdate.AnUpdateWaitingBehindAnInsertSeesItsNewRows`) ; la forme pressentie
    agrandit l'ensemble visité au nombre de lignes au début de chaque opération, sans relâcher le
    contrôle de borne ;
  - le CHECKPOINT explicite et la validation forcée attendent encore sur un fil du planificateur
    (page 06).
  - Le worktree `rag3db-tickets` de luciepc est gardé, sur master, avec son `build/` chaud.

## Le 11 octobre, le soir — le visited set (`3e0f98f08`)

- **Cause** (l'hypothèse du cœur) : l'état d'une écriture sur le graphe HNSW est créé avant que la
  table prenne le verrou de l'index ; l'attente reprend l'instantané, et le graphe atteint les
  lignes que l'autre écrivain a validées, au-delà de l'ensemble visité.
- **Correctif** : `VisitedState::ensureSize`, appelé par `fitVisitedToTable` au début de `update`,
  `delete_`, `finalizeDelete`, `commitInsert`, et de `search` par précaution. La borne reste.
- **Preuves** :
  - le témoin du banc, rouge 3 sur 3 sur master, vert 5 sur 5 ;
  - H4 rouge 3 sur 3 sur `cad384056` sans le correctif (seulement « beyond »), vert 5 sur 5 avec ;
  - la liste complète verte sur `cad384056`, puis le rejeu court vert sur B2 (`332d7027e`).
  Le témoin et H4 sortent de `known_red`, le ticket est fermé.
- **Un témoin retiré** : la recherche après un SET qui attend (demandé par le cœur) passait déjà
  sur master. `QUERY_VECTOR_INDEX` ne cherche qu'une fois par état, compté après le SET.
- **Fautes** :
  - mon verdict ajouté à la liste prenait la ligne `FIN` pour un rouge : la liste était verte, la
    règle est corrigée ;
  - une boucle H4 sur l'ancienne base est restée orpheline après une coupure ssh ; l'arrêter a
    été refusé, elle a fini seule. Les passes longues partent maintenant détachées (`setsid`).
  - deux conflits sur l'index des tickets au rebase : on repart de la version de master et on
    rejoue ses deux lignes, puis `check_tickets.py`.
- **Pour reprendre** :
  - le rebâti unique de la lib commune de luciepc (pour le cœur et pour moi), après le « fini »
    de codeparsers et le hash de `systemconfig-abi` ; pas d'ABI changée par le visited set ;
  - **pour Lucie, la lib commune et l'hôte figé** (pid 132823, code-lourd, quatrième gel). La
    lib de luciepc est celle que le cœur a rebâtie à 20 h 39 sur `a72acfeef` (identifiant
    `rag3db-build-6df47a543`) : sans le visited set (`3e0f98f08`, poussé après). L'orchestration autorisait un
    rebâti sous l'hôte figé ; ma session l'a refusé (classifieur), je n'ai pas insisté. La forme
    proposée garde la pile intacte : déplacer par `mv` les deux fichiers mappés (`librag3db.so`,
    inode 35665439 ; `libvector.rag3db_extension`, inode 35665435) en `.fige-132823`, même inode,
    puis bâtir à neuf et vérifier que les inodes ont changé. Elle est écrite dans
    `~/.cache/rag3db-tickets-notes/rebatir-lib-commune.sh` (variable `FIGE_PID`) sur ce poste.
    **Aucun `.fige-132823` n'existe** : le script n'a pas été lancé. La copie du script vers
    luciepc a été refusée, et je n'ai pas pu vérifier si elle avait eu lieu. Sans `FIGE_PID`, les
    deux versions s'arrêtent net (exit 2) tant que l'hôte charge la lib. À trancher : arrêter
    l'hôte (après en avoir pris la pile), ou accorder la forme par déplacement ;
  - **pour Lucie, ce qui attend sur luciepc** (12 octobre, 0 h 30) : le client d'index de
    codeparsers (pid 132918, `index.sh lourd principal`) s'adresse à l'hôte figé et tient le
    poste. Derrière lui attendent ma mesure (pid 242676, les témoins voisins du point de reprise)
    et son script parent `att-vert.sh`, puis `att-2.sh` (le témoin à pause d'1 s, cinq fois sur
    master puis cinq fois avec le correctif), et les bâtis lourds des autres, dont le binaire de
    G. Terminer le client 132918 libère la place : tout cela repart seul, dans l'ordre. Rien
    n'est à relancer de mon côté ;
  - le CHECKPOINT explicite et la validation forcée sur un fil du planificateur (page 06) ;
  - le COPY de blobs (page 04).

## La nuit du 11 au 12 octobre — QUERY_VECTOR_INDEX, DROP/CREATE, la suppression HNSW

- **QUERY_VECTOR_INDEX déclare k** (`db1044912`). La prémisse « sans estimation, 32 fils » était
  fausse (mesurée : un fil, card=1 posé en dur). Le lot corrige l'estimation (k), le masque
  dimensionné sur la table, et la cardinalité d'une fonction de table filtrée dans le
  CardinalityUpdater.
- **DROP/CREATE contre une recherche** (ticket du banc). Pas 1 écrit, relu deux fois par le cœur :
  - la liste des index sous verrou ;
  - l'index capturé en `shared_ptr` et reconnu par les oid de ses tables de relations ;
  - le refus nommé ;
  - un retrait concurrent reconnu au catalogue (`isIndexChangedAfterSnapshotOf`) ;
  - l'annulation du catalogue sous son verrou (commit à part).

  La liste complète a trouvé une régression (un index détaché, passé un point de reprise, pris pour
  un DROP), corrigée. Verte sur 4ac16805d. Le push a été refusé, à raison, par l'arrivée du
  balayage sous masque du cœur. Le rejeu sur ea914bbae plante dans `lookupOffsets` (code de
  41d282843), corrigé par le cœur dans son lot de l'insertion : **retenu jusqu'à son hash**. Le
  pas 2 (la rétention) est au ticket ouvert.
- **La suppression sous index HNSW** (ticket du proto). Les témoins du banc sont sur master
  (`baf76c474`), et son profil par piles tranche : keepNodeReachable, une recherche ef = 200 par
  voisin survivant, pèse 88 %. Elle est amplifiée par la lecture aléatoire non checkpointée (un
  verrou et un isSelected par ligne). Partage avec le cœur : à lui le stockage (resize quadratique
  du point de reprise, verrou par fenêtre, detachDelete sans findMatchingRow, tri par suite), à moi
  extension/vector :
  - **(A)** écrit : `keepFormerNeighboursReachable`, partagé par update et finalizeDelete ;
  - **(C)** l'arrêt de la chaîne UpdateInfo, écrit, borné à ×2 par le calcul ;
  - à faire : initUpdateState par ligne, les prepareRelScan de createRels.

  Rien n'est bâti avant le lot du cœur.
- **La lib de luciepc** : le bâti de G au magasin fait foi, rag3db-lourd ne se rebâtit plus
  (mémoire `magasin-de-libs`).
- **Fautes** :
  - un témoin de recherche vert sur master, retiré ;
  - mon verdict de liste qui prenait `FIN` pour un rouge ;
  - deux arrêts de mes propres tâches refusés ce soir-là (je n'insiste pas, je le signale).
- **Pour reprendre** :
  - le push DROP/CREATE après le hash du cœur (rebase, rejeu court) ;
  - puis (A) et (C), mesurés sur le master qui porte le lot du cœur ;
  - le lot des attentes du point de reprise (`c103b9208`), qui attend luciepc.

## Le 12 octobre, la journée — quatre lots sur master, la rétention en attente

**Sur master :**
- **Le DROP/CREATE contre une recherche** (`2b0f2820b`, `c42fc98bf`) : la liste des index sous
  verrou, l'index capturé et reconnu par ses tables de relations, le refus nommé, le retrait
  concurrent reconnu au catalogue (`isIndexChangedAfterSnapshotOf`, qui couvre aussi un DROP
  validé après le début du lecteur, remarque du cœur), l'annulation du catalogue sous son verrou.
  La liste a d'abord trouvé une régression : un index détaché, passé un point de reprise, était
  pris pour un DROP. Le push a été refusé deux fois par des avances du moteur, et un rejeu a
  attrapé le plantage du masque du cœur (`lookupOffsets`, corrigé par lui).
- **Les attentes du point de reprise** (`9ea7cd0fc`) : `BlockingWait` dans le gestionnaire de
  transactions (CHECKPOINT explicite, validation forcée, file d'A7). Témoin rouge 5/5 sur master
  (le COMMIT attend 4,0 s), vert 5/5.
- **La suppression sous HNSW** (`690029ab7`, en trois commits) :
  - (C) l'arrêt de la chaîne UpdateInfo ;
  - la recherche qui ne complète plus jusqu'à k, cause réelle des « nœuds supprimés rendus par le
    CALL » et des pseudo-lignes qui faisaient planter le masque ;
  - (A) `keepFormerNeighboursReachable`, partagée par update et finalizeDelete. Forme du produit :
    136 → 6,9 ms ; index rempli par SET, 15 000 lignes : 161 → 5 ms. Atteignabilité prouvée par
    `Cases/RegionDelete`, témoin demandé par le cœur. Le proto a retiré son contournement
    (`c0679ed33`).
- **Le bâti Windows** (`0fc7dd10f`) : `RAG3DB_API` retiré de `NodeTable::loadIndex`, que la classe
  exporte déjà (MSVC C2487, relevé par G).
- **La lib de luciepc** a été publiée au magasin (identifiant `rag3db-build-3cca0ae1e…`), après
  l'arrêt de l'hôte par Lucie.

**En attente — la rétention** (commit local `205d2923e`, worktree `rag3db-retention`, sur
`799e0fad5`) : la version du holder, `INDEX_CHANGE`, l'effacement au point de reprise, la
résolution par les oid, le DROP en exclusif. Les trois témoins de la rétention sont verts. Le
cœur a relu et validé. La liste complète a été arrêtée par la pause de Lucie. Deux corrections
viennent de la relecture et de la liste :
- **forme (α)** : le CREATE en exclusif et les écritures en partagé sont écartés, parce que le
  point de reprise forcé du CREATE attendrait l'écrivain qui attend son verrou ;
- **identifiant de holder partagé** : chaque holder reçoit maintenant un identifiant unique ;
  partagé, le DROP de l'index détaché effaçait la clé primaire.

Le lot ouvre le ticket des lignes validées pendant le bâti d'un CREATE, témoin rouge sur master
(309 lignes manquent sur 20 312).

**Fautes** :
- `RealVectorsReachability` compté vert alors qu'il se sautait (faute d'export) ;
- une mesure « après » jouée sur les anciens binaires, le bâti ayant échoué : le script s'arrête
  maintenant ;
- deux `RAG3DB_API` en double, dont un a cassé Windows ;
- l'identifiant de holder partagé.

**Pour reprendre** (après la pause, au mot de l'orchestration) :
- la rétention : rebase, liste complète sur luciepc (`ret-liste.sh`), push, worktree effacé ;
- puis le ticket du CREATE concurrent, formes (β) et (γ′) à peser avec le cœur.

## Le 18 octobre — la reprise : la rétention sur master (`9b861579a`), trois tickets, /tmp

**La rétention est sur master** en `9b861579a`. Elle versionne les holders d'index du
stockage (DROP et CREATE d'un index visibles au seul auteur jusqu'à la validation, annulés par
`INDEX_CHANGE`, effacés au point de reprise). Elle ferme le ticket de la rétention et ouvre celui
du CREATE concurrent, dont le témoin `VectorIndexUpdate.RowsCommittedDuringACreateAreIndexed`
est en known_red.

Deux listes complètes l'ont jouée sur luciepc, le lot appliqué sur chaque base.

| base | bâti | transaction | api | c_api | copy | stockage | banc | vector disque / mémoire | e2e | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| `2cb6b2524` | 0 | 275 | 105 | 136 | 23 | 83 | 7 rouges, 283 verts, conforme | 74 / 63 | 1 867 | VERT |
| `87a362e30` (E10), bâti neuf | 0 | 282 | 105 | 136 | 23 | 83 | 7 rouges, 283 verts, conforme | 74 / 63 | 1 867 | VERT |

Dans la première, les trois `VectorIndexRetention` passent. Les deux SKIPPED de
`transaction_test` sont des sondes (`VectorIndexCrashReopenProbe.OpenInAFreshProcess`,
`WalReader.PrintAFile`).

La première liste prouvait le lot sur `2cb6b2524`. L'orchestration a fait passer E10 avant moi,
une corruption de données que le cœur poussait déjà : si j'avais poussé d'abord, sa passe
repartait pour une heure. Mon rejeu, lui, ne coûtait que du temps à luciepc.

**Le conflit avec E10.** E10 prenait la valeur 9 de `UndoRecordType` (`ADD_COLUMN`), que la
rétention prenait aussi (`INDEX_CHANGE`). J'ai gardé les deux et passé `INDEX_CHANGE` à 10. Le
tampon d'annulation ne vit qu'en mémoire, ni au WAL ni sur disque : aucun format ne change. Les
deux déclarations avancées, les deux `create*` et les deux cas du commit et du rollback sont
gardés.

**Le push depuis luciepc**, permis par Lucie ce jour-là (clé ssh L-Defraiteur). Le script
(`ret-rejeu-luciepc.sh`) tourne détaché par `setsid nohup`, pour survivre à une coupure du ssh.
Il vérifie le gmail, commite le lot par `git am` et joue la liste. Puis, seulement dans un
`if grep -qx "verdict VERT"`, il refuse si du code du moteur est arrivé depuis la base
(code 9), rebase, refuse sur un conflit (code 8), passe `check_tickets`, pousse une fois sans
force et écrit les trois contrôles au journal. Je les ai relus d'ici.

**Le premier rejeu sur E10 est tombé rouge au bâti, par ma faute.** Le `clean` demandé pour un
bâti neuf (E10 ajoute des méthodes virtuelles) efface la grammaire Cypher générée. Elle est
suivie par git, et le bâti suivant ne la régénère pas. Le `if` a retenu le push. Le script
restaure maintenant `third_party` et exige un arbre suivi propre avant de bâtir. J'ai ouvert le
ticket moteur à la demande de l'orchestration (`c3ddc94e4`).

**Deux tickets ouverts à la reprise** (`5781ce02a`, docs seules) :
- **l'hôte ne rend pas la mémoire d'un gros index** : 10 Go dans l'échange, trois jours après le
  rattrapage du 12 ;
- **les tests lancés hors de `run_e2e.sh` laissent leurs bases dans `/tmp`** : un
  `remove_dir_all` appelé sur une base, qui est un fichier, et `rag3weaver_cache` que rien
  n'efface.

**Le ménage de `/tmp`**, confié par l'orchestration : 4 033 entrées de `e2e_rag3daemon` et
`e2e_code` des 11 et 12 octobre, effacées par une liste écrite (`23faa8c57`). Aucun processus
ne les tenait (`/proc`, `fuser`, `lsof`). Elles pesaient 141 Mio, ce qui écarte cette piste pour
l'échange.

**Mes fautes du jour :**
- le `clean` qui a cassé le premier rejeu ;
- un commit de ticket sans `git add` du fichier neuf, suivi d'un push « réussi » qui n'envoyait
  rien (HEAD égal à master). Le script de docs refuse maintenant ce cas.

**La suite :** le ticket du CREATE concurrent, que je tiens, en consultant le cœur en léger sur
le point de reprise forcé et les verrous. Mon worktree `rag3db-tickets` de luciepc reste, propre,
sur `9b861579a`, avec son `build/moteur`, jusqu'à la fin de la session.

## Le 18 au soir et le 19 octobre — quatre lots sur master, la page de (A)

**Les lots sur master** :

| commit | lot | ticket |
|---|---|---|
| `a333c34f2` | un ROLLBACK sous un tampon plein n'alloue rien, la transaction quitte toujours le système, ~ClientContext ne lève pas, un CHECKPOINT refusé avant tout changement laisse la base ouverte | fermé ; « à confirmer » ouvert pour l'annulation d'un COPY |
| `cf71c929d` | (δ) : le CREATE_VECTOR_INDEX rattrape à sa validation ce que les autres ont validé pendant son bâti ; la borne à la première ligne encore à valider, la déduplication du balayage ; les états d'index par holderID | fermé ; défaut connu ouvert : la retenue du rattrapage |
| `2822e314d` | R1 : une extension charge ses index avant ses fonctions, une capture commune dans NodeTable, les accès par nom avec la transaction | celui du banc fermé ; la rétention pour tous les index reste ouverte pour geo (R2) |
| `4f9274a38` | la réservation d'adressage par tranches de 16 Gio au lieu de 8 Tio d'un bloc | fermé |

Chacun a passé `e2e_code` entier sur sa propre lib (27/27, la lib et l'extension prouvées par
« moteur lié »), puis la liste complète, avant son push.

**(δ) et sa retenue.** Le scénario des vingt écrivains a montré que (δ), juste, retient les
autres connexions 115 à 273 s pendant son rattrapage, contre 7,8 s sur master, qui perdait des
lignes en silence. Ces 7,8 s de master viennent de la famine du bâti (le bâti prend tous les
fils, 2 en test), pas d'un verrou : le banc l'a daté, puis confirmé.

(δ′), des passes de rattrapage avant la validation, a échoué : un seul fil ne rattrape pas vingt
écrivains, et ses arêtes ont rempli le tampon. L'orchestration a tranché : la justesse d'abord,
(δ) sur master avec sa retenue en défaut connu, puis (A), le vrai CREATE INDEX CONCURRENTLY.

**La page de (A)** (`07-create-index-concurrently.md`, `4a7d74cf2`, précisée en `67feff3af`) :
- trois transactions et des états NOTÉ, ENTRETENU, PRÊT, plus INCOMPLET après un arrêt
  brutal ;
- les écrivains notent puis entretiennent, et décident à leur validation ;
- le rattrapage des seules lignes notées ;
- la retenue qui reste, celle du point de reprise forcé du bâti.

Elle est relue par le cœur C++, avec sept précisions.

**Les défauts trouvés en route** :
- **Ma rétention** (`9b861579a`) avait laissé sans filtre la finalisation d'un COPY : un CREATE,
  un DROP puis un COPY finalisaient dans l'index retiré (« _UPPER does not exist in catalog »).
  La régression a d'abord été imputée au rollback du cœur, et les magasins de libs sont revenus
  en arrière. Le cœur l'a corrigée avec le filtre de visibilité que j'ai relu.
- **Le ROLLBACK** : posé à l'échange des groupes, le drapeau « la mémoire a changé » venait
  trop tard. Les colonnes du groupe persistant s'écrivent par le fichier ombre avant, et le point
  de reprise de fermeture se bloquait. Le drapeau est maintenant posé après la copie fusionnée.
- **Le chargement d'extension** : la fenêtre entre l'enregistrement des fonctions et le
  chargement des index. Seul un crochet de test l'a rendue déterministe : 30 rechargements sous
  charge ne la montraient pas.
- **Les « Mmap … failed » du banc** sous les cas longs : une faute du banc (un plafond posé en
  absolu), et non une fuite du moteur.

**Mes fautes du jour** :
- la rétention non relue sur tous ses parcours (mémoire `relire-tous-les-parcours-d-une-structure`) ;
- le drapeau du point de reprise posé trop tard ;
- deux témoins verts présentés d'abord comme des témoins, avant le crochet ;
- un test qui gardait des résultats au-delà de la vie de sa base.

**La suite** : (A), d'abord ses témoins rouges. Restent aussi le ticket « à confirmer » de
l'annulation d'un COPY sous un tampon plein, et R2 de la rétention (geo, quand geo servira).

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
