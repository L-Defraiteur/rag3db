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
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

use crate::llm::{
    emit, first_stop, holdback, Finish, Flow, GenOptions, Llm, LlmError, TokenSink, ToolCall, Turn,
};
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
    /// `extra_body.google.stream_function_call_arguments` — voir `request_body`.
    stream_tool_arguments: bool,
    /// `tool_choice: "validated"` — voir [`OpenAiLlm::with_validated_tool_choice`].
    google_validated_tool_choice: bool,
    retry: RetryPolicy,
    clock: std::sync::Arc<dyn Clock>,
    /// État du générateur de gigue. `with_jitter_seed` le fixe pour les tests.
    jitter_state: std::sync::atomic::AtomicU64,
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
            stream_tool_arguments: false,
            google_validated_tool_choice: false,
            retry: RetryPolicy::default(),
            clock: std::sync::Arc::new(SystemClock),
            jitter_state: std::sync::atomic::AtomicU64::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.subsec_nanos() as u64 | 1)
                    .unwrap_or(0x2545_F491_4F6C_DD1D),
            ),
            context_len: 128_000,
            // `http_status_as_error(false)` est **indispensable** : par défaut,
            // ureq rend `Err` pour tout statut hors 2xx, si bien qu'on ne voit
            // jamais la réponse — ni son corps (le message du fournisseur, la
            // détection du dépassement de contexte, celle de la signature
            // manquante), ni ses en-têtes (`Retry-After`), ni même le code
            // exact autrement que dans une chaîne. Sans cette ligne, tout
            // 4xx/5xx devient un échec de transport indistinct.
            agent: ureq::Agent::new_with_config(
                ureq::Agent::config_builder().http_status_as_error(false).build(),
            ),
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
    /// Règle le réessai. Voir [`RetryPolicy`].
    pub fn with_retry(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }

    /// Désactive tout réessai — équivaut à [`RetryPolicy::none`].
    pub fn without_retry(mut self) -> Self {
        self.retry = RetryPolicy::none();
        self
    }

    /// Remplace l'horloge. Réservé aux tests : sans ça, vérifier le plafond
    /// total prendrait cinq minutes.
    pub fn with_clock(mut self, clock: std::sync::Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Fixe la graine de la gigue, pour des attentes reproductibles en test.
    pub fn with_jitter_seed(self, seed: u64) -> Self {
        self.jitter_state.store(seed | 1, std::sync::atomic::Ordering::Relaxed);
        self
    }

    /// Applique la gigue. `xorshift64` : pas de dépendance, et déterministe
    /// dès que la graine l'est.
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
        let unit = (x >> 11) as f64 / (1u64 << 53) as f64; // [0, 1)
        let factor = 1.0 + self.retry.jitter * (unit * 2.0 - 1.0);
        Duration::from_secs_f64((d.as_secs_f64() * factor).max(0.0))
    }

    /// Gigue **additive** : rend `d` augmenté de 0 à `jitter × d`, jamais
    /// diminué. Pour un délai imposé par le fournisseur, qui est un plancher.
    fn jitter_up(&self, d: Duration) -> Duration {
        if self.retry.jitter <= 0.0 {
            return d;
        }
        let extra = self.jittered(d).saturating_sub(d);
        d + extra.min(Duration::from_secs_f64(d.as_secs_f64() * self.retry.jitter))
    }

    /// Attend `total`, **par tranches**, en restant annulable et sans
    /// immobiliser un thread du pool luciole.
    ///
    /// C'est le cœur du sujet, pas la formule de backoff. Un
    /// `thread::sleep(60s)` dans une tâche luciole immobilise un thread une
    /// minute entière ; quatre threads et quatre agents limités, et le
    /// scheduler gèle — c'est l'interblocage du doc 48 §4 sous un autre
    /// déguisement, l'attente prenant la place du puits bloquant.
    ///
    /// Deux garanties :
    /// - **sur un thread de scheduler**, on exécute du travail prêt
    ///   (`run_one_step`) au lieu de dormir, exactement comme
    ///   `merge_permits::acquire` ;
    /// - **l'attente est interruptible** : le puits est consulté à chaque
    ///   tranche, donc une annulation est vue en moins d'une seconde.
    #[allow(clippy::too_many_arguments)]
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
            let event = crate::llm::RetryEvent {
                phase: crate::llm::RetryPhase::Waiting,
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
            // Par tranches, pour que le puits puisse dire stop entre deux.
            self.clock.sleep(SLICE.min(remaining));
        }
    }

    /// Active `tool_choice: "validated"`, **propre à Google** — d'où sa place
    /// ici plutôt que dans [`crate::llm::ToolChoice`], qui reste l'intersection
    /// des fournisseurs.
    ///
    /// Documenté par Vertex comme correspondant au mode `VALIDATED` de
    /// `FunctionCallingConfig`, avec la mention « This is Google-specific ».
    /// Il contraint le modèle à produire **soit un appel conforme au schéma,
    /// soit du langage naturel** — là où `required` (mode `ANY`) interdit le
    /// texte libre. C'est aussi le mode qui devient le défaut dès qu'on
    /// combine outils et sorties structurées sur Gemini 3.
    ///
    /// Ne s'applique **que** si l'appelant a laissé
    /// [`crate::llm::ToolChoice::Auto`], c'est-à-dire « je n'ai pas d'avis » :
    /// un `Required`, un `None` ou un outil nommé explicites gardent la main.
    pub fn with_validated_tool_choice(mut self) -> Self {
        self.google_validated_tool_choice = true;
        self
    }

    /// Active les extensions propres à Google (`extra_body.google.*`).
    /// Posé par [`Self::vertex`] et [`Self::ai_studio`] ; à ne pas activer
    /// pour un fournisseur générique, qui peut rejeter le champ.
    /// Demander à Vertex de fragmenter les arguments d'appel d'outil au fil
    /// de l'eau. Défectueux sur les valeurs multi-lignes (voir `request_body`).
    pub fn with_streamed_tool_arguments(mut self) -> Self {
        self.stream_tool_arguments = true;
        self
    }

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
        let messages: Vec<Value> =
            turns.iter().map(|t| message_json(t, self.google_extras)).collect();

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
        // Paramètre de premier plan, pas une extension : il part vers TOUS les
        // fournisseurs, y compris OpenAI. Absent par défaut, donc rien ne
        // change pour qui ne le connaît pas. On n'utilise **pas**
        // `extra_body.google.thinking_config` : c'est une extension
        // propriétaire, donc soumise à « un paramètre inconnu est ignoré » —
        // elle est avalée en silence, mesures à l'appui dans
        // `ReasoningEffort`. `reasoning_effort`, lui, est validé : une valeur
        // invalide répond 400 en nommant celles qu'il accepte.
        if let Some(effort) = opts.reasoning {
            body.insert("reasoning_effort".into(), json!(effort.as_str()));
        }
        // Indépendant de `tools` : on peut vouloir une sortie structurée sans
        // aucun outil, et l'inverse. ⚠ Chez Google, **combiner les deux est en
        // préversion et réservé aux modèles Gemini 3** ; dans ce cas le mode
        // `VALIDATED` devient le défaut, que l'on demande `validated` ou non
        // (cf. `with_validated_tool_choice`).
        if let Some(format) = &opts.response_format {
            body.insert("response_format".into(), format.to_openai_json());
        }
        // `opts.stop` n'est **délibérément pas** envoyé. Ne pas « corriger »
        // ceci : le fournisseur écrase `Finish::stop(seq)` et `Finish::eos()` en
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
            // `tool_choice` n'a de sens qu'avec des outils. Ce n'est pas une
            // précaution de style : OpenAI répond 400 — « Invalid value for
            // 'tool_choice': 'tool_choice' is only allowed when 'tools' are
            // specified. » — y compris pour `"none"`, pourtant censé être le
            // défaut sans outils. D'où l'émission sous condition.
            //
            // ⚠ La forme objet qui nomme un outil
            // (`{"type":"function","function":{"name":…}}`) n'est **pas**
            // documentée par Vertex : sa table ne liste que les quatre valeurs
            // chaînes et conclut qu'un paramètre non supporté est ignoré. Elle
            // a pourtant été **vérifiée empiriquement** le 25 août 2026 sur
            // `gemini-3.5-flash` — deux outils déclarés, l'outil nommé est bien
            // celui qui est appelé. À retester si le comportement dérive :
            // déclarer deux outils sans rapport, en nommer un, et vérifier
            // lequel revient dans `tool_calls`.
            // ⚠ Second piège, celui-là purement OpenAI : le **guide**
            // function-calling montre `tool_choice: {"type":"function",
            // "name":"…"}` **à plat**, mais ce guide est écrit pour la
            // Responses API. En `/chat/completions`, la spec OpenAPI n'accepte
            // que la forme **imbriquée** sous `function`. Ne pas « corriger »
            // ce qui suit en recopiant le guide.
            let choice = if self.google_validated_tool_choice
                && opts.tool_choice == crate::llm::ToolChoice::Auto
            {
                json!("validated")
            } else {
                opts.tool_choice.to_openai_json()
            };
            body.insert("tool_choice".into(), choice);
            // Vertex ne fragmente les arguments d'un appel d'outil que si on
            // le demande ; sans ça ils arrivent d'un bloc. **Désactivé par
            // défaut** depuis le 25 août 2026 : fragmentés, les arguments
            // multi-lignes arrivent avec des retours à la ligne non échappés,
            // et le flux se termine sur un `499 CANCELLED` au milieu de la
            // valeur (dump SSE, appel `edit`). Opt-in : `with_streamed_tool_arguments()`.
            if self.google_extras && self.stream_tool_arguments {
                body.insert(
                    "extra_body".into(),
                    json!({ "google": { "stream_function_call_arguments": true } }),
                );
            }
        }
        Value::Object(body)
    }
}

// ─── Réessai ────────────────────────────────────────────────────────────────

/// Horloge, pour que les tests n'attendent pas vraiment.
///
/// Sans cette indirection, un test du plafond total dormirait cinq minutes.
/// `SystemClock` en production, une horloge virtuelle en test.
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
    /// Dort **au plus** `d`. Une implémentation de test avance son temps
    /// virtuel sans dormir.
    fn sleep(&self, d: Duration);
}

/// L'horloge réelle.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }
}

/// Quand et combien de temps réessayer.
///
/// ⚠ **Le réessai n'est pas la réponse au débit.** Il rattrape un incident ;
/// il ne rattrape pas une charge soutenue — quatre agents qui saturent le
/// quota se retrouveront à attendre en chœur, et le backoff ne fera que
/// répartir la douleur. La vraie prévention est de **borner les appels
/// concurrents** en amont, avec un sémaphore d'admission (la généralisation du
/// `merge_permits` de lucivy, cf. doc 48). Si ce filet se met à mordre
/// souvent, ce n'est pas lui qu'il faut régler.
#[derive(Debug, Clone, PartialEq)]
pub struct RetryPolicy {
    /// Tentatives au total, **celle d'origine comprise**. `1` = aucun
    /// réessai. Défaut 4 : trois attentes, ce qui couvre un pic de quota
    /// sans transformer une panne durable en attente interminable.
    pub max_attempts: u32,
    /// Plafond de temps passé dans l'appel, attentes comprises. Défaut
    /// 5 minutes — au-delà, un agent qui attend en silence est indiscernable
    /// d'un agent bloqué.
    pub max_total: Duration,
    /// Première attente après un 429. Défaut 60 s : les quotas de Vertex se
    /// comptent par minute, réessayer plus tôt ne fait que consommer une
    /// tentative.
    pub base_429: Duration,
    /// Première attente après un 5xx ou une erreur de transport. Défaut 1 s :
    /// une panne passagère se dissipe en secondes, pas en minutes.
    pub base_5xx: Duration,
    /// Facteur de croissance entre deux attentes. Défaut 2.
    pub factor: f64,
    /// Plafond d'une attente. Défaut 120 s.
    pub max_backoff: Duration,
    /// Amplitude de la gigue, en fraction. Défaut 0.2 = ±20 %.
    ///
    /// **Obligatoire, pas cosmétique** : sans elle, N appels refusés au même
    /// instant réessaient au même instant et reproduisent la rafale qui a
    /// causé le 429.
    pub jitter: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 4,
            max_total: Duration::from_secs(300),
            base_429: Duration::from_secs(60),
            base_5xx: Duration::from_secs(1),
            factor: 2.0,
            max_backoff: Duration::from_secs(120),
            jitter: 0.2,
        }
    }
}

impl RetryPolicy {
    /// Aucun réessai.
    pub fn none() -> Self {
        Self { max_attempts: 1, ..Self::default() }
    }

    /// Attente pour le `n`-ième réessai (1 = le premier), avant gigue.
    fn backoff(&self, base: Duration, attempt: u32) -> Duration {
        let grown = base.as_secs_f64() * self.factor.powi(attempt.saturating_sub(1) as i32);
        Duration::from_secs_f64(grown.min(self.max_backoff.as_secs_f64()))
    }
}

