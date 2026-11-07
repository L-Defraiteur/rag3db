// Une erreur pendant la validation, après l'ajout des lignes aux groupes (ticket du 10 octobre
// 2026, « validation à moitié appliquée sur échec après l'ajout des lignes »).
//
// NodeTable::commit ajoute d'abord les lignes locales aux groupes de la table (étape 1), puis
// écrit les lignes dans les index (étape 3). Si l'étape 3 lève, la transaction est annulée ; les
// lignes de l'étape 1 doivent disparaître avec elle, et la table rester celle d'avant : le même
// compte, aucune clé de la transaction échouée, et les validations suivantes à leur place. Le
// témoin fait lever l'étape 3 par un index d'essai posé sur la table (ThrowingIndex), puisque le
// chemin vu le 10 octobre (deux visibilités d'unicité) est fermé depuis A3′.
#include <atomic>
#include <memory>
#include <string>

#include "catalog/catalog.h"
#include "catalog/catalog_entry/table_catalog_entry.h"
#include "common/exception/runtime.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/index/index.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"
#include "transaction/transaction.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::storage;
using namespace rag3db::testing;

namespace {

std::atomic<bool> throwAtCommit{false};

// Un index qui ne garde rien et qui lève, armé, quand la validation lui donne ses lignes.
class ThrowingIndex final : public Index {
public:
    explicit ThrowingIndex(IndexInfo info) : Index{std::move(info), nullptr} {}
    std::unique_ptr<InsertState> initInsertState(ClientContext*, visible_func) override {
        return std::make_unique<InsertState>();
    }
    std::unique_ptr<DeleteState> initDeleteState(const rag3db::transaction::Transaction*,
        MemoryManager*, visible_func) override {
        return std::make_unique<DeleteState>();
    }
    void delete_(rag3db::transaction::Transaction*, const ValueVector&, DeleteState&) override {}
    bool needCommitInsert() const override { return true; }
    void commitInsert(rag3db::transaction::Transaction*, const ValueVector&,
        const std::vector<ValueVector*>&, InsertState&) override {
        if (throwAtCommit.load()) {
            throw RuntimeException("injected failure while inserting into an index at commit");
        }
    }
};

class HalfAppliedCommit : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        ok("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64);");
        ok("UNWIND range(0, 9) AS i CREATE (:Item {id: i, v: i});");
    }
    void TearDown() override {
        throwAtCommit = false;
        EmptyDBTest::TearDown();
    }
    void ok(const std::string& q) {
        auto result = conn->query(q);
        ASSERT_TRUE(result->isSuccess()) << q << " : " << result->getErrorMessage();
    }
    std::string row(const std::string& q) {
        auto result = conn->query(q);
        if (!result->isSuccess()) {
            return "REFUS " + result->getErrorMessage();
        }
        return result->hasNext() ? result->getNext()->toString() : "AUCUNE LIGNE";
    }
    void addThrowingIndex() {
        auto* context = getClientContext(*conn);
        auto* entry = rag3db::catalog::Catalog::Get(*context)->getTableCatalogEntry(
            &rag3db::transaction::DUMMY_TRANSACTION, "Item");
        auto& table = getStorageManager(*database)->getTable(entry->getTableID())->cast<NodeTable>();
        table.addIndex(std::make_unique<ThrowingIndex>(IndexInfo{"throwing_index", "THROWING",
            entry->getTableID(), {1}, {PhysicalTypeID::INT64}, false /* primary */,
            false /* builtin */}));
    }
};

TEST_F(HalfAppliedCommit, AFailureAfterTheRowsAreAppendedLeavesTheTableAsItWas) {
    addThrowingIndex();
    throwAtCommit = true;
    ok("BEGIN TRANSACTION;");
    ok("CREATE (:Item {id: 100, v: 1});");
    ok("CREATE (:Item {id: 101, v: 1});");
    const auto commit = row("COMMIT;");
    EXPECT_NE(commit.find("injected failure"), std::string::npos)
        << "[check: setup] the commit did not fail at step 3 : " << commit;
    conn->query("ROLLBACK;");
    throwAtCommit = false;
    EXPECT_EQ(row("MATCH (i:Item) RETURN count(*);"), "10\n")
        << "[check: failed-commit-leaves-the-count]";
    EXPECT_EQ(row("MATCH (i:Item) WHERE i.id >= 100 RETURN count(*);"), "0\n")
        << "[check: failed-commit-leaves-no-row]";
    // Les validations suivantes trouvent leurs lignes à leur place.
    ok("CREATE (:Item {id: 102, v: 2});");
    EXPECT_EQ(row("MATCH (i:Item {id: 102}) RETURN i.v;"), "2\n")
        << "[check: next-commit-finds-its-row]";
    ok("MATCH (i:Item {id: 5}) DELETE i;");
    EXPECT_EQ(row("MATCH (i:Item) RETURN count(*);"), "10\n")
        << "[check: next-delete-finds-its-row]";
    // Les clés de la transaction échouée sont libres.
    ok("CREATE (:Item {id: 100, v: 3});");
    EXPECT_EQ(row("MATCH (i:Item {id: 100}) RETURN i.v;"), "3\n")
        << "[check: failed-commit-keys-are-free]";
    EXPECT_EQ(row("MATCH (i:Item) RETURN sum(i.id);"), std::to_string(45 - 5 + 102 + 100) + "\n")
        << "[check: rows-and-offsets-agree]";
}

} // namespace
