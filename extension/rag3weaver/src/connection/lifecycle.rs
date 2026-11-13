//! Revocation belongs to a catalog connection family, not to a backend's
//! optional poison hook. Writer, readers and escaped Arc clones share it.
use super::{BufferPoolChoice, DbConnection, DbError, QueryParam, QueryResult};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub(crate) struct ConnectionLifecycle(Mutex<Option<String>>);
impl ConnectionLifecycle {
    pub(crate) fn invalidate(&self, reason: &str) {
        let mut failure = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if failure.is_none() {
            *failure = Some(reason.into());
        }
    }
    pub(crate) fn reason(&self) -> Option<String> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}
pub(crate) struct RevocableConnection<C: DbConnection + ?Sized> {
    inner: Arc<C>,
    lifecycle: Arc<ConnectionLifecycle>,
}
impl<C: DbConnection + ?Sized> RevocableConnection<C> {
    pub(crate) fn new(inner: Arc<C>, lifecycle: Arc<ConnectionLifecycle>) -> Self {
        Self { inner, lifecycle }
    }
    fn check(&self) -> Result<(), DbError> {
        match self.must_reopen() {
            Some(reason) => Err(DbError::MustReopen(reason)),
            None => Ok(()),
        }
    }
}
impl<C: DbConnection + ?Sized> DbConnection for RevocableConnection<C> {
    fn execute(&self, query: &str) -> Result<QueryResult, DbError> {
        self.check()?;
        self.inner.execute(query)
    }
    fn execute_with_params(
        &self,
        query: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        self.check()?;
        self.inner.execute_with_params(query, params)
    }
    fn must_reopen(&self) -> Option<String> {
        self.lifecycle.reason().or_else(|| self.inner.must_reopen())
    }
    fn buffer_pool(&self) -> Option<BufferPoolChoice> {
        self.inner.buffer_pool()
    }
    fn poison(&self, reason: &str) {
        self.lifecycle.invalidate(reason);
        self.inner.poison(reason);
    }
    fn read_connection(&self, threads: u64) -> Option<Arc<dyn DbConnection>> {
        if self.must_reopen().is_some() {
            return None;
        }
        self.inner.read_connection(threads).map(|reader| {
            Arc::new(RevocableConnection::new(reader, self.lifecycle.clone()))
                as Arc<dyn DbConnection>
        })
    }
    fn close_without_checkpoint(&self) {
        self.inner.close_without_checkpoint();
    }
    // Delegating these methods is essential for pooled backends. Replacing
    // them by textual SQL would lose their pinned transaction session.
    fn begin(&self) -> Result<(), DbError> {
        self.check()?;
        self.inner.begin()
    }
    fn begin_read_only(&self) -> Result<(), DbError> {
        self.check()?;
        self.inner.begin_read_only()
    }
    fn commit(&self) -> Result<(), DbError> {
        self.check()?;
        self.inner.commit()
    }
    // Rollback is a cleanup capability after revocation, not a new operation.
    fn rollback(&self) -> Result<(), DbError> {
        self.inner.rollback()
    }
    fn execute_with_local_settings(
        &self,
        query: &str,
        params: &[QueryParam],
        settings: &[(&str, String)],
    ) -> Result<QueryResult, DbError> {
        self.check()?;
        self.inner
            .execute_with_local_settings(query, params, settings)
    }
    fn database_path(&self) -> Option<std::path::PathBuf> {
        self.inner.database_path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Backend {
        calls: Arc<AtomicUsize>,
    }
    impl DbConnection for Backend {
        fn execute(&self, _: &str) -> Result<QueryResult, DbError> {
            panic!("textual transaction fallback must not be used")
        }
        fn execute_with_params(&self, _: &str, _: &[QueryParam]) -> Result<QueryResult, DbError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(QueryResult::default())
        }
        fn begin(&self) -> Result<(), DbError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn begin_read_only(&self) -> Result<(), DbError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn commit(&self) -> Result<(), DbError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn rollback(&self) -> Result<(), DbError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn execute_with_local_settings(
            &self,
            _: &str,
            _: &[QueryParam],
            _: &[(&str, String)],
        ) -> Result<QueryResult, DbError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(QueryResult::default())
        }
        fn read_connection(&self, _: u64) -> Option<Arc<dyn DbConnection>> {
            Some(Arc::new(Backend {
                calls: self.calls.clone(),
            }))
        }
    }
    #[test]
    fn revocation_preserves_backend_transactions_and_reaches_existing_readers() {
        let calls = Arc::new(AtomicUsize::new(0));
        let lifecycle = Arc::new(ConnectionLifecycle::default());
        let writer: Arc<dyn DbConnection> = Arc::new(RevocableConnection::new(
            Arc::new(Backend {
                calls: calls.clone(),
            }),
            lifecycle,
        ));
        let reader = writer.read_connection(2).unwrap();
        writer.begin().unwrap();
        writer.begin_read_only().unwrap();
        writer.commit().unwrap();
        writer
            .execute_with_local_settings("SELECT", &[], &[("setting", "value".into())])
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 4);
        writer.poison("commit outcome unknown");
        let before = calls.load(Ordering::SeqCst);
        assert!(reader.execute_with_params("SELECT", &[]).is_err());
        assert!(writer.execute("WRITE").is_err());
        assert!(writer.begin().is_err());
        assert!(writer.commit().is_err());
        assert!(reader
            .execute_with_local_settings("SELECT", &[], &[])
            .is_err());
        assert!(writer.read_connection(2).is_none());
        assert_eq!(calls.load(Ordering::SeqCst), before);
        writer.rollback().unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), before + 1);
        assert!(reader.must_reopen().is_some());
    }
}
