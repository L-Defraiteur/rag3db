//! Node registry: declarative schemas, factories, and central instantiation.
//!
//! - [`NodeSchema`] — declares ports, config params for a node type
//! - [`NodeFactory`] — creates `Box<dyn Node>` from name + JSON config
//! - [`NodeRegistry`] — maps node_type → factory, enables checkpoint/mermaid integration

use std::collections::HashMap;

use super::node::Node;
use super::port::PortDef;

// ─── ConfigParam ─────────────────────────────────────────────────────────────

/// Type of a configuration parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigParamType {
    String,
    Int,
    Float,
    Bool,
    Json,
}

/// D'où viennent les valeurs admises d'un paramètre de chaîne.
///
/// Un modèle ne peut pas inventer `HAS_SIGNALS` si le schéma ne le permet
/// pas : une liste close devient un `enum` JSON Schema, et une valeur hors
/// liste est refusée avant d'instancier quoi que ce soit, avec la liste dans
/// l'erreur. Le savoir est déclaré **par le nœud** qui consomme le
/// paramètre (`FetchRelatedNode` sait que `direction` vaut `Outgoing` ou
/// `Incoming`) ; une fiche de graphe-outil qui expose ce paramètre en
/// hérite par câblage — voir `GraphTool::bind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choices {
    /// Une liste close, connue à l'écriture.
    Fixed(Vec<String>),
    /// Les cibles qu'accepte `Catalog::resolve_search_target` (`@targets`) —
    /// résolues contre le catalogue au moment où la fiche est rendue.
    Targets,
    /// Les relations déclarées dans le schéma du catalogue (`@relations`).
    Relations,
}

impl Choices {
    pub fn fixed<S: Into<String>, I: IntoIterator<Item = S>>(values: I) -> Self {
        Self::Fixed(values.into_iter().map(Into::into).collect())
    }

    /// La forme qui s'écrit dans une fiche (`%% choices: p = …`).
    pub fn spec(&self) -> String {
        match self {
            Self::Fixed(values) => values.join(" | "),
            Self::Targets => "@targets".to_string(),
            Self::Relations => "@relations".to_string(),
        }
    }

