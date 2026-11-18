//! **Les commandes du terminal** (lot tty-13) : `/…` pour le terminal lui-même,
//! `!…` pour une commande lancée là où il tourne, et Tab qui complète les deux.
//!
//! Une saisie qui commence par `/` est une commande du terminal, jamais un envoi
//! à l'équipier ; `//` en tête envoie le texte avec un seul `/`. Une commande `!`
//! ne vient que de la saisie, validée par Entrée : ni le modèle, ni un outil, ni
//! un texte collé ne la lancent.

/// Une commande du terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Commande {
    Aide,
    Quitter,
    Arreter,
    Theme(Option<String>),
    Tache(Option<String>),
    Depot(Option<String>),
    Journal,
    Effacer,
    EffacerHistorique,
    Contexte,
    /// `/etapes` : montrer ou cacher dans l'historique les étapes de l'équipier.
    Etapes,
    /// `/model` : les modèles de la tâche, ou en choisir un par son nom (tty-14).
    Modele(Option<String>),
    /// `/nom` : le nom de l'agent de la tâche, ou le changer (c7, 15 nov.).
    Nom(Option<String>),
    /// `!…` : une commande lancée là où tourne le terminal.
    Shell(String),
    Inconnue(String),
}

/// Les commandes `/`, avec leur aide, dans l'ordre de `/aide`.
pub const COMMANDES: &[(&str, &str)] = &[
    ("help", "les touches et les commandes (ancien : /aide)"),
    ("aide", "les touches et les commandes"),
    ("stop", "arrête l'équipier (comme Ctrl-G) (ancien : /arreter)"),
    ("arreter", "arrête l'équipier (comme Ctrl-G)"),
    ("context", "le remplissage du contexte du modèle de l'équipier"),
    ("repo", "<nom> : passe à un autre dépôt (ancien : /depot)"),
    ("depot", "<nom> : passe à un autre dépôt"),
    ("clear", "efface l'écran visible ; « /clear history » efface l'historique des envois (ancien : /effacer)"),
    ("effacer", "efface l'écran visible ; « /effacer historique » efface l'historique des envois"),
    ("steps", "montre ou cache dans l'historique les appels d'outil et les étapes de l'équipier (ancien : /etapes)"),
    ("etapes", "montre ou cache dans l'historique les appels d'outil et les étapes de l'équipier"),
    ("log", "où relire la séance (ancien : /journal)"),
    ("journal", "où relire la séance"),
    ("model", "<nom> : change le modèle de la tâche ; seul, les modèles déclarés"),
    ("name", "<x> : nomme l'agent de la tâche ; seul, son nom ; « /nom - » reprend celui du dépôt (ancien : /nom)"),
    ("nom", "<x> : nomme l'agent de la tâche ; seul, son nom ; « /nom - » reprend celui du dépôt"),
    ("quit", "quitte le terminal (ancien : /quitter)"),
    ("quitter", "quitte le terminal"),
    ("task", "<nom> : passe à une autre tâche (ancien : /tache)"),
    ("tache", "<nom> : passe à une autre tâche"),
    ("theme", "<nom> : change de thème"),
];

