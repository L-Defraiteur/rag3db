// Le point de reprise qui ne fige personne (ticket
// 2026-10-18-un-point-de-reprise-fige-toutes-les-transactions-le-temps-de-s-ecrire, voie 2, la
// forme de PostgreSQL) : un point de reprise n'arrête ni les BEGIN ni les lectures, et le tampon
// écrit sur disque des pages validées et pas encore reportées quand il en manque (l'écriture à la
// demande, « steal »), le journal permettant de les rejouer. Aujourd'hui, un point de reprise
// retient tous les BEGIN, lecteurs compris, et le tampon ne rend pas une page validée tant qu'elle
// n'est pas reportée.
//
// - CheckpointNoStall.BeginDuringACheckpointDoesNotWait : une table de vecteurs réécrits en entier
//   (des milliers de pages à reporter) ; une connexion lance CHECKPOINT, une autre ouvre une
//   transaction 30 ms plus tard. begin-during-a-checkpoint-does-not-wait : le BEGIN rend la main en
//   moins de 100 ms, quelle que soit la durée du point de reprise.
// - CheckpointNoStall.ReadDuringACheckpointDoesNotWait : la même chose pour une lecture par clé en
//   automatique (read-during-a-checkpoint-does-not-wait).
// - CheckpointNoStall.CommittedPagesAreWrittenOutWhenTheBufferIsFull : le tampon des tests (64 Mo),
//   le point de reprise automatique coupé ; un écrivain valide des lots de vecteurs jusqu'à TARGET
//   lignes, plus que le tampon n'en tient. committed-pages-are-written-out-when-the-buffer-is-full
//   : aucun lot n'est refusé par le tampon, et chaque ligne validée se relit. C'est le tableau des
//   seuils (ticket du point de reprise, « La première voie, mesurée ») retourné en témoin ; le
//   scénario des vingt écrivains le joue à plusieurs par SCENARIO_NO_AUTO_CHECKPOINT=1.
// - CheckpointNoStall.CrashAfterWrittenOutPagesReplaysExactly : la même écriture dans un processus
//   fils, tué par SIGKILL base ouverte, après TARGET lignes validées ; le père rouvre.
//   crash-after-written-out-pages-replays-exactly : le fils a validé TARGET lignes (des pages ont
//   donc été rendues), et la base rouverte en porte exactement le nombre que son journal dit
//   validé.
//
// Les témoins de V2-2 (page 19 des docs coeur-cpp, 38baca51a), écrits avant le lot :
// - CheckpointNoStall.OpenReaderOnNodesIsNotWaitedFor (V2-2a) : un lecteur ouvre une transaction
//   en lecture seule et lit un compte et une somme sur une table de nœuds que le point de reprise
//   réécrit ; un écrivain modifie une ligne et en crée une, et valide ; puis CHECKPOINT, lecteur
//   toujours ouvert. checkpoint-does-not-wait-for-an-open-reader : le CHECKPOINT réussit pendant
//   que le lecteur reste ouvert (aujourd'hui, il attend son départ jusqu'au délai, 5 s, et échoue)
//   ; open-reader-keeps-its-snapshot : le lecteur relit la même chose pendant et après le point de
//   reprise, sans voir l'écriture validée après son BEGIN.
// - CheckpointNoStall.OpenReaderOnRelsIsNotWaitedFor (V2-2b) : la même chose sur une table de
//   relations dont une propriété a été réécrite en entier.
// - CheckpointNoStall.WriterCommittedDuringACheckpointSurvivesACrash (V2-2c) : dans un fils, un
//   CHECKPOINT coûteux part ; 30 ms après, un écrivain fait BEGIN, une ligne, COMMIT ; le fils se
//   tue par SIGKILL, base ouverte, dès la validation. writer-commits-during-a-checkpoint : la
//   validation est faite avant la fin du point de reprise ; committed-row-survives-a-crash-during-
//   the-checkpoint : la base rouverte porte la ligne.
// - CheckpointOpenReader.OpenReaderStillSeesARowDeletedAfterItsBegin (V2-2a, la suppression) : la
//   ligne relue par le lecteur est supprimée et validée entre son BEGIN et le CHECKPOINT ; il doit
//   encore la voir pendant et après (la suppression portée avec son horodatage).
// - CheckpointNoStall.WriterCommittedWhileTheCheckpointWritesSurvivesItsDeath (V2-2c, le journal
//   scellé) : dans un fils, un Checkpointer qui, après checkpointStorage (toutes les tables
//   reportées, avant le catalogue et l'en-tête : le point le plus proche de « entre deux tables »
//   sans crochet dans le moteur), laisse un écrivain valider jusqu'à 3 s, puis tue le processus
//   base ouverte. writer-commits-while-the-checkpoint-writes : l'écrivain a validé dans cette
//   fenêtre ; committed-row-survives-the-checkpoints-death : la base rouverte porte sa ligne (le
//   journal scellé, sans CHECKPOINT final, rejoué, puis le journal neuf).

