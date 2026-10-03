// Les témoins des verrous, écrits avant les verrous (note de conception
// docs/3-octobre-2026-15h47/01-note-de-conception-les-verrous.md, §6). Les trois écarts
// sont tranchés : annonce en tête (CALL acquire_locks), option A hors annonce
// (instantané par transaction, erreur nommée et transitoire après l'attente si le
// premier a validé), verrou partagé sur les extrémités d'une relation créée.
//
// Chaque cas dit la marche qui doit le rendre vert (known_red.txt). Deux sortes de
// rouge, distinguées par leurs étiquettes : « la fonction n'existe pas »
// (acquire-locks-exists, lock-timeout-exists) et « mauvais comportement » (tout le
// reste). Les erreurs à venir sont reconnues par les fragments de message annoncés à la
// session cœur C++ (classifyRefusal) : « deadlock », « lock … timeout », « could not
// serialize », « Interrupted ».
//
// L'attente se prouve par l'ordre des événements (mark, waitFor, runMarked) : la fin de
// l'écriture de celui qui attend vient après la fin de la transaction de celui qui
// tient le verrou. Chaque cas a son délai de garde (launch) : un moteur qui se bloque
// donne un rouge, pas une passe qui ne finit pas.

#ifndef __SINGLE_THREADED__

#include <signal.h>
#include <sys/wait.h>

#include <atomic>
#include <filesystem>
#include <iostream>
#include <thread>

#include "bench_harness.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "integrity/integrity_checker.h"
#include "processor/result/flat_tuple.h"

using namespace rag3db::common;
using namespace rag3db::testing;
using namespace rag3db::testing::concurrency;

