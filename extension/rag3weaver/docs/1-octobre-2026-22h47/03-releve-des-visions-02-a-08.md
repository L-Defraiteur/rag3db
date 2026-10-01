# Relevé — docs de vision 02 à 08 contre le code

**1er octobre 2026.** Relevé fait par un agent de repérage, en lecture seule, sur `mtg-experiments` (`3886a458e`). Annexe de [01 — la réconciliation des objectifs](01-reconciliation-des-objectifs.md). Chaque « absent » dit ce qui a été cherché. Ce relevé est une photographie : il n'est pas tenu à jour.

---

Rapport de réconciliation : docs de vision 02, 03, 04, 05, 07 et 08 confrontés au code au 1er octobre 2026

J'ai lu les six docs en entier. J'ai vérifié chaque élément par grep et par lecture dans `extension/rag3weaver/{src,tests,templates,Cargo.toml}`. La recherche de lecteurs xlsx, pdf ou docx dans `src/` (moteur C++) n'a rien trouvé. Branche `mtg-experiments`, HEAD `3886a458e`.

Sauf mention contraire, les chemins sont relatifs à `/home/lucied/git_workspaces/rag3db/extension/rag3weaver/`.

---

## Doc 02 — Les deux moitiés du moteur

**Thèse.** Le moteur a deux moitiés :
- le mode simple : des documents, une structure découverte ;
- le mode KB : des catalogues d'objets, une structure déclarée et agrégée depuis plusieurs entités.

La moitié déclarée existe. Il manque l'entrée, c'est-à-dire la porte d'un tableur. Elle doit prendre la forme d'un graphe-outil de normalisation qui synchronise par identifiant. Côté documents, il manque les lecteurs pdf, docx et pptx.

| élément | § | état | preuve |
|---|---|---|---|
| Socle déclaré : EntityConfig, register_entity/kb, Choice/Tags, HasAny/HasAll/HasNone/Between, RRF, rerank, org×project | 2 | fait | `src/config.rs` (FieldType, l.15), `src/search.rs` (FusionStrategy l.63). `KBUpdateNode` n'existe plus : les KB sont repliées en entités dérivées (`src/derived_kb.rs`). |
| title_boost / content_boost appliqués | 2 | absent, mais désormais signalé | `src/config.rs:285-292` : « Accepté mais non appliqué ». `src/catalog.rs:1705-1745` émet un `CatalogEvent::Warning` quand une valeur non par défaut est posée. |
| Remplacement par une topologie : une branche BM25 par champ, pesée à la fusion | 2 | partiel | `templates/weighted_search.mmd` : deux `BM25SearchNode(fields=…)` puis `FuseResultsNode(weights=…)`. Seul `src/dataflow/mermaid.rs:832` (un test de parsing) l'utilise. Ce n'est pas branché sur `register_kb`, et aucune éval du gain n'existe. |
| Identifiant unique auto-détecté ou déclaré | 3 | partiel | Déclaré seulement (`hashsafe`, `src/uuid.rs:hashsafe_uuid`). Pas d'auto-détection : `needs_confirmation`, `detect_id` et `unique_id` sont introuvables. |
| Champs flexibles (type + filtre + rôle) | 3 | fait (forme déclarée) | `SimpleFieldDef`, `FilterOp` |
| Plusieurs KB créées implicitement quand un champ les référence | 3 | fait | `src/derived_kb.rs` traduit `title_for`/`content_for` en entités dérivées (`schema::resolve_entity_kbs`) |
| Mode zéro configuration avec table d'inférence (choice < 20, tags si `\|`, price, date, images…) | 3 | absent | Aucun code d'inférence trouvé. Il n'existe pas de type `price`, `date` ni `images` : FieldType n'a que `Timestamp`. |
| Synchronisation par identifiant (inchangés / modifiés / nouveaux / disparus) | 3 | partiel | `FlushResult.unchanged` (`src/records.rs:150`, dont le commentaire cite « 93 inchangés… »). Suppression des disparus seulement pour le code (`src/code_tools.rs:1088,1143 reingest_file`). Rien de générique pour un catalogue. |
| Graphe-outil de normalisation de tableur (table brute → entités + relations) | 5 | absent | Recherche de xlsx, calamine, tableur, spreadsheet et classeur : aucun résultat dans `src/` ni dans `Cargo.toml` |
| Phases persistées, reprise qui saute ce qui est fait | 5 | partiel (générique) | Checkpoints du dataflow (`src/dataflow/checkpoint_store.rs`, `Catalog::drain_resume` à `src/catalog.rs:6499`). Rien de propre aux tableurs. |
| Deux représentations depuis une ligne (texte depuis les valeurs originales, payload depuis les normalisées) | 5 | absent | Pas de pipeline de normalisation |
| Avertissements lisibles et corrigeables par un agent | 5 | partiel | `FlushResult.warnings`, `UpdateStatus::Failed(cause)`, `EchecDeGroupe` (`src/records.rs`). `ValidationIssue{code,path,message}` (`src/harness.rs:55`) existe, mais pas d'illustration par mini-tableau. |
| Lecteurs pdf / docx / pptx | 6 | absent | Seule occurrence : `"pdf"` dans une liste d'extensions ignorées (`src/code.rs:84`) |
| OCR qui « couvre déjà les PDF scannés » | 6 | partiel | `OcrNode` prend une image (`src/dataflow/ocr_nodes.rs`). Aucune rastérisation de PDF : l'affirmation n'est pas étayée par le code. |
| File = conteneur, jamais chunké, unité de grep/read | 6 | fait | `src/code.rs:file_config` (aucun champ de contenu). Outils `templates/tools/grep.mmd` et `read.mmd`, `src/code_tools.rs:grep_files` (regex sur fichiers, pas lucivy) et `read_file`. |
| Taille de chunk dérivée de la limite du modèle | 6 | partiel | Les troncatures sont comptées et signalées (`Embedder::troncatures`, `src/embedder.rs:74`, `src/catalog.rs:signaler_les_troncatures` l.6253). Pas de dérivation : `src/code.rs:172` dit « À dériver … quand on saura la lire ». |

