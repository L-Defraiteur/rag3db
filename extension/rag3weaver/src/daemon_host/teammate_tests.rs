//! **L'équipier sur faux modèle** (page 07 du 25 octobre 2026) : la boucle à
//! pas, la boîte aux lettres, le budget, les plans et l'arrêt, sur un petit
//! dépôt git qui n'est pas rag3weaver.

use super::chat::ChatModel;
use super::chat_qui_code::{depot_inscrit, hote};
use super::teammate::{PlanState, Status};
use crate::llm::{CallbackLlm, Llm, MockLlm, Turn};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

/// Le numéro du pas, lu dans le message de la frontière.
fn step_of(turns: &[Turn]) -> usize {
    turns
        .iter()
        .rev()
        .find(|t| t.role == "user")
        .and_then(|t| t.content.strip_prefix("[Pas "))
        .and_then(|s| s.split(']').next())
        .and_then(|n| n.trim().parse().ok())
        .unwrap_or(0)
}

fn last_user(turns: &[Turn]) -> String {
    turns.iter().rev().find(|t| t.role == "user").map(|t| t.content.clone()).unwrap_or_default()
}

/// Les résultats d'outils du tour en cours (depuis le dernier message de l'usager).
fn tool_results(turns: &[Turn]) -> Vec<String> {
    let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
    turns[depuis..].iter().filter(|t| t.role == "tool").map(|t| t.content.clone()).collect()
}

fn calls(v: &[(&str, Value)]) -> Vec<(String, String)> {
    v.iter().map(|(n, a)| (n.to_string(), a.to_string())).collect()
}

fn answer(turns: &[Turn], opts: &crate::llm::GenOptions, sink: &mut dyn crate::llm::TokenSink, text: &str, tool_calls: Vec<(String, String)>) -> Result<(crate::llm::Finish, crate::llm::Usage), crate::llm::LlmError> {
    // The mock counts whitespace-separated fragments, with a window of its own (4 096 by default).
    // The fake teammates declare up to 16 384; a read given to the model as text (with its newlines)
    // counts more fragments than the same read escaped in JSON (`\n` is no space), and step 2 of
    // the three-item plan overflowed the inner mock's window, not the teammate's (19 Nov. 2026).
    MockLlm::new(text).with_context_len(16384).with_tool_calls(tool_calls).generate(turns, opts, sink)
}

/// Ce que le faux modèle a vu : les frontières, pas par pas, et les refus.
#[derive(Default)]
struct Seen {
    frontiers: Vec<String>,
    refusals: Vec<String>,
}

/// **Un plan à trois éléments, dont un à valider** : la question part dans la
/// boîte au pas 1 ; la réponse du superviseur, écrite pendant le pas 1, est lue
/// à la frontière du pas 2 ; le pas 2 s'étire (treize lectures) et le budget
/// force `step_done` ; les trois éléments se font en trois pas ; le dernier pas
/// rend compte et clôt le plan.
fn three_items(seen: Arc<Mutex<Seen>>, mailbox: std::path::PathBuf) -> ChatModel {
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        let (seen, mailbox) = (seen.clone(), mailbox.clone());
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 16384, move |turns, opts, sink| {
            let step = step_of(turns);
            let results = tool_results(turns);
            if results.is_empty() {
                seen.lock().unwrap().frontiers.push(last_user(turns));
            }
            for r in &results {
                if r.contains("declare step_done now") {
                    seen.lock().unwrap().refusals.push(r.clone());
                }
            }
            let edit = |path: &str, text: &str| json!({ "path": path, "content": text });
            match (step, results.len()) {
                // Pas 1 : le plan, puis le README, puis step_done. Entre les deux,
                // « le superviseur » répond au bloquant dans la boîte.
                (1, 0) => answer(turns, opts, sink, "", calls(&[("edit_plan", json!({ "plan": {
                    "request": "Écrire la doc du petit dépôt : README, CHANGELOG, et le titre à valider",
                    "proposal": "Trois fichiers courts, dans l'ordre",
                    "items": [
                        { "id": "readme", "text": "Écrire README.md", "status": "todo", "alone": true },
                        { "id": "changelog", "text": "Écrire CHANGELOG.md", "status": "todo", "alone": true },
                        { "id": "titre", "text": "Poser le titre du README", "status": "blocked", "alone": false, "question": "Quel titre pour le README ?" }
                    ] } }))])),
                (1, 1) => {
                    use std::io::Write;
                    let mut f = std::fs::OpenOptions::new().append(true).open(&mailbox).unwrap();
                    writeln!(f, "{}", json!({ "at": 1, "from": "supervisor", "kind": "answer", "item": "titre", "text": "Le titre : « Petit dépôt »" })).unwrap();
                    answer(turns, opts, sink, "", calls(&[("edit_file", edit("README.md", "# Doc\n"))]))
                }
                (1, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "readme", "succeeded": true, "done": "README écrit", "remaining": "" }))])),
                // Pas 2 : la réponse est là ; l'élément « titre » devient faisable ;
                // la borne de sûreté plus une lectures d'un coup : la dernière est refusée.
                (2, 0) => {
                    let mut v: Vec<(String, String)> = vec![("edit_plan".into(), json!({ "item": { "id": "titre", "status": "todo", "alone": true, "reason": "le superviseur a répondu" } }).to_string())];
                    for _ in 0..crate::backend::TeammatePolicy::default().max_tool_calls_per_step {
                        v.push(("read_file".into(), json!({ "path": "src/lib.rs" }).to_string()));
                    }
                    answer(turns, opts, sink, "", v)
                }
                (2, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "changelog", "succeeded": false, "done": "j'ai relu le dépôt", "remaining": "écrire le CHANGELOG" }))])),
                // Pas 3 : le CHANGELOG. Pas 4 : le titre.
                (3, 0) => answer(turns, opts, sink, "", calls(&[
                    ("edit_file", edit("CHANGELOG.md", "# Changelog\n")),
                    ("suggest", json!({ "text": "Voudrais-tu aussi un CONTRIBUTING.md ? Ce n'est pas dans la demande.", "item": "contrib" })),
                ])),
                (3, 2) => {
                    use std::io::Write;
                    let mut f = std::fs::OpenOptions::new().append(true).open(&mailbox).unwrap();
                    writeln!(f, "{}", json!({ "at": 2, "from": "supervisor", "kind": "answer", "item": "contrib", "text": "Oui, vas-y pour le CONTRIBUTING." })).unwrap();
                    answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "changelog", "succeeded": true, "done": "CHANGELOG écrit ; CONTRIBUTING proposé", "remaining": "" }))]))
                }
                (3, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "changelog", "succeeded": true, "done": "CHANGELOG écrit", "remaining": "" }))])),
                (4, 0) => answer(turns, opts, sink, "", calls(&[
                    ("edit_plan", json!({ "item": { "id": "contrib", "text": "Écrire CONTRIBUTING.md (proposé, accepté)", "status": "todo", "alone": true } })),
                    ("edit_file", json!({ "path": "README.md", "old": "# Doc", "new": "# Petit dépôt" })),
                ])),
                (4, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "titre", "succeeded": true, "done": "titre posé", "remaining": "" }))])),
                (5, 0) => answer(turns, opts, sink, "", calls(&[("edit_file", edit("CONTRIBUTING.md", "# Contribuer\n"))])),
                (5, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "contrib", "succeeded": true, "done": "CONTRIBUTING écrit", "remaining": "" }))])),
                // Pas 6 : plus rien : compte rendu, clôture, step_done.
                (6, 0) => answer(turns, opts, sink, "", calls(&[
                    ("report", json!({ "text": "La doc est écrite : README (titre validé), CHANGELOG." })),
                    ("edit_plan", json!({ "close": { "outcome": "accomplished", "reason": "les trois éléments sont faits", "concepts": ["documentation"] } })),
                ])),
                (6, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "compte rendu", "remaining": "" }))])),
                _ => answer(turns, opts, sink, "Je n'ai plus rien à faire.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "", "remaining": "" }))])),
            }
        })))
    })
}

#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_teammate_works_a_three_item_plan_step_by_step() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let seen = Arc::new(Mutex::new(Seen::default()));
    // La boîte vit dans le dossier de la tâche : le faux modèle (qui y écrit la
    // réponse du superviseur) la reçoit une fois la tâche ouverte.
    let host = hote(d.path(), three_items(seen.clone(), d.path().join("placeholder"))).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let t = host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let host = host.with_chat_model(three_items(seen.clone(), files.mailbox_path()));
    files.append("supervisor", "request", None, "Écris la doc du petit dépôt : un README, un CHANGELOG ; le titre reste à valider.").unwrap();
    let outcome = host.run_teammate(&h.id, "doc").expect("l'équipier tourne");
    assert_eq!(outcome.steps.len(), 6, "{outcome:?}");
    assert_eq!(outcome.ended, "plus rien à faire : compte rendu");

    // La boîte : la question au pas 1, la réponse, quatre step_done, le compte rendu, la fin.
    let boite = files.mailbox();
    let kinds: Vec<(String, String)> = boite.iter().map(|l| (l["from"].as_str().unwrap_or("").into(), l["kind"].as_str().unwrap_or("").into())).collect();
    assert_eq!(kinds[0], ("supervisor".into(), "request".into()));
    assert_eq!(kinds[1], ("teammate".into(), "question".into()), "{kinds:?}");
    assert_eq!(boite[1]["item"], "titre");
    assert_eq!(boite.iter().filter(|l| l["kind"] == "step_done").count(), 6, "{kinds:?}");
    // Le hors-plan : une proposition pendant le pas 3, le plan continue (le
    // step_done du pas 3 suit la proposition), et le « oui » à la frontière du
    // pas 4 ouvre l'élément.
    let proposition = boite.iter().position(|l| l["kind"] == "proposal").expect("la proposition dans la boîte");
    assert_eq!(boite[proposition]["item"], "contrib");
    assert!(boite[proposition + 1..].iter().any(|l| l["kind"] == "step_done" && l["item"] == "changelog"), "le plan continue après la proposition : {kinds:?}");
    assert!(boite.iter().any(|l| l["kind"] == "report" && l["from"] == "teammate"), "{kinds:?}");
    assert_eq!(kinds.last().unwrap(), &("host".into(), "ended".into()));

    // La réponse écrite pendant le pas 1 est lue à la frontière du pas 2.
    let seen = seen.lock().unwrap();
    assert!(seen.frontiers[1].contains("Le titre : « Petit dépôt »") && seen.frontiers[1].starts_with("[Pas 2]"), "{}", seen.frontiers[1]);
    assert!(seen.frontiers[0].contains("Réponds ou agis"), "{}", seen.frontiers[0]);
    // La borne de sûreté : la dernière lecture refusée, en nommant step_done.
    assert_eq!(seen.refusals.len(), 1, "{:?}", seen.refusals);
    assert!(seen.refusals[0].contains("step_done"));
    assert!(seen.frontiers[3].contains("Oui, vas-y pour le CONTRIBUTING") && seen.frontiers[3].contains("(élément contrib)"), "{}", seen.frontiers[3]);
    // Le dernier pas sait qu'il ne reste rien.
    assert!(seen.frontiers[5].contains("rends compte"), "{}", seen.frontiers[5]);

    // Deux runs journalisés au moins, chacun avec son step_done et la demande.
    let runs: Vec<String> = outcome.steps.iter().filter_map(|s| s["run"].as_str().map(str::to_string)).collect();
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(10);
    for run in &runs[..2] {
        let j = loop {
            if let Some(j) = host.journal_des_runs().lire(&t.id, run).filter(|j| j["tours"].is_array()) {
                break j;
            }
            assert!(std::time::Instant::now() < fin, "le journal du pas {run}");
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        assert!(j["demande"].as_str().is_some_and(|d| d.starts_with("Écris la doc")), "{}", j["demande"]);
        assert!(j["evenements"].as_array().unwrap().iter().any(|e| e["kind"] == "ToolCallStarted" && e["tool"] == "step_done"), "step_done dans le journal du pas");
    }
    assert_eq!(outcome.steps[0]["step_done"]["item"], "readme");
    assert_eq!(outcome.steps[1]["step_done"]["succeeded"], false);

    // Le plan clos, avec ses pas et ses notions ; plus de plan vivant.
    let clos = files.closed();
    assert_eq!(clos.len(), 1);
    assert_eq!((clos[0].state, clos[0].items.iter().filter(|i| i.status == Status::Done).count(), clos[0].concepts.clone()), (PlanState::Accomplished, 4, vec!["documentation".to_string()]));
    assert_eq!(clos[0].steps.len(), 6, "{:?}", clos[0].steps);
    assert!(files.read_plans().plans.is_empty());
    // Les fichiers sont sur la branche de la tâche.
    let racine = &t.workspace.as_ref().unwrap().racine;
    assert_eq!(std::fs::read_to_string(racine.join("README.md")).unwrap(), "# Petit dépôt\n");
    assert!(racine.join("CHANGELOG.md").is_file() && racine.join("CONTRIBUTING.md").is_file());
}

/// **Le plan n'est pas obligatoire, et un plan actif oblige** (Lucie) : une
/// petite demande se fait sans plan (un `step_done`, pas de `plan.json`) ; une
/// demande longue ouvre un plan ; une demande nouvelle pendant ce plan le met en
/// pause ; la frontière suivante rappelle le plan en pause.
fn small_then_long(seen: Arc<Mutex<Seen>>, mailbox: std::path::PathBuf) -> ChatModel {
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        let (seen, mailbox) = (seen.clone(), mailbox.clone());
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 16384, move |turns, opts, sink| {
            let step = step_of(turns);
            let results = tool_results(turns);
            let frontier = last_user(turns);
            if results.is_empty() {
                seen.lock().unwrap().frontiers.push(frontier.clone());
            }
            let second = frontier.contains("Écris la doc complète") || frontier.contains("plan actif") || frontier.contains("plans en pause") || frontier.contains("décide d'abord");
            match (second, step, results.len()) {
                // La petite demande : on répond, step_done, sans plan.
                (false, _, 0) => answer(turns, opts, sink, "Bonjour !", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "salué", "remaining": "" }))])),
                // La demande longue : un plan ; pendant le pas, une demande nouvelle arrive.
                (true, 1, 0) => answer(turns, opts, sink, "", calls(&[("edit_plan", json!({ "plan": {
                    "request": "Écrire la doc complète", "proposal": "README puis CONTRIBUTING",
                    "items": [{ "id": "readme", "text": "README", "status": "todo" }, { "id": "contrib", "text": "CONTRIBUTING", "status": "todo" }] } }))])),
                (true, 1, 1) => {
                    use std::io::Write;
                    let mut f = std::fs::OpenOptions::new().append(true).open(&mailbox).unwrap();
                    writeln!(f, "{}", json!({ "at": 1, "from": "supervisor", "kind": "request", "text": "Centre le div, c'est urgent." })).unwrap();
                    answer(turns, opts, sink, "", calls(&[("edit_file", json!({ "path": "README.md", "content": "# Doc\n" }))]))
                }
                (true, 1, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "readme", "succeeded": true, "done": "README", "remaining": "" }))])),
                // Pas 2 : la frontière demande une décision : pause, puis le petit plan du div.
                (true, 2, 0) => answer(turns, opts, sink, "", calls(&[("edit_plan", json!({ "pause": { "reason": "le superviseur demande le div d'abord" }, "plan": {
                    "request": "Centrer le div", "proposal": "une règle CSS", "items": [{ "id": "div", "text": "centrer", "status": "todo" }] } }))])),
                (true, 2, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "div", "succeeded": true, "done": "centré", "remaining": "" }))])),
                // Pas 3 : le plan 2 n'a plus rien ; la frontière liste le plan 1 en pause :
                // on clôt le 2 et on reprend le 1.
                (true, 3, 0) => answer(turns, opts, sink, "", calls(&[("edit_plan", json!({ "close": { "outcome": "accomplished", "reason": "centré" }, "resume": 1 }))])),
                (true, 3, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "contrib", "succeeded": true, "done": "CONTRIBUTING écrit (plan repris)", "remaining": "" }))])),
                (true, _, 0) => answer(turns, opts, sink, "", calls(&[("report", json!({ "text": "Tout est fait." })), ("edit_plan", json!({ "close": { "outcome": "accomplished", "reason": "fait" } }))])),
                _ => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "", "remaining": "" }))])),
            }
        })))
    })
}

#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_small_request_needs_no_plan_and_an_active_plan_is_paused_then_recalled() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let seen = Arc::new(Mutex::new(Seen::default()));
    let host = hote(d.path(), small_then_long(seen.clone(), d.path().join("placeholder"))).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let host = host.with_chat_model(small_then_long(seen.clone(), files.mailbox_path()));

    // 1. « Bonjour » : un pas, un step_done, pas de plan.
    files.append("supervisor", "request", None, "Bonjour").unwrap();
    let o = host.run_teammate(&h.id, "doc").unwrap();
    assert_eq!((o.steps.len(), o.ended.as_str()), (1, "fait sans plan"), "{o:?}");
    assert!(!files.plan_path().exists() && files.closed().is_empty());
    assert_eq!(files.mailbox().iter().filter(|l| l["kind"] == "step_done").count(), 1);

    // 2. La demande longue : un plan, une demande nouvelle qui le met en pause,
    //    la frontière qui le rappelle, la reprise.
    files.append("supervisor", "request", None, "Écris la doc complète du dépôt : README et CONTRIBUTING.").unwrap();
    let o = host.run_teammate(&h.id, "doc").unwrap();
    assert_eq!(o.ended, "plus rien à faire : compte rendu", "{o:?}");
    let seen = seen.lock().unwrap();
    // La relance ne relit pas « Bonjour ».
    assert!(!seen.frontiers[1].contains("Bonjour"), "{}", seen.frontiers[1]);
    let f2 = &seen.frontiers[2];
    assert!(f2.contains("Centre le div") && f2.contains("décide d'abord"), "{f2}");
    assert_eq!(o.steps[1]["decision"], "pause", "{:?}", o.steps[1]);
    let f3 = &seen.frontiers[3];
    assert!(f3.contains("plans en pause") && f3.contains("Écrire la doc complète") && f3.contains("le superviseur demande le div d'abord"), "{f3}");
    let clos = files.closed();
    assert_eq!(clos.iter().map(|p| (p.number, p.state)).collect::<Vec<_>>(), vec![(1, PlanState::Accomplished), (2, PlanState::Accomplished)]);
    assert!(files.read_plans().plans.is_empty());
}

/// **L'arrêt passe tout de suite** : un `stop` dans la boîte pendant un pas
/// arrête le run en cours ; l'équipier ne fait pas de pas suivant et la boîte
/// dit la fin.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_stop_in_the_mailbox_cuts_the_running_step() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let (started_tx, started_rx) = std::sync::mpsc::channel::<()>();
    let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
    let go_rx = Arc::new(Mutex::new(go_rx));
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        let (started_tx, go_rx) = (started_tx.clone(), go_rx.clone());
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            if tool_results(turns).is_empty() {
                return answer(turns, opts, sink, "", calls(&[("read_file", json!({ "path": "src/lib.rs" }))]));
            }
            // Le pas est en route : on le dit, et on attend l'arrêt.
            let _ = started_tx.send(());
            let _ = go_rx.lock().unwrap().recv_timeout(std::time::Duration::from_secs(10));
            answer(turns, opts, sink, "", calls(&[("read_file", json!({ "path": "README.md" }))]))
        })))
    });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let r = host.post_to_teammate(&h.id, "doc", "supervisor", "request", None, "Lis tout le dépôt et résume-le.", &Value::Null).expect("la request ouvre la tâche et lance l'équipier");
    assert_eq!(r["started"], true, "{r}");
    started_rx.recv_timeout(std::time::Duration::from_secs(20)).expect("le pas démarre");
    let r = host.post_to_teammate(&h.id, "doc", "c7", "stop", None, "", &Value::Null).expect("le stop");
    assert_eq!(r["started"], false, "{r}");
    let _ = go_tx.send(());
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while host.teammate_running(&h.id, "doc") {
        assert!(std::time::Instant::now() < fin, "l'équipier ne s'arrête pas");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let boite = files.mailbox();
    let ended = boite.iter().find(|l| l["kind"] == "ended").expect("la fin dans la boîte");
    assert!(ended["text"].as_str().unwrap().starts_with("arrêté par"), "{ended}");
    assert!(!boite.iter().any(|l| l["kind"] == "step_done"), "aucun pas n'a fini de lui-même : {boite:?}");
}

