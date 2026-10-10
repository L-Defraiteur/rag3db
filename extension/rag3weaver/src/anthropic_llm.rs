//! [`Llm`] distant, derrière l'**API Messages d'Anthropic** (`/v1/messages`,
//! `stream: true`) — Claude en agent, sur le même trait que tout le monde.
//!
//! **Pas de couche de compatibilité OpenAI.** Anthropic en propose une, mais
//! elle fait perdre ce qui distingue ces modèles : la réflexion adaptative
//! (les blocs `thinking` et leur signature à rejouer), les blocs `tool_use`
//! et `tool_result` typés, le `stop_reason: "refusal"` des classifieurs, le
//! détail du cache dans `usage`. On parle donc le protocole natif, en HTTP
//! brut sur le même `ureq` bloquant que [`crate::openai_llm`], et pour la
//! même raison : [`Llm::generate`] est synchrone, le SSE pousse, un
//! `BufRead::read_line` est la lecture la plus directe.
//!
//! ## Ce qui est pris à `openai_llm`, et ce qui ne l'est pas
//!
//! Repris tels quels : [`Auth`] (la clé va dans `x-api-key`, un jeton OAuth
//! dans `Authorization: Bearer`), [`secret_from_env`], [`RetryPolicy`],
//! [`Clock`]. Ce sont des outils de client HTTP, pas du vocabulaire OpenAI ;
//! c'est pourquoi la feature `anthropic-llm` tire `openai-llm`.
//!
//! Réécrits : le corps de la requête (le `system` est un champ de premier
//! niveau, les outils sont des blocs, un résultat d'outil est un message
//! `user`), la lecture du flux (des événements typés, pas des `choices`), la
//! correspondance des fins. Les séquences d'arrêt sont détectées **chez
//! nous** avec [`first_stop`] et [`holdback`], comme pour OpenAI : le
//! fournisseur les accepte, mais les détecter ici garde une seule mécanique,
//! déjà prouvée, pour tous les clients.
//!
//! ## Les trois choses propres à Claude que le trait ne connaît pas
//!
//! - **La réflexion.** Envoyée en `thinking: {type: "adaptive"}` par défaut —
//!   sur Claude Opus 5 c'est le comportement sans réglage, sur Opus 4.8 ou
//!   Sonnet 5 c'est ce qui l'allume. Le texte de réflexion (résumé par le
//!   fournisseur ; la chaîne brute n'est jamais rendue) part dans
//!   [`TokenSink::on_reasoning`]. Les blocs `thinking` d'un tour qui annonce
//!   des outils doivent **revenir à l'identique** au tour suivant : ils
//!   voyagent dans [`ToolCall::provider_extra`] du premier appel, opaques,
//!   exactement comme la `thought_signature` de Gemini. Un tour sans appel
//!   d'outil ne les garde pas : le trait n'a pas de place pour eux, et le
//!   fournisseur les ignore pour les tours passés (sauf Claude Fable 5.1,
//!   qui vérifie l'historique — voir `with_thinking_binding_drop`).
//! - **Le refus.** Un classifieur peut décliner : HTTP 200,
//!   `stop_reason: "refusal"`, une catégorie dans `stop_details`. C'est rendu
//!   comme une **erreur nommée** ([`LlmError::Model`] qui commence par
//!   `refus`), jamais comme un texte vide qui passerait pour une réponse. Le
//!   repli serveur (`fallbacks: "default"`) existe en option
//!   ([`AnthropicLlm::with_default_fallbacks`]) et reste **éteint** : pour une
//!   comparaison de modèles, un remplacement silencieux fausserait la mesure.
//! - **L'échantillonnage.** `temperature` et `top_p` ne partent **jamais** :
//!   Claude Opus 5 et Fable 5 les refusent (400). Le `temperature: 0.0` de
//!   [`GenOptions`] n'a donc pas d'effet ici — c'est le réglage d'effort qui
//!   tient lieu de levier ([`ReasoningEffort`] → `output_config.effort`).
//!
//! ## Comment l'appeler depuis le dataflow
//!
//! Même règle que pour [`crate::openai_llm`] : l'appel bloque son thread
//! pendant toute la génération, il part d'une tâche
//! `Scheduler::task_pipe_to(Priority::Idle, …)`, jamais d'un `Actor::handle`.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

use crate::llm::{
    emit, first_stop, holdback, Finish, Flow, GenOptions, Llm, LlmError, ReasoningEffort,
    ResponseFormat, RetryEvent, RetryPhase, TokenSink, ToolCall, ToolChoice, Turn, Usage,
};
pub use crate::openai_llm::{secret_from_env, Auth, Clock, RetryPolicy, SystemClock};
use crate::tools::ToolDef;

/// Le modèle par défaut. Opus 5 : le plus capable des modèles sans classifieur
/// biologie, et celui que la comparaison avec Gemini prend en premier.
pub const DEFAULT_MODEL: &str = "claude-opus-5";
/// La version de l'API Messages. Une seule valeur publique depuis 2023 ; les
/// nouveautés passent par `anthropic-beta`, pas par cette date.
pub const API_VERSION: &str = "2023-06-01";
/// La variable qui porte la clé. Lue, jamais écrite, jamais affichée.
pub const API_KEY_ENV: &str = "ANTHROPIC_API_KEY";
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
/// Clé sous laquelle les blocs de réflexion voyagent dans
/// [`ToolCall::provider_extra`]. Préfixée pour qu'aucun autre fournisseur ne
/// la lise par accident.
const THINKING_EXTRA: &str = "anthropic_thinking";
/// En-tête bêta du repli serveur en forme `"default"`. La forme en tableau
/// prend un autre en-tête ; les mélanger est un 400.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
/// En-tête bêta qui laisse le fournisseur **ignorer** un bloc de réflexion
/// dont l'historique ne correspond plus, au lieu de refuser la requête.
const THINKING_BINDING_BETA: &str = "thinking-binding-controls-2026-08-01";

/// Ce que le fournisseur rend du raisonnement. Le raisonnement lui-même a
/// lieu et se facture pareil dans les deux cas ; seule la visibilité change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingDisplay {
    /// Un résumé lisible, poussé dans [`TokenSink::on_reasoning`]. Le défaut
    /// ici : c'est ce que le chat affiche déjà pour Gemini.
    Summarized,
    /// Des blocs `thinking` au texte vide — rien à montrer, moins d'octets.
    Omitted,
}

impl ThinkingDisplay {
    fn as_str(self) -> &'static str {
        match self {
            ThinkingDisplay::Summarized => "summarized",
            ThinkingDisplay::Omitted => "omitted",
        }
    }
}

/// Générateur distant sur l'API Messages d'Anthropic.
pub struct AnthropicLlm {
    base_url: String,
    model: String,
    auth: Auth,
    context_len: usize,
    agent: ureq::Agent,
    /// `thinking: {type: "adaptive"}` envoyé ou non.
    thinking: bool,
    display: ThinkingDisplay,
    /// `fallbacks: "default"` + son en-tête bêta.
    default_fallbacks: bool,
    /// `thinking.block_binding.prefix_mismatch_behavior: "drop_block"` + son
    /// en-tête bêta.
    thinking_binding_drop: bool,
    retry: RetryPolicy,
    clock: Arc<dyn Clock>,
    jitter_state: std::sync::atomic::AtomicU64,
}

impl std::fmt::Debug for AnthropicLlm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicLlm")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("auth", &self.auth)
            .field("context_len", &self.context_len)
            .field("thinking", &self.thinking)
            .finish()
    }
}

