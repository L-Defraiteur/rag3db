#include "storage/table/node_table.h"

#include "storage/storage_version_info.h"

#include <algorithm>
#include <array>
#include <cstdio>

#include "catalog/catalog.h"
#include "catalog/catalog_entry/index_catalog_entry.h"
#include "catalog/catalog_entry/node_table_catalog_entry.h"
#include "common/cast.h"
#include "common/data_chunk/data_chunk_state.h"
#include "common/exception/duplicated_primary_key.h"
#include "common/exception/message.h"
#include "common/exception/runtime.h"
#include "common/exception/transaction_manager.h"
#include "common/string_format.h"
#include "common/types/types.h"
#include "main/client_context.h"
#include "storage/local_storage/local_node_table.h"
#include "storage/local_storage/local_storage.h"
#include "storage/local_storage/local_table.h"
#include "storage/page_manager.h"
#include "storage/storage_manager.h"
#include "storage/wal/local_wal.h"
#include "transaction/lock_manager.h"
#include "transaction/transaction.h"
#include "transaction/transaction_manager.h"

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
    table->rollbackIndexInsert(StorageUtils::getStartOffsetOfNodeGroup(nodeGroupIdx) + startRow,
        transaction::Transaction::Get(*context));

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

// Les types par numéro de colonne, trous compris (ANY pour une colonne supprimée) : la forme des
// vecteurs du stockage, à la différence de LocalNodeTable::getNodeTableColumnTypes (par position).
std::vector<LogicalType> NodeTable::getColumnTypesBySlot(const TableCatalogEntry& entry) {
    std::vector<LogicalType> types;
    types.reserve(entry.getNumColumnSlots());
    for (auto i = 0u; i < entry.getNumColumnSlots(); i++) {
        types.push_back(LogicalType::ANY());
    }
    for (auto& property : entry.getProperties()) {
        types[entry.getColumnID(property.getName())] = property.getType().copy();
    }
    return types;
}

NodeTable::NodeTable(const StorageManager* storageManager,
    const NodeTableCatalogEntry* nodeTableEntry, MemoryManager* mm)
    : Table{nodeTableEntry, storageManager, mm},
      pkColumnID{nodeTableEntry->getColumnID(nodeTableEntry->getPrimaryKeyName())},
      versionRecordHandler(this) {
    auto* dataFH = storageManager->getDataFH();
    auto& pageAllocator = *dataFH->getPageManager();
    columns.resize(nodeTableEntry->getNumColumnSlots());
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
    // Chaque holder a son identifiant, unique dans la table : la validation et l'annulation d'un
    // CREATE ou d'un DROP le retrouvent par lui (un identifiant partagé retirait la clé primaire).
    indexes.back().setHolderID(nextHolderID++);
    nodeGroups = std::make_unique<NodeGroupCollection>(*mm,
        getColumnTypesBySlot(*nodeTableEntry), enableCompression,
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
        nodeScanState.nodeGroupKeepAlive = nodeGroups->getNodeGroupShared(nodeScanState.nodeGroupIdx);
        nodeGroup = nodeScanState.nodeGroupKeepAlive.get();
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

// Le balayage sous un petit masque (page coeur-cpp/12 §10) : les décalages de la tranche sont
// relus un à un, comme lookupMultiple, mais la sélection de sortie ne garde que les lignes
// trouvées (une ligne supprimée pour cette transaction n'y est pas) : le probe reçoit les lignes
// masquées, et elles seules, au lieu de chaque fenêtre de 2 048 lignes qui en contient une.
void NodeTable::lookupOffsets(Transaction* transaction, NodeTableScanState& scanState) const {
    scanState.resetOutVectors();
    const auto numOffsets = scanState.maskedOffsets.size();
    auto& selVector = scanState.outState->getSelVectorUnsafe();
    selVector.setToUnfiltered(numOffsets);
    for (auto i = 0u; i < numOffsets; i++) {
        scanState.nodeIDVector->setNull(i, false);
        scanState.nodeIDVector->setValue<internalID_t>(i,
            internalID_t{scanState.maskedOffsets[i], tableID});
    }
    auto found = selVector.getMutableBuffer();
    sel_t numFound = 0;
    for (auto i = 0u; i < numOffsets; i++) {
        const auto nodeOffset = scanState.maskedOffsets[i];
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
        if (scanState.source != source || scanState.nodeGroupIdx != nodeGroupIdx ||
            scanState.nodeGroup == nullptr) {
            // Un décalage masqué sans groupe (le masqueur a pu retenir un décalage au-delà des
            // lignes de la table — un index qui garde des nœuds disparus, une ligne locale
            // d'une autre table locale — que le balayage par fenêtres ne lisait pas non plus) :
            // la ligne n'existe pas, on passe.
            NodeGroup* nodeGroup = nullptr;
            if (isUnCommitted) {
                const auto localTable = transaction->getLocalStorage()->getLocalTable(tableID);
                if (localTable) {
                    const auto& localNodeTable = localTable->cast<LocalNodeTable>();
                    if (nodeGroupIdx < localNodeTable.getNumNodeGroups()) {
                        nodeGroup = localNodeTable.getNodeGroup(nodeGroupIdx);
                    }
                }
            } else {
                nodeGroup = nodeGroups->getNodeGroup(nodeGroupIdx, true /* mayOutOfBound */);
            }
            if (nodeGroup == nullptr) {
                scanState.nodeGroup = nullptr;
                continue;
            }
            scanState.source = source;
            scanState.nodeGroupIdx = nodeGroupIdx;
            initScanState(transaction, scanState);
        }
        scanState.rowIdxVector->setValue<row_idx_t>(i, rowIdxInGroup);
        if (scanState.nodeGroup->lookupSharedLock(transaction, scanState, i)) {
            found[numFound++] = i;
        }
    }
    if (numFound != numOffsets) {
        selVector.setToFiltered(numFound);
    }
}

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
        throw DuplicatedPrimaryKeyException(pkVector->getAsValue(selVector[0])->toString());
    }
}

