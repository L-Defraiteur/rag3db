//! **Les hooks de Claude Code sur un vrai gabarit `memory`** (lot 3 bis, 7 novembre
//! 2026) : ce que les faux hôtes des témoins unitaires ne voient pas — ils
//! répondent à tout `cited_by`, et c'est ainsi que la valeur du handle est restée
//! fausse jusqu'à ce que 73 l'essaie sur la mémoire de Lucie. Ici, l'hôte est
//! réel et la mémoire aussi (une base temporaire du gabarit livré) ; le hook
//! parle à l'hôte en direct, par le même trait `Hote` que son client HTTP.
//!
//! Le fil : une fiche d'identité `t1` avec son L1 `handle` écrits comme la
//! consigne le dit à l'agent ; `SessionStart` la retrouve par la valeur nue
//! `session:<uuid>` ; `PostCompact` range le résumé en L0 et en L1 `compaction`
//! sur la fiche de tâche citée par l'`identity` ; `Stop` dépose un run
//! `claude_code_turn` sous le backend de la mémoire, relu au journal des runs.
//!
//! Sur le vrai gabarit `memory` (ses signaux : `RAG3WEAVER_EMBED_SERVICE`).
#![cfg(all(feature = "rag3db-native", feature = "daemon"))]
use rag3weaver::claude_code_hook::{repondre, Evenement, Hote, Reglage};
use rag3weaver::daemon_host::DaemonHost;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

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

/// L'hôte joint **en direct**, sans HTTP : le même trait que `ClientHote`.
struct HoteEnDirect<'a> {
    host: &'a DaemonHost,
    session: String,
    manifest: PathBuf,
}

impl Hote for HoteEnDirect<'_> {
    fn nom(&self) -> &str {
        "memory"
    }
    fn appeler(&self, outil: &str, arguments: Value) -> Result<Value, String> {
        let v = self.host.call(Some(&self.session), &self.manifest, outil, arguments);
        if v["ok"] == json!(false) {
            return Err(format!("{outil} : {}", v["error"].as_str().unwrap_or("?")));
        }
        Ok(v)
    }
    fn deposer_run(&self, corps: Value) -> Result<Value, String> {
        let (statut, v) = self.host.handle("POST", "/journal/run", Some(&self.session), corps.to_string().as_bytes());
        if statut != 200 {
            return Err(format!("journal/run : {statut} {v}"));
        }
        Ok(v)
    }
    fn manifest(&self) -> Option<&Path> {
        Some(&self.manifest)
    }
}

fn evenement(v: Value) -> Evenement {
    Evenement::lire(&v.to_string()).unwrap()
}

