# rag3weaver for Node — alpha

**Work in progress.** `rag3weaver` is an agent runtime where an agent's whole
world lives in one embedded database: the data, the words, the vectors, the
code graph, the processing graphs, the tools, the sessions and the memories —
all declared, all addressable, all in one process. This package is its backend
for Node: the native binary `rag3weaver-backend` is installed by the
sub-package of your platform, and this package launches it and talks to it
over JSON lines. One API: the backend's.

It runs on [rag3db](https://github.com/L-Defraiteur/rag3db) (an embedded graph
database with HNSW vectors, a fork of Kuzu) and on
[lucivy](https://github.com/L-Defraiteur/lucivy/) (BM25 full text, in-process).
Native only; no model weights are ever in the package.

## State: `0.0.1-alpha`

| Platform | Sub-package | State |
|---|---|---|
| Linux x86_64 (glibc ≥ 2.28) | `rag3weaver-linux-x64-gnu` | published, tested end to end |
| macOS arm64 (Apple silicon) | `rag3weaver-darwin-arm64` | built and tested on our runners |
| Windows x64 (MSVC) | `rag3weaver-windows-x64` | built and tested on our runners |

Not there yet: Linux musl and arm64, macOS x64. The API is the backend's, in
JSON lines, without a comfort layer; it may change before 0.1.

## The idea

*Everything lives in one system, and everything in it is declared.*

A knowledge base is a declaration: its fields, its chunking, its relations, its
search signals. A processing step is a graph of typed nodes, written as a
Mermaid diagram. A tool for the agent is a node. One more declared thing, and
the system knows how to ingest it, index it, search it, and show what it did,
node by node. The templates shipped in this package (`templates/`) are those
declarations for a code workspace: the backend, and its tools.

## Quick start

```js
const { Backend, prepareManifest, templatesDir, vectorExtensionPath } = require('rag3weaver');
const fs = require('node:fs');

const manifest = prepareManifest(`${templatesDir()}/backends/code/backend.json`, {
  name: 'my-project',
  database: '/path/to/base.rag3db',
  vector_extension: vectorExtensionPath(),
  workspace: { root: '/path/to/project', commands: 'off', index: 'code' },
  tools: { run_command: undefined },
});
fs.writeFileSync('/path/to/backend.json', JSON.stringify(manifest));

const backend = await Backend.open('/path/to/backend.json');
const d = await backend.describe();            // tools, capabilities, warnings
await backend.call('index', { confirm: true }); // indexes in the background
const r = await backend.call('search_code', { query: 'where is the configuration read?', options: {} });
console.log(r.presentation);                   // the readable rendering
if (r.after) console.log(r.after.text);        // the Links section, as a tree
await backend.shutdown();
```

## What works

- Open a backend of code on a folder; list and read its files; `grep_files`;
  `search_code` (a scan of the files before any index, full text + meaning
  after); `usages` and `impact` on a symbol; `edit_file`, `estimate`, `schema`.
- Embeddings through a service: `RAG3WEAVER_EMBED_SERVICE=host:port[,host:port…]`
  in the environment, or `models.embed.address` in the manifest
  (`granite-278m`, 768 dimensions, by default).
- **The embedder is optional.** Without a service the backend starts and says
  so (`warnings` in `describe` and `index_state`), indexes full text only and
  leaves the vectors as a debt; search answers by the words and says that the
  dense signal is not available. When a service appears, the debt is paid
  without re-indexing.
- `journal` / `journal_read` (an agent's thread), `index_state`, a clean
  shutdown, and `mustReopen` on an error that requires reopening the database.

## What is missing

- Commands (`run_command`): the shipped template keeps them closed
  (`commands: 'off'`). The command sandbox is Linux only (Landlock); elsewhere
  the package sets `"sandbox": {"mode": "off"}` for you, and the commands stay
  closed unless `workspace.commands` opens them.
- A local embedding model: a service is required for vectors.
- A comfort layer over the JSON-lines API.

Test: `npm test` — an empty folder, three files, one search, no service.
