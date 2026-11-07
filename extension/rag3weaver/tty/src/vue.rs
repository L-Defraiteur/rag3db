//! **La vue inline** (lot tty-2). Un modèle pur : des événements de l'hôte et des
//! touches entrent ; en sortent des lignes **validées**, qui partent dans
//! l'historique du terminal (`insert_before`), et une **zone vivante** de trois
//! lignes, redessinée sur place : la ligne en cours d'écriture, l'état, la
//! saisie. L'historique reste celui du terminal : le défilement, la copie, tmux
//! et mosh continuent de marcher, et seul le bas de l'écran se redessine.

use crate::anim::{self, Capacites, Glyphes, Niveau, Quelle};
use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthChar;

/// La hauteur de la zone vivante.
pub const HAUTEUR: u16 = 3;

/// Ce que fait l'agent, d'après les événements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Etat {
    /// Rien ne tourne : on attend une demande ou l'équipier attend la boîte.
    Attend,
    /// Un appel au modèle est parti, aucun token n'est encore venu.
    Reflechit,
    /// Les tokens arrivent.
    Ecrit,
    /// Un outil tourne.
    Agit(String),
    /// L'équipier a fini (`ended`).
    Fini(String),
}

/// Ce qu'une touche demande à la boucle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Écrire à l'équipier : le texte, et son genre (une `request` la première
    /// fois, qui ouvre la tâche ; une `instruction` ensuite).
    Envoyer(String, &'static str),
    /// Arrêter le pas en cours (`stop`).
    Arreter,
    /// Quitter le terminal (l'équipier continue).
    Quitter,
}

/// Une touche, indépendante du backend (les tests n'ont pas de terminal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Touche {
    Car(char),
    Entree,
    Retour,
    Echap,
    CtrlC,
    CtrlD,
}

/// Le style d'une ligne validée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Genre {
    Texte,
    Outil,
    Boite,
    Note,
}

/// Une ligne validée, prête pour l'historique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validee {
    pub genre: Genre,
    pub texte: String,
}

impl Validee {
    pub fn line(&self) -> Line<'static> {
        let style = match self.genre {
            Genre::Texte => Style::default(),
            Genre::Outil => Style::default().fg(Color::Red),
            Genre::Boite => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            Genre::Note => Style::default().add_modifier(Modifier::DIM),
        };
        Line::from(Span::styled(self.texte.clone(), style))
    }
}

/// Le modèle de la vue.
#[derive(Debug)]
pub struct Vue {
    largeur: u16,
    en_cours: String,
    validees: Vec<Validee>,
    pub etat: Etat,
    saisie: String,
    demande_posee: bool,
    pub theme: Theme,
    pub capacites: Capacites,
    /// La graine du Glitch (fixée dans les tests).
    pub graine: u64,
    /// Ce que chaque outil a le droit de faire : il lit, ou il écrit ou exécute
    /// (la `policy` de `describe`). Un outil inconnu lit.
    politiques: HashMap<String, Quelle>,
    /// Depuis quand l'animation en cours tourne.
    depuis: Instant,
    /// « dépôt · branche · tâche », jamais l'id d'un backend.
    pub entete: String,
}

/// La largeur d'un texte en colonnes.
pub fn colonnes(s: &str) -> usize {
    s.chars().map(|c| c.width().unwrap_or(0)).sum()
}

impl Vue {
    pub fn new(largeur: u16, demande_posee: bool) -> Self {
        Self {
            largeur: largeur.max(8),
            en_cours: String::new(),
            validees: Vec::new(),
            etat: Etat::Attend,
            saisie: String::new(),
            demande_posee,
            theme: Theme::daemon(),
            capacites: Capacites { couleurs: Niveau::Vraie, glyphes: Glyphes::Surs },
            graine: 0x5eed,
            politiques: HashMap::new(),
            depuis: Instant::now(),
            entete: String::new(),
        }
    }

    /// Change d'état ; l'animation repart de sa première image quand elle change.
    fn changer(&mut self, e: Etat) {
        let avant = self.quelle();
        self.etat = e;
        if self.quelle() != avant {
            self.depuis = Instant::now();
        }
    }

    /// Les politiques des outils, depuis la réponse de `describe` (`{tools: [{name, policy}]}`).
    pub fn politiques(&mut self, describe: &Value) {
        let outils = describe["tools"].as_array().or_else(|| describe["result"]["tools"].as_array()).cloned().unwrap_or_default();
        for o in outils {
            let (Some(nom), p) = (o["name"].as_str(), &o["policy"]) else { continue };
            let ecrit = p["write_files"] == true || p["run_commands"] == true;
            self.politiques.insert(nom.to_string(), if ecrit { Quelle::Ecrit } else { Quelle::Lit });
        }
    }

