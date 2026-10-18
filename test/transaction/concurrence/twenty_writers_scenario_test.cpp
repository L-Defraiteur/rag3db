// Vingt clients, un DDL par minute, un point de reprise en route, une migration qui échoue : le
// scénario des migrations concurrentes (banc, 13 octobre 2026 ; bornes arrêtées avec le cœur C++
// d'après sa lecture du code à 65cb9a1f3, page 15 de coeur-cpp ; forme finale au §6 de
// 13-octobre-2026-arbre-principal/01-plusieurs-developpeurs-et-leurs-migrations.md).
//
// Une base ouverte une fois, une connexion par fil : 20 écrivains de Doc (MERGE par la clé, SET de
// deux propriétés et du vecteur, une relation vers un voisin ; une fois sur dix dans un
// BEGIN … COMMIT), 2 écrivains de Note (une table que le DDL ne touche jamais), 4 lecteurs (un Doc
// par sa clé, puis ses voisins), et un DDL par période :
//   1. ALTER TABLE Doc ADD note STRING
//   2. ALTER TABLE Doc ADD rang INT64 DEFAULT 0
//   3. CREATE REL TABLE Cite(FROM Doc TO Doc)
//   4. CALL CREATE_VECTOR_INDEX('Doc', 'doc_vec', 'vec', metric := 'cosine')
//   5. CHECKPOINT
//   6. BEGIN ; ADD a ; ADD a (refusé) ; ROLLBACK, puis, avant tout point de reprise,
//      ALTER TABLE Doc ADD b STRING DEFAULT 'b-defaut', un SET de b et sa relecture.
//
// Le point de reprise automatique est en marche (le réglage du produit) jusqu'à la minute 6, coupé
// juste avant sa migration. La minute 6 et E10. Le ADD a annulé garde sa colonne au stockage
// (alter.cpp : addColumn à l'exécution, « rollback … handled by checkpoint »), le catalogue rendu
// redonne son identifiant de colonne N à b, et le stockage range b à l'indice N + 1. Le SET et le
// balayage passent tous deux par l'identifiant du catalogue : un SET de b puis sa relecture tombent
// dans la même colonne (celle de a) et se relisent tels qu'écrits. Ce qui montre le trou, c'est le
// DÉFAUT de b, rempli à l'indice N + 1 et lu à N : les lignes d'avant le ADD doivent lire
// 'b-defaut'. Le défaut de rang (minute 2, aucune annulation avant) est le témoin de contrôle. Puis
// la base est fermée proprement et rouverte : la valeur écrite et le défaut de b doivent survivre
// au point de reprise.
//
// Contrôles (rouge = un défaut ; une borne « pas promise » est imprimée seulement) :
// - no-unnamed-error : aucune erreur sans nom (message vide, exception de la bibliothèque standard)
// ;
// - create-rel-table-holds-writers-at-most-its-duration, checkpoint-holds-writers-at-most-its-
//   duration, index-creation-holds-writers-at-most-its-duration : la latence d'un écrivain qui
//   chevauche le DDL dépasse le p99 hors DDL de la même passe d'au plus la durée du DDL + 50 ms ;
// - index-creation-refuses-no-reader : aucune lecture refusée pendant CREATE_VECTOR_INDEX ;
// - failed-migration-is-refused-by-name, failed-migration-leaves-nothing,
//   failed-migration-disturbs-no-one : le second ADD a refusé sous un nom, pas de a au catalogue,
//   aucun acteur ne reçoit pendant la minute 6 une erreur jamais vue hors DDL ;
// - added-column-reads-its-default (rang et b), added-column-reads-what-was-set (b) : E10 ;
// - counts-match-commits : Doc, Link et Note égaux aux écritures validées, avant et après la
//   réouverture ;
// - catalog-as-expected, added-column-survives-reopen : après la réouverture.
// Les ADD COLUMN (minutes 1 et 2) n'ont pas de borne promise : leur durée et l'attente des
// écrivains de Doc et de Note sont imprimées (E1). Le p99 hors DDL est mesuré, pas promis.
//
// - TwentyWriters.DdlEveryFifteenSeconds : la version courte (environ 2 min avec le chargement) ;
// - TwentyWriters.DdlEveryMinute : la passe de six minutes ;
// - TwentyWriters.KilledDuringTheCheckpoint : un fils joue la version à 5 s jusqu'au CHECKPOINT de
//   la minute 5 et le père le tue par SIGKILL 20 ms après son départ ; la base rouverte doit porter
//   chaque écriture que le fils avait vue validée (son journal, écrit après chaque validation).
// Les trois sont dans long.txt. Les chiffres : une ligne « [scénario] » par fenêtre et par acteur.

#ifndef __SINGLE_THREADED__

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <algorithm>
#include <atomic>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <functional>
#include <iostream>
#include <map>
#include <mutex>
#include <optional>
#include <random>
#include <regex>
#include <set>
#include <sstream>
#include <string>
#include <thread>
#include <vector>

#include "bench_harness.h"
#include "common/build_id.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/buffer_manager/buffer_manager.h"
#include "storage/buffer_manager/memory_manager.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

using Clock = std::chrono::steady_clock;

constexpr int64_t NUM_DOCS = 50'000;
constexpr int64_t NUM_LINKS = 150'000;
// Les écrivains tirent leur clé dans [0, KEY_RANGE) : une sur six est neuve.
constexpr int64_t KEY_RANGE = NUM_DOCS * 6 / 5;
constexpr int DIM = 64;
constexpr int NUM_DOC_WRITERS = 20;
constexpr int NUM_NOTE_WRITERS = 2;
constexpr int NUM_READERS = 4;
constexpr double SLACK_MS = 50;
constexpr auto B_DEFAULT = "b-defaut";
constexpr int64_t NUM_B_SET = 10;
// Le tampon du produit, pas celui des tests (64 Mo) : sous 64 Mo plein, un BEGIN refusé laisse la
// connexion sans transaction et la requête suivante plante (ticket
// 2026-10-18-un-begin-refuse-laisse-la-connexion-sans-transaction, témoin failed_begin_test.cpp).
constexpr uint64_t BUFFER_POOL = 2ull * 1024 * 1024 * 1024;