#ifndef __SINGLE_THREADED__

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <atomic>
#include <chrono>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <random>
#include <string>
#include <thread>
#include <vector>

#include "../flaky_checkpointer.h"
#include "bench_harness.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/checkpointer.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

using Clock = std::chrono::steady_clock;
constexpr int DIM = 64;
// Les lignes du point de reprise : COPY, puis chaque vecteur réécrit, pour des milliers de pages à
// reporter.
constexpr int64_t CHECKPOINT_ROWS = 100'000;
constexpr double MAX_WAIT_MS = 100;
constexpr double MIN_CHECKPOINT_MS = 500;
// Le tampon des tests (64 Mo) tient environ 170 000 lignes de ce format (FullBufferRollback, 18
// octobre) ; TARGET en demande plus du double.
constexpr int64_t TARGET = 400'000;
constexpr int64_t BATCH = 5'000;

double msSince(Clock::time_point start) {
    return std::chrono::duration<double, std::milli>(Clock::now() - start).count();
}

std::string vectorLiteral(int64_t seed) {
    std::string out = "[";
    for (auto d = 0; d < DIM; ++d) {
        out += (d ? "," : "") + std::to_string(0.001 * static_cast<double>((seed * 31 + d) % 997));
    }
    return out + "]";
}

class CheckpointNoStall : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
    }

    static void ok(rag3db::main::Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess())
            << "[check: query] " << query.substr(0, 160) << " : " << result->getErrorMessage();
    }

    // Une base de CHECKPOINT_ROWS lignes dont chaque vecteur a été réécrit après le dernier point
    // de reprise : le prochain CHECKPOINT a des milliers de pages à reporter.
    void prepareACostlyCheckpoint() {
        systemConfig->bufferPoolSize = 2ull * 1024 * 1024 * 1024;
        createDBAndConn();
        concurrency::applyBenchSettings(*conn);
        ok(*conn, stringFormat("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[{}]);", DIM));
        const auto path = databasePath + ".docs.csv";
        {
            std::ofstream out(path);
            for (int64_t id = 0; id < CHECKPOINT_ROWS; ++id) {
                out << id << ",\"" << vectorLiteral(id) << "\"\n";
            }
        }
        ok(*conn, "COPY Doc FROM '" + path + "' (header=false);");
        std::filesystem::remove(path);
        ok(*conn, "CHECKPOINT;");
        ok(*conn, stringFormat("MATCH (d:Doc) SET d.vec = {};", vectorLiteral(7)));
    }

    // CHECKPOINT sur la connexion principale, et other 30 ms après son départ ; rend (durée du
    // point de reprise, durée de other, erreur de other).
    struct Overlap {
        double checkpointMs;
        double otherMs;
        std::string otherError;
    };
    Overlap duringACheckpoint(rag3db::main::Connection& other, const std::string& otherQuery) {
        Overlap out{};
        const auto start = Clock::now();
        std::thread checkpoint([&] {
            auto result = conn->query("CHECKPOINT;");
            out.checkpointMs = msSince(start);
            if (!result->isSuccess()) {
                out.checkpointMs = -1;
            }
        });
        std::this_thread::sleep_for(std::chrono::milliseconds(30));
        const auto otherStart = Clock::now();
        auto result = other.query(otherQuery);
        out.otherMs = msSince(otherStart);
        out.otherError = result->isSuccess() ? "" : result->getErrorMessage();
        checkpoint.join();
        return out;
    }

    // Des lots de BATCH lignes validés jusqu'à TARGET ; rend le nombre de lignes validées et le
    // premier refus. log, s'il est donné, reçoit le compte après chaque validation.
    static std::pair<int64_t, std::string> fill(rag3db::main::Connection& connection,
        std::FILE* log = nullptr) {
        int64_t committed = 0;
        while (committed < TARGET) {
            auto result =
                connection.query(stringFormat("UNWIND range({}, {}) AS i CREATE (:Doc {id: "
                                              "i, vec: {}});",
                    committed, committed + BATCH - 1, vectorLiteral(committed)));
            if (!result->isSuccess()) {
                return {committed, result->getErrorMessage()};
            }
            committed += BATCH;
            if (log) {
                std::fprintf(log, "%ld\n", static_cast<long>(committed));
                std::fflush(log);
            }
        }
        return {committed, ""};
    }
};

