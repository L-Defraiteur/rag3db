//! [`Llm`] distant, derrière un endpoint **compatible OpenAI**
//! (`/chat/completions`, `stream: true`).
//!
//! Un seul client couvre : OpenAI, **Vertex AI** (`.../endpoints/openapi`),
//! **Gemini AI Studio** (`/v1beta/openai`), Mistral, llama.cpp `--server`,
//! vLLM, Ollama, OpenRouter. C'est le résultat qui justifie de ne pas écrire
//! un adaptateur par fournisseur : Gemini parle OpenAI, streaming et appels
//! d'outils compris.
//!
//! ## Pourquoi un client HTTP bloquant
//!
//! [`Llm::generate`] est synchrone (cf. [`crate::llm`]) et le SSE **pousse**
//! ses fragments : un `BufRead::read_line` sur une socket bloquante en est la
//! lecture la plus directe. luciole ne change rien à ce choix — son
//! `AsyncScope` est un exécuteur, pas un réacteur d'I/O.
//!
//! ## Comment l'appeler depuis le dataflow
//!
//! L'appel bloque son thread pendant toute la génération. Il ne doit donc
//! **jamais** partir depuis un `Actor::handle` ; la forme correcte est
//! `Scheduler::task_pipe_to(Priority::Idle, …)`, qui le pose sur un thread du
//! pool et rend la main tout de suite. Et le puits ne doit jamais bloquer non
//! plus : `ActorRef::try_send` + `run_one_step()` si la mailbox est pleine,
//! jamais `send` (cf. le test `deadlock_rule` de ce module).

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::time::Instant;

use serde_json::{json, Map, Value};

use crate::llm::{Finish, Flow, GenOptions, Llm, LlmError, TokenSink, Turn};
use crate::tools::ToolDef;

/// Comment on prouve son identité à l'endpoint. Ne dérive **pas** `Debug` :
/// un secret ne doit pas pouvoir arriver dans un journal par accident.
#[derive(Clone)]
pub enum Auth {
    /// Aucun en-tête — llama.cpp/Ollama en local, serveur de test.
    None,
    /// `Authorization: Bearer <t>`. OpenAI, Mistral, AI Studio (la clé d'API
    /// s'y met telle quelle), et Vertex (jeton OAuth2, cf. [`crate::gcp_auth`]).
    Bearer(String),
    /// Un en-tête arbitraire, p. ex. `("x-goog-api-key", clé)`.
    Header(String, String),
}

impl std::fmt::Debug for Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Jamais la valeur.
        match self {
            Auth::None => write!(f, "Auth::None"),
            Auth::Bearer(_) => write!(f, "Auth::Bearer(<redacted>)"),
            Auth::Header(k, _) => write!(f, "Auth::Header({k:?}, <redacted>)"),
        }
    }
}

/// Lit un secret dans l'environnement, avec un message qui dit quoi faire.
/// Le contenu de la variable n'apparaît jamais dans l'erreur.
pub fn secret_from_env(var: &str) -> Result<String, LlmError> {
    match std::env::var(var) {
        Ok(v) if !v.trim().is_empty() => Ok(v),
        _ => Err(LlmError::Model(format!(
            "variable d'environnement {var} absente ou vide"
        ))),
    }
}

/// Générateur distant. `base_url` est la racine **sans** `/chat/completions`.
pub struct OpenAiLlm {
    base_url: String,
    model: String,
    auth: Auth,
    context_len: usize,
    agent: ureq::Agent,
    /// Les extensions `extra_body.google.*` ne partent que vers Google.
    /// Vertex ignore en silence les paramètres qu'il ne connaît pas, mais un
    /// serveur strict (llama.cpp, Ollama, Mistral) peut répondre 400 — et
    /// notre argument est justement qu'un seul client parle à tous.
    google_extras: bool,
}

impl std::fmt::Debug for OpenAiLlm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiLlm")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("auth", &self.auth)
            .field("context_len", &self.context_len)
            .finish()
    }
}