    /// L'animation de l'état, s'il en a une.
    pub fn quelle(&self) -> Option<Quelle> {
        match &self.etat {
            Etat::Reflechit => Some(Quelle::Reflechit),
            Etat::Agit(o) => Some(*self.politiques.get(o).unwrap_or(&Quelle::Lit)),
            _ => None,
        }
    }

    /// Le numéro de l'image à `maintenant`.
    pub fn image(&self, maintenant: Instant) -> u64 {
        let ms = self.quelle().map(|q| q.de(&self.theme).ms).unwrap_or(1).max(1);
        (maintenant.saturating_duration_since(self.depuis).as_millis() as u64) / ms
    }

    /// Le temps jusqu'à l'image suivante, s'il y a une animation.
    pub fn prochaine_image(&self, maintenant: Instant) -> Option<Duration> {
        let ms = self.quelle()?.de(&self.theme).ms.max(1);
        let ecoule = maintenant.saturating_duration_since(self.depuis).as_millis() as u64;
        Some(Duration::from_millis(ms - ecoule % ms))
    }

    pub fn redimensionner(&mut self, largeur: u16) {
        self.largeur = largeur.max(8);
        self.couper();
    }

    /// Les lignes validées depuis le dernier appel, dans l'ordre.
    pub fn prendre_lignes(&mut self) -> Vec<Validee> {
        std::mem::take(&mut self.validees)
    }

    fn valider(&mut self, genre: Genre, texte: String) {
        self.validees.push(Validee { genre, texte });
    }

    /// Clôt la ligne en cours, si elle a quelque chose.
    fn clore(&mut self) {
        if !self.en_cours.is_empty() {
            let t = std::mem::take(&mut self.en_cours);
            self.valider(Genre::Texte, t);
        }
    }

    /// Coupe la ligne en cours à la largeur : ce qui déborde part valider.
    fn couper(&mut self) {
        let max = self.largeur as usize;
        while colonnes(&self.en_cours) > max {
            let mut l = 0;
            let mut coupe = self.en_cours.len();
            for (i, c) in self.en_cours.char_indices() {
                let w = c.width().unwrap_or(0);
                if l + w > max {
                    coupe = i;
                    break;
                }
                l += w;
            }
            // De préférence au dernier blanc avant la coupe (un mot entier).
            let tete = &self.en_cours[..coupe];
            let coupe = match tete.rfind(' ') {
                Some(b) if b > 0 => b + 1,
                _ => coupe,
            };
            let reste = self.en_cours.split_off(coupe);
            let ligne = std::mem::replace(&mut self.en_cours, reste);
            self.valider(Genre::Texte, ligne.trim_end().to_string());
        }
    }

    /// Du texte du modèle, au fil de l'eau.
    pub fn texte(&mut self, t: &str) {
        let mut morceaux = t.split('\n');
        if let Some(premier) = morceaux.next() {
            self.en_cours.push_str(premier);
            self.couper();
        }
        for m in morceaux {
            let t = std::mem::take(&mut self.en_cours);
            self.valider(Genre::Texte, t);
            self.en_cours.push_str(m);
            self.couper();
        }
    }

    /// Une trame de `/ide/flux` (`{evenement: {kind, …}}`).
    pub fn trame(&mut self, enveloppe: &Value) {
        let e = &enveloppe["evenement"];
        match e["kind"].as_str().unwrap_or_default() {
            "LlmCall" => self.changer(Etat::Reflechit),
            "Token" => {
                self.changer(Etat::Ecrit);
                if let Some(t) = e["text"].as_str() {
                    self.texte(t);
                }
            }
            "ToolCallStarted" => {
                self.clore();
                let outil = e["tool"].as_str().unwrap_or("?").to_string();
                self.valider(Genre::Outil, format!("» {outil}"));
                self.changer(Etat::Agit(outil));
            }
            "ToolCallFinished" => {
                let dit = if e["ok"] == true { format!("  fait ({} ms)", e["ms"].as_u64().unwrap_or(0)) } else { format!("  refusé ({})", e["error_kind"].as_str().unwrap_or("erreur")) };
                self.valider(Genre::Outil, dit);
                self.changer(Etat::Reflechit);
            }
            "RunFinished" => {
                self.clore();
                if !matches!(self.etat, Etat::Fini(_)) {
                    self.changer(Etat::Attend);
                }
            }
            "MailboxLine" => self.ligne_de_boite(&e["line"]),
            _ => {}
        }
    }

