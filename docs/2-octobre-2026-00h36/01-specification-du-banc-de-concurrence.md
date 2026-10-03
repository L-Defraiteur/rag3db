# Spécification du banc de concurrence et du vérificateur d'intégrité (A1)

**2 octobre 2026, rag3db-19.** Rien n'est bâti ni exécuté : tous les « attendu » sont déduits du code.

LU (sur origin/master aa7d64484) : le plan du 2 octobre (en entier), la relecture MVCC du 6 septembre, le journal, transaction_test.cpp (les neuf Concurrent*) et lecteurs_concurrents_test.cpp (cartographiés par un agent). J'ai vérifié moi-même node_table.cpp:380-403, delete_executor.cpp et in_mem_hash_index.h:284-296.

Faits qui orientent le banc :
- debug_enable_multi_writes ne s'allume que par CALL : c'est un champ de DBConfig, pas de SystemConfig. On ne peut pas le passer à l'ouverture, et il ne survit pas à une réouverture. Son seul point de lecture est transaction_manager.cpp:35.
- validatePkNotExists ne cherche que dans l'index global, filtré par isVisible(transaction). La clé non validée d'une autre transaction vit dans son LocalStorage, donc elle est invisible.
- DELETE d'un nœud vérifie les relations par throwIfNodeHasRels, mais sur l'instantané du supprimeur. Une relation non validée de l'autre transaction lui est invisible.
- Les neuf Concurrent* utilisent tous des clés disjointes, sans réouverture ni CHECKPOINT. Remarque au passage, non vérifiée : ConcurrentRelationshipUpdatesWithMixedTransactions (l.639) valide si i%3!=0, soit les fils 1, 2 et 3, ce qui ferait 3000 et pas les 2000 attendus. Soit le test ne fait pas ce qu'on croit, soit il est faux. À regarder en le lançant une fois.

## 1. La forme du banc

Nouvel exécutable test/transaction/concurrence/, déclaré par add_rag3db_test(concurrence_test …), pour avoir accès aux internes (niveau 2 du vérificateur). Fixture dérivée d'EmptyDBTest, base sur disque ; le mode mémoire est sauté, avec la raison écrite.

Chaque cas s'écrit une seule fois, sous la forme d'un « scénario » : une fonction scénario(Ouvreur&, int numeroEcrivain, Journal&). Elle obtient sa connexion par l'Ouvreur et range ses issues dans le Journal. Le lanceur est un paramètre du test (TEST_P sur {Fil, Processus}) :
- Fil : N std::thread, une Database partagée, une Connection par fil.
- Processus : N fork() faits AVANT toute ouverture de Database dans le père (règle de lecteurs_concurrents_test.cpp). Chaque fils ouvre sa Database sur le même chemin et sort par _exit. Le Journal est alors un tableau en mmap MAP_SHARED, avec une case par écrivain.
Le Journal compte, par écrivain : les succès, les refus attendus classés par message (« Write-write conflict », « duplicated primary key », « connected edges », « Cannot start a new write transaction », « Could not set lock ») et les erreurs inattendues, texte gardé. Toute erreur inattendue fait échouer le cas : un refus n'est jamais compté vert sans que son message soit lu.

La synchronisation se fait par une barrière : std::barrier en mode fil, compteur atomique sur la page partagée en mode processus. Les cas déterministes n'ont donc besoin d'aucun crochet dans le moteur : la fenêtre de course est tenue ouverte par des transactions explicites (BEGIN … barrière … COMMIT).

Les cas aléatoires tirent leurs nombres d'une graine imprimée en tête, rejouable par variable d'environnement (CONCURRENCE_GRAINE), et d'un nombre d'itérations réglable (CONCURRENCE_ITERATIONS).

Réglages de base : CALL debug_enable_multi_writes=true, auto_checkpoint=false pendant la phase à chaud. Le point de reprise automatique dans un commit attendrait les autres transactions et expirerait au bout de 5 s, ce qui mesurerait autre chose. Les points de reprise sont donc explicites, dans les cas qui les veulent.

Mode Processus aujourd'hui : le second écrivain est refusé à l'ouverture (F_WRLCK). Je propose :
(a) un cas qui affirme ce contrat tel qu'il est : refus franc, message lu ;
(b) les cas de conflit sautés en mode Processus, avec la raison « B non construite ». Ces sauts sont comptés et affichés comme sauts, jamais comme verts.
Le jour où B existe, on retire le saut ; on ne réécrit rien.

