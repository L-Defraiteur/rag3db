//! **La boucle du terminal** (lot tty-2) : la vue inline sur crossterm (Windows,
//! macOS, Linux). Trois sources entrent dans un même canal : le flux de l'hôte
//! (un fil), la boîte relue toutes les 2 s tant que le flux ne la relaie pas
//! (un fil), et le clavier. L'écran se redessine au plus 30 fois par seconde,
//! et seulement quand quelque chose a changé : un token qui arrive entre deux
//! images attend la suivante, les tokens d'une même image partent ensemble.

use crate::anim::Capacites;
use crate::client::{self, Depot};
use crate::endpoint::Endpoint;
use crate::flux;
use crate::journal::Journal;
use crate::vue::{Action, Touche, Validee, Vue, HAUTEUR};
use ratatui::crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::{TerminalOptions, Viewport};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

/// L'intervalle minimal entre deux images (30 par seconde).
pub const IMAGE: Duration = Duration::from_millis(33);

enum Msg {
    Trame(Value),
    Boite(Value),
    Note(String),
    Politiques(Value),
    Entete(String),
    /// L'état de l'équipier selon `GET /ide/teammate/list`.
    EtatHote(Option<String>),
}

/// La tâche suivie : son dépôt, et son nom.
#[derive(Clone)]
pub struct Tache {
    pub depot: Depot,
    pub task: String,
}

/// **Ce qu'on apprend d'une tâche** : le manifeste de son worktree (le filtre du
/// flux : une trame d'un autre worktree ne la regarde pas), son en-tête, et ce
/// que chacun de ses outils a le droit de faire.
fn connaitre(agent: &ureq::Agent, ep: &Endpoint, tache: &Tache, filtre: &Mutex<Option<String>>, tx: &mpsc::Sender<Msg>) {
    let chemin = format!("/ide/teammate?backend={}&task={}", client::encoder(&tache.depot.id), client::encoder(&tache.task));
    let worktree = client::lire(agent, ep, &chemin).ok().and_then(|v| v["backend"].as_str().map(str::to_string));
    if let Some(m) = &worktree {
        *filtre.lock().unwrap_or_else(|p| p.into_inner()) = Some(m.clone());
    }
    let manifeste = worktree.unwrap_or_else(|| tache.depot.manifest.clone());
    if let Ok(etat) = client::lire_brut(agent, ep, "/ide/etat") {
        let _ = tx.send(Msg::Entete(client::entete(&etat, &manifeste, &tache.task)));
    }
    if let Ok(d) = client::poster_brut(agent, ep, "/ide/describe", &json!({ "manifest": manifeste })) {
        let _ = tx.send(Msg::Politiques(d));
    }
}

/// Pourquoi la vue s'est arrêtée.
#[derive(Debug)]
pub enum Echec {
    /// La vue inline n'a pas pu démarrer (le terminal ne rend pas la position du
    /// curseur, par exemple) : rien n'est encore parti à l'hôte, `--plain` peut
    /// prendre la suite ; le journal lui est rendu.
    VueIndisponible(String, Option<Journal>),
    /// Une erreur pendant la séance.
    Autre(String),
}

