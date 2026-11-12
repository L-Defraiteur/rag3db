// Témoin privé, hors des suites automatiques ; ticket :
// docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-12-verifier-la-quarantaine-d-une-annulation-partielle-pour-les-transactions-deja-ouvertes.md
// Diagnostic de chemin seulement. Ne construit jamais un Checkpointer après la faute.
#include <cerrno>
#include <csignal>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>
#include <utility>
#include <vector>

#include "catalog/catalog.h"
#include "common/exception/runtime.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/checkpointer.h"
#include "storage/index/index.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"
#include "transaction/flaky_checkpointer.h"
#include "transaction/transaction.h"
#include "transaction/transaction_context.h"
#include "transaction/transaction_manager.h"

using namespace rag3db;
namespace fs = std::filesystem;

namespace {
constexpr const char* UNDO_FAULT = "diagnostic: one-shot rollback index fault";
constexpr const char* CHECKPOINT_STOP = "diagnostic: factory reached before checkpoint construction";

void require(bool condition, const std::string& message) {
    if (!condition) {
        throw std::runtime_error(message);
    }
}

struct Fault {
    main::ClientContext* owner = nullptr;
    rag3db::common::transaction_t transactionID = 0;
    unsigned calls = 0;
    bool armed = false;
    bool ownerMatched = false;
};

// Le callback n'a pas de paramètre transaction : l'identité est contrôlée via le
// contexte de TA et l'exclusivité de la table A dans cette fixture sur un seul fil.
class RollbackFaultIndex final : public storage::Index {
public:
    RollbackFaultIndex(storage::IndexInfo info, Fault& fault)
        : Index{std::move(info), nullptr}, fault{fault} {}
    std::unique_ptr<InsertState> initInsertState(main::ClientContext*, storage::visible_func) override {
        return std::make_unique<InsertState>();
    }
    std::unique_ptr<DeleteState> initDeleteState(const transaction::Transaction*,
        storage::MemoryManager*, storage::visible_func) override {
        return std::make_unique<DeleteState>();
    }
    void delete_(transaction::Transaction*, const rag3db::common::ValueVector&, DeleteState&) override {}
    void rollbackInsert(rag3db::common::offset_t firstOffset) override {
        ++fault.calls;
        auto* active = transaction::TransactionContext::Get(*fault.owner)->getActiveTransaction();
        fault.ownerMatched = active && active->getID() == fault.transactionID;
        require(fault.armed && fault.calls == 1 && fault.ownerMatched && firstOffset == 2,
            "callback inattendu ou mauvaise transaction");
        fault.armed = false;
        throw rag3db::common::RuntimeException(UNDO_FAULT);
    }

private:
    Fault& fault;
};

// Aucun objet Database/Connection n'est détruit dans l'enfant : même une sortie
// d'erreur rejoint _exit dans main, sans checkpoint ou rollback implicite.
void diagnose(const fs::path& root, std::ofstream& log) {
    auto record = [&](const std::string& line) { log << line << std::endl; };
    auto* db = new main::Database((root / "base.rag3db").string(),
        main::SystemConfig(64ull << 20, 2, true, false, -1u, false,
            16ull << 20, false /* forceCheckpointOnClose */));
    auto* a = new main::Connection(db);
    auto* b = new main::Connection(db);
    auto* control = new main::Connection(db);
    for (auto* connection : {a, b, control}) {
        connection->setMaxNumThreadForExec(2);
        connection->setQueryTimeOut(1000);
    }
    // Installée avant toute requête, y compris la préparation : aucun checkpoint
    // réel n'est permis dans cette tranche, même par un chemin imprévu.
    unsigned factoryCalls = 0;
    testing::FlakyCheckpointer intercept{
        [&](main::ClientContext&) -> std::unique_ptr<storage::Checkpointer> {
            ++factoryCalls;
            record("checkpoint-factory\t" + std::to_string(factoryCalls));
            throw rag3db::common::RuntimeException(CHECKPOINT_STOP);
        }};
    intercept.setCheckpointer(*a->getClientContext());
    auto query = [&](main::Connection& connection, const std::string& label,
                     const std::string& text) {
        auto result = connection.query(text);
        record(label + "\t" + (result->isSuccess() ? "ok" : result->getErrorMessage()));
        return result;
    };
    auto ok = [&](main::Connection& connection, const std::string& text) {
        auto result = query(connection, "setup", text);
        require(result->isSuccess(), "prémisse refusée : " + text);
    };
    ok(*a, "CALL enable_multi_writes=true;");
    ok(*a, "CALL auto_checkpoint=false;");
    ok(*a, "CALL force_checkpoint_on_copy=false;");
    ok(*a, "CALL copy_journal_threshold=1073741824;");
    ok(*a, "CREATE NODE TABLE A(id INT64 PRIMARY KEY, v INT64);");
    ok(*a, "CREATE NODE TABLE B(id INT64 PRIMARY KEY, v INT64);");
    ok(*a, "UNWIND range(0, 1) AS i CREATE (:A {id: i, v: i});");
    ok(*a, "UNWIND range(0, 1) AS i CREATE (:B {id: i, v: i});");

    auto* contextA = a->getClientContext();
    const transaction::Transaction latest(transaction::TransactionType::READ_ONLY,
        transaction::Transaction::START_TRANSACTION_ID + 1000,
        transaction::Transaction::LATEST_COMMITTED_TS);
    auto* entry = catalog::Catalog::Get(*contextA)->getTableCatalogEntry(&latest, "A");
    auto& table = storage::StorageManager::Get(*contextA)->getTable(entry->getTableID())
                      ->cast<storage::NodeTable>();
    Fault fault;
    table.addIndex(std::make_unique<RollbackFaultIndex>(
        storage::IndexInfo{"diagnostic_rollback", "DIAGNOSTIC", entry->getTableID(), {0},
            {rag3db::common::PhysicalTypeID::INT64}, false, false}, fault));

    const auto csv = root / "a.csv";
    { std::ofstream data(csv); data << "100,100\n101,101\n";
      require(data.good(), "écriture CSV refusée"); }
    ok(*a, "BEGIN TRANSACTION;");
    ok(*b, "BEGIN TRANSACTION;");
    auto* txA = transaction::TransactionContext::Get(*contextA)->getActiveTransaction();
    auto* txB = transaction::TransactionContext::Get(*b->getClientContext())->getActiveTransaction();
    require(txA && txB && txA->getID() != txB->getID(), "deux transactions requises");
    const auto idA = txA->getID();
    const auto idB = txB->getID();
    auto* manager = transaction::TransactionManager::Get(*contextA);
    ok(*a, "COPY A FROM '" + csv.string() + "' (HEADER=false);");
    ok(*b, "CREATE (:B {id: 200, v: 200});");
    require(table.getNumNodeGroups() == 1 && table.getNumTuplesInNodeGroup(0) == 4,
        "les deux lignes du COPY ne sont pas dans le groupe partagé attendu");
    require(!txA->shouldForceCheckpoint(), "COPY a basculé en checkpoint forcé");
    require(manager->getLockManager().getNumLocksHeld(idA) > 0, "TA ne tenait aucun verrou");
    auto index = table.getIndex("diagnostic_rollback", txA);
    require(index.has_value() && (*index)->isLoaded(), "index de fixture non visible/chargé");
    fault.owner = contextA;
    fault.transactionID = idA;
    fault.armed = true;
    record("transactions\t" + std::to_string(idA) + "\t" + std::to_string(idB));

    require(factoryCalls == 0, "fabrique atteinte pendant la préparation");
    auto rollback = query(*a, "rollback-A", "ROLLBACK;");
    require(!rollback->isSuccess() && rollback->getErrorMessage().find(UNDO_FAULT) != std::string::npos,
        "faute undo attendue non observée");
    require(fault.calls == 1 && !fault.armed && fault.ownerMatched, "injection non one-shot");
    require(factoryCalls == 0, "fabrique atteinte avant observation de TB");
    require(!manager->isActive(idA), "TA encore active");
    require(manager->getLockManager().getNumLocksHeld(idA) == 0, "verrous TA encore tenus");
    require(manager->isActive(idB), "TB devait être déjà ouverte");
    record("rollback-A-confirmed\tone-shot\towner-matched\tinactive\tlocks=0");

    const std::string quarantine = transaction::TransactionManager::REOPEN_AFTER_FAILED_ROLLBACK;
    for (const auto* command : {"BEGIN TRANSACTION;", "CHECKPOINT;"}) {
        auto result = query(*control, "public-control", command);
        require(!result->isSuccess() && result->getErrorMessage().find(quarantine) != std::string::npos,
            "contrôle public sans refus nommé de quarantaine");
        require(factoryCalls == 0, "contrôle public a atteint la fabrique");
    }
    txB->setForceCheckpoint();
    require(txB->shouldForceCheckpoint(), "TB non forcée");
    auto commit = query(*b, "commit-B-forced", "COMMIT;");
    require(!commit->isSuccess(), "succès inattendu : aucune fabrique réelle autorisée");
    if (factoryCalls == 1) {
        require(commit->getErrorMessage().find(CHECKPOINT_STOP) != std::string::npos,
            "fabrique atteinte mais exception sentinelle non remontée");
        record("diagnostic\tinternal-path-reached\tno-checkpointer-constructed");
    } else {
        require(factoryCalls == 0 && commit->getErrorMessage().find(quarantine) != std::string::npos,
            "ni chemin interne observé ni refus nommé : résultat inconclusif");
        record("diagnostic\tnamed-quarantine-before-factory");
    }
    require(fault.calls == 1, "rollback de TA rejoué");
    record("scope\tstructural-only\tno-real-checkpoint-after-fault\tno-reopen\tno-corruption-verdict");
}
} // namespace

