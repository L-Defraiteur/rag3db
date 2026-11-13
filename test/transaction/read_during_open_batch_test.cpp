// Une lecture pendant un lot d'écriture voit le lot entier ou rien (ticket de recherche du
// 13 novembre 2026, docs/recherche/tickets/2026-11-13-une-lecture-pendant-un-lot-reindexe-voit-le-
// lot-a-moitie-sous-la-lib-entiere.md). Dans rag3weaver, sous charge, une transaction READ ONLY a vu
// l'arête MENTIONS d'un lot pas encore validé, sans son arête DEFINES.
//
// La forme est celle que la capture du test Rust a relevée (LD_PRELOAD sur Connection::query et
// executeWithParams, 13 novembre). L'écrivain, dans une seule transaction, crée par MERGE :
// - un Scope de définition et un Scope d'appel ;
// - un Symbol ;
// - l'arête DEFINES et l'arête MENTIONS vers ce Symbol ;
// puis il valide.
// Des lecteurs, chacun sur sa Connection, posent dans une même transaction READ ONLY les trois
// questions de `usages` :
// - les définitions du Symbol (Symbol {uuid} <-DEFINES- Scope) ;
// - ses mentions (Symbol {uuid} <-MENTIONS- Scope) ;
// - le Scope d'appel par un balayage sur son nom.
// Un même instantané doit répondre tout ou rien. Le cas tourne dans un processus fils.
#ifndef __SINGLE_THREADED__

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <atomic>
#include <cstdio>
#include <memory>
#include <string>
#include <thread>
#include <vector>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

constexpr int NUM_READERS = 6;
constexpr int NUM_BATCHES = 400;

