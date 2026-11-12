//! CPU uniquement : `rustc --edition=2021 --test .../serveur.rs -o <binaire>`.
#[path = "../../src/serveur.rs"]
mod serveur;

use serveur::Serveur;
use std::{net::TcpListener, sync::{Arc, Barrier}, time::Duration};

#[test]
fn deux_demandeurs_ne_lancent_qu_un_processus() {
    let dossier = std::env::temp_dir().join(format!("activation-{}", std::process::id()));
    std::fs::create_dir_all(&dossier).unwrap();
    let compte = dossier.join("departs");
    let pid = dossier.join("pid");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let adresse = listener.local_addr().unwrap().to_string();
    let script = r#"
import http.server,os,sys,time
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
    drop(listener);
    let fils: Vec<_> = (0..2).map(|_| {
        let s = s.clone(); let b = barriere.clone();
        std::thread::spawn(move || { b.wait(); s.assurer().map(|_| ()) })
    }).collect();
    barriere.wait();
    let resultats: Vec<_> = fils.into_iter().map(|f| f.join().unwrap()).collect();
    // Arrêter uniquement notre fixture, même si le témoin est rouge.
    if let Ok(p) = std::fs::read_to_string(&pid) {
        let _ = std::process::Command::new("kill").args(["-TERM", p.trim()]).status();
    }
    let departs = std::fs::read_to_string(&compte).unwrap().lines().count();
    std::fs::remove_dir_all(&dossier).unwrap();
    assert_eq!(departs, 1, "une seule activation, résultats : {resultats:?}");
    assert!(resultats.iter().all(Result::is_ok), "{resultats:?}");
}