/// `journal` reçoit en texte brut ce qui part dans l'historique ; `mosh` dit que
/// cet historique ne se relira pas, et la zone vivante donne alors la commande
/// qui relit le journal (lot tty-7).
pub fn lancer(ep: Endpoint, tache: Tache, demande: Option<String>, depuis: Option<u64>, theme: crate::theme::Theme, mut journal: Option<Journal>, mosh: bool) -> Result<(), Echec> {
    // La vue d'abord : si elle ne démarre pas, rien n'est parti à l'hôte (ni la
    // demande, ni un abonnement), et `--plain` prend la suite sans doublon.
    let mut terminal = match ratatui::try_init_with_options(TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }) {
        Ok(t) => t,
        Err(e) => {
            let _ = ratatui::try_restore();
            return Err(Echec::VueIndisponible(e.to_string(), journal));
        }
    };
    // Le collage d'un bloc (bracketed paste) : un texte collé arrive entier, ses
    // sauts de ligne ne valent pas Entrée. Un terminal qui ne le connaît pas
    // ignore la demande.
    let _ = ratatui::crossterm::execute!(std::io::stdout(), EnableBracketedPaste);
    let agent = client::agent();
    let (tx, rx) = mpsc::channel::<Msg>();
    let arret = Arc::new(AtomicBool::new(false));
    let boite_au_flux = Arc::new(AtomicBool::new(false));
    let filtre: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    connaitre(&agent, &ep, &tache, &filtre, &tx);

    // Le flux, sur son fil ; la demande part une fois l'abonnement pris.
    let flux_fil = {
        let (ep, tx, arret, boite_au_flux, agent, filtre) = (ep.clone(), tx.clone(), arret.clone(), boite_au_flux.clone(), agent.clone(), filtre.clone());
        let tache = tache.clone();
        let mut demande = demande.clone();
        std::thread::spawn(move || {
            let tx2 = tx.clone();
            let mut ouvert = || {
                if let Some(texte) = demande.take() {
                    match client::poster(&agent, &ep, "/ide/teammate/mailbox", &json!({ "backend": tache.depot.id, "task": tache.task, "kind": "request", "text": texte })) {
                        // La demande ouvre la tâche : son worktree existe désormais.
                        Ok(_) => connaitre(&agent, &ep, &tache, &filtre, &tx2),
                        Err(e) => {
                            let _ = tx2.send(Msg::Note(format!("la demande est refusée : {e}")));
                        }
                    }
                }
            };
            let tx3 = tx.clone();
            let r = flux::suivre_sans_fin(&ep, &["agent", "tokens", "mailbox"], depuis, &arret, &mut ouvert, |p| {
                let _ = tx3.send(Msg::Note(format!("flux coupé ({p}) : je reprends")));
            }, |t| {
                if t.event == "lacune" {
                    let _ = tx.send(Msg::Note("lacune : le tampon de l'hôte a perdu des événements".into()));
                } else if let Ok(v) = serde_json::from_str::<Value>(&t.data) {
                    if let Some(f) = filtre.lock().unwrap_or_else(|p| p.into_inner()).as_deref() {
                        if v["backend"].as_str() != Some(f) {
                            return flux::Suite::Continuer;
                        }
                    }
                    if v["evenement"]["kind"] == "MailboxLine" {
                        boite_au_flux.store(true, Ordering::SeqCst);
                    }
                    let _ = tx.send(Msg::Trame(v));
                }
                flux::Suite::Continuer
            });
            if let Err(e) = r {
                let _ = tx.send(Msg::Note(format!("le flux est refusé : {e}")));
            }
        })
    };
    // La boîte, relue tant que le flux ne la relaie pas.
    {
        let (ep, tx, arret, boite_au_flux, agent) = (ep.clone(), tx.clone(), arret.clone(), boite_au_flux.clone(), agent.clone());
        let chemin = format!("/ide/teammate?backend={}&task={}", client::encoder(&tache.depot.id), client::encoder(&tache.task));
        let (parent, task) = (tache.depot.id.clone(), tache.task.clone());
        std::thread::spawn(move || {
            let mut lues = 0usize;
            while !arret.load(Ordering::SeqCst) {
                // L'état de l'équipier (lot tty-4), par la liste de son superviseur.
                if let Ok(liste) = client::lire(&agent, &ep, "/ide/teammate/list?supervisor=supervisor") {
                    if tx.send(Msg::EtatHote(crate::boite::etat_dans_la_liste(&liste, &parent, &task))).is_err() {
                        return;
                    }
                }
                if let Ok(v) = client::lire(&agent, &ep, &chemin) {
                    let lignes = v["mailbox"].as_array().cloned().unwrap_or_default();
                    for l in lignes.iter().skip(lues) {
                        if !boite_au_flux.load(Ordering::SeqCst) && tx.send(Msg::Boite(l.clone())).is_err() {
                            return;
                        }
                    }
                    lues = lignes.len();
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        });
    }

    let largeur = terminal.size().map(|s| s.width).unwrap_or(80);
    let mut vue = Vue::new(largeur, demande.is_some());
    vue.capacites = Capacites::du_terminal();
    vue.theme = theme;
    if let (true, Some(j)) = (mosh, &journal) {
        vue.repli = Some(format!("sous mosh, l'historique ne se relit pas au défilement : {}", j.relecture()));
    }
    vue.graine = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1) | 1;
    let rendu = boucle(&mut terminal, &mut vue, &rx, &agent, &ep, &tache, &mut journal);
    arret.store(true, Ordering::SeqCst);
    // La zone vivante reste à l'écran telle quelle ; le curseur passe dessous.
    let _ = terminal.draw(|f| vue.dessiner(f, f.area()));
    let _ = ratatui::crossterm::execute!(std::io::stdout(), DisableBracketedPaste);
    ratatui::restore();
    println!();
    let _ = flux_fil.join();
    rendu.map_err(Echec::Autre)
}

