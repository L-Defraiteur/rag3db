//! **Les appels JSON à l'hôte** : `{ok, result | error}`, en HTTP 200 même pour
//! un refus (les routes `/ide/*`).

use super::endpoint::Endpoint;
use serde_json::Value;
use std::time::Duration;

/// Un agent HTTP qui ne transforme pas un statut en erreur : l'hôte dit
/// pourquoi dans le corps.
pub fn agent() -> ureq::Agent {
    ureq::Agent::new_with_config(ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(Duration::from_secs(60))).build())
}

fn rendu(statut: u16, texte: &str, quoi: &str) -> Result<Value, String> {
    let v: Value = serde_json::from_str(texte).map_err(|e| format!("{quoi} : réponse illisible ({statut}, {e}) : {texte}"))?;
    if v["ok"] == true {
        Ok(v.get("result").cloned().unwrap_or(Value::Null))
    } else {
        Err(format!("{quoi} : {} ({statut})", v["error"].as_str().map(str::to_string).unwrap_or_else(|| v.to_string())))
    }
}

/// `POST <chemin>` avec un corps JSON.
pub fn poster(agent: &ureq::Agent, ep: &Endpoint, chemin: &str, corps: &Value) -> Result<Value, String> {
    post_with(agent, ep, chemin, corps, None)
}

fn post_with(agent: &ureq::Agent, ep: &Endpoint, chemin: &str, corps: &Value, user_token: Option<&str>) -> Result<Value, String> {
    let mut q = agent.post(format!("{}{chemin}", ep.base())).header("Authorization", &format!("Bearer {}", ep.token)).header("content-type", "application/json");
    if let Some(u) = user_token {
        q = q.header("X-Rag3weaver-User", u);
    }
    let mut r = q
        .send(corps.to_string())
        .map_err(|e| format!("POST {chemin} : l'hôte {} ne répond pas : {e}", ep.address))?;
    let statut = r.status().as_u16();
    let texte = r.body_mut().read_to_string().map_err(|e| e.to_string())?;
    rendu(statut, &texte, &format!("POST {chemin}"))
}

/// **`POST` of what the user typed at the keyboard**: as `poster`, with `X-Rag3weaver-User` when the user's token is
/// known, so that the host signs the line `by.kind = "human"` (the mode auto's human title, 21 November 2026). Never
/// for what a command line or a script sends.
pub fn post_typed(agent: &ureq::Agent, ep: &Endpoint, chemin: &str, corps: &Value) -> Result<Value, String> {
    post_with(agent, ep, chemin, corps, ep.user_token.as_deref())
}

/// **Open a folder on the host** (`POST /dossiers/ouvrir`, as the IDE's « Ouvrir un dossier »): a host session first
/// (`POST /session`), whose header the route asks for. Gives the host's answer.
pub fn open_folder(agent: &ureq::Agent, ep: &Endpoint, path: &str) -> Result<Value, String> {
    let s = poster_brut(agent, ep, "/session", &serde_json::json!({}))?;
    let session = s["session"].as_str().ok_or_else(|| format!("l'hôte n'a pas ouvert de session : {s}"))?.to_string();
    let mut r = agent
        .post(format!("{}/dossiers/ouvrir", ep.base()))
        .header("Authorization", &format!("Bearer {}", ep.token))
        .header("X-Rag3weaver-Session", &session)
        .header("content-type", "application/json")
        .send(serde_json::json!({ "path": path }).to_string())
        .map_err(|e| format!("l'hôte {} ne répond pas : {e}", ep.address))?;
    let statut = r.status().as_u16();
    let texte = r.body_mut().read_to_string().map_err(|e| e.to_string())?;
    rendu(statut, &texte, "POST /dossiers/ouvrir")
}

