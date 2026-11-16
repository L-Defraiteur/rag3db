//! Test-only supervisor and failure evidence for the documentation scenario.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const DENIAL_TEXT: &str = "Do not remove Cargo.lock. This command is denied. Continue the original documentation request without deleting files.";

/// Baseline for the red witness: the old real-model fixture had no supervisor.
/// The narrowly scoped denial is added only after that witness is seen failing.
pub fn documentation_answer(_mailbox: &[Value]) -> Result<Option<Value>, String> {
    Ok(None)
}

pub struct DocumentationFixture {
    directory: Option<tempfile::TempDir>,
    path: PathBuf,
}
impl DocumentationFixture {
    pub fn new() -> Self {
        let directory = tempfile::Builder::new().prefix("documentation-fixture-").tempdir().unwrap();
        let path = directory.path().to_path_buf();
        Self { directory: Some(directory), path }
    }
    pub fn path(&self) -> &Path { &self.path }
    pub fn preserve(&mut self, reason: &str, mailbox: &[Value]) {
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
            self.preserve("unexpected assertion; full runs/chats/task files are preserved", &[]);
        }
    }
}