**Décisions de Lucie.** Aucune n'est citée mot pour mot. Le doc mentionne seulement que la distinction a été « retrouvée par un rappel oral ». Les deux « Correction du 5 septembre 2026 » ne lui sont pas attribuées.

---

## Doc 03 — Normaliser des tableurs

**Thèse.** Le doc tire les leçons d'un pipeline Python réel :
- quatre phases fixes et persistées ;
- le renversement d'échelle : ne montrer au modèle que des statistiques et des unités ;
- l'ordre statistiques → arbitrage du modèle → écrasement déterministe journalisé ;
- deux représentations par ligne ;
- des avertissements illustrés non bloquants au lieu de questions ;
- une identité de ligne à résoudre.

Il prépare le portage Rust.

| élément | § | état | preuve |
|---|---|---|---|
| Quatre phases séquentielles persistées (1a structurel … 4 matérialisation) | 1 | absent | Pas de pipeline tableur. Seuls existent les checkpoints génériques (`checkpoint_store.rs`). |
| Défusion des cellules, boîte englobante, marges avec décalage, lignes vides comme séparateurs | 1, 10 | absent | Aucun lecteur de classeur |
| Renversement d'échelle : parse en code, seuil de 70 %, n'envoyer que les unités distinctes | 2 | absent | Recherche de unité, canonical et unit : seuls `src/meter.rs` (compteur de consommation) et `src/events.rs` répondent, sans rapport |
| Statistiques par colonne injectées au prompt | 3 | absent | — |
| Garde-fous déterministes (500 distincts, ratio 0,5, plancher 50) qui écrasent le modèle, avec journal | 3 | absent | — |
| Deux représentations, valeur brute gardée à côté | 4 | absent | — |
| Plusieurs tableaux par feuille, en-tête répété, sections sans titre nommées | 5 | absent | — |
| Type date avec désambiguïsation jour/mois sur la colonne | 5, 11.3 | absent | FieldType a `Timestamp`, mais aucune reconnaissance de date |
| Lignes de total (test arithmétique) | 5, 11.2 | absent | — |
| Valeurs sentinelles | 5 | absent | — |
| Détection des doublons de lignes | 5 | absent | `src/hash.rs` fait une déduplication par contenu à la réingestion, pas une détection de lignes en double |
| Avertissements localisés et illustrés (mini-tableau, témoin), dédupliqués et plafonnés | 7 | absent sous cette forme | Il y a `ValidationIssue{code,path,message,params}` (`src/harness.rs`) et des `warnings` en chaînes |
| Flux de progression fin, avec instantané rejouable | 7 | partiel (générique) | Bus d'événements `NodeRun` et `RunStarted`/`RunFinished` (`src/events.rs`). Pas de progression « bloc n sur m » propre à l'ingestion. |
| « Le modèle propose, le code applique », test d'idempotence | 8.2 | non vérifiable | Pas de pipeline concerné |
| Identifiant de ligne porté par la donnée | 8.7 | partiel | `hashsafe` existe, mais rien pour une ligne sans clé |
| Sortie structurée validée par schéma | 10 | partiel | `ResponseFormat::JsonSchema{strict}` (`src/llm.rs:663`), crate `jsonschema`. Ni `schemars` ni `llguidance` dans `Cargo.toml`. |
| Troncature détectée sans recherche de sous-chaîne dans un message d'erreur | 8.5, 10 | fait (côté LLM) | `FinishReason::MaxTokens` (`src/llm.rs:141-145`) |
| Découpage adaptatif et réinjection du contexte entre morceaux | 10 | absent | — |
| Distinction cellule absente / chaîne vide / formule sans cache | 10 | absent | — |
| Mini-évaluateur arithmétique pour les formules de conversion | 10 | absent | `rhai` existe (`src/harness.rs`, `RhaiNode`), mais pour des règles métier |
| Journal de corrections réversible et inspectable | 11.5 | absent | — |
| Reprise d'import limitée aux feuilles modifiées | 11.6 | absent | — |
| Mesure du coût du ré-embedding intégral | 11.7 | absent | Des bancs d'ingestion existent (`tests/e2e_charge_ingestion.rs`, `e2e_mesure_ingestion_code.rs`), mais pas sur ce sujet |

