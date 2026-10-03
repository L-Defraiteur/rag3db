// Reproductions minimales des rouges imprévus de l'étape 2 (C4 et C6), écrites sans le
// harnais du banc — des connexions brutes, des std::thread ou un seul fil — pour
// qu'un défaut du banc ne puisse pas en être la cause. Chacune imprime sa trace brute.
//
// Ce qu'elles ont tranché le 3 octobre :
// - C6 est un défaut du moteur : dans un seul fil, sans concurrence, la suppression et
//   la mise à jour d'une même ligne valident toutes deux (rouge).
// - C4 était un défaut du banc : sans le harnais, aucune configuration ne dérive
//   (vert). Le Worker envoyait encore des instructions après un échec ; le moteur ayant
//   annulé la transaction, elles partaient en auto-commit. Ces tests restent comme
//   témoins : des virements concurrents gardent la somme.

#ifndef __SINGLE_THREADED__

#include <atomic>
#include <iostream>
#include <map>
#include <mutex>
#include <random>
#include <thread>

#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "processor/result/flat_tuple.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

// Une instruction exécutée par un écrivain, dans l'ordre global des fins
// d'instruction (sequence).
struct Event {
    uint64_t sequence;
    uint32_t writer;
    uint32_t transfer;
    std::string statement;
    std::string outcome;
};

struct TransferRecord {
    uint32_t writer;
    uint32_t transfer;
    int64_t from;
    int64_t to;
    int64_t value;
    bool debitApplied = false;
    bool creditApplied = false;
    bool committed = false;
};

