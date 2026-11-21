//! The witnesses of the teammate's web tools: `fetch` (a page as readable text, bounded, its status and redirects, a
//! local address refused by default) and `web_search` (Gemini's Google Search grounding on Vertex), each against a
//! local server. A module of its own (page 03 of ED, 21 November 2026): the teammate's other witnesses are written by
//! other lots at the end of `teammate_tests.rs`.

use super::*;

/// A local HTTP server for the witnesses of `fetch`: `/page` (HTML, a script and visible text), `/missing` (404),
/// `/moved` (302 to `/page`). Returns its base address.
fn web_server() -> String {
    use std::io::{Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    std::thread::spawn(move || {
        for c in l.incoming().flatten() {
            let mut c = c;
            let mut buf = [0u8; 2048];
            let n = c.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
            let (status, extra, body) = match path.as_str() {
                "/page" => ("200 OK", "Content-Type: text/html; charset=utf-8\r\n".to_string(),
                    format!("<html><head><title>T</title><script>var secretScript = 1;</script><style>p {{}}</style></head><body><h1>Visible heading</h1><p>Visible text &amp; more.</p><p>{}</p></body></html>", "padding ".repeat(80))),
                "/moved" => ("302 Found", "Location: /page\r\n".to_string(), String::new()),
                _ => ("404 Not Found", "Content-Type: text/plain\r\n".to_string(), "nothing here".to_string()),
            };
            let _ = write!(c, "HTTP/1.1 {status}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        }
    });
    base
}


/// **fetch** (the teammate's base, 20 November 2026): a page comes back as readable text (no script, no style),
/// cut at its bound with the cut said; a 404 is said, not hidden; a redirect is followed and reported; a local
/// address is refused by default, naming `teammate.fetch.allow_local`.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_teammate_fetches_a_page_says_its_status_and_refuses_a_local_address_by_default() {
    let base = web_server();
    for allow_local in [false, true] {
        let d = tempfile::tempdir().unwrap();
        let (_depot, manifeste) = depot_inscrit(d.path(), true);
        let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
        // reply_wait_s: once the pages are read, the teammate waits for the supervisor; the witness does not.
        m["teammate"] = json!({ "fetch": { "allow_local": allow_local, "max_bytes": 300 }, "reply_wait_s": 2 });
        std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
        let declarations = manifeste.parent().unwrap();
        for args in [&["add", "-A"][..], &["commit", "-q", "-m", "fetch"]] {
            assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
        }
        let loaded = crate::backend::PreparedBackend::load(&manifeste).map(|p| p.manifest.teammate.clone().unwrap_or_default().fetch.allow_local);
        assert_eq!(loaded, Ok(allow_local), "the manifest loads with its teammate.fetch");
        let results = Arc::new(Mutex::new(Vec::<Value>::new()));
        let model: ChatModel = {
            let (base, results) = (base.clone(), results.clone());
            Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
                let (base, results) = (base.clone(), results.clone());
                Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                    let got = tool_results(turns);
                    if got.is_empty() {
                        return answer(turns, opts, sink, "", calls(&[
                            ("fetch", json!({ "url": format!("{base}/page") })),
                            ("fetch", json!({ "url": format!("{base}/missing") })),
                            ("fetch", json!({ "url": format!("{base}/moved") })),
                        ]));
                    }
                    if results.lock().unwrap().is_empty() {
                        *results.lock().unwrap() = got.iter().map(|r| serde_json::from_str(r).unwrap_or(json!(r))).collect();
                    }
                    answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "lu", "remaining": "" }))]))
                })))
            })
        };
        let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
        host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
        let files = host.teammate_files(&h.id, "doc").unwrap();
        files.append("supervisor", "request", None, "Lis ces pages.").unwrap();
        let outcome = host.run_teammate(&h.id, "doc");
        let r = results.lock().unwrap().clone();
        assert_eq!(r.len(), 3, "allow_local={allow_local}: {r:?}\noutcome: {outcome:?}\nmailbox: {:?}", files.mailbox());
        if !allow_local {
            for x in &r {
                assert_eq!(x["ok"], json!(false), "{x}");
                let e = x["error"].as_str().unwrap_or_default();
                assert!(e.contains("local") && e.contains("teammate.fetch.allow_local"), "a local address is refused by default: {x}");
            }
            continue;
        }
        let page = &r[0];
        assert_eq!((page["ok"].clone(), page["status"].clone()), (json!(true), json!(200)), "{page}");
        let text = page["text"].as_str().unwrap_or_default();
        assert!(text.contains("Visible heading") && text.contains("Visible text & more."), "readable text: {page}");
        assert!(!text.contains("secretScript") && !text.contains("<p>"), "no script, no markup: {page}");
        assert!(page["cut_bytes"].as_u64().is_some_and(|n| n > 0), "cut at max_bytes, and said: {page}");
        let missing = &r[1];
        assert_eq!((missing["ok"].clone(), missing["status"].clone()), (json!(false), json!(404)), "a 404 is said: {missing}");
        let moved = &r[2];
        assert_eq!(moved["status"], json!(200), "{moved}");
        assert!(moved["url"].as_str().is_some_and(|u| u.ends_with("/page")), "{moved}");
        assert_eq!(moved["redirects"].as_array().map(Vec::len), Some(1), "the redirect is reported: {moved}");
    }
}


