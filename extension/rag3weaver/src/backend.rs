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
    collections::{BTreeMap, BTreeSet, HashMap},
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
    /// La déclaration d'avant de l'embarquement dense. Gardée : c'est un
    /// alias de `models.embed` — l'une ou l'autre, pas les deux.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embeddings: Option<EmbeddingService>,
    /// **Les modèles, par capacité** — en local, en service à nous, ou chez
    /// un tiers : la même déclaration pour toutes
    /// (`crate::model_source::ModelSource`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub models: BTreeMap<crate::model_source::Capability, crate::model_source::ModelSource>,
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
    /// La surface de code (3 octobre 2026) : quelle source de fichiers, sous
    /// quelle racine, avec quelle porte de commandes. Absente : aucun outil
    /// de code ne peut lire ni écrire — le défaut sûr.
    #[serde(default)]
    pub workspace: Option<crate::backend_code::WorkspaceConfig>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingService {
    /// Vide ou absente : l'adresse vient de `RAG3WEAVER_EMBED_SERVICE`
    /// (`DaemonEmbedder::from_service`), choisie par le modèle demandé.
    #[serde(default)]
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
    /// La même chose, dans la déclaration commune.
    pub fn source(&self) -> crate::model_source::ModelSource {
        use crate::model_source::{Addresses, Fallback, ModelSource, Provider};
        ModelSource {
            provider: match self.provider {
                EmbeddingProvider::Daemon => Provider::Service,
                EmbeddingProvider::Compatible => Provider::Compatible,
            },
            model: self.model.clone(),
            address: Addresses::parse(&self.address),
            protocol: None,
            api_key_env: self.api_key_env.clone(),
            fallback: Fallback::Refuse,
            dimensions: Some(self.dimensions),
            context_tokens: None,
            params: Default::default(),
        }
    }
}

