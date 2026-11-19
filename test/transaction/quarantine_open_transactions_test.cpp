// The quarantine after a rollback that could not complete also holds for the transactions already
// open (ticket
// docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-12-verifier-la-quarantaine-d-une-annulation-partielle-pour-les-transactions-deja-ouvertes.md).
// When TA's rollback throws halfway through the shared storage, the manager refuses every BEGIN and
// CHECKPOINT by name (REOPEN_AFTER_FAILED_ROLLBACK). TB, a write transaction open before, could
// still COMMIT: a forced COMMIT reached the checkpointer, which writes to the data files what
// memory holds, a state half rolled back. First shown by Codex's private diagnostic of 12 Nov.
// (branch cx-hotfixs-temoin-quarantaine), here as an ordinary witness. Each case runs in a child
// process that never destroys its database: closing it would checkpoint the state under test.

#include <sys/wait.h>
#include <unistd.h>

#include <cstdio>
#include <filesystem>
#include <fstream>
#include <memory>
#include <string>

#include "catalog/catalog.h"
#include "common/exception/runtime.h"
#include "flaky_checkpointer.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/checkpointer.h"
#include "storage/index/index.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"
#include "transaction/transaction.h"
#include "transaction/transaction_context.h"
#include "transaction/transaction_manager.h"

namespace rag3db::testing {
namespace {

constexpr const char* UNDO_FAULT = "witness: one-shot rollback index fault";
constexpr const char* CHECKPOINT_STOP = "witness: checkpointer reached";

// An index on table A whose rollback of an insert throws once: TA's rollback stops halfway.
class RollbackFaultIndex final : public storage::Index {
public:
    RollbackFaultIndex(storage::IndexInfo info, bool& armed)
        : Index{std::move(info), std::make_unique<storage::IndexStorageInfo>()}, armed{armed} {}
    std::unique_ptr<InsertState> initInsertState(main::ClientContext*,
        storage::visible_func) override {
        return std::make_unique<InsertState>();
    }
    std::unique_ptr<DeleteState> initDeleteState(const transaction::Transaction*,
        storage::MemoryManager*, storage::visible_func) override {
        return std::make_unique<DeleteState>();
    }
    void delete_(transaction::Transaction*, const common::ValueVector&, DeleteState&) override {}
    void rollbackInsert(common::offset_t) override {
        if (armed) {
            armed = false;
            throw common::RuntimeException(UNDO_FAULT);
        }
    }

private:
    bool& armed;
};

class QuarantineOpenTransactions : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        systemConfig->forceCheckpointOnClose = false;
        report = std::filesystem::path(databasePath).parent_path() /
                 ("quarantine-" + std::to_string(getpid()) + ".txt");
    }

