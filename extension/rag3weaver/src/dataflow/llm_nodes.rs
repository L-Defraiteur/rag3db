//! `LlmNode` — le nœud de génération minimal (étape 1) : un prompt entre,
//! le texte sort. Le modèle est un service (`"llm"`, `Arc<dyn Llm>`), comme
//! `"embedder"` et `"ocr"` — le nœud ne charge rien lui-même.
//!
//! Le streaming existe déjà **dans le trait** ([`TokenSink`]) mais pas
//! encore *au travers du graphe* : ce nœud accumule dans un [`StringSink`]
//! et publie le texte entier sur son port de sortie. L'étape 3 remplacera
//! ce puits par une boîte aux lettres luciole (un [`crate::llm::ChannelSink`])
//! sans toucher ni au trait [`Llm`], ni à `Node::execute`, ni aux ports
//! déclarés ici — c'est précisément ce que le choix du puits achète.

use std::sync::Arc;

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeRegistry, NodeSchema};
use super::port::{take_or_clone, PortDef, PortType, PortValue};
use crate::llm::{GenOptions, Llm, LlmOutput, StringSink, Turn};
use crate::tools::tool_defs;

/// Clé du service LLM dans le [`super::ServiceRegistry`].
pub const LLM_SERVICE: &str = "llm";

/// Clé du registre de nœuds. Le doc de [`crate::tools`] pose que le
/// catalogue d'outils *est* le registre de nœuds ; pour que `with_tools`
/// puisse le lire, le catalogue le publie comme service à côté du LLM.
pub const NODE_REGISTRY_SERVICE: &str = "node_registry";

/// **Input** : `prompt` — `String` (un simple tour utilisateur) ou
/// `Vec<Turn>` (conversation complète). PortType::Text — le même que la
/// sortie `text` d'`OcrNode`, pour qu'un OCR puisse alimenter un LLM sans
/// adaptateur ; les deux charges utiles acceptées reprennent la
/// convention du port `image` d'`OcrNode`.
///
/// **Outputs** : `text` — `String`, la réponse entière (PortType::Text) ;
/// `llm` — [`LlmOutput`] (texte + raison de fin + comptage, PortType::Llm).
///
/// **Config** : `max_tokens` (512), `temperature` (0.0, glouton),
/// `top_p` (1.0), `stop` (liste de chaînes, vide), `with_tools` (`false` —
/// joint les outils du registre au prompt).
///
/// **Métriques** : `llm_prompt_tokens`, `llm_completion_tokens`, `llm_ms`,
/// `llm_tokens_per_s`.
pub struct LlmNode {
    node_name: String,
    opts: GenOptions,
    with_tools: bool,
}

impl LlmNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string(), opts: GenOptions::default(), with_tools: false }
    }

    pub fn with_options(mut self, opts: GenOptions) -> Self {
        self.opts = opts;
        self
    }

    /// Joint les outils du registre (service `"node_registry"`) au prompt.
    pub fn with_tools(mut self, yes: bool) -> Self {
        self.with_tools = yes;
        self
    }

    /// Lit le port `prompt` sous ses deux formes acceptées.
    fn take_turns(ctx: &mut NodeContext) -> Result<Vec<Turn>, String> {
        let pv = ctx.take_input("prompt").ok_or("LlmNode: missing 'prompt' input")?;
        if let PortValue::Data(ref arc) = pv {
            if arc.is::<Vec<Turn>>() {
                return take_or_clone::<Vec<Turn>>(pv)
                    .ok_or_else(|| "LlmNode: bad Vec<Turn> payload".to_string());
            }
        }
        let text = take_or_clone::<String>(pv)
            .ok_or("LlmNode: 'prompt' must carry String or Vec<Turn>")?;
        Ok(vec![Turn::user(text)])
    }
}

