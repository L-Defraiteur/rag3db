#include "storage/local_storage/local_storage.h"

#include "storage/local_storage/local_node_table.h"
#include "storage/local_storage/local_rel_table.h"
#include "storage/local_storage/local_table.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"
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
    } else if (table.getTableType() == TableType::NODE &&
               tables.at(tableID)->getNumTotalRows() == 0) {
        tables.at(tableID)->cast<LocalNodeTable>().restartAt(
            table.getNumTotalRows(nullptr /* transaction */));
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
    for (auto& [tableID, pendingStats] : pendingNodeStats) {
        // Une table retirée par la transaction après son COPY n'a plus de statistiques.
        if (!catalog->containsTable(transaction, tableID)) {
            continue;
        }
        auto& nodeTable = storageManager->getTable(tableID)->cast<NodeTable>();
        for (const auto& pending : pendingStats) {
            nodeTable.mergeStats(pending.columnIDs, pending.stats);
        }
    }
    pendingNodeStats.clear();
    for (auto& optimisticAllocator : optimisticAllocators) {
        optimisticAllocator->commit();
    }
}

void LocalStorage::addPendingNodeStats(table_id_t tableID, std::vector<column_id_t> columnIDs,
    TableStats stats) {
    pendingNodeStats[tableID].push_back(PendingNodeStats{std::move(columnIDs), std::move(stats)});
}

const std::vector<LocalStorage::PendingNodeStats>* LocalStorage::getPendingNodeStats(
    table_id_t tableID) const {
    const auto found = pendingNodeStats.find(tableID);
    return found == pendingNodeStats.end() ? nullptr : &found->second;
}

void LocalStorage::flushNodeTable(table_id_t tableID) {
    const auto found = tables.find(tableID);
    if (found == tables.end() || found->second->getNumTotalRows() == 0) {
        return;
    }
    auto& localTable = *found->second;
    KU_ASSERT(localTable.getTableType() == TableType::NODE);
    auto catalog = catalog::Catalog::Get(clientContext);
    auto transaction = transaction::Transaction::Get(clientContext);
    auto storageManager = StorageManager::Get(clientContext);
    // Comme au commit : les relations locales créées vers ces nœuds les désignent par leurs
    // décalages provisoires. Avec un seul écrivain ce sont déjà les définitifs ; si un autre
    // écrivain a validé des nœuds dans la table depuis, ils ont changé, et les relations
    // suivent. Ce remappage en cours de transaction n'est pas prouvé sous concurrence (témoin
    // attendu de la marche A3′).
    std::unordered_map<table_id_t, row_idx_t> numLocalRelsBeforeFlush;
    for (auto& [relTableID, localRelTable] : tables) {
        if (localRelTable->getTableType() == TableType::REL) {
            numLocalRelsBeforeFlush[relTableID] = localRelTable->getNumTotalRows();
        }
    }
    const auto tableEntry = catalog->getTableCatalogEntry(transaction, tableID);
    const auto table = storageManager->getTable(tableID);
    local_node_offset_map_t nodeOffsetMap;
    nodeOffsetMap[tableID] = LocalNodeOffsetMap{localTable.cast<LocalNodeTable>().getStartOffset(),
        table->getNumTotalRows(nullptr /* transaction */), localTable.getNumTotalRows()};
    // Les lignes entrent dans la table, pas leurs statistiques : la transaction n'est pas
    // validée. Elles attendent avec celles du COPY qui suit.
    auto& localNodeTable = localTable.cast<LocalNodeTable>();
    addPendingNodeStats(tableID, localNodeTable.getCommittedColumnIDs(),
        localNodeTable.takeStats());
    table->commit(&clientContext, tableEntry, &localTable);
    for (auto& [relTableID, localRelTable] : tables) {
        if (localRelTable->getTableType() == TableType::REL) {
            // Une table de relations locale née pendant le versement (un index qui relie les
            // nouvelles lignes) ne porte que des décalages définitifs.
            const auto numRowsBefore = numLocalRelsBeforeFlush.find(relTableID);
            localRelTable->cast<LocalRelTable>().remapNodeOffsets(nodeOffsetMap,
                numRowsBefore == numLocalRelsBeforeFlush.end() ? 0 : numRowsBefore->second);
        }
    }
}

void LocalStorage::rollback() {
    auto mm = MemoryManager::Get(clientContext);
    for (auto& [_, localTable] : tables) {
        localTable->clear(*mm);
    }
    pendingNodeStats.clear();
    for (auto& optimisticAllocator : optimisticAllocators) {
        optimisticAllocator->rollback();
    }
    auto* bufferManager = mm->getBufferManager();
    PageManager::Get(clientContext)->clearEvictedBMEntriesIfNeeded(bufferManager);
}

} // namespace storage
} // namespace rag3db
