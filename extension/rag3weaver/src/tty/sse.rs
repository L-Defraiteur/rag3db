//! **Le décodeur SSE** : des octets reçus dans n'importe quel découpage, des
//! trames complètes en sortie. On ne décode une ligne qu'entière : un caractère
//! UTF-8 coupé entre deux lectures n'est jamais abîmé.

/// Une trame SSE : son `id` s'il en a un, son genre (`message` par défaut),
/// ses données (les lignes `data:` jointes par `\n`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trame {
    pub id: Option<u64>,
    pub event: String,
    pub data: String,
}

/// L'état entre deux lectures.
#[derive(Debug, Default)]
pub struct Decodeur {
    tampon: Vec<u8>,
    id: Option<u64>,
    event: String,
    data: Vec<String>,
}

impl Decodeur {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute des octets ; rend les trames qu'ils terminent.
    pub fn pousser(&mut self, octets: &[u8]) -> Vec<Trame> {
        self.tampon.extend_from_slice(octets);
        let mut trames = Vec::new();
        while let Some(fin) = self.tampon.iter().position(|&b| b == b'\n') {
            let mut ligne: Vec<u8> = self.tampon.drain(..=fin).collect();
            ligne.pop();
            if ligne.last() == Some(&b'\r') {
                ligne.pop();
            }
            let ligne = String::from_utf8_lossy(&ligne);
            if ligne.is_empty() {
                // La ligne vide clôt une trame ; un commentaire seul n'en fait pas.
                if !self.data.is_empty() || !self.event.is_empty() {
                    let event = std::mem::take(&mut self.event);
                    trames.push(Trame {
                        id: self.id.take(),
                        event: if event.is_empty() { "message".into() } else { event },
                        data: self.data.join("\n"),
                    });
                    self.data.clear();
                }
                self.id = None;
                continue;
            }
            if ligne.starts_with(':') {
                continue;
            }
            let (champ, valeur) = match ligne.find(':') {
                Some(i) => (&ligne[..i], ligne[i + 1..].strip_prefix(' ').unwrap_or(&ligne[i + 1..])),
                None => (&ligne[..], ""),
            };
            match champ {
                "id" => self.id = valeur.trim().parse().ok(),
                "event" => self.event = valeur.to_string(),
                "data" => self.data.push(valeur.to_string()),
                _ => {}
            }
        }
        trames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLUX: &str = ": ide\n\nid: 7\ndata: {\"a\":1}\n\n: battement\n\nevent: lacune\ndata: {\"depuis\":1}\n\nid: 8\ndata: é\ndata: è\n\n";

    fn attendu() -> Vec<Trame> {
        vec![
            Trame { id: Some(7), event: "message".into(), data: "{\"a\":1}".into() },
            Trame { id: None, event: "lacune".into(), data: "{\"depuis\":1}".into() },
            Trame { id: Some(8), event: "message".into(), data: "é\nè".into() },
        ]
    }

    #[test]
    fn the_frames_come_out_whole_whatever_the_cut() {
        let octets = FLUX.as_bytes();
        for taille in 1..=octets.len() {
            let mut d = Decodeur::new();
            let mut vues = Vec::new();
            for morceau in octets.chunks(taille) {
                vues.extend(d.pousser(morceau));
            }
            assert_eq!(vues, attendu(), "découpage par {taille} octets");
        }
    }

    #[test]
    fn crlf_lines_and_comments_alone_make_no_frame() {
        let mut d = Decodeur::new();
        assert!(d.pousser(b": battement\r\n\r\n").is_empty());
        assert_eq!(d.pousser(b"id: 3\r\ndata: x\r\n\r\n"), vec![Trame { id: Some(3), event: "message".into(), data: "x".into() }]);
    }
}
