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

/// La racine des couches, résolue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RacineResolue {
    /// Le merge-base de toutes les pointes, et la ou les suivies qui le fixent (sans elles, il avancerait).
    Calculee { commit: String, fixee_par: Vec<String> },
    /// Les suivies n'ont aucun ancêtre commun (des historiques orphelins) : chacune est sa propre racine.
    SansAncetreCommun,
    Vide,
    Declaree { rev: String, commit: String },
    /// Une racine déclarée que git ne nomme pas : dite, jamais devinée.
    Introuvable(String),
}

/// La parente d'une couche, résolue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParenteResolue {
    Aucune,
    Declaree(String),
    /// Le parent commun le plus récent parmi les suivies, et la base (le point de fourche).
    Inferee { parente: String, base: String },
    /// Plusieurs suivies aux pointes égales : rien à décider, dit tel quel (sans avertissement).
    Egales(Vec<String>),
    /// Plusieurs suivies, aux fourches différentes et sans ordre entre elles : à déclarer.
    Ambigue(Vec<String>),
    /// La couche est dans un cycle de fusions (un flux multi-env) : sa parente est à déclarer.
    DansUnFlux,
}

/// Une couche d'histoire : une ref résolue, sa pointe, sa parente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoucheResolue {
    pub nom: String,
    pub pointe: String,
    pub parente: ParenteResolue,
}

/// La pile d'un dépôt : sa racine, ses couches, ses flux, ce qui est à dire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pile {
    pub racine: RacineResolue,
    pub couches: Vec<CoucheResolue>,
    /// Les cycles de fusions entre suivies (« flux détecté entre develop, staging et main »), chacun trié.
    pub flux: Vec<Vec<String>>,
    pub avertissements: Vec<String>,
}

