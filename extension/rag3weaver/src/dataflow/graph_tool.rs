//! Graphes-outils : **un outil exposé à un LLM est un graphe entier, plus une
//! fiche d'identité.**
//!
//! Le doc 36 dit qu'un agent est un sous-graphe qui se compile en workflow.
//! Un outil en est le cas dégénéré : un sous-graphe dont on publie le nom, la
//! description, les paramètres typés et le port qui porte le résultat. La
//! plomberie interne (`FlushNode`, `SparseCommitNode`, un `ResolveParentNode`
//! qu'aucun modèle ne devrait avoir à câbler) cesse d'être un problème — elle
//! n'est plus la surface.
//!
//! ## La fiche, dans l'en-tête du fichier Mermaid
//!
//! ```text
//! %% tool: search
//! %% description: Cherche dans une entité ou une base de connaissances.
//! %% param: query string! -- Texte de la requête, en langue naturelle.
//! %% param: limit int = 10 -- Nombre maximum de résultats.
//! %% result: resolve.results
//!
//! graph LR
//!     source["SearchSourceNode(target_name=$target, query=$query)"]
//!     ...
//! ```
//!
//! Quatre directives, une par ligne, dans un commentaire `%%` :
//!
//! | directive | forme | rôle |
//! |---|---|---|
//! | `tool:` | `%% tool: <nom>` | le nom que le modèle appellera |
//! | `description:` | `%% description: <texte>` | répétable, les lignes sont jointes par une espace |
//! | `param:` | `%% param: <nom> <type>[!] [= <défaut JSON>] -- <description>` | un paramètre typé |
//! | `result:` | `%% result: <nœud>.<port>` | le port qui porte le résultat |
//!
//! Les types sont **le vocabulaire de [`ConfigParamType`]** — `string`, `int`,
//! `float`, `bool`, `json` — et pas un dialecte de plus. `!` marque un
//! paramètre requis ; `= <JSON>` un paramètre facultatif **avec** son défaut
//! (un facultatif sans défaut est refusé : son `$var` ne se résoudrait pas).
//! Le séparateur ` -- ` ouvre la description, ce qui laisse `:` et `=` libres
//! dans un défaut JSON comme dans une description.
//!
//! Toute autre ligne `%%` reste un commentaire libre : les gabarits de
//! `templates/` d'avant sont toujours valides, et un graphe-outil reste
//! lisible par [`parse_mermaid`](super::mermaid::parse_mermaid), qui ignore
//! déjà les `%%`.
//!
//! ## Où va un paramètre
//!
//! Dans la substitution `$nom` que [`parse_mermaid_template`] offrait déjà —
//! c'est la voie naturelle, et elle rend la liaison **visible dans le graphe
//! lui-même** plutôt que dans une table de correspondance à côté. Deux
//! renforts par rapport au mécanisme brut :
//!
//! 1. **La liaison est bijective et vérifiée.** Tout `$var` du graphe doit
//!    être un paramètre déclaré, et tout paramètre déclaré doit apparaître
//!    quelque part — un paramètre qui ne dit pas où il va est une erreur de
//!    fiche, pas un silence.
//! 2. **La substitution est typée.** `parse_mermaid_template` rend toujours
//!    une chaîne : `limit=$limit` donnait `"limit": "10"`, que
//!    `BM25SearchNodeFactory` lit avec `as_u64()` — donc `None`, donc le
//!    défaut, silencieusement. (C'est le cas de `templates/simple_*.mmd`
//!    aujourd'hui.) Ici le gabarit garde le `$var` tel quel dans la
//!    définition, et [`substitute_definition`] y injecte la **valeur JSON**
//!    de l'argument, entier compris.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::sync::Arc;

use serde_json::{Map, Value};

use crate::llm::{ToolCall, Turn};
use crate::tools::{params_object_schema, ToolDef};

use super::checkpoint::{port_value_to_checkpoint, GraphDefinition};
use super::graph::DataflowGraph;
use super::graph_node::GraphNodeFactory;
use super::mermaid::{parse_mermaid_template, to_mermaid};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeRegistry};
use super::port::{PortType, PortValue};
use super::runtime::DataflowRuntime;
use super::services::ServiceRegistry;

/// Garde-fou du moteur séquentiel — identique à celui des suites E2E.
const MAX_ITERATIONS: usize = 100;

// ─── Erreurs ────────────────────────────────────────────────────────────────

/// Ce qui peut échouer entre un `ToolCall` et un résultat d'outil.
///
/// Chaque variante porte un **code stable** ([`Self::kind`]) : c'est lui qui
/// part dans le résultat rendu au modèle, pour qu'un agent puisse distinguer
/// « j'ai mal appelé » de « l'outil a échoué » sans lire du français.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphToolError {
    /// Fiche incohérente (construction), jamais un appel.
    Spec(String),
    UnknownTool { name: String, known: Vec<String> },
    BadArgumentsJson(String),
    MissingArgument(String),
    UnknownArgument { name: String, known: Vec<String> },
    TypeMismatch {
        name: String,
        expected: &'static str,
        got: &'static str,
    },
    /// La valeur n'est pas dans la liste des valeurs admises du paramètre —
    /// une relation qui n'existe pas dans le schéma, une cible qui n'est ni
    /// une entité ni une base de connaissances. La liste est **dans**
    /// l'erreur : c'est ce qui permet au modèle de se corriger au tour
    /// suivant plutôt que d'inventer un autre nom.
    BadChoice {
        name: String,
        value: String,
        choices: Vec<String>,
    },
    /// Le graphe n'a pas pu être construit (port absent, type incompatible,
    /// entrée requise non connectée…). Le message vient de
    /// [`DataflowGraph::connect`] / [`DataflowGraph::validate`], qui nomment
    /// déjà le nœud et le port fautifs.
    Build(String),
    /// Un nœud du graphe désigne un type que le registre ne connaît pas.
    /// Nomme **l'instance** en plus du type — ce que
    /// `NodeRegistry::create` seul ne fait pas.
    UnknownNodeType { node: String, node_type: String },
    /// Un nœud du graphe désigne un type que la politique interdit.
    ForbiddenNodeType { node: String, node_type: String },
    /// Le graphe n'est pas un DAG. Nomme les nœuds pris dans le cycle —
    /// `topological_sort` dit seulement « cycle detected ».
    Cycle { nodes: Vec<String> },
    Execution(String),
    NoResult { node: String, port: String },
    Unserializable { node: String, port: String, detail: String },
    Panic(String),
}

impl GraphToolError {
    /// Code stable, destiné à être lu par un agent.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Spec(_) => "spec",
            Self::UnknownTool { .. } => "unknown_tool",
            Self::BadArgumentsJson(_) => "bad_arguments_json",
            Self::MissingArgument(_) => "missing_argument",
            Self::UnknownArgument { .. } => "unknown_argument",
            Self::TypeMismatch { .. } => "type_mismatch",
            Self::BadChoice { .. } => "bad_choice",
            Self::Build(_) => "build",
            Self::UnknownNodeType { .. } => "unknown_node_type",
            Self::ForbiddenNodeType { .. } => "forbidden_node_type",
            Self::Cycle { .. } => "cycle",
            Self::Execution(_) => "execution",
            Self::NoResult { .. } => "no_result",
            Self::Unserializable { .. } => "unserializable",
            Self::Panic(_) => "panic",
        }
    }

    /// Le résultat d'outil qu'un modèle reçoit à la place d'une réponse.
    ///
    /// Du JSON, comme [`crate::llm::INTERRUPTED_TOOL_RESULT`] : un agent le
    /// relit sans casser, et il voit tout de suite que l'appel a échoué.
    pub fn to_tool_json(&self) -> String {
        serde_json::json!({ "error": self.kind(), "detail": self.to_string() }).to_string()
    }
}

impl fmt::Display for GraphToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spec(d) => write!(f, "fiche d'outil invalide : {d}"),
            Self::UnknownTool { name, known } => {
                write!(f, "outil inconnu '{name}' ; connus : {}", known.join(", "))
            }
            Self::BadArgumentsJson(d) => write!(f, "arguments illisibles (JSON invalide) : {d}"),
            Self::MissingArgument(n) => write!(f, "argument requis manquant : '{n}'"),
            Self::UnknownArgument { name, known } => write!(
                f,
                "argument inconnu '{name}' ; attendus : {}",
                known.join(", ")
            ),
            Self::TypeMismatch { name, expected, got } => write!(
                f,
                "argument '{name}' : attendu {expected}, reçu {got}"
            ),
            Self::BadChoice { name, value, choices } => write!(
                f,
                "argument '{name}' : '{value}' n'est pas une valeur admise ; admises : {}",
                choices.join(", ")
            ),
            Self::Build(d) => write!(f, "le graphe n'a pas pu être construit : {d}"),
            Self::UnknownNodeType { node, node_type } => write!(
                f,
                "nœud '{node}' : type '{node_type}' inconnu du registre"
            ),
            Self::ForbiddenNodeType { node, node_type } => write!(
                f,
                "nœud '{node}' : le type '{node_type}' n'est pas autorisé ici"
            ),
            Self::Cycle { nodes } => write!(
                f,
                "le graphe contient un cycle ; nœuds impliqués : {}",
                nodes.join(", ")
            ),
            Self::Execution(d) => write!(f, "l'exécution du graphe a échoué : {d}"),
            Self::NoResult { node, port } => {
                write!(f, "aucune valeur sur le port de résultat '{node}.{port}'")
            }
            Self::Unserializable { node, port, detail } => write!(
                f,
                "le port de résultat '{node}.{port}' ne se sérialise pas : {detail}"
            ),
            Self::Panic(d) => write!(f, "l'outil a paniqué : {d}"),
        }
    }
}

impl std::error::Error for GraphToolError {}

// ─── Types de paramètres ────────────────────────────────────────────────────

/// Nom du type dans la fiche — le vocabulaire de [`ConfigParamType`].
pub fn param_type_name(t: &ConfigParamType) -> &'static str {
    match t {
        ConfigParamType::String => "string",
        ConfigParamType::Int => "int",
        ConfigParamType::Float => "float",
        ConfigParamType::Bool => "bool",
        ConfigParamType::Json => "json",
    }
}

fn param_type_from_name(s: &str) -> Option<ConfigParamType> {
    Some(match s {
        "string" => ConfigParamType::String,
        "int" => ConfigParamType::Int,
        "float" => ConfigParamType::Float,
        "bool" => ConfigParamType::Bool,
        "json" => ConfigParamType::Json,
        _ => return None,
    })
}

/// Nom JSON Schema de ce qu'on a **reçu**, pour un message d'erreur lisible.
fn json_kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) => {
            if n.is_f64() {
                "number"
            } else {
                "integer"
            }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Un argument est-il du type déclaré ?
///
/// Un entier passe pour un `float` (c'est ce que `number` autorise en JSON
/// Schema) ; l'inverse est refusé, parce que `10.5` dans un `limit` est une
/// erreur du modèle, pas une valeur à tronquer en silence.
fn value_matches(t: &ConfigParamType, v: &Value) -> bool {
    match t {
        ConfigParamType::String => v.is_string(),
        ConfigParamType::Int => v.is_i64() || v.is_u64(),
        ConfigParamType::Float => v.is_number(),
        ConfigParamType::Bool => v.is_boolean(),
        // `param_schema` annonce `type: object` pour `Json` ; on valide ce
        // qu'on annonce. (C'est la dette déjà nommée dans `tools.rs` : sans
        // sous-schéma, un objet libre ne borne rien.)
        ConfigParamType::Json => v.is_object(),
    }
}

// ─── Substitution ───────────────────────────────────────────────────────────

/// Les variables `$nom` présentes dans les configurations d'un graphe.
pub fn template_vars(def: &GraphDefinition) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for node in &def.nodes {
        scan_vars(&node.config, &mut out);
    }
    out
}

fn scan_vars(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::String(s) => {
            let mut rest = s.as_str();
            while let Some(pos) = rest.find('$') {
                rest = &rest[pos + 1..];
                let end = rest
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(rest.len());
                if end > 0 {
                    out.insert(rest[..end].to_string());
                }
                rest = &rest[end..];
            }
        }
        Value::Array(a) => a.iter().for_each(|x| scan_vars(x, out)),
        Value::Object(o) => o.values().for_each(|x| scan_vars(x, out)),
        _ => {}
    }
}

