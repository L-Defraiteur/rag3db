//! **La boîte de l'équipier, côté terminal** (lot tty-4) : ce qui attend une
//! réponse du superviseur, et la réponse qu'on renvoie. Sans ratatui, pour que
//! la vue et le témoin e2e (par le chemin de l'hôte) jouent la même logique.
//!
//! Les formes viennent de la page de l'équipier
//! (`extension/rag3weaver/docs/07-novembre-2026-everything-declarative/01-l-equipier-agent-de-code.md`,
//! § 2) : chaque ligne a un `id` stable (`"<parent>/<task>:<n>"`) ; une
//! `question`, une `approval` (avec sa `command`) ou une `proposal` de
//! l'équipier attend tant qu'aucune ligne ne porte `reply_to: <son id>` ; la
//! réponse est un `answer` avec `reply_to`, et `approved` pour une approbation.

use serde_json::{json, Value};

/// Une ligne de l'équipier qui attend la réponse du superviseur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attente {
    pub id: String,
    /// `question`, `approval` ou `proposal`.
    pub kind: String,
    pub text: String,
    /// La commande d'une `approval`.
    pub command: Option<String>,
}

/// Ce que le terminal sait de la boîte.
#[derive(Debug, Default)]
pub struct Boite {
    attentes: Vec<Attente>,
}

impl Boite {
    /// Une ligne de la boîte, dans l'ordre où elle arrive.
    pub fn ligne(&mut self, l: &Value) {
        // Toute ligne qui y répond retire ce qu'elle vise, d'où qu'elle vienne.
        if let Some(r) = l["reply_to"].as_str() {
            self.attentes.retain(|a| a.id != r);
        }
        let kind = l["kind"].as_str().unwrap_or_default();
        if l["from"] == "teammate" && matches!(kind, "question" | "approval" | "proposal") {
            if let Some(id) = l["id"].as_str() {
                if !self.attentes.iter().any(|a| a.id == id) {
                    self.attentes.push(Attente {
                        id: id.to_string(),
                        kind: kind.to_string(),
                        text: l["text"].as_str().unwrap_or_default().to_string(),
                        command: l["command"].as_str().map(str::to_string),
                    });
                }
            }
        }
    }

    /// Ce qui attend, du plus ancien au plus récent.
    pub fn attentes(&self) -> &[Attente] {
        &self.attentes
    }

    /// Ce à quoi une réponse répond : le plus ancien en attente.
    pub fn premiere(&self) -> Option<&Attente> {
        self.attentes.first()
    }
}

/// **Le corps de la réponse** (`POST /ide/teammate/mailbox`) à `a`, de la part
/// du superviseur ; `accord` pour une approbation (`approved`).
pub fn reponse(backend: &str, task: &str, a: &Attente, texte: &str, accord: Option<bool>) -> Value {
    let mut v = json!({ "backend": backend, "task": task, "kind": "answer", "text": texte, "reply_to": a.id });
    if let Some(oui) = accord {
        v["approved"] = json!(oui);
    }
    v
}

/// **L'état d'une tâche** dans `GET /ide/teammate/list` (`running`, `waiting`,
/// `ended`, `error`), cherché par son dépôt (`parent`) et son nom.
pub fn etat_dans_la_liste(liste: &Value, parent: &str, task: &str) -> Option<String> {
    let equipiers = liste["teammates"].as_array().or_else(|| liste["result"]["teammates"].as_array())?;
    equipiers.iter().find(|t| t["parent"] == parent && t["task"] == task).and_then(|t| t["state"].as_str()).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ligne(n: usize, from: &str, kind: &str, extra: Value) -> Value {
        let mut l = json!({ "id": format!("a1/doc:{n}"), "at": n, "from": from, "kind": kind, "text": format!("ligne {n}") });
        if let (Some(o), Some(e)) = (l.as_object_mut(), extra.as_object()) {
            for (k, v) in e {
                o.insert(k.clone(), v.clone());
            }
        }
        l
    }

    #[test]
    fn what_the_teammate_asks_waits_until_a_line_replies_to_it() {
        let mut b = Boite::default();
        b.ligne(&ligne(0, "supervisor", "request", json!({})));
        b.ligne(&ligne(1, "teammate", "question", json!({ "item": "titre" })));
        b.ligne(&ligne(2, "teammate", "approval", json!({ "command": "cargo build" })));
        b.ligne(&ligne(3, "teammate", "report", json!({})));
        assert_eq!(b.attentes().iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), vec!["a1/doc:1", "a1/doc:2"], "un report n'attend pas de réponse");
        assert_eq!(b.attentes()[1].command.as_deref(), Some("cargo build"));
        b.ligne(&ligne(4, "supervisor", "answer", json!({ "reply_to": "a1/doc:1" })));
        assert_eq!(b.premiere().map(|a| a.id.as_str()), Some("a1/doc:2"), "la question répondue ne tient plus");
        b.ligne(&ligne(5, "host", "report", json!({ "reply_to": "a1/doc:2" })));
        assert!(b.attentes().is_empty(), "toute ligne qui y répond la retire");
    }

    #[test]
    fn the_answer_body_replies_to_the_item_and_carries_the_approval() {
        let a = Attente { id: "a1/doc:2".into(), kind: "approval".into(), text: "commande en attente d'accord : echo".into(), command: Some("echo".into()) };
        assert_eq!(
            reponse("a1", "doc", &a, "oui", Some(true)),
            json!({ "backend": "a1", "task": "doc", "kind": "answer", "text": "oui", "reply_to": "a1/doc:2", "approved": true })
        );
        let q = Attente { id: "a1/doc:1".into(), kind: "question".into(), text: "Quel titre ?".into(), command: None };
        let r = reponse("a1", "doc", &q, "Petit dépôt", None);
        assert_eq!(r["reply_to"], "a1/doc:1");
        assert!(r.get("approved").is_none(), "une question n'a pas d'accord : {r}");
    }

    #[test]
    fn the_state_of_a_task_is_found_by_its_repo_and_name() {
        let liste = json!({ "supervisor": "supervisor", "teammates": [
            { "task": "doc", "parent": "b2", "state": "running" },
            { "task": "doc", "parent": "a1", "state": "waiting" }
        ] });
        assert_eq!(etat_dans_la_liste(&liste, "a1", "doc").as_deref(), Some("waiting"));
        assert_eq!(etat_dans_la_liste(&liste, "a1", "autre"), None);
    }
}