impl OpenAiLlm {
    /// Endpoint compatible OpenAI quelconque. `base_url` sans le
    /// `/chat/completions` final (`https://api.openai.com/v1`,
    /// `http://127.0.0.1:8080/v1` pour llama.cpp…).
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            auth: Auth::None,
            google_extras: false,
            context_len: 128_000,
            agent: ureq::Agent::new_with_defaults(),
        }
    }

    /// **Gemini via AI Studio** — une simple clé d'API, pas d'OAuth. C'est la
    /// voie « deux minutes » : <https://aistudio.google.com/apikey>, la clé va
    /// dans une variable d'environnement, jamais dans le code.
    ///
    /// ```no_run
    /// # use rag3weaver::openai_llm::{OpenAiLlm, secret_from_env};
    /// let key = secret_from_env("GEMINI_API_KEY").unwrap();
    /// let llm = OpenAiLlm::ai_studio(key, "gemini-2.5-flash");
    /// ```
    pub fn ai_studio(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new("https://generativelanguage.googleapis.com/v1beta/openai", model)
            .with_google_extras()
            .with_auth(Auth::Bearer(api_key.into()))
            .with_context_len(1_000_000)
    }

    /// **Gemini via Vertex AI** — jeton OAuth2 (une heure), voir
    /// [`crate::gcp_auth::TokenSource`]. `location` vaut `"global"` ou une
    /// région (`"europe-west1"`), `model` prend son préfixe (`"google/…"`).
    ///
    /// ```no_run
    /// # use rag3weaver::openai_llm::OpenAiLlm;
    /// # let token = String::new();
    /// let llm = OpenAiLlm::vertex("mon-projet", "global", token, "google/gemini-2.5-flash");
    /// ```
    pub fn vertex(
        project: &str,
        location: &str,
        access_token: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        // `global` n'a pas de préfixe régional ; toute autre région en a un.
        let host = if location == "global" {
            "https://aiplatform.googleapis.com".to_string()
        } else {
            format!("https://{location}-aiplatform.googleapis.com")
        };
        let base = format!(
            "{host}/v1/projects/{project}/locations/{location}/endpoints/openapi"
        );
        Self::new(base, model)
            .with_google_extras()
            .with_auth(Auth::Bearer(access_token.into()))
            .with_context_len(1_000_000)
    }

    /// Repointe l'endpoint sans toucher au reste — sert surtout à faire viser
    /// un serveur de test à un constructeur de fournisseur.
    /// Active les extensions propres à Google (`extra_body.google.*`).
    /// Posé par [`Self::vertex`] et [`Self::ai_studio`] ; à ne pas activer
    /// pour un fournisseur générique, qui peut rejeter le champ.
    pub fn with_google_extras(mut self) -> Self {
        self.google_extras = true;
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_auth(mut self, auth: Auth) -> Self {
        self.auth = auth;
        self
    }

    pub fn with_context_len(mut self, n: usize) -> Self {
        self.context_len = n;
        self
    }

    /// Corps de la requête. Public pour qu'un test puisse l'inspecter sans
    /// ouvrir de socket — il ne contient aucun secret (l'auth est un en-tête).
    pub fn request_body(&self, turns: &[Turn], opts: &GenOptions) -> Value {
        let messages: Vec<Value> = turns
            .iter()
            .map(|t| json!({ "role": t.role, "content": t.content }))
            .collect();

        let mut body = Map::new();
        body.insert("model".into(), json!(self.model));
        body.insert("messages".into(), json!(messages));
        body.insert("stream".into(), json!(true));
        // Sans ça, OpenAI et AI Studio ne renvoient pas `usage` en streaming.
        // Vertex l'envoie de toute façon et ignore le champ en silence — c'est
        // sa politique générale : un paramètre inconnu n'est jamais rejeté,
        // donc une faute de frappe ici échoue sans un mot.
        body.insert("stream_options".into(), json!({ "include_usage": true }));
        body.insert("max_tokens".into(), json!(opts.max_tokens));
        body.insert("temperature".into(), json!(opts.temperature));
        body.insert("top_p".into(), json!(opts.top_p));
        // `opts.stop` n'est **délibérément pas** envoyé. Ne pas « corriger »
        // ceci : le fournisseur écrase `Finish::Stop(seq)` et `Finish::Eos` en
        // un seul `finish_reason: "stop"`, si bien qu'on ne saurait plus dire
        // quelle séquence a coupé — ni même s'il y en a eu une. On les détecte
        // donc nous-mêmes sur le flux (cf. `first_stop`/`holdback` dans
        // `read_sse`). L'économie perdue est négligeable : on ferme la socket
        // dès la correspondance, donc seuls les quelques fragments déjà en vol
        // sont facturés, pas la fin de la génération.
        if !opts.tools.is_empty() {
            // `ToolDef::to_openai_json` tel quel. ⚠ Vertex documente attendre
            // ici une **spec OpenAPI**, pas un JSON Schema : le sous-ensemble
            // qu'on émet (type/properties/required/description/default) passe,
            // mais `additionalProperties: false` — que `tools.rs` pose exprès
            // pour borner une grammaire — n'y est pas garanti honoré.
            let tools: Vec<Value> = opts.tools.iter().map(ToolDef::to_openai_json).collect();
            body.insert("tools".into(), json!(tools));
            body.insert("tool_choice".into(), json!("auto"));
            // Vertex ne fragmente les arguments d'un appel d'outil que si on
            // le demande ; sans ça ils arrivent d'un bloc (ce que le parseur
            // gère aussi, mais on perd le fil au fil de l'eau).
            if self.google_extras {
                body.insert(
                    "extra_body".into(),
                    json!({ "google": { "stream_function_call_arguments": true } }),
                );
            }
        }
        Value::Object(body)
    }
}

/// Un appel d'outil en cours de reconstitution : `id` et `name` n'arrivent
/// qu'une fois, `arguments` est un flux de fragments à concaténer.
#[derive(Default)]
struct ToolAcc {
    id: String,
    name: String,
    arguments: String,
}

impl ToolAcc {
    fn payload(tools: &[ToolAcc]) -> String {
        json!(tools
            .iter()
            .map(|t| json!({ "id": t.id, "name": t.name, "arguments": t.arguments }))
            .collect::<Vec<_>>())
        .to_string()
    }
}

