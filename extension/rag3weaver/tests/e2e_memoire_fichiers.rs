//! E2E : **la mémoire dans ses fichiers** (`docs/18-octobre-2026-memoire-longue/04-…` §4
//! et §5) — le gabarit `memory` câblé sur `files_root` :
//!
//! - une source s'écrit à son chemin (`sources/….md`), son contenu tel quel ; un genre de
//!   Ref à `genres/<nom>.md` ; une fiche à `fiches/<uuid>.md`, un fichier composé (son
//!   en-tête, `## État`, `## Ancres`, `## Apports`) réécrit par la réaction de miroir ;
//! - **le bloc machine se ferme** sur les lignes sans fichier à elles : la Ref `doc:`
//!   qu'un apport cite y voyage, et aucune colonne de vecteur n'y entre ;
//! - la vérification à blanc rend `ok`, vecteurs compris ;
//! - **une base perdue revient entière** depuis le dossier : sources, genres, fiches,
//!   apports, Refs, et leurs liens, sans lien compté vers une ligne absente.
//!
//! Sur le vrai gabarit (ses signaux : `RAG3WEAVER_EMBED_SERVICE`).
//!
//! Run with: ./run_e2e.sh --test e2e_memoire_fichiers

#![cfg(feature = "rag3db-native")]

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::Rag3dbConnection;

/// Le gabarit `memory` copié sous `dir/templates`, et un manifeste `nom` sur la base
/// `base` : deux manifestes du même dossier partagent les mêmes fichiers.
fn manifeste(dir: &Path, nom: &str, base: &str) -> PathBuf {
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
    if !dir.join("templates").exists() {
        copier(&Path::new(env!("CARGO_MANIFEST_DIR")).join("templates"), &dir.join("templates"));
    }
    let gabarit = dir.join("templates/backends/memory");
    let mut m: Value = serde_json::from_slice(&std::fs::read(gabarit.join("backend.json")).unwrap()).unwrap();
    m["vector_extension"] = json!(crate::common::vector_extension());
    m["database"] = json!(base);
    let chemin = gabarit.join(nom);
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    chemin
}

fn ouvrir(chemin: &Path) -> Backend {
    let prepared = PreparedBackend::load(chemin).expect("le gabarit memory charge");
    let base = prepared.path(&prepared.manifest.database);
    let (embedder, _) = prepared.connect_embedder().unwrap_or_else(|e| {
        panic!("le témoin embarque : posez RAG3WEAVER_EMBED_SERVICE — un témoin sauté n'est pas vert ({e})")
    });
    prepared.open(Box::new(Rag3dbConnection::new(&base).expect("la base")), Some(embedder)).expect("l'ouverture")
}

fn appel(b: &Backend, outil: &str, args: Value) -> Value {
    b.call(outil, args.clone()).unwrap_or_else(|e| panic!("{outil} {args} : {e}"))
}

const SOURCE: &str = "sources/2026-10-25-essai.md";
const CONTENU: &str = "# Le journal\n\n## Reprise\n\nLe journal se rejoue à l'ouverture, en entier, avant toute écriture.\n";
const FICHE: &str = "le moteur de reprise du WAL";
const ANCRE: &str = "src/storage/wal/replay.cpp";

/// Une source, une fiche, son apport rangé, sa provenance `doc:`, une ancre `path`.
/// Rend l'uuid de la fiche.
fn ecrire(b: &Backend) -> String {
    for (genre, description) in [("doc", "un document source et sa section"), ("path", "un chemin du dépôt")] {
        appel(b, "create_ref_type", json!({"name": genre, "description": description, "forme": "relatif", "confirm": true}));
    }
    appel(b, "put_source", json!({"record": {"chemin": SOURCE, "titre": "Le journal", "contenu": CONTENU, "origine": "envoi"}}));
    let fiche = appel(b, "put_fiche", json!({"record": {"name": FICHE, "about": "Comment le moteur rejoue son journal à l'ouverture.", "state": "current"}}));
    let uuid = fiche["result"]["uuid"].as_str().unwrap_or_else(|| panic!("l'uuid de la fiche : {fiche}")).to_string();
    let cle = json!({"fiche": FICHE, "source": SOURCE, "section": "Reprise"});
    let mut apport = cle.clone();
    apport["texte"] = json!("Le journal se rejoue à l'ouverture, en entier, avant toute écriture.");
    apport["niveau"] = json!("L1");
    apport["state"] = json!("current");
    appel(b, "put_apport", json!({"record": apport}));
    appel(b, "relier_apport", json!({"links": [{"from": {"name": FICHE}, "to": cle}]}));
    appel(b, "citer_source", json!({"genre": "doc", "valeur": format!("{SOURCE}#Reprise"), "fiche": cle}));
    appel(b, "ancrer_fiche", json!({"genre": "path", "valeur": ANCRE, "fiche": {"name": FICHE}}));
    uuid
}

