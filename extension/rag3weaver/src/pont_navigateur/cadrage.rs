//! **Le cadrage Native Messaging** : une longueur sur 4 octets en petit-boutiste, puis le JSON
//! UTF-8. Chrome plafonne à 1 Mio une trame de l'hôte vers l'extension.

use serde_json::Value;

/// Le plafond d'une trame de l'hôte vers l'extension (Chrome).
pub const MAX_TRAME_VERS_EXTENSION: usize = 1024 * 1024;
/// Le plafond d'une trame lue (Chrome : 64 Mio de l'extension vers l'hôte) : au-delà, le canal est
/// jugé rompu plutôt que d'allouer sans fin.
pub const MAX_TRAME_LUE: usize = 64 * 1024 * 1024;

/// Une trame prête à écrire, ou le refus d'une trame trop grosse (rien n'est écrit).
pub fn encoder(message: &Value) -> Result<Vec<u8>, String> {
    let corps = serde_json::to_vec(message).map_err(|e| format!("trame illisible : {e}"))?;
    if corps.len() > MAX_TRAME_VERS_EXTENSION {
        return Err(format!("trame trop grosse pour l'extension : {} octets (au plus {MAX_TRAME_VERS_EXTENSION}, 1 Mio)", corps.len()));
    }
    let mut trame = Vec::with_capacity(4 + corps.len());
    trame.extend((corps.len() as u32).to_le_bytes());
    trame.extend(corps);
    Ok(trame)
}

/// **Le décodeur incrémental** : il reçoit des morceaux et rend les messages complets ; un corps
/// qui n'est pas du JSON se rend en erreur, et la lecture continue à la trame suivante.
#[derive(Default)]
pub struct Decodeur {
    tampon: Vec<u8>,
}

impl Decodeur {
    pub fn pousser(&mut self, morceau: &[u8]) -> Vec<Result<Value, String>> {
        self.tampon.extend_from_slice(morceau);
        let mut lus = Vec::new();
        let mut debut = 0;
        while self.tampon.len() - debut >= 4 {
            let n = u32::from_le_bytes(self.tampon[debut..debut + 4].try_into().expect("quatre octets")) as usize;
            if n > MAX_TRAME_LUE {
                lus.push(Err(format!("trame annoncée de {n} octets, au-delà de {MAX_TRAME_LUE} : le canal est rompu")));
                self.tampon.clear();
                return lus;
            }
            if self.tampon.len() - debut < 4 + n {
                break;
            }
            let corps = &self.tampon[debut + 4..debut + 4 + n];
            lus.push(serde_json::from_slice(corps).map_err(|e| {
                let apercu: String = String::from_utf8_lossy(corps).chars().take(200).collect();
                format!("trame illisible : {e} ({apercu})")
            }));
            debut += 4 + n;
        }
        self.tampon.drain(..debut);
        lus
    }
}
