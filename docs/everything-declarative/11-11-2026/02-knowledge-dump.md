# Knowledge dump — le déclaratif de rag3weaver, pour reprendre sans mémoire

*Écrit le 11 novembre 2026 par la session « everything declarative » (ED). L'état des travaux en
cours est dans `01-rapport-de-session.md`, à côté. Ici : comment les pièces marchent, où elles sont,
comment on travaille, et ce qu'on a appris à nos dépens. Tous les chemins de code sont relatifs à
`extension/rag3weaver/` dans le dépôt rag3db, sauf mention.*

## 1. Le modèle : un backend déclaratif

- **Un backend** = un manifeste JSON (`backend.json`) qui déclare une base rag3db, des entités, des
  modèles (embarquement, creux, LLM) et des **outils**. Le binaire ne contient pas les outils : il
  exécute ce que le manifeste déclare (`src/backend.rs`, `BackendManifest`, `PreparedBackend::load`).
- **Un outil** = un graphe de nœuds écrit en Mermaid dans un fichier `.mmd` (`templates/tools/*.mmd`).
  En tête, des directives : `%% tool:`, `%% description:`, `%% param: nom type = défaut -- aide`,
  `%% result: <nœud>.<port>` (le port qui porte le résultat). Chaque nœud est un type Rust enregistré
  (`src/dataflow/*_nodes.rs`, par exemple `RunCommandNode`, `IndexNode`, `WorktreeNode`).
  `Backend::call(nom, arguments)` exécute le graphe et rend `{result, metadata, run}`, plus
  `presentation` (un port `text` du nœud résultat) et `ui` (un port `ui`, § 6) s'ils existent.
