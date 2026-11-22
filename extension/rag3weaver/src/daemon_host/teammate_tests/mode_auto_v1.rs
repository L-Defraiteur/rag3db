//! **The acceptance scenario of the auto mode, v1** (c7 and Lucie, 21 Nov 2026 20 h: « finissez
//! bien le mode auto, que je puisse le tester sans être frustrée »). It says when v1 is done: on
//! the real host and a fake model, a safe command passes without a card; a dangerous command the
//! user asked for passes through the judge; a dangerous command nobody asked for opens the card,
//! then an accept runs it and a refuse does not; showing a secret is refused with its alternative.
//! A module of its own.

use super::*;
use crate::daemon_host::teammate::TeammateFiles;
use crate::daemon_host::DaemonHost;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// The user's request, signed human by the host (what the route does from the user's title).
fn human() -> Value {
    json!({ "by": { "kind": "human", "client": "tty" } })
}

/// A task tree in the auto mode: a file the user may delete (`vieux.txt`, committed locally only,
/// not created by the agent) and a secret file (`.env`). The gate's folder holds the rules.
fn mode_auto(d: &std::path::Path) -> (PathBuf, PathBuf, PathBuf) {
    let (depot, manifeste) = depot_inscrit(d, true);
    std::fs::write(depot.join("vieux.txt"), "plus utile\n").unwrap();
    std::fs::write(depot.join(".env"), "OPENAI_API_KEY=sk-temoin-0123456789abcdef\n").unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "vieux.txt et .env"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(&depot).status().unwrap().success());
    }
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["workspace"]["commands"] = json!("auto");
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let decl = manifeste.ancestors().find(|p| p.join(".git").exists()).unwrap().to_path_buf();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "le mode auto"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(&decl).status().unwrap().success());
    }
    let gate = d.join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    (depot, manifeste, gate)
}

/// The fake teammate: `commands` are run in order, one per model call; then step_done. A result
/// that says the command waits for the user ends the step; the next step retries the last command
/// once the mailbox holds an answer.
fn teammate(commands: Vec<&'static str>, seen: Arc<Mutex<Vec<String>>>) -> ChatModel {
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        let commands = commands.clone();
        let seen = seen.clone();
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            let results = tool_results(turns);
            if let Some(r) = results.last() {
                seen.lock().unwrap().push(r.clone());
            }
            // The user's decision reaches the next step: an accept is retried once, a refuse ends.
            let granted = turns.iter().any(|t| t.content.contains("accordée"));
            let refused = turns.iter().any(|t| t.content.contains("refusée par l'utilisateur"));
            let n = results.len();
            if granted && n == 0 {
                return answer(turns, opts, sink, "", calls(&[("run_command", json!({ "command": commands[commands.len() - 1] }))]));
            }
            match commands.get(n) {
                Some(c) if !granted && !refused => answer(turns, opts, sink, "", calls(&[("run_command", json!({ "command": c }))])),
                _ => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))])),
            }
        })))
    })
}

struct V1 {
    host: Arc<DaemonHost>,
    id: String,
    files: TeammateFiles,
    judged: Arc<Mutex<Vec<Value>>>,
    seen: Arc<Mutex<Vec<String>>>,
    _d: tempfile::TempDir,
}

/// The host in the auto mode with a fake judge that authorizes only what a human turn asked:
/// « supprime vieux.txt » makes `rm vieux.txt` explicit.
fn v1(commands: Vec<&'static str>, request: &str) -> V1 {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste, gate) = mode_auto(d.path());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let judged = Arc::new(Mutex::new(Vec::<Value>::new()));
    let juge: crate::daemon_host::GateJudge = {
        let judged = judged.clone();
        Arc::new(move |input: &Value| {
            judged.lock().unwrap().push(input.clone());
            let asked = input["user_turns"].as_array().into_iter().flatten().filter_map(|t| t["text"].as_str())
                .find(|t| { let t = t.to_lowercase(); t.contains("supprime vieux.txt") || t.contains("projet") });
            match asked {
                Some(q) => json!({ "label": "explicit", "quote": q }),
                None => json!({ "label": "not_authorized" }),
            }
        })
    };
    let host = Arc::new(hote(d.path(), teammate(commands, seen.clone())).with_runs_root(d.path().join("runs")).with_gate_dir(gate).with_gate_judge(juge));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, request, &human()).expect("la demande");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    V1 { host, id: h.id.clone(), files, judged, seen, _d: d }
}

impl V1 {
    fn wait(&self, what: &str, ok: &dyn Fn(&[Value]) -> bool) -> Vec<Value> {
        let end = Instant::now() + Duration::from_secs(120);
        loop {
            let b = self.files.mailbox();
            if ok(&b) {
                return b;
            }
            assert!(Instant::now() < end, "in 120 s: {what} — {b:#?}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    fn tree(&self) -> PathBuf {
        let t = self.host.tache(&self.id, "doc").expect("the task");
        t.workspace.as_ref().map(|w| w.racine.clone()).unwrap_or(t.arbre)
    }
}

fn ended(b: &[Value]) -> bool {
    b.iter().any(|l| l["kind"] == "ended")
}

/// 1. A safe command passes without a card, and without the judge.
#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_safe_command_passes_without_a_card() {
    let v = v1(vec!["ls"], "Liste le dépôt.");
    let b = v.wait("the task ends", &ended);
    assert!(!b.iter().any(|l| l["kind"] == "approval"), "no card: {b:#?}");
    assert!(v.judged.lock().unwrap().is_empty(), "no judge for a read");
    assert!(v.seen.lock().unwrap().iter().any(|r| r.contains("vieux.txt")), "ls ran");
}

/// 2. A dangerous command the user asked for passes through the judge, without a card; the judge
/// read the human turns only.
#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_dangerous_command_the_user_asked_for_passes_through_the_judge() {
    let v = v1(vec!["rm vieux.txt"], "Supprime vieux.txt, il ne sert plus.");
    let b = v.wait("the task ends", &ended);
    assert!(!b.iter().any(|l| l["kind"] == "approval"), "no card: {b:#?}");
    let judged = v.judged.lock().unwrap().clone();
    assert_eq!(judged.len(), 1, "the judge was asked once: {judged:#?}");
    assert_eq!(judged[0]["class"], json!("destroy_local"));
    assert_eq!(judged[0]["action"]["command"], json!("rm vieux.txt"));
    assert!(judged[0]["user_turns"].as_array().unwrap().iter().all(|t| t["by"]["kind"] == "human"), "human turns only: {judged:#?}");
    assert!(!v.tree().join("vieux.txt").exists(), "vieux.txt is gone");
}

