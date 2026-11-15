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
- the status line: what the teammate waits for comes first (an approval with its command, a
  question), otherwise its state (`attend`, `réfléchit…`, `agit : <tool>`, `fini : …`);
- the input line.

While the model thinks, or a tool reads or writes, the status line plays an animation chosen by what
the tool is allowed to do. On the right, in muted colour, it shows the repository, the branch and the
task, when they fit.

Mailbox lines start with a header in the theme's accent, in French: `[équipier · réponse]`,
`[toi · consigne]`, `[équipier · compte rendu]`, `[équipier · accord demandé]`, `[hôte · fini]`…
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
| Ctrl-C | clears a non-empty input, otherwise quits |
| Ctrl-D, empty input | quits |
| AltGr | its character (on Windows AltGr comes as Ctrl+Alt, and is kept) |

The input spans several lines: each folds to the width, and the live zone grows with it up to eight
input lines, then a window follows the cursor, a cut edge showing `… `. Wide characters count for two
columns. The terminal's cursor sits where the input's cursor is. An empty input says, muted, how to
break a line: Shift-Enter, Alt-Enter, or under tmux that Shift-Enter sends.

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
  tmux (`TMUX`), the empty input says so, and Alt-Enter breaks the line. tmux may bring true colour down to 256 colours; the nearest colours of every theme stay right.
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
rag3weaver), compiled into the binary and read by rag3weaver's IDE as well. Each gives its colours
(background, accent text, halo, muted), its three animations, a code palette and an ASCII set, and
each is held to the same minimal contrast against its background: 4.5 for text and readable code,
3 for what goes with it. The optional field `couleurs.prose` gives the prose a colour of its own;
absent, the prose is in the terminal's colour, which is the case for all five themes. Fields are
added to the theme schema, never renamed or removed, so the IDE keeps reading them.

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
