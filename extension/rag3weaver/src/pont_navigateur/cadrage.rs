//! **Le cadrage Native Messaging** : une longueur sur 4 octets en petit-boutiste, puis le JSON
//! UTF-8. Chrome plafonne à 1 Mio une trame de l'hôte vers l'extension.

use serde_json::Value;

/// Le plafond d'une trame de l'hôte vers l'extension (Chrome).
pub const MAX_TRAME_VERS_EXTENSION: usize = 1024 * 1024;

/// Une trame prête à écrire, ou le refus d'une trame trop grosse (rien n'est écrit).
pub fn encoder(_message: &Value) -> Result<Vec<u8>, String> {
    Err("cadrage : pas encore".into())
}

/// **Le décodeur incrémental** : il reçoit des morceaux et rend les messages complets ; un corps
/// qui n'est pas du JSON se rend en erreur, et la lecture continue à la trame suivante.
#[derive(Default)]
pub struct Decodeur {
    tampon: Vec<u8>,
}

impl Decodeur {
    pub fn pousser(&mut self, _morceau: &[u8]) -> Vec<Result<Value, String>> {
        let _ = &self.tampon;
        Vec::new()
    }
}
