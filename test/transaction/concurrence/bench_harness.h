#pragma once

// Le harnais du banc de concurrence (marche A1,
// docs/2-octobre-2026-00h36/01-specification-du-banc-de-concurrence.md).
//
// Un scénario s'écrit une fois, pour un écrivain : il reçoit un Worker qui porte sa
// connexion, son numéro et la barrière commune. Le lanceur décide si les écrivains
// sont des fils ou des processus. Tout l'état partagé (barrière, comptes rendus) vit
// dans une projection MAP_SHARED anonyme, pour que le même code serve aux deux.
// En mode Processus, chaque écrivain est un processus créé par fork() qui ouvre sa
// propre Database : le père ne doit en tenir aucune ouverte à ce moment-là.

#include <sys/mman.h>
#include <sys/wait.h>
#include <unistd.h>

#include <array>
#include <atomic>
#include <chrono>
#include <cstring>
#include <functional>
#include <memory>
#include <new>
#include <stdexcept>
#include <string>
#include <string_view>
#include <thread>
#include <vector>

#include "main/connection.h"
#include "main/database.h"

namespace rag3db {
namespace testing {
namespace concurrency {

enum class LaunchMode { Thread, Process };

inline std::string launchModeName(LaunchMode mode) {
    switch (mode) {
    case LaunchMode::Thread:
        return "Thread";
    case LaunchMode::Process:
        return "Process";
    }
    return "?";
}

// Les refus qu'un écrivain peut recevoir, classés par leur message. Un refus n'est
// jamais compté sans que son message ait été lu.
enum class Refusal : uint8_t {
    WriteWriteConflict,
    DuplicatePrimaryKey,
    ConnectedEdges,
    WriterRefused,
    CheckpointTimeout,
    FileLock,
    Unexpected,
};
constexpr size_t NUM_REFUSALS = 7;

inline std::string_view refusalName(Refusal refusal) {
    switch (refusal) {
    case Refusal::WriteWriteConflict:
        return "write-write conflict";
    case Refusal::DuplicatePrimaryKey:
        return "duplicated primary key";
    case Refusal::ConnectedEdges:
        return "connected edges";
    case Refusal::WriterRefused:
        return "second writer refused";
    case Refusal::CheckpointTimeout:
        return "checkpoint timeout";
    case Refusal::FileLock:
        return "file lock";
    case Refusal::Unexpected:
        return "UNEXPECTED";
    }
    return "?";
}

inline Refusal classifyRefusal(std::string_view message) {
    if (message.find("Write-write conflict") != std::string_view::npos) {
        return Refusal::WriteWriteConflict;
    }
    if (message.find("duplicated primary key") != std::string_view::npos) {
        return Refusal::DuplicatePrimaryKey;
    }
    if (message.find("has connected edges") != std::string_view::npos) {
        return Refusal::ConnectedEdges;
    }
    if (message.find("Only one write transaction at a time") != std::string_view::npos) {
        return Refusal::WriterRefused;
    }
    if (message.find("Timeout waiting for active transactions") != std::string_view::npos) {
        return Refusal::CheckpointTimeout;
    }
    if (message.find("Could not set lock on file") != std::string_view::npos) {
        return Refusal::FileLock;
    }
    return Refusal::Unexpected;
}

constexpr uint32_t MAX_WORKERS = 16;
constexpr size_t MESSAGE_CAPACITY = 512;

// Entre processus, une atomique n'est sûre que si elle n'a pas de verrou caché.
static_assert(std::atomic<uint32_t>::is_always_lock_free);

// Ce qu'un écrivain rapporte. Des atomiques et des tableaux fixes seulement : la
// structure doit pouvoir vivre dans une page partagée entre processus.
struct WorkerReport {
    std::atomic<uint32_t> statementsSucceeded{0};
    std::atomic<uint32_t> commitsSucceeded{0};
    std::array<std::atomic<uint32_t>, NUM_REFUSALS> refusals{};
    char firstUnexpected[MESSAGE_CAPACITY]{};
};

struct SharedArea {
    std::atomic<uint32_t> barrierArrived{0};
    std::atomic<uint32_t> barrierGeneration{0};
    std::atomic<uint32_t> barrierTimedOut{0};
    // Un drapeau libre pour les scénarios (C8 : les écrivains ont fini).
    std::atomic<uint32_t> flag{0};
    uint32_t numWorkers = 0;
    std::array<WorkerReport, MAX_WORKERS> workers{};
};

// La zone partagée, projetée anonyme et MAP_SHARED : visible des fils comme des
// processus créés après elle.
class SharedMapping {
public:
    explicit SharedMapping(uint32_t numWorkers) {
        if (numWorkers > MAX_WORKERS) {
            throw std::runtime_error("too many workers for the shared area");
        }
        memory = mmap(nullptr, sizeof(SharedArea), PROT_READ | PROT_WRITE,
            MAP_ANONYMOUS | MAP_SHARED, -1, 0);
        if (memory == MAP_FAILED) {
            throw std::runtime_error("mmap of the shared area failed");
        }
        area = new (memory) SharedArea();
        area->numWorkers = numWorkers;
    }
    ~SharedMapping() {
        area->~SharedArea();
        munmap(memory, sizeof(SharedArea));
    }
    SharedMapping(const SharedMapping&) = delete;
    SharedMapping& operator=(const SharedMapping&) = delete;

