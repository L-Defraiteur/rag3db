//! E2E : **la reconstruction de la base depuis les fichiers** (lot M3a de la
//! mémoire, `docs/18-octobre-2026-memoire-longue/04-…` §5) — une entité à
//! fichier propre :
//!
//! - **la parité, champ à champ** : des lignes écrites par un vrai outil (une
//!   réécrite, une sans champ facultatif), puis une base neuve reconstruite
//!   depuis leurs fichiers ; les deux bases portent les mêmes lignes, chaque
//!   champ déclaré, dates et révision comprises ;
//! - une reconstruction sur une base qui n'est pas vide est refusée, et le dit ;
//! - un champ que les fichiers ne portent pas est dit (`fields_not_carried`) au
//!   lieu d'être perdu en silence.
//!
//! Lot M3c : un dossier qui s'écarte de la base (un champ édité à la main, un
//! fichier effacé) se dit à l'ouverture, dans `index_state` (`files` et un
//! avertissement) ; la vérification à blanc reconstruit une copie du dossier
//! dans une base jetable et la compare, champ à champ, à la base vivante.
//!
//! Run with: ./run_e2e.sh --test e2e_reconstruction

#![cfg(feature = "rag3db-native")]

use std::path::{Path, PathBuf};

use serde_json::json;

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::connection::CypherValue;
use rag3weaver::Rag3dbConnection;

