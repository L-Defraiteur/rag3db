//! **La session** : ce qu'on garde d'un tour à l'autre, et ce qu'on cesse de
//! payer.
//!
//! Voir [doc 13](../docs/25-aout-2026-18h58/13-la-session-comme-graphe.md) §3
//! et §5.
//!
//! Une conversation avec outils grossit d'une façon particulière : le
//! résultat d'un `read` de deux cents lignes obtenu au tour 2 est **réenvoyé
//! au modèle à chaque tour suivant**. Au tour 10, on l'a payé neuf fois, et
//! le plus souvent pour rien — il a servi une fois, au tour où il est arrivé.
//!
//! Ce module tient les deux moitiés de la réponse :
//!
//! 1. **`absorb`** réduit dans l'historique ce qui n'a plus à y être en
//!    entier ;
//! 2. **la table de renvois** garde le contenu intact, à côté, adressable par
//!    un nom court et stable — `#read-2`.
//!
//! Le point de (2) n'est pas l'étiquette, c'est que [`SessionTools`] expose
//! `recall` : sans un outil qui les résout, des renvois ne sont que de la
//! décoration, et réduire l'historique devient une perte d'information
//! (doc 13 §5).
//!
//! ## Ce que ce module ne fait pas
//!
//! Il ne touche à rien tant qu'on ne le lui demande pas. [`Absorb::Whole`]
//! est le défaut, et il reproduit la boucle d'aujourd'hui **à la lettre** :
//! aucun agent ne change de comportement du seul fait que ce fichier existe.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::llm::{ToolCall, Turn};
use crate::tools::ToolDef;

/// Ce qu'on garde d'un résultat d'outil **dans l'invite**.
///
/// Le contenu entier, lui, est toujours gardé dans la session : réduire
/// l'historique n'est pas oublier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Absorb {
    /// Tout, tel quel, à chaque tour. Le comportement d'aujourd'hui, et le
    /// témoin auquel on compare.
    Whole,
    /// Au-delà de `max_chars`, la tête et un renvoi. S'applique **dès le tour
    /// où le résultat arrive** : un résultat énorme l'est déjà maintenant.
    Bounded { max_chars: usize },
    /// Borné, et **périmé** : passé `after_turns` tours, il ne reste qu'une
    /// ligne. C'est la politique qui paye, parce qu'elle vise exactement ce
    /// qui coûte — l'ancien, pas le gros.
    Stale { max_chars: usize, after_turns: usize },
}

impl Default for Absorb {
    fn default() -> Self {
        Self::Whole
    }
}

/// Ce qu'on a mis de côté pour un appel donné.
#[derive(Debug, Clone)]
struct Kept {
    handle: String,
    tool: String,
    /// Le contenu **entier**, gardé une fois. Toutes les formes réduites en
    /// sont dérivées, jamais dérivées les unes des autres : c'est ce qui rend
    /// [`Session::absorb`] idempotent et réversible.
    content: String,
    /// Le tour où il est arrivé.
    turn: usize,
}

#[derive(Debug, Default)]
struct Inner {
    policy: Absorb,
    turn: usize,
    /// `tool_call_id → Kept`. L'identifiant d'appel est la seule clé stable
    /// ici : les indices de `turns` bougent, les contenus sont réécrits.
    by_call: BTreeMap<String, Kept>,
    /// `#read-2 → tool_call_id`.
    by_handle: BTreeMap<String, String>,
    /// Le compteur par nom d'outil, pour numéroter les renvois.
    counters: BTreeMap<String, usize>,
}

/// Ce qu'un passage d'[`Session::absorb`] a changé, en caractères.
///
/// Rendu pour être **dit** : une politique qui jette la moitié d'un
/// historique sans le tracer se débogue à l'aveugle (doc 13 §8).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Compaction {
    /// Combien de résultats ont été réécrits ce tour-ci.
    pub rewritten: usize,
    /// Caractères présents dans l'historique après passage.
    pub kept: usize,
    /// Caractères retirés de l'historique — gardés dans la session.
    pub dropped: usize,
}

