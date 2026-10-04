# Banc de concurrence — ce que la session sait

Mis à jour le 4 octobre 2026. Étiquettes : **[exécuté]** vu en lançant ;
**[lu]** vu dans le code ; **[déduit]**.

## 1. Les fichiers

Tous sous `test/transaction/concurrence/`, dans une seule cible, `concurrence_test` :

| fichier | rôle |
|---|---|
| `bench_harness.h` | lanceurs (fils, processus), `Worker`, barrière, journal d'événements, délai de garde, classement des erreurs, `probeOpenInFreshProcess`, `disableCoreDumps`, `loadVectorExtension` |
| `concurrency_bench_test.cpp` | `ConcurrencyBench` : C0 à C9 et H1 à H4, sous Hot / Reopen / Crash, en fils ou en processus |
| `integrity_checker.cpp` (en-tête `test/include/integrity/`) | vérificateur de niveau 1 (Cypher), de niveau 2 (internes), invariant de l'index vectoriel, vidage canonique, adjacence stockée |
| `minimal_reproduction_test.cpp` | reproductions sans harnais, dans un seul fil : C4, C6, la transaction en échec, deux colonnes, l'index sur cent lignes |
| `integrity_checker_test.cpp` | témoins rouges du vérificateur : relation pendante fabriquée par `NodeTable::delete_`, côté source et côté destination |
| `harness_mechanics_test.cpp` | la mécanique éprouvée sur elle-même : ordre des événements, délai de garde |
| `lock_bench_test.cpp` | témoins des verrous (§6 de la note), verrou d'index, réouverture de l'index, second témoin d'A3′ |
| `single_writer_crash_test.cpp` | arrêt brutal avec un seul écrivain, reprise avec un index d'extension, `OpenProbe` |
| `vector_index_update_test.cpp` | l'index vectoriel juste en service : mise à jour de vecteurs, chemin du produit, ligne lointaine ; essais répétés |
| `upstream_fixes_test.cpp` | les correctifs de Ladybug et de Vela qui nous manquent ; les cas qui plantent tournent dans un fils qui sort base ouverte |
| `long.txt` | les cas de plus de vingt secondes, hors de la passe par défaut (`CONCURRENCE_LONG=1`) |
| `uncommitted_relations_test.cpp` | relations supprimées puis créées dans une transaction, relues et réécrites (verts depuis `c8fdaf196`) |
| `known_red.txt`, `probabilistic.txt` | les rouges attendus, avec leurs étiquettes et leur marche ; les cas probabilistes |
| `compare_known_red.cmake`, `compare_tsan_signatures.py`, `tsan_signatures.txt` | les deux comparaisons |

## 2. Le harnais

- **Un scénario, deux lanceurs.** Les fils partagent une `Database` ; les processus,
  créés par fork, ouvrent chacun la leur. Les deux partagent une zone `mmap(MAP_SHARED)`
  faite d'atomiques sans verrou : comptes rendus, barrière, événements, drapeau.
- **Le `Worker`.**
  - Il classe chaque erreur par son message.
  - Il n'envoie plus rien après un échec dans une transaction. Le moteur a annulé, et
    l'instruction suivante partirait en auto-commit ; c'est la cause du faux rouge de C4.
  - `runMarked`, `mark`, `waitFor` et `waitForAny` permettent de prouver l'ordre réel.
  - `writeInTurn` enchaîne les écritures, pour éviter les écritures simultanées qui font
    planter le moteur avant A5.
  - `commitInOrderByEvents` fait valider les écrivains dans un ordre donné, sans
    barrière : prêt pour « le second attend ».
- **Le délai de garde**, 60 s par défaut : il interrompt les connexions ou tue les
  processus. Si un fil reste bloqué, le processus de test s'arrête, et la passe finit en
  rouge plutôt que de ne pas finir. **[exécuté]** L'interruption n'est pas regardée
  pendant un `range()` géant.