int main(int argc, char** argv) {
    if (argc != 2 || !fs::path(argv[1]).is_absolute() || !fs::is_directory(argv[1])) {
        std::cerr << "usage: quarantine_released_diagnostic <répertoire privé existant absolu>\n";
        return 2;
    }
    // Apostrophe interdite : le chemin devient un littéral Cypher, sans échappement implicite.
    auto pattern = (fs::path(argv[1]) / "released-XXXXXX").string();
    if (pattern.find('\'') != std::string::npos || pattern.find('\\') != std::string::npos) {
        return 2;
    }
    std::vector<char> name(pattern.begin(), pattern.end());
    name.push_back('\0');
    auto* created = mkdtemp(name.data());
    if (!created) { return 2; }
    const fs::path root(created);
    std::cout << "artefacts conservés : " << root << std::endl;
    const auto pid = fork();
    if (pid < 0) { return 2; }
    if (pid == 0) {
        const rlimit noCore{0, 0};
        if (setrlimit(RLIMIT_CORE, &noCore) != 0) { _exit(3); }
        std::signal(SIGALRM, SIG_DFL);
        sigset_t signals;
        sigemptyset(&signals);
        sigaddset(&signals, SIGALRM);
        if (sigprocmask(SIG_UNBLOCK, &signals, nullptr) != 0) { _exit(3); }
        alarm(12); // Borne dure de l'enfant, indépendante des délais de requêtes.
        std::ofstream log(root / "steps.tsv");
        if (!log) { _exit(3); }
        try {
            diagnose(root, log);
            log.flush();
            _exit(log.good() ? 0 : 3);
        } catch (const std::exception& e) {
            log << "inconclusive\t" << e.what() << std::endl;
            _exit(3);
        } catch (...) {
            log << "inconclusive\tunknown exception" << std::endl;
            _exit(3);
        }
    }
    int status = 0;
    pid_t waited;
    do { waited = waitpid(pid, &status, 0); } while (waited < 0 && errno == EINTR);
    std::ofstream report(root / "parent.tsv");
    report << "pid\t" << pid << "\nwaited\t" << waited << "\nstatus\t" << status << '\n';
    const bool observed = waited == pid && WIFEXITED(status) && WEXITSTATUS(status) == 0;
    report << "result\t" << (observed ? "diagnostic-observed" : "inconclusive") << std::endl;
    return observed && report.good() ? 0 : 1;
}
