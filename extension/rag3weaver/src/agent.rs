//! La boucle d'agent : générer, exécuter les outils demandés, réinjecter,
//! recommencer — jusqu'à ce que le modèle réponde, ou qu'une borne morde.
//!
//! Elle est courte parce que tout ce qu'elle assemble existait déjà :
//!
//! - [`crate::llm`] garde l'identité des appels à travers les interruptions
//!   ([`Finish::tool_calls`] est un champ, pas le contenu d'une variante) et
//!   sait refermer un historique malformé ([`close_orphan_tool_calls`]) ;
//! - [`crate::dataflow::GraphToolRegistry`] rend **toujours** un
//!   [`Turn::tool_result`], succès ou échec, avec un message écrit pour être
//!   lu par un modèle. La moitié « réessai d'outil » d'une boucle d'agent est
//!   donc déjà faite : un `Err` remonté arrêterait la boucle, un résultat
//!   d'erreur la nourrit.
//!
//! Ce que cette couche ajoute, et qui n'existait nulle part : l'ordre des
//! tours (tous les appels avant tous les résultats), les bornes, l'annulation
//! qui laisse un historique rejouable, et l'arrêt sur erreur répétée.
//!
//! ```ignore
//! let toolbox = GraphToolBox::new(&tools, &nodes, services);
//! let agent = Agent::new(&llm, &toolbox).with_max_iterations(6);
//! let mut turns = vec![Turn::system("…"), Turn::user("cherche les livres Rust")];
//! let mut sink = StringSink::default();
//! let run = agent.run(&mut turns, &mut sink)?;
//! println!("{} — {:?} en {} tours", sink.text, run.stop, run.iterations);
//! ```

use std::sync::Arc;

use crate::llm::{
    close_orphan_tool_calls, Finish, FinishReason, Flow, GenOptions, Llm, LlmError, RetryEvent,
    TokenSink, ToolCall, Turn, Usage, INTERRUPTED_TOOL_RESULT,
};
use crate::tools::ToolDef;

// ─── ToolBox ────────────────────────────────────────────────────────────────

/// Ce dont la boucle a besoin d'un outillage : le décrire, et l'exécuter.
///
/// **Pourquoi un trait plutôt que `&GraphToolRegistry` en dur.** Trois
/// raisons, la première étant la plus contraignante :
///
/// 1. **Le registre ne se suffit pas à lui-même.**
///    `GraphToolRegistry::call` exige un `&NodeRegistry` *et* un
///    `Arc<ServiceRegistry>` — deux notions du dataflow. Les mettre dans la
///    signature de [`Agent::run`] ferait entrer la base de données et le
///    catalogue dans une boucle qui ne fait qu'appeler un modèle. Le trait
///    laisse [`GraphToolBox`] les lier **une fois**, à la construction, et la
///    boucle n'en connaît plus rien.
/// 2. **Le méta-outil de composition n'a pas de nom à chercher.** Ma couche
///    expose déjà `run_definition_as_tool_content`, qui exécute un
///    `GraphDefinition` *sans* registre d'outils. Un outil qui prend un
///    graphe en argument et le fait tourner n'entre dans aucune `BTreeMap` de
///    noms ; il entre en revanche très bien dans ce trait.
/// 3. **Les outils non-graphes.** Un `ToolBox` peut être un client HTTP, un
///    serveur MCP, une fermeture ([`CallbackToolBox`]). Le doc 36 les liste
///    (`kind: http|code|mcp`) ; rien dans la boucle ne doit présumer qu'un
///    outil est un graphe.
///
/// **Le contrat, et il n'est pas négociable :** [`Self::call`] ne peut pas
/// échouer. Un outil qui explose rend un [`Turn::tool_result`] dont le
/// contenu décrit l'échec — c'est ce qui permet au modèle de se rattraper, et
/// c'est ce qui permet à [`Agent::run`] de garder l'historique bien formé.
pub trait ToolBox {
    /// Exécute un appel et rend le tour de résultat correspondant.
    ///
    /// Doit rendre un `Turn` dont `tool_call_id` vaut exactement `call.id` :
    /// c'est ce que les fournisseurs apparient, et un identifiant perdu rend
    /// la conversation irrejouable.
    fn call(&self, call: &ToolCall) -> Turn;

    /// Le même appel, **sous un run** : l'outillage qui sait le faire donne
    /// ce run pour parent à ce qu'il exécute (le graphe d'un outil publie
    /// alors `RunStarted { parent: run }`). Par défaut, l'appel simple.
    fn call_in(&self, call: &ToolCall, _run: &str) -> Turn {
        self.call(call)
    }

    /// Les outils à annoncer au modèle. Le défaut — aucun — sert un outillage
    /// qui n'expose rien (un `ToolBox` de test qui refuse tout).
    fn tool_defs(&self) -> Vec<ToolDef> {
        Vec::new()
    }

    /// Cet outil rend-il un **accusé** tout de suite, son résultat plus tard ?
    ///
    /// Déclaré par la fiche (`%% async: true`), jamais deviné : ni le modèle
    /// ni un seuil de durée n'en décident, sinon le même outil répondrait
    /// tantôt d'un coup tantôt en deux temps, et le modèle n'apprendrait rien
    /// ([doc 10](../docs/26-aout-2026-20h29/10-outils-asynchrones.md) §4.1).
    fn is_async(&self, _tool: &str) -> bool {
        false
    }
}

impl<T: ToolBox + ?Sized> ToolBox for &T {
    fn call(&self, call: &ToolCall) -> Turn {
        (**self).call(call)
    }
    fn is_async(&self, tool: &str) -> bool {
        (**self).is_async(tool)
    }
    fn call_in(&self, call: &ToolCall, run: &str) -> Turn {
        (**self).call_in(call, run)
    }
    fn tool_defs(&self) -> Vec<ToolDef> {
        (**self).tool_defs()
    }
}

impl<T: ToolBox + ?Sized> ToolBox for Arc<T> {
    fn call(&self, call: &ToolCall) -> Turn {
        (**self).call(call)
    }
    fn is_async(&self, tool: &str) -> bool {
        (**self).is_async(tool)
    }
    fn call_in(&self, call: &ToolCall, run: &str) -> Turn {
        (**self).call_in(call, run)
    }
    fn tool_defs(&self) -> Vec<ToolDef> {
        (**self).tool_defs()
    }
}

/// Les graphes-outils, liés à leur registre de nœuds et à leurs services.
///
/// C'est l'adaptateur qui referme les trois arguments de
/// [`crate::dataflow::GraphToolRegistry::call`] en un seul objet.
pub struct GraphToolBox<'a> {
    tools: &'a crate::dataflow::GraphToolRegistry,
    nodes: &'a crate::dataflow::NodeRegistry,
    services: Arc<crate::dataflow::ServiceRegistry>,
    policy: crate::dataflow::NodeTypePolicy,
}

impl<'a> GraphToolBox<'a> {
    pub fn new(
        tools: &'a crate::dataflow::GraphToolRegistry,
        nodes: &'a crate::dataflow::NodeRegistry,
        services: Arc<crate::dataflow::ServiceRegistry>,
    ) -> Self {
        Self { tools, nodes, services, policy: crate::dataflow::NodeTypePolicy::All }
    }

    /// Borne les types de nœuds que les graphes-outils ont le droit
    /// d'instancier. Voir [`crate::dataflow::NodeTypePolicy`].
    pub fn with_policy(mut self, policy: crate::dataflow::NodeTypePolicy) -> Self {
        self.policy = policy;
        self
    }
}

impl ToolBox for GraphToolBox<'_> {
    fn call(&self, call: &ToolCall) -> Turn {
        self.tools
            .call_with_policy(call, self.nodes, self.services.clone(), &self.policy)
    }

    /// Une couche de services par-dessus les partagés, avec `"parent_run"` :
    /// le graphe de l'outil naît sous le run de l'agent.
    fn call_in(&self, call: &ToolCall, run: &str) -> Turn {
        let mut layer = crate::dataflow::ServiceRegistry::layered(self.services.clone());
        layer.register("parent_run", run.to_string());
        self.tools
            .call_with_policy(call, self.nodes, Arc::new(layer), &self.policy)
    }

    fn is_async(&self, tool: &str) -> bool {
        self.tools.get(tool).is_some_and(|t| t.is_async())
    }

    /// Les fiches, résolues contre le catalogue des services quand il y en a
    /// un : les `enum` de cibles et de relations sont ceux du schéma courant.
    fn tool_defs(&self) -> Vec<ToolDef> {
        let catalog = self
            .services
            .get::<Arc<std::sync::Mutex<crate::catalog::Catalog>>>("catalog")
            .cloned();
        let guard = catalog.as_ref().and_then(|c| c.lock().ok());
        crate::tools::graph_tool_defs_with(self.tools, guard.as_deref())
    }
}

/// Outillage par fermeture — pendant de [`crate::llm::CallbackLlm`].
///
/// La fermeture rend le **contenu** du résultat ; l'appariement `id`/nom est
/// fait ici, pour qu'un outillage ad hoc ne puisse pas le rater.
pub struct CallbackToolBox {
    defs: Vec<ToolDef>,
    #[allow(clippy::type_complexity)]
    f: Box<dyn Fn(&ToolCall) -> String + Send + Sync>,
}

impl CallbackToolBox {
    pub fn new(
        defs: Vec<ToolDef>,
        f: impl Fn(&ToolCall) -> String + Send + Sync + 'static,
    ) -> Self {
        Self { defs, f: Box::new(f) }
    }
}

impl ToolBox for CallbackToolBox {
    fn call(&self, call: &ToolCall) -> Turn {
        Turn::tool_result(&call.id, &call.name, (self.f)(call))
    }
    fn tool_defs(&self) -> Vec<ToolDef> {
        self.defs.clone()
    }
}

// ─── Bornes ─────────────────────────────────────────────────────────────────

