// Brouillon de mesure, non livré : quelles formes de « retrouver des nœuds par leur clé
// primaire, par lot » passent par l'index, et lesquelles balaient la table. Pour chaque
// forme : les opérateurs du plan et le temps d'un lot de 300 clés, selon le nombre de
// nœuds (FORMES_NOEUDS).

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <regex>
#include <set>
#include <string>
#include <unordered_map>
#include <vector>

#include "common/types/value/value.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/prepared_statement.h"

using namespace rag3db::testing;
using rag3db::common::LogicalType;
using rag3db::common::StructField;
using rag3db::common::Value;

namespace {

class PkMatchFormsScratch : public EmptyDBTest {
protected:
    static constexpr int64_t BATCH = 300;
    using Params = std::unordered_map<std::string, std::unique_ptr<Value>>;

    int64_t numNodes = 0;

    void SetUp() override {
        EmptyDBTest::SetUp();
        const auto value = std::getenv("FORMES_NOEUDS");
        numNodes = value ? std::atoll(value) : 2000;
        if (std::getenv("FORMES_MEMOIRE")) {
            databasePath = ":memory:";
        }
        fprintf(stderr, "[formes] tampon de la base de test : %lu Mo ; %ld nœuds par table\n",
            static_cast<unsigned long>(systemConfig->bufferPoolSize >> 20),
            static_cast<long>(numNodes));
        createDBAndConn();
        ok("CREATE NODE TABLE Src(_uuid STRING PRIMARY KEY, v INT64);");
        ok("CREATE NODE TABLE Dst(_uuid STRING PRIMARY KEY, v INT64);");
        ok("CREATE REL TABLE REL(FROM Src TO Dst);");
        ok("UNWIND range(0, " + std::to_string(numNodes - 1) +
            ") AS i CREATE (:Src {_uuid: 's' + CAST(i AS STRING), v: 0});");
        ok("UNWIND range(0, " + std::to_string(numNodes - 1) +
            ") AS i CREATE (:Dst {_uuid: 'd' + CAST(i AS STRING), v: 0});");
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query.substr(0, 200) << " : "
                                         << result->getErrorMessage();
    }

    std::string key(char prefix, int64_t turn, int64_t i) const {
        return prefix + std::to_string((turn * 7919 + i * 31) % numNodes);
    }

    static std::unique_ptr<Value> stringList(std::vector<std::string> values) {
        std::vector<std::unique_ptr<Value>> children;
        for (auto& value : values) {
            children.push_back(std::make_unique<Value>(value));
        }
        return std::make_unique<Value>(LogicalType::LIST(LogicalType::STRING()),
            std::move(children));
    }

    std::unique_ptr<Value> uuids(char prefix, int64_t turn) const {
        std::vector<std::string> values;
        for (int64_t i = 0; i < BATCH; ++i) {
            values.push_back(key(prefix, turn, i));
        }
        return stringList(std::move(values));
    }

    std::unique_ptr<Value> items(int64_t turn, bool link) const {
        std::vector<std::unique_ptr<Value>> rows;
        for (int64_t i = 0; i < BATCH; ++i) {
            std::vector<StructField> fields;
            std::vector<std::unique_ptr<Value>> children;
            if (link) {
                fields.emplace_back("from_uuid", LogicalType::STRING());
                fields.emplace_back("to_uuid", LogicalType::STRING());
                children.push_back(std::make_unique<Value>(key('s', turn, i)));
                children.push_back(std::make_unique<Value>(key('d', turn, i)));
            } else {
                fields.emplace_back("_uuid", LogicalType::STRING());
                children.push_back(std::make_unique<Value>(key('s', turn, i)));
            }
            rows.push_back(std::make_unique<Value>(LogicalType::STRUCT(std::move(fields)),
                std::move(children)));
        }
        auto type = LogicalType::LIST(rows[0]->getDataType().copy());
        return std::make_unique<Value>(std::move(type), std::move(rows));
    }

    static std::string operators(const std::string& plan) {
        std::set<std::string> names;
        std::regex op("([A-Z_]{4,})\\[\\d+\\]");
        for (auto it = std::sregex_iterator(plan.begin(), plan.end(), op);
             it != std::sregex_iterator(); ++it) {
            names.insert((*it)[1]);
        }
        std::string out;
        for (auto& name : names) {
            if (name == "RESULT_COLLECTOR" || name == "FLATTEN" || name == "PROJECTION") {
                continue;
            }
            out += name + " ";
        }
        return out;
    }

    template<typename MakeParams>
    void measure(const std::string& name, const std::string& query, MakeParams makeParams) {
        auto explain = conn->prepare("EXPLAIN " + query);
        std::string plan = "(EXPLAIN refusé)";
        if (explain->isSuccess()) {
            auto result = conn->executeWithParams(explain.get(), makeParams(0));
            plan = result->isSuccess() ? operators(result->toString()) : result->getErrorMessage();
        }
        auto prepared = conn->prepare(query);
        if (!prepared->isSuccess()) {
            fprintf(stderr, "[formes] %-28s REFUSÉE : %s\n", name.c_str(),
                prepared->getErrorMessage().substr(0, 150).c_str());
            return;
        }
        static constexpr int64_t TURNS = 10;
        const auto start = std::chrono::steady_clock::now();
        std::string failure;
        for (int64_t turn = 1; turn <= TURNS; ++turn) {
            auto result = conn->executeWithParams(prepared.get(), makeParams(turn));
            if (!result->isSuccess()) {
                failure = result->getErrorMessage().substr(0, 150);
                break;
            }
        }
        const auto ms =
            std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - start)
                .count() /
            TURNS;
        if (!failure.empty()) {
            fprintf(stderr, "[formes] %-28s ÉCHEC : %s | plan : %s\n", name.c_str(),
                failure.c_str(), plan.c_str());
        } else {
            fprintf(stderr, "[formes] %-28s %8.1f ms par lot de %ld | plan : %s\n", name.c_str(),
                ms, static_cast<long>(BATCH), plan.c_str());
        }
    }
};

