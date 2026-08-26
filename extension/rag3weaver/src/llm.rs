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

/// Un appel d'outil annoncé par le modèle.
///
/// `id` est **l'identité de l'appel pour le fournisseur** : OpenAI et Vertex
/// exigent qu'un message de résultat le reprenne mot pour mot, et refusent la
/// requête (400) si un appel reste sans réponse. C'est pourquoi ce type
/// remplace la chaîne JSON d'avant : on accumulait la structure pour la jeter
/// à la frontière, et un `id` perdu rend la conversation **irrejouable**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    /// Identifiant opaque. Vient du fournisseur (`call_…`), ou de
    /// [`ToolCall::local_id`] pour un modèle local qui n'en a pas.
    pub id: String,
    /// Nom de l'outil, c'est-à-dire le `node_type` d'un [`crate::tools::ToolDef`].
    pub name: String,
    /// Arguments **bruts**, tels que le modèle les a émis : une chaîne qui
    /// contient du JSON. On ne la parse pas ici — un appel tronqué par
    /// `max_tokens` produit du JSON invalide, et il faut quand même pouvoir
    /// le refermer.
    pub arguments: String,
    /// Données que le fournisseur attache à **cet appel** et qu'il exige de
    /// revoir, à l'identique, au tour suivant. **Opaque** : `llm.rs` ne sait
    /// pas ce qu'il y a dedans, ne le lit pas, ne le valide pas — il le
    /// transporte.
    ///
    /// C'est ce qui porte le `thought_signature` de Gemini 3.x. Un champ
    /// `thought_signature: Option<String>` aurait fait entrer le vocabulaire
    /// d'un fournisseur dans le type générique que voient le modèle local et
    /// les 28 nœuds ; ici, seul [`crate::openai_llm`] sait le remplir et le
    /// relire, et le prochain fournisseur qui inventera son propre jeton de
    /// continuité n'imposera pas un champ de plus.
    ///
    /// `None` sur le chemin local (`MockLlm`, burn) : il n'y a rien à rejouer,
    /// et **rien de superflu ne doit être sérialisé**.
    pub provider_extra: Option<serde_json::Value>,
}

impl ToolCall {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments: arguments.into(),
            provider_extra: None,
        }
    }

    /// Attache les données opaques du fournisseur. Voir [`Self::provider_extra`].
    pub fn with_provider_extra(mut self, extra: serde_json::Value) -> Self {
        self.provider_extra = Some(extra);
        self
    }

    /// Identifiant pour un modèle **local**, qui n'en reçoit pas du
    /// fournisseur (`MockLlm`, et le modèle burn à venir).
    ///
    /// C'est un condensat blake3 de ce qui identifie l'appel dans sa
    /// conversation : le contexte (typiquement les tours déjà joués), le rang
    /// de l'appel dans le tour, le nom de l'outil et ses arguments. Deux
    /// propriétés, et ce sont les deux qu'on veut :
    ///
    /// - **déterministe** — rejouer la même conversation regénère exactement
    ///   les mêmes `id`, ce qui est précisément l'invariant à tenir (et ce qui
    ///   rend les tests reproductibles, sans horloge ni compteur global) ;
    /// - **sans collision en pratique** — deux appels différents, même dans un
    ///   long historique, ne partagent pas d'identifiant, donc l'appariement
    ///   par `id` reste sans ambiguïté.
    ///
    /// Le préfixe `call_` suit la convention d'OpenAI ; aucun fournisseur
    /// n'impose de format, mais s'en écarter n'apporterait rien.
    pub fn local_id(context: &str, index: usize, name: &str, arguments: &str) -> String {
        let mut h = blake3::Hasher::new();
        h.update(context.as_bytes());
        h.update(&(index as u64).to_le_bytes());
        h.update(name.as_bytes());
        h.update(arguments.as_bytes());
        format!("call_local_{}", &h.finalize().to_hex()[..16])
    }

    /// Le même, mais qui construit l'appel complet.
    pub fn local(context: &str, index: usize, name: &str, arguments: &str) -> Self {
        // Pas de `provider_extra` : un modèle local n'a rien à rejouer.
        Self::new(Self::local_id(context, index, name, arguments), name, arguments)
    }
}

/// Pourquoi la génération s'est terminée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishReason {
    /// Le modèle a émis son jeton de fin : réponse complète.
    Eos,
    /// `max_tokens` atteint : réponse tronquée par notre plafond.
    MaxTokens,
    /// Une séquence de `stop` est apparue (elle est donnée) : réponse
    /// complète du point de vue de l'appelant, qui l'avait demandée.
    Stop(String),
    /// Le puits a répondu [`Flow::Stop`] : réponse **incomplète**. Distinct
    /// de [`FinishReason::Stop`] parce qu'une interface doit savoir si elle
    /// affiche une réponse finie ou un fragment abandonné.
    Cancelled,
    /// Le modèle demande un ou plusieurs outils.
    ToolCall,
}

/// Comment la génération s'est terminée, **et** ce que le modèle avait déjà
/// annoncé à cet instant.
///
/// `tool_calls` est un champ et non le contenu d'une variante : c'est ce qui
/// fait tenir l'invariant *par le type*. Un appel annoncé puis interrompu
/// (`Cancelled`) ou tronqué (`MaxTokens`) doit rester récupérable — sinon son
/// `id` est perdu, l'appel reste orphelin, et la requête suivante part en 400.
/// Mettre le vecteur dans les seules variantes « concernées » laisserait la
/// prochaine variante ajoutée le réintroduire, ce bogue-là.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finish {
    pub reason: FinishReason,
    /// Appels annoncés par le modèle, **quelle que soit** `reason`. Dans
    /// l'ordre où le modèle les a annoncés.
    pub tool_calls: Vec<ToolCall>,
}

impl Finish {
    pub fn new(reason: FinishReason, tool_calls: Vec<ToolCall>) -> Self {
        Self { reason, tool_calls }
    }
    pub fn eos() -> Self {
        Self::new(FinishReason::Eos, Vec::new())
    }
    pub fn max_tokens() -> Self {
        Self::new(FinishReason::MaxTokens, Vec::new())
    }
    pub fn stop(seq: impl Into<String>) -> Self {
        Self::new(FinishReason::Stop(seq.into()), Vec::new())
    }
    pub fn cancelled() -> Self {
        Self::new(FinishReason::Cancelled, Vec::new())
    }
    /// Fin normale sur demande d'outils.
    pub fn tool_call(calls: Vec<ToolCall>) -> Self {
        Self::new(FinishReason::ToolCall, calls)
    }
    /// Attache des appels à une fin qui n'en portait pas — c'est par là que
    /// `Cancelled` et `MaxTokens` gardent ce que le modèle avait annoncé.
    pub fn with_tool_calls(mut self, calls: Vec<ToolCall>) -> Self {
        self.tool_calls = calls;
        self
    }

    /// Vrai si la réponse est exploitable telle quelle (rien ne manque du
    /// point de vue de l'appelant).
    pub fn is_complete(&self) -> bool {
        matches!(
            self.reason,
            FinishReason::Eos | FinishReason::Stop(_) | FinishReason::ToolCall
        )
    }

    /// Vrai s'il reste des appels à refermer — vrai même après une annulation.
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
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

    /// Appelé **avant** puis **pendant** chaque attente de réessai. Rendre
    /// [`Flow::Stop`] interrompt l'attente et abandonne l'appel.
    ///
    /// C'est le seul point d'annulation d'une attente : le reste du contrat
    /// suppose qu'un appel pousse des jetons, or ici il n'en pousse aucun
    /// pendant des dizaines de secondes. Le défaut — continuer sans rien
    /// faire — préserve tous les puits existants.
    ///
    /// Voir [`RetryPhase`] : un puits qui journalise ne réagit qu'à
    /// [`RetryPhase::Scheduled`] ; [`RetryPhase::Waiting`] revient plusieurs
    /// fois par seconde et sert à rendre l'attente interruptible.
    fn on_retry(&mut self, _event: &RetryEvent<'_>) -> Flow {
        Flow::Continue
    }
}

