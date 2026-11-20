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
               json.dumps({"database": "x", "tools": [], "workspace": {"commands": "approval", "root": ".", "sandbox": {"extra_read": ["~/.rustup", "/opt/x"], "env": {"KEEP": "1"}}}}))
        git(self.repo, "add", "-A")
        git(self.repo, "-c", "user.name=T", "-c", "user.email=t@example.org", "commit", "-qm", "templates")
        d = lanceur.freeze_declarations(self.repo, "HEAD", os.path.join(self.tmp.name, "decl"))
        with open(os.path.join(d, "templates/backends/code/backend.json")) as f:
            w = json.load(f)["workspace"]
        self.assertEqual(w["commands"], "auto")
        # The campaign host has a HOME of its own: « ~ » there is a link, and the sandbox reads
        # the real chain (cargo could not read ~/.rustup/settings.toml, case tty 09, 19 Nov).
        self.assertEqual(w["sandbox"]["extra_read"], [os.path.join(os.path.expanduser("~"), ".rustup"), "/opt/x"])
        # A tree without its own toolchain file (lucivy) falls back on rustup's default, read in
        # RUSTUP_HOME: under the campaign HOME it was the link, and rustup said « no default is
        # configured » (case lucivy 03, 19 Nov). The sandbox gets the real homes.
        self.assertEqual(w["sandbox"]["env"]["RUSTUP_HOME"], os.path.join(os.path.expanduser("~"), ".rustup"))
        self.assertEqual(w["sandbox"]["env"]["CARGO_HOME"], os.path.join(os.path.expanduser("~"), ".cargo"))
        self.assertEqual(w["sandbox"]["env"]["KEEP"], "1", "the other variables stay")

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


    def test_the_trace_sums_the_steps_of_the_task_and_names_the_slow_calls(self):
        runs = os.path.join(self.tmp.name, "runs", "abc")
        os.makedirs(runs)
        def llm(ms, prompt, cached, out, retries=0):
            return {"kind": "LlmCall", "ms": str(ms), "prompt_tokens": str(prompt), "cached_prompt_tokens": str(cached),
                    "completion_tokens": str(out), "retries": str(retries), "retry_log": "[]", "model": "m", "finish": "ToolCall"}
        def outil(nom, ms, ok=True):
            return [{"kind": "ToolCallStarted", "tool": nom, "arguments": "{}"},
                    {"kind": "ToolCallFinished", "tool": nom, "ms": str(ms), "ok": str(ok), "error_kind": "None"}]
        def run(nom, pas, parent, ms, evs, usage):
            d = {"run": nom, "pas": pas, "tache": {"parent": parent, "nom": "campagne"}, "fin": {"ms": ms, "ok": True},
                 "evenements": evs, "usage_du_chat": usage}
            chemin = os.path.join(runs, nom + ".json")
            with open(chemin, "w") as f:
                json.dump(d, f)
            subprocess.run(["zstd", "-q", "--rm", chemin], check=True)
        run("agent-1", 1, "repo1", 10000, [llm(3000, 7000, 0, 20), *outil("read_file", 100), llm(5000, 9000, 6000, 30), *outil("run_command", 2000, False)],
            {"input_tokens": 10000, "cached_input_tokens": 6000, "output_tokens": 50, "reasoning_tokens": 7})
        run("agent-2", 2, "repo1", 400000, [llm(384000, 9000, 0, 34, retries=1)], {"input_tokens": 9000, "cached_input_tokens": 0, "output_tokens": 34, "reasoning_tokens": 0})
        run("agent-3", 1, "un-autre", 5, [llm(1, 1, 0, 1)], {})
        with open(os.path.join(runs, "graph-9.json.zst"), "wb") as f:
            f.write(b"")
        t = lanceur.build_trace(os.path.dirname(runs), "repo1")
        self.assertEqual(t["steps"], 2)
        self.assertEqual(t["model_calls"], 3)
        self.assertEqual(t["model_ms"], 392000)
        self.assertEqual(t["wall_ms"], 410000)
        self.assertEqual((t["input_tokens"], t["cached_tokens"], t["output_tokens"], t["reasoning_tokens"]), (19000, 6000, 84, 7))
        self.assertEqual(t["tool_calls"], {"read_file": 1, "run_command": 1})
        self.assertEqual(t["tool_errors"], 1)
        self.assertEqual(t["retries"], 1)
        self.assertEqual(t["slowest_model_calls"][0], {"step": 2, "ms": 384000, "prompt_tokens": 9000, "cached_tokens": 0, "output_tokens": 34, "retries": 1})
        self.assertEqual([s["step"] for s in t["per_step"]], [1, 2])


    def test_a_wrapped_block_naming_its_repository_is_read_and_cloned(self):
        b = {"lanceur": {"repo": self.repo, "parent": self.parent, "tree": "crate", "needs_lib": False,
                         "witness": "the_witness", "test": "cargo test", "suites": [], "hints": [], "ours": self.ours}}
        p = os.path.join(self.tmp.name, "cas-enveloppe.md")
        ecrire(p, "# Un cas\n\n```json\n" + json.dumps(b) + "\n```\n\n" + lanceur.REPORT_TITLE + "\n\n> f rend 1.\n")
        block, report = lanceur.read_case(p)
        self.assertEqual(block["parent"], self.parent)
        self.assertEqual(report, "f rend 1.")
        depots = os.path.join(self.tmp.name, "depots")
        repo = lanceur.repo_of(block, "/nowhere/the-default-repo", depots)
        self.assertTrue(repo.startswith(depots), repo)
        self.assertEqual(git(repo, "rev-parse", self.parent + "^{commit}"), self.parent)
        self.assertEqual(lanceur.repo_of({"parent": self.parent}, self.repo, depots), self.repo, "no repo: the default one")


    def test_a_script_witness_is_judged_by_its_exit_code(self):
        # A witness that is a script (recherche, case 03): bash for .sh, python3 for .py, red by a
        # non-zero exit code, never red when its last line says VERT.
        dest = os.path.join(self.tmp.name, "arbre")
        block, _ = lanceur.read_case(self.case("> f rend 1.", witness="tests/test_f.sh", test="bash tests/test_f.sh"))
        lanceur.build_tree(block, self.repo, os.path.join(self.tmp.name, "depots"), dest)
        script = os.path.join(dest, "crate/tests/test_f.sh")
        ecrire(script, "echo 'f rend 1'\necho ROUGE\nexit 1\n")
        rouge = lanceur.prove_red(block, dest, os.path.join(self.tmp.name, "target"), poste=None)
        self.assertTrue(rouge["red"], rouge)
        self.assertEqual(rouge["failed"], ["tests/test_f.sh"])
        ecrire(script, "echo VERT\nexit 0\n")
        self.assertFalse(lanceur.prove_red(block, dest, os.path.join(self.tmp.name, "target"), poste=None)["red"])
        ecrire(script, "echo VERT\nexit 3\n")
        rouge = lanceur.prove_red(block, dest, os.path.join(self.tmp.name, "target"), poste=None)
        self.assertFalse(rouge["red"], "a script that says VERT and fails is not a red: {}".format(rouge))
        # A Python witness, by python3.
        block, _ = lanceur.read_case(self.case("> f rend 1.", witness="tests/test_f.py", test="python3 tests/test_f.py"))
        ecrire(os.path.join(dest, "crate/tests/test_f.py"), "import sys\nprint('ROUGE')\nsys.exit(1)\n")
        self.assertTrue(lanceur.prove_red(block, dest, os.path.join(self.tmp.name, "target"), poste=None)["red"])


if __name__ == "__main__":
    unittest.main()
