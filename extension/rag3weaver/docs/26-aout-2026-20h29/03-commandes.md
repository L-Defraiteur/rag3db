# 03 — Toutes les commandes, et les pièges qui vont avec

Depuis `extension/rag3weaver/` sauf mention contraire.

## 1. Les tests unitaires — rapides, sans base

```sh
cargo test --lib                                  # 740, sans feature
cargo test --features code,openai-llm --lib       # 838
cargo test --features rag3db-native,code,openai-llm --lib -- events::   # un module
cargo test --features code --lib -- render_nodes  # un sujet
```

## 2. Les tests E2E — **toujours par `run_e2e.sh`**

```sh
./run_e2e.sh                        # tout : 33 suites, 257 tests
./run_e2e.sh --summary              # + le tableau par suite
./run_e2e.sh --test e2e_code        # une suite
./run_e2e.sh --test e2e_code ingest_our_own       # un test
./run_e2e.sh --features openai-llm --test e2e_cloud_code_agent   # ajouter une feature
./run_e2e.sh --build                # forcer la reconstruction de rag3db + extensions
./run_e2e.sh --build-only           # construire sans tester
```

> **Le piège n° 1.** Un `cargo test --test …` lancé **à la main** recompile
> rag3db **en statique**, et l'extension vectorielle ne résout plus ses
> symboles : `undefined symbol: rag3db::catalog::IndexAuxInfo`. `run_e2e.sh`
> exporte `RAG3DB_SHARED=1`, `RAG3DB_LIBRARY_DIR`, `RAG3DB_INCLUDE_DIR` et
> `LD_LIBRARY_PATH`. **Je suis tombé dedans deux fois aujourd'hui**, dont
> une en compilant en release.

Pour lancer un binaire déjà compilé sans passer par cargo (utile pour
mesurer sans compilation) :

```sh
W=$PWD
RAG3DB_SHARED=1 RAG3DB_ROOT=$W/../.. \
  LD_LIBRARY_PATH=$W/../../build/native-test/src \
  ./target/debug/deps/e2e_burn_embedder-<hash> --ignored --nocapture
```

## 3. Les variables d'environnement

| Variable | Ce qu'elle fait |
|---|---|
| `RAG3WEAVER_INGEST_PROFILE=1` | la durée **de chaque nœud** du pipeline d'ingestion |
| `RAG3WEAVER_CLOUD_QUESTIONS=3` | ne poser que ces questions au modèle (`1,3` accepté) |
| `RAG3WEAVER_LOCAL_LLM=http://127.0.0.1:8080/v1` | viser un endpoint local **au lieu** de Vertex |
| `RAG3WEAVER_LOCAL_MODEL=qwen3-coder-30b` | son nom dans les traces |
| `RAG3WEAVER_SSE_DUMP=/tmp/sse.log` | capturer le flux brut d'un fournisseur |
| `RAG3DB_PROBE_HNSW=1` | les sondes HNSW à 4 096 lignes |
| `RAG3DB_MAX_DB_SIZE` | surcharger le 1 Tio réservé en mémoire |
| `GOOGLE_APPLICATION_CREDENTIALS` | **`.vault/vertex-sa.json`** à la racine du dépôt |
| `GOOGLE_CLOUD_PROJECT` | le `project_id` du même fichier |

## 4. Un agent contre un modèle réel

**Le nuage** (quelques centimes par question) :

```sh
export GOOGLE_APPLICATION_CREDENTIALS="$PWD/../../.vault/vertex-sa.json"
export GOOGLE_CLOUD_PROJECT="$(python3 -c "import json;print(json.load(open('$GOOGLE_APPLICATION_CREDENTIALS'))['project_id'])")"
./run_e2e.sh --features openai-llm --test e2e_cloud_code_agent
```

**En local, gratuit** — aucun adaptateur, le client OpenAI *est* le client
llama.cpp :

```sh
~/git_workspaces/llama.cpp/build/bin/llama-server \
  -m ~/ML/models/Qwen3-Coder-30B-abliterated-Q6_K/*.gguf \
  --device Vulkan1 -ngl 99 --host 127.0.0.1 --port 8080 --jinja \
  -c 131072 --flash-attn on --cache-type-k q8_0 --cache-type-v q8_0 --cache-ram 2048

RAG3WEAVER_LOCAL_LLM=http://127.0.0.1:8080/v1 RAG3WEAVER_LOCAL_MODEL=qwen3-coder-30b \
  ./run_e2e.sh --features openai-llm --test e2e_cloud_code_agent
```

