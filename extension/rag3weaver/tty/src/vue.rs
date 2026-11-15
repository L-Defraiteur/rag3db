//! **La vue inline** (lot tty-2). Un modèle pur : des événements de l'hôte et des
//! touches entrent ; en sortent des lignes **validées**, qui partent dans
//! l'historique du terminal (`insert_before`), et une **zone vivante** de trois
//! lignes, redessinée sur place : la ligne en cours d'écriture, l'état, la
//! saisie. L'historique reste celui du terminal : le défilement, la copie, tmux
//! et mosh continuent de marcher, et seul le bas de l'écran se redessine.

use crate::anim::{self, Capacites, Glyphes, Niveau, Quelle};
use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
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
    /// Appliquer (`o`) ou rejeter (`n`) une proposition de déclarations (tty-16).
    Proposition { backend: String, id: u64, appliquer: bool },
    /// Une commande du terminal (`/…` ou `!…`), jamais envoyée à l'équipier (tty-13).
    Commande(crate::commandes::Commande),
    /// Tab : compléter la saisie jusqu'au curseur.
    Completer(String),
    /// Ouvrir la visionneuse des appels d'outil, sur le dernier (Ctrl-O).
    Ouvrir,
    /// Écrire à l'équipier : le texte, et son genre (une `request` la première
    /// fois, qui ouvre la tâche ; une `instruction` ensuite).
    Envoyer(String, &'static str),
    /// Arrêter le pas en cours (`stop`).
    Arreter,
    /// Quitter le terminal (l'équipier continue).
    Quitter,
    /// Répondre à ce que l'équipier attend (lot tty-4) : `answer` avec
    /// `reply_to`, et `approved` pour une approbation.
    Repondre { attente: crate::boite::Attente, texte: String, accord: Option<bool> },
}

/// Une touche, indépendante du backend (les tests n'ont pas de terminal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Touche {
    Car(char),
    Entree,
    Retour,
    /// ←, →, Début, Fin (et Ctrl-A, Ctrl-E), Suppr.
    Gauche,
    Droite,
    Debut,
    Fin,
    Suppr,
    /// Ctrl-← et Ctrl-→ (et Alt-B, Alt-F).
    MotGauche,
    MotDroite,
    /// Ctrl-W, Ctrl-U, Ctrl-K.
    EffacerMot,
    EffacerDebut,
    EffacerFin,
    /// ↑ et ↓ : la ligne d'au-dessus ou d'en dessous, sinon l'historique des envois.
    Haut,
    Bas,
    /// Alt-Entrée : un saut de ligne dans la saisie, sans l'envoyer.
    Saut,
    /// Un collage d'un bloc (bracketed paste) : ses sauts de ligne restent dans
    /// la saisie, rien ne part avant Entrée.
    Coller(String),
    Echap,
    CtrlC,
    CtrlD,
    /// Arrêter, comme Échap, mais sans le délai d'échappement d'un multiplexeur
    /// (tmux attend 500 ms par défaut avant de rendre un Échap seul, mesuré au
    /// lot tty-6).
    CtrlG,
    /// Ctrl-O : ouvrir un appel d'outil en grand (tty-11).
    CtrlO,
    /// Tab : compléter une commande `/` ou `!` (tty-13).
    Tab,
    /// Page précédente et Page suivante (la visionneuse).
    PageHaut,
    PageBas,
}

/// Au plus tant de lignes dans l'aperçu d'un appel d'outil ; le reste par Ctrl-O.
pub const APERCU: usize = 12;

/// Au plus tant de lignes d'écran pour la saisie ; au-delà, elle défile.
pub const SAISIE_MAX: usize = 8;

/// Le style d'une ligne validée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Genre {
    Texte,
    Outil,
    Boite,
    Note,
    /// Un bloc de code entier (entre deux clôtures ```), à colorer avant
    /// l'historique : `texte` est son code, `langue` celle de sa clôture.
    Code,
}

/// Un repli dit dans la zone vivante : une phrase, et la commande qui le règle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repli {
    pub phrase: String,
    pub commande: String,
}

/// Une proposition de déclarations qui attend un accord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionEnAttente {
    pub backend: String,
    pub id: u64,
    pub chemins: Vec<String>,
    /// La validation l'a acceptée (sinon l'appliquer sera refusé, et elle le dit).
    pub acceptee: bool,
}

/// Une ligne validée (ou un bloc de code entier), prête pour l'historique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validee {
    pub genre: Genre,
    pub texte: String,
    /// La langue d'un bloc de code (`rust`, `ts`…), vide si la clôture n'en dit pas.
    pub langue: String,
    /// La longueur, en octets, de l'en-tête de la ligne (`[équipier · réponse] `
    /// d'une ligne de boîte) : il garde l'accent du thème, la suite est de la prose.
    pub tete: usize,
}

/// Un bloc de code en cours de réception.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Bloc {
    langue: String,
    lignes: Vec<String>,
}