TEST_F(PkMatchFormsScratch, Measure) {
    auto one = [&](const char* name, std::unique_ptr<Value> value) {
        Params params;
        params[name] = std::move(value);
        return params;
    };
    // Une clé par appel : la référence, la recherche par l'index.
    {
        auto prepared = conn->prepare("MATCH (n:Src {_uuid: $u}) RETURN n.v;");
        ASSERT_TRUE(prepared->isSuccess()) << prepared->getErrorMessage();
        const auto start = std::chrono::steady_clock::now();
        for (int64_t i = 0; i < BATCH; ++i) {
            auto result = conn->executeWithParams(prepared.get(),
                one("u", std::make_unique<Value>(key('s', 1, i))));
            ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        }
        fprintf(stderr, "[formes] %-28s %8.1f ms pour %ld appels\n", "une clé par appel",
            std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - start)
                .count(),
            static_cast<long>(BATCH));
    }
    measure("structure, lecture",
        "UNWIND $items AS item MATCH (n:Src {_uuid: item._uuid}) RETURN count(n);",
        [&](int64_t turn) { return one("items", items(turn, false)); });
    measure("structure, SET",
        "UNWIND $items AS item MATCH (n:Src {_uuid: item._uuid}) SET n.v = 1;",
        [&](int64_t turn) { return one("items", items(turn, false)); });
    measure("valeurs simples, lecture",
        "UNWIND $uuids AS u MATCH (n:Src {_uuid: u}) RETURN count(n);",
        [&](int64_t turn) { return one("uuids", uuids('s', turn)); });
    measure("valeurs simples, WHERE",
        "UNWIND $uuids AS u MATCH (n:Src) WHERE n._uuid = u RETURN count(n);",
        [&](int64_t turn) { return one("uuids", uuids('s', turn)); });
    measure("valeurs simples, SET",
        "UNWIND $uuids AS u MATCH (n:Src {_uuid: u}) SET n.v = 2;",
        [&](int64_t turn) { return one("uuids", uuids('s', turn)); });
    measure("IN $uuids, lecture", "MATCH (n:Src) WHERE n._uuid IN $uuids RETURN count(n);",
        [&](int64_t turn) { return one("uuids", uuids('s', turn)); });
    measure("IN $uuids, SET", "MATCH (n:Src) WHERE n._uuid IN $uuids SET n.v = 3;",
        [&](int64_t turn) { return one("uuids", uuids('s', turn)); });
    auto two = [&](int64_t turn) {
        Params params;
        params["froms"] = uuids('s', turn);
        params["tos"] = uuids('d', turn);
        return params;
    };
    measure("liens, structure (actuelle)",
        "UNWIND $items AS item MATCH (a:Src {_uuid: item.from_uuid}), (b:Dst {_uuid: "
        "item.to_uuid}) MERGE (a)-[r:REL]->(b);",
        [&](int64_t turn) { return one("items", items(turn, true)); });
    measure("liens, listes parallèles",
        "UNWIND range(1, size($froms)) AS i MATCH (a:Src {_uuid: $froms[i]}), (b:Dst {_uuid: "
        "$tos[i]}) MERGE (a)-[r:REL]->(b);",
        two);
    measure("liens, deux MATCH enchaînés",
        "UNWIND $items AS item MATCH (a:Src {_uuid: item.from_uuid}) WITH a, item MATCH (b:Dst "
        "{_uuid: item.to_uuid}) MERGE (a)-[r:REL]->(b);",
        [&](int64_t turn) { return one("items", items(turn, true)); });
    measure("liens, WITH f,t ; deux MATCH",
        "UNWIND $items AS item WITH item.from_uuid AS f, item.to_uuid AS t MATCH (a:Src {_uuid: "
        "f}) MATCH (b:Dst {_uuid: t}) MERGE (a)-[r:REL]->(b);",
        [&](int64_t turn) { return one("items", items(turn, true)); });
    measure("liens, WITH f,t ; WITH a,t",
        "UNWIND $items AS item WITH item.from_uuid AS f, item.to_uuid AS t MATCH (a:Src {_uuid: "
        "f}) WITH a, t MATCH (b:Dst {_uuid: t}) MERGE (a)-[r:REL]->(b);",
        [&](int64_t turn) { return one("items", items(turn, true)); });
    measure("liens, WITH f,t ; CREATE",
        "UNWIND $items AS item WITH item.from_uuid AS f, item.to_uuid AS t MATCH (a:Src {_uuid: "
        "f}) WITH a, t MATCH (b:Dst {_uuid: t}) CREATE (a)-[r:REL]->(b);",
        [&](int64_t turn) { return one("items", items(turn, true)); });
    measure("nœuds, WITH u ; SET",
        "UNWIND $items AS item WITH item._uuid AS u MATCH (n:Src {_uuid: u}) SET n.v = 4;",
        [&](int64_t turn) { return one("items", items(turn, false)); });
}

} // namespace