**Décisions de Lucie.** Aucune n'est citée : le doc relate les leçons d'un autre projet. Une règle y est posée comme principe : « On ne demande à l'humain que ce qui est irrattrapable en aval et silencieusement destructeur. »

---

## Doc 04 — Le catalogue comme graphe

**Thèse.** « Tout ce que le système sait faire est une donnée dans le système. » Les entités NodeType, GraphTool, GraphVersion, Invocation et Tag vivent en base, et l'agent y cherche ses outils comme un document. Il compose seulement si rien ne convient et promeut sur preuve. Le reranker maintient l'ontologie des tags. Il n'y a aucun accès Cypher, pour rester portable entre backends.

| élément | § | état | preuve |
|---|---|---|---|
| `GraphDefinition::hash()` (BLAKE3) pour l'adressage par contenu | 2 | fait | `src/dataflow/checkpoint.rs:335`, exposé en `graph_hash` (`src/backend.rs:779,809`) |
| GraphNode : un nœud est un sous-graphe | 2 | fait | `src/dataflow/graph_node.rs` |
| to_mermaid / parse_mermaid_template | 2 | fait | `src/dataflow/mermaid.rs`, `GraphTool::from_mermaid`/`to_mermaid` (`src/dataflow/graph_tool.rs`) |
| Entité `NodeType` en base, projection à sens unique du registre | 3 | absent | `"NodeType"` : 0 occurrence. Les nœuds sont seulement décrits par `describe_backend` (`src/backend.rs:618`). |
| Entité `GraphTool` (name, description, params_schema, status draft/promoted/deprecated) | 3 | partiel | `GraphTool` est une struct en mémoire et `GraphToolRegistry::attach` existe (`graph_tool.rs:791,1592`). Les .mmd sont indexés comme fiches `Template` de famille `graph` (`src/template.rs`). Pas de status, pas d'entité « GraphTool ». |
| Entité `GraphVersion` immuable, `HAS_VERSION` / `CURRENT` | 3 | absent | 0 occurrence. `content_hash` existe sur la fiche Template, sans historique. |
| Entité `Invocation` et `INVOKED` | 3 | partiel (substitut) | L'entité `Trace` porte tool, call_id, ok, ms, run_id (`src/dataflow/trace_nodes.rs:trace_config`). Pas de lien vers une version exacte. |
| `USES_NODE_TYPE`, `CONTAINS`, `COMPOSED_BY` | 3 | absent | 0 occurrence |
| Promotion sur preuve (N succès, aucune erreur) | 3 | absent | Un `Lifecycle` draft→promoted n'existe que comme fixture de test (`src/config.rs:1579 fn promotion()`). Aucune requête de promotion. |
| `Tag` comme nœud de concept, avec mémoires et `TAGGED` / `RELATES_TO` | 4 | absent | `RELATES_TO` : 0. `"Tag"` n'apparaît que dans des tests de schéma (`tests/e2e_idempotent_registration.rs:1131`). |
| Reranker mainteneur d'ontologie, graphe-outil `resolve_tag` | 5 | absent | `resolve_tag` : 0 |
| Anti-quasi-doublon de graphes par rerank avant enregistrement | 5 | absent | `adopt` ne fait pas de rerank. `src/template.rs:455` liste seulement les voisins de même famille quand un nom est inconnu. |
| Boucle chercher → composer → Invocation → promotion | 6 | partiel | Chercher : `search(target='Template')` (testé, `tests/e2e_catalogue_gabarits.rs`). Composer : `validate_search`/`run_search` en Mermaid lecture seule (`src/backend.rs:597-620`). Ni Invocation ni promotion. |
| Décision 1 : un agent est un outil (une ligne de GraphTool) | 7.1 | absent | Pas de nœud ou d'outil qui enveloppe `Agent::run` |
| Décision 2 : deux espaces de tags, dont un temporel | 7.2 | absent | — |
| Décision 3 : aucun Cypher pour les agents, `NodeTypePolicy` exclut `CypherNode` | 7.3 | fait (backend) | `NodeTypePolicy::only(SEARCH_NODES)` pour les graphes composés par le modèle (`src/backend.rs:578-596,722,777,818,867`). `CypherNode` n'y figure pas. Ailleurs, l'agent et les graph tools tournent en `NodeTypePolicy::All` (`src/agent.rs:243`, `graph_tool.rs:1320`). |
| Drapeau `portable` sur NodeType | 8.4 | absent | — |
| Santé et fusion des tags | 9 | absent | — |
| Rerank par lots en `Priority::Idle` (luciole) | 9 | caduc | luciole a été retiré (doc 07 §6). Aucun lien vers luciole dans `Cargo.toml`. |

