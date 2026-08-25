//! Génération de texte par un LLM décodeur, **en streaming**. Étape 1 :
//! le trait, les puits et les implémentations de test — aucun modèle.
//!
//! Même doctrine que [`crate::embedder`], [`crate::reranker`] et
//! [`crate::ocr`] : le trait [`Llm`] est la seule surface que le dataflow
//! voit (service `"llm"`, nœud [`crate::dataflow::LlmNode`]) ; le mock sert
//! aux tests, l'implémentation produit viendra derrière le même trait
//! (burn/wgpu pour le local, un fournisseur compatible OpenAI pour le
//! distant — les deux poussent leurs fragments, donc les deux entrent ici).
//!
//! ## Pourquoi un puits et pas un itérateur
//!
//! [`Llm::generate`] est **synchrone** et prend `&self`, comme les trois
//! autres traits : c'est ce qui le rend appelable tel quel depuis
//! `Node::execute(&mut self, ctx)`, qui n'est pas `async`. Les jetons ne
//! sortent donc pas par la valeur de retour mais par un [`TokenSink`] que
//! l'appelant fournit. Trois raisons de préférer ça à un `Iterator` :
//!
//! - un itérateur obligerait le générateur à rendre la main entre deux
//!   jetons, donc à sortir le cache KV et l'état d'échantillonnage de la
//!   boucle pour les stocker dans une structure — lourd, et impossible à
//!   plaquer sur un transport qui *pousse* (SSE d'une API distante) ;
//! - le retour du puits ([`Flow`]) est le **point d'annulation** : il
//!   remonte du consommateur jusqu'au générateur sans canal de contrôle
//!   séparé, et jusqu'au GPU quand le modèle sera réel ;
//! - c'est le choix qu'a fait `burn-lm` en amont (`GeneratedItemEmitter` +
//!   `InferenceJobListener`), ce qui laisse la porte ouverte.
//!
//! Le décodage contraint (grammaire compilée depuis [`crate::tools`]) et le
//! chat template arriveront à l'étape 6 : un champ `constraint` optionnel
//! dans [`GenOptions`], sans casser ce qui est ici.

use std::fmt;
use std::sync::Arc;
use std::time::Instant;

use crate::tools::ToolDef;

// ─── Flux et fin de génération ───────────────────────────────────────────────

/// Ce que le puits répond au générateur après chaque fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Continue à générer.
    Continue,
    /// Arrête tout de suite — la réponse sera **incomplète**.
    Stop,
}

/// Pourquoi la génération s'est terminée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finish {
    /// Le modèle a émis son jeton de fin : réponse complète.
    Eos,
    /// `max_tokens` atteint : réponse tronquée par notre plafond.
    MaxTokens,
    /// Une séquence de `stop` est apparue (elle est donnée) : réponse
    /// complète du point de vue de l'appelant, qui l'avait demandée.
    Stop(String),
    /// Le puits a répondu [`Flow::Stop`] : réponse **incomplète**. Distinct
    /// de [`Finish::Stop`] parce qu'une interface doit savoir si elle
    /// affiche une réponse finie ou un fragment abandonné.
    Cancelled,
    /// Le modèle demande un outil ; charge utile brute, parsée à l'étape 6.
    ToolCall(String),
}

impl Finish {
    /// Vrai si la réponse est exploitable telle quelle (rien ne manque du
    /// point de vue de l'appelant).
    pub fn is_complete(&self) -> bool {
        matches!(self, Finish::Eos | Finish::Stop(_) | Finish::ToolCall(_))
    }
}

// ─── Puits ───────────────────────────────────────────────────────────────────

/// Reçoit les fragments au fil de l'eau. `&mut self` : un puits porte son
/// état (tampon, canal, pointeur de fonction FFI).
pub trait TokenSink: Send {
    /// Appelé une fois par fragment décodé. Rendre [`Flow::Stop`] annule.
    fn on_token(&mut self, delta: &str) -> Flow;

    /// Appelé une seule fois, à la toute fin, quelle que soit la raison.
    fn on_finish(&mut self, _reason: &Finish) {}
}

/// Puits qui accumule tout : pour un nœud non streamant, ou un test de
/// parité entre deux implémentations.
#[derive(Debug, Clone, Default)]
pub struct StringSink {
    pub text: String,
}

impl TokenSink for StringSink {
    fn on_token(&mut self, delta: &str) -> Flow {
        self.text.push_str(delta);
        Flow::Continue
    }
}