/// **La boîte adressée, par le chemin de l'hôte** (page 07-novembre-2026-everything-declarative/01,
/// § 2) : une `request` postée sous `claude-code:s1` ouvre la tâche ; la question de l'équipier
/// attend `claude-code:s1` dans `pending` (et pas une autre clé), deux lectures n'y changent rien ;
/// la réponse par `reply_to` la sort et l'équipier la lit à sa frontière ; la fin attend d'être
/// vue, `seen` la sort ; `list` dit l'état ; chaque ligne porte son `id` et passe sur le flux en
/// `MailboxLine`.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_addressed_mailbox_by_the_host_routes() {
    use std::time::{Duration, Instant};
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let frontieres = Arc::new(Mutex::new(Vec::<String>::new()));
    let vues = frontieres.clone();
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        let vues = vues.clone();
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            let results = tool_results(turns);
            if results.is_empty() {
                vues.lock().unwrap().push(last_user(turns));
            }
            match (step_of(turns), results.len()) {
                (1, 0) => answer(turns, opts, sink, "", calls(&[("edit_plan", json!({ "plan": {
                    "request": "Le titre du README", "proposal": "Demander le titre, puis l'écrire",
                    "items": [{ "id": "titre", "text": "Poser le titre", "status": "blocked", "alone": false, "question": "Quel titre pour le README ?" }] } }))])),
                (1, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "question posée", "remaining": "le titre" }))])),
                // Le pas d'après la réponse (le pas 2 : l'attente ne consomme pas de
                // numéro, elle ne fait que suspendre l'équipier).
                (_, 0) => answer(turns, opts, sink, "", calls(&[("edit_plan", json!({ "close": { "outcome": "accomplished", "reason": "le titre est donné", "concepts": ["documentation"] } }))])),
                _ => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "plan clos", "remaining": "" }))])),
            }
        })))
    });
    let id = crate::daemon_host::DaemonHost::new().hosted(&manifeste).unwrap().id.clone();
    let (taches, chats) = (d.path().join("taches"), d.path().join("chats"));
    let (adresse, _fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| h.with_racine_des_taches(taches).with_chats_root(chats).with_chat_model(model));
    let racine = format!("http://{adresse}");
    let get = |route: &str| -> Value {
        let mut r = crate::daemon::agent_de_temoin().get(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    let post = |route: &str, corps: Value| -> Value {
        let mut r = crate::daemon::agent_de_temoin()
            .post(format!("{racine}{route}"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(corps.to_string())
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    let decrit = post("/ide/describe", json!({ "manifest": manifeste }));
    assert_eq!(decrit["ok"], true, "{decrit}");
    let attendre = |quoi: &str, ok: &dyn Fn() -> Option<Value>| -> Value {
        let borne = Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(v) = ok() {
                return v;
            }
            assert!(Instant::now() < borne, "en 60 s : {quoi}");
            std::thread::sleep(Duration::from_millis(50));
        }
    };

    let r = post("/ide/teammate/mailbox", json!({ "backend": id, "task": "doc", "from": "claude-code:s1", "kind": "request", "text": "Écris le titre du README." }));
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(r["result"]["line"]["id"], format!("{id}/doc:0"), "la première ligne a le rang 0 : {r}");

    // La question attend s1, et elle seule ; deux lectures ne la consomment pas.
    let pending = |who: &str| get(&format!("/ide/teammate/pending?for={who}"))["result"]["items"].as_array().cloned().unwrap_or_default();
    let question = attendre("la question de l'équipier dans pending", &|| pending("claude-code:s1").into_iter().find(|i| i["kind"] == "question"));
    assert_eq!((question["task"].clone(), question["parent"].clone(), question["item"].clone(), question["from"].clone()), (json!("doc"), json!(id), json!("titre"), json!("teammate")), "{question}");
    let qid = question["id"].as_str().unwrap().to_string();
    assert!(qid.starts_with(&format!("{id}/doc:")), "{qid}");
    assert!(pending("claude-code:s2").is_empty(), "une autre clé ne voit rien");
    assert!(pending("claude-code:s1").iter().any(|i| i["id"] == qid.as_str()), "une seconde lecture la rend encore");
    let liste = attendre("l'équipier qui attend la boîte", &|| {
        let l = get("/ide/teammate/list?supervisor=claude-code:s1")["result"]["teammates"].as_array().cloned().unwrap_or_default();
        l.into_iter().find(|t| t["task"] == "doc" && t["state"] == "waiting")
    });
    assert_eq!(liste["parent"], json!(id), "{liste}");

    // La réponse vise la question ; elle sort de pending, l'équipier la lit.
    let r = post("/ide/teammate/mailbox", json!({ "backend": id, "task": "doc", "from": "claude-code:s1", "kind": "answer", "reply_to": qid, "text": "Le titre : « Petit dépôt »" }));
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(r["result"]["line"]["reply_to"], json!(qid), "{r}");
    let fin = attendre("la fin de l'équipier dans pending", &|| pending("claude-code:s1").into_iter().find(|i| i["kind"] == "ended"));
    assert!(!pending("claude-code:s1").iter().any(|i| i["id"] == qid.as_str()), "la question répondue est sortie");
    assert!(frontieres.lock().unwrap().iter().any(|f| f.contains("Petit dépôt")), "la réponse lue à la frontière : {:?}", frontieres.lock().unwrap());

    // La fin sort par seen, pas par une lecture.
    let fid = fin["id"].as_str().unwrap().to_string();
    assert!(pending("claude-code:s1").iter().any(|i| i["id"] == fid.as_str()), "relue, la fin est encore là");
    let r = post("/ide/teammate/seen", json!({ "for": "claude-code:s1", "ids": [fid] }));
    assert_eq!(r["result"]["seen"], 1, "{r}");
    assert!(!pending("claude-code:s1").iter().any(|i| i["kind"] == "ended"), "vue, la fin est sortie");
    let etat = get("/ide/teammate/list?supervisor=claude-code:s1")["result"]["teammates"][0].clone();
    assert_eq!(etat["state"], "ended", "{etat}");

    // La boîte entière : un id par ligne, le step_done avec ses champs.
    let equipier = get(&format!("/ide/teammate?backend={id}&task=doc"))["result"].clone();
    let manifeste_de_la_tache = equipier["backend"].as_str().expect("le manifeste de la tâche").to_string();
    let boite = equipier["mailbox"].as_array().cloned().unwrap();
    let ids: Vec<String> = boite.iter().map(|l| l["id"].as_str().unwrap().to_string()).collect();
    assert_eq!(ids, (0..boite.len()).map(|n| format!("{id}/doc:{n}")).collect::<Vec<_>>(), "un id par rang : {boite:?}");
    let pas = boite.iter().find(|l| l["kind"] == "step_done").expect("un step_done");
    assert_eq!((pas["succeeded"].clone(), pas["done"].clone()), (json!(true), json!("question posée")), "{pas}");

    // Le flux : chaque ligne y est passée en MailboxLine, la ligne en objet.
    let flux = {
        use std::io::{Read, Write};
        let mut s = std::net::TcpStream::connect(&adresse).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        s.write_all(b"GET /ide/flux?sujets=mailbox&depuis=0 HTTP/1.1\r\nHost: x\r\nCookie: rag3weaver_ide=j\r\n\r\n").unwrap();
        let (mut lu, mut tampon) = (Vec::new(), [0u8; 8192]);
        while !String::from_utf8_lossy(&lu).contains(&format!("\"id\":\"{fid}\"")) {
            match s.read(&mut tampon) {
                Ok(0) | Err(_) => break,
                Ok(n) => lu.extend_from_slice(&tampon[..n]),
            }
        }
        String::from_utf8_lossy(&lu).into_owned()
    };
    let relayees: Vec<Value> = flux.lines().filter_map(|l| l.strip_prefix("data: ")).filter_map(|l| serde_json::from_str(l).ok()).collect();
    assert!(relayees.iter().all(|e: &Value| e["sujet"] == "mailbox" && e["evenement"]["kind"] == "MailboxLine" && e["backend"] == manifeste_de_la_tache.as_str()), "{relayees:?}");
    let relayes: Vec<&str> = relayees.iter().filter_map(|e| e["evenement"]["line"]["id"].as_str()).collect();
    for i in [qid.as_str(), fid.as_str()] {
        assert!(relayes.contains(&i), "{i} sur le flux : {relayes:?}");
    }
    eprintln!("boîte adressée : {} lignes, {} relayées sur le flux", boite.len(), relayes.len());
}

/// **Un backend qui ne s'ouvre plus : le pas se fait sur les fichiers** (page
/// 07-novembre-2026-everything-declarative/01, § 1). La base de la tâche est fermée puis
/// abîmée avant le lancement : l'équipier part quand même (son modèle vient du manifeste
/// préparé, sa boîte se lit sans bus), lit, édite et cherche sur le worktree en le disant
/// en tête de chaque retour, rend « indisponible » pour un outil d'index, et finit son pas.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_teammate_works_on_the_files_when_its_backend_does_not_open() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let vus = Arc::new(Mutex::new(Vec::<String>::new()));
    let model = {
        let vus = vus.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let vus = vus.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if let Some(r) = results.last() {
                    vus.lock().unwrap().push(r.clone());
                }
                match results.len() {
                    0 => answer(turns, opts, sink, "", calls(&[("read_file", json!({ "path": "src/lib.rs" }))])),
                    1 => answer(turns, opts, sink, "", calls(&[("edit_file", json!({ "path": "README.md", "content": "# Écrit sans index\n" }))])),
                    2 => answer(turns, opts, sink, "", calls(&[("search_code", json!({ "query": "bonjour" }))])),
                    3 => answer(turns, opts, sink, "", calls(&[("usages", json!({ "name": "bonjour" }))])),
                    _ => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "lu, écrit, cherché sans index", "remaining": "" }))])),
                }
            })))
        }) as ChatModel
    };
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let t = host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    // La base de la tâche : fermée, puis remplacée par un fichier qui n'en est pas une.
    let (_, session) = host.handle("POST", "/session", None, br#"{"client":"le-temoin"}"#);
    let ferme = host.close(session["session"].as_str(), &t.manifest);
    assert_eq!(ferme["result"]["closed"], true, "{ferme}");
    let base = &t.copie.base;
    if base.is_dir() {
        std::fs::remove_dir_all(base).unwrap();
    }
    std::fs::write(base, vec![0xFFu8; 8192]).unwrap();
    let _ = std::fs::remove_file(base.with_extension("wal"));
    assert!(host.with_backend_id(&t.id, |_| ()).is_err(), "la base abîmée ne doit plus s'ouvrir");

    let files = host.teammate_files(&h.id, "doc").unwrap();
    files.append("supervisor", "request", None, "Lis src/lib.rs, écris un README, cherche « bonjour ».").unwrap();
    let outcome = host.run_teammate(&h.id, "doc").expect("l'équipier tourne sans son backend");
    assert_eq!(outcome.ended, "fait sans plan", "{outcome:?}");

    let vus = vus.lock().unwrap().clone();
    assert_eq!(vus.len(), 4, "{vus:#?}");
    for (i, quoi) in ["la lecture", "l'édition", "la recherche"].iter().enumerate() {
        assert!(vus[i].starts_with("index indisponible : repli sur les fichiers"), "{quoi} dit son repli : {}", vus[i]);
    }
    assert!(vus[0].contains("\"bonjour\""), "la lecture rend le fichier : {}", vus[0]);
    assert!(vus[2].contains("signal : grep_repli") && vus[2].contains("src/lib.rs"), "{}", vus[2]);
    assert!(vus[3].contains("\"unavailable\""), "un outil d'index dit qu'il est indisponible : {}", vus[3]);
    // Le worktree du code de la tâche (sa racine de workspace), là où le repli écrit.
    let racine = t.workspace.as_ref().map(|w| w.racine.clone()).expect("la tâche a un workspace");
    assert_eq!(std::fs::read_to_string(racine.join("README.md")).unwrap(), "# Écrit sans index\n", "l'édition est sur le disque du worktree");
    let boite = files.mailbox();
    assert!(boite.iter().any(|l| l["kind"] == "step_done" && l["succeeded"] == true), "{boite:?}");
    eprintln!("sans backend : {} replis, le pas fini", vus.len() - 1);
}

/// **Un pas qui n'aboutit pas ne termine pas l'équipier** : le modèle tombe en panne au
/// premier pas, la boîte le dit, le pas suivant repart de la frontière et finit ; un modèle
/// toujours en panne arrête l'équipier au bout de `teammate.max_failed_steps` (3) échecs
/// d'affilée, en le disant.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_failed_step_does_not_end_the_teammate_until_the_declared_count() {
    for toujours in [false, true] {
        let d = tempfile::tempdir().unwrap();
        let (_depot, manifeste) = depot_inscrit(d.path(), true);
        let appels = Arc::new(Mutex::new(0usize));
        let model = {
            let appels = appels.clone();
            Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
                let appels = appels.clone();
                Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                    let n = {
                        let mut a = appels.lock().unwrap();
                        *a += 1;
                        *a
                    };
                    if toujours || n == 1 {
                        return Err(crate::llm::LlmError::Model("le fournisseur ne répond pas".into()));
                    }
                    answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait après la panne", "remaining": "" }))]))
                })))
            }) as ChatModel
        };
        let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
        host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
        let files = host.teammate_files(&h.id, "doc").unwrap();
        files.append_with("supervisor", "request", None, "Une petite chose.", &json!({ "expects": "none" })).unwrap();
        // Une fin en erreur se rend comme telle (et s'écrit `ended` dans la boîte) :
        // l'équipier s'arrête en le disant, il ne plante pas.
        let ended = match host.run_teammate(&h.id, "doc") {
            Ok(o) => o.ended,
            Err(e) => e,
        };
        let rates = files.mailbox().iter().filter(|l| l["kind"] == "report" && l["text"].as_str().unwrap_or_default().contains("n'a pas abouti")).count();
        assert!(files.mailbox().iter().any(|l| l["kind"] == "ended" && l["text"] == ended.as_str()), "la fin est dans la boîte : {ended}");
        if toujours {
            assert!(ended.starts_with("erreur : ") && ended.contains("max_failed_steps = 3"), "{ended}");
            assert_eq!(rates, 3, "trois échecs dits");
        } else {
            assert_eq!(ended, "fait sans plan");
            assert_eq!(rates, 1, "l'échec du premier pas est dit");
        }
    }
}

/// **Une fermeture passe entre deux appels d'outil** : le pas ne tient plus le backend
/// pour tout son tour. Pendant le pas, le backend de la tâche est fermé par la route
/// `POST /backend/close` (sans attendre la fin du tour), et l'appel suivant le rouvre.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_backend_closes_between_two_tool_calls_of_a_step() {
    use std::time::{Duration, Instant};
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let id = crate::daemon_host::DaemonHost::new().hosted(&manifeste).unwrap().id.clone();
    let racine = Arc::new(Mutex::new(String::new()));
    let ferme = Arc::new(Mutex::new(Value::Null));
    let lus = Arc::new(Mutex::new(Vec::<String>::new()));
    let model = {
        let (racine, ferme, lus, id) = (racine.clone(), ferme.clone(), lus.clone(), id.clone());
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (racine, ferme, lus, id) = (racine.clone(), ferme.clone(), lus.clone(), id.clone());
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if let Some(r) = results.last() {
                    lus.lock().unwrap().push(r.clone());
                }
                match results.len() {
                    0 => answer(turns, opts, sink, "", calls(&[("read_file", json!({ "path": "src/lib.rs" }))])),
                    1 => {
                        // Le tour est en vol : le manifeste de la tâche se lit par la route
                        // de l'équipier, et son backend se ferme.
                        let r = racine.lock().unwrap().clone();
                        let lire = |route: String| -> Value {
                            let mut x = crate::daemon::agent_de_temoin().get(format!("{r}{route}")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
                            serde_json::from_str(&x.body_mut().read_to_string().unwrap()).unwrap()
                        };
                        let tache = lire(format!("/ide/teammate?backend={id}&task=doc"))["result"]["backend"].as_str().unwrap().to_string();
                        let mut s = crate::daemon::agent_de_temoin().post(format!("{r}/session")).header("Authorization", "Bearer j").header("content-type", "application/json").send(json!({ "client": "le témoin" }).to_string()).unwrap();
                        let session: Value = serde_json::from_str(&s.body_mut().read_to_string().unwrap()).unwrap();
                        let mut x = crate::daemon::agent_de_temoin()
                            .post(format!("{r}/backend/close"))
                            .header("Authorization", "Bearer j")
                            .header("X-Rag3weaver-Session", session["session"].as_str().unwrap_or_default())
                            .header("content-type", "application/json")
                            .send(json!({ "manifest": tache }).to_string())
                            .unwrap();
                        *ferme.lock().unwrap() = serde_json::from_str(&x.body_mut().read_to_string().unwrap()).unwrap();
                        answer(turns, opts, sink, "", calls(&[("read_file", json!({ "path": "README.md" }))]))
                    }
                    _ => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "deux lectures", "remaining": "" }))])),
                }
            })))
        }) as ChatModel
    };
    let (taches, chats) = (d.path().join("taches"), d.path().join("chats"));
    let (adresse, _fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| h.with_racine_des_taches(taches).with_chats_root(chats).with_chat_model(model));
    *racine.lock().unwrap() = format!("http://{adresse}");
    let r = racine.lock().unwrap().clone();
    let post = |route: &str, corps: Value| -> Value {
        let mut x = crate::daemon::agent_de_temoin().post(format!("{r}{route}")).header("Cookie", "rag3weaver_ide=j").header("content-type", "application/json").send(corps.to_string()).unwrap();
        serde_json::from_str(&x.body_mut().read_to_string().unwrap()).unwrap()
    };
    assert_eq!(post("/ide/describe", json!({ "manifest": manifeste }))["ok"], true);
    let v = post("/ide/teammate/mailbox", json!({ "backend": id, "task": "doc", "from": "supervisor", "kind": "request", "text": "Lis deux fichiers.", "expects": "none" }));
    assert_eq!(v["ok"], true, "{v}");
    let borne = Instant::now() + Duration::from_secs(60);
    loop {
        let mut x = crate::daemon::agent_de_temoin().get(format!("{r}/ide/teammate/list?supervisor=supervisor")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
        let l: Value = serde_json::from_str(&x.body_mut().read_to_string().unwrap()).unwrap();
        if l["result"]["teammates"][0]["state"] == "ended" {
            break;
        }
        assert!(Instant::now() < borne, "en 60 s, l'équipier n'a pas fini : {l}");
        std::thread::sleep(Duration::from_millis(100));
    }
    let ferme = ferme.lock().unwrap().clone();
    assert_eq!(ferme["result"]["closed"], true, "la fermeture est passée pendant le tour : {ferme}");
    let lus = lus.lock().unwrap().clone();
    assert!(lus.len() >= 2 && lus[0].contains("pub fn bonjour") && lus[1].contains("Petit dépôt"), "les deux lectures, par le backend (rouvert pour la seconde) : {lus:#?}");
    assert!(!lus.iter().any(|l| l.starts_with("index indisponible")), "un backend qui se rouvre n'a pas de repli : {lus:#?}");
}