class MinimalReproduction : public EmptyDBTest {
public:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        systemConfig->bufferPoolSize = 1024 * 1024 * 1024;
        createDBAndConn();
        mustRun("CALL debug_enable_multi_writes=true;");
        mustRun("CALL auto_checkpoint=false;");
    }

    void mustRun(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << "\n" << result->getErrorMessage();
    }

    int64_t queryInt(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    struct TransfersOutcome {
        int64_t expectedSum = 0;
        int64_t actualSum = 0;
        std::vector<TransferRecord> transfers;
        std::vector<Event> events;
        std::map<int64_t, int64_t> actualBalances;
    };

    // Des virements de deux mises à jour (débit puis crédit) par transaction, chacune
    // renvoyant le solde qu'elle vient d'écrire. Une transaction dont une instruction
    // échoue est annulée par ROLLBACK. Chaque montant est unique (numéro du virement),
    // pour qu'un écart se rattache à un virement.
    TransfersOutcome runTransfers(uint32_t numWriters, int64_t numAccounts,
        uint32_t transfersPerWriter, uint64_t seed) {
        mustRun("CREATE NODE TABLE Account(id INT64 PRIMARY KEY, balance INT64);");
        for (auto i = 0; i < numAccounts; ++i) {
            mustRun(stringFormat("CREATE (:Account {id: {}, balance: 1000000});", i));
        }
        TransfersOutcome outcome;
        std::atomic<uint64_t> sequence{0};
        std::mutex eventsMutex;
        std::vector<std::vector<TransferRecord>> perWriter(numWriters);
        std::vector<std::thread> threads;
        for (auto w = 0u; w < numWriters; ++w) {
            threads.emplace_back([&, w] {
                rag3db::main::Connection connection(database.get());
                std::mt19937_64 random(seed + w);
                const auto record = [&](uint32_t transfer, const std::string& statement,
                                        const std::string& result) {
                    const auto order = sequence.fetch_add(1);
                    std::lock_guard lock{eventsMutex};
                    outcome.events.push_back({order, w, transfer, statement, result});
                };
                const auto run = [&](uint32_t transfer, const std::string& statement,
                                     const std::string& query) {
                    auto result = connection.query(query);
                    std::string text;
                    if (!result->isSuccess()) {
                        text = "ERROR " + result->getErrorMessage();
                    } else if (result->hasNext()) {
                        text = "-> " + result->getNext()->getValue(0)->toString();
                    } else {
                        text = "ok";
                    }
                    record(transfer, statement, text);
                    return result->isSuccess();
                };
                for (auto t = 0u; t < transfersPerWriter; ++t) {
                    const auto id = w * 100000 + t;
                    const int64_t from = random() % numAccounts;
                    const int64_t to = (from + 1 + random() % (numAccounts - 1)) % numAccounts;
                    // Montant unique, impair, pour qu'une somme d'écarts se décompose.
                    const int64_t value = 2 * id + 1;
                    TransferRecord transfer{w, t, from, to, value};
                    run(t, "BEGIN", "BEGIN TRANSACTION;");
                    transfer.debitApplied = run(t, stringFormat("debit {} by {}", from, value),
                        stringFormat("MATCH (a:Account {id: {}}) SET a.balance = a.balance - {} "
                                     "RETURN a.balance;",
                            from, value));
                    if (transfer.debitApplied) {
                        transfer.creditApplied = run(t, stringFormat("credit {} by {}", to, value),
                            stringFormat("MATCH (a:Account {id: {}}) SET a.balance = "
                                         "a.balance + {} RETURN a.balance;",
                                to, value));
                    }
                    if (transfer.debitApplied && transfer.creditApplied) {
                        transfer.committed = run(t, "COMMIT", "COMMIT;");
                    } else {
                        run(t, "ROLLBACK", "ROLLBACK;");
                    }
                    perWriter[w].push_back(transfer);
                }
            });
        }
        for (auto& thread : threads) {
            thread.join();
        }
        for (auto& transfers : perWriter) {
            outcome.transfers.insert(outcome.transfers.end(), transfers.begin(), transfers.end());
        }
        std::sort(outcome.events.begin(), outcome.events.end(),
            [](const Event& a, const Event& b) { return a.sequence < b.sequence; });
        outcome.expectedSum = numAccounts * 1000000;
        outcome.actualSum = queryInt("MATCH (a:Account) RETURN sum(a.balance);");
        auto balances = conn->query("MATCH (a:Account) RETURN a.id, a.balance;");
        while (balances->hasNext()) {
            const auto row = balances->getNext();
            outcome.actualBalances[row->getValue(0)->getValue<int64_t>()] =
                row->getValue(1)->getValue<int64_t>();
        }
        return outcome;
    }

    // Écart par compte : solde lu moins solde attendu d'après les seuls virements
    // validés ; puis les virements non validés dont le débit a été appliqué et dont le
    // montant explique l'écart du compte débité.
    static std::string explain(const TransfersOutcome& outcome, int64_t numAccounts) {
        std::map<int64_t, int64_t> expected;
        for (auto i = 0; i < numAccounts; ++i) {
            expected[i] = 1000000;
        }
        uint32_t committed = 0;
        uint32_t failedAfterDebit = 0;
        for (const auto& t : outcome.transfers) {
            if (t.committed) {
                expected[t.from] -= t.value;
                expected[t.to] += t.value;
                ++committed;
            } else if (t.debitApplied) {
                ++failedAfterDebit;
            }
        }
        std::string out = stringFormat("sum expected {} actual {} (drift {}); {} transfers, {} "
                                       "committed, {} failed after their debit\n",
            outcome.expectedSum, outcome.actualSum, outcome.actualSum - outcome.expectedSum,
            outcome.transfers.size(), committed, failedAfterDebit);
        for (const auto& [account, balance] : outcome.actualBalances) {
            const auto drift = balance - expected[account];
            out += stringFormat("  account {}: expected {} actual {} drift {}\n", account,
                expected[account], balance, drift);
            if (drift == 0) {
                continue;
            }
            // Les débits non validés de ce compte, dont la somme d'un sous-ensemble
            // devrait refaire l'écart s'ils ont survécu à l'annulation.
            int64_t sumOfLostDebits = 0;
            for (const auto& t : outcome.transfers) {
                if (!t.committed && t.debitApplied && t.from == account) {
                    sumOfLostDebits += t.value;
                }
            }
            out += stringFormat("    debits of rolled-back transfers from this account: "
                                "total {}\n",
                sumOfLostDebits);
        }
        return out;
    }

    static std::string eventsOf(const TransfersOutcome& outcome, size_t limit) {
        std::string out;
        for (auto i = 0u; i < outcome.events.size() && i < limit; ++i) {
            const auto& e = outcome.events[i];
            out += stringFormat("  #{} w{} t{} {} {}\n", e.sequence, e.writer, e.transfer,
                e.statement, e.outcome);
        }
        return out;
    }
};

