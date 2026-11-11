# Cœur C++ — rapport de session au 11 novembre 2026

Ce rapport est écrit pour un agent qui reprend le moteur C++ de rag3db sans aucune mémoire des
échanges précédents. Le vocabulaire est défini dans `docs/coeur-cpp/11-11-2026/02-knowledge-dump.md`.
Les sessions qui pilotaient ce travail s'appelaient « c7 » (l'orchestration, qui parle au nom de
Lucie), « hotfixs » (une seconde session moteur, qui tient les tickets et les témoins), et « G »
(qui bâtit et pose les libs). La session dont ce rapport est la trace s'appelait « cœur c++ ».

## 1. Le croisement des deux sens des relations : corrigé (29a7bc52a)

**Le défaut.** Après un index complet d'une base de code (la commande `index confirm` de
rag3weaver, qui réécrit toute la base), de petites tables de relations comme `INHERITS_FROM`,
`LOCKS` ou `QUERIES` donnaient des résultats différents selon le sens de lecture. Le sens avant
(de la source vers la destination) et le sens arrière (de la destination vers la source) ne
comptaient plus les mêmes relations, certaines étaient vues deux fois, d'autres rattachées au
mauvais nœud, et une rangée vide se lisait comme « relation 0 vers le nœud 0 ».

**La cause.** Au point de reprise (le moment où le moteur recopie dans le fichier de données ce que
le journal contient), une table de relations est rangée par nœud en « listes » contiguës, dans un
format appelé CSR (défini dans le knowledge dump). Le point de reprise réécrit seulement les
« régions » (des tranches de 1 024 nœuds, ou des groupes de tranches) qui ont changé. Quand une
seule mise à jour (un `SET` sur une propriété d'une relation existante) forçait la réécriture
d'une grande région, le moteur recalculait la place des listes dans cette région, mais ne recopiait
que les colonnes modifiées. La colonne des voisins et celle des identifiants de relation restaient
à l'ancienne place, et le nouvel en-tête les lisait ailleurs.

**Le correctif**, commit `29a7bc52a` sur master (fonction `csrOffsetsMoveInRegion` dans
`src/storage/table/csr_node_group.cpp`) : une région dont un décalage ou une longueur de liste
change réécrit toutes ses colonnes.

**Le témoin** (un test qui rougit avant le correctif et verdit après) :
`LateRelTableRewrites.UpdatingACopiedRelKeepsBothDirectionsThroughACheckpoint`, dans
`test/transaction/concurrence/late_rel_table_rewrites_test.cpp` (commit `e3afa05eb`, écrit par
hotfixs : trois relations versées par COPY, un SET, un CHECKPOINT).

**Ce qui a été joué** sur le poste luciepc : le témoin rouge sans le correctif et vert avec ; les
deux recettes d'hotfixs sur des copies de base, sans écart ; `transaction_test` entier vert (357
cas, 2 outils sautés) ; `concurrence_test` entier avec 21 rouges déjà connus et 6 autres,
départagés : aucun ne dépend du correctif (le détail est dans le ticket ci-dessous).

**Où on en est.** La lib de `29a7bc52a` est dans le magasin des deux postes
(`~/.cache/rag3db-libs/rag3db-build-29a7bc52a…-Release-…`), bâtie le 9 novembre à 10 h 31.
D'après G, l'hôte rag3weaver sous cette lib ne montre plus d'écart (contrôle du 9 novembre à
12 h 07, journal `~/.local/share/rag3weaver/instantanes/controle-29a-controle/code-lourd.log` à
luciepc, chemin donné par c7) ; ce rapport ne l'a pas vérifié lui-même. Le ticket
`docs/coeur-cpp/tickets/2026-11-08-un-index-complet-croise-les-deux-sens-des-petites-tables-de-relations.md`
reste ouvert : il attend le rejeu par hotfixs de sa trace exacte de 3 178 instructions sous cette
lib, et un index complet vert sur une copie. C'est le cœur qui le ferme, dans un commit de docs.

**Les bases de Lucie gardent leur écart.** Le correctif empêche le défaut de se reproduire ; il ne
répare pas les listes déjà abîmées. La réparation est un index complet de chaque base, et c'est
Lucie qui décide quand.

## 2. L'annulation inachevée après un pool plein : témoin écrit, pas joué

