// Embind wrapper for rag3weaver Rust static library.
// Exposes a `Weaver` class to JavaScript via emscripten embind.

#include <string>
#include <atomic>
#include <cstdint>
#include <emscripten/bind.h>
using namespace emscripten;

// Async callback type matching Rust's AsyncCallback
typedef void (*async_callback_t)(const char* result_json, uintptr_t user_data);

extern "C" {
    const char* rag3weaver_version();
    void* rag3weaver_catalog_new(const char* config_json, const char* db_path);
    void rag3weaver_catalog_destroy(void* ctx);
    const char* rag3weaver_create(void* ctx, const char* entity_type, const char* fields_json);
    const char* rag3weaver_drain(void* ctx);
    const char* rag3weaver_count(void* ctx, const char* entity_type);
    // Async drain: spawns on rayon pool, calls callback when done
    void rag3weaver_drain_async(const void* ctx, async_callback_t callback, uintptr_t user_data);
    // Threading validation tests
    const char* rag3weaver_test_threads();
    const char* rag3weaver_test_async_pool();
    const char* rag3weaver_test_rayon();
    const char* rag3weaver_test_tokio_mt();
}

// ── Async drain support ─────────────────────────────────────────────────

/// Holds the result of an in-flight async drain operation.
/// Allocated on the heap by drainAsyncStart(), freed by drainAsyncResult().
struct PendingDrain {
    std::string result;
    std::atomic<bool> done{false};
};

/// Callback invoked from a Rust rayon pool thread when drain completes.
/// Copies the result string (Rust may reuse the buffer) and signals completion.
static void drain_callback(const char* result_json, uintptr_t user_data) {
    auto* pd = reinterpret_cast<PendingDrain*>(user_data);
    pd->result = result_json ? std::string(result_json) : R"({"error":"null result"})";
    pd->done.store(true, std::memory_order_release);
}

// ── Weaver class ────────────────────────────────────────────────────────

class Weaver {
    void* ctx_;
public:
    Weaver(std::string config, std::string path) {
        ctx_ = rag3weaver_catalog_new(config.c_str(), path.c_str());
        if (!ctx_) {
            throw std::runtime_error("rag3weaver_catalog_new failed");
        }
    }

    ~Weaver() {
        if (ctx_) {
            rag3weaver_catalog_destroy(ctx_);
            ctx_ = nullptr;
        }
    }

    std::string create(std::string entityType, std::string fieldsJson) {
        const char* result = rag3weaver_create(ctx_, entityType.c_str(), fieldsJson.c_str());
        return result ? std::string(result) : R"({"error":"null result"})";
    }

    /// Synchronous drain (blocks until complete).
    std::string drain() {
        const char* result = rag3weaver_drain(ctx_);
        return result ? std::string(result) : R"({"error":"null result"})";
    }

    /// Start an async drain on the rayon pool. Returns a handle (opaque pointer).
    /// Use drainAsyncPoll() to check completion, drainAsyncResult() to get the result.
    uintptr_t drainAsyncStart() {
        auto* pd = new PendingDrain();
        rag3weaver_drain_async(ctx_, drain_callback, reinterpret_cast<uintptr_t>(pd));
        return reinterpret_cast<uintptr_t>(pd);
    }

    /// Check if an async drain has completed (non-blocking).
    static bool drainAsyncPoll(uintptr_t handle) {
        auto* pd = reinterpret_cast<PendingDrain*>(handle);
        return pd->done.load(std::memory_order_acquire);
    }

    /// Get the result of a completed async drain and free the handle.
    /// Must only be called after drainAsyncPoll() returns true.
    static std::string drainAsyncResult(uintptr_t handle) {
        auto* pd = reinterpret_cast<PendingDrain*>(handle);
        std::string result = std::move(pd->result);
        delete pd;
        return result;
    }

    std::string count(std::string entityType) {
        const char* result = rag3weaver_count(ctx_, entityType.c_str());
        return result ? std::string(result) : R"({"error":"null result"})";
    }

    static std::string version() {
        const char* v = rag3weaver_version();
        return v ? std::string(v) : "unknown";
    }

    static std::string testThreads() {
        const char* r = rag3weaver_test_threads();
        return r ? std::string(r) : R"({"error":"null result"})";
    }

    static std::string testAsyncPool() {
        const char* r = rag3weaver_test_async_pool();
        return r ? std::string(r) : R"({"error":"null result"})";
    }

    static std::string testRayon() {
        const char* r = rag3weaver_test_rayon();
        return r ? std::string(r) : R"({"error":"null result"})";
    }

    static std::string testTokioMt() {
        const char* r = rag3weaver_test_tokio_mt();
        return r ? std::string(r) : R"({"error":"null result"})";
    }
};

EMSCRIPTEN_BINDINGS(rag3weaver_wasm) {
    class_<Weaver>("Weaver")
        .constructor<std::string, std::string>()
        .function("create", &Weaver::create)
        .function("drain", &Weaver::drain)
        .function("drainAsyncStart", &Weaver::drainAsyncStart)
        .class_function("drainAsyncPoll", &Weaver::drainAsyncPoll)
        .class_function("drainAsyncResult", &Weaver::drainAsyncResult)
        .function("count", &Weaver::count)
        .class_function("version", &Weaver::version)
        .class_function("testThreads", &Weaver::testThreads)
        .class_function("testAsyncPool", &Weaver::testAsyncPool)
        .class_function("testRayon", &Weaver::testRayon)
        .class_function("testTokioMt", &Weaver::testTokioMt);
}