/// **L'équipier reprend après un arrêt de l'hôte** (page
/// 07-novembre-2026-everything-declarative/01, § 1). Un premier hôte ouvre la tâche ; sa boîte
/// a une demande et une question de l'équipier, sans fin — l'hôte s'est arrêté pendant le
/// travail. Un second hôte, servi sur les mêmes dossiers, relit les tâches : la boîte dit la
/// reprise, l'équipier repart à sa frontière et finit. Une tâche dont la demande a déjà sa fin
/// ne repart pas.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_teammate_resumes_after_the_host_restarts() {
    use std::time::{Duration, Instant};
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let pas = Arc::new(Mutex::new(0usize));
    let model = {
        let pas = pas.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let pas = pas.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                *pas.lock().unwrap() += 1;
                answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "repris et fini", "remaining": "" }))]))
            })))
        }) as ChatModel
    };
    // Le premier hôte : deux tâches, l'une interrompue, l'autre déjà finie.
    let (taches, chats) = (d.path().join("taches"), d.path().join("chats"));
    let id = {
        let host = hote(d.path(), model.clone());
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
        for nom in ["coupee", "finie"] {
            host.ouvrir_une_tache(&h.id, nom).expect("la tâche s'ouvre");
            let files = host.teammate_files(&h.id, nom).unwrap();
            files.append_with("supervisor", "request", None, "Écris une ligne dans le README.", &json!({ "expects": "none" })).unwrap();
            files.append("teammate", "step_done", None, "fait : la moitié").unwrap();
            if nom == "finie" {
                files.append("host", "ended", None, "fait sans plan").unwrap();
            }
        }
        h.id.clone()
    };
    // Le second hôte, sur les mêmes dossiers : il relit les tâches et sert.
    let (adresse, _fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
        h.with_racine_des_taches(taches).with_chats_root(chats).with_chat_model(model).avec_les_taches_relues()
    });
    let lire = |task: &str| -> Value {
        let mut r = crate::daemon::agent_de_temoin().get(format!("http://{adresse}/ide/teammate?backend={id}&task={task}")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    let borne = Instant::now() + Duration::from_secs(60);
    let boite = loop {
        let b = lire("coupee")["result"]["mailbox"].as_array().cloned().unwrap_or_default();
        if b.iter().any(|l| l["kind"] == "ended") {
            break b;
        }
        assert!(Instant::now() < borne, "en 60 s, l'équipier interrompu n'a pas repris et fini : {b:#?}");
        std::thread::sleep(Duration::from_millis(100));
    };
    let reprise = boite.iter().position(|l| l["from"] == "host" && l["text"].as_str().unwrap_or_default().contains("reprise à la frontière")).expect("la reprise dite");
    let fin = boite.iter().position(|l| l["kind"] == "ended").unwrap();
    assert!(reprise < fin, "la reprise avant la fin : {boite:#?}");
    assert!(boite.iter().any(|l| l["kind"] == "step_done" && l["done"] == "repris et fini"), "{boite:#?}");
    // La tâche finie ne repart pas.
    let finie = lire("finie")["result"]["mailbox"].as_array().cloned().unwrap_or_default();
    assert!(!finie.iter().any(|l| l["text"].as_str().unwrap_or_default().contains("reprise")), "une tâche finie ne reprend pas : {finie:#?}");
    assert_eq!(*pas.lock().unwrap(), 1, "un seul pas, celui de la reprise");
}

/// **L'équipier ne finit jamais en silence** (premier essai réel de c7, 7 novembre) : la
/// `request` qui ouvre la tâche dit d'où elle part (branche, commit) dans la boîte et dans sa
/// réponse ; un pas qui se termine sans `step_done` écrit sa réponse finale en `report`.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_step_without_step_done_reports_and_the_request_says_where_the_task_starts() {
    use std::time::{Duration, Instant};
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            answer(turns, opts, sink, "Je n'ai pas d'outil d'écriture : je m'arrête là.", Vec::new())
        })))
    });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let r = host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Corrige le ticket.", &Value::Null).expect("la request");
    let depart = r["task"]["depart"].as_str().expect("la réponse dit le commit de départ").to_string();
    assert_eq!(depart.len(), 40, "{r}");
    assert_eq!(r["task"]["branche"], "agent/doc", "{r}");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let borne = Instant::now() + Duration::from_secs(60);
    let boite = loop {
        let b = files.mailbox();
        if b.iter().any(|l| l["kind"] == "report" && l["from"] == "teammate") {
            break b;
        }
        assert!(Instant::now() < borne, "en 60 s, aucun report de l'équipier : {b:#?}");
        std::thread::sleep(Duration::from_millis(50));
    };
    let ouverte = boite.iter().find(|l| l["from"] == "host" && l["text"].as_str().unwrap_or_default().starts_with("tâche ouverte")).expect("la boîte dit d'où part la tâche");
    assert!(ouverte["text"].as_str().unwrap().contains(&depart[..9]), "{ouverte}");
    let rapport = boite.iter().find(|l| l["kind"] == "report" && l["from"] == "teammate").unwrap();
    // La réponse seule, en copie de l'hôte (16 novembre 2026 : plus de préfixe « le pas N s'est
    // terminé sans step_done », que tty devait reconnaître pour ne pas l'afficher).
    assert_eq!((&rapport["text"], &rapport["origin"]), (&json!("Je n'ai pas d'outil d'écriture : je m'arrête là."), &json!("host_copy")), "{rapport}");
    // Le superviseur le voit.
    assert!(files.pending_for("c7").iter().any(|l| l["id"] == rapport["id"]), "le report attend c7");
}

/// **Les commandes par la porte déclarée** (page 07-novembre-2026-everything-declarative/01,
/// lot 2) : dans une tâche d'un arbre inscrit (`commands: approval`), l'équipier a
/// `run_command` ; une commande de la liste sûre passe (`ls`), une autre est mise en attente et
/// sa question part dans la boîte (`approval`, la commande) ; le « oui » du superviseur l'accorde,
/// et au pas suivant la même commande passe, dans le worktree de la tâche.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_teammate_runs_commands_through_the_declared_gate_and_asks_for_the_rest() {
    use std::time::{Duration, Instant};
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let vus = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let vus = vus.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let vus = vus.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                // **A loaded machine, made deterministic** (ticket of 8 November 2026): the second tool result
                // reaches the model late. The supervisor's mailbox may already hold the question by then.
                if results.len() == 2 {
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
                if let Some(r) = results.last() {
                    vus.lock().unwrap().push(r.clone());
                }
                let accorde = last_user(turns).contains("commande accordée") || turns.iter().any(|t| t.content.contains("commande accordée"));
                let run = |ligne: &str| ("run_command", json!({ "command": ligne }));
                match (accorde, results.len()) {
                    (false, 0) => answer(turns, opts, sink, "", calls(&[run("ls")])),
                    (false, 1) => answer(turns, opts, sink, "", calls(&[run("echo accorde-moi")])),
                    (false, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": false, "done": "ls joué", "remaining": "echo attend l'accord" }))])),
                    (true, 0) => answer(turns, opts, sink, "", calls(&[run("echo accorde-moi")])),
                    (true, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "echo joué après l'accord", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Liste le dépôt, puis dis bonjour par echo.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let attendre = |quoi: &str, ok: &dyn Fn(&[Value]) -> bool| -> Vec<Value> {
        let borne = Instant::now() + Duration::from_secs(90);
        loop {
            let b = files.mailbox();
            if ok(&b) {
                return b;
            }
            assert!(Instant::now() < borne, "en 90 s : {quoi} — {b:#?}");
            std::thread::sleep(Duration::from_millis(50));
        }
    };
    // Le pas fini sur la question : l'équipier est suspendu, la boîte le dit (le pas a
    // rendu tous ses résultats au modèle avant cette ligne).
    let boite = attendre("la question de l'accord et l'attente", &|b| {
        b.iter().any(|l| l["kind"] == "approval") && b.iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.starts_with("en attente du superviseur depuis")))
    });
    let question = boite.iter().find(|l| l["kind"] == "approval").unwrap().clone();
    assert_eq!(question["command"], "echo accorde-moi", "{question}");
    assert!(files.pending_for("c7").iter().any(|l| l["id"] == question["id"]), "la question attend le superviseur");
    {
        let vus = vus.lock().unwrap();
        assert!(!vus[0].contains("en attente") && !vus[0].contains("refusée"), "ls passe par la liste sûre : {}", vus[0]);
        assert!(vus[1].contains(crate::dataflow::run_nodes::EN_ATTENTE) && vus[1].contains("dans la boîte du superviseur"), "{}", vus[1]);
    }
    host.post_to_teammate(&h.id, "doc", "c7", "answer", None, "oui", &json!({ "reply_to": question["id"], "approved": true })).expect("le oui");
    let boite = attendre("la commande jouée après l'accord", &|b| b.iter().any(|l| l["kind"] == "step_done" && l["succeeded"] == true));
    assert!(boite.iter().any(|l| l["from"] == "host" && l["text"].as_str().unwrap_or_default().starts_with("commande accordée pour la tâche : echo accorde-moi")), "{boite:#?}");
    let vus = vus.lock().unwrap();
    assert!(vus.iter().any(|r| r.contains("accorde-moi") && !r.contains("en attente")), "echo a tourné après l'accord : {vus:#?}");
}

/// **Une ligne postée pendant le dernier pas ne se perd pas** (8 novembre 2026, la passe du
/// lot des commandes) : le oui du superviseur arrivait entre le pas et la fin du lancement,
/// la boucle finissait sans le lire, et la poste ne relançait rien (l'équipier était encore
/// en route). Ici, pendant son unique pas, le superviseur écrit une consigne ; le pas se
/// déclare fini ; l'équipier doit faire un second pas qui la lit.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_line_posted_during_the_last_step_is_read_before_the_teammate_ends() {
    use std::time::{Duration, Instant};
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let boite: Arc<Mutex<Option<super::teammate::TeammateFiles>>> = Arc::new(Mutex::new(None));
    let model: ChatModel = {
        let boite = boite.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let boite = boite.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let fini = |done: &str| calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": done, "remaining": "" }))]);
                if last_user(turns).contains("consigne tardive") {
                    return answer(turns, opts, sink, "", fini("consigne lue"));
                }
                if let Some(f) = boite.lock().unwrap().as_ref() {
                    f.append("c7", "instruction", None, "consigne tardive : relis le README").unwrap();
                }
                answer(turns, opts, sink, "", fini("premier pas"))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    // La boîte d'abord (le faux modèle y écrit), puis la request qui lance l'équipier.
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    *boite.lock().unwrap() = Some(host.teammate_files(&h.id, "doc").unwrap());
    host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Lis le dépôt.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let borne = Instant::now() + Duration::from_secs(90);
    loop {
        let b = files.mailbox();
        if b.iter().any(|l| l["kind"] == "step_done" && l["done"] == "consigne lue") {
            break;
        }
        assert!(Instant::now() < borne, "en 90 s, la consigne écrite pendant le pas n'a pas été lue : {b:#?}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// **Attendre le superviseur ne tue pas la tâche** (8 novembre 2026, essai 4 de c7, la
/// tâche racine-disparue) : l'équipier posait une question, puis marquait chaque pas
/// d'attente « raté : en attente de la sortie de cargo test par le superviseur » ; au
/// troisième, l'hôte arrêtait la tâche, et chaque pas d'attente rappelait le modèle. Une
/// question sans réponse suspend l'équipier : aucun appel au modèle, la boîte le dit une
/// fois (« en attente du superviseur depuis HH:MM »), le pas ne compte ni réussi ni raté ;
/// la réponse le réveille et le pas suivant la lit.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn waiting_for_the_supervisor_suspends_the_teammate_without_failing_the_task() {
    use std::time::{Duration, Instant};
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let pas = Arc::new(Mutex::new(0usize));
    let model: ChatModel = {
        let pas = pas.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let pas = pas.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if results.is_empty() {
                    *pas.lock().unwrap() += 1;
                }
                let repondu = last_user(turns).contains("la sortie : 3 passed");
                match (step_of(turns), results.len(), repondu) {
                    (1, 0, _) => answer(turns, opts, sink, "", calls(&[("edit_plan", json!({ "plan": {
                        "request": "Le témoin de la racine disparue", "proposal": "Écrire le témoin, le faire jouer, corriger",
                        "items": [
                            { "id": "jouer", "text": "Faire jouer le témoin par le superviseur", "status": "blocked", "alone": false, "question": "Peux-tu jouer cargo test sur le témoin et me rendre la sortie ?" },
                            { "id": "corriger", "text": "Corriger le code", "status": "todo", "alone": true }
                        ] } }))])),
                    (_, 0, true) => answer(turns, opts, sink, "", calls(&[("edit_plan", json!({ "close": { "outcome": "accomplished", "reason": "le témoin passe", "concepts": ["témoin"] } }))])),
                    (_, _, true) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "corriger", "succeeded": true, "done": "corrigé après la sortie", "remaining": "" }))])),
                    _ => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "jouer", "succeeded": false, "done": "raté : en attente de la sortie de cargo test par le superviseur", "remaining": "la sortie" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Écris le témoin de la racine disparue.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let attendre = |quoi: &str, ok: &dyn Fn(&[Value]) -> bool| -> Vec<Value> {
        let borne = Instant::now() + Duration::from_secs(90);
        loop {
            let b = files.mailbox();
            if ok(&b) {
                return b;
            }
            assert!(Instant::now() < borne, "en 90 s : {quoi} — {b:#?}");
            std::thread::sleep(Duration::from_millis(50));
        }
    };
    let boite = attendre("l'attente dite dans la boîte", &|b| b.iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.starts_with("en attente du superviseur depuis"))));
    let question = boite.iter().find(|l| l["kind"] == "question" && l["from"] == "teammate").expect("la question").clone();
    // Trois secondes d'attente : ni nouveau pas, ni arrêt.
    std::thread::sleep(Duration::from_secs(3));
    let boite = files.mailbox();
    assert_eq!(*pas.lock().unwrap(), 1, "aucun appel au modèle pendant l'attente : {boite:#?}");
    // L'équipier suspendu retient l'arrêt d'inactivité de l'hôte (l'hôte de Lucie, 4 h 46).
    let retenu = host.travail_en_cours();
    assert!(retenu.as_deref().is_some_and(|c| c.contains("équipier(s) en route")), "{retenu:?}");
    assert!(!boite.iter().any(|l| l["text"].as_str().unwrap_or_default().contains("pas de suite sans élément réussi")), "attendre n'arrête pas la tâche : {boite:#?}");
    assert!(!boite.iter().any(|l| l["kind"] == "ended"), "{boite:#?}");
    assert_eq!(boite.iter().filter(|l| l["from"] == "host" && l["text"].as_str().unwrap_or_default().starts_with("en attente du superviseur")).count(), 1, "l'attente se dit une fois : {boite:#?}");
    // La réponse réveille l'équipier ; le pas suivant la lit et finit.
    host.post_to_teammate(&h.id, "doc", "c7", "answer", None, "la sortie : 3 passed", &json!({ "reply_to": question["id"] })).expect("la réponse");
    let boite = attendre("le pas qui lit la réponse", &|b| b.iter().any(|l| l["kind"] == "step_done" && l["succeeded"] == true));
    assert!(boite.iter().any(|l| l["kind"] == "ended" || l["kind"] == "step_done"), "{boite:#?}");
    assert_eq!(*pas.lock().unwrap(), 2, "un seul pas après la réponse : {boite:#?}");
}

/// **Une tâche joue cargo test par la porte** (8 novembre 2026, essai 5 de c7 : la porte
/// marchait, mais cargo ne lisait pas ~/.rustup dans le bac à sable). Le petit dépôt devient
/// un crate ; l'équipier joue `cargo test --lib -- un_test` par `run_command` (la porte le
/// laisse passer sans accord : un test ne modifie que le target) et lit « test result: ok. 1
/// passed » — la chaîne Rust en lecture, le target partagé du dépôt (`{repo_target}`) et le
/// TMPDIR de la tâche (`{task_dir}/tmp`), déclarés par le gabarit code.
#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_task_plays_cargo_test_through_the_gate() {
    use std::time::{Duration, Instant};
    if !std::env::var_os("HOME").map(std::path::PathBuf::from).is_some_and(|h| h.join(".rustup").is_dir()) {
        eprintln!("pas de ~/.rustup sur ce poste : test sauté, dit franchement");
        return;
    }
    if crate::commande::BacASable::autour("/").ruleset().is_err() {
        eprintln!("Landlock indisponible sur ce noyau : test sauté, dit franchement");
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    std::fs::write(depot.join("Cargo.toml"), "[package]\nname = \"petit-depot\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    std::fs::write(depot.join("src/lib.rs"), "pub fn un() -> u32 { 1 }\n#[cfg(test)]\nmod tests { #[test] fn un_test() { assert_eq!(super::un(), 1); } }\n").unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "un crate"]] {
        let ok = std::process::Command::new("git").args(args).current_dir(&depot).status().unwrap().success();
        assert!(ok, "git {args:?}");
    }
    let vus = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let vus = vus.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let vus = vus.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if let Some(r) = results.last() {
                    vus.lock().unwrap().push(r.clone());
                }
                let cargo = ("run_command", json!({ "command": "cargo test --lib -- un_test" }));
                match results.len() {
                    0 => answer(turns, opts, sink, "", calls(&[cargo])),
                    _ => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "cargo test joué", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Joue le test un_test.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let attendre = |quoi: &str, ok: &dyn Fn(&[Value]) -> bool| -> Vec<Value> {
        let borne = Instant::now() + Duration::from_secs(300);
        loop {
            let b = files.mailbox();
            if ok(&b) {
                return b;
            }
            assert!(Instant::now() < borne, "en 300 s : {quoi} — {b:#?} — vu par le modèle : {:#?}", vus.lock().unwrap());
            std::thread::sleep(Duration::from_millis(100));
        }
    };
    let boite = attendre("cargo joué dans la tâche", &|b| b.iter().any(|l| l["kind"] == "step_done" && l["succeeded"] == true));
    assert!(!boite.iter().any(|l| l["kind"] == "approval"), "cargo test passe la porte sans accord : {boite:#?}");
    let vus = vus.lock().unwrap();
    assert!(vus.iter().any(|r| r.contains("test result: ok. 1 passed")), "cargo test a tourné dans la tâche : {vus:#?}");
    // Le target partagé du dépôt a reçu le bâti ; on le rend.
    let cible = crate::commande::cible_du_depot("petit-depot");
    assert!(cible.join("debug").is_dir(), "le bâti dans {}", cible.display());
    let _ = std::fs::remove_dir_all(&cible);
}

/// **Un pas repart d'une session neuve** (8 novembre 2026, le quota de l'essai 3 : chaque
/// pas renvoyait tout le fil, de 5 275 jetons d'entrée au pas 1 à 122 242 au pas 10). Le pas
/// 1 lit trois fois le dépôt puis déclare son pas ; le premier appel du pas 2 ne reçoit que
/// le système et son message, qui porte la demande en cours et ce que le pas 1 a déclaré.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn each_step_starts_from_a_fresh_thread_with_the_request_and_the_steps_done() {
    use std::time::{Duration, Instant};
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let premiers = Arc::new(Mutex::new(Vec::<(usize, usize, String)>::new()));
    let model: ChatModel = {
        let premiers = premiers.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let premiers = premiers.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                let pas = step_of(turns);
                if results.is_empty() {
                    premiers.lock().unwrap().push((pas, turns.len(), last_user(turns)));
                }
                let lire = ("read_file", json!({ "path": "src/lib.rs" }));
                match (pas, results.len()) {
                    (1, 0) => answer(turns, opts, sink, "", calls(&[("edit_plan", json!({ "plan": {
                        "request": "Relire puis conclure", "proposal": "Deux éléments",
                        "items": [{ "id": "lire", "text": "Lire le dépôt", "status": "todo", "alone": true }, { "id": "conclure", "text": "Conclure", "status": "todo", "alone": true }] } }))])),
                    (1, 1..=3) => answer(turns, opts, sink, "", calls(&[lire])),
                    (1, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "lire", "succeeded": true, "done": "trois lectures de src/lib.rs", "remaining": "conclure" }))])),
                    (_, 0) => answer(turns, opts, sink, "", calls(&[("edit_plan", json!({ "close": { "outcome": "accomplished", "reason": "lu et conclu", "concepts": ["lecture"] } }))])),
                    _ => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "conclure", "succeeded": true, "done": "conclu", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Relis le dépôt et conclus.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let borne = Instant::now() + Duration::from_secs(90);
    while !files.mailbox().iter().any(|l| l["kind"] == "ended") {
        assert!(Instant::now() < borne, "en 90 s, pas de fin : {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(50));
    }
    let premiers = premiers.lock().unwrap();
    let (_, n, message) = premiers.iter().find(|(p, _, _)| *p == 2).cloned().unwrap_or_else(|| panic!("aucun pas 2 : {premiers:#?}"));
    assert_eq!(n, 2, "le pas 2 part du système et de son message seuls : {premiers:#?}");
    assert!(message.contains("La demande en cours : Relis le dépôt et conclus."), "{message}");
    assert!(message.contains("trois lectures de src/lib.rs") && message.contains("reste : conclure"), "le pas 1 rappelé : {message}");
}

