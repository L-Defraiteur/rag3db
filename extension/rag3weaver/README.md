# rag3weaver

**Work in progress.** A Rust agent runtime and orchestrator where an agent's
whole world lives in one embedded database: the data, the words, the vectors,
the code graph, the processing graphs, the tools, the sessions and the memories
— all declared, all addressable, all in one process.

It runs on [rag3db](../../README.md) (graph + HNSW, embedded) and on
[lucivy](https://github.com/L-Defraiteur/lucivy/) 4.3.2 (BM25, in-process). Native only.

## The idea

*Everything lives in one system, and everything in it is declared.*

A knowledge base is a declaration (`EntityConfig`): its fields, its chunking,
its relations, its search signals. A processing step is a graph of typed nodes,
written as a Mermaid diagram. A tool for the agent is a node. A memory is a
template. One more declared thing, and the system knows how to ingest it, index
it, search it, and show what it did, node by node.

The aim, in one image: *you talk to your code or to your site, and it changes in
front of you* — no report to read, the screen shows what happens, and everything
is clickable. The long version (French):
[`visions/00-vision-generale.md`](visions/00-vision-generale.md).

## What exists today

| Layer | What it is | State |
|---|---|---|
| ingestion | records → chunks → rows → links → embeddings, by batches, resumable after a crash | delivered, measured on benches |
| hybrid search | BM25 (lucivy) + dense HNSW + learned sparse, rank fusion with exact matches first, per-signal scores, highlighted passages, optional cross-encoder reranking | delivered |
| dataflow | a DAG runtime with typed ports, 56 built-in nodes (71 with `code`), Mermaid templates, sub-graphs called once or per item, execution reports, checkpoints and undo | delivered |
| code graph | `codeparsers` (submodule, 13 languages): scopes, symbols, usages, impact, links between files, C and C++ calls resolved through their `#include`s; a synchronizer that re-indexes what changed | delivered |
| the agent's backend | `rag3weaver-backend` and `rag3weaver-chat`: search, read, grep, edit, run, sections after each tool; models through an OpenAI-compatible API, the Anthropic Messages API, or a local server | delivered, in daily use on this repository |
| the teammate | an agent that works alone on a task branch, step by step, and talks to its supervisor through a mailbox | delivered |
| Claude Code and Codex | hooks that bring the memory into a Claude Code session, a mailbox and a channel shared by sessions, Codex threads and teammates | delivered; Codex delivery tested against a stand-in, not yet against Codex itself |
| the IDE and the terminal | a React IDE served by the host; `rag3weaver-tty`, a terminal client that also works over ssh | delivered |
| embeddings | granite-embedding-278m by default (107m on a first large index or a weak GPU), MiniLM, multilingual MiniLM, BGE-M3 — all on burn/wgpu (Vulkan); an embedding service and a daemon (`rag3daemon`, `rag3weaver-embeddings`) | delivered |
| multi-tenant | `org` × `project` on every row, one FTS and sparse index per cell | delivered |
| other backends | `SchemaDialect` trait with a PostgreSQL implementation (`postgres` feature); declared Plans render a read in both dialects | partial — the contract a dialect must honour is not written yet |
| long memory | the `memory` template: cards as Markdown files, rebuilt from their folder if the database is lost; references, states, `cited_by`, `whois` | delivered; an archivist that sorts and merges cards is in progress |
| context cards, declared backend and views, talking to a page | | visions — nothing coded |

## Measured

First index of this repository (about 7 000 files, 80 000 scopes, 1.1 million
relations), on disk, with embeddings:

| | September 2026 | 8 October 2026 |
|---|---|---|
| duration | 753 s | **78 s** (median of three runs) |
| peak memory | | 9 Go |
| first searchable results | at the end | after 8–10 s |

The 78 s comes from three settings that are **the defaults since 10 October
2026**: full text stored beside the database as files (instead of blobs inside
it) for a new database, one transaction per batch, and batches of 2 048 files.
Each can be turned back: `RAG3WEAVER_FTS=blobs`, `RAG3WEAVER_TX_PAR_PAQUET=0`,
`RAG3WEAVER_BATCH_FILES=64`. Numbers come from a held measurement on one machine
(AMD Radeon 8060S); they say "faster", not "fast everywhere".

## Quick start

```rust
let mut catalog = Catalog::new(conn, embedder, config);
catalog.initialize().await?;

catalog.register_entity("Product", EntityConfig {
    fields: vec![
        SimpleFieldDef::new("name", FieldType::String, FieldRole::Title),
        SimpleFieldDef::new("description", FieldType::Text, FieldRole::Content),
    ],
    signals: SearchSignals::BM25 | SearchSignals::VECTOR,
    ..Default::default()
}).await?;

catalog.ingest_entities("Product", vec![
    btree_map!{ "name" => "Rust Book", "description" => "A guide to Rust..." },
]).await?;

let response = catalog.search("Product", "rust ownership", SearchOptions::default()).await?;
for r in &response.results {
    println!("{} (score={:.4})", r.uuid, r.score);
}
```

A search is itself a graph. The same thing as a Mermaid template
(`templates/simple_hybrid_search.mmd`):

```mermaid
graph LR
    source["SearchSourceNode(target_name='$target', query='$query')"]
    vector["VectorSearchNode(limit='$limit')"]
    bm25["BM25SearchNode(limit='$limit')"]
    fuse["FuseResultsNode"]
    resolve["ResolveParentNode"]

    source -->|query| vector
    source -->|query| bm25
    source -->|query:query| resolve
    vector -->|results:vector| fuse
    bm25 -->|results:bm25| fuse
    fuse -->|results| resolve
```

```rust
let (output, report) = runtime.execute_with_report(&mut graph).await?;
// report.nodes: name, status, duration_ms, output ports, metrics
// report.edges: from, to, value summary
```

## Search

Three signals, combinable as bitflags (`BM25`, `VECTOR`, `SPARSE`), fused by
reciprocal rank (or weighted), with each signal either feeding candidates or
boosting them. BM25 has six query modes — `Auto` (the default), `Contains`
(substring across tokens, fuzzy, relaxed separators), `ContainsSplit`, `Regex`,
`Parse` (boolean syntax), `Symbol` (exact, separators included: `foo->bar`,
`std::sync::Arc<Mutex<T>>`).

With `Auto`, a query goes to lucivy twice, in parallel: whole, as one substring
at distance 1, and word by word. What the whole query found is ranked ahead of
everything the fusion ranked, so a sentence copied from a file brings that file
back first. A long query is bounded and says so in `SearchMeta.warnings`: past
900 characters the whole query is searched without fuzziness, and the word
search keeps the 64 longest terms. lucivy treats `'` and `’` as the same
apostrophe since 4.3.2.

Each result says what each signal made of it — score and rank in that signal's
own list (`signals`), the signal that weighs most in its fused rank
(`bestSignal`), and the scale of its chunk's score (`chunk.signal`); the chunk
shown is the one from that signal. These scores are not comparable with each
other. Each chunk carries its highlighted passages (`highlights`, in UTF-16
units of the chunk text): the spans lucivy found, and, with
`options.highlights = "sparse"`, passages of a chunk found by the dense signal
alone, from the positions the bge-m3 service weighs (no tokenizer or model in
the host; the query words instead when the service does not answer). Engine
warnings come back in `SearchMeta.warnings` on every search; the `search_code`
tool also returns its metadata as JSON.

When a signal is unavailable or fails, the search falls back on the others and
says so in its status (`signal is not available` / `signal failed`) — never a
silent empty result. A vector index that is being rebuilt, missing or
incomplete makes the dense signal step aside, by name, while the words answer.
At the end of each index run, every full-text index counts its documents
against the rows and reports a gap. A filtered search reads all its signals
from the same committed state, and a filter applies to the learned-sparse
signal too.

Full text is stored either **as files in `<base>.fts/`** (the default for a new
database on disk: lower memory, faster reopen; the folder is part of the database,
copy and back it up with it) or **in the database as blobs** (an existing database
that has them keeps them, and `RAG3WEAVER_FTS=blobs` asks for them). A database in
memory keeps its full text in itself.

The dense index prunes its HNSW graph with the classic heuristic since 10 October
2026 (engine `9a2818df9`): no row is left unreachable by its own vector, at +3 to 5 %
per query. **An existing index keeps its old graph** (the format does not change, only
the neighbour lists a write touches are rewritten); to get the new pruning, recreate it
(`DROP_VECTOR_INDEX`, then `CREATE_VECTOR_INDEX`).

## The code graph

With the `code` feature, a repository becomes a graph: files, scopes, symbols,
`DEFINES`, `CONSUMES`, `REFERENCES`, `MENTIONS` edges, with a `resolution` mark on each
edge saying how it was found (by path, by name, guessed). A C or C++ call is
resolved only to what its file includes. The synchronizer
indexes by batches, marks what it has proven, and on restart redoes only what is
incomplete. Tools built on it: `usages`, `impact` (what depends on this), links
rendered as a tree.

## The agent

rag3weaver is not a library an agent calls; it is where the agent runs.

- **The loop** (`agent.rs`): generate, execute the requested tools, feed the
  results back, start again — until the model answers or a bound bites. It is
  short because everything it assembles already exists.
- **Tools are nodes.** A tool definition for the model is generated from a
  node's schema (`tools.rs`): name, description, typed parameters. Nothing is
  written twice, and a tool call is a graph execution, with its report.
- **A backend is a manifest** (`backend.rs`): payload schemas and the graph
  tools it chooses to expose. No domain vocabulary in the engine; the code
  backend (`backend_code.rs`) declares its `workspace` — which files, under
  which root, read-only or not, with which command gate — so the same engine
  serves a cloud agent on a snapshot and a desktop agent on the real tree.
- **Sessions, postures, work domain** (`session.rs`, `postures.rs`,
  `work_domain.rs`): what is kept from one turn to the next and what stops
  being paid for; who stayed silent towards whom in a multi-party
  conversation; the dispersed set of repositories and notes an agent has in
  view.
- **Event-driven graphs** (`dataflow/reactor.rs`, `react_nodes.rs`): a graph
  declares what it reacts to on the bus (`each` / `batch` / `debounce`) and
  runs when it happens — the piece that lets a change in the data trigger
  work.
- **The model is a service**, like the embedder: `LlmNode`, a `decider`
  (a text, a closed list of options, a probability per option), an
  OpenAI-compatible client, the Anthropic Messages API (`anthropic-llm`,
  `"protocol": "anthropic"` in `models.llm`) or a local server, all behind the
  same `Llm` trait; streaming is in the trait, and a model may declare its
  reasoning budget (`models.llm.reasoning`).

### Working alone: steps, a judge, a memory

An agent that works on its own is not one long turn. It writes a **mini-plan**,
does one item per **step**, declares the step done or failed, and is relaunched
with what arrived meanwhile. Every step is a **run** in the host's journal, and a
**judge** reads each run: one verdict on the session, one on every tool it
called. Both land in the memory as fiches, where the trend across runs is what
decides the next move. The supervisor (a person, or an orchestrating agent) only
ever writes to a mailbox.

```mermaid
flowchart TD
    S["Supervisor<br/>a person, or an orchestrating agent"] -->|"request, answers to blockers"| B[("Mailbox<br/>one file per task + a bus topic")]
    B -->|"opens or wakes the task"| A["Agent on the task branch<br/>read, search, edit; no commands"]
    A -->|"writes, edits, closes"| P["Mini-plan<br/>request as understood, proposal,<br/>items: todo / doing / done / blocked"]
    P -->|"blockers need a validation"| B
    A -->|"one step = one item"| T{"step_done<br/>succeeded?"}
    T -->|"yes: item done"| R["Relaunch<br/>inbox and current plan injected"]
    T -->|"no: reason kept, plan edited"| R
    R --> A
    H["Host budget<br/>12 calls without step_done: refused"] -.-> A
    C["Stop (C4)"] -.->|"immediately"| A
    A -.->|"each step is a run"| J["Run journal<br/>calls, results as received, durations<br/>bounded by weight and age, pinned runs kept"]
    J --> G["Judge<br/>a Gemini agent: reads the run, replays what reads,<br/>opens the result, compares to earlier verdicts"]
    G -->|"session verdict<br/>score, facts, frictions, trend"| F[("Task fiche<br/>current plan, closed plans, verdicts")]
    G -->|"tool verdict<br/>typed defects with proof, severity, proposed shape"| O[("Tool fiche<br/>one per tool, across all tasks")]
    P -->|"closed plans"| F
```

All of it is on master: the step loop, the mini-plan and the mailbox
(`0ab04909a`), the run journal and the session judge, the tool verdicts (missing
or misnamed parameter, a parameter that should be optional, a return that is
malformed, inconsistent, verbose, off-topic or incomplete, each with its call id
and a proposed shape, kept in a fiche per tool), and the journal bounded by
weight and age with pinned runs. Since then:

- **Each step starts a fresh thread**, carrying the request, the last 20 step
  results, the mailbox and the plan — a long task no longer drags its whole
  history into every call.
- **A question waits for its answer.** An unanswered question or approval
  suspends the teammate without calling the model; the mailbox says since when,
  and the step counts neither as done nor as failed.
- **Commands go through the gate.** In a task whose workspace declares a command
  gate, a command the gate holds becomes an approval in the mailbox; the
  workspace's declared scripts are pre-approved. The Landlock sandbox can run
  `cargo`, with a target shared per repository and the network closed.
- **A provider out of quota is waited for**, not counted as a failure: the same
  step is replayed after a growing delay, and the teammate stops after ten such
  steps, or at once on an exhausted daily quota. It resumes after a host
  restart, and a working teammate keeps the host from going idle.
- **Without its backend, it still reads and edits** the task's files, and says
  so; it stops after three failed steps in a row.

The supervisor follows it at `/ide/teammate` (`/ide/teammate/mailbox`,
`/ide/teammate/pending`, `/ide/teammate/seen`, `/ide/teammate/list`), in the IDE
or in a terminal (below).

The binaries `rag3weaver-backend` and `rag3weaver-chat` are this runtime served
over HTTP, with sections rendered after each tool so that a person reading over
the agent's shoulder sees what it did.

## Claude Code and Codex

The memory and the mailbox reach agents that do not run in rag3weaver.

- **Hooks for Claude Code** (`rag3weaver-backend hook`, installed per session by
  `scripts/hooks_claude_code.sh`, never globally unless asked): at the start of
  a session, who you are and what you were holding; on each prompt, at most one
  related memory card, what others changed in the files you touched, the
  judge's last verdict on your task and the mailbox; after a compaction, the
  summary kept on the task card and an identity check; at about 70 % of the real
  context, a suggestion to tidy up. A recall that takes too long finishes in the
  background and shows at the next tool call, after a judge that can only
  remove candidates. A hook always exits 0 and is silent without a host. Its
  texts are templates (`templates/hooks/`).
- **A shared mailbox** (`rag3weaver-backend mail send|reply|pending|seen`):
  Claude Code sessions, Codex threads (`codex:<name>`) and teammates write to
  each other; a reply goes to the author of what it answers.
- **A channel** (`rag3weaver-backend canal`, an MCP server for Claude Code) that
  pushes mailbox items into a running session, with `reply`, `send` and `claim`.

## The IDE and the terminal

The host serves an IDE in React (`ui/ide/`, built into `templates/ide/app/` and
embedded, never loaded from elsewhere): the chat, files with in-page editing,
search, tasks and their review with the teammate, session verdicts, the tree,
symbols, the graph, the live DAG of a running tool (with a Stop button),
reactions and proposals. "My searches" lists the searches made in a repository,
by you and by agents under their name, with their first results and the ones you
clicked (`GET /ide/recherches`). Code in the chat is coloured by the same
tree-sitter grammars that index it (`POST /highlight`).

`rag3weaver-tty` (crate `tty/`) follows a teammate from a terminal: the agent's
text scrolls into the terminal's own history and three lines are redrawn at the
bottom; Enter talks to the teammate, Esc stops it. It works over ssh (at most
30 redraws per second), resumes after a cut without loss or duplicate
(`Last-Event-ID`), degrades to 256 colours, 16, none and ASCII, and colours code
with the host's grammars and the theme `templates/themes/daemon.json`.
`--plain` writes the stream as text.

## Embeddings and rerankers

All production models run on burn (wgpu/Vulkan — AMD, NVIDIA, Apple with one
implementation); candle stays as the parity oracle (`candle-embedder`), checked
bit for bit, not by a rounded cosine.

| Model | Dims | Use |
|---|---|---|
| granite-embedding-278m-multilingual | 768 | default |
| granite-embedding-107m-multilingual | 384 | first index of a large repository, or a weak GPU |
| all-MiniLM-L6-v2, paraphrase-multilingual-MiniLM-L12-v2 | 384 | light |
| BGE-M3 | 1024 | dense + learned sparse in one forward pass (`DualEmbedder`) |

Rerankers (`RerankNode`, `set_reranker`): ms-marco MiniLM, mmarco multilingual
MiniLM, bge-reranker-v2-m3. OCR (`ocr`, `burn-ocr`): PP-OCR on burn. Weights are
not bundled; they are published as `burnpack` files with upstream attribution
(see `generated/README.md`).

Any provider plugs in through traits: `Embedder`, `SparseEmbedder`,
`DualEmbedder`, `Reranker` (and their `Callback*` helpers). The core has no ML
dependency.

## Features

No feature is on by default.

| Feature | What it adds |
|---|---|
| `rag3db-native` | the rag3db connection (links the shared library built from the fork) |
| `postgres` | the PostgreSQL dialect and blob store |
| `burn-embedder` | the burn models on Vulkan (`burn-rocm` for ROCm) |
| `candle-embedder`, `bge-m3`, `cuda` | the candle reference models |
| `code` | the code graph (`codeparsers`, sandboxed file access) |
| `daemon` | the embedding daemon and service client |
| `openai-llm` | an OpenAI-compatible LLM client |
| `anthropic-llm` | the Anthropic Messages API client (implies `openai-llm`) |
| `navigateur` | the IDE's tests in a headless browser |
| `ocr`, `burn-ocr` | OCR |

Runtime knobs are `RAG3WEAVER_*` environment variables (about sixty; the ones
that matter for ingestion are `RAG3WEAVER_EMBED_MODEL`, `RAG3WEAVER_EMBED_SERVICE`,
`RAG3WEAVER_GPU_DUTY`, `RAG3WEAVER_EMBED_CHAR_BUDGET`, `RAG3WEAVER_FTS`,
`RAG3WEAVER_TX_PAR_PAQUET`, `RAG3WEAVER_BATCH_FILES`,
`RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION`). They are documented where they are read.

## Tests

```bash
cargo test --lib --features "rag3db-native"        # unit tests
./run_e2e.sh                                        # end-to-end suites (131 e2e_*.rs files under tests/)
./run_e2e.sh --test e2e_code                        # one suite
```

End-to-end tests link the rag3db shared library (see `BUILD.md` at the root).
Several suites embed on the GPU; the ones that measure durability kill the
process with the database open and replay the journal.

## Where things are

```
src/
├── catalog.rs, catalog/        register, ingest, search; the batch synchronizer
├── search*.rs, fts_*.rs        signals, fusion, lucivy handle, FTS storage modes
├── dataflow/                   graph, runtime, ports, 47 nodes, Mermaid, reports, checkpoints
├── code*.rs, backend*.rs       code graph, agent backend and its tools
├── burn_*.rs, candle_*.rs      models
├── dialect.rs                  SchemaDialect: rag3db, PostgreSQL
├── scope.rs                    org × project
├── daemon_host/                the host: run journal, judge, chat, tasks, teammate, Codex delivery
└── bin/                        rag3weaver-backend, rag3weaver-chat, rag3daemon, rag3weaver-embeddings, rag3db-check-lib
templates/                      Mermaid pipelines, backends, hooks, the IDE bundle, themes
ui/ide/                         the IDE (React)
tty/                            rag3weaver-tty
scripts/                        hooks_claude_code.sh and other tools
codeparsers/                    submodule
docs/                           design notes and session reports (French), dated folders
visions/                        where this is going (French)
```

## License

[Luciform Research Source License (LRSL) v1.2](LICENSE)
