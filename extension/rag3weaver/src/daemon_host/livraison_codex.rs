//! **La livraison des lignes de boîte à Codex** (lot 5 des hooks, page 07 § 4).
//!
//! L'abonné écoute le flux de l'IDE, sujet `mailbox`. Pour chaque ligne adressée
//! à `codex:<nom>`, il résout le nom en UUID de fil et la dépose par
//! `codex queue --thread <uuid>` (`codex_threads::traiter`). Il range ensuite
//! l'issue dans la même boîte, en une ligne `delivery` (`from: host`, `to` =
//! l'expéditeur, `reply_to`, `delivered`). La boîte ne dépend pas de ce module
//! (everything declarative) : une livraison lente ne bloque pas l'équipier, et
//! l'expéditeur voit l'attente dans son `pending`. Une ligne `delivery` n'est
//! jamais relivrée, donc l'abonné ne boucle pas.

use super::DaemonHost;
use serde_json::{json, Value};
use std::io::Write;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::Duration;

/// Un écrivain qui lit les trames SSE du flux et en rend les données.
struct LecteurDeTrames {
    tampon: String,
    vers: Sender<Value>,
}

impl Write for LecteurDeTrames {
    fn write(&mut self, octets: &[u8]) -> std::io::Result<usize> {
        self.tampon.push_str(&String::from_utf8_lossy(octets));
        while let Some(i) = self.tampon.find("\n\n") {
            let trame: String = self.tampon.drain(..i + 2).collect();
            for l in trame.lines() {
                if let Some(d) = l.strip_prefix("data: ") {
                    if let Ok(v) = serde_json::from_str::<Value>(d) {
                        if self.vers.send(v).is_err() {
                            return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "abonné parti"));
                        }
                    }
                }
            }
        }
        Ok(octets.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// La ligne à livrer d'une donnée du flux, si elle est pour Codex et n'est pas
/// elle-même une livraison : `(parent, tâche, ligne)`.
pub fn a_livrer(donnee: &Value) -> Option<(String, String, Value)> {
    let e = &donnee["evenement"];
    if e["kind"] != "MailboxLine" {
        return None;
    }
    let ligne = &e["line"];
    if ligne["kind"] == "delivery" || !ligne["to"].as_str().is_some_and(|t| t.starts_with("codex:")) {
        return None;
    }
    Some((e["parent"].as_str()?.to_string(), e["task"].as_str()?.to_string(), ligne.clone()))
}

impl DaemonHost {
    /// **Sert les livraisons vers Codex** : un abonné au flux, un fil qui livre.
    pub fn servir_les_livraisons_codex(self: &Arc<Self>) {
        let (vers, depuis) = channel::<Value>();
        let mut sujets = std::collections::HashSet::new();
        sujets.insert("mailbox".to_string());
        self.ide_flux().abonner(Box::new(LecteurDeTrames { tampon: String::new(), vers }), Some(sujets), None);
        let hote = Arc::downgrade(self);
        let _ = std::thread::Builder::new().name("livraison-codex".into()).spawn(move || {
            for donnee in depuis {
                let Some((parent, tache, ligne)) = a_livrer(&donnee) else { continue };
                let Some(h) = hote.upgrade() else { break };
                h.livrer_a_codex(&parent, &tache, &ligne);
            }
        });
    }

    /// Livre une ligne à Codex et range l'issue dans la boîte de sa tâche.
    pub fn livrer_a_codex(&self, parent: &str, tache: &str, ligne: &Value) {
        let Some(d) = crate::codex_threads::traiter(ligne, &crate::codex_threads::racine(), &crate::codex_threads::binaire(), Duration::from_secs(15)) else { return };
        let extra = json!({ "to": d["to"], "reply_to": d["reply_to"], "delivered": d["delivered"], "thread": d["thread"] });
        match self.teammate_files(parent, tache).and_then(|f| f.append_with("host", "delivery", None, d["text"].as_str().unwrap_or(""), &extra)) {
            Ok(_) => eprintln!("[codex] {parent}/{tache} {} : {}", ligne["id"].as_str().unwrap_or("?"), d["text"].as_str().unwrap_or("")),
            Err(e) => eprintln!("[codex] {parent}/{tache} : la livraison ne se range pas : {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_lecteur_rend_les_donnees_des_trames_et_ignore_le_reste() {
        let (vers, depuis) = channel();
        let mut l = LecteurDeTrames { tampon: String::new(), vers };
        l.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n: ide\n\n").unwrap();
        l.write_all(b": battement\n\nid: 3\ndata: {\"sujet\":\"mailbox\",\"evenement\":{\"kind\":\"MailboxLine\"").unwrap();
        l.write_all(b",\"parent\":\"b1\",\"task\":\"doc\",\"line\":{\"id\":\"b1/doc:4\",\"to\":\"codex:essai\",\"kind\":\"message\"}}}\n\n").unwrap();
        let v: Vec<Value> = depuis.try_iter().collect();
        assert_eq!(v.len(), 1, "une trame coupée en deux écritures se recolle : {v:?}");
        let (p, t, ligne) = a_livrer(&v[0]).unwrap();
        assert_eq!((p.as_str(), t.as_str(), ligne["id"].as_str()), ("b1", "doc", Some("b1/doc:4")));
    }

    #[test]
    fn seules_les_lignes_pour_codex_et_jamais_une_livraison_se_livrent() {
        let d = |line: Value| json!({ "evenement": { "kind": "MailboxLine", "parent": "b1", "task": "doc", "line": line } });
        assert!(a_livrer(&d(json!({ "to": "claude-code:s1", "kind": "answer" }))).is_none());
        assert!(a_livrer(&d(json!({ "to": "codex:essai", "kind": "delivery" }))).is_none(), "une livraison ne se relivre pas");
        assert!(a_livrer(&d(json!({ "kind": "report" }))).is_none());
        assert!(a_livrer(&json!({ "evenement": { "kind": "StepDone" } })).is_none());
        assert!(a_livrer(&d(json!({ "to": "codex:essai", "kind": "message" }))).is_some());
    }
}
