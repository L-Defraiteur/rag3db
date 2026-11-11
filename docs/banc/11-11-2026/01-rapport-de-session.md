# Le banc : rapport de session (11 novembre 2026)

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

## 2. Le même banc en GGUF par llama-server : bloqué, en attente d'un choix

Demande du 9 novembre (orchestration, pour la page de la session 96 sur le classeur local,
`docs/09-novembre-2026-memoire-longue/03-le-classeur-local-dans-le-rappel.md`) : rejouer les
440 paires avec Qwen3-4B dans la forme de production. La forme de production, c'est llama-server
avec un fichier GGUF (format de poids de llama.cpp), en quantification Q8_0 puis Q4_K_M, avec le
cache de préfixe `cache_prompt`. Les chiffres du § 1 étaient en torch bf16.

**L'état** :
- llama-server n'existe pas à luciepc.
- Le portable l'a par les paquets CachyOS `llama-cpp` 0.6.0 (build 11429, commit d812350493),
  `ggml` 0.26.0, `ggml-hip` et `ggml-vulkan`.
- Les binaires et bibliothèques du portable ont été copiés dans
  `~/.cache/rag3db-banc-notes/llama/bin/` à luciepc (1,4 Go). Ils y sont **inertes** : ils
  plantent par « Instruction non permise ». Le portable installe les paquets compilés en
  x86-64-v4 (AVX-512), que le processeur de luciepc n'a pas (avx2 seulement).
- Qwen ne publie pas de GGUF officiel de Qwen3-4B-Instruct-2507. L'orchestration a choisi de
  convertir nous-mêmes depuis les safetensors officiels déjà à luciepc, avec
  `convert_hf_to_gguf.py` de llama.cpp et le paquet pip `gguf`, en versions épinglées.

**Bloqué sur** : le choix entre deux voies, posé à l'orchestration le 9 novembre et sans réponse
à ce jour.
- (1) Bâtir llama.cpp à luciepc depuis les sources, au commit d812350493, avec le moteur HIP pour
  gfx1201, sous `poste lourd`, `-j 8`. Le même clone donne le script de conversion et
  `llama-quantize`. C'est la voie proposée.
- (2) Que Lucie installe les paquets à luciepc, ce qui demande sudo.

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
| `llama/bin/` | 1,4 Go | binaires du portable, inertes |
| `h_petits.py`, `h_multi.py`, `gpu-*.sh`, `cpu-*.sh` | | scripts du juge local |
| `paires2*.jsonl`, `cas2*.jsonl`, `petits-*.jsonl`, `multi-*.jsonl` | | jeux et résultats |

Toujours à luciepc :
- `~/.cache/rag3weaver-build/banc-archives/` : les diffs des worktrees de mesure effacés le
  7 novembre.
- `~/.cache/rag3weaver-build/` porte encore des journaux `build-vingt.*.log` et des scripts du banc
  (`fenetre-jeu.sh`, `vingt-push.sh`…).

## 5. Les tickets ouverts du banc (dans `docs/tickets/`, non déplacés)

- `2026-10-10-un-groupe-de-copies-bien-plus-grand-que-le-degre.md` : 1 402 copies d'un même
  vecteur ne sont pas toutes joignables dans l'index HNSW. Cinq cas en known_red, trois en
  probabilistic.
- `2026-10-11-l-elagage-peut-couper-le-dernier-pont-vers-une-region.md` : l'élagage peut rendre
  une région du graphe injoignable. Vu sur une variante d'essai seulement.
- `2026-10-12-sans-point-de-reprise-l-insertion-sous-hnsw-ralentit-lot-apres-lot.md` : la pente
  de l'insertion, en probabilistic.txt.
- `2026-10-12-les-retrecissements-de-l-insertion-hnsw-alourdissent-son-cout.md` : les
  rétrécissements de voisinage coûtent ×1,3 à ×1,6.
- `2026-10-18-un-bati-d-index-prend-tous-les-fils-et-affame-les-autres-connexions.md` : un
  CREATE_VECTOR_INDEX prend tous les fils de travail.
- `2026-10-18-sur-une-table-hnsw-les-ecrivains-passent-un-par-un.md` : le verrou exclusif de
  l'index sérialise les écrivains. La forme I2 est choisie.
- `2026-10-18-un-point-de-reprise-fige-toutes-les-transactions-le-temps-de-s-ecrire.md` : réduit
  par V2-1c et V2-2. Restent les écrivains, au format 44.
- `2026-10-19-la-memoire-d-un-index-avec-ses-vecteurs-a-mesurer.md` : avec G, la mesure de la
  mémoire en phase vecteurs.
- `2026-11-08-le-juge-du-rappel-coupe-sa-reponse-sous-400-jetons.md` : pour archi, la réflexion
  de Gemini prend les 400 jetons du juge des hooks.
- `2026-11-08-un-temoin-de-temps-du-banc-rougit-sous-charge.md` : un témoin de temps,
  `LockBench`, est passé en probabilistic.

## 6. Les trois prochains pas

1. Obtenir le choix de la voie llama.cpp (bâtir à luciepc au commit d812350493, ou paquets par
   Lucie). Puis convertir Qwen3-4B en GGUF Q8_0 et Q4_K_M depuis les safetensors officiels, et
   rejouer les 440 paires par llama-server sur card0, avec et sans `cache_prompt`. Les écarts
   avec torch bf16 vont dans une page du banc et à la session 96.
2. Agrandir le jeu du juge local au-delà de 88 positifs, avec les sessions de luciepc et
   LR_CodeRag, même graine. C'est le prérequis d'une spécialisation de Laya ou d'un petit Qwen.
3. Le décideur d'actions (page 03, § 7) : quand la porte de l'atelier aura journalisé quelques
   centaines d'approbations et de refus, mesurer Qwen3-4B en « cette action est-elle permise ? »
   par le taux de faux « permis ».
