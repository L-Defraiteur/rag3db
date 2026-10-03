#include "integrity/integrity_checker.h"

#include <algorithm>
#include <map>
#include <stdexcept>

#include "catalog/catalog.h"
#include "catalog/catalog_entry/rel_group_catalog_entry.h"
#include "catalog/catalog_entry/table_catalog_entry.h"
#include "common/data_chunk/data_chunk_state.h"
#include "common/string_format.h"
#include "common/vector/value_vector.h"
#include "main/client_context.h"
#include "processor/result/flat_tuple.h"
#include "storage/buffer_manager/memory_manager.h"
#include "storage/storage_manager.h"
#include "storage/table/csr_node_group.h"
#include "storage/table/node_table.h"
#include "storage/table/rel_table.h"
#include "transaction/transaction.h"

using namespace rag3db::common;

namespace rag3db {
namespace testing {
namespace integrity {

namespace {

// Au-delà, une violation est résumée : la sortie brute doit rester lisible.
constexpr size_t MAX_LISTED = 10;

constexpr const char* MISSING = "<missing>";

std::unique_ptr<main::QueryResult> mustQuery(main::Connection& connection,
    const std::string& query) {
    auto result = connection.query(query);
    if (!result->isSuccess()) {
        throw std::runtime_error(
            "integrity checker query failed: " + query + "\n  " + result->getErrorMessage());
    }
    return result;
}

std::vector<std::vector<std::string>> rows(main::Connection& connection, const std::string& query) {
    auto result = mustQuery(connection, query);
    std::vector<std::vector<std::string>> out;
    while (result->hasNext()) {
        const auto tuple = result->getNext();
        std::vector<std::string> row;
        for (auto i = 0u; i < tuple->len(); ++i) {
            row.push_back(tuple->getValue(i)->toString());
        }
        out.push_back(std::move(row));
    }
    return out;
}

struct NodeTable {
    std::string name;
    std::string primaryKey;
    std::vector<std::string> properties;
};

struct RelTable {
    std::string name;
    std::string source;
    std::string destination;
    std::vector<std::string> properties;
    // Position de src_id et dst_id dans properties, ou -1.
    int sourceKeyIndex = -1;
    int destinationKeyIndex = -1;
};

struct Schema {
    std::vector<NodeTable> nodeTables;
    std::vector<RelTable> relTables;
};

Schema loadSchema(main::Connection& connection) {
    Schema schema;
    for (const auto& row : rows(connection, "CALL show_tables() RETURN name, type;")) {
        if (row[1] == "NODE") {
            NodeTable table{row[0]};
            for (const auto& property : rows(connection,
                     stringFormat("CALL table_info('{}') RETURN name, `primary key`;", row[0]))) {
                table.properties.push_back(property[0]);
                if (property[1] == "True") {
                    table.primaryKey = property[0];
                }
            }
            schema.nodeTables.push_back(std::move(table));
        } else if (row[1] == "REL") {
            RelTable rel{row[0]};
            const auto connections = rows(connection,
                stringFormat("CALL show_connection('{}') RETURN `source table name`, "
                             "`destination table name`;",
                    rel.name));
            // Une table de relations à plusieurs paires de tables n'est pas dans le banc.
            if (connections.size() != 1) {
                throw std::runtime_error("integrity checker: rel table " + rel.name +
                                         " has several connections, not supported");
            }
            rel.source = connections[0][0];
            rel.destination = connections[0][1];
            for (const auto& property :
                rows(connection, stringFormat("CALL table_info('{}') RETURN name;", rel.name))) {
                if (property[0] == SOURCE_KEY_PROPERTY) {
                    rel.sourceKeyIndex = static_cast<int>(rel.properties.size());
                } else if (property[0] == DESTINATION_KEY_PROPERTY) {
                    rel.destinationKeyIndex = static_cast<int>(rel.properties.size());
                }
                rel.properties.push_back(property[0]);
            }
            schema.relTables.push_back(std::move(rel));
        }
    }
    return schema;
}

std::string castAll(const std::string& variable, const std::vector<std::string>& properties) {
    std::string out;
    for (const auto& property : properties) {
        out += stringFormat(", CAST({}.{} AS STRING)", variable, property);
    }
    return out;
}

// Les lignes visibles d'une table de nœuds : offset -> toutes les propriétés.
struct VisibleRows {
    std::map<std::string, std::vector<std::string>> byOffset;
    size_t primaryKeyIndex = 0;

