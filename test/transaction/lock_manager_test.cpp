// Le gestionnaire de verrous des écritures parallèles, seul (marche V1) : sans table ni
// stockage, par des fils qui prennent et rendent des ressources. L'attente se prouve par
// l'ordre des événements — celui qui attend n'obtient son verrou qu'après la libération —, et
// isWaiting dit qu'un fil est réellement en file avant qu'on libère.

#ifndef __SINGLE_THREADED__

#include <atomic>
#include <chrono>
#include <future>
#include <thread>
#include <vector>

#include "gtest/gtest.h"
#include "transaction/lock_manager.h"

using namespace rag3db::transaction;
using namespace std::chrono_literals;

namespace {

using clock_t_ = LockManager::clock;

class LockManagerTest : public ::testing::Test {
protected:
    static constexpr rag3db::common::table_id_t TABLE = 7;

    static LockResource row(const std::string& key) { return LockResource::row(TABLE, key); }
    static clock_t_::time_point soon() { return clock_t_::now() + 10s; }

    // Attend que la transaction soit en file (ou échoue le test après deux secondes).
    void waitUntilWaiting(rag3db::common::transaction_t transaction) {
        const auto limit = clock_t_::now() + 2s;
        while (!locks.isWaiting(transaction)) {
            ASSERT_LT(clock_t_::now(), limit) << "transaction " << transaction << " never waited";
            std::this_thread::sleep_for(1ms);
        }
    }

