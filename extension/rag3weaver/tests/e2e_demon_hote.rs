//! **L'hôte du démon, marche 1 de la forme C**
//! (`docs/11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`, §7).
//!
//! Ce que la marche promet, éprouvé sur de vraies bases :
//!
//! 1. deux backends dans un processus, chacun sa base ;
//! 2. deux sessions sur un backend : il est chargé **une fois**, et sa
//!    réaction s'exécute une fois (le témoin qui condamne `--demon`,
//!    retourné) ;
//! 3. sans session, un refus nommé ;
//! 4. une base à rouvrir ne lâche que son backend : l'appel rend une erreur
//!    transitoire, le suivant rouvre, et l'autre backend ne voit rien ;
//! 5. le tout par le fil HTTP, jusqu'à `POST /arret`.
//!
//! Le manifeste est celui d'`e2e_reaction_montee` (sans signal, donc sans
//! service d'embarquement), avec un `get`.
#![cfg(all(feature = "rag3db-native", feature = "daemon"))]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use rag3weaver::backend::PreparedBackend;
use rag3weaver::daemon_host::{serve_daemon, DaemonHost, REOPENED, SESSION_HEADER};
use rag3weaver::{FailedCheckpointHook, Rag3dbConnection};

fn schema(etats: &[&str]) -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "additionalProperties": false,
        "required": ["titre", "corps", "etat", "revision", "created_at", "updated_at"],
        "properties": {
            "titre": {"type": "string", "minLength": 1},
            "corps": {"type": "string", "minLength": 1},
            "etat": {"type": "string", "enum": etats},
            "revision": {"type": "integer", "minimum": 1},
            "created_at": {"type": "integer"},
            "updated_at": {"type": "integer"}
        }
    })
}

fn entite(fichier: &str, cycle: Option<Value>) -> Value {
    let mut config = json!({
        "fields": {
            "titre": {"type": "string", "isTitle": true},
            "corps": {"type": "string", "isContent": true},
            "etat": {"type": "string"}
        },
        "hashsafe": ["titre"],
        "returnFields": ["titre", "corps", "etat", "revision"],
        "signals": []
    });
    if let Some(cycle) = cycle {
        config["lifecycle"] = cycle;
    }
    json!({
        "schema": fichier,
        "config": config,
        "writes": {"created_at": "created_at", "updated_at": "updated_at", "revision": "revision"}
    })
}

fn extension_vecteur() -> String {
    let racine = std::env::var("RAG3DB_ROOT")
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").display().to_string());
    format!("{racine}/extension/vector/build/libvector.rag3db_extension")
}

/// Un manifeste dans `dir` : fiches et sujets, `put`, `get`, l'ancre, et la
/// réaction `review_anchored`.
fn manifeste(dir: &Path) -> PathBuf {
    let gabarits = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let put = gabarits.join("backends/notebook/graphs/put.mmd");
    let get = gabarits.join("backends/notebook/graphs/get.mmd");
    let lien = gabarits.join("tools/link.mmd");
    let reaction = gabarits.join("backends/memory/graphs/review_anchored.mmd");
    std::fs::write(dir.join("fiche.json"), serde_json::to_vec_pretty(&schema(&["courant", "a_revoir"])).unwrap()).unwrap();
    std::fs::write(dir.join("sujet.json"), serde_json::to_vec_pretty(&schema(&["courant"])).unwrap()).unwrap();
    let cycle = json!({
        "field": "etat",
        "initial": "courant",
        "transitions": [{"name": "revoir", "from": "courant", "to": "a_revoir"}]
    });
    let manifest = json!({
        "version": 1,
        "name": "demon-hote",
        "database": "base.rag3db",
        "embeddings": {"address": "127.0.0.1:9", "model": "none", "dimensions": 8},
        "vector_extension": extension_vecteur(),
        "entities": {"Fiche": entite("fiche.json", Some(cycle)), "Sujet": entite("sujet.json", None)},
        "relations": {"ANCRE": {"from": "Fiche", "to": "Sujet"}},
        "tools": {
            "put_fiche": {"graph": put, "bindings": {"entity": "Fiche"}, "input_payloads": {"record": {"entity": "Fiche", "view": "editable"}}},
            "get_fiche": {"graph": get, "bindings": {"entity": "Fiche"}, "input_payloads": {"record": {"entity": "Fiche", "view": "identity"}}},
            "put_sujet": {"graph": put, "bindings": {"entity": "Sujet"}, "input_payloads": {"record": {"entity": "Sujet", "view": "editable"}}},
            "ancrer": {"graph": lien, "bindings": {"relation": "ANCRE"}}
        },
        "reactions": {
            "review_anchored": {
                "graph": reaction,
                "bindings": {"target": "Fiche", "relation": "ANCRE", "transition": "revoir", "source": "Sujet"}
            }
        }
    });
    let chemin = dir.join("backend.json");
    std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    chemin
}

