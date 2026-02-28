#include "index/tantivy_index.h"

#include "catalog/tantivy_index_catalog_entry.h"
#include "common/serializer/buffer_reader.h"
#include "common/serializer/buffer_writer.h"
#include "common/serializer/deserializer.h"
#include "common/serializer/serializer.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

namespace rag3db {
namespace tantivy_fts_extension {

using namespace storage;
using namespace common;

// ── TantivyStorageInfo ──────────────────────────────────────────────────────

std::shared_ptr<BufferWriter> TantivyStorageInfo::serialize() const {
    auto writer = std::make_shared<BufferWriter>();
    auto ser = Serializer(writer);
    ser.write<offset_t>(numCheckpointedNodes);
    return writer;
}

std::unique_ptr<IndexStorageInfo> TantivyStorageInfo::deserialize(
    std::unique_ptr<BufferReader> reader) {
    offset_t numCheckpointedNodes = 0;
    Deserializer deSer{std::move(reader)};
    deSer.deserializeValue<offset_t>(numCheckpointedNodes);
    return std::make_unique<TantivyStorageInfo>(numCheckpointedNodes);
}

// ── TantivyIndex ────────────────────────────────────────────────────────────

TantivyIndex::TantivyIndex(IndexInfo indexInfo,
    std::unique_ptr<IndexStorageInfo> storageInfo, rust::Box<::TantivyHandle> handle)
    : Index{std::move(indexInfo), std::move(storageInfo)}, handle_{std::move(handle)} {
    // Cache user-visible field ids and types for fast insert.
    auto fields = get_field_ids(*handle_);
    for (const auto& f : fields) {
        if (std::string(f.name) == "_node_id") {
            continue;
        }
        fieldIds_.push_back(f.field_id);
        auto ft = std::string(f.field_type);
        fieldTypes_.push_back(ft);
        if (ft != "text") {
            hasMixedFields_ = true;
        }
    }
}

IndexType TantivyIndex::getIndexType() {
    static const IndexType TANTIVY_INDEX_TYPE{"TANTIVY",
        IndexConstraintType::SECONDARY_NON_UNIQUE, IndexDefinitionType::EXTENSION, load};
    return TANTIVY_INDEX_TYPE;
}

std::unique_ptr<Index> TantivyIndex::load(main::ClientContext* context, StorageManager*,
    IndexInfo indexInfo, std::span<uint8_t> storageInfoBuffer) {
    auto reader =
        std::make_unique<BufferReader>(storageInfoBuffer.data(), storageInfoBuffer.size());
    auto si = TantivyStorageInfo::deserialize(std::move(reader));
    // Get indexPath from the catalog aux info.
    auto catalog = catalog::Catalog::Get(*context);
    auto indexEntry = catalog->getIndex(transaction::Transaction::Get(*context), indexInfo.tableID,
        indexInfo.name);
    auto& auxInfo = indexEntry->getAuxInfo().cast<TantivyIndexAuxInfo>();
    auto handle = open_index(auxInfo.indexPath);
    return std::make_unique<TantivyIndex>(std::move(indexInfo), std::move(si), std::move(handle));
}

/// Map tantivy field type + physical type → LogicalType for scan vectors.
/// PhysicalTypeID is needed because BOOL maps to "i64" in Tantivy
/// but requires LogicalType::BOOL() for correct ValueVector reads.
static LogicalType fieldTypeToLogicalType(const std::string& ft, PhysicalTypeID physType) {
    if (physType == PhysicalTypeID::BOOL) return LogicalType::BOOL();
    if (ft == "u64") return LogicalType::UINT64();
    if (ft == "i64") return LogicalType::INT64();
    if (ft == "f64") return LogicalType::DOUBLE();
    return LogicalType::STRING();
}

// ── Insert / Delete states ──────────────────────────────────────────────────

struct TantivyInsertState final : Index::InsertState {};
struct TantivyDeleteState final : Index::DeleteState {};

struct TantivyUpdateState final : Index::UpdateState {
    // Index of the updated column within indexInfo.columnIDs.
    size_t updatedColumnIdx;
    std::unique_ptr<Index::DeleteState> deleteState;
    StorageManager* storageManager;
    MemoryManager* memoryManager;

