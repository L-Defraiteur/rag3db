// L'index de clé primaire sur disque qui grandit : aucune clé ne se perd à la division des cases.
//
// Quand un point de reprise agrandit un index non vide, HashIndex::splitSlots divise ses cases
// une à une et garde de côté les cases de débordement neuves. Si l'index fait plus que doubler,
// une case créée dans l'appel est redivisée dans le même appel ; quand sa chaîne passait par une
// case de débordement neuve, la division la lisait dans une mémoire que l'ajout suivant venait
// de libérer. Une clé restait sur disque sous une mauvaise empreinte : la ligne était là, un
// parcours la rendait, et l'index ne la retrouvait plus jamais — « Unable to find primary key
// value » au COPY de relations suivant. Trouvé par la session de l'arbre principal (5 octobre
// 2026) : une première indexation en deux paquets de 50 000 symboles.
//
// Il faut une part de l'index qui passe un niveau pendant l'appel et une case neuve qui déborde :
// l'événement est rare par case, d'où des tailles en dizaines de milliers de lignes. Les clés
// sont des chaînes longues — neuf entrées par case, contre quatorze pour un entier.

#include <filesystem>
#include <fstream>
#include <string>

#include <unistd.h>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::testing;

namespace {

class HashIndexSplitTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        csvPath = (std::filesystem::temp_directory_path() /
                   ("hash_index_split_" + std::to_string(::getpid()) + ".csv"))
                      .string();
    }

    void TearDown() override {
        std::filesystem::remove(csvPath);
        EmptyDBTest::TearDown();
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    int64_t single(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    static std::string key(int64_t i) {
        auto digits = std::to_string(i);
        return "une-cle-assez-longue-pour-deborder-" + std::string(9 - digits.size(), '0') +
               digits;
    }

    void copyKeys(int64_t first, int64_t count) {
        {
            std::ofstream csv(csvPath);
            for (int64_t i = first; i < first + count; i++) {
                csv << key(i) << "\n";
            }
        }
        ok("COPY T(k) FROM '" + csvPath + "' (header=false);");
    }

    // Chaque clé cherchée par l'index, une requête par clé ; le compte des introuvables et les
    // premières d'entre elles, pas la liste.
    void expectEveryKeyIsFound(int64_t numKeys, const std::string& when) {
        EXPECT_EQ(single("MATCH (n:T) RETURN count(*);"), numKeys) << when;
        int64_t numMissing = 0;
        std::string firstMissing;
        for (int64_t i = 0; i < numKeys; i++) {
            auto result = conn->query("MATCH (n:T {k: '" + key(i) + "'}) RETURN count(*);");
            if (!result->isSuccess() || !result->hasNext() ||
                result->getNext()->getValue(0)->getValue<int64_t>() != 1) {
                if (numMissing++ < 5) {
                    firstMissing += " " + std::to_string(i);
                }
            }
        }
        EXPECT_EQ(numMissing, 0) << when << " : keys the index does not find (first ones):"
                                 << firstMissing;
    }

    // Un petit COPY, puis un grand qui fait plus que doubler l'index ; chacun validé, avec son
    // point de reprise.
    void run(int64_t first, int64_t second) {
        ok("CREATE NODE TABLE T(k STRING PRIMARY KEY);");
        copyKeys(0, first);
        copyKeys(first, second);
        expectEveryKeyIsFound(first + second, "in the session");
        createDBAndConn();
        expectEveryKeyIsFound(first + second, "after reopening");
    }

    std::string csvPath;
};

// Les trois couples ont perdu une ou deux clés à chaque passe sur le moteur d'avant le correctif
// (bâti Release, huit passes sur huit) ; sous ASan, heap-use-after-free dans splitSlots.
TEST_F(HashIndexSplitTest, NoKeyIsLostWhenTheIndexGrowsFivefold) {
    run(47000, 200000);
}

TEST_F(HashIndexSplitTest, NoKeyIsLostWhenTheIndexGrowsSevenfold) {
    run(30000, 200000);
}

TEST_F(HashIndexSplitTest, NoKeyIsLostWhenTheIndexTriples) {
    run(47000, 100000);
}

// Les insertions ordinaires passent par la même division, au point de reprise. Par lots : une
// seule transaction de 200 000 lignes déborde le tampon de la base de test.
TEST_F(HashIndexSplitTest, NoKeyIsLostWhenInsertsGrowTheIndex) {
    ok("CALL auto_checkpoint=false;");
    ok("CREATE NODE TABLE T(k STRING PRIMARY KEY);");
    copyKeys(0, 47000);
    for (int64_t first = 47000; first < 247000; first += 10000) {
        {
            std::ofstream csv(csvPath);
            for (int64_t i = first; i < first + 10000; i++) {
                csv << key(i) << "\n";
            }
        }
        ok("LOAD FROM '" + csvPath + "' (header=false) CREATE (:T {k: column0});");
    }
    ok("CHECKPOINT;");
    expectEveryKeyIsFound(247000, "in the session");
    createDBAndConn();
    expectEveryKeyIsFound(247000, "after reopening");
}

} // namespace
