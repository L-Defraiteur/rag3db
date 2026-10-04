// Un défaut du moteur d'origine, sans index vectoriel ni concurrence, que la mise à jour
// de l'index HNSW a révélé (session cœur C++, 4 octobre ; livré aussi de son côté dans
// test/transaction/uncommitted_rels_scan_test.cpp). Un seul écrivain.
//
// Une transaction supprime des relations d'un nœud qui sont sur disque (un CHECKPOINT
// après leur création : en mémoire, le défaut ne se voit pas), en crée de nouvelles depuis
// ce même nœud, puis relit : elle reçoit la même relation locale répétée à la place des
// nouvelles. Si elle écrit d'après ce qu'elle a relu, la base reste fausse après le
// COMMIT. Cause désignée par le cœur C++ : LocalRelTable::scan ; le même défaut est
// corrigé chez Ladybug (ffe855873, 23 avril 2026).
//
// Rouges trois passes sur trois avant c8fdaf196, verts trois passes sur trois après
// (4 octobre) : ils restent comme témoins verts.

#ifndef __SINGLE_THREADED__

#include <algorithm>
#include <vector>

#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "integrity/integrity_checker.h"
#include "processor/result/flat_tuple.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

class UncommittedRelations : public EmptyDBTest {
public:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
        mustRun("CALL auto_checkpoint=false;");
    }

    void mustRun(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << "[check: query] " << query << "\n"
                                         << result->getErrorMessage();
    }

    // 200 nœuds ; le nœud 0 relié aux nœuds 1 à 60 ; sur disque si onDisk.
    void createHub(bool onDisk) {
        mustRun("CREATE NODE TABLE N(id INT64 PRIMARY KEY);");
        mustRun("CREATE REL TABLE E(FROM N TO N);");
        mustRun("UNWIND range(0, 199) AS i CREATE (:N {id: i});");
        mustRun("MATCH (a:N {id: 0}), (b:N) WHERE b.id >= 1 AND b.id <= 60 CREATE (a)-[:E]->(b);");
        if (onDisk) {
            mustRun("CHECKPOINT;");
        }
    }

    void deleteEvenAndCreateNew() {
        mustRun("MATCH (a:N {id: 0})-[r:E]->(b:N) WHERE b.id % 2 = 0 DELETE r;");
        mustRun("MATCH (a:N {id: 0}), (b:N) WHERE b.id >= 100 AND b.id <= 158 CREATE (a)-[:E]->(b);");
    }

    std::vector<int64_t> hubTargets() {
        std::vector<int64_t> targets;
        auto result = conn->query("MATCH (a:N {id: 0})-[:E]->(b:N) RETURN b.id;");
        EXPECT_TRUE(result->isSuccess()) << "[check: query] " << result->getErrorMessage();
        while (result->isSuccess() && result->hasNext()) {
            targets.push_back(result->getNext()->getValue(0)->getValue<int64_t>());
        }
        std::sort(targets.begin(), targets.end());
        return targets;
    }

    // Les impairs de 1 à 59, puis 100 à 158, chacun une fois.
    static std::vector<int64_t> expectedTargets() {
        std::vector<int64_t> expected;
        for (auto i = 1; i <= 59; i += 2) {
            expected.push_back(i);
        }
        for (auto i = 100; i <= 158; ++i) {
            expected.push_back(i);
        }
        return expected;
    }

    static std::string show(const std::vector<int64_t>& ids) {
        std::string text;
        for (auto id : ids) {
            text += std::to_string(id) + " ";
        }
        return text;
    }

    void expectIntegrity() {
        const auto violations = integrity::checkLevel1(*conn);
        std::string checks;
        for (const auto& violation : violations) {
            checks += "[check: " + violation.invariant + "] ";
        }
        EXPECT_TRUE(violations.empty()) << checks << integrity::describe(violations);
    }
};

