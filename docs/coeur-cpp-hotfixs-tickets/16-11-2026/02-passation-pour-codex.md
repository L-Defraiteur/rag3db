# Passation pour Codex : le moteur C++, côté hotfixs (16 novembre 2026)

Cette page est écrite pour une session Codex neuve, qui n'a que les docs et les tickets. Elle reprend le périmètre de
la session « coeur c++/hotfixs/tickets » : les défauts du moteur C++ (`src/`, `test/`, `extension/vector`,
`tools/shell`) qui ne sont pas dans la file de la session cœur. Le rapport de la nuit, avec le détail de ce qui a
atterri, est dans [01-rapport-de-la-nuit.md](01-rapport-de-la-nuit.md).

## 1. L'état

**Sur master depuis la nuit du 15 au 16 novembre**, chacun avec son témoin :

| Ce qui a changé | Où | Commit sur master |
| --- | --- | --- |
| Le nom d'un fichier que la base crée survit à une coupure : `FileSystem::syncDirectory`, appelé par `VirtualFileSystem::openFile` quand il crée le fichier | `src/common/file_system/` ; témoins `test/transaction/power_cut_test.cpp` | 0818e6819 |
| La suppression du journal par un point de reprise, sans synchronisation du répertoire : prouvée sans dommage, témoin gardé | `power_cut_test.cpp` | c6bf19dda |
| Le fichier de déversement repart de la page 0 (`FileHandle::restartAtPageZero`), et `clearFile` ne vide plus sous un groupe déversé (compte des tampons sur disque) | `src/storage/buffer_manager/spiller.cpp`, `src/storage/file_handle.cpp` ; témoins `test/storage/buffer_manager_test.cpp` | 5dc3247c8 |
| Le shell ne perd plus d'octets d'une entrée par pipe (`getCursorPosition` ne questionne qu'un terminal) | `tools/shell/linenoise.cpp` ; témoin `tools/shell/test/test_shell_basics.py` | 843bda91a |
| Tout `ShellTest` a son propre historique (`-p` temporaire) | `tools/shell/test/conftest.py` | 1b2be095c |
| Le journal des runs de rag3weaver s'écrit d'un coup (temporaire puis `rename`) | `extension/rag3weaver/src/daemon_host/journal_des_runs.rs` | 08643d923 |

**En branche** : rien de cette session. Les branches poussées (`journal-dir-sync`, `clearfile-keeps-live-spills-2`,
`shell-pipe-input-keeps-bytes`…) sont toutes sur master.

**Ce qu'il faut savoir pour ne rien casser** :

- **La procédure** de passe et de push est [docs/procedures/push-d-un-lot.md](../../procedures/push-d-un-lot.md).
  Pour un lot du moteur seul, lire sa section « Un lot du moteur seul (C++) : une seule recette ».
- **`passe_epinglee.sh`** se lance avec un **nom** de dossier de journaux, jamais un chemin : il le place sous
  `~/.cache/rag3weaver-build/`.
- **`poste lourd`** se lance sous `POSTE_MEM_MAX=32G`, sinon il refuse une validation de passe.
- **La lib d'un lot du moteur seul** est le BUILD_ID que le binaire joué écrit lui-même, jamais une ligne écrite à la
  main. Le plus simple : `echo "CALL BUILD_ID() RETURN *;" | <build>/tools/shell/rag3db > $J/build-id.log`, avec le
  shell bâti de la tête du lot. Le journal brut de `transaction_test` nomme aussi un faux BUILD_ID (`ffffffff…`, un
  test le fabrique) : ne pas le copier dans le dossier de journaux, sinon la garde s'arrête sur « plusieurs libs ».
- **Bâtir `rag3db_vector_extension`** avec `transaction_test`. Sans elle, dix tests tombent sur leur
  `LOAD EXTENSION`, ce qui n'est pas un rouge du lot.
- **Après tout rebase**, `python3 extension/rag3weaver/scripts/check_tickets.py`, et `git submodule update --init`
  dans le worktree, sinon le statut montre `extension/rag3weaver/codeparsers` modifié.
- **Un ticket fermé dans un lot** : ajouter la copie dans `docs/tickets/closed/`, sans supprimer la copie ouverte si
  master l'a modifiée. La garde retire elle-même les copies ouvertes après son rebase.
- **luciepc** est le poste des passes : `ssh -T lucied@luciepc 'bash -s' <<'EOF' … EOF`. Le shell de connexion y est
  fish, et `bash -s` est obligatoire.

## 2. Le travail pour Codex, dans l'ordre

**État final de la session hotfixs (16 novembre 2026, 10 h 5x, à l'arrêt demandé par Lucie)** : aucune entrée n'est
en cours. Les quatorze entrées ci-dessous sont toutes **à faire**, aucune n'est commencée. Toutes les branches de la
nuit sont sur master, aucune n'est en branche seule. Aucune passe ni unité ne tourne à luciepc. Les worktrees
`rag3db-extension-dite` et `rag3db-hotfixs-shell` de luciepc sont propres (statut vide) et peuvent servir de modèle,
mais Codex prend les siens (`cx-…`).

### La passe d'un lot du moteur, une fois pour toutes

**Le worktree et le bâti, à luciepc**, une fois par lot :

```bash
git -C ~/git_workspaces/rag3db fetch -q origin master
git -C ~/git_workspaces/rag3db worktree add ~/git_workspaces/cx-<nom> -b cx-<nom> origin/master
cd ~/git_workspaces/cx-<nom> && git submodule update -q --init
cmake -S . -B build/cx -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTS=TRUE -DBUILD_SHELL=ON \
  -DBUILD_EXTENSIONS="vector;geo" -DBUILD_BENCHMARK=OFF
```

**Le bâti et les suites**, toujours sous le verrou du poste, jamais plus de `-j 8` :

```bash
cd ~/.cache/rag3weaver-build
POSTE_MEM_MAX=32G ./poste lourd timeout 7200 bash -c '
  cd ~/git_workspaces/cx-<nom>
  cmake --build build/cx --target rag3db_vector_extension rag3db_shell transaction_test buffer_manager_test -j 8 &&
  (cd build/cx/test/transaction && ./transaction_test --gtest_filter="<le témoin>")'
```

Une passe longue tourne de préférence dans une unité :
`systemd-run --user --unit cx-<nom> --working-directory=$HOME/.cache/rag3weaver-build -E POSTE_MEM_MAX=32G bash -c "./poste lourd …"`.

**Ce que la passe d'un lot moteur joue au minimum** :

- le témoin, rouge sur master d'abord (sur un commit qui n'a que le témoin), puis vert ;
- `transaction_test` entier : 392 verts et 2 sautés au 16 novembre, `VectorIndexCrashReopenProbe.OpenInAFreshProcess`
  et `WalReader.PrintAFile`, deux outils qui attendent une variable ;
- les binaires de `test/storage`, dont les noms sont dans `test/storage/CMakeLists.txt` (`add_rag3db_test`) ;
- `e2e_test` (`build/cx/test/runner/e2e_test`) quand le lot touche le Cypher, le planificateur ou un opérateur.
  Demander à l'orchestration le filtre ou la durée attendue avant de le jouer entier.

**Les journaux** vont dans `~/.cache/rag3weaver-build/<nom-du-dossier>/`, avec trois éléments :
- `resume.log`, dont la première ligne est `testé <hash court> <branche> (…)`, et la dernière exactement `VERT` ;
- `base`, le résultat de `git merge-base HEAD origin/master` ;
- le journal qui nomme le BUILD_ID, écrit par le shell bâti, comme dit plus haut.

**Le push**, seulement au « à toi » de l'orchestration :
`if tail -1 $J/resume.log | grep -qx VERT; then ./poste push <session> -- bash ~/.cache/rag3weaver-build/push_garde.sh cx-<nom> <nom-du-dossier>; fi`,
puis les trois contrôles : statut vide, aucun rebase en cours, et `FETCH_HEAD` égal à `HEAD`.

### 1. Vérifier qu'un ADD COLUMN attend l'écrivain ouvert, et fermer (2 h)

- **Ticket** : [docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-14-un-add-column-passe-sous-une-transaction-d-ecriture-ouverte.md](../tickets/2026-11-14-un-add-column-passe-sous-une-transaction-d-ecriture-ouverte.md).
- **À faire** : le verrou de schéma par table du cœur (681b35b38, `src/processor/operator/ddl/alter.cpp`,
  `lock_manager.h`) fait attendre un ALTER derrière les écrivains ouverts sur sa table. Reprendre le cas du ticket
  (connexion A dans `BEGIN` qui écrit, connexion B qui fait `ALTER TABLE … ADD`) en test gtest, dans
  `test/transaction/` : l'ADD doit attendre le COMMIT de A, puis aboutir. Le test s'appelle par exemple
  `SchemaLock.AnAddColumnWaitsForTheOpenWriter`.
- **Preuve** : si le test est vert sur master, c'est un témoin, et le ticket se ferme avec lui. S'il est rouge, le
  verrou ne couvre pas ce cas : l'écrire au ticket et le donner au cœur, sans corriger.
- **Ne pas toucher** : le verrou lui-même, qui est au cœur.
- **Bloqué par** : rien.

### 2. Le CREATE_VECTOR_INDEX concurrent lit une ligne au-delà de la colonne en cache (8 à 10 h)

- **Ticket** : [2026-11-08-le-create-vector-index-concurrent-lit-une-ligne-au-dela-de-la-colonne-en-cache.md](../tickets/2026-11-08-le-create-vector-index-concurrent-lit-une-ligne-au-dela-de-la-colonne-en-cache.md).
- **À faire** : les morceaux de `createInMemHNSWTableFunc` (`extension/vector/src/function/create_hnsw_index.cpp`)
  sont bornés par un nombre de lignes lu après la mise en cache de la colonne (`_CACHE_COLUMN`,
  `src/function/table/cache_column.cpp`). Les deux doivent venir du même instantané. La forme attendue : borner
  l'insertion par le nombre de lignes de la colonne en cache, et non par un compte relu ensuite.
- **Preuve** : un témoin dans `test/transaction/`, un CREATE_VECTOR_INDEX pendant qu'un écrivain insère dans la
  table. Sous ASAN, le rapport `heap-buffer-overflow` dans `NullMask::isNull` ; en Release, un compte : les nœuds
  insérés dans le graphe égalent les lignes de la colonne en cache. Un bâti ASAN se configure avec
  `-DENABLE_ADDRESS_SANITIZER=TRUE` dans un répertoire de bâti à part (`build/cx-asan`).
- **Bloqué par** : la file du cœur touche le même fichier, avec son lot du CREATE_VECTOR_INDEX lent
  (`create_hnsw_index.cpp` et `NodeTable::lockIndexForWrite`). Demander au cœur, par l'orchestration, qui prend le
  fichier, avant d'écrire une ligne.

### 3. Un rang déplacé pendant un CREATE n'est pas toujours rendu premier (8 à 12 h, après le 2)

- **Ticket** : [2026-11-07-un-rang-deplace-pendant-un-create-n-est-pas-toujours-rattrape-premier.md](../tickets/2026-11-07-un-rang-deplace-pendant-un-create-n-est-pas-toujours-rattrape-premier.md).
- **À faire** : comprendre pourquoi un rang mis à jour pendant le bâti sort troisième et non premier à son vecteur
  exact. Trois hypothèses au ticket : le nouveau vecteur a trop peu de voisins, l'ancien est encore atteint, ou deux
  copies se disputent le premier rang. Le rattrapage est (δ), 6fe9ffe9b, dans le CREATE concurrent.
- **Preuve** : le cas existe, `VectorIndexCreateInMemory.ChangesCommittedDuringACreateAreSeen`
  (`test/transaction/vector_index_update_test.cpp`), rouge environ une fois sur quatre seul
  (`--gtest_repeat=96` : 25 sur 96 au 15 novembre). Il faut un témoin déterministe : un rang déplacé à un instant
  choisi du bâti, puis la recherche à son vecteur. Le correctif retire la ligne du cas de
  `test/transaction/concurrence/probabilistic.txt` dans son commit.
- **Bloqué par** : le 2, qui touche les mêmes fichiers.

### 4. Le nombre de lignes hors du graphe d'un index vectoriel, lisible sans écrire (4 à 6 h)

- **Ticket** : [2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md](../tickets/2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md).
- **À faire** : une colonne de `SHOW_INDEXES`, `rows_outside_graph` (nulle pour un index qui n'a pas cette notion), à
  la manière de `state` et d'`extension_error`. Elle compte les lignes de la table au-delà de
  `numCheckpointedNodes` (`extension/vector/src/include/index/hnsw_index.h`), plus celles que le dernier bâti n'a
  pas prises, sous l'instantané de l'appelant, sans verrou d'écriture.
- **Preuve** : un index bâti, puis un COPY forcé validé juste après le CREATE (recette du ticket du 18 octobre, cité
  dans le ticket) : la colonne rend ce nombre ; après `rebuild`, zéro. Puis `e2e_test` sur les fichiers de test qui
  lisent `SHOW_INDEXES`, dont les colonnes changent.
- **Ne pas toucher** : le seuil de reconstruction, qui est au produit (rag3weaver).
- **Bloqué par** : rien, mais prévenir l'arbre principal : rag3weaver lit `SHOW_INDEXES` par position ou par nom.

### 5. La quarantaine d'une annulation partielle, pour les transactions déjà ouvertes (8 h)

- **Ticket** : [2026-11-12-verifier-la-quarantaine-d-une-annulation-partielle-pour-les-transactions-deja-ouvertes.md](../tickets/2026-11-12-verifier-la-quarantaine-d-une-annulation-partielle-pour-les-transactions-deja-ouvertes.md).
- **À faire** : un diagnostic. Le ticket relève que `commit` et `checkpointNoLock`
  (`src/transaction/transaction_manager.cpp`) n'appellent pas `throwIfCheckpointFailedNoLock` à leur entrée, alors
  que `beginTransaction` et `checkpoint` le font. Écrire un témoin : une annulation qui échoue à mi-chemin
  (`rollbackFailed`) pendant qu'une autre transaction reste ouverte, puis le COMMIT de celle-ci, ou un point de
  reprise. Le témoin doit montrer si un état à moitié annulé est lu ou écrit au disque.
- **Preuve** : si le témoin est rouge, le correctif pose la garde à l'entrée de `commit` et de `checkpointNoLock`. S'il
  est vert, il reste en garde, et le ticket se ferme avec lui.
- **Bloqué par** : la file du cœur lit `transaction_manager.cpp` pour son lot « CREATE REL TABLE retient un
  écrivain ». Se coordonner par l'orchestration avant d'écrire dans ce fichier.

### 6. Le journal d'une transaction sans borne hors COPY (6 à 10 h)

- **Ticket** : [docs/tickets/2026-10-05-journal-d-une-transaction-sans-borne-hors-copy.md](../../tickets/2026-10-05-journal-d-une-transaction-sans-borne-hors-copy.md).
- **À faire** : depuis 71cffbc4b, seul un COPY journalisé regarde `copy_journal_threshold`. Une très grosse
  transaction d'insertions ordinaires remplit encore le tampon de son journal, et finit sur « The buffer pool is
  full ». Le ticket demande de regarder le seuil à toute écriture au journal local, au moins pour une transaction qui
  porte un COPY. Lire le ticket en entier : la décision du 5 octobre borne ce qui est voulu.
- **Preuve** : un témoin d'une transaction d'insertions qui dépasse le seuil sous un petit tampon : refusée
  aujourd'hui, elle doit valider après le correctif. Puis `transaction_test` entier.
- **Bloqué par** : rien.

### 7. Deux nœuds liés par leur clé dans un même MATCH se croisent avant la jointure (8 à 12 h)

- **Ticket** : [docs/tickets/2026-10-12-deux-noeuds-lies-par-leur-cle-dans-un-meme-match-se-croisent-avant-la-jointure.md](../../tickets/2026-10-12-deux-noeuds-lies-par-leur-cle-dans-un-meme-match-se-croisent-avant-la-jointure.md).
- **À faire** : `UNWIND … MATCH (m {id: i}), (d {uuid: kd}) CREATE (m)-[:R]->(d)` construit le produit des deux
  ensembles masqués (10⁸ paires) avant la jointure à la liste. La même écriture en deux MATCH passe en une seconde.
  Le planificateur doit joindre chaque nœud à la liste par sa clé avant le produit.
- **Preuve** : un fichier de `test/test_files` joué par `e2e_test`, avec la recette du ticket sous un tampon de
  512 Mo : « buffer pool is full » aujourd'hui, une seconde après. Puis `e2e_test` sur tout le planificateur.
- **Bloqué par** : le ticket d'everything-declarative
  `docs/everything-declarative/tickets/2026-11-14-un-in-sur-la-cle-primaire-balaie-toute-la-table.md` touche aussi
  le planificateur, et il est donné au cœur. Demander par l'orchestration si les deux se font ensemble.

### 8. Un DROP TABLE ne rend pas ses pages au disque (6 h)

- **Ticket** : [docs/coeur-cpp/tickets/2026-11-14-un-drop-table-ne-rend-pas-ses-pages-au-disque.md](../../coeur-cpp/tickets/2026-11-14-un-drop-table-ne-rend-pas-ses-pages-au-disque.md),
  et son voisin [2026-11-14-apres-un-drop-la-table-recreee-ne-reemploie-pas-les-pages-rendues.md](../../coeur-cpp/tickets/2026-11-14-apres-un-drop-la-table-recreee-ne-reemploie-pas-les-pages-rendues.md).
- **À faire** : après `DROP TABLE` et `CHECKPOINT`, les pages libres en fin de fichier pourraient être rendues au
  disque par une troncature. Le lot etendue-gardes interdit que `numPages` baisse (« numPages ne baisse plus »,
  `src/storage/free_space_manager.cpp`) : une troncature doit donc être une opération à part, au point de reprise,
  seulement sur la plage libre qui termine le fichier, et seulement après la synchronisation des métadonnées.
- **Preuve** : un témoin qui crée une table, la remplit, la retire, fait CHECKPOINT, et mesure la taille du fichier
  avant et après. Puis `transaction_test` entier, et les témoins de l'étendue (`test/transaction/data_file_extent_test.cpp`).
- **Bloqué par** : rien, mais c'est un souhait. Le faire après les 1 à 7.

### 9. Le RTree reçoit des décalages provisoires (2 h pour savoir, 6 h pour corriger)

- **Ticket** : [docs/tickets/2026-10-05-rtree-recoit-des-decalages-provisoires.md](../../tickets/2026-10-05-rtree-recoit-des-decalages-provisoires.md).
- **À faire d'abord** : jouer la piste du ticket (une table à index RTree, `BEGIN`, une insertion, `ROLLBACK`, puis
  une recherche spatiale qui couvre le point). Le ticket dit que l'index ne se crée pas aujourd'hui : commencer par
  vérifier si c'est toujours vrai.
- **Si le défaut est réel** : faire passer le RTree par `commitInsert` comme les deux autres index, et lui donner son
  crochet d'annulation.
- **Bloqué par** : l'orchestration a classé l'extension geo sans utilisateur (10 octobre). Ne pas en faire plus que le
  diagnostic sans son accord.

### 10. Un COPY garde un doublon selon les fils (4 à 6 h)

- **Ticket** : [docs/tickets/2026-10-10-copy-garde-un-doublon-selon-les-fils.md](../../tickets/2026-10-10-copy-garde-un-doublon-selon-les-fils.md).
- **À faire** : sous `IGNORE_ERRORS=true` à plusieurs fils, la ligne gardée entre deux doublons suit l'ordre de fin
  des fils, et non l'ordre du fichier. La forme attendue : garder la première du fichier (tri par numéro de ligne
  dans l'`IndexBuilder`, ou comparaison des numéros de ligne au conflit). Lire d'abord le ticket : il cite ce que
  font les moteurs établis.
- **Preuve** : `test/test_files/exceptions/copy/duplicated.test` joué en `PARALLEL=true` sur un CSV assez gros pour
  plusieurs blocs de 8 Kio : non déterministe aujourd'hui, la première ligne du fichier ensuite.
- **Bloqué par** : rien (classé confort).

### 11. La cible `clean` efface la grammaire Cypher suivie par git (2 h)

- **Ticket** : [docs/tickets/2026-10-18-la-cible-clean-efface-la-grammaire-cypher-suivie-par-git.md](../../tickets/2026-10-18-la-cible-clean-efface-la-grammaire-cypher-suivie-par-git.md).
- **À faire** : `cmake --build <dir> --target clean` efface les quatre fichiers suivis de
  `third_party/antlr4_cypher/` (le lexeur, l'analyseur et leurs en-têtes). Ils ne doivent pas être des sorties
  déclarées du bâti : regarder `third_party/antlr4_cypher/CMakeLists.txt` et la règle qui les déclare `OUTPUT`.
- **Preuve** : `clean`, puis un bâti qui passe sans restauration, et `git status` vide.
- **Bloqué par** : rien.

### 12. Les traces de `run_e2e.sh` font rougir la garde de retrait (2 à 3 h)

- **Ticket** : [2026-11-14-les-traces-de-run-e2e-font-rougir-la-garde-de-retrait.md](../tickets/2026-11-14-les-traces-de-run-e2e-font-rougir-la-garde-de-retrait.md).
- **À faire** : `extension/rag3weaver/run_e2e.sh` écrit `target/charge-last.tsv` et `target/e2e-last.log` dans le
  worktree. Les écrire sous `CARGO_TARGET_DIR`, hors du worktree. `node_modules` de l'IDE est un second artefact : le
  ticket propose de le préserver ou de le déclarer reproductible pour `scripts/garde_worktree.py`.
- **Preuve** : une passe `run_e2e.sh` dans un worktree, puis `garde_worktree.py` VERT au retrait, sans contournement.
  C'est un lot Rust ou script : la passe est celle de la procédure (`passe_epinglee.sh`).
- **Bloqué par** : rien.

### 13. Le premier événement du SIGSEGV d'optimisticRead (confort, 4 h de lecture au plus)

- **Ticket** : [docs/coeur-cpp/tickets/2026-11-08-un-sigsegv-dans-optimisticread-sous-lecture-concurrente-pendant-une-indexation.md](../../coeur-cpp/tickets/2026-11-08-un-sigsegv-dans-optimisticread-sous-lecture-concurrente-pendant-une-indexation.md).
- **À faire** : le plantage est corrigé (une erreur à la place). Reste à trouver ce qui a allongé une base de 455 000
  pages à 83 millions, le 8 novembre. La section « Le lot de lecture du 16 novembre » dit ce qui est écarté. La piste
  qui reste est l'écriture partielle du COPY borné : 977c2c866, `NodeGroup::extendFlushedHead`, le rollback de
  `OptimisticAllocator`.
- **Preuve** : seulement un témoin qui fait grandir le fichier au-delà de ce qui est tenu. Aucun code sans lui.
- **Bloqué par** : rien, mais c'est du confort. Le faire en dernier.

### 14. La voie C++ de la garde ignore `tools/shell` (2 h, pour G si G est là)

- **Ticket** : [2026-11-16-la-voie-c-de-la-garde-ignore-tools-shell.md](../tickets/2026-11-16-la-voie-c-de-la-garde-ignore-tools-shell.md).
- **À faire** : ajouter `tools/shell/` (et le reste de `tools/` compilé avec le moteur) aux chemins de
  `cpp_transferable` dans `tools/poste/push_garde.sh`, avec son témoin de script dans `tools/poste/test_push_garde.sh`.
  Puis le dire dans la section « Un lot du moteur seul » de la procédure.
- **Bloqué par** : la garde appartient à la session G (optimiseur). Ne la toucher qu'avec son accord.

## 3. Ce qui attend Lucie (Codex n'y touche pas)

- **Refuser d'ouvrir une base dont une extension notée manque**, avec une option pour l'ouvrir quand même :
  [2026-10-10-ouverture-sans-une-extension-dont-une-table-depend.md](../tickets/2026-10-10-ouverture-sans-une-extension-dont-une-table-depend.md).
  Aujourd'hui l'ouverture réussit et le dit (`extension_error`, et un avertissement de rag3weaver depuis 7ece05d8a).
  Le refus est une décision de produit.
- **Le contrat d'un ADD COLUMN sous un écrivain ouvert**, si l'entrée 1 montre que le verrou de schéma ne le couvre
  pas : attendre, refuser, ou documenter le rattrapage actuel.
- **Les morceaux de Scope à plusieurs arêtes vers le même Scope**
  ([2026-11-14-des-morceaux-de-scope-portent-plusieurs-aretes-vers-le-meme-scope.md](../tickets/2026-11-14-des-morceaux-de-scope-portent-plusieurs-aretes-vers-le-meme-scope.md)) :
  ils se ferment par le retrait de l'ancien index (le pas de l'arbre principal qui suit 1b), et ne demandent rien
  au moteur.

## 4. Les règles du périmètre

- **Langue** : identifiants, noms de tests, scripts, options et unités en anglais pour tout code neuf. La prose (docs,
  tickets, commits) reste en français. On ne renomme pas l'existant.
- **Un worktree et un bâti à soi**, avec des branches `cx-…`. On ne commite ni ne bascule rien dans l'arbre de Lucie
  (`~/git_workspaces/rag3db`).
- **Rouge d'abord** : le témoin seul, sur son propre commit, doit être vu rouge avant le correctif. Un témoin vert
  qui ne vérifie pas que son cas a été joué ne prouve rien : lui donner une condition de mise en place
  (`[check: setup]`).
- **Une suite non jouée est rouge**, et un test sauté n'est pas vert.
- **Passe et push** par [docs/procedures/push-d-un-lot.md](../../procedures/push-d-un-lot.md), jamais sans le
  « à toi » de l'orchestration. Chaque commit est poussé sur sa branche dans la minute. Pas de force push, pas de
  suppression de branche distante : une branche rebasée se pousse sous un nouveau nom (`-2`, `-3`).
- **Identité** : luciedefraiteur@gmail.com, et aucune mention d'un assistant dans les commits.
- **Rien vers les amonts** Kuzu, Vela ou Ladybug : on lit leurs correctifs, on ne leur propose rien.
- **Le poste** : tout bâti et toute suite lourde passent par `poste lourd`, jamais plus de `-j 8`. Une mesure passe par
  `poste mesure`.
- **Un défaut connu non corrigé** reçoit un ticket sous `docs/<session>/tickets/`. Fermer, c'est faire un `git mv` vers
  `docs/tickets/closed/`, avec l'État « fermé » et la preuve.
- **Format** : clang-format-18 sur les fichiers C++ du lot, par l'image `rag3db-clang-format-18:latest`, avant le push.
