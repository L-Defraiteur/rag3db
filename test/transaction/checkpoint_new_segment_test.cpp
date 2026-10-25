// V2-1b, pièce 1 : au point de reprise, un segment de colonne changé est rebâti sur des pages
// neuves (plus de page ombre, plus de mutation de l'objet persistant) ; un segment inchangé est
// repris tel quel, partagé ; l'ancien segment rend ses pages à sa mort, et elles redeviennent
// libres au point de reprise suivant. Ce que ça doit garder vrai : chaque type de colonne se
// relit juste après un point de reprise et une réouverture (entiers, chaînes à dictionnaire,
// listes, structs, relations), et le fichier ne grossit pas à chaque cycle de mise à jour
// (les pages des anciens segments sont réemployées).
#include <filesystem>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class CheckpointNewSegment : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed: the subject is its pages";
        }
        createDBAndConn();
    }
    void run(const std::string& q) const {
        auto result = conn->query(q);
        ASSERT_TRUE(result->isSuccess()) << q << " : " << result->getErrorMessage();
    }
    std::string row(const std::string& q) const {
        auto result = conn->query(q);
        if (!result->isSuccess()) {
            return "REFUS " + result->getErrorMessage();
        }
        return result->hasNext() ? result->getNext()->toString() : "AUCUNE LIGNE";
    }
    void reopen() {
        conn.reset();
        database.reset();
        createDBAndConn();
    }
    uintmax_t fileSize() const { return std::filesystem::file_size(databasePath); }
};

// Chaque type de colonne : une ligne changée au milieu d'un gros segment, le point de reprise
// rebâtit ce segment seul, tout se relit, avant et après réouverture.
TEST_F(CheckpointNewSegment, OneChangedRowPerTypeReadsBackAfterCheckpointAndReopen) {
    run("CREATE NODE TABLE T(id INT64 PRIMARY KEY, n INT64, s STRING, l INT64[], "
        "st STRUCT(a INT64, b STRING), d DOUBLE);");
    run("UNWIND range(1, 20000) AS i CREATE (:T {id: i, n: i * 3, s: 'chaîne ' + CAST(i AS "
        "STRING), l: [i, i + 1], st: {a: i, b: 'b' + CAST(i AS STRING)}, d: i * 0.5});");
    run("CHECKPOINT;");
    run("MATCH (t:T {id: 10000}) SET t.n = -1, t.s = 'changée', t.l = [7, 8, 9], "
        "t.st = {a: -2, b: 'bb'}, t.d = -0.25;");
    run("CHECKPOINT;");
    const auto expected = "-1|changée|[7,8,9]|{a: -2, b: bb}|-0.250000\n";
    EXPECT_EQ(row("MATCH (t:T {id: 10000}) RETURN t.n, t.s, t.l, t.st, t.d;"), expected);
    EXPECT_EQ(row("MATCH (t:T {id: 9999}) RETURN t.n, t.s, t.l, t.st, t.d;"),
        "29997|chaîne 9999|[9999,10000]|{a: 9999, b: b9999}|4999.500000\n");
    EXPECT_EQ(row("MATCH (t:T) RETURN count(*), sum(t.n);"), "20000|599999999\n");
    reopen();
    EXPECT_EQ(row("MATCH (t:T {id: 10000}) RETURN t.n, t.s, t.l, t.st, t.d;"), expected);
    EXPECT_EQ(row("MATCH (t:T {id: 1}) RETURN t.n, t.s, t.l, t.st, t.d;"),
        "3|chaîne 1|[1,2]|{a: 1, b: b1}|0.500000\n");
    EXPECT_EQ(row("MATCH (t:T) RETURN count(*), sum(t.n);"), "20000|599999999\n");
}

// Une relation à propriétés : la même chose par le chemin CSR.
TEST_F(CheckpointNewSegment, OneChangedRelationReadsBackAfterCheckpointAndReopen) {
    run("CREATE NODE TABLE A(id INT64 PRIMARY KEY);");
    run("CREATE REL TABLE R(FROM A TO A, w INT64, name STRING);");
    run("UNWIND range(1, 3000) AS i CREATE (:A {id: i});");
    run("MATCH (a:A), (b:A) WHERE b.id = a.id % 3000 + 1 CREATE (a)-[:R {w: a.id, name: 'r' + "
        "CAST(a.id AS STRING)}]->(b);");
    run("CHECKPOINT;");
    run("MATCH (a:A {id: 1500})-[r:R]->() SET r.w = -5, r.name = 'changée';");
    run("CHECKPOINT;");
    EXPECT_EQ(row("MATCH (a:A {id: 1500})-[r:R]->() RETURN r.w, r.name;"), "-5|changée\n");
    EXPECT_EQ(row("MATCH ()-[r:R]->() RETURN count(*), sum(r.w);"), "3000|4499995\n");
    reopen();
    EXPECT_EQ(row("MATCH (a:A {id: 1500})-[r:R]->() RETURN r.w, r.name;"), "-5|changée\n");
    EXPECT_EQ(row("MATCH (a:A {id: 1501})-[r:R]->() RETURN r.w, r.name;"), "1501|r1501\n");
    EXPECT_EQ(row("MATCH ()-[r:R]->() RETURN count(*), sum(r.w);"), "3000|4499995\n");
}