    LockManager locks;
};

TEST_F(LockManagerTest, AnExclusiveLockMakesTheNextWriterWaitUntilItIsReleased) {
    ASSERT_EQ(locks.acquire(1, row("a"), LockMode::EXCLUSIVE, soon()), LockOutcome::ACQUIRED);
    std::atomic<bool> released{false};
    std::atomic<bool> acquiredAfterRelease{false};
    auto second = std::async(std::launch::async, [&] {
        const auto outcome = locks.acquire(2, row("a"), LockMode::EXCLUSIVE, soon());
        acquiredAfterRelease = released.load();
        return outcome;
    });
    waitUntilWaiting(2);
    EXPECT_EQ(second.wait_for(50ms), std::future_status::timeout) << "the second writer must wait";
    released = true;
    locks.releaseAll(1);
    EXPECT_EQ(second.get(), LockOutcome::ACQUIRED);
    EXPECT_TRUE(acquiredAfterRelease);
    EXPECT_EQ(locks.getNumLocksHeld(2), 1u);
    locks.releaseAll(2);
    EXPECT_EQ(locks.getNumResources(), 0u);
}

TEST_F(LockManagerTest, SharedLocksDoNotWaitForEachOther) {
    for (rag3db::common::transaction_t transaction = 1; transaction <= 8; transaction++) {
        EXPECT_EQ(locks.acquire(transaction, row("hub"), LockMode::SHARED, clock_t_::now()),
            LockOutcome::ACQUIRED)
            << transaction;
    }
}

TEST_F(LockManagerTest, AnExclusiveLockWaitsForEverySharedHolder) {
    ASSERT_EQ(locks.acquire(1, row("hub"), LockMode::SHARED, soon()), LockOutcome::ACQUIRED);
    ASSERT_EQ(locks.acquire(2, row("hub"), LockMode::SHARED, soon()), LockOutcome::ACQUIRED);
    auto writer = std::async(std::launch::async,
        [&] { return locks.acquire(3, row("hub"), LockMode::EXCLUSIVE, soon()); });
    waitUntilWaiting(3);
    locks.releaseAll(1);
    EXPECT_EQ(writer.wait_for(50ms), std::future_status::timeout) << "one shared holder is left";
    locks.releaseAll(2);
    EXPECT_EQ(writer.get(), LockOutcome::ACQUIRED);
}

TEST_F(LockManagerTest, ASharedLockDoesNotOvertakeAWaitingWriter) {
    ASSERT_EQ(locks.acquire(1, row("hub"), LockMode::SHARED, soon()), LockOutcome::ACQUIRED);
    auto writer = std::async(std::launch::async,
        [&] { return locks.acquire(2, row("hub"), LockMode::EXCLUSIVE, soon()); });
    waitUntilWaiting(2);
    std::atomic<bool> writerDone{false};
    std::atomic<bool> sharedCameAfterTheWriter{false};
    auto lateShared = std::async(std::launch::async, [&] {
        const auto outcome = locks.acquire(3, row("hub"), LockMode::SHARED, soon());
        sharedCameAfterTheWriter = writerDone.load();
        return outcome;
    });
    waitUntilWaiting(3);
    locks.releaseAll(1);
    EXPECT_EQ(writer.get(), LockOutcome::ACQUIRED);
    writerDone = true;
    locks.releaseAll(2);
    EXPECT_EQ(lateShared.get(), LockOutcome::ACQUIRED);
    EXPECT_TRUE(sharedCameAfterTheWriter);
}

TEST_F(LockManagerTest, ATransactionTakesAgainWhatItHoldsAndUpgradesWhenAlone) {
    ASSERT_EQ(locks.acquire(1, row("a"), LockMode::SHARED, soon()), LockOutcome::ACQUIRED);
    EXPECT_EQ(locks.acquire(1, row("a"), LockMode::SHARED, clock_t_::now()),
        LockOutcome::ACQUIRED);
    EXPECT_EQ(locks.acquire(1, row("a"), LockMode::EXCLUSIVE, clock_t_::now()),
        LockOutcome::ACQUIRED);
    EXPECT_EQ(locks.acquire(1, row("a"), LockMode::SHARED, clock_t_::now()),
        LockOutcome::ACQUIRED);
    EXPECT_EQ(locks.getNumLocksHeld(1), 1u);
    // Il est bien exclusif maintenant.
    EXPECT_EQ(locks.acquire(2, row("a"), LockMode::SHARED, clock_t_::now() + 30ms),
        LockOutcome::TIMEOUT);
}

TEST_F(LockManagerTest, TwoWritersInOppositeOrderGetExactlyOneDeadlock) {
    ASSERT_EQ(locks.acquire(1, row("a"), LockMode::EXCLUSIVE, soon()), LockOutcome::ACQUIRED);
    ASSERT_EQ(locks.acquire(2, row("b"), LockMode::EXCLUSIVE, soon()), LockOutcome::ACQUIRED);
    auto first = std::async(std::launch::async,
        [&] { return locks.acquire(1, row("b"), LockMode::EXCLUSIVE, soon()); });
    waitUntilWaiting(1);
    // La demande qui fermerait le cycle est refusée tout de suite, sans attendre.
    const auto before = clock_t_::now();
    EXPECT_EQ(locks.acquire(2, row("a"), LockMode::EXCLUSIVE, soon()), LockOutcome::DEADLOCK);
    EXPECT_LT(clock_t_::now() - before, 1s);
    EXPECT_FALSE(locks.isWaiting(2));
    // Celui qui a reçu l'erreur annule : l'autre passe.
    locks.releaseAll(2);
    EXPECT_EQ(first.get(), LockOutcome::ACQUIRED);
}

TEST_F(LockManagerTest, ACycleOfThreeIsDetected) {
    ASSERT_EQ(locks.acquire(1, row("a"), LockMode::EXCLUSIVE, soon()), LockOutcome::ACQUIRED);
    ASSERT_EQ(locks.acquire(2, row("b"), LockMode::EXCLUSIVE, soon()), LockOutcome::ACQUIRED);
    ASSERT_EQ(locks.acquire(3, row("c"), LockMode::EXCLUSIVE, soon()), LockOutcome::ACQUIRED);
    auto first = std::async(std::launch::async,
        [&] { return locks.acquire(1, row("b"), LockMode::EXCLUSIVE, soon()); });
    waitUntilWaiting(1);
    auto second = std::async(std::launch::async,
        [&] { return locks.acquire(2, row("c"), LockMode::EXCLUSIVE, soon()); });
    waitUntilWaiting(2);
    EXPECT_EQ(locks.acquire(3, row("a"), LockMode::EXCLUSIVE, soon()), LockOutcome::DEADLOCK);
    locks.releaseAll(3);
    EXPECT_EQ(second.get(), LockOutcome::ACQUIRED);
    locks.releaseAll(2);
    EXPECT_EQ(first.get(), LockOutcome::ACQUIRED);
}

TEST_F(LockManagerTest, TwoUpgradesOfTheSameSharedLockDeadlock) {
    ASSERT_EQ(locks.acquire(1, row("a"), LockMode::SHARED, soon()), LockOutcome::ACQUIRED);
    ASSERT_EQ(locks.acquire(2, row("a"), LockMode::SHARED, soon()), LockOutcome::ACQUIRED);
    auto first = std::async(std::launch::async,
        [&] { return locks.acquire(1, row("a"), LockMode::EXCLUSIVE, soon()); });
    waitUntilWaiting(1);
    EXPECT_EQ(locks.acquire(2, row("a"), LockMode::EXCLUSIVE, soon()), LockOutcome::DEADLOCK);
    locks.releaseAll(2);
    EXPECT_EQ(first.get(), LockOutcome::ACQUIRED);
}

// Une montée passe devant la file : un écrivain qui attend ce détenteur ne peut pas le bloquer.
TEST_F(LockManagerTest, AnUpgradeDoesNotWaitBehindAQueuedWriter) {
    ASSERT_EQ(locks.acquire(1, row("a"), LockMode::SHARED, soon()), LockOutcome::ACQUIRED);
    auto writer = std::async(std::launch::async,
        [&] { return locks.acquire(2, row("a"), LockMode::EXCLUSIVE, soon()); });
    waitUntilWaiting(2);
    EXPECT_EQ(locks.acquire(1, row("a"), LockMode::EXCLUSIVE, clock_t_::now()),
        LockOutcome::ACQUIRED);
    locks.releaseAll(1);
    EXPECT_EQ(writer.get(), LockOutcome::ACQUIRED);
}

TEST_F(LockManagerTest, TheDeadlineGivesATimeoutAndLeavesNoGhostInTheQueue) {
    ASSERT_EQ(locks.acquire(1, row("a"), LockMode::EXCLUSIVE, soon()), LockOutcome::ACQUIRED);
    const auto before = clock_t_::now();
    EXPECT_EQ(locks.acquire(2, row("a"), LockMode::EXCLUSIVE, before + 100ms),
        LockOutcome::TIMEOUT);
    const auto waited = clock_t_::now() - before;
    EXPECT_GE(waited, 100ms);
    EXPECT_LT(waited, 2s);
    EXPECT_FALSE(locks.isWaiting(2));
    EXPECT_EQ(locks.getNumLocksHeld(2), 0u);
    // Celui qui a abandonné ne retient personne : le suivant passe dès la libération.
    locks.releaseAll(1);
    EXPECT_EQ(locks.acquire(3, row("a"), LockMode::EXCLUSIVE, clock_t_::now()),
        LockOutcome::ACQUIRED);
}

TEST_F(LockManagerTest, AnInterruptionEndsTheWait) {
    ASSERT_EQ(locks.acquire(1, row("a"), LockMode::EXCLUSIVE, soon()), LockOutcome::ACQUIRED);
    std::atomic<bool> interrupted{false};
    auto waiter = std::async(std::launch::async, [&] {
        return locks.acquire(2, row("a"), LockMode::EXCLUSIVE, soon(),
            [&] { return interrupted.load(); });
    });
    waitUntilWaiting(2);
    interrupted = true;
    EXPECT_EQ(waiter.get(), LockOutcome::INTERRUPTED);
    EXPECT_FALSE(locks.isWaiting(2));
}

// Les prises groupées se font dans un ordre unique : deux transactions qui annoncent les mêmes
// ressources dans l'ordre inverse ne s'interbloquent jamais, et passent l'une après l'autre.
TEST_F(LockManagerTest, GroupedAcquisitionsInOppositeOrderNeverDeadlock) {
    for (auto round = 0; round < 200; round++) {
        const std::vector<LockRequest> forward{{row("a"), LockMode::EXCLUSIVE},
            {row("b"), LockMode::EXCLUSIVE}, {LockResource::index(TABLE), LockMode::EXCLUSIVE}};
        const std::vector<LockRequest> backward{{LockResource::index(TABLE), LockMode::EXCLUSIVE},
            {row("b"), LockMode::EXCLUSIVE}, {row("a"), LockMode::EXCLUSIVE}};
        std::atomic<int> inside{0};
        std::atomic<bool> overlapped{false};
        const auto work = [&](rag3db::common::transaction_t transaction,
                              const std::vector<LockRequest>& requests) {
            const auto outcome = locks.acquire(transaction, requests, soon());
            if (outcome == LockOutcome::ACQUIRED) {
                overlapped = overlapped || inside.fetch_add(1) != 0;
                std::this_thread::yield();
                inside.fetch_sub(1);
            }
            locks.releaseAll(transaction);
            return outcome;
        };
        auto one = std::async(std::launch::async, work, 1, std::cref(forward));
        auto two = std::async(std::launch::async, work, 2, std::cref(backward));
        ASSERT_EQ(one.get(), LockOutcome::ACQUIRED) << "round " << round;
        ASSERT_EQ(two.get(), LockOutcome::ACQUIRED) << "round " << round;
        ASSERT_FALSE(overlapped) << "round " << round;
    }
    EXPECT_EQ(locks.getNumResources(), 0u);
}

TEST_F(LockManagerTest, AResourceAskedTwiceIsTakenOnceInItsStrongestMode) {
    const std::vector<LockRequest> requests{{row("a"), LockMode::SHARED},
        {row("a"), LockMode::EXCLUSIVE}, {row("a"), LockMode::SHARED}};
    ASSERT_EQ(locks.acquire(1, requests, soon()), LockOutcome::ACQUIRED);
    EXPECT_EQ(locks.getNumLocksHeld(1), 1u);
    EXPECT_EQ(locks.acquire(2, row("a"), LockMode::SHARED, clock_t_::now() + 30ms),
        LockOutcome::TIMEOUT);
}

// Les genres de ressource sont des espaces distincts, et une ligne ne tient pas l'index de sa
// table.
TEST_F(LockManagerTest, RowsAndIndexesAreDistinctResources) {
    ASSERT_EQ(locks.acquire(1, row(""), LockMode::EXCLUSIVE, soon()), LockOutcome::ACQUIRED);
    EXPECT_EQ(locks.acquire(2, LockResource::index(TABLE), LockMode::EXCLUSIVE, clock_t_::now()),
        LockOutcome::ACQUIRED);
    EXPECT_EQ(locks.acquire(3, LockResource::row(TABLE + 1, ""), LockMode::EXCLUSIVE,
                  clock_t_::now()),
        LockOutcome::ACQUIRED);
    EXPECT_EQ(locks.acquire(4, LockResource::index(TABLE + 1), LockMode::EXCLUSIVE,
                  clock_t_::now()),
        LockOutcome::ACQUIRED);
}

// Un cycle qui passe par une montée : T1 et T2 tiennent le même partagé ; T1 tient une ligne
// et demande la montée ; T2 demande cette ligne. La demande de T2 ferme le cycle.
TEST_F(LockManagerTest, ACycleThroughAnUpgradeIsDetected) {
    ASSERT_EQ(locks.acquire(1, row("hub"), LockMode::SHARED, soon()), LockOutcome::ACQUIRED);
    ASSERT_EQ(locks.acquire(2, row("hub"), LockMode::SHARED, soon()), LockOutcome::ACQUIRED);
    ASSERT_EQ(locks.acquire(1, row("a"), LockMode::EXCLUSIVE, soon()), LockOutcome::ACQUIRED);
    auto upgrading = std::async(std::launch::async,
        [&] { return locks.acquire(1, row("hub"), LockMode::EXCLUSIVE, soon()); });
    waitUntilWaiting(1);
    EXPECT_EQ(locks.acquire(2, row("a"), LockMode::EXCLUSIVE, soon()), LockOutcome::DEADLOCK);
    locks.releaseAll(2);
    EXPECT_EQ(upgrading.get(), LockOutcome::ACQUIRED);
}

TEST_F(LockManagerTest, TheMessagesCarryTheirStableFragments) {
    const auto resource = row("k42");
    EXPECT_NE(LockManager::describe(LockOutcome::DEADLOCK, resource)
                  .find(LockManager::DEADLOCK_DETECTED),
        std::string::npos);
    EXPECT_NE(
        LockManager::describe(LockOutcome::TIMEOUT, resource).find(LockManager::LOCK_TIMEOUT),
        std::string::npos);
    EXPECT_NE(LockManager::describe(LockOutcome::DEADLOCK, resource).find("k42"),
        std::string::npos);
    EXPECT_EQ(LockManager::describe(LockOutcome::INTERRUPTED, resource), "Interrupted.");
}

// Beaucoup de fils, peu de ressources : jamais deux détenteurs exclusifs, jamais un exclusif
// avec un partagé, et tout finit. À jouer aussi sous ThreadSanitizer. Rend le nombre
// d'interblocages détectés.
static uint64_t stress(LockManager& locks, bool ascendingOrder) {
    static constexpr int NUM_THREADS = 8;
    static constexpr int NUM_ROUNDS = 400;
    static constexpr int NUM_RESOURCES = 3;
    std::atomic<int> exclusiveHolders[NUM_RESOURCES] = {};
    std::atomic<int> sharedHolders[NUM_RESOURCES] = {};
    std::atomic<bool> broken{false};
    std::atomic<uint64_t> deadlocks{0};
    std::vector<std::thread> threads;
    for (auto thread = 0; thread < NUM_THREADS; thread++) {
        threads.emplace_back([&, thread] {
            for (auto round = 0; round < NUM_ROUNDS; round++) {
                const rag3db::common::transaction_t transaction =
                    static_cast<uint64_t>(thread) * NUM_ROUNDS + round + 1;
                // Deux prises l'une après l'autre. Dans un ordre qui dépend du fil, les
                // interblocages sont possibles et doivent être détectés ; dans l'ordre
                // croissant des ressources, il ne peut pas y en avoir.
                int first = (thread + round) % NUM_RESOURCES;
                int second = (first + 1 + (thread % 2)) % NUM_RESOURCES;
                auto firstMode = round % 3 == 0 ? LockMode::SHARED : LockMode::EXCLUSIVE;
                if (ascendingOrder && second < first) {
                    std::swap(first, second);
                    firstMode = LockMode::EXCLUSIVE;
                }
                std::vector<std::pair<int, LockMode>> held;
                for (const auto& [resource, mode] :
                    {std::pair{first, firstMode}, std::pair{second, LockMode::EXCLUSIVE}}) {
                    const auto outcome = locks.acquire(transaction,
                        LockResource::row(7, std::to_string(resource)), mode,
                        LockManager::clock::now() + 10s);
                    if (outcome == LockOutcome::DEADLOCK) {
                        deadlocks++;
                        break;
                    }
                    if (outcome != LockOutcome::ACQUIRED) {
                        broken = true;
                        break;
                    }
                    if (mode == LockMode::EXCLUSIVE) {
                        if (exclusiveHolders[resource].fetch_add(1) != 0 ||
                            sharedHolders[resource].load() != 0) {
                            broken = true;
                        }
                    } else {
                        sharedHolders[resource].fetch_add(1);
                        if (exclusiveHolders[resource].load() != 0) {
                            broken = true;
                        }
                    }
                    held.emplace_back(resource, mode);
                }
                for (const auto& [resource, mode] : held) {
                    (mode == LockMode::EXCLUSIVE ? exclusiveHolders : sharedHolders)[resource]
                        .fetch_sub(1);
                }
                locks.releaseAll(transaction);
            }
        });
    }
    for (auto& thread : threads) {
        thread.join();
    }
    EXPECT_FALSE(broken);
    EXPECT_EQ(locks.getNumResources(), 0u);
    return deadlocks.load();
}

TEST_F(LockManagerTest, ManyThreadsNeverHoldAnExclusiveLockTogether) {
    const auto deadlocks = stress(locks, false /* ascendingOrder */);
    std::cerr << "  deadlocks detected: " << deadlocks << "\n";
}

// Pas de faux interblocage : quand chaque transaction prend ses ressources dans l'ordre
// croissant, aucun cycle n'est possible, et aucun ne doit être annoncé.
TEST_F(LockManagerTest, ManyThreadsInOneOrderNeverGetADeadlock) {
    EXPECT_EQ(stress(locks, true /* ascendingOrder */), 0u);
}

} // namespace

#endif