class ReadDuringOpenBatch : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
    }
    static int64_t count(Connection& c, const std::string& q, bool& ok) {
        auto r = c.query(q);
        if (!r->isSuccess() || !r->hasNext()) {
            fprintf(stderr, "[témoin] lecture refusée : %s\n", r->getErrorMessage().c_str());
            ok = false;
            return -1;
        }
        return r->getNext()->getValue(0)->getValue<int64_t>();
    }
    static bool run(Connection& c, const std::string& q) {
        auto r = c.query(q);
        if (!r->isSuccess()) {
            fprintf(stderr, "[témoin] écriture refusée : %s : %s\n", q.c_str(),
                r->getErrorMessage().c_str());
        }
        return r->isSuccess();
    }
    // Le fils sort 0 si chaque lecture a vu tout ou rien, 3 si une lecture a vu le lot à moitié,
    // 4 si une instruction de l'écrivain a échoué.
    int play(bool checkpointFirst) {
        const auto pid = fork();
        if (pid == 0) {
            int status = 0;
            {
                Database db(databasePath, *systemConfig);
                Connection writer(&db);
                writer.query("CALL auto_checkpoint=false;");
                for (const auto* q : {"CREATE NODE TABLE Scope(uuid STRING, name STRING, "
                                      "declarations STRING, PRIMARY KEY(uuid));",
                         "CREATE NODE TABLE Symbol(uuid STRING, name STRING, PRIMARY KEY(uuid));",
                         "CREATE REL TABLE DEFINES(FROM Scope TO Symbol);",
                         "CREATE REL TABLE MENTIONS(FROM Scope TO Symbol, line INT64);",
                         // Des lignes validées avant les lots : des groupes déjà là, que les lots
                         // prolongent.
                         "UNWIND range(0, 299) AS i CREATE (:Scope {uuid: 'base-s' + CAST(i AS "
                         "STRING), name: 'base' + CAST(i AS STRING), declarations: ''});",
                         "UNWIND range(0, 299) AS i CREATE (:Symbol {uuid: 'base-y' + CAST(i AS "
                         "STRING), name: 'base' + CAST(i AS STRING)});",
                         "UNWIND range(0, 299) AS i MATCH (a:Scope {uuid: 'base-s' + CAST(i AS "
                         "STRING)}), (b:Symbol {uuid: 'base-y' + CAST(i AS STRING)}) CREATE "
                         "(a)-[:DEFINES]->(b), (a)-[:MENTIONS {line: 1}]->(b);"}) {
                    if (!run(writer, q)) {
                        _exit(4);
                    }
                }
                if (checkpointFirst && !run(writer, "CHECKPOINT;")) {
                    _exit(4);
                }
                // Le lot k est en cours (ouvert ou en validation) quand inFlight vaut k ; il est
                // validé quand committed dépasse k.
                std::atomic<int> inFlight{-1};
                std::atomic<int> committed{0};
                std::atomic<bool> stop{false};
                std::atomic<int> wrong{0};
                std::atomic<int64_t> reads{0};
                std::vector<std::thread> readers;
                for (auto t = 0; t < NUM_READERS; t++) {
                    readers.emplace_back([&] {
                        Connection reader(&db);
                        while (!stop.load()) {
                            const auto upTo = committed.load();
                            const auto k = inFlight.load();
                            if (k < 0) {
                                continue;
                            }
                            const auto ks = std::to_string(k);
                            bool ok = reader.query("BEGIN TRANSACTION READ ONLY;")->isSuccess();
                            const auto defs = count(reader,
                                "MATCH (d:Symbol {uuid: 'y" + ks +
                                    "'})<-[:DEFINES]-(m:Scope) RETURN count(m);",
                                ok);
                            const auto mentions = count(reader,
                                "MATCH (d:Symbol {uuid: 'y" + ks +
                                    "'})<-[r:MENTIONS]-(m:Scope) RETURN count(r);",
                                ok);
                            const auto callers = count(reader,
                                "MATCH (m:Scope) WHERE m.name = 'h" + ks + "' RETURN count(m);", ok);
                            reader.query("COMMIT;");
                            reads++;
                            if (!ok) {
                                continue;
                            }
                            const bool whole = defs == mentions && mentions == callers &&
                                               (defs == 0 || defs == 1);
                            const bool lost = k < upTo && defs != 1;
                            if ((!whole || lost) && wrong++ < 10) {
                                fprintf(stderr,
                                    "[témoin] lot %d vu à moitié : DEFINES %ld, MENTIONS %ld, "
                                    "Scope d'appel %ld (lots validés : %d)\n",
                                    k, static_cast<long>(defs), static_cast<long>(mentions),
                                    static_cast<long>(callers), upTo);
                            }
                        }
                    });
                }
                for (auto k = 0; k < NUM_BATCHES && status == 0; k++) {
                    const auto ks = std::to_string(k);
                    inFlight.store(k);
                    const std::vector<std::string> batch = {"BEGIN TRANSACTION;",
                        "UNWIND ['d" + ks + "', 'c" + ks +
                            "'] AS u MERGE (n:Scope {uuid: u}) SET n.name = CASE WHEN u = 'd" +
                            ks + "' THEN 'def" + ks + "' ELSE 'h" + ks +
                            "' END, n.declarations = '\"arrivee" + ks + "\"';",
                        "UNWIND ['y" + ks + "'] AS u MERGE (n:Symbol {uuid: u}) SET n.name = "
                                            "'arrivee" +
                            ks + "';",
                        "UNWIND [{f: 'd" + ks + "', t: 'y" + ks +
                            "'}] AS item MATCH (a:Scope {uuid: item.f}), (b:Symbol {uuid: "
                            "item.t}) MERGE (a)-[:DEFINES]->(b);",
                        "UNWIND [{f: 'c" + ks + "', t: 'y" + ks +
                            "'}] AS item MATCH (a:Scope {uuid: item.f}), (b:Symbol {uuid: "
                            "item.t}) MERGE (a)-[r:MENTIONS]->(b) SET r.line = 1;"};
                    for (const auto& q : batch) {
                        if (!run(writer, q)) {
                            status = 4;
                        }
                    }
                    // Le lot reste ouvert un instant, comme la pause du lot de rag3weaver.
                    std::this_thread::sleep_for(std::chrono::milliseconds(2));
                    if (!run(writer, "COMMIT;")) {
                        status = 4;
                    }
                    committed.store(k + 1);
                    if (k % 50 == 49 && !run(writer, "CHECKPOINT;")) {
                        status = 4;
                    }
                }
                stop.store(true);
                for (auto& r : readers) {
                    r.join();
                }
                fprintf(stderr, "[témoin] %ld lectures, %d vues à moitié\n",
                    static_cast<long>(reads.load()), wrong.load());
                if (status == 0 && wrong.load() > 0) {
                    status = 3;
                }
            }
            _exit(status);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        return status;
    }
    static void expectWhole(int status) {
        ASSERT_FALSE(WIFSIGNALED(status))
            << "[check: batch-read-does-not-crash] the child died of signal " << WTERMSIG(status);
        EXPECT_EQ(WEXITSTATUS(status), 0)
            << "[check: batch-read-sees-all-or-nothing] 3: a read saw half a batch, 4: a write "
               "failed";
    }
};

TEST_F(ReadDuringOpenBatch, AReadSeesTheBatchWholeOrNotAtAll) {
    expectWhole(play(false));
}

TEST_F(ReadDuringOpenBatch, AReadSeesTheBatchWholeOrNotAtAllOverCheckpointedRows) {
    expectWhole(play(true));
}

} // namespace

#endif
