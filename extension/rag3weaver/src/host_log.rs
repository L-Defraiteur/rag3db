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

/// Ce qu'une ligne attend au plus une sortie qui ne prend pas (un pic) avant
/// d'être jetée : l'hôte est ralenti d'autant au pire, jamais figé.
///
/// **D'où vient 50 ms** (mesuré le 7 novembre 2026, 18 h, par
/// `scripts/mesure_patience_du_journal.py` : écriture non bloquante et `poll`
/// POLLOUT, comme le relais) : le temps que met la sortie à reprendre après
/// `EAGAIN`, en rafales de 1 000 lignes de 1 Kio et de 10 lignes de 64 Kio,
/// trois passes chacune, sur les deux postes en charge (portable : 11,6 ;
/// luciepc : 15,5).
/// - Vers le socket de flux de journald (le stderr d'un service systemd) :
///   2 à 13 `EAGAIN` par rafale, attente médiane 0,37 à 0,65 ms, **pire
///   1,14 ms** sur les 36 rafales.
/// - Vers un fichier en ajout (`StandardError=append:`, le cas de l'unité de
///   l'hôte) : aucun `EAGAIN` (un fichier ordinaire ne bloque pas), écriture
///   au pire 0,03 ms.
/// 50 ms laisse environ 44 fois le pire mesuré : un journald qui cale plus
/// longtemps (mémoire sous pression, fsync) est une panne, et la perte s'y
/// dit. La première version disait 200 ms sans mesure.
const WRITE_PATIENCE: std::time::Duration = std::time::Duration::from_millis(50);

/// Une ligne plus longue est coupée là : le relais ne grossit pas sans borne
/// sur une sortie sans fin de ligne.
const MAX_LINE: usize = 64 * 1024;

/// **Le relais ne bloque jamais l'hôte** (c7, 7 novembre 2026). Toutes les
/// écritures du démon et de la lib passent par le tube ; si ce fil cessait de
/// le vider, le tube (64 Kio) se remplirait et chaque `eprintln!` du démon,
/// chaque ligne de la lib, bloquerait : l'hôte se figerait. D'où :
/// - la sortie d'origine est **non bloquante** (`O_NONBLOCK`) : un journal
///   systemd lent, un tube plein, un disque plein rendent `EAGAIN` ou une
///   erreur, jamais une attente sans fin. Sur `EAGAIN`, la ligne attend que la
///   sortie reprenne au plus [`WRITE_PATIENCE`] (un pic de journald ne perd
///   rien) ; au-delà, ou sur une autre erreur, elle est **jetée et comptée**,
///   les suivantes n'attendent plus jusqu'à ce qu'une écriture passe, et la
///   ligne qui passe dit combien ont été perdues ;
/// - le fil **ne s'arrête qu'à la fin du tube** (tous les bouts d'écriture
///   fermés, c'est-à-dire `stop_timestamps`) : une erreur de lecture se
///   réessaie, aucune ne le fait sortir ; rien dans la boucle ne panique
///   (pas d'`unwrap`, pas d'indice), et une panique quand même serait
///   rattrapée et la boucle reprendrait ;
/// - une ligne est bornée à [`MAX_LINE`] : pas de mémoire sans borne.
/// Le seul appel bloquant du fil est la lecture du tube, qui attend des
/// lignes, jamais la sortie.
#[cfg(unix)]
fn spawn_relay(read_fd: libc::c_int, out_fd: libc::c_int) -> std::io::Result<std::thread::JoinHandle<()>> {
    // SAFETY : `out_fd` appartient au relais. O_NONBLOCK se pose sur la
    // description, partagée avec le fd 2 d'origine (un terminal, si l'hôte
    // tourne à la main) : ses drapeaux d'avant sont rendus en sortant.
    let flags = unsafe { libc::fcntl(out_fd, libc::F_GETFL) };
    unsafe {
        libc::fcntl(out_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    std::thread::Builder::new().name("journal-heure".into()).spawn(move || {
        let mut relay = Relay { out_fd, lost: 0, torn: false };
        let mut pending: Vec<u8> = Vec::with_capacity(4096);
        let mut buf = [0u8; 8192];
        loop {
            // SAFETY : `read_fd` appartient au relais ; `buf` vit ici.
            let n = unsafe { libc::read(read_fd, buf.as_mut_ptr().cast(), buf.len()) };
            if n == 0 {
                break; // la fin du tube : plus aucun écrivain
            }
            if n < 0 {
                // EINTR ou autre : jamais une raison de cesser de vider le tube.
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }
            let chunk = buf.get(..n as usize).unwrap_or_default();
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                for &b in chunk {
                    if b == b'\n' || pending.len() >= MAX_LINE {
                        relay.emit(&pending);
                        pending.clear();
                        if b == b'\n' {
                            continue;
                        }
                    }
                    pending.push(b);
                }
            }));
        }
        if !pending.is_empty() {
            relay.emit(&pending);
        }
        // SAFETY : les deux fd sont au relais, fermés une fois ; les drapeaux
        // de la sortie d'origine lui sont rendus.
        unsafe {
            if flags >= 0 {
                libc::fcntl(out_fd, libc::F_SETFL, flags);
            }
            libc::close(read_fd);
            libc::close(out_fd);
        }
    })
}

