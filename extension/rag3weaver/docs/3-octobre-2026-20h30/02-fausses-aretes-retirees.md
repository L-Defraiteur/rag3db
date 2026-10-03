# Les fausses arêtes retirées — la preuve, classe par classe

3 octobre 2026. Ce que retire le lot des fausses arêtes de codeparsers
(`f0faa82`), mesuré contre la révision juste avant (`924a1d3`, résolution
déjà déterministe), sur le corpus d'`e2e_code` (`src/dataflow`, 38 fichiers),
avec les options de résolveur de rag3weaver. Une relation est comptée comme
rag3weaver la dédoublonne : (type, source, cible).

C'est cette liste qui autorise `e2e_code` et le banc à bouger : un seuil qui
bouge se justifie par des arêtes d'ici ; un résultat qui se dégrade sans
qu'une arête retirée l'explique bloque la fusion.

## Régénérer

```bash
cd extension/rag3weaver
scripts/aretes_retirees.sh 924a1d3 f0faa82 src/dataflow
```

Le script bâtit chaque révision dans un worktree détaché sous
`~/.cache/rag3weaver-build/aretes-retirees/`, avec son propre target, y passe
l'exemple `scripts/relations_tsv.rs`, puis classe avec
`scripts/classer_aretes_retirees.py`. La liste complète (une ligne par arête,
avec la ligne de source de son premier site) reste hors dépôt :
`~/.cache/rag3weaver-build/aretes-retirees/924a1d3-f0faa82.liste.tsv`.

Chaque cause est lue dans la source, à la ligne des sites de l'arête :
« variable locale » veut dire que le fichier lie ce nom par `let`, `for` ou
`:=` ; « mot de commentaire », que tous les sites tombent sur des lignes de
commentaire. Une arête qu'aucune cause n'explique serait classée « autre » :
il n'y en a aucune.

## Ce qui reste faux, et n'est pas dans ce lot

- Les paramètres de fermeture (`|s|`, `|f|`) et les motifs de `match` ne
  sont pas encore pris pour des locales : une partie des 73 héritages devenus
  `CONSUMES` vise encore un tel nom (`PostgresCheckpointStore → s`).
- Un appel qualifié par une variable dont on ne connaît pas le type
  (`node.with_delai(t)`) ne devient jamais une relation : c'est le trou le
  plus coûteux pour « tous les appels d'un symbole ».
- La ligne du premier site est parfois celle de la signature du scope : les
  références d'une méthode remontées à sa classe y sont posées.

## Le tableau et dix exemples par classe

relations d'usage distinctes retirées : 1552 ; ajoutées : 73
  CONSUMES : 7064 → 5666
  IMPLEMENTS : 182 → 101
  INHERITSFROM : 0 → 0

| Cause | Type | Nombre |
|---|---|---|
| variable locale (let, for, :=, fermetures comprises) | CONSUMES | 655 |
| mot de commentaire | CONSUMES | 578 |
| mot de commentaire (ligne décalée avant le correctif) | CONSUMES | 232 |
| héritage deviné, devenu CONSUMES | IMPLEMENTS | 73 |
| variable locale (let, for, :=, fermetures comprises) | IMPLEMENTS | 6 |
| impl vers son propre type | CONSUMES | 6 |
| impl vers son propre type | IMPLEMENTS | 2 |

**variable locale (let, for, :=, fermetures comprises)** (CONSUMES, 655)

- `src/dataflow/checkpoint.rs:185` checkpoint_encode_batch → checkpoint — `let checkpoint: Vec<CheckpointRelationRecord> =`
- `src/dataflow/derive_nodes.rs:147` DeriveNode → entity — `for (entity, racines) in &par_entite {`
- `src/dataflow/graph_node.rs:51` GraphNode → schema — `pub fn from_definition(`
- `src/dataflow/graph_tool.rs:1409` parse_header → choices — `let mut choices = Vec::new();`
- `src/dataflow/migrations.rs:80` scan_migration_dir → name — `let (version, name) = match parse_migration_filename(&filename) {`
- `src/dataflow/ocr_nodes.rs:227` tests → f — `let f = OcrNodeFactory;`
- `src/dataflow/record_nodes.rs:1634` EmbedNode → embed_dense — `fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {`
- `src/dataflow/render_nodes.rs:745` structured_tree → key — `for (i,(key,child)) in complex.iter().enumerate() {`
- `src/dataflow/runtime.rs:835` execute_as → result — `let result = self.execute_inner(graph, &run_id);`
- `src/dataflow/search_nodes.rs:221` fetch_related → result — `let result = conn`

**mot de commentaire** (CONSUMES, 578)

- `src/dataflow/checkpoint.rs:1` file_scope_01 → types — `//! Checkpoint types and serialization for dataflow execution persistence.`
- `src/dataflow/code_nodes.rs:320` file_scope_05 → result — `/// `list` : les fichiers de la `file_source` sous un préfixe, avec leur état`
- `src/dataflow/graph.rs:12` file_scope_01 → input — `/// A directed edge connecting an output port to an input port.`
- `src/dataflow/graph_tool.rs:604` file_scope_12 → EventSourceNode — `/// La même, sous un identifiant de run choisi (`None` : généré) — l'adresse`
- `src/dataflow/migration_nodes.rs:23` file_scope_01 → result — `/// The result is stored as the undo context (serialized JSON rows).`
- `src/dataflow/node_registry.rs:12` file_scope_01 → ConfigParam — `// ─── ConfigParam ─────────────────────────────────────────────────────────────`
- `src/dataflow/port.rs:279` file_scope_07 → through — `///`
- `src/dataflow/record_nodes.rs:2302` file_scope_17 → table — `///`
- `src/dataflow/render_nodes.rs:828` file_scope_26 → forme — `///`
- `src/dataflow/search_nodes.rs:281` file_scope_04 → port — `/// porte `_frame_title` (le titre du parent) et `_frame` (son champ de`

