//! Serveur SSE local, écrit à la main, **sans aucune dépendance** (std only).
//! Aucun appel réseau réel, aucun secret : il rejoue des trames enregistrées.
//!
//! Partagé par `openai_llm_sse.rs` et `openai_llm_luciole.rs` via `mod common`.
//! `dead_code` est permis : chaque binaire de test n'en utilise qu'une part.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};

pub struct FakeServer {
    pub url: String,
    /// Corps JSON de la requête reçue (pour vérifier ce qu'on a envoyé).
    pub request: mpsc::Receiver<String>,
    /// Nombre de trames que le serveur a réussi à écrire avant que le client
    /// ne ferme la socket — c'est la preuve que `Flow::Stop` coupe vraiment.
    pub written: Arc<AtomicUsize>,
    _handle: std::thread::JoinHandle<()>,
}

impl FakeServer {
    /// Répond une erreur HTTP au lieu d'un flux — pour vérifier le mapping
    /// vers `LlmError` et qu'aucun secret ne fuite dans le message.
    pub fn start_error(status: u16, body: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        let written = Arc::new(AtomicUsize::new(0));
        let body = body.to_string();
        let handle = std::thread::spawn(move || {
            let Ok((mut sock, _)) = listener.accept() else { return };
            let req = read_request(&mut sock);
            let _ = tx.send(req);
            let _ = sock.write_all(
                format!(
                    "HTTP/1.1 {status} Error\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            );
            let _ = sock.flush();
        });
        Self { url: format!("http://127.0.0.1:{port}/v1"), request: rx, written, _handle: handle }
    }

    /// `frames` : les lignes `data: ...` à émettre, dans l'ordre.
    /// `repeat_last` : réémet la dernière trame à l'infini (test d'annulation).
    pub fn start(frames: Vec<String>, repeat_last: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        let written = Arc::new(AtomicUsize::new(0));
        let w = Arc::clone(&written);

        let handle = std::thread::spawn(move || {
            let Ok((mut sock, _)) = listener.accept() else { return };
            let body = read_request(&mut sock);
            let _ = tx.send(body);
            let _ = sock.write_all(
                b"HTTP/1.1 200 OK\r\n\
                  Content-Type: text/event-stream\r\n\
                  Cache-Control: no-cache\r\n\
                  Connection: close\r\n\r\n",
            );
            let _ = sock.flush();
            let mut send = |f: &str| -> bool {
                // Une trame SSE = "data: <json>\n\n".
                let ok = sock.write_all(format!("data: {f}\n\n").as_bytes()).is_ok()
                    && sock.flush().is_ok();
                if ok {
                    w.fetch_add(1, Ordering::SeqCst);
                }
                ok
            };
            for f in &frames {
                if !send(f) {
                    return;
                }
                // Laisse le client consommer : sans ça tout tient dans le
                // tampon de la socket et l'annulation ne se voit pas.
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            if repeat_last {
                let last = frames.last().cloned().unwrap_or_default();
                for _ in 0..100_000 {
                    if !send(&last) {
                        return; // broken pipe : le client a coupé
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
            let _ = send("[DONE]");
        });

        Self {
            url: format!("http://127.0.0.1:{port}/v1"),
            request: rx,
            written,
            // Détaché : ne JAMAIS joindre — si aucun client ne se connecte,
            // le thread dort dans `accept()` et le join pendrait pour
            // toujours (c'est exactement ce qui est arrivé au premier jet).
            _handle: handle,
        }
    }
}

/// Lit une requête HTTP/1.1 minimale et rend son corps.
fn read_request(sock: &mut TcpStream) -> String {
    let mut r = BufReader::new(sock.try_clone().expect("clone"));
    let mut len = 0usize;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let t = line.trim();
        if t.is_empty() {
            break;
        }
        if let Some(v) = t.to_ascii_lowercase().strip_prefix("content-length:") {
            len = v.trim().parse().unwrap_or(0);
        }
    }
    let mut buf = vec![0u8; len];
    let _ = r.read_exact(&mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

// ── Trames enregistrées (forme OpenAI / Vertex openapi / AI Studio) ─────

pub fn text_frames() -> Vec<String> {
    vec![
        r#"{"id":"c1","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}"#.into(),
        r#"{"id":"c1","choices":[{"index":0,"delta":{"content":"Bonjour"},"finish_reason":null}]}"#.into(),
        r#"{"id":"c1","choices":[{"index":0,"delta":{"content":" le"},"finish_reason":null}]}"#.into(),
        r#"{"id":"c1","choices":[{"index":0,"delta":{"content":" monde"},"finish_reason":null}]}"#.into(),
        r#"{"id":"c1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#.into(),
        // Chunk final `usage` : n'existe QUE si stream_options.include_usage.
        r#"{"id":"c1","choices":[],"usage":{"prompt_tokens":11,"completion_tokens":3,"total_tokens":14}}"#.into(),
    ]
}

/// Un appel d'outil en deltas : `id`/`name` une seule fois, `arguments`
/// fragmenté — le piège que tout client doit gérer.
pub fn tool_frames() -> Vec<String> {
    vec![
        r#"{"choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"call_a1","type":"function","function":{"name":"KBQuerySourceNode","arguments":""}}]},"finish_reason":null}]}"#.into(),
        r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"kb_"}}]},"finish_reason":null}]}"#.into(),
        r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"name\":\"docs\","}}]},"finish_reason":null}]}"#.into(),
        r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"query\":\"luciole\"}"}}]},"finish_reason":null}]}"#.into(),
        r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#.into(),
        r#"{"choices":[],"usage":{"prompt_tokens":180,"completion_tokens":24,"total_tokens":204}}"#.into(),
    ]
}

pub fn length_frames() -> Vec<String> {
    vec![
        r#"{"choices":[{"index":0,"delta":{"content":"tronq"},"finish_reason":null}]}"#.into(),
        r#"{"choices":[{"index":0,"delta":{},"finish_reason":"length"}]}"#.into(),
    ]
}
