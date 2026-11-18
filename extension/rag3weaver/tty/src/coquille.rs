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
pub fn lancer(cmd: &str, dossier: &Path, colonnes: u16, lignes: u16, entree: i32, ecran: &mut dyn Write) -> Result<Resultat, String> {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;
    use std::os::unix::process::CommandExt;
    let maitre = unsafe { pty::posix_openpt(pty::O_RDWR | pty::O_NOCTTY) };
    if maitre < 0 || unsafe { pty::grantpt(maitre) } != 0 || unsafe { pty::unlockpt(maitre) } != 0 {
        return Err(format!("le pseudo-terminal ne s'ouvre pas : {}", std::io::Error::last_os_error()));
    }
    let nom = unsafe { std::ffi::CStr::from_ptr(pty::ptsname(maitre)) }.to_string_lossy().into_owned();
    let taille = pty::Taille { lignes, colonnes, x: 0, y: 0 };
    unsafe { pty::ioctl(maitre, pty::TIOCSWINSZ, &taille as *const pty::Taille) };
    let esclave = std::fs::OpenOptions::new().read(true).write(true).open(&nom).map_err(|e| format!("{nom} : {e}"))?;
    let mut fils = unsafe {
        std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(dossier)
            .stdin(esclave.try_clone().map_err(|e| e.to_string())?)
            .stdout(esclave.try_clone().map_err(|e| e.to_string())?)
            .stderr(esclave)
            // Une session à elle, dont le pseudo-terminal est le terminal de contrôle :
            // c'est lui que sudo ouvre pour lire le mot de passe.
            .pre_exec(|| {
                if pty::setsid() < 0 || pty::ioctl(0, pty::TIOCSCTTY, 0usize) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            })
            .spawn()
            .map_err(|e| format!("la commande ne part pas : {e}"))?
    };
    let mut m = unsafe { std::fs::File::from_raw_fd(maitre) };
    let mut sortie = Vec::new();
    let mut tampon = [0u8; 8192];
    let mut code = None;
    let mut fini = false;
    loop {
        let mut fds = [pty::PollFd { fd: entree, events: pty::POLLIN, revents: 0 }, pty::PollFd { fd: maitre, events: pty::POLLIN, revents: 0 }];
        let n = unsafe { pty::poll(fds.as_mut_ptr(), 2, if fini { 50 } else { 100 }) };
        if n > 0 && fds[1].revents & pty::POLLIN != 0 {
            match m.read(&mut tampon) {
                Ok(0) | Err(_) => {
                    if fini {
                        break;
                    }
                }
                Ok(k) => {
                    sortie.extend_from_slice(&tampon[..k]);
                    let _ = ecran.write_all(&tampon[..k]);
                    let _ = ecran.flush();
                    continue;
                }
            }
        }
        if n > 0 && fds[0].revents & pty::POLLIN != 0 {
            let k = unsafe { pty::read(entree, tampon.as_mut_ptr(), tampon.len()) };
            if k > 0 {
                let _ = m.write_all(&tampon[..k as usize]);
            }
        }
        if fini && (n == 0 || fds[1].revents & (pty::POLLHUP | pty::POLLERR) != 0) {
            break;
        }
        if !fini {
            if let Some(statut) = fils.try_wait().map_err(|e| e.to_string())? {
                code = statut.code();
                fini = true;
            }
        }
    }
    Ok(Resultat { code, sortie })
}

/// Les appels de la libc : le pseudo-terminal, la session, l'attente.
#[cfg(unix)]
mod pty {
    use std::os::raw::{c_char, c_int, c_short, c_ulong};

    #[repr(C)]
    pub struct Taille {
        pub lignes: u16,
        pub colonnes: u16,
        pub x: u16,
        pub y: u16,
    }

    #[repr(C)]
    pub struct PollFd {
        pub fd: c_int,
        pub events: c_short,
        pub revents: c_short,
    }

    pub const O_RDWR: c_int = 2;
    #[cfg(target_os = "linux")]
    pub const O_NOCTTY: c_int = 0o400;
    #[cfg(not(target_os = "linux"))]
    pub const O_NOCTTY: c_int = 0x20000;
    #[cfg(target_os = "linux")]
    pub const TIOCSWINSZ: c_ulong = 0x5414;
    #[cfg(not(target_os = "linux"))]
    pub const TIOCSWINSZ: c_ulong = 0x8008_7467;
    #[cfg(target_os = "linux")]
    pub const TIOCSCTTY: c_ulong = 0x540E;
    #[cfg(not(target_os = "linux"))]
    pub const TIOCSCTTY: c_ulong = 0x2000_7461;
    pub const POLLIN: c_short = 0x1;
    pub const POLLERR: c_short = 0x8;
    pub const POLLHUP: c_short = 0x10;

    extern "C" {
        pub fn posix_openpt(flags: c_int) -> c_int;
        pub fn grantpt(fd: c_int) -> c_int;
        pub fn unlockpt(fd: c_int) -> c_int;
        pub fn ptsname(fd: c_int) -> *mut c_char;
        pub fn ioctl(fd: c_int, requete: c_ulong, ...) -> c_int;
        pub fn setsid() -> c_int;
        pub fn poll(fds: *mut PollFd, n: c_ulong, attente: c_int) -> c_int;
        pub fn read(fd: c_int, tampon: *mut u8, n: usize) -> isize;
    }
}

