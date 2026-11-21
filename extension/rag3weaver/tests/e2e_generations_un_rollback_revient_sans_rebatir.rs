//! **Un paquet défait, puis la réouverture : sans rebâti** (lucivy 4.4, (A)). Un paquet échoue au paquet 3 (deux
//! paquets par validation) : ROLLBACK des lignes, catalogue empoisonné, à rouvrir. Le plein texte des paquets défaits a
//! été synchronisé et marqué : le dossier est au-dessus de la base. La réouverture le ramène à la génération de la base
//! au lieu de le jeter (la réouverture elle-même reste : d'autres caches gardent des écritures défaites).
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use crate::e2e_generations_commun as g;

#[test]
#[ignore]
fn a_rolled_back_packet_reopens_without_a_rebuild() {
    let d = g::dossier("rollback");
    let base = d.join("base.rag3db");
    let (statut, sortie) = g::lancer("echoueur", &base, &[("RAG3WEAVER_TEST_FAIL_IN_BATCH", "3")]);
    assert!(statut.success(), "le paquet échoue, le ROLLBACK se fait, le fils finit :\n{sortie}");
    assert!(sortie.contains("ECHEC "), "le paquet a échoué :\n{sortie}");
    let avant = g::generations(&base);
    assert!(
        avant.iter().any(|(_, en_base, disque)| disque > en_base),
        "le plein texte défait est au-dessus de la base : {avant:?}"
    );

    let (statut, sortie) = g::lancer("repreneur", &base, &[]);
    assert!(statut.success(), "{sortie}");
    assert!(!sortie.contains(g::REBATI), "après un ROLLBACK, la réouverture revient sans rebâti :\n{sortie}");
    assert_eq!(g::comptes("repreneur", &sortie), g::temoin("rollback-temoin"), "les comptes d'une passe sans échec");
    let _ = std::fs::remove_dir_all(&d);
}