fn boucle(terminal: &mut ratatui::DefaultTerminal, vue: &mut Vue, rx: &mpsc::Receiver<Msg>, agent: &ureq::Agent, ep: &Endpoint, tache: &Tache, journal: &mut Option<Journal>) -> Result<(), String> {
    let mut sale = true;
    let mut derniere = Instant::now() - IMAGE;
    let mut image_vue = u64::MAX;
    loop {
        // Tout ce qui est arrivé, d'un coup.
        while let Ok(m) = rx.try_recv() {
            match m {
                Msg::Trame(v) => vue.trame(&v),
                Msg::Boite(l) => vue.ligne_de_boite(&l),
                Msg::Note(n) => vue.note(&n),
                Msg::Politiques(d) => vue.politiques(&d),
                Msg::Entete(e) => vue.entete = e,
                Msg::EtatHote(e) => vue.etat_de_l_hote(e),
            }
            sale = true;
        }
        // Une animation change d'image à sa cadence (10 images/s au plus vite).
        if vue.quelle().is_some() {
            let n = vue.image(Instant::now());
            if n != image_vue {
                image_vue = n;
                sale = true;
            }
        }
        if sale && derniere.elapsed() >= IMAGE {
            let colorer = |texte: &str, langue: &str| if langue.is_empty() { None } else { client::surligner(agent, ep, texte, langue) };
            dessiner(terminal, vue, &colorer, journal.as_mut())?;
            sale = false;
            derniere = Instant::now();
        }
        let mut attente = if sale { IMAGE.saturating_sub(derniere.elapsed()) } else { IMAGE };
        if let Some(p) = vue.prochaine_image(Instant::now()) {
            attente = attente.min(p);
        }
        if event::poll(attente).map_err(|e| e.to_string())? {
            let action = match event::read().map_err(|e| e.to_string())? {
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    sale = true;
                    touche_de(k).and_then(|t| vue.touche(t))
                }
                // Un collage d'un bloc : ses sauts de ligne restent dans la saisie.
                Event::Paste(texte) => {
                    sale = true;
                    vue.touche(Touche::Coller(texte))
                }
                Event::Resize(l, _) => {
                    vue.redimensionner(l);
                    sale = true;
                    None
                }
                _ => None,
            };
            match action {
                Some(Action::Quitter) => return Ok(()),
                Some(Action::Envoyer(texte, kind)) => {
                    if let Err(e) = client::poster(agent, ep, "/ide/teammate/mailbox", &json!({ "backend": tache.depot.id, "task": tache.task, "kind": kind, "text": texte })) {
                        vue.note(&format!("refusé : {e}"));
                    }
                }
                Some(Action::Repondre { attente, texte, accord }) => {
                    if let Err(e) = client::poster(agent, ep, "/ide/teammate/mailbox", &crate::boite::reponse(&tache.depot.id, &tache.task, &attente, &texte, accord)) {
                        vue.note(&format!("la réponse est refusée : {e}"));
                    }
                }
                Some(Action::Arreter) => {
                    if let Err(e) = client::poster(agent, ep, "/ide/teammate/mailbox", &json!({ "backend": tache.depot.id, "task": tache.task, "kind": "stop", "text": "" })) {
                        vue.note(&format!("l'arrêt est refusé : {e}"));
                    } else {
                        vue.note("arrêt demandé");
                    }
                }
                None => {}
            }
        }
    }
}

