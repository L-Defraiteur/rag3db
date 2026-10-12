// La jointure par clé (page coeur-cpp/14) : `UNWIND $liste AS u MATCH (d:T {clé: u})` ne lisait
// pas d par sa clé mais balayait d entier sous une jointure de hachage sur propriété, sans masque.
// Le masque par clés fait lire à ce balayage les lignes masquées, et elles seules.

#include <set>
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
        // 60 000 relations créées en une instruction demandent plus que le tampon du gabarit.
        systemConfig->bufferPoolSize = 512 * 1024 * 1024;
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
// clé (`(d)<-[:R]-(m)`) dont le voisin m est grand et lu par ses propriétés — le planificateur
// le joint alors par hachage sur m._ID, avec le balayage de M côté build (la forme du saut
// d'usages de F : Symbol ← MENTIONS ← Scope) ; les deux balayages sont sous un masque.
TEST_F(KeyMaskJoin, AParameterListOfStringKeysWithAHop) {
    ok("CREATE NODE TABLE D(uuid STRING PRIMARY KEY, name STRING);");
    ok("CREATE NODE TABLE M(id INT64 PRIMARY KEY, name STRING, path STRING, line INT64);");
    ok("CREATE REL TABLE R(FROM M TO D);");
    ok("UNWIND range(0, 19999) AS i CREATE (:D {uuid: 'd-' + CAST(i, 'STRING'), name: 'n' + CAST(i, 'STRING')});");
    ok("UNWIND range(0, 59999) AS i CREATE (:M {id: i, name: 'm' + CAST(i, 'STRING'), path: 'p/' + CAST(i % 97, 'STRING'), line: i % 1000});");
    // Un nœud par MATCH : deux nœuds liés par leur clé dans un même motif se croisent (le produit
    // des deux ensembles masqués) avant la jointure avec la liste — un travers du planificateur,
    // hors de ce lot ; et une clé calculée dans le motif ne devient pas une condition de jointure.
    for (auto low = 0; low < 60000; low += 10000) {
        ok("UNWIND range(" + std::to_string(low) + ", " + std::to_string(low + 9999) +
           ") AS i MATCH (m:M {id: i}) WITH m, 'd-' + CAST(m.id % 20000, 'STRING') AS kd MATCH (d:D {uuid: kd}) CREATE (m)-[:R]->(d);");
    }
    ok("CHECKPOINT;");
    ASSERT_EQ(rows("MATCH ()-[r:R]->() RETURN count(r);")[0], "60000\n") << "[check: edges-created] ";
    auto prepared = conn->prepare("UNWIND $uuids AS u MATCH (d:D {uuid: u})<-[:R]-(m:M) RETURN u, d.name, m.id, m.name, m.path, m.line;");
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
    // Chaque d de la liste a trois voisins m (i, i + 20 000, i + 40 000).
    uint64_t n = 0;
    std::set<std::string> seen;
    while (result->hasNext()) {
        const auto t = result->getNext();
        seen.insert(t->getValue(0)->toString() + "/" + t->getValue(2)->toString());
        EXPECT_EQ(t->getValue(1)->toString(), "n" + t->getValue(0)->toString().substr(2));
        EXPECT_EQ(t->getValue(3)->toString(), "m" + t->getValue(2)->toString());
        ++n;
    }
    EXPECT_EQ(n, 150u) << "[check: hop-rows] ";
    EXPECT_EQ(seen.size(), 150u) << "[check: hop-rows-distinct] ";
    const auto plan = profile("UNWIND ['d-0', 'd-400', 'd-800'] AS u MATCH (d:D {uuid: u})<-[:R]-(m:M) RETURN u, d.name, m.id, m.name, m.path, m.line;");
    ASSERT_NE(plan.find("SEMI_MASKER"), std::string::npos) << "[check: plan] " << plan;
    const auto read = outputTuplesOf(plan, "SCAN_NODE_TABLE");
    ASSERT_TRUE(read.has_value()) << plan;
    EXPECT_LE(*read, 90) << "[check: scan-reads-the-keys] " << *read << " rows\n" << plan;
    // Le second niveau : le voisin m est masqué à son tour par les d trouvés (deux masqueurs
    // dans le plan), au lieu d'être balayé entier.
    size_t maskers = 0;
    for (auto at = plan.find("SEMI_MASKER"); at != std::string::npos;
         at = plan.find("SEMI_MASKER", at + 11)) {
        ++maskers;
    }
    EXPECT_GE(maskers, 2u) << "[check: second-level-masked] " << maskers << " semi-masker(s)\n" << plan;
}