**Décisions de Lucie, mot pour mot.**
- Rappel en tête du doc : « un outil = un graphe entier + un spécificateur ».
- §4 : « Idée de Lucie » pour le Tag comme concept.
- §7, décisions du 25 août :
  - « **1. Un agent est un outil.** »
  - « **2. Deux espaces de tags, dont un temporel.** »
  - « **3. Aucun accès Cypher pour les agents — et ce n'est pas une question de sûreté.** » Avec cette citation : « Si une capacité manque par rapport à Cypher, on l'ajoute à l'abstraction — on ne régresse pas vers du Cypher qui nous enchaînerait à cette base pour toujours. »

---

## Doc 05 — Ce qui a tenu depuis février

**Thèse.** Le doc compare les visions de février à août. Le cœur déclaratif a tenu au mot près. Il distingue les dérives choisies des oublis : special_ops, persistance des opérations, getFilterOptions, routage multi-KB, enrichissement d'images. Il réclame six idées dormantes et acte que l'axe s'est retourné : le natif est devenu le projet, le navigateur l'horizon.

| élément | § | état | preuve |
|---|---|---|---|
| Consistency Immediate/Eventual/Strict | 2 | fait | `src/search.rs:23` |
| Rrf/Weighted, rrf_k = 60, flush_insertions, drain, SignalRole::Boost, Explore | 2 | fait | `src/search.rs:63,79,773`, `src/catalog.rs:5975,6331` |
| `special_ops` (grep/read) branché | 3, 4.2 | absent (champ mort, désormais signalé) | `src/config.rs:299-300`, avertissement dans `src/catalog.rs:1726` |
| grep et read-with-offset exposés aux agents | 4.2 | fait, hors special_ops et hors lucivy | `templates/tools/grep.mmd`, `read.mmd`, `src/code_tools.rs:grep_files` (crate regex), `read_file` |
| Persistance et inspection des opérations en échec, retry sélectif | 3 | partiel | Rejeu : `drain_resume`, checkpoints. Inspection : `EchecDeGroupe`, `UpdateStatus::Failed` (`src/records.rs`). Pas d'API qui liste et relance une opération en échec (`retry_operation` / `getFailedOperations` introuvables). |
| `getFilterOptions()` / facettes | 3 | absent | Recherche de facet et filter_options : 0 |
| Routage multi-KB par intention (`searchAll`, `QueryRouter`) | 3 | absent | 0 occurrence |
| Enrichissement automatique image → contenu (`analyze: true`) | 3 | absent | `OcrNode` existe, sans câblage déclaratif d'un champ image |
| Protocole d'identifiant unique en quatre étapes, `needs_confirmation` | 4.1 | absent | 0 occurrence |
| `max_size: auto` dérivé de `max_input_tokens` | 4.3 | partiel | Signalement seul (voir doc 02). Pas de `max_input_tokens()` sur le trait Embedder. |
| Rapport de validation à l'ingestion (bloquant / avertissement / suggestion) | 4.4 | partiel | `validate_schema` ne valide que la config (`src/validator.rs`). Règles métier rhai via `ValidationRuleNode`/`ValidationReport` (`src/dataflow/validation_nodes.rs`, `src/harness.rs`). Refus des transitions de lifecycle non déclarées (`src/catalog.rs:4499`). Pas de contrôle des valeurs `choice` non déclarées, pas de suggestion. |
| Boost par champ par branche FTS, avec le cas d'éval de février (2,10 contre 1,02) | 4.5 | partiel | Gabarit `weighted_search.mmd` présent. Le cas d'éval est introuvable (aucun parseJSON ni poids 2.5 dans tests). |
| Profondeur d'exploration par type de relation, preset ExploreStrategy | 4.6 | absent | Recherche de maxDepthPerRelation, ExploreStrategy et depth_per : 0 |
| Détection dynamique de conteneur (hasChildren) | 4.6 | non trouvé | Recherche de has_children, is_container et conteneur dans `src/code.rs` : 0 |
| Format « CSV enrichi » `#schema:` | 4.6 | absent | 0 occurrence |
| Retournement navigateur → natif acté | 7, 8 | fait (par doc 07) | wasm retiré : pas de `build_wasm.sh`, pas de feature wasm dans `Cargo.toml` |

