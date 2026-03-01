#include "index/rtree_index.h"

#include "catalog/spatial_index_catalog_entry.h"
#include "common/serializer/buffer_writer.h"
#include "common/serializer/deserializer.h"
#include "common/serializer/serializer.h"
#include "main/client_context.h"
#include "main/database.h"
#include "storage/storage_manager.h"

#include <filesystem>
#include <fstream>

namespace rag3db {
namespace geo_extension {

// ─── SpatialStorageInfo ────────────────────────────────────────────────────────

std::shared_ptr<common::BufferWriter> SpatialStorageInfo::serialize() const {
    auto bufferWriter = std::make_shared<common::BufferWriter>();
    auto serializer = common::Serializer(bufferWriter);
    serializer.write<std::string>(indexFilePath);
    serializer.write<common::offset_t>(numIndexedNodes);
    return bufferWriter;
}

std::unique_ptr<storage::IndexStorageInfo> SpatialStorageInfo::deserialize(
    std::unique_ptr<common::BufferReader> reader) {
    common::Deserializer deSer{std::move(reader)};
    std::string path;
    common::offset_t numNodes;
    deSer.deserializeValue<std::string>(path);
    deSer.deserializeValue<common::offset_t>(numNodes);
    return std::make_unique<SpatialStorageInfo>(std::move(path), numNodes);
}

// ─── Insert/Delete/Update States ───────────────────────────────────────────────

struct RTreeInsertState final : storage::Index::InsertState {
    ~RTreeInsertState() override = default;
};

struct RTreeDeleteState final : storage::Index::DeleteState {
    ~RTreeDeleteState() override = default;
};

struct RTreeUpdateState final : storage::Index::UpdateState {
    ~RTreeUpdateState() override = default;
};

// ─── RTreeIndex ────────────────────────────────────────────────────────────────

RTreeIndex::RTreeIndex(storage::IndexInfo indexInfo,
    std::unique_ptr<storage::IndexStorageInfo> si, std::unique_ptr<RTree> rtree)
    : Index(std::move(indexInfo), std::move(si)), rtree_(std::move(rtree)) {}

std::unique_ptr<storage::Index> RTreeIndex::load(main::ClientContext* /*context*/,
    storage::StorageManager* /*storageManager*/, storage::IndexInfo indexInfo,
    std::span<uint8_t> storageInfoBuffer) {
    auto reader = std::make_unique<common::BufferReader>(storageInfoBuffer.data(),
        storageInfoBuffer.size());
    auto si = SpatialStorageInfo::deserialize(std::move(reader));
    auto& spatialInfo = si->cast<SpatialStorageInfo>();
    auto rtree = loadFromFile(spatialInfo.indexFilePath);
    return std::make_unique<RTreeIndex>(std::move(indexInfo), std::move(si), std::move(rtree));
}

// ─── CRUD Hooks ────────────────────────────────────────────────────────────────

std::unique_ptr<storage::Index::InsertState> RTreeIndex::initInsertState(main::ClientContext*,
    storage::visible_func) {
    return std::make_unique<RTreeInsertState>();
}

void RTreeIndex::insert(transaction::Transaction*, const common::ValueVector& nodeIDVector,
    const std::vector<common::ValueVector*>& indexVectors, InsertState&) {
    std::lock_guard<std::mutex> lock(mutex_);
    uint32_t dims = rtree_->dimensions();
    for (auto i = 0u; i < nodeIDVector.state->getSelSize(); i++) {
        auto pos = nodeIDVector.state->getSelVector()[i];
        auto nodeOffset = nodeIDVector.readNodeOffset(pos);

        // Check if any coordinate column is null.
        bool isNull = false;
        for (uint32_t d = 0; d < dims; d++) {
            if (indexVectors[d]->isNull(pos)) {
                isNull = true;
                break;
            }
        }
        if (isNull) continue;

        std::vector<double> coords(dims);
        for (uint32_t d = 0; d < dims; d++) {
            coords[d] = indexVectors[d]->getValue<double>(pos);
        }
        rtree_->insert(nodeOffset, coords);
    }
}

std::unique_ptr<storage::Index::DeleteState> RTreeIndex::initDeleteState(
    const transaction::Transaction*, storage::MemoryManager*, storage::visible_func) {
    return std::make_unique<RTreeDeleteState>();
}

void RTreeIndex::delete_(transaction::Transaction*,
    const common::ValueVector& nodeIDVector, DeleteState&) {
    std::lock_guard<std::mutex> lock(mutex_);
    for (auto i = 0u; i < nodeIDVector.state->getSelSize(); i++) {
        auto pos = nodeIDVector.state->getSelVector()[i];
        auto nodeOffset = nodeIDVector.readNodeOffset(pos);
        rtree_->remove(nodeOffset);
    }
}

std::unique_ptr<storage::Index::UpdateState> RTreeIndex::initUpdateState(main::ClientContext*,
    common::column_id_t, storage::visible_func) {
    return std::make_unique<RTreeUpdateState>();
}

void RTreeIndex::update(transaction::Transaction*,
    const common::ValueVector& nodeIDVector, common::ValueVector& propertyVector,
    UpdateState&) {
    std::lock_guard<std::mutex> lock(mutex_);
    for (auto i = 0u; i < nodeIDVector.state->getSelSize(); i++) {
        auto pos = nodeIDVector.state->getSelVector()[i];
        auto nodeOffset = nodeIDVector.readNodeOffset(pos);
        auto valuePos = propertyVector.state->getSelVector()[i];

        if (propertyVector.isNull(valuePos)) {
            rtree_->remove(nodeOffset);
        } else {
            // For update, we receive only one column at a time.
            // Since spatial index spans multiple columns, we do remove + reinsert.
            // The full coords are not available here, so just remove for now.
            // Full update support requires commitInsert pattern.
            rtree_->remove(nodeOffset);
        }
    }
}

// ─── Persistence ───────────────────────────────────────────────────────────────

void RTreeIndex::checkpoint(main::ClientContext*, storage::PageAllocator&) {
    saveToFile();
}

void RTreeIndex::saveToFile() const {
    auto& spatialInfo = storageInfo->cast<SpatialStorageInfo>();
    auto dir = std::filesystem::path(spatialInfo.indexFilePath).parent_path();
    std::filesystem::create_directories(dir);

    std::vector<uint8_t> buffer;
    rtree_->serialize(buffer);

    std::ofstream ofs(spatialInfo.indexFilePath, std::ios::binary);
    ofs.write(reinterpret_cast<const char*>(buffer.data()),
        static_cast<std::streamsize>(buffer.size()));
}

std::unique_ptr<RTree> RTreeIndex::loadFromFile(const std::string& path) {
    std::ifstream ifs(path, std::ios::binary | std::ios::ate);
    if (!ifs.is_open()) {
        return std::make_unique<RTree>(2); // Fallback empty 2D tree.
    }
    auto size = static_cast<size_t>(ifs.tellg());
    ifs.seekg(0, std::ios::beg);
    std::vector<uint8_t> buffer(size);
    ifs.read(reinterpret_cast<char*>(buffer.data()), static_cast<std::streamsize>(size));
    return RTree::deserialize(buffer.data(), size);
}

// ─── Queries ───────────────────────────────────────────────────────────────────

std::vector<std::pair<uint64_t, double>> RTreeIndex::knn(const std::vector<double>& query,
    uint32_t k) const {
    std::lock_guard<std::mutex> lock(mutex_);
    return rtree_->knn(query, k);
}

std::vector<uint64_t> RTreeIndex::searchBBox(const BoundingBox& bbox) const {
    std::lock_guard<std::mutex> lock(mutex_);
    return rtree_->searchBBox(bbox);
}

std::vector<std::pair<uint64_t, double>> RTreeIndex::searchRadius(
    const std::vector<double>& center, double radius) const {
    std::lock_guard<std::mutex> lock(mutex_);
    return rtree_->searchRadius(center, radius);
}

std::vector<std::pair<uint64_t, double>> RTreeIndex::searchOBB(const std::vector<double>& center,
    const std::vector<double>& halfExtents, const math::Quat& quat) const {
    std::lock_guard<std::mutex> lock(mutex_);
    return rtree_->searchOBB(center, halfExtents, quat);
}

std::vector<std::pair<uint64_t, double>> RTreeIndex::searchFrustum(const double* planes,
    uint32_t numPlanes, const std::vector<double>& refPoint) const {
    std::lock_guard<std::mutex> lock(mutex_);
    return rtree_->searchFrustum(planes, numPlanes, refPoint);
}

} // namespace geo_extension
} // namespace rag3db
