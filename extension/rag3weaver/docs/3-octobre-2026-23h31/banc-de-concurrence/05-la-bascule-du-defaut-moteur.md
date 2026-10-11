# Le banc de la bascule du défaut moteur : le mode multi-écrivains allumé par défaut

11 octobre 2026, session du banc. Le protocole est écrit avant la branche, pour être prêt quand
le cœur C++ la donne (son worktree `rag3db-a9b`, sur luciepc).

## 1. Ce qui change

La mesure A9 a donné un coût nul (`87137a766`, `lock_cost_test.cpp`). Par la règle de Lucie, le
mode multi-écrivains devient le défaut du moteur. L'ancienne sémantique, un seul écrivain et le
second refusé, ne s'obtient plus que par le CALL.

Sur master aujourd'hui :
- le réglage s'appelle `debug_enable_multi_writes` (`src/include/main/settings.h:140`) ;
- il est faux par défaut (`db_config.cpp:33`, `enableMultiWrites{false}`).

La branche dira son nom final : `enable_multi_writes`, ou l'ancien nom gardé comme alias. Ce point
est à relever en premier, parce que tous les tests qui posent l'ancien nom en dépendent.

## 2. L'artefact

Comme toute mesure du banc, on commence par prouver ce qui tourne :
- la branche du cœur, bâtie dans **son propre** worktree et son propre `build/`, sans lien vers un
  autre arbre ;
- la lib et l'extension plus récentes que leurs sources, avec leurs md5 et leurs dates au
  résumé ;
- l'identifiant de bâti de l'extension accepté par le moteur du même bâti ;
- un témoin d'une ligne qui lit le défaut, sur une base neuve et sans CALL :
  `CALL current_setting('<nom>') RETURN *;` doit rendre `true`.

La référence est master à la même heure, bâti de la même façon. Les deux comparaisons se jouent
l'une après l'autre, dans la même tenue `poste mesure`.

## 3. La comparaison du banc, ligne à ligne

1. `compare_known_red.cmake` sur la branche, puis sur master ; d'abord les cas courts, puis
   `CONCURRENCE_LONG=1`.
2. Un tableau qui donne, pour chaque ligne de `known_red.txt` (8 attendues) et pour chaque cas
   dont l'état change, l'état sur master, l'état sur la branche, et la raison.
3. Chaque écart prend l'une de ces trois raisons, sinon il est un défaut à nommer :
   - **un rouge qui disparaît parce que le mode le couvre** : un cas qui ne posait pas le CALL et
     attendait le comportement multi-écrivains ;
   - **un vert qui rougit parce que le défaut change** : un cas qui comptait, sans le dire, sur le
     défaut éteint. Il faut alors soit lui ajouter `=false`, soit réécrire son attente. C'est au
     cœur de trancher, au cas par cas, et le banc écrit les témoins ;
   - **une course révélée** : un cas qui ne rougit que sous la charge. On le rejoue seul, comme le
     veut la règle du banc, avant de l'imputer au moteur.
4. Les probabilistes (`probabilistic.txt`) sont rejoués dix fois chacun sur la branche ; on
   compare leurs taux à ceux de master, sans en tirer de verdict sur un seul essai.

Le harnais du banc pose déjà `debug_enable_multi_writes=true` à chaque ouverture
(`bench_harness.h:564`). Pour le banc lui-même, la bascule ne devrait donc rien changer, à deux
réserves près : un renommage du réglage, et ce qui s'ouvre hors du harnais (le processus neuf de
`probeOpenInFreshProcess`, les rejeux après arrêt brutal). C'est ce que la comparaison doit
montrer.

## 4. Au-delà du banc : les suites qui comptent sur l'ancien défaut

Sur master du 11 octobre, ces cas lisent ou attendent le défaut éteint **sans le dire** :
- `InsertLockTest.WithoutMultiWritesNothingIsLocked`, `…WithoutMultiWritesAnAnnouncementTakesNothing`,
  `…WithoutMultiWritesTheSnapshotStillRules` (`insert_lock_test.cpp:75`, `558`, `703`). Leur nom dit
  « sans le mode » ; après la bascule, ils devront le poser par `=false`, sans quoi ils testeront
  le mode allumé sous un nom faux ;
- `test/test_files/transaction/concurrency/ddl_serial_execution.test:12`, qui attend le refus
  « Cannot start a new write transaction… Only one write transaction at a time is allowed » ;
- tout test hors du banc qui ouvre deux transactions d'écriture et attend que la seconde soit
  refusée. Une recherche par message n'en trouve qu'un, celui de la ligne ci-dessus ; la liste
  complète (`transaction_test`, `api`, `c_api`, l'e2e complet) le dira par ses rouges.

Ces cas posent ou poseront le CALL :
- `debug_enable_multi_writes=true` : `bench_harness.h`, `concurrency_bench_test.cpp`,
  `minimal_reproduction_test.cpp`, `insert_lock_test.cpp` (15), `transaction_test.cpp` (9),
  `failed_transaction_test.cpp`, `local_offset_remap_test.cpp`, les quatre fabriques de
  `journal_with_duplicate_key/`, et quatre fichiers e2e de `test/test_files/transaction/concurrency/`
  (25 occurrences). Après la bascule, ces CALL deviennent sans effet. Ils peuvent rester, comme
  documentation de l'intention ;
- `=false`, donc l'ancien mode par le CALL : `lock_cost_test.cpp` (sa colonne `SingleWriter`),
  `journaled_copy_test.cpp:1858` (`prepare(child, false)`) et `insert_lock_test.cpp:640`.

Le livrable de cette partie est la liste, relevée sur la branche, des cas qui exercent encore
l'ancien mode par le CALL. C'est le reste de la sémantique d'avant, qu'on garde volontairement.

## 5. Ce que le banc rend

- Le tableau du §3 et ses raisons.
- La liste du §4 relevée sur la branche.
- Les lignes de `known_red.txt` et de `probabilistic.txt` à changer, avec leurs témoins réécrits
  s'il en faut, sur une branche partie de celle du cœur, pour qu'il la prenne en avance rapide.
- La liste complète verte, ou ses rouges expliqués, avant la fusion.
