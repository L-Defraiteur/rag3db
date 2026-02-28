// Embind wrapper for rag3weaver Rust static library.
// Exposes a `Weaver` class to JavaScript via emscripten embind.

#include <string>
#include <vector>
#include <atomic>
#include <cstdint>
#include <emscripten/bind.h>
#include <emscripten/val.h>
using namespace emscripten;

// Async callback type matching Rust's AsyncCallback
typedef void (*async_callback_t)(const char* result_json, uintptr_t user_data);

extern "C" {
    const char* rag3weaver_version();
    void* rag3weaver_catalog_new(const char* config_json, const char* db_path);
    void rag3weaver_catalog_destroy(void* ctx);
    int64_t rag3weaver_create(void* ctx, const char* entity_type, const char* fields_json);
    const char* rag3weaver_get_uuid(const void* ctx, int64_t handle);
    int64_t rag3weaver_link(void* ctx, int64_t from, int64_t to,
                            const char* rel_type, const char* props_json);
    const char* rag3weaver_drain(void* ctx);
    const char* rag3weaver_count(void* ctx, const char* entity_type);
    // Async drain: spawns on rayon pool, calls callback when done
    void rag3weaver_drain_async(const void* ctx, async_callback_t callback, uintptr_t user_data);
    // Async search: spawns on rayon pool, calls callback when done
    void rag3weaver_search_async(const void* ctx, const char* kb_name,
        const char* query, const char* options_json,
        async_callback_t callback, uintptr_t user_data);
    // Threading validation tests
    const char* rag3weaver_test_threads();
    const char* rag3weaver_test_async_pool();
    const char* rag3weaver_test_rayon();
    const char* rag3weaver_test_tokio_mt();
    // Candle embedding test (receives model bytes, returns embedding JSON)
    const char* rag3weaver_candle_embed(
        const uint8_t* config, size_t config_len,
        const uint8_t* tokenizer, size_t tokenizer_len,
        const uint8_t* weights, size_t weights_len,
        const char* text);
}

// ── Async support (generic for drain, search, etc.) ─────────────────────

/// Holds the result of an in-flight async operation.
/// Allocated on the heap by *AsyncStart(), freed by asyncResult().
struct PendingAsync {
    std::string result;
    std::atomic<bool> done{false};
};

/// Callback invoked from a Rust rayon pool thread when an async op completes.
/// Copies the result string (Rust may reuse the buffer) and signals completion.
static void async_callback(const char* result_json, uintptr_t user_data) {
    auto* pa = reinterpret_cast<PendingAsync*>(user_data);
    pa->result = result_json ? std::string(result_json) : R"({"error":"null result"})";
    pa->done.store(true, std::memory_order_release);
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

    int create(std::string entityType, std::string fieldsJson) {
        return (int)rag3weaver_create(ctx_, entityType.c_str(), fieldsJson.c_str());
    }

    std::string getUuid(int handle) {
        const char* r = rag3weaver_get_uuid(ctx_, (int64_t)handle);
        return r ? std::string(r) : R"({"error":"null result"})";
    }

    int link(int from, int to, std::string relType, std::string propsJson) {
        return (int)rag3weaver_link(ctx_, (int64_t)from, (int64_t)to,
                                    relType.c_str(), propsJson.c_str());
    }

    /// Synchronous drain (blocks until complete).
    std::string drain() {
        const char* result = rag3weaver_drain(ctx_);
        return result ? std::string(result) : R"({"error":"null result"})";
    }

    /// Start an async drain on the rayon pool. Returns a handle (opaque pointer).
    /// Use asyncPoll() to check completion, asyncResult() to get the result.
    uintptr_t drainAsyncStart() {
        auto* pa = new PendingAsync();
        rag3weaver_drain_async(ctx_, async_callback, reinterpret_cast<uintptr_t>(pa));
        return reinterpret_cast<uintptr_t>(pa);
    }

    /// Start an async search on the rayon pool. Returns a handle (opaque pointer).
    uintptr_t searchAsyncStart(std::string kb, std::string query, std::string optionsJson) {
        auto* pa = new PendingAsync();
        rag3weaver_search_async(ctx_, kb.c_str(), query.c_str(), optionsJson.c_str(),
            async_callback, reinterpret_cast<uintptr_t>(pa));
        return reinterpret_cast<uintptr_t>(pa);
    }

    /// Check if an async operation has completed (non-blocking).
    static bool asyncPoll(uintptr_t handle) {
        return reinterpret_cast<PendingAsync*>(handle)->done.load(std::memory_order_acquire);
    }

    /// Get the result of a completed async operation and free the handle.
    /// Must only be called after asyncPoll() returns true.
    static std::string asyncResult(uintptr_t handle) {
        auto* pa = reinterpret_cast<PendingAsync*>(handle);
        std::string result = std::move(pa->result);
        delete pa;
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

    /// Test candle embedding: receives model files as JS Uint8Arrays + text,
    /// creates CandleEmbedder in WASM, embeds text, returns JSON result.
    static std::string testCandleEmbed(val configArr, val tokenizerArr,
                                        val weightsArr, std::string text) {
        auto configVec = convertJSArrayToNumberVector<uint8_t>(configArr);
        auto tokenizerVec = convertJSArrayToNumberVector<uint8_t>(tokenizerArr);
        auto weightsVec = convertJSArrayToNumberVector<uint8_t>(weightsArr);

        const char* r = rag3weaver_candle_embed(
            configVec.data(), configVec.size(),
            tokenizerVec.data(), tokenizerVec.size(),
            weightsVec.data(), weightsVec.size(),
            text.c_str()
        );
        return r ? std::string(r) : R"({"error":"null result"})";
    }
};

EMSCRIPTEN_BINDINGS(rag3weaver_wasm) {
    class_<Weaver>("Weaver")
        .constructor<std::string, std::string>()
        .function("create", &Weaver::create)
        .function("getUuid", &Weaver::getUuid)
        .function("link", &Weaver::link)
        .function("drain", &Weaver::drain)
        .function("drainAsyncStart", &Weaver::drainAsyncStart)
        .function("searchAsyncStart", &Weaver::searchAsyncStart)
        .class_function("asyncPoll", &Weaver::asyncPoll)
        .class_function("asyncResult", &Weaver::asyncResult)
        .function("count", &Weaver::count)
        .class_function("version", &Weaver::version)
        .class_function("testThreads", &Weaver::testThreads)
        .class_function("testAsyncPool", &Weaver::testAsyncPool)
        .class_function("testRayon", &Weaver::testRayon)
        .class_function("testTokioMt", &Weaver::testTokioMt)
        .class_function("testCandleEmbed", &Weaver::testCandleEmbed);
}
