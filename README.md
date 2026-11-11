# rag3db

**Work in progress.** An embedded graph database for agents that read, write and
search in one place — and the engine under [rag3weaver](extension/rag3weaver/README.md),
the agent runtime built on it.

rag3db is a fork of [Kuzu](https://github.com/kuzudb/kuzu) v0.11.2.2: a graph
database that runs inside your process, speaks Cypher, and carries its own
indexes. On top of the fork we keep two native extensions (HNSW vectors, spatial
R-tree), and we spend most of our time on the write, rollback and recovery paths,
because an agent that writes into a live database must never lose or corrupt it.

Full-text search (BM25) and learned-sparse search are **not** C++ extensions:
they live in rag3weaver, through [lucivy](https://github.com/L-Defraiteur/lucivy/).
Upstream's `fts` extension was removed from the tree on 10 October 2026 (it had
not been built since August); git history keeps it. Full text in Cypher for a
standalone rag3db is a question for the day rag3db ships on its own.

## The idea

*Everything lives in one system, and everything in it is declared.*

The database (graph, words, vectors), the processing (graphs of typed nodes), the
tools, the memories, the views: one more declared thing, and the system knows how
to run it — no side service, no side code. What makes that possible is the
plumbing this repository is about:

| Because the engine… | …the product can |
|---|---|
| keeps everything in one system, with an address for each thing | make everything clickable: a row, a query, a graph, an execution step |
| records each execution node by node | show what the agent does, instead of having it tell |
| holds the graph, the words and the meaning together | give living context cards: what changed, what depends, what looks alike |
| validates and recovers properly | let an agent write into a database in service without losing it |
| will let several writers in at once | let a team work on the same site, each with their own agent |

The memory archivist files document sections under existing cards while preserving
their exact names, including long judge reports.

The long version, in French: [`extension/rag3weaver/visions/00-vision-generale.md`](extension/rag3weaver/visions/00-vision-generale.md).

## Where the engine stands

The engine has a written stopping point, **the stele**
([`docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md`](docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md)):
four conditions, and when they hold we tag a version and stop looking at the
engine except on a defect observed in the product.

| Condition | State (8 November 2026) |
|---|---|
| locks | done: row and index locks, named in their errors; a write waits for a lock up to `CALL lock_timeout` (30 s by default) |
| journaled bulk load — `COPY` no longer forces its own checkpoint | done, the default since 10 October 2026 (`CALL force_checkpoint_on_copy=true` brings the forced checkpoint back); a COPY whose journal would exceed `copy_journal_threshold` (an eighth of the buffer pool, 256 MiB at most) still falls back to it |
| parallel writers | on by default since 11 October 2026: several write transactions at once in one process (`CALL enable_multi_writes=false` for a single writer), and a second process that opens for writing waits up to 5 s for the file lock instead of failing; a checkpoint no longer stops readers, but a writer's `BEGIN` still waits for it (in progress) |
| no known defect that corrupts or loses data | not met: 37 open engine tickets, among them durability after an I/O fault or a power cut (directory `fsync` still missing), a crash in the vector index under a concurrent read, and an overflow in the concurrent `CREATE_VECTOR_INDEX` |

Every known defect is a file in [`docs/tickets/`](docs/tickets/) (98 open, 220
closed, at the time of writing), with its state, a minimal recipe and its
witness test; [`docs/tickets/README.md`](docs/tickets/README.md) is the
generated index. What landed, one line per step, is in
[`docs/nouveautes.md`](docs/nouveautes.md). Design notes, session reports and
tickets are written in French.

## What the fork changes

- **Vector search in Cypher `WHERE`.** `VECTOR_SEARCH(column, query, k)` is an
  index-scan predicate: the optimizer runs the HNSW search and exposes
  `VECTOR_DISTANCE()` as a column. The table functions
  (`CREATE_VECTOR_INDEX`, `QUERY_VECTOR_INDEX`, `DROP_VECTOR_INDEX`) remain.
- **Deletes and updates reach the HNSW index**, with batched edge cleanup, and
  rollback tells the index what it must forget.
- **A spatial extension**: N-dimensional R-tree with KNN, radius, bounding box,
  oriented box and frustum queries, plus 19 scalar geometry functions
  (`CREATE_SPATIAL_INDEX`, `QUERY_SPATIAL_INDEX`, `geo_*`).
- **Several writers at once**, by default: row and index locks, a second process
  that waits for the writer in place instead of failing, and a relationship
  `COPY` that never links another transaction's nodes.
- **Readers that never wait for a checkpoint**: a read sees the last committed
  state while a checkpoint writes; a reader opened before it keeps reading its
  own generation until the next one. Column ids never move (a dropped column
  leaves a hole), and rows of a rolled-back write are marked as such, so no
  reader ever sees them.
- **A storage format with a number** (43 today), set by hand: the header and the
  journal name the build that wrote them; a database or a journal from a newer
  engine is refused with both names; formats 39 to 42 open and are rewritten to
  43 at the first checkpoint, after which older engines refuse the file.
- **Hardened write paths**: all-or-nothing forced transactions, the journal
  written and synced before a commit is published (a commit whose journal cannot
  be synced publishes nothing), a journaled `COPY` replayed after a crash even
  when the same transaction then updates what it copied, a large blob `COPY`
  bounded in bytes so it fits a small buffer pool, a second open for writing
  refused by name inside the same process, `fsync` after replay, a compact
  journal form for numeric arrays, and a journal memory bound with a fallback to
  a forced checkpoint above it.
- **Reads that hold together**: in a `READ ONLY` transaction a failing statement
  no longer dooms the ones after it, graph projections run there too, an
  interruption stops the whole query, and a prepared statement frees what it
  bound when it dies (four readers of a real repository: 30 GB → 7.1 GB).
- **`CREATE_VECTOR_INDEX` without freezing writes**: it waits only for the
  writers already open, then builds while the others write; `SHOW_INDEXES` says
  whether an index is ready, under construction or incomplete. Every open loads
  the extensions recorded beside the database, even after an empty journal, and
  an extension that fails to load no longer goes unnoticed: the database opens
  and `SHOW_INDEXES` names the path and the cause in `extension_error`. HNSW pruning uses
  the classic heuristic since 10 October 2026 (an existing index keeps its graph
  until it is recreated).
- **A concurrency bench** (`test/`, `known_red` list: 16 lines, from 59 on
  10 October) that plays the forms an agent produces: kill during a bulk load,
  reopen, replay, compare row by row; and a power-cut double of the file system
  that loses every unsynced write with the process.

Nothing is sent upstream: the fork stays proprietary; upstream is read, not
copied.

## Layout

```
rag3db (fork of Kuzu v0.11.2.2)
|-- src/, test/                     the engine and its tests (incl. the concurrency bench)
|-- extension/vector/               HNSW extension (+ DELETE/UPDATE, VECTOR_SEARCH predicate)
|-- extension/geo/                  spatial extension (R-tree + 19 geo functions)
|-- extension/rag3weaver/           the Rust orchestrator (own README)
|   |-- codeparsers/                submodule: scopes, usages, relations for 13 languages
|   +-- visions/                    where the project is going (French)
|-- docs/                           stele, tickets, journal, design notes (French)
|-- tools/wasm/, tools/nodejs_api/  browser and Node.js builds, last built 24 August 2026
+-- BUILD.md
```

## Build

See [BUILD.md](BUILD.md).

```bash
./build.sh            # engine + vector + geo
./build.sh test       # + extension tests

# or by hand
mkdir -p build/release && cd build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . --parallel 8
```

The Rust side links the shared library this build produces; see the rag3weaver
README for its tests.

## Targets

| Target | State |
|---|---|
| Native, Linux x86_64 | the one we work on daily |
| Node.js (NAPI), browser WASM | built and tested on 24 August 2026, not maintained since; rag3weaver itself is native only |

## Provenance and license

- **Kuzu**: [kuzudb/kuzu](https://github.com/kuzudb/kuzu) v0.11.2.2, MIT (see `NOTICE`).
- **lucivy**: [L-Defraiteur/lucivy](https://github.com/L-Defraiteur/lucivy/), LRSL v1.2.

[Luciform Research Source License (LRSL) v1.2](LICENSE) — source-available, free
under 100K EUR/year of revenue.