/// **La pile** des sources résolues : la racine dans sa nature, la parente de chaque couche (déclarée,
/// aucune, ou inférée parmi les suivies : le parent commun le plus récent, admis seulement s'il est unique),
/// et les flux. Sans ancre : `main` n'a aucune place particulière.
pub fn resoudre_la_pile(racine: &std::path::Path, declaration: &Declaration, sources: &[SourceResolue]) -> Result<Pile, String> {
    use crate::couches::git;
    // Les couches d'histoire, une par ref résolue (la première source qui la nomme fixe sa parente voulue).
    let mut couches: Vec<(RefResolue, Parente)> = Vec::new();
    for s in sources {
        for r in &s.refs {
            if !couches.iter().any(|(c, _)| c.nom == r.nom) {
                couches.push((r.clone(), s.source.parent.clone()));
            }
        }
    }
    let pointes: Vec<String> = couches.iter().map(|(r, _)| r.pointe.clone()).collect();
    let octopus = |p: &[String]| -> Option<String> {
        if p.is_empty() {
            return None;
        }
        let mut args = vec!["merge-base", "--octopus"];
        args.extend(p.iter().map(String::as_str));
        git(racine, &args).ok().filter(|s| !s.is_empty())
    };
    let racine_resolue = match &declaration.root {
        Racine::Vide => RacineResolue::Vide,
        Racine::Declaree(rev) => match git(racine, &["rev-parse", "--verify", "--quiet", &format!("{rev}^{{commit}}")]) {
            Ok(c) if !c.is_empty() => RacineResolue::Declaree { rev: rev.clone(), commit: c },
            _ => RacineResolue::Introuvable(rev.clone()),
        },
        Racine::Calculee => match octopus(&pointes) {
            None => RacineResolue::SansAncetreCommun,
            Some(commit) => {
                let fixee_par = couches
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| {
                        let autres: Vec<String> = pointes.iter().enumerate().filter(|(j, _)| j != i).map(|(_, p)| p.clone()).collect();
                        octopus(&autres).as_deref() != Some(commit.as_str())
                    })
                    .map(|(_, (r, _))| r.nom.clone())
                    .collect();
                RacineResolue::Calculee { commit, fixee_par }
            }
        },
    };
    let est_ancetre = |a: &str, b: &str| -> bool {
        std::process::Command::new("git").arg("-C").arg(racine).args(["merge-base", "--is-ancestor", a, b]).status().is_ok_and(|s| s.success())
    };
    let merge_base = |a: &str, b: &str| -> Option<String> { git(racine, &["merge-base", a, b]).ok().filter(|s| !s.is_empty()) };

    // Les flux : X → Y quand une fusion faite par Y (sur sa ligne de premier parent) apporte un commit fait
    // par X (sur la sienne). Un cycle de ces arêtes est un flux multi-env.
    let ligne = |r: &str, merges: bool| -> Vec<String> {
        let mut args = vec!["rev-list", "--first-parent"];
        if merges {
            args.extend(["--merges", "--parents"]);
        }
        args.push(r);
        git(racine, &args).map(|s| s.lines().map(str::to_string).collect()).unwrap_or_default()
    };
    let lignes: Vec<std::collections::HashSet<String>> = couches.iter().map(|(r, _)| ligne(&r.pointe, false).into_iter().collect()).collect();
    let n = couches.len();
    let mut arete = vec![vec![false; n]; n];
    // La plus récente fusion de Y qui apporte la ligne de X (la ligne de premier parent est rendue de la
    // pointe vers le passé : la première trouvée).
    let mut fusion_de: Vec<Vec<Option<String>>> = vec![vec![None; n]; n];
    for (y, (ry, _)) in couches.iter().enumerate() {
        for fusion in ligne(&ry.pointe, true) {
            let mut mots = fusion.split_whitespace();
            let Some(m) = mots.next() else { continue };
            for apporte in mots.skip(1) {
                for x in (0..n).filter(|&x| x != y) {
                    if lignes[x].contains(apporte) {
                        arete[x][y] = true;
                        fusion_de[x][y].get_or_insert_with(|| m.to_string());
                    }
                }
            }
        }
    }
    // Une couche **contenue** dans une autre (sa pointe est un ancêtre strict de l'autre pointe) ne compte
    // dans aucun flux (décision de c7, 14 nov.) ; **finie** dans une autre quand celle-ci l'a fusionnée.
    let contenue_dans = |i: usize, j: usize| -> bool {
        i != j && couches[i].0.pointe != couches[j].0.pointe && est_ancetre(&couches[i].0.pointe, &couches[j].0.pointe)
    };
    let contenue: Vec<bool> = (0..n).map(|i| (0..n).any(|j| contenue_dans(i, j))).collect();
    let finie: Vec<Vec<bool>> = (0..n).map(|i| (0..n).map(|j| arete[i][j] && contenue_dans(i, j)).collect()).collect();
    let finie_dans = |i: usize, j: usize| -> bool { finie[i][j] };
    for i in 0..n {
        if contenue[i] {
            for j in 0..n {
                arete[i][j] = false;
                arete[j][i] = false;
            }
        }
    }
    // La fermeture transitive, puis les composantes à plus d'une couche.
    let mut atteint = arete.clone();
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                if atteint[i][k] && atteint[k][j] {
                    atteint[i][j] = true;
                }
            }
        }
    }
    let mut flux: Vec<Vec<String>> = Vec::new();
    let mut dans_un_flux = vec![false; n];
    for i in 0..n {
        if dans_un_flux[i] {
            continue;
        }
        let mut cycle: Vec<usize> = (0..n).filter(|&j| j == i || (atteint[i][j] && atteint[j][i])).collect();
        if cycle.len() > 1 {
            cycle.sort_by(|a, b| couches[*a].0.nom.cmp(&couches[*b].0.nom));
            for &j in &cycle {
                dans_un_flux[j] = true;
            }
            flux.push(cycle.iter().map(|&j| couches[j].0.nom.clone()).collect());
        }
    }
    flux.sort();

    let nom_complet = |p: &str| -> String {
        if p.starts_with("refs/") {
            return p.to_string();
        }
        let tete = format!("refs/heads/{p}");
        if git(racine, &["rev-parse", "--verify", "--quiet", &tete]).is_ok() { tete } else { p.to_string() }
    };
    let mut avertissements = Vec::new();
    let mut resolues = Vec::with_capacity(n);
    for (i, (r, voulue)) in couches.iter().enumerate() {
        let parente = match voulue {
            Parente::Aucune => ParenteResolue::Aucune,
            Parente::Declaree(p) => ParenteResolue::Declaree(nom_complet(p)),
            Parente::Inferee if dans_un_flux[i] => ParenteResolue::DansUnFlux,
            // Une couche finie : sa parente est la suivie qui l'a fusionnée, sa base la fourche avec ce que
            // celle-ci était avant la fusion.
            Parente::Inferee if (0..n).any(|j| finie_dans(i, j)) => {
                let dans: Vec<usize> = (0..n).filter(|&j| finie_dans(i, j)).collect();
                match dans.as_slice() {
                    [j] => {
                        let avant = fusion_de[i][*j].as_deref().map(|m| format!("{m}^1")).unwrap_or_else(|| couches[*j].0.pointe.clone());
                        let base = merge_base(&r.pointe, &avant).unwrap_or_else(|| r.pointe.clone());
                        ParenteResolue::Inferee { parente: couches[*j].0.nom.clone(), base }
                    }
                    plusieurs => {
                        let mut noms: Vec<String> = plusieurs.iter().map(|&j| couches[j].0.nom.clone()).collect();
                        noms.sort();
                        avertissements.push(format!("couche « {} » ambiguë : fusionnée dans {} — à déclarer", r.nom, noms.join(" et ")));
                        ParenteResolue::Ambigue(noms)
                    }
                }
            }
            Parente::Inferee => {
                // Les candidates : les autres suivies dont la fourche avec celle-ci n'est pas sa propre pointe
                // (une suivie qui la contient est une enfant, pas une parente).
                let candidates: Vec<(String, String)> = couches
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i && !finie_dans(*j, i))
                    .filter_map(|(_, (p, _))| merge_base(&r.pointe, &p.pointe).filter(|b| *b != r.pointe).map(|b| (p.nom.clone(), b)))
                    .collect();
                let plus_recentes: Vec<&(String, String)> = candidates
                    .iter()
                    .filter(|(_, b)| !candidates.iter().any(|(_, autre)| autre != b && est_ancetre(b, autre)))
                    .collect();
                match plus_recentes.as_slice() {
                    [] => ParenteResolue::Aucune,
                    [(p, b)] => ParenteResolue::Inferee { parente: p.clone(), base: b.clone() },
                    plusieurs => {
                        let mut noms: Vec<String> = plusieurs.iter().map(|(p, _)| p.clone()).collect();
                        noms.sort();
                        let pointe_de = |nom: &str| couches.iter().find(|(c, _)| c.nom == nom).map(|(c, _)| c.pointe.clone());
                        let premiere = pointe_de(&noms[0]);
                        if noms.iter().all(|n| pointe_de(n) == premiere) {
                            ParenteResolue::Egales(noms)
                        } else {
                            avertissements.push(format!("couche « {} » ambiguë : sa parente est {} — à déclarer", r.nom, noms.join(" ou ")));
                            ParenteResolue::Ambigue(noms)
                        }
                    }
                }
            }
        };
        resolues.push(CoucheResolue { nom: r.nom.clone(), pointe: r.pointe.clone(), parente });
    }
    for f in &flux {
        avertissements.push(format!("flux détecté entre {} : déclare leurs parentes", f.join(", ")));
    }
    if let RacineResolue::Introuvable(rev) = &racine_resolue {
        avertissements.push(format!("racine déclarée « {rev} » introuvable"));
    }
    Ok(Pile { racine: racine_resolue, couches: resolues, flux, avertissements })
}

