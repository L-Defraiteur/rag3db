//! **Les témoins du relai, de l'écoute et du manifeste d'hôte natif** : un faux Chrome (une paire
//! de sockets pour son entrée et sa sortie), un faux hôte (un socket Unix qui écoute), et une
//! fausse extension qui parle au vrai socket de l'[`Ecoute`].

use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::json;

use super::cadrage::Decodeur;
use super::ecoute::Ecoute;
use super::hote_natif::{chemin_du_relai, manifeste_d_hote, ID_EXTENSION, NOM_HOTE_NATIF};
use super::pont::Politique;
use super::protocole::Commande;
use super::relai::{relayer, valider_le_socket, Fin};

const DELAI: Duration = Duration::from_secs(5);

/// Un dossier neuf pour un socket (court : un chemin de socket Unix tient en 108 octets).
fn dossier() -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!("pont-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn trame(v: &serde_json::Value) -> Vec<u8> {
    let corps = serde_json::to_vec(v).unwrap();
    let mut t = (corps.len() as u32).to_le_bytes().to_vec();
    t.extend(corps);
    t
}

fn lire_exactement(s: &mut impl Read, n: usize) -> Vec<u8> {
    let mut v = vec![0u8; n];
    s.read_exact(&mut v).unwrap();
    v
}

/// Le faux hôte accepte la connexion du relai au plus [`DELAI`] : sans elle, le témoin échoue
/// (il ne pend pas).
fn accepter(ecoute: &UnixListener) -> (UnixStream, std::os::unix::net::SocketAddr) {
    ecoute.set_nonblocking(true).unwrap();
    let debut = std::time::Instant::now();
    loop {
        match ecoute.accept() {
            Ok((s, a)) => {
                s.set_nonblocking(false).unwrap();
                return (s, a);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock && debut.elapsed() < DELAI => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => panic!("le relai ne s'est pas connecté au faux hôte en {DELAI:?} : {e}"),
        }
    }
}

/// Lance le relai sur un faux Chrome : rend (l'entrée que Chrome écrit, la sortie que Chrome lit,
/// le fil du relai).
fn relai_sur(socket: PathBuf) -> (UnixStream, UnixStream, std::thread::JoinHandle<Result<Fin, String>>) {
    let (chrome_ecrit, relai_lit) = UnixStream::pair().unwrap();
    let (relai_ecrit, chrome_lit) = UnixStream::pair().unwrap();
    let fil = std::thread::spawn(move || relayer(relai_lit, relai_ecrit, &socket));
    chrome_lit.set_read_timeout(Some(DELAI)).unwrap();
    (chrome_ecrit, chrome_lit, fil)
}

#[test]
fn le_relai_passe_les_octets_dans_les_deux_sens_a_l_identique() {
    let d = dossier();
    let sock = d.join("h.sock");
    let ecoute = UnixListener::bind(&sock).unwrap();
    let (mut chrome_ecrit, mut chrome_lit, _fil) = relai_sur(sock);
    let (mut hote, _) = accepter(&ecoute);
    hote.set_read_timeout(Some(DELAI)).unwrap();
    let vers_hote = trame(&json!({"type": "hello", "browserId": "b", "extensionVersion": "0.0.30", "protocolRevision": 5}));
    chrome_ecrit.write_all(&vers_hote).unwrap();
    assert_eq!(lire_exactement(&mut hote, vers_hote.len()), vers_hote, "Chrome → hôte, octet pour octet");
    let vers_chrome = trame(&json!({"type": "ready", "version": "témoin"}));
    hote.write_all(&vers_chrome).unwrap();
    assert_eq!(lire_exactement(&mut chrome_lit, vers_chrome.len()), vers_chrome, "hôte → Chrome, octet pour octet");
}

#[test]
fn le_relai_s_arrete_quand_l_hote_ferme() {
    let d = dossier();
    let sock = d.join("h.sock");
    let ecoute = UnixListener::bind(&sock).unwrap();
    let (_chrome_ecrit, _chrome_lit, fil) = relai_sur(sock);
    let (hote, _) = accepter(&ecoute);
    drop(hote);
    assert_eq!(fil.join().unwrap(), Ok(Fin::Hote));
}

#[test]
fn le_relai_s_arrete_quand_chrome_ferme() {
    let d = dossier();
    let sock = d.join("h.sock");
    let ecoute = UnixListener::bind(&sock).unwrap();
    let (chrome_ecrit, _chrome_lit, fil) = relai_sur(sock);
    let (_hote, _) = accepter(&ecoute);
    drop(chrome_ecrit);
    assert_eq!(fil.join().unwrap(), Ok(Fin::Chrome));
}

#[test]
fn sans_hote_le_relai_le_dit_et_n_ecrit_rien_vers_chrome() {
    let d = dossier();
    let (_chrome_ecrit, mut chrome_lit, fil) = relai_sur(d.join("absent.sock"));
    let e = fil.join().unwrap().expect_err("aucun hôte n'écoute");
    assert!(e.contains("absent.sock"), "le refus nomme le socket : {e}");
    let mut reste = Vec::new();
    chrome_lit.set_read_timeout(Some(Duration::from_millis(300))).unwrap();
    let _ = chrome_lit.read_to_end(&mut reste);
    assert!(reste.is_empty(), "rien vers Chrome : {reste:?}");
}

#[test]
fn le_relai_ne_parle_qu_a_un_socket_unix_jamais_a_un_port() {
    for adresse in ["127.0.0.1:7970", "localhost:7970", "tcp://127.0.0.1:7970", "relatif/pont.sock"] {
        let e = valider_le_socket(adresse).expect_err(adresse);
        assert!(e.contains("socket Unix"), "{adresse} : {e}");
    }
    assert_eq!(valider_le_socket("/run/user/1000/rag3weaver/pont-navigateur.sock"), Ok(PathBuf::from("/run/user/1000/rag3weaver/pont-navigateur.sock")));
}

/// Une fausse extension sur le socket de l'écoute : envoie son `hello`, puis répond `ok` à tout.
fn fausse_extension(sock: &std::path::Path, browser_id: &str) -> UnixStream {
    let s = UnixStream::connect(sock).unwrap();
    let mut e = s.try_clone().unwrap();
    e.write_all(&trame(&json!({"type": "hello", "browserId": browser_id, "extensionVersion": "0.0.30", "protocolRevision": 5, "features": ["rag3weaver-gardes"]}))).unwrap();
    let mut l = s.try_clone().unwrap();
    std::thread::spawn(move || {
        let mut d = Decodeur::default();
        let mut buf = [0u8; 4096];
        loop {
            let n = match l.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => n,
            };
            for m in d.pousser(&buf[..n]).into_iter().flatten() {
                if m["type"] == "command" {
                    let r = json!({"type": "result", "result": {"id": m["command"]["id"], "ok": true, "data": {"pong": true}}});
                    if e.write_all(&trame(&r)).is_err() {
                        return;
                    }
                }
            }
        }
    });
    s
}

