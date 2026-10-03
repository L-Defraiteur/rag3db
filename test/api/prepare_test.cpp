#include "api_test/api_test.h"

using namespace rag3db::common;
using namespace rag3db::testing;

static void checkTuple(rag3db::processor::FlatTuple* tuple, const std::string& groundTruth) {
    ASSERT_STREQ(tuple->toString().c_str(), groundTruth.c_str());
}

TEST_F(ApiTest, issueTest1) {
    conn->query("CREATE NODE TABLE T(id SERIAL, name STRING, PRIMARY KEY(id));");
    conn->query("CREATE (t:T {name: \"foo\"});");
    auto preparedStatement = conn->prepare("MATCH (t:T {id: $p}) return t.name;");
    auto result = conn->execute(preparedStatement.get(), std::make_pair(std::string("p"), 0));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "foo\n");
    ASSERT_FALSE(result->hasNext());
}

TEST_F(ApiTest, issueTest2) {
    conn->query("CREATE NODE TABLE NodeOne(id INT64, name STRING, PRIMARY KEY(id));");
    conn->query("CREATE NODE TABLE NodeTwo(id INT64, name STRING, PRIMARY KEY(id));");
    conn->query("CREATE Rel TABLE RelA(from NodeOne to NodeOne);");
    conn->query("CREATE Rel TABLE RelB(from NodeTwo to NodeOne, name String);");
    conn->query("CREATE (t: NodeOne {id:1, name: \"Alice\"});");
    conn->query("CREATE (t: NodeOne {id:2, name: \"Jack\"});");
    conn->query("CREATE (t: NodeTwo {id:3, name: \"Bob\"});");
    auto preparedStatement = conn->prepare("MATCH (a:NodeOne { id: $a_id }),"
                                           "(b:NodeTwo { id: $b_id }),"
                                           "(c: NodeOne{ id: $c_id } )"
                                           " MERGE"
                                           " (a)-[:RelA]->(c),"
                                           " (b)-[r:RelB { name: $my_param }]->(c)"
                                           " return r.name;");
    auto result = conn->execute(preparedStatement.get(), std::make_pair(std::string("a_id"), 1),
        std::make_pair(std::string("b_id"), 3), std::make_pair(std::string("c_id"), 2),
        std::make_pair(std::string("my_param"), "friend"));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "friend\n");
    ASSERT_FALSE(result->hasNext());
}

TEST_F(ApiTest, issueTest) {
    auto preparedStatement = conn->prepare("RETURN $1 + 1;");
    auto result =
        conn->execute(preparedStatement.get(), std::make_pair(std::string("1"), (int8_t)1));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "2\n");
    ASSERT_FALSE(result->hasNext());
}

TEST_F(ApiTest, MultiParamsPrepare) {
    auto preparedStatement = conn->prepare(
        "MATCH (a:person) WHERE a.fName STARTS WITH $n OR a.fName CONTAINS $xx RETURN COUNT(*)");
    auto result = conn->execute(preparedStatement.get(), std::make_pair(std::string("n"), "A"),
        std::make_pair(std::string("xx"), "ooq"));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "2\n");
    ASSERT_FALSE(result->hasNext());
}

TEST_F(ApiTest, PrepareBool) {
    auto preparedStatement =
        conn->prepare("MATCH (a:person) WHERE a.isStudent = $1 RETURN COUNT(*)");
    auto result = conn->execute(preparedStatement.get(), std::make_pair(std::string("1"), true));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "3\n");
    ASSERT_FALSE(result->hasNext());
}

TEST_F(ApiTest, PrepareInt) {
    auto preparedStatement = conn->prepare("MATCH (a:person) WHERE a.age = 35 RETURN a.age + $1");
    auto result =
        conn->execute(preparedStatement.get(), std::make_pair(std::string("1"), (int64_t)10));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "45\n");
    ASSERT_FALSE(result->hasNext());
}

TEST_F(ApiTest, PrepareDouble) {
    auto preparedStatement =
        conn->prepare("MATCH (a:person) WHERE a.age = 35 RETURN a.eyeSight + $1");
    auto result =
        conn->execute(preparedStatement.get(), std::make_pair(std::string("1"), (double)10.5));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "15.500000\n");
    ASSERT_FALSE(result->hasNext());
}

