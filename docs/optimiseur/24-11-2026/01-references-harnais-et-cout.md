# Références : harnais d'agents de code et coût

24 novembre 2026. Rassemblé pour l'étude du harnais de rag3weaver. Rien n'a été exécuté dans les dépôts
clonés : ils ont été lus avec grep, sed et cat. Les PDF ont été lus par `pdftotext`.

Convention : **lu** veut dire que l'affirmation vient du code ou de l'article, avec le fichier ou la section
cités. **Déduit** veut dire que c'est notre interprétation. Les chiffres des articles sont ceux des auteurs,
mesurés sur leurs bancs et leurs modèles, et rien n'a été reproduit ici.

Racine : `~/git_workspaces/G/24-11-2026-references/`

## 1. Dépôts

| Projet | Adresse vérifiée | Commit cloné (date) | Chemin local |
|---|---|---|---|
| Goose | https://github.com/aaif-goose/goose (`block/goose` y redirige en 301) | `03a343f9fe309a2c2cfe63a5561cbfe9ab38762e` (2026-11-24) | `goose/` |
| Hermes Agent | https://github.com/NousResearch/hermes-agent | `1b39ecabee1bd3f501c7ebf12ec7a19a84a7676a` (2026-11-24) | `hermes-agent/` |
| Pi (pi-mono) | https://github.com/earendil-works/pi (`badlogic/pi-mono` y redirige) | `d4760131efe0740b9b7f7ad1f126734b96a011c0` (2026-11-24) | `pi-mono/` |
| Command Code | https://github.com/CommandCodeAI/command-code | `941ee30174a7f33b0dff30bcb2e47d75a745735e` (2026-11-18) | `command-code/` |
| GenericAgent | https://github.com/lsdefine/GenericAgent (c'est la réf. [18] de Claw-SWE-Bench, arXiv 2604.17091) | `a0521ab4f92b4f3b57d5b1f070ff4296c9062e03` (2026-11-23) | `GenericAgent/` |
| Aider | https://github.com/Aider-AI/aider | `5dc9490bb35f9729ef2c95d00a19ccd30c26339c` (2026-05-22) | `aider/` |
| gemini-cli (déjà là) | https://github.com/google-gemini/gemini-cli | `03862c4164844c6cf8063b4cc48546ec1609cf2e` | `~/git_workspaces/c7/24-11-2026-gemini-cli` |
| opencode (déjà là) | https://github.com/anomalyco/opencode | `7f90d8d20176295a4943dbd42d4d68c4688a71fe` | `~/git_workspaces/c7/20-11-2026-opencode` |

Tous les clones sont en `--depth 1`, sans sous-modules.

**Command Code n'est pas open source.** Le dépôt public ne contient que le README, des logos, des modèles
d'issues et un registre de « mods » (`mods/registry.json`). Il n'a ni code ni licence (l'API GitHub renvoie
`license: null`). Le clone est gardé pour le README seul ; il n'y a rien à y lire sur le coût.

### 1.1 Goose (Rust)

La logique vit dans `crates/goose` et dans les crates `crates/oi-kit-*`.

- **Contexte**
  - `crates/goose/src/agents/state_machine/inference_preparation.rs`, `prepare()` : construit le prompt
    système, filtre les outils, puis ajoute un message « turn context » (heure, répertoire, budget)
    seulement s'il diffère du précédent.
  - `crates/goose/src/agents/reply_parts.rs` : seuls les `agent_visible_messages()` partent au modèle.
    Un message compacté reste stocké mais devient invisible.
  - `crates/goose/src/token_counter.rs` : comptage tiktoken `o200k_base`, avec un cache LRU de 1 024
    entrées.
- **Compaction**
  - Seuil : `DEFAULT_COMPACTION_THRESHOLD = 0.8` (`crates/oi-kit-context-management/src/lib.rs`) et
    `DEFAULT_AUTO_COMPACT_TOKEN_LIMIT = 225_000` (`crates/goose/src/context_mgmt/mod.rs`). C'est le plus
    petit des deux qui s'applique. Le code justifie la limite par le palier de prix d'OpenAI au-delà de
    272k.
  - Opération : `crates/oi-kit-agent/src/compaction/mod.rs`. Le déclenchement additionne l'usage rapporté
    par le fournisseur et une estimation des résultats d'outils pas encore rapportés.
  - Ce qui reste visible (`compaction/layout.rs`) : le résumé, un texte de continuation, le dernier prompt
    utilisateur rejoué et le turn context.
  - Le résumé est demandé en JSON structuré (`crates/oi-kit-context-management/src/prompts/compaction.md`).
    Si le résumé lui-même déborde, `summarize.rs` retire des réponses d'outils en partant du milieu, par
    paliers `[0, 10, 20, 50, 100]` %.
  - Le résumé paire par paire des appels d'outils (`state_machine/ops_tool_pair_compaction.rs`) existe
    mais est désactivé par défaut (`GOOSE_TOOL_PAIR_SUMMARIZATION`).
