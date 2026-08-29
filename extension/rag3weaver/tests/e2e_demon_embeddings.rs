//! E2E : **le démon d'embedding**, avec les vrais poids de BGE-M3.
//!
//! Ce que ce test prouve, et qui est toute la raison d'être du démon : le
//! premier client paie le chargement, **les suivants ne le paient pas**. Le
//! 29 août 2026, une passe E2E rechargeait BGE-M3 sept fois — 2 047 s de
//! chargement contre 1 111 s de tests, parce que sept processus tiraient
//! chacun 2,2 Go vers la même carte.
//!
//! Les poids : `~/.cache/rag3weaver/bge-m3/{model.bpk,tokenizer.json}`, ou
//! `RAG3WEAVER_BGE_M3_BPK` / `RAG3WEAVER_BGE_M3_TOKENIZER`.
//!
//! ```bash
//! cargo test --features daemon,burn-embedder --test e2e_demon_embeddings \
//!   -- --ignored --nocapture
//! ```

#![cfg(all(feature = "daemon", feature = "burn-embedder"))]

use std::time::Instant;

use rag3weaver::daemon::DaemonEmbedder;
use rag3weaver::embedder::{DualEmbedder, Embedder};
use rag3weaver::serveur::{Etat, Fin};

/// Un port que personne ne tient à cet instant.
fn port_libre() -> String {
    let e = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let a = e.local_addr().unwrap().to_string();
    drop(e);
    a
}

fn cosinus(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (na * nb)
}

#[test]
#[ignore = "charge 2,2 Go de poids et lance un processus"]
fn un_modele_charge_une_fois_sert_plusieurs_clients() {
    let adresse = port_libre();
    let journal = std::env::temp_dir().join("rag3weaver-e2e-demon");

    // Le binaire tel que cargo vient de le construire — pas un `which`, pas un
    // chemin en dur : le test sert ce qu'il a compilé.
    let programme = env!("CARGO_BIN_EXE_rag3weaver-embeddings");
    let serveur = DaemonEmbedder::serveur(&adresse, programme)
        .journal_dans(&journal)
        // Hermétique : ce démon-ci ne doit pas survivre au test. `Fin::Laisser`
        // reste le défaut, parce qu'en vrai c'est la survie qu'on veut.
        .fin(Fin::Arreter);

    assert_eq!(serveur.etat(), Etat::Absent, "le port doit être libre au départ");

    // ── Le premier client paie ────────────────────────────────────────────
    let t0 = Instant::now();
    let premier = DaemonEmbedder::assurer(&serveur)
        .unwrap_or_else(|e| panic!("assurer : {e}\n  journal : {}", journal.join("rag3weaver-embeddings.log").display()));
    let chargement = t0.elapsed();
    println!("▸ premier client (a lancé le démon) : {chargement:.1?}");

    let id = premier.identite();
    assert_eq!(id.modele, "bge-m3");
    assert_eq!(id.dim, 1024, "BGE-M3 rend du 1024");
    assert!(id.dual, "BGE-M3 sait rendre dense et creux en une passe");
    assert!(!id.factice, "un démon ne doit jamais blanchir un factice");

    // ── Le second ne paie pas ─────────────────────────────────────────────
    assert_eq!(serveur.etat(), Etat::Repond, "il doit se reconnaître lui-même");
    let t1 = Instant::now();
    let second = DaemonEmbedder::joindre(&adresse).expect("joindre");
    let attache = t1.elapsed();
    println!("▸ second client (s'est attaché)    : {attache:.1?}");

    assert!(
        attache.as_secs_f32() < 1.0,
        "s'attacher doit être instantané, mesuré {attache:?}"
    );
    assert!(
        attache < chargement / 10,
        "toute la thèse du démon : {attache:?} contre {chargement:?}"
    );

    // ── Et c'est bien le même modèle ──────────────────────────────────────
    let textes: Vec<String> = ["un chien qui aboie", "un chat qui miaule", "une base de données en colonnes"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let vecteurs = premier.embed(&textes).expect("embed");
    assert_eq!(vecteurs.len(), 3);
    assert!(vecteurs.iter().all(|v| v.len() == 1024));
    assert!(
        vecteurs.iter().all(|v| v.iter().any(|x| *x != 0.0)),
        "des vecteurs nuls voudraient dire un factice déguisé"
    );

    let anima = cosinus(&vecteurs[0], &vecteurs[1]);
    let hors = cosinus(&vecteurs[0], &vecteurs[2]);
    println!("▸ cos(chien, chat) = {anima:.3}   cos(chien, base de données) = {hors:.3}");
    assert!(
        anima > hors,
        "de vrais embeddings rapprochent les deux animaux : {anima:.3} contre {hors:.3}"
    );

    // Le second client doit obtenir **exactement** les mêmes vecteurs : c'est
    // le même modèle, pas une seconde copie.
    let encore = second.embed(&textes[..1].to_vec()).expect("embed depuis le second");
    assert_eq!(encore[0], vecteurs[0], "un seul modèle, une seule réponse");

    // Le creux traverse aussi.
    let (dense, creux) = second.embed_dual(&textes[..1].to_vec()).expect("embed_dual");
    assert_eq!(dense[0].len(), 1024);
    assert!(!creux[0].indices.is_empty(), "BGE-M3 rend des poids lexicaux");
    assert_eq!(creux[0].indices.len(), creux[0].values.len());
    println!("▸ creux : {} poids lexicaux", creux[0].nnz());

    println!(
        "\n  Économie sur cette passe : {:.1?} — et c'est par processus de test qui aurait rechargé.",
        chargement.saturating_sub(attache)
    );
}
