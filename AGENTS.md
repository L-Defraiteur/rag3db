# AGENTS.md

Instructions for any coding agent working in this repository (Claude Code, Codex, others).

## Read first

1. [`docs/README.md`](docs/README.md): how the docs are organised (one folder per working
   session), where to start, and how to take over a session's scope. Its **« Reprendre »**
   sections give, for each unfinished worktree, its path, its branch, its last commit and the
   commands to continue from there.
2. `docs/orchestration/<latest date>/02-knowledge-dump.md`: the rules of the game.
3. [`docs/procedures/push-d-un-lot.md`](docs/procedures/push-d-un-lot.md): how a change is
   tested and pushed.

The docs are in French; the rag3weaver README is in English.

## The rules that are never broken

- **No force push**, not even `--force-with-lease`. **No deletion of a remote branch.** A
  rebased branch is pushed under a new name.
- **Lucie's main tree** (`~/git_workspaces/rag3db`) is hers: nothing is committed in it, and no
  branch is switched there. Work in your own worktree, with its own branch and its own cargo
  target.
- **A test is seen failing before the fix** (red first), then passing. A test that skips is not
  green.
- **Every commit is pushed to its branch** within the minute; nothing stays local.
- **No mention of an AI assistant in commit messages.** The git identity is Lucie's personal
  address, never a work address.
- **A known defect that is not fixed gets a ticket**, under `docs/<session>/tickets/`, not a line
  in a message.
- **Heavy builds and test runs** go through the machine lock (`~/.cache/rag3weaver-build/poste
  lourd …`), and never with `-j$(nproc)`: leave two cores free.

## The MCP tools at hand

When the rag3weaver host runs on the machine, its MCP servers give you the repository and a
memory:

- **the code index** (`search_code`, `usages`, `outline`, `impact`, `callees`, `read_file`,
  `grep_files`, `list_files`): search by words and meaning, find definitions and uses, read the
  outline of a file. Prefer them to a full-text grep; `grep_files` is for an exact literal;
- **the long-term memory** (`recall_fiches`, `get_fiche`, `put_fiche`, `recall_apports`,
  `cited_by`…): what earlier sessions learned, as cards. Read before deciding; write what you
  learn that is not already in the code or the git history.

`daemon_status` says whether the host answers, and what to do if it does not.

## rag3weaver

The crate lives in [`extension/rag3weaver/`](extension/rag3weaver/): its
[README](extension/rag3weaver/README.md) says what it does and how to start. It will one day be a
repository of its own: write anything about it so that it stands alone, with no path back to this
repository's root.
