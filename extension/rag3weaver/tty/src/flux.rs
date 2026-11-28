//! **Le flux de l'hôte** (`GET /ide/flux`), lu sur une connexion TCP à nous :
//! l'hôte écrit un flux SSE brut (ni longueur ni découpage en morceaux), et une
//! lecture bornée dans le temps laisse vérifier l'arrêt entre deux octets. La
//! reprise passe par `Last-Event-ID` : l'hôte rejoue ce qui suit, ou dit une
//! `lacune` quand son tampon ne le garde plus.

use super::client::encoder;
use super::endpoint::Endpoint;
use super::sse::{Decodeur, Trame};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Ce que le lecteur veut après une trame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suite {
    Continuer,
    /// Fermer la connexion (un témoin coupe ainsi le flux au milieu).
    Couper,
}

/// Pourquoi une lecture s'est arrêtée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fin {
    /// Le lecteur a coupé.
    Coupe,
    /// L'arrêt demandé de l'extérieur.
    Arrete,
    /// La connexion est tombée (l'hôte s'arrête, le réseau coupe).
    Tombee(String),
}

/// Le pas de vérification de l'arrêt pendant une lecture.
const PAS: Duration = Duration::from_millis(200);

/// **Une connexion au flux** : rend le dernier `id` vu (ou `depuis` si aucun)
/// et la raison de la fin. Une erreur est un refus avant le flux (jeton,
/// adresse) : la réessayer ne servirait à rien.
///
/// `ouvert` est appelé une fois l'abonnement pris (l'en-tête 200 lu) : ce qui
/// se publie ensuite arrivera. Un client qui lance un run sans curseur attend
/// ce moment avant de le lancer.
pub fn suivre(ep: &Endpoint, sujets: &[&str], depuis: Option<u64>, arret: &AtomicBool, ouvert: &mut dyn FnMut(), mut f: impl FnMut(&Trame) -> Suite) -> Result<(Option<u64>, Fin), String> {
    let adresse = ep.address.to_socket_addrs().map_err(|e| format!("{} : {e}", ep.address))?.next().ok_or_else(|| format!("{} : aucune adresse", ep.address))?;
    let mut tcp = match TcpStream::connect_timeout(&adresse, Duration::from_secs(5)) {
        Ok(t) => t,
        Err(e) => return Ok((depuis, Fin::Tombee(format!("connexion à {} : {e}", ep.address)))),
    };
    tcp.set_read_timeout(Some(PAS)).map_err(|e| e.to_string())?;
    let _ = tcp.set_nodelay(true);
    let mut requete = format!(
        "GET /ide/flux?sujets={} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nAccept: text/event-stream\r\n",
        encoder(&sujets.join(",")),
        ep.address,
        ep.token
    );
    if let Some(d) = depuis {
        requete.push_str(&format!("Last-Event-ID: {d}\r\n"));
    }
    requete.push_str("\r\n");
    if let Err(e) = tcp.write_all(requete.as_bytes()) {
        return Ok((depuis, Fin::Tombee(format!("envoi de la requête : {e}"))));
    }

    let mut dernier = depuis;
    let mut entete: Vec<u8> = Vec::new();
    let mut entete_lu = false;
    let mut decodeur = Decodeur::new();
    let mut octets = [0u8; 16 * 1024];
    loop {
        if arret.load(Ordering::SeqCst) {
            return Ok((dernier, Fin::Arrete));
        }
        let n = match tcp.read(&mut octets) {
            Ok(0) => return Ok((dernier, Fin::Tombee("l'hôte a fermé le flux".into()))),
            Ok(n) => n,
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted) => continue,
            Err(e) => return Ok((dernier, Fin::Tombee(e.to_string()))),
        };
        let mut corps = &octets[..n];
        if !entete_lu {
            entete.extend_from_slice(corps);
            let Some(fin) = entete.windows(4).position(|w| w == b"\r\n\r\n") else {
                if entete.len() > 64 * 1024 {
                    return Err("en-tête du flux démesuré".into());
                }
                continue;
            };
            let premiere = String::from_utf8_lossy(&entete[..fin]).lines().next().unwrap_or_default().to_string();
            if !premiere.contains(" 200") {
                let reste = String::from_utf8_lossy(&entete[fin + 4..]).into_owned();
                return Err(format!("le flux est refusé : {premiere} — {}", reste.trim()));
            }
            entete_lu = true;
            ouvert();
            let suite = entete.split_off(fin + 4);
            for t in decodeur.pousser(&suite) {
                if let Some(id) = t.id {
                    dernier = Some(id);
                }
                if f(&t) == Suite::Couper {
                    return Ok((dernier, Fin::Coupe));
                }
            }
            corps = &[];
        }
        for t in decodeur.pousser(corps) {
            if let Some(id) = t.id {
                dernier = Some(id);
            }
            if f(&t) == Suite::Couper {
                return Ok((dernier, Fin::Coupe));
            }
        }
    }
}

