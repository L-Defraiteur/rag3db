//! **The thin client says the Claude Code session that runs it** (recherche, 21 Nov 2026: the list of
//! the agents of the machine, on the IDE's welcome page, depends on it). A guard written green, said
//! as such: on 21 Nov the name was missing only from MCP servers started before the lot.

use super::*;

/// Under a process named `claude` whose registry entry is in the home, a descendant reads that
/// session's id and name; out of such a process, nothing.
#[test]
fn a_client_under_a_claude_process_reads_its_session_and_name() {
    let d = tempfile::tempdir().unwrap();
    let claude = d.path().join("claude");
    std::fs::copy("/bin/sh", &claude).unwrap();
    // The fake Claude Code: a shell named claude whose child sleeps (`; :` keeps the shell alive).
    let mut parent = std::process::Command::new(&claude).args(["-c", "sleep 30; :"]).spawn().unwrap();
    let enfants = |pid: u32| std::fs::read_to_string(format!("/proc/{pid}/task/{pid}/children")).unwrap_or_default();
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let enfant: u32 = loop {
        if let Some(p) = enfants(parent.id()).split_whitespace().next().and_then(|p| p.parse().ok()) {
            break p;
        }
        assert!(std::time::Instant::now() < fin, "the fake claude has no child");
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let maison = d.path().join("maison");
    std::fs::create_dir_all(maison.join(".claude/sessions")).unwrap();
    std::fs::write(maison.join(format!(".claude/sessions/{}.json", parent.id())), json!({ "sessionId": "S-garde", "name": "le témoin" }).to_string()).unwrap();
    let lu = claude_de(enfant, &maison);
    let _ = parent.kill();
    let _ = parent.wait();
    assert_eq!(lu, Some(json!({ "session_id": "S-garde", "name": "le témoin", "pid": parent.id() })));
    // Out of a claude process (this test's own parent chain may hold one: the home has no entry for it).
    let vide = d.path().join("vide");
    std::fs::create_dir_all(vide.join(".claude/sessions")).unwrap();
    assert_eq!(claude_de(1, &vide), None, "pid 1 has no claude ancestor");
}
