//! Appelant de qualification privé : source déclarée, vraie acquisition et vrai embedder.
//! Le contrôleur externe tient les preuves GPU, les bornes et les groupes possédés.
#[cfg(not(feature = "daemon"))]
fn main() {
    eprintln!("banc_residence exige la feature daemon");
    std::process::exit(2);
}

#[cfg(feature = "daemon")]
fn main() {
    if let Err(e) = banc::run() {
        eprintln!("{e}");
        std::process::exit(2);
    }
}

#[cfg(feature = "daemon")]
mod banc {
    use rag3weaver::model_source::{self, ModelSource};
    use serde::Deserialize;
    use serde_json::json;
    use std::{fs::{File, OpenOptions}, io::Write, path::PathBuf,
        sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}, time::{Duration, Instant}};

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Config {
        id: String,
        source: ModelSource,
        journal: PathBuf,
        annulation: PathBuf,
        fin: PathBuf,
        budget_ms: u64,
        attend_annulation: bool,
        attend_factice: bool,
        textes: Vec<String>,
        #[serde(default)]
        gate_usage: Option<PathBuf>,
    }

    struct Trace { fichier: Mutex<File>, debut: Instant, id: String }
    impl Trace {
        fn event(&self, event: &str, details: serde_json::Value) {
            let mut f = self.fichier.lock().unwrap();
            writeln!(f, "{}", json!({"event":event,"elapsed_us":self.debut.elapsed().as_micros(),
                "pid":std::process::id(),"client":self.id,"details":details})).unwrap();
            f.flush().unwrap();
        }
    }

    pub fn run() -> Result<(), String> {
        let path = std::env::args().nth(1).ok_or("fichier de configuration privé requis")?;
        let c: Config = serde_json::from_slice(&std::fs::read(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if c.budget_ms == 0 || c.textes.is_empty() { return Err("budget et textes explicites requis".into()); }
        let trace = Arc::new(Trace { fichier: Mutex::new(OpenOptions::new().create_new(true).write(true)
            .open(&c.journal).map_err(|e| e.to_string())?), debut: Instant::now(), id: c.id });
        trace.event("demande", json!({"model":c.source.model,"dimensions":c.source.dimensions}));
        // Cette première demande rend les phases observables ; le raccordement
        // de l'embedder ci-dessous passe ensuite par le connecteur public réel.
        let demande = Arc::new(model_source::activation::demander(&c.source)?);
        let annule = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let (d, a, s, t) = (demande.clone(), annule.clone(), stop.clone(), trace.clone());
        let annulation = c.annulation.clone();
        let observateur = std::thread::spawn(move || {
            let mut avant = None;
            while !s.load(Ordering::Acquire) {
                let phase = d.phase();
                if avant.as_ref() != Some(&phase) {
                    t.event("phase", json!({"phase":format!("{phase:?}")}));
                    avant = Some(phase);
                }
                if annulation.is_file() { a.store(true, Ordering::Release); }
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        let resultat = (|| -> Result<(), String> {
            let attente = Instant::now();
            match demande.attendre(&annule, Duration::from_millis(c.budget_ms)) {
                Err(e) => {
                    trace.event("attente_erreur", json!({"cause":e,"attente_us":attente.elapsed().as_micros()}));
                    if c.attend_annulation && e.contains("annulée") { return Ok(()); }
                    return Err(e);
                }
                Ok(bail) => {
                    trace.event("acquis", json!({"adresse":bail.adresse(),"attente_us":attente.elapsed().as_micros()}));
                    if c.attend_annulation { return Err("annulation attendue mais bail obtenu".into()); }
                    if let Some(gate) = &c.gate_usage {
                        trace.event("usage_en_attente", json!({"gate":gate}));
                        while !gate.is_file() {
                            if annule.load(Ordering::Acquire) { return Err("usage annulé avant mesure".into()); }
                            if trace.debut.elapsed() >= Duration::from_millis(c.budget_ms) {
                                return Err("preuve de placement non reçue dans le budget".into());
                            }
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        trace.event("usage_debloque", json!({"gate":gate}));
                    }
                    let debut_connecteur = Instant::now();
                    let (client, origine) = model_source::connect_embedder_annulable(&c.source, &annule)?;
                    trace.event("connecteur_pret", json!({"connexion_us":debut_connecteur.elapsed().as_micros(),
                        "origine":origine.label(),"model":client.name(),"dimensions":client.dim(),"factice":client.is_mock()}));
                    if client.is_mock() != c.attend_factice || client.name() != c.source.model
                        || Some(client.dim()) != c.source.dimensions {
                        return Err("identité/factice du client différents du plan".into());
                    }
                    let debut_calcul = Instant::now();
                    let vectors = client.embed(&c.textes).map_err(|e| e.to_string())?;
                    if vectors.len() != c.textes.len() || vectors.iter().any(|v| v.len() != client.dim()
                        || v.iter().any(|x| !x.is_finite()) || v.iter().all(|x| *x == 0.0)) {
                        return Err("vecteurs invalides ou vides".into());
                    }
                    trace.event("usage_reussi", json!({"calcul_us":debut_calcul.elapsed().as_micros(),
                        "textes":vectors.len(),"dimensions":client.dim(),"factice":client.is_mock()}));
                }
            }
            Ok(())
        })();
        if let Err(e) = &resultat { trace.event("erreur", json!({"cause":e})); }
        // Aucun arrêt de fournisseur maintenu par le client ; le contrôleur n'arrête
        // que le groupe privé qu'il a lancé, après usages et preuves de fin.
        while resultat.is_ok() && !c.fin.is_file() && trace.debut.elapsed() < Duration::from_millis(c.budget_ms) {
            std::thread::sleep(Duration::from_millis(5));
        }
        stop.store(true, Ordering::Release);
        observateur.join().map_err(|_| "observateur arrêté anormalement")?;
        if resultat.is_ok() && !c.fin.is_file() { return Err("fin de scénario non reçue dans le budget".into()); }
        trace.event("fin_demandeur", json!({"ok":resultat.is_ok()}));
        resultat
    }
}