TEST_F(CheckpointNoStall, BeginDuringACheckpointDoesNotWait) {
    prepareACostlyCheckpoint();
    rag3db::main::Connection other(database.get());
    const auto r = duringACheckpoint(other, "BEGIN TRANSACTION;");
    other.query("ROLLBACK;");
    std::cerr << "  [point de reprise] CHECKPOINT " << r.checkpointMs
              << " ms ; BEGIN pendant : " << r.otherMs << " ms "
              << (r.otherError.empty() ? "" : "(" + r.otherError + ")") << "\n";
    ASSERT_GE(r.checkpointMs, MIN_CHECKPOINT_MS)
        << "[check: plan] the checkpoint took " << r.checkpointMs << " ms: nothing is witnessed";
    EXPECT_TRUE(r.otherError.empty() && r.otherMs <= MAX_WAIT_MS)
        << "[check: begin-during-a-checkpoint-does-not-wait] BEGIN took " << r.otherMs
        << " ms during a " << r.checkpointMs << " ms checkpoint " << r.otherError;
}

TEST_F(CheckpointNoStall, ReadDuringACheckpointDoesNotWait) {
    prepareACostlyCheckpoint();
    rag3db::main::Connection other(database.get());
    const auto r = duringACheckpoint(other, "MATCH (d:Doc {id: 4242}) RETURN d.id;");
    std::cerr << "  [point de reprise] CHECKPOINT " << r.checkpointMs
              << " ms ; lecture pendant : " << r.otherMs << " ms "
              << (r.otherError.empty() ? "" : "(" + r.otherError + ")") << "\n";
    ASSERT_GE(r.checkpointMs, MIN_CHECKPOINT_MS)
        << "[check: plan] the checkpoint took " << r.checkpointMs << " ms: nothing is witnessed";
    EXPECT_TRUE(r.otherError.empty() && r.otherMs <= MAX_WAIT_MS)
        << "[check: read-during-a-checkpoint-does-not-wait] the read took " << r.otherMs
        << " ms during a " << r.checkpointMs << " ms checkpoint " << r.otherError;
}

TEST_F(CheckpointNoStall, CommittedPagesAreWrittenOutWhenTheBufferIsFull) {
    createDBAndConn();
    concurrency::applyBenchSettings(*conn);
    ok(*conn, stringFormat("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[{}]);", DIM));
    const auto [committed, refusal] = fill(*conn);
    auto count = conn->query("MATCH (d:Doc) RETURN count(d);");
    const auto read = count->isSuccess() && count->hasNext() ?
                          count->getNext()->getValue(0)->getValue<int64_t>() :
                          -1;
    std::cerr << "  [point de reprise] tampon de 64 Mo, sans point de reprise : " << committed
              << " lignes validées sur " << TARGET << ", relues " << read
              << (refusal.empty() ? "" : ", refus : " + refusal.substr(0, 120)) << "\n";
    EXPECT_TRUE(refusal.empty() && committed == TARGET && read == TARGET)
        << "[check: committed-pages-are-written-out-when-the-buffer-is-full] " << committed
        << " rows committed of " << TARGET << ", " << read << " read back ; " << refusal;
}

