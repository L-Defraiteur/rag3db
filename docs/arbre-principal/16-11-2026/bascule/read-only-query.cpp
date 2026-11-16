// Private migration evidence helper. Engine read-only mode, full rows, failures stay nonzero.
#include <exception>
#include <iostream>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    if (argc < 3) { std::cerr << "usage: read-only-query BASE QUERY...\n"; return 64; }
    try {
        SystemConfig config;
        config.readOnly = true;
        config.bufferPoolSize = 1ull << 30;
        Database database(argv[1], config);
        Connection connection(&database);
        auto build = connection.query("CALL BUILD_ID() RETURN *");
        if (!build->isSuccess()) { std::cerr << "BUILD_ID_ERROR\t" << build->getErrorMessage() << '\n'; return 1; }
        while (build->hasNext()) { std::cerr << "BUILD_ID\t" << build->getNext()->toString(); }
        for (int index = 2; index < argc; ++index) {
            auto result = connection.query(argv[index]);
            std::cout << "BEGIN_QUERY\t" << (index - 2) << '\n';
            if (!result->isSuccess()) { std::cerr << "QUERY_ERROR\t" << result->getErrorMessage() << '\n'; return 1; }
            unsigned long long rows = 0;
            while (result->hasNext()) { std::cout << "ROW\t" << result->getNext()->toString(); ++rows; }
            std::cout << "END_QUERY\t" << rows << '\n';
        }
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "OPEN_OR_QUERY_ERROR\t" << error.what() << '\n';
        return 1;
    }
}
