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

/// Test-only bridge to the existing manifest SandboxConfig, before add_files
/// copies/commits the template. No declaration means the normal fixture stays unchanged.
#[cfg(test)]
pub fn connect_declared_sandbox<S: serde::de::DeserializeOwned>(manifest: &mut Value) -> Result<bool, String> {
    let Some(file) = std::env::var_os("RAG3WEAVER_DOCUMENTATION_SANDBOX_FILE") else { return Ok(false) };
    let file = PathBuf::from(file);
    if !file.is_absolute() || !file.is_file() || file.canonicalize().map_err(|e| e.to_string())? != file {
        return Err("private sandbox declaration must be an absolute regular unlinked file".into());
    }
    let targets = PathBuf::from(std::env::var_os("RAG3WEAVER_TARGETS_DIR")
        .ok_or("private fixture sandbox requires explicit RAG3WEAVER_TARGETS_DIR")?);
    if !targets.is_absolute() || !targets.is_dir() || targets.canonicalize().map_err(|e| e.to_string())? != targets {
        return Err("private session target root must be an existing canonical directory".into());
    }
    let sandbox: Value = serde_json::from_slice(&std::fs::read(&file).map_err(|e| e.to_string())?)
        .map_err(|e| format!("private sandbox JSON: {e}"))?;
    // Both callers instantiate this with the real backend_code::SandboxConfig.
    // Keep the original JSON so omitted defaults do not change the committed declaration.
    let _: S = serde_json::from_value(sandbox.clone()).map_err(|e| format!("private SandboxConfig: {e}"))?;
    if !matches!(sandbox.get("mode"), None | Some(Value::Null)) && sandbox["mode"] != "landlock" {
        return Err("private fixture must retain Landlock".into());
    }
    if sandbox.get("network").is_some_and(|v| v != &Value::Bool(false))
        || sandbox.get("extra_write").is_some_and(|v| v.as_array().is_none_or(|v| !v.is_empty())) {
        return Err("private fixture does not grant network or extra writes".into());
    }
    if sandbox["env"]["CARGO_TARGET_DIR"] != "{repo_target}"
        || sandbox["env"]["TMPDIR"] != "{task_dir}/tmp"
        || sandbox["env"]["CARGO_NET_OFFLINE"] != "true" {
        return Err("private fixture must use the declared repo target, task tmp and offline Cargo".into());
    }
    let jobs = sandbox["env"]["CARGO_BUILD_JOBS"].as_str().and_then(|v| v.parse::<usize>().ok())
        .ok_or("private fixture Cargo jobs must be a positive integer")?;
    let allocation = match std::env::var("POSTE_FILS") {
        Ok(v) => v.parse::<usize>().map_err(|_| "invalid POSTE_FILS")?.min(8),
        Err(std::env::VarError::NotPresent) => 8,
        Err(_) => return Err("invalid POSTE_FILS".into()),
    };
    if jobs == 0 || jobs > allocation { return Err("private fixture Cargo jobs exceed the poste allocation".into()) }
    if manifest["workspace"]["commands"] != "approval" {
        return Err("private fixture must preserve command approval".into());
    }
    manifest["workspace"]["sandbox"] = sandbox;
    Ok(true)
}

