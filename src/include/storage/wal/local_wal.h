#pragma once

#include <map>
#include <vector>

#include "storage/wal/wal_record.h"

namespace rag3db {
namespace binder {
struct BoundAlterInfo;
} // namespace binder
namespace common {
class InMemFileWriter;
class ValueVector;
} // namespace common
namespace catalog {
class CatalogEntry;
} // namespace catalog

namespace storage {
class WAL;
class LocalWAL {
    friend class WAL;

public:
    explicit LocalWAL(MemoryManager& mm, bool enableChecksums);

    void logCreateCatalogEntryRecord(catalog::CatalogEntry* catalogEntry, bool isInternal);
    void logDropCatalogEntryRecord(uint64_t tableID, catalog::CatalogEntryType type);
    void logAlterCatalogEntryRecord(const binder::BoundAlterInfo* alterInfo);
    void logUpdateSequenceRecord(common::sequence_id_t sequenceID, uint64_t kCount);

    void logTableInsertion(common::table_id_t tableID, common::TableType tableType,
        common::row_idx_t numRows, const std::vector<common::ValueVector*>& vectors);
    void logNodeDeletion(common::table_id_t tableID, common::offset_t nodeOffset,
        common::ValueVector* pkVector);
    void logNodeUpdate(common::table_id_t tableID, common::column_id_t columnID,
        common::offset_t nodeOffset, common::ValueVector* propertyVector);
    void logRelDelete(common::table_id_t tableID, common::ValueVector* srcNodeVector,
        common::ValueVector* dstNodeVector, common::ValueVector* relIDVector);
    void logRelDetachDelete(common::table_id_t tableID, common::RelDataDirection direction,
        common::ValueVector* srcNodeVector);
    void logRelUpdate(common::table_id_t tableID, common::column_id_t columnID,
        common::ValueVector* srcNodeVector, common::ValueVector* dstNodeVector,
        common::ValueVector* relIDVector, common::ValueVector* propertyVector);

    void logLoadExtension(std::string path);

    void logBeginTransaction();
    void logCommit();

    void clear();
    uint64_t getSize();
    // Vide le journal et n'y écrit plus rien : sa transaction s'est repliée sur le point de
    // reprise forcé, qui la rend durable seul.
    void discard();

    // La sonde RAG3DB_PROFILE_JOURNAL : ce que pèse ce journal, par type d'enregistrement et par
    // table. Vide si la variable n'est pas posée.
    struct Weight {
        uint8_t recordType;
        common::table_id_t tableID;
        uint64_t numBytes;
        uint64_t numRecords;
        uint64_t numRows;
    };
    std::vector<Weight> getWeights();
    static bool profiled();

private:
    void addNewWALRecord(const WALRecord& walRecord,
        common::table_id_t tableID = common::INVALID_TABLE_ID, uint64_t numRows = 0);

private:
    std::mutex mtx;
    std::shared_ptr<common::InMemFileWriter> inMemWriter;
    common::Serializer serializer;
    std::map<std::pair<uint8_t, common::table_id_t>, Weight> weights;
    bool discarded = false;
};

} // namespace storage
} // namespace rag3db
