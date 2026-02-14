const http = require("http");
const fs = require("fs");
const path = require("path");

const PORT = process.env.PORT || 3333;
const WASM_DIR = path.join(__dirname, "..", "..", "package", "nodejs", "rag3db");
const STATIC_DIR = __dirname;

const MIME_TYPES = {
  ".html": "text/html",
  ".js": "application/javascript",
  ".wasm": "application/wasm",
  ".css": "text/css",
};

const server = http.createServer((req, res) => {
  // COOP/COEP headers required for SharedArrayBuffer (pthreads)
  res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
  res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");

  const pathname = new URL(req.url, "http://localhost").pathname;
  const url = pathname === "/" ? "/index.html" : pathname;
  const ext = path.extname(url);
  const mime = MIME_TYPES[ext] || "application/octet-stream";

  // Serve rag3db_wasm.js from the WASM build dir
  let filePath;
  if (url.startsWith("/rag3db_wasm")) {
    filePath = path.join(WASM_DIR, path.basename(url));
  } else {
    filePath = path.join(STATIC_DIR, url);
  }

  fs.readFile(filePath, (err, data) => {
    if (err) {
      res.writeHead(404);
      res.end(`Not found: ${url}`);
      return;
    }
    res.writeHead(200, { "Content-Type": mime });
    res.end(data);
  });
});

server.listen(PORT, () => {
  console.log(`Test server on http://localhost:${PORT}`);
});
