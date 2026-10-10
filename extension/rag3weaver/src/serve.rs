//! `serve` : un serveur HTTP local qui mène chaque requête à
//! [`Backend::route`] — la troisième façon d'entrer dans un graphe déclaré,
//! après l'outil (l'agent) et l'événement (une réaction).
//!
//! **Local seulement** : le proto n'a pas d'authentification, donc rien ne
//! s'écoute hors de la boucle locale (la frontière est celle du fichier, comme
//! pour les démons — voir [`crate::daemon::est_local`]). Les gardes fournies
//! viendront avec les secrets et les intégrations (vision §9).
//!
//! **L'arrêt rend la main** : `POST /arret` (local) ferme l'écoute et laisse
//! l'appelant fermer la base proprement (`Backend::shutdown`) — jamais un
//! `exit` qui couperait le journal.
use crate::backend::Backend;
use crate::routes::{RouteRequest, RouteResponse};
use serde_json::{Map, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Le chemin qui arrête le serveur.
pub const STOP: &str = "/arret";

/// Écoute sur `address` (boucle locale seulement) et sert jusqu'à `POST /arret`.
pub fn serve(backend: Arc<Backend>, address: &str, threads: usize) -> Result<(), String> {
    if !crate::daemon::est_local(address) {
        return Err(format!(
            "serve n'écoute que sur la boucle locale (le proto n'a pas d'authentification) : \
             {address}"
        ));
    }
    let server = tiny_http::Server::http(address).map_err(|e| format!("{address} : {e}"))?;
    serve_on(server, backend, threads);
    Ok(())
}

/// Sert sur un écouteur déjà ouvert, jusqu'à `POST /arret`.
pub fn serve_on(server: tiny_http::Server, backend: Arc<Backend>, threads: usize) {
    let server = Arc::new(server);
    let stopping = Arc::new(AtomicBool::new(false));
    let count = threads.max(1);
    let workers: Vec<_> = (0..count)
        .map(|i| {
            let (server, backend, stopping) = (server.clone(), backend.clone(), stopping.clone());
            std::thread::Builder::new()
                .name(format!("serve-{i}"))
                .spawn(move || {
                    while !stopping.load(Ordering::SeqCst) {
                        let Ok(mut request) = server.recv() else {
                            break;
                        };
                        let method = request.method().as_str().to_string();
                        let url = request.url().to_string();
                        if method == "POST" && path_of(&url) == STOP {
                            stopping.store(true, Ordering::SeqCst);
                            let _ = request.respond(response(RouteResponse::text(200, "arrêt")));
                            // Un déblocage par fil : chacun sort de son `recv`.
                            for _ in 0..count {
                                server.unblock();
                            }
                            break;
                        }
                        let content_type = request
                            .headers()
                            .iter()
                            .find(|h| h.field.equiv("Content-Type"))
                            .map(|h| h.value.as_str().to_string());
                        let mut body = Vec::new();
                        let answer = match request.as_reader().read_to_end(&mut body) {
                            Err(e) => RouteResponse::text(400, format!("corps illisible : {e}")),
                            Ok(_) => match decode(&method, &url, content_type.as_deref(), &body) {
                                Ok(decoded) => backend.route(&decoded),
                                Err(refused) => refused,
                            },
                        };
                        let _ = request.respond(response(answer));
                    }
                })
                .expect("un fil de serve")
        })
        .collect();
    for worker in workers {
        let _ = worker.join();
    }
}

fn response(answer: RouteResponse) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    tiny_http::Response::from_string(answer.body)
        .with_status_code(answer.status)
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], answer.content_type.as_bytes())
                .expect("en-tête littéral"),
        )
}

fn path_of(url: &str) -> &str {
    url.split('?').next().unwrap_or(url)
}

/// Une requête HTTP en [`RouteRequest`] : chemin et requête décodés (`%xx`,
/// `+`), corps JSON (un objet) ou formulaire (`application/x-www-form-urlencoded`,
/// des chaînes). Rien d'autre n'est lu.
pub fn decode(
    method: &str,
    url: &str,
    content_type: Option<&str>,
    body: &[u8],
) -> Result<RouteRequest, RouteResponse> {
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    let path = path
        .split('/')
        .map(|segment| percent(segment, false))
        .collect::<Vec<_>>()
        .join("/");
    let query = form(query);
    let kind = content_type
        .map(|c| c.split(';').next().unwrap_or(c).trim().to_ascii_lowercase())
        .unwrap_or_default();
    let body = if body.is_empty() {
        None
    } else if kind == "application/json" {
        match serde_json::from_slice::<Value>(body) {
            Ok(value @ Value::Object(_)) => Some(value),
            Ok(_) => return Err(RouteResponse::text(400, "le corps JSON doit être un objet")),
            Err(e) => {
                return Err(RouteResponse::text(
                    400,
                    format!("corps JSON illisible : {e}"),
                ))
            }
        }
    } else if kind == "application/x-www-form-urlencoded" {
        let text = String::from_utf8_lossy(body);
        Some(Value::Object(
            form(&text)
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect::<Map<_, _>>(),
        ))
    } else {
        return Err(RouteResponse::text(
            415,
            format!("corps « {kind} » : JSON ou formulaire seulement"),
        ));
    };
    Ok(RouteRequest {
        method: method.to_string(),
        path,
        query,
        body,
    })
}

fn form(text: &str) -> Vec<(String, String)> {
    text.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (percent(k, true), percent(v, true))
        })
        .collect()
}

/// `%xx` décodé ; `+` est une espace dans une requête ou un formulaire.
fn percent(text: &str, plus_is_space: bool) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(b) => {
                        out.push(b);
                        i += 3;
                        continue;
                    }
                    None => out.push(b'%'),
                }
            }
            b'+' if plus_is_space => out.push(b' '),
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_request_is_decoded_into_a_route_request() {
        let r = decode(
            "GET",
            "/hello/Lucie%20D?value=%7B%22n%22%3A1%7D&q=a+b",
            None,
            b"",
        )
        .unwrap();
        assert_eq!(r.path, "/hello/Lucie D");
        assert_eq!(
            r.query,
            [
                ("value".to_string(), "{\"n\":1}".to_string()),
                ("q".to_string(), "a b".to_string())
            ]
        );
        assert_eq!(r.body, None);
    }

    #[test]
    fn a_form_and_a_json_body_are_read() {
        let r = decode(
            "POST",
            "/products",
            Some("application/x-www-form-urlencoded"),
            b"name=Chaise%20%26%20table&price=12",
        )
        .unwrap();
        assert_eq!(
            r.body,
            Some(json!({"name": "Chaise & table", "price": "12"}))
        );
        let r = decode(
            "POST",
            "/p",
            Some("application/json; charset=utf-8"),
            b"{\"a\":1}",
        )
        .unwrap();
        assert_eq!(r.body, Some(json!({"a": 1})));
    }

    #[test]
    fn a_body_that_is_not_json_or_a_form_is_refused() {
        assert_eq!(
            decode("POST", "/p", Some("application/json"), b"[1]")
                .unwrap_err()
                .status,
            400
        );
        assert_eq!(
            decode("POST", "/p", Some("application/json"), b"{")
                .unwrap_err()
                .status,
            400
        );
        assert_eq!(
            decode("POST", "/p", Some("text/plain"), b"x")
                .unwrap_err()
                .status,
            415
        );
    }

    #[test]
    fn a_malformed_escape_is_kept_as_written() {
        assert_eq!(percent("100%", false), "100%");
        assert_eq!(percent("%zz", false), "%zz");
    }
}