impl AnthropicLlm {
    /// Un client sur `api.anthropic.com`, authentifié par clé d'API.
    ///
    /// ```no_run
    /// # use rag3weaver::anthropic_llm::{AnthropicLlm, secret_from_env, API_KEY_ENV};
    /// let key = secret_from_env(API_KEY_ENV).unwrap();
    /// let llm = AnthropicLlm::new(key, "claude-opus-5");
    /// ```
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.into(),
            model: model.into(),
            auth: Auth::Header("x-api-key".into(), api_key.into()),
            // Claude Opus 5 et ses contemporains : un million de jetons.
            context_len: 1_000_000,
            // `http_status_as_error(false)` : voir `OpenAiLlm::new` — sans ça
            // un 4xx/5xx est un échec de transport sans corps ni en-têtes.
            agent: ureq::Agent::new_with_config(
                ureq::Agent::config_builder().http_status_as_error(false).build(),
            ),
            thinking: true,
            display: ThinkingDisplay::Summarized,
            default_fallbacks: false,
            thinking_binding_drop: false,
            retry: RetryPolicy::default(),
            clock: Arc::new(SystemClock),
            jitter_state: std::sync::atomic::AtomicU64::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.subsec_nanos() as u64 | 1)
                    .unwrap_or(0x2545_F491_4F6C_DD1D),
            ),
        }
    }

    /// La clé lue dans `ANTHROPIC_API_KEY`. L'erreur nomme la variable, jamais
    /// sa valeur.
    pub fn from_env(model: impl Into<String>) -> Result<Self, LlmError> {
        Ok(Self::new(secret_from_env(API_KEY_ENV)?, model))
    }

    /// Une autre racine : un mandataire, un serveur de test. Sans le
    /// `/v1/messages` final.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Remplace l'authentification — un jeton OAuth passe en
    /// [`Auth::Bearer`] (avec l'en-tête bêta `oauth-2025-04-20`, ajouté ici).
    pub fn with_auth(mut self, auth: Auth) -> Self {
        self.auth = auth;
        self
    }

    pub fn with_context_len(mut self, n: usize) -> Self {
        self.context_len = n;
        self
    }

    /// N'envoie pas `thinking`. Sur Claude Opus 5 le fournisseur réfléchit
    /// quand même (adaptatif par défaut) ; sur Opus 4.8 et Sonnet 5 il ne
    /// réfléchit plus. Utile surtout pour un modèle qui ne connaît pas le
    /// réglage.
    pub fn without_thinking(mut self) -> Self {
        self.thinking = false;
        self
    }

    pub fn with_thinking_display(mut self, display: ThinkingDisplay) -> Self {
        self.display = display;
        self
    }

    /// Active le repli serveur : sur un refus de classifieur, le fournisseur
    /// rejoue la requête sur le modèle qu'il recommande pour cette catégorie,
    /// dans le même appel. **Éteint par défaut** : le modèle qui répond n'est
    /// alors plus forcément celui demandé, ce qu'une comparaison de modèles
    /// ne peut pas tolérer en silence. Le nom du modèle servi se lit dans
    /// `message_start` ; il n'est pas remonté par le trait.
    pub fn with_default_fallbacks(mut self) -> Self {
        self.default_fallbacks = true;
        self
    }

    /// Demande au fournisseur d'**ignorer** un bloc de réflexion rejoué dont
    /// l'historique ne correspond plus, au lieu de refuser la requête. Ne
    /// concerne que Claude Fable 5.1, qui vérifie l'historique ; les autres
    /// modèles acceptent le réglage et ne s'en servent pas. À allumer si un
    /// 400 parle de `prefix_binding_mismatch`.
    pub fn with_thinking_binding_drop(mut self) -> Self {
        self.thinking_binding_drop = true;
        self
    }

    pub fn with_retry(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }

    pub fn without_retry(mut self) -> Self {
        self.retry = RetryPolicy::none();
        self
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    pub fn with_jitter_seed(self, seed: u64) -> Self {
        self.jitter_state.store(seed | 1, std::sync::atomic::Ordering::Relaxed);
        self
    }

    /// Les en-têtes bêta que la configuration demande.
    fn betas(&self) -> Vec<&'static str> {
        let mut b = Vec::new();
        if self.default_fallbacks {
            b.push(FALLBACK_BETA);
        }
        if self.thinking_binding_drop {
            b.push(THINKING_BINDING_BETA);
        }
        if matches!(self.auth, Auth::Bearer(_)) {
            b.push("oauth-2025-04-20");
        }
        b
    }

    /// Le corps de la requête, tel qu'il part. Public pour les tests et pour
    /// qui veut voir ce que le modèle reçoit.
    pub fn request_body(&self, turns: &[Turn], opts: &GenOptions) -> Value {
        let (system, messages) = messages_json(turns);

        let mut body = Map::new();
        body.insert("model".into(), json!(self.model));
        body.insert("max_tokens".into(), json!(opts.max_tokens));
        body.insert("stream".into(), json!(true));
        if !system.is_empty() {
            body.insert("system".into(), json!([{ "type": "text", "text": system }]));
        }
        body.insert("messages".into(), Value::Array(messages));
        // `temperature`/`top_p` : volontairement absents (voir l'en-tête du
        // module). `stop` : détecté chez nous, comme chez OpenAI.
        if self.thinking {
            let mut thinking = Map::new();
            thinking.insert("type".into(), json!("adaptive"));
            thinking.insert("display".into(), json!(self.display.as_str()));
            if self.thinking_binding_drop {
                thinking.insert(
                    "block_binding".into(),
                    json!({ "prefix_mismatch_behavior": "drop_block" }),
                );
            }
            body.insert("thinking".into(), Value::Object(thinking));
        }
        let mut output = Map::new();
        if let Some(effort) = opts.reasoning {
            output.insert("effort".into(), json!(effort_str(effort)));
        }
        if let Some(format) = &opts.response_format {
            if let Some(f) = format_json(format) {
                output.insert("format".into(), f);
            }
        }
        if !output.is_empty() {
            body.insert("output_config".into(), Value::Object(output));
        }
        if !opts.tools.is_empty() {
            let tools: Vec<Value> = opts.tools.iter().map(tool_json).collect();
            body.insert("tools".into(), json!(tools));
            // Comme chez OpenAI, `tool_choice` n'a de sens qu'avec des outils.
            body.insert("tool_choice".into(), tool_choice_json(&opts.tool_choice));
        }
        if self.default_fallbacks {
            body.insert("fallbacks".into(), json!("default"));
        }
        Value::Object(body)
    }

    fn jittered(&self, d: Duration) -> Duration {
        use std::sync::atomic::Ordering::Relaxed;
        if self.retry.jitter <= 0.0 {
            return d;
        }
        let mut x = self.jitter_state.load(Relaxed);
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.jitter_state.store(x, Relaxed);
        let unit = (x >> 11) as f64 / (1u64 << 53) as f64;
        let factor = 1.0 + self.retry.jitter * (unit * 2.0 - 1.0);
        Duration::from_secs_f64((d.as_secs_f64() * factor).max(0.0))
    }

    /// Gigue additive pour un délai imposé par le fournisseur (un plancher).
    fn jitter_up(&self, d: Duration) -> Duration {
        if self.retry.jitter <= 0.0 {
            return d;
        }
        let extra = self.jittered(d).saturating_sub(d);
        d + extra.min(Duration::from_secs_f64(d.as_secs_f64() * self.retry.jitter))
    }

    /// L'attente par tranches, annulable par le puits — la même que celle de
    /// `OpenAiLlm`, pour les mêmes raisons (ne pas immobiliser un thread du
    /// pool, voir une annulation en moins d'une seconde).
    fn cooperative_wait(
        &self,
        total: Duration,
        started: Instant,
        attempt: u32,
        reason: &str,
        from_server: bool,
        sink: &mut dyn TokenSink,
    ) -> Flow {
        const SLICE: Duration = Duration::from_millis(200);
        let deadline = self.clock.now() + total;
        loop {
            let now = self.clock.now();
            if now >= deadline {
                return Flow::Continue;
            }
            let remaining = deadline.saturating_duration_since(now);
            let event = RetryEvent {
                phase: RetryPhase::Waiting,
                attempt,
                max_attempts: self.retry.max_attempts,
                wait: remaining,
                elapsed: now.saturating_duration_since(started),
                reason,
                from_server,
            };
            if sink.on_retry(&event) == Flow::Stop {
                return Flow::Stop;
            }
            self.clock.sleep(SLICE.min(remaining));
        }
    }
}

