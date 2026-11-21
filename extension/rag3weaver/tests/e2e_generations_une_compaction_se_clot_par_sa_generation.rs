//! **Une compaction se clôt par sa propre génération** (lucivy 4.4, (A)). Une compaction ne crée pas de version de la
//! base : elle commite sous la génération courante, et `release_below(g)` ne relâche jamais `g`. Sans génération à
//! elle, l'ancien index entier resterait retenu par le méta de `g` jusqu'au paquet suivant d'une synchronisation : la
//! place du plein texte doublerait après chaque maintenance de nuit (mesuré sur le dépôt : 1 015 → 2 015 Mo). Close
//! par sa génération, puis relâchée, elle rend la place tout de suite.
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use crate::e2e_generations_commun as g;

#[test]
#[ignore]
fn a_compaction_closes_with_its_own_generation_and_gives_the_space_back() {
    let d = g::case_dir("compaction");
    let base = d.join("base.rag3db");
    let (statut, sortie) = g::launch("premier", &base, &[]);
    assert!(statut.success(), "{sortie}");
    let avant = g::generations(&base);

    let (statut, sortie) = g::launch("mainteneur", &base, &[]);
    assert!(statut.success(), "la maintenance avec compaction va au bout :\n{sortie}");
    let ligne = sortie.lines().find_map(|l| l.strip_prefix("TAILLE ")).unwrap_or_else(|| panic!("pas de dir_size :\n{sortie}"));
    let nombre = |k: &str| ligne.split_whitespace().find_map(|p| p.strip_prefix(k)).and_then(|v| v.parse::<u64>().ok()).unwrap();
    let (taille_avant, taille_apres) = (nombre("avant="), nombre("apres="));
    println!("▸ plein texte : {taille_avant} octets avant la compaction, {taille_apres} après ; générations {avant:?} → {:?}", g::generations(&base));
    assert!(
        taille_apres * 10 <= taille_avant * 13,
        "après la compaction, la place revient : {taille_avant} → {taille_apres} octets (l'ancien index retenu ?)"
    );
    let apres = g::generations(&base);
    assert!(apres.iter().all(|(_, en_base, disque)| en_base == disque), "le case_dir et la base d'accord : {apres:?}");
    let _ = std::fs::remove_dir_all(&d);
}
