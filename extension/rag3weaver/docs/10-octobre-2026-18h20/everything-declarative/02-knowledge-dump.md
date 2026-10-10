# Everything declarative (chantier I) — relevé de connaissances

Ce que la session sait du code et des dépendances, lu sur `origin/master` à
`51a92a4cd` le 10 octobre 2026. Chemins relatifs à `extension/rag3weaver/`.

## 1. Le script aujourd'hui (rhai)

- La logique n'est **pas** dans `src/dataflow/rhai_node.rs` : elle est dans
  `src/harness.rs`, `pub fn evaluate(script: &str, input: &Value, limits:
  &RhaiLimits) -> Result<Value, String>`. Trois appelants : `RhaiNode`,
  `ValidationRuleNode` (`validation_nodes.rs`), `AddRefNode`/`eprouver`
  (`ref_nodes.rs`).
- `RhaiLimits { operations, source_bytes, json_bytes, collection_items,
  timeout_ms }`, défauts 1 000 000 / 64 Kio / 16 Mio / 131 072 / 1 000 ms.
  Un `rhai::Engine` par appel, pas de précompilation. Exposé au script :
  `json_string`, `content_hash` (blake3). `eval`, `import`, `export`
  désactivés ; `print`/`debug` muets ; échéance par `on_progress`. Pas de
  plafond mémoire au sens propre (tailles et opérations).
- L'entrée du script est la constante `input` ; le résultat est la valeur de
  la dernière expression ; tout passe par `serde_json::Value`.
- `RhaiNode` : entrée `value`, sortie `result` (`PortType::Map`) ; script par
  la configuration (`script` en ligne, ou `script_id` cherché dans le service
  `"rhai_scripts"`, rempli par `backend.rs` depuis `manifest.scripts`, 64 Kio
  au plus). Le service `"rhai_limits"` n'est enregistré par personne : ce
  sont toujours les défauts.
- Tests : `harness.rs` (`scripts_are_pure_bounded_and_fail_closed`,
  `final_claim_is_not_acceptance_and_correction_can_complete`),
  `node_factories.rs` (`le_schema_declare_les_memes_ports_que_le_noeud` : une
  configuration minimale par type, obligatoire pour tout type nouveau),
  `validation_nodes.rs`, `scripts/test_backend_harness.py`.
- `rhai 1.26.1`, features `serde`, `sync`, `no_module`, non optionnelle.

## 2. Le moteur de graphes, ce qui compte pour les lots 2 et 3

- `trait Node: Send` synchrone (`node_type`, `inputs`, `outputs`, `execute(&mut
  NodeContext)`, `node_config`, `name`, annulation). `NodeContext` : `input`,
  `take_input`, `set_output`, journal, `service::<T>(clé)`, `run_id`.
- `PortValue::{Data(Arc<dyn Any + Send + Sync>), Trigger}` ; `PortType` est
  une énumération fermée (`Map`, `Any`, `Results`…), compatibilité par
  égalité ou `Any` ; **pas de schéma JSON par port**.
- `PortDef`, `NodeSchema`, `ConfigParam`, `Node::node_type()` portent des
  `&'static str` ; `GraphNodeFactory::templated` (`graph_node.rs`) fait
  `Box::leak` — un rechargement à chaud fuirait à chaque fois.
- `NodeRegistry` : `register`, `create`, `schema`, `types`, `has`, aucune
  désinscription. Les intégrés : `register_builtins` (`node_factories.rs`) et
  `BUILTIN_NODE_COUNT` à tenir à jour.
- `.mmd` : `id["TypeName(clé=val, value=$param)"]`, en-têtes `%% tool:`,
  `param:`, `on:`, `policy:`. Politique des types permis :
  `NodeTypePolicy::{All, only([...])}` (listes dans `backend.rs`,
  `backend_code.rs`) — un type de nœud scripté nouveau doit y entrer.
- `GraphNode` : un sous-graphe comme nœud, ports libres = interface, rebâti
  à chaque exécution ; `GraphNodeFactory::templated` est le modèle du « nœud
  déclaré sous un nom ».
