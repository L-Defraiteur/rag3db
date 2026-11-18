use std::sync::Arc;

use super::*;
use crate::memory_dialect::MemoryDialect;

fn catalog_on_memory() -> crate::Catalog {
    let mut catalog = crate::Catalog::without_embedding_model(Box::new(MemoryConnection::new()), crate::CatalogConfig::default());
    catalog.set_dialect(Arc::new(MemoryDialect));
    catalog
}

/// **A statement the memory dialect does not translate is refused by its name** — never guessed.
#[test]
fn a_statement_out_of_the_memory_dialect_is_refused_by_its_name() {
    let conn = MemoryConnection::new();
    let refusal = crate::dialect::SchemaDialect::untranslated(&MemoryDialect, "embed_set");
    let e = conn.execute(&refusal).unwrap_err().to_string();
    assert!(e.contains("embed_set"), "{e}");
    let e = conn.execute("MATCH (n) RETURN n").unwrap_err().to_string();
    assert!(e.contains("hors du dialecte mémoire"), "{e}");
}

/// **A catalog initializes on the memory connection** (the families ddl and meta).
#[test]
fn a_catalog_initializes_on_the_memory_connection() {
    let mut catalog = catalog_on_memory();
    catalog.initialize().expect("a catalog initializes on the memory connection");
    catalog.persist_meta_key("probe:key", "value").expect("the meta is written");
    assert_eq!(catalog.read_meta_key("probe:key").expect("the meta is read").as_deref(), Some("value"));
}

/// **The schema of the references registers on the memory connection** (ddl), and the image knows its tables.
#[test]
fn the_references_schema_registers_on_the_memory_connection() {
    let mut catalog = catalog_on_memory();
    catalog.initialize().unwrap();
    crate::memory_resolution::register_code_schema_for_references(&mut catalog).expect("the schema of the references registers");
}