/// **Un 429 n'est pas un pas raté** (8 novembre 2026, essai 3 : « le pas 6 n'a pas abouti :
/// quota dépassé après 2 reprises en 186 s », et trois refus d'affilée arrêtaient l'équipier
/// par `max_failed_steps`). Le fournisseur refuse trois fois de suite (autant que
/// `max_failed_steps`) : l'équipier attend une fenêtre à chaque refus, le dit dans la boîte,
/// rejoue le même pas, et finit quand le fournisseur revient.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_provider_refusal_waits_a_window_and_does_not_fail_the_step() {
    use std::time::{Duration, Instant};
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    // La fenêtre du témoin : 1 s (le défaut est 60), dans les déclarations commitées.
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["teammate"] = json!({ "quota_wait_s": 1 });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "une fenêtre courte"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let appels = Arc::new(Mutex::new(0usize));
    let model: ChatModel = {
        let appels = appels.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let appels = appels.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let n = {
                    let mut a = appels.lock().unwrap();
                    *a += 1;
                    *a
                };
                if n <= 3 {
                    return Err(crate::llm::LlmError::Unavailable { status: Some(429), quota: crate::llm::Quota::Throttled, retries: 2, waited_ms: 10, message: "RESOURCE_EXHAUSTED".into() });
                }
                answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait après les refus", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    // The bus of the task, before the request: each backoff says `Waiting` (cause model), then `Resumed`.
    let tache = host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let t = host.hote_par_id(&tache.id).unwrap();
    let mut agent = host.with_backend(&t, |b| b.bus_de_publication().subscribe(crate::events::topic::AGENT)).unwrap();
    host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Fais le travail.", &json!({ "expects": "none" })).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let borne = Instant::now() + Duration::from_secs(90);
    let boite = loop {
        let b = files.mailbox();
        if b.iter().any(|l| l["kind"] == "ended") {
            break b;
        }
        assert!(Instant::now() < borne, "en 90 s, pas de fin : {b:#?}");
        std::thread::sleep(Duration::from_millis(100));
    };
    let fin = boite.iter().find(|l| l["kind"] == "ended").unwrap();
    assert!(!fin["text"].as_str().unwrap_or_default().starts_with("erreur"), "trois 429 n'arrêtent pas l'équipier : {boite:#?}");
    assert!(boite.iter().any(|l| l["kind"] == "step_done" && l["succeeded"] == true), "{boite:#?}");
    let attentes: Vec<&str> = boite.iter().filter_map(|l| l["text"].as_str()).filter(|t| t.contains("attend le fournisseur")).collect();
    assert_eq!(attentes.len(), 3, "chaque refus dit son attente : {boite:#?}");
    assert!(attentes[0].starts_with("le pas 1 attend le fournisseur (1 refus") && attentes[2].contains("reprise dans 4 s"), "{attentes:#?}");
    assert!(!boite.iter().any(|l| l["text"].as_str().unwrap_or_default().contains("n'a pas abouti")), "un refus n'est pas un pas raté : {boite:#?}");
    let mut evenements = Vec::new();
    while let Ok(e) = agent.try_recv() {
        evenements.push(e.to_json());
    }
    let suite: Vec<String> = evenements
        .iter()
        .filter(|e| e["kind"] == "Waiting" || e["kind"] == "Resumed")
        .map(|e| format!("{} {} {}", e["kind"].as_str().unwrap_or_default(), e["cause"].as_str().unwrap_or_default(), e["what"].as_str().unwrap_or_default()))
        .collect();
    assert_eq!(suite, ["Waiting model faux-equipier", "Resumed model ", "Waiting model faux-equipier", "Resumed model ", "Waiting model faux-equipier", "Resumed model "], "each backoff says it waits for the model, then resumes : {evenements:#?}");
    let reprises: Vec<u64> = evenements.iter().filter(|e| e["kind"] == "Resumed").filter_map(|e| e["waited_ms"].as_u64()).collect();
    assert!(reprises.len() == 3 && reprises[2] >= 4000, "the whole backoff (1, 2, then 4 s) : {reprises:?}");
    assert!(evenements.iter().filter(|e| e["kind"] == "Waiting").all(|e| e["run"].as_str().is_some_and(|r| r.starts_with("agent-"))), "on the run of the refused step : {evenements:#?}");
    let fenetres: Vec<&Value> = evenements.iter().filter(|e| e["kind"] == "Waiting").map(|e| &e["wait_ms"]).collect();
    assert_eq!(fenetres, [&json!(1000), &json!(2000), &json!(4000)], "the window announced, for « reprise dans N s » (archi) : {evenements:#?}");
}

/// Attend dans la boîte une ligne qui satisfait `ok`, 90 s au plus.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
fn attendre_dans(files: &super::teammate::TeammateFiles, quoi: &str, ok: &dyn Fn(&[Value]) -> bool) -> Vec<Value> {
    use std::time::{Duration, Instant};
    let borne = Instant::now() + Duration::from_secs(90);
    loop {
        let b = files.mailbox();
        if ok(&b) {
            return b;
        }
        assert!(Instant::now() < borne, "en 90 s : {quoi} — {b:#?}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(all(feature = "code", feature = "rag3db-native"))]
fn dit_par_l_hote(b: &[Value], debut: &str) -> bool {
    b.iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.starts_with(debut)))
}

/// **Un `step_done` qui dit ce qui reste n'est pas une fin** (ticket du 9 novembre 2026 :
/// `readme-nouveautes-novembre` s'est close « fait sans plan » sur un premier pas dont
/// `remaining` disait « Validation du verdict et soumission des ajouts un par un »). Sans
/// plan, le pas suivant part, la boîte le dit ; il voit ce qui restait ; sans `remaining`,
/// la fin reste « fait sans plan ».
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_step_done_that_says_what_remains_is_not_the_end() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let frontieres = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let frontieres = frontieres.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let frontieres = frontieres.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                if tool_results(turns).is_empty() {
                    frontieres.lock().unwrap().push(last_user(turns));
                }
                let remaining = if step_of(turns) == 1 { "Soumettre les ajouts un par un" } else { "" };
                answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "verdicts établis", "remaining": remaining }))]))
            })))
        })
    };
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    files.append_with("supervisor", "request", None, "Établis les verdicts, puis soumets chaque ajout.", &json!({ "expects": "none" })).unwrap();
    let o = host.run_teammate(&h.id, "doc").unwrap();
    let boite = files.mailbox();
    assert_eq!((o.steps.len(), o.ended.as_str()), (2, "fait sans plan"), "{o:?} — {boite:#?}");
    assert!(dit_par_l_hote(&boite, "pas fini : reste « Soumettre les ajouts un par un »"), "la boîte dit pourquoi le pas suivant part : {boite:#?}");
    let f = frontieres.lock().unwrap();
    assert!(f[1].contains("Soumettre les ajouts un par un"), "le pas 2 voit ce qui restait : {}", f[1]);
}

/// La présentation que demande le ticket de tty (14 novembre 2026), telle qu'à lire.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
const PRESENTATION: &str = "# L'équipier\n\n- je lis le code\n- j'écris des correctifs\n- je joue les tests\n\nUn mot en **gras**.";

/// Un équipier qui fait ce que `pas` dit à son premier pas, puis s'arrête ; il note ce qu'il voit
/// (la frontière, le message système, la description de `report`).
#[cfg(all(feature = "code", feature = "rag3db-native"))]
fn un_pas(pas: fn(&[Turn], &crate::llm::GenOptions, &mut dyn crate::llm::TokenSink) -> Result<(crate::llm::Finish, crate::llm::Usage), crate::llm::LlmError>, vu: Arc<Mutex<Vec<String>>>) -> ChatModel {
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        let vu = vu.clone();
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            if tool_results(turns).is_empty() {
                let systeme = turns.iter().filter(|t| t.role == "system").map(|t| t.content.clone()).collect::<Vec<_>>().join("\n");
                let report = opts.tools.iter().find(|d| d.name == "report").map(|d| d.description.clone()).unwrap_or_default();
                vu.lock().unwrap().push(format!("{systeme}\n--- report : {report}\n--- frontière : {}", last_user(turns)));
                pas(turns, opts, sink)
            } else {
                answer(turns, opts, sink, "", Vec::new())
            }
        })))
    })
}

/// **Le contenu demandé arrive dans la boîte** (ticket de tty, 14 novembre 2026 : à « présente-toi :
/// un titre, une liste, un mot en gras », l'équipier a rendu « Présentation fournie au superviseur
/// conformément au format demandé », et le contenu n'était nulle part). Un équipier qui écrit le
/// contenu dans sa réponse puis appelle `step_done` : la boîte le porte. Et ce que le modèle reçoit
/// dit où va un texte demandé : `report` porte le texte lui-même, jamais une phrase sur lui.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_text_a_teammate_writes_in_its_step_reaches_the_mailbox() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let vu = Arc::new(Mutex::new(Vec::<String>::new()));
    let model = un_pas(
        |turns, opts, sink| answer(turns, opts, sink, PRESENTATION, calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "présentation écrite", "remaining": "" }))])),
        vu.clone(),
    );
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    files.append("supervisor", "request", None, "Présente-toi : un titre, une liste à puces de trois choses que tu sais faire, et un mot en gras.").unwrap();
    let o = host.run_teammate(&h.id, "doc").unwrap();
    let boite = files.mailbox();
    let portee: Vec<&Value> = boite.iter().filter(|l| l["from"] == "teammate" && l["text"].as_str().is_some_and(|t| t.contains("# L'équipier") && t.contains("**gras**"))).collect();
    assert_eq!(portee.len(), 1, "le contenu écrit dans le pas arrive une fois dans la boîte ({o:?}) : {boite:#?}");
    let vu = vu.lock().unwrap();
    assert!(vu[0].contains("le texte lui-même"), "le message système et l'outil report disent où va un texte demandé : {}", vu[0]);
}

/// **Écrit et rapporté, le contenu n'est qu'une fois dans la boîte** (la garde de c7 sur le
/// correctif) : le modèle écrit la présentation dans sa réponse et la passe aussi à `report` ;
/// le `report` l'emporte, le texte du pas ne la double pas.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_text_both_written_and_reported_is_in_the_mailbox_once() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model = un_pas(
        |turns, opts, sink| {
            answer(turns, opts, sink, PRESENTATION, calls(&[("report", json!({ "text": PRESENTATION })), ("step_done", json!({ "item": "", "succeeded": true, "done": "présentation écrite", "remaining": "" }))]))
        },
        Arc::default(),
    );
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    files.append("supervisor", "request", None, "Présente-toi : un titre, une liste à puces de trois choses que tu sais faire, et un mot en gras.").unwrap();
    host.run_teammate(&h.id, "doc").unwrap();
    let boite = files.mailbox();
    let portee = boite.iter().filter(|l| l["from"] == "teammate" && l["text"].as_str().is_some_and(|t| t.contains("# L'équipier"))).count();
    assert_eq!(portee, 1, "le contenu une seule fois, celui du report : {boite:#?}");
}

/// **A report about the text is not the text** (ticket « L'équipier rend un compte rendu au lieu du
/// contenu demandé », reopened by archi on 15 November 2026 on tty's finding): to a request for a
/// text, with no `expects`, the model writes no text, calls `report` with a sentence *about* what it
/// would have written, then `step_done` succeeded — the séance of 14 November on Gemini 3.8 Flash.
/// Today the task ends « fait sans plan » and the content is nowhere. The witness does not choose the
/// guard (archi designs it): it only refuses that silent end. Either the content reaches the mailbox
/// (the fake model writes it from its second step on, if a guard relaunches it), or the host says in
/// the mailbox that the requested content is missing, and the task has not ended.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_report_about_the_text_is_not_the_text() {
    use std::time::{Duration, Instant};
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model = un_pas(
        |turns, opts, sink| {
            if est_le_juge(turns) {
                answer(turns, opts, sink, "missing", Vec::new())
            } else if step_of(turns) <= 1 {
                answer(turns, opts, sink, "", calls(&[
                    ("report", json!({ "text": "Présentation fournie au superviseur conformément au format demandé (titre, 3 puces, mot en gras)." })),
                    ("step_done", json!({ "item": "", "succeeded": true, "done": "Présentation rédigée avec titre, trois points de compétence et un mot en gras.", "remaining": "" })),
                ]))
            } else {
                answer(turns, opts, sink, PRESENTATION, calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "présentation écrite", "remaining": "" }))]))
            }
        },
        Arc::default(),
    );
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Présente-toi en quelques lignes : un titre, une liste à puces de trois choses que tu sais faire, et un mot en gras.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let contenu = |b: &[Value]| b.iter().any(|l| l["from"] == "teammate" && l["text"].as_str().is_some_and(|t| t.contains("# L'équipier") && t.contains("**gras**")));
    // The end or the content, whichever comes first; a guard that waits for the supervisor gives neither.
    let borne = Instant::now() + Duration::from_secs(20);
    let boite = loop {
        let b = files.mailbox();
        if contenu(&b) || b.iter().any(|l| l["kind"] == "ended") || Instant::now() >= borne {
            break b;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    if contenu(&boite) {
        return;
    }
    let fin = boite.iter().find(|l| l["kind"] == "ended");
    assert!(fin.is_none(), "the task ended ({}) with only a report about the text, and the content is nowhere : {boite:#?}", fin.map(|f| f["text"].to_string()).unwrap_or_default());
    let apres_le_pas = boite.iter().skip_while(|l| l["kind"] != "step_done").any(|l| l["from"] == "host" && l["kind"] != "ended");
    assert!(apres_le_pas, "the task waits without saying that the requested content is missing : {boite:#?}");
}

/// The closed question the host asks the teammate's model at the end of a step (archi, 19 November 2026).
#[cfg(all(feature = "code", feature = "rag3db-native"))]
fn est_le_juge(turns: &[Turn]) -> bool {
    last_user(turns).contains("delivered, missing or action")
}

/// The step of the séance of 14 November: no text, a report about the work, `step_done` succeeded.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
fn compte_rendu(turns: &[Turn], opts: &crate::llm::GenOptions, sink: &mut dyn crate::llm::TokenSink) -> Result<(crate::llm::Finish, crate::llm::Usage), crate::llm::LlmError> {
    answer(turns, opts, sink, "", calls(&[
        ("report", json!({ "text": "Fichiers rangés dans le dossier conformément à la demande." })),
        ("step_done", json!({ "item": "", "succeeded": true, "done": "fichiers rangés", "remaining": "" })),
    ]))
}

/// One request, one run of the teammate, with a model that does what `pas` says; the mailbox and what
/// the model saw.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
fn une_demande(pas: fn(&[Turn], &crate::llm::GenOptions, &mut dyn crate::llm::TokenSink) -> Result<(crate::llm::Finish, crate::llm::Usage), crate::llm::LlmError>, demande: &str) -> (Vec<Value>, Vec<String>, String) {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    // A red that waits for the supervisor waits 3 s, not 1 800 (it held the laptop's heavy slot, 19 Nov.).
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["teammate"]["reply_wait_s"] = json!(3);
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "une attente courte"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let vu = Arc::new(Mutex::new(Vec::<String>::new()));
    let host = hote(d.path(), un_pas(pas, vu.clone())).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    files.append("supervisor", "request", None, demande).unwrap();
    let o = host.run_teammate(&h.id, "doc").unwrap();
    let vu = vu.lock().unwrap().clone();
    (files.mailbox(), vu, o.ended)
}

/// **The judge says « action »: the task ends as today** (archi's second witness): a request for an
/// action, done, with a short report about it; the host asks once, and the sane case does not change.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_report_judged_an_action_lets_the_task_end() {
    let (boite, vu, fin) = une_demande(
        |turns, opts, sink| if est_le_juge(turns) { answer(turns, opts, sink, "action", Vec::new()) } else { compte_rendu(turns, opts, sink) },
        "Range les fichiers du dossier docs par date.",
    );
    assert_eq!(vu.iter().filter(|v| v.contains("delivered, missing or action")).count(), 1, "the host asks the judge once : {vu:#?}");
    assert_eq!(fin, "fait sans plan", "{boite:#?}");
    assert!(!boite.iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.contains("requested") || t.contains("check"))), "{boite:#?}");
}

/// **The verdict is the judge's first word** (archi's review): « delivered, not missing » is a
/// delivery, not a miss; a `contains` read it as « missing » and relaunched.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_verdict_is_the_judge_s_first_word() {
    let (boite, vu, fin) = une_demande(
        |turns, opts, sink| if est_le_juge(turns) { answer(turns, opts, sink, "Delivered, not missing.", Vec::new()) } else { compte_rendu(turns, opts, sink) },
        "Range les fichiers du dossier docs par date.",
    );
    assert_eq!(vu.iter().filter(|v| v.contains("delivered, missing or action")).count(), 1, "the host asks the judge once : {vu:#?}");
    assert_eq!(fin, "fait sans plan", "« Delivered, not missing. » is a delivery : {boite:#?}");
    assert!(!boite.iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.contains("requested") || t.contains("check"))), "{boite:#?}");
}

/// **The judge thinks little, and counts in the run's usage** (archi's review): a reasoning model
/// spends its 50 tokens thinking (2 805 reasoning tokens seen in D1 on gemini-3.8-flash) and answers
/// nothing; the call asks for the least reasoning, or leaves room for it. And its consumption is
/// recorded on the step's run, by the chat's meter, so that the run's usage carries it.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_judge_thinks_little_and_counts_in_the_run_s_usage() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let du_juge: Arc<Mutex<Vec<(usize, Option<crate::llm::ReasoningEffort>)>>> = Arc::default();
    let model: ChatModel = {
        let du_juge = du_juge.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let du_juge = du_juge.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                if est_le_juge(turns) {
                    du_juge.lock().unwrap().push((opts.max_tokens, opts.reasoning));
                    return answer(turns, opts, sink, "action", Vec::new());
                }
                if tool_results(turns).is_empty() { compte_rendu(turns, opts, sink) } else { answer(turns, opts, sink, "", Vec::new()) }
            })))
        })
    };
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    files.append("supervisor", "request", None, "Range les fichiers du dossier docs par date.").unwrap();
    let o = host.run_teammate(&h.id, "doc").unwrap();
    let appels = du_juge.lock().unwrap().clone();
    assert_eq!(appels.len(), 1, "one judge call : {appels:?}");
    let (max_tokens, reasoning) = appels[0];
    assert!(reasoning == Some(crate::llm::ReasoningEffort::Minimal) || max_tokens >= 512, "the judge's reasoning is cut, or has room : max_tokens {max_tokens}, reasoning {reasoning:?}");
    let run = o.steps.last().and_then(|s| s["run"].as_str()).unwrap_or_default().to_string();
    let du_run: Vec<String> = host.chat_meter.all().into_iter().filter(|c| c.run == run).map(|c| c.agent).collect();
    assert!(du_run.iter().any(|a| a == "content-judge"), "the judge's consumption is on the step's run ({run}) : {du_run:?}");
}