impl Compaction {
    pub fn is_noop(&self) -> bool {
        self.rewritten == 0
    }
}

/// L'état qui survit au tour : la politique, le compteur, la table de
/// renvois.
///
/// Partagée par `Arc` et mutable derrière un `Mutex`, comme
/// [`crate::postures::Postures`] : plusieurs mains y touchent — la boucle
/// pour absorber, l'outillage pour résoudre un renvoi.
#[derive(Debug, Default)]
pub struct Session {
    inner: Mutex<Inner>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// La politique d'absorption. Sans appel, [`Absorb::Whole`].
    pub fn with_policy(self, policy: Absorb) -> Self {
        if let Ok(mut g) = self.inner.lock() {
            g.policy = policy;
        }
        self
    }

    pub fn policy(&self) -> Absorb {
        self.inner.lock().map(|g| g.policy).unwrap_or_default()
    }

    /// Un tour de plus. Rend le numéro du tour qui commence.
    pub fn advance(&self) -> usize {
        match self.inner.lock() {
            Ok(mut g) => {
                g.turn += 1;
                g.turn
            }
            Err(_) => 0,
        }
    }

    pub fn turn(&self) -> usize {
        self.inner.lock().map(|g| g.turn).unwrap_or(0)
    }

    /// Le contenu entier derrière un renvoi.
    ///
    /// Accepte `#read-2` comme `read-2` : un modèle qui recopie un nom ne
    /// recopie pas toujours la ponctuation, et refuser pour un dièse serait
    /// une sévérité sans objet.
    pub fn recall(&self, handle: &str) -> Option<String> {
        let key = handle.trim();
        let key = key.strip_prefix('#').unwrap_or(key);
        let g = self.inner.lock().ok()?;
        let id = g.by_handle.get(&format!("#{key}"))?;
        g.by_call.get(id).map(|k| k.content.clone())
    }

