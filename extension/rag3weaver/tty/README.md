# rag3weaver-tty

A terminal client for the rag3weaver host: it follows a teammate (the code agent) from a terminal,
streams what it does, and lets you talk to it, approve its commands and stop it. It needs nothing
but the rag3weaver host: no tmux, no mosh, no terminal emulator of its own. It runs where the host
runs, or on another machine through an ssh tunnel, on Linux, macOS and Windows (crossterm).

MIT, like rag3weaver. The crate does not link the engine: it is a plain HTTP client of the host.

## Run it

```
rag3weaver-tty --depot <name|path> --tache <name> [--demande "<text>"]
               [--theme daemon|foret|cyberpunk|cyborg|alien] [--journal <file>] [--plain]
               [--daemon-json <path> | --daemon-cmd "<command>"] [--adresse host:port] [--depuis <id>]
```

- `--depot` names an attached repository, by name or path; `--tache` names the task (its branch is
  `agent/<name>`). `--demande` opens the task with a first request; without it, the terminal follows
  the task as it is.
- The host is found through its `daemon.json` (address and token), by default the local one.
  On another machine: `ssh -N -L 17970:127.0.0.1:7970 <host machine>`, then
  `--daemon-cmd "ssh <host machine> cat .local/share/rag3weaver/daemon.json" --adresse 127.0.0.1:17970`.
- `--plain` writes the stream as plain text. It also takes over by itself when the inline view cannot
  start, for instance when the terminal does not report its cursor position, and it says so.
- The old names `--backend` and `--task` are still read, with a warning.

```
cargo build --release --bin rag3weaver-tty
```

Without the `vue` feature (default on), the crate is only the stream client and `--plain`, with no
ratatui in the build.

## What you see

The agent's text scrolls into the terminal's own history: the scrollback, copy and search are the
terminal's. Three lines at the bottom are redrawn, at most 30 times per second:

- the line being streamed, or a fallback note when nothing streams (see *mosh* below);
- the status line: what waits for you comes first, and is never covered by an animation (an
  approval with its command, a question, then a declarations proposal with its paths). Otherwise it
  is never a bare state word: the theme's animation while the teammate works, with what it waits
  for named beside it (`le modèle répond… (gemini-3.8-flash, 12 s)`, `run_command cargo check
  travaille… (40 s)`); the theme's `repos` animation when it is your turn; a sentence when it
  stopped on an error or ended without answering;
- the input line.

While the model thinks, or a tool reads or writes, the status line plays an animation chosen by what
the tool is allowed to do. On the right, in muted colour, it shows the repository, the branch and the
task, when they fit.

The history keeps what was asked and what was answered (and a `!` command with its output). The
last working tool call scrolls on the muted top line of the live zone instead, without a verdict
(a refusal is said); the step's plumbing (`step_done`, `report`, `edit_plan`), steps done and the
host's own words are not shown; Ctrl-O and the journal keep them all. `--etapes`
(`RAG3WEAVER_TTY_ETAPES=1`, or `/etapes` during the session) writes them all to the history.

An approval request is live state: it is said on the status line, not written to the history,
and still said once the teammate's step has ended. It shows two buttons, `[ Oui ]` chosen in
advance and `Non`: ← → change the choice (the chosen one in reverse video), Enter validates it,
and `o` or `n` answer at once, with an empty input; the answer, from the terminal or from the IDE, leaves
one line, `› accordé : <command>` or `› refusé : <command>`. A declarations proposal of the main
repository (the `declarations` stream) waits the same way after the mailbox: `o` applies it, `n`
rejects it (`POST /ide/propositions/appliquer`, `/rejeter`).

Mailbox lines start with a header in the theme's accent, in French: `[équipier · réponse]`,
`[toi · consigne]`, `[équipier · compte rendu]`, `[hôte · fini]`…
A line received twice (once by re-reading the mailbox, once from the stream) is written once.
The text after the header, like the model's own text, is prose: in the terminal's colour, folded
between words, with common Markdown rendered without its marks (`#` titles in bold accent, `-`/`*`/`+`
bullets as `•`, `**bold**`, `*italic*`, `` `inline code` `` tinted). An asterisk only counts when it
wraps a word: `src/*.rs` and `2 * 3` stay as they are. Code blocks are coloured by the host's own
grammars (`POST /highlight`), in the theme's code palette, and fold as they are.

## Keys

The input is a line editor, like a shell's.