TEST_F(UncommittedRelations, TransactionReadsItsNewRelations) {
    createHub(true /* onDisk */);
    mustRun("BEGIN TRANSACTION;");
    deleteEvenAndCreateNew();
    const auto inTransaction = hubTargets();
    EXPECT_EQ(inTransaction, expectedTargets())
        << "[check: transaction-reads-its-relations] read " << show(inTransaction);
    mustRun("COMMIT;");
    const auto committed = hubTargets();
    EXPECT_EQ(committed, expectedTargets())
        << "[check: committed-relations] read " << show(committed);
    expectIntegrity();
}

// Sans lecture dans la transaction : ce qui est validé.
TEST_F(UncommittedRelations, CommittedRelationsAfterDeleteAndCreate) {
    createHub(true /* onDisk */);
    mustRun("BEGIN TRANSACTION;");
    deleteEvenAndCreateNew();
    mustRun("COMMIT;");
    const auto committed = hubTargets();
    EXPECT_EQ(committed, expectedTargets())
        << "[check: committed-relations] read " << show(committed);
    expectIntegrity();
}

// L'écriture qui suit la lecture de travers : la suppression des nouvelles relations en
// laisse.
TEST_F(UncommittedRelations, DeleteTheNewRelationsInTheSameTransaction) {
    createHub(true /* onDisk */);
    mustRun("BEGIN TRANSACTION;");
    deleteEvenAndCreateNew();
    mustRun("MATCH (a:N {id: 0})-[r:E]->(b:N) WHERE b.id >= 100 DELETE r;");
    mustRun("COMMIT;");
    std::vector<int64_t> expected;
    for (auto i = 1; i <= 59; i += 2) {
        expected.push_back(i);
    }
    const auto committed = hubTargets();
    EXPECT_EQ(committed, expected) << "[check: deleted-relations-gone] read " << show(committed);
    expectIntegrity();
}

// L'écriture qui suit la lecture de travers : un MERGE des mêmes relations les double.
TEST_F(UncommittedRelations, MergeTheNewRelationsAgain) {
    createHub(true /* onDisk */);
    mustRun("BEGIN TRANSACTION;");
    deleteEvenAndCreateNew();
    mustRun("UNWIND range(100, 158) AS i MATCH (a:N {id: 0}), (b:N {id: i}) MERGE (a)-[:E]->(b);");
    mustRun("COMMIT;");
    const auto committed = hubTargets();
    EXPECT_EQ(committed, expectedTargets())
        << "[check: merge-no-duplicate] read " << show(committed);
    expectIntegrity();
}

// Garde-fou vert : les mêmes écritures sur des relations encore en mémoire.
TEST_F(UncommittedRelations, InMemoryRelationsReadBack) {
    createHub(false /* onDisk */);
    mustRun("BEGIN TRANSACTION;");
    deleteEvenAndCreateNew();
    const auto inTransaction = hubTargets();
    EXPECT_EQ(inTransaction, expectedTargets())
        << "[check: transaction-reads-its-relations] read " << show(inTransaction);
    mustRun("COMMIT;");
    EXPECT_EQ(hubTargets(), expectedTargets()) << "[check: committed-relations] ";
}

// Garde-fou vert : un MERGE par lot dont toutes les lignes partent du même nœud, sans
// suppression.
TEST_F(UncommittedRelations, MergeBatchFromTheSameNodeWithoutDelete) {
    createHub(true /* onDisk */);
    mustRun("BEGIN TRANSACTION;");
    mustRun("UNWIND range(100, 158) AS i MATCH (a:N {id: 0}), (b:N {id: i}) MERGE (a)-[:E]->(b);");
    mustRun("COMMIT;");
    std::vector<int64_t> expected;
    for (auto i = 1; i <= 60; ++i) {
        expected.push_back(i);
    }
    for (auto i = 100; i <= 158; ++i) {
        expected.push_back(i);
    }
    const auto committed = hubTargets();
    EXPECT_EQ(committed, expected) << "[check: merge-no-duplicate] read " << show(committed);
    expectIntegrity();
}

} // namespace

#endif
