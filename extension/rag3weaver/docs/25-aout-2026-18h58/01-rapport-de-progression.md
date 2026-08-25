# 01 — Rapport de progression, 25 août 2026

Écrit à 18h58, après trente commits dans la journée (`c5f0bb0ed` → `3d4c51f83`,
134 fichiers, +37 497 / −709). La trace détaillée est dans
`../23-aout-2026-20h33/` (docs 44 à 52) et la couche au-dessus dans
`../vision_roadmap_08_2026/`. Ce document est le résumé de ce qui a bougé, ce
qui est mesuré, ce qui a été décidé, et ce qui attend.

## 1. Ce qui est livré aujourd'hui

| chantier | commits | état |
|---|---|---|
| **OCR unitaire** (chantier 4) | `0a25239d0`, `e8d08d711` | `trait Ocr`, `OcrNode`, PP-OCRv6 tiny (det + rec) sur burn, post-DB et CTC en Rust pur. Poids publiés sur HF, re-téléchargement vérifié. |
| **LLM — trait et fournisseurs cloud** (chantier 4 bis) | `ad7db7f92`, `3ac47c8ca`, `63ff04942` → `649fd93da` | `trait Llm` en streaming par puits, `OpenAiLlm` (SSE, ureq 3), Vertex et AI Studio, auth GCP sans nouveau crate, identité des appels d'outils préservée à travers les interruptions, `reasoning_effort` + `thought_signature`, `tool_choice` / `response_format` avec vérification du mode strict, réessai avec backoff. **Bug latent corrigé** : la branche non-200 de ureq ne s'exécutait jamais. |
| **Définitions d'outils** | `898021ec2` | `ToolDef` généré depuis `NodeSchema`, tri stable pour le cache de préfixe. |
| **Un outil est un graphe** | `76a566b58` | `GraphTool` = graphe + spécificateur (`%% tool:`, `%% param:`, `%% result:`), substitution typée, imbrication démontrée. |
| **La boucle d'agent** | `c77a2c809` | `Agent::run`, `ToolBox`, limites, reprise après interruption. |
| **Agent hors ligne** | `37acd3f9f` | `BurnLlm` sur Qwen2.5-0.5B (996 Mo), la boucle tourne sans réseau. 4,7 j/s, trois fois plus lent que le graphe nu — inexpliqué. |
| **Gabarits : `$var` typé** | `b8bf5f57f` | Six gabarits sur sept avaient `limit` et `gpu_batch_size` jamais respectés (chaîne → `as_u64()` → défaut). Corrigé, régressions ajoutées. |
| **Recherche composable** | `3d4c51f83` | Signaux étiquetés, fusion N-aire avec port `signals` en fan-in et poids par nom, `BM25SearchNode(fields=…)`, `RerankNode` (remplace après la fusion, module en `boost` dedans), vecteur via `SearchBackend`, `result_mode` sur les nœuds de signal, gabarit `weighted_search.mmd`. |
| **Documents** | 12 commits | 46 (OCR), 47 (LLM/TTS/STT, sept révisions), 48 (pour lucivy), 49 (catalogue comme graphe), 50 (chemin local), 51 (la vision), 52 (recherche composable), et la série `vision_roadmap_08_2026/` en six documents. |

## 2. Ce qui est mesuré

- **Unitaires** : 720 / 720 (`cargo test --lib`), dont 16 sur les nœuds
  génériques de recherche et 23 sur le parseur mermaid.
- **`e2e_generic_search`** : 12 / 12. Le test décisif : ordre BM25
  `[Rust, Python, Knife]`, rerank en boost préférant le couteau →
  `[Knife, Rust, Python]` — le boosté monte, les deux autres gardent leur
  ordre. Les huit tests d'équivalence nœuds ↔ `Catalog::search` restent verts
  après le passage du vecteur par le backend.
- **`e2e_graph_tool`** 4 / 4, **`e2e_agent_loop`** 4 / 4.
- **Passe E2E complète du soir** : 28 suites, **234 / 234** après deux
  correctifs d'environnement — voir §7.
- **Vertex** : 0,72 $ consommés sur les crédits de démarrage, confirmé par
  Lucie sur le solde (1 995,09 → 1 994,37 $).
- **Le matin** : passe complète 206 / 206 avant le chantier OCR.

## 3. Ce qui a été décidé (par Lucie, dans la journée)

