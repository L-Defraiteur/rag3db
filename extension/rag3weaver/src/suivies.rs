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

/// Une ref résolue : une couche d'histoire, nommée par son nom **complet** (`refs/heads/<x>`, `refs/tags/<x>`),
/// ou par le hash pour un `commit:`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefResolue {
    pub nom: String,
    pub pointe: String,
}

/// Une couche de travail : le non-commité d'**un worktree** (nommée par son chemin), sur la branche qu'il a
/// extraite, ou détachée sur un commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoucheDeTravail {
    pub chemin: std::path::PathBuf,
    pub branche: Option<String>,
    pub commit: String,
}

/// Une source, résolue : ses refs et ses couches de travail, ou ce qui l'a laissée vide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceResolue {
    pub source: Source,
    pub refs: Vec<RefResolue>,
    pub travail: Vec<CoucheDeTravail>,
    /// Pourquoi la source ne résout rien (« branch:x : aucune branche de ce nom »), dit dans l'état.
    pub vide: Option<String>,
}

/// **Résoudre les sources** d'une déclaration dans le dépôt de `racine` : un `git for-each-ref` et un
/// `git worktree list --porcelain` pour toutes les sources, plus un `git rev-parse` par sélecteur
/// `commit:`. `worktree_du_backend` est le worktree que le backend indexe ; `taches`, les branches des tâches
/// que l'atelier a ouvertes (`agent/<nom>`). Une source qui ne résout rien se dit dans `vide`, elle ne fait
/// pas échouer la résolution.
pub fn resoudre_les_sources(
    racine: &std::path::Path,
    worktree_du_backend: &std::path::Path,
    taches: &[String],
    declaration: &Declaration,
) -> Result<Vec<SourceResolue>, String> {
    let refs = lire_les_refs(racine)?;
    let worktrees = lire_les_worktrees(racine)?;
    let du_backend = worktree_du_backend.canonicalize().ok();
    Ok(declaration
        .sources
        .iter()
        .map(|source| {
            let (refs_de_la_source, wts, vide): (Vec<RefResolue>, Vec<&CoucheDeTravail>, Option<String>) = match &source.select {
                Selecteur::Branche(nom) => {
                    let complet = format!("refs/heads/{nom}");
                    let trouvees: Vec<RefResolue> = refs.iter().filter(|r| r.nom == complet).cloned().collect();
                    let vide = trouvees.is_empty().then(|| format!("branch:{nom} : aucune branche de ce nom"));
                    (trouvees, Vec::new(), vide)
                }
                Selecteur::Refs(motif) => {
                    let trouvees: Vec<RefResolue> = refs.iter().filter(|r| ref_correspond(motif, &r.nom)).cloned().collect();
                    let vide = trouvees.is_empty().then(|| format!("refs:{motif} : aucune ref ne correspond"));
                    (trouvees, Vec::new(), vide)
                }
                Selecteur::Commit(rev) => match crate::couches::git(racine, &["rev-parse", "--verify", "--quiet", &format!("{rev}^{{commit}}")]) {
                    Ok(sha) if !sha.is_empty() => (vec![RefResolue { nom: sha.clone(), pointe: sha }], Vec::new(), None),
                    _ => (Vec::new(), Vec::new(), Some(format!("commit:{rev} : aucun commit de ce nom"))),
                },
                Selecteur::Worktree(quel) => {
                    let choisis: Vec<&CoucheDeTravail> = match quel {
                        Worktree::Tous => worktrees.iter().collect(),
                        Worktree::DuBackend => du_backend
                            .as_ref()
                            .and_then(|b| worktrees.iter().filter(|w| b.starts_with(&w.chemin)).max_by_key(|w| w.chemin.as_os_str().len()))
                            .into_iter()
                            .collect(),
                        Worktree::Chemin(c) => {
                            let voulu = std::path::Path::new(c).canonicalize().ok();
                            worktrees.iter().filter(|w| voulu.as_ref() == Some(&w.chemin)).collect()
                        }
                    };
                    let vide = choisis.is_empty().then(|| match quel {
                        Worktree::DuBackend => format!("worktree:backend : {} n'est dans aucun worktree du dépôt", worktree_du_backend.display()),
                        Worktree::Tous => "worktree:* : le dépôt n'a aucun worktree".to_string(),
                        Worktree::Chemin(c) => format!("worktree:{c} : aucun worktree du dépôt à ce chemin"),
                    });
                    (refs_des_worktrees(&refs, &choisis), choisis, vide)
                }
                Selecteur::Taches => {
                    let noms: Vec<String> = taches.iter().map(|t| format!("refs/heads/{t}")).collect();
                    let trouvees: Vec<RefResolue> = refs.iter().filter(|r| noms.contains(&r.nom)).cloned().collect();
                    let wts: Vec<&CoucheDeTravail> = worktrees.iter().filter(|w| w.branche.as_ref().is_some_and(|b| noms.contains(b))).collect();
                    let vide = trouvees.is_empty().then(|| "tasks : aucune tâche ouverte par l'atelier".to_string());
                    (trouvees, wts, vide)
                }
            };
            SourceResolue {
                source: source.clone(),
                refs: refs_de_la_source,
                travail: if source.work { wts.into_iter().cloned().collect() } else { Vec::new() },
                vide,
            }
        })
        .collect())
}