enum class Actor { DocWriter, NoteWriter, Reader };

std::string actorName(Actor actor) {
    switch (actor) {
    case Actor::DocWriter:
        return "doc";
    case Actor::NoteWriter:
        return "note";
    default:
        return "lecteur";
    }
}

// Une transaction d'un acteur, en millisecondes depuis le début du scénario ; error vaut -1 si elle
// a validé, l'indice de son message rangé sinon.
struct Sample {
    double start;
    double end;
    int error;
};

struct Ddl {
    std::string name;
    double start = 0;
    double end = 0;
    bool ok = false;
    std::string error;
};

// Ce qu'un fil a vu ; lu après la fin des fils.
struct Journal {
    Actor actor = Actor::DocWriter;
    std::vector<Sample> samples;
    int64_t commits = 0;
    std::set<int64_t> newKeys;
};

// Les messages d'erreur, chiffres effacés, rangés une fois pour tous les fils.
class ErrorCatalog {
public:
    int add(const std::string& message) {
        static const std::regex digits("[0-9]+");
        auto key = std::regex_replace(message.substr(0, 200), digits, "#");
        std::lock_guard lock{mutex};
        const auto [it, inserted] = indexOf.emplace(key, static_cast<int>(messages.size()));
        if (inserted) {
            messages.push_back(key);
        }
        return it->second;
    }
    const std::string& at(int index) const { return messages.at(index); }
    size_t size() const { return messages.size(); }

private:
    std::mutex mutex;
    std::map<std::string, int> indexOf;
    std::vector<std::string> messages;
};

// Une erreur sans nom : un message vide, ou celui d'une exception de la bibliothèque standard.
bool isUnnamed(const std::string& message) {
    if (message.find_first_not_of(" \n\t") == std::string::npos) {
        return true;
    }
    for (const auto* fragment : {"basic_string", "vector::", "map::at", "_M_", "bad_alloc",
             "bad_function_call", "unordered_map", "std::"}) {
        if (message.find(fragment) != std::string::npos) {
            return true;
        }
    }
    return false;
}

std::string vectorLiteral(std::mt19937& engine) {
    std::uniform_real_distribution<float> uniform(-1, 1);
    std::ostringstream out;
    out.precision(3);
    out << "[";
    for (auto d = 0; d < DIM; ++d) {
        out << (d ? "," : "") << uniform(engine);
    }
    out << "]";
    return out.str();
}

int64_t countOf(rag3db::main::Connection& connection, const std::string& query) {
    auto result = connection.query(query);
    if (!result->isSuccess() || !result->hasNext()) {
        throw std::runtime_error(query + " : " + result->getErrorMessage());
    }
    return result->getNext()->getValue(0)->getValue<int64_t>();
}

std::set<std::string> namesOf(rag3db::main::Connection& connection, const std::string& query) {
    auto result = connection.query(query);
    if (!result->isSuccess()) {
        throw std::runtime_error(query + " : " + result->getErrorMessage());
    }
    std::set<std::string> out;
    while (result->hasNext()) {
        out.insert(result->getNext()->getValue(0)->toString());
    }
    return out;
}

void mustRun(rag3db::main::Connection& connection, const std::string& query) {
    auto result = connection.query(query);
    if (!result->isSuccess()) {
        throw std::runtime_error(query.substr(0, 160) + " : " + result->getErrorMessage());
    }
}

// Doc et Note, 50 000 Doc au vecteur NULL et 150 000 relations par COPY, puis un point de reprise.
void load(rag3db::main::Connection& connection, const std::string& databasePath) {
    concurrency::applyBenchSettings(connection);
    concurrency::loadVectorExtension(connection);
    mustRun(connection, stringFormat("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, titre STRING, "
                                     "corps STRING, vec FLOAT[{}]);",
                            DIM));
    mustRun(connection, "CREATE NODE TABLE Note(id INT64 PRIMARY KEY, texte STRING);");
    mustRun(connection, "CREATE REL TABLE Link(FROM Doc TO Doc);");
    const auto docs = databasePath + ".docs.csv";
    const auto links = databasePath + ".links.csv";
    {
        std::ofstream out(docs);
        for (int64_t id = 0; id < NUM_DOCS; ++id) {
            out << id << ",titre " << id << ",corps " << id << ",\n";
        }
    }
    {
        std::ofstream out(links);
        std::mt19937 engine(11);
        std::uniform_int_distribution<int64_t> any(0, NUM_DOCS - 1);
        for (int64_t i = 0; i < NUM_LINKS; ++i) {
            out << i % NUM_DOCS << "," << any(engine) << "\n";
        }
    }
    mustRun(connection, "COPY Doc FROM '" + docs + "' (header=false);");
    mustRun(connection, "COPY Link FROM '" + links + "' (header=false);");
    std::filesystem::remove(docs);
    std::filesystem::remove(links);
    mustRun(connection, "CHECKPOINT;");
}

// Le journal des validations d'un fils qui va être tué : une ligne écrite (write(2), sans fsync :
// un SIGKILL ne perd pas le cache de pages) après chaque validation vue.
class CommitLog {
public:
    explicit CommitLog(const std::string& path) : file{std::fopen(path.c_str(), "w")} {}
    ~CommitLog() {
        if (file) {
            std::fclose(file);
        }
    }
    void write(const std::string& line) {
        std::lock_guard lock{mutex};
        std::fputs((line + "\n").c_str(), file);
        std::fflush(file);
    }

private:
    std::mutex mutex;
    std::FILE* file;
};

