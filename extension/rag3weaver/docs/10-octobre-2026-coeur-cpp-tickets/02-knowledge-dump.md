# Cœur C++, correctifs et tickets — ce que la session sait

Ce qui est affirmé porte un fichier et une ligne (à `d2bd94630` ou `ba11e003b`) ou une mesure ;
le reste est dit « non vérifié ». Complète les relevés de la session cœur C++ (stèle) :
`../3-octobre-2026-23h31/coeur-cpp/02-knowledge-dump.md`.
**Dernière mise à jour : 25 octobre 2026, 21 h 15.**

## Le 25 octobre 2026 — ce que le code fait désormais (à create-concurrently-etat-3)

- **L'état de construction d'un index** (`IndexBuildState`, storage/index/index.h).
  - READY vaut 0 : un holder d'avant le format 43 est prêt.
  - Les transitions sont versionnées comme createdTS : markBuildState(txID), puis
    commitBuildState(ts) par un INDEX_CHANGE de genre BUILD_STATE, ou rollbackBuildState.
  - buildStateFor(id, startTS) donne l'état vu à l'instantané (les recherches et SHOW_INDEXES).
    committedBuildState() donne le dernier état validé, que les écrivains lisent sous le verrou
    d'index.
  - withBuilder : un NOTED ou MAINTAINED dont le jeton (`ClientContext::newQueryScopedToken`,
    rendu en fin de queryNoLock) est mort vaut INCOMPLETE. Il en va de même d'un état relu du
    disque ou du journal, puisqu'il n'a pas de jeton.
- **Les écrivains** ne touchent un holder que s'il est READY ou MAINTAINED (`receivesWrites`) :
  isWritableIndex, initUpdateState, l'étape 3 de la validation et la finalisation du COPY.
- **Les refus.**
  - captureVisibleIndexes : « Index X of table T is under construction (state NOTED) … » ou
    « … is incomplete: its build did not finish. Rebuild it: drop it, then create it again. »
  - validateIndexExistence(CREATE) sur un INCOMPLETE : « Index X is behind its table T: its build
    did not finish (state INCOMPLETE) ».
- **Le CREATE réécrit** (rewriteCreateHNSWQuery) : BEGIN, les deux CREATE REL TABLE,
  _CREATE_HNSW_NOTE, COMMIT ; puis _WAIT_HNSW_WRITERS ; puis BEGIN, [cache], _CREATE_HNSW_INDEX,
  RETURN, COMMIT.
  - Le bind interne du bâti saute la vérification d'existence.
  - finalizeHNSWTableFinalizeFunc appelle `NodeTable::completeNotedIndex` (l'index bâti remplace
    l'index vide, transition vers READY), puis le rattrapage de (δ) en action d'avant validation,
    et le point de reprise forcé.
- **L'attente des écrivains plus anciens** :
  `TransactionManager::waitForWriteTransactionsStartedBefore(ctx, startTS, self)`, exportée
  membre par membre (la classe ne l'est pas). Elle attend sur transactionsChanged sous le verrou
  public ; le seuil est le startTS de sa propre transaction. D'après le cœur, il n'y a pas
  d'interblocage avec checkpointWriting de V2-1c.
- **La validation forcée de T_B attend toutes les autres transactions, lecteurs compris.** C'est
  ce qui ferme le trou de l'écrivain parti pendant NOTÉ (mesuré : le CREATE finit 5,3 s après
  lui). Un lecteur ouvert pendant tout le CREATE le fait donc expirer
  (RolledBackCreateCanBeRedone) ; l'index reste alors INCOMPLETE.
- **Le format 43** :
  - un octet d'état après chaque holder (IndexHolder::serialize ; relu dans NodeTable::deserialize
    si la version vaut 0 ou est supérieure à 42) ;
  - le type de journal INDEX_STATE_RECORD = 19 (IndexStateRecord, replayIndexStateRecord).
- **SHOW_INDEXES** : la colonne `state` est la 7e et dernière, lue au holder. Une entrée sans
  holder s'y dit INCOMPLETE.
- **La marche T0** : une instruction qui échoue dans un bloc explicite laisse la transaction
  ouverte et marquée en échec. Il faut ROLLBACK : un COMMIT serait refusé. C'est vrai aussi du
  bloc de T_B d'un CREATE en échec.