/// L'identifiant d'un dépôt : la racine du plus vieux commit parmi les historiques de ses suivies (date
/// d'auteur ; à égalité, toutes nommées, et la première par hash fait l'identifiant). Stable d'un poste à
/// l'autre, sans ancre (décision de c7, 14 nov.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identifiant {
    pub id: String,
    /// Les racines à égalité de date avec `id`, `id` compris, quand il y en a plusieurs.
    pub egaux: Vec<String>,
}

/// **L'identifiant du dépôt** depuis les pointes suivies ; `None` sans pointe.
pub fn identifiant_des_suivies(racine: &std::path::Path, pointes: &[String]) -> Result<Option<Identifiant>, String> {
    use crate::couches::git;
    if pointes.is_empty() {
        return Ok(None);
    }
    let mut args = vec!["rev-list", "--max-parents=0"];
    args.extend(pointes.iter().map(String::as_str));
    let racines: Vec<String> = git(racine, &args)?.lines().map(str::to_string).filter(|l| !l.is_empty()).collect();
    if racines.is_empty() {
        return Ok(None);
    }
    let mut args = vec!["log", "--no-walk=unsorted", "--format=%H %at"];
    args.extend(racines.iter().map(String::as_str));
    let mut datees: Vec<(i64, String)> = git(racine, &args)?
        .lines()
        .filter_map(|l| {
            let (h, t) = l.split_once(' ')?;
            Some((t.trim().parse().ok()?, h.to_string()))
        })
        .collect();
    datees.sort();
    let Some((premiere, _)) = datees.first().cloned() else { return Ok(None) };
    let mut egaux: Vec<String> = datees.iter().filter(|(t, _)| *t == premiere).map(|(_, h)| h.clone()).collect();
    egaux.sort();
    egaux.dedup();
    let id = egaux[0].clone();
    Ok(Some(Identifiant { id, egaux: if egaux.len() > 1 { egaux } else { Vec::new() } }))
}

/// **Migrer les clés d'un dépôt** dans `_catalog_meta` : de l'identifiant `ancien` à `neuf` (ils peuvent être
/// égaux), et les noms courts de branche (`couche:<dépôt>:<branche>`, `couche_fermee:`) vers le nom complet
/// (`refs/heads/<branche>`). **Rien ne se perd** : une clé cible déjà posée n'est jamais écrasée, la source
/// reste alors et le journal le dit. Rend le journal de ce qui a bougé.
pub fn migrer_les_cles(catalog: &crate::catalog::Catalog, ancien: &str, neuf: &str) -> Result<Vec<String>, String> {
    let e = |e: crate::catalog::CatalogError| e.to_string();
    let mut journal = Vec::new();
    let mut deplacer = |source: &str, valeur: &str, cible: &str| -> Result<(), String> {
        if source == cible {
            return Ok(());
        }
        match catalog.read_meta_key(cible).map_err(e)?.filter(|v| !v.is_empty()) {
            Some(deja) if deja == valeur => {
                catalog.persist_meta_key(source, "").map_err(e)?;
                journal.push(format!("{source} : déjà en {cible}, retirée"));
            }
            Some(_) => journal.push(format!("{source} gardée : {cible} est déjà posée autrement (la garde, rien ne s'écrase)")),
            None => {
                catalog.persist_meta_key(cible, valeur).map_err(e)?;
                catalog.persist_meta_key(source, "").map_err(e)?;
                journal.push(format!("{source} → {cible}"));
            }
        }
        Ok(())
    };
    for genre in ["couche", "couche_fermee"] {
        let prefixe = format!("{genre}:{ancien}:");
        for (cle, valeur) in catalog.meta_par_prefixe(&prefixe).map_err(e)? {
            let reste = &cle[prefixe.len()..];
            let complet = if reste.starts_with("refs/") { reste.to_string() } else { format!("refs/heads/{reste}") };
            deplacer(&cle, &valeur, &format!("{genre}:{neuf}:{complet}"))?;
        }
    }
    let ancienne = format!("branches_suivies:{ancien}");
    if let Some(valeur) = catalog.read_meta_key(&ancienne).map_err(e)?.filter(|v| !v.is_empty()) {
        deplacer(&ancienne, &valeur, &format!("branches_suivies:{neuf}"))?;
    }
    Ok(journal)
}

/// La résolution entière d'un dépôt : ses sources, sa pile, son identifiant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub sources: Vec<SourceResolue>,
    pub pile: Pile,
    pub identifiant: Option<Identifiant>,
}

/// **Le résolveur d'un dépôt, et son cache** : la résolution ne se refait que si l'empreinte bouge. Cette
/// empreinte est le hash des sorties de `git for-each-ref` et `git worktree list --porcelain`, de la
/// déclaration, des tâches et du worktree du backend. Sans mouvement, deux appels git et rien d'autre : pas
/// de merge-base (le ticket O(suivies²) de l'état).
#[derive(Default)]
pub struct Resolveur {
    cache: std::sync::Mutex<Option<(u64, Resolution)>>,
    calculs: std::sync::atomic::AtomicUsize,
}

