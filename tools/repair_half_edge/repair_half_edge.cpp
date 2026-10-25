// Réparation d'une relation trouvée d'un seul côté (ticket
// 2026-10-25-une-relation-locks-trouvee-d-un-seul-cote-fait-echouer-le-drain-de-la-montre).
//
// Une relation est rangée deux fois, une par sens. Quand un des deux rangements l'a perdue, la
// suppression ordinaire (RelTable::delete_) la cherche d'abord au sens avant, ne l'y trouve pas
// et ne fait rien ; la mise à jour la refuse (« not found in the forward direction »). La moitié
// restante ne s'efface donc par aucune requête. Ce programme l'efface dans le seul sens où elle
// est, puis écrit un point de reprise.
//
// Il vérifie d'abord, par deux requêtes ancrées chacune sur un bout, que la relation manque à un
// sens et se trouve à l'autre ; sinon il refuse sans rien écrire. La suppression ne passe pas par
// le journal : son rejeu (RelTable::delete_, les deux sens) échouerait sur le sens qui ne la
// porte pas. Elle n'est durable que par le point de reprise qui suit, écrit avant de rendre la
// main ; un arrêt entre les deux la perd, et la base reste comme avant.
//
// La base doit être fermée par tout autre processus. Travailler sur une copie, la vérifier, puis
// seulement la base elle-même.
//
// Bâtir dans l'arbre du moteur (il lie la bibliothèque statique, internes compris) :
//   cmake --build <build> --target repair_half_edge
// Usage : repair_half_edge <base> <groupe> <source> <destination> <offset source>
//                          <offset destination> <offset relation>
//   ex. : repair_half_edge base LOCKS Scope Symbol 16249 13803 851
// Code de sortie : 0 réparée, 1 refusée (rien d'écrit), 2 erreur.
#include <iostream>
#include <string>

#include "catalog/catalog.h"
#include "catalog/catalog_entry/rel_group_catalog_entry.h"
#include "common/data_chunk/data_chunk_state.h"
#include "common/enums/rel_direction.h"
#include "common/vector/value_vector.h"
#include "main/client_context.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/buffer_manager/memory_manager.h"
#include "storage/storage_manager.h"
#include "storage/table/rel_table.h"
#include "storage/table/rel_table_data.h"
#include "transaction/transaction.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::storage;

namespace {

int64_t count(Connection& conn, const std::string& query) {
    auto result = conn.query(query);
    if (!result->isSuccess()) {
        std::cerr << "ERREUR " << query << " : " << result->getErrorMessage() << "\n";
        exit(2);
    }
    return result->getNext()->getValue(0)->getValue<int64_t>();
}

void run(Connection& conn, const std::string& query) {
    auto result = conn.query(query);
    if (!result->isSuccess()) {
        std::cerr << "ERREUR " << query << " : " << result->getErrorMessage() << "\n";
        exit(2);
    }
}

} // namespace

int main(int argc, char** argv) {
    if (argc < 8) {
        std::cerr << "usage : repair_half_edge <base> <groupe> <source> <destination> "
                     "<offset source> <offset destination> <offset relation>\n";
        return 2;
    }
    const std::string group = argv[2], srcLabel = argv[3], dstLabel = argv[4];
    const offset_t srcOffset = std::stoull(argv[5]), dstOffset = std::stoull(argv[6]),
                   relOffset = std::stoull(argv[7]);
    SystemConfig config;
    config.bufferPoolSize = 4ull << 30;
    config.autoCheckpoint = false;
    Database db(argv[1], config);
    Connection conn(&db);

    // Les deux rangements, chacun lu depuis son bout : le sens avant ancré sur la source, le sens
    // arrière sur la destination.
    const auto pattern = "(a:" + srcLabel + ")-[r:" + group + "]->(b:" + dstLabel + ")";
    const auto rel = " AND offset(id(r)) = " + std::to_string(relOffset) + " RETURN count(*);";
    const auto inFwd = count(conn, "MATCH " + pattern + " WHERE offset(id(a)) = " +
                                       std::to_string(srcOffset) + rel);
    const auto inBwd = count(conn, "MATCH " + pattern + " WHERE offset(id(b)) = " +
                                       std::to_string(dstOffset) + rel);
    std::cout << "relation " << relOffset << " : sens avant " << inFwd << ", sens arrière "
              << inBwd << "\n";
    if (inFwd + inBwd != 1) {
        std::cout << "refusée : la relation n'est pas d'un seul côté\n";
        return 1;
    }
    const auto direction = inFwd == 1 ? RelDataDirection::FWD : RelDataDirection::BWD;
    const auto boundOffset = direction == RelDataDirection::FWD ? srcOffset : dstOffset;

    auto* context = conn.getClientContext();
    run(conn, "BEGIN TRANSACTION;");
    auto* transaction = rag3db::transaction::Transaction::Get(*context);
    auto* catalog = rag3db::catalog::Catalog::Get(*context);
    const auto srcTableID = catalog->getTableCatalogEntry(transaction, srcLabel)->getTableID();
    const auto dstTableID = catalog->getTableCatalogEntry(transaction, dstLabel)->getTableID();
    const auto* info = catalog->getTableCatalogEntry(transaction, group)
                           ->constCast<rag3db::catalog::RelGroupCatalogEntry>()
                           .getRelEntryInfo(srcTableID, dstTableID);
    if (info == nullptr) {
        std::cerr << "ERREUR : " << group << " ne relie pas " << srcLabel << " à " << dstLabel
                  << "\n";
        return 2;
    }
    auto& relTable = StorageManager::Get(*context)->getTable(info->oid)->cast<RelTable>();
    auto* memoryManager = MemoryManager::Get(*context);
    const auto state = DataChunkState::getSingleValueDataChunkState();
    ValueVector boundVector(LogicalType::INTERNAL_ID(), memoryManager);
    boundVector.setState(state);
    boundVector.setValue<internalID_t>(0,
        internalID_t{boundOffset, direction == RelDataDirection::FWD ? srcTableID : dstTableID});
    ValueVector relIDVector(LogicalType::INTERNAL_ID(), memoryManager);
    relIDVector.setState(state);
    relIDVector.setValue<internalID_t>(0, internalID_t{relOffset, info->oid});
    if (!relTable.getDirectedTableData(direction)->delete_(transaction, boundVector,
            relIDVector)) {
        run(conn, "ROLLBACK;");
        std::cout << "refusée : la suppression ne trouve pas la relation dans le sens "
                  << (direction == RelDataDirection::FWD ? "avant" : "arrière") << "\n";
        return 1;
    }
    // Sans le drapeau, le point de reprise passerait la table.
    relTable.setHasChanges();
    run(conn, "COMMIT;");
    run(conn, "CHECKPOINT;");
    std::cout << "réparée : relation " << relOffset << " effacée du sens "
              << (direction == RelDataDirection::FWD ? "avant" : "arrière")
              << ", point de reprise écrit\n";
    return 0;
}
