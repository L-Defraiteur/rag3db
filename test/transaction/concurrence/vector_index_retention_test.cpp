// L'index vectoriel retiré ou recréé, vu des autres transactions (ticket
// 2026-10-12-un-index-retire-ou-recree-reste-visible-a-tous-dans-le-stockage, pas 2 de hotfixs).
// Le catalogue est versionné, la liste des index du stockage ne l'est pas : un DROP retire l'index
// pour tout le monde avant sa validation, et une annulation ne le rend pas ; un CREATE l'ajoute
// avant sa validation, et une annulation ne l'en retire pas. Témoins écrits avant le correctif :
// - VectorIndexRetention.ReaderBeforeDropSeesTheOldIndex : une transaction en lecture seule
//   commence (et lit) avant qu'un DROP ne valide en face, puis cherche de nouveau : elle doit
//   rendre les mêmes résultats que sa recherche d'avant. (Un CREATE validé en face d'un lecteur ouvert
//   n'existe pas : CREATE_VECTOR_INDEX force un point de reprise, qui attend le départ des
//   lecteurs, et c'est voulu.)
// - VectorIndexRetention.RolledBackCreateCanBeRedone : un lecteur ouvert ; en face, un CREATE dont
//   le point de reprise forcé attend les lecteurs jusqu'au délai, puis échoue et s'annule ; le
//   lecteur fermé, le même CREATE doit réussir, sans « already exists », et l'index répondre.
// - VectorIndexRetention.RolledBackDropKeepsTheIndex : DROP dans une transaction annulée : l'index
//   doit rester et rendre les mêmes résultats. Sans la rétention, le DROP détruit l'index dans le
//   stockage tout de suite, l'annulation ne rend que l'entrée du catalogue, et la liaison le prend
//   pour un index détaché par le rejeu (message faux, index perdu).

#include <string>
#include <vector>

#include "bench_harness.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

constexpr int64_t NUM_ROWS = 300;
constexpr auto SEARCH = "CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [7, 7, 7, 150], 10) RETURN "
                        "node.id ORDER BY node.id;";

class VectorIndexRetention : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
        concurrency::loadVectorExtension(*conn);
        ok(*conn, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
        ok(*conn, stringFormat("UNWIND range(0, {}) AS i CREATE (:Doc {id: i, vec: [i % 17, "
                               "i % 23, i % 29, i]});",
                      NUM_ROWS - 1));
    }

    static void ok(rag3db::main::Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess())
            << "[check: query] " << query.substr(0, 160) << " : " << result->getErrorMessage();
    }

    void createIndex(rag3db::main::Connection& connection, const std::string& metric) {
        ok(connection, stringFormat("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := "
                                    "'{}');",
                           metric));
    }

    // Les identifiants rendus par la recherche, ou le message d'erreur précédé de « ERREUR : ».
    static std::string search(rag3db::main::Connection& connection) {
        auto result = connection.query(SEARCH);
        if (!result->isSuccess()) {
            return "ERREUR : " + result->getErrorMessage().substr(0, 200);
        }
        std::string out;
        while (result->hasNext()) {
            out += result->getNext()->getValue(0)->toString() + " ";
        }
        return out;
    }
};

TEST_F(VectorIndexRetention, ReaderBeforeDropSeesTheOldIndex) {
    createIndex(*conn, "l2");
    rag3db::main::Connection reader(database.get());
    rag3db::main::Connection writer(database.get());
    ok(reader, "BEGIN TRANSACTION READ ONLY;");
    const auto before = search(reader);
    ASSERT_EQ(before.rfind("ERREUR", 0), std::string::npos) << "[check: query] " << before;
    ok(writer, "CALL DROP_VECTOR_INDEX('Doc', 'doc_index');");
    const auto after = search(reader);
    std::cerr << "  [rétention] avant : " << before << "\n  [rétention] après : " << after << "\n";
    EXPECT_EQ(after, before) << "[check: reader-before-drop-sees-old-index] ";
    // Une recherche refusée a déjà fermé la transaction : la fin n'est pas jugée.
    reader.query("ROLLBACK;");
}

TEST_F(VectorIndexRetention, RolledBackCreateCanBeRedone) {
    // Le point de reprise forcé du CREATE attend les lecteurs jusqu'au délai par défaut (5 s ; le
    // réglage du délai n'est ouvert qu'à DBTest).
    {
        rag3db::main::Connection reader(database.get());
        ok(reader, "BEGIN TRANSACTION READ ONLY;");
        ok(reader, "MATCH (n:Doc) RETURN count(*);");
        auto failed = conn->query(
            "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
        std::cerr << "  [rétention] CREATE face au lecteur : "
                  << (failed->isSuccess() ? std::string("réussi") :
                                            failed->getErrorMessage().substr(0, 160))
                  << "\n";
        ASSERT_FALSE(failed->isSuccess())
            << "[check: create-fails-behind-a-reader] le CREATE devait échouer sur le délai";
        ok(reader, "COMMIT;");
    }
    // Depuis le CREATE concurrent ((A)), l'entrée et le holder sont de T_A, validée avant le bâti :
    // le CREATE qui échoue au bâti laisse un index INCOMPLETE. Il ne se refait pas par-dessus (un
    // CREATE le refuse en disant de le retirer) ; DROP puis CREATE le rebâtissent.
    auto shown = conn->query("CALL SHOW_INDEXES() RETURN index_name, state;");
    std::string state = "<absent>";
    if (shown->isSuccess() && shown->hasNext()) {
        state = shown->getNext()->getValue(1)->toString();
    }
    shown.reset();
    std::cerr << "  [rétention] état après le CREATE annulé : " << state << "\n";
    EXPECT_EQ(state, "INCOMPLETE") << "[check: rolled-back-create-can-be-redone]";
    auto refused = conn->query(
        "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    const auto refusal = refused->isSuccess() ? std::string("réussi") : refused->getErrorMessage();
    std::cerr << "  [rétention] CREATE par-dessus l'index incomplet : " << refusal.substr(0, 200)
              << "\n";
    EXPECT_NE(refusal.find("is behind its table"), std::string::npos)
        << "[check: rolled-back-create-can-be-redone] " << refusal;
    refused.reset();
    ok(*conn, "CALL DROP_VECTOR_INDEX('Doc', 'doc_index');");
    auto result = conn->query(
        "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    const auto message = result->isSuccess() ? std::string() : result->getErrorMessage();
    std::cerr << "  [rétention] CREATE après DROP : "
              << (result->isSuccess() ? std::string("réussi") : message.substr(0, 200)) << "\n";
    EXPECT_TRUE(result->isSuccess()) << "[check: rolled-back-create-can-be-redone] " << message;
    if (result->isSuccess()) {
        const auto found = search(*conn);
        EXPECT_EQ(found.rfind("ERREUR", 0), std::string::npos)
            << "[check: recreated-index-answers] " << found;
    }
}

TEST_F(VectorIndexRetention, RolledBackDropKeepsTheIndex) {
    createIndex(*conn, "l2");
    const auto before = search(*conn);
    ASSERT_EQ(before.rfind("ERREUR", 0), std::string::npos) << "[check: query] " << before;
    ok(*conn, "BEGIN TRANSACTION;");
    ok(*conn, "CALL DROP_VECTOR_INDEX('Doc', 'doc_index');");
    ok(*conn, "ROLLBACK;");
    const auto after = search(*conn);
    std::cerr << "  [rétention] après un DROP annulé : " << after << "\n";
    EXPECT_EQ(after, before) << "[check: rolled-back-drop-keeps-index] ";
}

} // namespace