/// Remplace les `$nom` d'une définition gabarit par les valeurs de `args`.
///
/// Une chaîne qui vaut **exactement** `"$nom"` prend la valeur JSON de
/// l'argument, type compris (`limit=$limit` avec `10` donne bien `10`, pas
/// `"10"`). Une chaîne qui contient un `$nom` parmi d'autre texte
/// (`query='doc: $query'`) est interpolée textuellement.
pub fn substitute_definition(def: &GraphDefinition, args: &Map<String, Value>) -> GraphDefinition {
    let mut out = def.clone();
    for node in &mut out.nodes {
        node.config = substitute_value(&node.config, args);
    }
    out
}

fn substitute_value(v: &Value, args: &Map<String, Value>) -> Value {
    match v {
        Value::String(s) => {
            if let Some(name) = s.strip_prefix('$') {
                if !name.is_empty()
                    && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                {
                    if let Some(replacement) = args.get(name) {
                        return replacement.clone();
                    }
                }
            }
            Value::String(interpolate(s, args))
        }
        Value::Array(a) => Value::Array(a.iter().map(|x| substitute_value(x, args)).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, x)| (k.clone(), substitute_value(x, args)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn interpolate(s: &str, args: &Map<String, Value>) -> String {
    if !s.contains('$') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find('$') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];
        let end = after
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(after.len());
        if end == 0 {
            out.push('$');
            rest = after;
            continue;
        }
        match args.get(&after[..end]) {
            Some(Value::String(v)) => out.push_str(v),
            Some(other) => out.push_str(&other.to_string()),
            None => {
                out.push('$');
                out.push_str(&after[..end]);
            }
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// Valide `args` contre `params` et rend l'objet **complet** (défauts remplis).
///
/// Les trois erreurs d'appel sont ici, et elles sont franches : un argument
/// manquant, un argument en trop, un argument du mauvais type. Jamais un
/// graphe lancé à moitié.
pub fn resolve_params(
    params: &[ConfigParam],
    args: &Value,
) -> Result<Map<String, Value>, GraphToolError> {
    let known: Vec<String> = params.iter().map(|p| p.name.to_string()).collect();

    // `null` vaut « pas d'arguments » : certains fournisseurs l'émettent pour
    // un outil sans paramètre plutôt qu'un `{}`.
    let empty = Map::new();
    let given = match args {
        Value::Object(o) => o,
        Value::Null => &empty,
        other => {
            return Err(GraphToolError::BadArgumentsJson(format!(
                "objet attendu, reçu {}",
                json_kind(other)
            )))
        }
    };

    for key in given.keys() {
        if !params.iter().any(|p| p.name == key) {
            return Err(GraphToolError::UnknownArgument {
                name: key.clone(),
                known,
            });
        }
    }

    let mut out = Map::new();
    for p in params {
        match given.get(p.name) {
            Some(v) => {
                if !value_matches(&p.param_type, v) {
                    return Err(GraphToolError::TypeMismatch {
                        name: p.name.to_string(),
                        expected: param_type_name(&p.param_type),
                        got: json_kind(v),
                    });
                }
                out.insert(p.name.to_string(), v.clone());
            }
            None if p.required => {
                return Err(GraphToolError::MissingArgument(p.name.to_string()))
            }
            None => {
                if let Some(d) = &p.default {
                    out.insert(p.name.to_string(), d.clone());
                }
            }
        }
    }
    Ok(out)
}

// ─── Frontière de capacités ─────────────────────────────────────────────────

/// Quels types de nœuds un graphe a le droit d'instancier.
///
/// **Pourquoi ce crochet existe, alors qu'il laisse tout passer aujourd'hui.**
/// Les graphes-outils déclarés à la main sont écrits par nous : rien à borner.
/// Mais la suite prévue est un méta-outil où *le modèle compose le graphe*, et
/// là un graphe est du code : `DeleteRecordNode`, `UpdateRecordNode`,
/// `InsertRecordNode` écrivent dans la base de l'utilisateur, et `CypherNode`
/// exécute du Cypher arbitraire. Le point où l'on décide « ce graphe-là n'a
/// pas le droit d'écrire » doit exister **avant** qu'un graphe sorte d'un
/// modèle, sinon il n'existera jamais au bon endroit — c'est-à-dire entre la
/// définition et l'instanciation, ici.
///
/// Ce n'est pas la politique, c'est son emplacement. La politique (quels
/// nœuds, selon quoi, décidés par qui) est un chantier à part.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum NodeTypePolicy {
    /// Tout est permis — le défaut, et le bon défaut pour un graphe écrit à la main.
    #[default]
    All,
    /// Liste blanche de types de nœuds.
    Only(BTreeSet<String>),
}

impl NodeTypePolicy {
    pub fn all() -> Self {
        Self::All
    }

    pub fn only<S: Into<String>, I: IntoIterator<Item = S>>(types: I) -> Self {
        Self::Only(types.into_iter().map(Into::into).collect())
    }

    pub fn allows(&self, node_type: &str) -> bool {
        match self {
            Self::All => true,
            Self::Only(set) => set.contains(node_type),
        }
    }
}

/// Vérifie qu'aucun nœud de `def` n'emploie un type que `policy` interdit.
///
/// Séparé de toute construction : c'est *le* point de contrôle, et il doit
/// pouvoir être appelé (et testé) seul, sur un `GraphDefinition` d'où qu'il
/// vienne — y compris d'un modèle.
pub fn validate_node_types(
    def: &GraphDefinition,
    policy: &NodeTypePolicy,
) -> Result<(), GraphToolError> {
    for node in &def.nodes {
        if !policy.allows(&node.node_type) {
            return Err(GraphToolError::ForbiddenNodeType {
                node: node.name.clone(),
                node_type: node.node_type.clone(),
            });
        }
    }
    Ok(())
}

// ─── Exécuter un graphe, sans registre d'outils ─────────────────────────────

/// Les nœuds pris dans un cycle : ceux qu'un tri topologique n'atteint jamais.
///
/// `DataflowGraph::topological_sort` sait dire qu'il y a un cycle, pas où —
/// et « invalid graph » ne permet à personne de se rattraper. Refait ici sur
/// la **définition** (donc sans instancier quoi que ce soit) pour nommer les
/// coupables dans le message rendu au modèle.
fn cycle_nodes(def: &GraphDefinition) -> Vec<String> {
    let names: BTreeSet<&str> = def.nodes.iter().map(|n| n.name.as_str()).collect();
    let mut in_degree: BTreeMap<&str, usize> = names.iter().map(|n| (*n, 0)).collect();
    for e in &def.edges {
        if names.contains(e.to_node.as_str()) {
            *in_degree.entry(e.to_node.as_str()).or_default() += 1;
        }
    }
    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(n, _)| *n)
        .collect();
    let mut settled: BTreeSet<&str> = BTreeSet::new();
    while let Some(node) = queue.pop() {
        if !settled.insert(node) {
            continue;
        }
        for e in def.edges.iter().filter(|e| e.from_node == node) {
            if let Some(d) = in_degree.get_mut(e.to_node.as_str()) {
                *d -= 1;
                if *d == 0 {
                    queue.push(e.to_node.as_str());
                }
            }
        }
    }
    names
        .into_iter()
        .filter(|n| !settled.contains(n))
        .map(String::from)
        .collect()
}

/// Construit un graphe exécutable depuis une définition **quelconque**.
///
/// Aucune fiche, aucun nom, aucun registre d'outils : la définition peut
/// venir d'un `GraphTool` instancié comme d'un modèle qui l'a composée. Les
/// contrôles, dans l'ordre où ils rattrapent le plus tôt :
///
/// 1. la **politique de capacités** ([`validate_node_types`]) ;
/// 2. les types de nœuds connus — en nommant l'instance, ce que
///    `NodeRegistry::create` seul ne fait pas ;
/// 3. l'acyclicité, en nommant les nœuds du cycle ;
/// 4. la construction elle-même (ports, compatibilité, entrées requises),
///    dont les messages nomment déjà nœud et port.
pub fn build_definition(
    def: &GraphDefinition,
    nodes: &NodeRegistry,
    policy: &NodeTypePolicy,
) -> Result<DataflowGraph, GraphToolError> {
    validate_node_types(def, policy)?;

    for node in &def.nodes {
        if !nodes.has(&node.node_type) {
            return Err(GraphToolError::UnknownNodeType {
                node: node.name.clone(),
                node_type: node.node_type.clone(),
            });
        }
    }

    let cycle = cycle_nodes(def);
    if !cycle.is_empty() {
        return Err(GraphToolError::Cycle { nodes: cycle });
    }

    let graph = DataflowGraph::from_definition(def, nodes).map_err(GraphToolError::Build)?;
    graph.validate().map_err(GraphToolError::Build)?;
    Ok(graph)
}

/// Construit, exécute, et rend le port `result` sérialisé en JSON.
///
/// Le chemin complet **graphe → validation → exécution → résultat**, atteignable
/// sans passer par [`GraphToolRegistry`] : un graphe composé à la volée n'a pas
/// de nom à chercher.
pub fn execute_definition(
    def: &GraphDefinition,
    nodes: &NodeRegistry,
    services: Arc<ServiceRegistry>,
    policy: &NodeTypePolicy,
    result: (&str, &str),
) -> Result<String, GraphToolError> {
    execute_definition_as(def, nodes, services, policy, result, None)
}

/// La même, sous un identifiant de run choisi (`None` : généré) — l'adresse
/// du graphe sur le bus, pour un `EventSourceNode(topics='inbox')`.
pub fn execute_definition_as(
    def: &GraphDefinition,
    nodes: &NodeRegistry,
    services: Arc<ServiceRegistry>,
    policy: &NodeTypePolicy,
    result: (&str, &str),
    run_id: Option<&str>,
) -> Result<String, GraphToolError> {
    let (result_node, result_port) = result;
    let mut graph = build_definition(def, nodes, policy)?;
    let runtime = DataflowRuntime::with_services_arc(MAX_ITERATIONS, services);
    let output = match run_id {
        Some(id) => runtime.execute_as(&mut graph, id),
        None => runtime.execute(&mut graph),
    }
    .map_err(GraphToolError::Execution)?;
    let value = output
        .get(result_node, result_port)
        .ok_or_else(|| GraphToolError::NoResult {
            node: result_node.to_string(),
            port: result_port.to_string(),
        })?;
    render_port_value(value).map_err(|detail| GraphToolError::Unserializable {
        node: result_node.to_string(),
        port: result_port.to_string(),
        detail,
    })
}

/// La même, **qui ne peut pas échouer** : le contenu d'un résultat d'outil.
///
/// Structure invalide, type de nœud interdit, cycle, échec d'exécution,
/// panique d'un nœud : tout devient un JSON `{"error": …, "detail": …}` que
/// le modèle peut lire et corriger. Un `Err` remonté arrête une boucle
/// d'agent ; un résultat d'erreur la nourrit.
pub fn run_definition_as_tool_content(
    def: &GraphDefinition,
    nodes: &NodeRegistry,
    services: Arc<ServiceRegistry>,
    policy: &NodeTypePolicy,
    result: (&str, &str),
) -> String {
    // Un nœud qui panique ne doit pas emporter la boucle d'agent avec lui.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute_definition(def, nodes, services, policy, result)
    }));
    match outcome {
        Ok(Ok(content)) => content,
        Ok(Err(e)) => e.to_tool_json(),
        Err(payload) => {
            let msg = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panique sans message".to_string());
            GraphToolError::Panic(msg).to_tool_json()
        }
    }
}

// ─── Choices ────────────────────────────────────────────────────────────────

/// Refuse une valeur hors des valeurs admises — après [`resolve_params`],
/// donc sur des arguments déjà typés et complétés. Les listes du catalogue
/// ne sont vérifiées que s'il est fourni.
pub fn check_choices(
    params: &[ConfigParam],
    resolved: &Map<String, Value>,
    catalog: Option<&crate::catalog::Catalog>,
) -> Result<(), GraphToolError> {
    for p in params {
        let Some(choices) = &p.choices else { continue };
        let Some(Value::String(value)) = resolved.get(p.name) else { continue };
        let Some((values, _)) = choices.resolve(catalog) else { continue };
        if !values.contains(value) {
            return Err(GraphToolError::BadChoice {
                name: p.name.to_string(),
                value: value.clone(),
                choices: values,
            });
        }
    }
    Ok(())
}