namespace {

constexpr std::chrono::milliseconds CASE_GUARD{20'000};
// Le temps pendant lequel celui qui tient le verrou le garde après que l'autre a commencé
// son écriture : assez long pour que l'écriture de l'autre, si elle n'attend pas, finisse
// avant.
constexpr std::chrono::milliseconds HOLD{300};
constexpr std::chrono::milliseconds EVENT_WAIT{5'000};

class LockBench : public EmptyDBTest {
public:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
        applyBenchSettings(*conn);
    }

    void mustRun(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << "\n" << result->getErrorMessage();
    }

    int64_t queryInt(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << "[check: query] " << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    void launchCase(SharedArea& area, const Scenario& scenario) {
        launch(LaunchMode::Thread, Opener{database.get()}, area, scenario, CASE_GUARD);
        std::cerr << describe(area);
        EXPECT_EQ(area.guardExpired.load(), 0u)
            << "[check: guard] the engine blocked a writer past the case's guard";
        EXPECT_EQ(totalRefusals(area, Refusal::Unexpected), 0u) << "[check: no-unexpected-error] ";
    }

    // Un fait du vérificateur : aucune violation d'intégrité.
    void expectIntegrity() {
        auto violations = integrity::checkLevel1(*conn);
        const auto level2 = integrity::checkLevel2(*conn);
        violations.insert(violations.end(), level2.begin(), level2.end());
        std::string checks;
        for (const auto& violation : violations) {
            checks += "[check: " + violation.invariant + "] ";
        }
        std::cerr << integrity::describe(violations);
        EXPECT_TRUE(violations.empty()) << checks;
    }
};

uint32_t refusals(const SharedArea& area, uint32_t worker, Refusal refusal) {
    return area.workers[worker].refusals[static_cast<size_t>(refusal)].load();
}

// Le détenteur et l'attendant : l'écrivain 0 ouvre sa transaction, écrit (il prend le
// verrou), attend que l'écrivain 1 ait commencé son écriture, la garde HOLD, puis valide
// ou annule (événement « end »). L'écrivain 1 attend la fin de l'écriture du 0, ouvre sa
// transaction, écrit (avec les verrous, il attend), puis valide (« commit »).
struct HolderAndWaiter {
    std::string holderWrite;
    std::string waiterWrite;
    bool holderRollsBack = false;
    // Une instruction qui échoue après l'écriture du détenteur, avant son ROLLBACK : la
    // transaction en échec doit, elle aussi, libérer ses verrous à l'annulation.
    std::string holderFailingStatement;
};

Scenario holderAndWaiter(HolderAndWaiter script) {
    return [script](Worker& worker) {
        if (worker.index() == 0) {
            worker.begin();
            worker.runMarked(script.holderWrite, "write");
            if (!script.holderFailingStatement.empty()) {
                worker.runMarked(script.holderFailingStatement, "failing");
            }
            worker.waitFor(1, "write:start", EVENT_WAIT);
            std::this_thread::sleep_for(HOLD);
            worker.mark("end:start");
            if (script.holderRollsBack) {
                worker.rollback();
            } else {
                worker.commit();
            }
            worker.mark("end:done");
            return;
        }
        // L'attendant écrit quand le détenteur a fini tout ce qu'il fait avant de garder
        // son verrou : son écriture, et son instruction en échec s'il en a une.
        if (script.holderFailingStatement.empty()) {
            worker.waitForAny(0, {"write:done", "write:failed"}, EVENT_WAIT);
        } else {
            worker.waitForAny(0, {"failing:done", "failing:failed"}, EVENT_WAIT);
        }
        worker.begin();
        worker.runMarked(script.waiterWrite, "write");
        worker.commitMarked("commit");
    };
}

// L'écrivain 1 a-t-il attendu ? Sa fin d'écriture vient après la fin de la transaction
// du détenteur.
void expectWaited(const SharedArea& area) {
    const auto holderEnd = eventIndex(area, 0, "end:done");
    auto waiterEnd = eventIndex(area, 1, "write:done");
    if (waiterEnd < 0) {
        waiterEnd = eventIndex(area, 1, "write:failed");
    }
    EXPECT_GT(waiterEnd, holderEnd)
        << "[check: waited] the second writer's write ended before the first writer's "
           "transaction ended: it did not wait for the lock";
}

void expectWaiterSucceeds(const SharedArea& area) {
    EXPECT_GE(eventIndex(area, 1, "write:done"), 0) << "[check: waiter-writes] ";
    EXPECT_GE(eventIndex(area, 1, "commit:done"), 0) << "[check: waiter-commits] ";
}

void expectWaiterRefused(const SharedArea& area, Refusal refusal, const char* name) {
    EXPECT_GE(eventIndex(area, 1, "write:failed"), 0) << "[check: waiter-refused] ";
    EXPECT_EQ(refusals(area, 1, refusal), 1u)
        << "[check: waiter-gets-" << name << "] the second writer must get the named error " << name
        << " after waiting";
    EXPECT_LT(eventIndex(area, 1, "commit:done"), 0) << "[check: waiter-does-not-commit] ";
}

} // namespace

// ── 1. L'attente, prouvée par l'ordre des événements (V1, puis A3′ et A4′) ────────────
// Pour chaque conflit, deux issues : le détenteur valide (le second reçoit l'erreur
// nommée, option A) ; le détenteur annule (le second passe et valide). Aujourd'hui,
// sans verrou, l'écriture du second finit aussitôt : rouge « waited », et le reste selon
// le conflit.

// La même clé primaire (A3′) : après l'attente, le second reçoit l'erreur de clé en
// double (comme PostgreSQL) ; si le détenteur annule, le second insère.
TEST_F(LockBench, SameKeyWaitsThenGetsDuplicateKey) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64);");
    SharedMapping mapping(2);
    auto& area = mapping.get();
    launchCase(area, holderAndWaiter({.holderWrite = "CREATE (:Item {id: 7, v: 0});",
                         .waiterWrite = "CREATE (:Item {id: 7, v: 1});"}));
    expectWaited(area);
    expectWaiterRefused(area, Refusal::DuplicatePrimaryKey, "duplicated-key");
    EXPECT_EQ(queryInt("MATCH (n:Item) WHERE n.id = 7 RETURN count(n);"), 1)
        << "[check: one-row-per-key] ";
}