// ─── Le corps ────────────────────────────────────────────────────────────────

/// `output_config.effort`. `Minimal` n'existe pas chez Anthropic : il devient
/// `low`, l'effort le plus bas ; `xhigh` et `max` ne sont pas dans
/// [`ReasoningEffort`], qui reste l'intersection des fournisseurs.
fn effort_str(e: ReasoningEffort) -> &'static str {
    match e {
        ReasoningEffort::Minimal | ReasoningEffort::Low => "low",
        ReasoningEffort::Medium => "medium",
        ReasoningEffort::High => "high",
    }
}

/// `output_config.format`. Seul le schéma a une forme chez Anthropic ;
/// `Text` n'envoie rien, et `JsonObject` non plus — sans schéma, il n'y a rien
/// à garantir, et le prompt doit demander du JSON comme ailleurs. `strict`
/// n'a pas d'équivalent : la sortie est toujours contrainte au schéma.
fn format_json(f: &ResponseFormat) -> Option<Value> {
    match f {
        ResponseFormat::Text | ResponseFormat::JsonObject => None,
        ResponseFormat::JsonSchema { schema, .. } => {
            Some(json!({ "type": "json_schema", "schema": schema }))
        }
    }
}

/// Un outil, dans la forme des blocs d'Anthropic : `input_schema` et non
/// `parameters`, et pas d'enveloppe `function`.
fn tool_json(t: &ToolDef) -> Value {
    json!({
        "name": t.name,
        "description": t.description,
        "input_schema": t.parameters,
    })
}

/// `tool_choice`, toujours un objet. ⚠ `any` et `tool` sont refusés (400) par
/// Claude Fable 5.1 et Opus 5.5 ; Opus 5 les accepte.
fn tool_choice_json(c: &ToolChoice) -> Value {
    match c {
        ToolChoice::Auto => json!({ "type": "auto" }),
        ToolChoice::Required => json!({ "type": "any" }),
        ToolChoice::None => json!({ "type": "none" }),
        ToolChoice::Function(name) => json!({ "type": "tool", "name": name }),
    }
}

/// Les tours, dans la forme de l'API Messages : le `system` de tête à part,
/// puis des messages `user`/`assistant` qui **alternent** — deux tours de même
/// rôle qui se suivent se fondent en un message, ce qui est ce que le
/// fournisseur exige des résultats d'outils (tous dans **un** message `user`
/// qui suit l'annonce) et ce qu'il tolère ailleurs.
///
/// Un tour `system` qui n'est pas en tête (le bloc d'attente du harnais, par
/// exemple) devient un message `{"role": "system"}` dans la conversation :
/// c'est la voie documentée pour une consigne en cours de route, qui ne casse
/// pas le préfixe en cache. Claude Opus 5 et Fable 5 la connaissent ; Sonnet 5
/// non.
fn messages_json(turns: &[Turn]) -> (String, Vec<Value>) {
    let mut system = String::new();
    let mut messages: Vec<Value> = Vec::new();
    let mut head = true;

    for t in turns {
        let (role, blocks) = if t.role == "system" && head {
            if !system.is_empty() {
                system.push_str("\n\n");
            }
            system.push_str(&t.content);
            continue;
        } else if t.role == "system" {
            ("system", vec![json!({ "type": "text", "text": t.content })])
        } else if let Some(id) = &t.tool_call_id {
            // Un résultat d'outil est un bloc d'un message `user`.
            let mut block = Map::new();
            block.insert("type".into(), json!("tool_result"));
            block.insert("tool_use_id".into(), json!(id));
            if !t.content.is_empty() {
                block.insert("content".into(), json!(t.content));
            }
            ("user", vec![Value::Object(block)])
        } else if !t.tool_calls.is_empty() {
            // Assistant qui annonce des appels : d'abord la réflexion qu'il
            // avait eue (rejouée telle quelle), puis son texte, puis les appels.
            let mut blocks = Vec::new();
            if let Some(thinking) = t.tool_calls[0]
                .provider_extra
                .as_ref()
                .and_then(|e| e.get(THINKING_EXTRA))
                .and_then(Value::as_array)
            {
                blocks.extend(thinking.iter().cloned());
            }
            if !t.content.is_empty() {
                blocks.push(json!({ "type": "text", "text": t.content }));
            }
            for c in &t.tool_calls {
                // `input` est un **objet**, pas une chaîne : la forme du
                // protocole. Un appel tronqué repart en `{}` plutôt que de
                // rendre la conversation irrejouable.
                let input: Value =
                    serde_json::from_str(&crate::llm::arguments_for_wire(&c.arguments))
                        .unwrap_or_else(|_| json!({}));
                blocks.push(json!({
                    "type": "tool_use",
                    "id": c.id,
                    "name": c.name,
                    "input": input,
                }));
            }
            ("assistant", blocks)
        } else {
            // Un bloc de texte vide est refusé par le fournisseur : un tour
            // vide ne produit aucun bloc, et se fond dans son voisin.
            let blocks = if t.content.is_empty() {
                Vec::new()
            } else {
                vec![json!({ "type": "text", "text": t.content })]
            };
            (t.role.as_str(), blocks)
        };
        head = false;

        match messages.last_mut() {
            Some(last) if last["role"] == role && role != "system" => {
                if let Some(arr) = last["content"].as_array_mut() {
                    arr.extend(blocks);
                }
            }
            _ => messages.push(json!({ "role": role, "content": blocks })),
        }
    }
    (system, messages)
}

// ─── Le flux ─────────────────────────────────────────────────────────────────

/// Un bloc de contenu en cours de réception, par `index`.
#[derive(Debug, Clone)]
enum Block {
    Text,
    Thinking { thinking: String, signature: String },
    Redacted { data: String },
    ToolUse { id: String, name: String, input: String },
    /// `fallback`, `server_tool_use`, ou un type inventé après ce code :
    /// ignoré, jamais une erreur.
    Other,
}

/// Les blocs de réflexion, dans la forme à rejouer.
fn thinking_blocks(blocks: &BTreeMap<u64, Block>) -> Vec<Value> {
    blocks
        .values()
        .filter_map(|b| match b {
            Block::Thinking { thinking, signature } => {
                Some(json!({ "type": "thinking", "thinking": thinking, "signature": signature }))
            }
            Block::Redacted { data } => Some(json!({ "type": "redacted_thinking", "data": data })),
            _ => None,
        })
        .collect()
}