/// Une liste de valeurs admises ne convient qu'à une chaîne, et son défaut
/// doit en faire partie — sinon la fiche promettrait une valeur qu'elle
/// refuserait.
fn check_choices_fit(p: &ConfigParam, choices: &Choices) -> Result<(), GraphToolError> {
    let spec = |d: String| GraphToolError::Spec(d);
    if p.param_type != ConfigParamType::String {
        return Err(spec(format!("choices '{}' : seul un paramètre string se borne", p.name)));
    }
    if let (Choices::Fixed(values), Some(Value::String(d))) = (choices, &p.default) {
        if !values.contains(d) {
            return Err(spec(format!(
                "choices '{}' : le défaut '{d}' n'est pas dans la liste ({})",
                p.name,
                values.join(", ")
            )));
        }
    }
    Ok(())
}

/// Les paramètres de nœuds qu'un `$name` du gabarit alimente :
/// `(nœud.paramètre, sa déclaration)`. Une valeur de configuration qui vaut
/// exactement `"$name"` est un câblage ; `"$name"` enfoui dans une chaîne
/// plus longue n'en est pas un.
fn wired_params(template: &GraphDefinition, registry: &NodeRegistry, name: &str) -> Vec<(String, ConfigParam)> {
    let needle = format!("${name}");
    let mut out = Vec::new();
    for node in &template.nodes {
        let Some(cfg) = node.config.as_object() else { continue };
        let Some(schema) = registry.schema(&node.node_type) else { continue };
        for (key, val) in cfg {
            if val.as_str() != Some(needle.as_str()) {
                continue;
            }
            if let Some(cp) = schema.config_params.iter().find(|c| c.name == key) {
                out.push((format!("{}.{}", node.name, key), cp.clone()));
            }
        }
    }
    out
}

/// `<param> = @targets` | `<param> = @relations` | `<param> = a | b | c`
fn parse_choices(line: &str) -> Result<(String, Choices), GraphToolError> {
    let spec = |d: String| GraphToolError::Spec(d);
    let (name, rest) = line
        .split_once('=')
        .ok_or_else(|| spec(format!("choices '{line}' : attendu '<param> = …'")))?;
    let name = name.trim();
    if name.is_empty() {
        return Err(spec(format!("choices '{line}' : nom de paramètre vide")));
    }
    let rest = rest.trim();
    let choices = match rest {
        "@targets" => Choices::Targets,
        "@relations" => Choices::Relations,
        other if other.starts_with('@') => {
            return Err(spec(format!(
                "choices '{name}' : source '{other}' inconnue (@targets, @relations)"
            )))
        }
        other => {
            let values: Vec<String> = other
                .split('|')
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect();
            if values.is_empty() {
                return Err(spec(format!("choices '{name}' : liste vide")));
            }
            Choices::Fixed(values)
        }
    };
    Ok((name.to_string(), choices))
}

// ─── GraphTool ──────────────────────────────────────────────────────────────

/// Le **spécificateur** : la face publique d'un graphe.
///
/// Ce qu'il porte, et rien de plus : un nom, une description, des paramètres
/// typés ([`ConfigParam`], le même vocabulaire que les nœuds), le graphe
/// **gabarit** (les `$param` y survivent tels quels) et le port qui porte le
/// résultat.
#[derive(Debug, Clone)]
pub struct GraphTool {
    name: String,
    description: String,
    params: Vec<ConfigParam>,
    /// Les paramètres déclarés **sans type** dans la fiche : ils prennent
    /// type, défaut et valeurs admises du nœud qu'ils alimentent, à
    /// [`Self::bind`]. Vide une fois lié.
    untyped: BTreeSet<String>,
    template: GraphDefinition,
    result_node: String,
    result_port: String,
}

impl GraphTool {
    /// Construction depuis Rust.
    ///
    /// `result` s'écrit `nœud.port` ; le port peut lui-même contenir un point
    /// (c'est le cas d'un [`super::GraphNode`], dont les ports libres
    /// s'appellent `nœud_interne.port`), la coupure se fait au **premier**.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        params: Vec<ConfigParam>,
        template: GraphDefinition,
        result: &str,
    ) -> Result<Self, GraphToolError> {
        Self::assemble(name.into(), description.into(), params, BTreeSet::new(), template, result)
    }

    fn assemble(
        name: String,
        description: String,
        params: Vec<ConfigParam>,
        untyped: BTreeSet<String>,
        template: GraphDefinition,
        result: &str,
    ) -> Result<Self, GraphToolError> {
        let (result_node, result_port) = result.split_once('.').ok_or_else(|| {
            GraphToolError::Spec(format!("port de résultat '{result}' : attendu 'nœud.port'"))
        })?;
        let tool = Self {
            name,
            description,
            params,
            untyped,
            template,
            result_node: result_node.to_string(),
            result_port: result_port.to_string(),
        };
        tool.check_spec()?;
        Ok(tool)
    }

    /// Borne un paramètre à une liste de valeurs admises.
    ///
    /// Le paramètre doit exister et être une chaîne ; un défaut déclaré doit
    /// figurer dans une liste close — sinon la fiche promettrait une valeur
    /// qu'elle refuserait.
    pub fn with_choices(mut self, param: &str, choices: Choices) -> Result<Self, GraphToolError> {
        let name = self.name.clone();
        let p = self
            .params
            .iter_mut()
            .find(|p| p.name == param)
            .ok_or_else(|| GraphToolError::Spec(format!("choices '{param}' : paramètre inconnu de l'outil '{name}'")))?;
        check_choices_fit(p, &choices)?;
        p.choices = Some(choices);
        Ok(self)
    }

    /// Lie la fiche au registre de nœuds : chaque `$param` **hérite** de ce
    /// qu'il alimente.
    ///
    /// Un `$param` du gabarit est une entrée de configuration du graphe,
    /// câblée sur chaque `(nœud, paramètre)` où il apparaît. Le nœud sait
    /// déjà le type, le défaut et les valeurs admises de son paramètre ; la
    /// fiche n'a pas à les redire. Ici :
    ///
    /// - un paramètre déclaré **sans type** (`%% param: direction -- …`)
    ///   prend le type, le défaut et le caractère requis du paramètre de nœud
    ///   qu'il alimente ;
    /// - un paramètre sans `choices` ni sous-schéma hérite de ceux du nœud ;
    /// - ce qui est déclaré explicitement dans la fiche **prime** ;
    /// - un `$param` câblé sur deux nœuds qui ne sont pas d'accord (types,
    ///   listes) est une erreur de fiche — la trancher en déclarant.
    ///
    /// Un nœud-outil ([`Self::as_node_factory`]) publie ses paramètres
    /// complets dans son schéma, donc l'héritage traverse les niveaux :
    /// `search_expand` hérite de `SearchTool`, qui a hérité de
    /// `SearchSourceNode`. Les types de nœuds inconnus du registre ne
    /// contribuent rien — ils seront refusés à la construction.
    pub fn bind(&self, registry: &NodeRegistry) -> Result<Self, GraphToolError> {
        let spec = |d: String| GraphToolError::Spec(d);
        let mut tool = self.clone();
        for p in &mut tool.params {
            let wired = wired_params(&self.template, registry, p.name);
            if self.untyped.contains(p.name) {
                let types: BTreeSet<&'static str> =
                    wired.iter().map(|(_, w)| param_type_name(&w.param_type)).collect();
                match types.len() {
                    0 => {
                        return Err(spec(format!(
                            "paramètre '{}' de l'outil '{}' : sans type, et câblé sur aucun paramètre de nœud connu",
                            p.name, self.name
                        )))
                    }
                    1 => {}
                    _ => {
                        return Err(spec(format!(
                            "paramètre '{}' de l'outil '{}' : sans type, et câblé sur des paramètres de types différents ({})",
                            p.name,
                            self.name,
                            wired.iter().map(|(at, w)| format!("{at} : {}", param_type_name(&w.param_type))).collect::<Vec<_>>().join(" ; ")
                        )))
                    }
                }
                p.param_type = wired[0].1.param_type.clone();
                if p.default.is_none() {
                    p.default = wired.iter().find_map(|(_, w)| w.default.clone());
                }
                if !p.required {
                    p.required = p.default.is_none() && wired.iter().any(|(_, w)| w.required);
                }
            } else {
                for (at, w) in &wired {
                    if w.param_type != p.param_type {
                        return Err(spec(format!(
                            "paramètre '{}' de l'outil '{}' : déclaré {}, mais {at} attend {}",
                            p.name, self.name, param_type_name(&p.param_type), param_type_name(&w.param_type)
                        )));
                    }
                }
            }
            if p.choices.is_none() {
                let mut found: Vec<(&String, &Choices)> =
                    wired.iter().filter_map(|(at, w)| w.choices.as_ref().map(|c| (at, c))).collect();
                found.dedup_by(|a, b| a.1 == b.1);
                match found.as_slice() {
                    [] => {}
                    [(_, c)] => {
                        check_choices_fit(p, c)?;
                        p.choices = Some((*c).clone());
                    }
                    many => {
                        return Err(spec(format!(
                            "paramètre '{}' de l'outil '{}' : câblé sur des nœuds dont les valeurs admises ne sont pas d'accord ({}) — déclarer `%% choices:` pour trancher",
                            p.name, self.name,
                            many.iter().map(|(at, c)| format!("{at} : {}", c.spec())).collect::<Vec<_>>().join(" ; ")
                        )))
                    }
                }
            }
            if p.json_schema.is_none() {
                let mut found: Vec<(&String, &Value)> =
                    wired.iter().filter_map(|(at, w)| w.json_schema.as_ref().map(|j| (at, j))).collect();
                found.dedup_by(|a, b| a.1 == b.1);
                match found.as_slice() {
                    [] => {}
                    [(_, j)] => p.json_schema = Some((*j).clone()),
                    many => {
                        return Err(spec(format!(
                            "paramètre '{}' de l'outil '{}' : câblé sur des nœuds dont les sous-schémas ne sont pas d'accord ({})",
                            p.name, self.name,
                            many.iter().map(|(at, _)| at.as_str()).collect::<Vec<_>>().join(" ; ")
                        )))
                    }
                }
            }
        }
        tool.untyped.clear();
        tool.check_spec()?;
        Ok(tool)
    }

    fn check_spec(&self) -> Result<(), GraphToolError> {
        let spec = GraphToolError::Spec;
        if self.name.trim().is_empty() {
            return Err(spec("nom d'outil vide".into()));
        }
        if self.description.trim().is_empty() {
            return Err(spec(format!("outil '{}' sans description", self.name)));
        }
        for (field, text) in [
            ("le nom", self.name.as_str()),
            ("la description", self.description.as_str()),
        ] {
            if text.contains('\n') {
                return Err(spec(format!(
                    "{field} de l'outil '{}' tient sur une ligne (la fiche est un en-tête Mermaid)",
                    self.name
                )));
            }
        }

        let mut seen = BTreeSet::new();
        for p in &self.params {
            if !seen.insert(p.name) {
                return Err(spec(format!("paramètre '{}' déclaré deux fois", p.name)));
            }
            if p.description.trim().is_empty() || p.description.contains('\n') {
                return Err(spec(format!(
                    "paramètre '{}' : description vide ou multiligne",
                    p.name
                )));
            }
            if p.required && p.default.is_some() {
                return Err(spec(format!(
                    "paramètre '{}' : requis *et* pourvu d'un défaut",
                    p.name
                )));
            }
            // Un facultatif sans défaut laisserait son `$var` non résolu dans
            // la configuration d'un nœud : refusé à la construction plutôt
            // que découvert à l'exécution.
            // Un paramètre sans type le prendra du nœud, avec son défaut :
            // la règle s'applique à la liaison.
            if !p.required && p.default.is_none() && !self.untyped.contains(p.name) {
                return Err(spec(format!(
                    "paramètre facultatif '{}' sans valeur par défaut",
                    p.name
                )));
            }
            if let Some(d) = &p.default {
                if !value_matches(&p.param_type, d) {
                    return Err(spec(format!(
                        "paramètre '{}' : défaut {} incompatible avec le type {}",
                        p.name,
                        d,
                        param_type_name(&p.param_type)
                    )));
                }
            }
        }

        // La liaison paramètres ↔ graphe est bijective, et vérifiée.
        let used = template_vars(&self.template);
        for v in &used {
            if !self.params.iter().any(|p| p.name == v) {
                return Err(spec(format!(
                    "le graphe utilise ${v}, qui n'est pas un paramètre déclaré"
                )));
            }
        }
        for p in &self.params {
            if !used.contains(p.name) {
                return Err(spec(format!(
                    "le paramètre '{}' n'apparaît nulle part dans le graphe (${})",
                    p.name, p.name
                )));
            }
        }

        if !self
            .template
            .nodes
            .iter()
            .any(|n| n.name == self.result_node)
        {
            return Err(spec(format!(
                "le port de résultat désigne le nœud '{}', absent du graphe",
                self.result_node
            )));
        }
        Ok(())
    }

    /// Construction depuis un fichier Mermaid dont l'en-tête `%%` porte la fiche.
    pub fn from_mermaid(source: &str) -> Result<Self, GraphToolError> {
        let header = parse_header(source)?;

        // Le gabarit est parsé avec des variables **identité** : chaque
        // `$param` déclaré se substitue à lui-même, et survit donc dans la
        // définition. Deux bénéfices d'un coup : la substitution devient
        // typée (elle a lieu plus tard, sur la valeur JSON), et un `$var` non
        // déclaré est refusé ici, par `UnknownVariable`.
        let identity: HashMap<String, String> = header
            .params
            .iter()
            .map(|p| (p.name.to_string(), format!("${}", p.name)))
            .collect();
        let template = parse_mermaid_template(source, &identity)
            .map_err(|e| GraphToolError::Spec(e.to_string()))?;

        let mut tool = Self::assemble(
            header.name,
            header.description,
            header.params,
            header.untyped.into_iter().collect(),
            template,
            &header.result,
        )?;
        for (param, choices) in header.choices {
            tool = tool.with_choices(&param, choices)?;
        }
        Ok(tool)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn params(&self) -> &[ConfigParam] {
        &self.params
    }
    /// La définition **gabarit** : les `$param` y sont encore des chaînes.
    pub fn template(&self) -> &GraphDefinition {
        &self.template
    }
    /// `(nœud, port)` qui porte le résultat.
    pub fn result(&self) -> (&str, &str) {
        (&self.result_node, &self.result_port)
    }

    /// La fiche d'identité, telle qu'un LLM la reçoit — sans catalogue : les
    /// listes closes deviennent des `enum`, les listes `@targets` /
    /// `@relations` restent des chaînes libres.
    pub fn tool_def(&self) -> ToolDef {
        self.tool_def_with(None)
    }

    /// La même, résolue contre le catalogue : les cibles et les relations
    /// **réelles**, au moment où la fiche part vers le modèle.
    pub fn tool_def_with(&self, catalog: Option<&crate::catalog::Catalog>) -> ToolDef {
        let mut parameters = params_object_schema(&self.params);
        for p in &self.params {
            let Some(choices) = &p.choices else { continue };
            let Some((values, hint)) = choices.resolve(catalog) else { continue };
            let Some(schema) = parameters["properties"].get_mut(p.name).and_then(Value::as_object_mut) else {
                continue;
            };
            schema.insert("enum".into(), Value::Array(values.into_iter().map(Value::String).collect()));
            if let Some(hint) = hint {
                let description = schema.get("description").and_then(Value::as_str).unwrap_or("").to_string();
                schema.insert("description".into(), Value::String(format!("{description} {hint}")));
            }
        }
        ToolDef {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters,
        }
    }

    /// Le graphe **et** sa fiche, réémis en Mermaid — l'aller-retour de
    /// [`Self::from_mermaid`].
    pub fn to_mermaid(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("%% tool: {}\n", self.name));
        out.push_str(&format!("%% description: {}\n", self.description));
        for p in &self.params {
            // Non lié : la fiche se réémet telle qu'écrite, sans type.
            if self.untyped.contains(p.name) {
                let bang = if p.required { "!" } else { "" };
                let default = p.default.as_ref().map(|d| format!(" = {d}")).unwrap_or_default();
                out.push_str(&format!("%% param: {}{bang}{default} -- {}\n", p.name, p.description));
                continue;
            }
            let ty = param_type_name(&p.param_type);
            let spec = if p.required {
                format!("{ty}!")
            } else {
                let d = p.default.clone().unwrap_or(Value::Null);
                format!("{ty} = {d}")
            };
            out.push_str(&format!("%% param: {} {} -- {}\n", p.name, spec, p.description));
        }
        for p in &self.params {
            if let Some(choices) = &p.choices {
                out.push_str(&format!("%% choices: {} = {}\n", p.name, choices.spec()));
            }
        }
        out.push_str(&format!(
            "%% result: {}.{}\n\n",
            self.result_node, self.result_port
        ));
        out.push_str(&to_mermaid(&self.template));
        out
    }

    /// Valide les arguments et rend l'objet complet (défauts remplis).
    /// Les listes closes sont vérifiées ; celles qui viennent du catalogue
    /// ne le sont pas sans lui — voir [`Self::validate_arguments_with`].
    pub fn validate_arguments(&self, args: &Value) -> Result<Map<String, Value>, GraphToolError> {
        self.validate_arguments_with(args, None)
    }

    /// La même, avec les valeurs admises résolues contre le catalogue : une
    /// relation absente du schéma est un [`GraphToolError::BadChoice`] qui
    /// nomme les relations existantes, pas un graphe qui rend zéro voisin.
    pub fn validate_arguments_with(
        &self,
        args: &Value,
        catalog: Option<&crate::catalog::Catalog>,
    ) -> Result<Map<String, Value>, GraphToolError> {
        let resolved = resolve_params(&self.params, args)?;
        check_choices(&self.params, &resolved, catalog)?;
        Ok(resolved)
    }

    /// Valide, substitue, et rend la définition **concrète** à exécuter.
    pub fn instantiate(&self, args: &Value) -> Result<GraphDefinition, GraphToolError> {
        self.instantiate_with(args, None)
    }

    /// La même, sous les valeurs admises du catalogue.
    pub fn instantiate_with(
        &self,
        args: &Value,
        catalog: Option<&crate::catalog::Catalog>,
    ) -> Result<GraphDefinition, GraphToolError> {
        let resolved = self.validate_arguments_with(args, catalog)?;
        Ok(substitute_definition(&self.template, &resolved))
    }

    /// Construit le graphe sans l'exécuter — de quoi vérifier une fiche
    /// (types de nœuds connus, ports du graphe cohérents) à froid.
    pub fn build(
        &self,
        nodes: &NodeRegistry,
        args: &Value,
    ) -> Result<DataflowGraph, GraphToolError> {
        build_definition(&self.instantiate(args)?, nodes, &NodeTypePolicy::All)
    }

    /// Exécute l'outil : valide, substitue, puis délègue à
    /// [`execute_definition`].
    ///
    /// Politique [`NodeTypePolicy::All`] : la fiche et le graphe d'un
    /// graphe-outil déclaré sont écrits à la main, pas produits par un
    /// modèle — il n'y a rien à borner. Un graphe composé à la volée passera
    /// par [`Self::execute_with_policy`] ou directement par
    /// [`execute_definition`].
    pub fn execute(
        &self,
        nodes: &NodeRegistry,
        services: Arc<ServiceRegistry>,
        args: &Value,
    ) -> Result<String, GraphToolError> {
        self.execute_with_policy(nodes, services, args, &NodeTypePolicy::All)
    }

    /// La même, sous une frontière de capacités explicite.
    pub fn execute_with_policy(
        &self,
        nodes: &NodeRegistry,
        services: Arc<ServiceRegistry>,
        args: &Value,
        policy: &NodeTypePolicy,
    ) -> Result<String, GraphToolError> {
        let def = self.instantiate(args)?;
        execute_definition(
            &def,
            nodes,
            services,
            policy,
            (&self.result_node, &self.result_port),
        )
    }

    /// Le graphe-outil vu comme un **type de nœud**, pour qu'un autre
    /// graphe-outil puisse le contenir.
    ///
    /// C'est là toute la composition : le nœud publie les paramètres de
    /// l'outil comme `config_params`, et sa fabrique substitue la
    /// configuration reçue dans le gabarit avant de matérialiser le
    /// sous-graphe. `registry` est le registre que verra le **sous**-graphe.
    pub fn as_node_factory(
        &self,
        node_type: &str,
        registry: Arc<NodeRegistry>,
    ) -> Result<GraphNodeFactory, GraphToolError> {
        GraphNodeFactory::templated(
            node_type,
            &self.description,
            self.template.clone(),
            self.params.clone(),
            registry,
        )
        .map_err(GraphToolError::Spec)
    }
}

