#include "processor/operator/persistent/index_builder.h"

#include <thread>
#include <tuple>

#include "common/assert.h"
#include "common/cast.h"
#include "common/exception/copy.h"
#include "common/exception/message.h"
#include "common/type_utils.h"
#include "common/types/ku_string.h"
#include "storage/index/hash_index_utils.h"
#include "storage/table/node_table.h"
#include "storage/table/string_chunk_data.h"

namespace rag3db {
namespace processor {

using namespace rag3db::common;
using namespace rag3db::storage;

template<typename T>
bool IndexBufferWithWarningData<T>::full() const {
    return indexBuffer.full() || (warningDataBuffer != nullptr && warningDataBuffer->full());
}

template<typename T>
void IndexBufferWithWarningData<T>::append(T key, common::offset_t value,
    OptionalWarningSourceData&& warningData) {
    indexBuffer.push_back(std::make_pair(key, value));
    if (warningData.has_value()) {
        if (warningDataBuffer == nullptr) {
            warningDataBuffer = std::make_unique<OptionalWarningDataBuffer::element_type>();
        }
        warningDataBuffer->push_back(warningData.value());
    }
}

IndexBuilderGlobalQueues::IndexBuilderGlobalQueues(transaction::Transaction* transaction,
    NodeTable* nodeTable)
    : nodeTable(nodeTable), transaction{transaction} {
    TypeUtils::visit(
        pkTypeID(), [&](ku_string_t) { queues.emplace<Queue<std::string>>(); },
        [&]<HashablePrimitive T>(T) { queues.emplace<Queue<T>>(); }, [](auto) { KU_UNREACHABLE; });
}

PhysicalTypeID IndexBuilderGlobalQueues::pkTypeID() const {
    return nodeTable->getPKIndex()->keyTypeID();
}

void IndexBuilderGlobalQueues::consume(NodeBatchInsertErrorHandler& errorHandler) {
    for (auto index = 0u; index < NUM_HASH_INDEXES; index++) {
        maybeConsumeIndex(index, errorHandler);
    }
}

// The position of a line in the files of a COPY: the file, the block, the line in the block (the
// order CopyFromFileError sorts by). fileIdx exists for a CSV only; it is the column after the
// shared ones.
static uint64_t warningValue(const WarningSourceData& data, size_t idx) {
    return std::visit([](auto value) { return static_cast<uint64_t>(value); }, data.values[idx]);
}

static bool comesFirstInTheFiles(const WarningSourceData& a, const WarningSourceData& b) {
    const auto fileOf = [](const WarningSourceData& data) {
        return data.numValues > CopyConstants::SHARED_WARNING_DATA_NUM_COLUMNS ?
                   warningValue(data, CopyConstants::SHARED_WARNING_DATA_NUM_COLUMNS) :
                   0;
    };
    return std::make_tuple(fileOf(a), a.getBlockIdx(), a.getOffsetInBlock()) <
           std::make_tuple(fileOf(b), b.getBlockIdx(), b.getOffsetInBlock());
}

void IndexBuilderGlobalQueues::maybeConsumeIndex(size_t index,
    NodeBatchInsertErrorHandler& errorHandler) {
    auto& pkIndex = nodeTable->getPKIndex()->cast<PrimaryKeyIndex>();
    if (!pkIndex.tryLockTypedIndex(index)) {
        return;
    }

    std::visit(
        [&](auto&& queues) {
            using T = std::decay_t<decltype(queues.type)>;
            auto lck = pkIndex.adoptLockOfTypedIndex(index);
            auto& positions = filePositions[index];
            const auto isVisible = [&](offset_t offset) {
                return nodeTable->isVisible(transaction, offset);
            };
            const auto reject = [&](const auto& key, offset_t offset,
                                    const OptionalWarningSourceData& warningData) {
                errorHandler.handleError(
                    IndexBuilderError<T>{.message = ExceptionMessage::duplicatePKException(
                                             TypeUtils::toString(key)),
                        .key = key,
                        .nodeID = nodeID_t{offset, nodeTable->getTableID()},
                        .warningData = warningData});
            };
            IndexBufferWithWarningData<T> bufferWithWarningData;
            while (queues.array[index].pop(bufferWithWarningData)) {
                auto& buffer = bufferWithWarningData.indexBuffer;
                auto& warningDataBuffer = bufferWithWarningData.warningDataBuffer;
                uint64_t insertBufferOffset = 0;
                while (insertBufferOffset < buffer.size()) {
                    auto numValuesInserted = pkIndex.appendWithIndexPosNoLock(transaction, buffer,
                        insertBufferOffset, index, isVisible);
                    if (warningDataBuffer != nullptr) {
                        for (auto i = 0u; i < numValuesInserted; i++) {
                            positions[buffer[insertBufferOffset + i].second] =
                                (*warningDataBuffer)[insertBufferOffset + i];
                        }
                    }
                    insertBufferOffset += numValuesInserted;
                    if (insertBufferOffset >= buffer.size()) {
                        break;
                    }
                    const auto& erroneousEntry = buffer[insertBufferOffset];
                    OptionalWarningSourceData erroneousEntryWarningData;
                    if (warningDataBuffer != nullptr) {
                        erroneousEntryWarningData = (*warningDataBuffer)[insertBufferOffset];
                    }
                    // Between two lines of this COPY with the same key, the first of the files
                    // wins, whatever thread reached the index first (ticket 2026-10-10-copy-garde-
                    // un-doublon-selon-les-fils). A key held by a row from before the COPY, or
                    // from another statement, keeps its row as before.
                    offset_t holder = INVALID_OFFSET;
                    if (erroneousEntryWarningData.has_value() &&
                        pkIndex.lookupLocalWithIndexPosNoLock(erroneousEntry.first, index, holder,
                            isVisible)) {
                        const auto held = positions.find(holder);
                        if (held != positions.end() &&
                            comesFirstInTheFiles(*erroneousEntryWarningData, held->second)) {
                            // The holder is the later line: it loses (its row is deleted and its
                            // own line reported), and the entry is tried again at the same place.
                            const OptionalWarningSourceData holderWarningData = held->second;
                            positions.erase(held);
                            pkIndex.discardLocalWithIndexPosNoLock(erroneousEntry.first, index);
                            reject(erroneousEntry.first, holder, holderWarningData);
                            continue;
                        }
                    }
                    reject(erroneousEntry.first, erroneousEntry.second, erroneousEntryWarningData);
                    insertBufferOffset += 1; // skip the erroneous index then continue
                }
            }
            return;
        },
        std::move(queues));
}

IndexBuilderLocalBuffers::IndexBuilderLocalBuffers(IndexBuilderGlobalQueues& globalQueues)
    : globalQueues(&globalQueues) {
    TypeUtils::visit(
        globalQueues.pkTypeID(),
        [&](ku_string_t) { buffers = std::make_unique<Buffers<std::string>>(); },
        [&]<HashablePrimitive T>(T) { buffers = std::make_unique<Buffers<T>>(); },
        [](auto) { KU_UNREACHABLE; });
}

void IndexBuilderLocalBuffers::flush(NodeBatchInsertErrorHandler& errorHandler) {
    std::visit(
        [&](auto&& buffers) {
            for (auto i = 0u; i < buffers->size(); i++) {
                globalQueues->insert(i, std::move((*buffers)[i]), errorHandler);
            }
        },
        buffers);
}

IndexBuilder::IndexBuilder(std::shared_ptr<IndexBuilderSharedState> sharedState)
    : sharedState(std::move(sharedState)), localBuffers(this->sharedState->globalQueues) {}

void IndexBuilderSharedState::quitProducer() {
    if (producers.fetch_sub(1, std::memory_order_relaxed) == 1) {
        done.store(true, std::memory_order_relaxed);
    }
}

static OptionalWarningSourceData getWarningDataFromChunks(
    const std::vector<storage::ColumnChunkData*>& chunks, common::idx_t posInChunk) {
    OptionalWarningSourceData ret;
    if (!chunks.empty()) {
        ret = WarningSourceData::constructFromData(chunks, posInChunk);
    }
    return ret;
}

void IndexBuilder::insert(const ColumnChunkData& chunk,
    const std::vector<ColumnChunkData*>& warningData, offset_t nodeOffset, offset_t numNodes,
    NodeBatchInsertErrorHandler& errorHandler) {
    TypeUtils::visit(
        chunk.getDataType().getPhysicalType(),
        [&]<HashablePrimitive T>(T) {
            for (auto i = 0u; i < numNodes; i++) {
                if (checkNonNullConstraint(chunk, warningData, nodeOffset, i, errorHandler)) {
                    auto value = chunk.getValue<T>(i);
                    localBuffers.insert(value, nodeOffset + i,
                        getWarningDataFromChunks(warningData, i), errorHandler);
                }
            }
        },
        [&](ku_string_t) {
            auto& stringColumnChunk = ku_dynamic_cast<const StringChunkData&>(chunk);
            for (auto i = 0u; i < numNodes; i++) {
                if (checkNonNullConstraint(chunk, warningData, nodeOffset, i, errorHandler)) {
                    auto value = stringColumnChunk.getValue<std::string>(i);
                    localBuffers.insert(std::move(value), nodeOffset + i,
                        getWarningDataFromChunks(warningData, i), errorHandler);
                }
            }
        },
        [&](auto) {
            throw CopyException(ExceptionMessage::invalidPKType(chunk.getDataType().toString()));
        });
}

void IndexBuilder::finishedProducing(NodeBatchInsertErrorHandler& errorHandler) {
    localBuffers.flush(errorHandler);
    sharedState->consume(errorHandler);
    while (!sharedState->isDone()) {
        std::this_thread::sleep_for(std::chrono::microseconds(500));
        sharedState->consume(errorHandler);
    }
}

void IndexBuilder::finalize(ExecutionContext* /*context*/,
    NodeBatchInsertErrorHandler& errorHandler) {
    // Flush anything added by last node group.
    localBuffers.flush(errorHandler);

    sharedState->consume(errorHandler);
}

bool IndexBuilder::checkNonNullConstraint(const ColumnChunkData& chunk,
    const std::vector<storage::ColumnChunkData*>& warningData, offset_t nodeOffset,
    offset_t chunkOffset, NodeBatchInsertErrorHandler& errorHandler) {
    const auto* nullChunk = chunk.getNullData();
    if (nullChunk->isNull(chunkOffset)) {
        TypeUtils::visit(
            chunk.getDataType().getPhysicalType(),
            [&](struct_entry_t) {
                // primary key cannot be struct
                KU_UNREACHABLE;
            },
            [&]<typename T>(T) {
                errorHandler.handleError<T>({.message = ExceptionMessage::nullPKException(),
                    .key = {},
                    .nodeID =
                        nodeID_t{nodeOffset + chunkOffset, sharedState->nodeTable->getTableID()},
                    .warningData = getWarningDataFromChunks(warningData, chunkOffset)});
            });
        return false;
    }
    return true;
}

} // namespace processor
} // namespace rag3db
