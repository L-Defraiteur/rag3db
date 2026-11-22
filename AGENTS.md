# AGENTS.md

Instructions for any coding agent working in this repository (Claude Code, Codex, others).

## Leitmotiv

What we are building, in Lucie's words (22 November 2026):

> **Become the hive mind. Control entire swarms of agents.**

A **swarm** is a group of agents working on one goal, each with a role, led by an orchestrator. The **hive** is all
of a user's swarms, on every machine and project. The user is the hive mind: not one team to follow, but several
swarms to command, and still everything understood. In French:

- « Lâchez une équipe d'agents sur vos projets. Vous garderez le fil. »
- « Ils travaillent seuls. Vous savez toujours où ils en sont. »
- « Une horde d'agents au travail, et vous comprenez tout. »
- « Vos agents travaillent toute la nuit. Le matin, vous savez tout. »
- « L'équipe d'agents qui se souvient. »

Ultra-autonomous agents that spawn other sessions and talk to each other; safety checks exist to validate what
runs without a human, not to keep one at the keyboard. An interface is simple to use, offers advanced things, and
a newcomer talks to an agent and work starts. Judge every change against this.

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
  cpu …` for cores, `poste gpu <GiB> …` for graphics memory; `poste lourd` is an alias of `cpu`), and never with `-j$(nproc)`: leave two cores free.

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
