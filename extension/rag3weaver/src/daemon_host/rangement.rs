//! **The session archived as it goes** (page 07 of 18 Nov 2026, rag3weaver-archi; Lucie: « the
//! archivist at each slice of tokens, and that's it; at compaction time, everything is already
//! filed but the last slice »). At the end of each turn the host writes the turn's messages to
//! the memory declared by `bilans.memoire` (`put_turn`); once the session holds more than
//! `memory.slice_tokens` since the last archived turn, the slice is filed in the background by
//! the archivist (`archive_slice`), one at a time per conversation, and the next turn never waits.
//! The mark « archived through turn T » moves only on a report without error, to the value the
//! memory returns; it lives in `chats/<backend>.rangement.json`, whose only writer is the
//! archiving thread.

use serde_json::Value;
use std::path::Path;

/// The two calls the host makes to the memory: a seam, so that a witness can play them.
pub trait Rangeur: Send + Sync {
    /// `put_turn` on the memory `memoire`: one message of a turn.
    fn put_turn(&self, memoire: &Path, line: &Value) -> Result<Value, String>;
    /// `archive_slice` on the memory `memoire`: `{run, from_turn, to_turn, context}`; returns
    /// `{report, archived_through_turn}`.
    fn archive_slice(&self, memoire: &Path, args: &Value) -> Result<Value, String>;
}
