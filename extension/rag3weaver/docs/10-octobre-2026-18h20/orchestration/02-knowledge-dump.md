# Orchestration — relevé de connaissances, 10 octobre 2026

Tout ce que la session d'orchestration sait de l'architecture, des scripts,
des postes et des projets, pour une session qui reprend sans contexte. Les
rapports des autres sessions (dans les dossiers datés de `extension/rag3weaver/docs/`
et `docs/`) sont plus précis sur chaque chantier ; ceci est la carte.

## 1. Le dépôt

- `L-Defraiteur/rag3db`, **public**, licence LRSL v1.2 (Luciform Research).
  Master en avance rapide, jamais de push en force ; commits en français
  sans ligne d'attribution ni mention d'IA ; identifiants en anglais ;
  adresse gmail de Lucie partout (jamais `sairen.io`).
- `/` : le fork de Kuzu v0.11.2.2 (C++) — `src/`, `test/` (dont le banc de
  concurrence, `known_red.txt`), `extension/vector` (HNSW, avec DELETE/UPDATE
  et le prédicat `VECTOR_SEARCH` en `WHERE`), `extension/geo` (R-tree,
  19 fonctions). `extension/fts` et `third_party/fuzzy-fst` **retirés** le 10
  (plein texte = lucivy, côté Rust). `BUILD.md`, `./build.sh`.
  Configuration de la lib commune : `cmake -S . -B build/lecteurs-csv -G Ninja
  -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSIONS=vector -DBUILD_SHELL=FALSE
  -DBUILD_TESTS=FALSE -DBUILD_EXTENSION_TESTS=OFF`, puis `cmake --build … -j 8`.
- `extension/rag3weaver/` : le crate Rust (dépend de `tools/rust_api` par
  `cxx`). `src/` (~90 fichiers), `src/dataflow/` (47 nœuds, runtime, réacteur,
  Mermaid, rapports, checkpoints), `ir/` (la crate `rag3weaver-ir` : `Value`,
  `QueryParam`, filtres, `Scope`, les formes `Hop` et `Count`), `codeparsers/`
  (sous-module, 13 langages, `vendor/tree-sitter-scss`), `bindings/nodejs/`
  (le paquet npm et `demo/demo.sh`), `templates/*.mmd`, `tests/` (87 e2e,
  tous `#[ignore]`), `run_e2e.sh`, `docs/` (dossiers datés par session),
  `visions/`.
- Binaires : `rag3weaver-backend` (stdio, lignes JSON, `describe`/`call`),
  `rag3weaver-chat`, `rag3daemon` (une base derrière une adresse : le seul
  processus qui l'ouvre), `rag3weaver-embeddings` (service d'embarquement
  HTTP : `/embed`, `/embed_dual`, `/embed_sparse`, `/rerank`, `/ocr`,
  `/sante`).
- Features : `rag3db-native`, `postgres`, `burn-embedder` (Vulkan ;
  `burn-rocm`), `candle-embedder`, `bge-m3`, `cuda`, `code` (codeparsers,
  landlock Linux seul), `daemon`, `openai-llm`, `ocr`, `burn-ocr`. Branche
  `anthropic-llm` : feature `anthropic-llm` (client Messages en HTTP brut).
- Variables `RAG3WEAVER_*` (~60) : `EMBED_SERVICE=127.0.0.1:7979,…` (adresses
  complètes), `EMBED_MODEL`, `GPU_DUTY=70`, `EMBED_CHAR_BUDGET=4096`,
  `TX_PAR_PAQUET`, `TX_PAQUETS_PAR_VALIDATION`, `BATCH_FILES`, `FTS=blobs|fichiers`,
  `ESTIMATE_COPY_JOURNALISE`, `SANS_CARTE_LOCALE=1`, `MOTEUR_ANCIEN=1`.
  Réglage moteur : `CALL force_checkpoint_on_copy=true|false` (défaut `false`
  depuis `ff9bad960` ; rag3weaver pose `true` en mode blobs),
  `CALL copy_journal_threshold`, `CALL current_setting('copy_journal_fallbacks')`,
  sonde `RAG3DB_PROFILE_JOURNAL=1`, `RAG3DB_PROFILE_CHECKPOINT`.