/// **A judge that fails does not block** (archi's third witness): the host says « fin sans texte, à
/// vérifier », and the task ends as today.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_judge_that_fails_says_the_end_is_to_be_checked() {
    let (boite, vu, fin) = une_demande(
        |turns, opts, sink| {
            if est_le_juge(turns) {
                Err(crate::llm::LlmError::Unavailable { status: Some(503), quota: crate::llm::Quota::Throttled, retries: 0, waited_ms: 0, message: "le juge est en panne".into() })
            } else {
                compte_rendu(turns, opts, sink)
            }
        },
        "Range les fichiers du dossier docs par date.",
    );
    assert_eq!(vu.iter().filter(|v| v.contains("delivered, missing or action")).count(), 1, "the host asks the judge once : {vu:#?}");
    assert_eq!(fin, "fait sans plan", "{boite:#?}");
    assert!(boite.iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.starts_with("ended without a text, to be checked"))), "{boite:#?}");
}

/// **A tool that waits says so on the bus, once, and says when it resumes** (c7 and archi, 19
/// November 2026: the terminal names a cause beside its animation, and only the host knows that a
/// call is waiting). `teammate.tool_wait_notice_s` at 1 s; the fake model calls `read_file` while the
/// test holds the task backend's slot for 2.5 s. The bus of the task carries one
/// `Waiting {run, cause: "tool", what: "read_file", since_ms}` on the step's run, past the threshold,
/// then one `Resumed {run, cause: "tool", waited_ms}` after it; the fast call of the next step, none.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_waiting_tool_says_so_on_the_bus_once_and_says_when_it_resumes() {
    use std::time::{Duration, Instant};
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["teammate"]["tool_wait_notice_s"] = json!(1);
    m["teammate"]["reply_wait_s"] = json!(5);
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    // A task reads its manifest from the committed declarations.
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "un seuil d'une seconde"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let tenu: Arc<Mutex<Option<Arc<super::Hosted>>>> = Arc::default();
    let model: ChatModel = {
        let tenu = tenu.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let tenu = tenu.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let lire = calls(&[("read_file", json!({ "path": "README.md" }))]);
                match (step_of(turns), tool_results(turns).len()) {
                    (1, 0) => {
                        // The slot of the task backend, held 2.5 s from another thread before the call.
                        let h = tenu.lock().unwrap().clone().expect("the task backend");
                        let (pris, attendre) = std::sync::mpsc::channel();
                        std::thread::spawn(move || {
                            let borne = Instant::now() + Duration::from_secs(5);
                            let _slot = loop {
                                if let Ok(slot) = h.backend.try_write() {
                                    break slot;
                                }
                                assert!(Instant::now() < borne, "the slot of the task backend is held by the turn");
                                std::thread::sleep(Duration::from_millis(10));
                            };
                            pris.send(()).unwrap();
                            std::thread::sleep(Duration::from_millis(2500));
                        });
                        attendre.recv_timeout(Duration::from_secs(6)).expect("the slot taken");
                        answer(turns, opts, sink, "", lire)
                    }
                    (_, 0) => answer(turns, opts, sink, "", lire),
                    (n, _) => answer(turns, opts, sink, "Le README dit « Petit dépôt ».", calls(&[("step_done", json!({ "item": "", "succeeded": n >= 2, "done": "lu", "remaining": if n >= 2 { "" } else { "relire" } }))])),
                }
            })))
        })
    };
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let tache = host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let t = host.hote_par_id(&tache.id).unwrap();
    let mut agent = host.with_backend(&t, |b| b.bus_de_publication().subscribe(crate::events::topic::AGENT)).unwrap();
    *tenu.lock().unwrap() = Some(t.clone());
    let files = host.teammate_files(&h.id, "doc").unwrap();
    files.append("supervisor", "request", None, "Lis le README.").unwrap();
    let o = host.run_teammate(&h.id, "doc").unwrap();
    let mut evenements = Vec::new();
    while let Ok(e) = agent.try_recv() {
        evenements.push(e.to_json());
    }
    let attentes: Vec<&Value> = evenements.iter().filter(|e| e["kind"] == "Waiting").collect();
    let reprises: Vec<&Value> = evenements.iter().filter(|e| e["kind"] == "Resumed").collect();
    let kinds: Vec<&Value> = evenements.iter().map(|e| &e["kind"]).collect();
    assert_eq!((attentes.len(), reprises.len()), (1, 1), "one Waiting and one Resumed ({o:?}) : {kinds:?}");
    let (w, r) = (attentes[0], reprises[0]);
    assert_eq!((&w["cause"], &w["what"], &r["cause"]), (&json!("tool"), &json!("read_file"), &json!("tool")), "{w} {r}");
    assert!(w["since_ms"].as_u64().is_some_and(|ms| ms >= 1000), "past the threshold : {w}");
    assert!(r["waited_ms"].as_u64().is_some_and(|ms| ms >= 2000), "the whole wait : {r}");
    assert_eq!(w["run"], r["run"], "{w} {r}");
    let premier_run = evenements.iter().find(|e| e["kind"] == "RunStarted").map(|e| e["run"].clone());
    assert_eq!(Some(w["run"].clone()), premier_run, "on the run of the first step : {kinds:?}");
    let (iw, ir) = (evenements.iter().position(|e| e["kind"] == "Waiting").unwrap(), evenements.iter().position(|e| e["kind"] == "Resumed").unwrap());
    assert!(iw < ir, "Waiting before Resumed : {kinds:?}");
    // The mailbox line is for the model too, which reads the mailbox at its frontier: in English
    // (archi's review of cx-chat-waiting-2).
    let boite = files.mailbox();
    let ligne = boite.iter().find(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.contains("read_file") && t.contains("1 s"))).map(|l| l["text"].as_str().unwrap_or_default().to_string());
    assert_eq!(ligne.as_deref(), Some("\"read_file\" has been waiting for 1 s (an index is probably holding the write); the call goes on, nothing is abandoned"), "{boite:#?}");
}

/// **Une request qui exige un report ne se clôt pas sans lui** (`expects: "report"`, déclaré
/// par le superviseur) : un `step_done` sans `report` dit « fait sans le livrable demandé » et
/// l'équipier attend la boîte sans appeler le modèle ; une ligne du superviseur le relance, le
/// report vient, la tâche finit.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_request_that_expects_a_report_does_not_end_without_it() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let pas = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let pas = pas.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let pas = pas.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if results.is_empty() {
                    pas.lock().unwrap().push(last_user(turns));
                }
                let fini = calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "verdicts établis", "remaining": "" }))]);
                match (step_of(turns), results.len()) {
                    (1, _) => answer(turns, opts, sink, "", fini),
                    (_, 0) => answer(turns, opts, sink, "", calls(&[("report", json!({ "text": "Verdicts : 31 lignes, 4 ajouts." }))])),
                    _ => answer(turns, opts, sink, "", fini),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Rends tes verdicts.", &json!({ "expects": "report" })).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let boite = attendre_dans(&files, "l'attente du livrable", &|b| dit_par_l_hote(b, "fait sans le livrable demandé") || b.iter().any(|l| l["kind"] == "ended"));
    assert!(!boite.iter().any(|l| l["kind"] == "ended"), "sans report, pas de fin : {boite:#?}");
    assert!(dit_par_l_hote(&boite, "fait sans le livrable demandé (un report dans la boîte) : j'attends le superviseur"), "{boite:#?}");
    assert!(pas.lock().unwrap()[0].contains("un report"), "la frontière dit ce que la demande exige : {}", pas.lock().unwrap()[0]);
    std::thread::sleep(std::time::Duration::from_secs(3));
    assert_eq!(pas.lock().unwrap().len(), 1, "aucun appel au modèle pendant l'attente");
    host.post_to_teammate(&h.id, "doc", "c7", "message", None, "Le report, s'il te plaît.", &Value::Null).expect("le rappel");
    let boite = attendre_dans(&files, "la fin après le report", &|b| b.iter().any(|l| l["kind"] == "ended"));
    assert!(boite.iter().any(|l| l["from"] == "teammate" && l["kind"] == "report"), "{boite:#?}");
    let fin = boite.iter().find(|l| l["kind"] == "ended").unwrap();
    assert_eq!(fin["text"], "fait sans plan", "{boite:#?}");
}

/// **Une request qui exige l'accord du superviseur ne se clôt pas sans lui**
/// (`expects: "approval"`) : un report suivi d'un `step_done` attend ; un `answer`
/// `approved: true` du superviseur est l'accord ; la tâche finit après lui.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_request_that_expects_approval_does_not_end_without_it() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            let fini = calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "proposé", "remaining": "" }))]);
            match (step_of(turns), tool_results(turns).len()) {
                (1, 0) => answer(turns, opts, sink, "", calls(&[("report", json!({ "text": "Je propose quatre ajouts." }))])),
                _ => answer(turns, opts, sink, "", fini),
            }
        })))
    });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Propose les ajouts, et attends mon accord.", &json!({ "expects": "approval" })).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let boite = attendre_dans(&files, "l'attente de l'accord", &|b| dit_par_l_hote(b, "fait sans le livrable demandé") || b.iter().any(|l| l["kind"] == "ended"));
    assert!(!boite.iter().any(|l| l["kind"] == "ended"), "sans accord, pas de fin : {boite:#?}");
    assert!(dit_par_l_hote(&boite, "fait sans le livrable demandé (l'accord du superviseur) : j'attends le superviseur"), "{boite:#?}");
    let report = boite.iter().find(|l| l["from"] == "teammate" && l["kind"] == "report").expect("le report").clone();
    host.post_to_teammate(&h.id, "doc", "c7", "answer", None, "d'accord", &json!({ "reply_to": report["id"], "approved": true })).expect("l'accord");
    let boite = attendre_dans(&files, "la fin après l'accord", &|b| b.iter().any(|l| l["kind"] == "ended"));
    assert_eq!(boite.iter().find(|l| l["kind"] == "ended").unwrap()["text"], "fait sans plan", "{boite:#?}");
}

/// **Une demande sans `expects` attend au moins une ligne de l'équipier** (relevé de tty, 15
/// novembre 2026, run `agent-1a1df060b51-346` : deux `read_file` refusés, puis un `step_done`
/// « exemple TypeScript fourni » sans rien dans la boîte). Le `step_done` qui la finirait sans
/// rien est relancé une fois : la boîte le dit et cite le `step_done` ; le pas suivant voit la
/// relance, son texte arrive dans la boîte, la tâche finit « fait sans plan », sans pas raté.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_request_without_expects_waits_for_a_line_and_the_lying_step_done_is_relaunched_once() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let frontieres = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let frontieres = frontieres.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let frontieres = frontieres.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if results.is_empty() {
                    frontieres.lock().unwrap().push(last_user(turns));
                }
                let menteur = calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "exemple TypeScript fourni", "remaining": "" }))]);
                match (step_of(turns), results.len()) {
                    (1, 0) => answer(turns, opts, sink, "", calls(&[("read_file", json!({ "path": "absent/saisie.rs" })), ("read_file", json!({ "path": "absent/README.md" }))])),
                    (1, _) => answer(turns, opts, sink, "", menteur),
                    _ => answer(turns, opts, sink, "```typescript\nclass Exemple {}\n```", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "exemple écrit", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "tty", "request", None, "Lis deux fichiers, puis réponds avec un exemple de classe TypeScript.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let boite = attendre_dans(&files, "la fin", &|b| b.iter().any(|l| l["kind"] == "ended"));
    assert!(dit_par_l_hote(&boite, "relance : ta demande attend une réponse dans la boîte"), "la relance est dite : {boite:#?}");
    let relance = boite.iter().find(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.starts_with("relance"))).unwrap();
    assert!(relance["text"].as_str().unwrap().contains("exemple TypeScript fourni"), "elle cite le step_done : {relance}");
    assert!(frontieres.lock().unwrap().iter().skip(1).any(|f| f.contains("relance")), "le pas suivant voit la relance : {:?}", frontieres.lock().unwrap());
    assert!(boite.iter().any(|l| l["from"] == "teammate" && l["text"].as_str().is_some_and(|t| t.contains("class Exemple"))), "le contenu arrive : {boite:#?}");
    assert_eq!(boite.iter().find(|l| l["kind"] == "ended").unwrap()["text"], "fait sans plan", "{boite:#?}");
    assert!(!boite.iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.contains("raté"))), "la relance n'est pas un pas raté : {boite:#?}");
}

/// **Toujours rien après la relance** : l'équipier attend le superviseur, sans rappeler le modèle,
/// au plus `teammate.reply_wait_s` ; au-delà, la demande se dit sans réponse de l'équipier et la
/// tâche finit en échec nommé (c7, 15 novembre 2026 : un équipier muet ne suspend pas son
/// superviseur pour toujours).
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn still_nothing_after_the_relaunch_waits_for_the_supervisor_then_fails_by_name() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["teammate"] = json!({ "reply_wait_s": 6 });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    // Dans les déclarations commitées : le manifeste de la tâche est celui de son worktree.
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "une attente courte"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let appels = Arc::new(Mutex::new(0usize));
    let model: ChatModel = {
        let appels = appels.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let appels = appels.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                *appels.lock().unwrap() += 1;
                answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "tty", "request", None, "Réponds-moi.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let boite = attendre_dans(&files, "l'attente", &|b| dit_par_l_hote(b, "fait sans le livrable demandé") || b.iter().any(|l| l["kind"] == "ended"));
    assert!(!boite.iter().any(|l| l["kind"] == "ended"), "pas de fin sans rien : {boite:#?}");
    assert!(dit_par_l_hote(&boite, "fait sans le livrable demandé (une ligne de l'équipier dans la boîte) : j'attends le superviseur"), "{boite:#?}");
    let vus = *appels.lock().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));
    assert_eq!(*appels.lock().unwrap(), vus, "aucun appel au modèle pendant l'attente");
    let boite = attendre_dans(&files, "l'échec nommé après le délai", &|b| b.iter().any(|l| l["kind"] == "ended"));
    let fin = boite.iter().find(|l| l["kind"] == "ended").unwrap();
    assert!(fin["text"].as_str().unwrap_or_default().starts_with("sans réponse de l'équipier"), "l'échec est nommé : {fin}");
    assert!(fin["text"].as_str().unwrap_or_default().contains("teammate.reply_wait_s"), "il dit le réglage : {fin}");
}

/// **Une tâche qui a écrit un fichier finit comme avant** : le travail est dans les fichiers, pas
/// dans la boîte ; ni relance ni attente (forme B de c7, 15 novembre 2026).
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_request_whose_teammate_wrote_a_file_ends_as_before() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| match tool_results(turns).len() {
            0 => answer(turns, opts, sink, "", calls(&[("edit_file", json!({ "path": "src/lib.rs", "old": "\"bonjour\"", "new": "\"salut\"" }))])),
            _ => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "salut écrit", "remaining": "" }))])),
        })))
    });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Remplace bonjour par salut.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let boite = attendre_dans(&files, "la fin", &|b| b.iter().any(|l| l["kind"] == "ended") || dit_par_l_hote(b, "relance"));
    assert!(!dit_par_l_hote(&boite, "relance"), "un fichier écrit : pas de relance : {boite:#?}");
    assert_eq!(boite.iter().find(|l| l["kind"] == "ended").unwrap()["text"], "fait sans plan", "{boite:#?}");
    let demande = boite.iter().position(|l| l["kind"] == "request").unwrap();
    assert_eq!(files.read_trace(), Some(demande), "la trace de la demande est sur disque");
}

/// **La trace survit à un redémarrage** : une demande dont l'équipier a écrit un fichier avant
/// l'arrêt de l'hôte (sa trace sur disque) ne se relance pas au pas de la reprise, qui finit sans
/// rien de plus (forme B de c7, 15 novembre 2026 : c'est le cas d'un redémarrage pendant le travail).
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_trace_left_before_a_restart_spares_the_request_its_relaunch() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "repris et fini", "remaining": "" }))]))
        })))
    });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    // L'hôte d'avant : la demande, un pas qui a écrit (sa trace), puis l'arrêt.
    files.append("supervisor", "request", None, "Écris une ligne dans le README.").unwrap();
    let demande = files.mailbox().iter().position(|l| l["kind"] == "request").unwrap();
    files.write_trace(demande);
    files.append("teammate", "step_done", None, "fait : la moitié").unwrap();
    // La reprise, bornée : relancée à tort, elle attendrait le superviseur, et le témoin le dit.
    let (tx, rx) = std::sync::mpsc::channel();
    let (hote_de_la_reprise, parent) = (host.clone(), h.id.clone());
    std::thread::spawn(move || {
        let _ = tx.send(hote_de_la_reprise.run_teammate(&parent, "doc"));
    });
    let reprise = rx.recv_timeout(std::time::Duration::from_secs(60));
    let boite = files.mailbox();
    assert!(!dit_par_l_hote(&boite, "relance"), "la trace d'avant l'arrêt compte : {boite:#?}");
    let o = reprise.expect("la reprise finit en 60 s").expect("la reprise");
    assert_eq!(o.ended, "fait sans plan", "{boite:#?}");
}

/// **`expects: "none"`** : une demande explicitement silencieuse finit sans ligne et sans relance.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_request_that_expects_none_ends_without_a_line() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "rangé", "remaining": "" }))]))
        })))
    });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Range le dossier.", &json!({ "expects": "none" })).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let boite = attendre_dans(&files, "la fin", &|b| b.iter().any(|l| l["kind"] == "ended") || dit_par_l_hote(b, "relance") || dit_par_l_hote(b, "fait sans le livrable demandé"));
    assert_eq!(boite.iter().find(|l| l["kind"] == "ended").map(|l| l["text"].clone()), Some(json!("fait sans plan")), "{boite:#?}");
    assert!(!dit_par_l_hote(&boite, "relance"), "{boite:#?}");
}

/// **`expects` se déclare sur une request, et seulement `report`, `approval` ou `none`.**
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn expects_is_a_request_field_with_three_values() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("pas de modèle ici".into()) });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    let e = host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Fais-le.", &json!({ "expects": "un roman" })).unwrap_err();
    assert!(e.contains("expects") && e.contains("report") && e.contains("approval") && e.contains("none"), "{e}");
    assert!(host.tache(&h.id, "doc").is_none(), "une request refusée n'ouvre pas de tâche");
}

/// **Le droit d'écrire d'une tâche se relit à chaque ouverture** (les quinze « oui » de Lucie, 15
/// novembre 2026 : `essai-tty`, ouverte quand le parent n'écrivait pas, gardait `ecrit: false`
/// au registre après que le manifeste a déclaré `task_writes`).
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_task_reads_its_write_right_from_its_parent_at_each_opening() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    // Le parent de Lucie à la création de la tâche : en lecture seule, sans task_writes, donc sans
    // outil d'écriture (un manifeste qui en déclare un sous read_only est refusé au chargement).
    let ecrire = |read_only: bool, task_writes: bool| {
        let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
        m["workspace"]["read_only"] = json!(read_only);
        m["workspace"]["task_writes"] = json!(task_writes);
        if let Some(outils) = m["tools"].as_object_mut() {
            outils.retain(|_, o| o["policy"]["write_files"] != json!(true));
        }
        std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    };
    ecrire(true, false);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("pas de modèle ici".into()) });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let t = host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let droit = || host.surcharges_de(&t.manifest).and_then(|s| s.workspace).map(|(_, ecrit)| ecrit);
    assert_eq!(droit(), Some(false), "le parent n'écrit pas : la tâche non plus");
    ecrire(true, true);
    assert_eq!(droit(), Some(true), "le parent déclare task_writes : la tâche écrit à sa prochaine ouverture");
}

/// Le manifeste du parent comme celui de Lucie au matin du 15 novembre : `task_writes`, mais aucun
/// outil d'écriture déclaré.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
fn parent_sans_outil_d_ecriture(manifeste: &std::path::Path, read_only: bool, task_writes: bool) {
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(manifeste).unwrap()).unwrap();
    m["workspace"]["read_only"] = json!(read_only);
    m["workspace"]["task_writes"] = json!(task_writes);
    if let Some(outils) = m["tools"].as_object_mut() {
        outils.retain(|_, o| o["policy"]["write_files"] != json!(true));
    }
    std::fs::write(manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    // Dans les déclarations commitées : la tâche part de leur copie git.
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "un parent sans outil d'écriture"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
}

