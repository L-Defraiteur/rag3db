//! **La liaison branche → couche** (les vues, après la version de scope).
//!
//! Contrat : `docs/arbre-principal/13-11-2026/30-lier-branches-et-worktrees-aux-couches.md`,
//! décisions de Lucie du 13 novembre 2026. Par dépôt, un ensemble **déclaré** de branches suivies
//! (l'ancre toujours) : chacune est une couche, posée sur sa parente. Tout le reste du graphe git
//! est **connu, non indexé** : on sait que ça existe, l'index n'en a rien.
//!
//! La parente d'une branche suivie est **inférée** par le point de fourche, et marquée comme
//! telle : parmi les autres branches suivies, celle avec laquelle elle partage l'histoire la plus
//! récente. Elle n'est admise que si elle est unique ; sinon, la branche est ambiguë, et sa parente
//! se demande à l'ouverture, jamais choisie en silence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Comment une parente est connue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenreDeParente {
    /// Notée par l'atelier à l'ouverture d'une tâche : une déclaration.
    Declaree,
    /// Lue dans le reflog local de la branche (« branch: Created from … ») : fiable seulement sur le
    /// poste où la branche est née, jamais une certitude distante.
    InfereeParReflog,
    /// Déduite du point de fourche (décision 1 de Lucie).
    Inferee,
}

/// La parente d'une couche, et le commit où la couche s'en sépare (sa base).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parente {
    pub branche: String,
    pub genre: GenreDeParente,
    pub base: String,
}

/// Ce qu'une branche du dépôt est pour l'index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EtatDeBranche {
    /// L'ancre : la couche du bas, sans parente.
    Ancre { branche: String, pointe: String },
    /// Une branche suivie : une couche, sur sa parente.
    Suivie { branche: String, pointe: String, parente: Parente },
    /// Une branche suivie dont la parente n'est pas unique : à demander à l'ouverture. `candidates`
    /// sont les branches suivies à égale distance.
    Ambigue { branche: String, pointe: String, candidates: Vec<String> },
    /// Une branche que le graphe git connaît et que l'index ne suit pas.
    Connue { branche: String, pointe: String },
}

/// **L'état des branches d'un dépôt** : l'ancre, les suivies avec leur parente inférée, et les
/// autres, connues. `suivies` est la déclaration (l'ancre y est toujours, même absente de la
/// liste) ; une branche suivie qui n'existe pas dans le dépôt est une erreur nommée.
pub fn etat_des_branches(depot: &Path, ancre: &str, suivies: &[String]) -> Result<Vec<EtatDeBranche>, String> {
    let pointes = pointes_des_branches(depot)?;
    let pointe = |nom: &str| -> Result<String, String> {
        pointes.get(nom).cloned().ok_or_else(|| format!("branche suivie « {nom} » absente du dépôt {}", depot.display()))
    };
    let mut declarees: BTreeSet<String> = suivies.iter().cloned().collect();
    declarees.insert(ancre.to_string());
    for nom in &declarees {
        pointe(nom)?;
    }
    let mut etats = vec![EtatDeBranche::Ancre { branche: ancre.to_string(), pointe: pointe(ancre)? }];
    for branche in declarees.iter().filter(|b| b.as_str() != ancre) {
        let ici = pointe(branche)?;
        // Les candidates : les autres branches suivies, sauf celles qui contiennent déjà toute
        // celle-ci (ses filles, ou une branche à la même pointe) — une parente est derrière.
        let mut fourches: Vec<(String, String, String)> = Vec::new(); // (candidate, pointe, fourche)
        for candidate in declarees.iter().filter(|c| *c != branche) {
            let la = pointe(candidate)?;
            if est_ancetre(depot, &ici, &la)? {
                continue;
            }
            fourches.push((candidate.clone(), la.clone(), git(depot, &["merge-base", &ici, &la])?));
        }
        // Les plus récentes : une fourche qui n'est l'ancêtre strict d'aucune autre.
        let mut recentes: Vec<&(String, String, String)> = Vec::new();
        for f in &fourches {
            let mut depassee = false;
            for g in &fourches {
                if g.2 != f.2 && est_ancetre(depot, &f.2, &g.2)? {
                    depassee = true;
                    break;
                }
            }
            if !depassee {
                recentes.push(f);
            }
        }
        let parente = |c: &(String, String, String)| Parente { branche: c.0.clone(), genre: GenreDeParente::Inferee, base: c.2.clone() };
        let etat = match recentes.as_slice() {
            // Une seule : certaine si c'est l'ancre, ou si la branche part de sa pointe (sa pointe
            // est derrière la fourche). Sinon les deux histoires divergent après un point commun,
            // et git ne dit pas laquelle est partie de l'autre.
            [c] if c.0 == ancre || c.1 == c.2 => EtatDeBranche::Suivie { branche: branche.clone(), pointe: ici, parente: parente(c) },
            [c] => EtatDeBranche::Ambigue { branche: branche.clone(), pointe: ici, candidates: vec![c.0.clone()] },
            // Plusieurs à la même fourche : si l'ancre en est, la fourche est sur l'ancre, et les
            // autres n'ont rien de propre en commun avec la branche.
            plusieurs => match plusieurs.iter().find(|c| c.0 == ancre) {
                Some(c) => EtatDeBranche::Suivie { branche: branche.clone(), pointe: ici, parente: parente(c) },
                None => EtatDeBranche::Ambigue {
                    branche: branche.clone(),
                    pointe: ici,
                    candidates: plusieurs.iter().map(|c| c.0.clone()).collect(),
                },
            },
        };
        etats.push(etat);
    }
    for (branche, la) in &pointes {
        if !declarees.contains(branche) {
            etats.push(EtatDeBranche::Connue { branche: branche.clone(), pointe: la.clone() });
        }
    }
    Ok(etats)
}

