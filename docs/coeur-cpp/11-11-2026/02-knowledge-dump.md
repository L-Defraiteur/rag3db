# Cœur C++ — ce qu'il faut savoir pour reprendre le moteur (11 novembre 2026)

Écrit pour un agent qui n'a aucune mémoire des échanges passés. L'état du moment est dans
`docs/coeur-cpp/11-11-2026/01-rapport-de-session.md`. Ici : ce qui ne change pas d'un jour à l'autre.

## 1. Ce qu'est rag3db

- **rag3db** est une base de données de graphe embarquée en C++, un fork de **Kuzu** (langage de
  requête Cypher, tables de nœuds et tables de relations, stockage en colonnes). Le dépôt est
  `github.com/L-Defraiteur/rag3db`, branche principale `master`.
- **Rien ne part vers les amonts** (Kuzu, et les autres forks Vela et Ladybug) : aucun correctif
  proposé là-bas. On peut lire leurs commits pour s'en inspirer.
- **rag3weaver** est un crate Rust dans `extension/rag3weaver` : un « agent de code » qui indexe des
  dépôts de code dans des bases rag3db et les sert par un **hôte** (un démon, `rag3weaver-backend`).
  Il lie le moteur comme une bibliothèque partagée (`librag3db.so`). Le sous-module
  `extension/rag3weaver/codeparsers` (les analyseurs de code) apparaît souvent « modifié » dans
  `git status` : c'est un pointeur de sous-module, à ne pas committer par erreur.
- Les extensions **vector** (index HNSW de vecteurs, `extension/vector`) et **geo** se bâtissent
  avec le moteur et doivent porter le **même identifiant de bâti** que lui, sinon le moteur refuse
  de les charger (« Extension VECTOR was built from …, this engine from … »).

## 2. Le stockage, en mots simples

- **Le fichier de données** (`<base>.rag3db`) est découpé en **pages** de 4 Kio. L'**en-tête**
  (page 0) dit où sont le catalogue et les métadonnées, et l'**étendue** (`numDataPages`, le nombre
  de pages que le dernier point de reprise savait tenir).
- **Le format** est un numéro posé dans l'en-tête, aujourd'hui **43**
  (`src/include/storage/storage_version_info.h`, `STORAGE_FORMAT_VERSION`). Ce fichier dit ce que
  chaque version a ajouté (40 : l'étendue ; 41 : le bâti du dernier écrivain ; 42 : des numéros de
  colonnes stables ; 43 : l'état de construction d'un index). Le moteur lit les bases de 39 à 43,
  et refuse une base plus récente que lui.
- **Les groupes de nœuds** : une table est découpée en groupes de 131 072 lignes
  (`NODE_GROUP_SIZE`, 2^17). Chaque colonne d'un groupe est rangée en **segments**, des suites de
  pages.
- **Le journal** (WAL, `<base>.rag3db.wal`) reçoit chaque transaction validée. Une validation
  n'est publiée qu'une fois le journal synchronisé sur disque.
- **Le point de reprise** (checkpoint) recopie dans le fichier de données ce que le journal
  contient, puis vide le journal. Il passe par un **fichier fantôme** (shadow file) : les pages
  changées y sont écrites d'abord, puis appliquées, pour qu'une coupure au milieu reste réparable.
  Il se déclenche à un seuil de taille du journal (`checkpointThreshold`), ou par `CHECKPOINT;`.