// C4 tracé : quatre écrivains, huit comptes, comme dans le banc, mais sans harnais.
// Invariant : la somme ne bouge pas.
TEST_F(MinimalReproduction, C4_TransfersTraced) {
    constexpr int64_t numAccounts = 8;
    const auto outcome = runTransfers(4, numAccounts, 100, 20261002);
    std::cerr << explain(outcome, numAccounts);
    EXPECT_EQ(outcome.actualSum, outcome.expectedSum);
}

// Les petites formes : chaque combinaison d'écrivains et de comptes est un test.
TEST_F(MinimalReproduction, C4_TwoWritersTwoAccounts) {
    const auto outcome = runTransfers(2, 2, 300, 20261002);
    std::cerr << explain(outcome, 2);
    if (outcome.actualSum != outcome.expectedSum) {
        std::cerr << eventsOf(outcome, 400);
    }
    EXPECT_EQ(outcome.actualSum, outcome.expectedSum);
}

TEST_F(MinimalReproduction, C4_ThreeWritersTwoAccounts) {
    const auto outcome = runTransfers(3, 2, 300, 20261002);
    std::cerr << explain(outcome, 2);
    EXPECT_EQ(outcome.actualSum, outcome.expectedSum);
}

TEST_F(MinimalReproduction, C4_TwoWritersThreeAccounts) {
    const auto outcome = runTransfers(2, 3, 300, 20261002);
    std::cerr << explain(outcome, 3);
    EXPECT_EQ(outcome.actualSum, outcome.expectedSum);
}

// C6 dans un seul fil, deux connexions, sans aucune concurrence : la suppression et la
// mise à jour de la même ligne, chacune dans sa transaction ouverte avant que l'autre
// ne valide. Invariant : la seconde à valider échoue.
static void deleteAndUpdateSingleThread(MinimalReproduction& test, rag3db::main::Database& database,
    bool deleteCommitsFirst) {
    test.mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64);");
    test.mustRun("CREATE (:Item {id: 1, v: 0});");
    rag3db::main::Connection deleter(&database);
    rag3db::main::Connection updater(&database);
    const auto show = [](const char* what, rag3db::main::QueryResult& result) {
        std::cerr << "  " << what << ": "
                  << (result.isSuccess() ? "ok" : "ERROR " + result.getErrorMessage()) << "\n";
        return result.isSuccess();
    };
    show("deleter BEGIN", *deleter.query("BEGIN TRANSACTION;"));
    show("updater BEGIN", *updater.query("BEGIN TRANSACTION;"));
    show("deleter DELETE", *deleter.query("MATCH (n:Item {id: 1}) DELETE n;"));
    show("updater SET", *updater.query("MATCH (n:Item {id: 1}) SET n.v = 1;"));
    bool first = false;
    bool second = false;
    if (deleteCommitsFirst) {
        first = show("deleter COMMIT", *deleter.query("COMMIT;"));
        second = show("updater COMMIT", *updater.query("COMMIT;"));
    } else {
        first = show("updater COMMIT", *updater.query("COMMIT;"));
        second = show("deleter COMMIT", *deleter.query("COMMIT;"));
    }
    auto rows = test.conn->query("MATCH (n:Item) RETURN n.id, n.v;");
    std::cerr << "  rows after: " << rows->toString();
    EXPECT_FALSE(first && second) << "the delete and the update both committed";
}

TEST_F(MinimalReproduction, C6_DeleteCommitsFirstSingleThread) {
    deleteAndUpdateSingleThread(*this, *database, true);
}

TEST_F(MinimalReproduction, C6_UpdateCommitsFirstSingleThread) {
    deleteAndUpdateSingleThread(*this, *database, false);
}

} // namespace

#endif