TEST_F(LockBench, SameKeyWaitsThenInsertsAfterRollback) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64);");
    SharedMapping mapping(2);
    auto& area = mapping.get();
    launchCase(area, holderAndWaiter({.holderWrite = "CREATE (:Item {id: 7, v: 0});",
                         .waiterWrite = "CREATE (:Item {id: 7, v: 1});",
                         .holderRollsBack = true}));
    expectWaited(area);
    expectWaiterSucceeds(area);
    EXPECT_EQ(queryInt("MATCH (n:Item {id: 7}) RETURN n.v;"), 1) << "[check: waiter-value] ";
}

// La même ligne, deux mises à jour de la même colonne (A4′) : aujourd'hui le second
// échoue tout de suite (« Write-write conflict ») ; ensuite il attend, puis reçoit
// l'erreur nommée de l'option A si le détenteur a validé.
TEST_F(LockBench, SameRowUpdateWaitsThenGetsSerializationError) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, a INT64, b INT64);");
    mustRun("CREATE (:Item {id: 1, a: 0, b: 0});");
    SharedMapping mapping(2);
    auto& area = mapping.get();
    launchCase(area, holderAndWaiter({.holderWrite = "MATCH (n:Item {id: 1}) SET n.a = 1;",
                         .waiterWrite = "MATCH (n:Item {id: 1}) SET n.a = 2;"}));
    expectWaited(area);
    expectWaiterRefused(area, Refusal::SerializationFailure, "serialization-error");
    EXPECT_EQ(queryInt("MATCH (n:Item {id: 1}) RETURN n.a;"), 1) << "[check: holder-value] ";
}

TEST_F(LockBench, SameRowUpdateWaitsThenWritesAfterRollback) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, a INT64, b INT64);");
    mustRun("CREATE (:Item {id: 1, a: 0, b: 0});");
    SharedMapping mapping(2);
    auto& area = mapping.get();
    launchCase(area, holderAndWaiter({.holderWrite = "MATCH (n:Item {id: 1}) SET n.a = 1;",
                         .waiterWrite = "MATCH (n:Item {id: 1}) SET n.a = 2;",
                         .holderRollsBack = true}));
    expectWaited(area);
    expectWaiterSucceeds(area);
    EXPECT_EQ(queryInt("MATCH (n:Item {id: 1}) RETURN n.a;"), 2) << "[check: waiter-value] ";
}

// La même ligne, deux colonnes (A4′, §4 de la note) : le verrou est à la ligne, le
// second attend aussi.
TEST_F(LockBench, SameRowOtherColumnWaitsThenGetsSerializationError) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, a INT64, b INT64);");
    mustRun("CREATE (:Item {id: 1, a: 0, b: 0});");
    SharedMapping mapping(2);
    auto& area = mapping.get();
    launchCase(area, holderAndWaiter({.holderWrite = "MATCH (n:Item {id: 1}) SET n.a = 1;",
                         .waiterWrite = "MATCH (n:Item {id: 1}) SET n.b = 1;"}));
    expectWaited(area);
    expectWaiterRefused(area, Refusal::SerializationFailure, "serialization-error");
}

// Suppression contre mise à jour de la même ligne (A4′, C6 sous l'option A).
TEST_F(LockBench, DeleteThenUpdateWaitsThenGetsSerializationError) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, a INT64, b INT64);");
    mustRun("CREATE (:Item {id: 1, a: 0, b: 0});");
    SharedMapping mapping(2);
    auto& area = mapping.get();
    launchCase(area, holderAndWaiter({.holderWrite = "MATCH (n:Item {id: 1}) DELETE n;",
                         .waiterWrite = "MATCH (n:Item {id: 1}) SET n.a = 2;"}));
    expectWaited(area);
    expectWaiterRefused(area, Refusal::SerializationFailure, "serialization-error");
    EXPECT_EQ(queryInt("MATCH (n:Item) RETURN count(n);"), 0) << "[check: row-deleted] ";
}

