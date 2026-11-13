// Un lecteur qui cherche par clé primaire pendant qu'un écrivain insère et que des points de
// reprise passent (ticket
// docs/coeur-cpp/tickets/2026-11-13-sous-vingt-ecrivains-un-lecteur-meurt-dans-l-
// index-de-cle-primaire-sur-disque.md). Dans TwentyWriters.DdlEveryMinute, un
// PrimaryKeyScanNodeTable est mort dans DiskArrayInternal::get en lisant pips[0] d'un vecteur de
// PIP vide : le tableau disque de l'index de hachage était cru non vide alors que ses PIP n'étaient
// pas encore dans pips.
//
// La table part vide, donc sans PIP. Un fil lit en boucle MATCH (d:Doc {id: k}) dans des
// transactions READ ONLY neuves ; l'écrivain insère des lots de clés neuves, valide, et lance un
// point de reprise tous les quelques lots. Une variante annule un lot sur deux. Chaque cas tourne
// dans un processus fils : un SIGSEGV y est un rouge nommé (pk-lookup-does-not-crash), pas la fin
// de la passe.
#ifndef __SINGLE_THREADED__

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <atomic>
#include <cstdio>
#include <string>
#include <thread>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class PkLookupUnderCheckpoint : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed: the subject is the index on disk";
        }
    }
    // Joue le cas dans un fils ; rend son statut. Le fils sort 0 si tout s'est bien passé, 3 si une
    // lecture a rendu un résultat faux, 4 si une instruction de l'écrivain a échoué.
    int play(bool rollBackEveryOther) {
        const auto pid = fork();
        if (pid == 0) {
            int status = 0;
            {
                Database db(databasePath, *systemConfig);
                Connection writer(&db);
                Connection reader(&db);
                writer.query("CALL auto_checkpoint=false;");
                if (!writer.query("CREATE NODE TABLE Doc(id INT64, name STRING, PRIMARY KEY(id));")
                        ->isSuccess()) {
                    _exit(4);
                }
                std::atomic<bool> stop{false};
                std::atomic<int64_t> committed{0};
                std::atomic<int> wrong{0};
                std::thread readerThread([&] {
                    int64_t k = 0;
                    while (!stop.load()) {
                        // Relevé avant l'instantané : toute clé en deçà y est validée.
                        const auto upTo = committed.load();
                        auto begun = reader.query("BEGIN TRANSACTION READ ONLY;");
                        if (!begun->isSuccess()) {
                            fprintf(stderr, "[témoin] BEGIN du lecteur refusé : %s\n",
                                begun->getErrorMessage().c_str());
                        }
                        const auto key = upTo > 0 ? (k++ % upTo) : 0;
                        auto r = reader.query(
                            "MATCH (d:Doc {id: " + std::to_string(key) + "}) RETURN count(d);");
                        if (!r->isSuccess()) {
                            fprintf(stderr, "[témoin] lecture refusée : %s\n",
                                r->getErrorMessage().c_str());
                        } else if (upTo > 0) {
                            const auto n = r->getNext()->getValue(0)->getValue<int64_t>();
                            if (n != 1 && wrong++ < 5) {
                                fprintf(stderr,
                                    "[témoin] clé %ld manquée : compte %ld, %ld clés validées\n",
                                    static_cast<long>(key), static_cast<long>(n),
                                    static_cast<long>(upTo));
                            }
                        }
                        reader.query("COMMIT;");
                    }
                });
                int64_t next = 0;
                for (auto batch = 0; batch < 400 && status == 0; batch++) {
                    const bool rollBack = rollBackEveryOther && batch % 2 == 1;
                    writer.query("BEGIN TRANSACTION;");
                    const auto from = rollBack ? 10'000'000 + batch * 100 : next;
                    if (!writer
                            .query("UNWIND range(" + std::to_string(from) + ", " +
                                   std::to_string(from + 99) +
                                   ") AS i CREATE (:Doc {id: i, name: 'n' + CAST(i AS STRING)});")
                            ->isSuccess()) {
                        status = 4;
                    }
                    if (rollBack) {
                        writer.query("ROLLBACK;");
                    } else {
                        auto done = writer.query("COMMIT;");
                        if (!done->isSuccess()) {
                            fprintf(stderr, "[témoin] COMMIT refusé : %s\n",
                                done->getErrorMessage().c_str());
                            status = 4;
                        }
                        next += 100;
                        committed.store(next);
                    }
                    if (batch % 5 == 4) {
                        writer.query("CHECKPOINT;");
                    }
                }
                stop.store(true);
                readerThread.join();
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
    static void expectClean(int status) {
        ASSERT_FALSE(WIFSIGNALED(status))
            << "[check: pk-lookup-does-not-crash] the child died of signal " << WTERMSIG(status);
        EXPECT_EQ(WEXITSTATUS(status), 0) << "[check: pk-lookup-reads-right] 3: a lookup missed a "
                                             "committed key, 4: a write failed";
    }
};

TEST_F(PkLookupUnderCheckpoint, LookupsDuringInsertsAndCheckpointsDoNotCrash) {
    expectClean(play(false));
}

TEST_F(PkLookupUnderCheckpoint, LookupsDuringRolledBackInsertsAndCheckpointsDoNotCrash) {
    expectClean(play(true));
}

} // namespace

#endif