- Défauts du produit depuis `21a1d67c5` : plein texte en fichiers
  (`<base>.fts/`) pour une base neuve, transaction par paquet active, 2 048
  fichiers par paquet validés un par un, refus nommé de la transaction sur
  un dialecte sans `transactions`. Modèle par défaut granite-278m (768),
  granite-107m au premier gros index ou GPU faible.

## 2. Les docs qui font foi

- `docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md` — les quatre
  conditions et leur état (§2 tenu par le cœur C++).
- `docs/journal-des-chantiers.md` — ce qui est ouvert, non fusionné, en
  attente ; `docs/tickets/` (README : convention) — un défaut = un fichier.
- `extension/rag3weaver/docs/8-octobre-2026-16h29/orchestration/` — `01-plan-de-reprise.md`
  (chantiers A-H, §0 l'état du soir), `02-l-execution-asynchrone-des-graphes.md`,
  `03-le-paquet-npm.md` (§5 bis : démon / formulaire / magicien).
- `extension/rag3weaver/docs/3-octobre-2026-23h31/` — les rapports de chaque
  session (coeur-cpp : pages 03-07 dont 06 la fuite de pages, 07 les
  verrous ; banc-de-concurrence/04 la mise à jour de vecteurs ; embarquements/01-04
  le contrat du dialecte, le Cypher hors dialecte, les séries).
- `extension/rag3weaver/docs/10-octobre-2026-*/` — arbre-principal (la
  commande en fond), recherche (étape 1 async), claude-en-agent, coeur-cpp-tickets,
  mémoire (18h00 : exécution adressable, serveur MCP).
- `extension/rag3weaver/visions/` — `00-vision-generale.md` et les datées.
- `docs/optimiseur/3-octobre-2026-14h13/02-les-textes-reecrits.md` — les
  envois tracel-ai (B cubek#776 fusionnée ; A cubecl#1804 ouverte ; C, D, E
  en pause ; script `~/.cache/rag3weaver-build/envoi-tracel/envoyer.sh`).

## 3. Les postes

- **Ici** (`lucie-tablette`, Radeon 8060S, 32 cœurs, 121 Go, CachyOS, noyau
  7.2.9) : `/tmp` est de la RAM (61 Gio) — jamais de target ni de clone dans
  le scratchpad ; tout lourd sous `~/.cache/rag3weaver-build/poste lourd|mesure`
  (verrou + portée systemd à 40 Go, code 137 si débordement ; `mesure` =
  exclusif, pour les durées/mémoire/rappel ; `lourd` = partagé, nice 15 ; un
  lourd attend au plus `POSTE_PORTE_MAX`=600 s à la porte) ; `timeout` devant
  tout test ; jamais tuer par nom de binaire ni `pgrep -f`. Docker installé le
  10 mais `lucied` pas encore dans le groupe. Boutons « Jeu TV (luciepc) »
  dans la barre.
- **luciepc** (`lucied@luciepc` par Tailscale, règle SSH `accept` posée le 10,
  shell fish : envoyer du bash par `ssh … bash -s`) : 24 cœurs, 93 Go, deux
  Navi 48, Docker, les trois services d'embarquement locaux
  (7878 granite-278m, 7879 bge-m3 + reranker + OCR, 7880 granite-107m ; tunnels
  ici 7979/7980/7981 : `ssh -f -N -o ExitOnForwardFailure=yes -o
  ServerAliveInterval=30 -L 127.0.0.1:7979:127.0.0.1:7878 lucied@luciepc`,
  etc.). `~/git_workspaces/rag3db` (clone principal, MTG), `rag3db-service`
  (les services), `rag3db-lourd` (lib commune, détaché sur master, personne
  n'y bascule), un worktree par chantier (`rag3db-<chantier>`), `poste` copié
  dans `~/.cache/rag3weaver-build/`. Pour `cargo test` sans rebâtir le C++ :
  `RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR=$BUILD/src RAG3DB_INCLUDE_DIR=$BUILD/src
  LD_LIBRARY_PATH=$BUILD/src RAG3DB_ROOT=~/git_workspaces/rag3db-lourd
  CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-<chantier>` (c'est le
  couple LIBRARY_DIR/INCLUDE_DIR qui évite cmake). Les mesures restent ici.
- **CI GitHub** : `.github/workflows/paquet-npm.yml` (manuel : Linux par
  l'image manylinux_2_28, Windows MSVC pur en PowerShell, macOS arm64),
  `rag3weaver-workflow.yml`. Publication future par `release.yml` copié de
  lucivy (OIDC, porte `PUBLISH_ENABLED`, un relecteur requis à ajouter).

## 4. Les identifiants (dans `.vault/`, hors git ; ne pas lire)

`anthropic.env` (`ANTHROPIC_API_KEY`), `hf.env`, Vertex (`lr-hub-472010`),
`forks/*.bundle` (les branches tracel-ai). npm : session `npm login` du
compte `luciformresearch` dans `~/.npmrc` (un OTP par écriture). crates.io :
`cargo.env` attendu. gh : compte actif `luciedefraiteur-sairen`, le personnel
`L-Defraiteur` par `GH_TOKEN=$(gh auth token --user L-Defraiteur)`.

## 5. Les sessions et leurs règles

- Une session par chantier, chacune dans son worktree ; l'arbre principal est
  partagé (commit par chemins, pas de stash, index partagé) ; l'orchestration
  commite ses docs depuis un worktree détaché et avance l'arbre principal en
  `--ff-only` quand il est propre.
- Les noms (`rag3db-xx`) changent à chaque relance ; `/rename` ne survit pas.
- Un rebase qui apporte du code → rejouer avant de pousser ; la batterie
  complète une fois à la fusion (hors carte locale de jour, régime doux) ;
  un e2e sans `#[ignore]` ne tourne jamais ; une lib plus vieille que ses
  sources est refusée par `run_e2e.sh`.
- Estimations : « 1 jour » de session ≈ 1 h 30 ; compter en passes.
- Lucie décide : conception, rendu visible, ce qui sort du dépôt ; le reste
  est tranché par l'orchestration en le disant, réversible.

## 6. Les chiffres à garder

- Premier index de ce dépôt (~7 000 fichiers, 80 000 scopes, 1,1 M de
  relations) : 753 s en septembre → 78 s (fichiers 2 048 × 1), pic 9 Go,
  cherchable après 8-10 s ; COPY journalisé +1 % (80 contre 81 s).
- Journal : 25 octets par cellule (3 type, 1 null, 4 taille, 16 identité,
  1 drapeau) ; 466 Mio pour 62 Mo de texte (relations 168, texte en double
  124, coût par cellule 175) ; seuil du repli = tampon/8 plafonné 256 Mio ;
  une page de 4 Kio par réservation (~44 000 pour 180 Mio, 40 ns chacune).
- Groupe plein d'un COPY : 131 072 lignes (en deçà, rien n'est écrit avant
  le point de reprise).
- Élagage HNSW (k2) : rappel@10 = 1, 0 introuvable sur 12 278 vecteurs,
  temps par requête +3-5 %, fichier +2 %, degré 35 → 60.
- Paquet npm : binaire 79 Mo strippé, glibc 2.28 ; bâti 20 min sur le runner,
  3 min 25 à froid dans Docker sur luciepc, 39 s avec cache ; Windows 60 Mo
  (47 min), macOS arm64 65 Mo (15 min).
- Banc de concurrence : ~62 rouges connus / ~182 verts ; 57 des rouges sont
  les marches des verrous et du multi-écrivains.
