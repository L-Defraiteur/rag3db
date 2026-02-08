const fs = require("fs");
const path = require("path");
const process = require("process");

const RAG3DB_WASM_INDEX_PATH = path.join(__dirname, "node_modules", "rag3db-wasm", "index.js");
const RAG3DB_WASM_WORKER_PATH = path.join(__dirname, "node_modules", "rag3db-wasm", "rag3db_wasm_worker.js");
const DESTINATION_PATH = path.join(__dirname, "public");

if (!fs.existsSync(RAG3DB_WASM_INDEX_PATH) || !fs.existsSync(RAG3DB_WASM_WORKER_PATH)) {
    console.log("Rag3db WebAssembly module not found. Please run `npm i` to install the dependencies.");
    process.exit(1);
}

console.log("Copying Rag3db WebAssembly module to public directory...");
console.log(`Copying ${RAG3DB_WASM_INDEX_PATH} to ${DESTINATION_PATH}...`);
fs.copyFileSync(RAG3DB_WASM_INDEX_PATH, path.join(DESTINATION_PATH, "index.js"));
console.log(`Copying ${RAG3DB_WASM_WORKER_PATH} to ${DESTINATION_PATH}...`);
fs.copyFileSync(RAG3DB_WASM_WORKER_PATH, path.join(DESTINATION_PATH, "rag3db_wasm_worker.js"));
console.log("Done.");
