#include "common/test_hooks.h"

#include <atomic>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
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
std::atomic<bool> poisonFresh{false};
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
    if (std::strcmp(at, "add-column-between-chunked-groups") == 0) {
        fprintf(stderr, "TEST HOOK: the next ADD COLUMN will fail after its first chunked group\n");
        armedFailPoint = static_cast<uint8_t>(TestFailPoint::ADD_COLUMN_BETWEEN_CHUNKED_GROUPS);
        return;
    }
    if (std::strcmp(at, "string-dictionary-append") == 0) {
        fprintf(stderr, "TEST HOOK: the next string appended to a dictionary will fail\n");
        armedFailPoint = static_cast<uint8_t>(TestFailPoint::STRING_DICTIONARY_APPEND);
        return;
    }
    fprintf(stderr,
        "TEST HOOK: unknown point RAG3DB_TEST_FAIL_AT=%s ignored (known: "
        "rel-commit-between-directions, add-column-between-chunked-groups, "
        "string-dictionary-append)\n",
        at);
}

void TestHooks::setPoisonFreshBuffers(bool poison) {
    poisonFresh = poison;
}

bool TestHooks::poisonFreshBuffers() {
    return poisonFresh.load(std::memory_order_relaxed);
}

void TestHooks::armFailPoint(TestFailPoint point, uint32_t delayMs) {
    armedFailDelayMs = delayMs;
    armedFailPoint = static_cast<uint8_t>(point);
}

static const char* failPointName(TestFailPoint point) {
    switch (point) {
    case TestFailPoint::REL_COMMIT_BETWEEN_DIRECTIONS:
        return "rel-commit-between-directions";
    case TestFailPoint::ADD_COLUMN_BETWEEN_CHUNKED_GROUPS:
        return "add-column-between-chunked-groups";
    case TestFailPoint::STRING_DICTIONARY_APPEND:
        return "string-dictionary-append";
    default:
        return "none";
    }
}

void TestHooks::failOnceIfArmed(TestFailPoint point) {
    // Le chemin chaud (appendString passe ici à chaque chaîne) : une lecture simple quand rien
    // n'est armé, avant l'échange atomique.
    if (armedFailPoint.load(std::memory_order_relaxed) != static_cast<uint8_t>(point)) {
        return;
    }
    auto expected = static_cast<uint8_t>(point);
    if (point != TestFailPoint::NONE && armedFailPoint.compare_exchange_strong(expected,
                                            static_cast<uint8_t>(TestFailPoint::NONE))) {
        if (const auto delay = armedFailDelayMs.load(); delay > 0) {
            std::this_thread::sleep_for(std::chrono::milliseconds(delay));
        }
        throw RuntimeException(
            std::string("TEST HOOK: failure injected at ") + failPointName(point));
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
