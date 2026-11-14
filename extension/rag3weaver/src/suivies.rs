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
}