/// Les appels d'outils reconstitués, dans l'ordre d'annonce, la réflexion
/// accrochée au premier.
fn collect_calls(blocks: &BTreeMap<u64, Block>) -> Vec<ToolCall> {
    let mut calls: Vec<ToolCall> = blocks
        .values()
        .filter_map(|b| match b {
            Block::ToolUse { id, name, input } => Some(ToolCall::new(id, name, input)),
            _ => None,
        })
        .collect();
    let thinking = thinking_blocks(blocks);
    if let (Some(first), false) = (calls.first_mut(), thinking.is_empty()) {
        first.provider_extra = Some(json!({ THINKING_EXTRA: thinking }));
    }
    calls
}

/// Ce que dit une erreur du fournisseur, bornée.
fn error_message(v: &Value) -> String {
    let kind = v["error"]["type"].as_str().unwrap_or("error");
    let msg = v["error"]["message"].as_str().unwrap_or("(sans message)");
    let mut m = format!("{kind}: {msg}");
    m.truncate(512);
    m
}

/// Le cœur : la boucle SSE. Un événement = une ligne `event:` puis une ligne
/// `data: {json}` ; le `type` dans le JSON redit l'événement, on ne lit donc
/// que `data:`. Séparé de `generate` pour être rejouable sur n'importe quel
/// `BufRead`, sans socket.
fn read_sse(
    reader: &mut impl BufRead,
    opts: &GenOptions,
    context_len: usize,
    sink: &mut dyn TokenSink,
) -> Result<(Finish, Usage), LlmError> {
    let mut line = String::new();
    let mut blocks: BTreeMap<u64, Block> = BTreeMap::new();
    let mut usage = Usage::default();
    let mut pending = String::new();
    let mut emitted = 0usize;
    let mut stop_reason: Option<String> = None;
    let mut stop_sequence: Option<String> = None;
    let mut stop_details: Option<Value> = None;

    /// Le compte du prompt, dans la forme d'Anthropic : le cache est compté
    /// **à part** de `input_tokens`, notre `Usage` le compte **dedans**. On
    /// additionne pour tenir le contrat du type, et on garde la part lue au
    /// cache à côté. Rend `false` si l'objet ne porte pas de compte.
    fn read_input(u: &Value, usage: &mut Usage) -> bool {
        let Some(input) = u["input_tokens"].as_u64() else { return false };
        let read = u["cache_read_input_tokens"].as_u64().unwrap_or(0) as usize;
        let written = u["cache_creation_input_tokens"].as_u64().unwrap_or(0) as usize;
        usage.prompt_tokens = input as usize + read + written;
        usage.cached_prompt_tokens = read;
        true
    }

    let cancelled = |blocks: &BTreeMap<u64, Block>, usage: &mut Usage, emitted: usize| {
        if usage.completion_tokens == 0 {
            usage.completion_tokens = emitted;
        }
        Finish::cancelled().with_tool_calls(collect_calls(blocks))
    };

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => return Err(LlmError::Model(e.to_string())),
        }
        if let Ok(path) = std::env::var("RAG3WEAVER_SSE_DUMP") {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                let _ = f.write_all(line.as_bytes());
                if !line.ends_with('\n') {
                    let _ = f.write_all(b"\n");
                }
            }
        }
        let Some(data) = line.trim_end_matches(['\r', '\n']).strip_prefix("data:") else {
            continue;
        };
        let data = data.trim_start();
        if data.is_empty() {
            continue;
        }
        let ev: Value = serde_json::from_str(data)
            .map_err(|e| LlmError::Model(format!("bad SSE event: {e}")))?;

        match ev["type"].as_str().unwrap_or("") {
            "error" => return Err(LlmError::Model(error_message(&ev))),
            "message_start" => {
                read_input(&ev["message"]["usage"], &mut usage);
            }
            "content_block_start" => {
                let Some(index) = ev["index"].as_u64() else { continue };
                let cb = &ev["content_block"];
                let block = match cb["type"].as_str().unwrap_or("") {
                    "text" => Block::Text,
                    "thinking" => Block::Thinking {
                        thinking: cb["thinking"].as_str().unwrap_or("").to_string(),
                        signature: cb["signature"].as_str().unwrap_or("").to_string(),
                    },
                    "redacted_thinking" => Block::Redacted {
                        data: cb["data"].as_str().unwrap_or("").to_string(),
                    },
                    "tool_use" => Block::ToolUse {
                        id: cb["id"].as_str().unwrap_or("").to_string(),
                        name: cb["name"].as_str().unwrap_or("").to_string(),
                        // Le début d'un appel porte `input: {}` ; les vrais
                        // arguments arrivent en `input_json_delta`.
                        input: String::new(),
                    },
                    _ => Block::Other,
                };
                // Le texte initial d'un bloc `text` ouvert (toujours vide en
                // pratique) suit le même chemin qu'un delta.
                let initial = matches!(block, Block::Text)
                    .then(|| cb["text"].as_str().unwrap_or("").to_string())
                    .unwrap_or_default();
                blocks.insert(index, block);
                if !initial.is_empty() {
                    if let Some(finish) =
                        push_text(&initial, opts, sink, &mut pending, &mut emitted, &blocks, &mut usage)?
                    {
                        return Ok((finish, usage));
                    }
                }
            }
            "content_block_delta" => {
                let Some(index) = ev["index"].as_u64() else { continue };
                let d = &ev["delta"];
                match d["type"].as_str().unwrap_or("") {
                    // Seul un bloc `text` ouvert alimente la réponse : le
                    // texte d'un bloc d'une autre nature (résultat d'un outil
                    // serveur, par exemple) n'est pas la parole du modèle.
                    "text_delta" if matches!(blocks.get(&index), Some(Block::Text)) => {
                        let text = d["text"].as_str().unwrap_or("");
                        if let Some(finish) =
                            push_text(text, opts, sink, &mut pending, &mut emitted, &blocks, &mut usage)?
                        {
                            return Ok((finish, usage));
                        }
                    }
                    "thinking_delta" => {
                        let text = d["thinking"].as_str().unwrap_or("");
                        if let Some(Block::Thinking { thinking, .. }) = blocks.get_mut(&index) {
                            thinking.push_str(text);
                        }
                        if !text.is_empty() && sink.on_reasoning(text) == Flow::Stop {
                            return Ok((cancelled(&blocks, &mut usage, emitted), usage));
                        }
                    }
                    "signature_delta" => {
                        if let Some(Block::Thinking { signature, .. }) = blocks.get_mut(&index) {
                            signature.push_str(d["signature"].as_str().unwrap_or(""));
                        }
                    }
                    "input_json_delta" => {
                        if let Some(Block::ToolUse { input, .. }) = blocks.get_mut(&index) {
                            input.push_str(d["partial_json"].as_str().unwrap_or(""));
                        }
                    }
                    _ => {}
                }
            }
            "message_delta" => {
                if let Some(r) = ev["delta"]["stop_reason"].as_str() {
                    stop_reason = Some(r.to_string());
                }
                if let Some(s) = ev["delta"]["stop_sequence"].as_str() {
                    stop_sequence = Some(s.to_string());
                }
                let details = ev["delta"].get("stop_details").or_else(|| ev.get("stop_details"));
                if let Some(d) = details.filter(|d| !d.is_null()) {
                    stop_details = Some(d.clone());
                }
                // L'usage de fin est **complet** (relevé sur le flux réel du
                // 10 octobre 2026 : `input_tokens` et les deux comptes de cache
                // y sont redits) et cumulatif : la dernière valeur fait foi.
                read_input(&ev["usage"], &mut usage);
                if let Some(n) = ev["usage"]["output_tokens"].as_u64() {
                    usage.completion_tokens = n as usize;
                }
            }
            "message_stop" => break,
            // `ping`, `content_block_stop`, et ce qui viendra.
            _ => {}
        }
    }

    // Un flux qui se ferme avant d'avoir dit comment il finit est une
    // **coupure**, pas une fin : la réponse est tronquée sans que le
    // fournisseur l'ait dit, et un appel d'outil à moitié reçu n'est pas un
    // appel. Ce qui a déjà été poussé l'a été ; l'appelant sait par l'erreur
    // qu'il ne doit pas le prendre pour une réponse.
    if stop_reason.is_none() {
        return Err(LlmError::Model(
            "flux coupé avant `message_delta` : réponse tronquée par la connexion".into(),
        ));
    }

    // Ce qui restait retenu n'amorçait pas de séquence d'arrêt : il sort.
    if !pending.is_empty() && emit(sink, &mut emitted, &pending).is_err() {
        return Ok((cancelled(&blocks, &mut usage, emitted), usage));
    }
    if usage.completion_tokens == 0 {
        usage.completion_tokens = emitted;
    }

    let calls = collect_calls(&blocks);
    let finish = match stop_reason.as_deref() {
        Some("refusal") => {
            // Jamais un texte vide qui passerait pour une réponse.
            let category = stop_details
                .as_ref()
                .and_then(|d| d["category"].as_str())
                .unwrap_or("non précisée");
            let explanation = stop_details
                .as_ref()
                .and_then(|d| d["explanation"].as_str())
                .map(|e| format!(" — {e}"))
                .unwrap_or_default();
            return Err(LlmError::Model(format!(
                "refus du fournisseur (stop_reason: refusal, catégorie : {category}){explanation}"
            )));
        }
        Some("max_tokens") => Finish::max_tokens().with_tool_calls(calls),
        // Le modèle a rempli sa fenêtre : ce n'est pas notre plafond, c'est
        // le sien, et la suite est une compaction — le même signal que le 400
        // d'un prompt trop long.
        Some("model_context_window_exceeded") => {
            return Err(LlmError::ContextOverflow { max: context_len, got: usage.prompt_tokens });
        }
        Some("stop_sequence") => {
            Finish::stop(stop_sequence.unwrap_or_default()).with_tool_calls(calls)
        }
        Some("tool_use") => Finish::tool_call(calls),
        // `end_turn`, `pause_turn`, ou rien : la présence d'appels fait foi,
        // comme chez OpenAI.
        _ if !calls.is_empty() => Finish::tool_call(calls),
        _ => Finish::eos(),
    };
    Ok((finish, usage))
}

