//! **judge_all: a path outside the sandbox's roots goes to the judge, then the card** (Lucie, 21 Nov 2026, 20 h; page 07
//! of rag3weaver-archi, « suite »). The sandbox stops it with an `open_root` request; the gate asks the judge, whose yes
//! grants the root to the task's agent and replays the command; anything else is a card of class `open_root`, whose
//! yes grants it too. A root broader than a project goes to the card with a warning, never to the judge. A module of
//! its own, on the auto mode's scenario.

use super::mode_auto_v1::{ended, v1_with};
use super::*;

/// A folder outside the task's roots, with a file to read.
fn ailleurs(d: &std::path::Path) -> std::path::PathBuf {
    let a = d.join("ailleurs");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::write(a.join("notes.txt"), "les notes d'ailleurs\n").unwrap();
    a.canonicalize().unwrap()
}

fn granted(v: &super::mode_auto_v1::V1, root: &std::path::Path) -> Vec<String> {
    v.host.root_grants(None).into_iter().filter(|g| g.root == root).map(|g| g.granted_by).collect()
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn an_outside_path_the_user_asked_for_is_granted_by_the_judge_and_replayed() {
    let mut root = std::path::PathBuf::new();
    let v = v1_with(
        |d| {
            root = ailleurs(d);
            (vec![format!("cat {}", root.join("notes.txt").display())], format!("Lis les notes dans {}.", root.display()))
        },
        &["les notes"],
    );
    v.wait("the task ends", &ended);
    let judged = v.judged.lock().unwrap().clone();
    assert!(judged.iter().any(|i| i["class"] == "open_root" && i["action"]["open_root"]["path"] == json!(root.display().to_string())), "the judge saw the root: {judged:#?}");
    assert_eq!(granted(&v, &root), ["judge"], "granted by the judge to the task's agent");
    assert!(v.seen.lock().unwrap().iter().any(|r| r.contains("les notes d'ailleurs")), "the command was replayed: {:?}", v.seen.lock().unwrap());
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn an_outside_path_nobody_asked_for_opens_a_card_and_a_yes_grants_it() {
    let mut root = std::path::PathBuf::new();
    let v = v1_with(
        |d| {
            root = ailleurs(d);
            (vec![format!("cat {}", root.join("notes.txt").display())], "Lis le README.".to_string())
        },
        &["les notes"],
    );
    let b = v.wait("the card", &|b| b.iter().any(|l| l["kind"] == "approval"));
    let card = b.iter().find(|l| l["kind"] == "approval").unwrap().clone();
    assert_eq!(card["class"], json!("open_root"), "{card}");
    assert_eq!(card["args"]["root"], json!(root.display().to_string()), "{card}");
    assert!(granted(&v, &root).is_empty(), "nothing granted before the answer");
    v.host.post_to_teammate(&v.id, "doc", "lucie", "answer", None, "oui", &json!({ "reply_to": card["id"], "approved": true, "by": { "kind": "human", "client": "tty" } })).unwrap();
    v.wait("the task ends", &ended);
    assert_eq!(granted(&v, &root), ["user"], "the human's yes grants the root");
    assert!(v.seen.lock().unwrap().iter().any(|r| r.contains("les notes d'ailleurs")), "replayed after the yes: {:?}", v.seen.lock().unwrap());
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_root_broader_than_a_project_goes_to_the_card_with_its_warning() {
    let mut large = std::path::PathBuf::new();
    let v = v1_with(
        |d| {
            large = d.join("projets");
            for p in ["un", "deux"] {
                std::fs::create_dir_all(large.join(p).join(".git")).unwrap();
            }
            large = large.canonicalize().unwrap();
            (vec![format!("ls {}", large.display())], format!("Regarde mes projets dans {}.", large.display()))
        },
        &["projets"],
    );
    let b = v.wait("the card", &|b| b.iter().any(|l| l["kind"] == "approval"));
    let card = b.iter().find(|l| l["kind"] == "approval").unwrap().clone();
    assert_eq!(card["class"], json!("open_root"), "{card}");
    let warning = card["args"]["warning"].as_str().unwrap_or_default();
    assert!(warning.contains("un") && warning.contains("deux"), "the card says what the root holds: {card}");
    assert!(v.judged.lock().unwrap().iter().all(|i| i["class"] != "open_root"), "a broad root never goes to the judge");
}