/// **Le flux sans fin** : se reconnecte après une chute en reprenant au dernier
/// `id`, avec une attente qui double de 200 ms à 5 s. S'arrête quand le lecteur
/// coupe, sur l'arrêt, ou sur un refus. `tombee` est prévenu de chaque chute.
pub fn suivre_sans_fin(ep: &Endpoint, sujets: &[&str], mut depuis: Option<u64>, arret: &AtomicBool, ouvert: &mut dyn FnMut(), mut tombee: impl FnMut(&str), mut f: impl FnMut(&Trame) -> Suite) -> Result<(Option<u64>, Fin), String> {
    let mut attente = Duration::from_millis(200);
    loop {
        let (dernier, fin) = suivre(ep, sujets, depuis, arret, ouvert, &mut f)?;
        if dernier != depuis {
            attente = Duration::from_millis(200);
        }
        depuis = dernier;
        match fin {
            Fin::Tombee(pourquoi) => {
                tombee(&pourquoi);
                let mut dormi = Duration::ZERO;
                while dormi < attente {
                    if arret.load(Ordering::SeqCst) {
                        return Ok((depuis, Fin::Arrete));
                    }
                    std::thread::sleep(PAS.min(attente - dormi));
                    dormi += PAS;
                }
                attente = (attente * 2).min(Duration::from_secs(5));
            }
            autre => return Ok((depuis, autre)),
        }
    }
}

#[cfg(test)]
mod new_host_tests {
    use super::*;
    use std::io::{BufRead, BufReader};

    /// A host of a test: each connection gets `(epoch, frames)` in turn; the request headers are kept.
    fn host(connexions: Vec<(&'static str, Vec<(u64, &'static str)>)>) -> (Endpoint, std::sync::mpsc::Receiver<String>) {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = l.local_addr().unwrap().to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for ((epoch, frames), c) in connexions.into_iter().zip(l.incoming().flatten()) {
                let mut r = BufReader::new(c.try_clone().unwrap());
                let mut requete = String::new();
                loop {
                    let mut line = String::new();
                    if r.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    requete.push_str(&line);
                }
                let _ = tx.send(requete);
                let mut c = c;
                let mut corps = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n: ide\n\nevent: host\ndata: {{\"epoch\":\"{epoch}\",\"pid\":1}}\n\n");
                for (id, data) in frames {
                    corps.push_str(&format!("id: {id}\ndata: {data}\n\n"));
                }
                let _ = c.write_all(corps.as_bytes());
                let _ = c.flush();
                std::thread::sleep(Duration::from_millis(100));
            }
        });
        (Endpoint { address, token: "t".into(), user_token: None }, rx)
    }

    /// **A terminal that resumes on a new host starts over** (Lucie, 24 November 2026: after a restart of the host,
    /// the terminal showed nothing more — its `Last-Event-ID` was past every id of the new host, which sent nothing).
    /// The host's epoch comes first on each connection; on another epoch the terminal says the host restarted, and
    /// reconnects without `Last-Event-ID`, so that the new host's events reach it.
    #[test]
    fn a_terminal_resuming_on_a_new_host_starts_over_and_says_so() {
        let (ep, requetes) = host(vec![
            ("A", vec![(40, r#"{"n":"avant"}"#)]),
            ("B", vec![(3, r#"{"n":"ignore"}"#)]),
            ("B", vec![(4, r#"{"n":"apres"}"#)]),
        ]);
        let arret = AtomicBool::new(false);
        let mut vus: Vec<String> = Vec::new();
        let _ = suivre_sans_fin(&ep, &["agent"], None, &arret, &mut || {}, |_| {}, |t| {
            vus.push(if t.event.is_empty() || t.event == "message" { t.data.clone() } else { format!("event:{}", t.event) });
            if t.data.contains("apres") { Suite::Couper } else { Suite::Continuer }
        });
        let r: Vec<String> = requetes.try_iter().collect();
        assert!(r[1].contains("Last-Event-ID: 40"), "the first resumption asks from the last id: {:?}", r[1]);
        assert!(r.get(2).is_some_and(|q| !q.contains("Last-Event-ID")), "on a new host, it reconnects from now, without Last-Event-ID: {r:?}");
        assert!(vus.iter().any(|v| v == "event:host_restarted"), "it says the host restarted: {vus:?}");
        assert!(vus.iter().any(|v| v.contains("apres")), "the new host's events reach it: {vus:?}");
        assert!(!vus.iter().any(|v| v.contains("ignore")), "nothing is read on the connection that asked from the old id: {vus:?}");
    }
}
