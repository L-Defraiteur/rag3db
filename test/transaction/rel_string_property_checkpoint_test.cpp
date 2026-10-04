// Une propriété de chaîne d'une table de relations, après un point de reprise qui ne réécrit
// qu'une partie de ses régions.
//
// Le point de reprise d'une table de relations relit, région par région, les lignes déjà sur
// disque. Pour une colonne de chaînes, cette relecture partielle d'un segment range dans le
// bloc les seules chaînes de la région et renumérote les lignes. Elle numérotait les chaînes
// dans leur ordre d'apparition et les rangeait dans l'ordre du dictionnaire du disque : quand
// les deux ordres diffèrent, les lignes de la région échangent leurs chaînes. Aucune erreur ;
// les autres propriétés et l'autre sens de la table restent justes.
//
// Vu dans une base du produit le 4 octobre 2026 : 140 relations lues « nom » dans un sens et
// « fichier » dans l'autre. Il suffit de deux générations de relations aux chaînes distinctes,
// de la mise à jour d'une seule relation et d'un point de reprise.
//
// Les listes de chaînes passent par la même relecture partielle, sur les relations comme sur
// les nœuds (ListColumn::scanSegment ne relit que la plage de ses listes) : deux cas à la fin.

#include <sstream>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::testing;

namespace {

class RelStringPropertyCheckpointTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        ok("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);");
        ok("CREATE REL TABLE M(FROM Doc TO Doc, res STRING, line INT64);");
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    // Les lignes du résultat, une par ligne de texte.
    std::string rows(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        std::string out;
        while (result->isSuccess() && result->hasNext()) {
            out += result->getNext()->toString();
        }
        return out;
    }

    // Deux résultats égaux, ligne à ligne. Jamais EXPECT_EQ sur ces textes : quand ils
    // diffèrent, gtest en calcule la différence dans une table quadratique en nombre de lignes
    // (400 000 lignes : la mémoire de l'hôte, le 4 octobre 2026). Le compte et les premières
    // lignes suffisent.
    static void expectSameRows(const std::string& expected, const std::string& actual,
        const std::string& what) {
        if (expected == actual) {
            return;
        }
        std::istringstream left(expected), right(actual);
        std::string a, b, firsts;
        uint64_t numDifferent = 0, numLines = 0;
        while (true) {
            const bool hasA = static_cast<bool>(std::getline(left, a));
            const bool hasB = static_cast<bool>(std::getline(right, b));
            if (!hasA && !hasB) {
                break;
            }
            numLines++;
            if (!hasA || !hasB || a != b) {
                if (numDifferent++ < 5) {
                    firsts += "\n  attendu « " + (hasA ? a : std::string("<rien>")) +
                              " », lu « " + (hasB ? b : std::string("<rien>")) + " »";
                }
            }
        }
        ADD_FAILURE() << what << " : " << numDifferent << " lignes sur " << numLines
                      << " diffèrent" << firsts;
    }

    // Chaque relation avec sa chaîne, lue dans le sens direct puis dans le sens inverse.
    std::string forward() {
        return rows("MATCH (a:Doc)-[r:M]->(b:Doc) RETURN a.id, b.id, r.line, r.res "
                    "ORDER BY a.id, b.id, r.line;");
    }
    std::string backward() {
        return rows("MATCH (b:Doc)<-[r:M]-(a:Doc) RETURN a.id, b.id, r.line, r.res "
                    "ORDER BY a.id, b.id, r.line;");
    }

    // Deux générations de relations, un point de reprise après chacune : 2 000 « vieux » entre
    // 2 400 nœuds, puis 2 170 « fichier » dont 140 entre 200 nœuds neufs.
    void twoGenerations() {
        ok("UNWIND range(0, 2399) AS i CREATE (:Doc {id: i, name: 'ancien'});");
        ok("UNWIND range(0, 1999) AS i MATCH (a:Doc {id: i}) WITH a, i "
           "MATCH (b:Doc {id: (i * 11) % 2400}) CREATE (a)-[:M {res: 'vieux', line: -1}]->(b);");
        ok("CHECKPOINT;");
        ok("UNWIND range(2400, 2599) AS i CREATE (:Doc {id: i, name: 'neuf'});");
        ok("UNWIND range(0, 2029) AS i MATCH (a:Doc {id: i % 2400}) WITH a, i "
           "MATCH (b:Doc {id: (i * 7 + 3) % 2400}) "
           "CREATE (a)-[:M {res: 'fichier', line: i}]->(b);");
        ok("UNWIND range(0, 139) AS i MATCH (a:Doc {id: 2400 + i}) WITH a, i "
           "MATCH (b:Doc {id: 2400 + (i * 3 + 1) % 60}) "
           "CREATE (a)-[:M {res: 'fichier', line: 5000 + i}]->(b);");
        ok("CHECKPOINT;");
    }
};

// La mise à jour d'une seule relation, puis un point de reprise : aucune autre relation ne
// change de chaîne, dans aucun sens.
TEST_F(RelStringPropertyCheckpointTest, OneUpdateThenCheckpointKeepsEveryOtherString) {
    twoGenerations();
    ok("MATCH (a:Doc {id: 2400})-[r:M]->(b:Doc) SET r.res = 'nom';");
    const auto before = forward();
    expectSameRows(before, backward(), "avant le point de reprise, entre les deux formes");
    ok("CHECKPOINT;");
    expectSameRows(before, forward(), "après le point de reprise, première forme");
    expectSameRows(before, backward(), "après le point de reprise, seconde forme");
    EXPECT_EQ("0\n", rows("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE r.line >= 0 AND r.res = 'vieux' "
                          "RETURN count(*);"));
    EXPECT_EQ("0\n", rows("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE r.line < 0 AND r.res <> 'vieux' "
                          "RETURN count(*);"));
}