/// Sérialise une valeur de port pour la rendre au modèle.
///
/// Réutilise [`port_value_to_checkpoint`] : la conversion « valeur de port →
/// JSON » existe déjà pour les checkpoints, et elle couvre exactement les
/// types que nos ports transportent (`Vec<UnifiedResult>`, `SearchMeta`,
/// `Vec<(String, String)>`, les lots d'ingestion…).
fn render_port_value(value: &PortValue) -> Result<String, String> {
    let cpv = port_value_to_checkpoint(value)?;
    match cpv.data_json {
        // Une chaîne JSON nue est du texte pour le modèle (markdown de
        // `read` / `grep`) : rendue sans guillemets ni échappements.
        Some(json) => Ok(match serde_json::from_str::<serde_json::Value>(&json) {
            Ok(serde_json::Value::String(text)) => text,
            _ => json,
        }),
        None if cpv.port_type == PortType::Empty => Ok(r#"{"ok":true}"#.to_string()),
        None => Err(format!("type de port {:?} non sérialisable", cpv.port_type)),
    }
}

// ─── En-tête Mermaid ────────────────────────────────────────────────────────

struct Header {
    name: String,
    description: String,
    params: Vec<ConfigParam>,
    /// Les paramètres déclarés sans type — à lier.
    untyped: Vec<String>,
    choices: Vec<(String, Choices)>,
    result: String,
}

/// Lit les directives `%%` qui précèdent le graphe.
///
/// S'arrête à la première ligne non vide qui n'est pas un commentaire : la
/// fiche est un **en-tête**, et les `%%` du corps restent des commentaires.
fn parse_header(source: &str) -> Result<Header, GraphToolError> {
    let spec = GraphToolError::Spec;
    let mut name = None;
    let mut description = String::new();
    let mut params: Vec<ConfigParam> = Vec::new();
    let mut untyped = Vec::new();
    let mut choices = Vec::new();
    let mut result = None;

    for raw in source.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let Some(body) = line.strip_prefix("%%") else {
            break;
        };
        let body = body.trim();
        if let Some(v) = body.strip_prefix("tool:") {
            name = Some(v.trim().to_string());
        } else if let Some(v) = body.strip_prefix("description:") {
            if !description.is_empty() {
                description.push(' ');
            }
            description.push_str(v.trim());
        } else if let Some(v) = body.strip_prefix("param:") {
            let (param, is_untyped) = parse_param(v.trim())?;
            if is_untyped {
                untyped.push(param.name.to_string());
            }
            params.push(param);
        } else if let Some(v) = body.strip_prefix("choices:") {
            choices.push(parse_choices(v.trim())?);
        } else if let Some(v) = body.strip_prefix("result:") {
            result = Some(v.trim().to_string());
        }
        // Toute autre ligne `%%` est un commentaire libre.
    }

    Ok(Header {
        name: name.ok_or_else(|| spec("en-tête sans directive '%% tool:'".into()))?,
        description,
        params,
        untyped,
        choices,
        result: result.ok_or_else(|| spec("en-tête sans directive '%% result:'".into()))?,
    })
}