    SharedArea& get() { return *area; }

private:
    void* memory = nullptr;
    SharedArea* area = nullptr;
};

// Un écrivain vu par son scénario.
class Worker {
public:
    // connection est nul quand l'écrivain n'a pas pu ouvrir la base (mode Processus) :
    // il participe encore aux barrières, mais toute instruction échoue.
    Worker(uint32_t index, main::Connection* connection, SharedArea& area)
        : workerIndex{index}, connection{connection}, area{area}, report{area.workers[index]} {}

    uint32_t index() const { return workerIndex; }
    uint32_t numWorkers() const { return area.numWorkers; }
    bool connected() const { return connection != nullptr; }
    SharedArea& shared() { return area; }

    // Exécute une instruction ; en cas d'échec, lit et classe le message. Un échec
    // dans une transaction explicite la tient pour perdue : commit() fera ROLLBACK.
    bool run(const std::string& query) {
        if (connection == nullptr) {
            recordFailure("no connection: the database could not be opened");
            return false;
        }
        auto result = connection->query(query);
        if (result->isSuccess()) {
            report.statementsSucceeded.fetch_add(1);
            return true;
        }
        recordFailure(result->getErrorMessage());
        if (inTransaction) {
            transactionFailed = true;
        }
        return false;
    }

    bool begin() {
        transactionFailed = false;
        inTransaction = run("BEGIN TRANSACTION;");
        return inTransaction;
    }

    // Annule volontairement la transaction ouverte. Si une instruction y a échoué, le
    // moteur l'a déjà annulée : le ROLLBACK est envoyé sans être compté, comme dans
    // commit().
    void rollback() {
        if (!inTransaction) {
            return;
        }
        inTransaction = false;
        if (transactionFailed) {
            if (connection != nullptr) {
                connection->query("ROLLBACK;");
            }
            return;
        }
        run("ROLLBACK;");
    }

    // Valide la transaction ouverte. Si une instruction y a échoué, on ne valide pas :
    // on annule, et l'éventuel refus de ROLLBACK (transaction déjà annulée par le
    // moteur) n'est pas compté, il ne dit rien du conflit observé.
    bool commit() {
        if (!inTransaction) {
            return false;
        }
        inTransaction = false;
        if (transactionFailed) {
            if (connection != nullptr) {
                connection->query("ROLLBACK;");
            }
            return false;
        }
        if (run("COMMIT;")) {
            report.commitsSucceeded.fetch_add(1);
            return true;
        }
        return false;
    }

    // Les écrivains valident l'un après l'autre, dans l'ordre donné (numéros
    // d'écrivains), une barrière entre chaque : l'ordre des commits est ainsi fixé.
    void commitInOrder(const std::vector<uint32_t>& order) {
        for (auto turn : order) {
            if (turn == workerIndex) {
                commit();
            }
            sync();
        }
    }

    // Barrière commune à tous les écrivains. Attente active : std::atomic::wait
    // n'est pas garanti entre processus. Au-delà de 30 s, la barrière est déclarée
    // rompue et tout le monde passe, pour qu'un écrivain bloqué ne fige pas la passe.
    void sync() {
        using namespace std::chrono;
        const auto generation = area.barrierGeneration.load(std::memory_order_acquire);
        if (area.barrierArrived.fetch_add(1, std::memory_order_acq_rel) + 1 == area.numWorkers) {
            area.barrierArrived.store(0, std::memory_order_relaxed);
            area.barrierGeneration.fetch_add(1, std::memory_order_release);
            return;
        }
        const auto deadline = steady_clock::now() + seconds(30);
        while (area.barrierGeneration.load(std::memory_order_acquire) == generation) {
            if (area.barrierTimedOut.load() != 0 || steady_clock::now() > deadline) {
                area.barrierTimedOut.store(1);
                return;
            }
            std::this_thread::yield();
        }
    }

