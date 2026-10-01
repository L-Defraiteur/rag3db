// Un lecteur d'un autre processus ouvre la base pendant qu'un écrivain y
// travaille. Son ouverture lit trois choses à trois instants, sans verrou : le
// journal (pour décider), le fichier de données, puis le journal encore (pour
// le rejouer). Si l'écrivain fait un point de reprise entre deux, le lecteur
// charge un fichier qui contient déjà les transactions et les rejoue par-dessus.
//
// Ces tests rendent la course déterministe : un écrivain et un lecteur dans le
// même processus (le lecteur ne prend aucun verrou), et un crochet de test qui
// fait faire son CHECKPOINT à l'écrivain à un point précis de l'ouverture du
// lecteur. Ils ne dépendent d'aucune vitesse.
//
// Ce qu'ils ne couvrent pas : un lecteur qui RESTE ouvert pendant qu'un point
// de reprise passe. La revérification à l'ouverture ne le protège pas ; il faut
// une époque écrite dans le fichier (marche suivante).

#include <functional>
#include <memory>
#include <string>

#include "api_test/private_api_test.h"
#include "common/exception/runtime.h"
#include "flaky_checkpointer.h"
#include "gmock/gmock.h"
#include "storage/checkpointer.h"
#include "storage/shadow_file.h"
#include "storage/storage_manager.h"
#include "storage/wal/wal.h"
#include "storage/wal/wal_replayer.h"
#include "transaction/transaction_manager.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::storage;
using namespace rag3db::testing;
using namespace rag3db::transaction;
using ::testing::HasSubstr;

// Un point de reprise arrêté à mi-chemin : le CHECKPOINT est journalisé, les
// pages sont appliquées au fichier de données, et le journal n'est PAS encore
// tronqué. C'est l'instant où un lecteur qui a déjà parcouru le journal charge
// un fichier contenant ce que le journal lui dit de rejouer. Même doublure que
// FlakyCheckpointerFailsOnClearingFiles (checkpoint_test.cpp).
class CheckpointerStoppedBeforeTruncatingJournal final : public Checkpointer {
public:
    explicit CheckpointerStoppedBeforeTruncatingJournal(ClientContext& context)
        : Checkpointer(context) {}

    void logCheckpointAndApplyShadowPages() override {
        auto& shadowFile = StorageManager::Get(clientContext)->getShadowFile();
        shadowFile.flushAll(clientContext);
        WAL::Get(clientContext)->logAndFlushCheckpoint(&clientContext);
        shadowFile.applyShadowPages(clientContext);
        throw RuntimeException("checkpoint stopped before truncating the journal.");
    }
};

class ReadOnlyOpenTest : public PrivateApiTest {
protected:
    // Pas de jeu de données : chaque test pose le sien, petit.
    std::string getInputDir() override { return "empty"; }

    void SetUp() override {
        PrivateApiTest::SetUp();
        run("CALL force_checkpoint_on_close=false");
        run("CALL auto_checkpoint=false");
    }

    void TearDown() override {
        WALReplayer::setReadOnlyOpenHookForTesting(nullptr);
        reader.conn.reset();
        reader.db.reset();
        PrivateApiTest::TearDown();
    }

    // Ces tests demandent un journal non vide sur disque.
    bool notApplicable() const { return inMemMode || systemConfig->checkpointThreshold == 0; }

    struct Reader {
        std::unique_ptr<Database> db;
        std::unique_ptr<Connection> conn;
        std::string error; // vide si l'ouverture a réussi
        bool opened() const { return error.empty(); }
    };

    // Ouvre la base en lecture seule, dans ce même processus, à côté de
    // l'écrivain (`database` / `conn` du harnais).
    void openReader() {
        reader = Reader{};
        auto config = *systemConfig;
        config.readOnly = true;
        try {
            reader.db = std::make_unique<Database>(databasePath, config);
            reader.conn = std::make_unique<Connection>(reader.db.get());
        } catch (std::exception& e) {
            reader.db.reset();
            reader.error = e.what();
        }
    }

    // Arme le crochet : au point `phase` de la prochaine ouverture en lecture
    // seule, l'écrivain exécute `query`, une seule fois.
    void writerRunsAt(ReadOnlyOpenPhase phase, std::string query) {
        auto fired = std::make_shared<bool>(false);
        WALReplayer::setReadOnlyOpenHookForTesting(
            [this, phase, fired, query = std::move(query)](ReadOnlyOpenPhase current) {
                if (current != phase || *fired) {
                    return;
                }
                *fired = true;
                auto result = conn->query(query);
                EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
            });
        hookFired = fired;
    }

