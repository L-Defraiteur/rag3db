//! **An agent alone works in the user's folder itself** (Lucie, 23 Nov 2026: « je ne comprends pas cette copie
//! cachée… pourquoi ? », then « oui absolument »): its tools, its commands and its paths are hers; the end of a turn
//! commits, on her current branch, only what the turn changed, what she had changed before staying uncommitted; and
//! closing the task leaves her folder, its `.git` and her branch intact. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

fn git_out(dir: &std::path::Path, args: &[&str]) -> String {
    let o = std::process::Command::new("git").arg("-C").arg(dir).args(args).output().unwrap();
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn an_agent_alone_works_in_her_folder_and_closing_leaves_it_intact() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    let branche = git_out(&depot, &["rev-parse", "--abbrev-ref", "HEAD"]);
    // What she had changed before the turn.
    std::fs::write(depot.join("README.md"), "# Petit dépôt\n\nsa note à elle\n").unwrap();
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| match tool_results(turns).len() {
            0 => answer(turns, opts, sink, "", calls(&[("run_command", json!({ "command": "sh -c 'pwd > where.txt'" }))])),
            1 => answer(turns, opts, sink, "", calls(&[("edit_file", json!({ "path": "a.txt", "content": "a\n" }))])),
            _ => answer(turns, opts, sink, "C'est fait.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))])),
        })))
    });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Écris a.txt.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) {
        assert!(Instant::now() < fin, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    let b = files.mailbox();
    let tete = git_out(&depot, &["rev-parse", "HEAD"]);

    // First: closing the task leaves her folder, its .git and her branch intact.
    host.fermer_une_tache(&h.id, "doc").expect("the task closes");
    assert!(depot.join(".git").is_dir(), "her .git is intact");
    assert_eq!(git_out(&depot, &["rev-parse", "--abbrev-ref", "HEAD"]), branche, "her branch is intact");
    assert_eq!(git_out(&depot, &["rev-parse", "HEAD"]), tete, "her head is intact");
    assert!(depot.join("src/lib.rs").is_file(), "her files are intact");

    // The agent worked in her folder itself.
    let la = std::fs::read_to_string(depot.join("where.txt")).unwrap_or_default();
    assert_eq!(std::path::Path::new(la.trim()).canonicalize().ok(), depot.canonicalize().ok(), "a command runs in her folder: {la:?} {b:#?}");
    assert!(depot.join("a.txt").is_file(), "what the agent wrote is there");
    // The turn's commit, on her branch, takes only what the turn changed.
    let dans = git_out(&depot, &["show", "--name-only", "--format=", "HEAD"]);
    assert!(dans.contains("a.txt") && dans.contains("where.txt"), "the turn's commit: {dans}");
    assert!(!dans.contains("README.md"), "what she changed before stays out of it: {dans}");
    assert!(git_out(&depot, &["status", "--porcelain"]).contains("README.md"), "and stays uncommitted");
}
