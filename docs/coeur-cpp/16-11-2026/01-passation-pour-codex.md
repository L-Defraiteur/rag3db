# Cœur C++ : passation pour Codex (16 novembre 2026)

Pour une session Codex neuve, qui n'a que les docs et les tickets. Écrite par la session « cœur c++ »
(Claude) à la demande de Lucie, par c7. Le reste du savoir du cœur est dans
`docs/coeur-cpp/11-11-2026/02-knowledge-dump.md` et dans les rapports
`docs/coeur-cpp/14-11-2026/01-rapport-de-session.md` et `docs/coeur-cpp/15-11-2026/01-rapport-de-session.md`.

## 1. État

**Sur master, posé par le cœur les 15 et 16 novembre** (chacun avec ses témoins, ses tickets fermés) :

| hash | lot |
|---|---|
| `101ff50a6` | un ALTER ou un DROP TABLE attend les écritures ouvertes de sa table (verrou de schéma, genre TABLE au `LockManager`) ; une écriture dont la table a changé de colonnes depuis le départ de sa transaction est refusée par un message qui contient `Schema changed, retry:` (`Transaction::SCHEMA_CHANGED_RETRY`) |
| `2c11a7995f` | un ADD COLUMN refusé à mi-chemin ne laisse rien au stockage (le SIGSEGV de la colonne ajoutée) |
| `2385b5f616` | une version de mise à jour naît à 4 lignes au lieu de 2 048 : le pool ne sature plus sous vingt écrivains ; `TwentyWriters.DdlEveryMinute` passe de `known_red.txt` à `probabilistic.txt` |
| `7897f2453`, `e5e626803`, `1d5580d74` | l'IN sur la clé primaire planifié en accès par la clé ; l'IN haché ; le pliage des listes littérales |
| `373b5477b`, `0c41f9fe6` | un COMMIT refusé s'annule sous son verrou ; le verrou de la clé primaire |

**En branche, non jugé, à ne pas pousser tel quel** : `dictionary-index-after-string-2` (`256413fde`).
Une ligne de chaîne n'y est comptée qu'une fois sa chaîne au dictionnaire. C'était une piste pour le
SIGSEGV du dictionnaire. Le témoin `DictionaryAppendRefused`, même empoisonné, passe vert avant le
correctif comme après : la piste est tombée, et ce correctif ne se juge sur rien. La branche porte aussi
deux outils utiles, réutilisables : le point de faute `string-dictionary-append` et le crochet
`TestHooks::setPoisonFreshBuffers` (tampons neufs remplis de 0xFF).

**En cours à luciepc** : une passe TSan sur le SIGSEGV du dictionnaire. Bâti
`~/git_workspaces/rag3db-colonnes/build/tsan`, script `~/.cache/rag3db-moteur-notes/dict-tsan.sh`,
journal `~/.cache/rag3db-moteur-notes/dict-tsan/passe.out`. Les rapports sont dans
`~/.cache/rag3db-moteur-notes/dict-tsan/tsan-<tour>.*`. Le tour 1 a donné 15 rapports, aucun touchant
`DictionaryChunk` ou `StringChunkData`. Voir l'entrée C1 pour la lire.

**Ce qu'il faut savoir pour ne rien casser** :
- luciepc (24 cœurs) est le poste des passes ; tout bâti ou test lourd passe par
  `~/.cache/rag3weaver-build/poste lourd …`, à `-j 8` au plus, une validation sous
  `POSTE_MEM_MAX=32G`. Un bâti de `build/tsan` à -j8 a pris 8,1 Gio.
- Un lot qui ne touche que `src/`, `test/`, `extension/vector`, des CMakeLists ou `cmake/` passe par
  la **voie des suites C++ transférables** : voir `docs/procedures/push-d-un-lot.md`, section
  « Un lot du moteur seul ». La lib que la garde cherche dans les journaux est le **BUILD_ID que les
  binaires de test écrivent eux-mêmes** (`rag3db-build-<40 hex>…`), jamais une ligne écrite à la main.
- Si master a changé quoi que ce soit sous `test/` depuis ta base (même le juge known_red), la voie ne
  s'applique plus : reposer la branche sur master et rejouer ce qui en dépend.
- Une tête qui ajoute autre chose que du `.md` au commit testé est refusée par la garde.
- Un `e2e_test` du moteur sans `--gtest_filter='*'` dit « PASSED 0 tests » : ce n'est pas un vert.
- `DropNodeTableReclaim` est le seul rouge admis d'`e2e_test`, sous son ticket (entrée S1).