/// Ce qui empêche la boucle de tourner en silence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentLimits {
    /// Nombre maximal d'appels au modèle. **Borne dure, toujours active** :
    /// c'est elle qui garantit la terminaison, quoi que fasse le modèle.
    ///
    /// Huit par défaut : assez pour une recherche, une relance et une
    /// synthèse, trop peu pour qu'une boucle pathologique coûte cher.
    pub max_iterations: usize,

    /// Budget total de jetons (préremplissage **et** génération, cumulés sur
    /// tous les tours). Vérifié *avant* chaque appel : la boucle s'arrête dès
    /// que le cumul l'atteint, elle ne le dépasse donc que du dernier appel.
    ///
    /// `None` par défaut, et c'est délibéré : le coût d'un jeton dépend du
    /// modèle, et un plafond arbitraire couperait une vraie session au milieu
    /// d'une réponse — un échec plus insidieux que pas de plafond du tout,
    /// puisque `max_iterations` borne déjà la terminaison. C'est un frein de
    /// **coût**, à régler quand on connaît le modèle et le budget.
    pub token_budget: Option<usize>,

    /// Arrêter quand le même outil rend **deux fois exactement la même
    /// erreur**. Voir [`StopReason::RepeatedError`].
    pub stop_on_repeated_error: bool,

    /// Contenu posé dans les résultats fabriqués pour refermer les appels
    /// qu'une interruption a laissés orphelins.
    pub interrupted_tool_result: String,

    /// Au **dernier** appel autorisé, un tour utilisateur qui dit au modèle
    /// que c'est le dernier — et les outils lui sont retirés
    /// (`ToolChoice::None`). Sans ça, un modèle consciencieux passe ses
    /// derniers tours à re-vérifier et la boucle s'arrête sur
    /// `MaxIterations` sans réponse, mission pourtant accomplie (25 août
    /// 2026, Gemini renommant une fonction). `None` désactive.
    pub final_nudge: Option<String>,
}

/// Le texte du dernier tour, par défaut.
pub const FINAL_NUDGE: &str = "This is your last step: no more tool calls are possible. \
Answer now with what you have — what you did, what you found, what remains uncertain.";

impl Default for AgentLimits {
    fn default() -> Self {
        Self {
            max_iterations: 8,
            token_budget: None,
            stop_on_repeated_error: true,
            interrupted_tool_result: INTERRUPTED_TOOL_RESULT.to_string(),
            final_nudge: Some(FINAL_NUDGE.to_string()),
        }
    }
}

/// Pourquoi la boucle s'est arrêtée. Typé, pas une chaîne : un appelant doit
/// pouvoir distinguer « le modèle a fini » de « on l'a coupé », et une
/// interface doit savoir si elle affiche une réponse ou un fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// Le modèle a répondu sans demander d'outil. Porte la raison de fin du
    /// dernier appel : `Eos` est une réponse finie, `MaxTokens` une réponse
    /// **tronquée** qu'il ne faut pas présenter comme complète.
    Finished(FinishReason),
    /// [`AgentLimits::max_iterations`] atteint alors que le modèle appelait
    /// encore des outils.
    MaxIterations,
    /// [`AgentLimits::token_budget`] atteint.
    TokenBudget,
    /// Un puits a rendu [`Flow::Stop`]. L'historique a été refermé : il est
    /// rejouable tel quel.
    Cancelled,
    /// Le même outil a rendu deux fois de suite exactement la même erreur.
    /// La troisième tentative aurait été du gaspillage.
    RepeatedError {
        /// Nom de l'outil qui a échoué deux fois.
        tool: String,
        /// Le contenu d'erreur, à l'identique — c'est ce qui a été comparé.
        detail: String,
    },
    /// **L'agent s'est tu de lui-même** — il a appelé [`PAUSE_DIALOGUE`]
    /// ([doc 12](../docs/26-aout-2026-20h29/12-conversations-a-plusieurs.md)).
    ///
    /// Ce n'est ni une panne ni une limite : c'est une décision. Un silence
    /// par défaillance et un silence choisi se ressemblent en sortie et n'ont
    /// rien à voir — d'où une variante à part, et le genre qui dit ce qui le
    /// réveillera.
    Paused {
        /// Envers qui. Vide : envers le fil entier.
        with: String,
        /// Le genre — c'est **la condition de réveil** (doc 12 §8.1).
        kind: PauseKind,
        /// Ce qu'un humain lit.
        reason: String,
    },
}

/// **Pourquoi un agent se tait**, et donc ce qui le réveillera.
///
/// Le genre *est* la condition de réveil : deux genres qui se réveillent
/// pareil sont un seul genre. C'est le critère qui empêche la liste d'enfler
/// — « poliment terminé » et « travail fini » attendent tous deux un nouveau
/// message, donc c'est le même genre, et la nuance appartient au texte.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PauseKind {
    /// Plus rien à dire. Réveil : un message qui m'est adressé.
    Finished,
    /// J'attends ce run — un outil asynchrone, un enfant. Réveil : sa fin.
    /// **Fait une arête** dans le graphe d'attente.
    WaitingForRun(String),
    /// J'attends que ce participant parle. **Fait une arête**, et c'est par
    /// là qu'un cycle se forme.
    WaitingForPeer(String),
    /// J'attends une direction, de qui voudra. **Ne fait pas d'arête** : un
    /// humain n'attend pas, il vit sa vie — compter cette attente
    /// fabriquerait de faux blocages tous les quarts d'heure.
    WaitingForInstruction,
    /// Je ne peux pas continuer et je ne sais pas ce qui me débloquerait.
    /// **Rien ne me réveille** : c'est le seul genre qui doit remonter.
    /// Ce n'est pas `Finished` — l'un a la forme d'un succès, l'autre d'un
    /// échec, et les confondre cacherait ce qu'on veut voir.
    Blocked,
}

impl PauseKind {
    /// Lit un genre, avec la liste exacte en cas d'erreur — même discipline
    /// que les `%% choices:` des fiches.
    pub fn parse(s: &str, argument: Option<&str>) -> Result<Self, String> {
        match s.trim() {
            "finished" => Ok(Self::Finished),
            "waiting_for_run" => Ok(Self::WaitingForRun(argument.unwrap_or_default().to_string())),
            "waiting_for_peer" => Ok(Self::WaitingForPeer(argument.unwrap_or_default().to_string())),
            "waiting_for_instruction" => Ok(Self::WaitingForInstruction),
            "blocked" => Ok(Self::Blocked),
            other => Err(format!(
                "genre '{other}' n'est pas une valeur admise ; admises : blocked, finished, \
                 waiting_for_instruction, waiting_for_peer, waiting_for_run"
            )),
        }
    }

    /// Ce genre crée-t-il une arête dans le graphe d'attente — donc peut-il
    /// participer à un blocage circulaire (doc 12 §4) ?
    pub fn waits_on_someone(&self) -> bool {
        matches!(self, Self::WaitingForRun(_) | Self::WaitingForPeer(_))
    }

    /// Qui ou quoi est attendu, s'il y a lieu.
    pub fn awaited(&self) -> Option<&str> {
        match self {
            Self::WaitingForRun(id) | Self::WaitingForPeer(id) => Some(id),
            _ => None,
        }
    }
}

/// Le nom réservé par lequel un agent se met en pause. Intercepté par la
/// boucle **avant** l'outillage : se taire est une décision de la boucle, pas
/// un appel de graphe.
pub const PAUSE_DIALOGUE: &str = "pause_dialogue";
/// Le nom réservé par lequel un pair confirme une pause.
pub const CONFIRM_PAUSE: &str = "confirm_pause";

/// Ce qu'une exécution a coûté et comment elle s'est terminée.
///
/// L'historique n'est pas ici : [`Agent::run`] le met à jour **en place**,
/// pour qu'une session interrompue puisse être reprise en poussant un tour de
/// plus dans le même vecteur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRun {
    /// L'identifiant du run — celui de ses événements et de sa trace.
    pub run: String,
    /// Le dernier texte émis par le modèle, quel que soit le tour.
    pub text: String,
    /// Nombre d'appels au modèle.
    pub iterations: usize,
    /// Nombre d'appels d'outils **exécutés**.
    pub tool_calls: usize,
    /// Nombre de résultats d'outils qui portaient une erreur.
    pub tool_errors: usize,
    /// Appels orphelins refermés à la sortie — non nul après une annulation
    /// au milieu d'un appel.
    pub closed_orphans: usize,
    /// Messages lus dans la boîte et injectés en tours `user`
    /// ([`Agent::with_inbox`]).
    pub messages: usize,
    /// Cumul de tous les appels : jetons, durée, **et réessais**.
    pub usage: Usage,
    pub stop: StopReason,
}

impl AgentRun {
    /// Jetons consommés en tout — ce que borne [`AgentLimits::token_budget`].
    pub fn total_tokens(&self) -> usize {
        self.usage.prompt_tokens + self.usage.completion_tokens
    }
}

// ─── Le puits en dérivation ─────────────────────────────────────────────────

/// Laisse passer le texte vers le puits de l'appelant **et** le garde.
///
/// Les appels d'outils, eux, ne passent pas : ils ne sont jamais poussés en
/// jetons. Un `LlmNode` non streamant peut donc utiliser la boucle avec un
/// [`crate::llm::StringSink`] et n'y trouver que du texte.
///
/// `on_finish` est **avalé** : le contrat de [`TokenSink`] dit « une seule
/// fois, à la toute fin », et une boucle fait plusieurs appels. C'est
/// [`Agent::run`] qui l'appelle une fois, avec la fin du dernier appel.
struct TeeSink<'a> {
    inner: &'a mut dyn TokenSink,
    text: String,
}

impl TokenSink for TeeSink<'_> {
    fn on_token(&mut self, delta: &str) -> Flow {
        self.text.push_str(delta);
        self.inner.on_token(delta)
    }
    fn on_finish(&mut self, _reason: &Finish) {}
    fn on_retry(&mut self, event: &RetryEvent<'_>) -> Flow {
        self.inner.on_retry(event)
    }
}

// ─── L'agent ────────────────────────────────────────────────────────────────

/// Un modèle, un outillage, des bornes.
pub struct Agent<'a> {
    llm: &'a dyn Llm,
    tools: &'a (dyn ToolBox + Sync),
    opts: GenOptions,
    limits: AgentLimits,
    /// Le bus où la boucle publie ce qu'elle fait — sans jamais l'attendre.
    events: Option<crate::events::EventBus>,
    /// Le nom sous lequel elle publie.
    name: String,
    /// L'identifiant du prochain run, s'il est choisi ; sinon généré.
    run_id: Option<String>,
    /// Lire sa boîte (`run.<id>.inbox`) entre deux tours.
    inbox: bool,
    /// Les postures de la session : où s'inscrit une pause, et où se lit un
    /// blocage. Partagées — c'est tout l'intérêt.
    postures: Option<Arc<crate::postures::Postures>>,
    /// La session : ce qu'on garde d'un tour à l'autre, et ce qu'on cesse de
    /// payer. Absente, la boucle envoie tout l'historique tel quel — c'est le
    /// comportement d'hier, et le témoin auquel on compare.
    session: Option<Arc<crate::session::Session>>,
}

/// Le curseur sous lequel un agent lit sa boîte. Un message envoyé **avant**
/// le run n'est vu que si ce curseur a été ouvert avant
/// (`bus.cursor(&inbox_topic(id), AGENT_INBOX_CURSOR)`) ; l'agent l'ouvre
/// lui-même au début de son run.
pub const AGENT_INBOX_CURSOR: &str = "agent";