- **Les gabarits livrés** : tout `templates/` est embarqué dans le binaire
  (`src/builtin_templates.rs`, `build.rs`). Une valeur `builtin:tools/x.mmd` dans un manifeste
  désigne le fichier livré de ce nom ; il suit donc chaque bâti. Les principaux manifestes livrés :
  `templates/backends/code/backend.json` (un dépôt de code indexé) et
  `templates/backends/atelier/backend.json` (l'atelier, § 4).
- **`workspace`** (`src/backend_code.rs`, `WorkspaceConfig`) : la source de fichiers d'un backend.
  Clés utiles : `root` (sa racine), `read_only`, `task_writes` (les écritures seulement dans une
  tâche), `git_init`, `commands` (la porte des commandes : `off`, `standard`, `approval`, `auto`),
  `commands_outside_tasks` (la porte ouverte hors d'une tâche, pour l'atelier), `index` (`"code"`
  enregistre le schéma de code), `sandbox` (le bac à sable Landlock des commandes : `mode`,
  `network`, `extra_read`, `extra_write`, `env`), `scripts`, `generated`, `locks_via`, `queries_via`.
  `deny_unknown_fields` : un champ inconnu refuse le chargement.
- **`embeddings`** (`EmbeddingService`) : `model`, `dimensions`, `address` (vide : la variable
  d'environnement `RAG3WEAVER_EMBED_SERVICE`), `catch_up_on_open` (§ 3). Une adresse déclarée
  l'emporte sur la variable.
- **`chat_instructions`** : un texte du manifeste ajouté au message système du chat de ce backend.

## 2. Le gabarit vivant (`src/builtin_templates.rs`)

But : un manifeste posé d'après un gabarit livré suit les bâtis sans geste manuel (décision de Lucie
le 8 novembre).
- **`record(manifeste, nom_livré)`**, appelé juste après qu'une pose écrit le manifeste
  (`code_tree::add_files`, l'installeur de l'atelier) : écrit `livre.json` avec le nom du gabarit,
  son contenu d'alors et l'empreinte blake3 du fichier écrit.
- **Où vit cet état** : `state_dir_of(manifeste)` = `$RAG3WEAVER_STATE_DIR`, sinon
  `$XDG_STATE_HOME/rag3weaver`, sinon `~/.local/state/rag3weaver`, puis
  `manifestes/<16 premiers caractères du blake3 du chemin canonique>/`. **Jamais à côté du
  manifeste** : un fichier en plus dans l'arbre des déclarations est, pour une tâche, une
  déclaration inconnue, et sa fusion est refusée (appris le 8 novembre).
- **`refresh(manifeste)`**, appelé par l'hôte à chaque ouverture (`daemon_host.rs`, `open_slot`),
  rend `Refresh` :
  `NotRecorded` (pas de `livre.json`) ; `EditedByHand` (le fichier n'a plus l'empreinte écrite :
  gardé) ; `TrackedByGit` (le manifeste est suivi par un dépôt git, `git ls-files --error-unmatch` :
  gardé, il ne se réécrit pas hors revue) ; `UpToDate` ; `Refreshed { added, removed }` : fusion à
  trois sur les feuilles JSON (le livré neuf + ce que la pose avait changé au livré d'alors), les
  chemins d'outils sans copie locale deviennent `builtin:`, et `TemplateRefreshed` passe sur le flux
  `repos`.
- Limites connues : ne monte que le manifeste (pas les `.mmd` copiés) ; une copie posée avant
  `livre.json` n'est jamais suivie. La page M8
  (`docs/09-novembre-2026-memoire-longue/02-m8-la-montee-des-gabarits.md`) propose de les lever.

## 3. L'hôte, ses backends, l'index et le rattrapage

- **L'hôte** (`src/daemon_host.rs`, `DaemonHost`) tient les backends en mémoire seulement : la table
  `backends`, une entrée `Hosted` par manifeste (chemin canonique). `hosted()` crée l'entrée sans
  ouvrir ; `with_backend()` ouvre au besoin (`open_slot`) et prête le backend ; `close()` (route
  `POST /backend/close`, exige une session) ferme (`close_slot`). Aucun état des backends ne survit
  à un redémarrage, sauf ce que les tâches et les sessions de chat relues rappellent.
- **`open_slot`**, dans l'ordre : `builtin_templates::refresh`, chargement, refus si un écrivain
  autonome tient la base, ouverture, pose de l'hôte des worktrees (atelier), relais du bus vers le
  flux de l'IDE, journal des runs, puis `worktrees::a_l_ouverture`.
- **L'index** (`src/dataflow/index_nodes.rs`) : l'outil `index` lance un fil
  (`spawn_stoppable_index`), qui écrit un journal (`dossier_journaux()/index/<ms>-<pid>.out`). Le
  contrat : **la dernière ligne** est `DONE_LINE` (« indexation terminée », éventuellement
  « — <reste dû> »), ou commence par `FAILED_PREFIX` (« indexation échouée ») ou `STOPPED_PREFIX`
  (« indexation arrêtée »). Rien ne s'écrit après. Deux temps : le plein texte, puis les vecteurs en
  passes de 512 morceaux par table (`embarquer_le_retard_suivi`). `indexings_running(catalog)` compte
  les index vivants d'un catalogue ; `stop_indexings_of(catalog)` les arrête au paquet suivant et
  les attend.
- **La sonde** : avant de partir, `index` mesure le débit du service d'embarquement (deux appels :
  un texte, puis un échantillon), sauf si un débit est déjà noté. Un service qui refuse tout fait
  donc refuser l'index.
- **Le témoin du premier index**, `.premier-index` : `worktrees::marque_du_premier_index(manifeste)`
  = `state_dir_of(manifeste)/premier-index`, qui contient le chemin du journal de l'index lancé.
  Ôté quand ce journal finit par `DONE_LINE` (`premier_index_a_finir`).
- **`lancer_sur(b, manifeste, suivi)`** (`src/daemon_host/worktrees.rs`) : le seul point qui lance
  l'index d'un worktree. Il ne lance pas de second index si un tourne, pose le témoin, et branche
  le suiveur d'archi (`suivre_le_premier_index`, qui publie `IndexProgress` puis `RepoIndexed` sur
  le flux `repos` en lisant le journal). Appelé par l'attache, la reprise et l'ouverture.
- **`a_l_ouverture`** : à chaque ouverture, l'index repart en fond si le premier index a été coupé
  (le témoin), ou si le manifeste déclare `embeddings.catch_up_on_open` et que des morceaux sont
  sans vecteur. Un index ainsi reparti est marqué `Hosted.rattrapage = true` : il **ne tient pas la
  porte**, une fermeture l'arrête au paquet suivant. Un index qu'un outil attend tient la porte :
  `lancer_l_index` remet le marquage à faux (8be79cbbf), et `close_slot` refuse alors en
  `BUSY_INDEXING`.