    // Plays the case in a child: TA's rollback stops halfway, then TB, open before, commits
    // (forced or not). The child writes one line per step to the report and leaves by _exit.
    void play(bool forceTB) {
        const auto pid = fork();
        if (pid == 0) {
            std::ofstream out(report);
            auto line = [&](const std::string& s) { out << s << std::endl; };
            auto* db = new main::Database(databasePath, *systemConfig);
            auto* a = new main::Connection(db);
            auto* b = new main::Connection(db);
            auto* control = new main::Connection(db);
            unsigned factoryCalls = 0;
            FlakyCheckpointer intercept{
                [&](main::ClientContext&) -> std::unique_ptr<storage::Checkpointer> {
                    ++factoryCalls;
                    throw common::RuntimeException(CHECKPOINT_STOP);
                }};
            intercept.setCheckpointer(*a->getClientContext());
            for (const auto* q :
                {"CALL auto_checkpoint=false;", "CALL force_checkpoint_on_copy=false;",
                    "CALL copy_journal_threshold=1073741824;",
                    "CREATE NODE TABLE A(id INT64 PRIMARY KEY, v INT64);",
                    "CREATE NODE TABLE B(id INT64 PRIMARY KEY, v INT64);",
                    "UNWIND range(0, 1) AS i CREATE (:A {id: i, v: i});",
                    "UNWIND range(0, 1) AS i CREATE (:B {id: i, v: i});"}) {
                if (!a->query(q)->isSuccess()) {
                    line(std::string("setup-failed\t") + q);
                    _exit(3);
                }
            }
            auto* context = a->getClientContext();
            const transaction::Transaction latest(transaction::TransactionType::READ_ONLY,
                transaction::Transaction::START_TRANSACTION_ID + 1000,
                transaction::Transaction::LATEST_COMMITTED_TS);
            auto* entry = catalog::Catalog::Get(*context)->getTableCatalogEntry(&latest, "A");
            auto& table = storage::StorageManager::Get(*context)
                              ->getTable(entry->getTableID())
                              ->cast<storage::NodeTable>();
            bool armed = false;
            table.addIndex(std::make_unique<RollbackFaultIndex>(
                storage::IndexInfo{"witness_rollback", "WITNESS", entry->getTableID(), {0},
                    {common::PhysicalTypeID::INT64}, false, false},
                armed));
            const auto csv = std::filesystem::path(databasePath).parent_path() /
                             ("quarantine-a-" + std::to_string(getpid()) + ".csv");
            {
                std::ofstream data(csv);
                data << "100,100\n101,101\n";
            }
            if (!a->query("BEGIN TRANSACTION;")->isSuccess() ||
                !b->query("BEGIN TRANSACTION;")->isSuccess() ||
                !a->query("COPY A FROM '" + csv.string() + "' (HEADER=false);")->isSuccess() ||
                !b->query("CREATE (:B {id: 200, v: 200});")->isSuccess()) {
                line("setup-failed\ttransactions");
                _exit(3);
            }
            auto* txB = transaction::TransactionContext::Get(*b->getClientContext())
                            ->getActiveTransaction();
            if (forceTB) {
                txB->setForceCheckpoint();
            }
            armed = true;
            const auto rollback = a->query("ROLLBACK;");
            line(std::string("rollback-a\t") +
                 (rollback->isSuccess() ? "ok" : rollback->getErrorMessage()));
            const auto begin = control->query("BEGIN TRANSACTION;");
            line(std::string("begin-control\t") +
                 (begin->isSuccess() ? "ok" : begin->getErrorMessage()));
            const auto commit = b->query("COMMIT;");
            line(std::string("commit-b\t") +
                 (commit->isSuccess() ? "ok" : commit->getErrorMessage()));
            line("checkpointer-calls\t" + std::to_string(factoryCalls));
            out.flush();
            _exit(0);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        ASSERT_TRUE(WIFEXITED(status) && WEXITSTATUS(status) == 0)
            << "[check: setup] child status " << status << "\n"
            << readReport();
    }

    std::string readReport() const {
        std::ifstream in(report);
        return {std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>()};
    }

    std::string value(const std::string& key) const {
        std::ifstream in(report);
        std::string l;
        while (std::getline(in, l)) {
            if (l.starts_with(key + "\t")) {
                return l.substr(key.size() + 1);
            }
        }
        return "<missing>";
    }

    void expectQuarantineHeldForTB() const {
        const std::string quarantine =
            transaction::TransactionManager::REOPEN_AFTER_FAILED_ROLLBACK;
        ASSERT_NE(value("rollback-a").find(UNDO_FAULT), std::string::npos)
            << "[check: setup] the rollback fault did not fire\n"
            << readReport();
        ASSERT_NE(value("begin-control").find(quarantine), std::string::npos)
            << "[check: setup] no quarantine after the half-done rollback\n"
            << readReport();
        EXPECT_NE(value("commit-b").find(quarantine), std::string::npos)
            << "[check: open-write-transaction-refused-in-quarantine] TB committed after the "
               "half-done rollback\n"
            << readReport();
        EXPECT_EQ(value("checkpointer-calls"), "0")
            << "[check: no-checkpoint-in-quarantine] the checkpointer was reached\n"
            << readReport();
    }

    std::filesystem::path report;
};

TEST_F(QuarantineOpenTransactions, AForcedCommitOpenBeforeIsRefusedWithoutCheckpointing) {
    play(true /*forceTB*/);
    if (HasFatalFailure()) {
        return;
    }
    expectQuarantineHeldForTB();
}

TEST_F(QuarantineOpenTransactions, AnOrdinaryCommitOpenBeforeIsRefused) {
    play(false /*forceTB*/);
    if (HasFatalFailure()) {
        return;
    }
    expectQuarantineHeldForTB();
}

} // namespace
} // namespace rag3db::testing