impl Resolveur {
    /// La résolution, depuis le cache si l'empreinte n'a pas bougé.
    pub fn resoudre(
        &self,
        racine: &std::path::Path,
        worktree_du_backend: &std::path::Path,
        taches: &[String],
        declaration: &Declaration,
    ) -> Result<Resolution, String> {
        use std::hash::{Hash, Hasher};
        let refs = crate::couches::git(racine, &["for-each-ref", "--format=%(refname)%00%(objectname)%00%(*objectname)"])?;
        let worktrees = crate::couches::git(racine, &["worktree", "list", "--porcelain"])?;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (refs, worktrees, format!("{declaration:?}"), taches, worktree_du_backend).hash(&mut h);
        let empreinte = h.finish();
        let mut cache = self.cache.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((e, resolution)) = cache.as_ref() {
            if *e == empreinte {
                return Ok(resolution.clone());
            }
        }
        let sources = resoudre_les_sources(racine, worktree_du_backend, taches, declaration)?;
        let pile = resoudre_la_pile(racine, declaration, &sources)?;
        let pointes: Vec<String> = pile.couches.iter().map(|c| c.pointe.clone()).collect();
        let identifiant = identifiant_des_suivies(racine, &pointes)?;
        let resolution = Resolution { sources, pile, identifiant };
        self.calculs.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *cache = Some((empreinte, resolution.clone()));
        Ok(resolution)
    }

    /// Combien de résolutions entières ce résolveur a calculées (les autres sont venues du cache).
    pub fn calculs(&self) -> usize {
        self.calculs.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// L'identifiant de la couche de travail du worktree du backend : celui des lignes de 2a′-1
/// (`COUCHE_DU_BACKEND`), posé tel quel et jamais dérivé, pour que rien de posé ne bouge.
pub const ID_DE_LA_COUCHE_DU_BACKEND: &str = "backend";

/// **Les identifiants stables des couches** d'une résolution (décision d'ED et d'arbre principal, 14 nov.,
/// page `docs/arbre-principal/14-11-2026/02-les-suivies-tracked.md`) : un id par couche, distinct de son nom,
/// dérivé de sa naissance (le dépôt, le nom complet et la pointe à la première vue) et gardé dans
/// `_catalog_meta` (`fiche_couche:<dépôt>:<id>`, et l'index `couche_nom:<dépôt>:<nom>`). Un renommage garde
/// l'id ; une ref effacée reçoit sa fin ; recréée ensuite sous le même nom, elle reçoit un id neuf ; la couche
/// de travail du worktree du backend reçoit [`ID_DE_LA_COUCHE_DU_BACKEND`]. Rend nom → id, une entrée par
/// couche d'histoire (son nom complet) et par couche de travail (`worktree:<chemin>`).
pub fn identifiants_des_couches(
    catalog: &crate::catalog::Catalog,
    depot: &str,
    worktree_du_backend: &std::path::Path,
    resolution: &Resolution,
) -> Result<std::collections::BTreeMap<String, String>, String> {
    use serde_json::json;
    use std::collections::BTreeMap;
    let e = |e: crate::catalog::CatalogError| e.to_string();
    let index = |nom: &str| format!("couche_nom:{depot}:{nom}");
    let fiche = |id: &str| format!("fiche_couche:{depot}:{id}");
    let lire_fiche = |id: &str| -> Result<Option<Value>, String> {
        Ok(catalog.read_meta_key(&fiche(id)).map_err(e)?.filter(|v| !v.is_empty()).and_then(|v| serde_json::from_str(&v).ok()))
    };
    let ecrire_fiche = |id: &str, v: &Value| catalog.persist_meta_key(&fiche(id), &v.to_string()).map_err(e);

    // Ce qui est là : chaque couche d'histoire par son nom complet, chaque couche de travail par son chemin.
    let mut presents: BTreeMap<String, String> = resolution.pile.couches.iter().map(|c| (c.nom.clone(), c.pointe.clone())).collect();
    for s in &resolution.sources {
        for t in &s.travail {
            presents.insert(format!("worktree:{}", t.chemin.display()), t.commit.clone());
        }
    }
    let du_backend = worktree_du_backend.canonicalize().ok().and_then(|b| {
        resolution
            .sources
            .iter()
            .flat_map(|s| s.travail.iter())
            .filter(|t| b.starts_with(&t.chemin))
            .max_by_key(|t| t.chemin.as_os_str().len())
            .map(|t| format!("worktree:{}", t.chemin.display()))
    });

    // Ce que l'index tenait de vivant.
    let prefixe = format!("couche_nom:{depot}:");
    let mut vivants: BTreeMap<String, (String, Value)> = BTreeMap::new();
    for (cle, id) in catalog.meta_par_prefixe(&prefixe).map_err(e)? {
        if let Some(f) = lire_fiche(&id)? {
            if f.get("fin").is_none() {
                vivants.insert(cle[prefixe.len()..].to_string(), (id, f));
            }
        }
    }
    let disparus: Vec<String> = vivants.keys().filter(|n| !presents.contains_key(*n)).cloned().collect();
    let mut apparus: Vec<String> = presents.keys().filter(|n| !vivants.contains_key(*n)).cloned().collect();
    let mut out: BTreeMap<String, String> = BTreeMap::new();

    // Un renommage, seulement sans équivoque (la garde d'ED) : sur cette pointe, une seule couche a disparu,
    // une seule est apparue, et aucune autre couche présente ne la porte.
    for d in &disparus {
        let (id, f) = vivants[d].clone();
        let vue = f.get("pointe_vue").and_then(Value::as_str).unwrap_or_default().to_string();
        let meme_pointe_disparus = disparus.iter().filter(|x| vivants[*x].1.get("pointe_vue").and_then(Value::as_str) == Some(vue.as_str())).count();
        let sur_la_pointe: Vec<&String> = apparus.iter().filter(|n| presents[*n] == vue).collect();
        let autres_vivants = presents.iter().filter(|(n, p)| **p == vue && !apparus.contains(n)).count();
        let meme_genre = |n: &str| n.starts_with("worktree:") == d.starts_with("worktree:");
        if meme_pointe_disparus == 1 && sur_la_pointe.len() == 1 && autres_vivants == 0 && meme_genre(sur_la_pointe[0]) {
            let neuf = sur_la_pointe[0].clone();
            let mut f = f.clone();
            f["nom"] = json!(neuf);
            ecrire_fiche(&id, &f)?;
            catalog.persist_meta_key(&index(d), "").map_err(e)?;
            catalog.persist_meta_key(&index(&neuf), &id).map_err(e)?;
            apparus.retain(|n| *n != neuf);
            out.insert(neuf, id);
        } else {
            let mut f = f.clone();
            f["fin"] = json!(true);
            ecrire_fiche(&id, &f)?;
            catalog.persist_meta_key(&index(d), "").map_err(e)?;
        }
    }
    // Les naissances : l'id dérivé de la naissance, jamais celui d'une autre lignée (une fiche qui existe déjà
    // sous cet id, finie, fait prendre le suivant).
    for n in &apparus {
        let pointe = presents[n].clone();
        let id = if du_backend.as_deref() == Some(n.as_str()) && lire_fiche(ID_DE_LA_COUCHE_DU_BACKEND)?.is_none_or(|f| f.get("fin").is_some()) {
            ID_DE_LA_COUCHE_DU_BACKEND.to_string()
        } else {
            let mut rang = 0u32;
            loop {
                let r = rang.to_string();
                let parts: Vec<&str> = if rang == 0 { vec![depot, n, &pointe] } else { vec![depot, n, &pointe, &r] };
                let id = crate::uuid::hashsafe_uuid("Couche", &parts);
                if lire_fiche(&id)?.is_none() {
                    break id;
                }
                rang += 1;
            }
        };
        ecrire_fiche(&id, &json!({"nom": n, "naissance": {"nom": n, "pointe": pointe}, "pointe_vue": pointe}))?;
        catalog.persist_meta_key(&index(n), &id).map_err(e)?;
        out.insert(n.clone(), id);
    }
    // Les vivantes restées : leur pointe vue suit.
    for (n, (id, f)) in &vivants {
        if let Some(p) = presents.get(n) {
            if f.get("pointe_vue").and_then(Value::as_str) != Some(p.as_str()) {
                let mut f = f.clone();
                f["pointe_vue"] = json!(p);
                ecrire_fiche(id, &f)?;
            }
            out.insert(n.clone(), id.clone());
        }
    }
    Ok(out)
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

    /// Un dépôt de test vierge, pour les décors de la pile.
    fn vierge() -> (tempfile::TempDir, std::path::PathBuf) {
        let t = tempfile::tempdir().unwrap();
        let d = t.path().join("depot");
        std::fs::create_dir_all(&d).unwrap();
        git(&d, &["init", "-q"]);
        (t, d)
    }

    fn commit(d: &std::path::Path, m: &str) {
        git(d, &["commit", "-q", "--allow-empty", "-m", m]);
    }

    fn branches(noms: &[&str], parent: Parente) -> Declaration {
        Declaration {
            root: Racine::Calculee,
            sources: noms.iter().map(|n| source(Selecteur::Branche((*n).into()), parent.clone(), Histoire::Pointe, false)).collect(),
        }
    }

    fn pile(d: &std::path::Path, decl: &Declaration) -> Pile {
        let sources = resoudre_les_sources(d, d, &[], decl).unwrap();
        resoudre_la_pile(d, decl, &sources).unwrap()
    }

    fn parente<'a>(p: &'a Pile, nom: &str) -> &'a ParenteResolue {
        &p.couches.iter().find(|c| c.nom == format!("refs/heads/{nom}")).unwrap_or_else(|| panic!("{nom} absente : {p:?}")).parente
    }