- **La reprise après un redémarrage** : `DaemonHost::reprendre_les_premiers_index`, lancé dans un
  fil par `serve`, rouvre les worktrees inscrits dont le témoin dit un index pas fini. Un dépôt
  inscrit sans témoin (la base principale de Lucie) n'est jamais ouvert par là.
- **Les réclamations** (`src/catalog.rs`, `reclamer_le_retard_sauf`) : une passe réclame ses
  morceaux par `_embed_claim = "<horodatage>|<writer_id>"`. Une autre passe ne les prend qu'après
  `MARQUE_PERIMEE_MS` = 60 s, sauf le même écrivain. `writer_id` change à chaque ouverture du
  catalogue. Défaut ouvert : une passe coupée garde ses réclamations (correctif sur la branche
  `reclamations-rendues`, voir le rapport).
- **L'état de l'index** (`Backend::index_states`) : une entrée par entité (`text`, `vectors`,
  niveaux `never` / `running` / `ready` / `not_declared` / `failed`, `text_percent`,
  `vectors_percent`), plus `fts_counts` (documents et lignes du dernier index), `indexings` (index
  vivants), `busy` (écriture qui tient le catalogue), `watches`, `reads`, `embedding`. Une note
  `running` d'avant un arrêt, sans index vivant et avec des comptes complets, se lit `ready`
  (`text_stale_note`).

## 4. L'atelier (`templates/backends/atelier/backend.json`, `src/daemon_host/worktrees.rs`)

- Une conversation qui attache des worktrees du poste et y travaille. Installé par
  `rag3weaver-backend atelier` (`src/bin/rag3weaver-backend.rs`, `atelier_cli`), qui copie les
  gabarits sous `<config>/atelier/templates`, pose la base, la racine, note la pose.
- **Outils** : `attach_worktree` (inscrit un worktree déjà sur le disque comme arbre de code,
  `code_tree::add_files`, l'ouvre, lance son index), `list_worktrees`, `open_branch` (ouvre une
  branche dans un worktree frère par `git worktree add`, la commande dite dans la réponse),
  `show_progress`, `run_command` (par la porte), et depuis archi (9 novembre) `read_file`,
  `read_scope`, `replace_text`, `replace_lines`, `insert_lines`, `ask_permission`
  (`src/daemon_host/atelier_fichiers.rs`).
- **La porte** (`src/commande.rs`) : `approval` laisse passer une liste sûre et demande l'accord de
  l'utilisatrice pour le reste ; la décision arrive par la carte d'accord de l'IDE. Hors d'une tâche,
  le chat ne lance des commandes que si `workspace.commands_outside_tasks` est vrai
  (`OutilsDuChat`, `src/daemon_host/chat.rs`).
- **Le bac à sable** : chaque commande passe par Landlock (`backend_code::build_bac_a_sable`), le
  domaine en écriture étant la racine du workspace. Décisions de Lucie, pas toutes codées : la racine
  par défaut `~/rw-code` (codée par archi, d360896d9) ; un réglage `home` / `atelier` du domaine ;
  « dehors » comme extension demandée par la porte et journalisée, pour un futur décideur local.
- **`writes_outside_tasks`** (archi, 76647d66d) : l'atelier écrit hors d'une tâche ; un worktree né
  de l'atelier s'écrit sans question, un autre chemin passe par `ask_permission`.
- **Le champ `ui`** (`src/llm.rs`, `Turn.ui` ; `src/events.rs`, `ToolCallFinished.ui`) : ce que la page
  rend sous un appel d'outil, jamais envoyé au modèle (les fournisseurs construisent leurs messages
  champ par champ). Un outil le pose par un port `ui` de son nœud résultat ; `BackendTools` l'ôte du
  texte. La page a un registre `kind → composant` (`UiDuTour.tsx`, archi) : `index_progress`,
  `accord`.

## 5. L'équipier par pas et le juge

