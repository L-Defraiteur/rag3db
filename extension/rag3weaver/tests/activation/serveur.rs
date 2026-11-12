//! CPU uniquement : `rustc --edition=2021 --test .../serveur.rs -o <binaire>`.
#[path = "../../src/serveur.rs"]
mod serveur;

use serveur::Serveur;
use std::{net::TcpListener, sync::{Arc, Barrier}, time::Duration};

fn exercer(annuler: bool) {
    let dossier = std::env::temp_dir().join(format!("activation-{}-{}", std::process::id(), annuler));
    std::fs::create_dir_all(&dossier).unwrap();
    let compte = dossier.join("departs");
    let pid = dossier.join("pid");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let adresse = listener.local_addr().unwrap().to_string();
    let script = r#"
import http.server,os,sys,time,threading
threading.Timer(8, lambda: os._exit(0)).start()
with open(sys.argv[2],'a') as f: f.write('depart\n')
time.sleep(.3)
class H(http.server.BaseHTTPRequestHandler):
 def do_GET(self):
  self.send_response(200);self.end_headers();self.wfile.write(b'activation-fixture')
 def log_message(self,*args): pass
s=http.server.HTTPServer(('127.0.0.1',int(sys.argv[1])),H)
with open(sys.argv[3],'w') as f:f.write(str(os.getpid()))
s.serve_forever()
"#;
    let s = Serveur::new("activation-fixture", &adresse, "python3")
        .args(["-c", script, adresse.split(':').next_back().unwrap(), compte.to_str().unwrap(), pid.to_str().unwrap()])
        .sante("/sante", "activation-fixture")
        .journal_dans(&dossier).attente(Duration::from_secs(5));
    let barriere = Arc::new(Barrier::new(3));
    let annule = Arc::new(std::sync::atomic::AtomicBool::new(false));
    drop(listener);
    let fils: Vec<_> = (0..2).map(|i| {
        let s = s.clone(); let b = barriere.clone(); let dossier = dossier.clone(); let annule = annule.clone();
        std::thread::spawn(move || {
            let d = serveur::activation::acquerir(&s, &dossier).unwrap();
            b.wait();
            let non = std::sync::atomic::AtomicBool::new(false);
            d.attendre(if i == 0 { &annule } else { &non }, Duration::from_secs(5)).map(|_| ())
        })
    }).collect();
    barriere.wait();
    if annuler { annule.store(true, std::sync::atomic::Ordering::Release); }
    let resultats: Vec<_> = fils.into_iter().map(|f| f.join().unwrap()).collect();
    // Arrêter uniquement notre fixture, même si le témoin est rouge.
    if let Ok(p) = std::fs::read_to_string(&pid) {
        let _ = std::process::Command::new("kill").args(["-TERM", p.trim()]).status();
    }
    let departs = std::fs::read_to_string(&compte).unwrap().lines().count();
    std::fs::remove_dir_all(&dossier).unwrap();
    assert_eq!(departs, 1, "une seule activation, résultats : {resultats:?}");
    if annuler {
        assert_eq!(resultats.iter().filter(|r| r.is_err()).count(), 1, "{resultats:?}");
        assert!(resultats.iter().filter_map(|r| r.as_ref().err()).all(|e| e.contains("annulée")));
    } else { assert!(resultats.iter().all(Result::is_ok), "{resultats:?}"); }
}

#[test]
fn deux_demandeurs_ne_lancent_qu_un_processus() { exercer(false); }

#[test]
fn annuler_un_demandeur_ne_tue_pas_l_autre() { exercer(true); }

