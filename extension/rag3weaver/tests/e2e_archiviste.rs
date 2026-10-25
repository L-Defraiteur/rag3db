//! E2E : **l'archiviste, lot M4** — un Markdown entre par `archive`, des apports L1
//! classés sortent (`docs/25-octobre-2026-memoire-longue/06-m4-l-archiviste.md`).
//!
//! Sur le vrai gabarit `memory` (ses signaux, donc un service d'embarquement :
//! `RAG3WEAVER_EMBED_SERVICE`), le modèle déclaré au manifeste étant un faux
//! serveur SSE local (`tests/common/fake_sse.rs`) qui répond, section après
//! section, le nom d'une fiche, `none`, puis `create` :
//!
//! - le document est gardé tel quel (la Source, à son chemin par défaut) ;
//! - la section rangée est un apport L1 de la fiche, relié, qui cite sa
//!   provenance `doc:<chemin>#<titre>` ; `cited_by` la rend ;
//! - la section `none` n'écrit rien ; la section `create` écrit une fiche
//!   `proposed` nommée par son titre, et son apport ;
//! - sans `models.llm`, la Source est gardée et chaque section est dite en erreur
//!   dans le rapport de l'étage de décision.
//!
//! Run with: ./run_e2e.sh --test e2e_archiviste

#![cfg(all(feature = "rag3db-native", feature = "openai-llm"))]

use std::path::Path;

use serde_json::{json, Value};

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::Rag3dbConnection;

use crate::common::fake_sse::{FakeServer, Reply};

fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Le gabarit `memory` dans une copie de `templates/`, l'extension vector du
/// bâti, et une retouche du manifeste (le modèle déclaré).
fn ouvrir(dir: &Path, retoucher: impl Fn(&mut Value)) -> Backend {
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
    copier(&crate_dir().join("templates"), &dir.join("templates"));
    let manifeste = dir.join("templates/backends/memory/backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["vector_extension"] = json!(crate::common::vector_extension());
    retoucher(&mut m);
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let prepared = PreparedBackend::load(&manifeste).expect("le gabarit memory charge");
    let (embedder, _) = prepared.connect_embedder().unwrap_or_else(|e| {
        panic!("le témoin embarque (bge-m3) : posez RAG3WEAVER_EMBED_SERVICE — un témoin sauté n'est pas vert ({e})")
    });
    prepared
        .open(Box::new(Rag3dbConnection::new(&dir.join("memoire.rag3db")).expect("la base")), Some(embedder))
        .expect("l'ouverture")
}

fn appel(b: &Backend, outil: &str, args: Value) -> Value {
    b.call(outil, args.clone()).unwrap_or_else(|e| panic!("{outil} {args} : {e}"))
}

/// Les trames d'un modèle qui répond une étiquette, sous le schéma strict du Decide.
fn etiquette(label: &str) -> Reply {
    Reply::Sse {
        frames: vec![
            format!(r#"{{"id":"a1","choices":[{{"index":0,"delta":{{"role":"assistant","content":"{{\"label\": \"{label}\"}}"}},"finish_reason":null}}]}}"#),
            r#"{"id":"a1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#.into(),
        ],
    }
}

const FICHE: &str = "le moteur de reprise du WAL";
const DOCUMENT: &str = "# Journal du 26\n\n## Reprise\n\nLe journal se rejoue à l'ouverture, en entier, avant toute écriture : le moteur de reprise du WAL relit chaque enregistrement.\n\n## Météo\n\nIl pleut sur la ville.\n\n## Le compositeur\n\nÀ 100 % de rapport cyclique, le compositeur figeait ; à 70 % l'écran reste utilisable.\n";
const CHEMIN: &str = "sources/2026-10-26-journal-du-26.md";

fn apports_de(b: &Backend, fiche: &str) -> Vec<Value> {
    let r = appel(b, "recall_apports", json!({"query": "journal compositeur pluie", "options": {"limit": 20, "consistency": "strict",
        "filter_condition": {"field": {"key": "fiche", "value": fiche}}}}));
    r["result"].as_array().cloned().unwrap_or_default().into_iter().map(|x| x["data"].clone()).collect()
}

#[test]
#[ignore]
fn un_document_entre_et_ses_sections_sont_rangees_une_par_fiche_none_ou_create() {
    let serveur = FakeServer::start_sequence(vec![etiquette(FICHE), etiquette("none"), etiquette("create")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    appel(&b, "put_fiche", json!({"record": {"name": FICHE, "about": "Comment le moteur rejoue son journal à l'ouverture.", "state": "current"}}));

    let r = appel(&b, "archive", json!({"titre": "Journal du 26", "contenu": DOCUMENT, "jour": "2026-10-26"}));
    // Le dernier étage (les liens et la provenance des sections rangées sur une fiche) : une section, sans erreur.
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    assert_eq!(r["result"]["elements"], json!(1), "une section rangée sur une fiche existante : {r}");
    let decide = &r["metadata"]["decide.report"];
    assert_eq!((decide["elements"].as_u64(), decide["ok"].as_u64()), (Some(3), Some(3)), "trois sections décidées : {r}");
    assert_eq!(serveur.connections.load(std::sync::atomic::Ordering::SeqCst), 3, "une décision par section");

    // Le L0, tel quel, à son chemin par défaut.
    let source = dir.path().join("templates/backends/memory/fichiers").join(CHEMIN);
    let texte = std::fs::read_to_string(&source).unwrap_or_else(|e| panic!("{} : {e}", source.display()));
    assert!(texte.contains("## Le compositeur") && texte.contains("origine: \"envoi\""), "{texte}");

    // « Reprise » : un apport L1 de la fiche, qui cite sa provenance.
    let apports = apports_de(&b, FICHE);
    assert_eq!(apports.len(), 1, "{apports:?}");
    assert_eq!(apports[0]["section"], json!("Reprise"));
    assert_eq!(apports[0]["source"], json!(CHEMIN));
    assert!(apports[0]["texte"].as_str().unwrap().contains("relit chaque enregistrement"));

    // « Le compositeur » : une fiche proposée, nommée par le titre, avec son apport.
    let proposee = appel(&b, "get_fiche", json!({"record": {"name": "Le compositeur"}}));
    assert_eq!(proposee["result"]["data"]["state"], json!("proposed"), "{proposee}");
    assert_eq!(proposee["result"]["data"]["via"], json!("archiviste"));
    assert_eq!(apports_de(&b, "Le compositeur").len(), 1);

    // « Météo » : rien d'écrit — la section reste dans le L0.
    let meteo = appel(&b, "recall_apports", json!({"query": "pleut ville", "options": {"limit": 5, "consistency": "strict"}}));
    assert!(meteo["result"].as_array().unwrap().iter().all(|x| x["data"]["section"] != json!("Météo")), "{meteo}");

    // cited_by rend les deux fiches depuis le source, chacune avec son apport.
    let c = appel(&b, "cited_by", json!({"value": CHEMIN, "format": "json"}));
    let fiches: Vec<Value> = serde_json::from_str(c["result"].as_str().expect("json")).unwrap();
    let noms: Vec<&str> = fiches.iter().map(|f| f["data"]["name"].as_str().unwrap()).collect();
    assert_eq!(fiches.len(), 2, "{noms:?}");
    assert!(noms.contains(&FICHE) && noms.contains(&"Le compositeur"), "{noms:?}");

    // Rejouer le même document passe (la Source est immuable et identique) ; un autre contenu au même chemin refuse.
    let e = b.call("archive", json!({"titre": "Journal du 26", "contenu": "# Autre\n\nAutre texte.\n", "jour": "2026-10-26"})).err().expect("un autre contenu au même chemin");
    assert!(e.contains("immu") || e.contains("chemin") || e.contains(CHEMIN), "{e}");
}

#[test]
#[ignore]
fn sans_modele_declare_le_document_est_garde_et_chaque_section_est_dite_en_erreur() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    let r = appel(&b, "archive", json!({"titre": "Journal du 26", "contenu": DOCUMENT, "jour": "2026-10-26"}));
    let decide = &r["metadata"]["decide.report"];
    assert_eq!(decide["ok"], json!(0), "{r}");
    let erreurs = decide["errors"].as_array().expect("errors");
    assert_eq!(erreurs.len(), 3, "{r}");
    assert!(erreurs.iter().all(|e| e["error"].as_str().unwrap_or("").contains("llm")), "{erreurs:?}");
    assert_eq!(r["result"]["elements"], json!(0), "rien n'est rangé : {r}");
    let source = dir.path().join("templates/backends/memory/fichiers").join(CHEMIN);
    assert!(source.exists(), "le L0 est gardé : {}", source.display());
}
