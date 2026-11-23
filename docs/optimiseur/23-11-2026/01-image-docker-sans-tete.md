# Une image Docker de rag3weaver, sans tête, devant un vLLM tiers

23 novembre 2026, au soir. Question de Lucie : « à quel point on saura packager pour un conteneur Docker ? »
Contexte : rag3weaver tourne sans écran chez un tiers, et ses modèles sont servis par le vLLM de ce tiers. Ce document
évalue ce qu'il faut mettre dans l'image et ce qui bloque aujourd'hui ; il ne change aucun code. Les faits viennent du
binaire installé à la tablette (commit ee1b4dfc7f, lib 3a2639714e) et du code de master.

**En bref : rien ne bloque sur le fond.** L'hôte sait déjà parler à un modèle et à un embarqueur compatibles OpenAI ;
il ne dépend que d'une seule lib à nous ; ses chemins se règlent par variables ; il écoute seul sans systemd. Ce
qui manque est de l'emballage, pas de l'architecture : **une première image qui tourne demande environ une journée
d'agent** (≈ 1 h 30 réelle d'après l'échelle habituelle), une image propre et petite deux à trois.

## 1. Le binaire de l'hôte et la lib rag3db

- **Fait.** `rag3weaver-backend` (259 Mo, sans section de débogage) ne demande, hors de la libc, que
  `librag3db.so` (27 Mo). Son RUNPATH pointe le magasin de libs : `~/.cache/rag3db-libs/rag3db-build-<id>…`. La lib
  elle-même ne demande que libstdc++, libgcc_s, libm et libc.
- **Dans l'image.** Copier la lib à côté du binaire et le lier avec `-Wl,-rpath,$ORIGIN/../lib` (ou poser
  `LD_LIBRARY_PATH`). C'est exactement le **rpath stable** du chantier des compilations : il évite aussi le bâti
  complet à chaque changement de lib. `rag3db-check-lib` (lancé avant l'hôte par l'unité systemd) reste utile comme
  contrôle au démarrage du conteneur.
- **Le vrai obstacle : la glibc.** Bâtis sur CachyOS, le binaire demande `GLIBC_2.44` et la lib `GLIBC_2.43` et
  `GLIBCXX_3.4.32` (libstdc++ de GCC 13). Debian 12 a la glibc 2.36, Debian 13 la 2.41 : **le binaire d'aujourd'hui ne
  tourne dans aucune image Debian**. Il faut bâtir la lib et l'hôte dans une image ancienne (manylinux_2_28, glibc
  2.28, comme les roues de lucivy, ou Debian 12), avec libstdc++ liée statiquement (`-static-libstdc++`) ou copiée
  dans `lib/`.
- **Effort.** Faible pour le lien (une option et un `COPY`) ; **moyen pour le bâti dans une image ancienne** : la lib
  C++ doit y compiler (version de CMake, de GCC, dépendances), ce qui n'a jamais été essayé. Le contournement immédiat, `LD_LIBRARY_PATH=/opt/rag3weaver/lib`,
  ne demande aucun rebâti.

## 2. codeparsers et lucivy

- **Fait.** Tous deux sont des crates Rust liées **statiquement** dans le binaire (lucivy 4.5.0 depuis crates.io,
  codeparsers par chemin, sous-module, grammaires tree-sitter comprises). Aucune `.so` à part.