/// Puits « boîte aux lettres » : chaque fragment part dans un canal. C'est
/// la forme qu'aura le port en flux du dataflow (étape 3) — un récepteur
/// fermé annule proprement la génération.
pub struct ChannelSink(pub std::sync::mpsc::SyncSender<String>);

impl TokenSink for ChannelSink {
    fn on_token(&mut self, delta: &str) -> Flow {
        if self.0.send(delta.to_string()).is_ok() {
            Flow::Continue
        } else {
            Flow::Stop
        }
    }
}

/// Puits qui compte, et qui sait s'arrêter au bout de `stop_after`
/// fragments : c'est lui qui prouve, en test, que [`Flow::Stop`] remonte
/// bien jusqu'au générateur.
#[derive(Debug, Clone, Default)]
pub struct CountingSink {
    pub tokens: usize,
    pub chars: usize,
    /// `None` = ne s'arrête jamais de lui-même.
    pub stop_after: Option<usize>,
    /// Renseigné par [`TokenSink::on_finish`].
    pub finished: Option<Finish>,
}

impl CountingSink {
    /// Un puits qui annule après `n` fragments.
    pub fn stopping_after(n: usize) -> Self {
        Self { stop_after: Some(n), ..Default::default() }
    }
}

impl TokenSink for CountingSink {
    fn on_token(&mut self, delta: &str) -> Flow {
        self.tokens += 1;
        self.chars += delta.chars().count();
        match self.stop_after {
            Some(n) if self.tokens >= n => Flow::Stop,
            _ => Flow::Continue,
        }
    }
    fn on_finish(&mut self, reason: &Finish) {
        self.finished = Some(reason.clone());
    }
}

// ─── Conversation et options ─────────────────────────────────────────────────

/// Un tour de conversation. `role` est une chaîne et pas un enum : c'est
/// exactement ce que les chat templates itèrent (`system`, `user`,
/// `assistant`, et `tool` à l'étape 6), et un modèle inconnu peut en
/// inventer un sans qu'on ait à recompiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    pub role: String,
    pub content: String,
}

impl Turn {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self { role: role.into(), content: content.into() }
    }
    pub fn system(content: impl Into<String>) -> Self {
        Self::new("system", content)
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self::new("user", content)
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new("assistant", content)
    }
}

/// Réglages d'un appel. `tools` porte les [`ToolDef`] *typées* — on ne
/// repasse pas par une chaîne JSON : c'est le chat template (étape 6) qui
/// les sérialisera, et la même donnée servira à compiler la grammaire.
#[derive(Debug, Clone, PartialEq)]
pub struct GenOptions {
    /// Plafond de fragments générés (0 = aucun fragment).
    pub max_tokens: usize,
    /// `0.0` = glouton, donc déterministe — le défaut, parce qu'un LLM
    /// dans un pipeline RAG doit être reproductible avant d'être créatif.
    pub temperature: f32,
    pub top_p: f32,
    /// Séquences qui terminent la génération. La séquence elle-même n'est
    /// pas émise ; ce qui la précède l'est **verbatim**, espaces compris —
    /// rogner serait une surprise, et corromprait un bloc de code qui se
    /// termine par un retour à la ligne avant le `stop`.
    pub stop: Vec<String>,
    /// Outils exposés au modèle, en général `crate::tools::tool_defs()`.
    pub tools: Vec<ToolDef>,
}

impl Default for GenOptions {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            temperature: 0.0,
            top_p: 1.0,
            stop: Vec::new(),
            tools: Vec::new(),
        }
    }
}

impl GenOptions {
    pub fn with_max_tokens(mut self, n: usize) -> Self {
        self.max_tokens = n;
        self
    }
    pub fn with_temperature(mut self, t: f32) -> Self {
        self.temperature = t;
        self
    }
    pub fn with_top_p(mut self, p: f32) -> Self {
        self.top_p = p;
        self
    }
    pub fn with_stop(mut self, stop: Vec<String>) -> Self {
        self.stop = stop;
        self
    }
    pub fn with_tools(mut self, tools: Vec<ToolDef>) -> Self {
        self.tools = tools;
        self
    }
}

/// Comptage d'un appel. `prompt_tokens` est ce qu'a coûté le préremplissage,
/// `completion_tokens` le nombre de fragments émis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub ms: u64,
}

