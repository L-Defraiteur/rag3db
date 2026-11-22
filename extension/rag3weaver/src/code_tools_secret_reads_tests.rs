//! **The read tools never show a secret file** (22 Nov 2026: in the dry run of the auto mode, « Montre-moi le .env »
//! went through `read_file`, and the value was in the mailbox; the command guard only saw `cat`). A module of its own.

use super::*;

const KEY: &str = "sk-witness-0123456789abcdef";

fn source() -> Snapshot {
    Snapshot::new(
        "t",
        [
            (".env".to_string(), format!("OPENAI_API_KEY={KEY}\n")),
            ("config/prod.pem".to_string(), format!("-----BEGIN PRIVATE KEY-----\n{KEY}\n")),
            (".env.example".to_string(), "OPENAI_API_KEY=\n".to_string()),
            ("src/main.rs".to_string(), "fn main() {}\n".to_string()),
        ],
    )
}

#[test]
fn read_file_refuses_a_secret_and_says_how_to_use_it() {
    for path in [".env", "config/prod.pem"] {
        let r = read_file(&source(), None, path, 1, 50);
        let shown = format!("{r:?}");
        assert!(!shown.contains(KEY), "{path} shown: {shown}");
        let e = r.expect_err("a secret file is refused");
        assert!(e.contains("showing a secret is refused") && e.contains("secret_store"), "{e}");
    }
    assert!(read_file(&source(), None, ".env.example", 1, 50).is_ok(), "an example file is not a secret");
}

#[test]
fn a_raw_read_refuses_a_secret_too() {
    let r = read_file_raw(&source(), &RootPolicy::closed(), ".env");
    assert!(!format!("{r:?}").contains(KEY), "{r:?}");
    assert!(r.is_err());
}

#[test]
fn grep_never_searches_a_secret_file() {
    let g = grep_files(&source(), None, "OPENAI_API_KEY", &GrepOptions::default()).unwrap();
    let shown = serde_json::to_string(&g).unwrap();
    assert!(!shown.contains(KEY), "{shown}");
    assert!(g.matches.iter().all(|m| m.path == ".env.example"), "{shown}");
}
