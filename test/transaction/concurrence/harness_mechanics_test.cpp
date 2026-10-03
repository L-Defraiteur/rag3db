// La mécanique des cas d'attente sur verrou, éprouvée sur elle-même. La décision de
// Lucie (2 octobre) : l'écrivain attendra sur un verrou par clé au lieu d'échouer. Les
// trois cas qui le prouveront (le premier annule : le second réussit ; le premier
// valide : le second reçoit la clé en double ; un interblocage finit par une erreur
// nommée dans un délai borné) attendent la note de conception de la session cœur C++.
// Ce qu'ils demandent au banc existe déjà et se teste ici :
// - un journal d'événements datés, dans leur ordre réel (Worker::mark, runMarked) ;
// - une attente sur l'événement d'un autre écrivain (Worker::waitFor), là où une
//   barrière ne marche plus — l'autre peut être bloqué par le moteur ;
// - un délai de garde par cas (launch), pour qu'un moteur qui se bloque donne un rouge
//   et non une passe qui ne finit pas.

#ifndef __SINGLE_THREADED__

#include <iostream>

#include "bench_harness.h"
#include "graph_test/private_graph_test.h"

using namespace rag3db::testing;
using namespace rag3db::testing::concurrency;

namespace {

class HarnessMechanics : public EmptyDBTest {
public:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
    }
};

// Un écrivain attend qu'un autre ait marqué un événement, puis marque le sien :
// l'ordre lu après coup est celui qui a eu lieu.
TEST_F(HarnessMechanics, EventsKeepTheRealOrder) {
    SharedMapping mapping(2);
    auto& area = mapping.get();
    bool sawTheOther = false;
    launch(LaunchMode::Thread, Opener{database.get()}, area, [&](Worker& worker) {
        if (worker.index() == 0) {
            std::this_thread::sleep_for(std::chrono::milliseconds(20));
            worker.mark("first");
        } else {
            sawTheOther = worker.waitFor(0, "first", std::chrono::milliseconds(2'000));
            worker.mark("second");
        }
    });
    std::cerr << describeEvents(area);
    EXPECT_TRUE(sawTheOther);
    EXPECT_LT(eventIndex(area, 0, "first"), eventIndex(area, 1, "second"));
    EXPECT_EQ(area.guardExpired.load(), 0u);
}

// Une instruction qui dure plus que la garde : la garde expire, la connexion est
// interrompue, l'instruction échoue par « Interrupted », et la passe continue.
TEST_F(HarnessMechanics, GuardInterruptsAStatementThatOutlivesIt) {
    SharedMapping mapping(1);
    auto& area = mapping.get();
    const auto start = std::chrono::steady_clock::now();
    launch(
        LaunchMode::Thread, Opener{database.get()}, area,
        [](Worker& worker) {
            // Un calcul par ligne, filtré : le moteur ne peut ni factoriser ni
            // compter sans parcourir, et l'interruption est regardée entre deux paquets.
            worker.runMarked("UNWIND range(1, 2000) AS a UNWIND range(1, 2000) AS b "
                             "UNWIND range(1, 2000) AS c WITH a * b + c AS x "
                             "WHERE x % 7 = 3 RETURN count(*);",
                "long");
        },
        std::chrono::milliseconds(300));
    const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::steady_clock::now() - start);
    std::cerr << describe(area) << "  elapsed " << elapsed.count() << " ms\n";
    EXPECT_EQ(area.guardExpired.load(), 1u);
    EXPECT_EQ(area.workers[0].refusals[static_cast<size_t>(Refusal::Interrupted)].load(), 1u);
    EXPECT_GE(eventIndex(area, 0, "long:failed"), 0);
}

// En mode Processus, un écrivain qui dépasse la garde est tué ; launch rend faux.
TEST_F(HarnessMechanics, GuardKillsAWriterProcessThatOutlivesIt) {
    conn.reset();
    database.reset();
    SharedMapping mapping(1);
    auto& area = mapping.get();
    const auto start = std::chrono::steady_clock::now();
    const bool exitedNormally = launch(
        LaunchMode::Process, Opener{nullptr, databasePath, *systemConfig}, area,
        [](Worker&) { std::this_thread::sleep_for(std::chrono::seconds(30)); },
        std::chrono::milliseconds(300));
    const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::steady_clock::now() - start);
    std::cerr << "  elapsed " << elapsed.count() << " ms\n";
    EXPECT_FALSE(exitedNormally);
    EXPECT_EQ(area.guardExpired.load(), 1u);
    EXPECT_LT(elapsed.count(), 10'000);
    createDBAndConn();
}

} // namespace

#endif
