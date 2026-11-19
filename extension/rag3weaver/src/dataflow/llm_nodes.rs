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
    /// La forme imposée à la réponse (`response_schema`) : envoyée en format
    /// strict, et la réponse lue en JSON sur le port `json`.
    response_schema: Option<serde_json::Value>,
    /// The task line (lot B1): a declared parameter, short, set before the context parts.
    instruction: Option<String>,
}

/// **La réponse lue sous son schéma** : du JSON, de la forme de la racine du
/// schéma (`type`), avec ses clés `required` si c'est un objet. Un modèle qui
/// n'honore pas `response_format` (un local, un faux serveur) rend du texte :
/// l'erreur le dit, on ne devine pas un JSON dans une phrase.
fn read_under_schema(texte: &str, schema: &serde_json::Value) -> Result<serde_json::Value, String> {
    let head: String = texte.trim().chars().take(200).collect();
    let v: serde_json::Value = serde_json::from_str(texte.trim())
        .map_err(|e| format!("la réponse n'est pas du JSON ({e}) alors qu'un response_schema est déclaré : « {head} »"))?;
    let expected = schema.get("type").and_then(serde_json::Value::as_str);
    let ok = match expected {
        Some("object") => v.is_object(),
        Some("array") => v.is_array(),
        Some("string") => v.is_string(),
        Some("integer") => v.is_i64() || v.is_u64(),
        Some("number") => v.is_number(),
        Some("boolean") => v.is_boolean(),
        _ => true,
    };
    if !ok {
        return Err(format!("la réponse n'a pas la forme du response_schema (type {}) : « {head} »", expected.unwrap_or("?")));
    }
    if let (Some(obj), Some(req)) = (v.as_object(), schema.get("required").and_then(serde_json::Value::as_array)) {
        let missing: Vec<&str> = req.iter().filter_map(serde_json::Value::as_str).filter(|k| !obj.contains_key(*k)).collect();
        if !missing.is_empty() {
            return Err(format!("la réponse n'a pas les clés requises du response_schema ({}) : « {head} »", missing.join(", ")));
        }
    }
    Ok(v)
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
        Self { node_name: name.to_string(), opts: GenOptions::default(), with_tools: false, stop_when_unread: false, response_schema: None, instruction: None }
    }

    /// Impose une forme JSON à la réponse ; elle sort aussi sur `json`.
    pub fn with_response_schema(mut self, schema: serde_json::Value) -> Self {
        self.response_schema = Some(schema);
        self
    }

    /// The task line, set before the context parts (lot B1).
    pub fn with_instruction(mut self, instruction: String) -> Self {
        self.instruction = Some(instruction);
        self
    }

    /// **The prompt of a declared context** (lot B1, 19 November 2026): the `instruction`, then each part of the
    /// `context` list in order as a section, then the `prompt` text if one comes — the variable last, for the
    /// provider's prefix cache. `None` when the node has neither an instruction nor a context: the `prompt`/`value`
    /// ports are read as before.
    fn compose(&self, ctx: &mut NodeContext) -> Result<Option<Vec<Turn>>, String> {
        let context = ctx.take_input("context");
        if context.is_none() && self.instruction.is_none() {
            return Ok(None);
        }
        let mut pieces: Vec<String> = Vec::new();
        if let Some(i) = self.instruction.as_ref().filter(|i| !i.is_empty()) {
            pieces.push(i.clone());
        }
        if let Some(pv) = context {
            let parts = take_or_clone::<serde_json::Value>(pv).ok_or("LlmNode: 'context' carries a list of parts {name, title, value, format?, max_tokens?}")?;
            let parts = match parts {
                serde_json::Value::Array(a) => a,
                serde_json::Value::Null => Vec::new(),
                other => vec![other],
            };
            for part in &parts {
                if let Some(section) = render_part(part)? {
                    pieces.push(section);
                }
            }
        }
        if let Some(pv) = ctx.take_input("prompt") {
            let text = take_or_clone::<String>(pv).ok_or("LlmNode: with a context, 'prompt' carries a text")?;
            if !text.is_empty() {
                pieces.push(text);
            }
        }
        if pieces.is_empty() {
            return Err("LlmNode: the instruction and the context are both empty: no prompt".into());
        }
        Ok(Some(vec![Turn::user(pieces.join("\n\n"))]))
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
        let pv = match ctx.take_input("prompt") {
            Some(pv) => pv,
            None => {
                let v = ctx.take_input("value").ok_or("LlmNode: missing 'prompt' input")?;
                return match v.downcast::<serde_json::Value>() {
                    Some(serde_json::Value::String(s)) => Ok(vec![Turn::user(s.clone())]),
                    Some(serde_json::Value::Object(m)) if m.get("prompt").and_then(serde_json::Value::as_str).is_some() => {
                        Ok(vec![Turn::user(m["prompt"].as_str().unwrap_or_default().to_string())])
                    }
                    _ => Err("LlmNode: 'value' must carry a text, or {prompt}".into()),
                };
            }
        };
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
        let turns = match self.compose(ctx)? {
            Some(turns) => turns,
            None => Self::take_turns(ctx)?,
        };

        let mut opts = self.opts.clone();
        if self.with_tools {
            // Demander des outils et ne pas en avoir changerait le
            // comportement du modèle en silence : c'est une erreur.
            let registry = ctx
                .service::<Arc<NodeRegistry>>(NODE_REGISTRY_SERVICE)
                .ok_or("LlmNode: 'with_tools' is set but the 'node_registry' service is missing")?;
            opts.tools = tool_defs(registry);
        }
        if let Some(schema) = &self.response_schema {
            opts = opts.with_response_format(crate::llm::ResponseFormat::strict_schema("response", schema.clone()));
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

        // Sous un schéma, la réponse se lit avant tout envoi : une réponse non
        // conforme est une erreur, et aucun port ne sort à moitié.
        let json = match &self.response_schema {
            Some(schema) => Some(
                read_under_schema(&text, schema).map_err(|e| format!("LlmNode ({}) : {e}", llm.name()))?,
            ),
            None => None,
        };
        let out = LlmOutput { text, finish, usage };
        if let Some(v) = json {
            ctx.set_output("json", PortValue::new(v));
        }
        ctx.set_output("text", PortValue::new(out.text.clone()));
        ctx.set_output("llm", PortValue::new(out));
        Ok(())
    }
}

