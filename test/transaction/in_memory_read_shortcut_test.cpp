// La lecture en mémoire des lignes écrites depuis le dernier point de reprise (page coeur-cpp/13
// §6) : le résumé du bloc (VersionInfo::allRowsVisibleTo — toutes les insertions validées au plus
// tard à l'instantané, aucune suppression → rien à vérifier ligne à ligne) et la lecture des suites
// contiguës d'une liste CSR compactée par un balayage de bloc (CSRNodeGroup::scanRun).
//
// Ce que les témoins fixent : la visibilité ne change pas (un lecteur d'avant ne voit pas ce qui
// est validé après lui, une suppression validée disparaît pour qui commence après, les lignes de
// la transaction elle-même se voient), et une suite lue d'un bloc rend les mêmes propriétés et les
// mêmes numéros de ligne que ligne à ligne — le DELETE qui suit, qui passe par le numéro de ligne,
// efface exactement ce qu'il a lu.

#include <numeric>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class InMemoryReadShortcut : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        reader = std::make_unique<Connection>(database.get());
        ok("CREATE NODE TABLE A(id INT64 PRIMARY KEY);");
        ok("CREATE NODE TABLE B(id INT64 PRIMARY KEY, v INT64);");
        ok("CREATE REL TABLE R(FROM A TO B, k INT64);");
        ok("CREATE (:A {id: 0});");
        ok("UNWIND range(0, 2999) AS i CREATE (:B {id: i, v: i * 7});");
        ok("CHECKPOINT;");
    }

    void TearDown() override {
        reader.reset();
        EmptyDBTest::TearDown();
    }

    static void ok(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    void ok(const std::string& query) { ok(*conn, query); }

    static int64_t scalar(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        if (!result->isSuccess() || !result->hasNext()) {
            return -1;
        }
        return result->getNext()->getValue(0)->getValue<int64_t>();
    }
    int64_t count(Connection& c) { return scalar(c, "MATCH (:A)-[r:R]->(:B) RETURN count(r);"); }
    int64_t sumK(Connection& c) { return scalar(c, "MATCH (:A)-[r:R]->(:B) RETURN sum(r.k);"); }
    // La somme des propriétés lues par le saut : celle de la relation et celle du voisin.
    int64_t sumKV(Connection& c) {
        return scalar(c, "MATCH (:A)-[r:R]->(b:B) RETURN sum(r.k + b.v);");
    }

    std::unique_ptr<Connection> reader;
};

} // namespace

// Les relations validées en mémoire : un lecteur d'avant ne les voit pas, un lecteur d'après les
// voit toutes (c'est le bloc entier, sans vérification ligne à ligne) ; une suppression validée les
// retire pour qui commence après et les laisse à qui a commencé avant ; la transaction qui écrit
// voit ses propres lignes avant de valider.
TEST_F(InMemoryReadShortcut, VisibilityIsUnchangedByTheBlockSummary) {
    ok(*reader, "BEGIN TRANSACTION READ ONLY;");
    EXPECT_EQ(count(*reader), 0);
    ok("MATCH (a:A), (b:B) WHERE b.id < 1000 CREATE (a)-[:R {k: b.id}]->(b);");
    EXPECT_EQ(count(*reader), 0) << "[check: old-reader-sees-nothing]";
    EXPECT_EQ(count(*conn), 1000) << "[check: new-reader-sees-the-block]";
    EXPECT_EQ(sumK(*conn), 999 * 1000 / 2);
    ok(*reader, "COMMIT;");
    EXPECT_EQ(count(*reader), 1000);
    // La transaction qui écrit voit ses lignes, pas les autres.
    ok("BEGIN TRANSACTION;");
    ok("MATCH (a:A), (b:B) WHERE b.id >= 1000 AND b.id < 1500 CREATE (a)-[:R {k: b.id}]->(b);");
    EXPECT_EQ(count(*conn), 1500) << "[check: writer-sees-its-own-rows]";
    EXPECT_EQ(count(*reader), 1000) << "[check: other-reader-does-not]";
    ok("COMMIT;");
    EXPECT_EQ(count(*reader), 1500);
    // Une suppression : le lecteur d'avant garde la ligne, celui d'après ne l'a plus.
    ok(*reader, "BEGIN TRANSACTION READ ONLY;");
    ok("MATCH (:A)-[r:R]->(b:B) WHERE b.id < 10 DELETE r;");
    EXPECT_EQ(count(*reader), 1500) << "[check: old-reader-keeps-deleted-rows]";
    EXPECT_EQ(count(*conn), 1490) << "[check: new-reader-without-them]";
    ok(*reader, "COMMIT;");
    EXPECT_EQ(count(*reader), 1490);
    // Après le point de reprise, mêmes comptes.
    ok("CHECKPOINT;");
    EXPECT_EQ(count(*conn), 1490);
    EXPECT_EQ(sumK(*conn), 1499 * 1500 / 2 - 45);
}

// Une liste compactée faite de suites contiguës : lue d'un bloc par suite. Les propriétés de la
// relation et du voisin, puis la somme, sont celles du chemin ligne à ligne ; et le DELETE qui lit
// ces relations (par leur numéro de ligne) les efface toutes.
TEST_F(InMemoryReadShortcut, ContiguousRunsReadInBlocksGiveTheSameRows) {
    ok("MATCH (a:A), (b:B) CREATE (a)-[:R {k: b.id}]->(b);");
    EXPECT_EQ(count(*conn), 3000);
    const int64_t sumAll = 2999 * 3000 / 2;
    EXPECT_EQ(sumK(*conn), sumAll);
    // Une relation sur trois part : la liste, compactée à la fin de cette validation, devient des
    // suites de deux lignes.
    ok("MATCH (:A)-[r:R]->(b:B) WHERE b.id % 3 = 2 DELETE r;");
    int64_t sumKept = 0, sumKeptV = 0, kept = 0;
    for (int64_t i = 0; i < 3000; i++) {
        if (i % 3 != 2) {
            kept++;
            sumKept += i;
            sumKeptV += i + i * 7;
        }
    }
    EXPECT_EQ(count(*conn), kept) << "[check: runs-count]";
    EXPECT_EQ(sumK(*conn), sumKept) << "[check: runs-rel-property]";
    EXPECT_EQ(sumKV(*conn), sumKeptV) << "[check: runs-neighbour-property]";
    // Un lecteur d'avant une nouvelle validation lit encore ligne à ligne (le bloc a une
    // insertion postérieure à lui) : mêmes sommes.
    ok(*reader, "BEGIN TRANSACTION READ ONLY;");
    ok("MATCH (a:A), (b:B) WHERE b.id % 3 = 2 AND b.id < 300 CREATE (a)-[:R {k: b.id + 10000}]->(b);");
    EXPECT_EQ(sumK(*reader), sumKept) << "[check: old-reader-row-by-row]";
    EXPECT_EQ(count(*conn), kept + 100);
    ok(*reader, "COMMIT;");
    // Le DELETE passe par le numéro de ligne de chaque relation lue : tout part.
    ok("MATCH (:A)-[r:R]->(:B) DELETE r;");
    EXPECT_EQ(count(*conn), 0) << "[check: delete-by-row-idx-after-run-scan]";
    ok("MATCH (a:A), (b:B) WHERE b.id < 500 CREATE (a)-[:R {k: b.id}]->(b);");
    EXPECT_EQ(count(*conn), 500);
    EXPECT_EQ(sumK(*conn), 499 * 500 / 2);
    ok("CHECKPOINT;");
    EXPECT_EQ(sumK(*conn), 499 * 500 / 2) << "[check: after-checkpoint]";
}