**Décisions de Lucie.** Aucune n'est citée mot pour mot. Le doc cite les documents de février, par exemple : « On ne doit pas créer un système code-only. On doit **instancier** le système universel pour le code. »

---

## Doc 07 — Événements, runs et boucles

**Thèse.** Un bus à sujets, en fire-and-forget. Chaque run a une identité qui sert d'adresse (`run.<id>`, `run.<id>.inbox`). Chaque boucle réagit à sa frontière :
- le graphe, quand un nœud s'exécute ;
- l'agent, entre deux tours ;
- le réacteur, quand un événement arrive.

La trace est une entité cherchable et sert de mémoire de travail. Le doc est en grande partie un constat de livraison (nuit du 26 août).

| élément | § | état | preuve |
|---|---|---|---|
| Bus à sujets, curseurs nommés, emit/emit_on, tampon qui écarte le plus ancien | 1, 2 | fait | `src/events.rs` : `cursor` l.564, `emit` l.582, `emit_on` l.635 |
| run_id, `run.<id>`, parent, RunStarted/RunFinished | 1.3, 5 | fait | `src/events.rs:110,173,389 run_topic`, `src/dataflow/node.rs:195 run_id`, `src/dataflow/runtime.rs:827 execute_as`, `src/dataflow/services.rs:49 layered` |
| EventSourceNode, TraceSinkNode, entité Trace avec tous ses champs | 3.1, 4 | fait | `src/dataflow/trace_nodes.rs:trace_config` (run_id, parent_run_id, kind, agent, tool, call_id, node, ok, ms, tokens, at_ms, summary) |
| SendMessageNode, `send_message`, agent `with_inbox` qui lit entre deux tours | 3.1, 3.2 | fait | `trace_nodes.rs:992`, `events.rs:596`, `src/agent.rs:776,941 read_inbox`. Test : `tests/e2e_agent_loop.rs:534` |
| Schéma lié Run / Message / CHILD_OF / SENT_BY / SENT_TO | 5 | fait | `trace_nodes.rs:56-68,157 message_config` |
| Reactor (`%% on:` / `%% policy:`, each/batch/debounce, ReactorHandle) | 3.3 | fait | `src/dataflow/reactor.rs:44,123,329`, `templates/trace.mmd` (`%% on: agent, dataflow, messages`). Test : `e2e_agent_loop.rs:700` |
| Deux agents qui conversent par leurs boîtes, bornés | 3.2 | fait | `e2e_agent_loop.rs:765`. Voir aussi `tests/e2e_conversation_a_plusieurs.rs` et `events.rs:605 send_in`, `628 broadcast` |
| `interrupt` : un message urgent force `Flow::Stop` | 3.2, 5 | partiel | Seule l'annulation par l'utilisateur existe (`src/chat.rs:228-247`, drapeau `cancelled` → `Flow::Stop`) avec fermeture des orphelins (`INTERRUPTED_TOOL_RESULT`, `src/llm.rs:422`). Aucun message marqué urgent (recherche de urgent : 0). |
| Boîte durable (`MessageSourceNode`) | 5 | absent | `MessageSourceNode` : 0. Les `Message` sont écrits en base, mais aucun nœud ne lit une boîte durable. |
| Résultats bruts compacts (search, FetchRelatedNode) | 5, 6 | partiel | `RenderResultsNode` retire les nulls (`src/dataflow/render_nodes.rs:771`), gabarit `templates/render/compact.md.jinja`. `FetchRelatedNode` lui-même n'est pas modifié (`src/dataflow/search_nodes.rs:85`). |
| Wasm retiré | 6 | fait | Pas de `wasm_ffi.rs`, pas de `build_wasm.sh`, pas de feature wasm |
| Luciole retiré, runtime parallèle par niveau | 6 | fait | Aucune dépendance luciole. `lucivy-core` reste (`Cargo.toml:61`). |
| Tokio pour attendre | 6 | fait | `Cargo.toml:34-37` |
| Services `event_bus` et `events` distincts, `run_topic` par publieur | 6 | fait | `events.rs:389`, `EVENTS_SERVICE` (`dataflow/mod.rs:71`) |