#[cfg(not(unix))]
pub fn lancer(_cmd: &str, _dossier: &Path, _colonnes: u16, _lignes: u16, _entree: i32, _ecran: &mut dyn Write) -> Result<Resultat, String> {
    Err("les commandes ! ne tournent pas encore sous Windows".into())
}

/// **La sortie pour l'équipier** : sans les séquences du terminal (couleurs,
/// curseur, titres) ni les retours chariot, au plus `max` octets, son début et sa
/// fin gardés quand elle est plus longue.
pub fn nettoyer(sortie: &[u8], max: usize) -> String {
    let texte = String::from_utf8_lossy(sortie);
    let mut out = String::new();
    let mut c = texte.chars().peekable();
    while let Some(x) = c.next() {
        match x {
            // CSI (ESC [ … lettre), OSC (ESC ] … BEL ou ESC \), et les autres ESC à un caractère.
            '\x1b' => match c.next() {
                Some('[') => {
                    for y in c.by_ref() {
                        if ('@'..='~').contains(&y) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    while let Some(y) = c.next() {
                        if y == '\x07' || (y == '\x1b' && c.peek() == Some(&'\\')) {
                            if y == '\x1b' {
                                c.next();
                            }
                            break;
                        }
                    }
                }
                _ => {}
            },
            '\r' | '\x07' => {}
            autre => out.push(autre),
        }
    }
    let out = out.trim_end().to_string();
    if out.len() <= max {
        return out;
    }
    let mut debut = max / 2;
    while !out.is_char_boundary(debut) {
        debut -= 1;
    }
    let mut fin = out.len() - max / 2;
    while !out.is_char_boundary(fin) {
        fin += 1;
    }
    format!("{}\n… ({} octets coupés) …\n{}", &out[..debut], fin - debut, &out[fin..])
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

    /// Un tube dont on écrit `entree` après `apres` millisecondes, comme une
    /// frappe venue après l'invite : le bout lu sert d'entrée.
    fn tube(entree: &[u8], apres: u64) -> i32 {
        let mut fds = [0i32; 2];
        assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
        let mut w = unsafe { <std::fs::File as std::os::unix::io::FromRawFd>::from_raw_fd(fds[1]) };
        let entree = entree.to_vec();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(apres));
            w.write_all(&entree).unwrap();
        });
        fds[0]
    }

    /// Une commande tourne dans le dossier donné, sa sortie s'affiche et se
    /// garde, et son code de sortie revient.
    #[test]
    fn a_command_runs_in_its_folder_shows_and_keeps_its_output_and_its_code() {
        let d = std::env::temp_dir().join(format!("tty-coquille-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let mut ecran = Vec::new();
        let e = tube(b"", 0);
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
        // La frappe vient après l'invite, quand l'écho est coupé (une frappe tapée
        // d'avance, avant l'invite, serait renvoyée à l'écran, comme par tout terminal).
        let e = tube(b"s3cr3t-tres-long\n", 500);
        let r = lancer(faux_sudo, Path::new("/"), 80, 24, e, &mut ecran).unwrap();
        unsafe { close(e) };
        let vu = String::from_utf8_lossy(&r.sortie);
        assert!(vu.contains("Mot de passe :") && vu.contains("lu 16 caractères"), "{vu:?}");
        assert!(!vu.contains("s3cr3t"), "le mot de passe dans la sortie : {vu:?}");
        let c = consigne(faux_sudo, Path::new("/"), &r);
        assert!(!c.contains("s3cr3t") && c.contains("lu 16 caractères") && c.contains("(code 0)"), "{c}");
    }

    /// **A password typed before the prompt never reaches the kept output** (ticket of 15 nov.): typed while
    /// the echo is still on, the pty echoes it to the screen like any terminal, but what is typed is never
    /// kept; the command still reads it, and its own output is kept.
    #[test]
    fn a_password_typed_before_the_prompt_never_reaches_the_kept_output() {
        let fake_sudo = "sleep 0.3; stty -echo; printf 'Mot de passe : '; read p; stty echo; echo; echo \"lu ${#p} caractères\"";
        let mut screen = Vec::new();
        let e = tube(b"s3cr3t-tres-long\n", 0);
        let r = lancer(fake_sudo, Path::new("/"), 80, 24, e, &mut screen).unwrap();
        unsafe { close(e) };
        let kept = String::from_utf8_lossy(&r.sortie);
        assert!(kept.contains("Mot de passe :") && kept.contains("lu 16 caractères"), "the command read it: {kept:?}");
        assert!(!kept.contains("s3cr3t"), "the password typed early is in the kept output: {kept:?}");
        let c = consigne(fake_sudo, Path::new("/"), &r);
        assert!(!c.contains("s3cr3t"), "{c}");
    }

    /// What is typed while the echo is on is shown, not kept; the command's own output is kept whole.
    #[test]
    fn what_is_typed_is_shown_but_not_kept() {
        let mut screen = Vec::new();
        let e = tube(b"oui\n", 200);
        let r = lancer("printf 'Continuer ? '; read a; echo \"réponse : $a\"", Path::new("/"), 80, 24, e, &mut screen).unwrap();
        unsafe { close(e) };
        let kept = String::from_utf8_lossy(&r.sortie).replace('\r', "");
        let shown = String::from_utf8_lossy(&screen).replace('\r', "");
        assert!(shown.contains("Continuer ? oui\n"), "the echo is on the screen: {shown:?}");
        assert!(kept.contains("Continuer ? ") && kept.contains("réponse : oui"), "{kept:?}");
        assert!(!kept.contains("Continuer ? oui"), "the typed answer is not kept: {kept:?}");
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
