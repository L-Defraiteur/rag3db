//! Write-back buffer in front of a [`BlobStore`], flushed at explicit boundaries.
//!
//! lucivy's `ManagedDirectory` rewrites `.managed.json` once per registered
//! file, and a dirty commit registers dozens (segment files + SFX sidecars, per
//! shard). Through [`CypherBlobStore`](crate::cypher_blob_store::CypherBlobStore)
//! every one of those writes is a `MERGE … SET b._data` round-trip with a binary
//! payload — invisible with `MemBlobStore`, very real against the database.
//!
//! This buffer absorbs the writes and pushes only the **last** version of each
//! key at [`flush`](BufferedBlobStore::flush). Two properties make it safe:
//!
//! - **Read-your-writes.** Every read (`load`, `exists`, `list`, `blob_len`,
//!   `load_range`) consults the buffer before the backend. lucivy re-reads what
//!   it just wrote during a commit; serving a stale version would corrupt the
//!   index silently.
//! - **Deletes are not buffered.** They go straight through and drop any pending
//!   write for the same key, so a flush can never resurrect a deleted file.
//!
//! What it changes: between a lucivy `commit()` and our `flush()`, the index
//! lives only in memory. The `Catalog` flushes at the end of every drain, after
//! reindex, on shutdown and on drop, so the window is a few milliseconds inside
//! one call — and nothing was durable before the drain either.

use std::collections::BTreeMap;
use std::io;
use std::sync::Mutex;

use lucivy_core::blob_store::BlobStore;

/// Counters exposed for profiling: how many round-trips the buffer absorbed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlushStats {
    /// `save()` calls received since the previous flush.
    pub saves_received: usize,
    /// Bytes handed to `save()` since the previous flush.
    pub bytes_received: usize,
    /// Distinct keys actually pushed to the backend by this flush.
    pub saves_pushed: usize,
    /// Bytes actually pushed by this flush.
    pub bytes_pushed: usize,
}

impl FlushStats {
    /// Round-trips this flush did *not* make, compared to writing through.
    pub fn round_trips_saved(&self) -> usize {
        self.saves_received.saturating_sub(self.saves_pushed)
    }
}

/// Backend that can persist several blobs in one round-trip.
///
/// The default loops over [`BlobStore::save`]; a database-backed store can
/// override it with a single batched statement.
pub trait BatchSave: BlobStore {
    fn save_many(&self, items: Vec<(String, String, Vec<u8>)>) -> io::Result<()> {
        for (index_name, file_name, data) in items {
            self.save(&index_name, &file_name, &data)?;
        }
        Ok(())
    }
}

struct Pending {
    /// `(index_name, file_name) -> data`. BTreeMap so a flush pushes keys in
    /// a stable order — `list()` output and error messages stay reproducible.
    writes: BTreeMap<(String, String), Vec<u8>>,
    saves_received: usize,
    bytes_received: usize,
}

pub struct BufferedBlobStore<S: BatchSave> {
    inner: S,
    pending: Mutex<Pending>,
}