/// La première ligne du **bloc d'attentes**. Sert à le retrouver pour le
/// remplacer : il est réécrit à chaque tour, jamais empilé.
///
/// C'est un marqueur, donc il doit être stable et improbable dans du texte
/// ordinaire ; c'est aussi ce que le modèle lit en premier, donc il doit dire
/// ce qu'il est.
pub const WAITING_BLOCK: &str = "— état de la session —";

impl<'a> Agent<'a> {
    /// Les options de génération partent des défauts, **avec les outils de
    /// l'outillage déjà déclarés** : oublier de les annoncer est la faute
    /// qu'on ne veut pas avoir à diagnostiquer.
    pub fn new(llm: &'a dyn Llm, tools: &'a (dyn ToolBox + Sync)) -> Self {
        Self {
            llm,
            tools,
            opts: GenOptions::default().with_tools(tools.tool_defs()),
            limits: AgentLimits::default(),
            events: None,
            name: "agent".to_string(),
            run_id: None,
            inbox: false,
            postures: None,
            session: None,
        }
    }

    /// Partager les postures de la session.
    ///
    /// Sans elles, une pause s'arrête à ce run : personne ne peut voir qui
    /// attend qui, donc personne ne peut voir un blocage. C'est l'objet
    /// **commun** qui rend l'attente circulaire détectable.
    pub fn with_postures(mut self, postures: Arc<crate::postures::Postures>) -> Self {
        self.postures = Some(postures);
        self
    }

    /// Partager la session : la politique d'absorption et la table de renvois.
    ///
    /// **L'outillage doit être enveloppé par [`crate::session::SessionTools`]**
    /// si on veut que `recall` existe — la boucle n'ajoute pas d'outil dans le
    /// dos de l'appelant. Sans lui, absorber reste correct mais devient une
    /// perte : le modèle voit un renvoi qu'il ne peut pas suivre.
    pub fn with_session(mut self, session: Arc<crate::session::Session>) -> Self {
        self.session = Some(session);
        self
    }

    /// Remplace les options de génération. **L'appelant reprend alors la
    /// responsabilité de `tools`** : ce qu'il passe est envoyé tel quel.
    pub fn with_gen_options(mut self, opts: GenOptions) -> Self {
        self.opts = opts;
        self
    }

    /// Publie chaque appel au modèle et chaque appel d'outil (arguments
    /// exacts, résultat, durée, réessais) sur le bus. Fire and forget : si
    /// personne n'écoute, rien ne se passe ; si le tampon déborde, le plus
    /// ancien est écarté — la boucle ne ralentit jamais pour son observateur.
    pub fn with_events(mut self, bus: crate::events::EventBus) -> Self {
        self.events = Some(bus);
        self
    }

    /// Le nom de l'agent dans ses événements (défaut : `agent`).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// L'identifiant du run — son adresse sur le bus (`run.<id>`,
    /// `run.<id>.inbox`). Généré (`agent-…`) si absent.
    pub fn with_run_id(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    /// Lit sa boîte **entre deux tours** : avant chaque appel au modèle, les
    /// messages arrivés (`Event::Message` sur `run.<id>.inbox`) deviennent
    /// des tours `user` — `[message de <from>] <content>` — dans l'ordre
    /// d'arrivée, l'historique intact. Un message arrivé pendant un appel
    /// d'outil est vu à l'itération suivante : la latence est un tour, la
    /// granularité à laquelle l'agent raisonne. Demande `with_events`.
    pub fn with_inbox(mut self) -> Self {
        self.inbox = true;
        self
    }

    /// Les messages en attente dans la boîte, en tours `user`. Rend combien.
    /// Traite `pause_dialogue` / `confirm_pause`.
    ///
    /// Rend le résultat d'outil à coller dans l'historique, et la raison
    /// d'arrêt s'il faut s'arrêter. `confirm_pause` **n'arrête pas** : il
    /// répond à une pause reçue, il n'en déclare pas une.
    ///
    /// Le pair est prévenu par un message, pas par une réplique — c'est ce
    /// qui tue la boucle de politesses : une pause n'appelle pas de réponse
    /// (doc 12 §2.2).
    fn handle_pause(&self, call: &ToolCall, run_id: &str) -> (Turn, Option<StopReason>) {
        let args: serde_json::Value = serde_json::from_str(&call.arguments).unwrap_or_default();
        let field = |k: &str| args.get(k).and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let (with, reason) = (field("avec"), field("raison"));
        let confirming = call.name == CONFIRM_PAUSE;

        // Une raison est **obligatoire** : sans elle, la pause devient la
        // porte de sortie quand le modèle ne sait pas quoi faire, et on aura
        // des agents qui s'endorment au lieu de travailler (doc 11 §1).
        if reason.is_empty() {
            return (
                Turn::tool_result(
                    call.id.clone(),
                    call.name.clone(),
                    format!("{{\"error\":\"bad_argument\",\"detail\":\"{} exige une 'raison' : une pause sans raison n'est pas une décision\"}}", call.name),
                ),
                None,
            );
        }
        let kind = match PauseKind::parse(&field("genre"), Some(&field("attend"))) {
            Ok(k) => k,
            Err(detail) if confirming => {
                // Confirmer n'exige pas de genre : on rend celui de l'autre.
                let _ = detail;
                PauseKind::Finished
            }
            Err(detail) => {
                return (
                    Turn::tool_result(
                        call.id.clone(),
                        call.name.clone(),
                        format!("{{\"error\":\"bad_choice\",\"detail\":\"{detail}\"}}"),
                    ),
                    None,
                );
            }
        };

        if let Some(bus) = &self.events {
            let verbe = if confirming { "a confirmé la pause" } else { "a mis la communication en pause" };
            let suite = if confirming {
                String::new()
            } else {
                format!(
                    " Si tu penses devoir la clore de ton côté aussi, appelle \
                     {CONFIRM_PAUSE}(avec: \"{}\", raison: …).",
                    self.name
                )
            };
            if !with.is_empty() {
                bus.send_message(
                    run_id,
                    &self.name,
                    &with,
                    &format!("{} {verbe} — raison : {reason}.{suite}", self.name),
                );
            }
            bus.emit(crate::events::CatalogEvent::Message {
                run: run_id.to_string(),
                from: self.name.clone(),
                to: with.clone(),
                content: format!(
                    "[{}] genre={} raison={reason}",
                    if confirming { "pause confirmée" } else { "pause" },
                    serde_json::to_string(&kind).unwrap_or_default()
                ),
            });
        }

        // La posture s'inscrit dans la session : c'est l'objet **commun** qui
        // rend l'attente circulaire détectable. Sans lui, une pause s'arrête
        // à ce run et personne ne peut voir qui attend qui.
        if let Some(postures) = &self.postures {
            postures.record(
                &self.name,
                crate::postures::Posture { with: with.clone(), kind: kind.clone(), reason: reason.clone() },
            );
            // Et si ça ferme une boucle, on le **dit** : un blocage annoncé
            // est un problème, un blocage silencieux est une panne.
            for cycle in postures.deadlocks() {
                if cycle.iter().any(|n| n == &self.name) {
                    let phrase = format!("blocage : {} s'attendent mutuellement", cycle.join(" → "));
                    eprintln!("[rag3weaver] {phrase}");
                    if let Some(bus) = &self.events {
                        bus.emit(crate::events::CatalogEvent::Error {
                            context: "postures".to_string(),
                            message: phrase,
                        });
                    }
                }
            }
        }

        let acquitte = format!(
            "{{\"statut\":\"{}\",\"avec\":\"{with}\",\"genre\":{},\"raison\":\"{reason}\"}}",
            if confirming { "pause confirmée" } else { "en pause" },
            serde_json::to_string(&kind).unwrap_or_default()
        );
        let turn = Turn::tool_result(call.id.clone(), call.name.clone(), acquitte);
        let stop = (!confirming).then(|| StopReason::Paused { with, kind, reason });
        (turn, stop)
    }

    /// Lance un appel d'outil **sous un run enfant**, dans un fil de portée,
    /// et poste son résultat dans la boîte de l'agent.
    ///
    /// Le fil est *scoped* : il emprunte l'outillage sans exiger `'static`,
    /// et il est joint quand le run de l'agent se termine. Un outil
    /// asynchrone travaille donc **pendant** que la boucle parle, et aucun
    /// résultat ne peut survivre à l'agent qui l'a demandé.
    ///
    /// Le résultat arrive comme un message ordinaire, préfixé de sa poignée :
    /// c'est ce qui permet au modèle de le rattacher à sa demande, trois
    /// tours plus tard s'il le faut.
    fn spawn_async_tool<'s>(
        &'s self,
        call: &ToolCall,
        run_id: &str,
        handle: &str,
        scope: &'s std::thread::Scope<'s, '_>,
    ) {
        let Some(bus) = self.events.clone() else { return };
        let call = call.clone();
        let inbox = run_id.to_string();
        let child = crate::events::new_run_id(&call.name);
        let handle = handle.to_string();
        let tools = self.tools;
        let agent = self.name.clone();
        scope.spawn(move || {
            let started = std::time::Instant::now();
            let result = tools.call_in(&call, &child);
            // `send_message` publie **sur la boîte du destinataire** en plus
            // du sujet des messages ; `emit` seul irait dans `messages` et
            // l'agent ne le verrait jamais.
            bus.send_message(
                &child,
                &format!("outil {handle}"),
                &inbox,
                &format!(
                    "{handle} ({}) a rendu après {} ms :\n{}",
                    call.name,
                    started.elapsed().as_millis(),
                    result.content
                ),
            );
            let _ = &agent;
        });
    }

    fn read_inbox(&self, run_id: &str, turns: &mut Vec<Turn>) -> usize {
        let Some(bus) = &self.events else { return 0 };
        if !self.inbox {
            return 0;
        }
        let cursor = bus.cursor(&crate::events::inbox_topic(run_id), AGENT_INBOX_CURSOR);
        let mut rx = match cursor.lock() {
            Ok(rx) => rx,
            Err(_) => return 0,
        };
        let mut n = 0;
        loop {
            match rx.try_recv() {
                Ok(crate::events::Event::Message { from, content, .. }) => {
                    turns.push(Turn::user(format!("[message de {from}] {content}")));
                    n += 1;
                }
                Ok(_) => {}
                Err(async_broadcast::TryRecvError::Overflowed(_)) => continue,
                Err(_) => break,
            }
        }
        n
    }

    fn emit(&self, event: crate::events::CatalogEvent) {
        if let Some(bus) = &self.events {
            bus.emit(event);
        }
    }

