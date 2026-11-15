//! **Le relai** que Chrome lance par Native Messaging (`rag3weaver-pont-navigateur`) : il se
//! connecte à l'hôte rag3weaver du poste par un socket Unix, jamais par un port, et recopie les
//! octets dans les deux sens sans lire les trames. Sa sortie standard est le canal de Chrome :
//! rien d'autre que les octets de l'hôte n'y va ; tout journal va sur l'erreur standard.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

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
pub fn valider_le_socket(_texte: &str) -> Result<PathBuf, String> {
    Err("relai : pas encore".into())
}

/// Le socket de l'hôte du poste : [`VAR_SOCKET`], sinon le dossier d'exécution de la session.
pub fn chemin_du_socket() -> Result<PathBuf, String> {
    Err("relai : pas encore".into())
}

/// **Relaie** Chrome (son entrée et sa sortie) et l'hôte (le socket), jusqu'à ce qu'un côté ferme.
/// Un hôte absent rend une erreur, et rien n'est écrit vers Chrome.
pub fn relayer<R: Read + Send + 'static, W: Write + Send + 'static>(_chrome_entree: R, _chrome_sortie: W, _socket: &Path) -> Result<Fin, String> {
    Err("relai : pas encore".into())
}