// Le saut d'une table vers elle-même : `(d:S {clé: u})-[:P]->(m:S)`. À cardinalités égales,
// l'énumérateur partait de m (non masqué) et joignait d par hachage : S entier était balayé pour
// m, même pour une clé. Le nœud lié par sa clé à la partie précédente s'estime à la cardinalité de
// cette partie : l'extension part de d masqué, et m n'est lu que par ses voisins.
TEST_F(KeyMaskJoin, AHopWithinTheSameTableStartsFromTheKeyedNode) {
    ok("CREATE NODE TABLE S(uuid STRING PRIMARY KEY, name STRING, path STRING, line INT64);");
    ok("CREATE REL TABLE P(FROM S TO S);");
    ok("UNWIND range(0, 59999) AS i CREATE (:S {uuid: 's-' + CAST(i, 'STRING'), name: 'n' + CAST(i, 'STRING'), path: 'p/' + CAST(i % 97, 'STRING'), line: i % 1000});");
    for (auto low = 0; low < 60000; low += 10000) {
        ok("UNWIND range(" + std::to_string(low) + ", " + std::to_string(low + 9999) +
           ") AS i WITH 's-' + CAST(i, 'STRING') AS ka, 's-' + CAST((i * 7 + 1) % 60000, 'STRING') AS kb MATCH (a:S {uuid: ka}) WITH a, kb MATCH (b:S {uuid: kb}) CREATE (a)-[:P]->(b);");
    }
    ASSERT_EQ(rows("MATCH ()-[p:P]->() RETURN count(p);")[0], "60000\n") << "[check: edges-created] ";
    ok("CHECKPOINT;");
    const std::string query = "UNWIND ['s-0', 's-400', 's-800'] AS u MATCH (d:S {uuid: u})-[:P]->(m:S) RETURN u, m.uuid, m.name, m.path, m.line;";
    const auto got = rows(query);
    ASSERT_EQ(got.size(), 3u) << "[check: hop-rows] ";
    EXPECT_EQ(got[0], "s-0|s-1|n1|p/1|1\n") << "[check: first-row] ";
    const auto plan = profile(query);
    ASSERT_NE(plan.find("SEMI_MASKER"), std::string::npos) << "[check: plan] " << plan;
    // Aucun balayage de S ne sort plus de 10 × 3 lignes : ni d (masqué), ni m (lu par l'extension).
    std::istringstream in(plan);
    std::vector<std::string> lines;
    for (std::string line; std::getline(in, line);) {
        lines.push_back(line);
    }
    int64_t worst = 0;
    for (auto i = 0u; i < lines.size(); ++i) {
        for (auto at = lines[i].find("SCAN_NODE_TABLE["); at != std::string::npos;
             at = lines[i].find("SCAN_NODE_TABLE[", at + 1)) {
            for (auto j = i + 1; j < lines.size() && j < i + 14; ++j) {
                const auto key = lines[j].find("NumOutputTuples:", at > 20 ? at - 20 : 0);
                if (key != std::string::npos && key < at + 40) {
                    worst = std::max<int64_t>(worst, std::stoll(lines[j].substr(key + 16)));
                    break;
                }
            }
        }
    }
    EXPECT_LE(worst, 30) << "[check: extend-starts-from-the-keyed-node] a scan of S put out " << worst
                         << " rows for 3 keys\n" << plan;
}