    pub fn with_limits(mut self, limits: AgentLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn with_max_iterations(mut self, n: usize) -> Self {
        self.limits.max_iterations = n;
        self
    }

    pub fn with_token_budget(mut self, n: usize) -> Self {
        self.limits.token_budget = Some(n);
        self
    }

    pub fn gen_options(&self) -> &GenOptions {
        &self.opts
    }
    pub fn limits(&self) -> &AgentLimits {
        &self.limits
    }

    /// Fait tourner la boucle sur `turns`, qui est mis à jour en place.
    ///
    /// **L'historique rendu est toujours bien formé** : quel que soit le
    /// chemin de sortie — y compris une erreur du modèle — chaque appel
    /// annoncé a son résultat. C'est la condition pour que la conversation
    /// reparte sans 400.
    ///
    /// `Err` est réservé aux échecs **du modèle** (transport, fenêtre de
    /// contexte) : le modèle ne peut pas s'en rattraper, et les faire passer
    /// pour un tour d'assistant mentirait. Les échecs **d'outils**, eux, ne
    /// remontent jamais : ils entrent dans la conversation.
    pub fn run(
        &self,
        turns: &mut Vec<Turn>,
        sink: &mut dyn TokenSink,
    ) -> Result<AgentRun, LlmError> {
        let run_id = self
            .run_id
            .clone()
            .unwrap_or_else(|| crate::events::new_run_id("agent"));
        let run_started = std::time::Instant::now();
        if self.inbox {
            if let Some(bus) = &self.events {
                // Ouvert avant tout le reste : ce qui arrive pendant le run
                // est vu, même pendant un appel d'outil.
                bus.cursor(&crate::events::inbox_topic(&run_id), AGENT_INBOX_CURSOR);
            }
        }
        self.emit(crate::events::CatalogEvent::RunStarted {
            run: run_id.clone(),
            parent: None,
            kind: "agent".to_string(),
            name: self.name.clone(),
            scope: self.events.as_ref().map(|b| b.scope().clone()),
        });
        let outcome = self.run_inner(turns, sink, &run_id);
        self.emit(crate::events::CatalogEvent::RunFinished {
            run: run_id,
            kind: "agent".to_string(),
            ms: run_started.elapsed().as_millis() as u64,
            ok: outcome.is_ok(),
        });
        outcome
    }

    /// Réduit dans l'historique ce qui n'a plus à y être en entier.
    ///
    /// Sans session, **rien** : c'est la boucle d'hier, à la lettre.
    fn absorb_history(&self, turns: &mut [Turn]) {
        let Some(session) = &self.session else { return };
        session.advance();
        let c = session.absorb(turns);
        if !c.is_noop() {
            // Une politique qui jette la moitié d'un historique sans le dire
            // se débogue à l'aveugle (doc 13 §8).
            self.emit(crate::events::CatalogEvent::TurnCompacted {
                run: self.run_id.clone().unwrap_or_else(|| self.name.clone()),
                rewritten: c.rewritten,
                kept: c.kept,
                dropped: c.dropped,
            });
        }
    }

    /// **Ce qui attend doit se voir** (doc 12 §9) : un bloc d'état, dérivé des
    /// postures au moment d'assembler, jamais un message qu'on archive.
    ///
    /// Trois choix, chacun payé :
    ///
    /// - **dérivé, donc jamais périmé** : la pause tombe, la ligne s'en va, et
    ///   personne n'a à penser à nettoyer ;
    /// - **vide quand il n'y a rien** : un bloc toujours présent apprend au
    ///   modèle à ne plus le lire ;
    /// - **en dernier**, pas en tête : il change à chaque tour, et le mettre au
    ///   début invaliderait le préfixe que le fournisseur met en cache — on
    ///   paierait la visibilité au prix de tout l'historique.
    fn refresh_waiting_block(&self, turns: &mut Vec<Turn>) {
        let Some(postures) = &self.postures else { return };
        turns.retain(|t| !(t.role == "system" && t.content.starts_with(WAITING_BLOCK)));
        let block = postures.describe_for(&self.name);
        if !block.is_empty() {
            turns.push(Turn::system(format!("{WAITING_BLOCK}\n{block}")));
        }
    }

    fn run_inner(
        &self,
        turns: &mut Vec<Turn>,
        sink: &mut dyn TokenSink,
        run_id: &str,
    ) -> Result<AgentRun, LlmError> {
        let mut run = AgentRun {
            run: run_id.to_string(),
            text: String::new(),
            iterations: 0,
            tool_calls: 0,
            tool_errors: 0,
            closed_orphans: 0,
            messages: 0,
            usage: Usage::default(),
            stop: StopReason::MaxIterations,
        };
        // La dernière erreur observée, `(outil, contenu)`. Remise à zéro par
        // le moindre succès : un résultat utile entre deux échecs, c'est un
        // progrès, pas une boucle.
        let mut last_error: Option<(String, String)> = None;
        let mut last_finish = Finish::eos();
        // Une pause termine le run **sans texte** : c'est tout l'objet.
        let mut paused = false;

        // Un fil de portée pour toute la boucle : les outils asynchrones y
        // travaillent pendant qu'on parle, et il est joint quand le run se
        // termine. Aucun résultat ne peut donc survivre à l'agent qui l'a
        // demandé — c'est le même choix qu'au runtime dataflow.
        let interrupted: Result<(), LlmError> = std::thread::scope(|scope| {
        loop {
            if run.iterations >= self.limits.max_iterations {
                run.stop = StopReason::MaxIterations;
                break;
            }
            if self
                .limits
                .token_budget
                .is_some_and(|b| run.total_tokens() >= b)
            {
                run.stop = StopReason::TokenBudget;
                break;
            }

            // Dernier appel autorisé : le dire, et retirer les outils.
            let last_call = run.iterations + 1 >= self.limits.max_iterations;
            let mut opts = self.opts.clone();
            if last_call && run.iterations > 0 {
                if let Some(nudge) = &self.limits.final_nudge {
                    if !turns.last().is_some_and(|t| t.role == "user" && t.content == *nudge) {
                        turns.push(Turn::user(nudge.clone()));
                    }
                    opts.tool_choice = crate::llm::ToolChoice::None;
                }
            }
            // La boîte, à la frontière du tour : jamais au milieu d'un appel.
            run.messages += self.read_inbox(run_id, turns);
            // **Assembler l'invite** — les deux seules choses qui la
            // réécrivent, et elles le font ici, à un seul endroit.
            self.absorb_history(turns);
            self.refresh_waiting_block(turns);
            let mut tee = TeeSink { inner: sink, text: String::new() };
            let generated = self.llm.generate(turns, &opts, &mut tee);
            let text = std::mem::take(&mut tee.text);
            let (finish, usage) = match generated {
                Ok(ok) => ok,
                Err(e) => {
                    // Même en échec, on ne laisse pas un historique impropre.
                    run.closed_orphans += close_orphan_tool_calls(
                        turns,
                        &self.limits.interrupted_tool_result,
                    );
                    return Err(e);
                }
            };

            run.iterations += 1;

            // Un appel d'outil resté **dans le texte** : certains serveurs
            // locaux ne le convertissent pas en `tool_calls` (mesuré le
            // 26 août sur Qwen3-Coder par `llama-server` — une question sur
            // cinq perdue pour cette seule raison, doc 11). Sans ce
            // rattrapage, la boucle voit « aucun outil demandé » et conclut
            // le tour alors que le modèle avait bien décidé d'agir.
            let (mut text, mut finish) = (text, finish);
            if !finish.has_tool_calls() {
                let (cleaned, recovered, diagnostics) = crate::llm::recover_tool_calls(&text);
                for detail in diagnostics {
                    self.emit(crate::events::CatalogEvent::Warning {
                        context: format!("agent:{}", self.name),
                        message: detail,
                    });
                }
                if !recovered.is_empty() {
                    // Jamais silencieux : ce qui a été rattrapé se voit dans
                    // la trace **et** dans le compteur, pour qui n'écoute pas.
                    self.emit(crate::events::CatalogEvent::Warning {
                        context: format!("agent:{}", self.name),
                        message: format!(
                            "{} appel(s) d'outil récupéré(s) dans le texte — le serveur n'a pas converti la réponse du modèle",
                            recovered.len()
                        ),
                    });
                    run.usage.recovered_calls += recovered.len() as u32;
                    // Le texte gardé dans l'historique ne contient plus
                    // l'appel : le modèle le relirait comme du contenu.
                    text = cleaned;
                    finish = finish.with_tool_calls(recovered);
                }
            }

            run.text = text.clone();
            accumulate(&mut run.usage, &usage);
            last_finish = finish.clone();
            self.emit(crate::events::CatalogEvent::LlmCall {
                run: run_id.to_string(),
                agent: self.name.clone(),
                iteration: run.iterations,
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                ms: usage.ms,
                retries: usage.retries,
                finish: format!("{:?}", finish.reason),
                tool_calls: finish.tool_calls.len(),
            });

            // Parler lève sa propre pause : on ne peut pas être en pause et
            // répondre en même temps (doc 12 §2.1). C'est ce qui permet à un
            // pair de réengager sans cérémonie.
            if let Some(postures) = &self.postures {
                if !text.is_empty() || finish.has_tool_calls() {
                    postures.speak(&self.name);
                }
            }

            // ── Pas d'outil demandé : c'est fini ────────────────────────
            if !finish.has_tool_calls() {
                // Un fragment abandonné reste dans l'historique : l'utilisateur
                // l'a vu, le cacher ferait mentir la conversation. Un tour vide
                // (annulation avant le premier jeton), non.
                if !text.is_empty() {
                    turns.push(Turn::assistant(text));
                }
                run.stop = if finish.reason == FinishReason::Cancelled {
                    StopReason::Cancelled
                } else {
                    StopReason::Finished(finish.reason.clone())
                };
                break;
            }

            // ── Des outils : tous les appels d'abord ────────────────────
            // L'ordre est le contrat du protocole : un tour d'assistant qui
            // annonce N appels, puis N tours `tool`, sans rien entre eux.
            turns.push(Turn::assistant_with_calls(text, finish.tool_calls.clone()));

            // Interrompu **pendant** l'annonce : les appels existent, ils
            // n'ont pas tourné. On ne les exécute pas — l'utilisateur a dit
            // stop — et la fermeture d'orphelins en fin de fonction leur
            // fabriquera un résultat.
            if finish.reason == FinishReason::Cancelled {
                run.stop = StopReason::Cancelled;
                break;
            }

            // … puis tous les résultats. Un appel tronqué par `max_tokens`
            // porte des arguments en JSON invalide : on l'exécute quand même,
            // l'outillage en fait une erreur lisible, et le modèle reprend.
            let mut repeated: Option<(String, String)> = None;
            for call in &finish.tool_calls {
                self.emit(crate::events::CatalogEvent::ToolCallStarted {
                    run: run_id.to_string(),
                    agent: self.name.clone(),
                    call_id: call.id.clone(),
                    tool: call.name.clone(),
                    arguments: call.arguments.clone(),
                });
                let started = std::time::Instant::now();

                // **Se taire est une décision de la boucle**, pas un appel de
                // graphe : on intercepte avant l'outillage. Un outil ne peut
                // pas arrêter la boucle qui l'appelle ; cette action, si.
                if call.name == PAUSE_DIALOGUE || call.name == CONFIRM_PAUSE {
                    let (turn, pause) = self.handle_pause(call, run_id);
                    turns.push(turn);
                    run.tool_calls += 1;
                    if let Some(p) = pause {
                        run.stop = p;
                        paused = true;
                    }
                    continue;
                }

                // **Asynchrone** : on rend un accusé tout de suite et le vrai
                // résultat arrive plus tard dans la boîte (doc 10). Le
                // protocole impose une réponse à chaque appel dans le même
                // tour — « plus tard » ne peut donc pas vouloir dire « pas de
                // réponse », mais « une réponse qui dit que ça travaille ».
                let result = if self.tools.is_async(&call.name) && self.events.is_some() {
                    let handle = format!("#{}-{}", call.name, run.tool_calls + 1);
                    self.spawn_async_tool(call, run_id, &handle, scope);
                    Turn::tool_result(
                        call.id.clone(),
                        call.name.clone(),
                        format!(
                            "{{\"handle\": \"{handle}\", \"statut\": \"en cours\", \"outil\": \"{}\", \
                             \"suite\": \"le résultat arrivera comme message ; tu peux parler en attendant\"}}",
                            call.name
                        ),
                    )
                } else {
                    self.tools.call_in(call, run_id)
                };
                run.tool_calls += 1;
                if self.events.is_some() {
                    let error_kind = error_kind(&result.content);
                    self.emit(crate::events::CatalogEvent::ToolCallFinished {
                        run: run_id.to_string(),
                        agent: self.name.clone(),
                        call_id: call.id.clone(),
                        tool: call.name.clone(),
                        ok: error_detail(&result.content).is_none(),
                        error_kind,
                        ms: started.elapsed().as_millis() as u64,
                        bytes: result.content.len(),
                    });
                }
                match error_detail(&result.content) {
                    Some(detail) => {
                        run.tool_errors += 1;
                        let key = (call.name.clone(), detail.to_string());
                        if self.limits.stop_on_repeated_error
                            && last_error.as_ref() == Some(&key)
                            && repeated.is_none()
                        {
                            repeated = Some(key.clone());
                        }
                        last_error = Some(key);
                    }
                    None => last_error = None,
                }
                turns.push(result);
            }

            // Une pause a été prononcée pendant ce tour : on sort. Les autres
            // appels du même tour ont eu leur résultat — l'historique reste
            // bien formé, il est rejouable tel quel.
            if paused {
                break;
            }

            if let Some((tool, detail)) = repeated {
                run.stop = StopReason::RepeatedError { tool, detail };
                break;
            }
        }
        Ok(())
        }); // fin du fil de portée : les outils asynchrones sont joints ici
        interrupted?;
        // Une pause ne produit **pas** de texte : c'est ce qui la distingue
        // d'une réponse. Un agent qui répondrait « d'accord, j'attends »
        // aurait mal compris la consigne (doc 11 §1).
        if paused {
            run.text.clear();
        }

        // Une dernière lecture : un résultat arrivé pendant le dernier tour,
        // ou juste après, est **dans l'historique** plutôt que perdu.
        run.messages += self.read_inbox(run_id, turns);

        run.closed_orphans +=
            close_orphan_tool_calls(turns, &self.limits.interrupted_tool_result);
        // Le puits de l'appelant est prévenu **une seule fois**, avec la fin
        // du dernier appel — comme le veut le contrat de `TokenSink`.
        sink.on_finish(&last_finish);
        Ok(run)
    }
}

fn accumulate(total: &mut Usage, one: &Usage) {
    total.prompt_tokens += one.prompt_tokens;
    total.completion_tokens += one.completion_tokens;
    total.ms += one.ms;
    total.retries += one.retries;
    total.recovered_calls += one.recovered_calls;
}

/// Le détail d'erreur d'un résultat d'outil, s'il en porte une.
///
/// La convention est celle de tout le crate : un échec est un objet JSON avec
/// un champ `error` (voir `GraphToolError::to_tool_json` et
/// [`INTERRUPTED_TOOL_RESULT`]). Un outil qui rend un tableau de résultats,
/// `{"ok":true}` ou du texte libre n'est pas une erreur.
///
/// La comparaison porte sur le **document entier** et pas seulement sur le
/// code : deux `unknown_argument` sur des arguments différents sont deux
/// erreurs différentes, et le modèle progresse peut-être entre les deux.
/// Le `kind` d'un résultat d'outil en erreur (`{"error": "bad_choice", …}`).
fn error_kind(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    serde_json::from_str::<serde_json::Value>(trimmed)
        .ok()?
        .get("error")?
        .as_str()
        .map(str::to_string)
}

fn error_detail(content: &str) -> Option<&str> {
    let trimmed = content.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    let parsed: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    parsed.get("error")?.as_str()?;
    Some(trimmed)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{CallbackLlm, CountingSink, MockLlm, StringSink};
    use std::sync::Mutex;

    /// Un modèle qui joue une suite de [`MockLlm`], un par tour, et rejoue le
    /// dernier ensuite. `MockLlm` seul annoncerait les mêmes outils à chaque
    /// tour, donc ne s'arrêterait jamais ; il reste la source du déterminisme
    /// (les identifiants d'appel viennent de `ToolCall::local`).
    fn scripted(steps: Vec<MockLlm>) -> CallbackLlm {
        assert!(!steps.is_empty());
        let steps = Mutex::new((steps, 0usize));
        CallbackLlm::new("scripted", 4096, move |turns, opts, sink| {
            let step = {
                let mut g = steps.lock().unwrap();
                let i = g.1.min(g.0.len() - 1);
                g.1 += 1;
                g.0[i].clone()
            };
            step.generate(turns, opts, sink)
        })
    }


    /// **Un outil asynchrone rend un accusé, pas un résultat** — et l'agent
    /// peut parler pendant qu'il travaille (doc 10).
    ///
    /// Trois choses vérifiées d'un coup : l'accusé porte une poignée et se
    /// distingue d'un résultat ; le vrai résultat arrive plus tard **dans la
    /// boîte**, préfixé de cette poignée ; et l'agent a produit du texte
    /// entre les deux, ce qui est tout l'objet de l'exercice.
    #[test]
    fn an_async_tool_answers_with_a_handle_and_the_result_comes_later() {
        use std::sync::atomic::{AtomicBool, Ordering};

        static STARTED: AtomicBool = AtomicBool::new(false);
        struct SlowBox(Vec<ToolDef>);
        impl ToolBox for SlowBox {
            fn call(&self, call: &ToolCall) -> Turn {
                STARTED.store(true, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(30));
                Turn::tool_result(call.id.clone(), call.name.clone(), "trouvé : port.rs:101".to_string())
            }
            fn is_async(&self, tool: &str) -> bool {
                tool == "cherche"
            }
            fn tool_defs(&self) -> Vec<ToolDef> {
                self.0.clone()
            }
        }

        let tools = SlowBox(vec![ToolDef {
            name: "cherche".to_string(),
            description: "d".to_string(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        }]);
        let llm = scripted(vec![
            MockLlm::new("").with_tool_calls(vec![("cherche", "{}")]),
            MockLlm::new("je regarde, deux secondes"),
        ]);
        let bus = crate::events::EventBus::new(64);
        let agent = Agent::new(&llm, &tools).with_events(bus.clone()).with_inbox();

        let mut turns = vec![Turn::user("où est merge_port_values ?")];
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();

        // 1. L'accusé : une poignée, et un statut qui ne se confond pas avec
        //    un résultat. Sans ça le modèle raconterait « voilà » pour « c'est
        //    parti », et il aurait l'air de mentir.
        let ack = turns.iter().find(|t| t.tool_call_id.is_some()).expect("un résultat d'outil");
        eprintln!("[accusé] {}", ack.content);
        assert!(ack.content.contains("#cherche-1"), "{}", ack.content);
        assert!(ack.content.contains("en cours"), "{}", ack.content);
        assert!(!ack.content.contains("port.rs:101"), "l'accusé n'est pas le résultat : {}", ack.content);

        // 2. L'agent a parlé pendant ce temps.
        assert!(run.text.contains("je regarde"), "{}", run.text);

        // 3. Le vrai résultat est arrivé, dans la boîte, sous sa poignée.
        let late = turns.iter().find(|t| t.content.contains("port.rs:101"));
        eprintln!("[tardif] {:?}", late.map(|t| t.content.clone()));
        let late = late.expect("le résultat doit finir par arriver");
        assert!(late.content.contains("#cherche-1"), "rattachable à la demande : {}", late.content);
        assert!(STARTED.load(Ordering::SeqCst));
    }

    /// **Se taire est une décision**, et raccrocher n'appelle pas de réponse.
    ///
    /// Trois choses d'un coup : la pause arrête le run sans produire de
    /// texte ; le pair reçoit une **notification** avec le mode d'emploi de
    /// `confirm_pause` — pas une réplique, donc rien qui appelle une réponse,
    /// et c'est ce qui tue la boucle de politesses ; et une pause sans raison
    /// est refusée.
    #[test]
    fn an_agent_can_fall_silent_and_the_peer_is_told_not_replied_to() {
        let bus = crate::events::EventBus::new(64);
        // La boîte du pair, ouverte avant qu'on lui parle.
        let inbox = bus.cursor(&crate::events::inbox_topic("run-b"), AGENT_INBOX_CURSOR);

        let llm = scripted(vec![MockLlm::new("").with_tool_calls(vec![(
            PAUSE_DIALOGUE,
            r#"{"avec":"run-b","genre":"finished","raison":"on s'est tout dit"}"#,
        )])]);
        let tools = toolbox(vec![]);
        let agent = Agent::new(&llm, &tools).with_events(bus.clone()).with_name("run-a").with_run_id("run-a");

        let mut turns = vec![Turn::user("merci beaucoup, au revoir")];
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();

        // 1. Le run s'arrête sur une décision, pas sur une limite.
        match &run.stop {
            StopReason::Paused { with, kind, reason } => {
                assert_eq!(with, "run-b");
                assert_eq!(kind, &PauseKind::Finished);
                assert_eq!(reason, "on s'est tout dit");
            }
            other => panic!("attendu une pause, obtenu {other:?}"),
        }
        // 2. Et sans un mot : une pause n'est pas une réponse.
        assert!(run.text.is_empty(), "{:?}", run.text);

        // 3. Le pair est **prévenu**, avec de quoi confirmer s'il le veut.
        let mut rx = inbox.lock().unwrap();
        let mut recus = Vec::new();
        while let Ok(crate::events::Event::Message { content, from, .. }) = rx.try_recv() {
            recus.push(format!("{from}: {content}"));
        }
        eprintln!("[boîte de run-b] {recus:?}");
        assert_eq!(recus.len(), 1, "{recus:?}");
        assert!(recus[0].contains("mis la communication en pause"), "{recus:?}");
        assert!(recus[0].contains("on s'est tout dit"), "{recus:?}");
        assert!(recus[0].contains(CONFIRM_PAUSE), "le mode d'emploi voyage avec : {recus:?}");
    }

    /// Une pause sans raison n'est pas une décision — elle serait la porte de
    /// sortie quand le modèle ne sait pas quoi faire.
    #[test]
    fn a_pause_without_a_reason_is_refused() {
        let llm = scripted(vec![
            MockLlm::new("").with_tool_calls(vec![(PAUSE_DIALOGUE, r#"{"avec":"run-b","genre":"finished"}"#)]),
            MockLlm::new("bon, je continue alors"),
        ]);
        let tools = toolbox(vec![]);
        let agent = Agent::new(&llm, &tools).with_name("run-a");
        let mut turns = vec![Turn::user("?")];
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();

        let refus = turns.iter().find(|t| t.tool_call_id.is_some()).unwrap();
        eprintln!("[refus] {}", refus.content);
        assert!(refus.content.contains("bad_argument"), "{}", refus.content);
        assert!(!matches!(run.stop, StopReason::Paused { .. }), "la boucle continue : {:?}", run.stop);

        // Et un genre inconnu rend la liste exacte, comme les fiches.
        assert!(PauseKind::parse("waiting", None).unwrap_err().contains("waiting_for_peer"));
    }

    /// **Deux agents qui s'attendent** : chacun se tait poliment, personne
    /// n'est en faute, et rien ne se passe. Le blocage est détecté et **dit**.
    #[test]
    fn two_agents_waiting_on_each_other_are_told_they_are_stuck() {
        let postures = Arc::new(crate::postures::Postures::new());
        let tools = toolbox(vec![]);

        let pause_toward = |peer: &'static str| {
            scripted(vec![MockLlm::new("").with_tool_calls(vec![(
                PAUSE_DIALOGUE,
                Box::leak(
                    format!(r#"{{"avec":"{peer}","genre":"waiting_for_peer","attend":"{peer}","raison":"à toi"}}"#)
                        .into_boxed_str(),
                ) as &'static str,
            )])])
        };

        for (me, peer) in [("a", "b"), ("b", "a")] {
            let llm = pause_toward(peer);
            let agent = Agent::new(&llm, &tools).with_name(me).with_postures(postures.clone());
            let mut turns = vec![Turn::user("?")];
            let run = agent.run(&mut turns, &mut StringSink::default()).unwrap();
            assert!(matches!(run.stop, StopReason::Paused { .. }), "{:?}", run.stop);
        }

        let blocages = postures.deadlocks();
        eprintln!("[blocages] {blocages:?}");
        assert_eq!(blocages, vec![vec!["a".to_string(), "b".to_string()]]);

        // Et l'un des deux reparle : le blocage tombe, sans cérémonie.
        assert!(postures.speak("a"));
        assert!(postures.deadlocks().is_empty());
    }

    /// Le genre décide de ce qui réveille, donc de ce qui peut se bloquer.
    #[test]
    fn only_waiting_on_someone_can_deadlock() {
        assert!(PauseKind::WaitingForPeer("b".into()).waits_on_someone());
        assert!(PauseKind::WaitingForRun("#t-1".into()).waits_on_someone());
        // Attendre une instruction n'est pas attendre quelqu'un : un humain
        // n'attend pas, il vit sa vie. Compter cette attente fabriquerait de
        // faux blocages tous les quarts d'heure.
        assert!(!PauseKind::WaitingForInstruction.waits_on_someone());
        assert!(!PauseKind::Finished.waits_on_someone());
        // `blocked` n'attend personne — mais rien ne le réveille non plus,
        // et c'est le seul genre qui doit remonter.
        assert!(!PauseKind::Blocked.waits_on_someone());
        assert_eq!(PauseKind::WaitingForPeer("b".into()).awaited(), Some("b"));
    }

    /// Outillage qui rend ce qu'on lui dit, par nom d'outil.
    fn toolbox(replies: Vec<(&'static str, &'static str)>) -> CallbackToolBox {
        let map: Vec<(String, String)> = replies
            .into_iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        CallbackToolBox::new(Vec::new(), move |call| {
            map.iter()
                .find(|(n, _)| *n == call.name)
                .map(|(_, r)| r.clone())
                .unwrap_or_else(|| r#"{"error":"unknown_tool","detail":"inconnu"}"#.into())
        })
    }

    /// Au dernier appel autorisé, le modèle reçoit le tour « dernier pas »
    /// et `ToolChoice::None` ; avant, ni l'un ni l'autre.
    #[test]
    fn the_last_call_gets_the_nudge_and_no_tools() {
        use std::sync::Arc;
        let seen: Arc<Mutex<Vec<(bool, crate::llm::ToolChoice)>>> = Arc::new(Mutex::new(Vec::new()));
        let seen2 = seen.clone();
        let llm = CallbackLlm::new("always-calls", 4096, move |turns, opts, sink| {
            let nudged = turns.last().is_some_and(|t| t.role == "user" && t.content == FINAL_NUDGE);
            seen2.lock().unwrap().push((nudged, opts.tool_choice.clone()));
            MockLlm::new("").with_tool_calls(vec![("search", "{}")]).generate(turns, opts, sink)
        });
        let toolbox = toolbox(vec![("search", "[]")]);
        let agent = Agent::new(&llm, &toolbox).with_limits(AgentLimits { max_iterations: 3, ..Default::default() });
        let mut turns = start();
        let run = agent.run(&mut turns, &mut StringSink::default()).unwrap();
        assert_eq!(run.stop, StopReason::MaxIterations);
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 3);
        assert_eq!(seen[0], (false, crate::llm::ToolChoice::Auto));
        assert_eq!(seen[1], (false, crate::llm::ToolChoice::Auto));
        assert_eq!(seen[2], (true, crate::llm::ToolChoice::None), "last call: nudged, no tools");
        // Le tour de relance est dans l'historique, une seule fois.
        assert_eq!(turns.iter().filter(|t| t.content == FINAL_NUDGE).count(), 1);
    }

    fn start() -> Vec<Turn> {
        vec![Turn::system("tu es utile"), Turn::user("cherche")]
    }

    /// L'invariant du protocole : chaque appel annoncé a son résultat, et le
    /// bloc de résultats suit immédiatement son tour d'assistant.
    fn assert_well_formed(turns: &[Turn]) {
        assert!(
            crate::llm::orphan_tool_calls(turns).is_empty(),
            "appels orphelins : {:?}",
            crate::llm::orphan_tool_calls(turns)
                .iter()
                .map(|c| &c.id)
                .collect::<Vec<_>>()
        );
        assert!(
            crate::llm::dangling_tool_results(turns).is_empty(),
            "résultats sans appel : {:?}",
            crate::llm::dangling_tool_results(turns)
        );
        for (i, t) in turns.iter().enumerate() {
            if t.tool_calls.is_empty() {
                continue;
            }
            let results = turns[i + 1..]
                .iter()
                .take_while(|t| t.is_tool_result())
                .count();
            assert_eq!(
                results,
                t.tool_calls.len(),
                "tour {i} : {} appels mais {results} résultats collés",
                t.tool_calls.len()
            );
        }
    }

    // ── 1. Un tour sans outil ───────────────────────────────────────

    #[test]
    fn a_plain_answer_ends_the_loop() {
        let llm = MockLlm::new("Voici la réponse.");
        let tools = toolbox(vec![]);
        let agent = Agent::new(&llm, &tools);

        let mut turns = start();
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();

        assert_eq!(run.iterations, 1);
        assert_eq!(run.tool_calls, 0);
        assert_eq!(run.stop, StopReason::Finished(FinishReason::Eos));
        assert_eq!(sink.text, "Voici la réponse.");
        assert_eq!(run.text, "Voici la réponse.");
        assert_eq!(turns.len(), 3);
        assert_eq!(turns[2].role, "assistant");
        assert!(run.usage.completion_tokens > 0);
        assert_well_formed(&turns);
    }

    // ── 2. Un outil, puis la réponse ────────────────────────────────

    #[test]
    fn a_tool_call_then_a_final_answer() {
        let llm = scripted(vec![
            MockLlm::new("").with_tool_calls(vec![("search", r#"{"query":"rust"}"#)]),
            MockLlm::new("J'ai trouvé deux livres."),
        ]);
        let tools = toolbox(vec![("search", r#"[{"uuid":"a"},{"uuid":"b"}]"#)]);
        let agent = Agent::new(&llm, &tools);

        let mut turns = start();
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();

        assert_eq!(run.iterations, 2);
        assert_eq!(run.tool_calls, 1);
        assert_eq!(run.tool_errors, 0);
        assert_eq!(run.stop, StopReason::Finished(FinishReason::Eos));

        // system, user, assistant(appel), tool(résultat), assistant(texte)
        assert_eq!(turns.len(), 5);
        assert_eq!(turns[2].tool_calls.len(), 1);
        assert_eq!(turns[2].tool_calls[0].name, "search");
        assert!(turns[3].is_tool_result());
        assert_eq!(turns[3].tool_call_id, Some(turns[2].tool_calls[0].id.clone()));
        assert_eq!(turns[3].content, r#"[{"uuid":"a"},{"uuid":"b"}]"#);
        assert_eq!(turns[4].content, "J'ai trouvé deux livres.");

        // Le puits n'a vu que du texte : aucun JSON d'appel ni de résultat.
        assert_eq!(sink.text, "J'ai trouvé deux livres.");
        assert_well_formed(&turns);
    }

    // ── 3. Appels parallèles ────────────────────────────────────────

    #[test]
    fn parallel_calls_are_all_announced_before_all_results() {
        let llm = scripted(vec![
            MockLlm::new("").with_tool_calls(vec![
                ("search", r#"{"query":"a"}"#),
                ("search", r#"{"query":"b"}"#),
                ("other", "{}"),
            ]),
            MockLlm::new("Synthèse."),
        ]);
        let tools = toolbox(vec![("search", "[1]"), ("other", "[2]")]);
        let agent = Agent::new(&llm, &tools);

        let mut turns = start();
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();

        assert_eq!(run.tool_calls, 3);
        // system, user, assistant(3 appels), 3 × tool, assistant
        assert_eq!(turns.len(), 7);
        let announced = &turns[2].tool_calls;
        assert_eq!(announced.len(), 3);
        // Les trois résultats suivent, dans l'ordre et sans intercalation.
        for (k, call) in announced.iter().enumerate() {
            assert!(turns[3 + k].is_tool_result());
            assert_eq!(turns[3 + k].tool_call_id.as_deref(), Some(call.id.as_str()));
        }
        assert_eq!(turns[6].role, "assistant");
        // Deux appels au même outil ont des identifiants distincts.
        assert_ne!(announced[0].id, announced[1].id);
        assert_well_formed(&turns);
    }

    // ── 4. Une erreur d'outil nourrit le tour suivant ───────────────

    #[test]
    fn a_tool_error_feeds_the_next_turn() {
        let llm = scripted(vec![
            MockLlm::new("").with_tool_calls(vec![("search", "{}")]),
            MockLlm::new("Pardon, je reformule.")
                .with_tool_calls(vec![("search", r#"{"query":"rust"}"#)]),
            MockLlm::new("Voilà."),
        ]);
        // La première fois une erreur, ensuite un succès : deux erreurs
        // *différentes* n'arrêteraient pas non plus, mais ici on veut voir la
        // boucle continuer après un échec.
        let calls = Mutex::new(0usize);
        let tools = CallbackToolBox::new(Vec::new(), move |_call| {
            let mut n = calls.lock().unwrap();
            *n += 1;
            if *n == 1 {
                r#"{"error":"missing_argument","detail":"argument requis manquant : 'query'"}"#
                    .into()
            } else {
                r#"[{"uuid":"a"}]"#.into()
            }
        });
        let agent = Agent::new(&llm, &tools);

        let mut turns = start();
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();

        assert_eq!(run.iterations, 3);
        assert_eq!(run.tool_calls, 2);
        assert_eq!(run.tool_errors, 1);
        assert_eq!(run.stop, StopReason::Finished(FinishReason::Eos));
        // L'erreur est bien dans l'historique, lisible par le modèle.
        assert!(turns[3].content.contains("missing_argument"));
        assert!(turns[3].is_tool_result());
        assert_well_formed(&turns);
    }

    // ── 5. La borne d'itérations mord ───────────────────────────────

    #[test]
    fn max_iterations_stops_a_model_that_never_concludes() {
        // Un seul pas, rejoué : le modèle appelle un outil à l'infini.
        let llm = scripted(vec![
            MockLlm::new("").with_tool_calls(vec![("search", r#"{"query":"x"}"#)])
        ]);
        let tools = toolbox(vec![("search", "[]")]);
        let agent = Agent::new(&llm, &tools).with_max_iterations(3);

        let mut turns = start();
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();

        assert_eq!(run.stop, StopReason::MaxIterations);
        assert_eq!(run.iterations, 3);
        assert_eq!(run.tool_calls, 3);
        assert_eq!(run.closed_orphans, 0, "chaque appel a bien eu son résultat");
        assert_well_formed(&turns);
    }

    #[test]
    fn the_token_budget_stops_the_loop_too() {
        let llm = scripted(vec![
            MockLlm::new("un deux trois").with_tool_calls(vec![("search", "{}")])
        ]);
        let tools = toolbox(vec![("search", "[]")]);
        let agent = Agent::new(&llm, &tools)
            .with_max_iterations(100)
            .with_token_budget(12);

        let mut turns = start();
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();

        assert_eq!(run.stop, StopReason::TokenBudget);
        assert!(run.total_tokens() >= 12, "cumul : {}", run.total_tokens());
        assert!(run.iterations < 100);
        assert_well_formed(&turns);
    }

    #[test]
    fn a_zero_iteration_budget_does_nothing_at_all() {
        let llm = MockLlm::new("jamais appelé");
        let tools = toolbox(vec![]);
        let agent = Agent::new(&llm, &tools).with_max_iterations(0);
        let mut turns = start();
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();
        assert_eq!(run.iterations, 0);
        assert_eq!(run.stop, StopReason::MaxIterations);
        assert_eq!(turns.len(), 2);
    }

    // ── 6. Interruption au milieu d'un appel ────────────────────────

    #[test]
    fn an_interruption_mid_call_leaves_a_replayable_history() {
        // Le modèle annonce deux appels *et* du texte ; le puits coupe au
        // deuxième fragment. `MockLlm` annonce les appels d'emblée, comme un
        // vrai flux SSE : ils existent déjà quand l'annulation tombe.
        let llm = MockLlm::new("Je cherche tout de suite").with_tool_calls(vec![
            ("search", r#"{"query":"a"}"#),
            ("search", r#"{"query":"b"}"#),
        ]);
        let tools = toolbox(vec![("search", "[1]")]);
        let agent = Agent::new(&llm, &tools);

        let mut turns = start();
        let mut sink = CountingSink::stopping_after(2);
        let run = agent.run(&mut turns, &mut sink).unwrap();

        assert_eq!(run.stop, StopReason::Cancelled);
        assert_eq!(run.iterations, 1);
        assert_eq!(run.tool_calls, 0, "on n'exécute pas ce que l'utilisateur a coupé");
        assert_eq!(run.closed_orphans, 2);

        // system, user, assistant(2 appels), 2 résultats fabriqués
        assert_eq!(turns.len(), 5);
        assert_eq!(turns[2].tool_calls.len(), 2);
        assert!(turns[3].content.contains("interrupted"));
        assert!(turns[4].content.contains("interrupted"));
        assert_well_formed(&turns);

        // Et l'historique repart : l'utilisateur reprend la parole.
        turns.push(Turn::user("finalement, cherche 'rust'"));
        let llm2 = MockLlm::new("D'accord.");
        let agent2 = Agent::new(&llm2, &tools);
        let mut sink2 = StringSink::default();
        let run2 = agent2.run(&mut turns, &mut sink2).unwrap();
        assert_eq!(run2.stop, StopReason::Finished(FinishReason::Eos));
        assert_well_formed(&turns);
    }

    #[test]
    fn a_cancellation_without_calls_keeps_the_partial_text() {
        let llm = MockLlm::new("Je commence à répondre puis on me coupe");
        let tools = toolbox(vec![]);
        let agent = Agent::new(&llm, &tools);
        let mut turns = start();
        let mut sink = CountingSink::stopping_after(3);
        let run = agent.run(&mut turns, &mut sink).unwrap();

        assert_eq!(run.stop, StopReason::Cancelled);
        assert_eq!(turns.len(), 3);
        assert_eq!(turns[2].content, "Je commence à");
        assert_eq!(run.closed_orphans, 0);
    }

    #[test]
    fn the_caller_sink_is_finished_exactly_once() {
        let llm = scripted(vec![
            MockLlm::new("").with_tool_calls(vec![("search", "{}")]),
            MockLlm::new("fini"),
        ]);
        let tools = toolbox(vec![("search", "[]")]);
        let agent = Agent::new(&llm, &tools);
        let mut turns = start();
        let mut sink = CountingSink::default();
        agent.run(&mut turns, &mut sink).unwrap();
        // Deux appels au modèle, mais une seule fin — celle du dernier.
        assert_eq!(sink.finished, Some(Finish::eos()));
    }

    // ── 7. Erreur répétée ───────────────────────────────────────────

    #[test]
    fn the_same_error_twice_stops_the_loop() {
        let llm = scripted(vec![
            MockLlm::new("").with_tool_calls(vec![("search", "{}")])
        ]);
        let err = r#"{"error":"missing_argument","detail":"argument requis manquant : 'query'"}"#;
        let tools = toolbox(vec![("search", err)]);
        let agent = Agent::new(&llm, &tools).with_max_iterations(50);

        let mut turns = start();
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();

        match &run.stop {
            StopReason::RepeatedError { tool, detail } => {
                assert_eq!(tool, "search");
                assert_eq!(detail, err);
            }
            other => panic!("attendu RepeatedError, reçu {other:?}"),
        }
        assert_eq!(run.iterations, 2, "on s'arrête au deuxième échec identique");
        assert_eq!(run.tool_calls, 2);
        assert_well_formed(&turns);
    }

    #[test]
    fn two_different_errors_do_not_stop_the_loop() {
        let llm = scripted(vec![
            MockLlm::new("").with_tool_calls(vec![("search", "{}")])
        ]);
        let n = Mutex::new(0usize);
        let tools = CallbackToolBox::new(Vec::new(), move |_c| {
            let mut g = n.lock().unwrap();
            *g += 1;
            format!(r#"{{"error":"missing_argument","detail":"essai {}"}}"#, *g)
        });
        let agent = Agent::new(&llm, &tools).with_max_iterations(4);
        let mut turns = start();
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();
        assert_eq!(run.stop, StopReason::MaxIterations);
        assert_eq!(run.tool_errors, 4);
    }

    #[test]
    fn a_success_between_two_identical_errors_is_progress() {
        let llm = scripted(vec![
            MockLlm::new("").with_tool_calls(vec![("search", "{}")])
        ]);
        let err = r#"{"error":"execution","detail":"boum"}"#;
        let n = Mutex::new(0usize);
        let tools = CallbackToolBox::new(Vec::new(), move |_c| {
            let mut g = n.lock().unwrap();
            *g += 1;
            // erreur, succès, erreur, succès… : jamais deux de suite.
            if *g % 2 == 1 { err.to_string() } else { "[]".to_string() }
        });
        let agent = Agent::new(&llm, &tools).with_max_iterations(5);
        let mut turns = start();
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();
        assert_eq!(run.stop, StopReason::MaxIterations);
    }

    #[test]
    fn the_repeated_error_guard_can_be_switched_off() {
        let llm = scripted(vec![
            MockLlm::new("").with_tool_calls(vec![("search", "{}")])
        ]);
        let tools = toolbox(vec![("search", r#"{"error":"execution","detail":"boum"}"#)]);
        let agent = Agent::new(&llm, &tools).with_limits(AgentLimits {
            max_iterations: 3,
            stop_on_repeated_error: false,
            ..Default::default()
        });
        let mut turns = start();
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();
        assert_eq!(run.stop, StopReason::MaxIterations);
        assert_eq!(run.tool_errors, 3);
    }

    // ── Détection d'erreur ──────────────────────────────────────────

    #[test]
    fn what_counts_as_a_tool_error() {
        assert!(error_detail(r#"{"error":"execution","detail":"x"}"#).is_some());
        assert!(error_detail(INTERRUPTED_TOOL_RESULT).is_some());
        // Ce qui n'en est pas :
        assert!(error_detail(r#"[{"uuid":"a"}]"#).is_none(), "un tableau de résultats");
        assert!(error_detail(r#"{"ok":true}"#).is_none(), "un déclencheur");
        assert!(error_detail("texte libre").is_none());
        assert!(error_detail(r#"{"error":42}"#).is_none(), "`error` doit être une chaîne");
        assert!(error_detail(r#"{"errors":["a"]}"#).is_none());
        assert!(error_detail("").is_none());
    }

    // ── Erreur du modèle ────────────────────────────────────────────

    #[test]
    fn a_model_error_propagates_but_leaves_a_clean_history() {
        // Premier tour : des appels. Deuxième : le modèle casse.
        let n = Mutex::new(0usize);
        let first = MockLlm::new("").with_tool_calls(vec![("search", "{}")]);
        let llm = CallbackLlm::new("flaky", 4096, move |turns, opts, sink| {
            let mut g = n.lock().unwrap();
            *g += 1;
            if *g == 1 {
                first.generate(turns, opts, sink)
            } else {
                Err(LlmError::Model("le fournisseur ne répond plus".into()))
            }
        });
        let tools = toolbox(vec![("search", "[]")]);
        let agent = Agent::new(&llm, &tools);

        let mut turns = start();
        let mut sink = StringSink::default();
        let err = agent.run(&mut turns, &mut sink).unwrap_err();
        assert!(matches!(err, LlmError::Model(_)), "{err}");
        // L'historique reste envoyable : le premier appel a eu son résultat.
        assert_well_formed(&turns);
    }

    // ── Câblage ─────────────────────────────────────────────────────

    #[test]
    fn the_toolbox_defs_are_announced_to_the_model() {
        let (nodes, graph_tools) = crate::dataflow::builtin_graph_tools().unwrap();
        let services = Arc::new(crate::dataflow::ServiceRegistry::new());
        let toolbox = GraphToolBox::new(&graph_tools, &nodes, services);
        let llm = MockLlm::new("ok");
        let agent = Agent::new(&llm, &toolbox);

        let names: Vec<&str> =
            agent.gen_options().tools.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, crate::dataflow::graph_tool::BUILTIN_TOOL_NAMES);
        assert_eq!(agent.limits().max_iterations, 8);
        assert_eq!(agent.limits().token_budget, None);
    }

    #[test]
    fn a_graph_toolbox_turns_a_failure_into_a_readable_result() {
        // Sans services, l'exécution échoue — et ça ne remonte pas en `Err`.
        let (nodes, graph_tools) = crate::dataflow::builtin_graph_tools().unwrap();
        let services = Arc::new(crate::dataflow::ServiceRegistry::new());
        let toolbox = GraphToolBox::new(&graph_tools, &nodes, services);
        let llm = scripted(vec![
            MockLlm::new("").with_tool_calls(vec![(
                "search",
                r#"{"target":"Product","query":"rust"}"#,
            )]),
            MockLlm::new("Je n'ai pas pu chercher."),
        ]);
        let agent = Agent::new(&llm, &toolbox);

        let mut turns = start();
        let mut sink = StringSink::default();
        let run = agent.run(&mut turns, &mut sink).unwrap();
        assert_eq!(run.tool_errors, 1);
        assert_eq!(run.stop, StopReason::Finished(FinishReason::Eos));
        let v: serde_json::Value = serde_json::from_str(&turns[3].content).unwrap();
        assert!(v["error"].is_string(), "{}", turns[3].content);
        assert_well_formed(&turns);
    }

    // ── La session : ce qu'on assemble, et ce qu'on cesse de payer ──

    /// **Le bloc d'attentes est une lecture, pas un message.**
    ///
    /// Il apparaît parce qu'un pair s'est tu en attendant celui-ci, et il
    /// s'en va tout seul quand ce pair reparle — sans que personne n'ait à
    /// nettoyer quoi que ce soit (doc 12 §9.2).
    #[test]
    fn le_bloc_d_attentes_apparait_et_disparait_tout_seul() {
        use crate::postures::{Posture, Postures};
        use std::sync::Arc;

        let postures = Arc::new(Postures::new());
        postures.record(
            "chercheur",
            Posture {
                with: "indexeur".into(),
                kind: PauseKind::WaitingForPeer("indexeur".into()),
                reason: "il me faut le chemin exact".into(),
            },
        );

        let vus: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let vus2 = vus.clone();
        let llm = CallbackLlm::new("mock", 4096, move |turns, opts, sink| {
            vus2.lock().unwrap().push(
                turns.iter().filter(|t| t.content.starts_with(WAITING_BLOCK)).map(|t| t.content.clone()).collect(),
            );
            MockLlm::new("vu").generate(turns, opts, sink)
        });
        let tools = toolbox(vec![]);
        let agent = Agent::new(&llm, &tools).with_name("indexeur").with_postures(postures.clone());

        let mut turns = start();
        agent.run(&mut turns, &mut StringSink::default()).unwrap();
        let bloc = vus.lock().unwrap()[0].clone();
        assert!(bloc.contains("chercheur"), "{bloc}");
        assert!(bloc.contains("chemin exact"), "{bloc}");

        // Le pair reparle : l'attente cesse, la ligne s'en va.
        postures.speak("chercheur");
        let mut turns = start();
        agent.run(&mut turns, &mut StringSink::default()).unwrap();
        assert_eq!(vus.lock().unwrap()[1], "", "un bloc vide n'a rien à faire là");
        assert!(!turns.iter().any(|t| t.content.starts_with(WAITING_BLOCK)));
    }

    /// Il est **remplacé**, jamais empilé : dix tours d'attente ne font pas
    /// dix blocs.
    #[test]
    fn le_bloc_d_attentes_ne_s_empile_pas() {
        use crate::postures::{Posture, Postures};
        use std::sync::Arc;

        let postures = Arc::new(Postures::new());
        postures.record(
            "chercheur",
            Posture { with: "indexeur".into(), kind: PauseKind::WaitingForPeer("indexeur".into()), reason: "x".into() },
        );
        let llm = CallbackLlm::new("mock", 4096, |turns, opts, sink| {
            MockLlm::new("").with_tool_calls(vec![("search", "{}")]).generate(turns, opts, sink)
        });
        let tools = toolbox(vec![("search", "[]")]);
        let agent = Agent::new(&llm, &tools)
            .with_name("indexeur")
            .with_postures(postures)
            .with_limits(AgentLimits { max_iterations: 4, ..Default::default() });
        let mut turns = start();
        agent.run(&mut turns, &mut StringSink::default()).unwrap();
        assert_eq!(turns.iter().filter(|t| t.content.starts_with(WAITING_BLOCK)).count(), 1);
    }

    /// **Sans session, rien ne change.** C'est la promesse qui vient avant
    /// toutes les autres : le chemin simple reste le chemin simple.
    #[test]
    fn sans_session_l_historique_n_est_pas_touche() {
        let tools = gros_read();
        let llm = tour_par_tour(3);
        let agent = Agent::new(&llm, &tools).with_limits(AgentLimits { max_iterations: 4, ..Default::default() });
        let mut turns = start();
        agent.run(&mut turns, &mut StringSink::default()).unwrap();
        assert!(turns.iter().any(|t| t.content.len() == 20_000));
    }

    /// Un `read` qui rend vingt mille caractères — la taille d'un fichier
    /// qu'on lit vraiment.
    fn gros_read() -> CallbackToolBox {
        CallbackToolBox::new(
            vec![ToolDef { name: "read".into(), description: String::new(), parameters: serde_json::json!({}) }],
            |_| "y".repeat(20_000),
        )
    }

    /// Un modèle qui appelle `read` `n` fois, puis répond.
    fn tour_par_tour(n: usize) -> CallbackLlm {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let reste = AtomicUsize::new(n);
        CallbackLlm::new("mock", 4096, move |turns, opts, sink| {
            if reste.fetch_sub(1, Ordering::SeqCst) > 0 {
                MockLlm::new("").with_tool_calls(vec![("read", "{}")]).generate(turns, opts, sink)
            } else {
                MockLlm::new("fini").generate(turns, opts, sink)
            }
        })
    }

    /// **Le chiffre de vérité** (doc 13 §9.4) : une conversation de dix tours,
    /// les mêmes appels, mesurée avec et sans absorption.
    ///
    /// Ce que le test fixe, ce n'est pas un ratio — il dépend de la taille des
    /// résultats — c'est la **forme** : ça baisse beaucoup, et rien n'est
    /// perdu, puisque `recall` rend l'original au caractère près.
    #[test]
    fn dix_tours_avec_et_sans_absorption() {
        use crate::session::{Absorb, Session, SessionTools};
        use std::sync::Arc;

        fn mesure(session: Option<Arc<Session>>) -> usize {
            let inner = gros_read();
            let vu = Arc::new(Mutex::new(0usize));
            let vu2 = vu.clone();
            let llm = {
                use std::sync::atomic::{AtomicUsize, Ordering};
                let reste = AtomicUsize::new(9);
                CallbackLlm::new("mock", 4096, move |turns: &[Turn], opts: &GenOptions, sink: &mut dyn TokenSink| {
                    // Ce qu'on envoie au modèle, ce tour-ci : la seule chose
                    // qui se paie.
                    *vu2.lock().unwrap() += turns.iter().map(|t| t.content.len()).sum::<usize>();
                    if reste.fetch_sub(1, Ordering::SeqCst) > 0 {
                        MockLlm::new("").with_tool_calls(vec![("read", "{}")]).generate(turns, opts, sink)
                    } else {
                        MockLlm::new("fini").generate(turns, opts, sink)
                    }
                })
            };
            let limits = AgentLimits { max_iterations: 12, ..Default::default() };
            let mut turns = start();
            match &session {
                Some(s) => {
                    let tools = SessionTools::new(&inner, s.clone());
                    Agent::new(&llm, &tools)
                        .with_session(s.clone())
                        .with_limits(limits)
                        .run(&mut turns, &mut StringSink::default())
                        .unwrap();
                }
                None => {
                    Agent::new(&llm, &inner)
                        .with_limits(limits)
                        .run(&mut turns, &mut StringSink::default())
                        .unwrap();
                }
            }
            let n = *vu.lock().unwrap();
            n
        }

        let sans = mesure(None);
        let session = Arc::new(Session::new().with_policy(Absorb::Stale { max_chars: 2_000, after_turns: 2 }));
        let avec = mesure(Some(session.clone()));

        // Neuf résultats de 20 000 caractères, réenvoyés à chaque tour : le
        // témoin est quadratique, et c'est bien ça le problème.
        assert!(sans > 800_000, "témoin : {sans}");
        assert!(avec * 4 < sans, "avec {avec}, sans {sans}");
        // Et rien n'est perdu.
        assert_eq!(session.recall("#read-1").map(|c| c.len()), Some(20_000));
        eprintln!("[dix tours] sans absorption {sans} caractères, avec {avec}");
    }
}
