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

#include <cmath>
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
#include "storage/shadow_file.h"
#include "storage/storage_manager.h"
#include "storage/storage_utils.h"
#include "storage/wal/wal.h"
#include "storage/wal/wal_replayer.h"

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
    // La marque CHECKPOINT est au journal, les pages fantômes ne sont pas encore recopiées :
    // la reprise devra les rejouer depuis le fichier fantôme.
    AfterCheckpointLogged,
    // Pages recopiées et synchronisées, journal vidé (tronqué, pas supprimé), fichier
    // fantôme encore plein : la reprise trouve un journal vide à côté d'un fichier fantôme.
    AfterJournalCleared,
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
    case DeathPoint::AfterCheckpointLogged:
        return "AfterCheckpointLogged";
    case DeathPoint::AfterJournalCleared:
        return "AfterJournalCleared";
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
        if (point == DeathPoint::AfterCheckpointLogged) {
            // Les deux premiers pas de Checkpointer::logCheckpointAndApplyShadowPages, puis la
            // mort avant la recopie des pages.
            rag3db::storage::StorageManager::Get(clientContext)
                ->getShadowFile()
                .flushAll(clientContext);
            rag3db::storage::WAL::Get(clientContext)->logAndFlushCheckpoint(&clientContext);
            dieNow();
        }
        if (point == DeathPoint::AfterJournalCleared) {
            // Les pas de Checkpointer::logCheckpointAndApplyShadowPages jusqu'au vidage du
            // journal, puis la mort avant celui du fichier fantôme.
            auto& shadowFile = rag3db::storage::StorageManager::Get(clientContext)->getShadowFile();
            shadowFile.flushAll(clientContext);
            auto wal = rag3db::storage::WAL::Get(clientContext);
            wal->logAndFlushCheckpoint(&clientContext);
            shadowFile.applyShadowPages(clientContext);
            wal->clear();
            dieNow();
        }
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
    if (point == DeathPoint::AfterJournalCleared) {
        const auto shadowPath = rag3db::storage::StorageUtils::getShadowFilePath(databasePath);
        EXPECT_TRUE(std::filesystem::exists(
                        rag3db::storage::StorageUtils::getWALFilePath(databasePath)) &&
                    walSize() == 0)
            << "[check: journal-cleared] the journal is not there and empty";
        EXPECT_TRUE(std::filesystem::exists(shadowPath) && std::filesystem::file_size(shadowPath) > 0)
            << "[check: shadow-file-left] no shadow file next to the empty journal";
    } else if (point != DeathPoint::AfterApplyingShadowPages) {
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
        DeathPoint::AfterSerialize, DeathPoint::AfterHeader, DeathPoint::AfterCheckpointLogged,
        DeathPoint::AfterJournalCleared, DeathPoint::AfterApplyingShadowPages),
    [](const ::testing::TestParamInfo<DeathPoint>& info) { return deathPointName(info.param); });

// La reprise d'un journal terminé par un CHECKPOINT rejoue les pages du fichier fantôme, puis
// supprime le journal et le fichier fantôme. Une mort entre les deux suppressions ne doit pas
// empêcher la base de s'ouvrir (ticket « durabilité sur faute d'entrée-sortie, coupure ou mort
// au mauvais instant », le cas que la stèle exige). Dans l'ancien ordre (le fichier fantôme
// d'abord), elle laissait un journal terminé par un CHECKPOINT sans son fichier fantôme, et
// l'ouverture échouait. Les points d'arrêt sont ceux de WALReplayer::RecoveryPoint.
class RecoveryDeath : public SingleWriterCrash {
public:
    // L'état de départ : la mort d'un point de reprise juste après sa marque au journal.
    void dieAfterTheCheckpointIsLogged() {
        runChild([&](rag3db::main::Database&, rag3db::main::Connection& connection) {
            mustQuery(connection, "CALL auto_checkpoint=false;");
            mustQuery(connection, "CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64);");
            mustQuery(connection, "UNWIND range(0, 1999) AS i CREATE (:Item {id: i, v: i});");
            mustQuery(connection, "CHECKPOINT;");
            mustQuery(connection, "UNWIND range(2000, 3999) AS i CREATE (:Item {id: i, v: i});");
            mustQuery(connection, "MATCH (n:Item) WHERE n.id % 10 = 7 SET n.v = n.v + 100000;");
            FlakyCheckpointer dying([](rag3db::main::ClientContext& context) {
                return std::make_unique<DyingCheckpointer>(context,
                    DeathPoint::AfterCheckpointLogged);
            });
            dying.setCheckpointer(*connection.getClientContext());
            connection.query("CHECKPOINT;");
        });
        expectJournalToReplay();
    }