    /// **La racine, dans ses trois natures** : calculée (le merge-base de toutes les pointes, et la suivie
    /// qui la fixe : une branche partie de loin la tire en arrière), vide, déclarée ; une racine déclarée que
    /// git ne nomme pas est dite introuvable ; des historiques orphelins n'ont aucun ancêtre commun.
    #[test]
    fn la_racine_dans_ses_trois_natures_et_la_suivie_qui_la_fixe() {
        let (_t, d) = vierge();
        commit(&d, "m1");
        git(&d, &["branch", "vieille"]);
        commit(&d, "m2");
        commit(&d, "m3");
        git(&d, &["switch", "-q", "-c", "a"]);
        commit(&d, "a1");
        git(&d, &["switch", "-q", "vieille"]);
        commit(&d, "v1");
        git(&d, &["switch", "-q", "main"]);
        let m1 = git(&d, &["rev-parse", "main~2"]);
        let toutes = pile(&d, &branches(&["main", "a", "vieille"], Parente::Inferee));
        assert_eq!(toutes.racine, RacineResolue::Calculee { commit: m1.clone(), fixee_par: vec!["refs/heads/vieille".into()] }, "{toutes:?}");
        let sans_la_vieille = pile(&d, &branches(&["main", "a"], Parente::Inferee));
        assert_eq!(sans_la_vieille.racine, RacineResolue::Calculee { commit: git(&d, &["rev-parse", "main"]), fixee_par: vec!["refs/heads/main".into()] }, "{sans_la_vieille:?}");
        let mut vide = branches(&["main", "a"], Parente::Inferee);
        vide.root = Racine::Vide;
        assert_eq!(pile(&d, &vide).racine, RacineResolue::Vide);
        let mut declaree = branches(&["main", "a"], Parente::Inferee);
        declaree.root = Racine::Declaree("main~1".into());
        assert_eq!(pile(&d, &declaree).racine, RacineResolue::Declaree { rev: "main~1".into(), commit: git(&d, &["rev-parse", "main~1"]) });
        declaree.root = Racine::Declaree("nulle-part".into());
        assert_eq!(pile(&d, &declaree).racine, RacineResolue::Introuvable("nulle-part".into()));
        git(&d, &["switch", "-q", "--orphan", "orpheline"]);
        commit(&d, "o1");
        git(&d, &["switch", "-q", "main"]);
        let orphelins = pile(&d, &branches(&["main", "orpheline"], Parente::Inferee));
        assert_eq!(orphelins.racine, RacineResolue::SansAncetreCommun, "{orphelins:?}");
        assert_eq!(parente(&orphelins, "orpheline"), &ParenteResolue::Aucune, "deux suivies sans ancêtre commun : aucune parente");
    }

