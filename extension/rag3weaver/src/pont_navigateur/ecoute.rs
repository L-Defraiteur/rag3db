//! **L'écoute de l'hôte** : le socket Unix où le relai se connecte. Chaque connexion devient un
//! [`Pont`], rangé sous le `browserId` de son `hello` ; une nouvelle connexion du même navigateur
//! remplace l'ancienne, qui est fermée, et l'hôte le dit.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::pont::{Politique, Pont};

/// Le `hello` d'une connexion s'attend au plus ce temps ; sans lui, elle est fermée.
const ATTENTE_DU_HELLO: Duration = Duration::from_secs(10);

type Ponts = Arc<Mutex<BTreeMap<String, (Pont, UnixStream)>>>;

/// L'écoute ouverte ; la lâcher ferme le socket et retire son fichier.
pub struct Ecoute {
    chemin: PathBuf,
    ponts: Ponts,
    arret: Arc<AtomicBool>,
}

impl Ecoute {
    /// Ouvre le socket (dossier 0700, socket 0600). Un socket où un autre hôte écoute déjà est un
    /// refus ; un fichier mort est remplacé.
    pub fn ouvrir(chemin: &Path, version_hote: &str, politique: Politique) -> Result<Ecoute, String> {
        let dossier = chemin.parent().ok_or_else(|| format!("{} : pas de dossier", chemin.display()))?;
        std::fs::create_dir_all(dossier).map_err(|e| format!("{} : {e}", dossier.display()))?;
        std::fs::set_permissions(dossier, std::fs::Permissions::from_mode(0o700)).map_err(|e| format!("{} : {e}", dossier.display()))?;
        if chemin.exists() {
            if UnixStream::connect(chemin).is_ok() {
                return Err(format!("{} : un autre hôte écoute déjà ce socket", chemin.display()));
            }
            std::fs::remove_file(chemin).map_err(|e| format!("{} : le socket mort ne s'efface pas : {e}", chemin.display()))?;
        }
        let ecoute = UnixListener::bind(chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        std::fs::set_permissions(chemin, std::fs::Permissions::from_mode(0o600)).map_err(|e| format!("{} : {e}", chemin.display()))?;
        let ponts: Ponts = Default::default();
        let arret = Arc::new(AtomicBool::new(false));
        let (p, a, version) = (ponts.clone(), arret.clone(), version_hote.to_string());
        std::thread::Builder::new()
            .name("pont-navigateur-ecoute".into())
            .spawn(move || {
                for connexion in ecoute.incoming() {
                    if a.load(Ordering::SeqCst) {
                        return;
                    }
                    let Ok(flux) = connexion else { continue };
                    let (p, version, politique) = (p.clone(), version.clone(), politique.clone());
                    std::thread::spawn(move || accueillir(flux, &p, &version, politique));
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Ecoute { chemin: chemin.to_path_buf(), ponts, arret })
    }

    pub fn chemin(&self) -> &Path {
        &self.chemin
    }

    /// Le pont du navigateur qui a dit ce `browserId`.
    pub fn pont(&self, browser_id: &str) -> Option<Pont> {
        self.ponts.lock().unwrap_or_else(|p| p.into_inner()).get(browser_id).map(|(p, _)| p.clone())
    }

    /// Les navigateurs connectés.
    pub fn navigateurs(&self) -> Vec<String> {
        self.ponts.lock().unwrap_or_else(|p| p.into_inner()).iter().filter(|(_, (p, _))| p.etat().connectee).map(|(k, _)| k.clone()).collect()
    }
}

/// Une connexion : son pont, son `hello`, puis sa place (l'ancienne du même navigateur fermée).
fn accueillir(flux: UnixStream, ponts: &Ponts, version: &str, politique: Politique) {
    let (Ok(lecteur), Ok(garde)) = (flux.try_clone(), flux.try_clone()) else { return };
    let pont = Pont::demarrer(lecteur, flux, version, politique);
    if !pont.attendre_hello(ATTENTE_DU_HELLO) {
        eprintln!("[pont navigateur] une connexion sans hello après {} s est fermée", ATTENTE_DU_HELLO.as_secs());
        let _ = garde.shutdown(std::net::Shutdown::Both);
        return;
    }
    let Some(id) = pont.etat().browser_id else { return };
    let ancienne = ponts.lock().unwrap_or_else(|p| p.into_inner()).insert(id.clone(), (pont, garde));
    if let Some((_, flux)) = ancienne {
        eprintln!("[pont navigateur] le navigateur {id} s'est reconnecté : l'ancienne connexion est fermée");
        let _ = flux.shutdown(std::net::Shutdown::Both);
    }
}

impl Drop for Ecoute {
    fn drop(&mut self) {
        self.arret.store(true, Ordering::SeqCst);
        // Débloque l'`accept` en cours, qui voit l'arrêt.
        let _ = UnixStream::connect(&self.chemin);
        for (_, (_, flux)) in self.ponts.lock().unwrap_or_else(|p| p.into_inner()).iter() {
            let _ = flux.shutdown(std::net::Shutdown::Both);
        }
        let _ = std::fs::remove_file(&self.chemin);
    }
}