- **Une interruption ne vaut que pour l'instruction en cours** (executeNoLock appelle
  resetActiveQuery). Voir le ticket 26db89a10.
- **Le coût du rattrapage du nouveau chemin** (instrumentation jetable, une passe, charge 70) :
  1 787 lignes déplacées ; sortie 0,15 s ; finalizeDelete 1,7 s de réécriture de 34 291 voisins,
  plus 1,3 s de joignabilité sans ancre ; réinsertion 9,1 s ; rétrécissement 1,5 s ; environ 14 s
  au total, pour un CREATE de 55 s. L'essentiel est le bâti affamé par la charge.
- **Le témoin d'acceptation de (A) sur master**, sous une charge de 30 à 70 : CREATE de 48 à 61 s,
  attente la plus longue de 41 à 56 s (contre 9 à 11 s le 19). Il se départage seulement au calme.

## Le tampon, le journal local et les doublures

- **Le tampon par défaut d'un test qui passe par `SystemConfig{}`** vaut 0,8 × la mémoire
  physique (`database.cpp:56-76`, `constants.h:55`) : ~97 Gio sur ce poste. Le seuil du repli
  d'un `COPY` journalisé (`Transaction::copyJournalThreshold`, un huitième du tampon) y prend
  donc son plafond, 256 Mio. Le tampon « minuscule » des tests est
  `TestHelper::DEFAULT_BUFFER_POOL_SIZE_FOR_TESTING`, pas celui-là.
- **Le journal local réserve une page de 4 Kio à la fois** (`InMemFileWriter::write`,
  `in_mem_file_writer.cpp:22` → `MemoryManager::allocateBuffer` → `mallocBuffer` →
  `BufferManager::reserve`, `memory_manager.cpp:54-57`, `:70-72`). Un refus y lève le même
  message que tout autre manque de tampon. Coût mesuré : ~40 ns par réservation, négligeable.
- **Une relation sans propriété pèse 75 octets au journal** : trois `INTERNAL_ID` de 25 octets
  (drapeau de nullité 1, type 3, `isNull` 1, `childrenSize` 4, identité 16). Le compte retombe
  sur la mesure du 5 octobre.
- **`FlakyBufferManager`** refuse quand `(reserveCount + 1) % failureFrequency == 0` ; le
  compteur est cumulé entre requêtes, la fréquence doublée après un refus. Un test qui remet la
  fréquence à chaque essai annule ce doublement. Toute doublure « une réservation sur N » voit un
  `COPY` journalisé réserver ~6 fois plus (7 000 hors journal, 45 000 avec, pour twitter).
- **Un `COPY` multi-fils interrompu par un refus** en compte souvent deux : les autres fils
  réservent encore avant de voir l'erreur.
- **`CALL force_checkpoint_on_copy=false`** se pose par connexion : un test peut jouer le chemin
  journalisé sans changer le défaut ni rebâtir (`client_config.h` est inclus partout).

## Les statistiques

- Voir le rapport, lot (c) : un HyperLogLog ne recule pas (`hyperloglog.h:33`) ; `DELETE` et
  `SET` ne touchent pas `TableStats` ; les lignes écartées par `IGNORE_ERRORS` restent
  comptées ; seule la cardinalité est recalée (`node_group_collection.cpp:217`, `:290`) ; les
  tables de relations n'ont aucune statistique tenue (`nextRelOffset`).

## Pièges

- **Le plein texte sépare les chiffres d'un mot** : « word77 » cherche « word » (rapporté par
  la session cœur C++, non vérifié ici). À savoir pour un témoin qui cherche des mots numérotés.
- **Le verrou du poste** : un `poste lourd` attend la porte au plus 600 s, puis attend en partagé
  la fin de la mesure en cours, sans délai. Il ne « sort » pas sur le délai.
- **Après un plantage de l'éditeur**, les travaux de fond lancés par la session meurent sans
  trace (fichier de sortie vide) : relancer et le dire.
- **Le rapport cœur C++ de l'arbre principal** peut être en retard sur `origin/master` : lire
  les docs depuis un arbre neuf.

## La nuit du 10 au 11 octobre