/// Ce qu'on décide de faire d'un échec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// Réessayable, avec cette base d'attente.
    Retry(RetryBase),
    /// Jamais : réessayer masquerait le problème au lieu de le résoudre.
    Fatal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RetryBase {
    Throttled,
    Transient,
}

/// Classe un échec à partir de son statut **et de son corps**.
///
/// La règle qui compte est la négative : **un 4xx autre que 408/409/429 n'est
/// jamais réessayé**. Un 400 (schéma invalide), un 401 (jeton périmé), un 403
/// (droit manquant) ne guérissent pas en attendant ; les réessayer ne fait que
/// retarder de plusieurs minutes un message d'erreur qui était déjà juste.
///
/// Le corps est nécessaire parce que **tous les 429 ne se valent pas**, et
/// c'est la trouvaille la moins évidente de ce chantier :
///
/// - un quota **par minute** se dissipe, il faut attendre ;
/// - un quota **par jour** (Gemini : `quotaId` contenant `PerDay`, remis à
///   zéro à minuit heure du Pacifique) ou une **limite de dépense** (OpenAI :
///   solde épuisé, plafond de projet atteint) ne se dissipe pas. Attendre
///   quatre fois soixante secondes ne fait que retarder de quatre minutes le
///   même message.
///
/// Les SDK officiels ne font pas cette distinction ; c'est peu cher et ça
/// évite exactement le genre d'attente qui donne l'impression d'un gel.
fn classify(status: u16, body: &str) -> Verdict {
    match status {
        429 if is_permanent_quota(body) => Verdict::Fatal,
        // 408 (timeout) et 409 (conflit) sont réessayés par les SDK d'OpenAI
        // comme par celui de Google.
        408 | 409 | 429 => Verdict::Retry(RetryBase::Throttled),
        s if (500..600).contains(&s) => Verdict::Retry(RetryBase::Transient),
        _ => Verdict::Fatal,
    }
}

/// Vrai si ce 429 ne guérira pas en attendant.
fn is_permanent_quota(body: &str) -> bool {
    // Gemini : le `quotaId` d'un `QuotaFailure` nomme sa fenêtre.
    if body.contains("PerDay") {
        return true;
    }
    // OpenAI : ce ne sont pas des limites de débit mais des limites d'argent.
    let low = body.to_ascii_lowercase();
    ["credit balance", "spend limit", "usage limit", "billing_hard_limit"]
        .iter()
        .any(|m| low.contains(m))
}

/// Lit le délai imposé par le fournisseur, s'il y en a un. Il **gagne** sur
/// tout calcul : lui seul connaît l'état de son quota.
///
/// Trois formes, dans l'ordre de préférence :
/// 1. `retry-after-ms` — en millisecondes ;
/// 2. `retry-after` — en secondes entières, ou en date HTTP ;
/// 3. `details[].retryDelay` dans le corps, forme Google (`"27s"`).
fn server_retry_after(headers: &ureq::http::HeaderMap, body: &str) -> Option<Duration> {
    let get = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());

    if let Some(ms) = get("retry-after-ms").and_then(|v| v.trim().parse::<u64>().ok()) {
        return Some(Duration::from_millis(ms));
    }
    if let Some(raw) = get("retry-after") {
        let raw = raw.trim();
        if let Ok(secs) = raw.parse::<u64>() {
            return Some(Duration::from_secs(secs));
        }
        if let Some(d) = http_date_delay(raw) {
            return Some(d);
        }
    }
    body_retry_delay(body)
}

/// `"27s"`, `"1.5s"` → durée. Forme de `google.rpc.RetryInfo`, que Google met
/// dans le corps plutôt que dans un en-tête.
fn parse_google_duration(v: &str) -> Option<Duration> {
    let secs = v.trim().strip_suffix('s')?.parse::<f64>().ok()?;
    (secs.is_finite() && secs >= 0.0).then(|| Duration::from_secs_f64(secs))
}

/// Cherche `retryDelay` n'importe où dans le corps d'erreur : Google le place
/// sous `error.details[]`, mais la profondeur exacte a déjà changé.
fn body_retry_delay(body: &str) -> Option<Duration> {
    let v: Value = serde_json::from_str(body).ok()?;
    fn find(v: &Value) -> Option<Duration> {
        match v {
            Value::Object(m) => {
                if let Some(d) = m.get("retryDelay").and_then(Value::as_str) {
                    if let Some(d) = parse_google_duration(d) {
                        return Some(d);
                    }
                }
                m.values().find_map(find)
            }
            Value::Array(a) => a.iter().find_map(find),
            _ => None,
        }
    }
    find(&v)
}

/// Convertit une date HTTP (`"Wed, 21 Oct 2015 07:28:00 GMT"`) en délai à
/// partir de maintenant. Rend `None` si la date est passée ou illisible.
fn http_date_delay(raw: &str) -> Option<Duration> {
    // `Jour, JJ Mois AAAA HH:MM:SS GMT` — seule forme que la spec impose de
    // produire ; on ne gère pas les deux formats obsolètes.
    let p: Vec<&str> = raw.split_whitespace().collect();
    if p.len() != 6 {
        return None;
    }
    let day: i64 = p[1].parse().ok()?;
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .iter()
    .position(|m| *m == p[2])? as i64
        + 1;
    let year: i64 = p[3].parse().ok()?;
    let hms: Vec<&str> = p[4].split(':').collect();
    if hms.len() != 3 {
        return None;
    }
    let (h, mi, sec): (i64, i64, i64) =
        (hms[0].parse().ok()?, hms[1].parse().ok()?, hms[2].parse().ok()?);

    // Jours depuis l'époque (algorithme « days_from_civil » de Howard Hinnant).
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let target = days * 86_400 + h * 3_600 + mi * 60 + sec;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    (target > now).then(|| Duration::from_secs((target - now) as u64))
}

/// Profondeur d'imbrication maximale acceptée par le mode strict.
const MAX_SCHEMA_DEPTH: usize = 10;
/// Nombre total de propriétés acceptées.
const MAX_SCHEMA_PROPERTIES: usize = 5_000;
/// Nombre de valeurs dans une énumération.
const MAX_ENUM_VALUES: usize = 1_000;
/// Taille cumulée, en caractères.
const MAX_SCHEMA_CHARS: usize = 120_000;
/// Au-delà de ce nombre de valeurs, une énumération de chaînes est en plus
/// plafonnée en longueur cumulée.
const BIG_ENUM_THRESHOLD: usize = 250;
/// Ce plafond-là.
const BIG_ENUM_CHARS: usize = 15_000;

/// Mots-clés JSON Schema que le mode strict **refuse**.
const FORBIDDEN_KEYWORDS: &[&str] =
    &["allOf", "not", "if", "then", "else", "dependentRequired", "dependentSchemas"];

/// Vérifie qu'un schéma respecte les contraintes du **mode strict** avant
/// l'envoi. `Ok(())` ou la liste de ce qu'il faut corriger.
///
/// Pourquoi vérifier chez nous plutôt que laisser le fournisseur répondre :
/// son 400 sur ce sujet est particulièrement opaque — il nomme un chemin
/// interne sans dire quoi changer, et il n'en signale **qu'un à la fois**, ce
/// qui transforme la mise au point d'un schéma imbriqué en une série
/// d'allers-retours facturés. Ici on les rend tous d'un coup, avec le geste à
/// faire.
///
/// ⚠ **Appelé dès qu'un `json_schema` est fourni, que `strict` soit vrai ou
/// non.** La table des paramètres de Vertex ne mentionne nulle part `strict` :
/// il y est donc probablement ignoré en silence, l'adhérence au schéma venant
/// de `responseJsonSchema` côté Gemini. Conditionner la vérification au
/// drapeau reviendrait à envoyer des schémas invalides à un fournisseur qui
/// les accepte sans les respecter — le pire des deux mondes.
///
/// ⚠ Ces règles sont celles d'**OpenAI**. Gemini décrit son `parameters`
/// comme une spec **OpenAPI 3.0**, pas un JSON Schema — la doc de Vertex le
/// dit explicitement (« This differs from the OpenAI parameters field, which
/// is described as a JSON Schema object »). En pratique son sous-ensemble est
/// plus étroit : par exemple il n'accepte que `enum` et `date-time` comme
/// `format` de chaîne, et refuse `uri`. On ne modélise pas cette seconde
/// grille ici — la première suffit à éviter les erreurs les plus fréquentes,
/// et un schéma qui passe le strict d'OpenAI est déjà bien plus proche de ce
/// que Gemini accepte qu'un JSON Schema quelconque.
///
/// Ce qui **passe** en strict : `$ref` et `$defs`, **récursivité comprise**
/// (`{"$ref": "#"}`), `anyOf`, `pattern`, `format`, les bornes numériques,
/// `minItems`/`maxItems`. Ce qui est **refusé** : voir [`FORBIDDEN_KEYWORDS`].
///
/// Le « Fully recursive schemas are not supported » de Vertex vise bien la
/// récursion **racine**, pas `$ref` en général : un `$ref` non récursif vers
/// `$defs` a été vérifié le 25 août 2026 sur Vertex (HTTP 200, sortie
/// conforme au schéma). D'où le fait qu'on ne le refuse pas. La récursion
/// racine, elle, n'a pas pu être mesurée — un 429 est tombé pendant l'essai.
/// `oneOf` n'apparaît dans aucune liste de support : on le signale, avec
/// `anyOf` comme remplaçant.
pub fn check_strict_schema(schema: &Value) -> Result<(), String> {
    let mut problems = Vec::new();

    if schema.get("anyOf").is_some() {
        problems.push("racine : le mode strict interdit un `anyOf` à la racine".to_string());
    }
    if schema.get("type").and_then(Value::as_str) != Some("object") {
        problems.push(
            "racine : le mode strict exige un schéma de `\"type\": \"object\"`".to_string(),
        );
    }

    let size = schema.to_string().len();
    if size > MAX_SCHEMA_CHARS {
        problems.push(format!(
            "schéma de {size} caractères : le plafond est {MAX_SCHEMA_CHARS}"
        ));
    }

    let mut properties = 0usize;
    check_node(schema, "#", 1, &mut properties, &mut problems);
    if properties > MAX_SCHEMA_PROPERTIES {
        problems.push(format!(
            "{properties} propriétés : le plafond est {MAX_SCHEMA_PROPERTIES}"
        ));
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join(" ; "))
    }
}

/// Vérifie le `name` d'un `json_schema` : `[a-zA-Z0-9_-]`, 64 caractères au
/// plus, non vide. C'est le seul champ requis de l'objet interne.
pub fn check_schema_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("le nom du schéma est vide".into());
    }
    if name.len() > 64 {
        return Err(format!("nom de {} caractères : le plafond est 64", name.len()));
    }
    if let Some(bad) = name.chars().find(|c| !c.is_ascii_alphanumeric() && *c != '_' && *c != '-') {
        return Err(format!(
            "le nom du schéma contient {bad:?} : seuls [a-zA-Z0-9_-] sont acceptés"
        ));
    }
    Ok(())
}