**Décisions de Lucie, mot pour mot.** « **Wasm abandonné pour rag3weaver** (26 août, décision de Lucie) : ça ne servait qu'à se contraindre — pas de fils, pas d'async — pour un usage que personne n'avait ; lucivy garde le sien. »

Le retrait de luciole, le retour de tokio et le choix « deux clés pour deux rôles » figurent dans « ce qu'on a tranché », sans attribution nominative.

---

## Doc 08 — Des catalogues de gabarits

**Thèse.** Un catalogue de gabarits ferme la boucle du doc 01. Il couvre quatre familles : entités, graphes, composants React et motifs transversaux. Ces familles sont rangées selon deux axes, famille et catégorie, avec une palette « sous la main ». Quatre verbes s'y appliquent : chercher, poser, adopter sous un nom, modifier sans rien casser. Le critère de réussite : un backend debout, produit par un agent à partir de gabarits, avec un front, et la mesure de ce qu'il a dû écrire lui-même.

| élément | § | état | preuve |
|---|---|---|---|
| Fiches de gabarits en base, contenu sur disque | 2 | fait | `src/template.rs` (`TEMPLATE_ENTITY="Template"`, `template_config`, `scan`, `sync_templates` l.611) |
| Gabarits d'entités user / conversation / product | 2 | fait | `templates/entities/{user,conversation,product}.json` |
| Gabarits d'entités document / event | 2 | absent | Absents de `templates/entities/` |
| Gabarits de graphes indexés | 2 | fait | `Family::Graph`, répertoire `templates/tools/*.mmd` |
| Gabarits de composants React | 2 | absent | `Family::Component` déclaré (`src/template.rs:55`), mais pas de `templates/components/` ni de fichier React. `place` refuse les familles autres qu'`entity` (`src/dataflow/template_nodes.rs:88-97`). |
| Motif « versionné » | 2 | fait | `templates/patterns/versioned.json`, `Pattern::apply` (`src/template.rs:349-385`). Testé : `un_motif_ajoute_ses_champs_avant_l_enregistrement`. |
| Motifs effacé en douceur / audité / possédé / étiqueté | 2 | absent | Seul `versioned.json` existe. `Lifecycle` existe dans `config.rs:727`, mais pas sous forme de motif. |
| Agent versionné | 2 | absent | — |
| L'agent enregistre ses gabarits (adopt) | 2 | fait | `templates/tools/adopt.mmd`, `template_nodes.rs`. Test : `un_gabarit_adopte_se_repose` |
| Axes famille (fermée) et catégorie (ouverte), filtre par catégorie | 2 bis | fait | `enum Family`, champ `category`. Testé dans `tests/e2e_catalogue_gabarits.rs` |
| `WorkPalette` | 2 bis | absent | Recherche de palette : 0. Il existe `attach` et `allowed_tools` dans `templates/apps/notebook/chat.json`. |
| Verbe chercher | 3 | fait | `search(target='Template')`. Test : `un_agent_trouve_ses_gabarits_comme_il_trouve_un_document` |
| Verbe poser | 3 | fait (entités seulement) | `templates/tools/place.mmd`, `PlaceTemplateNode`. Test : `poser_un_gabarit_l_enregistre_vraiment` |
| Verbe adopter sous un nom | 3 | fait | Paramètre `as` de place/adopt, `GraphToolRegistry::attach` |
| Verbe modifier sans casser | 3 | partiel | Un gabarit de projet masque celui de la bibliothèque (test `un_gabarit_de_projet_masque_celui_de_la_bibliotheque`). Aucun test de modification après pose. |
| Lien après pose (`POSÉ_DEPUIS`) | 4 | partiel | Pas d'arête. Champ `derived_from` (empreinte d'origine) sur les gabarits adoptés (`src/template.rs:134`). |
| Signaler « ce gabarit a bougé depuis que tu l'as pris » | 4 | partiel | `TemplateSync` signale les bibliothèques qui ont bougé pour les gabarits adoptés (`template.rs:582`, `template_nodes.rs:375`). Pas de suivi des entités posées. |
| Frontière : RootPolicy / WorkDomain appliqués au catalogue | 4 | non vérifié | `RootPolicy` (`code_tools.rs:338`) et `WorkDomain` (`src/work_domain.rs`) existent. Je n'ai pas trouvé de lien explicite avec la lecture des gabarits. |
| Backend debout, produit par un agent, avec un front | 5 | partiel | Backends déclaratifs : `src/backend.rs`, `bin/rag3weaver-backend.rs`, `templates/backends/{notebook,validated-result}`. Chat TUI et web : `templates/apps/notebook`. Rien ne montre qu'un agent les ait produits. Le test `un_agent_monte_comme_en_production_trouve_pose_et_adopte` enchaîne des appels d'outils scriptés, sans LLM. |
| Mesure « combien a-t-il eu à écrire lui-même » | 5 | absent | Rien trouvé |

