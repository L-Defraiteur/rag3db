//! **Réduire une ligne de commande en argv — ou refuser en le nommant.**
//!
//! Ce module ne juge rien. Il produit des **faits** : « cette ligne, ce sont
//! ces invocations-là », ou « je ne sais pas la réduire, et voici pourquoi ».
//! Juger est le travail de `rag3weaver::commande`, et les deux sont séparés
//! exprès : un parseur qui décide serait un parseur qu'on ne peut pas tester
//! sans une politique.
//!
//! # Pourquoi un parseur, et pas des motifs
//!
//! Deux agents open source examinés le 30 août 2026 filtrent les commandes par
//! motifs sur une chaîne — l'un par découpage à la regex avant une liste
//! blanche, l'autre en aplatissant l'argv avant une liste noire. Aucun ne
//! parse, et l'un des deux porte le TODO qui le dit.
//!
//! Le problème d'une liste blanche sans parseur : `git status && rm -rf ~`
//! commence par `git status`. Celui d'une liste noire : elle énumère le mal,
//! et se trompe dès la onzième forme.
//!
//! Un parseur permet la seule règle qui tienne : **on n'exécute que ce qu'on a
//! su réduire**. Ce qui n'entre pas dans la forme attendue n'est pas « permis
//! par défaut », il est refusé avec son nom.
//!
//! # Ce qu'on réduit, et ce qu'on refuse
//!
//! Réduit : une commande simple, et des commandes simples enchaînées par
//! `&&`, `||`, `;` ou `|`. Chacune devient une [`Invocation`] que la politique
//! jugera — **toutes**, pas seulement la première.
//!
//! Refusé, et nommé : substitution (`$(…)`), expansion (`$VAR`), redirection
//! (`>`), arrière-plan (`&`), joker (`*`), et tout nœud qu'on ne sait pas
//! réduire. Le refus est une information, pas un échec : il dit à l'appelant
//! quoi changer.

use tree_sitter::{Node, Parser};

/// Une commande simple, telle qu'on l'exécutera : un programme, des arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub programme: String,
    pub args: Vec<String>,
    /// Reçoit la sortie d'une autre commande (`… | ceci`).
    ///
    /// **Ça change la nature de la chose.** `sh` seul attend son entrée du
    /// terminal ; `curl … | sh` exécute ce qui arrive par le tuyau. Perdre
    /// cette information, c'est laisser passer la forme d'attaque la plus
    /// banale qui soit.
    pub tuyau_entrant: bool,
    /// Comment cette invocation est liée à la précédente.
    pub liaison: Liaison,
}

/// Ce qui joint deux commandes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liaison {
    /// La première de la ligne.
    Premiere,
    /// `&&` — seulement si la précédente a réussi.
    SiReussi,
    /// `||` — seulement si la précédente a échoué.
    SiEchoue,
    /// `;` — dans tous les cas.
    Puis,
    /// `|` — reçoit la sortie de la précédente.
    Tuyau,
}

/// Pourquoi on n'a pas su réduire.
///
/// **Chaque variante est une chose qu'on pourrait décider de supporter un
/// jour.** Un refus générique fermerait cette porte et ne dirait rien à
/// l'appelant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refus {
    /// `$(…)` ou `` `…` `` : le contenu ne sera connu qu'à l'exécution.
    Substitution,
    /// `$VAR`, `${…}` : idem.
    Expansion,
    /// `>`, `<`, `>>`, `<<` : écrit ailleurs que là où on regarde.
    Redirection,
    /// `&` : la commande survit à l'appel.
    ArrierePlan,
    /// `*`, `?`, `[…]` : ce qui sera passé au programme dépend du disque.
    Joker,
    /// La ligne ne se parse pas.
    Syntaxe,
    /// Vide, ou seulement des commentaires.
    Vide,
    /// Une construction qu'on ne sait pas réduire — le nom du nœud, pour
    /// qu'on puisse décider plus tard si elle mérite d'être supportée.
    Inconnu(String),
}

impl std::fmt::Display for Refus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Substitution => write!(f, "substitution de commande : ce qui sera exécuté n'est pas connu d'avance"),
            Self::Expansion => write!(f, "expansion de variable : la valeur n'est pas connue d'avance"),
            Self::Redirection => write!(f, "redirection : écrit ailleurs que là où on regarde"),
            Self::ArrierePlan => write!(f, "arrière-plan (&) : la commande survivrait à l'appel"),
            Self::Joker => write!(f, "joker (*, ?) : ce qui sera passé dépend du disque"),
            Self::Syntaxe => write!(f, "la ligne ne se parse pas"),
            Self::Vide => write!(f, "rien à exécuter"),
            Self::Inconnu(k) => write!(f, "construction non réduite : `{k}`"),
        }
    }
}

/// **Réduire une ligne en invocations, ou refuser.**
pub fn decomposer(ligne: &str) -> Result<Vec<Invocation>, Refus> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bash::LANGUAGE.into())
        .map_err(|_| Refus::Syntaxe)?;
    let arbre = parser.parse(ligne, None).ok_or(Refus::Syntaxe)?;
    let racine = arbre.root_node();
    if racine.has_error() {
        return Err(Refus::Syntaxe);
    }

    let source = ligne.as_bytes();
    let mut out = Vec::new();
    visiter(racine, source, Liaison::Premiere, &mut out)?;
    if out.is_empty() {
        return Err(Refus::Vide);
    }
    Ok(out)
}

