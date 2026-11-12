#pragma once

// DIAGNOSTIC PRIVE, jamais destine a une lib distribuee. Un processus, une base,
// armement avant les acteurs, lecture seulement apres leur join. Pas d'allocation.
#include <chrono>
#include <cstdint>
#include <cstdio>
#include "common/api.h"

namespace rag3db::common::private_saturation {
enum class Phase : uint64_t { NONE = 0, ROLLBACK = 1, PK = 2 };
enum class Site : uint64_t { UNKNOWN = 0, PAGE = 1, MALLOC_BUFFER = 2 };
enum class Refusal : uint64_t { POOL = 1, FRAME_GROUPS = 2 };
RAG3DB_API bool armed() noexcept;
RAG3DB_API void arm(std::chrono::steady_clock::time_point scenarioStart) noexcept;
RAG3DB_API void finish(FILE* output) noexcept;
RAG3DB_API void sample(uint64_t used) noexcept;
RAG3DB_API void ddl(uint64_t value) noexcept;
RAG3DB_API void refuse(Refusal reason, uint64_t used, uint64_t requested) noexcept;
RAG3DB_API void transactions(uint64_t active, uint64_t writers, uint64_t oldestTS) noexcept;
RAG3DB_API void transferred(uint64_t rows) noexcept;
RAG3DB_API void published() noexcept;
RAG3DB_API void actorResult(bool success, bool writer) noexcept;
struct RAG3DB_API Scope {
    explicit Scope(Phase phase, uint64_t tx = 0) noexcept;
    ~Scope() noexcept;
    Scope(const Scope&) = delete;
    Scope& operator=(const Scope&) = delete;
private:
    bool enabled;
    Phase previous;
    uint64_t previousTx;
    Phase phase;
    uint64_t start;
};
struct RAG3DB_API AllocationSite {
    explicit AllocationSite(Site value) noexcept;
    ~AllocationSite() noexcept;
private:
    Site previous;
};
} // namespace rag3db::common::private_saturation
