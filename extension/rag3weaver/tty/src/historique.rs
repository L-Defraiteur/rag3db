//! **L'historique des envois, d'une séance à l'autre** (lot tty-13) : comme celui
//! d'un shell, ↑ y retrouve ce qui a été envoyé lors des séances d'avant.
//!
//! Un fichier à soi, `<dossier d'état>/historique.txt` : un envoi par ligne, en
//! chaîne JSON (un envoi sur plusieurs lignes y tient sur une seule), les
//! [`GARDE`] derniers au plus, lisible par soi seul sous Unix (`600`).
//! `/effacer historique` le supprime.

use std::path::{Path, PathBuf};

/// Combien d'envois on garde.
pub const GARDE: usize = 1000;

/// Le fichier de l'historique, dans le dossier d'état du terminal.
pub fn chemin() -> Option<PathBuf> {
    crate::journal::dossier_d_etat().map(|d| d.join("historique.txt"))
}

/// Les envois gardés, du plus ancien au plus récent ; rien si le fichier manque.
pub fn charger(_chemin: &Path) -> Vec<String> {
    Vec::new()
}

/// Ajoute un envoi (un envoi vide ou pareil au dernier ne s'ajoute pas), et
/// réécrit le fichier quand il dépasse deux fois [`GARDE`].
pub fn ajouter(_chemin: &Path, _envoi: &str) -> Result<(), String> {
    Ok(())
}

/// Supprime l'historique.
pub fn effacer(_chemin: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dossier(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tty-historique-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// Un envoi se garde d'une séance à l'autre, sur plusieurs lignes compris,
    /// sans doublon de suite ni envoi vide, et l'effacer le supprime.
    #[test]
    fn sent_lines_survive_the_session_multi_line_included_and_can_be_erased() {
        let d = dossier("a");
        let f = d.join("sous").join("historique.txt");
        assert_eq!(charger(&f), Vec::<String>::new(), "rien avant le premier envoi");
        for e in ["un", "deux\ntrois", "deux\ntrois", "  ", "quatre « é »"] {
            ajouter(&f, e).unwrap();
        }
        assert_eq!(charger(&f), vec!["un", "deux\ntrois", "quatre « é »"]);
        assert_eq!(std::fs::read_to_string(&f).unwrap().lines().count(), 3, "un envoi par ligne");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&f).unwrap().permissions().mode() & 0o777, 0o600, "lisible par soi seul");
        }
        effacer(&f).unwrap();
        assert_eq!(charger(&f), Vec::<String>::new());
        effacer(&f).unwrap();
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Au-delà de deux fois la garde, le fichier se réécrit avec les derniers
    /// envois seuls ; la lecture ne rend jamais plus que la garde.
    #[test]
    fn the_file_keeps_only_the_last_entries() {
        let d = dossier("b");
        let f = d.join("historique.txt");
        for i in 0..(2 * GARDE + 5) {
            ajouter(&f, &format!("envoi {i}")).unwrap();
        }
        let h = charger(&f);
        assert_eq!(h.len(), GARDE);
        assert_eq!(h.last().map(String::as_str), Some(format!("envoi {}", 2 * GARDE + 4).as_str()));
        assert!(std::fs::read_to_string(&f).unwrap().lines().count() <= 2 * GARDE);
        std::fs::write(&f, "\"bon\"\npas du json\n\"aussi bon\"\n").unwrap();
        assert_eq!(charger(&f), vec!["bon", "aussi bon"], "une ligne illisible est sautée");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
