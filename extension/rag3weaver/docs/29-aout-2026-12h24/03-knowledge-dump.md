# 03 — Comment on lance, où on regarde

*29 août 2026. Ce qu'il faut savoir pour reprendre sans redécouvrir.*

## 1. Lancer les tests

```sh
cd ~/git_workspaces/rag3db/extension/rag3weaver

cargo test --lib --features rag3db-native,burn-embedder,burn-ocr,code   # 880, instantané
./run_e2e.sh --summary                                                  # 265, ~8 min
./run_e2e.sh --summary --test e2e_code                                  # une suite
./run_e2e.sh --summary --test e2e_code read_and_grep                    # un test
./run_e2e.sh --features openai-llm --test e2e_cloud_code_agent          # features en plus
```

**Épingler les cartes**, sinon l'embedder atterrit sur l'affichage :

```sh
RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:1 \
RAG3WEAVER_BURN_DEVICE_RERANKER=gpu:1 \
RAG3WEAVER_BURN_DEVICE_OCR=gpu:1 \
  ./run_e2e.sh --summary
```

**Mesuré le 29 août** : `gpu:0` = card2 = les écrans ; `gpu:1` = card0 = la
libre. À **revérifier** — l'ordre d'énumération de wgpu dépend de ce qui est
disponible au lancement. `charge.py` le montre en deux lignes.

## 2. Les identifiants existent

`~/git_workspaces/rag3db/.vault/` — **ne jamais en imprimer le contenu**.

```sh
GOOGLE_APPLICATION_CREDENTIALS=~/git_workspaces/rag3db/.vault/vertex-sa.json \
GOOGLE_CLOUD_PROJECT=lr-hub-472010 \
  ./run_e2e.sh --features openai-llm --test e2e_cloud_code_agent
```

Un `0 passed` sur une suite cloud est un **saut**, pas un succès. Le test
s'arrête proprement quand la variable manque.

## 3. L'agent local, par llama.cpp

```sh
~/git_workspaces/llama.cpp/build/bin/llama-server \
  -m ~/ML/models/Qwen3-Coder-30B-abliterated-Q6_K/*.gguf \
  --device Vulkan1 -ngl 99 --host 127.0.0.1 --port 8080 --jinja \
  -c 131072 --flash-attn on --cache-type-k q8_0 --cache-type-v q8_0

RAG3WEAVER_LOCAL_LLM=http://127.0.0.1:8080/v1 RAG3WEAVER_LOCAL_MODEL=qwen3-coder-30b \
RAG3WEAVER_ARTEFACT=essai \
  ./run_e2e.sh --features openai-llm --test e2e_conversation_a_plusieurs
```

`--jinja` **n'est pas facultatif** : sans lui, pas de gabarit, donc pas
d'appels d'outils. Le serveur tient 30,5 des 32,6 Go de card0 — donc pendant
qu'il tourne, l'embedder ne peut pas y aller. L'arrêter quand on ne s'en sert
pas, ou baisser `-c`.

L'artefact sort dans `target/artefacts/fil-<nom>-<horodatage>.md`, écrit **au
fil de l'eau** : `tail -f` marche pendant que les agents parlent.

## 4. Mesurer la machine

`charge.py` échantillonne pendant les passes ; le résumé des pics s'affiche à
la fin.

```sh
./charge.py --une-fois
./charge.py --sortie /tmp/charge.tsv --intervalle 5 &
./charge.py --resume /tmp/charge.tsv
```

**La colonne qui répond à « mon PC rame » est `io_full`** —
`/proc/pressure/io`, le pourcentage de temps où *aucune* tâche n'avance. Le
28 août : 60 %, avec le CPU à 11 % et zéro pression mémoire. La cause était
`vm.dirty_bytes = 256 Mo` (CachyOS) contre 48 Go écrits par une passe.

`split-debuginfo = "unpacked"` a divisé ça par deux (1,4 Go → 739 Mo par
binaire, `target/debug/deps` de 162 à 26 Go). Le levier restant est
`vm.dirty_bytes`, qui demande root et n'a pas été appliqué.

## 5. Où regarder, par symptôme

