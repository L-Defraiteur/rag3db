//! E2E : **le modèle de la mémoire, lot M1** — la fiche est le sujet, un apport
//! (L1) par ce qu'une source en a dit, la source (L0) gardée telle quelle
//! (`docs/18-octobre-2026-memoire-longue/04-les-fiches-dans-des-fichiers.md`).
//!
//! Sur le vrai gabarit `memory` (ses signaux, donc un service d'embarquement :
//! `RAG3WEAVER_EMBED_SERVICE`) :
//!
//! - une source, une fiche, un apport rangé sur la fiche, sa provenance par une
//!   Ref `doc:` vers `<source>#<section>`, une ancre `path` ; tout se relit ;
//! - **la source ne change jamais** : la réécrire autrement est refusé ;
//! - la fiche se retrouve par la recherche, l'apport aussi ;
//! - les outils d'avant le modèle (`put_memory`, `put_subject`) marchent encore :
//!   M1 ajoute, il ne retire rien (les outils changent avec l'archiviste, M4).
//!
//! Run with: ./run_e2e.sh --test e2e_memoire_modele

#![cfg(feature = "rag3db-native")]

use std::path::Path;

use serde_json::{json, Value};

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::Rag3dbConnection;

fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Le gabarit `memory` dans une copie de `templates/`, son extension vector
/// prise au bâti (comme le banc des fiches classées).
fn ouvrir(dir: &Path) -> Backend {
    ouvrir_avec(dir, |_, _| {})
}

/// Le même gabarit, et une retouche du dossier copié et du manifeste avant
/// l'ouverture (un modèle déclaré, un outil d'essai).
fn ouvrir_avec(dir: &Path, retoucher: impl Fn(&Path, &mut Value)) -> Backend {
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
    retoucher(&dir.join("templates/backends/memory"), &mut m);
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

const SOURCE: &str = "sources/2026-10-18-essai.md";
const CONTENU: &str = "# Le journal\n\n## Reprise\n\nLe journal se rejoue à l'ouverture, en entier, avant toute écriture.\n";
const FICHE: &str = "le moteur de reprise du WAL";

#[test]
#[ignore]
fn une_source_une_fiche_un_apport_et_sa_provenance() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path());

    appel(&b, "put_source", json!({"record": {"chemin": SOURCE, "titre": "Le journal", "contenu": CONTENU, "origine": "envoi"}}));
    appel(&b, "put_fiche", json!({"record": {"name": FICHE, "about": "Comment le moteur rejoue son journal à l'ouverture.", "state": "current"}}));
    let apport = json!({"fiche": FICHE, "source": SOURCE, "section": "Reprise",
                        "texte": "Le journal se rejoue à l'ouverture, en entier, avant toute écriture.", "niveau": "L1", "state": "current"});
    appel(&b, "put_apport", json!({"record": apport}));
    let lien = appel(&b, "relier_apport", json!({"links": [{"from": {"name": FICHE}, "to": {"fiche": FICHE, "source": SOURCE, "section": "Reprise"}}]}));
    assert!(lien.to_string().contains('1'), "un lien posé : {lien}");

    // La provenance : une Ref doc:<source>#<section> ; l'ancre : une Ref path.
    for (genre, description) in [("doc", "un document source et sa section"), ("path", "un chemin du dépôt")] {
        appel(&b, "create_ref_type", json!({"name": genre, "description": description, "forme": "relatif", "confirm": true}));
    }
    appel(&b, "citer_source", json!({"genre": "doc", "valeur": format!("{SOURCE}#Reprise"),
                                       "fiche": {"fiche": FICHE, "source": SOURCE, "section": "Reprise"}}));
    appel(&b, "ancrer_fiche", json!({"genre": "path", "valeur": "src/storage/wal/replay.cpp", "fiche": {"name": FICHE}}));

    let cat = b.catalog.lock().unwrap();
    let compter = |q: &str| cat.execute_raw_with_params(q, &[]).expect(q).rows[0][0].as_i64().unwrap();
    assert_eq!(compter("MATCH (f:Fiche)-[:A_POUR_APPORT]->(a:Apport {niveau: 'L1'}) RETURN count(*)"), 1);
    assert_eq!(compter("MATCH (a:Apport)-[:APPORT_CITE]->(r:Ref {genre: 'doc'}) WHERE r.valeur = 'sources/2026-10-18-essai.md#Reprise' RETURN count(*)"), 1);
    assert_eq!(compter("MATCH (f:Fiche)-[:ANCRE]->(r:Ref {genre: 'path'}) RETURN count(*)"), 1);
    let gardee = cat
        .execute_raw_with_params("MATCH (s:Source {chemin: 'sources/2026-10-18-essai.md'}) RETURN s.contenu", &[])
        .expect("la source");
    assert_eq!(gardee.rows[0][0].as_str(), Some(CONTENU), "la source est gardée telle quelle");
    drop(cat);

    // La fiche et l'apport se retrouvent par la recherche.
    let r = appel(&b, "recall_fiches", json!({"query": "rejouer le journal", "options": {"limit": 3}}));
    assert!(r.to_string().contains(FICHE), "la fiche se retrouve : {r}");
    let r = appel(&b, "recall_apports", json!({"query": "rejoue à l'ouverture", "options": {"limit": 3}}));
    assert!(r.to_string().contains("Reprise"), "l'apport se retrouve : {r}");
}

