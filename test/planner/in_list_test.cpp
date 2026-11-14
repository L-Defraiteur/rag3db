// Un `IN` sur une liste constante ne coûte pas un parcours de la liste par ligne (ticket
// docs/everything-declarative/tickets/2026-11-14-un-in-sur-la-cle-primaire-balaie-toute-la-table.md).
// `n.k IN $liste` se planifie en balayage puis FILTER LIST_CONTAINS, et LIST_CONTAINS cherchait
// l'élément par un parcours de la liste : N clés sur N lignes, du N² (5 s pour 72 652 clés là où
// `UNWIND … MATCH {k: u}` prend 113 ms). Le témoin du coût compare les deux formes sur la même
// table ; les autres cas tiennent la sémantique de l'IN, la même avant et après.
#include <algorithm>
#include <chrono>
#include <string>
#include <unordered_map>
#include <vector>

#include "common/types/value/value.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::common;
using namespace rag3db::main;

namespace rag3db {
namespace testing {

class InListTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
    }
    void fill(int64_t n) {
        ASSERT_TRUE(conn->query("CREATE NODE TABLE T(k STRING, i INT64, PRIMARY KEY(k));")
                        ->isSuccess());
        ASSERT_TRUE(conn->query("COPY T FROM (UNWIND range(0, " + std::to_string(n - 1) +
                                ") AS x RETURN 'key-' + CAST(x AS STRING), x);")
                        ->isSuccess());
    }
    static std::unique_ptr<Value> stringList(const std::vector<std::string>& items) {
        std::vector<std::unique_ptr<Value>> children;
        children.reserve(items.size());
        for (const auto& item : items) {
            children.push_back(std::make_unique<Value>(item));
        }
        return std::make_unique<Value>(LogicalType::LIST(LogicalType::STRING()),
            std::move(children));
    }
    static std::unique_ptr<Value> intList(const std::vector<int64_t>& items) {
        std::vector<std::unique_ptr<Value>> children;
        children.reserve(items.size());
        for (const auto item : items) {
            children.push_back(std::make_unique<Value>(item));
        }
        return std::make_unique<Value>(LogicalType::LIST(LogicalType::INT64()), std::move(children));
    }
    // Le nombre de lignes rendues par `query` sous le paramètre $l.
    int64_t countWith(const std::string& query, std::unique_ptr<Value> list) {
        auto prepared = conn->prepare(query);
        EXPECT_TRUE(prepared->isSuccess()) << prepared->getErrorMessage();
        std::unordered_map<std::string, std::unique_ptr<Value>> params;
        params.emplace("l", std::move(list));
        auto result = conn->executeWithParams(prepared.get(), std::move(params));
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }
    double millisWith(const std::string& query, const std::vector<std::string>& keys,
        int64_t expected) {
        const auto start = std::chrono::steady_clock::now();
        EXPECT_EQ(countWith(query, stringList(keys)), expected) << query;
        return std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - start)
            .count();
    }
    static std::vector<std::string> keys(int64_t from, int64_t to) {
        std::vector<std::string> out;
        for (auto x = from; x < to; x++) {
            out.push_back("key-" + std::to_string(x));
        }
        return out;
    }
    static constexpr const char* IN_FORM = "MATCH (n:T) WHERE n.k IN $l RETURN count(*);";
    static constexpr const char* KEY_FORM =
        "UNWIND $l AS u MATCH (n:T {k: u}) RETURN count(*);";
};

TEST_F(InListTest, InOnAConstantListMatchesTheKeyForm) {
    fill(5000);
    // Toutes les clés, la moitié, des absentes, des doublons.
    EXPECT_EQ(countWith(IN_FORM, stringList(keys(0, 5000))), 5000);
    EXPECT_EQ(countWith(IN_FORM, stringList(keys(0, 2500))), 2500);
    auto mixed = keys(4990, 5010);
    EXPECT_EQ(countWith(IN_FORM, stringList(mixed)), 10);
    auto twice = keys(0, 100);
    auto again = keys(0, 100);
    twice.insert(twice.end(), again.begin(), again.end());
    EXPECT_EQ(countWith(IN_FORM, stringList(twice)), 100) << "a row counts once";
    EXPECT_EQ(countWith(IN_FORM, stringList({})), 0);
    EXPECT_EQ(countWith(IN_FORM, stringList({"key-7"})), 1);
}

TEST_F(InListTest, InOnAnIntegerColumnMatchesItsValues) {
    fill(5000);
    std::vector<int64_t> values;
    for (int64_t x = -50; x < 4000; x += 2) {
        values.push_back(x);
    }
    EXPECT_EQ(countWith("MATCH (n:T) WHERE n.i IN $l RETURN count(*);", intList(values)), 2000);
}

TEST_F(InListTest, AListThatChangesWithTheRowIsReadForEachRow) {
    fill(3000);
    // La liste dépend de la ligne : rien ne se garde d'une ligne à l'autre.
    auto r = conn->query("MATCH (n:T) WHERE n.k IN [n.k, 'absente', 'key-1'] RETURN count(*);");
    ASSERT_TRUE(r->isSuccess()) << r->getErrorMessage();
    EXPECT_EQ(r->getNext()->getValue(0)->getValue<int64_t>(), 3000);
    auto s = conn->query("MATCH (n:T) WHERE 'key-' + CAST(n.i + 1 AS STRING) IN "
                         "['key-' + CAST(n.i AS STRING), 'key-0'] RETURN count(*);");
    ASSERT_TRUE(s->isSuccess()) << s->getErrorMessage();
    EXPECT_EQ(s->getNext()->getValue(0)->getValue<int64_t>(), 0);
}

TEST_F(InListTest, InOnAConstantListIsNotQuadratic) {
    constexpr int64_t n = 40000;
    fill(n);
    const auto all = keys(0, n);
    millisWith(IN_FORM, all, n);
    millisWith(KEY_FORM, all, n);
    std::vector<double> in, key;
    for (auto i = 0; i < 3; i++) {
        in.push_back(millisWith(IN_FORM, all, n));
        key.push_back(millisWith(KEY_FORM, all, n));
    }
    std::sort(in.begin(), in.end());
    std::sort(key.begin(), key.end());
    const auto ratio = in[1] / std::max(key[1], 1.0);
    std::cerr << "[témoin] IN " << in[1] << " ms, UNWIND et clé " << key[1] << " ms, rapport "
              << ratio << "\n";
    EXPECT_LT(ratio, 4.0) << "[check: in-list-is-not-quadratic] IN " << in[1]
                          << " ms against the key form " << key[1] << " ms";
}

} // namespace testing
} // namespace rag3db
