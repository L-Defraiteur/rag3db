//! Closed, metadata-only native storage. No Catalog initialization or model access.
use super::*;
use crate::{
    catalog::metadata,
    connection::{CypherValue, DbConnection, QueryParam},
    dialect::{Rag3dbDialect, SchemaDialect},
    Rag3dbConnection,
};
use std::path::{Path, PathBuf};

const KEY: &str = "declared_publication_registry:v1";
const FILE: &str = "declarations.rag3db";
const BUFFER: u64 = 64 * 1024 * 1024;

fn storage(error: impl std::fmt::Display) -> PublicationError {
    PublicationError::Storage(error.to_string())
}
fn connection(root: &Path, read_only: bool) -> Result<Rag3dbConnection> {
    Rag3dbConnection::with_config(
        root.join(FILE),
        rag3db::SystemConfig::default()
            .buffer_pool_size(BUFFER)
            .read_only(read_only),
    )
    .map_err(storage)
}
fn existing(root: &Path) -> Result<()> {
    let dir = std::fs::symlink_metadata(root).map_err(|_| PublicationError::NotOwned)?;
    let file =
        std::fs::symlink_metadata(root.join(FILE)).map_err(|_| PublicationError::NotOwned)?;
    if !dir.is_dir() || !file.is_file() {
        return Err(PublicationError::NotOwned);
    }
    Ok(())
}

// The caller owns the transaction. Both inventory and all metadata are read in it.
fn read_registry(conn: &dyn DbConnection, origin: &str) -> Result<Registry> {
    let inspection =
        crate::storage_inspection::inspect_on(conn).map_err(PublicationError::Corrupt)?;
    if inspection.tables != ["_catalog_meta"] {
        return Err(PublicationError::UnknownContract);
    }
    let rows = conn
        .execute_with_params(
            &Rag3dbDialect.load_meta_by_prefix("prefix"),
            &[QueryParam::new(
                "prefix",
                CypherValue::String(String::new()),
            )],
        )
        .map_err(storage)?;
    let rows = metadata::decode_rows(rows).map_err(PublicationError::Corrupt)?;
    if rows.len() != 1 {
        return Err(PublicationError::UnknownContract);
    }
    let record = Registry::decode(rows.get(KEY).ok_or(PublicationError::UnknownContract)?)?;
    if record.origin != origin {
        return Err(PublicationError::OriginMismatch);
    }
    Ok(record)
}
fn write_registry(conn: &dyn DbConnection, record: &Registry) -> Result<()> {
    record.validate()?;
    metadata::write_value(conn, &Rag3dbDialect, KEY, &canonical(record)?).map_err(storage)
}

// COMMIT errors have an unknown outcome. No receipt escapes, and the connection
// may never be reused or checkpointed from a potentially inconsistent memory view.
fn transaction<T>(
    conn: &dyn DbConnection,
    poisoned: &mut bool,
    read_only: bool,
    action: impl FnOnce(&dyn DbConnection) -> Result<T>,
) -> Result<T> {
    if *poisoned || conn.must_reopen().is_some() {
        return Err(PublicationError::NeedsReopen);
    }
    let begin = if read_only {
        conn.begin_read_only()
    } else {
        conn.begin()
    };
    begin.map_err(storage)?;
    let value = match action(conn) {
        Ok(value) => value,
        Err(error) => {
            return match conn.rollback() {
                Ok(()) => Err(error),
                Err(rollback) => {
                    *poisoned = true;
                    conn.poison("publication rollback failed");
                    conn.close_without_checkpoint();
                    Err(storage(format!("{error}; rollback failed: {rollback}")))
                }
            };
        }
    };
    if let Err(error) = conn.commit() {
        *poisoned = true;
        conn.poison("publication commit outcome unknown");
        conn.close_without_checkpoint();
        let rollback = conn.rollback();
        return Err(storage(match rollback {
            Ok(()) => format!("commit outcome unknown, reopen and resolve operation: {error}"),
            Err(rollback) => format!("commit outcome unknown, reopen and resolve operation: {error}; rollback failed: {rollback}"),
        }));
    }
    Ok(value)
}

/// A private exclusive writer for a dedicated, explicitly new declaration catalog.
/// It never opens or initializes an index and never establishes its storage format.
pub struct OwnedPublicationCatalog {
    conn: Rag3dbConnection,
    origin: String,
    poisoned: bool,
    root: PathBuf,
}
impl OwnedPublicationCatalog {
    pub fn create_new(
        root: impl AsRef<Path>,
        origin: NewPublicationOrigin,
        operation: &str,
        initial: DeclarationSnapshot,
    ) -> Result<Self> {
        // Validate all inputs before creating the directory. Its existence is
        // claimed atomically; an error never removes it or anything inside it.
        let record = Registry::genesis(origin, operation, initial)?;
        record.validate()?;
        let root = root.as_ref();
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(root).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                PublicationError::ExistingPath
            } else {
                storage(e)
            }
        })?;
        let conn = connection(root, false)?;
        let mut poisoned = false;
        transaction(&conn, &mut poisoned, false, |conn| {
            conn.execute(&Rag3dbDialect.create_meta_table())
                .map_err(storage)?;
            write_registry(conn, &record)
        })?;
        Ok(Self {
            conn,
            origin: record.origin,
            poisoned,
            root: root.into(),
        })
    }

    pub fn open_owned(root: impl AsRef<Path>, expected_origin: &str) -> Result<Self> {
        validate_id(expected_origin, "declaration-origin-v1")?;
        let root = root.as_ref();
        existing(root)?;
        {
            let reader = connection(root, true)?;
            transaction(&reader, &mut false, true, |conn| {
                read_registry(conn, expected_origin)
            })?;
        }
        existing(root)?;
        let conn = connection(root, false)?;
        let mut poisoned = false;
        transaction(&conn, &mut poisoned, true, |conn| {
            read_registry(conn, expected_origin)
        })?;
        Ok(Self {
            conn,
            origin: expected_origin.into(),
            poisoned,
            root: root.into(),
        })
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn observe(&mut self) -> Result<PublishedDeclaration> {
        transaction(&self.conn, &mut self.poisoned, true, |conn| {
            let record = read_registry(conn, &self.origin)?;
            record.published(&record.head)
        })
    }
    pub fn prepare(
        &mut self,
        operation: &str,
        target: DeclarationSnapshot,
    ) -> Result<PublicationIntent> {
        transaction(&self.conn, &mut self.poisoned, true, |conn| {
            read_registry(conn, &self.origin)?.prepare(operation, target)
        })
    }
    pub fn publish(&mut self, intent: &PublicationIntent) -> Result<PublicationOutcome> {
        transaction(&self.conn, &mut self.poisoned, false, |conn| {
            let mut record = read_registry(conn, &self.origin)?;
            let outcome = record.apply(intent)?;
            if outcome.disposition == PublicationDisposition::Committed {
                write_registry(conn, &record)?;
            }
            Ok(outcome)
        })
    }
}

#[cfg(test)]
mod tests;
