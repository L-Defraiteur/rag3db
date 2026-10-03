// L'arrêt brutal, un seul écrivain (mode multi-écrivains éteint, celui qui est en
// service). Demandé par l'orchestration le 3 octobre au soir : les correctifs de reprise
// livrés ces jours-ci n'étaient éprouvés que par un échec injecté ou un journal tronqué à
// la main ; aucun test ne tuait le processus. Ici, chaque cas fait travailler un processus
// fils, base ouverte, puis le tue par SIGKILL — lui-même à un instant précis, ou le père
// après un délai — et le père rouvre la base et vérifie son contenu.
//
// Règles (voir lock_bench_test.cpp pour le harnais) :
// - le SIGKILL part DANS la portée de la Database : sortir du bloc la fermerait proprement,
//   avec un point de reprise final, et rien ne serait rejoué (la faute corrigée le
//   3 octobre au soir) ;
// - quand le cas doit laisser un journal, le père exige un .wal non vide avant de rouvrir
//   (journal-to-replay) ;
// - pour tuer au milieu d'un point de reprise, un Checkpointer de test se tue avant ou
//   après une de ses phases, installé par FlakyCheckpointer (test/transaction), sans
//   toucher src/.
// Attendu : tout vert. Un rouge ici est un défaut du mode en service.

#ifndef __SINGLE_THREADED__

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <filesystem>
#include <fstream>
#include <functional>
#include <iostream>
#include <random>
#include <thread>

#include "../flaky_checkpointer.h"
#include "bench_harness.h"
#include "common/exception/runtime.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "integrity/integrity_checker.h"
#include "processor/result/flat_tuple.h"
#include "storage/checkpointer.h"
#include "storage/database_header.h"
#include "storage/storage_utils.h"

using namespace rag3db::common;
using namespace rag3db::testing;
using namespace rag3db::testing::concurrency;

namespace {

// Les instants où un point de reprise peut mourir, dans l'ordre de writeCheckpoint.
enum class DeathPoint {
    BeforeStorage,
    AfterStorage,
    AfterSerialize,
    AfterHeader,
    AfterApplyingShadowPages,
};

std::string deathPointName(DeathPoint point) {
    switch (point) {
    case DeathPoint::BeforeStorage:
        return "BeforeStorage";
    case DeathPoint::AfterStorage:
        return "AfterStorage";
    case DeathPoint::AfterSerialize:
        return "AfterSerialize";
    case DeathPoint::AfterHeader:
        return "AfterHeader";
    case DeathPoint::AfterApplyingShadowPages:
        return "AfterApplyingShadowPages";
    }
    return "?";
}

[[noreturn]] void dieNow() {
    kill(getpid(), SIGKILL);
    _exit(9);
}

// Un point de reprise qui meurt, pour de bon, à l'instant donné.
class DyingCheckpointer final : public rag3db::storage::Checkpointer {
public:
    DyingCheckpointer(rag3db::main::ClientContext& context, DeathPoint point)
        : Checkpointer(context), point{point} {}

protected:
    bool checkpointStorage() override {
        if (point == DeathPoint::BeforeStorage) {
            dieNow();
        }
        const auto changed = Checkpointer::checkpointStorage();
        if (point == DeathPoint::AfterStorage) {
            dieNow();
        }
        return changed;
    }
    void serializeCatalogAndMetadata(rag3db::storage::DatabaseHeader& header,
        bool hasStorageChanges) override {
        Checkpointer::serializeCatalogAndMetadata(header, hasStorageChanges);
        if (point == DeathPoint::AfterSerialize) {
            dieNow();
        }
    }
    void writeDatabaseHeader(const rag3db::storage::DatabaseHeader& header) override {
        Checkpointer::writeDatabaseHeader(header);
        if (point == DeathPoint::AfterHeader) {
            dieNow();
        }
    }
    void logCheckpointAndApplyShadowPages() override {
        Checkpointer::logCheckpointAndApplyShadowPages();
        if (point == DeathPoint::AfterApplyingShadowPages) {
            dieNow();
        }
    }

private:
    DeathPoint point;
};

// Un point de reprise qui échoue dans sa phase de stockage (comme FlakyCheckpointer de
// checkpoint_test.cpp) : la base doit ensuite tout refuser jusqu'à sa réouverture
// (7072183db).
class FailingCheckpointer final : public rag3db::storage::Checkpointer {
public:
    explicit FailingCheckpointer(rag3db::main::ClientContext& context) : Checkpointer(context) {}

protected:
    bool checkpointStorage() override {
        throw rag3db::common::RuntimeException("checkpoint failed on purpose");
    }
};

class SingleWriterCrash : public EmptyDBTest {
public:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
    }

