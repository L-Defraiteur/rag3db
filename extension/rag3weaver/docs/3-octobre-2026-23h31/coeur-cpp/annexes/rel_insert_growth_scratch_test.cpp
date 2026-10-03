// Brouillon de mesure, non livré : le temps d'un lot de relations de taille fixe à mesure
// que la table de relations grossit. La requête est celle de rag3weaver
// (batch_link_labeled) : UNWIND d'une liste de paires, MATCH par clé primaire des deux
// côtés, MERGE ou CREATE de la relation. Variables d'environnement :
//   CROISSANCE_VERBE=MERGE|CREATE   (défaut MERGE)
//   CROISSANCE_FORME=epars|moyeu    (défaut epars ; moyeu : toutes les relations visent
//                                    un petit nombre de nœuds très connectés)
//   CROISSANCE_MEMOIRE=1            (base en mémoire)
//   CROISSANCE_LOTS, CROISSANCE_TAILLE (défaut 25 lots de 4000)

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <string>

#include "graph_test/private_graph_test.h"
#include <unordered_map>
#include <vector>

#include "common/types/value/value.h"
#include "main/connection.h"
#include "main/prepared_statement.h"

using namespace rag3db::testing;

namespace {

std::string env(const char* name, const std::string& fallback) {
    const auto value = std::getenv(name);
    return value ? value : fallback;
}

class RelInsertGrowthScratch : public EmptyDBTest {
protected:
    static inline const int64_t NUM_NODES = std::atoll(env("CROISSANCE_NOEUDS", "20000").c_str());

    void SetUp() override {
        EmptyDBTest::SetUp();
        if (std::getenv("CROISSANCE_MEMOIRE")) {
            databasePath = ":memory:";
        }
        createDBAndConn();
        ok("CREATE NODE TABLE Src(_uuid STRING PRIMARY KEY);");
        ok("CREATE NODE TABLE Dst(_uuid STRING PRIMARY KEY);");
        ok("CREATE REL TABLE REL(FROM Src TO Dst);");
        ok("CREATE REL TABLE REL2(FROM Src TO Dst);");
        ok("CREATE REL TABLE REL3(FROM Dst TO Src);");
        ok("UNWIND range(0, " + std::to_string(NUM_NODES - 1) +
            ") AS i CREATE (:Src {_uuid: 's' + CAST(i AS STRING)});");
        ok("UNWIND range(0, " + std::to_string(NUM_NODES - 1) +
            ") AS i CREATE (:Dst {_uuid: 'd' + CAST(i AS STRING)});");
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess())
            << query.substr(0, 200) << " : " << result->getErrorMessage();
    }
};

using rag3db::common::LogicalType;
using rag3db::common::StructField;
using rag3db::common::Value;

// La liste passe en paramètre, comme dans rag3weaver ; lots de quelques centaines.
TEST_F(RelInsertGrowthScratch, Measure) {
    const auto verb = env("CROISSANCE_VERBE", "MERGE");
    const auto hub = env("CROISSANCE_FORME", "epars") == "moyeu";
    const auto numBatches = std::atoll(env("CROISSANCE_LOTS", "400").c_str());
    const auto batchSize = std::atoll(env("CROISSANCE_TAILLE", "300").c_str());
    const auto window = std::atoll(env("CROISSANCE_FENETRE", "40").c_str());
    // CROISSANCE_REQUETE=with : les champs sortis de la structure avant le MATCH.
    const auto head = env("CROISSANCE_REQUETE", "struct") == "with" ?
        std::string("UNWIND $items AS item WITH item.from_uuid AS f, item.to_uuid AS t "
                    "MATCH (a:Src {_uuid: f}), (b:Dst {_uuid: t}) ") :
        std::string("UNWIND $items AS item MATCH (a:Src {_uuid: item.from_uuid}), "
                    "(b:Dst {_uuid: item.to_uuid}) ");
    auto prepared = conn->prepare(head + verb + " (a)-[r:REL]->(b);");
    {
        auto plan = conn->query("EXPLAIN UNWIND [{from_uuid: 's1', to_uuid: 'd1'}] AS item " +
                                head.substr(head.find("AS item") + 8) + verb + " (a)-[r:REL]->(b);");
        ASSERT_TRUE(plan->isSuccess()) << plan->getErrorMessage();
        fprintf(stderr, "[plan-forme]\n%s\n", plan->toString().c_str());
    }
    ASSERT_TRUE(prepared->isSuccess()) << prepared->getErrorMessage();
    for (const std::string variant : {"MERGE (a)-[r:REL]->(b)", "RETURN count(*)"}) {
        auto plan = conn->query(
            "EXPLAIN UNWIND [{from_uuid: 's1', to_uuid: 'd1'}, {from_uuid: 's2', to_uuid: 'd2'}] "
            "AS item MATCH (a:Src {_uuid: item.from_uuid}), (b:Dst {_uuid: item.to_uuid}) " +
            variant + ";");
        ASSERT_TRUE(plan->isSuccess()) << plan->getErrorMessage();
        fprintf(stderr, "[plan] %s\n%s\n", variant.c_str(), plan->toString().c_str());
    }
    int64_t next = 0;
    double windowMs = 0;
    for (int64_t batch = 0; batch < numBatches; ++batch) {
        std::vector<std::unique_ptr<Value>> rows;
        for (int64_t i = 0; i < batchSize; ++i, ++next) {
            const auto from = (next * 7919) % NUM_NODES;
            const auto to = hub ? (next % 20) : ((next * 104729 + next / NUM_NODES) % NUM_NODES);
            std::vector<StructField> fields;
            fields.emplace_back("from_uuid", LogicalType::STRING());
            fields.emplace_back("to_uuid", LogicalType::STRING());
            std::vector<std::unique_ptr<Value>> children;
            children.push_back(std::make_unique<Value>("s" + std::to_string(from)));
            children.push_back(std::make_unique<Value>("d" + std::to_string(to)));
            rows.push_back(std::make_unique<Value>(LogicalType::STRUCT(std::move(fields)),
                std::move(children)));
        }
        auto type = LogicalType::LIST(rows[0]->getDataType().copy());
        std::unordered_map<std::string, std::unique_ptr<Value>> params;
        params["items"] = std::make_unique<Value>(std::move(type), std::move(rows));
        const auto start = std::chrono::steady_clock::now();
        auto result = conn->executeWithParams(prepared.get(), std::move(params));
        windowMs +=
            std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - start)
                .count();
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        if ((batch + 1) % window == 0) {
            fprintf(stderr, "[croissance] %s %s lots %3ld-%3ld (%ld relations déjà) : %.1f ms par lot de %ld\n",
                verb.c_str(), hub ? "moyeu" : "epars", static_cast<long>(batch + 1 - window),
                static_cast<long>(batch), static_cast<long>((batch + 1 - window) * batchSize),
                windowMs / window, static_cast<long>(batchSize));
            windowMs = 0;
        }
    }
    auto count = conn->query("MATCH ()-[r:REL]->() RETURN count(*);");
    ASSERT_TRUE(count->isSuccess());
    fprintf(stderr, "[croissance] total %ld relations\n",
        static_cast<long>(count->getNext()->getValue(0)->getValue<int64_t>()));
}

} // namespace