// Suppression d'un nœud contre création d'une relation vers lui (A4′, C2) : la création
// prend un verrou partagé sur l'extrémité, la suppression l'exclusif ; le second attend,
// puis l'erreur nommée (l'extrémité a disparu).
TEST_F(LockBench, DeleteThenNewRelationWaitsThenGetsSerializationError) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, a INT64, b INT64);");
    mustRun("CREATE REL TABLE Link(FROM Item TO Item, src_id INT64, dst_id INT64);");
    mustRun("CREATE (:Item {id: 1, a: 0, b: 0}), (:Item {id: 2, a: 0, b: 0});");
    SharedMapping mapping(2);
    auto& area = mapping.get();
    launchCase(area, holderAndWaiter({.holderWrite = "MATCH (n:Item {id: 1}) DELETE n;",
                         .waiterWrite = "MATCH (a:Item {id: 1}), (b:Item {id: 2}) "
                                        "CREATE (a)-[:Link {src_id: 1, dst_id: 2}]->(b);"}));
    expectWaited(area);
    expectWaiterRefused(area, Refusal::SerializationFailure, "serialization-error");
    expectIntegrity();
}

// ── 4. L'interblocage (V1) ─────────────────────────────────────────────────────────────
// L'écrivain 0 met à jour la ligne 1 puis la 2 ; l'écrivain 1 la 2 puis la 1. Exactement
// une erreur d'interblocage, l'autre valide. Aujourd'hui : deux « Write-write conflict »
// immédiats, personne ne valide.
TEST_F(LockBench, DeadlockGivesExactlyOneNamedErrorAndTheOtherCommits) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, a INT64, b INT64);");
    mustRun("CREATE (:Item {id: 1, a: 0, b: 0}), (:Item {id: 2, a: 0, b: 0});");
    SharedMapping mapping(2);
    auto& area = mapping.get();
    launchCase(area, [](Worker& worker) {
        const auto first = worker.index() == 0 ? 1 : 2;
        const auto second = worker.index() == 0 ? 2 : 1;
        worker.begin();
        worker.runMarked(
            stringFormat("MATCH (n:Item {id: {}}) SET n.a = {};", first, worker.index() + 10),
            "first");
        worker.waitForAny(1 - worker.index(), {"first:done", "first:failed"}, EVENT_WAIT);
        // L'écrivain 1 demande sa seconde ligne 200 ms après que l'écrivain 0 a demandé la
        // sienne : avec les verrous, l'écrivain 0 attend alors déjà, et c'est la demande
        // de l'écrivain 1 qui ferme le cycle. Sans cet écart, l'issue d'aujourd'hui
        // dépendait de l'ordonnancement (16 fois sur 20).
        if (worker.index() == 1) {
            worker.waitFor(0, "second:start", EVENT_WAIT);
            std::this_thread::sleep_for(std::chrono::milliseconds(200));
        }
        worker.runMarked(
            stringFormat("MATCH (n:Item {id: {}}) SET n.a = {};", second, worker.index() + 10),
            "second");
        worker.commitMarked("commit");
    });
    EXPECT_EQ(totalRefusals(area, Refusal::Deadlock), 1u)
        << "[check: one-deadlock-error] exactly one writer must get the deadlock error";
    EXPECT_EQ(totalCommits(area), 1u) << "[check: other-commits] ";
    EXPECT_EQ(totalRefusals(area, Refusal::WriteWriteConflict), 0u)
        << "[check: no-immediate-conflict] with locks, a conflict waits instead of failing";
}

