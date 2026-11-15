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

