//! Acquisition coordonnée d'un fournisseur local explicitement déclaré.
//!
//! Le verrou fichier sérialise les activations entre processus ; le registre
//! rassemble les attentes du processus courant. Ni annulation d'un demandeur,
//! ni fin d'un bail n'arrêtent un fournisseur maintenu. Aucune éviction implicite.
use super::{Attache, Fin, Serveur};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions, TryLockError},
    path::{Path, PathBuf},
    sync::{atomic::{AtomicBool, Ordering}, Arc, Condvar, Mutex, OnceLock, Weak},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Coordination,
    Activation,
    Disponible,
    Echec(String),
}

struct Etat {
    phase: Phase,
    resultat: Option<Result<Arc<Attache>, String>>,
}
struct Partage {
    serveur: Serveur,
    etat: Mutex<Etat>,
    changement: Condvar,
}
type Registre = Mutex<HashMap<PathBuf, Weak<Partage>>>;
fn registre() -> &'static Registre {
    static R: OnceLock<Registre> = OnceLock::new();
    R.get_or_init(Default::default)
}

/// Une attente indépendante : son annulation ne possède pas le processus.
pub struct Demande(Arc<Partage>);

/// Tient l'acquisition pendant l'usage. Politique de cette tranche : maintenir.
/// Ce bail local n'est pas un protocole de comptage des usages distants.
pub struct Bail {
    attache: Arc<Attache>,
}
impl Bail {
    pub fn adresse(&self) -> &str { self.attache.adresse() }
}

impl Demande {
    pub fn phase(&self) -> Phase { self.0.etat.lock().unwrap().phase.clone() }

    /// `budget` borne cette attente, indépendamment du budget de l'activation.
    pub fn attendre(&self, annule: &AtomicBool, budget: Duration) -> Result<Bail, String> {
        let debut = Instant::now();
        let mut etat = self.0.etat.lock().unwrap();
        loop {
            if annule.load(Ordering::Acquire) { return Err("attente d'activation annulée".into()); }
            if let Some(resultat) = &etat.resultat {
                return resultat.as_ref().map(|a| Bail { attache: a.clone() }).map_err(Clone::clone);
            }
            let restant = budget.saturating_sub(debut.elapsed());
            if restant.is_zero() { return Err("délai d'attente de l'activation dépassé".into()); }
            etat = self.0.changement.wait_timeout(etat, restant.min(Duration::from_millis(20))).unwrap().0;
        }
    }
}

/// Une seule activation à la fois par adresse déclarée. Tous les
/// participants doivent utiliser le même dossier de coordination. Il appartient
/// au compte du fournisseur, hors worktree ; les fichiers verrou ne sont jamais
/// supprimés pendant les usages (sinon deux inodes permettraient deux verrous).
///
/// L'adresse déclarée est locale : un port fixe, ou **le port 0**. Sur le port
/// 0, le fournisseur se lie lui-même et annonce l'adresse prise
/// ([`super::LIGNE_ADRESSE`]) ; elle est retenue au dossier de coordination
/// ([`adresse_retenue`]), et une activation suivante la sonde avant de lancer.
/// Aucun port n'est réservé puis relâché : un autre processus ne peut pas le
/// prendre entre les deux (ticket du test d'activation entre processus, 13
/// novembre 2026).
pub fn acquerir(serveur: &Serveur, coordination: &Path) -> Result<Demande, String> {
    if serveur.attente.is_zero() || Instant::now().checked_add(serveur.attente).is_none() {
        return Err("budget d'activation nul ou non représentable".into());
    }
    let adresse = serveur.adresse().parse::<std::net::SocketAddr>().map_err(|_| "adresse numérique d'activation requise")?;
    if !adresse.ip().is_loopback() {
        return Err("activation gérée : adresse locale requise".into());
    }
    if serveur.fin != Fin::Laisser {
        return Err("activation partagée : politique de maintien requise".into());
    }
    if !matches!(serveur.sonde, super::Sonde::Http { .. }) {
        return Err("activation gérée : sonde d'identité HTTP requise".into());
    }
    std::fs::create_dir_all(coordination).map_err(|e| format!("coordination : {e}"))?;
    let dossier = coordination.canonicalize().map_err(|e| format!("coordination : {e}"))?;
    let cle = dossier.join(format!("{}.lock", adresse.to_string().replace(':', "_")));
    let mut r = registre().lock().unwrap();
    if let Some(existant) = r.get(&cle).and_then(Weak::upgrade) {
        if existant.serveur != *serveur {
            return Err("activation concurrente : recettes différentes pour la même adresse".into());
        }
        return Ok(Demande(existant));
    }
    let partage = Arc::new(Partage {
        serveur: serveur.clone(),
        etat: Mutex::new(Etat { phase: Phase::Coordination, resultat: None }),
        changement: Condvar::new(),
    });
    r.insert(cle.clone(), Arc::downgrade(&partage));
    let travail = partage.clone();
    std::thread::Builder::new().name("activation-fournisseur".into()).spawn(move || {
        let resultat = activer(&travail, &cle).map(Arc::new);
        let mut e = travail.etat.lock().unwrap();
        e.phase = match &resultat { Ok(_) => Phase::Disponible, Err(e) => Phase::Echec(e.clone()) };
        e.resultat = Some(resultat);
        travail.changement.notify_all();
        drop(e);
        // Une prochaine demande re-sonde le service, même si d'anciens baux
        // vivent encore. Un résultat prêt n'est pas une preuve de vie éternelle.
        registre().lock().unwrap().remove(&cle);
    }).map_err(|e| format!("fil d'activation : {e}"))?;
    Ok(Demande(partage))
}