    TantivyUpdateState(size_t updatedColumnIdx, std::unique_ptr<Index::DeleteState> deleteState,
        StorageManager* sm, MemoryManager* mm)
        : updatedColumnIdx{updatedColumnIdx}, deleteState{std::move(deleteState)},
          storageManager{sm}, memoryManager{mm} {}
};

std::unique_ptr<Index::InsertState> TantivyIndex::initInsertState(main::ClientContext*,
    visible_func) {
    return std::make_unique<TantivyInsertState>();
}

std::unique_ptr<Index::DeleteState> TantivyIndex::initDeleteState(
    const transaction::Transaction*, MemoryManager*, visible_func) {
    return std::make_unique<TantivyDeleteState>();
}

std::unique_ptr<Index::UpdateState> TantivyIndex::initUpdateState(main::ClientContext* context,
    column_id_t columnID, visible_func isVisible) {
    // Find which index in our columnIDs matches the updated column.
    size_t updatedIdx = 0;
    for (size_t i = 0; i < indexInfo.columnIDs.size(); i++) {
        if (indexInfo.columnIDs[i] == columnID) {
            updatedIdx = i;
            break;
        }
    }
    auto delState = initDeleteState(transaction::Transaction::Get(*context),
        MemoryManager::Get(*context), isVisible);
    return std::make_unique<TantivyUpdateState>(updatedIdx, std::move(delState),
        StorageManager::Get(*context), MemoryManager::Get(*context));
}

void TantivyIndex::update(transaction::Transaction* transaction,
    const ValueVector& nodeIDVector, ValueVector& propertyVector,
    UpdateState& updateState) {
    auto& state = updateState.cast<TantivyUpdateState>();
    // 1. Delete old document.
    delete_(transaction, nodeIDVector, *state.deleteState);

    // 2. Re-read all indexed columns from the node table to rebuild the document.
    auto pos = nodeIDVector.state->getSelVector()[0];
    auto nodeOffset = nodeIDVector.getValue<nodeID_t>(pos).offset;

    auto& nodeTable = state.storageManager->getTable(indexInfo.tableID)->cast<NodeTable>();

    auto dataChunk = DataChunkState::getSingleValueDataChunkState();
    ValueVector idVec{LogicalType::INTERNAL_ID(), state.memoryManager, dataChunk};

    std::vector<std::unique_ptr<ValueVector>> scanVecs;
    std::vector<ValueVector*> scanPtrs;
    for (size_t c = 0; c < indexInfo.columnIDs.size(); c++) {
        auto lt = fieldTypeToLogicalType(fieldTypes_[c], indexInfo.keyDataTypes[c]);
        auto vec = std::make_unique<ValueVector>(std::move(lt),
            state.memoryManager, dataChunk);
        scanPtrs.push_back(vec.get());
        scanVecs.push_back(std::move(vec));
    }

    NodeTableScanState scanState{&idVec, scanPtrs, dataChunk};
    scanState.setToTable(transaction, &nodeTable, indexInfo.columnIDs);

    internalID_t nid = {nodeOffset, indexInfo.tableID};
    idVec.setValue(0, nid);
    nodeTable.initScanState(transaction, scanState, nid.tableID, nid.offset);
    nodeTable.lookup(transaction, scanState);

    // 3. Replace the updated column with the new property vector value.
    //    The propertyVector comes from the SET statement and has the new value.
    std::vector<ValueVector*> insertPtrs;
    for (size_t c = 0; c < indexInfo.columnIDs.size(); c++) {
        if (c == state.updatedColumnIdx) {
            insertPtrs.push_back(&propertyVector);
        } else {
            insertPtrs.push_back(scanPtrs[c]);
        }
    }

    // 4. Re-insert using the existing insert() method.
    TantivyInsertState insertState;
    insert(transaction, nodeIDVector, insertPtrs, insertState);
}

// ── insert ──────────────────────────────────────────────────────────────────

void TantivyIndex::insert(transaction::Transaction*, const ValueVector& nodeIDVector,
    const std::vector<ValueVector*>& propertyVectors, InsertState&) {
    for (auto i = 0u; i < nodeIDVector.state->getSelSize(); i++) {
        auto pos = nodeIDVector.state->getSelVector()[i];
        auto nodeOffset = nodeIDVector.getValue<nodeID_t>(pos).offset;

        if (!hasMixedFields_) {
            std::vector<DocFieldText> fields;
            for (size_t f = 0; f < propertyVectors.size(); f++) {
                if (propertyVectors[f]->isNull(pos)) {
                    continue;
                }
                auto text = propertyVectors[f]->getValue<ku_string_t>(pos).getAsString();
                DocFieldText field;
                field.field_id = fieldIds_[f];
                field.value = rust::String(text);
                fields.push_back(std::move(field));
            }
            add_document_texts(*handle_, static_cast<uint64_t>(nodeOffset),
                rust::Slice<const DocFieldText>(fields.data(), fields.size()));
        } else {
            std::vector<DocFieldText> textFields;
            std::vector<DocFieldU64> u64Fields;
            std::vector<DocFieldI64> i64Fields;
            std::vector<DocFieldF64> f64Fields;
            for (size_t f = 0; f < propertyVectors.size(); f++) {
                if (propertyVectors[f]->isNull(pos)) {
                    continue;
                }
                auto& ft = fieldTypes_[f];
                if (ft == "text") {
                    auto text =
                        propertyVectors[f]->getValue<ku_string_t>(pos).getAsString();
                    DocFieldText field;
                    field.field_id = fieldIds_[f];
                    field.value = rust::String(text);
                    textFields.push_back(std::move(field));
                } else if (ft == "u64") {
                    DocFieldU64 field;
                    field.field_id = fieldIds_[f];
                    field.value = propertyVectors[f]->getValue<uint64_t>(pos);
                    u64Fields.push_back(field);
                } else if (ft == "i64") {
                    DocFieldI64 field;
                    field.field_id = fieldIds_[f];
                    if (indexInfo.keyDataTypes[f] == PhysicalTypeID::BOOL) {
                        field.value = propertyVectors[f]->getValue<bool>(pos) ? 1 : 0;
                    } else {
                        field.value = propertyVectors[f]->getValue<int64_t>(pos);
                    }
                    i64Fields.push_back(field);
                } else if (ft == "f64") {
                    DocFieldF64 field;
                    field.field_id = fieldIds_[f];
                    field.value = propertyVectors[f]->getValue<double>(pos);
                    f64Fields.push_back(field);
                }
            }
            add_document_mixed(*handle_, static_cast<uint64_t>(nodeOffset),
                rust::Slice<const DocFieldText>(textFields.data(), textFields.size()),
                rust::Slice<const DocFieldU64>(u64Fields.data(), u64Fields.size()),
                rust::Slice<const DocFieldI64>(i64Fields.data(), i64Fields.size()),
                rust::Slice<const DocFieldF64>(f64Fields.data(), f64Fields.size()));
        }
    }
    dirty_ = true;
}

// ── delete ──────────────────────────────────────────────────────────────────

void TantivyIndex::delete_(transaction::Transaction*, const ValueVector& nodeIDVector,
    DeleteState&) {
    for (auto i = 0u; i < nodeIDVector.state->getSelSize(); i++) {
        auto pos = nodeIDVector.state->getSelVector()[i];
        auto nodeOffset = nodeIDVector.getValue<nodeID_t>(pos).offset;
        delete_by_node_id(*handle_, static_cast<uint64_t>(nodeOffset));
    }
    dirty_ = true;
}

// ── Flush / Persistence ─────────────────────────────────────────────────────

void TantivyIndex::flushIfDirty() {
    if (!dirty_) {
        return;
    }
    commit(*handle_);
    reload_reader(*handle_);
    dirty_ = false;
}

void TantivyIndex::checkpointInMemory() {
    if (dirty_) {
        commit(*handle_);
        reload_reader(*handle_);
        dirty_ = false;
    }
}

void TantivyIndex::checkpoint(main::ClientContext*, PageAllocator&) {
    // Tantivy manages its own files — nothing to checkpoint via rag3db page allocator.
}

void TantivyIndex::rollbackCheckpoint() {
    rollback(*handle_);
}

void TantivyIndex::finalize(main::ClientContext* context) {
    auto& si = storageInfo->cast<TantivyStorageInfo>();
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

    // Build output vectors for each indexed column with correct types.
    std::vector<column_id_t> colIDs = indexInfo.columnIDs;
    std::vector<std::unique_ptr<ValueVector>> outputVecs;
    std::vector<ValueVector*> outputPtrs;
    for (size_t c = 0; c < colIDs.size(); c++) {
        auto logicalType = fieldTypeToLogicalType(fieldTypes_[c], indexInfo.keyDataTypes[c]);
        auto vec = std::make_unique<ValueVector>(std::move(logicalType),
            MemoryManager::Get(*context), dataChunk);
        outputPtrs.push_back(vec.get());
        outputVecs.push_back(std::move(vec));
    }

    NodeTableScanState scanState{&idVector, outputPtrs, dataChunk};
    scanState.setToTable(transaction, &nodeTable, colIDs);

    internalID_t nodeID = {INVALID_OFFSET, indexInfo.tableID};
    for (auto offset = si.numCheckpointedNodes; offset < numTotalRows; offset++) {
        nodeID.offset = offset;
        idVector.setValue(0, nodeID);
        nodeTable.initScanState(transaction, scanState, nodeID.tableID, nodeID.offset);
        nodeTable.lookup(transaction, scanState);

        if (!hasMixedFields_) {
            std::vector<DocFieldText> fields;
            for (size_t f = 0; f < colIDs.size(); f++) {
                if (outputPtrs[f]->isNull(0)) {
                    continue;
                }
                auto text = outputPtrs[f]->getValue<ku_string_t>(0).getAsString();
                DocFieldText field;
                field.field_id = fieldIds_[f];
                field.value = rust::String(text);
                fields.push_back(std::move(field));
            }
            add_document_texts(*handle_, static_cast<uint64_t>(offset),
                rust::Slice<const DocFieldText>(fields.data(), fields.size()));
        } else {
            std::vector<DocFieldText> textFields;
            std::vector<DocFieldU64> u64Fields;
            std::vector<DocFieldI64> i64Fields;
            std::vector<DocFieldF64> f64Fields;
            for (size_t f = 0; f < colIDs.size(); f++) {
                if (outputPtrs[f]->isNull(0)) {
                    continue;
                }
                auto& ft = fieldTypes_[f];
                if (ft == "text") {
                    auto text = outputPtrs[f]->getValue<ku_string_t>(0).getAsString();
                    DocFieldText field;
                    field.field_id = fieldIds_[f];
                    field.value = rust::String(text);
                    textFields.push_back(std::move(field));
                } else if (ft == "u64") {
                    DocFieldU64 field;
                    field.field_id = fieldIds_[f];
                    field.value = outputPtrs[f]->getValue<uint64_t>(0);
                    u64Fields.push_back(field);
                } else if (ft == "i64") {
                    DocFieldI64 field;
                    field.field_id = fieldIds_[f];
                    if (indexInfo.keyDataTypes[f] == PhysicalTypeID::BOOL) {
                        field.value = outputPtrs[f]->getValue<bool>(0) ? 1 : 0;
                    } else {
                        field.value = outputPtrs[f]->getValue<int64_t>(0);
                    }
                    i64Fields.push_back(field);
                } else if (ft == "f64") {
                    DocFieldF64 field;
                    field.field_id = fieldIds_[f];
                    field.value = outputPtrs[f]->getValue<double>(0);
                    f64Fields.push_back(field);
                }
            }
            add_document_mixed(*handle_, static_cast<uint64_t>(offset),
                rust::Slice<const DocFieldText>(textFields.data(), textFields.size()),
                rust::Slice<const DocFieldU64>(u64Fields.data(), u64Fields.size()),
                rust::Slice<const DocFieldI64>(i64Fields.data(), i64Fields.size()),
                rust::Slice<const DocFieldF64>(f64Fields.data(), f64Fields.size()));
        }
    }
    commit(*handle_);
    reload_reader(*handle_);
    si.numCheckpointedNodes = numTotalRows;
}

} // namespace tantivy_fts_extension
} // namespace rag3db
