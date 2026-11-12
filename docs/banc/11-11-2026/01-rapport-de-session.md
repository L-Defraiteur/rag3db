# Le banc : rapport de session (11 novembre 2026)

> **Précision ultérieure de Lucie — 12 novembre 2026.** Le but est de garder
> le GPU seat1 disponible pour jouer : les poids, le cache et le calcul du banc
> doivent rester sur card2. Un contexte d’initialisation minuscule et stable
> sur card0 peut être toléré ; sa seule ouverture ne doit plus faire échouer
> le banc. Les notices exigeant zéro accès ci-dessous décrivent le contrat
> précédent. ROCm initialise aussi des ressources des GPU masqués ; le
> [ticket k10](../tickets/2026-11-11-mesurer-le-juge-local-avec-dix-fiches.md)
> donne les sources et les contrôles d’attribution restant à qualifier.
> Zéro **requête du banc** a été joué : le serveur avait effectué son warmup
> interne, dont l’exécution n’a pas été attribuée par compteur PCI.
> Aucun nouveau chargement ni retry sans nouvelle fenêtre coordonnée.

> **Consigne actuelle — 12 novembre 2026.** Card0/seat1 (PCI 07:00.0)
> est interdite aux sessions. Seule card2 (PCI 04:00.0) peut être utilisée,
> après coordination de charge et lecture de la VRAM, avec
> `ROCR_VISIBLE_DEVICES=0` explicite et identité PCI vérifiée. Les anciennes
> commandes card0 ci-dessous sont historiques et ne doivent pas être rejouées.
> Le passage du 12 novembre a été arrêté avant toute inférence : ROCm ouvrait
> aussi un contexte de 40 KiB sur card0 malgré le filtre. Aucun nouveau
> chargement avant exclusion stricte de seat1 et nouvelle fenêtre coordonnée.
> Suivi : [ticket k10](../tickets/2026-11-11-mesurer-le-juge-local-avec-dix-fiches.md).

Ce rapport s'adresse à un agent qui reprend le travail sans aucune mémoire de nos échanges. Le
**banc** est la session qui mesure : les témoins de concurrence du moteur rag3db (des tests gtest
en C++), les listes de rouges connus, les tickets de défauts, et depuis le 8 novembre les essais de
petits modèles locaux pour le préchargement de mémoire. Le dépôt est
`~/git_workspaces/rag3db`, un moteur de graphe en C++ dérivé de Kuzu, avec une extension
vectorielle HNSW et un produit en Rust, `extension/rag3weaver`.

Deux postes :
- **le portable**, où tourne cette session ;
- **luciepc**, joint par `ssh lucied@luciepc`. Son shell est fish, d'où le passage par
  `bash -c '…'`. Il porte deux cartes AMD Radeon AI PRO R9700 et les services d'embarquement.

Le knowledge dump du même dossier (`02-knowledge-dump.md`) dit comment tout se lance.

## 1. Le juge local de fiches de mémoire : fait

**La question** (Lucie, 8 novembre) : un petit modèle local peut-il dire, à la place de Gemini,
si un bout de mémoire est pertinent pour ce que fait l'agent ? Le but est le préchargement de
mémoire : avant chaque tour, le rappel propose des fiches, et un juge choisit celles à donner au
modèle. Les pages de résultats, sur master :

- `extension/rag3weaver/docs/08-novembre-2026-banc/01-modele-de-decision.md` : premier jeu, tiré
  des rappels réels. Il n'a aucun vrai positif : rien n'y est mesurable.
- `extension/rag3weaver/docs/08-novembre-2026-banc/02-le-jeu-a-vrais-positifs.md` : le jeu
  construit, les petits modèles, le relecteur et le dense.
- `extension/rag3weaver/docs/08-novembre-2026-banc/03-le-juge-local.md` : les modèles plus
  gros, les formes de contexte, le préfixe partagé, la liste, la recommandation et la suite.

**Le jeu** :
- 88 fiches « positives », c'est-à-dire utiles, construites depuis de vraies sessions et dont la
  provenance est vérifiée mot pour mot ;
- chacune face à 4 fiches « négatives » : 2 proches d'un autre travail, 2 au hasard ;
- soit 440 paires (contexte, fiche) ;
- le test porte sur 3 sessions, 170 paires.

**La mesure principale** : dans chaque cas de 5 fiches, la bonne est-elle classée première ? Le
hasard donne 0,20.