/// `<nom> <type>[!] [= <défaut JSON>] -- <description>`
///
/// Le type peut manquer (`<nom>[!] [= <défaut>] -- <description>`) : le
/// paramètre est alors **à lier** ([`GraphTool::bind`]) et prend le type du
/// paramètre de nœud qu'il alimente. Rend `(param, sans_type)`.
fn parse_param(line: &str) -> Result<(ConfigParam, bool), GraphToolError> {
    let spec = |d: String| GraphToolError::Spec(d);
    let (head, description) = line
        .split_once("--")
        .ok_or_else(|| spec(format!("param '{line}' : séparateur ' -- ' manquant")))?;
    let description = description.trim();
    if description.is_empty() {
        return Err(spec(format!("param '{line}' : description vide")));
    }

    let head = head.trim();
    let (name_part, rest) = match head.split_once(|c: char| c.is_whitespace() || c == '=') {
        Some((n, _)) => (n, head[n.len()..].trim()),
        None => (head, ""),
    };
    let (pname, name_required) = match name_part.strip_suffix('!') {
        Some(n) => (n, true),
        None => (name_part, false),
    };
    if pname.is_empty() {
        return Err(spec(format!("param '{line}' : nom vide")));
    }

    let (type_part, default) = match rest.split_once('=') {
        Some((t, d)) => {
            let d = d.trim();
            let parsed: Value = serde_json::from_str(d).map_err(|e| {
                spec(format!("param '{pname}' : défaut '{d}' n'est pas du JSON ({e})"))
            })?;
            (t.trim(), Some(parsed))
        }
        None => (rest, None),
    };

    let (type_name, type_required) = match type_part.trim().strip_suffix('!') {
        Some(t) => (t.trim(), true),
        None => (type_part.trim(), false),
    };
    let untyped = type_name.is_empty();
    // Sans type : `String` en attendant la liaison, qui le remplacera.
    let param_type = if untyped {
        ConfigParamType::String
    } else {
        param_type_from_name(type_name).ok_or_else(|| {
            spec(format!(
                "param '{pname}' : type '{type_name}' inconnu (string, int, float, bool, json)"
            ))
        })?
    };

    // `ConfigParam` veut du `&'static str` — même fuite volontaire que
    // `GraphNodeFactory`, pour la même raison : une fiche lue d'un fichier
    // vit aussi longtemps que le registre qui la porte.
    Ok((
        ConfigParam {
            name: Box::leak(pname.trim().to_string().into_boxed_str()),
            param_type,
            required: name_required || type_required,
            default,
            description: Box::leak(description.to_string().into_boxed_str()),
            choices: None,
            json_schema: None,
        },
        untyped,
    ))
}

// ─── GraphToolRegistry ──────────────────────────────────────────────────────

/// Le catalogue des graphes-outils — **distinct** du [`NodeRegistry`].
///
/// Distinct, et pas greffé, pour une raison de fond : un `NodeRegistry`
/// répond à « quel nœud sais-je instancier », un `GraphToolRegistry` à « quoi
/// ai-je le droit de proposer à un modèle ». Les deux listes ne se
/// recouvrent pas — 28 nœuds bruts d'un côté, une poignée d'actions
/// complètes de l'autre — et un `NodeSchema` n'a nulle part où mettre le port
/// de résultat, qui est justement ce qui fait d'un graphe un outil. Les
/// greffer aurait obligé chaque nœud à porter un drapeau « exposable » : une
/// mauvaise boîte à outils avec une colonne de plus.
///
/// Les deux registres se rejoignent quand même, par
/// [`GraphTool::as_node_factory`] : un graphe-outil peut *aussi* devenir un
/// type de nœud, et c'est ce qui rend la composition possible.
///
/// Une `BTreeMap` : l'ordre est stable par construction, sans tri à chaque
/// appel — le préfixe du prompt ne bouge pas, la mise en cache tient.
#[derive(Debug, Clone, Default)]
pub struct GraphToolRegistry {
    tools: BTreeMap<String, Arc<GraphTool>>,
}

