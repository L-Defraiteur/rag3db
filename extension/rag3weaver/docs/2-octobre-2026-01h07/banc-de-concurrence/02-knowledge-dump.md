# Banc de concurrence — knowledge dump, 2 octobre 2026

Ce que la session « banc » sait, orienté sur sa partie. Chaque point est étiqueté :
**exécuté** (vu en lançant), **lu** (vu dans le code), **déduit** (raisonné sans
exécution), **non su**.

## 1. Où sont les choses

Branche `banc-de-concurrence`, commit `00b2a0263` :

| fichier | rôle |
|---|---|
| `test/transaction/concurrence/bench_harness.h` | lanceurs, `Worker`, barrière, zone partagée, classement des refus |
| `test/transaction/concurrence/concurrency_bench_test.cpp` | fixture `ConcurrencyBench`, cas C0 à C9, triple vérification |
| `test/include/integrity/integrity_checker.h` + `test/transaction/concurrence/integrity_checker.cpp` | vérificateur de niveau 1, vidage canonique |
| `test/transaction/concurrence/known_red.txt` | rouges attendus, un nom gtest complet par ligne |
| `test/transaction/concurrence/compare_known_red.cmake` | la comparaison lancée par ctest |
| `test/transaction/concurrence/CMakeLists.txt` | la cible `concurrence_test`, les labels |
| `docs/2-octobre-2026-00h36/01-specification-du-banc-de-concurrence.md` | la spécification, avec les réponses de l'orchestration en §7 |

## 2. Comment le banc est fait

- **Un scénario, deux lanceurs.** Un cas s'écrit une seule fois, du point de vue d'un
  écrivain : `Scenario = std::function<void(Worker&)>`. `launch(mode, opener, area,
  scenario)` lance N écrivains :
  - en **Fil**, ce sont des `std::thread` sur une `Database` partagée, une
    `Connection` chacun ;
  - en **Processus**, ce sont des `fork()`, et chaque fils ouvre sa propre
    `Database` sur le même chemin. Le père doit avoir fermé la sienne avant
    (`conn.reset(); database.reset();`), sinon c'est le verrou de fichier qui
    répond.
- **La zone partagée** (`SharedArea`) est une projection `mmap(MAP_ANONYMOUS |
  MAP_SHARED)` créée avant les fils ou les processus. Elle ne contient que des
  atomiques et des tableaux fixes. `static_assert(std::atomic<uint32_t>::
  is_always_lock_free)`, parce qu'entre processus une atomique à verrou caché ne
  marche pas.