## 2. Travail pour Codex, dans l'ordre

Pour chaque entrée : le ticket, ce qu'il faut faire, la preuve, la taille, ce qui bloque. Les
commandes de passe sont communes :

```
# Ton arbre : un worktree à toi, une branche cx-<sujet> depuis origin/master, et un bâti à toi.
cmake -S <worktree> -B <worktree>/build/moteur -G Ninja -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_TESTS=TRUE "-DBUILD_EXTENSIONS=vector;geo"
~/.cache/rag3weaver-build/poste lourd cmake --build <worktree>/build/moteur -j 8 \
  --target transaction_test concurrence_test e2e_test rag3db_vector_extension
# Les suites (toujours le bâti de la tête qu'on pousse) :
POSTE_MEM_MAX=32G ~/.cache/rag3weaver-build/poste lourd <build>/test/transaction/transaction_test
( cd <build>/test/transaction/concurrence && POSTE_MEM_MAX=32G ~/.cache/rag3weaver-build/poste lourd \
    ctest -R concurrence_test.known_red --output-on-failure )
POSTE_MEM_MAX=32G ~/.cache/rag3weaver-build/poste lourd <build>/test/runner/e2e_test --gtest_filter='*'
# Le scénario des vingt écrivains (environ 6 à 10 min par tour) :
CONCURRENCE_LONG=1 POSTE_MEM_MAX=32G ~/.cache/rag3weaver-build/poste lourd \
  <build>/test/transaction/concurrence/concurrence_test --gtest_filter='TwentyWriters.DdlEveryMinute'
```

Un témoin neuf se joue **rouge sur le commit qui l'ajoute seul**, puis vert sur le correctif. Un
commit pour le témoin, un pour le correctif, dans cet ordre.

### Pertes de données

**P1. Fermer le COMMIT sous index vectoriel qui sautait des tables**
`docs/coeur-cpp/tickets/2026-11-13-un-commit-sous-index-vectoriel-saute-des-tables-de-noeuds.md`
- À faire : le correctif est sur master (`8275be382b`, `e93710f361`). Le ticket attend la lib bâtie
  par G (la bascule du 16 novembre) et le rejeu de l'arbre principal. Vérifier que les deux ont eu
  lieu (`docs/optimiseur/16-11-2026/`, `docs/orchestration/16-11-2026/`), puis fermer le ticket : État
  « corrigé sur master `8275be382b` », une section « Fermé » qui cite la lib et le rejeu, et le
  déplacer sous `docs/tickets/closed/`. Contrôle : `python3 -I extension/rag3weaver/scripts/check_tickets.py --index`.
- Taille : 30 min. Bloque : la bascule de lib et le rejeu de l'arbre principal ; s'ils n'ont pas eu
  lieu, ne rien fermer.

### Plantages