// ── 5. L'annonce en tête (V2) ──────────────────────────────────────────────────────────
// Les mêmes deux transactions annoncent {1, 2} et {2, 1} : le moteur prend les verrous
// dans un ordre unique ; aucune erreur, les deux valident, l'une après l'autre.
TEST_F(LockBench, AnnouncedLocksNeverDeadlockAndBothCommitInTurn) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, a INT64, b INT64);");
    mustRun("CREATE (:Item {id: 1, a: 0, b: 0}), (:Item {id: 2, a: 0, b: 0});");
    SharedMapping mapping(2);
    auto& area = mapping.get();
    launchCase(area, [](Worker& worker) {
        const auto first = worker.index() == 0 ? 1 : 2;
        const auto second = worker.index() == 0 ? 2 : 1;
        worker.sync();
        worker.begin();
        worker.runMarked(stringFormat("CALL acquire_locks('Item', [{}, {}]);", first, second),
            "announce");
        worker.runMarked(stringFormat("MATCH (n:Item {id: {}}) SET n.a = n.a + 1;", first),
            "first");
        worker.runMarked(stringFormat("MATCH (n:Item {id: {}}) SET n.a = n.a + 1;", second),
            "second");
        worker.commitMarked("commit");
    });
    EXPECT_EQ(totalRefusals(area, Refusal::MissingFunction), 0u)
        << "[check: acquire-locks-exists] CALL acquire_locks does not exist yet";
    uint32_t errors = 0;
    for (auto r = 0u; r < NUM_REFUSALS; ++r) {
        errors += totalRefusals(area, static_cast<Refusal>(r));
    }
    EXPECT_EQ(errors, 0u) << "[check: no-error] ";
    EXPECT_EQ(totalCommits(area), 2u) << "[check: both-commit] ";
    // L'une après l'autre : l'annonce de l'une se termine après le commit de l'autre.
    const bool zeroFirst =
        eventIndex(area, 0, "commit:done") >= 0 &&
        eventIndex(area, 1, "announce:done") > eventIndex(area, 0, "commit:done");
    const bool oneFirst = eventIndex(area, 1, "commit:done") >= 0 &&
                          eventIndex(area, 0, "announce:done") > eventIndex(area, 1, "commit:done");
    EXPECT_TRUE(zeroFirst || oneFirst) << "[check: in-turn] ";
    EXPECT_EQ(queryInt("MATCH (n:Item) RETURN sum(n.a);"), 4) << "[check: all-updates] ";
}

// ── 6. Le rollback et l'erreur libèrent les verrous (V1, T0) ──────────────────────────
// Le rollback : couvert par les cas « …AfterRollback » ci-dessus. L'erreur : le
// détenteur écrit, puis une de ses instructions échoue (la transaction passe en échec,
// T0), puis il annule ; celui qui attendait derrière passe.
TEST_F(LockBench, FailedTransactionReleasesItsLocksAtRollback) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, a INT64, b INT64);");
    mustRun("CREATE (:Item {id: 1, a: 0, b: 0});");
    SharedMapping mapping(2);
    auto& area = mapping.get();
    launchCase(area, holderAndWaiter({.holderWrite = "MATCH (n:Item {id: 1}) SET n.a = 1;",
                         .waiterWrite = "MATCH (n:Item {id: 1}) SET n.a = 2;",
                         .holderRollsBack = true,
                         .holderFailingStatement = "CREATE (:Item {id: 1, a: 9, b: 9});"}));
    EXPECT_EQ(refusals(area, 0, Refusal::DuplicatePrimaryKey), 1u)
        << "[check: holder-error] the holder's failing statement must fail";
    expectWaited(area);
    expectWaiterSucceeds(area);
    EXPECT_EQ(queryInt("MATCH (n:Item {id: 1}) RETURN n.a;"), 2) << "[check: waiter-value] ";
}