- **Sorties d'outils**
  - Shell, dans `crates/goose/src/agents/platform_extensions/developer/shell.rs` (`truncate_output`) :
    au-delà de 2000 lignes ou 50 000 octets, le modèle ne voit que les 50 dernières lignes (10 000 octets
    au plus). La sortie complète va dans un fichier temporaire, et une notice invite à la lire par
    `head`, `tail` ou `sed`.
  - Toute réponse MCP de plus de 200 000 caractères est écrite dans un fichier, et seul son chemin est
    renvoyé (`crates/goose/src/agents/large_response_handler.rs`).
- **Modèles**
  - Pas de lead/worker dans le code lu.
  - `crates/goose/src/providers/toolshim.rs` : pour un modèle sans appel d'outils natif, un petit modèle
    interprète extrait les appels du texte (`mistral-nemo` par défaut sous Ollama).
  - Les sous-agents peuvent prendre un autre modèle (`GOOSE_SUBAGENT_MODEL`, dans
    `platform_extensions/summon.rs`). Ils ne renvoient qu'un `{summary}`.
  - `state_machine/ops_auto_effort.rs` : un classifieur choisit l'effort de raisonnement par tour.
    Désactivé par défaut.
- **Parallèle** : `crates/oi-kit-agent/src/mcp/batch.rs`, `ToolCallBatch::run`, exécute en parallèle les
  appels d'un même tour. Au plus 5 sous-agents concurrents par défaut.
- **Cache**
  - `crates/oi-kit-provider-types/src/cache_semantics.rs` classe chaque fournisseur selon son type de
    cache.
  - `formats/anthropic.rs` pose `cache_control` sur le système, sur le dernier outil et sur les deux
    derniers messages utilisateur. TTL de 5 min, ou 1 h avec `GOOSE_CACHE_TTL`.
  - OpenAI reçoit `prompt_cache_key` égal à l'identifiant de session (`crates/oi-kit-providers/src/openai.rs`).
  - Les outils et les extensions sont triés par nom pour garder le préfixe stable
    (`agents/prompt_manager.rs:114`, `reply_parts.rs:210`).
- **Autres économies**
  - Outils différés (`state_machine/ops_toolcalling.rs`, `GOOSE_TOOL_DEFERRAL=lookup`) : le prompt ne
    liste que les noms des outils MCP, et un schéma n'arrive qu'après `enable_tools`.
  - « Code mode » (`platform_extensions/code_execution.rs`) : les outils passent par `execute_javascript`.
  - `state_machine/ops_context_relevance.rs` juge si d'anciennes sorties restent utiles, mais en ombre
    seulement : la requête n'est pas modifiée.

### 1.2 Hermes Agent (Python)

C'est le dépôt le plus riche des six sur le coût.

- **Contexte**
  - `agent/turn_request_assembly.py`, `assemble_api_request()` puis `plan_request_cache()`, canonicalise
    le préfixe pour qu'il reste identique octet par octet, et pose le plan de cache en dernier.
  - Le prompt système (`agent/system_prompt.py`) est construit une fois par session, en trois blocs :
    stable, contexte, volatile. Seule une compaction le reconstruit.
  - Le comptage de pré-vol est une estimation à 4 caractères par token (`agent/model_metadata.py`). Le
    chiffre réel vient de `usage.prompt_tokens`.