    /// Les renvois attribués, dans l'ordre : `(handle, outil, caractères)`.
    pub fn handles(&self) -> Vec<(String, String, usize)> {
        self.inner
            .lock()
            .map(|g| {
                g.by_handle
                    .iter()
                    .filter_map(|(h, id)| g.by_call.get(id).map(|k| (h.clone(), k.tool.clone(), k.content.len())))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// **Réduit l'historique** selon la politique, en gardant tout à côté.
    ///
    /// Idempotent : chaque forme est dérivée du contenu entier mémorisé, donc
    /// deux passages au même tour rendent le même texte, et un passage à un
    /// tour plus tardif ne peut que réduire davantage.
    pub fn absorb(&self, turns: &mut [Turn]) -> Compaction {
        let Ok(mut g) = self.inner.lock() else {
            return Compaction::default();
        };
        let policy = g.policy;
        let now = g.turn;
        let mut out = Compaction::default();

        for turn in turns.iter_mut() {
            let Some(id) = turn.tool_call_id.clone() else {
                continue;
            };
            // Première rencontre : on met de côté le contenu entier et on
            // attribue un renvoi. Un résultat vu au tour 2 reste daté du
            // tour 2, même si on le relit dix fois.
            if !g.by_call.contains_key(&id) {
                let tool = turn.tool_name.clone().unwrap_or_else(|| "tool".to_string());
                let n = g.counters.entry(tool.clone()).and_modify(|n| *n += 1).or_insert(1);
                let handle = format!("#{tool}-{n}");
                g.by_handle.insert(handle.clone(), id.clone());
                g.by_call.insert(
                    id.clone(),
                    Kept { handle, tool, content: std::mem::take(&mut turn.content), turn: now },
                );
                // `content` a été pris : on le remet sous sa forme du moment.
                let kept = &g.by_call[&id];
                turn.content = render(kept, policy, now);
                out.kept += turn.content.len();
                if turn.content.len() < kept.content.len() {
                    out.rewritten += 1;
                    out.dropped += kept.content.len() - turn.content.len();
                }
                continue;
            }

            let kept = &g.by_call[&id];
            let form = render(kept, policy, now);
            if form != turn.content {
                out.rewritten += 1;
                out.dropped += turn.content.len().saturating_sub(form.len());
                turn.content = form;
            }
            out.kept += turn.content.len();
        }
        out
    }
}

/// La forme d'un résultat, au tour `now`, sous `policy`.
///
/// Toujours calculée depuis `kept.content` — jamais depuis ce qui est
/// actuellement dans l'historique.
fn render(kept: &Kept, policy: Absorb, now: usize) -> String {
    let total = kept.content.chars().count();
    match policy {
        Absorb::Whole => kept.content.clone(),
        Absorb::Bounded { max_chars } => bounded(kept, max_chars, total),
        Absorb::Stale { max_chars, after_turns } => {
            if now.saturating_sub(kept.turn) >= after_turns {
                stale(kept, total)
            } else {
                bounded(kept, max_chars, total)
            }
        }
    }
}

fn bounded(kept: &Kept, max_chars: usize, total: usize) -> String {
    if total <= max_chars {
        return kept.content.clone();
    }
    let cut = kept.content.char_indices().nth(max_chars).map(|(i, _)| i).unwrap_or(kept.content.len());
    let head = &kept.content[..cut];
    format!("{head}\n…\n[{} — {total} caractères en tout, `recall(\"{}\")` pour la suite]", kept.handle, kept.handle)
}

fn stale(kept: &Kept, total: usize) -> String {
    format!("[{} — {}, {total} caractères, `recall(\"{}\")` pour le relire]", kept.handle, kept.tool, kept.handle)
}

/// Le nom de l'outil qui résout un renvoi.
pub const RECALL_TOOL: &str = "recall";

/// L'outillage **plus** `recall`.
///
/// Enveloppe une boîte existante : la session ne remplace pas les outils, elle
/// en ajoute un. C'est ce qui permet d'absorber sans rien perdre — le modèle
/// peut toujours revenir chercher ce qu'on a mis de côté.
pub struct SessionTools<'a> {
    inner: &'a (dyn crate::agent::ToolBox + Sync),
    session: std::sync::Arc<Session>,
}

impl<'a> SessionTools<'a> {
    pub fn new(inner: &'a (dyn crate::agent::ToolBox + Sync), session: std::sync::Arc<Session>) -> Self {
        Self { inner, session }
    }

    fn def() -> ToolDef {
        ToolDef {
            name: RECALL_TOOL.to_string(),
            description: "Relit en entier un résultat d'outil abrégé dans l'historique. \
                          L'argument est le renvoi affiché à sa place, par exemple \"#read-2\"."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "handle": { "type": "string", "description": "Le renvoi, par exemple \"#read-2\"." }
                },
                "required": ["handle"],
            }),
        }
    }
}

impl crate::agent::ToolBox for SessionTools<'_> {
    fn call(&self, call: &ToolCall) -> Turn {
        if call.name != RECALL_TOOL {
            return self.inner.call(call);
        }
        Turn::tool_result(call.id.clone(), RECALL_TOOL, self.resolve(&call.arguments))
    }

    fn call_in(&self, call: &ToolCall, run: &str) -> Turn {
        if call.name != RECALL_TOOL {
            return self.inner.call_in(call, run);
        }
        Turn::tool_result(call.id.clone(), RECALL_TOOL, self.resolve(&call.arguments))
    }

    fn tool_defs(&self) -> Vec<ToolDef> {
        let mut defs = self.inner.tool_defs();
        defs.push(SessionTools::def());
        defs
    }

    fn is_async(&self, tool: &str) -> bool {
        tool != RECALL_TOOL && self.inner.is_async(tool)
    }
}

