//! **Les témoins du pont, sur un faux pont** : une fausse extension dans un fil parle la révision 5
//! par une paire de sockets, compte ce qu'elle reçoit et répond selon le cas.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use super::cadrage::{encoder, Decodeur, MAX_TRAME_VERS_EXTENSION};
use super::baux::EtatBail;
use super::pont::{Politique, Pont};
use super::protocole::{Claim, Commande, Resultat, VersExtension, VersHote};

const DELAI: Duration = Duration::from_secs(5);

/// Une trame écrite à la main, comme l'extension l'écrit.
fn trame(v: &Value) -> Vec<u8> {
    let corps = serde_json::to_vec(v).unwrap();
    let mut t = (corps.len() as u32).to_le_bytes().to_vec();
    t.extend(corps);
    t
}

/// Ce que la fausse extension fait d'une commande : des messages à renvoyer (vide : se taire).
type Repondeur = Box<dyn FnMut(&Commande) -> Vec<VersHote> + Send>;

/// **La fausse extension** : elle envoie `hello` à la révision donnée, note le `ready` et les
/// commandes reçues, et répond par son répondeur.
struct FausseExtension {
    recues: Arc<Mutex<Vec<Commande>>>,
    ready: Arc<Mutex<Option<String>>>,
    ecrivain: Arc<Mutex<UnixStream>>,
}

impl FausseExtension {
    fn lancer(revision: Option<u32>, mut repondeur: Repondeur) -> (Pont, FausseExtension) {
        Self::lancer_avec(revision, Politique::default(), move |c| repondeur(c))
    }

