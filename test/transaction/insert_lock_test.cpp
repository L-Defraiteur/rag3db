// Marche A3′ : les verrous d'une insertion et d'un COPY, hors du banc de concurrence. Le banc
// prouve l'attente entre fils ; ici, ce qui se prouve dans un seul fil : hors du mode
// multi-écrivains rien n'est pris ; sous ce mode, une insertion tient l'index de sa table en
// partagé et sa clé en exclusif, un COPY l'index en exclusif, tout est rendu à la fin de la
// transaction ; une seconde insertion de la même clé par une autre connexion attend (ici
// jusqu'au délai) ; et le contrôle d'unicité regarde le dernier état validé, non l'instantané.

#include <atomic>
#include <chrono>
#include <filesystem>
#include <fstream>
#include <thread>
#include <vector>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "transaction/lock_manager.h"
#include "transaction/transaction.h"
#include "transaction/transaction_manager.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::testing;
using namespace rag3db::transaction;

namespace {

class InsertLockTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        mustRun(*conn, "CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64);");
    }

    static void mustRun(Connection& connection, const std::string& query) {
        const auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << "\n" << result->getErrorMessage();
    }

    static std::string failureOf(Connection& connection, const std::string& query) {
        const auto result = connection.query(query);
        return result->isSuccess() ? "" : result->getErrorMessage();
    }

    LockManager& locks() { return getTransactionManager(*database)->getLockManager(); }

    static transaction_t transactionIDOf(Connection& connection) {
        return Transaction::Get(*connection.getClientContext())->getID();
    }

    std::string copyOfKeys(int64_t first, int64_t count) {
        const auto path = databasePath + ".keys.csv";
        std::ofstream csv(path);
        for (auto id = first; id < first + count; id++) {
            csv << id << "," << id << "\n";
        }
        return "COPY Item FROM '" + path + "' (header=false);";
    }

    void TearDown() override {
        std::filesystem::remove(databasePath + ".keys.csv");
        EmptyDBTest::TearDown();
    }
};

TEST_F(InsertLockTest, WithoutMultiWritesNothingIsLocked) {
    mustRun(*conn, "BEGIN TRANSACTION;");
    mustRun(*conn, "CREATE (:Item {id: 7, v: 0});");
    mustRun(*conn, copyOfKeys(100, 10));
    EXPECT_EQ(locks().getNumResources(), 0u);
    EXPECT_EQ(locks().getNumLocksHeld(transactionIDOf(*conn)), 0u);
    mustRun(*conn, "COMMIT;");
}

TEST_F(InsertLockTest, AnInsertHoldsTheIndexSharedAndItsKeyExclusiveUntilTheEnd) {
    mustRun(*conn, "CALL debug_enable_multi_writes=true;");
    for (const auto* end : {"COMMIT;", "ROLLBACK;"}) {
        mustRun(*conn, "BEGIN TRANSACTION;");
        mustRun(*conn, "CREATE (:Item {id: 7, v: 0});");
        const auto id = transactionIDOf(*conn);
        EXPECT_EQ(locks().getNumLocksHeld(id), 2u) << "the index, shared, and the key";
        mustRun(*conn, "CREATE (:Item {id: 8, v: 0});");
        EXPECT_EQ(locks().getNumLocksHeld(id), 3u) << "the index again, and a second key";
        mustRun(*conn, end);
        EXPECT_EQ(locks().getNumLocksHeld(id), 0u) << end;
        EXPECT_EQ(locks().getNumResources(), 0u) << end;
        mustRun(*conn, "MATCH (n:Item) DELETE n;");
    }
}

