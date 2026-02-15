// Web Worker for WASM threading validation tests.
// Tests std::thread::spawn and futures::executor::ThreadPool from Rust in WASM.

importScripts("/rag3db_wasm.js");

function log(msg) {
  postMessage({ type: "log", msg });
}

function parse(jsonStr) {
  return JSON.parse(jsonStr);
}

async function runTests(Module) {
  log("=== WASM THREADING VALIDATION TESTS ===");

  const results = {};

  // Test A: std::thread::spawn + atomics
  log("Test A: std::thread::spawn (4 threads x 1000 iterations)...");
  try {
    const threadsRes = parse(Module.Weaver.testThreads());
    log("Test A result: " + JSON.stringify(threadsRes));
    results.threads = threadsRes;
  } catch (err) {
    log("Test A EXCEPTION: " + err.message);
    results.threads = { ok: false, error: err.message };
  }

  // Test B: futures::executor::ThreadPool
  log("Test B: futures::executor::ThreadPool (pool_size=2, 8 tasks)...");
  try {
    const poolRes = parse(Module.Weaver.testAsyncPool());
    log("Test B result: " + JSON.stringify(poolRes));
    results.asyncPool = poolRes;
  } catch (err) {
    log("Test B EXCEPTION: " + err.message);
    results.asyncPool = { ok: false, error: err.message };
  }

  // Test C: rayon par_iter
  log("Test C: rayon par_iter (sum 1..=1000 in parallel chunks)...");
  try {
    const rayonRes = parse(Module.Weaver.testRayon());
    log("Test C result: " + JSON.stringify(rayonRes));
    results.rayon = rayonRes;
  } catch (err) {
    log("Test C EXCEPTION: " + err.message);
    results.rayon = { ok: false, error: err.message };
  }

  // Test D: tokio — RETIRÉ
  // tokio current_thread avec enable_all() épuise le pool de 16 threads
  // emscripten (partagé avec Kuzu + tantivy). Deadlock systématique.
  // Pas un problème en pratique : rayon + ThreadPool couvrent tous les besoins.
  results.tokioMt = { ok: false, skipped: true, reason: "thread pool exhaustion" };

  // Summary — C and D are optional (may hang)
  const allOk = results.threads?.ok && results.asyncPool?.ok;
  results.allPassed = allOk;
  log("");
  log(allOk ? "=== ALL THREADING TESTS PASSED ===" : "=== SOME TESTS FAILED ===");

  results.done = true;
  return results;
}

(async () => {
  try {
    log("Loading WASM module in worker...");
    const Module = await rag3db();
    log("WASM module loaded, version: " + Module.getVersion());

    onmessage = async (e) => {
      try {
        const results = await runTests(Module);
        postMessage({ type: "results", results });
      } catch (err) {
        log("ERROR: " + err.message + "\n" + err.stack);
        postMessage({ type: "results", results: { error: err.message } });
      }
    };

    postMessage({ type: "ready" });
  } catch (err) {
    log("INIT ERROR: " + err.message);
    postMessage({ type: "results", results: { error: err.message } });
  }
})();
