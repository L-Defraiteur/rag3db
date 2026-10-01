#include "integrity/integrity_checker.h"

#include <map>
#include <stdexcept>

#include "common/string_format.h"
#include "processor/result/flat_tuple.h"

using namespace rag3db::common;

namespace rag3db {
namespace testing {
namespace integrity {

namespace {

// Au-delà, une violation est résumée : la sortie brute doit rester lisible.
constexpr size_t MAX_LISTED = 10;

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
};

struct RelTable {
    std::string name;
    std::string source;
    std::string destination;
    std::string sourcePrimaryKey;
    std::string destinationPrimaryKey;
    bool carriesEndpointKeys = false;
};

// Une relation telle qu'un balayage la rend, avec les clés lues sur ses extrémités.
struct Edge {
    std::string sourceOffset;
    std::string destinationOffset;
    std::string sourceKey;
    std::string destinationKey;
    std::string declaredSourceKey;
    std::string declaredDestinationKey;
};

std::string primaryKeyOf(main::Connection& connection, const std::string& table) {
    for (const auto& row : rows(connection,
             stringFormat("CALL table_info('{}') RETURN name, `primary key`;", table))) {
        if (row[1] == "True") {
            return row[0];
        }
    }
    return "";
}

bool hasProperty(main::Connection& connection, const std::string& table,
    const std::string& property) {
    for (const auto& row :
        rows(connection, stringFormat("CALL table_info('{}') RETURN name;", table))) {
        if (row[0] == property) {
            return true;
        }
    }
    return false;
}

// Les lignes visibles d'une table de nœuds : offset -> clé primaire.
std::map<std::string, std::string> visibleRows(main::Connection& connection,
    const NodeTable& table) {
    std::map<std::string, std::string> out;
    for (const auto& row :
        rows(connection, stringFormat("MATCH (n:{}) RETURN offset(id(n)), CAST(n.{} AS STRING);",
                             table.name, table.primaryKey))) {
        out[row[0]] = row[1];
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

// Balaye les relations en partant d'un côté. Le WITH oblige le plan à lier d'abord ce
// côté, puis à étendre vers l'autre (vérifié par EXPLAIN : SCAN_REL_TABLE depuis le
// côté lié). Une relation dont ce côté est invisible n'est pas vue, ce qui est le but :
// comparer les deux sens fait sortir les relations pendantes.
// On ne lit aucune propriété de l'autre extrémité : y accéder joint sa table de nœuds
// et efface en silence une relation dont l'extrémité est supprimée. Les clés des
// extrémités viennent des lignes visibles, lues à part.
std::map<std::string, Edge> scanEdges(main::Connection& connection, const RelTable& rel,
    bool fromSource, const std::map<std::string, std::string>& sourceKeys,
    const std::map<std::string, std::string>& destinationKeys) {
    const auto declared = rel.carriesEndpointKeys ?
                              stringFormat(", CAST(r.{} AS STRING), CAST(r.{} AS STRING)",
                                  SOURCE_KEY_PROPERTY, DESTINATION_KEY_PROPERTY) :
                              std::string(", '', ''");
    const auto pattern = fromSource ? stringFormat("MATCH (a:{}) WITH a MATCH (a)-[r:{}]->(b:{})",
                                          rel.source, rel.name, rel.destination) :
                                      stringFormat("MATCH (b:{}) WITH b MATCH (a:{})-[r:{}]->(b)",
                                          rel.destination, rel.source, rel.name);
    const auto query =
        stringFormat("{} RETURN offset(id(r)), offset(id(a)), offset(id(b)){};", pattern, declared);
    const auto keyOf = [](const std::map<std::string, std::string>& keys,
                           const std::string& offset) {
        const auto it = keys.find(offset);
        return it == keys.end() ? std::string("<missing>") : it->second;
    };
    std::map<std::string, Edge> edges;
    for (auto& row : rows(connection, query)) {
        edges[row[0]] = Edge{row[1], row[2], keyOf(sourceKeys, row[1]),
            keyOf(destinationKeys, row[2]), row[3], row[4]};
    }
    return edges;
}

std::string edgeText(const std::string& relOffset, const Edge& edge) {
    return stringFormat("rel@{} ({}@{} -> {}@{})", relOffset, edge.sourceKey, edge.sourceOffset,
        edge.destinationKey, edge.destinationOffset);
}

void checkRelTable(main::Connection& connection, const RelTable& rel,
    const std::map<std::string, std::map<std::string, std::string>>& visible,
    std::vector<Violation>& violations) {
    const auto& sourceKeys = visible.at(rel.source);
    const auto& destinationKeys = visible.at(rel.destination);
    const auto forward =
        scanEdges(connection, rel, true /* fromSource */, sourceKeys, destinationKeys);
    const auto backward =
        scanEdges(connection, rel, false /* fromSource */, sourceKeys, destinationKeys);

    std::string onlyOneSide;
    size_t numOnlyOneSide = 0;
    for (const auto& [side, mine, other] : {std::tuple{"forward only", &forward, &backward},
             std::tuple{"backward only", &backward, &forward}}) {
        for (const auto& [offset, edge] : *mine) {
            if (other->contains(offset)) {
                continue;
            }
            if (++numOnlyOneSide <= MAX_LISTED) {
                onlyOneSide += stringFormat(" [{}] {}", side, edgeText(offset, edge));
            }
        }
    }
    if (numOnlyOneSide != 0) {
        violations.push_back({"rel-directions-agree",
            stringFormat("table {}: {} relations seen from one side only:{}", rel.name,
                numOnlyOneSide, onlyOneSide)});
    }

    std::map<std::string, Edge> all = forward;
    all.insert(backward.begin(), backward.end());
    std::string dangling;
    size_t numDangling = 0;
    std::string wrong;
    size_t numWrong = 0;
    for (const auto& [offset, edge] : all) {
        const bool sourceVisible = sourceKeys.contains(edge.sourceOffset);
        const bool destinationVisible = destinationKeys.contains(edge.destinationOffset);
        if ((!sourceVisible || !destinationVisible) && ++numDangling <= MAX_LISTED) {
            dangling += stringFormat(" {}{}{}", edgeText(offset, edge),
                sourceVisible ? "" : " source missing",
                destinationVisible ? "" : " destination missing");
        }
        // Une extrémité absente est déjà une violation ; on ne la recompte pas ici.
        if (rel.carriesEndpointKeys && sourceVisible && destinationVisible &&
            (edge.declaredSourceKey != edge.sourceKey ||
                edge.declaredDestinationKey != edge.destinationKey)) {
            if (++numWrong <= MAX_LISTED) {
                wrong += stringFormat(" {} declared ({} -> {})", edgeText(offset, edge),
                    edge.declaredSourceKey, edge.declaredDestinationKey);
            }
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

} // namespace

std::vector<Violation> checkLevel1(main::Connection& connection) {
    std::vector<NodeTable> nodeTables;
    std::vector<RelTable> relTables;
    for (const auto& row : rows(connection, "CALL show_tables() RETURN name, type;")) {
        if (row[1] == "NODE") {
            nodeTables.push_back({row[0], primaryKeyOf(connection, row[0])});
        } else if (row[1] == "REL") {
            relTables.push_back({row[0]});
        }
    }
    for (auto& rel : relTables) {
        const auto connections =
            rows(connection, stringFormat("CALL show_connection('{}') RETURN `source table name`, "
                                          "`destination table name`, `source table primary key`, "
                                          "`destination table primary key`;",
                                 rel.name));
        // Une table de relations à plusieurs paires de tables n'est pas dans le banc.
        if (connections.size() != 1) {
            throw std::runtime_error("integrity checker: rel table " + rel.name +
                                     " has several connections, not supported");
        }
        rel.source = connections[0][0];
        rel.destination = connections[0][1];
        rel.sourcePrimaryKey = connections[0][2];
        rel.destinationPrimaryKey = connections[0][3];
        rel.carriesEndpointKeys = hasProperty(connection, rel.name, SOURCE_KEY_PROPERTY) &&
                                  hasProperty(connection, rel.name, DESTINATION_KEY_PROPERTY);
    }

    std::vector<Violation> violations;
    std::map<std::string, std::map<std::string, std::string>> visible;
    for (const auto& table : nodeTables) {
        visible[table.name] = visibleRows(connection, table);
        if (!table.primaryKey.empty()) {
            checkPrimaryKey(connection, table, violations);
        }
    }
    for (const auto& rel : relTables) {
        checkRelTable(connection, rel, visible, violations);
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