- `--jinja` **n'est pas facultatif** : sans lui, pas de gabarit de
  discussion, donc pas d'appels d'outils convertis.
- `--device Vulkan1` : la carte qui **ne** pilote **pas** l'écran (vérifier
  avec `llama-server --list-devices` et `rocm-smi --showmeminfo vram`).

### Ce que ça coûte en VRAM, mesuré le 27 août

Le modèle est **Qwen3-Coder-30B-A3B** en `Q6_K` : 25,09 Go de poids, et
*trente milliards de paramètres dont trois actifs par jeton*. C'est ça qui le
rend rapide — un modèle dense de 30 Go ferait passer tous ses poids par la
bande passante à chaque jeton ; celui-ci n'en touche qu'un dixième. En
revanche tous les experts doivent être résidents : on paye la **mémoire** de
30 milliards et la **vitesse** de trois.

| contexte | cache KV | VRAM | libre sur 34,21 Go |
|---|---|---|---|
| 32 768 | f16 | 28,30 Go | 5,91 |
| 65 536 | f16 | 31,55 Go | 2,66 |
| **131 072** | **q8_0** | **32,02 Go** | **2,19** |

**Quadrupler le contexte coûte 3,7 Go**, parce que quantifier le cache le
divise par deux et compense le doublement. `q8_0` perd un peu de précision —
c'est un compromis, pas un repas gratuit.

Deux choses que le log dit et qu'on aurait cherchées longtemps :

- `n_ctx_train = 262144` — le modèle est entraîné pour **256k**. Rester à
  32k n'utilisait qu'un huitième de sa capacité. 256k demanderait encore
  ~6 Go et ne rentre pas avec les poids en Q6.
- `prompt cache is enabled, size limit: 8192 MiB` — un cache de prompts de
  **8 Go en RAM hôte**, pas en VRAM. C'est une bonne part de ce que le serveur
  laissait partir en zram (voir « Ménage de la machine » plus bas).
  `--cache-ram 2048` le borne, `--cache-ram 0` le supprime.
- `n_parallel = 4, kv_unified = true` : les quatre slots **partagent** un
  seul cache. Sans ça, 128k en demanderait quatre fois plus.

## 5. Mesurer

```sh
# Ingestion, par phase puis par nœud
./run_e2e.sh --test e2e_code reingest_is_idempotent          # entities_ms / relations_ms / symbols_ms
RAG3WEAVER_INGEST_PROFILE=1 ./run_e2e.sh --test e2e_code ingest_our_own

# Index vectoriel : incrémental contre construction en masse
./run_e2e.sh --test e2e_code building_the_vector

# Rendu compact contre JSON brut
./run_e2e.sh --test e2e_code read_and_grep_as_graph_tools
```

Quelle carte travaille, et à quel prix :

```sh
rocm-smi --showuse --showmeminfo vram
ls -l /proc/<pid>/fd | grep renderD          # quel nœud de rendu le processus tient
grep -o "libvulkan_radeon\|lvp_icd" /proc/<pid>/maps   # vrai pilote, ou Vulkan logiciel
uptime                                        # la charge — c'est la compilation qui la fait monter
```

> **La compilation est ce qui fige la machine**, pas l'inférence : charge
> 11,6 avec `-j 8` pour **un** binaire de test (binaires de ~900 Mo en
> debug), contre 1,7 cœur pour quatre tests BGE-M3 sur GPU. Pour travailler
> pendant : `cargo build -j 8`.

## 6. Le fork et ses extensions

```sh
cd ../..                                      # racine rag3db
cmake --build build/native-test --target rag3db_vector_extension -j"$(nproc)"
rm extension/vector/build/libvector.rag3db_extension   # avant reconstruction si ABI douteuse
```

## 7. Ce qui demande des poids ou des identifiants