- **Dans l'image.** Rien à ajouter. Le bâti a besoin du sous-module `codeparsers` récupéré (`git submodule update
  --init`).
- **Effort.** Nul.

## 3. L'embarquement

- **Fait.** Trois chemins existent déjà, choisis par la configuration de l'hôte :
  1. **`compatible`, protocole `openai`** : `POST <base>/embeddings`, avec `model`, `dimensions` déclarées et une clé
     lue dans une variable nommée (`api_key_env`). C'est le contrat de `vLLM --task embed`. **Rien à coder** pour un
     vLLM tiers.
  2. **le service d'embarquement à nous** (`RAG3WEAVER_EMBED_SERVICE`, `POST /embed`), aujourd'hui des services GPU
     à luciepc joints par tunnel ;
  3. **local** (feature `burn-embedder`, BGE-M3 par wgpu/Vulkan). Vulkan est ouvert par `dlopen` à l'usage seulement :
     sans GPU, l'hôte démarre quand même.
- **Dans l'image.** Le chemin 1, pointé sur leur vLLM. Un embarqueur CPU embarqué n'est **pas** recommandé : burn
  vise un GPU, le CPU serait lent sur un dépôt réel, et le tiers a déjà un serveur de modèles.
- **À savoir.** Le modèle d'embarquement fixe les dimensions de la base. Changer de modèle, c'est réindexer. Une base
  bâtie à la tablette avec bge-m3 ne se transporte pas vers un autre modèle.
- **Effort.** Une configuration à écrire et un essai à jouer : faible. Un essai bout en bout contre un vLLM réel reste
  à faire : `HttpEmbedder` n'est essayé aujourd'hui que contre un faux serveur.

## 4. Les chemins de données et de config

- **Fait.** Tout est réglable par variable : `RAG3WEAVER_DATA_DIR`, `RAG3WEAVER_CONFIG_DIR`, `RAG3WEAVER_STATE_DIR`,
  `RAG3WEAVER_CACHE_DIR`, `RAG3WEAVER_BUILTIN_TEMPLATES_DIR`, et les `XDG_*`. L'essai d'hôte de chaque pose le
  prouve déjà : il isole un hôte entier par ces seules variables.
- **Dans l'image.** Deux volumes : `/data` (les bases, chats, tâches, instantanés) et `/config`. Le cache des
  gabarits livrés peut rester dans le conteneur : ils sont dans le binaire et s'extraient au démarrage.
- **À prévoir.** Le volume grossit : 53 Gio à la tablette aujourd'hui, dont 22 de tâches et 16 d'instantanés.
  Chez un tiers, il faudra une rétention des tâches et des instantanés (aucune purge automatique n'est vérifiée ici).
  Le volume doit être un système de fichiers local : les bases rag3db tiennent un WAL et un verrou d'écrivain, pas
  de NFS.
- **Effort.** Faible pour monter ; moyen pour la rétention, si le tiers garde l'hôte longtemps.

## 5. Le lancement

- **Fait.** `rag3weaver-backend daemon [--address …] [--threads …]` lie lui-même son adresse
  (`RAG3WEAVER_DAEMON_ADDRESS`, défaut 127.0.0.1:7970). La socket systemd (`LISTEN_FDS`) n'est qu'une option.
- **Dans l'image.** `ENTRYPOINT ["rag3weaver-backend","daemon","--address","0.0.0.0:7970"]`, sous `tini` pour que
  l'arrêt soit propre.
- **À vérifier.** L'arrêt sur `SIGTERM` : les bases doivent se fermer avant la mort du processus, sinon le WAL peut
  casser au redémarrage. Le défaut est connu (« le WAL casse sur arrêt brutal »). Poser `stop_grace_period` large
  (60 s ou plus) et un essai d'arrêt dans le conteneur. L'arrêt par inactivité (`RAG3WEAVER_DAEMON_IDLE_MIN`) est à
  couper : sans socket systemd, personne ne le relancerait.
- **Exposition.** L'hôte écoute aujourd'hui sur 127.0.0.1 seulement ; ce qu'il garde quand il écoute sur le réseau
  n'est pas évalué ici (à vérifier avant tout). En attendant : le publier derrière un proxy avec authentification,
  ou seulement sur le réseau interne du tiers.
- **Effort.** Faible pour le lancement ; un essai d'arrêt propre à jouer.

## 6. git et les outils que l'agent lance

- **Fait.** L'hôte lance `git` partout (atterrissages, worktrees, diffs) et `sh` pour les commandes de l'agent.
  L'agent de code lance ce que le projet demande : cargo, node, npm, python…
- **Dans l'image.** Une base Debian slim (même glibc que le bâti, ou plus récente), avec `git`, `ca-certificates`,
  `tini`. Les outils du projet dépendent du client : une image de base « outils » par langage, ou une image dérivée
  que le tiers complète. C'est le point le plus ouvert : il se décide avec le tiers, pas dans le code.
- **Effort.** Faible pour git ; variable pour les chaînes d'outils (une image Rust + Node complète pèse 1 à 2 Gio).

## 7. Les identifiants

- **Fait.** Les clés sont lues par variable nommée (`api_key_env` pour l'embarqueur et pour les modèles
  compatibles), et Vertex par `GOOGLE_APPLICATION_CREDENTIALS` (un chemin). L'hôte lit aussi
  `~/.config/rag3weaver/rag3weaver-daemon.env` par l'unité systemd, ce qui disparaît en conteneur.
- **Dans l'image.** Rien. Les clés entrent par secrets Docker montés en fichier, ou par variables de l'orchestrateur ;
  un vLLM interne n'en demande souvent pas.
- **Effort.** Nul à faible.

## 8. Taille et temps de bâti

- **Taille estimée.** Debian slim (~80 Mo) + git (~40 Mo) + binaire (259 Mo) + lib (27 Mo) : **≈ 400 Mo**, sans les
  outils du projet. Le binaire se réduit en bâtissant sans `burn-embedder`/`burn-ocr` (inutiles avec un vLLM) et
  avec `strip` : à mesurer, probablement sous 150 Mo.
- **Temps de bâti.** La lib rag3db en C++ est le gros morceau (un bâti à froid se compte en dizaines de minutes) ;
  elle se bâtit une fois par version et se copie dans l'image depuis le magasin de libs, ou se bâtit dans une étape
  à part, mise en cache. L'hôte Rust : quelques minutes en incrémental, 20 à 30 min à froid.
- **Effort.** Un Dockerfile multi-étapes ; moyen pour un bâti reproductible en CI.

## Le plus court chemin vers une première image

1. **Bâtir l'hôte à luciepc** avec `--features rag3db-native,code,daemon,openai-llm` (sans burn), et
   `-C link-args=-Wl,-rpath,$ORIGIN/../lib`.
2. **Image Debian slim** : `git`, `ca-certificates`, `tini` ; `/opt/rag3weaver/bin/rag3weaver-backend`,
   `/opt/rag3weaver/lib/librag3db.so` (la lib épinglée, copiée du magasin).
3. **Configuration** : modèle `compatible`/`openai` et embarqueur `compatible`/`openai` vers l'adresse du vLLM,
   `dimensions` du modèle d'embarquement déclarées ; `RAG3WEAVER_DATA_DIR=/data`, `RAG3WEAVER_CONFIG_DIR=/config` ;
   `RAG3WEAVER_DAEMON_IDLE_MIN` coupé.
4. **Lancement** : `tini -- rag3weaver-backend daemon --address 0.0.0.0:7970`, `stop_grace_period: 120s`, volume
   `/data` local.
5. **Essai** : un docker compose avec un vLLM (ou un faux serveur compatible), un dépôt monté, une indexation et
   une recherche ; puis un `docker stop` suivi d'un redémarrage, pour prouver que les bases rouvrent.

Ce qui reste ouvert, à trancher avec le tiers : les chaînes d'outils de l'agent dans l'image, l'exposition réseau
de l'hôte, et la rétention des données.
