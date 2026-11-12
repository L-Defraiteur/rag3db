#pragma once

#include <atomic>

#include "storage/wal/wal_record.h"

namespace rag3db {
namespace common {
class BufferedFileWriter;
class VirtualFileSystem;
} // namespace common

namespace storage {
class LocalWAL;
class WAL {
public:
    WAL(const std::string& dbPath, bool readOnly, bool enableChecksums,
        common::VirtualFileSystem* vfs);
    ~WAL();

    // Écrit et synchronise les enregistrements d'une validation. Si l'écriture ou la
    // synchronisation lève, le journal revient à sa taille d'avant (ses enregistrements ne seront
    // jamais rejoués) et l'exception remonte : la validation est annulée.
    void logCommittedWAL(LocalWAL& localWAL, main::ClientContext* context);
    static constexpr const char* JOURNAL_BROKEN =
        "A write to the journal failed and the journal could not be restored: nothing more can be "
        "committed until the database is reopened, and the outcome of the last commit is what the "
        "journal holds at reopen";
    void logAndFlushCheckpoint(main::ClientContext* context);

    // Clear any buffer in the WAL writer. Also truncate the WAL file to 0 bytes.
    void clear();
    // Reset the WAL writer to nullptr, and remove the WAL file if it exists.
    void reset();

    uint64_t getFileSize();

    // Les transactions qu'un COPY journalisé a repliées sur le point de reprise forcé, depuis
    // l'ouverture de la base (Transaction::fallBackToForcedCheckpoint).
    void noteCopyJournalFallback() { numCopyJournalFallbacks++; }
    uint64_t getNumCopyJournalFallbacks() const { return numCopyJournalFallbacks; }

    static WAL* Get(const main::ClientContext& context);

private:
    void initWriter(main::ClientContext* context);
    void addNewWALRecordNoLock(const WALRecord& walRecord);
    void flushAndSyncNoLock();
    void writeHeader(main::ClientContext& context);

private:
    std::mutex mtx;
    std::atomic<uint64_t> numCopyJournalFallbacks{0};
    std::string walPath;
    bool inMemory;
    [[maybe_unused]] bool readOnly;
    common::VirtualFileSystem* vfs;
    std::unique_ptr<common::FileInfo> fileInfo;

    // Since most writes to the shared WAL will be flushing local WAL (which has its own checksums),
    // these writes can go through the normal writer. We do still need a checksum writer though for
    // writing COMMIT/CHECKPOINT records
    std::unique_ptr<common::Serializer> serializer;
    // L'écrivain du fichier sous serializer (posé par initWriter) : son tampon se jette après une
    // écriture ratée, pour que sa destruction ne réécrive rien.
    common::BufferedFileWriter* fileWriter = nullptr;
    // Une écriture du journal a échoué et le journal n'a pas pu revenir à sa taille d'avant : plus
    // rien ne s'y écrit jusqu'à la réouverture, qui tranche la dernière validation par ce que le
    // journal porte alors.
    bool brokenUntilReopen = false;
    bool enableChecksums;
};

} // namespace storage
} // namespace rag3db