1. **Le LLM local n'est pas une ambition** — on se base sur llama.cpp, Ollama
   ou un endpoint compatible OpenAI ; on a laissé `BurnLlm` aller au bout
   *pour l'expérience*, et il tourne. TTS/STT locaux gardent leur intérêt pour
   l'agent de code embarqué (le développeur a déjà un LLM, pas un moteur de
   parole).
2. **Aucun Cypher pour les agents.** Agnosticité de backend, pas sûreté : si
   une capacité manque à l'abstraction, on l'ajoute, on ne régresse pas vers
   une requête qui nous lie à cette base pour toujours.
3. **Un outil est un graphe entier**, s'il a un spécificateur.
4. **Une entité `Tag`** liable à tout, deux espaces dont un temporel ; le
   reranker pourrait aider à décider quel tag se lie à quel autre.
5. **Pas de MCP maintenant.** Il manque de quoi avaler le réel :
   `codeparsers`, un graphe de normalisation xlsx, des lecteurs de documents.
6. **La vision** : un chaos contrôlé — un agent de code qui gère la base dans
   laquelle il vit et peut construire des backends avec la même technologie.
   Et le rappel : la KB a été faite pour **ingérer des catalogues
   génériquement**.
7. **Les pondérations sont des topologies, pas des réglages.** La KB garde
   l'index et perd la recette.

## 4. Ce qui a été trouvé et qu'on n'a pas caché

- **Trois vérifications aveugles payées** (doc 43) : un `grep` qui ne trouvait
  jamais rien, un `cargo` lancé depuis la mauvaise racine, un script relisant
  la réponse précédente sur HTTP 000. Filtre corrigé :
  `grep -cE '^\s+--> (src|tests|examples)/'`.
- **`Catalog::search` n'est pas un graphe** : 450 lignes impératives, et le
  mode « composé » ne fait que le rappeler. Deux chemins de recherche à
  maintenir tant qu'il ne construit pas lui-même un gabarit.
- **La KB était à moitié construite** : les index multi-stratégies de février
  existent, la combinaison inter-KB n'a jamais existé — une requête = une
  cible.
- **Le titre d'une entité simple n'est pas dans l'index BM25**
  (`bm25_fields = content_fields()`).
- **`fusion.rs` est mort et public** ; `title_boost`, `content_boost`,
  `special_ops`, `boost` par champ : désérialisés, jamais lus. Annotés.
- **`codeparsers`** (repérage fait, non intégré) : compile, 65 tests verts,
  mais `RelationshipResolutionResult.files` et `.external_libraries` sont
  **toujours vides**, les résolveurs d'imports (2 150 lignes) ne sont jamais
  instanciés, la résolution des relations exige tous les fichiers à la fois,
  et seules les lignes sont exposées — pas les offsets d'octets, disponibles
  gratuitement au point d'extraction.
- **`AsyncScope` de luciole est un exécuteur, pas un réacteur** ; `StreamDag`
  ne porte pas de `PortValue`. Rapport 48 envoyé à la session lucivy.
- **L'export fp16 de Qwen2.5 chez onnx-community est numériquement cassé** ;
  canaris en place.

## 5. En attente d'une décision humaine

- **`cargo publish` de lucivy 3.0.0** — irréversible. Ensuite : dépendances
  par chemin → `lucivy-core = "3"`, `sparse-vector = "0.3"`,
  `luciole = "0.2"`, et vérifier une seule entrée `luciole` dans `Cargo.lock`.
- **Rapport de bugs à `tracel-ai`** — 16 entrées, 8 avec cas minimal, dans le
  scratchpad. Non publié.
- **Licence du lexique de prononciation français** — binaire permissif contre
  Lexique 4.00 (CC BY-SA).

## 6. La suite, dans l'ordre

1. **`codeparsers` intégré** — nœud `ParseCode` fichier-par-fichier → entités
   `File` / `Scope` / `Scope_Chunk` + relations ; résolution des relations en
   nœud séparé (globale) ; `File` jamais chunké, source des offsets ;
   `project` dès le premier jour. Corriger d'abord les deux maps vides et
   exposer les offsets d'octets.
2. **`grep` et `read`** sur `File`.
3. **`Catalog::search` devient un gabarit** — le monolithe en
   `search_default.mmd`, `KBConfig` réduite à des défauts de variables ;
   `fusion.rs`, `title_boost`, `content_boost` partent avec.