    /// Les valeurs, résolues contre un catalogue quand il en faut un, et un
    /// complément de description (les relations avec leurs extrémités : un
    /// `enum` ne sait pas dire que `DEFINED_IN` va de `Scope` à `File`).
    ///
    /// `None` quand la source manque (pas de catalogue) ou qu'elle est vide
    /// (rien d'enregistré) : un `enum` vide interdirait tout, et une fiche
    /// rendue sans catalogue reste utilisable — juste moins contrainte.
    pub fn resolve(&self, catalog: Option<&crate::catalog::Catalog>) -> Option<(Vec<String>, Option<String>)> {
        match self {
            Self::Fixed(values) => Some((values.clone(), None)),
            Self::Targets => {
                let names = catalog?.search_target_names();
                (!names.is_empty()).then_some((names, None))
            }
            Self::Relations => {
                let rels = catalog?.relation_summaries();
                if rels.is_empty() {
                    return None;
                }
                let hint = format!(
                    "Relations du schéma : {}.",
                    rels.iter()
                        .map(|(name, from, to)| format!("{name} ({from}→{to})"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                Some((rels.into_iter().map(|(name, _, _)| name).collect(), Some(hint)))
            }
        }
    }
}

/// Describes a configuration parameter accepted by a node factory.
#[derive(Debug, Clone)]
pub struct ConfigParam {
    pub name: &'static str,
    pub param_type: ConfigParamType,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub description: &'static str,
    /// Valeurs admises d'un paramètre `String` — `enum` dans le schéma,
    /// refus avant instanciation. `None` : chaîne libre.
    pub choices: Option<Choices>,
    /// Sous-schéma d'un paramètre `Json` — sans lui, un objet libre, ce qui
    /// est justement le cas où un décodage contraint ne borne plus rien.
    pub json_schema: Option<serde_json::Value>,
}

/// Refuse, au point central d'instanciation, une valeur hors d'une liste
/// close : une fabrique de base qui retombe silencieusement sur son défaut
/// (`direction` inconnue → `Outgoing`) ne doit pas masquer une faute de
/// frappe. Les listes du catalogue (`@targets`, `@relations`) demandent le
/// catalogue et se vérifient à la fiche.
pub fn check_fixed_choices(params: &[ConfigParam], config: &serde_json::Value) -> Result<(), String> {
    for p in params {
        let Some(Choices::Fixed(values)) = &p.choices else { continue };
        let Some(value) = config.get(p.name).and_then(|v| v.as_str()) else { continue };
        if !values.iter().any(|v| v == value) {
            return Err(format!(
                "'{}' : '{value}' n'est pas une valeur admise ; admises : {}",
                p.name,
                values.join(", ")
            ));
        }
    }
    Ok(())
}

// ─── NodeSchema ──────────────────────────────────────────────────────────────

/// Declarative schema for a node type: ports + config params.
///
/// Used for introspection, validation, Mermaid graph rendering, and documentation.
#[derive(Debug, Clone)]
pub struct NodeSchema {
    pub node_type: &'static str,
    pub description: &'static str,
    pub inputs: Vec<PortDef>,
    pub outputs: Vec<PortDef>,
    pub config_params: Vec<ConfigParam>,
}

// ─── NodeFactory ─────────────────────────────────────────────────────────────

/// Abstract factory for creating node instances from JSON config.
///
/// Heavy dependencies (DbConnection, Catalog, Embedder) are NOT passed at
/// creation time — they are resolved at execution time via `ctx.service()`.
pub trait NodeFactory: Send + Sync {
    /// Create a node instance.
    ///
    /// - `name` — unique instance name (e.g. "primary_search", "fetch_related_0")
    /// - `config` — JSON configuration (e.g. `{"relation": "HAS_FILE", "limit": 10}`)
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String>;

    /// Node type identifier (must match `Node::node_type()`).
    fn node_type(&self) -> &'static str;

    /// Declarative schema for this node type.
    fn schema(&self) -> NodeSchema;
}

// ─── NodeRegistry ────────────────────────────────────────────────────────────

/// Central registry mapping node_type → factory.
///
/// Used by checkpoint restore, Mermaid parser, and graph builder helpers.
pub struct NodeRegistry {
    factories: HashMap<&'static str, Box<dyn NodeFactory>>,
}

/// Les ports **déclarés** d'un type de nœud, lus là où ils le sont : le
/// schéma de sa fabrique.
///
/// Existe pour qu'il n'y ait qu'un seul rédacteur. Il y avait deux
/// écritures des mêmes ports — le schéma, que lit la composition de
/// graphes, et `Node::inputs`/`outputs`, que lit le montage d'un graphe
/// plat — et elles avaient dérivé : le port `meta` de `BM25SearchNode` et
/// de `RenderResultsNode` existait sur le nœud et manquait au schéma, donc
/// l'outil `search`, qui **compose**, ne le voyait pas.
///
/// Le schéma reste la source parce qu'il est le seul disponible **avant**
/// d'avoir une instance : le sondeur de `GraphNodeFactory::templated` lit
/// des schémas de type sur une définition où les `$param` peuvent encore
/// traîner (voir `graph_node.rs`).
pub fn ports_declares(factory: &dyn NodeFactory) -> (Vec<PortDef>, Vec<PortDef>) {
    let s = factory.schema();
    (s.inputs, s.outputs)
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Register a factory. Keyed by `factory.node_type()`.
    pub fn register(&mut self, factory: Box<dyn NodeFactory>) {
        self.factories.insert(factory.node_type(), factory);
    }

    /// Create a node instance from type, name, and config.
    pub fn create(
        &self,
        node_type: &str,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn Node>, String> {
        let factory = self
            .factories
            .get(node_type)
            .ok_or_else(|| format!("unknown node type: {node_type}"))?;
        check_fixed_choices(&factory.schema().config_params, config)
            .map_err(|e| format!("{node_type} '{name}': {e}"))?;
        factory.create(name, config)
    }

    /// Get the schema for a registered node type.
    pub fn schema(&self, node_type: &str) -> Option<NodeSchema> {
        self.factories.get(node_type).map(|f| f.schema())
    }

    /// List all registered node types.
    pub fn types(&self) -> Vec<&'static str> {
        self.factories.keys().copied().collect()
    }

    /// Check if a node type is registered.
    pub fn has(&self, node_type: &str) -> bool {
        self.factories.contains_key(node_type)
    }
}

// ─── simple_factory! macro ───────────────────────────────────────────────────

/// Generates a trivial NodeFactory for nodes whose constructor is `Type::new()` (no config).
///
/// Usage:
/// ```ignore
/// simple_factory!(ComposeNodeFactory, ComposeNode, "ComposeNode",
///     "Attaches fetched children to root results",
///     &[PortDef { name: "results", port_type: PortType::Results, required: true },
///       PortDef { name: "children", port_type: PortType::Children, required: false }],
///     &[PortDef { name: "results", port_type: PortType::Results, required: false }],
/// );
/// ```
#[macro_export]
macro_rules! simple_factory {
    ($factory:ident, $node:ty, $type_name:expr, $desc:expr, $inputs:expr, $outputs:expr $(,)?) => {
        pub struct $factory;

        impl $crate::dataflow::node_registry::NodeFactory for $factory {
            fn create(
                &self,
                _name: &str,
                _config: &serde_json::Value,
            ) -> Result<Box<dyn $crate::dataflow::node::Node>, String> {
                Ok(Box::new(<$node>::new()))
            }

            fn node_type(&self) -> &'static str {
                $type_name
            }

            fn schema(&self) -> $crate::dataflow::node_registry::NodeSchema {
                $crate::dataflow::node_registry::NodeSchema {
                    node_type: $type_name,
                    description: $desc,
                    inputs: $inputs.to_vec(),
                    outputs: $outputs.to_vec(),
                    config_params: vec![],
                }
            }
        }
    };
}

/// Like `simple_factory!` but for nodes whose constructor is `Type::new(name: &str)`.
#[macro_export]
macro_rules! named_factory {
    ($factory:ident, $node:ty, $type_name:expr, $desc:expr, $inputs:expr, $outputs:expr $(,)?) => {
        pub struct $factory;

        impl $crate::dataflow::node_registry::NodeFactory for $factory {
            fn create(
                &self,
                name: &str,
                _config: &serde_json::Value,
            ) -> Result<Box<dyn $crate::dataflow::node::Node>, String> {
                Ok(Box::new(<$node>::new(name)))
            }

            fn node_type(&self) -> &'static str {
                $type_name
            }

            fn schema(&self) -> $crate::dataflow::node_registry::NodeSchema {
                $crate::dataflow::node_registry::NodeSchema {
                    node_type: $type_name,
                    description: $desc,
                    inputs: $inputs.to_vec(),
                    outputs: $outputs.to_vec(),
                    config_params: vec![],
                }
            }
        }
    };
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::port::{PortDef, PortType, PortValue};
    use super::super::node::NodeContext;

    // ── Fake node for testing ────────────────────────────────────────

    struct FakeNode {
        node_name: String,
        #[allow(dead_code)] // présent pour la désérialisation, pas relu
        value: i64,
    }

    impl FakeNode {
        fn new() -> Self {
            Self { node_name: "fake".into(), value: 0 }
        }
    }

    
    impl Node for FakeNode {
        fn name(&self) -> &str { &self.node_name }
        fn inputs(&self) -> Vec<PortDef> { vec![] }
        fn outputs(&self) -> Vec<PortDef> {
            vec![PortDef { name: "out", port_type: PortType::Empty, required: false }]
        }
        fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
            ctx.set_output("out", PortValue::Trigger);
            Ok(())
        }
        fn node_type(&self) -> &'static str { "FakeNode" }
    }