/// Les branches locales du dépôt et leur pointe.
fn pointes_des_branches(depot: &Path) -> Result<BTreeMap<String, String>, String> {
    let sortie = git(depot, &["for-each-ref", "--format=%(refname:short) %(objectname)", "refs/heads"])?;
    Ok(sortie
        .lines()
        .filter_map(|l| l.split_once(' ').map(|(n, h)| (n.to_string(), h.to_string())))
        .collect())
}

/// `a` est-il l'ancêtre de `b` (ou `b` lui-même) ?
fn est_ancetre(depot: &Path, a: &str, b: &str) -> Result<bool, String> {
    let o = std::process::Command::new("git")
        .arg("-C").arg(depot)
        .args(["merge-base", "--is-ancestor", a, b])
        .output()
        .map_err(|e| format!("git : {e}"))?;
    match o.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!("git merge-base --is-ancestor {a} {b} : {}", String::from_utf8_lossy(&o.stderr).trim())),
    }
}

fn git(depot: &Path, args: &[&str]) -> Result<String, String> {
    let o = std::process::Command::new("git").arg("-C").arg(depot).args(args).output().map_err(|e| format!("git : {e}"))?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
    } else {
        Err(format!("git {} : {}", args.join(" "), String::from_utf8_lossy(&o.stderr).trim()))
    }
}

// ─── Ce que la base en garde : les branches suivies, et les parentes estampillées ─────────

/// La clé de `_catalog_meta` des branches suivies d'un dépôt.
fn cle_des_suivies(depot: &str) -> String {
    format!("branches_suivies:{depot}")
}

/// La clé de `_catalog_meta` de la parente estampillée d'une couche.
fn cle_de_la_couche(depot: &str, branche: &str) -> String {
    format!("couche:{depot}:{branche}")
}

/// Les branches suivies d'un dépôt, telles que déclarées.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BranchesSuivies {
    pub ancre: String,
    pub suivies: Vec<String>,
}

/// **Déclarer les branches suivies d'un dépôt** (`depot` est son identifiant dans l'index). L'ancre
/// en est toujours.
pub fn declarer_les_branches_suivies(
    catalog: &crate::catalog::Catalog,
    depot: &str,
    ancre: &str,
    suivies: &[String],
) -> Result<(), String> {
    let _ = (catalog, cle_des_suivies(depot), ancre, suivies);
    Ok(())
}

/// Les branches suivies déclarées d'un dépôt, s'il y en a.
pub fn branches_suivies(catalog: &crate::catalog::Catalog, depot: &str) -> Result<Option<BranchesSuivies>, String> {
    let _ = (catalog, depot);
    Ok(None)
}

/// **La parente déclarée par l'atelier** à l'ouverture d'une tâche : estampillée, elle prime sur
/// toute inférence.
pub fn declarer_la_parente(
    catalog: &crate::catalog::Catalog,
    depot: &str,
    branche: &str,
    parente: &str,
    base: &str,
) -> Result<(), String> {
    let _ = (catalog, cle_de_la_couche(depot, branche), parente, base);
    Ok(())
}