- **Les générations (V2)** : depuis fin octobre, un point de reprise ne bloque plus les lecteurs. Il
  bâtit des blocs neufs, et l'ancienne génération reste lisible pour les lecteurs entrés avant lui.
  Pages d'un bloc remplacé rendues libres à sa mort. Pages libres gérées par le
  `FreeSpaceManager` (l'« espace libre »).
- **Le pool de tampons** (buffer pool) est la mémoire où le moteur garde les pages. Quand il est
  plein et que rien ne peut être évincé, une allocation lève « Unable to allocate memory! The buffer
  pool is full ».
- **Les tables de relations en CSR** : pour chaque sens (avant, de la source vers la destination ;
  arrière, l'inverse), les relations d'un groupe sont rangées par nœud, en **listes** contiguës. Un
  **en-tête CSR** donne pour chaque nœud le début et la longueur de sa liste. Les listes sont
  groupées en **régions** : une région feuille couvre 1 024 nœuds, et les régions de niveau plus
  haut doublent à chaque niveau, jusqu'au groupe entier. Des **trous** (places vides) sont laissés
  pour les insertions futures. Au point de reprise, seules les régions changées sont réécrites ; une
  région trop dense « remonte » d'un niveau (`isWithinDensityBound`, densités 1,0 pour une feuille,
  0,8 au sommet), et une région remontée voit ses listes recalculées (`populateRegionCSROffsets`).
  Le code : `src/storage/table/csr_node_group.cpp`.
- **Les verrous** : plusieurs écrivains écrivent en parallèle. Le `LockManager` (dans
  `TransactionManager`) pose des verrous sur ce qu'une transaction écrit, rendus à sa fin. Un
  conflit d'écriture lève « could not serialize access due to concurrent update ».
- **L'annulation** : une transaction garde un **tampon d'annulation** (undo buffer) de ce qu'elle a
  fait. Si l'annulation échoue à mi-chemin, le moteur se met en état « à rouvrir » et refuse tout
  (`rollbackFailed` dans `src/transaction/transaction_manager.cpp`).

## 3. Les suites de tests

- **`transaction_test`** (sources dans `test/transaction/`) : 359 cas, de vingt minutes à une
  heure à luciepc selon la charge du disque. À jouer en entier avant tout push qui touche le stockage ou les
  transactions.
- **`concurrence_test`** (sources dans `test/transaction/concurrence/`) : le **banc** de concurrence,
  environ 420 cas, dont 55 qui se sautent hors de leur lanceur, longs (scénarios à vingt
  écrivains, coûts mesurés) : près d'une heure. Il a des rouges connus :
  - `test/transaction/concurrence/known_red.txt` : défauts connus, une ligne `<nom du cas>
    <étiquette>` ;
  - `test/transaction/concurrence/probabilistic.txt` : cas qui rougissent selon la charge.
  Un rouge du banc ne compte que s'il n'est dans aucun des deux. Celui qui a la mesure décide d'y
  inscrire un cas.
- Les tests de bas niveau du stockage : les binaires de `build/<dossier>/test/storage/`.
- **Un test qui se saute n'est pas vert** : lire la ligne `[  SKIPPED ]` et vérifier que ce sont des
  outils attendus (`WalReader.PrintAFile`, `VectorIndexCrashReopenProbe.OpenInAFreshProcess`).
- Les binaires de test se lancent depuis un dossier à part (par exemple
  `mkdir -p /tmp/rag3db-x && cd /tmp/rag3db-x`), car ils créent des bases dans le dossier courant.

## 4. Bâtir

- Toujours `cmake --build <dossier de bâti> -j 8` : jamais `-j$(nproc)`, il faut laisser des cœurs
  libres. Sous fish, `ninja` est un alias à `-j32` : ne pas l'utiliser.
- Les options de bâti qui changent l'interface binaire entrent dans l'**identifiant de bâti**
  (`cmake/build_id.cmake`, `CMakeLists.txt` vers la ligne 430) : par exemple
  `rag3db-build-<commit>-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`. `p12` est la
  taille de page en puissance de deux, `v11` la capacité d'un vecteur de calcul, `g17` la taille
  d'un groupe de nœuds, `s18` la taille maximale d'un segment ; puis contrôles à l'exécution,
  fil unique, ASAN, TSAN, UBSAN. Un arbre modifié ajoute `-dirty-<empreinte>`.
- **Le magasin de libs** : `~/.cache/rag3db-libs/<identifiant de bâti>/` sur chaque poste. L'hôte
  rag3weaver et les binaires sont liés à une lib du magasin par leur RUNPATH. C'est G qui bâtit et
  pose les libs ; une lib n'entre au magasin qu'après les suites du lot.
- **ABI** (interface binaire) : changer un membre de données, une taille ou une signature exportée
  (`RAG3DB_API`, `src/include/main`, l'API C, `tools/rust_api`) oblige à rebâtir l'hôte. Ajouter une
  méthode à une classe interne ne l'oblige pas. Ne jamais mettre `RAG3DB_API` sur une méthode d'une
  classe déjà exportée.
- `--target <un test>` ne rebâtit pas l'extension vector : faire un bâti complet avant le banc.
  `--target rag3db` ne rebâtit que la lib statique : la lib partagée est `--target src/librag3db.so`.
- `cmake --build … --target clean` efface aussi la grammaire Cypher générée, qui est suivie par git :
  la restaurer ensuite par `git checkout -- <fichiers>`.
- Les bâtis et les suites lourdes tournent à **luciepc**, le second poste, sous le verrou
  `~/.cache/rag3weaver-build/poste lourd <commande>`. Un script distant se lance détaché
  (`setsid nohup bash <script> > <journal> 2>&1 < /dev/null &`) et se suit en lisant son journal.

## 5. Départager un rouge

1. **Seul** : rejouer le cas seul (`--gtest_filter=<cas>`), sous `poste lourd`, et lire son
   message. Un rouge vu seulement dans le banc entier, sous la charge des autres, est d'abord un
   rouge d'environnement.
2. **Avec et sans le changement** : rejouer le même cas sur le commit testé et sur la base (master
   sans le changement). Même verdict des deux côtés : le changement n'y est pour rien.
3. **ASAN** : bâtir une lib avec `-DENABLE_ADDRESS_SANITIZER=TRUE` (dossier `build/asan` de
   `~/git_workspaces/rag3db-bissect` à luciepc), lier le témoin à elle, et jouer avec
   `ASAN_OPTIONS=detect_leaks=0:halt_on_error=1`. Aucun rapport ASAN veut dire : une erreur de
   logique, pas de mémoire.
4. **Une trace d'étude** : des `fprintf(stderr, …)` posés dans un worktree à part, jamais poussés,
   pour voir ce que fait le moteur sur la recette réduite. C'est ainsi qu'a été trouvée la cause du
   croisement des deux sens.
5. **Rouge d'abord** : un correctif vient avec un témoin qu'on a vu rouge sans lui, puis vert avec
   lui. On ne dit jamais la couleur d'un témoin qu'on n'a pas joué.

## 6. Les pièges appris

- **Pas d'`EXPECT_EQ` sur de grands textes** : un rouge sur 400 000 octets fait réserver à gtest
  une table géante et épuise la mémoire. Comparer des tailles ou des empreintes.
- **Mourir base ouverte** : un test d'arrêt brutal tue le processus fils dans la portée de la
  `Database` (`kill(getpid(), SIGKILL)`), sinon la fermeture propre efface ce qu'on voulait voir.
- **Une coupure de courant se simule** par `CutFileSystem`
  (`test/include/test_helper/cut_file_system.h`), installé par
  `VirtualFileSystem::setDefaultFileSystemFactoryForTesting` : les écritures non synchronisées sont
  perdues quand le fils meurt.
- **Le shell de l'outil Bash est zsh** : une variable non citée ne se découpe pas en mots ;
  `G=(git -c … )` puis `$G commit` ; et `$var:e` est un modificateur, écrire `${var}:e`.
- **luciepc parle fish** : passer par `ssh … bash -s <<'EOF'`.
- **`pgrep -f motif` se trouve lui-même** : ne jamais tuer sur sa sortie sans exclure son propre
  processus.
- **`/tmp` du portable est de la mémoire vive** (tmpfs) : pas de dossier de bâti ni de grosse base
  dedans.
- **L'arbre `~/git_workspaces/rag3db` du portable est celui de Lucie** : on n'y bascule jamais de
  branche, on n'y committe pas de code, on n'y laisse aucun fichier. Pour lire master depuis là :
  `git show origin/master:<chemin>`. Tout travail se fait dans un worktree à soi
  (`git worktree add -b <branche> ~/git_workspaces/rag3db-<nom> origin/master`, puis
  `git submodule update -q --init extension/rag3weaver/codeparsers`).

## 7. Les règles de commit et de push

- Identité : `git -c user.email=luciedefraiteur@gmail.com -c "user.name=Lucie Defraiteur"`, jamais
  l'adresse professionnelle ; aucune ligne d'attribution à Claude dans les messages.
- Committer par chemins (`git commit -- <chemins>`), messages en français.
- Chaque commit se pousse, sur sa propre branche ; après un rebase, une branche porte un nouveau
  nom (`-2`, `-3`). Jamais de push en force, jamais de suppression de branche distante.
- Le code arrive sur master par `poste push "coeur c++" -- bash
  ~/.cache/rag3db-moteur-notes/push-local.sh <worktree> <hash testé> <base testée>` (portable).
  « Poussé » ne se dit qu'après les trois contrôles imprimés : arbre propre, aucun rebase en cours,
  master égal à HEAD.
- Les docs seules se poussent directement sur master, depuis un worktree de docs : commit par
  chemins, `git rebase origin/master`, `git push origin HEAD:master`.
- Les tickets vivent dans `docs/tickets/` ou dans le dossier de leur session (ceux du cœur dans
  `docs/coeur-cpp/tickets/`), fermés dans `docs/tickets/closed/` par `git mv`. Un
  en-tête porte État, Famille, Gravité, Atteignable en service, Touche rag3weaver. Le contrôle est
  `python3 -I extension/rag3weaver/scripts/check_tickets.py`, à jouer avant chaque commit de ticket.

## 8. Les tickets ouverts de cette session

- `docs/coeur-cpp/tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md` :
  après un pool plein, une annulation échoue à mi-chemin et la base refuse tout jusqu'à sa
  réouverture ; témoin écrit, pas joué.
- `docs/coeur-cpp/tickets/2026-11-08-un-index-complet-croise-les-deux-sens-des-petites-tables-de-relations.md` :
  corrigé en `29a7bc52a` ; à fermer après le rejeu d'hotfixs sous cette lib.
- `docs/coeur-cpp/tickets/2026-11-08-un-sigsegv-dans-optimisticread-sous-lecture-concurrente-pendant-une-indexation.md` :
  mécanisme corrigé en `9bae60786` ; la première cause des pages posées loin au-delà du fichier reste
  inconnue.
- `docs/coeur-cpp/tickets/2026-10-04-durabilite-sur-faute-d-entree-sortie-ou-coupure.md` : reste la
  synchronisation du répertoire après création ou suppression du journal et du fichier fantôme.
- `docs/coeur-cpp/tickets/2026-10-18-un-point-de-reprise-fige-toutes-les-transactions-le-temps-de-s-ecrire.md` :
  réduit par V2-1c ; restent V2-2c (un écrivain qui entre pendant l'écriture du point de reprise)
  et V2-3.

## Format C++ — précision du lot de nuit du12novembre

La CI utilise clang-format18 ; le dernier job terminé lu confirme18.1.3-1ubuntu1,
image et empreintes du lot dans `../12-11-2026/22-ci-format18-inventaire-source-et-recette.md`.
Les annexes et sous-modules sont exclus par .clang-format-ignore. Pour juger le
format avant push, appeler le script avec --clang-format-executable pointant sur
la18 vérifiée, sans --in-place **et sans --dry-run** : ce dernier ne fait qu'imprimer
les commandes et ne compare rien. Vérifier un inventaire non vide et lire le code/
diff du vrai contrôle ; ne pas assimiler sa sortie à une CI future réellement verte.