impl Node for LlmNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "LlmNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "prompt", port_type: PortType::Text, required: true }]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![
            PortDef { name: "text", port_type: PortType::Text, required: false },
            PortDef { name: "llm", port_type: PortType::Llm, required: false },
        ]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let llm = ctx
            .service::<Arc<dyn Llm>>(LLM_SERVICE)
            .cloned()
            .ok_or("LlmNode: 'llm' service not found")?;
        let turns = Self::take_turns(ctx)?;

        let mut opts = self.opts.clone();
        if self.with_tools {
            // Demander des outils et ne pas en avoir changerait le
            // comportement du modèle en silence : c'est une erreur.
            let registry = ctx
                .service::<Arc<NodeRegistry>>(NODE_REGISTRY_SERVICE)
                .ok_or("LlmNode: 'with_tools' is set but the 'node_registry' service is missing")?;
            opts.tools = tool_defs(registry);
        }

        // Étape 1 : on accumule. Étape 3 : ce puits devient une mailbox.
        let mut sink = StringSink::default();
        let (finish, usage) = llm
            .generate(&turns, &opts, &mut sink)
            .map_err(|e| format!("LlmNode ({}): {e}", llm.name()))?;

        ctx.metric("llm_prompt_tokens", usage.prompt_tokens as f64);
        ctx.metric("llm_completion_tokens", usage.completion_tokens as f64);
        ctx.metric("llm_ms", usage.ms as f64);
        ctx.metric("llm_tokens_per_s", usage.tokens_per_s());
        ctx.info(&format!(
            "LlmNode ({}): {} turns, {} tools -> {} tokens ({:?}), {} ms",
            llm.name(),
            turns.len(),
            opts.tools.len(),
            usage.completion_tokens,
            finish,
            usage.ms
        ));
        if !finish.is_complete() {
            ctx.warn(&format!("LlmNode ({}): incomplete answer ({finish:?})", llm.name()));
        }

        let out = LlmOutput { text: sink.text, finish, usage };
        ctx.set_output("text", PortValue::new(out.text.clone()));
        ctx.set_output("llm", PortValue::new(out));
        Ok(())
    }
}

// ─── Factory ─────────────────────────────────────────────────────────────────

pub struct LlmNodeFactory;

/// Lit un flottant borné, avec défaut.
fn opt_f32(config: &serde_json::Value, key: &str, default: f32, lo: f64, hi: f64) -> Result<f32, String> {
    match config.get(key) {
        None | Some(serde_json::Value::Null) => Ok(default),
        Some(v) => v
            .as_f64()
            .filter(|x| (lo..=hi).contains(x))
            .map(|x| x as f32)
            .ok_or_else(|| format!("LlmNode: '{key}' must be a number in [{lo}, {hi}]")),
    }
}

