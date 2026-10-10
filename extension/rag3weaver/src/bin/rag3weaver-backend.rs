//! Persistent backend host. Transport adapters use the same manifest-generated tools.
use rag3weaver::{backend::PreparedBackend, Rag3dbConnection};
use serde_json::{json, Value};
use std::{
    io::{self, BufRead, Write},
    path::Path,
};
fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: rag3weaver-backend backend.json [--describe]")?;
    let option = args.next();
    if option.as_deref().is_some_and(|v| v != "--describe") || args.next().is_some() {
        return Err("usage: rag3weaver-backend backend.json [--describe]".into());
    }
    let prepared = PreparedBackend::load(Path::new(&path))?;
    if option.as_deref() == Some("--describe") {
        println!("{}", prepared.describe());
        return Ok(());
    }
    let database = prepared.path(&prepared.manifest.database);
    if let Some(parent) = database.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Un backend en mots seuls n'exige pas de service d'embarquement ; un
    // backend qui déclare du vecteur démarre sans lui en le disant (plein
    // texte seul, vecteurs en dette — `PreparedBackend::open` redit la phrase
    // dans le reçu). Une identité de modèle différente reste un refus.
    let embedder = if prepared.needs_embeddings() {
        match prepared.connect_embedder() {
            Ok((embedder, _)) => Some(embedder),
            Err(raison) => {
                eprintln!("[rag3weaver] pas de service d'embarquement : index en plein texte seul, vecteurs en dette ({raison})");
                None
            }
        }
    } else {
        None
    };
    let tampon = rag3weaver::rag3db_connection::buffer_pool_choice(prepared.manifest.buffer_pool);
    eprintln!("[rag3weaver] tampon du moteur : {}", rag3weaver::rag3db_connection::describe_buffer_pool(tampon));
    let conn = Rag3dbConnection::with_manifest_buffer_pool(&database, prepared.manifest.buffer_pool).map_err(|e| e.to_string())?;
    // Crochet de test (`scripts/test_backend_must_reopen.py`) : le k-ième
    // appel d'outil rencontre la base comme après un point de reprise échoué.
    let hook = conn.failed_checkpoint_hook();
    let fail_at_call: Option<usize> = std::env::var("RAG3WEAVER_TEST_FAILED_CHECKPOINT_AT_CALL")
        .ok()
        .and_then(|v| v.parse().ok());
    let mut calls = 0usize;
    let mut backend = prepared.open(Box::new(conn), embedder)?;
    for line in io::stdin().lock().lines() {
        let request: Result<Value, String> = line.map_err(|e| e.to_string())
            .and_then(|line| serde_json::from_str(&line).map_err(|e| e.to_string()));
        if request.as_ref().ok().is_some_and(|v| v["op"] == "shutdown") {
            if let Some(reason) = backend.must_reopen() {
                println!("{}", json!({"ok":false,"error":format!("la base doit être rouverte : {reason}"),"mustReopen":true}));
                io::stdout().flush().map_err(|e| e.to_string())?;
                drop(backend);
                std::process::exit(rag3weaver::connection::EXIT_MUST_REOPEN);
            }
            backend.shutdown()?;
            drop(backend); // Database checkpoint/destruction must finish before acknowledgement.
            println!("{}", json!({"ok":true,"result":{"closed":true}}));
            io::stdout().flush().map_err(|e| e.to_string())?;
            return Ok(());
        }
        let result = request.and_then(|v| -> Result<Value, String> {
            match v["op"].as_str() {
                Some("describe") => Ok(backend.describe()),
                Some("call") => {
                    calls += 1;
                    if fail_at_call == Some(calls) {
                        hook.after(0);
                    }
                    backend.call(
                        v["name"].as_str().ok_or("tool name missing")?,
                        v.get("arguments").cloned().unwrap_or(json!({})),
                    )
                }
                // Réservées à l'hôte : jamais exposées comme outils.
                Some("journal") => backend.journal(v["events"].as_array().ok_or("events missing")?),
                Some("index_state") => backend.index_states(),
                Some("journal_read") => backend.journal_read(
                    v["conversation"].as_str().ok_or("conversation missing")?,
                    v["since_ms"].as_i64().unwrap_or(0),
                ),
                _ => Err("op must be describe, call, journal, journal_read or index_state".into()),
            }
        });
        // **Un point de reprise a échoué** : la réponse le dit, puis l'hôte
        // s'arrête avec `EXIT_MUST_REOPEN` pour être relancé sur une base
        // rouverte. Pas de `shutdown` : il demanderait un point de reprise
        // que le moteur refuse.
        let must_reopen = backend.must_reopen();
        let response = match (result, &must_reopen) {
            (_, Some(reason)) => json!({"ok":false,"error":format!("la base doit être rouverte : {reason}"),"mustReopen":true}),
            (Ok(v), None) => json!({"ok":true,"result":v}),
            (Err(e), None) => json!({"ok":false,"error":e}),
        };
        println!("{response}");
        io::stdout().flush().map_err(|e| e.to_string())?;
        if let Some(reason) = must_reopen {
            eprintln!("rag3weaver-backend : la base doit être rouverte — arrêt pour relance ({reason})");
            drop(backend);
            std::process::exit(rag3weaver::connection::EXIT_MUST_REOPEN);
        }
    }
    backend.shutdown()
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
