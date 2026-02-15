// Web Worker for rag3weaver WASM tests.
// Loads rag3db_wasm.js (which includes Weaver via embind) and runs tests.

importScripts("/rag3db_wasm.js");

function log(msg) {
  postMessage({ type: "log", msg });
}

function parse(jsonStr) {
  return JSON.parse(jsonStr);
}

// Helper: run a raw Cypher query via the existing Connection embind class
function queryRows(conn, sql) {
  const r = conn.query(sql);
  if (!r.isSuccess()) {
    const err = r.getErrorMessage();
    r.delete();
    throw new Error(err);
  }
  const rows = r.getAsJsArrayOfObjects();
  r.delete();
  return rows;
}

async function runTests(Module) {
  log("=== RAG3WEAVER WASM TESTS ===");

  const results = {};

  // Test 1: Weaver.version()
  const version = Module.Weaver.version();
  log("Test 1 (version): " + version);
  results.version = version;

  // Test 2: Create a Weaver instance (in-memory)
  const config = JSON.stringify({
    name: "test-weaver",
    entities: {
      Document: {
        fields: {
          title: { fieldType: "String" },
          body: { fieldType: "Text" }
        }
      }
    },
    embeddingDim: 4
  });

  log("Creating Weaver with config: " + config);
  const weaver = new Module.Weaver(config, "");
  log("Test 2 (constructor): OK");
  results.constructorOk = true;

  // Test 3: Create entities
  const docs = [
    { title: "Rust Guide", body: "Rust is a systems programming language" },
    { title: "Python Intro", body: "Python is great for data science" },
    { title: "JS Handbook", body: "JavaScript powers the modern web" },
  ];

  let createsOk = true;
  for (const doc of docs) {
    const res = parse(weaver.create("Document", JSON.stringify(doc)));
    log("Test 3 (create): " + JSON.stringify(res));
    if (res.error) {
      createsOk = false;
      throw new Error("create failed: " + res.error);
    }
  }
  results.createCount = docs.length;
  results.createsOk = createsOk;
  log("Test 3: " + docs.length + " entities created (UUIDs resolved after drain)");

  // Test 4: Drain
  const drainRes = parse(weaver.drain());
  log("Test 4 (drain): " + JSON.stringify(drainRes));
  results.drain = drainRes;

  // Test 5: Count
  const countRes = parse(weaver.count("Document"));
  log("Test 5 (count): " + JSON.stringify(countRes));
  results.count = countRes.count;

  // Test 6: Async drain (drainAsyncStart / Poll / Result)
  log("Test 6: Creating 2 more entities for async drain...");
  const asyncDocs = [
    { title: "Go Tutorial", body: "Go is great for concurrency" },
    { title: "C++ Reference", body: "C++ is a powerful systems language" },
  ];
  for (const doc of asyncDocs) {
    const res = parse(weaver.create("Document", JSON.stringify(doc)));
    if (res.error) throw new Error("create failed: " + res.error);
  }

  log("Test 6: Starting async drain...");
  const handle = weaver.drainAsyncStart();
  log("Test 6: Got handle, polling...");

  // Poll until done (non-blocking loop via Promise + setTimeout)
  const asyncDrainRes = await new Promise((resolve, reject) => {
    let polls = 0;
    const poll = () => {
      polls++;
      if (Module.Weaver.drainAsyncPoll(handle)) {
        const json = Module.Weaver.drainAsyncResult(handle);
        log("Test 6: Async drain done after " + polls + " polls");
        resolve(parse(json));
      } else if (polls > 30000) {
        reject(new Error("Async drain timeout after 30000 polls"));
      } else {
        setTimeout(poll, 1);
      }
    };
    poll();
  });

  log("Test 6 (async drain): " + JSON.stringify(asyncDrainRes));
  results.asyncDrain = asyncDrainRes;

  // Test 7: Count after async drain — should be 5 total
  const countRes2 = parse(weaver.count("Document"));
  log("Test 7 (count after async): " + JSON.stringify(countRes2));
  results.countAfterAsync = countRes2.count;

  // Cleanup
  weaver.delete();
  log("Weaver destroyed");

  results.done = true;
  log("=== ALL TESTS PASSED ===");
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