#[test]
#[ignore]
fn une_source_ne_change_jamais() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path());
    let source = json!({"chemin": SOURCE, "titre": "Le journal", "contenu": CONTENU, "origine": "envoi"});
    appel(&b, "put_source", json!({"record": source}));
    // La même : rejouée à l'identique, acceptée.
    appel(&b, "put_source", json!({"record": source}));
    // Autre contenu sous le même chemin : refusé.
    let mut autre = source.clone();
    autre["contenu"] = json!("# Autre chose\n");
    let e = b.call("put_source", json!({"record": autre})).err().expect("une source ne se réécrit pas");
    assert!(e.contains("immutable"), "le refus dit pourquoi : {e}");
}

#[test]
#[ignore]
fn les_outils_d_avant_le_modele_marchent_encore() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path());
    appel(&b, "put_subject", json!({"record": {"name": "régime GPU", "about": "le rapport cyclique des passes", "state": "current"}}));
    appel(&b, "put_memory", json!({"record": {"claim": "Un rapport cyclique de 70 % garde l'écran utilisable.", "why": "mesuré", "kind": "fact",
                                              "origin": "inferred", "said_as": "", "reach": "person", "state": "current"}}));
}

/// Un outil d'essai : l'élément donné passe par `Decide via='llm'`.
const DECIDE_ESSAI: &str = "%% tool: decide_essai
%% description: Un Decide par le modèle déclaré, pour le témoin.
%% param: element json! -- L'élément à étiqueter ({text}).
%% result: decide.element

graph LR
    source[\"RhaiNode(script='input', value=$element)\"]
    decide[\"Decide(via='llm', labels='oui|non', field='text', as='verdict')\"]
    source -->|result| decide
";

/// **`models.llm` est joint à l'ouverture et sert les graphes du backend** (M4).
/// Avant, seuls le chat et le juge lisaient la déclaration : un `Decide
/// via='llm'` dans un graphe d'outil disait « ni 'decider' ni 'llm' servi »
/// alors que le manifeste déclarait le modèle. Ici le modèle déclaré est un faux
/// serveur SSE local qui répond `{"label": "oui"}` : l'outil rend l'élément
/// étiqueté, et le serveur a bien reçu la requête. Sans déclaration, c'est
/// « ni decider ni llm ».
#[cfg(feature = "openai-llm")]
#[test]
#[ignore]
fn un_modele_declare_au_manifeste_sert_les_graphes_du_backend() {
    let outil = |dossier: &Path, m: &mut Value| {
        std::fs::write(dossier.join("graphs/decide_essai.mmd"), DECIDE_ESSAI).unwrap();
        m["tools"]["decide_essai"] = json!({"graph": "graphs/decide_essai.mmd"});
    };
    // Sans modèle déclaré : le graphe le dit, et nomme ce qui manque.
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir_avec(dir.path(), outil);
    let e = b.call("decide_essai", json!({"element": {"text": "bonjour"}})).err().expect("sans modèle, l'outil refuse");
    assert!(e.contains("ni 'decider' ni 'llm' servi"), "{e}");
    drop(b);
    // Avec `models.llm` : le modèle est joint à l'ouverture et le graphe l'appelle.
    // Le modèle est un faux serveur SSE local (tests/common/fake_sse.rs) qui
    // répond l'étiquette « oui » en JSON, comme un modèle sous schéma strict.
    let serveur = crate::common::fake_sse::FakeServer::start(
        vec![
            r#"{"id":"d1","choices":[{"index":0,"delta":{"role":"assistant","content":"{\"label\": \"oui\"}"},"finish_reason":null}]}"#.into(),
            r#"{"id":"d1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#.into(),
        ],
        false,
    );
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir_avec(dir.path(), |dossier, m| {
        outil(dossier, m);
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    let r = appel(&b, "decide_essai", json!({"element": {"text": "bonjour"}}));
    assert_eq!(r["verdict"], json!("oui"), "l'élément ressort étiqueté par le modèle déclaré : {r}");
    let requete = serveur.request.recv_timeout(std::time::Duration::from_secs(5)).expect("le faux modèle a reçu la requête");
    assert!(requete.contains("bonjour") && requete.contains("faux"), "la requête porte le texte et le modèle déclaré : {requete}");
}
