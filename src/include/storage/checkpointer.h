#pragma once

#include "storage/database_header.h"
#include "storage/page_range.h"
#include "storage/readers_lock.h"

namespace rag3db {
namespace transaction {
class Transaction;
}
namespace catalog {
class Catalog;
}
namespace common {
class VirtualFileSystem;
} // namespace common
namespace testing {
struct FSMLeakChecker;
}
namespace main {
class AttachedRag3dbDatabase;
} // namespace main

namespace storage {
class StorageManager;

class Checkpointer {
    friend class main::AttachedRag3dbDatabase;
    friend struct testing::FSMLeakChecker;

public:
    explicit Checkpointer(main::ClientContext& clientContext);
    virtual ~Checkpointer();

    void writeCheckpoint();
    void rollback();

    void readCheckpoint();

    static bool canAutoCheckpoint(const main::ClientContext& clientContext,
        const transaction::Transaction& transaction);

protected:
    virtual bool checkpointStorage();
    virtual void serializeCatalogAndMetadata(DatabaseHeader& databaseHeader,
        bool hasStorageChanges);
    virtual void writeDatabaseHeader(const DatabaseHeader& header);
    virtual void logCheckpointAndApplyShadowPages();
    // Attend, borné, les ouvertures en lecture seule d'autres processus, puis tient le verrou des
    // lecteurs jusqu'à la fin de writeCheckpoint : voir ReadersLock. À appeler juste avant la
    // marque CHECKPOINT, premier changement qu'un lecteur verrait.
    void holdReadersLock();

private:
    static void readCheckpoint(main::ClientContext* context, catalog::Catalog* catalog,
        StorageManager* storageManager);
    static void returnOwnerlessPages(const DatabaseHeader& header, StorageManager* storageManager);

    PageRange serializeCatalog(const catalog::Catalog& catalog, StorageManager& storageManager);
    PageRange serializeMetadata(const catalog::Catalog& catalog, StorageManager& storageManager);

protected:
    main::ClientContext& clientContext;
    bool isInMemory;
    ReadersLock readersLock;
};

} // namespace storage
} // namespace rag3db