/// Exercise the real fixture setup in a separate invocation of this test ELF.
/// Only the child sees the declared sandbox; other tests never see set_var.
#[cfg(test)]
pub fn witness_declared_sandbox_setup(test_name: &str, setup: impl FnOnce(&Path) -> PathBuf) {
    const DECLARATION: &str = "RAG3WEAVER_DOCUMENTATION_SANDBOX_FILE";
    const CHILD: &str = "RAG3WEAVER_DOCUMENTATION_SETUP_WITNESS";
    const ROOT: &str = "RAG3WEAVER_DOCUMENTATION_SETUP_ROOT";
    if std::env::var(CHILD).ok().as_deref() == Some(test_name) {
        let root = PathBuf::from(std::env::var_os(ROOT).expect("private witness root"));
        let declaration = PathBuf::from(std::env::var_os(DECLARATION).expect("private sandbox declaration"));
        assert_eq!(root.canonicalize().unwrap(), root);
        assert_eq!(declaration.parent(), Some(root.as_path()));
        let expected: Value = serde_json::from_slice(&std::fs::read(declaration).unwrap()).unwrap();
        let manifest = setup(&root.join("setup"));
        let actual: Value = serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
        assert_eq!(actual["workspace"]["commands"], "approval", "fixture command policy must stay approval");
        assert_eq!(actual["workspace"]["sandbox"], expected,
            "DOCUMENTATION_PRIVATE_SANDBOX_NOT_CONNECTED_BEFORE_ADD_FILES");
        let config = manifest.ancestors().find(|p| p.join(".git").exists()).expect("fixture config Git root");
        let relative = manifest.strip_prefix(config).unwrap().to_str().unwrap();
        let output = std::process::Command::new("git").arg("-C").arg(config)
            .args(["show", &format!("HEAD:{relative}")]).output().unwrap();
        assert!(output.status.success(), "fixture manifest must be committed before a task");
        let committed: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(committed["workspace"]["sandbox"], expected,
            "DOCUMENTATION_PRIVATE_SANDBOX_NOT_COMMITTED_BEFORE_TASK");
        let head = std::process::Command::new("git").arg("-C").arg(config)
            .args(["rev-parse", "HEAD"]).output().unwrap();
        assert!(head.status.success(), "the private fixture HEAD is readable");
        // Positive fields only: never publish the whole command environment.
        let sandbox = &committed["workspace"]["sandbox"];
        println!("DOCUMENTATION_PRIVATE_SANDBOX_COMMITTED {}", json!({
            "name": test_name,
            "manifest": manifest,
            "head": String::from_utf8_lossy(&head.stdout).trim(),
            "commands": committed["workspace"]["commands"],
            "extra_read": sandbox["extra_read"],
            "target_token": sandbox["env"]["CARGO_TARGET_DIR"],
            "tmp_token": sandbox["env"]["TMPDIR"],
            "cargo_jobs": sandbox["env"]["CARGO_BUILD_JOBS"],
            "offline": sandbox["env"]["CARGO_NET_OFFLINE"],
            "targets_root": std::env::var_os("RAG3WEAVER_TARGETS_DIR").map(PathBuf::from),
            "manifest_equals_declaration": true,
            "head_equals_declaration": true
        }));
        return;
    }

    let mut fixture = DocumentationFixture::new();
    let root = fixture.path().canonicalize().unwrap();
    let templates = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code/backend.json");
    let template: Value = serde_json::from_slice(&std::fs::read(templates).unwrap()).unwrap();
    let mut sandbox = template["workspace"]["sandbox"].clone();
    let q = root.join("private-controller");
    std::fs::create_dir(&q).unwrap();
    sandbox["extra_read"].as_array_mut().unwrap().push(json!(q));
    sandbox["env"]["CARGO_BUILD_JOBS"] = json!("1");
    sandbox["env"]["CARGO_TARGET_DIR"] = json!("{repo_target}");
    let declaration = root.join("sandbox.json");
    std::fs::write(&declaration, serde_json::to_vec_pretty(&sandbox).unwrap()).unwrap();
    let targets = root.join("private-session-targets");
    std::fs::create_dir(&targets).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([test_name, "--exact", "--nocapture", "--test-threads=1"])
        .env(DECLARATION, declaration).env(CHILD, test_name).env(ROOT, &root)
        .env("RAG3WEAVER_TARGETS_DIR", targets)
        .output().expect("run the actual setup in the same test ELF");
    std::fs::write(root.join("setup-child.stdout"), &output.stdout).unwrap();
    std::fs::write(root.join("setup-child.stderr"), &output.stderr).unwrap();
    if !output.status.success() {
        fixture.fail(&format!("DOCUMENTATION_PRIVATE_SANDBOX_SETUP_REFUSED: {}\n{}\n{}",
            output.status, String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)), &[]);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "the setup child must actually judge this exact witness: {stdout}");
    println!("DOCUMENTATION_SETUP_CHILD_OK {test_name}\n{stdout}");
    if !output.stderr.is_empty() {
        eprintln!("DOCUMENTATION_SETUP_CHILD_STDERR {test_name}\n{}", String::from_utf8_lossy(&output.stderr));
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