/// Pousse du texte dans le puits en tenant les séquences d'arrêt. Rend
/// `Some(finish)` quand la génération s'arrête là (séquence trouvée, ou
/// annulation).
#[allow(clippy::too_many_arguments)]
fn push_text(
    text: &str,
    opts: &GenOptions,
    sink: &mut dyn TokenSink,
    pending: &mut String,
    emitted: &mut usize,
    blocks: &BTreeMap<u64, Block>,
    usage: &mut Usage,
) -> Result<Option<Finish>, LlmError> {
    if text.is_empty() {
        return Ok(None);
    }
    let done = |usage: &mut Usage, emitted: usize, finish: Finish| {
        if usage.completion_tokens == 0 {
            usage.completion_tokens = emitted;
        }
        Ok(Some(finish))
    };
    if opts.stop.is_empty() {
        if emit(sink, emitted, text).is_err() {
            let f = Finish::cancelled().with_tool_calls(collect_calls(blocks));
            return done(usage, *emitted, f);
        }
        return Ok(None);
    }
    pending.push_str(text);
    if let Some((pos, seq)) = first_stop(pending, &opts.stop) {
        let head = pending[..pos].to_string();
        let cancelled = emit(sink, emitted, &head).is_err();
        let calls = collect_calls(blocks);
        let f = if cancelled {
            Finish::cancelled().with_tool_calls(calls)
        } else {
            Finish::stop(seq).with_tool_calls(calls)
        };
        return done(usage, *emitted, f);
    }
    let keep = holdback(pending, &opts.stop);
    let cut = pending.len() - keep;
    if cut > 0 {
        let head = pending[..cut].to_string();
        pending.drain(..cut);
        if emit(sink, emitted, &head).is_err() {
            let f = Finish::cancelled().with_tool_calls(collect_calls(blocks));
            return done(usage, *emitted, f);
        }
    }
    Ok(None)
}

// ─── Les erreurs HTTP et le réessai ──────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Throttled,
    Transient,
    Fatal,
}

/// Même règle que chez OpenAI : un 4xx autre que 408/409/429 n'est jamais
/// réessayé. `529 overloaded_error` est le 503 d'Anthropic.
fn classify(status: u16) -> Verdict {
    match status {
        408 | 409 | 429 => Verdict::Throttled,
        s if (500..600).contains(&s) => Verdict::Transient,
        _ => Verdict::Fatal,
    }
}

/// `retry-after`, en secondes entières — la seule forme que le fournisseur
/// envoie.
fn server_retry_after(headers: &ureq::http::HeaderMap) -> Option<Duration> {
    headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
}

impl Llm for AnthropicLlm {
    fn generate(
        &self,
        turns: &[Turn],
        opts: &GenOptions,
        sink: &mut dyn TokenSink,
    ) -> Result<(Finish, Usage), LlmError> {
        if turns.is_empty() {
            return Err(LlmError::Prompt("no turns".into()));
        }
        if let Some(t) = turns.iter().find(|t| t.role.is_empty()) {
            return Err(LlmError::Prompt(format!("turn with empty role: {:?}", t.content)));
        }
        if let ToolChoice::Function(name) = &opts.tool_choice {
            if !opts.tools.iter().any(|t| &t.name == name) {
                let known: Vec<&str> = opts.tools.iter().map(|t| t.name.as_str()).collect();
                return Err(LlmError::Prompt(format!(
                    "tool_choice impose l'outil `{name}`, absent de `tools` (connus : {})",
                    if known.is_empty() { "aucun".to_string() } else { known.join(", ") }
                )));
            }
        }

        let started = self.clock.now();
        let body = serde_json::to_string(&self.request_body(turns, opts))
            .map_err(|e| LlmError::Prompt(e.to_string()))?;
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let betas = self.betas().join(",");

        let mut retries: u32 = 0;
        loop {
            let mut req = self
                .agent
                .post(&url)
                .header("content-type", "application/json")
                .header("accept", "text/event-stream")
                .header("anthropic-version", API_VERSION);
            if !betas.is_empty() {
                req = req.header("anthropic-beta", &betas);
            }
            req = match &self.auth {
                Auth::None => req,
                Auth::Bearer(t) => req.header("authorization", &format!("Bearer {t}")),
                Auth::Header(k, v) => req.header(k.as_str(), v.as_str()),
            };

            let (err, verdict, server_delay) = match req.send(body.clone()) {
                Ok(mut resp) if resp.status().as_u16() == 200 => {
                    // La frontière : plus aucun réessai une fois le flux
                    // ouvert, pour ne jamais pousser deux fois un début de
                    // réponse (voir `OpenAiLlm`).
                    let mut reader = BufReader::new(resp.body_mut().as_reader());
                    let (finish, mut usage) =
                        read_sse(&mut reader, opts, self.context_len, sink)?;
                    drop(reader);
                    usage.ms = self.clock.now().saturating_duration_since(started).as_millis()
                        as u64;
                    usage.retries = retries;
                    sink.on_finish(&finish);
                    return Ok((finish, usage));
                }
                Ok(mut resp) => {
                    let status = resp.status().as_u16();
                    let headers = resp.headers().clone();
                    let raw = resp
                        .body_mut()
                        .read_to_string()
                        .unwrap_or_else(|e| format!("(corps d'erreur illisible : {e})"));
                    let msg = match serde_json::from_str::<Value>(&raw) {
                        Ok(v) if v.get("error").is_some() => error_message(&v),
                        _ => {
                            let mut m = raw.trim().to_string();
                            m.truncate(512);
                            if m.is_empty() {
                                m = "(corps d'erreur vide)".into();
                            }
                            m
                        }
                    };
                    if msg.contains("prompt is too long") {
                        return Err(LlmError::ContextOverflow { max: self.context_len, got: 0 });
                    }
                    (
                        LlmError::Model(format!("HTTP {status}: {msg}")),
                        classify(status),
                        server_retry_after(&headers),
                    )
                }
                Err(e) => (LlmError::Model(e.to_string()), Verdict::Transient, None),
            };

            if verdict == Verdict::Fatal || retries + 1 >= self.retry.max_attempts {
                return Err(err);
            }
            retries += 1;
            let base = match verdict {
                Verdict::Throttled => self.retry.base_429,
                _ => self.retry.base_5xx,
            };
            let from_server = server_delay.is_some();
            let wait = match server_delay {
                Some(d) => self.jitter_up(d.min(self.retry.max_backoff)),
                None => {
                    let grown = base.as_secs_f64()
                        * self.retry.factor.powi(retries.saturating_sub(1) as i32);
                    self.jittered(Duration::from_secs_f64(
                        grown.min(self.retry.max_backoff.as_secs_f64()),
                    ))
                }
            };
            let elapsed = self.clock.now().saturating_duration_since(started);
            if elapsed + wait > self.retry.max_total {
                return Err(err);
            }
            let reason = err.to_string();
            let announce = RetryEvent {
                phase: RetryPhase::Scheduled,
                attempt: retries,
                max_attempts: self.retry.max_attempts,
                wait,
                elapsed,
                reason: &reason,
                from_server,
            };
            if sink.on_retry(&announce) == Flow::Stop {
                return Err(err);
            }
            if self.cooperative_wait(wait, started, retries, &reason, from_server, sink)
                == Flow::Stop
            {
                return Err(err);
            }
        }
    }