impl Usage {
    /// Débit de génération, `0.0` si rien n'a été mesuré.
    pub fn tokens_per_s(&self) -> f64 {
        if self.ms == 0 {
            return 0.0;
        }
        self.completion_tokens as f64 * 1000.0 / self.ms as f64
    }
}

/// Ce qu'un appel produit, une fois le flux terminé — l'équivalent de
/// [`crate::ocr::OcrOutput`] pour la génération.
#[derive(Debug, Clone, PartialEq)]
pub struct LlmOutput {
    pub text: String,
    pub finish: Finish,
    pub usage: Usage,
}

// ─── Erreurs ─────────────────────────────────────────────────────────────────

/// Erreurs de génération. Seul le message est contractuel.
#[derive(Debug, Clone, PartialEq)]
pub enum LlmError {
    /// Le modèle a échoué (chargement, inférence, décodage).
    Model(String),
    /// La conversation est inutilisable (vide, rôle inconnu, template).
    Prompt(String),
    /// Le prompt dépasse la fenêtre de contexte.
    ContextOverflow { max: usize, got: usize },
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::Model(m) => write!(f, "llm model error: {m}"),
            LlmError::Prompt(m) => write!(f, "llm prompt error: {m}"),
            LlmError::ContextOverflow { max, got } => {
                write!(f, "llm context overflow: {got} tokens, max {max}")
            }
        }
    }
}

impl std::error::Error for LlmError {}

// ─── Le trait ────────────────────────────────────────────────────────────────

/// Générateur de texte. Synchrone et `&self` : appelable depuis
/// `Node::execute`. Les fragments sortent par `sink` **pendant** l'appel ;
/// `generate` ne rend la main qu'à la fin.
pub trait Llm: Send + Sync {
    /// Génère à partir de `turns`. L'implémentation **doit** appeler
    /// [`TokenSink::on_finish`] exactement une fois avant de rendre `Ok`,
    /// et **doit** honorer [`Flow::Stop`] en s'arrêtant immédiatement.
    fn generate(
        &self,
        turns: &[Turn],
        opts: &GenOptions,
        sink: &mut dyn TokenSink,
    ) -> Result<(Finish, Usage), LlmError>;

    /// Fenêtre de contexte utilisable, en jetons.
    fn context_len(&self) -> usize;

    /// Nom lisible (modèle), pour les diagnostics.
    fn name(&self) -> &str {
        "llm"
    }
}

/// Indispensable : le `ServiceRegistry` stocke et redonne le type concret
/// `Arc<dyn Llm>`, donc `Arc<dyn Llm>` doit lui-même être un [`Llm`].
impl<T: Llm + ?Sized> Llm for Arc<T> {
    fn generate(
        &self,
        turns: &[Turn],
        opts: &GenOptions,
        sink: &mut dyn TokenSink,
    ) -> Result<(Finish, Usage), LlmError> {
        (**self).generate(turns, opts, sink)
    }
    fn context_len(&self) -> usize {
        (**self).context_len()
    }
    fn name(&self) -> &str {
        (**self).name()
    }
}

/// Confort : génère et rend le texte entier, sans avoir à câbler un puits.
pub fn generate_to_string(
    llm: &dyn Llm,
    turns: &[Turn],
    opts: &GenOptions,
) -> Result<LlmOutput, LlmError> {
    let mut sink = StringSink::default();
    let (finish, usage) = llm.generate(turns, opts, &mut sink)?;
    Ok(LlmOutput { text: sink.text, finish, usage })
}

// ─── Découpage en fragments ──────────────────────────────────────────────────

