//! E2E : **l'archiviste, lot M4** — un Markdown entre par `archive`, des apports L1
//! classés sortent (`docs/25-octobre-2026-memoire-longue/06-m4-l-archiviste.md`).
//!
//! Sur le vrai gabarit `memory` (ses signaux, donc un service d'embarquement :
//! `RAG3WEAVER_EMBED_SERVICE`), le modèle déclaré au manifeste étant un faux
//! serveur SSE local (`tests/common/fake_sse.rs`) qui répond, section après
//! section, l’alias d’une fiche, `none`, puis `create` :
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

/// Une étiquette et un nom, sous le schéma strict du Decide quand il demande un nom (`name_as`).
/// Une étiquette, un nom et une phrase qui dit le sujet (`about_as`, voie create).
fn etiquette_decrite(label: &str, name: &str, about: &str) -> Reply {
    Reply::Sse {
        frames: vec![
            format!(r#"{{"id":"a1","choices":[{{"index":0,"delta":{{"role":"assistant","content":"{{\"label\": \"{label}\", \"name\": \"{name}\", \"about\": \"{about}\"}}"}},"finish_reason":null}}]}}"#),
            r#"{"id":"a1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#.into(),
        ],
    }
}

fn etiquette_nommee(label: &str, name: &str) -> Reply {
    Reply::Sse {
        frames: vec![
            format!(r#"{{"id":"a1","choices":[{{"index":0,"delta":{{"role":"assistant","content":"{{\"label\": \"{label}\", \"name\": \"{name}\"}}"}},"finish_reason":null}}]}}"#),
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
    let serveur = FakeServer::start_sequence(vec![etiquette("candidate_1"), etiquette("none"), etiquette("create")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    appel(&b, "put_fiche", json!({"record": {"name": FICHE, "about": "Comment le moteur rejoue son journal à l'ouverture.", "state": "current"}}));

    // La création est éteinte par défaut (tentative 5 des fiches classées) : le témoin l'allume.
    let r = appel(&b, "archive", json!({"titre": "Journal du 26", "contenu": DOCUMENT, "jour": "2026-10-26", "allow_create": true}));
    // Le résultat est le rapport des décisions : trois sections, aucune en erreur.
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    assert_eq!((r["result"]["elements"].as_u64(), r["result"]["ok"].as_u64()), (Some(3), Some(3)), "trois sections décidées : {r}");
    assert_eq!(serveur.connections.load(std::sync::atomic::Ordering::SeqCst), 3, "une décision par section");

    let request: Value = serde_json::from_str(&serveur.request.recv().unwrap()).unwrap();
    assert_eq!(request["response_format"]["json_schema"]["schema"]["properties"]["label"]["enum"],
        json!(["candidate_1", "none", "create"]));
    assert!(request["messages"].to_string().contains(FICHE), "le nom exact reste présenté au modèle");

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
    let decide = &r["result"];
    assert_eq!(decide["ok"], json!(0), "{r}");
    let erreurs = decide["errors"].as_array().expect("errors");
    assert_eq!(erreurs.len(), 3, "{r}");
    assert!(erreurs.iter().all(|e| e["error"].as_str().unwrap_or("").contains("llm")), "{erreurs:?}");
    let rien = appel(&b, "recall_apports", json!({"query": "journal compositeur pluie", "options": {"limit": 5, "consistency": "strict"}}));
    assert_eq!(rien["result"].as_array().map(|a| a.len()), Some(0), "rien n'est rangé : {rien}");
    let source = dir.path().join("templates/backends/memory/fichiers").join(CHEMIN);
    assert!(source.exists(), "le L0 est gardé : {}", source.display());
}


fn script_archive(script: &str, input: Value) -> Result<Value, String> {
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("input", rhai::serde::to_dynamic(input).unwrap());
    let output = rhai::Engine::new()
        .eval_with_scope::<rhai::Dynamic>(&mut scope, script)
        .map_err(|e| e.to_string())?;
    rhai::serde::from_dynamic(&output).map_err(|e| e.to_string())
}

fn preparer_alias(noms: &[&str]) -> Value {
    script_archive(include_str!("../templates/backends/memory/scripts/archive_candidates.rhai"), json!({
        // Au moins six mots : une création sans phrase qui dit le sujet est refusée (14 novembre 2026).
        "section": {"title": "Reprise", "text": "Le journal se rejoue à l'ouverture, en entier.", "chemin": CHEMIN, "index": 0},
        "candidates": 5, "allow_create": true,
        "value": noms.iter().map(|name| json!({"data": {"name": name, "about": "Description conservée"}})).collect::<Vec<_>>()
    })).unwrap()
}

fn resoudre_alias(section: Value) -> Result<Value, String> {
    script_archive(include_str!("../templates/backends/memory/scripts/archive_decided.rhai"),
        json!({"value": section, "allow_create": true}))
}

#[test]
#[ignore]
fn les_alias_courts_resolvent_le_nom_exact_sans_changer_l_identite() {
    let long = format!("bilan : {} fin é\"\\", "mémoire ".repeat(320));
    // Un nom réservé et un nom qui ressemble à un alias restent des identités ordinaires.
    let noms = [long.as_str(), "none", "create", "candidate_1"];
    let prepared = preparer_alias(&noms);
    assert_eq!(prepared["candidates"], json!(["candidate_1", "candidate_2", "candidate_3", "candidate_4"]));
    for (i, nom) in noms.iter().enumerate() {
        assert!(prepared["prompt"].as_str().unwrap().contains(nom));
        let mut section = prepared.clone();
        section["verdict"] = json!(format!("candidate_{}", i + 1));
        let out = resoudre_alias(section).unwrap();
        assert_eq!(out["kind"], "fiche");
        assert_eq!(out["fiche"], *nom);
        assert_eq!(out["verdict"], *nom);
        assert_eq!(out["apport_key"]["fiche"], *nom);
        assert_eq!(out["apport_record"]["fiche"], *nom);
        assert_eq!(out["links"][0]["from"]["name"], *nom);
    }
    assert!(prepared["prompt"].as_str().unwrap().contains("Description conservée"));
}

#[test]
#[ignore]
fn un_alias_inconnu_ou_ambigu_est_refuse_explicitement() {
    let prepared = preparer_alias(&["une fiche", "une autre"]);
    let mut unknown = prepared.clone();
    unknown["verdict"] = json!("candidate_99");
    let err = resoudre_alias(unknown).unwrap_err();
    assert!(err.contains("inconnu") && err.contains("candidate_99"), "{err}");
    let mut ambiguous = prepared.clone();
    // La liste de résolution peut être corrompue : aucun choix arbitraire n'est permis.
    ambiguous["candidate_names"][1]["alias"] = json!("candidate_1");
    ambiguous["verdict"] = json!("candidate_1");
    let err = resoudre_alias(ambiguous).unwrap_err();
    assert!(err.contains("ambigu") && err.contains("candidate_1"), "{err}");
    let mut missing = prepared;
    missing["candidate_names"] = json!([]);
    missing["verdict"] = json!("candidate_1");
    let err = resoudre_alias(missing).unwrap_err();
    assert!(err.contains("inconnu"), "{err}");
}

#[test]
#[ignore]
fn les_noms_dupliques_ne_creent_pas_d_alias_ambigus_et_les_decisions_fixes_restent() {
    let prepared = preparer_alias(&["même nom", "même nom"]);
    assert_eq!(prepared["candidates"], json!(["candidate_1"]));
    for decision in ["none", "create"] {
        let mut section = prepared.clone();
        section["verdict"] = json!(decision);
        assert_eq!(resoudre_alias(section).unwrap()["kind"], decision);
    }
}

/// **Une section voit les fiches que les sections précédentes du même document viennent de
/// proposer** (13 novembre 2026, l'archivage à blanc des docs de la semaine : 326 fiches pour 432
/// sections, et aucune qui réunisse deux sections d'un même document ; ticket
/// `2026-11-13-une-section-ne-voit-pas-les-fiches-proposees-plus-haut-dans-son-document`). Deux
/// sections sur le même sujet : le faux modèle crée pour la première, puis choisit, pour la seconde,
/// la seule candidate qui puisse exister (la base est vide), la fiche proposée par la première. Une
/// seule fiche, deux apports.
#[test]
#[ignore]
fn deux_sections_d_un_document_sur_le_meme_sujet_font_une_seule_fiche() {
    let serveur = FakeServer::start_sequence(vec![etiquette("create"), etiquette("candidate_1")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    const NOTES: &str = "# Notes du 13\n\n## La reprise du WAL\n\nLe journal se rejoue à l'ouverture, en entier.\n\n## La reprise, suite\n\nLe moteur de reprise relit chaque enregistrement avant toute écriture.\n";
    let r = appel(&b, "archive", json!({"titre": "Notes du 13", "contenu": NOTES, "jour": "2026-11-13", "allow_create": true}));
    assert_eq!(r["result"]["errors"], json!([]), "les deux sections décidées sans erreur : {r}");
    assert_eq!(r["result"]["ok"], json!(2), "{r}");
    assert_eq!(apports_de(&b, "La reprise du WAL").len(), 2, "les deux sections sur la fiche proposée par la première");
    assert!(apports_de(&b, "La reprise, suite").is_empty(), "aucune seconde fiche");
}

/// **Une fiche créée porte le nom d'un sujet, pas le titre de sa section** (13 novembre 2026,
/// l'archivage à blanc : « 1. Le jeu », « Le témoin », « En file » devenaient des fiches). Sur la voie
/// create, le modèle rend aussi un nom (schéma `{label, name}`) ; la fiche prend ce nom.
#[test]
#[ignore]
fn une_fiche_creee_porte_le_nom_du_sujet_que_le_modele_donne() {
    let serveur = FakeServer::start_sequence(vec![etiquette_nommee("create", "La reprise du journal")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    const NOTE: &str = "# Note du 13\n\n## 1. Le jeu\n\nLe journal se rejoue à l'ouverture, en entier, avant toute écriture.\n";
    let r = appel(&b, "archive", json!({"titre": "Note du 13", "contenu": NOTE, "jour": "2026-11-13", "allow_create": true}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    let request: Value = serde_json::from_str(&serveur.request.recv().unwrap()).unwrap();
    assert!(request["response_format"]["json_schema"]["schema"]["properties"].get("name").is_some(), "le schéma demande un nom : {request}");
    assert_eq!(apports_de(&b, "La reprise du journal").len(), 1, "la fiche porte le nom du sujet");
    assert!(apports_de(&b, "1. Le jeu").is_empty(), "pas le titre de la section");
}

/// **Le même sujet sous deux formulations, dans deux documents, fait une seule fiche** (13 novembre
/// 2026, le second archivage à blanc : « Hooks Claude Code », « Hooks Claude Code de rag3weaver » et
/// « Hooks de Claude Code », trois fiches de 11 à 12 sections chacune). Le premier document crée
/// « Hooks Claude Code » ; le second, sans aucune candidate présentée (`candidates: 0`), répond create
/// et « Hooks de Claude Code » : seule la recherche par le nom de sujet peut retrouver la première.
#[test]
#[ignore]
fn le_meme_sujet_sous_deux_formulations_fait_une_seule_fiche() {
    let serveur = FakeServer::start_sequence(vec![
        etiquette_nommee("create", "Hooks Claude Code"),
        etiquette_nommee("create", "Hooks de Claude Code"),
    ]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    const A: &str = "# Lot 1\n\n## La commande mince\n\nLes hooks de Claude Code appellent l'hôte par une commande mince, qui sort toujours en zéro.\n";
    const B: &str = "# Lot 2\n\n## Les blocs de lecture\n\nÀ chaque prompt, les hooks de Claude Code glissent une fiche de la mémoire dans la session.\n";
    let r = appel(&b, "archive", json!({"titre": "Lot 1", "contenu": A, "jour": "2026-11-07", "allow_create": true}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    let r = appel(&b, "archive", json!({"titre": "Lot 2", "contenu": B, "jour": "2026-11-07", "allow_create": true, "candidates": 0}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    assert_eq!(apports_de(&b, "Hooks Claude Code").len(), 2, "le second document rejoint la fiche du premier");
    assert!(apports_de(&b, "Hooks de Claude Code").is_empty(), "aucune fiche voisine de plus");
}

/// **Dans un même document, un sujet voisin d'une fiche proposée plus haut la rejoint**, et un sujet
/// d'un seul mot contenu dans un autre ne rejoint rien. La première section crée « Hooks Claude Code » ;
/// la deuxième répond create avec « Hooks de Claude Code » (le même jeu de mots, les mots vides ôtés) :
/// elle rejoint la première. La troisième répond create avec « Hooks » : un seul mot, elle fait sa fiche.
#[test]
#[ignore]
fn un_sujet_voisin_d_une_fiche_proposee_plus_haut_la_rejoint() {
    let serveur = FakeServer::start_sequence(vec![
        etiquette_nommee("create", "Hooks Claude Code"),
        etiquette_nommee("create", "Hooks de Claude Code"),
        etiquette_nommee("create", "Hooks"),
    ]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    const NOTES: &str = "# Lot 3\n\n## La commande mince\n\nLes hooks de Claude Code appellent l'hôte par une commande mince.\n\n## Les blocs\n\nÀ chaque prompt, les hooks de Claude Code glissent une fiche.\n\n## Ailleurs\n\nUn crochet de git, tout autre chose.\n";
    let r = appel(&b, "archive", json!({"titre": "Lot 3", "contenu": NOTES, "jour": "2026-11-13", "allow_create": true}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    assert_eq!(apports_de(&b, "Hooks Claude Code").len(), 2, "la deuxième section rejoint la première");
    assert!(apports_de(&b, "Hooks de Claude Code").is_empty(), "aucune fiche voisine de plus");
    assert_eq!(apports_de(&b, "Hooks").len(), 1, "un seul mot ne rejoint pas une fiche plus longue");
}

/// **Un document journal donne une fiche par chantier, pas par moment** (14 novembre 2026, Lucie, sur
/// le troisième archivage à blanc : « Relance du 7 novembre 2026 », « Matinée du 8 novembre 2026 »,
/// « Nuit du 7 au 8 novembre 2026 »). L'instruction dit au modèle de nommer le chantier ; un nom de
/// fiche neuve qui porte une date (un mois et un nombre) est refusé, et le refus se dit dans le rapport.
/// Une fiche déjà là reste joignable quel que soit son nom.
#[test]
#[ignore]
fn un_journal_donne_une_fiche_par_chantier_pas_par_moment() {
    let serveur = FakeServer::start_sequence(vec![
        etiquette_nommee("create", "Matinée du 8 novembre 2026"),
        etiquette_nommee("create", "Bascule au format 43"),
    ]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    const JOURNAL: &str = "# Journal du 8 novembre 2026\n\n## 9 h 10\n\nLe banc a repris ses mesures après la nuit.\n\n## 11 h 40\n\nLa bascule au format 43 passe les témoins du cœur.\n";
    let r = appel(&b, "archive", json!({"titre": "Journal du 8 novembre 2026", "contenu": JOURNAL, "jour": "2026-11-08", "allow_create": true}));
    let request: Value = serde_json::from_str(&serveur.request.recv().unwrap()).unwrap();
    assert!(request["messages"].to_string().contains("chantier"), "l'instruction demande le chantier : {}", request["messages"]);
    assert_eq!(r["result"]["ok"], json!(1), "{r}");
    let erreurs = r["result"]["errors"].to_string();
    assert!(erreurs.contains("Matinée du 8 novembre 2026") && erreurs.contains("date"), "le refus se dit : {r}");
    assert!(apports_de(&b, "Matinée du 8 novembre 2026").is_empty(), "aucune fiche nommée par un moment");
    assert_eq!(apports_de(&b, "Bascule au format 43").len(), 1, "le chantier fait sa fiche");
    let f = appel(&b, "get_fiche", json!({"record": {"name": "Bascule au format 43"}}));
    assert_eq!(f["result"]["data"]["state"], json!("proposed"), "une fiche créée reste proposée : {f}");
}

/// **Une fiche courante qu'une section rejoint reste courante** (14 novembre 2026, le premier garde-fou
/// de la voie 3 : l'archiviste crée en `proposed`, et rien d'autre que la relecture de Lucie — la
/// transition `accept` — ne fait une fiche `current`). La section répond create avec le nom exact d'une
/// fiche `current` qu'aucune candidate ne présente : elle la rejoint, et l'état ne bouge pas.
#[test]
#[ignore]
fn une_fiche_courante_rejointe_reste_courante() {
    let serveur = FakeServer::start_sequence(vec![etiquette_nommee("create", FICHE)]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    appel(&b, "put_fiche", json!({"record": {"name": FICHE, "about": "Comment le moteur rejoue son journal à l'ouverture.", "state": "current"}}));
    const NOTE: &str = "# Note\n\n## Le rejeu\n\nLe journal se rejoue en entier à l'ouverture.\n";
    let r = appel(&b, "archive", json!({"titre": "Note", "contenu": NOTE, "jour": "2026-11-14", "allow_create": true, "candidates": 0}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    assert_eq!(apports_de(&b, FICHE).len(), 1, "la section rejoint la fiche");
    let f = appel(&b, "get_fiche", json!({"record": {"name": FICHE}}));
    assert_eq!(f["result"]["data"]["state"], json!("current"), "{f}");
}

/// **La recherche par nom est exacte, pas un classement tronqué** (14 novembre 2026, la première passe
/// sur la mémoire de Lucie : quatre paires de noms voisins sont restées séparées, la voisine se
/// classant de la 5e à la 38e place sur 116 alors que l'étage n'en lisait que 8). 119 fiches leurres
/// (« Claude Code de rag3weaver, essai k ») partagent trois mots avec le nom proposé sans en être
/// voisines, et passent devant dans un classement ; une seule fiche,
/// « Hooks de rag3weaver », l'est. La section, sans candidate, répond create avec « hooks Claude Code
/// de rag3weaver » : elle rejoint la voisine, où qu'elle se classe. Les options de la recherche des
/// candidates (`limit: 1` ici) ne bornent pas la recherche par le nom.
#[test]
#[ignore]
fn la_recherche_par_nom_trouve_la_voisine_parmi_cent_vingt_fiches() {
    let serveur = FakeServer::start_sequence(vec![etiquette_nommee("create", "hooks Claude Code de rag3weaver")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    appel(&b, "put_fiche", json!({"record": {"name": "Hooks de rag3weaver", "about": "Une note brève, sans autre mot du sujet.", "state": "proposed"}}));
    for k in 1..=119 {
        appel(&b, "put_fiche", json!({"record": {"name": format!("Claude Code de rag3weaver, essai {k}"),
            "about": format!("Les hooks Claude Code de rag3weaver, essai {k}."), "state": "proposed"}}));
    }
    const NOTE: &str = "# Lot 4\n\n## Le binaire des hooks\n\nLes hooks Claude Code de rag3weaver passent par une commande mince.\n";
    let r = appel(&b, "archive", json!({"titre": "Lot 4", "contenu": NOTE, "jour": "2026-11-14", "allow_create": true, "candidates": 0,
        "search_options": {"limit": 1}}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    assert_eq!(apports_de(&b, "Hooks de rag3weaver").len(), 1, "la section rejoint la voisine");
    assert!(apports_de(&b, "hooks Claude Code de rag3weaver").is_empty(), "aucune fiche voisine de plus");
}

/// **Une fiche créée porte une phrase qui dit son sujet** (14 novembre 2026, la première passe sur la
/// mémoire de Lucie : des fiches décrites par « 1. », « Session archi, 9 novembre 2026. », « Accepté par
/// c7. » — la première phrase de la section, coupée au premier point). Le modèle rend aussi `about`, une
/// phrase qui dit ce que la fiche couvre ; une phrase de moins de six mots est écartée, et la fiche prend
/// alors la première phrase de la section qui en a au moins six.
#[test]
#[ignore]
fn une_fiche_creee_porte_une_phrase_qui_dit_son_sujet() {
    let serveur = FakeServer::start_sequence(vec![
        etiquette_decrite("create", "Le jeu des clés", "Comment les sessions du poste se partagent les clés de leurs dépôts."),
        etiquette_decrite("create", "Le rappel tiré du prompt", "Accepté par c7."),
    ]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    const NOTE: &str = "# Note\n\n## 1. Le jeu\n\n1. Le jeu des clés se joue à deux.\n\n## Le rappel\n\nAccepté par c7. Le rappel tire ses mots du prompt de la session, sans redite d'un tour à l'autre.\n";
    let r = appel(&b, "archive", json!({"titre": "Note", "contenu": NOTE, "jour": "2026-11-14", "allow_create": true}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    let request: Value = serde_json::from_str(&serveur.request.recv().unwrap()).unwrap();
    assert!(request["response_format"]["json_schema"]["schema"]["properties"]["about"].is_object(), "le schéma demande about : {}", request["response_format"]);
    let f = appel(&b, "get_fiche", json!({"record": {"name": "Le jeu des clés"}}));
    assert_eq!(f["result"]["data"]["about"], json!("Comment les sessions du poste se partagent les clés de leurs dépôts."), "{f}");
    let f = appel(&b, "get_fiche", json!({"record": {"name": "Le rappel tiré du prompt"}}));
    assert_eq!(f["result"]["data"]["about"], json!("Le rappel tire ses mots du prompt de la session, sans redite d'un tour à l'autre."), "une phrase trop courte est écartée : {f}");
}

/// **Confirmer une fiche proposée, d'un appel** (14 novembre 2026, les gestes de Lucie : la page de
/// survol donne l'uuid de chaque fiche que l'archiviste propose). `confirmer_fiche(uuid)` applique la
/// transition `accept` — proposed → current — et rend l'estampille (révision, date, auteur posé par
/// l'hôte). Une fiche qui n'est pas proposée est refusée en le disant, un uuid inconnu aussi.
#[test]
#[ignore]
fn une_fiche_proposee_se_confirme_par_son_uuid() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    let f = appel(&b, "put_fiche", json!({"record": {"name": "Le jeu des clés", "about": "Comment les sessions du poste se partagent les clés de leurs dépôts.", "state": "proposed"}}));
    let uuid = f["result"]["uuid"].as_str().expect("l'uuid").to_string();
    let r = appel(&b, "confirmer_fiche", json!({"uuid": uuid}));
    assert_eq!(r["result"]["state"], json!("current"), "{r}");
    assert_eq!(r["result"]["revision"], json!(2), "la confirmation est une écriture de plus : {r}");
    let lue = appel(&b, "get_fiche", json!({"record": {"name": "Le jeu des clés"}}));
    assert_eq!(lue["result"]["data"]["state"], json!("current"), "{lue}");
    let e = b.call("confirmer_fiche", json!({"uuid": uuid})).err().expect("une fiche déjà courante ne se confirme pas");
    assert!(e.contains("proposed") && e.contains("current"), "le refus dit l'état : {e}");
    let e = b.call("confirmer_fiche", json!({"uuid": "00000000-0000-0000-0000-000000000000"})).err().expect("un uuid inconnu est refusé");
    assert!(e.contains("aucune fiche"), "{e}");
}

/// **La trace de l'archiviste** (14 novembre 2026, le jeu de banc pour juger un décideur sur les
/// décisions de l'archiviste) : `trace: true` fait rendre, par section, le texte, les candidates dans
/// l'ordre présenté (alias, nom, résumé), l'étiquette brute du modèle, le sujet proposé, la voie du
/// rangement et le kind décidé. Rien n'en est écrit dans la base ; sans `trace`, le rapport ne change pas.
#[test]
#[ignore]
fn la_trace_rend_par_section_les_candidates_et_l_etiquette_brute() {
    let serveur = FakeServer::start_sequence(vec![etiquette("candidate_1"), etiquette_decrite("create", "Hooks de Claude Code", "Les crochets de Claude Code qui appellent la mémoire de rag3weaver."), etiquette("none")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    appel(&b, "put_fiche", json!({"record": {"name": FICHE, "about": "Comment le moteur rejoue son journal à l'ouverture.", "state": "current"}}));
    const NOTE: &str = "# Note\n\n## Reprise\n\nLe journal se rejoue à l'ouverture, en entier, avant toute écriture.\n\n## Les crochets\n\nLes hooks de Claude Code appellent la mémoire à chaque prompt de la session.\n";
    let r = appel(&b, "archive", json!({"titre": "Note", "contenu": NOTE, "jour": "2026-11-14", "allow_create": true, "trace": true}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    let t = r["result"]["results"].as_array().unwrap_or_else(|| panic!("la trace : {r}"));
    assert_eq!(t.len(), 2, "{r}");
    assert_eq!(t[0]["section"], json!("Reprise"), "{r}");
    assert!(t[0]["texte"].as_str().unwrap().contains("se rejoue"), "{r}");
    assert_eq!(t[0]["etiquette"], json!("candidate_1"), "l'étiquette brute du modèle : {r}");
    assert_eq!(t[0]["candidates"][0]["alias"], json!("candidate_1"), "{r}");
    assert_eq!(t[0]["candidates"][0]["nom"], json!(FICHE), "{r}");
    assert!(t[0]["candidates"][0]["resume"].as_str().unwrap().contains("rejoue"), "{r}");
    assert_eq!(t[0]["voie"], json!("candidate"), "{r}");
    assert_eq!(t[0]["kind"], json!("fiche"), "{r}");
    assert_eq!(t[1]["etiquette"], json!("create"), "{r}");
    assert_eq!(t[1]["sujet"], json!("Hooks de Claude Code"), "{r}");
    assert_eq!(t[1]["kind"], json!("create"), "{r}");
    let sans = appel(&b, "archive", json!({"titre": "Note 2", "contenu": "# Note 2\n\n## Rien\n\nUne section de plus, sans aucun rapport avec le reste.\n", "jour": "2026-11-14"}));
    assert!(sans["result"].get("results").is_none(), "sans trace, le rapport ne change pas : {sans}");
}

/// **Le plafond de jetons du Decide de l'archiviste est déclaré dans son graphe** (15 novembre 2026, les
/// rejeux avec trace : 1 section sur 432 et 1 sur 152 tronquées au plafond par défaut de 2048, depuis
/// que la réponse porte un nom et une phrase, et que Gemini y compte sa réflexion). La requête envoyée
/// au modèle porte `max_tokens: 4096`, celui qu'`archive_decide.mmd` déclare.
#[test]
#[ignore]
fn le_decide_de_l_archiviste_demande_le_plafond_declare() {
    let serveur = FakeServer::start_sequence(vec![etiquette("none")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    const NOTE: &str = "# Note\n\n## Une section\n\nUne section de plus, sans rapport avec aucune fiche existante.\n";
    let r = appel(&b, "archive", json!({"titre": "Note", "contenu": NOTE, "jour": "2026-11-15", "allow_create": true}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    let request: Value = serde_json::from_str(&serveur.request.recv().unwrap()).unwrap();
    assert_eq!(request["max_tokens"], json!(4096), "le plafond déclaré par archive_decide.mmd : {}", request["max_tokens"]);
}

/// **Le contexte d'un envoi est montré au modèle et jamais rangé** (15 novembre 2026, le banc
/// des fenêtres longues) : `context` porte ce qui précède (les résumés des fenêtres d'avant, le
/// chevauchement, « fenêtre n/N ») ; il est dans le prompt de chaque Decide, avant la section, et
/// n'est écrit ni dans le L0 ni dans aucun L1.
#[test]
#[ignore]
fn le_contexte_d_un_envoi_est_montre_au_decide_et_jamais_range() {
    const CONTEXTE: &str = "fenêtre 2 sur 3, tours 40-79 ; CONTEXTEMARQUE la fenêtre d'avant a rangé la reprise du journal.";
    let serveur = FakeServer::start_sequence(vec![etiquette_decrite("create", "La reprise du journal", "Comment le moteur rejoue son journal à l’ouverture."), etiquette("none"), etiquette("none")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    let r = appel(&b, "archive", json!({"titre": "Journal du 26", "contenu": DOCUMENT, "jour": "2026-10-26", "allow_create": true, "context": CONTEXTE}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    // Dans chaque Decide, avant la section.
    for i in 0..3 {
        let messages = serde_json::from_str::<Value>(&serveur.request.recv().unwrap()).unwrap()["messages"].to_string();
        let (c, s) = (messages.find("CONTEXTEMARQUE"), messages.find("Section «"));
        assert!(c.is_some() && s.is_some() && c < s, "Decide {i} : le contexte est dans le prompt, avant la section : {messages}");
    }
    // Ni dans le L0, ni dans un L1.
    let source = dir.path().join("templates/backends/memory/fichiers").join(CHEMIN);
    let l0 = std::fs::read_to_string(&source).unwrap_or_else(|e| panic!("{} : {e}", source.display()));
    assert!(!l0.contains("CONTEXTEMARQUE"), "le contexte n'est pas dans le L0 : {l0}");
    let apports = apports_de(&b, "La reprise du journal");
    assert_eq!(apports.len(), 1, "{apports:?}");
    assert!(apports.iter().all(|a| !a.to_string().contains("CONTEXTEMARQUE")), "le contexte n'est dans aucun L1 : {apports:?}");
}

/// Une réponse du faux serveur suivie du bloc `usage` du protocole OpenAI (le dernier fragment).
fn avec_usage(r: Reply, prompt: u64, cached: u64, completion: u64) -> Reply {
    match r {
        Reply::Sse { mut frames } => {
            frames.push(format!(r#"{{"id":"a1","choices":[],"usage":{{"prompt_tokens":{prompt},"completion_tokens":{completion},"prompt_tokens_details":{{"cached_tokens":{cached}}}}}}}"#));
            Reply::Sse { frames }
        }
        autre => autre,
    }
}

/// **L'usage du modèle se lit dans la trace** (15 novembre 2026 : le banc doit lire le cache de Vertex
/// avant de dépenser) : sous `trace: true`, chaque section porte `usage {prompt_tokens,
/// cached_prompt_tokens, completion_tokens, ms}` tel que le fournisseur l'a rendu, et le rapport porte
/// le total de la passe (`usage_total`). Un modèle qui ne rend pas de bloc usage (un local) laisse les
/// compteurs **absents**, pas à zéro, et le total le compte à part.
#[test]
#[ignore]
fn l_usage_du_modele_se_lit_par_section_et_au_total_de_la_passe() {
    let serveur = FakeServer::start_sequence(vec![
        avec_usage(etiquette_decrite("create", "La reprise du journal", "Comment le moteur rejoue son journal à l’ouverture."), 120, 100, 7),
        etiquette("none"),
        avec_usage(etiquette("none"), 80, 0, 3),
    ]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    let r = appel(&b, "archive", json!({"titre": "Journal du 26", "contenu": DOCUMENT, "jour": "2026-10-26", "allow_create": true, "trace": true}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    let results = r["result"]["results"].as_array().unwrap_or_else(|| panic!("la trace : {r}"));
    assert_eq!(results.len(), 3, "{r}");
    let u0 = &results[0]["usage"];
    assert_eq!((u0["prompt_tokens"].as_u64(), u0["cached_prompt_tokens"].as_u64(), u0["completion_tokens"].as_u64()), (Some(120), Some(100), Some(7)), "la section 1 : {u0}");
    assert!(u0["ms"].is_u64(), "la durée de l'appel : {u0}");
    let u1 = &results[1]["usage"];
    for champ in ["prompt_tokens", "cached_prompt_tokens", "completion_tokens"] {
        assert!(u1.get(champ).is_none(), "sans bloc usage, « {champ} » est absent, pas à zéro : {u1}");
    }
    let u2 = &results[2]["usage"];
    assert_eq!((u2["prompt_tokens"].as_u64(), u2["cached_prompt_tokens"].as_u64()), (Some(80), Some(0)), "un cache vide rendu est un zéro rendu : {u2}");
    let t = &r["result"]["usage_total"];
    assert_eq!(t["calls"], json!(3), "{t}");
    assert_eq!(t["without_usage"], json!(1), "{t}");
    assert_eq!((t["prompt_tokens"].as_u64(), t["cached_prompt_tokens"].as_u64(), t["completion_tokens"].as_u64()), (Some(200), Some(100), Some(10)), "{t}");
    assert!(t["ms"].is_u64(), "{t}");
}

/// **La requête de la recherche des candidates est bornée** (15 novembre 2026 : une section de 43 000
/// caractères, embarquée entière comme requête, a tué le service bge-m3) : le titre et le début de la
/// section, coupés à `query_chars` caractères (4 000 par défaut) sur une frontière de mot. La trace dit
/// la longueur envoyée (`query_chars`). Le témoin ne passe pas par l'embarqueur pour la mesurer.
#[test]
#[ignore]
fn la_requete_des_candidates_est_bornee_et_la_trace_dit_sa_longueur() {
    let longue = format!("# Un tour long\n\n## Le tour\n\n{}\n", "le journal se rejoue en entier avant toute écriture ".repeat(800));
    assert!(longue.chars().count() > 40_000);
    let serveur = FakeServer::start_sequence(vec![etiquette("none"), etiquette("none")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    let r = appel(&b, "archive", json!({"titre": "Un tour long", "contenu": longue, "jour": "2026-11-15", "trace": true}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    let n = r["result"]["results"][0]["query_chars"].as_u64().unwrap_or_else(|| panic!("la trace dit la longueur de la requête : {}", r["result"]["results"][0]));
    assert!(n <= 4_000 && n > 3_000, "la requête est bornée à 4 000 caractères par défaut : {n}");
    let r = appel(&b, "archive", json!({"titre": "Un tour long, encore", "contenu": longue, "jour": "2026-11-15", "trace": true, "query_chars": 1000}));
    let n = r["result"]["results"][0]["query_chars"].as_u64().unwrap_or(u64::MAX);
    assert!(n <= 1_000 && n > 500, "la borne déclarée : {n}");
}

/// **Le titre du contexte se règle** (15 novembre 2026, le relevé du banc : sous « déjà rangé, ne pas
/// ranger », le modèle a lu que la section elle-même était rangée, et 22 sections ont perdu leur sujet).
/// `context_title` remplace le titre par défaut devant le contexte ; sans lui, le titre d'avant reste.
#[test]
#[ignore]
fn le_titre_du_contexte_se_regle_et_son_defaut_ne_change_pas() {
    const TITRE: &str = "TITREMARQUE ce qui existe déjà dans la mémoire, et ce qui précède ; la section ci-dessous n’est pas encore rangée :";
    let serveur = FakeServer::start_sequence(vec![etiquette("none"), etiquette("none"), etiquette("none"), etiquette("none"), etiquette("none"), etiquette("none")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    appel(&b, "archive", json!({"titre": "Journal du 26", "contenu": DOCUMENT, "jour": "2026-10-26", "context": "CONTEXTEMARQUE", "context_title": TITRE}));
    for i in 0..3 {
        let messages = serde_json::from_str::<Value>(&serveur.request.recv().unwrap()).unwrap()["messages"].to_string();
        let (t, c, s) = (messages.find("TITREMARQUE"), messages.find("CONTEXTEMARQUE"), messages.find("Section «"));
        assert!(t.is_some() && t < c && c < s, "Decide {i} : le titre réglé, puis le contexte, puis la section : {messages}");
        assert!(!messages.contains("déjà rangé, ne pas ranger"), "Decide {i} : le titre par défaut est remplacé");
    }
    // Sans context_title, le défaut d'avant : la F d'aujourd'hui ne bouge pas.
    appel(&b, "archive", json!({"titre": "Journal du 27", "contenu": DOCUMENT, "jour": "2026-10-27", "context": "CONTEXTEMARQUE"}));
    let messages = serde_json::from_str::<Value>(&serveur.request.recv().unwrap()).unwrap()["messages"].to_string();
    assert!(messages.contains("Ce qui précède (déjà rangé, ne pas ranger)"), "le titre par défaut reste : {messages}");
}

/// Une réponse texte du faux serveur (un modèle qui rend du contenu libre, JSON ou non).
fn text_reply(content: &str) -> Reply {
    Reply::Sse {
        frames: vec![
            json!({"id": "a1", "choices": [{"index": 0, "delta": {"role": "assistant", "content": content}, "finish_reason": null}]}).to_string(),
            r#"{"id":"a1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#.into(),
        ],
    }
}

const LONG_TURN: &str = "# Un tour\n\n## turns 5-5\n\nPARAUN la reprise du journal relit chaque enregistrement.\n\nPARADEUX le WAL se rejoue avant toute écriture.\n\nPARATROIS le compositeur figeait à cent pour cent.\n\nPARAQUATRE à soixante-dix pour cent l’écran reste utilisable.\n\nPARACINQ le banc mesure le cache de Vertex.\n\nPARASIX la première fenêtre se lit avant de dépenser.\n";

/// **Une section longue est découpée en sujets avant le Decide** (`segment_over`, 15 novembre 2026) :
/// le modèle rend ses sujets par paragraphes numérotés (`response_schema`), chaque sujet devient une
/// sous-section `<titre> §a-b`, puis un Decide chacune — trois sujets, trois L1, chacun son §. Éteint par
/// défaut (`segment_over` 0) : les autres témoins de cette suite le gardent.
#[test]
#[ignore]
fn a_long_section_is_split_into_topics_then_filed_topic_by_topic() {
    let serveur = FakeServer::start_sequence(vec![
        text_reply(r#"{"topics": [{"title": "la reprise", "from": 1, "to": 2}, {"title": "le compositeur", "from": 3, "to": 4}, {"title": "le cache", "from": 5, "to": 6}]}"#),
        etiquette_decrite("create", "La reprise du journal", "Comment le moteur rejoue son journal à l’ouverture."),
        etiquette_decrite("create", "Le compositeur du portable", "Le compositeur de l’écran du portable et son rapport cyclique."),
        etiquette_decrite("create", "Le cache de Vertex", "Ce que le cache de Vertex sert sur une fenêtre du banc."),
    ]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    let r = appel(&b, "archive", json!({"titre": "Un tour", "contenu": LONG_TURN, "jour": "2026-11-15", "allow_create": true, "segment_over": 200}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    // Le découpage a demandé ses sujets sous un schéma, sur les paragraphes numérotés.
    let premiere: Value = serde_json::from_str(&serveur.request.recv().unwrap()).unwrap();
    assert!(premiere["response_format"]["json_schema"]["schema"]["properties"]["topics"].is_object(), "le découpage demande un schéma : {}", premiere["response_format"]);
    assert!(premiere["messages"].to_string().contains("[3]"), "les paragraphes sont numérotés : {}", premiere["messages"]);
    for (fiche, section, dedans, dehors) in [
        ("La reprise du journal", "turns 5-5 §1-2", "PARADEUX", "PARATROIS"),
        ("Le compositeur du portable", "turns 5-5 §3-4", "PARAQUATRE", "PARACINQ"),
        ("Le cache de Vertex", "turns 5-5 §5-6", "PARASIX", "PARAUN"),
    ] {
        let a = apports_de(&b, fiche);
        assert_eq!(a.len(), 1, "{fiche} : {a:?}");
        assert_eq!(a[0]["section"], json!(section), "{fiche}");
        let t = a[0]["texte"].as_str().unwrap();
        assert!(t.contains(dedans) && !t.contains(dehors), "{fiche} : le texte de ses paragraphes seulement : {t}");
    }
}

/// **Un découpage que le modèle ne rend pas en JSON est refusé en le nommant** : rien n'est rangé, et
/// l'erreur nomme response_schema ; l'archiviste ne devine pas des sujets dans une phrase.
#[test]
#[ignore]
fn a_split_that_is_not_json_is_refused_by_name() {
    let serveur = FakeServer::start_sequence(vec![text_reply("Voici les sujets : la reprise, le compositeur, le cache.")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    let r = b.call("archive", json!({"titre": "Un tour", "contenu": LONG_TURN, "jour": "2026-11-15", "allow_create": true, "segment_over": 200}));
    let dit = match r { Ok(v) => v.to_string(), Err(e) => e.to_string() };
    assert!(dit.contains("response_schema"), "le refus nomme response_schema : {dit}");
    assert!(apports_de(&b, "la reprise").is_empty(), "rien n'est rangé");
}

/// A slice of six turns, in the single form rendered from `Turn` rows: one `## turn <n>` per
/// turn, one `### <role> · <time>` per message, a tool call with its name, status, params and
/// result.
const SIX_TURNS: [&str; 6] = [
    "SLICEONE la reprise du journal relit chaque enregistrement.",
    "SLICETWO le WAL se rejoue avant toute écriture.",
    "SLICETHREE le compositeur figeait à cent pour cent.",
    "SLICEFOUR à soixante-dix pour cent l’écran reste utilisable.",
    "SLICEFIVE la reprise ne relit pas deux fois un enregistrement validé.",
    "SLICESIX le banc mesure le cache de Vertex.",
];

fn slice_of_six_turns() -> String {
    let mut s = String::from("# run chat:test, turns 1-6\n\n");
    for (i, t) in SIX_TURNS.iter().enumerate() {
        let n = i + 1;
        s.push_str(&format!("## turn {n}\n\n### user · 2026-11-18T20:0{n}:00Z\n\n{t}\n\n### assistant · 2026-11-18T20:0{n}:30Z\n\nCompris.\n\n"));
        if n == 3 {
            s.push_str("### tool · 2026-11-18T20:03:40Z · read_file · ok\n\nparams:\n```json\n{\"path\": \"src/compositor.rs\"}\n```\n\nresult:\n```\nfn duty() -> u8 { 70 }\n```\n\n");
        }
    }
    s
}

/// **The two-step archivist on a slice** (18 November 2026, Lucie's form): one structured
/// extraction over the whole slice (no base, no candidates), then one Decide per extracted topic.
/// A topic may span two ranges; a turn no topic covers and no `not_filed` reason sets aside goes
/// to a residual « reste » segment, filed like the others — nothing falls silently.
#[test]
#[ignore]
fn slice_files_one_l1_per_topic_and_a_residual_for_uncovered_turns() {
    let extraction = r#"{"slice_summary": "SUMMARYMARK la reprise et le compositeur", "topics": [
        {"title": "la reprise du journal", "turns": [{"from": 1, "to": 2}, {"from": 5, "to": 5}], "said": "SAIDREPRISE le journal se rejoue, sans relire deux fois", "open": false, "attributes": [{"dimension": "couche", "value": "moteur"}]},
        {"title": "le compositeur", "turns": [{"from": 3, "to": 4}], "said": "SAIDCOMPOSITEUR il figeait à 100 %, utilisable à 70 %", "open": true, "attributes": []}
    ], "not_filed": []}"#;
    let serveur = FakeServer::start_sequence(vec![
        text_reply(extraction),
        etiquette_decrite("create", "La reprise du journal", "Comment le moteur rejoue son journal à l’ouverture."),
        etiquette_decrite("create", "Le compositeur du portable", "Le compositeur de l’écran du portable et son rapport cyclique."),
        etiquette_decrite("create", "Le cache de Vertex", "Ce que le cache de Vertex sert sur une fenêtre du banc."),
    ]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    let r = appel(&b, "archive_slice", json!({"titre": "run chat:test, turns 1-6", "chemin": "sources/chat-test-turns-1-6.md", "contenu": slice_of_six_turns(), "jour": "2026-11-18", "allow_create": true, "trace": true}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    // Step 1: one extraction call, under a schema, over numbered turns, with no candidate fiche.
    let first: Value = serde_json::from_str(&serveur.request.recv().unwrap()).unwrap();
    let schema = &first["response_format"]["json_schema"]["schema"]["properties"];
    assert!(schema["topics"].is_object() && schema["slice_summary"].is_object(), "the extraction schema: {schema}");
    let prompt = first["messages"].to_string();
    assert!(prompt.contains("turn 6") && prompt.contains("SLICESIX"), "the whole slice goes to the extraction");
    assert!(!prompt.contains("Fiches candidates") && !prompt.contains("candidate_1"), "no candidate in step 1: {prompt}");
    // Step 2: one Decide per topic, plus the residual.
    assert_eq!(serveur.connections.load(std::sync::atomic::Ordering::SeqCst), 4, "1 extraction + 3 Decide");
    for (fiche, section, inside, outside) in [
        ("La reprise du journal", "turns 1-5 · la reprise du journal", "SLICEFIVE", "SLICETHREE"),
        ("Le compositeur du portable", "turns 3-4 · le compositeur", "SLICEFOUR", "SLICEFIVE"),
        ("Le cache de Vertex", "turns 6-6 · reste", "SLICESIX", "SLICEONE"),
    ] {
        let a = apports_de(&b, fiche);
        assert_eq!(a.len(), 1, "{fiche}: {a:?}");
        assert_eq!(a[0]["section"], json!(section), "{fiche}");
        let t = a[0]["texte"].as_str().unwrap();
        assert!(t.contains(inside) && !t.contains(outside), "{fiche}: its own turns only: {t}");
    }
    // The default step2_input is `both`: the extracted `said`, then the raw turns.
    let reprise = &apports_de(&b, "La reprise du journal")[0]["texte"];
    assert!(reprise.as_str().unwrap().contains("SAIDREPRISE") && reprise.as_str().unwrap().contains("SLICEONE"), "said and raw turns: {reprise}");
    // The report carries what the extraction said about the whole slice.
    assert!(r["result"]["slice_summary"].as_str().is_some_and(|s| s.contains("SUMMARYMARK")), "{r}");
    assert_eq!(r["result"]["open"], json!(["le compositeur"]), "{r}");
}

/// The same six turns, written as `Turn` rows by put_turn, the way the host writes them at the end
/// of each turn: seq 0 is the user message.
fn put_six_turns(b: &Backend) {
    for (i, t) in SIX_TURNS.iter().enumerate() {
        let n = i as i64 + 1;
        appel(b, "put_turn", json!({"run": "chat:test", "turn": n, "seq": 0, "at": format!("2026-11-18T20:0{n}:00Z"), "role": "user", "text": t}));
        appel(b, "put_turn", json!({"run": "chat:test", "turn": n, "seq": 1, "at": format!("2026-11-18T20:0{n}:30Z"), "role": "assistant", "text": "Compris."}));
        if n == 3 {
            appel(b, "put_turn", json!({"run": "chat:test", "turn": 3, "seq": 2, "at": "2026-11-18T20:03:40Z", "role": "tool", "text": "",
                "tool_name": "read_file", "params": "{\"path\": \"src/compositor.rs\"}", "result": "fn duty() -> u8 { 70 }", "status": "ok"}));
        }
    }
}

/// **put_turn** (18 November 2026): one row per message, keyed by (run, turn, seq): written again,
/// it stays one row. Secrets are masked before anything is stored, a tool's params are bounded at
/// 4000 characters and its result at 8000, with the original size said; the call says what it did.
#[test]
#[ignore]
fn put_turn_masks_secrets_bounds_tool_payloads_and_keeps_one_row_per_message() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    let said = appel(&b, "put_turn", json!({"run": "chat:t", "turn": 1, "seq": 0, "at": "2026-11-18T21:00:00Z", "role": "user",
        "text": "my token is ghp_0123456789abcdefABCDEF0123456789abcd, keep it"}));
    assert_eq!(said["result"], json!({"written": true, "masked": 1, "cut": false}), "{said}");
    appel(&b, "put_turn", json!({"run": "chat:t", "turn": 1, "seq": 0, "at": "2026-11-18T21:00:00Z", "role": "user",
        "text": "my token is ghp_0123456789abcdefABCDEF0123456789abcd, keep it"}));
    let long = "x".repeat(9000);
    let said = appel(&b, "put_turn", json!({"run": "chat:t", "turn": 1, "seq": 1, "at": "2026-11-18T21:00:05Z", "role": "tool", "text": "",
        "tool_name": "grep_files", "params": format!("{{\"pattern\": \"{}\"}}", "p".repeat(5000)), "result": long, "status": "error"}));
    assert_eq!(said["result"], json!({"written": true, "masked": 0, "cut": true}), "{said}");
    let r = appel(&b, "render_turns", json!({"run": "chat:t", "from_turn": 1, "to_turn": 1}));
    let text = r["result"]["text"].as_str().unwrap();
    assert_eq!(r["result"]["messages"], json!(2), "one row per (run, turn, seq): {text}");
    assert_eq!(text.matches("### user").count(), 1, "{text}");
    assert!(!text.contains("ghp_0123456789") && text.contains("[masked secret: github token]"), "{text}");
    assert!(text.contains("### tool · 2026-11-18T21:00:05Z · grep_files · error"), "{text}");
    assert!(text.contains("… (9000 characters)") && !text.contains(&"x".repeat(8001)), "the result is bounded at 8000");
    assert!(text.contains("characters)") && !text.contains(&"p".repeat(4001)), "the params are bounded at 4000");
}

/// **put_turn's patterns are a parameter** (lot B6): a pattern the call declares is masked, without a rebuild. The
/// declared defaults are proven by the witness above (the github token of its text).
#[test]
#[ignore]
fn put_turn_masks_a_pattern_added_by_its_parameter() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    let patterns = json!([
        {"name": "github token", "kind": "prefix", "prefix": "ghp_", "min_body": 30},
        {"name": "acme key", "kind": "prefix", "prefix": "acme_", "min_body": 8}
    ]);
    let said = appel(&b, "put_turn", json!({"run": "chat:p", "turn": 1, "seq": 0, "at": "", "role": "user",
        "text": "acme_12345678 and ghp_0123456789abcdefABCDEF0123456789abcd", "patterns": patterns}));
    assert_eq!(said["result"]["masked"], json!(2), "{said}");
    let r = appel(&b, "render_turns", json!({"run": "chat:p", "from_turn": 1, "to_turn": 1}));
    let text = r["result"]["text"].as_str().unwrap();
    assert!(text.contains("[masked secret: acme key]") && text.contains("[masked secret: github token]"), "{text}");
}

/// **One form, written in one place** (archi and mémoire, 18 November 2026): render_turns makes the
/// slice from the `Turn` rows, and its text is exactly the form the banc sends in `contenu`.
#[test]
#[ignore]
fn render_turns_gives_exactly_the_form_the_banc_sends() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    put_six_turns(&b);
    let r = appel(&b, "render_turns", json!({"run": "chat:test", "from_turn": 1, "to_turn": 6}));
    assert_eq!(r["result"]["text"].as_str().unwrap(), slice_of_six_turns());
    assert_eq!(r["result"]["turns"], json!([1, 2, 3, 4, 5, 6]));
    let r = appel(&b, "render_turns", json!({"run": "chat:test", "from_turn": 3, "to_turn": 4}));
    let t = r["result"]["text"].as_str().unwrap();
    assert!(t.starts_with("# run chat:test, turns 3-4\n\n## turn 3\n") && t.contains("SLICEFOUR") && !t.contains("SLICEFIVE"), "{t}");
}

/// **archive_slice by range**, the host's call: no text passes; the tool renders the turns from
/// their rows, keeps that text as the slice Source (sources/<run slug>-turns-<a>-<b>.md, origin
/// run_trace), files it in two steps, and says how far the run is archived.
#[test]
#[ignore]
fn archive_slice_by_range_files_the_rendered_turns_and_says_how_far() {
    let extraction = r#"{"slice_summary": "SUMMARYMARK", "topics": [
        {"title": "la reprise du journal", "turns": [{"from": 1, "to": 2}, {"from": 5, "to": 5}], "said": "SAIDREPRISE", "open": false, "attributes": []},
        {"title": "le compositeur", "turns": [{"from": 3, "to": 4}], "said": "SAIDCOMPOSITEUR", "open": true, "attributes": []}
    ], "not_filed": []}"#;
    let serveur = FakeServer::start_sequence(vec![
        text_reply(extraction),
        etiquette_decrite("create", "La reprise du journal", "Comment le moteur rejoue son journal à l’ouverture."),
        etiquette_decrite("create", "Le compositeur du portable", "Le compositeur de l’écran du portable et son rapport cyclique."),
        etiquette_decrite("create", "Le cache de Vertex", "Ce que le cache de Vertex sert sur une fenêtre du banc."),
    ]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    put_six_turns(&b);
    let r = appel(&b, "archive_slice", json!({"run": "chat:test", "from_turn": 1, "to_turn": 6, "jour": "2026-11-18", "allow_create": true}));
    assert_eq!(r["result"]["errors"], json!([]), "{r}");
    assert_eq!(r["result"]["archived_through_turn"], json!(6), "{r}");
    // The path written, for the host's list of filed slices (compose_restart's sources).
    assert_eq!(r["result"]["source"], json!("sources/chat-test-turns-1-6.md"), "{r}");
    let first: Value = serde_json::from_str(&serveur.request.recv().unwrap()).unwrap();
    assert!(first["messages"].to_string().contains("read_file · ok"), "the tool call reaches the extraction");
    let s = appel(&b, "select_sources", json!({"filter": {"field": {"key": "chemin", "value": "sources/chat-test-turns-1-6.md"}}}));
    let rows = s["result"].as_array().cloned().unwrap_or_default();
    assert_eq!(rows.len(), 1, "the slice Source: {s}");
    assert_eq!(rows[0]["data"]["contenu"].as_str().unwrap(), slice_of_six_turns(), "the Source is the rendered slice");
    assert_eq!(rows[0]["data"]["origine"], json!("run_trace"));
    assert_eq!(apports_de(&b, "Le cache de Vertex")[0]["section"], json!("turns 6-6 · reste"));
}

/// A slice whose filing reports an error is not counted as archived: archived_through_turn stays
/// before it (from_turn - 1), and the host files it again.
#[test]
#[ignore]
fn archive_slice_with_an_error_does_not_advance() {
    let extraction = r#"{"slice_summary": "s", "topics": [
        {"title": "la reprise du journal", "turns": [{"from": 1, "to": 6}], "said": "SAIDREPRISE", "open": false, "attributes": []}
    ], "not_filed": []}"#;
    let serveur = FakeServer::start_sequence(vec![text_reply(extraction), text_reply("not a label")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    put_six_turns(&b);
    let r = appel(&b, "archive_slice", json!({"run": "chat:test", "from_turn": 1, "to_turn": 6, "jour": "2026-11-18", "allow_create": true}));
    assert_ne!(r["result"]["errors"], json!([]), "{r}");
    assert_eq!(r["result"]["archived_through_turn"], json!(0), "{r}");
}

/// Three tool calls and a user message over two days, for the turn listings.
fn put_calls_over_two_days(b: &Backend) {
    for (turn, at, tool, status, params, result) in [
        (1, "2026-11-17T10:00:00Z", "archive", "error", "{\"titre\": \"hier\"}", "PARSEFAILED the model replied no JSON"),
        (2, "2026-11-18T09:00:00Z", "read_file", "error", "{\"path\": \"absent.rs\"}", "no such file"),
        (3, "2026-11-18T11:00:00Z", "archive", "ok", "{\"titre\": \"aujourd'hui\"}", "3 sections filed"),
    ] {
        appel(b, "put_turn", json!({"run": "chat:days", "turn": turn, "seq": 0, "at": at, "role": "user", "text": format!("ZEBRAFINCH turn {turn}")}));
        appel(b, "put_turn", json!({"run": "chat:days", "turn": turn, "seq": 1, "at": at, "role": "tool", "tool_name": tool, "status": status, "params": params, "result": result}));
    }
}

/// **list_turns** (18 November 2026, Lucie: « revoir un truc qui aurait mal marché, vérifier que la
/// veille on a bien appelé tel outil, quel est son résultat, quels paramètres »): the turns by time,
/// newest first, filtered by tool, status, role, run and date, each with its params and result.
#[test]
#[ignore]
fn list_turns_finds_yesterdays_failed_call_with_its_params_and_result() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    put_calls_over_two_days(&b);
    let r = appel(&b, "list_turns", json!({"tool_name": "archive", "status": "error", "since": "2026-11-17T00:00:00Z"}));
    let rows = r["result"]["turns"].as_array().cloned().unwrap_or_default();
    assert_eq!(rows.len(), 1, "{r}");
    assert_eq!(rows[0]["turn"], json!(1));
    assert_eq!(rows[0]["params"], json!("{\"titre\": \"hier\"}"));
    assert!(rows[0]["result"].as_str().unwrap().contains("PARSEFAILED"), "{r}");
    // By time, newest first; `until` bounds the other end.
    let r = appel(&b, "list_turns", json!({"run": "chat:days", "role": "tool", "until": "2026-11-18T10:00:00Z"}));
    let turns: Vec<Value> = r["result"]["turns"].as_array().unwrap().iter().map(|x| x["turn"].clone()).collect();
    assert_eq!(turns, vec![json!(2), json!(1)], "{r}");
    // `limit` keeps the newest.
    let r = appel(&b, "list_turns", json!({"run": "chat:days", "limit": 2}));
    let ats: Vec<Value> = r["result"]["turns"].as_array().unwrap().iter().map(|x| x["at"].clone()).collect();
    assert_eq!(ats, vec![json!("2026-11-18T11:00:00Z"), json!("2026-11-18T11:00:00Z")], "{r}");
    assert_eq!(r["result"]["total"], json!(6), "the total before the limit: {r}");
}

/// **search_turns** looks in the turns, and only it: the ordinary recalls never see a Turn (Lucie:
/// « le filtre par défaut de la recherche mémoire ne les trouve pas ; il faut préciser qu'on veut
/// chercher dedans »).
#[test]
#[ignore]
fn search_turns_finds_turns_and_the_ordinary_recalls_do_not() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    put_calls_over_two_days(&b);
    let r = appel(&b, "search_turns", json!({"query": "ZEBRAFINCH", "status": "ok", "options": {"consistency": "strict"}}));
    let rows = r["result"]["turns"].as_array().cloned().unwrap_or_default();
    assert!(!rows.is_empty() && rows.iter().all(|x| x["status"] == json!("ok")), "{r}");
    assert!(rows.iter().any(|x| x["text"].as_str().is_some_and(|t| t.contains("ZEBRAFINCH"))), "{r}");
    for outil in ["recall_apports", "recall_fiches"] {
        let r = appel(&b, outil, json!({"query": "ZEBRAFINCH", "options": {"consistency": "strict"}}));
        assert!(!r.to_string().contains("ZEBRAFINCH turn"), "{outil} sees a turn: {r}");
    }
}

/// **read_turns** follows the chain down to the turns: each turn of the range, in order, with its
/// state — `indexed` (read from Turn) or `gone` (not there any more); never a silent gap.
#[test]
#[ignore]
fn read_turns_gives_each_turn_of_the_range_with_its_state() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    put_calls_over_two_days(&b);
    let r = appel(&b, "read_turns", json!({"run": "chat:days", "from_turn": 2, "to_turn": 4}));
    let t = r["result"]["turns"].as_array().cloned().unwrap_or_default();
    assert_eq!(t.len(), 3, "{r}");
    assert_eq!((t[0]["turn"].clone(), t[0]["state"].clone()), (json!(2), json!("indexed")));
    assert_eq!(t[0]["messages"].as_array().unwrap().len(), 2, "{r}");
    assert_eq!(t[0]["messages"][1]["result"], json!("no such file"));
    assert_eq!((t[2]["turn"].clone(), t[2]["state"].clone()), (json!(4), json!("gone")));
    assert_eq!(t[2]["messages"], json!([]));
}

/// **The TTL of the turns** (19 November 2026, tranché par c7 : une expiration déclarée par
/// l'entité) : purge_turns expires the Turn rows older than the bound, with their full text; the
/// slice Source and the L1 filed from those turns stay (they cite a range, not rows).
#[test]
#[ignore]
fn purge_turns_expires_old_turns_and_keeps_the_slice_and_its_l1s() {
    let extraction = r#"{"slice_summary": "s", "topics": [
        {"title": "la reprise du journal", "turns": [{"from": 1, "to": 6}], "said": "SAIDREPRISE", "open": false, "attributes": []}
    ], "not_filed": []}"#;
    let serveur = FakeServer::start_sequence(vec![
        text_reply(extraction),
        etiquette_decrite("create", "La reprise du journal", "Comment le moteur rejoue son journal à l’ouverture."),
    ]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    put_six_turns(&b);
    let r = appel(&b, "archive_slice", json!({"run": "chat:test", "from_turn": 1, "to_turn": 6, "allow_create": true}));
    assert_eq!(r["result"]["archived_through_turn"], json!(6), "{r}");
    // The default bound is the entity's ttl_days (30): fresh turns stay.
    let r = appel(&b, "purge_turns", json!({}));
    assert_eq!(r["result"]["expired"], json!(0), "{r}");
    assert_eq!(r["result"]["older_than_days"], json!(30), "{r}");
    // 0 days: every row written before now goes.
    let r = appel(&b, "purge_turns", json!({"older_than_days": 0}));
    assert_eq!(r["result"]["expired"], json!(13), "6 turns × 2 messages + 1 tool call: {r}");
    let read = appel(&b, "read_turns", json!({"run": "chat:test", "from_turn": 1, "to_turn": 6}));
    assert!(read["result"]["turns"].as_array().unwrap().iter().all(|t| t["state"] == json!("gone")), "{read}");
    let found = appel(&b, "search_turns", json!({"query": "SLICEONE", "options": {"consistency": "strict"}}));
    assert_eq!(found["result"]["turns"], json!([]), "the full text goes with the row: {found}");
    let s = appel(&b, "select_sources", json!({"filter": {"field": {"key": "chemin", "value": "sources/chat-test-turns-1-6.md"}}}));
    assert_eq!(s["result"].as_array().map(Vec::len), Some(1), "the slice Source stays: {s}");
    assert_eq!(apports_de(&b, "La reprise du journal").len(), 1, "the L1 stays");
}

/// **The guard is the entity's declaration** : expire on an entity that does not declare
/// `writes.expires` is refused, naming it — a tool can expire nothing else than turns.
#[test]
#[ignore]
fn expire_is_refused_on_an_entity_that_does_not_declare_it() {
    let dir = tempfile::tempdir().expect("dossier");
    let racine = dir.path().to_path_buf();
    let b = ouvrir(dir.path(), |m| {
        std::fs::write(
            racine.join("templates/backends/memory/graphs/expire_fiches.mmd"),
            "%% tool: expire_fiches\n%% description: test\n%% param: record json = {} -- Reserved.\n%% result: x.record\n\ngraph LR\n    x[\"EntityRecordNode(entity='Fiche', action='expire', record=$record, older_than_days=0)\"]\n",
        )
        .unwrap();
        m["tools"]["expire_fiches"] = json!({"graph": "graphs/expire_fiches.mmd"});
    });
    appel(&b, "put_fiche", json!({"record": {"name": "Une fiche qui reste", "about": "Une fiche qui ne doit jamais expirer.", "state": "current"}}));
    let said = match b.call("expire_fiches", json!({})) { Ok(v) => v.to_string(), Err(e) => e.to_string() };
    assert!(said.contains("Fiche") && said.contains("writes.expires"), "refused, naming the entity: {said}");
    let f = appel(&b, "get_fiche", json!({"record": {"name": "Une fiche qui reste"}}));
    assert!(f["result"]["data"].is_object(), "the fiche stays: {f}");
}

/// The six turns filed by range, with three topics (1-2 and 5, 3-4, and the residual 6).
fn six_turns_filed(b: &Backend, serveur: &FakeServer) {
    let _ = serveur;
    put_six_turns(b);
    let r = appel(b, "archive_slice", json!({"run": "chat:test", "from_turn": 1, "to_turn": 6, "allow_create": true}));
    assert_eq!(r["result"]["archived_through_turn"], json!(6), "{r}");
}

fn three_topics_server() -> FakeServer {
    let extraction = r#"{"slice_summary": "SUMMARYMARK la reprise et le compositeur", "topics": [
        {"title": "la reprise du journal", "turns": [{"from": 1, "to": 2}, {"from": 5, "to": 5}], "said": "SAIDREPRISE", "open": false, "attributes": []},
        {"title": "le compositeur", "turns": [{"from": 3, "to": 4}], "said": "SAIDCOMPOSITEUR", "open": true, "attributes": []}
    ], "not_filed": []}"#;
    FakeServer::start_sequence(vec![
        text_reply(extraction),
        etiquette_decrite("create", "La reprise du journal", "Comment le moteur rejoue son journal à l’ouverture."),
        etiquette_decrite("create", "Le compositeur du portable", "Le compositeur de l’écran du portable et son rapport cyclique."),
        etiquette_decrite("create", "Le cache de Vertex", "Ce que le cache de Vertex sert sur une fenêtre du banc."),
    ])
}

/// **The chain, both ways** (18 November 2026, Lucie: « ce L1 venait de telle plage ; telle plage
/// allait de tel tour à tel tour ; si je veux, je relis exactement tel tour »): the L1 and the slice
/// Source say their run and range in clear; get_slice, list_slices and trace_turn follow them; from a
/// fiche's L1, read_turns gives back the same turns.
#[test]
#[ignore]
fn the_chain_goes_from_a_turn_to_its_slice_its_l1s_and_their_fiches_and_back() {
    let serveur = three_topics_server();
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    six_turns_filed(&b, &serveur);
    // The L1 says its range in clear.
    let compositeur = &apports_de(&b, "Le compositeur du portable")[0];
    assert_eq!((compositeur["run"].clone(), compositeur["from_turn"].clone(), compositeur["to_turn"].clone()), (json!("chat:test"), json!(3), json!(4)), "{compositeur}");
    // The slice Source too, with its summary.
    let s = appel(&b, "select_sources", json!({"filter": {"field": {"key": "chemin", "value": "sources/chat-test-turns-1-6.md"}}}));
    let src = &s["result"][0]["data"];
    assert_eq!((src["run"].clone(), src["from_turn"].clone(), src["to_turn"].clone()), (json!("chat:test"), json!(1), json!(6)), "{s}");
    assert!(src["slice_summary"].as_str().is_some_and(|x| x.contains("SUMMARYMARK")), "{s}");
    // get_slice, by its path or by a turn it contains.
    for args in [json!({"chemin": "sources/chat-test-turns-1-6.md"}), json!({"run": "chat:test", "turn": 4})] {
        let g = appel(&b, "get_slice", args.clone());
        let r = &g["result"];
        assert_eq!(r["state"], json!("closed"), "{args}: {g}");
        assert_eq!((r["chemin"].clone(), r["from_turn"].clone(), r["to_turn"].clone()), (json!("sources/chat-test-turns-1-6.md"), json!(1), json!(6)), "{g}");
        let mut fiches: Vec<String> = r["l1"].as_array().unwrap().iter().map(|x| x["fiche"].as_str().unwrap().to_string()).collect();
        fiches.sort();
        assert_eq!(fiches, vec!["La reprise du journal", "Le cache de Vertex", "Le compositeur du portable"], "{g}");
    }
    // list_slices: the run's slices in order, each with its count of L1.
    let l = appel(&b, "list_slices", json!({"run": "chat:test"}));
    assert_eq!(l["result"]["slices"].as_array().map(Vec::len), Some(1), "{l}");
    assert_eq!(l["result"]["slices"][0]["l1"], json!(3), "{l}");
    // trace_turn: the turn, its slice, the L1 that cover it and their fiches.
    let t = appel(&b, "trace_turn", json!({"run": "chat:test", "turn": 3}));
    let r = &t["result"];
    assert_eq!(r["turn"]["state"], json!("indexed"), "{t}");
    assert!(r["turn"]["messages"].to_string().contains("SLICETHREE"), "{t}");
    assert_eq!(r["slice"]["chemin"], json!("sources/chat-test-turns-1-6.md"), "{t}");
    let mut fiches: Vec<String> = r["l1"].as_array().unwrap().iter().map(|x| x["fiche"].as_str().unwrap().to_string()).collect();
    fiches.sort();
    assert_eq!(fiches, vec!["La reprise du journal", "Le compositeur du portable"], "1-5 and 3-4 cover turn 3, 6-6 does not: {t}");
    // And back: from the L1 of a fiche, read_turns gives its turns.
    let back = appel(&b, "read_turns", json!({"run": compositeur["run"], "from_turn": compositeur["from_turn"], "to_turn": compositeur["to_turn"]}));
    assert!(back["result"]["turns"][0]["messages"].to_string().contains("SLICETHREE"), "{back}");
}

/// A turn that no slice contains yet: its slice is `open`, with no L1 — « pas encore rangé » is true
/// by construction.
#[test]
#[ignore]
fn a_turn_not_filed_yet_belongs_to_an_open_slice() {
    let serveur = three_topics_server();
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    six_turns_filed(&b, &serveur);
    appel(&b, "put_turn", json!({"run": "chat:test", "turn": 7, "seq": 0, "at": "2026-11-18T20:07:00Z", "role": "user", "text": "SLICESEVEN pas encore rangé"}));
    let t = appel(&b, "trace_turn", json!({"run": "chat:test", "turn": 7}));
    assert_eq!(t["result"]["slice"]["state"], json!("open"), "{t}");
    assert_eq!(t["result"]["l1"], json!([]), "{t}");
    assert!(t["result"]["turn"]["messages"].to_string().contains("SLICESEVEN"), "{t}");
    let g = appel(&b, "get_slice", json!({"run": "chat:test", "turn": 7}));
    assert_eq!(g["result"]["state"], json!("open"), "{g}");
}

/// The user message of a request the fake server received.
fn user_prompt(request: &Value) -> String {
    request["messages"].as_array().unwrap().iter().filter(|m| m["role"] == json!("user")).map(|m| m["content"].as_str().unwrap_or_default().to_string()).collect::<Vec<_>>().join("\n")
}

/// **B3, the reduced prompt** (Lucie, 19 November 2026: « éviter qu'il y ait beaucoup de prompt à part les
/// descriptions, si c'est pour l'extraction structurée » ; « même la consigne est paramétrable »): the extraction
/// prompt is the one-line instruction (a declared parameter) then the slice as a context part — nothing else. The
/// rules that were prose live in the descriptions of the response schema's fields.
#[test]
#[ignore]
fn the_slice_extraction_prompt_is_the_instruction_and_the_slice_only() {
    let serveur = FakeServer::start_sequence(vec![text_reply(r#"{"slice_summary": "s", "topics": [], "not_filed": [{"from": 1, "to": 6, "reason": "nothing"}]}"#)]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    let _ = b.call("archive_slice", json!({"titre": "run chat:test, turns 1-6", "chemin": "sources/chat-test-turns-1-6.md", "contenu": slice_of_six_turns(), "allow_create": true}));
    let first: Value = serde_json::from_str(&serveur.request.recv().unwrap()).unwrap();
    assert_eq!(user_prompt(&first), format!("Split this slice of a conversation into topics.\n\n## The slice\n\n{}", slice_of_six_turns()));
    let props = &first["response_format"]["json_schema"]["schema"]["properties"];
    let topic = &props["topics"]["items"]["properties"];
    assert!(topic["title"]["description"].as_str().is_some_and(|d| d.contains("never a date")), "the rule of a topic is in the schema: {props}");
    assert!(topic["turns"]["description"].as_str().is_some_and(|d| d.contains("## turn")), "{props}");
    assert!(props["not_filed"]["description"].as_str().is_some_and(|d| d.contains("reason")), "{props}");
    // The instruction is a parameter.
    let serveur = FakeServer::start_sequence(vec![text_reply(r#"{"slice_summary": "s", "topics": [], "not_filed": [{"from": 1, "to": 6, "reason": "nothing"}]}"#)]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    let _ = b.call("archive_slice", json!({"titre": "t", "chemin": "sources/t.md", "contenu": slice_of_six_turns(), "extract_instruction": "Find the topics."}));
    let first: Value = serde_json::from_str(&serveur.request.recv().unwrap()).unwrap();
    assert!(user_prompt(&first).starts_with("Find the topics.\n\n## The slice\n\n"), "{}", user_prompt(&first));
}

/// **The extraction asks for little reasoning by default** (19 November 2026: on Vertex, the reasoning of Gemini
/// counts in max_tokens, and cut 5 extractions of 19): `extract_reasoning` is declared, `low` by default, and reaches
/// the request.
#[test]
#[ignore]
fn the_slice_extraction_asks_low_reasoning_by_default() {
    let serveur = FakeServer::start_sequence(vec![text_reply(r#"{"slice_summary": "s", "topics": [], "not_filed": [{"from": 1, "to": 6, "reason": "nothing"}]}"#)]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    let _ = b.call("archive_slice", json!({"titre": "t", "chemin": "sources/t.md", "contenu": slice_of_six_turns()}));
    let first: Value = serde_json::from_str(&serveur.request.recv().unwrap()).unwrap();
    assert_eq!(first["reasoning_effort"], json!("low"), "{}", first);
}

/// **The slice report carries the usage of its extraction** (19 November 2026, the banc: the extraction's tokens
/// were only estimated): `extraction_usage` says the prompt, cached and completion tokens the provider returned.
#[test]
#[ignore]
fn the_slice_report_carries_the_usage_of_its_extraction() {
    let serveur = FakeServer::start_sequence(vec![avec_usage(text_reply(r#"{"slice_summary": "s", "topics": [], "not_filed": [{"from": 1, "to": 6, "reason": "nothing"}]}"#), 5000, 1200, 900)]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    let r = appel(&b, "archive_slice", json!({"titre": "t", "chemin": "sources/t.md", "contenu": slice_of_six_turns()}));
    let u = &r["result"]["extraction_usage"];
    assert_eq!((u["prompt_tokens"].clone(), u["cached_prompt_tokens"].clone(), u["completion_tokens"].clone()), (json!(5000), json!(1200), json!(900)), "{r}");
}

/// **The version of the template, everywhere a filing leaves a trace** (20 November 2026, the seed of an agent that
/// edits its own memory workflow: a drop must be tied to the version that produced it): every tool call says the
/// template it ran on (`metadata.template`: its content hash and its reload number), and the slice Source and the L1
/// it filed carry that hash.
#[test]
#[ignore]
fn the_template_version_is_on_the_call_the_source_and_the_l1() {
    let serveur = three_topics_server();
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    put_six_turns(&b);
    let r = appel(&b, "archive_slice", json!({"run": "chat:test", "from_turn": 1, "to_turn": 6, "allow_create": true}));
    let hash = r["metadata"]["template"]["hash"].as_str().unwrap_or_else(|| panic!("the call says its template: {r}")).to_string();
    assert!(hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()), "{hash}");
    assert_eq!(r["metadata"]["template"]["version"], json!(0), "{r}");
    let s = appel(&b, "select_sources", json!({"filter": {"field": {"key": "chemin", "value": "sources/chat-test-turns-1-6.md"}}}));
    assert_eq!(s["result"][0]["data"]["template"], json!(hash), "the Source: {s}");
    assert_eq!(apports_de(&b, "Le compositeur du portable")[0]["template"], json!(hash), "the L1");
}

/// **An edited template is a new version**: after a script of the template changes and the backend reloads, the
/// calls say another hash and the next reload number.
#[test]
#[ignore]
fn an_edited_template_reloaded_is_a_new_version() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    let before = appel(&b, "list_turns", json!({}));
    let h0 = before["metadata"]["template"]["hash"].as_str().unwrap_or_else(|| panic!("{before}")).to_string();
    let script = dir.path().join("templates/backends/memory/scripts/turns_list.rhai");
    let mut text = std::fs::read_to_string(&script).unwrap();
    text.push_str("\n// edited by the witness\n");
    std::fs::write(&script, text).unwrap();
    let reloaded = b.reload().expect("the edited template reloads");
    assert_eq!(reloaded.version, 1);
    let after = appel(&b, "list_turns", json!({}));
    assert_eq!(after["metadata"]["template"]["version"], json!(1), "{after}");
    assert_ne!(after["metadata"]["template"]["hash"], json!(h0), "another content, another hash");
}

/// **The cited graphs are on disk, in the template** (20 November 2026: a graph cited by `@builtin:` came from the
/// binary, and an agent that edited the template's copy edited nothing): a cited graph edited in the template's
/// folder changes what the call plays, and the version of the template. Here `turns_select.mmd`, cited by
/// read_turns: limited to one row, a turn of two messages shows one.
#[test]
#[ignore]
fn a_cited_graph_edited_on_disk_changes_what_the_call_plays() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    for seq in 0..2 {
        appel(&b, "put_turn", json!({"run": "chat:g", "turn": 1, "seq": seq, "at": "", "role": "user", "text": format!("message {seq}")}));
    }
    let before = appel(&b, "read_turns", json!({"run": "chat:g", "from_turn": 1, "to_turn": 1}));
    assert_eq!(before["result"]["turns"][0]["messages"].as_array().map(Vec::len), Some(2), "{before}");
    let h0 = before["metadata"]["template"]["hash"].clone();
    let cited = dir.path().join("templates/backends/memory/graphs/turns_select.mmd");
    let text = std::fs::read_to_string(&cited).unwrap();
    let edited = text.replace("limit=0", "limit=1");
    assert_ne!(edited, text, "the witness edits the cited graph");
    std::fs::write(&cited, edited).unwrap();
    b.reload().expect("the edited template reloads");
    let after = appel(&b, "read_turns", json!({"run": "chat:g", "from_turn": 1, "to_turn": 1}));
    assert_eq!(after["result"]["turns"][0]["messages"].as_array().map(Vec::len), Some(1), "the edited graph is played: {after}");
    assert_ne!(after["metadata"]["template"]["hash"], h0, "and the version changed");
}

/// The split of a long section (archive, segment_over): the instruction, then its numbered paragraphs as a part;
/// the rules of a topic are in the schema.
#[test]
#[ignore]
fn the_split_prompt_is_the_instruction_and_the_numbered_section_only() {
    let serveur = FakeServer::start_sequence(vec![text_reply(r#"{"topics": []}"#), etiquette("none")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    let _ = b.call("archive", json!({"titre": "Un tour", "contenu": LONG_TURN, "jour": "2026-11-15", "segment_over": 200}));
    let first: Value = serde_json::from_str(&serveur.request.recv().unwrap()).unwrap();
    let p = user_prompt(&first);
    assert!(p.starts_with("Split this section into topics.\n\n## The section: "), "{p}");
    assert!(p.contains("[1] ") && p.contains("[3] ") && !p.contains("A topic is"), "numbered paragraphs, no prose rule: {p}");
    let topic = &first["response_format"]["json_schema"]["schema"]["properties"]["topics"]["items"]["properties"];
    assert!(topic["from"]["description"].as_str().is_some_and(|d| d.contains("[n]")), "{topic}");
}

/// An extraction that is not the requested JSON is refused, naming response_schema: nothing is
/// guessed and nothing is filed.
#[test]
#[ignore]
fn a_slice_extraction_that_is_not_json_is_refused_by_name() {
    let serveur = FakeServer::start_sequence(vec![text_reply("Les sujets sont la reprise et le compositeur.")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    let r = b.call("archive_slice", json!({"titre": "run chat:test, turns 1-6", "chemin": "sources/chat-test-turns-1-6.md", "contenu": slice_of_six_turns(), "jour": "2026-11-18", "allow_create": true}));
    let said = match r { Ok(v) => v.to_string(), Err(e) => e.to_string() };
    assert!(said.contains("response_schema"), "the refusal names response_schema: {said}");
    assert!(apports_de(&b, "La reprise du journal").is_empty(), "nothing filed");
}

/// **Un apport recopié sort des rappels par défaut, et se voit en ôtant le filtre** (15 novembre 2026,
/// les gestes de Lucie : scinder ou rejoindre une fiche recopie ses apports sous la fiche issue, et
/// l'exemplaire d'origine reste comme provenance). L'état `recopied` (la transition `recopy` du cycle
/// de vie d'Apport) est écarté par le filtre par défaut de `recall_apports` ; un `filter_condition`
/// explicite, même vide, l'ôte.
#[test]
#[ignore]
fn un_apport_recopie_sort_des_rappels_par_defaut() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    appel(&b, "put_fiche", json!({"record": {"name": FICHE, "about": "Comment le moteur rejoue son journal à l'ouverture.", "state": "current"}}));
    for (section, texte) in [("Vivant", "Le journal se rejoue en entier à l'ouverture, section vivante."), ("Recopié", "Le journal se rejoue en entier à l'ouverture, section recopiée.")] {
        appel(&b, "put_apport", json!({"record": {"fiche": FICHE, "source": CHEMIN, "section": section, "texte": texte, "niveau": "L1", "state": "current"}}));
    }
    appel(&b, "put_apport", json!({"record": {"fiche": FICHE, "source": CHEMIN, "section": "Recopié", "texte": "Le journal se rejoue en entier à l'ouverture, section recopiée.", "niveau": "L1", "state": "recopied"}, "expected_revision": 1}));
    let sections = |options: Value| -> Vec<String> {
        let r = appel(&b, "recall_apports", json!({"query": "journal rejoue ouverture", "options": options}));
        r["result"].as_array().cloned().unwrap_or_default().iter().filter_map(|x| x["data"]["section"].as_str().map(str::to_string)).collect()
    };
    let par_defaut = sections(json!({"limit": 20, "consistency": "strict"}));
    assert!(par_defaut.contains(&"Vivant".to_string()) && !par_defaut.contains(&"Recopié".to_string()), "par défaut, l'exemplaire recopié est écarté : {par_defaut:?}");
    let tout = sections(json!({"limit": 20, "consistency": "strict", "filter_condition": {}}));
    assert!(tout.contains(&"Vivant".to_string()) && tout.contains(&"Recopié".to_string()), "le filtre ôté, il se voit : {tout:?}");
}

/// Une fiche proposée et ses apports, une section chacun, sous la source CHEMIN. Rend l'uuid de la fiche.
fn fiche_et_apports(b: &Backend, nom: &str, sections: &[&str]) -> String {
    let f = appel(b, "put_fiche", json!({"record": {"name": nom, "about": format!("Le sujet {nom}, et tout ce que ses sections en disent."), "state": "proposed"}}));
    for s in sections {
        let texte = format!("La section {s} du sujet {nom}, assez longue pour être une phrase.");
        appel(b, "put_apport", json!({"record": {"fiche": nom, "source": CHEMIN, "section": s, "texte": texte, "niveau": "L1", "state": "current"}}));
        appel(b, "relier_apport", json!({"links": [{"from": {"name": nom}, "to": {"fiche": nom, "source": CHEMIN, "section": s}}]}));
    }
    f["result"]["uuid"].as_str().expect("l'uuid").to_string()
}

/// Les sections que le rappel des apports rend, pour une requête, avec les options données.
fn sections_rappelees(b: &Backend, options: Value) -> Vec<String> {
    let r = appel(b, "recall_apports", json!({"query": "section sujet phrase", "options": options}));
    r["result"].as_array().cloned().unwrap_or_default().iter().filter_map(|x| x["data"]["section"].as_str().map(str::to_string)).collect()
}

/// Les noms des fiches que `cited_by` rend pour une valeur.
fn fiches_qui_citent(b: &Backend, valeur: &str) -> Vec<String> {
    let c = appel(b, "cited_by", json!({"value": valeur, "format": "json"}));
    let fiches: Vec<Value> = serde_json::from_str(c["result"].as_str().unwrap_or("[]")).unwrap_or_default();
    fiches.iter().filter_map(|f| f["data"]["name"].as_str().map(str::to_string)).collect()
}

/// **Scinder une fiche, d'un appel** (15 novembre 2026, les gestes de Lucie au survol). Chaque groupe
/// de sections devient une fiche proposée, avec sa phrase et les copies de ses apports (l'identité d'un
/// apport porte le nom de sa fiche : il se recopie, il ne se déplace pas). L'ancienne est retirée, ses
/// apports passent `recopied` (provenance, hors des rappels par défaut), et chaque fiche neuve porte une
/// Ref `derived_from` vers l'ancienne. Une section oubliée dans les groupes est refusée.
#[test]
#[ignore]
fn une_fiche_se_scinde_par_groupes_de_sections() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    let uuid = fiche_et_apports(&b, "Le thème large", &["Un", "Deux", "Trois"]);
    let e = b.call("scinder_fiche", json!({"uuid": uuid, "groupes": [{"name": "Le premier thème", "about": "Ce que les deux premières sections disent du thème.", "sections": ["Un", "Deux"]}]})).err().expect("une section oubliée est refusée");
    assert!(e.contains("Trois"), "le refus nomme la section oubliée : {e}");
    let r = appel(&b, "scinder_fiche", json!({"uuid": uuid, "groupes": [
        {"name": "Le premier thème", "about": "Ce que les deux premières sections disent du thème.", "sections": ["Un", "Deux"]},
        {"name": "Le second thème", "about": "Ce que la troisième section dit du thème, à part.", "sections": ["Trois"]}]}));
    assert!(r["ok"] != json!(false), "{r}");
    let etat = |nom: &str| appel(&b, "get_fiche", json!({"record": {"name": nom}}))["result"]["data"]["state"].clone();
    assert_eq!(etat("Le thème large"), json!("retired"));
    assert_eq!(etat("Le premier thème"), json!("proposed"));
    assert_eq!(etat("Le second thème"), json!("proposed"));
    assert_eq!(apports_de(&b, "Le premier thème").len(), 2, "les copies sous la fiche issue");
    assert_eq!(apports_de(&b, "Le second thème").len(), 1);
    let defaut = sections_rappelees(&b, json!({"limit": 20, "consistency": "strict"}));
    assert_eq!(defaut.iter().filter(|s| *s == "Un").count(), 1, "par défaut, un seul exemplaire : {defaut:?}");
    let tout = sections_rappelees(&b, json!({"limit": 20, "consistency": "strict", "filter_condition": {}}));
    assert_eq!(tout.iter().filter(|s| *s == "Un").count(), 2, "le filtre ôté, l'exemplaire d'origine se voit : {tout:?}");
    let issues = fiches_qui_citent(&b, "Le thème large");
    assert!(issues.contains(&"Le premier thème".to_string()) && issues.contains(&"Le second thème".to_string()), "la Ref derived_from : {issues:?}");
}

/// **Rejoindre deux fiches, d'un appel** : les apports de A passent sur B (recopiés), A est retirée, et B
/// porte une Ref `alias` qui garde le nom de A.
#[test]
#[ignore]
fn deux_fiches_se_rejoignent_et_le_nom_de_l_une_devient_alias() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    let a = fiche_et_apports(&b, "Hooks de rag3weaver", &["Le binaire"]);
    let cible = fiche_et_apports(&b, "hooks Claude Code de rag3weaver", &["Le rappel"]);
    let r = appel(&b, "rejoindre_fiches", json!({"uuid_a": a, "uuid_b": cible}));
    assert!(r["ok"] != json!(false), "{r}");
    let etat = |nom: &str| appel(&b, "get_fiche", json!({"record": {"name": nom}}))["result"]["data"]["state"].clone();
    assert_eq!(etat("Hooks de rag3weaver"), json!("retired"));
    assert_eq!(etat("hooks Claude Code de rag3weaver"), json!("proposed"));
    assert_eq!(apports_de(&b, "hooks Claude Code de rag3weaver").len(), 2, "ses apports et la copie de ceux de A");
    let defaut = sections_rappelees(&b, json!({"limit": 20, "consistency": "strict"}));
    assert_eq!(defaut.iter().filter(|s| *s == "Le binaire").count(), 1, "{defaut:?}");
    assert!(fiches_qui_citent(&b, "Hooks de rag3weaver").contains(&"hooks Claude Code de rag3weaver".to_string()), "la Ref alias");
}