TEST_F(CheckpointNoStall, CrashAfterWrittenOutPagesReplaysExactly) {
    namespace fs = std::filesystem;
    fs::create_directories(fs::path(databasePath).parent_path());
    const auto logPath = databasePath + ".committed";
    fs::remove(logPath);
    const auto pid = fork();
    if (pid == 0) {
        concurrency::disableCoreDumps();
        rag3db::main::Database child(databasePath, *systemConfig);
        rag3db::main::Connection connection(&child);
        concurrency::applyBenchSettings(connection);
        connection.query(
            stringFormat("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[{}]);", DIM));
        auto* log = std::fopen(logPath.c_str(), "w");
        fill(connection, log);
        // Mourir base ouverte : sortir de la portée fermerait la base proprement.
        kill(getpid(), SIGKILL);
        _exit(9);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    ASSERT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
        << "[check: plan] the child was not killed, status " << status;
    int64_t logged = 0;
    {
        std::ifstream in(logPath);
        for (int64_t value; in >> value;) {
            logged = value;
        }
    }
    createDBAndConn();
    concurrency::applyBenchSettings(*conn);
    auto count = conn->query("MATCH (d:Doc) RETURN count(d);");
    ASSERT_TRUE(count->isSuccess()) << "[check: query] " << count->getErrorMessage();
    const auto reopened = count->getNext()->getValue(0)->getValue<int64_t>();
    std::cerr << "  [point de reprise] arrêt brutal : le fils a validé " << logged << " lignes sur "
              << TARGET << ", la base rouverte en porte " << reopened << "\n";
    EXPECT_TRUE(logged == TARGET && reopened == logged)
        << "[check: crash-after-written-out-pages-replays-exactly] the child committed " << logged
        << " rows of " << TARGET << " (fewer means the buffer refused instead of writing pages "
        << "out), the reopened database holds " << reopened;
}

// Une base de relations dont une propriété a été réécrite après le dernier point de reprise : le
// prochain CHECKPOINT a les pages de la table de relations à reporter.
constexpr int64_t REL_NODES = 10'000;
constexpr int64_t REL_EDGES = 300'000;

class CheckpointOpenReader : public CheckpointNoStall {
protected:
    void prepareACostlyRelCheckpoint() {
        systemConfig->bufferPoolSize = 2ull * 1024 * 1024 * 1024;
        createDBAndConn();
        concurrency::applyBenchSettings(*conn);
        ok(*conn, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY);");
        ok(*conn, "CREATE REL TABLE Link(FROM Doc TO Doc, weight INT64);");
        ok(*conn, stringFormat("UNWIND range(0, {}) AS i CREATE (:Doc {id: i});", REL_NODES - 1));
        const auto path = databasePath + ".links.csv";
        {
            std::ofstream out(path);
            std::mt19937 engine(3);
            std::uniform_int_distribution<int64_t> any(0, REL_NODES - 1);
            for (int64_t i = 0; i < REL_EDGES; ++i) {
                out << any(engine) << "," << any(engine) << "," << i << "\n";
            }
        }
        ok(*conn, "COPY Link FROM '" + path + "' (header=false);");
        std::filesystem::remove(path);
        ok(*conn, "CHECKPOINT;");
        ok(*conn, "MATCH ()-[l:Link]->() SET l.weight = l.weight + 1;");
    }

    static std::string read(rag3db::main::Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        if (!result->isSuccess()) {
            return "ERREUR : " + result->getErrorMessage().substr(0, 160);
        }
        std::string out;
        while (result->hasNext()) {
            out += result->getNext()->toString();
        }
        return out;
    }

    // Le lecteur ouvert avant le CHECKPOINT : il lit, l'écrivain valide, le CHECKPOINT part sur la
    // connexion principale ; le lecteur relit 30 ms après son départ et après sa fin, puis ferme.
    void openReaderAcrossACheckpoint(const std::string& query,
        const std::vector<std::string>& writes) {
        rag3db::main::Connection reader(database.get());
        rag3db::main::Connection writer(database.get());
        ok(reader, "BEGIN TRANSACTION READ ONLY;");
        const auto before = read(reader, query);
        for (const auto& write : writes) {
            ok(writer, write);
        }
        const auto seenByOthers = read(writer, query);
        const auto start = Clock::now();
        std::atomic<bool> done{false};
        double checkpointMs = 0;
        std::string checkpointError;
        std::thread checkpoint([&] {
            auto result = conn->query("CHECKPOINT;");
            checkpointMs = msSince(start);
            checkpointError = result->isSuccess() ? "" : result->getErrorMessage();
            done = true;
        });
        std::this_thread::sleep_for(std::chrono::milliseconds(30));
        const auto during = read(reader, query);
        const bool checkpointEndedFirst = done.load();
        checkpoint.join();
        const auto after = read(reader, query);
        reader.query("COMMIT;");
        std::cerr << "  [lecteur ouvert] CHECKPOINT " << checkpointMs << " ms"
                  << (checkpointError.empty() ?
                             "" :
                             " (refusé : " + checkpointError.substr(0, 120) + ")")
                  << " ; le lecteur lit avant « " << before << " », pendant « " << during
                  << " », après « " << after << " » ; les autres lisent « " << seenByOthers
                  << " »\n";
        ASSERT_NE(seenByOthers, before)
            << "[check: plan] the writes did not change what the query reads: nothing is witnessed";
        ASSERT_FALSE(checkpointEndedFirst)
            << "[check: plan] the checkpoint ended before the read made during it";
        EXPECT_TRUE(checkpointError.empty())
            << "[check: checkpoint-does-not-wait-for-an-open-reader] the CHECKPOINT, with a reader "
               "open, took "
            << checkpointMs << " ms and was refused: " << checkpointError;
        EXPECT_TRUE(during == before && after == before)
            << "[check: open-reader-keeps-its-snapshot] before « " << before << " », during « "
            << during << " », after « " << after << " »";
    }
};

TEST_F(CheckpointOpenReader, OpenReaderOnNodesIsNotWaitedFor) {
    prepareACostlyCheckpoint();
    openReaderAcrossACheckpoint("MATCH (d:Doc) RETURN count(d), sum(d.vec[1]);",
        {stringFormat("MATCH (d:Doc {id: 7}) SET d.vec = {};", vectorLiteral(424242)),
            stringFormat("CREATE (:Doc {id: 50000000, vec: {}});", vectorLiteral(1))});
}

TEST_F(CheckpointOpenReader, OpenReaderOnRelsIsNotWaitedFor) {
    prepareACostlyRelCheckpoint();
    openReaderAcrossACheckpoint("MATCH ()-[l:Link]->() RETURN count(l), sum(l.weight);",
        {"MATCH (a:Doc {id: 1}), (b:Doc {id: 2}) CREATE (a)-[:Link {weight: 1000000}]->(b);",
            "MATCH ()-[l:Link]->() WHERE l.weight = 6 SET l.weight = 7000000;"});
}

TEST_F(CheckpointNoStall, WriterCommittedDuringACheckpointSurvivesACrash) {
    namespace fs = std::filesystem;
    const auto logPath = databasePath + ".writer";
    fs::create_directories(fs::path(databasePath).parent_path());
    fs::remove(logPath);
    const auto pid = fork();
    if (pid == 0) {
        concurrency::disableCoreDumps();
        prepareACostlyCheckpoint();
        rag3db::main::Connection writer(database.get());
        std::atomic<bool> checkpointDone{false};
        std::thread checkpoint([&] {
            conn->query("CHECKPOINT;");
            checkpointDone = true;
        });
        std::this_thread::sleep_for(std::chrono::milliseconds(30));
        const bool begun = writer.query("BEGIN TRANSACTION;")->isSuccess();
        const bool created =
            begun &&
            writer.query(stringFormat("CREATE (:Doc {id: 60000000, vec: {}});", vectorLiteral(2)))
                ->isSuccess();
        const bool committed = created && writer.query("COMMIT;")->isSuccess();
        const bool during = !checkpointDone.load();
        {
            std::ofstream log(logPath);
            log << "committed " << committed << " during " << during << std::endl;
        }
        // Mourir base ouverte, le point de reprise peut-être encore en cours.
        kill(getpid(), SIGKILL);
        _exit(9);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    ASSERT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
        << "[check: plan] the child was not killed, status " << status;
    std::string word;
    int committed = 0;
    int during = 0;
    {
        std::ifstream in(logPath);
        in >> word >> committed >> word >> during;
    }
    ASSERT_EQ(committed, 1) << "[check: plan] the writer did not commit";
    createDBAndConn();
    concurrency::applyBenchSettings(*conn);
    auto count = conn->query("MATCH (d:Doc {id: 60000000}) RETURN count(d);");
    ASSERT_TRUE(count->isSuccess()) << "[check: query] " << count->getErrorMessage();
    const auto present = count->getNext()->getValue(0)->getValue<int64_t>();
    std::cerr << "  [écrivain tué] validé pendant le point de reprise : " << during
              << " ; ligne à la réouverture : " << present << "\n";
    EXPECT_EQ(during, 1) << "[check: writer-commits-during-a-checkpoint] the writer committed only "
                            "after the checkpoint ended";
    EXPECT_EQ(present, 1) << "[check: committed-row-survives-a-crash-during-the-checkpoint]";
}

TEST_F(CheckpointOpenReader, OpenReaderStillSeesARowDeletedAfterItsBegin) {
    prepareACostlyCheckpoint();
    openReaderAcrossACheckpoint(
        "MATCH (d:Doc) RETURN count(d), sum(CASE WHEN d.id = 9 THEN 1 ELSE 0 END);",
        {"MATCH (d:Doc {id: 9}) DELETE d;"});
}

// L'écrivain de V2-2c (le journal scellé) : le Checkpointer le laisse valider après avoir reporté
// les tables, puis meurt.
std::atomic<bool> writerCommittedInTheWindow{false};

class CheckpointerDyingAfterStorage final : public rag3db::storage::Checkpointer {
public:
    explicit CheckpointerDyingAfterStorage(rag3db::main::ClientContext& context)
        : Checkpointer(context) {}

protected:
    bool checkpointStorage() override {
        const auto changed = Checkpointer::checkpointStorage();
        const auto deadline = Clock::now() + std::chrono::seconds(3);
        while (!writerCommittedInTheWindow.load() && Clock::now() < deadline) {
            std::this_thread::sleep_for(std::chrono::milliseconds(5));
        }
        kill(getpid(), SIGKILL);
        _exit(9);
        return changed;
    }
};

TEST_F(CheckpointNoStall, WriterCommittedWhileTheCheckpointWritesSurvivesItsDeath) {
    namespace fs = std::filesystem;
    const auto logPath = databasePath + ".sealed";
    fs::create_directories(fs::path(databasePath).parent_path());
    fs::remove(logPath);
    const auto pid = fork();
    if (pid == 0) {
        concurrency::disableCoreDumps();
        prepareACostlyCheckpoint();
        const FlakyCheckpointer dying([](rag3db::main::ClientContext& context) {
            return std::make_unique<CheckpointerDyingAfterStorage>(context);
        });
        dying.setCheckpointer(*conn->getClientContext());
        std::thread checkpoint([&] { conn->query("CHECKPOINT;"); });
        std::this_thread::sleep_for(std::chrono::milliseconds(30));
        rag3db::main::Connection writer(database.get());
        const bool begun = writer.query("BEGIN TRANSACTION;")->isSuccess();
        const bool created =
            begun &&
            writer.query(stringFormat("CREATE (:Doc {id: 70000000, vec: {}});", vectorLiteral(3)))
                ->isSuccess();
        const bool committed = created && writer.query("COMMIT;")->isSuccess();
        {
            std::ofstream log(logPath);
            log << "committed " << committed << std::endl;
        }
        writerCommittedInTheWindow = committed;
        checkpoint.join();
        // Le Checkpointer tue le processus ; s'il ne l'a pas fait, mourir ici, base ouverte.
        kill(getpid(), SIGKILL);
        _exit(9);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    ASSERT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
        << "[check: plan] the child was not killed, status " << status;
    std::string word;
    int committed = 0;
    {
        std::ifstream in(logPath);
        in >> word >> committed;
    }
    createDBAndConn();
    concurrency::applyBenchSettings(*conn);
    auto count = conn->query("MATCH (d:Doc {id: 70000000}) RETURN count(d);");
    ASSERT_TRUE(count->isSuccess()) << "[check: query] " << count->getErrorMessage();
    const auto present = count->getNext()->getValue(0)->getValue<int64_t>();
    std::cerr << "  [journal scellé] l'écrivain a validé pendant l'écriture du point de reprise : "
              << committed << " ; ligne à la réouverture : " << present << "\n";
    EXPECT_EQ(committed, 1) << "[check: writer-commits-while-the-checkpoint-writes] the writer "
                               "could not commit before the checkpoint died";
    EXPECT_EQ(present, static_cast<int64_t>(committed))
        << "[check: committed-row-survives-the-checkpoints-death] committed " << committed
        << ", present " << present;
}

} // namespace

#endif
