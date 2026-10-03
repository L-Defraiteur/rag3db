# Embarquements — ce que la session sait

Mis à jour le 3 octobre 2026, vers 23 h 50. À lire avec le rapport de session
à côté ; les procédures détaillées des services sont dans
`docs/3-octobre-2026-14h26/02-le-service-d-embarquement-sur-l-autre-poste.md`,
les mesures dans `…/03-indexer-ce-depot.md`.

## 1. Les services de l'autre poste (luciepc)

`ssh lucied@luciepc` ; son shell est fish : passer par `ssh … bash -s < script`.
Rien n'y est commité ni poussé. Le code vient du worktree
`~/git_workspaces/rag3db-service` (détaché sur `origin/master`), bâti par
`cargo build --release --bin rag3weaver-embeddings --features daemon,burn-embedder,burn-ocr -j8`
(trois minutes). Les poids sont sous `~/.cache/rag3weaver/` là-bas.

| Service | Écoute là-bas | Tunnel ici | Lancé par |
|---|---|---|---|
| granite-278m | 127.0.0.1:7878 | 127.0.0.1:7979 | `rag3weaver-embeddings` |
| bge-m3 (dense, creux, dual) + relecteur `bge-reranker-v2-m3` + OCR `ppocrv6-tiny` | 127.0.0.1:7879 | 127.0.0.1:7980 | `rag3weaver-embeddings`, avec `RAG3WEAVER_RERANK_MODEL` et `RAG3WEAVER_OCR_MODEL` |
| granite-107m | 127.0.0.1:7880 | 127.0.0.1:7981 | `rag3weaver-embeddings` |
| JevK5-4B Q8_0 (décision) | 127.0.0.1:7881 | 127.0.0.1:7982 | `llama-server` déjà présent (`~/git_workspaces/llama.cpp/build/bin/`, Vulkan, juin 2026) |
| Qwen2.5-7B-Instruct Q4_K_M (petit LLM à outils) — **arrêté**, relancé à la demande | 127.0.0.1:7882 | 127.0.0.1:7983 | le même `llama-server`, `--jinja` |

- **La carte** : la R9700 à `0000:04:00.0` (`card2`, `gpu:0` pour wgpu,
  `Vulkan0` pour llama.cpp). L'autre R9700 (`0000:07:00.0`, `card0`) est
  l'écran de `seat1` : on n'y touche pas. Se vérifie à chaque lancement par
  `mem_info_vram_used` des deux cartes. Elle porte environ 22 Gio sur 32 avec
  tout sauf le petit LLM (6 Gio de plus).
- **L'adresse d'écoute du démon est `--adresse`**, pas une variable.
- **Arrêter** : `kill -TERM` du pid trouvé par `ss -ltnp | grep <port>` ;
  `pidof` rend plusieurs processus.
- **Rien ne se relance seul** : ni les démons après un redémarrage de luciepc,
  ni les tunnels après un redémarrage de l'un des deux postes.
