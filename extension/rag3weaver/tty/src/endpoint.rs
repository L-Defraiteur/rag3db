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
    /// **The user's token** (`user/user_token` beside `daemon.json`, the mode auto's human title, 21 November 2026):
    /// sent as `X-Rag3weaver-User` only with what is typed at the keyboard. Absent, typed lines go without it.
    pub user_token: Option<String>,
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
        Ok(Self { address: sans_schema(&f.address), token: f.token, user_token: None })
    }

    pub fn depuis_fichier(chemin: &Path) -> Result<Self, String> {
        let octets = std::fs::read(chemin).map_err(|e| format!("{} : {e} (l'hôte tourne-t-il ?)", chemin.display()))?;
        let mut ep = Self::depuis_json(&octets)?;
        // The user's token lives beside daemon.json, in `user/` (0700, hidden from the agents under the sandbox).
        ep.user_token = chemin.parent().and_then(|d| std::fs::read_to_string(d.join("user").join("user_token")).ok()).map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        Ok(ep)
    }

    /// La sortie d'une commande qui imprime un `daemon.json` (par exemple
    /// `ssh luciepc cat .local/share/rag3weaver/daemon.json`).
    pub fn depuis_commande(commande: &str) -> Result<Self, String> {
        Self::depuis_json(&run(commande)?)
    }

    /// Le `daemon.json` de cet utilisateur, au même endroit que l'hôte l'écrit
    /// (`RAG3WEAVER_DAEMON_DIR` l'emporte, comme chez l'hôte).
    pub fn par_defaut() -> Result<Self, String> {
        Self::depuis_fichier(&fichier_par_defaut()?)
    }

    /// The user's token read by a command (`--user-token-cmd`, for a host on another machine).
    pub fn with_user_token_from_command(mut self, command: &str) -> Result<Self, String> {
        let t = String::from_utf8_lossy(&run(command)?).trim().to_string();
        self.user_token = (!t.is_empty()).then_some(t);
        Ok(self)
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

/// What `commande` prints, or why it fails.
fn run(commande: &str) -> Result<Vec<u8>, String> {
    #[cfg(windows)]
    let sortie = std::process::Command::new("cmd").args(["/C", commande]).output();
    #[cfg(not(windows))]
    let sortie = std::process::Command::new("sh").args(["-c", commande]).output();
    let sortie = sortie.map_err(|e| format!("« {commande} » ne se lance pas : {e}"))?;
    if !sortie.status.success() {
        return Err(format!("« {commande} » : {} — {}", sortie.status, String::from_utf8_lossy(&sortie.stderr).trim()));
    }
    Ok(sortie.stdout)
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
        assert_eq!(e, Endpoint { address: "127.0.0.1:7970".into(), token: "t".into(), user_token: None });
        assert_eq!(e.base(), "http://127.0.0.1:7970");
        assert_eq!(e.a_l_adresse("localhost:17970").address, "localhost:17970");
    }

    /// **The user's token is read beside `daemon.json`** (`user/user_token`, trimmed), and kept through a tunnel;
    /// without the file, there is none and the host is still reached.
    #[test]
    fn the_user_token_is_read_beside_daemon_json_and_absent_without_its_file() {
        let d = std::env::temp_dir().join(format!("tty-user-token-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("user")).unwrap();
        std::fs::write(d.join("daemon.json"), r#"{"address":"127.0.0.1:7970","token":"t"}"#).unwrap();
        assert_eq!(Endpoint::depuis_fichier(&d.join("daemon.json")).unwrap().user_token, None, "no file, no token");
        std::fs::write(d.join("user").join("user_token"), "u-123\n").unwrap();
        let e = Endpoint::depuis_fichier(&d.join("daemon.json")).unwrap();
        assert_eq!(e.user_token.as_deref(), Some("u-123"));
        assert_eq!(e.a_l_adresse("localhost:17970").user_token.as_deref(), Some("u-123"), "a tunnel keeps it");
        let by_command = Endpoint::depuis_json(br#"{"address":"a:1","token":"t"}"#).unwrap().with_user_token_from_command("printf 'u-456\\n'").unwrap();
        assert_eq!(by_command.user_token.as_deref(), Some("u-456"), "read by a command for a remote host");
        let _ = std::fs::remove_dir_all(&d);
    }
}
