#pragma once

#include <mutex>

#include "catalog/catalog.h"
#include "shadow_file.h"
#include "storage/index/index.h"
#include "storage/wal/wal.h"

namespace rag3db {
namespace main {
class Database;
} // namespace main

namespace catalog {
class CatalogEntry;
class NodeTableCatalogEntry;
class RelGroupCatalogEntry;
struct RelTableCatalogInfo;
} // namespace catalog

namespace storage {
class Table;
class NodeTable;
class RelTable;
class DiskArrayCollection;
struct DatabaseHeader;

class RAG3DB_API StorageManager {
public:
    StorageManager(const std::string& databasePath, bool readOnly, bool enableChecksums,
        MemoryManager& memoryManager, bool enableCompression, common::VirtualFileSystem* vfs);
    ~StorageManager();

    Table* getTable(common::table_id_t tableID);
    // Les tables de relations dont une extrémité est cette table de nœuds.
    std::vector<Table*> getRelTablesOf(common::table_id_t nodeTableID);
    std::vector<Table*> getNodeTables();

    static void recover(main::ClientContext& clientContext, bool throwOnWalReplayFailure,
        bool enableChecksums);

    void createTable(catalog::TableCatalogEntry* entry);
    void addRelTable(catalog::RelGroupCatalogEntry* entry,
        const catalog::RelTableCatalogInfo& info);

    bool checkpoint(main::ClientContext* context, PageAllocator& pageAllocator);
    void finalizeCheckpoint();
    void rollbackCheckpoint(const catalog::Catalog& catalog);

    // Le point de reprise en cours a changé ce que le processus tient en mémoire (le premier
    // échange de groupes d'une table, le point de reprise de ses index, une table de relations,
    // une table retirée, puis tout ce qui suit le stockage). Avant, une exception ne laisse rien
    // à défaire que Checkpointer::rollback ne défasse : le point de reprise est refusé, nommé,
    // et la base reste ouverte (ticket du ROLLBACK sous un tampon plein, défaut 3). Après, la
    // base est à rouvrir (TransactionManager::checkpointFailed).
    // Le point de reprise en cours (V2-2, posé par le Checkpointer) : son horodatage T et le
    // plus vieux instantané ouvert F. F < T : les groupes gardent leur ancienne génération.
    void setCheckpointTimestamps(common::transaction_t checkpointTS_,
        common::transaction_t oldestReaderTS_) {
        checkpointTS = checkpointTS_;
        oldestReaderTS = oldestReaderTS_;
    }
    common::transaction_t getCheckpointTS() const { return checkpointTS; }
    common::transaction_t getOldestReaderTS() const { return oldestReaderTS; }
    void markCheckpointChangedMemory() { checkpointChangedMemory = true; }
    bool* getCheckpointChangedMemoryFlag() { return &checkpointChangedMemory; }
    bool hasCheckpointChangedMemory() const { return checkpointChangedMemory; }

    WAL& getWAL() const;
    ShadowFile& getShadowFile() const;
    FileHandle* getDataFH() const { return dataFH; }
    std::string getDatabasePath() const { return databasePath; }
    bool isReadOnly() const { return readOnly; }
    bool compressionEnabled() const { return enableCompression; }
    bool isInMemory() const { return inMemory; }

    void registerIndexType(IndexType indexType) {
        registeredIndexTypes.push_back(std::move(indexType));
    }
    std::optional<std::reference_wrapper<const IndexType>> getIndexType(
        const std::string& typeName) const;

    void serialize(const catalog::Catalog& catalog, common::Serializer& ser);
    // We need to pass in the catalog and storageManager explicitly as they can be from
    // attachedDatabase.
    void deserialize(main::ClientContext* context, const catalog::Catalog* catalog,
        common::Deserializer& deSer);

    void initDataFileHandle(common::VirtualFileSystem* vfs, main::ClientContext* context);

    // If the database header hasn't been created yet, calling these methods will create + return
    // the header
    common::ku_uuid_t getOrInitDatabaseID(const main::ClientContext& clientContext);
    const storage::DatabaseHeader* getOrInitDatabaseHeader(
        const main::ClientContext& clientContext);

    void setDatabaseHeader(std::unique_ptr<storage::DatabaseHeader> header);

    static StorageManager* Get(const main::ClientContext& context);

private:
    void createNodeTable(catalog::NodeTableCatalogEntry* entry);

    void createRelTableGroup(catalog::RelGroupCatalogEntry* entry);

    void reclaimDroppedTables(const catalog::Catalog& catalog);

private:
    std::mutex mtx;
    bool checkpointChangedMemory = false;
    common::transaction_t checkpointTS = 0;
    common::transaction_t oldestReaderTS = 0;
    std::string databasePath;
    std::unique_ptr<storage::DatabaseHeader> databaseHeader;
    bool readOnly;
    FileHandle* dataFH;
    std::unordered_map<common::table_id_t, std::unique_ptr<Table>> tables;
    MemoryManager& memoryManager;
    std::unique_ptr<WAL> wal;
    std::unique_ptr<ShadowFile> shadowFile;
    bool enableCompression;
    bool inMemory;
    std::vector<IndexType> registeredIndexTypes;
};

} // namespace storage
} // namespace rag3db
