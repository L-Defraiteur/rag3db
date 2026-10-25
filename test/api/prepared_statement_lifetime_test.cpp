// La vie d'une requête préparée (ticket 2026-10-25-chaque-requete-preparee-reste-en-memoire-pour-
// la-vie-de-sa-connexion). Chaque préparation laissait sa requête liée (parsedStatement,
// logicalPlan, colonnes) dans un magasin de la connexion, indexé par un compteur, que rien ne
// vidait : un hôte qui prépare à chaque appel grossissait sans borne (23,7 Go vivants avec quatre
// lecteurs). La préparation porte désormais sa requête et la rend en mourant.
#include <malloc.h>

#include <filesystem>

#include "api_test/api_test.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

// La croissance tolérée de la mémoire vivante pour 3 000 préparations détruites : un plafond fixe,
// loin de la croissance de master (mesurée dans le commit), et sans rapport avec N.
constexpr int64_t PREPARED_GROWTH_BOUND = 4ll << 20;

class PreparedStatementLifetime : public ApiTest {
protected:
    // La mémoire vivante du tas (glibc) : ce que les préparations gardent, sans ce que malloc
    // garde de libéré.
    static int64_t liveHeapBytes() {
        malloc_trim(0);
        return static_cast<int64_t>(mallinfo2().uordblks);
    }

    static std::string distinctQuery(int64_t i) {
        return stringFormat("MATCH (a:person) WHERE a.ID = {} RETURN a.fName, a.age + {};", i, i);
    }

    // Prépare et exécute une fois `count` requêtes, la préparation détruite à chaque tour, et rend
    // la croissance de la mémoire vivante.
    int64_t growthOver(Connection& connection, int64_t count, bool distinct) {
        const auto before = liveHeapBytes();
        for (int64_t i = 0; i < count; i++) {
            auto prepared = connection.prepare(distinct ? distinctQuery(i) :
                                                          "MATCH (n) RETURN count(n);");
            EXPECT_TRUE(prepared->isSuccess()) << prepared->getErrorMessage();
            auto result = connection.execute(prepared.get());
            EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        }
        return liveHeapBytes() - before;
    }
};

// N requêtes distinctes préparées puis détruites sur une connexion : la mémoire vivante ne croît
// pas comme N. Un tour d'échauffement d'abord (les caches du catalogue, du tampon et de malloc se
// remplissent), puis la croissance de 3 000 préparations.
TEST_F(PreparedStatementLifetime, DistinctPreparationsDoNotGrowTheConnection) {
    growthOver(*conn, 300, true /*distinct*/);
    const auto growth = growthOver(*conn, 3000, true /*distinct*/);
    std::cerr << "  3000 distinct preparations: live heap grew by " << growth << " bytes ("
              << growth / 3000 << " per preparation)\n";
    EXPECT_LT(growth, PREPARED_GROWTH_BOUND)
        << "[check: prepared-statements-released] " << growth << " bytes";
}

// La recette du ticket : la même requête préparée N fois, la préparation détruite à chaque fois.
TEST_F(PreparedStatementLifetime, TheSameQueryPreparedManyTimesDoesNotGrowTheConnection) {
    growthOver(*conn, 300, false /*distinct*/);
    const auto growth = growthOver(*conn, 3000, false /*distinct*/);
    std::cerr << "  the same query prepared 3000 times: live heap grew by " << growth
              << " bytes (" << growth / 3000 << " per preparation)\n";
    EXPECT_LT(growth, PREPARED_GROWTH_BOUND)
        << "[check: prepared-statements-released] " << growth << " bytes";
}

// Des préparations qui survivent à leur connexion, puis à leur base : le cas d'un appelant Rust,
// dont le PreparedStatement n'est pas lié à la durée de vie de sa connexion. Leur destruction
// après coup ne touche rien de mort.
TEST_F(PreparedStatementLifetime, PreparationsOutliveTheirConnectionAndTheirDatabase) {
    const auto path = databasePath + "_outlive";
    std::filesystem::remove_all(path);
    std::vector<std::unique_ptr<PreparedStatement>> kept;
    {
        auto db = std::make_unique<Database>(path, SystemConfig());
        {
            Connection connection(db.get());
            ASSERT_TRUE(connection.query("CREATE NODE TABLE T(id INT64 PRIMARY KEY, v STRING);")
                            ->isSuccess());
            ASSERT_TRUE(connection.query("UNWIND range(0, 99) AS i CREATE (:T {id: i, v: 'x'});")
                            ->isSuccess());
            for (int64_t i = 0; i < 50; i++) {
                auto prepared = connection.prepare(
                    stringFormat("MATCH (t:T) WHERE t.id >= $low RETURN count(t) + {};", i));
                ASSERT_TRUE(prepared->isSuccess()) << prepared->getErrorMessage();
                if (i % 2 == 0) {
                    auto result = connection.execute(prepared.get(),
                        std::make_pair(std::string("low"), static_cast<int64_t>(i)));
                    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
                }
                kept.push_back(std::move(prepared));
            }
            // La connexion meurt ici, ses préparations vivantes.
        }
        // Une moitié meurt après la connexion, l'autre après la base.
        kept.erase(kept.begin(), kept.begin() + 25);
    }
    kept.clear();
    std::filesystem::remove_all(path);
    SUCCEED() << "[check: preparations-outlive-their-connection]";
}

// Une préparation exécutée sur une autre connexion que la sienne exécute sa requête. Les noms du
// magasin étaient des compteurs par connexion : la première préparation de A se nommait « 0 », et
// sur B, « 0 » désignait la première préparation de B, exécutée à sa place sans erreur.
TEST_F(PreparedStatementLifetime, APreparationRunsItsOwnQueryOnAnotherConnection) {
    Connection a(database.get());
    Connection b(database.get());
    auto onA = a.prepare("RETURN 1;");
    ASSERT_TRUE(onA->isSuccess()) << onA->getErrorMessage();
    auto onB = b.prepare("RETURN 2;");
    ASSERT_TRUE(onB->isSuccess()) << onB->getErrorMessage();
    auto result = b.execute(onA.get());
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_TRUE(result->hasNext());
    EXPECT_EQ(result->getNext()->getValue(0)->toString(), "1")
        << "[check: preparation-runs-its-own-query]";
}

} // namespace