/// À quel moment de l'attente le puits est appelé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryPhase {
    /// Le réessai vient d'être décidé. `wait` est l'attente **complète**.
    /// Appelé **une seule fois** par tentative : c'est là qu'on journalise.
    Scheduled,
    /// Pendant l'attente, plusieurs fois par seconde. `wait` est ce qu'il
    /// **reste**. Ne pas journaliser ici, sauf à vouloir des centaines de
    /// lignes ; c'est le point où une annulation devient visible.
    Waiting,
}

/// Ce que le puits apprend d'un réessai. Aucune allocation : il est construit
/// à chaque tranche d'attente.
#[derive(Debug, Clone)]
pub struct RetryEvent<'a> {
    pub phase: RetryPhase,
    /// Numéro du réessai à venir : 1 pour le premier.
    pub attempt: u32,
    /// Plafond de tentatives, celle d'origine comprise.
    pub max_attempts: u32,
    /// Attente complète ([`RetryPhase::Scheduled`]) ou restante
    /// ([`RetryPhase::Waiting`]).
    pub wait: std::time::Duration,
    /// Temps écoulé depuis le début de l'appel, réessais compris.
    pub elapsed: std::time::Duration,
    /// Pourquoi on réessaie — `"HTTP 429"`, `"HTTP 503"`, un message de
    /// transport. Destiné à l'humain, jamais analysé.
    pub reason: &'a str,
    /// Vrai si l'attente vient d'un `Retry-After` du fournisseur plutôt que
    /// de notre calcul. Utile pour comprendre une attente longue.
    pub from_server: bool,
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Turn {
    pub role: String,
    pub content: String,
    /// Pour un tour `assistant` : les appels d'outils qu'il annonce. Vide
    /// partout ailleurs. C'est la moitié manquante d'une conversation avec
    /// outils — sans elle, un historique ne peut pas être rejoué.
    pub tool_calls: Vec<ToolCall>,
    /// Pour un tour `tool` : l'appel auquel ce résultat répond. C'est ce que
    /// le fournisseur apparie avec les `tool_calls` du tour d'assistant.
    pub tool_call_id: Option<String>,
    /// Pour un tour `tool` : le nom de l'outil. Facultatif chez OpenAI, mais
    /// certains fournisseurs le lisent, et il rend un historique lisible.
    pub tool_name: Option<String>,
}

impl Turn {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self { role: role.into(), content: content.into(), ..Default::default() }
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

    /// Tour d'assistant qui **annonce des appels d'outils**. `content` est
    /// souvent vide : un modèle qui appelle un outil ne dit en général rien.
    pub fn assistant_with_calls(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self { tool_calls, ..Self::new("assistant", content) }
    }

    /// Tour qui porte le **résultat** d'un appel. `id` doit être exactement
    /// celui annoncé par le modèle.
    pub fn tool_result(
        id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let name = name.into();
        Self {
            tool_call_id: Some(id.into()),
            tool_name: if name.is_empty() { None } else { Some(name) },
            ..Self::new("tool", content)
        }
    }

    /// Vrai si ce tour est un résultat d'outil.
    pub fn is_tool_result(&self) -> bool {
        self.tool_call_id.is_some()
    }
}

/// Signature factice que Google documente pour les historiques qui n'en ont
/// pas — trace importée d'un autre modèle, appels fabriqués côté client, ou
/// conversation démarrée sur notre modèle local puis reprise sur Gemini 3.x.
///
/// **Dernier recours, jamais automatique.** Google prévient : « it will
/// negatively impact model performance ». On l'expose parce que le cas
/// hybride est réel, pas parce qu'il est recommandé ; c'est à l'appelant de
/// décider, avec [`ToolCall::with_provider_extra`].
pub const SKIP_THOUGHT_SIGNATURE_VALIDATOR: &str = "skip_thought_signature_validator";

/// Contenu posé dans un résultat d'outil fabriqué pour refermer un appel qui
/// n'a jamais été exécuté. Une chaîne JSON : l'agent la relit sans casser sur
/// du texte libre, et le modèle comprend que l'outil n'a pas tourné.
pub const INTERRUPTED_TOOL_RESULT: &str = r#"{"error":"interrupted","detail":"l'appel n'a pas été exécuté"}"#;

/// Les appels d'outils d'un historique qui n'ont **pas** de résultat.
///
/// C'est le garde-fou qui évite le 400.
///
/// État réel de ce qu'on sait, sans embellir :
///
/// - **OpenAI** rejette en 400 : *"An assistant message with 'tool_calls' must
///   be followed by tool messages responding to each 'tool_call_id'. The
///   following tool_call_ids did not have response messages: …"*. Attention :
///   c'est une **validation serveur observée et reproduite**, elle n'est
///   documentée nulle part — ni dans le guide, ni dans le schéma OpenAPI (où
///   `content` et `tool_calls` sont de simples champs sans contrainte
///   croisée). Ne pas s'attendre à la trouver dans la doc.
/// - **Google ne documente rien** sur ce point, ni pour Vertex ni pour AI
///   Studio. Mais l'API Gemini **native**, vers laquelle la couche de
///   compatibilité traduit, rejette tout déséquilibre : *"Please ensure that
///   the number of function response parts is equal to the number of function
///   call parts"*. Il faut donc tabler sur **au moins autant de sévérité que
///   chez OpenAI**, avec un message d'erreur de style Google.
///
/// Rend les appels dans leur ordre d'apparition.
pub fn orphan_tool_calls(turns: &[Turn]) -> Vec<&ToolCall> {
    let answered: std::collections::HashSet<&str> = turns
        .iter()
        .filter_map(|t| t.tool_call_id.as_deref())
        .collect();
    turns
        .iter()
        .flat_map(|t| t.tool_calls.iter())
        .filter(|c| !answered.contains(c.id.as_str()))
        .collect()
}

/// L'inverse : les résultats qui ne répondent à aucun appel annoncé.
///
/// OpenAI les refuse aussi, en 400 : *"Invalid parameter: messages with role
/// 'tool' must be a response to a preceeding message with 'tool_calls'"* (la
/// faute d'orthographe est dans le message du serveur ; observée, pas
/// documentée). C'est le symptôme d'un historique tronqué par le début —
/// typiquement une fenêtre glissante qui a coupé le tour d'assistant en
/// gardant ses résultats.
pub fn dangling_tool_results(turns: &[Turn]) -> Vec<&str> {
    let announced: std::collections::HashSet<&str> = turns
        .iter()
        .flat_map(|t| t.tool_calls.iter())
        .map(|c| c.id.as_str())
        .collect();
    turns
        .iter()
        .filter_map(|t| t.tool_call_id.as_deref())
        .filter(|id| !announced.contains(id))
        .collect()
}

