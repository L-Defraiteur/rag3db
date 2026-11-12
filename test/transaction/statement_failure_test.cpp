// Une instruction qui échoue dans une transaction (tickets du 25 octobre 2026).
//
// - Une transaction en lecture seule n'est pas condamnée par l'échec d'une de ses instructions,
//   qu'il tombe à l'analyse, à la liaison ou à l'exécution : elle n'a rien écrit, il n'y a rien à
//   défaire. Avant, l'échec d'un signal de search_code (un index absent au binder, le DROP d'un
//   graphe projeté absent à l'exécution) faisait refuser toutes les lectures suivantes de
//   l'instantané (« nothing can run in it »). Une transaction qui écrit reste condamnée.
// - Une interruption vaut pour la requête, pas pour l'instruction : posée pendant qu'un
//   CREATE_VECTOR_INDEX attend les écrivains plus anciens que lui (une instruction de sa
//   réécriture, sans opérateur), elle l'arrête. Avant, chaque instruction de la réécriture
//   remettait le drapeau à zéro, et le CREATE finissait « ok ».
#include <atomic>
#include <chrono>
#include <future>
#include <string>
#include <thread>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "test_helper/test_helper.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class StatementFailure : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        ok("CREATE NODE TABLE T(id INT64 PRIMARY KEY, v FLOAT[4]);");
        ok("UNWIND range(1, 50) AS i CREATE (:T {id: i, v: [CAST(i AS FLOAT), 0.0, 0.0, 0.0]});");
    }
    void loadVector() {
        ok("LOAD EXTENSION '" +
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension") +
            "';");
    }
    void ok(const std::string& q) { ok(*conn, q); }
    static void ok(Connection& c, const std::string& q) {
        auto result = c.query(q);
        ASSERT_TRUE(result->isSuccess()) << q << " : " << result->getErrorMessage();
    }
    static std::string row(Connection& c, const std::string& q) {
        auto result = c.query(q);
        if (!result->isSuccess()) {
            return "REFUS " + result->getErrorMessage();
        }
        return result->hasNext() ? result->getNext()->toString() : "AUCUNE LIGNE";
    }
    std::string row(const std::string& q) { return row(*conn, q); }
};

TEST_F(StatementFailure, AFailedStatementDoesNotCondemnAReadOnlyTransaction) {
    loadVector();
    ok("BEGIN TRANSACTION READ ONLY;");
    // À la liaison : l'index n'existe pas.
    const auto atBind = row("CALL QUERY_VECTOR_INDEX('T', 'absent', "
                            "CAST([1.0, 0.0, 0.0, 0.0] AS FLOAT[4]), 3) RETURN node.id;");
    EXPECT_TRUE(atBind.starts_with("REFUS")) << atBind;
    EXPECT_EQ(row("MATCH (t:T) RETURN count(*);"), "50\n")
        << "[check: read-after-bind-failure-runs]";
    // À l'analyse : une requête qui ne se lit pas.
    EXPECT_TRUE(row("MATCH (t:T RETURN t;").starts_with("REFUS"));
    EXPECT_EQ(row("MATCH (t:T) WHERE t.id = 7 RETURN t.id;"), "7\n")
        << "[check: read-after-parse-failure-runs]";
    // À l'exécution : le DROP d'un graphe projeté absent.
    const auto atRun = row("CALL DROP_PROJECTED_GRAPH('absent');");
    EXPECT_TRUE(atRun.starts_with("REFUS")) << atRun;
    EXPECT_EQ(row("MATCH (t:T) RETURN count(*);"), "50\n")
        << "[check: read-after-runtime-failure-runs]";
    // Une écriture est refusée sous READ ONLY, et ce refus non plus ne condamne pas.
    EXPECT_TRUE(row("CREATE (:T {id: 1000});").starts_with("REFUS"));
    EXPECT_EQ(row("MATCH (t:T) RETURN count(*);"), "50\n")
        << "[check: read-after-refused-write-runs]";
    ok("COMMIT;");
    EXPECT_EQ(row("MATCH (t:T) RETURN count(*);"), "50\n");
}

TEST_F(StatementFailure, AFailedStatementStillCondemnsAWriteTransaction) {
    ok("BEGIN TRANSACTION;");
    ok("CREATE (:T {id: 1000});");
    EXPECT_TRUE(row("MATCH (t:T RETURN t;").starts_with("REFUS"));
    const auto after = row("MATCH (t:T) RETURN count(*);");
    EXPECT_NE(after.find("nothing can run in it"), std::string::npos)
        << "[check: write-transaction-stays-condemned] " << after;
    ok("ROLLBACK;");
    EXPECT_EQ(row("MATCH (t:T) RETURN count(*);"), "50\n")
        << "[check: condemned-write-left-nothing]";
}

TEST_F(StatementFailure, OneInterruptStopsACreateWaitingForOlderWriters) {
    if (inMemMode) {
        GTEST_SKIP() << "on-disk database needed";
    }
    loadVector();
    // Un écrivain plus ancien, ouvert : le CREATE l'attend (CREATE concurrent, (A)).
    Connection writer(database.get());
    ok(writer, "BEGIN TRANSACTION;");
    ok(writer, "CREATE (:T {id: 2000, v: [0.0, 0.0, 0.0, 1.0]});");
    Connection creator(database.get());
    std::atomic<bool> done{false};
    const auto started = std::chrono::steady_clock::now();
    auto creating = std::async(std::launch::async, [&] {
        auto result = creator.query("CALL CREATE_VECTOR_INDEX('T', 'idx', 'v', metric := 'l2');");
        done = true;
        return result->isSuccess() ? std::string("ok") : result->getErrorMessage();
    });
    // Le CREATE est dans son attente : une seule interruption.
    std::this_thread::sleep_for(std::chrono::milliseconds(800));
    ASSERT_FALSE(done.load()) << "[check: setup] the CREATE did not wait for the open writer";
    creator.interrupt();
    const auto status = creating.wait_for(std::chrono::seconds(3));
    const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::steady_clock::now() - started)
                             .count();
    ASSERT_EQ(status, std::future_status::ready)
        << "[check: one-interrupt-stops-the-wait] the CREATE still runs 3 s after the interrupt";
    const auto outcome = creating.get();
    EXPECT_NE(outcome.find("Interrupted"), std::string::npos)
        << "[check: one-interrupt-stops-the-create] " << outcome << " (" << elapsed << " ms)";
    creator.query("ROLLBACK;");
    ok(writer, "ROLLBACK;");
}

} // namespace