- **Côté client** : `export RAG3WEAVER_SERVICE_EMBED=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981`
  (l'ancien nom `RAG3WEAVER_EMBED_SERVICE` est lu aussi). Le creux, le
  relecteur et l'OCR lisent ces mêmes adresses quand ils n'ont pas de variable
  à eux. La décision : `RAG3WEAVER_SERVICE_DECIDE=http://127.0.0.1:7982`.

## 2. Architecture : où est quoi

| Fichier | Ce qu'il porte |
|---|---|
| `src/burst.rs` | Le régulateur de rafale : `BurstSettings` (durée visée, pause), `BurstPacer` (pur : règle de trois), `BurstGate` (l'horloge), `plan` / `plan_remote`, `active()` (faut-il ménager l'écran). |
| `src/regime.rs` | `Regime` (confort, plein), la carte du poste (`card_class`, `sole_card_drives_display`), ce que le pilote dit (wgpu : carte intégrée). |
| `src/embedding_choice.rs` | L'heuristique du premier index : 278m par défaut, 107m si plus de 50 000 fichiers, carte faible ou absente, ou seule carte portant l'affichage ; `CardClass::Service` les fait taire. |
| `src/daemon/embeddings.rs` | Le démon (`EmbedDaemon` : `/embed`, `/embed_dual`, `/embed_sparse`, `/rerank`, `/ocr`) et ses clients (`DaemonEmbedder`, `DaemonReranker`, `DaemonOcr`). |
| `src/model_source.rs` | **Un modèle, en service ou en local** : `Capability`, `ModelSource`, `resolve` (choix par le modèle servi, refus, repli déclaré, `Origin`), et `connect_embedder` / `connect_sparse` / `connect_reranker` / `connect_ocr`. |
| `src/decider.rs` | La décision : trait `Decider`, `option_probabilities` (pur), `LlamaServerDecider`, `connect_decider`, clé de service `decider`. |
| `src/estimate.rs` | `estimate` : `survey` (comptes par une politique), `Rate`, `probe_rate`, `Estimate`, `estimate_here`. |
| `src/catalog/progress.rs` | `index_progress(_for)`, `IndexState` et `index_state(_for)`, `note_indexing_started`, `refresh_index_states`, le débit noté en méta. |
| `src/dataflow/index_nodes.rs` | `EstimateNode`, `IndexNode`, `spawn_index` / `run_index` (le journal), `confirmation_refusal`. |
| `src/ingest_profile.rs` | Les chronomètres cumulés de l'ingestion, publiés en `[ingest-total]`. |
| `templates/tools/estimate.mmd`, `index.mmd` | Les fiches d'outils. |

Au manifeste d'un backend : `models.<capacité>` (`embed`, `sparse`, `rerank`,
`ocr`, `decide`), ou l'ancienne section `embeddings` pour le dense — l'une ou
l'autre. Un signal `sparse` déclaré sans `models.sparse` refuse au démarrage.

## 3. Lancer les tests

Depuis le worktree, avec la bibliothèque C++ de l'arbre principal :

```bash
B=/home/lucied/git_workspaces/rag3db/build/lecteurs-csv
export RAG3DB_ROOT=/home/lucied/git_workspaces/rag3db RAG3DB_SHARED=1 \
  RAG3DB_LIBRARY_DIR=$B/src RAG3DB_INCLUDE_DIR=$B/src LD_LIBRARY_PATH=$B/src \
  CARGO_INCREMENTAL=0 LUCIVY_SCHEDULER_THREADS=8
cargo test --features rag3db-native,burn-embedder,burn-ocr,code,daemon --lib -j6
```

Les e2e, par le service distant et avec un port local à soi :

```bash
RAG3DB_BUILD=$B RAG3WEAVER_SERVICE_EMBED=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981 \
  RAG3WEAVER_EMBEDDINGS_ADDR=127.0.0.1:7890 CARGO_BUILD_JOBS=6 RAG3WEAVER_CHARGE=0 \
  ./run_e2e.sh --summary --test e2e_estimate
```

- `cargo check --lib` **sans feature** doit passer aussi (un module qui dépend
  du code se garde par `cfg(feature = "code")`).
- Les scripts Python du backend demandent l'environnement
  `experiments/mtga/.venv` de l'arbre principal (module `mcp`), le binaire
  `target/debug/rag3weaver-backend` du worktree, et deux liens dans le
  worktree vers `build/lecteurs-csv` et `extension/vector/build` de l'arbre
  principal. `test_migration_v8.py` prend le binaire v7 de
  `/var/tmp/migration-v8/backend-v7`.
- **La passe du dépôt entier** : `e2e_estimate`, test
  `ce_depot_est_cherchable_par_mots_avant_ses_vecteurs`, avec
  `RAG3WEAVER_ESTIMATE_REPO=1` (mots), `RAG3WEAVER_ESTIMATE_VECTORS=1` (puis
  les vecteurs), `RAG3WEAVER_INGEST_PROFILE=1` (la ventilation). Six à dix
  minutes ; la lancer détachée et lire son fichier de sortie.
- Les suites qui restent sur la carte d'ici, variable posée ou non :
  `e2e_burn_*`, `e2e_demon_embeddings`, `e2e_mesure_ingestion_code`,
  `e2e_banc_bge_m3` (`tests/common`, `SUITES_LOCALES`).

## 4. Mesures

**Le service** : parité des vecteurs entre l'iGPU d'ici et la R9700, granite-278m
en Flex32 — écart absolu maximal 4,1e-5. La composition d'un lot déplace un
vecteur d'autant (4,4e-5 entre un texte seul et le même dans un lot) : ce
n'est pas un bug. Débit vu d'ici par le tunnel : 148 000 caractères/s en 278m.
Décision JevK5 : 109 à 117 ms sur un prompt court, 230 à 525 ms de 170 à 900
jetons. Relecture par le démon : 189 ms pour trois passages.