impl<S: BatchSave> BufferedBlobStore<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            pending: Mutex::new(Pending {
                writes: BTreeMap::new(),
                saves_received: 0,
                bytes_received: 0,
            }),
        }
    }

    /// The wrapped backend, for callers that need it directly.
    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// Number of keys waiting to be pushed.
    pub fn pending_len(&self) -> usize {
        self.lock().writes.len()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Pending> {
        // A poisoned lock means a panic mid-`save`; the map itself is still
        // consistent (each mutation is a single insert/remove), so keep going.
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Push every pending write to the backend, last version wins.
    ///
    /// On failure the unpushed entries are put back, so the next flush (or the
    /// drop-time flush) retries rather than losing them.
    pub fn flush(&self) -> io::Result<FlushStats> {
        let (items, mut stats) = {
            let mut p = self.lock();
            let stats = FlushStats {
                saves_received: p.saves_received,
                bytes_received: p.bytes_received,
                saves_pushed: 0,
                bytes_pushed: 0,
            };
            p.saves_received = 0;
            p.bytes_received = 0;
            let items: Vec<(String, String, Vec<u8>)> = std::mem::take(&mut p.writes)
                .into_iter()
                .map(|((i, f), d)| (i, f, d))
                .collect();
            (items, stats)
        };

        if items.is_empty() {
            return Ok(stats);
        }

        stats.saves_pushed = items.len();
        stats.bytes_pushed = items.iter().map(|(_, _, d)| d.len()).sum();

        // Keep a copy to restore on failure. Cloning here costs one extra pass
        // over the payload; a flush that fails must not lose an index.
        let backup = items.clone();
        match self.inner.save_many(items) {
            Ok(()) => Ok(stats),
            Err(e) => {
                let mut p = self.lock();
                for (i, f, d) in backup {
                    // A write that arrived during the failed push is newer —
                    // don't overwrite it with the stale backup.
                    p.writes.entry((i, f)).or_insert(d);
                }
                p.saves_received += stats.saves_received;
                p.bytes_received += stats.bytes_received;
                Err(e)
            }
        }
    }
}

impl<S: BatchSave> BlobStore for BufferedBlobStore<S> {
    fn save(&self, index_name: &str, file_name: &str, data: &[u8]) -> io::Result<()> {
        let mut p = self.lock();
        p.saves_received += 1;
        p.bytes_received += data.len();
        p.writes
            .insert((index_name.to_string(), file_name.to_string()), data.to_vec());
        Ok(())
    }

    fn load(&self, index_name: &str, file_name: &str) -> io::Result<Vec<u8>> {
        if let Some(d) = self
            .lock()
            .writes
            .get(&(index_name.to_string(), file_name.to_string()))
        {
            return Ok(d.clone());
        }
        self.inner.load(index_name, file_name)
    }

    fn delete(&self, index_name: &str, file_name: &str) -> io::Result<()> {
        // Drop the pending write first: if the backend delete then fails, the
        // buffer still can't resurrect the file on the next flush.
        self.lock()
            .writes
            .remove(&(index_name.to_string(), file_name.to_string()));
        self.inner.delete(index_name, file_name)
    }

    fn exists(&self, index_name: &str, file_name: &str) -> io::Result<bool> {
        if self
            .lock()
            .writes
            .contains_key(&(index_name.to_string(), file_name.to_string()))
        {
            return Ok(true);
        }
        self.inner.exists(index_name, file_name)
    }

    fn list(&self, index_name: &str) -> io::Result<Vec<String>> {
        let mut names = self.inner.list(index_name)?;
        {
            let p = self.lock();
            for (i, f) in p.writes.keys() {
                if i == index_name && !names.contains(f) {
                    names.push(f.clone());
                }
            }
        }
        names.sort();
        Ok(names)
    }

    fn blob_len(&self, index_name: &str, file_name: &str) -> io::Result<Option<u64>> {
        if let Some(d) = self
            .lock()
            .writes
            .get(&(index_name.to_string(), file_name.to_string()))
        {
            return Ok(Some(d.len() as u64));
        }
        self.inner.blob_len(index_name, file_name)
    }

    fn load_range(
        &self,
        index_name: &str,
        file_name: &str,
        range: std::ops::Range<u64>,
    ) -> io::Result<Option<Vec<u8>>> {
        if let Some(d) = self
            .lock()
            .writes
            .get(&(index_name.to_string(), file_name.to_string()))
        {
            let start = (range.start as usize).min(d.len());
            let end = (range.end as usize).min(d.len()).max(start);
            return Ok(Some(d[start..end].to_vec()));
        }
        self.inner.load_range(index_name, file_name, range)
    }
}

impl<S: BatchSave> Drop for BufferedBlobStore<S> {
    /// Last line of defence: a `Catalog` flushes explicitly, but a buffer that
    /// dies with pending writes would lose an index. Best effort, loud on error.
    fn drop(&mut self) {
        if self.pending_len() == 0 {
            return;
        }
        if let Err(e) = self.flush() {
            eprintln!(
                "[rag3weaver] BufferedBlobStore dropped with {} unflushed blob(s), flush failed: {e}",
                self.pending_len()
            );
        }
    }
}

impl BatchSave for lucivy_core::blob_store::MemBlobStore {}

#[cfg(test)]
mod tests {
    use super::*;
    use lucivy_core::blob_store::MemBlobStore;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// MemBlobStore that counts backend `save` calls.
    struct Counting {
        inner: MemBlobStore,
        saves: Arc<AtomicUsize>,
    }
    impl BlobStore for Counting {
        fn load(&self, i: &str, f: &str) -> io::Result<Vec<u8>> { self.inner.load(i, f) }
        fn save(&self, i: &str, f: &str, d: &[u8]) -> io::Result<()> {
            self.saves.fetch_add(1, Ordering::SeqCst);
            self.inner.save(i, f, d)
        }
        fn delete(&self, i: &str, f: &str) -> io::Result<()> { self.inner.delete(i, f) }
        fn exists(&self, i: &str, f: &str) -> io::Result<bool> { self.inner.exists(i, f) }
        fn list(&self, i: &str) -> io::Result<Vec<String>> { self.inner.list(i) }
        fn blob_len(&self, i: &str, f: &str) -> io::Result<Option<u64>> { self.inner.blob_len(i, f) }
        fn load_range(&self, i: &str, f: &str, r: std::ops::Range<u64>) -> io::Result<Option<Vec<u8>>> {
            self.inner.load_range(i, f, r)
        }
    }
    impl BatchSave for Counting {}

    fn counting() -> (BufferedBlobStore<Counting>, Arc<AtomicUsize>) {
        let saves = Arc::new(AtomicUsize::new(0));
        let store = Counting { inner: MemBlobStore::new(), saves: saves.clone() };
        (BufferedBlobStore::new(store), saves)
    }

    #[test]
    fn repeated_writes_collapse_to_one_round_trip() {
        let (b, saves) = counting();
        for v in 0..20u8 {
            b.save("idx", ".managed.json", &[v]).unwrap();
        }
        assert_eq!(saves.load(Ordering::SeqCst), 0, "nothing pushed before flush");

        let stats = b.flush().unwrap();
        assert_eq!(saves.load(Ordering::SeqCst), 1);
        assert_eq!(stats.saves_received, 20);
        assert_eq!(stats.saves_pushed, 1);
        assert_eq!(stats.round_trips_saved(), 19);
        assert_eq!(b.inner().load("idx", ".managed.json").unwrap(), vec![19], "last version wins");
    }

    #[test]
    fn reads_see_pending_writes() {
        let (b, _) = counting();
        b.save("idx", "seg.bin", b"hello world").unwrap();

        assert!(b.exists("idx", "seg.bin").unwrap());
        assert_eq!(b.load("idx", "seg.bin").unwrap(), b"hello world");
        assert_eq!(b.blob_len("idx", "seg.bin").unwrap(), Some(11));
        assert_eq!(b.load_range("idx", "seg.bin", 6..11).unwrap(), Some(b"world".to_vec()));
        assert_eq!(b.list("idx").unwrap(), vec!["seg.bin"]);
        assert!(!b.inner().exists("idx", "seg.bin").unwrap(), "backend untouched until flush");
    }

    #[test]
    fn list_merges_backend_and_pending() {
        let (b, _) = counting();
        b.inner().save("idx", "old.bin", b"x").unwrap();
        b.save("idx", "new.bin", b"y").unwrap();
        b.save("other", "elsewhere.bin", b"z").unwrap();
        assert_eq!(b.list("idx").unwrap(), vec!["new.bin", "old.bin"]);
    }

    #[test]
    fn delete_cancels_pending_write_and_reaches_backend() {
        let (b, saves) = counting();
        // Seed the backend directly, bypassing the counter.
        b.inner().inner.save("idx", "stale.bin", b"old").unwrap();
        b.save("idx", "stale.bin", b"new").unwrap();
        b.delete("idx", "stale.bin").unwrap();

        assert!(!b.exists("idx", "stale.bin").unwrap());
        assert!(!b.inner().exists("idx", "stale.bin").unwrap(), "delete went through");
        let stats = b.flush().unwrap();
        assert_eq!(stats.saves_pushed, 0, "a deleted key must not be resurrected");
        assert_eq!(saves.load(Ordering::SeqCst), 0, "no backend save at all");
    }

    #[test]
    fn save_after_delete_is_kept() {
        let (b, _) = counting();
        b.delete("idx", "f.bin").unwrap();
        b.save("idx", "f.bin", b"reborn").unwrap();
        b.flush().unwrap();
        assert_eq!(b.inner().load("idx", "f.bin").unwrap(), b"reborn");
    }

    #[test]
    fn flush_on_empty_buffer_is_free() {
        let (b, saves) = counting();
        let stats = b.flush().unwrap();
        assert_eq!(stats, FlushStats::default());
        assert_eq!(saves.load(Ordering::SeqCst), 0);
    }

    /// A backend that refuses every save, to check nothing is lost on failure.
    struct Refusing;
    impl BlobStore for Refusing {
        fn load(&self, _: &str, _: &str) -> io::Result<Vec<u8>> { Err(io::Error::new(io::ErrorKind::NotFound, "no")) }
        fn save(&self, _: &str, _: &str, _: &[u8]) -> io::Result<()> { Err(io::Error::new(io::ErrorKind::Other, "db down")) }
        fn delete(&self, _: &str, _: &str) -> io::Result<()> { Ok(()) }
        fn exists(&self, _: &str, _: &str) -> io::Result<bool> { Ok(false) }
        fn list(&self, _: &str) -> io::Result<Vec<String>> { Ok(vec![]) }
    }
    impl BatchSave for Refusing {}

    #[test]
    fn failed_flush_keeps_pending_for_retry() {
        let b = BufferedBlobStore::new(Refusing);
        b.save("idx", "a.bin", b"1").unwrap();
        b.save("idx", "b.bin", b"2").unwrap();

        assert!(b.flush().is_err());
        assert_eq!(b.pending_len(), 2, "unpushed writes must survive a failed flush");
        assert_eq!(b.load("idx", "a.bin").unwrap(), b"1", "and remain readable");

        // Drop with pending + refusing backend: must not panic, only warn.
        drop(b);
    }
}