// ── 7. L'interruption et le délai pendant une attente (V1) ────────────────────────────
// L'interruption : l'écrivain 1 règle un délai de requête (CALL timeout), puis attend un
// verrou que le détenteur garde plus longtemps ; son instruction échoue par
// « Interrupted », pas avant le délai, et sa transaction est annulée ; le détenteur
// valide.
static void interruptedOrTimedOutWait(LockBench& bench, const std::string& waiterSetting,
    Refusal expected, const char* name, bool settingMayBeMissing) {
    bench.mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, a INT64, b INT64);");
    bench.mustRun("CREATE (:Item {id: 1, a: 0, b: 0});");
    SharedMapping mapping(2);
    auto& area = mapping.get();
    bench.launchCase(area, [waiterSetting](Worker& worker) {
        if (worker.index() == 0) {
            worker.begin();
            worker.runMarked("MATCH (n:Item {id: 1}) SET n.a = 1;", "write");
            worker.waitFor(1, "write:start", EVENT_WAIT);
            // Plus long que le délai de l'attendant.
            std::this_thread::sleep_for(std::chrono::milliseconds(1'500));
            worker.mark("end:start");
            worker.commit();
            worker.mark("end:done");
            return;
        }
        worker.runMarked(waiterSetting, "setting");
        worker.waitForAny(0, {"write:done", "write:failed"}, EVENT_WAIT);
        worker.begin();
        worker.runMarked("MATCH (n:Item {id: 1}) SET n.a = 2;", "write");
        // Après l'erreur, la transaction est annulée : rien ne doit pouvoir valider.
        worker.commitMarked("commit");
    });
    if (settingMayBeMissing) {
        EXPECT_EQ(refusals(area, 1, Refusal::MissingFunction), 0u)
            << "[check: lock-timeout-exists] the lock wait timeout setting does not exist "
               "yet";
    }
    EXPECT_EQ(refusals(area, 1, expected), 1u) << "[check: waiter-gets-" << name << "] ";
    const auto start = eventIndex(area, 1, "write:start");
    const auto end = eventIndex(area, 1, "write:failed");
    const bool waitedTheDelay =
        start >= 0 && end >= 0 && area.events[end].micros - area.events[start].micros >= 250'000;
    EXPECT_TRUE(waitedTheDelay) << "[check: waited-the-delay] the error must come after the "
                                   "delay, not at once";
    EXPECT_LT(eventIndex(area, 1, "commit:done"), 0) << "[check: waiter-does-not-commit] ";
    EXPECT_EQ(totalCommits(area), 1u) << "[check: holder-commits] ";
    EXPECT_EQ(bench.queryInt("MATCH (n:Item {id: 1}) RETURN n.a;"), 1) << "[check: holder-value] ";
}

TEST_F(LockBench, InterruptedWaitGivesInterruptedAndRollsBack) {
    interruptedOrTimedOutWait(*this, "CALL timeout=300;", Refusal::Interrupted, "interrupted",
        false);
}

TEST_F(LockBench, LockTimeoutGivesNamedErrorAndRollsBack) {
    interruptedOrTimedOutWait(*this, "CALL lock_timeout=300;", Refusal::LockTimeout, "lock-timeout",
        true);
}

// ── 8. Le nœud-carrefour (V1, A4′ : verrou partagé sur les extrémités) ───────────────
// Huit écrivains créent chacun, dans sa transaction, une relation vers le même nœud ;
// aucun n'attend l'autre : chacun voit les huit écritures finies avant de valider. Vert
// aujourd'hui (aucun verrou) ; il resterait vert sous le partagé, et deviendrait rouge
// sous un verrou exclusif sur les extrémités (celui de Neo4j). Garde-fou de l'écart 3.
TEST_F(LockBench, HubNodeWritersNeverWaitForEachOther) {
    static constexpr uint32_t numWriters = 8;
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, a INT64, b INT64);");
    mustRun("CREATE REL TABLE Link(FROM Item TO Item, src_id INT64, dst_id INT64);");
    mustRun("UNWIND range(0, 8) AS i CREATE (:Item {id: i, a: 0, b: 0});");
    SharedMapping mapping(numWriters);
    auto& area = mapping.get();
    std::atomic<uint32_t> sawEveryWrite{0};
    launchCase(area, [&](Worker& worker) {
        const auto own = worker.index() + 1;
        worker.begin();
        worker.writeInTurn(stringFormat("MATCH (n:Item {id: {}}), (h:Item {id: 0}) "
                                        "CREATE (n)-[:Link {src_id: {}, dst_id: 0}]->(h);",
            own, own));
        bool all = true;
        for (auto other = 0u; other < numWriters; ++other) {
            all = all && worker.waitForAny(other, {"write:done", "write:failed"},
                             std::chrono::milliseconds(3'000));
        }
        sawEveryWrite.fetch_add(all ? 1 : 0);
        worker.commitMarked("commit");
    });
    EXPECT_EQ(sawEveryWrite.load(), numWriters)
        << "[check: no-one-waits] a writer waited for another on the hub node";
    EXPECT_EQ(totalCommits(area), numWriters) << "[check: all-commit] ";
    EXPECT_EQ(queryInt("MATCH ()-[r:Link]->() RETURN count(r);"), numWriters)
        << "[check: rel-count] ";
    expectIntegrity();
}

// ── 9. Rouvrir une base puis interroger l'index vectoriel tout de suite ──────────────
// Signalé le 3 octobre : « Index … is not loaded yet » vu une fois dans e2e_code à la
// réouverture, non reproduit. Hypothèse de la session cœur C++ (journal §6) : une
// requête a touché l'index avant la fin du chargement de l'extension. Deux formes,
// répétées : (a) rouvrir, charger l'extension, interroger aussitôt ; (b) un second fil
// interroge l'index en boucle pendant que le premier charge l'extension — avant le
// chargement, l'erreur attendue est « function … is not defined », jamais « not loaded
// yet ». Invariant : le message « is not loaded yet » n'apparaît jamais.
class IndexReopen : public LockBench {
public:
    void createIndexedTable() {
        loadVectorExtension(*conn);
        mustRun("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
        mustRun("UNWIND range(0, 299) AS i CREATE (:Doc {id: i, vec: [CAST(i % 17 AS FLOAT), "
                "CAST(i % 23 AS FLOAT), CAST(i % 29 AS FLOAT), CAST(i AS FLOAT)]});");
        mustRun("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    }
    static constexpr const char* QUERY =
        "CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [1.0, 2.0, 3.0, 4.0], 5) RETURN "
        "node.id;";
};

TEST_F(IndexReopen, QueryRightAfterReopenAndLoad) {
    createIndexedTable();
    uint32_t notLoaded = 0;
    uint32_t otherErrors = 0;
    std::string first;
    for (auto round = 0; round < 30; ++round) {
        createDBAndConn();
        loadVectorExtension(*conn);
        auto result = conn->query(QUERY);
        if (!result->isSuccess()) {
            const auto message = result->getErrorMessage();
            (message.find("is not loaded yet") != std::string::npos ? notLoaded : otherErrors)++;
            if (first.empty()) {
                first = message;
            }
        }
    }
    std::cerr << "  not loaded yet: " << notLoaded << ", other errors: " << otherErrors
              << (first.empty() ? "" : ", first: " + first) << "\n";
    EXPECT_EQ(notLoaded, 0u) << "[check: index-loaded] ";
    EXPECT_EQ(otherErrors, 0u) << "[check: query-succeeds] ";
}

TEST_F(IndexReopen, QueryWhileTheExtensionLoads) {
    createIndexedTable();
    uint32_t notLoaded = 0;
    uint32_t unexpected = 0;
    std::string first;
    for (auto round = 0; round < 30; ++round) {
        createDBAndConn();
        std::atomic<bool> loaded{false};
        std::thread reader([&] {
            rag3db::main::Connection connection(database.get());
            for (auto attempt = 0; attempt < 2'000; ++attempt) {
                auto result = connection.query(QUERY);
                if (result->isSuccess()) {
                    if (loaded.load()) {
                        return;
                    }
                    continue;
                }
                const auto message = result->getErrorMessage();
                if (message.find("is not loaded yet") != std::string::npos) {
                    ++notLoaded;
                } else if (message.find("is not defined") == std::string::npos) {
                    ++unexpected;
                }
                if (first.empty() && message.find("is not defined") == std::string::npos) {
                    first = message;
                }
                if (loaded.load()) {
                    return;
                }
            }
        });
        loadVectorExtension(*conn);
        loaded.store(true);
        reader.join();
    }
    std::cerr << "  not loaded yet: " << notLoaded << ", unexpected: " << unexpected
              << (first.empty() ? "" : ", first: " + first) << "\n";
    EXPECT_EQ(notLoaded, 0u) << "[check: index-loaded] ";
    EXPECT_EQ(unexpected, 0u) << "[check: no-unexpected-error] ";
}

// (c) La cause, isolée par la session de l'arbre principal (3 octobre) : un index écrit
// par CHECKPOINT, puis un DROP_VECTOR_INDEX qui ne vit que dans le journal, puis la mort
// du processus sans fermeture. Au rejeu, le catalogue n'a plus l'index, mais la table en
// garde un exemplaire non chargé : recréer l'index lève « Index … is not loaded yet ».
// Ici, un processus fils fait tout jusqu'à sa mort par SIGKILL ; le père rouvre, recrée
// l'index et l'interroge. Invariant : la recréation et la requête réussissent, sans
// « is not loaded yet ». Déterministe.
TEST_F(IndexReopen, DropAfterCheckpointThenCrashLeavesAnUnloadedIndex) {
    conn.reset();
    database.reset();
    const auto pid = fork();
    if (pid == 0) {
        disableCoreDumps();
        try {
            rag3db::main::Database childDatabase(databasePath, *systemConfig);
            rag3db::main::Connection childConnection(&childDatabase);
            loadVectorExtension(childConnection);
            for (const auto* query : {"CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);",
                     "UNWIND range(0, 299) AS i CREATE (:Doc {id: i, vec: [CAST(i % 17 AS FLOAT), "
                     "CAST(i % 23 AS FLOAT), CAST(i % 29 AS FLOAT), CAST(i AS FLOAT)]});",
                     "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2', "
                     "skip_if_exists := true);",
                     "CHECKPOINT;", "CALL DROP_VECTOR_INDEX('Doc', 'doc_index');"}) {
                if (!childConnection.query(query)->isSuccess()) {
                    _exit(2);
                }
            }
            // Mourir ici, base ouverte : sortir du bloc la fermerait proprement (point de
            // reprise final), et le DROP ne vivrait plus seulement dans le journal.
            kill(getpid(), SIGKILL);
        } catch (...) {
            _exit(3);
        }
        _exit(4);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    ASSERT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
        << "[check: setup] the child failed before its kill";
    createDBAndConn();
    loadVectorExtension(*conn);
    std::cerr << "  indexes after replay: " << queryInt("CALL SHOW_INDEXES() RETURN count(*);")
              << "\n";
    auto recreate = conn->query("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := "
                                "'l2', skip_if_exists := true);");
    const auto recreateError = recreate->isSuccess() ? "" : recreate->getErrorMessage();
    std::cerr << "  recreate: " << (recreateError.empty() ? "ok" : recreateError) << "\n";
    EXPECT_EQ(recreateError.find("is not loaded yet"), std::string::npos)
        << "[check: index-loaded] ";
    EXPECT_TRUE(recreateError.empty()) << "[check: index-recreated] ";
    auto query = conn->query(QUERY);
    std::cerr << "  query: " << (query->isSuccess() ? "ok" : query->getErrorMessage()) << "\n";
    EXPECT_TRUE(query->isSuccess()) << "[check: query-succeeds] ";
}

#endif
