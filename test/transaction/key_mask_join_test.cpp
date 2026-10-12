// La jointure par clé (page coeur-cpp/14) : `UNWIND $liste AS u MATCH (d:T {clé: u})` ne lisait
// pas d par sa clé mais balayait d entier sous une jointure de hachage sur propriété, sans masque.
// Le masque par clés fait lire à ce balayage les lignes masquées, et elles seules.

#include <sstream>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class KeyMaskJoin : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    std::string profile(const std::string& query) {
        auto result = conn->query("PROFILE " + query);
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        std::string out;
        while (result->isSuccess() && result->hasNext()) {
            out += result->getNext()->toString() + "\n";
        }
        return out;
    }

    // Les lignes sorties d'un opérateur, lues dans le plan dessiné (forme du banc,
    // masked_scan_test.cpp) : la boîte nommée exactement, puis le premier NumOutputTuples sous elle.
    static std::optional<int64_t> outputTuplesOf(const std::string& plan, const std::string& name) {
        std::vector<std::string> lines;
        std::istringstream in(plan);
        for (std::string line; std::getline(in, line);) {
            lines.push_back(line);
        }
        for (auto i = 0u; i < lines.size(); ++i) {
            const auto column = lines[i].find(name);
            if (column == std::string::npos) {
                continue;
            }
            const auto before = column == 0 ? ' ' : lines[i][column - 1];
            const auto after = column + name.size() < lines[i].size() ? lines[i][column + name.size()] : ' ';
            if (std::isalnum(static_cast<unsigned char>(before)) || before == '_' ||
                std::isalnum(static_cast<unsigned char>(after)) || after == '_') {
                continue;
            }
            for (auto j = i + 1; j < lines.size() && j < i + 12; ++j) {
                const auto key = lines[j].find("NumOutputTuples:", column > 20 ? column - 20 : 0);
                if (key != std::string::npos && key < column + name.size() + 20) {
                    return std::stoll(lines[j].substr(key + 16));
                }
            }
        }
        return std::nullopt;
    }

    std::vector<std::string> rows(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        std::vector<std::string> out;
        while (result->isSuccess() && result->hasNext()) {
            out.push_back(result->getNext()->toString());
        }
        return out;
    }
};

} // namespace

// 72 000 lignes, une liste de 117 clés INT64 éparpillées : le balayage sort ≈ 117 lignes, pas
// 72 000 ; les lignes rendues sont celles des clés, dans l'ordre de la liste ; une clé en double
// rend deux fois sa ligne, une clé absente n'en rend aucune.
TEST_F(KeyMaskJoin, AnUnwindOfKeysReadsTheKeysNotTheTable) {
    ok("CREATE NODE TABLE T(id INT64 PRIMARY KEY, v INT64);");
    ok("UNWIND range(0, 71999) AS i CREATE (:T {id: i, v: i * 2});");
    ok("CHECKPOINT;");
    std::string list = "[";
    for (auto k = 0; k < 117; ++k) {
        list += (k ? "," : "") + std::to_string(k * 615);
    }
    list += "]";
    const auto query = "UNWIND " + list + " AS u MATCH (d:T {id: u}) RETURN u, d.v;";
    const auto got = rows(query);
    ASSERT_EQ(got.size(), 117u);
    EXPECT_EQ(got[0], "0|0\n") << "[check: first-row] ";
    EXPECT_EQ(got[116], std::to_string(116 * 615) + "|" + std::to_string(116 * 615 * 2) + "\n")
        << "[check: last-row] ";
    const auto plan = profile(query);
    ASSERT_NE(plan.find("SEMI_MASKER"), std::string::npos)
        << "[check: plan] no semi-masker for the key join\n" << plan;
    const auto read = outputTuplesOf(plan, "SCAN_NODE_TABLE");
    ASSERT_TRUE(read.has_value()) << "[check: plan] no SCAN_NODE_TABLE in the plan\n" << plan;
    EXPECT_LE(*read, 10 * 117) << "[check: scan-reads-the-keys] the scan under the key mask put out "
                               << *read << " rows of 72 000";
    // Doublons et absents.
    const auto dup = rows("UNWIND [615, 615, 123456789] AS u MATCH (d:T {id: u}) RETURN u, d.v;");
    ASSERT_EQ(dup.size(), 2u) << "[check: duplicates-kept-absent-dropped] ";
    EXPECT_EQ(dup[0], "615|1230\n");
    EXPECT_EQ(dup[1], "615|1230\n");
}

// La forme du produit : une liste de chaînes en paramètre, la clé STRING, et un saut derrière la
// clé (`(d)<-[:R]-(m)`), le balayage de d sous le masque.
TEST_F(KeyMaskJoin, AParameterListOfStringKeysWithAHop) {
    ok("CREATE NODE TABLE D(uuid STRING PRIMARY KEY, name STRING);");
    ok("CREATE NODE TABLE M(id INT64 PRIMARY KEY);");
    ok("CREATE REL TABLE R(FROM M TO D);");
    ok("UNWIND range(0, 19999) AS i CREATE (:D {uuid: 'd-' + CAST(i, 'STRING'), name: 'n' + CAST(i, 'STRING')});");
    ok("UNWIND range(0, 999) AS i CREATE (:M {id: i});");
    ok("UNWIND range(0, 999) AS i MATCH (m:M {id: i}), (d:D {uuid: 'd-' + CAST(i * 20, 'STRING')}) CREATE (m)-[:R]->(d);");
    ok("CHECKPOINT;");
    auto prepared = conn->prepare("UNWIND $uuids AS u MATCH (d:D {uuid: u})<-[:R]-(m:M) RETURN u, d.name, m.id;");
    ASSERT_TRUE(prepared->isSuccess()) << prepared->getErrorMessage();
    std::vector<std::unique_ptr<rag3db::common::Value>> keys;
    for (auto k = 0; k < 50; ++k) {
        keys.push_back(std::make_unique<rag3db::common::Value>(
            rag3db::common::LogicalType::STRING(), "d-" + std::to_string(k * 400)));
    }
    auto list = rag3db::common::Value(
        rag3db::common::LogicalType::LIST(rag3db::common::LogicalType::STRING()), std::move(keys));
    auto result = conn->execute(prepared.get(), std::make_pair(std::string("uuids"), list));
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    uint64_t n = 0;
    while (result->hasNext()) {
        const auto t = result->getNext();
        EXPECT_EQ(t->getValue(0)->toString(), "d-" + std::to_string(n * 400));
        EXPECT_EQ(t->getValue(1)->toString(), "n" + std::to_string(n * 400));
        EXPECT_EQ(t->getValue(2)->toString(), std::to_string(n * 20));
        ++n;
    }
    EXPECT_EQ(n, 50u) << "[check: hop-rows] ";
    const auto plan = profile("UNWIND ['d-0', 'd-400', 'd-800'] AS u MATCH (d:D {uuid: u})<-[:R]-(m:M) RETURN u, d.name, m.id;");
    ASSERT_NE(plan.find("SEMI_MASKER"), std::string::npos) << "[check: plan] " << plan;
    const auto read = outputTuplesOf(plan, "SCAN_NODE_TABLE");
    ASSERT_TRUE(read.has_value()) << plan;
    EXPECT_LE(*read, 30) << "[check: scan-reads-the-keys] " << *read << " rows of 20 000\n" << plan;
}
