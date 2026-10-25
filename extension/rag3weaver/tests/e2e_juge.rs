//! **Le juge de session, sur faux modèle** (page 04 du 25 octobre 2026) : la
//! tuyauterie entière. Un tour du chat d'un backend de code laisse son journal ;
//! le juge enquête par ses outils (lire le run, rejouer l'appel de lecture,
//! ouvrir le résultat, relire les bilans précédents), rend son verdict dans la
//! forme fixe, et la fiche de bilan est rangée dans une vraie mémoire : la Fiche
//! (`via: juge`), le L0 (le journal), le L1 « bilan » lisible avec son JSON. Un
//! ticket est proposé sur la description d'un outil. Une seconde session de la
//! même tâche retrouve la première, et s'y compare.
//!
//! Sur le vrai gabarit `memory` (ses signaux : `RAG3WEAVER_EMBED_SERVICE`).

use rag3weaver::daemon_host::DaemonHost;
use rag3weaver::llm::{CallbackLlm, Llm, MockLlm};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn copier(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        let cible = dst.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copier(&e.path(), &cible);
        } else {
            std::fs::copy(e.path(), &cible).unwrap();
        }
    }
}

/// Ce que le faux juge a vu, pour les assertions.
#[derive(Default)]
struct Vu {
    rejoue: Vec<String>,
    precedents: Vec<String>,
}