/// A local server that plays Vertex's generateContent with Google Search grounding: three sources, two supports.
/// It keeps the request bodies it receives.
fn grounding_server(bodies: Arc<Mutex<Vec<Value>>>) -> String {
    use std::io::{BufRead, Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    std::thread::spawn(move || {
        for c in l.incoming().flatten() {
            let mut r = std::io::BufReader::new(c.try_clone().unwrap());
            let mut len = 0usize;
            loop {
                let mut line = String::new();
                if r.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0u8; len];
            let _ = r.read_exact(&mut body);
            bodies.lock().unwrap().push(serde_json::from_slice(&body).unwrap_or(Value::Null));
            let answer = json!({ "candidates": [{ "content": { "role": "model", "parts": [{ "text": "Rust 1.98 stabilised the new trait solver. It shipped on 13 November." }] },
                "finishReason": "STOP",
                "groundingMetadata": {
                    "webSearchQueries": ["rust 1.98 release"],
                    "groundingChunks": [
                        { "web": { "uri": "https://blog.rust-lang.org/2026/11/13/Rust-1.98.0.html", "title": "blog.rust-lang.org" } },
                        { "web": { "uri": "https://releases.rs/docs/1.98.0/", "title": "releases.rs" } },
                        { "web": { "uri": "https://news.example.org/rust", "title": "news.example.org" } }
                    ],
                    "groundingSupports": [
                        { "segment": { "startIndex": 0, "endIndex": 42, "text": "Rust 1.98 stabilised the new trait solver." }, "groundingChunkIndices": [0, 1], "confidenceScores": [0.9, 0.7] },
                        { "segment": { "startIndex": 43, "endIndex": 69, "text": "It shipped on 13 November." }, "groundingChunkIndices": [0], "confidenceScores": [0.8] }
                    ]
                } }] });
            let text = answer.to_string();
            let mut c = c;
            let _ = write!(c, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len());
        }
    });
    base
}


/// **web_search** (the teammate's base, 20 November 2026): a query answered by Gemini's Google Search grounding on
/// Vertex — the answer, the queries run, and the sources with title, address, the passage they support and their
/// provenance, bounded by teammate.web_search.max_sources and the cut said; a model that is not Gemini is refused,
/// with the reason.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_teammate_searches_the_web_through_gemini_grounding_and_a_model_that_is_not_gemini_is_refused() {
    let bodies = Arc::new(Mutex::new(Vec::<Value>::new()));
    let base = grounding_server(bodies.clone());
    for model in ["google/gemini-3.8-flash", "anthropic/claude-sonnet-5-5"] {
        let d = tempfile::tempdir().unwrap();
        let (_depot, manifeste) = depot_inscrit(d.path(), true);
        let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
        m["teammate"] = json!({ "web_search": { "max_sources": 2, "model": model, "endpoint": base }, "reply_wait_s": 2 });
        std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
        let declarations = manifeste.parent().unwrap();
        for args in [&["add", "-A"][..], &["commit", "-q", "-m", "web_search"]] {
            assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
        }
        let results = Arc::new(Mutex::new(Vec::<Value>::new()));
        let chat: ChatModel = {
            let results = results.clone();
            Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
                let results = results.clone();
                Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                    let got = tool_results(turns);
                    if got.is_empty() {
                        return answer(turns, opts, sink, "", calls(&[("web_search", json!({ "query": "what did Rust 1.98 stabilise?" }))]));
                    }
                    if results.lock().unwrap().is_empty() {
                        *results.lock().unwrap() = got.iter().map(|r| serde_json::from_str(r).unwrap_or(json!(r))).collect();
                    }
                    answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "cherché", "remaining": "" }))]))
                })))
            })
        };
        let host = hote(d.path(), chat).with_runs_root(d.path().join("runs"));
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
        host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
        let files = host.teammate_files(&h.id, "doc").unwrap();
        files.append("supervisor", "request", None, "Cherche ce que Rust 1.98 a stabilisé.").unwrap();
        let sent_before = bodies.lock().unwrap().len();
        let outcome = host.run_teammate(&h.id, "doc");
        let r = results.lock().unwrap().clone();
        assert_eq!(r.len(), 1, "{model}: {r:?}\noutcome: {outcome:?}");
        let x = &r[0];
        if model.starts_with("anthropic/") {
            assert_eq!(x["ok"], json!(false), "{x}");
            let e = x["error"].as_str().unwrap_or_default();
            assert!(e.contains("Gemini") && e.contains(model) && e.contains("teammate.web_search.model"), "refused with the reason: {x}");
            assert_eq!(bodies.lock().unwrap().len(), sent_before, "a model that is not Gemini sends nothing");
            continue;
        }
        assert_eq!(x["ok"], json!(true), "{x}");
        assert!(x["answer"].as_str().is_some_and(|a| a.contains("trait solver")), "{x}");
        assert_eq!(x["queries"], json!(["rust 1.98 release"]), "{x}");
        let sources = x["sources"].as_array().cloned().unwrap_or_default();
        assert_eq!(sources.len(), 2, "bounded by max_sources: {x}");
        assert_eq!((x["total_sources"].clone(), x["cut_sources"].clone()), (json!(3), json!(1)), "the cut is said: {x}");
        assert_eq!(sources[0]["title"], "blog.rust-lang.org");
        assert_eq!(sources[0]["url"], "https://blog.rust-lang.org/2026/11/13/Rust-1.98.0.html");
        let snippet = sources[0]["snippet"].as_str().unwrap_or_default();
        assert!(snippet.contains("trait solver") && snippet.contains("13 November"), "the passages it supports: {x}");
        assert_eq!(sources[0]["provenance"]["supports"], json!(2), "{x}");
        assert_eq!(sources[1]["provenance"]["supports"], json!(1), "{x}");
        let sent = bodies.lock().unwrap().last().cloned().unwrap_or_default();
        assert_eq!(sent["tools"], json!([{ "googleSearch": {} }]), "grounding asked: {sent}");
        assert!(sent["contents"].to_string().contains("what did Rust 1.98 stabilise?"), "{sent}");
    }
}
