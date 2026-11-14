#include "common/test_hooks.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>

#include <atomic>
#include <chrono>
#include <thread>

#include "common/exception/runtime.h"

namespace rag3db {
namespace common {

TestDiePoint TestHooks::readFromEnvironment() {
    const auto* armed = std::getenv("RAG3DB_TEST_HOOKS"); // NOLINT(*-mt-unsafe)
    const auto* at = std::getenv("RAG3DB_TEST_DIE_AT");   // NOLINT(*-mt-unsafe)
    if (at == nullptr || at[0] == '\0') {
        return TestDiePoint::NONE;
    }
    if (armed == nullptr || std::strcmp(armed, "1") != 0) {
        fprintf(stderr,
            "TEST HOOK: RAG3DB_TEST_DIE_AT=%s ignored: RAG3DB_TEST_HOOKS=1 is not set\n", at);
        return TestDiePoint::NONE;
    }
    if (std::strcmp(at, "checkpoint-logged") == 0) {
        fprintf(stderr, "TEST HOOK: this process will _exit(%d) at checkpoint-logged\n",
            DIE_EXIT_CODE);
        return TestDiePoint::CHECKPOINT_LOGGED;
    }
    fprintf(stderr,
        "TEST HOOK: unknown point RAG3DB_TEST_DIE_AT=%s ignored (known: "
        "checkpoint-logged)\n",
        at);
    return TestDiePoint::NONE;
}

namespace {
std::atomic<uint8_t> armedFailPoint{static_cast<uint8_t>(TestFailPoint::NONE)};
std::atomic<uint32_t> armedFailDelayMs{0};
} // namespace

void TestHooks::armFailPointFromEnvironment() {
    const auto* armed = std::getenv("RAG3DB_TEST_HOOKS"); // NOLINT(*-mt-unsafe)
    const auto* at = std::getenv("RAG3DB_TEST_FAIL_AT");  // NOLINT(*-mt-unsafe)
    if (at == nullptr || at[0] == '\0') {
        return;
    }
    if (armed == nullptr || std::strcmp(armed, "1") != 0) {
        fprintf(stderr,
            "TEST HOOK: RAG3DB_TEST_FAIL_AT=%s ignored: RAG3DB_TEST_HOOKS=1 is not set\n", at);
        return;
    }
    if (std::strcmp(at, "rel-commit-between-directions") == 0) {
        fprintf(stderr,
            "TEST HOOK: the next relationship commit will fail between its two directions\n");
        armedFailPoint = static_cast<uint8_t>(TestFailPoint::REL_COMMIT_BETWEEN_DIRECTIONS);
        return;
    }
    fprintf(stderr,
        "TEST HOOK: unknown point RAG3DB_TEST_FAIL_AT=%s ignored (known: "
        "rel-commit-between-directions)\n",
        at);
}

void TestHooks::armFailPoint(TestFailPoint point, uint32_t delayMs) {
    armedFailDelayMs = delayMs;
    armedFailPoint = static_cast<uint8_t>(point);
}

void TestHooks::failOnceIfArmed(TestFailPoint point) {
    auto expected = static_cast<uint8_t>(point);
    if (point != TestFailPoint::NONE &&
        armedFailPoint.compare_exchange_strong(expected,
            static_cast<uint8_t>(TestFailPoint::NONE))) {
        if (const auto delay = armedFailDelayMs.load(); delay > 0) {
            std::this_thread::sleep_for(std::chrono::milliseconds(delay));
        }
        throw RuntimeException("TEST HOOK: failure injected at rel-commit-between-directions");
    }
}

void TestHooks::dieIfAt(TestDiePoint armed, TestDiePoint point) {
    if (armed != TestDiePoint::NONE && armed == point) {
        fflush(stderr);
        std::_Exit(DIE_EXIT_CODE);
    }
}

} // namespace common
} // namespace rag3db