/// La sortie du relais : une ligne horodatée par écriture, jamais d'attente.
#[cfg(unix)]
struct Relay {
    out_fd: libc::c_int,
    /// Les lignes jetées depuis la dernière qui a passé.
    lost: u64,
    /// La dernière écriture s'est arrêtée au milieu d'une ligne.
    torn: bool,
}

#[cfg(unix)]
impl Relay {
    fn emit(&mut self, line: &[u8]) {
        let mut l = Vec::with_capacity(line.len() + 96);
        if self.torn {
            l.push(b'\n');
        }
        let ts = now_iso();
        if self.lost > 0 {
            l.extend_from_slice(format!("{ts} [journal] {} ligne(s) perdue(s) : la sortie du journal ne prenait plus\n", self.lost).as_bytes());
        }
        l.extend_from_slice(ts.as_bytes());
        l.push(b' ');
        l.extend_from_slice(line);
        l.push(b'\n');
        let mut done = 0usize;
        let debut = std::time::Instant::now();
        while done < l.len() {
            let rest = l.get(done..).unwrap_or_default();
            // SAFETY : écriture d'un tampon que nous possédons sur notre fd.
            let w = unsafe { libc::write(self.out_fd, rest.as_ptr().cast(), rest.len()) };
            if w > 0 {
                done += w as usize;
                continue;
            }
            let err = std::io::Error::last_os_error();
            if w < 0 && err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            // La sortie ne prend pas pour l'instant (EAGAIN) : on lui laisse un
            // court délai pour reprendre, borné par ligne ; une sortie qui a
            // déjà perdu une ligne n'en reçoit plus, jusqu'à ce qu'une écriture
            // passe. ENOSPC, EBADF… : jetée tout de suite.
            if w < 0 && err.kind() == std::io::ErrorKind::WouldBlock && self.lost == 0 {
                let reste = WRITE_PATIENCE.saturating_sub(debut.elapsed());
                if !reste.is_zero() {
                    let mut pfd = libc::pollfd { fd: self.out_fd, events: libc::POLLOUT, revents: 0 };
                    // SAFETY : un pollfd à nous, sur notre fd.
                    let pret = unsafe { libc::poll(&mut pfd, 1, reste.as_millis().max(1) as libc::c_int) };
                    if pret > 0 {
                        continue;
                    }
                }
            }
            break;
        }
        if done == l.len() {
            self.lost = 0;
            self.torn = false;
        } else {
            self.lost += 1;
            self.torn = done > 0;
        }
    }
}