/// `GET <chemin et requête>`.
pub fn lire(agent: &ureq::Agent, ep: &Endpoint, chemin: &str) -> Result<Value, String> {
    let mut r = agent
        .get(format!("{}{chemin}", ep.base()))
        .header("Authorization", &format!("Bearer {}", ep.token))
        .call()
        .map_err(|e| format!("GET {chemin} : l'hôte {} ne répond pas : {e}", ep.address))?;
    let statut = r.status().as_u16();
    let texte = r.body_mut().read_to_string().map_err(|e| e.to_string())?;
    rendu(statut, &texte, &format!("GET {chemin}"))
}

/// Encode une valeur de requête (les octets hors `[A-Za-z0-9-._~]` en `%XX`).
pub fn encoder(v: &str) -> String {
    v.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// `POST <chemin>` d'une route qui rend son JSON tel quel (`/ide/describe`).
pub fn poster_brut(agent: &ureq::Agent, ep: &Endpoint, chemin: &str, corps: &Value) -> Result<Value, String> {
    let mut r = agent
        .post(format!("{}{chemin}", ep.base()))
        .header("Authorization", &format!("Bearer {}", ep.token))
        .header("content-type", "application/json")
        .send(corps.to_string())
        .map_err(|e| format!("POST {chemin} : l'hôte {} ne répond pas : {e}", ep.address))?;
    let texte = r.body_mut().read_to_string().map_err(|e| e.to_string())?;
    serde_json::from_str(&texte).map_err(|e| format!("POST {chemin} : réponse illisible ({e})"))
}

/// `GET <chemin>` d'une route qui rend son JSON tel quel (sans `{ok, result}`),
/// comme `/ide/etat`.
pub fn lire_brut(agent: &ureq::Agent, ep: &Endpoint, chemin: &str) -> Result<Value, String> {
    let mut r = agent
        .get(format!("{}{chemin}", ep.base()))
        .header("Authorization", &format!("Bearer {}", ep.token))
        .call()
        .map_err(|e| format!("GET {chemin} : l'hôte {} ne répond pas : {e}", ep.address))?;
    let statut = r.status().as_u16();
    let texte = r.body_mut().read_to_string().map_err(|e| e.to_string())?;
    if statut != 200 {
        return Err(format!("GET {chemin} : refus {statut} — {texte}"));
    }
    serde_json::from_str(&texte).map_err(|e| format!("GET {chemin} : réponse illisible ({e})"))
}

/// **Un dépôt attaché à l'hôte** : son id (pour le JSON, jamais pour l'écran),
/// son manifeste, son nom de dépôt et sa branche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Depot {
    pub id: String,
    pub manifest: String,
    pub repo: String,
    pub branch: String,
}

fn depot_de(b: &Value) -> Depot {
    let s = |k: &str| b[k].as_str().unwrap_or_default().to_string();
    Depot { id: s("id"), manifest: s("manifest"), repo: s("repo"), branch: s("branch") }
}

/// **Le dépôt que l'hôte a résolu** (`POST /depots/resoudre`, la route d'arbre : un seul nom partout, le nom court,
/// le nom complet `code-<nom>`, l'id, le manifeste, la racine du dépôt ou le dossier du manifeste), complété par son
/// entrée de l'état de l'hôte (`GET /ide/etat`, pour sa branche).
pub fn depot_resolu(etat: &Value, resolu: &Value) -> Result<Depot, String> {
    let id = resolu["id"].as_str().filter(|i| !i.is_empty()).ok_or_else(|| format!("l'hôte n'a pas dit l'id du dépôt résolu : {resolu}"))?;
    if let Some(b) = etat["backends"].as_array().and_then(|v| v.iter().find(|b| b["id"] == id && b["tache"].is_null())) {
        return Ok(depot_de(b));
    }
    let s = |k: &str| resolu[k].as_str().unwrap_or_default().to_string();
    Ok(Depot { id: id.to_string(), manifest: s("manifest"), repo: s("repo"), branch: String::new() })
}