/// Première séquence d'arrêt présente dans `text` : celle qui apparaît le plus
/// tôt, et à position égale la plus longue. Rend son décalage en octets et la
/// séquence. Le texte qui la précède est gardé **verbatim** par l'appelant,
/// espaces compris — même règle que le `MockLlm` de [`crate::llm`].
fn first_stop(text: &str, stops: &[String]) -> Option<(usize, String)> {
    let mut best: Option<(usize, &String)> = None;
    for s in stops.iter().filter(|s| !s.is_empty()) {
        if let Some(pos) = text.find(s.as_str()) {
            let better = match best {
                None => true,
                Some((p, b)) => pos < p || (pos == p && s.len() > b.len()),
            };
            if better {
                best = Some((pos, s));
            }
        }
    }
    best.map(|(p, s)| (p, s.clone()))
}

/// Combien d'octets retenir à la fin de `pending` parce qu'ils pourraient être
/// le début d'une séquence d'arrêt coupée entre deux trames SSE (`"Obser"`
/// puis `"vation:"`).
///
/// C'est le plus long suffixe de `pending` qui soit un préfixe **strict** d'une
/// séquence — un préfixe complet aurait déjà été vu par [`first_stop`]. Sans
/// cette rétention, on pousserait dans le puits du texte qu'il aurait fallu
/// couper : irrattrapable, puisque le puits *pousse*.
fn holdback(pending: &str, stops: &[String]) -> usize {
    let mut best = 0usize;
    for s in stops.iter().filter(|s| !s.is_empty()) {
        let max = (s.len() - 1).min(pending.len());
        for k in (best + 1..=max).rev() {
            let at = pending.len() - k;
            // Ne jamais couper au milieu d'un caractère multi-octet.
            if !pending.is_char_boundary(at) {
                continue;
            }
            if pending.as_bytes()[at..] == s.as_bytes()[..k] {
                best = k;
                break;
            }
        }
    }
    best
}

/// Pousse un fragment dans le puits. `Err(())` = le puits demande l'arrêt.
fn emit(sink: &mut dyn TokenSink, emitted: &mut usize, frag: &str) -> Result<(), ()> {
    if frag.is_empty() {
        return Ok(());
    }
    *emitted += 1;
    if sink.on_token(frag) == Flow::Stop {
        Err(())
    } else {
        Ok(())
    }
}

impl Llm for OpenAiLlm {
    fn generate(
        &self,
        turns: &[Turn],
        opts: &GenOptions,
        sink: &mut dyn TokenSink,
    ) -> Result<(Finish, crate::llm::Usage), LlmError> {
        if turns.is_empty() {
            return Err(LlmError::Prompt("no turns".into()));
        }
        if let Some(t) = turns.iter().find(|t| t.role.is_empty()) {
            return Err(LlmError::Prompt(format!("turn with empty role: {:?}", t.content)));
        }

        let started = Instant::now();
        let body = serde_json::to_string(&self.request_body(turns, opts))
            .map_err(|e| LlmError::Prompt(e.to_string()))?;

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mut req = self
            .agent
            .post(&url)
            .header("content-type", "application/json")
            .header("accept", "text/event-stream");
        req = match &self.auth {
            Auth::None => req,
            Auth::Bearer(t) => req.header("authorization", &format!("Bearer {t}")),
            Auth::Header(k, v) => req.header(k.as_str(), v.as_str()),
        };

        let mut resp = req.send(body).map_err(|e| LlmError::Model(e.to_string()))?;
        let status = resp.status().as_u16();
        if status != 200 {
            // Le corps d'erreur d'un fournisseur ne contient pas le secret
            // envoyé, mais on le tronque : il peut être très long.
            let mut msg = resp
                .body_mut()
                .read_to_string()
                .unwrap_or_else(|e| format!("(corps d'erreur illisible : {e})"));
            msg.truncate(512);
            if msg.trim().is_empty() {
                msg = "(corps d'erreur vide)".into();
            }
            if msg.contains("context length") || msg.contains("maximum context") {
                return Err(LlmError::ContextOverflow { max: self.context_len, got: 0 });
            }
            return Err(LlmError::Model(format!("HTTP {status}: {}", msg.trim())));
        }

        let mut reader = BufReader::new(resp.body_mut().as_reader());
        let (finish, mut usage) = read_sse(&mut reader, opts, sink)?;
        // Fermer la socket : `ureq` ne rend une connexion au pool que si son
        // corps a été lu jusqu'au bout, donc un abandon la coupe bien.
        drop(reader);

        usage.ms = started.elapsed().as_millis() as u64;
        sink.on_finish(&finish);
        Ok((finish, usage))
    }

    fn context_len(&self) -> usize {
        self.context_len
    }

    fn name(&self) -> &str {
        &self.model
    }
}

