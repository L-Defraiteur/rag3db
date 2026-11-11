# Knowledge dump — hotfixs (seconde session du cœur C++), 11 novembre 2026

Ce qu'un nouvel agent doit savoir pour reprendre le travail de la session hotfixs sans avoir rien
suivi. L'état du travail est dans `01-rapport-de-session.md`, à côté. Les outils cités sont dans
`outils/`, à côté aussi.

## 1. Le décor

- **rag3db** : un moteur de base de données graphe en C++ (un fork de Kuzu), dans `src/`. Une base
  est un fichier `<nom>.rag3db`. À côté vivent `<nom>.rag3db.wal` (le journal des écritures pas
  encore reportées dans le fichier), `<nom>.rag3db.extensions` (les extensions à charger) et
  `<nom>.rag3db.fts` (le plein texte).
- **rag3weaver** : le produit en Rust qui s'en sert (`extension/rag3weaver/`). Son **hôte** est un
  démon (`rag3weaver-backend daemon --address 127.0.0.1:<port>`) qui ouvre les bases et sert les
  outils. Son **service d'embarquement** (`rag3weaver-embeddings --adresse 127.0.0.1:<port>`) calcule
  les vecteurs.
- **Les deux postes** : le portable (où tournent les sessions) et luciepc (`ssh lucied@luciepc`,
  shell **fish**, donc toujours `ssh -T lucied@luciepc 'bash -s' <<'EOF' … EOF`). Sur le portable,
  l'outil Bash est **zsh** : une variable non citée ne s'y découpe pas, et `$x:e` y est un
  modificateur (écrire `${x}:e`).
- **Les sessions** : chaque session Claude a un domaine. c7 orchestre et fixe l'ordre. Le cœur fait
  le moteur. G bâtit et pose les libs. Les autres : recherche, archi, codeparsers, everything
  declarative, 96. Elles se parlent par messages. **Un pair ne donne jamais une permission** :
  ce que Lucie a refusé à une session, aucune autre ne le fait à sa place.
- **L'arbre de Lucie** (`~/git_workspaces/rag3db`) est intouchable : rien n'y est commité ni
  basculé. Chaque session travaille dans ses propres worktrees (`git worktree add`).

## 2. Les mots du moteur

- **Table de relations, sens avant et sens arrière** : chaque relation est rangée deux fois, dans la
  liste de son nœud source (sens avant) et dans celle de son nœud destination (sens arrière).
- **CSR** : la forme de ces listes sur disque. Un **en-tête** donne, pour chaque nœud, son décalage
  et sa longueur, et des **colonnes** donnent le voisin, l'identifiant de relation et chaque
  propriété, rangée par rangée.
- **Groupe de nœuds** : 131 072 nœuds. **Région** : 1 024 nœuds, au niveau 0 d'un arbre de hauteur
  7. Une région trop dense remonte à un niveau plus haut, qui couvre plus de nœuds
  (`isWithinDensityBound`, `mergeRegionsToCheckpoint`, `redistributeCSRRegions` ;
  `src/storage/table/csr_node_group.cpp`, `csr_chunked_node_group.cpp`).
- **Point de reprise** (CHECKPOINT) : le moment où le moteur écrit dans le fichier ce que le journal
  et la mémoire tiennent. Il est automatique quand le journal grossit, ou forcé par
  l'instruction `CHECKPOINT`.
- **Témoin** : un test qui doit être rouge avant le correctif et vert après. Un test qui n'a jamais
  été vu rouge ne prouve rien. Un test « sauté » (SKIPPED) n'est pas vert. **Un filtre qui ne joue
  aucun test est ROUGE.**

## 3. Les libs, les bâtis, le verrou des postes