**C1. Le point de reprise qui meurt dans le dictionnaire d'une colonne de chaînes** — **EN COURS** (arrêté le 16 novembre à 11 h au mot
de Lucie ; reprendre depuis la passe TSan ci-dessous)
`docs/coeur-cpp/tickets/2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines.md`
- Ce qu'on sait : SIGSEGV dans `std::_Hash_bytes` ← `DictionaryChunk::appendString` ←
  `StringChunkData::finalize` ← `Column::checkpointSegment`, pendant un point de reprise demandé par un
  COMMIT (`TransactionManager::writeRequestedCheckpointIfLastNoLock`). Reproduit en Release sous
  `DdlEveryMinute` (1 tour sur 12 avec le pool du scénario réduit à 200 Mio, et deux fois sous le pool
  saturé d'avant `2385b5f616`). Jamais sous ASan (7 tours). La piste « ligne comptée avant sa chaîne »
  est écartée (voir la section 1).
- À faire, dans l'ordre :
  0. **Où en est la passe TSan, laissée finir seule** : au plus 6 tours de `DdlEveryMinute` sous
     200 Mio, lancés à 08 h 12, environ 45 min chacun, arrêt au premier rapport qui touche le
     dictionnaire. Ligne par tour dans `~/.cache/rag3db-moteur-notes/dict-tsan/passe.out` (« rapports
     TSan N ; touchant le dictionnaire M ») ; elle finit par `FIN`. Aux tours 1 et 2 : 15 et 18
     rapports, aucun sur le dictionnaire. Ce qu'ils montrent : des courses sur
     `RelTable::getNumTotalRows` et `RelTable::reserveRelOffsets`, sur `CatalogEntry::setTimestamp`
     (écrit au COMMIT hors du verrou du `CatalogSet` ; voir le ticket de l'entrée B1) et sur
     `Checkpointer::canAutoCheckpoint`. **Comment juger** : un rapport qui nomme `DictionaryChunk` ou
     `StringChunkData` dans l'une de ses deux piles est la piste, avec ses deux fils. Les autres ne
     sont pas ce défaut, mais se comparent à `test/transaction/concurrence/tsan_signatures.txt` ; une
     signature absente de cette liste a droit à son ticket. Aucun rapport sur le dictionnaire en
     6 tours : passer à l'étape 2.
  1. Lire la passe TSan en cours (section 1). `grep -l 'DictionaryChunk\|StringChunkData'
     ~/.cache/rag3db-moteur-notes/dict-tsan/tsan-*`. Un rapport qui les touche donne les deux piles
     concurrentes : qui écrit dans le `StringChunkData` pendant `finalize`. Les autres rapports sont
     déjà connus pour la plupart (`test/transaction/concurrence/tsan_signatures.txt`).
  2. Si TSan ne dit rien en 6 tours : lire qui peut toucher un `StringChunkData` d'un groupe en
     mémoire pendant `NodeGroup::checkpointInMemAndOnDisk`. Les candidats : une mise à jour hors place
     (`StringChunkData::write`, qui ajoute au même dictionnaire), un lecteur sous `LatestCommittedView`,
     et le rattrapage d'un CREATE_VECTOR_INDEX. Puis le verrou qui devrait les exclure.
  3. Le témoin rouge : un cas réduit dans `test/transaction/` (nom de test en anglais) qui écrit des
     chaînes pendant un point de reprise. Pour rendre le défaut visible sans plantage par chance, les
     outils de la branche `dictionary-index-after-string-2` : cherry-pick du commit du crochet
     `setPoisonFreshBuffers` et de celui du point de faute.
  4. Le correctif, puis `DdlEveryMinute` ×10 sous 200 Mio (même réduction du pool, hors dépôt, par
     `sed` sur `BUFFER_POOL` de `test/transaction/concurrence/twenty_writers_scenario_test.cpp`, rendu
     propre après le bâti) : aucun SIGSEGV de cette pile.
- Ne pas toucher : `DictionaryChunk::appendString` sans témoin rouge ; `finalize` a déjà une garde
  contre un pool plein en route (le remappage construit à part, publié d'un coup).
- Taille : 1 à 2 jours. Bloque : rien.

**C2. Un lecteur qui meurt dans l'index de clé primaire sur disque**
`docs/coeur-cpp/tickets/2026-11-13-sous-vingt-ecrivains-un-lecteur-meurt-dans-l-index-de-cle-primaire-sur-disque.md`
- Ce qu'on sait : non reproduit en 3 tours après le verrou de la clé primaire (`0c41f9fe6`). Depuis,
  aucun SIGSEGV de cette pile dans les quelque 60 tours de `DdlEveryMinute` joués les 15 et 16
  novembre (journaux `~/.cache/rag3db-moteur-notes/*/ddl-*.log` à luciepc ; les cores se listent par
  `coredumpctl list`).
- À faire : relever, dans ces journaux et ces cores, s'il existe une pile dans l'index de clé
  primaire (`coredumpctl debug <pid> --debugger=gdb -A "-batch -ex 'bt 20'"`, avec le binaire gardé à
  côté du journal quand il y en a un). Aucune : écrire le relevé au ticket et le fermer comme
  « couvert par `0c41f9fe6`, non revu en N tours ». Sinon : la pile au ticket, et un témoin réduit.
- Taille : 2 h (fermeture) à 1 jour (s'il revient). Bloque : rien.

**C3. Le SIGSEGV dans optimisticRead** :
`docs/coeur-cpp/tickets/2026-11-08-un-sigsegv-dans-optimisticread-sous-lecture-concurrente-pendant-une-indexation.md`.
Pris par la session hotfixs le 16 novembre. Ne le reprendre que si son État ne dit plus « en cours ».

### Réponses fausses

**F1. Une recherche par clé primaire qui rate une clé validée pendant un point de reprise**
`docs/coeur-cpp/tickets/2026-11-13-une-recherche-par-cle-primaire-rate-des-cles-validees-pendant-un-point-de-reprise.md`
- Ce qu'on sait : corrigé par `0c41f9fe6`, mais revu rouge une fois le 16 novembre dans un
  `transaction_test` entier (`PkLookupUnderCheckpoint.LookupsDuringInsertsAndCheckpointsDoNotCrash`,
  `[check: pk-lookup-reads-right] 3`). Joué seul 60 fois : 0 rouge. Il dépend donc de l'ordre ou de la
  charge.
- À faire : le reproduire dans l'ordre du binaire entier. `transaction_test
  --gtest_filter='<les 20 cas qui le précèdent dans le journal>:PkLookupUnderCheckpoint.*'
  --gtest_repeat=20` ; l'ordre se lit dans `~/.cache/rag3db-moteur-notes/dict-fix2/transaction.log`.
  S'il rougit : la cause (une recherche par clé qui lit l'index pendant que le point de reprise
  le réécrit), un témoin, le correctif. S'il ne rougit pas en 100 : l'écrire au ticket, et le cas
  passe en `probabilistic.txt` avec la mesure.
- Taille : 0,5 à 2 jours. Bloque : rien.

### Blocages

**B1. Après un CHECKPOINT, des écrivains refusés « Schema changed, retry: » sans fin**
`docs/coeur-cpp/tickets/2026-11-15-apres-un-checkpoint-les-ecrivains-sont-refuses-schema-changed-sans-fin.md`
- Ce qu'on sait : vu une fois en 42 tours, jamais depuis le correctif du pool. Un refus sans fin exige
  une tête non validée en catalogue. Ce n'est pas une annulation marquée à moitié faite, puisque le
  CHECKPOINT a réussi.
- À faire : rien de neuf sans occurrence (décision de c7). Si un tour de `DdlEveryMinute` le montre
  (des « Schema changed, retry: » qui ne finissent pas, `failed-migration-disturbs-no-one` rouge) :
  rejouer sous la sonde `sonde-diag.py` (dans le scratchpad du cœur, recopiée dans
  `~/.cache/rag3db-moteur-notes/stuck-probe/sonde-diag.py`). Elle s'applique à
  `src/catalog/catalog.cpp`, hors dépôt, et écrit la tête et l'instantané à chaque refus. Puis la
  cause au ticket.
- Taille : 0, sauf occurrence. Bloque : une occurrence.

**B2. Prendre le verrou de table à la liaison, comme PostgreSQL**
`docs/coeur-cpp/tickets/2026-11-15-une-ecriture-se-lie-avant-de-prendre-le-verrou-de-sa-table.md`
- Ce qu'on sait : aujourd'hui, une écriture se lie sur le schéma de son instantané, puis prend le
  verrou de sa table à l'exécution (`Transaction::lockTableForWrite`, appelé par `NodeTable::insert`
  et ses voisins). Si un ALTER est passé entre les deux, elle est refusée « Schema changed, retry: »
  et doit recommencer.
- À faire : prendre ce verrou en partagé au moment de la liaison, dans le binder, pour chaque table
  qu'une clause d'écriture lie (CREATE, SET, DELETE, MERGE, COPY ; les tables de relations comme les
  tables de nœuds). Le refus nommé d'aujourd'hui reste le filet de la transaction ouverte avant
  l'ALTER. Ne pas toucher : l'ALTER et le DROP TABLE, qui prennent le verrou en exclusif ; le
  CREATE_VECTOR_INDEX, qui le prend en partagé.
- Preuve : `AlterWaitsForWriters.AWriterCreatingDuringTheAlterWritesNoRowOfTheOldSchema` et
  `AddColumnRollback.TwoAltersOfOneTableDoNotOverlap` passent sur le chemin « OK », sans refus ni
  relance. Les ajuster dans le commit du correctif, et dire le changement de contrat dans
  `docs/nouveautes.md`. Puis `transaction_test`, known_red et `e2e_test` entiers.
- Taille : 1,5 jour. Bloque : rien. Le contrat des refus est lu côté Rust par rag3weaver (ED et archi,
  qui recommencent sur le préfixe) : prévenir l'orchestration avant le push.

**B3. Un point de reprise qui fige toutes les transactions le temps de s'écrire**
`docs/coeur-cpp/tickets/2026-10-18-un-point-de-reprise-fige-toutes-les-transactions-le-temps-de-s-ecrire.md`
- Réduit par V2-1c. Lire le ticket : ce qui reste, la forme V2-2 prévue. Gros : ne pas le commencer
  sans le mot de l'orchestration.

### Lenteurs

**L1. Un CREATE_VECTOR_INDEX sous quarante écrivains dure quatre minutes**
`docs/coeur-cpp/tickets/2026-11-15-un-create-vector-index-sous-quarante-ecrivains-dure-quatre-minutes.md`
- Ce qu'on sait : bissecté au commit `f42d90d13` (le CREATE en deux transactions) : 36 s avant,
  165 s après. Piste : dès la validation de T_A, le holder NOTED est visible et chargé, et
  `NodeTable::lockIndexForWrite` (`src/storage/table/node_table.cpp`) fait prendre le verrou INDEX
  **en exclusif** à chaque écrivain pendant tout le bâti de T_B, alors qu'aucun n'écrit dans un index
  NOTED (`receivesWrites()` faux).
- À faire : (1) mesurer d'abord où attend l'écrivain le plus long, par `eu-stack` échantillonné pendant
  `VectorIndexCreateUnderManyWriters.NoWriterWaitsLongDuringTheCreate` (perf et valgrind sont absents
  des postes ; eu-stack toutes les 2 s sur le pid du test, les piles comptées). L'écrire au ticket.
  (2) Si c'est le verrou INDEX : `hasLoadedSecondaryIndex` ne compte qu'un index qui reçoit les
  écritures, de sorte qu'un holder NOTED ne fasse pas prendre le verrou.
- Preuve : `NoWriterWaitsLongDuringTheCreate` revient au niveau de sa ligne known_red (CREATE de
  l'ordre de 15 s ; l'instruction la plus longue sous 5 s) ; et restent verts
  `VectorIndexCreateInMemory.*`, `VectorIndexUpdate.*During*` (sous CONCURRENCE_LONG=1) et
  `VectorIndexCreateUnderManyWriters.AWriterOpenDuringACreateFinishesWithoutRetrying`. Si le témoin
  passe vert, retirer sa ligne de `known_red.txt` dans le même commit.
- Ne pas toucher : le rattrapage du CREATE (l'attente des écrivains d'avant T_A, `TransactionManager::
  waitForWriteTransactionsStartedBefore`), dont dépend la justesse de l'index.
- Taille : 1 jour. Bloque : rien.

**L2. Un CREATE REL TABLE qui retient un écrivain au-delà de sa durée**
`docs/coeur-cpp/tickets/2026-11-16-un-create-rel-table-retient-un-ecrivain-au-dela-de-sa-duree.md`
- Ce qu'on sait : rouge dans 5 journaux de `DdlEveryMinute` sur 50
  (`create-rel-table-holds-writers-at-most-its-duration`). Les écrivains de Note, d'une table que le
  DDL ne touche pas, attendent autant : un verrou global plutôt qu'un verrou de table.
- À faire : eu-stack échantillonné pendant la minute 3 d'un tour, pour dire où attend l'écrivain (le
  mutex des appels publics du `TransactionManager`, le point de reprise que le DDL déclenche, ou
  autre). Puis la cause, un témoin réduit, le correctif.
- Preuve : `DdlEveryMinute` ×10 sans ce contrôle rouge, puis le cas quitte `probabilistic.txt` pour la
  comparaison (il y est à cause de ce seul rouge).
- Taille : 1 jour. Bloque : rien.

**L3. Une suppression sous un index vectoriel rempli par SET qui coûte avec sa taille**
`docs/coeur-cpp/tickets/2026-11-13-une-suppression-sous-un-index-vectoriel-rempli-par-set-coute-avec-sa-taille-avant-le-point-de-reprise.md`
- À faire : ce que dit sa section de fermeture : la cause dans la suppression sous un index HNSW non
  checkpointé, ou une mesure sous `poste mesure` qui prouve que la croissance vient du banc.
- Taille : 1 jour. Bloque : rien.

### Place disque

**S1. Après un DROP, la table recréée ne réemploie pas les pages rendues (`DropNodeTableReclaim`)**
`docs/coeur-cpp/tickets/2026-11-14-apres-un-drop-la-table-recreee-ne-reemploie-pas-les-pages-rendues.md`
- À faire : la bissection que dit le ticket, entre le dernier `e2e_test` entier vert connu et master,
  sur `e2e_test --gtest_filter='transaction~ddl~ddl_empty.DropNodeTableReclaim'` (rapide). Puis la
  cause, le correctif, et le cas vert : c'est le seul rouge admis d'`e2e_test`, et le fermer rend
  `e2e_test` entièrement vert.
- Taille : 0,5 à 1 jour. Bloque : rien.

**S2. Un DROP TABLE qui ne rend pas ses pages au disque** :
`docs/coeur-cpp/tickets/2026-11-14-un-drop-table-ne-rend-pas-ses-pages-au-disque.md`. Un souhait
(hotfixs). Après S1, et seulement au mot de l'orchestration.

## 3. Ce qui attend Lucie (Codex n'y touche pas)

- **L'ALTER qui attend les écritures ouvertes**, au lieu d'être refusé : c'est le comportement de
  PostgreSQL, choisi par c7 à sa place le 15 novembre (`a2f9c9b65`, « qu'elle peut retourner »). Son
  prix, à décider : un ALTER posé pendant une indexation en masse attend la plus longue transaction
  d'écriture (un paquet de 2 048 fichiers), soit des minutes, et les écrivains qui arrivent attendent
  derrière lui.
- **Le correctif de la branche `dictionary-index-after-string-2`** : il n'est pas jugé, et il n'est pas
  nécessaire au regard des témoins. Le garder comme durcissement (il ne change rien de visible), ou
  l'effacer. À dire à l'orchestration, qui tranche.

## 4. Les règles du périmètre

- **Langue** : identifiants, noms de tests, de sondes et de scripts en anglais pour tout code neuf ;
  la prose (commits, docs, tickets, commentaires longs) en français, avec ses accents.
- **Un worktree et un bâti à toi**, jamais l'arbre de Lucie (`~/git_workspaces/rag3db`) : une
  branche `cx-<sujet>` depuis `origin/master`, un bâti à toi. Ne pas bâtir dans un dossier où une
  passe tourne.
- **La passe et le push** par `docs/procedures/push-d-un-lot.md`, section « Un lot du moteur seul » :
  sous `poste push`, dans un `if grep -qx VERT`, les trois contrôles (status vide, aucun rebase,
  FETCH_HEAD = HEAD), et jamais sans le « à toi » de l'orchestration (c7).
- **Rouge d'abord** : un témoin se voit rouge sur le commit qui l'ajoute, avant le correctif. Un
  témoin qui passe vert avant le correctif ne prouve rien : on le dit, et le correctif ne pousse pas.
- **Une suite non jouée est rouge.** Un juge rouge sur un seul point vert (« known red turned green »)
  se dit à l'orchestration avec une mesure du cas seul. Une ligne de `known_red.txt` ne change
  qu'avec une mesure.
- **Identité** : `git -c user.email=luciedefraiteur@gmail.com -c "user.name=Lucie Defraiteur"`, commit
  par chemins (`git commit -- <chemins>`), aucune mention d'un assistant dans les messages. Pas de
  force push, pas de suppression de branche distante. Une branche rebasée se pousse sous un nom neuf.
- **Rien vers les amonts** (Kuzu, Vela, Ladybug) : aucun correctif proposé.
- **Le poste** : `-j 8` au plus, tout lourd sous `poste lourd`, une validation sous
  `POSTE_MEM_MAX=32G`. Une passe longue détachée (`setsid nohup`) se relit à l'heure prévue. On n'arrête
  que ses propres processus (`$!`, ou `ps --ppid`), jamais par un `pgrep -f` nu.
- **Un défaut connu non corrigé a son ticket** sous `docs/coeur-cpp/tickets/`, contrôlé par
  `python3 -I extension/rag3weaver/scripts/check_tickets.py --index` (familles admises : moteur ou
  produit).

## État final (16 novembre 2026, 11 h, arrêt au mot de Lucie)

Sur master : les lots du 15 et du 16 (section 1), le dernier en `2385b5f616`. En branche, poussée,
non jugée : `dictionary-index-after-string-2` (`256413fde`). En cours : l'entrée C1, passe TSan finie
seule à luciepc (à lire comme dit en C1, étape 0). Aucun autre pas commencé ; toutes les autres entrées
sont à prendre telles qu'écrites.