// ─── Factory ─────────────────────────────────────────────────────────────────

/// A JSON value with its object keys sorted, at every depth: the same value always renders the same bytes.
fn sorted(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(o) => {
            let mut keys: Vec<&String> = o.keys().collect();
            keys.sort();
            let mut m = serde_json::Map::new();
            for k in keys {
                m.insert(k.clone(), sorted(&o[k]));
            }
            serde_json::Value::Object(m)
        }
        serde_json::Value::Array(a) => serde_json::Value::Array(a.iter().map(sorted).collect()),
        other => other.clone(),
    }
}

/// `json` (the default): compact, keys sorted, one line per element of a list. A string is never quoted.
fn render_json(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(a) => a.iter().map(|x| match x {
            serde_json::Value::String(s) => s.clone(),
            other => sorted(other).to_string(),
        }).collect::<Vec<_>>().join("\n"),
        other => sorted(other).to_string(),
    }
}

/// `kv`: an object as « key: value » lines, keys sorted (a string value as it is, any other as compact JSON);
/// a list as its elements so rendered, separated by a blank line.
fn render_kv(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Object(o) => {
            let mut keys: Vec<&String> = o.keys().collect();
            keys.sort();
            keys.iter().map(|k| match &o[*k] {
                serde_json::Value::String(s) => format!("{k}: {s}"),
                other => format!("{k}: {}", sorted(other)),
            }).collect::<Vec<_>>().join("\n")
        }
        serde_json::Value::Array(a) => a.iter().map(render_kv).collect::<Vec<_>>().join("\n\n"),
        other => render_json(other),
    }
}

