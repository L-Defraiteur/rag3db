#include "storage/local_storage/local_storage.h"

#include "storage/local_storage/local_node_table.h"
#include "storage/local_storage/local_rel_table.h"
#include "storage/local_storage/local_table.h"
#include "storage/storage_manager.h"
#include "storage/table/rel_table.h"
#include "storage/table/table.h"

using namespace rag3db::common;
using namespace rag3db::transaction;

namespace rag3db {
namespace storage {

LocalTable* LocalStorage::getOrCreateLocalTable(Table& table) {
    const auto tableID = table.getTableID();
    auto catalog = catalog::Catalog::Get(clientContext);
    auto transaction = transaction::Transaction::Get(clientContext);
    auto& mm = *MemoryManager::Get(clientContext);
    if (!tables.contains(tableID)) {
        switch (table.getTableType()) {
        case TableType::NODE: {
            auto tableEntry = catalog->getTableCatalogEntry(transaction, table.getTableID());
            tables[tableID] = std::make_unique<LocalNodeTable>(tableEntry, table, mm);
        } break;
        case TableType::REL: {
            // We have to fetch the rel group entry from the catalog to based on the relGroupID.
            auto tableEntry =
                catalog->getTableCatalogEntry(transaction, table.cast<RelTable>().getRelGroupID());
            tables[tableID] = std::make_unique<LocalRelTable>(tableEntry, table, mm);
        } break;
        default:
            KU_UNREACHABLE;
        }
    }
    return tables.at(tableID).get();
}

LocalTable* LocalStorage::getLocalTable(table_id_t tableID) const {
    if (tables.contains(tableID)) {
        return tables.at(tableID).get();
    }
    return nullptr;
}

PageAllocator* LocalStorage::addOptimisticAllocator() {
    auto* dataFH = StorageManager::Get(clientContext)->getDataFH();
    if (dataFH->isInMemoryMode()) {
        return dataFH->getPageManager();
    }
    UniqLock lck{mtx};
    optimisticAllocators.emplace_back(
        std::make_unique<OptimisticAllocator>(*dataFH->getPageManager()));
    return optimisticAllocators.back().get();
}

void LocalStorage::commit() {
    auto catalog = catalog::Catalog::Get(clientContext);
    auto transaction = transaction::Transaction::Get(clientContext);
    auto storageManager = StorageManager::Get(clientContext);
    // The nodes this transaction created were numbered from the size each table had when the
    // transaction first wrote to it. If another transaction committed nodes to the same table
    // since, they land further: note where, so that the relationships created towards them in
    // this transaction can follow. Without it they pointed to the other transaction's nodes.
    local_node_offset_map_t nodeOffsetMap;
    // What each local relationship table held before the nodes are committed: committing nodes
    // can add rows to some (an index creating edges for the new nodes), and those carry final
    // offsets already.
    std::unordered_map<table_id_t, row_idx_t> numLocalRelsBeforeCommit;
    for (auto& [tableID, localTable] : tables) {
        if (localTable->getTableType() == TableType::REL) {
            numLocalRelsBeforeCommit[tableID] = localTable->getNumTotalRows();
        }
    }
    for (auto& [tableID, localTable] : tables) {
        if (localTable->getTableType() == TableType::NODE) {
            const auto tableEntry = catalog->getTableCatalogEntry(transaction, tableID);
            const auto table = storageManager->getTable(tableID);
            const auto oldStartOffset = localTable->cast<LocalNodeTable>().getStartOffset();
            const auto newStartOffset = table->getNumTotalRows(nullptr /* transaction */);
            const auto numRows = localTable->getNumTotalRows();
            table->commit(&clientContext, tableEntry, localTable.get());
            if (numRows > 0) {
                nodeOffsetMap[tableID] =
                    LocalNodeOffsetMap{oldStartOffset, newStartOffset, numRows};
            }
        }
    }
    for (auto& [tableID, localTable] : tables) {
        if (localTable->getTableType() == TableType::REL) {
            const auto table = storageManager->getTable(tableID);
            const auto tableEntry =
                catalog->getTableCatalogEntry(transaction, table->cast<RelTable>().getRelGroupID());
            const auto numRowsBefore = numLocalRelsBeforeCommit.find(tableID);
            localTable->cast<LocalRelTable>().remapNodeOffsets(nodeOffsetMap,
                numRowsBefore == numLocalRelsBeforeCommit.end() ? 0 : numRowsBefore->second);
            table->commit(&clientContext, tableEntry, localTable.get());
        }
    }
    for (auto& optimisticAllocator : optimisticAllocators) {
        optimisticAllocator->commit();
    }
}

void LocalStorage::rollback() {
    auto mm = MemoryManager::Get(clientContext);
    for (auto& [_, localTable] : tables) {
        localTable->clear(*mm);
    }
    for (auto& optimisticAllocator : optimisticAllocators) {
        optimisticAllocator->rollback();
    }
    auto* bufferManager = mm->getBufferManager();
    PageManager::Get(clientContext)->clearEvictedBMEntriesIfNeeded(bufferManager);
}

} // namespace storage
} // namespace rag3db
