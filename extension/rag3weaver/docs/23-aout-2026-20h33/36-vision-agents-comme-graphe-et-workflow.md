# Doc 36 — Vision : l'agent est un sous-graphe qui se compile en workflow, et des agents construisent des agents

Notée le 24 août 2026 au soir, à la demande de Lucie. Ce n'est pas un plan :
c'est la direction qui donne un sens à l'ordre des chantiers du doc 29. À
relire quand on se demande « pourquoi on fait ça ».

## 1. L'idée en une phrase

**La configuration d'un agent et son exécution sont le même objet** : un
sous-graphe de la base (ce que l'agent *est*) qui se compile en DAG dataflow
(ce que l'agent *fait*), et dont chaque exécution retombe dans la base (ce que
l'agent *a fait*). Le RAG, le graphe et le workflow ne sont pas trois produits
juxtaposés : ce sont les trois faces d'un seul.

## 2. L'agent comme sous-graphe

```
(:Agent {name, project, voice, language, version})
   -[:USES_PROMPT]->      (:Prompt {template, version})
   -[:RETRIEVES_FROM]->   (:KB {name, project})              ← nos KB, telles quelles
   -[:CAN_CALL]->         (:Tool {kind: http|code|mcp, mode: sync|async,
                                  timeout, schema, version})
   -[:LISTENS_WITH]->     (:Model {role: stt, weights: burnpack})
   -[:THINKS_WITH]->      (:Model {role: llm, ...})
   -[:SPEAKS_WITH]->      (:Model {role: tts, ...})
   -[:REMEMBERS_IN]->     (:Memory {policy})                  ← une KB aussi
   -[:GOVERNED_BY]->      (:Policy {max_turns, escalation, pii, budget})
   -[:DERIVED_FROM]->     (:Agent)                            ← lignée (voir §4)
```

Tout est une entité rag3weaver ordinaire : enregistrée par le `Catalog`,
indexée (BM25 exact pour les identifiants et les schémas d'outils, dense
pour « l'agent qui gère les réclamations »), filtrée par `project`. La
représentation textuelle de ce sous-graphe est le « YAML universel » qui
dort depuis mars — pas un format de plus, la *sérialisation* de l'agent.

## 3. L'agent comme workflow

Un `AgentCompiler` lit le sous-graphe et émet un `DataflowGraph` :

```
SttNode ─▶ TurnNode ─▶ SearchNode(KB) ─▶ PromptNode ─▶ LlmNode(stream)
                                                          │
                                        ┌─────────────────┴───────────┐
                                        ▼                             ▼
                                  RouterNode ─▶ ToolNode(s)      TtsNode(stream)
                                        │        sync | async         │
                                        └─────▶ MemoryWriteNode ◀─────┘
```

Chaque nœud est un acteur luciole ; les outils **async** sont naturels (un
`ToolNode` envoie, reçoit une `Reply` plus tard, pendant que le TTS parle
déjà la première phrase) ; les outils **code** sont des nœuds Rust du
`NodeRegistry` (ou du WASM chargé à chaud, plus tard) ; les outils **http**
un seul nœud paramétré par le schéma stocké dans le graphe ; les outils
**mcp** pareil, vers un serveur.

Ce que l'infra existante donne gratuitement :

| Déjà là | Ce que ça devient |
|---|---|
| checkpoints + `undo` | **la conversation est rembobinable** : rejouer un tour avec un autre prompt, comparer |
| `DataflowRecorder` JSONL, taps, `ExecutionReport` | **la trace est une donnée** : `(:Turn)-[:CALLED]->(:ToolCall {latency, status})`, cherchable avec le même moteur |
| KB + `project` | **la mémoire épisodique est une KB** ; ce que l'agent retient est indexé comme un document |
| enregistreur + replay | **l'éval est un replay** : N sessions enregistrées rejouées contre une nouvelle configuration, différences mesurées (le chantier « éval » du doc 29, avec un usage concret) |
| `Catalog` + DAG exposés | **le MCP tombe seul** : lire/modifier la configuration (graphe) *et* lancer un tour (DAG) |

## 4. Des agents qui construisent des agents

C'est le point que Lucie a ajouté, et c'est celui qui ferme la boucle.

Si les agents, leurs sessions, leurs traces et leurs évaluations sont dans le
graphe et l'index, alors **un agent qui construit un agent n'est qu'un agent
de plus**, dont les KB sont :

- le **catalogue des agents** (leurs sous-graphes, sérialisés) ;
- les **sessions** (transcriptions, appels d'outils, latences, échecs) ;
- les **évaluations** (quel prompt a mieux marché sur quelles sessions) ;
- la **documentation des outils** (schémas, exemples d'appels réussis).

Son travail : RAG sur tout ça, puis émettre un sous-graphe — c'est-à-dire
*écrire dans la base* via le même `Catalog`, avec `-[:DERIVED_FROM]->` vers
ses sources. Le nouvel agent est immédiatement compilable, exécutable,
évaluable, et sa lignée est une requête graphe (« quels agents descendent de
celui-ci, et lesquels font mieux que lui sur les sessions de juillet ? »).

Deux garde-fous qui sont *aussi* des données du graphe : `(:Policy {budget})`
borne ce qu'un agent constructeur peut dépenser, et un agent dérivé n'est
promu qu'après une évaluation rejouée — la promotion est une arête, pas un
déploiement.

## 5. Ce qui manque vraiment, dans l'ordre où ça bloque

1. **`project` sur tout** — un `(:Agent)` sans projet est la première chose
   qu'on devrait migrer. C'est le chantier 2 du doc 29 ; il vient avant
   parce que tout ce qui précède en dépend.
2. **Ports en streaming** — aujourd'hui un `PortValue` est une valeur ; il
   faut un port qui porte un flux (jetons LLM, morceaux audio). Une boîte aux
   lettres luciole fait exactement ça.
3. **Nœuds modèles** sur burn : STT, LLM, TTS (chantier 4 bis du doc 29),
   avec une interface de streaming substituable par un fournisseur cloud
   (ElevenLabs, Gradium).
4. **`ToolNode` async** avec délai, annulation, et `Reply` tardive.
5. **`AgentCompiler`** sous-graphe → `DataflowGraph`, et son inverse
   (sérialisation YAML).
6. **Éval par replay** — sans elle, « des agents qui construisent des agents »
   est une boucle sans juge.

Rien de tout ça ne se commence avant la fin des chantiers 2-4 bis du doc 29.
Mais c'est la raison de leur ordre.
