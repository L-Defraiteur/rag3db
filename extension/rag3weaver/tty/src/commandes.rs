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
    /// `!…` : une commande lancée là où tourne le terminal.
    Shell(String),
    Inconnue(String),
}

/// Les commandes `/`, avec leur aide, dans l'ordre de `/aide`.
pub const COMMANDES: &[(&str, &str)] = &[
    ("aide", "les touches et les commandes"),
    ("arreter", "arrête l'équipier (comme Ctrl-G)"),
    ("context", "le remplissage du contexte du modèle de l'équipier"),
    ("depot", "<nom> : passe à un autre dépôt"),
    ("effacer", "efface l'écran visible ; « /effacer historique » efface l'historique des envois"),
    ("journal", "où relire la séance"),
    ("quitter", "quitte le terminal"),
    ("tache", "<nom> : passe à une autre tâche"),
    ("theme", "<nom> : change de thème"),
];

/// Ce que la saisie demande : une commande, ou `None` pour un envoi à
/// l'équipier (le texte à envoyer, un `//` de tête ramené à `/`, en second).
pub fn lire(_texte: &str) -> (Option<Commande>, Option<String>) {
    (None, None)
}

/// Ce que Tab peut proposer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sources {
    pub themes: Vec<String>,
    pub taches: Vec<String>,
    pub depots: Vec<String>,
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
pub fn completer(_texte: &str, _s: &Sources) -> Option<Completion> {
    None
}

/// Le dernier mot de `texte` sert-il un chemin de fichier (`!` après la commande) ?
pub fn mot_de_fichier(_texte: &str) -> Option<&str> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(completer("/e", &s).map(|c| c.candidats), Some(vec!["effacer".into()]).filter(|_| false).or(Some(vec![])), "une seule commande en e : complétée");
        assert_eq!(completer("/e", &s).map(|c| c.texte), Some("/effacer ".into()));
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
