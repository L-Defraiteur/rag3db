#!/usr/bin/env python3
"""French identifiers: which identifiers are written in French (21 Nov. 2026, Lucie: identifiers are in English).

Two layers, each usable alone:

- the function: `Lexicon.load()` then `french_reason(identifier, lexicon)` says why an identifier is French (« accent »,
  « mot français « ouvrir » »), or None. A word of the identifier (split at _, -, ., /, and camelCase) is French when it
  has at least four letters, is in the French word list and not in the English one (both compared without accents), or
  is in `always_french.txt`; `never_french.txt` silences a word. The lists live in ../data/wordlists (see its README);
- the reading of a lot: `lot_identifiers(repository, base, head)` lists the identifiers that the lines added by
  `git diff base...head` declare or name, in code files (Rust, TS/JS, Python, shell, JSON templates): functions,
  types, constants, test names, shell functions and subcommands, routes (« "/…" »), JSON keys, serde and clap names,
  the values of keys that name a node, a tool or a type, and the names of the files the lot adds. Text is never read:
  comments, displayed strings, prompts, error messages, the prose of templates; nor the strings of test files and Rust
  test modules, which are data (their function and test names are judged); nor generated files (generated_files). An identifier also present on a line the
  lot removes, or already written in the base's tree, is not new.

Usage: french_identifiers.py <repository> <base> <head>
  One line per French identifier, « path:line  identifier  (what, reason) ». Always exits 0: it is a warning, never a
  refusal (the pusher writes it to the lot's log and to its author's inbox, and the lot goes on).
"""
import json
import os
import re
import subprocess
import sys
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
# FRENCH_IDENTIFIERS_WORDLISTS points elsewhere (a copy of the script out of the crate).
WORDLISTS = os.environ.get("FRENCH_IDENTIFIERS_WORDLISTS") or os.path.normpath(os.path.join(HERE, "..", "data", "wordlists"))

# ── The function ─────────────────────────────────────────────────────────────────────────────────────────────────────

WORD = re.compile(r"[A-ZÀ-Þ]?[a-zß-ſ]+|[A-ZÀ-Þ]+(?![a-z])|[0-9]+")


def plain(word):
    """Lower case, without accents."""
    return "".join(c for c in unicodedata.normalize("NFD", word) if unicodedata.category(c) != "Mn").lower()


def _list(path):
    out = set()
    if os.path.isfile(path):
        with open(path, encoding="utf-8") as f:
            for raw in f:
                w = raw.split("#", 1)[0].strip()
                if w:
                    out.add(plain(w))
    return out


class Lexicon:
    def __init__(self, french, english, always=(), never=()):
        self.french, self.english, self.always, self.never = set(french), set(english), set(always), set(never)

    @classmethod
    def load(cls, root=WORDLISTS):
        with open(os.path.join(root, "fr", "words.json"), encoding="utf-8") as f:
            french = {plain(w) for w in json.load(f)}
        with open(os.path.join(root, "en", "words.json"), encoding="utf-8") as f:
            english = {w.lower() for w in json.load(f)}
        return cls(french, english, _list(os.path.join(root, "always_french.txt")), _list(os.path.join(root, "never_french.txt")))

    def is_french(self, word):
        w = plain(word)
        if w in self.never:
            return False
        # A word of always_french.txt counts in the plural too (« dossiers », « vues »).
        if w in self.always or (len(w) > 3 and w[-1] in "sx" and w[:-1] in self.always):
            return True
        return len(w) >= 4 and w in self.french and w not in self.english


def words_of(identifier):
    for part in re.split(r"[_\-./{}:*\s]+", identifier):
        yield from WORD.findall(part)


def french_reason(identifier, lexicon):
    """Why the identifier is French, or None."""
    if any(ord(c) > 127 for c in identifier):
        return "accent"
    for w in words_of(identifier):
        if lexicon.is_french(w):
            return f"mot français « {plain(w)} »"
    return None


# ── The reading of a lot ─────────────────────────────────────────────────────────────────────────────────────────────

CODE_EXT = {".rs": "rust", ".ts": "js", ".tsx": "js", ".js": "js", ".mjs": "js", ".cjs": "js", ".py": "python",
            ".sh": "shell", ".bash": "shell", ".json": "json"}