    fn lancer_avec(revision: Option<u32>, politique: Politique, mut repondeur: impl FnMut(&Commande) -> Vec<VersHote> + Send + 'static) -> (Pont, FausseExtension) {
        let (hote, ext) = UnixStream::pair().unwrap();
        let ecrivain = Arc::new(Mutex::new(ext.try_clone().unwrap()));
        let recues = Arc::new(Mutex::new(Vec::new()));
        let ready = Arc::new(Mutex::new(None));
        let (r, rd, e) = (recues.clone(), ready.clone(), ecrivain.clone());
        let mut lecteur = ext;
        std::thread::spawn(move || {
            let hello = json!({"type": "hello", "browserId": "faux-navigateur", "extensionVersion": "0.0.30", "protocolRevision": revision, "features": ["cdp", "page-evaluate"]});
            let mut hello = hello;
            if revision.is_none() {
                hello.as_object_mut().unwrap().remove("protocolRevision");
            }
            if e.lock().unwrap().write_all(&trame(&hello)).is_err() {
                return;
            }
            let mut dec = Decodeur::default();
            let mut buf = [0u8; 8192];
            loop {
                let n = match lecteur.read(&mut buf) {
                    Ok(0) | Err(_) => return,
                    Ok(n) => n,
                };
                for m in dec.pousser(&buf[..n]) {
                    let Ok(m) = m else { continue };
                    match serde_json::from_value::<VersExtension>(m) {
                        Ok(VersExtension::Ready { version }) => *rd.lock().unwrap() = Some(version),
                        Ok(VersExtension::Command { command }) => {
                            r.lock().unwrap().push(command.clone());
                            for reponse in repondeur(&command) {
                                let v = serde_json::to_value(&reponse).unwrap();
                                if e.lock().unwrap().write_all(&trame(&v)).is_err() {
                                    return;
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
        });
        let pont = Pont::demarrer(hote.try_clone().unwrap(), hote, "rag3weaver-temoin", politique);
        (pont, FausseExtension { recues, ready, ecrivain })
    }

    fn recues(&self) -> Vec<Commande> {
        self.recues.lock().unwrap().clone()
    }

    fn envoyer_brut(&self, v: &Value) {
        self.ecrivain.lock().unwrap().write_all(&trame(v)).unwrap();
    }

    fn fermer(&self) {
        let _ = self.ecrivain.lock().unwrap().shutdown(std::net::Shutdown::Both);
    }
}

fn ok(c: &Commande, data: Value) -> Vec<VersHote> {
    vec![VersHote::Result { result: Resultat { id: c.id.clone(), ok: true, data: Some(data), page: c.page.clone(), ..Default::default() } }]
}

/// Répond `ok` à tout, et `page: "123"` à un `claim`.
fn repondeur_simple() -> Repondeur {
    Box::new(|c: &Commande| {
        let mut r = ok(c, json!({"pong": true}));
        if c.action == "claim" {
            if let VersHote::Result { result } = &mut r[0] {
                result.page = Some("123".into());
            }
        }
        r
    })
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
fn le_cadrage_fait_l_aller_retour_meme_en_morceaux() {
    let m = json!({"type": "command", "command": {"id": "1", "action": "ping"}});
    let t = encoder(&m).expect("une petite trame s'encode");
    assert_eq!(&t[..4], &(t.len() as u32 - 4).to_le_bytes(), "la longueur en tête, petit-boutiste");
    let mut deux = t.clone();
    deux.extend(&t);
    let mut d = Decodeur::default();
    let mut lus = Vec::new();
    for morceau in deux.chunks(3) {
        lus.extend(d.pousser(morceau));
    }
    assert_eq!(lus.len(), 2, "deux trames découpées par trois octets : {lus:?}");
    assert!(lus.iter().all(|r| r.as_ref().ok() == Some(&m)));
}

#[test]
fn une_trame_trop_grosse_vers_l_extension_est_refusee_avant_l_envoi() {
    let gros = json!({"code": "x".repeat(MAX_TRAME_VERS_EXTENSION)});
    let e = encoder(&gros).expect_err("plus de 1 Mio : refusé");
    assert!(e.contains("1048576") || e.contains("1 Mio"), "le refus dit la borne : {e}");
}

#[test]
fn un_corps_illisible_se_dit_et_la_trame_suivante_se_lit() {
    let mut d = Decodeur::default();
    let mut t = 3u32.to_le_bytes().to_vec();
    t.extend(b"{x}");
    t.extend(trame(&json!({"type": "event"})));
    let lus = d.pousser(&t);
    assert_eq!(lus.len(), 2, "{lus:?}");
    assert!(lus[0].is_err() && lus[1].is_ok(), "{lus:?}");
}

#[test]
fn hello_recoit_ready_et_l_etat_le_dit() {
    let (pont, ext) = FausseExtension::lancer(Some(5), repondeur_simple());
    assert!(pont.attendre_hello(DELAI), "le hello de l'extension est lu");
    let etat = pont.etat();
    assert!(etat.connectee && etat.revision == Some(5) && etat.browser_id.as_deref() == Some("faux-navigateur"), "{etat:?}");
    assert!(attendre(|| ext.ready.lock().unwrap().is_some()), "l'hôte répond ready");
    assert_eq!(ext.ready.lock().unwrap().as_deref(), Some("rag3weaver-temoin"));
    let r = pont.envoyer(Commande::nouvelle("ping"), DELAI).expect("ping");
    assert_eq!(r.data, Some(json!({"pong": true})));
}

#[test]
fn une_autre_revision_refuse_tout_sauf_ping_sans_l_envoyer() {
    let (pont, ext) = FausseExtension::lancer(Some(4), repondeur_simple());
    assert!(pont.attendre_hello(DELAI));
    let mut exec = Commande::nouvelle("exec");
    exec.code = Some("document.title".into());
    let e = pont.envoyer(exec, DELAI).expect_err("révision 4 : exec refusé");
    assert_eq!(e.code, "protocol_mismatch", "{e:?}");
    assert!(pont.envoyer(Commande::nouvelle("ping"), DELAI).is_ok(), "ping passe toujours");
    let actions: Vec<String> = ext.recues().into_iter().map(|c| c.action).collect();
    assert_eq!(actions, vec!["ping".to_string()], "exec n'est jamais parti");
}

#[test]
fn deux_commandes_servies_a_l_envers_vont_chacune_a_son_appel() {
    // La fausse extension garde la première et répond aux deux à l'arrivée de la seconde, à l'envers.
    let attente: Arc<Mutex<Option<Commande>>> = Default::default();
    let a = attente.clone();
    let (pont, _ext) = FausseExtension::lancer(
        Some(5),
        Box::new(move |c: &Commande| {
            let mut a = a.lock().unwrap();
            match a.take() {
                None => {
                    *a = Some(c.clone());
                    vec![]
                }
                Some(premiere) => {
                    let mut r = ok(c, json!({"code": c.code}));
                    r.extend(ok(&premiere, json!({"code": premiere.code})));
                    r
                }
            }
        }),
    );
    assert!(pont.attendre_hello(DELAI));
    let p1 = pont.clone();
    let un = std::thread::spawn(move || {
        let mut c = Commande::nouvelle("exec");
        c.code = Some("un".into());
        p1.envoyer(c, DELAI)
    });
    assert!(attendre(|| attente.lock().unwrap().is_some()), "la première est arrivée");
    let mut c = Commande::nouvelle("exec");
    c.code = Some("deux".into());
    let deux = pont.envoyer(c, DELAI).expect("deux");
    let un = un.join().unwrap().expect("un");
    assert_eq!(deux.data, Some(json!({"code": "deux"})));
    assert_eq!(un.data, Some(json!({"code": "un"})));
}

#[test]
fn un_delai_depasse_rend_timeout_et_le_resultat_tardif_est_jete() {
    let (pont, ext) = FausseExtension::lancer(Some(5), Box::new(|_c: &Commande| vec![]));
    assert!(pont.attendre_hello(DELAI));
    let e = pont.envoyer(Commande::nouvelle("ping"), Duration::from_millis(200)).expect_err("personne ne répond");
    assert_eq!(e.code, "timeout", "{e:?}");
    let id = ext.recues()[0].id.clone();
    let cmd = ext.recues()[0].clone();
    assert!(cmd.deadline_at.is_some(), "deadlineAt est posé : {cmd:?}");
    ext.envoyer_brut(&json!({"type": "result", "result": {"id": id, "ok": true}}));
    std::thread::sleep(Duration::from_millis(100));
    assert!(pont.etat().connectee, "un résultat tardif ne casse pas le pont");
}

#[test]
fn le_canal_ferme_rend_extension_disconnected_a_l_appel_en_attente() {
    let (pont, ext) = FausseExtension::lancer(Some(5), Box::new(|_c: &Commande| vec![]));
    assert!(pont.attendre_hello(DELAI));
    let p = pont.clone();
    let attente = std::thread::spawn(move || p.envoyer(Commande::nouvelle("ping"), DELAI));
    assert!(attendre(|| !ext.recues().is_empty()));
    ext.fermer();
    let e = attente.join().unwrap().expect_err("le canal s'est fermé");
    assert_eq!(e.code, "extension_disconnected", "{e:?}");
    assert!(!pont.etat().connectee);
}

#[test]
fn un_onglet_rendu_a_l_humain_ne_se_reprend_que_par_un_nouveau_claim() {
    let (pont, ext) = FausseExtension::lancer(Some(5), repondeur_simple());
    assert!(pont.attendre_hello(DELAI));
    let session = Some("mcp:temoin".to_string());
    let mut claim = Commande::nouvelle("claim");
    claim.session = session.clone();
    claim.claim = Some(Claim { tab_id: Some(123), ..Default::default() });
    assert_eq!(pont.envoyer(claim.clone(), DELAI).expect("claim").page.as_deref(), Some("123"));
    assert_eq!(pont.baux().first().map(|b| b.etat), Some(EtatBail::Active), "{:?}", pont.baux());

    let mut mark = Commande::nouvelle("mark");
    mark.session = session.clone();
    mark.page = Some("123".into());
    mark.mark = Some(json!("handoff"));
    pont.envoyer(mark, DELAI).expect("mark");
    let mut fin = Commande::nouvelle("session-finalize");
    fin.session = session.clone();
    pont.envoyer(fin, DELAI).expect("finalize");
    assert_eq!(pont.baux().first().map(|b| b.etat), Some(EtatBail::Handoff), "{:?}", pont.baux());

    let envoyees = ext.recues().len();
    let mut exec = Commande::nouvelle("exec");
    exec.session = session.clone();
    exec.page = Some("123".into());
    exec.code = Some("document.title".into());
    let e = pont.envoyer(exec.clone(), DELAI).expect_err("l'onglet est à l'humain");
    assert_eq!(e.code, "page_handed_off", "{e:?}");
    assert_eq!(ext.recues().len(), envoyees, "l'exec n'est pas parti vers l'extension");

    assert!(pont.envoyer(claim, DELAI).is_ok(), "un nouveau claim");
    assert_eq!(pont.baux().first().map(|b| b.etat), Some(EtatBail::Active));
    assert!(pont.envoyer(exec, DELAI).is_ok(), "repris, l'exec passe");
}

#[test]
fn un_evenement_tab_released_ferme_le_bail_et_tab_closed_le_retire() {
    let (pont, ext) = FausseExtension::lancer(Some(5), repondeur_simple());
    assert!(pont.attendre_hello(DELAI));
    ext.envoyer_brut(&json!({"type": "event", "event": {"kind": "tab_acquired", "session": "s", "page": "7", "tabId": 7, "origin": "user"}}));
    assert!(attendre(|| pont.baux().iter().any(|b| b.page == "7" && b.etat == EtatBail::Active && b.origine == "user")), "{:?}", pont.baux());
    ext.envoyer_brut(&json!({"type": "event", "event": {"kind": "tab_released", "session": "s", "page": "7", "tabId": 7}}));
    assert!(attendre(|| pont.baux().iter().any(|b| b.page == "7" && b.etat == EtatBail::Released)), "{:?}", pont.baux());
    let mut exec = Commande::nouvelle("exec");
    exec.session = Some("s".into());
    exec.page = Some("7".into());
    assert_eq!(pont.envoyer(exec, DELAI).expect_err("rendu").code, "page_released");
    ext.envoyer_brut(&json!({"type": "event", "event": {"kind": "tab_closed", "session": "s", "page": "7", "tabId": 7}}));
    assert!(attendre(|| pont.baux().is_empty()), "{:?}", pont.baux());
    assert_eq!(pont.evenements().len(), 3);
}

#[test]
fn les_passe_partout_sont_fermes_par_defaut_sans_envoi() {
    let (pont, ext) = FausseExtension::lancer(Some(5), repondeur_simple());
    assert!(pont.attendre_hello(DELAI));
    for action in ["cdp", "chrome-call", "cookies"] {
        let mut c = Commande::nouvelle(action);
        c.cdp_method = Some("Network.getAllCookies".into());
        c.chrome_method = Some("tabs.query".into());
        let e = pont.envoyer(c, DELAI).expect_err(action);
        assert_eq!(e.code, "action_closed", "{action} : {e:?}");
    }
    assert!(ext.recues().is_empty(), "rien n'est parti : {:?}", ext.recues());
}

#[test]
fn une_methode_cdp_listee_par_la_politique_passe() {
    let mut politique = Politique::default();
    politique.methodes_cdp.insert("Page.captureScreenshot".into());
    let (pont, ext) = FausseExtension::lancer_avec(Some(5), politique, |c| ok(c, json!({})));
    assert!(pont.attendre_hello(DELAI));
    let mut c = Commande::nouvelle("cdp");
    c.cdp_method = Some("Page.captureScreenshot".into());
    assert!(pont.envoyer(c, DELAI).is_ok(), "listée : envoyée");
    let mut c = Commande::nouvelle("cdp");
    c.cdp_method = Some("Network.getAllCookies".into());
    assert_eq!(pont.envoyer(c, DELAI).expect_err("non listée").code, "action_closed");
    assert_eq!(ext.recues().len(), 1);
}
