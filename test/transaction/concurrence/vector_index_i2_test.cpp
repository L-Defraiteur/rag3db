// Les témoins de la marche I2 (ticket
// 2026-10-18-sur-une-table-hnsw-les-ecrivains-passent-un-par-un, forme retenue) : l'index d'une
// table HNSW n'est pris en exclusif qu'à la validation, et la maintenance du graphe pour un SET de
// vecteur ou une suppression (update, delete_, finalizeDelete) est reportée au commit, avec un état
// par transaction. Aujourd'hui (marche I1), chaque écrivain tient l'index en exclusif de sa
// première écriture à sa validation, et le SET et la suppression réécrivent le graphe pendant la
// transaction.
//
// Une table de 1 000 lignes, vecteurs FLOAT[4], indexée par HNSW (l2). A ouvre un bloc, écrit, puis
// garde sa transaction ouverte HOLD_MS ; pendant ce temps, B ou un lecteur agit en automatique.
// - VectorIndexI2.SetsOnDifferentRowsDoNotWait : A et B changent le vecteur de deux lignes
//   différentes. vector-sets-on-different-rows-do-not-wait : le SET de B finit avant la validation
//   de A.
// - VectorIndexI2.OverlappingDeletesNeitherBlockNorConflict : A et B suppriment deux lignes aux
//   vecteurs presque égaux (leurs voisinages se recouvrent, la forme de H3).
//   overlapping-deletes-do-not-block : la suppression de B finit avant la validation de A ;
//   overlapping-deletes-do-not-conflict : elle n'est pas refusée ; graph-right-after-overlapping-
//   deletes : après les deux validations, une recherche exhaustive rend chaque ligne restante une
//   fois et aucune supprimée.
// - VectorIndexI2.ReaderDuringAnOpenSetSeesTheGraphBefore : A change le vecteur d'une ligne vers un
//   point lointain et garde sa transaction ouverte ; un lecteur cherche par l'ancien vecteur.
//   reader-during-an-open-set-does-not-wait : la recherche ne dure pas plus d'une seconde ;
//   reader-during-an-open-set-sees-the-graph-before : elle rend la ligne en tête, par son ancien
//   vecteur, et une recherche par le nouveau ne la rend pas.

#ifndef __SINGLE_THREADED__

#include <atomic>
#include <chrono>
#include <iostream>
#include <map>
#include <string>
#include <thread>
#include <vector>

#include "bench_harness.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

using Clock = std::chrono::steady_clock;
constexpr int64_t NUM_ROWS = 1'000;
constexpr int64_t HOLD_MS = 1'500;
// Deux lignes aux vecteurs presque égaux : leurs voisinages dans le graphe se recouvrent.
constexpr int64_t TWIN_A = 500;
constexpr int64_t TWIN_B = 501;

double msSince(Clock::time_point start) {
    return std::chrono::duration<double, std::milli>(Clock::now() - start).count();
}

class VectorIndexI2 : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
        concurrency::applyBenchSettings(*conn);
        concurrency::loadVectorExtension(*conn);
        ok(*conn, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
        // Les lignes sur une spirale ; les deux jumelles au même point, à un millième près.
        ok(*conn, stringFormat("UNWIND range(0, {}) AS i CREATE (:Doc {id: i, vec: [cos(i * 0.1) * "
                               "i, sin(i * 0.1) * i, i % 17, i % 5]});",
                      NUM_ROWS - 1));
        ok(*conn, stringFormat("MATCH (d:Doc {id: {}}), (t:Doc {id: {}}) SET d.vec = [t.vec[1] + "
                               "0.001, t.vec[2], t.vec[3], t.vec[4]];",
                      TWIN_B, TWIN_A));
        ok(*conn, "CALL CREATE_VECTOR_INDEX('Doc', 'doc_vec', 'vec', metric := 'l2');");
    }

    static void ok(rag3db::main::Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess())
            << "[check: query] " << query.substr(0, 160) << " : " << result->getErrorMessage();
    }

    static std::string vectorOf(rag3db::main::Connection& connection, int64_t id) {
        auto result = connection.query(stringFormat("MATCH (d:Doc {id: {}}) RETURN d.vec;", id));
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->toString() :
                   "";
    }

    // Les identifiants rendus par une recherche, dans l'ordre, ou « ERREUR : … ».
    static std::vector<std::string> search(rag3db::main::Connection& connection,
        const std::string& vector, int64_t k) {
        auto result = connection.query(stringFormat("CALL QUERY_VECTOR_INDEX('Doc', 'doc_vec', {}, "
                                                    "{}, efs := {}) RETURN node.id;",
            vector, k, std::max<int64_t>(k, 64)));
        if (!result->isSuccess()) {
            return {"ERREUR : " + result->getErrorMessage().substr(0, 160)};
        }
        std::vector<std::string> out;
        while (result->hasNext()) {
            out.push_back(result->getNext()->getValue(0)->toString());
        }
        return out;
    }

    // A ouvre un bloc et y joue sa requête ; other joue la sienne en automatique pendant que A
    // garde sa transaction ouverte HOLD_MS, puis A valide. Rend (fin de other, départ de la
    // validation de A, erreur de other), en ms depuis le départ de other.
    struct Race {
        double otherEnd;
        double commitStart;
        std::string otherError;
    };
    Race race(rag3db::main::Connection& a, const std::string& aQuery,
        rag3db::main::Connection& other, const std::string& otherQuery) {
        ok(a, "BEGIN TRANSACTION;");
        ok(a, aQuery);
        const auto start = Clock::now();
        std::atomic<double> otherEnd{-1};
        std::string otherError;
        std::thread thread([&] {
            auto result = other.query(otherQuery);
            otherError = result->isSuccess() ? "" : result->getErrorMessage();
            otherEnd = msSince(start);
        });
        std::this_thread::sleep_for(std::chrono::milliseconds(HOLD_MS));
        const auto commitStart = msSince(start);
        ok(a, "COMMIT;");
        thread.join();
        return {otherEnd.load(), commitStart, otherError};
    }
};