**Le défaut** (ticket
`docs/coeur-cpp/tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md`).
Dans le scénario de banc `TwentyWriters.DdlEveryMinute`, une instruction échoue parce que le pool
de tampons (la mémoire où le moteur garde les pages) est plein. Son annulation demande à son tour de
la mémoire, échoue à mi-chemin, et le moteur se met d'office en état « à rouvrir » : chaque
transaction suivante est refusée avec « A rollback of this database could not complete, so it must
be closed and reopened » (318 434 refus dans le journal du banc). L'hôte rag3weaver ne sait pas
aujourd'hui qu'il doit alors rouvrir la base.

**Le code en cause.** `TransactionManager::rollback` et `failAfterStatement` dans
`src/transaction/transaction_manager.cpp` posent `rollbackFailed` quand
`Transaction::rollback` (dans `src/transaction/transaction.cpp`) lève après avoir commencé à défaire
(`halfDoneRollback`). La question ouverte est ce qui, dans le parcours d'annulation du tampon
d'annulation (`undoBuffer->rollback`), demande de la mémoire au pool : sans doute le balayage des
clés insérées pour les retirer de l'index de clé primaire.

**Où on en est.** Un témoin réduit est écrit sur la recette du banc existant
`FullBufferRollback` : `FullBufferRollback.ARollbackUnderAFullBufferLeavesTheDatabaseUsable`, dans
`test/transaction/concurrence/full_buffer_rollback_test.cpp`, sur la branche
`annulation-sous-pool-plein`, commit `6e7f11680`. **Il n'a jamais été joué** : sa couleur sur master
n'est pas connue. c7 a demandé, avant tout correctif, de lire ce que font les moteurs établis
(PostgreSQL réserve une petite mémoire d'erreur, SQLite et DuckDB marquent la base invalide) plutôt
que d'inventer une règle.

## 3. Les gardes de l'étendue du fichier (9bae60786) et la cause qui reste inconnue

**Le défaut.** Le 8 novembre, l'hôte rag3weaver est mort deux fois de SIGSEGV dans
`BufferManager::optimisticRead`, en lisant une page vers le numéro 83 millions d'un fichier de
455 000 pages. Une base de c7 (`code-essais-c7`) portait des segments de vecteurs vivants à la
page 83 108 096 d'un fichier creux (318 Gio apparents pour 1,8 Gio réels).

**Ce qui a été corrigé** (lot sur master de `4560e5e83` à `9bae60786`) :
- `FreeSpaceManager::handleLastPageRange` ne baisse plus le nombre de pages du fichier en mémoire ;
- `Checkpointer::returnOwnerlessPages` ne rend libres que les pages au-delà de tout ce qui est tenu
  (tables, catalogue, métadonnées, espace libre) ;
- le `PageManager` refuse une plage au-delà du fichier, `pin` et `optimisticRead` lèvent une erreur
  pour une page au-delà, `writePagesToFile` refuse d'écrire au-delà ; chacun imprime la pile de
  l'appelant sur stderr.

Témoins dans `test/transaction/data_file_extent_test.cpp`. `transaction_test` entier vert.

**Ce qui reste inconnu** : l'événement qui, entre 0 h 08 et 8 h 50 le 8 novembre, sous les libs
17a094d12 ou dadbf0041, a posé des pages loin au-delà de la fin du fichier. Décision de c7 : ne plus
chercher à l'aveugle ; le prochain cas sera refusé et sa pile se lira dans le journal de l'hôte.
Ticket :
`docs/coeur-cpp/tickets/2026-11-08-un-sigsegv-dans-optimisticread-sous-lecture-concurrente-pendant-une-indexation.md`.
La base de c7 abîmée est mise de côté dans
`~/.cache/rag3db-moteur-notes/essais-c7/etat-ferme-17h14/` (poste principal).

## 4. Ce qui est sur disque

Rien n'a été supprimé.

**Le portable** (le poste principal, celui de Lucie) :
- `~/git_workspaces/rag3db-etendue` : worktree du lot de l'étendue, branche `etendue-gardes-3`,
  propre, sur master ; peut être effacé.
- `~/git_workspaces/rag3db-csr` : worktree de la branche `annulation-sous-pool-plein` (le témoin non
  joué), propre.
- `~/git_workspaces/rag3db-docs-note` : worktree de docs de cette session.
- `~/.cache/rag3db-moteur-notes/` (13 Gio réels) : scripts de passe, journaux, et sous `essais-c7/`
  les copies reflink de la base creuse (`base.rag3db` de 16 h 52 et son journal, `sans-wal`,
  `ecriture`, `apres-fermeture`, `avec-le-lot`, `etat-ferme-17h14`), plus les sondes en lecture
  seule `croise.cpp` (pages à la fois libres et tenues), `entete.py` (étendue de l'en-tête),
  `fsm.cpp`, `sonde.cpp`, `lire-valeurs.cpp`. Les copies sont creuses : `du` les dit petites, `ls`
  les dit à 318 Gio.

**Poste luciepc** :
- `~/git_workspaces/rag3db-colonnes`, dossier de bâti `build/moteur` (Release) : le worktree où se
  bâtissent les passes ; laissé détaché sur `813d7eda8` (master du 9 novembre plus le témoin, sans
  le correctif). Une entrée de stash ancienne, pas de cette session.
- `~/git_workspaces/rag3db-bissect`, dossier `build/asan` (22 Gio, RelWithDebInfo avec ASAN, sur
  `9bae60786`) : il porte une trace d'étude non committée dans `src/storage/table/csr_node_group.cpp`
  (lignes `[csr-etude]`), à jeter par `git checkout -- src/storage/table/csr_node_group.cpp`. Le stash
  contient d'anciens essais du correctif HNSW, déjà sur master.
- `~/.cache/rag3db-moteur-notes/` (4,5 Gio) : `etendue/` (passes du lot de l'étendue), `csr/`
  (passes du correctif des deux sens et le départage), `deux-sens-asan/` (rejeu ASAN et trace
  d'étude, avec des copies de base).

## Reprendre

Deux postes : **le portable** (le poste principal, où tournent les sessions) et **luciepc** (un
second poste, joint par `ssh -T -o BatchMode=yes lucied@luciepc bash -s <<'EOF' … EOF` ; son shell
de connexion est fish, d'où le `bash -s`). Les bâtis et les suites lourdes tournent à luciepc.
Toute commande lourde passe par le verrou du poste, `~/.cache/rag3weaver-build/poste lourd
<commande>` (il limite le nombre de travaux lourds simultanés).

### A. L'annulation inachevée après un pool plein — non fini

- **Où** : le portable, `~/git_workspaces/rag3db-csr`, branche `annulation-sous-pool-plein`,
  dernier commit `6e7f11680` (poussé sur origin), au-dessus de master `833c89539`.
- **Fait** : le témoin `FullBufferRollback.ARollbackUnderAFullBufferLeavesTheDatabaseUsable`
  (fichier `test/transaction/concurrence/full_buffer_rollback_test.cpp`). Il rejoue la recette du
  banc : A ouvre une transaction et insère 20 000 lignes ; B valide des lots jusqu'à remplir le pool ;
  A fait ROLLBACK ; puis le témoin exige qu'un `BEGIN` de A passe (contrôle
  `no-half-done-rollback`) et que les lignes validées par B se relisent (contrôle
  `committed-rows-read-back`). Le cas tourne dans un processus fils.
- **Reste** : le jouer (sa couleur est inconnue), puis la cause et le correctif (voir le
  paragraphe 2 et les prochains pas).
- **Build** : aucun pour cette branche. Le dossier de bâti à utiliser est celui de luciepc,
  `~/git_workspaces/rag3db-colonnes/build/moteur` (Release, déjà configuré), aujourd'hui sur
  `813d7eda8`.
- **Commandes** (à luciepc) :
  ```
  cd ~/git_workspaces/rag3db-colonnes
  git fetch -q origin annulation-sous-pool-plein && git checkout -q --detach 6e7f11680
  git submodule update -q --init extension/rag3weaver/codeparsers
  ~/.cache/rag3weaver-build/poste lourd cmake --build build/moteur -j 8
  mkdir -p /tmp/rag3db-annulation && cd /tmp/rag3db-annulation
  ~/git_workspaces/rag3db-colonnes/build/moteur/test/transaction/concurrence/concurrence_test \
    --gtest_filter='FullBufferRollback.*'
  ```
  Lire dans la sortie les lignes `[rollback sous tampon plein] le fils` : elles disent chaque
  étape (« ROLLBACK passed/refused », « BEGIN AFTER ROLLBACK passed/refused », « B ROWS READ »).
- **Départage** si le rouge surprend : rejouer le cas seul plusieurs fois, puis sous une lib ASAN
  (voir C) pour exclure une erreur mémoire.
- **Push** du correctif, une fois vert et ses suites jouées (`transaction_test` entier,
  `concurrence_test` entier comparé à `known_red.txt` et `probabilistic.txt`), depuis le portable :
  ```
  cd ~/git_workspaces/rag3db-csr && git submodule update -q --init extension/rag3weaver/codeparsers
  ~/.cache/rag3weaver-build/poste push "coeur c++" -- \
    bash ~/.cache/rag3db-moteur-notes/push-local.sh ~/git_workspaces/rag3db-csr <hash testé> <base testée>
  ```
  Le script refuse si master a reçu du code depuis la base testée ; il rebase, pousse sur master,
  puis imprime trois contrôles (arbre propre, aucun rebase en cours, master égal à HEAD). Ne dire
  « poussé » qu'après avoir lu « POUSSE ».

### B. Les gardes de l'étendue — fini

Tout est sur master (`9bae60786`) et joué. Le worktree `~/git_workspaces/rag3db-etendue` (branche
`etendue-gardes-3`, propre) n'a plus rien à faire et peut être effacé. Rien à reprendre, sauf lire
le journal de l'hôte si un refus « beyond the file » y apparaît : sa pile dira d'où vient le numéro
de page faux.

### C. La trace d'étude du point de reprise CSR — à jeter

- **Où** : luciepc, `~/git_workspaces/rag3db-bissect`, détaché sur `9bae60786`, avec une
  modification non committée de `src/storage/table/csr_node_group.cpp` (des `fprintf` préfixés
  `[csr-etude]` qui impriment, au point de reprise, les régions réécrites et les listes déplacées).
- **Build** : `build/asan` (RelWithDebInfo, ASAN actif, 22 Gio), bâti avec la trace.
- **Usage** : c'est l'outil qui a trouvé la cause du croisement des deux sens. Pour s'en resservir,
  rebâtir la lib partagée par `cmake --build build/asan -j 8 --target src/librag3db.so` (la cible
  `rag3db` seule ne refait que la lib statique), et lier le programme de test à
  `build/asan/src/librag3db.so`.
- **Pour le jeter** : `git checkout -- src/storage/table/csr_node_group.cpp`. Le stash du worktree
  contient d'anciens essais du correctif HNSW, déjà sur master, sans valeur.

## 5. Ce qui est bloqué, et sur quoi

- Fermer le ticket de l'index complet : attend le rejeu d'hotfixs sous la lib `29a7bc52a`.
- Lever la règle « aucun index complet sur les bases de Lucie » : décision de c7, qui l'attache à
  la pose de la lib `29a7bc52a`.

## 6. Les trois prochains pas

1. Jouer le témoin `FullBufferRollback.ARollbackUnderAFullBufferLeavesTheDatabaseUsable` sur
   master (branche `annulation-sous-pool-plein`), dans un processus à part, et dire sa couleur.
   S'il est vert, réduire autrement (une annulation d'insertions à clé STRING sous un pool plein).
2. S'il est rouge : trouver quelle étape de l'annulation demande de la mémoire, lire ce que font
   PostgreSQL, SQLite et DuckDB, puis proposer à c7 une règle (mémoire réservée à l'annulation, ou
   annulation qui n'alloue rien) avant de la coder.
3. Fermer le ticket de l'index complet dès le vert d'hotfixs ; jeter la trace d'étude de
   `rag3db-bissect`.

## 7. Les tickets ouverts du cœur

À cette session (« cœur c++ »), tous dans `docs/coeur-cpp/tickets/` depuis le 11 novembre :
- `docs/coeur-cpp/tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md` — l'annulation
  inachevée après un pool plein ; témoin écrit, non joué.
- `docs/coeur-cpp/tickets/2026-11-08-un-index-complet-croise-les-deux-sens-des-petites-tables-de-relations.md` — corrigé en
  `29a7bc52a`, à fermer après le rejeu d'hotfixs.
- `docs/coeur-cpp/tickets/2026-11-08-un-sigsegv-dans-optimisticread-sous-lecture-concurrente-pendant-une-indexation.md` —
  mécanisme corrigé en `9bae60786`, première cause inconnue.
- `docs/coeur-cpp/tickets/2026-10-04-durabilite-sur-faute-d-entree-sortie-ou-coupure.md` — reste la synchronisation du
  répertoire après création ou suppression du journal et du fichier fantôme, et son témoin.
- `docs/coeur-cpp/tickets/2026-10-18-un-point-de-reprise-fige-toutes-les-transactions-le-temps-de-s-ecrire.md` — réduit par
  V2-1c ; restent V2-2c (un écrivain qui entre pendant l'écriture du point de reprise) et V2-3.
  Rendu par le banc au cœur le 11 novembre (3e795b1f5). Ses deux témoins d'écrivain sont dans
  `known_red.txt` : `CheckpointNoStall.WriterCommittedDuringACheckpointSurvivesACrash` et
  `CheckpointNoStall.WriterCommittedWhileTheCheckpointWritesSurvivesItsDeath`.

Au nom du cœur, tenus surtout par hotfixs (la « seconde session cœur C++ ») : les tickets de
`docs/tickets/` dont l'en-tête dit « seconde session cœur C++ », par exemple
`2026-10-19-un-create-vector-index-sous-ecritures-retient-les-autres-le-temps-de-son-rattrapage.md`,
`2026-11-07-un-rang-deplace-pendant-un-create-n-est-pas-toujours-rattrape-premier.md` et
`2026-10-18-les-lignes-d-un-copy-force-valide-apres-un-create-restent-hors-du-graphe.md`. Les autres
tickets qui nomment le cœur sont des optimisations ou du confort classés « hors stèle » par
l'orchestration.

## Actualisation — reprise du 11 novembre

Le ticket des deux sens est fermé après relecture des deux preuves hotfixs
`reprise-codex-11nov-29a7/{d796,item2}.log` (0 erreur et aucun écart). Il vit dans
`docs/tickets/closed/`. La réparation des bases existantes attend toujours Lucie.

Le témoin initial d’annulation passe ; le défaut est désormais reproduit par COPY
avec des clés STRING longues : `65486c02d`, branche `cx-coeur-annulation`,
worktree portable `~/git_workspaces/cx-coeur-annulation`. Les preuves et les conditions exactes
sont dans le ticket d’annulation. Aucun correctif, aucune suite entière jouée sur ce lot.
À luciepc, `rag3db-colonnes` porte encore les variantes exploratoires au-dessus de
`6e7f11680` ; ne pas écraser ce travail avant sauvegarde/synchronisation.

La pile d’étude (`annulation/trace.log` à luciepc) confirme l’allocation refusée :
`rollbackPKIndexInsert` → `scanIndexColumns` → `StringVector::reserveString` →
`InMemOverflowBuffer::allocateNewBlock` → `MemoryManager::allocateBuffer` →
`BufferManager::pin`. Les traces temporaires restent uniquement dans le worktree
d’étude distant ; elles ne sont pas dans les commits. Le voisin INT64 reste vert.

### Correctif sauvegardé — essais en attente de place

`0511dece7`, branche publiée `cx-coeur-annulation`, worktree
`~/git_workspaces/cx-coeur-annulation`. `discardLocalRange` retire les entrées
de l’index local par plage de décalages de l’undo. Compactage des slots par
déplacement, sans relire les colonnes ; tous les types suivent la même primitive.
Les tests ajoutés couvrent chaînes longues en collision sur plusieurs slots,
répétition, réinsertion, plage vide et fin des décalages sans débordement. Le témoin
COPY exige aussi une réouverture avec A vide et les comptes validés de B conservés.

Aucun vert annoncé pour le correctif : à ce point, la compilation attend le
verrou `poste lourd` à luciepc (première en file, quatre places occupées).
Lanceur actif : `~/.cache/rag3db-moteur-notes/annulation/vert.sh`, processus
`3809571`, verrou en attente `3809573`. Journal `annulation/vert.log`.
Il bâtit tout, joue `local_hash_index_test`, puis les six cas FullBufferRollback
et CopyRollbackUnderAFullBuffer trois fois. Les sources sont copiées dans
`~/git_workspaces/rag3db-colonnes`, encore détaché sur `6e7f11680`, avec le code du
correctif ; vérifier leur diff contre `0511dece7` avant d’attribuer les résultats.
La trace temporaire du pool a été sauvegardée dans `annulation/trace-seule.patch`
et retirée des sources distantes avant cette compilation.

Après les ciblés : `transaction_test` entier puis `concurrence_test` entier, comparer
aux rouges connus et probabilistes. Mesurer séparément le coût avec plusieurs
enregistrements undo. Aucun tour de push de code : fenêtre recherche en cours.
Aucune lib du magasin n’a été modifiée pour ce lot.
