// Web Worker for rag3weaver WASM tests.
// Loads rag3db_wasm.js (which includes Weaver via embind) and runs tests.

importScripts("/rag3db_wasm.js");

function log(msg) {
  postMessage({ type: "log", msg });
}

function parse(jsonStr) {
  return JSON.parse(jsonStr);
}

// Generic async polling helper (shared by drain, search, etc.)
function pollAsync(Module, handle) {
  return new Promise((resolve, reject) => {
    let polls = 0;
    const poll = () => {
      polls++;
      if (Module.Weaver.asyncPoll(handle)) {
        const json = Module.Weaver.asyncResult(handle);
        resolve({ result: parse(json), polls });
      } else if (polls > 30000) {
        reject(new Error("async timeout after 30000 polls"));
      } else {
        setTimeout(poll, 1);
      }
    };
    poll();
  });
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
          title: { fieldType: "Text", titleFor: "main" },
          body: { fieldType: "Text" }
        }
      }
    },
    relations: {
      REFERENCES: {
        from: "Document",
        to: "Document"
      }
    },
    knowledgeBases: { main: {} },
    embeddingDim: 4
  });

  log("Creating Weaver with config: " + config);
  const weaver = new Module.Weaver(config, "");
  log("Test 2 (constructor): OK");
  results.constructorOk = true;

  // Test 3: Create entities — create() now returns int handle (>= 0)
  const docs = [
    { title: "Rust Guide", body: "Rust is a systems programming language" },
    { title: "Python Intro", body: "Python is great for data science" },
    { title: "JS Handbook", body: "JavaScript powers the modern web" },
  ];

  const handles = [];
  let createsOk = true;
  for (const doc of docs) {
    const handle = weaver.create("Document", JSON.stringify(doc));
    log("Test 3 (create): handle=" + handle);
    if (handle < 0) {
      createsOk = false;
      throw new Error("create failed: returned " + handle);
    }
    handles.push(handle);
  }
  results.createCount = docs.length;
  results.createsOk = createsOk;
  results.handles = handles;
  log("Test 3: " + docs.length + " entities created with handles " + JSON.stringify(handles));

  // Test 3b: getUuid before drain — should be pending
  const uuidBefore = parse(weaver.getUuid(handles[0]));
  log("Test 3b (getUuid before drain): " + JSON.stringify(uuidBefore));
  results.uuidBeforeDrain = uuidBefore;

  // Test 4: Drain — should include resolved array
  const drainRes = parse(weaver.drain());
  log("Test 4 (drain): processed=" + drainRes.processed + " failed=" + drainRes.failed +
      " resolved=" + (drainRes.resolved ? drainRes.resolved.length : 0));
  results.drain = drainRes;

  // Test 4b: getUuid after drain — should have UUID
  const uuidAfter = parse(weaver.getUuid(handles[0]));
  log("Test 4b (getUuid after drain): " + JSON.stringify(uuidAfter));
  results.uuidAfterDrain = uuidAfter;

  // Test 5: Count
  const countRes = parse(weaver.count("Document"));
  log("Test 5 (count): " + JSON.stringify(countRes));
  results.count = countRes.count;

  // Test 6: Async drain (drainAsyncStart / Poll / Result) with link
  log("Test 6: Creating 2 more entities + link for async drain...");
  const h4 = weaver.create("Document", JSON.stringify(
    { title: "Go Tutorial", body: "Go is great for concurrency" }
  ));
  const h5 = weaver.create("Document", JSON.stringify(
    { title: "C++ Reference", body: "C++ is a powerful systems language" }
  ));

  // Test link between two handles
  const linkResult = weaver.link(h4, h5, "REFERENCES", "{}");
  log("Test 6 (link): result=" + linkResult);
  results.linkResult = linkResult;

  log("Test 6: Starting async drain...");
  const drainHandle = weaver.drainAsyncStart();
  log("Test 6: Got handle, polling...");

  // Poll until done (non-blocking loop via generic helper)
  const { result: asyncDrainRes, polls: drainPolls } = await pollAsync(Module, drainHandle);
  log("Test 6: Async drain done after " + drainPolls + " polls");

  log("Test 6 (async drain): processed=" + asyncDrainRes.processed +
      " resolved=" + (asyncDrainRes.resolved ? asyncDrainRes.resolved.length : 0));
  results.asyncDrain = asyncDrainRes;

  // Test 7: Count after async drain — should be 5 total
  const countRes2 = parse(weaver.count("Document"));
  log("Test 7 (count after async): " + JSON.stringify(countRes2));
  results.countAfterAsync = countRes2.count;

  // Test 8: getUuid for async-drained handles
  const uuid4 = parse(weaver.getUuid(h4));
  const uuid5 = parse(weaver.getUuid(h5));
  log("Test 8 (getUuid async handles): h4=" + JSON.stringify(uuid4) + " h5=" + JSON.stringify(uuid5));
  results.asyncUuids = { h4: uuid4, h5: uuid5 };

  // Test 9: Search async (MockEmbedder → zero vectors, 0 results expected)
  log("Test 9: Starting async search...");
  const searchHandle = weaver.searchAsyncStart("main", "test query",
    JSON.stringify({ limit: 5, consistency: "immediate" }));
  const { result: searchRes, polls: searchPolls } = await pollAsync(Module, searchHandle);
  if (searchRes.error) {
    log("Test 9 (search async): ERROR: " + searchRes.error);
  } else {
    log("Test 9 (search async): done after " + searchPolls + " polls, " +
        "results=" + searchRes.results.length + " meta.query=" + searchRes.meta.query);
  }
  results.search = searchRes;

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
