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
            (vec![format!("cat {}", root.join("notes.txt").display())], "Lis les notes du dossier « ailleurs », à côté du projet.".to_string())
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

/// **A root the judge grants is said in the mailbox** (tty and archi, 22 Nov 2026): unlike a line's grant, it lasts;
/// the user sees it, with the root, its mode and the words it rests on.
#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_root_the_judge_grants_is_said_in_the_mailbox() {
    let mut root = std::path::PathBuf::new();
    let v = v1_with(
        |d| {
            root = ailleurs(d);
            (vec![format!("cat {}", root.join("notes.txt").display())], "Lis les notes du dossier « ailleurs », à côté du projet.".to_string())
        },
        &["les notes"],
    );
    let b = v.wait("the task ends", &ended);
    let dit = b.iter().find(|l| l["from"] == "host" && l["kind"] == "root_granted").unwrap_or_else(|| panic!("the grant is said: {b:#?}"));
    assert_eq!(dit["root"], json!(root.display().to_string()), "{dit}");
    assert_eq!(dit["by"], json!("judge"), "{dit}");
    assert!(dit["text"].as_str().unwrap_or_default().contains("racine accordée par le juge"), "{dit}");
}

/// **A command the judge lets through is said in the mailbox** (c7, 22 Nov 2026: a pass by the judge left no trace, an
/// audit's hole): a host line `command_granted`, with the command, its class, by judge, and the words it rests on.
#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_command_the_judge_lets_through_is_said_in_the_mailbox() {
    let v = super::mode_auto_v1::v1_with(|_| (vec!["rm vieux.txt".to_string()], "Supprime vieux.txt, il ne sert plus.".to_string()), &["supprime vieux.txt"]);
    let b = v.wait("the task ends", &ended);
    let dit = b.iter().find(|l| l["from"] == "host" && l["kind"] == "command_granted").unwrap_or_else(|| panic!("the pass is said: {b:#?}"));
    assert_eq!(dit["command"], json!("rm vieux.txt"), "{dit}");
    assert_eq!(dit["class"], json!("destroy_local"), "{dit}");
    assert_eq!(dit["by"], json!("judge"), "{dit}");
    assert!(dit["quote"].as_str().unwrap_or_default().contains("Supprime vieux.txt"), "{dit}");
}

/// **A protected folder's card gives the agent's own reason**, never the text of a card before it (recherche's audit).
#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn the_agents_reason_is_never_a_card() {
    let d = tempfile::tempdir().unwrap();
    let f = crate::daemon_host::teammate::TeammateFiles::new(d.path().to_path_buf(), "p/t");
    f.append_with("teammate", "report", None, "Je vais lire le dépôt.", &json!({})).unwrap();
    f.append_with("teammate", "approval", None, "commande en attente d'accord : git status", &json!({ "command": "git status" })).unwrap();
    assert_eq!(crate::daemon_host::gate::last_agent_turn(&f), "Je vais lire le dépôt.");
}