TEST_F(InsertLockTest, ACopyHoldsTheIndexExclusiveUntilTheEnd) {
    mustRun(*conn, "CALL debug_enable_multi_writes=true;");
    mustRun(*conn, "BEGIN TRANSACTION;");
    mustRun(*conn, copyOfKeys(100, 10));
    const auto id = transactionIDOf(*conn);
    EXPECT_EQ(locks().getNumLocksHeld(id), 1u) << "the index, exclusive";
    mustRun(*conn, "CREATE (:Item {id: 7, v: 0});");
    EXPECT_EQ(locks().getNumLocksHeld(id), 2u) << "the index is already held; plus the key";
    mustRun(*conn, "ROLLBACK;");
    EXPECT_EQ(locks().getNumResources(), 0u);
}

TEST_F(InsertLockTest, ASecondInsertOfTheSameKeyWaitsForTheFirstTransaction) {
    Connection other(database.get());
    mustRun(*conn, "CALL debug_enable_multi_writes=true;");
    mustRun(other, "CALL lock_timeout=200;");
    mustRun(*conn, "BEGIN TRANSACTION;");
    mustRun(*conn, "CREATE (:Item {id: 7, v: 0});");
    mustRun(other, "BEGIN TRANSACTION;");
    // Un autre clé ne gêne pas.
    mustRun(other, "CREATE (:Item {id: 8, v: 1});");
    // La même clé attend, ici jusqu'au délai : l'erreur nommée, et la transaction est annulée.
    const auto error = failureOf(other, "CREATE (:Item {id: 7, v: 1});");
    EXPECT_NE(error.find(LockManager::LOCK_TIMEOUT), std::string::npos) << error;
    EXPECT_EQ(locks().getNumLocksHeld(transactionIDOf(*conn)), 2u)
        << "the first transaction still holds what it took";
    mustRun(other, "ROLLBACK;");
    mustRun(*conn, "COMMIT;");
    EXPECT_EQ(locks().getNumResources(), 0u);
    // Après la validation de la première, la même clé est un doublon, l'autre clé passe.
    const auto duplicate = failureOf(other, "CREATE (:Item {id: 7, v: 1});");
    EXPECT_NE(duplicate.find("duplicated primary key"), std::string::npos) << duplicate;
    mustRun(other, "CREATE (:Item {id: 8, v: 1});");
}

TEST_F(InsertLockTest, UniquenessIsCheckedAgainstTheLatestCommitNotTheSnapshot) {
    Connection other(database.get());
    mustRun(*conn, "CALL debug_enable_multi_writes=true;");
    // L'instantané de 'other' précède la validation de la clé 7 par 'conn' ; sans
    // chevauchement, aucun verrou ne les sépare. Seul le contrôle contre le dernier état
    // validé refuse la seconde insertion (C1_SnapshotPredatesCommit du banc).
    mustRun(other, "BEGIN TRANSACTION;");
    mustRun(other, "MATCH (n:Item) RETURN count(n);");
    mustRun(*conn, "CREATE (:Item {id: 7, v: 0});");
    const auto duplicate = failureOf(other, "CREATE (:Item {id: 7, v: 1});");
    EXPECT_NE(duplicate.find("duplicated primary key"), std::string::npos) << duplicate;
    mustRun(other, "ROLLBACK;");
    auto result = conn->query("MATCH (n:Item) WHERE n.id = 7 RETURN count(n);");
    ASSERT_TRUE(result->isSuccess());
    EXPECT_EQ(result->getNext()->getValue(0)->getValue<int64_t>(), 1);
}

