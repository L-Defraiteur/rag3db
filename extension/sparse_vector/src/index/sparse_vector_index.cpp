#include "index/sparse_vector_index.h"

#include "catalog/sparse_vector_catalog_entry.h"
#include "common/serializer/buffer_reader.h"
#include "common/serializer/buffer_writer.h"
#include "common/serializer/deserializer.h"
#include "common/serializer/serializer.h"
#include "common/types/value/nested.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

namespace rag3db {
namespace sparse_vector_extension {

using namespace storage;
using namespace common;

// ── SparseVectorStorageInfo ─────────────────────────────────────────────────

std::shared_ptr<BufferWriter> SparseVectorStorageInfo::serialize() const {
    auto writer = std::make_shared<BufferWriter>();
    auto ser = Serializer(writer);
    ser.write<offset_t>(numCheckpointedNodes);
    return writer;
}

std::unique_ptr<IndexStorageInfo> SparseVectorStorageInfo::deserialize(
    std::unique_ptr<BufferReader> reader) {
    offset_t numCheckpointedNodes = 0;
    Deserializer deSer{std::move(reader)};
    deSer.deserializeValue<offset_t>(numCheckpointedNodes);
    return std::make_unique<SparseVectorStorageInfo>(numCheckpointedNodes);
}

// ── SparseVectorIndex ───────────────────────────────────────────────────────

SparseVectorIndex::SparseVectorIndex(IndexInfo indexInfo,
    std::unique_ptr<IndexStorageInfo> storageInfo, rust::Box<::SparseHandle> handle)
    : Index{std::move(indexInfo), std::move(storageInfo)}, handle_{std::move(handle)} {}

IndexType SparseVectorIndex::getIndexType() {
    static const IndexType SPARSE_VECTOR_INDEX_TYPE{"SPARSE_VECTOR",
        IndexConstraintType::SECONDARY_NON_UNIQUE, IndexDefinitionType::EXTENSION, load};
    return SPARSE_VECTOR_INDEX_TYPE;
}

std::unique_ptr<Index> SparseVectorIndex::load(main::ClientContext* context, StorageManager*,
    IndexInfo indexInfo, std::span<uint8_t> storageInfoBuffer) {
    auto reader =
        std::make_unique<BufferReader>(storageInfoBuffer.data(), storageInfoBuffer.size());
    auto si = SparseVectorStorageInfo::deserialize(std::move(reader));
    auto catalog = catalog::Catalog::Get(*context);
    auto indexEntry = catalog->getIndex(transaction::Transaction::Get(*context), indexInfo.tableID,
        indexInfo.name);
    auto& auxInfo = indexEntry->getAuxInfo().cast<SparseVectorIndexAuxInfo>();
    auto handle = open_sparse_index(auxInfo.indexPath);
    return std::make_unique<SparseVectorIndex>(
        std::move(indexInfo), std::move(si), std::move(handle));
}

// ── Insert / Delete states ──────────────────────────────────────────────────

struct SparseVectorInsertState final : Index::InsertState {};
struct SparseVectorDeleteState final : Index::DeleteState {};

struct SparseVectorUpdateState final : Index::UpdateState {
    size_t updatedColumnIdx;
    std::unique_ptr<Index::DeleteState> deleteState;
    StorageManager* storageManager;
    MemoryManager* memoryManager;

