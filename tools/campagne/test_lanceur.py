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


if __name__ == "__main__":
    unittest.main()