fn check_node(
    node: &Value,
    path: &str,
    depth: usize,
    properties: &mut usize,
    out: &mut Vec<String>,
) {
    let Some(obj) = node.as_object() else { return };

    if depth > MAX_SCHEMA_DEPTH {
        out.push(format!(
            "{path} : imbrication de {depth} niveaux, le plafond est {MAX_SCHEMA_DEPTH}"
        ));
        return;
    }

    for kw in FORBIDDEN_KEYWORDS {
        if obj.contains_key(*kw) {
            out.push(format!("{path} : `{kw}` n'est pas supporté en mode strict"));
        }
    }
    if obj.contains_key("oneOf") {
        out.push(format!(
            "{path} : `oneOf` ne figure dans aucune liste de support — utiliser `anyOf`"
        ));
    }
    if let Some(values) = obj.get("enum").and_then(Value::as_array) {
        if values.len() > MAX_ENUM_VALUES {
            out.push(format!(
                "{path} : {} valeurs d'énumération, le plafond est {MAX_ENUM_VALUES}",
                values.len()
            ));
        }
        // Règle supplémentaire : au-delà de 250 valeurs de chaîne dans une
        // seule énumération, leur longueur cumulée est plafonnée.
        if values.len() > BIG_ENUM_THRESHOLD {
            let chars: usize = values.iter().filter_map(Value::as_str).map(str::len).sum();
            if chars > BIG_ENUM_CHARS {
                out.push(format!(
                    "{path} : énumération de {} valeurs totalisant {chars} caractères, \
                     le plafond est {BIG_ENUM_CHARS} au-delà de {BIG_ENUM_THRESHOLD} valeurs",
                    values.len()
                ));
            }
        }
    }

    if obj.get("type").and_then(Value::as_str) == Some("object") {
        if obj.get("additionalProperties") != Some(&Value::Bool(false)) {
            out.push(format!("{path} : ajouter `\"additionalProperties\": false`"));
        }
        let required: std::collections::HashSet<&str> = obj
            .get("required")
            .and_then(Value::as_array)
            .map(|r| r.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        if let Some(props) = obj.get("properties").and_then(Value::as_object) {
            *properties += props.len();
            let missing: Vec<&str> = props
                .keys()
                .map(String::as_str)
                .filter(|k| !required.contains(k))
                .collect();
            if !missing.is_empty() {
                out.push(format!(
                    "{path} : ajouter à `required` : {} \
                     (en mode strict tout champ déclaré est requis ; pour rendre un champ \
                     facultatif, donner à son `type` l'union avec `\"null\"`)",
                    missing.join(", ")
                ));
            }
            for (k, v) in props {
                check_node(v, &format!("{path}/{k}"), depth + 1, properties, out);
            }
        }
    }
    if let Some(items) = obj.get("items") {
        check_node(items, &format!("{path}/items"), depth + 1, properties, out);
    }
    if let Some(arr) = obj.get("anyOf").and_then(Value::as_array) {
        for (i, v) in arr.iter().enumerate() {
            check_node(v, &format!("{path}/anyOf/{i}"), depth + 1, properties, out);
        }
    }
    // `$defs` ne compte pas comme un niveau d'imbrication : c'est un
    // dictionnaire de définitions, pas une structure. On ne suit jamais les
    // `$ref` — c'est ce qui rend un schéma récursif (`{"$ref": "#"}`), pourtant
    // accepté en strict, analysable sans boucler.
    if let Some(defs) = obj.get("$defs").and_then(Value::as_object) {
        for (k, v) in defs {
            check_node(v, &format!("{path}/$defs/{k}"), depth, properties, out);
        }
    }
}

/// La signature de contournement, dans la forme que Gemini attend.
///
/// **Dernier recours, pas un défaut confortable.** Gemini 3.x refuse en 400
/// tout appel d'outil rejoué sans `thought_signature`. Or nos appels d'origine
/// locale (`MockLlm`, le modèle burn à venir) n'en ont aucune : ils portent nos
/// identifiants blake3 et rien d'autre. Sans ce pont, une conversation née sur
/// le modèle embarqué et reprise sur Gemini partirait en 400 — ce qui viderait
/// de son sens l'idée d'un trait [`crate::llm::Llm`] unique.
///
/// Google documente cette échappatoire pour exactement ce cas (appels
/// fabriqués côté client, traces importées d'un autre modèle) en prévenant
/// qu'elle **dégrade les performances du modèle**. On ne l'émet donc que vers
/// un fournisseur Google, et seulement quand le tour n'a **aucune** signature
/// d'origine à rejouer.
///
/// Cette dernière condition est ce qui protège le cas des appels parallèles :
/// Gemini ne signe que le **premier** d'un lot, les suivants sont légitimement
/// nus, et leur fabriquer une signature serait une régression. Un tour qui
/// porte au moins une signature est donc laissé tel quel.
///
/// Reste un cas qu'on ne sait pas trancher : une réponse Gemini **2.5**, non
/// signée par nature, est indiscernable d'un appel fabriqué chez nous. On y
/// met alors l'échappatoire. C'est délibéré : ne rien mettre risque un **400
/// dur** sur 3.x, la mettre risque une **dégradation douce** sur 2.5 — entre
/// un échec franc et une baisse de qualité, on choisit la baisse de qualité.
fn skip_validator() -> Value {
    json!({ "google": { "thought_signature": crate::llm::SKIP_THOUGHT_SIGNATURE_VALIDATOR } })
}

/// Un appel d'outil en cours de reconstitution : `id` et `name` n'arrivent
/// qu'une fois, `arguments` est un flux de fragments à concaténer.
#[derive(Default)]
struct ToolAcc {
    id: String,
    name: String,
    arguments: String,
    /// `extra_content` de cet appel, capté tel quel. Voir
    /// [`crate::llm::ToolCall::provider_extra`] : on ne l'interprète pas.
    extra: Option<Value>,
}

impl ToolAcc {
    /// Les appels reconstitués, dans l'ordre d'annonce du modèle.
    fn collect(tools: &[ToolAcc]) -> Vec<ToolCall> {
        tools
            .iter()
            .map(|t| {
                let call = ToolCall::new(&t.id, &t.name, &t.arguments);
                match &t.extra {
                    Some(e) => call.with_provider_extra(e.clone()),
                    None => call,
                }
            })
            .collect()
    }
}

/// Un tour, dans la forme attendue par `/chat/completions`. Trois formes
/// réelles, et elles ne se sérialisent pas pareil.
///
/// `google` active le pont de signatures décrit dans [`skip_validator`].
fn message_json(t: &Turn, google: bool) -> Value {
    let mut m = Map::new();
    m.insert("role".into(), json!(t.role));
    if let Some(id) = &t.tool_call_id {
        // Résultat d'outil. `tool_call_id` doit reprendre mot pour mot l'`id`
        // annoncé : c'est là-dessus que le fournisseur apparie, et un appel
        // laissé sans résultat fait échouer toute la requête.
        m.insert("tool_call_id".into(), json!(id));
        if let Some(n) = &t.tool_name {
            m.insert("name".into(), json!(n));
        }
        m.insert("content".into(), json!(t.content));
    } else if !t.tool_calls.is_empty() {
        // Un tour issu de Gemini porte **au moins une** signature : celle du
        // premier appel. Si aucun appel du tour n'en a, le tour ne vient pas
        // de Gemini — il est né chez nous (modèle local) ou chez un autre
        // fournisseur. C'est ce qui distingue « absence légitime » (2ᵉ appel
        // parallèle, que Google ne signe pas et n'attend pas signé) de
        // « absence à combler ».
        let turn_is_signed = t.tool_calls.iter().any(|c| c.provider_extra.is_some());
        // Assistant qui annonce des appels. `content` est `null` — jamais la
        // chaîne vide — quand le modèle n'a rien dit. Ce n'est pas du style :
        // le schéma d'OpenAI rend `content` facultatif dès qu'il y a des
        // `tool_calls`, mais **Gemini répond 400 sur `""`**. `null` est la
        // seule forme que les deux acceptent.
        m.insert(
            "content".into(),
            if t.content.is_empty() { Value::Null } else { json!(t.content) },
        );
        m.insert(
            "tool_calls".into(),
            json!(t
                .tool_calls
                .iter()
                .map(|c| {
                    let mut call = Map::new();
                    call.insert("id".into(), json!(c.id));
                    call.insert("type".into(), json!("function"));
                    // `arguments` est une **chaîne** contenant du JSON, pas un
                    // objet : c'est le format du protocole, et le renvoyer
                    // autrement fait échouer l'appariement.
                    // … et un objet **valide** : Google valide l'historique et
                    // refuse toute la requête sinon. Un appel tronqué par le
                    // flux repart en `{}` plutôt que de bloquer la conversation.
                    call.insert(
                        "function".into(),
                        json!({ "name": c.name, "arguments": crate::llm::arguments_for_wire(&c.arguments) }),
                    );
                    // Rejeu à l'identique de ce que le fournisseur avait
                    // attaché (`thought_signature` chez Gemini 3.x).
                    if let Some(extra) = &c.provider_extra {
                        call.insert("extra_content".into(), extra.clone());
                    } else if google && !turn_is_signed {
                        // Appel fabriqué chez nous, envoyé à Google : voir
                        // `skip_validator`.
                        call.insert("extra_content".into(), skip_validator());
                    }
                    // Sinon : rien du tout. Surtout pas une clé vide ou
                    // `null` — un appel parallèle non signé est parfaitement
                    // légitime, et Google ne l'attend pas.
                    Value::Object(call)
                })
                .collect::<Vec<_>>()),
        );
    } else {
        m.insert("content".into(), json!(t.content));
    }
    Value::Object(m)
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

        // Forcer un outil qui n'est pas dans `tools` est un 400 aussi opaque
        // que le précédent — et c'est typiquement une faute de frappe ou un
        // nœud renommé. On la rend ici, avec la liste des noms connus.
        if let crate::llm::ToolChoice::Function(name) = &opts.tool_choice {
            if !opts.tools.iter().any(|t| &t.name == name) {
                let known: Vec<&str> = opts.tools.iter().map(|t| t.name.as_str()).collect();
                return Err(LlmError::Prompt(format!(
                    "tool_choice impose l'outil `{name}`, absent de `tools` (connus : {})",
                    if known.is_empty() { "aucun".to_string() } else { known.join(", ") }
                )));
            }
        }

        // Vérifié avant d'ouvrir la moindre socket : un schéma non conforme
        // est une erreur de l'appelant, pas une erreur du modèle.
        //
        // Volontairement **sans regarder `strict`** : Vertex ne mentionne pas
        // ce drapeau dans sa table de paramètres, il y est donc probablement
        // ignoré. Ne vérifier qu'en `strict: true` laisserait passer des
        // schémas invalides vers un fournisseur qui les accepte sans les
        // respecter — on croirait la sortie contrainte alors qu'elle ne l'est
        // pas.
        if let Some(crate::llm::ResponseFormat::JsonSchema { name, schema, .. }) =
            &opts.response_format
        {
            if let Err(why) = check_schema_name(name) {
                return Err(LlmError::Prompt(format!("response_format : {why}")));
            }
            if let Err(why) = check_strict_schema(schema) {
                return Err(LlmError::Prompt(format!(
                    "response_format `{name}` non conforme au mode strict : {why}"
                )));
            }
        }

        let started = self.clock.now();
        let body = serde_json::to_string(&self.request_body(turns, opts))
            .map_err(|e| LlmError::Prompt(e.to_string()))?;
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        // Nombre de réessais déjà effectués. `attempt + 1` tentatives faites.
        let mut retries: u32 = 0;
        loop {
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

            // Ce que cette tentative a produit : soit on rend, soit on décide
            // de réessayer.
            let (err, verdict, server_delay) = match req.send(body.clone()) {
                Ok(mut resp) if resp.status().as_u16() == 200 => {
                    // ─────────────────── LA FRONTIÈRE ───────────────────
                    // À partir d'ici, **plus aucun réessai**. Le flux est
                    // ouvert : `read_sse` peut avoir poussé des jetons dans le
                    // puits, et rejouer la requête les dupliquerait — un
                    // consommateur verrait deux fois le début de la réponse,
                    // sans aucun moyen de le savoir. On préfère rendre une
                    // erreur franche.
                    //
                    // Un 200 suivi d'une coupure immédiate, avant la moindre
                    // trame, serait sûr à rejouer ; on ne le fait pas non plus,
                    // parce que distinguer « rien poussé » de « quelque chose
                    // poussé » demanderait à `read_sse` de le rapporter jusque
                    // dans son type d'erreur, pour un gain marginal.
                    let mut reader = BufReader::new(resp.body_mut().as_reader());
                    let (finish, mut usage) = read_sse(&mut reader, opts, sink)?;
                    // Fermer la socket : `ureq` ne rend une connexion au pool
                    // que si son corps a été lu jusqu'au bout.
                    drop(reader);

                    usage.ms = self
                        .clock
                        .now()
                        .saturating_duration_since(started)
                        .as_millis() as u64;
                    usage.retries = retries;
                    sink.on_finish(&finish);
                    return Ok((finish, usage));
                }
                Ok(mut resp) => {
                    let status = resp.status().as_u16();
                    // Les en-têtes avant le corps : lire le corps emprunte
                    // `resp` de façon exclusive.
                    let headers = resp.headers().clone();
                    let mut msg = resp
                        .body_mut()
                        .read_to_string()
                        .unwrap_or_else(|e| format!("(corps d'erreur illisible : {e})"));
                    msg.truncate(512);
                    if msg.trim().is_empty() {
                        msg = "(corps d'erreur vide)".into();
                    }
                    // Deux erreurs qu'on sait typer, et qu'on rend telles
                    // quelles : ni l'une ni l'autre n'est réessayable.
                    if msg.contains("context length") || msg.contains("maximum context") {
                        return Err(LlmError::ContextOverflow { max: self.context_len, got: 0 });
                    }
                    // Gemini refuse un appel d'outil rejoué sans sa signature.
                    // Le libellé varie (« Function call FC1 in the 1. content
                    // block is missing a thought_signature. » / « ... in
                    // functionCall parts ... »), donc on ne compare **jamais**
                    // une chaîne exacte : la présence du mot suffit.
                    if msg.contains("thought_signature") {
                        return Err(LlmError::Model(format!(
                            "HTTP {status}: {} — un appel d'outil a été rejoué sans sa \
                             `thought_signature`. Les `ToolCall` doivent conserver leur \
                             `provider_extra` d'un tour à l'autre.",
                            msg.trim()
                        )));
                    }
                    let delay = server_retry_after(&headers, &msg);
                    (
                        LlmError::Model(format!("HTTP {status}: {}", msg.trim())),
                        classify(status, &msg),
                        delay,
                    )
                }
                Err(e) => (
                    LlmError::Model(e.to_string()),
                    // Socket coupée **avant** la première trame : rien n'a été
                    // poussé, rejouer est sûr.
                    Verdict::Retry(RetryBase::Transient),
                    None,
                ),
            };

            let Verdict::Retry(base_kind) = verdict else {
                return Err(err);
            };
            if retries + 1 >= self.retry.max_attempts {
                return Err(err);
            }

            retries += 1;
            let base = match base_kind {
                RetryBase::Throttled => self.retry.base_429,
                RetryBase::Transient => self.retry.base_5xx,
            };
            // Le délai du fournisseur **gagne** sur tout calcul : lui seul sait
            // où en est son quota. On lui applique quand même le plafond, pour
            // qu'un `Retry-After: 3600` ne fasse pas attendre une heure.
            let from_server = server_delay.is_some();
            let wait = match server_delay {
                // Le délai du fournisseur est un **plancher**, pas une
                // estimation : OpenAI demande explicitement de « treat this
                // value as a minimum and add a small random delay ». La gigue
                // est donc additive ici — jamais soustractive, sinon on
                // réessaierait avant l'heure dite. Et on plafonne quand même,
                // pour qu'un `Retry-After: 3600` ne fasse pas attendre une
                // heure (le SDK d'OpenAI coupe pareillement à 120 s).
                Some(d) => self.jitter_up(d.min(self.retry.max_backoff)),
                None => self.jittered(self.retry.backoff(base, retries)),
            };

            // Plafond total : mieux vaut rendre l'erreur que dépasser.
            let elapsed = self.clock.now().saturating_duration_since(started);
            if elapsed + wait > self.retry.max_total {
                return Err(err);
            }

            let reason = err.to_string();
            let announce = crate::llm::RetryEvent {
                phase: crate::llm::RetryPhase::Scheduled,
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

/// Le cœur : la boucle SSE. Une trame par ligne `data: {json}`,
/// `data: [DONE]` termine. Séparé de `generate` pour être testable sur
/// n'importe quel `BufRead` — donc sans socket.
/// Un objet d'erreur (ou un tableau d'un objet d'erreur) écrit hors `data:`.
fn stray_error(stray: &str) -> Option<String> {
    let t = stray.trim();
    if t.is_empty() {
        return None;
    }
    let v: Value = serde_json::from_str(t).ok()?;
    let err = v.get("error").or_else(|| v.as_array()?.first()?.get("error"))?;
    let mut m = format!("stream error: {err}");
    m.truncate(512);
    Some(m)
}

fn read_sse(
    reader: &mut impl BufRead,
    opts: &GenOptions,
    sink: &mut dyn TokenSink,
) -> Result<(Finish, crate::llm::Usage), LlmError> {
    let mut line = String::new();
    let mut tools: Vec<ToolAcc> = Vec::new();
    let mut by_id: HashMap<String, usize> = HashMap::new();
    let mut by_index: HashMap<usize, usize> = HashMap::new();
    // Dernier appel touché, pour les deltas sans `id` ni `index`.
    let mut last_slot: Option<usize> = None;
    let mut usage = crate::llm::Usage::default();
    // Texte reçu mais pas encore poussé : il pourrait amorcer une séquence
    // d'arrêt. Vide dès qu'on sait qu'il n'en est rien.
    let mut pending = String::new();
    let mut emitted = 0usize;
    let mut reason: Option<String> = None;
    // Lignes hors protocole SSE (voir plus bas).
    let mut stray = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // Flux fermé par le serveur. Une erreur écrite hors `data:`
                // prime sur tout ce qu'on a accumulé : un appel d'outil
                // tronqué par un 499 n'est pas un appel.
                if let Some(msg) = stray_error(&stray) {
                    return Err(LlmError::Model(msg));
                }
                break;
            }
            Ok(_) => {}
            Err(e) => return Err(LlmError::Model(e.to_string())),
        }
        // Tout le reste est du bruit SSE légitime : ligne vide de séparation,
        // `event:`, `:` de keep-alive.
        // `RAG3WEAVER_SSE_DUMP=<fichier>` : chaque ligne brute du flux y est
        // ajoutée — pour voir ce qu'un fournisseur envoie vraiment quand un
        // appel d'outil arrive tronqué ou mal formé.
        if let Ok(path) = std::env::var("RAG3WEAVER_SSE_DUMP") {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                let _ = f.write_all(line.as_bytes());
                if !line.ends_with('\n') {
                    let _ = f.write_all(b"\n");
                }
            }
        }
        // Un objet d'erreur JSON que Vertex écrit **sans** `data:`, sur
        // plusieurs lignes, quand il coupe un flux (`499 CANCELLED` au milieu
        // d'un appel d'outil, 25 août 2026) : on l'accumule, et à la
        // fermeture du flux, s'il se lit, c'est l'erreur.
        let Some(data) = line.trim_end_matches(['\r', '\n']).strip_prefix("data:") else {
            let t = line.trim();
            if !t.is_empty() && !t.starts_with(':') && !t.starts_with("event:") && !t.starts_with("id:") && !t.starts_with("retry:") {
                stray.push_str(t);
                stray.push('\n');
            }
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
            // La part servie depuis le cache. Le fournisseur l'envoie, on la
            // jetait — et sans elle un coût est faux d'un ordre de grandeur.
            //
            // Deux orthographes selon la couche : `prompt_tokens_details`
            // pour les API compatibles OpenAI, `cached_content_token_count`
            // quand Gemini transparaît sous la compatibilité. On lit les deux
            // plutôt que de parier sur laquelle arrivera.
            usage.cached_prompt_tokens = u["prompt_tokens_details"]["cached_tokens"]
                .as_u64()
                .or_else(|| u["cached_content_token_count"].as_u64())
                .unwrap_or(0) as usize;
        }

        let Some(choice) = chunk["choices"].get(0) else { continue };
        if let Some(r) = choice["finish_reason"].as_str() {
            reason = Some(r.to_string());
        }
        let delta = &choice["delta"];

        if let Some(calls) = delta["tool_calls"].as_array() {
            for c in calls {
                // ⚠ Deux défauts de l'`index` côté Google, et ils se
                // cumulent. Il n'est pas fiable — les exemples de Vertex font
                // passer un même appel de `index: 1` à `index: 0` — et en
                // streaming il est carrément **absent** (Google envoie
                // l'appel entier dans une seule trame et omet le champ que la
                // spec OpenAI impose, ce qui casse les validateurs stricts).
                // On route donc par `id` dès qu'il est là ; `index` ne sert
                // qu'aux deltas anonymes, la forme d'OpenAI. Et on ne
                // l'enregistre que s'il est réellement présent : le supposer
                // à 0 ferait converger tous les appels d'une même trame vers
                // la même case.
                let idx = c["index"].as_u64().map(|v| v as usize);
                let fresh = |tools: &mut Vec<ToolAcc>, id: Option<&str>| {
                    tools.push(ToolAcc {
                        id: id.unwrap_or_default().to_string(),
                        ..Default::default()
                    });
                    tools.len() - 1
                };
                let slot = match (c["id"].as_str(), idx) {
                    (Some(id), _) => {
                        let s = match by_id.get(id) {
                            Some(s) => *s,
                            None => {
                                let s = fresh(&mut tools, Some(id));
                                by_id.insert(id.to_string(), s);
                                s
                            }
                        };
                        if let Some(i) = idx {
                            by_index.insert(i, s);
                        }
                        s
                    }
                    (None, Some(i)) => match by_index.get(&i) {
                        Some(s) => *s,
                        None => {
                            let s = fresh(&mut tools, None);
                            by_index.insert(i, s);
                            s
                        }
                    },
                    // Ni `id` ni `index` : le seul rattachement raisonnable
                    // est le dernier appel touché.
                    (None, None) => match last_slot {
                        Some(s) => s,
                        None => fresh(&mut tools, None),
                    },
                };
                last_slot = Some(slot);
                let acc = &mut tools[slot];
                if let Some(n) = c["function"]["name"].as_str() {
                    acc.name = n.to_string();
                }
                if let Some(a) = c["function"]["arguments"].as_str() {
                    acc.arguments.push_str(a);
                }
                // `extra_content` porte le `thought_signature` de Gemini 3.x,
                // que le fournisseur exige de revoir à l'identique au tour
                // suivant sous peine de 400. On le capte **opaque** : ni lu,
                // ni validé, ni interprété — juste transporté. Il peut arriver
                // sur n'importe quelle trame de l'appel, d'où la mise à jour
                // à chaque fois qu'il est présent.
                if let Some(e) = c.get("extra_content").filter(|v| !v.is_null()) {
                    acc.extra = Some(e.clone());
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
                    return Ok((
                        Finish::cancelled().with_tool_calls(ToolAcc::collect(&tools)),
                        usage,
                    ));
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
                // Même sur une coupure : un appel déjà annoncé repart avec.
                let calls = ToolAcc::collect(&tools);
                return Ok((
                    if cancelled {
                        Finish::cancelled().with_tool_calls(calls)
                    } else {
                        Finish::stop(seq).with_tool_calls(calls)
                    },
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
                    return Ok((
                        Finish::cancelled().with_tool_calls(ToolAcc::collect(&tools)),
                        usage,
                    ));
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
        return Ok((Finish::cancelled().with_tool_calls(ToolAcc::collect(&tools)), usage));
    }

    let finish = match reason.as_deref() {
        // Un appel tronqué par `max_tokens` a des `arguments` incomplets —
        // mais son `id` est bon, et il doit être refermé comme les autres.
        Some("length") => Finish::max_tokens().with_tool_calls(ToolAcc::collect(&tools)),
        Some("tool_calls") | Some("function_call") => {
            Finish::tool_call(ToolAcc::collect(&tools))
        }
        Some("content_filter") => {
            return Err(LlmError::Model("content_filter".into()));
        }
        // Certains fournisseurs oublient `finish_reason` quand il n'y a que
        // des appels d'outils : la présence d'un accumulateur fait foi.
        _ if !tools.is_empty() => Finish::tool_call(ToolAcc::collect(&tools)),
        // `finish_reason: "stop"` ne dit pas si c'est l'EOS du modèle ou une
        // séquence de `stop` — le cas `Finish::Stop` est traité plus haut,
        // quand la séquence apparaît vraiment dans le flux.
        _ => Finish::eos(),
    };
    if usage.completion_tokens == 0 {
        usage.completion_tokens = emitted;
    }
    Ok((finish, usage))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{
        CountingSink, FinishReason, ReasoningEffort, ResponseFormat, StringSink, ToolChoice,
    };

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
        assert_eq!(finish, Finish::eos());
        assert!(finish.is_complete());
        assert_eq!(usage.prompt_tokens, 11);
        assert_eq!(usage.completion_tokens, 3, "compté par le fournisseur, pas par nous");
    }

    #[test]
    fn flow_stop_aborts_immediately() {
        let mut sink = CountingSink::stopping_after(2);
        let (finish, usage) = replay(TEXT, &GenOptions::default(), &mut sink);
        assert_eq!(finish, Finish::cancelled());
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
        assert_eq!(finish, Finish::stop("FIN"));
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
        assert_eq!(finish, Finish::stop("Observation:"));
        assert_eq!(text, "Je pense. ");
    }

    #[test]
    fn stop_sequence_split_across_three_frames() {
        let (finish, text, _) = replay_frags(&["a", "O", "b", "s", "ervation:x"], &["Observation:"]);
        assert_eq!(finish, Finish::stop("Observation:"));
        assert_eq!(text, "a");
    }

    #[test]
    fn a_false_start_is_released_without_loss_or_duplication() {
        // `"Obs"` amorce `"Observation:"`, mais la suite ne confirme pas : le
        // texte retenu doit ressortir intact, une seule fois.
        let (finish, text, _) =
            replay_frags(&["Obs", "curité totale"], &["Observation:"]);
        assert_eq!(finish, Finish::eos());
        assert_eq!(text, "Obscurité totale");
    }

    #[test]
    fn a_false_start_at_the_very_end_of_the_stream_is_flushed() {
        // Le flux se termine alors qu'on retenait encore un préfixe : il sort.
        let (finish, text, _) = replay_frags(&["fin de texte : Obs"], &["Observation:"]);
        assert_eq!(finish, Finish::eos());
        assert_eq!(text, "fin de texte : Obs");
    }

    #[test]
    fn earliest_stop_wins_then_the_longest() {
        // La plus précoce gagne, même déclarée en second.
        let (finish, text, _) =
            replay_frags(&["réponse ici et la suite"], &["suite", "ici"]);
        assert_eq!(finish, Finish::stop("ici"));
        assert_eq!(text, "réponse ");

        // À position égale, la plus longue gagne.
        let (finish, text, _) = replay_frags(&["avant STOPNET après"], &["STOP", "STOPNET"]);
        assert_eq!(finish, Finish::stop("STOPNET"));
        assert_eq!(text, "avant ");
    }

    #[test]
    fn holdback_never_splits_a_multibyte_character() {
        // `é` fait deux octets : une rétention naïve couperait dedans.
        let (finish, text, _) = replay_frags(&["café", "é FIN"], &["éé"]);
        assert_eq!(finish, Finish::stop("éé"));
        assert_eq!(text, "caf");
    }

    #[test]
    fn eos_without_any_stop_sequence_reports_eos() {
        let mut sink = StringSink::default();
        let opts = GenOptions::default().with_stop(vec!["FIN".into()]);
        let (finish, _) = replay(TEXT, &opts, &mut sink);
        assert_eq!(finish, Finish::eos(), "aucune séquence dans le flux");
        assert!(finish.is_complete());
        assert_eq!(sink.text, "Bonjour le monde", "rien n'est retenu à tort");
    }

    #[test]
    fn empty_stop_sequence_is_ignored_and_nothing_is_held_back() {
        let opts = GenOptions::default().with_stop(vec![String::new()]);
        let mut sink = StringSink::default();
        let (finish, _) = replay(TEXT, &opts, &mut sink);
        assert_eq!(finish, Finish::eos());
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
        assert_eq!(finish, Finish::cancelled());
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
        assert_eq!(finish, Finish::stop("FIN"));
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
        assert_eq!(finish.reason, FinishReason::ToolCall, "eu {finish:?}");
        let calls = &finish.tool_calls;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_a1");
        assert_eq!(calls[0].name, "KBQuerySourceNode");
        // Les fragments recollés forment un JSON valide.
        let args: Value = serde_json::from_str(&calls[0].arguments).unwrap();
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
        assert_eq!(finish.reason, FinishReason::ToolCall);
        let calls = &finish.tool_calls;
        assert_eq!(calls[0].name, "f_a");
        assert_eq!(calls[0].arguments, "{\"x\":1}");
        assert_eq!(calls[1].name, "f_b");
        assert_eq!(calls[1].arguments, "{\"y\":2}");
    }

    #[test]
    fn length_maps_to_max_tokens() {
        let frames = &[
            r#"{"choices":[{"index":0,"delta":{"content":"tronq"},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{},"finish_reason":"length"}]}"#,
        ];
        let mut sink = StringSink::default();
        let (finish, _) = replay(frames, &GenOptions::default(), &mut sink);
        assert_eq!(finish, Finish::max_tokens());
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
        assert_eq!(finish, Finish::eos());
    }

    #[test]
    fn error_chunk_becomes_a_model_error() {
        let body = "data: {\"error\":{\"message\":\"boom\"}}\n\n";
        let mut r = BufReader::new(body.as_bytes());
        let mut sink = StringSink::default();
        let err = read_sse(&mut r, &GenOptions::default(), &mut sink).unwrap_err();
        assert!(matches!(err, LlmError::Model(m) if m.contains("boom")), "eu autre chose");
    }

    /// Les fiches `edit` / `list` (feature `code`) ont des paramètres à
    /// défaut `""` : le corps envoyé doit rester du JSON valide — Vertex a
    /// répondu « Expected a valid JSON object in the request » (25 août 2026).
    #[cfg(feature = "code")]
    #[test]
    fn request_body_with_code_tools_is_valid_json() {
        let llm = OpenAiLlm::new("http://localhost:1/v1", "m");
        let (_, tools) = crate::dataflow::graph_tool::builtin_graph_tools().unwrap();
        let defs = crate::tools::graph_tool_defs(&tools);
        let opts = GenOptions::default().with_max_tokens(64).with_tools(defs);
        let body = llm.request_body(&hello(), &opts);
        let text = serde_json::to_string(&body).unwrap();
        let back: serde_json::Value = serde_json::from_str(&text).expect("the request body must round-trip as JSON");
        let names: Vec<&str> = back["tools"].as_array().unwrap().iter().map(|t| t["function"]["name"].as_str().unwrap()).collect();
        assert_eq!(names, crate::dataflow::graph_tool::BUILTIN_TOOL_NAMES);
        let edit = &back["tools"][0]["function"]["parameters"];
        eprintln!("{}", serde_json::to_string_pretty(edit).unwrap());
        assert_eq!(edit["properties"]["old"]["default"], "");
    }

    /// Un flux coupé par Vertex : les fragments d'un appel, puis un objet
    /// d'erreur multi-ligne sans `data:`. Ce n'est pas un appel tronqué à
    /// exécuter, c'est une erreur.
    #[test]
    fn a_stray_error_object_after_the_stream_is_an_error() {
        let body = concat!(
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"z\",\"function\":{\"name\":\"edit\",\"arguments\":\"{\\\"new\\\":\\\"a\"}}]},\"finish_reason\":null}]}\n\n",
            "[{\n  \"error\": {\n    \"code\": 499,\n    \"message\": \"The operation was cancelled.\",\n    \"status\": \"CANCELLED\"\n  }\n}\n]\n"
        );
        let mut reader = std::io::Cursor::new(body);
        let mut sink = StringSink::default();
        let err = read_sse(&mut reader, &GenOptions::default(), &mut sink).unwrap_err();
        assert!(err.to_string().contains("499"), "{err}");
    }

    /// Ce qu'on renvoie au fournisseur est toujours un objet JSON valide.
    #[test]
    fn resent_tool_call_arguments_are_valid_objects() {
        let llm = OpenAiLlm::new("http://localhost:1/v1", "m");
        let raw_newline = ToolCall::local("t", 0, "edit", "{\"new\":\"a\nb\"}");
        let truncated = ToolCall::local("t", 1, "edit", "{\"new\":\"pub fn len(");
        let turns = vec![
            Turn::user("q"),
            Turn::assistant_with_calls("", vec![raw_newline, truncated]),
            Turn::tool_result("t-0", "edit", "ok"),
            Turn::tool_result("t-1", "edit", "ok"),
        ];
        let body = llm.request_body(&turns, &GenOptions::default());
        let calls = body["messages"][1]["tool_calls"].as_array().unwrap();
        let a0 = calls[0]["function"]["arguments"].as_str().unwrap();
        let a1 = calls[1]["function"]["arguments"].as_str().unwrap();
        assert_eq!(serde_json::from_str::<Value>(a0).unwrap()["new"], "a\nb");
        assert_eq!(a1, "{}");
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
        // Tous les nœuds passent en outils, dans la forme de `tools.rs`.
        assert_eq!(body["tools"].as_array().unwrap().len(), crate::dataflow::node_factories::BUILTIN_NODE_COUNT);
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

        // ...et les deux constructeurs Google ne l'écrivent que si on leur
        // demande la fragmentation des arguments (défectueuse sur les
        // valeurs multi-lignes, 25 août 2026).
        for llm in [
            OpenAiLlm::ai_studio("cle", "google/gemini-2.5-flash"),
            OpenAiLlm::vertex("jeton", "projet", "global", "google/gemini-2.5-flash"),
        ] {
            let body = llm.request_body(&hello(), &opts);
            assert!(body["extra_body"].is_null(), "off by default: {}", body["extra_body"]);
            let body = llm.with_streamed_tool_arguments().request_body(&hello(), &opts);
            assert_eq!(body["extra_body"]["google"]["stream_function_call_arguments"], true);
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

    // ─── Aller-retour d'un historique avec outils ───────────────────────────

    /// Relit un corps de requête en tours — l'inverse de `message_json`.
    /// C'est ce qui permet de prouver l'aller-retour plutôt que de l'affirmer.
    fn turns_from_body(body: &Value) -> Vec<Turn> {
        body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| {
                let role = m["role"].as_str().unwrap();
                let content = m["content"].as_str().unwrap_or("").to_string();
                if let Some(id) = m["tool_call_id"].as_str() {
                    Turn::tool_result(id, m["name"].as_str().unwrap_or(""), content)
                } else if let Some(calls) = m["tool_calls"].as_array() {
                    Turn::assistant_with_calls(
                        content,
                        calls
                            .iter()
                            .map(|c| {
                                let call = ToolCall::new(
                                    c["id"].as_str().unwrap(),
                                    c["function"]["name"].as_str().unwrap(),
                                    c["function"]["arguments"].as_str().unwrap(),
                                );
                                match c.get("extra_content") {
                                    Some(e) => call.with_provider_extra(e.clone()),
                                    None => call,
                                }
                            })
                            .collect(),
                    )
                } else {
                    Turn::new(role, content)
                }
            })
            .collect()
    }

    fn history_with_two_parallel_calls() -> (Vec<Turn>, Vec<ToolCall>) {
        let calls = vec![
            ToolCall::new("call_a1", "KBQuerySourceNode", r#"{"kb_name":"docs","query":"a"}"#),
            ToolCall::new("call_b2", "ComposeNode", r#"{"template":"b"}"#),
        ];
        let turns = vec![
            Turn::system("tu es utile"),
            Turn::user("fais les deux"),
            Turn::assistant_with_calls("", calls.clone()),
            Turn::tool_result("call_a1", "KBQuerySourceNode", "12 résultats"),
            Turn::tool_result("call_b2", "ComposeNode", "assemblé"),
            Turn::user("et maintenant ?"),
        ];
        (turns, calls)
    }

    #[test]
    fn round_trip_preserves_tool_call_ids_and_their_order() {
        // Le cœur de la demande : sérialiser un historique à deux appels
        // parallèles, le relire, et retrouver les mêmes identifiants dans le
        // même ordre.
        let (turns, calls) = history_with_two_parallel_calls();
        let llm = OpenAiLlm::new("http://x/v1", "m");
        let body = llm.request_body(&turns, &GenOptions::default());
        let back = turns_from_body(&body);

        assert_eq!(back, turns, "l'aller-retour doit être exact");

        let ids: Vec<&str> =
            back.iter().flat_map(|t| t.tool_calls.iter()).map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["call_a1", "call_b2"], "identifiants et ordre conservés");
        let answered: Vec<&str> =
            back.iter().filter_map(|t| t.tool_call_id.as_deref()).collect();
        assert_eq!(answered, ["call_a1", "call_b2"]);
        // Les arguments restent des chaînes de JSON, pas des objets.
        for c in &calls {
            assert!(serde_json::from_str::<Value>(&c.arguments).is_ok());
        }
        // Et l'historique est bien formé au sens du fournisseur.
        assert!(crate::llm::orphan_tool_calls(&back).is_empty());
        assert!(crate::llm::dangling_tool_results(&back).is_empty());
    }

    #[test]
    fn assistant_with_calls_serializes_the_protocol_shape() {
        let (turns, _) = history_with_two_parallel_calls();
        let llm = OpenAiLlm::new("http://x/v1", "m");
        let body = llm.request_body(&turns, &GenOptions::default());
        let asst = &body["messages"][2];
        assert_eq!(asst["role"], "assistant");
        // `content: null` explicite quand le modèle n'a rien dit.
        assert!(asst["content"].is_null());
        assert_eq!(asst["tool_calls"][0]["id"], "call_a1");
        assert_eq!(asst["tool_calls"][0]["type"], "function");
        assert_eq!(asst["tool_calls"][0]["function"]["name"], "KBQuerySourceNode");
        // `arguments` est une CHAÎNE contenant du JSON.
        assert!(asst["tool_calls"][0]["function"]["arguments"].is_string());

        let res = &body["messages"][3];
        assert_eq!(res["role"], "tool");
        assert_eq!(res["tool_call_id"], "call_a1");
        assert_eq!(res["name"], "KBQuerySourceNode");
        assert!(res["content"].is_string());
        assert!(res.get("tool_calls").is_none(), "un résultat n'annonce rien");

        // Un assistant qui a parlé ET appelé garde son texte.
        let t = Turn::assistant_with_calls("je regarde", vec![ToolCall::new("i", "n", "{}")]);
        let body = llm.request_body(&[t], &GenOptions::default());
        assert_eq!(body["messages"][0]["content"], "je regarde");
    }

    #[test]
    fn an_assistant_with_calls_never_serializes_an_empty_string_content() {
        // Gemini rejette `"content": ""` sur un assistant qui annonce des
        // appels. On doit émettre `null`, et pour toutes les variantes de
        // construction.
        let call = ToolCall::new("call_A", "f", "{}");
        let llm = OpenAiLlm::new("http://x/v1", "m");
        for t in [
            Turn::assistant_with_calls("", vec![call.clone()]),
            Turn { tool_calls: vec![call], ..Turn::assistant("") },
        ] {
            let body = llm.request_body(&[t], &GenOptions::default());
            let c = &body["messages"][0]["content"];
            assert!(c.is_null(), "attendu null, eu {c}");
            assert_ne!(c.as_str(), Some(""), "la chaîne vide fait 400 chez Gemini");
        }
    }

    #[test]
    fn an_interrupted_call_is_closed_and_the_body_becomes_well_formed() {
        // Le modèle annonce `call_X`, le puits coupe en plein flux.
        let frames = &[
            r#"{"choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"call_X","type":"function","function":{"name":"KBQuerySourceNode","arguments":"{\"kb_"}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{"content":"je cherche"},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{"content":" encore"},"finish_reason":null}]}"#,
        ];
        let mut sink = CountingSink::stopping_after(1);
        let (finish, _) = replay(frames, &GenOptions::default(), &mut sink);

        assert_eq!(finish.reason, FinishReason::Cancelled);
        assert!(!finish.is_complete());
        // L'identifiant annoncé a survécu à l'annulation.
        assert_eq!(finish.tool_calls.len(), 1);
        assert_eq!(finish.tool_calls[0].id, "call_X");
        assert_eq!(finish.tool_calls[0].name, "KBQuerySourceNode");
        // Les arguments sont tronqués — donc invalides — mais c'est l'`id`
        // qui compte pour refermer.
        assert!(serde_json::from_str::<Value>(&finish.tool_calls[0].arguments).is_err());

        // On reprend : le tour d'assistant est reconstruit, l'appel refermé.
        let mut turns = vec![
            Turn::user("cherche"),
            Turn::assistant_with_calls("", finish.tool_calls.clone()),
        ];
        assert_eq!(crate::llm::orphan_tool_calls(&turns).len(), 1, "malformé tel quel");
        crate::llm::close_orphan_tool_calls(&mut turns, crate::llm::INTERRUPTED_TOOL_RESULT);

        let llm = OpenAiLlm::new("http://x/v1", "m");
        let body = llm.request_body(&turns, &GenOptions::default());
        let back = turns_from_body(&body);
        assert!(crate::llm::orphan_tool_calls(&back).is_empty(), "chaque appel a son résultat");
        assert_eq!(body["messages"][2]["tool_call_id"], "call_X");
    }

    #[test]
    fn partially_executed_parallel_calls_all_get_a_result() {
        // Trois appels annoncés, deux exécutés avant l'interruption : les
        // trois doivent repartir avec un résultat, sinon 400.
        let frames = &[
            // Une trame SSE tient sur UNE ligne : pas de retour à la ligne ici.
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"f1","arguments":"{}"}},{"index":1,"id":"call_2","function":{"name":"f2","arguments":"{}"}},{"index":2,"id":"call_3","function":{"name":"f3","arguments":"{}"}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{"content":"a"},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{"content":"b"},"finish_reason":null}]}"#,
        ];
        let mut sink = CountingSink::stopping_after(1);
        let (finish, _) = replay(frames, &GenOptions::default(), &mut sink);
        assert_eq!(finish.reason, FinishReason::Cancelled);
        assert_eq!(finish.tool_calls.len(), 3, "les trois identifiants survivent");

        let mut turns = vec![
            Turn::user("fais trois choses"),
            Turn::assistant_with_calls("", finish.tool_calls.clone()),
            Turn::tool_result("call_1", "f1", "ok 1"),
            Turn::tool_result("call_2", "f2", "ok 2"),
        ];
        assert_eq!(crate::llm::orphan_tool_calls(&turns).len(), 1);
        assert_eq!(
            crate::llm::close_orphan_tool_calls(&mut turns, crate::llm::INTERRUPTED_TOOL_RESULT),
            1
        );

        let llm = OpenAiLlm::new("http://x/v1", "m");
        let body = llm.request_body(&turns, &GenOptions::default());
        let back = turns_from_body(&body);
        assert!(crate::llm::orphan_tool_calls(&back).is_empty());
        let answered: Vec<&str> =
            back.iter().filter_map(|t| t.tool_call_id.as_deref()).collect();
        assert_eq!(answered, ["call_1", "call_2", "call_3"]);
        // Les deux exécutés gardent leur vrai résultat.
        assert_eq!(body["messages"][2]["content"], "ok 1");
        assert_eq!(body["messages"][4]["content"], crate::llm::INTERRUPTED_TOOL_RESULT);
    }

    // ─── reasoning_effort ───────────────────────────────────────────────────

    #[test]
    fn reasoning_effort_is_absent_by_default() {
        // Ne rien envoyer = ne rien changer pour un fournisseur qui ne connaît
        // pas le réglage.
        let llm = OpenAiLlm::new("http://x/v1", "m");
        let body = llm.request_body(&hello(), &GenOptions::default());
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn reasoning_effort_serializes_the_four_accepted_values() {
        let llm = OpenAiLlm::new("http://x/v1", "m");
        for (effort, expected) in [
            (ReasoningEffort::Minimal, "minimal"),
            (ReasoningEffort::Low, "low"),
            (ReasoningEffort::Medium, "medium"),
            (ReasoningEffort::High, "high"),
        ] {
            let opts = GenOptions::default().with_reasoning(effort);
            let body = llm.request_body(&hello(), &opts);
            assert_eq!(body["reasoning_effort"], expected);
            assert_eq!(effort.to_string(), expected);
        }
    }

    #[test]
    fn reasoning_effort_goes_to_every_provider_and_thinking_config_is_never_used() {
        // C'est un paramètre de premier plan, pas une extension Google : il
        // part même vers un fournisseur générique. Et on n'émet JAMAIS
        // `thinking_config`, qui serait avalé en silence.
        let opts = GenOptions::default().with_reasoning(ReasoningEffort::Low);
        for llm in [
            OpenAiLlm::new("http://x/v1", "m"),
            OpenAiLlm::ai_studio("k", "gemini-3.5-flash"),
            OpenAiLlm::vertex("p", "global", "t", "google/gemini-3.5-flash"),
        ] {
            let body = llm.request_body(&hello(), &opts);
            assert_eq!(body["reasoning_effort"], "low");
            let dump = body.to_string();
            assert!(!dump.contains("thinking_config"), "thinking_config émis : {dump}");
            assert!(!dump.contains("thinking_budget"));
        }
    }

    #[test]
    fn without_reasoning_clears_it() {
        let opts = GenOptions::default()
            .with_reasoning(ReasoningEffort::High)
            .without_reasoning();
        assert!(opts.reasoning.is_none());
    }

    // ─── thought_signature (opaque) ─────────────────────────────────────────

    /// Trames d'un appel d'outil Gemini 3.x, signature comprise.
    const SIGNED: &[&str] = &[
        r#"{"choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"call_X","type":"function","function":{"name":"KBQuerySourceNode","arguments":"{\"kb_"}}]},"finish_reason":null}]}"#,
        r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_X","function":{"arguments":"name\":\"docs\"}"},"extra_content":{"google":{"thought_signature":"Cq4BAdHtim9sig=="}}}]},"finish_reason":null}]}"#,
        r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#,
    ];

    #[test]
    fn thought_signature_is_captured_opaquely() {
        let mut sink = StringSink::default();
        let (finish, _) = replay(SIGNED, &GenOptions::default(), &mut sink);
        assert_eq!(finish.reason, FinishReason::ToolCall);
        let extra = finish.tool_calls[0].provider_extra.as_ref().expect("signature captée");
        // Capté tel quel, sans que `llm.rs` sache ce que c'est.
        assert_eq!(extra["google"]["thought_signature"], "Cq4BAdHtim9sig==");
    }

    #[test]
    fn thought_signature_survives_a_full_round_trip_identically() {
        // Le tour 1 produit la signature ; le tour 2 doit la représenter au
        // fournisseur **à l'identique**, sinon 400 sur Gemini 3.x.
        let mut sink = StringSink::default();
        let (finish, _) = replay(SIGNED, &GenOptions::default(), &mut sink);
        let original = finish.tool_calls[0].provider_extra.clone().unwrap();

        let mut turns = vec![
            Turn::user("cherche"),
            Turn::assistant_with_calls("", finish.tool_calls.clone()),
        ];
        crate::llm::close_orphan_tool_calls(&mut turns, crate::llm::INTERRUPTED_TOOL_RESULT);

        let llm = OpenAiLlm::vertex("p", "global", "t", "google/gemini-3.5-flash");
        let body = llm.request_body(&turns, &GenOptions::default());
        // Présente dans le corps, au bon endroit, à l'identique.
        assert_eq!(body["messages"][1]["tool_calls"][0]["extra_content"], original);
        assert_eq!(
            body["messages"][1]["tool_calls"][0]["extra_content"]["google"]["thought_signature"],
            "Cq4BAdHtim9sig=="
        );
        // Et l'aller-retour complet la rend bit pour bit.
        let back = turns_from_body(&body);
        assert_eq!(back[1].tool_calls[0].provider_extra.as_ref(), Some(&original));
        assert_eq!(back[1].tool_calls, turns[1].tool_calls);
    }

    #[test]
    fn thought_signature_survives_a_cancellation() {
        // L'invariant couvre aussi l'opaque : une interruption ne doit pas
        // plus perdre la signature qu'elle ne perd l'identifiant.
        let mut sink = CountingSink::stopping_after(1);
        let frames = &[
            SIGNED[0],
            SIGNED[1],
            r#"{"choices":[{"index":0,"delta":{"content":"je cherche"},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{"content":" encore"},"finish_reason":null}]}"#,
        ];
        let (finish, _) = replay(frames, &GenOptions::default(), &mut sink);
        assert_eq!(finish.reason, FinishReason::Cancelled);
        assert_eq!(finish.tool_calls[0].id, "call_X");
        assert_eq!(
            finish.tool_calls[0].provider_extra.as_ref().unwrap()["google"]
                ["thought_signature"],
            "Cq4BAdHtim9sig=="
        );
    }

    #[test]
    fn parallel_calls_only_the_first_is_signed_and_that_is_preserved() {
        // Forme réelle de Gemini en appels parallèles : UNE signature, sur le
        // premier appel. Les suivants n'en ont pas et n'en attendent pas — il
        // ne faut donc surtout pas leur en fabriquer une.
        let frames = &[
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_1","type":"function","function":{"name":"f1","arguments":"{}"},"extra_content":{"google":{"thought_signature":"SIG1"}}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_2","type":"function","function":{"name":"f2","arguments":"{}"}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#,
        ];
        let mut sink = StringSink::default();
        let (finish, _) = replay(frames, &GenOptions::default(), &mut sink);
        assert_eq!(finish.tool_calls.len(), 2, "les deux appels malgré l'absence d'`index`");
        assert_eq!(finish.tool_calls[0].id, "call_1");
        assert_eq!(finish.tool_calls[1].id, "call_2");
        assert!(finish.tool_calls[0].provider_extra.is_some());
        assert!(finish.tool_calls[1].provider_extra.is_none(), "absence légitime");

        // Au rejeu : la signature du premier repart, le second reste nu — pas
        // d'échappatoire fabriquée, le tour est déjà d'origine Google.
        let turns = vec![Turn::assistant_with_calls("", finish.tool_calls.clone())];
        let llm = OpenAiLlm::vertex("p", "global", "t", "google/gemini-3.5-flash");
        let body = llm.request_body(&turns, &GenOptions::default());
        let calls = &body["messages"][0]["tool_calls"];
        assert_eq!(calls[0]["extra_content"]["google"]["thought_signature"], "SIG1");
        assert!(calls[1].get("extra_content").is_none(), "rien à fabriquer ici : {}", calls[1]);
    }

    #[test]
    fn sequential_calls_keep_one_signature_each() {
        let calls = vec![
            ToolCall::new("c1", "f1", "{}")
                .with_provider_extra(json!({"google":{"thought_signature":"S1"}})),
            ToolCall::new("c2", "f2", "{}")
                .with_provider_extra(json!({"google":{"thought_signature":"S2"}})),
        ];
        let turns = vec![Turn::assistant_with_calls("", calls)];
        let llm = OpenAiLlm::vertex("p", "global", "t", "google/gemini-3.5-flash");
        let body = llm.request_body(&turns, &GenOptions::default());
        let c = &body["messages"][0]["tool_calls"];
        assert_eq!(c[0]["extra_content"]["google"]["thought_signature"], "S1");
        assert_eq!(c[1]["extra_content"]["google"]["thought_signature"], "S2");
        // Aller-retour : chacune revient sur son propre appel.
        let back = turns_from_body(&body);
        assert_eq!(back[0].tool_calls[0].provider_extra, turns[0].tool_calls[0].provider_extra);
        assert_eq!(back[0].tool_calls[1].provider_extra, turns[0].tool_calls[1].provider_extra);
    }

    #[test]
    fn a_locally_born_call_gets_the_escape_hatch_only_towards_google() {
        // Le pont entre les deux mondes : une conversation née sur le modèle
        // local doit rester rejouable vers Gemini 3.x.
        let call = ToolCall::local("ctx", 0, "KBQuerySourceNode", r#"{"kb_name":"docs"}"#);
        assert!(call.provider_extra.is_none());
        let turns = vec![Turn::assistant_with_calls("", vec![call])];

        for llm in [
            OpenAiLlm::vertex("p", "global", "t", "google/gemini-3.5-flash"),
            OpenAiLlm::ai_studio("k", "gemini-3.5-flash"),
        ] {
            let body = llm.request_body(&turns, &GenOptions::default());
            assert_eq!(
                body["messages"][0]["tool_calls"][0]["extra_content"]["google"]
                    ["thought_signature"],
                crate::llm::SKIP_THOUGHT_SIGNATURE_VALIDATOR,
                "sans ce pont, Gemini 3.x répondrait 400"
            );
        }

        // Mais jamais vers un fournisseur non-Google : ce serait un champ
        // inconnu qu'un serveur strict pourrait refuser.
        let generic = OpenAiLlm::new("http://x/v1", "m");
        let body = generic.request_body(&turns, &GenOptions::default());
        assert!(!body.to_string().contains("extra_content"));
        assert!(!body.to_string().contains("skip_thought_signature_validator"));
    }

    #[test]
    fn closing_orphans_keeps_all_calls_before_all_results() {
        // Ordre imposé par Google en appels parallèles : FC1, FC2, FC3, puis
        // FR1, FR2, FR3. Entrelacer donne un 400. Trois appels, deux
        // orphelins : le comblement ne doit pas casser cet ordre.
        let calls: Vec<ToolCall> = (1..=3)
            .map(|i| ToolCall::new(format!("call_{i}"), format!("f{i}"), "{}"))
            .collect();
        let mut turns = vec![
            Turn::user("fais trois choses"),
            Turn::assistant_with_calls("", calls),
            Turn::tool_result("call_1", "f1", "ok 1"),
        ];
        assert_eq!(
            crate::llm::close_orphan_tool_calls(&mut turns, crate::llm::INTERRUPTED_TOOL_RESULT),
            2
        );

        let llm = OpenAiLlm::vertex("p", "global", "t", "google/gemini-3.5-flash");
        let body = llm.request_body(&turns, &GenOptions::default());
        let msgs = body["messages"].as_array().unwrap();
        let roles: Vec<&str> = msgs.iter().map(|m| m["role"].as_str().unwrap()).collect();
        assert_eq!(roles, ["user", "assistant", "tool", "tool", "tool"]);
        // Les trois appels sont dans UN message assistant, avant tout résultat.
        assert_eq!(msgs[1]["tool_calls"].as_array().unwrap().len(), 3);
        let answered: Vec<&str> =
            msgs.iter().filter_map(|m| m["tool_call_id"].as_str()).collect();
        assert_eq!(answered, ["call_1", "call_2", "call_3"], "ordre d'annonce conservé");
        assert!(crate::llm::orphan_tool_calls(&turns).is_empty());
    }

    #[test]
    fn a_provider_call_without_signature_is_fine() {
        // Absence légitime : Gemini 2.5, ou modèle sans réflexion. Rien ne
        // doit casser à la lecture.
        let frames = &[
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_Y","type":"function","function":{"name":"f","arguments":"{}"}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#,
        ];
        let mut sink = StringSink::default();
        let (finish, _) = replay(frames, &GenOptions::default(), &mut sink);
        assert_eq!(finish.reason, FinishReason::ToolCall);
        assert!(finish.tool_calls[0].provider_extra.is_none());

        // Au rejeu vers Google, un tour SANS aucune signature reçoit
        // l'échappatoire — voir `skip_validator`. C'est un arbitrage assumé :
        // on ne sait pas distinguer « réponse Gemini 2.5, non signée par
        // nature » de « appel fabriqué chez nous ». Ne rien mettre risque un
        // **400 dur** sur Gemini 3.x ; mettre l'échappatoire risque une
        // **dégradation douce** sur 2.5. On préfère l'échec doux.
        let turns = vec![Turn::assistant_with_calls("", finish.tool_calls.clone())];
        let llm = OpenAiLlm::vertex("p", "global", "t", "google/gemini-3.5-flash");
        let body = llm.request_body(&turns, &GenOptions::default());
        assert_eq!(
            body["messages"][0]["tool_calls"][0]["extra_content"]["google"]
                ["thought_signature"],
            crate::llm::SKIP_THOUGHT_SIGNATURE_VALIDATOR
        );
        // Vers un fournisseur générique, en revanche, rien n'est ajouté.
        let generic = OpenAiLlm::new("http://x/v1", "m");
        assert!(!generic
            .request_body(&turns, &GenOptions::default())
            .to_string()
            .contains("extra_content"));
    }

    #[test]
    fn a_local_tool_call_serializes_no_extra_content() {
        // Chemin local : rien à rejouer, donc la clé ne doit pas apparaître —
        // pas même en `null`, qu'il faudrait espérer voir toléré.
        let call = ToolCall::local("ctx", 0, "KBQuerySourceNode", r#"{"kb_name":"docs"}"#);
        assert!(call.provider_extra.is_none());
        let turns = vec![Turn::assistant_with_calls("", vec![call])];
        let llm = OpenAiLlm::new("http://x/v1", "m");
        let body = llm.request_body(&turns, &GenOptions::default());
        let serialized = &body["messages"][0]["tool_calls"][0];
        assert!(serialized.get("extra_content").is_none(), "clé superflue : {serialized}");
        assert!(!body.to_string().contains("extra_content"));
        // Le reste de la forme est intact.
        assert_eq!(serialized["type"], "function");
        assert!(serialized["function"]["arguments"].is_string());
    }

    // ─── tool_choice ────────────────────────────────────────────────────────

    #[test]
    fn tool_choice_serializes_the_four_forms() {
        let llm = OpenAiLlm::new("http://x/v1", "m");
        let tool = ToolDef {
            name: "KBQuerySourceNode".into(),
            description: "d".into(),
            parameters: json!({"type":"object","properties":{},"additionalProperties":false}),
        };
        let base = GenOptions::default().with_tools(vec![tool]);

        // Défaut : `auto`, exactement le comportement historique.
        assert_eq!(llm.request_body(&hello(), &base)["tool_choice"], "auto");

        for (choice, expected) in [
            (ToolChoice::Auto, json!("auto")),
            (ToolChoice::Required, json!("required")),
            (ToolChoice::None, json!("none")),
            (
                ToolChoice::Function("KBQuerySourceNode".into()),
                json!({"type":"function","function":{"name":"KBQuerySourceNode"}}),
            ),
        ] {
            let opts = base.clone().with_tool_choice(choice);
            assert_eq!(llm.request_body(&hello(), &opts)["tool_choice"], expected);
        }
    }

    #[test]
    fn tool_choice_is_not_sent_without_tools() {
        // Un `tool_choice` orphelin est au mieux inutile, au pire un 400.
        let llm = OpenAiLlm::new("http://x/v1", "m");
        let opts = GenOptions::default().with_tool_choice(ToolChoice::Required);
        let body = llm.request_body(&hello(), &opts);
        assert!(body.get("tool_choice").is_none());
        assert!(body.get("tools").is_none());
    }

    #[test]
    fn a_tool_call_finishing_with_stop_is_still_a_tool_call() {
        // Ce filet est **actif**, pas théorique. Trois sources distinctes de
        // `finish_reason: "stop"` sur un tour qui appelle pourtant un outil :
        //
        // 1. **OpenAI, outil nommé.** Attesté par un membre du staff, puis
        //    reproduit (openai-node #305, openai-dotnet #64). Aujourd'hui ni
        //    documenté ni garanti — donc à tolérer, jamais à supposer.
        // 2. **Google, en streaming.** Signalé le 20 décembre 2025 : sur
        //    l'endpoint compatible OpenAI, le même appel rendrait
        //    `"tool_calls"` en non-streamé et `"stop"` en streamé. **Non
        //    reproduit chez nous au 25 août 2026** — nos mesures sur
        //    `gemini-3.5-flash` et `gemini-3.7-flash`, en streaming, rendent
        //    bien `"tool_calls"`. Gardé par précaution : trois signalements
        //    indépendants existent, le défaut peut dépendre du modèle, et le
        //    filet ne coûte rien.
        // 3. Les passerelles compatibles OpenAI qui recopient mal le champ.
        //
        // La mesure du 25 août 2026 sur `gemini-3.5-flash` (quatre formes de
        // `tool_choice`, toutes rendant `"tool_calls"`) ne contredit pas le
        // point 2 : le défaut dépend du modèle. Conclusion opérationnelle,
        // valable pour les trois fournisseurs : **ne jamais trancher sur
        // `finish_reason` seul**, toujours sur la présence d'appels accumulés.
        let frames = &[
            r#"{"choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"call_S","type":"function","function":{"name":"KBQuerySourceNode","arguments":"{\"kb_name\":"}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"docs\"}"}}]},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#,
        ];
        let mut sink = StringSink::default();
        let (finish, _) = replay(frames, &GenOptions::default(), &mut sink);

        assert_eq!(
            finish.reason,
            FinishReason::ToolCall,
            "`stop` + appels accumulés doit rester un appel d'outil"
        );
        assert!(finish.is_complete());
        assert_eq!(finish.tool_calls.len(), 1);
        assert_eq!(finish.tool_calls[0].id, "call_S");
        let args: Value = serde_json::from_str(&finish.tool_calls[0].arguments).unwrap();
        assert_eq!(args["kb_name"], "docs");
    }

    #[test]
    fn stop_without_any_tool_call_stays_eos() {
        // Le pendant : sans appel accumulé, `stop` reste une fin de texte.
        let mut sink = StringSink::default();
        let (finish, _) = replay(TEXT, &GenOptions::default(), &mut sink);
        assert_eq!(finish.reason, FinishReason::Eos);
        assert!(finish.tool_calls.is_empty());
    }

    // ─── response_format ────────────────────────────────────────────────────

    #[test]
    fn tool_choice_has_the_same_shape_for_every_provider() {
        // `tool_choice` est un paramètre standard, pas une extension Google :
        // même forme partout, et il cohabite avec `extra_body.google`.
        let tool = ToolDef {
            name: "f".into(),
            description: "d".into(),
            parameters: json!({"type":"object","properties":{},"additionalProperties":false}),
        };
        let opts = GenOptions::default()
            .with_tools(vec![tool])
            .with_tool_choice(ToolChoice::Function("f".into()));
        for llm in [
            OpenAiLlm::new("http://x/v1", "m"),
            OpenAiLlm::ai_studio("k", "gemini-3.5-flash"),
            OpenAiLlm::vertex("p", "global", "t", "google/gemini-3.5-flash"),
        ] {
            let body = llm.request_body(&hello(), &opts);
            assert_eq!(
                body["tool_choice"],
                json!({"type":"function","function":{"name":"f"}})
            );
        }
        // L'extension Google reste dans `extra_body`, séparée — et seulement
        // sur demande (défectueuse sur les arguments multi-lignes).
        let vertex = OpenAiLlm::vertex("p", "global", "t", "google/gemini-3.5-flash");
        let body = vertex.request_body(&hello(), &opts);
        assert!(body["extra_body"].is_null(), "off by default: {}", body["extra_body"]);
        let streamed = vertex.with_streamed_tool_arguments();
        let body = streamed.request_body(&hello(), &opts);
        assert_eq!(body["extra_body"]["google"]["stream_function_call_arguments"], true);
    }

    #[test]
    fn forcing_an_undeclared_tool_fails_before_any_socket() {
        let llm = OpenAiLlm::new("http://127.0.0.1:1/v1", "m");
        let tool = ToolDef {
            name: "KBQuerySourceNode".into(),
            description: "d".into(),
            parameters: json!({"type":"object","properties":{},"additionalProperties":false}),
        };
        let opts = GenOptions::default()
            .with_tools(vec![tool])
            .with_tool_choice(ToolChoice::Function("KBQuerySourceNodee".into()));
        let mut sink = StringSink::default();
        match llm.generate(&hello(), &opts, &mut sink).unwrap_err() {
            LlmError::Prompt(m) => {
                assert!(m.contains("KBQuerySourceNodee"), "{m}");
                assert!(m.contains("KBQuerySourceNode"), "les noms connus aident : {m}");
            }
            other => panic!("attendu Prompt, eu {other:?}"),
        }
    }

    #[test]
    fn response_format_is_absent_by_default() {
        let llm = OpenAiLlm::new("http://x/v1", "m");
        let body = llm.request_body(&hello(), &GenOptions::default());
        assert!(body.get("response_format").is_none());
    }

    #[test]
    fn response_format_serializes_the_three_forms() {
        let llm = OpenAiLlm::new("http://x/v1", "m");
        let schema = json!({
            "type": "object",
            "properties": { "réponse": { "type": "string" } },
            "required": ["réponse"],
            "additionalProperties": false
        });
        let cases = [
            (ResponseFormat::Text, json!({"type":"text"})),
            (ResponseFormat::JsonObject, json!({"type":"json_object"})),
            (
                ResponseFormat::strict_schema("extraction", schema.clone()),
                json!({
                    "type": "json_schema",
                    "json_schema": {"name":"extraction","schema":schema,"strict":true}
                }),
            ),
        ];
        for (format, expected) in cases {
            let opts = GenOptions::default().with_response_format(format);
            assert_eq!(llm.request_body(&hello(), &opts)["response_format"], expected);
        }
    }

    #[test]
    fn response_format_and_tools_coexist() {
        let llm = OpenAiLlm::new("http://x/v1", "m");
        let tool = ToolDef {
            name: "f".into(),
            description: "d".into(),
            parameters: json!({"type":"object","properties":{},"additionalProperties":false}),
        };
        let opts = GenOptions::default()
            .with_tools(vec![tool])
            .with_tool_choice(ToolChoice::Required)
            .with_response_format(ResponseFormat::JsonObject);
        let body = llm.request_body(&hello(), &opts);
        assert_eq!(body["tool_choice"], "required");
        assert_eq!(body["response_format"]["type"], "json_object");
        assert!(body["tools"].is_array());
    }

    // ─── mode strict : vérification avant l'envoi ───────────────────────────

    #[test]
    fn only_tool_defs_without_optional_params_satisfy_strict_mode() {
        // Contre-intuitif, et mesuré : `tools.rs` pose bien
        // `additionalProperties: false`, mais ça ne suffit pas. Le mode strict
        // exige que **tout** champ déclaré soit dans `required`, or nos
        // `ToolDef` n'y mettent que les paramètres obligatoires. Résultat au
        // 25 août 2026 : 13 des 28 nœuds passent, 15 échouent — tous ceux qui
        // ont au moins un paramètre facultatif.
        //
        // Conséquence pratique : on ne peut pas reprendre tel quel le
        // `parameters` d'un `ToolDef` comme schéma de `response_format`
        // strict. Ça ne gêne pas `tools` (les schémas d'outils ne sont pas
        // soumis à cette contrainte), seulement la réutilisation en sortie
        // structurée.
        let registry = {
            let mut r = crate::dataflow::node_registry::NodeRegistry::new();
            crate::dataflow::register_builtins(&mut r);
            r
        };

        // Sans paramètre : rien à mettre dans `required`, donc conforme.
        let compose = crate::tools::tool_def(&registry.schema("ComposeNode").unwrap());
        assert_eq!(check_strict_schema(&compose.parameters), Ok(()));

        // Avec un paramètre facultatif : refusé, et le message dit lequel et
        // comment le rendre acceptable.
        let kb = crate::tools::tool_def(&registry.schema("KBQuerySourceNode").unwrap());
        let err = check_strict_schema(&kb.parameters).unwrap_err();
        assert!(err.contains("options"), "le champ facultatif doit être nommé : {err}");
        assert!(err.contains("null"), "le remède doit être donné : {err}");

        // Et au moins un nœud échoue, sinon ce test ne prouverait rien.
        let defs = crate::tools::tool_defs(&registry);
        let refused = defs
            .iter()
            .filter(|d| check_strict_schema(&d.parameters).is_err())
            .count();
        assert!(refused > 0 && refused < defs.len(), "{refused} sur {}", defs.len());
    }

    #[test]
    fn a_json_param_is_an_open_object_and_strict_mode_says_so() {
        // La dette documentée dans `tools.rs` : un `ConfigParamType::Json`
        // devient un objet libre, sans `additionalProperties`. Le mode strict
        // le refuse — c'est le premier endroit où cette dette se paie.
        let registry = {
            let mut r = crate::dataflow::node_registry::NodeRegistry::new();
            crate::dataflow::register_builtins(&mut r);
            r
        };
        let embed = crate::tools::tool_def(&registry.schema("EmbedNode").unwrap());
        let err = check_strict_schema(&embed.parameters).unwrap_err();
        assert!(err.contains("#/signals"), "le chemin du sous-objet : {err}");
        assert!(err.contains("additionalProperties"), "{err}");
    }

    #[test]
    fn strict_check_names_every_problem_and_says_what_to_do() {
        // Objet imbriqué sans `additionalProperties`, et un champ absent de
        // `required` : les deux doivent être signalés d'un coup.
        let schema = json!({
            "type": "object",
            "properties": {
                "titre": { "type": "string" },
                "auteur": {
                    "type": "object",
                    "properties": { "nom": { "type": "string" } }
                }
            },
            "required": ["titre"],
            "additionalProperties": false
        });
        let err = check_strict_schema(&schema).unwrap_err();
        assert!(err.contains("auteur"), "{err}");
        assert!(err.contains("additionalProperties"), "{err}");
        assert!(err.contains("required"), "{err}");
        // Le message dit quoi faire pour un champ facultatif.
        assert!(err.contains("null"), "{err}");
    }

    #[test]
    fn strict_check_walks_arrays_unions_and_defs() {
        let schema = json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": { "type": "object", "properties": { "x": {"type":"string"} } }
                }
            },
            "required": ["items"],
            "additionalProperties": false
        });
        let err = check_strict_schema(&schema).unwrap_err();
        assert!(err.contains("#/items/items"), "chemin attendu dans : {err}");

        // Une racine qui n'est pas un objet est refusée d'emblée.
        let err = check_strict_schema(&json!({"type":"string"})).unwrap_err();
        assert!(err.contains("racine"), "{err}");
    }

    #[test]
    fn a_non_conforming_strict_schema_fails_before_any_socket() {
        // Port injoignable : si l'erreur n'était pas rendue avant l'ouverture,
        // on aurait une erreur de transport à la place.
        let llm = OpenAiLlm::new("http://127.0.0.1:1/v1", "m");
        let bad = json!({ "type": "object", "properties": { "a": {"type":"string"} } });
        let opts = GenOptions::default()
            .with_response_format(ResponseFormat::strict_schema("x", bad));
        let mut sink = StringSink::default();
        let err = llm.generate(&hello(), &opts, &mut sink).unwrap_err();
        match err {
            LlmError::Prompt(m) => {
                assert!(m.contains("mode strict"), "{m}");
                assert!(m.contains('x'), "le nom du schéma doit apparaître : {m}");
            }
            other => panic!("attendu Prompt, eu {other:?}"),
        }

        // Et le MÊME schéma en `strict: false` est refusé aussi. Vertex ne
        // mentionne nulle part `strict` : il l'ignore probablement, donc
        // n'attendre que de lui la contrainte reviendrait à croire la sortie
        // bornée alors qu'elle ne l'est pas.
        let lax = GenOptions::default().with_response_format(ResponseFormat::JsonSchema {
            name: "x".into(),
            schema: json!({ "type": "object", "properties": { "a": {"type":"string"} } }),
            strict: false,
        });
        let err = llm.generate(&hello(), &lax, &mut sink).unwrap_err();
        assert!(matches!(err, LlmError::Prompt(_)), "vérifié même sans `strict` : {err:?}");
    }

    #[test]
    fn validated_is_google_only_and_never_overrides_an_explicit_choice() {
        let tool = ToolDef {
            name: "f".into(),
            description: "d".into(),
            parameters: json!({"type":"object","properties":{},"additionalProperties":false}),
        };
        let base = GenOptions::default().with_tools(vec![tool]);
        let vertex = OpenAiLlm::vertex("p", "global", "t", "google/gemini-3.5-flash")
            .with_validated_tool_choice();

        // `Auto` = « je n'ai pas d'avis » : l'extension s'applique.
        assert_eq!(vertex.request_body(&hello(), &base)["tool_choice"], "validated");

        // Un choix explicite garde la main, quel qu'il soit.
        for choice in [
            ToolChoice::Required,
            ToolChoice::None,
            ToolChoice::Function("f".into()),
        ] {
            let opts = base.clone().with_tool_choice(choice.clone());
            assert_eq!(
                vertex.request_body(&hello(), &opts)["tool_choice"],
                choice.to_openai_json(),
                "{choice:?} doit primer sur `validated`"
            );
        }

        // Sans l'appel explicite, rien ne change — et `validated` ne peut pas
        // partir vers un fournisseur générique, faute de constructeur.
        let plain = OpenAiLlm::vertex("p", "global", "t", "google/gemini-3.5-flash");
        assert_eq!(plain.request_body(&hello(), &base)["tool_choice"], "auto");
        let generic = OpenAiLlm::new("http://x/v1", "m");
        assert_eq!(generic.request_body(&hello(), &base)["tool_choice"], "auto");
    }

    #[test]
    fn strict_check_refuses_the_unsupported_keywords() {
        for kw in ["allOf", "not", "if", "then", "else", "dependentRequired", "dependentSchemas"] {
            let schema = json!({
                "type": "object",
                "properties": {},
                "required": [],
                "additionalProperties": false,
                kw: {}
            });
            let err = check_strict_schema(&schema)
                .expect_err(&format!("`{kw}` aurait dû être refusé"));
            assert!(err.contains(kw), "{err}");
        }
        // `oneOf` n'est listé nulle part : on le signale, avec le remplaçant.
        let schema = json!({
            "type": "object", "properties": {}, "required": [],
            "additionalProperties": false, "oneOf": []
        });
        let err = check_strict_schema(&schema).unwrap_err();
        assert!(err.contains("oneOf") && err.contains("anyOf"), "{err}");
    }

    #[test]
    fn strict_check_accepts_refs_defs_and_recursion() {
        // `$ref`, `$defs` et la récursivité racine sont supportés en strict.
        // Le vérificateur ne doit ni les refuser ni boucler dessus.
        let schema = json!({
            "type": "object",
            "properties": {
                "enfants": { "type": "array", "items": { "$ref": "#" } },
                "nœud": { "$ref": "#/$defs/n" }
            },
            "required": ["enfants", "nœud"],
            "additionalProperties": false,
            "$defs": {
                "n": {
                    "type": "object",
                    "properties": { "v": { "type": "string" } },
                    "required": ["v"],
                    "additionalProperties": false
                }
            }
        });
        assert_eq!(check_strict_schema(&schema), Ok(()));

        // Et un `$defs` fautif est bien signalé, avec son chemin.
        let schema = json!({
            "type": "object", "properties": {}, "required": [],
            "additionalProperties": false,
            "$defs": { "n": { "type": "object", "properties": { "v": {"type":"string"} } } }
        });
        let err = check_strict_schema(&schema).unwrap_err();
        assert!(err.contains("#/$defs/n"), "{err}");
    }

    #[test]
    fn strict_check_enforces_the_documented_limits() {
        // Profondeur : 10 niveaux.
        let mut deep = json!({"type":"string"});
        for _ in 0..12 {
            deep = json!({
                "type": "object",
                "properties": { "n": deep },
                "required": ["n"],
                "additionalProperties": false
            });
        }
        let err = check_strict_schema(&deep).unwrap_err();
        assert!(err.contains("imbrication"), "{err}");

        // Énumération : 1 000 valeurs.
        let big: Vec<String> = (0..1001).map(|i| i.to_string()).collect();
        let schema = json!({
            "type": "object",
            "properties": { "e": { "type": "string", "enum": big } },
            "required": ["e"],
            "additionalProperties": false
        });
        let err = check_strict_schema(&schema).unwrap_err();
        assert!(err.contains("énumération"), "{err}");

        // Grande énumération de chaînes : plafond de longueur cumulée.
        let long: Vec<String> = (0..300).map(|i| format!("{i:0>60}")).collect();
        let schema = json!({
            "type": "object",
            "properties": { "e": { "type": "string", "enum": long } },
            "required": ["e"],
            "additionalProperties": false
        });
        let err = check_strict_schema(&schema).unwrap_err();
        assert!(err.contains("caractères"), "{err}");

        // 300 valeurs courtes restent acceptables : le seuil porte bien sur
        // la longueur cumulée, pas sur le seul nombre.
        let short: Vec<String> = (0..300).map(|i| i.to_string()).collect();
        let schema = json!({
            "type": "object",
            "properties": { "e": { "type": "string", "enum": short } },
            "required": ["e"],
            "additionalProperties": false
        });
        assert_eq!(check_strict_schema(&schema), Ok(()));
    }

    #[test]
    fn schema_names_follow_the_protocol() {
        assert_eq!(check_schema_name("extraction_v2-1"), Ok(()));
        assert!(check_schema_name("").is_err());
        assert!(check_schema_name(&"x".repeat(65)).is_err());
        let err = check_schema_name("mon schéma").unwrap_err();
        assert!(err.contains("a-zA-Z0-9_-"), "{err}");

        // Et le nom est vérifié avant toute socket.
        let llm = OpenAiLlm::new("http://127.0.0.1:1/v1", "m");
        let opts = GenOptions::default().with_response_format(ResponseFormat::strict_schema(
            "nom invalide",
            json!({"type":"object","properties":{},"required":[],"additionalProperties":false}),
        ));
        let mut sink = StringSink::default();
        assert!(matches!(
            llm.generate(&hello(), &opts, &mut sink).unwrap_err(),
            LlmError::Prompt(_)
        ));
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
