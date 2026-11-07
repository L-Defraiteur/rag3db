//! **Le journal de l'hôte du démon dit l'heure et l'auteur de chaque ligne**
//! (ticket du 11 octobre 2026, « Le journal de l'hôte du démon n'a ni heure ni
//! auteur »).
//!
//! `daemon.log` est la sortie brute du processus de l'hôte (l'unité systemd ou
//! le client qui le lance y redirigent ses fd 1 et 2). Aucune façade ne passe
//! entre les quelque 270 `eprintln!` du produit et ce fichier, et la lib du
//! moteur écrit directement sur le fd 2 : la marque d'heure se pose donc **au
//! niveau du fd**. [`install_timestamps`] remplace les fd 1 et 2 de l'hôte par
//! un tube ; un fil relit chaque ligne et l'écrit sur la sortie d'origine,
//! précédée de l'heure locale ISO à la milliseconde, décalage compris
//! (`2026-11-07T17:40:12.345+01:00 [démon] …`). [`stop_timestamps`] rend les
//! fd d'origine et attend que le fil ait tout écrit : les dernières lignes de
//! l'arrêt ne se perdent pas.
//!
//! L'auteur : une ligne qui naît d'un appel d'outil nomme son appelant. La
//! route `/call` de l'hôte pose l'appelant de ce fil le temps de l'appel
//! ([`enter_call`]) ; une ligne qui veut le dire ajoute [`caller_suffix`]
//! (` ⟨mcp (pid 1234) · session s-… · backend b1⟩`). Le crochet après outil
//! le fait : une rafale de `[crochet search_code]` s'attribue au client qui
//! boucle.

use std::cell::RefCell;

thread_local! {
    /// L'appelant de l'appel en cours dans ce fil, déjà mis en forme.
    static CALLER: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Remet l'appelant d'avant en sortant de l'appel.
pub struct CallerGuard(Option<String>);

impl Drop for CallerGuard {
    fn drop(&mut self) {
        CALLER.with(|c| *c.borrow_mut() = self.0.take());
    }
}

/// Pose l'appelant de ce fil le temps d'un appel d'outil : son client
/// (`mcp (pid N)`, l'IDE…), sa session et le backend appelé.
pub fn enter_call(client: &str, session: &str, backend: &str) -> CallerGuard {
    let caller = format!("{client} · session {session} · backend {backend}");
    CallerGuard(CALLER.with(|c| c.replace(Some(caller))))
}

/// ` ⟨client · session · backend⟩` quand ce fil sert un appel, sinon rien.
pub fn caller_suffix() -> String {
    CALLER.with(|c| c.borrow().as_ref().map(|s| format!(" ⟨{s}⟩")).unwrap_or_default())
}

/// L'heure locale ISO à la milliseconde, avec son décalage :
/// `2026-11-07T17:40:12.345+01:00`.
pub fn now_iso() -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    format_iso(now.as_secs() as i64, now.subsec_millis(), local_offset_secs(now.as_secs() as i64))
}

/// La forme ISO d'un instant (secondes Unix, millisecondes) vu à un décalage
/// donné (secondes à l'est de UTC).
pub fn format_iso(secs: i64, millis: u32, offset: i64) -> String {
    let local = secs + offset;
    let (days, rem) = (local.div_euclid(86_400), local.rem_euclid(86_400));
    let (y, m, d) = civil_from_days(days);
    let sign = if offset < 0 { '-' } else { '+' };
    let off = offset.abs();
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{millis:03}{sign}{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60,
        off / 3600,
        off % 3600 / 60
    )
}

/// Le jour civil d'un nombre de jours depuis 1970-01-01 (H. Hinnant).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

#[cfg(unix)]
fn local_offset_secs(secs: i64) -> i64 {
    let t = secs as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY : localtime_r écrit dans `tm`, que nous possédons ; `t` vit le temps de l'appel.
    if unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
        return 0;
    }
    tm.tm_gmtoff as i64
}

#[cfg(not(unix))]
fn local_offset_secs(_secs: i64) -> i64 {
    0
}

/// Ce qu'il faut pour rendre les fd d'origine et attendre le fil.
#[cfg(unix)]
struct Installed {
    stdout: libc::c_int,
    stderr: libc::c_int,
    reader: std::thread::JoinHandle<()>,
}

#[cfg(unix)]
static INSTALLED: std::sync::Mutex<Option<Installed>> = std::sync::Mutex::new(None);

