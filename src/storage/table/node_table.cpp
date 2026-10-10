#include "storage/table/node_table.h"

#include <algorithm>
#include <array>
#include <cstdio>

#include "catalog/catalog.h"
#include "catalog/catalog_entry/index_catalog_entry.h"
#include "catalog/catalog_entry/node_table_catalog_entry.h"
#include "common/cast.h"
#include "common/data_chunk/data_chunk_state.h"
#include "common/exception/message.h"
#include "common/exception/runtime.h"
#include "common/exception/transaction_manager.h"
#include "common/string_format.h"
#include "common/types/types.h"
#include "main/client_context.h"
#include "storage/local_storage/local_node_table.h"
#include "storage/local_storage/local_storage.h"
#include "storage/local_storage/local_table.h"
#include "storage/storage_manager.h"
#include "storage/wal/local_wal.h"
#include "transaction/lock_manager.h"
#include "transaction/transaction.h"

using namespace rag3db::catalog;
using namespace rag3db::common;
using namespace rag3db::transaction;
using namespace rag3db::evaluator;

namespace rag3db {
namespace storage {

static std::pair<ChunkedNodeGroup*, row_idx_t> getChunkedGroupAndRow(const NodeGroup& nodeGroup,
    row_idx_t rowIdx) {
    for (auto chunkedGroupIdx = 0u; chunkedGroupIdx < nodeGroup.getNumChunkedGroups();
         chunkedGroupIdx++) {
        auto* chunkedGroup = nodeGroup.getChunkedNodeGroup(chunkedGroupIdx);
        if (rowIdx < chunkedGroup->getNumRows()) {
            return {chunkedGroup, rowIdx};
        }
        rowIdx -= chunkedGroup->getNumRows();
    }
    KU_UNREACHABLE;
}

// The nodes a transaction created, logged when it commits, with the values they have then. Every
// row is logged, deleted or not, so that a replay gives each the offset it got here; the
// deletions follow, with those final offsets.
static void logLocalNodeInsertionsToWAL(LocalWAL& wal, table_id_t tableID,
    LocalNodeTable& localNodeTable, MemoryManager& mm) {
    for (auto localNodeGroupIdx = 0u; localNodeGroupIdx < localNodeTable.getNumNodeGroups();
         localNodeGroupIdx++) {
        const auto localNodeGroup = localNodeTable.getNodeGroup(localNodeGroupIdx);
        for (auto chunkedGroupIdx = 0u; chunkedGroupIdx < localNodeGroup->getNumChunkedGroups();
             chunkedGroupIdx++) {
            const auto* chunkedGroup = localNodeGroup->getChunkedNodeGroup(chunkedGroupIdx);
            for (auto startRow = 0u; startRow < chunkedGroup->getNumRows();
                 startRow += DEFAULT_VECTOR_CAPACITY) {
                const auto numRows = std::min<row_idx_t>(DEFAULT_VECTOR_CAPACITY,
                    chunkedGroup->getNumRows() - startRow);
                auto state = std::make_shared<DataChunkState>();
                state->getSelVectorUnsafe().setToUnfiltered(numRows);
                std::vector<std::unique_ptr<ValueVector>> ownedVectors;
                std::vector<ValueVector*> vectors;
                ownedVectors.reserve(chunkedGroup->getNumColumns());
                vectors.reserve(chunkedGroup->getNumColumns());
                for (auto columnID = 0u; columnID < chunkedGroup->getNumColumns(); columnID++) {
                    const auto& columnChunk = chunkedGroup->getColumnChunk(columnID);
                    auto vector =
                        std::make_unique<ValueVector>(columnChunk.getDataType().copy(), &mm, state);
                    ChunkState chunkState;
                    columnChunk.scan(&DUMMY_TRANSACTION, chunkState, *vector, startRow, numRows);
                    vectors.push_back(vector.get());
                    ownedVectors.push_back(std::move(vector));
                }
                wal.logTableInsertion(tableID, TableType::NODE, numRows, vectors);
            }
        }
    }
}

NodeTableVersionRecordHandler::NodeTableVersionRecordHandler(NodeTable* table) : table(table) {}

void NodeTableVersionRecordHandler::applyFuncToChunkedGroups(version_record_handler_op_t func,
    node_group_idx_t nodeGroupIdx, row_idx_t startRow, row_idx_t numRows,
    transaction_t commitTS) const {
    table->getNodeGroupNoLock(nodeGroupIdx)
        ->applyFuncToChunkedGroups(func, startRow, numRows, commitTS);
}

void NodeTableVersionRecordHandler::rollbackInsert(main::ClientContext* context,
    node_group_idx_t nodeGroupIdx, row_idx_t startRow, row_idx_t numRows) const {
    table->rollbackPKIndexInsert(context, startRow, numRows, nodeGroupIdx);
    table->rollbackIndexInsert(StorageUtils::getStartOffsetOfNodeGroup(nodeGroupIdx) + startRow);

    // the only case where a node group would be empty (and potentially removed before) is if an
    // exception occurred while adding its first chunk
    KU_ASSERT(nodeGroupIdx < table->getNumNodeGroups() || startRow == 0);
    if (nodeGroupIdx < table->getNumNodeGroups()) {
        VersionRecordHandler::rollbackInsert(context, nodeGroupIdx, startRow, numRows);
        table->getNodeGroupNoLock(nodeGroupIdx)->rollbackInsert(startRow);
    }
    // Le compte de la table recule de tout ce que cet enregistrement y avait ajouté, que les
    // lignes aient été écrites ou non. Les décalages sont réservés, et comptés, avant
    // l'écriture ; quand celle-ci échoue (mémoire épuisée au milieu d'un COPY), le groupe ne
    // porte pas ces lignes. Ne retirer que ce que le groupe porte laissait le compte trop
    // haut d'un lot par échec : la table donnait ensuite des décalages au-delà de ses lignes.
    table->rollbackGroupCollectionInsert(numRows);
}

NodeGroupScanResult NodeTableScanState::scanNext(Transaction* transaction, offset_t startOffset,
    offset_t numNodes) {
    KU_ASSERT(columns.size() == outputVectors.size());
    if (source == TableScanSource::NONE) {
        return NODE_GROUP_SCAN_EMPTY_RESULT;
    }
    auto nodeGroupStartOffset = StorageUtils::getStartOffsetOfNodeGroup(nodeGroupIdx);
    const auto tableID = table->getTableID();
    if (source == TableScanSource::UNCOMMITTED) {
        nodeGroupStartOffset = transaction->getUncommittedOffset(tableID, nodeGroupStartOffset);
    }
    auto startOffsetInGroup = startOffset - nodeGroupStartOffset;
    const NodeGroupScanResult scanResult =
        nodeGroup->scan(transaction, *this, startOffsetInGroup, numNodes);
    if (scanResult == NODE_GROUP_SCAN_EMPTY_RESULT) {
        return scanResult;
    }
    for (auto i = 0u; i < scanResult.numRows; i++) {
        nodeIDVector->setValue(i,
            nodeID_t{nodeGroupStartOffset + scanResult.startRow + i, tableID});
    }
    return scanResult;
}

std::unique_ptr<NodeTableScanState> IndexScanHelper::initScanState(const Transaction* transaction,
    DataChunk& dataChunk) {
    std::vector<ValueVector*> outVectors;
    for (auto& vector : dataChunk.valueVectors) {
        outVectors.push_back(vector.get());
    }
    auto scanState = std::make_unique<NodeTableScanState>(nullptr, outVectors, dataChunk.state);
    scanState->setToTable(transaction, table, index->getIndexInfo().columnIDs, {});
    return scanState;
}

namespace {

struct UncommittedIndexInserter final : IndexScanHelper {
    UncommittedIndexInserter(row_idx_t startNodeOffset, NodeTable* table, Index* index,
        visible_func isVisible)
        : IndexScanHelper(table, index), startNodeOffset(startNodeOffset),
          nodeIDVector(LogicalType::INTERNAL_ID()), isVisible(std::move(isVisible)) {}

    std::unique_ptr<NodeTableScanState> initScanState(const Transaction* transaction,
        DataChunk& dataChunk) override;

    bool processScanOutput(main::ClientContext* context, NodeGroupScanResult scanResult,
        const std::vector<ValueVector*>& scannedVectors) override;