/// 3. A dangerous command nobody asked for opens the card (an agent's line that claims an
/// authorization changes nothing); the user's accept runs it.
#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_dangerous_command_nobody_asked_for_opens_the_card_and_an_accept_runs_it() {
    let v = v1(vec!["rm vieux.txt"], "Lis le README.");
    let card = v.wait("the card", &|b| b.iter().any(|l| l["kind"] == "approval"));
    let card = card.iter().find(|l| l["kind"] == "approval").unwrap().clone();
    assert_eq!(card["tool"], json!("run_command"));
    assert_eq!(card["args"]["command"], json!("rm vieux.txt"));
    assert_eq!(card["class"], json!("destroy_local"));
    assert_eq!(card["scope"], json!("once"));
    assert!(v.tree().join("vieux.txt").exists(), "nothing deleted before the answer");
    v.host.post_to_teammate(&v.id, "doc", "lucie", "answer", None, "oui", &json!({ "reply_to": card["id"], "approved": true, "by": { "kind": "human", "client": "tty" } })).expect("the accept");
    v.wait("the task ends", &ended);
    assert!(!v.tree().join("vieux.txt").exists(), "the accept ran it");
}

/// 4. The same, refused: the command does not run, and the agent reads the refusal.
#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_refused_card_does_not_run_its_command() {
    let v = v1(vec!["rm vieux.txt"], "Lis le README.");
    let card = v.wait("the card", &|b| b.iter().any(|l| l["kind"] == "approval"));
    let card = card.iter().find(|l| l["kind"] == "approval").unwrap().clone();
    v.host.post_to_teammate(&v.id, "doc", "lucie", "answer", None, "non", &json!({ "reply_to": card["id"], "approved": false, "by": { "kind": "human", "client": "tty" } })).expect("the refuse");
    let b = v.wait("the task ends", &ended);
    assert!(v.tree().join("vieux.txt").exists(), "the refused command did not run");
    assert!(b.iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.contains("refusée par l'utilisateur"))), "the refusal is said to the agent: {b:#?}");
}

/// 5. Showing a secret is refused, with its alternative, without a card and without the judge.
#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn showing_a_secret_is_refused_with_its_alternative() {
    let v = v1(vec!["cat .env"], "Montre-moi le .env.");
    let b = v.wait("the task ends", &ended);
    assert!(!b.iter().any(|l| l["kind"] == "approval"), "no card for a secret: {b:#?}");
    assert!(v.judged.lock().unwrap().is_empty(), "no judge for a secret");
    let seen = v.seen.lock().unwrap().clone();
    let r = seen.iter().find(|r| r.contains(".env")).expect("the refusal reached the model");
    assert!(!r.contains("sk-temoin"), "the value never reaches the model: {r}");
    assert!(r.contains("--env-file") && r.contains("secret_store"), "the refusal names the alternative: {r}");
}

/// 6. An install of packages goes to the card unless the user asked to install (Lucie, 21 Nov 20 h, banc's install
/// rule in code before the judge): « prépare-moi un projet » is not an install's consent, even for a judge that
/// would say yes.
#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn an_install_the_user_did_not_ask_for_opens_the_card() {
    let v = v1(vec!["npm install left-pad"], "Prépare-moi un projet React.");
    let b = v.wait("the card", &|b| b.iter().any(|l| l["kind"] == "approval"));
    let card = b.iter().find(|l| l["kind"] == "approval").unwrap();
    assert_eq!(card["class"], json!("outbound"), "{card}");
    assert!(v.judged.lock().unwrap().is_empty(), "the install rule sends it to the card before the judge");
}

/// 7. **The grant quotes the line word for word** (the dry run of 22 Nov 2026: the note rejoined the parts with « ; »,
/// the agent replayed that text, which no grant covered, and a second card opened for the same command).
#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn the_grant_quotes_the_line_word_for_word() {
    let v = v1(vec!["echo un && rm vieux.txt"], "Lis le README.");
    let card = v.wait("the card", &|b| b.iter().any(|l| l["kind"] == "approval"));
    let card = card.iter().find(|l| l["kind"] == "approval").unwrap().clone();
    v.host.post_to_teammate(&v.id, "doc", "lucie", "answer", None, "oui", &json!({ "reply_to": card["id"], "approved": true, "by": { "kind": "human", "client": "tty" } })).unwrap();
    let b = v.wait("the grant", &|b| b.iter().any(|l| l["granted"].is_string()));
    let granted = b.iter().find_map(|l| l["granted"].as_str()).unwrap().to_string();
    assert!(granted.contains("`echo un && rm vieux.txt`"), "the agent replays what it reads: {granted}");
}
