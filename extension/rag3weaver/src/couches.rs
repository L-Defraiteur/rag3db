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

use std::path::Path;

/// Comment une parente est connue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenreDeParente {
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
    let _ = (depot, ancre, suivies);
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