bool NodeTable::isWritableIndex(IndexHolder& indexHolder, const Transaction* transaction) {
    if (indexHolder.isDetached() || !isIndexVisible(indexHolder, transaction)) {
        return false;
    }
    if (indexHolder.isLoaded()) {
        // Pas encore bâti (NOTED), ou à refaire (INCOMPLETE) : l'écrivain n'y touche pas. L'état
        // est lu au moment d'écrire, sous le verrou d'index (page 07, §3 bis.1).
        return indexHolder.receivesWrites();
    }
    if (transaction->isRecovery()) {
        indexHolder.detach();
        hasChanges = true;
        return false;
    }
    throw RuntimeException(stringFormat("Index {} {} {}.", indexHolder.getName(),
        INDEX_NOT_LOADED_FOR_WRITE, tableName));
}

IndexPropertyPositions NodeTable::getIndexPropertyPositions(
    const TableCatalogEntry& tableEntry) const {
    std::shared_lock lck{indexesMtx};
    IndexPropertyPositions positions;
    for (const auto& holder : indexes) {
        auto& ofHolder = positions[holder.getHolderID()];
        for (const auto columnID : holder.getIndexInfo().columnIDs) {
            ofHolder.push_back(tableEntry.getPropertyPosition(columnID));
        }
    }
    return positions;
}