impl SessionTools<'_> {
    /// **Ne peut pas échouer** : un renvoi inconnu rend un texte qui le dit,
    /// avec ceux qui existent — c'est le contrat de `ToolBox`, et c'est ce
    /// qui permet au modèle de se rattraper tout seul.
    fn resolve(&self, arguments: &str) -> String {
        let handle = serde_json::from_str::<serde_json::Value>(arguments)
            .ok()
            .and_then(|v| v.get("handle").and_then(|h| h.as_str()).map(str::to_string));
        let Some(handle) = handle else {
            return "recall : il faut un argument `handle`, par exemple {\"handle\": \"#read-2\"}.".to_string();
        };
        match self.session.recall(&handle) {
            Some(content) => content,
            None => {
                let known: Vec<String> = self.session.handles().into_iter().map(|(h, _, _)| h).collect();
                if known.is_empty() {
                    format!("recall : renvoi « {handle} » inconnu, et aucun n'a encore été attribué.")
                } else {
                    format!("recall : renvoi « {handle} » inconnu. Connus : {}.", known.join(", "))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::ToolBox;

    fn result(id: &str, tool: &str, content: &str) -> Turn {
        Turn::tool_result(id, tool, content)
    }

    fn long(n: usize) -> String {
        "x".repeat(n)
    }

    #[test]
    fn le_defaut_ne_touche_a_rien() {
        // La seule promesse qui compte avant toutes les autres : exister ne
        // change rien.
        let s = Session::new();
        let mut turns = vec![Turn::user("va voir"), result("c1", "read", &long(50_000))];
        let c = s.absorb(&mut turns);
        assert!(c.is_noop(), "{c:?}");
        assert_eq!(turns[1].content.len(), 50_000);
    }

    #[test]
    fn un_gros_resultat_est_borne_des_son_arrivee() {
        let s = Session::new().with_policy(Absorb::Bounded { max_chars: 100 });
        let mut turns = vec![result("c1", "read", &long(5_000))];
        let c = s.absorb(&mut turns);
        assert_eq!(c.rewritten, 1);
        assert!(turns[0].content.starts_with(&long(100)));
        assert!(turns[0].content.contains("#read-1"), "{}", turns[0].content);
        // Rien n'est perdu : le renvoi rend l'original.
        assert_eq!(s.recall("#read-1").unwrap().len(), 5_000);
    }

    #[test]
    fn un_petit_resultat_reste_entier() {
        let s = Session::new().with_policy(Absorb::Bounded { max_chars: 100 });
        let mut turns = vec![result("c1", "read", "trois lignes")];
        let c = s.absorb(&mut turns);
        assert!(c.is_noop(), "{c:?}");
        assert_eq!(turns[0].content, "trois lignes");
    }

    #[test]
    fn ce_qui_vieillit_devient_une_ligne() {
        let s = Session::new().with_policy(Absorb::Stale { max_chars: 10_000, after_turns: 3 });
        let mut turns = vec![result("c1", "read", &long(4_000))];
        // Tour 0 : il vient d'arriver, il sert, on le garde.
        assert!(s.absorb(&mut turns).is_noop());
        assert_eq!(turns[0].content.len(), 4_000);

        for _ in 0..3 {
            s.advance();
        }
        let c = s.absorb(&mut turns);
        assert_eq!(c.rewritten, 1);
        assert!(turns[0].content.len() < 120, "{}", turns[0].content);
        assert!(turns[0].content.contains("4000 caractères"), "{}", turns[0].content);
        assert!(turns[0].content.contains("recall"), "{}", turns[0].content);
        assert_eq!(c.dropped, 4_000 - turns[0].content.len());
    }

    #[test]
    fn absorber_deux_fois_ne_ronge_pas() {
        // La forme est toujours dérivée du contenu entier : sans ça, chaque
        // passage tronquerait la troncature, et l'historique fondrait.
        let s = Session::new().with_policy(Absorb::Bounded { max_chars: 100 });
        let mut turns = vec![result("c1", "read", &long(5_000))];
        s.absorb(&mut turns);
        let apres_un = turns[0].content.clone();
        let c = s.absorb(&mut turns);
        assert!(c.is_noop(), "{c:?}");
        assert_eq!(turns[0].content, apres_un);
    }

    #[test]
    fn un_resultat_garde_le_tour_ou_il_est_arrive() {
        // Sinon un vieux résultat rajeunirait à chaque passage et ne
        // périmerait jamais.
        let s = Session::new().with_policy(Absorb::Stale { max_chars: 10_000, after_turns: 2 });
        let mut turns = vec![result("c1", "read", &long(4_000))];
        s.absorb(&mut turns);
        s.advance();
        s.absorb(&mut turns);
        assert_eq!(turns[0].content.len(), 4_000, "encore frais au tour 1");
        s.advance();
        s.absorb(&mut turns);
        assert!(turns[0].content.len() < 120, "périmé au tour 2");
    }

    #[test]
    fn les_renvois_sont_numerotes_par_outil() {
        let s = Session::new().with_policy(Absorb::Bounded { max_chars: 10 });
        let mut turns = vec![
            result("c1", "read", &long(500)),
            result("c2", "grep", &long(500)),
            result("c3", "read", &long(500)),
        ];
        s.absorb(&mut turns);
        let noms: Vec<String> = s.handles().into_iter().map(|(h, _, _)| h).collect();
        assert_eq!(noms, vec!["#grep-1", "#read-1", "#read-2"]);
    }

    #[test]
    fn le_diese_est_facultatif() {
        let s = Session::new().with_policy(Absorb::Bounded { max_chars: 10 });
        let mut turns = vec![result("c1", "read", &long(500))];
        s.absorb(&mut turns);
        assert_eq!(s.recall("read-1"), s.recall("#read-1"));
        assert_eq!(s.recall(" #read-1 "), s.recall("#read-1"));
    }

    #[test]
    fn couper_ne_casse_pas_un_caractere() {
        let s = Session::new().with_policy(Absorb::Bounded { max_chars: 5 });
        let mut turns = vec![result("c1", "read", "éàüéàüéàüéàü")];
        s.absorb(&mut turns);
        assert!(turns[0].content.starts_with("éàüéà"), "{}", turns[0].content);
    }

    struct Rien;
    impl ToolBox for Rien {
        fn call(&self, call: &ToolCall) -> Turn {
            Turn::tool_result(call.id.clone(), call.name.clone(), "rien")
        }
        fn tool_defs(&self) -> Vec<ToolDef> {
            vec![ToolDef { name: "read".into(), description: String::new(), parameters: serde_json::json!({}) }]
        }
    }

    fn appel(name: &str, arguments: &str) -> ToolCall {
        ToolCall::new("c9", name, arguments)
    }

    #[test]
    fn recall_rend_le_contenu_entier() {
        let s = std::sync::Arc::new(Session::new().with_policy(Absorb::Bounded { max_chars: 10 }));
        let mut turns = vec![result("c1", "read", &long(500))];
        s.absorb(&mut turns);

        let tools = SessionTools::new(&Rien, s.clone());
        let out = tools.call(&appel(RECALL_TOOL, r##"{"handle":"#read-1"}"##));
        assert_eq!(out.content.len(), 500);
        assert_eq!(out.tool_call_id.as_deref(), Some("c9"));
    }

    #[test]
    fn un_renvoi_inconnu_dit_ceux_qui_existent() {
        // Un outil ne peut pas échouer : il rend de quoi se rattraper.
        let s = std::sync::Arc::new(Session::new().with_policy(Absorb::Bounded { max_chars: 10 }));
        let mut turns = vec![result("c1", "read", &long(500))];
        s.absorb(&mut turns);

        let tools = SessionTools::new(&Rien, s.clone());
        let out = tools.call(&appel(RECALL_TOOL, r##"{"handle":"#read-9"}"##));
        assert!(out.content.contains("#read-1"), "{}", out.content);
    }

    #[test]
    fn recall_s_ajoute_sans_rien_retirer() {
        let s = std::sync::Arc::new(Session::new());
        let tools = SessionTools::new(&Rien, s);
        let noms: Vec<String> = tools.tool_defs().into_iter().map(|d| d.name).collect();
        assert_eq!(noms, vec!["read", RECALL_TOOL]);
    }

    #[test]
    fn les_autres_appels_passent_au_travers() {
        let s = std::sync::Arc::new(Session::new());
        let tools = SessionTools::new(&Rien, s);
        let out = tools.call(&appel("read", "{}"));
        assert_eq!(out.content, "rien");
    }
}
