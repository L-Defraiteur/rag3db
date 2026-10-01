#pragma once

#include <functional>

#include "storage/wal/wal_record.h"

namespace rag3db {
namespace main {
class ClientContext;
} // namespace main

namespace storage {

// Test-only: the points of a read-only open at which a test can make a writer
// act. See WALReplayer::setReadOnlyOpenHookForTesting.
enum class ReadOnlyOpenPhase : uint8_t {
    // The journal has been scanned; the data file has not been read yet.
    JOURNAL_SCANNED = 0,
    // The data file has been read; the journal has not been replayed yet.
    DATA_FILE_READ = 1,
};

class WALReplayer {
public:
    explicit WALReplayer(main::ClientContext& clientContext);

    void replay(bool throwOnWalReplayFailure, bool enableChecksums) const;

    // The refusal of a read-only open that a writer's checkpoint crossed.
    static constexpr const char* CHECKPOINT_CROSSED_READ_ONLY_OPEN =
        "The database was checkpointed by another process while this read-only open was reading "
        "it";

    // Test-only. Runs `hook` at each ReadOnlyOpenPhase of a read-only open, so that a test can
    // make a writer checkpoint or commit at a precise point instead of racing for it. Never
    // called on a read-write open, and a no-op unless a test sets it. Pass nullptr to clear.
    using read_only_open_hook_t = std::function<void(ReadOnlyOpenPhase)>;
    static void setReadOnlyOpenHookForTesting(read_only_open_hook_t hook);

private:
    struct WALReplayInfo {
        uint64_t offsetDeserialized = 0;
        bool isLastRecordCheckpoint = false;
        // The dry run ran out of file inside a record: the end of the journal
        // was torn by a stop during an append. Everything after
        // offsetDeserialized belongs to a transaction that never committed.
        bool tornEnd = false;
    };

    void setAsideCutBytes(common::FileInfo& fileInfo, uint64_t offsetDeserialized,
        bool tornEnd) const;

    void replayWALRecord(WALRecord& walRecord) const;
    void replayCreateCatalogEntryRecord(WALRecord& walRecord) const;
    void replayDropCatalogEntryRecord(const WALRecord& walRecord) const;
    void replayAlterTableEntryRecord(const WALRecord& walRecord) const;
    void replayTableInsertionRecord(const WALRecord& walRecord) const;
    void replayNodeDeletionRecord(const WALRecord& walRecord) const;
    void replayNodeUpdateRecord(const WALRecord& walRecord) const;
    void replayRelDeletionRecord(const WALRecord& walRecord) const;
    void replayRelDetachDeletionRecord(const WALRecord& walRecord) const;
    void replayRelUpdateRecord(const WALRecord& walRecord) const;
    void replayCopyTableRecord(const WALRecord& walRecord) const;
    void replayUpdateSequenceRecord(const WALRecord& walRecord) const;

    void replayNodeTableInsertRecord(const WALRecord& walRecord) const;
    void replayRelTableInsertRecord(const WALRecord& walRecord) const;

    void replayLoadExtensionRecord(const WALRecord& walRecord) const;

    // This function is used to deserialize the WAL records without actually applying them to the
    // storage.
    WALReplayInfo dryReplay(common::FileInfo& fileInfo, bool throwOnWalReplayFailure,
        bool enableChecksums) const;

    void runReadOnlyOpenHook(ReadOnlyOpenPhase phase) const;

    void removeWALAndShadowFiles() const;
    void removeFileIfExists(const std::string& path) const;

    std::unique_ptr<common::FileInfo> openWALFile() const;
    void syncWALFile(const common::FileInfo& fileInfo) const;
    void truncateWALFile(common::FileInfo& fileInfo, uint64_t size) const;

private:
    main::ClientContext& clientContext;
    std::string walPath;
    std::string shadowFilePath;
};

} // namespace storage
} // namespace rag3db