TEST_F(ApiTest, PrepareString) {
    auto preparedStatement =
        conn->prepare("MATCH (a:person) WHERE a.fName STARTS WITH $n RETURN COUNT(*)");
    auto result = conn->execute(preparedStatement.get(), std::make_pair(std::string("n"), "A"));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "1\n");
    ASSERT_FALSE(result->hasNext());
}

TEST_F(ApiTest, PrepareDate) {
    auto preparedStatement =
        conn->prepare("MATCH (a:person) WHERE a.birthdate = $n RETURN COUNT(*)");
    auto result = conn->execute(preparedStatement.get(),
        std::make_pair(std::string("n"), Date::fromDate(1900, 1, 1)));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "2\n");
    ASSERT_FALSE(result->hasNext());
}

TEST_F(ApiTest, PrepareTimestamp) {
    auto preparedStatement =
        conn->prepare("MATCH (a:person) WHERE a.registerTime = $n RETURN COUNT(*)");
    auto date = Date::fromDate(2011, 8, 20);
    auto time = Time::fromTime(11, 25, 30);
    auto result = conn->execute(preparedStatement.get(),
        std::make_pair(std::string("n"), Timestamp::fromDateTime(date, time)));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "1\n");
    ASSERT_FALSE(result->hasNext());
}

TEST_F(ApiTest, PrepareInterval) {
    auto preparedStatement =
        conn->prepare("MATCH (a:person) WHERE a.lastJobDuration = $n RETURN COUNT(*)");
    std::string intervalStr = "3 years 2 days 13 hours 2 minutes";
    auto result = conn->execute(preparedStatement.get(),
        std::make_pair(std::string("n"),
            Interval::fromCString(intervalStr.c_str(), intervalStr.length())));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "2\n");
    ASSERT_FALSE(result->hasNext());
}

TEST_F(ApiTest, PrepareDefaultParam) {
    auto preparedStatement = conn->prepare("RETURN to_int8($1)");
    auto result = conn->execute(preparedStatement.get(), std::make_pair(std::string("1"), "1"));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "1\n");
    ASSERT_FALSE(result->hasNext());
    preparedStatement = conn->prepare("RETURN size($1)");
    result = conn->execute(preparedStatement.get(), std::make_pair(std::string("1"), 1));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "1\n");
}

TEST_F(ApiTest, PrepareDefaultListParam) {
    auto preparedStatement = conn->prepare("RETURN [1, $1]");
    auto result =
        conn->execute(preparedStatement.get(), std::make_pair(std::string("1"), (int64_t)1));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "[1,1]\n");
    result = conn->execute(preparedStatement.get(), std::make_pair(std::string("1"), "as"));
    ASSERT_FALSE(result->isSuccess());
    ASSERT_STREQ(result->getErrorMessage().c_str(),
        "Binder exception: Expression $1 has data type STRING but expected INT64. Implicit cast is "
        "not supported.");
    preparedStatement = conn->prepare("RETURN [$1]");
    result = conn->execute(preparedStatement.get(), std::make_pair(std::string("1"), "as"));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "[as]\n");
    preparedStatement = conn->prepare("RETURN [to_int32($1)]");
    result = conn->execute(preparedStatement.get(), std::make_pair(std::string("1"), "10"));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "[10]\n");
}

TEST_F(ApiTest, PrepareDefaultStructParam) {
    auto preparedStatement = conn->prepare("RETURN {a:$1}");
    auto result = conn->execute(preparedStatement.get(), std::make_pair(std::string("1"), "10"));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "{a: 10}\n");
    result = conn->execute(preparedStatement.get(), std::make_pair(std::string("1"), 1));
    ASSERT_TRUE(result->isSuccess());
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "{a: 1}\n");
}

TEST_F(ApiTest, PrepareDefaultMapParam) {
    auto preparedStatement = conn->prepare("RETURN map([$1], [$2])");
    auto result = conn->execute(preparedStatement.get(), std::make_pair(std::string("1"), "10"),
        std::make_pair(std::string("2"), "abc"));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "{10=abc}\n");
}

