//! **La cellule des bouts d'un lien** (§3 bis de la page du lien,
//! `docs/11-octobre-2026-memoire-longue/01-un-lien-entre-deux-identites-declarees.md`),
//! sur la migration (a) de l'arbre principal (cc1254ac9 : une cellule autre que
//! la cellule par défaut entre dans `_uuid`) :
//!
//! - par défaut, les deux bouts d'un lien se résolvent dans la cellule de la
//!   session qui l'écrit ;
//! - un lien vers une autre cellule se **déclare** : `to_cell` (et `from_cell`)
//!   sur le lien, `{org, project}` ;
//! - un bout introuvable dans la cellule visée est refusé en nommant l'identité
//!   et la cellule.
//!
//! Le cas d'usage : ancrer une mémoire d'un projet à un sujet global.

#![cfg(all(feature = "rag3db-native", feature = "daemon"))]

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use rag3weaver::daemon_host::DaemonHost;
use rag3weaver::Rag3dbConnection;

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
    crate::common::vector_extension()
}

/// Fiches, sujets, l'ancre ; pas de réaction (ce témoin ne regarde que le lien).
fn manifeste(dir: &Path) -> PathBuf {
    let gabarits = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let put = gabarits.join("backends/notebook/graphs/put.mmd");
    let lien = gabarits.join("tools/link.mmd");
    std::fs::write(dir.join("fiche.json"), serde_json::to_vec_pretty(&schema(&["courant"])).unwrap()).unwrap();
    std::fs::write(dir.join("sujet.json"), serde_json::to_vec_pretty(&schema(&["courant"])).unwrap()).unwrap();
    let manifest = json!({
        "version": 1,
        "name": "cellule-des-bouts",
        "database": "base.rag3db",
        "embeddings": {"address": "127.0.0.1:9", "model": "none", "dimensions": 8},
        "vector_extension": extension_vecteur(),
        "entities": {"Fiche": entite("fiche.json", None), "Sujet": entite("sujet.json", None)},
        "relations": {"ANCRE": {"from": "Fiche", "to": "Sujet"}},
        "tools": {
            "put_fiche": {"graph": put, "bindings": {"entity": "Fiche"}, "input_payloads": {"record": {"entity": "Fiche", "view": "editable"}}},
            "put_sujet": {"graph": put, "bindings": {"entity": "Sujet"}, "input_payloads": {"record": {"entity": "Sujet", "view": "editable"}}},
            "ancrer": {"graph": lien, "bindings": {"relation": "ANCRE"}}
        }
    });
    let chemin = dir.join("backend.json");
    std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    chemin
}

fn dossier(cas: &str) -> PathBuf {
    let d = PathBuf::from(std::env::var("HOME").unwrap())
        .join(".cache/rag3weaver-build/cellule-des-bouts")
        .join(format!("{cas}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn ok(reponse: &Value) -> &Value {
    assert_eq!(reponse["ok"], json!(true), "{reponse}");
    &reponse["result"]
}

fn put(host: &DaemonHost, session: &str, manifest: &Path, outil: &str, titre: &str) -> Value {
    host.call(Some(session), manifest, outil, json!({"record": {"titre": titre, "corps": format!("le corps de {titre}"), "etat": "courant"}}))
}

fn ancrer(host: &DaemonHost, session: &str, manifest: &Path, lien: Value) -> Value {
    host.call(Some(session), manifest, "ancrer", json!({"links": [lien]}))
}

/// Le texte d'un refus, quelle que soit sa forme (erreur d'outil, ou rapport
/// d'un lien non posé).
fn texte(r: &Value) -> String {
    r.to_string()
}

#[test]
#[ignore]
fn un_lien_vers_une_autre_cellule_se_declare_par_to_cell() {
    let d = dossier("to-cell");
    let m = manifeste(&d);
    let host = DaemonHost::new();
    let alpha = host.open_session("alpha", Some(json!({"org": "acme", "project": "alpha"})), vec![], None);
    let global = host.open_session("global", Some(json!({"org": "acme", "project": "global"})), vec![], None);
    ok(&host.open(Some(&alpha), &m));
    ok(&put(&host, &global, &m, "put_sujet", "le sujet partagé"));
    ok(&put(&host, &alpha, &m, "put_fiche", "une fiche d'alpha"));

    // Sans déclaration : les deux bouts dans la cellule d'alpha — le sujet n'y est pas.
    let r = ancrer(&host, &alpha, &m, json!({"from": {"titre": "une fiche d'alpha"}, "to": {"titre": "le sujet partagé"}}));
    let refus = texte(&r);
    assert!(r["ok"] != json!(true) || refus.contains("missing"), "sans to_cell, le sujet de global n'est pas trouvé dans alpha : {refus}");
    assert!(refus.contains("le sujet partagé") && refus.contains("acme/alpha"), "le refus nomme le bout et la cellule où il a été cherché : {refus}");

    // Déclaré : `to_cell` vise global.
    let r = ancrer(
        &host,
        &alpha,
        &m,
        json!({"from": {"titre": "une fiche d'alpha"}, "to": {"titre": "le sujet partagé"}, "to_cell": {"org": "acme", "project": "global"}}),
    );
    ok(&r);
    host.shutdown();

    let conn = Rag3dbConnection::new(d.join("base.rag3db")).unwrap();
    use rag3weaver::connection::DbConnection;
    let l = conn.execute("MATCH (f:Fiche)-[:ANCRE]->(s:Sujet) RETURN f.titre, f._project, s.titre, s._project").unwrap();
    let lignes: Vec<String> = l.rows.iter().map(|x| x.iter().map(|v| v.as_str().unwrap_or("?").to_string()).collect::<Vec<_>>().join("/")).collect();
    assert_eq!(lignes, vec!["une fiche d'alpha/alpha/le sujet partagé/global"], "le lien relie deux cellules");
    drop(conn);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
#[ignore]
fn un_bout_hors_de_la_cellule_visee_est_refuse_en_la_nommant() {
    let d = dossier("hors-cellule");
    let m = manifeste(&d);
    let host = DaemonHost::new();
    let alpha = host.open_session("alpha", Some(json!({"org": "acme", "project": "alpha"})), vec![], None);
    let global = host.open_session("global", Some(json!({"org": "acme", "project": "global"})), vec![], None);
    ok(&host.open(Some(&alpha), &m));
    ok(&put(&host, &global, &m, "put_sujet", "le sujet partagé"));
    ok(&put(&host, &alpha, &m, "put_fiche", "une fiche d'alpha"));
    let r = ancrer(
        &host,
        &alpha,
        &m,
        json!({"from": {"titre": "une fiche d'alpha"}, "to": {"titre": "le sujet partagé"}, "to_cell": {"org": "acme", "project": "beta"}}),
    );
    let refus = texte(&r);
    assert!(refus.contains("le sujet partagé") && refus.contains("acme/beta"), "le refus nomme le bout et la cellule visée : {refus}");
    let r = ancrer(&host, &alpha, &m, json!({"from": {"titre": "une fiche d'alpha"}, "to": {"titre": "x"}, "to_cell": {"org": "", "project": "beta"}}));
    assert!(texte(&r).contains("to_cell"), "une cellule mal formée est refusée en nommant le champ : {r}");
    host.shutdown();
    let _ = std::fs::remove_dir_all(d);
}
