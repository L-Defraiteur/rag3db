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

The long version, in French: [`extension/rag3weaver/visions/00-vision-generale.md`](extension/rag3weaver/visions/00-vision-generale.md).

## Where the engine stands

The engine has a written stopping point, **the stele**
([`docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md`](docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md)):
four conditions, and when they hold we tag a version and stop looking at the
engine except on a defect observed in the product.

| Condition | State (8 October 2026) |
|---|---|
| no known defect that corrupts or loses data | the defects found by the benches since 3 October are fixed on `master` (COPY rollback and its primary-key index, a key lost by the index when it grows, DROP COLUMN at checkpoint, read-only open with a non-empty journal, estimated statistics); what remains is in the vector index (reachability after bulk updates, the pruning rule) |
| journaled bulk load — `COPY` no longer forces its own checkpoint | written and tested, **behind a setting** (`CALL force_checkpoint_on_copy=false`); one defect blocks making it the default (pages left without an owner after a crash) |
| locks | the lock manager exists; the finer levels come after the bulk load |
| parallel writers | last; the mode is off outside the concurrency bench |

Every known defect is a file in [`docs/tickets/`](docs/tickets/) (78 at the time of
writing), with its state, a minimal recipe and its witness test.
[`docs/journal-des-chantiers.md`](docs/journal-des-chantiers.md) lists what is
open, unmerged or waiting for a decision. Design notes, session reports and
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
- **Hardened write paths**: all-or-nothing forced transactions, checkpoints that
  no longer hold a public lock while waiting, a second open for writing refused
  by name inside the same process, `fsync` after replay, a compact journal form
  for numeric arrays, and a journal memory bound with a fallback to a forced
  checkpoint above it.
- **A concurrency bench** (`test/`, `known_red` list) that plays the forms an
  agent produces: kill during a bulk load, reopen, replay, compare row by row.

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
|-- extension/lucivy/ld-lucivy/     submodule, reference only (rag3weaver takes lucivy from crates.io)
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