// Une ligne changée fait rebâtir son segment sur des pages neuves, et les pages de l'ancien
// segment redeviennent libres au même point de reprise : le nombre de pages libres grandit de
// la taille du segment (25 pages pour 50 000 valeurs sur 16 bits, mesuré par storage_info).
// Avec la réécriture en place d'avant, seules les pages de métadonnées du point de reprise
// précédent revenaient (deux : rouge sur master). La valeur neuve reste dans l'intervalle des
// valeurs du segment et le segment ne remplit pas ses pages exactement : dans ces deux cas,
// l'ancien chemin réécrivait déjà hors place et rendait les pages lui aussi.
TEST_F(CheckpointNewSegment, UpdatingOneRowRebuildsItsSegmentOnNewPages) {
    run("CREATE NODE TABLE T(id INT64 PRIMARY KEY, n INT64);");
    run("UNWIND range(1, 50000) AS i CREATE (:T {id: i, n: i});");
    run("CHECKPOINT;");
    const auto freeBefore = std::stoll(row("CALL fsm_info() RETURN coalesce(sum(num_pages), 0);"));
    run("MATCH (t:T {id: 25000}) SET t.n = 2;");
    run("CHECKPOINT;");
    const auto freeAfter = std::stoll(row("CALL fsm_info() RETURN coalesce(sum(num_pages), 0);"));
    const auto segmentPages = (50000 * 16 / 8) / RAG3DB_PAGE_SIZE;
    EXPECT_GE(freeAfter - freeBefore, segmentPages / 2)
        << "the old segment's pages did not come back: rewritten in place (" << freeBefore
        << " -> " << freeAfter << " free pages, segment of about " << segmentPages << " pages)";
    EXPECT_EQ(row("MATCH (t:T {id: 25000}) RETURN t.n;"), "2\n");
    EXPECT_EQ(row("MATCH (t:T) RETURN sum(t.n);"), "1250000002\n");
}

// Les pages d'un segment remplacé reviennent : des cycles répétés de mise à jour et de point
// de reprise ne font pas grossir le fichier au-delà du premier (les anciennes pages, rendues à
// la mort de l'ancien bloc, sont libres au point de reprise suivant et réemployées).
TEST_F(CheckpointNewSegment, ReplacedSegmentsGiveTheirPagesBack) {
    run("CREATE NODE TABLE T(id INT64 PRIMARY KEY, n INT64, s STRING);");
    run("UNWIND range(1, 50000) AS i CREATE (:T {id: i, n: i, s: 'v' + CAST(i AS STRING)});");
    run("CHECKPOINT;");
    uintmax_t sizes[6] = {};
    for (auto cycle = 0; cycle < 6; cycle++) {
        run("MATCH (t:T) SET t.n = t.n + 1, t.s = 's' + CAST(" + std::to_string(cycle) +
            " AS STRING);");
        run("CHECKPOINT;");
        sizes[cycle] = fileSize();
    }
    EXPECT_EQ(row("MATCH (t:T {id: 7}) RETURN t.n, t.s;"), "13|s5\n");
    // Après le deuxième cycle les anciennes pages sont réemployées : le fichier ne grossit plus
    // (une marge d'une page de métadonnées par cycle est tolérée).
    for (auto cycle = 2; cycle < 6; cycle++) {
        EXPECT_LE(sizes[cycle], sizes[1] + 2 * cycle * RAG3DB_PAGE_SIZE)
            << "cycle " << cycle << " : " << sizes[cycle] << " octets contre " << sizes[1]
            << " au cycle 1";
    }
    reopen();
    EXPECT_EQ(row("MATCH (t:T {id: 7}) RETURN t.n, t.s;"), "13|s5\n");
    EXPECT_EQ(row("MATCH (t:T) RETURN count(*);"), "50000\n");
}

} // namespace