/// Un faux modèle qui sert le chat (lire `main.rs`, puis répondre) et le juge
/// (ses quatre outils dans l'ordre, puis le verdict dans la forme fixe).
fn faux(vu: Arc<Mutex<Vu>>) -> rag3weaver::daemon_host::chat::ChatModel {
    Arc::new(move |_b: &rag3weaver::backend::Backend| -> Result<Arc<dyn Llm>, String> {
        let vu = vu.clone();
        Ok(Arc::new(CallbackLlm::new("faux-modele", 8192, move |turns, opts, sink| {
            let systeme = turns.first().map(|t| t.content.clone()).unwrap_or_default();
            let outils_vus: Vec<&rag3weaver::llm::Turn> = turns.iter().filter(|t| t.role == "tool").collect();
            if !systeme.contains("session judge") {
                // Le chat : lire main.rs, puis répondre.
                if turns.last().is_some_and(|t| t.role == "tool") {
                    return MockLlm::new("C'est un main vide.").generate(turns, opts, sink);
                }
                return MockLlm::new("").with_tool_calls(vec![("read_file", r#"{"path":"main.rs"}"#)]).generate(turns, opts, sink);
            }
            if opts.response_format.is_some() {
                // Le dernier pas : le verdict. Il se compare au précédent s'il l'a vu.
                let deja = turns.iter().any(|t| t.role == "tool" && t.tool_name.as_deref() == Some("bilans_precedents") && t.content.contains("Note "));
                let verdict = json!({
                    "resume": "Le tour a lu main.rs et l'a résumé.",
                    "note": 8, "confiance": "haute", "objectif": "atteint",
                    "faits": [{ "texte": "read_file a rendu main.rs entier", "nature": "dit" }],
                    "outils": [{ "outil": "read_file", "appels": 1, "echecs": 0, "inutiles": 0,
                                 "ambigus": [{ "retour": "(End of file - 1 lines)", "cru": "le fichier est vide" }] }],
                    "frictions": [], "attentes": [], "manquait": [],
                    "a_corriger": [{ "outil": "read_file", "phrase": "(End of file - 1 lines)",
                                     "proposee": "Lit un fichier du projet, avec ses numéros de ligne ; la dernière ligne dit combien le fichier en a." }],
                    "comparaison": if deja { json!({ "verdict": "pareil", "sur": "la même lecture que la session précédente" }) }
                                   else { json!({ "verdict": "premier", "sur": "aucun bilan précédent" }) },
                });
                return MockLlm::new(verdict.to_string().as_str()).generate(turns, opts, sink);
            }
            let appel = |nom: &str, args: Value| MockLlm::new("").with_tool_calls(vec![(nom, args.to_string().as_str())]);
            match outils_vus.len() {
                0 => appel("lire_le_run", json!({})).generate(turns, opts, sink),
                1 => {
                    let lu: Value = serde_json::from_str(&outils_vus[0].content).unwrap_or(Value::Null);
                    let id = lu["evenements"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .find(|e| e["kind"] == "ToolCallStarted" && e["tool"] == "read_file")
                        .and_then(|e| e["call_id"].as_str())
                        .unwrap_or("absent")
                        .to_string();
                    appel("rejouer_un_outil", json!({ "call_id": id })).generate(turns, opts, sink)
                }
                2 => {
                    vu.lock().unwrap().rejoue.push(outils_vus[1].content.clone());
                    appel("ouvrir_le_resultat", json!({})).generate(turns, opts, sink)
                }
                3 => appel("bilans_precedents", json!({})).generate(turns, opts, sink),
                _ => {
                    vu.lock().unwrap().precedents.push(outils_vus[3].content.clone());
                    MockLlm::new("J'ai assez.").generate(turns, opts, sink)
                }
            }
        })))
    })
}

fn journal(host: &DaemonHost, backend: &str, run: &str) -> Value {
    let fin = Instant::now() + Duration::from_secs(10);
    loop {
        let lu = host
            .journal_des_runs()
            .fichier(backend, run)
            .and_then(|p| serde_json::from_slice::<Value>(&std::fs::read(p).ok()?).ok())
            .filter(|v| v["tours"].is_array());
        if let Some(v) = lu {
            return v;
        }
        assert!(Instant::now() < fin, "le journal du run {run} n'est jamais écrit");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Le banc : une mémoire, un backend de code qui déclare `bilans` vers elle,
/// un hôte sur le faux modèle ; rend l'hôte, la mémoire, l'id du backend.
fn banc(d: &Path, vu: Arc<Mutex<Vu>>) -> (DaemonHost, PathBuf, String) {
    let gabarits = d.join("templates");
    copier(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits);
    // La mémoire.
    let memoire = gabarits.join("backends/memory/backend.json");
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&memoire).unwrap()).unwrap();
    m["database"] = json!(d.join("memoire.rag3db").display().to_string());
    std::fs::write(&memoire, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    // Le backend de code jugé : un arbre, sa base, et `bilans` vers la mémoire.
    let arbre = d.join("code");
    std::fs::create_dir_all(&arbre).unwrap();
    std::fs::write(arbre.join("main.rs"), "fn main() {}\n").unwrap();
    let code = gabarits.join("backends/code/backend.json");
    let mut c: Value = serde_json::from_str(&std::fs::read_to_string(&code).unwrap()).unwrap();
    c["database"] = json!(d.join("code.rag3db").display().to_string());
    c["workspace"]["root"] = json!(arbre.display().to_string());
    c["bilans"] = json!({ "memoire": memoire.display().to_string() });
    std::fs::write(&code, serde_json::to_string_pretty(&c).unwrap()).unwrap();

    let host = DaemonHost::new()
        .with_runs_root(d.join("runs"))
        .with_chats_root(d.join("chats"))
        .with_racine_des_taches(d.join("taches"))
        .with_chat_model(faux(vu.clone()));
    // Hébergé par un premier appel ; son id, dans l'état de l'hôte.
    let s = host.open_session("le témoin", None, Vec::new(), None);
    let lu = host.call(Some(&s), &code, "read_file", json!({ "path": "main.rs" }));
    assert_eq!(lu["ok"], true, "le backend de code s'héberge : {lu}");
    let code_canon = code.canonicalize().unwrap();
    let id = host.state()["backends"]
        .as_array()
        .and_then(|b| b.iter().find(|b| b["manifest"].as_str().map(PathBuf::from).and_then(|p| p.canonicalize().ok()) == Some(code_canon.clone())))
        .unwrap_or_else(|| panic!("le backend de code n'est pas dans l'état : {}", host.state()))["id"]
        .as_str()
        .unwrap()
        .to_string();

    (host, memoire, id)
}

/// **Le juge, de bout en bout, sur faux modèle.**
#[test]
#[ignore]
fn le_juge_relit_un_tour_range_sa_fiche_et_compare_la_session_suivante() {
    let d = tempfile::tempdir().unwrap();
    let vu = Arc::new(Mutex::new(Vu::default()));
    let (host, memoire, id) = banc(d.path(), vu.clone());

    // Première session.
    let r1 = host.chat_turn(&id, "Lis main.rs et dis-moi ce qu'il fait.").expect("le tour");
    let run1 = r1["run"].as_str().unwrap().to_string();
    journal(&host, &id, &run1);
    let b1 = host.juger(&id, &run1).unwrap_or_else(|e| panic!("le juge : {e}"));
    eprintln!("bilan 1 : {b1}");
    assert_eq!(b1["fiche"], "bilan : lis main.rs et dis-moi ce qu'il fait", "{b1}");
    assert_eq!(b1["verdict"]["comparaison"]["verdict"], "premier", "{b1}");
    {
        let v = vu.lock().unwrap();
        assert!(v.rejoue.first().is_some_and(|r| r.contains("fn main()")), "le juge a rejoué read_file : {:?}", v.rejoue);
    }
    let ticket = &b1["tickets"][0];
    assert!(ticket["proposition"].is_u64(), "un ticket sur la description de read_file : {b1}");

    // La fiche dans la mémoire : la Fiche via juge, l'apport « bilan » avec son JSON.
    let appeler_memoire = |outil: &str, args: Value| -> Value {
        let s = host.open_session("le témoin", None, Vec::new(), None);
        host.call(Some(&s), &memoire, outil, args)
    };
    let f = appeler_memoire("get_fiche", json!({ "record": { "name": b1["fiche"] } }));
    assert!(f.to_string().contains("\"via\":\"juge\""), "la fiche, via juge : {f}");
    let apports = appeler_memoire("recall_apports", json!({ "query": "bilan", "options": { "limit": 5, "filter_condition": { "field": { "key": "fiche", "value": b1["fiche"] } } } }));
    let texte = apports.to_string();
    assert!(texte.contains("Note 8/10") && texte.contains("```json") && texte.contains("(End of file - 1 lines)"), "le L1 lisible, son JSON, le retour ambigu nommé : {apports}");

    // Seconde session, même tâche : le juge retrouve la première et s'y compare.
    let r2 = host.chat_turn(&id, "Lis main.rs et dis-moi ce qu'il fait.").expect("le second tour");
    let run2 = r2["run"].as_str().unwrap().to_string();
    journal(&host, &id, &run2);
    let b2 = host.juger(&id, &run2).unwrap_or_else(|e| panic!("le second juge : {e}"));
    assert_eq!(b2["fiche"], b1["fiche"], "la même fiche");
    assert!(vu.lock().unwrap().precedents.last().is_some_and(|p| p.contains("Note 8/10")), "le second juge a lu le premier bilan");
    assert_eq!(b2["verdict"]["comparaison"]["verdict"], "pareil", "{b2}");
}

/// **Un bilan qu'il faut demander n'existerait pas** (c7) : l'hôte servi juge
/// lui-même la fin d'un tour du backend qui déclare `bilans`, et la fiche
/// apparaît dans la mémoire sans qu'on appelle le juge.
#[test]
#[ignore]
fn le_juge_se_declenche_seul_a_la_fin_d_un_tour() {
    let d = tempfile::tempdir().unwrap();
    let (host, memoire, id) = banc(d.path(), Arc::new(Mutex::new(Vu::default())));
    let host = Arc::new(host);
    host.servir_les_bilans();
    host.chat_turn(&id, "Lis main.rs et dis-moi ce qu'il fait.").expect("le tour");
    let s = host.open_session("le témoin", None, Vec::new(), None);
    let fin = Instant::now() + Duration::from_secs(60);
    loop {
        let apports = host.call(Some(&s), &memoire, "recall_apports", json!({ "query": "bilan", "options": { "limit": 5, "filter_condition": { "field": { "key": "fiche", "value": "bilan : lis main.rs et dis-moi ce qu'il fait" } } } }));
        if apports.to_string().contains("Note 8/10") {
            break;
        }
        assert!(Instant::now() < fin, "aucun bilan en 60 s : le juge ne s'est pas déclenché seul ({apports})");
        std::thread::sleep(Duration::from_millis(200));
    }
}
