# Knowledge dump — comment on travaille ce soir

Complète [`../6-septembre-2026-11h43/03`](../6-septembre-2026-11h43/03-knowledge-dump.md).

## Mesurer

```sh
# la référence du chemin d'indexation (profil par nœud, drain compris)
./run_e2e.sh --test e2e_mesure_ingestion_code                                   # src/dataflow, BGE-M3 par le démon
RAG3WEAVER_MESURE_MODELE=granite-107m ./run_e2e.sh --test e2e_mesure_ingestion_code
RAG3WEAVER_MESURE_RACINE=/home/lucied/git_workspaces/rag3db/src RAG3WEAVER_MESURE_MODELE=granite-107m \
  ./run_e2e.sh --test e2e_mesure_ingestion_code                                # le cœur C++, 1 642 fichiers, ~1 min
RAG3WEAVER_MESURE_LIGNE_A_LIGNE=1 …                                             # sans l'index HNSW en masse
# le modèle seul                                    # les arêtes seules
RAG3WEAVER_SANS_DEMON=1 ./run_e2e.sh --test e2e_banc_bge_m3 ; ./run_e2e.sh --test e2e_banc_liens
```

Lignes à lire : `[mesure] …`, `[ingest-profile] ms nœud métriques`
(`model_ms` / `write_ms` sur `embed`), `[drain-profile]`, `[link-profile]`,
`symboles/…`, et le bloc `CHARGE` (occupation de la carte, VRAM, plus gros
processus). **Une mesure se fait seul sur la carte** : les suites GPU de
l'autre session et les `rustc` en cours la bruitent (36 → 44 s vus).

## Le démon d'embarquement

- Un seul, sur `127.0.0.1:7878`, lancé par le harnais ; `pidof
  rag3weaver-embeddings` pour le trouver, jamais `pgrep -f` (le motif attrape
  ton propre shell — code 144).
- Le harnais remplace un démon d'une autre construction (empreinte
  `chemin@mtime`). Il choisit son modèle par `RAG3WEAVER_EMBED_MODEL`.
- Ses journaux : `/tmp/rag3weaver-demons/`.
- La carte : `gpu:1` = `0000:07:00.0` = la TV (ROCm GPU[1]) ; `gpu:0` =
  `04:00.0` = le bureau de Lucie (deux écrans). `RAG3WEAVER_BURN_DEVICE_EMBEDDER`
  force ; sinon le régime prend la carte au moins d'écrans actifs.

## Les variables qui comptent

| variable | rôle |
|---|---|
| `RAG3WEAVER_REGIME` | `confort` (défaut du script) / `plein` ; bride seulement si la carte est partagée |
| `RAG3WEAVER_EMBED_THREADS` | producteurs du pipeline (2) |
| `RAG3WEAVER_EMBED_CHAR_BUDGET`, `RAG3WEAVER_GPU_DUTY` | réglages explicites, gagnent sur le régime |
| `RAG3WEAVER_BURN_FLOAT` | `f16` / `bf16` / `f32` — f16 pur rend de faux vecteurs (cosinus 0,708), Flex32 est le défaut |
| `RAG3WEAVER_INGEST_PROFILE` | profil par nœud, drain et symboles compris |
| `RAG3WEAVER_LINK…` | rien : l'expérience CREATE a été retirée |

## Deux sessions dans un arbre

- **`git add` par chemins explicites**, jamais `-A`. Vérifier `git diff
  --cached --stat` avant le commit.
- Un fichier non compilable bloque l'autre session : corriger ou commiter
  tout de suite, et la prévenir (`SendMessage` vers son socket).
- Fichiers de la session moteur ce soir : `burn_*.rs`, `generated/*`,
  `daemon/embeddings.rs`, `bin/rag3weaver-embeddings.rs`, `burn_device.rs`,
  `tests/e2e_banc_*` sauf `e2e_banc_liens.rs`, `tests/common/mod.rs`.
  Les miens : `code.rs`, `catalog.rs`, `record_nodes.rs`, `embedder.rs`,
  `dialect.rs`, `config.rs`, `chunker.rs`, `template.rs`, `agent.rs`,
  `search_nodes.rs`, `render_nodes.rs`, `regime.rs`, les gabarits et docs.

## Pièges appris ce soir

- **Ligne à ligne contre en masse** : un index HNSW et une table d'arêtes
  coûtent cent fois plus ligne à ligne. `bulk_vector_index` et `COPY`.
  `CREATE` au lieu de `MERGE` ne change **rien**.
- `COPY` refuse tout le fichier dès qu'une clé manque ; `MERGE` sautait la
  paire en silence. Les bouts absents sont comptés (`dangling`).
- Un conseil de lot pris tel quel peut demander 3 Go d'attention à la carte
  (256 × 512 jetons) : la borne de surface (`LotBudget::max_area`).
- « La carte la moins chargée » se retourne dès qu'un modèle occupe la carte
  libre. Compter les écrans actifs, pas la VRAM.
- Un test e2e sans `#[ignore]` n'est pas joué par `run_e2e.sh` (`--ignored`).
- `f16` sur toute la chaîne déborde (LayerNorm, softmax) ; le f16 ROCm ne
  compile pas (WMMA gfx12 dans cubecl-hip).
- Le tick de fond, le battement de cœur de la marque, l'entrée « indexer un
  dépôt » n'ont **pas d'hôte** : aucun processus de production ne garde un
  catalogue en vie. C'est la prochaine question de produit.

## Où regarder

| la question | l'endroit |
|---|---|
| « qu'embarque un scope » | `code.rs::own_texts`, `scope_config` |
| « comment on découpe » | `chunker.rs::chunk_lines`, `ChunkStrategy::Lines` |
| « les lots » | `embedder.rs::lot_budget`, `stable_batches`, `LotBudget` |
| « le pipeline » | `record_nodes.rs::embed_pipeline` |
| « les liens en masse » | `record_nodes.rs::copier_les_liens`, `dialect::copy_links_from_csv` |
| « la vue par parent » | `config::GroupBy`, `search_nodes::GroupFrameNode`, `render_nodes::group_key` |
| « le modèle de la base » | `Catalog::check_embedding_model`, `scope::EMBEDDING_MODEL_KEY` |
| « le montage d'un agent » | `agent::mount_agent_services_on` |
| « les gabarits du projet » | `template.rs`, `Catalog::sync_templates` |
