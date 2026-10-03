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

#include <signal.h>
#include <sys/mman.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>

#include <algorithm>
#include <array>
#include <atomic>
#include <cctype>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <functional>
#include <initializer_list>
#include <memory>
#include <mutex>
#include <new>
#include <stdexcept>
#include <string>
#include <string_view>
#include <thread>
#include <vector>

#include "main/connection.h"
#include "main/database.h"
#include "test_helper/test_helper.h"

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
    Interrupted,
    // Les erreurs des verrous à venir (note de conception, §2), reconnues par les
    // fragments annoncés à la session cœur C++ le 3 octobre : tant qu'elles n'existent
    // pas, aucun cas ne les reçoit.
    Deadlock,
    LockTimeout,
    SerializationFailure,
    // Une fonction ou un réglage que le moteur ne connaît pas encore (acquire_locks,
    // lock_timeout) : « rouge, la fonction n'existe pas », distinct d'un mauvais
    // comportement.
    MissingFunction,
    Unexpected,
};
constexpr size_t NUM_REFUSALS = 12;

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
    case Refusal::Interrupted:
        return "interrupted";
    case Refusal::Deadlock:
        return "deadlock";
    case Refusal::LockTimeout:
        return "lock timeout";
    case Refusal::SerializationFailure:
        return "serialization failure";
    case Refusal::MissingFunction:
        return "missing function";
    case Refusal::Unexpected:
        return "UNEXPECTED";
    }
    return "?";
}

inline bool containsIgnoringCase(std::string_view text, std::string_view fragment) {
    return std::search(text.begin(), text.end(), fragment.begin(), fragment.end(),
               [](char a, char b) { return std::tolower(a) == std::tolower(b); }) != text.end();
}

inline Refusal classifyRefusal(std::string_view message) {
    // D'abord : une fonction ou un réglage inconnu. Son message porte le nom demandé
    // (« Invalid option name: lock_timeout ») et ne doit pas passer pour l'erreur qu'il
    // nomme. Messages relevés le 3 octobre.
    if (containsIgnoringCase(message, "Invalid option name") ||
        (containsIgnoringCase(message, "function") &&
            containsIgnoringCase(message, "does not exist")) ||
        containsIgnoringCase(message, "is not defined")) {
        return Refusal::MissingFunction;
    }
    if (containsIgnoringCase(message, "deadlock")) {
        return Refusal::Deadlock;
    }
    if (containsIgnoringCase(message, "lock") && containsIgnoringCase(message, "timeout") &&
        !containsIgnoringCase(message, "Timeout waiting for active transactions")) {
        return Refusal::LockTimeout;
    }
    if (containsIgnoringCase(message, "could not serialize")) {
        return Refusal::SerializationFailure;
    }
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
    if (message.find("Interrupted") != std::string_view::npos) {
        return Refusal::Interrupted;
    }
    return Refusal::Unexpected;
}

constexpr uint32_t MAX_WORKERS = 16;
constexpr size_t MESSAGE_CAPACITY = 512;
constexpr uint32_t MAX_EVENTS = 256;
constexpr size_t LABEL_CAPACITY = 48;

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

// Un événement daté, écrit par un écrivain (Worker::mark). L'ordre des index est
// l'ordre réel des marques ; l'horloge est monotone et commune aux processus d'une
// même machine. ready passe à 1 quand l'événement est entièrement écrit.
struct EventRecord {
    std::atomic<uint32_t> ready{0};
    uint32_t worker = 0;
    int64_t micros = 0;
    char label[LABEL_CAPACITY]{};
};

inline int64_t monotonicMicros() {
    return std::chrono::duration_cast<std::chrono::microseconds>(
        std::chrono::steady_clock::now().time_since_epoch())
        .count();
}