/// Découpe un texte comme un tokeniseur BPE le rendrait : l'espace part
/// **avec** le mot qui suit (`"a b"` → `["a", " b"]`), si bien que
/// concaténer les fragments redonne exactement le texte d'origine.
pub fn fragments(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        if ch.is_whitespace() && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        cur.push(ch);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Cherche la première séquence de `stops` dans `acc + frag`. Rend le
/// nombre d'octets de `frag` à conserver et la séquence trouvée.
fn stop_hit(acc: &str, frag: &str, stops: &[String]) -> Option<(usize, String)> {
    let combined = format!("{acc}{frag}");
    let mut best: Option<(usize, &String)> = None;
    for s in stops.iter().filter(|s| !s.is_empty()) {
        if let Some(pos) = combined.find(s.as_str()) {
            if best.is_none_or(|(p, _)| pos < p) {
                best = Some((pos, s));
            }
        }
    }
    let (pos, seq) = best?;
    Some((pos.saturating_sub(acc.len()).min(frag.len()), seq.clone()))
}

// ─── Mock ────────────────────────────────────────────────────────────────────

/// LLM de test : rend un texte fixé d'avance, découpé en fragments, sans
/// regarder la conversation (mais il vérifie qu'elle n'est pas vide).
/// Déterministe par construction — c'est ce qui permet de tester le nœud,
/// l'annulation, `max_tokens` et `stop` sans modèle.
#[derive(Debug, Clone)]
pub struct MockLlm {
    /// Texte rendu, avant application de `max_tokens` et `stop`.
    pub reply: String,
    /// Fenêtre annoncée par [`Llm::context_len`].
    pub context_len: usize,
}

impl Default for MockLlm {
    fn default() -> Self {
        Self::new("Bonjour, je suis un modèle de test.")
    }
}

impl MockLlm {
    pub fn new(reply: impl Into<String>) -> Self {
        Self { reply: reply.into(), context_len: 4096 }
    }

    pub fn with_context_len(mut self, n: usize) -> Self {
        self.context_len = n;
        self
    }
}

impl Llm for MockLlm {
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
        let prompt_tokens: usize = turns.iter().map(|t| fragments(&t.content).len()).sum();
        if prompt_tokens > self.context_len {
            return Err(LlmError::ContextOverflow { max: self.context_len, got: prompt_tokens });
        }

        let started = Instant::now();
        let mut acc = String::new();
        let mut emitted = 0usize;
        let mut finish = Finish::Eos;

        for frag in fragments(&self.reply) {
            if emitted >= opts.max_tokens {
                finish = Finish::MaxTokens;
                break;
            }
            if let Some((keep, seq)) = stop_hit(&acc, &frag, &opts.stop) {
                if keep > 0 {
                    emitted += 1;
                    let head = &frag[..keep];
                    acc.push_str(head);
                    if sink.on_token(head) == Flow::Stop {
                        finish = Finish::Cancelled;
                        break;
                    }
                }
                finish = Finish::Stop(seq);
                break;
            }
            acc.push_str(&frag);
            emitted += 1;
            if sink.on_token(&frag) == Flow::Stop {
                finish = Finish::Cancelled;
                break;
            }
        }

        sink.on_finish(&finish);
        Ok((
            finish,
            Usage {
                prompt_tokens,
                completion_tokens: emitted,
                ms: started.elapsed().as_millis() as u64,
            },
        ))
    }

    fn context_len(&self) -> usize {
        self.context_len
    }

    fn name(&self) -> &str {
        "mock-llm"
    }
}

// ─── Callback ────────────────────────────────────────────────────────────────

/// LLM par fermeture (hôtes non-Rust, tests) — pendant de
/// [`crate::reranker::CallbackReranker`].
pub struct CallbackLlm {
    name: String,
    context_len: usize,
    #[allow(clippy::type_complexity)]
    f: Box<
        dyn Fn(&[Turn], &GenOptions, &mut dyn TokenSink) -> Result<(Finish, Usage), LlmError>
            + Send
            + Sync,
    >,
}

impl CallbackLlm {
    pub fn new(
        name: impl Into<String>,
        context_len: usize,
        f: impl Fn(&[Turn], &GenOptions, &mut dyn TokenSink) -> Result<(Finish, Usage), LlmError>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self { name: name.into(), context_len, f: Box::new(f) }
    }
}

impl Llm for CallbackLlm {
    fn generate(
        &self,
        turns: &[Turn],
        opts: &GenOptions,
        sink: &mut dyn TokenSink,
    ) -> Result<(Finish, Usage), LlmError> {
        (self.f)(turns, opts, sink)
    }
    fn context_len(&self) -> usize {
        self.context_len
    }
    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello() -> Vec<Turn> {
        vec![Turn::system("tu es utile"), Turn::user("bonjour")]
    }

    #[test]
    fn fragments_roundtrip_and_carry_leading_space() {
        assert_eq!(fragments("Bonjour le monde"), ["Bonjour", " le", " monde"]);
        assert_eq!(fragments(""), Vec::<String>::new());
        for text in ["a", "  a b ", "un\ndeux\ttrois", "élan émoji 🦀 fin"] {
            assert_eq!(fragments(text).concat(), text, "perte sur {text:?}");
        }
    }