- Runtime synchrone par niveaux topologiques (`std::thread::scope`), pas de
  tokio dedans ; événements `DataflowEvent` diffusés, `RunStarted` /
  `RunFinished` sur le bus `"event_bus"`, checkpoints. **`RunScope` n'existe
  pas** ; les services passent par `ServiceRegistry` (`layered`).
- `PreparedBackend::load` lit manifeste et `.mmd` une fois ; rien ne recharge.
- Réacteur : un fil avec un runtime tokio courant, une sonnette par sujet,
  `ReactPolicy::{Each, Batch, Debounce}` ; `backend.rs` charge les
  `reactions` mais ne les monte pas.

## 3. Binaires et HTTP

- Pas de clap : chaque binaire lit `std::env::args()`.
- `src/mcp.rs` **n'est pas sur master** : branche `memoire-longue-4`
  (`1396ad0c9`, « pas vert ») ; forme : premier argument = sous-commande
  (`mcp`) dans `rag3weaver-backend`.
- HTTP existant : `src/daemon/mod.rs` (feature `daemon`), `trait Service`,
  `servir` sur `tiny_http`. tokio n'a que `sync, rt, macros, time`. Aucune
  crate axum, notify, ni JavaScript.
- `[lints.rust] warnings = "deny"`.

## 4. Le moteur JavaScript (vérifié le 10 oct. dans les sources des crates)

- **rquickjs 0.14.0** (18 sept. 2026, MIT) embarque **quickjs-ng 0.16.2** ;
  le build ne compile que `quickjs.c`, `libregexp.c`, `libunicode.c`,
  `dtoa.c` (par `cc`, pas de libclang sans la feature `bindgen`) :
  `quickjs-libc` n'est jamais compilé, donc ni fichiers ni système.
  `Context::base` = objets de base seuls. `Runtime::set_interrupt_handler`
  appelé toutes les 10 000 opérations, exception non rattrapable.
  `set_memory_limit` **sans effet si la feature `rust-alloc` ou `allocator`
  est active**. Pile 256 Kio par défaut. JSON par `ctx.json_parse` /
  `json_stringify`.
- Écartés : boa 0.22 (ni arrêt par durée ni plafond mémoire, ~7× plus lent
  d'après un banc tiers, plusieurs Mo) ; deno_core/V8 (surdimensionné) ;
  javy, quickjs_runtime (une couche de plus sans gain).
- Retrait des types, retenu : **`swc_ts_fast_strip` 59.0.0** (6 oct. 2026,
  Apache-2.0), `operate(cm, handler, input, Options { mode, .. })`,
  `Mode::StripOnly` (défaut). Efface par plages sur le texte : chaque octet
  effacé devient un espace (sauts de ligne gardés, remplissage UTF-16 hors
  ASCII), `fix_asi` pour l'insertion des points-virgules ; mêmes lignes et
  mêmes colonnes, sans carte de source. Refusés avec un diagnostic précis :
  `enum` (sauf `declare enum`), `namespace` porteur de code, `module X {}`,
  propriétés de paramètre de constructeur, `<T>expr`. `import type` /
  `export type` effacés. C'est l'« erasableSyntaxOnly » de TypeScript 5.8.
  ~152 paquets au Cargo.lock (20 swc_*), dont les transformations du mode
  `Transform`, non optionnelles.
- Écarté : oxc 0.153 (pas de retrait seul ; parser → semantic → transformer
  → codegen régénère le code, positions perdues sans carte de source ;
  ~122 paquets).

## 5. luciepc

- Worktree `~/git_workspaces/rag3db-proto`, sous-modules initialisés,
  identité git `luciedefraiteur@gmail.com` vérifiée.
- Lib commune `~/git_workspaces/rag3db-lourd/build/lecteurs-csv/src/librag3db.so`
  bâtie à `f1b5e7407` (10 oct. 19 h 23) ; master porte depuis A3′ (19
  fichiers C++, en-têtes de `transaction/` et `storage/`) : elle est plus
  vieille que ses sources pour `run_e2e.sh`.
- Variables : `BUILD=~/git_workspaces/rag3db-lourd/build/lecteurs-csv`,
  `RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR=$BUILD/src RAG3DB_INCLUDE_DIR=$BUILD/src
  LD_LIBRARY_PATH=$BUILD/src RAG3DB_ROOT=~/git_workspaces/rag3db-lourd
  CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-I`.
