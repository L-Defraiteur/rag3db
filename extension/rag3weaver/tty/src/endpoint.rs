//! **Où est l'hôte, et son jeton.** Le terminal tourne là où est l'hôte
//! (`daemon.json` local), ou à côté de lui sur une autre machine : on lit
//! alors le `daemon.json` distant par une commande (`ssh luciepc cat …`) et on
//! force l'adresse du tunnel (`ssh -L`). Rien ici n'ouvre le moteur.

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// L'adresse (`hôte:port`, sans schéma) et le jeton.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub address: String,
    pub token: String,
}

#[derive(Deserialize)]
struct Fiche {
    address: String,
    token: String,
}

impl Endpoint {
    /// Le contenu d'un `daemon.json`.
    pub fn depuis_json(octets: &[u8]) -> Result<Self, String> {
        let f: Fiche = serde_json::from_slice(octets).map_err(|e| format!("daemon.json illisible : {e}"))?;
        Ok(Self { address: sans_schema(&f.address), token: f.token })
    }

    pub fn depuis_fichier(chemin: &Path) -> Result<Self, String> {
        let octets = std::fs::read(chemin).map_err(|e| format!("{} : {e} (l'hôte tourne-t-il ?)", chemin.display()))?;
        Self::depuis_json(&octets)
    }

    /// La sortie d'une commande qui imprime un `daemon.json` (par exemple
    /// `ssh luciepc cat .local/share/rag3weaver/daemon.json`).
    pub fn depuis_commande(commande: &str) -> Result<Self, String> {
        #[cfg(windows)]
        let sortie = std::process::Command::new("cmd").args(["/C", commande]).output();
        #[cfg(not(windows))]
        let sortie = std::process::Command::new("sh").args(["-c", commande]).output();
        let sortie = sortie.map_err(|e| format!("« {commande} » ne se lance pas : {e}"))?;
        if !sortie.status.success() {
            return Err(format!("« {commande} » : {} — {}", sortie.status, String::from_utf8_lossy(&sortie.stderr).trim()));
        }
        Self::depuis_json(&sortie.stdout)
    }

    /// Le `daemon.json` de cet utilisateur, au même endroit que l'hôte l'écrit
    /// (`RAG3WEAVER_DAEMON_DIR` l'emporte, comme chez l'hôte).
    pub fn par_defaut() -> Result<Self, String> {
        Self::depuis_fichier(&fichier_par_defaut()?)
    }

    /// Une autre adresse, même jeton : l'extrémité locale d'un tunnel.
    pub fn a_l_adresse(mut self, address: &str) -> Self {
        self.address = sans_schema(address);
        self
    }

    /// La base des URL des appels JSON.
    pub fn base(&self) -> String {
        format!("http://{}", self.address)
    }
}

fn sans_schema(a: &str) -> String {
    a.trim().trim_start_matches("http://").trim_end_matches('/').to_string()
}

/// Le chemin du `daemon.json` (le même que `daemon_lifecycle`, qui vit sous le
/// moteur : ce binaire ne le charge pas).
pub fn fichier_par_defaut() -> Result<PathBuf, String> {
    if let Some(d) = std::env::var_os("RAG3WEAVER_DAEMON_DIR").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(d).join("daemon.json"));
    }
    let absolu = |v: &str| std::env::var_os(v).map(PathBuf::from).filter(|p| p.is_absolute());
    #[cfg(windows)]
    let racine = absolu("LOCALAPPDATA").or_else(|| absolu("USERPROFILE").map(|p| p.join("AppData").join("Local"))).map(|p| p.join("rag3weaver"));
    #[cfg(not(windows))]
    let racine = absolu("HOME").map(|p| p.join(".local").join("share").join("rag3weaver"));
    racine.map(|r| r.join("daemon.json")).ok_or_else(|| "ni RAG3WEAVER_DAEMON_DIR ni dossier personnel absolu : où est l'hôte ?".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_daemon_json_gives_address_and_token_and_a_tunnel_keeps_the_token() {
        let e = Endpoint::depuis_json(br#"{"address":"http://127.0.0.1:7970/","pid":1,"token":"t","build":"b"}"#).unwrap();
        assert_eq!(e, Endpoint { address: "127.0.0.1:7970".into(), token: "t".into() });
        assert_eq!(e.base(), "http://127.0.0.1:7970");
        assert_eq!(e.a_l_adresse("localhost:17970").address, "localhost:17970");
    }
}