Triple vérification de chaque invariant (plan §11) :
- à chaud, sur la base ouverte ;
- après CHECKPOINT, fermeture et réouverture ;
- après arrêt brutal puis rejeu : le scénario tourne dans un fils, SIGKILL après le dernier commit acquitté, puis le père rouvre. Cette dernière passe utilise le lanceur Processus même pour les cas « fil » : les écrivains sont des fils à l'intérieur d'un seul processus fils.

## 2. Les cas
(rouge = j'attends une corruption aujourd'hui ; *déduit* partout, rien n'est exécuté)

**C0** — Témoin, clés disjointes. N écrivains insèrent des nœuds et des relations sur des domaines disjoints.
- Invariants : tout le vérificateur.
- Attendu : VERT. Il prouve que le banc et le vérificateur ne crient pas sans raison.

**C1** — Même clé primaire, déterministe. Deux écrivains : BEGIN, CREATE (:K {id: 7}), barrière, COMMIT tous les deux. Variante à trois écrivains.
- Invariant : exactement un succès ; count = count(distinct id) ; une seule entrée d'index par clé.
- Attendu : ROUGE (deux succès, deux lignes « 7 »). Le plan §2 et la marche A3 tiennent à ce cas.

**C1'** — Même clé primaire, sous charge. N fils en auto-commit tirent leurs clés dans un petit domaine (K = 16) pendant un temps fixe.
- Invariant : count = count(distinct).
- Attendu : ROUGE probable, mais pas garanti à chaque passe, la fenêtre étant courte. C'est C1 qui fait foi.

**C2** — Relation vers un nœud supprimé, déterministe. Nœuds a et b déjà validés. T1 : BEGIN, MATCH (a) DELETE a, barrière, COMMIT. T2 : BEGIN, MATCH a, b CREATE (a)-[:R]->(b), barrière, COMMIT. Deux ordres de commit (T1 puis T2, T2 puis T1).
- Invariant : aucune relation pendante ; les directions avant et arrière portent les mêmes arêtes.
- Attendu : ROUGE dans les deux ordres. C'est la marche A4.

**C2'** — Même chose sous charge : des supprimeurs contre des créateurs de relations sur un petit ensemble de nœuds.
- Attendu : ROUGE probable.

**C3** — Offsets locaux qui se chevauchent (A2). Deux écrivains, chacun dans une transaction explicite : il crée M nœuds neufs, puis des relations entre ses propres nœuds neufs, chaque relation portant src_id et dst_id en propriétés ; barrière ; COMMIT.
- Invariant : pour chaque relation, les clés des vraies extrémités sont égales à src_id et dst_id.
- Attendu : ROUGE (relations vers les nœuds de l'autre). Il n'y a ni remappageNodeOffsets chez nous, ni aucune erreur.

**C4** — Virements. Comptes à solde, transferts aléatoires, réessai sur « Write-write conflict ».
- Invariant : la somme ne bouge pas ; aucun solde ne vient d'une mise à jour perdue.
- Attendu : VERT, puisque le conflit de mise à jour existe (update_info.cpp:31-38). Non vérifié.

**C5** — Double suppression de la même ligne, avec barrière.
- Invariant : exactement un succès, l'autre refusé avec « Write-write conflict ».
- Attendu : VERT fonctionnellement. Sous TSan, ROUGE possible : création paresseuse de versionInfo sans verrou, marche A5.

**C6** — Suppression contre mise à jour de la même ligne, avec barrière, dans les deux ordres.
- Invariant : la ligne est soit supprimée, soit mise à jour, jamais une ligne supprimée qui porte la mise à jour.
- Attendu : INCONNU (plan §13, non examiné). Le banc tranche.

**C7** — Mélange aléatoire : insertions de clés en petit domaine, suppressions (simples et DETACH), relations, mises à jour, rollbacks volontaires.
- Invariants : tout le vérificateur.
- Attendu : ROUGE tant que C1 et C2 le sont. C'est le filet des marches suivantes.

**C8** — Point de reprise sous écrivains : un fil lance CHECKPOINT en boucle pendant que d'autres écrivent sur des clés disjointes.
- Invariant : les refus de point de reprise sont lus (délai d'attente) ; la base reste intègre.
- Attendu : VERT pour l'intégrité, avec des refus de délai attendus. C'est le futur témoin de A7.

**C9** — Contrat inter-processus d'aujourd'hui, mode Processus seulement : un second écrivain est refusé à l'ouverture, avec « Could not set lock ».
- Attendu : VERT. C'est lui qu'on retournera avec B.

## 3. Le vérificateur d'intégrité

Une bibliothèque de test (test/include/integrity/…) et non une fonction du moteur pour A1. Je pose plus bas la question d'en faire un CALL check_integrity(). Elle prend une Connection, ou une Database, et rend une liste de violations nommées. Une liste vide veut dire vert ; chaque violation sort avec la ligne fautive.

Niveau 1, par Cypher (API publique, marche dans un fils et sur une base rouverte en lecture seule) :
- Pour chaque table de nœuds, lue par CALL show_tables() et table_info : count(n) = count(DISTINCT n.pk), et la liste des clés en double si ce n'est pas le cas.
- Pour chaque table de relations : les extrémités portées en propriétés (src_id, dst_id, quand le schéma du banc les a) égales aux clés des vraies extrémités.
- Un vidage canonique (toutes les lignes, toutes les relations, triées) haché. Il est comparé à chaud, après point de reprise et réouverture, et après arrêt brutal et rejeu : ce sont les « mêmes réponses avant et après ».

Niveau 2, par les internes (PrivateGraphTest et getStorageManager), sous une transaction de lecture fraîche :
- PK : pour chaque ligne visible de chaque NodeTable, le lookup de sa clé dans l'index global rend exactement cet offset. Aucune clé de l'index ne mène à une ligne invisible ou absente.
- Relations : balayage CSR avant et arrière, en multiensembles de (src, dst, relID) ; les deux doivent être égaux. Chaque extrémité doit être une ligne visible de sa table.
- Compteurs : le nombre de lignes des statistiques égale le balayage.
- Pages : on réutilise FSMLeakChecker::checkForLeakedPages, déjà présent côté test.
Le niveau 2 voit ce que Cypher ne voit pas : une relation pendante n'apparaît pas dans un MATCH qui part du nœud supprimé, mais elle reste dans la CSR de l'autre direction.

## 4. ThreadSanitizer et durée

- Build dédié dans le worktree : make relwithdebinfo TSAN=1, répertoire séparé (build/tsan), -j8. Pas de cible make tsan ; ENABLE_THREAD_SANITIZER existe (CMakeLists.txt:117, 272-278).
- Sous TSan, seul le lanceur Fil tourne : fork avec des fils actifs n'est pas sûr sous TSan. Itérations réduites par variable d'environnement.
- Aucun fichier de suppression n'existe. Je n'en crée pas pour le moteur : toute course trouvée est une constatation, rapportée. Une suppression ne viendrait que pour une bibliothèque tierce, chacune nommée et justifiée.
- Durées visées, non mesurées : banc normal sous 60 s pour l'exécutable entier, cas déterministes en millisecondes ; sous TSan, sous 10 min. Le build TSan de tout le moteur est lui aussi à mesurer. Je l'estime à au moins une heure à -j8, sans base pour le dire.

## 5. Ce que je ne sais pas

1. Si le commit d'une transaction relit l'index global avec une visibilité qui lui ferait voir une clé validée après son départ. Si oui, C1 est vert et le plan se trompe. Le banc le dira : c'est le but.
2. Les API internes exactes pour balayer une NodeTable et une CSR depuis un test (niveau 2). À cartographer avant de coder.
3. Si une relation pendante fait planter une requête (lecture d'un offset invalide) plutôt que de passer en silence. Cela change la forme de l'échec, pas l'invariant.
4. Le comportement de throwIfNodeHasRels dans C2, quand T2 valide d'abord sa relation : T1 la voit-il à son commit ? Probablement non, puisque la vérification est faite à l'écriture et pas au commit.
5. Le coût réel du build TSan et le ralentissement du banc.
6. HNSW : hors du banc A1, sauf avis contraire (plan §13, non étudié).

## 6. Questions

- **Q1** Un CALL check_integrity() dans le moteur (code neuf, pas une correction), ou une bibliothèque côté test seulement pour A1 ? Je propose la bibliothèque côté test d'abord, et le CALL à une marche suivante.
- **Q2** Comment les cas rouges entrent dans ctest. Ils restent rouges, jamais relâchés. Je propose un label ctest « concurrence-rouge-connu » et une ligne au journal (§6) qui les nomme, pour qu'une passe complète ne les confonde pas avec une régression. Ou bien les laisser rouges dans la passe par défaut, à ton choix.
- **Q3** Le cas C8 et le point de reprise automatique : je le coupe partout ailleurs. D'accord ?
- **Q4** Une ligne au journal des chantiers dans mon premier commit (règle 1) : je la mets sur ma branche ?

## 7. Réponses de l'orchestrateur (2 octobre)

- **Q1** : bibliothèque côté test pour A1 ; le `CALL check_integrity()` viendra à une marche suivante, s'il vient.
- **Q2** : les cas rouges tournent dans la passe et restent rouges, sous le label ctest `concurrence-rouge-connu`. Une liste des rouges attendus vit dans le dépôt, et la passe la compare au résultat : un rouge connu qui devient vert sans marche qui le corrige se signale autant qu'un vert qui devient rouge. Jamais de `DISABLED_`.
- **Q3** : point de reprise automatique coupé partout sauf C8, écrit en tête du banc avec la raison.
- **Q4** : la ligne du journal des chantiers est tenue par l'orchestrateur, sur master.
- **Arrêt brutal** : master rouvre au dernier COMMIT complet et copie ce qu'il retire dans `<base>.wal.ecarte-<ms>`. Tout commit acquitté avant le SIGKILL doit être là après réouverture ; le fichier écarté est nettoyé entre les cas.
- **Lecteur concurrent** : tant que `lecteur-reverifie-a-l-ouverture` n'est pas sur master, le vérificateur lit après la fin des écrivains, ou dans leur processus, jamais depuis un lecteur concurrent.
- **Ordre de livraison** : étape 1, le lanceur Fil, le niveau 1 et C0 à C3, puis le tableau attendu/observé ; étape 2, le niveau 2, C4 à C9, le lanceur Processus et la triple vérification ; étape 3, ThreadSanitizer.
- HNSW hors du banc A1.

## 8. Ce que les étapes 1 et 2 ont ajouté (3 octobre)

- **Règle du vérificateur : ne jamais lire une propriété d'une extrémité pour compter
  ou lister des relations.** Lire `a.id` dans `MATCH (a)-[r]->(b)` joint la table des
  nœuds de a et efface en silence une relation dont a est supprimé, alors que
  `count(r)` la compte encore. La première version du vérificateur a rendu un faux
  vert sur C2 par ce chemin. Son témoin rouge est
  `IntegrityCheckerWitness.SeesARelationWhoseSourceWasDeletedBehindTheExecutor`. Il
  supprime un nœud par `NodeTable::delete_`, sans le contrôle de l'exécuteur, et
  exige que les deux niveaux voient la relation pendante. Ce rouge est fabriqué : il
  ne dépend d'aucun défaut du moteur.
- **Niveau 2 du vérificateur** (`checkLevel2`), dans une transaction de lecture et
  sans le planificateur :
  - chaque ligne visible se retrouve à son offset par l'index de clé primaire ;
  - le stockage et Cypher voient le même nombre de lignes visibles ;
  - la CSR, balayée dans les deux sens depuis tous les offsets, suppressions
    comprises, porte les mêmes relations, et chacune a deux extrémités visibles.
- **Après une erreur dans une transaction explicite, le moteur annule la transaction
  et repasse en auto-commit** : l'instruction suivante est validée seule. Le banc
  n'envoie donc plus rien après un échec (`Worker::run`). C'est ce défaut du banc qui
  faisait monter la somme de C4 à l'étape 2 ; C4 est vert. Côté moteur, c'est un
  défaut : PostgreSQL refuse toute instruction jusqu'au ROLLBACK, Neo4j marque la
  transaction comme échouée, et le principe du projet est de faire comme eux
  (orchestration, 3 octobre). D'où le rouge
  `MinimalReproduction.FailedTransactionRefusesFurtherStatements` : après l'erreur,
  toute instruction doit être refusée par une erreur nommée jusqu'au ROLLBACK, et rien
  de ce qui suit l'erreur ne doit être validé.
- **Le dossier temporaire des bases** : le nom d'un test paramétré contient des « / »,
  et les bases passent donc par des répertoires intermédiaires partagés par tous les
  processus de test. Le banc ne les supprime plus : le faire quand ils étaient vides
  a probablement fait échouer une passe parallèle (déduit du code, non reproduit).
- **C7** : cinq manches par test. Sur vingt passes, il est rouge 18/20 à chaud, 19/20
  après réouverture et 15/20 après arrêt brutal : ce n'est pas unanime. Il porte le
  label `concurrence-probabiliste` (`probabilistic.txt`) et la comparaison l'ignore.
  Son rouge est un vrai rouge ; son vert ne prouve rien.
- **C8** : au plus trois CHECKPOINT et 100 insertions par écrivain, de 11 à 17 s par
  variante. Le délai d'attente du point de reprise (5 s) est privé : l'accès sera
  demandé quand la marche A7 s'ouvrira.
- **Les cas d'attente sur verrou.** Décision de Lucie : l'écrivain attendra sur un
  verrou par clé au lieu d'échouer. Trois cas sont à écrire après la note de la
  session cœur C++ :
  - le premier écrivain annule : le second, qui attendait, réussit ;
  - le premier valide : le second reçoit la clé en double ;
  - un interblocage se termine par une erreur nommée, dans un délai borné.

  Leur mécanique existe et se teste elle-même (`HarnessMechanics`) :
  - `Worker::mark` et `runMarked` tiennent un journal d'événements datés, dans
    l'ordre réel ;
  - `Worker::waitFor` attend l'événement d'un autre écrivain, puisqu'un écrivain
    bloqué n'atteindrait pas une barrière ;
  - un délai de garde par cas (60 s par défaut) interrompt les connexions, ou tue
    les processus, à l'échéance. Si un fil reste bloqué malgré l'interruption, le
    processus de test s'arrête en rouge.

  Constat : le drapeau d'interruption n'est pas regardé pendant l'évaluation d'un
  `range()` géant. Une attente de verrou devra le regarder.

## 9. Étape 3 : ThreadSanitizer (3 octobre)

**Construire et lancer.** Le build est séparé, dans son propre répertoire, et ne
porte aucune suppression pour le moteur :

```bash
cmake -S . -B build/tsan -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo -DBUILD_TESTS=TRUE \
  -DBUILD_SHELL=FALSE -DBUILD_BENCHMARK=OFF -DENABLE_THREAD_SANITIZER=TRUE
cmake --build build/tsan -j 8 --target concurrence_test          # 314 s, moteur compris
CONCURRENCE_ITERATIONS=20 TSAN_OPTIONS="halt_on_error=0 log_path=<dossier>/tsan" \
  ./build/tsan/test/transaction/concurrence/concurrence_test \
  --gtest_filter='*Thread_Hot*:*Thread_Reopen*:MinimalReproduction.*:IntegrityCheckerWitness.*:HarnessMechanics.EventsKeepTheRealOrder:HarnessMechanics.GuardInterrupts*'
```

Cette passe dure 38 s. Le lanceur Fil tourne seul : les variantes Crash et Processus
reposent sur `fork`, et le banc ne les fait pas tourner sous TSan.

**Résultat : 24 avertissements, sur neuf lignes du moteur.** Un rapport par ligne,
avec les deux côtés de l'accès, est dans `02-threadsanitizer-premiere-passe.txt`.
Chaque cas a été relancé seul pour savoir lequel déclenche quoi :

| cas | courses |
|---|---|
| C5 double suppression | `VectorVersionInfo::delete_` (`version_info.cpp:98, 108, 112`), deux suppressions sans verrou — la marche A5. Sous TSan : 9 passes sur 10 |
| C6 suppression contre mise à jour | `VectorVersionInfo::isDeleted` (`version_info.cpp:194, 204`), lu par la mise à jour pendant que la suppression écrit. Vue une fois, puis 0 sur 10 |
| C7 mélange | tout ce qui précède, plus `CSRNodeGroup::append` (`csr_node_group.cpp:357`), `VectorVersionInfo::append` (`:75`), `delete_` (`:119`), `isSelected` (`:149`) et `isInserted` (`:231`) |
| non attribué | `VectorVersionInfo::setDeleteCommitTS` (`version_info.cpp:143`, l'estampille de la suppression posée au commit, contre une lecture) : vu dans la passe complète, dans aucun relancement cas par cas |
| C0 à C4, C8, reproductions minimales, témoin, mécanique | aucun avertissement |

TSan n'est pas encore branché dans ctest. Un rouge TSan n'est pas déterministe non
plus : il ne voit une course que si les deux accès ont effectivement lieu sans relation
d'ordre. Le brancher demande de décider quels cas y tournent, et ce qu'on y compare.

## 10. Ce que la relecture du cœur C++ a changé (3 octobre)

Relecture de `455939802` par la session cœur C++, qui l'a jugé fusionnable tel quel
(fusionné dans `master`, `407fe0b04`). Ses remarques sont traitées ici, sur la branche
`banc-remarques-de-relecture`. Tout ce qui suit est exécuté.

- **Les identifiants de relation en double** (`rel-ids-unique`, `stored-rel-ids-unique`) :
  deux relations de même identifiant s'écrasaient en silence dans les tables des deux
  niveaux. Chaque balayage relève maintenant tout identifiant revu. Aucun cas ne le
  produit aujourd'hui, et il n'a pas encore de témoin.
- **La raison de chaque rouge est épinglée.** Chaque attente du banc porte une étiquette
  `[check: …]` dans son message d'échec, et chaque invariant violé la sienne.
  `known_red.txt` donne, pour chaque nom, l'ensemble exact d'étiquettes attendu ; la
  comparaison échoue sur « known red for another reason ». Un échec sans étiquette
  compte comme `untagged`. Éprouvé sur une copie modifiée de la liste.
- **Un faux vert du niveau 1, trouvé par la nouvelle variante de C2 où la destination
  est supprimée.** `MATCH (a:S) WITH a MATCH (a)-[r]->(b:D)` ne force rien : l'optimiseur
  partait de b et étendait vers l'arrière, et une relation vers une destination supprimée
  disparaissait des deux côtés (`count(r)` rendait même 0). Seul le niveau 2 la voyait.
  Le balayage passe maintenant par une indication de jointure,
  `MATCH (a:S)-[r:R]->(b) HINT (a JOIN r) JOIN b`, vérifiée par EXPLAIN : il balaye a,
  étend en avant, et ne balaye jamais la table de b. Le témoin a désormais deux
  variantes, source et destination.
- **Nouveaux cas, tous rouges aujourd'hui** :
  - `C1_SnapshotPredatesCommit` : la même clé, sans chevauchement des écritures,
    depuis un instantané antérieur au commit de l'autre. Il impose à A3 de contrôler
    contre le dernier état validé ;
  - C2, destination supprimée et `DETACH DELETE` contre nouvelle relation, dans les deux
    ordres de commit ;
  - `MinimalReproduction.SameRowTwoColumnsSingleThread` : deux mises à jour de la même
    ligne sur deux colonnes, validées toutes deux. Son témoin sur la même colonne est
    vert : « Write-write conflict » immédiat.
- **C6** : l'attendu dépend de l'écart n° 2 de la note sur les verrous
  (`docs/3-octobre-2026-15h47/01-note-de-conception-les-verrous.md`). Avec l'option A,
  un instantané par transaction, c'est « jamais les deux » ; avec l'option B, un
  instantané par instruction, les deux validant et la mise à jour touchant zéro ligne
  serait admissible. C'est écrit dans le cas, qui reste rouge.
- **Les scripts sont prêts pour « le second attend »** : `writeInTurn` et
  `commitInOrderByEvents` remplacent les barrières de C1, C2, C5 et C6. Le premier à
  valider attend la fin de l'écriture des autres pendant au plus 2 s, puis valide. Ce
  délai, qui aujourd'hui n'est jamais atteint, laissera passer un écrivain que le moteur
  bloquera demain.
- **La course du chemin de suppression fait planter le processus.** On l'a vu par
  SIGSEGV :
  - dans `VersionInfo::isSelected`, par la recherche de clé primaire, dans C5 : 3
    plantages en au plus 1000 répétitions ;
  - dans `VersionInfo::isDeleted`, dans C2, le `MATCH` de la création de relation
    pendant un `DELETE` concurrent : environ 2 passes sur 15.

  C'est la course que TSan voit entre `initDeletionVersionArray` et `isDeleted` /
  `isSelected`, et la marche A5 la fermera. Deux conséquences dans le banc. C1, C2 et C6
  enchaînent leurs écritures (`writeInTurn`) : même anomalie logique, aucune écriture
  simultanée, 0 plantage sur 30 passes. La comparaison `known_red` ne lance plus les
  cas probabilistes, puisqu'un plantage de C5 emportait toute la passe ; ctest les
  lance un par un sous leur label. La comparaison garde aussi le JSON de toute passe en
  échec.
- **La passe ThreadSanitizer** (`concurrence_tsan.signatures`, label `concurrence-tsan`)
  n'existe que dans le build TSan :
  - C0 doit être propre ;
  - C5 et C6 sont comparés dans un seul sens à `tsan_signatures.txt` (6 signatures, à
    la ligne près, pour les deux côtés de l'accès) ;
  - C7 est lancé et rapporté, jamais comparé. À la ligne comme à la fonction, il fait
    encore apparaître de nouvelles signatures après soixante répétitions, alors que C5
    et C6 se stabilisent dès la première passe.

  Elle dure de 50 à 60 s.

**Règle, posée le 3 octobre** : tout cas ou tout contrôle du banc qui dépend du sens de
parcours d'une relation doit forcer ce sens par une indication de jointure (`HINT`) et
le vérifier par `EXPLAIN`. Un `WITH` ou l'ordre d'écriture du motif ne forcent rien :
l'optimiseur choisit, et un mauvais choix efface en silence ce que le cas cherche.

## 11. Les cas sur une table indexée par HNSW (3 octobre)

Demandés par l'étude de la session cœur C++
(`docs/3-octobre-2026-15h47/02-hnsw-sous-plusieurs-ecrivains.md`, §5). La table `Doc`
porte 300 documents à vecteur de 8 composantes, chaque vecteur étant une fonction de
l'identifiant, et l'index `doc_index`. Tout ce qui suit est exécuté.

**Le build** : `-DBUILD_EXTENSIONS=vector` sur `build/release`. L'extension se
construit en 10 s, dans l'arbre source (`extension/vector/build/libvector.rag3db_extension`),
comme pour les tests `.test`. Elle n'est pas persistante : sans elle, un index d'une
base rouverte n'est plus interrogeable (`extension_loaded = False`). Le banc la recharge
donc à chaque ouverture (`BenchCase::vectorExtension`, `Opener::vectorExtension`,
`reopen()`).

**L'invariant** (`checkVectorIndexes`, appelé par le niveau 1) : pour chaque index
HNSW, une recherche exhaustive (`k = efs =` nombre de lignes vivantes qui portent un
vecteur) doit rendre exactement ces lignes.
- `vector-index-complete` : aucune ligne vivante oubliée ;
- `vector-index-no-dead` : aucune ligne morte, absente ou répétée ;
- `vector-extension-loaded` : un index dont l'extension n'est pas chargée est une
  violation, jamais un saut.

Les identifiants rendus sont comptés un à un : `RETURN count(*)` rend toujours `k`.

**L'exécution isolée** (`BenchCase::isolated`). Le scénario tourne dans un processus
fils, qui vérifie, ferme la base et sort ; le père rouvre et revérifie. Un plantage du
moteur devient le rouge `writer-process-crashed` au lieu de tuer la passe. Une base qui
ne se rouvre plus devient le rouge `database-reopens`. La variante Reopen d'un cas
isolé est sautée, puisque sa vérification Hot ferme et rouvre déjà la base. Les fils
du banc n'écrivent pas de vidage mémoire : sans cela, chaque plantage coûtait 20 à 27 s.

| cas | à chaud (isolé) | après arrêt brutal |
|---|---|---|
| H1, deux écrivains insèrent des vecteurs, transactions ouvertes ensemble | **rouge** : le fils meurt par SIGSEGV au second commit (défaut 5, corrigé par A2) | **rouge**, idem |
| H2, une suppression contre une insertion, deux ordres | vert | vert |
| H3, deux suppressions de documents dont les voisinages se recouvrent (115 et 121 : 41 voisins communs, lus dans les arêtes stockées de l'index) | **rouge** : « Write-write conflict » chez le second, déterministe sur 10 essais | **rouge**, idem |
| H4, le mélange de C7 sur la table indexée | probabiliste : SIGSEGV, ou SIGABRT sur « corrupted double-linked list », et une fois une base qui ne se rouvre plus (« Found duplicated primary key value » au rejeu) | idem |

H1 et H3 sont dans `known_red.txt`, H4 dans `probabilistic.txt`.

**Non fait : H4 sous ThreadSanitizer.** L'extension se construit dans l'arbre source, à
un chemin fixe. Un build TSan avec l'extension écraserait celle du build Release, et
le banc Release chargerait alors une extension instrumentée, ou l'inverse. Il faudrait
un worktree à part pour le build TSan, ou un répertoire de sortie de l'extension
propre à chaque build, ce qui touche le CMake du dépôt. C'est à décider.

## 12. Les témoins des verrous, et une correction (3 octobre au soir)

### Une correction d'abord : la variante Crash ne rejouait aucun journal

Jusqu'au 3 octobre au soir, le processus fils de la variante Crash (`runThenCrash`)
déclarait sa base dans un bloc `try` et se tuait **après** ce bloc. La base était donc
détruite, c'est-à-dire fermée proprement, avec un point de reprise final, avant le
SIGKILL. Pour tout cas où le moteur ne plante pas de lui-même, « après arrêt brutal et
rejeu » voulait dire en fait « après fermeture propre et réouverture ».

**Ce qui était faux, et qui est retiré** : l'affirmation de l'étape 2 selon laquelle
les corruptions de C1 à C3 « passent le rejeu à l'identique » et que « le rejeu du
journal réinsère la clé en double sans rien refuser ». Avec un vrai arrêt brutal :
- **C1 ne se rouvre plus.** Une clé primaire en double validée sous concurrence fait
  échouer le rejeu (« Found duplicated primary key value 7 »), et la base est
  inutilisable. C'est déterministe, et cela n'a pas besoin d'index vectoriel : la
  forme de H4, en sûr.
- **C2 en DETACH DELETE, relation validée d'abord** : le rejeu « répare » la relation
  pendante, et les réponses diffèrent avant et après l'arrêt.

Le fils meurt maintenant base ouverte. Un témoin empêche l'erreur de revenir en
silence : avant de rouvrir, le père exige un journal non vide à rejouer
(`journal-to-replay`). Les journaux observés vont de 171 octets à 238 Ko.

L'erreur a été trouvée en écrivant le cas du `DROP_VECTOR_INDEX` rejoué : il restait
vert alors que la recette de la session de l'arbre principal rougissait.

### Les cas du §6 de la note sur les verrous

Fichier `lock_bench_test.cpp`. Écrits sous les choix tranchés : annonce en tête,
option A, verrou partagé sur les extrémités. Les erreurs à venir sont reconnues par
les fragments convenus avec la session cœur C++ :
- « deadlock » ;
- « lock » et « timeout », pour `CALL lock_timeout=<ms>` ;
- « could not serialize » ;
- « Interrupted » ;
- `CALL acquire_locks('Table', [clés])`.

Une fonction ou un réglage inconnu (`acquire-locks-exists`, `lock-timeout-exists`) est
un rouge distinct d'un mauvais comportement. Il est reconnu en premier, puisque son
message porte le nom demandé.

| § | cas | marche | aujourd'hui |
|---|---|---|---|
| 1 | même clé, même ligne (même colonne et autre colonne), suppression contre mise à jour, suppression contre nouvelle relation : le second attend (sa fin d'écriture vient après la fin de la transaction du premier), puis l'erreur nommée, ou il passe si le premier annule | V1, A3′, A4′ | rouge : `waited` partout, plus l'issue propre à chaque cas |
| 2 | l'unicité quand l'instantané précède le commit de l'autre | A3′ | rouge (`C1_SnapshotPredatesCommit`, existant) |
| 3 | la même ligne, deux colonnes | A4′ | rouge (`SameRowTwoColumnsSingleThread`, et sa variante qui attend) |
| 4 | l'interblocage : exactement une erreur « deadlock », l'autre valide | V1 | rouge : pas de « deadlock », un « Write-write conflict » immédiat |
| 5 | l'annonce {1, 2} contre {2, 1} : aucune erreur, les deux valident l'une après l'autre | V2 | rouge : `acquire_locks` n'existe pas |
| 6 | le rollback, et la transaction en échec puis ROLLBACK, libèrent le verrou | V1 (avec T0) | rouge : `waited` |
| 7 | l'interruption (`CALL timeout`) et le délai (`CALL lock_timeout`) pendant une attente, chacun avec son erreur, après le délai | V1 | rouge ; le délai : le réglage n'existe pas |
| 8 | le nœud-carrefour : huit écrivains, aucun n'attend l'autre | garde-fou de l'écart 3 | **vert**, et doit le rester |
| 9 | rouvrir puis interroger l'index vectoriel : séquentiel, et pendant le chargement de l'extension (30 fois chacun) | — | vert |
| 9 bis | index écrit par CHECKPOINT, DROP_VECTOR_INDEX dans le seul journal, mort : le recréer lève « Index … is not loaded yet » | correctif du rejeu | **rouge, déterministe** |

Deux cas dépendaient de l'ordonnancement ; ils sont rendus déterministes sans changer
ce qu'ils exigeront des verrous :
- l'interblocage : l'écrivain 1 demande sa seconde ligne 200 ms après l'écrivain 0 ;
- la transaction en échec : l'attendant écrit après l'instruction en échec du
  détenteur.

Les quinze cas sont stables sur vingt lancements.

**Les scripts que A4′ devra réécrire** avec mark / waitFor, parce qu'avec l'attente
leur ordre de commit devient impossible (celui qui attend ne peut pas valider le
premier) : C5, C6_UpdateCommitsFirst, et les trois C2_*RelationCommitsFirst. C1 passe
tel quel, avec 2 s d'attente de plus. La note est écrite dans les cas.