| Key | Effect |
|---|---|
| ← → | one grapheme (a combining accent, a composed emoji, an ideogram is one step); typing inserts at the cursor |
| Home End, Ctrl-A Ctrl-E | both ends of the cursor's line |
| Ctrl-← Ctrl-→, Alt-← Alt-→, Alt-B Alt-F | one word |
| Backspace, Delete | the grapheme before the cursor, the one under it |
| Ctrl-W, Alt-Backspace | the word before, up to a space, not past the start of the line |
| Ctrl-U, Ctrl-K | up to the start, up to the end, of the line |
| ↑ ↓ | the line above or below in a multi-line input, otherwise what was sent (no duplicates); the draft comes back |
| Shift-Enter, Alt-Enter | a new line without sending: Shift-Enter where the emulator announces kitty's keyboard protocol (requested from that emulator only, given back on exit), Alt-Enter everywhere |
| a paste | arrives whole (bracketed paste): its newlines stay in the input, nothing is sent before Enter |
| Enter | answers what the teammate waits for, otherwise opens the task, then talks to it |
| `o` / `n`, empty input | grants or refuses the command the teammate waits for |
| Esc, Ctrl-G | stops the teammate (Ctrl-G without the escape delay of a multiplexer) |
| Ctrl-O | opens a tool call full size, in an alternate screen (see *Tool calls*) |
| Tab | completes a `/` command or a `!` command (see *Commands*) |
| Ctrl-C | clears a non-empty input, otherwise quits |
| Ctrl-D, empty input | quits |
| AltGr | its character (on Windows AltGr comes as Ctrl+Alt, and is kept) |

The input spans several lines: each folds to the width, and the live zone grows with it up to eight
input lines, then a window follows the cursor, a cut edge showing `… `. Wide characters count for two
columns. The terminal's cursor sits where the input's cursor is. An empty input says, muted, how to
break a line: Shift-Enter, Alt-Enter, or under tmux that Shift-Enter sends.

## Commands

An input starting with `/` is a command of the terminal, never sent to the teammate (`//` sends a
text starting with one `/`): `/aide`, `/quitter`, `/arreter`, `/theme <name>`, `/tache <name>` and
`/depot <name>` (the view starts again on another task or repository, keeping the session journal
and theme), `/journal`, `/effacer` (clears the visible screen) and `/effacer historique`, and
`/context` (how full the teammate's model context is, from the host's `GET /ide/chat`), and
`/model` (the language models the task's manifest declares, the default and the task's own marked;
`/model <name>` makes the task use one, kept with the task by the host).