- **Les erreurs reconnues.** Les fragments sont convenus avec le cœur C++ :
  - « deadlock » ; « lock » et « timeout » ; « could not serialize » ; « Interrupted » ;
  - pour la garde 1, « is behind its table » ;
  - une fonction ou un réglage inconnu (`Invalid option name`, `function … does not
    exist`) est classé **en premier**, puisque son message contient le nom demandé.

## 3. La variante Crash et sa correction

- **[exécuté] La faute.** Jusqu'au 3 octobre au soir, le fils déclarait sa base dans un
  bloc `try` et se tuait après ce bloc. Le destructeur avait donc fermé la base, avec un
  point de reprise final, et aucun journal n'était rejoué.
- **Comment elle a été trouvée.** Le cas du `DROP_VECTOR_INDEX` rejoué restait vert,
  alors que la recette de l'arbre principal rougissait. La comparaison des deux formes a
  montré la différence.
- **La correction.** Le SIGKILL part maintenant dans la portée de la base, et un témoin
  (`journal-to-replay`) exige un `.wal` non vide avant de rouvrir.
- **Ce qui a changé** (audit au §12 de la spécification) :
  - C1 : la base ne se rouvre plus ;
  - C7 : rouge certain ;
  - C2 avec `DETACH DELETE`, relation validée d'abord : le rejeu répare la relation
    pendante.

## 4. Rouvrir dans un processus neuf

- **[exécuté] Pourquoi.** Un processus qui a déjà chargé l'extension vector la garde
  chargée. Le rejeu y trouve l'index, et la base s'ouvre, là où un service qui redémarre
  plantait. Les variantes Crash de H1 à H3 étaient vertes pour cette raison.
- **Comment.** `probeOpenInFreshProcess` copie la base, fait un fork puis un `execv` du
  banc lui-même sur `OpenProbe.OpenFromEnvironment`, et lit le signal.
- **Ce qu'elle rapporte.** Seuls les plantages, sous l'étiquette
  `database-opens-without-crash`. Une erreur à l'ouverture reste `database-reopens`,
  nommée par la réouverture ordinaire.

## 5. Les familles de cas, et ce qu'elles prouvent (master, 3 octobre au soir)

- **Mode multi-écrivains** (allumé seulement dans le banc).
  - C1 à C3, C6 : les corruptions de l'écriture parallèle, rangées sous A3′ et A4′.
  - C4 : vert, puisque c'était un défaut du banc.
  - C5 : vert depuis A5, sur 600 essais.
  - C7 : probabiliste à chaud ; certain après arrêt brutal (base perdue).
  - C8 : le point de reprise sous écrivains gèle, et le délai est privé.
  - C9 : le second processus écrivain est refusé.
- **L'index HNSW.**
  - H1 : vert depuis A2.
  - H2 : vert.
  - H3 : deux suppressions sans rapport entre elles, aux voisinages qui se recouvrent,
    reçoivent un « Write-write conflict ».
  - H4 : probabiliste ; plantages, corruption du tas, base qui ne se rouvre plus.
  - L'index construit sur 98 ou 100 lignes de `embeddings-8-1k.csv` perd le nœud 13, sans
    aucune concurrence. C'est un cas limite des petits index.
- **Les témoins des verrous** (`lock_bench_test.cpp`), sous V1, A3′, A4′ et V2.
  - L'attente se prouve par l'ordre des événements.
  - Deux rouges disent seulement qu'une fonction n'existe pas encore : `acquire_locks` et
    `lock_timeout`.
  - Le nœud-carrefour est vert, et doit le rester.
  - La réouverture de l'index est verte.
  - Le verrou d'index (auto-commit, bloc, annonce) est rouge.
- **L'arrêt brutal avec un seul écrivain** : vert pour les enregistrements de plus de
  4 Ko, la fin de journal déchirée, la mort à cinq instants d'un point de reprise, le
  point de reprise échoué, le COPY tout ou rien, et les suppressions d'A5 sur un index
  d'une session précédente.