| symptôme | où |
|---|---|
| un résultat de recherche est mal rendu | `dataflow/render_nodes.rs`, `templates/render/*.jinja` |
| un outil manque ou a le mauvais nom | `templates/tools/*.mmd`, `graph_tool.rs::builtin_graph_tools` |
| un classement est faux | isoler les signaux : `SearchOptions.signals`, puis le cosinus nu |
| une relation n'existe pas dans le graphe | `code.rs::key_data` — la clé d'identité doit être **entière** |
| une docstring manque | `codeparsers/src/scope_extraction/*_parser.rs` |
| un lot d'embedding est trop gros | `record_nodes.rs::budget_batches`, `RAG3WEAVER_EMBED_CHAR_BUDGET` |
| la carte est saturée | `RAG3WEAVER_GPU_DUTY`, mais l'ingestion n'utilise que 2 % du GPU |
| un gabarit ne se pose pas | `template.rs::place_entity_with`, puis `EntityConfig::validate` |

## 6. Les pièges, par probabilité

1. **Compiler pendant une passe** — la passe devient incohérente.
2. **Un test qui n'assère que l'appartenance** — « la liste contient `user` »
   est vrai quel que soit l'ordre. Affirmer le **premier**.
3. **Un montage qui contredit sa question** — indexer `src/dataflow` et
   demander où est le catalogue.
4. **`cargo build --lib` ne compile pas le code sous features** — toujours
   `--features rag3db-native,burn-embedder,burn-ocr,code`.
5. **`chunked: false` avec des signaux vectoriels** — refusé, et c'est bien :
   l'index vectoriel vit sur la table de chunks.
6. **Un `%%` d'en-tête mal orthographié** dans un `.mmd` passe en silence.
7. **crates.io sans `User-Agent`** rend une réponse vide, pas une erreur — on
   conclut à tort que rien n'est publié.
8. **Un sentinelle `grep -q` trop large** : `SUMMARY` matche « BATCHING
   SUMMARY », `TOTAL` matche « TOTAL : 186.9 ms ». Utiliser
   `^  TOTAL +[0-9]+ passed`.
9. **Un embedder factice** (`HashEmbedder`) rend `search` borgne sans le dire.
10. **`.gitmodules` est modifié localement** — ne jamais le commiter.

## 7. Les règles de méthode

**Écrire d'abord, filtrer ensuite.** Rediriger vers un fichier, puis `tail -f`
dessus. Jamais de pipe filtrant en bout de chaîne, jamais de guetteur `pgrep`.

**Séparer les responsabilités.** Un type de plus ne se justifie pas ; les
fondre, si. Quatre notions appelées « racine » l'ont prouvé.

**Laisser deux cœurs libres** : `-j $(( $(nproc) - 2 ))`, jamais `-j$(nproc)`.

**Mesurer avant de conclure**, et le dire quand la mesure contredit. Cette
semaine : l'iGPU recommandé puis retiré, « on envoie un par un » qui était
faux, les threads lucivy accusés à tort (31 s → 14 s était un démarrage à
froid), et un test vert qui ne prouvait rien.

## 8. Les tests qui valent d'être lus

- `e2e_catalogue_gabarits.rs` — les briques isolées : cosinus nu, puis chaque
  signal. C'est le modèle de la dichotomie.
- `e2e_code.rs::read_and_grep_as_graph_tools` — la surface d'outils sur du vrai
  code : chemins relatifs, arbre de relations, `grep(relation)`.
- `e2e_agent_loop.rs` — la boucle, la trace à deux étages, l'interruption.
- `e2e_conversation_a_plusieurs.rs` — l'expérience à plusieurs, et son artefact.
- `e2e_charge_ingestion.rs` — ce qu'une vraie ingestion coûte à la machine.

## 9. Ailleurs

- `~/LR_CodeRag/ragforge/docs/BRAIN_SEARCH_OUTPUT_PROPOSAL.md` et
  `brain_search_example_output.md` — la forme du rendu vient de là.
- `~/LR_CodeRag/community-docs/packages/ragforge-core/src/tools/` — la maquette
  d'origine : `grep_files({analyze})`, `search_files` flou,
  `change_directory`, `extract_dependency_hierarchy`.
- `~/git_workspaces/lucivy` — branche `main`, publiée en **3.0.8** sur
  crates.io. On prend la version, plus le chemin.