STRING = re.compile(r'"(?:[^"\\\n]|\\.)*"' + r"|'(?:[^'\\\n]|\\.)*'")
DECLS = {
    "rust": [r"\bfn\s+(\w+)", r"\b(?:struct|enum|trait|type|mod|union)\s+(\w+)", r"\b(?:const|static)\s+(?:mut\s+)?(\w+)\s*:",
             r"\bmacro_rules!\s*(\w+)"],
    "js": [r"\bfunction\s*\*?\s*(\w+)", r"\b(?:class|interface|type|enum|namespace)\s+(\w+)",
           r"\bexport\s+(?:const|let|var)\s+(\w+)"],
    "python": [r"^\s*(?:async\s+)?def\s+(\w+)", r"^\s*class\s+(\w+)"],
    "shell": [r"^\s*(?:function\s+)?([A-Za-z_][\w-]*)\s*\(\)\s*\{", r"^\s*([a-z][\w-]*(?:\|[a-z][\w-]*)*)\)\s"],
}
NAMED = re.compile(r'\b(?:rename|alias|name|long|rename_all)\s*=\s*"([^"\s]+)"')
IDENT = r"[A-Za-z_À-ſ][\wÀ-ſ.-]*"
KEY = re.compile(r'"(' + IDENT + r')"\s*:')
# The keys whose value names something (a node, a tool, a type…): the value is an identifier when it has no space.
NAMING_KEYS = {"name", "id", "tool", "node", "type", "kind", "command", "subcommand", "op", "action", "route"}
NAMING = re.compile(r'"(' + "|".join(sorted(NAMING_KEYS)) + r')"\s*:\s*"(' + IDENT + r')"')
ROUTE = re.compile(r"^/[\wÀ-ſ{}:.*-]+(?:/[\wÀ-ſ{}:.*-]*)*$")


def _git(repo, *args):
    return subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True, check=True).stdout


def _kind(repo, head, path):
    ext = os.path.splitext(path)[1].lower()
    if ext in CODE_EXT:
        return CODE_EXT[ext]
    if ext == "":  # a script without extension, by its shebang
        try:
            first = _git(repo, "show", f"{head}:{path}").split("\n", 1)[0]
        except subprocess.CalledProcessError:
            return None
        if first.startswith("#!"):
            return "python" if "python" in first else "shell" if re.search(r"\b(ba|z|da)?sh\b", first) else None
    return None


def _code(line, kind):
    """The line without its comment, its strings kept (routes and keys live in them)."""
    if kind == "json":
        return line
    code, i = "", 0
    pieces = []
    for m in STRING.finditer(line):
        pieces.append((line[i:m.start()], False)); pieces.append((m.group(0), True)); i = m.end()
    pieces.append((line[i:], False))
    for text, is_string in pieces:
        if is_string:
            code += text; continue
        cut = None
        if kind in ("rust", "js"):
            k = text.find("//"); cut = k if k >= 0 else None
            if kind == "js" and cut is None and text.lstrip().startswith("*"):
                return code
        else:  # python, shell: a # that opens a word
            m = re.search(r"(?:^|\s)#", text)
            if m and not (kind == "shell" and re.search(r"\$\{?#", text)):
                cut = m.start() + (1 if text[m.start()].isspace() else 0)
        if cut is not None:
            return code + text[:cut]
        code += text
    return code


def line_identifiers(line, kind):
    """(identifier, what) that one line of code declares or names."""
    code = _code(line, kind)
    if not code.strip():
        return []
    found = [(k, "clé JSON") for k in KEY.findall(code)]
    found += [(v, f"nom ({k})") for k, v in NAMING.findall(code)]
    if kind == "json":
        return found
    bare = STRING.sub('""', code)
    for pat in DECLS.get(kind, []):
        for m in re.finditer(pat, bare):
            found += [(n, "déclaration") for n in m.group(1).split("|")]
    found += [(n, "nom déclaré") for n in NAMED.findall(code)]
    for s in STRING.findall(code):
        body = s[1:-1]
        if len(body) > 1 and ROUTE.match(body):
            found.append((body, "route"))
    return found


# What a string names in a test file is data, not an identifier (21 Nov., c7): the strings of test_* files, of tests/, and
# of a Rust file's test module are not read; the names of their functions and tests still are.
STRING_WHATS = ("clé JSON", "clé", "route", "nom déclaré")


def is_test_file(path):
    base = os.path.basename(path)
    return base.startswith("test_") or path.startswith("tests/") or "/tests/" in path or base.endswith("_tests.rs") or base == "tests.rs"


