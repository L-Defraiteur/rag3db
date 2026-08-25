# 09 — Knowledge dump : lancer, trouver, ne pas se faire avoir

25 août 2026, minuit. Ce qu'il faut savoir pour reprendre le travail sans
relire quarante documents. Tout est vérifié ce soir.

## 1. Lancer

```sh
cd extension/rag3weaver

# Unitaires (rapides). Le jeu de features change ce qui compile ET ce qui compte.
cargo test --lib                                   # 720, sans feature
cargo test --features code,openai-llm --lib        # 806, avec les nœuds de code et le client cloud

# E2E : TOUJOURS par le script — il pose LD_LIBRARY_PATH, RAG3DB_*, les features.
./run_e2e.sh --summary                             # les 30 suites, résumé par suite (≈ 10 min, burn_llm en prend 2)
./run_e2e.sh --test e2e_code                       # une suite
./run_e2e.sh --test e2e_search phase4              # un filtre dans une suite
./run_e2e.sh --build                               # forcer la reconstruction de rag3db (C++ modifié)
./run_e2e.sh --features openai-llm --test e2e_cloud_code_agent   # ajouter une feature au jeu

# Un seul test unitaire, avec sa sortie
cargo test --features code --lib own_source_tests -- --ignored --nocapture
```

Le jeu de features par défaut des E2E : `rag3db-native,burn-embedder,burn-llm,burn-ocr,code`.
Un test sous `#![cfg(feature = "…")]` hors de ce jeu **n'existe pas** — la
suite affiche `0 passed` sans rien dire. `--features a,b` ajoute au jeu.

**Le résumé du script** : s'il n'a pas 30 lignes, il le dit (« INCOMPLETE —
n suite(s) not run »). Avant le 25 août il affichait un total partiel comme
un total complet. Un `TOTAL` sans `INCOMPLETE`, c'est un vrai total.

**Compter nos diagnostics dans le bruit de lucivy** :
`cargo check … 2>&1 | grep -cE '^\s+--> (src|tests|examples)/'`. Un
`grep "rag3weaver/src/"` ne trouve **jamais rien**. Un `cargo` lancé depuis
la racine du dépôt ne compile rien et rend « 0 » par absence de sortie.

## 2. Les tests qui coûtent ou qui demandent quelque chose

| test | demande | coûte |
|---|---|---|
| `e2e_cloud_code_agent`, `e2e_cloud_schema_probe` | `GOOGLE_APPLICATION_CREDENTIALS=…/.vault/vertex-sa.json` et `GOOGLE_CLOUD_PROJECT=<project_id du JSON>` ; sautent sinon | des centimes par question (14 000–40 000 jetons) |
| `e2e_burn_code_agent`, `e2e_burn_agent`, `e2e_burn_llm` | les poids dans `~/.cache/rag3weaver/qwen2.5-0.5b-instruct/` (téléchargés au premier passage) | 1–2 min, 2 Go de GPU chacun — ne pas les enchaîner en parallèle |
| `e2e_hnsw_scale` | `RAG3DB_PROBE_HNSW=1` pour les sondes à 4 096 (3 min) ; les 1 024 sont des canaris permanents | — |
| `e2e_burn_ocr`, `*_reranker`, `*_minilm` | poids dans `~/.cache/rag3weaver/` | GPU |

La clé Vertex : **`.vault/vertex-sa.json` à la racine du dépôt** (gitignoré).
Celle de `~/LR_CodeRag/secrets/` est l'ancienne, révoquée. Le `project_id`
est dans le JSON. Ne jamais afficher le fichier ; filtrer les sorties sur la
valeur du jeton, pas sur le mot « token ».

## 3. Où regarder dans le code

`extension/rag3weaver/src/` :

| quoi | où |
|---|---|
| le catalogue, `search` (le monolithe), `ingest_entities`, `link`, `drain`, `find_by_field`, `entity_uuid` | `catalog.rs` |
| configuration d'entité (`hashsafe`, `return_fields`, `chunking`), KB | `config.rs` |
| fusion N-aire, `SearchTarget`, `fuse_signals`, `DEFAULT_RRF_K` | `search.rs` |
| les nœuds de recherche (`SearchSourceNode` … `FuseResultsNode`, `RerankNode`) | `dataflow/generic_search_nodes.rs` |
| fabriques et **`BUILTIN_NODE_COUNT`** (à lire, jamais à écrire en dur) | `dataflow/node_factories.rs` |
| graphes-outils, `%% tool:` / `%% param:`, `builtin_graph_tools`, **`BUILTIN_TOOL_NAMES`**, `render_port_value` | `dataflow/graph_tool.rs` |
| gabarits Mermaid (`$var` nu = typé, `'$var'` = chaîne) | `dataflow/mermaid.rs`, `templates/*.mmd`, `templates/tools/*.mmd` |
| le code comme graphe : schéma `File`/`Scope`/`Library`, `analyze`, `analyze_source`, `ingest_code`, `fold_lambdas` | `code.rs` (feature `code`) |
| `FileSource` (`WorkingTree`, `Snapshot`), `read_file`, `grep_files`, `list_files`, `edit_file`, `reingest_file` | `code_tools.rs` |
| `ParseCodeNode`, `CodeIngestNode`, `ReadFileNode`, `GrepNode`, `ListFilesNode`, `EditFileNode` | `dataflow/code_nodes.rs` |
| la boucle d'agent, `AgentLimits` (**`final_nudge`**), `GraphToolBox` | `agent.rs` |
| le trait `Llm`, `ToolCall`, `repair_arguments_json`, `arguments_for_wire` | `llm.rs` |
| le client SSE, Vertex / AI Studio, `read_sse`, `stray_error`, **`RAG3WEAVER_SSE_DUMP`** | `openai_llm.rs` |
| JWT + OAuth2 Google | `gcp_auth.rs` |
| `ToolDef` depuis `NodeSchema` (tri stable pour le cache de préfixe) | `tools.rs` |
| le modèle local, l'OCR, les embedders, les rerankers | `burn_llm.rs`, `burn_ppocr.rs`, `burn_*_embedder.rs`, `burn_*reranker.rs` |
| index plein-texte lucivy (`upsert_document`) | `fts_handle.rs` |
| `HashEmbedder` (vecteurs unitaires déterministes) — **pas `MockEmbedder`** (vecteurs nuls) dès qu'on ingère plus qu'une poignée | `embedder.rs` |

