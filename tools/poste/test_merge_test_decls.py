#!/usr/bin/env python3
"""Witness of merge_test_decls.py, the merge driver of the test declaration files (24 Nov. 2026): two lots that each add
a `#[path = "…"] mod x;` at the same place of a tests/*.rs are merged by union, sorted; any other conflict stays one.
Usage: python3 tools/poste/test_merge_test_decls.py [path of merge_test_decls.py] — exit 0 when every case is green."""
import os, subprocess, sys, tempfile

DRIVER = os.path.realpath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), "merge_test_decls.py"))
red = 0


def say(ok, what):
    global red
    print(("vert   " if ok else "ROUGE  ") + what)
    red |= not ok


def git(d, *a, check=True):
    return subprocess.run(["git", "-C", d, *a], capture_output=True, text=True, check=check,
                          env={**os.environ, "LANG": "C", "LC_ALL": "C"})


def repo(base):
    d = tempfile.mkdtemp()
    git(d, "init", "-q", "-b", "master")
    git(d, "config", "user.email", "t@t"); git(d, "config", "user.name", "t")
    os.makedirs(f"{d}/extension/rag3weaver/tests")
    with open(f"{d}/.git/info/attributes", "w") as f:
        f.write("extension/rag3weaver/tests/*.rs merge=test-decls\n")
    git(d, "config", "merge.test-decls.driver", f"python3 {DRIVER} %O %A %B %P")
    write(d, base); git(d, "add", "-A"); git(d, "commit", "-qm", "base")
    return d


F = "extension/rag3weaver/tests/e2e_memoire.rs"


def write(d, text):
    with open(f"{d}/{F}", "w") as f:
        f.write(text)


def branch(d, name, text):
    git(d, "checkout", "-q", "-b", name, "master"); write(d, text)
    git(d, "commit", "-qam", name); git(d, "checkout", "-q", "master")


BASE = '''//! The memory suite.
#[path = "memoire/a_first.rs"]
mod a_first;

fn helper() {}
'''

# 1. Two lots, one declaration each, at the same place: the second lands by union, both modules, sorted.
d = repo(BASE)
branch(d, "x", BASE.replace("mod a_first;\n", 'mod a_first;\n/// The cards.\n#[path = "memoire/cards.rs"]\nmod cards;\n'))
branch(d, "y", BASE.replace("mod a_first;\n", 'mod a_first;\n#[path = "memoire/beta.rs"]\nmod beta;\n'))
git(d, "merge", "-q", "--no-edit", "x")
r = git(d, "merge", "-q", "--no-edit", "y", check=False)
text = open(f"{d}/{F}").read()
say(r.returncode == 0 and "<<<<<<<" not in text, f"two declarations at the same place: merged, no conflict (rc {r.returncode})")
say(0 <= text.find("mod beta;") < text.find("mod cards;") and "/// The cards." in text and text.count("mod a_first;") == 1,
    "both modules there, sorted by name, each with its doc line and path")
say(text.rstrip().endswith("fn helper() {}"), "the rest of the file untouched")

# 2. The same declaration on both sides: kept once.
d = repo(BASE)
same = BASE.replace("mod a_first;\n", 'mod a_first;\n#[path = "memoire/beta.rs"]\nmod beta;\n')
branch(d, "x", same + "fn x() {}\n"); branch(d, "y", same + "fn y() {}\n")
git(d, "merge", "-q", "--no-edit", "x"); git(d, "merge", "-q", "--no-edit", "y", check=False)
say(open(f"{d}/{F}").read().count("mod beta;") == 1, "the same declaration on both sides: kept once")

# 3. A conflict that is not only declarations stays a conflict.
d = repo(BASE)
branch(d, "x", BASE.replace("fn helper() {}", "fn helper() { one() }"))
branch(d, "y", BASE.replace("fn helper() {}", "fn helper() { two() }"))
git(d, "merge", "-q", "--no-edit", "x")
r = git(d, "merge", "-q", "--no-edit", "y", check=False)
say(r.returncode != 0 and "<<<<<<<" in open(f"{d}/{F}").read(), f"a conflict of code stays a conflict (rc {r.returncode})")

# 4. One side a declaration, the other code at the same place: a conflict.
d = repo(BASE)
branch(d, "x", BASE.replace("mod a_first;\n", 'mod a_first;\n#[path = "memoire/beta.rs"]\nmod beta;\n'))
branch(d, "y", BASE.replace("mod a_first;\n", "mod a_first;\nconst N: usize = 3;\n"))
git(d, "merge", "-q", "--no-edit", "x")
r = git(d, "merge", "-q", "--no-edit", "y", check=False)
say(r.returncode != 0, f"a declaration against code at the same place: a conflict (rc {r.returncode})")

sys.exit(1 if red else 0)