/// Referme tous les appels orphelins en insérant un résultat par appel
/// manquant. Rend le nombre d'appels comblés.
///
/// À appeler avant d'envoyer un historique repris après une interruption :
/// c'est ce qui transforme une conversation malformée en conversation
/// rejouable.
///
/// Chaque résultat est inséré **juste après le tour d'assistant qui l'a
/// annoncé** (à la suite des résultats déjà présents pour ce tour), et non à
/// la fin de l'historique. Ce n'est pas de l'esthétique : le bloc de messages
/// `tool` doit suivre *immédiatement* son message assistant, sans qu'aucun
/// message `user`, `system` ou `assistant` ne s'intercale — insérer à la fin
/// casserait tout historique où l'utilisateur a repris la parole après
/// l'interruption, ce qui est exactement le scénario visé.
pub fn close_orphan_tool_calls(turns: &mut Vec<Turn>, content: &str) -> usize {
    let answered: std::collections::HashSet<String> =
        turns.iter().filter_map(|t| t.tool_call_id.clone()).collect();
    let mut inserted = 0usize;
    // À rebours : une insertion ne décale alors aucun index restant à traiter.
    for i in (0..turns.len()).rev() {
        if turns[i].tool_calls.is_empty() {
            continue;
        }
        let missing: Vec<(String, String)> = turns[i]
            .tool_calls
            .iter()
            .filter(|c| !answered.contains(&c.id))
            .map(|c| (c.id.clone(), c.name.clone()))
            .collect();
        if missing.is_empty() {
            continue;
        }
        // Fin du bloc de résultats qui suit déjà ce tour d'assistant.
        let mut at = i + 1;
        while at < turns.len() && turns[at].is_tool_result() {
            at += 1;
        }
        for (k, (id, name)) in missing.into_iter().enumerate() {
            turns.insert(at + k, Turn::tool_result(id, name, content));
            inserted += 1;
        }
    }
    inserted
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
    /// Comment le modèle choisit ses outils. Sans effet s'il n'y a pas
    /// d'outils : le champ n'est alors pas envoyé du tout.
    pub tool_choice: ToolChoice,
    /// Forme imposée à la sortie. `None` = on n'envoie rien, donc le modèle
    /// répond en texte libre comme aujourd'hui.
    pub response_format: Option<ResponseFormat>,
    /// Combien le modèle a le droit de « réfléchir » avant de répondre.
    /// `None` = on n'envoie rien, donc rien ne change pour un fournisseur qui
    /// ne connaît pas ce réglage. Voir [`ReasoningEffort`] : ce n'est pas un
    /// réglage cosmétique, c'est **le** levier de coût et de latence.
    pub reasoning: Option<ReasoningEffort>,
}

impl Default for GenOptions {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            temperature: 0.0,
            top_p: 1.0,
            stop: Vec::new(),
            tools: Vec::new(),
            tool_choice: ToolChoice::Auto,
            response_format: None,
            reasoning: None,
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
    /// Impose la façon dont le modèle choisit ses outils.
    pub fn with_tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = choice;
        self
    }
    /// Impose une forme à la sortie. Voir [`ResponseFormat`].
    pub fn with_response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }
    /// Borne la réflexion du modèle. Voir [`ReasoningEffort`] pour les mesures.
    pub fn with_reasoning(mut self, effort: ReasoningEffort) -> Self {
        self.reasoning = Some(effort);
        self
    }
    /// Ne rien envoyer : le fournisseur décide (et, sur Gemini 3.x, il décide
    /// de réfléchir jusqu'à saturer `max_tokens`).
    pub fn without_reasoning(mut self) -> Self {
        self.reasoning = None;
        self
    }
}

/// Comment le modèle choisit ses outils, sérialisé en `tool_choice`.
///
/// Type fermé, comme [`ReasoningEffort`] : une faute de frappe doit être une
/// erreur de compilation, pas un 400.
///
/// ⚠ **Le piège du `finish_reason`.** Avec [`ToolChoice::Auto`] et
/// [`ToolChoice::Required`], le fournisseur rend `finish_reason: "tool_calls"`.
/// Avec [`ToolChoice::Function`], il rend **`"stop"`** — le même code que pour
/// une fin de texte ordinaire. Un client qui déciderait « appel d'outil ou
/// pas » d'après `finish_reason` casserait donc précisément sur le cas le plus
/// contraint. [`crate::openai_llm`] tranche sur la présence d'appels accumulés,
/// jamais sur le seul `finish_reason`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ToolChoice {
    /// Le modèle décide : appeler un outil, ou répondre en texte. Le défaut,
    /// et le comportement historique.
    #[default]
    Auto,
    /// Le modèle **doit** appeler un outil, celui qu'il veut.
    Required,
    /// Aucun outil, même si `tools` en propose. Utile pour forcer une
    /// synthèse en fin de boucle d'agent.
    None,
    /// **Cet** outil précis, nommé. C'est la sortie structurée du pauvre :
    /// un outil dont le schéma décrit la forme voulue.
    Function(String),
}

impl ToolChoice {
    /// La forme du protocole : une chaîne pour les trois premiers, un objet
    /// pour l'outil nommé.
    pub fn to_openai_json(&self) -> serde_json::Value {
        match self {
            ToolChoice::Auto => serde_json::json!("auto"),
            ToolChoice::Required => serde_json::json!("required"),
            ToolChoice::None => serde_json::json!("none"),
            ToolChoice::Function(name) => serde_json::json!({
                "type": "function",
                "function": { "name": name },
            }),
        }
    }
}

/// Forme imposée à la sortie, sérialisée en `response_format`.
///
/// Le schéma est un [`serde_json::Value`] brut : on ne dépend d'aucune
/// bibliothèque de dérivation, et nos [`crate::tools::ToolDef`] en produisent
/// déjà — leur champ `parameters` se branche ici tel quel.
#[derive(Debug, Clone, PartialEq)]
pub enum ResponseFormat {
    /// Texte libre, explicitement. Rarement utile — ne rien envoyer revient
    /// au même — mais permet d'annuler un réglage hérité.
    Text,
    /// JSON valide, sans schéma imposé. Le prompt **doit** demander du JSON,
    /// sans quoi le modèle peut tourner jusqu'à `max_tokens`.
    JsonObject,
    /// JSON conforme à un schéma. `strict` demande au fournisseur de le
    /// garantir plutôt que de le suggérer — et impose alors des contraintes
    /// sur le schéma lui-même.
    JsonSchema {
        /// Nom du schéma, exigé par le protocole.
        name: String,
        /// JSON Schema de la réponse attendue.
        schema: serde_json::Value,
        strict: bool,
    },
}

impl ResponseFormat {
    /// Raccourci pour un schéma strict.
    pub fn strict_schema(name: impl Into<String>, schema: serde_json::Value) -> Self {
        ResponseFormat::JsonSchema { name: name.into(), schema, strict: true }
    }

    /// La forme du protocole.
    pub fn to_openai_json(&self) -> serde_json::Value {
        match self {
            ResponseFormat::Text => serde_json::json!({ "type": "text" }),
            ResponseFormat::JsonObject => serde_json::json!({ "type": "json_object" }),
            ResponseFormat::JsonSchema { name, schema, strict } => serde_json::json!({
                "type": "json_schema",
                "json_schema": { "name": name, "schema": schema, "strict": strict },
            }),
        }
    }
}

/// Budget de réflexion, sérialisé en `reasoning_effort`.
///
/// **Type fermé, et volontairement.** Une faute de frappe doit être une erreur
/// de compilation : le fournisseur, lui, répond 400 en énumérant les valeurs
/// qu'il accepte — trop tard, et en production.
///
/// ## Pourquoi ça compte (mesuré sur Vertex, `gemini-3.5-flash`, 34 375 jetons
/// d'entrée)
///
/// | réglage | temps | jetons de réflexion | issue | coût |
/// |---|---|---|---|---|
/// | aucun | 90 s | 11 520 | **tronqué** | ~0,050 $ |
/// | `thinking_budget: 5600` | 66 s | **15 361** (ignoré) | **tronqué** | ~0,050 $ |
/// | [`ReasoningEffort::Low`] | **9 s** | 0 | complet | **0,0149 $** |
/// | [`ReasoningEffort::Minimal`] | 12 s | 0 | complet | 0,0145 $ |
///
/// Première chose à retenir : la réflexion **s'étend jusqu'à remplir
/// `max_tokens`** si on ne la borne pas (11 520 sur 12 000 ; 15 361 sur
/// 16 000), ce qui tronque la vraie réponse.
///
/// ## Pourquoi `reasoning_effort` et pas `thinking_config`
///
/// La mesure ci-dessus montre `thinking_budget: 5600` sans effet. La doc de
/// Google en donne la raison, et elle n'est pas « l'extension est ignorée » :
/// `extra_body.google.thinking_config` **est officiellement supporté**. Mais
/// sur Gemini 3.x, le budget **en jetons** n'a plus d'effet fin — c'est
/// `thinking_level` (une énumération) qui pilote, `thinking_budget` ne
/// subsistant que pour les modèles 2.5. On avait donc réglé le mauvais bouton,
/// pas un bouton débranché.
///
/// Deux raisons de s'en tenir à `reasoning_effort` :
///
/// - les deux sont **mutuellement exclusifs** — « only one of
///   `reasoning_effort` or `extra_body.google.thinking_config` may be
///   specified ». En envoyer deux est une erreur, pas une priorité
///   silencieuse. Si quelqu'un ajoute `thinking_config` un jour, il **doit**
///   retirer `reasoning_effort` ;
/// - `reasoning_effort` est un paramètre standard, donc portable : il vaut
///   aussi pour OpenAI, alors que `thinking_config` ne parle qu'à Google.
///
/// ## Pourquoi ces quatre valeurs, et pas cinq
///
/// `none` est délibérément absent : OpenAI l'accepte, **Gemini 3.x le
/// refuse** (« Reasoning cannot be turned off for Gemini 2.5 Pro or 3
/// models »). L'exposer inviterait à écrire du code qui marche sur un
/// fournisseur et casse sur l'autre. Symétriquement, `xhigh` et `max`
/// existent chez OpenAI seulement. Ces quatre-là sont l'intersection utile ;
/// `low`, `medium` et `high` sont les trois qui n'ont jamais posé problème
/// nulle part.
///
/// Correspondance documentée par Google : `minimal`/`low` → 1 024 jetons de
/// réflexion sur 2.5, `medium` → 8 192, `high` → 24 576. Sur Gemini 3.5 Flash
/// le défaut du modèle est `medium`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningEffort {
    /// Le moins possible. Mesuré : 12 s, 0 jeton de réflexion. Google avait
    /// un bogue qui le refusait sur Gemini 3 Flash (400 énumérant les valeurs
    /// valides) ; corrigé depuis, mais `Low` reste le choix sûr.
    Minimal,
    /// Le meilleur compromis mesuré : 9 s, réponse complète, 0,0149 $.
    Low,
    Medium,
    High,
}

