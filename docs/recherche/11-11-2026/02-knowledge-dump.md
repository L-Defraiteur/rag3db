# Session « recherche » : ce qu'il faut savoir pour reprendre (11 novembre 2026)

Tout ce qu'un agent sans mémoire de nos échanges doit savoir pour reprendre le périmètre de la
session « recherche » : la recherche de rag3weaver, l'arrêt des backends et de l'hôte, les tests
e2e et leur passe, les scripts, les pièges, les mesures, les tickets. L'état des chantiers est dans
`docs/recherche/11-11-2026/01-rapport-de-session.md`.

Les chemins de code sont relatifs à `extension/rag3weaver/` sauf mention contraire.

## 1. Les pièces

- **rag3db** : la base de graphe en C++ (fork de Kuzu), à la racine du dépôt. Elle produit
  `librag3db.so`, la **lib**.
- **rag3weaver** (`extension/rag3weaver/`) : la crate Rust qui lie la lib et fournit les
  catalogues, l'indexation du code, la recherche, les outils des agents (un serveur MCP), l'hôte
  et l'IDE web.
- **Backend** (`src/backend.rs`, `Backend`) : un manifeste JSON ouvert (`backend.json` : entités,
  outils, modèles, signaux de recherche) avec sa base, son **catalogue** (`src/catalog.rs`,
  `Catalog` : le schéma, les écritures, les index) et ses réactions.
- **Outil** : un graphe de nœuds de dataflow écrit en Mermaid (`templates/tools/*.mmd`), exécuté
  par `src/dataflow/`. Exemples : `search_code` (`templates/tools/search_workspace.mmd`),
  `recall_fiches` et `recall_apports` (`templates/tools/search_structured.mmd`).
- **L'hôte** (`src/daemon_host.rs`, `DaemonHost`) : le démon qui tient plusieurs backends ouverts
  et sert les clients (MCP, IDE) par HTTP, port 7970 sur le portable. Il tient le **journal des
  runs** (`src/daemon_host/journal_des_runs.rs`) : un fichier `runs_root/<backend>/<run>.json.zst`
  par appel d'outil ou tour de chat.
- **L'IDE** : la page web de l'hôte, en React (`ui/ide/`), bâtie en `templates/ide/app/` par
  `scripts/ide_bundle.sh`. Le bundle bâti est commité : `scripts/ide_bundle.sh --check` le
  compare au code source.
- **Le bus et `Backend::run`** : les événements d'un run (`src/events.rs` : `RunStarted`,
  `RunFinished`, `ToolCallStarted`…) sont publiés sur un `EventBus` par sujet (`agent`, `dataflow`,
  `reactions`, `catalog`). Le journal des runs s'y abonne, un fil par sujet. `Backend::run` et
  `Backend::call` exécutent un outil. Un lot qui touche le bus, `Backend::run` ou l'hôte rejoue
  `e2e_lectures`.

## 2. La recherche

Une recherche (`search_code`, `recall_fiches`…) passe par un graphe de nœuds : `SearchSourceNode`
résout la cible, puis chaque **signal** cherche de son côté, puis `FuseResultsNode` fusionne, puis
`ResolveParentNode` remonte du morceau au document.

