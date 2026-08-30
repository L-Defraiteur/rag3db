//! **Exécuter une commande, sans donner la machine.**
//!
//! Le modèle l'a mis en première place : *« un agent qui ne peut pas tester ses
//! modifications est un agent qui produit du code cassé »*. Reste à le faire
//! sans que « lancer les tests » et « effacer le dépôt » passent par la même
//! porte.
//!
//! Conçu dans
//! `docs/30-aout-2026-04h00/03-donner-des-commandes-a-un-agent.md`.
//!
//! # La décision qui protège vraiment : pas de shell
//!
//! Une commande s'exécute par son **argv**, jamais par un interpréteur.
//! Sans cette règle, toute liste blanche est décorative : `cargo test; rm -rf ~`
//! commence par `cargo`, et un préfixe autorisé laisse passer ce qui le suit.
//!
//! Le prix est réel — pas de `|`, `>`, `&&`, `$(…)`, pas de joker — et il est
//! petit devant l'alternative. Qui veut un shell demande `sh -c`
//! explicitement, et c'est alors une commande dont la famille ne dit rien de ce
//! qu'elle fait : elle n'entrera jamais dans une liste blanche.

use std::collections::BTreeSet;
use std::sync::Mutex;

// ─── Ce qu'on exécute ────────────────────────────────────────────────────────