    #[test]
    fn mock_streams_the_whole_reply() {
        let llm = MockLlm::new("Bonjour le monde");
        let mut sink = StringSink::default();
        let (finish, usage) = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();
        assert_eq!(sink.text, "Bonjour le monde");
        assert_eq!(finish, Finish::Eos);
        assert!(finish.is_complete());
        assert_eq!(usage.completion_tokens, 3);
        assert_eq!(usage.prompt_tokens, 4); // "tu es utile" (3) + "bonjour" (1)
        assert_eq!(llm.name(), "mock-llm");
        assert_eq!(llm.context_len(), 4096);
    }

    #[test]
    fn max_tokens_truncates() {
        let llm = MockLlm::new("un deux trois quatre");
        let mut sink = StringSink::default();
        let opts = GenOptions::default().with_max_tokens(2);
        let (finish, usage) = llm.generate(&hello(), &opts, &mut sink).unwrap();
        assert_eq!(sink.text, "un deux");
        assert_eq!(finish, Finish::MaxTokens);
        assert!(!finish.is_complete(), "tronqué par notre plafond, pas fini");
        assert_eq!(usage.completion_tokens, 2);

        // Cas limite : aucun fragment autorisé.
        let mut sink = StringSink::default();
        let opts = GenOptions::default().with_max_tokens(0);
        let (finish, _) = llm.generate(&hello(), &opts, &mut sink).unwrap();
        assert_eq!(sink.text, "");
        assert_eq!(finish, Finish::MaxTokens);
    }

    #[test]
    fn stop_sequence_cuts_and_is_not_emitted() {
        let llm = MockLlm::new("réponse ici FIN et la suite");
        let mut sink = StringSink::default();
        let opts = GenOptions::default().with_stop(vec!["FIN".into()]);
        let (finish, _) = llm.generate(&hello(), &opts, &mut sink).unwrap();
        // Préfixe verbatim : l'espace qui précède "FIN" est conservé.
        assert_eq!(sink.text, "réponse ici ");
        assert_eq!(finish, Finish::Stop("FIN".into()));
        assert!(finish.is_complete(), "l'appelant a demandé ce stop");

        // La séquence la plus précoce gagne, même déclarée en second.
        let mut sink = StringSink::default();
        let opts = GenOptions::default().with_stop(vec!["suite".into(), "ici".into()]);
        let (finish, _) = llm.generate(&hello(), &opts, &mut sink).unwrap();
        assert_eq!(finish, Finish::Stop("ici".into()));
        assert_eq!(sink.text, "réponse ");

        // Un stop vide est ignoré (sinon il couperait à l'octet 0).
        let mut sink = StringSink::default();
        let opts = GenOptions::default().with_stop(vec![String::new()]);
        let (finish, _) = llm.generate(&hello(), &opts, &mut sink).unwrap();
        assert_eq!(finish, Finish::Eos);
        assert_eq!(sink.text, "réponse ici FIN et la suite");
    }

    #[test]
    fn stop_cutting_mid_fragment_keeps_the_head() {
        // "FIN" est collé au mot : le fragment " iciFIN" doit être coupé.
        let llm = MockLlm::new("réponse iciFIN et la suite");
        let mut sink = StringSink::default();
        let opts = GenOptions::default().with_stop(vec!["FIN".into()]);
        let (finish, usage) = llm.generate(&hello(), &opts, &mut sink).unwrap();
        assert_eq!(sink.text, "réponse ici");
        assert_eq!(finish, Finish::Stop("FIN".into()));
        assert_eq!(usage.completion_tokens, 2);
    }

    #[test]
    fn sink_stop_cancels_the_generation() {
        let llm = MockLlm::new("un deux trois quatre cinq");
        let mut sink = CountingSink::stopping_after(2);
        let (finish, usage) = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();
        assert_eq!(finish, Finish::Cancelled);
        assert!(!finish.is_complete(), "annulé : la réponse est incomplète");
        assert_eq!(sink.tokens, 2, "le générateur s'arrête net, il n'en pousse pas un de plus");
        assert_eq!(usage.completion_tokens, 2);
        assert_eq!(sink.finished, Some(Finish::Cancelled), "on_finish est appelé même annulé");
    }

