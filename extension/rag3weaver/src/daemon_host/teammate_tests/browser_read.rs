//! **The teammate reads the browser through the bridge** (lot 1 of the browser tool, tty, 21 November 2026). The
//! witnesses run on a fake extension over a Unix socket, without Chrome: it says hello, answers each command, and
//! counts what it receives, so that a refusal is proven by nothing sent.
use crate::backend::BrowserPolicy;
use crate::daemon_host::browser_tools::{family, BrowserTools, BROWSER_CONSOLE, BROWSER_READ, BROWSER_TABS};
use crate::pont_navigateur::cadrage::{encoder, Decodeur};
use crate::pont_navigateur::ecoute::Ecoute;
use crate::pont_navigateur::pont::Politique;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const DOCS: &str = "https://docs.example.org/guide";
const BANK: &str = "https://bank.example.com/";
const ARIA: &str = "- heading \"Guide\" [ref=e1]\n- textbox \"Recherche\" [ref=e2]: rust\n- textbox \"Mot de passe\" [ref=e3]: hunter2\n- button \"Entrer\" [ref=e4]";

/// The fake extension: tab 7 on the docs, tab 9 on a bank; it answers each command and keeps it.
pub(super) fn fake_extension(socket: &std::path::Path) -> Arc<Mutex<Vec<Value>>> {
    fake_extension_with(socket, ARIA)
}

/// The fake extension, its page's accessibility tree given.
pub(super) fn fake_extension_with(socket: &std::path::Path, aria: &'static str) -> Arc<Mutex<Vec<Value>>> {
    let received = Arc::new(Mutex::new(Vec::<Value>::new()));
    let mut s = UnixStream::connect(socket).expect("the bridge listens");
    let hello = json!({ "type": "hello", "browserId": "chrome-essai", "extensionVersion": "0.0.0", "protocolRevision": 5, "features": [] });
    s.write_all(&encoder(&hello).unwrap()).unwrap();
    let kept = received.clone();
    std::thread::spawn(move || {
        let mut d = Decodeur::default();
        let mut buf = [0u8; 65536];
        loop {
            let n = match s.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => n,
            };
            for m in d.pousser(&buf[..n]).into_iter().flatten() {
                if m["type"] != "command" {
                    continue;
                }
                let c = m["command"].clone();
                kept.lock().unwrap().push(c.clone());
                let page = c["page"].as_str().unwrap_or_default();
                let url = if page == "9" { BANK } else { DOCS };
                let data = match c["action"].as_str().unwrap_or_default() {
                    "user-tabs" => json!({ "tabs": [ { "tabId": 7, "url": DOCS, "title": "Guide" }, { "tabId": 9, "url": BANK, "title": "Banque" } ] }),
                    "claim" => json!({ "page": c["claim"]["tabId"].to_string(), "url": if c["claim"]["tabId"] == 9 { BANK } else { DOCS }, "title": "Guide" }),
                    "exec" if c["code"].as_str().unwrap_or_default().contains(".aria(") => json!(aria),
                    "exec" => json!({ "url": url, "title": "Guide" }),
                    "console" => json!({ "items": [ { "sequence": 1, "at": 1795240000000u64, "level": "error", "text": "TypeError: x is undefined" } ], "next": 1 }),
                    _ => json!(null),
                };
                let r = json!({ "type": "result", "result": { "id": c["id"], "ok": true, "data": data } });
                if s.write_all(&encoder(&r).unwrap()).is_err() {
                    return;
                }
            }
        }
    });
    received
}

/// A listening bridge on a socket of the test's own, the fake extension connected, and the task's browser.
fn bench(domains: &[&str]) -> (tempfile::TempDir, BrowserTools, Arc<Mutex<Vec<Value>>>) {
    let d = tempfile::tempdir().unwrap();
    let socket = d.path().join("pont.sock");
    let ecoute = Arc::new(Ecoute::ouvrir(&socket, "essai", Politique::default()).expect("the bridge listens"));
    let received = fake_extension(&socket);
    let end = Instant::now() + Duration::from_secs(10);
    while ecoute.navigateurs().is_empty() {
        assert!(Instant::now() < end, "the fake extension never said hello");
        std::thread::sleep(Duration::from_millis(20));
    }
    let policy = BrowserPolicy { domains: domains.iter().map(|s| s.to_string()).collect(), ..BrowserPolicy::default() };
    (d, BrowserTools { policy, ecoute, session: "teammate:essai".into() }, received)
}