/// Un programme et ses arguments. **Jamais une ligne de commande.**
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Commande {
    pub programme: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// Les programmes dont le premier verbe change tout : `git status` et
/// `git push --force` n'ont rien à voir, et les mettre dans la même famille
/// reviendrait à autoriser le second en accordant le premier.
const MULTI_VERBES: &[&str] = &[
    "git", "cargo", "npm", "pnpm", "yarn", "docker", "kubectl", "go", "systemctl", "podman", "gh",
    "pip", "poetry", "make", "just", "terraform",
];

impl Commande {
    pub fn new(programme: impl Into<String>, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self { programme: programme.into(), args: args.into_iter().map(Into::into).collect() }
    }

    /// **La famille** : ce sur quoi une permission peut porter.
    ///
    /// Le programme seul, sauf pour ceux dont le premier verbe décide du sens.
    /// Les chemins et les options n'en font jamais partie : une permission qui
    /// dépendrait d'un chemin serait à réaccorder à chaque fichier.
    pub fn famille(&self) -> String {
        if MULTI_VERBES.contains(&self.programme.as_str()) {
            if let Some(verbe) = self.args.iter().find(|a| !a.starts_with('-')) {
                return format!("{} {verbe}", self.programme);
            }
        }
        self.programme.clone()
    }

    /// Pour l'affichage et les journaux. **Pas pour l'exécution** : ce qui est
    /// exécuté est l'argv, et une chaîne ne sert qu'à être lue.
    pub fn lisible(&self) -> String {
        std::iter::once(self.programme.clone())
            .chain(self.args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

// ─── Ce qu'on observe ────────────────────────────────────────────────────────

/// Ce qu'on a pu constater d'une commande, avant de la juger.
///
/// **Pourquoi on stocke les faits et pas seulement la décision** : une décision
/// enregistrée sans ses raisons ne se rejoue pas. Le jour où la politique
/// change, les faits permettent de re-trancher et d'auditer ; un booléen « ça
/// s'est bien passé » ne permet ni l'un ni l'autre.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Faits {
    /// Modifie des fichiers. **Vrai par défaut** pour ce qu'on ne connaît
    /// pas : dans le doute, une commande écrit.
    pub ecrit: bool,
    /// Sort de la machine.
    pub reseau: bool,
    /// Détruit ou réécrit sans retour : `rm`, `--force`, `reset --hard`.
    pub irreversible: bool,
    /// `sudo`, `doas`, `su`, `pkexec`.
    pub eleve: bool,
    /// `sh -c`, `bash -c` : le contenu échappe à toute analyse de famille.
    pub shell: bool,
}

/// Les familles qu'on sait être en lecture seule. Volontairement courte : une
/// liste qu'on allonge à la demande vaut mieux qu'une liste qu'on élague après
/// un incident.
const LECTURE_SEULE: &[&str] = &[
    "ls", "cat", "head", "tail", "wc", "file", "stat", "pwd", "which", "env", "date", "uname",
    "rg", "grep", "find", "fd", "tree", "du", "df", "ps",
    "git status", "git diff", "git log", "git show", "git branch", "git remote", "git blame",
    "cargo check", "cargo test", "cargo tree", "cargo metadata", "cargo fmt", "cargo clippy",
    "npm test", "npm ls", "go test", "go vet", "make -n",
];

const ELEVATION: &[&str] = &["sudo", "doas", "su", "pkexec", "runas"];
const SHELLS: &[&str] = &["sh", "bash", "zsh", "fish", "dash", "ksh", "csh", "tcsh", "pwsh"];
const RESEAU: &[&str] = &["curl", "wget", "ssh", "scp", "rsync", "nc", "ping", "ftp"];
const DESTRUCTEURS: &[&str] = &["rm", "rmdir", "shred", "dd", "mkfs", "truncate", "chown", "chmod"];

/// Ce qu'on peut dire d'une commande **sans l'exécuter**.
pub fn observer(c: &Commande) -> Faits {
    let famille = c.famille();
    let lecture_seule = LECTURE_SEULE.contains(&famille.as_str());
    let options: Vec<&str> = c.args.iter().map(String::as_str).collect();
    let force = options.iter().any(|a| *a == "--force" || *a == "-f" || *a == "--hard");

    Faits {
        // **Dans le doute, ça écrit.** L'inverse — supposer inoffensif ce
        // qu'on ne connaît pas — est exactement la faute qu'une liste blanche
        // existe pour empêcher.
        ecrit: !lecture_seule,
        reseau: RESEAU.contains(&c.programme.as_str())
            || matches!(famille.as_str(), "git push" | "git pull" | "git fetch" | "git clone")
            || matches!(famille.as_str(), "cargo publish" | "cargo install" | "npm install" | "npm publish"),
        irreversible: DESTRUCTEURS.contains(&c.programme.as_str())
            || (famille.starts_with("git ") && force)
            || famille == "git reset" && force,
        eleve: ELEVATION.contains(&c.programme.as_str()),
        shell: SHELLS.contains(&c.programme.as_str()) && options.contains(&"-c"),
    }
}

// ─── Ce qu'on décide ─────────────────────────────────────────────────────────

/// Ce qu'on fait, maintenant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Decision {
    Autorise,
    /// Quelqu'un doit trancher. **Ce n'est pas un refus poli** : c'est un état
    /// où la réponse n'existe pas encore.
    Demande,
    Refuse,
}

/// Pour quoi d'autre le verdict vaut. **La pièce qui empêche de redemander
/// cinquante fois.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Portee {
    /// Cet appel, et lui seul.
    CetteFois,
    /// Le même argv exactement, pour la session.
    CetteCommande,
    /// La même famille, pour la session.
    CetteFamille,
    /// La famille, écrite dans la configuration — au-delà de la session.
    Toujours,
}

/// Sur quoi le verdict repose.
///
/// **Séparer `UtilisateurExplicite` de `JugeeInoffensive` est le cœur du
/// dispositif** : une commande peut tourner parce qu'on l'a permise, ou parce
/// qu'elle *semble* anodine. Ce ne sont pas les mêmes risques, et les
/// confondre perd la trace du moment où un humain s'est engagé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Fondement {
    /// L'opérateur l'a écrit avant la session.
    Configuration,
    /// Quelqu'un a dit oui, à cette chose, dans cette session.
    UtilisateurExplicite,
    /// Une portée acquise plus tôt couvre ce cas.
    DejaAccorde,
    /// Personne n'a rien dit ; la sentinelle estime.
    JugeeInoffensive,
}

/// Le jugement porté sur une commande.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Verdict {
    pub decision: Decision,
    pub portee: Portee,
    pub fondement: Fondement,
    pub faits: Faits,
    /// À dire à l'humain. Une phrase, pas un code.
    pub motif: String,
}