/// Marque d'heure en tête de chaque ligne écrite sur les fd 1 et 2 de ce
/// processus, à poser au démarrage de l'hôte (avant toute ligne). Une seconde
/// pose ne fait rien. Hors Unix, rien : les lignes partent sans heure.
#[cfg(unix)]
pub fn install_timestamps() -> Result<(), String> {
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
        // La copie de la sortie d'origine que le relais écrit, non bloquante.
        let out_fd = libc::dup(stderr);
        if out_fd < 0 {
            return Err(format!("dup de la sortie : {}", std::io::Error::last_os_error()));
        }
        libc::fcntl(out_fd, libc::F_SETFD, libc::FD_CLOEXEC);
        let reader = spawn_relay(read_end, out_fd).map_err(|e| format!("fil du journal : {e}"))?;
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
    // Un enfant lancé par l'hôte (une commande) peut tenir encore le tube par
    // ses fd 1 et 2 : le relais ne verrait jamais la fin. L'arrêt n'attend
    // donc que deux secondes que le tube se vide.
    let debut = std::time::Instant::now();
    while !i.reader.is_finished() && debut.elapsed() < std::time::Duration::from_secs(2) {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    if i.reader.is_finished() {
        let _ = i.reader.join();
    }
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

    /// Une sortie d'origine qui ne prend plus (un tube que personne ne lit) :
    /// l'écrivain du démon ne bloque pas, le relais jette et compte ; la
    /// sortie rouverte, la ligne suivante dit combien ont été perdues.
    #[cfg(unix)]
    #[test]
    fn une_sortie_pleine_ne_fige_pas_l_ecrivain() {
        use std::io::{Read, Write};
        use std::os::fd::FromRawFd;
        let mut entree = [0 as libc::c_int; 2];
        let mut sortie = [0 as libc::c_int; 2];
        // SAFETY : deux tubes à ce test.
        unsafe {
            assert_eq!(libc::pipe(entree.as_mut_ptr()), 0);
            assert_eq!(libc::pipe(sortie.as_mut_ptr()), 0);
        }
        let relais = spawn_relay(entree[0], sortie[1]).unwrap();
        // SAFETY : le bout d'écriture du tube d'entrée est au test.
        let mut w = unsafe { std::fs::File::from_raw_fd(entree[1]) };
        // 4 Mio de lignes, soixante fois la capacité d'un tube, pendant que
        // personne ne lit la sortie : l'écrivain ne doit jamais attendre.
        let (fini, attente) = std::sync::mpsc::channel();
        let ecrivain = std::thread::spawn(move || {
            let ligne = format!("{}\n", "x".repeat(1023));
            for _ in 0..4096 {
                w.write_all(ligne.as_bytes()).unwrap();
            }
            fini.send(()).unwrap();
            w
        });
        assert!(
            attente.recv_timeout(std::time::Duration::from_secs(20)).is_ok(),
            "l'écrivain est bloqué : le relais attend la sortie au lieu de jeter"
        );
        let mut w = ecrivain.join().unwrap();
        // La sortie se vide (quelqu'un la relit enfin) ; la ligne suivante passe
        // et dit combien ont été jetées.
        // SAFETY : le bout de lecture de la sortie est au test.
        let mut r = unsafe { std::fs::File::from_raw_fd(sortie[0]) };
        let lecteur = std::thread::spawn(move || {
            let mut lu = String::new();
            let _ = r.read_to_string(&mut lu);
            lu
        });
        std::thread::sleep(std::time::Duration::from_millis(200));
        w.write_all(b"apres\n").unwrap();
        drop(w); // la fin du tube : le relais s'arrête
        relais.join().unwrap();
        let lu = lecteur.join().unwrap();
        assert!(lu.len() < 4096 * 1024, "la sortie a tout reçu ({} octets) : elle ne débordait pas", lu.len());
        let perte = lu.lines().find(|l| l.contains("[journal] ")).unwrap_or_else(|| panic!("les pertes ne se disent pas : {}", lu.lines().last().unwrap_or_default()));
        assert!(perte.contains("ligne(s) perdue(s)"), "{perte}");
        assert!(lu.lines().last().unwrap_or_default().ends_with(" apres"), "la ligne d'après passe : {:?}", lu.lines().last());
    }

    #[cfg(unix)]
    #[test]
    fn une_ligne_sans_fin_est_coupee_a_sa_borne() {
        use std::io::{Read, Write};
        use std::os::fd::FromRawFd;
        let mut entree = [0 as libc::c_int; 2];
        let mut sortie = [0 as libc::c_int; 2];
        // SAFETY : deux tubes à ce test.
        unsafe {
            assert_eq!(libc::pipe(entree.as_mut_ptr()), 0);
            assert_eq!(libc::pipe(sortie.as_mut_ptr()), 0);
        }
        let relais = spawn_relay(entree[0], sortie[1]).unwrap();
        let lecteur = std::thread::spawn(move || {
            let mut lu = String::new();
            // SAFETY : le bout de lecture de la sortie est à ce fil.
            let _ = unsafe { std::fs::File::from_raw_fd(sortie[0]) }.read_to_string(&mut lu);
            lu
        });
        {
            // SAFETY : le bout d'écriture du tube d'entrée est à ce test.
            let mut w = unsafe { std::fs::File::from_raw_fd(entree[1]) };
            w.write_all(&vec![b'y'; MAX_LINE + 10]).unwrap();
        }
        relais.join().unwrap();
        let lu = lecteur.join().unwrap();
        let lignes: Vec<&str> = lu.lines().collect();
        assert_eq!(lignes.len(), 2, "{} lignes", lignes.len());
        assert!(lignes.iter().all(|l| l.len() <= MAX_LINE + 40 && l.as_bytes().get(10) == Some(&b'T')));
    }
}