#[test]
#[ignore]
fn les_hooks_retrouvent_la_session_rangent_la_compaction_et_deposent_le_run_sur_une_vraie_memoire() {
    let d = tempfile::tempdir().unwrap();
    let gabarits = d.path().join("templates");
    copier(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits);
    let memoire = gabarits.join("backends/memory/backend.json");
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&memoire).unwrap()).unwrap();
    m["database"] = json!(d.path().join("memoire.rag3db").display().to_string());
    std::fs::write(&memoire, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let host = DaemonHost::new().with_runs_root(d.path().join("runs"));
    let session = host.open_session("claude-code-hook", None, Vec::new(), None);
    let h = HoteEnDirect { host: &host, session: session.clone(), manifest: memoire.clone() };
    let mut reglage = Reglage::defaut();
    reglage.journal = d.path().join("hooks.log");
    reglage.templates_dir = d.path().join("gabarits-hook");
    std::env::remove_var("RAG3WEAVER_AGENT_KEY");
    let uuid = "494dbbc6-aaaa-bbbb-cccc-000000000001";

    // 1. Une session inconnue : la demande du handle, rien d'écrit.
    let r = repondre(&evenement(json!({ "session_id": uuid, "hook_event_name": "SessionStart", "source": "startup" })), &reglage, Ok(&h));
    assert!(r.bloc.contains("Aucune fiche d'identité ne cite cette session"), "{}", r.bloc);
    assert!(r.bloc.contains(&format!("session:{uuid}")), "{}", r.bloc);
    let fiches = h.appeler("recall_fiches", json!({ "query": "t1", "options": { "limit": 5 } })).unwrap();
    assert!(fiches["result"]["result"].as_array().map(Vec::is_empty).unwrap_or(true), "rien n'est créé à la place de l'agent : {fiches}");

    // 2. L'agent fait ce que la consigne dit : sa fiche, son L1 handle, la Ref handle.
    h.appeler("put_fiche", json!({ "record": { "name": "t1", "about": "la session d'essai des hooks", "state": "current", "author": "t1", "via": "claude-code" } })).unwrap();
    let source = format!("sessions/{uuid}");
    h.appeler("put_apport", json!({ "record": { "fiche": "t1", "source": source, "section": "handle", "texte": format!("session:{uuid}"), "niveau": "L1", "state": "current", "via": "claude-code", "author": "t1" } })).unwrap();
    h.appeler("relier_apport", json!({ "links": [{ "from": { "name": "t1" }, "to": { "fiche": "t1", "source": source, "section": "handle" } }] })).unwrap();
    let _ = h.appeler("create_ref_type", json!({ "name": "handle", "description": "Une adresse d'un agent dans son harnais", "confirm": true }));
    h.appeler("citer_source", json!({ "genre": "handle", "valeur": format!("session:{uuid}"), "fiche": { "fiche": "t1", "source": source, "section": "handle" } })).unwrap();
    // Et son identity, sept lignes, qui cite la fiche de tâche.
    h.appeler("put_fiche", json!({ "record": { "name": "essai des hooks", "about": "la tâche de la session t1", "state": "current", "author": "t1", "via": "claude-code" } })).unwrap();
    let identity = "who: t1 — la session d'essai\nruntime: claude-code\ncan: les outils MCP\ndoing: l'essai des hooks (fiche:essai des hooks)\nholding: rien\nmandates: aucun\npeers: c7";
    h.appeler("put_apport", json!({ "record": { "fiche": "t1", "source": "identite", "section": "identity", "texte": identity, "niveau": "L1", "state": "current", "via": "claude-code", "author": "t1" } })).unwrap();
    h.appeler("relier_apport", json!({ "links": [{ "from": { "name": "t1" }, "to": { "fiche": "t1", "source": "identite", "section": "identity" } }] })).unwrap();

    // 3. SessionStart retrouve la session par la valeur nue du handle (le défaut de 73).
    let r = repondre(&evenement(json!({ "session_id": uuid, "hook_event_name": "SessionStart", "source": "resume" })), &reglage, Ok(&h));
    assert!(r.bloc.contains("Tu es **t1** — la session d'essai des hooks"), "la fiche par session:<uuid>, genre handle — {}\n{:?}", r.bloc, r.journal);
    assert!(r.bloc.contains("- En cours : l'essai des hooks"), "{}", r.bloc);
    assert!(r.bloc.contains("Tâche courante : fiche « essai des hooks », sans plan écrit"), "{}", r.bloc);

    // 4. PostCompact : le L0, le L1 compaction sur la fiche de tâche, ses deux Refs.
    let r = repondre(&evenement(json!({ "session_id": uuid, "hook_event_name": "PostCompact", "trigger": "manual", "compact_summary": "## Résumé\nla session t1 essayait les hooks." })), &reglage, Ok(&h));
    assert!(r.journal.iter().any(|l| l.contains("L1 compaction écrit sur « essai des hooks »")), "{:?}", r.journal);
    let apports = h.appeler("recall_apports", json!({ "query": "compaction", "options": { "limit": 5, "filter_condition": { "field": { "key": "fiche", "value": "essai des hooks" } } } })).unwrap();
    let lignes = apports["result"]["result"].as_array().cloned().unwrap_or_default();
    let trouve = lignes.iter().find(|x| x["data"]["section"] == "compaction").unwrap_or_else(|| panic!("le L1 compaction : {apports}"));
    // La recherche ne rend que les champs de retour d'Apport ; l'enregistrement
    // entier (via, author, texte) se lit par son identité.
    let source = trouve["data"]["source"].as_str().unwrap().to_string();
    assert!(source.starts_with("sources/") && source.ends_with(&format!("-compaction-{}.md", &uuid[..8])), "{source}");
    let entier = h.appeler("get_apport", json!({ "record": { "fiche": "essai des hooks", "source": source, "section": "compaction" } })).unwrap();
    let compaction = &entier["result"]["result"]["data"];
    assert_eq!((compaction["via"].as_str(), compaction["author"].as_str(), compaction["niveau"].as_str()), (Some("hook"), Some("t1"), Some("L1")), "{entier}");
    assert!(compaction["texte"].as_str().unwrap().contains("la session t1 essayait les hooks."));
    let cite = h.appeler("cited_by", json!({ "value": format!("session:{uuid}"), "genres": ["handle"], "under": false, "format": "json", "limit": 5 })).unwrap();
    let texte = cite["result"]["result"].as_str().unwrap_or("").to_string();
    assert!(texte.contains("\"essai des hooks\"") && texte.contains("\"t1\""), "la Ref handle de la compaction et celle de l'identité : {texte}");

    // 5. Stop : le run claude_code_turn sous le backend de la mémoire, relu au journal.
    let transcription = d.path().join(format!("{uuid}.jsonl"));
    std::fs::write(&transcription, concat!(
        r#"{"type":"user","promptId":"p1","message":{"role":"user","content":"essaie les hooks"}}"#, "\n",
        r#"{"type":"assistant","promptId":"p1","message":{"role":"assistant","usage":{"input_tokens":1,"cache_read_input_tokens":10},"content":[{"type":"text","text":"j'essaie"},{"type":"tool_use","name":"mcp__memoire-longue__get_fiche","id":"tu1","input":{"record":{"name":"t1"}}}]}}"#, "\n",
        r#"{"type":"user","promptId":"p1","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"tu1","content":"{\"name\":\"t1\"}","is_error":false}]}}"#, "\n",
        r#"{"type":"assistant","promptId":"p1","message":{"role":"assistant","content":[{"type":"text","text":"fait."}]}}"#, "\n",
    )).unwrap();
    let r = repondre(&evenement(json!({ "session_id": uuid, "hook_event_name": "Stop", "prompt_id": "p1", "transcript_path": transcription, "cwd": d.path(), "stop_hook_active": false, "last_assistant_message": "fait." })), &reglage, Ok(&h));
    assert!(r.journal.iter().any(|l| l.contains(&format!("run déposé : claude-code/{uuid}-p1"))), "{:?}", r.journal);
    let backend = host.state()["backends"].as_array().and_then(|b| b.first()).and_then(|b| b["id"].as_str()).unwrap().to_string();
    let run = host.journal_des_runs().lire(&backend, &format!("{uuid}-p1")).unwrap_or_else(|| panic!("le run sous le backend {backend} : {}", host.state()));
    assert_eq!((run["genre"].as_str(), run["agent"].as_str(), run["session"].as_str()), (Some("claude_code_turn"), Some("t1"), Some(format!("session:{uuid}").as_str())));
    assert_eq!(run["tours"].as_array().map(Vec::len), Some(4), "{run}");
    assert_eq!(run["tours"][1]["tool_calls"][0]["name"], "mcp__memoire-longue__get_fiche");
    assert_eq!(run["fin"]["kind"], "RunFinished");
    assert!(run["serveurs"].is_object(), "la table des serveurs, même vide : {run}");
    host.close_session(&session);
}