| Suite | Condition | Coût |
|---|---|---|
| `e2e_burn_*` | poids dans `~/.cache/rag3weaver/` (téléchargés au premier passage) | GPU, quelques secondes |
| `e2e_cloud_code_agent`, `e2e_cloud_schema_probe` | `.vault/vertex-sa.json` + `GOOGLE_CLOUD_PROJECT`, sinon **sautées** | centimes |
| `e2e_hnsw_scale` | rien ; `RAG3DB_PROBE_HNSW=1` ajoute les sondes 4 096 | ~1 min |

## 8. Les neuf pièges, par probabilité

1. **`cargo test --test` à la main** → lien statique, extension inchargeable. Passer par `run_e2e.sh`.
2. **Feature absente** → 0 test exécuté, et la suite paraît verte.
3. **Compteurs en dur** dans les tests (`BUILTIN_NODE_COUNT`, `RELATIONS.len()`) — les mettre à jour en ajoutant un nœud ou une relation.
4. **`Catalog::get` rend `{"n": Map}`** — déballer.
5. **`MockEmbedder` rend des vecteurs nuls** ; `HashEmbedder` non — pour un test de recherche vectorielle, prendre le second.
6. **`ureq` lève sur les 4xx** : sans `http_status_as_error(false)`, le corps de l'erreur est perdu.
7. **Extension `.so` périmée** après un rebuild du cœur → `rm` puis reconstruire la cible.
8. **Ne pas commiter `.gitmodules`** (modifié localement), ne pas toucher l'arbre de **lucivy** (dépendance par chemin, volontaire — actuellement 3.0.5). Pas de trailer d'attribution IA dans les commits.
9. **Le chemin d'un fichier est relatif à la `FileSource`**, pas au dépôt. Depuis aujourd'hui, `read`, `grep` et `list` le **disent** quand un préfixe ne rend rien.

## 9. Par où entrer dans le code

| Question | Fichier |
|---|---|
| Comment un outil est-il défini ? | `templates/tools/*.mmd`, `src/dataflow/graph_tool.rs` |
| Comment un graphe s'exécute-t-il ? | `src/dataflow/runtime.rs` (`run_level`) |
| Qu'est-ce qu'un résultat de recherche ? | `src/search_strategy.rs`, `src/dataflow/render_nodes.rs` |
| Où vivent les index ? | `src/catalog.rs` (`register_entity`), `src/schema.rs` |
| Comment les événements circulent-ils ? | `src/events.rs`, `src/dataflow/reactor.rs` |
| Comment le code est-il ingéré ? | `src/code.rs` (`ingest_code`, `resolve_across_batches`) |
| Comment l'agent boucle-t-il ? | `src/agent.rs` (`run_inner`) |

## 10. Les documents, dans l'ordre de lecture

