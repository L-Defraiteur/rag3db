# Cœur C++, correctifs et tickets — ce que la session sait

Ce qui est affirmé porte un fichier et une ligne (à `d2bd94630` ou `ba11e003b`) ou une mesure ;
le reste est dit « non vérifié ». Complète les relevés de la session cœur C++ (stèle) :
`../3-octobre-2026-23h31/coeur-cpp/02-knowledge-dump.md`.
**Dernière mise à jour : 11 octobre 2026, 0 h 30.**

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
