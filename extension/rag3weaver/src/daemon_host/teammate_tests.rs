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
    MockLlm::new(text).with_tool_calls(tool_calls).generate(turns, opts, sink)
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