/// One context part as a section: « ## title », a blank line, the value rendered by its `format` (json, the
/// default, or kv), cut to `max_tokens` (estimated: characters / 4, rounded up) with the cut said. An empty part
/// renders nothing.
fn render_part(part: &serde_json::Value) -> Result<Option<String>, String> {
    let o = part.as_object().ok_or("LlmNode: a context part is an object {name, title, value, format?, max_tokens?}")?;
    let name = o.get("name").and_then(serde_json::Value::as_str).unwrap_or("?");
    let title = o
        .get("title")
        .and_then(serde_json::Value::as_str)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| format!("LlmNode: the context part « {name} » has no title"))?;
    let value = o.get("value").unwrap_or(&serde_json::Value::Null);
    if crate::dataflow::collect_node::is_empty(value) {
        return Ok(None);
    }
    let mut text = match o.get("format").and_then(serde_json::Value::as_str) {
        None | Some("json") => render_json(value),
        Some("kv") => render_kv(value),
        Some(other) => return Err(format!("LlmNode: the context part « {name} » asks for the format « {other} »; known: json, kv")),
    };
    if let Some(max) = o.get("max_tokens").filter(|v| !v.is_null()) {
        let max = max.as_u64().filter(|m| *m > 0).ok_or_else(|| format!("LlmNode: the context part « {name} »: max_tokens is a positive integer"))? as usize;
        let chars = text.chars().count();
        let tokens = chars.div_ceil(4);
        if tokens > max {
            text = text.chars().take(max * 4).collect::<String>();
            text.push_str(&format!("\n[cut: {max} of {tokens} estimated tokens kept]"));
        }
    }
    Ok(Some(format!("## {title}\n\n{text}")))
}

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

        let response_schema = match config.get("response_schema") {
            None | Some(serde_json::Value::Null) => None,
            Some(v @ serde_json::Value::Object(m)) if !m.is_empty() => Some(v.clone()),
            Some(_) => return Err("LlmNode: 'response_schema' must be a non-empty JSON Schema object".into()),
        };
        let opts = GenOptions::default()
            .with_max_tokens(max_tokens)
            .with_temperature(temperature)
            .with_top_p(top_p)
            .with_stop(stop);
        let mut node = LlmNode::new(name).with_options(opts).with_tools(with_tools).with_stop_when_unread(stop_when_unread);
        match config.get("instruction") {
            None | Some(serde_json::Value::Null) => {}
            Some(serde_json::Value::String(i)) => node = node.with_instruction(i.clone()),
            Some(_) => return Err("LlmNode: 'instruction' is a text".into()),
        }
        Ok(Box::new(match response_schema {
            Some(schema) => node.with_response_schema(schema),
            None => node,
        }))
    }

    fn node_type(&self) -> &'static str {
        "LlmNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "LlmNode".into(),
            description: "Generates text from a prompt via the 'llm' service (streaming-capable)".into(),
            // `prompt` (Text) : un texte ou une conversation ; `value` (Map) : un texte rendu en JSON par
            // un RhaiNode, ou {prompt}. L'un des deux, au moment de jouer.
            // `context` (lot B1): a list of parts {name, title, value, format?, max_tokens?}, usually from a
            // Collect; rendered after `instruction`, and before a `prompt` text if one comes.
            inputs: vec![
                PortDef::value("prompt", PortType::Text, false),
                PortDef::value("value", PortType::Map, false),
                PortDef::value("context", PortType::Map, false),
            ],
            outputs: vec![
                PortDef::value("text", PortType::Text, false),
                PortDef::value("llm", PortType::Llm, false),
                PortDef::value("tokens", PortType::Text, false).stream(super::port::PortKind::Finite),
                PortDef::value("json", PortType::Map, false),
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
                    name: "instruction".into(),
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "The task line, short: set first, before the context parts. The meaning goes in the descriptions of response_schema's fields".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "response_schema".into(),
                    param_type: ConfigParamType::Json,
                    required: false,
                    default: None,
                    description: "A JSON Schema the reply must follow, sent as a strict response_format; the reply is then read as JSON onto the 'json' port, and a reply that is not JSON of that shape is an error naming response_schema (a model that ignores response_format is not guessed)".into(),
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

    /// Un modèle qui rend un texte fixe et note le `response_format` reçu.
    fn replies_with(text: &'static str, seen: Arc<std::sync::Mutex<Option<crate::llm::ResponseFormat>>>) -> Arc<dyn Llm> {
        Arc::new(crate::llm::CallbackLlm::new("spy", 4096, move |_t, opts, sink| {
            *seen.lock().unwrap() = opts.response_format.clone();
            for f in crate::llm::fragments(text) {
                sink.on_token(&f);
            }
            sink.on_finish(&Finish::eos());
            Ok((Finish::eos(), crate::llm::Usage::default()))
        }))
    }

    fn topics_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {"topics": {"type": "array", "items": {"type": "object",
                "properties": {"title": {"type": "string"}, "from": {"type": "integer"}, "to": {"type": "integer"}},
                "required": ["title", "from", "to"], "additionalProperties": false}}},
            "required": ["topics"], "additionalProperties": false
        })
    }

    /// **`response_schema`** : le schéma va au modèle en format strict, et la
    /// réponse, du JSON, sort sur le port `json` (le texte reste sur `text`).
    #[test]
    fn response_schema_reaches_the_model_and_the_json_comes_out() {
        let seen = Arc::new(std::sync::Mutex::new(None));
        let reply = r#"{"topics": [{"title": "la reprise", "from": 1, "to": 3}]}"#;
        let mut node = LlmNodeFactory.create("seg", &serde_json::json!({"response_schema": topics_schema()})).unwrap();
        let mut ctx = ctx_with(replies_with(reply, seen.clone()));
        ctx.set_input("prompt", PortValue::new("découpe".to_string()));
        node.execute(&mut ctx).unwrap();
        match seen.lock().unwrap().clone() {
            Some(crate::llm::ResponseFormat::JsonSchema { schema, strict, .. }) => {
                assert!(strict, "le format est strict");
                assert_eq!(schema, topics_schema());
            }
            autre => panic!("le schéma n'a pas atteint le modèle : {autre:?}"),
        }
        let mut out = ctx.drain_outputs();
        let json = out.remove("json").and_then(take_or_clone::<serde_json::Value>).expect("le port json");
        assert_eq!(json["topics"][0]["title"], "la reprise");
        assert_eq!(out.remove("text").and_then(take_or_clone::<String>).as_deref(), Some(reply));
        assert!(LlmNodeFactory.schema().outputs.iter().any(|p| p.name == "json"), "le port json est déclaré");
    }

    /// **Un modèle qui n'honore pas le format** (local, faux serveur) : le nœud le
    /// dit par une erreur nommée, et le port `json` reste vide ; il ne devine pas.
    #[test]
    fn a_reply_that_is_not_json_is_refused_by_name() {
        for reply in ["Voici les sujets : la reprise, la météo.", r#"{"autre": 1}"#] {
            let seen = Arc::new(std::sync::Mutex::new(None));
            let mut node = LlmNodeFactory.create("seg", &serde_json::json!({"response_schema": topics_schema()})).unwrap();
            let mut ctx = ctx_with(replies_with(reply, seen));
            ctx.set_input("prompt", PortValue::new("découpe".to_string()));
            let e = node.execute(&mut ctx).unwrap_err();
            assert!(e.contains("response_schema"), "l'erreur nomme response_schema : {e}");
            assert!(ctx.drain_outputs().remove("json").is_none(), "le port json reste vide");
        }
    }

    /// Un `response_schema` qui n'est pas un objet est refusé à la création.
    #[test]
    fn a_response_schema_that_is_not_an_object_is_refused() {
        let e = LlmNodeFactory.create("seg", &serde_json::json!({"response_schema": "topics"})).err().expect("refusé");
        assert!(e.contains("response_schema"), "{e}");
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
        // text, llm, le flux tokens, et json (la réponse sous response_schema) ; stop_when_unread et
        // response_schema s'ajoutent aux cinq réglages, puis instruction (lot B1).
        assert_eq!(schema.outputs.len(), 4);
        assert_eq!(schema.config_params.len(), 8);
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

    // ── Lot B1 (19 November 2026): the context of a model call, declared ──

    /// A model that notes the text of the prompt it receives, and replies JSON.
    fn notes_prompt(seen: Arc<std::sync::Mutex<String>>) -> Arc<dyn Llm> {
        Arc::new(crate::llm::CallbackLlm::new("spy", 4096, move |turns, _opts, sink| {
            *seen.lock().unwrap() = turns.iter().map(|t| t.content.clone()).collect::<Vec<_>>().join("\n---\n");
            for f in crate::llm::fragments("{\"topics\": []}") {
                sink.on_token(&f);
            }
            sink.on_finish(&Finish::eos());
            Ok((Finish::eos(), crate::llm::Usage::default()))
        }))
    }

    fn registry() -> NodeRegistry {
        let mut r = NodeRegistry::new();
        register_builtins(&mut r);
        r
    }

    /// **Collect** gathers its named inputs into one list, in the order its configuration declares them,
    /// whatever the order they arrive in: two branches that finish in disorder give the same list. An empty
    /// input (a disabled ingredient) is dropped, or kept with `keep_empty`.
    #[test]
    fn collect_gives_its_inputs_in_declared_order_whatever_their_arrival() {
        let r = registry();
        let parts = |a: &str| serde_json::json!({"name": a, "title": a, "value": format!("text of {a}")});
        let mut lists = Vec::new();
        for arrival in [["slice", "identity", "report"], ["report", "slice", "identity"]] {
            let mut node = r.create("Collect", "c", &serde_json::json!({"inputs": "identity,report,slice"})).expect("Collect is a node");
            let mut ctx = NodeContext::with_services(Arc::new(ServiceRegistry::new()));
            for name in arrival {
                ctx.set_input(name, PortValue::new(parts(name)));
            }
            node.execute(&mut ctx).unwrap();
            let list = ctx.drain_outputs().remove("list").and_then(take_or_clone::<serde_json::Value>).expect("the list port");
            lists.push(list);
        }
        assert_eq!(lists[0], lists[1], "the same list whatever the arrival");
        let names: Vec<&str> = lists[0].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["identity", "report", "slice"]);
        // An empty input: dropped by default, kept with keep_empty.
        for (keep, n) in [(false, 1), (true, 2)] {
            let mut node = r.create("Collect", "c", &serde_json::json!({"inputs": "identity,report", "keep_empty": keep})).unwrap();
            let mut ctx = NodeContext::with_services(Arc::new(ServiceRegistry::new()));
            ctx.set_input("identity", PortValue::new(parts("identity")));
            ctx.set_input("report", PortValue::new(serde_json::json!({"name": "report", "title": "Report", "value": ""})));
            node.execute(&mut ctx).unwrap();
            let list = ctx.drain_outputs().remove("list").and_then(take_or_clone::<serde_json::Value>).unwrap();
            assert_eq!(list.as_array().unwrap().len(), n, "keep_empty={keep}: {list}");
        }
        assert!(r.schema_for("Collect", &serde_json::json!({"inputs": "a,b"})).unwrap().inputs.iter().map(|p| p.name.to_string()).eq(["a", "b"]),
            "one input port per declared name, in order");
    }

    /// **The context of LlmNode**: the `instruction` (a declared parameter), then each part in the order of
    /// the list, as a section — « ## title », then the value: a string as it is, any other value as JSON with
    /// sorted keys, one line per element of a list, or « key: value » lines with format kv. An empty part is
    /// not rendered. The prompt is the same byte for byte.
    #[test]
    fn llm_context_parts_render_as_sections_after_the_instruction() {
        let seen = Arc::new(std::sync::Mutex::new(String::new()));
        let mut node = LlmNodeFactory
            .create("extract", &serde_json::json!({"instruction": "Split this slice into topics.", "response_schema": topics_schema()}))
            .unwrap();
        let mut ctx = ctx_with(notes_prompt(seen.clone()));
        ctx.set_input("context", PortValue::new(serde_json::json!([
            {"name": "identity", "title": "Identity", "value": "I am the archivist."},
            {"name": "fiche", "title": "Fiche", "value": {"name": "Zeta", "about": "x", "alpha": 1}},
            {"name": "turns", "title": "Turns", "value": [{"turn": 2, "b": 1}, {"turn": 1}]},
            {"name": "off", "title": "Disabled", "value": ""},
            {"name": "kv", "title": "As key-value", "value": {"b": "two", "a": 1}, "format": "kv"}
        ])));
        node.execute(&mut ctx).expect("the context is the prompt");
        assert_eq!(
            *seen.lock().unwrap(),
            "Split this slice into topics.\n\n## Identity\n\nI am the archivist.\n\n## Fiche\n\n{\"about\":\"x\",\"alpha\":1,\"name\":\"Zeta\"}\n\n## Turns\n\n{\"b\":1,\"turn\":2}\n{\"turn\":1}\n\n## As key-value\n\na: 1\nb: two"
        );
    }

    /// A part over its `max_tokens` (estimated: characters / 4, rounded up) is cut, and the section says it.
    #[test]
    fn a_context_part_over_its_ceiling_is_cut_and_says_so() {
        let seen = Arc::new(std::sync::Mutex::new(String::new()));
        let mut node = LlmNodeFactory.create("extract", &serde_json::json!({"instruction": "Read."})).unwrap();
        let mut ctx = ctx_with(notes_prompt(seen.clone()));
        ctx.set_input("context", PortValue::new(serde_json::json!([
            {"name": "long", "title": "Long", "value": "a".repeat(100), "max_tokens": 10}
        ])));
        node.execute(&mut ctx).unwrap();
        assert_eq!(*seen.lock().unwrap(), format!("Read.\n\n## Long\n\n{}\n[cut: 10 of 25 estimated tokens kept]", "a".repeat(40)));
    }
}