impl GraphToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enregistre un outil. Un nom déjà pris est **refusé** : deux outils
    /// homonymes rendraient un `ToolCall` ambigu.
    pub fn register(&mut self, tool: GraphTool) -> Result<(), GraphToolError> {
        if self.tools.contains_key(&tool.name) {
            return Err(GraphToolError::Spec(format!(
                "outil '{}' déjà enregistré",
                tool.name
            )));
        }
        self.tools.insert(tool.name.clone(), Arc::new(tool));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Arc<GraphTool>> {
        self.tools.get(name)
    }
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
    pub fn names(&self) -> Vec<&str> {
        self.tools.keys().map(String::as_str).collect()
    }
    pub fn tools(&self) -> impl Iterator<Item = &Arc<GraphTool>> {
        self.tools.values()
    }
    pub fn len(&self) -> usize {
        self.tools.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// **Exécute un appel d'outil et rend le tour de conversation.**
    ///
    /// Ne échoue jamais : outil inconnu, arguments illisibles, argument
    /// manquant, graphe qui explose — tout ressort en résultat d'outil JSON,
    /// avec l'`id` de l'appel préservé. C'est la condition pour qu'un agent
    /// puisse se rattraper : une erreur remontée arrête la boucle, un
    /// résultat d'erreur la nourrit.
    pub fn call(
        &self,
        call: &ToolCall,
        nodes: &NodeRegistry,
        services: Arc<ServiceRegistry>,
    ) -> Turn {
        self.call_with_policy(call, nodes, services, &NodeTypePolicy::All)
    }

    /// La même, sous une frontière de capacités explicite.
    pub fn call_with_policy(
        &self,
        call: &ToolCall,
        nodes: &NodeRegistry,
        services: Arc<ServiceRegistry>,
        policy: &NodeTypePolicy,
    ) -> Turn {
        let content = self.call_content(call, nodes, services, policy);
        Turn::tool_result(&call.id, &call.name, content)
    }

    /// Retrouver le graphe par son nom, puis le donner à exécuter — et rien
    /// de plus. Tout ce qui suit la substitution vit dans
    /// [`run_definition_as_tool_content`], qui n'a pas besoin de registre :
    /// c'est la même porte pour un outil déclaré et pour un graphe composé à
    /// la volée.
    fn call_content(
        &self,
        call: &ToolCall,
        nodes: &NodeRegistry,
        services: Arc<ServiceRegistry>,
        policy: &NodeTypePolicy,
    ) -> String {
        let Some(tool) = self.tools.get(&call.name) else {
            return GraphToolError::UnknownTool {
                name: call.name.clone(),
                known: self.tools.keys().cloned().collect(),
            }
            .to_tool_json();
        };

        let raw = call.arguments.trim();
        let args: Value = if raw.is_empty() {
            Value::Object(Map::new())
        } else {
            // Réparation d'abord : des retours à la ligne bruts dans une
            // chaîne (Vertex fragmenté, modèle local) ne doivent pas suffire
            // à refuser l'appel.
            match serde_json::from_str(&crate::llm::repair_arguments_json(raw)) {
                Ok(v) => v,
                Err(e) => return GraphToolError::BadArgumentsJson(e.to_string()).to_tool_json(),
            }
        };

        // Le catalogue n'est tenu que le temps de valider : les nœuds du
        // graphe le reprendront pendant l'exécution.
        let def = {
            let catalog = services
                .get::<Arc<std::sync::Mutex<crate::catalog::Catalog>>>("catalog")
                .cloned();
            let guard = catalog.as_ref().and_then(|c| c.lock().ok());
            match tool.instantiate_with(&args, guard.as_deref()) {
                Ok(d) => d,
                Err(e) => return e.to_tool_json(),
            }
        };
        run_definition_as_tool_content(&def, nodes, services, policy, tool.result())
    }
}

// ─── Graphes-outils fournis ─────────────────────────────────────────────────

/// `search` — requête + limite → résultats. Trois nœuds, une fiche.
pub const SEARCH_TOOL_MERMAID: &str = include_str!("../../templates/tools/search.mmd");

/// `search_expand` — `search` **contenu** dans un graphe qui étend chaque
/// résultat par une relation. La preuve que la composition tient.
/// `read` et `grep` (feature `code`) : sur la `file_source`, annotés par le graphe.
#[cfg(feature = "code")]
pub const READ_TOOL_MERMAID: &str = include_str!("../../templates/tools/read.mmd");
#[cfg(feature = "code")]
pub const GREP_TOOL_MERMAID: &str = include_str!("../../templates/tools/grep.mmd");
#[cfg(feature = "code")]
pub const LIST_TOOL_MERMAID: &str = include_str!("../../templates/tools/list.mmd");
#[cfg(feature = "code")]
pub const EDIT_TOOL_MERMAID: &str = include_str!("../../templates/tools/edit.mmd");

/// Les noms des graphes-outils fournis, dans l'ordre où le modèle les voit
/// (trié — le cache de préfixe en dépend).
#[cfg(feature = "code")]
pub const BUILTIN_TOOL_NAMES: [&str; 6] = ["edit", "grep", "list", "read", "search", "search_expand"];
#[cfg(not(feature = "code"))]
pub const BUILTIN_TOOL_NAMES: [&str; 2] = ["search", "search_expand"];

pub const SEARCH_EXPAND_TOOL_MERMAID: &str =
    include_str!("../../templates/tools/search_expand.mmd");

/// Le type de nœud sous lequel `search` est enregistré pour être contenu.
pub const SEARCH_TOOL_NODE_TYPE: &str = "SearchTool";

/// Les deux registres prêts à l'emploi : les nœuds (28 fournis + `SearchTool`)
/// et les graphes-outils (`search`, `search_expand`).
///
/// Le registre de nœuds rendu contient `SearchTool`, ce qui rend `search`
/// **contenable** : `search_expand` s'en sert comme d'un nœud ordinaire.
pub fn builtin_graph_tools() -> Result<(NodeRegistry, GraphToolRegistry), GraphToolError> {
    // Le registre que verra le sous-graphe de `search` : les nœuds de base.
    let inner = {
        let mut r = NodeRegistry::new();
        super::node_factories::register_builtins(&mut r);
        Arc::new(r)
    };

    // Chaque fiche est **liée** au registre qu'elle voit : `search` hérite
    // de `SearchSourceNode` (les cibles), puis devient le nœud `SearchTool`
    // avec ses paramètres complets, dont `search_expand` hérite à son tour.
    let search = GraphTool::from_mermaid(SEARCH_TOOL_MERMAID)?.bind(&inner)?;

    let mut nodes = NodeRegistry::new();
    super::node_factories::register_builtins(&mut nodes);
    nodes.register(Box::new(
        search.as_node_factory(SEARCH_TOOL_NODE_TYPE, inner)?,
    ));
    let expand = GraphTool::from_mermaid(SEARCH_EXPAND_TOOL_MERMAID)?.bind(&nodes)?;

    let mut tools = GraphToolRegistry::new();
    tools.register(search)?;
    #[cfg(feature = "code")]
    {
        tools.register(GraphTool::from_mermaid(READ_TOOL_MERMAID)?.bind(&nodes)?)?;
        tools.register(GraphTool::from_mermaid(GREP_TOOL_MERMAID)?.bind(&nodes)?)?;
        tools.register(GraphTool::from_mermaid(LIST_TOOL_MERMAID)?.bind(&nodes)?)?;
        tools.register(GraphTool::from_mermaid(EDIT_TOOL_MERMAID)?.bind(&nodes)?)?;
    }
    tools.register(expand)?;
    Ok((nodes, tools))
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::checkpoint::{EdgeDef, NodeDef};
    use serde_json::json;

    /// `Result::unwrap_err` demande `T: Debug`, que ni `DataflowGraph` ni
    /// `Box<dyn Node>` n'implémentent.
    fn only_err<T>(r: Result<T, GraphToolError>) -> GraphToolError {
        match r {
            Ok(_) => panic!("attendu une erreur"),
            Err(e) => e,
        }
    }

    fn p(
        name: &'static str,
        param_type: ConfigParamType,
        required: bool,
        default: Option<Value>,
        description: &'static str,
    ) -> ConfigParam {
        ConfigParam { name, param_type, required, default, description, choices: None, json_schema: None }
    }

    /// Le même outil que `templates/tools/search.mmd`, construit en Rust.
    fn search_in_rust() -> GraphTool {
        let template = GraphDefinition {
            nodes: vec![
                NodeDef {
                    name: "source".into(),
                    node_type: "SearchSourceNode".into(),
                    config: json!({"target_name": "$target", "query": "$query"}),
                },
                NodeDef {
                    name: "bm25".into(),
                    node_type: "BM25SearchNode".into(),
                    config: json!({"limit": "$limit"}),
                },
                NodeDef {
                    name: "resolve".into(),
                    node_type: "ResolveParentNode".into(),
                    config: json!({}),
                },
            ],
            edges: vec![
                EdgeDef { from_node: "source".into(), from_port: "query".into(), to_node: "bm25".into(), to_port: "query".into() },
                EdgeDef { from_node: "source".into(), from_port: "query".into(), to_node: "resolve".into(), to_port: "query".into() },
                EdgeDef { from_node: "bm25".into(), from_port: "results".into(), to_node: "resolve".into(), to_port: "results".into() },
            ],
        };
        GraphTool::new(
            "search",
            "Cherche dans une entité ou une base de connaissances.",
            vec![
                p("target", ConfigParamType::String, true, None, "Nom de l'entité ou de la KB."),
                p("query", ConfigParamType::String, true, None, "Texte de la requête."),
                p("limit", ConfigParamType::Int, false, Some(json!(10)), "Nombre maximum de résultats."),
            ],
            template,
            "resolve.results",
        )
        .unwrap()
    }

    // ── Construction ────────────────────────────────────────────────

    #[test]
    fn build_from_rust() {
        let t = search_in_rust();
        assert_eq!(t.name(), "search");
        assert_eq!(t.result(), ("resolve", "results"));
        assert_eq!(t.params().len(), 3);
    }

    #[test]
    fn build_from_mermaid() {
        let t = GraphTool::from_mermaid(SEARCH_TOOL_MERMAID).unwrap();
        assert_eq!(t.name(), "search");
        assert_eq!(t.result(), ("resolve", "results"));
        let names: Vec<&str> = t.params().iter().map(|p| p.name).collect();
        assert_eq!(names, vec!["target", "query", "limit"]);
        assert!(t.params()[1].required);
        assert_eq!(t.params()[2].default, Some(json!(10)));
        assert_eq!(t.params()[2].param_type, ConfigParamType::Int);
    }

    #[test]
    fn rust_and_mermaid_agree() {
        // Les deux voies de construction donnent le même outil — à la lettre
        // des descriptions près, que le fichier soigne davantage.
        let a = search_in_rust();
        let b = GraphTool::from_mermaid(SEARCH_TOOL_MERMAID).unwrap();
        assert_eq!(a.name(), b.name());
        assert_eq!(a.result(), b.result());
        for (x, y) in a.params().iter().zip(b.params()) {
            assert_eq!(x.name, y.name);
            assert_eq!(x.param_type, y.param_type);
            assert_eq!(x.required, y.required);
            assert_eq!(x.default, y.default);
        }
        assert_eq!(a.tool_def().parameters["required"], b.tool_def().parameters["required"]);
        assert_eq!(a.template().nodes.len(), b.template().nodes.len());
        assert_eq!(a.template().edges.len(), b.template().edges.len());
        for (x, y) in a.template().nodes.iter().zip(&b.template().nodes) {
            assert_eq!(x.name, y.name);
            assert_eq!(x.node_type, y.node_type);
            assert_eq!(x.config, y.config);
        }
    }

    #[test]
    fn a_graph_with_a_header_still_parses_by_the_old_path() {
        // `parse_mermaid` ignore les `%%` : la fiche ne gêne personne.
        // (Avec les `$var` substitués, comme n'importe quel gabarit.)
        let mut vars = HashMap::new();
        for (k, v) in [("target", "Product"), ("query", "rust"), ("limit", "10")] {
            vars.insert(k.to_string(), v.to_string());
        }
        let def = parse_mermaid_template(SEARCH_TOOL_MERMAID, &vars).unwrap();
        assert_eq!(def.nodes.len(), 3);
        assert_eq!(def.edges.len(), 3);
    }

    // ── Aller-retour Mermaid avec la fiche ──────────────────────────

    #[test]
    fn mermaid_roundtrip_keeps_the_spec() {
        for source in [SEARCH_TOOL_MERMAID, SEARCH_EXPAND_TOOL_MERMAID] {
            let t = GraphTool::from_mermaid(source).unwrap();
            let emitted = t.to_mermaid();
            let back = GraphTool::from_mermaid(&emitted).unwrap();

            assert_eq!(back.name(), t.name());
            assert_eq!(back.description(), t.description());
            assert_eq!(back.result(), t.result());
            assert_eq!(back.params().len(), t.params().len());
            for (a, b) in t.params().iter().zip(back.params()) {
                assert_eq!(a.name, b.name);
                assert_eq!(a.param_type, b.param_type);
                assert_eq!(a.required, b.required);
                assert_eq!(a.default, b.default);
                assert_eq!(a.description, b.description);
            }
            assert_eq!(back.template().nodes.len(), t.template().nodes.len());
            for (a, b) in t.template().nodes.iter().zip(&back.template().nodes) {
                assert_eq!(a.name, b.name);
                assert_eq!(a.node_type, b.node_type);
                assert_eq!(a.config, b.config);
            }
            assert_eq!(back.template().edges.len(), t.template().edges.len());
            // Stable au deuxième tour.
            assert_eq!(back.to_mermaid(), emitted);
        }
    }

    #[test]
    fn rust_built_tool_also_roundtrips() {
        let t = search_in_rust();
        let back = GraphTool::from_mermaid(&t.to_mermaid()).unwrap();
        assert_eq!(back.name(), "search");
        assert_eq!(back.params().len(), 3);
        assert_eq!(back.to_mermaid(), t.to_mermaid());
    }

    // ── Fiches invalides ────────────────────────────────────────────

    #[test]
    fn spec_errors_are_named() {
        let def = |cfg: Value| GraphDefinition {
            nodes: vec![NodeDef { name: "n".into(), node_type: "ComposeNode".into(), config: cfg }],
            edges: vec![],
        };

        // $var non déclaré
        let e = GraphTool::new("t", "d", vec![], def(json!({"x": "$oops"})), "n.results").unwrap_err();
        assert!(e.to_string().contains("$oops"), "{e}");

        // paramètre déclaré qui ne va nulle part
        let e = GraphTool::new(
            "t", "d",
            vec![p("orphan", ConfigParamType::String, true, None, "d")],
            def(json!({})), "n.results",
        ).unwrap_err();
        assert!(e.to_string().contains("n'apparaît nulle part"), "{e}");

        // facultatif sans défaut
        let e = GraphTool::new(
            "t", "d",
            vec![p("x", ConfigParamType::String, false, None, "d")],
            def(json!({"x": "$x"})), "n.results",
        ).unwrap_err();
        assert!(e.to_string().contains("sans valeur par défaut"), "{e}");

        // port de résultat sur un nœud absent
        let e = GraphTool::new("t", "d", vec![], def(json!({})), "absent.results").unwrap_err();
        assert!(e.to_string().contains("absent du graphe"), "{e}");

        // port de résultat sans point
        let e = GraphTool::new("t", "d", vec![], def(json!({})), "results").unwrap_err();
        assert!(e.to_string().contains("nœud.port"), "{e}");
    }

    #[test]
    fn header_param_grammar_errors() {
        assert!(parse_param("query string!").is_err(), "séparateur manquant");
        assert!(parse_param("query blob! -- d").is_err(), "type inconnu");
        assert!(parse_param("limit int = nope -- d").is_err(), "défaut non JSON");
        assert!(parse_param("! -- d").is_err(), "nom vide");
        // Un défaut JSON peut contenir `:` et `=` : le séparateur est ` -- `.
        let (ok, untyped) = parse_param(r#"opts json = {"a":1} -- Options : brutes"#).unwrap();
        assert_eq!(ok.default, Some(json!({"a": 1})));
        assert_eq!(ok.description, "Options : brutes");
        assert!(!untyped);
        // Sans type : à lier. `!` sur le nom, défaut possible.
        let (p, untyped) = parse_param("direction -- d").unwrap();
        assert!(untyped && !p.required && p.default.is_none());
        let (p, untyped) = parse_param("direction! -- d").unwrap();
        assert!(untyped && p.required);
        let (p, untyped) = parse_param(r#"direction = "Incoming" -- d"#).unwrap();
        assert!(untyped && p.default == Some(json!("Incoming")));
    }

    // ── ToolDef ─────────────────────────────────────────────────────

    #[test]
    fn tool_def_exposes_declared_params() {
        let d = GraphTool::from_mermaid(SEARCH_TOOL_MERMAID).unwrap().tool_def();
        assert_eq!(d.name, "search");
        assert!(!d.description.is_empty());
        assert_eq!(d.parameters["type"], "object");
        assert_eq!(d.parameters["additionalProperties"], false);
        assert_eq!(d.parameters["properties"]["query"]["type"], "string");
        assert_eq!(d.parameters["properties"]["limit"]["type"], "integer");
        assert_eq!(d.parameters["properties"]["limit"]["default"], 10);
        let required: Vec<&str> = d.parameters["required"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(required, vec!["target", "query"]);
        // Aucun paramètre de plomberie n'a fuité.
        let props = d.parameters["properties"].as_object().unwrap();
        assert_eq!(props.len(), 3);
        assert!(!props.contains_key("fuzzy_distance"));
        assert!(!props.contains_key("result_mode"));
    }

    // ── Valeurs admises ─────────────────────────────────────────────

    #[test]
    fn choices_are_parsed_from_the_header_and_written_back() {
        let src = "%% tool: t\n%% description: d\n%% param: relation string! -- r\n\
                   %% param: direction string = \"Outgoing\" -- d\n\
                   %% choices: relation = @relations\n%% choices: direction = Outgoing | Incoming\n\
                   %% result: f.children\n\ngraph LR\n    f[\"FetchRelatedNode(relation=$relation, direction=$direction)\"]\n";
        let t = GraphTool::from_mermaid(src).unwrap();
        let choices = |t: &GraphTool, n: &str| t.params().iter().find(|p| p.name == n).unwrap().choices.clone();
        assert_eq!(choices(&t, "relation"), Some(Choices::Relations));
        assert_eq!(choices(&t, "direction"), Some(Choices::fixed(["Outgoing", "Incoming"])));
        let emitted = t.to_mermaid();
        assert!(emitted.contains("%% choices: direction = Outgoing | Incoming\n"), "{emitted}");
        assert!(emitted.contains("%% choices: relation = @relations\n"), "{emitted}");
        let back = GraphTool::from_mermaid(&emitted).unwrap();
        assert_eq!(choices(&back, "relation"), choices(&t, "relation"));
        assert_eq!(choices(&back, "direction"), choices(&t, "direction"));
    }

    #[test]
    fn a_fixed_list_becomes_an_enum_and_bounds_the_call() {
        let (_, tools) = builtin_graph_tools().unwrap();
        let t = tools.get("search_expand").unwrap();
        let d = t.tool_def();
        assert_eq!(d.parameters["properties"]["direction"]["enum"], json!(["Outgoing", "Incoming"]));
        // Sans catalogue, les listes `@…` restent des chaînes libres.
        assert!(d.parameters["properties"]["relation"].get("enum").is_none());
        assert!(d.parameters["properties"]["target"].get("enum").is_none());

        let ok = json!({"target": "Scope", "query": "x", "relation": "CONSUMES", "direction": "Incoming"});
        assert!(t.validate_arguments(&ok).is_ok());
        let bad = json!({"target": "Scope", "query": "x", "relation": "CONSUMES", "direction": "Sideways"});
        let err = t.validate_arguments(&bad).unwrap_err();
        assert_eq!(err.kind(), "bad_choice");
        let text = err.to_string();
        assert!(text.contains("'Sideways'") && text.contains("Outgoing, Incoming"), "{text}");
    }

    // ── Héritage par câblage ────────────────────────────────────────

    fn find<'a>(t: &'a GraphTool, name: &str) -> &'a ConfigParam {
        t.params().iter().find(|p| p.name == name).unwrap_or_else(|| panic!("param {name}"))
    }

    #[test]
    fn bound_tools_inherit_choices_through_two_levels() {
        let (_, tools) = builtin_graph_tools().unwrap();
        // `search` : `target` alimente `SearchSourceNode.target_name`, qui
        // déclare les cibles du catalogue — sans `%% choices:` dans la fiche.
        let search = tools.get("search").unwrap();
        assert_eq!(find(search, "target").choices, Some(Choices::Targets));
        assert_eq!(find(search, "query").choices, None);

        // `search_expand` : `target` traverse `SearchTool` ; `relation` et
        // `direction` viennent de `FetchRelatedNode` — et `direction` n'a
        // pas de type dans la fiche : il prend celui du nœud, son défaut,
        // sa liste.
        let expand = tools.get("search_expand").unwrap();
        assert_eq!(find(expand, "target").choices, Some(Choices::Targets));
        assert_eq!(find(expand, "relation").choices, Some(Choices::Relations));
        let direction = find(expand, "direction");
        assert_eq!(direction.param_type, ConfigParamType::String);
        assert_eq!(direction.default, Some(json!("Outgoing")));
        assert!(!direction.required);
        assert_eq!(direction.choices, Some(Choices::fixed(["Outgoing", "Incoming"])));
        assert_eq!(direction.description, "Sens de parcours depuis chaque résultat.");

        let d = expand.tool_def();
        assert_eq!(d.parameters["properties"]["direction"]["enum"], json!(["Outgoing", "Incoming"]));
        assert_eq!(d.parameters["properties"]["direction"]["default"], json!("Outgoing"));
        let err = expand
            .validate_arguments(&json!({"target": "X", "query": "q", "relation": "R", "direction": "Sideways"}))
            .unwrap_err();
        assert_eq!(err.kind(), "bad_choice");
    }

    #[test]
    fn a_nested_tool_node_checks_fixed_choices_at_creation() {
        let (nodes, _) = builtin_graph_tools().unwrap();
        // `FetchRelatedNode` sait que `direction` est `Outgoing | Incoming` :
        // une valeur hors liste est refusée à la création, pas à l'exécution.
        let err = nodes
            .create("FetchRelatedNode", "f", &json!({"relation": "R", "direction": "Sideways"}))
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(err.contains("Sideways") && err.contains("Outgoing, Incoming"), "{err}");
    }

    fn registry_with(params: Vec<ConfigParam>, node_type: &'static str) -> NodeRegistry {
        struct F(&'static str, Vec<ConfigParam>);
        impl super::super::node_registry::NodeFactory for F {
            fn create(&self, _: &str, _: &Value) -> Result<Box<dyn super::super::node::Node>, String> {
                Err("fabrique de test".into())
            }
            fn node_type(&self) -> &'static str {
                self.0
            }
            fn schema(&self) -> super::super::node_registry::NodeSchema {
                super::super::node_registry::NodeSchema {
                    node_type: self.0,
                    description: "test",
                    inputs: vec![],
                    outputs: vec![],
                    config_params: self.1.clone(),
                }
            }
        }
        let mut r = NodeRegistry::new();
        r.register(Box::new(F(node_type, params)));
        r
    }

    fn cp(name: &'static str, choices: Option<Choices>) -> ConfigParam {
        ConfigParam {
            name,
            param_type: ConfigParamType::String,
            required: false,
            default: None,
            description: "d",
            choices,
            json_schema: None,
        }
    }

    #[test]
    fn a_param_wired_to_disagreeing_nodes_is_a_spec_error_unless_declared() {
        let src = |extra: &str| {
            format!(
                "%% tool: t\n%% description: d\n%% param: m string = \"a\" -- m\n{extra}%% result: x.out\n\n\
                 graph LR\n    x[\"N(p=$m)\"]\n    y[\"N(q=$m)\"]\n"
            )
        };
        let registry = registry_with(
            vec![cp("p", Some(Choices::fixed(["a", "b"]))), cp("q", Some(Choices::fixed(["a", "c"])))],
            "N",
        );
        let err = GraphTool::from_mermaid(&src("")).unwrap().bind(&registry).unwrap_err().to_string();
        assert!(err.contains("ne sont pas d'accord") && err.contains("x.p") && err.contains("y.q"), "{err}");
        // Déclaré dans la fiche : l'explicite prime, plus de conflit.
        let bound = GraphTool::from_mermaid(&src("%% choices: m = a\n")).unwrap().bind(&registry).unwrap();
        assert_eq!(find(&bound, "m").choices, Some(Choices::fixed(["a"])));
    }

    #[test]
    fn an_untyped_param_must_be_wired_and_a_typed_one_must_agree() {
        let registry = registry_with(vec![cp("p", None)], "N");
        let unwired = "%% tool: t\n%% description: d\n%% param: m -- m\n%% result: x.out\n\n\
                       graph LR\n    x[\"Unknown(p=$m)\"]\n";
        let err = GraphTool::from_mermaid(unwired).unwrap().bind(&registry).unwrap_err().to_string();
        assert!(err.contains("sans type") && err.contains("aucun paramètre de nœud connu"), "{err}");

        let mistyped = "%% tool: t\n%% description: d\n%% param: m int = 3 -- m\n%% result: x.out\n\n\
                        graph LR\n    x[\"N(p=$m)\"]\n";
        let err = GraphTool::from_mermaid(mistyped).unwrap().bind(&registry).unwrap_err().to_string();
        assert!(err.contains("déclaré int") && err.contains("x.p attend string"), "{err}");

        // Sans type, avec `!` : requis, type hérité.
        let untyped = "%% tool: t\n%% description: d\n%% param: m! -- m\n%% result: x.out\n\n\
                       graph LR\n    x[\"N(p=$m)\"]\n";
        let bound = GraphTool::from_mermaid(untyped).unwrap().bind(&registry).unwrap();
        let m = find(&bound, "m");
        assert!(m.required && m.param_type == ConfigParamType::String && m.choices.is_none());
        assert!(bound.to_mermaid().contains("%% param: m string! -- m\n"));
    }

    #[test]
    fn choices_are_checked_against_the_spec() {
        let base = |extra: &str| {
            format!(
                "%% tool: t\n%% description: d\n%% param: mode string = \"fast\" -- m\n\
                 %% param: n int = 1 -- n\n{extra}\n%% result: a.out\n\ngraph LR\n    a[\"FetchRelatedNode(relation=$mode, limit=$n)\"]\n"
            )
        };
        let spec = |extra: &str| GraphTool::from_mermaid(&base(extra)).unwrap_err().to_string();
        let e = spec("%% choices: nope = a | b"); assert!(e.contains("paramètre inconnu"), "{e}");
        assert!(spec("%% choices: n = a | b").contains("seul un paramètre string"));
        assert!(spec("%% choices: mode = slow | slower").contains("le défaut 'fast'"));
        assert!(spec("%% choices: mode = @planets").contains("source '@planets' inconnue"));
        assert!(spec("%% choices: mode = |").contains("liste vide"));
        assert!(GraphTool::from_mermaid(&base("%% choices: mode = fast | slow")).is_ok());
    }

    #[test]
    fn the_model_sees_tools_not_the_29_raw_nodes() {
        let (nodes, tools) = builtin_graph_tools().unwrap();
        let exposed = crate::tools::graph_tool_defs(&tools);
        let names: Vec<&str> = exposed.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, BUILTIN_TOOL_NAMES, "ordre stable");
        // Le registre de nœuds reste introspectable, lui.
        assert_eq!(crate::tools::tool_defs(&nodes).len(), crate::dataflow::node_factories::BUILTIN_NODE_COUNT + 1, "nœuds + SearchTool");
        let openai = crate::tools::graph_tool_defs_openai(&tools);
        assert_eq!(openai.len(), BUILTIN_TOOL_NAMES.len());
        assert_eq!(openai[0]["function"]["name"], BUILTIN_TOOL_NAMES[0]);
    }

    #[test]
    fn tool_order_is_stable_across_runs() {
        let mut a = GraphToolRegistry::new();
        a.register(GraphTool::from_mermaid(SEARCH_EXPAND_TOOL_MERMAID).unwrap()).unwrap();
        a.register(GraphTool::from_mermaid(SEARCH_TOOL_MERMAID).unwrap()).unwrap();
        assert_eq!(a.names(), vec!["search", "search_expand"]);
    }

    #[test]
    fn duplicate_tool_name_is_refused() {
        let mut r = GraphToolRegistry::new();
        r.register(GraphTool::from_mermaid(SEARCH_TOOL_MERMAID).unwrap()).unwrap();
        assert!(r.register(GraphTool::from_mermaid(SEARCH_TOOL_MERMAID).unwrap()).is_err());
    }

    // ── Validation des arguments : les trois cas ────────────────────

    #[test]
    fn missing_argument_is_an_error() {
        let t = search_in_rust();
        let e = t.validate_arguments(&json!({"target": "Product"})).unwrap_err();
        assert_eq!(e.kind(), "missing_argument");
        assert!(e.to_string().contains("'query'"), "{e}");
    }

    #[test]
    fn unknown_argument_is_an_error() {
        let t = search_in_rust();
        let e = t
            .validate_arguments(&json!({"target": "P", "query": "q", "fuzzy_distance": 2}))
            .unwrap_err();
        assert_eq!(e.kind(), "unknown_argument");
        assert!(e.to_string().contains("fuzzy_distance"), "{e}");
    }

    #[test]
    fn wrong_type_is_an_error() {
        let t = search_in_rust();
        let e = t
            .validate_arguments(&json!({"target": "P", "query": "q", "limit": "dix"}))
            .unwrap_err();
        assert_eq!(e.kind(), "type_mismatch");
        assert!(e.to_string().contains("attendu int, reçu string"), "{e}");

        // Un flottant n'est pas un entier — pas de troncature silencieuse.
        let e = t
            .validate_arguments(&json!({"target": "P", "query": "q", "limit": 10.5}))
            .unwrap_err();
        assert_eq!(e.kind(), "type_mismatch");

        // Les arguments ne sont même pas un objet.
        let e = t.validate_arguments(&json!([1, 2])).unwrap_err();
        assert_eq!(e.kind(), "bad_arguments_json");
    }

    #[test]
    fn defaults_are_filled_in() {
        let t = search_in_rust();
        let args = t.validate_arguments(&json!({"target": "P", "query": "q"})).unwrap();
        assert_eq!(args["limit"], 10);
        assert_eq!(args.len(), 3);
    }

    // ── Substitution ────────────────────────────────────────────────

    #[test]
    fn substitution_preserves_types() {
        let t = search_in_rust();
        let def = t
            .instantiate(&json!({"target": "Product", "query": "rust", "limit": 3}))
            .unwrap();
        let bm25 = def.nodes.iter().find(|n| n.name == "bm25").unwrap();
        assert_eq!(bm25.config["limit"], json!(3), "un entier reste un entier");
        assert!(bm25.config["limit"].is_i64());
        let source = def.nodes.iter().find(|n| n.name == "source").unwrap();
        assert_eq!(source.config["target_name"], "Product");
        assert_eq!(source.config["query"], "rust");
        // Le gabarit n'a pas bougé.
        assert_eq!(t.template().nodes[1].config["limit"], "$limit");
    }

    #[test]
    fn substitution_interpolates_inside_a_string() {
        let mut args = Map::new();
        args.insert("q".into(), json!("rust"));
        args.insert("n".into(), json!(3));
        let def = GraphDefinition {
            nodes: vec![NodeDef {
                name: "n".into(),
                node_type: "ComposeNode".into(),
                config: json!({"a": "titre: $q ($n)", "b": "$q", "c": "$", "d": 7}),
            }],
            edges: vec![],
        };
        let out = substitute_definition(&def, &args);
        assert_eq!(out.nodes[0].config["a"], "titre: rust (3)");
        assert_eq!(out.nodes[0].config["b"], json!("rust"));
        assert_eq!(out.nodes[0].config["c"], "$");
        assert_eq!(out.nodes[0].config["d"], 7);
    }

    // ── Construction du graphe (sans base) ──────────────────────────

    #[test]
    fn search_builds_a_valid_graph() {
        let (nodes, tools) = builtin_graph_tools().unwrap();
        let g = tools
            .get("search")
            .unwrap()
            .build(&nodes, &json!({"target": "Product", "query": "rust"}))
            .unwrap();
        let mut names = g.node_names();
        names.sort_unstable();
        assert_eq!(names, vec!["bm25", "resolve", "source"]);
    }

    #[test]
    fn search_expand_contains_search() {
        let (nodes, tools) = builtin_graph_tools().unwrap();
        let expand = tools.get("search_expand").unwrap();
        // Le graphe extérieur contient bien un nœud du type de l'autre outil.
        assert!(expand
            .template()
            .nodes
            .iter()
            .any(|n| n.node_type == SEARCH_TOOL_NODE_TYPE));

        let g = expand
            .build(
                &nodes,
                &json!({"target": "Product", "query": "rust", "relation": "HAS_VARIANT"}),
            )
            .unwrap();
        let mut names = g.node_names();
        names.sort_unstable();
        assert_eq!(names, vec!["compose", "fetch", "inner"]);
    }

    #[test]
    fn the_contained_tool_receives_its_arguments() {
        // `search_expand` passe ses propres paramètres au `SearchTool` qu'il
        // contient : la substitution traverse les deux niveaux.
        let (nodes, tools) = builtin_graph_tools().unwrap();
        let def = tools
            .get("search_expand")
            .unwrap()
            .instantiate(&json!({"target": "Product", "query": "rust", "limit": 4, "relation": "HAS_VARIANT"}))
            .unwrap();
        let inner = def.nodes.iter().find(|n| n.name == "inner").unwrap();
        assert_eq!(inner.config["target"], "Product");
        assert_eq!(inner.config["limit"], json!(4));

        // Et la fabrique de nœud materialise le sous-graphe avec ces valeurs.
        let node = nodes
            .create(SEARCH_TOOL_NODE_TYPE, "inner", &inner.config)
            .unwrap();
        let outs: Vec<&str> = node.outputs().iter().map(|p| p.name).collect();
        assert!(outs.contains(&"resolve.results"), "ports libres : {outs:?}");
    }

    #[test]
    fn the_contained_tool_validates_its_own_arguments() {
        let (nodes, _) = builtin_graph_tools().unwrap();
        let e = nodes
            .create(SEARCH_TOOL_NODE_TYPE, "inner", &json!({"target": "P"}))
            .err()
            .expect("un argument manquant doit être refusé");
        assert!(e.contains("query"), "{e}");
    }

    #[test]
    fn contained_tool_publishes_its_params_as_config() {
        // Le trou de `GraphNodeFactory` (config_params: vec![]) est bouché.
        let (nodes, _) = builtin_graph_tools().unwrap();
        let schema = nodes.schema(SEARCH_TOOL_NODE_TYPE).unwrap();
        let names: Vec<&str> = schema.config_params.iter().map(|p| p.name).collect();
        assert_eq!(names, vec!["target", "query", "limit"]);
    }

    // ── Appel d'outil → Turn ────────────────────────────────────────

    fn empty_services() -> Arc<ServiceRegistry> {
        Arc::new(ServiceRegistry::new())
    }

    #[test]
    fn unknown_tool_becomes_a_tool_result() {
        let (nodes, tools) = builtin_graph_tools().unwrap();
        let call = ToolCall::new("call_1", "delete_everything", "{}");
        let turn = tools.call(&call, &nodes, empty_services());
        assert_eq!(turn.role, "tool");
        assert_eq!(turn.tool_call_id.as_deref(), Some("call_1"));
        assert_eq!(turn.tool_name.as_deref(), Some("delete_everything"));
        let v: Value = serde_json::from_str(&turn.content).unwrap();
        assert_eq!(v["error"], "unknown_tool");
        assert!(v["detail"].as_str().unwrap().contains("search"));
    }

    #[test]
    fn malformed_arguments_become_a_tool_result() {
        let (nodes, tools) = builtin_graph_tools().unwrap();
        // Un appel tronqué par max_tokens : du JSON invalide.
        let call = ToolCall::new("call_2", "search", r#"{"query": "ru"#);
        let turn = tools.call(&call, &nodes, empty_services());
        let v: Value = serde_json::from_str(&turn.content).unwrap();
        assert_eq!(v["error"], "bad_arguments_json");
    }

    #[test]
    fn a_missing_argument_becomes_a_tool_result() {
        let (nodes, tools) = builtin_graph_tools().unwrap();
        let call = ToolCall::new("call_3", "search", r#"{"target": "Product"}"#);
        let turn = tools.call(&call, &nodes, empty_services());
        let v: Value = serde_json::from_str(&turn.content).unwrap();
        assert_eq!(v["error"], "missing_argument");
        assert_eq!(turn.tool_call_id.as_deref(), Some("call_3"));
    }

    #[test]
    fn an_execution_failure_becomes_a_tool_result_not_an_error() {
        // Aucun service enregistré : `SearchSourceNode` ne trouvera pas le
        // catalogue et échouera. L'appel doit quand même rendre un `Turn`.
        let (nodes, tools) = builtin_graph_tools().unwrap();
        let call = ToolCall::new(
            "call_4",
            "search",
            r#"{"target": "Product", "query": "rust"}"#,
        );
        let turn = tools.call(&call, &nodes, empty_services());
        assert_eq!(turn.role, "tool");
        assert_eq!(turn.tool_call_id.as_deref(), Some("call_4"));
        let v: Value = serde_json::from_str(&turn.content).unwrap();
        assert!(
            matches!(v["error"].as_str(), Some("execution") | Some("no_result") | Some("panic")),
            "attendu une erreur d'exécution lisible, reçu : {}",
            turn.content
        );
        assert!(v["detail"].as_str().is_some_and(|d| !d.is_empty()));
    }

    #[test]
    fn every_error_renders_as_readable_json() {
        let errors = [
            GraphToolError::Spec("x".into()),
            GraphToolError::UnknownTool { name: "x".into(), known: vec!["search".into()] },
            GraphToolError::BadArgumentsJson("x".into()),
            GraphToolError::MissingArgument("q".into()),
            GraphToolError::UnknownArgument { name: "z".into(), known: vec![] },
            GraphToolError::TypeMismatch { name: "l".into(), expected: "int", got: "string" },
            GraphToolError::Build("x".into()),
            GraphToolError::UnknownNodeType { node: "n".into(), node_type: "T".into() },
            GraphToolError::ForbiddenNodeType { node: "n".into(), node_type: "T".into() },
            GraphToolError::Cycle { nodes: vec!["a".into(), "b".into()] },
            GraphToolError::Execution("x".into()),
            GraphToolError::NoResult { node: "n".into(), port: "p".into() },
            GraphToolError::Unserializable { node: "n".into(), port: "p".into(), detail: "x".into() },
            GraphToolError::Panic("x".into()),
        ];
        let mut kinds = BTreeSet::new();
        for e in &errors {
            let v: Value = serde_json::from_str(&e.to_tool_json()).unwrap();
            assert_eq!(v["error"], e.kind());
            assert!(v["detail"].as_str().is_some_and(|d| !d.is_empty()));
            kinds.insert(e.kind());
        }
        assert_eq!(kinds.len(), errors.len(), "codes d'erreur dupliqués");
    }

    // ── Un graphe sans nom : la porte du méta-outil de demain ───────

    /// Un graphe quelconque, ici bien formé, là fautif.
    fn bare_def(node_type: &str) -> GraphDefinition {
        GraphDefinition {
            nodes: vec![
                NodeDef { name: "source".into(), node_type: "SearchSourceNode".into(),
                          config: json!({"target_name": "Product", "query": "rust"}) },
                NodeDef { name: "bm25".into(), node_type: node_type.into(),
                          config: json!({"limit": 3}) },
            ],
            edges: vec![EdgeDef {
                from_node: "source".into(), from_port: "query".into(),
                to_node: "bm25".into(), to_port: "query".into(),
            }],
        }
    }

    #[test]
    fn a_definition_is_executable_without_any_tool_registry() {
        let (nodes, _) = builtin_graph_tools().unwrap();
        // Construction seule : pas de nom, pas de fiche, pas de registre d'outils.
        let graph =
            build_definition(&bare_def("BM25SearchNode"), &nodes, &NodeTypePolicy::All).unwrap();
        let mut names = graph.node_names();
        names.sort_unstable();
        assert_eq!(names, vec!["bm25", "source"]);

        // Et le chemin complet rend un contenu de résultat d'outil, sans `Err`.
        let content = run_definition_as_tool_content(
            &bare_def("BM25SearchNode"),
            &nodes,
            Arc::new(ServiceRegistry::new()),
            &NodeTypePolicy::All,
            ("bm25", "results"),
        );
        let v: Value = serde_json::from_str(&content).unwrap();
        // Sans catalogue enregistré ça échoue — mais lisiblement.
        assert!(v["error"].is_string(), "contenu : {content}");
    }

    #[test]
    fn the_capability_boundary_is_consulted_before_anything_is_built() {
        let def = GraphDefinition {
            nodes: vec![NodeDef {
                name: "wipe".into(),
                node_type: "DeleteRecordNode".into(),
                config: json!({}),
            }],
            edges: vec![],
        };
        // Ouverte par défaut : c'est le bon défaut pour un graphe écrit à la main.
        assert!(validate_node_types(&def, &NodeTypePolicy::All).is_ok());

        // Fermée : le nœud *et* son type sont nommés.
        let policy = NodeTypePolicy::only(["BM25SearchNode", "SearchSourceNode"]);
        let e = validate_node_types(&def, &policy).unwrap_err();
        assert_eq!(e.kind(), "forbidden_node_type");
        assert!(e.to_string().contains("wipe"), "{e}");
        assert!(e.to_string().contains("DeleteRecordNode"), "{e}");

        // Et la frontière est bien franchie *avant* la construction.
        let (nodes, _) = builtin_graph_tools().unwrap();
        let e = only_err(build_definition(&def, &nodes, &policy));
        assert_eq!(e.kind(), "forbidden_node_type");
    }

    #[test]
    fn structural_errors_name_the_culprit() {
        let (nodes, _) = builtin_graph_tools().unwrap();

        // Type de nœud inconnu : l'instance est nommée, pas seulement le type.
        let e = only_err(build_definition(&bare_def("BogusNode"), &nodes, &NodeTypePolicy::All));
        assert_eq!(e.kind(), "unknown_node_type");
        assert!(e.to_string().contains("'bm25'"), "{e}");
        assert!(e.to_string().contains("BogusNode"), "{e}");

        // Cycle : les nœuds impliqués sont nommés.
        let cyclic = GraphDefinition {
            nodes: vec![
                NodeDef { name: "a".into(), node_type: "ResolveParentNode".into(), config: json!({}) },
                NodeDef { name: "b".into(), node_type: "ResolveParentNode".into(), config: json!({}) },
            ],
            edges: vec![
                EdgeDef { from_node: "a".into(), from_port: "results".into(), to_node: "b".into(), to_port: "results".into() },
                EdgeDef { from_node: "b".into(), from_port: "results".into(), to_node: "a".into(), to_port: "results".into() },
            ],
        };
        let e = only_err(build_definition(&cyclic, &nodes, &NodeTypePolicy::All));
        assert_eq!(e.kind(), "cycle");
        assert!(e.to_string().contains('a') && e.to_string().contains('b'), "{e}");

        // Port inexistant : le message de `connect` nomme déjà nœud et port.
        let bad_port = GraphDefinition {
            nodes: vec![
                NodeDef { name: "source".into(), node_type: "SearchSourceNode".into(),
                          config: json!({"target_name": "P", "query": "q"}) },
                NodeDef { name: "bm25".into(), node_type: "BM25SearchNode".into(), config: json!({}) },
            ],
            edges: vec![EdgeDef {
                from_node: "source".into(), from_port: "query".into(),
                to_node: "bm25".into(), to_port: "nope".into(),
            }],
        };
        let e = only_err(build_definition(&bad_port, &nodes, &NodeTypePolicy::All));
        assert_eq!(e.kind(), "build");
        assert!(e.to_string().contains("nope") && e.to_string().contains("bm25"), "{e}");

        // Entrée requise non connectée : nommée aussi.
        let dangling = GraphDefinition {
            nodes: vec![NodeDef { name: "resolve".into(), node_type: "ResolveParentNode".into(), config: json!({}) }],
            edges: vec![],
        };
        let e = only_err(build_definition(&dangling, &nodes, &NodeTypePolicy::All));
        assert!(e.to_string().contains("resolve") && e.to_string().contains("results"), "{e}");
    }

    #[test]
    fn structural_errors_come_back_as_tool_content_too() {
        let (nodes, _) = builtin_graph_tools().unwrap();
        for def in [bare_def("BogusNode"), bare_def("BM25SearchNode")] {
            let content = run_definition_as_tool_content(
                &def,
                &nodes,
                Arc::new(ServiceRegistry::new()),
                &NodeTypePolicy::only(["SearchSourceNode"]),
                ("bm25", "results"),
            );
            let v: Value = serde_json::from_str(&content).unwrap();
            assert_eq!(v["error"], "forbidden_node_type", "{content}");
        }
    }

    // ── Sérialisation du port de résultat ───────────────────────────

    #[test]
    fn results_render_as_json() {
        use crate::search_strategy::UnifiedResult;
        let r = UnifiedResult {
            signal: None,
            uuid: "abc".into(),
            score: 1.5,
            entity: Some("Product".into()),
            data: None,
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
        };
        let rendered = render_port_value(&PortValue::new(vec![r])).unwrap();
        let v: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(v[0]["uuid"], "abc");
        assert_eq!(v[0]["entity"], "Product");
    }

    #[test]
    fn a_trigger_renders_as_ok() {
        assert_eq!(render_port_value(&PortValue::Trigger).unwrap(), r#"{"ok":true}"#);
    }
}
