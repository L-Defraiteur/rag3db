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
    let mut r = agent
        .post(format!("{}{chemin}", ep.base()))
        .header("Authorization", &format!("Bearer {}", ep.token))
        .header("content-type", "application/json")
        .send(corps.to_string())
        .map_err(|e| format!("POST {chemin} : l'hôte {} ne répond pas : {e}", ep.address))?;
    let statut = r.status().as_u16();
    let texte = r.body_mut().read_to_string().map_err(|e| e.to_string())?;
    rendu(statut, &texte, &format!("POST {chemin}"))
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

/// **Le dépôt désigné** dans l'état de l'hôte (`GET /ide/etat`) : par son nom de
/// dépôt, le chemin de son manifeste (ou un dossier qui le contient), ou son id.
/// Les worktrees des tâches ne se désignent pas : on désigne le dépôt.
pub fn resoudre_depot(etat: &Value, depot: &str) -> Result<Depot, String> {
    let attaches: Vec<&Value> = etat["backends"].as_array().map(|v| v.iter().filter(|b| b["tache"].is_null()).collect()).unwrap_or_default();
    let d = depot.trim_end_matches('/');
    let exact: Vec<&Value> = attaches.iter().copied().filter(|b| b["id"] == d || b["repo"] == d || b["manifest"] == d).collect();
    let trouves = if exact.is_empty() { attaches.iter().copied().filter(|b| b["manifest"].as_str().is_some_and(|m| !d.is_empty() && m.starts_with(&format!("{d}/")))).collect() } else { exact };
    match trouves.as_slice() {
        [un] => Ok(depot_de(un)),
        [] => Err(format!(
            "aucun dépôt attaché ne s'appelle « {depot} » ; attachés : {}",
            attaches.iter().map(|b| b["repo"].as_str().or(b["manifest"].as_str()).unwrap_or("?").to_string()).collect::<Vec<_>>().join(", ")
        )),
        plusieurs => Err(format!("« {depot} » désigne plusieurs dépôts : {} — donnez le chemin du manifeste", plusieurs.iter().map(|b| b["manifest"].as_str().unwrap_or("?").to_string()).collect::<Vec<_>>().join(", "))),
    }
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

    #[test]
    fn a_repo_is_named_by_its_name_its_path_or_its_id_never_a_task_worktree() {
        let e = etat();
        assert_eq!(resoudre_depot(&e, "rag3db").unwrap().id, "b2");
        assert_eq!(resoudre_depot(&e, "/c/code-petit").unwrap().id, "a1");
        assert_eq!(resoudre_depot(&e, "/c/code-petit/backend.json").unwrap().id, "a1");
        assert_eq!(resoudre_depot(&e, "a1").unwrap().repo, "petit-depot", "un id reste lu");
        let refus = resoudre_depot(&e, "lucivy").unwrap_err();
        assert!(refus.contains("petit-depot") && refus.contains("rag3db") && !refus.contains("backend "), "{refus}");
        assert!(resoudre_depot(&e, "/c").unwrap_err().contains("plusieurs"));
    }

    #[test]
    fn the_header_names_the_repo_the_task_branch_and_the_task() {
        assert_eq!(entete(&etat(), "/t/parle/backend.json", "parle"), "petit-depot · tache/parle · tâche parle");
        assert_eq!(entete(&etat(), "/c/code-coeur/backend.json", "doc"), "rag3db · master · tâche doc");
        assert_eq!(entete(&etat(), "/inconnu", "x"), "tâche x");
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