- **Les signaux** :
  - **plein texte** (`BM25SearchNode`, avec lucivy) ;
  - **dense** (la similarité de vecteurs, avec l'extension vector du moteur) ;
  - **creux** (sparse : les poids par mot de bge-m3).
- **lucivy** : le moteur de plein texte (dépôt `L-Defraiteur/lucivy`, crates `lucivy-core` et
  `ld-lucivy` 4.3.2). Une requête part **entière** : un `contains` relâché à distance 1, qui sort
  aussi sur le port `exact` sous l'étiquette `bm25_exact`. Plusieurs mots partent aussi par
  l'analyse booléenne. Les résultats de la requête entière passent devant (`dominant`).
- **Les bornes de la requête longue**, mesurées : au-delà de `MAX_FUZZY_WHOLE_QUERY_CHARS` = 900
  caractères, la requête entière perd son flou ; la recherche par mots garde au plus
  `MAX_WORD_TERMS` = 64 termes ; le budget est de 500 ms.
- **La fusion** : RRF (`fuse_signals`, `src/search.rs`). Chaque signal apporte poids / (k + rang),
  avec k = 60. Les poids par défaut sont bm25 0,3, vector 0,7 et sparse 0,2 ; le gabarit
  `search_structured` déclare bm25 0,45, vector 0,55 et sparse 0,4.
- **Une ligne de résultat** (`UnifiedResult`, `src/dataflow/resultat.rs`) porte :
  - `score`, le rang fusionné, de l'ordre de 0,01 ;
  - `signals[]`, avec pour chaque signal son score brut et son rang ;
  - `bestSignal`, le signal qui pèse le plus dans le rang ;
  - `chunk`, le morceau montré, avec `chunk.score`, `chunk.signal` (l'échelle de ce score) et
    `chunk.highlights[{start, end, source}]`, en unités UTF-16 dans `chunk.text`.
  Le morceau montré est celui du signal qui pèse le plus dans le rang, plus le plus grand score
  brut. Les scores bruts ne se comparent pas d'un signal à l'autre : aucun seuil ne se pose sur
  eux.
- **Le surlignage** (`HighlightNode`) : les positions de lucivy, sinon le creux de bge-m3 calculé
  à l'affichage (`/embed_sparse` avec `"spans": true` du service, `embed_sparse_spans`), sinon les
  mots de la requête. `models.sparse` n'embarque qu'au surlignage, jamais en continu.
- **L'instantané L6** : un appel lit sous une transaction READ ONLY du moteur (`snapshot_safe`).
  La recherche filtrée y lit aussi, depuis la lib dadbf0041.
- **Les outils de recherche** d'un backend sont ceux dont le graphe porte un nœud de
  `SEARCH_NODES` (`src/backend_code.rs`, `SearchSourceNode`), sans nom d'outil en dur.
- **Mes recherches** (vue `ui/ide/src/views/Recherches.tsx`) : elle lit les appels journalisés des
  outils de recherche (`appel {client, humain, outil, arguments, rendu}` dans le journal des runs).
  Un clic se range dans le run de sa recherche (`JournalDesRuns::append_to_list`). Une page lit au
  plus 2 000 runs (`RUNS_READ_PER_PAGE`).

## 3. L'arrêt des backends et de l'hôte

- **`Backend::shutdown()`**, dans cet ordre :
  1. attendre les réactions, avec un délai (`RAG3WEAVER_STOP_WAIT_S`, 30 s) ; une réaction figée
     laisse le catalogue ouvert et se dit ;
  2. arrêter au prochain paquet et attendre les fils de fond du catalogue ;
  3. `Catalog::shutdown()`, qui vide la vue de lecture puis ferme le plein texte.
- **Le registre des fils de fond** (branche `arret-du-backend`, pas encore sur master) :
  `src/catalog_threads.rs`.
  - Ses fonctions : `register`, `running`, `latest`, `stop_and_join`.
  - Chaque fil s'inscrit avec son drapeau d'arrêt (lu entre deux paquets) et son genre :
    `Index` (une indexation) ou `FtsRebuild` (le rebâti du plein texte).
  - Le registre tient le catalogue par un `Weak` : un fil fini ne le garde pas en vie.
  - `index_nodes::indexings_running` ne compte que les index : l'hôte s'en sert pour refuser de
    fermer un backend qui indexe (`BUSY_INDEXING`). `stop_indexings_of` arrête et attend tous les
    fils.
- **`impl Drop for Backend`** (même branche) appelle `shutdown()`, une seule fois (résultat de tentative mémorisé).
  Sans lui, un backend lâché sans `shutdown()` laissait ses réacteurs (tâches tokio) et ses fils
  tenir le catalogue, qui se fermait plus tard, sur un autre fil, une fois son dossier effacé.
- **Côté lucivy**, `ShardedHandle::close` sortait sur la première erreur sans arrêter ses pools
  d'acteurs : PR #23 de lucivy.
- **L'hôte** : `close_slot` (route `/backend/close`) appelle `shutdown()`. Le journal des runs écrit
  un run 250 ms après sa fin, dans un fil détaché `journal-ecrire` que l'arrêt de l'hôte n'attend
  pas. C'est le ticket `2026-11-09-le-journal-des-runs-ecrit-apres-l-arret-de-son-hote.md`.
- **Le cache d'index du catalogue** (`catalog::index_cache_dir`) : `RAG3WEAVER_CACHE_DIR`, sinon
  `~/.cache/rag3weaver/cache`. Les crates lucivy et sparse-vector y posent
  `<pid>/<index>_<n>`. À l'ouverture d'un catalogue, les dossiers `<pid>` des processus morts
  sont effacés (`sweep_dead_index_caches`, par `daemon_lifecycle::pid_state`).

## 4. Les tests e2e

### La forme

- **Un seul binaire e2e** : `tests/e2e/main.rs` monte chaque `tests/e2e_*.rs` comme module. Il est
  généré par `tests/e2e/generer.py` ; une suite neuve demande `python3 tests/e2e/generer.py` et
  `git add`.
- **Un test e2e porte `#[ignore]`**, sinon il ne tourne jamais (`run_e2e.sh` passe
  `--include-ignored`).
- **Les témoins navigateur** portent
  `#[cfg_attr(not(feature = "navigateur"), ignore = …)]` et se jouent à part, par
  `scripts/temoins_navigateur.sh` ; Chrome est exigé.

### `run_e2e.sh`

- Il bâtit une fois, puis joue **un processus par suite**, chacune avec un `TMPDIR` neuf sous
  `/tmp/rag3weaver-e2e-<pid>/<suite>`, et un dossier disque `~/.cache/rag3weaver-build/e2e-<pid>/<suite>`
  (`RAG3WEAVER_E2E_DOSSIER`, `RAG3WEAVER_CHECKPOINT_DIR`, `RAG3WEAVER_CACHE_DIR`).
- **Un seul filtre** : `--test <suite>` choisit la suite, et seul le dernier nom positionnel filtre
  les tests. Une suite par appel.
- **Suites écartées en le disant** (« NON JOUÉES ») :
  - les suites à vrai modèle (`RAG3WEAVER_SUITES_A_MODELE=1` pour les jouer) ;
  - la carte locale ;
  - les **bancs à services** (`BANCS_A_SERVICES`, aujourd'hui `e2e_banc_fiches_classees`, qui exige
    `RAG3WEAVER_BANC_DECIDE` ou `RAG3WEAVER_BANC_VERTEX`) : jamais dans l'e2e entier, ils se jouent
    seuls avec `--test` et leur voie ;
  - et, dans la branche `arret-du-backend`, `RAG3WEAVER_E2E_SAUF="<suite> …"`, pour une mesure qui
    tient la carte (`e2e_charge_ingestion`).
- **Une suite non jouée faute de feature est ROUGE.** Lire « running N tests » de chaque suite.

### Le témoin des résidus

Fonction `residus_de_la_suite` de `run_e2e.sh` (lot `92180836b`) : avant d'effacer le `TMPDIR`
d'une suite, il le regarde.
- Une suite qui y laisse quelque chose est **ROUGE** : il en liste les chemins, les tailles et le
  contenu.
- Sauf si `tests/e2e/suites_qui_laissent_des_residus.txt` la nomme. Deux formes d'entrée :
  - `<suite>` seule : dette fixe. Elle en laisse à chaque passe, et si elle finit propre, elle est
    **rouge** aussi, pour que la liste maigrisse. Cette règle ne s'applique pas sous un filtre de
    tests ;
  - `<suite> <chemin du ticket>` : **rouge connu**, un résidu intermittent dont la cause est en
    ticket. Il se dit « résidu connu : <suite>, <ticket>, <contenu> », et propre reste propre.
- Une entrée sort de la liste dans le commit qui corrige sa cause, prouvé par plusieurs passes
  propres.

État de la liste :
- **Sur master** : `e2e_code`, `e2e_prise_atomique`, `e2e_rag3daemon`, `e2e_repli_de_branche` et
  `e2e_rouvrir` en dette fixe (des bases de test entières), plus `e2e_cited_by`, `e2e_juge` et
  `e2e_lectures` en rouges connus.
- **Dans `arret-du-backend`** : les cinq de dette fixe, plus `e2e_juge` en rouge connu, son ticket
  étant celui du journal des runs.

### Comment on les joue

- **À luciepc**, sous `poste lourd`, jamais sur le portable sans l'accord de l'orchestration.
- **Lib épinglée** : `RAG3DB_LIB=<id complet>`, jamais `courant` (le lien du magasin de libs que
  l'orchestration fait basculer). Si le C++ de master est plus récent que la lib, ajouter
  `RAG3WEAVER_MOTEUR_ANCIEN=1`, sinon `run_e2e.sh` refuse (« librag3db.so est plus vieux que le
  dernier commit de ses sources »).
- **Jeu de features large** : `rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm`
  (`run_e2e.sh --features openai-llm,anthropic-llm` ajoute les deux derniers).
- **Services d'embarquement** (`RAG3WEAVER_EMBED_SERVICE`) : à luciepc, `127.0.0.1:7878`
  (granite-278m), `7879` (bge-m3), `7880` (granite-107m) ; sur le portable, les tunnels
  `127.0.0.1:7979,7980,7981`. Un test qui embarque sans service panique : un test sauté n'est pas
  vert.
- **Postgres de test** à luciepc : `RAG3WEAVER_PG=postgres://rag3weaver:rag3weaver@127.0.0.1:5433/rag3weaver_test`
  (`e2e_postgres`, à jouer si un gabarit, `code.rs` ou l'IR change).
- **Régime GPU** : hors de luciepc, `RAG3WEAVER_EMBED_CHAR_BUDGET=4096` et `RAG3WEAVER_GPU_DUTY=70`
  (le régime doux, qui ménage l'écran de Lucie).
- **Un e2e entier qui embarque est un lourd pour la carte** : il passe quand la carte est à
  personne. L'orchestration peut demander de le suspendre ; on arrête alors le groupe entier de
  `run_e2e.sh`.

## 5. Les scripts de la session

Les originaux sont sur le portable, dans `/home/lucied/.cache/rag3weaver-build/`. On les édite là,
puis on les copie à luciepc par un fichier temporaire et un `mv`, jamais en écrasant un script qui
tourne.

- `passe_epinglee.sh <worktree> <journaux> <filtre du témoin> [temoins|-]` (à luciepc). Variables :
  `CIBLE=<target>`, `SUITE_DU_TEMOIN`, `SUITES_EN_PLUS`. Il joue :
  1. le témoin ;
  2. `e2e_lectures`, `e2e_code` et les suites en plus ;
  3. `scripts/check_features.sh` (cinq jeux de features) ;
  4. la preuve que rag3weaver a bien été compilé (« Checking rag3weaver ») ;
  5. la lib entière (`cargo test --lib`) ;
  6. les témoins navigateur.
  La dernière ligne de `<journaux>/resume.log` dit `VERT` ou `ROUGE <étape>`.
- `push_garde.sh <worktree> <journaux>`, sous `poste push rag3db-97 -- …` à luciepc. Il pousse
  `HEAD:master` seulement si :
  - la passe est VERTE sur ce commit (ou sur un commit dont il ne diffère que par des `.md`) ;
  - une seule lib a servi à toute la passe ;
  - aucun code n'est arrivé sur master depuis la base testée (`<journaux>/base`) ; le C++ du
    moteur et les `.md` ne comptent pas ;
  - aucun fichier `~/.cache/rag3weaver-build/master-tenu` n'existe.
  Puis il fait les **trois contrôles** : `git status` vide, aucun rebase en cours,
  `FETCH_HEAD = HEAD`, et il écrit `POUSSE`.
- `lib_seule.sh <worktree> <target> <journal> <filtre>` : un test de la lib seul. `FEAT=` règle les
  features ; ajouter `navigateur` pour un témoin navigateur.
- `e2e_entier.sh <worktree> <target> <journal>` : toute la passe e2e. `run_e2e.sh` y part dans sa
  propre session (`setsid`) ; à l'échéance (`DELAI_E2E`, par défaut 18 000 s), ou si le script est
  arrêté, **le groupe entier** reçoit TERM puis KILL.
- **Le verrou du poste** : `~/.cache/rag3weaver-build/poste`.
  - `poste lourd timeout N <cmd>` : une place de lourd (au plus 4 à luciepc, 1 sur le portable),
    le délai dans la commande.
  - `poste push <nom> -- <cmd>` : le verrou du push, pour le dernier pas seulement.
  - `poste mesure` : la machine seule, qui gèle la file des lourds.
  - `poste --qui` : qui tient quoi, à lire avant de lancer.

## 6. Les procédures

`docs/procedures/push-d-un-lot.md` est la référence.

En bref :
1. un témoin rouge d'abord, prouvé sur le code d'avant, pour la bonne raison ;
2. la passe à luciepc sous `poste lourd`, lib épinglée ;
3. relire les tickets du lot (`python3 extension/rag3weaver/scripts/check_tickets.py --lot origin/master`) ;
4. rebaser sur le master du moment ; s'il a apporté du Rust, un rejeu court (check_features, la lib
   entière, les e2e du changement) ;
5. le push sous `poste push`, dans `if grep -qx VERT` ;
6. les trois contrôles, puis le hash à l'orchestration.

Les règles qui vont avec :
- un commit par chemins (`git commit -- <chemins>`) ;
- l'identité `luciedefraiteur@gmail.com` posée dans chaque worktree ;
- jamais de ligne d'attribution à Claude ;
- jamais de push en force : après un rebase, un nom de branche neuf (`-2`, `-3`) ;
- jamais `git stash` ;
- jamais de suppression de branche distante (c'est la décision de Lucie) ;
- un worktree par lot, effacé après l'atterrissage avec sa branche locale ;
- un target cargo par lot (`target-97<lettre>` à luciepc, plafond 32 Go) ;
- jamais rien dans l'arbre de Lucie (`/home/lucied/git_workspaces/rag3db` reste sur master).

Les **tickets** (`docs/tickets/`) ont un en-tête obligatoire : `État`, `Famille`, `Gravité`,
`Atteignable en service`, `Touche rag3weaver`. Un ticket fermé va dans `docs/tickets/closed/`,
avec un `État` qui commence par « corrigé » ou « fermé ».

## 7. Les pièges appris

- **Un `timeout` ne tue que son fils.** `poste lourd timeout N` laisse `run_e2e.sh` et son binaire
  orphelins sur la carte. Après toute échéance, chercher ses restes par l'exécutable du target
  (`readlink /proc/<pid>/exe`) et le dossier de travail, puis arrêter l'arbre de leurs pids,
  **jamais par nom** (`pkill`, `pgrep -f` se trouvent eux-mêmes et touchent les autres sessions).
- **À luciepc, ssh tourne sous fish** : `ssh … bash -s <<'EOF'` ou `bash -c '…'`. Sur le portable,
  l'outil de commande est zsh : une variable non citée ne se découpe pas.
- **Un départage qui bascule de commit** doit vérifier `HEAD` avant de jouer : une bascule refusée
  par un fichier modifié joue sinon l'ancien commit des deux côtés.
- **Un embarqueur factice est refusé à l'ouverture d'un backend** (l'identité du modèle diffère du
  manifeste) : sans service, un test ouvre avec `None`, l'embarqueur « absent ».
- **Un rouge se prouve sur la base sans le correctif.** Si le correctif est déjà commité, appliquer
  sur master les commits de témoin sans les valider (`git cherry-pick --no-commit`, avec les
  témoins dont il dépend), jouer, puis revenir à la tête.
- **Une passe sous charge disque ne se juge pas** : le 8 novembre, le NVMe de luciepc en TRIM
  asynchrone rendait des rouges de délai. On lit `/proc/pressure/io` avant de mesurer.
- **Une suite qui embarque longtemps prend la carte à tout le poste**, et les rappels de Lucie
  ralentissent avec elle.
- **Copier une base pour la mesurer** : `.rag3db`, `.wal`, `.extensions` et `.fts` ensemble, sans
  écrivain (hôte arrêté, ou par instantané) ; comparer le nombre de documents avant de mesurer.

## 8. Les mesures et leurs chiffres

- **Banc de charge d'ingestion** (`e2e_charge_ingestion`), 8 novembre à luciepc : 14 012 scopes en
  6 380 325 ms (1 h 46), la carte card2 à 94 % en moyenne, card0 à 82 %, en régime Confort
  (budget 8 192, rapport cyclique 100 %). Le 9 novembre, arrêté après 1 h 14.
- **Le Surveillant**, seul, dix essais : master 5 rouges sur 10 (charge 17 à 23), branche 6 sur 10.
- **Requête longue de lucivy 4.3.2** : 21 ms à 104 caractères, 464 ms à 933, 785 ms à 1 397,
  d'où L = 900 au budget de 500 ms.
- **Résidus de master**, 8 novembre, avant le lot des résidus : 70 suites en laissaient, dont 64
  le seul cache d'index ; `/tmp/rag3weaver_cache` tenait 96 dossiers de pid, 119 Mo.
- **Index vectoriel**, lignes hors du graphe (banc 28b479b7e) : sur 120 000 vecteurs de dimension
  768, la recherche passe de 13,4 ms (aucune ligne hors du graphe) à 88,5 ms (100 000), soit
  0,75 à 1 µs par ligne. Seuil de reconstruction proposé : environ 13 400 lignes, pour que la
  recherche ne double pas.

## 9. Mes tickets ouverts, une ligne chacun

- `2026-11-09-le-journal-des-runs-ecrit-apres-l-arret-de-son-hote.md` : le fil `journal-ecrire`
  détaché écrit après l'arrêt de l'hôte (dans la branche `arret-du-backend`).
- `2026-11-08-le-temoin-de-l-embarquement-hors-verrou-rougit-sous-charge.md` : le Surveillant,
  5 rouges sur 10 sous charge.
- `2026-11-08-un-backend-ecrit-encore-dans-son-dossier-apres-son-drop.md` : corrigé par
  `arret-du-backend`, qui le ferme.
- `2026-11-09-le-banc-de-charge-d-ingestion-tient-la-carte-pres-de-deux-heures-dans-l-e2e-entier.md` :
  ses chiffres de débit, et sa place sous `poste mesure`.
- `2026-10-18-les-tests-ne-nettoient-pas-leurs-dossiers-temporaires.md` : le témoin des résidus et
  la dette déclarée.
- `2026-11-08-le-plein-texte-de-code-lourd-compte-plus-de-documents-que-de-lignes.md` : relevé par
  G, à regarder avec lucivy.
- `2026-11-07-une-requete-floue-de-lucivy-coute-en-proportion-de-ses-termes.md` : côté lucivy,
  bornée côté rag3weaver.
- `2026-11-07-un-arret-ne-coupe-pas-une-requete-du-moteur-en-cours.md` : l'arrêt d'un run ne coupe
  pas l'appel au moteur en cours.
- `2026-10-25-le-temoin-du-chat-rougit-encore-sur-le-tour-lent.md` : témoin du chat rouge sous la
  lib entière, pas repris.
- `2026-11-07-search-structured-le-score-d-une-ligne-n-est-pas-comparable-d-un-signal-a-l-autre.md` :
  ma part faite (`bestSignal`), le seuil est à archi.
- **À ouvrir** : « un COPY tué laisse son CSV dans le dossier temporaire »
  (`e2e_tx_par_paquet_arret`, `rag3weaver-lignes-*.csv` de `record_nodes.rs`).

## Précisions de reprise du 11 novembre en fin de journée

- La lib 29a7bc52a passe la garde sans `RAG3WEAVER_MOTEUR_ANCIEN` : ne pas poser ce drapeau par précaution.
- Pour une passe incluant les vrais tests du juge : `GOOGLE_CLOUD_PROJECT=lr-hub-472010`, `GOOGLE_APPLICATION_CREDENTIALS=$HOME/git_workspaces/rag3db/.vault/vertex-sa.json`, `RAG3WEAVER_EXIGER_GEMINI=1`. Le crate lit le fichier ; ne jamais en afficher le contenu. Cargo peut annoncer `passed` alors que le test a écrit `SAUTÉ` et est revenu : lire les journaux.
- Les suites transactionnelles existantes sont `e2e_tx_par_paquet_arret`, `e2e_tx_vecteurs_apres_rollback`, `e2e_tx_merge_puis_copy`, `e2e_tx_ligne_a_ligne` ; `e2e_tx_par_paquet` n'existe pas.
- Branches et worktrees de reprise prennent désormais le préfixe `cx-`. Le rapport actualisé donne l'état des passages et la relecture des vues ; les anciens chemins restent historiques.

- Pour le lot d'arrêt, les témoins `qu_on_laisse_tomber` sont des tests **unitaires** de `daemon_host`, pas des e2e_lectures. Utiliser `scripts/cargo_magasin.sh test --lib --features <jeu-large> qu_on_laisse_tomber -- --nocapture`, lire les deux noms et `2 passed; 0 failed; 0 ignored`. Les deux filets du lanceur e2e ne comptent pas pour ce témoin. Le script de reprise corrigé est `~/.cache/rag3weaver-build/cx-recherche-arret-passe-v2.sh` ; le vérificateur final produit `verdict-final.log` en lisant les résultats de chaque cible séparément.

- `Backend::shutdown(&mut self)` conserve désormais le résultat de sa tentative unique (`Option<Result<(),String>>`), pas un booléen ambigu. Après réaction figée, les handles ont été détachés : un second appel ne retente pas une fermeture et restitue la même erreur ; le catalogue n'est pas garanti fermé. Le Drop signale cette erreur conservée. Après succès, le second appel et le Drop ne refont pas la fermeture. Le test de gel est isolé dans un enfant avec sa propre variable de délai, pour ne pas bloquer le runtime partagé des autres tests.