    fn context_len(&self) -> usize {
        self.context_len
    }

    fn name(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{CountingSink, FinishReason, StringSink};

    fn hello() -> Vec<Turn> {
        vec![Turn::system("tu es utile"), Turn::user("bonjour")]
    }

    /// Rejoue des événements SSE enregistrés, sans socket. Chaque entrée est
    /// le JSON d'un événement ; la ligne `event:` est reconstruite de son
    /// `type`, comme le fournisseur l'écrit.
    fn replay(events: &[&str], opts: &GenOptions, sink: &mut dyn TokenSink) -> Result<(Finish, Usage), LlmError> {
        let body: String = events
            .iter()
            .map(|e| {
                let v: Value = serde_json::from_str(e).expect("événement de test valide");
                format!("event: {}\ndata: {e}\n\n", v["type"].as_str().unwrap_or(""))
            })
            .collect();
        let mut r = BufReader::new(body.as_bytes());
        read_sse(&mut r, opts, 1_000_000, sink)
    }

    /// Une transcription enregistrée : une réponse en texte, avec un bloc de
    /// réflexion résumé devant, et un `ping` au milieu.
    const TEXT: &[&str] = &[
        r#"{"type":"message_start","message":{"id":"msg_01","type":"message","role":"assistant","model":"claude-opus-5","content":[],"stop_reason":null,"usage":{"input_tokens":11,"cache_creation_input_tokens":0,"cache_read_input_tokens":4,"output_tokens":1}}}"#,
        r#"{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}"#,
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Une salutation."}}"#,
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig_abc"}}"#,
        r#"{"type":"content_block_stop","index":0}"#,
        r#"{"type":"ping"}"#,
        r#"{"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
        r#"{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Bonjour"}}"#,
        r#"{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":" le"}}"#,
        r#"{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":" monde"}}"#,
        r#"{"type":"content_block_stop","index":1}"#,
        r#"{"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":3}}"#,
        r#"{"type":"message_stop"}"#,
    ];

    /// Un appel d'outil : réflexion, un mot de texte, puis `tool_use` dont les
    /// arguments arrivent en deux morceaux.
    const TOOL: &[&str] = &[
        r#"{"type":"message_start","message":{"id":"msg_02","type":"message","role":"assistant","model":"claude-opus-5","content":[],"stop_reason":null,"usage":{"input_tokens":40,"output_tokens":1}}}"#,
        r#"{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}"#,
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Il faut lire le fichier."}}"#,
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig_tool"}}"#,
        r#"{"type":"content_block_stop","index":0}"#,
        r#"{"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
        r#"{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Je regarde."}}"#,
        r#"{"type":"content_block_stop","index":1}"#,
        r#"{"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"toolu_01","name":"read_file","input":{}}}"#,
        r#"{"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"path\": \"src/"}}"#,
        r#"{"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"lib.rs\"}"}}"#,
        r#"{"type":"content_block_stop","index":2}"#,
        r#"{"type":"message_delta","delta":{"stop_reason":"tool_use","stop_sequence":null},"usage":{"output_tokens":27}}"#,
        r#"{"type":"message_stop"}"#,
    ];

    /// Un refus de classifieur avant tout texte.
    const REFUSAL: &[&str] = &[
        r#"{"type":"message_start","message":{"id":"msg_03","type":"message","role":"assistant","model":"claude-opus-5","content":[],"stop_reason":null,"usage":{"input_tokens":9,"output_tokens":0}}}"#,
        r#"{"type":"message_delta","delta":{"stop_reason":"refusal","stop_sequence":null,"stop_details":{"type":"refusal","category":"cyber","explanation":"déclinée par un classifieur"}},"usage":{"output_tokens":0}}"#,
        r#"{"type":"message_stop"}"#,
    ];

    #[test]
    fn streams_text_pushes_reasoning_apart_and_reads_usage() {
        #[derive(Default)]
        struct Sink { text: String, reasoning: String }
        impl TokenSink for Sink {
            fn on_token(&mut self, t: &str) -> Flow { self.text.push_str(t); Flow::Continue }
            fn on_reasoning(&mut self, t: &str) -> Flow { self.reasoning.push_str(t); Flow::Continue }
        }
        let mut sink = Sink::default();
        let (finish, usage) = replay(TEXT, &GenOptions::default(), &mut sink).unwrap();
        assert_eq!(sink.text, "Bonjour le monde");
        assert_eq!(sink.reasoning, "Une salutation.");
        assert_eq!(finish, Finish::eos());
        assert_eq!(usage.completion_tokens, 3, "compté par le fournisseur");
        assert_eq!(usage.prompt_tokens, 15, "input + cache lu + cache écrit");
        assert_eq!(usage.cached_prompt_tokens, 4);
    }

    #[test]
    fn a_tool_call_is_rebuilt_with_its_thinking_attached() {
        let mut sink = StringSink::default();
        let (finish, usage) = replay(TOOL, &GenOptions::default(), &mut sink).unwrap();
        assert_eq!(sink.text, "Je regarde.");
        assert_eq!(finish.reason, FinishReason::ToolCall);
        assert_eq!(finish.tool_calls.len(), 1);
        let call = &finish.tool_calls[0];
        assert_eq!((call.id.as_str(), call.name.as_str()), ("toolu_01", "read_file"));
        assert_eq!(call.arguments, r#"{"path": "src/lib.rs"}"#);
        let extra = call.provider_extra.as_ref().expect("la réflexion voyage avec l'appel");
        assert_eq!(extra[THINKING_EXTRA][0]["signature"], "sig_tool");
        assert_eq!(extra[THINKING_EXTRA][0]["thinking"], "Il faut lire le fichier.");
        assert_eq!(usage.completion_tokens, 27);
    }

    #[test]
    fn a_refusal_is_a_named_error_never_an_empty_answer() {
        let mut sink = StringSink::default();
        let err = replay(REFUSAL, &GenOptions::default(), &mut sink).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("refus"), "{msg}");
        assert!(msg.contains("cyber"), "{msg}");
        assert!(sink.text.is_empty());
    }

    #[test]
    fn flow_stop_aborts_immediately() {
        let mut sink = CountingSink::stopping_after(2);
        let (finish, usage) = replay(TEXT, &GenOptions::default(), &mut sink).unwrap();
        assert_eq!(finish, Finish::cancelled());
        assert_eq!(sink.tokens, 2);
        assert_eq!(usage.completion_tokens, 2);
    }

    #[test]
    fn reasoning_can_cancel_before_any_public_text() {
        struct Sink;
        impl TokenSink for Sink {
            fn on_token(&mut self, _: &str) -> Flow { panic!("rien ne doit sortir") }
            fn on_reasoning(&mut self, _: &str) -> Flow { Flow::Stop }
        }
        let (finish, _) = replay(TEXT, &GenOptions::default(), &mut Sink).unwrap();
        assert_eq!(finish.reason, FinishReason::Cancelled);
    }

    fn text_events(parts: &[&str], stop_reason: &str) -> Vec<String> {
        let mut v = vec![
            r#"{"type":"message_start","message":{"usage":{"input_tokens":1,"output_tokens":0}}}"#.to_string(),
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#.to_string(),
        ];
        for p in parts {
            v.push(format!(
                r#"{{"type":"content_block_delta","index":0,"delta":{{"type":"text_delta","text":{}}}}}"#,
                Value::String(p.to_string())
            ));
        }
        v.push(format!(r#"{{"type":"message_delta","delta":{{"stop_reason":"{stop_reason}"}},"usage":{{"output_tokens":{}}}}}"#, parts.len()));
        v.push(r#"{"type":"message_stop"}"#.to_string());
        v
    }

    #[test]
    fn stop_sequence_split_across_events_is_caught_and_never_emitted() {
        let owned = text_events(&["Pensée: lire", " Obser", "vation: ceci"], "end_turn");
        let events: Vec<&str> = owned.iter().map(String::as_str).collect();
        let opts = GenOptions::default().with_stop(vec!["Observation:".into()]);
        let mut sink = StringSink::default();
        let (finish, _) = replay(&events, &opts, &mut sink).unwrap();
        assert_eq!(sink.text, "Pensée: lire ");
        assert_eq!(finish.reason, FinishReason::Stop("Observation:".into()));
    }

    #[test]
    fn a_false_start_is_released_at_the_end() {
        let owned = text_events(&["sans ", "Obs"], "end_turn");
        let events: Vec<&str> = owned.iter().map(String::as_str).collect();
        let opts = GenOptions::default().with_stop(vec!["Observation:".into()]);
        let mut sink = StringSink::default();
        let (finish, _) = replay(&events, &opts, &mut sink).unwrap();
        assert_eq!(sink.text, "sans Obs");
        assert_eq!(finish, Finish::eos());
    }

    #[test]
    fn max_tokens_keeps_the_announced_call() {
        let mut owned: Vec<String> = TOOL[..TOOL.len() - 2].iter().map(|s| s.to_string()).collect();
        owned.push(r#"{"type":"message_delta","delta":{"stop_reason":"max_tokens"},"usage":{"output_tokens":27}}"#.into());
        owned.push(r#"{"type":"message_stop"}"#.into());
        let events: Vec<&str> = owned.iter().map(String::as_str).collect();
        let (finish, _) = replay(&events, &GenOptions::default(), &mut StringSink::default()).unwrap();
        assert_eq!(finish.reason, FinishReason::MaxTokens);
        assert_eq!(finish.tool_calls.len(), 1, "l'id doit survivre pour être refermé");
    }

    #[test]
    fn a_stream_cut_before_its_end_is_an_error_not_an_answer() {
        // Tout sauf `message_delta` et `message_stop` : la connexion est tombée.
        let events: Vec<&str> = TOOL[..TOOL.len() - 2].to_vec();
        let mut sink = StringSink::default();
        let err = replay(&events, &GenOptions::default(), &mut sink).unwrap_err();
        assert!(err.to_string().contains("coupé"), "{err}");
        assert_eq!(sink.text, "Je regarde.", "ce qui est sorti est sorti ; l'erreur dit de ne pas s'y fier");
    }

    #[test]
    fn the_final_usage_of_message_delta_wins() {
        // Relevé sur le flux réel : `message_delta` redit les comptes d'entrée.
        let owned = vec![
            r#"{"type":"message_start","message":{"usage":{"input_tokens":1,"cache_read_input_tokens":0,"output_tokens":1}}}"#.to_string(),
            r#"{"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null,"stop_details":null},"usage":{"input_tokens":49,"cache_creation_input_tokens":10,"cache_read_input_tokens":20,"output_tokens":152,"output_tokens_details":{"thinking_tokens":21}}}"#.to_string(),
            r#"{"type":"message_stop"}"#.to_string(),
        ];
        let events: Vec<&str> = owned.iter().map(String::as_str).collect();
        let (_, usage) = replay(&events, &GenOptions::default(), &mut StringSink::default()).unwrap();
        assert_eq!((usage.prompt_tokens, usage.cached_prompt_tokens, usage.completion_tokens), (79, 20, 152));
    }

    #[test]
    fn a_server_stop_sequence_is_reported_as_such() {
        let owned = vec![
            r#"{"type":"message_start","message":{"usage":{"input_tokens":1}}}"#.to_string(),
            r#"{"type":"message_delta","delta":{"stop_reason":"stop_sequence","stop_sequence":"FIN"},"usage":{"output_tokens":2}}"#.to_string(),
        ];
        let events: Vec<&str> = owned.iter().map(String::as_str).collect();
        let (finish, _) = replay(&events, &GenOptions::default(), &mut StringSink::default()).unwrap();
        assert_eq!(finish.reason, FinishReason::Stop("FIN".into()));
    }

    #[test]
    fn an_error_event_is_an_error() {
        let events = [r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#];
        let err = replay(&events, &GenOptions::default(), &mut StringSink::default()).unwrap_err();
        assert_eq!(err.to_string(), "llm model error: overloaded_error: Overloaded");
    }

    #[test]
    fn unknown_block_types_are_ignored() {
        let owned = vec![
            r#"{"type":"message_start","message":{"usage":{"input_tokens":1}}}"#.to_string(),
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"fallback","from":{"model":"a"},"to":{"model":"b"}}}"#.to_string(),
            r#"{"type":"content_block_start","index":1,"content_block":{"type":"text","text":"ok"}}"#.to_string(),
            r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":1}}"#.to_string(),
        ];
        let events: Vec<&str> = owned.iter().map(String::as_str).collect();
        let mut sink = StringSink::default();
        let (finish, _) = replay(&events, &GenOptions::default(), &mut sink).unwrap();
        assert_eq!(sink.text, "ok");
        assert_eq!(finish, Finish::eos());
    }

    #[test]
    fn context_window_exceeded_is_a_context_overflow() {
        let owned = text_events(&["début"], "model_context_window_exceeded");
        let events: Vec<&str> = owned.iter().map(String::as_str).collect();
        let err = replay(&events, &GenOptions::default(), &mut StringSink::default()).unwrap_err();
        assert!(matches!(err, LlmError::ContextOverflow { .. }));
    }

    // ─── Le corps ──────────────────────────────────────────────────────────

    fn llm() -> AnthropicLlm {
        AnthropicLlm::new("clé", DEFAULT_MODEL)
    }

    #[test]
    fn request_body_is_the_messages_shape() {
        let body = llm().request_body(&hello(), &GenOptions::default().with_max_tokens(100));
        assert_eq!(body["model"], DEFAULT_MODEL);
        assert_eq!(body["max_tokens"], 100);
        assert_eq!(body["stream"], true);
        assert_eq!(body["system"][0]["text"], "tu es utile");
        assert_eq!(body["messages"], json!([{"role":"user","content":[{"type":"text","text":"bonjour"}]}]));
        assert_eq!(body["thinking"], json!({"type":"adaptive","display":"summarized"}));
        for absent in ["temperature", "top_p", "stop_sequences", "tools", "tool_choice", "output_config", "fallbacks"] {
            assert!(body.get(absent).is_none(), "{absent} ne doit pas partir par défaut");
        }
    }

    #[test]
    fn effort_and_format_go_to_output_config() {
        let opts = GenOptions::default()
            .with_reasoning(ReasoningEffort::Minimal)
            .with_response_format(ResponseFormat::strict_schema("r", json!({"type":"object"})));
        let body = llm().request_body(&hello(), &opts);
        assert_eq!(body["output_config"]["effort"], "low", "minimal n'existe pas : low");
        assert_eq!(body["output_config"]["format"], json!({"type":"json_schema","schema":{"type":"object"}}));
        let opts = GenOptions::default().with_response_format(ResponseFormat::JsonObject);
        assert!(llm().request_body(&hello(), &opts).get("output_config").is_none());
    }

    #[test]
    fn tools_and_tool_choice_take_the_block_form() {
        let tool = ToolDef { name: "read_file".into(), description: "lit".into(), parameters: json!({"type":"object","properties":{}}) };
        let opts = GenOptions::default().with_tools(vec![tool]).with_tool_choice(ToolChoice::Function("read_file".into()));
        let body = llm().request_body(&hello(), &opts);
        assert_eq!(body["tools"][0]["input_schema"]["type"], "object");
        assert!(body["tools"][0].get("function").is_none());
        assert_eq!(body["tool_choice"], json!({"type":"tool","name":"read_file"}));
        assert_eq!(tool_choice_json(&ToolChoice::Required), json!({"type":"any"}));
        assert_eq!(tool_choice_json(&ToolChoice::None), json!({"type":"none"}));
    }

    #[test]
    fn tool_results_merge_into_one_user_message_after_the_thinking_is_replayed() {
        let mut sink = StringSink::default();
        let (finish, _) = replay(TOOL, &GenOptions::default(), &mut sink).unwrap();
        let mut turns = hello();
        turns.push(Turn::assistant_with_calls(sink.text, finish.tool_calls.clone()));
        turns.push(Turn::tool_result("toolu_01", "read_file", "pub fn depart() {}"));
        turns.push(Turn::tool_result("toolu_02", "grep", ""));
        turns.push(Turn::user("et maintenant ?"));
        let body = llm().request_body(&turns, &GenOptions::default());
        let m = body["messages"].as_array().unwrap();
        assert_eq!(m.len(), 3, "user, assistant, user — les résultats et la question fondus");
        let assistant = &m[1]["content"];
        assert_eq!(assistant[0]["type"], "thinking");
        assert_eq!(assistant[0]["signature"], "sig_tool");
        assert_eq!(assistant[1], json!({"type":"text","text":"Je regarde."}));
        assert_eq!(assistant[2]["type"], "tool_use");
        assert_eq!(assistant[2]["input"], json!({"path":"src/lib.rs"}));
        let last = &m[2]["content"];
        assert_eq!(last[0]["type"], "tool_result");
        assert_eq!(last[0]["content"], "pub fn depart() {}");
        assert!(last[1].get("content").is_none(), "un résultat vide n'envoie pas de contenu vide");
        assert_eq!(last[2], json!({"type":"text","text":"et maintenant ?"}));
    }

    #[test]
    fn a_truncated_call_is_replayed_as_an_empty_object() {
        let call = ToolCall::new("toolu_x", "edit", r#"{"path": "a"#);
        let turns = vec![Turn::user("x"), Turn::assistant_with_calls("", vec![call]), Turn::tool_result("toolu_x", "edit", "interrompu")];
        let body = llm().request_body(&turns, &GenOptions::default());
        assert_eq!(body["messages"][1]["content"][0]["input"], json!({}));
    }

    #[test]
    fn a_mid_conversation_system_turn_stays_in_place() {
        let turns = vec![Turn::system("a"), Turn::system("b"), Turn::user("x"), Turn::system("rappel")];
        let body = llm().request_body(&turns, &GenOptions::default());
        assert_eq!(body["system"][0]["text"], "a\n\nb");
        let m = body["messages"].as_array().unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[1]["role"], "system");
    }

    #[test]
    fn options_add_their_betas_and_fields() {
        let l = llm().with_default_fallbacks().with_thinking_binding_drop();
        assert_eq!(l.betas(), vec![FALLBACK_BETA, THINKING_BINDING_BETA]);
        let body = l.request_body(&hello(), &GenOptions::default());
        assert_eq!(body["fallbacks"], "default");
        assert_eq!(body["thinking"]["block_binding"]["prefix_mismatch_behavior"], "drop_block");
        assert!(llm().betas().is_empty());
        assert_eq!(llm().with_auth(Auth::Bearer("t".into())).betas(), vec!["oauth-2025-04-20"]);
        assert!(llm().without_thinking().request_body(&hello(), &GenOptions::default()).get("thinking").is_none());
    }

    #[test]
    fn secrets_never_appear_in_debug_output() {
        let d = format!("{:?}", AnthropicLlm::new("sk-ant-secret", "m"));
        assert!(!d.contains("secret"), "{d}");
        assert!(d.contains("x-api-key"));
    }

    #[test]
    fn from_env_reports_the_variable_not_the_value() {
        std::env::remove_var(API_KEY_ENV);
        let err = AnthropicLlm::from_env("m").err().unwrap().to_string();
        assert!(err.contains(API_KEY_ENV), "{err}");
    }

    #[test]
    fn malformed_conversation_is_rejected_before_any_socket() {
        let l = llm().with_base_url("http://127.0.0.1:1");
        let mut sink = StringSink::default();
        assert!(matches!(l.generate(&[], &GenOptions::default(), &mut sink), Err(LlmError::Prompt(_))));
        let opts = GenOptions::default().with_tool_choice(ToolChoice::Function("absent".into()));
        assert!(matches!(l.generate(&hello(), &opts, &mut sink), Err(LlmError::Prompt(_))));
    }

    #[test]
    fn classify_follows_the_openai_rule() {
        assert_eq!(classify(429), Verdict::Throttled);
        assert_eq!(classify(529), Verdict::Transient);
        assert_eq!(classify(400), Verdict::Fatal);
        assert_eq!(classify(401), Verdict::Fatal);
    }

    #[test]
    fn arc_dyn_llm_still_works() {
        let l: Arc<dyn Llm> = Arc::new(llm());
        assert_eq!(l.name(), DEFAULT_MODEL);
        assert_eq!(l.context_len(), 1_000_000);
    }
}
