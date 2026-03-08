# Doc 21 — Synthèse : Vision Rhai & extensibilité à travers les docs

Date : 8 mars 2026

## Objectif

Rassembler et comparer tout ce qui a été écrit sur Rhai, les scripts et l'extensibilité depuis le 3 mars. La vision a évolué significativement au fil des sessions.

---

## 1. Inventaire des docs

### Session du 3 mars (dossier `3-mars-2026-23h31/`)

| Doc | Titre | Contenu principal | Sections Rhai |
|-----|-------|-------------------|---------------|
| **05** | Design : Extensibilité SearchQueue via Rhai | **Le doc fondateur Rhai.** Compare 4 options (Déclaratif, Rhai, JS callbacks, WASM plugins) → choisit Rhai. Design complet : builtins par tiers (1-4), ScriptHook (OnResult/OnCompose), emit/then/run_parallel, sandbox, block_in_place. | Intégralité (~1160 lignes) |
| **06** | Phases d'implémentation : SearchQueue + Rhai | Roadmap 5 phases (0-4) pour SearchQueue + Rhai. Phase 2 = Rhai core (fire-and-forget), Phase 3 = Rhai avancé (then, run_parallel, priorités), Phase 4 = Polish + presets domaine + Tier 4 (call_http). | Phases 2-4 |
| **07** | Suggestions ouvertes | Cache intra-drain pour builtins Rhai, SearchQueueEvents (observabilité scripts), dry_run, test harness pour scripts, mock builtins, Script REPL. | Sections 1, 2, 5 |

### Session du 6 mars (dossier `6-mars-2026-00h01/`)

| Doc | Titre | Contenu principal | Sections Rhai |
|-----|-------|-------------------|---------------|
| **05** | Session SearchQueue prep | Préparation implémentation SearchQueue. | Mentions script processor |
| **08** | Search Queue architecture review | Revue architecture SearchQueue. | Mentions ScriptHook/plugin |
| **09** | Dataflow graph design | **Pivot architectural : SearchQueue → Dataflow graph.** Le flux déclaratif (nœuds + edges + topo sort) remplace le flux impératif (emit + callbacks). | Implications pour Rhai mentionnées |
| **10** | Phases d'implémentation Dataflow Graph | **Nouveau roadmap 5 phases** pour le Dataflow. Phase 5 = Rhai ScriptNode avec @input/@output annotations, ScriptDynamicNode (@dynamic), PortValue ↔ Rhai conversion. | Section Phase 5 |
| **11** | Phase 1 completion report | Rapport de complétion Phase 1 core. | Phase 5 listé comme futur |
| **13** | Dataflow Phase 2 observability | Observabilité (Tap, ExecutionReport). | Script recording planifié |
| **14** | Recap et direction | Résumé et direction post-Phase 2. | Rhai en Phase 5 |

### Session du 7-8 mars (dossier `7-mars-2026-08h35/`)

| Doc | Titre | Contenu principal | Sections Rhai |
|-----|-------|-------------------|---------------|
| **01** | État des lieux | Snapshot de l'état du framework. Phase 5 = ScriptNode comme futur. | Mention Phase 5 |
| **07** | Design NodeRegistry | Registry + suppression DynamicNode. Fondation pour Phase 5. | Scope explicitement exclu (Phase 5) |
| **19** | Design : Phase 5 — Rhai ScriptNode | **Design détaillé ScriptNode** dans le contexte Dataflow (post-pivot). 4 questions design : ports, source, undo, feature flag. Builtins MVP : query_cypher, log. Scope = migrations + transforms. | Intégralité |
| **20** | Réflexion : Extensibilité — Nœuds custom et limites de Rhai | **Élargissement du scope** : 3 niveaux (ScriptNode/HttpNode/ProcessNode). Analyse des limites de Rhai (pas de réseau, pas de FS). Problème Deserialize transversal. | Intégralité |

---

## 2. Évolution de la vision

### Phase 1 — SearchQueue + Rhai (3 mars, Docs 05-07)

**Contexte** : Le pipeline de recherche est une SearchQueue impérative. Les scripts Rhai s'y greffent comme des hooks.

**Modèle** :
- `on_result(result, query)` — hook appelé pour chaque résultat
- `emit(op)` — enqueue une opération downstream (SearchRelated, FetchRelated, etc.)
- `then(fn_name)` — callback quand l'op émise a fini
- `run_parallel(ops)` — exécuter N ops en parallèle

**Builtins par tiers** :
- Tier 1 : `emit()` — le minimum
- Tier 2 : `get_related()`, `count_related()`, `has_relation()`, `get_chunks()`, `query_cypher()` — lecture KB
- Tier 3 : `search_bm25()`, `search_vector()` — recherche directe
- Tier 4 : `call_http()`, `log()` — opt-in, réseau

