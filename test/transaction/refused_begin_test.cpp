// Un BEGIN refusé laisse la connexion utilisable (ticket du banc, 18 octobre 2026 : « un BEGIN
// refusé laisse la connexion en transaction explicite sans transaction »).
//
// TransactionContext::beginWriteTransaction posait le mode MANUAL avant de demander la
// transaction au TransactionManager. Si celui-ci refusait (un second écrivain hors du mode
// multi-écrivains, un point de reprise en échec, le tampon plein), le mode restait MANUAL sans
// transaction : ROLLBACK répondait « No active transaction » sans rendre le mode, et la requête
// suivante se liait sur une transaction nulle (SIGSEGV dans la traversée du catalogue). Le mode
// ne passe plus à MANUAL qu'une fois la transaction obtenue. Le témoin du banc
// (concurrence/failed_begin_test.cpp) joue la même recette dans un processus fils ; celui-ci la
// joue en direct, dans la suite ordinaire.

#include <memory>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "common/exception/runtime.h"
#include "flaky_checkpointer.h"
#include "storage/checkpointer.h"
#include "storage/storage_manager.h"
#include "transaction/transaction_context.h"
#include "transaction/transaction_manager.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class RefusedBegin : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        other = std::make_unique<Connection>(database.get());
        ok(*conn, "CALL enable_multi_writes=false;");
        ok(*conn, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, titre STRING);");
    }

    void TearDown() override {
        other.reset();
        EmptyDBTest::TearDown();
    }

    static void ok(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    static std::string error(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        return result->isSuccess() ? "" : result->getErrorMessage();
    }
    static int64_t count(Connection& connection) {
        auto result = connection.query("MATCH (d:Doc) RETURN count(*);");
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    static rag3db::transaction::TransactionContext* txOf(Connection& connection) {
        return rag3db::transaction::TransactionContext::Get(*connection.getClientContext());
    }

    std::unique_ptr<Connection> other;
};

TEST_F(RefusedBegin, TheConnectionStaysInAutoModeAndWritesAgain) {
    ok(*conn, "BEGIN TRANSACTION;");
    ok(*conn, "CREATE (:Doc {id: 1, titre: 'a'});");
    // Le second écrivain est refusé par son nom, hors du mode multi-écrivains.
    const auto refused = error(*other, "BEGIN TRANSACTION;");
    ASSERT_NE(refused, "") << "the second BEGIN was accepted: nothing is witnessed";
    EXPECT_FALSE(txOf(*other)->hasActiveTransaction());
    EXPECT_TRUE(txOf(*other)->isAutoTransaction())
        << "the refused BEGIN left the connection in manual mode without a transaction";
    // ROLLBACK et COMMIT n'ont rien à fermer, et le disent.
    EXPECT_NE(error(*other, "ROLLBACK;").find("No active transaction"), std::string::npos);
    EXPECT_NE(error(*other, "COMMIT;").find("No active transaction"), std::string::npos);
    // La lecture passe, et l'écriture (en transaction automatique) attend ou est refusée par un
    // nom, mais ne plante pas.
    ok(*other, "RETURN 1;");
    ok(*conn, "COMMIT;");
    ok(*other, "MERGE (d:Doc {id: 2}) SET d.titre = 'b';");
    EXPECT_EQ(count(*conn), 2);
}

TEST_F(RefusedBegin, ARefusedReadBeginLeavesTheConnectionUsableToo) {
    ok(*conn, "BEGIN TRANSACTION;");
    ok(*conn, "CREATE (:Doc {id: 1, titre: 'a'});");
    // Un BEGIN de lecture passe toujours ; un BEGIN d'écriture refusé puis un BEGIN de lecture
    // accepté doivent se suivre sans trace du refus.
    ASSERT_NE(error(*other, "BEGIN TRANSACTION;"), "");
    ok(*other, "BEGIN TRANSACTION READ ONLY;");
    EXPECT_TRUE(txOf(*other)->hasActiveTransaction());
    ok(*other, "MATCH (d:Doc) RETURN count(*);");
    ok(*other, "COMMIT;");
    EXPECT_TRUE(txOf(*other)->isAutoTransaction());
    ok(*conn, "COMMIT;");
    EXPECT_EQ(count(*other), 1);
}

// Le second refus facile à provoquer : un point de reprise en échec après avoir changé la mémoire,
// après lequel le gestionnaire refuse tout BEGIN par un nom (REOPEN_AFTER_FAILED_CHECKPOINT)
// jusqu'à la réouverture. Refusé avant tout changement, il laisserait la base ouverte
// (FlakyCheckpointerTest.ARefusedCheckpointThatChangedNothingLeavesTheDatabaseOpen).
class CheckpointerFailingOnStorage final : public rag3db::storage::Checkpointer {
public:
    explicit CheckpointerFailingOnStorage(ClientContext& context) : Checkpointer(context) {}
    bool checkpointStorage() override {
        rag3db::storage::StorageManager::Get(clientContext)->markCheckpointChangedMemory();
        throw rag3db::common::RuntimeException("checkpoint failed on purpose.");
    }
};

TEST_F(RefusedBegin, ABeginRefusedAfterAFailedCheckpointLeavesTheConnectionUsable) {
    if (inMemMode) {
        GTEST_SKIP() << "a checkpoint needs an on-disk database";
    }
    ok(*conn, "CALL auto_checkpoint=false;");
    ok(*conn, "CALL force_checkpoint_on_close=false;");
    ok(*conn, "CREATE (:Doc {id: 1, titre: 'a'});");
    FlakyCheckpointer flaky([](ClientContext& context) {
        return std::make_unique<CheckpointerFailingOnStorage>(context);
    });
    flaky.setCheckpointer(*getClientContext(*conn));
    ASSERT_NE(error(*conn, "CHECKPOINT;"), "");
    const auto refused = error(*other, "BEGIN TRANSACTION;");
    ASSERT_NE(refused.find(rag3db::transaction::TransactionManager::REOPEN_AFTER_FAILED_CHECKPOINT),
        std::string::npos)
        << refused;
    EXPECT_FALSE(txOf(*other)->hasActiveTransaction());
    EXPECT_TRUE(txOf(*other)->isAutoTransaction())
        << "the refused BEGIN left the connection in manual mode without a transaction";
    // Tout est refusé jusqu'à la réouverture, par le nom du point de reprise ou, pour un ROLLBACK
    // sans transaction, par le sien : un refus, pas un plantage.
    const auto rollback = error(*other, "ROLLBACK;");
    EXPECT_TRUE(rollback.find("No active transaction") != std::string::npos ||
                rollback.find(
                    rag3db::transaction::TransactionManager::REOPEN_AFTER_FAILED_CHECKPOINT) !=
                    std::string::npos)
        << rollback;
    EXPECT_NE(error(*other, "RETURN 1;").find(
                  rag3db::transaction::TransactionManager::REOPEN_AFTER_FAILED_CHECKPOINT),
        std::string::npos);
    other.reset();
    createDBAndConn();
    EXPECT_EQ(count(*conn), 1);
}

} // namespace
