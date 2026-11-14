// Une projection de graphe rend sa mémoire à son retrait (ticket
// docs/everything-declarative/tickets/2026-11-14-une-projection-de-graphe-retient-sa-memoire-apres-
// son-retrait.md). Dans rag3weaver, `CALL PROJECT_GRAPH_CYPHER('_vf', '<Cypher de 2,9 Mo : un IN
// de 72 652 uuid en clair>')` puis `DROP_PROJECTED_GRAPH` retenait ~3,6 Gio par tour, linéairement.
//
// Les cas départagent, tour par tour, le RSS lu tel quel, puis après malloc_trim(0) :
// - (a) projeter et retirer avec un Cypher court qui rend autant de nœuds ;
// - (b) le même Cypher long, exécuté seul, hors de toute projection ;
// - (c) la séquence de rag3weaver telle quelle ;
// - (p) le même filtre, la liste en paramètre (le témoin plat d'ED).
// Le RSS après malloc_trim dit ce qui est tenu ; l'écart avec le RSS brut, ce que l'allocateur garde.
#include <malloc.h>

#include <cstdio>
#include <fstream>
#include <string>
#include <unordered_map>
#include <vector>

#include "common/types/value/value.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::common;
using namespace rag3db::main;

namespace rag3db {
namespace testing {

namespace {

int64_t rssMib() {
    std::ifstream status("/proc/self/status");
    std::string line;
    while (std::getline(status, line)) {
        if (line.rfind("VmRSS:", 0) == 0) {
            return std::stoll(line.substr(6)) / 1024;
        }
    }
    return -1;
}

} // namespace

class ProjectedGraphMemory : public EmptyDBTest {
protected:
    static constexpr int64_t NUM_PARENTS = 72652;
    static constexpr int64_t NUM_CHILDREN = 119887;
    static constexpr int ROUNDS = 6;

    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        run("CREATE NODE TABLE P(_uuid STRING, PRIMARY KEY(_uuid));");
        run("CREATE NODE TABLE C(id INT64, PRIMARY KEY(id));");
        run("CREATE REL TABLE C_FROM(FROM C TO P);");
        run("COPY P FROM (UNWIND range(0, " + std::to_string(NUM_PARENTS - 1) +
            ") AS x RETURN 'uuid-' + CAST(x AS STRING) + '-0123456789abcdef');");
        run("COPY C FROM (UNWIND range(0, " + std::to_string(NUM_CHILDREN - 1) +
            ") AS x RETURN x);");
        run("COPY C_FROM FROM (UNWIND range(0, " + std::to_string(NUM_CHILDREN - 1) +
            ") AS x RETURN x, 'uuid-' + CAST(x % " + std::to_string(NUM_PARENTS) +
            " AS STRING) + '-0123456789abcdef');");
        std::string list;
        for (int64_t x = 0; x < NUM_PARENTS; x++) {
            list += (x ? ", " : "") + std::string("\\'uuid-") + std::to_string(x) +
                    "-0123456789abcdef\\'";
        }
        quotedList = list;
    }
    void run(const std::string& q) {
        auto r = conn->query(q);
        ASSERT_TRUE(r->isSuccess()) << q.substr(0, 200) << " : " << r->getErrorMessage();
    }
    // Le Cypher projeté de rag3weaver, ses guillemets échappés pour le littéral du CALL.
    std::string longCypher() const {
        return "MATCH (n:C) MATCH (n)-[:C_FROM]->(p:P) WHERE p._uuid IN [" + quotedList +
               "] RETURN n";
    }
    static std::string unescaped(std::string s) {
        std::string out;
        out.reserve(s.size());
        for (size_t i = 0; i < s.size(); i++) {
            if (s[i] == '\\' && i + 1 < s.size() && s[i + 1] == '\'') {
                continue;
            }
            out += s[i];
        }
        return out;
    }
    // Joue `round` ROUNDS fois ; rend la croissance du RSS après malloc_trim, du premier tour au
    // dernier, et imprime les deux RSS de chaque tour.
    template<typename F>
    int64_t growth(const std::string& name, F round) {
        malloc_trim(0);
        const auto start = rssMib();
        int64_t first = -1, last = -1;
        for (auto i = 0; i < ROUNDS; i++) {
            round();
            const auto raw = rssMib();
            malloc_trim(0);
            const auto trimmed = rssMib();
            fprintf(stderr, "[mémoire] %s tour %d : RSS %ld Mio, après malloc_trim %ld Mio\n",
                name.c_str(), i + 1, static_cast<long>(raw), static_cast<long>(trimmed));
            if (i == 0) {
                first = trimmed;
            }
            last = trimmed;
        }
        fprintf(stderr, "[mémoire] %s : départ %ld Mio, croissance du tour 1 au tour %d : %ld Mio\n",
            name.c_str(), static_cast<long>(start), ROUNDS, static_cast<long>(last - first));
        return last - first;
    }
    std::string quotedList;
};

// La borne : six tours qui retiendraient chacun ce que retenait rag3weaver passent 20 Gio ; un tour
// qui ne retient rien laisse quelques dizaines de Mio de bruit.
static constexpr int64_t MAX_GROWTH_MIB = 256;

TEST_F(ProjectedGraphMemory, AShortProjectionGivesBackItsMemory) {
    EXPECT_LT(growth("(a) projection courte",
                  [&] {
                      run("CALL DROP_PROJECTED_GRAPH('_vf', skip_if_not_exists := true);");
                      run("CALL PROJECT_GRAPH_CYPHER('_vf', 'MATCH (n:C) MATCH "
                          "(n)-[:C_FROM]->(p:P) RETURN n');");
                      run("CALL DROP_PROJECTED_GRAPH('_vf', skip_if_not_exists := true);");
                  }),
        MAX_GROWTH_MIB)
        << "[check: a-short-projection-gives-back-its-memory]";
}

TEST_F(ProjectedGraphMemory, TheLongQueryAloneGivesBackItsMemory) {
    EXPECT_LT(growth("(b) requête longue seule",
                  [&] {
                      auto r = conn->query(
                          unescaped("MATCH (n:C) MATCH (n)-[:C_FROM]->(p:P) WHERE p._uuid IN [" +
                                    quotedList + "] RETURN count(n);"));
                      ASSERT_TRUE(r->isSuccess()) << r->getErrorMessage();
                  }),
        MAX_GROWTH_MIB)
        << "[check: the-long-query-alone-gives-back-its-memory]";
}

TEST_F(ProjectedGraphMemory, ALongProjectionGivesBackItsMemory) {
    EXPECT_LT(growth("(c) projection longue",
                  [&] {
                      run("CALL DROP_PROJECTED_GRAPH('_vf', skip_if_not_exists := true);");
                      run("CALL PROJECT_GRAPH_CYPHER('_vf', '" + longCypher() + "');");
                      run("CALL DROP_PROJECTED_GRAPH('_vf', skip_if_not_exists := true);");
                  }),
        MAX_GROWTH_MIB)
        << "[check: a-long-projection-gives-back-its-memory]";
}

TEST_F(ProjectedGraphMemory, TheFilterWithAParameterListGivesBackItsMemory) {
    std::vector<std::unique_ptr<Value>> children;
    for (int64_t x = 0; x < NUM_PARENTS; x++) {
        children.push_back(
            std::make_unique<Value>("uuid-" + std::to_string(x) + "-0123456789abcdef"));
    }
    const Value list(LogicalType::LIST(LogicalType::STRING()), std::move(children));
    auto prepared = conn->prepare("MATCH (n:C) MATCH (n)-[:C_FROM]->(p:P) WHERE p._uuid IN $l "
                                  "RETURN count(n);");
    ASSERT_TRUE(prepared->isSuccess()) << prepared->getErrorMessage();
    EXPECT_LT(growth("(p) liste en paramètre",
                  [&] {
                      std::unordered_map<std::string, std::unique_ptr<Value>> params;
                      params.emplace("l", list.copy());
                      auto r = conn->executeWithParams(prepared.get(), std::move(params));
                      ASSERT_TRUE(r->isSuccess()) << r->getErrorMessage();
                  }),
        MAX_GROWTH_MIB)
        << "[check: the-filter-with-a-parameter-list-gives-back-its-memory]";
}

} // namespace testing
} // namespace rag3db
