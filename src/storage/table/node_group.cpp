#include "storage/table/node_group.h"

#include <algorithm>

#include <mutex>
#include <shared_mutex>

#include "common/assert.h"
#include "common/types/types.h"
#include "common/uniq_lock.h"
#include "storage/buffer_manager/memory_manager.h"
#include "storage/enums/residency_state.h"
#include "storage/storage_utils.h"
#include "storage/table/chunked_node_group.h"
#include "storage/table/column_chunk.h"
#include "storage/table/column_chunk_scanner.h"
#include "storage/table/csr_chunked_node_group.h"
#include "storage/table/csr_node_group.h"
#include "storage/table/lazy_segment_scanner.h"
#include "storage/table/node_table.h"
#include "transaction/transaction.h"

using namespace rag3db::common;
using namespace rag3db::transaction;

namespace rag3db {
namespace storage {

row_idx_t NodeGroup::append(const Transaction* transaction,
    const std::vector<column_id_t>& columnIDs, ChunkedNodeGroup& chunkedGroup,
    row_idx_t startRowIdx, row_idx_t numRowsToAppend) {
    KU_ASSERT(numRowsToAppend <= chunkedGroup.getNumRows());
    std::vector<const ColumnChunk*> chunksToAppend(chunkedGroup.getNumColumns());
    for (auto i = 0u; i < chunkedGroup.getNumColumns(); i++) {
        chunksToAppend[i] = &chunkedGroup.getColumnChunk(i);
    }
    return append(transaction, columnIDs, chunksToAppend, startRowIdx, numRowsToAppend);
}

row_idx_t NodeGroup::append(const Transaction* transaction,
    const std::vector<column_id_t>& columnIDs, InMemChunkedNodeGroup& chunkedGroup,
    row_idx_t startRowIdx, row_idx_t numRowsToAppend) {
    KU_ASSERT(numRowsToAppend <= chunkedGroup.getNumRows());
    std::vector<const ColumnChunkData*> chunksToAppend(chunkedGroup.getNumColumns());
    for (auto i = 0u; i < chunkedGroup.getNumColumns(); i++) {
        chunksToAppend[i] = &chunkedGroup.getColumnChunk(i);
    }
    return append(transaction, columnIDs, chunksToAppend, startRowIdx, numRowsToAppend);
}

row_idx_t NodeGroup::append(const Transaction* transaction,
    const std::vector<column_id_t>& columnIDs, std::span<const ColumnChunkData*> chunkedGroup,
    row_idx_t startRowIdx, row_idx_t numRowsToAppend) {
    const auto lock = chunkedGroups.lock();
    const std::unique_lock appendGuard{inMemAppendMtx};
    const auto numRowsBeforeAppend = getNumRows();
    if (chunkedGroups.isEmpty(lock)) {
        chunkedGroups.appendGroup(lock,
            std::make_unique<ChunkedNodeGroup>(mm, dataTypes, enableCompression,
                StorageConfig::CHUNKED_NODE_GROUP_CAPACITY, 0, ResidencyState::IN_MEMORY));
    }
    row_idx_t numRowsAppended = 0u;
    while (numRowsAppended < numRowsToAppend) {
        auto lastChunkedGroup = chunkedGroups.getLastGroup(lock);
        if (!lastChunkedGroup || lastChunkedGroup->isFullOrOnDisk()) {
            chunkedGroups.appendGroup(lock,
                std::make_unique<ChunkedNodeGroup>(mm, dataTypes, enableCompression,
                    StorageConfig::CHUNKED_NODE_GROUP_CAPACITY,
                    numRowsBeforeAppend + numRowsAppended, ResidencyState::IN_MEMORY));
        }
        lastChunkedGroup = chunkedGroups.getLastGroup(lock);
        KU_ASSERT(StorageConfig::CHUNKED_NODE_GROUP_CAPACITY >= lastChunkedGroup->getNumRows());
        auto numToCopyIntoChunk =
            StorageConfig::CHUNKED_NODE_GROUP_CAPACITY - lastChunkedGroup->getNumRows();
        const auto numToAppendInChunk =
            std::min(numRowsToAppend - numRowsAppended, numToCopyIntoChunk);
        lastChunkedGroup->append(transaction, columnIDs, chunkedGroup,
            numRowsAppended + startRowIdx, numToAppendInChunk);
        numRowsAppended += numToAppendInChunk;
    }
    numRows += numRowsAppended;
    return numRowsBeforeAppend;
}

row_idx_t NodeGroup::append(const Transaction* transaction,
    const std::vector<column_id_t>& columnIDs, std::span<const ColumnChunk*> chunkedGroup,
    row_idx_t startRowIdx, row_idx_t numRowsToAppend) {
    const auto lock = chunkedGroups.lock();
    const std::unique_lock appendGuard{inMemAppendMtx};
    const auto numRowsBeforeAppend = getNumRows();
    if (chunkedGroups.isEmpty(lock)) {
        chunkedGroups.appendGroup(lock,
            std::make_unique<ChunkedNodeGroup>(mm, dataTypes, enableCompression,
                StorageConfig::CHUNKED_NODE_GROUP_CAPACITY, 0, ResidencyState::IN_MEMORY));
    }
    row_idx_t numRowsAppended = 0u;
    while (numRowsAppended < numRowsToAppend) {
        auto lastChunkedGroup = chunkedGroups.getLastGroup(lock);
        if (!lastChunkedGroup || lastChunkedGroup->isFullOrOnDisk()) {
            chunkedGroups.appendGroup(lock,
                std::make_unique<ChunkedNodeGroup>(mm, dataTypes, enableCompression,
                    StorageConfig::CHUNKED_NODE_GROUP_CAPACITY,
                    numRowsBeforeAppend + numRowsAppended, ResidencyState::IN_MEMORY));
        }
        lastChunkedGroup = chunkedGroups.getLastGroup(lock);
        KU_ASSERT(StorageConfig::CHUNKED_NODE_GROUP_CAPACITY >= lastChunkedGroup->getNumRows());
        auto numToCopyIntoChunk =
            StorageConfig::CHUNKED_NODE_GROUP_CAPACITY - lastChunkedGroup->getNumRows();
        const auto numToAppendInChunk =
            std::min(numRowsToAppend - numRowsAppended, numToCopyIntoChunk);
        lastChunkedGroup->append(transaction, columnIDs, chunkedGroup,
            numRowsAppended + startRowIdx, numToAppendInChunk);
        numRowsAppended += numToAppendInChunk;
    }
    numRows += numRowsAppended;
    return numRowsBeforeAppend;
}

void NodeGroup::append(const Transaction* transaction, const std::vector<ValueVector*>& vectors,
    const row_idx_t startRowIdx, const row_idx_t numRowsToAppend) {
    const auto lock = chunkedGroups.lock();
    const std::unique_lock appendGuard{inMemAppendMtx};
    const auto numRowsBeforeAppend = getNumRows();
    if (chunkedGroups.isEmpty(lock)) {
        chunkedGroups.appendGroup(lock,
            std::make_unique<ChunkedNodeGroup>(mm, dataTypes, enableCompression,
                StorageConfig::CHUNKED_NODE_GROUP_CAPACITY, 0 /*startOffset*/,
                ResidencyState::IN_MEMORY));
    }
    row_idx_t numRowsAppended = 0;
    while (numRowsAppended < numRowsToAppend) {
        auto lastChunkedGroup = chunkedGroups.getLastGroup(lock);
        if (!lastChunkedGroup || lastChunkedGroup->isFullOrOnDisk()) {
            chunkedGroups.appendGroup(lock,
                std::make_unique<ChunkedNodeGroup>(mm, dataTypes, enableCompression,
                    StorageConfig::CHUNKED_NODE_GROUP_CAPACITY,
                    numRowsBeforeAppend + numRowsAppended, ResidencyState::IN_MEMORY));
        }
        lastChunkedGroup = chunkedGroups.getLastGroup(lock);
        const auto numRowsToAppendInGroup = std::min(numRowsToAppend - numRowsAppended,
            StorageConfig::CHUNKED_NODE_GROUP_CAPACITY - lastChunkedGroup->getNumRows());
        lastChunkedGroup->append(transaction, vectors, startRowIdx + numRowsAppended,
            numRowsToAppendInGroup);
        numRowsAppended += numRowsToAppendInGroup;
    }
    numRows += numRowsAppended;
}

void NodeGroup::merge(Transaction*, std::unique_ptr<ChunkedNodeGroup> chunkedGroup) {
    KU_ASSERT(chunkedGroup->getNumColumns() == dataTypes.size());
    for (auto i = 0u; i < chunkedGroup->getNumColumns(); i++) {
        KU_ASSERT(chunkedGroup->getColumnChunk(i).getDataType().getPhysicalType() ==
                  dataTypes[i].getPhysicalType());
    }
    const auto lock = chunkedGroups.lock();
    numRows += chunkedGroup->getNumRows();
    chunkedGroups.appendGroup(lock, std::move(chunkedGroup));
}

void NodeGroup::initializeScanState(const Transaction* transaction, TableScanState& state) const {
    const auto lock = chunkedGroups.lock();
    initializeScanState(transaction, lock, state);
}

static void initializeScanStateForChunkedGroup(const TableScanState& state,
    const ChunkedNodeGroup* chunkedGroup) {
    KU_ASSERT(chunkedGroup);
    if (chunkedGroup->getResidencyState() != ResidencyState::ON_DISK) {
        return;
    }
    auto& nodeGroupScanState = *state.nodeGroupScanState;
    for (auto i = 0u; i < state.columnIDs.size(); i++) {
        KU_ASSERT(i < state.columnIDs.size());
        KU_ASSERT(i < nodeGroupScanState.chunkStates.size());
        const auto columnID = state.columnIDs[i];
        if (columnID == INVALID_COLUMN_ID || columnID == ROW_IDX_COLUMN_ID) {
            continue;
        }
        auto& chunk = chunkedGroup->getColumnChunk(columnID);
        auto& chunkState = nodeGroupScanState.chunkStates[i];
        chunk.initializeScanState(chunkState, state.columns[i]);
    }
}

void NodeGroup::initializeScanState(const Transaction*, const UniqLock& lock,
    TableScanState& state) const {
    auto& nodeGroupScanState = *state.nodeGroupScanState;
    nodeGroupScanState.chunkedGroupIdx = 0;
    nodeGroupScanState.groups = chunkedGroups.getAllGroups(lock);
    KU_ASSERT(!nodeGroupScanState.groups.empty());
    const auto* firstChunkedGroup = nodeGroupScanState.groups.front().get();
    nodeGroupScanState.nextRowToScan = firstChunkedGroup->getStartRowIdx();
    initializeScanStateForChunkedGroup(state, firstChunkedGroup);
}

void NodeGroup::extendScanGroups(const UniqLock& lock, NodeGroupScanState& scanState) const {
    const auto& live = chunkedGroups.getAllGroups(lock);
    auto& held = scanState.groups;
    if (held.empty()) {
        held = live;
        return;
    }
    if (live.empty() || live.front() != held.front()) {
        return;
    }
    for (auto i = held.size(); i < live.size(); i++) {
        held.push_back(live[i]);
    }
}

const ChunkedNodeGroup* NodeGroup::scanGroupAt(NodeGroupScanState& scanState,
    idx_t chunkedGroupIdx) const {
    if (chunkedGroupIdx >= scanState.groups.size()) {
        const auto lock = chunkedGroups.lock();
        extendScanGroups(lock, scanState);
        if (chunkedGroupIdx >= scanState.groups.size()) {
            return nullptr;
        }
    }
    return scanState.groups[chunkedGroupIdx].get();
}

void applySemiMaskFilter(const TableScanState& state, row_idx_t numRowsToScan,
    SelectionVector& selVector) {
    auto& nodeGroupScanState = *state.nodeGroupScanState;
    const auto startNodeOffset = nodeGroupScanState.nextRowToScan +
                                 StorageUtils::getStartOffsetOfNodeGroup(state.nodeGroupIdx);
    const auto endNodeOffset = startNodeOffset + numRowsToScan;
    const auto& arr = state.semiMask->range(startNodeOffset, endNodeOffset);
    if (arr.empty()) {
        selVector.setSelSize(0);
    } else {
        auto stat = selVector.getMutableBuffer();
        uint64_t numSelectedValues = 0;
        size_t i = 0, j = 0;
        while (i < numRowsToScan && j < arr.size()) {
            auto temp = arr[j] - startNodeOffset;
            if (selVector[i] < temp) {
                ++i;
            } else if (selVector[i] > temp) {
                ++j;
            } else {
                stat[numSelectedValues++] = temp;
                ++i;
                ++j;
            }
        }
        selVector.setToFiltered(numSelectedValues);
    }
}

NodeGroupScanResult NodeGroup::scan(const Transaction* transaction, TableScanState& state) const {
    // TODO(Guodong): Move the locked part of figuring out the chunked group to initScan.
    const auto lock = chunkedGroups.lock();
    auto& nodeGroupScanState = *state.nodeGroupScanState;
    auto& groups = nodeGroupScanState.groups;
    KU_ASSERT(nodeGroupScanState.chunkedGroupIdx < groups.size());
    const auto* chunkedGroup = groups[nodeGroupScanState.chunkedGroupIdx].get();
    if (nodeGroupScanState.nextRowToScan >=
        chunkedGroup->getNumRows() + chunkedGroup->getStartRowIdx()) {
        nodeGroupScanState.chunkedGroupIdx++;
        if (nodeGroupScanState.chunkedGroupIdx >= groups.size()) {
            extendScanGroups(lock, nodeGroupScanState);
            if (nodeGroupScanState.chunkedGroupIdx >= groups.size()) {
                return NODE_GROUP_SCAN_EMPTY_RESULT;
            }
        }
        initializeScanStateForChunkedGroup(state, groups[nodeGroupScanState.chunkedGroupIdx].get());
    }
    const auto& chunkedGroupToScan = *groups[nodeGroupScanState.chunkedGroupIdx];
    KU_ASSERT(nodeGroupScanState.nextRowToScan >= chunkedGroupToScan.getStartRowIdx());
    const auto rowIdxInChunkToScan =
        nodeGroupScanState.nextRowToScan - chunkedGroupToScan.getStartRowIdx();
    const auto numRowsToScan =
        std::min(chunkedGroupToScan.getNumRows() - rowIdxInChunkToScan, DEFAULT_VECTOR_CAPACITY);
    bool enableSemiMask =
        state.source == TableScanSource::COMMITTED && state.semiMask && state.semiMask->isEnabled();
    if (enableSemiMask) {
        applySemiMaskFilter(state, numRowsToScan, state.outState->getSelVectorUnsafe());
        if (state.outState->getSelVector().getSelSize() == 0) {
            state.nodeGroupScanState->nextRowToScan += numRowsToScan;
            return NodeGroupScanResult{nodeGroupScanState.nextRowToScan, 0};
        }
    }
    chunkedGroupToScan.scan(transaction, state, nodeGroupScanState, rowIdxInChunkToScan,
        numRowsToScan);
    const auto startRow = nodeGroupScanState.nextRowToScan;
    nodeGroupScanState.nextRowToScan += numRowsToScan;
    return NodeGroupScanResult{startRow, numRowsToScan};
}

NodeGroupScanResult NodeGroup::scan(Transaction* transaction, TableScanState& state,
    offset_t startOffsetInGroup, offset_t numRowsToScan) const {
    bool enableSemiMask =
        state.source == TableScanSource::COMMITTED && state.semiMask && state.semiMask->isEnabled();
    if (enableSemiMask) {
        applySemiMaskFilter(state, numRowsToScan, state.outState->getSelVectorUnsafe());
        if (state.outState->getSelVector().getSelSize() == 0) {
            state.nodeGroupScanState->nextRowToScan += numRowsToScan;
            return NodeGroupScanResult{state.nodeGroupScanState->nextRowToScan, 0};
        }
    }
    if (state.outputVectors.size() == 0) {
        KU_ASSERT(scanInternal(chunkedGroups.lock(), transaction, state, startOffsetInGroup,
                      numRowsToScan) == NodeGroupScanResult(startOffsetInGroup, numRowsToScan));
        return NodeGroupScanResult{startOffsetInGroup, numRowsToScan};
    }
    return scanInternal(chunkedGroups.lock(), transaction, state, startOffsetInGroup,
        numRowsToScan);
}

NodeGroupScanResult NodeGroup::scanInternal(const UniqLock& lock, Transaction* transaction,
    TableScanState& state, offset_t startOffsetInGroup, offset_t numRowsToScan) const {
    // Only meant for scanning once
    KU_ASSERT(numRowsToScan <= DEFAULT_VECTOR_CAPACITY);

    auto& nodeGroupScanState = *state.nodeGroupScanState;
    auto& groups = nodeGroupScanState.groups;
    if (groups.empty()) {
        extendScanGroups(lock, nodeGroupScanState);
    }
    KU_ASSERT(!groups.empty());
    auto startRowIdxInGroup = groups.front()->getStartRowIdx();
    if (startOffsetInGroup < startRowIdxInGroup) {
        numRowsToScan = std::min(numRowsToScan, startRowIdxInGroup - startOffsetInGroup);
        // If the scan starts before the first row in the group, skip the deleted part and return.
        return NodeGroupScanResult{startOffsetInGroup, numRowsToScan};
    }

    nodeGroupScanState.nextRowToScan = startOffsetInGroup;

    auto newChunkedGroupIdx = findChunkedGroupIdx(groups, startOffsetInGroup).first;
    if (newChunkedGroupIdx == INVALID_CHUNKED_GROUP_IDX) {
        extendScanGroups(lock, nodeGroupScanState);
        newChunkedGroupIdx = findChunkedGroupIdx(groups, startOffsetInGroup).first;
    }
    KU_ASSERT(newChunkedGroupIdx != INVALID_CHUNKED_GROUP_IDX);

    const auto* chunkedGroupToScan = groups[newChunkedGroupIdx].get();
    if (newChunkedGroupIdx != nodeGroupScanState.chunkedGroupIdx) {
        // If the chunked group matches the scan state, don't re-initialize it.
        // E.g., we may scan a group multiple times in parts
        initializeScanStateForChunkedGroup(state, chunkedGroupToScan);
        nodeGroupScanState.chunkedGroupIdx = newChunkedGroupIdx;
    }

    uint64_t numRowsScanned = 0;
    const auto rowIdxInChunkToScan =
        (startOffsetInGroup + numRowsScanned) - chunkedGroupToScan->getStartRowIdx();
    uint64_t numRowsToScanInChunk = std::min(numRowsToScan - numRowsScanned,
        chunkedGroupToScan->getNumRows() - rowIdxInChunkToScan);
    KU_ASSERT(startOffsetInGroup + numRowsToScanInChunk <= numRows);
    chunkedGroupToScan->scan(transaction, state, nodeGroupScanState, rowIdxInChunkToScan,
        numRowsToScanInChunk);
    numRowsScanned += numRowsToScanInChunk;
    nodeGroupScanState.nextRowToScan += numRowsToScanInChunk;

    return NodeGroupScanResult{startOffsetInGroup, numRowsScanned};
}

std::optional<bool> NodeGroup::lookupInScanGroups(const Transaction* transaction,
    const TableScanState& state, sel_t posInSel) const {
    auto& nodeGroupScanState = *state.nodeGroupScanState;
    const auto pos = state.rowIdxVector->state->getSelVector().getSelectedPositions()[posInSel];
    KU_ASSERT(!state.rowIdxVector->isNull(pos));
    const auto rowIdx = state.rowIdxVector->getValue<row_idx_t>(pos);
    const auto chunkedGroupIdx = findChunkedGroupIdx(nodeGroupScanState.groups, rowIdx).first;
    if (chunkedGroupIdx == INVALID_CHUNKED_GROUP_IDX) {
        return std::nullopt;
    }
    const auto& chunkedGroupToScan = *nodeGroupScanState.groups[chunkedGroupIdx];
    const auto rowIdxInChunkedGroup = rowIdx - chunkedGroupToScan.getStartRowIdx();
    return chunkedGroupToScan.lookup(transaction, state, nodeGroupScanState, rowIdxInChunkedGroup,
        posInSel);
}

bool NodeGroup::lookupMultiple(const UniqLock& lock, const Transaction* transaction,
    const TableScanState& state) const {
    idx_t numTuplesFound = 0;
    for (auto i = 0u; i < state.rowIdxVector->state->getSelVector().getSelSize(); i++) {
        auto found = lookupInScanGroups(transaction, state, i);
        if (!found) {
            extendScanGroups(lock, *state.nodeGroupScanState);
            found = lookupInScanGroups(transaction, state, i);
        }
        KU_ASSERT(found);
        numTuplesFound += found.value_or(false);
    }
    return numTuplesFound == state.rowIdxVector->state->getSelVector().getSelSize();
}

bool NodeGroup::lookup(const Transaction* transaction, const TableScanState& state,
    sel_t posInSel) const {
    const auto lock = chunkedGroups.lock();
    if (const auto found = lookupInScanGroups(transaction, state, posInSel)) {
        return *found;
    }
    extendScanGroups(lock, *state.nodeGroupScanState);
    const auto found = lookupInScanGroups(transaction, state, posInSel);
    KU_ASSERT(found);
    return found.value_or(false);
}

bool NodeGroup::lookupSharedLock(const Transaction* transaction, const TableScanState& state,
    sel_t posInSel) const {
    {
        const std::shared_lock appendGuard{inMemAppendMtx};
        if (const auto found = lookupInScanGroups(transaction, state, posInSel)) {
            return *found;
        }
    }
    // Un bloc ajouté depuis l'instantané : la garde est rendue avant le verrou de la
    // collection, qui se prend toujours avant elle.
    {
        const auto lock = chunkedGroups.lock();
        extendScanGroups(lock, *state.nodeGroupScanState);
    }
    const std::shared_lock appendGuard{inMemAppendMtx};
    const auto found = lookupInScanGroups(transaction, state, posInSel);
    KU_ASSERT(found);
    return found.value_or(false);
}

bool NodeGroup::lookupMultiple(const Transaction* transaction, const TableScanState& state) const {
    const auto lock = chunkedGroups.lock();
    return lookupMultiple(lock, transaction, state);
}

// NOLINTNEXTLINE(readability-make-member-function-const): Semantically non-const.
void NodeGroup::update(const Transaction* transaction, row_idx_t rowIdxInGroup,
    column_id_t columnID, const ValueVector& propertyVector) {
    KU_ASSERT(propertyVector.state->getSelVector().getSelSize() == 1);
    ChunkedNodeGroup* chunkedGroupToUpdate = nullptr;
    {
        const auto lock = chunkedGroups.lock();
        chunkedGroupToUpdate = findChunkedGroupFromRowIdx(lock, rowIdxInGroup);
    }
    KU_ASSERT(chunkedGroupToUpdate);
    const auto rowIdxInChunkedGroup = rowIdxInGroup - chunkedGroupToUpdate->getStartRowIdx();
    chunkedGroupToUpdate->update(transaction, rowIdxInChunkedGroup, columnID, propertyVector);
}

// NOLINTNEXTLINE(readability-make-member-function-const): Semantically non-const.
bool NodeGroup::delete_(const Transaction* transaction, row_idx_t rowIdxInGroup) {
    ChunkedNodeGroup* groupToDelete = nullptr;
    {
        const auto lock = chunkedGroups.lock();
        groupToDelete = findChunkedGroupFromRowIdx(lock, rowIdxInGroup);
    }
    KU_ASSERT(groupToDelete);
    const auto rowIdxInChunkedGroup = rowIdxInGroup - groupToDelete->getStartRowIdx();
    return groupToDelete->delete_(transaction, rowIdxInChunkedGroup);
}

row_idx_t NodeGroup::getNumLiveRows() const {
    const auto lock = chunkedGroups.lock();
    row_idx_t numLiveRows = 0;
    for (auto i = 0u; i < chunkedGroups.getNumGroups(lock); i++) {
        const auto chunkedGroup = chunkedGroups.getGroup(lock, i);
        const auto numRowsInGroup = chunkedGroup->getNumRows();
        numLiveRows += numRowsInGroup - chunkedGroup->getNumDeletions(
                                            &DUMMY_CHECKPOINT_TRANSACTION, 0, numRowsInGroup);
    }
    return numLiveRows;
}

bool NodeGroup::hasDeletions(const Transaction* transaction) const {
    const auto lock = chunkedGroups.lock();
    for (auto i = 0u; i < chunkedGroups.getNumGroups(lock); i++) {
        const auto chunkedGroup = chunkedGroups.getGroup(lock, i);
        if (chunkedGroup->hasDeletions(transaction)) {
            return true;
        }
    }
    return false;
}

void NodeGroup::addColumn(TableAddColumnState& addColumnState, PageAllocator* pageAllocator,
    ColumnStats* newColumnStats) {
    dataTypes.push_back(addColumnState.propertyDefinition.getType().copy());
    const auto lock = chunkedGroups.lock();
    for (auto& chunkedGroup : chunkedGroups.getAllGroups(lock)) {
        chunkedGroup->addColumn(mm, addColumnState, enableCompression, pageAllocator,
            newColumnStats);
    }
}

void NodeGroup::removeLastColumn(PageAllocator* pageAllocator) {
    KU_ASSERT(!dataTypes.empty());
    dataTypes.pop_back();
    const auto lock = chunkedGroups.lock();
    for (auto& chunkedGroup : chunkedGroups.getAllGroups(lock)) {
        chunkedGroup->removeLastColumn(pageAllocator);
    }
}

void NodeGroup::rollbackInsert(row_idx_t startRow) {
    const auto lock = chunkedGroups.lock();
    const std::unique_lock appendGuard{inMemAppendMtx};
    const auto numEmptyTrailingGroups = chunkedGroups.getNumEmptyTrailingGroups(lock);
    chunkedGroups.removeTrailingGroups(lock, numEmptyTrailingGroups);
    numRows = startRow;
    // Le curseur de réservation recule avec les lignes. Il n'était jamais reculé : chaque
    // insertion annulée rapprochait le groupe de « plein » sans qu'il porte une ligne de plus,
    // et un groupe tenu pour plein avant de l'être fait ouvrir le suivant — dont les lignes
    // n'ont alors plus le décalage que la table leur donne.
    nextRowToAppend = startRow;
}

void NodeGroup::reclaimStorage(PageAllocator& pageAllocator) const {
    reclaimStorage(pageAllocator, chunkedGroups.lock());
}

void NodeGroup::reclaimStorage(PageAllocator& pageAllocator, const UniqLock& lock) const {
    for (auto& chunkedGroup : chunkedGroups.getAllGroups(lock)) {
        chunkedGroup->reclaimStorage(pageAllocator);
    }
}

void NodeGroup::retireStorage(PageAllocator& pageAllocator, const UniqLock& lock) const {
    for (auto& chunkedGroup : chunkedGroups.getAllGroups(lock)) {
        chunkedGroup->retireStorage(pageAllocator);
    }
}

void NodeGroup::checkpoint(MemoryManager& memoryManager, NodeGroupCheckpointState& state) {
    const auto lock = chunkedGroups.lock();
    KU_ASSERT(chunkedGroups.getNumGroups(lock) >= 1);
    // Les trous d'abord : le bloc vidé (flushEmpty) et la copie fusionnée qui reprend ses
    // numéros lisent dataTypes, et une colonne supprimée doit y être ANY, pas son ancien type
    // (sinon elle renaît vide, et le bloc se relit avec zéro ligne).
    checkpointDataTypesNoLock(state);
    const auto firstGroup = chunkedGroups.getFirstGroup(lock);
    const auto hasPersistentData = firstGroup->getResidencyState() == ResidencyState::ON_DISK;
    // Re-populate version info here first.
    auto checkpointedVersionInfo = checkpointVersionInfo(lock, &DUMMY_CHECKPOINT_TRANSACTION);
    std::unique_ptr<ChunkedNodeGroup> checkpointedChunkedGroup;
    if (checkpointedVersionInfo->getNumDeletions(&DUMMY_CHECKPOINT_TRANSACTION, 0, numRows) ==
        numRows - firstGroup->getStartRowIdx()) {
        state.markChangedMemory();
        retireStorage(state.pageAllocator, lock);
        checkpointedChunkedGroup =
            ChunkedNodeGroup::flushEmpty(memoryManager, dataTypes, enableCompression,
                StorageConfig::CHUNKED_NODE_GROUP_CAPACITY, numRows, state.pageAllocator);
    } else {
        if (hasPersistentData) {
            checkpointedChunkedGroup = checkpointInMemAndOnDisk(memoryManager, lock, state);
        } else {
            checkpointedChunkedGroup = checkpointInMemOnly(memoryManager, lock, state);
        }
        checkpointedChunkedGroup->setVersionInfo(std::move(checkpointedVersionInfo));
    }
    // Déjà posé après la copie fusionnée ; l'échange, lui, ne se défait pas non plus.
    state.markChangedMemory();
    chunkedGroups.clear(lock);
    chunkedGroups.appendGroup(lock, std::move(checkpointedChunkedGroup));
}

// Les numéros ne bougent pas : une colonne qui n'est plus dans le point de reprise (supprimée)
// devient un trou, à son numéro.
void NodeGroup::checkpointDataTypesNoLock(const NodeGroupCheckpointState& state) {
    for (auto columnID = 0u; columnID < dataTypes.size(); columnID++) {
        if (std::find(state.columnIDs.begin(), state.columnIDs.end(), columnID) ==
            state.columnIDs.end()) {
            dataTypes[columnID] = LogicalType::ANY();
        }
    }
}

void NodeGroup::scanCommittedUpdatesForColumn(
    std::vector<ChunkCheckpointState>& chunkCheckpointStates, MemoryManager& memoryManager,
    const UniqLock& lock, column_id_t columnID, const Column* column) const {
    auto updateSegmentScanner =
        LazySegmentScanner(memoryManager, column->getDataType().copy(), enableCompression);
    ChunkState chunkState;
    auto& firstColumnChunk = chunkedGroups.getFirstGroup(lock)->getColumnChunk(columnID);
    const auto numPersistentRows = firstColumnChunk.getNumValues();
    firstColumnChunk.initializeScanState(chunkState, column);
    for (auto& chunkedGroup : chunkedGroups.getAllGroups(lock)) {
        chunkedGroup->getColumnChunk(columnID).scanCommitted<ResidencyState::ON_DISK>(
            &DUMMY_CHECKPOINT_TRANSACTION, chunkState, updateSegmentScanner);
    }
    KU_ASSERT(updateSegmentScanner.getNumValues() == numPersistentRows);
    updateSegmentScanner.rangeSegments(updateSegmentScanner.begin(), numPersistentRows,
        [&chunkCheckpointStates](auto& segment, auto, auto segmentLength, auto offsetInChunk) {
            if (segment.segmentData) {
                chunkCheckpointStates.emplace_back(std::move(segment.segmentData), offsetInChunk,
                    segmentLength);
            }
        });
}

std::unique_ptr<ChunkedNodeGroup> NodeGroup::checkpointInMemAndOnDisk(MemoryManager& memoryManager,
    const UniqLock& lock, NodeGroupCheckpointState& state) const {
    const auto firstGroup = chunkedGroups.getFirstGroup(lock);
    const auto numPersistentRows = firstGroup->getNumRows();
    std::vector<const Column*> columnPtrs;
    columnPtrs.reserve(state.columns.size());
    for (auto* column : state.columns) {
        columnPtrs.push_back(column);
    }
    const auto insertChunkedGroup = scanAllInsertedAndVersions<ResidencyState::IN_MEMORY>(
        memoryManager, lock, state.columnIDs, columnPtrs);
    // La copie fusionnée faite, les colonnes du groupe persistant s'écrivent en place (par le
    // fichier ombre) et changent leurs métadonnées : un échec d'après laisse des pages ombres
    // et des segments à moitié réécrits, que Checkpointer::rollback ne défait pas. Seule la
    // copie, juste au-dessus, peut échouer sans rien laisser.
    state.markChangedMemory();
    const auto numInsertedRows = insertChunkedGroup->getNumRows();
    // Le bloc neuf : pour chaque colonne vivante, un chunk neuf (segments inchangés repris,
    // segments changés rebâtis sur des pages neuves) ; rien n'est déplacé hors de l'ancien bloc,
    // qu'un balayage peut encore tenir. Un trou reste un trou.
    std::vector<std::unique_ptr<ColumnChunk>> newChunks(firstGroup->getNumColumns());
    for (auto i = 0u; i < state.columnIDs.size(); i++) {
        const auto columnID = state.columnIDs[i];
        KU_ASSERT(firstGroup && firstGroup->getResidencyState() == ResidencyState::ON_DISK);
        const auto columnHasUpdates = firstGroup->hasAnyUpdates(&DUMMY_CHECKPOINT_TRANSACTION,
            columnID, 0, firstGroup->getNumRows());
        if (numInsertedRows == 0 && !columnHasUpdates) {
            newChunks[columnID] = firstGroup->getColumnChunk(columnID).sharedView();
            continue;
        }
        std::vector<ChunkCheckpointState> chunkCheckpointStates;
        // Deux index : columnID pour le groupe persistant, qui garde toutes ses colonnes ; i
        // pour state.columns et le groupe des insertions, lus dans l'ordre de state.columnIDs.
        if (columnHasUpdates) {
            scanCommittedUpdatesForColumn(chunkCheckpointStates, memoryManager, lock, columnID,
                state.columns[i]);
        }
        if (numInsertedRows > 0) {
            chunkCheckpointStates.emplace_back(insertChunkedGroup->moveColumnChunk(i),
                numPersistentRows, numInsertedRows);
        }
        newChunks[columnID] = firstGroup->getColumnChunk(columnID).checkpoint(*state.columns[i],
            std::move(chunkCheckpointStates), state.pageAllocator);
    }
    // Les colonnes supprimées : leurs pages reviennent à la mort de l'ancien bloc.
    for (auto columnID = 0u; columnID < firstGroup->getNumColumns(); columnID++) {
        if (!newChunks[columnID] && firstGroup->hasColumnChunk(columnID)) {
            firstGroup->getColumnChunk(columnID).retireStorage(state.pageAllocator);
        }
    }
    auto checkpointedChunkedGroup =
        std::make_unique<ChunkedNodeGroup>(std::move(newChunks), firstGroup->getStartRowIdx());
    KU_ASSERT(checkpointedChunkedGroup->getResidencyState() == ResidencyState::ON_DISK);
    return checkpointedChunkedGroup;
}

std::unique_ptr<ChunkedNodeGroup> NodeGroup::checkpointInMemOnly(MemoryManager& memoryManager,
    const UniqLock& lock, const NodeGroupCheckpointState& state) const {
    // Flush insertChunkedGroup to persistent one.
    std::vector<const Column*> columnPtrs;
    columnPtrs.reserve(state.columns.size());
    for (auto& column : state.columns) {
        columnPtrs.push_back(column);
    }
    auto insertChunkedGroup = scanAllInsertedAndVersions<ResidencyState::IN_MEMORY>(memoryManager,
        lock, state.columnIDs, columnPtrs);
    // Après la copie, l'écriture des pages : rien de ce qui suit n'est réputé se défaire (voir
    // checkpointInMemAndOnDisk).
    state.markChangedMemory();
    // Le bloc vidé porte les colonnes vivantes dans l'ordre de state.columnIDs : chacune
    // reprend son numéro, les trous restent des trous.
    auto flushed = insertChunkedGroup->flush(&DUMMY_CHECKPOINT_TRANSACTION, state.pageAllocator);
    return std::make_unique<ChunkedNodeGroup>(memoryManager, *flushed, std::span{dataTypes},
        std::span{state.columnIDs});
}

std::unique_ptr<VersionInfo> NodeGroup::checkpointVersionInfo(const UniqLock& lock,
    const Transaction* transaction) const {
    auto checkpointVersionInfo = std::make_unique<VersionInfo>();
    row_idx_t currRow = 0;
    for (auto& chunkedGroup : chunkedGroups.getAllGroups(lock)) {
        if (chunkedGroup->hasVersionInfo()) {
            // TODO(Guodong): Optimize the for loop here to directly acess the version info.
            for (auto i = 0u; i < chunkedGroup->getNumRows(); i++) {
                if (chunkedGroup->isDeleted(transaction, i)) {
                    checkpointVersionInfo->delete_(transaction->getID(), currRow + i);
                }
            }
        }
        currRow += chunkedGroup->getNumRows();
    }
    return checkpointVersionInfo;
}

uint64_t NodeGroup::getEstimatedMemoryUsage() const {
    uint64_t memUsage = 0;
    const auto lock = chunkedGroups.lock();
    for (const auto& chunkedGroup : chunkedGroups.getAllGroups(lock)) {
        memUsage += chunkedGroup->getEstimatedMemoryUsage();
    }
    return memUsage;
}

void NodeGroup::serialize(Serializer& serializer) {
    // Serialize checkpointed chunks.
    serializer.writeDebuggingInfo("node_group_idx");
    serializer.write<node_group_idx_t>(nodeGroupIdx);
    serializer.writeDebuggingInfo("enable_compression");
    serializer.write<bool>(enableCompression);
    serializer.writeDebuggingInfo("format");
    serializer.write<NodeGroupDataFormat>(format);
    const auto lock = chunkedGroups.lock();
    KU_ASSERT(chunkedGroups.getNumGroups(lock) == 1);
    const auto chunkedGroup = chunkedGroups.getFirstGroup(lock);
    serializer.writeDebuggingInfo("has_checkpointed_data");
    serializer.write<bool>(chunkedGroup->getResidencyState() == ResidencyState::ON_DISK);
    if (chunkedGroup->getResidencyState() == ResidencyState::ON_DISK) {
        serializer.writeDebuggingInfo("checkpointed_data");
        chunkedGroup->serialize(serializer);
    }
}

std::unique_ptr<NodeGroup> NodeGroup::deserialize(MemoryManager& mm, Deserializer& deSer,
    const std::vector<LogicalType>& columnTypes) {
    std::string key;
    node_group_idx_t nodeGroupIdx = INVALID_NODE_GROUP_IDX;
    bool enableCompression = false;
    auto format = NodeGroupDataFormat::REGULAR;
    bool hasCheckpointedData = false;
    deSer.validateDebuggingInfo(key, "node_group_idx");
    deSer.deserializeValue<node_group_idx_t>(nodeGroupIdx);
    deSer.validateDebuggingInfo(key, "enable_compression");
    deSer.deserializeValue<bool>(enableCompression);
    deSer.validateDebuggingInfo(key, "format");
    deSer.deserializeValue<NodeGroupDataFormat>(format);
    deSer.validateDebuggingInfo(key, "has_checkpointed_data");
    deSer.deserializeValue<bool>(hasCheckpointedData);
    if (hasCheckpointedData) {
        deSer.validateDebuggingInfo(key, "checkpointed_data");
    }
    std::unique_ptr<ChunkedNodeGroup> chunkedNodeGroup;
    switch (format) {
    case NodeGroupDataFormat::REGULAR: {
        if (hasCheckpointedData) {
            chunkedNodeGroup = ChunkedNodeGroup::deserialize(mm, deSer);
        } else {
            chunkedNodeGroup = std::make_unique<ChunkedNodeGroup>(mm, columnTypes,
                enableCompression, 0, 0, ResidencyState::IN_MEMORY);
        }
        return std::make_unique<NodeGroup>(mm, nodeGroupIdx, enableCompression,
            std::move(chunkedNodeGroup));
    }
    case NodeGroupDataFormat::CSR: {
        if (hasCheckpointedData) {
            chunkedNodeGroup = ChunkedCSRNodeGroup::deserialize(mm, deSer);
            return std::make_unique<CSRNodeGroup>(mm, nodeGroupIdx, enableCompression,
                std::move(chunkedNodeGroup));
        } else {
            return std::make_unique<CSRNodeGroup>(mm, nodeGroupIdx, enableCompression,
                copyVector(columnTypes));
        }
    }
    default: {
        KU_UNREACHABLE;
    }
    }
}

std::pair<idx_t, row_idx_t> NodeGroup::findChunkedGroupIdx(
    const std::vector<std::shared_ptr<ChunkedNodeGroup>>& groups, row_idx_t rowIdx) {
    if (groups.empty() || rowIdx < groups.front()->getStartRowIdx()) {
        return {INVALID_CHUNKED_GROUP_IDX, INVALID_START_ROW_IDX};
    }
    rowIdx -= groups.front()->getStartRowIdx();
    const auto numRowsInFirstGroup = groups.front()->getNumRows();
    if (rowIdx < numRowsInFirstGroup) {
        return {0, rowIdx};
    }
    rowIdx -= numRowsInFirstGroup;
    const auto chunkedGroupIdx = rowIdx / StorageConfig::CHUNKED_NODE_GROUP_CAPACITY + 1;
    const auto rowIdxInChunk = rowIdx % StorageConfig::CHUNKED_NODE_GROUP_CAPACITY;
    if (chunkedGroupIdx >= groups.size()) {
        return {INVALID_CHUNKED_GROUP_IDX, INVALID_START_ROW_IDX};
    }
    return {chunkedGroupIdx, rowIdxInChunk};
}

std::pair<idx_t, row_idx_t> NodeGroup::findChunkedGroupIdxFromRowIdxNoLock(row_idx_t rowIdx) const {
    return findChunkedGroupIdx(chunkedGroups.getAllGroupsNoLock(), rowIdx);
}

ChunkedNodeGroup* NodeGroup::findChunkedGroupFromRowIdx(const UniqLock& lock,
    row_idx_t rowIdx) const {
    const auto [chunkedGroupIdx, rowIdxInChunkedGroup] =
        findChunkedGroupIdxFromRowIdxNoLock(rowIdx);
    if (chunkedGroupIdx == INVALID_CHUNKED_GROUP_IDX) {
        return nullptr;
    }
    return chunkedGroups.getGroup(lock, chunkedGroupIdx);
}

std::shared_ptr<ChunkedNodeGroup> NodeGroup::findChunkedGroupSharedFromRowIdx(const UniqLock& lock,
    row_idx_t rowIdx) const {
    const auto [chunkedGroupIdx, rowIdxInChunkedGroup] =
        findChunkedGroupIdxFromRowIdxNoLock(rowIdx);
    if (chunkedGroupIdx == INVALID_CHUNKED_GROUP_IDX) {
        return nullptr;
    }
    return chunkedGroups.getGroupShared(lock, chunkedGroupIdx);
}

ChunkedNodeGroup* NodeGroup::findChunkedGroupFromRowIdxNoLock(row_idx_t rowIdx) const {
    const auto [chunkedGroupIdx, rowIdxInChunkedGroup] =
        findChunkedGroupIdxFromRowIdxNoLock(rowIdx);
    if (chunkedGroupIdx == INVALID_CHUNKED_GROUP_IDX) {
        return nullptr;
    }
    return chunkedGroups.getGroupNoLock(chunkedGroupIdx);
}

template<ResidencyState RESIDENCY_STATE>
row_idx_t NodeGroup::getNumResidentRows(const UniqLock& lock) const {
    row_idx_t numResidentRows = 0u;
    for (auto& chunkedGroup : chunkedGroups.getAllGroups(lock)) {
        if (chunkedGroup->getResidencyState() == RESIDENCY_STATE) {
            numResidentRows += chunkedGroup->getNumRows();
        }
    }
    return numResidentRows;
}

template<ResidencyState RESIDENCY_STATE>
std::unique_ptr<InMemChunkedNodeGroup> NodeGroup::scanAllInsertedAndVersions(
    MemoryManager& memoryManager, const UniqLock& lock, const std::vector<column_id_t>& columnIDs,
    const std::vector<const Column*>& columns) const {
    auto numResidentRows = getNumResidentRows<RESIDENCY_STATE>(lock);
    std::vector<LogicalType> columnTypes;
    for (const auto* column : columns) {
        columnTypes.push_back(column->getDataType().copy());
    }
    auto mergedInMemGroup = std::make_unique<InMemChunkedNodeGroup>(memoryManager, columnTypes,
        enableCompression, numResidentRows, chunkedGroups.getFirstGroup(lock)->getStartRowIdx());
    auto scanState = std::make_unique<TableScanState>(columnIDs, columns);
    scanState->nodeGroupScanState = std::make_unique<NodeGroupScanState>(columnIDs.size());
    initializeScanState(&DUMMY_CHECKPOINT_TRANSACTION, lock, *scanState);
    for (auto& chunkedGroup : chunkedGroups.getAllGroups(lock)) {
        chunkedGroup->scanCommitted<RESIDENCY_STATE>(&DUMMY_CHECKPOINT_TRANSACTION, *scanState,
            *mergedInMemGroup);
    }
    for (auto i = 0u; i < columnIDs.size(); i++) {
        if (columnIDs[i] != 0) {
            KU_ASSERT(numResidentRows == mergedInMemGroup->getColumnChunk(i).getNumValues());
        }
    }
    mergedInMemGroup->setNumRows(numResidentRows);
    return mergedInMemGroup;
}

template std::unique_ptr<InMemChunkedNodeGroup>
NodeGroup::scanAllInsertedAndVersions<ResidencyState::ON_DISK>(MemoryManager& memoryManager,
    const UniqLock& lock, const std::vector<column_id_t>& columnIDs,
    const std::vector<const Column*>& columns) const;
template std::unique_ptr<InMemChunkedNodeGroup>
NodeGroup::scanAllInsertedAndVersions<ResidencyState::IN_MEMORY>(MemoryManager& memoryManager,
    const UniqLock& lock, const std::vector<column_id_t>& columnIDs,
    const std::vector<const Column*>& columns) const;

bool NodeGroup::isVisible(const Transaction* transaction, row_idx_t rowIdxInGroup) const {
    return isVisible(transaction->getStartTS(), transaction->getID(), rowIdxInGroup);
}

bool NodeGroup::isVisible(transaction_t startTS, transaction_t transactionID,
    row_idx_t rowIdxInGroup) const {
    // En propriété partagée : le bloc est lu hors du verrou de la collection, et l'annulation
    // d'une insertion d'une autre transaction peut l'en retirer entre-temps.
    std::shared_ptr<ChunkedNodeGroup> chunkedGroup;
    {
        const auto lock = chunkedGroups.lock();
        chunkedGroup = findChunkedGroupSharedFromRowIdx(lock, rowIdxInGroup);
    }
    if (!chunkedGroup) {
        return false;
    }
    const auto rowIdxInChunkedGroup = rowIdxInGroup - chunkedGroup->getStartRowIdx();
    return !chunkedGroup->isDeleted(startTS, transactionID, rowIdxInChunkedGroup) &&
           chunkedGroup->isInserted(startTS, transactionID, rowIdxInChunkedGroup);
}

bool NodeGroup::wasWrittenByCommitAfter(transaction_t startTS, transaction_t transactionID,
    row_idx_t rowIdxInGroup) const {
    std::shared_ptr<ChunkedNodeGroup> chunkedGroup;
    {
        const auto lock = chunkedGroups.lock();
        chunkedGroup = findChunkedGroupSharedFromRowIdx(lock, rowIdxInGroup);
    }
    if (!chunkedGroup) {
        return false;
    }
    return chunkedGroup->wasWrittenByCommitAfter(startTS, transactionID,
        rowIdxInGroup - chunkedGroup->getStartRowIdx());
}

bool NodeGroup::wasColumnUpdatedByCommitAfter(column_id_t columnID, transaction_t startTS,
    row_idx_t rowIdxInGroup) const {
    // En propriété partagée, comme isVisible : le bloc est lu hors du verrou de la collection.
    std::shared_ptr<ChunkedNodeGroup> chunkedGroup;
    {
        const auto lock = chunkedGroups.lock();
        chunkedGroup = findChunkedGroupSharedFromRowIdx(lock, rowIdxInGroup);
    }
    if (!chunkedGroup) {
        return false;
    }
    return chunkedGroup->getColumnChunk(columnID).hasCommittedUpdateAfter(startTS,
        rowIdxInGroup - chunkedGroup->getStartRowIdx());
}

bool NodeGroup::isVisibleNoLock(const Transaction* transaction, row_idx_t rowIdxInGroup) const {
    // Sans le verrou de chunkedGroups, mais sous la garde partagée : la liste des blocs
    // peut s'allonger sous un lecteur.
    const std::shared_lock appendGuard{inMemAppendMtx};
    const auto* chunkedGroup = findChunkedGroupFromRowIdxNoLock(rowIdxInGroup);
    if (!chunkedGroup) {
        return false;
    }
    const auto rowIdxInChunkedGroup = rowIdxInGroup - chunkedGroup->getStartRowIdx();
    return !chunkedGroup->isDeleted(transaction, rowIdxInChunkedGroup) &&
           chunkedGroup->isInserted(transaction, rowIdxInChunkedGroup);
}

bool NodeGroup::isDeleted(const Transaction* transaction, offset_t offsetInGroup) const {
    const auto lock = chunkedGroups.lock();
    const auto* chunkedGroup = findChunkedGroupFromRowIdx(lock, offsetInGroup);
    KU_ASSERT(chunkedGroup);
    return chunkedGroup->isDeleted(transaction, offsetInGroup - chunkedGroup->getStartRowIdx());
}

bool NodeGroup::isInserted(const Transaction* transaction, offset_t offsetInGroup) const {
    const auto lock = chunkedGroups.lock();
    const auto* chunkedGroup = findChunkedGroupFromRowIdx(lock, offsetInGroup);
    KU_ASSERT(chunkedGroup);
    return chunkedGroup->isInserted(transaction, offsetInGroup - chunkedGroup->getStartRowIdx());
}

void NodeGroup::applyFuncToChunkedGroups(version_record_handler_op_t func, row_idx_t startRow,
    row_idx_t numRows, transaction_t commitTS) const {
    KU_ASSERT(startRow <= getNumRows());

    auto lock = chunkedGroups.lock();
    const auto [chunkedGroupIdx, startRowInChunkedGroup] =
        findChunkedGroupIdxFromRowIdxNoLock(startRow);
    if (chunkedGroupIdx != INVALID_CHUNKED_GROUP_IDX) {
        auto curChunkedGroupIdx = chunkedGroupIdx;
        auto curStartRowIdxInChunk = startRowInChunkedGroup;

        auto numRowsLeft = numRows;
        while (numRowsLeft > 0 && curChunkedGroupIdx < chunkedGroups.getNumGroups(lock)) {
            auto* chunkedGroup = chunkedGroups.getGroup(lock, curChunkedGroupIdx);
            const auto numRowsForGroup =
                std::min(numRowsLeft, chunkedGroup->getNumRows() - curStartRowIdxInChunk);
            std::invoke(func, *chunkedGroup, curStartRowIdxInChunk, numRowsForGroup, commitTS);

            ++curChunkedGroupIdx;
            numRowsLeft -= numRowsForGroup;
            curStartRowIdxInChunk = 0;
        }
    }
}

row_idx_t NodeGroup::getStartRowIdxInGroupNoLock() const {
    return chunkedGroups.getFirstGroupNoLock()->getStartRowIdx();
}

row_idx_t NodeGroup::getStartRowIdxInGroup(const common::UniqLock& lock) const {
    return chunkedGroups.getFirstGroup(lock)->getStartRowIdx();
}

} // namespace storage
} // namespace rag3db