    row_idx_t startNodeOffset;
    ValueVector nodeIDVector;
    visible_func isVisible;
    std::unique_ptr<Index::InsertState> insertState;
};

struct RollbackPKDeleter final : IndexScanHelper {
    RollbackPKDeleter(row_idx_t startNodeOffset, row_idx_t numRows, NodeTable* table,
        PrimaryKeyIndex* pkIndex)
        : IndexScanHelper(table, pkIndex),
          semiMask(SemiMaskUtil::createMask(startNodeOffset + numRows)) {
        semiMask->maskRange(startNodeOffset, startNodeOffset + numRows);
        semiMask->enable();
    }

    std::unique_ptr<NodeTableScanState> initScanState(const Transaction* transaction,
        DataChunk& dataChunk) override;

    bool processScanOutput(main::ClientContext* context, NodeGroupScanResult scanResult,
        const std::vector<ValueVector*>& scannedVectors) override;

    std::unique_ptr<SemiMask> semiMask;
};

std::unique_ptr<NodeTableScanState> UncommittedIndexInserter::initScanState(
    const Transaction* transaction, DataChunk& dataChunk) {
    auto scanState = IndexScanHelper::initScanState(transaction, dataChunk);
    nodeIDVector.setState(dataChunk.state);
    scanState->source = TableScanSource::UNCOMMITTED;
    return scanState;
}

bool UncommittedIndexInserter::processScanOutput(main::ClientContext* context,
    NodeGroupScanResult scanResult, const std::vector<ValueVector*>& scannedVectors) {
    if (scanResult == NODE_GROUP_SCAN_EMPTY_RESULT) {
        return false;
    }
    for (auto i = 0u; i < scanResult.numRows; i++) {
        nodeIDVector.setValue(i, nodeID_t{startNodeOffset + i, table->getTableID()});
    }
    if (!insertState) {
        insertState = index->initInsertState(context, isVisible);
    }
    index->commitInsert(transaction::Transaction::Get(*context), nodeIDVector, {scannedVectors},
        *insertState);
    startNodeOffset += scanResult.numRows;
    return true;
}

std::unique_ptr<NodeTableScanState> RollbackPKDeleter::initScanState(const Transaction* transaction,
    DataChunk& dataChunk) {
    auto scanState = IndexScanHelper::initScanState(transaction, dataChunk);
    scanState->source = TableScanSource::COMMITTED;
    scanState->semiMask = semiMask.get();
    return scanState;
}

template<typename T>
concept notIndexHashable = !IndexHashable<T>;

bool RollbackPKDeleter::processScanOutput(main::ClientContext* context,
    NodeGroupScanResult scanResult, const std::vector<ValueVector*>& scannedVectors) {
    if (scanResult == NODE_GROUP_SCAN_EMPTY_RESULT) {
        return false;
    }
    KU_ASSERT(scannedVectors.size() == 1);
    auto& scannedVector = *scannedVectors[0];
    auto& pkIndex = index->cast<PrimaryKeyIndex>();
    const auto rollbackFunc = [&]<IndexHashable T>(T) {
        for (idx_t i = 0; i < scannedVector.state->getSelSize(); ++i) {
            const auto pos = scannedVector.state->getSelVector()[i];
            T key = scannedVector.getValue<T>(pos);
            static constexpr auto isVisible = [](offset_t) { return true; };
            // Seulement si la clé ne mène pas à une ligne validée. Le balayage rend des blocs
            // entiers, pas la seule plage annulée : on y trouve des lignes qui restent, et une
            // ligne refusée pour clé en double porte la clé d'une ligne qui reste. Retirer ces
            // clés faisait disparaître de l'index des lignes validées, tant que leur clé n'avait
            // pas passé un point de reprise.
            // Le critère n'est pas la plage de cet enregistrement : annuler un enregistrement
            // rend invisibles toutes les lignes de son bloc ajoutées par la transaction
            // (VectorVersionInfo::rollbackInsertions), celles des enregistrements qui restent à
            // annuler comprises ; leur propre balayage ne les rend plus, et leurs clés doivent
            // donc sortir ici.
            // Avec plusieurs écrivains, les lignes non validées d'un autre écrivain du même bloc
            // perdraient aussi leur clé : à reprendre avec les verrous.
            if (offset_t lookupOffset = 0;
                pkIndex.lookup(transaction::Transaction::Get(*context), key, lookupOffset,
                    isVisible) &&
                !table->isVisibleNoLock(&transaction::DUMMY_CHECKPOINT_TRANSACTION,
                    lookupOffset)) {
                // If we delete the key then it will not be visible to future transactions within
                // this process
                pkIndex.discardLocal(key);
            }
        }
    };
    TypeUtils::visit(scannedVector.dataType.getPhysicalType(), std::cref(rollbackFunc),
        []<notIndexHashable T>(T) { KU_UNREACHABLE; });
    return true;
}
} // namespace

void NodeTableScanState::setToTable(const Transaction* transaction, Table* table_,
    std::vector<column_id_t> columnIDs_, std::vector<ColumnPredicateSet> columnPredicateSets_,
    RelDataDirection) {
    TableScanState::setToTable(transaction, table_, columnIDs_, std::move(columnPredicateSets_));
    committedColumnIDs = columnIDs;
    columns.resize(columnIDs.size());
    for (auto i = 0u; i < columnIDs.size(); i++) {
        if (const auto columnID = columnIDs[i];
            columnID == INVALID_COLUMN_ID || columnID == ROW_IDX_COLUMN_ID) {
            columns[i] = nullptr;
        } else {
            columns[i] = &table->cast<NodeTable>().getColumn(columnID);
        }
    }
}

bool NodeTableScanState::scanNext(Transaction* transaction) {
    if (source == TableScanSource::NONE) {
        return false;
    }
    KU_ASSERT(columns.size() == outputVectors.size());
    const NodeGroupScanResult scanResult = nodeGroup->scan(transaction, *this);
    if (scanResult == NODE_GROUP_SCAN_EMPTY_RESULT) {
        return false;
    }
    auto nodeGroupStartOffset = StorageUtils::getStartOffsetOfNodeGroup(nodeGroupIdx);
    const auto tableID = table->getTableID();
    if (source == TableScanSource::UNCOMMITTED) {
        nodeGroupStartOffset = transaction->getUncommittedOffset(tableID, nodeGroupStartOffset);
    }
    for (auto i = 0u; i < scanResult.numRows; i++) {
        auto& nodeID = nodeIDVector->getValue<nodeID_t>(i);
        nodeID.tableID = tableID;
        nodeID.offset = nodeGroupStartOffset + scanResult.startRow + i;
    }
    return true;
}

NodeTable::NodeTable(const StorageManager* storageManager,
    const NodeTableCatalogEntry* nodeTableEntry, MemoryManager* mm)
    : Table{nodeTableEntry, storageManager, mm},
      pkColumnID{nodeTableEntry->getColumnID(nodeTableEntry->getPrimaryKeyName())},
      versionRecordHandler(this) {
    auto* dataFH = storageManager->getDataFH();
    auto& pageAllocator = *dataFH->getPageManager();
    const auto maxColumnID = nodeTableEntry->getMaxColumnID();
    columns.resize(maxColumnID + 1);
    for (auto& property : nodeTableEntry->getProperties()) {
        const auto columnID = nodeTableEntry->getColumnID(property.getName());
        const auto columnName =
            StorageUtils::getColumnName(property.getName(), StorageUtils::ColumnType::DEFAULT, "");
        columns[columnID] = ColumnFactory::createColumn(columnName, property.getType().copy(),
            dataFH, mm, shadowFile, enableCompression);
    }
    auto& pkDefinition = nodeTableEntry->getPrimaryKeyDefinition();
    KU_ASSERT(pkColumnID != INVALID_COLUMN_ID);
    auto hashIndexType = PrimaryKeyIndex::getIndexType();
    IndexInfo indexInfo{PrimaryKeyIndex::DEFAULT_NAME, hashIndexType.typeName, tableID,
        {pkColumnID}, {pkDefinition.getType().getPhysicalType()},
        hashIndexType.constraintType == IndexConstraintType::PRIMARY,
        hashIndexType.definitionType == IndexDefinitionType::BUILTIN};
    indexes.push_back(IndexHolder{PrimaryKeyIndex::createNewIndex(indexInfo,
        storageManager->isInMemory(), *mm, pageAllocator, shadowFile)});
    nodeGroups = std::make_unique<NodeGroupCollection>(*mm,
        LocalNodeTable::getNodeTableColumnTypes(*nodeTableEntry), enableCompression,
        storageManager->getDataFH() ? ResidencyState::ON_DISK : ResidencyState::IN_MEMORY,
        &versionRecordHandler);
}

row_idx_t NodeTable::getNumTotalRows(const Transaction* transaction) {
    auto numLocalRows = 0u;
    if (transaction && transaction->getLocalStorage()) {
        if (const auto localTable = transaction->getLocalStorage()->getLocalTable(tableID)) {
            numLocalRows = localTable->getNumTotalRows();
        }
    }
    return numLocalRows + nodeGroups->getNumTotalRows();
}

void NodeTable::initScanState(Transaction* transaction, TableScanState& scanState, bool) const {
    auto& nodeScanState = scanState.cast<NodeTableScanState>();
    NodeGroup* nodeGroup = nullptr;
    switch (nodeScanState.source) {
    case TableScanSource::COMMITTED: {
        nodeGroup = nodeGroups->getNodeGroup(nodeScanState.nodeGroupIdx);
        if (!nodeScanState.committedColumnIDs.empty()) {
            nodeScanState.columnIDs = nodeScanState.committedColumnIDs;
        }
    } break;
    case TableScanSource::UNCOMMITTED: {
        const auto localTable = transaction->getLocalStorage()->getLocalTable(tableID);
        KU_ASSERT(localTable);
        const auto& localNodeTable = localTable->cast<LocalNodeTable>();
        nodeGroup = localNodeTable.getNodeGroup(nodeScanState.nodeGroupIdx);
        KU_ASSERT(nodeGroup);
        // Les groupes locaux sont rangés par position de propriété : après
        // ALTER TABLE … DROP, ce n'est plus le numéro de colonne.
        if (nodeScanState.committedColumnIDs.empty()) {
            nodeScanState.committedColumnIDs = nodeScanState.columnIDs;
        }
        nodeScanState.columnIDs =
            localNodeTable.getLocalColumnIDs(nodeScanState.committedColumnIDs);
    } break;
    case TableScanSource::NONE: {
        // DO NOTHING.
    } break;
    default: {
        KU_UNREACHABLE;
    }
    }
    nodeScanState.initState(transaction, nodeGroup);
}

void NodeTable::initScanState(Transaction* transaction, TableScanState& scanState,
    table_id_t tableID, offset_t startOffset) const {
    if (transaction->isUnCommitted(tableID, startOffset)) {
        scanState.source = TableScanSource::UNCOMMITTED;
        scanState.nodeGroupIdx =
            StorageUtils::getNodeGroupIdx(transaction->getLocalRowIdx(tableID, startOffset));
    } else {
        scanState.source = TableScanSource::COMMITTED;
        scanState.nodeGroupIdx = StorageUtils::getNodeGroupIdx(startOffset);
    }
    initScanState(transaction, scanState);
}

bool NodeTable::scanInternal(Transaction* transaction, TableScanState& scanState) {
    scanState.resetOutVectors();
    return scanState.scanNext(transaction);
}

template<bool lock>
bool NodeTable::lookup(const Transaction* transaction, const TableScanState& scanState) const {
    KU_ASSERT(scanState.nodeIDVector->state->getSelVector().getSelSize() == 1);
    const auto nodeIDPos = scanState.nodeIDVector->state->getSelVector()[0];
    if (scanState.nodeIDVector->isNull(nodeIDPos)) {
        return false;
    }
    const auto nodeOffset = scanState.nodeIDVector->readNodeOffset(nodeIDPos);
    const offset_t rowIdxInGroup =
        transaction->isUnCommitted(tableID, nodeOffset) ?
            transaction->getLocalRowIdx(tableID, nodeOffset) -
                StorageUtils::getStartOffsetOfNodeGroup(scanState.nodeGroupIdx) :
            nodeOffset - StorageUtils::getStartOffsetOfNodeGroup(scanState.nodeGroupIdx);
    scanState.rowIdxVector->setValue<row_idx_t>(nodeIDPos, rowIdxInGroup);
    if constexpr (lock) {
        return scanState.nodeGroup->lookup(transaction, scanState);
    } else {
        return scanState.nodeGroup->lookupSharedLock(transaction, scanState);
    }
}

template bool NodeTable::lookup<true>(const Transaction* transaction,
    const TableScanState& scanState) const;
template bool NodeTable::lookup<false>(const Transaction* transaction,
    const TableScanState& scanState) const;

template<bool lock>
bool NodeTable::lookupMultiple(Transaction* transaction, TableScanState& scanState) const {
    const auto numRowsToRead = scanState.nodeIDVector->state->getSelSize();
    sel_t numRowsRead = 0;
    for (auto i = 0u; i < numRowsToRead; i++) {
        const auto nodeIDPos = scanState.nodeIDVector->state->getSelVector()[i];
        if (scanState.nodeIDVector->isNull(nodeIDPos)) {
            continue;
        }
        const auto nodeOffset = scanState.nodeIDVector->readNodeOffset(nodeIDPos);
        const auto isUnCommitted = transaction->isUnCommitted(tableID, nodeOffset);
        const auto source =
            isUnCommitted ? TableScanSource::UNCOMMITTED : TableScanSource::COMMITTED;
        const auto nodeGroupIdx =
            isUnCommitted ?
                StorageUtils::getNodeGroupIdx(transaction->getLocalRowIdx(tableID, nodeOffset)) :
                StorageUtils::getNodeGroupIdx(nodeOffset);
        const offset_t rowIdxInGroup =
            isUnCommitted ? transaction->getLocalRowIdx(tableID, nodeOffset) -
                                StorageUtils::getStartOffsetOfNodeGroup(nodeGroupIdx) :
                            nodeOffset - StorageUtils::getStartOffsetOfNodeGroup(nodeGroupIdx);
        if (scanState.source == source && scanState.nodeGroupIdx == nodeGroupIdx) {
            // If the scan state is already initialized for the same source and node group, we can
            // skip re-initialization.
        } else {
            scanState.source = source;
            scanState.nodeGroupIdx = nodeGroupIdx;
            initScanState(transaction, scanState);
        }
        scanState.rowIdxVector->setValue<row_idx_t>(nodeIDPos, rowIdxInGroup);
        if constexpr (lock) {
            numRowsRead += scanState.nodeGroup->lookup(transaction, scanState, i);
        } else {
            numRowsRead += scanState.nodeGroup->lookupSharedLock(transaction, scanState, i);
        }
    }
    return numRowsRead == numRowsToRead;
}

template bool NodeTable::lookupMultiple<true>(Transaction* transaction,
    TableScanState& scanState) const;
template bool NodeTable::lookupMultiple<false>(Transaction* transaction,
    TableScanState& scanState) const;

offset_t NodeTable::validateUniquenessConstraint(const Transaction* transaction,
    const ValueVector& pkVector) const {
    KU_ASSERT(pkVector.state->getSelVector().getSelSize() == 1);
    const auto pkVectorPos = pkVector.state->getSelVector()[0];
    if (offset_t offset = INVALID_OFFSET;
        getPKIndex()->lookup(transaction, const_cast<ValueVector*>(&pkVector), pkVectorPos, offset,
            [&](offset_t offset_) { return isVisible(transaction, offset_); })) {
        return offset;
    }
    if (const auto localTable = transaction->getLocalStorage()->getLocalTable(tableID)) {
        return localTable->cast<LocalNodeTable>().validateUniquenessConstraint(transaction,
            pkVector);
    }
    return INVALID_OFFSET;
}

void NodeTable::validatePkNotExists(const Transaction* transaction, ValueVector* pkVector) const {
    offset_t dummyOffset = INVALID_OFFSET;
    auto& selVector = pkVector->state->getSelVector();
    KU_ASSERT(selVector.getSelSize() == 1);
    if (pkVector->isNull(selVector[0])) {
        throw RuntimeException(ExceptionMessage::nullPKException());
    }
    if (getPKIndex()->lookup(transaction, pkVector, selVector[0], dummyOffset,
            getUniquenessVisibleFunc(transaction))) {
        throw RuntimeException(
            ExceptionMessage::duplicatePKException(pkVector->getAsValue(selVector[0])->toString()));
    }
}

bool NodeTable::isWritableIndex(IndexHolder& indexHolder, const Transaction* transaction) {
    if (indexHolder.isDetached()) {
        return false;
    }
    if (indexHolder.isLoaded()) {
        return true;
    }
    if (transaction->isRecovery()) {
        indexHolder.detach();
        hasChanges = true;
        return false;
    }
    throw RuntimeException(stringFormat("Index {} {} {}.", indexHolder.getName(),
        INDEX_NOT_LOADED_FOR_WRITE, tableName));
}

std::vector<std::vector<idx_t>> NodeTable::getIndexPropertyPositions(
    const TableCatalogEntry& tableEntry) const {
    std::vector<std::vector<idx_t>> positions(indexes.size());
    for (auto i = 0u; i < indexes.size(); i++) {
        for (const auto columnID : indexes[i].getIndexInfo().columnIDs) {
            positions[i].push_back(tableEntry.getPropertyPosition(columnID));
        }
    }
    return positions;
}

void NodeTable::initInsertState(main::ClientContext* context, TableInsertState& insertState) {
    auto& nodeInsertState = insertState.cast<NodeTableInsertState>();
    nodeInsertState.indexInsertStates.resize(indexes.size());
    if (nodeInsertState.indexPropertyPositions.size() != indexes.size()) {
        // Pas posées par l'appelant (le rejeu) : une recherche au catalogue.
        nodeInsertState.indexPropertyPositions =
            getIndexPropertyPositions(*catalog::Catalog::Get(*context)->getTableCatalogEntry(
                transaction::Transaction::Get(*context), tableID));
    }
    for (auto i = 0u; i < indexes.size(); i++) {
        auto& indexHolder = indexes[i];
        if (!isWritableIndex(indexHolder, transaction::Transaction::Get(*context))) {
            nodeInsertState.indexInsertStates[i] = nullptr;
            continue;
        }
        const auto index = indexHolder.getIndex();
        nodeInsertState.indexInsertStates[i] =
            index->initInsertState(context, [&](offset_t offset) {
                return isVisible(transaction::Transaction::Get(*context), offset);
            });
    }
}

void NodeTable::insert(Transaction* transaction, TableInsertState& insertState) {
    const auto& nodeInsertState = insertState.cast<NodeTableInsertState>();
    auto& nodeIDSelVector = nodeInsertState.nodeIDVector.state->getSelVector();
    KU_ASSERT(nodeInsertState.propertyVectors[0]->state->getSelVector().getSelSize() == 1);
    KU_ASSERT(nodeIDSelVector.getSelSize() == 1);
    if (nodeInsertState.nodeIDVector.isNull(nodeIDSelVector[0])) {
        return;
    }
    const auto localTable = transaction->getLocalStorage()->getOrCreateLocalTable(*this);
    // Marche A3′ : sous le mode multi-écrivains, l'insertion tient l'index de la table en
    // partagé (un COPY le tient en exclusif : les deux ne se croisent pas) et la clé en
    // exclusif, jusqu'à la fin de la transaction. Une seconde insertion de la même clé attend
    // ici ; puis le contrôle d'unicité, contre le dernier état validé, la refuse ou la laisse
    // passer selon ce que la première a fait.
    if (transaction->usesLocks()) {
        const auto pkPos = nodeInsertState.pkVector.state->getSelVector()[0];
        const std::array requests{
            LockRequest{LockResource::index(tableID), LockMode::SHARED},
            LockRequest{LockResource::row(tableID, lockKeyOf(nodeInsertState.pkVector, pkPos)),
                LockMode::EXCLUSIVE}};
        transaction->acquireLocks(requests);
    }
    validatePkNotExists(transaction, const_cast<ValueVector*>(&nodeInsertState.pkVector));
    localTable->insert(transaction, insertState);
    for (auto i = 0u; i < indexes.size(); i++) {
        if (!nodeInsertState.indexInsertStates[i]) {
            continue; // index sauté par initInsertState
        }
        auto index = indexes[i].getIndex();
        std::vector<ValueVector*> indexedPropertyVectors;
        for (const auto position : nodeInsertState.indexPropertyPositions[i]) {
            indexedPropertyVectors.push_back(insertState.propertyVectors[position]);
        }
        index->insert(transaction, nodeInsertState.nodeIDVector, indexedPropertyVectors,
            *nodeInsertState.indexInsertStates[i]);
    }
    if (!insertState.logToWAL) {
        localTable->doNotLogInsertions();
    }
    // Not logged here: NodeTable::commit logs the nodes of the transaction with the values they
    // have at commit, so that updates and deletions of those nodes, whose offsets are provisional
    // until then, need no record of their own.
    hasChanges = true;
}

void NodeTable::initUpdateState(main::ClientContext* context, TableUpdateState& updateState) const {
    auto& nodeUpdateState = updateState.cast<NodeTableUpdateState>();
    nodeUpdateState.indexUpdateState.resize(indexes.size());
    for (auto i = 0u; i < indexes.size(); i++) {
        auto& indexHolder = indexes[i];
        if (!indexHolder.isLoaded()) {
            // Un index non chargé ne répond pas ; sa description suffit à dire s'il porte
            // sur la colonne. S'il n'y porte pas, la mise à jour ne le regarde pas.
            const auto& columnIDs = indexHolder.getIndexInfo().columnIDs;
            if (std::find(columnIDs.begin(), columnIDs.end(), nodeUpdateState.columnID) !=
                columnIDs.end()) {
                const_cast<NodeTable*>(this)->isWritableIndex(
                    const_cast<IndexHolder&>(indexHolder),
                    transaction::Transaction::Get(*context));
            }
            nodeUpdateState.indexUpdateState[i] = nullptr;
            continue;
        }
        auto index = indexHolder.getIndex();
        if (index->isPrimary() || !index->isBuiltOnColumn(nodeUpdateState.columnID)) {
            nodeUpdateState.indexUpdateState[i] = nullptr;
            continue;
        }
        nodeUpdateState.indexUpdateState[i] =
            index->initUpdateState(context, nodeUpdateState.columnID, [&](offset_t offset) {
                return isVisible(transaction::Transaction::Get(*context), offset);
            });
    }
}

void NodeTable::update(Transaction* transaction, TableUpdateState& updateState) {
    // NOTE: We assume all inputs are flattened now. This is to simplify the implementation.
    // We should optimize this to take unflattened input later.
    auto& nodeUpdateState = updateState.constCast<NodeTableUpdateState>();
    KU_ASSERT(nodeUpdateState.nodeIDVector.state->getSelVector().getSelSize() == 1 &&
              nodeUpdateState.propertyVector.state->getSelVector().getSelSize() == 1);
    const auto pos = nodeUpdateState.nodeIDVector.state->getSelVector()[0];
    if (nodeUpdateState.nodeIDVector.isNull(pos)) {
        return;
    }
    const auto pkIndex = getPKIndex();
    if (nodeUpdateState.columnID == pkColumnID && pkIndex) {
        throw RuntimeException("Cannot update pk.");
    }
    const auto nodeOffset = nodeUpdateState.nodeIDVector.readNodeOffset(pos);
    // Une ligne de la transaction n'est pas encore dans un index qui l'insère à la validation
    // (commitInsert, l'index vectoriel) : il l'y insérera avec sa valeur finale. La mettre à jour
    // ici l'insérait deux fois — des arêtes en double dans le graphe (banc, 10 octobre).
    const auto isLocalRow = transaction->isUnCommitted(tableID, nodeOffset);
    // Marche A4′ : une ligne validée se prend en exclusif avant d'être écrite (une ligne de la
    // transaction n'appartient qu'à elle).
    if (!isLocalRow && transaction->usesLocks()) {
        lockRowForWrite(transaction, nodeOffset, lockKeyOfRow(transaction, nodeOffset));
    }
    for (auto i = 0u; i < indexes.size(); i++) {
        if (!nodeUpdateState.needToUpdateIndex(i)) {
            continue;
        }
        if (isLocalRow && indexes[i].needCommitInsert()) {
            continue;
        }
        auto index = indexes[i].getIndex();
        index->update(transaction, nodeUpdateState.nodeIDVector, nodeUpdateState.propertyVector,
            *nodeUpdateState.indexUpdateState[i]);
    }
    if (isLocalRow) {
        const auto localTable = transaction->getLocalStorage()->getLocalTable(tableID);
        KU_ASSERT(localTable);
        localTable->update(&DUMMY_TRANSACTION, updateState);
    } else {
        const auto nodeGroupIdx = StorageUtils::getNodeGroupIdx(nodeOffset);
        const auto rowIdxInGroup =
            nodeOffset - StorageUtils::getStartOffsetOfNodeGroup(nodeGroupIdx);
        nodeGroups->getNodeGroup(nodeGroupIdx)
            ->update(transaction, rowIdxInGroup, nodeUpdateState.columnID,
                nodeUpdateState.propertyVector);
    }
    // A node of this transaction is logged at commit with its final values: no record here, its
    // offset is provisional.
    if (!transaction->isUnCommitted(tableID, nodeOffset) && updateState.logToWAL &&
        transaction->shouldLogToWAL()) {
        KU_ASSERT(transaction->isWriteTransaction());
        auto& wal = transaction->getLocalWAL();
        wal.logNodeUpdate(tableID, nodeUpdateState.columnID, nodeOffset,
            &nodeUpdateState.propertyVector);
    }
    hasChanges = true;
}

void NodeTable::initDeleteStates(const Transaction* transaction, TableDeleteState& deleteState) {
    auto& nodeDeleteState = ku_dynamic_cast<NodeTableDeleteState&>(deleteState);
    if (nodeDeleteState.indexDeleteStatesInitialized) {
        return;
    }
    nodeDeleteState.indexDeleteStates.resize(indexes.size());
    for (auto i = 0u; i < indexes.size(); i++) {
        if (!isWritableIndex(indexes[i], transaction)) {
            nodeDeleteState.indexDeleteStates[i] = nullptr;
            continue;
        }
        nodeDeleteState.indexDeleteStates[i] =
            indexes[i].getIndex()->initDeleteState(transaction, memoryManager,
                getVisibleFunc(transaction));
    }
    nodeDeleteState.indexDeleteStatesInitialized = true;
}

void NodeTable::finalizeDelete(Transaction* transaction, TableDeleteState& deleteState) {
    auto& nodeDeleteState = ku_dynamic_cast<NodeTableDeleteState&>(deleteState);
    if (!nodeDeleteState.indexDeleteStatesInitialized) {
        return;
    }
    for (auto i = 0u; i < indexes.size(); i++) {
        if (!nodeDeleteState.indexDeleteStates[i]) {
            continue; // index sauté par initDeleteStates
        }
        indexes[i].getIndex()->finalizeDelete(transaction, *nodeDeleteState.indexDeleteStates[i]);
    }
}

bool NodeTable::delete_(Transaction* transaction, TableDeleteState& deleteState) {
    auto& nodeDeleteState = ku_dynamic_cast<NodeTableDeleteState&>(deleteState);
    KU_ASSERT(nodeDeleteState.nodeIDVector.state->getSelVector().getSelSize() == 1);
    const auto pos = nodeDeleteState.nodeIDVector.state->getSelVector()[0];
    if (nodeDeleteState.nodeIDVector.isNull(pos)) {
        return false;
    }
    bool isDeleted = false;
    const auto nodeOffset = nodeDeleteState.nodeIDVector.readNodeOffset(pos);
    // Marche A4′ : une ligne validée se prend en exclusif avant d'être supprimée ; la clé est
    // déjà là (le planificateur la lit pour l'index).
    if (transaction->usesLocks() && !transaction->isUnCommitted(tableID, nodeOffset)) {
        lockRowForWrite(transaction, nodeOffset,
            lockKeyOf(nodeDeleteState.pkVector,
                nodeDeleteState.pkVector.state->getSelVector()[0]));
    }
    // Use pre-created index delete states (initialized once, reused across calls).
    initDeleteStates(transaction, deleteState);
    for (auto i = 0u; i < indexes.size(); i++) {
        if (!nodeDeleteState.indexDeleteStates[i]) {
            continue; // index sauté par initDeleteStates
        }
        indexes[i].getIndex()->delete_(transaction, nodeDeleteState.nodeIDVector,
            *nodeDeleteState.indexDeleteStates[i]);
    }

    // The deletion of a node of this transaction is logged at commit, with its final offset.
    const auto isLocalNode = transaction->isUnCommitted(tableID, nodeOffset);
    if (isLocalNode) {
        const auto localTable = transaction->getLocalStorage()->getLocalTable(tableID);
        isDeleted = localTable->delete_(&DUMMY_TRANSACTION, deleteState);
    } else {
        const auto nodeGroupIdx = StorageUtils::getNodeGroupIdx(nodeOffset);
        const auto rowIdxInGroup =
            nodeOffset - StorageUtils::getStartOffsetOfNodeGroup(nodeGroupIdx);
        isDeleted = nodeGroups->getNodeGroup(nodeGroupIdx)->delete_(transaction, rowIdxInGroup);
        if (transaction->shouldAppendToUndoBuffer()) {
            transaction->pushDeleteInfo(nodeGroupIdx, rowIdxInGroup, 1, &versionRecordHandler);
        }
    }
    if (isDeleted) {
        hasChanges = true;
        if (!isLocalNode && deleteState.logToWAL && transaction->shouldLogToWAL()) {
            KU_ASSERT(transaction->isWriteTransaction());
            auto& wal = transaction->getLocalWAL();
            wal.logNodeDeletion(tableID, nodeOffset, &nodeDeleteState.pkVector);
        }
    }
    return isDeleted;
}

void NodeTable::addColumn(Transaction* transaction, TableAddColumnState& addColumnState,
    PageAllocator& pageAllocator) {
    auto& definition = addColumnState.propertyDefinition;
    columns.push_back(ColumnFactory::createColumn(definition.getName(), definition.getType().copy(),
        pageAllocator.getDataFH(), memoryManager, shadowFile, enableCompression));
    LocalTable* localTable = nullptr;
    if (transaction->getLocalStorage()) {
        localTable = transaction->getLocalStorage()->getLocalTable(tableID);
    }
    if (localTable) {
        localTable->addColumn(addColumnState);
    }
    nodeGroups->addColumn(addColumnState, &pageAllocator);
    hasChanges = true;
}

std::pair<offset_t, offset_t> NodeTable::appendToLastNodeGroup(Transaction* transaction,
    const std::vector<column_id_t>& columnIDs, InMemChunkedNodeGroup& chunkedGroup,
    PageAllocator& pageAllocator) {
    hasChanges = true;
    return nodeGroups->appendToLastNodeGroupAndFlushWhenFull(transaction, columnIDs, chunkedGroup,
        pageAllocator);
}

DataChunk NodeTable::constructDataChunkForColumns(const std::vector<column_id_t>& columnIDs) const {
    std::vector<LogicalType> types;
    for (const auto& columnID : columnIDs) {
        KU_ASSERT(columnID < columns.size());
        types.push_back(columns[columnID]->getDataType().copy());
    }
    return constructDataChunk(memoryManager, std::move(types));
}

void NodeTable::commit(main::ClientContext* context, TableCatalogEntry* tableEntry,
    LocalTable* localTable) {
    const auto startNodeOffset = nodeGroups->getNumTotalRows();
    auto& localNodeTable = localTable->cast<LocalNodeTable>();

    // L'ordre des colonnes des groupes locaux, tel que la table locale l'a reçu du catalogue
    // à sa création, et non celui du catalogue du commit : un ALTER dans la transaction, après
    // des lignes locales, les ferait différer.
    const auto columnIDsToCommit = localNodeTable.getCommittedColumnIDs();

    auto transaction = transaction::Transaction::Get(*context);
    // 1. Append all tuples from local storage to nodeGroups regardless of deleted or not.
    // Note: We cannot simply remove all deleted tuples in local node table, as they may have
    // connected local rels. Directly removing them will cause shift of committed node offset,
    // leading to an inconsistent result with connected rels.
    nodeGroups->append(transaction, columnIDsToCommit, localNodeTable.getNodeGroups());
    localNodeTable.rowsAreCommitted();
    const auto logLocalNodes = transaction->shouldLogToWAL() && localTable->logsInsertions();
    if (logLocalNodes) {
        logLocalNodeInsertionsToWAL(transaction->getLocalWAL(), tableID, localNodeTable,
            *memoryManager);
    }
    // 2. Set deleted flag for tuples that are deleted in local storage.
    // La clé d'une ligne locale, à sa position dans les groupes locaux.
    const auto localPKColumnID = localNodeTable.getLocalColumnID(pkColumnID);
    row_idx_t numLocalRows = 0u;
    for (auto localNodeGroupIdx = 0u; localNodeGroupIdx < localNodeTable.getNumNodeGroups();
         localNodeGroupIdx++) {
        const auto localNodeGroup = localNodeTable.getNodeGroup(localNodeGroupIdx);
        if (localNodeGroup->hasDeletions(transaction)) {
            // TODO(Guodong): Assume local storage is small here. Should optimize the loop away by
            // grabbing a set of deleted rows.
            for (auto row = 0u; row < localNodeGroup->getNumRows(); row++) {
                if (localNodeGroup->isDeleted(transaction, row)) {
                    const auto nodeOffset = startNodeOffset + numLocalRows + row;
                    if (logLocalNodes) {
                        const auto [chunkedGroup, rowInChunkedGroup] =
                            getChunkedGroupAndRow(*localNodeGroup, row);
                        ValueVector pkVector(
                            chunkedGroup->getColumnChunk(localPKColumnID).getDataType().copy(),
                            memoryManager, DataChunkState::getSingleValueDataChunkState());
                        ChunkState chunkState;
                        chunkedGroup->getColumnChunk(localPKColumnID)
                            .lookup(&DUMMY_TRANSACTION, chunkState, rowInChunkedGroup, pkVector, 0);
                        transaction->getLocalWAL().logNodeDeletion(tableID, nodeOffset, &pkVector);
                    }
                    const auto nodeGroupIdx = StorageUtils::getNodeGroupIdx(nodeOffset);
                    const auto rowIdxInGroup =
                        nodeOffset - StorageUtils::getStartOffsetOfNodeGroup(nodeGroupIdx);
                    [[maybe_unused]] const bool isDeleted =
                        nodeGroups->getNodeGroup(nodeGroupIdx)->delete_(transaction, rowIdxInGroup);
                    KU_ASSERT(isDeleted);
                    if (transaction->shouldAppendToUndoBuffer()) {
                        transaction->pushDeleteInfo(nodeGroupIdx, rowIdxInGroup, 1,
                            &versionRecordHandler);
                    }
                }
            }
        }
        numLocalRows += localNodeGroup->getNumRows();
    }

    // 3. Scan index columns for newly inserted tuples.
    for (auto& index : indexes) {
        // Chargé d'abord : needCommitInsert interroge l'index lui-même.
        if (!index.isLoaded()) {
            if (index.isDetached() || transaction->isRecovery()) {
                // Au rejeu du journal, la table reçoit ses lignes sans cet index : il est
                // détaché, à rebâtir.
                if (!index.isDetached()) {
                    index.detach();
                    hasChanges = true;
                }
                continue;
            }
            throw RuntimeException(
                "Cannot commit index insertions for index " + index.getName() +
                ", because it is not loaded. Please load the extension for the index first.");
        }
        if (!index.needCommitInsert()) {
            continue;
        }
        // L'index de clé primaire contrôle l'unicité avec la visibilité de l'insertion : un
        // contrôle qui différerait ici ferait échouer la validation après l'ajout des lignes
        // aux groupes (étape 1), une transaction à moitié validée (C7 du banc, 10 octobre).
        UncommittedIndexInserter indexInserter{startNodeOffset, this, index.getIndex(),
            index.getIndex()->isPrimary() ? getUniquenessVisibleFunc(transaction) :
                                            getVisibleFunc(transaction)};
        // We need to scan from local storage here because some tuples in local node groups might
        // have been deleted.
        scanIndexColumns(context, indexInserter, localNodeTable.getNodeGroups(), &localNodeTable);
    }

    // 4. Clear local table.
    localTable->clear(*MemoryManager::Get(*context));
}

visible_func NodeTable::getVisibleFunc(const Transaction* transaction) const {
    return
        [this, transaction](offset_t offset_) -> bool { return isVisible(transaction, offset_); };
}

// Sous les verrous (marche A3′), l'unicité se contrôle contre le dernier état validé et non
// contre l'instantané : une clé validée par un autre écrivain après l'instantané de cette
// transaction est un doublon (PostgreSQL lit l'état de validation de toute ligne qui porte la
// clé), et une clé dont la ligne a été supprimée et validée après l'instantané est libre. Le
// verrou de la clé, tenu par l'autre jusqu'à la fin de sa transaction, a fait attendre jusque-là :
// s'il a validé son insertion, c'est l'erreur de clé en double ; s'il a annulé, la clé est libre.
visible_func NodeTable::getUniquenessVisibleFunc(const Transaction* transaction) const {
    if (!transaction->usesLocks()) {
        return getVisibleFunc(transaction);
    }
    return [this, transaction](offset_t offset_) -> bool {
        return isVisibleToLatestCommit(transaction, offset_);
    };
}

bool NodeTable::checkpoint(main::ClientContext* context, TableCatalogEntry* tableEntry,
    PageAllocator& pageAllocator) {
    const bool ret = hasChanges;
    if (hasChanges) {
        // Deleted columns are vacuumed and not checkpointed.
        std::vector<std::unique_ptr<Column>> checkpointColumns;
        std::vector<column_id_t> columnIDs;
        for (auto& property : tableEntry->getProperties()) {
            auto columnID = tableEntry->getColumnID(property.getName());
            checkpointColumns.push_back(std::move(columns[columnID]));
            columnIDs.push_back(columnID);
        }
        columns = std::move(checkpointColumns);

        std::vector<Column*> checkpointColumnPtrs;
        for (const auto& column : columns) {
            checkpointColumnPtrs.push_back(column.get());
        }

        NodeGroupCheckpointState state{columnIDs, std::move(checkpointColumnPtrs), pageAllocator,
            memoryManager};
        nodeGroups->checkpoint(*memoryManager, state);
        for (auto& index : indexes) {
            index.checkpoint(context, pageAllocator);
        }
        tableEntry->vacuumColumnIDs(0 /*nextColumnID*/);
        renumberColumns(*catalog::Catalog::Get(*context),
            tableEntry->constCast<NodeTableCatalogEntry>());
        hasChanges = false;
    }
    return ret;
}

void NodeTable::rollbackPKIndexInsert(main::ClientContext* context, row_idx_t startRow,
    row_idx_t numRows_, node_group_idx_t nodeGroupIdx_) {
    const row_idx_t startNodeOffset =
        startRow + StorageUtils::getStartOffsetOfNodeGroup(nodeGroupIdx_);

    RollbackPKDeleter pkDeleter{startNodeOffset, numRows_, this, getPKIndex()};
    scanIndexColumns(context, pkDeleter, *nodeGroups);
}

// NOLINTNEXTLINE(readability-make-member-function-const): Semantically non-const.
void NodeTable::rollbackIndexInsert(offset_t firstRolledBackOffset) {
    for (auto& index : indexes) {
        if (index.isLoaded()) {
            index.getIndex()->rollbackInsert(firstRolledBackOffset);
        }
    }
}

void NodeTable::rollbackGroupCollectionInsert(row_idx_t numRows_) {
    nodeGroups->rollbackInsert(numRows_);
}

void NodeTable::rollbackCheckpoint() {
    for (auto& index : indexes) {
        index.rollbackCheckpoint();
    }
}

void NodeTable::reclaimStorage(PageAllocator& pageAllocator) const {
    nodeGroups->reclaimStorage(pageAllocator);
    getPKIndex()->reclaimStorage(pageAllocator);
}

TableStats NodeTable::getStats(const Transaction* transaction) const {
    auto stats = nodeGroups->getStats();
    if (const auto localTable = transaction->getLocalStorage()->getLocalTable(tableID)) {
        const auto& localNodeTable = localTable->cast<LocalNodeTable>();
        // Les statistiques locales sont rangées par position de propriété.
        stats.merge(localNodeTable.getCommittedColumnIDs(), localNodeTable.getStats());
    }
    // Et celles des lignes que la transaction a déjà écrites dans la table (un COPY, des lignes
    // locales versées avant lui), qui attendent sa validation.
    if (const auto pendingStats = transaction->getLocalStorage()->getPendingNodeStats(tableID)) {
        for (const auto& pending : *pendingStats) {
            stats.merge(pending.columnIDs, pending.stats);
        }
    }
    return stats;
}

bool NodeTable::isVisible(const Transaction* transaction, offset_t offset) const {
    return isVisible(transaction->getStartTS(), transaction->getID(), offset);
}

bool NodeTable::isVisible(transaction_t startTS, transaction_t transactionID,
    offset_t offset) const {
    auto [nodeGroupIdx, offsetInGroup] = StorageUtils::getNodeGroupIdxAndOffsetInChunk(offset);
    const auto* nodeGroup = getNodeGroup(nodeGroupIdx);
    return nodeGroup->isVisible(startTS, transactionID, offsetInGroup);
}

bool NodeTable::isVisibleToLatestCommit(const Transaction* transaction, offset_t offset) const {
    return isVisible(Transaction::LATEST_COMMITTED_TS, transaction->getID(), offset);
}

std::string NodeTable::lockKeyOf(const ValueVector& pkVector, sel_t pos) {
    return pkVector.getAsValue(pos)->toString();
}

std::string NodeTable::lockKeyOfRow(Transaction* transaction, offset_t nodeOffset) const {
    const auto state = DataChunkState::getSingleValueDataChunkState();
    ValueVector nodeIDVector(LogicalType::INTERNAL_ID(), memoryManager, state);
    nodeIDVector.setValue<internalID_t>(0, internalID_t{nodeOffset, tableID});
    ValueVector pkVector(columns[pkColumnID]->getDataType().copy(), memoryManager, state);
    NodeTableScanState scanState{&nodeIDVector, {&pkVector}, state};
    scanState.setToTable(transaction, const_cast<NodeTable*>(this), {pkColumnID});
    initScanState(transaction, scanState, tableID, nodeOffset);
    if (!lookup(transaction, scanState) || pkVector.isNull(0)) {
        throw RuntimeException(stringFormat(
            "Row {} of table {} has no primary key to lock: it is not visible to this "
            "transaction.",
            nodeOffset, tableName));
    }
    return lockKeyOf(pkVector, 0);
}

static void throwCouldNotSerialize(table_id_t tableID, const std::string& key,
    const char* what) {
    throw TransactionManagerException(stringFormat(
        "Transaction aborted: {} — {} was {} by another transaction after this one started. "
        "Roll it back and run it again.",
        LockManager::COULD_NOT_SERIALIZE, LockResource::row(tableID, key).toString(), what));
}

void NodeTable::throwIfWrittenByAnotherCommitAfterSnapshot(const Transaction* transaction,
    offset_t nodeOffset, const std::string& key) const {
    auto [nodeGroupIdx, offsetInGroup] = StorageUtils::getNodeGroupIdxAndOffsetInChunk(nodeOffset);
    if (getNodeGroup(nodeGroupIdx)
            ->wasWrittenByCommitAfter(transaction->getStartTS(), transaction->getID(),
                offsetInGroup)) {
        throwCouldNotSerialize(tableID, key, "written");
    }
}

void NodeTable::throwIfDeletedByAnotherCommitAfterSnapshot(const Transaction* transaction,
    offset_t nodeOffset, const std::string& key) const {
    if (!isVisibleToLatestCommit(transaction, nodeOffset)) {
        throwCouldNotSerialize(tableID, key, "deleted");
    }
}

void NodeTable::lockRowForWrite(Transaction* transaction, offset_t nodeOffset,
    const std::string& key) const {
    transaction->acquireLock(LockResource::row(tableID, key), LockMode::EXCLUSIVE);
    throwIfWrittenByAnotherCommitAfterSnapshot(transaction, nodeOffset, key);
}

bool NodeTable::isVisibleNoLock(const Transaction* transaction, offset_t offset) const {
    auto [nodeGroupIdx, offsetInGroup] = StorageUtils::getNodeGroupIdxAndOffsetInChunk(offset);
    if (nodeGroupIdx >= nodeGroups->getNumNodeGroupsNoLock()) {
        return false;
    }
    const auto* nodeGroup = getNodeGroupNoLock(nodeGroupIdx);
    return nodeGroup->isVisibleNoLock(transaction, offsetInGroup);
}

bool NodeTable::lookupPK(const Transaction* transaction, ValueVector* keyVector, uint64_t vectorPos,
    offset_t& result) const {
    if (transaction->getLocalStorage()) {
        if (const auto localTable = transaction->getLocalStorage()->getLocalTable(tableID);
            localTable && localTable->cast<LocalNodeTable>().lookupPK(transaction, keyVector,
                              vectorPos, result)) {
            return true;
        }
    }
    return getPKIndex()->lookup(transaction, keyVector, vectorPos, result,
        [&](offset_t offset) { return isVisibleNoLock(transaction, offset); });
}

void NodeTable::scanIndexColumns(main::ClientContext* context, IndexScanHelper& scanHelper,
    const NodeGroupCollection& nodeGroups_, const LocalNodeTable* localTable) const {
    auto dataChunk = constructDataChunkForColumns(scanHelper.index->getIndexInfo().columnIDs);
    const auto scanState =
        scanHelper.initScanState(transaction::Transaction::Get(*context), dataChunk);
    if (localTable) {
        // Les groupes d'une table locale sont rangés par position de propriété.
        scanState->columnIDs = localTable->getLocalColumnIDs(scanState->committedColumnIDs);
    }

    const auto numNodeGroups = nodeGroups_.getNumNodeGroups();
    for (node_group_idx_t nodeGroupToScan = 0u; nodeGroupToScan < numNodeGroups;
         ++nodeGroupToScan) {
        scanState->nodeGroup = nodeGroups_.getNodeGroupNoLock(nodeGroupToScan);

        // It is possible for the node group to have no chunked groups if we are rolling back due to
        // an exception that is thrown before any chunked groups could be appended to the node group
        if (scanState->nodeGroup->getNumChunkedGroups() > 0) {
            scanState->nodeGroupIdx = nodeGroupToScan;
            KU_ASSERT(scanState->nodeGroup);
            scanState->nodeGroup->initializeScanState(transaction::Transaction::Get(*context),
                *scanState);
            while (true) {
                if (const auto scanResult = scanState->nodeGroup->scan(
                        transaction::Transaction::Get(*context), *scanState);
                    !scanHelper.processScanOutput(context, scanResult, scanState->outputVectors)) {
                    break;
                }
            }
        }
    }
}

void NodeTable::logInsertedRowsToWAL(main::ClientContext* context, offset_t startOffset,
    offset_t endOffset) {
    if (endOffset <= startOffset) {
        return;
    }
    const auto transaction = transaction::Transaction::Get(*context);
    // Les colonnes du journal sont les propriétés du catalogue, dans leur ordre : c'est la forme
    // d'une ligne insérée par le chemin ordinaire, celle que le rejeu attend. Les colonnes du
    // stockage ne sont pas les mêmes après un ALTER TABLE … DROP.
    const auto tableEntry =
        catalog::Catalog::Get(*context)->getTableCatalogEntry(transaction, tableID);
    std::vector<column_id_t> columnIDs;
    for (auto& property : tableEntry->getProperties()) {
        columnIDs.push_back(tableEntry->getColumnID(property.getName()));
    }
    auto dataChunk = constructDataChunkForColumns(columnIDs);
    std::vector<ValueVector*> vectors;
    for (auto& vector : dataChunk.valueVectors) {
        vectors.push_back(vector.get());
    }
    auto scanState = std::make_unique<NodeTableScanState>(nullptr, vectors, dataChunk.state);
    scanState->setToTable(transaction, this, columnIDs, {});
    scanState->source = TableScanSource::COMMITTED;
    // Le masque évite de lire les blocs hors de la plage ; il ne filtre pas les lignes d'un
    // bloc, c'est fait plus bas.
    const auto semiMask = SemiMaskUtil::createMask(endOffset);
    semiMask->maskRange(startOffset, endOffset);
    semiMask->enable();
    scanState->semiMask = semiMask.get();

    auto& wal = transaction->getLocalWAL();
    row_idx_t numLogged = 0;
    const auto firstGroup = StorageUtils::getNodeGroupIdx(startOffset);
    const auto lastGroup = StorageUtils::getNodeGroupIdx(endOffset - 1);
    for (auto nodeGroupIdx = firstGroup;
         nodeGroupIdx <= lastGroup && nodeGroupIdx < nodeGroups->getNumNodeGroups();
         nodeGroupIdx++) {
        scanState->nodeGroup = nodeGroups->getNodeGroupNoLock(nodeGroupIdx);
        if (scanState->nodeGroup->getNumChunkedGroups() == 0) {
            continue;
        }
        scanState->nodeGroupIdx = nodeGroupIdx;
        scanState->nodeGroup->initializeScanState(transaction, *scanState);
        const auto groupStartOffset = StorageUtils::getStartOffsetOfNodeGroup(nodeGroupIdx);
        auto& selVector = dataChunk.state->getSelVectorUnsafe();
        while (true) {
            // Le balayage attend une sélection neuve à chaque bloc : le filtre du masque la lit
            // comme l'identité, et on la réécrit plus bas.
            selVector.setToUnfiltered(DEFAULT_VECTOR_CAPACITY);
            const auto scanResult = scanState->nodeGroup->scan(transaction, *scanState);
            if (scanResult == NODE_GROUP_SCAN_EMPTY_RESULT) {
                break;
            }
            // Ne garder que les lignes de la plage : selon le bloc, le balayage rend plus que
            // ce que le masque demande.
            const auto numScanned = scanResult.numRows == 0 ? 0 : selVector.getSelSize();
            std::vector<sel_t> kept;
            kept.reserve(numScanned);
            for (sel_t i = 0; i < numScanned; i++) {
                const auto pos = selVector[i];
                const auto offset = groupStartOffset + scanResult.startRow + pos;
                if (offset >= startOffset && offset < endOffset) {
                    kept.push_back(pos);
                }
            }
            if (kept.empty()) {
                continue;
            }
            auto buffer = selVector.getMutableBuffer();
            for (size_t i = 0; i < kept.size(); i++) {
                buffer[i] = kept[i];
            }
            selVector.setToFiltered(kept.size());
            wal.logTableInsertion(tableID, TableType::NODE, kept.size(), vectors);
            numLogged += kept.size();
            if (transaction->journalExceedsCopyThreshold()) {
                // Le journal de la transaction, qui vit en mémoire jusqu'à sa validation,
                // dépasse le seuil : elle se replie sur le point de reprise forcé. Rien de ce
                // qui était au journal n'y reste, rien de plus n'y va.
                transaction->fallBackToForcedCheckpoint();
                return;
            }
        }
    }
    if (numLogged != endOffset - startOffset) {
        // Le message porte de quoi lire l'écart : la plage attendue, et ce que la table dit
        // d'elle-même.
        std::string groups;
        for (node_group_idx_t idx = 0; idx < nodeGroups->getNumNodeGroups() && idx < 4; idx++) {
            const auto* group = nodeGroups->getNodeGroupNoLock(idx);
            groups += stringFormat(" group {}: {} rows, {} chunked groups;", idx,
                group->getNumRows(), group->getNumChunkedGroups());
        }
        throw RuntimeException(stringFormat(
            "Bulk load of table {}: {} rows were written to the journal for {} rows added "
            "(offsets {} to {}; the table holds {} rows in {} groups;{}). The load is rolled "
            "back.",
            tableName, numLogged, endOffset - startOffset, startOffset, endOffset,
            nodeGroups->getNumTotalRows(), nodeGroups->getNumNodeGroups(), groups));
    }
}

void NodeTable::addIndex(std::unique_ptr<Index> index) {
    if (getIndex(index->getName()).has_value()) {
        throw RuntimeException("Index with name " + index->getName() + " already exists.");
    }
    // Un exemplaire détaché du même nom n'est plus un index de la table : il cède la place.
    std::erase_if(indexes, [&](const IndexHolder& holder) {
        return holder.isDetached() &&
               StringUtils::caseInsensitiveEquals(holder.getName(), index->getName());
    });
    indexes.push_back(IndexHolder{std::move(index)});
    hasChanges = true;
}

void NodeTable::dropIndex(const std::string& name) {
    // L'index peut ne pas être chargé (au rejeu du journal, son extension ne l'est pas
    // encore), ou ne pas être là du tout (créé puis retiré dans le même journal).
    for (auto it = indexes.begin(); it != indexes.end(); ++it) {
        if (StringUtils::caseInsensitiveEquals(it->getName(), name)) {
            indexes.erase(it);
            return;
        }
    }
}

std::optional<std::reference_wrapper<IndexHolder>> NodeTable::getIndexHolder(
    const std::string& name) {
    for (auto& index : indexes) {
        if (!index.isDetached() && StringUtils::caseInsensitiveEquals(index.getName(), name)) {
            return index;
        }
    }
    return std::nullopt;
}

std::optional<Index*> NodeTable::getIndex(const std::string& name) const {
    for (auto& index : indexes) {
        if (!index.isDetached() && StringUtils::caseInsensitiveEquals(index.getName(), name)) {
            if (index.isLoaded()) {
                return index.getIndex();
            }
            throw RuntimeException(stringFormat(
                "Index {} is not loaded yet. Please load the index before accessing it.", name));
        }
    }
    return std::nullopt;
}

void NodeTable::serialize(Serializer& serializer) const {
    nodeGroups->serialize(serializer);
    // Un index détaché n'est plus un index de la table : il n'est pas écrit.
    const auto numAttached = std::count_if(indexes.begin(), indexes.end(),
        [](const IndexHolder& holder) { return !holder.isDetached(); });
    serializer.write<uint64_t>(numAttached);
    for (auto i = 0u; i < indexes.size(); ++i) {
        if (!indexes[i].isDetached()) {
            indexes[i].serialize(serializer);
        }
    }
}

void NodeTable::deserialize(main::ClientContext* context, StorageManager* storageManager,
    Deserializer& deSer) {
    nodeGroups->deserialize(deSer, *memoryManager);
    std::vector<IndexInfo> indexInfos;
    std::vector<length_t> storageInfoBufferSizes;
    std::vector<std::unique_ptr<uint8_t[]>> storageInfoBuffers;
    uint64_t numIndexes = 0u;
    deSer.deserializeValue<uint64_t>(numIndexes);
    indexInfos.reserve(numIndexes);
    storageInfoBufferSizes.reserve(numIndexes);
    storageInfoBuffers.reserve(numIndexes);
    for (uint64_t i = 0; i < numIndexes; ++i) {
        IndexInfo indexInfo = IndexInfo::deserialize(deSer);
        indexInfos.push_back(indexInfo);
        uint64_t storageInfoSize = 0u;
        deSer.deserializeValue<uint64_t>(storageInfoSize);
        storageInfoBufferSizes.push_back(storageInfoSize);
        auto storageInfoBuffer = std::make_unique<uint8_t[]>(storageInfoSize);
        deSer.read(storageInfoBuffer.get(), storageInfoSize);
        storageInfoBuffers.push_back(std::move(storageInfoBuffer));
    }
    indexes.clear();
    indexes.reserve(indexInfos.size());
    for (auto i = 0u; i < indexInfos.size(); ++i) {
        indexes.push_back(IndexHolder(indexInfos[i], std::move(storageInfoBuffers[i]),
            storageInfoBufferSizes[i]));
        if (indexInfos[i].isBuiltin) {
            indexes[i].load(context, storageManager);
        }
    }
    // Les numéros lus sur disque peuvent être périmés : StorageManager::deserialize les
    // recalcule ensuite (renumberColumns), avec le catalogue de cette base.
}

void NodeTable::renumberColumns(const catalog::Catalog& catalog,
    const NodeTableCatalogEntry& tableEntry) {
    pkColumnID = tableEntry.getColumnID(tableEntry.getPrimaryKeyName());
    const auto indexEntries = catalog.getIndexEntries(&DUMMY_CHECKPOINT_TRANSACTION, tableID);
    for (auto& index : indexes) {
        std::vector<column_id_t> columnIDs;
        if (index.getIndexInfo().isPrimary) {
            columnIDs = {pkColumnID};
        } else {
            const auto found = std::find_if(indexEntries.begin(), indexEntries.end(),
                [&](const auto* indexEntry) { return indexEntry->getIndexName() == index.getName(); });
            if (found == indexEntries.end()) {
                // Aucun cas connu (DROP_VECTOR_INDEX retire l'index de la table et du catalogue
                // ensemble) ; on ne fait pas échouer l'ouverture pour lui, mais l'écart se voit.
                fprintf(stderr,
                    "[rag3db] index %s of table %s has no catalog entry; its columns are left "
                    "as stored\n",
                    index.getName().c_str(), tableName.c_str());
                continue;
            }
            for (const auto propertyID : (*found)->getPropertyIDs()) {
                // getColumnID(idx_t) prend un identifiant de propriété (pas une position).
                columnIDs.push_back(tableEntry.getColumnID(common::idx_t{propertyID}));
            }
        }
        if (index.setColumnIDs(std::move(columnIDs))) {
            hasChanges = true;
        }
    }
}

} // namespace storage
} // namespace rag3db
