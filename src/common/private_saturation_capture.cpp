#include "common/private_saturation_capture.h"

#include <array>
#include <atomic>
#include <chrono>

namespace rag3db::common::private_saturation {
namespace {
using Clock = std::chrono::steady_clock;
static_assert(std::atomic<uint64_t>::is_always_lock_free);
std::atomic<bool> enabled{false};
std::atomic<uint64_t> firstState{0}, stage{0}, calls{0}, completedNs{0}, activePK{0};
std::atomic<uint64_t> activeTx{0}, writersTx{0}, oldestSnapshot{0};
std::atomic<uint64_t> rowsTransferred{0}, commitsPublished{0}, actorOK{0}, actorError{0}, writerOK{0}, writerError{0};
Clock::time_point origin;
thread_local Phase phase = Phase::NONE;
thread_local Site site = Site::UNKNOWN;
thread_local uint64_t transaction = 0;
struct Record {
    uint64_t at, readEnd, used, ddl, pkCalls, pkNs, pkActive, active, writers, oldest;
    uint64_t rows, commits, ok, errors, writerOK, writerError;
};
struct First {
    uint64_t reason = 0, phase = 0, site = 0, tx = 0, requested = 0;
    Record record{};
} first;
std::array<Record, 1280> samples{};
uint64_t sampleCount = 0, sampleOverflow = 0, lastSample = 0;
uint64_t now() noexcept {
    return std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now() - origin).count();
}
Record read(uint64_t used) noexcept {
    Record r{};
    r.at = now();
    r.used = used;
    r.ddl = stage.load(std::memory_order_relaxed);
    r.pkCalls = calls.load(std::memory_order_relaxed);
    r.pkNs = completedNs.load(std::memory_order_relaxed);
    r.pkActive = activePK.load(std::memory_order_relaxed);
    r.active = activeTx.load(std::memory_order_relaxed);
    r.writers = writersTx.load(std::memory_order_relaxed);
    r.oldest = oldestSnapshot.load(std::memory_order_relaxed);
    r.rows = rowsTransferred.load(std::memory_order_relaxed);
    r.commits = commitsPublished.load(std::memory_order_relaxed);
    r.ok = actorOK.load(std::memory_order_relaxed);
    r.errors = actorError.load(std::memory_order_relaxed);
    r.writerOK = writerOK.load(std::memory_order_relaxed);
    r.writerError = writerError.load(std::memory_order_relaxed);
    r.readEnd = now();
    return r;
}
void print(FILE* out, const char* kind, const Record& r) noexcept {
    // 16 champs scalaires, aucun texte de requete ou de cle.
    std::fprintf(out, "cx-saturation,%s,%llu,%llu,%llu,%llu,%llu,%llu,%llu,%llu,%llu,%llu,%llu,%llu,%llu,%llu,%llu,%llu\n",
        kind, (unsigned long long)r.at, (unsigned long long)r.readEnd,
        (unsigned long long)r.used, (unsigned long long)r.ddl,
        (unsigned long long)r.pkCalls, (unsigned long long)r.pkNs,
        (unsigned long long)r.pkActive, (unsigned long long)r.active,
        (unsigned long long)r.writers, (unsigned long long)r.oldest,
        (unsigned long long)r.rows, (unsigned long long)r.commits,
        (unsigned long long)r.ok, (unsigned long long)r.errors,
        (unsigned long long)r.writerOK, (unsigned long long)r.writerError);
}
} // namespace
bool armed() noexcept { return enabled.load(std::memory_order_relaxed); }
void arm(std::chrono::steady_clock::time_point scenarioStart) noexcept {
    // Une seule utilisation dans un processus neuf ; aucun reset pendant les acteurs.
    origin = scenarioStart;
    enabled.store(true, std::memory_order_release);
}
void ddl(uint64_t value) noexcept { if (armed()) stage.store(value); }
void transactions(uint64_t active, uint64_t writers, uint64_t oldestTS) noexcept {
    if (!armed()) return;
    activeTx.store(active, std::memory_order_relaxed);
    writersTx.store(writers, std::memory_order_relaxed);
    oldestSnapshot.store(oldestTS, std::memory_order_relaxed);
}
void transferred(uint64_t rows) noexcept {
    if (armed()) rowsTransferred.fetch_add(rows, std::memory_order_relaxed);
}
void published() noexcept {
    if (armed()) commitsPublished.fetch_add(1, std::memory_order_relaxed);
}
void actorResult(bool success, bool writer) noexcept {
    if (!armed()) return;
    (success ? actorOK : actorError).fetch_add(1, std::memory_order_relaxed);
    if (writer) (success ? writerOK : writerError).fetch_add(1, std::memory_order_relaxed);
}
void refuse(Refusal reason, uint64_t used, uint64_t requested) noexcept {
    if (!armed()) return;
    uint64_t expected = 0;
    if (!firstState.compare_exchange_strong(expected, 1, std::memory_order_acq_rel)) return;
    first.reason = static_cast<uint64_t>(reason);
    first.phase = static_cast<uint64_t>(phase);
    first.site = static_cast<uint64_t>(site);
    first.tx = transaction;
    first.requested = requested;
    first.record = read(used);
    firstState.store(2, std::memory_order_release);
}
void sample(uint64_t used) noexcept {
    if (!armed()) return;
    const auto tick = now();
    if (sampleCount && tick - lastSample < 1000000000ULL) return;
    lastSample = tick;
    if (sampleCount == samples.size()) { ++sampleOverflow; return; }
    samples[sampleCount++] = read(used);
}
Scope::Scope(Phase value, uint64_t tx) noexcept
    : enabled{armed()}, previous{private_saturation::phase}, previousTx{transaction},
      phase{value}, start{0} {
    if (!enabled) return;
    private_saturation::phase = value;
    if (tx) transaction = tx;
    if (value == Phase::PK) {
        calls.fetch_add(1, std::memory_order_relaxed);
        activePK.fetch_add(1, std::memory_order_relaxed);
        start = now();
    }
}
Scope::~Scope() noexcept {
    if (!enabled) return;
    if (phase == Phase::PK) {
        completedNs.fetch_add(now() - start, std::memory_order_relaxed);
        activePK.fetch_sub(1, std::memory_order_relaxed);
    }
    private_saturation::phase = previous;
    transaction = previousTx;
}
AllocationSite::AllocationSite(Site value) noexcept : previous{site} { site = value; }
AllocationSite::~AllocationSite() noexcept { site = previous; }
void finish(FILE* output) noexcept {
    // Appele par le thread du test apres join des acteurs ET de l'echantillonneur.
    enabled.store(false, std::memory_order_release);
    std::fprintf(output, "cx-saturation,schema,at_ns,read_end_ns,used_bytes,ddl,pk_calls,pk_completed_ns,pk_active,active_tx,writers_tx,oldest_snapshot_ts,transferred_rows,published_commits,actor_ok,actor_error,writer_ok,writer_error\n");
    for (uint64_t i = 0; i < sampleCount; ++i) print(output, "sample", samples[i]);
    const auto state = firstState.load(std::memory_order_acquire);
    std::fprintf(output, "cx-saturation,status,first_state=%llu,sample_count=%llu,overflow=%llu\n",
        (unsigned long long)state, (unsigned long long)sampleCount,
        (unsigned long long)sampleOverflow);
    if (state == 2) {
        std::fprintf(output, "cx-saturation,first_meta,reason=%llu,phase=%llu,site=%llu,tx=%llu,requested_bytes=%llu\n",
            (unsigned long long)first.reason, (unsigned long long)first.phase,
            (unsigned long long)first.site, (unsigned long long)first.tx,
            (unsigned long long)first.requested);
        print(output, "first", first.record);
    }
}
} // namespace rag3db::common::private_saturation
