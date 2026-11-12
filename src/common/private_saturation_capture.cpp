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
FILE* captureOutput = nullptr;
bool ioFailed = false, firstSaved = false;
void flushCapture() noexcept {
    if (!ioFailed && (std::fflush(captureOutput) != 0 || std::ferror(captureOutput))) {
        ioFailed = true;
    }
}
uint64_t now() noexcept {
    return std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now() - origin).count();
}
Record read(uint64_t used) noexcept {
    Record r{};
    r.at = now();
    r.used = used;
    r.ddl = stage.load(std::memory_order_relaxed);
    r.pkCalls = calls.load(std::memory_order_relaxed);
    // Exclut integralement la duree des appels encore actifs. pkActive > 0
    // interdit d'ecarter le cout PK sur le seul cumul des appels termines.
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
bool armed() noexcept { return enabled.load(std::memory_order_acquire); }
bool arm(std::chrono::steady_clock::time_point scenarioStart, FILE* output) noexcept {
    // Une seule utilisation dans un processus neuf ; aucun reset pendant les acteurs.
    samples.fill(Record{});
    first = First{};
    origin = scenarioStart;
    captureOutput = output;
    std::fprintf(captureOutput, "cx-saturation,schema,at_ns,read_end_ns,used_bytes,ddl,pk_calls,pk_completed_ns,pk_active,active_tx,writers_tx,oldest_snapshot_ts,transferred_rows,published_commits,actor_ok,actor_error,writer_ok,writer_error\n");
    flushCapture();
    if (ioFailed) return false;
    enabled.store(true, std::memory_order_release);
    return true;
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
namespace {
void saveFirst() noexcept {
    if (ioFailed || firstSaved || firstState.load(std::memory_order_acquire) != 2) return;
    std::fprintf(captureOutput, "cx-saturation,first_meta,reason=%llu,phase=%llu,site=%llu,tx=%llu,requested_bytes=%llu\n",
        (unsigned long long)first.reason, (unsigned long long)first.phase,
        (unsigned long long)first.site, (unsigned long long)first.tx,
        (unsigned long long)first.requested);
    print(captureOutput, "first", first.record);
    std::fprintf(captureOutput, "cx-saturation,first_saved\n");
    flushCapture();
    firstSaved = !ioFailed;
}
} // namespace
void sample(uint64_t used) noexcept {
    if (!armed()) return;
    // Unique ecrivain sampler ; aucun I/O dans refuse ou dans les scopes rollback.
    // La premiere capture est vidangee au prochain passage (cadence visee 100 ms),
    // meme si le prochain echantillon d'une seconde n'est pas encore du.
    saveFirst();
    const auto tick = now();
    if (sampleCount && tick - lastSample < 1000000000ULL) return;
    lastSample = tick;
    if (sampleCount == samples.size()) { ++sampleOverflow; return; }
    auto& r = samples[sampleCount++];
    r = read(used);
    if (!ioFailed) {
        print(captureOutput, "sample", r);
        flushCapture();
    }
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
bool finish() noexcept {
    // Apres joins seulement. Reprend l'unique role d'ecrivain du sampler arrete.
    enabled.store(false, std::memory_order_release);
    saveFirst();
    const auto state = firstState.load(std::memory_order_acquire);
    if (ioFailed) return false;
    const auto complete = state != 1 && sampleOverflow == 0 && (state != 2 || firstSaved);
    std::fprintf(captureOutput, "cx-saturation,status,complete=%u,first_state=%llu,sample_count=%llu,overflow=%llu\n",
        complete ? 1u : 0u, (unsigned long long)state, (unsigned long long)sampleCount,
        (unsigned long long)sampleOverflow);
    flushCapture();
    return complete && !ioFailed;
}
} // namespace rag3db::common::private_saturation
