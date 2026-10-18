// Un COPY de relations annulé vers un nœud qui a des relations persistantes (ticket du 18 octobre
// 2026 : « un COPY de relations vers un nœud qui a déjà des relations persistantes les fait
// disparaître »).
//
// L'annulation d'une insertion retire les groupes de queue « vides » de la collection
// (NodeGroupCollection::rollbackInsert → GroupCollection::getNumEmptyTrailingGroups). Le vide se
// jugeait sur getNumRows(), les lignes en mémoire seules : un groupe CSR dont toutes les relations
// sont dans son bloc persistant comptait 0 et partait avec elles ; le COPY suivant repartait d'un
// groupe neuf, et le point de reprise rendait la perte durable. Le vide se juge désormais par
// isEmpty(), que CSRNodeGroup définit avec son bloc persistant.

#include <fstream>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class CopyRollbackPersistentRels : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    int64_t scalar(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    // Quatre nœuds, dix relations 0 → 1 (k = 1 … 10), reprises : persistantes.
    void persistentRels(bool withProperty) {
        ok("CREATE NODE TABLE A(id INT64 PRIMARY KEY);");
        ok(withProperty ? "CREATE REL TABLE R(FROM A TO A, k INT64);" :
                          "CREATE REL TABLE R(FROM A TO A);");
        ok("UNWIND range(0, 3) AS i CREATE (:A {id: i});");
        ok(withProperty ? "MATCH (a:A {id: 0}), (b:A {id: 1}) UNWIND range(1, 10) AS i CREATE (a)-[:R {k: i}]->(b);" :
                          "MATCH (a:A {id: 0}), (b:A {id: 1}) UNWIND range(1, 10) AS i CREATE (a)-[:R]->(b);");
        ok("CHECKPOINT;");
    }
    // Un fichier de `count` relations 0 → 2, k = firstK …
    std::string relsCsv(const std::string& name, int64_t firstK, int64_t count, bool withProperty) {
        const auto path = databasePath + "." + name + ".csv";
        std::ofstream csv(path);
        for (int64_t i = 0; i < count; i++) {
            csv << "0,2";
            if (withProperty) {
                csv << "," << firstK + i;
            }
            csv << "\n";
        }
        return path;
    }
    void rolledBackCopy(const std::string& csv) {
        ok("BEGIN TRANSACTION;");
        ok("COPY R FROM '" + csv + "' (header=false);");
        ok("ROLLBACK;");
    }
    // Les relations du nœud 0 et celles vers le nœud 1, tout de suite, après un point de reprise,
    // puis après réouverture.
    void expectRels(int64_t fromNode0, int64_t toNode1) {
        auto check = [&](const std::string& when) {
            EXPECT_EQ(scalar("MATCH (a:A {id: 0})-[r:R]->() RETURN count(r);"), fromNode0) << when;
            EXPECT_EQ(scalar("MATCH ()-[r:R]->(b:A {id: 1}) RETURN count(r);"), toNode1) << when;
        };
        check("tout de suite");
        ok("CHECKPOINT;");
        check("après le point de reprise");
        if (inMemMode) {
            return;
        }
        createDBAndConn();
        check("après réouverture");
    }
};

TEST_F(CopyRollbackPersistentRels, ARolledBackCopyKeepsThePersistentRels) {
    persistentRels(true);
    rolledBackCopy(relsCsv("annulees", 1001, 1500, true));
    expectRels(10, 10);
}

TEST_F(CopyRollbackPersistentRels, ACopyAfterARolledBackCopyAddsToThePersistentRels) {
    persistentRels(true);
    rolledBackCopy(relsCsv("annulees", 1001, 1500, true));
    EXPECT_EQ(scalar("MATCH (a:A {id: 0})-[r:R]->() RETURN count(r);"), 10);
    ok("COPY R FROM '" + relsCsv("gardees", 5001, 1500, true) + "' (header=false);");
    EXPECT_EQ(scalar("MATCH ()-[r:R]->() WHERE r.k <= 10 RETURN count(r);"), 10);
    EXPECT_EQ(scalar("MATCH ()-[r:R]->() WHERE r.k > 1000 AND r.k <= 2500 RETURN count(r);"), 0);
    expectRels(1510, 10);
}

TEST_F(CopyRollbackPersistentRels, ARolledBackCopyFromASubqueryKeepsThePersistentRelsToo) {
    persistentRels(true);
    ok("BEGIN TRANSACTION;");
    ok("COPY R FROM (UNWIND range(1000, 2499) AS k RETURN 0, 2, k);");
    ok("ROLLBACK;");
    expectRels(10, 10);
}

TEST_F(CopyRollbackPersistentRels, ARolledBackCopyWithoutPropertyKeepsThePersistentRelsToo) {
    persistentRels(false);
    rolledBackCopy(relsCsv("annulees", 0, 1500, false));
    expectRels(10, 10);
}

TEST_F(CopyRollbackPersistentRels, ACreateAfterARolledBackCopyAddsToThePersistentRels) {
    persistentRels(true);
    rolledBackCopy(relsCsv("annulees", 1001, 1500, true));
    ok("MATCH (a:A {id: 0}), (c:A {id: 2}) UNWIND range(1, 5) AS i CREATE (a)-[:R {k: 7000 + i}]->(c);");
    expectRels(15, 10);
}

} // namespace