/// Ce que la saisie demande : une commande, ou `None` pour un envoi à
/// l'équipier (le texte à envoyer, un `//` de tête ramené à `/`, en second).
pub fn lire(texte: &str) -> (Option<Commande>, Option<String>) {
    let t = texte.trim_start();
    if let Some(reste) = t.strip_prefix("//") {
        return (None, Some(format!("/{reste}")));
    }
    if let Some(reste) = t.strip_prefix('/') {
        let mut mots = reste.split_whitespace();
        let nom = mots.next().unwrap_or_default();
        let arg: Vec<&str> = mots.collect();
        let arg = (!arg.is_empty()).then(|| arg.join(" "));
        let c = match nom {
            "aide" | "help" => Commande::Aide,
            "quitter" | "quit" => Commande::Quitter,
            "arreter" | "stop" => Commande::Arreter,
            "theme" => Commande::Theme(arg),
            "tache" | "task" => Commande::Tache(arg),
            "depot" | "repo" => Commande::Depot(arg),
            "journal" | "log" => Commande::Journal,
            "clear" if arg.as_deref() == Some("history") => Commande::EffacerHistorique,
            "clear" => Commande::Effacer,
            "effacer" if arg.as_deref() == Some("historique") => Commande::EffacerHistorique,
            "effacer" => Commande::Effacer,
            "context" | "contexte" => Commande::Contexte,
            "etapes" | "steps" => Commande::Etapes,
            "model" | "modele" => Commande::Modele(arg),
            "nom" | "name" => Commande::Nom(arg),
            autre => Commande::Inconnue(autre.to_string()),
        };
        return (Some(c), None);
    }
    if let Some(reste) = t.strip_prefix('!') {
        let cmd = reste.trim();
        return (Some(if cmd.is_empty() { Commande::Inconnue("!".into()) } else { Commande::Shell(cmd.to_string()) }), None);
    }
    (None, Some(texte.to_string()))
}

/// Le plus long préfixe commun de `mots`.
fn prefixe_commun(mots: &[&str]) -> String {
    let Some(premier) = mots.first() else { return String::new() };
    let mut n = premier.chars().count();
    for m in &mots[1..] {
        n = n.min(premier.chars().zip(m.chars()).take_while(|(a, b)| a == b).count());
    }
    premier.chars().take(n).collect()
}

/// Complète `mot` parmi `candidats` : le texte qui le remplace (suivi d'une
/// espace quand un seul candidat convient, sauf un dossier), et les candidats.
fn completer_mot(mot: &str, candidats: &[String], espace: bool) -> Option<(String, Vec<String>)> {
    let mut ok: Vec<&str> = candidats.iter().map(String::as_str).filter(|c| c.starts_with(mot)).collect();
    ok.sort();
    ok.dedup();
    match ok.len() {
        0 => None,
        1 => {
            let fin = if espace && !ok[0].ends_with('/') { " " } else { "" };
            Some((format!("{}{fin}", ok[0]), Vec::new()))
        }
        _ => Some((prefixe_commun(&ok), ok.iter().map(|c| c.to_string()).collect())),
    }
}

/// Ce que Tab peut proposer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sources {
    pub themes: Vec<String>,
    pub taches: Vec<String>,
    pub depots: Vec<String>,
    /// Les noms des modèles déclarés pour la tâche (`/model`).
    pub modeles: Vec<String>,
    /// Les exécutables du `PATH`, pour le premier mot d'un `!`.
    pub executables: Vec<String>,
    /// Les chemins du dossier courant qui commencent par un préfixe donné.
    pub fichiers: Vec<String>,
}

/// Une complétion : le texte qui remplace la saisie, et les candidats quand
/// il y en a plusieurs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub texte: String,
    pub candidats: Vec<String>,
}

/// **Tab** : complète le dernier mot de `texte` (la saisie jusqu'au curseur)
/// par ce que proposent les `sources`, jusqu'au plus long préfixe commun.
pub fn completer(texte: &str, s: &Sources) -> Option<Completion> {
    let (avant, mot) = match texte.rfind(char::is_whitespace) {
        Some(i) => texte.split_at(i + 1),
        None => ("", texte),
    };
    let (remplace, candidats) = if let Some(reste) = texte.strip_prefix('/') {
        if !reste.contains(char::is_whitespace) {
            let noms: Vec<String> = COMMANDES.iter().map(|(n, _)| n.to_string()).collect();
            let (m, c) = completer_mot(reste, &noms, true)?;
            (format!("/{m}"), c)
        } else {
            let nom = reste.split_whitespace().next().unwrap_or_default();
            let liste = match nom {
                "theme" => s.themes.clone(),
                "tache" | "task" => s.taches.clone(),
                "depot" | "repo" => s.depots.clone(),
                "model" => s.modeles.clone(),
                "effacer" => vec!["historique".to_string()],
                "clear" => vec!["history".to_string()],
                _ => return None,
            };
            let (m, c) = completer_mot(mot, &liste, false)?;
            (format!("{avant}{m}"), c)
        }
    } else {
        let reste = texte.strip_prefix('!')?;
        if !reste.contains(char::is_whitespace) {
            let (m, c) = completer_mot(reste, &s.executables, true)?;
            (format!("!{m}"), c)
        } else {
            let (m, c) = completer_mot(mot, &s.fichiers, true)?;
            (format!("{avant}{m}"), c)
        }
    };
    Some(Completion { texte: remplace, candidats })
}