impl NodeFactory for LlmNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let max_tokens = match config.get("max_tokens") {
            None | Some(serde_json::Value::Null) => 512usize,
            Some(v) => v
                .as_u64()
                .filter(|n| *n > 0 && *n <= 1_000_000)
                .ok_or("LlmNode: 'max_tokens' must be a positive integer")? as usize,
        };
        let temperature = opt_f32(config, "temperature", 0.0, 0.0, 2.0)?;
        let top_p = opt_f32(config, "top_p", 1.0, 0.0, 1.0)?;
        let stop = match config.get("stop") {
            None | Some(serde_json::Value::Null) => Vec::new(),
            Some(serde_json::Value::Array(items)) => items
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| "LlmNode: 'stop' must be an array of strings".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err("LlmNode: 'stop' must be an array of strings".into()),
        };
        let with_tools = match config.get("with_tools") {
            None | Some(serde_json::Value::Null) => false,
            Some(v) => v.as_bool().ok_or("LlmNode: 'with_tools' must be a boolean")?,
        };

        let opts = GenOptions::default()
            .with_max_tokens(max_tokens)
            .with_temperature(temperature)
            .with_top_p(top_p)
            .with_stop(stop);
        Ok(Box::new(LlmNode::new(name).with_options(opts).with_tools(with_tools)))
    }

    fn node_type(&self) -> &'static str {
        "LlmNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "LlmNode",
            description: "Generates text from a prompt via the 'llm' service (streaming-capable)",
            inputs: vec![PortDef { name: "prompt", port_type: PortType::Text, required: true }],
            outputs: vec![
                PortDef { name: "text", port_type: PortType::Text, required: false },
                PortDef { name: "llm", port_type: PortType::Llm, required: false },
            ],
            config_params: vec![
                ConfigParam {
                    name: "max_tokens",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(512)),
                    description: "Maximum number of tokens to generate",
                },
                ConfigParam {
                    name: "temperature",
                    param_type: ConfigParamType::Float,
                    required: false,
                    default: Some(serde_json::json!(0.0)),
                    description: "Sampling temperature in [0, 2]; 0 is greedy and deterministic",
                },
                ConfigParam {
                    name: "top_p",
                    param_type: ConfigParamType::Float,
                    required: false,
                    default: Some(serde_json::json!(1.0)),
                    description: "Nucleus sampling threshold in [0, 1]",
                },
                ConfigParam {
                    name: "stop",
                    param_type: ConfigParamType::Json,
                    required: false,
                    default: None,
                    description: "Array of strings that end generation when produced",
                },
                ConfigParam {
                    name: "with_tools",
                    param_type: ConfigParamType::Bool,
                    required: false,
                    default: Some(serde_json::json!(false)),
                    description: "Expose every registered node type to the model as a tool",
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::services::ServiceRegistry;
    use crate::dataflow::{register_builtins, DataflowGraph, DataflowRuntime};
    use crate::llm::{Finish, MockLlm};

    fn services_with(llm: Arc<dyn Llm>) -> ServiceRegistry {
        let mut s = ServiceRegistry::new();
        s.register(LLM_SERVICE, llm);
        s
    }

    fn services_with_registry(llm: Arc<dyn Llm>) -> ServiceRegistry {
        let mut s = services_with(llm);
        let mut registry = NodeRegistry::new();
        register_builtins(&mut registry);
        s.register(NODE_REGISTRY_SERVICE, Arc::new(registry));
        s
    }

    fn mock() -> Arc<dyn Llm> {
        Arc::new(MockLlm::new("La réponse est 42"))
    }

    fn ctx_with(llm: Arc<dyn Llm>) -> NodeContext {
        NodeContext::with_services(Arc::new(services_with(llm)))
    }

    #[test]
    fn string_prompt_in_text_out() {
        let mut node = LlmNode::new("llm");
        let mut ctx = ctx_with(mock());
        ctx.set_input("prompt", PortValue::new("Quelle est la réponse ?".to_string()));
        node.execute(&mut ctx).unwrap();

        let mut outputs = ctx.drain_outputs();
        let text = outputs.remove("text").and_then(take_or_clone::<String>).unwrap();
        assert_eq!(text, "La réponse est 42");
        let out = outputs.remove("llm").and_then(take_or_clone::<LlmOutput>).unwrap();
        assert_eq!(out.finish, Finish::eos());
        assert_eq!(out.usage.completion_tokens, 4);

        let metrics = ctx.drain_metrics();
        assert_eq!(metrics["llm_completion_tokens"], 4.0);
        // "Quelle est la réponse ?" = 5 fragments
        assert_eq!(metrics["llm_prompt_tokens"], 5.0);
        assert!(metrics.contains_key("llm_ms"));
        assert!(metrics.contains_key("llm_tokens_per_s"));
    }

    #[test]
    fn conversation_prompt_is_accepted_too() {
        let mut node = LlmNode::new("llm");
        let mut ctx = ctx_with(mock());
        let turns = vec![Turn::system("sois bref"), Turn::user("salut")];
        ctx.set_input("prompt", PortValue::new(turns));
        node.execute(&mut ctx).unwrap();
        let text = ctx.drain_outputs().remove("text").and_then(take_or_clone::<String>).unwrap();
        assert_eq!(text, "La réponse est 42");
        // 2 tours ("sois bref" = 2 fragments, "salut" = 1) => 3
        assert_eq!(ctx.drain_metrics()["llm_prompt_tokens"], 3.0);
    }

    #[test]
    fn max_tokens_and_stop_reach_the_model() {
        let opts = GenOptions::default().with_max_tokens(2);
        let mut node = LlmNode::new("llm").with_options(opts);
        let mut ctx = ctx_with(mock());
        ctx.set_input("prompt", PortValue::new("q".to_string()));
        node.execute(&mut ctx).unwrap();
        let out = ctx.drain_outputs().remove("llm").and_then(take_or_clone::<LlmOutput>).unwrap();
        assert_eq!(out.text, "La réponse");
        assert_eq!(out.finish, Finish::max_tokens());

        let opts = GenOptions::default().with_stop(vec!["est".into()]);
        let mut node = LlmNode::new("llm").with_options(opts);
        let mut ctx = ctx_with(mock());
        ctx.set_input("prompt", PortValue::new("q".to_string()));
        node.execute(&mut ctx).unwrap();
        let out = ctx.drain_outputs().remove("llm").and_then(take_or_clone::<LlmOutput>).unwrap();
        assert_eq!(out.text, "La réponse ", "préfixe verbatim avant le stop");
        assert_eq!(out.finish, Finish::stop("est"));
    }

    #[test]
    fn with_tools_hands_the_whole_registry_to_the_model() {
        // Le LLM inspecte `opts.tools` et rend le nom du premier outil :
        // c'est la preuve que le registre traverse bien le nœud.
        let seen: Arc<dyn Llm> = Arc::new(crate::llm::CallbackLlm::new("spy", 4096, |_t, opts, sink| {
            let text = format!("{} outils, premier={}", opts.tools.len(), opts.tools[0].name);
            for f in crate::llm::fragments(&text) {
                sink.on_token(&f);
            }
            sink.on_finish(&Finish::eos());
            Ok((Finish::eos(), crate::llm::Usage::default()))
        }));
        let mut node = LlmNode::new("llm").with_tools(true);
        let mut ctx = NodeContext::with_services(Arc::new(services_with_registry(seen)));
        ctx.set_input("prompt", PortValue::new("q".to_string()));
        node.execute(&mut ctx).unwrap();
        let text = ctx.drain_outputs().remove("text").and_then(take_or_clone::<String>).unwrap();
        // 29 nœuds enregistrés, triés : le premier est BM25SearchNode.
        assert_eq!(text, format!("{} outils, premier=BM25SearchNode", crate::dataflow::node_factories::BUILTIN_NODE_COUNT));
    }

    #[test]
    fn with_tools_without_the_registry_is_an_error() {
        let mut node = LlmNode::new("llm").with_tools(true);
        let mut ctx = ctx_with(mock());
        ctx.set_input("prompt", PortValue::new("q".to_string()));
        let err = node.execute(&mut ctx).unwrap_err();
        assert!(err.contains("node_registry"), "{err}");
    }

    #[test]
    fn missing_service_or_input_is_an_error() {
        let mut node = LlmNode::new("llm");
        let mut ctx = NodeContext::new();
        ctx.set_input("prompt", PortValue::new("q".to_string()));
        assert!(node.execute(&mut ctx).unwrap_err().contains("'llm' service"));

        let mut ctx = ctx_with(mock());
        assert!(node.execute(&mut ctx).unwrap_err().contains("missing 'prompt'"));

        let mut ctx = ctx_with(mock());
        ctx.set_input("prompt", PortValue::new(42u32));
        assert!(node.execute(&mut ctx).unwrap_err().contains("String or Vec<Turn>"));
    }

    #[test]
    fn model_errors_are_reported_with_the_model_name() {
        let boom: Arc<dyn Llm> = Arc::new(crate::llm::CallbackLlm::new("boom", 8, |_t, _o, _s| {
            Err(crate::llm::LlmError::Model("gpu on fire".into()))
        }));
        let mut node = LlmNode::new("llm");
        let mut ctx = ctx_with(boom);
        ctx.set_input("prompt", PortValue::new("q".to_string()));
        let err = node.execute(&mut ctx).unwrap_err();
        assert!(err.contains("boom") && err.contains("gpu on fire"), "{err}");
    }

    #[test]
    fn factory_validates_its_config() {
        let f = LlmNodeFactory;
        assert!(f.create("a", &serde_json::json!({})).is_ok());
        assert!(f
            .create("a", &serde_json::json!({"max_tokens": 8, "temperature": 0.7, "top_p": 0.9,
                                             "stop": ["\n\n"], "with_tools": true}))
            .is_ok());
        assert!(f.create("a", &serde_json::json!({"max_tokens": 0})).is_err());
        assert!(f.create("a", &serde_json::json!({"max_tokens": -3})).is_err());
        assert!(f.create("a", &serde_json::json!({"temperature": 5.0})).is_err());
        assert!(f.create("a", &serde_json::json!({"top_p": 1.5})).is_err());
        assert!(f.create("a", &serde_json::json!({"stop": "x"})).is_err());
        assert!(f.create("a", &serde_json::json!({"stop": [1, 2]})).is_err());
        assert!(f.create("a", &serde_json::json!({"with_tools": "yes"})).is_err());

        let schema = f.schema();
        assert_eq!(schema.node_type, "LlmNode");
        assert_eq!(schema.inputs[0].port_type, PortType::Text);
        assert_eq!(schema.outputs.len(), 2);
        assert_eq!(schema.config_params.len(), 5);
    }

    #[test]
    fn factory_config_actually_reaches_the_model() {
        let node = LlmNodeFactory
            .create("llm", &serde_json::json!({"max_tokens": 1}))
            .unwrap();
        let mut node = node;
        let mut ctx = ctx_with(mock());
        ctx.set_input("prompt", PortValue::new("q".to_string()));
        node.execute(&mut ctx).unwrap();
        let text = ctx.drain_outputs().remove("text").and_then(take_or_clone::<String>).unwrap();
        assert_eq!(text, "La");
    }

    // ── Le nœud dans un vrai graphe ─────────────────────────────────────

    /// Source minimale : pose un prompt sur son port de sortie.
    struct PromptSource(String);
    impl Node for PromptSource {
        fn name(&self) -> &str {
            "source"
        }
        fn node_type(&self) -> &'static str {
            "PromptSource"
        }
        fn outputs(&self) -> Vec<PortDef> {
            vec![PortDef { name: "out", port_type: PortType::Text, required: false }]
        }
        fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
            ctx.set_output("out", PortValue::new(self.0.clone()));
            Ok(())
        }
    }

    #[test]
    fn llm_node_runs_inside_a_dataflow_graph() {
        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(PromptSource("dis quelque chose".into()))).unwrap();
        graph.add_node(Box::new(LlmNode::new("llm"))).unwrap();
        graph.connect("source", "out", "llm", "prompt").unwrap();

        let runtime = DataflowRuntime::with_services(10, services_with(mock()));
        let output = runtime.execute(&mut graph).unwrap();

        let text = output.get("llm", "text").cloned().and_then(take_or_clone::<String>).unwrap();
        assert_eq!(text, "La réponse est 42");
        let out = output.get("llm", "llm").cloned().and_then(take_or_clone::<LlmOutput>).unwrap();
        assert_eq!(out.finish, Finish::eos());
        assert_eq!(out.usage.completion_tokens, 4);
    }

    #[test]
    fn ocr_text_can_feed_the_llm_prompt_port() {
        // `connect` est l'endroit où les PortType sont vérifiés. `text`
        // d'OcrNode et `prompt` de LlmNode sont tous deux PortType::Text :
        // un OCR alimente un LLM sans adaptateur. (`validate` échouerait
        // ici pour une autre raison — l'entrée `image` de l'OCR n'est pas
        // branchée — ce n'est pas ce qu'on teste.)
        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(crate::dataflow::OcrNode::new("ocr"))).unwrap();
        graph.add_node(Box::new(LlmNode::new("llm"))).unwrap();
        graph.connect("ocr", "text", "llm", "prompt").unwrap();

        // Et le port typé `ocr` (PortType::Ocr) doit être refusé.
        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(crate::dataflow::OcrNode::new("ocr"))).unwrap();
        graph.add_node(Box::new(LlmNode::new("llm"))).unwrap();
        assert!(graph.connect("ocr", "ocr", "llm", "prompt").is_err());
    }
}