- **L'index de clé sous `IGNORE_ERRORS`** : `IndexBuilder::maybeConsumeIndex`
  (`index_builder.cpp:73-93`) prend la valeur rendue par `appendWithIndexPosNoLock` pour la
  position du premier échec ; `HashIndex::appendNoLock` la rendait juste seulement dans sa branche
  en mémoire — corrigé `599f0494b`. Laquelle de deux lignes de même clé reste suit l'ordre
  d'arrivée dans l'index, donc des fils de lecture (pas de tri) ; `duplicated.test:81-82` le dit.
- **Une fonction de table à paramètre `LIST`** doit fournir `inferInputTypes` : sinon le binder
  construit `LogicalType(LIST)` sans type d'élément et lève (`bind_table_function.cpp:47-57`,
  `types.cpp:543-549`) — c'est pourquoi l'index spatial de geo ne se crée pas.
- **`nextRelOffset`** (`rel_table.h:214-219`) est à la fois l'estimation du nombre de relations
  (planificateur, `getNumRels`) et l'allocateur de leurs identités : intouchable ; ni l'annulation
  ni un `DELETE` ne le reculent.
- **Les paramètres d'une requête préparée** prennent le type de la valeur fournie
  (`bind_parameter_expression.cpp:16-17`), retypable seulement s'il contient `ANY`
  (`parameter_expression.cpp:10-20`) ; rag3weaver type un `NULL` non typé en `STRING` et une liste
  par son premier élément (`rag3db_connection.rs:663-688`).
- **L'identifiant de bâti** (`458ff7157`) : `cmake/build_id.cmake`, cible `rag3db_build_id`,
  `build/…/src/include/common/build_id.h` ; chaque extension exporte `build_id()` ;
  `ExtensionManager::loadExtension` compare entre `name()` et `init()`. Pour le lire :
  `strings <lib ou extension> | grep rag3db-build-`.
- **Le premier `put_note` d'un backend sur base neuve** est un `COPY` d'une ligne (rag3weaver,
  `ingest_entities_jusqu_a`, depuis `c7067475d`) ; un tel `COPY` sous 256 Mio passe dans le moteur
  nu jusqu'à `FLOAT[1024]`, index vectoriel et 32 fils (sonde, 36 cas). Le groupe local d'un `COPY`
  (`StorageConfig::NODE_GROUP_SIZE`) ne réserve pas sa capacité d'avance.
- **Jouer un script de backend sur luciepc** : bâtir `rag3weaver-backend` dans son worktree et son
  target (`RAG3DB_SHARED=1`, `RAG3DB_LIBRARY_DIR`/`RAG3DB_INCLUDE_DIR`/`RAG3DB_BUILD` vers la lib
  commune, `CARGO_TARGET_DIR`), un lien du binaire vers `extension/rag3weaver/target/debug/`, une
  COPIE de l'extension (jamais un lien), `LD_LIBRARY_PATH` vers la lib, les services locaux
  127.0.0.1:7878/7879/7880 ; le python du venv-mcp pour l'aller-retour MCP (sur le poste principal :
  `~/.cache/rag3weaver-build/venv-mcp`). Le target fait ~14 Go : l'effacer à la clôture.

## Le verrou des lecteurs (11 octobre)

- L'identité d'une ouverture en lecture seule, c'est la page 0 et le préfixe du journal :
  rien de `checkpointStorage` ne les change, puisque le moteur écrit par copie (pages neuves,
  pages fantômes). Le premier changement visible est `wal->logAndFlushCheckpoint`.
- Les doublures de test surchargent `logCheckpointAndApplyShadowPages` : un changement de
  signature les casserait toutes. D'où le verrou en membre, et `holdReadersLock()` appelé de
  l'intérieur.
- flock sépare deux descriptions de fichier d'un même processus : un test peut tenir le verrou
  lui-même pour simuler un autre processus.
- `poste lourd timeout N cmd`, jamais `timeout N poste lourd cmd` : la borne ne doit compter
  que le travail.

## Pièges de la nuit du 11 octobre

- Un en-tête inclus par un en-tête public (`db_config.h`, `database.h`…) doit figurer dans
  `scripts/headers.txt`. Sinon, l'en-tête unique (`rag3db.hpp`) échoue au bâti complet, alors
  qu'un bâti de cible ne le voit pas.
- Lire une variable d'environnement de façon statique, une fois par processus, se perd avec
  `fork` : le fils hérite de la valeur déjà lue par le père. Les crochets de test sont donc lus
  à chaque ouverture de base.