/// Le dernier mot de `texte` sert-il un chemin de fichier (`!` après la commande) ?
pub fn mot_de_fichier(texte: &str) -> Option<&str> {
    let reste = texte.strip_prefix('!')?;
    let i = reste.rfind(char::is_whitespace)?;
    Some(&reste[i + 1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_commands_preserve_legacy_semantics() {
        for (english, legacy) in [
            ("/help", "/aide"), ("/quit", "/quitter"),
            ("/stop", "/arreter"), ("/task sample", "/tache sample"),
            ("/repo sample", "/depot sample"), ("/log", "/journal"),
            ("/clear", "/effacer"), ("/clear history", "/effacer historique"),
            ("/steps", "/etapes"), ("/name Sample", "/nom Sample"),
        ] {
            assert_eq!(lire(english), lire(legacy), "{english}");
        }
        assert_eq!(lire("//help").1.as_deref(), Some("/help"));
    }

    #[test]
    fn english_completion_offers_steps_and_stop_and_task_arguments() {
        let sources = Sources { taches: vec!["sample".into()], ..Default::default() };
        let result = completer("/st", &sources).expect("English commands");
        assert_eq!(result.candidats, vec!["steps", "stop"]);
        assert_eq!(completer("/task sam", &sources).unwrap().texte, "/task sample");
        assert_eq!(completer("/clear h", &sources).unwrap().texte, "/clear history");
    }

    /// Une saisie en `/` est une commande, en `//` un envoi avec un seul `/`,
    /// en `!` une commande à lancer ; le reste part à l'équipier.
    #[test]
    fn a_slash_is_a_command_a_double_slash_is_sent_and_a_bang_runs() {
        let c = |t: &str| lire(t).0;
        assert_eq!(c("/aide"), Some(Commande::Aide));
        assert_eq!(c("  /quitter "), Some(Commande::Quitter));
        assert_eq!(c("/theme foret"), Some(Commande::Theme(Some("foret".into()))));
        assert_eq!(c("/theme"), Some(Commande::Theme(None)));
        assert_eq!(c("/tache essai"), Some(Commande::Tache(Some("essai".into()))));
        assert_eq!(c("/depot rag3db"), Some(Commande::Depot(Some("rag3db".into()))));
        assert_eq!(c("/effacer"), Some(Commande::Effacer));
        assert_eq!(c("/effacer historique"), Some(Commande::EffacerHistorique));
        assert_eq!(c("/context"), Some(Commande::Contexte));
        assert_eq!(c("/etapes"), Some(Commande::Etapes));
        assert_eq!(c("/model"), Some(Commande::Modele(None)));
        assert_eq!(c("/model rapide"), Some(Commande::Modele(Some("rapide".into()))));
        assert_eq!(c("/nom Ombre"), Some(Commande::Nom(Some("Ombre".into()))), "c7 : nommer l'agent de la tâche");
        assert_eq!(c("/nom"), Some(Commande::Nom(None)));
        assert_eq!(c("/arreter"), Some(Commande::Arreter));
        assert_eq!(c("/journal"), Some(Commande::Journal));
        assert_eq!(c("/nimporte"), Some(Commande::Inconnue("nimporte".into())));
        assert_eq!(c("!sudo pacman -Syu"), Some(Commande::Shell("sudo pacman -Syu".into())));
        assert_eq!(c("!"), Some(Commande::Inconnue("!".into())), "un ! seul ne lance rien");
        assert_eq!(lire("//etc/hosts est lu"), (None, Some("/etc/hosts est lu".into())));
        assert_eq!(lire("salut /aide"), (None, Some("salut /aide".into())), "seul le premier caractère compte");
        assert_eq!(lire("ligne\n!echo x"), (None, Some("ligne\n!echo x".into())), "un ! en tête d'une autre ligne ne lance rien");
    }

    /// Tab complète le nom d'une commande, puis son argument, jusqu'au plus
    /// long préfixe commun, et dit les candidats quand il y en a plusieurs.
    #[test]
    fn tab_completes_commands_their_arguments_and_lists_the_candidates() {
        let s = Sources { themes: vec!["daemon".into(), "foret".into(), "cyberpunk".into(), "cyborg".into()], taches: vec!["essai-tty".into(), "essai-ide".into()], ..Default::default() };
        assert_eq!(completer("/th", &s), Some(Completion { texte: "/theme ".into(), candidats: vec![] }));
        assert_eq!(completer("/theme cy", &s), Some(Completion { texte: "/theme cyb".into(), candidats: vec!["cyberpunk".into(), "cyborg".into()] }));
        assert_eq!(completer("/theme f", &s), Some(Completion { texte: "/theme foret".into(), candidats: vec![] }));
        assert_eq!(completer("/tache essai-", &s), Some(Completion { texte: "/tache essai-".into(), candidats: vec!["essai-ide".into(), "essai-tty".into()] }));
        assert_eq!(completer("/e", &s), Some(Completion { texte: "/e".into(), candidats: vec!["effacer".into(), "etapes".into()] }), "deux commandes en e : leurs noms");
        assert_eq!(completer("/ef", &s).map(|c| c.texte), Some("/effacer ".into()));
        let m = Sources { modeles: vec!["rapide".into(), "fin".into()], ..Default::default() };
        assert_eq!(completer("/model r", &m), Some(Completion { texte: "/model rapide".into(), candidats: vec![] }));
        assert_eq!(completer("/mo", &m), Some(Completion { texte: "/model ".into(), candidats: vec![] }));
        assert_eq!(completer("/zz", &s), None, "rien à proposer");
        assert_eq!(completer("bonjour", &s), None, "un envoi ne se complète pas");
    }

    /// Pour `!`, le premier mot se complète par les exécutables, les suivants
    /// par les fichiers.
    #[test]
    fn tab_completes_a_bang_by_executables_then_files() {
        let s = Sources { executables: vec!["git".into(), "gitk".into(), "grep".into()], fichiers: vec!["src/".into(), "src.txt".into()], ..Default::default() };
        assert_eq!(completer("!gi", &s), Some(Completion { texte: "!git".into(), candidats: vec!["git".into(), "gitk".into()] }));
        assert_eq!(completer("!gr", &s), Some(Completion { texte: "!grep ".into(), candidats: vec![] }));
        assert_eq!(mot_de_fichier("!cat sr"), Some("sr"));
        assert_eq!(mot_de_fichier("!ca"), None, "le premier mot est la commande");
        assert_eq!(mot_de_fichier("/theme f"), None);
        assert_eq!(completer("!cat sr", &s), Some(Completion { texte: "!cat src".into(), candidats: vec!["src.txt".into(), "src/".into()] }));
        let un = Sources { fichiers: vec!["src/".into()], ..Default::default() };
        assert_eq!(completer("!cat sr", &un), Some(Completion { texte: "!cat src/".into(), candidats: vec![] }), "un dossier ne prend pas d'espace");
    }
}