/// Marque d'heure en tête de chaque ligne écrite sur les fd 1 et 2 de ce
/// processus, à poser au démarrage de l'hôte (avant toute ligne). Une seconde
/// pose ne fait rien. Hors Unix, rien : les lignes partent sans heure.
#[cfg(unix)]
pub fn install_timestamps() -> Result<(), String> {
    use std::io::{BufRead, Write};
    use std::os::fd::FromRawFd;
    let mut slot = INSTALLED.lock().unwrap_or_else(|p| p.into_inner());
    if slot.is_some() {
        return Ok(());
    }
    let mut fds = [0 as libc::c_int; 2];
    // SAFETY : appels POSIX sur des fd que ce processus possède ; chaque échec se lit.
    unsafe {
        if libc::pipe(fds.as_mut_ptr()) != 0 {
            return Err(format!("tube du journal : {}", std::io::Error::last_os_error()));
        }
        let (stdout, stderr) = (libc::dup(1), libc::dup(2));
        if stdout < 0 || stderr < 0 {
            return Err(format!("dup des sorties : {}", std::io::Error::last_os_error()));
        }
        // La sortie d'origine ne passe pas aux enfants (le tube non plus).
        libc::fcntl(stdout, libc::F_SETFD, libc::FD_CLOEXEC);
        libc::fcntl(stderr, libc::F_SETFD, libc::FD_CLOEXEC);
        libc::fcntl(fds[0], libc::F_SETFD, libc::FD_CLOEXEC);
        let (read_end, write_end) = (fds[0], fds[1]);
        let reader = std::thread::Builder::new()
            .name("journal-heure".into())
            .spawn(move || {
                // SAFETY : `read_end` et la copie `out` sont à ce fil seul.
                let tube = std::io::BufReader::new(std::fs::File::from_raw_fd(read_end));
                let out_fd = libc::dup(stderr);
                let mut out = std::fs::File::from_raw_fd(out_fd);
                for ligne in tube.split(b'\n') {
                    let Ok(ligne) = ligne else { break };
                    let mut l = Vec::with_capacity(ligne.len() + 32);
                    l.extend_from_slice(now_iso().as_bytes());
                    l.push(b' ');
                    l.extend_from_slice(&ligne);
                    l.push(b'\n');
                    let _ = out.write_all(&l);
                }
            })
            .map_err(|e| format!("fil du journal : {e}"))?;
        if libc::dup2(write_end, 1) < 0 || libc::dup2(write_end, 2) < 0 {
            return Err(format!("dup2 des sorties : {}", std::io::Error::last_os_error()));
        }
        libc::close(write_end);
        *slot = Some(Installed { stdout, stderr, reader });
    }
    // Une panique qui emporte le processus : son message passe par le tube ;
    // un court délai laisse le fil l'écrire avant la sortie.
    let avant = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        avant(info);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }));
    Ok(())
}

#[cfg(not(unix))]
pub fn install_timestamps() -> Result<(), String> {
    Ok(())
}

/// Rend aux fd 1 et 2 leur sortie d'origine et attend que le fil ait écrit
/// tout ce que le tube tenait. À appeler à l'arrêt propre de l'hôte.
#[cfg(unix)]
pub fn stop_timestamps() {
    use std::io::Write;
    let Some(i) = INSTALLED.lock().unwrap_or_else(|p| p.into_inner()).take() else { return };
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    // SAFETY : les fd d'origine ont été gardés par `install_timestamps` ; le
    // dernier bout d'écriture du tube se ferme ici, le fil lit sa fin.
    unsafe {
        libc::dup2(i.stdout, 1);
        libc::dup2(i.stderr, 2);
        libc::close(i.stdout);
        libc::close(i.stderr);
    }
    let _ = i.reader.join();
}

#[cfg(not(unix))]
pub fn stop_timestamps() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_heure_iso_porte_les_millisecondes_et_le_decalage() {
        // 2026-11-07T16:40:12.345Z vu à +01:00.
        assert_eq!(format_iso(1_794_069_612, 345, 3600), "2026-11-07T17:40:12.345+01:00");
        assert_eq!(format_iso(0, 7, 0), "1970-01-01T00:00:00.007+00:00");
        assert_eq!(format_iso(951_782_400, 0, -5 * 3600 - 1800), "2000-02-28T18:30:00.000-05:30");
        let n = now_iso();
        assert_eq!(n.len(), 29, "{n}");
        assert_eq!(&n[10..11], "T");
    }

    #[test]
    fn l_appelant_vit_le_temps_de_l_appel_et_se_remet() {
        assert_eq!(caller_suffix(), "");
        {
            let _a = enter_call("mcp (pid 12)", "s-1", "b1");
            assert_eq!(caller_suffix(), " ⟨mcp (pid 12) · session s-1 · backend b1⟩");
            {
                let _b = enter_call("ide", "s-2", "b2");
                assert!(caller_suffix().contains("ide · session s-2"));
            }
            assert!(caller_suffix().contains("mcp (pid 12)"));
        }
        assert_eq!(caller_suffix(), "");
        // Un autre fil ne voit pas l'appelant de celui-ci.
        let _a = enter_call("mcp (pid 12)", "s-1", "b1");
        assert_eq!(std::thread::spawn(caller_suffix).join().unwrap(), "");
    }
}
