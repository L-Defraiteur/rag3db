#pragma once

#include <unordered_map>

#include "common/copy_constructors.h"
#include "storage/local_storage/local_table.h"
#include "storage/optimistic_allocator.h"

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

private:
    main::ClientContext& clientContext;
    std::unordered_map<common::table_id_t, std::unique_ptr<LocalTable>> tables;

    // The mutex is only needed when working with the optimistic allocators
    std::mutex mtx;
    std::vector<std::unique_ptr<OptimisticAllocator>> optimisticAllocators;
};

} // namespace storage
} // namespace rag3db