/// Le modèle de la vue.
#[derive(Debug)]
pub struct Vue {
    largeur: u16,
    en_cours: String,
    validees: Vec<Validee>,
    pub etat: Etat,
    saisie: crate::saisie::Saisie,
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
    /// Depuis quand on attend ce qui est en cours (l'appel au modèle, l'outil).
    attente_depuis: Instant,
    /// Le modèle du dernier appel, tel que l'hôte le nomme.
    modele: Option<String>,
    /// « dépôt · branche · tâche », jamais l'id d'un backend.
    pub entete: String,
    /// Un repli à dire dans la zone vivante, sur la ligne du texte en cours quand
    /// rien n'y passe : sous mosh, la commande qui relit le journal (lot tty-7).
    pub repli: Option<Repli>,
    /// Le terminal distingue Maj-Entrée d'Entrée (protocole clavier de kitty,
    /// tty-10) : la saisie vide le dit, sinon elle dit Alt-Entrée.
    pub maj_entree: bool,
    /// Sous tmux (variable `TMUX`), dont la configuration par défaut fait de
    /// Maj-Entrée une Entrée simple : la saisie vide le dit (tty-10).
    pub sous_tmux: bool,
    /// Les appels d'outil de la séance, dans l'ordre (tty-11).
    pub appels: Vec<crate::outils::Appel>,
    /// Les candidats d'une complétion, dits sur la ligne du haut jusqu'à la touche suivante.
    pub propositions: Option<String>,
    /// Montrer la plomberie de l'équipier dans l'historique (`--etapes`) : ses étapes faites, la fin
    /// de son pas, ses appels `report`, `step_done` et `suggest`. Sinon, elle se dit en sourdine sur
    /// la ligne du haut, et la prose à l'écran est la réponse seule (Lucie, 15 nov.).
    pub montrer_etapes: bool,
    /// L'équipier a répondu depuis la dernière demande : sa fin n'a rien à dire.
    repondu: bool,
    /// La dernière ligne de plomberie, dite en sourdine sur la ligne du haut.
    pub discret: Option<String>,
    /// Les propositions de déclarations qui attendent un accord, par leur backend et leur id (tty-16).
    pub propositions_en_attente: Vec<PropositionEnAttente>,
    /// L'accord dont le bouton choisi est Non, par sa clé ; tout autre est sur Oui.
    non_pour: Option<String>,
    /// Les lignes de boîte déjà écrites, par leur `id` : la relecture et le flux
    /// peuvent rendre la même (tty-9, vu à l'essai sous tmux).
    vues_de_boite: std::collections::HashSet<String>,
    /// Le bloc de code ouvert (```) : il se valide entier à sa clôture, pour se
    /// colorer d'un coup ; sa ligne en cours reste visible dans la zone vivante.
    bloc: Option<Bloc>,
    /// Ce que l'équipier attend du superviseur (questions, approbations).
    pub boite: crate::boite::Boite,
    /// L'état de l'équipier selon l'hôte (`running`, `waiting`, `ended`, `error`).
    pub etat_hote: Option<String>,
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
            saisie: crate::saisie::Saisie::default(),
            demande_posee,
            theme: Theme::daemon(),
            capacites: Capacites { couleurs: Niveau::Vraie, glyphes: Glyphes::Surs },
            graine: 0x5eed,
            politiques: HashMap::new(),
            depuis: Instant::now(),
            attente_depuis: Instant::now(),
            modele: None,
            entete: String::new(),
            repli: None,
            maj_entree: false,
            sous_tmux: false,
            appels: Vec::new(),
            propositions: None,
            montrer_etapes: false,
            repondu: true,
            discret: None,
            propositions_en_attente: Vec::new(),
            non_pour: None,
            vues_de_boite: std::collections::HashSet::new(),
            bloc: None,
            boite: crate::boite::Boite::default(),
            etat_hote: None,
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
        // **Jamais un mot d'état nu** (Lucie, 15 nov.) : au travail l'animation du thème, au repos
        // celle qui respire ; seuls une erreur et une fin sans réponse se disent par une phrase.
        match &self.etat {
            Etat::Reflechit | Etat::Ecrit => Some(Quelle::Reflechit),
            Etat::Agit(o) => Some(*self.politiques.get(o).unwrap_or(&Quelle::Lit)),
            _ if self.etat_hote.as_deref() == Some("error") => None,
            Etat::Fini(_) if !self.repondu => None,
            _ => Some(Quelle::Repos),
        }
    }

    /// Le numéro de l'image à `maintenant`.
    pub fn image(&self, maintenant: Instant) -> u64 {
        // Le texte qui coule est le mouvement : l'animation se fige (le coût par token, tty-2).
        if self.etat == Etat::Ecrit {
            return 0;
        }
        let ms = self.quelle().map(|q| q.de(&self.theme).ms).unwrap_or(1).max(1);
        (maintenant.saturating_duration_since(self.depuis).as_millis() as u64) / ms
    }

    /// Le temps jusqu'à l'image suivante, s'il y a une animation.
    pub fn prochaine_image(&self, maintenant: Instant) -> Option<Duration> {
        if self.etat == Etat::Ecrit {
            return None;
        }
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
        self.validees.push(Validee { genre, texte, langue: String::new(), tete: 0 });
    }

    /// Une ligne dont l'en-tête garde l'accent du thème.
    fn valider_avec_tete(&mut self, genre: Genre, tete: &str, corps: &str) {
        self.validees.push(Validee { genre, texte: format!("{tete}{corps}"), langue: String::new(), tete: tete.len() });
    }

    /// Une ligne de texte du modèle : elle ouvre ou ferme un bloc de code, y
    /// entre, ou part telle quelle.
    fn valider_texte(&mut self, ligne: String) {
        let cloture = ligne.trim_start().starts_with("```");
        match (&mut self.bloc, cloture) {
            (Some(_), true) => self.fermer_bloc(),
            (Some(b), false) => b.lignes.push(ligne),
            (None, true) => {
                let langue = ligne.trim_start().trim_start_matches('`').trim().to_string();
                self.bloc = Some(Bloc { langue, lignes: Vec::new() });
            }
            (None, false) => self.valider(Genre::Texte, ligne),
        }
    }

    /// Valide le bloc ouvert, fermé ou non.
    fn fermer_bloc(&mut self) {
        if let Some(b) = self.bloc.take() {
            self.validees.push(Validee { genre: Genre::Code, texte: b.lignes.join("\n"), langue: b.langue, tete: 0 });
        }
    }

    /// Clôt la ligne en cours, si elle a quelque chose ; un bloc resté ouvert
    /// se valide aussi (un outil, la fin du run le coupent).
    fn clore(&mut self) {
        if !self.en_cours.is_empty() {
            let t = std::mem::take(&mut self.en_cours);
            self.valider_texte(t);
        }
        self.fermer_bloc();
    }

    /// Coupe la ligne en cours à la largeur : ce qui déborde part valider. Dans
    /// un bloc de code, on ne coupe pas : le code se replie à l'affichage.
    fn couper(&mut self) {
        if self.bloc.is_some() {
            return;
        }
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
            self.valider_texte(ligne.trim_end().to_string());
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
            self.valider_texte(t);
            self.en_cours.push_str(m);
            self.couper();
        }
    }

    /// Une trame de `/ide/flux` (`{evenement: {kind, …}}`).
    pub fn trame(&mut self, enveloppe: &Value) {
        let e = &enveloppe["evenement"];
        match e["kind"].as_str().unwrap_or_default() {
            // L'hôte ne dit `LlmCall` qu'au retour du modèle : dès qu'un run part, l'équipier réfléchit.
            "RunStarted" | "LlmCall" => {
                // Un appel au modèle part : au départ du run, ou au retour du précédent.
                self.attente_depuis = Instant::now();
                if let Some(m) = e["model"].as_str() {
                    self.modele = Some(m.rsplit('/').next().unwrap_or(m).to_string());
                }
                self.changer(Etat::Reflechit);
            }
            "Token" => {
                self.changer(Etat::Ecrit);
                if let Some(t) = e["text"].as_str() {
                    self.texte(t);
                }
            }
            "ToolCallStarted" => {
                self.clore();
                self.attente_depuis = Instant::now();
                let outil = e["tool"].as_str().unwrap_or("?").to_string();
                let arguments = match &e["arguments"] {
                    Value::String(a) => a.clone(),
                    Value::Null => String::new(),
                    autre => autre.to_string(),
                };
                let mut appel = crate::outils::Appel::debut(self.appels.len() + 1, &outil, &arguments);
                appel.id = e["call_id"].as_str().unwrap_or_default().to_string();
                // Sans --etapes, l'appel défile dans la zone vivante (Lucie, 15 nov.) : l'historique ne
                // garde que la demande et la réponse ; Ctrl-O et le journal gardent le reste.
                if self.montrer_etapes {
                    self.valider(Genre::Outil, appel.ligne());
                } else if !matches!(outil.as_str(), "step_done" | "report" | "edit_plan") {
                    // La plomberie du pas (step_done, report, edit_plan) ne défile pas : elle n'est pas
                    // un travail que Lucie regarde ; Ctrl-O la garde.
                    self.discret = Some(appel.ligne());
                }
                self.appels.push(appel);
                self.changer(Etat::Agit(outil));
            }
            "ToolCallFinished" => {
                self.attente_depuis = Instant::now();
                let ok = e["ok"] == true;
                let dit = if ok { format!("  fait ({} ms)", e["ms"].as_u64().unwrap_or(0)) } else { format!("  refusé ({})", e["error_kind"].as_str().unwrap_or("erreur")) };
                if self.montrer_etapes {
                    self.valider(Genre::Outil, dit);
                } else if !ok {
                    // Réussi, l'appel garde sa ligne, sans verdict ; refusé, il le dit.
                    let tete = self.discret.take().unwrap_or_default();
                    self.discret = Some(format!("{tete} ·{}", dit.trim_start_matches(' ').replacen("", " ", 1)));
                }
                // L'appel par son identifiant, sinon le dernier qui tourne encore.
                let id = e["call_id"].as_str().unwrap_or_default();
                let i = self.appels.iter().rposition(|a| !id.is_empty() && a.id == id).or_else(|| self.appels.iter().rposition(|a| a.ok.is_none()));
                if let Some(a) = i.and_then(|i| self.appels.get_mut(i)) {
                    a.ok = Some(ok);
                    a.ms = e["ms"].as_u64().unwrap_or(0);
                    a.erreur = e["error_kind"].as_str().map(str::to_string);
                    a.extrait = e["result_excerpt"].as_str().map(str::to_string);
                    a.tronque = e["truncated"] == true;
                    a.octets = e["bytes"].as_u64();
                    if self.montrer_etapes {
                        let apercu = a.apercu(APERCU);
                        self.validees.extend(apercu);
                    }
                }
                self.changer(Etat::Reflechit);
            }
            "RunFinished" => {
                self.clore();
                if !matches!(self.etat, Etat::Fini(_)) {
                    self.changer(Etat::Attend);
                }
            }
            "MailboxLine" => self.ligne_de_boite(&e["line"]),
            "PropositionChanged" => {
                let backend = enveloppe["backend"].as_str().unwrap_or_default().to_string();
                self.proposition(&backend, e);
            }
            _ => {}
        }
    }

    /// Une ligne de la boîte de l'équipier.
    pub fn ligne_de_boite(&mut self, l: &Value) {
        if let Some(id) = l["id"].as_str() {
            if !self.vues_de_boite.insert(id.to_string()) {
                return;
            }
        }
        // Un accord répondu : sa commande, avant que la boîte ne l'oublie.
        let resultat = match (l["reply_to"].as_str(), l["approved"].as_bool()) {
            (Some(r), Some(oui)) => self.boite.attentes().iter().find(|a| a.id == r && a.kind == "approval").map(|a| format!("{} : {}", if oui { "accordé" } else { "refusé" }, a.command.clone().unwrap_or_default())),
            _ => None,
        };
        self.boite.ligne(l);
        match (l["from"].as_str(), l["kind"].as_str()) {
            (Some("supervisor"), Some("request" | "instruction")) => self.repondu = false,
            (Some("teammate"), Some("report" | "answer" | "question" | "approval" | "proposal" | "message")) => self.repondu = true,
            _ => {}
        }
        // **La plomberie de l'équipier** (Lucie, 15 nov.) : une étape faite et la fin du pas ne
        // s'écrivent pas ; l'étape se dit en sourdine sur la ligne du haut, la fin change l'état.
        if !self.montrer_etapes {
            let texte = l["text"].as_str().unwrap_or("");
            // L'hôte ne parle qu'au fil de la tâche (« tâche ouverte : … ») : en sourdine.
            if l["from"] == "host" && l["kind"] != "ended" {
                return;
            }
            if l["from"] == "teammate" && matches!(l["kind"].as_str(), Some("plan" | "edit_plan" | "started")) {
                self.discret = Some(format!("équipier : {}", texte.lines().next().unwrap_or_default()));
                return;
            }
            if l["from"] == "teammate" && l["kind"] == "step_done" {
                return;
            }
            if l["from"] == "host" && l["kind"] == "ended" {
                self.changer(Etat::Fini(texte.to_string()));
                return;
            }
        }
        // Une demande après la fin d'un run : l'équipier va repartir, ce n'est plus fini.
        if matches!(self.etat, Etat::Fini(_)) && l["from"] == "supervisor" && matches!(l["kind"].as_str(), Some("request" | "instruction" | "answer")) {
            self.changer(Etat::Attend);
        }
        // **Un accord demandé est un état vivant** (Lucie, 15 nov.) : la demande se dit sur la
        // ligne d'état et ne s'écrit pas ; sa réponse laisse une ligne, d'où qu'elle vienne.
        if l["from"] == "teammate" && l["kind"] == "approval" {
            return;
        }
        if let Some(r) = resultat {
            self.clore();
            self.valider_avec_tete(Genre::Boite, "› ", &r);
            return;
        }
        self.clore();
        let tete = crate::boite::en_tete(l);
        let texte = l["text"].as_str().unwrap_or("");
        // La sortie d'une commande ! revient par la boîte : elle est déjà à l'écran,
        // on n'en dit que la première ligne (tty-13).
        if l["from"] == "supervisor" && texte.starts_with("L'utilisateur a lancé la commande") {
            let premiere = texte.lines().next().unwrap_or_default().trim_end_matches(" :");
            self.valider_avec_tete(Genre::Boite, &tete, &format!("{premiere} (la sortie, ci-dessus)"));
            return;
        }
        // Le texte de la boîte comme du Markdown : un bloc de code y est un bloc de
        // sa langue (tty-11), et l'en-tête va sur la première ligne.
        let mut premiere = true;
        for b in crate::outils::blocs(texte, Genre::Boite) {
            match (b.genre, premiere) {
                (Genre::Code, true) => {
                    self.valider_avec_tete(Genre::Boite, &tete, "");
                    self.validees.push(b);
                }
                (Genre::Code, false) => self.validees.push(b),
                (_, true) => self.valider_avec_tete(Genre::Boite, &tete, &b.texte),
                (_, false) => self.valider(Genre::Boite, format!("  {}", b.texte)),
            }
            premiere = false;
        }
        if l["kind"] == "ended" {
            self.changer(Etat::Fini(texte.to_string()));
        }
    }

    /// L'état de l'équipier selon `GET /ide/teammate/list`.
    pub fn etat_de_l_hote(&mut self, etat: Option<String>) {
        self.etat_hote = etat;
    }

    /// Une note du terminal lui-même (une coupure du flux, un refus).
    pub fn note(&mut self, t: &str) {
        self.clore();
        self.valider(Genre::Note, format!("[tty] {t}"));
    }

    /// Une touche : rend l'action qu'elle demande.
    pub fn touche(&mut self, t: Touche) -> Option<Action> {
        // Les candidats d'une complétion se disent jusqu'à la touche suivante.
        if t != Touche::Tab {
            self.propositions = None;
        }
        match t {
            // Un accord attend et rien n'est tapé : `o` accorde, `n` refuse, ← → choisissent le
            // bouton, Entrée valide le choix (Oui d'avance).
            Touche::Car(c @ ('o' | 'O' | 'y' | 'Y' | 'n' | 'N')) if self.saisie.est_vide() && self.accord_en_cours().is_some() => self.repondre_accord(matches!(c, 'o' | 'O' | 'y' | 'Y')),
            Touche::Gauche if self.saisie.est_vide() && self.accord_en_cours().is_some() => {
                self.non_pour = None;
                None
            }
            Touche::Droite if self.saisie.est_vide() && self.accord_en_cours().is_some() => {
                self.non_pour = self.accord_en_cours();
                None
            }
            Touche::Entree if self.saisie.est_vide() && self.accord_en_cours().is_some() => {
                let oui = self.non_pour != self.accord_en_cours();
                self.repondre_accord(oui)
            }
            Touche::Car(c) => {
                self.saisie.inserer(c.encode_utf8(&mut [0; 4]));
                None
            }
            Touche::Retour => {
                self.saisie.effacer_avant();
                None
            }
            Touche::Gauche => self.edite(crate::saisie::Saisie::gauche),
            Touche::Droite => self.edite(crate::saisie::Saisie::droite),
            Touche::Debut => self.edite(crate::saisie::Saisie::debut),
            Touche::Fin => self.edite(crate::saisie::Saisie::fin),
            Touche::Suppr => self.edite(crate::saisie::Saisie::effacer_apres),
            Touche::MotGauche => self.edite(crate::saisie::Saisie::mot_gauche),
            Touche::MotDroite => self.edite(crate::saisie::Saisie::mot_droite),
            Touche::EffacerMot => self.edite(crate::saisie::Saisie::effacer_mot_avant),
            Touche::EffacerDebut => self.edite(crate::saisie::Saisie::effacer_debut),
            Touche::EffacerFin => self.edite(crate::saisie::Saisie::effacer_fin),
            Touche::Haut => self.edite(crate::saisie::Saisie::monter),
            Touche::Bas => self.edite(crate::saisie::Saisie::descendre),
            Touche::Saut => self.edite(|s| s.inserer("\n")),
            Touche::Coller(texte) => self.edite(|s| s.inserer(&texte)),
            Touche::Entree => {
                let texte = self.saisie.prendre();
                if texte.trim().is_empty() {
                    return None;
                }
                // Une commande du terminal (`/…`, `!…`) n'est jamais envoyée à l'équipier (tty-13).
                let texte = match crate::commandes::lire(&texte) {
                    (Some(c), _) => return Some(Action::Commande(c)),
                    (None, envoi) => envoi.unwrap_or(texte),
                };
                // Ce que l'équipier attend passe avant : la saisie y répond.
                if let Some(attente) = self.boite.premiere().cloned() {
                    self.clore();
                    self.valider_avec_tete(Genre::Boite, "› ", &texte);
                    return Some(Action::Repondre { attente, texte, accord: None });
                }
                let genre = self.genre_d_envoi();
                self.demande_posee = true;
                self.clore();
                self.valider_avec_tete(Genre::Boite, "› ", &texte);
                Some(Action::Envoyer(texte, genre))
            }
            Touche::Echap => Some(Action::Arreter),
            // Une saisie en cours se vide d'abord, comme dans un shell.
            Touche::CtrlC if !self.saisie.est_vide() => self.edite(crate::saisie::Saisie::vider),
            Touche::CtrlC => Some(Action::Quitter),
            Touche::CtrlO if !self.appels.is_empty() => Some(Action::Ouvrir),
            Touche::CtrlO => {
                self.note("aucun appel d'outil à ouvrir");
                None
            }
            Touche::PageHaut | Touche::PageBas => None,
            Touche::Tab if self.saisie.est_vide() => None,
            Touche::Tab => Some(Action::Completer(self.saisie.texte()[..self.saisie.curseur()].to_string())),
            Touche::CtrlG => Some(Action::Arreter),
            Touche::CtrlD if self.saisie.est_vide() => Some(Action::Quitter),
            Touche::CtrlD => None,
        }
    }

    /// Applique une complétion : le début de la saisie jusqu'au curseur est
    /// remplacé, et les candidats se disent sur la ligne du haut.
    pub fn appliquer_completion(&mut self, c: crate::commandes::Completion) {
        self.saisie.remplacer_debut(&c.texte);
        self.propositions = (!c.candidats.is_empty()).then(|| c.candidats.join("  "));
    }

    /// **Une proposition de déclarations** (`PropositionChanged`, sujet `declarations`, tty-16) :
    /// `proposed` la met en attente, `applied` ou `rejected` l'en retire.
    pub fn proposition(&mut self, backend: &str, e: &Value) {
        let Some(id) = e["id"].as_u64() else { return };
        match e["change"].as_str() {
            Some("proposed") => {
                if !self.propositions_en_attente.iter().any(|p| p.backend == backend && p.id == id) {
                    let chemins = e["paths"].as_array().into_iter().flatten().filter_map(|c| c.as_str().map(str::to_string)).collect();
                    self.propositions_en_attente.push(PropositionEnAttente { backend: backend.to_string(), id, chemins, acceptee: e["accepted"] != false });
                }
            }
            _ => self.propositions_en_attente.retain(|p| !(p.backend == backend && p.id == id)),
        }
    }

    /// Reprend les envois des séances d'avant (tty-13).
    pub fn reprendre_historique(&mut self, historique: Vec<String>) {
        self.saisie = crate::saisie::Saisie::avec_historique(historique);
    }

    /// **L'accord qui attend une réponse**, par sa clé : la demande de la boîte en tête si c'en
    /// est une, sinon la première proposition de déclarations quand la boîte n'attend rien.
    fn accord_en_cours(&self) -> Option<String> {
        if let Some(a) = self.boite.premiere() {
            return (a.kind == "approval").then(|| format!("boite:{}", a.id));
        }
        self.propositions_en_attente.first().map(|p| format!("proposition:{}:{}", p.backend, p.id))
    }

    /// Répond à l'accord en cours ; le prochain repartira sur Oui.
    fn repondre_accord(&mut self, oui: bool) -> Option<Action> {
        let cle = self.accord_en_cours()?;
        self.non_pour = None;
        self.clore();
        if cle.starts_with("boite:") {
            let attente = self.boite.premiere().cloned()?;
            return Some(Action::Repondre { attente, texte: if oui { "oui".into() } else { "non".into() }, accord: Some(oui) });
        }
        let p = self.propositions_en_attente.remove(0);
        self.valider_avec_tete(Genre::Boite, "› ", &format!("proposition ⟨{}⟩ : {}", p.id, if oui { "appliquer" } else { "rejeter" }));
        Some(Action::Proposition { backend: p.backend, id: p.id, appliquer: oui })
    }

    /// Les deux boutons d'un accord, le choisi entre crochets.
    fn boutons(&self) -> &'static str {
        if self.non_pour.is_some() && self.non_pour == self.accord_en_cours() {
            "  Oui   [ Non ]"
        } else {
            "[ Oui ]   Non  "
        }
    }

    /// Une édition de la saisie : aucune action.
    fn edite(&mut self, f: impl FnOnce(&mut crate::saisie::Saisie)) -> Option<Action> {
        f(&mut self.saisie);
        None
    }

    /// Le genre d'un envoi que fait le terminal lui-même (la sortie d'une
    /// commande `!`) : une demande s'il ouvre la tâche, une consigne ensuite.
    pub fn genre_d_un_envoi_du_terminal(&mut self) -> &'static str {
        let g = self.genre_d_envoi();
        self.demande_posee = true;
        g
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

    /// Quelque chose attend ta réponse : la ligne d'état le dit, sans animation.
    fn attend_de_toi(&self) -> bool {
        !matches!(self.etat, Etat::Fini(_)) && self.boite.premiere().is_some() || self.accord_en_cours().is_some()
    }

    /// Le texte de la ligne d'état, sans son animation.
    pub fn ligne_d_etat(&self) -> String {
        let c = self.coeur();
        // Ce que l'équipier attend de toi passe avant tout autre état.
        // Un accord demandé se dit même le pas fini : la commande attend toujours.
        if !matches!(self.etat, Etat::Fini(_)) || self.accord_en_cours().is_some_and(|k| k.starts_with("boite:")) {
            if let Some(a) = self.boite.premiere() {
                let reste = match self.boite.attentes().len() {
                    1 => String::new(),
                    n => format!(" (+{})", n - 1),
                };
                return match (a.kind.as_str(), &a.command) {
                    ("approval", Some(cmd)) => format!("{c} attend ton accord : {cmd}  {}  ← → Entrée, ou o / n{reste}", self.boutons()),
                    _ => format!("{c} attend ta réponse : {}{reste}", a.text.lines().next().unwrap_or_default()),
                };
            }
        }
        // Puis une proposition de déclarations qui attend (tty-16).
        if let Some(p) = self.propositions_en_attente.first() {
            let reste = match self.propositions_en_attente.len() {
                1 => String::new(),
                n => format!(" (+{})", n - 1),
            };
            let refus = if p.acceptee { "" } else { " — refusée par la validation" };
            return format!("{c} attend ton accord : proposition ⟨{}⟩ ({}){refus}  {}  ← → Entrée, ou o / n{reste}", p.id, p.chemins.join(", "), self.boutons());
        }
        match &self.etat {
            Etat::Reflechit | Etat::Ecrit => format!("{c} réfléchit…"),
            Etat::Agit(o) => format!("{c} agit : {o}"),
            _ if self.etat_hote.as_deref() == Some("error") => format!("{c} l'équipier s'est arrêté sur une erreur : Ctrl-O ou /journal pour voir"),
            Etat::Fini(_) if !self.repondu => format!("{c} il a fini sans répondre : redemande-lui, ou Ctrl-O pour voir ce qu'il a fait"),
            Etat::Attend | Etat::Fini(_) => format!("{c} à toi"),
        }
    }

    /// La ligne d'état à `maintenant` : l'animation quand l'état en a une, puis
    /// l'en-tête en sourdine à droite s'il tient.
    pub fn ligne_vivante(&self, largeur: usize, maintenant: Instant) -> Line<'static> {
        let mut ligne = match self.quelle().filter(|_| !self.attend_de_toi()) {
            // Le texte coule : il est le mouvement ; la ligne d'état n'en dit que le cœur. L'image
            // entière, figée, redite à chaque ligne qui monte, passait le plafond d'octets par
            // token (36,7 Ko pour 1 000, plafond 35 ; tty-2).
            Some(_) if self.etat == Etat::Ecrit => {
                let texte = anim::couleur(&self.theme.couleurs.texte, self.capacites.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default().add_modifier(Modifier::BOLD);
                Line::from(Span::styled(format!("{} …", self.coeur()), texte))
            }
            Some(q) => {
                // Ce qu'on attend, nommé à côté de l'animation (Lucie, 15 nov.).
                let n = maintenant.saturating_duration_since(self.attente_depuis).as_secs();
                let suite = match &self.etat {
                    Etat::Agit(o) if matches!(o.as_str(), "step_done" | "report" | "edit_plan") => String::new(),
                    Etat::Agit(o) => {
                        let cible = self.appels.last().and_then(|a| a.cible()).map(|c| format!(" {c}")).unwrap_or_default();
                        format!("{o}{cible} travaille… ({n} s)")
                    }
                    Etat::Reflechit => match &self.modele {
                        Some(m) => format!("le modèle répond… ({m}, {n} s)"),
                        None => format!("le modèle répond… ({n} s)"),
                    },
                    _ => String::new(),
                };
                anim::ligne(&self.theme, q, self.image(maintenant), self.graine, self.capacites, &suite)
            }
            None => {
                let rouge = anim::couleur(&self.theme.couleurs.texte, self.capacites.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default().add_modifier(Modifier::BOLD);
                let texte = self.ligne_d_etat();
                match ["[ Oui ]", "[ Non ]"].iter().find_map(|b| texte.find(b).map(|i| (i, i + b.len()))) {
                    Some((i, j)) if self.accord_en_cours().is_some() => Line::from(vec![
                        Span::styled(texte[..i].to_string(), rouge),
                        Span::styled(texte[i..j].to_string(), rouge.add_modifier(Modifier::REVERSED)),
                        Span::styled(texte[j..].to_string(), rouge),
                    ]),
                    _ => Line::from(Span::styled(texte, rouge)),
                }
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

    /// La hauteur que veut la zone vivante à `largeur` colonnes : le texte en
    /// cours, la ligne d'état, et la saisie sur au plus [`SAISIE_MAX`] lignes.
    pub fn hauteur_voulue(&self, largeur: u16) -> u16 {
        let n = self.saisie.lignes_affichees((largeur as usize).saturating_sub(2), SAISIE_MAX).lignes.len();
        2 + n as u16
    }

    /// Dessine la zone vivante telle qu'elle est à `maintenant`.
    pub fn dessiner_a(&self, f: &mut Frame, zone: Rect, maintenant: Instant) {
        let largeur = zone.width as usize;
        // La fin de la ligne en cours, si elle déborde (elle ne déborde pas : `couper`).
        let en_cours = queue(&self.en_cours, largeur);
        // La saisie sur les lignes qui restent (au plus SAISIE_MAX) ; vide, elle
        // dit comment passer à la ligne.
        let place = (zone.height as usize).saturating_sub(2).clamp(1, SAISIE_MAX);
        let a = self.saisie.lignes_affichees(largeur.saturating_sub(2), place);
        let sourdine = anim::couleur(&self.theme.couleurs.sourdine, self.capacites.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default();
        let mut saisie: Vec<Line> = Vec::new();
        if self.saisie.est_vide() {
            let aide = match (self.sous_tmux, self.maj_entree) {
                (true, _) => "sous tmux, Maj-Entrée envoie : Alt-Entrée pour la ligne (ou, dans tmux : set -g extended-keys on ; set -g extended-keys-format csi-u)",
                (false, true) => "Maj-Entrée : à la ligne",
                (false, false) => "Alt-Entrée : à la ligne",
            };
            saisie.push(Line::from(vec![Span::raw("› "), Span::styled(debut(aide, largeur.saturating_sub(3)), sourdine)]));
        } else {
            let n = a.lignes.len();
            for (i, l) in a.lignes.iter().enumerate() {
                let tete = if i == 0 && a.cachees_avant > 0 || i + 1 == n && a.cachees_apres > 0 && i > 0 {
                    "… "
                } else if i == 0 {
                    "› "
                } else {
                    "  "
                };
                saisie.push(Line::from(format!("{tete}{l}")));
            }
        }
        let haut = match (&self.propositions, &self.repli) {
            (None, _) if en_cours.is_empty() && self.discret.is_some() => {
                let sourdine = anim::couleur(&self.theme.couleurs.sourdine, self.capacites.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default();
                Line::from(Span::styled(debut(self.discret.as_deref().unwrap_or_default(), largeur), sourdine))
            }
            (Some(p), _) if en_cours.is_empty() => {
                let sourdine = anim::couleur(&self.theme.couleurs.sourdine, self.capacites.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default();
                Line::from(Span::styled(queue(p, largeur), sourdine))
            }
            (None, Some(r)) if en_cours.is_empty() => {
                let sourdine = anim::couleur(&self.theme.couleurs.sourdine, self.capacites.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default();
                // La phrase et la commande ; trop étroit, la commande seule, entière.
                let tout = format!("{} : {}", r.phrase, r.commande);
                let dit = if colonnes(&tout) <= largeur { tout } else { queue(&r.commande, largeur) };
                Line::from(Span::styled(dit, sourdine))
            }
            _ => Line::from(en_cours),
        };
        let mut lignes = vec![haut, self.ligne_vivante(largeur, maintenant)];
        lignes.extend(saisie);
        f.render_widget(Paragraph::new(lignes), zone);
        let x = zone.x + ((2 + a.colonne) as u16).min(zone.width.saturating_sub(1));
        let y = zone.y + (2 + a.ligne as u16).min(zone.height.saturating_sub(1));
        f.set_cursor_position((x, y));
    }
}

/// Le début de `s` qui tient dans `max` colonnes, `…` en bout quand il est coupé : une
/// indication se lit par son début.
fn debut(s: &str, max: usize) -> String {
    if colonnes(s) <= max {
        return s.to_string();
    }
    let mut l = 0;
    let mut out = String::new();
    for c in s.chars() {
        let w = c.width().unwrap_or(0);
        if l + w + 1 > max {
            break;
        }
        l += w;
        out.push(c);
    }
    out.push('…');
    out
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
    use crate::theme::rgb;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    use ratatui::{Terminal, TerminalOptions, Viewport};
    use serde_json::json;

    /// Le texte de chaque ligne de la zone vivante et la position du curseur,
    /// relative à la zone, pour une zone de `hauteur` lignes.
    fn dessin(v: &Vue, largeur: u16, hauteur: u16) -> (Vec<String>, (u16, u16)) {
        let mut t = Terminal::with_options(TestBackend::new(largeur, hauteur + 3), TerminalOptions { viewport: Viewport::Inline(hauteur) }).unwrap();
        let instant = v.depuis;
        let mut zone = None;
        t.draw(|f| {
            zone = Some(f.area());
            v.dessiner_a(f, f.area(), instant)
        })
        .unwrap();
        let (buf, z) = (t.backend().buffer().clone(), zone.unwrap());
        let lignes = (0..z.height).map(|y| (0..z.width).map(|x| buf[(z.x + x, z.y + y)].symbol().to_string()).collect::<String>().trim_end().to_string()).collect();
        let p = t.get_cursor_position().unwrap();
        (lignes, (p.x - z.x, p.y - z.y))
    }

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
    fn a_fenced_block_is_held_whole_until_its_closing_fence_then_validated_with_its_language() {
        let mut v = Vue::new(20, true);
        for t in ["Voici :\n```rust\nfn main() {\n", "    println!(\"une ligne bien plus longue que vingt colonnes\");\n", "}\n", "```\nFini."] {
            v.trame(&tok(t));
        }
        let lignes = v.prendre_lignes();
        assert_eq!(lignes.len(), 2, "{lignes:#?}");
        assert_eq!((lignes[0].genre, lignes[0].texte.as_str()), (Genre::Texte, "Voici :"));
        assert_eq!(lignes[1].genre, Genre::Code);
        assert_eq!(lignes[1].langue, "rust");
        assert_eq!(lignes[1].texte, "fn main() {\n    println!(\"une ligne bien plus longue que vingt colonnes\");\n}", "le code n'est pas coupé à la largeur");
        assert_eq!(v.en_cours, "Fini.");
        // Un bloc que le run laisse ouvert se valide à sa fin.
        v.trame(&tok("\n```py\nx = 1"));
        assert!(v.prendre_lignes().iter().all(|l| l.genre != Genre::Code), "ouvert : rien ne part encore");
        v.trame(&json!({ "evenement": { "kind": "RunFinished" } }));
        let fin = v.prendre_lignes();
        assert!(fin.iter().any(|l| l.genre == Genre::Code && l.langue == "py" && l.texte == "x = 1"), "{fin:#?}");
    }

    #[test]
    fn the_states_follow_the_events_and_the_mailbox_ends() {
        let mut v = Vue::new(80, false);
        v.montrer_etapes = true; // l'historique entier, comme sous --etapes
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
        assert_eq!(lignes, vec!["salut", "» read_file  ⟨1⟩", "  fait (3 ms)", "[hôte · fini] fait sans plan"]);
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
        assert_eq!(v.quelle(), Some(Quelle::Repos), "au repos, l'invite respire");
        assert!(ligne(&v, 0).starts_with("⁅ · ⁆ ⛧ ⁅ · ⁆ LE CERCLE T'ÉCOUTE..."), "{}", ligne(&v, 0));
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
        assert_eq!(v.quelle(), Some(Quelle::Reflechit), "les tokens coulent : l'animation reste, figée");
        assert_eq!(v.prochaine_image(t0), None, "figée : aucune image de plus pendant que le texte coule");
        assert_eq!(ligne(&v, 0), ligne(&v, 900));
    }

    /// **Les couleurs du thème s'appliquent aux cellules** quand l'agent
    /// réfléchit : le cœur dans la couleur `texte` en gras, les runes dans le
    /// halo sombre, le libellé dans `texte` ; et rien de rouge de base.
    #[test]
    fn the_theme_colours_reach_the_cells_while_the_model_thinks() {
        let mut v = Vue::new(80, true);
        v.trame(&json!({ "evenement": { "kind": "LlmCall" } }));
        let mut t = Terminal::with_options(TestBackend::new(80, 6), TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }).unwrap();
        let instant = v.depuis;
        t.draw(|f| v.dessiner_a(f, f.area(), instant)).unwrap();
        let buf = t.backend().buffer().clone();
        let ligne = (0..buf.area.height).find(|y| (0..buf.area.width).any(|x| buf[(x, *y)].symbol() == "⛧")).expect("la ligne du Cercle");
        let cellule = |s: &str| (0..buf.area.width).map(|x| &buf[(x, ligne)]).find(|c| c.symbol() == s).unwrap_or_else(|| panic!("pas de {s}")).clone();
        let (texte, halo) = (rgb(&v.theme.couleurs.texte).unwrap(), rgb(v.theme.couleurs.halo.last().unwrap()).unwrap());
        assert_eq!(cellule("⛧").fg, Color::Rgb(texte.0, texte.1, texte.2), "le cœur");
        assert!(cellule("⛧").modifier.contains(Modifier::BOLD));
        assert_eq!(cellule("ᚨ").fg, Color::Rgb(halo.0, halo.1, halo.2), "une rune du cercle");
        assert_eq!(cellule("I").fg, Color::Rgb(texte.0, texte.1, texte.2), "le libellé INVOCATION");
        assert!((0..buf.area.width).all(|x| buf[(x, ligne)].fg != Color::Red), "plus de rouge de base sur la ligne du thème");
    }

    /// **La saisie s'édite au clavier** : flèches, Début et Fin, mots, effacements,
    /// historique, saut de ligne et collage d'un bloc ; et le curseur de l'écran
    /// est là où la saisie l'a mis.
    #[test]
    fn the_input_line_is_edited_with_the_keyboard_and_the_screen_cursor_follows() {
        let mut v = Vue::new(60, true);
        let tape = |v: &mut Vue, t: &str| t.chars().for_each(|c| assert_eq!(v.touche(Touche::Car(c)), None));
        tape(&mut v, "lis vue.rs");
        for _ in 0..6 {
            v.touche(Touche::Gauche);
        }
        tape(&mut v, "src/");
        v.touche(Touche::Fin);
        v.touche(Touche::MotGauche);
        v.touche(Touche::Suppr);
        tape(&mut v, "R");
        v.touche(Touche::Debut);
        v.touche(Touche::Droite);
        v.touche(Touche::Retour);
        tape(&mut v, "L");
        assert_eq!(v.saisie.texte(), "Lis src/vue.Rs");
        v.touche(Touche::Fin);
        v.touche(Touche::Saut);
        v.touche(Touche::Coller("puis\r\nrésume".into()));
        assert_eq!(v.saisie.texte(), "Lis src/vue.Rs\npuis\nrésume", "le collage garde ses sauts, rien ne part");
        assert_eq!(v.touche(Touche::Entree), Some(Action::Envoyer("Lis src/vue.Rs\npuis\nrésume".into(), "instruction")));
        v.touche(Touche::Haut);
        assert_eq!(v.saisie.texte(), "Lis src/vue.Rs\npuis\nrésume", "↑ rappelle l'envoi");
        v.touche(Touche::EffacerDebut);
        v.touche(Touche::Haut);
        v.touche(Touche::EffacerFin);
        v.touche(Touche::EffacerMot);
        assert_eq!(v.saisie.texte(), "Lis src/vue.Rs\n\n", "Ctrl-W s'arrête au début de la ligne");
        v.touche(Touche::MotDroite);
        assert_eq!(v.touche(Touche::CtrlC), None, "Ctrl-C vide d'abord une saisie non vide");
        assert!(v.saisie.est_vide());
        assert_eq!(v.touche(Touche::CtrlC), Some(Action::Quitter), "puis quitte");

        // Le curseur de l'écran : « › » et une espace, puis la colonne de la saisie.
        let mut v = Vue::new(40, true);
        tape(&mut v, "漢字ab");
        v.touche(Touche::Gauche);
        let mut t = Terminal::with_options(TestBackend::new(40, 6), TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }).unwrap();
        let instant = v.depuis;
        let mut zone = None;
        t.draw(|f| {
            zone = Some(f.area());
            v.dessiner_a(f, f.area(), instant)
        })
        .unwrap();
        let z = zone.unwrap();
        let pos = t.get_cursor_position().unwrap();
        assert_eq!((pos.x - z.x, pos.y - z.y), (2 + 5, 2), "deux idéogrammes (4 colonnes), a, et le curseur avant b");
    }

    /// Une ligne de la boîte s'annonce en français, `[équipier · réponse]`, et
    /// son en-tête est marqué pour garder l'accent du thème.
    #[test]
    fn a_mailbox_line_is_announced_in_french_with_its_header_marked() {
        let mut v = Vue::new(80, true);
        v.ligne_de_boite(&json!({ "from": "teammate", "kind": "answer", "text": "le plan tient\net la suite" }));
        v.ligne_de_boite(&json!({ "from": "supervisor", "kind": "instruction", "text": "vas-y" }));
        v.ligne_de_boite(&json!({ "from": "teammate", "kind": "inédit", "text": "x" }));
        let l = v.prendre_lignes();
        let vus: Vec<(&str, &str)> = l.iter().map(|l| (&l.texte[..l.tete], &l.texte[l.tete..])).collect();
        assert_eq!(vus, vec![("[équipier · réponse] ", "le plan tient"), ("", "  et la suite"), ("[toi · consigne] ", "vas-y"), ("[équipier · inédit] ", "x")]);
    }

    /// **tty-9, vu à l'essai de Lucie** : une ligne de la boîte reçue deux fois
    /// (par la relecture et par le flux) ne s'écrit qu'une fois.
    #[test]
    fn a_mailbox_line_received_twice_is_written_once() {
        let mut v = Vue::new(80, true);
        let l = json!({ "id": "d/t:5", "from": "supervisor", "kind": "request", "text": "présente-toi" });
        v.ligne_de_boite(&l);
        v.trame(&json!({ "evenement": { "kind": "MailboxLine", "line": l } }));
        v.ligne_de_boite(&json!({ "id": "d/t:6", "from": "teammate", "kind": "report", "text": "voilà" }));
        let textes: Vec<String> = v.prendre_lignes().into_iter().map(|l| l.texte).collect();
        assert_eq!(textes, vec!["[toi · demande] présente-toi", "[équipier · compte rendu] voilà"]);
    }

    /// **tty-9** : la fin d'un run précédent, rejouée à l'ouverture, ne dit plus
    /// « fini » quand une demande l'a suivie : l'équipier va repartir.
    #[test]
    fn an_earlier_end_followed_by_a_request_is_no_longer_finished() {
        let mut v = Vue::new(80, true);
        v.ligne_de_boite(&json!({ "id": "d/t:4", "from": "host", "kind": "ended", "text": "fait sans plan" }));
        assert!(matches!(v.etat, Etat::Fini(_)));
        v.ligne_de_boite(&json!({ "id": "d/t:5", "from": "supervisor", "kind": "request", "text": "encore" }));
        assert!(!matches!(v.etat, Etat::Fini(_)), "{}", v.ligne_d_etat());
    }

    /// **tty-9** : les genres que l'équipier écrit le plus se disent en français.
    #[test]
    fn the_teammate_kinds_are_all_in_french() {
        let mut v = Vue::new(120, true);
        v.montrer_etapes = true; // l'historique entier, comme sous --etapes
        for (i, k) in ["report", "step_done", "plan", "edit_plan", "error", "started", "note"].iter().enumerate() {
            v.ligne_de_boite(&json!({ "id": format!("d/t:{i}"), "from": "teammate", "kind": k, "text": "x" }));
        }
        let tetes: Vec<String> = v.prendre_lignes().into_iter().map(|l| l.texte[..l.tete].to_string()).collect();
        assert_eq!(tetes, ["compte rendu", "étape faite", "plan", "plan revu", "erreur", "parti", "note"].iter().map(|k| format!("[équipier · {k}] ")).collect::<Vec<_>>());
    }

    /// **tty-9** : trop étroit pour la phrase, le repli garde sa commande
    /// entière plutôt qu'un bout de phrase coupé par le début.
    #[test]
    fn too_narrow_for_its_sentence_the_fallback_keeps_its_whole_command() {
        let mut v = Vue::new(110, true);
        let commande = "less /home/lucied/.local/state/rag3weaver/tty/seance-1794682347-581121.txt";
        v.repli = Some(Repli { phrase: "sous mosh, l'historique ne se relit pas au défilement".into(), commande: commande.into() });
        let mut t = Terminal::with_options(TestBackend::new(110, 6), TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }).unwrap();
        let instant = v.depuis;
        let mut zone = None;
        t.draw(|f| {
            zone = Some(f.area());
            v.dessiner_a(f, f.area(), instant)
        })
        .unwrap();
        let (buf, z) = (t.backend().buffer().clone(), zone.unwrap());
        let haut: String = (0..z.width).map(|x| buf[(z.x + x, z.y)].symbol().to_string()).collect::<String>().trim_end().to_string();
        assert!(haut.ends_with(commande), "{haut:?}");
        assert!(haut.starts_with("mosh") || haut.starts_with("less"), "rien de coupé par le début : {haut:?}");
    }

    /// **tty-10, au mot de Lucie** : la saisie s'écrit sur plusieurs lignes, la
    /// zone grandit avec elle, et le curseur de l'écran est sur la bonne ligne.
    #[test]
    fn a_multi_line_input_grows_the_live_zone_and_puts_the_cursor_on_its_line() {
        let mut v = Vue::new(40, true);
        assert_eq!(v.hauteur_voulue(40), 3, "vide : une ligne de saisie");
        "un".chars().for_each(|c| assert_eq!(v.touche(Touche::Car(c)), None));
        v.touche(Touche::Saut);
        "deux".chars().for_each(|c| assert_eq!(v.touche(Touche::Car(c)), None));
        assert_eq!(v.hauteur_voulue(40), 4);
        let (lignes, curseur) = dessin(&v, 40, 4);
        assert_eq!(&lignes[2..], &["› un".to_string(), "  deux".to_string()]);
        assert_eq!(curseur, (2 + 4, 3));
        for _ in 0..20 {
            v.touche(Touche::Saut);
        }
        assert_eq!(v.hauteur_voulue(40), 2 + SAISIE_MAX as u16, "au plus huit lignes de saisie");
        assert_eq!(v.touche(Touche::Entree), Some(Action::Envoyer(format!("un\ndeux{}", "\n".repeat(20)), "instruction")));
        assert_eq!(v.hauteur_voulue(40), 3, "après l'envoi, la zone redescend");
    }

    /// La saisie vide dit, en sourdine, comment passer à la ligne : Maj-Entrée
    /// là où le terminal la distingue, Alt-Entrée sinon. Le curseur reste au début.
    #[test]
    fn the_empty_input_says_how_to_break_a_line() {
        let mut v = Vue::new(60, true);
        let (lignes, curseur) = dessin(&v, 60, 3);
        assert_eq!(lignes[2], "› Alt-Entrée : à la ligne");
        assert_eq!(curseur, (2, 2));
        v.maj_entree = true;
        assert_eq!(dessin(&v, 60, 3).0[2], "› Maj-Entrée : à la ligne");
        v.touche(Touche::Car('x'));
        assert_eq!(dessin(&v, 60, 3).0[2], "› x", "l'indication part dès qu'on tape");
        let mut t = Vue::new(60, true);
        t.sous_tmux = true;
        let mut t = Vue::new(140, true);
        t.sous_tmux = true;
        assert_eq!(dessin(&t, 140, 3).0[2], "› sous tmux, Maj-Entrée envoie : Alt-Entrée pour la ligne (ou, dans tmux : set -g extended-keys on ; set -g extended-keys-format csi-u)", "sous tmux, le dire, et dire le réglage qui l'ouvre");
        let mut e = Vue::new(60, true);
        e.sous_tmux = true;
        let etroit = dessin(&e, 60, 3).0[2].clone();
        assert!(etroit.starts_with("› sous tmux, Maj-Entrée envoie : Alt-Entrée") && etroit.ends_with('…'), "trop étroite, l'indication garde son début : {etroit:?}");
    }

    /// **tty-11** : un appel d'outil se numérote, garde ce qu'il a fait, et son
    /// aperçu suit sa fin dans l'historique.
    #[test]
    fn a_tool_call_is_numbered_kept_and_previewed() {
        let mut v = Vue::new(100, true);
        v.montrer_etapes = true; // l'historique entier, comme sous --etapes
        let result = "**src/a.rs** — lines 1-1 of 1\nScopes: x\n```\n00001| fn main() {}\n```";
        v.trame(&json!({ "evenement": { "kind": "ToolCallStarted", "tool": "read_file", "call_id": "c1", "arguments": "{\"path\":\"src/a.rs\"}" } }));
        v.trame(&json!({ "evenement": { "kind": "ToolCallFinished", "tool": "read_file", "call_id": "c1", "ok": true, "ms": 3, "bytes": 90, "result_excerpt": json!({ "result": result }).to_string() } }));
        let l = v.prendre_lignes();
        let vus: Vec<(Genre, &str, &str)> = l.iter().map(|l| (l.genre, l.langue.as_str(), l.texte.as_str())).collect();
        assert_eq!(vus, vec![(Genre::Outil, "", "» read_file src/a.rs  ⟨1⟩"), (Genre::Outil, "", "  fait (3 ms)"), (Genre::Code, "rs", "fn main() {}")]);
        assert_eq!((v.appels.len(), v.appels[0].ok, v.appels[0].octets), (1, Some(true), Some(90)));
        assert_eq!(v.touche(Touche::CtrlO), Some(Action::Ouvrir));
        assert_eq!(Vue::new(80, true).touche(Touche::CtrlO), None, "sans appel, rien à ouvrir");
    }

    /// **tty-11, au mot de Lucie** : un bloc de code dans la boîte devient un bloc
    /// de code de sa langue, comme dans le texte du modèle.
    #[test]
    fn a_fenced_block_in_the_mailbox_is_a_code_block_of_its_language() {
        let mut v = Vue::new(100, true);
        v.ligne_de_boite(&json!({ "id": "d/t:1", "from": "teammate", "kind": "report", "text": "Voici :\n```typescript\nclass SomeClass {}\n```\nfin" }));
        v.ligne_de_boite(&json!({ "id": "d/t:2", "from": "teammate", "kind": "report", "text": "```rust\nfn a() {}\n```" }));
        let l = v.prendre_lignes();
        let vus: Vec<(Genre, &str, &str)> = l.iter().map(|l| (l.genre, l.langue.as_str(), l.texte.as_str())).collect();
        assert_eq!(vus, vec![
            (Genre::Boite, "", "[équipier · compte rendu] Voici :"),
            (Genre::Code, "typescript", "class SomeClass {}"),
            (Genre::Boite, "", "  fin"),
            (Genre::Boite, "", "[équipier · compte rendu] "),
            (Genre::Code, "rust", "fn a() {}"),
        ]);
    }

    /// **tty-13** : Entrée sur une commande `/` ou `!` la rend à la boucle,
    /// sans rien envoyer à l'équipier ; `//` part en texte, avec un seul `/`.
    #[test]
    fn enter_on_a_command_returns_it_and_sends_nothing() {
        use crate::commandes::Commande;
        let mut v = Vue::new(80, true);
        let tape = |v: &mut Vue, t: &str| t.chars().for_each(|c| { v.touche(Touche::Car(c)); });
        tape(&mut v, "/theme foret");
        assert_eq!(v.touche(Touche::Entree), Some(Action::Commande(Commande::Theme(Some("foret".into())))));
        assert!(v.saisie.est_vide());
        tape(&mut v, "!ls -la");
        assert_eq!(v.touche(Touche::Entree), Some(Action::Commande(Commande::Shell("ls -la".into()))));
        tape(&mut v, "//etc/hosts");
        assert_eq!(v.touche(Touche::Entree), Some(Action::Envoyer("/etc/hosts".into(), "instruction")));
        assert!(v.prendre_lignes().iter().all(|l| !l.texte.contains("theme foret") && !l.texte.contains("ls -la")), "une commande ne s'écrit pas comme un envoi");
    }

    /// Tab demande la complétion de la saisie jusqu'au curseur ; la complétion
    /// remplace ce début, garde la suite, et ses candidats se disent sur la ligne
    /// du haut jusqu'à la touche suivante.
    #[test]
    fn tab_asks_a_completion_which_replaces_the_start_and_lists_its_candidates() {
        let mut v = Vue::new(80, true);
        "/theme cy suite".chars().for_each(|c| { v.touche(Touche::Car(c)); });
        for _ in 0.." suite".len() {
            v.touche(Touche::Gauche);
        }
        assert_eq!(v.touche(Touche::Tab), Some(Action::Completer("/theme cy".into())));
        v.appliquer_completion(crate::commandes::Completion { texte: "/theme cyb".into(), candidats: vec!["cyberpunk".into(), "cyborg".into()] });
        assert_eq!(v.saisie.texte(), "/theme cyb suite");
        assert_eq!(v.saisie.curseur(), "/theme cyb".len());
        assert_eq!(v.propositions.as_deref(), Some("cyberpunk  cyborg"));
        assert_eq!(dessin(&v, 80, 3).0[0], "cyberpunk  cyborg", "sur la ligne du haut");
        v.touche(Touche::Car('o'));
        assert_eq!(v.propositions, None, "la touche suivante les efface");
        assert_eq!(Vue::new(80, true).touche(Touche::Tab), None, "rien à compléter dans une saisie vide");
    }

    /// La sortie d'une commande `!`, revenue par la boîte en consigne, ne
    /// s'affiche pas une seconde fois : sa première ligne seule.
    #[test]
    fn the_output_of_a_bang_command_coming_back_by_the_mailbox_is_said_in_one_line() {
        let mut v = Vue::new(120, true);
        let c = crate::coquille::consigne("ls", std::path::Path::new("/tmp"), &crate::coquille::Resultat { code: Some(0), sortie: b"a\r\nb\r\n".to_vec() });
        v.ligne_de_boite(&json!({ "id": "d/t:1", "from": "supervisor", "kind": "instruction", "text": c }));
        let l: Vec<String> = v.prendre_lignes().into_iter().map(|l| l.texte).collect();
        assert_eq!(l, vec!["[toi · consigne] L'utilisateur a lancé la commande `ls` dans /tmp ; voici le retour (code 0) (la sortie, ci-dessus)"]);
    }

    /// **La prose à l'écran est la réponse seule** (Lucie, second essai du 15 nov.) : une étape
    /// faite, la fin du pas et les appels de plomberie (`report`, `step_done`, `suggest`) ne
    /// s'écrivent pas dans l'historique ; le dernier appel de travail se dit en sourdine sur la ligne
    /// du haut, sans verdict (Lucie : « "fait sans plan" veut rien dire pour l'user »). Le compte
    /// rendu garde sa place, et `--etapes` remontre tout.
    #[test]
    fn the_teammate_plumbing_stays_out_of_the_history_unless_asked() {
        let fil = |v: &mut Vue| {
            v.ligne_de_boite(&json!({ "id": "d/t:0", "from": "supervisor", "kind": "request", "text": "lis a.rs" }));
            v.ligne_de_boite(&json!({ "id": "d/t:0b", "from": "host", "kind": "report", "text": "tâche ouverte : branche agent/x" }));
            let result = "**src/a.rs** — lines 1-1 of 1\nScopes: x\n```\n00001| fn main() {}\n```";
            v.trame(&json!({ "evenement": { "kind": "ToolCallStarted", "tool": "read_file", "call_id": "c0", "arguments": "{\"path\":\"src/a.rs\"}" } }));
            if !v.montrer_etapes {
                assert_eq!(v.discret.as_deref(), Some("» read_file src/a.rs  ⟨1⟩"), "l'appel défile dans la zone vivante");
            }
            v.trame(&json!({ "evenement": { "kind": "ToolCallFinished", "tool": "read_file", "call_id": "c0", "ok": true, "ms": 3, "bytes": 90, "result_excerpt": json!({ "result": result }).to_string() } }));
            v.trame(&json!({ "evenement": { "kind": "ToolCallStarted", "tool": "report", "call_id": "c1", "arguments": "{\"text\":\"x\"}" } }));
            v.trame(&json!({ "evenement": { "kind": "ToolCallFinished", "tool": "report", "call_id": "c1", "ok": true, "ms": 0 } }));
            v.ligne_de_boite(&json!({ "id": "d/t:1", "from": "teammate", "kind": "report", "text": "Voici la réponse." }));
            v.trame(&json!({ "evenement": { "kind": "ToolCallStarted", "tool": "step_done", "call_id": "c2", "arguments": "{}" } }));
            v.trame(&json!({ "evenement": { "kind": "ToolCallFinished", "tool": "step_done", "call_id": "c2", "ok": true, "ms": 0 } }));
            v.ligne_de_boite(&json!({ "id": "d/t:2", "from": "teammate", "kind": "step_done", "text": "fait : rapport fait au superviseur" }));
            v.ligne_de_boite(&json!({ "id": "d/t:3", "from": "host", "kind": "ended", "text": "fait sans plan" }));
        };
        let mut v = Vue::new(100, true);
        fil(&mut v);
        let l: Vec<String> = v.prendre_lignes().into_iter().map(|l| l.texte).collect();
        assert_eq!(l, vec!["[toi · demande] lis a.rs", "[équipier · compte rendu] Voici la réponse."], "la demande et la réponse seules (Lucie)");
        assert_eq!(v.quelle(), Some(Quelle::Repos), "il a répondu : aucun verdict de l'hôte, l'invite respire");
        assert_eq!(v.discret.as_deref(), Some("» read_file src/a.rs  ⟨1⟩"), "le dernier travail, sans verdict ; la plomberie ne défile pas");
        assert_eq!(v.appels.len(), 3, "les appels restent à Ctrl-O");
        assert_eq!(dessin(&v, 100, 3).0[0], "» read_file src/a.rs  ⟨1⟩", "en sourdine, sur la ligne du haut");

        let mut e = Vue::new(100, true);
        e.montrer_etapes = true;
        fil(&mut e);
        let l: Vec<String> = e.prendre_lignes().into_iter().map(|l| l.texte).collect();
        assert!(l.iter().any(|x| x.starts_with("» read_file")) && l.iter().any(|x| x == "fn main() {}") && l.iter().any(|x| x.starts_with("» report")) && l.iter().any(|x| x.starts_with("[équipier · étape faite]")) && l.iter().any(|x| x.starts_with("[hôte · fini]")) && l.iter().any(|x| x.starts_with("[hôte · compte rendu]")), "--etapes remontre tout : {l:?}");
    }

    /// **tty-16** : une proposition de déclarations attend un accord sur la ligne d'état ; `o`
    /// l'applique, `n` la rejette (la route de l'IDE) ; une demande d'accord de la boîte passe avant ;
    /// une proposition appliquée ou rejetée ailleurs (l'IDE) ne s'attend plus.
    #[test]
    fn a_declarations_proposal_waits_on_the_status_line_and_o_or_n_answers_it() {
        let mut v = Vue::new(120, true);
        let ev = |change: &str, id: u64| json!({ "kind": "PropositionChanged", "id": id, "change": change, "paths": ["backends/code/backend.json", "nodes/n.js"], "accepted": true });
        v.proposition("principal", &ev("proposed", 7));
        assert_eq!(v.ligne_d_etat(), "⛧ attend ton accord : proposition ⟨7⟩ (backends/code/backend.json, nodes/n.js)  [ Oui ]   Non    ← → Entrée, ou o / n");
        assert_eq!(v.touche(Touche::Car('o')), Some(Action::Proposition { backend: "principal".into(), id: 7, appliquer: true }));
        assert!(v.propositions_en_attente.is_empty(), "répondue, elle ne s'attend plus");
        v.proposition("principal", &ev("proposed", 8));
        assert_eq!(v.touche(Touche::Car('n')), Some(Action::Proposition { backend: "principal".into(), id: 8, appliquer: false }));

        v.proposition("principal", &ev("proposed", 9));
        v.ligne_de_boite(&json!({ "id": "d/t:1", "from": "teammate", "kind": "approval", "text": "commande en attente", "command": "cargo build" }));
        assert!(v.ligne_d_etat().contains("cargo build"), "une commande qui attend passe avant : {}", v.ligne_d_etat());
        v.proposition("principal", &ev("applied", 9));
        assert!(v.propositions_en_attente.is_empty(), "appliquée ailleurs, elle ne s'attend plus");
        assert_eq!(Vue::new(80, true).touche(Touche::Car('o')), None, "sans rien qui attende, o est une lettre");
        let mut f = Vue::new(120, true);
        f.trame(&json!({ "backend": "principal", "evenement": ev("proposed", 3) }));
        assert_eq!(f.propositions_en_attente.first().map(|p| (p.backend.as_str(), p.id)), Some(("principal", 3)), "par le flux, avec le backend de l'enveloppe");
    }

    /// Le repli (sous mosh, la commande qui relit le journal) se dit sur la
    /// ligne du texte en cours quand rien n'y passe, en sourdine, et garde sa fin
    /// (la commande) quand la largeur manque ; le texte en cours passe avant lui.
    #[test]
    fn the_fallback_shows_on_the_top_line_while_nothing_streams_and_keeps_its_command() {
        let ligne0 = |v: &Vue, largeur: u16| {
            let mut t = Terminal::with_options(TestBackend::new(largeur, 6), TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }).unwrap();
            let instant = v.depuis;
            let mut zone = None;
            t.draw(|f| {
                zone = Some(f.area());
                v.dessiner_a(f, f.area(), instant)
            })
            .unwrap();
            let (buf, z) = (t.backend().buffer().clone(), zone.unwrap());
            ((0..z.width).map(|x| buf[(z.x + x, z.y)].symbol().to_string()).collect::<String>().trim_end().to_string(), buf[(z.x, z.y)].fg)
        };
        let mut v = Vue::new(100, true);
        v.repli = Some(Repli { phrase: "sous mosh, l'historique ne se relit pas au défilement".into(), commande: "less /h/seance-1.txt".into() });
        let (texte, fg) = ligne0(&v, 100);
        assert_eq!(texte, "sous mosh, l'historique ne se relit pas au défilement : less /h/seance-1.txt");
        let s = rgb(&v.theme.couleurs.sourdine).unwrap();
        assert_eq!(fg, Color::Rgb(s.0, s.1, s.2), "en sourdine");
        assert!(ligne0(&v, 30).0.ends_with("less /h/seance-1.txt"), "{}", ligne0(&v, 30).0);
        v.trame(&tok("en cours"));
        assert_eq!(ligne0(&v, 100).0, "en cours", "le texte en cours passe avant le repli");
    }

    /// **Pour chaque thème embarqué**, ses couleurs atteignent les cellules du
    /// Cercle : son cœur dans `texte` en gras, un glyphe du cercle dans le halo
    /// sombre.
    #[test]
    fn every_theme_colours_its_circle_cells() {
        for (nom, _) in crate::theme::THEMES {
            let mut v = Vue::new(100, true);
            v.theme = crate::theme::Theme::par_nom(nom).unwrap();
            v.trame(&json!({ "evenement": { "kind": "LlmCall" } }));
            let mut t = Terminal::with_options(TestBackend::new(100, 6), TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }).unwrap();
            let instant = v.depuis;
            t.draw(|f| v.dessiner_a(f, f.area(), instant)).unwrap();
            let buf = t.backend().buffer().clone();
            let a = &v.theme.animations.reflechit;
            let coeur = v.theme.replis.get(&a.coeur).cloned().unwrap_or(a.coeur.clone());
            let glyphe = v.theme.replis.get(&a.glyphes[1]).cloned().unwrap_or(a.glyphes[1].clone());
            let ligne = (0..buf.area.height).find(|y| (0..buf.area.width).any(|x| buf[(x, *y)].symbol() == coeur)).unwrap_or_else(|| panic!("{nom} : la ligne du Cercle"));
            let cellule = |s: &str| (0..buf.area.width).map(|x| &buf[(x, ligne)]).find(|c| c.symbol() == s).unwrap_or_else(|| panic!("{nom} : pas de {s}")).clone();
            let (texte, halo) = (rgb(&v.theme.couleurs.texte).unwrap(), rgb(v.theme.couleurs.halo.last().unwrap()).unwrap());
            assert_eq!(cellule(&coeur).fg, Color::Rgb(texte.0, texte.1, texte.2), "{nom} : le cœur");
            assert!(cellule(&coeur).modifier.contains(Modifier::BOLD), "{nom}");
            assert_eq!(cellule(&glyphe).fg, Color::Rgb(halo.0, halo.1, halo.2), "{nom} : un glyphe du cercle");
        }
    }

    #[test]
    fn the_header_says_repo_branch_and_task_on_the_right_when_it_fits() {
        let mut v = Vue::new(80, true);
        v.entete = "rag3db · tty-3 · tâche parle".into();
        let texte = |v: &Vue, l: usize| v.ligne_vivante(l, v.depuis).spans.iter().map(|s| s.content.to_string()).collect::<String>();
        let large = texte(&v, 80);
        assert!(large.starts_with("⁅ · ⁆ ⛧") && large.ends_with("rag3db · tty-3 · tâche parle"), "{large}");
        assert_eq!(colonnes(&large), 80);
        assert!(!texte(&v, 20).contains("rag3db"), "trop étroit : l'en-tête se tait");
        assert!(!large.contains("backend"));
    }

    /// **Un accord demandé est un état vivant** (Lucie, 15 nov. : « les accords demandés, je dois
    /// pas les voir défiler comme ça, ça doit être vivant ») : la demande se dit dans la zone
    /// vivante, même quand l'équipier s'anime, et ne s'écrit pas ; l'historique n'en garde qu'une
    /// ligne de résultat quand la réponse revient, qu'elle vienne du terminal ou d'ailleurs.
    #[test]
    fn an_approval_request_lives_in_the_live_zone_and_leaves_one_result_line() {
        let mut v = Vue::new(120, true);
        let vivante = |v: &Vue| v.ligne_vivante(120, v.depuis).spans.iter().map(|s| s.content.to_string()).collect::<String>();
        let ecrites = |v: &mut Vue| v.prendre_lignes().into_iter().map(|l| l.texte).collect::<Vec<_>>();
        v.trame(&json!({ "evenement": { "kind": "ToolCallStarted", "tool": "run_command" } }));
        ecrites(&mut v);
        v.ligne_de_boite(&ligne_b(1, "teammate", "approval", json!({ "command": "git status" })));
        assert_eq!(ecrites(&mut v), Vec::<String>::new(), "la demande ne s'écrit pas");
        assert!(vivante(&v).starts_with("⛧ attend ton accord : git status  [ Oui ]"), "l'animation ne la recouvre pas : {}", vivante(&v));
        let Some(Action::Repondre { accord, .. }) = v.touche(Touche::Car('o')) else { panic!("o accorde") };
        assert_eq!(accord, Some(true));
        v.ligne_de_boite(&ligne_b(2, "supervisor", "answer", json!({ "reply_to": "a1/doc:1", "approved": true, "text": "oui" })));
        let apres = ecrites(&mut v);
        assert_eq!(apres.len(), 1, "une ligne de résultat : {apres:?}");
        assert!(apres[0].ends_with("accordé : git status"), "{apres:?}");
        assert!(!vivante(&v).contains("accord"), "répondue, elle quitte la zone vivante : {}", vivante(&v));
        // Répondue ailleurs (l'IDE) : la même ligne de résultat.
        v.ligne_de_boite(&ligne_b(3, "teammate", "approval", json!({ "command": "rm -rf build" })));
        v.ligne_de_boite(&ligne_b(4, "supervisor", "answer", json!({ "reply_to": "a1/doc:3", "approved": false, "text": "non" })));
        let ailleurs = ecrites(&mut v);
        assert_eq!(ailleurs.len(), 1, "{ailleurs:?}");
        assert!(ailleurs[0].ends_with("refusé : rm -rf build"), "{ailleurs:?}");
    }

    /// **Des boutons d'accord au clavier** (Lucie, 15 nov. : « faut pouvoir avoir "boutons" de
    /// validation qu'on peut accéder aux flèches du clavier, avec pré-sélectionné oui quand y a un
    /// choix ») : Oui est choisi d'avance, ← et → changent le choix, Entrée le valide ; `o` et `n`
    /// restent des raccourcis ; le bouton choisi est en vidéo inverse ; une nouvelle demande repart
    /// sur Oui. Une proposition de déclarations a les mêmes boutons.
    #[test]
    fn approval_buttons_answer_with_the_arrows_and_enter_yes_preselected() {
        let mut v = Vue::new(120, true);
        v.ligne_de_boite(&ligne_b(1, "teammate", "approval", json!({ "command": "cargo build" })));
        assert_eq!(v.ligne_d_etat(), "⛧ attend ton accord : cargo build  [ Oui ]   Non    ← → Entrée, ou o / n");
        let inverse = |v: &Vue| v.ligne_vivante(120, v.depuis).spans.iter().filter(|s| s.style.add_modifier.contains(Modifier::REVERSED)).map(|s| s.content.to_string()).collect::<Vec<_>>();
        assert_eq!(inverse(&v), vec!["[ Oui ]".to_string()]);
        assert_eq!(v.touche(Touche::Droite), None);
        assert!(v.ligne_d_etat().contains("  Oui   [ Non ]"), "{}", v.ligne_d_etat());
        assert_eq!(inverse(&v), vec!["[ Non ]".to_string()]);
        let Some(Action::Repondre { accord, .. }) = v.touche(Touche::Entree) else { panic!("Entrée valide le choix") };
        assert_eq!(accord, Some(false));
        v.ligne_de_boite(&ligne_b(2, "supervisor", "answer", json!({ "reply_to": "a1/doc:1", "approved": false })));
        v.ligne_de_boite(&ligne_b(3, "teammate", "approval", json!({ "command": "ls" })));
        assert!(v.ligne_d_etat().contains("[ Oui ]"), "une nouvelle demande repart sur Oui : {}", v.ligne_d_etat());
        v.ligne_de_boite(&json!({ "id": "a1/doc:3b", "from": "host", "kind": "ended", "text": "fait sans plan" }));
        assert!(v.ligne_d_etat().contains("attend ton accord : ls"), "le pas fini, la demande se dit encore : {}", v.ligne_d_etat());
        v.touche(Touche::Droite);
        v.touche(Touche::Gauche);
        let Some(Action::Repondre { accord, .. }) = v.touche(Touche::Entree) else { panic!("Entrée valide le choix") };
        assert_eq!(accord, Some(true));
        v.ligne_de_boite(&ligne_b(4, "supervisor", "answer", json!({ "reply_to": "a1/doc:3", "approved": true })));
        // Une proposition : les mêmes boutons, Entrée sur Oui l'applique.
        v.proposition("principal", &json!({ "kind": "PropositionChanged", "id": 7, "change": "proposed", "paths": ["nodes/n.js"], "accepted": true }));
        assert_eq!(v.ligne_d_etat(), "⛧ attend ton accord : proposition ⟨7⟩ (nodes/n.js)  [ Oui ]   Non    ← → Entrée, ou o / n");
        assert_eq!(v.touche(Touche::Entree), Some(Action::Proposition { backend: "principal".into(), id: 7, appliquer: true }));
        // Sans rien qui attende, les flèches sont celles de la saisie.
        assert_eq!(v.touche(Touche::Entree), None);
    }

    /// **L'animation dès que l'équipier part** (Lucie, 15 nov. : « j'ai plus la petite animation de
    /// chargement ») : l'hôte n'émet `LlmCall` qu'au retour du modèle (il porte `ms` et `finish`) ;
    /// entre `RunStarted` et lui, l'équipier réfléchit déjà. Relu sur le flux de sa tâche atelier :
    /// une consigne après la fin du pas, `RunStarted`, puis rien pendant tout le premier appel.
    #[test]
    fn a_run_that_starts_plays_the_thinking_animation_until_its_first_model_answer() {
        let mut v = Vue::new(120, true);
        v.ligne_de_boite(&json!({ "id": "d/t:1", "from": "host", "kind": "ended", "text": "fait sans plan" }));
        v.ligne_de_boite(&json!({ "id": "d/t:2", "from": "supervisor", "kind": "instruction", "text": "encore" }));
        assert_eq!(v.quelle(), Some(Quelle::Repos), "la consigne seule : l'équipier n'est pas encore parti");
        v.trame(&json!({ "evenement": { "kind": "RunStarted", "name": "chat:x" } }));
        assert_eq!(v.quelle(), Some(Quelle::Reflechit), "parti, il réfléchit : {}", v.ligne_d_etat());
        v.trame(&json!({ "evenement": { "kind": "ToolCallStarted", "tool": "step_done" } }));
        v.trame(&json!({ "evenement": { "kind": "ToolCallFinished", "tool": "step_done", "ok": true } }));
        v.trame(&json!({ "evenement": { "kind": "RunFinished" } }));
        assert_eq!(v.quelle(), Some(Quelle::Repos), "le run fini, l'invite au repos : {}", v.ligne_d_etat());
    }

    /// **Jamais un mot d'état nu** (Lucie, 15 nov. : « pourquoi ça m'écrit "attend", c'est
    /// horrible, on doit jamais voir "attend" comme ça : que des animations de chargement ou bien
    /// des trucs vivants ») : quand l'équipier travaille, l'animation du thème ; quand le tour est à
    /// elle, l'animation de repos ; s'il a fini sans répondre, une phrase qui lui dit quoi faire.
    #[test]
    fn the_status_line_never_shows_a_bare_state_word() {
        let mut v = Vue::new(120, true);
        let vivante = |v: &Vue| v.ligne_vivante(120, v.depuis).spans.iter().map(|s| s.content.to_string()).collect::<String>();
        let nu = |t: &str| ["attend", "fini", "fait sans plan", "écrit", "réfléchit"].iter().any(|m| t.contains(m));
        assert_eq!(v.quelle(), Some(Quelle::Repos), "au départ, l'invite respire : {}", vivante(&v));
        assert!(vivante(&v).contains("LE CERCLE T'ÉCOUTE") && !nu(&vivante(&v)), "{}", vivante(&v));
        v.ligne_de_boite(&json!({ "id": "d/t:1", "from": "supervisor", "kind": "request", "text": "lis a.rs" }));
        v.trame(&json!({ "evenement": { "kind": "RunStarted" } }));
        v.trame(&json!({ "evenement": { "kind": "Token", "text": "Je " } }));
        assert_eq!(v.quelle(), Some(Quelle::Reflechit), "le modèle qui écrit s'anime aussi : {}", vivante(&v));
        v.ligne_de_boite(&json!({ "id": "d/t:2", "from": "teammate", "kind": "report", "text": "Voici." }));
        v.trame(&json!({ "evenement": { "kind": "RunFinished" } }));
        v.ligne_de_boite(&json!({ "id": "d/t:3", "from": "host", "kind": "ended", "text": "fait sans plan" }));
        assert_eq!(v.quelle(), Some(Quelle::Repos), "il a répondu : le tour est à elle, l'invite respire : {}", vivante(&v));
        assert!(!nu(&vivante(&v)), "{}", vivante(&v));
        // Fini sans rien dire : une phrase à elle, pas un mot machine.
        v.ligne_de_boite(&json!({ "id": "d/t:4", "from": "supervisor", "kind": "instruction", "text": "encore" }));
        v.trame(&json!({ "evenement": { "kind": "RunStarted" } }));
        v.trame(&json!({ "evenement": { "kind": "RunFinished" } }));
        v.ligne_de_boite(&json!({ "id": "d/t:5", "from": "host", "kind": "ended", "text": "fait sans plan" }));
        assert_eq!(v.quelle(), None);
        assert_eq!(vivante(&v), "⛧ il a fini sans répondre : redemande-lui, ou Ctrl-O pour voir ce qu'il a fait");
        // En erreur : une phrase aussi.
        let mut e = Vue::new(120, true);
        e.etat_de_l_hote(Some("error".into()));
        assert_eq!(vivante(&e), "⛧ l'équipier s'est arrêté sur une erreur : Ctrl-O ou /journal pour voir");
    }

    /// **Ce qu'on attend, nommé à côté de l'animation** (Lucie, 15 nov. : « je veux voir
    /// l'animation et comprendre ce qu'il attend exactement ») : le modèle qui répond, avec son nom
    /// et le temps écoulé ; l'outil qui travaille, avec sa cible et son temps.
    #[test]
    fn what_is_awaited_is_named_beside_the_animation() {
        let mut v = Vue::new(160, true);
        let a = |v: &Vue, s: u64| v.ligne_vivante(160, v.attente_depuis + Duration::from_secs(s)).spans.iter().map(|s| s.content.to_string()).collect::<String>();
        v.trame(&json!({ "evenement": { "kind": "RunStarted" } }));
        assert!(a(&v, 12).ends_with(" le modèle répond… (12 s)"), "{}", a(&v, 12));
        v.trame(&json!({ "evenement": { "kind": "LlmCall", "model": "google/gemini-3.8-flash", "ms": 2457 } }));
        v.trame(&json!({ "evenement": { "kind": "ToolCallStarted", "tool": "run_command", "arguments": "{\"command\":\"cargo check\"}" } }));
        assert!(a(&v, 40).ends_with(" run_command cargo check travaille… (40 s)"), "{}", a(&v, 40));
        v.trame(&json!({ "evenement": { "kind": "ToolCallFinished", "tool": "run_command", "ok": true, "ms": 40_000 } }));
        assert!(a(&v, 3).ends_with(" le modèle répond… (gemini-3.8-flash, 3 s)"), "{}", a(&v, 3));
    }

    fn ligne_b(n: usize, from: &str, kind: &str, extra: Value) -> Value {
        let mut l = json!({ "id": format!("a1/doc:{n}"), "from": from, "kind": kind, "text": format!("texte {n}") });
        if let (Some(o), Some(e)) = (l.as_object_mut(), extra.as_object()) {
            for (k, v) in e {
                o.insert(k.clone(), v.clone());
            }
        }
        l
    }

    /// **La boîte dans la vue** (lot tty-4) : une approbation en attente se dit
    /// sur la ligne d'état avec sa commande ; `o` l'accorde, `n` la refuse, sans
    /// rien taper d'autre ; une saisie répond à ce qui attend avant d'être une
    /// instruction.
    #[test]
    fn what_the_teammate_waits_for_is_shown_and_answered_from_the_keyboard() {
        let mut v = Vue::new(120, true);
        v.ligne_de_boite(&ligne_b(1, "teammate", "approval", json!({ "command": "cargo build" })));
        v.ligne_de_boite(&ligne_b(2, "teammate", "question", json!({})));
        assert_eq!(v.ligne_d_etat(), "⛧ attend ton accord : cargo build  [ Oui ]   Non    ← → Entrée, ou o / n (+1)");
        let Some(Action::Repondre { attente, texte, accord }) = v.touche(Touche::Car('o')) else { panic!("o accorde") };
        assert_eq!((attente.id.as_str(), texte.as_str(), accord), ("a1/doc:1", "oui", Some(true)));
        // L'hôte rend la réponse : la boîte la voit, la question passe devant.
        v.ligne_de_boite(&ligne_b(3, "supervisor", "answer", json!({ "reply_to": "a1/doc:1", "approved": true })));
        assert_eq!(v.ligne_d_etat(), "⛧ attend ta réponse : texte 2");
        assert_eq!(v.touche(Touche::Car('o')), None, "sans approbation en attente, o est une lettre");
        v.touche(Touche::Retour);
        for c in "Petit dépôt".chars() {
            v.touche(Touche::Car(c));
        }
        let Some(Action::Repondre { attente, texte, accord }) = v.touche(Touche::Entree) else { panic!("la saisie répond") };
        assert_eq!((attente.id.as_str(), texte.as_str(), accord), ("a1/doc:2", "Petit dépôt", None));
        v.ligne_de_boite(&ligne_b(4, "supervisor", "answer", json!({ "reply_to": "a1/doc:2" })));
        v.etat_de_l_hote(Some("waiting".into()));
        assert_eq!(v.quelle(), Some(Quelle::Repos), "plus rien n'attend de toi : l'invite respire");
        v.ligne_de_boite(&ligne_b(5, "teammate", "approval", json!({ "command": "rm -rf build" })));
        let Some(Action::Repondre { accord, .. }) = v.touche(Touche::Car('n')) else { panic!("n refuse") };
        assert_eq!(accord, Some(false));
        let saisie_suivante: Vec<_> = "fais ça".chars().map(|c| v.touche(Touche::Car(c))).collect();
        assert!(saisie_suivante.iter().all(|a| a.is_none()), "{saisie_suivante:?}");
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
        assert_eq!(v.touche(Touche::CtrlG), Some(Action::Arreter), "Ctrl-G arrête, sans le délai d'Échap sous tmux");
        assert_eq!(v.touche(Touche::CtrlD), Some(Action::Quitter));
    }

    fn rendu(largeur: u16, hauteur: u16, v: &mut Vue) -> Vec<String> {
        let mut t = Terminal::with_options(TestBackend::new(largeur, hauteur), TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }).unwrap();
        let lignes = v.prendre_lignes();
        if !lignes.is_empty() {
            t.insert_before(lignes.len() as u16, |buf| {
                let rendu: Vec<Line> = lignes.iter().flat_map(|x| crate::code::lignes(x, &v.theme, v.capacites, largeur as usize, None)).collect();
                Paragraph::new(rendu).render(buf.area, buf);
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