    // Arme le crochet : au point `phase`, l'écrivain commence un point de
    // reprise qui s'arrête après avoir appliqué ses pages, journal intact.
    void writerCheckpointStopsHalfwayAt(ReadOnlyOpenPhase phase) {
        auto fired = std::make_shared<bool>(false);
        WALReplayer::setReadOnlyOpenHookForTesting([this, phase, fired](ReadOnlyOpenPhase current) {
            if (current != phase || *fired) {
                return;
            }
            *fired = true;
            FlakyCheckpointer([](ClientContext& context) -> std::unique_ptr<Checkpointer> {
                return std::make_unique<CheckpointerStoppedBeforeTruncatingJournal>(context);
            }).setCheckpointer(*getClientContext(*conn));
            auto result = conn->query("CHECKPOINT");
            EXPECT_FALSE(result->isSuccess()) << "le point de reprise devait s'arrêter à mi-chemin";
        });
        hookFired = fired;
    }

    static int64_t single(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        if (!result->isSuccess() || !result->hasNext()) {
            return -1;
        }
        return result->getNext()->getValue(0)->getValue<int64_t>();
    }

    void run(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    Reader reader;
    std::shared_ptr<bool> hookFired;
};

// Un point de reprise COMPLET de l'écrivain entre le parcours du journal et la
// lecture du fichier de données : le journal a disparu sous le lecteur.
TEST_F(ReadOnlyOpenTest, CompletedCheckpointAfterJournalScanIsRefused) {
    if (notApplicable()) {
        GTEST_SKIP();
    }
    run("CREATE NODE TABLE Pair(id INT64 PRIMARY KEY, twice INT64)");
    run("CHECKPOINT");
    for (auto i = 0; i < 20; i++) {
        run("CREATE (:Pair {id: " + std::to_string(i) + ", twice: " + std::to_string(i * 2) + "})");
    }

    writerRunsAt(ReadOnlyOpenPhase::JOURNAL_SCANNED, "CHECKPOINT");
    openReader();
    ASSERT_TRUE(*hookFired) << "le crochet n'a pas été atteint : le test ne prouve rien";
    // Le lecteur a lu un fichier de données qui contient déjà ce que le journal
    // lui disait de rejouer. Il doit le savoir par l'identité de ce qu'il a lu,
    // et refuser en le disant — pas buter sur une clé en double.
    ASSERT_FALSE(reader.opened()) << "ouvert alors qu'un point de reprise a traversé l'ouverture";
    EXPECT_THAT(reader.error, HasSubstr(WALReplayer::CHECKPOINT_CROSSED_READ_ONLY_OPEN));

    // Sans point de reprise en travers, la même ouverture réussit et lit juste.
    WALReplayer::setReadOnlyOpenHookForTesting(nullptr);
    openReader();
    ASSERT_TRUE(reader.opened()) << reader.error;
    EXPECT_EQ(single(*reader.conn, "MATCH (p:Pair) RETURN count(*)"), 20);
    EXPECT_EQ(single(*reader.conn, "MATCH (p:Pair) WHERE p.twice <> p.id * 2 RETURN count(*)"), 0);
}

// Le cas rouge sur le poste : le point de reprise est à MI-CHEMIN quand le
// lecteur lit le fichier de données — pages appliquées, journal pas encore
// tronqué. Le lecteur rejoue des nœuds que le fichier contient déjà.
TEST_F(ReadOnlyOpenTest, HalfDoneCheckpointAfterJournalScanIsRefusedNotMisread) {
    if (notApplicable()) {
        GTEST_SKIP();
    }
    run("CREATE NODE TABLE Pair(id INT64 PRIMARY KEY, twice INT64)");
    run("CHECKPOINT");
    for (auto i = 0; i < 20; i++) {
        run("CREATE (:Pair {id: " + std::to_string(i) + ", twice: " + std::to_string(i * 2) + "})");
    }

    writerCheckpointStopsHalfwayAt(ReadOnlyOpenPhase::JOURNAL_SCANNED);
    openReader();
    ASSERT_TRUE(*hookFired) << "le crochet n'a pas été atteint : le test ne prouve rien";
    ASSERT_FALSE(reader.opened()) << "ouvert alors qu'un point de reprise a traversé l'ouverture";
    EXPECT_THAT(reader.error, HasSubstr(WALReplayer::CHECKPOINT_CROSSED_READ_ONLY_OPEN));

    // Un lecteur qui arrive maintenant voit, lui, le CHECKPOINT en fin de
    // journal dès son parcours : c'est le refus des pages fantômes, inchangé.
    WALReplayer::setReadOnlyOpenHookForTesting(nullptr);
    openReader();
    ASSERT_FALSE(reader.opened());
    EXPECT_THAT(reader.error, HasSubstr("Couldn't replay shadow pages under read-only mode"));
}

// La même course avec SEULEMENT des relations dans le journal : aucune clé
// primaire ne s'oppose au rejeu. Le danger n'est plus une erreur, c'est un
// résultat faux en silence.
TEST_F(ReadOnlyOpenTest, RelationshipsAreNotDoubledByAHalfDoneCheckpoint) {
    if (notApplicable()) {
        GTEST_SKIP();
    }
    run("CREATE NODE TABLE Node(id INT64 PRIMARY KEY)");
    run("CREATE REL TABLE Link(FROM Node TO Node)");
    for (auto i = 0; i < 10; i++) {
        run("CREATE (:Node {id: " + std::to_string(i) + "})");
    }
    run("CHECKPOINT");
    for (auto i = 0; i < 9; i++) {
        run("MATCH (a:Node), (b:Node) WHERE a.id = " + std::to_string(i) +
            " AND b.id = " + std::to_string(i + 1) + " CREATE (a)-[:Link]->(b)");
    }
    ASSERT_EQ(single(*conn, "MATCH ()-[l:Link]->() RETURN count(l)"), 9);

    writerCheckpointStopsHalfwayAt(ReadOnlyOpenPhase::JOURNAL_SCANNED);
    openReader();
    ASSERT_TRUE(*hookFired) << "le crochet n'a pas été atteint : le test ne prouve rien";
    if (reader.opened()) {
        // S'il s'ouvre, il n'a pas le droit de voir autre chose que la vérité.
        EXPECT_EQ(single(*reader.conn, "MATCH ()-[l:Link]->() RETURN count(l)"), 9)
            << "le lecteur a rejoué des relations que le fichier de données contenait déjà";
        FAIL() << "ouvert alors qu'un point de reprise a traversé l'ouverture";
    }
    EXPECT_THAT(reader.error, HasSubstr(WALReplayer::CHECKPOINT_CROSSED_READ_ONLY_OPEN));
}

// Le point de reprise démarre plus tard : le fichier de données est déjà lu, et
// le journal disparaît sous le rejeu.
TEST_F(ReadOnlyOpenTest, CheckpointDuringJournalReplayIsRefused) {
    if (notApplicable()) {
        GTEST_SKIP();
    }
    run("CREATE NODE TABLE Pair(id INT64 PRIMARY KEY, twice INT64)");
    run("CHECKPOINT");
    for (auto i = 0; i < 20; i++) {
        run("CREATE (:Pair {id: " + std::to_string(i) + ", twice: " + std::to_string(i * 2) + "})");
    }

    writerRunsAt(ReadOnlyOpenPhase::DATA_FILE_READ, "CHECKPOINT");
    openReader();
    ASSERT_TRUE(*hookFired) << "le crochet n'a pas été atteint : le test ne prouve rien";
    ASSERT_FALSE(reader.opened()) << "ouvert alors qu'un point de reprise a traversé l'ouverture";
    EXPECT_THAT(reader.error, HasSubstr(WALReplayer::CHECKPOINT_CROSSED_READ_ONLY_OPEN));
}

// Le garde-fou contre une revérification trop large : un écrivain qui ne fait
// que VALIDER pendant l'ouverture, sans point de reprise, ne doit pas faire
// refuser le lecteur. Il lit l'état d'avant ce commit, et c'est juste.
TEST_F(ReadOnlyOpenTest, CommitDuringOpenIsNotRefused) {
    if (notApplicable()) {
        GTEST_SKIP();
    }
    run("CREATE NODE TABLE Pair(id INT64 PRIMARY KEY, twice INT64)");
    run("CHECKPOINT");
    for (auto i = 0; i < 20; i++) {
        run("CREATE (:Pair {id: " + std::to_string(i) + ", twice: " + std::to_string(i * 2) + "})");
    }

    for (auto phase : {ReadOnlyOpenPhase::JOURNAL_SCANNED, ReadOnlyOpenPhase::DATA_FILE_READ}) {
        const auto before = single(*conn, "MATCH (p:Pair) RETURN count(*)");
        writerRunsAt(phase, "CREATE (:Pair {id: " + std::to_string(1000 + before) +
                                ", twice: " + std::to_string((1000 + before) * 2) + "})");
        openReader();
        ASSERT_TRUE(*hookFired);
        ASSERT_TRUE(reader.opened()) << "refusé pour un simple commit : " << reader.error;
        // Le commit est arrivé après le parcours du journal : le lecteur ne le
        // voit pas, et ce qu'il voit est cohérent.
        EXPECT_EQ(single(*reader.conn, "MATCH (p:Pair) RETURN count(*)"), before);
        EXPECT_EQ(single(*reader.conn, "MATCH (p:Pair) WHERE p.twice <> p.id * 2 RETURN count(*)"),
            0);
    }
}