impl Verdict {
    /// **La portée ne s'invente pas : elle se borne au fondement.**
    ///
    /// Un jugement d'innocuité n'a pas d'autorité, il a un avis : il ne vaut
    /// jamais plus que la commande exacte. Et ce qui est irréversible, élevé
    /// ou passé au shell reste `CetteFois` quoi qu'il arrive — on ne demande
    /// pas cinquante fois pour lire, on demande à chaque fois pour détruire.
    pub fn borner(mut self) -> Self {
        let plafond = match self.fondement {
            Fondement::JugeeInoffensive => Portee::CetteCommande,
            _ => Portee::Toujours,
        };
        if self.portee > plafond {
            self.portee = plafond;
        }
        if self.faits.irreversible || self.faits.eleve || self.faits.shell {
            self.portee = Portee::CetteFois;
        }
        self
    }
}

// ─── Qui juge ────────────────────────────────────────────────────────────────

/// Ce que la sentinelle sait de la session au moment de juger.
#[derive(Debug, Clone, Default)]
pub struct Contexte {
    /// L'utilisateur a-t-il explicitement accordé cette famille dans cette
    /// session ? C'est à l'appelant de le savoir ; la sentinelle ne devine pas
    /// une volonté humaine.
    pub accorde_par_l_utilisateur: bool,
    /// Le domaine de travail, pour dire ce qui en sort.
    pub domaine: Option<std::path::PathBuf>,
}

/// **Qui juge une commande.** Enfichable exprès : la première implémentation
/// est un jeu de règles sans modèle, une seconde interrogera un petit modèle,
/// et rien n'empêche d'en écrire une troisième.
pub trait Sentinelle: Send + Sync {
    fn juger(&self, commande: &Commande, contexte: &Contexte) -> Verdict;
    /// Pour les journaux et les refus.
    fn nom(&self) -> &str {
        "sentinelle"
    }
}

/// La sentinelle par défaut : des règles, pas de modèle.
///
/// **C'est délibéré.** Un mécanisme de sûreté qui a besoin d'un modèle distant
/// pour dire non a un mode de panne de trop : le jour où le modèle ne répond
/// pas, il faut encore savoir refuser.
#[derive(Debug, Default)]
pub struct SentinelleDeBase;

impl Sentinelle for SentinelleDeBase {
    fn nom(&self) -> &str {
        "règles"
    }

    fn juger(&self, c: &Commande, ctx: &Contexte) -> Verdict {
        let faits = observer(c);
        let famille = c.famille();

        if faits.eleve {
            return Verdict {
                decision: Decision::Refuse,
                portee: Portee::CetteFois,
                fondement: Fondement::Configuration,
                faits,
                motif: format!("`{famille}` élève les privilèges : jamais sans un humain devant."),
            };
        }
        if faits.shell {
            return Verdict {
                decision: Decision::Demande,
                portee: Portee::CetteFois,
                fondement: Fondement::Configuration,
                faits,
                motif: "un shell exécute ce qu'on lui passe : sa famille ne dit rien de ce qu'il fait."
                    .into(),
            };
        }

        if ctx.accorde_par_l_utilisateur {
            return Verdict {
                decision: Decision::Autorise,
                portee: Portee::CetteFamille,
                fondement: Fondement::UtilisateurExplicite,
                faits,
                motif: format!("`{famille}` : accordé par l'utilisateur dans cette session."),
            }
            .borner();
        }

        if !faits.ecrit && !faits.reseau {
            return Verdict {
                decision: Decision::Autorise,
                portee: Portee::CetteFamille,
                fondement: Fondement::Configuration,
                faits,
                motif: format!("`{famille}` est en lecture seule."),
            }
            .borner();
        }

        Verdict {
            decision: Decision::Demande,
            portee: Portee::CetteCommande,
            fondement: Fondement::JugeeInoffensive,
            faits,
            motif: format!(
                "`{famille}` {} : je ne peux pas l'accorder tout seul.",
                match (faits.ecrit, faits.reseau) {
                    (true, true) => "écrit et sort de la machine",
                    (true, false) => "écrit",
                    (false, true) => "sort de la machine",
                    (false, false) => "n'est pas dans la liste connue",
                }
            ),
        }
        .borner()
    }
}