    // ── Fake factory ─────────────────────────────────────────────────

    struct FakeNodeFactory;

    impl NodeFactory for FakeNodeFactory {
        fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
            let value = config.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(Box::new(FakeNode { node_name: name.into(), value }))
        }
        fn node_type(&self) -> &'static str { "FakeNode" }
        fn schema(&self) -> NodeSchema {
            NodeSchema {
                node_type: "FakeNode",
                description: "A fake node for testing",
                inputs: vec![],
                outputs: vec![PortDef { name: "out", port_type: PortType::Empty, required: false }],
                config_params: vec![ConfigParam {
                    name: "value",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(0)),
                    description: "A fake value",
                    choices: None,
                    json_schema: None,
                }],
            }
        }
    }

    // ── Tests ────────────────────────────────────────────────────────

    #[test]
    fn registry_create_node() {
        let mut registry = NodeRegistry::new();
        registry.register(Box::new(FakeNodeFactory));

        let node = registry.create("FakeNode", "my_fake", &serde_json::json!({"value": 42})).unwrap();
        assert_eq!(node.name(), "my_fake");
        assert_eq!(node.node_type(), "FakeNode");
    }

    #[test]
    fn registry_unknown_type_errors() {
        let registry = NodeRegistry::new();
        let result = registry.create("Nonexistent", "n", &serde_json::json!({}));
        match result {
            Err(e) => assert!(e.contains("unknown node type")),
            Ok(_) => panic!("expected error"),
        }
    }

    #[test]
    fn registry_schema() {
        let mut registry = NodeRegistry::new();
        registry.register(Box::new(FakeNodeFactory));

        let schema = registry.schema("FakeNode").unwrap();
        assert_eq!(schema.node_type, "FakeNode");
        assert_eq!(schema.outputs.len(), 1);
        assert_eq!(schema.config_params.len(), 1);
        assert_eq!(schema.config_params[0].name, "value");
    }

    #[test]
    fn registry_types_and_has() {
        let mut registry = NodeRegistry::new();
        assert!(!registry.has("FakeNode"));
        assert!(registry.types().is_empty());

        registry.register(Box::new(FakeNodeFactory));
        assert!(registry.has("FakeNode"));
        assert_eq!(registry.types(), vec!["FakeNode"]);
    }

    #[test]
    fn schema_no_type_returns_none() {
        let registry = NodeRegistry::new();
        assert!(registry.schema("Unknown").is_none());
    }

    // ── Macro tests ──────────────────────────────────────────────────

    simple_factory!(
        SimpleFakeFactory,
        FakeNode,
        "SimpleFake",
        "Simple fake node",
        &[],
        &[PortDef { name: "out", port_type: PortType::Empty, required: false }],
    );

    #[test]
    fn simple_factory_macro_works() {
        let factory = SimpleFakeFactory;
        assert_eq!(factory.node_type(), "SimpleFake");
        let node = factory.create("test", &serde_json::json!({})).unwrap();
        assert_eq!(node.node_type(), "FakeNode");

        let schema = factory.schema();
        assert_eq!(schema.node_type, "SimpleFake");
        assert!(schema.config_params.is_empty());
        assert_eq!(schema.outputs.len(), 1);
    }
}