    std::string keyOf(const std::string& offset) const {
        const auto it = byOffset.find(offset);
        return it == byOffset.end() ? std::string(MISSING) : it->second[primaryKeyIndex];
    }
};

VisibleRows visibleRows(main::Connection& connection, const NodeTable& table) {
    VisibleRows out;
    out.primaryKeyIndex = static_cast<size_t>(
        std::find(table.properties.begin(), table.properties.end(), table.primaryKey) -
        table.properties.begin());
    for (auto& row : rows(connection, stringFormat("MATCH (n:{}) RETURN offset(id(n)){};",
                                          table.name, castAll("n", table.properties)))) {
        auto offset = row[0];
        row.erase(row.begin());
        out.byOffset[offset] = std::move(row);
    }
    return out;
}

void checkPrimaryKey(main::Connection& connection, const NodeTable& table,
    std::vector<Violation>& violations) {
    const auto counts =
        rows(connection, stringFormat("MATCH (n:{}) RETURN count(n), count(DISTINCT n.{});",
                             table.name, table.primaryKey));
    if (counts[0][0] == counts[0][1]) {
        return;
    }
    auto detail =
        stringFormat("table {}: count = {}, count(distinct {}) = {}; duplicated keys:", table.name,
            counts[0][0], table.primaryKey, counts[0][1]);
    for (const auto& row :
        rows(connection, stringFormat("MATCH (n:{}) WITH n.{} AS k, count(*) AS c WHERE c > 1 "
                                      "RETURN k, c ORDER BY k LIMIT {};",
                             table.name, table.primaryKey, MAX_LISTED))) {
        detail += stringFormat(" {}(x{})", row[0], row[1]);
    }
    violations.push_back({"primary-key-unique", detail});
}

// Une relation telle qu'un balayage la rend. Les clés des extrémités viennent des
// lignes visibles, jamais d'une lecture sur l'extrémité (règle de l'en-tête).
struct Edge {
    std::string sourceOffset;
    std::string destinationOffset;
    std::string sourceKey;
    std::string destinationKey;
    std::vector<std::string> properties;
};

std::string edgeText(const std::string& relOffset, const Edge& edge) {
    return stringFormat("rel@{} ({}@{} -> {}@{})", relOffset, edge.sourceKey, edge.sourceOffset,
        edge.destinationKey, edge.destinationOffset);
}

// Balaye les relations en partant d'un côté. L'indication de jointure oblige le plan à
// balayer d'abord ce côté, puis à étendre vers l'autre, sans jamais balayer la table de
// l'autre extrémité (vérifié par EXPLAIN le 3 octobre : SCAN_NODE_TABLE du côté lié,
// SCAN_REL_TABLE dans le sens voulu, rien d'autre). Une relation dont ce côté est
// invisible n'est pas vue, ce qui est le but : comparer les deux sens fait sortir les
// relations pendantes. Ni WITH ni l'absence d'étiquette ne suffisent : avec « MATCH (a)
// WITH a MATCH (a)-[r]->(b) », l'optimiseur partait de b et étendait vers l'arrière, et
// une relation dont b est supprimé disparaissait des deux côtés (variante « destination
// supprimée » de C2, que seul le niveau 2 voyait). L'autre extrémité ne porte pas
// d'étiquette et aucune de ses propriétés n'est lue (règle de l'en-tête).
// Deux relations distinctes de même identifiant s'écraseraient dans la table rendue :
// chaque identifiant déjà vu est noté dans duplicates (relecture du cœur C++, 3 octobre).
std::map<std::string, Edge> scanEdges(main::Connection& connection, const RelTable& rel,
    bool fromSource, const VisibleRows& sources, const VisibleRows& destinations,
    std::vector<std::string>& duplicates) {
    const auto pattern =
        fromSource ?
            stringFormat("MATCH (a:{})-[r:{}]->(b) HINT (a JOIN r) JOIN b", rel.source, rel.name) :
            stringFormat("MATCH (a)-[r:{}]->(b:{}) HINT (b JOIN r) JOIN a", rel.name,
                rel.destination);
    const auto query = stringFormat("{} RETURN offset(id(r)), offset(id(a)), offset(id(b)){};",
        pattern, castAll("r", rel.properties));
    std::map<std::string, Edge> edges;
    for (auto& row : rows(connection, query)) {
        Edge edge{row[1], row[2], sources.keyOf(row[1]), destinations.keyOf(row[2]),
            std::vector<std::string>(row.begin() + 3, row.end())};
        if (edges.contains(row[0])) {
            duplicates.push_back(stringFormat("[{}] {} and {}", fromSource ? "forward" : "backward",
                edgeText(row[0], edges.at(row[0])), edgeText(row[0], edge)));
        }
        edges[row[0]] = std::move(edge);
    }
    return edges;
}

// Toutes les relations d'une table, vues des deux côtés, et celles qu'un seul côté voit.
struct EdgeScan {
    std::map<std::string, Edge> all;
    std::vector<std::pair<std::string, std::string>> oneSideOnly;
    std::vector<std::string> duplicateIDs;
};

EdgeScan scanBothSides(main::Connection& connection, const RelTable& rel,
    const VisibleRows& sources, const VisibleRows& destinations) {
    EdgeScan scan;
    const auto forward =
        scanEdges(connection, rel, true /* fromSource */, sources, destinations, scan.duplicateIDs);
    const auto backward = scanEdges(connection, rel, false /* fromSource */, sources, destinations,
        scan.duplicateIDs);
    for (const auto& [offset, edge] : forward) {
        if (!backward.contains(offset)) {
            scan.oneSideOnly.emplace_back("forward only", offset);
        }
    }
    for (const auto& [offset, edge] : backward) {
        if (!forward.contains(offset)) {
            scan.oneSideOnly.emplace_back("backward only", offset);
        }
    }
    scan.all = forward;
    scan.all.insert(backward.begin(), backward.end());
    return scan;
}

void checkRelTable(main::Connection& connection, const RelTable& rel,
    const std::map<std::string, VisibleRows>& visible, std::vector<Violation>& violations) {
    const auto& sources = visible.at(rel.source);
    const auto& destinations = visible.at(rel.destination);
    const auto scan = scanBothSides(connection, rel, sources, destinations);

    if (!scan.duplicateIDs.empty()) {
        std::string detail =
            stringFormat("table {}: {} relation identifiers returned twice:", rel.name,
                scan.duplicateIDs.size());
        for (auto i = 0u; i < scan.duplicateIDs.size() && i < MAX_LISTED; ++i) {
            detail += " " + scan.duplicateIDs[i];
        }
        violations.push_back({"rel-ids-unique", detail});
    }

    if (!scan.oneSideOnly.empty()) {
        std::string detail = stringFormat(
            "table {}: {} relations seen from one side only:", rel.name, scan.oneSideOnly.size());
        for (auto i = 0u; i < scan.oneSideOnly.size() && i < MAX_LISTED; ++i) {
            const auto& [side, offset] = scan.oneSideOnly[i];
            detail += stringFormat(" [{}] {}", side, edgeText(offset, scan.all.at(offset)));
        }
        violations.push_back({"rel-directions-agree", detail});
    }

    std::string dangling;
    size_t numDangling = 0;
    std::string wrong;
    size_t numWrong = 0;
    for (const auto& [offset, edge] : scan.all) {
        const bool sourceVisible = sources.byOffset.contains(edge.sourceOffset);
        const bool destinationVisible = destinations.byOffset.contains(edge.destinationOffset);
        if ((!sourceVisible || !destinationVisible) && ++numDangling <= MAX_LISTED) {
            dangling += stringFormat(" {}{}{}", edgeText(offset, edge),
                sourceVisible ? "" : " source missing",
                destinationVisible ? "" : " destination missing");
        }
        // Une extrémité absente est déjà une violation ; on ne la recompte pas ici.
        if (rel.sourceKeyIndex < 0 || rel.destinationKeyIndex < 0 || !sourceVisible ||
            !destinationVisible) {
            continue;
        }
        const auto& declaredSource = edge.properties[rel.sourceKeyIndex];
        const auto& declaredDestination = edge.properties[rel.destinationKeyIndex];
        if ((declaredSource != edge.sourceKey || declaredDestination != edge.destinationKey) &&
            ++numWrong <= MAX_LISTED) {
            wrong += stringFormat(" {} declared ({} -> {})", edgeText(offset, edge), declaredSource,
                declaredDestination);
        }
    }
    if (numDangling != 0) {
        violations.push_back({"rel-endpoints-exist",
            stringFormat("table {}: {} relations with a missing endpoint:{}", rel.name, numDangling,
                dangling)});
    }
    if (numWrong != 0) {
        violations.push_back({"rel-endpoints-correct",
            stringFormat("table {}: {} relations attached to the wrong nodes:{}", rel.name,
                numWrong, wrong)});
    }
}

std::string join(const std::vector<std::string>& values) {
    std::string out;
    for (const auto& value : values) {
        out += "|" + value;
    }
    return out;
}

} // namespace

std::vector<Violation> checkLevel1(main::Connection& connection) {
    const auto schema = loadSchema(connection);
    std::vector<Violation> violations;
    std::map<std::string, VisibleRows> visible;
    for (const auto& table : schema.nodeTables) {
        visible[table.name] = visibleRows(connection, table);
        checkPrimaryKey(connection, table, violations);
    }
    for (const auto& rel : schema.relTables) {
        checkRelTable(connection, rel, visible, violations);
    }
    return violations;
}

namespace {

// Une relation vue par la CSR dans une direction : offset de relation -> (source,
// destination), toujours dans le sens de la table de relations.
using StoredEdges = std::map<common::offset_t, std::pair<common::offset_t, common::offset_t>>;

StoredEdges scanStoredEdges(transaction::Transaction* transaction, storage::MemoryManager* mm,
    storage::RelTable& relTable, storage::NodeTable& boundTable, common::RelDataDirection direction,
    std::vector<std::string>& duplicates) {
    const auto boundState = common::DataChunkState::getSingleValueDataChunkState();
    const auto outState = std::make_shared<common::DataChunkState>();
    common::ValueVector bound(common::LogicalType::INTERNAL_ID(), mm, boundState);
    common::ValueVector neighbour(common::LogicalType::INTERNAL_ID(), mm, outState);
    common::ValueVector relID(common::LogicalType::INTERNAL_ID(), mm, outState);
    storage::RelTableScanState scanState(*mm, &bound, {&neighbour, &relID}, outState,
        true /* randomLookup */);
    scanState.setToTable(transaction, &relTable,
        {storage::NBR_ID_COLUMN_ID, storage::REL_ID_COLUMN_ID}, {}, direction);
    StoredEdges edges;
    const auto numBound = boundTable.getNumTotalRows(transaction);
    for (common::offset_t offset = 0; offset < numBound; ++offset) {
        bound.setValue<common::nodeID_t>(0, {offset, boundTable.getTableID()});
        relTable.initScanState(transaction, scanState);
        while (relTable.scan(transaction, scanState)) {
            const auto& selection = outState->getSelVector();
            for (auto i = 0u; i < selection.getSelSize(); ++i) {
                const auto position = selection[i];
                const auto other = neighbour.getValue<common::nodeID_t>(position).offset;
                const auto rel = relID.getValue<common::internalID_t>(position).offset;
                const auto ends = direction == common::RelDataDirection::FWD ?
                                      std::pair{offset, other} :
                                      std::pair{other, offset};
                if (const auto it = edges.find(rel); it != edges.end()) {
                    duplicates.push_back(stringFormat("[{}] rel@{} (@{} -> @{}) and (@{} -> @{})",
                        direction == common::RelDataDirection::FWD ? "forward" : "backward", rel,
                        it->second.first, it->second.second, ends.first, ends.second));
                }
                edges[rel] = ends;
            }
        }
    }
    return edges;
}

void checkLevel2InTransaction(main::Connection& connection, const Schema& schema,
    const std::map<std::string, VisibleRows>& visible, std::vector<Violation>& violations) {
    auto* context = connection.getClientContext();
    auto* transaction = transaction::Transaction::Get(*context);
    auto* catalog = catalog::Catalog::Get(*context);
    auto* storageManager = storage::StorageManager::Get(*context);
    auto* mm = storage::MemoryManager::Get(*context);
    std::map<std::string, storage::NodeTable*> nodeTables;

    for (const auto& table : schema.nodeTables) {
        const auto* entry = catalog->getTableCatalogEntry(transaction, table.name);
        auto& nodeTable = storageManager->getTable(entry->getTableID())->cast<storage::NodeTable>();
        nodeTables[table.name] = &nodeTable;
        const auto& rows = visible.at(table.name);

        uint64_t storedVisible = 0;
        const auto numTotal = nodeTable.getNumTotalRows(transaction);
        for (common::offset_t offset = 0; offset < numTotal; ++offset) {
            storedVisible += nodeTable.isVisibleNoLock(transaction, offset) ? 1 : 0;
        }
        if (storedVisible != rows.byOffset.size()) {
            violations.push_back({"visible-rows-agree",
                stringFormat("table {}: the storage sees {} visible rows, Cypher {}", table.name,
                    storedVisible, rows.byOffset.size())});
        }

        common::ValueVector key(common::LogicalType::INT64(), mm,
            common::DataChunkState::getSingleValueDataChunkState());
        std::string mismatches;
        size_t numMismatches = 0;
        for (const auto& [offsetText, values] : rows.byOffset) {
            const auto& keyText = values[rows.primaryKeyIndex];
            int64_t keyValue = 0;
            try {
                keyValue = std::stoll(keyText);
            } catch (const std::exception&) {
                violations.push_back({"level-2-unsupported",
                    stringFormat("table {}: primary key {} is not INT64", table.name, keyText)});
                break;
            }
            key.setValue<int64_t>(0, keyValue);
            common::offset_t found = common::INVALID_OFFSET;
            const bool exists = nodeTable.lookupPK(transaction, &key, 0, found);
            const auto offset = std::stoull(offsetText);
            if ((!exists || found != offset) && ++numMismatches <= MAX_LISTED) {
                mismatches +=
                    exists ?
                        stringFormat(" key {} at @{}: index gives @{}", keyText, offset, found) :
                        stringFormat(" key {} at @{}: not in the index", keyText, offset);
            }
        }
        if (numMismatches != 0) {
            violations.push_back({"primary-key-index-agrees",
                stringFormat("table {}: {} visible rows not found at their own offset by the "
                             "primary key index:{}",
                    table.name, numMismatches, mismatches)});
        }
    }

    for (const auto& rel : schema.relTables) {
        const auto* entry = catalog->getTableCatalogEntry(transaction, rel.name);
        const auto relTableID =
            entry->constCast<catalog::RelGroupCatalogEntry>().getSingleRelEntryInfo().oid;
        auto& relTable = storageManager->getTable(relTableID)->cast<storage::RelTable>();
        auto* source = nodeTables.at(rel.source);
        auto* destination = nodeTables.at(rel.destination);
        std::map<common::RelDataDirection, StoredEdges> byDirection;
        std::vector<std::string> duplicates;
        for (const auto direction : relTable.getStorageDirections()) {
            byDirection[direction] = scanStoredEdges(transaction, mm, relTable,
                direction == common::RelDataDirection::FWD ? *source : *destination, direction,
                duplicates);
        }
        if (!duplicates.empty()) {
            std::string detail = stringFormat(
                "table {}: {} relation identifiers stored twice:", rel.name, duplicates.size());
            for (auto i = 0u; i < duplicates.size() && i < MAX_LISTED; ++i) {
                detail += " " + duplicates[i];
            }
            violations.push_back({"stored-rel-ids-unique", detail});
        }
        if (byDirection.size() == 2) {
            const auto& forward = byDirection.at(common::RelDataDirection::FWD);
            const auto& backward = byDirection.at(common::RelDataDirection::BWD);
            if (forward != backward) {
                std::string detail;
                size_t numDiffering = 0;
                for (const auto& [side, mine, other] : {std::tuple{"forward", &forward, &backward},
                         std::tuple{"backward", &backward, &forward}}) {
                    for (const auto& [relOffset, ends] : *mine) {
                        const auto it = other->find(relOffset);
                        if ((it == other->end() || it->second != ends) &&
                            ++numDiffering <= MAX_LISTED) {
                            detail += stringFormat(" [{}] rel@{} (@{} -> @{})", side, relOffset,
                                ends.first, ends.second);
                        }
                    }
                }
                violations.push_back({"stored-directions-agree",
                    stringFormat("table {}: the two CSR directions differ ({} entries):{}",
                        rel.name, numDiffering, detail)});
            }
        }
        std::string dangling;
        size_t numDangling = 0;
        for (const auto& [direction, edges] : byDirection) {
            for (const auto& [relOffset, ends] : edges) {
                const bool sourceVisible = source->isVisibleNoLock(transaction, ends.first);
                const bool destinationVisible =
                    destination->isVisibleNoLock(transaction, ends.second);
                if ((!sourceVisible || !destinationVisible) && ++numDangling <= MAX_LISTED) {
                    dangling += stringFormat(" [{}] rel@{} (@{} -> @{}){}{}",
                        direction == common::RelDataDirection::FWD ? "forward" : "backward",
                        relOffset, ends.first, ends.second,
                        sourceVisible ? "" : " source invisible",
                        destinationVisible ? "" : " destination invisible");
                }
            }
        }
        if (numDangling != 0) {
            violations.push_back({"stored-endpoints-visible",
                stringFormat("table {}: {} stored relation entries with an invisible "
                             "endpoint:{}",
                    rel.name, numDangling, dangling)});
        }
    }
}

} // namespace

std::vector<Violation> checkLevel2(main::Connection& connection) {
    const auto schema = loadSchema(connection);
    std::map<std::string, VisibleRows> visible;
    for (const auto& table : schema.nodeTables) {
        visible[table.name] = visibleRows(connection, table);
    }
    std::vector<Violation> violations;
    mustQuery(connection, "BEGIN TRANSACTION READ ONLY;");
    try {
        checkLevel2InTransaction(connection, schema, visible, violations);
    } catch (...) {
        connection.query("ROLLBACK;");
        throw;
    }
    mustQuery(connection, "COMMIT;");
    return violations;
}

std::vector<std::string> canonicalDump(main::Connection& connection) {
    const auto schema = loadSchema(connection);
    std::vector<std::string> lines;
    std::map<std::string, VisibleRows> visible;
    for (const auto& table : schema.nodeTables) {
        visible[table.name] = visibleRows(connection, table);
        for (const auto& [offset, values] : visible[table.name].byOffset) {
            lines.push_back("node " + table.name + join(values));
        }
    }
    for (const auto& rel : schema.relTables) {
        const auto scan =
            scanBothSides(connection, rel, visible.at(rel.source), visible.at(rel.destination));
        for (const auto& [offset, edge] : scan.all) {
            lines.push_back("rel " + rel.name + "|" + edge.sourceKey + "->" + edge.destinationKey +
                            join(edge.properties));
        }
    }
    std::sort(lines.begin(), lines.end());
    return lines;
}

std::vector<Violation> compareDumps(const std::vector<std::string>& before,
    const std::vector<std::string>& after, const std::string& beforeName,
    const std::string& afterName) {
    std::vector<std::string> onlyBefore;
    std::vector<std::string> onlyAfter;
    std::set_difference(before.begin(), before.end(), after.begin(), after.end(),
        std::back_inserter(onlyBefore));
    std::set_difference(after.begin(), after.end(), before.begin(), before.end(),
        std::back_inserter(onlyAfter));
    std::vector<Violation> violations;
    for (const auto& [name, lines] :
        {std::pair{beforeName, &onlyBefore}, std::pair{afterName, &onlyAfter}}) {
        if (lines->empty()) {
            continue;
        }
        std::string detail = stringFormat("{} lines only {}:", lines->size(), name);
        for (auto i = 0u; i < lines->size() && i < MAX_LISTED; ++i) {
            detail += " [" + (*lines)[i] + "]";
        }
        violations.push_back({"same-answers", detail});
    }
    return violations;
}

std::string describe(const std::vector<Violation>& violations) {
    if (violations.empty()) {
        return "  no violation\n";
    }
    std::string out;
    for (const auto& violation : violations) {
        out += "  VIOLATION " + violation.invariant + ": " + violation.detail + "\n";
    }
    return out;
}

} // namespace integrity
} // namespace testing
} // namespace rag3db