**Ce dépôt** (environ 6 840 fichiers retenus, 61 Mo, 76 680 scopes, 438 000
relations, 127 000 morceaux ; binaire de test, 8 fils lucivy) — temps pour
être cherchable par mots :

| Passe | Temps |
|---|---|
| Relations paquet par paquet | 1 798 s |
| Chargement en masse des relations | 523 s |
| Deux vidages aberrants fermés | 499 s |
| Requêtes par lot par jointure | 352 s |

Ventilation de la passe de 352 s : blobs de l'index plein texte poussés en
base 119 s (404 appels) ; symboles 56 s ; analyse 55 s dont 46 dans
l'analyseur ; nœud plein texte 29 s ; points de reprise ~30 s ; vidages de la
file 27 s ; insertions 29 s ; chargement final 12 s ; inchangés 11 s ; marques
20 s. Vecteurs ensuite : 692 s mesurés pour 651 prévus par la sonde (6 %).
`src/` de la crate (125 fichiers) : 11 à 19 s.

**Le binaire optimisé ne gagne rien** : le profil de test optimise déjà les
dépendances (`[profile.dev.package."*"] opt-level = 3`).

## 5. Défauts connus et limites

- **Les défauts provisoires** : rafale 50 ms / pause 150 ms ; seuil de
  confirmation cinq minutes. Aucun n'a été choisi par Lucie.
- **La recherche attend pendant le premier temps de l'indexation** :
  `spawn_index` tient le verrou du catalogue pendant tout `sync_source`.
- **L'état « prêt » veut dire « prêt à la dernière indexation »** : une
  écriture hors d'`index` ne le met pas à jour (un « jamais », lui, est
  recompté).
- **Les marques `relations_pending` sont par source** ; l'état d'une entité
  sans rapport avec le code les voit aussi.
- **`gpu_busy_percent` est propre aux pilotes AMD** : la détection « seule
  carte portant l'affichage » ne voit ni Intel ni NVIDIA.
- **Les graphes de recherche des gabarits de backend n'ont pas de branche
  `sparse`** (`search_structured.mmd`) ; `search_base.mmd` l'a.
- **`estimate` est branché pour le code** (politique `code::verdict`) ; la
  fonction est générique (`survey` prend une politique), le nœud ne l'est pas
  encore.
- **Le démon ne se cadence pas pour `/rerank` et `/ocr`** : ils prennent le
  verrou de passe et le rapport cyclique, pas le régulateur de rafale.
- **Un 7 milliards en 4 bits décroche en multi-tour sur un vrai outillage**
  (onze outils, fiches longues) alors qu'il tient avec deux outils : vérifié
  des deux côtés, c'est le modèle, pas le protocole.

## 6. Ce qui a été essayé sans succès, ou trouvé faux

- **« Cherchable par mots en une minute »** : un calcul, faux d'un facteur
  trente à la première mesure. Ne plus annoncer un temps qu'on n'a pas mesuré.
- **« Les 140 s hors nœuds sont les points de reprise »** : faux, c'étaient
  les blobs d'index. La somme des lignes de profil comptait aussi l'appel
  Symbol deux fois, et les lignes du runtime ne s'impriment qu'au-delà de
  20 ms.
- **Paralléliser l'analyse chez nous** : inutile, l'analyseur est déjà
  parallèle par fichier (46 des 55 s sont chez lui).
- **Laisser les tests locaux sur le port 7878** quand deux arbres jouent des
  e2e : chacun remplace le démon « périmé » de l'autre. Un port par arbre.
- **Un `Bge::Distant` dans `tests/common` qui ne relaie pas `distant()`** : le
  client cadençait ses rafales pour une carte qui n'était pas la sienne
  (1 019 s au lieu de 123 s).
- **Vider l'adresse du manifeste pour laisser la variable choisir** dans
  `test_migration_v8.py` : le binaire v7 ne connaît pas la variable. Le script
  écrit l'adresse lui-même.
- **Un ssh ou un push refusé par le garde de permissions** ne se contourne
  pas : c'est à Lucie de l'autoriser dans la fenêtre de la session.