class Scenario {
public:
    Scenario(rag3db::main::Database& database, std::chrono::milliseconds period,
        CommitLog* commitLog = nullptr)
        : database{database}, period{period}, commitLog{commitLog} {}

    // Les acteurs tournent pendant les DDL 1 à lastMinute, puis un quart de période ; onCheckpoint
    // est appelé juste avant le CHECKPOINT de la minute 5.
    void run(int lastMinute, const std::function<void()>& onCheckpoint = {}) {
        start = Clock::now();
        std::vector<std::thread> threads;
        journals.resize(NUM_DOC_WRITERS + NUM_NOTE_WRITERS + NUM_READERS);
        for (auto i = 0; i < static_cast<int>(journals.size()); ++i) {
            const auto actor = i < NUM_DOC_WRITERS                    ? Actor::DocWriter :
                               i < NUM_DOC_WRITERS + NUM_NOTE_WRITERS ? Actor::NoteWriter :
                                                                        Actor::Reader;
            journals[i].actor = actor;
            threads.emplace_back([this, i, actor] { actorLoop(i, actor); });
        }
        std::thread sampler([this] {
            auto* bufferManager = database.getMemoryManager()->getBufferManager();
            while (!stop) {
                bufferSamples.emplace_back(now(), bufferManager->getUsedMemory());
                std::this_thread::sleep_for(std::chrono::milliseconds(100));
            }
        });
        rag3db::main::Connection ddl(&database);
        for (auto minute = 1; minute <= lastMinute; ++minute) {
            std::this_thread::sleep_until(start + minute * period);
            switch (minute) {
            case 1:
                runDdl(ddl, "1 ADD note", "ALTER TABLE Doc ADD note STRING;");
                break;
            case 2:
                runDdl(ddl, "2 ADD rang DEFAULT 0", "ALTER TABLE Doc ADD rang INT64 DEFAULT 0;");
                break;
            case 3:
                runDdl(ddl, "3 CREATE REL TABLE", "CREATE REL TABLE Cite(FROM Doc TO Doc);");
                break;
            case 4:
                runDdl(ddl, "4 CREATE_VECTOR_INDEX",
                    "CALL CREATE_VECTOR_INDEX('Doc', 'doc_vec', 'vec', metric := 'cosine');");
                break;
            case 5:
                if (onCheckpoint) {
                    onCheckpoint();
                }
                runDdl(ddl, "5 CHECKPOINT", "CHECKPOINT;");
                break;
            case 6:
                ddl.query("CALL auto_checkpoint=false;");
                failedMigration(ddl);
                runDdl(ddl, "6 ADD b DEFAULT",
                    stringFormat("ALTER TABLE Doc ADD b STRING DEFAULT '{}';", B_DEFAULT));
                break;
            default:
                break;
            }
        }
        std::this_thread::sleep_for(period / 4);
        stop = true;
        for (auto& thread : threads) {
            thread.join();
        }
        sampler.join();
        end = now();
    }

    double now() const {
        return std::chrono::duration<double, std::milli>(Clock::now() - start).count();
    }

    std::vector<Journal> journals;
    std::vector<Ddl> ddls;
    ErrorCatalog errors;
    // La migration de la minute 6 : le message du second ADD a, celui du COMMIT qui suit.
    std::string secondAddError;
    std::string commitAfterFailureError;
    double end = 0;
    // Le tampon utilisé (octets), relevé toutes les 100 ms : (instant, octets).
    std::vector<std::pair<double, uint64_t>> bufferSamples;

private:
    void runDdl(rag3db::main::Connection& connection, const std::string& name,
        const std::string& query) {
        Ddl ddl;
        ddl.name = name;
        ddl.start = now();
        std::fprintf(stderr, "  [scénario] début de « %s » à %.0f ms\n", name.c_str(), ddl.start);
        auto result = connection.query(query);
        ddl.end = now();
        std::fprintf(stderr, "  [scénario] fin de « %s » à %.0f ms\n", name.c_str(), ddl.end);
        ddl.ok = result->isSuccess();
        ddl.error = ddl.ok ? "" : result->getErrorMessage();
        if (commitLog) {
            commitLog->write("DDL " + name.substr(0, 1) + " " + (ddl.ok ? "ok" : "refusé"));
        }
        ddls.push_back(ddl);
    }

    void failedMigration(rag3db::main::Connection& connection) {
        Ddl ddl;
        ddl.name = "6 migration annulée";
        ddl.start = now();
        auto begin = connection.query("BEGIN TRANSACTION;");
        auto first = connection.query("ALTER TABLE Doc ADD a STRING;");
        auto second = connection.query("ALTER TABLE Doc ADD a STRING;");
        // Après un échec dans le bloc, la connexion refuse tout jusqu'au ROLLBACK.
        auto commit = connection.query("COMMIT;");
        auto rollback = connection.query("ROLLBACK;");
        ddl.end = now();
        ddl.ok = begin->isSuccess() && first->isSuccess() && !second->isSuccess();
        ddl.error = ddl.ok ? "" :
                             "BEGIN " + begin->getErrorMessage() + " | ADD a " +
                                 first->getErrorMessage() + " | ADD a encore : " +
                                 (second->isSuccess() ? "accepté" : second->getErrorMessage());
        secondAddError = second->isSuccess() ? "" : second->getErrorMessage();
        commitAfterFailureError = commit->isSuccess() ? "" : commit->getErrorMessage();
        std::cerr << "  [scénario] migration annulée : second ADD « "
                  << secondAddError.substr(0, 160) << " » ; COMMIT « "
                  << (commit->isSuccess() ? "accepté" : commitAfterFailureError.substr(0, 160))
                  << " » ; ROLLBACK « "
                  << (rollback->isSuccess() ? "accepté" :
                                              rollback->getErrorMessage().substr(0, 160))
                  << " »\n";
        ddls.push_back(ddl);
    }