def test_module_start(repo, head, path):
    """The first line of a Rust file's test module (#[cfg(test)] or mod tests), or None."""
    if not path.endswith(".rs"):
        return None
    try:
        text = _git(repo, "show", f"{head}:{path}")
    except subprocess.CalledProcessError:
        return None
    for i, line in enumerate(text.splitlines(), 1):
        if re.match(r"\s*(#\[cfg\(test\)\]|mod\s+tests\b)", line):
            return i
    return None


def generated_files(repo, head, paths):
    """The paths the repository marks generated at head: listed in tools/poste/generated_files.txt (« <path> TAB
    <command> »), or marked linguist-generated by .gitattributes (21 Nov., c7: a minified bundle's text looked like a
    key). Generated files are not read; their sources are."""
    out = set()
    try:
        listed = _git(repo, "show", f"{head}:tools/poste/generated_files.txt")
        out.update(l.split("\t", 1)[0].strip() for l in listed.splitlines() if l.strip() and not l.startswith("#"))
    except subprocess.CalledProcessError:
        pass
    if paths:
        r = subprocess.run(["git", "-C", repo, "check-attr", "--source", head, "linguist-generated", "--", *paths],
                           capture_output=True, text=True)
        for line in r.stdout.splitlines():
            p, _, v = line.rpartition(": linguist-generated: ")
            if v.strip() in ("set", "true"):
                out.add(p)
    return out


def lot_identifiers(repo, base, head):
    """(path, line, identifier, what) for each identifier the lot adds; line 0 for an added file."""
    diff = _git(repo, "diff", "-U0", "--no-color", "--no-ext-diff", f"{base}...{head}")
    generated = generated_files(repo, head, _git(repo, "diff", "--name-only", f"{base}...{head}").splitlines())
    added, removed = [], set()
    path = kind = test_from = None
    test_file = False
    lineno = 0
    for raw in diff.splitlines():
        if raw.startswith("+++ "):
            p = raw[4:]; path = p[2:] if p.startswith("b/") else None
            kind = _kind(repo, head, path) if path and path not in generated else None
            test_file = bool(path) and is_test_file(path)
            test_from = test_module_start(repo, head, path) if path and kind == "rust" and not test_file else None
            continue
        if raw.startswith("--- "):
            continue
        m = re.match(r"@@ -\S+ \+(\d+)", raw)
        if m:
            lineno = int(m.group(1)); continue
        if kind is None:
            continue
        if raw.startswith("+"):
            data = test_file or (test_from is not None and lineno >= test_from)
            added += [(path, lineno, i, w) for i, w in line_identifiers(raw[1:], kind)
                      if not (data and (w in STRING_WHATS or w.startswith("nom (")))]
            lineno += 1
        elif raw.startswith("-"):
            removed.update(i for i, _ in line_identifiers(raw[1:], kind))
    for p in _git(repo, "diff", "--name-only", "--diff-filter=A", f"{base}...{head}").splitlines():
        if p.endswith(".md") or "/fixtures/" in p or p.startswith("extension/rag3weaver/data/wordlists/") or p in generated:
            continue
        added.append((p, 0, os.path.splitext(os.path.basename(p))[0], "fichier ajouté"))
    return [a for a in added if a[2] not in removed]


def in_tree(repo, rev, ident):
    """Whether the identifier is already written in the tree of rev, as a whole word (git grep -w -F)."""
    return subprocess.run(["git", "-C", repo, "grep", "-q", "-w", "-F", "-e", ident, rev, "--"],
                          capture_output=True).returncode == 0


def lot_french_identifiers(repo, base, head, lexicon=None):
    """(path, line, identifier, what, reason) for each French identifier the lot introduces, each identifier once. An
    identifier already in the base's tree is not new (21 Nov., c7: a lot that calls an existing French route)."""
    lexicon = lexicon or Lexicon.load()
    seen, out = set(), []
    for path, line, ident, what in lot_identifiers(repo, base, head):
        if ident in seen:
            continue
        seen.add(ident)
        r = french_reason(ident, lexicon)
        if r and not in_tree(repo, base, ident):
            out.append((path, line, ident, what, r))
    return out


def main():
    if len(sys.argv) != 4:
        print("usage: french_identifiers.py <repository> <base> <head>", file=sys.stderr)
        return 2
    for path, line, ident, what, reason in lot_french_identifiers(*sys.argv[1:4]):
        print(f"{path}:{line}  {ident}  ({what}, {reason})" if line else f"{path}  {ident}  ({what}, {reason})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