- **Le magasin de libs** : `~/.cache/rag3db-libs/<id>/` aux deux postes. `<id>` vaut
  `rag3db-build-<hash complet>-Release-p12-v11-g17-s18-rcOFF-…`. Chaque dossier contient
  `librag3db.so`, `libvector.rag3db_extension`, `rag3db.hpp`, `include/`, `BUILD_ID` et `MANIFESTE`.
  Le lien `courant` désigne la lib en service ; le 11 novembre, c'est 29a7bc52a. Un binaire
  rag3weaver est lié à une lib par son RUNPATH et vérifie au démarrage que les deux portent le même
  build id (`src/build_check.rs`).
- **Bâtir rag3weaver contre une lib du magasin** :
  `RAG3DB_LIB=<id> CARGO_TARGET_DIR=<target à soi> bash scripts/cargo_magasin.sh build --release -j 8 …`
  depuis `extension/rag3weaver`. Pour bâtir contre un build local, on écrit `RAG3DB_LIB=bati`
  avec `RAG3DB_BUILD=<dossier de build>` ; la lib est alors lue dans `$RAG3DB_BUILD/src`, et
  `libvector.rag3db_extension` doit être copiée à côté d'elle. **Un target cargo par session**,
  jamais sous `/tmp`, qui est un tmpfs (de la mémoire vive) : sous `~/.cache/rag3weaver-build/`.
- **Bâtir le moteur** : `cmake --build <build> -j 8 --target <cibles>`. Jamais `-j$(nproc)` : il
  faut laisser deux cœurs libres. `ninja` est un alias à `-j32` dans fish, à ne pas utiliser.
  Avec `--target <test>`, l'extension vectorielle n'est pas rebâtie : ajouter
  `rag3db_vector_extension`.
