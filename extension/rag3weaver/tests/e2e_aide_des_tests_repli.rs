//! E2E — **l'aide des tests et son repli** (ticket
//! `docs/arbre-principal/tickets/2026-11-14-l-aide-des-tests-panique-quand-aucun-service-ne-sert-le-modele.md`,
//! décision de c7, 14 novembre 2026). Sous une variable de service qui ne sert pas le
//! modèle demandé, l'aide le charge **sur place**, sur le périphérique que
//! `RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE` **nomme** (`gpu:N`, `igpu:N`, `cpu`), et le dit ;
//! sans périphérique nommé, ou le repli interdit (`non`), le test est **rouge** et nomme le
//! modèle. Jamais sauté, jamais deviné (décision de c7, 15 novembre 2026).
//!
//! Chaque cas se joue dans un processus à lui (le binaire relancé sur la sonde), pour que
//! la variable et le chargement unique (`LazyLock`) ne débordent pas sur les autres suites.
//!
//! ./run_e2e.sh --test e2e_aide_des_tests_repli
#![cfg(all(feature = "burn-embedder", feature = "daemon"))]

/// La sonde, jouée seule dans le processus relancé : granite-107m par l'aide, puis un calcul.
#[test]
#[ignore]
fn sonde_granite_107m() {
    // Judged in its relaunch only (`RAG3WEAVER_VALUES_PROBE`, set by `sonde_sous`) : played by a
    // pass where no service serves granite-107m and no fallback is named (the tablet, 20
    // November 2026), it was red with no defect of the code.
    if std::env::var_os("RAG3WEAVER_VALUES_PROBE").is_none() {
        println!("▸ sonde : hors relance, rien à juger");
        return;
    }
    let e = &*crate::common::burn::GRANITE_107M;
    let v = e.embed(&["une phrase d'essai".into()]).expect("le calcul de la sonde");
    println!("▸ sonde : dim {} factice {}", v[0].len(), e.is_mock());
}

/// The values probe : a test that compares values declares it, then computes. It judges
/// nothing outside its relaunch (`RAG3WEAVER_VALUES_PROBE`) : played by a pass under the
/// fallback `igpu:0`, it would be refused, which is what the relaunch checks.
#[test]
#[ignore]
fn values_probe_granite_107m() {
    if std::env::var_os("RAG3WEAVER_VALUES_PROBE").is_none() {
        println!("▸ sonde de valeurs : hors relance, rien à juger");
        return;
    }
    crate::common::burn::compares_values("granite-107m");
    let e = &*crate::common::burn::GRANITE_107M;
    let v = e.embed(&["une phrase d'essai".into()]).expect("le calcul de la sonde");
    println!("▸ sonde de valeurs : dim {}", v[0].len());
}