    /// **La parente inférée parmi les suivies** : le parent commun le plus récent, avec sa base ; une branche
    /// que les autres contiennent n'a pas de parente parmi elles ; deux candidates à égalité, c'est ambigu ;
    /// une parente déclarée ou « aucune » est gardée telle quelle.
    #[test]
    fn la_parente_inferee_parmi_les_suivies_unique_ou_ambigue() {
        let (_t, d) = vierge();
        commit(&d, "m1");
        commit(&d, "m2");
        git(&d, &["switch", "-q", "-c", "a"]);
        commit(&d, "a1");
        git(&d, &["switch", "-q", "-c", "b"]);
        commit(&d, "b1");
        git(&d, &["switch", "-q", "main"]);
        git(&d, &["branch", "c", "a"]);
        git(&d, &["branch", "e", "a"]);
        git(&d, &["switch", "-q", "e"]);
        commit(&d, "e1");
        git(&d, &["switch", "-q", "main"]);
        let p = pile(&d, &branches(&["main", "a", "b"], Parente::Inferee));
        assert_eq!(parente(&p, "a"), &ParenteResolue::Inferee { parente: "refs/heads/main".into(), base: git(&d, &["rev-parse", "main"]) }, "{p:?}");
        assert_eq!(parente(&p, "b"), &ParenteResolue::Inferee { parente: "refs/heads/a".into(), base: git(&d, &["rev-parse", "a"]) }, "{p:?}");
        assert_eq!(parente(&p, "main"), &ParenteResolue::Aucune, "les autres contiennent main : {p:?}");
        // a et c pointent sur le même commit, e en part : deux candidates à égalité.
        let egalite = pile(&d, &branches(&["main", "a", "c", "e"], Parente::Inferee));
        assert_eq!(parente(&egalite, "e"), &ParenteResolue::Egales(vec!["refs/heads/a".into(), "refs/heads/c".into()]), "{egalite:?}");
        assert!(!egalite.avertissements.iter().any(|a| a.contains("refs/heads/e")), "des pointes égales ne s'avertissent pas : {:?}", egalite.avertissements);
        let mut declaree = branches(&["main", "a", "b"], Parente::Inferee);
        declaree.sources[2].parent = Parente::Declaree("main".into());
        declaree.sources[1].parent = Parente::Aucune;
        let d2 = pile(&d, &declaree);
        assert_eq!(parente(&d2, "b"), &ParenteResolue::Declaree("refs/heads/main".into()));
        assert_eq!(parente(&d2, "a"), &ParenteResolue::Aucune);
    }

    /// **Une fonctionnalité finie n'est pas un flux** (décision de c7, 14 nov.) : `f` part de main, reprend
    /// main en elle, puis main la fusionne. Sa pointe est contenue dans main : elle ne compte dans aucun flux,
    /// sa parente est main, et main n'a pas `f` pour parente. Rien ne s'avertit.
    #[test]
    fn une_fonctionnalite_reprise_puis_fusionnee_n_est_pas_un_flux() {
        let (_t, d) = vierge();
        commit(&d, "m1");
        git(&d, &["switch", "-q", "-c", "f"]);
        commit(&d, "f1");
        git(&d, &["switch", "-q", "main"]);
        commit(&d, "m2");
        git(&d, &["switch", "-q", "f"]);
        git(&d, &["merge", "-q", "--no-ff", "--no-edit", "main"]);
        commit(&d, "f2");
        git(&d, &["switch", "-q", "main"]);
        git(&d, &["merge", "-q", "--no-ff", "--no-edit", "f"]);
        let p = pile(&d, &branches(&["main", "f"], Parente::Inferee));
        assert!(p.flux.is_empty(), "une fonctionnalité finie n'est pas un flux : {p:?}");
        assert!(matches!(parente(&p, "f"), ParenteResolue::Inferee { parente, .. } if parente == "refs/heads/main"), "{p:?}");
        assert_eq!(parente(&p, "main"), &ParenteResolue::Aucune, "{p:?}");
        assert!(p.avertissements.is_empty(), "{:?}", p.avertissements);
    }

    /// **Le flux multi-env** (Lucie, 14 nov.) : develop → staging → main par promotions, un hotfix sur main
    /// redescendu dans develop, puis la promotion suivante en cours (develop devant, un second hotfix sur
    /// main) : trois pointes dont aucune ne contient l'autre, liées par un cycle de fusions. L'état le nomme,
    /// et ne pose aucune parente inférée entre elles (elle changerait au prochain resync).
    #[test]
    fn un_cycle_de_fusions_entre_suivies_est_un_flux_nomme() {
        let (_t, d) = vierge();
        commit(&d, "m1");
        for b in ["develop", "staging"] {
            git(&d, &["branch", b]);
        }
        let fusionner = |dans: &str, quoi: &str| {
            git(&d, &["switch", "-q", dans]);
            git(&d, &["merge", "-q", "--no-ff", "--no-edit", quoi]);
        };
        let sur = |b: &str, m: &str| {
            git(&d, &["switch", "-q", b]);
            commit(&d, m);
        };
        sur("develop", "d1");
        fusionner("staging", "develop");
        fusionner("main", "staging");
        sur("main", "h1");
        fusionner("develop", "main");
        sur("develop", "d2");
        fusionner("staging", "develop");
        sur("develop", "d3");
        sur("main", "h2");
        git(&d, &["switch", "-q", "main"]);
        let p = pile(&d, &branches(&["develop", "staging", "main"], Parente::Inferee));
        assert_eq!(p.flux, vec![vec!["refs/heads/develop".to_string(), "refs/heads/main".into(), "refs/heads/staging".into()]], "{p:?}");
        for b in ["develop", "staging", "main"] {
            assert_eq!(parente(&p, b), &ParenteResolue::DansUnFlux, "{b} : {p:?}");
        }
        assert!(p.avertissements.iter().any(|a| a.contains("flux") && a.contains("develop") && a.contains("staging") && a.contains("main")), "{:?}", p.avertissements);
    }