/// Le fichier de la fiche, une fois que la réaction de miroir y a tout posé.
fn fichier_de_la_fiche(dir: &Path, uuid: &str) -> String {
    let chemin = dir.join("templates/backends/memory/fichiers/fiches").join(format!("{uuid}.md"));
    let debut = std::time::Instant::now();
    loop {
        let texte = std::fs::read_to_string(&chemin).unwrap_or_default();
        if texte.contains(ANCRE) && texte.contains("Reprise") && texte.contains("APPORT_CITE") {
            return texte;
        }
        assert!(debut.elapsed() < std::time::Duration::from_secs(10), "la fiche n'est pas complète après 10 s : {texte}");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn compter(b: &Backend, q: &str) -> i64 {
    b.catalog.lock().unwrap().execute_raw_with_params(q, &[]).unwrap_or_else(|e| panic!("{q} : {e}")).rows[0][0].as_i64().unwrap()
}

#[test]
#[ignore]
fn la_memoire_s_ecrit_dans_des_fichiers_lisibles_et_son_bloc_se_ferme() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(&manifeste(dir.path(), "backend.json", "memoire.rag3db"));
    let uuid = ecrire(&b);
    let racine = dir.path().join("templates/backends/memory/fichiers");

    let source = std::fs::read_to_string(racine.join(SOURCE)).expect("la source à son chemin");
    assert!(source.ends_with(CONTENU) || source.contains(CONTENU), "le contenu tel quel : {source}");
    for genre in ["doc", "path"] {
        let t = std::fs::read_to_string(racine.join("genres").join(format!("{genre}.md"))).expect("le fichier du genre");
        assert!(t.contains(&format!("name: \"{genre}\"")), "{t}");
    }

    let fiche = fichier_de_la_fiche(dir.path(), &uuid);
    assert!(fiche.starts_with("---\n"), "un en-tête YAML : {fiche}");
    for attendu in [format!("# {FICHE}"), "## État".into(), "## Ancres".into(), format!("`{ANCRE}`"), "## Apports".into(),
                    "### Reprise".into(), format!("[{SOURCE}](../{SOURCE})"), "UTC".into()] {
        assert!(fiche.contains(&attendu), "la fiche dit « {attendu} » : {fiche}");
    }
    // Le dossier se décrit : son gabarit, son format, les genres, les relations et leur sens.
    drop(b);
    let b = ouvrir(&dir.path().join("templates/backends/memory/backend.json"));
    let dossier = std::fs::read_to_string(racine.join("manifeste.yaml")).expect("le manifeste du dossier");
    for attendu in ["nom: memory", "format: 1", "genres:", "- doc", "- path", "ANCRE:", "une fiche est ancrée"] {
        assert!(dossier.contains(attendu), "le manifeste du dossier dit « {attendu} » : {dossier}");
    }
    drop(b);
    let bloc = &fiche[fiche.find(rag3weaver::fichiers_entite::DEBUT_DU_BLOC).expect("le bloc machine")..];
    assert!(bloc.contains(&format!("{SOURCE}#Reprise")), "la Ref doc: citée par l'apport voyage dans le bloc : {bloc}");
    assert!(!bloc.contains("embedding"), "aucune colonne de vecteur dans le bloc : {bloc}");
}

#[test]
#[ignore]
fn la_verification_a_blanc_d_une_memoire_a_vecteurs_rend_ok() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(&manifeste(dir.path(), "backend.json", "memoire.rag3db"));
    let uuid = ecrire(&b);
    fichier_de_la_fiche(dir.path(), &uuid);
    let r = b.verify_files().expect("la vérification");
    assert_eq!(r["ok"], true, "le dossier se reconstruit à l'identique, vecteurs mis à part : {r}");
    for e in ["Source", "RefType", "Fiche", "Apport", "Ref"] {
        assert!(r["entities"][e]["rows"].as_u64().unwrap_or(0) > 0, "{e} est comparée : {r}");
    }
    for rel in ["A_POUR_APPORT", "APPORT_CITE", "ANCRE"] {
        assert_eq!(r["relations"][rel]["pairs"], 1, "{rel} : {r}");
    }
}

#[test]
#[ignore]
fn une_memoire_perdue_revient_entiere_depuis_son_dossier() {
    let dir = tempfile::tempdir().expect("dossier");
    let vivante = ouvrir(&manifeste(dir.path(), "backend.json", "memoire.rag3db"));
    let uuid = ecrire(&vivante);
    fichier_de_la_fiche(dir.path(), &uuid);
    let requetes = [
        "MATCH (s:Source) RETURN count(*)",
        "MATCH (t:RefType) RETURN count(*)",
        "MATCH (f:Fiche)-[:A_POUR_APPORT]->(a:Apport {niveau: 'L1'}) RETURN count(*)",
        "MATCH (a:Apport)-[:APPORT_CITE]->(r:Ref {genre: 'doc'}) RETURN count(*)",
        "MATCH (f:Fiche)-[:ANCRE]->(r:Ref {genre: 'path'}) RETURN count(*)",
    ];
    let avant: Vec<i64> = requetes.iter().map(|q| compter(&vivante, q)).collect();
    assert_eq!(avant, [1, 2, 1, 1, 1]);
    drop(vivante);

    let neuve = ouvrir(&manifeste(dir.path(), "neuve.json", "neuve.rag3db"));
    let rapport = neuve.rebuild_from_files().expect("la reconstruction");
    assert_eq!(rapport["links"]["to_absent_rows"], json!({}), "aucun lien vers une ligne absente : {rapport}");
    assert_eq!(rapport["links"]["refused"], json!([]), "{rapport}");
    let apres: Vec<i64> = requetes.iter().map(|q| compter(&neuve, q)).collect();
    assert_eq!(apres, avant, "la mémoire revient entière : {rapport}");
}