fn actions(received: &Arc<Mutex<Vec<Value>>>) -> Vec<String> {
    received.lock().unwrap().iter().map(|c| c["action"].as_str().unwrap_or_default().to_string()).collect()
}

/// The family is absent without `teammate.browser`, and every parameter of it is described.
#[test]
fn the_browser_family_comes_only_with_its_declaration() {
    assert!(family(None).is_empty(), "no teammate.browser, no browser tool");
    let defs = family(Some(&BrowserPolicy::default()));
    let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, vec![BROWSER_TABS, BROWSER_READ, BROWSER_CONSOLE]);
    for d in &defs {
        for (p, v) in d.parameters["properties"].as_object().unwrap() {
            assert!(v["description"].as_str().is_some_and(|t| !t.trim().is_empty()), "{}.{p} has its sentence", d.name);
        }
    }
}

/// The tabs are listed with whether they may be taken; a tab outside the declared domains is refused before any
/// claim is sent.
#[test]
fn a_tab_outside_the_declared_domains_is_never_taken() {
    let (_d, b, received) = bench(&["example.org"]);
    let listed = b.call(BROWSER_TABS, &json!({})).expect("of the family").expect("listed");
    let tabs = listed["tabs"].as_array().expect("tabs");
    assert_eq!(tabs.len(), 2, "{listed}");
    assert!(tabs.iter().any(|t| t["tab_id"] == "7" && t["allowed"] == true), "{listed}");
    assert!(tabs.iter().any(|t| t["tab_id"] == "9" && t["allowed"] == false), "{listed}");
    let refused = b.call(BROWSER_TABS, &json!({ "claim": "9" })).unwrap().expect_err("outside the domains");
    assert!(refused.contains("bank.example.com") && refused.contains("teammate.browser.domains"), "{refused}");
    assert!(!actions(&received).contains(&"claim".to_string()), "nothing claimed: {:?}", actions(&received));
    let taken = b.call(BROWSER_TABS, &json!({ "claim": "7" })).unwrap().expect("taken");
    assert_eq!(taken["tab_id"], "7", "{taken}");
}

/// The accessibility tree comes with its refs, and a secret field's value is masked.
#[test]
fn a_page_is_read_with_its_refs_and_its_secrets_masked() {
    let (_d, b, received) = bench(&["example.org"]);
    let r = b.call(BROWSER_READ, &json!({ "tab_id": "7", "mode": "aria" })).unwrap().expect("read");
    let content = r["content"].as_str().unwrap_or_default();
    assert!(content.contains("[ref=e2]: rust") && content.contains("[ref=e4]"), "the tree and its refs: {r}");
    assert!(!content.contains("hunter2"), "a password is never read: {r}");
    assert_eq!(r["masked_fields"], 1, "{r}");
    assert_eq!((r["url"].as_str(), r["truncated"].as_bool()), (Some(DOCS), Some(false)), "{r}");
    // The page is read by the host's own code, never by code of the model.
    let codes: Vec<String> = received.lock().unwrap().iter().filter(|c| c["action"] == "exec").map(|c| c["code"].as_str().unwrap_or_default().to_string()).collect();
    assert!(codes.iter().all(|c| c.contains("__browsermcpPage") || c.contains("location.href")), "{codes:?}");
}

