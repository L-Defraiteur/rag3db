#!/usr/bin/env python3
"""Witness of menage.py (19 Nov 2026, Lucie and c7): nothing is touched by default; only a folder marked jetable
(`.jetable.json` inside it) can go, and within a family of marked folders the three most recent always stay, the others
go past 72 h. A folder held open by a process stays. --dry-run deletes nothing."""
import json, os, subprocess, sys, tempfile, time, unittest
ICI = os.path.dirname(os.path.abspath(__file__))
MENAGE = os.environ.get("MENAGE", os.path.join(ICI, "menage.py"))


def dossier(racine, nom, heures, marque=None, taille=1000):
    d = os.path.join(racine, nom); os.makedirs(d)
    with open(os.path.join(d, "base.rag3db"), "wb") as f: f.write(b"x" * taille)
    if marque is not None:
        m = dict(marque); m.setdefault("cree", time.strftime("%Y-%m-%dT%H:%M:%S", time.localtime(time.time() - heures * 3600)))
        with open(os.path.join(d, ".jetable.json"), "w") as f: json.dump(m, f)
    t = time.time() - heures * 3600; os.utime(d, (t, t))
    return d


def menage(*args):
    r = subprocess.run([sys.executable, "-I", MENAGE, *args], capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


class Menage(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(dir=os.path.expanduser("~/.cache")); self.r = self.tmp.name

    def tearDown(self):
        self.tmp.cleanup()

    def test_an_unmarked_folder_is_never_touched(self):
        for i in range(10): dossier(self.r, f"sans-marque-{i}", 240 + i)
        rc, out = menage("--racine", self.r)
        self.assertEqual(rc, 0, out)
        self.assertEqual(len(os.listdir(self.r)), 10, out)

    def test_three_marked_stay_whatever_their_age_the_others_go_past_72h(self):
        m = {"famille": "instantane-bascule", "par": "G"}
        for nom, h in [("a", 300), ("b", 280), ("c", 260), ("d", 250), ("e", 24)]:
            dossier(self.r, nom, h, m)
        rc, out = menage("--racine", self.r)
        self.assertEqual(rc, 0, out)
        # by date, e, d and c are the three most recent; a and b are past 72 h
        self.assertEqual(sorted(os.listdir(self.r)), ["c", "d", "e"], out)

    def test_a_fourth_marked_younger_than_72h_stays(self):
        m = {"famille": "f"}
        for nom, h in [("a", 1), ("b", 2), ("c", 3), ("d", 30)]:
            dossier(self.r, nom, h, m)
        menage("--racine", self.r)
        self.assertEqual(sorted(os.listdir(self.r)), ["a", "b", "c", "d"])

    def test_a_folder_marked_keep_zero_goes_at_once(self):
        dossier(self.r, "controle", 1, {"famille": "controle", "garder": 0, "age_max_h": 0})
        menage("--racine", self.r)
        self.assertEqual(os.listdir(self.r), [])

    def test_families_count_apart(self):
        for nom, h in [("x1", 100), ("x2", 110), ("x3", 120)]: dossier(self.r, nom, h, {"famille": "x"})
        for nom, h in [("y1", 100), ("y2", 110), ("y3", 120), ("y4", 130)]: dossier(self.r, nom, h, {"famille": "y"})
        menage("--racine", self.r)
        self.assertEqual(sorted(os.listdir(self.r)), ["x1", "x2", "x3", "y1", "y2", "y3"])

    def test_dry_run_deletes_nothing_and_says_what_would_go(self):
        for nom, h in [("a", 100), ("b", 110), ("c", 120), ("d", 130)]: dossier(self.r, nom, h, {"famille": "f"})
        rc, out = menage("--racine", self.r, "--a-blanc")
        self.assertEqual(len(os.listdir(self.r)), 4)
        self.assertIn("d", out)

    def test_a_folder_held_open_stays(self):
        for nom, h in [("a", 100), ("b", 110), ("c", 120)]: dossier(self.r, nom, h, {"famille": "f"})
        d = dossier(self.r, "ouvert", 130, {"famille": "f"})
        p = subprocess.Popen(["sleep", "30"], cwd=d)
        try:
            menage("--racine", self.r)
            self.assertIn("ouvert", os.listdir(self.r))
        finally:
            p.kill(); p.wait()

    def test_the_freed_bytes_are_said(self):
        for nom, h in [("a", 100), ("b", 110), ("c", 120)]: dossier(self.r, nom, h, {"famille": "f"})
        dossier(self.r, "d", 130, {"famille": "f"}, taille=5_000_000)
        rc, out = menage("--racine", self.r)
        self.assertRegex(out, r"rendus : [0-9.]+ ?Mio")


    def test_the_targets_are_swept_by_cargo_sweep_unless_a_build_holds_them(self):
        # --balayer <dossier des targets> <projet>: cargo sweep --time 2 on each target*/ that has a CACHEDIR.TAG, with
        # CARGO_TARGET_DIR on it; a target whose .cargo-lock a process holds is skipped (a build is running).
        bin_ = os.path.join(self.r, "bin"); os.makedirs(bin_)
        trace = os.path.join(self.r, "trace")
        with open(os.path.join(bin_, "cargo"), "w") as f:
            f.write('#!/bin/sh\necho "$CARGO_TARGET_DIR $*" >> ' + trace + '\n')
        os.chmod(os.path.join(bin_, "cargo"), 0o755)
        tg = os.path.join(self.r, "targets"); proj = os.path.join(self.r, "projet"); os.makedirs(proj)
        for n in ("target-a", "target-b", "pas-un-target"):
            os.makedirs(os.path.join(tg, n, "debug"))
        for n in ("target-a", "target-b"):
            open(os.path.join(tg, n, "CACHEDIR.TAG"), "w").close()
        lock = os.path.join(tg, "target-b", "debug", ".cargo-lock"); open(lock, "w").close()
        p = subprocess.Popen(["flock", lock, "sleep", "30"])
        try:
            time.sleep(0.5)
            env = dict(os.environ, PATH=bin_ + ":" + os.environ["PATH"])
            r = subprocess.run([sys.executable, "-I", MENAGE, "--racine", self.r, "--balayer", tg, proj], capture_output=True, text=True, env=env)
        finally:
            p.kill(); p.wait()
        lignes = open(trace).read().splitlines() if os.path.exists(trace) else []
        self.assertEqual(lignes, [os.path.join(tg, "target-a") + " sweep --time 2 " + proj], r.stdout + r.stderr)
        self.assertIn("target-b", r.stdout)


if __name__ == "__main__":
    unittest.main()
