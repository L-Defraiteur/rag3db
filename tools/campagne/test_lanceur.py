#!/usr/bin/env python3
"""Witness of the campaign launcher, lot 1 (rag3weaver-archi, 19 Nov 2026): on a small synthetic
repository (a parent, a submodule, a docs/ folder, a fix), the case is read, a leaking case is
refused, and the tree is built without history, without docs/, with its submodule."""
import json
import os
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lanceur  # noqa: E402


def git(d, *args):
    return subprocess.run(["git", "-C", d, *args], check=True, capture_output=True, text=True).stdout.strip()


def ecrire(chemin, texte):
    os.makedirs(os.path.dirname(chemin), exist_ok=True)
    with open(chemin, "w") as f:
        f.write(texte)


class Lanceur(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        t = self.tmp.name
        ident = ["-c", "user.name=T", "-c", "user.email=t@example.org", "-c", "protocol.file.allow=always"]
        # The submodule.
        self.sub = os.path.join(t, "sub")
        os.makedirs(self.sub)
        git(self.sub, "init", "-q")
        ecrire(os.path.join(self.sub, "lib.rs"), "pub fn s() {}\n")
        git(self.sub, "add", "-A")
        git(self.sub, *ident, "commit", "-qm", "sub")
        self.sub_commit = git(self.sub, "rev-parse", "HEAD")
        # The repository at the parent: code, a witness, a docs/ journal that tells the fix.
        self.repo = os.path.join(t, "repo")
        os.makedirs(self.repo)
        git(self.repo, "init", "-q")
        ecrire(os.path.join(self.repo, "crate/src/lib.rs"), "pub fn f() -> u8 { 1 }\n#[test]\nfn the_witness() { assert_eq!(f(), 2); }\n")
        ecrire(os.path.join(self.repo, "crate/Cargo.toml"), "[package]\nname = \"crate_du_cas\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n")
        ecrire(os.path.join(self.repo, "docs/session/01-le-principe.md"), "Le correctif : f rend 2.\n")
        ecrire(os.path.join(self.repo, "docs/procedures/push.md"), "La procédure.\n")
        ecrire(os.path.join(self.repo, "crate/README.md"), "Le crate.\n")
        git(self.repo, "add", "-A")
        subprocess.run(["git", "-C", self.repo, *ident, "submodule", "add", "-q", self.sub, "vendor/sub"], check=True, capture_output=True)
        git(self.repo, *ident, "commit", "-qm", "parent")
        self.parent = git(self.repo, "rev-parse", "HEAD")
        # The fix, and a file the witness needs, born with it.
        ecrire(os.path.join(self.repo, "crate/src/lib.rs"), "pub fn f() -> u8 { 2 }\n#[test]\nfn the_witness() { assert_eq!(f(), 2); }\n")
        ecrire(os.path.join(self.repo, "crate/tests/born.rs"), "// born with the fix\n")
        git(self.repo, "add", "-A")
        git(self.repo, *ident, "commit", "-qm", "fix")
        self.ours = git(self.repo, "rev-parse", "HEAD")

    def tearDown(self):
        self.tmp.cleanup()

    def case(self, report, **block):
        b = {"parent": self.parent, "tree": "crate", "needs_lib": False, "witness": "the_witness",
             "test": "cargo test", "suites": [], "hints": [], "ours": self.ours}
        b.update(block)
        p = os.path.join(self.tmp.name, "cas.md")
        ecrire(p, "# Un cas\n\n```json\n" + json.dumps(b, indent=2) + "\n```\n\n## Le point de départ\n\nUne piste : f doit rendre 2.\n\n"
               + lanceur.REPORT_TITLE + "\n\n" + report + "\n\n## Notre correctif\n\n" + self.ours + "\n")
        return p

    def test_the_case_is_read_and_only_the_report_is_given(self):
        block, report = lanceur.read_case(self.case("> f rend 1 au lieu de 2."))
        self.assertEqual(block["parent"], self.parent)
        self.assertEqual(report.strip(), "f rend 1 au lieu de 2.")
        self.assertNotIn("piste", report)
        self.assertNotIn(self.ours, report)

    def test_a_report_that_names_our_fix_or_its_files_is_refused(self):
        p = self.case("> Voir " + self.ours[:9] + ".")
        block, report = lanceur.read_case(p)
        self.assertTrue(any("ours" in r for r in lanceur.refusals(block, report, self.repo, "cas.md")))
        p = self.case("> Le défaut est dans crate/src/lib.rs.")
        block, report = lanceur.read_case(p)
        self.assertTrue(any("crate/src/lib.rs" in r for r in lanceur.refusals(block, report, self.repo, "cas.md")))
        block, report = lanceur.read_case(self.case("> f rend 1."))
        self.assertEqual(lanceur.refusals(block, report, self.repo, "cas.md"), [])

    def test_the_tree_has_no_history_no_docs_and_its_submodule(self):
        block, _ = lanceur.read_case(self.case("> f rend 1.", witness_files_from={"commit": self.ours, "paths": ["crate/tests/born.rs"]}))
        dest = os.path.join(self.tmp.name, "arbre")
        lanceur.build_tree(block, self.repo, os.path.join(self.tmp.name, "depots"), dest)
        self.assertEqual(git(dest, "rev-list", "--count", "HEAD"), "1", "one commit, no history")
        with open(os.path.join(dest, "crate/src/lib.rs")) as f:
            self.assertIn("f() -> u8 { 1 }", f.read(), "the parent, not the fix")
        self.assertFalse(os.path.exists(os.path.join(dest, "docs/session")), "no session journal")
        self.assertTrue(os.path.exists(os.path.join(dest, "docs/procedures/push.md")))
        self.assertTrue(os.path.exists(os.path.join(dest, "crate/README.md")))
        self.assertTrue(os.path.exists(os.path.join(dest, "vendor/sub/lib.rs")), "the submodule at its commit")
        self.assertTrue(os.path.exists(os.path.join(dest, "crate/tests/born.rs")), "the witness files")
        self.assertNotIn(self.ours, git(dest, "log", "--format=%H %s"))


    def test_the_red_is_proved_at_the_parent_on_the_assertion(self):
        block, _ = lanceur.read_case(self.case("> f rend 1.", test="cargo test -q -- --exact the_witness"))
        dest = os.path.join(self.tmp.name, "arbre")
        lanceur.build_tree(block, self.repo, os.path.join(self.tmp.name, "depots"), dest)
        target = os.path.join(self.tmp.name, "target")
        rouge = lanceur.prove_red(block, dest, target, poste=None)
        self.assertTrue(rouge["red"], rouge)
        self.assertEqual(rouge["failed"], ["the_witness"], rouge)
        # A witness that does not compile is not a red.
        with open(os.path.join(dest, "crate/src/lib.rs"), "a") as f:
            f.write("fn casse( {\n")
        rouge = lanceur.prove_red(block, dest, target, poste=None)
        self.assertFalse(rouge["red"], rouge)
        self.assertIn("compile", rouge["why"])


    def test_a_case_command_runs_without_the_incremental_cache(self):
        # 19 Nov 2026, c7: a case is built once, on its own target; its incremental/ is never reused, and archi's
        # target held 20 of its 31 Go there. CARGO_INCREMENTAL=0, unless the environment already says otherwise.
        dest = os.path.join(self.tmp.name, "arbre-incr"); os.makedirs(dest)
        old = os.environ.pop("CARGO_INCREMENTAL", None)
        try:
            rc, out = lanceur.run_test({}, dest, os.path.join(self.tmp.name, "t"), None, 'echo "INCR=${CARGO_INCREMENTAL:-absent}"')
            self.assertEqual(rc, 0, out)
            self.assertIn("INCR=0", out)
            os.environ["CARGO_INCREMENTAL"] = "1"
            rc, out = lanceur.run_test({}, dest, os.path.join(self.tmp.name, "t"), None, 'echo "INCR=${CARGO_INCREMENTAL:-absent}"')
            self.assertIn("INCR=1", out)
        finally:
            os.environ.pop("CARGO_INCREMENTAL", None)
            if old is not None:
                os.environ["CARGO_INCREMENTAL"] = old

    def test_git_is_refused_and_the_rest_granted(self):
        for c in ["git log", "git -C . show HEAD", "env A=b git status", "cd x && git diff", "/usr/bin/git log"]:
            self.assertFalse(lanceur.answer_for(c)["approved"], c)
        for c in ["cargo test -j 8", "env RUSTUP_HOME=/x cargo --version", "grep -rn digit src"]:
            self.assertTrue(lanceur.answer_for(c)["approved"], c)

    def test_the_frozen_declarations_let_commands_run_without_a_human(self):
        ecrire(os.path.join(self.repo, "extension/rag3weaver/templates/backends/code/backend.json"),
               json.dumps({"database": "x", "tools": [], "workspace": {"commands": "approval", "root": ".", "sandbox": {"extra_read": ["~/.rustup", "/opt/x"]}}}))
        git(self.repo, "add", "-A")
        git(self.repo, "-c", "user.name=T", "-c", "user.email=t@example.org", "commit", "-qm", "templates")
        d = lanceur.freeze_declarations(self.repo, "HEAD", os.path.join(self.tmp.name, "decl"))
        with open(os.path.join(d, "templates/backends/code/backend.json")) as f:
            w = json.load(f)["workspace"]
        self.assertEqual(w["commands"], "auto")
        # The campaign host has a HOME of its own: « ~ » there is a link, and the sandbox reads
        # the real chain (cargo could not read ~/.rustup/settings.toml, case tty 09, 19 Nov).
        self.assertEqual(w["sandbox"]["extra_read"], [os.path.join(os.path.expanduser("~"), ".rustup"), "/opt/x"])

    def test_the_diff_holds_new_files_and_uncommitted_changes(self):
        block, _ = lanceur.read_case(self.case("> f rend 1."))
        dest = os.path.join(self.tmp.name, "arbre")
        lanceur.build_tree(block, self.repo, os.path.join(self.tmp.name, "depots"), dest)
        base = git(dest, "rev-parse", "HEAD")
        ecrire(os.path.join(dest, "crate/src/lib.rs"), "pub fn f() -> u8 { 2 }\n")
        git(dest, "-c", "user.name=T", "-c", "user.email=t@example.org", "commit", "-qam", "agent")
        ecrire(os.path.join(dest, "crate/src/neuf.rs"), "// new\n")
        ecrire(os.path.join(dest, "crate/README.md"), "Le crate, revu.\n")
        diff, stat = lanceur.collect_diff(dest, base)
        self.assertIn("{ 2 }", diff)
        self.assertIn("crate/src/neuf.rs", diff)
        self.assertIn("revu", diff)
        self.assertIn("3 files changed", stat)
        self.assertEqual([l.strip() for l in git(dest, "status", "--porcelain").splitlines()], ["M crate/README.md", "?? crate/src/neuf.rs"], "the worktree and its index are left as they were")


if __name__ == "__main__":
    unittest.main()