- **La reprise avec un index d'extension.**
  - Condition : le journal ne porte plus le `LOAD EXTENSION`. Avant la garde 1, la base
    faisait planter le processus qui l'ouvrait.
  - La garde 1 (`fcd9a7882`) l'ouvre, et marque l'index « à rebâtir ».
  - Restent les témoins de la garde 2 : l'index juste sans rebâtir.
  - Quand le journal porte encore le chargement, le rejeu tient l'index à jour : vert.

## 5 bis. L'index vectoriel juste en service (4 octobre)

- **[exécuté] Le compte des pertes n'est pas stable.** La même recette, sur le même
  binaire, donne 779, 593, 772 lignes joignables d'une passe à l'autre. Un chiffre d'une
  passe ne vaut qu'en ordre de grandeur. Les témoins répètent donc des essais sur des
  tables neuves, et rougissent si un seul perd une ligne.
- **[exécuté] La recherche « exhaustive » dépend de son point d'entrée.** Dans un essai,
  790 lignes sortaient premières sur leur propre vecteur, et 772 seulement étaient
  joignables depuis la ligne 0. D'où les deux contrôles, sous une seule étiquette
  (`vector-index-exact`), puisque la même perte se voit par l'un, par l'autre ou par les
  deux.
- **[exécuté] Les résultats de `QUERY_VECTOR_INDEX` ne sont pas triés par distance**, et
  `distance` ne se lit pas en `double` (passer par `toString`). Ce sont deux fautes de la
  sonde, corrigées avant de rien conclure.
- **[exécuté] État après `c8fdaf196`.**
  - Verts : le chemin du produit (depuis NULL, par lots de 32, remplacement qui repasse
    par NULL ligne à ligne, second index, `DROP` puis `CREATE`), vingt lignes vers le
    même vecteur, 768 dimensions, la transaction annulée.
  - Rouges, environ un essai sur deux (33 à 90 % selon le cas) : les autres mises à jour.
  - Rouge sans aucune mise à jour : une ligne lointaine dans un nuage serré, un essai
    sur trois.
- **[exécuté] Coût.** La mise à jour est cinq à soixante fois plus lente depuis
  `c8fdaf196`. Les essais par cas sont réglés pour que la comparaison reste sous le quart
  d'heure ; `CONCURRENCE_VECTOR_RUNS` les remplace tous.

## 5 ter. La revue des amonts (4 octobre)

- **[exécuté] Une recette déduite d'un diff n'est pas un défaut.** Sur une trentaine de
  recettes proposées par les agents, vingt se sont confirmées chez nous ; deux n'ont pas
  rougi. Le cas Vela du lecteur retenu ne s'est vu qu'avec un lecteur plus long que le
  délai d'attente du test.
- **[exécuté] Un fils qui plante laisse une base que la fermeture fait planter à nouveau.**
  Une base rouverte dans le processus du test fait un point de reprise à la fermeture, et
  ce point de reprise plante lui aussi. Les témoins font donc tout dans des fils qui
  sortent base ouverte (`_exit` dans la portée de la base).
- **[lu] Les tags suffisent.** `ladybug-main-2026-08-31` a une histoire séparée de la
  nôtre : on la lit par dates. Vela, par `git log HEAD..vela-master-2026-09-03`.

## 5 quater. Les correctifs (4 octobre)

- **[exécuté] La TCK a refusé une heuristique.** Reconnaître un nœud lié sans étiquette à
  ce qu'il couvre toutes les tables cassait `tck/match/match3` Scenario26 : `(a1:X:Y)`
  couvre toutes les tables par ses étiquettes. Un marqueur posé à la création du nœud
  (`NodeExpression::isBoundWithoutLabel`) l'a remplacée.
- **[lu] Un correctif de l'amont peut avoir son propre trou.** Pour l'`OPTIONAL MATCH`
  doublé, Ladybug choisit le graphe porteur du balayage corrélé avant de sauter les
  graphes vides, et pourrait choisir celui qu'il saute. Nous choisissons après.
- **[exécuté] Un test de notre dépôt peut poser un contrat qu'un correctif de l'amont
  contredit.** C'est le cas du champ CSV `""`, que `escaped_newlines.test` voulait NULL.
  On s'est arrêté pour rendre la décision ; Lucie a choisi la chaîne vide, et le test a
  changé d'attendu dans le commit du correctif.