fn activer(partage: &Partage, cle: &Path) -> Result<Attache, String> {
    let debut = Instant::now();
    let echeance = debut.checked_add(partage.serveur.attente).ok_or("budget d'activation non représentable")?;
    let fichier: File = OpenOptions::new().create(true).truncate(false).read(true).write(true)
        .open(cle).map_err(|e| format!("verrou d'activation : {e}"))?;
    loop {
        if debut.elapsed() >= partage.serveur.attente { return Err("budget de coordination dépassé".into()); }
        match fichier.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(20)),
            Err(TryLockError::Error(e)) => return Err(format!("verrou d'activation : {e}")),
        }
    }
    partage.etat.lock().unwrap().phase = Phase::Activation;
    partage.changement.notify_all();
    let mut serveur = partage.serveur.clone();
    serveur.attente = serveur.attente.saturating_sub(debut.elapsed());
    let port_nul = super::resoudre(&serveur.adresse).is_some_and(|a| a.port() == 0);
    if !port_nul {
        // Assurer re-sonde sous le verrou, puis démarre uniquement si absent.
        // La durée inclut l'attente de coordination. Le fichier libère son verrou.
        return serveur.assurer_avant(echeance).map_err(|e| e.to_string());
    }
    // Port 0 : l'adresse retenue d'abord, sondée sous le verrou ; on ne
    // s'y attache que si le fournisseur y répond. Absente, muette ou prise
    // par un autre : on lance, et le fournisseur annonce l'adresse qu'il prend.
    let memo = memo_de(cle);
    if let Some(retenue) = std::fs::read_to_string(&memo).ok().map(|a| a.trim().to_string()).filter(|a| !a.is_empty()) {
        if let Some(addr) = super::resoudre(&retenue) {
            if super::sonder_avant(addr, &serveur.sonde, echeance) == super::Etat::Repond {
                return Ok(Attache {
                    nom: serveur.nom.clone(),
                    adresse: retenue,
                    enfant: None,
                    journal: serveur.journal(),
                    demarre_par_nous: false,
                    fin: serveur.fin,
                });
            }
        }
    }
    if Instant::now() >= echeance {
        return Err("budget d'activation dépassé pendant la sonde de l'adresse retenue".into());
    }
    let attache = serveur.assurer_avant(echeance).map_err(|e| e.to_string())?;
    // Écrite puis renommée : un lecteur ne voit jamais une adresse à moitié.
    let provisoire = memo.with_extension("adresse.tmp");
    std::fs::write(&provisoire, attache.adresse()).and_then(|()| std::fs::rename(&provisoire, &memo))
        .map_err(|e| format!("adresse retenue : {e}"))?;
    Ok(attache)
}

/// Le fichier où une activation sur le port 0 retient l'adresse prise : à
/// côté de son verrou, au dossier de coordination.
fn memo_de(cle: &Path) -> PathBuf {
    cle.with_extension("adresse")
}

/// **L'adresse qu'un fournisseur activé sur le port 0 a prise**, telle que le
/// dossier de coordination la retient pour l'adresse déclarée `adresse`.
/// `None` si rien n'y est retenu (pas encore activé, ou adresse fixe).
pub fn adresse_retenue(coordination: &Path, adresse: &str) -> Option<String> {
    let dossier = coordination.canonicalize().ok()?;
    let cle = dossier.join(format!("{}.lock", adresse.parse::<std::net::SocketAddr>().ok()?.to_string().replace(':', "_")));
    std::fs::read_to_string(memo_de(&cle)).ok().map(|a| a.trim().to_string()).filter(|a| !a.is_empty())
}