    // Une transaction, en automatique ou dans un bloc ; rend le message de l'échec, "" sinon.
    static std::string transact(rag3db::main::Connection& connection,
        const std::vector<std::string>& statements, bool block) {
        if (block) {
            auto begin = connection.query("BEGIN TRANSACTION;");
            if (!begin->isSuccess()) {
                // Imprimé tout de suite : la requête qui suit un BEGIN refusé peut tuer le
                // processus (ticket
                // 2026-10-18-un-begin-refuse-laisse-la-connexion-sans-transaction).
                std::fprintf(stderr, "  [scénario] BEGIN refusé : %s\n",
                    begin->getErrorMessage().substr(0, 200).c_str());
                return begin->getErrorMessage();
            }
        }
        for (const auto& statement : statements) {
            auto result = connection.query(statement);
            if (!result->isSuccess()) {
                if (block) {
                    connection.query("ROLLBACK;");
                }
                return result->getErrorMessage();
            }
            while (result->hasNext()) {
                result->getNext();
            }
        }
        if (block) {
            auto commit = connection.query("COMMIT;");
            if (!commit->isSuccess()) {
                connection.query("ROLLBACK;");
                return commit->getErrorMessage();
            }
        }
        return "";
    }

    void actorLoop(int index, Actor actor) {
        rag3db::main::Connection connection(&database);
        auto& journal = journals[index];
        std::mt19937 engine(static_cast<uint32_t>(1000 + index));
        std::uniform_int_distribution<int64_t> key(0, KEY_RANGE - 1);
        std::uniform_int_distribution<int64_t> neighbour(0, NUM_DOCS - 1);
        int64_t iteration = 0;
        while (!stop) {
            ++iteration;
            std::vector<std::string> statements;
            bool block = false;
            int64_t docKey = -1;
            switch (actor) {
            case Actor::DocWriter: {
                docKey = key(engine);
                const auto merge = stringFormat(
                    "MERGE (d:Doc {id: {}}) SET d.titre = 'titre {} {}', d.corps = 'corps {}', "
                    "d.vec = {}",
                    docKey, index, iteration, iteration, vectorLiteral(engine));
                const auto other = neighbour(engine);
                block = iteration % 10 == 0;
                if (block) {
                    statements = {merge + ";",
                        stringFormat("MATCH (d:Doc {id: {}}), (v:Doc {id: {}}) CREATE "
                                     "(d)-[:Link]->(v);",
                            docKey, other)};
                } else {
                    statements = {merge + stringFormat(" WITH d MATCH (v:Doc {id: {}}) CREATE "
                                                       "(d)-[:Link]->(v);",
                                              other)};
                }
            } break;
            case Actor::NoteWriter:
                block = iteration % 10 == 0;
                statements = {stringFormat("CREATE (:Note {id: {}, texte: 'note {}'});",
                    static_cast<int64_t>(index) * 1'000'000'000 + iteration, iteration)};
                break;
            default:
                statements = {stringFormat("MATCH (d:Doc {id: {}})-[:Link]->(v:Doc) RETURN "
                                           "d.titre, v.id;",
                    neighbour(engine))};
                break;
            }
            const auto started = now();
            const auto error = transact(connection, statements, block);
            const auto ended = now();
            if (error.empty()) {
                journal.samples.push_back({started, ended, -1});
                if (actor != Actor::Reader) {
                    journal.commits++;
                    if (docKey >= NUM_DOCS) {
                        journal.newKeys.insert(docKey);
                    }
                    if (commitLog) {
                        commitLog->write(actor == Actor::DocWriter ?
                                             "D " + std::to_string(docKey) :
                                             "N " + std::to_string(iteration));
                    }
                }
            } else {
                journal.samples.push_back({started, ended, errors.add(error)});
            }
        }
    }

    rag3db::main::Database& database;
    std::chrono::milliseconds period;
    CommitLog* commitLog;
    Clock::time_point start;
    std::atomic<bool> stop{false};
};

double percentile(std::vector<double> values, double fraction) {
    if (values.empty()) {
        return 0;
    }
    std::sort(values.begin(), values.end());
    const auto at = static_cast<size_t>(fraction * static_cast<double>(values.size() - 1) + 0.5);
    return values[std::min(at, values.size() - 1)];
}

class TwentyWriters : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed (checkpoint, reopening)";
        }
    }

    static bool overlaps(const Sample& sample, const Ddl& ddl) {
        return sample.start < ddl.end && sample.end > ddl.start;
    }

    // Les latences d'un acteur, chevauchant ou non un DDL ; ddl nul = hors de tout DDL. onlyThis :
    // celles qui ne chevauchent que ce DDL (deux DDL qui se suivent sans attendre, quand l'un dure
    // plus que la période, se partageraient sinon les mêmes attentes).
    static std::vector<double> latencies(const Scenario& scenario, Actor actor, const Ddl* ddl,
        bool onlyThis = false) {
        std::vector<double> out;
        for (const auto& journal : scenario.journals) {
            if (journal.actor != actor) {
                continue;
            }
            for (const auto& sample : journal.samples) {
                bool inAny = false;
                bool inOther = false;
                for (const auto& other : scenario.ddls) {
                    inAny = inAny || overlaps(sample, other);
                    inOther = inOther || (ddl && &other != ddl && overlaps(sample, other));
                }
                if (ddl ? overlaps(sample, *ddl) && !(onlyThis && inOther) : !inAny) {
                    out.push_back(sample.end - sample.start);
                }
            }
        }
        return out;
    }

    static std::map<int, int64_t> errorsOf(const Scenario& scenario, const Ddl* ddl,
        std::optional<Actor> actor = std::nullopt) {
        std::map<int, int64_t> out;
        for (const auto& journal : scenario.journals) {
            if (actor && journal.actor != *actor) {
                continue;
            }
            for (const auto& sample : journal.samples) {
                bool inAny = false;
                for (const auto& other : scenario.ddls) {
                    inAny = inAny || overlaps(sample, other);
                }
                if (sample.error >= 0 && (ddl ? overlaps(sample, *ddl) : !inAny)) {
                    out[sample.error]++;
                }
            }
        }
        return out;
    }

    // La fin de la première validation d'un écrivain après le début du DDL, moins ce début.
    static double firstCommitAfter(const Scenario& scenario, Actor actor, const Ddl& ddl) {
        double first = -1;
        for (const auto& journal : scenario.journals) {
            if (journal.actor != actor) {
                continue;
            }
            for (const auto& sample : journal.samples) {
                if (sample.error < 0 && sample.end > ddl.start &&
                    (first < 0 || sample.end < first)) {
                    first = sample.end;
                }
            }
        }
        return first < 0 ? -1 : first - ddl.start;
    }

    static void printWindow(const Scenario& scenario, const std::string& window, Actor actor,
        const Ddl* ddl) {
        const auto values = latencies(scenario, actor, ddl);
        int64_t numErrors = 0;
        for (const auto& [error, count] : errorsOf(scenario, ddl, actor)) {
            numErrors += count;
        }
        std::cerr << "  [scénario] fenêtre=« " << window << " » acteur=" << actorName(actor)
                  << " n=" << values.size() << " médiane=" << percentile(values, 0.5)
                  << " p99=" << percentile(values, 0.99) << " max="
                  << (values.empty() ? 0 : *std::max_element(values.begin(), values.end()))
                  << " ms erreurs=" << numErrors;
        if (ddl) {
            std::cerr << " durée_ddl=" << ddl->end - ddl->start
                      << " ms ddl=" << (ddl->ok ? "ok" : "refusé : " + ddl->error.substr(0, 120));
            if (actor != Actor::Reader) {
                std::cerr << " première_validation_après="
                          << firstCommitAfter(scenario, actor, *ddl) << " ms";
            }
        }
        std::cerr << " bati=" << RAG3DB_BUILD_ID << "\n";
    }

    const Ddl* ddlNamed(const Scenario& scenario, const std::string& prefix) {
        for (const auto& ddl : scenario.ddls) {
            if (ddl.name.starts_with(prefix)) {
                return &ddl;
            }
        }
        ADD_FAILURE() << "[check: plan] no DDL named " << prefix;
        return nullptr;
    }

    // Un écrivain qui chevauche le DDL n'attend pas plus que le DDL lui-même (+ 50 ms), au-delà de
    // ce qu'il attend hors DDL (son p99 de la même passe).
    void expectHeldAtMostItsDuration(const Scenario& scenario, const std::string& prefix,
        const std::string& label) {
        const auto* ddl = ddlNamed(scenario, prefix);
        if (!ddl) {
            return;
        }
        for (const auto actor : {Actor::DocWriter, Actor::NoteWriter}) {
            const auto during = latencies(scenario, actor, ddl, true /* onlyThis */);
            if (during.empty()) {
                continue;
            }
            const auto worst = *std::max_element(during.begin(), during.end());
            const auto usual = percentile(latencies(scenario, actor, nullptr), 0.99);
            EXPECT_LE(worst - usual, ddl->end - ddl->start + SLACK_MS)
                << "[check: " << label << "] " << actorName(actor) << " writer waited " << worst
                << " ms during « " << ddl->name << " » (" << ddl->end - ddl->start
                << " ms), against a p99 of " << usual << " ms outside the DDL";
        }
    }

    struct Expected {
        int64_t docs;
        int64_t links;
        int64_t notes;
    };

    static Expected expectedCounts(const Scenario& scenario) {
        Expected out{NUM_DOCS, NUM_LINKS, 0};
        std::set<int64_t> newKeys;
        for (const auto& journal : scenario.journals) {
            if (journal.actor == Actor::DocWriter) {
                out.links += journal.commits;
                newKeys.insert(journal.newKeys.begin(), journal.newKeys.end());
            } else if (journal.actor == Actor::NoteWriter) {
                out.notes += journal.commits;
            }
        }
        out.docs += static_cast<int64_t>(newKeys.size());
        return out;
    }

    void expectCounts(rag3db::main::Connection& connection, const Expected& expected,
        const std::string& moment) {
        const auto docs = countOf(connection, "MATCH (d:Doc) RETURN count(d);");
        const auto links = countOf(connection, "MATCH ()-[l:Link]->() RETURN count(l);");
        const auto notes = countOf(connection, "MATCH (n:Note) RETURN count(n);");
        std::cerr << "  [scénario] comptes " << moment << " : Doc " << docs << "/" << expected.docs
                  << ", Link " << links << "/" << expected.links << ", Note " << notes << "/"
                  << expected.notes << "\n";
        EXPECT_TRUE(docs == expected.docs && links == expected.links && notes == expected.notes)
            << "[check: counts-match-commits] " << moment << ": Doc " << docs << " for "
            << expected.docs << ", Link " << links << " for " << expected.links << ", Note "
            << notes << " for " << expected.notes;
    }

    // Les lignes d'avant le ADD dont la colonne ne lit pas son défaut.
    static int64_t notDefault(rag3db::main::Connection& connection, const std::string& column,
        const std::string& literal) {
        return countOf(connection,
            stringFormat("MATCH (d:Doc) WHERE d.id < {} AND d.id >= {} AND (d.{} IS NULL OR d.{} "
                         "<> {}) RETURN count(d);",
                NUM_DOCS, NUM_B_SET, column, column, literal));
    }

    static int64_t bAsSet(rag3db::main::Connection& connection) {
        return countOf(connection,
            stringFormat("MATCH (d:Doc) WHERE d.id < {} AND d.b = concat('b-ecrit-', CAST(d.id AS "
                         "STRING)) RETURN count(d);",
                NUM_B_SET));
    }

    void expectDefaults(rag3db::main::Connection& connection, const std::string& moment) {
        const auto rang = notDefault(connection, "rang", "0");
        const auto b = notDefault(connection, "b", stringFormat("'{}'", B_DEFAULT));
        std::cerr << "  [scénario] défauts " << moment << " : rang hors défaut " << rang
                  << ", b hors défaut " << b << " (sur " << NUM_DOCS - NUM_B_SET << ")\n";
        EXPECT_EQ(rang, 0) << "[check: added-column-reads-its-default] rang " << moment;
        EXPECT_EQ(b, 0) << "[check: added-column-reads-its-default] b " << moment;
    }

    void playScenario(std::chrono::milliseconds period) {
        systemConfig->bufferPoolSize = BUFFER_POOL;
        createDBAndConn();
        load(*conn, databasePath);
        // Le point de reprise automatique en marche, le réglage du produit ; le scénario le coupe
        // juste avant la migration de la minute 6, pour que rien ne s'intercale entre son
        // annulation et le ADD b. SCENARIO_NO_AUTO_CHECKPOINT=1 le coupe dès le départ : les pages
        // validées s'accumulent alors jusqu'au CHECKPOINT de la minute 5, et 20 écrivains de
        // vecteurs remplissent 2 Gio avant (joué le 18 octobre : 58 598 refus du tampon, le
        // CREATE_VECTOR_INDEX et le CHECKPOINT refusés).
        const bool autoCheckpoint = std::getenv("SCENARIO_NO_AUTO_CHECKPOINT") == nullptr;
        if (autoCheckpoint) {
            mustRun(*conn, "CALL auto_checkpoint=true;");
        }
        // SCENARIO_CHECKPOINT_THRESHOLD=<octets> : le seuil du journal qui déclenche le point de
        // reprise automatique (16 Mio par défaut ; ticket du cœur 82695b1d4, première voie).
        const auto* threshold = std::getenv("SCENARIO_CHECKPOINT_THRESHOLD");
        if (threshold != nullptr) {
            mustRun(*conn, stringFormat("CALL checkpoint_threshold={};", threshold));
        }
        Scenario scenario(*database, period);
        scenario.run(6);

        // Le pic du tampon : sur toute la passe, puis entre chaque début de DDL et le suivant.
        const auto peak = [&](double from, double to) {
            uint64_t out = 0;
            for (const auto& [at, used] : scenario.bufferSamples) {
                if (at >= from && at < to) {
                    out = std::max(out, used);
                }
            }
            return out / (1024 * 1024);
        };
        std::cerr << "  [scénario] tampon : " << BUFFER_POOL / (1024 * 1024)
                  << " Mio, point de reprise automatique "
                  << (autoCheckpoint ? "en marche" : "coupé") << ", seuil "
                  << (threshold ? threshold : "par défaut") << ", pic " << peak(0, scenario.end)
                  << " Mio\n";
        double from = 0;
        for (const auto& ddl : scenario.ddls) {
            std::cerr << "  [scénario] tampon avant « " << ddl.name << " » : pic "
                      << peak(from, ddl.start) << " Mio ; pendant : pic "
                      << peak(ddl.start, ddl.end) << " Mio\n";
            from = ddl.end;
        }
        std::cerr << "  [scénario] tampon après le dernier DDL : pic " << peak(from, scenario.end)
                  << " Mio\n";

        // La datation des longues attentes : par DDL et par acteur, les transactions qui
        // chevauchent le DDL et durent plus d'une seconde, leur nombre, et l'instant (relatif au
        // début du DDL) où la première a commencé et où la dernière a fini.
        for (const auto& ddl : scenario.ddls) {
            for (const auto actor : {Actor::DocWriter, Actor::NoteWriter, Actor::Reader}) {
                int64_t count = 0;
                double firstStart = 0, lastStart = 0, firstEnd = 0, lastEnd = 0;
                for (const auto& journal : scenario.journals) {
                    if (journal.actor != actor) {
                        continue;
                    }
                    for (const auto& sample : journal.samples) {
                        if (!overlaps(sample, ddl) || sample.end - sample.start < 1000) {
                            continue;
                        }
                        const auto start = sample.start - ddl.start;
                        const auto end = sample.end - ddl.start;
                        if (count == 0) {
                            firstStart = lastStart = start;
                            firstEnd = lastEnd = end;
                        }
                        firstStart = std::min(firstStart, start);
                        lastStart = std::max(lastStart, start);
                        firstEnd = std::min(firstEnd, end);
                        lastEnd = std::max(lastEnd, end);
                        count++;
                    }
                }
                if (count > 0) {
                    std::cerr << "  [scénario] attentes de plus d'1 s pendant « " << ddl.name
                              << " » (durée " << ddl.end - ddl.start
                              << " ms) acteur=" << actorName(actor) << " n=" << count
                              << " : commencées de " << firstStart << " à " << lastStart
                              << " ms, finies de " << firstEnd << " à " << lastEnd
                              << " ms après le début du DDL\n";
                }
            }
        }

        // Les fenêtres.
        for (const auto actor : {Actor::DocWriter, Actor::NoteWriter, Actor::Reader}) {
            printWindow(scenario, "hors DDL", actor, nullptr);
            for (const auto& ddl : scenario.ddls) {
                printWindow(scenario, ddl.name, actor, &ddl);
            }
        }
        for (const auto& [error, count] : errorsOf(scenario, nullptr)) {
            std::cerr << "  [scénario] erreur hors DDL n=" << count << " « "
                      << scenario.errors.at(error) << " »\n";
        }
        for (const auto& ddl : scenario.ddls) {
            for (const auto& [error, count] : errorsOf(scenario, &ddl)) {
                std::cerr << "  [scénario] erreur pendant « " << ddl.name << " » n=" << count
                          << " « " << scenario.errors.at(error) << " »\n";
            }
        }

        // Partout : aucune erreur sans nom.
        for (auto i = 0u; i < scenario.errors.size(); ++i) {
            EXPECT_FALSE(isUnnamed(scenario.errors.at(i)))
                << "[check: no-unnamed-error] « " << scenario.errors.at(i) << " »";
        }
        for (const auto& ddl : scenario.ddls) {
            if (!ddl.ok) {
                EXPECT_FALSE(isUnnamed(ddl.error))
                    << "[check: no-unnamed-error] « " << ddl.name << " » : « " << ddl.error << " »";
            }
        }

        // Les bornes des DDL.
        expectHeldAtMostItsDuration(scenario, "3 ",
            "create-rel-table-holds-writers-at-most-its-duration");
        expectHeldAtMostItsDuration(scenario, "4 ",
            "index-creation-holds-writers-at-most-its-duration");
        expectHeldAtMostItsDuration(scenario, "5 ",
            "checkpoint-holds-writers-at-most-its-duration");
        if (const auto* index = ddlNamed(scenario, "4 ")) {
            int64_t refused = 0;
            for (const auto& [error, count] : errorsOf(scenario, index, Actor::Reader)) {
                refused += count;
            }
            EXPECT_EQ(refused, 0) << "[check: index-creation-refuses-no-reader]";
        }

        // La migration annulée.
        EXPECT_TRUE(!scenario.secondAddError.empty() && !isUnnamed(scenario.secondAddError))
            << "[check: failed-migration-is-refused-by-name] « " << scenario.secondAddError << " »";
        if (const auto* migration = ddlNamed(scenario, "6 migration")) {
            const auto usual = errorsOf(scenario, nullptr);
            for (const auto& [error, count] : errorsOf(scenario, migration)) {
                EXPECT_TRUE(usual.contains(error))
                    << "[check: failed-migration-disturbs-no-one] " << count
                    << " transactions got « " << scenario.errors.at(error)
                    << " » during the failed migration, never outside a DDL";
            }
        }
        const auto columns = namesOf(*conn, "CALL TABLE_INFO('Doc') RETURN name;");
        EXPECT_FALSE(columns.contains("a")) << "[check: failed-migration-leaves-nothing]";

        // E10, avant tout point de reprise.
        mustRun(*conn, stringFormat("MATCH (d:Doc) WHERE d.id < {} SET d.b = concat('b-ecrit-', "
                                    "CAST(d.id AS STRING));",
                           NUM_B_SET));
        const auto asSet = bAsSet(*conn);
        EXPECT_EQ(asSet, NUM_B_SET) << "[check: added-column-reads-what-was-set] before reopening";
        expectDefaults(*conn, "avant le point de reprise");
        const auto expected = expectedCounts(scenario);
        expectCounts(*conn, expected, "avant le point de reprise");

        // Le point de reprise rend permanent ce que le stockage tient (ticket du cœur du 18
        // octobre).
        mustRun(*conn, "CHECKPOINT;");
        EXPECT_EQ(bAsSet(*conn), NUM_B_SET)
            << "[check: added-column-reads-what-was-set] after the checkpoint";
        expectDefaults(*conn, "après le point de reprise");

        // L'arrêt propre, puis la réouverture.
        conn.reset();
        database.reset();
        createDBAndConn();
        concurrency::applyBenchSettings(*conn);
        concurrency::loadVectorExtension(*conn);
        expectCounts(*conn, expected, "après la réouverture");
        const auto reopened = namesOf(*conn, "CALL TABLE_INFO('Doc') RETURN name;");
        const auto tables = namesOf(*conn, "CALL SHOW_TABLES() RETURN name;");
        const auto indexes = namesOf(*conn, "CALL SHOW_INDEXES() RETURN index_name;");
        const auto* index = ddlNamed(scenario, "4 ");
        const bool indexExpected = index && index->ok;
        EXPECT_TRUE(reopened.contains("note") && reopened.contains("rang") &&
                    reopened.contains("b") && !reopened.contains("a") && tables.contains("Cite") &&
                    indexes.contains("doc_vec") == indexExpected)
            << "[check: catalog-as-expected] note " << reopened.contains("note") << ", rang "
            << reopened.contains("rang") << ", b " << reopened.contains("b") << ", a "
            << reopened.contains("a") << ", Cite " << tables.contains("Cite") << ", doc_vec "
            << indexes.contains("doc_vec") << " (expected " << indexExpected << ")";
        EXPECT_EQ(bAsSet(*conn), NUM_B_SET) << "[check: added-column-survives-reopen] SET values";
        expectDefaults(*conn, "après la réouverture");
    }
};

