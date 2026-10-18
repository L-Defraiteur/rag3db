//! **Un COPY de liens ne part jamais d'un nœud déjà lié** (filet du ticket
//! `docs/tickets/2026-10-18-un-copy-de-relations-vers-un-noeud-deja-relie-perd-ses-relations-persistantes.md`,
//! 42a32bd52, 18 octobre 2026). Le moteur fait disparaître les relations
//! persistantes d'un nœud quand un COPY de la même table le touche. Le produit
//! copie en masse dès 200 arêtes d'un même groupe : un second arbre de code, un
//! premier index repris, la mémoire longue. Codeparsers l'a mesuré sur
//! code-lourd : des `HAS_PARENT`/`PARENT_OF` et des `INHERITS_FROM` perdus.
//!
//! **Ce qu'ils montrent** (18 octobre 2026) : verts sans filet. Le chemin CSV
//! du produit (`LinkRecordNode`, sans propriété) n'atteint pas le défaut du
//! moteur sur ces formes. Ils restent comme garde : un changement du chemin
//! d'écriture qui l'atteindrait les ferait rougir.
//!
//! Chaque cas : dix `MENTIONS` posées, un point de reprise, puis trois cents
//! de plus drainées ensemble, qui partent du même scope ou arrivent sur le
//! même symbole ; le compte doit tout garder. La transaction explicite est le
//! chemin du premier index (une transaction par paquet).

use crate::connection::CypherValue;
use crate::embedder::HashEmbedder;
use crate::{Catalog, CatalogConfig, Rag3dbConnection};
use std::collections::BTreeMap;

fn catalogue(d: &std::path::Path) -> Catalog {
    let conn = Rag3dbConnection::new(&d.join("base.rag3db")).expect("la base");
    let mut cat = Catalog::new(
        Box::new(conn),
        Box::new(HashEmbedder::new(8)),
        CatalogConfig { allow_mock_embedder: true, embedding_dim: 8, ..Default::default() },
    );
    cat.initialize().unwrap();
    crate::code::register_code_schema(&mut cat, crate::code::default_scope_chunking()).unwrap();
    for (table, prefixe) in [("Scope", "s"), ("Symbol", "y")] {
        for lot in (0..=310).collect::<Vec<usize>>().chunks(100) {
            let liste: Vec<String> = lot.iter().map(|i| format!("'{prefixe}{i}'")).collect();
            cat.execute_raw(&format!("UNWIND [{}] AS u CREATE (:{table} {{_uuid: u}})", liste.join(","))).unwrap();
        }
    }
    cat
}

fn compter(cat: &Catalog) -> i64 {
    cat.execute_raw("MATCH (:Scope)-[r:MENTIONS]->(:Symbol) RETURN count(r)").unwrap().rows[0][0].as_i64().unwrap()
}

/// Comment se termine le second drain.
#[derive(Clone, Copy, PartialEq)]
enum Fin {
    Automatique,
    Commit,
    Rollback,
}

/// Dix liens, un point de reprise, trois cents de plus ; le second drain en
/// transaction automatique, ou dans un `BEGIN … COMMIT` / `BEGIN … ROLLBACK`.
fn jouer(premiers: Vec<(String, String)>, suivants: Vec<(String, String)>, fin: Fin) -> i64 {
    let d = tempfile::tempdir().unwrap();
    let mut cat = catalogue(d.path());
    let avec = |v: Vec<(String, String)>| v.into_iter().map(|(a, b)| (a, b, BTreeMap::<String, CypherValue>::new())).collect::<Vec<_>>();
    cat.mettre_en_file_les_liens("MENTIONS", avec(premiers)).unwrap();
    cat.flush_insertions();
    assert_eq!(compter(&cat), 10);
    cat.conn_arc().execute("CHECKPOINT").unwrap();
    cat.mettre_en_file_les_liens("MENTIONS", avec(suivants)).unwrap();
    if fin != Fin::Automatique {
        cat.conn_arc().begin().unwrap();
    }
    cat.flush_insertions();
    match fin {
        Fin::Automatique => {}
        Fin::Commit => cat.conn_arc().commit().unwrap(),
        Fin::Rollback => cat.conn_arc().rollback().unwrap(),
    }
    compter(&cat)
}

fn du_scope_zero(r: std::ops::Range<usize>) -> Vec<(String, String)> {
    r.map(|i| ("s0".to_string(), format!("y{i}"))).collect()
}

fn vers_le_symbole_zero(r: std::ops::Range<usize>) -> Vec<(String, String)> {
    r.map(|i| (format!("s{i}"), "y0".to_string())).collect()
}

#[test]
fn un_second_drain_depuis_une_source_deja_liee_compte_juste() {
    assert_eq!(jouer(du_scope_zero(1..11), du_scope_zero(11..311), Fin::Automatique), 310, "les dix liens persistants du scope sont toujours là");
}

#[test]
fn un_second_drain_depuis_une_source_deja_liee_dans_une_transaction_compte_juste() {
    assert_eq!(jouer(du_scope_zero(1..11), du_scope_zero(11..311), Fin::Commit), 310, "les dix liens persistants du scope sont toujours là");
}

#[test]
fn un_second_drain_vers_une_arrivee_deja_liee_dans_une_transaction_compte_juste() {
    assert_eq!(jouer(vers_le_symbole_zero(1..11), vers_le_symbole_zero(11..311), Fin::Commit), 310, "les dix liens persistants du symbole sont toujours là");
}

/// Le cas « 0 après un ROLLBACK » de la recette du cœur : `RelationBatchNode`
/// (la mémoire longue) défait son lot de liens quand il échoue.
#[test]
fn un_second_drain_depuis_une_source_deja_liee_defait_garde_les_dix() {
    assert_eq!(jouer(du_scope_zero(1..11), du_scope_zero(11..311), Fin::Rollback), 10, "le ROLLBACK ne défait que le second drain");
}
