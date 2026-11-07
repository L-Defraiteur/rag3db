//! **L'équipier sur faux modèle** (page 07 du 25 octobre 2026) : la boucle à
//! pas, la boîte aux lettres, le budget, les plans et l'arrêt, sur un petit
//! dépôt git qui n'est pas rag3weaver.

use super::chat::ChatModel;
use super::chat_qui_code::{depot_inscrit, hote};
use super::teammate::{PlanState, Status, STEP_BUDGET};
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
                if r.contains("budget du pas") {
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
                // treize lectures d'un coup, le budget refuse la treizième.
                (2, 0) => {
                    let mut v: Vec<(String, String)> = vec![("edit_plan".into(), json!({ "item": { "id": "titre", "status": "todo", "alone": true, "reason": "le superviseur a répondu" } }).to_string())];
                    for _ in 0..STEP_BUDGET {
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
    // Le budget : la treizième lecture refusée, en nommant step_done.
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
                // Le pas d'après la réponse (son numéro compte aussi l'attente : la
                // boucle a tourné une fois à vide avant de se réveiller).
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
        let mut r = crate::daemon::agent().get(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    let post = |route: &str, corps: Value| -> Value {
        let mut r = crate::daemon::agent()
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
        files.append("supervisor", "request", None, "Une petite chose.").unwrap();
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
                            let mut x = crate::daemon::agent().get(format!("{r}{route}")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
                            serde_json::from_str(&x.body_mut().read_to_string().unwrap()).unwrap()
                        };
                        let tache = lire(format!("/ide/teammate?backend={id}&task=doc"))["result"]["backend"].as_str().unwrap().to_string();
                        let mut s = crate::daemon::agent().post(format!("{r}/session")).header("Authorization", "Bearer j").header("content-type", "application/json").send(json!({ "client": "le témoin" }).to_string()).unwrap();
                        let session: Value = serde_json::from_str(&s.body_mut().read_to_string().unwrap()).unwrap();
                        let mut x = crate::daemon::agent()
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
        let mut x = crate::daemon::agent().post(format!("{r}{route}")).header("Cookie", "rag3weaver_ide=j").header("content-type", "application/json").send(corps.to_string()).unwrap();
        serde_json::from_str(&x.body_mut().read_to_string().unwrap()).unwrap()
    };
    assert_eq!(post("/ide/describe", json!({ "manifest": manifeste }))["ok"], true);
    let v = post("/ide/teammate/mailbox", json!({ "backend": id, "task": "doc", "from": "supervisor", "kind": "request", "text": "Lis deux fichiers." }));
    assert_eq!(v["ok"], true, "{v}");
    let borne = Instant::now() + Duration::from_secs(60);
    loop {
        let mut x = crate::daemon::agent().get(format!("{r}/ide/teammate/list?supervisor=supervisor")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
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
            files.append("supervisor", "request", None, "Écris une ligne dans le README.").unwrap();
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
        let mut r = crate::daemon::agent().get(format!("http://{adresse}/ide/teammate?backend={id}&task={task}")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
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
    assert!(rapport["text"].as_str().unwrap().contains("sans step_done") && rapport["text"].as_str().unwrap().contains("Je n'ai pas d'outil d'écriture"), "{rapport}");
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
    let boite = attendre("la question de l'accord", &|b| b.iter().any(|l| l["kind"] == "approval"));
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
