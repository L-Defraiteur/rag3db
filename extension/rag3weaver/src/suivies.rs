//! **Les suivies** (`tracked`) : ce qu'un dépôt indexe en couches, déclaré par un seul objet générique
//! (Lucie, 14 novembre 2026 : « tout paramétrable, une abstraction où même des cas pas prévus marchent »).
//!
//! Une **source** est un sélecteur git (une branche, un motif de refs, un worktree, un commit, les tâches
//! de l'atelier) plus trois réglages : sa parente, son histoire, sa couche de travail. La **racine** est un
//! réglage : calculée (le merge-base des suivies), vide ou déclarée. Rien n'est deviné : un sélecteur qui
//! ne résout rien se dit. Page de conception : `docs/orchestration/14-11-2026/02-les-sources-suivies-une-abstraction-pour-les-couches.md`.
//!
//! Ce module déclare, résout et dit ; il n'indexe rien de neuf (les générations sont 2a′-3).

use serde_json::Value;

use crate::couches::BranchesSuivies;

/// La racine des couches d'un dépôt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Racine {
    /// Le merge-base de toutes les pointes suivies ; l'état nomme la suivie qui la fixe.
    Calculee,
    /// Aucune racine : chaque couche se décrit par l'ensemble de ses versions.
    Vide,
    /// Un rev nommé (une branche, un commit).
    Declaree(String),
}

/// Ce qu'une source sélectionne : ce que git sait nommer, sous un préfixe explicite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selecteur {
    /// `branch:<nom>`.
    Branche(String),
    /// `refs:<motif for-each-ref>`.
    Refs(String),
    /// `worktree:backend` (celui que le backend indexe), `worktree:*` ou `worktree:<chemin>`.
    Worktree(Worktree),
    /// `commit:<rev>`.
    Commit(String),
    /// `tasks` : les tâches que l'atelier ouvre (leurs branches `agent/<nom>`).
    Taches,
}

/// Quel worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Worktree {
    DuBackend,
    Tous,
    Chemin(String),
}

/// La parente d'une source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parente {
    /// Le parent commun le plus récent parmi les suivies, admis seulement s'il est unique.
    Inferee,
    Aucune,
    Declaree(String),
}

/// L'histoire qu'une source garde (servie par 2a′-3 et la rétention ; ici déclarée et résolue).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Histoire {
    Pointe,
    Tout,
    Derniers(u32),
    Citee,
}

/// Une source suivie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub select: Selecteur,
    pub parent: Parente,
    pub history: Histoire,
    /// La couche de travail (le non-commité d'un worktree).
    pub work: bool,
}

/// La déclaration des suivies d'un dépôt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub root: Racine,
    pub sources: Vec<Source>,
}

/// **La déclaration du manifeste**, ou le défaut quand `tracked` est absent : le worktree que le backend
/// indexe (avec sa couche de travail) et les tâches de l'atelier, la racine calculée.
pub fn declaration_du_manifeste(tracked: Option<&Value>) -> Result<Declaration, String> {
    let Some(tracked) = tracked else {
        return Ok(Declaration {
            root: Racine::Calculee,
            sources: vec![
                Source { select: Selecteur::Worktree(Worktree::DuBackend), parent: Parente::Inferee, history: Histoire::Pointe, work: true },
                Source { select: Selecteur::Taches, parent: Parente::Inferee, history: Histoire::Pointe, work: true },
            ],
        });
    };
    let objet = tracked.as_object().ok_or("tracked : un objet {root, sources} attendu")?;
    for cle in objet.keys() {
        if cle != "root" && cle != "sources" {
            return Err(format!("tracked : clé inconnue « {cle} » (root, sources)"));
        }
    }
    let root = match objet.get("root") {
        None => Racine::Calculee,
        Some(Value::String(s)) if s == "computed" => Racine::Calculee,
        Some(Value::String(s)) if s == "empty" => Racine::Vide,
        Some(Value::Object(o)) if o.len() == 1 && o.get("declared").and_then(Value::as_str).is_some_and(|r| !r.is_empty()) => {
            Racine::Declaree(o["declared"].as_str().unwrap_or_default().to_string())
        }
        Some(autre) => return Err(format!("tracked.root : « computed », « empty » ou {{\"declared\": \"<rev>\"}} attendu, pas {autre}")),
    };
    let sources = match objet.get("sources") {
        None => Vec::new(),
        Some(Value::Array(a)) => a.iter().enumerate().map(|(i, s)| lire_une_source(i, s)).collect::<Result<_, _>>()?,
        Some(autre) => return Err(format!("tracked.sources : une liste attendue, pas {autre}")),
    };
    Ok(Declaration { root, sources })
}