| candidat | bonne fiche première | temps |
|---|---|---|
| Gemini 3.8 Flash (référence, par le cloud Vertex) | 0,81 à 0,84 | 4,4 s par cas de 5 |
| **Qwen3-4B-Instruct-2507**, logits oui/non, contexte de fin de conversation de 2 000 jetons | **0,76** (0,74 sur le test) | 504 ms par paire, ~1,0 s par cas en préfixe partagé |
| Qwen3-4B, contexte de 6 000 jetons | 0,78 à 0,80 | 1,0 à 1,8 s par paire |
| Qwen3-4B, forme courte (dernière phrase de l'agent + dernier message) | 0,73 | 90 ms par paire |
| Qwen2.5-1.5B-Instruct, 2 000 jetons | 0,65 | 207 ms par paire, 424 ms par cas en préfixe partagé |
| Qwen3-1.7B, 2 000 jetons | 0,50 | 255 ms par paire |
| dense bge-m3 (cosinus, service en place) | 0,41 | 66 ms par cas |
| relecteur bge-reranker-v2-m3 (service en place) | 0,36 à 0,38 | ~60 ms par cas |
| Qwen2.5-0.5B, SmolLM2-360M, Laya (ModernBERT) | 0,02 à 0,23, le hasard | |
| liste A à E, ordre mélangé (Qwen3-4B) | 0,38 à 0,45 | 650 ms par cas |

**La recommandation** (page 03, § 4) :
1. Le rappel `recall_fiches` propose k = 5 fiches.
2. Qwen3-4B-Instruct-2507 les juge une à une, par les logits oui/non, avec le contexte partagé
   (préfixe en cache) et la fin de conversation bornée à 2 000 jetons.
3. On garde les 1 à 2 premières pour le modèle de l'agent.

Environ 1,2 s en tout, contre 4 à 23 s pour Gemini. Qwen3-4B s'emploie **en rangeur** (garder
les premières), **jamais en seuil** : l'échelle de ses scores varie avec le contexte.

## 2. Le même banc en GGUF par llama-server : fait le 11 novembre

Page : `docs/banc/11-11-2026/03-le-juge-en-gguf.md`. Le banc a tourné avec le bâti de llama.cpp de
Lucie à luciepc (`~/git_workspaces/llama.cpp`, b9590, commit d2462f8, `build-hip/bin/`), en
lecture et exécution seulement. Qwen3-4B a été converti en GGUF depuis les safetensors officiels,
en Q8_0 et Q4_K_M.

- **Q8_0 avec `cache_prompt`** : 0,76 de bonnes premières à 2 000 jetons, comme bf16, la même
  première fiche que bf16 dans 97 % des cas, 1,08 s par cas de 5.
- **Sans cache** : 3,3 s par cas.
- **Q4_K_M** : pareil à 2 000 jetons, mais 0,72 sur la forme courte.

Recommandation de production : Q8_0, avec `cache_prompt`.

