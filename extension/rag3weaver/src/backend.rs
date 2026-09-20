//! A backend is a manifest, payload schemas and explicitly exposed graph tools.
//! No domain vocabulary lives in this module. One host owns the catalogue.
use crate::dataflow::{
    build_definition, builtin_graph_tools, port_value_to_checkpoint, DataflowRuntime,
    GraphDefinition, GraphTool, NodeRegistry, NodeTypePolicy, ServiceRegistry,
};
use crate::{
    catalog::Catalog,
    config::{CatalogConfig, EntityConfig, RelationDef},
    connection::DbConnection,
    embedder::Embedder,
    json_schema::JsonSchemaMapping,
    search_backend::MoteurTexte,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackendManifest {
    pub version: u32,
    /// Script sources and hook resources are chosen by the host, never tool callers.
    #[serde(default)]
    pub scripts: BTreeMap<String, PathBuf>,
    pub name: String,
    pub database: PathBuf,
    pub embeddings: EmbeddingService,
    pub vector_extension: PathBuf,
    pub entities: BTreeMap<String, BackendEntity>,
    #[serde(default)]
    pub relations: HashMap<String, RelationDef>,
    /// Only these attachments become tools. Bindings cannot be overridden by callers.
    pub tools: BTreeMap<String, ToolAttachment>,
    /// Opt in to discovery and ad-hoc read-only Mermaid search tools.
    #[serde(default)]
    pub search_graphs: bool,
    /// Creation-time Lucivy option; existing indexes retain their persisted setting.
    #[serde(default)]
    pub fts_positions: Option<bool>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingService {
    pub address: String,
    pub model: String,
    pub dimensions: usize,
    #[serde(default)]
    pub provider: EmbeddingProvider,
    #[serde(default)]
    pub api_key_env: Option<String>,
}
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingProvider {
    #[default]
    Daemon,
    Compatible,
}
impl EmbeddingService {
    #[cfg(feature = "daemon")]
    pub fn connect(&self) -> Result<Box<dyn Embedder>, String> {
        match self.provider {
            EmbeddingProvider::Daemon => Ok(Box::new(
                crate::daemon::DaemonEmbedder::joindre(&self.address).map_err(|e| e.to_string())?,
            )),
            EmbeddingProvider::Compatible => Ok(Box::new(crate::http_embedder::HttpEmbedder::new(
                &self.address,
                &self.model,
                self.dimensions,
                self.api_key_env.as_deref(),
            )?)),
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackendEntity {
    pub schema: PathBuf,
    pub config: EntityConfig,
    #[serde(default)]
    pub writes: WritePolicy,
}
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WritePolicy {
    /// Optional server-generated epoch-millisecond fields, declared as integers in the schema.
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub revision: Option<String>,
    /// Immutable rows can be inserted or replayed identically, never overwritten.
    #[serde(default)]
    pub immutable: bool,
    #[serde(default)]
    pub transition_dates: Vec<TransitionDate>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransitionDate {
    pub field: String,
    pub to: Value,
    pub timestamp_field: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolAttachment {
    pub graph: PathBuf,
    #[serde(default)]
    pub harness: ToolHarness,
    #[serde(default)]
    pub bindings: serde_json::Map<String, Value>,
    /// Derive caller payloads from the entity schema, excluding server-owned fields.
    #[serde(default)]
    pub input_payloads: BTreeMap<String, InputPayload>,
    /// Terminal metadata ports to include alongside the declared result.
    #[serde(default)]
    pub metadata: Vec<OutputPort>,
}
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolHarness {
    pub input_schema: Option<PathBuf>,
    #[serde(default)]
    pub before: Vec<HookGraph>,
    #[serde(default)]
    pub after: Vec<HookGraph>,
    /// Post-acceptance transformations. Failure is delivery failure, not rollback.
    #[serde(default)]
    pub on_accept: Vec<HookGraph>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HookGraph {
    pub graph: PathBuf,
    #[serde(default)]
    pub data: BTreeMap<String, PathBuf>,
}
struct PreparedHook {
    tool: GraphTool,
    data: Value,
}
struct PreparedHarness {
    before: Vec<PreparedHook>,
    after: Vec<PreparedHook>,
    on_accept: Vec<PreparedHook>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InputPayload {
    pub entity: String,
    pub view: PayloadView,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadView {
    Identity,
    Editable,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutputPort {
    pub node: String,
    pub port: String,
}

pub struct PreparedBackend {
    pub manifest: BackendManifest,
    pub directory: PathBuf,
    entities: HashMap<String, EntityConfig>,
    mappings: HashMap<String, JsonSchemaMapping>,
    validators: HashMap<String, Arc<jsonschema::Validator>>,
    tools: BTreeMap<String, GraphTool>,
    nodes: NodeRegistry,
    tool_schemas: BTreeMap<String, Value>,
    tool_validators: HashMap<String, jsonschema::Validator>,
    harnesses: HashMap<String, PreparedHarness>,
    scripts: BTreeMap<String, String>,
}
impl PreparedBackend {
    /// Compile transport-independent search verbs without opening or querying a DB.
    pub fn compile_search_program(
        &self,
        program: &crate::dataflow::search_chain::SearchProgram,
    ) -> Result<crate::dataflow::search_chain::SearchPlan, String> {
        program.compile(
            &crate::dataflow::search_chain::SearchSchema {
                entities: &self.entities,
                relations: &self.manifest.relations,
            },
            &self.nodes,
        )
    }

    /// Pure preparation: read and validate descriptors before opening a database.
    pub fn load(path: &Path) -> Result<Self, String> {
        let manifest: BackendManifest =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if manifest.version != 1 {
            return Err("unsupported backend manifest version".into());
        }
        if manifest.database.as_os_str().is_empty()
            || manifest.database.to_string_lossy() == ":memoire:"
        {
            return Err("backend requires a persistent database path".into());
        }
        let directory = path
            .canonicalize()
            .map_err(|e| e.to_string())?
            .parent()
            .unwrap()
            .to_path_buf();
        let (mut nodes, _) = builtin_graph_tools().map_err(|e| e.to_string())?;
        nodes.register(Box::new(crate::backend_nodes::EntityRecordFactory));
        nodes.register(Box::new(crate::backend_nodes::EntityBatchFactory));
        nodes.register(Box::new(crate::backend_nodes::RelationBatchFactory));
        let mut schemas = HashMap::new();
        let mut mappings = HashMap::new();
        let mut entities = HashMap::new();
        let mut validators = HashMap::new();
        for (name, entity) in &manifest.entities {
            crate::schema::validate_identifier(name, "entity").map_err(|e| e.to_string())?;
            let schema: Value = serde_json::from_slice(
                &std::fs::read(directory.join(&entity.schema)).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            validators.insert(
                name.clone(),
                Arc::new(jsonschema::validator_for(&schema).map_err(|e| e.to_string())?),
            );
            let mapping = JsonSchemaMapping::from_schema(&schema)?;
            let mut config = entity.config.clone();
            for (field, declared) in &config.fields {
                let mapped = mapping
                    .fields
                    .get(field)
                    .ok_or_else(|| format!("{name}.{field}: missing from payload schema"))?;
                if mapped.field_type != declared.field_type {
                    return Err(format!("{name}.{field}: conflicting field type"));
                }
            }
            for (field, definition) in &mapping.fields {
                config
                    .fields
                    .entry(field.clone())
                    .or_insert_with(|| definition.clone());
            }
            for transition in &entity.writes.transition_dates {
                if !config.fields.contains_key(&transition.field) {
                    return Err(format!("unknown transition field {}", transition.field));
                }
            }
            let generated: Vec<&String> = [
                &entity.writes.created_at,
                &entity.writes.updated_at,
                &entity.writes.revision,
            ]
            .into_iter()
            .flatten()
            .chain(
                entity
                    .writes
                    .transition_dates
                    .iter()
                    .map(|t| &t.timestamp_field),
            )
            .collect();
            if generated
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != generated.len()
            {
                return Err("duplicate generated field".into());
            }
            for field in generated {
                let f = config
                    .fields
                    .get(field)
                    .ok_or_else(|| format!("generated field {field} missing"))?;
                if !matches!(
                    f.field_type,
                    crate::config::FieldType::Int64 | crate::config::FieldType::Integer
                ) {
                    return Err(format!("generated field {field} must be an integer"));
                }
            }
            config.validate()?;
            if config.hashsafe.as_ref().is_none_or(|v| v.is_empty()) {
                return Err(format!("{name}: stable hashsafe identity required"));
            }
            schemas.insert(name.clone(), schema);
            mappings.insert(name.clone(), mapping);
            entities.insert(name.clone(), config);
        }
        for (name, relation) in &manifest.relations {
            crate::schema::validate_identifier(name, "relation").map_err(|e| e.to_string())?;
            if !entities.contains_key(&relation.from) || !entities.contains_key(&relation.to) {
                return Err(format!("{name}: unknown relation endpoint"));
            }
        }
        let scripts = manifest
            .scripts
            .iter()
            .map(|(name, path)| {
                let text =
                    std::fs::read_to_string(directory.join(path)).map_err(|e| e.to_string())?;
                if text.len() > 65536 {
                    return Err(format!("script {name} too large"));
                }
                Ok((name.clone(), text))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        let prepare_hooks = |hooks: &[HookGraph]| -> Result<Vec<PreparedHook>, String> {
            hooks
                .iter()
                .map(|hook| {
                    let source = std::fs::read_to_string(directory.join(&hook.graph))
                        .map_err(|e| e.to_string())?;
                    let tool = GraphTool::from_mermaid(&source)
                        .and_then(|t| t.bind(&nodes))
                        .map_err(|e| e.to_string())?;
                    if tool.params().len() != 1 || tool.params()[0].name != "context" {
                        return Err("hook graph must declare exactly one context parameter".into());
                    }
                    let data = hook
                        .data
                        .iter()
                        .map(|(key, path)| {
                            let bytes =
                                std::fs::read(directory.join(path)).map_err(|e| e.to_string())?;
                            Ok((
                                key.clone(),
                                serde_json::from_slice::<Value>(&bytes)
                                    .map_err(|e| e.to_string())?,
                            ))
                        })
                        .collect::<Result<serde_json::Map<_, _>, String>>()?;
                    Ok(PreparedHook {
                        tool,
                        data: Value::Object(data),
                    })
                })
                .collect()
        };
        let mut harnesses = HashMap::new();
        let mut tools = BTreeMap::new();
        let mut tool_schemas = BTreeMap::new();
        let mut tool_validators = HashMap::new();
        for (name, attachment) in &manifest.tools {
            crate::schema::validate_identifier(name, "tool").map_err(|e| e.to_string())?;
            if manifest.search_graphs && SEARCH_TOOLS.contains(&name.as_str()) {
                return Err(format!("reserved search tool {name}"));
            }
            let source = std::fs::read_to_string(directory.join(&attachment.graph))
                .map_err(|e| e.to_string())?;
            let tool = GraphTool::from_mermaid(&source)
                .and_then(|t| t.bind(&nodes))
                .map_err(|e| e.to_string())?;
            for key in attachment.bindings.keys() {
                if !tool.params().iter().any(|p| p.name == key) {
                    return Err(format!("{name}: unknown binding {key}"));
                }
            }
            let mut input = tool.tool_def().parameters;
            for (parameter, payload) in &attachment.input_payloads {
                if attachment.bindings.contains_key(parameter)
                    || input["properties"].get(parameter).is_none()
                {
                    return Err(format!(
                        "{name}: payload parameter {parameter} absent or bound"
                    ));
                }
                let schema = schemas
                    .get(&payload.entity)
                    .ok_or_else(|| format!("unknown payload entity {}", payload.entity))?;
                let entity = &manifest.entities[&payload.entity];
                input["properties"][parameter] = payload_schema(schema, entity, &payload.view)?;
            }
            if let Some(props) = input.get_mut("properties").and_then(Value::as_object_mut) {
                for key in attachment.bindings.keys() {
                    props.remove(key);
                }
            }
            if let Some(required) = input.get_mut("required").and_then(Value::as_array_mut) {
                required.retain(|v| {
                    v.as_str()
                        .is_none_or(|k| !attachment.bindings.contains_key(k))
                });
            }
            if let Some(path) = &attachment.harness.input_schema {
                input = serde_json::from_slice(
                    &std::fs::read(directory.join(path)).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                let declared = tool.tool_def().parameters;
                let props = input["properties"]
                    .as_object()
                    .ok_or("harness schema requires root properties")?;
                if props.keys().any(|k| {
                    declared["properties"].get(k).is_none() || attachment.bindings.contains_key(k)
                }) {
                    return Err("harness schema cannot expose unknown or bound parameters".into());
                }
            }
            harnesses.insert(
                name.clone(),
                PreparedHarness {
                    before: prepare_hooks(&attachment.harness.before)?,
                    after: prepare_hooks(&attachment.harness.after)?,
                    on_accept: prepare_hooks(&attachment.harness.on_accept)?,
                },
            );
            input["additionalProperties"] = json!(false);
            tool_validators.insert(
                name.clone(),
                jsonschema::validator_for(&input).map_err(|e| e.to_string())?,
            );
            tool_schemas.insert(name.clone(), input);
            tools.insert(name.clone(), tool);
        }
        Ok(Self {
            manifest,
            directory,
            entities,
            mappings,
            validators,
            tools,
            nodes,
            tool_schemas,
            tool_validators,
            harnesses,
            scripts,
        })
    }
    pub fn path(&self, path: &Path) -> PathBuf {
        self.directory.join(path)
    }
    pub fn describe(&self) -> Value {
        let mut tools:Vec<Value>=self.tools.iter().map(|(name,t)|{
            json!({"name":name,"description":t.tool_def().description,"inputSchema":self.tool_schemas[name]})
        }).collect();
        if self.manifest.search_graphs {
            tools.extend(search_tool_definitions());
        }
        json!({"name":self.manifest.name,"version":self.manifest.version,"database":self.path(&self.manifest.database),"fts":"lucivy","embeddings":self.manifest.embeddings,"payloads":self.mappings,"tools":tools})
    }
    pub fn open(
        self,
        conn: Box<dyn DbConnection>,
        embedder: Box<dyn Embedder>,
    ) -> Result<Backend, String> {
        if embedder.is_mock()
            || embedder.name() != self.manifest.embeddings.model
            || embedder.dim() != self.manifest.embeddings.dimensions
        {
            return Err("embedding model identity differs from manifest".into());
        }
        let mut cat = Catalog::new(
            conn,
            embedder,
            CatalogConfig {
                name: Some(self.manifest.name.clone()),
                embedding_dim: self.manifest.embeddings.dimensions,
                checkpoint_dir: Some(
                    self.path(&self.manifest.database)
                        .with_extension("checkpoints"),
                ),
                ..Default::default()
            },
        );
        cat.set_moteur_texte(MoteurTexte::Lucivy);
        cat.set_fts_positions(self.manifest.fts_positions.unwrap_or(true));
        let ext = self.path(&self.manifest.vector_extension);
        cat.execute_raw(&format!(
            "LOAD EXTENSION '{}'",
            ext.to_string_lossy().replace('\'', "''")
        ))
        .map_err(|e| e.to_string())?;
        cat.initialize().map_err(|e| e.to_string())?;
        for (name, config) in &self.entities {
            cat.register_entity(name, config.clone())
                .map_err(|e| e.to_string())?;
        }
        for (name, r) in &self.manifest.relations {
            cat.register_relation_with(
                name,
                &r.from,
                &r.to,
                r.properties.clone().unwrap_or_default(),
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(Backend {
            prepared: self,
            catalog: Arc::new(Mutex::new(cat)),
        })
    }
}

// An explicit capability list: no SQL, script, write node or nested graph factory.
const SEARCH_NODES: &[&str] = &[
    "KBQuerySourceNode",
    "SelectRecordsNode",
    "SearchSourceNode",
    "BM25SearchNode",
    "VectorSearchNode",
    "SparseSearchNode",
    "RelatedResultsNode",
    "IntersectResultsNode",
    "LabelResultsNode",
    "FuseResultsNode",
    "RerankNode",
    "PaginateNode",
    "ResolveParentNode",
    "RenderResultsNode",
    "FetchRelatedNode",
    "ComposeNode",
    "GroupFrameNode",
];
const SEARCH_TOOLS: &[&str] = &["describe_backend", "validate_search", "run_search"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchGraphRequest {
    mermaid: String,
    #[serde(default = "empty_object")]
    arguments: Value,
    #[serde(default)]
    metadata: Vec<OutputPort>,
}
fn empty_object() -> Value {
    json!({})
}
fn search_tool_definitions() -> Vec<Value> {
    let input = json!({"type":"object","additionalProperties":false,"required":["mermaid"],"properties":{
        "mermaid":{"type":"string","minLength":1,"maxLength":65536,"description":"GraphTool Mermaid, including %% tool, %% param and %% result headers. Discover nodes and examples first."},
        "arguments":{"type":"object","description":"Values for graph parameters; no text interpolation required."},
        "metadata":{"type":"array","maxItems":16,"items":{"type":"object","additionalProperties":false,"required":["node","port"],"properties":{"node":{"type":"string"},"port":{"type":"string"}}}}
    }});
    vec![
        json!({"name":"describe_backend","description":"Discover entity payloads, identities, relations, read-only nodes, filter grammar and attached Mermaid examples.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}),
        json!({"name":"validate_search","description":"Validate a read-only Mermaid search without executing queries. Same validation as run_search; does not estimate cost or prove data availability.","inputSchema":input}),
        json!({"name":"run_search","description":"Execute a composed read-only Mermaid search on the persistent backend. Returns structured result and requested terminal metadata; does not truncate intermediate branches.","inputSchema":input}),
    ]
}
impl PreparedBackend {
    fn search_description(&self) -> Value {
        let nodes: Vec<Value> = SEARCH_NODES.iter().filter_map(|name| self.nodes.schema(name)).map(|s| {
            let port = |p: &crate::dataflow::PortDef| json!({"name":p.name,"type":format!("{:?}",p.port_type),"required":p.required});
            json!({"type":s.node_type,"description":s.description,
                "inputs":s.inputs.iter().map(port).collect::<Vec<_>>(),
                "outputs":s.outputs.iter().map(port).collect::<Vec<_>>(),
                "parameters":s.config_params.iter().map(|p| json!({"name":p.name,"type":format!("{:?}",p.param_type),"required":p.required,"default":p.default,"description":p.description,"schema":p.json_schema,"choices":p.choices.as_ref().map(|c| c.spec())})).collect::<Vec<_>>()})
        }).collect();
        let examples: Vec<Value> = self.tools.iter().filter(|(_, t)| t.template().nodes.iter().all(|n| SEARCH_NODES.contains(&n.node_type.as_str()))).map(|(name,t)| json!({"name":name,"mermaid":t.to_mermaid(),"bindings":self.manifest.tools[name].bindings,"metadata":self.manifest.tools[name].metadata})).collect();
        json!({"backend":self.describe(),"entities":self.entities,"relations":self.manifest.relations,"nodes":nodes,"examples":examples,
            "filter_examples":{"all":{"must":[]},"scalar":{"path":{"path":["field"],"value":[{"op":"eq","value":true}]}},"array":{"path":{"path":["tags"],"value":[{"op":"has_any","value":["value"]}]}},"nested":{"nested":{"path":["abilities"],"condition":{"field":{"key":"text_en","value":[{"op":"contains","value":"graveyard"}]}}}}},
            "limits":{"mermaid_bytes":65536,"nodes":64,"edges":256,"metadata_ports":16},
            "notes":["Ad-hoc graphs are read-only; declared named tools may permit writes.","Validation checks graph structure and declared endpoints, not query cost or runtime service health.","No implicit top-k on intermediate selections; use PaginateNode explicitly for final presentation.","FuseResultsNode merges duplicates by entity and UUID by default; use its duplicates option to retain occurrences."]})
    }
    fn prepare_search(
        &self,
        request: &SearchGraphRequest,
    ) -> Result<(GraphTool, GraphDefinition), String> {
        if request.mermaid.len() > 65536 {
            return Err("Mermaid exceeds 65536 bytes".into());
        }
        let tool = GraphTool::from_mermaid(&request.mermaid)
            .and_then(|t| t.bind(&self.nodes))
            .map_err(|e| e.to_string())?;
        let mut schema = tool.tool_def().parameters;
        schema["additionalProperties"] = json!(false);
        jsonschema::validator_for(&schema)
            .map_err(|e| e.to_string())?
            .validate(&request.arguments)
            .map_err(|e| e.to_string())?;
        let def = tool
            .instantiate(&request.arguments)
            .map_err(|e| e.to_string())?;
        if def.nodes.len() > 64 || def.edges.len() > 256 {
            return Err("search graph exceeds 64 nodes or 256 edges".into());
        }
        for node in &def.nodes {
            for key in ["entity", "target_name", "source_entity"] {
                if let Some(name) = node
                    .config
                    .get(key)
                    .and_then(Value::as_str)
                    .filter(|n| !n.is_empty())
                {
                    if !self.entities.contains_key(name) {
                        return Err(format!("{}.{}: unknown entity {name}", node.name, key));
                    }
                }
            }
            let entity = node
                .config
                .get("entity")
                .or_else(|| node.config.get("target_name"))
                .and_then(Value::as_str);
            let filter = node.config.get("filter").or_else(|| {
                node.config
                    .get("options")
                    .and_then(|o| o.get("filter_condition"))
            });
            if let (Some(entity), Some(filter)) = (entity, filter) {
                if !filter.is_null() {
                    let condition: crate::filter::FilterCondition =
                        serde_json::from_value(filter.clone()).map_err(|e| e.to_string())?;
                    crate::json_schema::validate_structured_filter(
                        &condition,
                        &self.entities[entity].fields,
                    )?;
                }
            }
            if let Some(name) = node.config.get("relation").and_then(Value::as_str) {
                let relation = self
                    .manifest
                    .relations
                    .get(name)
                    .ok_or_else(|| format!("{}: unknown relation {name}", node.name))?;
                if let Some(source) = node
                    .config
                    .get("source_entity")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                {
                    let expected = if node.config["direction"].as_str() == Some("Incoming") {
                        &relation.to
                    } else {
                        &relation.from
                    };
                    if source != expected {
                        return Err(format!(
                            "{}: source_entity must be {expected} for this relation direction",
                            node.name
                        ));
                    }
                }
            }
        }
        build_definition(
            &def,
            &self.nodes,
            &NodeTypePolicy::only(SEARCH_NODES.iter().copied()),
        )
        .map_err(|e| e.to_string())?;
        for (node, port) in std::iter::once(tool.result()).chain(
            request
                .metadata
                .iter()
                .map(|p| (p.node.as_str(), p.port.as_str())),
        ) {
            let n = def
                .nodes
                .iter()
                .find(|n| n.name == node)
                .ok_or_else(|| format!("unknown output node {node}"))?;
            let schema = self.nodes.schema(&n.node_type).unwrap();
            if !schema.outputs.iter().any(|p| p.name == port) {
                return Err(format!("unknown output {node}.{port}"));
            }
            if def
                .edges
                .iter()
                .any(|e| e.from_node == node && e.from_port == port)
            {
                return Err(format!(
                    "output {node}.{port} is consumed; expose a terminal port"
                ));
            }
        }
        Ok((tool, def))
    }
}

pub struct Backend {
    pub prepared: PreparedBackend,
    pub catalog: Arc<Mutex<Catalog>>,
}
impl Backend {
    /// Library API shared by applications and future transports. No MCP dispatch.
    pub fn run_search_program(
        &self,
        program: &crate::dataflow::search_chain::SearchProgram,
    ) -> Result<Value, String> {
        let plan = self.prepared.compile_search_program(program)?;
        let metadata = plan
            .metadata_nodes
            .iter()
            .map(|n| OutputPort {
                node: n.clone(),
                port: "meta".into(),
            })
            .collect::<Vec<_>>();
        let mut response = self.execute_plan(
            (&plan.result_node, "results"),
            &plan.graph,
            &metadata,
            &NodeTypePolicy::only(SEARCH_NODES.iter().copied()),
        )?;
        response["graph_hash"] = json!(plan.graph.hash());
        response["candidate_bounded"] = json!(plan.candidate_bounded);
        response["diagnostics"] = json!(plan.diagnostics);
        Ok(response)
    }

    pub fn call(&self, name: &str, args: Value) -> Result<Value, String> {
        if self.prepared.manifest.search_graphs && SEARCH_TOOLS.contains(&name) {
            let definition = search_tool_definitions()
                .into_iter()
                .find(|t| t["name"] == name)
                .unwrap();
            jsonschema::validator_for(&definition["inputSchema"])
                .map_err(|e| e.to_string())?
                .validate(&args)
                .map_err(|e| e.to_string())?;
            if name == "describe_backend" {
                return Ok(self.prepared.search_description());
            }
            let request: SearchGraphRequest =
                serde_json::from_value(args).map_err(|e| e.to_string())?;
            let (tool, def) = self.prepared.prepare_search(&request)?;
            if name == "validate_search" {
                return Ok(
                    json!({"valid":true,"read_only":true,"graph_hash":def.hash(),"plan":def,
                    "result":{"node":tool.result().0,"port":tool.result().1},
                    "validation":"structure, parameters, entities, relations and ports; no query executed"}),
                );
            }
            let mut response = self.execute_graph(
                &tool,
                &def,
                &request.metadata,
                &NodeTypePolicy::only(SEARCH_NODES.iter().copied()),
            )?;
            response["graph_hash"] = json!(def.hash());
            return Ok(response);
        }
        let tool = self
            .prepared
            .tools
            .get(name)
            .ok_or_else(|| format!("unexposed tool {name}"))?;
        let attachment = &self.prepared.manifest.tools[name];
        let mut args = args
            .as_object()
            .ok_or("arguments must be an object")?
            .clone();
        for key in args.keys() {
            if attachment.bindings.contains_key(key) {
                return Err(format!("cannot override binding {key}"));
            }
            if !tool.params().iter().any(|p| p.name == key) {
                return Err(format!("unknown argument {key}"));
            }
        }
        let errors: Vec<Value> = self.prepared.tool_validators[name].iter_errors(&Value::Object(args.clone())).map(|e|json!({"code":"schema","path":e.instance_path().to_string(),"message":e.to_string()})).collect();
        if !errors.is_empty() {
            if attachment.harness.input_schema.is_none()
                && attachment.harness.before.is_empty()
                && attachment.harness.after.is_empty()
                && attachment.harness.on_accept.is_empty()
            {
                return Err(errors[0]["message"]
                    .as_str()
                    .unwrap_or("invalid input")
                    .into());
            }
            return Ok(
                json!({"validation":{"accepted":false,"errors":errors,"warnings":[]},"stage":"input_schema","executed":false}),
            );
        }
        let mut context = json!({"tool":name,"arguments":args,"result":null});
        let harness = &self.prepared.harnesses[name];
        let before = self.validate_hooks(&harness.before, &context);
        if !before.accepted {
            return Ok(json!({"validation":before,"stage":"before","executed":false}));
        }
        args.extend(attachment.bindings.clone());
        let def = tool
            .instantiate(&Value::Object(args))
            .map_err(|e| e.to_string())?;
        let policy = NodeTypePolicy::only([
            "RhaiNode",
            "EntityRecordNode",
            "EntityBatchNode",
            "RelationBatchNode",
            "KBQuerySourceNode",
            "SelectRecordsNode",
            "RelatedResultsNode",
            "IntersectResultsNode",
            "LabelResultsNode",
            "SearchSourceNode",
            "BM25SearchNode",
            "VectorSearchNode",
            "SparseSearchNode",
            "FuseResultsNode",
            "RerankNode",
            "PaginateNode",
            "ResolveParentNode",
            "RenderResultsNode",
            "FetchRelatedNode",
            "ComposeNode",
            "GroupFrameNode",
        ]);
        let mut response = self.execute_graph(tool, &def, &attachment.metadata, &policy)?;
        context["result"] = response["result"].clone();
        let mut report = self.validate_hooks(&harness.after, &context);
        report.warnings.extend(before.warnings);
        if !report.accepted {
            response["validation"] = serde_json::to_value(report).unwrap();
            response["stage"] = json!("after");
            response["executed"] = json!(true);
            return Ok(response);
        }
        response["validation"] = serde_json::to_value(report).unwrap();
        let mut deliveries = Vec::new();
        for hook in &harness.on_accept {
            match self.call_hook(hook, &context) {
                Ok(value) => deliveries.push(value),
                Err(error) => {
                    response["delivery"] = json!({"ok":false,"error":error,"completed":deliveries});
                    return Ok(response);
                }
            }
        }
        response["delivery"] = json!({"ok":true,"results":deliveries});
        Ok(response)
    }
    fn call_hook(&self, hook: &PreparedHook, context: &Value) -> Result<Value, String> {
        let mut context = context.clone();
        context["data"] = hook.data.clone();
        let def = hook
            .tool
            .instantiate(&json!({"context":context}))
            .map_err(|e| e.to_string())?;
        // Hook graphs compute over supplied facts. They cannot invoke shell or mutate the DB.
        let response = self.execute_graph(
            &hook.tool,
            &def,
            &[],
            &NodeTypePolicy::only(["RhaiNode", "ValidationRuleNode", "ValidationMergeNode"]),
        )?;
        Ok(response["result"].clone())
    }
    fn validate_hooks(
        &self,
        hooks: &[PreparedHook],
        context: &Value,
    ) -> crate::harness::ValidationReport {
        use crate::harness::{ValidationIssue, ValidationReport};
        let mut combined = ValidationReport {
            accepted: true,
            errors: vec![],
            warnings: vec![],
        };
        for hook in hooks {
            match self
                .call_hook(hook, context)
                .and_then(ValidationReport::from_value)
            {
                Ok(report) => {
                    combined.warnings.extend(report.warnings);
                    combined.errors.extend(report.errors);
                    if !report.accepted {
                        combined.accepted = false;
                    }
                }
                Err(message) => {
                    combined.accepted = false;
                    combined.errors.push(ValidationIssue {
                        code: "hook_failed".into(),
                        path: "".into(),
                        message,
                        node: None,
                        params: Default::default(),
                    });
                }
            }
        }
        combined
    }
    fn execute_graph(
        &self,
        tool: &GraphTool,
        def: &GraphDefinition,
        metadata_ports: &[OutputPort],
        policy: &NodeTypePolicy,
    ) -> Result<Value, String> {
        self.execute_plan(tool.result(), def, metadata_ports, policy)
    }
    fn execute_plan(
        &self,
        result_port: (&str, &str),
        def: &GraphDefinition,
        metadata_ports: &[OutputPort],
        policy: &NodeTypePolicy,
    ) -> Result<Value, String> {
        let mut graph =
            build_definition(def, &self.prepared.nodes, policy).map_err(|e| e.to_string())?;
        // The host serializes tool calls; services are fresh so shutdown owns all handles.
        let mut services = ServiceRegistry::new();
        self.catalog
            .lock()
            .unwrap()
            .register_search_services(&mut services);
        services.register("catalog", self.catalog.clone());
        services.register("rhai_scripts", self.prepared.scripts.clone());
        services.register(
            "backend_relations",
            self.prepared.manifest.relations.clone(),
        );
        services.register("backend_mappings", self.prepared.mappings.clone());
        services.register("backend_validators", self.prepared.validators.clone());
        services.register(
            "backend_write_policies",
            self.prepared
                .manifest
                .entities
                .iter()
                .map(|(k, v)| (k.clone(), v.writes.clone()))
                .collect::<HashMap<_, _>>(),
        );
        let output = DataflowRuntime::with_services(100, services).execute(&mut graph)?;
        let read = |node: &str, port: &str| -> Result<Value, String> {
            let value = output
                .get(node, port)
                .ok_or_else(|| format!("output {node}.{port} absent; expose a terminal port"))?;
            let cp = port_value_to_checkpoint(value)?;
            match cp.data_json {
                Some(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
                None => Ok(Value::Null),
            }
        };
        let result = read(result_port.0, result_port.1)?;
        let mut metadata = serde_json::Map::new();
        for port in metadata_ports {
            metadata.insert(
                format!("{}.{}", port.node, port.port),
                read(&port.node, &port.port)?,
            );
        }
        let mut response = json!({"result":result,"metadata":metadata});
        if result_port.1 == "results" && output.get(result_port.0, "text").is_some() {
            response["presentation"] = read(result_port.0, "text")?;
        }
        Ok(response)
    }
    pub fn shutdown(&mut self) -> Result<(), String> {
        self.catalog
            .lock()
            .unwrap()
            .shutdown()
            .map_err(|e| e.to_string())
    }
}

fn payload_schema(
    schema: &Value,
    entity: &BackendEntity,
    view: &PayloadView,
) -> Result<Value, String> {
    let mut result = schema.clone();
    let props = result
        .get_mut("properties")
        .and_then(Value::as_object_mut)
        .ok_or("input payload views require root object properties")?;
    let generated: Vec<&String> = [
        &entity.writes.created_at,
        &entity.writes.updated_at,
        &entity.writes.revision,
    ]
    .into_iter()
    .flatten()
    .chain(
        entity
            .writes
            .transition_dates
            .iter()
            .map(|t| &t.timestamp_field),
    )
    .collect();
    let identity = entity.config.hashsafe.as_ref().ok_or("identity missing")?;
    props.retain(|key, _| match view {
        PayloadView::Identity => identity.contains(key),
        PayloadView::Editable => !generated.contains(&key),
    });
    let names: Vec<String> = props.keys().cloned().collect();
    if let Some(required) = result.get_mut("required").and_then(Value::as_array_mut) {
        required.retain(|v| v.as_str().is_some_and(|k| names.iter().any(|n| n == k)));
    }
    result["additionalProperties"] = json!(false);
    // Entity-local $refs must resolve inside this payload even when embedded in a tool schema.
    fn inline_refs(value: &mut Value, root: &Value, depth: usize) -> Result<(), String> {
        if depth > 32 {
            return Err("payload schema reference nesting too deep".into());
        }
        if let Some(reference) = value.get("$ref").and_then(Value::as_str).map(str::to_owned) {
            let pointer = reference
                .strip_prefix('#')
                .ok_or("only local schema references supported")?;
            let resolved = root
                .pointer(pointer)
                .ok_or("unresolved payload schema reference")?
                .clone();
            let mut siblings = value.as_object().cloned().unwrap_or_default();
            siblings.remove("$ref");
            *value = if siblings.is_empty() {
                resolved
            } else {
                json!({"allOf":[resolved,siblings]})
            };
        }
        match value {
            Value::Object(map) => {
                for child in map.values_mut() {
                    inline_refs(child, root, depth + 1)?;
                }
            }
            Value::Array(items) => {
                for child in items {
                    inline_refs(child, root, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    result.as_object_mut().unwrap().remove("$defs");
    inline_refs(&mut result, schema, 0)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_verbs_run_through_library_without_a_transport() {
        use crate::connection::{CallbackConnection, CypherValue, QueryResult};
        use crate::dataflow::search_chain::{Chain, SearchProgram};
        use crate::embedder::HashEmbedder;
        let (prepared, _) = search_fixture();
        let program = SearchProgram {
            branches: BTreeMap::new(),
            pipeline: Chain::select(
                "Note",
                serde_json::from_value(json!({
                    "field":{"key":"text","value":"$literal"}
                }))
                .unwrap(),
            )
            .page(5, 0)
            .render("tree"),
        };
        let plan = prepared.compile_search_program(&program).unwrap();
        assert_eq!(plan.entity, "Note");
        let conn = CallbackConnection::new(|q, _| {
            Ok(if q.starts_with("MATCH (n:Note)") {
                QueryResult {
                    columns: vec![],
                    rows: vec![vec![CypherValue::Map(
                        [
                            ("_uuid".into(), CypherValue::String("note-1".into())),
                            ("text".into(), CypherValue::String("$literal".into())),
                        ]
                        .into(),
                    )]],
                }
            } else {
                QueryResult::default()
            })
        });
        let mut catalog = Catalog::new(
            Box::new(conn),
            Box::new(HashEmbedder::new(8)),
            CatalogConfig {
                allow_mock_embedder: true,
                embedding_dim: 8,
                ..Default::default()
            },
        );
        catalog.initialize().unwrap();
        for (name, config) in &prepared.entities {
            let mut config = config.clone();
            config.signals = crate::search::SearchSignals::NONE;
            catalog.register_entity(name, config).unwrap();
        }
        let backend = Backend {
            prepared,
            catalog: Arc::new(Mutex::new(catalog)),
        };
        let response = backend.run_search_program(&program).unwrap();
        assert_eq!(response["result"][0]["uuid"], "note-1");
        assert_eq!(response["result"][0]["data"]["text"], "$literal");
        assert_eq!(response["candidate_bounded"], false);
        assert!(response["presentation"].is_string());
        assert_eq!(response["graph_hash"], plan.graph.hash());
    }
    #[test]
    fn notebook_describes_typed_inputs_without_opening_storage() {
        let prepared = PreparedBackend::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook/backend.json"),
        )
        .unwrap();
        let descriptor = prepared.describe();
        let tools = descriptor["tools"].as_array().unwrap();
        let put = tools.iter().find(|t| t["name"] == "put_note").unwrap();
        let schema = &put["inputSchema"];
        assert!(schema["properties"].get("entity").is_none());
        assert!(schema["properties"]["record"]["properties"]
            .get("created_at")
            .is_none());
        let validator = jsonschema::validator_for(schema).unwrap();
        let valid =
            json!({"record":{"key":"id","text":"hello","labels":["Blue"],"stage":"working"}});
        assert!(validator.is_valid(&valid));
        let mut invalid = valid.clone();
        invalid["record"]["labels"] = json!("Blue");
        assert!(!validator.is_valid(&invalid));
        invalid = valid.clone();
        invalid["record"]["created_at"] = json!(0);
        assert!(!validator.is_valid(&invalid));
        invalid = valid;
        invalid["entity"] = json!("Snapshot");
        assert!(!validator.is_valid(&invalid));
    }
    fn search_fixture() -> (PreparedBackend, SearchGraphRequest) {
        let p = PreparedBackend::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook/backend.json"),
        )
        .unwrap();
        let request = SearchGraphRequest { mermaid: "%% tool: select\n%% description: Select records\n%% param: entity string! -- Entity\n%% param: filter json! -- Filter\n%% result: select.results\ngraph LR\n select[\"SelectRecordsNode(entity=$entity, filter=$filter)\"]".into(), arguments: json!({"entity":"Note","filter":{"must":[]}}), metadata:vec![] };
        (p, request)
    }
    #[test]
    fn ad_hoc_search_validates_without_storage() {
        let (p, r) = search_fixture();
        let (_, def) = p.prepare_search(&r).unwrap();
        assert_eq!(def.nodes.len(), 1);
        let description = p.search_description();
        assert!(description["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| n["type"] != "EntityRecordNode"));
    }
    #[test]
    fn ad_hoc_search_rejects_unknown_entities_parameters_fields_and_outputs() {
        let (p, mut r) = search_fixture();
        r.arguments["entity"] = json!("Missing");
        assert!(p.prepare_search(&r).unwrap_err().contains("unknown entity"));
        r.arguments["entity"] = json!("Note");
        r.arguments["typo"] = json!(1);
        assert!(p.prepare_search(&r).is_err());
        r.arguments.as_object_mut().unwrap().remove("typo");
        r.arguments["filter"] = json!({"path":{"path":["missing"],"value":true}});
        assert!(p.prepare_search(&r).is_err());
        r.arguments["filter"] = json!({"must":[]});
        r.metadata.push(OutputPort {
            node: "select".into(),
            port: "missing".into(),
        });
        assert!(p.prepare_search(&r).unwrap_err().contains("unknown output"));
    }
    #[test]
    fn ad_hoc_search_rejects_writes_and_oversized_graphs() {
        let (p, mut r) = search_fixture();
        r.arguments = json!({});
        // Use the existing known-valid write graph: rejection must be policy, not parsing.
        let tool = &p.tools["put_note"];
        r.mermaid = tool.to_mermaid();
        r.arguments =
            json!({"entity":"Note","record":{"key":"x","text":"x","labels":[],"stage":"working"}});
        let err = p.prepare_search(&r).unwrap_err();
        assert!(err.contains("EntityRecordNode"), "{err}");
        r.mermaid = " ".repeat(65537);
        assert!(p.prepare_search(&r).unwrap_err().contains("65536"));
    }
    #[test]
    fn local_refs_remain_valid_inside_derived_tool_input() {
        let prepared = PreparedBackend::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook/backend.json"),
        )
        .unwrap();
        let entity = &prepared.manifest.entities["Note"];
        let schema = json!({"type":"object","properties":{"key":{"$ref":"#/$defs/Id"}},"required":["key"],"$defs":{"Id":{"type":"string","minLength":1}}});
        let derived = payload_schema(&schema, entity, &PayloadView::Identity).unwrap();
        let outer = json!({"type":"object","properties":{"record":derived}});
        let validator = jsonschema::validator_for(&outer).unwrap();
        assert!(validator.is_valid(&json!({"record":{"key":"one"}})));
        assert!(!validator.is_valid(&json!({"record":{"key":""}})));
    }
}