struct SharedArea {
    std::atomic<uint32_t> barrierArrived{0};
    std::atomic<uint32_t> barrierGeneration{0};
    std::atomic<uint32_t> barrierTimedOut{0};
    // Un drapeau libre pour les scénarios (C8 : les écrivains ont fini).
    std::atomic<uint32_t> flag{0};
    // Le délai de garde du cas a expiré : le moteur a bloqué un écrivain trop longtemps.
    std::atomic<uint32_t> guardExpired{0};
    std::atomic<uint32_t> numEvents{0};
    std::array<EventRecord, MAX_EVENTS> events{};
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
        // Après l'échec d'une instruction, le moteur a déjà annulé la transaction : une
        // instruction de plus partirait en auto-commit et serait validée seule. Elle
        // n'est donc pas envoyée. (Exécuté le 3 octobre : c'est ce qui faisait monter la
        // somme de C4 à l'étape 2 — un défaut du banc, pas du moteur.)
        if (inTransaction && transactionFailed) {
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

    // Marque un événement : c'est ce qui permet de prouver après coup qu'un écrivain a
    // attendu (sa fin d'instruction vient après la fin de celui qu'il attendait) ou
    // qu'il n'a pas attendu.
    void mark(std::string_view label) {
        const auto index = area.numEvents.fetch_add(1);
        if (index >= MAX_EVENTS) {
            return;
        }
        auto& event = area.events[index];
        event.worker = workerIndex;
        event.micros = monotonicMicros();
        const auto length = std::min(label.size(), LABEL_CAPACITY - 1);
        std::memcpy(event.label, label.data(), length);
        event.label[length] = '\0';
        event.ready.store(1, std::memory_order_release);
    }

    // Attend qu'un autre écrivain ait marqué un événement. Remplace la barrière quand
    // l'autre peut être bloqué par le moteur et ne jamais l'atteindre. Rend faux au
    // délai, sans bloquer la passe.
    bool waitFor(uint32_t worker, std::string_view label, std::chrono::milliseconds timeout) {
        return waitForAny(worker, {label}, timeout);
    }

    // Attend qu'un autre écrivain ait marqué l'un des événements donnés.
    bool waitForAny(uint32_t worker, std::initializer_list<std::string_view> labels,
        std::chrono::milliseconds timeout) {
        const auto deadline = std::chrono::steady_clock::now() + timeout;
        while (std::chrono::steady_clock::now() < deadline) {
            for (const auto label : labels) {
                if (eventIndexOf(worker, label) >= 0) {
                    return true;
                }
            }
            std::this_thread::yield();
        }
        return false;
    }

    // L'index d'un événement déjà entièrement marqué, ou -1.
    int eventIndexOf(uint32_t worker, std::string_view label) const {
        const auto count = std::min(area.numEvents.load(), MAX_EVENTS);
        for (auto i = 0u; i < count; ++i) {
            const auto& event = area.events[i];
            if (event.ready.load(std::memory_order_acquire) != 0 && event.worker == worker &&
                label == event.label) {
                return static_cast<int>(i);
            }
        }
        return -1;
    }

    // Valide, encadré par « <label>:start » et « <label>:done » ou « <label>:failed ».
    bool commitMarked(const std::string& label) {
        mark(label + ":start");
        const bool succeeded = commit();
        mark(label + (succeeded ? ":done" : ":failed"));
        return succeeded;
    }

    // Écrit à son tour : l'écrivain k attend la fin de l'écriture de l'écrivain k - 1,
    // puis écrit (runMarked(…, "write")). Les transactions ont toutes écrit avant le
    // premier commit — l'anomalie logique est la même — mais deux écritures ne touchent
    // jamais les informations de version en même temps : la course physique du chemin
    // de suppression sans verrou fait planter le processus (SIGSEGV dans
    // VersionInfo::isDeleted et isSelected, 3 octobre), et elle a ses propres cas (C5,
    // C7, la passe ThreadSanitizer). Le jour où le moteur fera attendre un écrivain sur
    // un verrou, l'écriture de k bloquera après celle de k - 1, ce qui ne change rien
    // ici.
    bool writeInTurn(const std::string& query) {
        constexpr std::chrono::milliseconds PREVIOUS_WRITE{30'000};
        if (workerIndex > 0) {
            waitForAny(workerIndex - 1, {"write:done", "write:failed"}, PREVIOUS_WRITE);
        }
        return runMarked(query, "write");
    }

    // Les commits dans l'ordre donné, réglés par les événements et non par une barrière,
    // pour les cas où chaque écrivain a écrit par runMarked(…, "write") dans sa
    // transaction. Chacun, à son tour : attend la fin de l'écriture des autres, au plus
    // WRITE_SETTLE — aujourd'hui elle arrive aussitôt, et les transactions ont donc
    // toutes écrit avant le premier commit ; le jour où le moteur fera attendre le
    // second écrivain sur un verrou (décision de Lucie, 2 octobre), son écriture ne
    // finira qu'après le commit du premier, le délai expirera et le premier validera,
    // là où une barrière aurait bloqué les deux. Puis il attend que le précédent dans
    // l'ordre ait fini de valider, et valide.
    void commitInOrderByEvents(const std::vector<uint32_t>& order) {
        constexpr std::chrono::milliseconds WRITE_SETTLE{2'000};
        constexpr std::chrono::milliseconds PREVIOUS_COMMIT{30'000};
        for (auto position = 0u; position < order.size(); ++position) {
            if (order[position] != workerIndex) {
                continue;
            }
            for (auto other = 0u; other < area.numWorkers; ++other) {
                if (other != workerIndex) {
                    waitForAny(other, {"write:done", "write:failed"}, WRITE_SETTLE);
                }
            }
            if (position > 0) {
                waitForAny(order[position - 1], {"commit:done", "commit:failed"}, PREVIOUS_COMMIT);
            }
            commitMarked("commit");
        }
    }

    // Une instruction encadrée par deux marques : « <label>:start » puis
    // « <label>:done » ou « <label>:failed ».
    bool runMarked(const std::string& query, const std::string& label) {
        mark(label + ":start");
        const bool succeeded = run(query);
        mark(label + (succeeded ? ":done" : ":failed"));
        return succeeded;
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
    // La base porte un index vectoriel : l'extension se recharge à chaque ouverture.
    bool vectorExtension = false;
};

// Dans un processus fils du banc : pas de vidage mémoire. Un fils que le moteur fait
// planter (H1, H4) en écrivait un, ce qui coûtait 20 à 27 s par plantage (3 octobre) ;
// le banc lit le signal, le vidage n'apprend rien de plus.
inline void disableCoreDumps() {
    const rlimit none{0, 0};
    setrlimit(RLIMIT_CORE, &none);
}

// L'extension vector, bâtie dans l'arbre source (-DBUILD_EXTENSIONS=vector), comme la
// charge le runner des tests .test. Elle n'est pas persistante : sans elle, un index
// HNSW d'une base rouverte n'est plus interrogeable.
inline void loadVectorExtension(main::Connection& connection) {
    const auto path =
        TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
    auto result = connection.query("LOAD EXTENSION '" + path + "';");
    if (!result->isSuccess()) {
        throw std::runtime_error("LOAD EXTENSION vector: " + result->getErrorMessage());
    }
}

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

// Le délai de garde par défaut d'un cas : au-delà, le moteur est tenu pour bloqué.
constexpr std::chrono::milliseconds DEFAULT_GUARD{60'000};
// Après l'interruption des connexions, le temps laissé aux écrivains pour sortir avant
// que le processus de test ne s'arrête de lui-même, en rouge.
constexpr std::chrono::milliseconds GUARD_GRACE{5'000};

// Lance area.numWorkers écrivains sur la même base et attend qu'ils aient fini, au plus
// jusqu'au délai de garde. À l'échéance : area.guardExpired, puis les connexions sont
// interrompues (fils) ou les processus tués (processus). Si des fils restent bloqués
// malgré l'interruption, le processus de test s'arrête : un moteur qui se bloque doit
// donner un rouge, pas une passe qui ne finit pas.
// Rend faux si un processus écrivain n'est pas sorti normalement.
inline bool launch(LaunchMode mode, const Opener& opener, SharedArea& area,
    const Scenario& scenario, std::chrono::milliseconds guard = DEFAULT_GUARD) {
    const auto deadline = std::chrono::steady_clock::now() + guard;
    switch (mode) {
    case LaunchMode::Thread: {
        std::mutex connectionsMutex;
        std::vector<main::Connection*> connections;
        std::atomic<uint32_t> finished{0};
        std::vector<std::thread> threads;
        for (auto i = 0u; i < area.numWorkers; ++i) {
            threads.emplace_back([&, i] {
                main::Connection connection(opener.shared);
                {
                    std::lock_guard lock{connectionsMutex};
                    connections.push_back(&connection);
                }
                Worker worker(i, &connection, area);
                scenario(worker);
                {
                    std::lock_guard lock{connectionsMutex};
                    std::erase(connections, &connection);
                }
                finished.fetch_add(1);
            });
        }
        while (finished.load() < area.numWorkers && std::chrono::steady_clock::now() < deadline) {
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
        if (finished.load() < area.numWorkers) {
            area.guardExpired.store(1);
            {
                std::lock_guard lock{connectionsMutex};
                for (auto* connection : connections) {
                    connection->interrupt();
                }
            }
            const auto graceEnd = std::chrono::steady_clock::now() + GUARD_GRACE;
            while (
                finished.load() < area.numWorkers && std::chrono::steady_clock::now() < graceEnd) {
                std::this_thread::sleep_for(std::chrono::milliseconds(1));
            }
            if (finished.load() < area.numWorkers) {
                std::fprintf(stderr,
                    "[bench] GUARD: writers still blocked after interruption; stopping the "
                    "test process so that the pass ends red instead of hanging\n");
                std::fflush(stderr);
                std::_Exit(3);
            }
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
                disableCoreDumps();
                std::unique_ptr<main::Database> database;
                std::unique_ptr<main::Connection> connection;
                std::string openError;
                try {
                    database = std::make_unique<main::Database>(opener.path, opener.config);
                    connection = std::make_unique<main::Connection>(database.get());
                    applyBenchSettings(*connection);
                    if (opener.vectorExtension) {
                        loadVectorExtension(*connection);
                    }
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
        std::vector<bool> reaped(children.size(), false);
        auto remaining = children.size();
        while (remaining > 0) {
            for (auto c = 0u; c < children.size(); ++c) {
                if (reaped[c]) {
                    continue;
                }
                int status = 0;
                if (waitpid(children[c], &status, WNOHANG) == children[c]) {
                    reaped[c] = true;
                    --remaining;
                    allExited = allExited && WIFEXITED(status) && WEXITSTATUS(status) == 0;
                }
            }
            if (remaining > 0 && std::chrono::steady_clock::now() >= deadline &&
                area.guardExpired.load() == 0) {
                area.guardExpired.store(1);
                for (auto c = 0u; c < children.size(); ++c) {
                    if (!reaped[c]) {
                        kill(children[c], SIGKILL);
                    }
                }
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
        return allExited;
    }
    }
    return false;
}

// L'index d'un événement marqué, ou -1.
inline int eventIndex(const SharedArea& area, uint32_t worker, std::string_view label) {
    const auto count = std::min(area.numEvents.load(), MAX_EVENTS);
    for (auto i = 0u; i < count; ++i) {
        if (area.events[i].worker == worker && label == area.events[i].label) {
            return static_cast<int>(i);
        }
    }
    return -1;
}

// Les événements dans leur ordre réel, avec le temps écoulé depuis le premier.
inline std::string describeEvents(const SharedArea& area) {
    std::string out;
    const auto count = std::min(area.numEvents.load(), MAX_EVENTS);
    const auto origin = count > 0 ? area.events[0].micros : 0;
    for (auto i = 0u; i < count; ++i) {
        const auto& event = area.events[i];
        out += "  event #" + std::to_string(i) + " writer " + std::to_string(event.worker) + " +" +
               std::to_string(event.micros - origin) + "us " + event.label + "\n";
    }
    return out;
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
    if (area.guardExpired.load() != 0) {
        out += "  GUARD EXPIRED: the engine blocked a writer past the case's guard\n";
    }
    out += describeEvents(area);
    return out;
}

} // namespace concurrency
} // namespace testing
} // namespace rag3db