void NodeTable::initInsertState(main::ClientContext* context, TableInsertState& insertState) {
    auto& nodeInsertState = insertState.cast<NodeTableInsertState>();
    const auto transaction = transaction::Transaction::Get(*context);
    // La liste se lit sous son verrou ; les états se créent hors de lui (un index peut relire sa
    // table).
    std::vector<std::pair<uint64_t, std::shared_ptr<Index>>> writable;
    {
        std::shared_lock lck{indexesMtx};
        for (auto& holder : indexes) {
            if (isWritableIndex(holder, transaction)) {
                writable.emplace_back(holder.getHolderID(), holder.getSharedIndex());
            }
        }
    }
    const auto positionsKnown = std::ranges::all_of(writable, [&](const auto& entry) {
        return nodeInsertState.indexPropertyPositions.contains(entry.first);
    });
    if (!positionsKnown) {
        // Pas posées par l'appelant (le rejeu), ou posées avant qu'un index n'apparaisse : une
        // recherche au catalogue.
        nodeInsertState.indexPropertyPositions = getIndexPropertyPositions(
            *catalog::Catalog::Get(*context)->getTableCatalogEntry(transaction, tableID));
    }
    nodeInsertState.indexInsertStates.clear();
    for (auto& [holderID, index] : writable) {
        auto state = index->initInsertState(context, [this, context](offset_t offset) {
            return isVisible(transaction::Transaction::Get(*context), offset);
        });
        nodeInsertState.indexInsertStates.push_back({holderID, std::move(index), std::move(state)});
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
        // Marche I1 : sur une table indexée, l'index en exclusif (lockIndexForWrite), sinon en
        // partagé ; la clé en exclusif dans les deux cas.
        const auto indexed = hasLoadedSecondaryIndex(transaction);
        const auto pkPos = nodeInsertState.pkVector.state->getSelVector()[0];
        const std::array requests{
            LockRequest{LockResource::index(tableID),
                indexed ? LockMode::EXCLUSIVE : LockMode::SHARED},
            LockRequest{LockResource::row(tableID, lockKeyOf(nodeInsertState.pkVector, pkPos)),
                LockMode::EXCLUSIVE}};
        transaction->acquireLocks(requests);
        if (indexed && !transaction->hasWritten()) {
            TransactionManager::Get(*transaction->getClientContext())
                ->refreshSnapshot(transaction);
        }
    }
    // Au rejeu du journal, l'unicité a été contrôlée quand le journal a été écrit ; un journal
    // du moteur d'avant le verrou de clé peut pourtant porter un doublon validé : la ligne est
    // rejouée (elle garde son décalage, dont dépendent les enregistrements qui suivent), et
    // c'est l'index de clé, au commit, qui note le doublon ; la reprise le règle à sa fin
    // (WALReplayer::settleReplayedDuplicates, marche A3′ reprise).
    if (!transaction->isRecovery()) {
        validatePkNotExists(transaction, const_cast<ValueVector*>(&nodeInsertState.pkVector));
    }
    localTable->insert(transaction, insertState);
    for (const auto& [holderID, index, state] : nodeInsertState.indexInsertStates) {
        std::vector<ValueVector*> indexedPropertyVectors;
        for (const auto position : nodeInsertState.indexPropertyPositions.at(holderID)) {
            indexedPropertyVectors.push_back(insertState.propertyVectors[position]);
        }
        index->insert(transaction, nodeInsertState.nodeIDVector, indexedPropertyVectors, *state);
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
    const auto transaction = transaction::Transaction::Get(*context);
    // La liste se lit sous son verrou ; les états se créent hors de lui.
    std::vector<std::pair<uint64_t, std::shared_ptr<Index>>> toUpdate;
    {
        std::shared_lock lck{indexesMtx};
        for (auto& indexHolder : indexes) {
            // Un index que la transaction ne voit pas (retiré avant son instantané, ou créé par
            // une autre sans validation) n'est pas mis à jour (ticket de la rétention).
            if (!isIndexVisible(indexHolder, transaction)) {
                continue;
            }
            if (!indexHolder.isLoaded()) {
                // Un index non chargé ne répond pas ; sa description suffit à dire s'il porte
                // sur la colonne. S'il n'y porte pas, la mise à jour ne le regarde pas.
                const auto& columnIDs = indexHolder.getIndexInfo().columnIDs;
                if (std::find(columnIDs.begin(), columnIDs.end(), nodeUpdateState.columnID) !=
                    columnIDs.end()) {
                    const_cast<NodeTable*>(this)->isWritableIndex(
                        const_cast<IndexHolder&>(indexHolder), transaction);
                }
                continue;
            }
            if (!indexHolder.receivesWrites()) {
                continue;
            }
            auto index = indexHolder.getSharedIndex();
            if (index->isPrimary() || !index->isBuiltOnColumn(nodeUpdateState.columnID)) {
                continue;
            }
            toUpdate.emplace_back(indexHolder.getHolderID(), std::move(index));
        }
    }
    nodeUpdateState.indexUpdateState.clear();
    for (auto& [holderID, index] : toUpdate) {
        auto state = index->initUpdateState(context, nodeUpdateState.columnID,
            [this, context](offset_t offset) {
                return isVisible(transaction::Transaction::Get(*context), offset);
            });
        nodeUpdateState.indexUpdateState.push_back({holderID, std::move(index), std::move(state)});
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
        lockIndexForWrite(transaction);
        lockRowForWrite(transaction, nodeOffset, lockKeyOfRow(transaction, nodeOffset));
    }
    for (const auto& [holderID, index, state] : nodeUpdateState.indexUpdateState) {
        if (isLocalRow && index->needCommitInsert()) {
            continue;
        }
        index->update(transaction, nodeUpdateState.nodeIDVector, nodeUpdateState.propertyVector,
            *state);
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
    // La liste se lit sous son verrou ; les états se créent hors de lui.
    std::vector<std::pair<uint64_t, std::shared_ptr<Index>>> writable;
    {
        std::shared_lock lck{indexesMtx};
        for (auto& holder : indexes) {
            if (isWritableIndex(holder, transaction)) {
                writable.emplace_back(holder.getHolderID(), holder.getSharedIndex());
            }
        }
    }
    nodeDeleteState.indexDeleteStates.clear();
    for (auto& [holderID, index] : writable) {
        auto state = index->initDeleteState(transaction, memoryManager, getVisibleFunc(transaction));
        nodeDeleteState.indexDeleteStates.push_back({holderID, std::move(index), std::move(state)});
    }
    nodeDeleteState.indexDeleteStatesInitialized = true;
}

void NodeTable::finalizeDelete(Transaction* transaction, TableDeleteState& deleteState) {
    auto& nodeDeleteState = ku_dynamic_cast<NodeTableDeleteState&>(deleteState);
    if (!nodeDeleteState.indexDeleteStatesInitialized) {
        return;
    }
    for (const auto& [holderID, index, state] : nodeDeleteState.indexDeleteStates) {
        index->finalizeDelete(transaction, *state);
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
        lockIndexForWrite(transaction);
        lockRowForWrite(transaction, nodeOffset,
            lockKeyOf(nodeDeleteState.pkVector,
                nodeDeleteState.pkVector.state->getSelVector()[0]));
    }
    // Use pre-created index delete states (initialized once, reused across calls).
    initDeleteStates(transaction, deleteState);
    for (const auto& [holderID, index, state] : nodeDeleteState.indexDeleteStates) {
        index->delete_(transaction, nodeDeleteState.nodeIDVector, *state);
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
    transaction->pushAddColumn(*this);
}

void NodeTable::rollbackAddColumn(main::ClientContext* context) {
    KU_ASSERT(!columns.empty());
    // Le stockage local de la transaction a reçu la colonne aussi ; il est jeté juste après.
    columns.pop_back();
    nodeGroups->removeLastColumn(PageManager::Get(*context));
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

    // 3. Scan index columns for newly inserted tuples. La liste se lit sous son verrou (le
    // CREATE d'une autre connexion peut l'allonger pendant cette validation) ; les index
    // reçoivent leurs lignes hors de lui.
    std::vector<std::shared_ptr<Index>> toInsertInto;
    {
        std::shared_lock lck{indexesMtx};
        for (auto& index : indexes) {
            // Un index que la transaction a retirée, ou qu'une autre crée sans avoir validé, ne
            // reçoit pas ses lignes (ticket de la rétention).
            if (!index.isDetached() && !isIndexVisible(index, transaction)) {
                continue;
            }
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
            if (index.needCommitInsert() && index.receivesWrites()) {
                toInsertInto.push_back(index.getSharedIndex());
            }
        }
    }
    for (const auto& index : toInsertInto) {
        // L'index de clé primaire contrôle l'unicité avec la visibilité de l'insertion : un
        // contrôle qui différerait ici ferait échouer la validation après l'ajout des lignes
        // aux groupes (étape 1), une transaction à moitié validée (C7 du banc, 10 octobre).
        UncommittedIndexInserter indexInserter{startNodeOffset, this, index.get(),
            index->isPrimary() ? getUniquenessVisibleFunc(transaction) :
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
        // Aucune transaction n'est ouverte : un index retiré et validé n'est plus visible de
        // personne, on l'efface (comme reclaimDroppedTables pour les tables).
        reclaimDroppedIndexes();
        // Deleted columns are vacuumed and not checkpointed. Les colonnes ne quittent la table
        // qu'après le point de reprise des groupes : un échec avant leur premier échange laisse la
        // table comme elle était, et le point de reprise refusé ne condamne pas la base (ticket du
        // ROLLBACK sous un tampon plein, défaut 3).
        std::vector<column_id_t> columnIDs;
        std::vector<Column*> checkpointColumnPtrs;
        for (auto& property : tableEntry->getProperties()) {
            auto columnID = tableEntry->getColumnID(property.getName());
            columnIDs.push_back(columnID);
            checkpointColumnPtrs.push_back(columns[columnID].get());
        }

        auto* storageManager = StorageManager::Get(*context);
        NodeGroupCheckpointState state{columnIDs, std::move(checkpointColumnPtrs), pageAllocator,
            memoryManager};
        state.changedMemory = storageManager->getCheckpointChangedMemoryFlag();
        state.checkpointTS = storageManager->getCheckpointTS();
        state.oldestReaderTS = storageManager->getOldestReaderTS();
        nodeGroups->checkpoint(*memoryManager, state);
        storageManager->markCheckpointChangedMemory();
        // Les numéros ne bougent pas : une colonne supprimée laisse son numéro vide (un trou),
        // les autres restent où elles sont. Plus de renumérotation du catalogue.
        for (auto columnID = 0u; columnID < columns.size(); columnID++) {
            if (std::find(columnIDs.begin(), columnIDs.end(), columnID) == columnIDs.end()) {
                columns[columnID].reset();
            }
        }
        for (auto& index : indexes) {
            index.checkpoint(context, pageAllocator);
        }
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
void NodeTable::rollbackIndexInsert(offset_t firstRolledBackOffset,
    const Transaction* transaction) {
    // Sous le verrou de la liste : le CREATE d'une autre connexion peut l'allonger pendant cette
    // annulation.
    std::shared_lock lck{indexesMtx};
    for (auto& index : indexes) {
        // Les seuls index que la transaction qui annule voyait et écrivait : pas un index retiré
        // avant elle, ni celui qu'une autre transaction crée sans avoir validé (le même filtre
        // que l'étape 3 de la validation).
        if (index.isLoaded() && !index.isDetached() && isIndexVisible(index, transaction)) {
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

bool NodeTable::wasColumnUpdatedByCommitAfter(column_id_t columnID, transaction_t startTS,
    offset_t offset) const {
    auto [nodeGroupIdx, offsetInGroup] = StorageUtils::getNodeGroupIdxAndOffsetInChunk(offset);
    const auto* nodeGroup = getNodeGroup(nodeGroupIdx);
    return nodeGroup->wasColumnUpdatedByCommitAfter(columnID, startTS, offsetInGroup);
}

bool NodeTable::isInserted(const Transaction* transaction, offset_t offset) const {
    auto [nodeGroupIdx, offsetInGroup] = StorageUtils::getNodeGroupIdxAndOffsetInChunk(offset);
    return getNodeGroup(nodeGroupIdx)->isInserted(transaction, offsetInGroup);
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
        // Sur une table à index secondaire, lockIndexForWrite a rafraîchi l'instantané avant
        // cette lecture : une ligne que le MATCH voyait et qu'un autre écrivain a supprimée et
        // validée depuis n'est plus visible. C'est l'erreur de sérialisation nommée (A4′), à
        // rejouer — pas une erreur d'exécution (H4 du banc, 10 fois sur 10 sous le défaut).
        if (!isVisibleToLatestCommit(transaction, nodeOffset)) {
            throw TransactionManagerException(stringFormat(
                "Transaction aborted: {} — row {} of table {} was deleted by another "
                "transaction after this one started. Roll it back and run it again.",
                LockManager::COULD_NOT_SERIALIZE, nodeOffset, tableName));
        }
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

bool NodeTable::hasLoadedSecondaryIndex(const Transaction* transaction) const {
    std::shared_lock lck{indexesMtx};
    return std::any_of(indexes.begin(), indexes.end(), [&](const IndexHolder& holder) {
        return holder.isLoaded() && !holder.getIndexInfo().isPrimary &&
               isIndexVisible(holder, transaction);
    });
}

void NodeTable::lockIndexForWrite(Transaction* transaction) const {
    // Sans index secondaire, aucun verrou d'index : le prendre en partagé ferait attendre un
    // écrivain derrière un CREATE d'index, que son point de reprise forcé attendrait à son tour
    // (ticket 2026-10-12-les-lignes-validees-pendant-le-bati-d-un-create-manquent-a-l-index).
    if (!transaction->usesLocks() || !hasLoadedSecondaryIndex(transaction)) {
        return;
    }
    transaction->acquireLock(LockResource::index(tableID), LockMode::EXCLUSIVE);
    if (!transaction->hasWritten()) {
        TransactionManager::Get(*transaction->getClientContext())->refreshSnapshot(transaction);
    }
}

void NodeTable::lockIndexForDDL(Transaction* transaction) const {
    if (!transaction->usesLocks()) {
        return;
    }
    transaction->acquireLock(LockResource::index(tableID), LockMode::EXCLUSIVE);
    if (!transaction->hasWritten()) {
        TransactionManager::Get(*transaction->getClientContext())->refreshSnapshot(transaction);
    }
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

bool NodeTable::isIndexVisible(const IndexHolder& holder, const Transaction* transaction) {
    if (transaction == nullptr) {
        return !holder.isDropped();
    }
    return holder.isVisibleTo(transaction->getID(), transaction->getStartTS());
}

void NodeTable::addIndex(std::unique_ptr<Index> index, Transaction* transaction) {
    addIndexInState(std::move(index), transaction, IndexBuildState::READY, {});
}

void NodeTable::addNotedIndex(std::unique_ptr<Index> emptyIndex, Transaction* transaction,
    std::weak_ptr<void> builderToken) {
    const auto name = emptyIndex->getName();
    addIndexInState(std::move(emptyIndex), transaction, IndexBuildState::NOTED,
        std::move(builderToken));
    logIndexBuildState(transaction, name, IndexBuildState::NOTED);
}

void NodeTable::logIndexBuildState(Transaction* transaction, const std::string& name,
    IndexBuildState state) const {
    // Une transaction forcée (T_B) a jeté son journal : son état est durable par le point de
    // reprise qui la suit.
    if (transaction != nullptr && transaction->shouldLogToWAL()) {
        transaction->getLocalWAL().logIndexStateRecord(tableID, name,
            static_cast<uint8_t>(state));
    }
}

Index* NodeTable::completeNotedIndex(const std::string& name, std::unique_ptr<Index> built,
    Transaction* transaction) {
    std::unique_lock lck{indexesMtx};
    for (auto& holder : indexes) {
        if (holder.isDetached() || !isIndexVisible(holder, transaction) ||
            !StringUtils::caseInsensitiveEquals(holder.getName(), name)) {
            continue;
        }
        const auto state = holder.buildStateFor(transaction->getID(), transaction->getStartTS());
        if (state != IndexBuildState::NOTED) {
            throw RuntimeException(stringFormat("Index {} of table {} cannot be built: it is {}.",
                name, tableName, indexBuildStateToString(state)));
        }
        holder.replaceIndex(std::move(built));
        holder.markBuildState(transaction->getID(), IndexBuildState::READY);
        hasChanges = true;
        transaction->pushIndexChange(*this, holder.getHolderID(), IndexChangeKind::BUILD_STATE);
        logIndexBuildState(transaction, name, IndexBuildState::READY);
        return holder.getIndex();
    }
    throw RuntimeException(stringFormat(
        "Index {} of table {} cannot be built: its entry was not created.", name, tableName));
}

void NodeTable::setIndexBuildStateForRecovery(const std::string& name, IndexBuildState state) {
    std::unique_lock lck{indexesMtx};
    for (auto& holder : indexes) {
        if (!holder.isDetached() && !holder.isDropped() &&
            StringUtils::caseInsensitiveEquals(holder.getName(), name)) {
            holder.setBuildState(state);
            hasChanges = true;
            return;
        }
    }
}

std::optional<IndexBuildState> NodeTable::getIndexBuildState(const std::string& name,
    const Transaction* transaction) const {
    std::shared_lock lck{indexesMtx};
    for (const auto& holder : indexes) {
        if (holder.isDetached() || !isIndexVisible(holder, transaction) ||
            !StringUtils::caseInsensitiveEquals(holder.getName(), name)) {
            continue;
        }
        return transaction == nullptr ?
                   holder.committedBuildState() :
                   holder.buildStateFor(transaction->getID(), transaction->getStartTS());
    }
    return std::nullopt;
}

void NodeTable::addIndexInState(std::unique_ptr<Index> index, Transaction* transaction,
    IndexBuildState state, std::weak_ptr<void> builderToken) {
    std::unique_lock lck{indexesMtx};
    // Un exemplaire vivant du même nom refuse ; un exemplaire retiré ne compte plus. Un exemplaire
    // retiré par une autre transaction non validée ne peut pas se trouver ici : un DROP tient
    // l'index de la table en exclusif jusqu'à sa validation, et un CREATE n'a lieu qu'en
    // transaction automatique, jamais après un DROP du même nom dans la même transaction.
    // Tout exemplaire non retiré de ce nom, validé ou non, d'où qu'il vienne : un CREATE non
    // validé d'une autre connexion bloque encore le même nom.
    if (getIndexNoLock(index->getName(), nullptr /*transaction*/).has_value()) {
        throw RuntimeException("Index with name " + index->getName() + " already exists.");
    }
    // Un exemplaire détaché du même nom n'est plus un index de la table : il cède la place.
    std::erase_if(indexes, [&](const IndexHolder& holder) {
        return holder.isDetached() &&
               StringUtils::caseInsensitiveEquals(holder.getName(), index->getName());
    });
    IndexHolder holder{std::move(index)};
    holder.setHolderID(nextHolderID++);
    holder.initBuildState(state, std::move(builderToken));
    const auto versioned = transaction != nullptr && !transaction->isRecovery();
    if (versioned) {
        holder.markCreatedBy(transaction->getID());
    }
    const auto holderID = holder.getHolderID();
    indexes.push_back(std::move(holder));
    hasChanges = true;
    if (versioned) {
        transaction->pushIndexChange(*this, holderID, IndexChangeKind::CREATE);
    }
}

void NodeTable::dropIndex(const std::string& name, Transaction* transaction) {
    std::unique_lock lck{indexesMtx};
    if (transaction == nullptr || transaction->isRecovery()) {
        // Au rejeu du journal : immédiat. L'index peut ne pas être chargé (son extension ne l'est
        // pas encore), ou ne pas être là du tout (créé puis retiré dans le même journal).
        for (auto it = indexes.begin(); it != indexes.end(); ++it) {
            if (!it->isDropped() && StringUtils::caseInsensitiveEquals(it->getName(), name)) {
                indexes.erase(it);
                return;
            }
        }
        return;
    }
    // Sous une transaction : marqué, pas effacé. Les transactions d'avant le voient encore ; le
    // point de reprise l'efface, l'annulation le rend.
    for (auto& holder : indexes) {
        if (isIndexVisible(holder, transaction) &&
            StringUtils::caseInsensitiveEquals(holder.getName(), name)) {
            holder.markDroppedBy(transaction->getID());
            hasChanges = true;
            transaction->pushIndexChange(*this, holder.getHolderID(), IndexChangeKind::DROP);
            return;
        }
    }
}

void NodeTable::commitIndexChange(uint64_t holderID, IndexChangeKind kind,
    transaction_t commitTS) {
    std::unique_lock lck{indexesMtx};
    for (auto& holder : indexes) {
        if (holder.getHolderID() == holderID) {
            switch (kind) {
            case IndexChangeKind::CREATE:
                holder.commitCreate(commitTS);
                break;
            case IndexChangeKind::DROP:
                holder.commitDrop(commitTS);
                break;
            case IndexChangeKind::BUILD_STATE:
                holder.commitBuildState(commitTS);
                break;
            }
            return;
        }
    }
}

void NodeTable::rollbackIndexChange(uint64_t holderID, IndexChangeKind kind) {
    std::unique_lock lck{indexesMtx};
    for (auto it = indexes.begin(); it != indexes.end(); ++it) {
        if (it->getHolderID() != holderID) {
            continue;
        }
        switch (kind) {
        case IndexChangeKind::CREATE:
            // L'index d'une recherche en cours reste vivant par son shared_ptr, celui d'un
            // écrivain aussi : ses états portent l'index et son holderID (IndexWriteState), lus
            // une fois sous ce verrou, et ne dépendent pas de la position dans la liste.
            indexes.erase(it);
            break;
        case IndexChangeKind::DROP:
            it->unmarkDropped();
            break;
        case IndexChangeKind::BUILD_STATE:
            it->rollbackBuildState();
            break;
        }
        return;
    }
}

void NodeTable::reclaimDroppedIndexes() {
    std::unique_lock lck{indexesMtx};
    std::erase_if(indexes, [](const IndexHolder& holder) { return holder.isDropCommitted(); });
}

std::optional<Index*> NodeTable::getIndex(const std::string& name,
    const Transaction* transaction) const {
    std::shared_lock lck{indexesMtx};
    return getIndexNoLock(name, transaction);
}

std::vector<std::shared_ptr<Index>> NodeTable::captureVisibleIndexes(const std::string& name,
    const Transaction* transaction) const {
    std::shared_lock lck{indexesMtx};
    std::vector<std::shared_ptr<Index>> visible;
    for (auto& index : indexes) {
        if (index.isDetached() || !isIndexVisible(index, transaction) ||
            !StringUtils::caseInsensitiveEquals(index.getName(), name)) {
            continue;
        }
        if (!index.isLoaded()) {
            throw RuntimeException(stringFormat(
                "Index {} is not loaded yet. Please load the index before accessing it.", name));
        }
        // L'état vu à l'instantané de la recherche : un index en construction se refuse par son
        // nom, jamais pris pour un index absent ; un index incomplet dit qu'il est à refaire.
        const auto state = transaction == nullptr ?
                               index.committedBuildState() :
                               index.buildStateFor(transaction->getID(), transaction->getStartTS());
        if (state == IndexBuildState::INCOMPLETE) {
            throw RuntimeException(stringFormat(
                "Index {} of table {} {}: its build did not finish. Rebuild it: drop it, then "
                "create it again.",
                name, tableName, INDEX_INCOMPLETE));
        }
        if (state != IndexBuildState::READY) {
            throw RuntimeException(stringFormat(
                "Index {} of table {} {} (state {}): a CREATE is building it. Run the query again "
                "once the CREATE is done.",
                name, tableName, INDEX_UNDER_CONSTRUCTION, indexBuildStateToString(state)));
        }
        visible.push_back(index.getSharedIndex());
    }
    return visible;
}

bool NodeTable::hasIndexHolder(const std::string& name, const Transaction* transaction) const {
    std::shared_lock lck{indexesMtx};
    return std::any_of(indexes.begin(), indexes.end(), [&](const IndexHolder& holder) {
        return !holder.isDetached() && isIndexVisible(holder, transaction) &&
               StringUtils::caseInsensitiveEquals(holder.getName(), name);
    });
}

bool NodeTable::loadIndex(const std::string& name, main::ClientContext* context,
    StorageManager* storageManager) {
    std::unique_lock lck{indexesMtx};
    for (auto& holder : indexes) {
        if (!holder.isDetached() && !holder.isDropped() &&
            StringUtils::caseInsensitiveEquals(holder.getName(), name)) {
            if (!holder.isLoaded()) {
                holder.load(context, storageManager);
            }
            return true;
        }
    }
    return false;
}

std::optional<Index*> NodeTable::getIndexNoLock(const std::string& name,
    const Transaction* transaction) const {
    for (auto& index : indexes) {
        if (!index.isDetached() && isIndexVisible(index, transaction) &&
            StringUtils::caseInsensitiveEquals(index.getName(), name)) {
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
    // Un index détaché, ou retiré, n'est plus un index de la table : il n'est pas écrit.
    const auto written = [](const IndexHolder& holder) {
        return !holder.isDetached() && !holder.isDropped();
    };
    const auto numAttached = std::count_if(indexes.begin(), indexes.end(), written);
    serializer.write<uint64_t>(numAttached);
    for (auto i = 0u; i < indexes.size(); ++i) {
        if (written(indexes[i])) {
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
    std::vector<IndexBuildState> buildStates;
    // Avant le format 43, un holder ne portait pas d'état : il était prêt.
    const auto version = deSer.getStorageVersion();
    const auto readsBuildState =
        version == 0 || version > StorageVersionInfo::VERSION_BEFORE_INDEX_BUILD_STATE;
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
        uint8_t buildState = static_cast<uint8_t>(IndexBuildState::READY);
        if (readsBuildState) {
            deSer.deserializeValue<uint8_t>(buildState);
        }
        buildStates.push_back(static_cast<IndexBuildState>(buildState));
    }
    indexes.clear();
    indexes.reserve(indexInfos.size());
    for (auto i = 0u; i < indexInfos.size(); ++i) {
        indexes.push_back(IndexHolder(indexInfos[i], std::move(storageInfoBuffers[i]),
            storageInfoBufferSizes[i]));
        indexes.back().setHolderID(nextHolderID++);
        // Sans jeton : un état autre que READY se lit INCOMPLETE (IndexHolder::withBuilder).
        indexes.back().setBuildState(buildStates[i]);
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