/// La touche de la vue pour une touche du terminal, ou rien.
pub fn touche_de(k: KeyEvent) -> Option<Touche> {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let alt = k.modifiers.contains(KeyModifiers::ALT);
    match k.code {
        KeyCode::Char('c') if ctrl => Some(Touche::CtrlC),
        KeyCode::Char('d') if ctrl => Some(Touche::CtrlD),
        KeyCode::Char('g') if ctrl => Some(Touche::CtrlG),
        KeyCode::Char('a') if ctrl => Some(Touche::Debut),
        KeyCode::Char('e') if ctrl => Some(Touche::Fin),
        KeyCode::Char('w') if ctrl => Some(Touche::EffacerMot),
        KeyCode::Char('u') if ctrl => Some(Touche::EffacerDebut),
        KeyCode::Char('k') if ctrl => Some(Touche::EffacerFin),
        // Une autre combinaison Ctrl + lettre n'entre pas dans la saisie.
        KeyCode::Char(_) if ctrl => None,
        KeyCode::Char('b') if alt => Some(Touche::MotGauche),
        KeyCode::Char('f') if alt => Some(Touche::MotDroite),
        KeyCode::Char(_) if alt => None,
        KeyCode::Char(c) => Some(Touche::Car(c)),
        // Alt-Entrée partout ; Maj-Entrée là où le terminal la distingue.
        KeyCode::Enter if alt || k.modifiers.contains(KeyModifiers::SHIFT) => Some(Touche::Saut),
        KeyCode::Enter => Some(Touche::Entree),
        KeyCode::Backspace if alt || ctrl => Some(Touche::EffacerMot),
        KeyCode::Backspace => Some(Touche::Retour),
        KeyCode::Left if alt || ctrl => Some(Touche::MotGauche),
        KeyCode::Right if alt || ctrl => Some(Touche::MotDroite),
        KeyCode::Left => Some(Touche::Gauche),
        KeyCode::Right => Some(Touche::Droite),
        KeyCode::Home => Some(Touche::Debut),
        KeyCode::End => Some(Touche::Fin),
        KeyCode::Delete => Some(Touche::Suppr),
        KeyCode::Up => Some(Touche::Haut),
        KeyCode::Down => Some(Touche::Bas),
        KeyCode::Esc => Some(Touche::Echap),
        _ => None,
    }
}

/// Ce qui colore un bloc de code : `(texte, langue) → spans`, ou rien.
pub type Colorer<'a> = &'a dyn Fn(&str, &str) -> Option<Vec<crate::client::Span>>;