- **`poste`** (`~/.cache/rag3weaver-build/poste`, aux deux postes) : le verrou des travaux lourds.
  - `poste lourd timeout N <commande>` : le délai se place à l'intérieur, après `lourd` ; au plus
    deux lourds à la fois à luciepc. Le processus tourne en `nice` 15 et en `ionice` idle.
  - `poste mesure` : priorité exclusive, pour une mesure.
  - `poste push` : ne couvre que le dernier pas, le push.
  - Arrêter un lourd, c'est arrêter **son script** (et ses enfants, par filiation) : `systemctl
    stop` de l'unité ne tue pas les processus.
- **Garde d'âge de `run_e2e.sh`** : il refuse une lib plus vieille que le dernier commit de ses
  sources C++. `RAG3WEAVER_MOTEUR_ANCIEN=1` la lève, à n'utiliser qu'en sachant qu'aucun moteur n'a
  bougé.

## 4. Rejouer un index exactement (la méthode des deux sens)

C'est ce qui a mené au correctif 29a7bc52a. La méthode vaut pour tout défaut déterministe qu'un
index complet produit.

1. **La copie saine.** On ne lit jamais une base vivante. On la copie fermée, quand
   `fuser <base>/*` est vide, par `cp -a --reflink=always <dossier>/. <copie>/` (copie instantanée
   sur btrfs), avec base, journal, extensions et `.fts` ensemble. Celle des deux sens :
   `~/.cache/rag3db-locks/coupure-12h06/code-lourd` à luciepc.
2. **La capture.** Un binaire d'étude de rag3weaver avec `RAG3WEAVER_TRACE_CYPHER_FICHIER=<fichier>`
   (branche `trace-exacte-9bae`, à ne pas atterrir ; code dans
   `extension/rag3weaver/src/rag3db_connection.rs`, fonctions `litteral_cypher` et
   `trace_cypher_fichier`). Il écrit chaque instruction sur une ligne, avec ses paramètres en
   littéraux Cypher, et copie dans `copies/` le CSV de chaque `COPY … FROM '<chemin>'`, que
   rag3weaver efface ensuite. (`RAG3WEAVER_TRACE_CYPHER=1` existe sur master, mais n'écrit que les
   noms et tailles des paramètres, bornés à 400 caractères, dans le journal de l'hôte.)
3. **Un hôte d'essai privé** : son propre port (7992), son propre dossier d'hôte, et un service
   d'embarquement à soi (7882), pour ne jamais charger le service partagé 7878. Le script :
   `~/.cache/rag3db-tickets-notes/departage-trace.sh <étiquette> <binaire> <id de lib>` à luciepc.
   Il fait la copie dans `~/.local/share/rag3weaver/instantanes/<étiquette>`, lance l'hôte, vérifie
   la lib chargée par `/proc/<pid>/maps`, lance l'index par l'API HTTP, arrête l'hôte par SIGTERM
   (jamais SIGKILL, qui casse le journal), puis joue le contrôle des deux sens.
4. **Le contrôle des deux sens** : `tools/check_rel_directions/check_rel_directions.cpp` (sur
   master). Il lit chaque table nœud par nœud dans les deux sens et compare les relations, leurs
   bouts et leurs propriétés. Code de sortie 1 si une table diverge.
5. **Le rejeu** : `outils/rejoue.cpp`. On le bâtit avec `check_rel_directions.cpp` à côté, en
   renommant sa fonction `main` en `check_rel_directions_main` par sed, puis
   `c++ -std=c++20 -O1 rejoue.cpp -I. -I<lib> -L<lib> -Wl,-rpath,<lib> -lrag3db`.
   Usage : `rejoue <copie>/base.rag3db <trace> <dernière ligne, 0 = toutes> <intervalle> <tables…>`.
   Il exécute la trace ligne à ligne et contrôle les tables nommées toutes les N écritures qui les
   touchent. Avec `REJOUE_DEPUIS=<ligne>`, il contrôle après chaque écriture à partir de cette
   ligne. Il s'arrête au premier écart en imprimant la ligne. Il ouvre la base avec le pool de
   l'hôte, 8 Gio.
6. **La dichotomie** : `bissecte.sh` cherche le plus petit K tel que « lignes 1..K, puis
   CHECKPOINT » rougisse. Chaque essai se fait sur une copie neuve. Une dizaine d'essais d'une
   minute et demie, disque au repos.
7. **La réduction** : une fixture (la base « 1..K−1 puis CHECKPOINT », verte), puis l'instruction
   K seule, puis élément par élément. Ensuite un cas rebâti de zéro (`outils/synth.cpp`) et une
   réduction automatique des données par ddmin (`ddmin.py` à luciepc). Enfin un gtest.
8. **Les sondes du cœur**, à passer sur une copie pour écarter un défaut de pages :
   - `croise` (`~/.cache/rag3db-moteur-notes/essais-c7/croise.cpp` au portable) cherche une page à
     la fois libre et tenue par un segment ;
   - `entete.py` (`~/.cache/rag3db-locks/sonde-croise/entete.py` à luciepc) compare l'étendue de
     l'en-tête, `numDataPages`, à la fin du dernier segment.

## 5. Les tickets

- **Où ils vivent** (depuis le 11 novembre ; détail dans `docs/README.md`, section « Les tickets ») : un ticket ouvert est dans `docs/tickets/<nom>.md` ou
  dans `docs/<session>/tickets/<nom>.md`. Un ticket fermé est dans `docs/tickets/closed/<nom>.md`.
  Fermer, c'est `git mv` vers `docs/tickets/closed/` en réécrivant l'État. Un nom ne vit qu'à un
  endroit. Le nom s'écrit `AAAA-MM-JJ-<phrase-du-defaut>.md`.
- **L'en-tête** : les lignes `- **Champ** : valeur` avant la première section `## …`.
  - **État** : « ouvert … » pour un ticket ouvert. Dans `closed/`, un premier mot qui dit qu'il est
    clos : corrigé, fermé, fait, résolu, abandonné, non (reproduit), accepté, dissous, remplacé.
  - **Famille** (ouvert seulement) : `moteur` (le correctif vit dans `src/` ou les extensions C++)
    ou `produit` (rag3weaver, codeparsers, lucivy, les outils).
  - **Gravité**, **Atteignable en service** et **Touche rag3weaver**.
- **Le script** : `extension/rag3weaver/scripts/check_tickets.py`. Son témoin est
  `test_check_tickets.py`, et `check_features.sh` appelle les deux.
  - Sans argument, il contrôle les en-têtes : `VERT  tickets : …` ou `ROUGE` avec la liste des refus.
  - `--index` écrit `docs/tickets/INDEX.md`, ignoré par git, avec le dossier de chaque ticket.
  - `--lot <base>` liste ce que le lot ouvre, ferme, déplace ou change d'état, à relire avant le
    push.
  - Il passe avant tout push qui touche `docs/`.
- **Un défaut connu non corrigé a son ticket** : jamais une ligne dans un rapport seulement.

## 6. Pousser un lot (la procédure des hotfixs)

1. **Un worktree par lot**, sur une branche à soi, partie de master. Chaque commit se pousse sur
   sa branche, rien ne reste en local.
2. **Commits par chemins** : `git commit -- <chemins>`, jamais `git add -A`. Identité :
   `git -c user.email=luciedefraiteur@gmail.com commit …`, jamais l'adresse professionnelle. Aucune
   attribution à une IA dans le message. Identifiants du code en anglais ; commentaires et docs en
   français.
3. **Le témoin rouge d'abord**, joué et lu rouge, puis le correctif, puis le témoin vert.
4. **La passe** : toutes les cibles de test que le lot touche, dans un build à soi, sous `poste
   lourd`. Pour un lot qui touche le moteur, aussi la lib Rust entière de rag3weaver sous la lib
   bâtie (`cargo test --lib`) et les e2e concernés. La passe écrit `VERT` ou `ROUGE` en dernière
   ligne. **Vert veut dire : aucune ligne ROUGE dans la sortie écrite dans un fichier, et le vrai
   code de sortie 0, lu sans tube.** Jamais un compte de lignes VERT, jamais `$?` derrière un `|`.
5. **Le push dans un `if` qui exige VERT**, jamais enchaîné après la passe dans la même commande.
   Un rouge jugé « d'environnement » reste un rouge tant qu'il n'a pas été rejoué vert (faute
   commise le 8 novembre avec le lot A, poussé sur une passe rouge).
6. **`outils/push-lot.sh <worktree> <base rejouée>`** :
   - un fetch, puis une garde : master ne doit avoir reçu aucun code moteur ni test depuis la base
     que la passe a rejouée, sinon il faut rejouer ;
   - un rebase, puis `check_tickets.py` et sa relecture `--lot` ;
   - `git push origin HEAD:master` ;
   - **les trois contrôles** : `git status` propre, aucun rebase en cours, le hash présent dans le
     master distant fraîchement relu.

   Pour les docs seules : `outils/push-docs.sh`.
7. **Jamais de push en force**, même `--force-with-lease` : après un rebase, on pousse sous un nom
   de branche neuf. Jamais de suppression de branche distante, c'est une décision de Lucie.
   Jamais de push dans la commande d'un rebase : un rebase arrêté sur un conflit se lit d'abord.
8. **La fenêtre de formatage** : la CI de master est rouge sur le format depuis le 18 octobre. Un
   lot ne formate que ses propres lignes, par
   `clang-format-18 --lines` (d'après `git diff -U0`). `clang-format-18` tourne dans l'image docker
   `rag3db-clang-format-18`. Le commit de format de tout l'arbre attend sa fenêtre.

## 7. Les pièges appris

- **Fish à luciepc** : `ssh luciepc 'VAR=x cmd'` ne passe pas toujours la variable. Le 9
  novembre, `CIBLE=concurrence_test` s'est perdue et c'est `transaction_test` qui a été bâti.
  Toujours `bash -s` avec un heredoc protégé (`<<'EOF'`), et `export` des variables.
- **`g1.sh` contre `g1c.sh`** : seul `g1c.sh` lit `CIBLE` et `CHEMIN`. `g1.sh` bâtit toujours
  `transaction_test`.
- **Les textes avec accents, accents graves ou `$(…)`** passent par un fichier écrit à part ou par un
  heredoc protégé, jamais par une chaîne citée dans la commande.
- **Ne tuer que ses propres processus**, retrouvés par filiation (`ps --ppid`, `ps -g <sid>`) ou par
  `$!`, jamais par `pkill` ou `pgrep -f <motif>` : `pgrep -f` se trouve lui-même.
- **Un guet** (une boucle qui attend la fin d'un travail) doit couvrir tous les états de fin.
  Sa condition se vérifie sur la vraie forme des lignes : le journal de l'hôte met un horodatage
  en tête de chaque ligne, et `strings` coupe sur les accents.
- **Le disque de luciepc** : un NVMe plein à 85 %. Le `fstrim` hebdomadaire sans
  `--minimum 16M` le tient à 100 % pendant une heure : le 9 novembre de 1 h 35 à 2 h 28, les
  validations btrfs y prenaient 6 à 9 minutes. Avant d'accuser la charge, lire
  `/proc/pressure/io`. Un rouge de durée sous pression E/S est une attente disque, et un temps
  mesuré là ne compte pas, seul le verdict.
- **Un appel refusé a pu s'exécuter** : en vérifier les effets avant de le refaire.
- **`/tmp` est de la mémoire vive** : rien de gros dedans. Le scratchpad des sessions y vit et
  disparaît : les outils qu'on veut garder vont dans `docs/…/outils/` ou sous `~/.cache/`.
- **Une lib et un binaire se prouvent** avant toute mesure : le md5 de la lib, le build id lu dans
  le binaire, et `/proc/<pid>/maps` d'un hôte lancé.

## 8. Mes tickets ouverts

Ce sont les tickets que la session hotfixs a ouverts ou portés. Le 11 novembre, cinq ont été
déplacés dans `docs/coeur-cpp-hotfixs-tickets/tickets/` ; trois avaient déjà été pris par la
session qui les tient.

Dans `docs/coeur-cpp-hotfixs-tickets/tickets/` :

- `2026-11-08-le-create-vector-index-concurrent-lit-une-ligne-au-dela-de-la-colonne-en-cache.md` : un
  dépassement de tampon trouvé par l'ASAN du cœur sous un CREATE VECTOR INDEX concurrent, pas
  encore traité.
- `2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md` : faire
  lire `rows_outside_graph` sans écriture, après G3 et G5.
- `2026-11-07-un-rang-deplace-pendant-un-create-n-est-pas-toujours-rattrape-premier.md` : un cas
  inscrit aux attendus probabilistes, la cause n'est pas écrite.
- `2026-10-25-la-ci-de-master-est-rouge-depuis-le-18-octobre-cassert-dans-geo-et-format-clang.md` :
  le point 1 est fait, le point 2 (le format) attend sa fenêtre.
- `2026-10-10-ouverture-sans-une-extension-dont-une-table-depend.md` : G3 a fait le chargement de
  toutes les extensions à l'ouverture ; reste à trancher le refus avec option.

Ailleurs :

- `docs/coeur-cpp/tickets/2026-11-08-un-index-complet-croise-les-deux-sens-des-petites-tables-de-relations.md` :
  corrigé en 29a7bc52a, 0 écart par l'hôte ; le cœur le ferme.
- `docs/recherche/tickets/2026-10-18-les-tests-ne-nettoient-pas-leurs-dossiers-temporaires.md` : le
  lot de l'équipier (`agent/tests-bases-dans-tmp`) en fait une partie.
- `docs/everything-declarative/tickets/2026-10-25-le-temoin-des-vues-de-l-ide-rougit-sous-le-lot.md` :
  relevé pendant un rejeu de push, pas repris.

## 9. Les outils joints (`outils/`)

- `rejoue.cpp` : le rejoueur de trace (section 4).
- `synth.cpp` : le cas des deux sens rebâti de zéro, avec trois modes de versement (`copy`,
  `lots:<n>`, `une`).
- `push-lot.sh` et `push-docs.sh` : le dernier pas d'un lot et d'un lot de docs (section 6).
  Leurs chemins de worktree sont ceux du portable ; à adapter.
