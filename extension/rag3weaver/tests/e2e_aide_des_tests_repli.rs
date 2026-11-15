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
    let e = &*crate::common::burn::GRANITE_107M;
    let v = e.embed(&["une phrase d'essai".into()]).expect("le calcul de la sonde");
    println!("▸ sonde : dim {} factice {}", v[0].len(), e.is_mock());
}

/// Le binaire relancé sur la sonde, sous une variable de service qui ne sert rien (un port
/// où personne n'écoute) et la variable de repli donnée.
fn sonde_sous(repli: Option<&str>) -> (bool, String) {
    let mut c = std::process::Command::new(std::env::current_exe().unwrap());
    c.args(["--exact", &crate::common::test_de_cette_suite!("sonde_granite_107m"), "--ignored", "--nocapture", "--test-threads=1"])
        .env("RAG3WEAVER_SERVICE_EMBED", "127.0.0.1:9")
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

/// **Le repli sur le CPU, là où un adaptateur Vulkan CPU existe** (A3) : à luciepc depuis le
/// 15 novembre (vulkan-swrast installé par Lucie, llvmpipe), `cpu` charge le vrai modèle sur place.
/// Le portable n'a pas encore le paquet : la sonde y rougirait sur « No CPU device found ».
#[test]
#[ignore]
fn sans_service_qui_le_sert_le_modele_se_charge_sur_le_cpu_ou_il_existe() {
    let (ok, texte) = sonde_sous(Some("cpu"));
    assert!(ok, "la sonde passe, le modèle chargé sur le CPU : {texte}");
    assert!(texte.contains("granite-107m sur place (Cpu)") && texte.contains("▸ sonde : dim 384 factice false"), "{texte}");
}

/// **Le repli interdit : le test est rouge, et nomme le modèle sans service.**
#[test]
#[ignore]
fn repli_interdit_le_test_est_rouge_et_nomme_le_modele() {
    let (ok, texte) = sonde_sous(Some("non"));
    assert!(!ok, "le repli interdit, la sonde rougit : {texte}");
    assert!(texte.contains("pas de service pour granite-107m"), "le rouge nomme le modèle : {texte}");
}
