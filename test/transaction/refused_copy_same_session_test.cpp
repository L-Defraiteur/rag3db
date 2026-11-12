// Ce qui suit, dans la même session, un COPY refusé faute de tampon (lot G2 de la seconde session
// cœur C++, 7 novembre 2026 ; ticket 2026-10-05-copy-refuse-pour-memoire-table-faussee-en-memoire).
//
// Le 5 octobre, un COPY refusé par « The buffer pool is full » après avoir réservé ses lignes ne
// faisait reculer ni le compte de la table ni son curseur de réservation : dans la même session,
// une ligne créée ensuite n'était plus trouvée par sa clé, et le point de reprise suivant enflait
// dans l'index de clé primaire. Une réouverture remettait tout d'aplomb ; le produit rouvre donc
// la base depuis (filet 99e6f0ee0). Ce témoin regarde la session elle-même, sans réouverture.

#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class RefusedCopySameSession : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        systemConfig->bufferPoolSize = 64ull * 1024 * 1024 + TestHelper::HASH_INDEX_MEM;
        createDBAndConn();
    }
    static void ok(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    void ok(const std::string& query) { ok(*conn, query); }
    int64_t scalar(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        if (!result->isSuccess() || !result->hasNext()) {
            return -1;
        }
        return result->getNext()->getValue(0)->getValue<int64_t>();
    }
};

TEST_F(RefusedCopySameSession, TheTableIsWholeAfterACopyRefusedForMemory) {
    ok("CREATE NODE TABLE T(id INT64, s STRING, PRIMARY KEY(id));");
    ok("UNWIND range(0, 999) AS i CREATE (:T {id: i, s: 'avant'});");
    ok("CHECKPOINT;");
    ASSERT_EQ(scalar("MATCH (t:T) RETURN count(*);"), 1000);

    // Un COPY de 300 000 lignes de 1 Kio : 300 Mo pour un tampon de 64 Mio.
    auto copy = conn->query(
        "COPY T FROM (UNWIND range(100000, 399999) AS i RETURN i, repeat('x', 1000) + string(i));");
    ASSERT_FALSE(copy->isSuccess())
        << "le COPY devait être refusé : le tampon n'est pas assez serré";
    ASSERT_NE(copy->getErrorMessage().find("buffer pool is full"), std::string::npos)
        << copy->getErrorMessage();
    std::cout << "refus du COPY : " << copy->getErrorMessage().substr(0, 200) << "\n";

    // Dans la même session, sans réouverture : la table a ses mille lignes, et rien du COPY.
    EXPECT_EQ(scalar("MATCH (t:T) RETURN count(*);"), 1000) << "le compte de la table a bougé";
    EXPECT_EQ(scalar("MATCH (t:T) WHERE t.id >= 100000 RETURN count(*);"), 0)
        << "des lignes du COPY refusé sont visibles";
    // Une ligne créée ensuite se trouve par sa clé, et sa clé n'est prise qu'une fois.
    ok("CREATE (:T {id: -1, s: 'après'});");
    EXPECT_EQ(scalar("MATCH (t:T {id: -1}) RETURN count(*);"), 1)
        << "la ligne créée n'est pas trouvée par sa clé";
    auto twice = conn->query("CREATE (:T {id: -1, s: 'doublon'});");
    EXPECT_FALSE(twice->isSuccess()) << "la clé -1 a été acceptée deux fois";
    // Une clé du COPY refusé est libre.
    ok("CREATE (:T {id: 100000, s: 'libre'});");
    EXPECT_EQ(scalar("MATCH (t:T {id: 100000}) RETURN count(*);"), 1);
    // Un petit COPY passe, puis un point de reprise, et tout est juste.
    ok("COPY T FROM (UNWIND range(5000, 5099) AS i RETURN i, 'petit');");
    ok("CHECKPOINT;");
    EXPECT_EQ(scalar("MATCH (t:T) RETURN count(*);"), 1102);
    EXPECT_EQ(scalar("MATCH (t:T) RETURN count(DISTINCT t.id);"), 1102);
    EXPECT_EQ(scalar("MATCH (t:T {id: 5050}) RETURN count(*);"), 1);
}

// La même chose pour un COPY de relations refusé : les relations d'avant restent, aucune du COPY
// n'est visible, et un petit COPY de relations passe ensuite.
TEST_F(RefusedCopySameSession, TheRelTableIsWholeAfterARelCopyRefusedForMemory) {
    ok("CREATE NODE TABLE N(id INT64, PRIMARY KEY(id));");
    ok("CREATE REL TABLE R(FROM N TO N, s STRING);");
    ok("UNWIND range(0, 1999) AS i CREATE (:N {id: i});");
    ok("COPY R FROM (UNWIND range(0, 99) AS i RETURN i, i + 1, 'avant');");
    ok("CHECKPOINT;");
    auto copy =
        conn->query("COPY R FROM (UNWIND range(0, 299999) AS i RETURN i % 2000, (i * 7) % 2000, "
                    "repeat('r', 1000) + string(i));");
    ASSERT_FALSE(copy->isSuccess())
        << "le COPY devait être refusé : le tampon n'est pas assez serré";
    ASSERT_NE(copy->getErrorMessage().find("buffer pool is full"), std::string::npos)
        << copy->getErrorMessage();
    EXPECT_EQ(scalar("MATCH ()-[r:R]->() RETURN count(*);"), 100)
        << "le compte des relations a bougé";
    EXPECT_EQ(scalar("MATCH (b:N)<-[r:R]-(a:N) RETURN count(*);"), 100);
    ok("COPY R FROM (UNWIND range(100, 149) AS i RETURN i, i + 1, 'petit');");
    ok("CHECKPOINT;");
    EXPECT_EQ(scalar("MATCH ()-[r:R]->() RETURN count(*);"), 150);
}

// Un COPY refusé dans une transaction explicite : la transaction est condamnée, son ROLLBACK passe,
// et la session écrit ensuite juste.
TEST_F(RefusedCopySameSession, AnExplicitTransactionRollsBackACopyRefusedForMemory) {
    ok("CREATE NODE TABLE T(id INT64, s STRING, PRIMARY KEY(id));");
    ok("UNWIND range(0, 999) AS i CREATE (:T {id: i, s: 'avant'});");
    ok("CHECKPOINT;");
    ok("BEGIN TRANSACTION;");
    ok("CREATE (:T {id: -5, s: 'dans la transaction'});");
    auto copy = conn->query(
        "COPY T FROM (UNWIND range(100000, 399999) AS i RETURN i, repeat('x', 1000) + string(i));");
    ASSERT_FALSE(copy->isSuccess());
    ASSERT_NE(copy->getErrorMessage().find("buffer pool is full"), std::string::npos)
        << copy->getErrorMessage();
    ok("ROLLBACK;");
    EXPECT_EQ(scalar("MATCH (t:T) RETURN count(*);"), 1000);
    EXPECT_EQ(scalar("MATCH (t:T {id: -5}) RETURN count(*);"), 0)
        << "la ligne de la transaction annulée reste";
    ok("CREATE (:T {id: -5, s: 'après'});");
    EXPECT_EQ(scalar("MATCH (t:T {id: -5}) RETURN count(*);"), 1);
    ok("CHECKPOINT;");
    EXPECT_EQ(scalar("MATCH (t:T) RETURN count(DISTINCT t.id);"), 1001);
}

} // namespace
