//! `LlmNode` — le nœud de génération : un prompt entre, le texte sort. Le
//! modèle est un service (`"llm"`, `Arc<dyn Llm>`), comme `"embedder"` et
//! `"ocr"` — le nœud ne charge rien lui-même.
//!
//! **Les jetons coulent au travers du graphe** (page
//! `docs/12-octobre-2026-recherche/01-le-premier-flux-de-jetons.md`) : un seul
//! puits émet chaque fragment sur la sortie `tokens` (un flux fini) et
//! l'accumule pour `text`. Qui lit le tout continue de lire le tout ; une
//! sortie `tokens` non branchée ne coûte rien (l'élément est jeté) et ne change
//! pas d'exécuteur. Le trait [`Llm`] n'a pas bougé : c'est ce que le choix du
//! puits achète.

use std::sync::Arc;

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeRegistry, NodeSchema};
use super::port::{take_or_clone, PortDef, PortType, PortValue};
use crate::llm::{Flow, GenOptions, Llm, LlmOutput, RetryEvent, TokenSink, Turn};
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
/// `llm` — [`LlmOutput`] (texte + raison de fin + comptage, PortType::Llm) ;
/// `tokens` — chaque fragment, au fil de la génération (PortType::Text, un
/// flux fini, optionnel). `text` est exactement la concaténation des fragments.
///
/// **Config** : `max_tokens` (512), `temperature` (0.0, glouton),
/// `top_p` (1.0), `stop` (liste de chaînes, vide), `with_tools` (`false` —
/// joint les outils du registre au prompt), `stop_when_unread` (`false` —
/// arrêter la génération quand plus personne ne lit `tokens` ; par défaut,
/// seule l'annulation du run l'arrête, puisque `text` peut être lu ailleurs).
///
/// **Métriques** : `llm_prompt_tokens`, `llm_completion_tokens`, `llm_ms`,
/// `llm_tokens_per_s`.
pub struct LlmNode {
    node_name: String,
    opts: GenOptions,
    with_tools: bool,
    stop_when_unread: bool,
}

/// **Le puits double** : chaque fragment part sur `tokens` et s'accumule pour
/// `text`. Ce que le modèle a produit est dans `text`, même quand le lecteur
/// de `tokens` lâche et que la génération s'arrête.
struct AnswerSink<'a> {
    text: String,
    tokens: super::node::EmitSink<'a>,
}

impl TokenSink for AnswerSink<'_> {
    fn on_token(&mut self, delta: &str) -> Flow {
        self.text.push_str(delta);
        self.tokens.on_token(delta)
    }

    fn on_retry(&mut self, event: &RetryEvent<'_>) -> Flow {
        self.tokens.on_retry(event)
    }
}