#[test]
fn une_sonde_qui_distille_des_octets_ne_reinitialise_pas_le_budget() {
    use std::io::{Read, Write};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let adresse = listener.local_addr().unwrap().to_string();
    let fil = std::thread::spawn(move || {
        let (mut flux, _) = listener.accept().unwrap();
        flux.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut tampon = [0; 1024]; let _ = flux.read(&mut tampon);
        for _ in 0..100 {
            if flux.write_all(b"x").is_err() { break; }
            std::thread::sleep(Duration::from_millis(30));
        }
    });
    let dossier = std::env::temp_dir().join(format!("activation-lente-{}", std::process::id()));
    let s = Serveur::new("lent", adresse, "/programme/non/lancable")
        .sante("/sante", "identite-absente").journal_dans(&dossier)
        .attente(Duration::from_millis(150));
    let debut = std::time::Instant::now();
    let d = serveur::activation::acquerir(&s, &dossier).unwrap();
    let erreur = d.attendre(&std::sync::atomic::AtomicBool::new(false), Duration::from_secs(2)).err().unwrap();
    let duree = debut.elapsed();
    fil.join().unwrap();
    std::fs::remove_dir_all(dossier).unwrap();
    assert!(duree < Duration::from_secs(1), "{duree:?} : {erreur}");
    assert!(erreur.contains("budget"), "{erreur}");
}

#[test]
fn un_lancement_entre_processus() {
    if let Ok(dossier) = std::env::var("CX_ACTIVATION_FIXTURE") {
        let dossier = std::path::PathBuf::from(dossier);
        let adresse = std::env::var("CX_ACTIVATION_ADRESSE").unwrap();
        let nom = std::env::var("CX_ACTIVATION_DEMANDEUR").unwrap();
        std::fs::write(dossier.join(nom), b"pret").unwrap();
        let limite = std::time::Instant::now() + Duration::from_secs(3);
        while !dossier.join("depart").exists() {
            assert!(std::time::Instant::now() < limite, "gate parent absente");
            std::thread::sleep(Duration::from_millis(5));
        }
        let s = Serveur::new("interprocessus", &adresse, "python3")
            .args([dossier.join("fixture.py").display().to_string(), adresse,
                dossier.display().to_string()])
            .sante("/sante", "interprocessus").journal_dans(&dossier)
            .attente(Duration::from_secs(5));
        let d = serveur::activation::acquerir(&s, &dossier).unwrap();
        d.attendre(&std::sync::atomic::AtomicBool::new(false), Duration::from_secs(6)).unwrap();
        return;
    }
    let dossier = std::env::temp_dir().join(format!("activation-processus-{}", std::process::id()));
    std::fs::create_dir_all(&dossier).unwrap();
    std::fs::write(dossier.join("fixture.py"), r#"
import http.server,os,sys,time,threading
from pathlib import Path
threading.Timer(8, lambda: os._exit(0)).start()
p=Path(sys.argv[2])
with (p/'compte').open('a') as f:f.write('depart\n')
time.sleep(.3)
class H(http.server.BaseHTTPRequestHandler):
 def do_GET(self):
  self.send_response(200);self.end_headers();self.wfile.write(b'interprocessus')
 def log_message(self,*args):pass
s=http.server.HTTPServer(('127.0.0.1',int(sys.argv[1].split(':')[1])),H)
(p/'pid').write_text(str(os.getpid()))
s.serve_forever()
"#).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let adresse = listener.local_addr().unwrap().to_string();
    drop(listener);
    let mut fils: Vec<_> = ["a", "b"].iter().map(|nom| {
        std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "un_lancement_entre_processus", "--nocapture"])
            .env("CX_ACTIVATION_FIXTURE", &dossier).env("CX_ACTIVATION_ADRESSE", &adresse)
            .env("CX_ACTIVATION_DEMANDEUR", nom).spawn().unwrap()
    }).collect();
    let limite = std::time::Instant::now() + Duration::from_secs(3);
    while !(dossier.join("a").exists() && dossier.join("b").exists()) {
        if std::time::Instant::now() >= limite { break; }
        std::thread::sleep(Duration::from_millis(5));
    }
    std::fs::write(dossier.join("depart"), b"go").unwrap();
    let resultats: Vec<_> = fils.iter_mut().map(|f| f.wait().unwrap()).collect();
    if let Ok(pid) = std::fs::read_to_string(dossier.join("pid")) {
        let _ = std::process::Command::new("kill").args(["-TERM", pid.trim()]).status();
    }
    let departs = std::fs::read_to_string(dossier.join("compte")).unwrap().lines().count();
    std::fs::remove_dir_all(dossier).unwrap();
    assert_eq!(departs, 1, "une seule création entre processus");
    assert!(resultats.iter().all(|s| s.success()), "{resultats:?}");
}