- **L'équipier** (`src/daemon_host/teammate.rs`) : un agent autonome qui fait une tâche par pas,
  avec un plan. Il parle à son superviseur par **la boîte** (une file de messages par tâche) ; ses
  outils propres : `edit_plan`, `step_done`, `report`. Les commandes passent par la porte déclarée.
- **La règle de fin** : aujourd'hui, une tâche sans plan se clôt « fait sans plan » sur un
  `step_done`, sans attendre ni le livrable ni le superviseur. C'est le ticket « une tâche sans plan
  se dit faite… », à corriger (§ 7 du rapport).
- **Le juge** (`src/daemon_host/juge.rs`) : juge chaque pas sur son but ; consigne
  `consigne_du_juge` (le message du pas, tronqué à 3 000 caractères), fiche mémoire
  `nom_de_fiche_de_tache` = « bilan : <tâche> », apport par pas titré `**pas N · date**`.
- **Le client du modèle** (`src/llm.rs`) : reprise après 429 ou 503 (base 60 s, plafond 600 s,
  `RetryPolicy` à six essais et 1 800 s au total) ; depuis 96 (a6637eeee), chaque reprise se dit et
  se borne.

## 6. Les scripts et les procédures

- **Le verrou du poste** : `~/.cache/rag3weaver-build/poste lourd <cmd>` (un nombre borné de lourds
  par poste), `poste push <session> -- <cmd>` (un push à la fois, avec l'état de la CI).
- **Le rejeu court** : `~/.cache/rag3weaver-build/I-court-9bae.sh` à luciepc (lib épinglée, la lib
  entière, `scripts/check_features.sh`, le natif seul), qui écrit `VERT` ou `ROUGE` en dernière
  ligne. Il touche `src/lib.rs` pour forcer la recompilation et refuse un vert sans « Compiling ».
- **Le push** : `~/.cache/rag3weaver-build/I-pousse-l2.sh`, sous `poste push`. Il exige le verdict
  égal à `HEAD`, rebase sans rejeu si master n'a reçu que des docs ou du C++ du moteur, sinon répond
  `REJOUER`, refuse sans avance rapide, pousse `HEAD:master`, puis fait les trois contrôles.
- **Le modèle de script de lot** : `~/.cache/rag3weaver-build/I-pousse-reclame.sh` (fetch de la
  branche, détache sur master, cherry-pick, `I-court-9bae.sh`, témoins de reprise et du flux,
  verdict, push). Lancé par
  `XDG_RUNTIME_DIR=/run/user/$(id -u) systemd-run --user --unit=<nom> --collect bash <script>`.
- **Les e2e** : `./run_e2e.sh --test e2e <suite>` (un seul filtre par appel). Services
  d'embarquement à luciepc : `RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879,127.0.0.1:7880`.
  Gemini pour l'équipier : `GOOGLE_CLOUD_PROJECT=lr-hub-472010` et
  `GOOGLE_APPLICATION_CREDENTIALS=/home/lucied/git_workspaces/rag3db/.vault/vertex-sa.json`,
  jamais lu ni copié autrement.
- **Les règles** : rouge d'abord ; push seulement au vert et au « à toi » de c7 quand il tient la
  file ; trois contrôles ; jamais de push en force (une nouvelle branche `-2`, `-3` après un rebase) ;
  commit par chemins (`git add <fichiers>`) ; identité `luciedefraiteur@gmail.com`, jamais l'adresse
  pro ; aucune ligne d'attribution dans les commits ; identifiants de code en anglais, commentaires
  et docs en français ; à l'écran et dans les réponses d'outils, le vocabulaire de Lucie (dépôt,
  worktree, branche, index ; « backend » seulement dans le code) ; un seuil ne se relâche pas sans
  preuve ; un rouge rejoué seul et vert se nomme, puis la passe se rejoue.

## 7. Les pièges appris

- **Le disque de luciepc** : un TRIM (`fstrim`, ou `discard=async`) ou un gros effacement btrfs
  sature le NVMe ; les tests à délai rougissent (« pas indexé en 60 s »). Lire `/proc/pressure/io`
  (avg60 < 5) avant de juger un rouge de durée, et ne pas juger sous la pression.
- **Les services d'embarquement partagés** : deux suites qui chargent le même service en même temps
  (une passe et sa bissection, un test de charge orphelin) se ralentissent ; mesurer le temps par
  texte d'un service (un `curl` POST sur `/embed`) avant d'accuser un commit.
- **Un témoin dépendant d'un service** impose le sien par un faux déclaré au manifeste
  (`EmbedDaemon::new(..).servir_lie(listener)` sur un port libre), ou joue sous les deux régimes ;
  jamais il ne suppose l'environnement.
- **Mesurer en rouvrant un dépôt** relance l'index s'il reste un témoin `.premier-index` : lire le
  journal plutôt que rouvrir.
- **`systemctl --user stop`** d'une unité ne tue pas la portée `poste lourd` qu'elle a lancée :
  arrêter aussi la portée `run-p….scope`.
- **`git worktree remove`** refuse un worktree qui contient un sous-module : `--force` sur un
  worktree propre à soi.
- **`-D warnings`** : des parenthèses inutiles ou une fonction morte cassent le bâti ; compiler avant
  de pousser une branche.
- **ssh vers luciepc** passe par fish : `$(…)`, `$var=` et un motif `"x$"` y cassent ; utiliser
  `bash -s <<'EOF'`.
- **Deux sessions sur un même fichier** : convenir de qui écrit quoi (le suiveur d'archi et la
  reprise d'ED dans `worktrees.rs`) ; celui qui atterrit second pose l'appel commun.

## 8. Les tickets ouverts d'ED

La liste, une ligne chacun, est au § 7 du rapport de session (`01-rapport-de-session.md`). Ne pas
les déplacer avant le mot de c7.


## Relais contextuel et invariant de version (11 novembre, soir)

Huit graphes atelier invoquent une opération littérale avec contexte et arguments
JSON typés ; le résolveur exact de backend refuse les cibles ambiguës, inconnues,
soi-même, espaces/vues non disponibles et capacités non prouvées en lecture seule.
Le reçu canonique `metadata.invocation` survit hors du texte tronqué, jusqu'aux
liens UI et au rechargement. wait_output compose le lecteur de journal existant.

Une vérification sur A suivie de call_in, qui relisait prepared, pouvait exécuter B
après apply_reload : témoin déterministe rouge949, puis vert074. Maintenant la même
version préparée parcourt vérification, cellule/réactions, garde agent, harnais,
graphe et présentation. call/call_in ordinaires composent les mêmes méthodes
internes ; ne pas réintroduire une lecture prepared au milieu de ce parcours.
État, sources, tickets et preuves finales : en tête du rapport de session.


## Ouverture déclarative des dérivées (lot en branche, pas encore intégré)

Une dérivée hérite de l'identité de sa racine et n'a donc pas hashsafe ; une racine
explicite de manifeste conserve l'exigence hashsafe. DerivedConfig.validate_context
porte la validation inter-déclarations commune : résolveurs de EntityConfig et
RelationDef, racine existante non dérivée, relation gather du bon sens, voisin et
champs existants. Catalog vérifie encore ces règles à register_entity ; Prepared
les compose avant écriture avec un plan entités/relations (from/gather/endpoints).
Une erreur de plan refuse avant le premier appel DB ; jamais de suppression des
contraintes pour rendre le graphe acyclique. La correspondance avec les modes
d'identité de 1a reste à relire avant passe et intégration.


## Composition effective des déclarations et du schéma nommé

code::schema_configs(scope_chunking) expose les vrais EntityConfig et RelationDef
(propriétés comprises), partagés par plan_schema et register_code_schema. Backend
compose ces faits avec les déclarations, valide le fournisseur autoporté et le
plan complet avant connexion, puis monte racines explicites, fournisseur/overlay
index_signals et déclarations dépendantes. Pas de fausse configuration de Scope.
Identiques coalescés ; propriétés de relation disjointes compatibles unies ;
conflits de même champ ou d’extrémités, identités/configs d’entités divergentes
refusés par nom avant DB. Les espaces de noms entité/relation restent distincts.
L’association search_index de1a reste un raccord de lecture, jamais une dépendance
de création inverse ; sa preuve publique appartient au ticket transféré ouvert.