impl LlmNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string(), opts: GenOptions::default(), with_tools: false, stop_when_unread: false }
    }

    /// Arrêter la génération quand plus personne ne lit `tokens`.
    pub fn with_stop_when_unread(mut self, yes: bool) -> Self {
        self.stop_when_unread = yes;
        self
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
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::llm_nodes::LlmNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::llm_nodes::LlmNodeFactory).1
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

        // Chaque fragment sur `tokens`, et le tout pour `text`.
        let (finish, usage, text) = {
            let tokens = if self.stop_when_unread {
                ctx.token_sink_stop_when_unread("tokens")
            } else {
                ctx.token_sink("tokens")
            };
            let mut sink = AnswerSink { text: String::new(), tokens };
            let (finish, usage) = llm
                .generate(&turns, &opts, &mut sink)
                .map_err(|e| format!("LlmNode ({}): {e}", llm.name()))?;
            (finish, usage, sink.text)
        };

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

        let out = LlmOutput { text, finish, usage };
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
        let stop_when_unread = match config.get("stop_when_unread") {
            None | Some(serde_json::Value::Null) => false,
            Some(v) => v.as_bool().ok_or("LlmNode: 'stop_when_unread' must be a boolean")?,
        };

        let opts = GenOptions::default()
            .with_max_tokens(max_tokens)
            .with_temperature(temperature)
            .with_top_p(top_p)
            .with_stop(stop);
        Ok(Box::new(
            LlmNode::new(name).with_options(opts).with_tools(with_tools).with_stop_when_unread(stop_when_unread),
        ))
    }

    fn node_type(&self) -> &'static str {
        "LlmNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "LlmNode".into(),
            description: "Generates text from a prompt via the 'llm' service (streaming-capable)".into(),
            inputs: vec![PortDef::value("prompt", PortType::Text, true)],
            outputs: vec![
                PortDef::value("text", PortType::Text, false),
                PortDef::value("llm", PortType::Llm, false),
                PortDef::value("tokens", PortType::Text, false).stream(super::port::PortKind::Finite),
            ],
            config_params: vec![
                ConfigParam {
                    name: "max_tokens".into(),
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(512)),
                    description: "Maximum number of tokens to generate".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "temperature".into(),
                    param_type: ConfigParamType::Float,
                    required: false,
                    default: Some(serde_json::json!(0.0)),
                    description: "Sampling temperature in [0, 2]; 0 is greedy and deterministic".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "top_p".into(),
                    param_type: ConfigParamType::Float,
                    required: false,
                    default: Some(serde_json::json!(1.0)),
                    description: "Nucleus sampling threshold in [0, 1]".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "stop".into(),
                    param_type: ConfigParamType::Json,
                    required: false,
                    default: None,
                    description: "Array of strings that end generation when produced".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "with_tools".into(),
                    param_type: ConfigParamType::Bool,
                    required: false,
                    default: Some(serde_json::json!(false)),
                    description: "Expose every registered node type to the model as a tool".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "stop_when_unread".into(),
                    param_type: ConfigParamType::Bool,
                    required: false,
                    default: Some(serde_json::json!(false)),
                    description: "Stop generating when nobody reads 'tokens' any more (by default only the run's cancellation stops it, since 'text' may be read elsewhere)".into(),
                    choices: None,
                    json_schema: None,
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
        // Les nœuds enregistrés, **triés par nom** — l'ordre doit être stable
        // d'une exécution à l'autre, sinon le préfixe du prompt change et la
        // mise en cache tombe. Le premier suit donc l'alphabet, pas l'ordre
        // d'enregistrement : `AdoptTemplateNode` du 30 août 2026 au 4 octobre,
        // puis `AddRefNode`, qui le précède à la lettre près (`AddR` < `AdoP`).
        //
        // **Ce que ce test protège n'est pas ce nom-là**, c'est que l'ordre
        // soit alphabétique et stable. Un nœud neuf qui change le premier le
        // fait donc légitimement tomber, et la bonne correction est de mettre
        // le nom à jour — pas d'assouplir l'assertion, qui ne verrait plus un
        // ordre devenu celui de l'enregistrement.
        let premier = if cfg!(feature = "code") { "AddRefNode" } else { "AddRefNode" };
        assert_eq!(
            text,
            format!("{} outils, premier={premier}", crate::dataflow::node_factories::BUILTIN_NODE_COUNT)
        );
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
        // text, llm, et le flux tokens ; stop_when_unread s'ajoute aux cinq réglages.
        assert_eq!(schema.outputs.len(), 3);
        assert_eq!(schema.config_params.len(), 6);
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
            vec![PortDef::value("out", PortType::Text, false)]
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

    // ── Les jetons en flux (page 12-octobre-2026-recherche/01) ─────────

    /// Lit `tokens` fragment par fragment, signale le premier, et lâche son
    /// entrée après `stop_after` fragments s'il est donné.
    struct TokenReader {
        seen: Arc<std::sync::Mutex<Vec<String>>>,
        first: Option<std::sync::mpsc::Sender<()>>,
        stop_after: Option<usize>,
    }
    impl Node for TokenReader {
        fn name(&self) -> &str {
            "lecteur"
        }
        fn node_type(&self) -> &'static str {
            "TokenReader"
        }
        fn inputs(&self) -> Vec<PortDef> {
            vec![PortDef::value("tokens", PortType::Text, true).posture(crate::dataflow::port::Posture::PerElement)]
        }
        fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
            while let Some(v) = ctx.next("tokens").map_err(|e| format!("{e:?}"))? {
                let t = take_or_clone::<String>(v).ok_or("tokens : une chaîne attendue")?;
                let mut seen = self.seen.lock().unwrap();
                seen.push(t);
                if let Some(tx) = self.first.take() {
                    let _ = tx.send(());
                }
                if self.stop_after.is_some_and(|n| seen.len() >= n) {
                    return Ok(());
                }
            }
            Ok(())
        }
    }

    fn graphe_a_jetons(llm: LlmNode, lecteur: TokenReader) -> DataflowGraph {
        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(PromptSource("parle".into()))).unwrap();
        graph.add_node(Box::new(llm)).unwrap();
        graph.add_node(Box::new(lecteur)).unwrap();
        graph.connect("source", "out", "llm", "prompt").unwrap();
        graph.connect("llm", "tokens", "lecteur", "tokens").unwrap();
        graph
    }

    /// **Les jetons coulent pendant la génération** (témoins 1 et 2) : le
    /// modèle ne rend son dernier fragment qu'après que l'aval a lu le premier
    /// — une poignée de main impossible si le nœud publiait à la fin. Et
    /// `text` est la concaténation exacte des fragments lus.
    #[test]
    fn tokens_flow_while_the_model_generates() {
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let rx = std::sync::Mutex::new(rx);
        let vu_avant_la_fin = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let drapeau = vu_avant_la_fin.clone();
        let llm = crate::llm::CallbackLlm::new("poignée", 4096, move |_, _, sink| {
            sink.on_token("bon");
            let lu = rx.lock().unwrap().recv_timeout(std::time::Duration::from_secs(5)).is_ok();
            drapeau.store(lu, std::sync::atomic::Ordering::SeqCst);
            sink.on_token("jour");
            Ok((Finish::eos(), crate::llm::Usage { completion_tokens: 2, ..Default::default() }))
        });
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut graph = graphe_a_jetons(
            LlmNode::new("llm"),
            TokenReader { seen: seen.clone(), first: Some(tx), stop_after: None },
        );
        assert!(graph.has_streams().unwrap(), "tokens branché : l'exécuteur concurrent");
        let output = DataflowRuntime::with_services(10, services_with(Arc::new(llm))).execute(&mut graph).unwrap();
        assert!(vu_avant_la_fin.load(std::sync::atomic::Ordering::SeqCst), "le premier jeton n'a pas été lu avant la fin de la génération");
        let text = output.get("llm", "text").cloned().and_then(take_or_clone::<String>).unwrap();
        assert_eq!(*seen.lock().unwrap(), ["bon", "jour"]);
        assert_eq!(text, seen.lock().unwrap().concat(), "text est la concaténation des jetons");
    }

    /// Un modèle de cent fragments qui compte ce qu'il produit, et s'arrête au
    /// premier `Flow::Stop`.
    fn cent_fragments(produits: Arc<std::sync::atomic::AtomicUsize>) -> crate::llm::CallbackLlm {
        crate::llm::CallbackLlm::new("cent", 4096, move |_, _, sink| {
            for i in 0..100 {
                produits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if sink.on_token(&format!("t{i} ")) == crate::llm::Flow::Stop {
                    return Ok((Finish::cancelled(), crate::llm::Usage { completion_tokens: i + 1, ..Default::default() }));
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            Ok((Finish::eos(), crate::llm::Usage { completion_tokens: 100, ..Default::default() }))
        })
    }

    /// **L'arrêt anticipé** (témoin 4) : le lecteur lâche `tokens` après trois
    /// fragments. Avec `stop_when_unread`, le modèle s'arrête : moins de cent
    /// fragments produits, `llm.finish` vaut `Cancelled`. Sans le réglage, il
    /// les produit tous et `text` est complet, puisque `text` peut être lu ailleurs.
    #[test]
    fn stop_when_unread_stops_the_model_when_the_reader_lets_go() {
        for arreter in [true, false] {
            let produits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let llm = cent_fragments(produits.clone());
            let node = LlmNodeFactory
                .create("llm", &serde_json::json!({"stop_when_unread": arreter}))
                .unwrap();
            let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
            let mut graph = DataflowGraph::new();
            graph.add_node(Box::new(PromptSource("parle".into()))).unwrap();
            graph.add_node(node).unwrap();
            graph.add_node(Box::new(TokenReader { seen: seen.clone(), first: None, stop_after: Some(3) })).unwrap();
            graph.connect("source", "out", "llm", "prompt").unwrap();
            graph.connect("llm", "tokens", "lecteur", "tokens").unwrap();
            let output = DataflowRuntime::with_services(10, services_with(Arc::new(llm))).execute(&mut graph).unwrap();
            let out = output.get("llm", "llm").cloned().and_then(take_or_clone::<LlmOutput>).unwrap();
            let n = produits.load(std::sync::atomic::Ordering::SeqCst);
            if arreter {
                assert!(n < 100, "stop_when_unread : {n} fragments produits sur 100");
                assert_eq!(out.finish, Finish::cancelled());
            } else {
                assert_eq!(n, 100, "sans le réglage, tous les fragments");
                assert_eq!(out.finish, Finish::eos());
                assert_eq!(out.text, (0..100).map(|i| format!("t{i} ")).collect::<String>());
            }
            assert_eq!(seen.lock().unwrap().len(), 3);
        }
    }

    /// **Le réglage et le port se lisent dans le schéma** : `tokens` est un flux
    /// fini optionnel, `stop_when_unread` un booléen faux par défaut.
    #[test]
    fn the_schema_says_the_tokens_stream_and_its_setting() {
        let schema = LlmNodeFactory.schema();
        let tokens = schema.outputs.iter().find(|p| p.name == "tokens").expect("une sortie tokens");
        assert_eq!(tokens.kind, crate::dataflow::port::PortKind::Finite);
        assert!(!tokens.required);
        let reglage = schema.config_params.iter().find(|c| c.name == "stop_when_unread").expect("le réglage");
        assert_eq!(reglage.default, Some(serde_json::json!(false)));
        assert!(LlmNodeFactory.create("llm", &serde_json::json!({"stop_when_unread": "oui"})).is_err());
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