/// Le cœur : la boucle SSE. Une trame par ligne `data: {json}`,
/// `data: [DONE]` termine. Séparé de `generate` pour être testable sur
/// n'importe quel `BufRead` — donc sans socket.
fn read_sse(
    reader: &mut impl BufRead,
    opts: &GenOptions,
    sink: &mut dyn TokenSink,
) -> Result<(Finish, crate::llm::Usage), LlmError> {
    let mut line = String::new();
    let mut tools: Vec<ToolAcc> = Vec::new();
    let mut by_id: HashMap<String, usize> = HashMap::new();
    let mut by_index: HashMap<usize, usize> = HashMap::new();
    let mut usage = crate::llm::Usage::default();
    // Texte reçu mais pas encore poussé : il pourrait amorcer une séquence
    // d'arrêt. Vide dès qu'on sait qu'il n'en est rien.
    let mut pending = String::new();
    let mut emitted = 0usize;
    let mut reason: Option<String> = None;

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // flux fermé par le serveur
            Ok(_) => {}
            Err(e) => return Err(LlmError::Model(e.to_string())),
        }
        // Tout le reste est du bruit SSE légitime : ligne vide de séparation,
        // `event:`, `:` de keep-alive.
        let Some(data) = line.trim_end_matches(['\r', '\n']).strip_prefix("data:") else {
            continue;
        };
        let data = data.trim_start();
        if data == "[DONE]" {
            break;
        }

        let chunk: Value = serde_json::from_str(data)
            .map_err(|e| LlmError::Model(format!("bad SSE chunk: {e}")))?;
        if let Some(err) = chunk.get("error") {
            let mut m = err.to_string();
            m.truncate(512);
            return Err(LlmError::Model(m));
        }
        if let Some(u) = chunk.get("usage").filter(|u| !u.is_null()) {
            usage.prompt_tokens = u["prompt_tokens"].as_u64().unwrap_or(0) as usize;
            usage.completion_tokens = u["completion_tokens"].as_u64().unwrap_or(0) as usize;
        }

        let Some(choice) = chunk["choices"].get(0) else { continue };
        if let Some(r) = choice["finish_reason"].as_str() {
            reason = Some(r.to_string());
        }
        let delta = &choice["delta"];

        if let Some(calls) = delta["tool_calls"].as_array() {
            for c in calls {
                let idx = c["index"].as_u64().unwrap_or(0) as usize;
                // ⚠ L'`index` de Vertex n'est pas fiable : ses propres
                // exemples font passer un même appel de `index: 1` à
                // `index: 0`. Un parseur qui indexe par `index` recolle les
                // arguments de deux outils différents. On route par `id` dès
                // qu'il est présent, et on ne retombe sur `index` que pour les
                // deltas anonymes (la forme d'OpenAI).
                let slot = match c["id"].as_str() {
                    Some(id) => {
                        let s = *by_id.entry(id.to_string()).or_insert_with(|| {
                            tools.push(ToolAcc { id: id.to_string(), ..Default::default() });
                            tools.len() - 1
                        });
                        by_index.insert(idx, s);
                        s
                    }
                    None => *by_index.entry(idx).or_insert_with(|| {
                        tools.push(ToolAcc::default());
                        tools.len() - 1
                    }),
                };
                let acc = &mut tools[slot];
                if let Some(n) = c["function"]["name"].as_str() {
                    acc.name = n.to_string();
                }
                if let Some(a) = c["function"]["arguments"].as_str() {
                    acc.arguments.push_str(a);
                }
            }
        }

        if let Some(text) = delta["content"].as_str().filter(|t| !t.is_empty()) {
            if opts.stop.is_empty() {
                // Rien à surveiller : chemin direct, aucune rétention.
                if emit(sink, &mut emitted, text).is_err() {
                    if usage.completion_tokens == 0 {
                        usage.completion_tokens = emitted;
                    }
                    // Rendre ici ferme la socket chez l'appelant : c'est le
                    // point d'annulation du contrat, il va jusqu'au réseau.
                    return Ok((Finish::Cancelled, usage));
                }
                continue;
            }

            pending.push_str(text);
            // Une séquence complète est là : on émet ce qui la précède, tel
            // quel, et on coupe. La séquence elle-même n'est jamais poussée.
            if let Some((pos, seq)) = first_stop(&pending, &opts.stop) {
                let head = pending[..pos].to_string();
                let cancelled = emit(sink, &mut emitted, &head).is_err();
                if usage.completion_tokens == 0 {
                    usage.completion_tokens = emitted;
                }
                return Ok((
                    if cancelled { Finish::Cancelled } else { Finish::Stop(seq) },
                    usage,
                ));
            }
            // Sinon on ne pousse que ce qui ne peut plus rien amorcer, et on
            // garde la queue pour la trame suivante.
            let keep = holdback(&pending, &opts.stop);
            let cut = pending.len() - keep;
            if cut > 0 {
                let head = pending[..cut].to_string();
                pending.drain(..cut);
                if emit(sink, &mut emitted, &head).is_err() {
                    if usage.completion_tokens == 0 {
                        usage.completion_tokens = emitted;
                    }
                    return Ok((Finish::Cancelled, usage));
                }
            }
        }
    }

    // Fin de flux : ce qui restait retenu n'était pas une séquence d'arrêt
    // (faux départ, p. ex. `"Obs"` suivi de `"curité"`). Il doit sortir — sans
    // perte ni duplication.
    if !pending.is_empty() && emit(sink, &mut emitted, &pending).is_err() {
        if usage.completion_tokens == 0 {
            usage.completion_tokens = emitted;
        }
        return Ok((Finish::Cancelled, usage));
    }

    let finish = match reason.as_deref() {
        Some("length") => Finish::MaxTokens,
        Some("tool_calls") | Some("function_call") => Finish::ToolCall(ToolAcc::payload(&tools)),
        Some("content_filter") => {
            return Err(LlmError::Model("content_filter".into()));
        }
        // Certains fournisseurs oublient `finish_reason` quand il n'y a que
        // des appels d'outils : la présence d'un accumulateur fait foi.
        _ if !tools.is_empty() => Finish::ToolCall(ToolAcc::payload(&tools)),
        // `finish_reason: "stop"` ne dit pas si c'est l'EOS du modèle ou une
        // séquence de `stop` — le cas `Finish::Stop` est traité plus haut,
        // quand la séquence apparaît vraiment dans le flux.
        _ => Finish::Eos,
    };
    if usage.completion_tokens == 0 {
        usage.completion_tokens = emitted;
    }
    Ok((finish, usage))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{CountingSink, StringSink};

    fn hello() -> Vec<Turn> {
        vec![Turn::system("tu es utile"), Turn::user("bonjour")]
    }

    /// Rejoue des trames SSE sans ouvrir la moindre socket.
    fn replay(frames: &[&str], opts: &GenOptions, sink: &mut dyn TokenSink) -> (Finish, crate::llm::Usage) {
        let body: String = frames.iter().map(|f| format!("data: {f}\n\n")).collect();
        let mut r = BufReader::new(body.as_bytes());
        read_sse(&mut r, opts, sink).unwrap()
    }

    const TEXT: &[&str] = &[
        r#"{"choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}"#,
        r#"{"choices":[{"index":0,"delta":{"content":"Bonjour"},"finish_reason":null}]}"#,
        r#"{"choices":[{"index":0,"delta":{"content":" le"},"finish_reason":null}]}"#,
        r#"{"choices":[{"index":0,"delta":{"content":" monde"},"finish_reason":null}]}"#,
        r#"{"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#,
        r#"{"choices":[],"usage":{"prompt_tokens":11,"completion_tokens":3,"total_tokens":14}}"#,
    ];

    #[test]
    fn streams_text_and_takes_usage_from_the_last_chunk() {
        let mut sink = StringSink::default();
        let (finish, usage) = replay(TEXT, &GenOptions::default(), &mut sink);
        assert_eq!(sink.text, "Bonjour le monde");
        assert_eq!(finish, Finish::Eos);
        assert!(finish.is_complete());
        assert_eq!(usage.prompt_tokens, 11);
        assert_eq!(usage.completion_tokens, 3, "compté par le fournisseur, pas par nous");
    }

    #[test]
    fn flow_stop_aborts_immediately() {
        let mut sink = CountingSink::stopping_after(2);
        let (finish, usage) = replay(TEXT, &GenOptions::default(), &mut sink);
        assert_eq!(finish, Finish::Cancelled);
        assert!(!finish.is_complete());
        assert_eq!(sink.tokens, 2, "pas un fragment de plus");
        assert_eq!(usage.completion_tokens, 2);
    }

    /// Fabrique une trame de contenu.
    fn frag(t: &str) -> String {
        format!(
            r#"{{"choices":[{{"index":0,"delta":{{"content":{}}},"finish_reason":null}}]}}"#,
            Value::String(t.to_string())
        )
    }

    fn replay_frags(parts: &[&str], stops: &[&str]) -> (Finish, String, usize) {
        let owned: Vec<String> = parts.iter().map(|p| frag(p)).collect();
        let mut frames: Vec<&str> = owned.iter().map(String::as_str).collect();
        let tail = r#"{"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#;
        frames.push(tail);
        let opts = GenOptions::default()
            .with_stop(stops.iter().map(|s| s.to_string()).collect());
        let mut sink = StringSink::default();
        let (finish, usage) = replay(&frames, &opts, &mut sink);
        (finish, sink.text, usage.completion_tokens)
    }

    #[test]
    fn stop_sequence_cuts_and_is_never_emitted() {
        let (finish, text, _) = replay_frags(&["réponse ici ", "FIN et la suite"], &["FIN"]);
        assert_eq!(finish, Finish::Stop("FIN".into()));
        assert!(finish.is_complete(), "l'appelant avait demandé ce stop");
        // Le préfixe est verbatim : l'espace avant `FIN` est conservé.
        assert_eq!(text, "réponse ici ");
    }

    #[test]
    fn stop_sequence_split_across_two_frames_is_still_caught() {
        // Le piège du SSE : la séquence arrive à cheval sur deux trames. Rien
        // de `"Observation:"` ne doit sortir, pas même le `"Obser"` de la
        // première — d'où la rétention.
        let (finish, text, _) =
            replay_frags(&["Je pense. ", "Obser", "vation: la suite"], &["Observation:"]);
        assert_eq!(finish, Finish::Stop("Observation:".into()));
        assert_eq!(text, "Je pense. ");
    }

    #[test]
    fn stop_sequence_split_across_three_frames() {
        let (finish, text, _) = replay_frags(&["a", "O", "b", "s", "ervation:x"], &["Observation:"]);
        assert_eq!(finish, Finish::Stop("Observation:".into()));
        assert_eq!(text, "a");
    }

    #[test]
    fn a_false_start_is_released_without_loss_or_duplication() {
        // `"Obs"` amorce `"Observation:"`, mais la suite ne confirme pas : le
        // texte retenu doit ressortir intact, une seule fois.
        let (finish, text, _) =
            replay_frags(&["Obs", "curité totale"], &["Observation:"]);
        assert_eq!(finish, Finish::Eos);
        assert_eq!(text, "Obscurité totale");
    }

    #[test]
    fn a_false_start_at_the_very_end_of_the_stream_is_flushed() {
        // Le flux se termine alors qu'on retenait encore un préfixe : il sort.
        let (finish, text, _) = replay_frags(&["fin de texte : Obs"], &["Observation:"]);
        assert_eq!(finish, Finish::Eos);
        assert_eq!(text, "fin de texte : Obs");
    }

    #[test]
    fn earliest_stop_wins_then_the_longest() {
        // La plus précoce gagne, même déclarée en second.
        let (finish, text, _) =
            replay_frags(&["réponse ici et la suite"], &["suite", "ici"]);
        assert_eq!(finish, Finish::Stop("ici".into()));
        assert_eq!(text, "réponse ");

        // À position égale, la plus longue gagne.
        let (finish, text, _) = replay_frags(&["avant STOPNET après"], &["STOP", "STOPNET"]);
        assert_eq!(finish, Finish::Stop("STOPNET".into()));
        assert_eq!(text, "avant ");
    }

    #[test]
    fn holdback_never_splits_a_multibyte_character() {
        // `é` fait deux octets : une rétention naïve couperait dedans.
        let (finish, text, _) = replay_frags(&["café", "é FIN"], &["éé"]);
        assert_eq!(finish, Finish::Stop("éé".into()));
        assert_eq!(text, "caf");
    }

    #[test]
    fn eos_without_any_stop_sequence_reports_eos() {
        let mut sink = StringSink::default();
        let opts = GenOptions::default().with_stop(vec!["FIN".into()]);
        let (finish, _) = replay(TEXT, &opts, &mut sink);
        assert_eq!(finish, Finish::Eos, "aucune séquence dans le flux");
        assert!(finish.is_complete());
        assert_eq!(sink.text, "Bonjour le monde", "rien n'est retenu à tort");
    }

    #[test]
    fn empty_stop_sequence_is_ignored_and_nothing_is_held_back() {
        let opts = GenOptions::default().with_stop(vec![String::new()]);
        let mut sink = StringSink::default();
        let (finish, _) = replay(TEXT, &opts, &mut sink);
        assert_eq!(finish, Finish::Eos);
        assert_eq!(sink.text, "Bonjour le monde");
    }

    #[test]
    fn flow_stop_wins_over_a_stop_sequence_in_the_same_frame() {
        // Le puits annule en recevant le préfixe : l'annulation prime, parce
        // que la réponse est incomplète du point de vue de l'appelant.
        let owned = [frag("tête "), frag("FIN")];
        let frames: Vec<&str> = owned.iter().map(String::as_str).collect();
        let opts = GenOptions::default().with_stop(vec!["FIN".into()]);
        let mut sink = CountingSink::stopping_after(1);
        let (finish, _) = replay(&frames, &opts, &mut sink);
        assert_eq!(finish, Finish::Cancelled);
        assert!(!finish.is_complete());
    }

    #[test]
    fn usage_after_a_client_side_cut_counts_our_fragments() {
        // On coupe avant le chunk final `usage` : le comptage du fournisseur
        // n'arrivera jamais. On rend ce qu'on sait — le nombre de fragments
        // poussés — et surtout on n'invente pas `prompt_tokens`.
        let owned = [frag("un "), frag("deux "), frag("FIN reste")];
        let frames: Vec<&str> = owned.iter().map(String::as_str).collect();
        let opts = GenOptions::default().with_stop(vec!["FIN".into()]);
        let mut sink = StringSink::default();
        let (finish, usage) = replay(&frames, &opts, &mut sink);
        assert_eq!(finish, Finish::Stop("FIN".into()));
        assert_eq!(sink.text, "un deux ");
        // Deux fragments poussés : la trame `"FIN reste"` a un préfixe vide,
        // qui n'est pas un fragment.
        assert_eq!(usage.completion_tokens, 2, "nos fragments, pas les jetons du modèle");
        assert_eq!(usage.prompt_tokens, 0, "inconnu : non renseigné plutôt qu'inventé");
    }

    #[test]
    fn first_stop_and_holdback_units() {
        let stops = vec!["Observation:".to_string()];
        assert_eq!(first_stop("abObservation:cd", &stops), Some((2, "Observation:".into())));
        assert_eq!(first_stop("ab", &stops), None);
        // Suffixe qui amorce la séquence → retenu.
        assert_eq!(holdback("blabla Obser", &stops), 5);
        assert_eq!(holdback("blabla O", &stops), 1);
        // Rien qui amorce → rien de retenu.
        assert_eq!(holdback("blabla xyz", &stops), 0);
        // Une séquence complète n'est jamais « retenue » : c'est `first_stop`.
        assert_eq!(holdback("Observation:", &stops), 0);
        // Un stop vide n'entraîne aucune rétention.
        assert_eq!(holdback("quoi que ce soit", &[String::new()]), 0);
    }

    #[test]
    fn tool_call_is_accumulated_from_deltas() {
        let frames = &[
            r#"{"choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"call_a1","type":"function","function":{"name":"KBQuerySourceNode","arguments":""}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"kb_"}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"name\":\"docs\","}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"query\":\"luciole\"}"}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#,
            r#"{"choices":[],"usage":{"prompt_tokens":180,"completion_tokens":24}}"#,
        ];
        let mut sink = StringSink::default();
        let (finish, usage) = replay(frames, &GenOptions::default(), &mut sink);
        assert_eq!(sink.text, "", "un appel d'outil ne pousse aucun jeton de texte");
        let Finish::ToolCall(payload) = &finish else { panic!("attendu ToolCall, eu {finish:?}") };
        let calls: Value = serde_json::from_str(payload).unwrap();
        assert_eq!(calls[0]["id"], "call_a1");
        assert_eq!(calls[0]["name"], "KBQuerySourceNode");
        // Les fragments recollés forment un JSON valide.
        let args: Value = serde_json::from_str(calls[0]["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(args["kb_name"], "docs");
        assert_eq!(args["query"], "luciole");
        assert!(finish.is_complete());
        assert_eq!(usage.completion_tokens, 24);
    }

    #[test]
    fn tool_calls_survive_vertex_shuffled_index() {
        // Relevé dans les exemples officiels de Vertex : l'`index` saute d'un
        // appel à l'autre. On route par `id`, donc rien ne se mélange.
        let frames = &[
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"id":"A","function":{"name":"f_a","arguments":"{\"x\":"}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"B","function":{"name":"f_b","arguments":"{\"y\":"}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"A","function":{"arguments":"1}"}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"id":"B","function":{"arguments":"2}"}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#,
        ];
        let mut sink = StringSink::default();
        let (finish, _) = replay(frames, &GenOptions::default(), &mut sink);
        let Finish::ToolCall(payload) = &finish else { panic!("attendu ToolCall") };
        let calls: Value = serde_json::from_str(payload).unwrap();
        assert_eq!(calls[0]["name"], "f_a");
        assert_eq!(calls[0]["arguments"], "{\"x\":1}");
        assert_eq!(calls[1]["name"], "f_b");
        assert_eq!(calls[1]["arguments"], "{\"y\":2}");
    }

    #[test]
    fn length_maps_to_max_tokens() {
        let frames = &[
            r#"{"choices":[{"index":0,"delta":{"content":"tronq"},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{},"finish_reason":"length"}]}"#,
        ];
        let mut sink = StringSink::default();
        let (finish, _) = replay(frames, &GenOptions::default(), &mut sink);
        assert_eq!(finish, Finish::MaxTokens);
        assert!(!finish.is_complete());
        assert_eq!(sink.text, "tronq");
    }

    #[test]
    fn sse_noise_is_ignored() {
        let body = ": keep-alive\n\nevent: message\ndata: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"}}]}\n\ndata: [DONE]\n\n";
        let mut r = BufReader::new(body.as_bytes());
        let mut sink = StringSink::default();
        let (finish, _) = read_sse(&mut r, &GenOptions::default(), &mut sink).unwrap();
        assert_eq!(sink.text, "ok");
        assert_eq!(finish, Finish::Eos);
    }

    #[test]
    fn error_chunk_becomes_a_model_error() {
        let body = "data: {\"error\":{\"message\":\"boom\"}}\n\n";
        let mut r = BufReader::new(body.as_bytes());
        let mut sink = StringSink::default();
        let err = read_sse(&mut r, &GenOptions::default(), &mut sink).unwrap_err();
        assert!(matches!(err, LlmError::Model(m) if m.contains("boom")), "eu autre chose");
    }

    #[test]
    fn request_body_is_the_openai_shape() {
        let llm = OpenAiLlm::new("http://x/v1", "gpt-x");
        let registry = {
            let mut r = crate::dataflow::node_registry::NodeRegistry::new();
            crate::dataflow::register_builtins(&mut r);
            r
        };
        let defs = crate::tools::tool_defs(&registry);
        let opts = GenOptions::default().with_max_tokens(64).with_tools(defs);
        let body = llm.request_body(&hello(), &opts);

        assert_eq!(body["stream"], true);
        assert_eq!(body["stream_options"]["include_usage"], true);
        assert_eq!(body["max_tokens"], 64);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][1]["content"], "bonjour");
        // Les 28 nœuds passent en outils, dans la forme de `tools.rs`.
        assert_eq!(body["tools"].as_array().unwrap().len(), 28);
        assert_eq!(body["tools"][0]["type"], "function");
        assert_eq!(body["tools"][0]["function"]["parameters"]["type"], "object");
        assert_eq!(body["tool_choice"], "auto");
        // Rien de propre à Google ici : ce constructeur est générique.
        // Le drapeau Vertex a son propre test, juste en dessous.
        assert!(body.get("extra_body").is_none());
    }

    /// Le drapeau propre à Google ne doit jamais partir vers un fournisseur
    /// générique : llama.cpp, Ollama ou Mistral peuvent répondre 400 sur un
    /// champ inconnu, alors que Vertex l'ignore en silence.
    #[test]
    fn google_extras_never_leak_to_a_generic_provider() {
        let generic = OpenAiLlm::new("http://localhost:8080/v1", "un-modele-local");
        let defs = {
            let mut r = crate::dataflow::node_registry::NodeRegistry::new();
            crate::dataflow::register_builtins(&mut r);
            crate::tools::tool_defs(&r)
        };
        let opts = GenOptions::default().with_max_tokens(64).with_tools(defs);
        let body = generic.request_body(&hello(), &opts);
        assert!(body["tools"].is_array(), "les outils partent bien");
        assert!(
            body.get("extra_body").is_none(),
            "extra_body ne doit pas exister hors Google, trouvé : {}",
            body["extra_body"]
        );

        // ...mais les deux constructeurs Google l'activent.
        for llm in [
            OpenAiLlm::ai_studio("cle", "google/gemini-2.5-flash"),
            OpenAiLlm::vertex("jeton", "projet", "global", "google/gemini-2.5-flash"),
        ] {
            let body = llm.request_body(&hello(), &opts);
            assert_eq!(
                body["extra_body"]["google"]["stream_function_call_arguments"],
                true
            );
        }
    }

    #[test]
    fn stop_is_never_sent_to_the_provider() {
        // Volontaire : le fournisseur rendrait `finish_reason: "stop"` sans
        // dire quelle séquence a coupé, ni même s'il y en a eu une. On coupe
        // nous-mêmes. Ne pas « corriger » en rajoutant la clé.
        let llm = OpenAiLlm::new("http://x/v1", "m");
        let opts = GenOptions::default().with_stop(vec!["FIN".into()]);
        assert!(llm.request_body(&hello(), &opts).get("stop").is_none());
        let body = llm.request_body(&hello(), &GenOptions::default());
        assert!(body.get("stop").is_none());
        assert!(body.get("tools").is_none(), "pas d'outils = pas de clé `tools`");
    }

    #[test]
    fn constructors_build_the_documented_urls() {
        let v = OpenAiLlm::vertex("proj", "global", "tok", "google/gemini-2.5-flash");
        assert_eq!(
            v.base_url,
            "https://aiplatform.googleapis.com/v1/projects/proj/locations/global/endpoints/openapi"
        );
        let v = OpenAiLlm::vertex("proj", "europe-west1", "tok", "google/gemini-2.5-flash");
        assert_eq!(
            v.base_url,
            "https://europe-west1-aiplatform.googleapis.com/v1/projects/proj/locations/europe-west1/endpoints/openapi"
        );
        let a = OpenAiLlm::ai_studio("k", "gemini-2.5-flash");
        assert_eq!(a.base_url, "https://generativelanguage.googleapis.com/v1beta/openai");
        assert_eq!(a.name(), "gemini-2.5-flash");
        assert_eq!(a.context_len(), 1_000_000);
    }

    #[test]
    fn secrets_never_appear_in_debug_output() {
        let llm = OpenAiLlm::ai_studio("SECRET-DE-LUCIE", "m");
        let shown = format!("{llm:?}");
        assert!(!shown.contains("SECRET-DE-LUCIE"), "secret fuité : {shown}");
        assert!(shown.contains("redacted"));
        let h = Auth::Header("x-goog-api-key".into(), "SECRET-DE-LUCIE".into());
        let shown = format!("{h:?}");
        assert!(!shown.contains("SECRET-DE-LUCIE"), "secret fuité : {shown}");
    }

    #[test]
    fn secret_from_env_reports_the_variable_not_the_value() {
        let err = secret_from_env("RAG3WEAVER_VARIABLE_QUI_N_EXISTE_PAS").unwrap_err();
        let m = err.to_string();
        assert!(m.contains("RAG3WEAVER_VARIABLE_QUI_N_EXISTE_PAS"));
    }

    #[test]
    fn malformed_conversation_is_rejected_before_any_socket() {
        // `http://127.0.0.1:1` est injoignable : si l'erreur n'est pas rendue
        // avant l'ouverture, le test le dirait.
        let llm = OpenAiLlm::new("http://127.0.0.1:1/v1", "m");
        let mut sink = StringSink::default();
        assert_eq!(
            llm.generate(&[], &GenOptions::default(), &mut sink).unwrap_err(),
            LlmError::Prompt("no turns".into())
        );
        let bad = vec![Turn::new("", "sans rôle")];
        assert!(matches!(
            llm.generate(&bad, &GenOptions::default(), &mut sink).unwrap_err(),
            LlmError::Prompt(_)
        ));
    }

    #[test]
    fn arc_dyn_llm_still_works() {
        // Ce que `ctx.service::<Arc<dyn Llm>>("llm")` exigera.
        let llm: std::sync::Arc<dyn Llm> = std::sync::Arc::new(OpenAiLlm::ai_studio("k", "m"));
        assert_eq!(llm.name(), "m");
        assert_eq!(llm.context_len(), 1_000_000);
    }
}