// La clé d'une ligne supprimée et validée après l'instantané est libre, à l'insertion ET à la
// validation : si la validation contrôlait l'unicité avec l'instantané, elle lèverait l'erreur
// de clé en double après avoir ajouté les lignes aux groupes — une transaction à moitié
// validée, vue dans C7 du banc (10 octobre : un groupe de nœuds introuvable à la suppression
// suivante, SIGSEGV).
TEST_F(InsertLockTest, AKeyDeletedAndCommittedAfterTheSnapshotIsFreeAtCommitToo) {
    Connection other(database.get());
    mustRun(*conn, "CALL debug_enable_multi_writes=true;");
    mustRun(*conn, "CREATE (:Item {id: 7, v: 0});");
    mustRun(other, "BEGIN TRANSACTION;");
    mustRun(other, "MATCH (n:Item) RETURN count(n);");
    mustRun(*conn, "MATCH (n:Item {id: 7}) DETACH DELETE n;");
    mustRun(other, "CREATE (:Item {id: 7, v: 1});");
    mustRun(other, "CREATE (:Item {id: 8, v: 1});");
    mustRun(other, "MATCH (n:Item {id: 8}) DETACH DELETE n;");
    mustRun(other, "COMMIT;");
    auto result = conn->query("MATCH (n:Item) RETURN n.id, n.v ORDER BY n.id;");
    ASSERT_TRUE(result->isSuccess());
    ASSERT_TRUE(result->hasNext());
    auto row = result->getNext();
    EXPECT_EQ(row->getValue(0)->getValue<int64_t>(), 7);
    EXPECT_EQ(row->getValue(1)->getValue<int64_t>(), 1);
    EXPECT_FALSE(result->hasNext());
    // Et la session continue : la clé 8, supprimée dans la même transaction, est libre.
    mustRun(*conn, "CREATE (:Item {id: 8, v: 2});");
    const auto duplicate = failureOf(*conn, "CREATE (:Item {id: 7, v: 2});");
    EXPECT_NE(duplicate.find("duplicated primary key"), std::string::npos) << duplicate;
}

// La famine des fils de l'ordonnanceur (banc, 10 octobre) : plus d'attendants que de fils
// ouvriers. Deux fils ouvriers, quatre connexions qui attendent la même clé dans un fil chacune ;
// le détenteur doit pouvoir valider tout de suite — sans fil de remplacement, sa validation
// n'avait plus de fil ouvrier et attendait la fin des attentes (le délai des verrous).
class FewWorkerThreadsLockTest : public InsertLockTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        systemConfig->maxNumThreads = 2;
        createDBAndConn();
        mustRun(*conn, "CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64);");
    }
};

TEST_F(FewWorkerThreadsLockTest, TheHolderCommitsWhileMoreWaitersThanWorkerThreadsWait) {
    mustRun(*conn, "CALL debug_enable_multi_writes=true;");
    mustRun(*conn, "BEGIN TRANSACTION;");
    mustRun(*conn, "CREATE (:Item {id: 7, v: 0});");
    constexpr auto numWaiters = 4u;
    std::atomic<uint32_t> started{0};
    std::vector<std::string> errors(numWaiters);
    std::vector<std::thread> waiters;
    for (auto i = 0u; i < numWaiters; i++) {
        waiters.emplace_back([&, i] {
            Connection waiter(database.get());
            started.fetch_add(1);
            errors[i] = failureOf(waiter, "CREATE (:Item {id: 7, v: 1});");
        });
    }
    // Tous en attente du verrou de la clé 7 : la seule chose qui se voit de l'extérieur.
    const auto limit = std::chrono::steady_clock::now() + std::chrono::seconds(10);
    auto& lockManager = locks();
    while (std::chrono::steady_clock::now() < limit &&
           (started.load() < numWaiters || lockManager.getNumResources() < 2)) {
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(200));
    const auto before = std::chrono::steady_clock::now();
    mustRun(*conn, "COMMIT;");
    const auto commitMs = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::steady_clock::now() - before)
                              .count();
    EXPECT_LT(commitMs, 3000) << "the holder's COMMIT waited " << commitMs
                              << " ms: no worker thread was left for it";
    for (auto& waiter : waiters) {
        waiter.join();
    }
    for (auto i = 0u; i < numWaiters; i++) {
        EXPECT_NE(errors[i].find("duplicated primary key"), std::string::npos)
            << "waiter " << i << ": " << errors[i];
    }
    EXPECT_EQ(locks().getNumResources(), 0u);
}

