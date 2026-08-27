# 03 — Knowledge dump : tout tester, et où regarder

27 août 2026. Le document qu'on ouvre quand on reprend froid.

## 1. Le strict minimum

```sh
cd extension/rag3weaver

cargo test --lib                              # 757, sans feature
cargo test --features code,openai-llm --lib   # 862, tout
./run_e2e.sh --summary                        # 33 suites, 276 E2E
```

**Le piège n° 1, et j'y suis tombé deux fois** : ne jamais lancer
`cargo test --test e2e_*` à la main. Les E2E ont besoin du lien natif que
`run_e2e.sh` met en place — sinon `undefined symbol: IndexAuxInfo` et une
heure perdue à chercher un bogue qui n'existe pas.

## 2. Lancer une suite, un test

```sh
./run_e2e.sh --test e2e_code                       # une suite
./run_e2e.sh --test e2e_code reingest_is_idem      # un test (filtre par nom)
./run_e2e.sh --test e2e_code 2>&1 | grep '^\[' # juste les traces
./run_e2e.sh --build                               # forcer la reconstruction C++
./run_e2e.sh --features openai-llm --test e2e_cloud_code_agent
```

**Le journal survit à la passe** — depuis le 27 août au soir :

```sh
./run_e2e.sh --summary                             # affiche le résumé…
less extension/rag3weaver/target/e2e-last.log      # …et tout est encore là
RAG3WEAVER_E2E_LOG=/ailleurs/passe.log ./run_e2e.sh --summary
```

Il tenait dans un `mktemp` effacé à la sortie, et seulement en mode
`--summary`. Une suite qui cassait ne laissait que des compteurs : retrouver
*quel* test coûtait une demi-heure de relance pour une ligne qu'on avait déjà
eue sous les yeux. Le résumé **nomme** maintenant les tests en échec.

> **Écrire d'abord, filtrer ensuite.** La règle vaut pour tout ce qui est long,
> pas seulement pour cette commande : `2>&1 | tee journal.log`, puis le `grep`
> qu'on veut. Un appelant qui réduit la sortie à trois lignes ne doit jamais
> être le seul endroit où elle a existé.

**Une passe à la fois.** Vérifier d'abord :

```sh
pgrep -f "[c]argo test --features rag3db-native" && echo occupé
```