    /// Une ligne de la boîte de l'équipier.
    pub fn ligne_de_boite(&mut self, l: &Value) {
        self.clore();
        let item = l["item"].as_str().filter(|i| !i.is_empty()).map(|i| format!(" [{i}]")).unwrap_or_default();
        let tete = format!("[{} · {}{item}] ", l["from"].as_str().unwrap_or("?"), l["kind"].as_str().unwrap_or("?"));
        let texte = l["text"].as_str().unwrap_or("");
        for (i, ligne) in texte.split('\n').enumerate() {
            self.valider(Genre::Boite, if i == 0 { format!("{tete}{ligne}") } else { format!("  {ligne}") });
        }
        if l["kind"] == "ended" {
            self.changer(Etat::Fini(texte.to_string()));
        }
    }

    /// Une note du terminal lui-même (une coupure du flux, un refus).
    pub fn note(&mut self, t: &str) {
        self.clore();
        self.valider(Genre::Note, format!("[tty] {t}"));
    }

    /// Une touche : rend l'action qu'elle demande.
    pub fn touche(&mut self, t: Touche) -> Option<Action> {
        match t {
            Touche::Car(c) => {
                self.saisie.push(c);
                None
            }
            Touche::Retour => {
                self.saisie.pop();
                None
            }
            Touche::Entree => {
                let texte = std::mem::take(&mut self.saisie);
                if texte.trim().is_empty() {
                    return None;
                }
                let genre = self.genre_d_envoi();
                self.demande_posee = true;
                self.clore();
                self.valider(Genre::Boite, format!("› {texte}"));
                Some(Action::Envoyer(texte, genre))
            }
            Touche::Echap => Some(Action::Arreter),
            Touche::CtrlC => Some(Action::Quitter),
            Touche::CtrlD if self.saisie.is_empty() => Some(Action::Quitter),
            Touche::CtrlD => None,
        }
    }

    /// Le genre de ligne de boîte qu'une saisie envoie.
    pub fn genre_d_envoi(&self) -> &'static str {
        if self.demande_posee { "instruction" } else { "request" }
    }

    /// Le cœur du thème, tel que le terminal sait le montrer.
    fn coeur(&self) -> String {
        match self.capacites.glyphes {
            Glyphes::Ascii => self.theme.ascii.coeur.clone(),
            _ => self.theme.animations.reflechit.coeur.clone(),
        }
    }

    /// Le texte de la ligne d'état, sans son animation.
    pub fn ligne_d_etat(&self) -> String {
        let c = self.coeur();
        match &self.etat {
            Etat::Attend => format!("{c} attend"),
            Etat::Reflechit => format!("{c} réfléchit…"),
            Etat::Ecrit => format!("{c} écrit"),
            Etat::Agit(o) => format!("{c} agit : {o}"),
            Etat::Fini(pourquoi) => format!("{c} fini : {pourquoi}"),
        }
    }

    /// La ligne d'état à `maintenant` : l'animation quand l'état en a une, puis
    /// l'en-tête en sourdine à droite s'il tient.
    pub fn ligne_vivante(&self, largeur: usize, maintenant: Instant) -> Line<'static> {
        let mut ligne = match self.quelle() {
            Some(q) => {
                let suite = if let Etat::Agit(o) = &self.etat { o.clone() } else { String::new() };
                anim::ligne(&self.theme, q, self.image(maintenant), self.graine, self.capacites, &suite)
            }
            None => {
                let rouge = anim::couleur(&self.theme.couleurs.texte, self.capacites.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default().add_modifier(Modifier::BOLD);
                Line::from(Span::styled(self.ligne_d_etat(), rouge))
            }
        };
        let occupe: usize = ligne.spans.iter().map(|s| colonnes(&s.content)).sum();
        let entete = colonnes(&self.entete);
        if !self.entete.is_empty() && occupe + 2 + entete <= largeur {
            let sourdine = anim::couleur(&self.theme.couleurs.sourdine, self.capacites.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default();
            ligne.spans.push(Span::raw(" ".repeat(largeur - occupe - entete)));
            ligne.spans.push(Span::styled(self.entete.clone(), sourdine));
        }
        ligne
    }

    /// Dessine la zone vivante dans `zone` (trois lignes), à l'heure qu'il est.
    pub fn dessiner(&self, f: &mut Frame, zone: Rect) {
        self.dessiner_a(f, zone, Instant::now())
    }

    /// Dessine la zone vivante telle qu'elle est à `maintenant`.
    pub fn dessiner_a(&self, f: &mut Frame, zone: Rect, maintenant: Instant) {
        let largeur = zone.width as usize;
        // La fin de la ligne en cours, si elle déborde (elle ne déborde pas : `couper`).
        let en_cours = queue(&self.en_cours, largeur);
        let saisie = queue(&format!("› {}", self.saisie), largeur.saturating_sub(1));
        let lignes = vec![Line::from(en_cours), self.ligne_vivante(largeur, maintenant), Line::from(saisie.clone())];
        f.render_widget(Paragraph::new(lignes), zone);
        let x = zone.x + (colonnes(&saisie) as u16).min(zone.width.saturating_sub(1));
        f.set_cursor_position((x, zone.y + 2));
    }
}