- gdb sur luciepc : `gdb -batch -ex "catch throw <type>" -ex run -ex "bt 40" --args <test>`
  donne la pile d'une exception. Les binaires de test Release ne sont pas dépouillés de leurs
  symboles.
- Une bissection se fait sur deux bâtis de tête, dans deux worktrees, chaînés dans un seul
  script. On vérifie d'abord par `git log A..B -- src` que seul le commit suspect diffère.
- `ColumnChunkData::split()` décide d'après `getSizeOnDiskInMemoryStats()`. Un bloc dont les
  statistiques ne sont pas tenues (les octets des chaînes, copiés par `memcpy`) paraît vide, et
  tout tient dans un segment. Pour une liste, `truncate()` ne retire rien du fils, et
  `ListChunkData::finalize()` revient tôt sur des décalages consécutifs. Un recul n'y est donc
  sûr que pour les chaînes.
- Un guetteur qui attend « FIN » dans un journal où l'on recopie un autre journal finissant par
  « FIN » s'arrête trop tôt. Prendre un marqueur unique (« TERMINÉ », « sonde remplace »).

## Le 11 octobre, l'après-midi

- **Les instructions BEGIN, COMMIT et CHECKPOINT s'exécutent sur un fil du planificateur**
  (l'opérateur Transaction ; deux fils au banc). Y attendre que d'autres transactions partent est
  une impasse, puisqu'elles ont besoin des mêmes fils pour partir. Une attente se fait dans le fil
  de l'appelant (`ClientContext::executeNoLock`), ou pas du tout.
- **Un témoin de concurrence se vérifie aussi contre sa propre satisfiabilité.** Une règle de
  relais (« valider seulement si un autre est ouvert ») empêche le point calme que la forme
  attend.
- **Un témoin qui doit rougir par mesure** : mesurer d'abord la série sur master et la série
  corrigée, puis choisir le plafond qui les sépare, et l'écrire avec les deux chiffres.
- **`poste push`** : pour un lot du moteur, une avance de master hors des chemins du bâti
  (`src cmake CMakeLists.txt test extension/vector extension/geo third_party scripts
  tools/rust_api`) ne demande pas de rejeu (page de G) ; le banc en `mesure` se joue hors du
  verrou.
- **gdb sous `ptrace_scope=1`** (luciepc) : on ne s'attache pas à un processus ; il faut lancer
  le test sous gdb, avec `timeout --signal=INT 25 gdb -batch -ex run -ex "thread apply all bt"`.

## Le 11 octobre, le soir

- **Un état d'index créé avant le verrou de l'index sert après lui.** Sous le mode multi-écrivains,
  `lockIndexForWrite` reprend l'instantané (`refreshSnapshot`) si la transaction n'a pas encore
  écrit ; tout ce qui a été dimensionné avant (ensemble visité, compteurs) doit se réajuster au
  début de l'opération.
- **`QUERY_VECTOR_INDEX` cherche une fois par état local** (`hasResultToOutput`) ; son état
  partagé compte les lignes à l'initialisation, après un SET en amont dans la même instruction.
- **Le témoin du concurrence_test** : les tests vector du banc (`vector_index_update_test.cpp`)
  sont dans `concurrence_test`, pas dans `transaction_test` (un filtre là rend « 0 tests »).
- **Une passe longue sur luciepc part détachée** (`setsid nohup bash script &`), son journal dans
  un fichier, le guetteur à part : une coupure ssh ne perd plus la sortie.
- **Un verdict se calcule sur toutes les lignes du fichier des codes**, y compris celles qui ne
  sont pas des suites (`FIN`) : les admettre explicitement.

## Le 12 octobre

- **Le catalogue est versionné, la liste des index du stockage ne l'était pas.** Un DROP retirait
  l'index pour tous avant de valider, une annulation ne le rendait pas, un CREATE l'ajoutait avant
  de valider. La rétention (en attente) donne au holder sa version, comme une ligne. La
  validation et l'annulation passent par `INDEX_CHANGE` (le crochet du catalogue n'a pas de
  contexte).
- **Un identifiant « stable » n'est stable que si chaque entrée le reçoit** : la clé primaire et
  les holders chargés du disque à 0 ont fait effacer la clé primaire par le DROP d'un autre.
