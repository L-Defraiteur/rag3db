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


/// **A connection that notes every COPY** it is asked to run, and delegates the rest.
struct CopyWitness {
    inner: Rag3dbConnection,
    copies: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl crate::connection::DbConnection for CopyWitness {
    fn execute(&self, cypher: &str) -> Result<crate::connection::QueryResult, crate::connection::DbError> {
        if cypher.trim_start().starts_with("COPY ") {
            self.copies.lock().unwrap().push(cypher.to_string());
        }
        self.inner.execute(cypher)
    }
    fn execute_with_params(
        &self,
        cypher: &str,
        params: &[crate::connection::QueryParam],
    ) -> Result<crate::connection::QueryResult, crate::connection::DbError> {
        self.inner.execute_with_params(cypher, params)
    }
    fn must_reopen(&self) -> Option<String> {
        self.inner.must_reopen()
    }
    fn buffer_pool(&self) -> Option<crate::connection::BufferPoolChoice> {
        self.inner.buffer_pool()
    }
    fn poison(&self, reason: &str) {
        self.inner.poison(reason)
    }
    fn close_without_checkpoint(&self) {
        self.inner.close_without_checkpoint()
    }
    fn begin(&self) -> Result<(), crate::connection::DbError> {
        self.inner.begin()
    }
    fn begin_read_only(&self) -> Result<(), crate::connection::DbError> {
        self.inner.begin_read_only()
    }
    fn commit(&self) -> Result<(), crate::connection::DbError> {
        self.inner.commit()
    }
    fn rollback(&self) -> Result<(), crate::connection::DbError> {
        self.inner.rollback()
    }
    fn database_path(&self) -> Option<std::path::PathBuf> {
        self.inner.database_path()
    }
}

/// **One pair already linked does not send the whole group to MERGE** (ticket
/// `docs/codeparsers/tickets/2026-11-18-la-pose-des-liens-de-la-resolution-repasse-par-merge.md`).
/// A `CONSUMES` with its properties is posed, then a group of 300 that repeats it with a new usage: the 299 new
/// pairs go by COPY, the repeated one by MERGE, and the repeated pair keeps the group's value, as under MERGE.
#[test]
fn a_link_group_with_one_pair_already_linked_still_copies_its_new_pairs() {
    let d = tempfile::tempdir().unwrap();
    let copies = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let conn = CopyWitness { inner: Rag3dbConnection::new(&d.path().join("base.rag3db")).expect("la base"), copies: copies.clone() };
    let mut cat = Catalog::new(
        Box::new(conn),
        Box::new(HashEmbedder::new(8)),
        CatalogConfig { allow_mock_embedder: true, embedding_dim: 8, ..Default::default() },
    );
    cat.initialize().unwrap();
    crate::code::register_code_schema(&mut cat, crate::code::default_scope_chunking()).unwrap();
    for lot in (0..=310).collect::<Vec<usize>>().chunks(100) {
        let liste: Vec<String> = lot.iter().map(|i| format!("'s{i}'")).collect();
        cat.execute_raw(&format!("UNWIND [{}] AS u CREATE (:Scope {{_uuid: u}})", liste.join(","))).unwrap();
    }
    let usage = |u: &str| BTreeMap::from([("usage".to_string(), CypherValue::String(u.to_string()))]);
    cat.mettre_en_file_les_liens("CONSUMES", vec![("s0".to_string(), "s1".to_string(), usage("call"))]).unwrap();
    cat.flush_insertions();
    copies.lock().unwrap().clear();
    let mut group: Vec<(String, String, BTreeMap<String, CypherValue>)> = (2..301).map(|i| ("s0".to_string(), format!("s{i}"), usage("call"))).collect();
    group.push(("s0".to_string(), "s1".to_string(), usage("type")));
    cat.mettre_en_file_les_liens("CONSUMES", group).unwrap();
    cat.flush_insertions();
    let copied: Vec<String> = copies.lock().unwrap().iter().filter(|c| c.contains("CONSUMES")).cloned().collect();
    assert!(!copied.is_empty(), "the 299 new pairs go by COPY, not the whole group by MERGE");
    let n = cat.execute_raw("MATCH (:Scope {_uuid: 's0'})-[r:CONSUMES]->(:Scope) RETURN count(r)").unwrap().rows[0][0].as_i64().unwrap();
    assert_eq!(n, 300, "every pair is there once");
    let u = cat.execute_raw("MATCH (:Scope {_uuid: 's0'})-[r:CONSUMES]->(:Scope {_uuid: 's1'}) RETURN r.usage").unwrap();
    assert_eq!(u.rows[0][0].as_str(), Some("type"), "the pair already linked receives the group's value, as under MERGE");
}
