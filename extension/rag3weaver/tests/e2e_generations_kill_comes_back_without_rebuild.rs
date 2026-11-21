//! **Le cas 810, à deux générations d'avance : la reprise revient, elle ne rebâtit pas** (lucivy 4.4, (A)). Un écrivain
//! meurt par SIGKILL au paquet 3, deux paquets par validation : les paquets 2 et 3 ont synchronisé et marqué leur plein
//! texte, la base ne les a pas validés. Le case_dir est au moins deux générations au-dessus de la base (k ≥ 2 : une
//! lucivy qui ne garderait que la génération d'avant passerait un témoin à k = 1). Le repreneur ne jette pas le
//! case_dir : il le ramène à la génération de la base, et finit aux counts d'une passe sans arrêt.
//!
//! Il dit aussi le temps de la reprise (`REPRISE_MS`), à comparer avant et après ; `GEN_GROS=1` pour le corpus de
//! 100 000 scopes.
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use crate::e2e_generations_common as g;

#[test]
#[ignore]
fn a_kill_two_generations_ahead_comes_back_without_a_rebuild() {
    let d = g::case_dir("arret-deux-generations");
    let base = d.join("base.rag3db");
    let (statut, sortie) = g::launch("ecrivain", &base, &[("RAG3WEAVER_TEST_KILL_IN_BATCH", g::killed_packet())]);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "mort par SIGKILL au paquet {} :\n{sortie}", g::killed_packet());
    let avant = g::generations(&base);
    assert!(!avant.is_empty(), "des index au plein texte marqués avant la mort");
    assert!(
        avant.iter().any(|(_, en_base, disque)| *disque >= en_base + if g::measuring() { 1 } else { 2 }),
        "au moins un case_dir deux générations au-dessus de la base (k ≥ 2) : {avant:?}"
    );

    let (statut, sortie) = g::launch("repreneur", &base, &[]);
    assert!(statut.success(), "la base rouvre, et la reprise va au bout :\n{sortie}");
    let ms = g::resume_ms("repreneur", &sortie);
    let remise = sortie.lines().find_map(|l| l.strip_prefix("REMISE_MS repreneur ")).unwrap_or("?").trim().to_string();
    println!("▸ remise en état du plein texte en {remise} ms ; reprise entière (resynchronisation comprise) en {ms} ms ; générations avant la reprise : {avant:?}");
    assert!(!sortie.contains(g::REBUILT), "un case_dir en avance revient à la génération de la base, sans rebâti :\n{sortie}");
    let apres = g::generations(&base);
    assert!(apres.iter().all(|(_, en_base, disque)| en_base == disque), "le case_dir et la base d'accord après la reprise : {apres:?}");
    let repris = g::counts("repreneur", &sortie);
    assert_eq!(repris, g::control_counts("arret-deux-generations-temoin"), "les counts d'une passe sans arrêt, plein texte compris");
    let _ = std::fs::remove_dir_all(&d);
}