/// **Toute tâche reçoit le socle d'un agent de code, quel que soit son manifeste** (vision du 15
/// novembre 2026 ; page 09) : sur un parent sans outil d'écriture, la tâche a `replace_text`,
/// `write` et `glob` ; un outil que le backend déclare garde le sien (l'homonyme gagne).
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn every_task_gets_the_code_agent_base_and_the_backend_homonym_wins() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    parent_sans_outil_d_ecriture(&manifeste, false, true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("pas de modèle ici".into()) });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let t = host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let (outils, graphe_de_read) = host
        .with_backend_id(&t.id, |b| {
            let m = &b.prepared().manifest;
            (m.tools.keys().cloned().collect::<Vec<_>>(), m.tools.get("read_file").map(|a| a.graph.display().to_string()))
        })
        .expect("la tâche s'ouvre comme backend");
    for o in ["replace_text", "replace_lines", "insert_lines", "write", "glob"] {
        assert!(outils.iter().any(|n| n == o), "le socle donne {o} : {outils:?}");
    }
    let principal: Vec<String> = host.with_backend(&h, |b| b.prepared().manifest.tools.keys().cloned().collect()).unwrap();
    assert!(!principal.iter().any(|n| n == "replace_text"), "le principal n'est pas une tâche : rien ne s'y greffe : {principal:?}");
    assert!(graphe_de_read.is_some_and(|g| !g.contains("atelier")), "read_file reste celui du backend : {:?}", host.with_backend_id(&t.id, |b| b.prepared().manifest.tools.get("read_file").map(|a| a.graph.clone())));
}

/// **Une tâche sans droit d'écrire ne reçoit que la lecture du socle** : `glob`, sans aucun outil
/// d'écriture (aucune écriture greffée ne contourne le droit de la tâche).
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_task_without_the_right_to_write_gets_only_the_reading_half_of_the_base() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    parent_sans_outil_d_ecriture(&manifeste, true, false);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("pas de modèle ici".into()) });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let t = host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let outils: Vec<String> = host.with_backend_id(&t.id, |b| b.prepared().manifest.tools.keys().cloned().collect()).expect("la tâche s'ouvre");
    assert!(outils.iter().any(|n| n == "glob"), "la lecture du socle : {outils:?}");
    for o in ["replace_text", "replace_lines", "insert_lines", "write"] {
        assert!(!outils.iter().any(|n| n == o), "pas d'écriture greffée sans droit d'écrire ({o}) : {outils:?}");
    }
}

/// **De bout en bout, sur le manifeste de Lucie** : un équipier édite par `replace_text` dans sa
/// tâche, le diff est au rendu et le fichier change dans le worktree de la tâche ; un
/// `replace_lines` sans lecture récente est refusé sans rien écrire ; un `write` sur un fichier qui
/// existe est refusé.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_teammate_edits_through_the_base_on_a_manifest_without_write_tools() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    parent_sans_outil_d_ecriture(&manifeste, false, true);
    let rendus = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let rendus = rendus.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let rendus = rendus.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if let Some(r) = results.last() {
                    rendus.lock().unwrap().push(r.clone());
                }
                match results.len() {
                    0 => answer(turns, opts, sink, "", calls(&[("replace_lines", json!({ "path": "src/lib.rs", "from": 1, "to": 1, "text": "x" }))])),
                    1 => answer(turns, opts, sink, "", calls(&[("write", json!({ "path": "README.md", "text": "écrasé" }))])),
                    2 => answer(turns, opts, sink, "", calls(&[("replace_text", json!({ "path": "src/lib.rs", "old": "\"bonjour\"", "new": "\"salut\"" }))])),
                    _ => answer(turns, opts, sink, "J'ai remplacé bonjour par salut.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "salut", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    // Comme le service au démarrage : les backends ouverts ensuite reçoivent l'hôte des worktrees.
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Remplace bonjour par salut dans src/lib.rs.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let boite = attendre_dans(&files, "la fin", &|b| b.iter().any(|l| l["kind"] == "ended"));
    let t = host.tache(&h.id, "doc").expect("la tâche");
    let racine = t.workspace.as_ref().map(|w| w.racine.clone()).expect("la racine de la tâche");
    let rendus = rendus.lock().unwrap().clone();
    assert!(rendus.len() >= 3, "trois rendus d'outil : {rendus:?} — {boite:#?}");
    assert!(rendus[0].contains("relis") || rendus[0].contains("lecture") || rendus[0].contains("lu "), "replace_lines sans lecture récente est refusé : {}", rendus[0]);
    assert!(rendus[1].contains("existe"), "write refuse d'écraser : {}", rendus[1]);
    assert!(rendus[2].contains("+") && rendus[2].contains("salut"), "replace_text rend son diff : {}", rendus[2]);
    let lib = std::fs::read_to_string(racine.join("src/lib.rs")).unwrap();
    assert!(lib.contains("\"salut\"") && !lib.contains("\"bonjour\""), "le fichier de la tâche a changé : {lib}");
    assert_eq!(std::fs::read_to_string(racine.join("README.md")).unwrap(), "# Petit dépôt\n", "write n'a rien écrasé");
}

/// **Algareth** (Lucie, 15 novembre 2026 : « ce premier-là je voudrais qu'il s'appelle Algareth ») :
/// une tâche ouverte avec un nom et une persona. L'invite du pas dit le nom et la persona ; les
/// lignes de l'équipier portent `name` à côté de `from: "teammate"` ; la garde de la demande (la
/// relance) lit toujours le rôle.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_named_teammate_says_its_name_and_its_lines_carry_it_beside_the_role() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let invites = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let invites = invites.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let invites = invites.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                if let Some(t) = turns.iter().find(|t| t.role == "system") {
                    invites.lock().unwrap().push(t.content.clone());
                }
                match (step_of(turns), tool_results(turns).len()) {
                    // Le premier pas finit sans rien laisser : la garde le relance.
                    (1, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))])),
                    _ => answer(turns, opts, sink, "Je suis Algareth ; voici la réponse.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "répondu", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache_nommee(&h.id, "doc", Some("Algareth"), Some("Calme et précis.")).expect("la tâche nommée s'ouvre");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Présente-toi.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let boite = attendre_dans(&files, "la fin", &|b| b.iter().any(|l| l["kind"] == "ended"));
    let invite = invites.lock().unwrap().first().cloned().unwrap_or_default();
    assert!(invite.contains("Your name is Algareth"), "l'invite dit le nom : {invite}");
    assert!(invite.contains("Calme et précis."), "et la persona : {invite}");
    let siennes: Vec<&Value> = boite.iter().filter(|l| l["from"] == "teammate").collect();
    assert!(!siennes.is_empty(), "{boite:#?}");
    assert!(siennes.iter().all(|l| l["name"] == "Algareth"), "chaque ligne de l'équipier porte son nom, le rôle reste teammate : {boite:#?}");
    assert!(boite.iter().filter(|l| l["from"] != "teammate").all(|l| l["name"].is_null()), "les autres lignes n'ont pas de nom d'agent : {boite:#?}");
    assert!(dit_par_l_hote(&boite, "relance"), "la garde de la demande lit toujours le rôle : {boite:#?}");
}

/// **Sans persona, l'invite n'ajoute rien de plus que le nom.**
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn without_a_persona_the_prompt_adds_only_the_name() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let invites = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let invites = invites.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let invites = invites.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                if let Some(t) = turns.iter().find(|t| t.role == "system") {
                    invites.lock().unwrap().push(t.content.clone());
                }
                answer(turns, opts, sink, "Bonjour.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "salué", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache_nommee(&h.id, "doc", Some("Algareth"), None).expect("la tâche nommée s'ouvre");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Bonjour.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    attendre_dans(&files, "la fin", &|b| b.iter().any(|l| l["kind"] == "ended"));
    let invite = invites.lock().unwrap().first().cloned().unwrap_or_default();
    assert!(invite.contains("Your name is Algareth") && !invite.contains("persona"), "le nom seul : {invite}");
}

/// **Sans nom choisi, l'équipier ne se présente pas par le défaut** (e2e_equipier sur Gemini, 15
/// novembre : « Bonjour, je m'appelle équipier ») : l'invite ne dit un nom que s'il est déclaré.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn an_unnamed_teammate_is_not_told_the_default_name() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let invites = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let invites = invites.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let invites = invites.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                if let Some(t) = turns.iter().find(|t| t.role == "system") {
                    invites.lock().unwrap().push(t.content.clone());
                }
                answer(turns, opts, sink, "Bonjour.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "salué", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Bonjour.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    attendre_dans(&files, "la fin", &|b| b.iter().any(|l| l["kind"] == "ended"));
    let invite = invites.lock().unwrap().first().cloned().unwrap_or_default();
    assert!(!invite.contains("Your name is"), "aucun nom n'est choisi : rien à dire : {invite}");
}

