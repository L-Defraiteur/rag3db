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
pub fn charger(chemin: &Path) -> Vec<String> {
    let Ok(texte) = std::fs::read_to_string(chemin) else { return Vec::new() };
    let tout: Vec<String> = texte.lines().filter_map(|l| serde_json::from_str::<String>(l).ok()).collect();
    let debut = tout.len().saturating_sub(GARDE);
    tout[debut..].to_vec()
}

/// Ajoute un envoi (un envoi vide ou pareil au dernier ne s'ajoute pas), et
/// réécrit le fichier quand il dépasse deux fois [`GARDE`].
pub fn ajouter(chemin: &Path, envoi: &str) -> Result<(), String> {
    use std::io::Write;
    if envoi.trim().is_empty() {
        return Ok(());
    }
    let lignes = std::fs::read_to_string(chemin).unwrap_or_default();
    if lignes.lines().last().and_then(|l| serde_json::from_str::<String>(l).ok()).as_deref() == Some(envoi) {
        return Ok(());
    }
    if let Some(d) = chemin.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(d).map_err(|e| format!("le dossier de l'historique {} : {e}", d.display()))?;
    }
    let ligne = serde_json::to_string(envoi).map_err(|e| e.to_string())?;
    if lignes.lines().count() >= 2 * GARDE {
        let mut garde = charger(chemin);
        garde.push(envoi.to_string());
        let debut = garde.len().saturating_sub(GARDE);
        let tout: String = garde[debut..].iter().filter_map(|e| serde_json::to_string(e).ok()).map(|l| l + "\n").collect();
        ecrire_a_soi(chemin, tout.as_bytes(), false)
    } else {
        ecrire_a_soi(chemin, format!("{ligne}\n").as_bytes(), true)
    }
    .and_then(|mut f| f.flush().map_err(|e| e.to_string()))
}

/// Ouvre le fichier en écriture (en ajout ou en le vidant), lisible par soi seul sous Unix.
fn ecrire_a_soi(chemin: &Path, octets: &[u8], ajout: bool) -> Result<std::fs::File, String> {
    use std::io::Write;
    let mut o = std::fs::OpenOptions::new();
    o.create(true).write(true);
    if ajout {
        o.append(true);
    } else {
        o.truncate(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    let mut f = o.open(chemin).map_err(|e| format!("l'historique {} : {e}", chemin.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(chemin, std::fs::Permissions::from_mode(0o600));
    }
    f.write_all(octets).map_err(|e| e.to_string())?;
    Ok(f)
}

/// Supprime l'historique.
pub fn effacer(chemin: &Path) -> Result<(), String> {
    match std::fs::remove_file(chemin) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("l'historique {} : {e}", chemin.display())),
    }
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