Les binaires copiés du portable dans `~/.cache/rag3db-banc-notes/llama/bin/` à luciepc étaient
inutiles (ils plantaient par manque d'AVX-512) : effacés le 11 novembre, avant 17 h 14.

## 3. Les autres travaux de la session, tous sur master

- Les témoins de V2-2 (le point de reprise qui lit un instantané) : trois lecteurs verts, deux
  écrivains en known_red jusqu'au format 44. Commit 072c467d7.
- La fenêtre de mesure de V2-1c (25 octobre) : les lecteurs passent pendant un point de reprise.
  Page `extension/rag3weaver/docs/25-octobre-2026-banc/01-fenetre-du-25-octobre.md`.
- La mesure des lignes hors du graphe : environ 0,8 µs par ligne à la dimension 768. Ticket du
  18 octobre, commit 28b479b7e.
- Le ménage du 7 novembre : page `extension/rag3weaver/docs/07-novembre-2026-banc/01-menage-et-atterrissage.md`.
- `SameRowsFirstCopy` déplacé de known_red à probabilistic.txt (5d7d9513a).

## 4. Ce qui reste sur disque (rien n'a été effacé pour ce rapport)

**Sur le portable** :
- `~/.cache/rag3db-banc-notes/` (1,1 Go). Il contient :
  - `decision/` : scripts, jeux et résultats du juge local ;
  - `push-lot.sh` et les journaux des pushes.
- Le worktree `~/git_workspaces/rag3db-begin` (branche `banc-temoins-v22`, 434 Mio). Il est atterri
  sur master et peut partir.

**À luciepc**, `~/.cache/rag3db-banc-notes/` (18 Go) :

| chemin | taille | contenu |
|---|---|---|
| `decision-venv/` | 197 Mio | venv Python avec `transformers` 5.22.2, `laya` 0.4.29 et `huggingface_hub` ; torch 2.15 vient du système |
| `hf/qwen3-4b-instruct-2507` | 7,6 Go | poids, Apache 2.0 |
| `hf/qwen3-1.7b` | 3,8 Go | poids, Apache 2.0 |
| `hf/qwen2.5-1.5b-instruct` | 2,9 Go | poids, Apache 2.0 |
| `hf/qwen2.5-0.5b-instruct` | 954 Mio | poids, Apache 2.0 |
| `hf/laya-multilingual` | 647 Mio | poids, Apache 2.0 |
| `hf/smollm2-360m-instruct` | 694 Mio | poids, Apache 2.0, formats ONNX effacés |
| ~~`llama/bin/`~~ | — | **effacé le 11 novembre, avant 17 h 14** (binaires du portable, inertes faute d'AVX-512 ; la source reste sur le portable, paquets `llama-cpp` et `ggml`) |
| `hf/qwen3-4b-{bf16,Q8_0,Q4_K_M}.gguf` | 14,8 Go | poids GGUF du banc du 11 novembre |
| `h_petits.py`, `h_multi.py`, `gpu-*.sh`, `cpu-*.sh` | | scripts du juge local |
| `paires2*.jsonl`, `cas2*.jsonl`, `petits-*.jsonl`, `multi-*.jsonl` | | jeux et résultats |

Toujours à luciepc :
- `~/.cache/rag3weaver-build/banc-archives/` : les diffs des worktrees de mesure effacés le
  7 novembre.
- `~/.cache/rag3weaver-build/` porte encore des journaux `build-vingt.*.log` et des scripts du banc
  (`fenetre-jeu.sh`, `vingt-push.sh`…).

## 5. Les tickets ouverts du banc (dans `docs/banc/tickets/`, déplacés le 11 novembre)

- `docs/banc/tickets/2026-10-10-un-groupe-de-copies-bien-plus-grand-que-le-degre.md` : 1 402 copies d'un même
  vecteur ne sont pas toutes joignables dans l'index HNSW. Cinq cas en known_red, trois en
  probabilistic.
- `docs/banc/tickets/2026-10-11-l-elagage-peut-couper-le-dernier-pont-vers-une-region.md` : l'élagage peut rendre
  une région du graphe injoignable. Vu sur une variante d'essai seulement.
- `docs/banc/tickets/2026-10-12-sans-point-de-reprise-l-insertion-sous-hnsw-ralentit-lot-apres-lot.md` : la pente
  de l'insertion, en probabilistic.txt.
- `docs/banc/tickets/2026-10-12-les-retrecissements-de-l-insertion-hnsw-alourdissent-son-cout.md` : les
  rétrécissements de voisinage coûtent ×1,3 à ×1,6.
- `docs/banc/tickets/2026-10-18-un-bati-d-index-prend-tous-les-fils-et-affame-les-autres-connexions.md` : un
  CREATE_VECTOR_INDEX prend tous les fils de travail.
- `docs/banc/tickets/2026-10-18-sur-une-table-hnsw-les-ecrivains-passent-un-par-un.md` : le verrou exclusif de
  l'index sérialise les écrivains. La forme I2 est choisie.
- `docs/coeur-cpp/tickets/2026-10-18-un-point-de-reprise-fige-toutes-les-transactions-le-temps-de-s-ecrire.md` (rendu au cœur C++ le 11 novembre : travail de moteur, V2-2c et V2-3) : réduit
  par V2-1c et V2-2. Restent les écrivains, au format 44.
- `docs/banc/tickets/2026-10-19-la-memoire-d-un-index-avec-ses-vecteurs-a-mesurer.md` : avec G, la mesure de la
  mémoire en phase vecteurs.
- `docs/rag3weaver-archi/tickets/2026-11-08-le-juge-du-rappel-coupe-sa-reponse-sous-400-jetons.md` (chez archi, pour qui il est) : pour archi, la réflexion
  de Gemini prend les 400 jetons du juge des hooks.
- `docs/banc/tickets/2026-11-08-un-temoin-de-temps-du-banc-rougit-sous-charge.md` : un témoin de temps,
  `LockBench`, est passé en probabilistic.

## 6. Les trois prochains pas

1. Le banc GGUF est fait (page 03 de ce dossier). Reste à mesurer k = 10 et à le tenir à jour
   pour la session mémoire, qui branche le juge.
2. Agrandir le jeu du juge local au-delà de 88 positifs, avec les sessions de luciepc et
   LR_CodeRag, même graine. C'est le prérequis d'une spécialisation de Laya ou d'un petit Qwen.
3. Le décideur d'actions (page 03, § 7) : quand la porte de l'atelier aura journalisé quelques
   centaines d'approbations et de refus, mesurer Qwen3-4B en « cette action est-elle permise ? »
   par le taux de faux « permis ».

## 7. Reprendre

Il n'y a **aucun worktree ni clone de travail inachevé**. Le banc GGUF a utilisé le clone de
llama.cpp de Lucie, `~/git_workspaces/llama.cpp` à luciepc, sans le modifier. Voici les points de
reprise, avec leur chemin exact.

### 7.1 Le banc GGUF de Qwen3-4B, à luciepc : fait

- **Fait** : la conversion, les deux quantifications, les 440 paires sous les deux formes, avec et
  sans cache, la page 03 du dossier.
- **Rejouer** :

```bash
cd ~/.cache/rag3db-banc-notes && rm -f gguf-Q*.jsonl && setsid nohup ~/.cache/rag3weaver-build/poste lourd ./gguf-jeu.sh > gguf-jeu.log 2>&1 & disown
```

  Les poids GGUF sont dans `~/.cache/rag3db-banc-notes/hf/qwen3-4b-*.gguf`. Le serveur se lance
  sur card0 seule (`ROCR_VISIBLE_DEVICES=1`), sur le port 8099, puis il est arrêté à la fin.
- **Reste** :
  - mesurer k = 10 ;
  - rejouer le juge sur un jeu plus grand quand il existera ;
  - brancher dans le produit, ce qui revient à la session mémoire.

### 7.2 Le juge local, à refaire ou à étendre

- **Fait** : tout, pages 01 à 03 sur master.
- **Reprendre une passe** : `luciepc:~/.cache/rag3db-banc-notes/`, par exemple

```bash
cd ~/.cache/rag3db-banc-notes && setsid nohup ~/.cache/rag3weaver-build/poste lourd ./gpu-qwen3.sh > gpu-qwen3.log 2>&1 & disown
```

  Les scripts reprennent là où leur fichier de sortie s'arrête. Pour une passe neuve, effacer
  d'abord le fichier de sortie `petits-<modèle>-card0*.jsonl`.
- **Étendre le jeu** : sur le portable, `~/.cache/rag3db-banc-notes/decision/`.
  - `g_construire.py` tire ses prompts de `~/.claude/projects/-home-lucied-git-workspaces-rag3db*/*.jsonl`.
    Pour d'autres sessions, ajouter leurs transcriptions à ce motif, effacer `a-construire.json`
    (le tirage), garder `construites.json` (les fiches déjà construites), puis lancer
    `python3 -I g_construire.py` et `python3 -I i_paires.py`.

### 7.3 Le worktree du portable

- `~/git_workspaces/rag3db-begin`, branche `banc-temoins-v22`, dernier commit 888b8b7e8.
- Son contenu est atterri sur master (072c467d7). Rien à reprendre : il peut être effacé
  (`git worktree remove --force`, ou effacer le dossier puis `git worktree prune`, parce qu'il
  porte des sous-modules).


## Mise à jour du 12 novembre — k10 arrêté, service rétabli

Suivi unique : `docs/banc/tickets/2026-11-11-mesurer-le-juge-local-avec-dix-fiches.md`,
porteur banc. Worktree `/home/lucied/git_workspaces/cx-banc-suivi-k10`, branche
`cx-banc-suivi-k10`. Les preuves de l’optimiseur ont été intégrées à cette branche.

Lucie interdit désormais card0/seat1 ; seule card2 est utilisable, avec coordination.
Les 440 paires k5 et 880 paires k10 ont été préparées et contrôlées. Une fenêtre
exclusive a été acquise le 12 novembre à 17:54:59 Paris ; l’optimiseur a arrêté
7882 après remise explicite par hotfixs et libération des autres consommateurs.

Le serveur Q8_0 a chargé ses poids/cache sur card2, mais ses fdinfo ont révélé
également un contexte card0 de 40 KiB malgré `ROCR_VISIBLE_DEVICES=0`. La garde
a arrêté le serveur à 17:55:50, avant toute inférence. Zéro paire jouée,
aucun résultat k10 ni nouvelle latence, aucun retry. La carte du modèle et
l’ouverture de contexte seat1 sont deux faits distincts.

7882 a été restauré par l’optimiseur à 17:56:22.790 (PID 3273544, même binaire,
configuration et santé réelle granite-278m/768/Flex32, card2). Le banc a revérifié
santé/PID/fdinfo, absence de son serveur et port 8099 libre. Le verrou mesure
a été rendu à 17:57:00, poste relu libre. Les trois autres services sont restés
inchangés selon les contrôles opérateur.

Reprise : lire le ticket et ses preuves `~/.cache/rag3db-banc-notes/k10-20261112/`
aux deux postes ; ne pas rejouer le lanceur existant. Établir statiquement une
frontière d’accès qui exclut les périphériques seat1, notamment leur exposition
par KFD, avant tout nouveau chargement autorisé. Le ticket porte cette suite
et reste ouvert. Les anciennes commandes du §7.1 ciblant card0 sont historiques
et ne doivent plus être exécutées. Aucune nouvelle sonde GPU autorisée à ce stade.