**Décisions de Lucie, mot pour mot.**
- « Pour la boucle étrange je pense qu'il faut des catalogues de templates aussi à chaque fois. Un backend, il faut justement pas qu'ils aient à tout coder, donc on leur prépare d'avance des `user`, `conversation`, `product`… Ils peuvent modifier les schémas mais ont une librairie de templates. Pareil pour le front : on leur pré-fait des templates de composants React qu'ils peuvent insérer où ils veulent, ils peuvent en enregistrer aussi, et bien sûr modifier le code une fois le template posé. »
- « Je pense que c'est le meilleur but car ça touche tout d'un coup, notre meilleure cible immédiate. »
- « la version c'est aussi un template intéressant de pattern, c'est utilisé souvent, exemple version d'un agent »
- « workpalette aussi, les templates que t'as sous la main »
- Hors citation : la note de `src/template.rs` dit « Décidé le 29 août 2026 » pour le choix « contenu sur disque, fiche en base ».

---

## Ce qui était voulu et reste entièrement absent

Classement par importance apparente dans les docs :

1. **Entrée par tableur.** Graphe-outil de normalisation xlsx (doc 02 §5, doc 03 en entier). Aucun lecteur de classeur, aucune phase 1a à 4, ni défusion, ni statistiques par colonne, ni garde-fous, ni double représentation.
2. **Inférence zéro configuration et détection de l'identifiant unique.** La table d'inférence et le protocole en quatre étapes avec `needs_confirmation` (doc 02 §3, doc 05 §4.1).
3. **Catalogue d'outils comme graphe en base.** Les entités `NodeType`, `GraphVersion`, `Invocation`, les relations `HAS_VERSION` / `CURRENT` / `USES_NODE_TYPE` / `CONTAINS` / `COMPOSED_BY`, et la promotion sur preuve (doc 04 §3).
4. **Ontologie de Tags.** Entité `Tag`, `TAGGED`, `RELATES_TO`, reranker mainteneur, `resolve_tag`, deux espaces dont un temporel, santé et fusion des tags (doc 04 §4, §5, §7.2, §9).
5. **« Un agent est un outil »** (décision de Lucie, doc 04 §7.1), ainsi que l'agent versionné (doc 08 §2).
6. **Gabarits de composants React** et **WorkPalette** (doc 08 §2 et §2 bis, demandés par Lucie).
7. **Mesure de « combien l'agent a dû écrire lui-même »** (doc 08 §5).
8. **Lecteurs pdf / docx / pptx** (doc 02 §6).
9. **Type date, lignes de total, sentinelles, doublons de lignes**, les « quatre vraies dettes » (doc 03 §5 et §9).
10. **Journal de corrections réversible**, reprise d'import par feuille modifiée, mesure du coût du ré-embedding (doc 03 §11).
11. **Facettes `getFilterOptions`**, **routage multi-KB par intention**, **enrichissement automatique image → contenu** (doc 05 §3).
12. **Profondeur d'exploration par type de relation**, preset `ExploreStrategy` (doc 05 §4.6).
13. **Boîte aux lettres durable** (`MessageSourceNode`) et **interruption par message urgent** (doc 07 §5).
14. **Motifs effacé en douceur / audité / possédé / étiqueté**, et gabarits d'entités `document` et `event` (doc 08 §2).
15. **Anti-quasi-doublon de graphes par rerank** avant enregistrement (doc 04 §5).
16. **Drapeau `portable` sur NodeType** (doc 04 §8.4), format « CSV enrichi » (doc 05 §4.6).
