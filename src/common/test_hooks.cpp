#include "common/test_hooks.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>

namespace rag3db {
namespace common {

TestDiePoint TestHooks::readFromEnvironment() {
    const auto* armed = std::getenv("RAG3DB_TEST_HOOKS"); // NOLINT(*-mt-unsafe)
    const auto* at = std::getenv("RAG3DB_TEST_DIE_AT");   // NOLINT(*-mt-unsafe)
    if (at == nullptr || at[0] == '\0') {
        return TestDiePoint::NONE;
    }
    if (armed == nullptr || std::strcmp(armed, "1") != 0) {
        fprintf(stderr, "TEST HOOK: RAG3DB_TEST_DIE_AT=%s ignored: RAG3DB_TEST_HOOKS=1 is not set\n",
            at);
        return TestDiePoint::NONE;
    }
    if (std::strcmp(at, "checkpoint-logged") == 0) {
        fprintf(stderr, "TEST HOOK: this process will _exit(%d) at checkpoint-logged\n",
            DIE_EXIT_CODE);
        return TestDiePoint::CHECKPOINT_LOGGED;
    }
    fprintf(stderr, "TEST HOOK: unknown point RAG3DB_TEST_DIE_AT=%s ignored (known: "
                    "checkpoint-logged)\n",
        at);
    return TestDiePoint::NONE;
}

void TestHooks::dieIfAt(TestDiePoint armed, TestDiePoint point) {
    if (armed != TestDiePoint::NONE && armed == point) {
        fflush(stderr);
        std::_Exit(DIE_EXIT_CODE);
    }
}

} // namespace common
} // namespace rag3db