    void mustRun(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << "\n" << result->getErrorMessage();
    }

    int64_t queryInt(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << "[check: query] " << query << "\n"
                                         << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    std::string queryText(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << "[check: query] " << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->toString() :
                   "";
    }

    // Repart d'une base vide : la base et tous ses fichiers voisins (journal, fichier
    // fantôme, journaux mis de côté), pas les fichiers de données du cas (.csv).
    void wipeDatabase() {
        conn.reset();
        database.reset();
        const auto path = std::filesystem::path(databasePath);
        const auto stem = path.filename().string();
        for (const auto& entry : std::filesystem::directory_iterator(path.parent_path())) {
            const auto name = entry.path().filename().string();
            if (name.starts_with(stem) && entry.path().extension() != ".csv") {
                std::filesystem::remove_all(entry.path());
            }
        }
    }

    uint64_t walSize() const {
        const auto path = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
        return std::filesystem::exists(path) ? std::filesystem::file_size(path) : 0;
    }

    // Fait travailler un fils base ouverte. work reçoit la connexion du fils ; s'il rend
    // la main, le fils se tue aussitôt, base encore ouverte. Si killAfter est donné, le
    // père tue le fils après ce délai, où qu'il en soit. Le père attend la mort par
    // SIGKILL ; il ne rouvre pas.
    void runChild(
        const std::function<void(rag3db::main::Database&, rag3db::main::Connection&)>& work,
        std::optional<std::chrono::milliseconds> killAfter = std::nullopt) {
        conn.reset();
        database.reset();
        const auto pid = fork();
        if (pid == 0) {
            disableCoreDumps();
            try {
                rag3db::main::Database childDatabase(databasePath, *systemConfig);
                rag3db::main::Connection childConnection(&childDatabase);
                work(childDatabase, childConnection);
                dieNow();
            } catch (const std::exception& e) {
                std::cerr << "  child failed: " << e.what() << "\n";
                _exit(3);
            }
        }
        if (killAfter) {
            std::this_thread::sleep_for(*killAfter);
            kill(pid, SIGKILL);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        ASSERT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
            << "[check: killed] the child did not die by SIGKILL: "
            << (WIFEXITED(status) ? "exit code " + std::to_string(WEXITSTATUS(status)) :
                                    "signal " + std::to_string(WTERMSIG(status)));
    }

    void expectJournalToReplay() {
        const auto size = walSize();
        std::cerr << "  journal to replay: " << size << " bytes\n";
        EXPECT_GT(size, 0u) << "[check: journal-to-replay] nothing to replay";
    }

    // Rouvre ; une base qui ne se rouvre plus est un rouge nommé. D'abord dans un
    // processus neuf (probeOpenInFreshProcess) : c'est ainsi qu'un service redémarre, et un
    // plantage à l'ouverture ne doit pas emporter la passe.
    bool reopen(bool vectorExtension = false) {
        const auto probe = probeOpenInFreshProcess(databasePath, vectorExtension);
        if (!probe.empty()) {
            ADD_FAILURE() << "[check: database-opens-without-crash] " << probe;
            return false;
        }
        try {
            createDBAndConn();
            if (vectorExtension) {
                loadVectorExtension(*conn);
            }
            return true;
        } catch (const std::exception& e) {
            ADD_FAILURE() << "[check: database-reopens] " << e.what();
            return false;
        }
    }

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

    // Après la reprise, la base doit encore accepter une écriture, un point de reprise et
    // une seconde réouverture.
    void expectStillWritable(const std::string& insert, bool vectorExtension = false) {
        auto result = conn->query(insert);
        EXPECT_TRUE(result->isSuccess())
            << "[check: writable-after-recovery] " << result->getErrorMessage();
        auto checkpoint = conn->query("CHECKPOINT;");
        EXPECT_TRUE(checkpoint->isSuccess())
            << "[check: checkpoint-after-recovery] " << checkpoint->getErrorMessage();
        checkpoint.reset();
        result.reset();
        if (reopen(vectorExtension)) {
            expectIntegrity();
        }
    }
};

void mustQuery(rag3db::main::Connection& connection, const std::string& query) {
    auto result = connection.query(query);
    if (!result->isSuccess()) {
        throw std::runtime_error(query + ": " + result->getErrorMessage());
    }
}

} // namespace

// 1. Des enregistrements de plus de 4096 octets dans le journal (955b1b136) : une chaîne
// de 20 000 caractères et une liste de 3 000 entiers dans un même commit ; mort avant
// tout point de reprise ; au rejeu, tout est relu à l'identique.
TEST_F(SingleWriterCrash, LargeJournalRecordsAreReplayedIntact) {
    const std::string longText(20'000, 'x');
    runChild([&](rag3db::main::Database&, rag3db::main::Connection& connection) {
        mustQuery(connection, "CALL auto_checkpoint=false;");
        mustQuery(connection, "CALL force_checkpoint_on_close=false;");
        mustQuery(connection,
            "CREATE NODE TABLE Big(id INT64 PRIMARY KEY, text STRING, numbers INT64[]);");
        mustQuery(connection,
            stringFormat("CREATE (:Big {id: 1, text: '{}', numbers: range(1, 3000)});", longText));
        mustQuery(connection, "UNWIND range(2, 400) AS i CREATE (:Big {id: i, text: 'y' + "
                              "CAST(i AS STRING), numbers: range(1, i)});");
    });
    expectJournalToReplay();
    if (!reopen()) {
        return;
    }
    EXPECT_EQ(queryInt("MATCH (b:Big {id: 1}) RETURN size(b.text);"), 20'000)
        << "[check: long-text] ";
    EXPECT_EQ(queryInt("MATCH (b:Big {id: 1}) RETURN size(b.numbers);"), 3'000)
        << "[check: long-list] ";
    EXPECT_EQ(queryInt("MATCH (b:Big {id: 1}) RETURN list_sum(b.numbers);"), 3000 * 3001 / 2)
        << "[check: long-list] ";
    EXPECT_EQ(queryInt("MATCH (b:Big) RETURN count(b);"), 400) << "[check: row-count] ";
    expectIntegrity();
    expectStillWritable("CREATE (:Big {id: 5000, text: 'z', numbers: [1]});");
}

// 2. La mort au milieu d'une écriture du journal (a66bb0b9d) : le fils valide des lots
// de 50 nœuds sans fin, chacun dans sa transaction, et compte ses commits acquittés en
// mémoire partagée ; le père le tue après un délai variable. Au rejeu, une fin déchirée
// est tolérée et mise de côté ; tout commit acquitté est là, aucun lot n'est coupé.
TEST_F(SingleWriterCrash, TornJournalEndKeepsEveryAcknowledgedCommit) {
    std::mt19937 random(20261003);
    for (auto round = 0; round < 8; ++round) {
        if (round > 0) {
            wipeDatabase();
        }
        SharedMapping mapping(1);
        auto& area = mapping.get();
        const auto delay = std::chrono::milliseconds(20 + random() % 180);
        runChild(
            [&](rag3db::main::Database&, rag3db::main::Connection& connection) {
                mustQuery(connection, "CALL auto_checkpoint=false;");
                mustQuery(connection,
                    "CREATE NODE TABLE Lot(id INT64 PRIMARY KEY, batch INT64, payload STRING);");
                for (int64_t batch = 0;; ++batch) {
                    mustQuery(connection, "BEGIN TRANSACTION;");
                    mustQuery(connection,
                        stringFormat("UNWIND range({}, {}) AS i CREATE (:Lot {id: i, batch: {}, "
                                     "payload: 'payload-' + CAST(i AS STRING) + "
                                     "'-0123456789012345678901234567890123456789'});",
                            batch * 50, batch * 50 + 49, batch));
                    mustQuery(connection, "COMMIT;");
                    area.workers[0].commitsSucceeded.fetch_add(1);
                }
            },
            delay);
        const auto acknowledged = area.workers[0].commitsSucceeded.load();
        std::cerr << "  round " << round << ": killed after " << delay.count() << " ms, "
                  << acknowledged << " batches acknowledged, journal " << walSize() << " bytes\n";
        if (!reopen()) {
            return;
        }
        if (acknowledged == 0) {
            continue;
        }
        const auto rows = queryInt("MATCH (n:Lot) RETURN count(n);");
        EXPECT_GE(rows, static_cast<int64_t>(acknowledged) * 50)
            << "[check: acknowledged-commits-kept] round " << round;
        EXPECT_EQ(rows % 50, 0) << "[check: no-torn-batch] round " << round;
        EXPECT_EQ(queryInt("MATCH (n:Lot) WITH n.batch AS b, count(*) AS c WHERE c <> 50 "
                           "RETURN count(b);"),
            0)
            << "[check: no-torn-batch] round " << round;
        expectIntegrity();
    }
}

// 3. La mort pendant un point de reprise, à cinq instants (le défaut DiskArray et
// l'index de clé primaire, 6bf46150b ; la panne pendant la phase de stockage,
// 093702c7f). Un premier point de reprise normal, puis des insertions, des
// suppressions et des mises à jour, puis un point de reprise qui meurt. Au rejeu : tout
// ce qui a été validé est là, l'index de clé primaire retrouve chaque ligne, la base
// accepte encore une écriture et un point de reprise.
class CheckpointDeath : public SingleWriterCrash,
                        public ::testing::WithParamInterface<DeathPoint> {};

TEST_P(CheckpointDeath, CommittedDataSurvivesADeathDuringCheckpoint) {
    const auto point = GetParam();
    runChild([&](rag3db::main::Database&, rag3db::main::Connection& connection) {
        mustQuery(connection, "CALL auto_checkpoint=false;");
        mustQuery(connection, "CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64, s STRING);");
        mustQuery(connection, "CREATE REL TABLE Link(FROM Item TO Item, src_id INT64, "
                              "dst_id INT64);");
        mustQuery(connection, "UNWIND range(0, 1999) AS i CREATE (:Item {id: i, v: i, s: "
                              "'first-' + CAST(i AS STRING)});");
        mustQuery(connection, "MATCH (a:Item), (b:Item) WHERE b.id = a.id + 1 AND a.id < 500 "
                              "CREATE (a)-[:Link {src_id: a.id, dst_id: b.id}]->(b);");
        mustQuery(connection, "CHECKPOINT;");
        mustQuery(connection, "UNWIND range(2000, 3999) AS i CREATE (:Item {id: i, v: i, s: "
                              "'second-' + CAST(i AS STRING)});");
        mustQuery(connection, "MATCH (n:Item) WHERE n.id % 10 = 3 DETACH DELETE n;");
        mustQuery(connection, "MATCH (n:Item) WHERE n.id % 10 = 7 SET n.v = n.v + 100000;");
        FlakyCheckpointer dying([point](rag3db::main::ClientContext& context) {
            return std::make_unique<DyingCheckpointer>(context, point);
        });
        dying.setCheckpointer(*connection.getClientContext());
        connection.query("CHECKPOINT;");
    });
    // Après l'application des pages fantômes, le point de reprise est consigné et le
    // journal peut déjà être vide : légitime à cet instant-là seulement.
    if (point != DeathPoint::AfterApplyingShadowPages) {
        expectJournalToReplay();
    }
    if (!reopen()) {
        return;
    }
    EXPECT_EQ(queryInt("MATCH (n:Item) RETURN count(n);"), 3600) << "[check: row-count] ";
    EXPECT_EQ(queryInt("MATCH (n:Item) WHERE n.id % 10 = 3 RETURN count(n);"), 0)
        << "[check: deletes-kept] ";
    EXPECT_EQ(queryInt("MATCH (n:Item) WHERE n.id % 10 = 7 AND n.v = n.id + 100000 RETURN "
                       "count(n);"),
        400)
        << "[check: updates-kept] ";
    EXPECT_EQ(queryInt("MATCH (n:Item {id: 3998}) RETURN n.v;"), 3998)
        << "[check: primary-key-lookup] ";
    EXPECT_EQ(queryText("MATCH (n:Item {id: 2468}) RETURN n.s;"), "second-2468")
        << "[check: primary-key-lookup] ";
    expectIntegrity();
    expectStillWritable("UNWIND range(4000, 4999) AS i CREATE (:Item {id: i, v: i, s: 'third'});");
}

INSTANTIATE_TEST_SUITE_P(Points, CheckpointDeath,
    ::testing::Values(DeathPoint::BeforeStorage, DeathPoint::AfterStorage,
        DeathPoint::AfterSerialize, DeathPoint::AfterHeader, DeathPoint::AfterApplyingShadowPages),
    [](const ::testing::TestParamInfo<DeathPoint>& info) { return deathPointName(info.param); });

// 4. Le point de reprise échoué (7072183db) : il échoue dans sa phase de stockage ; la
// base doit alors tout refuser ; puis le processus meurt. À la réouverture, tout ce qui
// était validé est là et la base est de nouveau utilisable.
TEST_F(SingleWriterCrash, FailedCheckpointThenDeathKeepsCommittedData) {
    SharedMapping mapping(1);
    auto& area = mapping.get();
    runChild([&](rag3db::main::Database&, rag3db::main::Connection& connection) {
        mustQuery(connection, "CALL auto_checkpoint=false;");
        mustQuery(connection, "CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64);");
        mustQuery(connection, "UNWIND range(0, 2999) AS i CREATE (:Item {id: i, v: i});");
        FlakyCheckpointer failing([](rag3db::main::ClientContext& context) {
            return std::make_unique<FailingCheckpointer>(context);
        });
        failing.setCheckpointer(*connection.getClientContext());
        if (connection.query("CHECKPOINT;")->isSuccess()) {
            _exit(5);
        }
        // Après l'échec : une écriture doit être refusée.
        if (!connection.query("CREATE (:Item {id: 9999, v: 0});")->isSuccess()) {
            area.flag.store(1);
        }
    });
    EXPECT_EQ(area.flag.load(), 1u)
        << "[check: refuses-after-failed-checkpoint] the database must refuse writes after a "
           "failed checkpoint";
    expectJournalToReplay();
    if (!reopen()) {
        return;
    }
    EXPECT_EQ(queryInt("MATCH (n:Item) RETURN count(n);"), 3000) << "[check: row-count] ";
    EXPECT_EQ(queryInt("MATCH (n:Item {id: 9999}) RETURN count(n);"), 0)
        << "[check: refused-write-absent] ";
    expectIntegrity();
    expectStillWritable("CREATE (:Item {id: 5000, v: 1});");
}

// 5. L'index vectoriel dans le seul journal. Le DROP est le cas
// IndexReopen.DropAfterCheckpointThenCrashLeavesAnUnloadedIndex (lock_bench_test.cpp) ;
// voici son symétrique : un index CRÉÉ après le dernier point de reprise, puis la mort.
// Au rejeu, l'index existe, il est utilisable et rend exactement les lignes vivantes.
TEST_F(SingleWriterCrash, VectorIndexCreatedOnlyInTheJournalIsUsableAfterRecovery) {
    runChild([&](rag3db::main::Database&, rag3db::main::Connection& connection) {
        loadVectorExtension(connection);
        mustQuery(connection, "CALL auto_checkpoint=false;");
        mustQuery(connection, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
        mustQuery(connection, "UNWIND range(0, 499) AS i CREATE (:Doc {id: i, vec: [CAST(i % 17 "
                              "AS FLOAT), CAST(i % 23 AS FLOAT), CAST(i % 29 AS FLOAT), CAST(i "
                              "AS FLOAT)]});");
        mustQuery(connection, "CHECKPOINT;");
        mustQuery(connection, "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := "
                              "'l2', skip_if_exists := true);");
        mustQuery(connection, "UNWIND range(500, 599) AS i CREATE (:Doc {id: i, vec: [1.0, 2.0, "
                              "3.0, CAST(i AS FLOAT)]});");
    });
    expectJournalToReplay();
    if (!reopen(true /* vectorExtension */)) {
        return;
    }
    EXPECT_EQ(queryInt("CALL SHOW_INDEXES() RETURN count(*);"), 1) << "[check: index-exists] ";
    expectIntegrity();
    expectStillWritable("CREATE (:Doc {id: 9000, vec: [9.0, 9.0, 9.0, 9.0]});", true);
}

// 6. La mort pendant un chargement en masse (COPY) de nœuds, puis de relations — le
// chemin par défaut du premier index côté rag3weaver. Le père tue le fils pendant le
// COPY, à un délai variable. À la réouverture : la table est vide ou complète (le COPY
// est atomique), l'intégrité est saine, la base accepte encore un COPY.
TEST_F(SingleWriterCrash, DeathDuringCopyLeavesAllOrNothing) {
    const auto nodesCsv = databasePath + ".nodes.csv";
    const auto relsCsv = databasePath + ".rels.csv";
    constexpr int64_t numNodes = 200'000;
    {
        std::ofstream nodes(nodesCsv);
        for (int64_t i = 0; i < numNodes; ++i) {
            nodes << i << ",name-" << i << "\n";
        }
        std::ofstream rels(relsCsv);
        for (int64_t i = 0; i + 1 < numNodes; ++i) {
            rels << i << "," << (i + 1) << "\n";
        }
    }
    std::mt19937 random(20261004);
    for (auto round = 0; round < 6; ++round) {
        wipeDatabase();
        const bool relsPhase = round % 2 == 1;
        // Assez tôt pour tomber au milieu du COPY des nœuds (il dure moins de 200 ms),
        // assez tard pour tomber dans celui des relations.
        const auto delay =
            std::chrono::milliseconds(relsPhase ? 150 + random() % 300 : 5 + random() % 150);
        runChild(
            [&](rag3db::main::Database&, rag3db::main::Connection& connection) {
                mustQuery(connection, "CREATE NODE TABLE Item(id INT64 PRIMARY KEY, name STRING);");
                mustQuery(connection, "CREATE REL TABLE Link(FROM Item TO Item);");
                if (relsPhase) {
                    mustQuery(connection, stringFormat("COPY Item FROM '{}';", nodesCsv));
                    mustQuery(connection, stringFormat("COPY Link FROM '{}';", relsCsv));
                } else {
                    mustQuery(connection, stringFormat("COPY Item FROM '{}';", nodesCsv));
                }
                // Le COPY a fini avant le délai : on attend la mort.
                std::this_thread::sleep_for(std::chrono::seconds(30));
            },
            delay);
        if (!reopen()) {
            return;
        }
        const auto tables = queryInt("CALL show_tables() WHERE name = 'Item' RETURN count(*);");
        int64_t nodes = 0;
        int64_t rels = 0;
        if (tables == 1) {
            nodes = queryInt("MATCH (n:Item) RETURN count(n);");
            rels = queryInt("MATCH ()-[r:Link]->() RETURN count(r);");
        }
        std::cerr << "  round " << round << (relsPhase ? " (rels)" : " (nodes)") << ": killed "
                  << "after " << delay.count() << " ms; nodes " << nodes << ", rels " << rels
                  << "\n";
        EXPECT_TRUE(nodes == 0 || nodes == numNodes)
            << "[check: copy-all-or-nothing] nodes " << nodes;
        EXPECT_TRUE(rels == 0 || rels == numNodes - 1)
            << "[check: copy-all-or-nothing] rels " << rels;
        if (tables == 1) {
            expectIntegrity();
        }
    }
    std::filesystem::remove(nodesCsv);
    std::filesystem::remove(relsCsv);
}

// 7. La mort pendant des suppressions suivies de recherches vectorielles (A5, A5 bis) :
// le fils supprime des documents par lots validés, en interrogeant l'index entre deux
// lots ; le père le tue après un délai variable. À la réouverture, l'index rend
// exactement les lignes vivantes.
TEST_F(SingleWriterCrash, DeathDuringDeletesAndVectorSearchKeepsTheIndexExact) {
    std::mt19937 random(20261005);
    for (auto round = 0; round < 6; ++round) {
        wipeDatabase();
        // La préparation (2 000 documents, l'index, le point de reprise) dure plusieurs
        // centaines de millisecondes : tuer après, pendant les suppressions.
        const auto delay = std::chrono::milliseconds(400 + random() % 1200);
        runChild(
            [&](rag3db::main::Database&, rag3db::main::Connection& connection) {
                loadVectorExtension(connection);
                mustQuery(connection, "CALL auto_checkpoint=false;");
                mustQuery(connection, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
                mustQuery(connection, "UNWIND range(0, 1999) AS i CREATE (:Doc {id: i, vec: "
                                      "[CAST(i % 17 AS FLOAT), CAST(i % 23 AS FLOAT), CAST(i % 29 "
                                      "AS FLOAT), CAST(i AS FLOAT)]});");
                mustQuery(connection, "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', "
                                      "metric := 'l2');");
                mustQuery(connection, "CHECKPOINT;");
                for (int64_t batch = 0; batch < 200; ++batch) {
                    mustQuery(connection,
                        stringFormat("MATCH (n:Doc) WHERE n.id >= {} AND n.id < {} DELETE n;",
                            batch * 9, batch * 9 + 9));
                    mustQuery(connection, "CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [1.0, "
                                          "2.0, 3.0, 4.0], 10) RETURN node.id;");
                }
                std::this_thread::sleep_for(std::chrono::seconds(30));
            },
            delay);
        if (!reopen(true /* vectorExtension */)) {
            return;
        }
        if (queryInt("CALL show_tables() WHERE name = 'Doc' RETURN count(*);") != 1) {
            std::cerr << "  round " << round << ": killed before the table existed\n";
            continue;
        }
        std::cerr << "  round " << round << ": killed after " << delay.count() << " ms; "
                  << queryInt("MATCH (n:Doc) RETURN count(n);") << " documents left\n";
        expectIntegrity();
    }
}

// 7 bis. La même mort, l'index créé dans une session PRÉCÉDENTE (la base fermée puis
// rouverte avant les suppressions) : la forme du 7 qui éprouve A5 et A5 bis sans passer
// par le plantage à l'ouverture d'un index créé dans la session morte (cas 7 et 5).
TEST_F(SingleWriterCrash, DeathDuringDeletesOfAnIndexFromAnEarlierSessionKeepsTheIndexExact) {
    std::mt19937 random(20261006);
    for (auto round = 0; round < 6; ++round) {
        wipeDatabase();
        {
            rag3db::main::Database earlier(databasePath, *systemConfig);
            rag3db::main::Connection connection(&earlier);
            loadVectorExtension(connection);
            mustQuery(connection, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
            mustQuery(connection, "UNWIND range(0, 1999) AS i CREATE (:Doc {id: i, vec: [CAST(i "
                                  "% 17 AS FLOAT), CAST(i % 23 AS FLOAT), CAST(i % 29 AS FLOAT), "
                                  "CAST(i AS FLOAT)]});");
            mustQuery(connection,
                "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
        }
        const auto delay = std::chrono::milliseconds(20 + random() % 300);
        runChild(
            [&](rag3db::main::Database&, rag3db::main::Connection& connection) {
                loadVectorExtension(connection);
                mustQuery(connection, "CALL auto_checkpoint=false;");
                for (int64_t batch = 0; batch < 200; ++batch) {
                    mustQuery(connection,
                        stringFormat("MATCH (n:Doc) WHERE n.id >= {} AND n.id < {} DELETE n;",
                            batch * 9, batch * 9 + 9));
                    mustQuery(connection, "CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [1.0, "
                                          "2.0, 3.0, 4.0], 10) RETURN node.id;");
                }
                std::this_thread::sleep_for(std::chrono::seconds(30));
            },
            delay);
        if (!reopen(true /* vectorExtension */)) {
            return;
        }
        std::cerr << "  round " << round << ": killed after " << delay.count() << " ms; "
                  << queryInt("MATCH (n:Doc) RETURN count(n);") << " documents left\n";
        expectIntegrity();
    }
}

// Le processus neuf d'une sonde d'ouverture (probeOpenInFreshProcess) : lancé par exec du
// banc lui-même, il ouvre la base dont le chemin est dans CONCURRENCE_OPEN_PROBE, charge
// l'extension vector si on le lui demande, fait une requête, et sort. Sauté dans une passe
// ordinaire.
TEST(OpenProbe, OpenFromEnvironment) {
    const char* path = std::getenv("CONCURRENCE_OPEN_PROBE");
    if (path == nullptr) {
        GTEST_SKIP() << "only run as the fresh process of an open probe";
    }
    const char* vector = std::getenv("CONCURRENCE_OPEN_PROBE_VECTOR");
    try {
        rag3db::main::Database database(path, *TestHelper::getSystemConfigFromEnv());
        rag3db::main::Connection connection(&database);
        if (vector != nullptr && std::string(vector) == "1") {
            loadVectorExtension(connection);
        }
        auto result = connection.query("CALL show_tables() RETURN count(*);");
        _exit(result->isSuccess() ? 0 : 2);
    } catch (...) {
        _exit(3);
    }
}

#endif
