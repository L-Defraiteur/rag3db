// Le lecteur du journal (wal_reader.h) : un outil de diagnostic en lecture seule, et ses témoins.
//
// - KnownScenario : un scénario connu (tables, insertions, relation, mise à jour, suppressions,
//   DETACH DELETE), son journal lu, la liste attendue des enregistrements de données.
// - TornCopySaysWhereItStops : le même journal coupé au milieu de son dernier enregistrement ; la
//   lecture s'arrête, déchirée, à la fin de l'avant-dernier.
// - PrintAFile : l'outil. RAG3DB_WAL_A_LIRE=<chemin d'un .wal> imprime ses enregistrements
//   (RAG3DB_WAL_BORNE : au plus tant, 100 000 par défaut). Sauté sans la variable.
//     RAG3DB_WAL_A_LIRE=/chemin/base.rag3db.wal build/release/test/transaction/transaction_test
//       --gtest_filter='WalReader.PrintAFile'

#include <cstdlib>
#include <filesystem>
#include <iostream>
#include <string>
#include <vector>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "storage/storage_utils.h"
#include "wal_reader.h"

using namespace rag3db::main;
using namespace rag3db::storage;
using namespace rag3db::testing;

namespace {

class WalReader : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "needs an on-disk database: an in-memory one writes no journal";
        }
        createDBAndConn();
        ok("CALL auto_checkpoint=false;");
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    // Le scénario : chaque instruction en auto-commit, une transaction chacune.
    void runScenario() {
        ok("CREATE NODE TABLE N(id INT64 PRIMARY KEY, v INT64);");
        ok("CREATE REL TABLE R(FROM N TO N);");
        ok("CREATE (:N {id: 1, v: 1}), (:N {id: 2, v: 2}), (:N {id: 3, v: 3});");
        ok("MATCH (a:N {id: 1}), (b:N {id: 2}) CREATE (a)-[:R]->(b);");
        ok("MATCH (n:N {id: 3}) SET n.v = 30;");
        ok("MATCH (:N {id: 1})-[r:R]->(:N {id: 2}) DELETE r;");
        ok("MATCH (a:N {id: 1}), (b:N {id: 3}) CREATE (a)-[:R]->(b);");
        ok("MATCH (n:N {id: 1}) DETACH DELETE n;");
    }

    // Une copie du journal vivant : la base reste ouverte, rien n'est rejoué.
    std::string copyOfTheJournal(const std::string& name) {
        const auto wal = StorageUtils::getWALFilePath(databasePath);
        const auto copy = databasePath + "." + name + ".wal";
        std::filesystem::copy_file(wal, copy, std::filesystem::copy_options::overwrite_existing);
        return copy;
    }

    // Les enregistrements de données, sans BEGIN, COMMIT ni catalogue.
    static std::vector<std::string> dataRecords(const WalReading& reading) {
        std::vector<std::string> out;
        for (const auto& e : reading.entries) {
            const auto name = walRecordTypeName(e.type);
            if (name == "BEGIN" || name == "COMMIT" || name == "CREATE_CATALOG_ENTRY" ||
                name == "UPDATE_SEQUENCE") {
                continue;
            }
            out.push_back(name + (e.detail.empty() ? "" : " " + e.detail));
        }
        return out;
    }
};

TEST_F(WalReader, KnownScenario) {
    runScenario();
    const auto reading = readWal(copyOfTheJournal("known"), *conn->getClientContext());
    std::cerr << formatWal(reading);
    EXPECT_EQ(reading.end, WalEnd::CLEAN) << "[check: wal-end] " << walEndName(reading.end);
    EXPECT_EQ(reading.endOffset, reading.fileSize) << "[check: wal-read-to-the-end] ";
    EXPECT_EQ(reading.lastCommitEnd, reading.fileSize) << "[check: wal-last-commit] ";
    // Toute écriture est dans une transaction, et il y en a une par instruction.
    int64_t begins = 0;
    for (const auto& e : reading.entries) {
        const auto name = walRecordTypeName(e.type);
        begins += name == "BEGIN";
        if (name != "BEGIN" && name != "COMMIT") {
            EXPECT_GE(e.transaction, 0) << "[check: wal-in-a-transaction] #" << e.index << " "
                                        << name;
        }
    }
    EXPECT_EQ(begins, 8) << "[check: wal-one-transaction-per-statement] ";
    // Les identifiants internes s'écrivent « table:décalage ». N a la table 0, R la table 1. Le
    // DETACH DELETE écrit la suppression du nœud avant les détachements de ses relations, aller
    // puis retour (relevé sur master du 11 octobre 2026).
    const std::vector<std::string> expected = {
        "TABLE_INSERTION table 0 rows 3",
        "TABLE_INSERTION table 1 rows 1 [0:0] [0:1] [1:0]",
        "NODE_UPDATE table 0 column 1 offset 2",
        "REL_DELETION table 1 src [0:0] dst [0:1] rel [1:0]",
        "TABLE_INSERTION table 1 rows 1 [0:0] [0:2] [1:1]",
        "NODE_DELETION table 0 offset 0",
        "REL_DETACH_DELETE table 1 fwd node [0:0]",
        "REL_DETACH_DELETE table 1 bwd node [0:0]",
    };
    EXPECT_EQ(dataRecords(reading), expected) << "[check: wal-known-records] ";
}

TEST_F(WalReader, TornCopySaysWhereItStops) {
    runScenario();
    const auto copy = copyOfTheJournal("torn");
    const auto whole = readWal(copy, *conn->getClientContext());
    ASSERT_GE(whole.entries.size(), 2u);
    const auto& last = whole.entries.back();
    // Coupé au milieu du dernier enregistrement.
    std::filesystem::resize_file(copy, last.offset + last.size / 2);
    const auto torn = readWal(copy, *conn->getClientContext());
    std::cerr << formatWal(torn);
    EXPECT_EQ(torn.end, WalEnd::TORN) << "[check: wal-torn] " << walEndName(torn.end);
    EXPECT_EQ(torn.entries.size(), whole.entries.size() - 1) << "[check: wal-torn-records] ";
    EXPECT_EQ(torn.endOffset, last.offset) << "[check: wal-torn-offset] ";
}

TEST_F(WalReader, PrintAFile) {
    const auto* path = std::getenv("RAG3DB_WAL_A_LIRE");
    if (path == nullptr) {
        GTEST_SKIP() << "RAG3DB_WAL_A_LIRE=<a .wal file> to print it";
    }
    uint64_t limit = 100000;
    if (const auto* borne = std::getenv("RAG3DB_WAL_BORNE")) {
        limit = std::strtoull(borne, nullptr, 10);
    }
    std::cout << formatWal(readWal(path, *conn->getClientContext(), limit));
}

} // namespace
