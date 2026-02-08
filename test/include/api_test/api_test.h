#pragma once

#include "graph_test/base_graph_test.h"

namespace rag3db {
namespace testing {

class ApiTest : public BaseGraphTest {
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