TEST_F(VectorIndexI2, SetsOnDifferentRowsDoNotWait) {
    rag3db::main::Connection a(database.get());
    rag3db::main::Connection b(database.get());
    const auto r = race(a, "MATCH (d:Doc {id: 10}) SET d.vec = [1, 2, 3, 4];", b,
        "MATCH (d:Doc {id: 900}) SET d.vec = [5, 6, 7, 8];");
    std::cerr << "  [I2] deux SET : celui de B finit à " << r.otherEnd << " ms ("
              << (r.otherError.empty() ? "validé" : "refusé : " + r.otherError.substr(0, 120))
              << "), la validation de A part à " << r.commitStart << " ms\n";
    EXPECT_TRUE(r.otherError.empty() && r.otherEnd < r.commitStart)
        << "[check: vector-sets-on-different-rows-do-not-wait] B's SET ended at " << r.otherEnd
        << " ms, A's COMMIT started at " << r.commitStart << " ms";
}

TEST_F(VectorIndexI2, OverlappingDeletesNeitherBlockNorConflict) {
    rag3db::main::Connection a(database.get());
    rag3db::main::Connection b(database.get());
    const auto r = race(a, stringFormat("MATCH (d:Doc {id: {}}) DELETE d;", TWIN_A), b,
        stringFormat("MATCH (d:Doc {id: {}}) DELETE d;", TWIN_B));
    std::cerr << "  [I2] deux suppressions voisines : celle de B finit à " << r.otherEnd << " ms ("
              << (r.otherError.empty() ? "validée" : "refusée : " + r.otherError.substr(0, 120))
              << "), la validation de A part à " << r.commitStart << " ms\n";
    EXPECT_LT(r.otherEnd, r.commitStart)
        << "[check: overlapping-deletes-do-not-block] B's DELETE ended at " << r.otherEnd
        << " ms, A's COMMIT started at " << r.commitStart << " ms";
    EXPECT_TRUE(r.otherError.empty())
        << "[check: overlapping-deletes-do-not-conflict] " << r.otherError;
    // Le graphe validé : chaque ligne restante une fois, aucune supprimée.
    const auto all = search(*conn, "[0, 0, 0, 0]", NUM_ROWS);
    std::map<std::string, int> seen;
    for (const auto& id : all) {
        seen[id]++;
    }
    const auto expectedLeft = NUM_ROWS - 1 - (r.otherError.empty() ? 1 : 0);
    int64_t twice = 0;
    for (const auto& [id, count] : seen) {
        twice += count > 1;
    }
    const bool deletedSeen = seen.contains(std::to_string(TWIN_A)) ||
                             (r.otherError.empty() && seen.contains(std::to_string(TWIN_B)));
    std::cerr << "  [I2] après les deux : " << seen.size() << " lignes rendues sur " << expectedLeft
              << ", " << twice << " en double, une supprimée rendue : " << deletedSeen << "\n";
    EXPECT_TRUE(static_cast<int64_t>(seen.size()) == expectedLeft && twice == 0 && !deletedSeen)
        << "[check: graph-right-after-overlapping-deletes] " << seen.size() << " rows for "
        << expectedLeft << ", " << twice << " twice, a deleted row returned: " << deletedSeen;
}

TEST_F(VectorIndexI2, ReaderDuringAnOpenSetSeesTheGraphBefore) {
    constexpr int64_t MOVED = 300;
    const auto before = vectorOf(*conn, MOVED);
    ASSERT_FALSE(before.empty()) << "[check: plan] no vector for row " << MOVED;
    const std::string after = "[100000, 100000, 100000, 100000]";
    rag3db::main::Connection a(database.get());
    rag3db::main::Connection reader(database.get());
    ok(a, "BEGIN TRANSACTION;");
    ok(a, stringFormat("MATCH (d:Doc {id: {}}) SET d.vec = {};", MOVED, after));
    const auto start = Clock::now();
    const auto byOld = search(reader, before, 1);
    const auto byNew = search(reader, after, 1);
    const auto readMs = msSince(start);
    ok(a, "COMMIT;");
    std::cerr << "  [I2] lecture pendant un SET ouvert : " << readMs
              << " ms ; par l'ancien vecteur : " << (byOld.empty() ? "rien" : byOld[0])
              << " ; par le nouveau : " << (byNew.empty() ? "rien" : byNew[0]) << "\n";
    EXPECT_LE(readMs, 1'000) << "[check: reader-during-an-open-set-does-not-wait] the two searches "
                                "took "
                             << readMs << " ms";
    const auto moved = std::to_string(MOVED);
    EXPECT_TRUE(!byOld.empty() && byOld[0] == moved && (byNew.empty() || byNew[0] != moved))
        << "[check: reader-during-an-open-set-sees-the-graph-before] by the old vector: "
        << (byOld.empty() ? "nothing" : byOld[0])
        << ", by the new one: " << (byNew.empty() ? "nothing" : byNew[0]);
}

} // namespace

#endif