    void recordFailure(const std::string& message) {
        const auto refusal = classifyRefusal(message);
        if (refusal == Refusal::Unexpected &&
            report.refusals[static_cast<size_t>(Refusal::Unexpected)].load() == 0) {
            std::strncpy(report.firstUnexpected, message.c_str(), MESSAGE_CAPACITY - 1);
        }
        report.refusals[static_cast<size_t>(refusal)].fetch_add(1);
    }

private:
    uint32_t workerIndex;
    main::Connection* connection;
    SharedArea& area;
    WorkerReport& report;
    bool inTransaction = false;
    bool transactionFailed = false;
};

using Scenario = std::function<void(Worker&)>;

// Ce qu'il faut pour donner une base à un écrivain : la Database commune en mode Fil,
// le chemin et la configuration pour l'ouvrir soi-même en mode Processus.
struct Opener {
    main::Database* shared = nullptr;
    std::string path;
    main::SystemConfig config;
};

// Les réglages du banc sur une Database qu'on vient d'ouvrir (voir l'en-tête du banc).
// Le premier n'est pas persistant : il se repose à chaque ouverture.
inline void applyBenchSettings(main::Connection& connection) {
    for (const auto* query :
        {"CALL debug_enable_multi_writes=true;", "CALL auto_checkpoint=false;"}) {
        auto result = connection.query(query);
        if (!result->isSuccess()) {
            throw std::runtime_error(std::string(query) + ": " + result->getErrorMessage());
        }
    }
}

// Lance area.numWorkers écrivains sur la même base et attend qu'ils aient fini.
// Rend faux si un processus écrivain n'est pas sorti normalement.
inline bool launch(LaunchMode mode, const Opener& opener, SharedArea& area,
    const Scenario& scenario) {
    switch (mode) {
    case LaunchMode::Thread: {
        std::vector<std::thread> threads;
        for (auto i = 0u; i < area.numWorkers; ++i) {
            threads.emplace_back([&, i] {
                main::Connection connection(opener.shared);
                Worker worker(i, &connection, area);
                scenario(worker);
            });
        }
        for (auto& thread : threads) {
            thread.join();
        }
        return true;
    }
    case LaunchMode::Process: {
        std::vector<pid_t> children;
        for (auto i = 0u; i < area.numWorkers; ++i) {
            const auto pid = fork();
            if (pid == 0) {
                std::unique_ptr<main::Database> database;
                std::unique_ptr<main::Connection> connection;
                std::string openError;
                try {
                    database = std::make_unique<main::Database>(opener.path, opener.config);
                    connection = std::make_unique<main::Connection>(database.get());
                    applyBenchSettings(*connection);
                } catch (const std::exception& e) {
                    openError = e.what();
                    connection.reset();
                    database.reset();
                }
                Worker worker(i, connection.get(), area);
                if (!openError.empty()) {
                    worker.recordFailure(openError);
                }
                scenario(worker);
                connection.reset();
                database.reset();
                _exit(0);
            }
            children.push_back(pid);
        }
        bool allExited = true;
        for (const auto pid : children) {
            int status = 0;
            waitpid(pid, &status, 0);
            allExited = allExited && WIFEXITED(status) && WEXITSTATUS(status) == 0;
        }
        return allExited;
    }
    }
    return false;
}

inline uint32_t totalCommits(const SharedArea& area) {
    uint32_t total = 0;
    for (auto i = 0u; i < area.numWorkers; ++i) {
        total += area.workers[i].commitsSucceeded.load();
    }
    return total;
}

inline uint32_t totalStatements(const SharedArea& area) {
    uint32_t total = 0;
    for (auto i = 0u; i < area.numWorkers; ++i) {
        total += area.workers[i].statementsSucceeded.load();
    }
    return total;
}

inline uint32_t totalRefusals(const SharedArea& area, Refusal refusal) {
    uint32_t total = 0;
    for (auto i = 0u; i < area.numWorkers; ++i) {
        total += area.workers[i].refusals[static_cast<size_t>(refusal)].load();
    }
    return total;
}

// Le compte rendu brut, écrit sur la sortie d'erreur pour qu'un rouge garde sa trace.
inline std::string describe(const SharedArea& area) {
    std::string out;
    for (auto i = 0u; i < area.numWorkers; ++i) {
        const auto& report = area.workers[i];
        out += "  writer " + std::to_string(i) +
               ": statements ok=" + std::to_string(report.statementsSucceeded.load()) +
               ", commits ok=" + std::to_string(report.commitsSucceeded.load());
        for (auto r = 0u; r < NUM_REFUSALS; ++r) {
            if (const auto count = report.refusals[r].load(); count != 0) {
                out += ", " + std::string(refusalName(static_cast<Refusal>(r))) + "=" +
                       std::to_string(count);
            }
        }
        if (report.firstUnexpected[0] != '\0') {
            out += "\n    first unexpected: " + std::string(report.firstUnexpected);
        }
        out += "\n";
    }
    if (area.barrierTimedOut.load() != 0) {
        out += "  BARRIER TIMED OUT\n";
    }
    return out;
}

} // namespace concurrency
} // namespace testing
} // namespace rag3db