- **Compaction**
  - Classe `ContextCompressor` (`agent/context_compressor.py`), enfichable par `agent/context_engine.py`.
  - Défauts (`hermes_cli/config_defaults.py`) : `threshold 0.50`, `target_ratio 0.20`,
    `protect_last_n 20`, `micro_compact False`.
  - La queue gardée vaut 2,5 % de la fenêtre, bornée entre 10k et 25k tokens
    (`agent/context_compressor_tail.py`).
  - Le résumé fait 20 % de ce qu'il remplace, entre 2k et 10k tokens. Il reprend verbatim les messages
    utilisateur (24 000 caractères au plus) et un index d'ancres.
  - Le résumé est fait par un modèle auxiliaire (`agent/auxiliary_client.py`, tâche `compression`), avec
    repli sur le modèle principal.
  - Avant de résumer, les anciennes sorties d'outils sont remplacées par
    « [Old tool output cleared to save context space] » (`_prune_old_tool_results`).
  - `agent/micro_compaction.py` (résumé roulant) est désactivé par défaut parce qu'il casse le cache.
- **Sorties d'outils**
  - Terminal : 50 000 octets et 2000 lignes au plus (`tools/tool_output_limits.py`), coupés 40 % en tête
    et 60 % en queue (`tools/tool_output_truncate.py`).
  - `read_file` rend 100 000 caractères au plus, avec un `next_offset` pour continuer. Une relecture d'un
    fichier inchangé rend un talon « File unchanged since last read » (`tools/file_tools_read_tracking.py`).
  - Débordement sur disque (`tools/tool_result_storage.py`, `tools/budget_config.py`) : 100 000
    caractères par résultat, 200 000 par tour, ou 15 % et 30 % de la fenêtre.
- **Modèles**
  - Modèle auxiliaire configurable par tâche (`agent/auxiliary_task_config.py`).
  - Sous-agents (`tools/delegate_tool.py`) : contexte vierge, le parent ne voit que le résumé,
    profondeur 1, 10 enfants concurrents au plus.
  - Le « smart_model_routing » n'existe que comme clé de configuration, désactivée ; aucune logique ne
    l'utilise.
- **Parallèle**
  - `agent/tool_executor.py` fait tourner 8 workers.
  - `agent/tool_dispatch_helpers.py` liste les outils sûrs en parallèle (`read_file`, `search_files`…),
    ceux qui ne le sont jamais, et les conflits de chemin en écriture.
  - `tools/code_execution_tool.py` : un script appelle les outils par RPC, et seul son stdout revient au
    modèle.
- **Cache**
  - `agent/prompt_caching.py` pose 4 points `cache_control` : préfixe statique, fin du système, deux
    derniers messages.
  - `agent/prompt_cache_scope.py` garde un `prompt_cache_key` stable à travers la compaction.
  - `agent/prompt_cache_lifecycle/` (`PromptCacheWarmer`) maintient le cache vivant à 5/6 du TTL, puis
    compacte ou décharge quand il expire. Préfixe minimal de 1024 tokens.
  - Gemini : seul `cachedContentTokenCount` est lu (`agent/gemini_native_adapter.py`) ; aucun cache n'est
    créé.
- **Autres économies**
  - `tools/tool_search.py` : les outils MCP sont différés derrière `tool_search`, `tool_describe` et
    `tool_call`.
  - Skills chargés à la demande (`tools/skills_tool.py`). Ceux ajoutés en cours de session arrivent en
    diff dans le message du tour, pas dans le système (`agent/skills_index_delta.py`).
  - La mémoire est figée dans le prompt au démarrage (`tools/memory_tool.py`).
  - Les AGENTS.md des sous-dossiers sont injectés dans le résultat d'outil, pas dans le système
    (`agent/subdirectory_hints.py`).

Déduit : le fil commun de Hermes est de ne jamais toucher au préfixe en cours de session. Il compacte peu
et tard, et protège le cache.

### 1.3 Pi (pi-mono, TypeScript)

- **Contexte et compaction** : `packages/coding-agent/src/core/compaction/compaction.ts`.
  - Comptage : l'usage réel de la dernière réponse, plus 4 caractères par token pour ce qui a suivi.
  - Déclencheur : `contextTokens > contextWindow - reserveTokens`, avec
    `DEFAULT_COMPACTION_SETTINGS = { reserveTokens: 16384, keepRecentTokens: 20000 }`. La coupe tombe sur
    une frontière de tour.
  - Prompts : `SUMMARIZATION_PROMPT` (Goal / Constraints / Progress / Key Decisions / Next Steps /
    Critical Context) et `UPDATE_SUMMARIZATION_PROMPT`, qui met à jour un résumé précédent.
  - Avant le résumé, chaque résultat d'outil est tronqué à 2000 caractères (`compaction/utils.ts`).