TEST_F(ApiTest, PrepareDefaultUnionParam) {
    auto preparedStatement = conn->prepare("RETURN union_value(a := $1)");
    auto result = conn->execute(preparedStatement.get(), std::make_pair(std::string("1"), "10"));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "10\n");
}

TEST_F(ApiTest, PrepareLargeJoin) {
    auto preparedStatement = conn->prepare(
        " MATCH "
        "(:person)-[:knows]->(:person)-[:knows]->(:person)-[:knows]->(:person)-[:knows]->(:person)-"
        "[:knows]->(:person)-[:knows]->(:person)-[:knows]->(:person)-[:knows]->(:person)-[:knows]->"
        "(:person)-[:knows]->(:person)-[:knows]->(:person)-[:knows]->(:person)-[:knows]->(:person)-"
        "[:knows]->(:person)-[:knows]->(:person)-[:knows]->(:person) RETURN COUNT(*)");
    ASSERT_TRUE(preparedStatement->isSuccess());
}

TEST_F(ApiTest, ParamNotExist) {
    auto preparedStatement =
        conn->prepare("MATCH (a:person) WHERE a.fName STARTS WITH $n RETURN COUNT(*)");
    auto result = conn->execute(preparedStatement.get(), std::make_pair(std::string("a"), "A"));
    ASSERT_FALSE(result->isSuccess());
    ASSERT_STREQ("Parameter n not found.", result->getErrorMessage().c_str());
    result = conn->execute(preparedStatement.get(), std::make_pair(std::string("a"), "A"),
        std::make_pair(std::string("n"), "A"));
    ASSERT_TRUE(result->isSuccess());
    ASSERT_STREQ("1\n", result->getNext()->toString().c_str());
}

TEST_F(ApiTest, ParamTypeError) {
    auto preparedStatement =
        conn->prepare("MATCH (a:person) WHERE a.fName STARTS WITH $n RETURN COUNT(*)");
    auto result =
        conn->execute(preparedStatement.get(), std::make_pair(std::string("n"), (int64_t)36));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "0\n");
}

TEST_F(ApiTest, MultipleExecutionOfPreparedStatement) {
    auto preparedStatement =
        conn->prepare("MATCH (a:person) WHERE a.fName STARTS WITH $n RETURN a.ID, a.fName");
    auto result = conn->execute(preparedStatement.get(), std::make_pair(std::string("n"), "A"));
    auto groundTruth = std::vector<std::string>{"0|Alice"};
    ASSERT_EQ(groundTruth, TestHelper::convertResultToString(*result));
    result = conn->execute(preparedStatement.get(), std::make_pair(std::string("n"), "B"));
    groundTruth = std::vector<std::string>{"2|Bob"};
    ASSERT_EQ(groundTruth, TestHelper::convertResultToString(*result));
}

TEST_F(ApiTest, issueTest4) {
    auto preparedStatement = conn->prepare("RETURN CAST($1, 'STRING')");
    auto result = conn->execute(preparedStatement.get(),
        std::make_pair(std::string("1"), int128_t((int32_t)-123456789)));
    ASSERT_TRUE(result->hasNext());
    checkTuple(result->getNext().get(), "-123456789\n");
    ASSERT_FALSE(result->hasNext());
}

TEST_F(ApiTest, PrepareExport) {
    if (databasePath == "" || databasePath == ":memory:") {
        return;
    }
    auto newDBPath = TestHelper::getTempDir("export_db") + "/newdb";
    auto preparedStatement = conn->prepare("EXPORT DATABASE '" + newDBPath + '\'');
    auto result = conn->execute(preparedStatement.get());
    ASSERT_TRUE(result->isSuccess());
}

TEST_F(ApiTest, ParameterWith) {
    auto preparedStatement = conn->prepare("WITH $1 AS x RETURN x");
    ASSERT_TRUE(preparedStatement->isSuccess());
    auto result = conn->execute(preparedStatement.get(),
        std::make_pair(std::string("1"), std::string("abc")));
    auto groupTruth = std::vector<std::string>{"abc"};
    ASSERT_EQ(groupTruth, TestHelper::convertResultToString(*result));
}