/// La fin de `s` qui tient dans `max` colonnes.
fn queue(s: &str, max: usize) -> String {
    let mut l = 0;
    let mut garde: Vec<char> = Vec::new();
    for c in s.chars().rev() {
        let w = c.width().unwrap_or(0);
        if l + w > max {
            break;
        }
        l += w;
        garde.push(c);
    }
    garde.into_iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::{Terminal, TerminalOptions, Viewport};
    use serde_json::json;

    fn tok(t: &str) -> Value {
        json!({ "evenement": { "kind": "Token", "text": t } })
    }

    #[test]
    fn streamed_text_validates_whole_lines_and_wraps_at_the_width() {
        let mut v = Vue::new(20, true);
        for t in ["Bonjour", " le", " monde.\nUne", " seconde", " ligne", " qui", " déborde", " de", " vingt."] {
            v.trame(&tok(t));
        }
        let lignes: Vec<String> = v.prendre_lignes().into_iter().map(|l| l.texte).collect();
        assert_eq!(lignes, vec!["Bonjour le monde.", "Une seconde ligne", "qui déborde de"]);
        assert_eq!(v.en_cours, "vingt.");
        v.redimensionner(4);
        assert_eq!(v.prendre_lignes().into_iter().map(|l| l.texte).collect::<Vec<_>>(), Vec::<String>::new(), "« vingt. » fait 6 colonnes, la largeur plancher est 8");
    }

    #[test]
    fn the_states_follow_the_events_and_the_mailbox_ends() {
        let mut v = Vue::new(80, false);
        assert_eq!(v.genre_d_envoi(), "request");
        v.trame(&json!({ "evenement": { "kind": "LlmCall" } }));
        assert_eq!(v.etat, Etat::Reflechit);
        v.trame(&tok("salut"));
        assert_eq!(v.etat, Etat::Ecrit);
        v.trame(&json!({ "evenement": { "kind": "ToolCallStarted", "tool": "read_file" } }));
        assert_eq!(v.etat, Etat::Agit("read_file".into()));
        v.trame(&json!({ "evenement": { "kind": "ToolCallFinished", "ok": true, "ms": 3 } }));
        v.ligne_de_boite(&json!({ "from": "host", "kind": "ended", "text": "fait sans plan" }));
        assert_eq!(v.etat, Etat::Fini("fait sans plan".into()));
        let lignes: Vec<String> = v.prendre_lignes().into_iter().map(|l| l.texte).collect();
        assert_eq!(lignes, vec!["salut", "» read_file", "  fait (3 ms)", "[host · ended] fait sans plan"]);
    }

    #[test]
    fn the_animation_follows_the_state_and_the_tool_policy() {
        let mut v = Vue::new(120, true);
        v.politiques(&json!({ "tools": [
            { "name": "read_file", "policy": { "read_files": true } },
            { "name": "edit_file", "policy": { "write_files": true } },
            { "name": "run_command", "policy": { "run_commands": true } }
        ] }));
        let t0 = Instant::now();
        let ligne = |v: &Vue, ms: u64| v.ligne_vivante(120, v.depuis + Duration::from_millis(ms)).spans.iter().map(|s| s.content.to_string()).collect::<String>();
        assert_eq!(v.quelle(), None);
        assert!(ligne(&v, 0).starts_with("⛧ attend"));
        v.trame(&json!({ "evenement": { "kind": "LlmCall" } }));
        assert_eq!(v.quelle(), Some(Quelle::Reflechit));
        assert!(ligne(&v, 0).starts_with("⟪ ᚛ᚨᛒ ⛧ ᚜⸸‡ ⟫ INVOCATION DU CODE..."));
        assert!(ligne(&v, 150).starts_with("⟪ ᚨᛒᛟ ⛧ ⸸‡᚛ ⟫"), "une image toutes les 150 ms");
        assert_eq!(v.prochaine_image(v.depuis + Duration::from_millis(40)), Some(Duration::from_millis(110)));
        for (outil, attendu) in [("read_file", Quelle::Lit), ("edit_file", Quelle::Ecrit), ("run_command", Quelle::Ecrit), ("inconnu", Quelle::Lit)] {
            v.trame(&json!({ "evenement": { "kind": "ToolCallStarted", "tool": outil } }));
            assert_eq!(v.quelle(), Some(attendu), "{outil}");
            assert!(ligne(&v, 0).ends_with(outil) || ligne(&v, 0).contains(&format!(" {outil}")), "le nom de l'outil suit l'animation : {}", ligne(&v, 0));
        }
        v.trame(&json!({ "evenement": { "kind": "Token", "text": "x" } }));
        assert_eq!(v.quelle(), None, "les tokens coulent : pas d'animation");
        assert_eq!(v.prochaine_image(t0), None);
    }

    #[test]
    fn the_header_says_repo_branch_and_task_on_the_right_when_it_fits() {
        let mut v = Vue::new(60, true);
        v.entete = "rag3db · tty-3 · tâche parle".into();
        let texte = |v: &Vue, l: usize| v.ligne_vivante(l, v.depuis).spans.iter().map(|s| s.content.to_string()).collect::<String>();
        let large = texte(&v, 60);
        assert!(large.starts_with("⛧ attend") && large.ends_with("rag3db · tty-3 · tâche parle"), "{large}");
        assert_eq!(colonnes(&large), 60);
        assert_eq!(texte(&v, 20), "⛧ attend", "trop étroit : l'en-tête se tait");
        assert!(!large.contains("backend"));
    }

    #[test]
    fn keys_type_send_stop_and_quit() {
        let mut v = Vue::new(80, false);
        for c in "fais ça".chars() {
            assert_eq!(v.touche(Touche::Car(c)), None);
        }
        v.touche(Touche::Retour);
        assert_eq!(v.touche(Touche::Entree), Some(Action::Envoyer("fais ç".into(), "request")), "la première saisie ouvre la tâche");
        assert_eq!(v.touche(Touche::Entree), None, "une saisie vide n'envoie rien");
        v.touche(Touche::Car('x'));
        assert_eq!(v.touche(Touche::Entree), Some(Action::Envoyer("x".into(), "instruction")), "les suivantes lui parlent");
        assert_eq!(v.touche(Touche::Echap), Some(Action::Arreter));
        assert_eq!(v.touche(Touche::CtrlD), Some(Action::Quitter));
    }

    fn rendu(largeur: u16, hauteur: u16, v: &mut Vue) -> Vec<String> {
        let mut t = Terminal::with_options(TestBackend::new(largeur, hauteur), TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }).unwrap();
        let lignes = v.prendre_lignes();
        if !lignes.is_empty() {
            t.insert_before(lignes.len() as u16, |buf| {
                Paragraph::new(lignes.iter().map(Validee::line).collect::<Vec<_>>()).render(buf.area, buf);
            })
            .unwrap();
        }
        t.draw(|f| v.dessiner(f, f.area())).unwrap();
        let buf = t.backend().buffer();
        (0..buf.area.height).map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string()).collect()
    }

    use ratatui::widgets::Widget;

    #[test]
    fn the_inline_view_puts_history_above_and_the_live_zone_below_at_80x24_and_40x10() {
        for (l, h) in [(80u16, 24u16), (40, 10)] {
            let mut v = Vue::new(l, true);
            v.trame(&tok("Première ligne.\nEn cours"));
            v.trame(&json!({ "evenement": { "kind": "LlmCall" } }));
            for c in "ok".chars() {
                v.touche(Touche::Car(c));
            }
            let ecran = rendu(l, h, &mut v);
            let non_vides: Vec<&String> = ecran.iter().filter(|s| !s.is_empty()).collect();
            assert_eq!(non_vides.len(), 4, "{l}×{h} : {ecran:#?}");
            assert_eq!((non_vides[0].as_str(), non_vides[1].as_str(), non_vides[3].as_str()), ("Première ligne.", "En cours", "› ok"), "{l}×{h} : {ecran:#?}");
            assert!(non_vides[2].contains("INVOCATION DU CODE"), "le modèle réfléchit : le Cercle — {l}×{h} : {ecran:#?}");
        }
    }
}
