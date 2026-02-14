// Web Worker that runs all WASM operations off the main thread.
// This prevents deadlocks: the main thread stays free to coordinate
// pthread Web Workers spawned by Emscripten.

importScripts("/rag3db_wasm.js");

function log(msg) {
  postMessage({ type: "log", msg });
}

function query(conn, sql) {
  const r = conn.query(sql);
  if (!r.isSuccess()) {
    const err = r.getErrorMessage();
    r.delete();
    throw new Error(err);
  }
  return r;
}

function queryRows(conn, sql) {
  const r = query(conn, sql);
  const rows = r.getAsJsArrayOfObjects();
  r.delete();
  return rows;
}

function exec(conn, sql) {
  const r = query(conn, sql);
  r.delete();
}

async function phase1(Module) {
  log("=== PHASE 1: Create + Persist ===");

  Module.FS.mkdir("/database");
  Module.FS.mount(Module.FS.filesystems.IDBFS, {}, "/database");
  log("IDBFS mounted at /database");

  const config = new Module.SystemConfig();
  config.maxNumThreads = 2;
  const db = new Module.Database("/database/mydb", config);
  const conn = new Module.Connection(db);
  log("Database + connection created (maxThreads=2)");

  exec(conn, `CREATE NODE TABLE docs (
    id UINT64, title STRING, body STRING,
    embedding FLOAT[4], PRIMARY KEY(id)
  )`);
  log("Table 'docs' created");

  const docs = [
    [0, "Rust Programming", "Rust is a systems programming language focused on safety", [0.1, 0.2, 0.3, 0.4]],
    [1, "Python ML", "Python is great for machine learning and data science", [0.5, 0.6, 0.7, 0.8]],
    [2, "JavaScript Web", "JavaScript powers modern web applications and frameworks", [0.12, 0.22, 0.32, 0.42]],
    [3, "Database Systems", "Graph databases store data as nodes and relationships", [0.9, 0.1, 0.05, 0.02]],
  ];
  for (const [id, title, body, emb] of docs) {
    exec(conn, `CREATE (:docs {id: ${id}, title: '${title}', body: '${body}', embedding: [${emb}]})`);
  }
  log(docs.length + " documents inserted");

  exec(conn, "CALL CREATE_TANTIVY_INDEX('docs', ['title', 'body'])");
  log("Tantivy FTS index created");

  exec(conn, "CALL CREATE_VECTOR_INDEX('docs', 'emb_idx', 'embedding', metric := 'cosine')");
  log("Vector HNSW index created");

  // Test contains
  const containsRows = queryRows(conn,
    `CALL QUERY_TANTIVY_INDEX('docs', '{"type":"contains","field":"body","value":"programming"}', 10) RETURN node_id, score`
  );
  log("Test 1 (contains 'programming'): " + containsRows.length + " results");

  // Test fuzzy
  const fuzzyRows = queryRows(conn,
    `CALL QUERY_TANTIVY_INDEX('docs', '{"type":"fuzzy","field":"body","value":"programing","distance":1}', 10) RETURN node_id, score`
  );
  log("Test 2 (fuzzy 'programing'): " + fuzzyRows.length + " results");

  // Test phrase
  const phraseRows = queryRows(conn,
    `CALL QUERY_TANTIVY_INDEX('docs', '{"type":"phrase","field":"body","terms":["systems","programming"]}', 10) RETURN node_id, score`
  );
  log("Test 3 (phrase 'systems programming'): " + phraseRows.length + " results");

  // Test vector
  const vectorRows = queryRows(conn,
    `CALL QUERY_VECTOR_INDEX('docs', 'emb_idx', [0.12, 0.22, 0.32, 0.42], 3) RETURN node.id, node.title, distance`
  );
  log("Test 4 (vector cosine top 3): " + vectorRows.length + " results");
  for (const row of vectorRows) {
    log("  id=" + row["node.id"] + " title=" + row["node.title"] + " dist=" + row["distance"]);
  }

  const results = {
    contains: containsRows.length,
    fuzzy: fuzzyRows.length,
    phrase: phraseRows.length,
    vector: vectorRows.length,
    vectorIds: vectorRows.map(r => r["node.id"]),
  };

  conn.delete();
  db.delete();
  config.delete();
  log("Database closed");

  await new Promise((resolve, reject) => {
    Module.FS.syncfs(false, (err) => err ? reject(err) : resolve());
  });
  log("syncfs(false) done — data saved to IndexedDB");

  Module.FS.unmount("/database");
  log("IDBFS unmounted");

  results.phase1 = "done";
  log("=== PHASE 1 COMPLETE ===");
  return results;
}

