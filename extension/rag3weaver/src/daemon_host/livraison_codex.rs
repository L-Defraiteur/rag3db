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

    /// La boîte de l'hôte, jointe en direct (le même trait que le client HTTP).
    struct BoiteDirecte(Arc<DaemonHost>);
    impl crate::mail::Boite for BoiteDirecte {
        fn pending(&self, pour: &str) -> Result<Vec<crate::mail::Item>, String> {
            Ok(self.0.teammate_pending(pour)["items"].as_array().map(|a| a.iter().filter_map(crate::mail::Item::depuis).collect()).unwrap_or_default())
        }
        fn seen(&self, pour: &str, ids: &[String]) -> Result<(), String> {
            self.0.teammate_seen(pour, ids).map(|_| ())
        }
        fn poster(&self, l: Value) -> Result<Value, String> {
            let s = |k: &str| l[k].as_str().unwrap_or("").to_string();
            self.0.post_to_teammate(&s("parent"), &s("task"), &s("from"), &s("kind"), l["item"].as_str(), &s("text"), &json!({ "to": l["to"], "reply_to": l["reply_to"], "approved": l["approved"] }))
        }
        fn equipiers(&self, sup: &str) -> Result<Vec<Value>, String> {
            Ok(self.0.teammate_list(sup)["teammates"].as_array().cloned().unwrap_or_default())
        }
    }

    /// **Le pont par le chemin de l'hôte** (c7) : une session Claude écrit à un fil
    /// Codex dans la boîte d'une tâche ; l'équipier ne part pas, la livraison part
    /// par `codex queue` (un faux codex) ; Codex répond par `mail reply`, la réponse
    /// est dans le `pending` de la session Claude avec son auteur, puis sort par
    /// `seen` ; un message sans destinataire, lui, lance l'équipier.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn un_message_de_claude_a_codex_se_livre_sans_lancer_l_equipier_et_la_reponse_revient() {
        let d = tempfile::tempdir().unwrap();
        let (_depot, manifeste) = super::super::chat_qui_code::depot_inscrit(d.path(), true);
        let sans_modele: super::super::chat::ChatModel = Arc::new(|_| Err("aucun modèle dans ce témoin : l'équipier ne doit pas partir".to_string()));
        let host = Arc::new(super::super::chat_qui_code::hote(d.path(), sans_modele));
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
        host.ouvrir_une_tache(&h.id, "pont").expect("la tâche s'ouvre");
        // Un fil Codex nommé « essai », et un faux codex qui range ses arguments.
        const U: &str = "01a1b6af-70d1-7b93-a9aa-0811cba4fc62";
        let codex = d.path().join("codex");
        std::fs::create_dir_all(&codex).unwrap();
        std::fs::write(codex.join("session_index.jsonl"), format!("{{\"id\":\"{U}\",\"thread_name\":\"essai\",\"updated_at\":\"2026-11-07T19:00:00Z\"}}\n")).unwrap();
        let trace = d.path().join("trace");
        let bin = d.path().join("faux-codex");
        std::fs::write(&bin, format!("#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\" >> {}; done\n", trace.display())).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::env::set_var("CODEX_HOME", &codex);
        std::env::set_var("RAG3WEAVER_CODEX_BIN", &bin);
        host.servir_les_livraisons_codex();
        let boite = BoiteDirecte(host.clone());

        // Claude → Codex, dans la boîte de la tâche : rangé, pas d'équipier.
        let r = crate::mail::Boite::poster(&boite, json!({ "parent": h.id, "task": "pont", "from": "claude-code:s1", "to": "codex:essai", "kind": "message", "text": "relis le diff du pont" })).expect("la boîte accepte un message");
        assert_eq!(r["started"], false, "un message pour Codex ne lance pas l'équipier : {r}");
        assert_eq!(r["why"], "not_addressed", "{r}");
        let id = r["line"]["id"].as_str().unwrap().to_string();
        assert!(!host.teammate_running(&h.id, "pont"));
        // La livraison part, au fil par son UUID, et se range dans la boîte.
        let files = host.teammate_files(&h.id, "pont").unwrap();
        let fin = std::time::Instant::now() + Duration::from_secs(10);
        let livraison = loop {
            if let Some(l) = files.mailbox().into_iter().find(|l| l["kind"] == "delivery" && l["reply_to"] == id.as_str()) {
                break l;
            }
            assert!(std::time::Instant::now() < fin, "aucune livraison en 10 s : {:?}", files.mailbox());
            std::thread::sleep(Duration::from_millis(50));
        };
        assert_eq!((livraison["delivered"].as_bool(), livraison["to"].as_str(), livraison["from"].as_str()), (Some(true), Some("claude-code:s1"), Some("host")), "{livraison}");
        let args = std::fs::read_to_string(&trace).unwrap();
        assert!(args.starts_with(&format!("queue\n--thread\n{U}\n--message\n")) && args.contains("relis le diff du pont") && args.contains(&format!("mail reply --from codex:essai --to {id}")), "{args}");
        assert!(boite.pending("codex:essai").unwrap().iter().any(|i| i.id == id), "le message attend Codex dans pending");

        // Codex répond par la commande que le message lui a donnée.
        let mut sortie = Vec::new();
        let args: Vec<String> = ["reply", "--from", "codex:essai", "--to", &id, "c'est", "relu"].iter().map(|s| s.to_string()).collect();
        let b2 = BoiteDirecte(host.clone());
        let code = crate::mail::commande_avec(&args, &mut sortie, &move |_| Ok(Box::new(BoiteDirecte(b2.0.clone())) as Box<dyn crate::mail::Boite>)).unwrap();
        assert_eq!(code, 0, "{}", String::from_utf8_lossy(&sortie));
        assert!(!host.teammate_running(&h.id, "pont"), "une réponse à une ligne de Claude ne lance pas l'équipier");
        let pour_claude = boite.pending("claude-code:s1").unwrap();
        let reponse = pour_claude.iter().find(|i| i.kind == "answer").unwrap_or_else(|| panic!("la réponse attend la session Claude : {pour_claude:?}"));
        assert_eq!((reponse.text.as_str(), reponse.from.as_deref()), ("c'est relu", Some("codex:essai")));
        // Elle ne demande rien : seen la fait sortir.
        boite.seen("claude-code:s1", &[reponse.id.clone()]).unwrap();
        assert!(!boite.pending("claude-code:s1").unwrap().iter().any(|i| i.id == reponse.id));

        // Un message sans destinataire est pour l'équipier : il part.
        let r = crate::mail::Boite::poster(&boite, json!({ "parent": h.id, "task": "pont", "from": "supervisor", "kind": "message", "text": "ajoute une ligne au README" })).unwrap();
        assert_eq!(r["started"], true, "{r}");
        std::env::remove_var("CODEX_HOME");
        std::env::remove_var("RAG3WEAVER_CODEX_BIN");
    }

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
