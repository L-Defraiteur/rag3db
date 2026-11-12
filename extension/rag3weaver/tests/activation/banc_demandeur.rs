//! Demandeur CPU séparé : appelle la vraie primitive commune, sans gestionnaire factice.
//! Construction : rustc --edition=2021 tests/activation/banc_demandeur.rs -o <binaire privé>.
#[allow(dead_code)]
#[path = "../../src/serveur.rs"]
mod serveur;

use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    net::TcpStream,
    path::PathBuf,
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    time::{Duration, Instant},
};

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() % 2 != 0 { return Err("arguments clé/valeur requis".into()); }
    let mut a = HashMap::new();
    for pair in args.chunks(2) {
        if a.insert(pair[0].clone(), pair[1].clone()).is_some() { return Err("argument dupliqué".into()); }
    }
    let value = |k: &str| a.get(k).cloned().ok_or_else(|| format!("argument absent : {k}"));
    let adresse = value("--adresse")?;
    let coordination = PathBuf::from(value("--coordination")?);
    let fixture = value("--fixture")?;
    let journal_service = value("--journal-service")?;
    let generation = value("--generation")?;
    let charge = value("--gate-chargement")?;
    let chauffe = value("--gate-chauffe")?;
    let annulation = PathBuf::from(value("--annulation")?);
    let fin = PathBuf::from(value("--fin")?);
    let journal = PathBuf::from(value("--journal-client")?);
    let id = value("--id")?;
    let attendu_annule = value("--attend-annulation")? == "oui";
    let ms: u64 = value("--budget-ms")?.parse().map_err(|e| format!("budget : {e}"))?;
    let budget = Duration::from_millis(ms);
    let debut = Instant::now();
    let fichier = Arc::new(Mutex::new(OpenOptions::new().create_new(true).write(true).open(&journal).map_err(|e| e.to_string())?));
    let trace = |event: &str, detail: &str| ecrire(&fichier, &id, debut, event, detail);
    let serveur = serveur::Serveur::new("rag3weaver-embeddings", &adresse, fixture)
        .args(["--adresse", &adresse, "--journal", &journal_service, "--generation", &generation,
               "--liberer-chargement", &charge, "--liberer-chauffe", &chauffe])
        .sante("/sante", "rag3weaver-embeddings")
        .journal_dans(journal.parent().ok_or("parent journal absent")?)
        .attente(budget);
    trace("demande", "debut");
    let demande = Arc::new(serveur::activation::acquerir(&serveur, &coordination)?);
    let annule = Arc::new(AtomicBool::new(false));
    let sortie = Arc::new(AtomicBool::new(false));
    let (d, c, s, f, client) = (demande.clone(), annule.clone(), sortie.clone(), fichier.clone(), id.clone());
    let observateur = std::thread::spawn(move || {
        let mut precedente = None;
        while !s.load(Ordering::Acquire) {
            let phase = d.phase();
            if precedente.as_ref() != Some(&phase) {
                ecrire(&f, &client, debut, "phase", &format!("{phase:?}"));
                precedente = Some(phase);
            }
            if annulation.is_file() { c.store(true, Ordering::Release); }
            std::thread::sleep(Duration::from_millis(5));
        }
    });
    let resultat = demande.attendre(&annule, budget);
    let verdict = match resultat {
        Ok(bail) => {
            trace("acquis", bail.adresse());
            let usage = utiliser(bail.adresse());
            trace(if usage.is_ok() { "usage_reussi" } else { "usage_echec" }, usage.as_ref().err().map(String::as_str).unwrap_or("embed factice"));
            if attendu_annule { Err("annulation attendue mais acquisition obtenue".into()) } else { usage }
        }
        Err(e) => {
            trace("attente_erreur", &e);
            if attendu_annule && e.contains("annulée") { Ok(()) } else { Err(e) }
        }
    };
    // Le processus demandeur reste vivant pendant le travail d'autrui : ce banc
    // ne prétend pas qualifier le crash du propriétaire d'une activation.
    while verdict.is_ok() && !fin.is_file() && debut.elapsed() < budget {
        std::thread::sleep(Duration::from_millis(5));
    }
    sortie.store(true, Ordering::Release);
    observateur.join().map_err(|_| "observateur arrêté anormalement")?;
    if verdict.is_ok() && !fin.is_file() { return Err("gate de fin absente dans le budget".into()); }
    trace("fin_demandeur", if verdict.is_ok() { "ok" } else { "rouge" });
    verdict
}

fn ecrire(fichier: &Arc<Mutex<File>>, id: &str, debut: Instant, event: &str, detail: &str) {
    let detail = detail.replace(['\t', '\r', '\n'], " ");
    let mut f = fichier.lock().unwrap();
    writeln!(f, "{}\t{}\t{}\t{}\t{}", debut.elapsed().as_nanos(), std::process::id(), id, event, detail).unwrap();
    f.flush().unwrap();
}

fn utiliser(adresse: &str) -> Result<(), String> {
    let mut stream = TcpStream::connect(adresse).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_secs(2))).map_err(|e| e.to_string())?;
    let body = r#"{"texts":["preuve CPU résidence"]}"#;
    write!(stream, "POST /embed HTTP/1.0\r\nHost: {adresse}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).map_err(|e| e.to_string())?;
    let mut response = String::new();
    stream.take(65536).read_to_string(&mut response).map_err(|e| e.to_string())?;
    if !response.starts_with("HTTP/1.0 200") || !response.contains("\"vectors\"") {
        return Err("réponse du fournisseur factice invalide".into());
    }
    Ok(())
}
