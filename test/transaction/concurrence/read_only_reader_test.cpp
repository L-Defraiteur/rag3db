#include <sys/mman.h>
#include <sys/wait.h>
#include <unistd.h>

#include <chrono>
#include <cstdlib>
#include <iostream>
#include <map>
#include <string>
#include <thread>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::testing;

// Un lecteur qui insiste pendant qu'un autre processus écrit (relevé par la session de
// l'arbre principal sur e2e_prise_atomique::un_lecteur_qui_insiste_pendant_qu_on_ecrit :
// un refus survit à la patience de l'appelant 1 à 3 fois sur 80).
//
// La même scène en Cypher brut, à deux processus : le fils écrit sans relâche, avec un
// point de reprise tous les cinq enregistrements ; le père ouvre la base en lecture seule
// quatre-vingts fois, et chaque ouverture a la patience de Rag3dbConnection::read_only
// (5, 10, 20, 40, 40… ms, dans un budget de 250 ms). Un refus qui survit à ce budget est
// relevé mot pour mot.

namespace {

struct WriterReport {
    volatile int ready;
    volatile int failed;
    volatile int rowsWritten;
    volatile int stopRequested;
};

WriterReport* shareReport() {
    auto* report = static_cast<WriterReport*>(mmap(nullptr, sizeof(WriterReport),
        PROT_READ | PROT_WRITE, MAP_ANONYMOUS | MAP_SHARED, -1, 0));
    *report = WriterReport{0, 0, 0, 0};
    return report;
}

constexpr uint64_t PATIENCE_MS = 250;
constexpr int CYCLES = 80;
// Réglable pour la mesure (RO_WRITE_PAUSE_US). Par défaut 0,2 ms : environ 470 points de
// reprise par seconde, un refus dans 17 passes sur 20 (4 octobre 2026). À 1 ms, le rythme du
// test d'origine, aucun refus en 5 passes ; à 0,5 ms, 2 refus sur 1 600 ouvertures.
useconds_t writePauseUs() {
    const auto* value = std::getenv("RO_WRITE_PAUSE_US");
    return value ? static_cast<useconds_t>(std::stoul(value)) : 200;
}

// Une ouverture en lecture seule suivie d'un compte. Rend le message d'erreur, vide si
// tout a réussi.
std::string openAndCount(const std::string& path, const rag3db::main::SystemConfig& base) {
    try {
        auto config = base;
        config.readOnly = true;
        rag3db::main::Database database(path, config);
        rag3db::main::Connection connection(&database);
        auto result = connection.query("MATCH (t:Work) RETURN count(t);");
        if (!result->isSuccess()) {
            return "query: " + result->getErrorMessage();
        }
        return "";
    } catch (const std::exception& e) {
        return e.what();
    }
}

// La patience de Rag3dbConnection::read_only_patient.
struct PatientOpen {
    std::string error; // vide si l'ouverture a fini par réussir
    int attempts = 0;
};

PatientOpen openPatiently(const std::string& path, const rag3db::main::SystemConfig& base) {
    const auto start = std::chrono::steady_clock::now();
    auto wait = std::chrono::milliseconds(5);
    PatientOpen open;
    while (true) {
        open.attempts++;
        open.error = openAndCount(path, base);
        if (open.error.empty()) {
            return open;
        }
        const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
            std::chrono::steady_clock::now() - start)
                                 .count();
        if (static_cast<uint64_t>(elapsed + wait.count()) > PATIENCE_MS) {
            return open;
        }
        std::this_thread::sleep_for(wait);
        wait = std::min(wait * 2, std::chrono::milliseconds(40));
    }
}

} // namespace

class ReadOnlyReader : public EmptyDBTest {
public:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
    }
};

TEST_F(ReadOnlyReader, InsistingReaderIsNotStarvedByCheckpoints) {
    conn.reset();
    database.reset();
    auto* report = shareReport();
    const auto path = databasePath;
    const auto baseConfig = *systemConfig;

    const auto pause = writePauseUs();
    const auto pid = fork();
    ASSERT_NE(pid, -1);
    if (pid == 0) {
        try {
            rag3db::main::Database database(path, baseConfig);
            rag3db::main::Connection connection(&database);
            connection.query("CREATE NODE TABLE Work(id INT64, status STRING, PRIMARY KEY(id));");
            report->ready = 1;
            for (int i = 0; !report->stopRequested; i++) {
                auto result = connection.query(
                    "CREATE (:Work {id: " + std::to_string(i) + ", status: 'free'});");
                if (result->isSuccess()) {
                    report->rowsWritten = i + 1;
                }
                if ((i + 1) % 5 == 0) {
                    connection.query("CHECKPOINT;");
                }
                // Le rythme d'un écrivain rag3weaver dans le test d'origine : quelques
                // centaines d'écritures par seconde, pas des milliers.
                usleep(pause);
            }
        } catch (const std::exception&) {
            report->failed = 1;
        }
        _exit(0);
    }
    for (int i = 0; i < 30000 && !report->ready && !report->failed; i++) {
        usleep(1000);
    }
    ASSERT_EQ(report->failed, 0);
    ASSERT_EQ(report->ready, 1);

    const auto start = std::chrono::steady_clock::now();
    const auto rowsAtStart = report->rowsWritten;
    int refused = 0;
    int read = 0;
    std::map<std::string, int> reasons;
    for (int cycle = 0; cycle < CYCLES; cycle++) {
        const auto open = openPatiently(path, baseConfig);
        if (open.error.empty()) {
            read++;
        } else {
            refused++;
            reasons[open.error]++;
            std::cerr << "  refused after " << open.attempts << " attempts: " << open.error
                      << "\n";
        }
        usleep(2000);
    }
    const auto rowsDuring = report->rowsWritten - rowsAtStart;
    const auto elapsedMs = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::steady_clock::now() - start)
                               .count();
    report->stopRequested = 1;
    int status = 0;
    waitpid(pid, &status, 0);
    std::cerr << "  read " << read << ", refused " << refused << ", writes " << rowsDuring
              << " in " << elapsedMs << " ms (" << rowsDuring / 5 << " checkpoints)\n";
    createDBAndConn();

    std::string summary;
    for (const auto& [reason, count] : reasons) {
        summary += "\n    " + std::to_string(count) + "× " + reason;
    }
    EXPECT_EQ(refused, 0) << "[check: reader-not-starved] " << refused << " of " << CYCLES
                          << " read-only opens refused beyond " << PATIENCE_MS << " ms:"
                          << summary;
}