- **Sorties d'outils** (`core/tools/truncate.ts`) : 2000 lignes et 50 Ko au plus. read, grep et find
  gardent la tête. bash garde la queue, et la sortie complète part dans un fichier
  (`tools/output-accumulator.ts`).
- **Modèles** : changement manuel seulement (`AgentSession.setModel`, `cycleModel`). Pas de modèle faible
  dédié.
- **Parallèle** : `packages/agent/src/agent-loop.ts` exécute en parallèle par défaut (`Promise.all`). Un
  outil peut imposer le mode séquentiel.
- **Cache**
  - `packages/ai/src/api/anthropic-messages.ts` pose `cache_control` sur le système, sur le dernier
    message utilisateur et sur le dernier outil, avec un TTL d'1 h si `cacheRetention` vaut « long ».
  - Réchauffage (`packages/ai/src/cache-warming.ts`) : rafraîchissement à 90 % du TTL, et seulement si
    l'économie attendue dépasse `0.05` $. La probabilité de reprise après inactivité est fixée à `0.15`.
  - `core/system-prompt.ts`, `diffSystemPromptSections` : seules les sections modifiées du système
    sont renvoyées.
- **Sobriété** : quatre outils par défaut (read, bash, edit, write), un préambule d'une phrase et des
  descriptions d'outils d'une ligne (`buildSystemPromptSections`).

### 1.4 GenericAgent (Python)

- **Contexte** : il n'y a pas de tokenizer, tout se compte en caractères. `trim_messages_history`
  (`llmcore.py`) fixe le plafond à `context_win * 3` caractères, avec `default_context_win = 38000`
  (80000 pour deepseek).
- **Compaction** : sans LLM.
  - `compress_history_tags(keep_recent=10, max_len=800)` tronque, en gardant début et fin, les
    `<thinking>`, `<tool_use>` et `<tool_result>` des anciens messages.
  - Au-delà du plafond, il compresse plus fort, puis supprime les tours anciens jusqu'à
    `cap * trim_keep_rate` (0.6 par défaut, 0.3 pour deepseek).
  - Le tour courant reçoit une « mémoire de travail » (`ga.py`, `_get_anchor_prompt`) : les 30 derniers
    résumés d'une ligne (80 caractères) et un `<key_info>`.
- **Sorties d'outils** (`smart_format`, `ga.py`) : début et fin. `code_run` est limité à 10 000
  caractères et `file_read` à 200 lignes. Le budget est divisé par le nombre d'appels du tour.
- **Modèles** : bascule vers une autre session en cas d'échec, puis retour après 300 s (`MixinSession`).
- **Parallèle** : non, les appels d'un tour sont traités l'un après l'autre (`agent_loop.py`).
- **Cache** : `cache_control` sur le dernier outil et sur les deux derniers messages utilisateur
  (`NativeClaudeSession.raw_ask`). Pas de réchauffage.
- **Densité**
  - Mémoire en couches L0 à L4 (`memory/`).
  - La mémoire globale n'est réinjectée que tous les 15 tours.
  - Le prompt système fait 881 octets (`assets/sys_prompt_en.txt`) et le schéma des outils 5,7 Ko
    (`assets/tools_schema.json`).

Déduit : la compaction mécanique et sans LLM de GenericAgent est cohérente avec son point le moins cher
dans Claw-SWE-Bench (section 2.5). Mais son taux de cache est plus bas que celui des autres harnais
(66,8 à 88,1 % contre plus de 90 %), et son taux de résolution aussi.

### 1.5 Aider (Python), la repo map

- **Repo map** (`aider/repomap.py`, `RepoMap`)
  - Les tags sont extraits par tree-sitter (`get_tags_raw`) et mis en cache sur disque
    (`.aider.tags.cache.v3`), avec invalidation par mtime.
  - Ils sont classés par `networkx.pagerank`, personnalisé par les fichiers du chat et par les
    identifiants mentionnés. Poids : ×10 pour un identifiant mentionné, ×50 pour un référent présent dans
    le chat, ×0,1 pour un nom en `_`.
  - `get_ranked_tags_map_uncached` cherche par dichotomie le nombre de tags qui tient dans le budget
    (`ok_err = 0.15`).
  - Budget : `map_tokens = 1024` par défaut. Il est multiplié quand aucun fichier n'est dans le chat :
    `--map-multiplier-no-files` vaut 2 dans `aider/args.py` (le constructeur de `RepoMap` a 8 en défaut,
    mais la CLI passe 2).