// Le lot de la base du produit : 140 mises à jour par MERGE … SET, puis le point de reprise.
TEST_F(RelStringPropertyCheckpointTest, ABatchOfUpdatesThenCheckpointKeepsBothDirectionsEqual) {
    twoGenerations();
    ok("UNWIND range(0, 139) AS i MATCH (a:Doc {id: 2400 + i}) WITH a, i "
       "MATCH (b:Doc {id: 2400 + (i * 3 + 1) % 60}) MERGE (a)-[r:M]->(b) SET r.res = 'nom';");
    const auto before = forward();
    expectSameRows(before, backward(), "avant le point de reprise, entre les deux formes");
    ok("CHECKPOINT;");
    expectSameRows(before, forward(), "après le point de reprise, première forme");
    expectSameRows(before, backward(), "après le point de reprise, seconde forme");
    EXPECT_EQ("140\n", rows("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE r.res = 'nom' RETURN count(*);"));
    EXPECT_EQ("140\n", rows("MATCH (b:Doc)<-[r:M]-(a:Doc) WHERE r.res = 'nom' RETURN count(*);"));
}

// Après la réouverture : ce que le point de reprise a écrit est ce qu'on relit.
TEST_F(RelStringPropertyCheckpointTest, TheStringsSurviveAReopen) {
    twoGenerations();
    ok("MATCH (a:Doc {id: 2400})-[r:M]->(b:Doc) SET r.res = 'nom';");
    const auto before = forward();
    ok("CHECKPOINT;");
    createDBAndConn();
    expectSameRows(before, forward(), "après le point de reprise, première forme");
    expectSameRows(before, backward(), "après le point de reprise, seconde forme");
}

// Une liste de chaînes sur des relations : mêmes deux générations, une mise à jour, un point de
// reprise. Avant le correctif, plus de la moitié des relations changeaient de liste, dans les
// deux rangements.
TEST_F(RelStringPropertyCheckpointTest, AListOfStringsOnRelsKeepsItsStrings) {
    ok("CREATE REL TABLE L(FROM Doc TO Doc, res STRING[], line INT64);");
    ok("UNWIND range(0, 2399) AS i CREATE (:Doc {id: i, name: 'ancien'});");
    ok("UNWIND range(0, 1999) AS i MATCH (a:Doc {id: i}) WITH a, i "
       "MATCH (b:Doc {id: (i * 11) % 2400}) "
       "CREATE (a)-[:L {res: ['vieux','vieux'], line: -1}]->(b);");
    ok("CHECKPOINT;");
    ok("UNWIND range(2400, 2599) AS i CREATE (:Doc {id: i, name: 'neuf'});");
    ok("UNWIND range(0, 2029) AS i MATCH (a:Doc {id: i % 2400}) WITH a, i "
       "MATCH (b:Doc {id: (i * 7 + 3) % 2400}) "
       "CREATE (a)-[:L {res: ['fichier','vieux'], line: i}]->(b);");
    ok("UNWIND range(0, 139) AS i MATCH (a:Doc {id: 2400 + i}) WITH a, i "
       "MATCH (b:Doc {id: 2400 + (i * 3 + 1) % 60}) "
       "CREATE (a)-[:L {res: ['fichier','vieux'], line: 5000 + i}]->(b);");
    ok("CHECKPOINT;");
    ok("MATCH (a:Doc {id: 2400})-[r:L]->(b:Doc) SET r.res = ['nom','fichier'];");
    const std::string fwd = "MATCH (a:Doc)-[r:L]->(b:Doc) RETURN a.id, b.id, r.line, r.res "
                            "ORDER BY a.id, b.id, r.line;";
    const std::string bwd = "MATCH (b:Doc)<-[r:L]-(a:Doc) RETURN a.id, b.id, r.line, r.res "
                            "ORDER BY a.id, b.id, r.line;";
    const auto before = rows(fwd);
    expectSameRows(before, rows(bwd), "avant le point de reprise, entre les deux formes");
    ok("CHECKPOINT;");
    expectSameRows(before, rows(fwd), "après le point de reprise, première forme");
    expectSameRows(before, rows(bwd), "après le point de reprise, seconde forme");
}

// Une liste de chaînes sur des nœuds : deux générations de 200 000 lignes aux listes
// alternées, des mises à jour éparses, un point de reprise. Avant le correctif, quatre lignes
// changeaient de liste.
TEST_F(RelStringPropertyCheckpointTest, AListOfStringsOnNodesKeepsItsStrings) {
    ok("CREATE NODE TABLE Tagged(id INT64 PRIMARY KEY, tags STRING[]);");
    ok("UNWIND range(0, 199999) AS i CREATE (:Tagged {id: i, "
       "tags: CASE WHEN i % 2 = 0 THEN ['b','a'] ELSE ['a','c','b'] END});");
    ok("CHECKPOINT;");
    ok("UNWIND range(200000, 399999) AS i CREATE (:Tagged {id: i, "
       "tags: CASE WHEN i % 3 = 0 THEN ['c','a'] ELSE ['b'] END});");
    ok("CHECKPOINT;");
    ok("MATCH (n:Tagged) WHERE n.id % 500 = 7 SET n.tags = ['a','b','c','a'];");
    const std::string all = "MATCH (n:Tagged) RETURN n.id, n.tags ORDER BY n.id;";
    const auto before = rows(all);
    ok("CHECKPOINT;");
    expectSameRows(before, rows(all), "après le point de reprise");
}

} // namespace
