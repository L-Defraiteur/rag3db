#pragma once

#include "common/serializer/buffer_reader.h"
#include "common/serializer/buffer_writer.h"
#include "index/rtree.h"
#include "storage/index/index.h"

#include <mutex>

namespace rag3db {
namespace storage {
class NodeTable;
}

namespace geo_extension {

struct SpatialStorageInfo final : storage::IndexStorageInfo {
    std::string indexFilePath;
    common::offset_t numIndexedNodes;

    SpatialStorageInfo(std::string indexFilePath, common::offset_t numIndexedNodes)
        : indexFilePath{std::move(indexFilePath)}, numIndexedNodes{numIndexedNodes} {}

    std::shared_ptr<common::BufferWriter> serialize() const override;
    static std::unique_ptr<storage::IndexStorageInfo> deserialize(
        std::unique_ptr<common::BufferReader> reader);
};

class RTreeIndex final : public storage::Index {
public:
    RTreeIndex(storage::IndexInfo indexInfo, std::unique_ptr<storage::IndexStorageInfo> storageInfo,
        std::unique_ptr<RTree> rtree);

    static std::unique_ptr<Index> load(main::ClientContext* context,
        storage::StorageManager* storageManager, storage::IndexInfo indexInfo,
        std::span<uint8_t> storageInfoBuffer);

    static storage::IndexType getIndexType() {
        static const storage::IndexType RTREE_INDEX_TYPE{"SPATIAL",
            storage::IndexConstraintType::SECONDARY_NON_UNIQUE,
            storage::IndexDefinitionType::EXTENSION, load};
        return RTREE_INDEX_TYPE;
    }

    // CRUD hooks.
    std::unique_ptr<InsertState> initInsertState(main::ClientContext* context,
        storage::visible_func isVisible) override;
    void insert(transaction::Transaction* transaction, const common::ValueVector& nodeIDVector,
        const std::vector<common::ValueVector*>& indexVectors, InsertState& insertState) override;

    std::unique_ptr<DeleteState> initDeleteState(const transaction::Transaction* transaction,
        storage::MemoryManager* mm, storage::visible_func isVisible) override;
    void delete_(transaction::Transaction* transaction,
        const common::ValueVector& nodeIDVector, DeleteState& deleteState) override;

    std::unique_ptr<UpdateState> initUpdateState(main::ClientContext* context,
        common::column_id_t columnID, storage::visible_func isVisible) override;
    void update(transaction::Transaction* transaction,
        const common::ValueVector& nodeIDVector, common::ValueVector& propertyVector,
        UpdateState& updateState) override;

    void checkpoint(main::ClientContext* context, storage::PageAllocator& pageAllocator) override;

    // Queries.
    std::vector<std::pair<uint64_t, double>> knn(const std::vector<double>& query,
        uint32_t k) const;
    std::vector<uint64_t> searchBBox(const BoundingBox& bbox) const;
    std::vector<std::pair<uint64_t, double>> searchRadius(const std::vector<double>& center,
        double radius) const;
    std::vector<std::pair<uint64_t, double>> searchOBB(const std::vector<double>& center,
        const std::vector<double>& halfExtents, const math::Quat& quat) const;
    std::vector<std::pair<uint64_t, double>> searchFrustum(const double* planes,
        uint32_t numPlanes, const std::vector<double>& refPoint) const;

    const RTree& getRTree() const { return *rtree_; }
    void saveToFile() const;

private:
    std::unique_ptr<RTree> rtree_;
    mutable std::mutex mutex_; // Protects rtree_ for concurrent access.

    static std::unique_ptr<RTree> loadFromFile(const std::string& path);
};

} // namespace geo_extension
} // namespace rag3db
