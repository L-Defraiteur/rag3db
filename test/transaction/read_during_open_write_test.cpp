// Un lecteur qui ouvre son instantané pendant qu'un autre écrivain tient sa transaction ouverte,
// après ses écritures non validées (ticket de recherche du 13 novembre 2026,
// docs/recherche/tickets/2026-11-13-une-lecture-pendant-un-lot-reindexe-voit-le-lot-a-moitie-
// sous-la-lib-entiere.md) : dans rag3weaver, une transaction READ ONLY du pool de lecture, ouverte
// pendant la pause d'un lot, a vu une ligne Scope, un Symbol et une arête MENTIONS du lot pas
// encore validé. Les écritures du lot passent par MERGE et, pour les liens, par COPY.
//
// Chaque cas : l'écrivain (connexion A) fait BEGIN et ses écritures ; le lecteur (connexion B)
// ouvre ensuite BEGIN TRANSACTION READ ONLY et compte ; rien de non validé ne doit y paraître, ni
// avant le COMMIT de A, ni après dans le même instantané.
#include <filesystem>
#include <fstream>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class ReadDuringOpenWrite : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
        reader = std::make_unique<Connection>(database.get());
        run(*conn, "CREATE NODE TABLE Scope(uuid STRING, PRIMARY KEY(uuid));");
        run(*conn, "CREATE NODE TABLE Symbol(uuid STRING, PRIMARY KEY(uuid));");
        run(*conn, "CREATE REL TABLE MENTIONS(FROM Scope TO Symbol);");
        run(*conn, "UNWIND range(0, 99) AS i CREATE (:Scope {uuid: 's' + CAST(i AS STRING)});");
        run(*conn, "UNWIND range(0, 99) AS i CREATE (:Symbol {uuid: 'y' + CAST(i AS STRING)});");
        run(*conn, "UNWIND range(0, 99) AS i MATCH (a:Scope {uuid: 's' + CAST(i AS STRING)}), "
                   "(b:Symbol {uuid: 'y' + CAST(i AS STRING)}) CREATE (a)-[:MENTIONS]->(b);");
        csv = (std::filesystem::temp_directory_path() /
               ("read_during_open_write_" + std::to_string(::getpid()) + ".csv"))
                  .string();
    }
    void TearDown() override {
        reader.reset();
        std::filesystem::remove(csv);
        EmptyDBTest::TearDown();
    }
    static void run(Connection& c, const std::string& q) {
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
    // Ce que voit le lecteur : nœuds des deux tables, arêtes, et les lignes nommées du lot.
    std::string seen() const {
        return row(*reader, "MATCH (n:Scope) RETURN count(n);") +
               row(*reader, "MATCH (n:Symbol) RETURN count(n);") +
               row(*reader, "MATCH (:Scope)-[r:MENTIONS]->(:Symbol) RETURN count(r);") +
               row(*reader, "MATCH (n:Scope) WHERE n.uuid STARTS WITH 'lot' RETURN count(n);") +
               row(*reader, "MATCH (n:Symbol) WHERE n.uuid STARTS WITH 'lot' RETURN count(n);");
    }
    // Le lecteur ouvre son instantané pendant la transaction de l'écrivain, compte, puis l'écrivain
    // valide et le lecteur recompte dans le même instantané.
    void readWhileOpenThenCommit(const std::string& context) {
        run(*reader, "BEGIN TRANSACTION READ ONLY;");
        EXPECT_EQ(seen(), "100\n100\n100\n0\n0\n") << context << " : pendant la transaction de A";
        run(*conn, "COMMIT;");
        EXPECT_EQ(seen(), "100\n100\n100\n0\n0\n")
            << context << " : après le COMMIT de A, même instantané";
        run(*reader, "COMMIT;");
    }
    std::unique_ptr<Connection> reader;
    std::string csv;
};

TEST_F(ReadDuringOpenWrite, NodesAndRelsCreatedByMergeStayInvisible) {
    run(*conn, "BEGIN TRANSACTION;");
    run(*conn, "UNWIND range(0, 31) AS i MERGE (:Scope {uuid: 'lot-s' + CAST(i AS STRING)});");
    run(*conn, "UNWIND range(0, 31) AS i MERGE (:Symbol {uuid: 'lot-y' + CAST(i AS STRING)});");
    run(*conn, "UNWIND range(0, 31) AS i MATCH (a:Scope {uuid: 'lot-s' + CAST(i AS STRING)}), "
               "(b:Symbol {uuid: 'lot-y' + CAST(i AS STRING)}) MERGE (a)-[:MENTIONS]->(b);");
    readWhileOpenThenCommit("MERGE");
}

TEST_F(ReadDuringOpenWrite, RelsCopiedAfterMergedNodesStayInvisible) {
    run(*conn, "BEGIN TRANSACTION;");
    run(*conn, "UNWIND range(0, 31) AS i MERGE (:Scope {uuid: 'lot-s' + CAST(i AS STRING)});");
    run(*conn, "UNWIND range(0, 31) AS i MERGE (:Symbol {uuid: 'lot-y' + CAST(i AS STRING)});");
    {
        std::ofstream f(csv);
        for (auto i = 0; i < 32; i++) {
            f << "lot-s" << i << ",lot-y" << i << "\n";
        }
    }
    run(*conn, "COPY MENTIONS FROM '" + csv + "' (header=false);");
    readWhileOpenThenCommit("MERGE puis COPY des liens");
}

TEST_F(ReadDuringOpenWrite, NodesAndRelsCopiedStayInvisible) {
    run(*conn, "BEGIN TRANSACTION;");
    run(*conn, "COPY Scope FROM (UNWIND range(0, 31) AS i RETURN 'lot-s' + CAST(i AS STRING) AS "
               "uuid);");
    run(*conn, "COPY Symbol FROM (UNWIND range(0, 31) AS i RETURN 'lot-y' + CAST(i AS STRING) AS "
               "uuid);");
    {
        std::ofstream f(csv);
        for (auto i = 0; i < 32; i++) {
            f << "lot-s" << i << ",lot-y" << i << "\n";
        }
    }
    run(*conn, "COPY MENTIONS FROM '" + csv + "' (header=false);");
    readWhileOpenThenCommit("COPY des nœuds et des liens");
}

TEST_F(ReadDuringOpenWrite, ALargeCopyStaysInvisible) {
    // Assez de lignes pour qu'un COPY vide des lots dans les groupes de nœuds avant sa validation.
    run(*conn, "BEGIN TRANSACTION;");
    run(*conn, "COPY Scope FROM (UNWIND range(0, 199999) AS i RETURN 'lot-s' + CAST(i AS STRING) "
               "AS uuid);");
    run(*reader, "BEGIN TRANSACTION READ ONLY;");
    EXPECT_EQ(row(*reader, "MATCH (n:Scope) RETURN count(n);"), "100\n");
    EXPECT_EQ(row(*reader, "MATCH (n:Scope) WHERE n.uuid STARTS WITH 'lot' RETURN count(n);"),
        "0\n");
    run(*conn, "COMMIT;");
    EXPECT_EQ(row(*reader, "MATCH (n:Scope) RETURN count(n);"), "100\n");
    run(*reader, "COMMIT;");
}

} // namespace
