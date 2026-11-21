//! **A secret a tool brings out is seen by no one** (lot 5b of the auto mode's v1, page 07 of rag3weaver-archi; the
//! single point agreed with archi, 21 November 2026): the toolbox of a chat turn or a teammate's step is wrapped
//! (`MaskedTools`) as close to the tools as can be, so the model, the step's work, the card, the journal, the stream
//! and the IDE all get the masked text. A file of its own (c7's rule).

use super::*;

const TOKEN: &str = "ghp_0123456789abcdefABCDEF0123456789abcd";

/// A model that reads `secret.txt`, then keeps what the tool returned.
fn reads_the_secret(seen: Arc<Mutex<Vec<String>>>) -> ChatModel {
    let calls = Arc::new(AtomicUsize::new(0));
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        let (calls, seen) = (calls.clone(), seen.clone());
        Ok(Arc::new(CallbackLlm::new("reads-a-secret", 8192, move |turns, opts, sink| {
            let model = match calls.fetch_add(1, Ordering::SeqCst) {
                0 => MockLlm::new("je lis").with_tool_calls(vec![("read_file", r#"{"path":"secret.txt"}"#)]),
                _ => {
                    seen.lock().unwrap().extend(turns.iter().filter(|t| t.role == "tool").map(|t| t.content.clone()));
                    MockLlm::new("Fini.")
                }
            };
            model.generate(turns, opts, sink)
        })))
    })
}

/// A file read by a tool carries a token: the model reads it masked, by its name, and the chat's state (what the IDE
/// shows) never holds its value.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_secret_brought_out_by_a_tool_is_seen_by_no_one() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (d, host, _manifeste, id) = banc(reads_the_secret(seen.clone()));
    std::fs::write(d.path().join("code/secret.txt"), format!("deploy with GITHUB_TOKEN={TOKEN}\n")).unwrap();
    let r = host.chat_turn(&id, "lis secret.txt").expect("the turn");
    let seen = seen.lock().unwrap().clone();
    assert!(!seen.is_empty(), "the model got the tool's result");
    assert!(seen.iter().all(|s| !s.contains(TOKEN)), "the model never sees the token: {seen:?}");
    assert!(seen.iter().any(|s| s.contains("[masked secret: github token]")), "it reads the masked name: {seen:?}");
    assert!(!r.to_string().contains(TOKEN), "the turn's answer: {r}");
    let state = host.chat_state(&id).expect("the chat's state");
    assert!(!state.to_string().contains(TOKEN), "the chat's state, what the IDE shows: {state}");
}

/// **The user's declared identity is not a secret** (Lucie, 21 November 2026, after a plugin blocked every e-mail
/// address, her own git identity included: « à ne pas reproduire »): an author's name and address, as `git log` shows
/// them, pass unmasked. A guard against a pattern that would reach them; nothing was masking them.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_users_git_identity_is_never_masked() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (d, host, _manifeste, id) = banc(reads_the_secret(seen.clone()));
    let author = "Author: Lucie Example <lucie@example.org>";
    std::fs::write(d.path().join("code/secret.txt"), format!("commit 1a2b3c\n{author}\n\n    the message\n")).unwrap();
    host.chat_turn(&id, "lis secret.txt").expect("the turn");
    let seen = seen.lock().unwrap().clone();
    assert!(seen.iter().any(|s| s.contains(author)), "the author's identity passes as it is: {seen:?}");
    assert!(seen.iter().all(|s| !s.contains("[masked secret")), "nothing is masked: {seen:?}");
}