// Un paramètre dans le prédicat d'un quantificateur : la liaison du prédicat
// copie l'expression, et ParsedParameterExpression::copy levait KU_UNREACHABLE
// (19 septembre 2026, filtres paramétrés de rag3weaver sur des listes).
TEST_F(ApiTest, ParameterInListQuantifier) {
    auto preparedStatement =
        conn->prepare("RETURN any(x IN [1, 2, 3] WHERE x = $v), all(x IN [1, 2, 3] WHERE x > $v)");
    ASSERT_TRUE(preparedStatement->isSuccess()) << preparedStatement->getErrorMessage();
    auto result = conn->execute(preparedStatement.get(),
        std::make_pair(std::string("v"), (int64_t)2));
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    auto groundTruth = std::vector<std::string>{"True|False"};
    ASSERT_EQ(groundTruth, TestHelper::convertResultToString(*result));
}

// Une liste de lignes passée en paramètre, dont un champ est nul : la forme
// `UNWIND $rows AS r CREATE (…)` des écritures par lots. Un client ne sait pas quel type
// donner à un NULL ; ce qu'il choisit ne doit pas empêcher de le poser dans une colonne typée.
static std::unique_ptr<Value> rowWithNullableField(int64_t id, std::unique_ptr<Value> v) {
    std::vector<StructField> fields;
    fields.emplace_back("id", LogicalType::INT64());
    fields.emplace_back("v", v->getDataType().copy());
    std::vector<std::unique_ptr<Value>> children;
    children.push_back(std::make_unique<Value>(id));
    children.push_back(std::move(v));
    return std::make_unique<Value>(LogicalType::STRUCT(std::move(fields)), std::move(children));
}

static std::unique_ptr<Value> listOfRows(std::vector<std::unique_ptr<Value>> rows) {
    auto type = LogicalType::LIST(rows[0]->getDataType().copy());
    return std::make_unique<Value>(std::move(type), std::move(rows));
}

static void insertRowsAndExpect(rag3db::main::Connection& conn, std::unique_ptr<Value> rows,
    const std::vector<std::string>& expected) {
    ASSERT_TRUE(conn.query("CREATE NODE TABLE T(id INT64 PRIMARY KEY, v INT64);")->isSuccess());
    auto prepared = conn.prepare("UNWIND $rows AS r CREATE (:T {id: r.id, v: r.v});");
    ASSERT_TRUE(prepared->isSuccess()) << prepared->getErrorMessage();
    std::unordered_map<std::string, std::unique_ptr<Value>> params;
    params["rows"] = std::move(rows);
    auto result = conn.executeWithParams(prepared.get(), std::move(params));
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    auto content = conn.query("MATCH (t:T) RETURN t.id, t.v ORDER BY t.id;");
    ASSERT_TRUE(content->isSuccess()) << content->getErrorMessage();
    ASSERT_EQ(expected, TestHelper::convertResultToString(*content, true /* checkOrder */));
}

TEST_F(ApiTest, RowsParameterWithAFieldNullEverywhereTypedAsString) {
    std::vector<std::unique_ptr<Value>> rows;
    rows.push_back(rowWithNullableField(1,
        std::make_unique<Value>(Value::createNullValue(LogicalType::STRING()))));
    rows.push_back(rowWithNullableField(2,
        std::make_unique<Value>(Value::createNullValue(LogicalType::STRING()))));
    insertRowsAndExpect(*conn, listOfRows(std::move(rows)), {"1|", "2|"});
}

TEST_F(ApiTest, RowsParameterWithAFieldNullEverywhereUntyped) {
    std::vector<std::unique_ptr<Value>> rows;
    rows.push_back(rowWithNullableField(1, std::make_unique<Value>(Value::createNullValue())));
    rows.push_back(rowWithNullableField(2, std::make_unique<Value>(Value::createNullValue())));
    insertRowsAndExpect(*conn, listOfRows(std::move(rows)), {"1|", "2|"});
}