async function phase2(Module) {
  log("=== PHASE 2: Reload + Verify ===");

  Module.FS.mkdir("/database");
  Module.FS.mount(Module.FS.filesystems.IDBFS, {}, "/database");

  await new Promise((resolve, reject) => {
    Module.FS.syncfs(true, (err) => err ? reject(err) : resolve());
  });
  log("syncfs(true) done — data loaded from IndexedDB");

  const files = Module.FS.readdir("/database");
  log("Files in /database: " + JSON.stringify(files));

  const config = new Module.SystemConfig();
  config.maxNumThreads = 2;
  const db = new Module.Database("/database/mydb", config);
  const conn = new Module.Connection(db);
  log("Database reopened");

  const containsRows = queryRows(conn,
    `CALL QUERY_TANTIVY_INDEX('docs', '{"type":"contains","field":"body","value":"programming"}', 10) RETURN node_id, score`
  );
  log("Test 5 (contains after reload): " + containsRows.length + " results");

  const fuzzyRows = queryRows(conn,
    `CALL QUERY_TANTIVY_INDEX('docs', '{"type":"fuzzy","field":"body","value":"programing","distance":1}', 10) RETURN node_id, score`
  );
  log("Test 6 (fuzzy after reload): " + fuzzyRows.length + " results");

  const vectorRows = queryRows(conn,
    `CALL QUERY_VECTOR_INDEX('docs', 'emb_idx', [0.12, 0.22, 0.32, 0.42], 3) RETURN node.id, node.title, distance`
  );
  log("Test 7 (vector after reload): " + vectorRows.length + " results");
  for (const row of vectorRows) {
    log("  id=" + row["node.id"] + " title=" + row["node.title"] + " dist=" + row["distance"]);
  }

  const allDocs = queryRows(conn, `MATCH (d:docs) RETURN d.id, d.title ORDER BY d.id`);
  log("Test 8 (all docs): " + allDocs.length + " documents");

  const results = {
    dbFiles: files,
    containsAfter: containsRows.length,
    fuzzyAfter: fuzzyRows.length,
    vectorAfter: vectorRows.length,
    vectorIdsAfter: vectorRows.map(r => r["node.id"]),
    allDocsAfter: allDocs.length,
  };

  conn.delete();
  db.delete();
  config.delete();
  Module.FS.unmount("/database");
  log("Database closed, IDBFS unmounted");

  results.phase2 = "done";
  log("=== PHASE 2 COMPLETE ===");
  return results;
}

(async () => {
  try {
    log("Loading WASM module in worker...");
    const Module = await rag3db();
    log("WASM module loaded, version: " + Module.getVersion());

    onmessage = async (e) => {
      try {
        const phase = e.data.phase;
        let results;
        if (phase === "1") {
          results = await phase1(Module);
        } else if (phase === "2") {
          results = await phase2(Module);
        }
        postMessage({ type: "results", results });
      } catch (err) {
        log("ERROR: " + err.message);
        postMessage({ type: "results", results: { error: err.message } });
      }
    };

    postMessage({ type: "ready" });
  } catch (err) {
    log("INIT ERROR: " + err.message);
    postMessage({ type: "results", results: { error: err.message } });
  }
})();