    #[test]
    fn channel_sink_streams_then_stops_when_receiver_drops() {
        let (tx, rx) = std::sync::mpsc::sync_channel::<String>(64);
        let llm = MockLlm::new("un deux trois");
        let mut sink = ChannelSink(tx);
        llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();
        drop(sink);
        assert_eq!(rx.iter().collect::<Vec<_>>(), ["un", " deux", " trois"]);

        // Récepteur fermé : le puits annule.
        let (tx, rx) = std::sync::mpsc::sync_channel::<String>(64);
        drop(rx);
        let mut sink = ChannelSink(tx);
        let (finish, usage) = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();
        assert_eq!(finish, Finish::Cancelled);
        assert_eq!(usage.completion_tokens, 1, "un seul essai avant de voir le canal fermé");
    }

    #[test]
    fn empty_or_malformed_conversation_is_an_error() {
        let llm = MockLlm::new("x");
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
    fn context_overflow_is_reported() {
        let llm = MockLlm::new("x").with_context_len(2);
        let mut sink = StringSink::default();
        let turns = vec![Turn::user("un deux trois quatre")];
        let err = llm.generate(&turns, &GenOptions::default(), &mut sink).unwrap_err();
        assert_eq!(err, LlmError::ContextOverflow { max: 2, got: 4 });
        assert!(err.to_string().contains("context overflow"), "{err}");
    }

    #[test]
    fn errors_display() {
        assert!(LlmError::Model("boom".into()).to_string().contains("boom"));
        assert!(LlmError::Prompt("bad".into()).to_string().contains("bad"));
    }

    #[test]
    fn generate_to_string_helper() {
        let llm = MockLlm::new("un deux");
        let out = generate_to_string(&llm, &hello(), &GenOptions::default()).unwrap();
        assert_eq!(out.text, "un deux");
        assert_eq!(out.finish, Finish::Eos);
        assert_eq!(out.usage.completion_tokens, 2);
    }

    #[test]
    fn arc_dyn_llm_is_itself_an_llm() {
        // C'est ce que `ctx.service::<Arc<dyn Llm>>("llm")` exige.
        let llm: Arc<dyn Llm> = Arc::new(MockLlm::new("ok"));
        fn takes_llm<L: Llm>(l: &L) -> usize {
            l.context_len()
        }
        assert_eq!(takes_llm(&llm), 4096);
        assert_eq!(llm.name(), "mock-llm");
        let mut sink = StringSink::default();
        llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();
        assert_eq!(sink.text, "ok");
    }

    #[test]
    fn callback_llm_forwards_everything() {
        let llm = CallbackLlm::new("cb", 128, |turns, opts, sink| {
            let text = format!("{} tours, max {}", turns.len(), opts.max_tokens);
            for f in fragments(&text) {
                if sink.on_token(&f) == Flow::Stop {
                    return Ok((Finish::Cancelled, Usage::default()));
                }
            }
            sink.on_finish(&Finish::Eos);
            Ok((Finish::Eos, Usage { prompt_tokens: 1, completion_tokens: 4, ms: 0 }))
        });
        let mut sink = StringSink::default();
        let opts = GenOptions::default().with_max_tokens(7);
        let (finish, usage) = llm.generate(&hello(), &opts, &mut sink).unwrap();
        assert_eq!(sink.text, "2 tours, max 7");
        assert_eq!(finish, Finish::Eos);
        assert_eq!(usage.completion_tokens, 4);
        assert_eq!(llm.name(), "cb");
        assert_eq!(llm.context_len(), 128);
    }

    #[test]
    fn usage_tokens_per_s() {
        assert_eq!(Usage::default().tokens_per_s(), 0.0);
        let u = Usage { prompt_tokens: 0, completion_tokens: 50, ms: 1000 };
        assert!((u.tokens_per_s() - 50.0).abs() < 1e-9);
    }

    #[test]
    fn gen_options_defaults_are_deterministic() {
        let o = GenOptions::default();
        assert_eq!(o.temperature, 0.0, "glouton par défaut : reproductible");
        assert_eq!(o.top_p, 1.0);
        assert_eq!(o.max_tokens, 512);
        assert!(o.stop.is_empty() && o.tools.is_empty());
    }

    #[test]
    fn turn_constructors() {
        assert_eq!(Turn::system("s").role, "system");
        assert_eq!(Turn::user("u").role, "user");
        assert_eq!(Turn::assistant("a").role, "assistant");
        assert_eq!(Turn::new("tool", "t").content, "t");
    }
}