/// A page outside the declared domains is not read: only its address is asked, and the refusal says it.
#[test]
fn a_page_outside_the_declared_domains_is_not_read() {
    let (_d, b, received) = bench(&["example.org"]);
    let refused = b.call(BROWSER_READ, &json!({ "tab_id": "9", "mode": "text" })).unwrap().expect_err("outside the domains");
    assert!(refused.contains("bank.example.com"), "{refused}");
    let codes: Vec<String> = received.lock().unwrap().iter().filter(|c| c["action"] == "exec").map(|c| c["code"].as_str().unwrap_or_default().to_string()).collect();
    assert!(codes.iter().all(|c| !c.contains("__browsermcpPage") && !c.contains("innerText")), "nothing of the page read: {codes:?}");
    assert!(b.call(BROWSER_CONSOLE, &json!({ "tab_id": "9" })).unwrap().is_err(), "nor its console");
    assert!(!actions(&received).contains(&"console".to_string()), "{:?}", actions(&received));
}

/// The console is read with its time, level and text.
#[test]
fn a_page_console_is_read() {
    let (_d, b, _received) = bench(&["example.org"]);
    let r = b.call(BROWSER_CONSOLE, &json!({ "tab_id": "7" })).unwrap().expect("read");
    let items = r["messages"].as_array().expect("messages");
    assert_eq!(items[0]["level"], "error", "{r}");
    assert!(items[0]["text"].as_str().is_some_and(|t| t.contains("TypeError")), "{r}");
    assert!(items[0]["at"].is_u64(), "{r}");
}

/// **Through the host** : a manifest that declares `teammate.browser` opens the host's one listening socket when its
/// backend opens, `/sante` says it and the linked browsers, and the teammate's model sees the family and reads a page
/// with it.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_manifest_with_teammate_browser_opens_the_listening_and_gives_the_family() {
    use super::super::chat::ChatModel;
    use super::super::chat_qui_code::{depot_inscrit, hote};
    use crate::llm::{CallbackLlm, Llm};
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["teammate"] = json!({ "browser": { "domains": ["example.org"] }, "reply_wait_s": 2 });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "browser"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let seen = Arc::new(Mutex::new((Vec::<String>::new(), Vec::<Value>::new())));
    let model: ChatModel = {
        let seen = seen.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let seen = seen.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let got = super::tool_results(turns);
                if got.is_empty() {
                    seen.lock().unwrap().0 = opts.tools.iter().map(|t| t.name.clone()).collect();
                    return super::answer(turns, opts, sink, "", super::calls(&[(BROWSER_READ, json!({ "tab_id": "7", "mode": "aria" }))]));
                }
                if seen.lock().unwrap().1.is_empty() {
                    seen.lock().unwrap().1 = got.iter().map(|r| serde_json::from_str(r).unwrap_or(json!(r))).collect();
                }
                super::answer(turns, opts, sink, "", super::calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "lu", "remaining": "" }))]))
            })))
        })
    };
    let socket = d.path().join("pont").join("pont.sock");
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs")).with_browser_socket(socket.clone());
    assert_eq!(host.handle("GET", "/sante", None, b"").1["browser"], Value::Null, "nothing listens before a manifest asks");
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let sante = host.handle("GET", "/sante", None, b"").1;
    assert_eq!(sante["browser"]["socket"], json!(socket.display().to_string()), "{sante}");
    let _received = fake_extension(&socket);
    let end = Instant::now() + Duration::from_secs(10);
    while host.handle("GET", "/sante", None, b"").1["browser"]["browsers"].as_array().is_none_or(|b| b.is_empty()) {
        assert!(Instant::now() < end, "/sante never said the linked browser");
        std::thread::sleep(Duration::from_millis(20));
    }
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    files.append("supervisor", "request", None, "Lis la page de l'onglet 7.").unwrap();
    let outcome = host.run_teammate(&h.id, "doc");
    let (tools, results) = seen.lock().unwrap().clone();
    for t in [BROWSER_TABS, BROWSER_READ, BROWSER_CONSOLE] {
        assert!(tools.iter().any(|n| n == t), "the model sees {t}: {tools:?}");
    }
    let r = results.first().cloned().unwrap_or_default();
    assert!(r["content"].as_str().is_some_and(|c| c.contains("[ref=e2]") && !c.contains("hunter2")), "{r}\noutcome: {outcome:?}");
}