/// **Un pas qui répond sans step_done n'arrête pas la tâche** (Lucie, 16 novembre 2026 : Algareth
/// a lu la fonction et les tests, puis rendu la main avec « Ce qui reste à exécuter au prochain
/// pas : … »). Le pas suivant part de lui-même, sa frontière dit de continuer jusqu'au bout, et la
/// tâche finit sans attendre le superviseur. La copie de la réponse est la réponse seule, marquée
/// `origin: "host_copy"`, sans préfixe.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_step_that_answers_without_step_done_is_continued_without_waiting() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let frontieres = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let frontieres = frontieres.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let frontieres = frontieres.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if results.is_empty() {
                    frontieres.lock().unwrap().push(last_user(turns));
                }
                match (step_of(turns), results.len()) {
                    (1, 0) => answer(turns, opts, sink, "", calls(&[("read_file", json!({ "path": "src/lib.rs" }))])),
                    // Le défaut de Lucie : une réponse qui annonce la suite, sans step_done.
                    (1, _) => answer(turns, opts, sink, "Next I will write the test, then run it.", vec![]),
                    _ => answer(turns, opts, sink, "The test is written and passes.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "test written", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Write a test for bonjour and run it.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let boite = attendre_dans(&files, "la fin", &|b| b.iter().any(|l| l["kind"] == "ended"));
    assert_eq!(boite.iter().find(|l| l["kind"] == "ended").unwrap()["text"], "fait sans plan", "la tâche finit d'elle-même : {boite:#?}");
    assert!(frontieres.lock().unwrap().iter().skip(1).any(|f| f.contains("until the request is done")), "le pas suivant est relancé jusqu'au bout : {:?}", frontieres.lock().unwrap());
    assert!(!boite.iter().any(|l| l["text"].as_str().is_some_and(|t| t.contains("s'est terminé sans step_done"))), "plus de préfixe dans la copie : {boite:#?}");
    let copie = boite.iter().find(|l| l["text"] == "Next I will write the test, then run it.").expect("la réponse du pas, seule");
    assert_eq!((&copie["from"], &copie["origin"]), (&json!("teammate"), &json!("host_copy")), "{copie}");
}

/// **A step that reads before it writes is not cut by a count of calls** (S1, D6 campaign of
/// 16 Nov 2026: 7 steps, 82 calls, 0 edit; the step stopped at 12 tool calls and 12 model calls).
/// Fifteen reads, then an edit, then step_done: one step, and the file is written.
#[test]
fn a_step_reads_fifteen_times_then_writes_and_finishes_in_one_step() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let seen = Arc::new(Mutex::new((Vec::<usize>::new(), Vec::<String>::new())));
    let model: ChatModel = {
        let seen = seen.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let seen = seen.clone();
            Ok(Arc::new(CallbackLlm::new("reads-then-writes", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                {
                    let mut s = seen.lock().unwrap();
                    s.0.push(step_of(turns));
                    s.1 = results.clone();
                }
                match results.len() {
                    n if n < 15 => answer(turns, opts, sink, "", calls(&[("read_file", json!({ "path": "src/lib.rs" }))])),
                    15 => answer(turns, opts, sink, "", calls(&[("edit_file", json!({ "path": "README.md", "content": "# Documentation\nThe bonjour function returns a greeting.\n" }))])),
                    _ => answer(turns, opts, sink, "README.md documents bonjour.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "documentation written", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the parent opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Document bonjour in README.md.", &Value::Null).expect("the request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let mailbox = attendre_dans(&files, "the end", &|b| b.iter().any(|l| l["kind"] == "ended"));
    let (steps, results) = seen.lock().unwrap().clone();
    assert!(!results.iter().any(|r| r.contains("declare step_done now")), "no call is refused: {results:#?}");
    assert!(steps.iter().all(|&n| n == 1), "one step only: {steps:?}");
    assert_eq!(mailbox.iter().filter(|l| l["kind"] == "step_done").count(), 1, "{mailbox:#?}");
    assert_eq!(mailbox.iter().find(|l| l["kind"] == "ended").unwrap()["text"], "fait sans plan", "{mailbox:#?}");
    let task = host.tache(&h.id, "doc").unwrap();
    let root = &task.workspace.as_ref().unwrap().racine;
    assert!(std::fs::read_to_string(root.join("README.md")).unwrap().contains("bonjour"));
}

/// **What bounds a step is its context** (S1): once the last model call carried more tokens than
/// `teammate.step_context_tokens` (64 000 by default), work tools are refused with a message that
/// names the bound and says to declare step_done; step_done itself passes.
#[test]
fn a_step_whose_context_is_full_is_told_to_declare_step_done() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let seen = seen.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let seen = seen.clone();
            Ok(Arc::new(CallbackLlm::new("full-context", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if step_of(turns) == 1 {
                    *seen.lock().unwrap() = results.clone();
                }
                let mut out = match results.len() {
                    0 | 1 => answer(turns, opts, sink, "", calls(&[("read_file", json!({ "path": "src/lib.rs" }))])),
                    _ => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": false, "done": "read src/lib.rs", "remaining": "write the documentation" }))])),
                };
                // The second call reports a full context: the read it asks for is refused.
                if let (Ok((_, usage)), 1) = (&mut out, results.len()) {
                    usage.prompt_tokens = 70_000;
                }
                out
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the parent opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Document bonjour in README.md.", &Value::Null).expect("the request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    attendre_dans(&files, "the first step_done", &|b| b.iter().any(|l| l["kind"] == "step_done"));
    host.post_to_teammate(&h.id, "doc", "lucie", "stop", None, "stop", &Value::Null).ok();
    let results = seen.lock().unwrap().clone();
    assert!(!results[0].contains("step_context_tokens"), "the first read passes: {results:#?}");
    assert!(results[1].contains("step_context_tokens") && results[1].contains("step_done"), "the second read is refused by the context bound: {results:#?}");
}

/// **The next step sees what the previous one did** (S2, D6 campaign of 16 Nov 2026: each step
/// started from a fresh thread with only the request and the step_done lines, and read everything
/// again). Step 1 reads src/lib.rs and stops without step_done; step 2's frontier carries a
/// `Previous step` block that names the file and its range, and a replace_lines on those lines
/// passes without a new read.
#[test]
fn the_next_step_sees_what_the_previous_step_read() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let seen = Arc::new(Mutex::new((Vec::<String>::new(), Vec::<String>::new())));
    let model: ChatModel = {
        let seen = seen.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let seen = seen.clone();
            Ok(Arc::new(CallbackLlm::new("previous-step", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if results.is_empty() {
                    seen.lock().unwrap().0.push(last_user(turns));
                }
                if step_of(turns) == 1 && !results.is_empty() {
                    seen.lock().unwrap().0.push(format!("STEP 1 RESULTS: {results:?}"));
                }
                if step_of(turns) == 2 {
                    seen.lock().unwrap().1 = results.clone();
                }
                match (step_of(turns), results.len()) {
                    (1, 0) => answer(turns, opts, sink, "", calls(&[("read_file", json!({ "path": "src/lib.rs", "offset": 1, "limit": 20 }))])),
                    (1, _) => answer(turns, opts, sink, "I read src/lib.rs; next I edit its first line.", vec![]),
                    (2, 0) => answer(turns, opts, sink, "", calls(&[("replace_lines", json!({ "path": "src/lib.rs", "from": 1, "to": 1, "text": "//! A small crate.\n" }))])),
                    _ => answer(turns, opts, sink, "Done.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "first line documented", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the parent opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Document the first line of src/lib.rs.", &Value::Null).expect("the request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let mailbox = attendre_dans(&files, "the end", &|b| b.iter().any(|l| l["kind"] == "ended"));
    let (frontiers, step2_results) = seen.lock().unwrap().clone();
    let step1 = frontiers.iter().find(|f| f.starts_with("STEP 1 RESULTS")).cloned().unwrap_or_default();
    let frontiers: Vec<String> = frontiers.into_iter().filter(|f| !f.starts_with("STEP 1 RESULTS")).collect();
    let second = frontiers.get(1).cloned().unwrap_or_default();
    assert!(second.contains("Previous step") && second.contains("src/lib.rs") && second.contains("lines 1-3"), "the second frontier names what step 1 read: {second}\n({step1})");
    assert!(second.contains("next I edit its first line"), "and the last text of step 1: {second}");
    // The read imprint lets the edit through without a new read (the write itself goes through
    // the atelier's host, which this bench does not run: its refusal is not the imprint's).
    assert!(step2_results.first().is_some_and(|r| !r.contains("relis d'abord")), "replace_lines needs no new read: {step2_results:?}");
    assert_eq!(mailbox.iter().find(|l| l["kind"] == "ended").unwrap()["text"], "fait sans plan", "{mailbox:#?}");
}

/// **A declared scripted `Previous step` that fails never breaks the step, and the box says it** (validation of the
/// scripted node, 21 November 2026), by the host's path: the manifest declares `teammate.carry_over_node`, its
/// `nodes/` script throws; the second frontier still carries the block (the Rust one), and a `host` report names the
/// script and its line, once.
#[test]
fn a_failing_scripted_previous_step_falls_back_and_the_box_says_it() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["teammate"]["carry_over_node"] = json!("PreviousStep");
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let ts = manifeste.parent().unwrap().join("nodes/previous_step.ts");
    let casse = std::fs::read_to_string(&ts).expect("the code template ships nodes/previous_step.ts").replace("  const w = inputs.work;", "  const w = inputs.work;\n  throw new Error(\"cassé exprès\");");
    std::fs::write(&ts, casse).unwrap();
    // A task starts from a commit of the declarations: the declaration and the broken script are committed, as a user
    // would.
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "teammate.carry_over_node, un script cassé"]] {
        let o = std::process::Command::new("git").arg("-C").arg(declarations).args(args).output().unwrap();
        assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
    }
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let seen = seen.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let seen = seen.clone();
            Ok(Arc::new(CallbackLlm::new("previous-step-script", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if results.is_empty() {
                    seen.lock().unwrap().push(last_user(turns));
                }
                match (step_of(turns), results.len()) {
                    (1, 0) => answer(turns, opts, sink, "", calls(&[("read_file", json!({ "path": "src/lib.rs", "offset": 1, "limit": 20 }))])),
                    (1, _) => answer(turns, opts, sink, "I read src/lib.rs.", vec![]),
                    _ => answer(turns, opts, sink, "Done.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "read", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the parent opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Read src/lib.rs.", &Value::Null).expect("the request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let mailbox = attendre_dans(&files, "the end", &|b| b.iter().any(|l| l["kind"] == "ended"));
    let second = seen.lock().unwrap().get(1).cloned().unwrap_or_default();
    assert!(second.contains("Previous step") && second.contains("src/lib.rs"), "the Rust block serves: {second}");
    let rapports: Vec<String> = mailbox
        .iter()
        .filter(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|s| s.contains("repli Rust")))
        .map(|l| l["text"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(rapports.len(), 1, "the box says the fallback once: {mailbox:#?}");
    assert!(rapports[0].contains("previous_step.ts") && rapports[0].contains("cassé exprès") && rapports[0].contains("line"), "{}", rapports[0]);
}

/// **A read of 5 lines covers 5 lines** (found with S2, 18 Nov 2026): the task's read_file (the
/// code backend's) takes `limit`, and the read imprint only knew `length` — a read of 5 lines was
/// counted as 200, and a replace_lines on a line never read passed.
#[test]
fn a_read_with_a_limit_covers_only_its_lines() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    let lines: String = (1..=30).map(|n| format!("// line {n}\n")).collect();
    std::fs::write(depot.join("src/long.rs"), &lines).unwrap();
    let git = |args: &[&str]| assert!(std::process::Command::new("git").arg("-C").arg(&depot).args(args).output().unwrap().status.success());
    git(&["add", "src/long.rs"]);
    git(&["commit", "-q", "-m", "a long file"]);
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let seen = seen.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let seen = seen.clone();
            Ok(Arc::new(CallbackLlm::new("read-limit", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if step_of(turns) == 1 {
                    *seen.lock().unwrap() = results.clone();
                }
                match (step_of(turns), results.len()) {
                    (1, 0) => answer(turns, opts, sink, "", calls(&[("read_file", json!({ "path": "src/long.rs", "offset": 1, "limit": 5 }))])),
                    (1, 1) => answer(turns, opts, sink, "", calls(&[("replace_lines", json!({ "path": "src/long.rs", "from": 20, "to": 20, "text": "// changed\n" }))])),
                    _ => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "tried", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the parent opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Change line 20 of src/long.rs.", &Value::Null).expect("the request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    attendre_dans(&files, "the first step_done", &|b| b.iter().any(|l| l["kind"] == "step_done"));
    host.post_to_teammate(&h.id, "doc", "lucie", "stop", None, "stop", &Value::Null).ok();
    let results = seen.lock().unwrap().clone();
    assert!(results[0].contains("line 5") && !results[0].contains("\"error\""), "the read passes with limit: {results:?}");
    assert!(results[1].contains("relis d'abord"), "line 20 was not read: replace_lines is refused: {results:?}");
}

/// **A follow-up request adds to the open request** (S3, D6 campaign of 16 Nov 2026: a short
/// « continue » became the request, and the agent went looking for the brief in a vision page).
/// The frontier after a « continue » carries the first request first, then the follow-up, and
/// the step_done of the step before it.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_follow_up_request_adds_to_the_open_request() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let frontiers = Arc::new(Mutex::new(Vec::<String>::new()));
    let model = |frontiers: Arc<Mutex<Vec<String>>>, files: Option<super::teammate::TeammateFiles>| -> ChatModel {
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (frontiers, files) = (frontiers.clone(), files.clone());
            Ok(Arc::new(CallbackLlm::new("follow-up", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if results.is_empty() {
                    frontiers.lock().unwrap().push(last_user(turns));
                }
                match (step_of(turns), results.len()) {
                    (1, 0) => {
                        // The supervisor sends « continue » while step 1 runs.
                        if let Some(f) = &files {
                            f.append("lucie", "request", None, "continue").unwrap();
                        }
                        answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": false, "done": "the test is written", "remaining": "run it" }))]))
                    }
                    _ => answer(turns, opts, sink, "The test passes.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "the test runs green", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = hote(d.path(), model(frontiers.clone(), None)).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the parent opens");
    host.ouvrir_une_tache(&h.id, "doc").expect("the task opens");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let host = host.with_chat_model(model(frontiers.clone(), Some(files.clone())));
    files.append("lucie", "request", None, "Write a test for bonjour").unwrap();
    host.run_teammate(&h.id, "doc").expect("the teammate runs");
    let frontiers = frontiers.lock().unwrap().clone();
    let second = frontiers.get(1).cloned().unwrap_or_default();
    let first_at = second.find("Write a test for bonjour");
    let follow_at = second.find("Follow-up: continue");
    assert!(first_at.is_some() && follow_at.is_some() && first_at < follow_at, "the first request, then the follow-up: {second}");
    assert!(second.contains("the test is written"), "the step_done before the follow-up is kept: {second}");
}

/// A memory that only records `put_turn` (the teammate's witness of the session archived as it
/// goes).
struct TurnsOnly(Mutex<Vec<Value>>);

impl crate::daemon_host::rangement::Rangeur for TurnsOnly {
    fn put_turn(&self, _memoire: &std::path::Path, line: &Value) -> Result<Value, String> {
        self.0.lock().unwrap().push(line.clone());
        Ok(json!({ "written": true }))
    }
    fn archive_slice(&self, _memoire: &std::path::Path, args: &Value) -> Result<Value, String> {
        Ok(json!({ "report": { "ok": true, "errors": [] }, "archived_through_turn": args["to_turn"] }))
    }
}

/// **The teammate's steps are archived as they go too** (18 Nov 2026): each step's messages go
/// to the memory under the run `<task>:<parent>`.
#[test]
fn the_teammate_steps_go_to_the_memory_as_they_go() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["bilans"] = json!({ "memoire": "memoire/backend.json" });
    m["memory"] = json!({ "slice_tokens": 30 });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    // The task takes the manifest as committed in its declarations.
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "the memory of the session"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("archived-steps", 8192, |turns, opts, sink| {
            answer(turns, opts, sink, &"y".repeat(200), calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "said hello", "remaining": "" }))]))
        })))
    });
    let memory = Arc::new(TurnsOnly(Mutex::new(Vec::new())));
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_rangeur(memory.clone()));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the parent opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Say hello.", &json!({ "expects": "none" })).expect("the request");
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let run = format!("doc:{}", h.id);
    loop {
        let lines = memory.0.lock().unwrap().clone();
        if lines.iter().any(|l| l["run"] == run.as_str() && l["turn"] == 1 && l["seq"] == 0 && l["role"] == "user") && lines.iter().any(|l| l["role"] == "tool" && l["tool_name"] == "step_done") {
            break;
        }
        assert!(std::time::Instant::now() < fin, "the step's messages under {run}: {lines:#?}");
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// **A paired task holds its entry while it works** (mode pair, lot 1, page 05 of 19 Nov 2026):
/// with `teammate.pair`, the agent's entry is in the sessions registry during its step, named after
/// the agent and busy; idle after the teammate ends (lot 2: a pair is reachable between two works);
/// gone once the task is closed. A task without it writes nothing.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_paired_task_holds_its_entry_while_it_works_and_leaves_none() {
    for pair in [true, false] {
        let d = tempfile::tempdir().unwrap();
        let (_depot, manifeste) = depot_inscrit(d.path(), true);
        let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
        m["teammate"] = json!({ "pair": pair });
        std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
        let declarations = manifeste.parent().unwrap();
        for args in [&["add", "-A"][..], &["commit", "-q", "-m", "le mode pair"]] {
            assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
        }
        let registre = super::pair::Registre { sessions: d.path().join("sessions"), socks: d.path().join("socks") };
        let vues = Arc::new(Mutex::new(Vec::<Value>::new()));
        let model: ChatModel = {
            let (vues, sessions) = (vues.clone(), registre.sessions.clone());
            Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
                let (vues, sessions) = (vues.clone(), sessions.clone());
                Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                    let entree = std::fs::read(sessions.join(format!("{}.json", std::process::id()))).ok().and_then(|o| serde_json::from_slice(&o).ok());
                    vues.lock().unwrap().push(entree.unwrap_or(Value::Null));
                    answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
                })))
            })
        };
        let host = hote(d.path(), model).with_runs_root(d.path().join("runs")).with_pair_registry(registre.clone());
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
        host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
        let files = host.teammate_files(&h.id, "doc").unwrap();
        files.append_with("supervisor", "request", None, "Fais-le.", &json!({ "expects": "none" })).unwrap();
        host.run_teammate(&h.id, "doc").unwrap();
        let vues = vues.lock().unwrap();
        let restes: Vec<_> = std::fs::read_dir(&registre.sessions).map(|r| r.flatten().map(|e| e.file_name()).collect()).unwrap_or_default();
        if pair {
            assert_eq!(vues[0]["status"], "busy", "the entry is there during the step: {:?}", vues[0]);
            assert!(vues[0]["name"].as_str().is_some_and(|n| !n.is_empty()), "named after the agent: {:?}", vues[0]);
            let apres: Value = serde_json::from_slice(&std::fs::read(registre.sessions.join(format!("{}.json", std::process::id()))).unwrap()).unwrap();
            assert_eq!(apres["status"], "idle", "reachable between two works: idle after the teammate ends");
            drop(vues);
            host.fermer_une_tache(&h.id, "doc").unwrap();
            let restes: Vec<_> = std::fs::read_dir(&registre.sessions).map(|r| r.flatten().map(|e| e.file_name()).collect()).unwrap_or_default();
            assert!(restes.is_empty(), "nothing left once the task is closed: {restes:?}");
        } else {
            assert_eq!(vues[0], Value::Null, "no pair, no entry");
            assert!(restes.is_empty(), "{restes:?}");
        }
    }
}

/// **A message to a paired task's socket is a request in its mailbox** (mode pair, lot 2): a
/// session writes the frame relevé on 16 Nov 2026 to the socket named by the entry; the mailbox
/// has a request from « session », its text the envelope as sent, with the peer's name and its
/// reply address; the teammate reads it at its next step.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_message_to_a_paired_task_socket_is_a_request_in_its_mailbox() {
    use std::io::Write;
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["teammate"] = json!({ "pair": true });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "le mode pair"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let registre = super::pair::Registre { sessions: d.path().join("sessions"), socks: d.path().join("socks") };
    let enveloppe = "<cross-session-message from=\"uds:/run/user/1000/cc-socks/42.sock\" from-name=\"rag3db-5a\" from-mode=\"prompting\">\nWhat is 2+2?\n</cross-session-message>";
    let frontieres = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let (frontieres, sessions) = (frontieres.clone(), registre.sessions.clone());
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (frontieres, sessions) = (frontieres.clone(), sessions.clone());
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                if tool_results(turns).is_empty() {
                    let n = { let mut f = frontieres.lock().unwrap(); f.push(last_user(turns)); f.len() };
                    if n == 1 {
                        // During the first step, a session writes to the socket of the entry.
                        let e: Value = serde_json::from_slice(&std::fs::read(sessions.join(format!("{}.json", std::process::id()))).unwrap()).unwrap();
                        let mut c = std::os::unix::net::UnixStream::connect(e["messagingSocketPath"].as_str().unwrap()).unwrap();
                        let trame = json!({ "msgV": 1, "msg_id": "m-1", "type": "user", "priority": "next", "from": "uds:/run/user/1000/cc-socks/42.sock",
                                            "message": { "role": "user", "content": "<cross-session-message from=\"uds:/run/user/1000/cc-socks/42.sock\" from-name=\"rag3db-5a\" from-mode=\"prompting\">\nWhat is 2+2?\n</cross-session-message>" } });
                        c.write_all(format!("{trame}\n").as_bytes()).unwrap();
                        drop(c);
                        std::thread::sleep(std::time::Duration::from_millis(300));
                    }
                }
                answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_pair_registry(registre));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    files.append_with("supervisor", "request", None, "Fais-le.", &json!({ "expects": "none" })).unwrap();
    // As in service: through start_teammate, so that a message during a step starts no second one.
    assert!(host.start_teammate(&h.id, "doc"));
    // The session's request waits for the teammate's line (no `expects`): once read, stop it.
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(90);
    while !frontieres.lock().unwrap().iter().skip(1).any(|t| t.contains("What is 2+2?")) {
        assert!(std::time::Instant::now() < fin, "the teammate reads the message at a next step: {:?}", frontieres.lock().unwrap());
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    host.post_to_teammate(&h.id, "doc", "supervisor", "stop", None, "enough", &json!({})).unwrap();
    attendre_dans(&files, "the teammate ends", &|b| b.iter().any(|l| l["kind"] == "ended"));
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while host.teammate_running(&h.id, "doc") && std::time::Instant::now() < fin {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let boite = files.mailbox();
    assert_eq!(boite.iter().filter(|l| l["kind"] == "ended").count(), 1, "one teammate, one end: {boite:#?}");
    let venue: Vec<&Value> = boite.iter().filter(|l| l["from"] == "session").collect();
    assert_eq!(venue.len(), 1, "the message is in the mailbox once: {boite:#?}");
    assert_eq!(venue[0]["kind"], "request");
    assert_eq!(venue[0]["text"], enveloppe);
    assert_eq!(venue[0]["peer"], "rag3db-5a");
    assert_eq!(venue[0]["reply_to_address"], "uds:/run/user/1000/cc-socks/42.sock");
    let f = frontieres.lock().unwrap();
    assert!(f.iter().skip(1).any(|t| t.contains("What is 2+2?")), "the teammate reads it at a next step: {f:?}");
}

/// **A pair is reachable between two works** (mode pair, lot 2, c7 on 19 Nov 2026): a paired task
/// is in the registry, idle, as soon as it is open; a message to its socket starts its teammate;
/// the entry is idle again when the teammate ends; closing the task leaves neither entry nor socket.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_paired_task_is_reachable_between_two_works_and_leaves_with_its_task() {
    use std::io::Write;
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["teammate"] = json!({ "pair": true });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "le mode pair"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let registre = super::pair::Registre { sessions: d.path().join("sessions"), socks: d.path().join("socks") };
    let frontieres = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let frontieres = frontieres.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let frontieres = frontieres.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                if tool_results(turns).is_empty() {
                    frontieres.lock().unwrap().push(last_user(turns));
                }
                answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "4", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_pair_registry(registre.clone()));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let fichier = registre.sessions.join(format!("{}.json", std::process::id()));
    let lire = || -> Value { serde_json::from_slice(&std::fs::read(&fichier).unwrap_or_default()).unwrap_or(Value::Null) };
    assert_eq!(lire()["status"], "idle", "visible as soon as the task is open, before any request");
    let socket = std::path::PathBuf::from(lire()["messagingSocketPath"].as_str().unwrap());
    let mut c = std::os::unix::net::UnixStream::connect(&socket).unwrap();
    let trame = json!({ "msgV": 1, "msg_id": "m-1", "type": "user", "priority": "next", "from": "uds:/run/user/1000/cc-socks/42.sock",
                        "message": { "role": "user", "content": "<cross-session-message from=\"uds:/run/user/1000/cc-socks/42.sock\" from-name=\"rag3db-5a\" from-mode=\"prompting\">\nWhat is 2+2?\n</cross-session-message>" } });
    c.write_all(format!("{trame}\n").as_bytes()).unwrap();
    drop(c);
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(90);
    while !frontieres.lock().unwrap().iter().any(|t| t.contains("What is 2+2?")) {
        assert!(std::time::Instant::now() < fin, "the message starts the teammate, which reads it: {:#?}", files.mailbox());
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    // It waits for its own line (lot 3 answers the peer): stop it.
    host.post_to_teammate(&h.id, "doc", "supervisor", "stop", None, "enough", &json!({})).unwrap();
    attendre_dans(&files, "the teammate ends", &|b| b.iter().any(|l| l["kind"] == "ended"));
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while (host.teammate_running(&h.id, "doc") || lire()["status"] != "idle") && std::time::Instant::now() < fin {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(lire()["status"], "idle", "idle again once its teammate ends");
    host.fermer_une_tache(&h.id, "doc").unwrap();
    assert!(!fichier.exists(), "the entry goes with the task");
    assert!(!socket.exists(), "and its socket");
}

/// **A pair answers a session** (mode pair, lot 3): a session writes to the socket of a paired
/// task; the teammate answers with send_to_session to the address the line carries; the session
/// receives the frame of the bus with the answer; the mailbox keeps the message out; and that
/// answer is the teammate's line: the request ends by itself.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_pair_answers_a_session_and_its_request_ends() {
    use std::io::{Read, Write};
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["teammate"] = json!({ "pair": true });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "le mode pair"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let registre = super::pair::Registre { sessions: d.path().join("sessions"), socks: d.path().join("socks") };
    // The session: its socket keeps what it receives.
    std::fs::create_dir_all(&registre.socks).unwrap();
    let sa_socket = registre.socks.join("42.sock");
    let ecoute = std::os::unix::net::UnixListener::bind(&sa_socket).unwrap();
    let recues = Arc::new(Mutex::new(Vec::<Value>::new()));
    {
        let recues = recues.clone();
        std::thread::spawn(move || {
            for c in ecoute.incoming().flatten() {
                let mut o = Vec::new();
                let _ = (&c).read_to_end(&mut o);
                if let Ok(v) = serde_json::from_slice::<Value>(&o) {
                    recues.lock().unwrap().push(v);
                }
            }
        });
    }
    let adresse = format!("uds:{}", sa_socket.display());
    let outils = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let (adresse, outils) = (adresse.clone(), outils.clone());
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (adresse, outils) = (adresse.clone(), outils.clone());
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                *outils.lock().unwrap() = opts.tools.iter().map(|d| d.name.clone()).collect();
                if tool_results(turns).is_empty() && last_user(turns).contains("What is 2+2?") {
                    return answer(turns, opts, sink, "", calls(&[("send_to_session", json!({ "to": adresse, "text": "4" }))]));
                }
                answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "répondu", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_pair_registry(registre.clone()));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let e: Value = serde_json::from_slice(&std::fs::read(registre.sessions.join(format!("{}.json", std::process::id()))).unwrap()).unwrap();
    let mut c = std::os::unix::net::UnixStream::connect(e["messagingSocketPath"].as_str().unwrap()).unwrap();
    let trame = json!({ "msgV": 1, "msg_id": "m-1", "type": "user", "priority": "next", "from": adresse,
                        "message": { "role": "user", "content": format!("<cross-session-message from=\"{adresse}\" from-name=\"rag3db-5a\" from-mode=\"prompting\">\nWhat is 2+2?\n</cross-session-message>") } });
    c.write_all(format!("{trame}\n").as_bytes()).unwrap();
    drop(c);
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let boite = attendre_dans(&files, "the request ends by itself once answered", &|b| b.iter().any(|l| l["kind"] == "ended"));
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while recues.lock().unwrap().is_empty() && std::time::Instant::now() < fin {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let recues = recues.lock().unwrap();
    assert_eq!(recues.len(), 1, "the session received one frame: {recues:?}");
    assert!(recues[0]["message"]["content"].as_str().unwrap_or_default().contains("\n4\n"), "{:?}", recues[0]);
    let sortie: Vec<&Value> = boite.iter().filter(|l| l["from"] == "teammate" && l["kind"] == "message").collect();
    assert_eq!(sortie.len(), 1, "the mailbox keeps the message out: {boite:#?}");
    assert_eq!(sortie[0]["direction"], "out");
    assert_eq!(sortie[0]["to"], adresse.as_str());
    assert_eq!(sortie[0]["text"], "4");
    assert!(outils.lock().unwrap().iter().any(|n| n == "send_to_session"), "a paired task's teammate has the tool");
}

/// **A task that is not paired has no send_to_session** (mode pair, lot 3).
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_task_that_is_not_paired_has_no_send_to_session() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let outils = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let outils = outils.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let outils = outils.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                *outils.lock().unwrap() = opts.tools.iter().map(|d| d.name.clone()).collect();
                answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
            })))
        })
    };
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    files.append_with("supervisor", "request", None, "Fais-le.", &json!({ "expects": "none" })).unwrap();
    host.run_teammate(&h.id, "doc").unwrap();
    let outils = outils.lock().unwrap();
    assert!(!outils.is_empty() && !outils.iter().any(|n| n == "send_to_session"), "{outils:?}");
}

