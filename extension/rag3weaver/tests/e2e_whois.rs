//! E2E : **`whois`** (`docs/25-octobre-2026-memoire-longue/04-…` §8, pièce 3),
//! lu par un Plan déclaré (`templates/backends/memory/plans/whois.json`) et
//! deux scripts (`scripts/whois_params.rhai`, `scripts/whois.rhai`) :
//!
//! - par un handle, la fiche d'identité qui le cite par un apport `handle`, sa
//!   clé, ses deux handles datés, ses alias datés (le dernier en tête) ;
//! - la tâche en cours est la première fiche de la ligne `doing:` du **dernier**
//!   apport `identity`, pas la première citée : A, citée plus tôt par un autre
//!   apport, vient après B parce que la ligne dit « B puis A » ;
//! - par la clé et par un alias, la même réponse ;
//! - un handle inconnu : tout à null, `said` le dit ; deux questions à la fois :
//!   refusé.
//!
//! Sur le vrai gabarit `memory` (ses signaux : `RAG3WEAVER_EMBED_SERVICE`).
//!
//! Run with: ./run_e2e.sh --test e2e_whois

#![cfg(feature = "rag3db-native")]

use std::path::Path;

use serde_json::{json, Value};

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::Rag3dbConnection;

/// Le gabarit `memory` dans une copie de `templates/`, son extension vector
/// prise au bâti (comme `e2e_cited_by`).
fn ouvrir(dir: &Path) -> Backend {
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
    copier(&Path::new(env!("CARGO_MANIFEST_DIR")).join("templates"), &dir.join("templates"));
    let manifeste = dir.join("templates/backends/memory/backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["vector_extension"] = json!(crate::common::vector_extension());
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let prepared = PreparedBackend::load(&manifeste).expect("le gabarit memory charge");
    let (embedder, _) = prepared.connect_embedder().unwrap_or_else(|e| {
        panic!("le témoin embarque : posez RAG3WEAVER_EMBED_SERVICE — un témoin sauté n'est pas vert ({e})")
    });
    prepared
        .open(Box::new(Rag3dbConnection::new(&dir.join("memoire.rag3db")).expect("la base")), Some(embedder))
        .expect("l'ouverture")
}

fn appel(b: &Backend, outil: &str, args: Value) -> Value {
    b.call(outil, args.clone()).unwrap_or_else(|e| panic!("{outil} {args} : {e}"))
}

/// Un apport L1 rangé sur sa fiche, qui cite des Refs. Chaque apport a sa
/// source : la clé d'un apport est (fiche, source, section), deux handles
/// sous la même source n'en feraient qu'un.
fn apport(b: &Backend, fiche: &str, source: &str, section: &str, texte: &str, cites: &[(&str, &str)]) {
    std::thread::sleep(std::time::Duration::from_millis(5));
    let cle = json!({"fiche": fiche, "source": source, "section": section});
    let mut record = cle.clone();
    record["texte"] = json!(texte);
    record["niveau"] = json!("L1");
    record["state"] = json!("current");
    appel(b, "put_apport", json!({"record": record}));
    appel(b, "relier_apport", json!({"links": [{"from": {"name": fiche}, "to": cle}]}));
    for (genre, valeur) in cites {
        appel(b, "citer_source", json!({"genre": genre, "valeur": valeur, "fiche": cle}));
    }
}

fn whois(b: &Backend, args: Value) -> Value {
    let r = appel(b, "whois", args);
    r.get("result").cloned().unwrap_or(r)
}

const IDENTITY: &str = "## identity\nwho: t1 — « équipier doc »\nruntime: claude-code ; session:s2\ncan: memoire-longue\ndoing: tache-b, puis tache-a\nholding: rien\nmandates: aucun\npeers: aucun";

fn peupler(b: &Backend) {
    for (genre, description) in [
        ("handle", "une adresse du harnais : session:<uuid>, uds:<socket>, task:<nom>"),
        ("alias", "un nom affiché par un agent"),
        ("fiche", "une fiche de la mémoire, par son nom"),
    ] {
        appel(b, "create_ref_type", json!({"name": genre, "description": description, "confirm": true}));
    }
    for (name, about) in [("t1", "Un équipier de documentation."), ("tache-a", "La tâche A."), ("tache-b", "La tâche B.")] {
        appel(b, "put_fiche", json!({"record": {"name": name, "about": about, "state": "current"}}));
    }
    apport(b, "t1", "sessions/1", "handle", "session:s1", &[("handle", "session:s1")]);
    apport(b, "t1", "sessions/1", "alias", "je m'affiche « aide »", &[("alias", "aide")]);
    // A est citée plus tôt, par un autre apport que l'identity.
    apport(b, "t1", "sessions/1", "plan", "d'abord la tâche A", &[("fiche", "tache-a")]);
    apport(b, "t1", "sessions/2", "handle", "session:s2", &[("handle", "session:s2")]);
    apport(b, "t1", "sessions/2", "alias", "je m'affiche « équipier doc »", &[("alias", "équipier doc")]);
    apport(b, "t1", "sessions/2", "identity", IDENTITY, &[("fiche", "tache-a"), ("fiche", "tache-b"), ("handle", "session:s2")]);
}

#[test]
#[ignore]
fn whois_rend_la_cle_les_handles_les_alias_et_la_tache_de_la_ligne_doing() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path());
    peupler(&b);

    let r = whois(&b, json!({"handle": "session:s1"}));
    assert_eq!(r["agent"], "t1", "{r}");
    assert_eq!(r["tache"], "tache-b", "la première de la ligne doing:, pas la première citée : {r}");
    assert_eq!(r["autres"], json!(["tache-a"]), "{r}");
    assert_eq!(r["identite"], IDENTITY, "{r}");
    assert!(r["identity_at"].as_i64().is_some_and(|t| t > 0), "{r}");
    assert_eq!(r["alias"], "équipier doc", "le dernier alias : {r}");
    let aliases: Vec<&str> = r["aliases"].as_array().unwrap().iter().map(|a| a["nom"].as_str().unwrap()).collect();
    assert_eq!(aliases, ["équipier doc", "aide"], "les plus récents d'abord : {r}");
    assert!(r["aliases"][0]["at"].as_i64() > r["aliases"][1]["at"].as_i64(), "datés : {r}");
    let handles: Vec<&str> = r["handles"].as_array().unwrap().iter().map(|h| h["valeur"].as_str().unwrap()).collect();
    assert_eq!(handles, ["session:s2", "session:s1"], "les deux handles, le dernier d'abord : {r}");
    assert!(r["said"].is_null(), "rien ne manque : {r}");

    // Par la clé, par un alias passé : la même réponse.
    assert_eq!(whois(&b, json!({"agent": "t1"})), r);
    assert_eq!(whois(&b, json!({"alias": "aide"})), r);

    // Un handle inconnu : tout à null, `said` le dit.
    let r = whois(&b, json!({"handle": "session:inconnue"}));
    for k in ["agent", "identite", "alias", "tache", "identity_at"] {
        assert!(r[k].is_null(), "{k} : {r}");
    }
    assert_eq!((r["aliases"].clone(), r["handles"].clone(), r["autres"].clone()), (json!([]), json!([]), json!([])), "{r}");
    assert!(r["said"].as_str().is_some_and(|s| s.contains("aucune fiche d'identité")), "{r}");

    // Une fiche sans apport identity : ses handles, `said` le dit.
    appel(&b, "put_fiche", json!({"record": {"name": "t2", "about": "Un autre.", "state": "current"}}));
    apport(&b, "t2", "sessions/1", "handle", "session:s9", &[("handle", "session:s9")]);
    let r = whois(&b, json!({"handle": "session:s9"}));
    assert_eq!((r["agent"].clone(), r["tache"].clone()), (json!("t2"), Value::Null), "{r}");
    assert!(r["said"].as_str().is_some_and(|s| s.contains("aucun apport identity")), "{r}");

    // Deux questions, ou aucune : refusé.
    let e = b.call("whois", json!({"agent": "t1", "handle": "session:s1"})).err().expect("deux questions");
    assert!(e.contains("un seul"), "{e}");
    assert!(b.call("whois", json!({})).is_err());
}