/// Les refs du dépôt, chacune sur son commit (une étiquette annotée, déréférencée).
fn lire_les_refs(racine: &std::path::Path) -> Result<Vec<RefResolue>, String> {
    let sortie = crate::couches::git(racine, &["for-each-ref", "--format=%(refname)%00%(objectname)%00%(*objectname)"])?;
    Ok(sortie
        .lines()
        .filter_map(|l| {
            let mut c = l.split('\0');
            let (nom, objet, deref) = (c.next()?, c.next()?, c.next().unwrap_or(""));
            Some(RefResolue { nom: nom.to_string(), pointe: if deref.is_empty() { objet } else { deref }.to_string() })
        })
        .collect())
}

/// Les worktrees du dépôt, dans l'ordre de git : chemin résolu, branche extraite (nom complet) ou détaché.
fn lire_les_worktrees(racine: &std::path::Path) -> Result<Vec<CoucheDeTravail>, String> {
    let sortie = crate::couches::git(racine, &["worktree", "list", "--porcelain"])?;
    let mut out = Vec::new();
    for bloc in sortie.split("\n\n") {
        let (mut chemin, mut commit, mut branche) = (None, None, None);
        for l in bloc.lines() {
            if let Some(p) = l.strip_prefix("worktree ") {
                chemin = Some(std::path::PathBuf::from(p));
            } else if let Some(h) = l.strip_prefix("HEAD ") {
                commit = Some(h.to_string());
            } else if let Some(b) = l.strip_prefix("branch ") {
                branche = Some(b.to_string());
            }
        }
        if let (Some(chemin), Some(commit)) = (chemin, commit) {
            out.push(CoucheDeTravail { chemin: chemin.canonicalize().unwrap_or(chemin), branche, commit });
        }
    }
    Ok(out)
}

/// Les refs des branches que des worktrees ont extraites, sans doublon (un worktree détaché n'en apporte pas).
fn refs_des_worktrees(refs: &[RefResolue], worktrees: &[&CoucheDeTravail]) -> Vec<RefResolue> {
    let mut out: Vec<RefResolue> = Vec::new();
    for w in worktrees {
        if let Some(b) = &w.branche {
            if let Some(r) = refs.iter().find(|r| &r.nom == b) {
                if !out.contains(r) {
                    out.push(r.clone());
                }
            }
        }
    }
    out
}