**mot de commentaire (ligne décalée avant le correctif)** (CONSUMES, 232)

- `src/dataflow/checkpoint.rs:52` file_scope_02 → CheckpointPortValue — ``
- `src/dataflow/derive_nodes.rs:75` file_scope_03 → hash — ``
- `src/dataflow/graph_node.rs:240` file_scope_02 → GraphNodeFactory — ``
- `src/dataflow/index_nodes.rs:47` file_scope_02 → noms — ``
- `src/dataflow/migrations.rs:259` file_scope_09 → MigrationRunner — ``
- `src/dataflow/node_registry.rs:89` file_scope_03 → parameter — ``
- `src/dataflow/port.rs:162` file_scope_04 → PortDef — ``
- `src/dataflow/record_nodes.rs:531` file_scope_06 → LinkRecordNode — ``
- `src/dataflow/render_nodes.rs:587` file_scope_21 → payload — ``
- `src/dataflow/runtime.rs:139` file_scope_04 → DataflowGraph — ``

**héritage deviné, devenu CONSUMES** (IMPLEMENTS, 73)

- `src/dataflow/checkpoint_store.rs:841` PostgresCheckpointStore → s — `.filter_map(|row| row[0].as_str().map(|s| s.to_string()))`
- `src/dataflow/code_nodes.rs:602` ListFilesNodeFactory → Node — `fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> `
- `src/dataflow/generic_search_nodes.rs:133` SearchSourceNode → f — `let has = |f: &str| fields.as_ref().is_some_and(|s| s.contains(f));`
- `src/dataflow/index_nodes.rs:130` EstimateNodeFactory → Node — `fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> `
- `src/dataflow/node_factories.rs:64` FieldWeightNodeFactory → Node — `) -> Result<Box<dyn super::node::Node>, String> {`
- `src/dataflow/node_factories.rs:473` FetchRelatedNodeFactory → Node — `) -> Result<Box<dyn super::node::Node>, String> {`
- `src/dataflow/node_factories.rs:1075` FuseResultsNodeFactory → Node — `) -> Result<Box<dyn super::node::Node>, String> {`
- `src/dataflow/node_registry.rs:351` FakeNodeFactory → FakeNode — `Ok(Box::new(FakeNode { node_name: name.into(), value }))`
- `src/dataflow/record_nodes.rs:3266` UpdateRecordNode → f — `let ty = config.entities.get(entity_name).and_then(|e| e.fields.get(k)).map(|f| &f.field_t`
- `src/dataflow/run_nodes.rs:339` WaitOutputNodeFactory → Node — `fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> `

**variable locale (let, for, :=, fermetures comprises)** (IMPLEMENTS, 6)

- `src/dataflow/code_nodes.rs:73` ParseCodeNode → f — `for f in &mut a.files {`
- `src/dataflow/record_nodes.rs:224` InsertRecordNode → i — `for i in (0..items.len()).rev() {`
- `src/dataflow/record_nodes.rs:889` LinkRecordNode → i — `for (i, rel) in items.iter_mut().enumerate() {`
- `src/dataflow/record_nodes.rs:2082` EmbedNode → i — `for (i, (dense, sparse)) in`
- `src/dataflow/record_nodes.rs:3072` UpdateRecordNode → i — `for (i, rec) in items.iter().enumerate() {`
- `src/dataflow/trace_nodes.rs:1057` SendMessageNodeFactory → p — `let p = |name: &'static str, required: bool, default: Option<serde_json::Value>, descripti`

**impl vers son propre type** (CONSUMES, 6)

- `src/dataflow/port.rs:80` PortValue → PortValue — `pub fn is_trigger(&self) -> bool {`
- `src/dataflow/port.rs:80` PortValue → PortValue — `pub fn is_trigger(&self) -> bool {`
- `src/dataflow/port.rs:158` PortType → PortType — `pub fn compatible_with(&self, other: &PortType) -> bool {`
- `src/dataflow/report.rs:121` ExecutionReport → ExecutionReport — `pub fn build(`
- `src/dataflow/runtime.rs:48` PortSnapshot → PortSnapshot — `pub(crate) fn from_port(name: &str, value: &PortValue) -> Self {`
- `src/dataflow/services.rs:49` ServiceRegistry → ServiceRegistry — `pub fn layered(parent: Arc<ServiceRegistry>) -> Self {`

**impl vers son propre type** (IMPLEMENTS, 2)

- `src/dataflow/port.rs:80` PortValue → PortValue — `pub fn is_trigger(&self) -> bool {`
- `src/dataflow/services.rs:49` ServiceRegistry → ServiceRegistry — `pub fn layered(parent: Arc<ServiceRegistry>) -> Self {`

ajoutées sans être d'anciens héritages : 0