/// **The page of a task follows its teammate by the stream, and answers its approvals** (c7, 20 November 2026: a run
/// of the terminal seen and reachable in the IDE; Lucie's invariant, one host for both). The page of the task « doc » is
/// open before the request. Without reloading, its mailbox shows the teammate's lines as they come, and the command
/// the gate held (`echo accorde-moi`) becomes a card with « oui » and « non », as the terminal's o / n. « oui » from
/// the page: the command runs at the next step, and the card goes.
#[cfg(all(unix, feature = "code", feature = "rag3db-native"))]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : étape séparée")]
#[test]
fn the_page_of_a_task_follows_its_teammate_and_answers_its_approval() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    use std::time::Duration;
    let Some(binaire) = navigateur_ou_saut("the_page_of_a_task_follows_its_teammate_and_answers_its_approval") else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            let results = tool_results(turns);
            let accorde = turns.iter().any(|t| t.content.contains("commande accordée"));
            let run = |ligne: &str| ("run_command", json!({ "command": ligne }));
            match (accorde, results.len()) {
                (false, 0) => answer(turns, opts, sink, "", calls(&[run("echo accorde-moi")])),
                (false, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": false, "done": "rien", "remaining": "echo attend l'accord" }))])),
                (true, 0) => answer(turns, opts, sink, "", calls(&[run("echo accorde-moi")])),
                (true, _) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "echo joué après l'accord", "remaining": "" }))])),
            }
        })))
    });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let racine = format!("http://{}", ecoute.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(ecoute, None).unwrap();
    let servi = host.clone();
    let fil = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(servi, Some("j".into())));
    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(&format!("{racine}/ide/code/{}/taches/doc", h.id), "!!document.querySelector('section.equipier') && document.getElementById('lien')?.textContent === 'flux : ouvert'");
    n.evaluer("window.__pasRecharge = true");
    host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Dis bonjour par echo.", &Value::Null).expect("la request");
    let carte = "document.querySelector('#attente-equipier')";
    n.attendre(&format!("({carte}?.textContent ?? '').includes('echo accorde-moi') && !!{carte}?.querySelector('button.accorder')"), "the page shows the held command as a card, by the stream");
    assert!(n.evaluer("[...document.querySelectorAll('ul.boite li')].some((l) => l.dataset.kind === 'approval')").as_bool().unwrap_or(false), "the mailbox shows the approval line");
    n.evaluer(&format!("{carte}.querySelector('button.accorder').click()"));
    n.attendre(
        "[...document.querySelectorAll('ul.boite li')].some((l) => l.dataset.kind === 'step_done' && l.textContent.includes('echo joué après'))",
        "the command ran after the page's « oui », and the mailbox shows it",
    );
    assert_eq!(n.evaluer(&format!("!!{carte}")), json!(false), "the card goes once answered");
    assert_eq!(n.evaluer("window.__pasRecharge === true"), json!(true), "followed without reloading the page");
    let boite = host.teammate_files(&h.id, "doc").unwrap().mailbox();
    assert!(boite.iter().any(|l| l["kind"] == "answer" && l["approved"] == true), "the page's answer is an approval in the mailbox : {boite:#?}");
    assert!(n.erreurs.is_empty(), "console : {:?}", n.erreurs);
    drop(n);
    let _ = crate::daemon::agent_with_timeout(Duration::from_secs(5))
        .post(format!("{racine}/arret"))
        .header("Authorization", "Bearer j")
        .header("content-type", "application/json")
        .send("{}");
    let _ = fil.join();
}

/// **One sentence per parameter** (Lucie, 20 Nov 2026): every property of every tool the teammate's
/// model receives has a description: what it does, its default, an example when the value has a
/// shape. A property without one is named.
#[test]
fn every_parameter_of_the_teammates_tools_is_described() {
    fn sans(prefixe: &str, schema: &Value, out: &mut Vec<String>) {
        if let Some(props) = schema["properties"].as_object() {
            for (nom, p) in props {
                let chemin = format!("{prefixe}.{nom}");
                if p["description"].as_str().map_or(true, |d| d.trim().is_empty()) {
                    out.push(chemin.clone());
                }
                sans(&chemin, p, out);
                sans(&format!("{chemin}[]"), &p["items"], out);
            }
        }
    }
    let mut defs = super::teammate::Step::tool_defs();
    defs.push(super::teammate::Step::fetch_def());
    defs.push(super::teammate::Step::send_to_session_def());
    let mut manquent = Vec::new();
    for d in &defs {
        sans(&d.name, &d.parameters, &mut manquent);
    }
    assert!(manquent.is_empty(), "parameters without a description: {manquent:#?}");
}

/// **A guest host resumes no teammate of the host it is the guest of** (the night of 20 Nov 2026: a
/// host started by hand by --address next to the service host resumed the atelier's teammate of
/// Lucie). Two hosts on the same tasks root: the owner's teammate waits for its request; the guest
/// says it leaves it, and starts nothing.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_guest_host_resumes_no_teammate_of_its_owner() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("no model here".into()) });
    let proprietaire = hote(d.path(), model.clone()).with_runs_root(d.path().join("runs"));
    let h = proprietaire.hosted(&manifeste).unwrap();
    proprietaire.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    proprietaire.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let files = proprietaire.teammate_files(&h.id, "doc").unwrap();
    files.append("supervisor", "request", None, "Fais-le.").unwrap();
    // The guest, on the same tasks root, rereads it (as the night's host did), then resumes.
    let invite = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs-invite")).en_invite("127.0.0.1:7970 (pid 2800109)".into()));
    invite.relire_les_taches();
    let lignes = invite.reprendre_les_equipiers();
    assert!(lignes.iter().any(|l| l.contains("127.0.0.1:7970 (pid 2800109)")), "the guest says whose teammates it leaves: {lignes:?}");
    assert!(!invite.teammate_running(&h.id, "doc"), "and starts none");
    assert!(!files.mailbox().iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.contains("repris") || t.contains("resum"))), "{:#?}", files.mailbox());
}

/// **A step whose reply is cut goes on** (lucivy 03, 19 Nov 2026): the model's first reply is cut at
/// max_tokens without a tool call; the step relaunches it (teammate.cut_relaunches_per_step, 2 by
/// default) instead of ending, and the step that follows the relaunch succeeds as one step.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_step_whose_reply_is_cut_goes_on_instead_of_ending() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let appels = Arc::new(Mutex::new(0usize));
    let model: ChatModel = {
        let appels = appels.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let appels = appels.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let n = { let mut a = appels.lock().unwrap(); *a += 1; *a };
                if n == 1 {
                    let usage = crate::llm::Usage { completion_tokens: 4092, reasoning_tokens: 3930, ..Default::default() };
                    return Ok((crate::llm::Finish::max_tokens(), usage));
                }
                answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait après la relance", "remaining": "" }))]))
            })))
        })
    };
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    files.append_with("supervisor", "request", None, "Fais-le.", &json!({ "expects": "none" })).unwrap();
    let o = host.run_teammate(&h.id, "doc").unwrap();
    let boite = files.mailbox();
    assert_eq!(o.steps.len(), 1, "the cut reply was relaunched within the step: {o:?} — {boite:#?}");
    assert!(!boite.iter().any(|l| l["text"].as_str().is_some_and(|t| t.contains("without step_done"))), "{boite:#?}");
}

/// **The gate of a child's views**: outside the set, refused by name; without a view, the active one;
/// a tool that reads no view is left as it is.
#[test]
fn a_call_outside_a_childs_views_is_refused_and_one_without_takes_the_active() {
    let vues = vec!["backend".to_string(), "refs/heads/master".to_string()];
    let e = super::chat::juger_la_vue(&json!({ "query": "x", "view": "worktree:/autre" }), &vues, "backend", true).unwrap_err();
    assert!(e.contains("worktree:/autre") && e.contains("backend") && e.contains("refs/heads/master"), "{e}");
    let a = super::chat::juger_la_vue(&json!({ "query": "x" }), &vues, "refs/heads/master", true).unwrap();
    assert_eq!(a["view"], "refs/heads/master", "without a view, the active one");
    let a = super::chat::juger_la_vue(&json!({ "query": "x", "view": "backend" }), &vues, "refs/heads/master", true).unwrap();
    assert_eq!(a["view"], "backend", "within the set, as asked");
    let a = super::chat::juger_la_vue(&json!({ "text": "x" }), &vues, "backend", false).unwrap();
    assert_eq!(a, json!({ "text": "x" }), "a tool without view is left as it is");
}

/// **A child is named after its session, gets its views from the parent, and only the parent changes
/// them** (page 20-11-2026/01 of rag3weaver-archi).
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_child_gets_its_views_from_the_parent_who_alone_changes_them() {
    assert_eq!(super::taches::nom_d_enfant("archi", "fouille"), "archi-fouille");
    assert_eq!(super::taches::nom_d_enfant("archi", "archi-fouille"), "archi-fouille", "already named");
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("no model here".into()) });
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let t = host.ouvrir_une_tache(&h.id, "archi-fouille").expect("la tâche s'ouvre");
    let vues = vec!["backend".to_string(), "refs/heads/master".to_string()];
    assert!(host.donner_les_vues(&h.id, "archi-fouille", "archi", vues.clone(), Some("ailleurs".into())).is_err(), "an active view outside the set is refused");
    host.donner_les_vues(&h.id, "archi-fouille", "archi", vues.clone(), Some("backend".into())).unwrap();
    assert_eq!(host.vues_de(&t.id), Some((vues.clone(), "backend".to_string())));
    // The parent adds, switches; an unknown active is refused.
    host.changer_les_vues(&h.id, "archi-fouille", &["worktree:/x".to_string()], &[], Some("worktree:/x")).unwrap();
    assert_eq!(host.vues_de(&t.id).unwrap().1, "worktree:/x");
    assert_eq!(host.vues_de(&t.id).unwrap().0.len(), 3);
    assert!(host.changer_les_vues(&h.id, "archi-fouille", &[], &[], Some("inconnue")).is_err());
    // Removing the active one falls back on the first that remains, and the child reads it.
    host.changer_les_vues(&h.id, "archi-fouille", &[], &["worktree:/x".to_string()], None).unwrap();
    assert_eq!(host.vues_de(&t.id), Some((vues.clone(), "backend".to_string())));
    let files = host.teammate_files(&h.id, "archi-fouille").unwrap();
    assert!(files.mailbox().iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.contains("worktree:/x") && t.contains("backend"))), "{:#?}", files.mailbox());
    // The set is never emptied.
    assert!(host.changer_les_vues(&h.id, "archi-fouille", &[], &vues, None).is_err());
}

/// A local HTTP server for the witnesses of `fetch`: `/page` (HTML, a script and visible text), `/missing` (404),
/// `/moved` (302 to `/page`). Returns its base address.
fn web_server() -> String {
    use std::io::{Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    std::thread::spawn(move || {
        for c in l.incoming().flatten() {
            let mut c = c;
            let mut buf = [0u8; 2048];
            let n = c.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
            let (status, extra, body) = match path.as_str() {
                "/page" => ("200 OK", "Content-Type: text/html; charset=utf-8\r\n".to_string(),
                    format!("<html><head><title>T</title><script>var secretScript = 1;</script><style>p {{}}</style></head><body><h1>Visible heading</h1><p>Visible text &amp; more.</p><p>{}</p></body></html>", "padding ".repeat(80))),
                "/moved" => ("302 Found", "Location: /page\r\n".to_string(), String::new()),
                _ => ("404 Not Found", "Content-Type: text/plain\r\n".to_string(), "nothing here".to_string()),
            };
            let _ = write!(c, "HTTP/1.1 {status}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        }
    });
    base
}

/// **fetch** (the teammate's base, 20 November 2026): a page comes back as readable text (no script, no style),
/// cut at its bound with the cut said; a 404 is said, not hidden; a redirect is followed and reported; a local
/// address is refused by default, naming `teammate.fetch.allow_local`.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_teammate_fetches_a_page_says_its_status_and_refuses_a_local_address_by_default() {
    let base = web_server();
    for allow_local in [false, true] {
        let d = tempfile::tempdir().unwrap();
        let (_depot, manifeste) = depot_inscrit(d.path(), true);
        let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
        // reply_wait_s: once the pages are read, the teammate waits for the supervisor; the witness does not.
        m["teammate"] = json!({ "fetch": { "allow_local": allow_local, "max_bytes": 300 }, "reply_wait_s": 2 });
        std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
        let declarations = manifeste.parent().unwrap();
        for args in [&["add", "-A"][..], &["commit", "-q", "-m", "fetch"]] {
            assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
        }
        let loaded = crate::backend::PreparedBackend::load(&manifeste).map(|p| p.manifest.teammate.clone().unwrap_or_default().fetch.allow_local);
        assert_eq!(loaded, Ok(allow_local), "the manifest loads with its teammate.fetch");
        let results = Arc::new(Mutex::new(Vec::<Value>::new()));
        let model: ChatModel = {
            let (base, results) = (base.clone(), results.clone());
            Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
                let (base, results) = (base.clone(), results.clone());
                Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                    let got = tool_results(turns);
                    if got.is_empty() {
                        return answer(turns, opts, sink, "", calls(&[
                            ("fetch", json!({ "url": format!("{base}/page") })),
                            ("fetch", json!({ "url": format!("{base}/missing") })),
                            ("fetch", json!({ "url": format!("{base}/moved") })),
                        ]));
                    }
                    if results.lock().unwrap().is_empty() {
                        *results.lock().unwrap() = got.iter().map(|r| serde_json::from_str(r).unwrap_or(json!(r))).collect();
                    }
                    answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "lu", "remaining": "" }))]))
                })))
            })
        };
        let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
        host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
        let files = host.teammate_files(&h.id, "doc").unwrap();
        files.append("supervisor", "request", None, "Lis ces pages.").unwrap();
        let outcome = host.run_teammate(&h.id, "doc");
        let r = results.lock().unwrap().clone();
        assert_eq!(r.len(), 3, "allow_local={allow_local}: {r:?}\noutcome: {outcome:?}\nmailbox: {:?}", files.mailbox());
        if !allow_local {
            for x in &r {
                assert_eq!(x["ok"], json!(false), "{x}");
                let e = x["error"].as_str().unwrap_or_default();
                assert!(e.contains("local") && e.contains("teammate.fetch.allow_local"), "a local address is refused by default: {x}");
            }
            continue;
        }
        let page = &r[0];
        assert_eq!((page["ok"].clone(), page["status"].clone()), (json!(true), json!(200)), "{page}");
        let text = page["text"].as_str().unwrap_or_default();
        assert!(text.contains("Visible heading") && text.contains("Visible text & more."), "readable text: {page}");
        assert!(!text.contains("secretScript") && !text.contains("<p>"), "no script, no markup: {page}");
        assert!(page["cut_bytes"].as_u64().is_some_and(|n| n > 0), "cut at max_bytes, and said: {page}");
        let missing = &r[1];
        assert_eq!((missing["ok"].clone(), missing["status"].clone()), (json!(false), json!(404)), "a 404 is said: {missing}");
        let moved = &r[2];
        assert_eq!(moved["status"], json!(200), "{moved}");
        assert!(moved["url"].as_str().is_some_and(|u| u.ends_with("/page")), "{moved}");
        assert_eq!(moved["redirects"].as_array().map(Vec::len), Some(1), "the redirect is reported: {moved}");
    }
}

/// A local server that plays Vertex's generateContent with Google Search grounding: three sources, two supports.
/// It keeps the request bodies it receives.
fn grounding_server(bodies: Arc<Mutex<Vec<Value>>>) -> String {
    use std::io::{BufRead, Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    std::thread::spawn(move || {
        for c in l.incoming().flatten() {
            let mut r = std::io::BufReader::new(c.try_clone().unwrap());
            let mut len = 0usize;
            loop {
                let mut line = String::new();
                if r.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0u8; len];
            let _ = r.read_exact(&mut body);
            bodies.lock().unwrap().push(serde_json::from_slice(&body).unwrap_or(Value::Null));
            let answer = json!({ "candidates": [{ "content": { "role": "model", "parts": [{ "text": "Rust 1.98 stabilised the new trait solver. It shipped on 13 November." }] },
                "finishReason": "STOP",
                "groundingMetadata": {
                    "webSearchQueries": ["rust 1.98 release"],
                    "groundingChunks": [
                        { "web": { "uri": "https://blog.rust-lang.org/2026/11/13/Rust-1.98.0.html", "title": "blog.rust-lang.org" } },
                        { "web": { "uri": "https://releases.rs/docs/1.98.0/", "title": "releases.rs" } },
                        { "web": { "uri": "https://news.example.org/rust", "title": "news.example.org" } }
                    ],
                    "groundingSupports": [
                        { "segment": { "startIndex": 0, "endIndex": 42, "text": "Rust 1.98 stabilised the new trait solver." }, "groundingChunkIndices": [0, 1], "confidenceScores": [0.9, 0.7] },
                        { "segment": { "startIndex": 43, "endIndex": 69, "text": "It shipped on 13 November." }, "groundingChunkIndices": [0], "confidenceScores": [0.8] }
                    ]
                } }] });
            let text = answer.to_string();
            let mut c = c;
            let _ = write!(c, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len());
        }
    });
    base
}

/// **web_search** (the teammate's base, 20 November 2026): a query answered by Gemini's Google Search grounding on
/// Vertex — the answer, the queries run, and the sources with title, address, the passage they support and their
/// provenance, bounded by teammate.web_search.max_sources and the cut said; a model that is not Gemini is refused,
/// with the reason.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_teammate_searches_the_web_through_gemini_grounding_and_a_model_that_is_not_gemini_is_refused() {
    let bodies = Arc::new(Mutex::new(Vec::<Value>::new()));
    let base = grounding_server(bodies.clone());
    for model in ["google/gemini-3.8-flash", "anthropic/claude-sonnet-5-5"] {
        let d = tempfile::tempdir().unwrap();
        let (_depot, manifeste) = depot_inscrit(d.path(), true);
        let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
        m["teammate"] = json!({ "web_search": { "max_sources": 2, "model": model, "endpoint": base }, "reply_wait_s": 2 });
        std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
        let declarations = manifeste.parent().unwrap();
        for args in [&["add", "-A"][..], &["commit", "-q", "-m", "web_search"]] {
            assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
        }
        let results = Arc::new(Mutex::new(Vec::<Value>::new()));
        let chat: ChatModel = {
            let results = results.clone();
            Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
                let results = results.clone();
                Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                    let got = tool_results(turns);
                    if got.is_empty() {
                        return answer(turns, opts, sink, "", calls(&[("web_search", json!({ "query": "what did Rust 1.98 stabilise?" }))]));
                    }
                    if results.lock().unwrap().is_empty() {
                        *results.lock().unwrap() = got.iter().map(|r| serde_json::from_str(r).unwrap_or(json!(r))).collect();
                    }
                    answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "cherché", "remaining": "" }))]))
                })))
            })
        };
        let host = hote(d.path(), chat).with_runs_root(d.path().join("runs"));
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
        host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
        let files = host.teammate_files(&h.id, "doc").unwrap();
        files.append("supervisor", "request", None, "Cherche ce que Rust 1.98 a stabilisé.").unwrap();
        let outcome = host.run_teammate(&h.id, "doc");
        let r = results.lock().unwrap().clone();
        assert_eq!(r.len(), 1, "{model}: {r:?}\noutcome: {outcome:?}");
        let x = &r[0];
        if model.starts_with("anthropic/") {
            assert_eq!(x["ok"], json!(false), "{x}");
            let e = x["error"].as_str().unwrap_or_default();
            assert!(e.contains("Gemini") && e.contains(model) && e.contains("teammate.web_search.model"), "refused with the reason: {x}");
            continue;
        }
        assert_eq!(x["ok"], json!(true), "{x}");
        assert!(x["answer"].as_str().is_some_and(|a| a.contains("trait solver")), "{x}");
        assert_eq!(x["queries"], json!(["rust 1.98 release"]), "{x}");
        let sources = x["sources"].as_array().cloned().unwrap_or_default();
        assert_eq!(sources.len(), 2, "bounded by max_sources: {x}");
        assert_eq!((x["total_sources"].clone(), x["cut_sources"].clone()), (json!(3), json!(1)), "the cut is said: {x}");
        assert_eq!(sources[0]["title"], "blog.rust-lang.org");
        assert_eq!(sources[0]["url"], "https://blog.rust-lang.org/2026/11/13/Rust-1.98.0.html");
        let snippet = sources[0]["snippet"].as_str().unwrap_or_default();
        assert!(snippet.contains("trait solver") && snippet.contains("13 November"), "the passages it supports: {x}");
        assert_eq!(sources[0]["provenance"]["supports"], json!(2), "{x}");
        assert_eq!(sources[1]["provenance"]["supports"], json!(1), "{x}");
        let sent = bodies.lock().unwrap().last().cloned().unwrap_or_default();
        assert_eq!(sent["tools"], json!([{ "googleSearch": {} }]), "grounding asked: {sent}");
        assert!(sent["contents"].to_string().contains("what did Rust 1.98 stabilise?"), "{sent}");
    }
    assert_eq!(bodies.lock().unwrap().len(), 1, "a model that is not Gemini sends nothing");
}