// ── Marche A4′ : les écritures d'une ligne et d'une relation sous les verrous ─────────────
// Le banc prouve l'attente entre fils ; ici, dans un seul fil, ce que le second voit APRÈS
// que le premier a validé entre son instantané et son écriture.

// Un DETACH DELETE détache aussi une relation attachée et validée par un autre après son
// instantané (lecture validée de Neo4j, le nœud tenu en exclusif), et non l'instantané de
// PostgreSQL : ni nœud ni relation ne restent, avant comme après une réouverture.
TEST_F(InsertLockTest, DetachDeleteAlsoDetachesARelationCommittedAfterTheSnapshot) {
    Connection other(database.get());
    mustRun(*conn, "CALL debug_enable_multi_writes=true;");
    mustRun(*conn, "CREATE REL TABLE Link(FROM Item TO Item, w INT64);");
    mustRun(*conn, "CREATE (:Item {id: 1, v: 0}), (:Item {id: 2, v: 0});");
    mustRun(other, "BEGIN TRANSACTION;");
    mustRun(other, "MATCH (n:Item) RETURN count(n);");
    mustRun(*conn, "MATCH (a:Item {id: 1}), (b:Item {id: 2}) CREATE (a)-[:Link {w: 7}]->(b);");
    mustRun(other, "MATCH (n:Item {id: 1}) DETACH DELETE n;");
    mustRun(other, "COMMIT;");
    for (auto pass = 0; pass < 2; pass++) {
        auto nodes = conn->query("MATCH (n:Item) RETURN n.id ORDER BY n.id;");
        ASSERT_TRUE(nodes->isSuccess());
        ASSERT_TRUE(nodes->hasNext());
        EXPECT_EQ(nodes->getNext()->getValue(0)->getValue<int64_t>(), 2) << "pass " << pass;
        EXPECT_FALSE(nodes->hasNext()) << "pass " << pass;
        auto rels = conn->query("MATCH ()-[r:Link]->() RETURN count(r);");
        ASSERT_TRUE(rels->isSuccess());
        EXPECT_EQ(rels->getNext()->getValue(0)->getValue<int64_t>(), 0) << "pass " << pass;
        auto dangling = conn->query("MATCH (a)-[r:Link]->(b) RETURN count(a) + count(b);");
        ASSERT_TRUE(dangling->isSuccess());
        EXPECT_EQ(dangling->getNext()->getValue(0)->getValue<int64_t>(), 0) << "pass " << pass;
        if (pass == 0) {
            createDBAndConn();
        }
    }
}

// Un DELETE sans DETACH est refusé par « has connected edges » d'après le dernier état validé :
// la relation attachée et validée par un autre après l'instantané compte. La base est intacte.
TEST_F(InsertLockTest, DeleteIsRefusedByARelationCommittedAfterTheSnapshot) {
    Connection other(database.get());
    mustRun(*conn, "CALL debug_enable_multi_writes=true;");
    mustRun(*conn, "CREATE REL TABLE Link(FROM Item TO Item, w INT64);");
    mustRun(*conn, "CREATE (:Item {id: 1, v: 0}), (:Item {id: 2, v: 0});");
    mustRun(other, "BEGIN TRANSACTION;");
    mustRun(other, "MATCH (n:Item) RETURN count(n);");
    mustRun(*conn, "MATCH (a:Item {id: 1}), (b:Item {id: 2}) CREATE (a)-[:Link {w: 7}]->(b);");
    const auto refused = failureOf(other, "MATCH (n:Item {id: 1}) DELETE n;");
    EXPECT_NE(refused.find("has connected edges"), std::string::npos) << refused;
    mustRun(other, "ROLLBACK;");
    auto nodes = conn->query("MATCH (n:Item) RETURN count(n);");
    ASSERT_TRUE(nodes->isSuccess());
    EXPECT_EQ(nodes->getNext()->getValue(0)->getValue<int64_t>(), 2);
    auto rels = conn->query("MATCH (a:Item {id: 1})-[r:Link]->(b:Item {id: 2}) RETURN r.w;");
    ASSERT_TRUE(rels->isSuccess());
    ASSERT_TRUE(rels->hasNext());
    EXPECT_EQ(rels->getNext()->getValue(0)->getValue<int64_t>(), 7);
}