/// **Le dépôt désigné** par son nom, résolu par l'hôte (`POST /depots/resoudre`), complété par son état.
pub fn resoudre(agent: &ureq::Agent, ep: &Endpoint, depot: &str) -> Result<Depot, String> {
    let resolu = poster(agent, ep, "/depots/resoudre", &serde_json::json!({ "name": depot }))?;
    let etat = lire_brut(agent, ep, "/ide/etat")?;
    depot_resolu(&etat, &resolu)
}

/// **L'en-tête d'un worktree** : « dépôt · branche · tâche », d'après l'état de
/// l'hôte et le manifeste du worktree.
pub fn entete(etat: &Value, manifest: &str, tache: &str) -> String {
    let b = etat["backends"].as_array().and_then(|v| v.iter().find(|b| b["manifest"] == manifest));
    let repo = b.and_then(|b| b["repo"].as_str()).unwrap_or("");
    let branche = b.and_then(|b| b["tache"]["branche"].as_str().or(b["branch"].as_str())).unwrap_or("");
    [repo.to_string(), branche.to_string(), format!("tâche {tache}")].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn etat() -> Value {
        json!({ "backends": [
            { "id": "a1", "manifest": "/c/code-petit/backend.json", "repo": "petit-depot", "branch": "master", "tache": null },
            { "id": "b2", "manifest": "/c/code-coeur/backend.json", "repo": "rag3db", "branch": "master", "tache": null },
            { "id": "t3", "manifest": "/t/parle/backend.json", "repo": "petit-depot", "branch": "tache/parle", "tache": { "nom": "parle", "parent": "a1", "branche": "tache/parle" } }
        ] })
    }

    /// **The host resolves the name, the terminal completes it** (arbre's route, 21 Nov. 2026: `code-tree add` said
    /// « code-compteur », the terminal knew only « compteur »): the resolved id is the repository, with its branch from
    /// the host's state; a resolution the state does not list still gives the repository, without a branch.
    #[test]
    fn the_repo_the_host_resolved_is_completed_by_the_state() {
        let r = json!({ "id": "a1", "repo": "petit-depot", "name": "code-petit", "manifest": "/c/code-petit/backend.json" });
        let d = depot_resolu(&etat(), &r).unwrap();
        assert_eq!((d.id.as_str(), d.repo.as_str(), d.branch.as_str(), d.manifest.as_str()), ("a1", "petit-depot", "master", "/c/code-petit/backend.json"));
        let seul = json!({ "id": "z9", "repo": "neuf", "name": "code-neuf", "manifest": "/c/code-neuf/backend.json" });
        assert_eq!(depot_resolu(&etat(), &seul).unwrap().repo, "neuf", "not in the state: the resolution alone");
        assert!(depot_resolu(&etat(), &json!({ "repo": "x" })).is_err(), "no id: refused");
    }

    #[test]
    fn the_header_names_the_repo_the_task_branch_and_the_task() {
        assert_eq!(entete(&etat(), "/t/parle/backend.json", "parle"), "petit-depot · tache/parle · tâche parle");
        assert_eq!(entete(&etat(), "/c/code-coeur/backend.json", "doc"), "rag3db · master · tâche doc");
        assert_eq!(entete(&etat(), "/inconnu", "x"), "tâche x");
    }

    /// **A conversation, not a task** (23 Nov.: the header said « tâche 23-11-06h38 » to a person who opened a
    /// conversation).
    #[test]
    fn the_header_says_conversation() {
        assert_eq!(entete(&etat(), "/inconnu", "23-11-06h38"), "conversation 23-11-06h38");
    }
}

/// Un span de coloration : octets `[debut, fin)` du texte, et sa sorte.
pub type Span = (usize, usize, String);