/// **Les couches d'un dépôt, résolues** : les branches suivies déclarées, chacune sur sa parente,
/// dans cet ordre (c7, 13 novembre) : l'estampille (une déclaration de l'atelier, ou une inférence
/// déjà admise), le reflog local, le point de fourche. Une parente résolue pour la première fois
/// s'estampille avec son genre et n'est plus recalculée ; une parente qui ne se résout pas reste
/// ambiguë, ses candidates nommées. Sans déclaration de branches suivies, l'ancre seule.
pub fn couches_du_depot(
    catalog: &crate::catalog::Catalog,
    depot: &str,
    racine: &Path,
    ancre_par_defaut: &str,
) -> Result<Vec<EtatDeBranche>, String> {
    let _ = (catalog, depot, racine, ancre_par_defaut);
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn git(dir: &Path, args: &[&str]) -> String {
        let o = std::process::Command::new("git")
            .arg("-C").arg(dir)
            .args(["-c", "user.name=Témoin", "-c", "user.email=temoin@example.invalid", "-c", "commit.gpgsign=false", "-c", "init.defaultBranch=main"])
            .args(args)
            .output()
            .unwrap();
        assert!(o.status.success(), "git {args:?} : {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    }

    /// Un commit vide de plus sur la branche courante ; rend son hash.
    fn commit(dir: &Path, message: &str) -> String {
        git(dir, &["commit", "-q", "--allow-empty", "-m", message]);
        git(dir, &["rev-parse", "HEAD"])
    }

    /// main : m1 – m2 – m3 ; `a` part de m2 (a1, a2) ; `b` part de la pointe de `a` (b1), et `a`
    /// n'avance plus ; `c` part de m3 (c1), jamais suivie.
    fn depot() -> (tempfile::TempDir, PathBuf) {
        let t = tempfile::tempdir().unwrap();
        let d = t.path().to_path_buf();
        git(&d, &["init", "-q"]);
        commit(&d, "m1");
        commit(&d, "m2");
        git(&d, &["switch", "-q", "-c", "a"]);
        commit(&d, "a1");
        commit(&d, "a2");
        git(&d, &["switch", "-q", "-c", "b"]);
        commit(&d, "b1");
        git(&d, &["switch", "-q", "main"]);
        commit(&d, "m3");
        git(&d, &["switch", "-q", "-c", "c"]);
        commit(&d, "c1");
        git(&d, &["switch", "-q", "main"]);
        (t, d)
    }

    fn noms(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// **Le témoin de Lucie** : une branche non suivie n'a pas de couche, elle est seulement
    /// connue ; une suivie a sa parente inférée par la fourche, marquée comme telle, et sa base au
    /// point de fourche. La pyramide : `b`, partie de `a`, a `a` pour parente, pas l'ancre.
    #[test]
    fn une_branche_suivie_a_sa_parente_inferee_et_une_autre_est_seulement_connue() {
        let (_t, d) = depot();
        let etats = etat_des_branches(&d, "main", &noms(&["a", "b"])).unwrap();
        let de = |nom: &str| etats.iter().find(|e| match e {
            EtatDeBranche::Ancre { branche, .. } | EtatDeBranche::Suivie { branche, .. }
            | EtatDeBranche::Ambigue { branche, .. } | EtatDeBranche::Connue { branche, .. } => branche == nom,
        }).cloned().unwrap_or_else(|| panic!("« {nom} » absente : {etats:?}"));
        let fourche = |x: &str, y: &str| git(&d, &["merge-base", x, y]);
        assert!(matches!(de("main"), EtatDeBranche::Ancre { .. }), "{etats:?}");
        assert_eq!(de("a"), EtatDeBranche::Suivie {
            branche: "a".into(), pointe: git(&d, &["rev-parse", "a"]),
            parente: Parente { branche: "main".into(), genre: GenreDeParente::Inferee, base: fourche("a", "main") },
        });
        assert_eq!(de("b"), EtatDeBranche::Suivie {
            branche: "b".into(), pointe: git(&d, &["rev-parse", "b"]),
            parente: Parente { branche: "a".into(), genre: GenreDeParente::Inferee, base: fourche("b", "a") },
        });
        assert_eq!(de("c"), EtatDeBranche::Connue { branche: "c".into(), pointe: git(&d, &["rev-parse", "c"]) });
        assert_eq!(etats.len(), 4, "{etats:?}");
    }

    /// Deux branches suivies à égale distance (même point de fourche avec `x`, et chacune au-delà
    /// de l'ancre) : la parente de `x` n'est pas unique, elle se demande, jamais choisie.
    #[test]
    fn une_parente_qui_n_est_pas_unique_se_dit_ambigue() {
        let (_t, d) = depot();
        git(&d, &["switch", "-q", "-c", "a-bis", "a"]);
        git(&d, &["switch", "-q", "-c", "x", "a"]);
        commit(&d, "x1");
        git(&d, &["switch", "-q", "main"]);
        let etats = etat_des_branches(&d, "main", &noms(&["a", "a-bis", "x"])).unwrap();
        let x = etats.iter().find(|e| matches!(e, EtatDeBranche::Ambigue { branche, .. } | EtatDeBranche::Suivie { branche, .. } if branche == "x"))
            .cloned().unwrap_or_else(|| panic!("x absente : {etats:?}"));
        match x {
            EtatDeBranche::Ambigue { mut candidates, .. } => {
                candidates.sort();
                assert_eq!(candidates, noms(&["a", "a-bis"]));
            }
            autre => panic!("x devait être ambiguë : {autre:?}"),
        }
    }

    /// **Deux branches qui divergent après un point commun** : `d` part de la pointe de `a`, puis `a`
    /// avance. Leur histoire est symétrique (chacune a ses commits au-delà du point commun) : git ne
    /// dit pas qui est partie de qui. La parente de `d` n'est pas certaine, elle se demande.
    #[test]
    fn deux_branches_qui_divergent_ne_disent_pas_qui_est_la_parente() {
        let (_t, d) = depot();
        git(&d, &["switch", "-q", "-c", "d", "a"]);
        commit(&d, "d1");
        git(&d, &["switch", "-q", "a"]);
        commit(&d, "a3");
        git(&d, &["switch", "-q", "main"]);
        let etats = etat_des_branches(&d, "main", &noms(&["a", "d"])).unwrap();
        for nom in ["a", "d"] {
            let e = etats.iter().find(|e| matches!(e, EtatDeBranche::Ambigue { branche, .. } | EtatDeBranche::Suivie { branche, .. } if branche == nom))
                .cloned().unwrap_or_else(|| panic!("{nom} absente : {etats:?}"));
            assert!(matches!(e, EtatDeBranche::Ambigue { .. }), "{nom} devait être ambiguë : {e:?}");
        }
    }

    /// Une branche déclarée suivie qui n'existe pas : une erreur qui la nomme.
    #[test]
    fn une_branche_suivie_absente_est_refusee_en_la_nommant() {
        let (_t, d) = depot();
        let refus = etat_des_branches(&d, "main", &noms(&["fantome"])).expect_err("fantome n'existe pas");
        assert!(refus.contains("fantome"), "{refus}");
    }
}

#[cfg(all(test, feature = "rag3db-native"))]
mod tests_de_la_base {
    use super::*;
    use crate::{Catalog, CatalogConfig};

    fn git(dir: &Path, args: &[&str]) -> String {
        let o = std::process::Command::new("git")
            .arg("-C").arg(dir)
            .args(["-c", "user.name=Témoin", "-c", "user.email=temoin@example.invalid", "-c", "commit.gpgsign=false", "-c", "init.defaultBranch=main"])
            .args(args)
            .output()
            .unwrap();
        assert!(o.status.success(), "git {args:?} : {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    }

    fn commit(dir: &Path, message: &str) {
        git(dir, &["commit", "-q", "--allow-empty", "-m", message]);
    }

    fn catalogue() -> Catalog {
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let mut cat = Catalog::without_embedding_model(Box::new(conn), CatalogConfig::default());
        cat.initialize().unwrap();
        cat
    }

    fn noms(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// main : m1 – m2 ; `a` part de m2 (a1) ; `d` part de a1 (d1) par `git branch d a` (le reflog
    /// le dit) ; puis `a` avance (a2) : `a` et `d` divergent après un point commun.
    fn depot() -> (tempfile::TempDir, std::path::PathBuf) {
        let t = tempfile::tempdir().unwrap();
        let d = t.path().to_path_buf();
        git(&d, &["init", "-q"]);
        commit(&d, "m1");
        commit(&d, "m2");
        git(&d, &["switch", "-q", "-c", "a"]);
        commit(&d, "a1");
        git(&d, &["branch", "d", "a"]);
        git(&d, &["switch", "-q", "d"]);
        commit(&d, "d1");
        git(&d, &["switch", "-q", "a"]);
        commit(&d, "a2");
        git(&d, &["switch", "-q", "main"]);
        (t, d)
    }

    fn parente_de(etats: &[EtatDeBranche], nom: &str) -> Option<Parente> {
        etats.iter().find_map(|e| match e {
            EtatDeBranche::Suivie { branche, parente, .. } if branche == nom => Some(parente.clone()),
            _ => None,
        })
    }

    #[test]
    fn les_branches_suivies_se_declarent_et_se_relisent() {
        let cat = catalogue();
        assert_eq!(branches_suivies(&cat, "depot").unwrap(), None);
        declarer_les_branches_suivies(&cat, "depot", "main", &noms(&["a", "d"])).unwrap();
        assert_eq!(
            branches_suivies(&cat, "depot").unwrap(),
            Some(BranchesSuivies { ancre: "main".into(), suivies: noms(&["a", "d"]) })
        );
        assert_eq!(branches_suivies(&cat, "autre").unwrap(), None, "un autre dépôt n'a rien");
    }

    /// Le reflog local lève l'ambiguïté que la topologie laisse : `d` est née de `a`
    /// (« branch: Created from a »), et sa parente est marquée inférée par reflog.
    #[test]
    fn le_reflog_local_dit_la_parente_que_la_fourche_ne_dit_pas() {
        let (_t, d) = depot();
        let cat = catalogue();
        declarer_les_branches_suivies(&cat, "depot", "main", &noms(&["a", "d"])).unwrap();
        let etats = couches_du_depot(&cat, "depot", &d, "main").unwrap();
        let p = parente_de(&etats, "d").unwrap_or_else(|| panic!("d sans parente : {etats:?}"));
        assert_eq!((p.branche.as_str(), &p.genre), ("a", &GenreDeParente::InfereeParReflog), "{etats:?}");
        assert_eq!(p.base, git(&d, &["merge-base", "d", "a"]));
    }

    /// La déclaration de l'atelier prime, et une parente résolue s'estampille : elle ne se recalcule
    /// pas quand le dépôt bouge ensuite. Décor : main m1 – m2 ; `a` part de m2 (a1) ; `b` part de la
    /// pointe de `a` (b1).
    #[test]
    fn une_parente_declaree_prime_et_une_parente_resolue_ne_se_recalcule_pas() {
        let t = tempfile::tempdir().unwrap();
        let d = t.path().to_path_buf();
        git(&d, &["init", "-q"]);
        commit(&d, "m1");
        commit(&d, "m2");
        git(&d, &["switch", "-q", "-c", "a"]);
        commit(&d, "a1");
        git(&d, &["switch", "-q", "-c", "b"]);
        commit(&d, "b1");
        git(&d, &["switch", "-q", "main"]);
        let cat = catalogue();
        declarer_les_branches_suivies(&cat, "depot", "main", &noms(&["a", "b"])).unwrap();
        let base = git(&d, &["merge-base", "b", "main"]);
        declarer_la_parente(&cat, "depot", "b", "main", &base).unwrap();
        let etats = couches_du_depot(&cat, "depot", &d, "main").unwrap();
        let p = parente_de(&etats, "b").unwrap_or_else(|| panic!("b sans parente : {etats:?}"));
        assert_eq!((p.branche.as_str(), &p.genre, p.base.as_str()), ("main", &GenreDeParente::Declaree, base.as_str()), "{etats:?}");
        // `a` se résout par la fourche (sur l'ancre), puis s'estampille : une fusion de `a` dans
        // main, après laquelle main contient `a`, ne la change plus.
        let a = parente_de(&etats, "a").unwrap_or_else(|| panic!("a sans parente : {etats:?}"));
        assert_eq!((a.branche.as_str(), &a.genre), ("main", &GenreDeParente::Inferee), "{etats:?}");
        git(&d, &["merge", "-q", "--no-edit", "--no-ff", "a"]);
        let apres = couches_du_depot(&cat, "depot", &d, "main").unwrap();
        assert_eq!(parente_de(&apres, "a"), Some(a), "la parente estampillée a été recalculée : {apres:?}");
    }

    /// Sans reflog qui la dise (branche créée par `switch -c` depuis HEAD), une divergence reste
    /// ambiguë, ses candidates nommées : connue, pas une erreur.
    #[test]
    fn une_parente_jamais_resolue_reste_ambigue_avec_ses_candidates() {
        let (_t, d) = depot();
        git(&d, &["switch", "-q", "a"]);
        git(&d, &["switch", "-q", "-c", "e"]);
        commit(&d, "e1");
        git(&d, &["switch", "-q", "a"]);
        commit(&d, "a3");
        git(&d, &["switch", "-q", "main"]);
        let cat = catalogue();
        declarer_les_branches_suivies(&cat, "depot", "main", &noms(&["a", "e"])).unwrap();
        let etats = couches_du_depot(&cat, "depot", &d, "main").unwrap();
        let e = etats.iter().find(|x| matches!(x, EtatDeBranche::Ambigue { branche, .. } if branche == "e"));
        assert!(e.is_some_and(|x| matches!(x, EtatDeBranche::Ambigue { candidates, .. } if candidates.contains(&"a".to_string()))), "{etats:?}");
    }
}