// Une mise à jour d'une ligne qu'un autre a mise à jour (autre colonne comprise) ou supprimée
// et validée après l'instantané : l'erreur de sérialisation (option A, PostgreSQL en lecture
// répétable) ; si l'autre a annulé, l'écriture passe.
TEST_F(InsertLockTest, AnUpdateAfterAnotherCommittedWriteOfTheRowIsASerializationError) {
    Connection other(database.get());
    mustRun(*conn, "CALL debug_enable_multi_writes=true;");
    mustRun(*conn, "CREATE (:Item {id: 1, v: 0}), (:Item {id: 2, v: 0}), (:Item {id: 3, v: 0});");
    mustRun(other, "BEGIN TRANSACTION;");
    mustRun(other, "MATCH (n:Item) RETURN count(n);");
    mustRun(*conn, "MATCH (n:Item {id: 1}) SET n.v = 10;");
    mustRun(*conn, "BEGIN TRANSACTION;");
    mustRun(*conn, "MATCH (n:Item {id: 3}) SET n.v = 30;");
    mustRun(*conn, "ROLLBACK;");
    const auto updated = failureOf(other, "MATCH (n:Item {id: 1}) SET n.v = 1;");
    EXPECT_NE(updated.find(LockManager::COULD_NOT_SERIALIZE), std::string::npos) << updated;
    mustRun(other, "ROLLBACK;");
    // Un instantané neuf, pris avant la suppression de la ligne 2 par l'autre.
    mustRun(other, "BEGIN TRANSACTION;");
    mustRun(other, "MATCH (n:Item) RETURN count(n);");
    mustRun(*conn, "MATCH (n:Item {id: 2}) DETACH DELETE n;");
    mustRun(*conn, "MATCH (n:Item {id: 3}) SET n.v = 33;");
    const auto deleted = failureOf(other, "MATCH (n:Item {id: 2}) SET n.v = 2;");
    EXPECT_NE(deleted.find(LockManager::COULD_NOT_SERIALIZE), std::string::npos) << deleted;
    mustRun(other, "ROLLBACK;");
    // Le dernier état validé : v = 10, la ligne 2 absente, v = 33 ; et depuis un instantané
    // neuf, la ligne 3 se met à jour.
    mustRun(other, "MATCH (n:Item {id: 3}) SET n.v = 3;");
    auto result = conn->query("MATCH (n:Item) RETURN n.id, n.v ORDER BY n.id;");
    ASSERT_TRUE(result->isSuccess());
    std::vector<std::pair<int64_t, int64_t>> rows;
    while (result->hasNext()) {
        auto row = result->getNext();
        rows.emplace_back(row->getValue(0)->getValue<int64_t>(), row->getValue(1)->getValue<int64_t>());
    }
    EXPECT_EQ(rows, (std::vector<std::pair<int64_t, int64_t>>{{1, 10}, {3, 3}}));
}

TEST_F(InsertLockTest, WithoutMultiWritesTheSnapshotStillRules) {
    // Hors du mode, rien ne change : un seul écrivain à la fois, l'autre connexion attend
    // son tour pour écrire, et le doublon est vu comme avant.
    mustRun(*conn, "CREATE (:Item {id: 7, v: 0});");
    const auto duplicate = failureOf(*conn, "CREATE (:Item {id: 7, v: 1});");
    EXPECT_NE(duplicate.find("duplicated primary key"), std::string::npos) << duplicate;
}

} // namespace