/// **Les spans de coloration d'un bloc**, par `POST /highlight` (les grammaires
/// tree-sitter de codeparsers, servies par l'hôte). `None` quand la langue n'est
/// pas connue des grammaires ou que l'hôte ne répond pas : le bloc reste sans
/// couleur, rien ne se perd.
pub fn surligner(agent: &ureq::Agent, ep: &Endpoint, texte: &str, langue: &str) -> Option<Vec<Span>> {
    let v = poster_brut(agent, ep, "/highlight", &serde_json::json!({ "text": texte, "language": langue })).ok()?;
    spans_de(&v, texte.len())
}

/// Les spans d'une réponse de `/highlight` (`spans: [[debut, fin, sorte, …], …]`),
/// gardés seulement s'ils tombent dans le texte.
pub fn spans_de(v: &Value, longueur: usize) -> Option<Vec<Span>> {
    let spans = v["spans"].as_array()?;
    Some(
        spans
            .iter()
            .filter_map(|s| Some((s[0].as_u64()? as usize, s[1].as_u64()? as usize, s[2].as_str()?.to_string())))
            .filter(|(a, b, _)| a < b && *b <= longueur)
            .collect(),
    )
}

#[cfg(test)]
mod tests_surligner {
    use super::*;
    use serde_json::json;

    #[test]
    fn spans_are_read_and_out_of_range_ones_dropped_and_an_unknown_language_is_none() {
        let v = json!({ "ok": true, "language": "rust", "spans": [[0, 2, "keyword", 0, 2], [3, 7, "function", 3, 7], [5, 99, "other", 5, 99]] });
        assert_eq!(spans_de(&v, 12), Some(vec![(0, 2, "keyword".into()), (3, 7, "function".into())]));
        assert_eq!(spans_de(&json!({ "ok": true, "language": null, "spans": null }), 3), None);
    }
}

#[cfg(test)]
mod tests_user_header {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    /// A one-shot host that keeps the headers of each request it answers.
    fn host(n: usize) -> (String, std::sync::mpsc::Receiver<Vec<String>>) {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = l.local_addr().unwrap().to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for c in l.incoming().take(n).flatten() {
                let mut r = BufReader::new(c.try_clone().unwrap());
                let (mut headers, mut len) = (Vec::new(), 0usize);
                loop {
                    let mut line = String::new();
                    if r.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                    headers.push(line.trim_end().to_ascii_lowercase());
                }
                let mut body = vec![0u8; len];
                let _ = std::io::Read::read_exact(&mut r, &mut body);
                let _ = tx.send(headers);
                let mut c = c;
                let _ = write!(c, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{{\"ok\":true}}");
            }
        });
        (address, rx)
    }

    /// **What is typed carries the user's token, what is not never does**: `post_typed` sends `X-Rag3weaver-User`
    /// beside the host's `Bearer`; `poster` (a `--request` given on the command line) sends only the `Bearer`; without a
    /// user's token, a typed line goes without the header.
    #[test]
    fn a_typed_line_carries_the_user_header_and_a_command_line_request_never_does() {
        let (address, rx) = host(3);
        let with = Endpoint { address: address.clone(), token: "t".into(), user_token: Some("u-123".into()) };
        let without = Endpoint { user_token: None, ..with.clone() };
        let a = agent();
        post_typed(&a, &with, "/ide/teammate/mailbox", &serde_json::json!({})).unwrap();
        let typed = rx.recv().unwrap();
        poster(&a, &with, "/ide/teammate/mailbox", &serde_json::json!({})).unwrap();
        let argument = rx.recv().unwrap();
        post_typed(&a, &without, "/ide/teammate/mailbox", &serde_json::json!({})).unwrap();
        let no_token = rx.recv().unwrap();
        assert!(typed.iter().any(|h| h == "x-rag3weaver-user: u-123"), "{typed:?}");
        assert!(typed.iter().any(|h| h == "authorization: bearer t"), "the host's token stays: {typed:?}");
        assert!(!argument.iter().any(|h| h.starts_with("x-rag3weaver-user")), "{argument:?}");
        assert!(!no_token.iter().any(|h| h.starts_with("x-rag3weaver-user")), "{no_token:?}");
    }
}