- **Le point de reprise forcé du CREATE attend toutes les transactions** : un verrou que le CREATE
  tiendrait jusqu'à sa validation, et qu'un écrivain attendrait, le fait échouer au délai.
- **QUERY_VECTOR_INDEX complétait son résultat jusqu'à k** par des pseudo-lignes à
  `INVALID_OFFSET` : c'était la cause du compte faux du CALL et du plantage du masque.
- **finalizeDelete cherchait le graphe pour chaque voisin** (88 % du profil) ; `update` l'évitait
  par des heuristiques, désormais partagées.
- **`RAG3DB_API` jamais sur un membre d'une classe déjà exportée** : MSVC refuse, GCC laisse
  passer.
- **Un test SKIPPED n'est pas un vert** ; un filtre qui rend « 0 tests » non plus (les tests vector
  du banc sont dans `concurrence_test`).
- **perf n'est installé ni ici ni sur luciepc** ; le banc profile par eu-stack, avec
  `~/.cache/rag3db-banc-notes/ptracer-any.so` en LD_PRELOAD (`ptrace_scope=1`).

## Le 18 octobre

- **`cmake --build <dir> --target clean` efface la grammaire Cypher suivie par git**
  (`third_party/antlr4_cypher`, déclarée en `OUTPUT` dans l'arbre des sources), et le bâti
  suivant ne la régénère pas, faute de changement de `hash.md5`. Après un clean, faire
  `git checkout -- third_party`, puis exiger un status suivi vide.
- **Deux lots qui ajoutent un type d'annulation se disputent la même valeur.** Le rebase le
  montre comme un conflit simple ; il faut renuméroter l'un. Le tampon d'annulation n'est pas
  persisté, donc la valeur est libre, mais chaque `switch` sur `UndoRecordType` doit garder les
  deux cas.
- **Un push depuis luciepc** se fait par un script détaché (`setsid nohup`) qui pousse lui-même
  dans un `if grep -qx "verdict VERT"`, avec le refus du code moteur nouveau et les trois
  contrôles écrits au journal. Le `gh` de luciepc connaît aussi le compte sairen, inactif ; le
  push passe par ssh (L-Defraiteur).
- **Un « push code 0 » ne prouve pas qu'un commit est parti** : si le commit a échoué, HEAD égale
  master et le push n'envoie rien. Vérifier qu'il y a des commits au-delà de master avant de
  pousser.

## Le 19 octobre

- **Le point de non-retour d'un point de reprise de groupe** est la première écriture de colonne
  du groupe persistant (`ColumnChunk::checkpoint` par le fichier ombre, dans
  `checkpointInMemAndOnDisk`), juste après la copie fusionnée (`scanAllInsertedAndVersions`), et
  non l'échange des groupes. Une table de relations change sa part persistante avant son
  échange.
- **Une extension enregistrait ses fonctions avant de charger ses index** : une requête se liait
  dans la fenêtre, et la capture, qui ne retient que les holders chargés, concluait au retrait.
  Une fenêtre rare se rend déterministe par un crochet (une pause lue dans une variable
  d'environnement), pas par des boucles.
- **L'adressage d'une base** se réserve par tranches (`VMRegion`, 16 Gio), parce que les pages
  n'ont besoin d'être contiguës qu'à l'intérieur d'un groupe de cadres, et que les groupes
  s'attribuent dans l'ordre.
- **Un lot qui touche l'index, le COPY ou le chargement** passe `e2e_code` sur sa propre lib :
  `RAG3DB_LIB=bati` et `RAG3DB_BUILD=<son build/lecteurs-csv>`, jamais `run_e2e.sh --build`, qui
  publie au magasin et déplace `courant`. Il se prouve par la ligne « moteur lié ».
- **Pousser depuis luciepc** se fait par une chaîne détachée (lib, e2e_code, liste, push dans un
  `if`). Son rebase final bute souvent sur le README des tickets : on résout ici, on vérifie que
  le code est identique à celui rejoué (le diff du lot contre sa propre base), et on pousse par
  `push-lot.sh` avec la base rejouée.
- **Une retenue de tous les acteurs pendant un bâti, à 20 ms près**, était la famine du
  planificateur (le bâti prend tous les fils), pas un verrou.
