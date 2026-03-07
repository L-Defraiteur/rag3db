// Worker: test setEmbedder() — swaps MockEmbedder for a real CandleEmbedder.
// Receives model config via postMessage, fetches model files, sets embedder,
// creates docs, drains, searches to verify real embeddings work.

importScripts("/rag3db_wasm.js");

const MODELS = {
  minilm: {
    hfBase: "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main",
    dim: 384,
    label: "all-MiniLM-L6-v2 (~23MB)",
  },
  multilingual: {
    hfBase: "https://huggingface.co/sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2/resolve/main",
    dim: 384,
    label: "paraphrase-multilingual-MiniLM-L12-v2 (~471MB)",
  },
};

function log(msg) {
  postMessage({ type: "log", msg });
}

function parse(jsonStr) {
  return JSON.parse(jsonStr);
}

function pollAsync(Module, handle) {
  return new Promise((resolve, reject) => {
    let polls = 0;
    const poll = () => {
      polls++;
      if (Module.Weaver.asyncPoll(handle)) {
        const json = Module.Weaver.asyncResult(handle);
        resolve({ result: parse(json), polls });
      } else if (polls > 60000) {
        reject(new Error("async timeout after 60000 polls"));
      } else {
        setTimeout(poll, 1);
      }
    };
    poll();
  });
}

async function runTest(Module, modelKey) {
  const model = MODELS[modelKey];
  if (!model) throw new Error("Unknown model: " + modelKey);

  const results = {};
  log("=== setEmbedder test: " + model.label + " ===");

  // 1. Create Weaver with MockEmbedder
  const config = JSON.stringify({
    name: "test-set-embedder",
    entities: {
      Article: {
        fields: {
          title: { fieldType: "Text", titleFor: "main" },
          body: { fieldType: "Text" },
        },
      },
    },
    knowledgeBases: { main: {} },
    embeddingDim: model.dim,
  });

  const weaver = new Module.Weaver(config, "");
  log("Weaver created (MockEmbedder, dim=" + model.dim + ")");
  results.constructorOk = true;

  // 2. Fetch model files
  log("Fetching model files: " + model.label + "...");
  const t0 = performance.now();
  const [configBuf, tokenizerBuf, weightsBuf] = await Promise.all([
    fetch(model.hfBase + "/config.json").then((r) => {
      if (!r.ok) throw new Error("config.json: " + r.status);
      return r.arrayBuffer();
    }),
    fetch(model.hfBase + "/tokenizer.json").then((r) => {
      if (!r.ok) throw new Error("tokenizer.json: " + r.status);
      return r.arrayBuffer();
    }),
    fetch(model.hfBase + "/model.safetensors").then((r) => {
      if (!r.ok) throw new Error("model.safetensors: " + r.status);
      return r.arrayBuffer();
    }),
  ]);
  const fetchMs = (performance.now() - t0).toFixed(0);
  log(
    "Model fetched in " + fetchMs + "ms (weights: " +
    (weightsBuf.byteLength / 1e6).toFixed(1) + "MB)"
  );
  results.fetchMs = Number(fetchMs);

  // 3. setEmbedder
  log("Calling setEmbedder()...");
  const t1 = performance.now();
  const setResult = parse(
    weaver.setEmbedder(
      new Uint8Array(configBuf),
      new Uint8Array(tokenizerBuf),
      new Uint8Array(weightsBuf)
    )
  );
  const setMs = (performance.now() - t1).toFixed(0);
  log("setEmbedder result: " + JSON.stringify(setResult) + " (" + setMs + "ms)");
  results.setEmbedder = setResult;
  results.setEmbedderMs = Number(setMs);

  if (!setResult.ok) {
    weaver.delete();
    results.done = false;
    return results;
  }

  // 4. Create docs with varying semantic content
  const docs = [
    { title: "Rust Programming", body: "Rust is a systems programming language focused on safety and performance" },
    { title: "French Cuisine", body: "La cuisine française est reconnue dans le monde entier pour sa finesse" },
    { title: "Machine Learning", body: "Deep learning uses neural networks with many layers to learn representations" },
  ];

  const handles = [];
  for (const doc of docs) {
    const h = weaver.create("Article", JSON.stringify(doc));
    if (h < 0) throw new Error("create failed: " + h);
    handles.push(h);
  }
  log("Created " + handles.length + " documents");
  results.createCount = handles.length;

  // 5. Drain (real embeddings now)
  log("Draining (computing real embeddings)...");
  const t2 = performance.now();
  const drainHandle = weaver.drainAsyncStart();
  const { result: drainRes, polls } = await pollAsync(Module, drainHandle);
  const drainMs = (performance.now() - t2).toFixed(0);
  log(
    "Drain done in " + drainMs + "ms (" + polls + " polls): processed=" +
    drainRes.processed + " failed=" + drainRes.failed
  );
  results.drain = drainRes;
  results.drainMs = Number(drainMs);

  // 6. Search — "programming language" should match Rust > French Cuisine
  log("Searching: 'programming language'...");
  const searchHandle = weaver.searchAsyncStart(
    "main",
    "programming language",
    JSON.stringify({ limit: 3, consistency: "immediate" })
  );
  const { result: searchRes } = await pollAsync(Module, searchHandle);
  log("Search results: " + searchRes.results.length + " hits");
  if (searchRes.results.length > 0) {
    for (const r of searchRes.results) {
      log("  score=" + r.score.toFixed(4) + " uuid=" + r.uuid);
    }
  }
  results.search = searchRes;

  // 7. Cleanup
  weaver.delete();
  log("Weaver destroyed");

  results.done = true;
  log("=== TEST COMPLETE ===");
  return results;
}

(async () => {
  try {
    log("Loading WASM module...");
    const Module = await rag3db();
    log("WASM loaded, version: " + Module.getVersion());

    onmessage = async (e) => {
      try {
        const modelKey = e.data.model || "minilm";
        const results = await runTest(Module, modelKey);
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