TEST_F(TwentyWriters, DdlEveryFifteenSeconds) {
    playScenario(std::chrono::seconds(15));
}

TEST_F(TwentyWriters, DdlEveryMinute) {
    playScenario(std::chrono::minutes(1));
}

TEST_F(TwentyWriters, KilledDuringTheCheckpoint) {
    namespace fs = std::filesystem;
    const auto logPath = databasePath + ".commits";
    const auto markerPath = databasePath + ".checkpoint";
    fs::create_directories(fs::path(databasePath).parent_path());
    fs::remove(logPath);
    fs::remove(markerPath);
    systemConfig->bufferPoolSize = BUFFER_POOL;
    const auto pid = fork();
    if (pid == 0) {
        concurrency::disableCoreDumps();
        try {
            rag3db::main::Database child(databasePath, *systemConfig);
            {
                rag3db::main::Connection connection(&child);
                load(connection, databasePath);
            }
            CommitLog commitLog(logPath);
            Scenario scenario(child, std::chrono::seconds(5), &commitLog);
            scenario.run(6, [&] { std::ofstream(markerPath) << "checkpoint\n"; });
        } catch (std::exception& e) {
            std::cerr << "  [scénario] fils : " << e.what() << "\n";
            _exit(3);
        }
        // Le CHECKPOINT et la suite ont fini avant le SIGKILL : le père le dira.
        _exit(4);
    }
    const auto deadline = Clock::now() + std::chrono::minutes(5);
    while (!fs::exists(markerPath) && Clock::now() < deadline) {
        int status = 0;
        if (waitpid(pid, &status, WNOHANG) == pid) {
            FAIL() << "[check: plan] the child left before the checkpoint, status " << status;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
    }
    ASSERT_TRUE(fs::exists(markerPath)) << "[check: plan] the child never reached the checkpoint";
    std::this_thread::sleep_for(std::chrono::milliseconds(20));
    kill(pid, SIGKILL);
    int status = 0;
    waitpid(pid, &status, 0);
    ASSERT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
        << "[check: plan] the child was not killed, status " << status;
    const auto walPath = databasePath + ".wal";
    std::cerr << "  [scénario] tué pendant le CHECKPOINT ; .wal "
              << (fs::exists(walPath) ? std::to_string(fs::file_size(walPath)) : "absent")
              << " octets\n";
    // Le fils est mort base ouverte (SIGKILL dans la portée de sa Database, vérifié ci-dessus) ;
    // sans journal à rejouer, la réouverture ne prouverait rien.
    ASSERT_TRUE(fs::exists(walPath) && fs::file_size(walPath) > 0)
        << "[check: journal-to-replay] no .wal left by the killed child";

    // Ce que le fils avait vu validé.
    std::set<int64_t> newKeys;
    int64_t docCommits = 0;
    int64_t noteCommits = 0;
    std::map<std::string, std::string> ddlOutcomes;
    {
        std::ifstream in(logPath);
        for (std::string kind, value; in >> kind >> value;) {
            if (kind == "D") {
                docCommits++;
                if (std::stoll(value) >= NUM_DOCS) {
                    newKeys.insert(std::stoll(value));
                }
            } else if (kind == "N") {
                noteCommits++;
            } else if (kind == "DDL") {
                std::string outcome;
                in >> outcome;
                ddlOutcomes[value] = outcome;
            }
        }
    }

    createDBAndConn();
    concurrency::applyBenchSettings(*conn);
    concurrency::loadVectorExtension(*conn);
    std::set<int64_t> present;
    {
        auto result =
            conn->query(stringFormat("MATCH (d:Doc) WHERE d.id >= {} RETURN d.id;", NUM_DOCS));
        ASSERT_TRUE(result->isSuccess()) << "[check: query] " << result->getErrorMessage();
        while (result->hasNext()) {
            present.insert(result->getNext()->getValue(0)->getValue<int64_t>());
        }
    }
    int64_t lost = 0;
    for (const auto key : newKeys) {
        lost += !present.contains(key);
    }
    const auto links = countOf(*conn, "MATCH ()-[l:Link]->() RETURN count(l);");
    const auto notes = countOf(*conn, "MATCH (n:Note) RETURN count(n);");
    std::cerr << "  [scénario] après l'arrêt brutal : Doc neufs " << present.size() << " (journal "
              << newKeys.size() << ", perdus " << lost << "), Link " << links << " (journal "
              << NUM_LINKS + docCommits << "), Note " << notes << " (journal " << noteCommits
              << ")\n";
    // Une validation faite et pas encore écrite au journal : au plus une par écrivain.
    EXPECT_TRUE(lost == 0 && links >= NUM_LINKS + docCommits &&
                links <= NUM_LINKS + docCommits + NUM_DOC_WRITERS && notes >= noteCommits &&
                notes <= noteCommits + NUM_NOTE_WRITERS)
        << "[check: brutal-stop-loses-no-commit] lost Doc " << lost << ", Link " << links << " for "
        << NUM_LINKS + docCommits << ", Note " << notes << " for " << noteCommits;
    const auto columns = namesOf(*conn, "CALL TABLE_INFO('Doc') RETURN name;");
    const auto tables = namesOf(*conn, "CALL SHOW_TABLES() RETURN name;");
    const auto indexes = namesOf(*conn, "CALL SHOW_INDEXES() RETURN index_name;");
    EXPECT_TRUE(columns.contains("note") == (ddlOutcomes["1"] == "ok") &&
                columns.contains("rang") == (ddlOutcomes["2"] == "ok") &&
                tables.contains("Cite") == (ddlOutcomes["3"] == "ok") &&
                indexes.contains("doc_vec") == (ddlOutcomes["4"] == "ok"))
        << "[check: brutal-stop-keeps-the-catalog] note " << columns.contains("note") << ", rang "
        << columns.contains("rang") << ", Cite " << tables.contains("Cite") << ", doc_vec "
        << indexes.contains("doc_vec");
}

} // namespace

#endif