- **Résumé de l'historique** (`aider/history.py`, `ChatSummary`) : il est fait par le modèle faible
  d'abord. Budget : `min(max(max_input_tokens / 16, 1024), 8192)` (`aider/models.py`).
- **Modèles**
  - Modèle faible pour les messages de commit et les résumés.
  - Mode architecte (`coders/architect_coder.py`) : l'éditeur reçoit le plan sans historique et sans
    repo map (`map_tokens=0`).
- **Cache** : `coders/chat_chunks.py` ordonne system, examples, fichiers en lecture seule, repo map,
  historique, fichiers du chat, tour courant. Trois points de cache sont posés (`--cache-prompts`).
  `--cache-keepalive-pings` envoie un ping toutes les 295 s.
- **Sorties** : aucune troncature n'a été trouvée. `/run` demande s'il faut ajouter « {k}k tokens » de
  sortie au chat.

### 1.6 gemini-cli et opencode (déjà clonés : noms de fichiers seulement, pas relus ici)

- gemini-cli :
  - `packages/core/src/context/chatCompressionService.ts` et `contextCompressionService.ts` ;
  - `packages/core/src/context/processors/{historyTruncationProcessor,nodeTruncationProcessor,rollingSummaryProcessor}.ts` ;
  - `packages/core/src/context/truncation.ts` ;
  - `packages/core/src/routing/modelRouterService.ts` et `strategies/classifierStrategy.ts` (routage de
    modèle par classifieur).
- opencode :
  - `packages/core/src/session/compaction.ts` et `packages/core/src/config/plugin/compaction.ts` ;
  - `packages/ai/src/cache-policy.ts` et `packages/ai/src/protocols/utils/cache.ts`.

## 2. Articles

Les PDF sont dans `~/git_workspaces/G/24-11-2026-references/articles/<id>.pdf`.

### 2.1 The Scaffold Effect in Coding Agents: Harness Choice as a Hidden Variable in Coding-Agent Evaluation

- https://arxiv.org/abs/2607.22585, PDF : `articles/2607.22585.pdf`
- **Ce qu'il mesure** : 300 essais, soit deux modèles (Qwen 3.6 Plus, MiniMax M2.5) sur 50 tâches
  Terminal-Bench Pro, dans trois harnais (Goose, OpenCode, OpenHands-SDK).
- **Ce qu'il trouve (lu)** :
  - jusqu'à 40× d'écart en tokens par tâche résolue (Goose ≪ OpenHands-SDK < OpenCode), pour 0 à 8 points
    de taux de réussite ;
  - OpenCode fait environ 1,2× plus de tours que Goose, mais 10× plus de tours « sans action » ;
  - les auteurs attribuent l'écart à la croissance du contexte par tour plutôt qu'au nombre de tours.
- **Pour nous (déduit)** : un tour qui ne fait rien coûte tout le contexte relu. Le compteur à suivre est
  le nombre de tours sans action.

### 2.2 System Optimizations for Cost-Efficient Agent Harnesses (« Agency »)

- https://arxiv.org/abs/2611.12122, PDF : `articles/2611.12122.pdf`
- **Ce qu'il mesure** : Claude Code, Codex et un harnais basique, sur 100 tâches de code (SWE-Bench,
  SWE-Bench Pro, Terminal-Bench 2) et 100 tâches de recherche documentaire (OfficeQA).
- **Constats (lu, section 3)** :
  - relire le contexte existant fait jusqu'à 73 % du coût, cache compris ;
  - l'agent n'utilise en moyenne que 27 % des sorties d'outils ;
  - jusqu'à 89 % des tours partent à fouiller le système de fichiers sur les tâches de recherche ;
  - les traces contiennent 1,3 à 2,7 sous-tâches indépendantes.
- **Techniques et gains (lu, sections 4 et 5.3, isolées, harnais basique et GPT-6-Luna)** :
  - compaction des sorties d'outils sans perte : 14 % de tokens en moins par compaction, −3 % de coût.
    Elle comprend des « pointeurs de lignes » vers les sorties antérieures identiques, qui ne laissent
    que le diff dans les boucles lire-éditer-exécuter.
  - ramasse-miettes du contexte par arbres de décision : −47 % de tokens par compaction, −12 % de coût.
  - « pile de sous-tâches », qui résume chaque groupe de 6 segments en ne gardant que les appels, 300
    caractères des éditions et 5 lignes des tests : −8 % de coût.
  - recherche par métadonnées SQLite du système de fichiers : −32 % de coût, mais sur OfficeQA, car elle
    sert rarement sur le code.
  - consigne de grouper les appels d'outils (annexe C) : −19 % d'étapes, mais +2 % de coût avec Luna.
  - au total : jusqu'à −35 %.
