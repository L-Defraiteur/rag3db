#include "integrity/integrity_checker.h"

#include <algorithm>
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

// Balaye les relations en partant d'un côté. Le WITH oblige le plan à lier d'abord ce
// côté, puis à étendre vers l'autre (vérifié par EXPLAIN : SCAN_REL_TABLE depuis le
// côté lié). Une relation dont ce côté est invisible n'est pas vue, ce qui est le but :
// comparer les deux sens fait sortir les relations pendantes.
std::map<std::string, Edge> scanEdges(main::Connection& connection, const RelTable& rel,
    bool fromSource, const VisibleRows& sources, const VisibleRows& destinations) {
    const auto pattern = fromSource ? stringFormat("MATCH (a:{}) WITH a MATCH (a)-[r:{}]->(b:{})",
                                          rel.source, rel.name, rel.destination) :
                                      stringFormat("MATCH (b:{}) WITH b MATCH (a:{})-[r:{}]->(b)",
                                          rel.destination, rel.source, rel.name);
    const auto query = stringFormat("{} RETURN offset(id(r)), offset(id(a)), offset(id(b)){};",
        pattern, castAll("r", rel.properties));
    std::map<std::string, Edge> edges;
    for (auto& row : rows(connection, query)) {
        Edge edge{row[1], row[2], sources.keyOf(row[1]), destinations.keyOf(row[2]),
            std::vector<std::string>(row.begin() + 3, row.end())};
        edges[row[0]] = std::move(edge);
    }
    return edges;
}

// Toutes les relations d'une table, vues des deux côtés, et celles qu'un seul côté voit.
struct EdgeScan {
    std::map<std::string, Edge> all;
    std::vector<std::pair<std::string, std::string>> oneSideOnly;
};

EdgeScan scanBothSides(main::Connection& connection, const RelTable& rel,
    const VisibleRows& sources, const VisibleRows& destinations) {
    const auto forward = scanEdges(connection, rel, true /* fromSource */, sources, destinations);
    const auto backward = scanEdges(connection, rel, false /* fromSource */, sources, destinations);
    EdgeScan scan;
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

std::string edgeText(const std::string& relOffset, const Edge& edge) {
    return stringFormat("rel@{} ({}@{} -> {}@{})", relOffset, edge.sourceKey, edge.sourceOffset,
        edge.destinationKey, edge.destinationOffset);
}

void checkRelTable(main::Connection& connection, const RelTable& rel,
    const std::map<std::string, VisibleRows>& visible, std::vector<Violation>& violations) {
    const auto& sources = visible.at(rel.source);
    const auto& destinations = visible.at(rel.destination);
    const auto scan = scanBothSides(connection, rel, sources, destinations);

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
