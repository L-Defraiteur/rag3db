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
