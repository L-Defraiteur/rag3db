# The images of the README

Every image of the README is a real capture. None is drawn by hand.

- **IDE views** (`ide-*.png`) are captured from a test host. A scripted model sits behind the chat,
  as in the browser tests. The views are the real IDE bundle of this crate.
- **Terminals** (the other `*.png`) are the real output of a command, kept as text under
  [`sorties/`](sorties/). The first line of each file is the command; the rest is its output, as
  printed. The same browser renders it as an image.

Regenerate them all, from the root of the crate:

```sh
RAG3WEAVER_EXIGER_NAVIGATEUR=1 cargo test --lib --features rag3db-native,code,daemon,navigateur \
  -- --ignored daemon_host::captures_du_readme
```

`RAG3WEAVER_CAPTURES=<folder>` writes them elsewhere.

## Where each terminal output comes from (13 November 2026)

| file | command | where |
|---|---|---|
| `sorties/trace-d-un-index.txt` | the `index` tool by MCP, then the journal it names | an isolated host built from `master`, on the codeparsers repository (184 files) |
| `sorties/memoire-recall.txt` | `put_fiche` three times, then `recall_fiches` by MCP | the memory template of this crate, on the same isolated host, with an embedding service declared |
| `sorties/juge-local.txt` | `python3 -I r_verdict.py Q8_0` | the bench notes of the local judge. It rereads the results of the 11 November run; it is not the run itself |
| `sorties/postgres.txt` | `./run_e2e.sh --test e2e_postgres` | PostgreSQL 17 + pgvector in a container. The engine library predates `master` (`RAG3WEAVER_MOTEUR_ANCIEN=1`); this suite tests the PostgreSQL dialect, not the engine |
| `sorties/lucivy.txt` | `python3 lucivy_demo.py .` ([the script](sorties/lucivy_demo.py)) | lucivy 4.3.3 from PyPI, in a fresh virtual environment, on the sources of this crate |

An output is only replaced by a new run of its command, never edited.
