// Brouillon de recherche, non livré : une colonne de chaînes longues, sur disque, relue par un
// balayage. AddressSanitizer a vu, dans la suite e2e_code de rag3weaver, une écriture hors
// tampon pendant DictionaryColumn::scan (la copie d'une page de données de chaîne dépasse le
// bloc de 32 768 octets du vecteur de sortie). Ce test cherche la forme minimale : des chaînes
// de plusieurs pages, des doublons consécutifs, plusieurs longueurs. À jouer avec
// MALLOC_CHECK_=3, qui fait avorter glibc sur un tas corrompu.

#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::testing;

namespace {

class LongStringScanScratch : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
    }
    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query.substr(0, 120) << " : "
                                         << result->getErrorMessage();
    }
};

TEST_F(LongStringScanScratch, Measure) {
    ok("CALL auto_checkpoint=false;");
    for (const int64_t length : {5000, 20000, 33000, 70000, 300000}) {
        for (const int64_t duplicates : {1, 3}) {
            const auto table = "T" + std::to_string(length) + "_" + std::to_string(duplicates);
            ok("CREATE NODE TABLE " + table + "(id INT64 PRIMARY KEY, kind STRING, body STRING);");
            // Des lignes dont le corps est long ; `duplicates` lignes consécutives portent le
            // même corps, puis le corps change.
            ok("UNWIND range(0, 39) AS i CREATE (:" + table +
                " {id: i, kind: 'k' + CAST(i % 4 AS STRING), body: repeat(CAST(i / " +
                std::to_string(duplicates) + " AS STRING) + '-', " + std::to_string(length / 3) +
                ")});");
            ok("CHECKPOINT;");
            auto result = conn->query("MATCH (n:" + table +
                                      ") WHERE n.kind = 'k1' RETURN n.id, size(n.body), n.body;");
            ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
            int64_t wrong = 0;
            while (result->hasNext()) {
                auto row = result->getNext();
                const auto id = row->getValue(0)->getValue<int64_t>();
                const auto body = row->getValue(2)->getValue<std::string>();
                std::string expected;
                const auto unit = std::to_string(id / duplicates) + "-";
                for (int64_t k = 0; k < length / 3; k++) {
                    expected += unit;
                }
                if (body != expected) {
                    wrong++;
                }
            }
            fprintf(stderr, "[chaines] longueur ~%ld, doublons %ld : %ld lignes fausses\n",
                static_cast<long>(length), static_cast<long>(duplicates), static_cast<long>(wrong));
            EXPECT_EQ(wrong, 0);
        }
    }
}

} // namespace