- **La barrière** est une génération et un compteur atomiques, en attente active
  (`std::atomic::wait` n'est pas garanti entre processus). Au bout de 30 s, elle
  est déclarée rompue : tout le monde passe, et le cas échoue (`BARRIER TIMED OUT`)
  au lieu de figer la passe.
- **Le journal par écrivain** (`WorkerReport`) compte les instructions réussies,
  les commits réussis et les refus **classés par leur message** : conflit
  d'écriture, clé en double, relations attachées, second écrivain, délai du point
  de reprise, verrou de fichier, inattendu. Il garde le texte de la première
  erreur inattendue. Toute erreur inattendue fait échouer le cas.
- **Les transactions explicites.** Le `Worker` tient `inTransaction` et
  `transactionFailed`. **Lu et exécuté** : après l'échec d'une instruction dans une
  transaction explicite, le moteur a déjà annulé la transaction, et un ROLLBACK
  ensuite répond « No active transaction for ROLLBACK ». `commit()` et `rollback()`
  envoient donc ce ROLLBACK sans le compter.
- **L'ordre des commits** est fixé par `commitInOrder({…})` : chaque écrivain
  valide à son tour, avec une barrière entre chaque.
- **Les cas déterministes** n'ont besoin d'aucun crochet dans le moteur : BEGIN,
  écriture, barrière, commits dans l'ordre. La fenêtre de course est tenue
  ouverte par les transactions elles-mêmes.
- **Graine et itérations** : `CONCURRENCE_GRAINE` (défaut 20261002, imprimée en tête
  de chaque cas) et `CONCURRENCE_ITERATIONS`. La graine fixe les choix des
  scénarios, pas l'ordonnancement des fils.
- **Le paramètre d'un test** est `{lanceur, moment}`, où le moment vaut `Hot`,
  `Reopen` ou `Crash`. Noms gtest :
  `Launchers/ConcurrencyBench.<Cas>/<Thread|Process>_<Hot|Reopen|Crash>`.
  - **Reopen** : vérification à chaud, vidage, `CHECKPOINT`, `createDBAndConn()`,
    vérification, comparaison des vidages.
  - **Crash** : le père ferme sa base et fait un `fork()`. Le fils ouvre la base,
    lance les écrivains en fils, écrit son vidage dans `<base>.hot-dump`, puis se
    tue par `SIGKILL`. Le père attend, rouvre (ce qui rejoue le journal), cherche
    les `<wal>.ecarte-*` (étape E du journal), qui feraient échouer le cas et sont
    nettoyés, puis vérifie et compare.
- **Les réglages communs**, posés par `applyBenchSettings` à chaque ouverture :
  `CALL debug_enable_multi_writes=true` et `CALL auto_checkpoint=false`. C8 rallume
  le point de reprise automatique.

## 3. Le vérificateur

- **Niveau 1, par Cypher** : il marche depuis n'importe quel processus. Le schéma
  est lu par `CALL show_tables()`, `CALL table_info('T')` (colonnes `name` et
  `primary key`) et `CALL show_connection('R')` (`source table name`,
  `destination table name`). Une table de relations à plusieurs paires de tables
  est refusée.
  - **Unicité de la clé primaire** : `count(n) = count(DISTINCT n.pk)`, et sinon la
    liste des clés en double.
  - **Relations** : un balayage depuis chaque côté.
    `MATCH (a:S) WITH a MATCH (a)-[r]->(b)` étend depuis a ;
    `MATCH (b:D) WITH b MATCH (a)-[r]->(b)` étend depuis b. Le sens a été
    **vérifié par EXPLAIN** (`SCAN_REL_TABLE` depuis le côté lié). Ne sont lus que
    `offset(id(r))`, `offset(id(a))`, `offset(id(b))` et les propriétés de r.
  - **Les clés des extrémités** viennent d'une lecture à part des lignes visibles
    (offset → propriétés). Une extrémité absente vaut `<missing>`.
  - Violations possibles : `primary-key-unique`, `rel-directions-agree` (relation vue
    d'un seul côté), `rel-endpoints-exist` (extrémité invisible),
    `rel-endpoints-correct` (`src_id` et `dst_id` portés par la relation, comparés
    aux clés réelles), et `same-answers` (le vidage diffère avant et après).
- **LA RÈGLE** : ne jamais lire une propriété d'une extrémité pour compter ou
  lister des relations. **Exécuté** : après C2, `count(r)` rend 1, mais
  `MATCH (a)-[r]->(b) RETURN a.id` ne rend rien, parce que lire `a.id` joint la
  table des nœuds et efface la relation pendante. Un vérificateur qui l'oublie
  rend un faux vert ; le premier l'a fait.
- **Le vidage canonique** contient toutes les lignes et toutes les relations, sans
  identifiant interne, triées. Les offsets peuvent changer au point de reprise :
  ils n'y sont pas.
- **Niveau 2, pas écrit.** Les API qu'il faudrait sont toutes **lues**, aucune n'a
  été essayée (cartographie d'un agent de lecture, chemins sur
  `banc-de-concurrence`) :
  - **transaction de lecture** : `conn->query("BEGIN TRANSACTION READ ONLY")`, puis
    `transaction::Transaction::Get(*conn->getClientContext())`, et `COMMIT`. Ne pas
    appeler `TransactionManager::beginTransaction` directement : il ne renseigne pas
    le `TransactionContext` ;
  - **tables** : `catalog::Catalog::Get(ctx)->getTableCatalogEntry(tx, nom)`, puis
    `storage::StorageManager::Get(ctx)->getTable(id)->cast<NodeTable>()`. Pour une
    relation, l'oid vient de
    `RelGroupCatalogEntry::getSingleRelEntryInfo().oid` ;
  - **lignes visibles** : boucler l'offset de 0 à `getNumTotalRows(tx)`, qui compte
    les supprimées, et filtrer par `isVisibleNoLock(tx, off)`, qui est borné, au
    contraire de `isVisible`. La clé se lit par `NodeTableScanState`,
    `initScanState` et `lookup` ;
  - **index** : `NodeTable::lookupPK(tx, &keyVector, 0, off)`. L'index **ne peut pas
    s'énumérer** sans modifier le code (membres privés, aucun itérateur). Les
    suppressions n'y sont pas physiques : une clé supprimée puis réinsérée a
    légitimement deux entrées, filtrées par la visibilité ;
  - **CSR** : `graph::OnDiskGraph` avec `prepareRelScan`, `scanFwd` et `scanBwd`
    (modèle : `test/storage/rel_scan_test.cpp`), qui ne filtre **pas** la
    visibilité des voisins. Une relation pendante y remonte donc ;
  - **témoin rouge du vérificateur** : `NodeTable::delete_(tx, NodeTableDeleteState
    (idVec, pkVec))` puis `finalizeDelete`, dans une transaction explicite validée
    par `COMMIT`. On supprime ainsi un nœud sans le contrôle `throwIfNodeHasRels`
    de l'exécuteur, et on fabrique à coup sûr une relation pendante ;
  - **statistiques** : `getStats(tx).getTableCard()` n'est jamais décrémentée et
    `RelTable::getNumTotalRows` compte les offsets réservés : aucun des deux n'est
    un compte de lignes vivantes.

## 4. Construire et lancer

- Worktree `../rag3db-banc`, build `build/release` :
  `cmake -S . -B build/release -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTS=TRUE
  -DBUILD_SHELL=FALSE -DBUILD_BENCHMARK=OFF`.
- **Toujours `cmake --build build/release -j 8 --target …`.** Dans le fish du poste,
  `ninja` est un alias vers `ninja -j32`. `/usr/bin/time` n'existe pas : mesurer avec
  `date +%s`.
- **Durées exécutées** : `transaction_test`, cœur compris, se bâtit en 240 s. Le
  banc seul se recompile en quelques secondes. La passe complète de l'étape 2
  prend 316 s, dont C8 presque tout ; sans C8, environ 1 s.
- **ctest** se lance depuis `build/release/test`, pas depuis `build/release` (le
  Makefile du dépôt fait pareil).
  - Les rouges connus portent le label `concurrence-rouge-connu`, au moyen de deux
    appels `gtest_discover_tests` avec `TEST_FILTER` : l'un les exclut, l'autre les
    prend avec le label.
  - `concurrence_test.known_red` lance tout le banc avec `--gtest_output=json`, et
    échoue sur tout écart avec `known_red.txt` : nouveau rouge, rouge connu devenu
    vert, rouge connu sauté ou absent.
- **Piège exécuté** : `file(STRINGS)` sans `ENCODING UTF-8` coupe les lignes sur
  les caractères accentués. Les commentaires de la liste devenaient de faux noms
  de tests.
- **Piège exécuté** : un `QueryResult` ne doit pas survivre à sa `Database`. Un
  résultat de `CHECKPOINT` gardé au-delà de `createDBAndConn()` a provoqué un
  SIGSEGV dans `~FactorizedTable`.
- **Piège** : le nom d'un test paramétré contient des « / ». Le chemin de base de
  test en fait des sous-dossiers, que le fixture retire dans `TearDown`.

## 5. Ce que j'ai appris du moteur

- **`debug_enable_multi_writes`** (lu, exécuté) ne s'allume que par
  `CALL debug_enable_multi_writes=true`. C'est un champ de `DBConfig`, pas de
  `SystemConfig` : il ne passe pas à l'ouverture et ne survit pas à une
  réouverture. Son seul effet est de sauter le refus du second écrivain
  (`transaction_manager.cpp:35`). **Aucune** détection de conflit en plus.
- **La visibilité** (lu, exécuté par C1 et C2) :
  - `validatePkNotExists` ne cherche que dans l'index global, filtré par
    `isVisible(transaction)` ; la clé non validée d'une autre transaction, dans
    son `LocalStorage`, est invisible ;
  - `DELETE` contrôle les relations par `throwIfNodeHasRels`, sur l'instantané du
    supprimeur ;
  - rien n'est revérifié au commit.
- **Les conflits qui existent** (lu, exécuté) : deux mises à jour de la même ligne,
  le premier écrivain gagne (« Write-write conflict », C4 en a vu 256) ; deux
  suppressions de la même ligne (C5, vert). **Il n'y en a pas** entre une
  suppression et une mise à jour (C6, exécuté), ni pour les insertions (C1).
- **Les commits sont sérialisés** (lu) : `commit` prend
  `mtxForSerializingPublicFunctionCalls`.
- **Le rejeu du journal** (exécuté, phase Crash) rejoue les doublons de C1 sans
  rien refuser, et rend les mêmes réponses qu'avant l'arrêt. Aucun fichier
  `.ecarte-` après des commits acquittés.
  **Faux, corrigé le 3 octobre au soir** : la phase Crash fermait la base avant le
  SIGKILL et ne rejouait rien. Avec un vrai arrêt brutal, le rejeu refuse le doublon
  de C1 et la base ne se rouvre plus. Voir la spécification du banc, §12.
- **Le point de reprise sous écrivains** (exécuté, C8) : avec le délai de 5 s,
  la plupart des CHECKPOINT expirent, et les écrivains restent gelés pendant
  l'attente (déduit de la durée, environ 17 × 5 s). Les commits des écrivains
  n'ont, eux, reçu aucun refus. Le délai est privé dans `TransactionManager`
  (`setCheckPointWaitTimeoutForTransactionsToLeaveInMicros`, réglé par le runner
  des tests `.test` en ami de la classe).
- **C4** (exécuté) : sous au moins trois écrivains libres, une transaction qui
  échoue sur sa seconde mise à jour garde souvent la première. Aucune reproduction
  déterministe : ni à deux écrivains, ni en barrières. **Mécanisme non su.**
- **Le second processus écrivain** (exécuté, C9) est refusé à l'ouverture avec
  « Could not set lock on file ».
- **Le test amont `ConcurrentRelationshipUpdatesWithMixedTransactions`** est juste
  (exécuté).

## 6. ThreadSanitizer

- **Lu** : `option(ENABLE_THREAD_SANITIZER)` (CMakeLists.txt:117, 272-278),
  `make … TSAN=1` dans le Makefile. Il n'y a pas de cible `make tsan`, et aucun
  fichier de suppression dans l'arbre.
- **Prévu, rien de fait** : un build à part (`build/tsan`, `-j8`), le lanceur Fil
  seulement (`fork` avec des fils actifs n'est pas sûr sous TSan), les itérations
  réduites par `CONCURRENCE_ITERATIONS`. Pas de fichier de suppression pour le
  moteur : toute course trouvée est une constatation. Candidats : C4 et C5 (le
  chemin de suppression sans verrou, marche A5).
- **Non su** : la durée du build TSan et le ralentissement du banc.

## 7. Ce qui reste ouvert

- La décision de Lucie, transmise par l'orchestration : **l'écrivain attendra sur
  un verrou par clé au lieu d'échouer**. Les invariants de C1 à C3 ne bougent pas,
  le comportement attendu change : le second écrivain attend, puis reçoit l'erreur
  ordinaire. Trois cas à écrire après la note de la session cœur C++ :
  - le premier écrivain annule : le second réussit ;
  - le premier valide : le second reçoit la clé en double ;
  - interblocage : erreur nommée dans un délai borné.

  Il faut un délai de garde côté banc, pour qu'un moteur qui se bloque donne un
  rouge et pas une passe sans fin.
- HNSW : hors du banc A1 (décision de l'orchestration), non étudié.
- Le sort de C7 (probabiliste) et la durée de C8 : à trancher.