`extension/rag3weaver/codeparsers/` : le parseur (tree-sitter, 12
langages). Entrée : `parallel/project_parser.rs` (`ProjectParser`,
`ParseProjectOptions.resolver_options`), `parallel/parser_worker.rs`
(`finalize` : hash, offsets), `relationship_resolution/relationship_resolver.rs`
(`include_file_level_refs`, `include_child_refs`), `utils/text.rs`
(troncatures UTF-8 sûres). Tests : `tests/relationships.rs` (65),
`tests/finalize.rs` (4).

Le cœur C++ et l'extension vectorielle : `src/storage/`,
`extension/vector/src/index/hnsw_index.cpp` (`shrinkForNode`,
`HNSWInsertState`, `pendingVector`). Build Debug :
`build/native-debug/` (`-DCMAKE_BUILD_TYPE=Debug -DENABLE_RUNTIME_CHECKS=ON`),
puis le binaire de test avec `LD_LIBRARY_PATH=build/native-debug/src`.
**Le fichier `extension/vector/build/libvector.rag3db_extension` est partagé
entre les builds** — le dernier construit gagne ; reconstruire `native-test`
après un passage en Debug.

## 4. Les services que les nœuds attendent (`ServiceRegistry`)

`"catalog"` (`Arc<Mutex<Catalog>>`), `"conn"` (`ConnService`), `"embedder"`
(`Arc<dyn Embedder>`), `"fts_handles"`, `"sparse_handles"`, `"reranker"`
(`Arc<dyn Reranker>`), **`"file_source"`** (`Arc<dyn FileSource>`),
`"ocr"`, `"llm"`. Les E2E montrent comment les assembler
(`tests/e2e_generic_search.rs::build_services`, `tests/e2e_code.rs`,
`tests/e2e_cloud_code_agent.rs::setup_on`).

## 5. Les pièges connus, par ordre de probabilité

1. **Un test sous une feature absente du jeu** compile et rend 0 — voir §1.
2. **Compteurs en dur** (`28 nœuds`, `2 outils`) : lire `BUILTIN_NODE_COUNT`
   et `BUILTIN_TOOL_NAMES`, ils suivent les features.
3. **`Catalog::get` rend le nœud entier sous `"n"`** ({"n": Map{…}}) ;
   `find_by_field` rend des colonnes `n.champ`. `code_tools::col` sait les
   trois formes.
4. **`MockEmbedder` = vecteurs nuls.** Bon pour trois lignes, pas pour mille.
5. **`ureq` lève une erreur sur les statuts non-2xx par défaut** : sans
   `http_status_as_error(false)`, le corps de l'erreur est perdu. Corrigé
   dans `openai_llm` et `gcp_auth` ; y penser pour tout nouveau client.
6. **Vertex `stream_function_call_arguments`** : ne pas réactiver sans lire
   [06](06-lacher-lagent-sur-notre-code.md) §6.
7. **`undefined symbol: rag3db::catalog::IndexAuxInfo` au `LOAD EXTENSION`.**
   Deux causes, à distinguer avant d'agir :
   - un `cargo test --test …` lancé **à la main**, sans `RAG3DB_SHARED=1`,
     `RAG3DB_LIBRARY_DIR` et `LD_LIBRARY_PATH` : le crate `rag3db` recompile
     alors le cœur **en statique** (longue recompilation, et l'extension ne
     trouve plus ses symboles dans le binaire). Remède : passer par
     `./run_e2e.sh --test <suite>`, qui exporte ces variables — les suites
     E2E ne se lancent pas autrement ;
   - une extension vraiment périmée après un rebuild du cœur (ou un crash
     ABI) : `rm extension/vector/build/libvector.rag3db_extension` puis
     `cmake --build build/native-test --target rag3db_vector_extension`.
8. **Ne pas toucher `.gitmodules`** (modifié localement par Lucie), ni
   l'arbre de lucivy (`~/git_workspaces/lucivy`, dépendance par chemin,
   voulue). Pas de trailer d'attribution IA dans les commits.
9. **Le chemin d'un fichier est relatif à la `FileSource`**, pas au dépôt :
   `services.rs`, pas `src/dataflow/services.rs`, quand la source est
   enracinée dans `src/dataflow`. `read` propose le bon chemin en cas
   d'erreur.

## 6. Les documents, et dans quel ordre les lire

1. `../vision_roadmap_08_2026/00` → `01` (la vision), `06` (la feuille de route).
2. Ce dossier : [01](01-rapport-de-progression.md) et
   [07](07-rapport-de-progression-soir.md) (la journée), [02](02-fichiers-en-temps-reel-deux-modes-git-et-histoire.md)
   (les décisions de conception du code), [06](06-lacher-lagent-sur-notre-code.md)
   (ce que l'agent fait et ne fait pas).
3. Au besoin : `../23-aout-2026-20h33/` 43 (passation, pièges de
   vérification), 47 (LLM/TTS/STT), 48 (pour lucivy), 52 (recherche
   composable) ; `docs/25-aout-2026-20h30/01` à la racine (le bug HNSW).
