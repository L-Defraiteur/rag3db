//! **Le relai** que Chrome lance par Native Messaging (`rag3weaver-pont-navigateur`) : il se
//! connecte à l'hôte rag3weaver du poste par un socket Unix, jamais par un port, et recopie les
//! octets dans les deux sens sans lire les trames. Sa sortie standard est le canal de Chrome :
//! rien d'autre que les octets de l'hôte n'y va ; tout journal va sur l'erreur standard.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// La variable qui désigne le socket (les témoins, un poste à part) ; sinon
/// `$XDG_RUNTIME_DIR/rag3weaver/pont-navigateur.sock`.
pub const VAR_SOCKET: &str = "RAG3WEAVER_PONT_NAVIGATEUR_SOCK";

/// Quel côté a fermé le premier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fin {
    Chrome,
    Hote,
}

/// **Le chemin d'un socket Unix**, jamais une adresse réseau : un chemin absolu, sans `host:port`.
pub fn valider_le_socket(texte: &str) -> Result<PathBuf, String> {
    let t = texte.trim();
    let refus = || format!("« {t} » : le relai ne parle qu'à un socket Unix de l'hôte du poste, par un chemin absolu, jamais à une adresse ni à un port");
    if t.is_empty() || t.contains("://") || !t.starts_with('/') {
        return Err(refus());
    }
    if t.rsplit('/').next().is_some_and(|nom| nom.contains(':')) {
        return Err(refus());
    }
    Ok(PathBuf::from(t))
}

/// Le socket de l'hôte du poste : [`VAR_SOCKET`], sinon le dossier d'exécution de la session.
pub fn chemin_du_socket() -> Result<PathBuf, String> {
    if let Some(v) = std::env::var_os(VAR_SOCKET).filter(|v| !v.is_empty()) {
        return valider_le_socket(&v.to_string_lossy());
    }
    let racine = std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()).ok_or_else(|| {
        format!("ni {VAR_SOCKET} ni XDG_RUNTIME_DIR : le relai ne sait pas où est le socket de l'hôte du poste")
    })?;
    valider_le_socket(&PathBuf::from(racine).join("rag3weaver").join("pont-navigateur.sock").to_string_lossy())
}

/// Recopie jusqu'à la fin de `de`, en vidant `vers` à chaque morceau (Chrome lit trame par trame).
fn recopier(mut de: impl Read, mut vers: impl Write) {
    let mut tampon = vec![0u8; 64 * 1024];
    loop {
        match de.read(&mut tampon) {
            Ok(0) | Err(_) => return,
            Ok(n) => {
                if vers.write_all(&tampon[..n]).and_then(|_| vers.flush()).is_err() {
                    return;
                }
            }
        }
    }
}

/// **Relaie** Chrome (son entrée et sa sortie) et l'hôte (le socket), jusqu'à ce qu'un côté ferme.
/// Un hôte absent rend une erreur, et rien n'est écrit vers Chrome.
pub fn relayer<R: Read + Send + 'static, W: Write + Send + 'static>(chrome_entree: R, chrome_sortie: W, socket: &Path) -> Result<Fin, String> {
    let hote = UnixStream::connect(socket).map_err(|e| format!("l'hôte rag3weaver du poste ne répond pas sur {} : {e}", socket.display()))?;
    let (vers_hote, de_hote) = (hote.try_clone().map_err(|e| e.to_string())?, hote.try_clone().map_err(|e| e.to_string())?);
    let (fin, attente) = mpsc::channel();
    let f = fin.clone();
    std::thread::spawn(move || {
        recopier(chrome_entree, vers_hote);
        let _ = f.send(Fin::Chrome);
    });
    std::thread::spawn(move || {
        recopier(de_hote, chrome_sortie);
        let _ = fin.send(Fin::Hote);
    });
    let premier = attente.recv().map_err(|e| e.to_string())?;
    // L'autre sens s'arrête aussi : le socket fermé débloque sa lecture.
    let _ = hote.shutdown(std::net::Shutdown::Both);
    Ok(premier)
}