impl BackendManifest {
    /// **L'embarquement dense de ce backend** : `models.embed`, ou la section
    /// `embeddings` d'avant. Les deux à la fois, ou aucune, c'est une erreur
    /// dite au chargement.
    pub fn embed_source(&self) -> Result<crate::model_source::ModelSource, String> {
        use crate::model_source::Capability;
        match (self.models.get(&Capability::Embed), &self.embeddings) {
            (Some(_), Some(_)) => Err("`models.embed` et `embeddings` déclarent tous deux l'embarquement : gardez-en un".into()),
            (Some(source), None) => match source.dimensions {
                Some(_) => Ok(source.clone()),
                None => Err("`models.embed` doit déclarer `dimensions`".into()),
            },
            (None, Some(legacy)) => Ok(legacy.source()),
            (None, None) => Err("ce backend ne déclare pas d'embarquement : `models.embed` (ou `embeddings`) est requis".into()),
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
    /// Ce que cette entité **est**, pour un agent qui choisit ses outils —
    /// reprise dans la description générée de chaque outil qui la vise et en
    /// tête du schéma de ses filtres. La leçon du deck builder : tous les
    /// `search_*` disaient le même texte, l'agent choisissait au hasard du
    /// nom. Dire **quand** prendre l'outil, pas ce qu'il fait techniquement.
    #[serde(default)]
    pub description: Option<String>,
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
    /// Surcharge la ligne `%% description:` du gabarit — un produit change
    /// une phrase sans dupliquer un `.mmd`. Absente : le gabarit garde son
    /// texte.
    #[serde(default)]
    pub description: Option<String>,
    /// Ce que cet outil a le droit de faire (fichiers, commandes) — tout
    /// `false` par défaut : la base déclarative seule, comme avant. C'est la
    /// politique qui fait deux produits d'un même moteur.
    #[serde(default)]
    pub policy: crate::backend_code::ToolPolicy,
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
    /// **Le crochet après outil** : un petit graphe qui ajoute une section
    /// au rendu de cet outil — et qui a le droit de se taire. Proposition du
    /// 3 octobre 2026 (docs/3-octobre-2026-23h05/01), validée.
    #[serde(default)]
    pub after: Option<AfterToolHook>,
}

/// Une section au rendu d'un outil, depuis un petit graphe. Il ne peut ni
/// écrire, ni bloquer l'outil (toute erreur se journalise et se tait), ni
/// lancer quoi que ce soit ([`crate::backend_code::hook_nodes`]). Le silence
/// — section vide — est la conduite par défaut, et il s'observe : le journal
/// du backend (stderr) compte déclenché / tu / en erreur.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AfterToolHook {
    pub graph: PathBuf,
    /// Le titre de la section. Défaut : « À voir aussi ».
    #[serde(default = "titre_a_voir_aussi")]
    pub title: String,
    /// Le budget de lignes de la section ; au-delà, tronquée et avouée.
    #[serde(default = "douze_lignes")]
    pub max_lines: usize,
    /// Passé au gabarit s'il déclare `$threshold` — un seuil de similarité,
    /// calibré au banc comme les poids.
    #[serde(default)]
    pub threshold: Option<f64>,
    /// La base sans l'écriture, seule, sauf déclaration (`read_files` pour
    /// un crochet qui balaye). `run_commands` est ignoré : jamais dans un
    /// crochet.
    #[serde(default)]
    pub policy: crate::backend_code::ToolPolicy,
    /// **Se taire tant que les vecteurs de cette entité ne sont pas prêts.**
    /// Le client « motif ailleurs » cherche par similarité : pendant une
    /// indexation, il n'a rien d'honnête à dire — try_lock puis
    /// `index_state_for(entité)`, et tout ce qui n'est pas `Ready` (verrou
    /// occupé compris) est un silence, compté « tu ».
    #[serde(default)]
    pub silent_unless_vectors_ready: Option<String>,
    /// **Ce que l'outil a résolu, exposé au crochet** : le port de l'outil
    /// (ex. `{"node": "render", "port": "results"}`) dont les `uuid` des
    /// résultats sont passés au gabarit sous `$result_uuids` — s'il les
    /// déclare. Jamais de re-calcul : un crochet qui relancerait la
    /// recherche pour retrouver les identités en divergerait (règle de la
    /// session mémoire). Les pseudo-uuids du balayage (`scan:…`) sont
    /// écartés : ils ne désignent rien en base.
    #[serde(default)]
    pub results_port: Option<OutputPort>,
}

fn titre_a_voir_aussi() -> String {
    "À voir aussi".into()
}

fn douze_lignes() -> usize {
    12
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
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutputPort {
    pub node: String,
    pub port: String,
}

pub struct PreparedBackend {
    pub manifest: BackendManifest,
    /// L'embarquement dense, résolu au chargement (`BackendManifest::embed_source`).
    embed: crate::model_source::ModelSource,
    pub directory: PathBuf,
    entities: HashMap<String, EntityConfig>,
    mappings: HashMap<String, JsonSchemaMapping>,
    validators: HashMap<String, Arc<jsonschema::Validator>>,
    tools: BTreeMap<String, GraphTool>,
    nodes: NodeRegistry,
    tool_schemas: BTreeMap<String, Value>,
    tool_validators: HashMap<String, jsonschema::Validator>,
    harnesses: HashMap<String, PreparedHarness>,
    /// Les graphes des crochets après outil, chargés et validés au même
    /// moment que les outils qu'ils suivent.
    after_hooks: BTreeMap<String, GraphTool>,
    scripts: BTreeMap<String, String>,
    /// Les entités **décrites** que chaque outil vise (payloads, bindings,
    /// nœuds de sélection/recherche) : ce que `describe()` reprend.
    tool_entities: BTreeMap<String, BTreeSet<String>>,
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
        let embed = manifest.embed_source()?;
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
        if let Some(w) = &manifest.workspace {
            if let Some(nom) = &w.index {
                if !crate::backend_code::KNOWN_INDEX_SCHEMAS.contains(&nom.as_str()) {
                    return Err(format!(
                        "workspace.index : schéma « {nom} » inconnu — les schémas connus : {}",
                        crate::backend_code::KNOWN_INDEX_SCHEMAS.join(", ")
                    ));
                }
            }
            // **Les signaux montés sur le schéma nommé se valident ici, dans
            // les deux sens** : un signal qui demande un modèle refuse sans
            // lui, et un modèle que rien ne lit refuse aussi.
            if !w.index_signals.is_empty() && w.index.is_none() {
                return Err(
                    "workspace.index_signals sans workspace.index : les signaux se montent \
                     sur les entités d'un schéma nommé — déclarez l'index, ou retirez la clé"
                        .into(),
                );
            }
            let mut index_veut_creux = false;
            for (entite, signaux) in &w.index_signals {
                for signal in signaux {
                    match signal.as_str() {
                        "bm25" | "vector" => {}
                        "sparse" => index_veut_creux = true,
                        autre => {
                            return Err(format!(
                                "workspace.index_signals.{entite} : signal « {autre} » \
                                 inconnu (bm25, vector, sparse)"
                            ));
                        }
                    }
                }
            }
            // **`auto` sans bac à sable refuse** : la sentinelle peut accorder
            // librement, il faut que le noyau tienne — c'est la condition qui
            // rend ce mode honnête. Et le mode landlock se prouve au
            // chargement : un noyau qui ne sait pas le dit ici, pas au
            // milieu d'un tour d'agent.
            #[cfg(feature = "code")]
            {
                use crate::backend_code::{CommandGate, SandboxMode};
                if w.commands == CommandGate::Auto && w.sandbox.mode == SandboxMode::Off {
                    return Err(
                        "workspace.commands: \"auto\" exige un bac à sable — retirez \
                         \"sandbox\": {\"mode\": \"off\"}, ou passez en \"approval\""
                            .into(),
                    );
                }
                #[cfg(target_os = "linux")]
                if w.sandbox.mode == SandboxMode::Landlock {
                    if let Err(e) = crate::commande::BacASable::autour("/").ruleset() {
                        return Err(format!(
                            "le bac à sable Landlock ne se monte pas sur ce noyau ({e}) — \
                             déclarez \"sandbox\": {{\"mode\": \"off\"}} en le sachant"
                        ));
                    }
                }
                #[cfg(not(target_os = "linux"))]
                if w.sandbox.mode == SandboxMode::Landlock && w.commands != CommandGate::Off {
                    return Err(
                        "le bac à sable Landlock demande Linux — \"sandbox\": \
                         {\"mode\": \"off\"} en le sachant"
                            .into(),
                    );
                }
            }
            if index_veut_creux
                && !manifest.models.contains_key(&crate::model_source::Capability::Sparse)
            {
                return Err(
                    "workspace.index_signals déclare le signal sparse : `models.sparse` est \
                     requis (par exemple {\"provider\": \"service\", \"model\": \"bge-m3\"})"
                        .into(),
                );
            }
            if cfg!(not(feature = "code")) {
                return Err(
                    "la clé `workspace` demande un binaire bâti avec la feature `code` — \
                     rebâtir avec elle, ou retirer la clé du manifeste"
                        .into(),
                );
            }
            let root = if w.root.is_absolute() {
                w.root.clone()
            } else {
                directory.join(&w.root)
            };
            if !root.is_dir() {
                return Err(format!(
                    "workspace : la racine {} n'existe pas — donnez un dossier existant, relatif au manifeste ou absolu",
                    root.display()
                ));
            }
        }
        let (mut nodes, _) = builtin_graph_tools().map_err(|e| e.to_string())?;
        nodes.register(Box::new(crate::backend_nodes::EntityRecordFactory));
        nodes.register(Box::new(crate::backend_nodes::EntityBatchFactory));
        nodes.register(Box::new(crate::backend_nodes::SnapshotFinishFactory));
        nodes.register(Box::new(crate::backend_nodes::SnapshotUndoFactory));
        nodes.register(Box::new(crate::backend_nodes::SnapshotSessionFactory));
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
        let mut after_hooks = BTreeMap::new();
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
            if let Some(hook) = &attachment.after {
                // Le graphe du crochet se charge et se valide ICI : un
                // crochet mal déclaré est une erreur de manifeste, pas un
                // silence — le silence couvre l'exécution, jamais le montage.
                let source = std::fs::read_to_string(directory.join(&hook.graph))
                    .map_err(|e| format!("{name}: crochet after : {e}"))?;
                let hook_tool = GraphTool::from_mermaid(&source)
                    .and_then(|t| t.bind(&nodes))
                    .map_err(|e| format!("{name}: crochet after : {e}"))?;
                let permis = crate::backend_code::hook_nodes(&hook.policy);
                for node in &hook_tool.template().nodes {
                    if !permis.contains(&node.node_type.as_str()) {
                        return Err(format!(
                            "{name}: crochet after : le nœud {} n'entre pas dans un \
                             crochet — un crochet n'écrit pas, ne bloque pas, ne lance \
                             rien ; pour lire des fichiers, ajoutez \"policy\": \
                             {{\"read_files\": true}} au crochet",
                            node.node_type
                        ));
                    }
                }
                if hook.max_lines == 0 {
                    return Err(format!("{name}: crochet after : max_lines doit être au moins 1"));
                }
                let veut_uuids = hook_tool.params().iter().any(|p| p.name == "result_uuids");
                if veut_uuids && hook.results_port.is_none() {
                    return Err(format!(
                        "{name}: crochet after : le gabarit déclare result_uuids — dites d'où \
                         ils viennent : \"results_port\": {{\"node\": \"render\", \
                         \"port\": \"results\"}}"
                    ));
                }
                if !veut_uuids && hook.results_port.is_some() {
                    return Err(format!(
                        "{name}: crochet after : results_port déclaré mais le gabarit ne \
                         demande pas result_uuids — retirez l'un ou ajoutez l'autre"
                    ));
                }
                after_hooks.insert(name.clone(), hook_tool);
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
            // **Les filtres et options de recherche, typés depuis l'entité.** Le
            // graphe dit quel paramètre alimente un filtre ou des options, et
            // sur quelle entité (littérale ou liée) : le schéma, la grammaire,
            // les champs, leur vocabulaire et un exemple en découlent, pour
            // n'importe quel backend. Le validateur plus bas les applique.
            for node in &tool.template().nodes {
                let (cle_param, cle_entite, options) = match node.node_type.as_str() {
                    "SelectRecordsNode" => ("filter", "entity", false),
                    "SearchSourceNode" => ("options", "target_name", true),
                    _ => continue,
                };
                let Some(param) = node.config.get(cle_param).and_then(Value::as_str).and_then(|v| v.strip_prefix('$')) else {
                    continue;
                };
                if input["properties"].get(param).is_none() {
                    continue;
                }
                let entite = match node.config.get(cle_entite).and_then(Value::as_str) {
                    Some(v) => match v.strip_prefix('$') {
                        Some(lie) => attachment.bindings.get(lie).and_then(Value::as_str).map(str::to_string),
                        None => Some(v.to_string()),
                    },
                    None => None,
                };
                let Some(entite) = entite else {
                    continue;
                };
                let Some(mapping) = mappings.get(&entite) else {
                    continue;
                };
                let mut description = crate::json_schema::filter_description(&mapping.filter_fields);
                if let Some(d) = manifest.entities.get(&entite).and_then(|b| b.description.as_deref()) {
                    description = format!("{d} — {description}");
                }
                input["properties"][param] = if options {
                    crate::json_schema::search_options_schema(&description)
                } else {
                    let mut filtre = crate::json_schema::filter_condition_schema();
                    filtre["description"] = Value::String(description);
                    filtre["default"] = json!({});
                    filtre
                };
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
        // La politique de chaque outil se vérifie au chargement : un graphe
        // qui déborde sa politique, ou une politique sans le workspace
        // qu'elle exige, se refuse en disant quoi faire.
        for (name, attachment) in &manifest.tools {
            if let Some(tool) = tools.get(name) {
                crate::backend_code::validate_tool_policy(
                    name,
                    tool.template().nodes.iter().map(|n| n.node_type.clone()),
                    &attachment.policy,
                    manifest.workspace.as_ref(),
                )?;
            }
        }
        // Les entités décrites que chaque outil vise : par ses payloads, par
        // un binding qui nomme une entité, ou par ses nœuds de sélection et
        // de recherche (entité littérale ou liée). `describe()` les reprend.
        let mut tool_entities: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (name, attachment) in &manifest.tools {
            let mut vues: BTreeSet<String> = attachment
                .input_payloads
                .values()
                .map(|p| p.entity.clone())
                .collect();
            for v in attachment.bindings.values() {
                if let Some(e) = v.as_str() {
                    if manifest.entities.contains_key(e) {
                        vues.insert(e.to_string());
                    }
                }
            }
            if let Some(tool) = tools.get(name) {
                for node in &tool.template().nodes {
                    let cle = match node.node_type.as_str() {
                        "SelectRecordsNode" => "entity",
                        "SearchSourceNode" => "target_name",
                        _ => continue,
                    };
                    let entite = match node.config.get(cle).and_then(Value::as_str) {
                        Some(v) => match v.strip_prefix('$') {
                            Some(lie) => attachment.bindings.get(lie).and_then(Value::as_str).map(str::to_string),
                            None => Some(v.to_string()),
                        },
                        None => None,
                    };
                    if let Some(e) = entite {
                        if manifest.entities.contains_key(&e) {
                            vues.insert(e);
                        }
                    }
                }
            }
            vues.retain(|e| manifest.entities[e].description.is_some());
            if !vues.is_empty() {
                tool_entities.insert(name.clone(), vues);
            }
        }
        Ok(Self {
            manifest,
            embed,
            directory,
            entities,
            mappings,
            validators,
            tools,
            after_hooks,
            nodes,
            tool_schemas,
            tool_validators,
            harnesses,
            scripts,
            tool_entities,
        })
    }
    /// Ce backend a-t-il besoin d'un service d'embarquement ? Oui dès
    /// qu'une entité déclare un signal vecteur ou sparse, ou qu'un
    /// workspace nomme le schéma de code (dont les scopes s'embarquent).
    /// Un backend en mots seuls démarre sans service — et son banc tourne
    /// sans carte.
    pub fn needs_embeddings(&self) -> bool {
        if self
            .manifest
            .workspace
            .as_ref()
            .and_then(|w| w.index.as_deref())
            .is_some()
        {
            return true;
        }
        self.entities
            .values()
            .any(|config| config.signals.vector() || config.signals.sparse())
    }

    pub fn path(&self, path: &Path) -> PathBuf {
        self.directory.join(path)
    }
    pub fn describe(&self) -> Value {
        let mut tools:Vec<Value>=self.tools.iter().map(|(name,t)|{
            // La surcharge du manifeste, sinon la ligne du gabarit ; puis la
            // description de chaque entité visée — c'est elle qui distingue
            // deux outils de même forme, et l'agent choisit sur elle.
            let attachment = &self.manifest.tools[name];
            let mut description = attachment
                .description
                .clone()
                .unwrap_or_else(|| t.tool_def().description);
            if let Some(vues) = self.tool_entities.get(name) {
                for e in vues {
                    if let Some(d) = self.manifest.entities[e].description.as_deref() {
                        description.push_str(&format!(" Cible « {e} » : {d}"));
                    }
                }
            }
            json!({"name":name,"description":description,"inputSchema":self.tool_schemas[name]})
        }).collect();
        if self.manifest.search_graphs {
            tools.extend(search_tool_definitions());
        }
        json!({"name":self.manifest.name,"version":self.manifest.version,"database":self.path(&self.manifest.database),"fts":"lucivy","embeddings":self.embed,"payloads":self.mappings,"tools":tools,"capabilities":["journal","journal_read"]})
    }
    /// La déclaration de l'embarquement dense de ce backend.
    pub fn embed_source(&self) -> &crate::model_source::ModelSource {
        &self.embed
    }
    fn embed_dimensions(&self) -> usize {
        self.embed.dimensions.unwrap_or(0)
    }
    /// L'embarqueur que la déclaration désigne, et d'où il calcule.
    pub fn connect_embedder(&self) -> Result<(Box<dyn Embedder>, crate::model_source::Origin), String> {
        crate::model_source::connect_embedder(&self.embed)
    }
    pub fn open(
        self,
        conn: Box<dyn DbConnection>,
        embedder: Option<Box<dyn Embedder>>,
    ) -> Result<Backend, String> {
        // Un backend en mots seuls démarre sans service d'embarquement ;
        // un backend qui déclare du vecteur le refuse en le nommant.
        let sans_service = embedder.is_none();
        if sans_service && self.needs_embeddings() {
            return Err(
                "ce backend déclare des signaux vecteur ou sparse (ou un workspace indexé) : \
                 un service d'embarquement est requis — donnez models.embed.address \
                 (ou embeddings.address), ou RAG3WEAVER_SERVICE_EMBED (alias RAG3WEAVER_EMBED_SERVICE)"
                    .into(),
            );
        }
        let embedder = match embedder {
            Some(e) => {
                if e.is_mock()
                    || e.name() != self.embed.model
                    || e.dim() != self.embed_dimensions()
                {
                    return Err("embedding model identity differs from manifest".into());
                }
                e
            }
            None => Box::new(crate::embedder::MockEmbedder::new(self.embed_dimensions())),
        };
        let mut cat = Catalog::new(
            conn,
            embedder,
            CatalogConfig {
                name: Some(self.manifest.name.clone()),
                embedding_dim: self.embed_dimensions(),
                allow_mock_embedder: sans_service,
                checkpoint_dir: Some(
                    self.path(&self.manifest.database)
                        .with_extension("checkpoints"),
                ),
                ..Default::default()
            },
        );
        cat.set_moteur_texte(MoteurTexte::Lucivy);
        cat.set_fts_positions(self.manifest.fts_positions.unwrap_or(true));
        // **Le creux, enfin branché.** Le backend comptait le signal `sparse`
        // pour exiger un service, puis ne donnait jamais d'embarqueur creux
        // au catalogue (trouvé le 3 octobre 2026) : le signal restait muet.
        // Déclaré sans `models.sparse`, c'est maintenant un refus qui dit
        // quoi écrire ; et le dual n'est pris que si c'est le même modèle des
        // deux côtés — sinon dense et creux viendraient de deux espaces.
        // Le relecteur et l'OCR : les nœuds existaient (`RerankNode`,
        // `OcrNode`), le backend ne leur donnait aucun service. Déclarés, ils
        // sont joints ici ; non déclarés, rien ne change.
        if let Some(source) = self.manifest.models.get(&crate::model_source::Capability::Rerank) {
            cat.set_reranker(crate::model_source::connect_reranker(source)?.0);
        }
        if let Some(source) = self.manifest.models.get(&crate::model_source::Capability::Ocr) {
            cat.set_ocr(crate::model_source::connect_ocr(source)?.0);
        }
        let veut_le_creux = self.entities.values().any(|config| config.signals.sparse())
            || self
                .manifest
                .workspace
                .as_ref()
                .is_some_and(|w| w.index_signals.values().flatten().any(|s| s == "sparse"));
        // Le sens inverse : un modèle creux que rien ne lit est une
        // déclaration morte — la famille « écrire ce que rien ne lira ».
        if self.manifest.models.contains_key(&crate::model_source::Capability::Sparse)
            && !veut_le_creux
        {
            return Err(
                "models.sparse est déclaré mais aucun signal sparse ne le lit — montez-le \
                 sur une entité (signals) ou sur le schéma nommé (workspace.index_signals)"
                    .into(),
            );
        }
        match (self.manifest.models.get(&crate::model_source::Capability::Sparse), veut_le_creux) {
            (None, true) if !sans_service => {
                return Err("ce backend déclare un signal sparse : `models.sparse` est requis \
                            (par exemple {\"provider\": \"service\", \"model\": \"bge-m3\"})"
                    .into());
            }
            (Some(source), _) => {
                let (clients, _) = crate::model_source::connect_sparse(source)?;
                cat.set_sparse_embedder(clients.sparse);
                if let Some(dual) = clients.dual.filter(|_| source.model == self.embed.model) {
                    cat.set_dual_embedder(dual);
                }
            }
            (None, _) => {}
        }
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
        // Le schéma **nommé** du workspace, par le moteur : la vérité d'un
        // schéma ne se duplique pas en JSON (elle dériverait) — elle se
        // nomme, et le moteur l'enregistre.
        #[cfg(feature = "code")]
        if let Some("code") = self
            .manifest
            .workspace
            .as_ref()
            .and_then(|w| w.index.as_deref())
        {
            crate::code::register_code_schema(&mut cat, crate::code::default_scope_chunking())
                .map_err(|e| e.to_string())?;
            // Les signaux déclarés par le manifeste montent sur les entités
            // du schéma — la déclaration du moteur reste neutre, chaque
            // produit choisit. Une entité inconnue refuse en listant.
            if let Some(w) = &self.manifest.workspace {
                for (entite, signaux) in &w.index_signals {
                    let Some(config) = cat.entity_configs().get(entite.as_str()).cloned() else {
                        let connues: Vec<String> =
                            cat.entity_configs().keys().map(|k| k.to_string()).collect();
                        return Err(format!(
                            "workspace.index_signals : « {entite} » n'est pas une entité du \
                             schéma « {} » — les entités : {}",
                            w.index.as_deref().unwrap_or("?"),
                            connues.join(", ")
                        ));
                    };
                    let mut config = config;
                    let mut s = crate::search::SearchSignals::NONE;
                    for signal in signaux {
                        s = s | match signal.as_str() {
                            "bm25" => crate::search::SearchSignals::BM25,
                            "vector" => crate::search::SearchSignals::VECTOR,
                            "sparse" => crate::search::SearchSignals::SPARSE,
                            _ => unreachable!("validé au chargement"),
                        };
                    }
                    config.signals = s;
                    cat.register_entity(entite, config).map_err(|e| e.to_string())?;
                }
            }
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
        #[cfg(feature = "code")]
        let (file_source, garde) = match &self.manifest.workspace {
            Some(w) => (
                Some(crate::backend_code::build_source(w, &self.directory, &self.manifest.name)?),
                crate::backend_code::build_garde(w),
            ),
            None => (None, None),
        };
        // Un modèle de décision déclaré se joint à l'ouverture : un service
        // injoignable se dit ici, pas au premier outil qui s'en sert.
        let decider = match self.manifest.models.get(&crate::model_source::Capability::Decide) {
            Some(source) => Some(crate::decider::connect_decider(source)?.0),
            None => None,
        };
        Ok(Backend {
            decider,
            prepared: self,
            catalog: Arc::new(Mutex::new(cat)),
            #[cfg(feature = "code")]
            file_source,
            #[cfg(feature = "code")]
            garde,
        })
    }
}

/// **Ce qu'un outil à harnais rend à lire**, pour tous les transports (MCP,
/// chat, API). Sa sortie utile vit dans la livraison (`on_accept`) — le texte
/// d'import d'un deck — et aucun client ne savait la montrer : le modèle
/// recevait le reçu en JSON, avec une chaîne échappée qu'il ne recopiait pas
/// (27 septembre 2026). Accepté : les textes livrés tels quels ; refusé : les
/// erreurs ; les avertissements en clair dessous. Une présentation déjà posée
/// par le graphe de l'outil n'est pas touchée.
fn harness_presentation(response: &mut Value) {
    let Some(validation) = response.get("validation").cloned() else { return };
    if response.get("presentation").is_some_and(|p| p.is_string()) {
        return;
    }
    let issues = |key: &str, label: &str| -> Vec<String> {
        validation[key].as_array().into_iter().flatten()
            .map(|i| format!("- {label} {} : {}", i["code"].as_str().unwrap_or("?"), i["message"].as_str().unwrap_or("")))
            .collect()
    };
    let mut parts: Vec<String> = Vec::new();
    if validation["accepted"] == false {
        parts.push("Refusé par la validation : corriger chaque erreur et soumettre à nouveau.".into());
        parts.extend(issues("errors", "erreur"));
    } else if response["delivery"]["ok"] == false {
        parts.push(format!("Accepté, mais la livraison a échoué : {}", response["delivery"]["error"]));
    } else {
        let delivered: Vec<&str> = response["delivery"]["results"].as_array().into_iter().flatten()
            .filter_map(Value::as_str).collect();
        if delivered.is_empty() {
            return;
        }
        parts.push(delivered.join("\n\n"));
    }
    parts.extend(issues("warnings", "avertissement"));
    response["presentation"] = Value::String(parts.join("\n"));
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
    /// La source de fichiers du `workspace` déclaré — construite à
    /// l'ouverture (un instantané lit l'arbre une fois).
    #[cfg(feature = "code")]
    file_source: Option<Arc<dyn crate::code_tools::FileSource>>,
    /// La porte des commandes déclarée ; absente, `run` refuse tout.
    #[cfg(feature = "code")]
    garde: Option<Arc<crate::commande::Garde>>,
    /// Le modèle de décision déclaré (`models.decide`), donné aux nœuds sous
    /// la clé `crate::decider::DECIDER_SERVICE`.
    decider: Option<Arc<dyn crate::decider::Decider>>,
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
        let mut response = self.call_tool(name, args)?;
        harness_presentation(&mut response);
        Ok(response)
    }

    fn call_tool(&self, name: &str, args: Value) -> Result<Value, String> {
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
        let args_for_hook = args.clone();
        let def = tool
            .instantiate(&Value::Object(args))
            .map_err(|e| e.to_string())?;
        // La politique **déclarée** de l'outil — vérifiée au chargement, elle
        // tient ici le runtime : la base déclarative, plus ce que l'outil a
        // déclaré (fichiers, commandes).
        let policy = NodeTypePolicy::only(
            crate::backend_code::allowed_nodes(&attachment.policy),
        );
        // Le crochet qui veut les uuids des résultats les capture au passage
        // de l'outil, par un port de métadonnées de plus — jamais un
        // re-calcul.
        let mut metadata_ports = attachment.metadata.clone();
        if let Some(port) = attachment.after.as_ref().and_then(|h| h.results_port.as_ref()) {
            if !metadata_ports.iter().any(|p| p.node == port.node && p.port == port.port) {
                metadata_ports.push(port.clone());
            }
        }
        let mut response = self.execute_graph(tool, &def, &metadata_ports, &policy)?;
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
        if attachment.after.is_some() {
            self.enrich_with_after_hook(name, &args_for_hook, &mut response);
        }
        Ok(response)
    }

    /// **Le crochet après outil** : monte son graphe avec les mêmes arguments
    /// que l'outil (plus `threshold` s'il est déclaré), et ajoute sa section
    /// au rendu — ou se tait. Toute erreur se journalise et se tait aussi :
    /// le résultat de l'outil part toujours, entier, inchangé. Le journal du
    /// backend (stderr) rend le silence observable : déclenché / tu / en
    /// erreur — sans lui, le défaut sûr serait un défaut invisible.
    fn enrich_with_after_hook(
        &self,
        tool_name: &str,
        args: &serde_json::Map<String, Value>,
        response: &mut Value,
    ) {
        let Some(hook) = self
            .prepared
            .manifest
            .tools
            .get(tool_name)
            .and_then(|a| a.after.as_ref())
        else {
            return;
        };
        let Some(graph) = self.prepared.after_hooks.get(tool_name) else {
            return;
        };
        if let Some(entite) = &hook.silent_unless_vectors_ready {
            let prets = self
                .catalog
                .try_lock()
                .ok()
                .and_then(|c| c.index_state_for(entite).ok())
                .is_some_and(|s| s.vectors == crate::catalog::Level::Ready);
            if !prets {
                eprintln!("[crochet {tool_name}] tu — les vecteurs de {entite} ne sont pas prêts");
                return;
            }
        }
        // Le crochet ne reçoit que ce que son gabarit déclare : les
        // arguments de l'outil qui l'intéressent, et le seuil s'il le
        // demande — `instantiate` refuse l'inconnu, à raison.
        let declares: std::collections::HashSet<&str> =
            graph.params().iter().map(|p| p.name.as_ref()).collect();
        let mut hargs: serde_json::Map<String, Value> = args
            .iter()
            .filter(|(k, _)| declares.contains(k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if declares.contains("threshold") {
            if let Some(t) = hook.threshold {
                hargs.entry("threshold".to_string()).or_insert(json!(t));
            }
        }
        if declares.contains("result_uuids") {
            if let Some(port) = &hook.results_port {
                let cle = format!("{}.{}", port.node, port.port);
                let uuids: Vec<String> = response["metadata"][&cle]
                    .as_array()
                    .map(|rs| {
                        rs.iter()
                            .filter_map(|r| r["uuid"].as_str())
                            .filter(|u| !u.starts_with("scan:"))
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();
                hargs.insert("result_uuids".to_string(), json!(uuids));
                // La métadonnée empruntée pour le crochet ne reste dans la
                // réponse que si l'outil la déclarait lui-même.
                let declaree = self
                    .prepared
                    .manifest
                    .tools
                    .get(tool_name)
                    .is_some_and(|a| a.metadata.iter().any(|p| p.node == port.node && p.port == port.port));
                if !declaree {
                    response["metadata"].as_object_mut().map(|m| m.remove(&cle));
                }
            }
        }
        // Un paramètre du gabarit absent des arguments garde son défaut ;
        // un requis manquant échoue ici — compté, jamais bloquant.
        let outcome = graph
            .instantiate(&Value::Object(hargs))
            .map_err(|e| e.to_string())
            .and_then(|def| {
                self.execute_graph(
                    graph,
                    &def,
                    &[],
                    &NodeTypePolicy::only(crate::backend_code::hook_nodes(&hook.policy)),
                )
            });
        match outcome {
            Ok(hresp) => {
                let texte = hresp["result"].as_str().unwrap_or_default().trim().to_string();
                if texte.is_empty() {
                    eprintln!("[crochet {tool_name}] tu — rien à dire");
                    return;
                }
                let lignes: Vec<&str> = texte.lines().collect();
                let coupees = lignes.len().saturating_sub(hook.max_lines);
                let mut section = lignes
                    .into_iter()
                    .take(hook.max_lines)
                    .collect::<Vec<_>>()
                    .join("\n");
                if coupees > 0 {
                    // Tronqué, et avoué — jamais une troncature muette.
                    section.push_str(&format!("\n_… et {coupees} lignes de plus._"));
                }
                eprintln!("[crochet {tool_name}] déclenché — {} lignes", hook.max_lines.min(texte.lines().count()));
                match response.get_mut("result") {
                    // Le rendu markdown gagne sa section à la suite.
                    Some(Value::String(r)) => {
                        r.push_str(&format!("\n\n### {}\n{}", hook.title, section));
                    }
                    // Un résultat structuré la porte à côté, sans le toucher.
                    _ => {
                        response["after"] = json!({"title": hook.title, "text": section});
                    }
                }
            }
            Err(e) => {
                eprintln!("[crochet {tool_name}] en erreur (l'outil répond quand même) : {e}");
            }
        }
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
        if let Some(decider) = &self.decider {
            services.register(crate::decider::DECIDER_SERVICE, decider.clone());
        }
        // La surface de code déclarée : la source et la porte du manifeste,
        // montées comme sur le chemin agent — sans elles, les nœuds de code
        // refusent d'eux-mêmes.
        #[cfg(feature = "code")]
        {
            if let Some(source) = &self.file_source {
                services.register::<Arc<dyn crate::code_tools::FileSource>>(
                    crate::code_tools::FILE_SOURCE_SERVICE,
                    source.clone(),
                );
                if let Some(w) = &self.prepared.manifest.workspace {
                    services.register(crate::generated::GENERATED_POLICY_SERVICE, w.generated.clone());
                }
            }
            if let Some(garde) = &self.garde {
                services.register(crate::dataflow::run_nodes::GARDE_SERVICE, garde.clone());
                if let Some(w) = &self.prepared.manifest.workspace {
                    let racine = if w.root.is_absolute() {
                        w.root.clone()
                    } else {
                        self.prepared.directory.join(&w.root)
                    };
                    if let Some(bac) = crate::backend_code::build_bac_a_sable(&w.sandbox, &racine) {
                        services.register(
                            crate::dataflow::run_nodes::BAC_A_SABLE_SERVICE,
                            std::sync::Arc::new(bac),
                        );
                    }
                }
            }
        }
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
    /// **Le journal d'une conversation, écrit au fil de l'eau** (27 septembre
    /// 2026). L'hôte (pas l'agent : ce n'est pas un outil) envoie chaque
    /// événement dès qu'il arrive, avec l'instant où il est arrivé : `RunStarted`,
    /// `Message` — le format de [`crate::dataflow::trace_nodes::record_runs_and_messages`].
    /// Ils deviennent `Run`, `Message`, `Conversation`, `Participant` liés :
    /// cherchables comme le reste, et relisibles par un agent plus tard.
    pub fn journal(&self, events: &[Value]) -> Result<Value, String> {
        let mut cat = self.catalog.lock().map_err(|_| "catalog lock poisoned")?;
        crate::dataflow::trace_nodes::register_trace_schema(&mut cat).map_err(|e| e.to_string())?;
        let (mut runs, mut messages) = (0, 0);
        for event in events {
            // Chaque événement garde **son** instant : un lot ne les aplatit pas.
            let at_ms = event.get("at_ms").and_then(Value::as_i64).ok_or("journal event without at_ms")?;
            let (r, m) = crate::dataflow::trace_nodes::record_runs_and_messages(&mut cat, std::slice::from_ref(event), at_ms)?;
            runs += r;
            messages += m;
        }
        Ok(json!({"runs": runs, "messages": messages}))
    }

    /// Les messages d'un fil, dans l'ordre où ils sont arrivés (`at_ms`).
    pub fn journal_read(&self, conversation: &str, since_ms: i64) -> Result<Value, String> {
        let cat = self.catalog.lock().map_err(|_| "catalog lock poisoned")?;
        if !cat.is_registered_entity(crate::dataflow::trace_nodes::MESSAGE_ENTITY) {
            return Ok(json!({"messages": []}));
        }
        let rows = cat
            .execute_raw_with_params(
                "MATCH (m:Message)-[:IN_CONVERSATION]->(c:Conversation) \
                 WHERE c.conversation_id = $conversation AND m.at_ms >= $since \
                 RETURN m.at_ms, m.at, m.from, m.to, m.content ORDER BY m.at_ms, m.seq",
                &[
                    crate::connection::QueryParam::new("conversation", conversation),
                    crate::connection::QueryParam::new("since", since_ms),
                ],
            )
            .map_err(|e| e.to_string())?;
        let messages: Vec<Value> = rows
            .rows
            .iter()
            .map(|r| {
                let v = |i: usize| r.get(i).map(|c| serde_json::to_value(c).unwrap_or(Value::Null)).unwrap_or(Value::Null);
                json!({"at_ms": v(0), "at": v(1), "from": v(2), "to": v(3), "content": v(4)})
            })
            .collect();
        Ok(json!({"messages": messages}))
    }

    /// La base doit-elle être rouverte ? Voir [`Catalog::must_reopen`] ; un
    /// hôte le demande après chaque requête.
    pub fn must_reopen(&self) -> Option<String> {
        self.catalog.lock().unwrap_or_else(|p| p.into_inner()).must_reopen()
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
            decider: None,
            prepared,
            catalog: Arc::new(Mutex::new(catalog)),
            #[cfg(feature = "code")]
            file_source: None,
            #[cfg(feature = "code")]
            garde: None,
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

    /// Le backend notebook copié dans un dossier jetable, avec les
    /// descriptions du pas produit : l'entité `Note` en gagne une, l'outil
    /// `get_note` une surcharge — `put_note` garde le texte de son gabarit.
    fn fixture_avec_descriptions() -> (tempfile::TempDir, PreparedBackend) {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook");
        let dir = tempfile::tempdir().unwrap();
        fn copier(src: &Path, dst: &Path) {
            std::fs::create_dir_all(dst).unwrap();
            for entry in std::fs::read_dir(src).unwrap() {
                let entry = entry.unwrap();
                let cible = dst.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copier(&entry.path(), &cible);
                } else {
                    std::fs::copy(entry.path(), &cible).unwrap();
                }
            }
        }
        copier(&src, dir.path());
        // Le gabarit partagé `../../tools/search_structured.mmd` sort du
        // dossier copié : on le rapatrie et on pointe dessus.
        let partage = src.join("../../tools/search_structured.mmd");
        std::fs::copy(&partage, dir.path().join("search_structured.mmd")).unwrap();
        let chemin = dir.path().join("backend.json");
        let mut manifest: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
        manifest["tools"]["search_notes"]["graph"] = json!("search_structured.mmd");
        manifest["entities"]["Note"]["description"] = json!(
            "Une note de travail : un texte court sous une clé stable. À prendre quand la question porte sur ce que l'utilisateur a écrit."
        );
        manifest["tools"]["get_note"]["description"] = json!("Relit une note précise par sa clé exacte.");
        std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        let p = PreparedBackend::load(&chemin).unwrap();
        (dir, p)
    }

    /// **La description d'entité se reprend dans les outils** (la leçon du
    /// deck builder : deux outils de même forme ne se distinguent que par
    /// elle) ; la surcharge d'attachement prime sur le gabarit ; sans
    /// surcharge, le gabarit garde son texte ; et le schéma des options de
    /// recherche commence par l'entité.
    #[test]
    fn la_description_d_entite_se_reprend_dans_les_outils() {
        let (_garde, p) = fixture_avec_descriptions();
        let d = p.describe();
        let outil = |nom: &str| -> Value {
            d["tools"].as_array().unwrap().iter().find(|t| t["name"] == nom).unwrap().clone()
        };
        let put = outil("put_note");
        let texte = put["description"].as_str().unwrap();
        assert!(texte.contains("note de travail"), "l'entité visée se dit : {texte}");
        assert!(texte.contains("révision attendue"), "le texte du gabarit reste : {texte}");
        let get = outil("get_note");
        let texte = get["description"].as_str().unwrap();
        assert!(texte.starts_with("Relit une note précise"), "la surcharge prime : {texte}");
        assert!(texte.contains("note de travail"), "et l'entité se dit aussi : {texte}");
        let search = outil("search_notes");
        let texte = search["description"].as_str().unwrap();
        assert!(texte.contains("note de travail"), "le binding `target` suffit à viser : {texte}");
        let filtre = &search["inputSchema"]["properties"]["options"]["properties"]["filter_condition"]["description"];
        assert!(
            filtre.as_str().unwrap().starts_with("Une note de travail"),
            "le schéma du filtre commence par l'entité : {filtre}"
        );
    }

    /// Le notebook copié avec un workspace et les outils de code attachés —
    /// deux manifestes du même moteur, la politique seule les distingue.
    fn fixture_code(
        source: &str,
        read_only: bool,
        outils: Value,
    ) -> (tempfile::TempDir, Result<PreparedBackend, String>) {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook");
        let dir = tempfile::tempdir().unwrap();
        fn copier(src: &Path, dst: &Path) {
            std::fs::create_dir_all(dst).unwrap();
            for entry in std::fs::read_dir(src).unwrap() {
                let entry = entry.unwrap();
                let cible = dst.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copier(&entry.path(), &cible);
                } else {
                    std::fs::copy(entry.path(), &cible).unwrap();
                }
            }
        }
        copier(&src, dir.path());
        let outils_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/tools");
        for mmd in ["read.mmd", "edit.mmd", "run.mmd"] {
            std::fs::copy(outils_dir.join(mmd), dir.path().join(mmd)).unwrap();
        }
        std::fs::copy(src.join("../../tools/search_structured.mmd"), dir.path().join("search_structured.mmd")).unwrap();
        let sources = dir.path().join("sources");
        std::fs::create_dir(&sources).unwrap();
        std::fs::write(sources.join("main.rs"), "fn main() { depart(); }\n").unwrap();
        let chemin = dir.path().join("backend.json");
        let mut manifest: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
        manifest["tools"]["search_notes"]["graph"] = json!("search_structured.mmd");
        manifest["workspace"] = json!({"source": source, "root": "sources", "read_only": read_only});
        for (nom, attachement) in outils.as_object().unwrap() {
            manifest["tools"][nom] = attachement.clone();
        }
        std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        let p = PreparedBackend::load(&chemin);
        (dir, p)
    }

    /// [`fixture_code`], plus un patch libre du manifeste.
    fn fixture_code_avec(
        source: &str,
        read_only: bool,
        outils: Value,
        patch: &dyn Fn(&mut Value),
    ) -> (tempfile::TempDir, Result<PreparedBackend, String>) {
        let (dir, _) = fixture_code(source, read_only, outils);
        let chemin = dir.path().join("backend.json");
        let mut manifest: Value =
            serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
        patch(&mut manifest);
        std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        let p = PreparedBackend::load(&chemin);
        (dir, p)
    }

    /// Un Backend sans base réelle, comme `search_verbs_…` : le catalogue
    /// répond à vide, la source vient du manifeste — ce qu'on éprouve est la
    /// politique et le chemin des fichiers, pas la persistance.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    fn backend_de_code(dir: &Path, prepared: PreparedBackend) -> Backend {
        use crate::embedder::HashEmbedder;
        // Une VRAIE base en mémoire : depuis que `edit` réindexe par la
        // synchronisation déclarée, la session fine s'écrit puis se RELIT en
        // base — une connexion factice qui rend des résultats vides la
        // trouvait « fermée » (le rouge du 3 octobre au soir ; le scénario
        // de tuyauterie, sur une vraie base, passait). Trouvé par ce test.
        let conn = crate::Rag3dbConnection::in_memory().expect("base en mémoire");
        let root = std::env::var("RAG3DB_ROOT")
            .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").display().to_string());
        conn.execute(&format!(
            "LOAD EXTENSION '{root}/extension/vector/build/libvector.rag3db_extension'"
        ))
        .unwrap();
        let mut catalog = Catalog::new(
            Box::new(conn),
            Box::new(HashEmbedder::new(8)),
            CatalogConfig { allow_mock_embedder: true, embedding_dim: 8, ..Default::default() },
        );
        catalog.initialize().unwrap();
        // Les nœuds de code consultent File/Scope : le schéma de code
        // s'enregistre, la connexion à vide absorbe les DDL.
        crate::code::register_code_schema(&mut catalog, crate::code::default_scope_chunking()).unwrap();
        let workspace = prepared.manifest.workspace.clone().unwrap();
        let file_source =
            crate::backend_code::build_source(&workspace, dir, &prepared.manifest.name).unwrap();
        Backend {
            decider: None,
            prepared,
            catalog: Arc::new(Mutex::new(catalog)),
            file_source: Some(file_source),
            garde: crate::backend_code::build_garde(&workspace),
        }
    }

    /// **La politique refuse au chargement, en disant quoi faire** — la
    /// leçon du deck builder : nommer le paramètre à changer, pas la règle.
    #[test]
    fn la_politique_refuse_en_disant_quoi_faire() {
        // edit attaché sans write_files : le refus nomme la capacité.
        let (_d, p) = fixture_code("working_tree", false, json!({
            "edit_file": {"graph": "edit.mmd", "policy": {"read_files": true}}
        }));
        let erreur = p.err().unwrap();
        assert!(erreur.contains("write_files"), "{erreur}");
        assert!(erreur.contains("ajoutez"), "le refus dit quoi faire : {erreur}");

        // write_files sur un workspace read_only : incohérence dite au load.
        let (_d, p) = fixture_code("working_tree", true, json!({
            "edit_file": {"graph": "edit.mmd", "policy": {"write_files": true}}
        }));
        let erreur = p.err().unwrap();
        assert!(erreur.contains("read_only"), "{erreur}");

        // run sans porte : la clé à changer est nommée.
        let (_d, p) = fixture_code("working_tree", false, json!({
            "run_command": {"graph": "run.mmd", "policy": {"run_commands": true}}
        }));
        let erreur = p.err().unwrap();
        assert!(erreur.contains("workspace.commands"), "{erreur}");
    }

    /// **Le même moteur, deux politiques** : l'une lit un instantané sans
    /// rien pouvoir écrire, l'autre édite un arbre de travail — et le disque
    /// le prouve.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn le_meme_moteur_sous_deux_politiques() {
        // La politique cloud : un instantané, lecture seule.
        let (dir, p) = fixture_code("snapshot", true, json!({
            "read_file": {"graph": "read.mmd", "policy": {"read_files": true}}
        }));
        let backend = backend_de_code(dir.path(), p.unwrap());
        let lu = backend
            .call_tool("read_file", json!({"path": "main.rs"}))
            .unwrap();
        assert!(
            lu["result"].as_str().unwrap_or_default().contains("depart()"),
            "l'instantané se lit : {lu}"
        );

        // La politique de poste : l'arbre réel, l'édition permise.
        let (dir, p) = fixture_code("working_tree", false, json!({
            "read_file": {"graph": "read.mmd", "policy": {"read_files": true}},
            "edit_file": {"graph": "edit.mmd", "policy": {"write_files": true}}
        }));
        let backend = backend_de_code(dir.path(), p.unwrap());
        backend
            .call_tool("edit_file", json!({"path": "main.rs", "old": "depart()", "new": "arrivee()"}))
            .unwrap();
        assert!(
            std::fs::read_to_string(dir.path().join("sources/main.rs"))
                .unwrap()
                .contains("arrivee()"),
            "l'arbre de travail est édité sur le disque"
        );
    }

    /// **Un backend en mots seuls démarre sans service d'embarquement** —
    /// signalé par la session mémoire : connect était appelé à l'ouverture
    /// sans regarder les signaux, et une adresse injoignable empêchait un
    /// backend bm25-only de démarrer. Et l'inverse : un backend qui déclare
    /// du vecteur refuse l'absence de service en la nommant.
    #[cfg(feature = "rag3db-native")]
    #[test]
    fn un_backend_en_mots_seuls_demarre_sans_service() {
        // Le notebook, ramené aux mots seuls, avec une adresse injoignable.
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook");
        let dir = tempfile::tempdir().unwrap();
        fn copier(src: &Path, dst: &Path) {
            std::fs::create_dir_all(dst).unwrap();
            for entry in std::fs::read_dir(src).unwrap() {
                let entry = entry.unwrap();
                let cible = dst.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copier(&entry.path(), &cible);
                } else {
                    std::fs::copy(entry.path(), &cible).unwrap();
                }
            }
        }
        copier(&src, dir.path());
        std::fs::copy(
            src.join("../../tools/search_structured.mmd"),
            dir.path().join("search_structured.mmd"),
        )
        .unwrap();
        let chemin = dir.path().join("backend.json");
        let mut manifest: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
        manifest["tools"]["search_notes"]["graph"] = json!("search_structured.mmd");
        manifest["entities"]["Note"]["config"]["signals"] = json!(["bm25"]);
        manifest["embeddings"]["address"] = json!("127.0.0.1:9");
        let root = std::env::var("RAG3DB_ROOT")
            .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").display().to_string());
        manifest["vector_extension"] =
            json!(format!("{root}/extension/vector/build/libvector.rag3db_extension"));
        std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        let prepared = PreparedBackend::load(&chemin).unwrap();
        assert!(!prepared.needs_embeddings(), "aucun signal vecteur déclaré");
        let conn = crate::Rag3dbConnection::in_memory().expect("base en mémoire");
        let backend = prepared
            .open(Box::new(conn), None)
            .expect("un backend en mots seuls démarre sans service");
        let r = backend
            .call_tool("put_note", json!({"record": {"key": "k1", "text": "le four ne chauffe plus", "labels": [], "stage": "working"}}))
            .unwrap();
        assert!(r.get("ok").map(|v| v != false).unwrap_or(true), "{r}");

        // L'inverse : le notebook d'origine (vecteur déclaré) refuse en nommant.
        let prepared = PreparedBackend::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook/backend.json"),
        )
        .unwrap();
        assert!(prepared.needs_embeddings());
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let erreur = match prepared.open(Box::new(conn), None) {
            Err(e) => e,
            Ok(_) => panic!("du vecteur déclaré sans service doit se refuser"),
        };
        assert!(erreur.contains("embarquement"), "{erreur}");
        assert!(erreur.contains("RAG3WEAVER_EMBED_SERVICE"), "le refus dit quoi faire : {erreur}");
    }

    /// **Le crochet après outil** : la section s'ajoute au rendu, une
    /// erreur du crochet laisse l'outil entier, et un crochet qui demande
    /// un nœud interdit est refusé au chargement, en disant quoi faire.
    #[cfg(feature = "rag3db-native")]
    #[test]
    fn le_crochet_apres_outil_enrichit_se_tait_et_se_valide() {
        // Le montage notebook en mots seuls, comme le test des embarquements.
        fn montage(after: Value, gabarit: &str) -> (tempfile::TempDir, Result<PreparedBackend, String>) {
            let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook");
            let dir = tempfile::tempdir().unwrap();
            fn copier(src: &Path, dst: &Path) {
                std::fs::create_dir_all(dst).unwrap();
                for entry in std::fs::read_dir(src).unwrap() {
                    let entry = entry.unwrap();
                    let cible = dst.join(entry.file_name());
                    if entry.file_type().unwrap().is_dir() {
                        copier(&entry.path(), &cible);
                    } else {
                        std::fs::copy(entry.path(), &cible).unwrap();
                    }
                }
            }
            copier(&src, dir.path());
            std::fs::copy(
                src.join("../../tools/search_structured.mmd"),
                dir.path().join("search_structured.mmd"),
            )
            .unwrap();
            std::fs::write(dir.path().join("apres_essai.mmd"), gabarit).unwrap();
            let chemin = dir.path().join("backend.json");
            let mut manifest: Value =
                serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
            manifest["tools"]["search_notes"]["graph"] = json!("search_structured.mmd");
            manifest["entities"]["Note"]["config"]["signals"] = json!(["bm25"]);
            manifest["embeddings"]["address"] = json!("127.0.0.1:9");
            manifest["tools"]["put_note"]["after"] = after;
            let root = std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").display().to_string()
            });
            manifest["vector_extension"] =
                json!(format!("{root}/extension/vector/build/libvector.rag3db_extension"));
            std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
            let prepared = PreparedBackend::load(&chemin);
            (dir, prepared)
        }
        let carte = "%% tool: apres_essai\n%% description: essai\n%% result: schema.result\n\ngraph LR\n    schema[\"SchemaNode\"]\n";

        // (a) La section s'ajoute au rendu de l'outil.
        let (_d, prepared) = montage(json!({"graph": "apres_essai.mmd", "title": "À voir aussi"}), carte);
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let backend = prepared.unwrap().open(Box::new(conn), None).unwrap();
        let r = backend
            .call_tool("put_note", json!({"record": {"key": "k1", "text": "essai", "labels": [], "stage": "working"}}))
            .unwrap();
        let rendu = serde_json::to_string(&r).unwrap();
        assert!(rendu.contains("À voir aussi"), "la section du crochet manque : {rendu}");

        // (b) Un gabarit qui exige un paramètre jamais fourni : l'outil
        // répond entier, sans section — l'erreur se journalise et se tait.
        let exigeant = "%% tool: apres_essai\n%% description: essai\n%% param: inconnu string! -- jamais fourni\n%% result: schema.result\n\ngraph LR\n    schema[\"SchemaNode(target=$inconnu)\"]\n";
        let (_d, prepared) = montage(json!({"graph": "apres_essai.mmd"}), exigeant);
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let backend = prepared.unwrap().open(Box::new(conn), None).unwrap();
        let r = backend
            .call_tool("put_note", json!({"record": {"key": "k2", "text": "essai", "labels": [], "stage": "working"}}))
            .unwrap();
        let rendu = serde_json::to_string(&r).unwrap();
        assert!(r.get("delivery").is_some(), "l'outil répond entier : {rendu}");
        assert!(!rendu.contains("À voir aussi"), "pas de section sur une erreur : {rendu}");

        // (c) Un nœud interdit dans le crochet : refus au chargement, qui
        // dit quoi faire.
        let interdit = "%% tool: apres_essai\n%% description: essai\n%% result: run.result\n\ngraph LR\n    run[\"RunCommandNode(command='ls')\"]\n";
        let (_d, prepared) = montage(json!({"graph": "apres_essai.mmd"}), interdit);
        let erreur = prepared.err().expect("un crochet qui lance se refuse au chargement");
        assert!(erreur.contains("crochet"), "{erreur}");
        assert!(erreur.contains("RunCommandNode"), "le refus nomme le nœud : {erreur}");

        // (d) Un gabarit qui demande les uuids des résultats sans dire d'où
        // ils viennent : refus au chargement, qui nomme results_port.
        let avide = "%% tool: apres_essai\n%% description: essai\n%% param: result_uuids json! -- les identités résolues\n%% result: src.query\n\ngraph LR\n    src[\"KBQuerySourceNode(kb_name='k', query='q', options=$result_uuids)\"]\n";
        let (_d, prepared) = montage(json!({"graph": "apres_essai.mmd"}), avide);
        let erreur = prepared.err().expect("result_uuids sans results_port se refuse");
        assert!(erreur.contains("results_port"), "le refus dit quoi déclarer : {erreur}");

        // (e) La porte des vecteurs : rien n'est indexé pour Note, le
        // crochet se tait — même graphe que (a), qui parlait.
        let (_d, prepared) = montage(
            json!({"graph": "apres_essai.mmd", "silent_unless_vectors_ready": "Note"}),
            carte,
        );
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let backend = prepared.unwrap().open(Box::new(conn), None).unwrap();
        let r = backend
            .call_tool("put_note", json!({"record": {"key": "k3", "text": "essai", "labels": [], "stage": "working"}}))
            .unwrap();
        let rendu = serde_json::to_string(&r).unwrap();
        assert!(
            !rendu.contains("À voir aussi"),
            "vecteurs pas prêts : le crochet se tait : {rendu}"
        );
    }

    /// **Les signaux du schéma nommé se valident dans les deux sens** : le
    /// signal sans modèle, la clé sans index, le signal inconnu — chacun
    /// refuse au chargement en disant quoi faire.
    #[test]
    fn index_signals_se_valide_dans_les_deux_sens() {
        let (_d, p) = fixture_code("working_tree", false, json!({}));
        assert!(p.is_ok(), "le montage de base passe");

        // index_signals sans index.
        let patch = |m: &mut Value| {
            m["workspace"]["index_signals"] = json!({"Scope": ["sparse"]});
        };
        let (_d, p) = fixture_code_avec("working_tree", false, json!({}), &patch);
        let e = p.err().unwrap();
        assert!(e.contains("workspace.index"), "{e}");

        // sparse déclaré sans models.sparse.
        let patch = |m: &mut Value| {
            m["workspace"]["index"] = json!("code");
            m["workspace"]["index_signals"] = json!({"Scope": ["bm25", "vector", "sparse"]});
        };
        let (_d, p) = fixture_code_avec("working_tree", false, json!({}), &patch);
        let e = p.err().unwrap();
        assert!(e.contains("models.sparse"), "{e}");

        // signal inconnu.
        let patch = |m: &mut Value| {
            m["workspace"]["index"] = json!("code");
            m["workspace"]["index_signals"] = json!({"Scope": ["fuzzy"]});
        };
        let (_d, p) = fixture_code_avec("working_tree", false, json!({}), &patch);
        let e = p.err().unwrap();
        assert!(e.contains("fuzzy"), "{e}");
    }

    /// **`models.embed` et `embeddings` disent la même chose** : l'une ou
    /// l'autre, jamais les deux, et l'absence des deux se dit au chargement.
    #[test]
    fn l_embarquement_se_declare_par_models_embed_ou_par_embeddings() {
        use crate::model_source::{Capability, Provider};
        let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook/backend.json");
        let base: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();

        // La section d'avant : un démon, à l'adresse écrite.
        let avant: BackendManifest = serde_json::from_value(base.clone()).unwrap();
        let s = avant.embed_source().unwrap();
        assert_eq!((s.provider, s.model.as_str(), s.dimensions), (Provider::Service, "bge-m3", Some(1024)));
        assert_eq!(s.address.0, ["127.0.0.1:7878"]);

        // La même chose sous `models.embed`, sans adresse : elle viendra de l'environnement.
        let mut neuf = base.clone();
        neuf.as_object_mut().unwrap().remove("embeddings");
        neuf["models"] = json!({"embed": {"provider": "service", "model": "granite-278m", "dimensions": 768}});
        let neuf: BackendManifest = serde_json::from_value(neuf).unwrap();
        let s = neuf.embed_source().unwrap();
        assert_eq!((s.provider, s.model.as_str(), s.dimensions, s.address.is_empty()), (Provider::Service, "granite-278m", Some(768), true));
        assert!(neuf.models.contains_key(&Capability::Embed));

        // Les deux à la fois, ou aucune : une erreur qui dit laquelle.
        let mut deux = base.clone();
        deux["models"] = json!({"embed": {"model": "granite-278m", "dimensions": 768}});
        let e = serde_json::from_value::<BackendManifest>(deux).unwrap().embed_source().unwrap_err();
        assert!(e.contains("gardez-en un"), "{e}");
        let mut aucune = base.clone();
        aucune.as_object_mut().unwrap().remove("embeddings");
        let e = serde_json::from_value::<BackendManifest>(aucune).unwrap().embed_source().unwrap_err();
        assert!(e.contains("models.embed"), "{e}");
        // Une capacité inconnue est une faute de frappe, pas un modèle.
        let mut faute = base;
        faute["models"] = json!({"embedd": {"model": "x"}});
        assert!(serde_json::from_value::<BackendManifest>(faute).is_err());
    }

    /// Sans aucune description déclarée, rien ne change : les textes des
    /// gabarits se rendent tels quels — une base d'avant se relit et se
    /// décrit comme avant.
    #[test]
    fn sans_description_les_gabarits_gardent_leur_texte() {
        let p = PreparedBackend::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook/backend.json"),
        )
        .unwrap();
        let d = p.describe();
        let put = d["tools"].as_array().unwrap().iter().find(|t| t["name"] == "put_note").unwrap();
        let texte = put["description"].as_str().unwrap();
        assert!(texte.contains("révision attendue"), "{texte}");
        assert!(!texte.contains("Cible «"), "pas d'entité à dire : {texte}");
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