- **Vision** : `docs/vision_roadmap_09_2026/01` (la vision), `06` (la feuille de route), `07` (événements, runs et boucles).
- **Hier soir** : `docs/25-aout-2026-18h58/07` à `09` (rapport, objectifs, savoir-faire), `11` (les mesures d'agents), `13` à `18` (session, écoute, identité, monde ouvert, relations, index vectoriel).
- **Aujourd'hui** : ce dossier — `01` (progression et pistes), `02` (architecture), `03` (celui-ci).

---

## Ménage de la machine (ajouté le 27 août)

### Ce qui est réglé une fois pour toutes

| Où | Quoi | Pourquoi |
|---|---|---|
| `rag3db/.cargo/config.toml` | `jobs = -2` | « tous les cœurs sauf deux » — portable, pas un nombre en dur |
| `run_e2e.sh` | `nproc - 2` pour cmake | c'est le build C++ qui fige le poste |
| `run_e2e.sh` | `confined()` — cgroup `MemoryHigh=16G` | que le build récupère **son** cache, pas les pages du bureau |

### Les commandes

```bash
# Le build prend plus (ou moins) de place — ou pas de cgroup du tout
RAG3WEAVER_BUILD_MEMORY_HIGH=24G ./run_e2e.sh
RAG3WEAVER_BUILD_MEMORY_HIGH=0   ./run_e2e.sh

# Ramener en RAM ce qui est parti en zram (demande sudo, refuse si la RAM
# libre ne suffit pas)
./unswap.sh

# Le diagnostic, quand « ça galère » sans que le CPU bouge
free -h && sysctl vm.swappiness
awk '{printf "%.1f Go stockés → %.1f Go de RAM\n", $1/1073741824, $2/1073741824}' /sys/block/zram0/mm_stat
for f in /proc/[0-9]*/status; do sw=$(awk '/VmSwap/{print $2}' "$f" 2>/dev/null); \
  [ -n "$sw" ] && [ "$sw" -gt 100000 ] && echo "$sw $(awk '/^Name/{print $2}' "$f")"; done | sort -rn | head
```

### Ce qu'on a appris, pour ne pas le rechercher

Le 27 août, poste « qui galère », **CPU à 3 %** :

- ce n'était pas un manque de RAM — 51 Go libres ;
- c'était **36 Go de pages compressées dans le zram**, dont Chrome et
  l'éditeur ;
- cause : `vm.swappiness = 150` (défaut CachyOS, pensé pour zram) veut dire
  « je préfère compresser des applications plutôt que jeter du cache
  disque », et un gros build fait défiler des gigaoctets à travers ce cache.

Donc **c'était bien la compilation, mais par ses entrées-sorties, pas par ses
threads** — le `-j` réduit ne traitait pas ça. Le réglage retenu :
`vm.swappiness = 80` (celui par défaut est pensé pour 16 Go de RAM, pas pour
93), le confinement par cgroup en prévention, `unswap.sh` en cure.

## Le dump de vecteurs creux pour lucivy (27 août)

```bash
./run_e2e.sh --test e2e_sparse_dump
# → target/sparse-dump/sparse-docs.jsonl  (2,0 Mo)
#   target/sparse-dump/sparse-queries.jsonl
# SPARSE_DUMP_DIR=/ailleurs pour choisir la destination
```

Produit de **vrais** vecteurs BGE-M3 depuis notre propre code, et imprime la
distribution : `nnz` par vecteur, quantiles de poids, et combien de
dimensions portent la moitié des occurrences — le déséquilibre dont le WAND
tire son élagage, et que des vecteurs synthétiques n'ont pas.

Mesure du 27 août : documents `nnz` médian 38, moyenne 45,2, **215 dimensions
sur 6 583 portent la moitié des occurrences** ; requêtes `nnz` médian 10.
Débit sur burn/Vulkan : **16 vecteurs/s** en documents, 46 en requêtes.

## Placer les modèles sur les cartes (27 août)

```sh
# Une variable par rôle ; RAG3WEAVER_BURN_DEVICE est le défaut de tous.
RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0 \
RAG3WEAVER_BURN_DEVICE_RERANKER=gpu:0 \
RAG3WEAVER_BURN_DEVICE_LLM=gpu:1 \
  ./run_e2e.sh --test e2e_burn_embedder
```

Valeurs : `default` · `cpu` · `gpu:N` · `igpu:N`. Le choix est **dit** au
chargement, et une valeur illisible ne bloque rien : on prévient et on prend
le défaut — un modèle sur la mauvaise carte vaut mieux qu'un modèle qui ne
tourne pas.

⚠ `gpu:1` est la deuxième carte **dans l'ordre de wgpu**, pas forcément celui
de `rocm-smi`. À vérifier une fois par machine :

```sh
rocm-smi --showmeminfo vram   # avant/après, pour voir laquelle a bougé
```

## Tracer la recherche vectorielle filtrée

```sh
RAG3DB_VECTOR_TRACE=1 ./run_e2e.sh --test e2e_code where_the_vector
```

Trois lignes qui disent laquelle des trois étapes du masque n'a pas eu lieu :
le plan de filtre est-il lié, le masque est-il enregistré pour la table,
combien de nœuds porte-t-il. C'est ce qui a trouvé, le 27 août, que le masque
était parfait et que `searchFromUnCheckpointed` ne le consultait pas.

**Un rappel qui a coûté une heure** : le build est en `Release`, donc les
`KU_ASSERT` de l'extension sont compilés à néant. Une forme de plan
inattendue passe sans bruit. La trace remplace l'assertion.

Et recompiler la seule extension, sans refaire le fork :

```sh
cmake --build build/native-test --target rag3db_vector_extension -j$(( $(nproc) - 2 ))
```