/// **Une ref correspond-elle au motif**, comme pour `git for-each-ref <motif>` : avec un joker (`*`, `?`),
/// une correspondance de chemin où le joker ne franchit pas `/` ; sans joker, la ref elle-même ou ce qui est
/// sous elle (`refs/heads/release` prend `refs/heads/release/1`).
fn ref_correspond(motif: &str, nom: &str) -> bool {
    if motif.contains(['*', '?']) {
        joker(motif.as_bytes(), nom.as_bytes())
    } else {
        let m = motif.trim_end_matches('/');
        nom == m || nom.strip_prefix(m).is_some_and(|reste| reste.starts_with('/'))
    }
}

fn joker(m: &[u8], n: &[u8]) -> bool {
    match (m.first(), n.first()) {
        (None, None) => true,
        (Some(b'*'), _) => joker(&m[1..], n) || (!n.is_empty() && n[0] != b'/' && joker(m, &n[1..])),
        (Some(b'?'), Some(&c)) => c != b'/' && joker(&m[1..], &n[1..]),
        (Some(&a), Some(&b)) => a == b && joker(&m[1..], &n[1..]),
        _ => false,
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

    fn git(dir: &std::path::Path, args: &[&str]) -> String {
        let o = std::process::Command::new("git")
            .arg("-C").arg(dir)
            .args(["-c", "user.name=Témoin", "-c", "user.email=temoin@example.invalid", "-c", "commit.gpgsign=false", "-c", "init.defaultBranch=main"])
            .args(args)
            .output()
            .unwrap();
        assert!(o.status.success(), "git {args:?} : {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    }

    /// Le décor : main (m1, m2) ; les branches `a`, `release/1`, `release/2`, `release/x/y`, `agent/t1` ; une
    /// étiquette `v1` ; un worktree `wt-a` sur `a`, un worktree détaché `wt-d` sur m1, et celui de la tâche
    /// `wt-t1` sur `agent/t1`.
    fn depot() -> (tempfile::TempDir, std::path::PathBuf) {
        let t = tempfile::tempdir().unwrap();
        let d = t.path().join("depot");
        std::fs::create_dir_all(&d).unwrap();
        git(&d, &["init", "-q"]);
        git(&d, &["commit", "-q", "--allow-empty", "-m", "m1"]);
        git(&d, &["commit", "-q", "--allow-empty", "-m", "m2"]);
        for b in ["a", "release/1", "release/2", "release/x/y", "agent/t1"] {
            git(&d, &["branch", b]);
        }
        git(&d, &["tag", "v1", "HEAD~1"]);
        git(&d, &["worktree", "add", "-q", t.path().join("wt-a").to_str().unwrap(), "a"]);
        git(&d, &["worktree", "add", "-q", "--detach", t.path().join("wt-d").to_str().unwrap(), "HEAD~1"]);
        git(&d, &["worktree", "add", "-q", t.path().join("wt-t1").to_str().unwrap(), "agent/t1"]);
        (t, d)
    }

    fn une(select: Selecteur, work: bool) -> Declaration {
        Declaration { root: Racine::Calculee, sources: vec![source(select, Parente::Inferee, Histoire::Pointe, work)] }
    }

    fn reel(p: &std::path::Path) -> std::path::PathBuf {
        p.canonicalize().unwrap()
    }

    /// **Chaque sélecteur résout ce que git nomme**, par son nom complet, et un sélecteur qui ne résout
    /// rien se dit sans faire échouer la résolution.
    #[test]
    fn chaque_selecteur_resout_ce_que_git_nomme() {
        let (t, d) = depot();
        let resoudre = |decl: &Declaration| resoudre_les_sources(&d, &d, &["agent/t1".to_string()], decl).unwrap().remove(0);
        let pointe = |r: &str| git(&d, &["rev-parse", r]);

        let a = resoudre(&une(Selecteur::Branche("a".into()), false));
        assert_eq!(a.refs, vec![RefResolue { nom: "refs/heads/a".into(), pointe: pointe("a") }]);
        assert_eq!((a.travail.len(), &a.vide), (0, &None));

        let absente = resoudre(&une(Selecteur::Branche("absente".into()), false));
        assert!(absente.refs.is_empty());
        assert!(absente.vide.as_deref().is_some_and(|v| v.contains("absente")), "{absente:?}");

        // Le motif : exactement ce que `git for-each-ref <motif>` rend.
        for motif in ["refs/heads/release/*", "refs/heads/release", "refs/tags/*", "refs/heads/*"] {
            let attendu: Vec<String> = git(&d, &["for-each-ref", "--format=%(refname)", motif]).lines().map(str::to_string).collect();
            let r = resoudre(&une(Selecteur::Refs(motif.into()), false));
            let noms: Vec<String> = r.refs.iter().map(|x| x.nom.clone()).collect();
            assert_eq!(noms, attendu, "le motif {motif}");
        }
        let etiquette = resoudre(&une(Selecteur::Refs("refs/tags/v1".into()), false));
        assert_eq!(etiquette.refs, vec![RefResolue { nom: "refs/tags/v1".into(), pointe: pointe("v1^{commit}") }], "une étiquette pointe sur son commit");

        let commit = resoudre(&une(Selecteur::Commit("HEAD~1".into()), false));
        assert_eq!(commit.refs, vec![RefResolue { nom: pointe("HEAD~1"), pointe: pointe("HEAD~1") }]);

        // Les worktrees : une couche de travail chacun, nommée par son chemin, sur sa branche ou détachée.
        let wa = resoudre_les_sources(&d, &t.path().join("wt-a"), &[], &une(Selecteur::Worktree(Worktree::DuBackend), true)).unwrap().remove(0);
        assert_eq!(wa.refs, vec![RefResolue { nom: "refs/heads/a".into(), pointe: pointe("a") }]);
        assert_eq!(wa.travail, vec![CoucheDeTravail { chemin: reel(&t.path().join("wt-a")), branche: Some("refs/heads/a".into()), commit: pointe("a") }]);
        let tous = resoudre(&une(Selecteur::Worktree(Worktree::Tous), true));
        let chemins: Vec<std::path::PathBuf> = tous.travail.iter().map(|c| c.chemin.clone()).collect();
        assert_eq!(chemins, vec![reel(&d), reel(&t.path().join("wt-a")), reel(&t.path().join("wt-d")), reel(&t.path().join("wt-t1"))]);
        let detache = tous.travail.iter().find(|c| c.chemin == reel(&t.path().join("wt-d"))).unwrap();
        assert_eq!((detache.branche.as_deref(), detache.commit.as_str()), (None, pointe("HEAD~1").as_str()), "le worktree détaché");
        assert!(!tous.refs.iter().any(|r| r.nom.is_empty()), "un worktree détaché n'apporte pas de ref : {:?}", tous.refs);
        let sans_travail = resoudre(&une(Selecteur::Worktree(Worktree::Chemin(t.path().join("wt-a").to_string_lossy().into_owned())), false));
        assert_eq!((sans_travail.refs.len(), sans_travail.travail.len()), (1, 0), "work: false ne garde que l'histoire");
        let ailleurs = resoudre(&une(Selecteur::Worktree(Worktree::Chemin("/nulle/part".into())), true));
        assert!(ailleurs.vide.as_deref().is_some_and(|v| v.contains("/nulle/part")), "{ailleurs:?}");

        // Les tâches : leur branche et la couche de travail de leur worktree.
        let taches = resoudre(&une(Selecteur::Taches, true));
        assert_eq!(taches.refs, vec![RefResolue { nom: "refs/heads/agent/t1".into(), pointe: pointe("agent/t1") }]);
        assert_eq!(taches.travail.iter().map(|c| c.chemin.clone()).collect::<Vec<_>>(), vec![reel(&t.path().join("wt-t1"))]);
    }
}