/// Parcourt l'arbre et rend les commandes simples, dans l'ordre.
///
/// **Tout nœud non prévu est un refus.** C'est la propriété qui fait la
/// sûreté : une grammaire évolue, et un parcours qui ignorerait ce qu'il ne
/// connaît pas laisserait passer la construction du mois prochain.
fn visiter(
    n: Node,
    src: &[u8],
    liaison: Liaison,
    out: &mut Vec<Invocation>,
) -> Result<(), Refus> {
    match n.kind() {
        "program" | "list" | "pipeline" | "compound_statement" | "subshell" => {
            let mut suivante = liaison;
            let mut curseur = n.walk();
            for enfant in n.children(&mut curseur) {
                match enfant.kind() {
                    "&&" => {
                        suivante = Liaison::SiReussi;
                        continue;
                    }
                    "||" => {
                        suivante = Liaison::SiEchoue;
                        continue;
                    }
                    ";" | "\n" => {
                        suivante = Liaison::Puis;
                        continue;
                    }
                    "|" => {
                        suivante = Liaison::Tuyau;
                        continue;
                    }
                    "&" => return Err(Refus::ArrierePlan),
                    "comment" => continue,
                    _ => {}
                }
                let avant = out.len();
                visiter(enfant, src, suivante, out)?;
                // La liaison ne vaut que pour la première commande qu'elle
                // introduit ; les suivantes ont la leur.
                if out.len() > avant {
                    suivante = Liaison::Puis;
                }
            }
            Ok(())
        }
        "command" => {
            let inv = reduire_commande(n, src, liaison)?;
            out.push(inv);
            Ok(())
        }
        "comment" => Ok(()),
        // Tout le reste : refusé et nommé.
        "command_substitution" => Err(Refus::Substitution),
        "expansion" | "simple_expansion" => Err(Refus::Expansion),
        // `redirected_statement` enveloppe la commande **et** sa redirection :
        // c'est le nœud que la grammaire produit pour `echo x > f`. Il est
        // arrivé ici par le défaut — refusé, avec le mauvais nom — et c'est
        // exactement ce qu'on veut d'un défaut. Il ne lui manquait que d'être
        // reconnu (30 août 2026).
        "redirected_statement" | "file_redirect" | "heredoc_redirect" | "herestring_redirect" => {
            Err(Refus::Redirection)
        }
        autre => Err(Refus::Inconnu(autre.to_string())),
    }
}

/// Une commande simple : son programme et ses arguments littéraux.
fn reduire_commande(n: Node, src: &[u8], liaison: Liaison) -> Result<Invocation, Refus> {
    let mut programme: Option<String> = None;
    let mut args = Vec::new();
    let mut curseur = n.walk();
    for enfant in n.children(&mut curseur) {
        match enfant.kind() {
            "command_name" => {
                programme = Some(mot(enfant, src)?);
            }
            "word" | "string" | "raw_string" | "number" | "concatenation" => {
                args.push(mot(enfant, src)?);
            }
            "file_redirect" | "heredoc_redirect" | "herestring_redirect" => {
                return Err(Refus::Redirection)
            }
            "command_substitution" => return Err(Refus::Substitution),
            "expansion" | "simple_expansion" => return Err(Refus::Expansion),
            // `VAR=x commande` : l'environnement change ce que fait le
            // programme, et on ne saurait pas le dire à la politique.
            "variable_assignment" => return Err(Refus::Inconnu("variable_assignment".into())),
            autre => return Err(Refus::Inconnu(autre.to_string())),
        }
    }
    Ok(Invocation {
        programme: programme.ok_or(Refus::Vide)?,
        args,
        tuyau_entrant: liaison == Liaison::Tuyau,
        liaison,
    })
}

/// Le texte d'un nœud, guillemets retirés, **et rien d'autre**.
///
/// Si le nœud contient une substitution ou une expansion, on refuse : sa
/// valeur n'est pas dans le texte.
fn mot(n: Node, src: &[u8]) -> Result<String, Refus> {
    let mut curseur = n.walk();
    for enfant in n.children(&mut curseur) {
        match enfant.kind() {
            "command_substitution" => return Err(Refus::Substitution),
            "expansion" | "simple_expansion" => return Err(Refus::Expansion),
            _ => {}
        }
    }
    let brut = n.utf8_text(src).map_err(|_| Refus::Syntaxe)?;
    // Un joker ne sera pas développé par nous — on l'exécute par argv, sans
    // shell — donc le programme le recevrait littéralement. Le refuser dit à
    // l'appelant de donner les chemins, au lieu de le laisser croire qu'il a
    // filtré quelque chose.
    if brut.contains('*') || brut.contains('?') {
        return Err(Refus::Joker);
    }
    Ok(sans_guillemets(brut))
}

fn sans_guillemets(s: &str) -> String {
    let s = s.trim();
    for q in ['"', '\''] {
        if s.len() >= 2 && s.starts_with(q) && s.ends_with(q) {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}