fn lire_une_source(i: usize, v: &Value) -> Result<Source, String> {
    let ou = |quoi: &str| format!("tracked.sources[{i}].{quoi}");
    let objet = v.as_object().ok_or_else(|| format!("tracked.sources[{i}] : un objet {{select, parent?, history?, work?}} attendu"))?;
    for cle in objet.keys() {
        if !["select", "parent", "history", "work"].contains(&cle.as_str()) {
            return Err(format!("{} : clé inconnue (select, parent, history, work)", ou(cle)));
        }
    }
    let texte = objet.get("select").and_then(Value::as_str).ok_or_else(|| format!("{} : un sélecteur préfixé attendu", ou("select")))?;
    let non_vide = |reste: &str| -> Result<String, String> {
        if reste.trim().is_empty() {
            Err(format!("{} : « {texte} » ne nomme rien", ou("select")))
        } else {
            Ok(reste.to_string())
        }
    };
    let select = if texte == "tasks" {
        Selecteur::Taches
    } else if let Some(r) = texte.strip_prefix("branch:") {
        Selecteur::Branche(non_vide(r)?)
    } else if let Some(r) = texte.strip_prefix("refs:") {
        Selecteur::Refs(non_vide(r)?)
    } else if let Some(r) = texte.strip_prefix("commit:") {
        Selecteur::Commit(non_vide(r)?)
    } else if let Some(r) = texte.strip_prefix("worktree:") {
        match r {
            "backend" => Selecteur::Worktree(Worktree::DuBackend),
            "*" => Selecteur::Worktree(Worktree::Tous),
            chemin => Selecteur::Worktree(Worktree::Chemin(non_vide(chemin)?)),
        }
    } else {
        return Err(format!("{} : « {texte} » n'a pas de préfixe connu (branch:, refs:, worktree:, commit:, tasks)", ou("select")));
    };
    let parent = match objet.get("parent") {
        None => Parente::Inferee,
        Some(Value::String(s)) if s == "inferred" => Parente::Inferee,
        Some(Value::String(s)) if s == "none" => Parente::Aucune,
        Some(Value::Object(o)) if o.len() == 1 && o.get("declared").and_then(Value::as_str).is_some_and(|r| !r.is_empty()) => {
            Parente::Declaree(o["declared"].as_str().unwrap_or_default().to_string())
        }
        Some(autre) => return Err(format!("{} : « inferred », « none » ou {{\"declared\": \"<branche>\"}} attendu, pas {autre}", ou("parent"))),
    };
    let history = match objet.get("history") {
        None => Histoire::Pointe,
        Some(Value::String(s)) if s == "tip" => Histoire::Pointe,
        Some(Value::String(s)) if s == "all" => Histoire::Tout,
        Some(Value::String(s)) if s == "cited" => Histoire::Citee,
        Some(Value::Object(o)) if o.len() == 1 && o.contains_key("last") => match o["last"].as_u64() {
            Some(n) if n > 0 && n <= u32::MAX as u64 => Histoire::Derniers(n as u32),
            _ => return Err(format!("{} : {{\"last\": N}} attend un entier positif, pas {}", ou("history"), o["last"])),
        },
        Some(autre) => return Err(format!("{} : « tip », « all », « cited » ou {{\"last\": N}} attendu, pas {autre}", ou("history"))),
    };
    let a_un_worktree = matches!(select, Selecteur::Worktree(_) | Selecteur::Taches);
    let work = match objet.get("work") {
        None => a_un_worktree,
        Some(Value::Bool(true)) if !a_un_worktree => {
            return Err(format!("{} : « {texte} » n'a pas de worktree, donc pas de couche de travail", ou("work")))
        }
        Some(Value::Bool(b)) => *b,
        Some(autre) => return Err(format!("{} : un booléen attendu, pas {autre}", ou("work"))),
    };
    Ok(Source { select, parent, history, work })
}

