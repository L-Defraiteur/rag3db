#include "processor/operator/partitioner.h"

#include "binder/expression/expression_util.h"
#include "main/client_context.h"
#include "processor/execution_context.h"
#include "processor/warning_context.h"
#include "storage/buffer_manager/memory_manager.h"
#include "storage/wal/local_wal.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"
#include "storage/table/rel_table.h"
#include "transaction/transaction.h"

using namespace rag3db::common;
using namespace rag3db::storage;

namespace rag3db {
namespace processor {

std::string PartitionerPrintInfo::toString() const {
    std::string result = "Indexes: ";
    result += binder::ExpressionUtil::toString(expressions);
    return result;
}

void PartitionerFunctions::partitionRelData(ValueVector* key, ValueVector* partitionIdxes) {
    KU_ASSERT(key->state == partitionIdxes->state &&
              key->dataType.getPhysicalType() == PhysicalTypeID::INT64);
    for (auto i = 0u; i < key->state->getSelVector().getSelSize(); i++) {
        const auto pos = key->state->getSelVector()[i];
        const partition_idx_t partitionIdx =
            key->getValue<offset_t>(pos) >> StorageConfig::NODE_GROUP_SIZE_LOG2;
        partitionIdxes->setValue(pos, partitionIdx);
    }
}

void CopyPartitionerSharedState::initialize(const logical_type_vec_t& columnTypes,
    idx_t numPartitioners, const main::ClientContext* clientContext) {
    PartitionerSharedState::initialize(columnTypes, numPartitioners, clientContext);
    Partitioner::initializePartitioningStates(columnTypes, partitioningBuffers, numPartitions,
        numPartitioners);
}

void CopyPartitionerSharedState::merge(
    const std::vector<std::unique_ptr<PartitioningBuffer>>& localPartitioningStates) {
    std::unique_lock xLck{mtx};
    KU_ASSERT(partitioningBuffers.size() == localPartitioningStates.size());
    for (auto partitioningIdx = 0u; partitioningIdx < partitioningBuffers.size();
         partitioningIdx++) {
        partitioningBuffers[partitioningIdx]->merge(*localPartitioningStates[partitioningIdx]);
    }
}

void CopyPartitionerSharedState::resetState(common::idx_t partitioningIdx) {
    PartitionerSharedState::resetState(partitioningIdx);
    partitioningBuffers[partitioningIdx].reset();
}

void PartitioningBuffer::merge(const PartitioningBuffer& localPartitioningState) const {
    KU_ASSERT(partitions.size() == localPartitioningState.partitions.size());
    for (auto partitionIdx = 0u; partitionIdx < partitions.size(); partitionIdx++) {
        auto& sharedPartition = partitions[partitionIdx];
        auto& localPartition = localPartitioningState.partitions[partitionIdx];
        sharedPartition->merge(*localPartition);
    }
}

Partitioner::Partitioner(PartitionerInfo info, PartitionerDataInfo dataInfo,
    std::shared_ptr<CopyPartitionerSharedState> sharedState,
    std::unique_ptr<PhysicalOperator> child, uint32_t id, std::unique_ptr<OPPrintInfo> printInfo)
    : Sink{type_, std::move(child), id, std::move(printInfo)}, dataInfo{std::move(dataInfo)},
      info{std::move(info)}, sharedState{std::move(sharedState)} {
    partitionIdxes = std::make_unique<ValueVector>(LogicalTypeID::INT64);
}

void Partitioner::initGlobalStateInternal(ExecutionContext* context) {
    const auto clientContext = context->clientContext;
    // If initialization is required
    if (!sharedState->srcNodeTable) {
        auto storageManager = StorageManager::Get(*clientContext);
        auto catalog = catalog::Catalog::Get(*clientContext);
        auto transaction = transaction::Transaction::Get(*clientContext);
        auto fromTableID =
            catalog->getTableCatalogEntry(transaction, dataInfo.fromTableName)->getTableID();
        auto toTableID =
            catalog->getTableCatalogEntry(transaction, dataInfo.toTableName)->getTableID();
        sharedState->srcNodeTable = storageManager->getTable(fromTableID)->ptrCast<NodeTable>();
        sharedState->dstNodeTable = storageManager->getTable(toTableID)->ptrCast<NodeTable>();
        auto& relGroupEntry = catalog->getTableCatalogEntry(transaction, dataInfo.tableName)
                                  ->constCast<catalog::RelGroupCatalogEntry>();
        auto relEntryInfo = relGroupEntry.getRelEntryInfo(fromTableID, toTableID);
        sharedState->relTable = storageManager->getTable(relEntryInfo->oid)->ptrCast<RelTable>();
    }
    sharedState->initialize(dataInfo.columnTypes, info.infos.size(), clientContext);
}

void Partitioner::initLocalStateInternal(ResultSet* resultSet, ExecutionContext* context) {
    localState = std::make_unique<PartitionerLocalState>();
    initializePartitioningStates(dataInfo.columnTypes, localState->partitioningBuffers,
        sharedState->numPartitions, info.infos.size());
    for (const auto& evaluator : dataInfo.columnEvaluators) {
        evaluator->init(*resultSet, context->clientContext);
    }
}

DataChunk Partitioner::constructDataChunk(const std::shared_ptr<DataChunkState>& state) const {
    const auto numColumns = dataInfo.columnEvaluators.size();
    DataChunk dataChunk(numColumns, state);
    for (auto i = 0u; i < numColumns; ++i) {
        auto& evaluator = dataInfo.columnEvaluators[i];
        dataChunk.insert(i, evaluator->resultVector);
    }
    return dataChunk;
}

void Partitioner::initializePartitioningStates(const logical_type_vec_t& columnTypes,
    std::vector<std::unique_ptr<PartitioningBuffer>>& partitioningBuffers,
    const std::array<partition_idx_t, CopyPartitionerSharedState::DIRECTIONS>& numPartitions,
    idx_t numPartitioners) {
    partitioningBuffers.resize(numPartitioners);
    for (auto partitioningIdx = 0u; partitioningIdx < numPartitioners; partitioningIdx++) {
        const auto numPartition = numPartitions[partitioningIdx];
        auto partitioningBuffer = std::make_unique<PartitioningBuffer>();
        partitioningBuffer->partitions.reserve(numPartition);
        for (auto i = 0u; i < numPartition; i++) {
            partitioningBuffer->partitions.push_back(
                std::make_unique<InMemChunkedNodeGroupCollection>(LogicalType::copy(columnTypes)));
        }
        partitioningBuffers[partitioningIdx] = std::move(partitioningBuffer);
    }
}

// Le chargement en masse journalisé s'applique-t-il à ce COPY de relations ? Oui quand le
// réglage force_checkpoint_on_copy est éteint, que la transaction écrit au journal, et que le
// COPY n'écarte aucune ligne (IGNORE_ERRORS) : une relation écartée après avoir été journalisée
// reviendrait au rejeu.
static bool journalsItsRels(const main::ClientContext& clientContext) {
    return !clientContext.getClientConfig()->forceCheckpointOnCopy &&
           transaction::Transaction::Get(clientContext)->shouldLogToWAL() &&
           !WarningContext::Get(clientContext)->getIgnoreErrorsOption();
}

void Partitioner::logRelsToWAL(ExecutionContext* context,
    const ValueVector& relOffsetVector) const {
    const auto clientContext = context->clientContext;
    const auto mm = MemoryManager::Get(*clientContext);
    const auto& relTable = *sharedState->relTable;
    const auto& selVector = relOffsetVector.state->getSelVector();
    const auto numRels = selVector.getSelSize();
    // Les colonnes du journal : l'origine, l'arrivée, l'identité, puis les propriétés — la
    // forme d'une relation locale, celle que le rejeu attend.
    const idx_t numColumns = relTable.getNumColumns() + 1;
    KU_ASSERT(numColumns <= dataInfo.columnEvaluators.size());
    const table_id_t tableIDs[] = {relTable.getFromNodeTableID(), relTable.getToNodeTableID(),
        relTable.getTableID()};
    const auto state = std::make_shared<DataChunkState>();
    state->getSelVectorUnsafe().setToUnfiltered(numRels);
    std::vector<std::unique_ptr<ValueVector>> ownedVectors;
    std::vector<ValueVector*> vectors;
    for (idx_t column = 0; column < numColumns; column++) {
        const auto& source = *dataInfo.columnEvaluators[column]->resultVector;
        const auto isInternalID = column < 3;
        auto vector = std::make_unique<ValueVector>(
            isInternalID ? LogicalType::INTERNAL_ID() : source.dataType.copy(), mm, state);
        for (sel_t i = 0; i < numRels; i++) {
            const auto pos = selVector[i];
            if (isInternalID) {
                // Le plan porte ces trois colonnes comme de simples décalages.
                vector->setValue<internalID_t>(i,
                    internalID_t{source.getValue<offset_t>(pos), tableIDs[column]});
            } else if (source.isNull(pos)) {
                vector->setNull(i, true);
            } else {
                vector->setNull(i, false);
                vector->copyFromVectorData(i, &source, pos);
            }
        }
        vectors.push_back(vector.get());
        ownedVectors.push_back(std::move(vector));
    }
    transaction::Transaction::Get(*clientContext)
        ->getLocalWAL()
        .logTableInsertion(relTable.getTableID(), TableType::REL, numRels, vectors);
}

void Partitioner::executeInternal(ExecutionContext* context) {
    const auto relOffsetVector = resultSet->getValueVector(info.relOffsetDataPos);
    const auto journaled = journalsItsRels(*context->clientContext);
    while (children[0]->getNextTuple(context)) {
        KU_ASSERT(dataInfo.columnEvaluators.size() >= 1);
        const auto numRels = relOffsetVector->state->getSelVector().getSelSize();
        evaluateExpressions(numRels);
        {
            // Journalisé : la réservation des identités et l'écriture au journal ne font qu'un,
            // pour que l'ordre du journal soit celui des identités.
            std::unique_lock<std::mutex> journalOrder;
            if (journaled) {
                journalOrder = std::unique_lock{sharedState->journalOrderMtx};
            }
            auto currentRelOffset = sharedState->relTable->reserveRelOffsets(numRels);
            for (auto i = 0u; i < numRels; i++) {
                const auto pos = relOffsetVector->state->getSelVector()[i];
                relOffsetVector->setValue<offset_t>(pos, currentRelOffset++);
            }
            if (journaled && numRels > 0) {
                logRelsToWAL(context, *relOffsetVector);
            }
        }
        for (auto partitioningIdx = 0u; partitioningIdx < info.infos.size(); partitioningIdx++) {
            auto& partitionInfo = info.infos[partitioningIdx];
            auto keyVector = dataInfo.columnEvaluators[partitionInfo.keyIdx]->resultVector;
            partitionIdxes->state = keyVector->state;
            partitionInfo.partitionerFunc(keyVector.get(), partitionIdxes.get());
            auto chunkToCopyFrom = constructDataChunk(keyVector->state);
            copyDataToPartitions(*MemoryManager::Get(*context->clientContext), partitioningIdx,
                chunkToCopyFrom);
        }
    }
    sharedState->merge(localState->partitioningBuffers);
}

void Partitioner::evaluateExpressions(uint64_t numRels) const {
    for (auto i = 0u; i < dataInfo.evaluateTypes.size(); ++i) {
        auto evaluator = dataInfo.columnEvaluators[i].get();
        switch (dataInfo.evaluateTypes[i]) {
        case ColumnEvaluateType::DEFAULT: {
            evaluator->evaluate(numRels);
        } break;
        default: {
            evaluator->evaluate();
        }
        }
    }
}

void Partitioner::copyDataToPartitions(MemoryManager& memoryManager,
    partition_idx_t partitioningIdx, const DataChunk& chunkToCopyFrom) const {
    std::vector<ValueVector*> vectorsToAppend;
    vectorsToAppend.reserve(chunkToCopyFrom.getNumValueVectors());
    for (auto j = 0u; j < chunkToCopyFrom.getNumValueVectors(); j++) {
        vectorsToAppend.push_back(&chunkToCopyFrom.getValueVectorMutable(j));
    }
    for (auto i = 0u; i < chunkToCopyFrom.state->getSelVector().getSelSize(); i++) {
        const auto posToCopyFrom = chunkToCopyFrom.state->getSelVector()[i];
        const auto partitionIdx = partitionIdxes->getValue<partition_idx_t>(posToCopyFrom);
        KU_ASSERT(
            partitionIdx < localState->getPartitioningBuffer(partitioningIdx)->partitions.size());
        const auto& partition =
            localState->getPartitioningBuffer(partitioningIdx)->partitions[partitionIdx];
        partition->append(memoryManager, vectorsToAppend, i, 1);
    }
}

} // namespace processor
} // namespace rag3db
