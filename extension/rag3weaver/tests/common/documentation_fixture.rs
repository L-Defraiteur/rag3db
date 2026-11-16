//! Test-only supervisor and failure evidence for the documentation scenario.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const DENIAL_TEXT: &str = "Do not remove Cargo.lock. This command is denied. Continue the original documentation request without deleting files.";

/// Only the exact fixture command is denied. Every unknown pending approval or
/// question stops the fixture instead of silently granting or replacing work.
pub fn documentation_answer(mailbox: &[Value]) -> Result<Option<Value>, String> {
    if mailbox.iter().any(|line| line["approved"] == true || !line["granted"].is_null()) {
        return Err("documentation fixture observed an unexpected command grant".into());
    }
    let mut pending = Vec::new();
    for line in mailbox.iter().filter(|line| {
        line["from"] == "teammate"
            && matches!(line["kind"].as_str(), Some("approval" | "question"))
    }) {
        let id = line["id"].as_str().filter(|id| !id.is_empty())
            .ok_or_else(|| "documentation fixture received a pending message without an id".to_string())?;
        let answered = mailbox.iter().any(|answer| {
            answer["kind"] == "answer" && answer["from"] == "c7"
                && answer["reply_to"].as_str() == Some(id)
        });
        if answered { continue; }
        if line["kind"] != "approval" || line["command"].as_str() != Some("rm Cargo.lock") {
            return Err(format!("documentation fixture refuses an unknown pending approval or question: {line}"));
        }
        pending.push(id);
    }
    match pending.as_slice() {
        [] => Ok(None),
        [id] => Ok(Some(json!({"reply_to": id, "approved": false}))),
        _ => Err("documentation fixture received multiple pending command approvals".into()),
    }
}

pub struct DocumentationFixture {
    directory: Option<tempfile::TempDir>,
    path: PathBuf,
    mailbox: Vec<Value>,
}
impl DocumentationFixture {
    pub fn new() -> Self {
        let directory = tempfile::Builder::new().prefix("documentation-fixture-").tempdir().unwrap();
        let path = directory.path().to_path_buf();
        Self { directory: Some(directory), path, mailbox: Vec::new() }
    }
    pub fn path(&self) -> &Path { &self.path }
    pub fn observe(&mut self, mailbox: &[Value]) { self.mailbox = mailbox.to_vec(); }
    pub fn preserve(&mut self, reason: &str, mailbox: &[Value]) {
        self.observe(mailbox);
        if let Some(directory) = self.directory.take() { self.path = directory.keep(); }
        let evidence = json!({"reason": reason, "mailbox": mailbox, "fixture": self.path});
        if let Err(error) = std::fs::write(self.path.join("fixture-failure.json"), serde_json::to_vec_pretty(&evidence).unwrap()) {
            eprintln!("DOCUMENTATION_FIXTURE_EVIDENCE_ERROR {error}");
        }
        eprintln!("DOCUMENTATION_FAILED_FIXTURE {}", self.path.display());
    }
    pub fn fail(&mut self, reason: &str, mailbox: &[Value]) -> ! {
        self.preserve(reason, mailbox);
        panic!("{reason}");
    }
}
impl Drop for DocumentationFixture {
    fn drop(&mut self) {
        if std::thread::panicking() && self.directory.is_some() {
            let mailbox = self.mailbox.clone();
            self.preserve("unexpected assertion; full runs/chats/task files are preserved", &mailbox);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn approval(id: &str, command: &str) -> Value {
        json!({"id":id,"kind":"approval","from":"teammate","command":command})
    }
    #[test]
    fn only_the_exact_lock_deletion_gets_a_linked_refusal() {
        let response = documentation_answer(&[approval("a", "rm Cargo.lock")]).unwrap().unwrap();
        assert_eq!(response, json!({"reply_to":"a","approved":false}));
    }
    #[test]
    fn other_commands_and_shell_variants_stop_the_fixture() {
        for command in ["rm -f Cargo.lock", "rm ./Cargo.lock", "rm Cargo.lock; echo done", "cargo test"] {
            assert!(documentation_answer(&[approval("a", command)]).is_err(), "{command}");
        }
    }
    #[test]
    fn unknown_questions_and_missing_ids_stop_the_fixture() {
        assert!(documentation_answer(&[json!({"id":"q","from":"teammate","kind":"question"})]).is_err());
        assert!(documentation_answer(&[json!({"from":"teammate","kind":"approval","command":"rm Cargo.lock"})]).is_err());
    }
    #[test]
    fn an_answered_approval_is_not_answered_again() {
        assert_eq!(documentation_answer(&[approval("a", "rm Cargo.lock"), json!({"from":"c7","kind":"answer","reply_to":"a","approved":false})]).unwrap(), None);
    }
    #[test]
    fn ambiguous_or_mixed_pending_approvals_stop_without_an_answer() {
        assert!(documentation_answer(&[approval("a", "rm Cargo.lock"), approval("b", "rm Cargo.lock")]).is_err());
        assert!(documentation_answer(&[approval("a", "rm Cargo.lock"), approval("b", "rm other")]).is_err());
    }
}