impl ReasoningEffort {
    /// La valeur du protocole. Ce sont les quatre que Google accepte ; OpenAI
    /// les accepte aussi.
    pub fn as_str(self) -> &'static str {
        match self {
            ReasoningEffort::Minimal => "minimal",
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
        }
    }
}

impl fmt::Display for ReasoningEffort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Comptage d'un appel. `prompt_tokens` est ce qu'a coûté le préremplissage,
/// `completion_tokens` le nombre de fragments émis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    /// Durée totale de l'appel, **attentes de réessai comprises**.
    pub ms: u64,
    /// Nombre de réessais effectués. `0` en temps normal.
    ///
    /// Redondant avec [`TokenSink::on_retry`], et c'est voulu : le rappel sert
    /// à *agir* (journaliser, annuler), ce compteur à ne jamais être muet.
    /// Un appelant qui n'a rien implémenté voit quand même, après coup, qu'un
    /// appel de 63 s en a passé 60 à attendre.
    pub retries: u32,
    /// Appels d'outils **récupérés dans le texte** ([`recover_tool_calls`]).
    /// `0` en temps normal. Même raison d'être que `retries` : un appelant
    /// qui n'a rien implémenté voit quand même, après coup, que le modèle
    /// n'a pas parlé le bon protocole.
    pub recovered_calls: u32,
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

// ─── Séquences d'arrêt (partagé cloud / local) ──────────────────────────────
//
// Ces trois fonctions étaient dans `openai_llm.rs`. Elles n'ont rien de
// spécifique à SSE : le problème qu'elles résolvent est celui de **tout**
// producteur qui pousse par morceaux — une trame SSE ou un jeton de
// décodeur — face à une séquence d'arrêt qui peut être à cheval sur deux
// morceaux. Le puits pousse, donc ce qu'on lui a donné est irrattrapable :
// la rétention est la seule façon de tenir la règle du préfixe verbatim.
//
// Publiques, et pas `pub(crate)` : le contrat de `GenOptions::stop` dit
// « verbatim, séquence non émise », et qui implémente `Llm` hors de cette
// crate doit le tenir. Le laisser réinventer, c'est le laisser produire le
// bogue qu'on a déjà corrigé deux fois.

