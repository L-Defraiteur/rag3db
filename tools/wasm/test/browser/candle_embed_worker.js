// Worker: test candle embedding in WASM.
// Fetches all-MiniLM-L6-v2 model files from HuggingFace, passes raw bytes
// to Rust CandleEmbedder via emscripten FFI, returns embedding result.

importScripts("/rag3db_wasm.js");

const HF_BASE =
  "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main";

async function run() {
  const Module = await rag3db();

  postMessage({ type: "status", msg: "WASM module loaded. Fetching model files (~23MB)..." });

  // Fetch the 3 model files in parallel
  const [configBuf, tokenizerBuf, weightsBuf] = await Promise.all([
    fetch(`${HF_BASE}/config.json`).then((r) => {
      if (!r.ok) throw new Error(`config.json: ${r.status}`);
      return r.arrayBuffer();
    }),
    fetch(`${HF_BASE}/tokenizer.json`).then((r) => {
      if (!r.ok) throw new Error(`tokenizer.json: ${r.status}`);
      return r.arrayBuffer();
    }),
    fetch(`${HF_BASE}/model.safetensors`).then((r) => {
      if (!r.ok) throw new Error(`model.safetensors: ${r.status}`);
      return r.arrayBuffer();
    }),
  ]);

  postMessage({
    type: "status",
    msg: `Model fetched (config: ${configBuf.byteLength}B, tokenizer: ${tokenizerBuf.byteLength}B, weights: ${(weightsBuf.byteLength / 1e6).toFixed(1)}MB). Running candle inference in WASM...`,
  });

  const config = new Uint8Array(configBuf);
  const tokenizer = new Uint8Array(tokenizerBuf);
  const weights = new Uint8Array(weightsBuf);

  // Call candle embedding via Rust WASM
  const resultStr = Module.Weaver.testCandleEmbed(
    config,
    tokenizer,
    weights,
    "Hello world"
  );
  const result = JSON.parse(resultStr);

  postMessage({ type: "result", ...result });
}

run().catch((err) =>
  postMessage({ type: "result", ok: false, error: err.message })
);