// ─── Ce que la session a acquis ──────────────────────────────────────────────

/// Les portées accordées pendant la session.
///
/// **Elle ne survit pas à la session.** Une permission accordée pendant un
/// travail ne doit pas s'appliquer au suivant : l'écrire dans la configuration
/// est un geste, pas un effet de bord.
#[derive(Debug, Default)]
pub struct Autorisations {
    familles: Mutex<BTreeSet<String>>,
    commandes: Mutex<BTreeSet<String>>,
}

impl Autorisations {
    pub fn new() -> Self {
        Self::default()
    }

    /// Retient ce qu'un verdict accorde au-delà de cet appel.
    pub fn retenir(&self, c: &Commande, v: &Verdict) {
        if v.decision != Decision::Autorise {
            return;
        }
        match v.portee {
            Portee::CetteFois => {}
            Portee::CetteCommande => {
                self.commandes.lock().unwrap().insert(c.lisible());
            }
            Portee::CetteFamille | Portee::Toujours => {
                self.familles.lock().unwrap().insert(c.famille());
            }
        }
    }

    /// Une portée acquise couvre-t-elle ce cas ?
    pub fn couvre(&self, c: &Commande) -> bool {
        self.familles.lock().unwrap().contains(&c.famille())
            || self.commandes.lock().unwrap().contains(&c.lisible())
    }

    /// Ce qui a été accordé, pour l'afficher.
    pub fn accordees(&self) -> Vec<String> {
        let mut out: Vec<String> = self.familles.lock().unwrap().iter().cloned().collect();
        out.extend(self.commandes.lock().unwrap().iter().cloned());
        out
    }
}

// ─── Le mode, et la porte ────────────────────────────────────────────────────

/// Ce qu'un agent a le droit de tenter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// **Le défaut** : la liste de lecture seule, et rien d'autre. Ce qui en
    /// sort est refusé sans qu'on demande — un agent qui ne fait que lire ne
    /// casse rien et n'interrompt personne.
    #[default]
    Standard,
    /// La liste tourne librement ; le reste demande à l'humain.
    Approbation,
    /// La liste tourne librement ; le reste, la sentinelle tranche.
    Auto,
}

/// **La porte.** Tout passe par elle, y compris ce qui sera refusé.
pub struct Garde {
    mode: Mode,
    sentinelle: Box<dyn Sentinelle>,
    acquis: Autorisations,
}

impl Garde {
    pub fn new(mode: Mode) -> Self {
        Self { mode, sentinelle: Box::new(SentinelleDeBase), acquis: Autorisations::new() }
    }