/// **L'ancienne clé** `branches_suivies:<dépôt>`, lue et plus écrite : son ancre devient une racine
/// déclarée, chaque nom une source `branch:`.
pub fn declaration_de_l_ancienne_cle(ancienne: &BranchesSuivies) -> Declaration {
    Declaration {
        root: Racine::Declaree(ancienne.ancre.clone()),
        sources: ancienne
            .suivies
            .iter()
            .map(|b| Source { select: Selecteur::Branche(b.clone()), parent: Parente::Inferee, history: Histoire::Pointe, work: false })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn source(select: Selecteur, parent: Parente, history: Histoire, work: bool) -> Source {
        Source { select, parent, history, work }
    }

    /// **Sans `tracked`, le défaut est la pratique d'aujourd'hui** : le worktree du backend avec sa couche
    /// de travail, les tâches de l'atelier, la racine calculée. `main` n'a aucune place particulière.
    #[test]
    fn sans_tracked_le_defaut_est_le_worktree_du_backend_et_les_taches() {
        assert_eq!(
            declaration_du_manifeste(None).unwrap(),
            Declaration {
                root: Racine::Calculee,
                sources: vec![
                    source(Selecteur::Worktree(Worktree::DuBackend), Parente::Inferee, Histoire::Pointe, true),
                    source(Selecteur::Taches, Parente::Inferee, Histoire::Pointe, true),
                ],
            }
        );
    }

    /// **Le manifeste se lit tel que déclaré**, chaque réglage à son défaut quand il est omis (`work` vrai
    /// pour un worktree et les tâches, faux sinon) ; une couche de travail sur une source sans worktree, un
    /// préfixe inconnu ou une racine illisible sont des refus nommés.
    #[test]
    fn le_manifeste_se_lit_et_ses_refus_se_nomment() {
        let lu = declaration_du_manifeste(Some(&json!({
            "root": {"declared": "main"},
            "sources": [
                {"select": "worktree:backend"},
                {"select": "tasks"},
                {"select": "branch:develop", "parent": {"declared": "main"}},
                {"select": "refs:refs/heads/release/*", "history": {"last": 5}},
                {"select": "commit:4f6f80ef4", "parent": "none", "history": "all"},
                {"select": "worktree:/chemin/du/worktree", "history": "cited", "work": false}
            ]
        })))
        .unwrap();
        assert_eq!(lu.root, Racine::Declaree("main".into()));
        assert_eq!(
            lu.sources,
            vec![
                source(Selecteur::Worktree(Worktree::DuBackend), Parente::Inferee, Histoire::Pointe, true),
                source(Selecteur::Taches, Parente::Inferee, Histoire::Pointe, true),
                source(Selecteur::Branche("develop".into()), Parente::Declaree("main".into()), Histoire::Pointe, false),
                source(Selecteur::Refs("refs/heads/release/*".into()), Parente::Inferee, Histoire::Derniers(5), false),
                source(Selecteur::Commit("4f6f80ef4".into()), Parente::Aucune, Histoire::Tout, false),
                source(Selecteur::Worktree(Worktree::Chemin("/chemin/du/worktree".into())), Parente::Inferee, Histoire::Citee, false),
            ]
        );
        assert_eq!(declaration_du_manifeste(Some(&json!({"root": "empty", "sources": []}))).unwrap().root, Racine::Vide);
        assert_eq!(declaration_du_manifeste(Some(&json!({"sources": [{"select": "worktree:*"}]}))).unwrap().root, Racine::Calculee);
        for (mauvais, mot) in [
            (json!({"sources": [{"select": "branch:a", "work": true}]}), "work"),
            (json!({"sources": [{"select": "branche:a"}]}), "branche:a"),
            (json!({"sources": [{"select": "branch:"}]}), "branch:"),
            (json!({"root": "quelque-part"}), "root"),
            (json!({"sources": [{"select": "tasks", "history": {"last": 0}}]}), "last"),
        ] {
            let refus = declaration_du_manifeste(Some(&mauvais)).expect_err(&format!("{mauvais} est accepté"));
            assert!(refus.contains(mot), "le refus de {mauvais} ne nomme pas « {mot} » : {refus}");
        }
    }

    /// **L'ancienne clé `branches_suivies:` se lit encore** : son ancre devient une racine déclarée, chaque
    /// nom une branche suivie, à la parente inférée.
    #[test]
    fn l_ancienne_cle_des_branches_suivies_se_lit_encore() {
        let ancienne = BranchesSuivies { ancre: "main".into(), suivies: vec!["a".into(), "agent/doc".into()] };
        assert_eq!(
            declaration_de_l_ancienne_cle(&ancienne),
            Declaration {
                root: Racine::Declaree("main".into()),
                sources: vec![
                    source(Selecteur::Branche("a".into()), Parente::Inferee, Histoire::Pointe, false),
                    source(Selecteur::Branche("agent/doc".into()), Parente::Inferee, Histoire::Pointe, false),
                ],
            }
        );
    }
}
