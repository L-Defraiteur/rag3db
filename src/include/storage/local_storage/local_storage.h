#pragma once

#include <unordered_map>
#include <vector>

#include "common/copy_constructors.h"
#include "storage/local_storage/local_table.h"
#include "storage/optimistic_allocator.h"
#include "storage/stats/table_stats.h"

namespace rag3db {
namespace main {
class ClientContext;
} // namespace main
namespace storage {
// Data structures in LocalStorage are not thread-safe.
// For now, we only support single thread insertions and updates. Once we optimize them with
// multiple threads, LocalStorage and its related data structures should be reworked to be
// thread-safe.
class LocalStorage {
public:
    explicit LocalStorage(main::ClientContext& clientContext) : clientContext{clientContext} {}
    DELETE_COPY_AND_MOVE(LocalStorage);

    // Do nothing if the table already exists, otherwise create a new local table.
    LocalTable* getOrCreateLocalTable(Table& table);
    // Return nullptr if no local table exists.
    LocalTable* getLocalTable(common::table_id_t tableID) const;
    // Aucune table locale : la transaction n'a rien inséré.
    bool isEmpty() const { return tables.empty(); }

    PageAllocator* addOptimisticAllocator();

    void commit();
    void rollback();

    // Verse dans la table, en cours de transaction, les lignes locales d'une table de nœuds —
    // ce que le commit fait pour elle : elles deviennent des lignes non validées ordinaires
    // de la transaction, à leurs décalages définitifs, écrites à son journal, inscrites à
    // l'index, annulables. Pour un COPY, qui écrit directement dans la table à la suite de ce
    // qu'elle contient : des lignes locales y occuperaient les mêmes décalages que les siennes.
    // Ne fait rien si la transaction n'a pas de ligne locale dans cette table.
    void flushNodeTable(common::table_id_t tableID);

    // Les statistiques de lignes que la transaction a déjà écrites dans une table de nœuds sans
    // les avoir validées — celles d'un COPY, celles des lignes locales versées avant lui. Elles
    // attendent ici : fusionnées dans la table à la validation, jetées à l'annulation, et
    // ajoutées à ce que la transaction lit de la table (NodeTable::getStats). Comme celles d'un
    // CREATE, qui attendent dans la table locale. Un compte de distincts ne se défait pas :
    // c'est pourquoi rien n'entre dans la table avant la validation.
    struct PendingNodeStats {
        std::vector<common::column_id_t> columnIDs;
        TableStats stats;
    };
    void addPendingNodeStats(common::table_id_t tableID,
        std::vector<common::column_id_t> columnIDs, TableStats stats);
    const std::vector<PendingNodeStats>* getPendingNodeStats(common::table_id_t tableID) const;

private:
    main::ClientContext& clientContext;
    std::unordered_map<common::table_id_t, std::unique_ptr<LocalTable>> tables;
    std::unordered_map<common::table_id_t, std::vector<PendingNodeStats>> pendingNodeStats;

    // The mutex is only needed when working with the optimistic allocators
    std::mutex mtx;
    std::vector<std::unique_ptr<OptimisticAllocator>> optimisticAllocators;
};

} // namespace storage
} // namespace rag3db