    /// Une autre sentinelle — celle à modèle, ou la vôtre.
    pub fn avec_sentinelle(mut self, s: Box<dyn Sentinelle>) -> Self {
        self.sentinelle = s;
        self
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn acquis(&self) -> &Autorisations {
        &self.acquis
    }

    /// **Le verdict, et lui seul.** Rien ne s'exécute ici : décider et faire
    /// sont deux gestes, et les séparer permet de montrer le premier.
    pub fn juger(&self, c: &Commande, ctx: &Contexte) -> Verdict {
        // Une portée acquise passe avant tout : c'est ce qui évite de
        // redemander, et d'appeler un modèle pour rien.
        if self.acquis.couvre(c) {
            return Verdict {
                decision: Decision::Autorise,
                portee: Portee::CetteFois,
                fondement: Fondement::DejaAccorde,
                faits: observer(c),
                motif: format!("`{}` : déjà accordé dans cette session.", c.famille()),
            };
        }

        let v = self.sentinelle.juger(c, ctx).borner();
        let v = match self.mode {
            // En standard, ce qui n'est pas déjà autorisé est **refusé**, pas
            // mis en attente : il n'y a personne pour répondre.
            Mode::Standard if v.decision != Decision::Autorise => Verdict {
                decision: Decision::Refuse,
                motif: format!(
                    "{} Mode standard : seule la lecture est permise. Les familles connues sont : {}.",
                    v.motif,
                    LECTURE_SEULE.join(", ")
                ),
                ..v
            },
            // En approbation, une estimation ne suffit pas : on demande.
            Mode::Approbation if v.fondement == Fondement::JugeeInoffensive => {
                Verdict { decision: Decision::Demande, ..v }
            }
            _ => v,
        };
        self.acquis.retenir(c, &v);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(p: &str, a: &[&str]) -> Commande {
        Commande::new(p, a.iter().copied())
    }

    // ── La famille ──────────────────────────────────────────────────────

    /// **`git` n'est pas une famille ; `git status` en est une.** Accorder
    /// `git` donnerait `git push --force` par la même occasion.
    #[test]
    fn la_famille_prend_le_verbe_quand_il_decide_du_sens() {
        assert_eq!(cmd("git", &["status"]).famille(), "git status");
        assert_eq!(cmd("git", &["push", "--force"]).famille(), "git push");
        assert_eq!(cmd("cargo", &["--offline", "test", "--lib"]).famille(), "cargo test");
        // Un programme ordinaire garde le sien : le chemin n'en fait pas partie,
        // sinon la permission serait à réaccorder à chaque fichier.
        assert_eq!(cmd("ls", &["-la", "/tmp"]).famille(), "ls");
        assert_eq!(cmd("rg", &["motif", "src/"]).famille(), "rg");
    }

    // ── Les faits ───────────────────────────────────────────────────────

    /// **Dans le doute, ça écrit.** Supposer inoffensif ce qu'on ne connaît pas
    /// est exactement la faute qu'une liste blanche existe pour empêcher.
    #[test]
    fn ce_qu_on_ne_connait_pas_est_suppose_ecrire() {
        assert!(observer(&cmd("programme_inconnu", &[])).ecrit);
        assert!(!observer(&cmd("git", &["status"])).ecrit);
        assert!(!observer(&cmd("cargo", &["test"])).ecrit);
    }

    #[test]
    fn on_reconnait_l_elevation_le_shell_et_l_irreversible() {
        assert!(observer(&cmd("sudo", &["ls"])).eleve);
        assert!(observer(&cmd("sh", &["-c", "n'importe quoi"])).shell);
        assert!(observer(&cmd("rm", &["-rf", "/"])).irreversible);
        assert!(observer(&cmd("git", &["push", "--force"])).irreversible);
        assert!(!observer(&cmd("git", &["push"])).irreversible);
        assert!(observer(&cmd("curl", &["https://exemple"])).reseau);
        assert!(observer(&cmd("git", &["push"])).reseau);
    }

    // ── La portée ───────────────────────────────────────────────────────

    /// **Un avis n'a pas d'autorité.** Une innocuité estimée ne vaut jamais
    /// pour toute une famille : elle vaut pour ce qu'on a regardé.
    #[test]
    fn une_estimation_ne_couvre_jamais_une_famille() {
        let v = Verdict {
            decision: Decision::Autorise,
            portee: Portee::CetteFamille,
            fondement: Fondement::JugeeInoffensive,
            faits: Faits::default(),
            motif: String::new(),
        }
        .borner();
        assert_eq!(v.portee, Portee::CetteCommande);
    }

    /// **Ce qui détruit se redemande à chaque fois**, même après un « oui ».
    #[test]
    fn l_irreversible_ne_s_allowliste_pas() {
        for faits in [
            Faits { irreversible: true, ..Default::default() },
            Faits { eleve: true, ..Default::default() },
            Faits { shell: true, ..Default::default() },
        ] {
            let v = Verdict {
                decision: Decision::Autorise,
                portee: Portee::Toujours,
                fondement: Fondement::UtilisateurExplicite,
                faits,
                motif: String::new(),
            }
            .borner();
            assert_eq!(v.portee, Portee::CetteFois, "faits : {faits:?}");
        }
    }

    // ── Les modes ───────────────────────────────────────────────────────

    #[test]
    fn en_standard_la_lecture_passe_et_le_reste_est_refuse_avec_la_liste() {
        let g = Garde::new(Mode::Standard);
        let ctx = Contexte::default();

        let v = g.juger(&cmd("git", &["status"]), &ctx);
        assert_eq!(v.decision, Decision::Autorise);
        assert_eq!(v.fondement, Fondement::Configuration);

        let v = g.juger(&cmd("git", &["push"]), &ctx);
        assert_eq!(v.decision, Decision::Refuse);
        // « qui dit non avec la liste » — un refus muet enverrait l'agent
        // réessayer autrement.
        assert!(v.motif.contains("git status"), "le refus doit nommer ce qui est permis : {}", v.motif);
    }

    #[test]
    fn en_approbation_ce_qui_ecrit_demande() {
        let g = Garde::new(Mode::Approbation);
        let ctx = Contexte::default();
        assert_eq!(g.juger(&cmd("cargo", &["test"]), &ctx).decision, Decision::Autorise);
        assert_eq!(g.juger(&cmd("cargo", &["build"]), &ctx).decision, Decision::Demande);
    }

    /// **Le cœur du dispositif** : un « oui » d'humain vaut pour la famille, et
    /// on ne redemande plus.
    #[test]
    fn un_oui_de_l_utilisateur_ne_se_redemande_pas() {
        let g = Garde::new(Mode::Auto);
        let accorde = Contexte { accorde_par_l_utilisateur: true, ..Default::default() };
        let muet = Contexte::default();

        let v = g.juger(&cmd("cargo", &["build", "--release"]), &accorde);
        assert_eq!(v.decision, Decision::Autorise);
        assert_eq!(v.fondement, Fondement::UtilisateurExplicite);
        assert_eq!(v.portee, Portee::CetteFamille);

        // Une autre commande de la même famille passe **sans** que l'appelant
        // ait à redire que l'utilisateur était d'accord.
        let v = g.juger(&cmd("cargo", &["build"]), &muet);
        assert_eq!(v.decision, Decision::Autorise);
        assert_eq!(v.fondement, Fondement::DejaAccorde);

        // Mais pas une autre famille du même programme.
        assert_eq!(g.juger(&cmd("cargo", &["publish"]), &muet).decision, Decision::Demande);
    }

    /// Et un « oui » sur quelque chose d'irréversible ne s'étend jamais.
    #[test]
    fn un_oui_sur_du_destructeur_ne_couvre_que_cette_fois() {
        let g = Garde::new(Mode::Auto);
        let accorde = Contexte { accorde_par_l_utilisateur: true, ..Default::default() };
        let v = g.juger(&cmd("rm", &["vieux.txt"]), &accorde);
        assert_eq!(v.portee, Portee::CetteFois);
        assert!(g.acquis().accordees().is_empty(), "rien ne doit être retenu");
        // Donc la fois suivante repasse par la case départ.
        assert_ne!(g.juger(&cmd("rm", &["autre.txt"]), &Contexte::default()).decision, Decision::Autorise);
    }

    #[test]
    fn l_elevation_est_refusee_meme_en_auto() {
        let g = Garde::new(Mode::Auto);
        let accorde = Contexte { accorde_par_l_utilisateur: true, ..Default::default() };
        let v = g.juger(&cmd("sudo", &["cargo", "test"]), &accorde);
        assert_eq!(v.decision, Decision::Refuse);
    }

    /// Un shell demande toujours : sa famille ne dit rien de ce qu'il fait.
    #[test]
    fn un_shell_demande_toujours() {
        let g = Garde::new(Mode::Auto);
        let accorde = Contexte { accorde_par_l_utilisateur: true, ..Default::default() };
        let v = g.juger(&cmd("bash", &["-c", "cargo test"]), &accorde);
        assert_eq!(v.decision, Decision::Demande);
        assert_eq!(v.portee, Portee::CetteFois);
    }

    /// **Le verdict se sérialise.** C'est ce qui permet de le tracer, de le
    /// rejouer, et de re-trancher le jour où la politique change.
    #[test]
    fn un_verdict_se_range_et_se_relit() {
        let g = Garde::new(Mode::Auto);
        let v = g.juger(&cmd("git", &["push"]), &Contexte::default());
        let json = serde_json::to_string(&v).expect("sérialisation");
        let relu: Verdict = serde_json::from_str(&json).expect("relecture");
        assert_eq!(relu, v);
        assert!(json.contains("reseau"), "les faits voyagent avec : {json}");
    }
}
