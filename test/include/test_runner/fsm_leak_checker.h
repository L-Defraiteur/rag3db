#pragma once

#include <cstdint>

namespace rag3db {
namespace main {
class Connection;
} // namespace main

namespace testing {

struct FSMLeakChecker {
    // Performs the whole leak check sequence; throws/asserts on failure
    static void checkForLeakedPages(main::Connection* conn);
};

} // namespace testing
} // namespace rag3db
