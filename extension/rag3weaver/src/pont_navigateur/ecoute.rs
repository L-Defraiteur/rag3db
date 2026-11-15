//! **L'écoute de l'hôte** : le socket Unix où le relai se connecte. Chaque connexion devient un
//! [`Pont`], rangé sous le `browserId` de son `hello` ; une nouvelle connexion du même navigateur
//! remplace l'ancienne, qui est fermée, et l'hôte le dit.

use std::path::{Path, PathBuf};

use super::pont::{Politique, Pont};

/// L'écoute ouverte ; la lâcher ferme le socket et retire son fichier.
pub struct Ecoute {
    chemin: PathBuf,
}

impl Ecoute {
    /// Ouvre le socket (dossier 0700, socket 0600). Un socket où un autre hôte écoute déjà est un
    /// refus ; un fichier mort est remplacé.
    pub fn ouvrir(chemin: &Path, _version_hote: &str, _politique: Politique) -> Result<Ecoute, String> {
        let _ = chemin;
        Err("écoute : pas encore".into())
    }

    pub fn chemin(&self) -> &Path {
        &self.chemin
    }

    /// Le pont du navigateur qui a dit ce `browserId`.
    pub fn pont(&self, _browser_id: &str) -> Option<Pont> {
        None
    }

    /// Les navigateurs connectés.
    pub fn navigateurs(&self) -> Vec<String> {
        Vec::new()
    }
}