fn attendre<F: Fn() -> bool>(f: F) -> bool {
    let debut = std::time::Instant::now();
    while debut.elapsed() < DELAI {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn l_ecoute_range_un_pont_par_navigateur_et_le_socket_est_a_soi() {
    let d = dossier();
    let sock = d.join("ecoute").join("pont.sock");
    let ecoute = Ecoute::ouvrir(&sock, "rag3weaver-temoin", Politique::default()).expect("l'écoute s'ouvre");
    assert_eq!(std::fs::metadata(&sock).unwrap().permissions().mode() & 0o777, 0o600, "le socket est en 0600");
    assert_eq!(std::fs::metadata(sock.parent().unwrap()).unwrap().permissions().mode() & 0o777, 0o700, "son dossier en 0700");
    let _ext = fausse_extension(&sock, "nav-1");
    assert!(attendre(|| ecoute.pont("nav-1").is_some()), "le pont du navigateur nav-1");
    let r = ecoute.pont("nav-1").unwrap().envoyer(Commande::nouvelle("ping"), DELAI).expect("ping par le socket");
    assert_eq!(r.data, Some(json!({"pong": true})));
    let mut cdp = Commande::nouvelle("cdp");
    cdp.cdp_method = Some("Network.getAllCookies".into());
    assert_eq!(ecoute.pont("nav-1").unwrap().envoyer(cdp, DELAI).expect_err("fermé").code, "action_closed", "les gardes du cœur tiennent par le socket");
}

#[test]
fn une_nouvelle_connexion_du_meme_navigateur_remplace_l_ancienne() {
    let d = dossier();
    let sock = d.join("pont.sock");
    let ecoute = Ecoute::ouvrir(&sock, "rag3weaver-temoin", Politique::default()).expect("l'écoute s'ouvre");
    let _a = fausse_extension(&sock, "nav-1");
    assert!(attendre(|| ecoute.pont("nav-1").is_some()));
    let ancien = ecoute.pont("nav-1").unwrap();
    let _b = fausse_extension(&sock, "nav-1");
    assert!(attendre(|| !ancien.etat().connectee), "l'ancienne connexion est fermée");
    assert!(attendre(|| ecoute.pont("nav-1").is_some_and(|p| p.etat().connectee)), "la nouvelle sert");
    assert_eq!(ecoute.navigateurs(), vec!["nav-1".to_string()]);
}

#[test]
fn un_second_hote_sur_le_meme_socket_est_refuse_et_un_fichier_mort_est_remplace() {
    let d = dossier();
    let sock = d.join("pont.sock");
    let premiere = Ecoute::ouvrir(&sock, "a", Politique::default()).expect("la première");
    assert!(Ecoute::ouvrir(&sock, "b", Politique::default()).is_err(), "un hôte écoute déjà");
    drop(premiere);
    assert!(!sock.exists(), "lâchée, l'écoute retire son socket");
    let mort = UnixListener::bind(&sock).unwrap();
    drop(mort);
    assert!(sock.exists());
    assert!(Ecoute::ouvrir(&sock, "c", Politique::default()).is_ok(), "un fichier mort est remplacé");
}

#[test]
fn le_manifeste_d_hote_pointe_le_relai_par_un_chemin_stable_et_n_admet_que_notre_extension() {
    let home = PathBuf::from("/home/temoin");
    assert_eq!(chemin_du_relai(&home), PathBuf::from("/home/temoin/.local/bin/rag3weaver-pont-navigateur"));
    let m = manifeste_d_hote(&home);
    assert_eq!(m["name"], json!(NOM_HOTE_NATIF));
    assert_eq!(m["type"], json!("stdio"));
    assert_eq!(m["path"], json!("/home/temoin/.local/bin/rag3weaver-pont-navigateur"));
    assert_eq!(m["allowed_origins"], json!([format!("chrome-extension://{ID_EXTENSION}/")]));
}
