#pragma once

#include <functional>
#include <string>

#include "storage/readers_lock.h"
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

// Test-only : les points nommés d'une reprise en écriture où un test d'arrêt brutal peut tuer
// le processus. Voir WALReplayer::setRecoveryHookForTesting. Les prochains témoins d'arrêt
// s'en servent plutôt que d'en poser d'autres.
enum class RecoveryPoint : uint8_t {
    // Les pages du fichier fantôme sont recopiées dans le fichier de données et synchronisées ;
    // ni le journal ni le fichier fantôme ne sont encore supprimés.
    SHADOW_PAGES_REPLAYED = 0,
    // Le journal est supprimé ; le fichier fantôme ne l'est pas encore.
    JOURNAL_REMOVED = 1,
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

    // Test-only. Runs `hook` at each RecoveryPoint of a read-write recovery, so that a test can
    // kill the process at that instant. A no-op unless a test sets it. Pass nullptr to clear.
    using recovery_hook_t = std::function<void(RecoveryPoint)>;
    static void setRecoveryHookForTesting(recovery_hook_t hook);

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
    void runRecoveryHook(RecoveryPoint point) const;

    // What a read-only open saw of the two files before reading them, so that it can tell
    // afterwards whether a writer's checkpoint crossed it. A reader takes no lock: the journal is
    // scanned, then the data file is read, then the journal is replayed, at three different
    // times. Between two checkpoints a writer only appends to the journal and writes pages that
    // the last checkpoint does not reference, so those three reads agree. A checkpoint rewrites
    // the header page and referenced pages in place and then truncates the journal: a reader that
    // straddles it can load a data file that already contains the transactions it then replays.
    struct ReadOnlyOpenIdentity {
        std::string headerPage; // page 0 of the data file, raw
        bool journalExists = false;
        uint64_t journalSize = 0;
        uint64_t journalChecksum = 0; // of the first journalSize bytes
    };
    ReadOnlyOpenIdentity captureReadOnlyOpenIdentity() const;
    // Throws CHECKPOINT_CROSSED_READ_ONLY_OPEN if the files no longer match `before`: the header
    // page changed, the journal bytes seen at the start are gone, or the journal now ends with a
    // CHECKPOINT record. A journal that merely grew by new commits is not a change.
    //
    // What this does NOT cover, on purpose: it needs an epoch written in the file, which is
    // step 5 ("marche 5") of docs/2-octobre-2026-00h17/01-ecritures-paralleles-vela-et-le-chemin.md:
    //  - a reader that STAYS open while a checkpoint passes: it reads pages on demand with the
    //    metadata it loaded at open, and nothing tells it they were rewritten;
    //  - two complete checkpoints within a single open that leave a byte-identical header page
    //    and an empty journal on both sides.
    void throwIfCheckpointCrossedReadOnlyOpen(const ReadOnlyOpenIdentity& before,
        bool enableChecksums) const;

    void removeWALAndShadowFiles() const;
    void removeFileIfExists(const std::string& path) const;

    std::unique_ptr<common::FileInfo> openWALFile() const;
    void syncWALFile(const common::FileInfo& fileInfo) const;
    void truncateWALFile(common::FileInfo& fileInfo, uint64_t size) const;
    // La reprise d'un écrivain qui change le fichier pour un lecteur (le rejeu des pages
    // fantômes, la troncature du journal) attend d'abord les ouvertures en lecture seule.
    ReadersLock holdReadersLock() const;

private:
    main::ClientContext& clientContext;
    std::string walPath;
    std::string shadowFilePath;
};

} // namespace storage
} // namespace rag3db