    SparseVectorUpdateState(size_t updatedColumnIdx,
        std::unique_ptr<Index::DeleteState> deleteState,
        StorageManager* sm, MemoryManager* mm)
        : updatedColumnIdx{updatedColumnIdx}, deleteState{std::move(deleteState)},
          storageManager{sm}, memoryManager{mm} {}
};

std::unique_ptr<Index::InsertState> SparseVectorIndex::initInsertState(main::ClientContext*,
    visible_func) {
    return std::make_unique<SparseVectorInsertState>();
}

std::unique_ptr<Index::DeleteState> SparseVectorIndex::initDeleteState(
    const transaction::Transaction*, MemoryManager*, visible_func) {
    return std::make_unique<SparseVectorDeleteState>();
}

std::unique_ptr<Index::UpdateState> SparseVectorIndex::initUpdateState(
    main::ClientContext* context, column_id_t columnID, visible_func isVisible) {
    size_t updatedIdx = 0;
    for (size_t i = 0; i < indexInfo.columnIDs.size(); i++) {
        if (indexInfo.columnIDs[i] == columnID) {
            updatedIdx = i;
            break;
        }
    }
    auto delState = initDeleteState(transaction::Transaction::Get(*context),
        MemoryManager::Get(*context), isVisible);
    return std::make_unique<SparseVectorUpdateState>(updatedIdx, std::move(delState),
        StorageManager::Get(*context), MemoryManager::Get(*context));
}

// ── Helper: extract sparse vector from LIST ValueVectors ────────────────────

static void extractSparseVector(const ValueVector& indicesVec, const ValueVector& weightsVec,
    idx_t pos, std::vector<uint32_t>& outIndices, std::vector<float>& outWeights) {
    outIndices.clear();
    outWeights.clear();

    auto indicesList = indicesVec.getValue<list_entry_t>(pos);
    auto* indicesData = ListVector::getDataVector(&indicesVec);
    for (auto j = indicesList.offset; j < indicesList.offset + indicesList.size; j++) {
        outIndices.push_back(static_cast<uint32_t>(indicesData->getValue<int64_t>(j)));
    }

    auto weightsList = weightsVec.getValue<list_entry_t>(pos);
    auto* weightsData = ListVector::getDataVector(&weightsVec);
    for (auto j = weightsList.offset; j < weightsList.offset + weightsList.size; j++) {
        outWeights.push_back(static_cast<float>(weightsData->getValue<double>(j)));
    }
}

// ── insert ──────────────────────────────────────────────────────────────────

void SparseVectorIndex::insert(transaction::Transaction*, const ValueVector& nodeIDVector,
    const std::vector<ValueVector*>& propertyVectors, InsertState&) {
    // propertyVectors[0] = indices (LIST[INT64]), propertyVectors[1] = weights (LIST[DOUBLE])
    for (auto i = 0u; i < nodeIDVector.state->getSelSize(); i++) {
        auto pos = nodeIDVector.state->getSelVector()[i];
        auto nodeOffset = nodeIDVector.getValue<nodeID_t>(pos).offset;

        if (propertyVectors[0]->isNull(pos) || propertyVectors[1]->isNull(pos)) {
            continue;
        }

        std::vector<uint32_t> indices;
        std::vector<float> weights;
        extractSparseVector(*propertyVectors[0], *propertyVectors[1], pos, indices, weights);

        add_sparse_document(*handle_, static_cast<uint64_t>(nodeOffset),
            rust::Slice<const uint32_t>(indices.data(), indices.size()),
            rust::Slice<const float>(weights.data(), weights.size()));
    }
    dirty_ = true;
}

// ── delete ──────────────────────────────────────────────────────────────────

void SparseVectorIndex::delete_(transaction::Transaction*, const ValueVector& nodeIDVector,
    DeleteState&) {
    for (auto i = 0u; i < nodeIDVector.state->getSelSize(); i++) {
        auto pos = nodeIDVector.state->getSelVector()[i];
        auto nodeOffset = nodeIDVector.getValue<nodeID_t>(pos).offset;
        delete_sparse_document(*handle_, static_cast<uint64_t>(nodeOffset));
    }
    dirty_ = true;
}

// ── update ──────────────────────────────────────────────────────────────────

void SparseVectorIndex::update(transaction::Transaction* transaction,
    const ValueVector& nodeIDVector, ValueVector& propertyVector,
    UpdateState& updateState) {
    auto& state = updateState.cast<SparseVectorUpdateState>();
    // 1. Delete old document.
    delete_(transaction, nodeIDVector, *state.deleteState);

    // 2. Re-read all indexed columns from the node table.
    auto pos = nodeIDVector.state->getSelVector()[0];
    auto nodeOffset = nodeIDVector.getValue<nodeID_t>(pos).offset;

    auto& nodeTable = state.storageManager->getTable(indexInfo.tableID)->cast<NodeTable>();

    auto dataChunk = DataChunkState::getSingleValueDataChunkState();
    ValueVector idVec{LogicalType::INTERNAL_ID(), state.memoryManager, dataChunk};

    std::vector<std::unique_ptr<ValueVector>> scanVecs;
    std::vector<ValueVector*> scanPtrs;
    // Column 0 = LIST[INT64], Column 1 = LIST[DOUBLE]
    scanVecs.push_back(std::make_unique<ValueVector>(
        LogicalType::LIST(LogicalType::INT64()), state.memoryManager, dataChunk));
    scanVecs.push_back(std::make_unique<ValueVector>(
        LogicalType::LIST(LogicalType::DOUBLE()), state.memoryManager, dataChunk));
    scanPtrs.push_back(scanVecs[0].get());
    scanPtrs.push_back(scanVecs[1].get());

    NodeTableScanState scanState{&idVec, scanPtrs, dataChunk};
    scanState.setToTable(transaction, &nodeTable, indexInfo.columnIDs);

    internalID_t nid = {nodeOffset, indexInfo.tableID};
    idVec.setValue(0, nid);
    nodeTable.initScanState(transaction, scanState, nid.tableID, nid.offset);
    nodeTable.lookup(transaction, scanState);

    // 3. Replace the updated column with the new property vector value.
    std::vector<ValueVector*> insertPtrs;
    for (size_t c = 0; c < indexInfo.columnIDs.size(); c++) {
        if (c == state.updatedColumnIdx) {
            insertPtrs.push_back(&propertyVector);
        } else {
            insertPtrs.push_back(scanPtrs[c]);
        }
    }

    // 4. Re-insert.
    SparseVectorInsertState insertState;
    insert(transaction, nodeIDVector, insertPtrs, insertState);
}

// ── Flush / Persistence ─────────────────────────────────────────────────────

void SparseVectorIndex::flushIfDirty() {
    if (!dirty_) {
        return;
    }
    sparse_commit(*handle_);
    dirty_ = false;
}

void SparseVectorIndex::checkpointInMemory() {
    if (dirty_) {
        sparse_commit(*handle_);
        dirty_ = false;
    }
}

void SparseVectorIndex::checkpoint(main::ClientContext*, PageAllocator&) {
    // Sparse vector manages its own file — nothing to checkpoint via rag3db page allocator.
}

void SparseVectorIndex::rollbackCheckpoint() {
    // No rollback support in V1 — in-memory index is already consistent.
}

void SparseVectorIndex::finalize(main::ClientContext* context) {
    auto& si = storageInfo->cast<SparseVectorStorageInfo>();
    auto* sm = StorageManager::Get(*context);
    auto& nodeTable = sm->getTable(indexInfo.tableID)->cast<NodeTable>();
    auto numTotalRows =
        nodeTable.getNumTotalRows(&transaction::DUMMY_CHECKPOINT_TRANSACTION);
    if (numTotalRows == si.numCheckpointedNodes) {
        return;
    }
    auto transaction = transaction::Transaction::Get(*context);
    auto dataChunk = DataChunkState::getSingleValueDataChunkState();
    ValueVector idVector{LogicalType::INTERNAL_ID(), MemoryManager::Get(*context), dataChunk};

    // Output vectors: LIST[INT64] for indices, LIST[DOUBLE] for weights.
    std::vector<std::unique_ptr<ValueVector>> outputVecs;
    std::vector<ValueVector*> outputPtrs;
    outputVecs.push_back(std::make_unique<ValueVector>(
        LogicalType::LIST(LogicalType::INT64()), MemoryManager::Get(*context), dataChunk));
    outputVecs.push_back(std::make_unique<ValueVector>(
        LogicalType::LIST(LogicalType::DOUBLE()), MemoryManager::Get(*context), dataChunk));
    outputPtrs.push_back(outputVecs[0].get());
    outputPtrs.push_back(outputVecs[1].get());

    NodeTableScanState scanState{&idVector, outputPtrs, dataChunk};
    scanState.setToTable(transaction, &nodeTable, indexInfo.columnIDs);

    internalID_t nodeID = {INVALID_OFFSET, indexInfo.tableID};
    for (auto offset = si.numCheckpointedNodes; offset < numTotalRows; offset++) {
        nodeID.offset = offset;
        idVector.setValue(0, nodeID);
        nodeTable.initScanState(transaction, scanState, nodeID.tableID, nodeID.offset);
        nodeTable.lookup(transaction, scanState);

        if (outputPtrs[0]->isNull(0) || outputPtrs[1]->isNull(0)) {
            continue;
        }

        std::vector<uint32_t> indices;
        std::vector<float> weights;
        extractSparseVector(*outputPtrs[0], *outputPtrs[1], 0, indices, weights);

        add_sparse_document(*handle_, static_cast<uint64_t>(offset),
            rust::Slice<const uint32_t>(indices.data(), indices.size()),
            rust::Slice<const float>(weights.data(), weights.size()));
    }
    sparse_commit(*handle_);
    si.numCheckpointedNodes = numTotalRows;
}

} // namespace sparse_vector_extension
} // namespace rag3db