4. Titre des entités simples indexé en BM25 ; `column=` sur le vecteur ; un
   E2E de fusion inter-KB.
5. Lecteurs de documents, graphe de normalisation xlsx, rapport de validation
   à l'ingestion.

## 7. Passe E2E complète du soir

Lancée à 19h après `3d4c51f83`. **Elle a d'abord échoué, et c'est
l'environnement, pas le code** — consigné parce que ça reviendra :

- `e2e_idempotent_registration` : 4 tests sur 22 en échec, tous sur
  `Rag3dbConnection::in_memory()` avec
  `Mmap for size 8796093022208 failed`. Relancée seule : **4 autres** tests,
  même erreur. Puis `e2e_search` : 15 sur 38, même erreur.
- Cause : chaque base en mémoire réserve **8 TiB** d'espace d'adressage
  virtuel ; `cargo test` lance jusqu'à `nproc` = 24 tests en parallèle dans
  un même processus ; 24 × 8 TiB > 128 TiB adressables. Dès que seize bases
  coexistent, la suivante échoue — au hasard, selon l'ordonnancement. La
  passe de ce matin (206/206) a eu de la chance.
- Ce qui réserve : le gestionnaire de tampons de kuzu `mmap`e d'un bloc une
  région de `max_db_size` (`vm_region.cpp:39`, `MAP_NORESERVE` — de
  l'espace d'adressage, pas de la RAM) pour placer ses pages à adresses
  fixes ; défaut `DEFAULT_VM_REGION_MAX_SIZE = 2^43` sur Linux 64 bits
  (`constants.h:62`), 256 Go ailleurs, 1 Go en wasm. Contraintes : puissance
  de deux, au moins deux groupes de pages.
- Correctif, **dans la bibliothèque** : une base en mémoire ne peut pas
  dépasser la RAM, donc `Rag3dbConnection::in_memory()` réserve désormais
  **1 TiB** (`IN_MEMORY_MAX_DB_SIZE`), pas 8 — 24 bases parallèles font
  24 TiB. Les bases sur disque gardent les 8 TiB de kuzu ;
  `RAG3DB_MAX_DB_SIZE` prime toujours. `run_e2e.sh` ne force rien : c'est le
  défaut qui est testé. Relancées : `e2e_idempotent_registration` **22/22**,
  `e2e_search` **38/38**.
- Autre trouvaille : `set -euo pipefail` arrête le script à la première suite
  en échec — onze suites n'avaient pas tourné et le résumé affichait
  « TOTAL 89 passed, 4 FAILED » comme s'il était complet. Non corrigé ce
  soir ; à savoir en lisant un résumé.

Décompte final, 28 suites, **234 tests, 0 échec** :

| suite | | suite | |
|---|---|---|---|
| agent_loop | 4 | phase0b | 14 |
| batch_observe | 2 | profile_overhead | 4 |
| burn_embedder | 4 | rerank | 3 |
| burn_minilm | 3 | result_mode | 10 |
| burn_multilingual_minilm | 5 | scope | 9 |
| burn_reranker | 5 | search | 38 |
| burn_xlmr_reranker | 8 | search_queue | 5 |
| checkpoint | 3 | simple_entity | 15 |
| dataflow_observe | 7 | symbol_search | 12 |
| drain_unified | 6 | undo | 4 |
| generic_search | 12 | native | 11 |
| graph_tool | 4 | highlight_long_text | 8 |
| idempotent_registration | 22 | burn_ocr | 4 |
| burn_llm | 10 | burn_agent | 2 |

`burn_agent`, `burn_llm`, `burn_ocr` étaient à **0 test exécuté** : leurs
features `burn-llm` / `burn-ocr` n'étaient pas dans le jeu des E2E
(`rag3db-native,burn-embedder`), et le commentaire de `e2e_burn_agent.rs`
renvoyait à un `--features` que le script n'acceptait pas. Une suite qui ne
tourne pas n'existe pas — et le jeu E2E est aussi l'inventaire de l'arsenal.
Les deux features sont désormais dans le jeu par défaut (`--features a,b`
pour en ajouter), et les trois suites tournent : OCR 4/4 en 2,6 s, LLM 10/10
en 130 s, agent 2/2 en 48 s. La passe complète gagne trois minutes.
