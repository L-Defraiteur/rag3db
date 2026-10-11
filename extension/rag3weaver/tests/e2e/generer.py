#!/usr/bin/env python3
"""Génère `tests/e2e/main.rs` : le binaire UNIQUE des suites e2e.

Chaque `tests/e2e_*.rs` y est monté comme module par `#[path]` — le fichier
ne bouge pas, ses `include_str!` relatifs et son historique non plus. À
rejouer après l'ajout ou le retrait d'une suite (run_e2e.sh refuse de partir
si une suite manque, et le test `chaque_suite_est_montee` le vérifie aussi).

Usage : python3 tests/e2e/generer.py   (depuis extension/rag3weaver)
"""
from pathlib import Path

CRATE = Path(__file__).resolve().parents[2]
TESTS = CRATE / "tests"

suites = sorted(p.stem for p in TESTS.glob("e2e_*.rs"))

entete = '''//! **Le binaire unique des suites e2e** (11 octobre 2026, demande de Lucie :
//! « bâtis une seule fois avec le même cœur, pas instanciés 50 fois »).
//!
//! Chaque suite reste un fichier `tests/e2e_*.rs`, monté ici comme module :
//! le crate de test se compile et se relie UNE fois au lieu d'une fois par
//! suite. `run_e2e.sh` lance ce binaire UNE FOIS PAR SUITE, filtré sur
//! `<suite>::` — chaque suite garde donc son processus à elle (ses
//! `set_var`, ses `OnceLock`, ses statics, son démon), comme avant.
//!
//! Lancé à la main sans filtre (`cargo test --test e2e`), il joue TOUTES les
//! suites dans un seul processus : l'isolation entre suites n'y est plus
//! garantie. Passer par `run_e2e.sh`, ou filtrer : `cargo test --test e2e
//! e2e_code::`.
//!
//! **Fichier généré** par `tests/e2e/generer.py` — ne pas l'éditer à la main.

#[allow(dead_code, unused_imports)]
#[path = "../common/mod.rs"]
mod common;

'''

mods = "".join(f'#[path = "../{s}.rs"]\nmod {s};\n' for s in suites)

filet = '''
/// **Chaque suite est montée.** Sans ce filet, une suite ajoutée sous
/// `tests/` sans être montée ici ne serait plus bâtie du tout (`autotests =
/// false`) — un saut en silence, le pire des défauts d'une batterie.
#[test]
fn chaque_suite_est_montee() {
    let ici = include_str!("main.rs");
    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut manquantes = Vec::new();
    for e in std::fs::read_dir(&dossier).expect("tests/ lisible").flatten() {
        let nom = e.file_name().to_string_lossy().to_string();
        if nom.starts_with("e2e_") && nom.ends_with(".rs") {
            if !ici.contains(&format!("#[path = \\"../{nom}\\"]")) {
                manquantes.push(nom);
            }
        }
    }
    assert!(
        manquantes.is_empty(),
        "suites non montées dans tests/e2e/main.rs : {manquantes:?} — \\
         rejouer `python3 tests/e2e/generer.py`"
    );
}
'''

filet += '''
/// **Aucune suite ne se relance par le nom court d'un de ses tests.** Dans ce
/// binaire, un test s'appelle `<suite>::<test>` : un fils lancé par
/// `current_exe()` avec `--exact <test>` ne jouerait rien, sortirait avec 0,
/// et le parent conclurait sur un fils vide — un vert vide sur les suites
/// d'arrêt brutal (relecture de rag3db-73, 11 octobre 2026). La forme juste :
/// `&crate::common::test_de_cette_suite!("role_enfant")`.
#[test]
fn aucune_suite_ne_se_relance_par_son_nom_court() {
    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut fautes = Vec::new();
    for e in std::fs::read_dir(&dossier).expect("tests/ lisible").flatten() {
        let nom = e.file_name().to_string_lossy().to_string();
        if !(nom.starts_with("e2e_") && nom.ends_with(".rs")) {
            continue;
        }
        let texte = std::fs::read_to_string(e.path()).unwrap_or_default();
        let compact: String = texte.split_whitespace().collect::<Vec<_>>().join(" ");
        if compact.contains("\\"--exact\\", \\"") {
            fautes.push(nom);
        }
    }
    assert!(
        fautes.is_empty(),
        "suites qui se relancent par un nom court (`--exact \\"test\\"`), qui ne désigne \\
         rien dans le binaire unique : {fautes:?} — employer \\
         `crate::common::test_de_cette_suite!(\\"test\\")`"
    );
}
'''
(TESTS / "e2e" / "main.rs").write_text(entete + mods + filet)
print(f"{len(suites)} suites montées dans tests/e2e/main.rs")