/// Les lignes validées dans l'historique (repliées, les blocs de code colorés
/// par `colorer`), puis la zone vivante. Les mêmes lignes, sans leurs couleurs,
/// vont au `journal` s'il y en a un.
pub fn dessiner<B: ratatui::backend::Backend>(terminal: &mut ratatui::Terminal<B>, vue: &mut Vue, colorer: Colorer, journal: Option<&mut Journal>) -> Result<(), String> {
    let validees: Vec<Validee> = vue.prendre_lignes();
    if !validees.is_empty() {
        let largeur = terminal.size().map(|s| s.width).unwrap_or(80) as usize;
        let mut lignes = Vec::new();
        for v in &validees {
            let spans = if v.genre == crate::vue::Genre::Code { colorer(&v.texte, &v.langue) } else { None };
            lignes.extend(crate::code::lignes(v, &vue.theme, vue.capacites, largeur, spans.as_deref()));
        }
        if let Some(j) = journal {
            let brutes: Vec<String> = lignes.iter().map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect()).collect();
            j.lignes(brutes.iter().map(String::as_str));
        }
        terminal
            .insert_before(lignes.len() as u16, |buf| {
                Paragraph::new(lignes).render(buf.area, buf);
            })
            .map_err(|e| format!("{e:?}"))?;
    }
    terminal.draw(|f| vue.dessiner(f, f.area())).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::{Backend, ClearType, CrosstermBackend, WindowSize};
    use ratatui::buffer::Cell;
    use ratatui::layout::{Position, Size};
    use ratatui::Terminal;
    use serde_json::json;
    use std::io::{self, Write};
    use std::sync::atomic::AtomicUsize;

    /// Un écrivain qui compte les octets et les jette.
    struct Compteur(Arc<AtomicUsize>);
    impl Write for Compteur {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.0.fetch_add(b.len(), Ordering::SeqCst);
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Le vrai backend crossterm, sans terminal : la taille est fixée, et tout ce
    /// qu'il écrirait est compté. Ce sont les octets qui passeraient par ssh.
    struct Compte {
        inner: CrosstermBackend<Compteur>,
        taille: Size,
    }
    impl Backend for Compte {
        type Error = io::Error;
        fn draw<'a, I: Iterator<Item = (u16, u16, &'a Cell)>>(&mut self, c: I) -> io::Result<()> {
            self.inner.draw(c)
        }
        fn append_lines(&mut self, n: u16) -> io::Result<()> {
            self.inner.append_lines(n)
        }
        fn hide_cursor(&mut self) -> io::Result<()> {
            self.inner.hide_cursor()
        }
        fn show_cursor(&mut self) -> io::Result<()> {
            self.inner.show_cursor()
        }
        fn get_cursor_position(&mut self) -> io::Result<Position> {
            Ok(Position::new(0, self.taille.height - HAUTEUR))
        }
        fn set_cursor_position<P: Into<Position>>(&mut self, p: P) -> io::Result<()> {
            self.inner.set_cursor_position(p)
        }
        fn clear(&mut self) -> io::Result<()> {
            self.inner.clear()
        }
        fn clear_region(&mut self, t: ClearType) -> io::Result<()> {
            self.inner.clear_region(t)
        }
        fn size(&self) -> io::Result<Size> {
            Ok(self.taille)
        }
        fn window_size(&mut self) -> io::Result<WindowSize> {
            Ok(WindowSize { columns_rows: self.taille, pixels: Size::new(0, 0) })
        }
        fn flush(&mut self) -> io::Result<()> {
            Backend::flush(&mut self.inner)
        }
    }

    /// Les octets émis pour `n` tokens d'un mot, l'écran redessiné toutes les
    /// `par_image` tokens.
    fn octets(n: usize, par_image: usize) -> usize {
        let compte = Arc::new(AtomicUsize::new(0));
        let backend = Compte { inner: CrosstermBackend::new(Compteur(compte.clone())), taille: Size::new(80, 24) };
        let mut t = Terminal::with_options(backend, TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }).unwrap();
        let mut vue = Vue::new(80, true);
        compte.store(0, Ordering::SeqCst);
        for i in 0..n {
            vue.trame(&json!({ "evenement": { "kind": "Token", "text": format!(" mot{i}") } }));
            if (i + 1) % par_image == 0 {
                dessiner(&mut t, &mut vue, &|_, _| None, None).unwrap();
            }
        }
        dessiner(&mut t, &mut vue, &|_, _| None, None).unwrap();
        compte.load(Ordering::SeqCst)
    }

    /// Les touches du terminal deviennent celles de l'éditeur : flèches, Début et
    /// Fin, Suppr, les raccourcis Ctrl et Alt d'un shell, Alt-Entrée.
    #[test]
    fn terminal_keys_map_to_the_line_editor() {
        let k = |code, m| touche_de(KeyEvent::new(code, m));
        let (rien, ctrl, alt) = (KeyModifiers::NONE, KeyModifiers::CONTROL, KeyModifiers::ALT);
        let attendus = [
            (k(KeyCode::Left, rien), Some(Touche::Gauche)),
            (k(KeyCode::Right, rien), Some(Touche::Droite)),
            (k(KeyCode::Home, rien), Some(Touche::Debut)),
            (k(KeyCode::End, rien), Some(Touche::Fin)),
            (k(KeyCode::Delete, rien), Some(Touche::Suppr)),
            (k(KeyCode::Up, rien), Some(Touche::Haut)),
            (k(KeyCode::Down, rien), Some(Touche::Bas)),
            (k(KeyCode::Left, ctrl), Some(Touche::MotGauche)),
            (k(KeyCode::Right, ctrl), Some(Touche::MotDroite)),
            (k(KeyCode::Left, alt), Some(Touche::MotGauche)),
            (k(KeyCode::Right, alt), Some(Touche::MotDroite)),
            (k(KeyCode::Char('b'), alt), Some(Touche::MotGauche)),
            (k(KeyCode::Char('f'), alt), Some(Touche::MotDroite)),
            (k(KeyCode::Char('a'), ctrl), Some(Touche::Debut)),
            (k(KeyCode::Char('e'), ctrl), Some(Touche::Fin)),
            (k(KeyCode::Char('w'), ctrl), Some(Touche::EffacerMot)),
            (k(KeyCode::Backspace, alt), Some(Touche::EffacerMot)),
            (k(KeyCode::Char('u'), ctrl), Some(Touche::EffacerDebut)),
            (k(KeyCode::Char('k'), ctrl), Some(Touche::EffacerFin)),
            (k(KeyCode::Enter, alt), Some(Touche::Saut)),
            (k(KeyCode::Enter, KeyModifiers::SHIFT), Some(Touche::Saut)),
            (k(KeyCode::Char('c'), ctrl), Some(Touche::CtrlC)),
            (k(KeyCode::Char('g'), ctrl), Some(Touche::CtrlG)),
            (k(KeyCode::Char('x'), ctrl), None),
            (k(KeyCode::Char('é'), rien), Some(Touche::Car('é'))),
            (k(KeyCode::Char('B'), KeyModifiers::SHIFT), Some(Touche::Car('B'))),
            (k(KeyCode::Enter, rien), Some(Touche::Entree)),
        ];
        let faux: Vec<_> = attendus.iter().enumerate().filter(|(_, (vu, attendu))| vu != attendu).map(|(i, (vu, attendu))| format!("#{i} : {vu:?} au lieu de {attendu:?}")).collect();
        assert!(faux.is_empty(), "{faux:#?}");
    }

    /// Le journal reçoit, en texte brut, chaque ligne envoyée dans l'historique,
    /// dans l'ordre : le texte replié à la largeur et le bloc de code entier.
    #[test]
    fn the_journal_receives_every_line_sent_to_the_history_without_colours() {
        let d = std::env::temp_dir().join(format!("tty-dessin-journal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut j = Journal::ouvrir(&d.join("j.txt")).unwrap();
        let backend = Compte { inner: CrosstermBackend::new(Compteur(Arc::new(AtomicUsize::new(0)))), taille: Size::new(40, 24) };
        let mut t = Terminal::with_options(backend, TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }).unwrap();
        let mut vue = Vue::new(40, true);
        for i in 0..30 {
            vue.trame(&json!({ "evenement": { "kind": "Token", "text": format!(" mot{i}") } }));
            dessiner(&mut t, &mut vue, &|_, _| None, Some(&mut j)).unwrap();
        }
        vue.trame(&json!({ "evenement": { "kind": "Token", "text": "\n```rust\nfn main() {}\n```\n" } }));
        dessiner(&mut t, &mut vue, &|_, _| None, Some(&mut j)).unwrap();
        let texte = std::fs::read_to_string(j.chemin()).unwrap();
        let mots: Vec<String> = texte.split_whitespace().filter(|m| m.starts_with("mot")).map(String::from).collect();
        assert_eq!(mots, (0..30).map(|i| format!("mot{i}")).collect::<Vec<_>>(), "{texte}");
        assert!(texte.contains("fn main() {}"), "{texte}");
        assert!(!texte.contains('\x1b'), "aucune séquence de couleur : {texte:?}");
        assert!(texte.lines().all(|l| l.chars().count() <= 40), "replié à la largeur : {texte}");
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Les octets émis pour `n` images d'une animation, la vue en `etat`.
    fn octets_d_animation(evenement: Value, images: u64) -> usize {
        let compte = Arc::new(AtomicUsize::new(0));
        let backend = Compte { inner: CrosstermBackend::new(Compteur(compte.clone())), taille: Size::new(80, 24) };
        let mut t = Terminal::with_options(backend, TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }).unwrap();
        let mut vue = Vue::new(80, true);
        vue.politiques(&json!({ "tools": [{ "name": "edit_file", "policy": { "write_files": true } }] }));
        vue.entete = "rag3db · tty-3 · tâche parle".into();
        vue.trame(&evenement);
        dessiner(&mut t, &mut vue, &|_, _| None, None).unwrap();
        let debut = std::time::Instant::now();
        compte.store(0, Ordering::SeqCst);
        let ms = vue.quelle().unwrap().de(&vue.theme).ms;
        for n in 1..=images {
            let a = debut + std::time::Duration::from_millis(n * ms);
            t.draw(|f| vue.dessiner_a(f, f.area(), a)).unwrap();
        }
        compte.load(Ordering::SeqCst)
    }

    /// **La mesure du lot tty-3** : ce que coûte une image du Daemon Code
    /// Summoning, l'écran sans autre changement (seule la ligne d'état bouge).
    /// Mesuré le 7 nov. 2026, couleur vraie, 80 colonnes, l'en-tête affiché :
    /// Cercle 82 o par image, Transmutation 60, Glitch 158. À leurs cadences,
    /// cela fait au plus 1,6 Ko/s (le Glitch, 10 images/s). Les bornes laissent un
    /// quart de marge.
    #[test]
    fn mesure_des_octets_par_image_d_animation() {
        let cercle = octets_d_animation(json!({ "evenement": { "kind": "LlmCall" } }), 100);
        let glitch = octets_d_animation(json!({ "evenement": { "kind": "ToolCallStarted", "tool": "edit_file" } }), 100);
        let lit = octets_d_animation(json!({ "evenement": { "kind": "ToolCallStarted", "tool": "read_file" } }), 100);
        eprintln!("MESURE tty-3 : 100 images — Cercle {cercle} o ({:.1} o/image), Transmutation {lit} o ({:.1}), Glitch {glitch} o ({:.1})", cercle as f64 / 100.0, lit as f64 / 100.0, glitch as f64 / 100.0);
        assert!(cercle > 0 && glitch > 0 && lit > 0);
        assert!(cercle <= 10_500, "Cercle : {cercle} o pour 100 images (mesuré 8 200)");
        assert!(lit <= 7_500, "Transmutation : {lit} o pour 100 images (mesuré 6 000)");
        assert!(glitch <= 20_000, "Glitch : {glitch} o pour 100 images (mesuré 15 812)");
    }

    /// **La mesure du lot tty-2** : ce que coûte un flux de 1 000 tokens à
    /// l'écran, une image par token (le pire) et une image pour dix (30 images/s
    /// à 300 tokens/s). Mesuré le 7 nov. 2026 : texte 6 890 o, une image par
    /// token 62 454 o (62,5 o/token), une pour dix 28 072 o (28,1 o/token) au lot
    /// tty-2 ; 63 174 et 28 792 au lot tty-3, où la ligne d'état prend la couleur
    /// vraie du thème au lieu du rouge de base (une séquence RGB par image), soit
    /// environ 280 o par image dont 69 de texte. Les bornes laissent un quart de
    /// marge sur ces chiffres : un changement qui les passe se justifie d'abord.
    #[test]
    fn mesure_des_octets_pour_mille_tokens() {
        let pire = octets(1000, 1);
        let groupe = octets(1000, 10);
        let texte: usize = (0..1000).map(|i| format!(" mot{i}").len()).sum();
        eprintln!("MESURE tty-2 : texte {texte} o ; une image par token {pire} o ({:.1} o/token) ; une pour dix {groupe} o ({:.1} o/token)", pire as f64 / 1000.0, groupe as f64 / 1000.0);
        assert!(groupe > texte, "le texte passe au moins une fois");
        assert!(groupe <= 35_000, "une image pour dix tokens : {groupe} o pour 1 000 tokens (mesuré 28 072)");
        assert!(pire <= 78_000, "une image par token : {pire} o pour 1 000 tokens (mesuré 62 454)");
    }
}
