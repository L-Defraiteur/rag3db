//! **Une commande `!` lancée là où tourne le terminal** (lot tty-13), dans un
//! pseudo-terminal à elle : `sudo` y demande son mot de passe, un outil
//! interactif y répond au clavier. Le terminal relaie les touches vers elle et sa
//! sortie vers l'écran, et garde une copie de la sortie seule.
//!
//! Ce qui se tape n'est jamais gardé, seulement ce que la commande écrit : `sudo`
//! coupe l'écho pendant qu'il lit le mot de passe, qui n'entre donc ni dans la
//! sortie gardée, ni dans la consigne envoyée à l'équipier, ni dans le journal.
//!
//! Unix seulement (`posix_openpt`, par la libc que la bibliothèque standard lie
//! déjà : aucune dépendance de plus). Sous Windows, la commande refuse et le dit.

use std::io::Write;
use std::path::Path;

/// Ce qu'a donné une commande.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resultat {
    /// Son code de sortie, `None` si un signal l'a arrêtée.
    pub code: Option<i32>,
    /// Ce qu'elle a écrit, tel quel.
    pub sortie: Vec<u8>,
}

/// Lance `cmd` par `/bin/sh -c` dans `dossier`, sur un pseudo-terminal de
/// `colonnes` × `lignes` ; relaie ce qui arrive sur le descripteur `entree` vers
/// elle, et sa sortie vers `ecran`, jusqu'à sa fin.
#[cfg(unix)]
pub fn lancer(_cmd: &str, _dossier: &Path, _colonnes: u16, _lignes: u16, _entree: i32, _ecran: &mut dyn Write) -> Result<Resultat, String> {
    Err("pas encore".into())
}

#[cfg(not(unix))]
pub fn lancer(_cmd: &str, _dossier: &Path, _colonnes: u16, _lignes: u16, _entree: i32, _ecran: &mut dyn Write) -> Result<Resultat, String> {
    Err("les commandes ! ne tournent pas encore sous Windows".into())
}

/// **La sortie pour l'équipier** : sans les séquences du terminal (couleurs,
/// curseur, titres) ni les retours chariot, au plus `max` octets, son début et sa
/// fin gardés quand elle est plus longue.
pub fn nettoyer(_sortie: &[u8], _max: usize) -> String {
    String::new()
}

/// La consigne qui part à l'équipier : ce que Lucie a lancé, où, et ce qu'en a dit la commande.
pub fn consigne(cmd: &str, dossier: &Path, r: &Resultat) -> String {
    let code = r.code.map(|c| c.to_string()).unwrap_or_else(|| "arrêtée par un signal".into());
    format!("L'utilisateur a lancé la commande `{cmd}` dans {} ; voici le retour (code {code}) :\n```\n{}\n```", dossier.display(), nettoyer(&r.sortie, 8 * 1024))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    /// Un tube dont on écrit `entree` d'avance : le bout lu sert d'entrée.
    fn tube(entree: &[u8]) -> i32 {
        let mut fds = [0i32; 2];
        assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
        let mut w = unsafe { <std::fs::File as std::os::unix::io::FromRawFd>::from_raw_fd(fds[1]) };
        w.write_all(entree).unwrap();
        drop(w);
        fds[0]
    }

    /// Une commande tourne dans le dossier donné, sa sortie s'affiche et se
    /// garde, et son code de sortie revient.
    #[test]
    fn a_command_runs_in_its_folder_shows_and_keeps_its_output_and_its_code() {
        let d = std::env::temp_dir().join(format!("tty-coquille-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let mut ecran = Vec::new();
        let e = tube(b"");
        let r = lancer("pwd; printf 'deux\nlignes\n'; exit 3", &d, 80, 24, e, &mut ecran).unwrap();
        unsafe { close(e) };
        let vu = String::from_utf8_lossy(&r.sortie);
        assert!(vu.contains(d.file_name().unwrap().to_str().unwrap()), "{vu:?}");
        assert!(vu.contains("deux\r\nlignes"), "la sortie d'un terminal : {vu:?}");
        assert_eq!(r.code, Some(3));
        assert_eq!(ecran, r.sortie, "ce qui s'affiche est ce qui se garde");
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// **La garde de c7** : un faux `sudo` coupe l'écho, demande et lit un mot de
    /// passe ; le secret ne paraît ni dans la sortie gardée, ni dans la consigne.
    #[test]
    fn a_password_read_with_echo_off_never_reaches_the_kept_output() {
        let faux_sudo = "stty -echo; printf 'Mot de passe : '; read p; stty echo; echo; echo \"lu ${#p} caractères\"";
        let mut ecran = Vec::new();
        let e = tube(b"s3cr3t-tres-long\n");
        let r = lancer(faux_sudo, Path::new("/"), 80, 24, e, &mut ecran).unwrap();
        unsafe { close(e) };
        let vu = String::from_utf8_lossy(&r.sortie);
        assert!(vu.contains("Mot de passe :") && vu.contains("lu 16 caractères"), "{vu:?}");
        assert!(!vu.contains("s3cr3t"), "le mot de passe dans la sortie : {vu:?}");
        let c = consigne(faux_sudo, Path::new("/"), &r);
        assert!(!c.contains("s3cr3t") && c.contains("lu 16 caractères") && c.contains("(code 0)"), "{c}");
    }

    /// La sortie pour l'équipier : sans couleurs, curseur ni retours chariot, et
    /// coupée au milieu quand elle est trop longue.
    #[test]
    fn the_output_for_the_teammate_drops_terminal_sequences_and_is_cut_in_the_middle() {
        assert_eq!(nettoyer(b"\x1b[31mrouge\x1b[0m\r\nfin\x1b]0;titre\x07\r\n", 1000), "rouge\nfin");
        let long: Vec<u8> = (0..1000).flat_map(|i| format!("ligne {i}\n").into_bytes()).collect();
        let n = nettoyer(&long, 200);
        assert!(n.starts_with("ligne 0\n") && n.ends_with("ligne 999"), "{n}");
        assert!(n.contains("… (") && n.len() < 300, "{n}");
    }
}