- **Règle centrale (lu, équation 2)** : une compression ne rapporte que si
  `p_read · N · Δ` dépasse le coût de réécriture du cache plus le coût des tours de re-lecture qu'elle
  provoque. Elle doit donc arriver tôt, avant que le contexte n'entre en cache, et ne jamais modifier un
  préfixe déjà caché.

### 2.3 TokenEconomizer: Experience-Driven Black-Box Cost Control for Coding Agents

- https://arxiv.org/abs/2610.22732, PDF : `articles/2610.22732.pdf` ; code :
  https://github.com/microsoft/TokenEconomizer (non cloné).
- **Ce qu'il mesure** : un contrôleur externe à Claude Code, Codex et Copilot CLI, sur des suites de
  tâches dans un même dépôt.
- **Constats (lu, sur 60 tâches SWE-Bench)** :
  - 23 schémas d'outils sont envoyés à chaque tour, alors qu'aucun dépôt n'en utilise plus de 7 ;
    16 ne sont jamais appelés et 3 font 98 % des appels ;
  - les échecs font 17 % des tâches mais 32 % de la dépense ;
  - les actions se répètent d'une tâche à l'autre dans un même dépôt (30 % après 5 tâches, contre 7 %
    avec l'historique d'un autre dépôt) ;
  - les stratégies fixes (RTK, Serena, repo map d'Aider) ne baissent pas le coût de façon régulière.
- **Résultat (lu)** : −18,4 à −30,9 % de coût à succès comparable, en combinant micro-skills appris,
  élagage des outils et arrêt des boucles d'échec. En ablation sous Claude Code, l'élagage des outils
  pèse le plus (−13,2 % et −17,7 %), puis les micro-skills (−11,8 % et −14,3 %).

### 2.4 An Empirical Study of Harness Design for Coding Agents

- https://arxiv.org/abs/2609.20804, PDF : `articles/2609.20804.pdf`
- **Ce qu'il mesure** : 176 réglages appariés, quatre modèles, SWE-Bench Verified et Terminal-Bench 2.1.
  Cinq stratégies de contexte (T0 à T4) sous des fenêtres de 32k à 128k, plus des ablations de la
  planification et de l'espace d'actions.
- **Ce qu'il trouve (lu)** :
  - la gestion de contexte compte d'autant plus que la fenêtre est petite, surtout parce qu'elle évite les
    débordements ;
  - T4 (élision des vieilles observations d'outils par règle, puis résumé LLM sélectif) a le coût agrégé
    le plus bas à succès voisin ;
  - rendre l'élision récupérable (`recall_event`) sert rarement et n'apporte rien ;
  - la planification réduit surtout le coût sur les modèles forts ;
  - un modèle qui maîtrise bash coûte moins cher avec bash seul qu'avec des outils prédéfinis.

### 2.5 Claw-SWE-Bench: A Benchmark for Evaluating OpenClaw-style Agent Harnesses on Coding Tasks

- https://arxiv.org/abs/2606.12344, PDF : `articles/2606.12344.pdf` ; données :
  https://github.com/opensquilla/claw-swe-bench
- **Ce qu'il mesure** : 350 tâches (8 langages, 43 dépôts). Sept harnais (OpenClaw, Hermes Agent, nanobot,
  ZeroClaw, GenericAgent, DeepSeek Harness, Meta-Harness) avec trois modèles, trois fois chacun, soit
  22 050 exécutions. La version Lite (80 tâches) coûte 25,3 % d'un passage complet.
- **Ce qu'il trouve (lu, table 2)** :
  - à modèle fixe, jusqu'à 24,3 points d'écart de Pass@1 ;
  - sous DeepSeek-V4.1-Flash, Hermes Agent a 81,7 % pour 36 $ (cache 96,9 %), et GenericAgent 71,5 %
    pour 12 $ (11,7 M tokens d'entrée hors cache, contre 41,8 M pour Hermes) ;
  - nanobot, avec un cache de 84,2 % et 184,9 M tokens d'entrée, coûte deux fois Hermes pour un résultat
    moindre ;
  - un adaptateur où le modèle écrit le diff à la main n'atteint que 19,3 % ; le même modèle passe à
    71,2 % quand il édite les fichiers et que le harnais exporte le patch.
- **Pour nous (déduit)** : le taux de cache et le volume d'entrée hors cache expliquent une grande part
  des écarts de coût entre harnais.

### 2.6 SWE-Pruner: Self-Adaptive Context Pruning for Coding Agents

- https://arxiv.org/abs/2601.16746, PDF : `articles/2601.16746.pdf` ; code :
  https://github.com/Ayanami1314/swe-pruner
- **Ce qu'il mesure** : un « écrémeur » de 0,6 B paramètres, placé en intergiciel, qui garde les lignes
  utiles d'une lecture de code selon un but formulé par l'agent (« focus on error handling »).
- **Ce qu'il trouve (lu)** :
  - les lectures (cat, grep, head) font 76,1 % des tokens sur SWE-Bench Verified avec Claude Sonnet 4.5 ;
  - l'élagage fait −23 à −38 % de tokens sur SWE-Bench, avec +1,2 à +1,4 point de succès, et −29 à
    −54 % sur SWE-QA ;
  - limite : évalué sur Python seulement.
- **Pour nous (déduit)** : c'est la référence la plus proche d'un agent qui relit beaucoup de code. À
  croiser avec 2.8, qui montre qu'un gain en tokens ne vaut pas un gain facturé.

### 2.7 Can your AI agent be cheaper? Investigating the effects of task specifications on token spend in agentic coding tasks

- https://arxiv.org/abs/2608.25399, PDF : `articles/2608.25399.pdf`
- **Ce qu'il mesure** : 2 700 exécutions avec Kimi K3 à trois niveaux d'effort, et douze formes de
  spécification de la même tâche.
- **Ce qu'il trouve (lu)** :
  - réduire une spécification complète à un simple récit utilisateur coûte +29,7 % de tokens et +16,4 %
    de tours ;
  - la variance d'une exécution à l'autre (×1,34) ne bouge pas, quelle que soit la spécification ;
  - la sensibilité au prompt dépend de la tâche, de 13 à 115 % ;
  - une sonde à 0,11 $ prédit la dépense d'une tâche inconnue à 36 % près, contre 161 % sans mesure.
- **Pour nous (déduit)** : une mission précise donnée à un agent est une économie directe. Une sonde bon
  marché permettrait de prévoir un budget avant de lancer un essaim.

### 2.8 Token Reduction Is Not Cost Reduction: An Empirical Study of End-to-End Efficiency in API-Based Coding Agents

- https://arxiv.org/abs/2607.12161 (aussi sur alphaxiv), PDF : `articles/2607.12161.pdf` ; artefacts :
  https://github.com/PointFiveLabs/ai-efficiency-benchmark. Les auteurs sont chez PointFive et ne
  développent pas les outils évalués.
- **Ce qu'il mesure** : 2 848 exécutions appariées de Claude Code, facturées par le fournisseur, avec et
  sans couche de réduction (RTK, Headroom…).
- **Ce qu'il trouve (lu)** :
  - le réglage le plus agressif retire 38,4 % des tokens de sortie d'outils mais augmente le coût facturé
    de 6,8 %, par des tours de diagnostic, de test et de re-lecture en plus ;
  - la facture se décompose en écriture de cache 44,3 %, lecture de cache 35,4 %, sortie 10,4 %, entrée
    hors cache 1,3 % et résidu non attribué 8,7 % ;
  - ce qu'une couche côté utilisateur peut toucher ne fait qu'environ 6 % du coût, dont environ 3,3 %
    pour les sorties d'outils ;
  - sur Codex, la même famille Headroom économise 12 à 14 % : l'effet dépend du couple couche, harnais,
    modèle et charge.
  - Recommandation : juger au coût facturé ajusté au succès, ne jamais transformer un flux lu par un
    programme, et préserver les preuves denses (traces d'erreur, tests).

### 2.9 Pages (adresses seulement)

- **Banc de Composio** : https://composio.dev/bench, et l'article
  https://composio.dev/bench/research/agent-harness-benchmark.
  - Environ 1 800 essais, neuf harnais (lu sur l'article).
  - À modèle fixe : 2,5× d'écart de coût sous DeepSeek V4 Pro, 13,1× sous GPT-6 Astra (de 1,20 $ à
    15,72 $ par tâche ; Claude Code au plus haut).
  - Les coûts sont des estimations, pas des factures (les auteurs le disent).
  - Composio vend les connecteurs qu'il compare.
- **OpenCode Zen et Go**
  - `opencode.ai/docs/zen/` redirige vers https://opencode.ai/docs/console/models/, et
    `opencode.ai/docs/go/` vers https://opencode.ai/docs/console/go/.
  - Zen : facturation à la requête, au prix par million de tokens, avec un prix « Cached Read » par
    modèle (par exemple Claude Sonnet 5.5 : 2,00 $ en entrée, 0,10 $ en lecture de cache).
  - Go : abonnement à 10 $ par mois, plafond compté en dollars par modèle. Un identifiant de session
    constant dans l'en-tête `x-opencode-session` aide le routage et le cache.
- **Cache de Gemini** : `ai.google.dev/gemini-api/docs/caching` redirige vers
  https://aistudio.google.com/docs/caching. Pour Vertex :
  https://docs.cloud.google.com/vertex-ai/generative-ai/docs/context-cache/context-cache-overview.
  - Le cache implicite est actif par défaut depuis Gemini 2.5. Il faut un préfixe d'au moins 4 096 tokens
    pour Gemini 3.x et 4, et 2 048 pour Gemini 2.5.
  - Le cache explicite n'existe pas pour Gemini 4.
  - En boucle d'agent sous Gemini 4, la requête précédente n'est reprise du cache que si la nouvelle
    ajoute au moins 2 050 tokens.
  - Le taux de remise n'est pas sur cette page : il est sur la page des prix.

## 3. Ce que nous devrions essayer en premier

1. **Mesurer la facture par poste avant d'optimiser.** Il s'agit de relever, par tour, l'écriture de
   cache, la lecture de cache, l'entrée hors cache et la sortie, puis le coût par tâche réussie. Source :
   2.8 (lu : environ 80 % de la facture en cache, et −38 % de tokens donnant +6,8 % de coût) ; 2.2
   (lu : la relecture fait jusqu'à 73 % du coût). Sans cette mesure, toute réduction de tokens peut
   coûter plus cher.
2. **Ne jamais réécrire un préfixe déjà en cache, et compacter à l'entrée.** Cela veut dire un prompt
   système figé par session, des outils triés, la clé de cache stable et les sorties réduites avant
   d'entrer dans le contexte. Source : 2.2, équation 2 et « Tool Output Compaction » (lu) ; code d'Hermes
   (`agent/turn_request_assembly.py`, `micro_compaction` désactivé parce qu'il casse le cache) et de
   Goose (`prompt_manager.rs:114`).
3. **Envoyer moins de schémas d'outils.** Il s'agit de différer les outils rares derrière une recherche,
   ou d'élaguer ceux qu'un dépôt n'appelle jamais. Source : 2.3 (lu : 16 outils sur 23 jamais appelés,
   et l'élagage seul donne −13 à −18 %) ; code d'Hermes (`tools/tool_search.py`) et de Goose
   (`GOOSE_TOOL_DEFERRAL`). Déduit : c'est notre cas, avec plusieurs serveurs MCP chargés à chaque
   session.
4. **Élider les vieilles sorties d'outils par règle avant tout résumé LLM.** Une sortie peut être
   remplacée par un talon une fois périmée, et une relecture d'un fichier inchangé peut rendre « inchangé ».
   Source : 2.4 (lu : T4 a le coût le plus bas, et l'élision récupérable ne sert à rien) ; Hermes
   (`_prune_old_tool_results`, `file_tools_read_tracking.py`) ; les pointeurs de lignes de 2.2.
5. **Remplacer les chaînes de recherche par une requête dans l'index.** Notre index de code est déjà une
   table de métadonnées interrogeable ; il faudrait mesurer combien de tours de grep et de cat il épargne.
   Source : 2.2 (lu : la recherche par métadonnées donne −32 % de coût, mais sur OfficeQA) ; 2.6 (lu :
   les lectures font 76 % des tokens) ; repo map d'Aider. Déduit et non prouvé sur du code : 2.3 trouve
   que la repo map d'Aider, appliquée telle quelle, ne baisse pas le coût régulièrement, et 2.8 que les
   effets dépendent du harnais. Il faut donc mesurer selon le point 1.