/// Première séquence d'arrêt présente dans `text` : celle qui apparaît le plus
/// tôt, et à position égale la plus longue. Rend son décalage en octets et la
/// séquence. Le texte qui la précède est gardé **verbatim** par l'appelant,
/// espaces compris — même règle que le `MockLlm` de [`crate::llm`].
pub fn first_stop(text: &str, stops: &[String]) -> Option<(usize, String)> {
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
pub fn holdback(pending: &str, stops: &[String]) -> usize {
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
pub fn emit(sink: &mut dyn TokenSink, emitted: &mut usize, frag: &str) -> Result<(), ()> {
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
    /// Appels d'outils à annoncer, en `(nom, arguments)`. Les identifiants
    /// sont dérivés de la conversation par [`ToolCall::local_id`] : un modèle
    /// local n'en reçoit pas du fournisseur, il doit les fabriquer — et les
    /// fabriquer **de façon déterministe**, sinon un rejeu présenterait des
    /// identifiants différents et casserait l'appariement.
    pub tool_calls: Vec<(String, String)>,
}

impl Default for MockLlm {
    fn default() -> Self {
        Self::new("Bonjour, je suis un modèle de test.")
    }
}

impl MockLlm {
    pub fn new(reply: impl Into<String>) -> Self {
        Self { reply: reply.into(), context_len: 4096, tool_calls: Vec::new() }
    }

    pub fn with_context_len(mut self, n: usize) -> Self {
        self.context_len = n;
        self
    }

    /// Fait annoncer des appels d'outils, en `(nom, arguments JSON)`.
    pub fn with_tool_calls(
        mut self,
        calls: Vec<(impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.tool_calls =
            calls.into_iter().map(|(n, a)| (n.into(), a.into())).collect();
        self
    }

    /// Les appels que cette conversation ferait annoncer. Publique parce que
    /// c'est ce qui permet à un test de connaître les identifiants attendus
    /// sans les copier à la main.
    pub fn announced_calls(&self, turns: &[Turn]) -> Vec<ToolCall> {
        let context = mock_context(turns);
        self.tool_calls
            .iter()
            .enumerate()
            .map(|(i, (n, a))| ToolCall::local(&context, i, n, a))
            .collect()
    }
}

/// Le contexte qui sert de graine aux identifiants locaux : les tours déjà
/// joués, aplatis. Deux conversations différentes donnent des identifiants
/// différents ; la même conversation redonne les mêmes.
fn mock_context(turns: &[Turn]) -> String {
    let mut out = String::new();
    for t in turns {
        out.push_str(&t.role);
        out.push('\u{1}');
        out.push_str(&t.content);
        out.push('\u{1}');
        for c in &t.tool_calls {
            out.push_str(&c.id);
            out.push('\u{1}');
        }
        if let Some(id) = &t.tool_call_id {
            out.push_str(id);
            out.push('\u{1}');
        }
    }
    out
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
        let mut finish = Finish::eos();

        // Les appels sont **annoncés d'emblée**, avant le texte : c'est ce que
        // fait un vrai flux SSE (l'`id` et le nom arrivent en premier, les
        // arguments par fragments). Ils sont donc déjà connus si le puits
        // annule au milieu — et c'est exactement le cas qu'il faut survivre.
        let announced = self.announced_calls(turns);

        for frag in fragments(&self.reply) {
            if emitted >= opts.max_tokens {
                finish = Finish::max_tokens();
                break;
            }
            if let Some((keep, seq)) = stop_hit(&acc, &frag, &opts.stop) {
                if keep > 0 {
                    emitted += 1;
                    let head = &frag[..keep];
                    acc.push_str(head);
                    if sink.on_token(head) == Flow::Stop {
                        finish = Finish::cancelled();
                        break;
                    }
                }
                finish = Finish::stop(seq);
                break;
            }
            acc.push_str(&frag);
            emitted += 1;
            if sink.on_token(&frag) == Flow::Stop {
                finish = Finish::cancelled();
                break;
            }
        }

        // Quelle que soit la raison de fin, les appels annoncés repartent
        // avec elle. C'est l'invariant : aucun identifiant n'est perdu.
        if !announced.is_empty() {
            if finish.reason == FinishReason::Eos {
                finish = Finish::tool_call(announced);
            } else {
                finish = finish.with_tool_calls(announced);
            }
        }

        sink.on_finish(&finish);
        Ok((
            finish,
            Usage {
                prompt_tokens,
                completion_tokens: emitted,
                ms: started.elapsed().as_millis() as u64,
                // Un modèle local ne réessaie rien.
                retries: 0, recovered_calls: 0 },
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
        assert_eq!(finish, Finish::eos());
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
        assert_eq!(finish, Finish::max_tokens());
        assert!(!finish.is_complete(), "tronqué par notre plafond, pas fini");
        assert_eq!(usage.completion_tokens, 2);

        // Cas limite : aucun fragment autorisé.
        let mut sink = StringSink::default();
        let opts = GenOptions::default().with_max_tokens(0);
        let (finish, _) = llm.generate(&hello(), &opts, &mut sink).unwrap();
        assert_eq!(sink.text, "");
        assert_eq!(finish, Finish::max_tokens());
    }

    #[test]
    fn stop_sequence_cuts_and_is_not_emitted() {
        let llm = MockLlm::new("réponse ici FIN et la suite");
        let mut sink = StringSink::default();
        let opts = GenOptions::default().with_stop(vec!["FIN".into()]);
        let (finish, _) = llm.generate(&hello(), &opts, &mut sink).unwrap();
        // Préfixe verbatim : l'espace qui précède "FIN" est conservé.
        assert_eq!(sink.text, "réponse ici ");
        assert_eq!(finish, Finish::stop("FIN"));
        assert!(finish.is_complete(), "l'appelant a demandé ce stop");

        // La séquence la plus précoce gagne, même déclarée en second.
        let mut sink = StringSink::default();
        let opts = GenOptions::default().with_stop(vec!["suite".into(), "ici".into()]);
        let (finish, _) = llm.generate(&hello(), &opts, &mut sink).unwrap();
        assert_eq!(finish, Finish::stop("ici"));
        assert_eq!(sink.text, "réponse ");

        // Un stop vide est ignoré (sinon il couperait à l'octet 0).
        let mut sink = StringSink::default();
        let opts = GenOptions::default().with_stop(vec![String::new()]);
        let (finish, _) = llm.generate(&hello(), &opts, &mut sink).unwrap();
        assert_eq!(finish, Finish::eos());
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
        assert_eq!(finish, Finish::stop("FIN"));
        assert_eq!(usage.completion_tokens, 2);
    }

    #[test]
    fn sink_stop_cancels_the_generation() {
        let llm = MockLlm::new("un deux trois quatre cinq");
        let mut sink = CountingSink::stopping_after(2);
        let (finish, usage) = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();
        assert_eq!(finish, Finish::cancelled());
        assert!(!finish.is_complete(), "annulé : la réponse est incomplète");
        assert_eq!(sink.tokens, 2, "le générateur s'arrête net, il n'en pousse pas un de plus");
        assert_eq!(usage.completion_tokens, 2);
        assert_eq!(sink.finished, Some(Finish::cancelled()), "on_finish est appelé même annulé");
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
        assert_eq!(finish, Finish::cancelled());
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
        assert_eq!(out.finish, Finish::eos());
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
                    return Ok((Finish::cancelled(), Usage::default()));
                }
            }
            sink.on_finish(&Finish::eos());
            Ok((Finish::eos(), Usage { prompt_tokens: 1, completion_tokens: 4, ms: 0, retries: 0 , recovered_calls: 0 }))
        });
        let mut sink = StringSink::default();
        let opts = GenOptions::default().with_max_tokens(7);
        let (finish, usage) = llm.generate(&hello(), &opts, &mut sink).unwrap();
        assert_eq!(sink.text, "2 tours, max 7");
        assert_eq!(finish, Finish::eos());
        assert_eq!(usage.completion_tokens, 4);
        assert_eq!(llm.name(), "cb");
        assert_eq!(llm.context_len(), 128);
    }

    // ─── Appels d'outils : identité, survie, appariement ────────────────────

    #[test]
    fn local_ids_are_deterministic_stable_and_distinct() {
        let turns = vec![Turn::user("cherche luciole")];
        let ctx = mock_context(&turns);
        let a = ToolCall::local_id(&ctx, 0, "KBQuerySourceNode", r#"{"q":1}"#);
        // Déterministe : même entrée, même identifiant. C'est ce qui rend un
        // rejeu identique — l'invariant demandé.
        assert_eq!(a, ToolCall::local_id(&ctx, 0, "KBQuerySourceNode", r#"{"q":1}"#));
        // Distinct dès qu'un seul élément change.
        assert_ne!(a, ToolCall::local_id(&ctx, 1, "KBQuerySourceNode", r#"{"q":1}"#));
        assert_ne!(a, ToolCall::local_id(&ctx, 0, "AutreNode", r#"{"q":1}"#));
        assert_ne!(a, ToolCall::local_id(&ctx, 0, "KBQuerySourceNode", r#"{"q":2}"#));
        let other = mock_context(&[Turn::user("autre chose")]);
        assert_ne!(a, ToolCall::local_id(&other, 0, "KBQuerySourceNode", r#"{"q":1}"#));
        // Forme : préfixe conventionnel, et seulement [a-z0-9_] — aucun
        // fournisseur n'impose de format, mais celui-ci passe partout.
        assert!(a.starts_with("call_local_"), "{a}");
        assert_eq!(a.len(), "call_local_".len() + 16);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'), "{a}");
    }

    #[test]
    fn turn_carries_the_three_real_shapes() {
        let plain = Turn::user("bonjour");
        assert!(plain.tool_calls.is_empty() && !plain.is_tool_result());

        let call = ToolCall::new("call_A", "KBQuerySourceNode", r#"{"kb_name":"docs"}"#);
        let asst = Turn::assistant_with_calls("", vec![call.clone()]);
        assert_eq!(asst.role, "assistant");
        assert_eq!(asst.tool_calls, vec![call]);
        assert!(!asst.is_tool_result());

        let res = Turn::tool_result("call_A", "KBQuerySourceNode", "12 résultats");
        assert_eq!(res.role, "tool");
        assert_eq!(res.tool_call_id.as_deref(), Some("call_A"));
        assert_eq!(res.tool_name.as_deref(), Some("KBQuerySourceNode"));
        assert!(res.is_tool_result());
        // Un nom vide ne devient pas `Some("")` : le champ est facultatif.
        assert_eq!(Turn::tool_result("call_A", "", "x").tool_name, None);
    }

    #[test]
    fn orphan_tool_calls_spots_what_would_make_a_400() {
        let a = ToolCall::new("call_A", "f_a", "{}");
        let b = ToolCall::new("call_B", "f_b", "{}");
        let mut turns = vec![
            Turn::user("fais deux choses"),
            Turn::assistant_with_calls("", vec![a.clone(), b.clone()]),
            Turn::tool_result("call_A", "f_a", "ok"),
        ];
        // `call_B` n'a pas de résultat : c'est exactement ce que le
        // fournisseur refuse.
        let orphans = orphan_tool_calls(&turns);
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].id, "call_B");
        assert!(dangling_tool_results(&turns).is_empty());

        assert_eq!(close_orphan_tool_calls(&mut turns, INTERRUPTED_TOOL_RESULT), 1);
        assert!(orphan_tool_calls(&turns).is_empty(), "plus rien d'orphelin");
        // Le résultat fabriqué garde le nom de l'outil, et son contenu est du
        // JSON lisible par l'agent.
        let last = turns.last().unwrap();
        assert_eq!(last.tool_call_id.as_deref(), Some("call_B"));
        assert_eq!(last.tool_name.as_deref(), Some("f_b"));
        assert!(serde_json::from_str::<serde_json::Value>(&last.content).is_ok());
        // Idempotent : rien à combler la seconde fois.
        assert_eq!(close_orphan_tool_calls(&mut turns, INTERRUPTED_TOOL_RESULT), 0);
    }

    #[test]
    fn close_inserts_results_next_to_their_assistant_turn() {
        // Le scénario qui casse une insertion en fin d'historique :
        // l'utilisateur a repris la parole après l'interruption. Le résultat
        // manquant doit se glisser AVANT ce tour d'utilisateur, sinon un
        // message `user` s'intercale entre l'assistant et ses résultats — ce
        // que le fournisseur refuse.
        let a = ToolCall::new("call_A", "f_a", "{}");
        let b = ToolCall::new("call_B", "f_b", "{}");
        let mut turns = vec![
            Turn::user("fais deux choses"),
            Turn::assistant_with_calls("", vec![a, b]),
            Turn::tool_result("call_A", "f_a", "ok"),
            Turn::user("laisse tomber, autre chose"),
        ];
        assert_eq!(close_orphan_tool_calls(&mut turns, INTERRUPTED_TOOL_RESULT), 1);

        let roles: Vec<&str> = turns.iter().map(|t| t.role.as_str()).collect();
        assert_eq!(roles, ["user", "assistant", "tool", "tool", "user"]);
        assert_eq!(turns[3].tool_call_id.as_deref(), Some("call_B"));
        assert!(orphan_tool_calls(&turns).is_empty());

        // Aucun message étranger ne s'intercale dans le bloc de résultats.
        let asst = roles.iter().position(|r| *r == "assistant").unwrap();
        let mut k = asst + 1;
        while k < turns.len() && turns[k].is_tool_result() {
            k += 1;
        }
        let announced = turns[asst].tool_calls.len();
        assert_eq!(k - asst - 1, announced, "les {announced} résultats suivent l'assistant");
    }

    #[test]
    fn close_handles_several_assistant_turns_independently() {
        let mut turns = vec![
            Turn::user("un"),
            Turn::assistant_with_calls("", vec![ToolCall::new("call_1", "f", "{}")]),
            Turn::user("deux"),
            Turn::assistant_with_calls("", vec![ToolCall::new("call_2", "g", "{}")]),
            Turn::user("trois"),
        ];
        assert_eq!(close_orphan_tool_calls(&mut turns, INTERRUPTED_TOOL_RESULT), 2);
        let roles: Vec<&str> = turns.iter().map(|t| t.role.as_str()).collect();
        assert_eq!(roles, ["user", "assistant", "tool", "user", "assistant", "tool", "user"]);
        assert_eq!(turns[2].tool_call_id.as_deref(), Some("call_1"));
        assert_eq!(turns[5].tool_call_id.as_deref(), Some("call_2"));
        assert!(orphan_tool_calls(&turns).is_empty());
    }

    #[test]
    fn local_ids_fit_the_provider_length_limit() {
        // OpenAI valide la longueur des `tool_calls[].id` : *"Expected a string
        // with maximum length 40"*. Nos identifiants locaux doivent y tenir.
        let id = ToolCall::local_id("un contexte de conversation assez long", 7, "UnNode", "{}");
        assert!(id.len() <= 40, "identifiant de {} caractères : {id}", id.len());
        assert_eq!(id.len(), 27);
    }

    #[test]
    fn dangling_tool_results_spot_the_reverse_error() {
        // Un résultat sans appel correspondant : symptôme d'un historique
        // tronqué par le début, refusé tout autant par le fournisseur.
        let turns = vec![Turn::user("x"), Turn::tool_result("call_Z", "f", "ok")];
        assert_eq!(dangling_tool_results(&turns), vec!["call_Z"]);
        assert!(orphan_tool_calls(&turns).is_empty());
    }

    #[test]
    fn announced_calls_survive_a_cancellation() {
        // LE scénario : le modèle annonce un appel, l'utilisateur interrompt.
        // L'identifiant ne doit pas disparaître avec l'annulation.
        let llm = MockLlm::new("je cherche un peu")
            .with_tool_calls(vec![("KBQuerySourceNode", r#"{"kb_name":"docs"}"#)]);
        let turns = vec![Turn::user("cherche")];
        let expected = llm.announced_calls(&turns);

        let mut sink = CountingSink::stopping_after(1);
        let (finish, _) = llm.generate(&turns, &GenOptions::default(), &mut sink).unwrap();

        assert_eq!(finish.reason, FinishReason::Cancelled);
        assert!(!finish.is_complete());
        assert!(finish.has_tool_calls(), "les appels annoncés doivent survivre");
        assert_eq!(finish.tool_calls, expected);
        // Et le puits les voit aussi, par `on_finish`.
        assert_eq!(sink.finished.unwrap().tool_calls, expected);
    }

    #[test]
    fn announced_calls_survive_max_tokens() {
        let llm = MockLlm::new("un deux trois quatre")
            .with_tool_calls(vec![("f", "{}")]);
        let turns = vec![Turn::user("x")];
        let opts = GenOptions::default().with_max_tokens(2);
        let mut sink = StringSink::default();
        let (finish, _) = llm.generate(&turns, &opts, &mut sink).unwrap();
        assert_eq!(finish.reason, FinishReason::MaxTokens);
        assert_eq!(finish.tool_calls, llm.announced_calls(&turns));
    }

    #[test]
    fn a_normal_finish_with_calls_is_a_tool_call_finish() {
        let llm = MockLlm::new("").with_tool_calls(vec![("f", "{}"), ("g", "{}")]);
        let turns = vec![Turn::user("x")];
        let mut sink = StringSink::default();
        let (finish, _) = llm.generate(&turns, &GenOptions::default(), &mut sink).unwrap();
        assert_eq!(finish.reason, FinishReason::ToolCall);
        assert!(finish.is_complete());
        assert_eq!(finish.tool_calls.len(), 2);
        // Deux appels dans le même tour ont des identifiants distincts.
        assert_ne!(finish.tool_calls[0].id, finish.tool_calls[1].id);
    }

    #[test]
    fn replaying_a_conversation_regenerates_the_same_ids() {
        // L'invariant, sur le chemin local : rejouer le même historique
        // représente au fournisseur exactement les mêmes identifiants.
        let llm = MockLlm::new("").with_tool_calls(vec![("f", r#"{"a":1}"#)]);
        let turns = vec![Turn::system("sys"), Turn::user("cherche")];
        let mut s1 = StringSink::default();
        let (f1, _) = llm.generate(&turns, &GenOptions::default(), &mut s1).unwrap();
        let mut s2 = StringSink::default();
        let (f2, _) = llm.generate(&turns, &GenOptions::default(), &mut s2).unwrap();
        assert_eq!(f1.tool_calls, f2.tool_calls);

        // Un historique différent donne d'autres identifiants — sinon deux
        // appels distincts d'une même conversation se confondraient.
        let longer = vec![Turn::system("sys"), Turn::user("cherche"), Turn::user("encore")];
        let mut s3 = StringSink::default();
        let (f3, _) = llm.generate(&longer, &GenOptions::default(), &mut s3).unwrap();
        assert_ne!(f1.tool_calls[0].id, f3.tool_calls[0].id);
    }

    #[test]
    fn the_interruption_cycle_closes_cleanly() {
        // Bout en bout : annonce → interruption → on referme → l'historique
        // est de nouveau bien formé et rejouable.
        let llm = MockLlm::new("je réfléchis longuement ici")
            .with_tool_calls(vec![("f_a", "{}"), ("f_b", "{}"), ("f_c", "{}")]);
        let mut turns = vec![Turn::user("fais trois choses")];

        let mut sink = CountingSink::stopping_after(1);
        let (finish, _) = llm.generate(&turns, &GenOptions::default(), &mut sink).unwrap();
        assert_eq!(finish.reason, FinishReason::Cancelled);
        assert_eq!(finish.tool_calls.len(), 3);

        // L'appelant reconstruit le tour d'assistant à partir de ce qu'il a
        // reçu — rien n'a été perdu.
        turns.push(Turn::assistant_with_calls("", finish.tool_calls.clone()));
        // Deux des trois ont eu le temps de tourner.
        turns.push(Turn::tool_result(&finish.tool_calls[0].id, "f_a", "ok a"));
        turns.push(Turn::tool_result(&finish.tool_calls[1].id, "f_b", "ok b"));
        assert_eq!(orphan_tool_calls(&turns).len(), 1, "le troisième est orphelin");

        assert_eq!(close_orphan_tool_calls(&mut turns, INTERRUPTED_TOOL_RESULT), 1);
        assert!(orphan_tool_calls(&turns).is_empty());
        assert!(dangling_tool_results(&turns).is_empty());
        // Les trois appels ont chacun leur résultat, et les identifiants sont
        // ceux qu'avait annoncés le modèle.
        let answered: Vec<&str> =
            turns.iter().filter_map(|t| t.tool_call_id.as_deref()).collect();
        for c in &finish.tool_calls {
            assert!(answered.contains(&c.id.as_str()), "{} sans résultat", c.id);
        }
    }

    #[test]
    fn finish_helpers() {
        assert!(Finish::eos().is_complete());
        assert!(Finish::stop("x").is_complete());
        assert!(Finish::tool_call(vec![]).is_complete());
        assert!(!Finish::cancelled().is_complete());
        assert!(!Finish::max_tokens().is_complete());
        assert!(!Finish::eos().has_tool_calls());
        let c = vec![ToolCall::new("i", "n", "{}")];
        assert!(Finish::cancelled().with_tool_calls(c.clone()).has_tool_calls());
        assert_eq!(Finish::new(FinishReason::Eos, c.clone()).tool_calls, c);
    }

    #[test]
    fn usage_tokens_per_s() {
        assert_eq!(Usage::default().tokens_per_s(), 0.0);
        let u = Usage { prompt_tokens: 0, completion_tokens: 50, ms: 1000, retries: 0 , recovered_calls: 0 };
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

// ─── Arguments d'appel d'outil : réparation et mise sur le fil ──────────────

/// Plafond de récupérations par réponse. Au-delà, on s'arrête et on le dit :
/// une récupération non bornée est une porte ouverte (l'idée vient du
/// `maxRecoveries` de LR_XMLParser, qui a raison sur ce point).
pub const MAX_RECOVERED_CALLS: usize = 8;

/// Longueur maximale fouillée. Un texte de plusieurs centaines de kilooctets
/// n'est pas un appel d'outil oublié, c'est une réponse.
const MAX_RECOVER_SCAN: usize = 64 * 1024;

/// Récupère les appels d'outils **restés dans le texte**, et rend
/// `(texte nettoyé, appels, diagnostics)`.
///
/// Certains modèles — Qwen3-Coder par `llama-server` en particulier —
/// écrivent leur appel dans le contenu au lieu du champ `tool_calls`, quand
/// le gabarit du serveur ne sait pas le convertir. Notre boucle voit alors
/// « aucun outil demandé » et conclut le tour : mesuré le 26 août sur
/// Qwen3-Coder-30B, une question sur cinq perdue pour cette seule raison.
///
/// **Ce n'est pas du XML** et on n'essaie pas d'en faire : `<function=x>`
/// n'est pas un nom de balise légal. C'est un scanner tolérant de trois
/// formes connues, borné en longueur et en nombre :
///
/// 1. `<function=NOM><parameter=CLÉ>valeur</parameter>…</function>` (Qwen3-Coder) ;
/// 2. `<tool_call>{"name":…,"arguments":{…}}</tool_call>` (Hermes, Qwen2.5) ;
/// 3. `[TOOL_CALLS] [{"name":…,"arguments":{…}}]` (Mistral).
///
/// Les diagnostics ne sont jamais silencieux : ce qui a été récupéré, et ce
/// qui a été laissé faute de place, revient dans la troisième valeur.
pub fn recover_tool_calls(text: &str) -> (String, Vec<ToolCall>, Vec<String>) {
    let mut calls = Vec::new();
    let mut diagnostics = Vec::new();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let scan_end = text.len().min(MAX_RECOVER_SCAN);
    if scan_end < text.len() {
        diagnostics.push(format!(
            "récupération : {} octets fouillés sur {}",
            scan_end,
            text.len()
        ));
    }
    let hay = &text[..floor_char_boundary(text, scan_end)];

    let push = |name: String, arguments: String, span: (usize, usize), calls: &mut Vec<ToolCall>, spans: &mut Vec<(usize, usize)>, diagnostics: &mut Vec<String>| {
        if calls.len() >= MAX_RECOVERED_CALLS {
            diagnostics.push(format!("récupération : plafond de {MAX_RECOVERED_CALLS} appels atteint, le reste est laissé dans le texte"));
            return;
        }
        let id = ToolCall::local_id("recovered", calls.len(), &name, &arguments);
        calls.push(ToolCall { id, name, arguments, provider_extra: None });
        spans.push(span);
    };

    // 1. `<function=NOM>` … `<parameter=CLÉ>` … `</function>`
    let mut at = 0usize;
    while let Some(rel) = hay[at..].find("<function=") {
        let start = at + rel;
        let Some(head_end) = hay[start..].find('>').map(|d| start + d) else { break };
        let name = hay[start + "<function=".len()..head_end].trim().to_string();
        let body_start = head_end + 1;
        let (body_end, span_end) = match hay[body_start..].find("</function>") {
            Some(d) => (body_start + d, body_start + d + "</function>".len()),
            None => (hay.len(), hay.len()),
        };
        let mut args = serde_json::Map::new();
        let body = &hay[body_start..body_end];
        let mut p = 0usize;
        while let Some(rel) = body[p..].find("<parameter=") {
            let ps = p + rel;
            let Some(key_end) = body[ps..].find('>').map(|d| ps + d) else { break };
            let key = body[ps + "<parameter=".len()..key_end].trim().to_string();
            let vs = key_end + 1;
            let ve = body[vs..].find("</parameter>").map(|d| vs + d).unwrap_or(body.len());
            args.insert(key, serde_json::Value::String(body[vs..ve].trim().to_string()));
            p = (ve + "</parameter>".len()).min(body.len());
        }
        if name.is_empty() {
            diagnostics.push("récupération : `<function=>` sans nom, ignoré".into());
        } else {
            push(name, serde_json::Value::Object(args).to_string(), (start, span_end), &mut calls, &mut spans, &mut diagnostics);
        }
        at = span_end.max(start + 1);
    }

    // 2. `<tool_call>{…}</tool_call>` — du JSON dans une balise.
    let mut at = 0usize;
    while let Some(rel) = hay[at..].find("<tool_call>") {
        let start = at + rel;
        let inner = start + "<tool_call>".len();
        let (end, span_end) = match hay[inner..].find("</tool_call>") {
            Some(d) => (inner + d, inner + d + "</tool_call>".len()),
            None => (hay.len(), hay.len()),
        };
        if spans.iter().any(|(a, b)| start < *b && *a < span_end) {
            at = span_end.max(start + 1);
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(hay[inner..end].trim()) {
            Ok(v) => match json_call(&v) {
                Some((name, arguments)) => push(name, arguments, (start, span_end), &mut calls, &mut spans, &mut diagnostics),
                None => diagnostics.push("récupération : `<tool_call>` sans `name`".into()),
            },
            Err(e) => diagnostics.push(format!("récupération : `<tool_call>` illisible ({e})")),
        }
        at = span_end.max(start + 1);
    }

    // 3. `[TOOL_CALLS] [ … ]`
    if let Some(start) = hay.find("[TOOL_CALLS]") {
        let inner = start + "[TOOL_CALLS]".len();
        if let Some(open) = hay[inner..].find('[') {
            let from = inner + open;
            if let Some(close) = hay[from..].rfind(']') {
                let span_end = from + close + 1;
                match serde_json::from_str::<serde_json::Value>(&hay[from..span_end]) {
                    Ok(serde_json::Value::Array(items)) => {
                        for v in &items {
                            match json_call(v) {
                                Some((name, arguments)) => push(name, arguments, (start, span_end), &mut calls, &mut spans, &mut diagnostics),
                                None => diagnostics.push("récupération : `[TOOL_CALLS]` sans `name`".into()),
                            }
                        }
                    }
                    Ok(_) => diagnostics.push("récupération : `[TOOL_CALLS]` n'est pas un tableau".into()),
                    Err(e) => diagnostics.push(format!("récupération : `[TOOL_CALLS]` illisible ({e})")),
                }
            }
        }
    }

    if calls.is_empty() {
        return (text.to_string(), calls, diagnostics);
    }

    // Le texte rendu au modèle ne garde pas les appels récupérés — ni les
    // balises orphelines que le serveur a laissées en chemin.
    spans.sort_unstable();
    let mut cleaned = String::with_capacity(text.len());
    let mut cursor = 0usize;
    for (a, b) in spans {
        if a >= cursor {
            cleaned.push_str(&text[cursor..a]);
            cursor = b;
        }
    }
    cleaned.push_str(&text[cursor.min(text.len())..]);
    for orphan in ["</tool_call>", "<tool_call>", "</function>"] {
        cleaned = cleaned.replace(orphan, "");
    }
    (cleaned.trim().to_string(), calls, diagnostics)
}

/// `{"name": …, "arguments": {…} | "…"}` → `(nom, arguments bruts)`.
fn json_call(v: &serde_json::Value) -> Option<(String, String)> {
    let name = v.get("name")?.as_str()?.to_string();
    let arguments = match v.get("arguments").or_else(|| v.get("parameters")) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => "{}".to_string(),
    };
    Some((name, arguments))
}

/// `str::floor_char_boundary` n'est pas stable : la même chose, à la main.
fn floor_char_boundary(s: &str, mut i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Échappe les caractères de contrôle **bruts** à l'intérieur des chaînes
/// JSON de `raw` (`\n` réel → `\\n`, etc.), sans toucher au reste.
///
/// Vertex, avec `stream_function_call_arguments`, fragmente les arguments en
/// morceaux qui portent les retours à la ligne **non échappés** — du JSON
/// invalide par construction pour toute valeur multi-ligne (25 août 2026,
/// un appel `edit`). Un modèle local peut faire pareil. Réparer avant de
/// parser évite de refuser un appel qui ne pèche que par là.
pub fn repair_arguments_json(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 16);
    let mut in_string = false;
    let mut escaped = false;
    for c in raw.chars() {
        if in_string {
            if escaped {
                out.push(c);
                escaped = false;
                continue;
            }
            match c {
                '\\' => {
                    out.push(c);
                    escaped = true;
                }
                '"' => {
                    out.push(c);
                    in_string = false;
                }
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        } else {
            if c == '"' {
                in_string = true;
            }
            out.push(c);
        }
    }
    out
}

/// Les arguments tels qu'on les **renvoie** à un fournisseur : un objet JSON
/// valide, ou `{}`. Google valide les arguments de l'historique et refuse
/// toute la requête sinon (« Expected a valid JSON object in the request ») ;
/// un appel tronqué par le flux ne doit pas rendre la conversation
/// irrécupérable. Le texte brut reste dans notre historique, lui.
pub fn arguments_for_wire(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "{}".to_string();
    }
    let repaired = repair_arguments_json(trimmed);
    match serde_json::from_str::<serde_json::Value>(&repaired) {
        Ok(serde_json::Value::Object(_)) => repaired,
        _ => "{}".to_string(),
    }
}

#[cfg(test)]
mod arguments_tests {
    use super::*;

    #[test]
    fn raw_newlines_inside_strings_are_escaped_and_nothing_else() {
        let raw = "{\"new\":\"    /// a\n    pub fn b() {}\",\"n\":1}";
        let fixed = repair_arguments_json(raw);
        let v: serde_json::Value = serde_json::from_str(&fixed).expect("valid after repair");
        assert_eq!(v["new"], "    /// a\n    pub fn b() {}");
        assert_eq!(v["n"], 1);
        // Un échappement déjà présent n'est pas doublé.
        assert_eq!(repair_arguments_json("{\"a\":\"x\\ny\"}"), "{\"a\":\"x\\ny\"}");
        // Hors chaîne, rien ne bouge (le retour à la ligne de mise en forme est légal).
        assert_eq!(repair_arguments_json("{\n\"a\": 1\n}"), "{\n\"a\": 1\n}");
    }

    #[test]
    fn wire_arguments_are_always_an_object() {
        assert_eq!(arguments_for_wire(""), "{}");
        assert_eq!(arguments_for_wire("{\"a\":\"b\nc\"}"), "{\"a\":\"b\\nc\"}");
        // Tronqué : irréparable → objet vide plutôt qu'une requête refusée.
        assert_eq!(arguments_for_wire("{\"new\":\"    pub fn len("), "{}");
        assert_eq!(arguments_for_wire("[1,2]"), "{}");
    }

    // ── Appels d'outils restés dans le texte ────────────────────────

    /// La sortie **exacte** de Qwen3-Coder-30B par `llama-server`, mesurée
    /// le 26 août : l'appel est dans le contenu, et le `</tool_call>` est
    /// orphelin — le serveur a commencé à convertir puis a renoncé.
    const QWEN_STRAY: &str = "I'll search for the `take_results` function to find where it's defined and which methods call it.\n\n<function=search>\n<parameter=target>\nScope\n</parameter>\n<parameter=query>\ntake_results\n</parameter>\n</function>\n</tool_call>";

    #[test]
    fn a_qwen_call_left_in_the_text_is_recovered() {
        let (text, calls, diags) = recover_tool_calls(QWEN_STRAY);
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].name, "search");
        let args: serde_json::Value = serde_json::from_str(&calls[0].arguments).unwrap();
        assert_eq!(args["target"], "Scope");
        assert_eq!(args["query"], "take_results");
        assert!(!calls[0].id.is_empty());
        // Le texte rendu ne garde ni l'appel ni la balise orpheline.
        assert_eq!(text, "I'll search for the `take_results` function to find where it's defined and which methods call it.");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn the_json_in_a_tag_and_the_mistral_form_are_recovered_too() {
        let hermes = "voici\n<tool_call>{\"name\": \"read\", \"arguments\": {\"path\": \"a.rs\"}}</tool_call>\nvoilà";
        let (text, calls, _) = recover_tool_calls(hermes);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read");
        assert_eq!(serde_json::from_str::<serde_json::Value>(&calls[0].arguments).unwrap()["path"], "a.rs");
        assert_eq!(text, "voici\n\nvoilà");

        let mistral = "[TOOL_CALLS] [{\"name\": \"grep\", \"arguments\": {\"pattern\": \"fn\"}}]";
        let (text, calls, _) = recover_tool_calls(mistral);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "grep");
        assert!(text.is_empty(), "{text:?}");
    }

    #[test]
    fn recovery_is_bounded_and_never_silent() {
        // Au-delà du plafond : on s'arrête, et on le dit.
        let many = (0..MAX_RECOVERED_CALLS + 3)
            .map(|i| format!("<function=read><parameter=path>f{i}.rs</parameter></function>"))
            .collect::<Vec<_>>()
            .join("\n");
        let (_, calls, diags) = recover_tool_calls(&many);
        assert_eq!(calls.len(), MAX_RECOVERED_CALLS);
        assert!(diags.iter().any(|d| d.contains("plafond")), "{diags:?}");

        // Un JSON illisible dans une balise : aucun appel, un diagnostic.
        let (_, calls, diags) = recover_tool_calls("<tool_call>{pas du json}</tool_call>");
        assert!(calls.is_empty());
        assert!(diags.iter().any(|d| d.contains("illisible")), "{diags:?}");

        // Une longueur déraisonnable est bornée, et signalée.
        let long = format!("{}<function=read><parameter=path>a.rs</parameter></function>", "x".repeat(MAX_RECOVER_SCAN + 10));
        let (_, calls, diags) = recover_tool_calls(&long);
        assert!(calls.is_empty());
        assert!(diags.iter().any(|d| d.contains("octets fouillés")), "{diags:?}");
    }

    #[test]
    fn an_honest_answer_is_left_alone() {
        let plain = "La fonction est définie dans `port.rs`, ligne 101. Rien à récupérer ici.";
        let (text, calls, diags) = recover_tool_calls(plain);
        assert_eq!(text, plain);
        assert!(calls.is_empty() && diags.is_empty());
    }
}
