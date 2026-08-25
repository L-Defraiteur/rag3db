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

    /// Les outils à annoncer au modèle. Le défaut — aucun — sert un outillage
    /// qui n'expose rien (un `ToolBox` de test qui refuse tout).
    fn tool_defs(&self) -> Vec<ToolDef> {
        Vec::new()
    }
}

impl<T: ToolBox + ?Sized> ToolBox for &T {
    fn call(&self, call: &ToolCall) -> Turn {
        (**self).call(call)
    }
    fn tool_defs(&self) -> Vec<ToolDef> {
        (**self).tool_defs()
    }
}

impl<T: ToolBox + ?Sized> ToolBox for Arc<T> {
    fn call(&self, call: &ToolCall) -> Turn {
        (**self).call(call)
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

    fn tool_defs(&self) -> Vec<ToolDef> {
        crate::tools::graph_tool_defs(self.tools)
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
}

impl Default for AgentLimits {
    fn default() -> Self {
        Self {
            max_iterations: 8,
            token_budget: None,
            stop_on_repeated_error: true,
            interrupted_tool_result: INTERRUPTED_TOOL_RESULT.to_string(),
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
}

/// Ce qu'une exécution a coûté et comment elle s'est terminée.
///
/// L'historique n'est pas ici : [`Agent::run`] le met à jour **en place**,
/// pour qu'une session interrompue puisse être reprise en poussant un tour de
/// plus dans le même vecteur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRun {
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
    tools: &'a dyn ToolBox,
    opts: GenOptions,
    limits: AgentLimits,
}

impl<'a> Agent<'a> {
    /// Les options de génération partent des défauts, **avec les outils de
    /// l'outillage déjà déclarés** : oublier de les annoncer est la faute
    /// qu'on ne veut pas avoir à diagnostiquer.
    pub fn new(llm: &'a dyn Llm, tools: &'a dyn ToolBox) -> Self {
        Self {
            llm,
            tools,
            opts: GenOptions::default().with_tools(tools.tool_defs()),
            limits: AgentLimits::default(),
        }
    }

    /// Remplace les options de génération. **L'appelant reprend alors la
    /// responsabilité de `tools`** : ce qu'il passe est envoyé tel quel.
    pub fn with_gen_options(mut self, opts: GenOptions) -> Self {
        self.opts = opts;
        self
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
        let mut run = AgentRun {
            text: String::new(),
            iterations: 0,
            tool_calls: 0,
            tool_errors: 0,
            closed_orphans: 0,
            usage: Usage::default(),
            stop: StopReason::MaxIterations,
        };
        // La dernière erreur observée, `(outil, contenu)`. Remise à zéro par
        // le moindre succès : un résultat utile entre deux échecs, c'est un
        // progrès, pas une boucle.
        let mut last_error: Option<(String, String)> = None;
        let mut last_finish = Finish::eos();

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

            let mut tee = TeeSink { inner: sink, text: String::new() };
            let generated = self.llm.generate(turns, &self.opts, &mut tee);
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
            run.text = text.clone();
            accumulate(&mut run.usage, &usage);
            last_finish = finish.clone();

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
                let result = self.tools.call(call);
                run.tool_calls += 1;
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

            if let Some((tool, detail)) = repeated {
                run.stop = StopReason::RepeatedError { tool, detail };
                break;
            }
        }

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
}