- **[exécuté] Un témoin doit discriminer : le jouer sans le correctif.** Le cas `SERIAL`
  du `MERGE` cherchait un nœud par un identifiant inexistant et comptait 0 sans le
  correctif comme avec. Réécrit pour comparer les identifiants rendus, il rougit sans et
  verdit avec. Une prédiction (le `SET` après un `MERGE` de nœud hors clé) ne s'est pas
  vérifiée ; elle reste comme garde-fou, et le commit le dit.
- **[lu] Le `MERGE` d'un motif créé dans le même lot** passe par la table des motifs
  créés, parce que l'existence est calculée avant l'écriture. Cette table garde
  désormais un identifiant par insertion (`map_merge.cpp`, `merge.cpp`).
- **[exécuté] La liste C++ (`build/liste-cpp.sh` du worktree, reprise de celle du cœur
  C++)** dure environ 25 minutes. Le banc y relit `known_red.txt` depuis les sources : n'y
  touchez pas pendant qu'elle tourne, ou la comparaison se fait contre un fichier qui ne
  correspond plus au binaire.
- **[exécuté] Les tests e2e lancés depuis la racine du worktree** y laissent
  `follows.csv`, `user.csv` et `user.parquet` : à supprimer après chaque passe.

## 6. Ce qui n'est pas atteignable sans crochet dans `src/`

- **La mort à chaque allocation pendant la phase de stockage d'un point de reprise.**
  Le `DyingCheckpointer` ne meurt qu'avant ou après une des quatre phases virtuelles de
  `Checkpointer`. Signalé au cœur C++.
- **Le délai d'attente du point de reprise** (C8). Il se règle par
  `TransactionManager::setCheckPointWaitTimeoutForTransactionsToLeaveInMicros`, qui est
  privé.
- **Ce que contient le graphe HNSW.** Le banc ne voit que ce que rend une recherche. Une
  ligne mise à jour pourrait garder son ancienne position dans le graphe sans que le
  contrôle le voie, si la recherche calcule ses distances sur les vecteurs de la table.

## 7. Les fautes du banc, et comment elles ont été trouvées

| faute | effet | trouvée par |
|---|---|---|
| lecture d'une propriété de l'extrémité dans le vérificateur | faux vert sur C2 | une sonde sur l'état de la base |
| `WITH` qui ne force pas le sens du balayage | faux vert sur la destination supprimée | la nouvelle variante de C2, puis `EXPLAIN` |
| instructions envoyées après un échec en transaction | faux rouge sur C4 (de l'argent créé) | des reproductions sans harnais |
| `file(STRINGS)` sans `ENCODING UTF-8` | les commentaires accentués devenaient de faux noms de tests | la comparaison qui échouait |
| répertoires intermédiaires supprimés en parallèle | course entre processus de test (déduite) | relecture |
| `REGEX REPLACE` ancré par `^` | les étiquettes attendues effacées | la comparaison qui échouait |
| variante Crash qui fermait la base | aucun rejeu ; faux verts et faux constats | un cas vert là où une autre recette rougissait |
| réouverture dans le processus du test | défaut d'ouverture invisible | l'extension restée chargée, comprise en comparant deux sondes |
| comptage des vecteurs mis à jour | faux rouge (« mises à jour perdues ») | relecture des valeurs avant et après la mort |

La règle qui en sort : un rouge imprévu se reproduit sans harnais avant d'accuser le
moteur, et un vert inattendu, quand une autre forme rougit, se compare à cette forme
avant d'innocenter le moteur.

## 8. Mesures utiles

- Build : `transaction_test`, cœur compris, 240 s ; l'extension vector, 10 s ; le
  build TSan, 314 s.
- La comparaison `known_red` par défaut : 2 min 17 ; avec les cas longs : 29 min. Les dix
  mille lignes : 12 à 27 min.
- Un fils qui plantait en écrivant un vidage mémoire coûtait 20 à 27 s ; les fils du
  banc n'en écrivent plus.
