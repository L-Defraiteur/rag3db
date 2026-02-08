#pragma once

#include "graph_test/private_graph_test.h"

namespace rag3db {
namespace testing {

class PrivateApiTest : public DBTest {
public:
    void SetUp() override {
        BaseGraphTest::SetUp();
        createDBAndConn();
        initGraph();
    }

    std::string getInputDir() override {
        return TestHelper::appendRag3dbRootPath("dataset/tinysnb/");
    }
};

} // namespace testing
} // namespace rag3db
