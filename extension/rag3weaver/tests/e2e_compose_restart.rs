//! **Le composeur du contexte d'après** (`compose_restart`, page du 15 novembre
//! 2026, « la compaction est l'archiviste ») : dans un budget, par priorité,
//! l'identité, la fiche de tâche, le bilan dans l'ordre des tours, les L1 de la
//! session dans l'ordre où ils ont été pris (la fiche en marge), puis ceux des
//! autres sur les mêmes fiches ; un L1 qui ne tient pas reste au rappel, entier.
//!
//! Sur le vrai gabarit `memory` (ses signaux : `RAG3WEAVER_EMBED_SERVICE`) ; aucun
//! modèle : les apports sont posés par `put_fiche` et `put_apport`.
#![cfg(feature = "rag3db-native")]

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::Rag3dbConnection;
use serde_json::{json, Value};
use std::path::Path;

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
        panic!("le témoin embarque (bge-m3) : posez RAG3WEAVER_EMBED_SERVICE — un témoin sauté n'est pas vert ({e})")
    });
    prepared
        .open(Box::new(Rag3dbConnection::new(&dir.join("memoire.rag3db")).expect("la base")), Some(embedder))
        .expect("l'ouverture")
}

fn appel(b: &Backend, outil: &str, args: Value) -> Value {
    b.call(outil, args.clone()).unwrap_or_else(|e| panic!("{outil} {args} : {e}"))
}

fn fiche(b: &Backend, nom: &str, about: &str) {
    appel(b, "put_fiche", json!({"record": {"name": nom, "about": about, "state": "current"}}));
}

fn apport(b: &Backend, fiche: &str, source: &str, section: &str, texte: &str) {
    appel(b, "put_apport", json!({"record": {"fiche": fiche, "source": source, "section": section, "texte": texte, "niveau": "L1", "state": "current"}}));
}

const S1: &str = "sources/2026-11-15-chat-turns-1-2.md";
const S2: &str = "sources/2026-11-15-chat-turns-3-3.md";
const AUTRE: &str = "sources/2026-11-01-une-autre-session.md";

#[test]
#[ignore]
fn le_contexte_d_apres_se_compose_par_priorite_dans_le_budget() {
    let d = tempfile::tempdir().unwrap();
    let b = ouvrir(d.path());
    fiche(&b, "agent:96", "La session mémoire, qui tient l'archiviste.");
    apport(&b, "agent:96", "sources/identite.md", "identity", "IDENTITEMARQUE je suis la session mémoire, clé 96.");
    fiche(&b, "bilan : la compaction", "TACHEMARQUE écrire le composeur du contexte d'après.");
    fiche(&b, "le journal de reprise", "Comment le moteur rejoue son journal.");
    fiche(&b, "le compositeur", "Le compositeur de l'écran du portable.");
    // Posés dans le désordre : l'ordre rendu est celui des tours, pas celui des écritures.
    apport(&b, "le compositeur", S1, "turns 2-2", "DEUXMARQUE le compositeur fige à cent pour cent.");
    apport(&b, "le journal de reprise", S1, "turns 1-1", "UNMARQUE la reprise relit chaque enregistrement.");
    apport(&b, "le journal de reprise", S2, "turns 3-3", &format!("TROISMARQUE {}", "le journal continue ".repeat(400)));
    apport(&b, "le compositeur", AUTRE, "Écran", "AUTREMARQUE à 70 % l'écran reste utilisable.");

    let r = appel(&b, "compose_restart", json!({
        "sources": [S1, S2], "identity": "agent:96", "task_fiche": "bilan : la compaction", "budget_tokens": 600
    }));
    let r = if r["result"].is_object() { r["result"].clone() } else { r };
    let texte = r["text"].as_str().unwrap_or_else(|| panic!("le texte composé : {r}"));
    eprintln!("{texte}");

    // Le bilan : les trois L1 de la session, dans l'ordre des tours, chacun sa fiche.
    assert_eq!(r["bilan"], json!([
        "turns 1-1 → fiche:le journal de reprise",
        "turns 2-2 → fiche:le compositeur",
        "turns 3-3 → fiche:le journal de reprise"
    ]), "{r}");
    // La priorité : identité, tâche, bilan, puis les L1 de la session.
    let ou = |m: &str| texte.find(m).unwrap_or_else(|| panic!("« {m} » manque au texte composé"));
    assert!(texte.contains("agent:96"), "l'identité se nomme");
    assert!(ou("IDENTITEMARQUE") < ou("TACHEMARQUE") && ou("TACHEMARQUE") < ou("turns 1-1 → ") && ou("turns 1-1 → ") < ou("UNMARQUE"),
        "identité, tâche, bilan, puis les L1");
    // Les L1 dans l'ordre des tours, la fiche en marge de chacun.
    assert!(ou("UNMARQUE") < ou("DEUXMARQUE"), "l'ordre des tours, pas celui des écritures");
    assert!(texte.contains("### turns 1-1 — fiche:le journal de reprise") && texte.contains("### turns 2-2 — fiche:le compositeur"), "la fiche en marge");
    // Le long tour 3 ne tient pas : il reste au rappel, entier, pas coupé.
    assert!(!texte.contains("TROISMARQUE"), "le L1 qui ne tient pas n'est pas coupé dans le texte");
    assert!(r["left_to_recall"].as_array().unwrap().iter().any(|x| x["section"] == "turns 3-3" && x["source"] == S2), "il reste au rappel : {r}");
    // Les autres sessions, sur les mêmes fiches, après la session.
    assert!(ou("DEUXMARQUE") < ou("AUTREMARQUE"), "les L1 des autres viennent après ceux de la session");
    // Le budget, et la règle d'estimation dite.
    assert!(r["tokens"].as_u64().unwrap() <= 600 && r["over_budget"] == false, "{r}");
    assert_eq!(r["estimate"], "ceil(caractères / 4)");
    assert_eq!(r["tokens"].as_u64().unwrap() as usize, texte.chars().count().div_ceil(4), "le compte rendu est celui du texte rendu");
}

#[test]
#[ignore]
fn sans_sources_le_composeur_refuse_en_le_disant() {
    let d = tempfile::tempdir().unwrap();
    let b = ouvrir(d.path());
    let e = b.call("compose_restart", json!({"sources": [], "budget_tokens": 100})).unwrap_err();
    assert!(e.to_string().contains("sources"), "{e}");
}