/// Un manifeste sur `base` (un nom de base sous `dir`), les fichiers sous
/// `dir/fichiers` ; `en_tete` : les champs que l'en-tête porte.
fn manifeste(dir: &Path, nom: &str, base: &str, en_tete: &[&str]) -> PathBuf {
    std::fs::write(
        dir.join("note.json"),
        serde_json::to_vec_pretty(&json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object", "additionalProperties": false,
            "required": ["titre", "corps", "revision", "created_at", "updated_at"],
            "properties": {
                "titre": {"type": "string", "minLength": 1}, "corps": {"type": "string", "minLength": 1},
                "etiquette": {"type": "string"},
                "revision": {"type": "integer"}, "created_at": {"type": "integer"}, "updated_at": {"type": "integer"}
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let put = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook/graphs/put.mmd");
    let m = json!({
        "version": 1, "name": "reconstruction", "database": base, "files_root": "fichiers",
        "embeddings": {"address": "127.0.0.1:9", "model": "none", "dimensions": 8},
        "vector_extension": crate::common::vector_extension(),
        "entities": {"Note": {
            "schema": "note.json",
            "config": {"fields": {"titre": {"type": "string", "isTitle": true}, "corps": {"type": "string", "isContent": true}},
                       "hashsafe": ["titre"], "signals": []},
            "writes": {"created_at": "created_at", "updated_at": "updated_at", "revision": "revision"},
            "files": {"dir": "notes", "name": "{uuid}.md", "header": en_tete, "body": "corps"}
        }},
        "tools": {"put_note": {"graph": put, "bindings": {"entity": "Note"},
                               "input_payloads": {"record": {"entity": "Note", "view": "editable"}}}}
    });
    let chemin = dir.join(nom);
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    chemin
}

const TOUT: &[&str] = &["titre", "etiquette", "revision", "created_at", "updated_at"];

fn ouvrir(chemin: &Path) -> Backend {
    let prepared = PreparedBackend::load(chemin).expect("le manifeste charge");
    let base = prepared.path(&prepared.manifest.database);
    prepared.open(Box::new(Rag3dbConnection::new(&base).expect("la base")), None).expect("l'ouverture")
}

/// Les lignes de la base, chaque champ déclaré, triées par uuid.
fn lignes(b: &Backend) -> Vec<Vec<CypherValue>> {
    let cat = b.catalog.lock().unwrap();
    cat.execute_raw(
        "MATCH (n:Note) RETURN n._uuid, n.titre, n.etiquette, n.corps, n.revision, n.created_at, n.updated_at ORDER BY n._uuid",
    )
    .expect("les lignes")
    .rows
}

#[test]
#[ignore]
fn une_base_reconstruite_depuis_les_fichiers_porte_les_memes_lignes_champ_a_champ() {
    let dir = tempfile::tempdir().expect("dossier");
    let vivante = ouvrir(&manifeste(dir.path(), "backend.json", "base.rag3db", TOUT));
    vivante.call("put_note", json!({"record": {"titre": "Le WAL", "etiquette": "moteur", "corps": "# Le WAL\n\nLe journal se rejoue.\n"}})).expect("une note");
    vivante.call("put_note", json!({"record": {"titre": "Sans étiquette", "corps": "Une note sans champ facultatif.\n"}})).expect("une note sans étiquette");
    vivante.call("put_note", json!({"record": {"titre": "Réécrite", "etiquette": "v1", "corps": "Première version.\n"}})).expect("une note");
    // Le temps passe : la réécriture doit garder sa date de création, et la sienne.
    std::thread::sleep(std::time::Duration::from_millis(20));
    vivante
        .call("put_note", json!({"record": {"titre": "Réécrite", "etiquette": "v2", "corps": "Seconde version, ---\n\navec un séparateur.\n"}, "expected_revision": 1}))
        .expect("la réécriture");
    let attendu = lignes(&vivante);
    assert_eq!(attendu.len(), 3);
    assert!(attendu.iter().any(|l| l[4] == CypherValue::Int(2)), "une ligne en révision 2 : {attendu:?}");
    drop(vivante);

    // Une base neuve, sur les mêmes fichiers.
    let neuve = ouvrir(&manifeste(dir.path(), "neuve.json", "neuve.rag3db", TOUT));
    let rapport = neuve.rebuild_from_files().expect("la reconstruction");
    assert_eq!(rapport["Note"]["restored"], 3, "{rapport}");
    assert_eq!(rapport["Note"]["refused"], json!([]), "{rapport}");
    assert_eq!(rapport["Note"]["fields_not_carried"], json!([]), "{rapport}");
    assert_eq!(lignes(&neuve), attendu, "la base reconstruite porte les mêmes lignes, champ à champ");

    // Une seconde reconstruction sur une base qui n'est plus vide : refusée, et dite.
    let refus = neuve.rebuild_from_files().expect_err("une base pleine");
    assert!(refus.contains("n'est pas vide") && refus.contains("Note : 3"), "{refus}");
}

#[test]
#[ignore]
fn un_champ_que_les_fichiers_ne_portent_pas_est_dit() {
    let dir = tempfile::tempdir().expect("dossier");
    let vivante = ouvrir(&manifeste(dir.path(), "backend.json", "base.rag3db", &["titre", "revision", "created_at", "updated_at"]));
    vivante.call("put_note", json!({"record": {"titre": "Une note", "etiquette": "perdue", "corps": "Le corps.\n"}})).expect("une note");
    drop(vivante);
    let neuve = ouvrir(&manifeste(dir.path(), "neuve.json", "neuve.rag3db", &["titre", "revision", "created_at", "updated_at"]));
    let rapport = neuve.rebuild_from_files().expect("la reconstruction");
    assert_eq!(rapport["Note"]["restored"], 1, "{rapport}");
    assert_eq!(rapport["Note"]["fields_not_carried"], json!(["etiquette"]), "le champ perdu se dit : {rapport}");
}

// ── M3b : les fichiers composés, par leur bloc machine ─────────────────────

fn schema(dir: &Path, nom: &str, champs: &[&str]) {
    let mut props = serde_json::Map::new();
    for c in champs {
        props.insert(c.to_string(), json!({"type": "string", "minLength": 1}));
    }
    for c in ["revision", "created_at", "updated_at"] {
        props.insert(c.into(), json!({"type": "integer"}));
    }
    let mut requis: Vec<&str> = champs.to_vec();
    requis.extend(["revision", "created_at", "updated_at"]);
    std::fs::write(
        dir.join(nom),
        serde_json::to_vec_pretty(&json!({"$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object",
            "additionalProperties": false, "required": requis, "properties": props}))
        .unwrap(),
    )
    .unwrap();
}

/// Un sujet à fichier composé qui lit ses notes (`A_NOTE`) ; les notes n'ont
/// pas de fichier à elles ; une note cite une Ref, sans fichier non plus.
fn manifeste_compose(dir: &Path, nom: &str, base: &str) -> PathBuf {
    schema(dir, "sujet.json", &["name", "about"]);
    schema(dir, "note.json", &["titre", "texte"]);
    schema(dir, "ref.json", &["cle"]);
    std::fs::write(
        dir.join("sujet.md.jinja"),
        "# {{ record.name }}\n\n{{ record.about }}\n\n## Notes\n{% for n in related.notes %}\n- {{ n.titre }} : {{ n.texte }}\n{% endfor %}\n",
    )
    .unwrap();
    let tpl = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let put = tpl.join("backends/notebook/graphs/put.mmd");
    let lien = tpl.join("tools/link.mmd");
    let ecrit = json!({"created_at": "created_at", "updated_at": "updated_at", "revision": "revision"});
    let m = json!({
        "version": 1, "name": "compose", "database": base, "files_root": "fichiers",
        "embeddings": {"address": "127.0.0.1:9", "model": "none", "dimensions": 8},
        "vector_extension": crate::common::vector_extension(),
        "entities": {
            "Sujet": {"schema": "sujet.json",
                "config": {"fields": {"name": {"type": "string", "isTitle": true}, "about": {"type": "string", "isContent": true}}, "hashsafe": ["name"], "signals": []},
                "writes": ecrit,
                "files": {"dir": "sujets", "name": "{uuid}.md", "header": ["name"], "template": "sujet.md.jinja",
                          "related": {"notes": {"relation": "A_NOTE", "order_by": "titre"}}}},
            "Note": {"schema": "note.json",
                "config": {"fields": {"titre": {"type": "string", "isTitle": true}, "texte": {"type": "string", "isContent": true}}, "hashsafe": ["titre"], "signals": []},
                "writes": ecrit},
            "Ref": {"schema": "ref.json",
                "config": {"fields": {"cle": {"type": "string", "isContent": true}}, "hashsafe": ["cle"], "signals": []},
                "writes": ecrit}
        },
        "relations": {"A_NOTE": {"from": "Sujet", "to": "Note"}, "CITE": {"from": "Note", "to": "Ref"}},
        "tools": {
            "put_sujet": {"graph": put, "bindings": {"entity": "Sujet"}, "input_payloads": {"record": {"entity": "Sujet", "view": "editable"}}},
            "put_note": {"graph": put, "bindings": {"entity": "Note"}, "input_payloads": {"record": {"entity": "Note", "view": "editable"}}},
            "put_ref": {"graph": put, "bindings": {"entity": "Ref"}, "input_payloads": {"record": {"entity": "Ref", "view": "editable"}}},
            "noter": {"graph": lien, "bindings": {"relation": "A_NOTE"}},
            "citer": {"graph": lien, "bindings": {"relation": "CITE"}}
        }
    });
    let chemin = dir.join(nom);
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    chemin
}

fn requete(b: &Backend, q: &str) -> Vec<Vec<CypherValue>> {
    b.catalog.lock().unwrap().execute_raw(q).unwrap_or_else(|e| panic!("{q} : {e}")).rows
}

#[test]
#[ignore]
fn les_fiches_d_une_base_perdue_reviennent_par_leur_bloc_machine_et_une_ligne_sans_fichier_voyage_avec_le_bloc() {
    let dir = tempfile::tempdir().expect("dossier");
    let vivante = ouvrir(&manifeste_compose(dir.path(), "backend.json", "base.rag3db"));
    let sujet = vivante.call("put_sujet", json!({"record": {"name": "Le WAL", "about": "La reprise, --> et & dedans."}})).expect("le sujet")["result"]["uuid"]
        .as_str()
        .unwrap()
        .to_string();
    vivante.call("put_note", json!({"record": {"titre": "rejeu", "texte": "en entier"}})).expect("une note");
    vivante.call("put_note", json!({"record": {"titre": "tampon", "texte": "les pages"}})).expect("une note");
    vivante.call("put_ref", json!({"record": {"cle": "src/storage/wal.cpp"}})).expect("une ref");
    vivante.call("citer", json!({"links": [{"from": {"titre": "rejeu"}, "to": {"cle": "src/storage/wal.cpp"}}]})).expect("la citation");
    vivante
        .call("noter", json!({"links": [{"from": {"name": "Le WAL"}, "to": {"titre": "rejeu"}}, {"from": {"name": "Le WAL"}, "to": {"titre": "tampon"}}]}))
        .expect("les notes du sujet");
    // Le fichier composé, réécrit par la réaction de miroir : son bloc porte le
    // sujet, ses deux notes et la citation.
    let fichier = dir.path().join("fichiers/sujets").join(format!("{sujet}.md"));
    let debut = std::time::Instant::now();
    loop {
        let texte = std::fs::read_to_string(&fichier).unwrap_or_default();
        if texte.contains("CITE") && texte.contains("tampon") && texte.contains("rejeu") {
            assert!(texte.contains(rag3weaver::fichiers_entite::DEBUT_DU_BLOC), "{texte}");
            break;
        }
        assert!(debut.elapsed() < std::time::Duration::from_secs(5), "le bloc machine n'est pas complet après 5 s : {texte}");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let sujets = "MATCH (n:Sujet) RETURN n._uuid, n.name, n.about, n.revision, n.created_at, n.updated_at ORDER BY n._uuid";
    let notes = "MATCH (n:Note) RETURN n._uuid, n.titre, n.texte, n.revision, n.created_at, n.updated_at ORDER BY n._uuid";
    let a_note = "MATCH (a:Sujet)-[:A_NOTE]->(b:Note) RETURN a._uuid, b._uuid ORDER BY b._uuid";
    let (s0, n0, l0) = (requete(&vivante, sujets), requete(&vivante, notes), requete(&vivante, a_note));
    assert_eq!((s0.len(), n0.len(), l0.len()), (1, 2, 2));
    drop(vivante);

    // La base est perdue : une base neuve, sur les mêmes fichiers.
    let neuve = ouvrir(&manifeste_compose(dir.path(), "neuve.json", "neuve.rag3db"));
    let rapport = neuve.rebuild_from_files().expect("la reconstruction");
    assert_eq!(rapport["Sujet"]["restored"], 1, "{rapport}");
    assert_eq!(rapport["Note"]["restored"], 2, "{rapport}");
    // La Ref n'a pas de fichier : citée par une note du bloc, elle y voyage
    // (la fermeture du bloc), et la citation se repose.
    assert_eq!(rapport["Ref"]["restored"], 1, "{rapport}");
    assert_eq!(rapport["links"]["posed"], 3, "{rapport}");
    assert_eq!(rapport["links"]["to_absent_rows"], json!({}), "aucun lien vers une ligne absente : {rapport}");
    assert_eq!(requete(&neuve, sujets), s0, "le sujet revient champ à champ, son texte avec --> et & compris");
    assert_eq!(requete(&neuve, notes), n0, "les notes reviennent champ à champ, depuis le bloc du sujet");
    assert_eq!(requete(&neuve, a_note), l0, "les liens du sujet à ses notes sont reposés");
    assert_eq!(
        requete(&neuve, "MATCH (:Note {titre: 'rejeu'})-[:CITE]->(r:Ref) RETURN r.cle"),
        vec![vec![CypherValue::String("src/storage/wal.cpp".into())]],
        "la Ref revient par le bloc qui la cite, et la citation avec elle"
    );
}

// ── M3c : la comparaison à l'ouverture, la vérification à blanc ────────────

/// Le fichier propre d'une note, par son uuid.
fn fichier_de(dir: &Path, uuid: &str) -> PathBuf {
    dir.join("fichiers/notes").join(format!("{uuid}.md"))
}

#[test]
#[ignore]
fn un_dossier_qui_s_ecarte_de_la_base_se_dit_a_l_ouverture() {
    let dir = tempfile::tempdir().expect("dossier");
    let chemin = manifeste(dir.path(), "backend.json", "base.rag3db", TOUT);
    let vivante = ouvrir(&chemin);
    let editee = vivante.call("put_note", json!({"record": {"titre": "Alpha", "etiquette": "moteur", "corps": "Le corps.\n"}})).expect("une note")["result"]["uuid"]
        .as_str()
        .unwrap()
        .to_string();
    let perdue = vivante.call("put_note", json!({"record": {"titre": "Beta", "corps": "Un autre corps.\n"}})).expect("une note")["result"]["uuid"]
        .as_str()
        .unwrap()
        .to_string();
    let etat = vivante.index_states().expect("l'état");
    assert_eq!(etat["files"]["gaps"], 0, "un dossier fidèle n'a pas d'écart : {etat}");
    drop(vivante);

    // Base fermée : une main édite un fichier, une autre en efface un.
    let texte = std::fs::read_to_string(fichier_de(dir.path(), &editee)).expect("le fichier d'Alpha");
    assert_eq!(texte.matches("moteur").count(), 1, "{texte}");
    std::fs::write(fichier_de(dir.path(), &editee), texte.replace("moteur", "stockage")).unwrap();
    std::fs::remove_file(fichier_de(dir.path(), &perdue)).unwrap();

    let rouverte = ouvrir(&chemin);
    let etat = rouverte.index_states().expect("l'état");
    assert_eq!(etat["files"]["gaps"], 2, "{etat}");
    let ecarts = etat["files"]["by_entity"]["Note"].as_array().expect("les écarts de Note").clone();
    assert!(
        ecarts.iter().any(|e| e["kind"] == "field" && e["uuid"] == editee.as_str() && e["field"] == "etiquette"
            && e["file_value"] == "stockage" && e["base_value"] == "moteur"),
        "le champ édité se nomme, avec ses deux valeurs : {ecarts:?}"
    );
    assert!(ecarts.iter().any(|e| e["kind"] == "row_without_file" && e["uuid"] == perdue.as_str()), "la ligne sans fichier se nomme : {ecarts:?}");
    let avertissements = etat["warnings"].as_array().expect("des avertissements").clone();
    assert!(avertissements.iter().any(|a| a.as_str().is_some_and(|a| a.starts_with("dossier ≠ base : 2 écart(s)"))), "{avertissements:?}");
}

#[test]
#[ignore]
fn la_verification_a_blanc_reconstruit_une_copie_et_la_compare_a_la_base_vivante() {
    let dir = tempfile::tempdir().expect("dossier");
    let vivante = ouvrir(&manifeste(dir.path(), "backend.json", "base.rag3db", TOUT));
    let uuid = vivante.call("put_note", json!({"record": {"titre": "Alpha", "etiquette": "moteur", "corps": "Le corps.\n"}})).expect("une note")["result"]["uuid"]
        .as_str()
        .unwrap()
        .to_string();
    vivante.call("put_note", json!({"record": {"titre": "Beta", "corps": "Un autre corps.\n"}})).expect("une note");
    let avant: Vec<_> = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name()).collect();

    let r = vivante.verify_files().expect("la vérification");
    assert_eq!(r["ok"], true, "un dossier fidèle se reconstruit à l'identique : {r}");
    assert_eq!(r["entities"]["Note"]["rows"], 2, "{r}");
    assert_eq!(r["rebuild"]["Note"]["restored"], 2, "{r}");

    // Le fichier s'écarte : la vérification le voit, champ nommé.
    let texte = std::fs::read_to_string(fichier_de(dir.path(), &uuid)).expect("le fichier d'Alpha");
    std::fs::write(fichier_de(dir.path(), &uuid), texte.replace("moteur", "stockage")).unwrap();
    let r = vivante.verify_files().expect("la vérification");
    assert_eq!(r["ok"], false, "{r}");
    assert_eq!(
        r["entities"]["Note"]["differing"],
        json!([{"uuid": uuid, "field": "etiquette", "live": "moteur", "rebuilt": "stockage"}]),
        "{r}"
    );
    // Rien n'est écrit à côté de la vraie base ; la vraie base n'a pas bougé.
    let apres: Vec<_> = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(avant, apres, "la vérification travaille dans un répertoire jetable");
    assert_eq!(lignes(&vivante).len(), 2);
}

// ── La version du format et le manifeste du dossier (25 oct. 2026) ─────────

/// Le manifeste `chemin`, son `files_format` remplacé.
fn avec_format(chemin: &Path, format: serde_json::Value) {
    let mut m: serde_json::Value = serde_json::from_slice(&std::fs::read(chemin).unwrap()).unwrap();
    m["files_format"] = format;
    std::fs::write(chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
}

fn le_manifeste_du_dossier(dir: &Path) -> PathBuf {
    dir.join("fichiers").join(rag3weaver::fichiers_entite::MANIFESTE_DU_DOSSIER)
}

#[test]
#[ignore]
fn un_dossier_sans_manifeste_ou_d_un_autre_gabarit_se_dit_et_ne_se_reconstruit_pas() {
    let dir = tempfile::tempdir().expect("dossier");
    let vivante = ouvrir(&manifeste(dir.path(), "backend.json", "base.rag3db", TOUT));
    vivante.call("put_note", json!({"record": {"titre": "Le WAL", "corps": "Le journal se rejoue.\n"}})).expect("une note");
    drop(vivante);
    let ecrit = std::fs::read_to_string(le_manifeste_du_dossier(dir.path())).expect("le manifeste du dossier, écrit à l'ouverture");
    for attendu in ["nom: reconstruction", "format: 1", "Note:", "dossier: notes"] {
        assert!(ecrit.contains(attendu), "le manifeste dit « {attendu} » : {ecrit}");
    }

    // Sans manifeste : l'ouverture le dit sans le réécrire, la reconstruction refuse.
    std::fs::remove_file(le_manifeste_du_dossier(dir.path())).unwrap();
    let neuve = ouvrir(&manifeste(dir.path(), "neuve.json", "neuve.rag3db", TOUT));
    let etat = neuve.index_states().expect("l'état");
    assert!(etat["files"]["manifest"].as_str().is_some_and(|e| e.contains("pas de manifeste.yaml")), "{etat}");
    assert!(etat["warnings"].to_string().contains("dossier :"), "l'avertissement le dit : {etat}");
    assert!(!le_manifeste_du_dossier(dir.path()).exists(), "un dossier sans manifeste n'en reçoit pas un deviné");
    let refus = neuve.rebuild_from_files().expect_err("sans manifeste");
    assert!(refus.contains("manifeste.yaml") && refus.contains("rien n'est reconstruit"), "{refus}");
    drop(neuve);

    // D'un autre gabarit : dit, refusé, et pas écrasé.
    std::fs::write(le_manifeste_du_dossier(dir.path()), "gabarit:\n  nom: autre\n  version: 1\nformat: 1\n").unwrap();
    let neuve = ouvrir(&manifeste(dir.path(), "neuve2.json", "neuve2.rag3db", TOUT));
    let refus = neuve.rebuild_from_files().expect_err("un autre gabarit");
    assert!(refus.contains("« autre »"), "{refus}");
    assert!(std::fs::read_to_string(le_manifeste_du_dossier(dir.path())).unwrap().contains("nom: autre"), "le manifeste d'un autre n'est pas écrasé");
}

#[test]
#[ignore]
fn un_dossier_d_anciens_fichiers_se_relit_par_sa_migration_et_un_format_inconnu_se_dit() {
    let dir = tempfile::tempdir().expect("dossier");
    let vivante = ouvrir(&manifeste(dir.path(), "backend.json", "base.rag3db", TOUT));
    vivante.call("put_note", json!({"record": {"titre": "Le WAL", "etiquette": "moteur", "corps": "Le journal se rejoue.\n"}})).expect("une note");
    let futur = vivante.call("put_note", json!({"record": {"titre": "Venue du futur", "corps": "Écrite par un harnais plus neuf.\n"}})).expect("une note")
        ["result"]["uuid"].as_str().unwrap().to_string();
    drop(vivante);
    let fichier = dir.path().join("fichiers/notes").join(format!("{futur}.md"));
    let texte = std::fs::read_to_string(&fichier).unwrap();
    assert!(texte.contains("\nformat: 1\n"), "chaque fichier porte son format : {texte}");
    std::fs::write(&fichier, texte.replace("\nformat: 1\n", "\nformat: 9\n")).unwrap();

    // Le format 2 : l'étiquette prend un « # ». Une migration déclarée, un pas.
    std::fs::write(
        dir.path().join("m1-2.rhai"),
        "let f = input.fields; if f.contains(\"etiquette\") { f.etiquette = \"#\" + f.etiquette; } f",
    )
    .unwrap();
    let chemin = manifeste(dir.path(), "neuve.json", "neuve.rag3db", TOUT);
    avec_format(&chemin, json!({"version": 2, "migrations": [{"from": 1, "to": 2, "script": "m1-2.rhai"}]}));
    let neuve = ouvrir(&chemin);
    let rapport = neuve.rebuild_from_files().expect("un dossier au format 1 se reconstruit sous le format 2");
    assert_eq!(rapport["Note"]["restored"], 1, "{rapport}");
    let refus = rapport["Note"]["refused"].to_string();
    assert!(refus.contains("format 9") && refus.contains("plus récent"), "le fichier d'un format inconnu se dit : {rapport}");
    let lignes = lignes(&neuve);
    assert_eq!(lignes.len(), 1);
    assert_eq!(lignes[0][2], CypherValue::String("#moteur".into()), "le champ passé par la migration : {lignes:?}");
    drop(neuve);

    // Sans la migration, un fichier au format 1 ne se devine pas : il se dit,
    // fichier par fichier (le dossier, lui, est passé au format 2 en s'ouvrant
    // sous le gabarit qui le migre).
    let chemin = manifeste(dir.path(), "neuve2.json", "neuve2.rag3db", TOUT);
    avec_format(&chemin, json!({"version": 2}));
    let rapport = ouvrir(&chemin).rebuild_from_files().expect("le dossier au format 2 se lit");
    assert_eq!(rapport["Note"]["restored"], 0, "{rapport}");
    assert!(rapport["Note"]["refused"].to_string().contains("aucune migration"), "{rapport}");
}
