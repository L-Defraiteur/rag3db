//! **A local install in the task's tree is a local write** (c7, 22 Nov 2026, the terminal's night: « crée-moi un projet
//! three.js avec webpack » must build without a card). It downloads, but sends nothing of the user out of the machine:
//! class `write_local`, by rule, without the judge. A global install, `sudo`, `cargo install` and a publication stay
//! judged. A module of its own.

use super::rule_class;

#[test]
fn a_local_install_is_a_local_write() {
    let d = tempfile::tempdir().unwrap();
    for ligne in ["npm install", "npm install three webpack", "npm i -D typescript", "pnpm add react", "yarn add vite", "npm ci", "npx create-vite@latest app", "cargo add serde", "cargo build"] {
        assert_eq!(rule_class(ligne, d.path()), "write_local", "`{ligne}`");
    }
    std::fs::create_dir_all(d.path().join(".venv/bin")).unwrap();
    assert_eq!(rule_class("pip install requests", d.path()), "write_local", "pip with the project's venv");
}

#[test]
fn a_global_install_and_a_publication_stay_judged() {
    let d = tempfile::tempdir().unwrap();
    for ligne in ["npm install -g typescript", "npm i --global vite", "cargo install ripgrep", "sudo npm install", "npm publish", "pip install requests"] {
        assert_ne!(rule_class(ligne, d.path()), "write_local", "`{ligne}`");
        assert_ne!(rule_class(ligne, d.path()), "read", "`{ligne}`");
    }
}

#[test]
fn an_install_says_what_it_installed_in_one_line() {
    let s = super::install_summary("npm install three webpack", "npm warn deprecated x\n\nadded 412 packages, and audited 413 packages in 9s\n\nfound 0 vulnerabilities\n");
    assert_eq!(s.as_deref(), Some("npm install : added 412 packages, and audited 413 packages in 9s"));
    assert_eq!(super::install_summary("ls -la", "total 0"), None);
}
