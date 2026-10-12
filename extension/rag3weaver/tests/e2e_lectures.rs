#![cfg(feature = "rag3db-native")]
//! **Des lectures qui n'attendent pas l'écrivain** (page
//! 12-octobre-2026-arbre-principal/02). Lot L1 : la connexion de lecture.
//! Une lecture d'outil voit l'état validé pendant qu'une transaction
//! d'écriture est ouverte sur la connexion principale, sans l'attendre et
//! sans y entrer.

use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn compte(cat: &Catalog, lecture: bool) -> i64 {
    let q = "MATCH (n:Essai) RETURN count(n)";
    let r = if lecture { cat.execute_read_with_params(q, &[]) } else { cat.execute_raw(q) };
    r.unwrap().rows[0][0].as_i64().unwrap()
}

/// **L'état d'avant, sans attendre** : une ligne validée, une seconde écrite
/// dans une transaction ouverte. La lecture la voit absente, et rend la main
/// tout de suite (dans le même fil : si elle attendait l'écrivain, le test ne
/// finirait pas). L'écrivain, lui, voit sa propre écriture. Après le COMMIT,
/// la lecture voit les deux.
#[test]
#[ignore]
fn une_lecture_voit_l_etat_valide_pendant_une_transaction_d_ecriture() {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let cat = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(8)), CatalogConfig::default());
    cat.execute_raw("CREATE NODE TABLE Essai(id INT64 PRIMARY KEY)").unwrap();
    cat.execute_raw("CREATE (:Essai {id: 1})").unwrap();
    assert_eq!(compte(&cat, true), 1);

    let ecrivain = cat.conn_arc();
    ecrivain.begin().unwrap();
    cat.execute_raw("CREATE (:Essai {id: 2})").unwrap();
    assert_eq!(compte(&cat, false), 2, "l'écrivain voit sa propre écriture");
    let t = std::time::Instant::now();
    assert_eq!(compte(&cat, true), 1, "la lecture voit l'état validé, pas la transaction ouverte");
    let ms = t.elapsed().as_millis();
    eprintln!("[lecture] pendant la transaction : {ms} ms");
    assert!(ms < 100, "la lecture n'attend pas l'écrivain : {ms} ms");
    ecrivain.commit().unwrap();
    assert_eq!(compte(&cat, true), 2, "après le COMMIT, la lecture voit les deux");
}

/// **Une lecture qui échoue ne défait pas l'écrivain** : une lecture fautive
/// sur la connexion de lecture, pendant la transaction, laisse la transaction
/// de l'écrivain entière (avant L1, une lecture sur la même connexion tournait
/// dans la transaction, et son échec la défaisait).
#[test]
#[ignore]
fn une_lecture_fautive_ne_defait_pas_la_transaction_de_l_ecrivain() {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let cat = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(8)), CatalogConfig::default());
    cat.execute_raw("CREATE NODE TABLE Essai(id INT64 PRIMARY KEY)").unwrap();
    let ecrivain = cat.conn_arc();
    ecrivain.begin().unwrap();
    cat.execute_raw("CREATE (:Essai {id: 7})").unwrap();
    assert!(cat.execute_read_with_params("MATCH (n:Absente) RETURN n", &[]).is_err(), "la lecture fautive échoue");
    ecrivain.commit().expect("la transaction de l'écrivain tient");
    assert_eq!(compte(&cat, true), 1);
}