    // Une reprise dans un fils, tuée au point donné.
    void dieDuringRecoveryAt(rag3db::storage::RecoveryPoint point) {
        conn.reset();
        database.reset();
        const auto pid = fork();
        if (pid == 0) {
            disableCoreDumps();
            rag3db::storage::WALReplayer::setRecoveryHookForTesting(
                [point](rag3db::storage::RecoveryPoint reached) {
                    if (reached == point) {
                        dieNow();
                    }
                });
            try {
                rag3db::main::Database childDatabase(databasePath, *systemConfig);
            } catch (const std::exception& e) {
                std::cerr << "  recovery failed: " << e.what() << "\n";
                _exit(3);
            }
            _exit(4);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        ASSERT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
            << "[check: killed-during-recovery] the recovery did not reach the point";
    }

    void expectTheData() {
        EXPECT_EQ(queryInt("MATCH (n:Item) RETURN count(n);"), 4000) << "[check: row-count] ";
        EXPECT_EQ(queryInt("MATCH (n:Item) WHERE n.id % 10 = 7 AND n.v = n.id + 100000 RETURN "
                           "count(n);"),
            400)
            << "[check: updates-kept] ";
        EXPECT_EQ(queryInt("MATCH (n:Item {id: 3998}) RETURN n.v;"), 3998)
            << "[check: primary-key-lookup] ";
        expectIntegrity();
    }
};

TEST_F(RecoveryDeath, DeathBetweenTheTwoRemovalsAtRecovery) {
    dieAfterTheCheckpointIsLogged();
    dieDuringRecoveryAt(rag3db::storage::RecoveryPoint::JOURNAL_REMOVED);
    if (HasFatalFailure() || !reopen()) {
        return;
    }
    expectTheData();
}

// L'état qu'une mort dans l'ancien ordre laissait : les pages recopiées, le fichier fantôme
// supprimé, le journal encore là. La reprise doit savoir qu'il n'y a plus rien à recopier.
TEST_F(RecoveryDeath, ShadowFileAlreadyReplayedAndRemoved) {
    dieAfterTheCheckpointIsLogged();
    dieDuringRecoveryAt(rag3db::storage::RecoveryPoint::SHADOW_PAGES_REPLAYED);
    if (HasFatalFailure()) {
        return;
    }
    std::filesystem::remove(rag3db::storage::StorageUtils::getShadowFilePath(databasePath));
    if (!reopen()) {
        return;
    }
    expectTheData();
}

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

// ── La reprise avec un index d'extension (marche à part, pas les verrous) ─────────────
// Une écriture validée ne doit pas rendre d'erreur (ticket « un COPY rend une erreur alors
// qu'il est validé », et « un CHECKPOINT retient un lecteur », 4 octobre). Le point de
// reprise lancé après la validation (automatique au seuil, ou forcé par un COPY) attend que
// les autres transactions partent ; s'il expire, son erreur remonte comme le résultat de
// l'instruction, alors que l'écriture est déjà visible. L'appelant croit à un échec et
// rejoue : il écrit deux fois. L'attendu : une erreur veut dire « rien n'est validé ».
static std::string writeCsv(const std::string& path, int from, int to) {
    std::ofstream out(path);
    for (auto i = from; i < to; ++i) {
        out << i << "\n";
    }
    return path;
}

TEST_F(SingleWriterCrash, AutoCheckpointTimeoutOnACommittedWrite) {
    mustRun("CALL auto_checkpoint=false;");
    mustRun("CREATE NODE TABLE P(id INT64 PRIMARY KEY);");
    mustRun("UNWIND range(1, 20000) AS i CREATE (:P {id: i});");
    mustRun("CHECKPOINT;");
    mustRun("CALL checkpoint_threshold=1;");
    mustRun("CALL auto_checkpoint=true;");
    // Un lecteur long sur une autre connexion, pendant que l'écrivain valide.
    rag3db::main::Connection reader(database.get());
    std::thread readerThread(
        [&] { reader.query("MATCH (a:P), (b:P) WHERE a.id + b.id = 7 RETURN count(*);"); });
    std::this_thread::sleep_for(std::chrono::milliseconds(200));
    auto write = conn->query("CREATE (:P {id: 999999});");
    readerThread.join();
    const auto present = queryInt("MATCH (p:P) WHERE p.id + 0 = 999999 RETURN count(*);");
    std::cerr << "  write: " << (write->isSuccess() ? "ok" : write->getErrorMessage())
              << " ; row present: " << present << "\n";
    EXPECT_TRUE(write->isSuccess() || present == 0)
        << "[check: error-means-not-committed] the write returned an error but is committed";
}

TEST_F(SingleWriterCrash, CopyWhileATransactionIsOpen) {
    mustRun("CREATE NODE TABLE C(id INT64 PRIMARY KEY);");
    rag3db::main::Connection other(database.get());
    other.query("BEGIN TRANSACTION READ ONLY;");
    other.query("MATCH (c:C) RETURN count(*);");
    auto copy = conn->query(
        "COPY C FROM '" + writeCsv(databasePath + ".open.csv", 0, 1000) + "' (header=false);");
    other.query("COMMIT;");
    const auto present = queryInt("MATCH (c:C) RETURN count(*);");
    std::cerr << "  copy: " << (copy->isSuccess() ? "ok" : copy->getErrorMessage())
              << " ; rows present: " << present << "\n";
    EXPECT_TRUE(copy->isSuccess() || present == 0)
        << "[check: error-means-not-committed] the COPY returned an error but is committed";
}

// La forme grave : un COPY qui a rendu une erreur, puis la mort du processus. Le COPY n'est pas
// au journal (sa durabilité est son propre point de reprise) : ce que d'autres ont vu de lui
// avant la mort doit survivre. Avant le 4 octobre au soir, il rendait l'erreur avec ses lignes
// visibles, et elles disparaissaient ; il est désormais annulé avant d'être visible.
TEST_F(SingleWriterCrash, CopyThatReturnedAnErrorThenDeath) {
    mustRun("CREATE NODE TABLE C(id INT64 PRIMARY KEY);");
    mustRun("CHECKPOINT;");
    const auto csv = writeCsv(databasePath + ".death.csv", 0, 1000);
    const auto seenFile = databasePath + ".seen";
    runChild([&](rag3db::main::Database& childDatabase, rag3db::main::Connection& connection) {
        rag3db::main::Connection other(&childDatabase);
        other.query("BEGIN TRANSACTION READ ONLY;");
        other.query("MATCH (c:C) RETURN count(*);");
        auto copy = connection.query("COPY C FROM '" + csv + "' (header=false);");
        other.query("COMMIT;");
        auto seen = connection.query("MATCH (c:C) RETURN count(*);");
        const auto count = seen->getNext()->getValue(0)->getValue<int64_t>();
        std::ofstream(seenFile) << count;
        std::cerr << "  child: copy " << (copy->isSuccess() ? "ok" : "error") << ", rows seen "
                  << count << "\n";
    });
    int64_t seen = -1;
    std::ifstream(seenFile) >> seen;
    if (!reopen()) {
        return;
    }
    const auto after = queryInt("MATCH (c:C) RETURN count(*);");
    std::cerr << "  after death: " << after << " rows\n";
    EXPECT_TRUE(after == 0 || after == 1000) << "[check: copy-all-or-nothing] " << after;
    EXPECT_EQ(after, seen) << "[check: visible-copy-survives-death] " << seen
                           << " rows seen before the death, " << after << " after";
}

// Après un COPY annulé parce qu'une transaction restait ouverte : aucune ligne, aucune clé
// fantôme, et le même COPY passe une fois la transaction fermée. La cardinalité gonflée par
// l'annulation est un défaut connu, rangé en confort (ticket
// 2026-10-04-copy-refuse-gonfle-la-cardinalite.md) : ce cas ne la regarde pas.
TEST_F(SingleWriterCrash, CopyCancelledByAnOpenTransactionLeavesNothing) {
    mustRun("CREATE NODE TABLE C(id INT64 PRIMARY KEY);");
    const auto csv = writeCsv(databasePath + ".cancelled.csv", 0, 1000);
    {
        rag3db::main::Connection other(database.get());
        other.query("BEGIN TRANSACTION READ ONLY;");
        other.query("MATCH (c:C) RETURN count(*);");
        auto copy = conn->query("COPY C FROM '" + csv + "' (header=false);");
        EXPECT_FALSE(copy->isSuccess()) << "[check: copy-cancelled] the COPY should wait and fail";
        other.query("COMMIT;");
    }
    EXPECT_EQ(queryInt("MATCH (c:C) RETURN count(*);"), 0) << "[check: nothing-visible] ";
    EXPECT_EQ(queryInt("MATCH (c:C {id: 5}) RETURN count(*);"), 0) << "[check: no-ghost-key] ";
    auto again = conn->query("COPY C FROM '" + csv + "' (header=false);");
    EXPECT_TRUE(again->isSuccess()) << "[check: copy-again-passes] " << again->getErrorMessage();
    again.reset();
    EXPECT_EQ(queryInt("MATCH (c:C) RETURN count(*);"), 1000) << "[check: copy-again-passes] ";
    EXPECT_EQ(queryInt("MATCH (c:C {id: 5}) RETURN count(*);"), 1) << "[check: key-found] ";
    // Puis un point de reprise, une réouverture, et la relecture de toutes les tables : les
    // pages rendues par l'annulation ne doivent rien avoir abîmé (session cœur C++).
    mustRun("CHECKPOINT;");
    if (!reopen()) {
        return;
    }
    EXPECT_EQ(queryInt("MATCH (c:C) WHERE c.id + 0 >= 0 RETURN count(*);"), 1000)
        << "[check: rows-after-reopen] ";
    expectIntegrity();
}

// Un COPY réussi, puis la mort : ses lignes sont là.
TEST_F(SingleWriterCrash, DeathAfterASuccessfulCopy) {
    mustRun("CREATE NODE TABLE C(id INT64 PRIMARY KEY);");
    mustRun("CHECKPOINT;");
    const auto csv = writeCsv(databasePath + ".ok.csv", 0, 1000);
    runChild([&](rag3db::main::Database&, rag3db::main::Connection& connection) {
        auto copy = connection.query("COPY C FROM '" + csv + "' (header=false);");
        std::cerr << "  child: copy " << (copy->isSuccess() ? "ok" : copy->getErrorMessage())
                  << "\n";
    });
    if (!reopen()) {
        return;
    }
    EXPECT_EQ(queryInt("MATCH (c:C) RETURN count(*);"), 1000) << "[check: copy-survives-death] ";
}

// Le point de reprise automatique, quand une transaction reste ouverte au-delà du délai : il
// est reporté, l'écriture validée ne rend pas d'erreur, et il se fait à une validation
// suivante, une fois la transaction fermée (comme PostgreSQL, un point de reprise ne fait
// jamais échouer une validation).
TEST_F(SingleWriterCrash, AutoCheckpointPostponedWhileATransactionStaysOpen) {
    mustRun("CALL auto_checkpoint=false;");
    mustRun("CREATE NODE TABLE P(id INT64 PRIMARY KEY);");
    mustRun("UNWIND range(1, 1000) AS i CREATE (:P {id: i});");
    mustRun("CHECKPOINT;");
    mustRun("CALL checkpoint_threshold=1;");
    mustRun("CALL auto_checkpoint=true;");
    rag3db::main::Connection other(database.get());
    other.query("BEGIN TRANSACTION READ ONLY;");
    other.query("MATCH (p:P) RETURN count(*);");
    auto write = conn->query("CREATE (:P {id: 999999});");
    EXPECT_TRUE(write->isSuccess()) << "[check: committed-write-succeeds] "
                                    << write->getErrorMessage();
    write.reset();
    other.query("COMMIT;");
    const auto walBefore = walSize();
    auto next = conn->query("CREATE (:P {id: 1000000});");
    EXPECT_TRUE(next->isSuccess()) << "[check: next-write-succeeds] " << next->getErrorMessage();
    next.reset();
    EXPECT_LT(walSize(), walBefore) << "[check: postponed-checkpoint-done] the journal should "
                                       "have been folded by the next commit";
    EXPECT_EQ(queryInt("MATCH (p:P) WHERE p.id + 0 >= 999999 RETURN count(*);"), 2)
        << "[check: rows-present] ";
}

// Une rafale d'écritures sous un lecteur resté ouvert, le journal au-delà du seuil : chaque
// validation reporte son point de reprise sans l'attendre. Dix écritures ne prennent pas dix
// délais (relecture de la session cœur C++, 4 octobre).
TEST_F(SingleWriterCrash, BurstOfWritesUnderAnOpenReaderDoesNotWait) {
    mustRun("CALL auto_checkpoint=false;");
    mustRun("CREATE NODE TABLE P(id INT64 PRIMARY KEY);");
    mustRun("CHECKPOINT;");
    mustRun("CALL checkpoint_threshold=1;");
    mustRun("CALL auto_checkpoint=true;");
    rag3db::main::Connection other(database.get());
    other.query("BEGIN TRANSACTION READ ONLY;");
    other.query("MATCH (p:P) RETURN count(*);");
    const auto start = std::chrono::steady_clock::now();
    for (auto i = 0; i < 10; ++i) {
        auto write = conn->query(stringFormat("CREATE (:P {id: {}});", i));
        EXPECT_TRUE(write->isSuccess()) << "[check: committed-write-succeeds] "
                                        << write->getErrorMessage();
    }
    const auto elapsed = std::chrono::steady_clock::now() - start;
    other.query("COMMIT;");
    const auto ms = std::chrono::duration_cast<std::chrono::milliseconds>(elapsed).count();
    std::cerr << "  ten writes under an open reader: " << ms << " ms\n";
    EXPECT_LT(ms, 2000) << "[check: writes-do-not-wait] each write waited for the checkpoint";
    EXPECT_EQ(queryInt("MATCH (p:P) RETURN count(*);"), 10) << "[check: rows-present] ";
}

// La forme explicite, celle de la transaction par paquet de rag3weaver : BEGIN ; COPY ;
// COMMIT, avec une autre transaction ouverte. Le COMMIT rend l'erreur de délai (un COMMIT qui
// échoue ferme le bloc), et rien ne reste : ni ligne, ni clé, puis point de reprise,
// réouverture, intégrité.
TEST_F(SingleWriterCrash, ExplicitCopyCommitWhileATransactionIsOpen) {
    mustRun("CREATE NODE TABLE C(id INT64 PRIMARY KEY);");
    const auto csv = writeCsv(databasePath + ".explicit.csv", 0, 1000);
    rag3db::main::Connection other(database.get());
    other.query("BEGIN TRANSACTION READ ONLY;");
    other.query("MATCH (c:C) RETURN count(*);");
    mustRun("BEGIN TRANSACTION;");
    auto copy = conn->query("COPY C FROM '" + csv + "' (header=false);");
    if (!copy->isSuccess()) {
        // Le COPY n'est pas permis dans un bloc explicite : rien à éprouver par cette forme.
        std::cerr << "  COPY in a block: " << copy->getErrorMessage() << "\n";
        conn->query("ROLLBACK;");
        other.query("COMMIT;");
        GTEST_SKIP() << "COPY is not allowed in an explicit transaction";
    }
    auto commit = conn->query("COMMIT;");
    std::cerr << "  COMMIT: " << (commit->isSuccess() ? "ok" : commit->getErrorMessage())
              << "\n";
    EXPECT_FALSE(commit->isSuccess()) << "[check: commit-waits-and-fails] ";
    conn->query("ROLLBACK;");
    other.query("COMMIT;");
    EXPECT_EQ(queryInt("MATCH (c:C) RETURN count(*);"), 0) << "[check: nothing-visible] ";
    EXPECT_EQ(queryInt("MATCH (c:C {id: 5}) RETURN count(*);"), 0) << "[check: no-ghost-key] ";
    mustRun("CHECKPOINT;");
    if (!reopen()) {
        return;
    }
    EXPECT_EQ(queryInt("MATCH (c:C) RETURN count(*);"), 0) << "[check: nothing-after-reopen] ";
    expectIntegrity();
}

// Deux instances de la même base dans un processus. Le verrou de fichier du moteur
// (fcntl F_SETLK) n'exclut rien à l'intérieur d'un même processus : deux Database sur le même
// chemin font chacune leur point de reprise, avec deux gestionnaires d'espace libre qui
// donnent les mêmes pages. C'est la cause trouvée par la session cœur C++ à la corruption
// d'e2e_code (4 octobre au soir), dont le test rouvre la base pendant que l'instance d'avant
// finit sa fermeture. Attendu : la seconde ouverture est refusée. Dans un fils, qui sort sans
// fermer : aucun point de reprise de fermeture n'abîme la base du test.
TEST_F(SingleWriterCrash, SecondDatabaseOnTheSamePathInOneProcessIsRefused) {
    mustRun("CREATE NODE TABLE P(id INT64 PRIMARY KEY);");
    conn.reset();
    database.reset();
    const auto pid = fork();
    if (pid == 0) {
        disableCoreDumps();
        try {
            auto first = std::make_unique<rag3db::main::Database>(databasePath, *systemConfig);
            try {
                auto second =
                    std::make_unique<rag3db::main::Database>(databasePath, *systemConfig);
                // Ni l'une ni l'autre ne se ferme : sortir sans destructeur.
                second.release();
                first.release();
                _exit(5);
            } catch (const std::exception& e) {
                std::cerr << "  second open refused: " << e.what() << "\n";
                first.release();
                // Le refus attendu est l'erreur nommée, levée avant tout rejeu.
                _exit(std::string(e.what()).find("is already open for writing in this "
                                                  "process") != std::string::npos ?
                          0 :
                          4);
            }
        } catch (const std::exception& e) {
            std::cerr << "  first open failed: " << e.what() << "\n";
            _exit(3);
        }
    }
    int status = 0;
    waitpid(pid, &status, 0);
    createDBAndConn();
    EXPECT_TRUE(WIFEXITED(status) && WEXITSTATUS(status) == 0)
        << "[check: second-open-refused] "
        << (WIFSIGNALED(status) ? "killed by signal " + std::to_string(WTERMSIG(status)) :
            WEXITSTATUS(status) == 5 ? std::string("the second Database opened") :
            WEXITSTATUS(status) == 4 ? std::string("refused, but not by the named error") :
                                       "exit code " + std::to_string(WEXITSTATUS(status)));
}

// Condition élargie par la session cœur C++ (3 octobre au soir) : le plantage à
// l'ouverture vient de ce que le journal ne porte plus le LOAD EXTENSION — n'importe quel
// point de reprise après le chargement de l'extension, puis une écriture dans la table
// indexée, puis la mort. (La création d'index plantait toujours parce qu'elle écrit
// elle-même un point de reprise.) Ici : l'index vient d'une session précédente ; la
// session suivante charge l'extension, fait un CHECKPOINT, écrit, et meurt.
// Les attendus des deux gardes que la session cœur C++ code :
// - garde 1 : la base s'ouvre avec toutes ses lignes validées, l'index est « à rebâtir » :
//   la recherche et la création refusent par une erreur nommée, reconnue par le fragment
//   « is behind its table » (HNSWIndexUtils::INDEX_BEHIND_ITS_TABLE, nom fixé par la
//   session cœur C++), DROP puis CREATE le rebâtissent ; vrai aussi pour la mise à jour
//   d'un vecteur ;
// - garde 2, plus tard : l'index est juste dès la réouverture, sans rebâtir. Quand elle
//   est arrivée (4 octobre), les témoins de la garde 1 (IndexToRebuildIsNamed) ont reçu la
//   condition qui la fait encore jouer : la liste des extensions perdue.
enum class IndexedWrite { Insert, Delete, UpdateVector };

std::string indexedWriteName(IndexedWrite write) {
    switch (write) {
    case IndexedWrite::Insert:
        return "Insert";
    case IndexedWrite::Delete:
        return "Delete";
    case IndexedWrite::UpdateVector:
        return "UpdateVector";
    }
    return "?";
}

class ExtensionIndexRecovery : public SingleWriterCrash,
                               public ::testing::WithParamInterface<IndexedWrite> {
public:
    static constexpr int64_t NUM_DOCS = 1000;

    // L'index, créé dans une session précédente, fermée proprement.
    void createIndexInAnEarlierSession() {
        conn.reset();
        database.reset();
        rag3db::main::Database earlier(databasePath, *systemConfig);
        rag3db::main::Connection connection(&earlier);
        loadVectorExtension(connection);
        mustQuery(connection, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
        mustQuery(connection, stringFormat("UNWIND range(0, {}) AS i CREATE (:Doc {id: i, vec: "
                                           "[CAST(i % 17 AS FLOAT), CAST(i % 23 AS FLOAT), "
                                           "CAST(i % 29 AS FLOAT), CAST(i AS FLOAT)]});",
                                  NUM_DOCS - 1));
        mustQuery(connection,
            "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    }

    // L'écriture du cas, dans une session qui a chargé l'extension puis fait un
    // point de reprise ; « round » décale les lignes touchées pour une seconde mort.
    static void writeAfterCheckpoint(rag3db::main::Connection& connection, IndexedWrite write,
        int64_t round, bool checkpointAfterLoad = true) {
        loadVectorExtension(connection);
        mustQuery(connection, "CALL auto_checkpoint=false;");
        if (checkpointAfterLoad) {
            mustQuery(connection, "CHECKPOINT;");
        }
        const auto first = round * 20;
        switch (write) {
        case IndexedWrite::Insert:
            mustQuery(connection,
                stringFormat("UNWIND range({}, {}) AS i CREATE (:Doc {id: i, vec: [9.0, 9.0, "
                             "9.0, CAST(i AS FLOAT)]});",
                    5000 + first, 5000 + first + 19));
            break;
        case IndexedWrite::Delete:
            mustQuery(connection, stringFormat("MATCH (n:Doc) WHERE n.id >= {} AND n.id < {} "
                                               "DELETE n;",
                                      first, first + 20));
            break;
        case IndexedWrite::UpdateVector:
            mustQuery(connection, stringFormat("MATCH (n:Doc) WHERE n.id >= {} AND n.id < {} "
                                               "SET n.vec = [7.0, 7.0, 7.0, 7.0];",
                                      100 + first, 100 + first + 20));
            break;
        }
    }

    // Ce que la table doit contenir après « rounds » écritures validées.
    void expectEveryCommittedRow(IndexedWrite write, int64_t rounds) {
        const auto expected = write == IndexedWrite::Insert ? NUM_DOCS + 20 * rounds :
                              write == IndexedWrite::Delete ? NUM_DOCS - 20 * rounds :
                                                              NUM_DOCS;
        EXPECT_EQ(queryInt("MATCH (n:Doc) RETURN count(n);"), expected) << "[check: row-count] ";
        if (write == IndexedWrite::UpdateVector) {
            // Le vecteur entier, à partir de l'identifiant 100 : 58 lignes d'origine ont
            // déjà 7 en première composante, et l'identifiant 7 vaut [7, 7, 7, 7].
            EXPECT_EQ(queryInt("MATCH (n:Doc) WHERE n.id >= 100 AND n.vec = [7.0, 7.0, 7.0, "
                               "7.0] RETURN count(n);"),
                20 * rounds)
                << "[check: updated-vectors] ";
        }
    }

    // Pour une mise à jour de vecteur, l'invariant de l'index ne suffit pas : il vérifie que
    // chaque ligne vivante est rendue, pas que l'index connaît son NOUVEAU vecteur (une ligne
    // mise à jour reste trouvable sous l'ancien). Une recherche du nouveau vecteur doit
    // rendre les lignes mises à jour parmi ses plus proches.
    void expectIndexKnowsTheUpdatedVectors(IndexedWrite write, int64_t rounds) {
        if (write != IndexedWrite::UpdateVector) {
            return;
        }
        const auto expected = 20 * rounds;
        auto result = conn->query(stringFormat("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [7.0, "
                                               "7.0, 7.0, 7.0], {}, efs := 500) WITH node WHERE "
                                               "node.id >= 100 RETURN count(node.id);",
            expected + 1));
        EXPECT_TRUE(result->isSuccess()) << "[check: search-works] " << result->getErrorMessage();
        const auto found =
            result->isSuccess() ? result->getNext()->getValue(0)->getValue<int64_t>() : -1;
        EXPECT_EQ(found, expected)
            << "[check: index-knows-new-vectors] a search for the new vector finds " << found
            << " of the " << expected << " updated rows";
    }

    // Ouvre après une mort, comme un service qui redémarre : processus neuf, puis
    // réouverture ici, l'extension chargée.
    bool reopenAfterDeath() { return reopen(true /* vectorExtension */); }

    void dieAfterWriting(IndexedWrite write, int64_t round, bool checkpointAfterLoad = true) {
        runChild([&](rag3db::main::Database&, rag3db::main::Connection& connection) {
            writeAfterCheckpoint(connection, write, round, checkpointAfterLoad);
        });
        expectJournalToReplay();
    }
};

// Commun aux deux gardes : la base s'ouvre, sans planter, avec toutes ses lignes validées.
TEST_P(ExtensionIndexRecovery, OpensWithEveryCommittedRow) {
    createIndexInAnEarlierSession();
    dieAfterWriting(GetParam(), 0);
    if (!reopenAfterDeath()) {
        return;
    }
    expectEveryCommittedRow(GetParam(), 1);
}

// Garde 1 : l'index est « à rebâtir », et le dit par une erreur nommée à la recherche
// comme à la création ; DROP puis CREATE le rebâtissent, exact.
TEST_P(ExtensionIndexRecovery, IndexToRebuildIsNamed) {
    createIndexInAnEarlierSession();
    dieAfterWriting(GetParam(), 0);
    // Depuis la garde 2, la reprise charge l'extension notée à côté de la base et l'index
    // reste juste. La garde 1 ne joue plus que si la reprise n'a pas l'extension : ici, la
    // liste a été perdue (session cœur C++, 4 octobre).
    std::filesystem::remove(rag3db::storage::StorageUtils::getExtensionsFilePath(databasePath));
    if (!reopenAfterDeath()) {
        return;
    }
    auto search = conn->query(
        "CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [1.0, 2.0, 3.0, 4.0], 5) RETURN node.id;");
    const auto searchError = search->isSuccess() ? "" : search->getErrorMessage();
    std::cerr << "  search: " << (searchError.empty() ? "ok" : searchError) << "\n";
    EXPECT_TRUE(containsIgnoringCase(searchError, "is behind its table"))
        << "[check: search-names-index-to-rebuild] ";
    auto create = conn->query("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := "
                              "'l2', skip_if_exists := true);");
    const auto createError = create->isSuccess() ? "" : create->getErrorMessage();
    std::cerr << "  create: " << (createError.empty() ? "ok" : createError) << "\n";
    EXPECT_TRUE(containsIgnoringCase(createError, "is behind its table"))
        << "[check: create-names-index-to-rebuild] ";
    auto drop = conn->query("CALL DROP_VECTOR_INDEX('Doc', 'doc_index');");
    auto rebuild =
        conn->query("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    EXPECT_TRUE(drop->isSuccess() && rebuild->isSuccess())
        << "[check: drop-and-create-rebuild] "
        << (drop->isSuccess() ? rebuild->getErrorMessage() : drop->getErrorMessage());
    drop.reset();
    rebuild.reset();
    search.reset();
    create.reset();
    expectIntegrity();
}

// Garde 2 (15fcc3474) : l'index est juste dès la réouverture, sans rebâtir.
TEST_P(ExtensionIndexRecovery, IndexExactWithoutRebuild) {
    createIndexInAnEarlierSession();
    dieAfterWriting(GetParam(), 0);
    // Le vert doit dire « la liste des extensions a servi » : le journal ne porte plus le
    // LOAD EXTENSION (un point de reprise l'a suivi), et c'est la liste notée à côté de la
    // base qui le rend à la reprise. Sans elle, le cas éprouverait autre chose.
    ASSERT_TRUE(std::filesystem::exists(
        rag3db::storage::StorageUtils::getExtensionsFilePath(databasePath)))
        << "[check: extensions-list-written] no list of extensions next to the database";
    if (!reopenAfterDeath()) {
        return;
    }
    auto search = conn->query(
        "CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [1.0, 2.0, 3.0, 4.0], 5) RETURN node.id;");
    EXPECT_TRUE(search->isSuccess()) << "[check: search-works] " << search->getErrorMessage();
    search.reset();
    expectIndexKnowsTheUpdatedVectors(GetParam(), 1);
    expectIntegrity();
}

// Une seconde mort après la première réouverture : ouverture, écriture, mort, ouverture.
TEST_P(ExtensionIndexRecovery, SecondDeathAfterRecovery) {
    createIndexInAnEarlierSession();
    dieAfterWriting(GetParam(), 0);
    if (!reopenAfterDeath()) {
        return;
    }
    dieAfterWriting(GetParam(), 1);
    if (!reopenAfterDeath()) {
        return;
    }
    expectEveryCommittedRow(GetParam(), 2);
}

// Quand le journal porte encore le LOAD EXTENSION (aucun point de reprise après lui dans
// la session qui meurt), le rejeu tient l'index à jour : il répond juste sans rebâtir —
// insertion et suppression aujourd'hui ; la mise à jour d'un vecteur depuis le correctif
// de la session cœur C++ (avant, le rejeu d'une mise à jour n'informait aucun index).
TEST_P(ExtensionIndexRecovery, IndexMaintainedWhenTheJournalHoldsTheLoad) {
    createIndexInAnEarlierSession();
    dieAfterWriting(GetParam(), 0, false /* checkpointAfterLoad */);
    if (!reopenAfterDeath()) {
        return;
    }
    expectEveryCommittedRow(GetParam(), 1);
    auto search = conn->query(
        "CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [1.0, 2.0, 3.0, 4.0], 5) RETURN node.id;");
    EXPECT_TRUE(search->isSuccess()) << "[check: search-works] " << search->getErrorMessage();
    search.reset();
    expectIndexKnowsTheUpdatedVectors(GetParam(), 1);
    expectIntegrity();
}

INSTANTIATE_TEST_SUITE_P(Writes, ExtensionIndexRecovery,
    ::testing::Values(IndexedWrite::Insert, IndexedWrite::Delete, IndexedWrite::UpdateVector),
    [](const ::testing::TestParamInfo<IndexedWrite>& info) {
        return indexedWriteName(info.param);
    });

// La forme de la reprise de rag3weaver après un paquet défait (5 octobre). La reprise de
// l'arbre principal y plante, et ce plantage reste inexpliqué : SIGSEGV dans
// OnDiskHNSWIndex::update → shrinkForNode, sous un SET de vecteur, après un paquet défait et
// une réouverture avec journal. Ce qu'on en sait par la session cœur C++ :
// - l'index est posé sur la table vide : cosinus, 64 dimensions, mu 30, ml 60 ;
// - les lignes arrivent par paquets de 32, en COPY journalisés ;
// - un paquet échoue, et ROLLBACK annule plusieurs COPY d'une même transaction ;
// - à la reprise, MERGE … SET repose les vecteurs de lignes déjà validées, puis des SET
//   posent les autres.
// Trois fins :
// - la mort après quatre paquets validés ;
// - trois COPY annulés, un point de reprise, un paquet de plus validé, puis la mort ;
// - trois COPY annulés, puis une fermeture propre.
// La reprise tourne dans un processus fils, pour qu'un plantage n'emporte pas la passe.
enum class ProductEnd { Death, RolledBackCheckpointThenDeath, RolledBackThenClose };

std::string productEndName(ProductEnd end) {
    switch (end) {
    case ProductEnd::Death:
        return "Death";
    case ProductEnd::RolledBackCheckpointThenDeath:
        return "RolledBackCheckpointThenDeath";
    case ProductEnd::RolledBackThenClose:
        return "RolledBackThenClose";
    }
    return "?";
}

class ProductReloadRecovery : public SingleWriterCrash,
                              public ::testing::WithParamInterface<ProductEnd> {
public:
    static constexpr int64_t BATCH = 32;
    static constexpr int DIMENSIONS = 64;

    // Un vecteur de 64 dimensions par ligne et par génération, sans deux directions voisines.
    static std::string vectorOf(int64_t id, int generation) {
        std::string text = "[";
        for (auto j = 0; j < DIMENSIONS; j++) {
            auto value = static_cast<double>((id * 31 + j * 7 + generation * 13) % 97) / 97.0;
            if (j == (id + generation * 11) % DIMENSIONS) {
                value += 2.0;
            }
            text += (j ? "," : "") + std::to_string(value);
        }
        return text + "]";
    }

    std::string csvPath(int64_t batch) const {
        return databasePath + ".batch" + std::to_string(batch) + ".csv";
    }

    void writeBatches(int64_t numBatches) const {
        for (auto batch = 0; batch < numBatches; batch++) {
            std::ofstream csv(csvPath(batch));
            for (auto id = batch * BATCH; id < (batch + 1) * BATCH; id++) {
                csv << id << ",\"" << vectorOf(id, 0) << "\"\n";
            }
        }
    }

    void createIndexOnTheEmptyTable() {
        conn.reset();
        database.reset();
        rag3db::main::Database earlier(databasePath, *systemConfig);
        rag3db::main::Connection connection(&earlier);
        loadVectorExtension(connection);
        mustQuery(connection, stringFormat("CREATE NODE TABLE Chunk(id INT64 PRIMARY KEY, emb "
                                           "FLOAT[{}]);",
                                  DIMENSIONS));
        mustQuery(connection, "CALL CREATE_VECTOR_INDEX('Chunk', 'chunk_index', 'emb', mu := 30, "
                              "ml := 60, pu := 0.05, metric := 'cosine', alpha := 1.1, efc := "
                              "200);");
    }

    void load(rag3db::main::Connection& connection, ProductEnd end) const {
        loadVectorExtension(connection);
        mustQuery(connection, "CALL auto_checkpoint=false;");
        mustQuery(connection, "CALL force_checkpoint_on_copy=false;");
        const auto copy = [&](int64_t batch) {
            mustQuery(connection, "COPY Chunk FROM '" + csvPath(batch) + "' (header=false);");
        };
        for (auto batch = 0; batch < 4; batch++) {
            mustQuery(connection, "BEGIN TRANSACTION;");
            copy(batch);
            mustQuery(connection, "COMMIT;");
        }
        if (end == ProductEnd::Death) {
            return;
        }
        mustQuery(connection, "BEGIN TRANSACTION;");
        for (auto batch = 4; batch < 7; batch++) {
            copy(batch);
        }
        mustQuery(connection, "ROLLBACK;");
        if (end == ProductEnd::RolledBackCheckpointThenDeath) {
            mustQuery(connection, "CHECKPOINT;");
            copy(7);
        }
    }

    // Les lignes validées : les paquets 0 à 3, et le paquet 7 après le point de reprise.
    std::vector<int64_t> expectedIds(ProductEnd end) const {
        std::vector<int64_t> ids;
        for (auto id = 0; id < 4 * BATCH; id++) {
            ids.push_back(id);
        }
        if (end == ProductEnd::RolledBackCheckpointThenDeath) {
            for (auto id = 7 * BATCH; id < 8 * BATCH; id++) {
                ids.push_back(id);
            }
        }
        return ids;
    }

    // Dans le fils de la reprise : chaque ligne doit sortir en tête pour le vecteur que
    // generationOf lui donne ; rend le nombre de lignes manquées.
    static int64_t missedRows(rag3db::main::Connection& connection,
        const std::vector<int64_t>& ids, const std::function<int(int64_t)>& generationOf,
        const char* when) {
        const auto numRows = static_cast<int64_t>(ids.size());
        int64_t missed = 0;
        for (const auto id : ids) {
            auto result = connection.query("CALL QUERY_VECTOR_INDEX('Chunk', 'chunk_index', " +
                                           vectorOf(id, generationOf(id)) +
                                           ", 3, efs := 200) RETURN node.id, distance;");
            if (!result->isSuccess()) {
                std::cerr << "  " << when << ": search for " << id << ": "
                          << result->getErrorMessage() << "\n";
                return numRows;
            }
            int64_t best = -1;
            double bestDistance = INFINITY;
            while (result->hasNext()) {
                auto tuple = result->getNext();
                const auto distance = std::stod(tuple->getValue(1)->toString());
                if (distance < bestDistance) {
                    bestDistance = distance;
                    best = tuple->getValue(0)->getValue<int64_t>();
                }
            }
            if (best != id && ++missed <= 3) {
                std::cerr << "  " << when << ": row " << id << " gives " << best << "\n";
            }
        }
        std::cerr << "  " << when << ": " << numRows - missed << "/" << numRows
                  << " rows found by their exact vector\n";
        return missed;
    }

    // Le fils de la reprise. Codes : 0 tenu, 4 requête échouée, 5 lignes manquées après le
    // rejeu, 6 après les SET, 7 après un point de reprise et une réouverture, 8 mauvais compte.
    int recoverInAChild(const std::vector<int64_t>& ids) {
        const auto numRows = static_cast<int64_t>(ids.size());
        const auto pid = fork();
        if (pid == 0) {
            disableCoreDumps();
            const auto run = [&]() -> int {
                auto childDatabase =
                    std::make_unique<rag3db::main::Database>(databasePath, *systemConfig);
                auto connection = std::make_unique<rag3db::main::Connection>(childDatabase.get());
                loadVectorExtension(*connection);
                const auto ok = [&](const std::string& query) {
                    auto result = connection->query(query);
                    if (!result->isSuccess()) {
                        std::cerr << "  " << query.substr(0, 120) << ": "
                                  << result->getErrorMessage() << "\n";
                    }
                    return result->isSuccess();
                };
                auto count = connection->query("MATCH (n:Chunk) RETURN count(n);");
                if (!count->isSuccess() ||
                    count->getNext()->getValue(0)->getValue<int64_t>() != numRows) {
                    return 8;
                }
                count.reset();
                if (missedRows(*connection, ids, [](int64_t) { return 0; }, "after the replay")) {
                    return 5;
                }
                // Les vecteurs reposés par MERGE … SET, par paquets, sur des lignes validées.
                for (auto first = 0; first < numRows; first += BATCH) {
                    std::string rows;
                    for (auto i = first; i < std::min(numRows, first + BATCH); i++) {
                        const auto id = ids[i];
                        rows += (rows.empty() ? "" : ", ") + stringFormat("{id: {}, v: {}}", id,
                                                                  vectorOf(id, 1));
                    }
                    if (!ok("UNWIND [" + rows + "] AS r MERGE (n:Chunk {id: r.id}) SET n.emb = "
                                                "r.v;")) {
                        return 4;
                    }
                }
                // Puis des SET ligne à ligne, vers des vecteurs neufs.
                for (const auto id : ids) {
                    if (id % 3 != 0) {
                        continue;
                    }
                    if (!ok(stringFormat("MATCH (n:Chunk {id: {}}) SET n.emb = {};", id,
                            vectorOf(id, 2)))) {
                        return 4;
                    }
                }
                const auto generation = [](int64_t id) { return id % 3 == 0 ? 2 : 1; };
                if (missedRows(*connection, ids, generation, "after the SETs")) {
                    return 6;
                }
                if (!ok("CHECKPOINT;")) {
                    return 4;
                }
                connection.reset();
                childDatabase.reset();
                childDatabase =
                    std::make_unique<rag3db::main::Database>(databasePath, *systemConfig);
                connection = std::make_unique<rag3db::main::Connection>(childDatabase.get());
                loadVectorExtension(*connection);
                if (missedRows(*connection, ids, generation, "after reopening")) {
                    return 7;
                }
                return 0;
            };
            int code = 3;
            try {
                code = run();
            } catch (const std::exception& e) {
                std::cerr << "  recovery child: " << e.what() << "\n";
            }
            _exit(code);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        if (WIFSIGNALED(status)) {
            return 128 + WTERMSIG(status);
        }
        return WIFEXITED(status) ? WEXITSTATUS(status) : -1;
    }
};

TEST_P(ProductReloadRecovery, VectorsSetAfterRecoveryAreFound) {
    const auto end = GetParam();
    writeBatches(8);
    createIndexOnTheEmptyTable();
    if (end == ProductEnd::RolledBackThenClose) {
        // Une fermeture propre : le point de reprise de la fermeture n'est pas coupé.
        rag3db::main::Database session(databasePath, *systemConfig);
        rag3db::main::Connection connection(&session);
        load(connection, end);
    } else {
        runChild([&](rag3db::main::Database&, rag3db::main::Connection& connection) {
            load(connection, end);
        });
        expectJournalToReplay();
    }
    std::cerr << "  journal at reopening: " << walSize() << " bytes\n";
    const auto probe = probeOpenInFreshProcess(databasePath, true /* vectorExtension */);
    ASSERT_TRUE(probe.empty()) << "[check: database-opens-without-crash] " << probe;
    const auto code = recoverInAChild(expectedIds(end));
    std::cerr << "  recovery child: " << code << "\n";
    EXPECT_LT(code, 128) << "[check: recovery-survives] the recovery died by signal "
                         << code - 128 << " (11 = SIGSEGV)";
    EXPECT_NE(code, 8) << "[check: row-count] ";
    EXPECT_NE(code, 4) << "[check: recovery-queries] ";
    EXPECT_NE(code, 3) << "[check: recovery-runs] ";
    EXPECT_NE(code, 5) << "[check: rows-found-after-replay] ";
    EXPECT_NE(code, 6) << "[check: rows-found-after-the-sets] ";
    EXPECT_NE(code, 7) << "[check: rows-found-after-reopening] ";
    for (auto batch = 0; batch < 8; batch++) {
        std::filesystem::remove(csvPath(batch));
    }
}

INSTANTIATE_TEST_SUITE_P(Ends, ProductReloadRecovery,
    ::testing::Values(ProductEnd::Death, ProductEnd::RolledBackCheckpointThenDeath,
        ProductEnd::RolledBackThenClose),
    [](const ::testing::TestParamInfo<ProductEnd>& info) { return productEndName(info.param); });

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

// Le chargement en masse journalisé (étape 2 de la session cœur C++, page
// coeur-cpp/04-le-chargement-en-masse-journalise.md, §7, cas 1 et 2). Avec
// force_checkpoint_on_copy=false, un COPY de nœuds écrit ses lignes au journal au lieu de
// forcer un point de reprise. Aucun point de reprise entre-temps : auto_checkpoint=false, et
// le journal vérifié non vide avant de rouvrir.
class JournaledCopyDeath : public SingleWriterCrash {
public:
    std::string csvPath() const { return databasePath + ".journaled.csv"; }

    void writeCsv(int64_t first, int64_t count) const {
        std::ofstream csv(csvPath());
        for (auto id = first; id < first + count; id++) {
            csv << id << ",name " << id << "\n";
        }
    }

    static void journaledSession(rag3db::main::Connection& connection) {
        mustQuery(connection, "CALL auto_checkpoint=false;");
        mustQuery(connection, "CALL force_checkpoint_on_copy=false;");
        mustQuery(connection, "CALL force_checkpoint_on_close=false;");
    }

    uint64_t dataFileSize() const {
        std::error_code ec;
        const auto size = std::filesystem::file_size(databasePath, ec);
        return ec ? 0 : size;
    }
};

// Cas 1 : mort au milieu d'un COPY. Le fils valide cent lignes, pose un marqueur, lance un
// COPY de 300 000 lignes ; le père le tue un délai donné après le marqueur. Trois morts de
// suite sur la même base, chacune suivie d'une réouverture : les cent lignes validées sont
// là, le COPY est tout ou rien, et le fichier de données ne grossit pas d'une mort à
// l'autre (les pages qu'un COPY tué avait écrites ne s'accumulent pas).
TEST_F(JournaledCopyDeath, DeathInTheMiddleOfACopyLeavesNothingOfIt) {
    // Un million de lignes : le COPY dure assez pour que les trois morts tombent dedans,
    // même sur une machine chargée.
    constexpr int64_t COPY_ROWS = 1'000'000;
    writeCsv(0, COPY_ROWS);
    const auto marker = databasePath + ".copy-started";
    runChild([&](rag3db::main::Database&, rag3db::main::Connection& connection) {
        journaledSession(connection);
        mustQuery(connection, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);");
        mustQuery(connection, "CHECKPOINT;");
        mustQuery(connection, "UNWIND range(1000000, 1000099) AS i CREATE (:Doc {id: i, name: "
                              "'kept ' + CAST(i AS STRING)});");
    });
    const std::vector<std::chrono::milliseconds> delays{std::chrono::milliseconds(30),
        std::chrono::milliseconds(80), std::chrono::milliseconds(150)};
    std::vector<uint64_t> sizes;
    int killedMidway = 0;
    for (size_t round = 0; round < delays.size(); round++) {
        const auto delay = delays[round];
        std::filesystem::remove(marker);
        conn.reset();
        database.reset();
        const auto pid = fork();
        if (pid == 0) {
            disableCoreDumps();
            try {
                rag3db::main::Database childDatabase(databasePath, *systemConfig);
                rag3db::main::Connection childConnection(&childDatabase);
                journaledSession(childConnection);
                // Une ligne validée par ce fils : le journal n'est pas vide à sa mort, même
                // si le COPY n'a rien validé (la fermeture du père a fait un point de reprise).
                mustQuery(childConnection, "CREATE (:Doc {id: " + std::to_string(2000000 + round) +
                                               ", name: 'round'});");
                std::ofstream(marker) << "1";
                childConnection.query("COPY Doc FROM '" + csvPath() + "' (header=false);");
                // Le COPY a fini avant le délai : attendre la mort, base ouverte.
                while (true) {
                    pause();
                }
            } catch (const std::exception& e) {
                std::cerr << "  child failed: " << e.what() << "\n";
                _exit(3);
            }
        }
        for (int i = 0; i < 30000 && !std::filesystem::exists(marker); i++) {
            usleep(1000);
        }
        std::this_thread::sleep_for(delay);
        kill(pid, SIGKILL);
        int status = 0;
        waitpid(pid, &status, 0);
        ASSERT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
            << "[check: killed] the child did not die by SIGKILL";
        expectJournalToReplay();
        if (!reopen()) {
            return;
        }
        EXPECT_EQ(queryInt("MATCH (n:Doc) WHERE n.id >= 1000000 AND n.id < 2000000 RETURN "
                           "count(*);"),
            100)
            << "[check: committed-rows-kept] after a death " << delay.count() << " ms into a COPY";
        EXPECT_EQ(queryInt("MATCH (n:Doc) WHERE n.id >= 2000000 RETURN count(*);"),
            static_cast<int64_t>(round) + 1)
            << "[check: committed-rows-kept] the row committed just before the COPY";
        const auto copied = queryInt("MATCH (n:Doc) WHERE n.id < 1000000 RETURN count(*);");
        EXPECT_TRUE(copied == 0 || copied == COPY_ROWS)
            << "[check: copy-all-or-nothing] " << copied << " rows of the COPY after a death "
            << delay.count() << " ms into it";
        const auto midway = copied == 0;
        if (midway) {
            killedMidway++;
        } else {
            // Le COPY avait fini : le défaire pour que la mort suivante tombe encore dedans.
            mustRun("MATCH (n:Doc) WHERE n.id < 1000000 DELETE n;");
        }
        expectIntegrity();
        conn.reset();
        database.reset();
        const auto size = dataFileSize();
        // Seules les morts au milieu d'un COPY se comparent : un COPY fini a écrit ses
        // pages pour de bon.
        if (midway) {
            sizes.push_back(size);
        }
        std::cerr << "  death " << delay.count() << " ms into the COPY: " << copied
                  << " rows of it, data file " << size << " bytes\n";
    }
    EXPECT_GT(killedMidway, 0) << "[check: death-in-the-middle] every COPY finished before its "
                                  "death: the case did not test a death in the middle";
    if (sizes.size() >= 2) {
        EXPECT_LE(sizes.back(), sizes.front() + sizes.front() / 2)
            << "[check: data-file-bounded] the data file grew from " << sizes.front() << " to "
            << sizes.back() << " bytes over " << sizes.size() << " deaths in a COPY";
    }
    createDBAndConn();
    // La table accepte encore le même COPY, en entier.
    mustRun("COPY Doc FROM '" + csvPath() + "' (header=false);");
    EXPECT_EQ(queryInt("MATCH (n:Doc) WHERE n.id < 1000000 RETURN count(*);"), COPY_ROWS)
        << "[check: copy-after-the-deaths] ";
    std::filesystem::remove(marker);
    std::filesystem::remove(csvPath());
}

// Cas 2 : mort juste après un COPY validé, suivi d'écritures ordinaires qui relient ses
// lignes. Toutes les lignes, chaque clé retrouvée par l'index, les deux sens de la table de
// relations d'accord (contrôle d'intégrité, niveaux 1 et 2), et la base reste inscriptible.
TEST_F(JournaledCopyDeath, DeathJustAfterACommittedCopyKeepsEveryKeyAndBothDirections) {
    constexpr int64_t COPY_ROWS = 20'000;
    writeCsv(0, COPY_ROWS);
    runChild([&](rag3db::main::Database&, rag3db::main::Connection& connection) {
        journaledSession(connection);
        mustQuery(connection, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);");
        mustQuery(connection, "CREATE REL TABLE Next(FROM Doc TO Doc, step INT64);");
        mustQuery(connection, "CHECKPOINT;");
        mustQuery(connection, "COPY Doc FROM '" + csvPath() + "' (header=false);");
        mustQuery(connection, "MATCH (a:Doc), (b:Doc) WHERE a.id % 10 = 0 AND b.id = a.id + 1 "
                              "CREATE (a)-[:Next {step: a.id}]->(b);");
    });
    expectJournalToReplay();
    if (!reopen()) {
        return;
    }
    EXPECT_EQ(queryInt("MATCH (n:Doc) RETURN count(*);"), COPY_ROWS) << "[check: copied-rows] ";
    EXPECT_EQ(queryInt("MATCH (n:Doc) WHERE n.name = 'name ' + CAST(n.id AS STRING) RETURN "
                       "count(*);"),
        COPY_ROWS)
        << "[check: copied-values] ";
    EXPECT_EQ(queryInt("MATCH (a:Doc)-[r:Next]->(b:Doc) WHERE b.id = a.id + 1 AND r.step = a.id "
                       "RETURN count(*);"),
        COPY_ROWS / 10)
        << "[check: relations-on-the-copied-rows] ";
    EXPECT_EQ(queryInt("MATCH (b:Doc)<-[r:Next]-(a:Doc) RETURN count(*);"), COPY_ROWS / 10)
        << "[check: relations-backward] ";
    expectIntegrity();
    expectStillWritable("CREATE (:Doc {id: 20000, name: 'name 20000'});");
    std::filesystem::remove(csvPath());
}

#endif