    /// Un commit daté, pour départager des racines.
    fn commit_date(d: &std::path::Path, m: &str, date: &str) {
        let o = std::process::Command::new("git")
            .arg("-C").arg(d)
            .args(["-c", "user.name=Témoin", "-c", "user.email=temoin@example.invalid", "-c", "commit.gpgsign=false", "commit", "-q", "--allow-empty", "-m", m])
            .env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date)
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }

    /// **L'identifiant du dépôt est la racine du plus vieux commit des suivies** : un historique orphelin plus
    /// récent ne le change pas ; suivi seul, il donne le sien ; deux racines de même date sont nommées toutes
    /// les deux, et la première par hash fait l'identifiant.
    #[test]
    fn l_identifiant_du_depot_est_la_racine_du_plus_vieux_commit() {
        let (_t, d) = vierge();
        commit_date(&d, "vieux", "2020-01-01T00:00:00Z");
        commit_date(&d, "m2", "2021-01-01T00:00:00Z");
        let vieux = git(&d, &["rev-parse", "main~1"]);
        git(&d, &["switch", "-q", "--orphan", "neuve"]);
        commit_date(&d, "neuve", "2024-01-01T00:00:00Z");
        let neuve = git(&d, &["rev-parse", "neuve"]);
        git(&d, &["switch", "-q", "--orphan", "jumelle"]);
        commit_date(&d, "jumelle", "2024-01-01T00:00:00Z");
        let jumelle = git(&d, &["rev-parse", "jumelle"]);
        git(&d, &["switch", "-q", "main"]);
        let id = |b: &[&str]| identifiant_des_suivies(&d, &b.iter().map(|x| git(&d, &["rev-parse", x])).collect::<Vec<_>>()).unwrap().unwrap();
        assert_eq!(id(&["main", "neuve"]), Identifiant { id: vieux.clone(), egaux: vec![] });
        assert_eq!(id(&["neuve"]), Identifiant { id: neuve.clone(), egaux: vec![] });
        let mut egaux = vec![neuve.clone(), jumelle.clone()];
        egaux.sort();
        assert_eq!(id(&["neuve", "jumelle"]), Identifiant { id: egaux[0].clone(), egaux: egaux.clone() });
        assert_eq!(identifiant_des_suivies(&d, &[]).unwrap(), None);
    }

    fn catalogue() -> crate::Catalog {
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let mut cat = crate::Catalog::without_embedding_model(Box::new(conn), crate::CatalogConfig::default());
        cat.initialize().unwrap();
        cat
    }

    /// **Les clés migrent sans rien perdre** : d'un identifiant à l'autre, les noms courts en nom complet
    /// (`a` devient `refs/heads/a`, même si une étiquette `a` existe : une clé courte a toujours nommé une
    /// branche) ; une clé cible déjà posée n'est jamais écrasée, la source reste et le journal le dit.
    #[test]
    fn les_cles_migrent_en_nom_complet_sans_rien_perdre() {
        let cat = catalogue();
        let lire = |k: &str| cat.read_meta_key(k).unwrap().filter(|v| !v.is_empty());
        let poser = |k: &str, v: &str| cat.persist_meta_key(k, v).unwrap();
        poser("couche:ancien:a", "{\"branche\":\"main\"}");
        poser("couche_fermee:ancien:a", "fermée");
        poser("couche:ancien:refs/heads/b", "{\"branche\":\"a\"}");
        poser("couche:ancien:c", "{\"branche\":\"court\"}");
        poser("couche:neuf:refs/heads/c", "{\"branche\":\"deja\"}");
        poser("branches_suivies:ancien", "{\"ancre\":\"main\",\"suivies\":[\"a\"]}");
        let journal = migrer_les_cles(&cat, "ancien", "neuf").unwrap();
        assert_eq!(lire("couche:neuf:refs/heads/a").as_deref(), Some("{\"branche\":\"main\"}"));
        assert_eq!(lire("couche_fermee:neuf:refs/heads/a").as_deref(), Some("fermée"));
        assert_eq!(lire("couche:neuf:refs/heads/b").as_deref(), Some("{\"branche\":\"a\"}"));
        assert_eq!(lire("couche:neuf:refs/tags/a"), None, "une clé courte nomme une branche, jamais une étiquette");
        assert_eq!(lire("branches_suivies:neuf").as_deref(), Some("{\"ancre\":\"main\",\"suivies\":[\"a\"]}"));
        for vide in ["couche:ancien:a", "couche_fermee:ancien:a", "couche:ancien:refs/heads/b", "branches_suivies:ancien"] {
            assert_eq!(lire(vide), None, "{vide} reste après sa migration");
        }
        // La garde : la cible déjà posée gagne, la source n'est pas effacée, et c'est dit.
        assert_eq!(lire("couche:neuf:refs/heads/c").as_deref(), Some("{\"branche\":\"deja\"}"));
        assert_eq!(lire("couche:ancien:c").as_deref(), Some("{\"branche\":\"court\"}"), "la source a été perdue");
        assert!(journal.iter().any(|l| l.contains("couche:ancien:c") && l.contains("garde")), "{journal:?}");
        // Le même identifiant : seuls les noms courts se réécrivent.
        poser("couche:neuf:d", "{\"branche\":\"main\"}");
        migrer_les_cles(&cat, "neuf", "neuf").unwrap();
        assert_eq!(lire("couche:neuf:refs/heads/d").as_deref(), Some("{\"branche\":\"main\"}"));
        assert_eq!(lire("couche:neuf:d"), None);
    }

    /// **Sans mouvement de refs, la résolution vient du cache** : un deuxième appel ne recalcule rien ; un
    /// commit sur une suivie, une branche neuve sous un motif ou une tâche de plus la recalculent.
    #[test]
    fn sans_mouvement_de_refs_la_resolution_vient_du_cache() {
        let (_t, d) = vierge();
        commit(&d, "m1");
        git(&d, &["branch", "a"]);
        let decl = Declaration {
            root: Racine::Calculee,
            sources: vec![
                source(Selecteur::Branche("main".into()), Parente::Inferee, Histoire::Pointe, false),
                source(Selecteur::Refs("refs/heads/r/*".into()), Parente::Inferee, Histoire::Pointe, false),
            ],
        };
        let r = Resolveur::default();
        let premiere = r.resoudre(&d, &d, &[], &decl).unwrap();
        assert_eq!(r.calculs(), 1);
        assert_eq!(r.resoudre(&d, &d, &[], &decl).unwrap(), premiere, "le cache rend la même résolution");
        assert_eq!(r.calculs(), 1, "un deuxième appel sans mouvement a recalculé");
        commit(&d, "m2");
        let apres = r.resoudre(&d, &d, &[], &decl).unwrap();
        assert_eq!(r.calculs(), 2, "un commit sur une suivie n'a pas recalculé");
        assert_ne!(apres.pile.couches[0].pointe, premiere.pile.couches[0].pointe);
        git(&d, &["branch", "r/neuve"]);
        let motif = r.resoudre(&d, &d, &[], &decl).unwrap();
        assert_eq!(r.calculs(), 3, "une branche neuve sous le motif n'a pas recalculé");
        assert!(motif.sources[1].refs.iter().any(|x| x.nom == "refs/heads/r/neuve"), "{motif:?}");
        r.resoudre(&d, &d, &["a".to_string()], &decl).unwrap();
        assert_eq!(r.calculs(), 4, "une tâche de plus n'a pas recalculé");
    }

    /// **Une couche a un identifiant stable, distinct de son nom** : le même d'une résolution à l'autre ; un
    /// renommage de branche le garde ; une branche effacée puis recréée ailleurs sous le même nom en reçoit un
    /// neuf, et l'ancienne fiche garde sa fin ; la couche de travail du worktree du backend reçoit `backend`.
    #[test]
    fn une_couche_a_un_identifiant_stable_distinct_de_son_nom() {
        let (_t, d) = vierge();
        commit(&d, "m1");
        git(&d, &["switch", "-q", "-c", "a"]);
        commit(&d, "a1");
        git(&d, &["switch", "-q", "main"]);
        let cat = catalogue();
        let decl = Declaration {
            root: Racine::Calculee,
            sources: vec![
                source(Selecteur::Worktree(Worktree::DuBackend), Parente::Inferee, Histoire::Pointe, true),
                source(Selecteur::Refs("refs/heads/*".into()), Parente::Inferee, Histoire::Pointe, false),
            ],
        };
        let ids = |r: &Resolveur| identifiants_des_couches(&cat, "depot", &d, &r.resoudre(&d, &d, &[], &decl).unwrap()).unwrap();
        let r = Resolveur::default();
        let premiers = ids(&r);
        let a = premiers["refs/heads/a"].clone();
        assert_eq!(premiers[&format!("worktree:{}", reel(&d).display())], ID_DE_LA_COUCHE_DU_BACKEND, "{premiers:?}");
        assert_ne!(premiers["refs/heads/main"], ID_DE_LA_COUCHE_DU_BACKEND, "l'histoire de main n'est pas la couche de travail du backend");
        assert_ne!(a, premiers["refs/heads/main"]);
        assert_eq!(ids(&r), premiers, "l'identifiant a changé sans mouvement");
        // Le renommage garde l'identifiant.
        git(&d, &["branch", "-m", "a", "b"]);
        let renomme = ids(&r);
        assert_eq!(renomme.get("refs/heads/b"), Some(&a), "le renommage a changé l'identifiant : {renomme:?}");
        assert!(!renomme.contains_key("refs/heads/a"), "{renomme:?}");
        // Effacée, puis recréée ailleurs sous le même nom : un identifiant neuf, l'ancien garde sa fin.
        git(&d, &["branch", "-D", "b"]);
        ids(&r);
        let fiche = cat.read_meta_key(&format!("fiche_couche:depot:{a}")).unwrap().unwrap_or_default();
        assert!(fiche.contains("fin"), "la fiche de la couche effacée n'a pas sa fin : {fiche}");
        git(&d, &["branch", "b", "main"]);
        let recree = ids(&r);
        assert_ne!(recree["refs/heads/b"], a, "une branche recréée ailleurs a repris l'identifiant de l'ancienne");
        // La garde d'ED : une branche effacée et une autre créée sur la même pointe, pendant qu'une ref vivante
        // (main) la porte encore, ce n'est pas un renommage : une fin et une naissance.
        let b = recree["refs/heads/b"].clone();
        git(&d, &["branch", "-D", "b"]);
        git(&d, &["branch", "c", "main"]);
        let apres = ids(&r);
        assert_ne!(apres["refs/heads/c"], b, "une branche neuve sur une pointe que main porte a repris l'identifiant d'une effacée : {apres:?}");
        // La garde de c7 : deux couches vivantes n'ont jamais le même identifiant.
        let mut vus: Vec<&String> = apres.values().collect();
        vus.sort();
        let avant = vus.len();
        vus.dedup();
        assert_eq!(vus.len(), avant, "deux couches vivantes partagent un identifiant : {apres:?}");
    }
}