/// The binary relaunched on the values probe, no service serving the model, the fallback on
/// `igpu:0`, and the variable that allows an integrated GPU given or not.
fn values_probe_under(allow: Option<&str>) -> (bool, String) {
    let mut c = std::process::Command::new(std::env::current_exe().unwrap());
    c.args(["--exact", &crate::common::test_de_cette_suite!("values_probe_granite_107m"), "--ignored", "--nocapture", "--test-threads=1"])
        .env("RAG3WEAVER_SERVICE_EMBED", "127.0.0.1:9")
        .env_remove("RAG3WEAVER_EMBED_SERVICE")
        .env("RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE", "igpu:0")
        .env("RAG3WEAVER_VALUES_PROBE", "1")
        .env_remove("RAG3WEAVER_EMBEDDER_ALLOW_INTEGRATED_GPU");
    if let Some(a) = allow {
        c.env("RAG3WEAVER_EMBEDDER_ALLOW_INTEGRATED_GPU", a);
    }
    let out = c.output().expect("relancer le binaire sur la sonde de valeurs");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

/// **A test that compares values refuses the fallback on an integrated GPU, and names the
/// ticket** (20 November 2026, ticket `2026-11-20-sur-l-igpu-granite-rend-des-vecteurs-faux-en-lot`) ;
/// `RAG3WEAVER_EMBEDDER_ALLOW_INTEGRATED_GPU=1` lets it through. A test of paths keeps the
/// fallback (A1 above).
#[test]
#[ignore]
fn a_values_test_refuses_the_fallback_on_an_integrated_gpu() {
    let (ok, text) = values_probe_under(None);
    assert!(!ok, "the values probe is red on the fallback igpu:0 : {text}");
    assert!(text.contains("ce test compare des valeurs") && text.contains("2026-11-20-sur-l-igpu-granite-rend-des-vecteurs-faux-en-lot"),
        "the red says why and names the ticket : {text}");
    let (ok, text) = values_probe_under(Some("1"));
    assert!(ok && text.contains("▸ sonde de valeurs : dim 384"), "allowed, the probe computes : {text}");
}

/// Le binaire relancé sur la sonde, sous une variable de service qui ne sert rien (un port
/// où personne n'écoute) et la variable de repli donnée.
fn sonde_sous(repli: Option<&str>) -> (bool, String) {
    let mut c = std::process::Command::new(std::env::current_exe().unwrap());
    c.args(["--exact", &crate::common::test_de_cette_suite!("sonde_granite_107m"), "--ignored", "--nocapture", "--test-threads=1"])
        .env("RAG3WEAVER_SERVICE_EMBED", "127.0.0.1:9")
        .env("RAG3WEAVER_VALUES_PROBE", "1")
        .env_remove("RAG3WEAVER_EMBED_SERVICE")
        .env_remove("RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE");
    if let Some(r) = repli {
        c.env("RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE", r);
    }
    let sortie = c.output().expect("relancer le binaire sur la sonde");
    let texte = format!("{}{}", String::from_utf8_lossy(&sortie.stdout), String::from_utf8_lossy(&sortie.stderr));
    (sortie.status.success(), texte)
}

/// **Aucun service ne sert le modèle, un périphérique de repli nommé : il s'y charge sur place,
/// et l'aide le dit** (A1). À luciepc, `igpu:0` est l'Intel ARL, qui ne porte aucun écran
/// (relevé du 15 novembre, 00 h 2x : ses quatre sorties « disconnected ») ; jamais card0
/// (l'affichage de Lucie) ni card2 (les services).
#[test]
#[ignore]
fn sans_service_qui_le_sert_le_modele_se_charge_sur_le_peripherique_nomme_et_le_dit() {
    let (ok, texte) = sonde_sous(Some("igpu:0"));
    assert!(ok, "la sonde passe, le modèle chargé sur l'iGPU : {texte}");
    assert!(texte.contains("granite-107m sur place (IntegratedGpu(0))"), "l'aide dit le repli et son périphérique : {texte}");
    assert!(texte.contains("▸ sonde : dim 384 factice false"), "le vrai modèle, pas un factice : {texte}");
}

/// **Aucun service ne sert le modèle, aucun périphérique de repli nommé : rouge, nommé** (A2,
/// décision de c7 du 15 novembre : pas de défaut CPU qui n'existe pas — le `Cpu` de burn passe
/// par Vulkan, et aucun des deux postes n'a d'adaptateur Vulkan CPU).
#[test]
#[ignore]
fn sans_service_ni_peripherique_de_repli_le_test_est_rouge_et_le_dit() {
    let (ok, texte) = sonde_sous(None);
    assert!(!ok, "sans repli nommé, la sonde rougit : {texte}");
    assert!(texte.contains("pas de service pour granite-107m") && texte.contains("aucun périphérique de repli nommé"),
        "le rouge nomme le modèle et le repli manquant : {texte}");
}

/// **Le repli interdit : le test est rouge, et nomme le modèle sans service.**
#[test]
#[ignore]
fn repli_interdit_le_test_est_rouge_et_nomme_le_modele() {
    let (ok, texte) = sonde_sous(Some("non"));
    assert!(!ok, "le repli interdit, la sonde rougit : {texte}");
    assert!(texte.contains("pas de service pour granite-107m"), "le rouge nomme le modèle : {texte}");
}

/// **A suite that is not local is not recognised as local** (20 November 2026).
#[test]
#[ignore]
fn this_suite_is_not_recognised_as_local() {
    assert!(!crate::common::burn::suite_locale(), "e2e_aide_des_tests_repli is not a local suite (test {:?})", std::thread::current().name());
}