(Ne pas guetter `run_e2e.sh` : le guetteur se trouve lui-même et ne sort
jamais. J'ai écrit ce bogue, il est instructif.)

## 3. Les variables d'environnement

| Variable | Sert à |
|---|---|
| `RAG3WEAVER_INGEST_PROFILE=1` | durée de chaque nœud d'ingestion |
| `RAG3DB_VECTOR_TRACE=1` | les trois étapes du masque HNSW |
| `RAG3WEAVER_BUILD_MEMORY_HIGH=24G` | budget du cgroup de build (`0` = désactivé) |
| `RAG3WEAVER_BURN_DEVICE_{EMBEDDER,RERANKER,OCR,LLM}=gpu:1` | carte par rôle |
| `RAG3WEAVER_BURN_DEVICE` | défaut global (`default` \| `cpu` \| `gpu:N` \| `igpu:N`) |
| `RAG3WEAVER_TIMEZONE`, `TZ` | fuseau d'affichage |
| `RAG3WEAVER_LOCAL_LLM`, `RAG3WEAVER_LOCAL_MODEL` | agent local |
| `RAG3WEAVER_BGE_M3_BPK`, `_TOKENIZER` | poids BGE-M3 |
| `SPARSE_DUMP_DIR` | où écrire le dump de vecteurs |
| `RAG3DB_ROOT` | racine du fork, si elle n'est pas déduite |

## 4. L'agent, en nuage et en local

```sh
# Vertex — ne jamais imprimer le contenu du fichier de clé
export GOOGLE_APPLICATION_CREDENTIALS="$PWD/../../.vault/vertex-sa.json"
export GOOGLE_CLOUD_PROJECT="$(python3 -c "import json;print(json.load(open('$GOOGLE_APPLICATION_CREDENTIALS'))['project_id'])")"
./run_e2e.sh --features openai-llm --test e2e_cloud_code_agent

# Local — llama.cpp EST le client OpenAI, aucun adaptateur
~/git_workspaces/llama.cpp/build/bin/llama-server \
  -m ~/ML/models/Qwen3-Coder-30B-abliterated-Q6_K/*.gguf \
  --device Vulkan1 -ngl 99 --host 127.0.0.1 --port 8080 --jinja \
  -c 131072 --flash-attn on --cache-type-k q8_0 --cache-type-v q8_0 --cache-ram 2048

RAG3WEAVER_LOCAL_LLM=http://127.0.0.1:8080/v1 RAG3WEAVER_LOCAL_MODEL=qwen3-coder-30b \
  ./run_e2e.sh --features openai-llm --test e2e_cloud_code_agent
```

- `--jinja` **n'est pas facultatif** : sans lui, pas de gabarit, donc pas
  d'appels d'outils.
- `--device Vulkan1` : la carte qui **ne** pilote **pas** l'écran.
- VRAM mesurée : 28,3 Go à 32k · 31,6 à 64k · **32,0 à 128k avec le cache en
  `q8_0`**. Le modèle est entraîné pour 256k.

## 5. Mesurer

```sh
# Ingestion, par phase puis par nœud
./run_e2e.sh --test e2e_code reingest_is_idempotent
RAG3WEAVER_INGEST_PROFILE=1 ./run_e2e.sh --test e2e_code ingest_our_own

# Index vectoriel : incrémental contre construction en masse
./run_e2e.sh --test e2e_code building_the_vector

# Le masque du HNSW, verdict imprimé
RAG3DB_VECTOR_TRACE=1 ./run_e2e.sh --test e2e_code where_the_vector

# Vrais vecteurs BGE-M3 (pour lucivy), avec la distribution
./run_e2e.sh --test e2e_sparse_dump
```

## 6. La machine

```sh
free -h && sysctl vm.swappiness
awk '{printf "%.1f Go stockés → %.1f Go de RAM\n", $1/1073741824, $2/1073741824}' /sys/block/zram0/mm_stat
rocm-smi --showmeminfo vram
./unswap.sh                       # ramener le zram en RAM (sudo)
```

**Ce qu'on a appris le 27 août** : poste qui rame, **CPU à 3 %**, 51 Go
libres — et 36 Go de pages compressées dans le zram, dont Chrome et
l'éditeur. Cause : `vm.swappiness = 150` (défaut CachyOS, pensé pour zram) et
un gros build qui fait défiler des gigaoctets dans le cache de fichiers.
C'était donc bien la compilation, **par ses entrées-sorties et pas par ses
threads**. Réglages retenus : `jobs = -2`, cgroup sur le build,
`vm.swappiness = 80`.

## 7. Recompiler juste ce qu'il faut

```sh
cmake --build build/native-test --target rag3db_vector_extension -j$(( $(nproc) - 2 ))
```

Le fork entier prend beaucoup plus longtemps ; l'extension seule, une minute.
Et **le build est en `Release`**, donc les `KU_ASSERT` sont compilés à néant :
une forme de plan inattendue passe sans bruit. C'est pour ça que la trace
existe.

## 8. Les pièges, par probabilité

1. `cargo test --test e2e_*` à la main → `undefined symbol`.
2. Deux passes en parallèle → durées fausses, contention du dossier de build.
   **Et n'importe quelle compilation pendant une passe compte comme une
   deuxième** : même `target/`, même verrou cargo. Les suites tardives ne
   testent alors plus le même code que les premières — *une passe incohérente
   est pire qu'une passe absente*, elle donne un vert auquel on croit.
3. Ne garder qu'une sortie filtrée d'une commande longue : le jour où elle
   casse, ce qu'on cherche est exactement ce qu'on a jeté (§2).
4. Filtrer une date par préfixe sur `at` → faux du décalage de fuseau.
5. Croire un commentaire plutôt que mesurer.
6. Publier une durée absolue mesurée pendant qu'autre chose tourne (±30 %).
7. Oublier `--jinja` sur `llama-server`.
8. Ajouter un champ à une entité et casser un `EntityConfig { .. }` littéral
   ailleurs — la passe complète devient muette (« 0 passed, 33 non lancées »).
9. Un `%% param:` sans type reste à lier ; sans `%% choices:`, le modèle
   invente des valeurs.
10. Chercher `Symbol` par vecteur : il n'a pas de chunk, donc pas d'embedding.
11. Laisser `target/` grossir : 100 Go par jour de binaires périmés, cargo ne
    ramasse jamais. Le disque à 94 %, c'était ça. **Corrigé le 27 au soir** :
    `run_e2e.sh` fait le ménage avant chaque passe (`menage_target.py`,
    `RAG3WEAVER_NO_GC=1` pour l'éviter) et met `CARGO_INCREMENTAL=0` — il ne
    rapporte rien sur 34 binaires construits une fois, et coûtait 124 Go en
    trois jours. La boucle d'édition, elle, garde l'incrémental.

## 9. Où regarder quand ça casse

| Symptôme | Regarder |
|---|---|
| Un résultat de recherche trop large | le filtre est-il honoré par **ce** nœud ? `generic_search_nodes.rs` |
| Un scope dupliqué après un `edit` | l'identité — `code.rs::stable_scope_keys` |
| Un fichier en double | `origin.rs`, et la source du curseur |
| Une relation qui disparaît | la couche `Symbol`, `code.rs::resolve_across_batches` |
| Un agent muet | `postures.rs` — est-il en pause ? un blocage ? |
| Un outil qui ne rend rien | est-il `%% async:` ? le résultat est dans la boîte |
| Une date fausse | `at` est UTC ; filtrer sur `at_ms` |
| Une ingestion lente | `RAG3WEAVER_INGEST_PROFILE=1` — c'est l'index, pas les embeddings |

## 10. Les tests qui valent une lecture

- `e2e_code::a_project_converges_however_it_was_ingested` — les quatre façons
  dont un index grossit.
- `e2e_code::where_the_vector_pre_filter_stands_today` — un canari qui
  imprime son verdict.
- `e2e_code::the_same_file_seen_from_two_roots_is_one_identity` — un test de
  constat devenu test de propriété.
- `agent::tests::an_async_tool_answers_with_a_handle_and_the_result_comes_later`
- `postures::tests::two_agents_waiting_on_each_other_is_a_deadlock`
- `trace_nodes::tests::a_period_is_a_shorthand_and_it_knows_the_calendar` —
  mars fait 743 heures.