**Sandbox** : additif (rien par défaut, on expose ce qu'on veut). `allowed_tiers` dans ScriptConfig.

**Qualités** :
- Design très complet et réfléchi (~1160 lignes)
- Builtins KB-aware (abstraient les tables internes)
- Test harness et DX pensés (mocks, dry_run, REPL)

### Phase 2 — Pivot Dataflow (6 mars, Docs 09-10)

**Changement majeur** : La SearchQueue est remplacée par un graph dataflow (DAG + topo sort). Le flux est déclaratif, pas impératif.

**Impact sur Rhai** :
- `emit()`/`then()`/`run_parallel()` n'ont plus de sens → remplacés par la topologie du graph
- `ScriptHook` (OnResult/OnCompose) disparaît → le script est un nœud comme un autre
- Les tiers de builtins sont simplifiés → "ce que le ServiceRegistry expose"

**Nouveau modèle (Doc 10 Phase 5)** :
- `ScriptNode` — un nœud normal (`impl Node`) dont l'`execute()` est un script Rhai
- `@input results: Results` / `@output filtered: Results` — annotations pour les ports
- `ScriptDynamicNode` (`@dynamic`) — peut émettre de nouveaux nœuds
- PortValue ↔ Rhai Dynamic — conversion bidirectionnelle

### Phase 3 — Design concret ScriptNode (8 mars, Doc 19)

**Contexte** : Phases 1-4 du Dataflow terminées (489 tests). Premier design concret de ScriptNode.

**Choix** :
- Ports fixes MVP : trigger(in)/result(out, Map)/done(out, Empty)
- Source : inline (`script='...'`) et fichier (`file='path.rhai'`)
- Undo : optionnel via `fn undo()` dans le script
- Feature flag : `rhai-script`
- Builtins MVP : `query_cypher`, `log`, `set_output`, `set_undo_context`

**Biais identifié** : le doc est orienté migrations. Les use cases search (filtre, reranking, custom compose) ne sont pas couverts car les ports fixes (trigger/result/done) ne permettent pas de passer des Results/Children entre nœuds.

### Phase 4 — Élargissement et limites (8 mars, Doc 20)

**Constat** : Rhai est insuffisant pour les vrais use cases d'extensibilité (LLM, APIs, connecteurs).

**Vision 3 niveaux** :
1. **ScriptNode (Rhai)** — transformations internes (filtre, normalisation, Cypher multi-step)
2. **HttpNode** — appels HTTP déclaratifs (REST, LLM, APIs)
3. **ProcessNode** — subprocess externe (Python, Node.js, OAuth complexe)

**Problèmes transversaux identifiés** :
- Les types search (`UnifiedResult`, `ChildSummary`, etc.) n'ont que `Serialize`, pas `Deserialize` → round-trip impossible
- Le checkpoint system est aussi impacté (stub pour non-Batch types)
- Les ports du ScriptNode doivent être configurables (pas seulement trigger/result/done)

---

## 3. Ce qui est resté constant

Malgré les pivots, certains principes n'ont jamais changé :

| Principe | Doc 05 (3 mars) | Doc 19/20 (8 mars) | Statut |
|----------|-----------------|---------------------|--------|
| **Sandbox additif** | On expose uniquement ce qu'on veut | Identique | Validé |
| **Rhai comme langage** | Choisi parmi 4 options | Toujours le choix, avec réserves (Q2 doc 20) | Validé |
| **Feature flag** | Non mentionné explicitement | `rhai-script` (~2MB) | Validé |
| **`block_in_place`** pour async | Doc 05 §9 | Doc 19 §6 | Validé |
| **Limites d'exécution** | max_operations=100K, call_levels=32, etc. | Identique | Validé |
| **query_cypher** comme builtin core | Tier 2 | MVP builtin | Validé |
| **log** comme builtin | Tier 4 (opt-in) → remonté | MVP builtin | Validé |

---

## 4. Ce qui a changé

| Concept | Doc 05 (3 mars) | Doc 19/20 (8 mars) | Pourquoi |
|---------|-----------------|---------------------|----------|
| **Architecture** | SearchQueue (impératif) | Dataflow graph (déclaratif) | Pivot du 6 mars, plus flexible |
| **Script = ?** | Hook sur une queue (OnResult, OnCompose) | Nœud dans un graph (impl Node) | Le script est un citoyen de première classe |
| **emit/then/run_parallel** | Pattern central | Disparu — remplacé par topologie | Le graph gère l'orchestration |
| **Tiers de builtins** | 4 tiers (1-4) avec allowed_tiers | Simplifié : "ce que le ServiceRegistry expose" | Le registry unifie l'accès aux services |
| **Builtins KB-aware** | get_related, count_related, has_relation, etc. | Hors scope MVP — query_cypher suffit | Simplification MVP, ajout futur |
| **call_http** | Tier 4 opt-in dans Rhai | Nœud HttpNode séparé | Séparation des responsabilités |
| **Ports** | Pas de concept de ports (hooks) | Ports typés, statiques ou configurables | Le graph impose les types |
| **Undo** | Pas dans le scope | Oui via fn undo() optionnel | Le framework migrations le demande |
| **ScriptDynamicNode** | emit() dans la queue | @dynamic flag → GraphEmitter | Le graph gère l'expansion dynamique |

---

## 5. Ce qui reste ouvert

### Du Doc 05 (non transposé)

Ces idées du design initial n'ont pas été reprises dans les docs récents mais restent potentiellement pertinentes :

- **Builtins KB-aware** (get_related, count_related, has_relation, resolve_source, kb_meta, get_chunks) — simplifieraient beaucoup les scripts search. Actuellement le script devrait faire du Cypher brut pour obtenir ces infos.
- **Presets domaine** (code_search_strategy, document_search_strategy) — templates prêts à l'emploi. L'infrastructure Mermaid + templates existe maintenant, les presets pourraient être des .mmd avec des ScriptNode intégrés.
- **Cache intra-exécution** (Doc 07 §1) — cache les résultats de builtins pendant une exécution de graph. Pas implémenté mais pertinent si les scripts font beaucoup de queries Cypher.
- **Test harness pour scripts** (Doc 07 §5) — mock builtins, test_script() function, assertions intégrées. Important pour la DX.
- **dry_run** (Doc 07 §2) — mode qui montre les ops sans exécuter. Pertinent pour les scripts de migration.
- **Script REPL** (Doc 07 §2.3) — hors scope mais utile long terme.

### Du Doc 20 (questions ouvertes)

- **Q1** : Scope Phase 5 — ScriptNode seul (A), +HttpNode (B), ou les trois (C) ?
- **Q2** : Rhai vs alternatives (Lua, QuickJS, Starlark) ?
- **Q3** : HttpNode auth (API key simple vs OAuth client_credentials) ?
- **Q4** : Batching/pagination (un appel vs loop) ?
- **Q5** : Gestion des credentials (variables d'env, vault) ?

### Transversal

- **Deserialize sur types search** — pré-requis pour les ports typés ET le checkpoint complet. Mécanique mais impacte ~10 types.
- **Ports configurables** — le ScriptNode doit accepter des ports déclarés via config Mermaid (`in_results='Results'`) pour s'insérer dans les pipelines search.
- **camelCase** — les types sérialisés utilisent `#[serde(rename_all = "camelCase")]`. Les scripts Rhai manipulent des clés camelCase (`r.startLine` pas `r.start_line`).

---

## 6. Index des docs (accès rapide)

### Docs principaux Rhai/extensibilité

| Doc | Chemin | Résumé en une ligne |
|-----|--------|---------------------|
| 05 (3 mars) | `3-mars-2026-23h31/05-design-extensibilite-rhai.md` | Design fondateur : choix de Rhai, builtins par tiers, ScriptHook, emit/then, sandbox |
| 06 (3 mars) | `3-mars-2026-23h31/06-phases-implementation.md` | Roadmap SearchQueue + Rhai en 5 phases (obsolète pour l'architecture, utile pour les idées) |
| 07 (3 mars) | `3-mars-2026-23h31/07-suggestions-ouvertes.md` | Cache, observabilité, dry_run, test harness, REPL |
| 10 (6 mars) | `6-mars-2026-00h01/10-implementation-phases.md` | Roadmap Dataflow en 5 phases — Phase 5 = ScriptNode avec @input/@output |
| 19 (8 mars) | `7-mars-2026-08h35/19-design-rhai-scriptnode.md` | Design concret ScriptNode Dataflow — ports fixes, inline/file, undo, feature flag |
| 20 (8 mars) | `7-mars-2026-08h35/20-reflexion-extensibilite-noeuds-custom.md` | 3 niveaux d'extensibilité (Rhai/Http/Process), limites, Deserialize, questions ouvertes |

### Docs architecturaux (contexte pour Rhai)

| Doc | Chemin | Pertinence |
|-----|--------|------------|
| 01 (7 mars) | `7-mars-2026-08h35/01-etat-des-lieux.md` | État du framework, Phase 5 en perspective |
| 07 (7 mars) | `7-mars-2026-08h35/07-design-node-registry.md` | NodeRegistry — fondation pour ScriptNodeFactory |
| 09 (6 mars) | `6-mars-2026-00h01/09-dataflow-graph-design.md` | Le pivot SearchQueue → Dataflow |
| 14 (6 mars) | `6-mars-2026-00h01/14-recap-et-direction.md` | Direction post-Phase 2 |

### Docs avec mentions secondaires de Rhai

| Doc | Chemin | Mention |
|-----|--------|---------|
| 05 (6 mars) | `6-mars-2026-00h01/05-session-search-queue-prep.md` | Script processor planning |
| 08 (6 mars) | `6-mars-2026-00h01/08-search-queue-architecture-review.md` | Plugin/ScriptHook concepts |
| 11 (6 mars) | `6-mars-2026-00h01/11-phase1-completion-report.md` | Phase 5 listé comme futur |
| 13 (6 mars) | `6-mars-2026-00h01/13-dataflow-phase2-observability.md` | Script recording planifié |