`!<command>` runs it with `/bin/sh -c` where the terminal runs, in the task's worktree when it is on
this machine. The screen goes to the command, on a pseudo-terminal of its own (`posix_openpt`, through
the libc the standard library already links), so `sudo` can ask for its password and an interactive
tool can be answered. Only what the command writes is kept, never what is typed: a password read with
echo off reaches neither the kept output, nor the teammate, nor the session journal. Then the output
goes to the teammate as an instruction (« L'utilisateur a lancé la commande … ; voici le retour »),
without terminal sequences, cut in the middle beyond 8 KB. A `!` command only comes from what is typed
and sent with Enter: a mailbox line or a paste never runs one. Unix only for now.

Tab completes command names and their arguments (themes, tasks and repositories from the host), and,
after `!`, executables from the `PATH` and then files; the candidates show, muted, on the top line.
The history of what was sent survives the session in `<state folder>/historique.txt`, readable by its
owner only, the last thousand entries.

## Tool calls

Each tool call is numbered and says what it targets: `» read_file src/vue.rs  ⟨3⟩`. A read shows a
preview, the first 12 lines in the file's language, coloured by the host's grammars, then what is
left. A `.md` file renders as prose, its fenced blocks coloured in their language. An `edit_file`
shows its replacement as removed and added lines, or the beginning of the content it writes. A
fenced block in the mailbox is a code block of its language too. The host sends the first 4 KB of a
result and its full size: a cut excerpt says so (`extrait, 4 Ko sur 16 Ko`) and never looks whole.

Ctrl-O opens the last call full size, with its arguments, in an alternate screen, like `less`. ↑ ↓
scroll, Page Up and Page Down (or Space) move by a page, Home and End go to the ends, ← → go to the
previous or next call, and Esc, `q`, Ctrl-O or Ctrl-C close it. The main screen, its scrollback and
the terminal's history are left untouched and come back as they were.

## Over ssh, tmux and mosh

Nothing has to be installed or configured for these. Each degradation found is either fixed or turned
into a fallback the terminal states itself. Measured with the harness of `mesures/` (a scripted fake
host and a pty), both ways between two machines:

| | Host to screen, median | Keystroke echo, median |
|---|---|---|
| ssh | ~47 ms | 10–20 ms |
| tmux, local | ~14 ms | 0.3 ms |
| tmux, seen over ssh | ~48 ms | ~8–15 ms |
| mosh | ~47 ms | ~17 ms |

- **tmux** waits 500 ms by default before passing a lone Esc: use Ctrl-G to stop the teammate at
  once. With its default settings it also turns Shift-Enter into a plain Enter, which sends: under
  tmux (`TMUX`), the empty input says so, and Alt-Enter breaks the line. A tmux set with
  `extended-keys on` and `extended-keys-format csi-u` lets Shift-Enter through: the terminal reads these
  settings, asks for modifyOtherKeys mode 1 and gives it back on exit; the empty input names them. tmux may bring true colour down to 256 colours; the nearest colours of every theme stay right.
- **mosh** keeps no scrollback beyond the screen. Under mosh, which the terminal detects through a
  `mosh-server` ancestor (Linux, `/proc`), a session journal opens by itself and the top line of the
  live zone gives the command that reads it back: `less <journal>`. When the sentence does not fit,
  the command stays whole. Inside a container whose pid namespace hides the `mosh-server`, under a
  tmux started inside mosh, or on macOS, nothing is detected and nothing is said; `--journal` stays
  available.

## The session journal

`--journal <file>` writes, as plain text without colours, every line sent to the terminal's history:
the folded prose, code blocks, tools and the mailbox. In `--plain`, it receives the same text as the
output. Journals opened by themselves go to `$XDG_STATE_HOME/rag3weaver/tty/` (by default
`~/.local/state/rag3weaver/tty/`, `%LOCALAPPDATA%` on Windows) as `seance-<seconds>-<pid>.txt`.
Opening one first removes the `seance-*.txt` files there that are more than seven days old, and
nothing else.

## Colours, glyphs and themes

Colours degrade from true colour to 256, 16 and none (`NO_COLOR`), from `COLORTERM` and `TERM`.
Glyphs are a safe set by default; `RAG3WEAVER_TTY_GLYPHES=tous` adds the rarer ones and `=ascii`
keeps to ASCII.

Five themes ship with it: `daemon` (the default), `foret`, `cyberpunk`, `cyborg` and `alien`, chosen
by `--theme` or `RAG3WEAVER_TTY_THEME`. They are rag3weaver's theme files (`templates/themes/` of
rag3weaver), compiled into the binary and read by rag3weaver's IDE as well. Each theme is a named
palette in OKLCH (eight colours for each of the five: background, neutral, accent, two halo colours
and hues) with a mode, dark or light. Its colours and its code palette are derived from it by one
writer, rag3weaver's `scripts/themes_palettes.py`, from roles shared by every theme
(`templates/theme-roles.json`) and the theme's own overrides. A code kind starts from the palette
colour nearest its target hue and shifts toward it; a colour out of sRGB loses chroma at constant
hue and lightness; every pair is held to WCAG 2 contrast against the background (4.5 for text and
readable code, 3 for what goes with it), or the script refuses. `--check` turns red when a theme no
longer matches its palette. The optional field `couleurs.prose` gives the prose a colour of its own;
absent, the prose is in the terminal's colour. Fields are added to the theme schema, never renamed or
removed, so the IDE keeps reading them.

## What it reads from the host

`GET /ide/flux?sujets=agent,tokens,mailbox` (SSE, resumed after a cut with `Last-Event-ID`, without
loss or duplicate; a `lacune` event says when the host's buffer lost events), `GET /ide/etat`,
`GET /ide/describe` (each tool's policy), `GET /ide/teammate` and `GET /ide/teammate/list`,
`POST /ide/teammate/mailbox` (requests, instructions, answers with `reply_to`, stop) and
`POST /highlight`. Every call carries the host's bearer token.

## Tests

`cargo test` (with the view) and `cargo test --no-default-features` run the crate's own tests, with
no terminal: the line editor, the key mapping, the rendering of prose, Markdown and code, the
journal, the mosh detection, the stream decoder and its resumption. `mesures/mesure.py` drives the
real binary through ssh, tmux or mosh against `mesures/faux_hote.py`, and checks latency, echo,
resizing, keys (stop, approval, the line editor as typed by a real terminal, Ctrl-C), the journal and
the colours of every theme. rag3weaver's own end-to-end suite runs the stream client against a real
host, with a fake teammate.
