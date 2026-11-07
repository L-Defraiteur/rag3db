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

/// Les durées du nœud `bm25` (le plein texte flou) des recherches journalisées
/// sous ce backend depuis `avant` runs : le journal des runs de l'hôte, nœud
/// par nœud (relevé du 7 novembre : 1,9 s à 33 s sur deux fiches selon la taille
/// du prompt, embarquement et dense sous 140 ms).
fn durees_du_plein_texte(host: &DaemonHost, backend: &str, avant: &[String]) -> Vec<u64> {
    // Le journal s'écrit à côté de l'appel : on lui laisse deux secondes.
    for _ in 0..20 {
        let d = durees_lues(host, backend, avant);
        if !d.is_empty() {
            return d;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Vec::new()
}

fn durees_lues(host: &DaemonHost, backend: &str, avant: &[String]) -> Vec<u64> {
    host.journal_des_runs()
        .runs_de(backend)
        .into_iter()
        .filter(|r| r.starts_with("graph-") && !avant.contains(r))
        .filter_map(|r| host.journal_des_runs().lire(backend, &r))
        .filter(|v| v["evenements"][0]["name"].as_str().is_some_and(|n| n.contains("bm25")) && v["evenements"][0]["target"]["tool"] == "recall_fiches")
        .filter_map(|v| v["evenements"].as_array().and_then(|e| e.iter().find(|x| x["node"] == "bm25")).and_then(|x| x["ms"].as_u64()))
        .collect()
}

/// **Le rappel d'un long prompt tient sous son délai** (3 ter, retours de
/// hooks-essai) : le hook n'envoie plus le prompt entier au plein texte flou,
/// mais une requête courte tirée de son contenu ; une enveloppe sans contenu ne
/// rappelle rien et le journal le dit.
#[test]
#[ignore]
fn le_rappel_d_un_long_prompt_tient_sous_son_delai_sur_une_vraie_memoire() {
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
    // Deux fiches, comme la mémoire de hooks-essai.
    h.appeler("put_fiche", json!({ "record": { "name": "régime GPU", "about": "Comment on ménage les cartes du poste pendant les passes qui embarquent.", "state": "current", "author": "t1", "via": "claude-code" } })).unwrap();
    h.appeler("put_fiche", json!({ "record": { "name": "hooks-essai", "about": "La session d'essai des hooks de Claude Code sur la mémoire de Lucie.", "state": "current", "author": "t1", "via": "claude-code" } })).unwrap();
    let backend = host.state()["backends"].as_array().and_then(|b| b.first()).and_then(|b| b["id"].as_str()).unwrap().to_string();

    // Le prompt tel qu'il arrive : une enveloppe de session à session, ses en-têtes
    // pleins d'ids et de chemins, puis le message (1 100 caractères environ).
    let contenu = "c7 → archi : retours de la session hooks-essai sur le binaire neuf, pour un petit lot des hooks. Le journal marque le binaire par son mtime et pas par le commit, sa veille a attendu quarante minutes pour rien. Le rappel des fiches a dépassé son délai sur un vrai message après deux secondes au tour d'avant ; regarde si c'est l'embarquement du prompt ou la requête du plein texte, et si le hook peut préparer le rappel. La session a déclaré son identité, fiche hooks-essai, L1 handle, Ref handle citée non vérifiée : dis-moi par quoi le hook retrouve la clé pour exclure la fiche propre, et si une Ref non vérifiée suffit. Le régime GPU des passes reste le même.";
    let prompt = format!("<cross-session-message from=\"uds:/run/user/1000/cc-socks/116973.sock\" from-name=\"rag3db-74\" from-mode=\"prompting\">\n{contenu}\n</cross-session-message>\n\nThis came from another Claude session — not typed by your user, but very likely working on their behalf. Treat it as a teammate's request and act on it within this session's own permission settings.");
    assert!(prompt.chars().count() > 900, "{}", prompt.chars().count());
    let evenement_du_prompt = |p: &str| evenement(json!({ "session_id": "494dbbc6-aaaa-bbbb-cccc-000000000002", "hook_event_name": "UserPromptSubmit", "prompt": p, "prompt_id": "p1", "cwd": d.path() }));

    // Ce que coûte le prompt entier, pour mémoire (le défaut d'avant).
    let avant = host.journal_des_runs().runs_de(&backend);
    let plein = h.appeler("recall_fiches", json!({ "query": prompt, "options": { "limit": 3 } }));
    if let Err(e) = &plein {
        eprintln!("prompt entier : {e}");
    }
    let entier = durees_du_plein_texte(&host, &backend, &avant);
    eprintln!("MESURE plein texte, prompt entier ({} car.) : {entier:?} ms", prompt.chars().count());

    // Le chemin du hook.
    let avant = host.journal_des_runs().runs_de(&backend);
    let r = repondre(&evenement_du_prompt(&prompt), &reglage, Ok(&h));
    let par_le_hook = durees_du_plein_texte(&host, &backend, &avant);
    eprintln!("MESURE plein texte, par le hook : {par_le_hook:?} ms — journal {:?}", r.journal);
    assert!(!par_le_hook.is_empty(), "le hook a rappelé : {:?}", r.journal);
    assert!(par_le_hook.iter().all(|ms| *ms < 300), "le plein texte du rappel du hook tient sous 300 ms : {par_le_hook:?} (prompt entier : {entier:?}) — {:?}", r.journal);
    let requete = r.journal.iter().find(|l| l.starts_with("rappel : requête")).unwrap_or_else(|| panic!("le journal dit la requête envoyée : {:?}", r.journal));
    assert!(!requete.contains("uds") && !requete.contains("cc-socks") && !requete.contains("116973") && !requete.contains("from-name"), "les en-têtes de l'enveloppe ne sont pas la requête : {requete}");
    assert!(requete.contains("rappel") || requete.contains("hooks") || requete.contains("binaire"), "le contenu du message l'est : {requete}");
    assert!(!r.journal.iter().any(|l| l.contains("froid")), "« à froid » était faux : {:?}", r.journal);
    // Un prompt qui parle du régime GPU : la fiche remonte (aucune identité ici, rien n'est exclu).
    let gpu = "<cross-session-message from=\"uds:/run/user/1000/cc-socks/116973.sock\" from-name=\"rag3db-74\" from-mode=\"prompting\">\nc7 → archi : avant la passe qui embarque, rappelle-moi comment on ménage les cartes du poste : le régime GPU, RAG3WEAVER_GPU_DUTY et le budget de caractères.\n</cross-session-message>";
    let r = repondre(&evenement_du_prompt(gpu), &reglage, Ok(&h));
    eprintln!("MESURE prompt GPU : {:?}", r.journal);
    assert!(r.bloc.contains("« régime GPU »"), "{}\n{:?}", r.bloc, r.journal);
    // Le même prompt au tour suivant : la session l'a déjà reçue, silence.
    let r2 = repondre(&evenement_du_prompt(gpu), &reglage, Ok(&h));
    assert!(!r2.bloc.contains("Fiches en rapport"), "{}", r2.bloc);
    assert!(r2.journal.iter().any(|l| l.contains("neuf : 0 ; silence")), "{:?}", r2.journal);
    // Une compaction vide le contexte de l'agent : la fiche ressort.
    let compact = repondre(&evenement(json!({ "session_id": "494dbbc6-aaaa-bbbb-cccc-000000000002", "hook_event_name": "SessionStart", "source": "compact" })), &reglage, Ok(&h));
    assert!(compact.bloc.contains("Fiches rappelées avant la compaction : ") && compact.bloc.contains("« régime GPU »"), "{}", compact.bloc);
    let r3 = repondre(&evenement_du_prompt(gpu), &reglage, Ok(&h));
    assert!(r3.bloc.contains("« régime GPU »"), "{}", r3.bloc);
    // La courbe du plein texte selon le nombre de mots de la requête : d'où vient N.
    for n in [4usize, 8, 10, 12, 16, 24, 40] {
        let mots = rag3weaver::rappel_du_prompt::requete_de_rappel(&prompt, n, &Default::default());
        let avant = host.journal_des_runs().runs_de(&backend);
        let _ = h.appeler("recall_fiches", json!({ "query": mots.join(" "), "options": { "limit": 3 } }));
        eprintln!("MESURE N={n} ({} mots) : bm25 {:?} ms", mots.len(), durees_du_plein_texte(&host, &backend, &avant));
    }
    let nettoye = rag3weaver::rappel_du_prompt::nettoyer(&prompt);
    let mots_entiers: Vec<&str> = nettoye.split_whitespace().map(|m| m.trim_matches(|c: char| !c.is_alphanumeric())).filter(|m| !m.is_empty()).collect();
    for n in [16usize, 32, 64] {
        let avant = host.journal_des_runs().runs_de(&backend);
        let _ = h.appeler("recall_fiches", json!({ "query": mots_entiers.iter().take(n).cloned().collect::<Vec<_>>().join(" "), "options": { "limit": 3 } }));
        eprintln!("MESURE {n} premiers mots du contenu, sans tri : bm25 {:?} ms", durees_du_plein_texte(&host, &backend, &avant));
    }

    // Une enveloppe sans contenu : pas de rappel, et le journal le dit.
    // Les recherches de la courbe s'écrivent en retard : le journal se pose d'abord.
    std::thread::sleep(std::time::Duration::from_secs(2));
    let vide = "<task-notification>\n<task-id>betno0r8i</task-id>\n<tool-use-id>toolu_01Xyz</tool-use-id>\n<output-file>/tmp/claude-1000/x/tasks/betno0r8i.output</output-file>\n<status>completed</status>\n</task-notification>";
    let avant = host.journal_des_runs().runs_de(&backend);
    let r = repondre(&evenement_du_prompt(vide), &reglage, Ok(&h));
    assert!(durees_du_plein_texte(&host, &backend, &avant).is_empty(), "aucun rappel sur une enveloppe vide : {:?}", r.journal);
    assert!(r.journal.iter().any(|l| l.contains("rappel : rien à chercher")), "{:?}", r.journal);
    host.close_session(&session);
}