/// Un dossier neuf sous le cache (pas /tmp, qui est de la mémoire vive).
fn dossier(cas: &str) -> PathBuf {
    let d = PathBuf::from(std::env::var("HOME").unwrap())
        .join(".cache/rag3weaver-build/demon-hote")
        .join(format!("{cas}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn ok(reponse: &Value) -> &Value {
    assert_eq!(reponse["ok"], json!(true), "{reponse}");
    &reponse["result"]
}

/// Une fiche par son titre : son état et sa révision, ou `None`.
fn fiche(host: &DaemonHost, session: &str, manifest: &Path, titre: &str) -> Option<(String, i64)> {
    let r = host.call(Some(session), manifest, "get_fiche", json!({"record": {"titre": titre}}));
    if r["ok"] != json!(true) {
        return None;
    }
    // La ligne est sous `result.data` ; absente, il n'y a pas de titre.
    let ligne = &r["result"]["result"]["data"];
    ligne.get("titre")?;
    Some((ligne["etat"].as_str().unwrap_or_default().to_string(), ligne["revision"].as_i64().unwrap_or(0)))
}

fn put(host: &DaemonHost, session: &str, manifest: &Path, outil: &str, titre: &str) -> Value {
    put_corps(host, session, manifest, outil, titre, &format!("le corps de {titre}"))
}

/// `etat` est requis par la vue `editable` (comme dans le gabarit livré) :
/// l'état de départ, que la transition fera avancer.
fn put_corps(host: &DaemonHost, session: &str, manifest: &Path, outil: &str, titre: &str, corps: &str) -> Value {
    host.call(Some(session), manifest, outil, json!({"record": {"titre": titre, "corps": corps, "etat": "courant"}}))
}

#[test]
#[ignore]
fn deux_backends_dans_un_processus_chacun_sa_base() {
    let (da, db) = (dossier("a"), dossier("b"));
    let (ma, mb) = (manifeste(&da), manifeste(&db));
    let host = DaemonHost::new();
    let s = host.open_session("témoin", None, vec![]);
    ok(&host.open(Some(&s), &ma));
    ok(&host.open(Some(&s), &mb));
    ok(&put(&host, &s, &ma, "put_fiche", "seulement-dans-a"));
    assert!(fiche(&host, &s, &ma, "seulement-dans-a").is_some(), "la fiche est dans A");
    assert!(fiche(&host, &s, &mb, "seulement-dans-a").is_none(), "et pas dans B");
    let etat = host.state();
    let backends = etat["backends"].as_array().unwrap();
    assert_eq!(backends.len(), 2, "{etat}");
    assert!(backends.iter().all(|b| b["opened"] == json!(1)), "chacun ouvert une fois : {etat}");
    host.shutdown();
    for d in [da, db] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
#[ignore]
fn deux_sessions_un_backend_charge_une_fois_et_une_seule_reaction() {
    let d = dossier("partage");
    let m = manifeste(&d);
    // Deux écritures du même chemin : le démon les reconnaît pour un seul
    // backend, par le chemin canonique.
    let m_bis = d.join(".").join("backend.json");
    let host = DaemonHost::new();
    let sa = host.open_session("session A", None, vec![]);
    let sb = host.open_session("session B", None, vec![]);
    ok(&host.open(Some(&sa), &m));
    ok(&host.open(Some(&sb), &m_bis));
    let etat = host.state();
    assert_eq!(etat["backends"].as_array().unwrap().len(), 1, "un seul backend pour deux chemins du même manifeste : {etat}");
    assert_eq!(etat["sessions"].as_array().unwrap().len(), 2, "{etat}");

    // A écrit et ancre, puis change le sujet : la réaction de l'unique
    // réacteur transitionne la fiche ancrée.
    ok(&put(&host, &sa, &m, "put_fiche", "la-fiche"));
    ok(&put(&host, &sa, &m, "put_sujet", "le-sujet"));
    ok(&host.call(Some(&sa), &m, "ancrer", json!({"links": [{"from": {"titre": "la-fiche"}, "to": {"titre": "le-sujet"}}]})));
    // Une mise à jour porte la révision qu'elle remplace (écriture optimiste).
    ok(&host.call(
        Some(&sa),
        &m,
        "put_sujet",
        json!({"record": {"titre": "le-sujet", "corps": "le sujet a changé", "etat": "courant"}, "expected_revision": 1}),
    ));
    let debut = std::time::Instant::now();
    let vue_par_b = loop {
        let f = fiche(&host, &sb, &m_bis, "la-fiche");
        if f.as_ref().is_some_and(|(etat, _)| etat == "a_revoir") || debut.elapsed() > Duration::from_secs(20) {
            break f;
        }
        std::thread::sleep(Duration::from_millis(200));
    };
    let (etat_b, revision) = vue_par_b.expect("B lit la fiche que A a écrite");
    assert_eq!(etat_b, "a_revoir", "B voit la transition que la réaction a faite");
    // Une seule transition : un seul réacteur pour deux sessions.
    std::thread::sleep(Duration::from_secs(2));
    let (_, revision_apres) = fiche(&host, &sb, &m_bis, "la-fiche").unwrap();
    assert_eq!(revision_apres, revision, "aucune seconde exécution de la réaction");
    host.shutdown();
    let _ = std::fs::remove_dir_all(d);
}

#[test]
#[ignore]
fn sans_session_un_refus_nomme() {
    let d = dossier("sans-session");
    let m = manifeste(&d);
    let host = DaemonHost::new();
    let corps = serde_json::to_vec(&json!({"manifest": m})).unwrap();
    let (statut, reponse) = host.handle("POST", "/open", None, &corps);
    assert_eq!(statut, 401, "{reponse}");
    assert!(reponse["error"].as_str().unwrap().contains("POST /session d'abord"), "{reponse}");
    let (statut, reponse) = host.handle("POST", "/open", Some("s-inventee"), &corps);
    assert_eq!(statut, 401, "{reponse}");
    assert!(reponse["error"].as_str().unwrap().contains("session inconnue"), "{reponse}");
    let _ = std::fs::remove_dir_all(d);
}

#[test]
#[ignore]
fn une_base_a_rouvrir_ne_lache_que_son_backend() {
    let (da, db) = (dossier("rouvrir-a"), dossier("rouvrir-b"));
    let (ma, mb) = (manifeste(&da), manifeste(&db));
    // L'ouvreur garde le crochet de chaque base, pour en faire échouer une.
    let crochets: Arc<Mutex<HashMap<PathBuf, FailedCheckpointHook>>> = Arc::default();
    let pour_l_ouvreur = crochets.clone();
    let host = DaemonHost::with_opener(Box::new(move |prepared: PreparedBackend| {
        let base = prepared.path(&prepared.manifest.database);
        let conn = Rag3dbConnection::new(&base).map_err(|e| e.to_string())?;
        pour_l_ouvreur.lock().unwrap().insert(base, conn.failed_checkpoint_hook());
        prepared.open(Box::new(conn), None)
    }));
    let s = host.open_session("témoin", None, vec![]);
    ok(&host.open(Some(&s), &ma));
    ok(&host.open(Some(&s), &mb));
    ok(&put(&host, &s, &ma, "put_fiche", "avant"));

    // La prochaine instruction de A rencontre un point de reprise échoué.
    crochets.lock().unwrap()[&da.join("base.rag3db")].after(0);
    let r = put(&host, &s, &ma, "put_fiche", "pendant");
    assert_eq!(r["ok"], json!(false), "{r}");
    assert!(r["error"].as_str().unwrap().starts_with(REOPENED), "une erreur transitoire nommée : {r}");
    assert_eq!(r["transient"], json!(true), "{r}");

    // B n'a rien vu ; A se rouvre au prochain appel, et garde ce qu'il avait validé.
    ok(&put(&host, &s, &mb, "put_fiche", "b-continue"));
    assert!(fiche(&host, &s, &ma, "avant").is_some(), "A rouvert lit ce qui était validé");
    ok(&put(&host, &s, &ma, "put_fiche", "apres"));
    let etat = host.state();
    let par_base: HashMap<String, i64> = etat["backends"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| (b["manifest"].as_str().unwrap().to_string(), b["opened"].as_i64().unwrap()))
        .collect();
    let canon = |m: &Path| std::fs::canonicalize(m).unwrap().display().to_string();
    assert_eq!(par_base[&canon(&ma)], 2, "A ouvert deux fois : {etat}");
    assert_eq!(par_base[&canon(&mb)], 1, "B jamais rouvert : {etat}");
    host.shutdown();
    for d in [da, db] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
#[ignore]
fn par_le_fil_http_jusqu_a_l_arret() {
    let d = dossier("http");
    let m = manifeste(&d);
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let host = Arc::new(DaemonHost::new());
    let servi = {
        let host = host.clone();
        std::thread::spawn(move || serve_daemon(host, server, 4))
    };
    let url = |chemin: &str| format!("http://127.0.0.1:{port}{chemin}");
    // Un refus garde son corps : le statut n'est pas une erreur du client.
    let agent = ureq::Agent::new_with_config(ureq::Agent::config_builder().http_status_as_error(false).build());
    let post = |chemin: &str, session: Option<&str>, corps: Value| -> (u16, Value) {
        let mut req = agent.post(&url(chemin));
        if let Some(s) = session {
            req = req.header(SESSION_HEADER, s);
        }
        let mut r = req
            .header("Content-Type", "application/json")
            .send(corps.to_string())
            .unwrap_or_else(|e| panic!("{chemin} : {e}"));
        let statut = r.status().as_u16();
        (statut, serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap())
    };
    let get = |chemin: &str| -> Value {
        serde_json::from_str(&agent.get(&url(chemin)).call().unwrap().body_mut().read_to_string().unwrap()).unwrap()
    };
    let sante = get("/sante");
    assert_eq!(sante["service"], json!("rag3weaver-daemon"), "{sante}");
    let (statut, refus) = post("/call", None, json!({"manifest": m, "name": "put_fiche", "arguments": {}}));
    assert_eq!(statut, 401, "{refus}");
    let (_, session) = post("/session", None, json!({"client": "témoin http", "keys": ["dev"]}));
    let s = session["session"].as_str().unwrap().to_string();
    let (statut, ouvert) = post("/open", Some(&s), json!({"manifest": m}));
    assert_eq!(statut, 200, "{ouvert}");
    assert!(ouvert["result"]["describe"]["tools"].as_array().unwrap().len() >= 4, "{ouvert}");
    let (statut, ecrit) =
        post("/call", Some(&s), json!({"manifest": m, "name": "put_fiche", "arguments": {"record": {"titre": "par-le-fil", "corps": "x", "etat": "courant"}}}));
    assert_eq!(statut, 200, "{ecrit}");
    let etat = get("/etat");
    assert_eq!(etat["sessions"][0]["client"], json!("témoin http"), "l'état du démon dit qui est connecté : {etat}");
    assert_eq!(etat["sessions"][0]["keys"], json!(["dev"]), "{etat}");
    let (statut, _) = post("/arret", None, json!({}));
    assert_eq!(statut, 200);
    servi.join().expect("le démon s'arrête et ferme ses backends");
    let _ = std::fs::remove_dir_all(d);
}
